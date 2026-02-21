// RodioBackend — native cross-platform audio playback via rodio.
//
// Primary backend. Uses WASAPI on Windows, ALSA on Linux, CoreAudio on macOS.
//
// Design notes:
//   - rodio 0.21 API: OutputStreamBuilder::open_default_stream(), Sink::connect_new(),
//     Decoder::try_from(file).
//   - Sink::get_pos() provides accurate position tracking (updated every ~5 ms by
//     the audio thread's periodic_access callback).
//   - A fresh Sink is created on each play() call. Dropping the old Sink cleanly
//     stops the previous track via its Drop impl.
//   - OutputStream must remain alive for the duration of playback — drop = silence.

use std::path::Path;
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

use super::{AudioError, PlaybackBackend};

/// Native audio backend using rodio (WASAPI / ALSA / CoreAudio).
///
/// Position is queried via `Sink::get_pos()`, which rodio updates every ~5 ms
/// from the underlying `track_position()` source wrapper.
pub struct RodioBackend {
    /// Output stream — **must stay alive** for the duration of playback.
    /// Drop = silence. The underscore prefix is idiomatic for "kept alive, not read directly".
    _stream: OutputStream,
    /// Active playback sink.  Replaced on every `play()` call by dropping the old one.
    sink: Sink,
    /// Total track duration captured from the decoder headers when available.
    total_duration: Option<Duration>,
    /// Current volume level, clamped to [0.0, 1.0].
    volume: f32,
}

impl std::fmt::Debug for RodioBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RodioBackend")
            .field("is_playing", &self.is_playing())
            .field("is_paused", &self.is_paused())
            .field("position", &self.position())
            .field("volume", &self.volume)
            .finish()
    }
}

impl RodioBackend {
    /// Create a new `RodioBackend`, initialising the default audio output device.
    ///
    /// Returns `Err(AudioError::DeviceNotFound)` when no output device is available
    /// (headless CI, WSL2 without audio passthrough, containers, etc.).
    pub fn new() -> Result<Self, AudioError> {
        let mut stream =
            OutputStreamBuilder::open_default_stream().map_err(|_| AudioError::DeviceNotFound)?;
        // Suppress the "Dropping OutputStream" stderr message — not appropriate for a TUI.
        stream.log_on_drop(false);
        let sink = Sink::connect_new(stream.mixer());

        Ok(Self {
            _stream: stream,
            sink,
            total_duration: None,
            volume: crate::constants::audio::DEFAULT_VOLUME,
        })
    }
}

// ---------- PlaybackBackend impl --------------------------------------------

impl PlaybackBackend for RodioBackend {
    /// Load and play a local audio file (MP3, AAC, OGG, FLAC, WAV).
    ///
    /// A fresh `Sink` is created for each invocation.  Dropping the old `Sink`
    /// stops any in-progress playback cleanly via its `Drop` impl.
    fn play(&mut self, path: &Path) -> Result<(), AudioError> {
        let file = std::fs::File::open(path).map_err(AudioError::Io)?;
        // Decoder::try_from(File) wraps in BufReader, reads byte_len from metadata,
        // and enables seeking — the recommended approach in rodio 0.21.
        let decoder =
            Decoder::try_from(file).map_err(|e| AudioError::DecodingFailed(e.to_string()))?;

        // Capture duration before the decoder is moved into the sink.
        let total_duration = decoder.total_duration();

        // Create a fresh sink for this track.  The previous sink is dropped here,
        // which stops any current playback cleanly.
        // Note: self._stream and self.sink are distinct fields; Rust allows borrowing
        // self._stream (for mixer()) while assigning to self.sink.
        let new_sink = Sink::connect_new(self._stream.mixer());
        new_sink.set_volume(self.volume);
        new_sink.append(decoder);
        self.sink = new_sink;

        self.total_duration = total_duration;
        Ok(())
    }

    fn pause(&mut self) {
        if self.is_playing() {
            self.sink.pause();
        }
    }

