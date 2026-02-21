// ExternalPlayerBackend — spawns mpv, vlc, or ffplay as a subprocess.
//
// This backend is used when:
//   - `config.audio.external_player` is explicitly set, OR
//   - RodioBackend fails to initialize (no ALSA device, WSL2, containers, etc.)
//
// Limitations (documented per trait method):
//   - pause() / resume()  → no-op; external process cannot be paused via this interface
//   - seek()              → Err(AudioError::Unsupported)
//   - position()          → None; cannot query external process
//   - duration()          → None; cannot query external process
//   - set_volume()        → stores value only; cannot control external process volume

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

use super::{AudioError, PlaybackBackend};

/// Fallback audio backend that delegates playback to an external CLI media player.
///
/// Supported players (auto-detected in order): `mpv`, `vlc`, `ffplay`.
pub struct ExternalPlayerBackend {
    /// The player executable name or full path (e.g. `"mpv"`, `"/usr/bin/vlc"`).
    player_command: String,
    /// Running child process handle, `None` when stopped.
    child: Option<Child>,
    /// Path of the file currently (or last) played.
    current_path: Option<PathBuf>,
    /// Stored volume (0.0–1.0); cannot be applied to the external process.
    volume: f32,
}

impl std::fmt::Debug for ExternalPlayerBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalPlayerBackend")
            .field("player_command", &self.player_command)
            .field("is_playing", &self.child.is_some())
            .field("volume", &self.volume)
            .finish()
    }
}

impl ExternalPlayerBackend {
    /// Create a backend that will use `player_command` as the executable.
    pub fn new(player_command: String) -> Self {
        Self {
            player_command,
            child: None,
            current_path: None,
            volume: crate::constants::audio::DEFAULT_VOLUME,
        }
    }

    /// Auto-detect an available player from the standard candidates
    /// (`mpv` → `vlc` → `ffplay`).
    ///
    /// Returns `Err(AudioError::ExternalPlayerNotFound)` when none is found.
    pub fn detect() -> Result<Self, AudioError> {
        Self::detect_from_candidates(&["mpv", "vlc", "ffplay"])
    }

    /// Detect from a custom candidate list — primarily useful for unit tests.
    pub(crate) fn detect_from_candidates(candidates: &[&str]) -> Result<Self, AudioError> {
        for &candidate in candidates {
            let ver_arg = version_flag(candidate);
            if Command::new(candidate).arg(ver_arg).output().is_ok() {
                return Ok(Self::new(candidate.to_string()));
            }
        }
        Err(AudioError::ExternalPlayerNotFound(format!(
            "No supported player found (tried: {})",
            candidates.join(", ")
        )))
    }
}

// ---------- PlaybackBackend impl --------------------------------------------

impl PlaybackBackend for ExternalPlayerBackend {
    /// Spawn the configured player with `path` as the target.
    ///
    /// Any previously-running child process is killed before the new one is spawned.
    fn play(&mut self, path: &Path) -> Result<(), AudioError> {
        // Kill any already-running child before spawning a new one.
        self.stop();

        let args = build_player_args(&self.player_command, path);
        let child = Command::new(&self.player_command)
            .args(args)
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    AudioError::ExternalPlayerNotFound(self.player_command.clone())
                } else {
                    AudioError::Io(e)
                }
            })?;

        self.child = Some(child);
        self.current_path = Some(path.to_path_buf());
        Ok(())
    }

    /// No-op — external processes cannot be paused via this interface.
    fn pause(&mut self) {
        // External players have no pause mechanism exposed via CLI args at spawn time.
        // The AudioManager will show "External player — limited controls" in the UI.
    }

    /// No-op — external player is either playing or stopped; cannot resume.
    fn resume(&mut self) {}

    /// Kill the running child process, reap it (best-effort), and clear the handle.
    ///
    /// Note: `Child::kill()` sends SIGKILL (Unix) / TerminateProcess (Windows).
    /// We then call `wait()` best-effort so the process is reaped and does not
    /// remain as a zombie on Unix.
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            // Best-effort reap to avoid zombie processes on Unix.
            let _ = child.wait();
        }
        self.current_path = None;
    }

    /// Seek is not supported for external players.
    fn seek(&mut self, _position: Duration) -> Result<(), AudioError> {
        Err(AudioError::Unsupported(
            "Seek not supported with external player".to_string(),
        ))
    }

    /// Store the volume value; it cannot be applied to the running external process.
    ///
    /// Volume is clamped to [0.0, 1.0] consistent with `constants::audio::DEFAULT_VOLUME`.
    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Always `None` — cannot query external process position.
    fn position(&self) -> Option<Duration> {
        None
    }

    /// Always `None` — cannot query external process duration.
    fn duration(&self) -> Option<Duration> {
        None
    }

    /// `true` while a child process handle is held (i.e. after a successful `play()`).
    ///
    /// Note: this returns `true` even if the external process has already exited
    /// on its own. The `AudioManager` is responsible for polling and cleaning up
    /// finished processes via `stop()`.
    fn is_playing(&self) -> bool {
        self.child.is_some()
    }

    /// Always `false` — external players cannot be paused.
    fn is_paused(&self) -> bool {
        false
    }

    /// `true` when no child process is held.
    fn is_stopped(&self) -> bool {
        self.child.is_none()
    }
}