    fn resume(&mut self) {
        if self.is_paused() {
            self.sink.play();
        }
    }

    /// Stop playback and clear duration state.
    ///
    /// Uses `Sink::clear()` (synchronous) rather than `Sink::stop()` (async signal)
    /// so that `is_stopped()` returns `true` immediately after this call returns.
    /// `clear()` blocks at most one audio-thread tick (~5 ms) until sources are removed.
    fn stop(&mut self) {
        self.sink.clear();
        self.total_duration = None;
    }

    /// Seek to `position` within the current track.
    ///
    /// rodio 0.21's `try_seek` blocks up to ~5 ms while the audio thread processes
    /// the seek order, then updates `Sink::get_pos()` to the new position.
    fn seek(&mut self, position: Duration) -> Result<(), AudioError> {
        self.sink
            .try_seek(position)
            .map_err(|e| AudioError::SeekFailed(e.to_string()))
    }

    /// Set volume, clamped to [0.0, 1.0].
    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.volume);
    }

    /// Returns the current playback position via `Sink::get_pos()`.
    ///
    /// Accuracy is within ~5 ms (the rodio periodic_access update interval).
    /// Returns `None` when stopped (including when the track has ended naturally).
    fn position(&self) -> Option<Duration> {
        if self.is_stopped() {
            None
        } else {
            Some(self.sink.get_pos())
        }
    }

    fn duration(&self) -> Option<Duration> {
        self.total_duration
    }

    /// `true` while a source is active **and** not paused.
    ///
    /// Returns `false` once the track ends naturally (sink becomes empty).
    fn is_playing(&self) -> bool {
        !self.sink.empty() && !self.sink.is_paused()
    }

    /// `true` while a source is active **and** paused.
    fn is_paused(&self) -> bool {
        !self.sink.empty() && self.sink.is_paused()
    }

    /// `true` when the sink is empty — either explicitly stopped or track ended naturally.
    ///
    /// `AudioManager` uses this to detect `TrackEnded` events
    /// (see `rodio::Sink::empty()` docs).
    fn is_stopped(&self) -> bool {
        self.sink.empty()
    }
}

// ---------- Tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Attempt to create a `RodioBackend`.  Returns `None` when no audio output
    /// device is available — callers use this to skip tests gracefully in CI.
    fn try_create_backend() -> Option<RodioBackend> {
        RodioBackend::new().ok()
    }

    /// Write a minimal silent PCM WAV file of `duration_secs` seconds.
    /// 44 100 Hz · mono · 16-bit · silence.
    fn write_test_wav(path: &Path, duration_secs: u32) {
        let sample_rate: u32 = 44_100;
        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let num_samples: u32 = sample_rate * duration_secs;
        let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
        let block_align = num_channels * bits_per_sample / 8;
        let data_size = num_samples * num_channels as u32 * bits_per_sample as u32 / 8;

        let mut bytes: Vec<u8> = Vec::with_capacity((44 + data_size) as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36u32 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&num_channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.extend_from_slice(&vec![0u8; data_size as usize]);

        std::fs::write(path, &bytes).unwrap();
    }

    // ── Constructor / initial state ─────────────────────────────────────────

    #[test]
    fn test_rodio_backend_new_succeeds_with_audio_device() {
        // May be skipped in CI environments without an audio output device.
        match RodioBackend::new() {
            Ok(backend) => {
                // Assert — fresh backend is in stopped state
                assert!(backend.is_stopped());
                assert!(!backend.is_playing());
                assert!(!backend.is_paused());
                assert_eq!(backend.volume, crate::constants::audio::DEFAULT_VOLUME);
            }
            Err(AudioError::DeviceNotFound) => {
                eprintln!(
                    "No audio device found — skipping \
                    test_rodio_backend_new_succeeds_with_audio_device"
                );
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn test_rodio_backend_initial_state_is_stopped() {
        // Arrange / Act
        let backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_initial_state_is_stopped");
                return;
            }
        };

        // Assert
        assert!(backend.is_stopped());
        assert!(!backend.is_playing());
        assert!(!backend.is_paused());
        assert!(backend.position().is_none());
        assert!(backend.duration().is_none());
    }

    // ── Volume ──────────────────────────────────────────────────────────────

    #[test]
    fn test_rodio_backend_volume_clamps_to_valid_range() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping test_rodio_backend_volume_clamps_to_valid_range"
                );
                return;
            }
        };

        // Act — above max
        backend.set_volume(2.0);
        // Assert
        assert!((backend.volume - 1.0).abs() < f32::EPSILON);

        // Act — below min
        backend.set_volume(-1.0);
        // Assert
        assert!((backend.volume - 0.0).abs() < f32::EPSILON);

        // Act — valid mid-range
        backend.set_volume(0.5);
        // Assert
        assert!((backend.volume - 0.5).abs() < f32::EPSILON);
    }

    // ── Idle-state no-ops ───────────────────────────────────────────────────

    #[test]
    fn test_rodio_backend_stop_on_idle_is_noop() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_stop_on_idle_is_noop");
                return;
            }
        };

        // Act — stop() when already stopped must not panic
        backend.stop();

        // Assert
        assert!(backend.is_stopped());
    }

    #[test]
    fn test_rodio_backend_pause_on_idle_is_noop() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_pause_on_idle_is_noop");
                return;
            }
        };

        // Act — pause() when already stopped must not panic
        backend.pause();

        // Assert
        assert!(backend.is_stopped());
    }

    #[test]
    fn test_rodio_backend_resume_on_idle_is_noop() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_resume_on_idle_is_noop");
                return;
            }
        };

        // Act — resume() when stopped must not panic
        backend.resume();

        // Assert
        assert!(backend.is_stopped());
    }

    // ── Error cases ─────────────────────────────────────────────────────────

    #[test]
    fn test_rodio_backend_play_missing_file_returns_io_error() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping \
                    test_rodio_backend_play_missing_file_returns_io_error"
                );
                return;
            }
        };

        // Act
        let result = backend.play(Path::new("/nonexistent/podcast_episode.mp3"));

        // Assert
        assert!(matches!(result, Err(AudioError::Io(_))));
    }

    // ── Position returns None when stopped ──────────────────────────────────

    #[test]
    fn test_rodio_backend_position_returns_none_when_stopped() {
        // Arrange
        let backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping \
                    test_rodio_backend_position_returns_none_when_stopped"
                );
                return;
            }
        };

        // Assert
        assert!(backend.position().is_none());
    }

    // ── Play / state transitions ─────────────────────────────────────────────

    #[test]
    fn test_rodio_backend_play_sets_playing_state() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_play_sets_playing_state");
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 5);

        // Act
        backend.play(&wav_path).expect("play should succeed");

        // Assert
        assert!(backend.is_playing());
        assert!(!backend.is_paused());
        assert!(!backend.is_stopped());
    }

    #[test]
    fn test_rodio_backend_pause_resume_cycle() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_pause_resume_cycle");
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 5);
        backend.play(&wav_path).expect("play should succeed");

        // Act — pause
        backend.pause();

        // Assert — paused
        assert!(backend.is_paused());
        assert!(!backend.is_playing());
        assert!(!backend.is_stopped());

        // Act — resume
        backend.resume();

        // Assert — playing again
        assert!(backend.is_playing());
        assert!(!backend.is_paused());
    }

    #[test]
    fn test_rodio_backend_stop_during_play_clears_state() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping \
                    test_rodio_backend_stop_during_play_clears_state"
                );
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 5);
        backend.play(&wav_path).expect("play should succeed");
        assert!(backend.is_playing());

        // Act
        backend.stop();

        // Assert
        assert!(backend.is_stopped());
        assert!(!backend.is_playing());
        assert!(backend.position().is_none());
        assert!(backend.duration().is_none());
    }

    // ── Position tracking ────────────────────────────────────────────────────

    #[test]
    fn test_rodio_backend_position_tracking_accumulates_correctly() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping \
                    test_rodio_backend_position_tracking_accumulates_correctly"
                );
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 5);

        // Act — play, wait briefly, pause, wait, resume
        backend.play(&wav_path).expect("play should succeed");
        std::thread::sleep(Duration::from_millis(100));

        backend.pause();
        // Sleep 20 ms to let the audio thread's periodic_access tick process the pause
        // and freeze controls.position before we read it.
        std::thread::sleep(Duration::from_millis(20));
        let pos_after_pause = backend
            .position()
            .expect("position should be Some while paused");

        // Position while paused must not advance
        std::thread::sleep(Duration::from_millis(100));
        let pos_still_paused = backend
            .position()
            .expect("position should be Some while paused");
        assert_eq!(
            pos_after_pause, pos_still_paused,
            "position must not advance while paused"
        );

        // Resume and verify position advances again
        backend.resume();
        std::thread::sleep(Duration::from_millis(100));
        let pos_after_resume = backend
            .position()
            .expect("position should be Some while playing");
        assert!(
            pos_after_resume > pos_after_pause,
            "position must advance after resume"
        );
    }

    #[test]
    fn test_rodio_backend_position_advances_during_playback() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping \
                    test_rodio_backend_position_advances_during_playback"
                );
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 5);

        // Act
        backend.play(&wav_path).expect("play should succeed");
        let pos_t0 = backend.position().expect("position Some while playing");
        std::thread::sleep(Duration::from_millis(150));
        let pos_t1 = backend.position().expect("position Some while playing");

        // Assert — position should have advanced by roughly 150 ms (allow ±80 ms slop)
        let delta = pos_t1.saturating_sub(pos_t0);
        assert!(
            delta >= Duration::from_millis(50),
            "position advanced by only {delta:?} — expected ~150 ms"
        );
    }

    // ── Duration ─────────────────────────────────────────────────────────────

    #[test]
    fn test_rodio_backend_duration_from_wav_file() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!("No audio device — skipping test_rodio_backend_duration_from_wav_file");
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 3); // 3-second WAV

        // Act
        backend.play(&wav_path).expect("play should succeed");

        // Assert — rodio's Decoder should read WAV duration from headers
        if let Some(dur) = backend.duration() {
            // Allow ±200 ms tolerance
            let expected = Duration::from_secs(3);
            let diff = if dur > expected {
                dur - expected
            } else {
                expected - dur
            };
            assert!(
                diff <= Duration::from_millis(200),
                "duration {dur:?} not within 200 ms of 3 s"
            );
        }
        // If duration() returns None, that is also acceptable per the trait contract.
    }

    // ── Second play() replaces first ─────────────────────────────────────────

    #[test]
    fn test_rodio_backend_second_play_replaces_first() {
        // Arrange
        let mut backend = match try_create_backend() {
            Some(b) => b,
            None => {
                eprintln!(
                    "No audio device — skipping \
                    test_rodio_backend_second_play_replaces_first"
                );
                return;
            }
        };
        let dir = tempfile::TempDir::new().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_test_wav(&wav_path, 5);

        // Act — play twice
        backend.play(&wav_path).expect("first play should succeed");
        std::thread::sleep(Duration::from_millis(100));
        backend.play(&wav_path).expect("second play should succeed");

        // Assert — should be playing and position should be near zero
        assert!(backend.is_playing());
        let pos = backend.position().expect("position Some while playing");
        assert!(
            pos < Duration::from_millis(500),
            "position after second play should be near 0, got {pos:?}"
        );
    }

    // ── Position math (device-independent) ───────────────────────────────────

    #[test]
    fn test_position_math_duration_arithmetic() {
        // Verify that Duration arithmetic used internally produces correct values.
        // Device-independent — does not require an audio output device.
        let base = Duration::from_secs(30);
        let elapsed = Duration::from_millis(500);
        assert_eq!(base + elapsed, Duration::from_millis(30_500));
        assert_eq!(
            base.checked_sub(Duration::from_secs(5)),
            Some(Duration::from_secs(25))
        );
    }
}