// ---------- Drop ------------------------------------------------------------

impl Drop for ExternalPlayerBackend {
    fn drop(&mut self) {
        // Kill any running child process when the backend is dropped, then reap
        // it best-effort to avoid zombie processes on Unix.
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------- Pure helpers ----------------------------------------------------

/// Extract the lowercase player basename from an executable path or name,
/// stripping path separators and platform extensions (e.g. `.exe` on Windows).
///
/// Examples:
/// - `"mpv"` → `"mpv"`
/// - `"/usr/bin/mpv"` → `"mpv"`
/// - `r"C:\Program Files\mpv\mpv.exe"` → `"mpv"`
/// - `"ffplay.exe"` → `"ffplay"`
fn player_basename(cmd: &str) -> String {
    Path::new(cmd)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| cmd.to_lowercase())
}

/// Returns the version-check flag for `cmd`.
/// ffplay/ffmpeg/ffprobe use single-dash (`-version`); all others use `--version`.
fn version_flag(cmd: &str) -> &'static str {
    match player_basename(cmd).as_str() {
        "ffplay" | "ffmpeg" | "ffprobe" => "-version",
        _ => "--version",
    }
}

/// Build the argument list for spawning `player_command` with `path`.
///
/// Returns only the *arguments* (not the executable name itself).
/// Uses `player_basename()` so that full paths and `.exe` suffixes on Windows
/// are handled correctly (e.g. `C:\bin\mpv.exe` → uses mpv headless flags).
pub(crate) fn build_player_args(player_command: &str, path: &Path) -> Vec<String> {
    let path_str = path.to_string_lossy().to_string();
    match player_basename(player_command).as_str() {
        "mpv" => vec!["--no-video".into(), "--really-quiet".into(), path_str],
        "vlc" => vec![
            "--intf".into(),
            "dummy".into(),
            "--no-video".into(),
            path_str,
        ],
        "ffplay" => vec![
            "-nodisp".into(),
            "-loglevel".into(),
            "quiet".into(),
            path_str,
        ],
        // Unknown player: just pass the path as the sole argument.
        _ => vec![path_str],
    }
}

// ---------- Tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── Constructor / initial state ─────────────────────────────────────────

    #[test]
    fn test_external_backend_new_stores_command() {
        // Arrange / Act
        let backend = ExternalPlayerBackend::new("mpv".to_string());
        // Assert
        assert_eq!(backend.player_command, "mpv");
        assert!(backend.child.is_none());
        assert!(backend.current_path.is_none());
    }

    #[test]
    fn test_external_backend_is_playing_false_when_no_child() {
        // Arrange
        let backend = ExternalPlayerBackend::new("mpv".to_string());
        // Assert
        assert!(!backend.is_playing());
    }

    #[test]
    fn test_external_backend_is_stopped_true_when_new() {
        // Arrange
        let backend = ExternalPlayerBackend::new("mpv".to_string());
        // Assert
        assert!(backend.is_stopped());
        assert!(!backend.is_paused());
    }

    // ── Unsupported operations ──────────────────────────────────────────────

    #[test]
    fn test_external_backend_pause_is_noop() {
        // Arrange
        let mut backend = ExternalPlayerBackend::new("mpv".to_string());
        // Act — must not panic
        backend.pause();
        // Assert — still stopped
        assert!(backend.is_stopped());
    }

    #[test]
    fn test_external_backend_resume_is_noop() {
        // Arrange
        let mut backend = ExternalPlayerBackend::new("mpv".to_string());
        // Act — must not panic
        backend.resume();
        // Assert — still stopped
        assert!(backend.is_stopped());
    }

    #[test]
    fn test_external_backend_seek_returns_unsupported_error() {
        // Arrange
        let mut backend = ExternalPlayerBackend::new("mpv".to_string());
        // Act
        let result = backend.seek(Duration::from_secs(30));
        // Assert
        assert!(matches!(result, Err(AudioError::Unsupported(_))));
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Seek not supported"));
    }

    #[test]
    fn test_external_backend_position_returns_none() {
        let backend = ExternalPlayerBackend::new("mpv".to_string());
        assert!(backend.position().is_none());
    }

    #[test]
    fn test_external_backend_duration_returns_none() {
        let backend = ExternalPlayerBackend::new("mpv".to_string());
        assert!(backend.duration().is_none());
    }

    // ── Volume ──────────────────────────────────────────────────────────────

    #[test]
    fn test_external_backend_set_volume_stores_value() {
        // Arrange
        let mut backend = ExternalPlayerBackend::new("mpv".to_string());
        // Act
        backend.set_volume(0.5);
        // Assert
        assert!((backend.volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_external_backend_set_volume_clamps_to_range() {
        // Arrange
        let mut backend = ExternalPlayerBackend::new("mpv".to_string());
        // Act — below min
        backend.set_volume(-1.0);
        assert!((backend.volume - 0.0).abs() < f32::EPSILON);
        // Act — above max (clamped to 1.0 per constants::audio range)
        backend.set_volume(5.0);
        assert!((backend.volume - 1.0).abs() < f32::EPSILON);
    }

    // ── Stop on idle ────────────────────────────────────────────────────────

    #[test]
    fn test_external_backend_stop_on_idle_is_noop() {
        // Arrange
        let mut backend = ExternalPlayerBackend::new("mpv".to_string());
        // Act — stop when no child must not panic
        backend.stop();
        // Assert
        assert!(backend.is_stopped());
    }

    // ── Detect ──────────────────────────────────────────────────────────────

    #[test]
    fn test_external_backend_detect_returns_error_when_no_player() {
        // Arrange — use a definitely non-existent binary
        // Act
        let result =
            ExternalPlayerBackend::detect_from_candidates(&["__nonexistent_binary_abc123__"]);
        // Assert
        assert!(matches!(result, Err(AudioError::ExternalPlayerNotFound(_))));
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("__nonexistent_binary_abc123__"));
    }

    #[test]
    fn test_external_backend_detect_stores_found_player_command() {
        // Use the current test binary as a guaranteed-present executable.
        // Any real binary in PATH works; current_exe() is hermetic across environments.
        let current = std::env::current_exe().expect("could not resolve current_exe");
        let current_str = current.to_string_lossy().to_string();

        // Act
        let result = ExternalPlayerBackend::detect_from_candidates(&[current_str.as_str()]);
        // Assert — the test binary is always present, so detection must succeed.
        assert!(result.is_ok());
    }

    // ── Player argument construction ─────────────────────────────────────────

    #[test]
    fn test_external_backend_play_constructs_correct_mpv_args() {
        // Arrange
        let path = Path::new("/podcasts/episode.mp3");
        // Act
        let args = build_player_args("mpv", path);
        // Assert
        assert_eq!(
            args,
            vec!["--no-video", "--really-quiet", "/podcasts/episode.mp3"]
        );
    }

    #[test]
    fn test_external_backend_play_constructs_correct_vlc_args() {
        // Arrange
        let path = Path::new("/podcasts/episode.mp3");
        // Act
        let args = build_player_args("vlc", path);
        // Assert
        assert_eq!(
            args,
            vec!["--intf", "dummy", "--no-video", "/podcasts/episode.mp3"]
        );
    }

    #[test]
    fn test_external_backend_play_constructs_correct_ffplay_args() {
        // Arrange
        let path = Path::new("/podcasts/episode.mp3");
        // Act
        let args = build_player_args("ffplay", path);
        // Assert
        assert_eq!(
            args,
            vec!["-nodisp", "-loglevel", "quiet", "/podcasts/episode.mp3"]
        );
    }

    #[test]
    fn test_external_backend_play_unknown_player_passes_only_path() {
        // Arrange
        let path = Path::new("/podcasts/episode.mp3");
        // Act
        let args = build_player_args("mplayer", path);
        // Assert
        assert_eq!(args, vec!["/podcasts/episode.mp3"]);
    }

    #[test]
    fn test_external_backend_full_path_player_basename_detected() {
        // Arrange — full Unix path to mpv
        let path = Path::new("/tmp/ep.mp3");
        // Act
        let args = build_player_args("/usr/bin/mpv", path);
        // Assert — uses mpv args despite full path
        assert_eq!(args, vec!["--no-video", "--really-quiet", "/tmp/ep.mp3"]);
    }

    #[test]
    fn test_external_backend_exe_suffix_stripped_for_args() {
        // Arrange — Windows-style path with .exe extension
        let path = Path::new("C:\\podcasts\\ep.mp3");
        // Act — mpv.exe should match mpv args
        let args = build_player_args("mpv.exe", path);
        // Assert
        assert_eq!(
            args,
            vec!["--no-video", "--really-quiet", "C:\\podcasts\\ep.mp3"]
        );
    }

    #[test]
    fn test_external_backend_exe_suffix_stripped_for_vlc() {
        let path = Path::new("episode.mp3");
        let args = build_player_args("vlc.exe", path);
        assert_eq!(args, vec!["--intf", "dummy", "--no-video", "episode.mp3"]);
    }

    #[test]
    fn test_external_backend_full_windows_path_mpv_exe() {
        // Full Windows path — mpv.exe at arbitrary location
        let path = Path::new("episode.mp3");
        let args = build_player_args(r"C:\Program Files\mpv\mpv.exe", path);
        assert_eq!(args, vec!["--no-video", "--really-quiet", "episode.mp3"]);
    }

    // ── Stop kills process (Unix only) ────────────────────────────────────────

    /// Verify that `stop()` kills a running child process and clears the handle.
    /// Uses `sleep 100` as a long-running proxy (Unix only).
    #[cfg(unix)]
    #[test]
    fn test_external_backend_stop_kills_running_process() {
        // Arrange — spawn a long-running process and inject it as the child
        let child = Command::new("sleep").arg("100").spawn();
        if child.is_err() {
            // sleep not available — skip
            return;
        }
        let mut backend = ExternalPlayerBackend::new("sleep".to_string());
        backend.child = Some(child.unwrap());

        assert!(backend.is_playing());

        // Act
        backend.stop();

        // Assert
        assert!(!backend.is_playing());
        assert!(backend.is_stopped());
        assert!(backend.child.is_none());
    }

    // ── Version flag helper ──────────────────────────────────────────────────

    #[test]
    fn test_version_flag_ffplay_uses_single_dash() {
        assert_eq!(version_flag("ffplay"), "-version");
        assert_eq!(version_flag("ffmpeg"), "-version");
        assert_eq!(version_flag("ffprobe"), "-version");
    }

    #[test]
    fn test_version_flag_others_use_double_dash() {
        assert_eq!(version_flag("mpv"), "--version");
        assert_eq!(version_flag("vlc"), "--version");
        assert_eq!(version_flag("mplayer"), "--version");
    }

    #[test]
    fn test_version_flag_exe_suffix_stripped() {
        // Windows executables with .exe suffix must still match correctly.
        assert_eq!(version_flag("ffplay.exe"), "-version");
        assert_eq!(version_flag("mpv.exe"), "--version");
    }

    #[test]
    fn test_version_flag_full_windows_path() {
        assert_eq!(version_flag(r"C:\bin\ffplay.exe"), "-version");
        assert_eq!(version_flag(r"C:\bin\mpv.exe"), "--version");
    }
}
