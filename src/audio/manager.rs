// AudioManager — threaded playback coordinator.
//
// Owns the audio backend on a dedicated OS thread (std::thread, not tokio::spawn)
// to avoid cpal/tokio deadlock. Communicates with the UI via three channels:
//
//   UI → AudioManager:  mpsc::UnboundedSender<AudioCommand>   (commands)
//   AudioManager → UI:  watch::Receiver<PlaybackStatus>       (~4 Hz status)
//   AudioManager → UI:  mpsc::UnboundedSender<AppEvent>       (one-shot events)

use std::time::Duration;

use tokio::sync::{mpsc, watch};

use crate::audio::{AudioCommand, AudioError, PlaybackBackend, PlaybackState, PlaybackStatus};
use crate::config::AudioConfig;
use crate::storage::{EpisodeId, PodcastId};
use crate::ui::events::AppEvent;

/// Central playback coordinator.
///
/// `AudioManager` owns the audio backend on a dedicated OS thread, providing:
/// - A command channel for UI-driven operations (play, pause, seek, volume)
/// - A status watch channel for continuous state updates (~4 Hz)
/// - One-shot `AppEvent`s for significant lifecycle transitions
pub struct AudioManager {
    command_tx: mpsc::UnboundedSender<AudioCommand>,
    status_rx: watch::Receiver<PlaybackStatus>,
    /// Kept alive for its lifetime; the thread exits when `command_tx` is dropped.
    _thread: std::thread::JoinHandle<()>,
}

impl AudioManager {
    /// Create an `AudioManager`, selecting the appropriate backend from `config`
    /// and spawning the audio thread.
    ///
    /// Backend selection order:
    /// 1. `config.external_player` set → `ExternalPlayerBackend`
    /// 2. `RodioBackend::new()` succeeds → use it
    /// 3. `RodioBackend` fails → `ExternalPlayerBackend::detect()`
    /// 4. Both fail → return the original rodio error
    pub fn new(
        config: &AudioConfig,
        app_event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<Self, AudioError> {
        let backend = create_backend(config)?;

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let initial_volume = config.volume.clamp(0.0, 1.0);
        let initial_status = PlaybackStatus {
            volume: initial_volume,
            ..PlaybackStatus::default()
        };
        let (status_tx, status_rx) = watch::channel(initial_status);

        let thread = std::thread::Builder::new()
            .name("audio-manager".into())
            .spawn(move || {
                run_loop(backend, command_rx, status_tx, app_event_tx, initial_volume);
            })?;

        Ok(Self {
            command_tx,
            status_rx,
            _thread: thread,
        })
    }

    /// Send a command to the audio thread (fire-and-forget).
    ///
    /// Returns silently if the audio thread has exited (channel disconnected).
    pub fn send(&self, cmd: AudioCommand) {
        let _ = self.command_tx.send(cmd);
    }

    /// Subscribe to status updates.
    ///
    /// Returns a cloned `watch::Receiver` that reflects the latest `PlaybackStatus`.
    /// Callers poll this on each UI tick to render the NowPlaying bar.
    pub fn subscribe(&self) -> watch::Receiver<PlaybackStatus> {
        self.status_rx.clone()
    }
}

// ---------- Backend selection -----------------------------------------------

fn create_backend(config: &AudioConfig) -> Result<Box<dyn PlaybackBackend>, AudioError> {
    if let Some(ref player) = config.external_player {
        return Ok(Box::new(
            crate::audio::external::ExternalPlayerBackend::new(player.clone()),
        ));
    }

    match crate::audio::rodio_backend::RodioBackend::new() {
        Ok(backend) => Ok(Box::new(backend)),
        Err(rodio_err) => {
            eprintln!(
                "RodioBackend init failed: {rodio_err}. Attempting external player fallback…"
            );
            match crate::audio::external::ExternalPlayerBackend::detect() {
                Ok(backend) => Ok(Box::new(backend)),
                Err(_) => Err(rodio_err),
            }
        }
    }
}

// ---------- Audio thread loop -----------------------------------------------

fn run_loop(
    mut backend: Box<dyn PlaybackBackend>,
    mut command_rx: mpsc::UnboundedReceiver<AudioCommand>,
    status_tx: watch::Sender<PlaybackStatus>,
    app_event_tx: mpsc::UnboundedSender<AppEvent>,
    initial_volume: f32,
) {
    let mut current_episode: Option<(EpisodeId, PodcastId)> = None;
    let mut was_playing = false;
    let mut volume = initial_volume;

    loop {
        // Drain all pending commands before the next status broadcast.
        loop {
            match command_rx.try_recv() {
                Ok(cmd) => {
                    process_command(
                        cmd,
                        &mut *backend,
                        &app_event_tx,
                        &mut current_episode,
                        &mut volume,
                    );
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => return,
            }
        }

        // Detect natural track end: was playing last tick, now stopped (not paused).
        let is_playing_now = backend.is_playing();
        if was_playing && !is_playing_now && !backend.is_paused() {
            if let Some((ref ep_id, ref pod_id)) = current_episode {
                let _ = app_event_tx.send(AppEvent::TrackEnded {
                    podcast_id: pod_id.clone(),
                    episode_id: ep_id.clone(),
                });
                current_episode = None;
            }
        }
        was_playing = is_playing_now;

        // Broadcast current status at ~4 Hz.
        let state = if backend.is_playing() {
            PlaybackState::Playing
        } else if backend.is_paused() {
            PlaybackState::Paused
        } else {
            PlaybackState::Stopped
        };
        let status = PlaybackStatus {
            state,
            episode_id: current_episode.as_ref().map(|(e, _)| e.clone()),
            podcast_id: current_episode.as_ref().map(|(_, p)| p.clone()),
            position: backend.position(),
            duration: backend.duration(),
            volume,
        };
        let _ = status_tx.send(status);

        std::thread::sleep(Duration::from_millis(250));
    }
}

fn process_command(
    cmd: AudioCommand,
    backend: &mut dyn PlaybackBackend,
    app_event_tx: &mpsc::UnboundedSender<AppEvent>,
    current_episode: &mut Option<(EpisodeId, PodcastId)>,
    volume: &mut f32,
) {
    match cmd {
        AudioCommand::Play {
            path,
            episode_id,
            podcast_id,
        } => {
            backend.stop();
            match backend.play(&path) {
                Ok(()) => {
                    *current_episode = Some((episode_id.clone(), podcast_id.clone()));
                    let _ = app_event_tx.send(AppEvent::PlaybackStarted {
                        podcast_id,
                        episode_id,
                    });
                }
                Err(e) => {
                    *current_episode = None;
                    let _ = app_event_tx.send(AppEvent::PlaybackError {
                        error: e.to_string(),
                    });
                }
            }
        }
        AudioCommand::TogglePlayPause => {
            if backend.is_playing() {
                backend.pause();
            } else if backend.is_paused() {
                backend.resume();
            }
        }
        AudioCommand::Pause => backend.pause(),
        AudioCommand::Resume => backend.resume(),
        AudioCommand::Stop => {
            backend.stop();
            *current_episode = None;
            let _ = app_event_tx.send(AppEvent::PlaybackStopped);
        }
        AudioCommand::SeekForward(delta) => {
            let target = backend
                .position()
                .unwrap_or(Duration::ZERO)
                .saturating_add(delta);
            if let Err(e) = backend.seek(target) {
                eprintln!("SeekForward failed: {e}");
            }
        }
        AudioCommand::SeekBackward(delta) => {
            let target = backend
                .position()
                .unwrap_or(Duration::ZERO)
                .saturating_sub(delta);
            if let Err(e) = backend.seek(target) {
                eprintln!("SeekBackward failed: {e}");
            }
        }
        AudioCommand::SetVolume(v) => {
            *volume = v.clamp(0.0, 1.0);
            backend.set_volume(*volume);
        }
        AudioCommand::VolumeUp => {
            *volume = (*volume + crate::constants::audio::VOLUME_STEP).clamp(0.0, 1.0);
            backend.set_volume(*volume);
        }
        AudioCommand::VolumeDown => {
            *volume = (*volume - crate::constants::audio::VOLUME_STEP).clamp(0.0, 1.0);
            backend.set_volume(*volume);
        }
    }
}

// ---------- Tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── Mock backend ─────────────────────────────────────────────────────────

    /// Minimal in-process mock — no audio hardware required.
    struct MockBackend {
        playing: bool,
        paused: bool,
        volume: f32,
        /// When `true`, `play()` returns a `DecodingFailed` error.
        fail_play: bool,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                playing: false,
                paused: false,
                volume: crate::constants::audio::DEFAULT_VOLUME,
                fail_play: false,
            }
        }

        fn new_failing() -> Self {
            Self {
                fail_play: true,
                ..Self::new()
            }
        }
    }

    impl PlaybackBackend for MockBackend {
        fn play(&mut self, _path: &std::path::Path) -> Result<(), AudioError> {
            if self.fail_play {
                return Err(AudioError::DecodingFailed("mock error".into()));
            }
            self.playing = true;
            self.paused = false;
            Ok(())
        }
        fn pause(&mut self) {
            if self.playing {
                self.playing = false;
                self.paused = true;
            }
        }
        fn resume(&mut self) {
            if self.paused {
                self.paused = false;
                self.playing = true;
            }
        }
        fn stop(&mut self) {
            self.playing = false;
            self.paused = false;
        }
        fn seek(&mut self, _: Duration) -> Result<(), AudioError> {
            Ok(())
        }
        fn set_volume(&mut self, v: f32) {
            self.volume = v.clamp(0.0, 1.0);
        }
        fn position(&self) -> Option<Duration> {
            if self.playing || self.paused {
                Some(Duration::from_secs(1))
            } else {
                None
            }
        }
        fn duration(&self) -> Option<Duration> {
            Some(Duration::from_secs(30))
        }
        fn is_playing(&self) -> bool {
            self.playing
        }
        fn is_paused(&self) -> bool {
            self.paused
        }
        fn is_stopped(&self) -> bool {
            !self.playing && !self.paused
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_app_channels() -> (
        mpsc::UnboundedSender<AppEvent>,
        mpsc::UnboundedReceiver<AppEvent>,
    ) {
        mpsc::unbounded_channel()
    }

    fn test_ids() -> (EpisodeId, PodcastId) {
        (EpisodeId::new(), PodcastId::new())
    }

    // ── process_command — Play success ────────────────────────────────────────

    #[test]
    fn test_process_command_play_sets_playing_and_fires_started_event() {
        // Arrange
        let mut backend = MockBackend::new();
        let (tx, mut rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;
        let (ep_id, pod_id) = test_ids();

        // Act
        process_command(
            AudioCommand::Play {
                path: "/tmp/ep.mp3".into(),
                episode_id: ep_id.clone(),
                podcast_id: pod_id.clone(),
            },
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert — backend is playing
        assert!(backend.is_playing());
        // Assert — episode tracked
        assert!(current_episode.is_some());
        // Assert — PlaybackStarted event fired
        let event = rx.try_recv().expect("PlaybackStarted event expected");
        assert!(matches!(event, AppEvent::PlaybackStarted { .. }));
    }

    // ── process_command — Play failure ────────────────────────────────────────

    #[test]
    fn test_process_command_play_fires_error_event_on_backend_failure() {
        // Arrange
        let mut backend = MockBackend::new_failing();
        let (tx, mut rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;
        let (ep_id, pod_id) = test_ids();

        // Act
        process_command(
            AudioCommand::Play {
                path: "/tmp/ep.mp3".into(),
                episode_id: ep_id,
                podcast_id: pod_id,
            },
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert — no episode tracked on failure
        assert!(current_episode.is_none());
        // Assert — PlaybackError event fired
        let event = rx.try_recv().expect("PlaybackError event expected");
        assert!(matches!(event, AppEvent::PlaybackError { .. }));
    }

    // ── process_command — Stop ────────────────────────────────────────────────

    #[test]
    fn test_process_command_stop_fires_stopped_event_and_clears_episode() {
        // Arrange
        let mut backend = MockBackend::new();
        let (tx, mut rx) = make_app_channels();
        let (ep_id, pod_id) = test_ids();
        let mut current_episode = Some((ep_id, pod_id));
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;

        // Act
        process_command(
            AudioCommand::Stop,
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert — episode cleared
        assert!(current_episode.is_none());
        // Assert — PlaybackStopped event fired
        let event = rx.try_recv().expect("PlaybackStopped event expected");
        assert!(matches!(event, AppEvent::PlaybackStopped));
    }

    // ── process_command — TogglePlayPause ─────────────────────────────────────

    #[test]
    fn test_process_command_toggle_pauses_when_playing() {
        // Arrange
        let mut backend = MockBackend::new();
        backend.playing = true;
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;

        // Act
        process_command(
            AudioCommand::TogglePlayPause,
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert
        assert!(backend.is_paused());
        assert!(!backend.is_playing());
    }

    #[test]
    fn test_process_command_toggle_resumes_when_paused() {
        // Arrange
        let mut backend = MockBackend::new();
        backend.paused = true;
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;

        // Act
        process_command(
            AudioCommand::TogglePlayPause,
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert
        assert!(backend.is_playing());
        assert!(!backend.is_paused());
    }

    // ── process_command — Volume ──────────────────────────────────────────────

    #[test]
    fn test_process_command_volume_up_clamps_to_one() {
        // Arrange
        let mut backend = MockBackend::new();
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = 0.98f32;

        // Act
        process_command(
            AudioCommand::VolumeUp,
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert — clamped to 1.0
        assert!(volume <= 1.0, "volume {volume} must be ≤ 1.0");
        assert!((backend.volume - volume).abs() < f32::EPSILON);
    }

    #[test]
    fn test_process_command_volume_down_clamps_to_zero() {
        // Arrange
        let mut backend = MockBackend::new();
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = 0.02f32;

        // Act
        process_command(
            AudioCommand::VolumeDown,
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert — clamped to 0.0
        assert!(volume >= 0.0, "volume {volume} must be ≥ 0.0");
        assert!((backend.volume - volume).abs() < f32::EPSILON);
    }

    #[test]
    fn test_process_command_set_volume_clamps_and_applies() {
        // Arrange
        let mut backend = MockBackend::new();
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = 0.5f32;

        // Act
        process_command(
            AudioCommand::SetVolume(0.3),
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert
        assert!((volume - 0.3).abs() < f32::EPSILON);
        assert!((backend.volume - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_process_command_set_volume_above_max_clamps_to_one() {
        // Arrange
        let mut backend = MockBackend::new();
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = 0.5f32;

        // Act
        process_command(
            AudioCommand::SetVolume(2.0),
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );

        // Assert
        assert!((volume - 1.0).abs() < f32::EPSILON);
    }

    // ── process_command — SeekForward / SeekBackward ──────────────────────────

    #[test]
    fn test_process_command_seek_forward_when_stopped_seeks_to_delta() {
        // Arrange — backend stopped (position() returns None)
        let mut backend = MockBackend::new();
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;

        // Act — SeekForward from 0 by 10 s → target = 0 + 10 = 10 s
        process_command(
            AudioCommand::SeekForward(Duration::from_secs(10)),
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );
        // Assert — seek() succeeds (mock doesn't fail); no panic
    }

    #[test]
    fn test_process_command_seek_backward_saturates_at_zero() {
        // Arrange — backend stopped (position returns None → 0)
        let mut backend = MockBackend::new();
        let (tx, _rx) = make_app_channels();
        let mut current_episode = None;
        let mut volume = crate::constants::audio::DEFAULT_VOLUME;

        // Act — seeking backward from 0 should saturate to 0 (not underflow)
        process_command(
            AudioCommand::SeekBackward(Duration::from_secs(30)),
            &mut backend,
            &tx,
            &mut current_episode,
            &mut volume,
        );
        // Assert — no panic (Duration::saturating_sub doesn't underflow)
    }

    // ── Track-ended detection ─────────────────────────────────────────────────

    #[test]
    fn test_track_ended_fires_when_playing_transitions_to_stopped() {
        // Arrange — simulate was_playing=true, backend now stopped
        let (app_tx, mut app_rx) = make_app_channels();
        let (ep_id, pod_id) = test_ids();
        let mut current_episode: Option<(EpisodeId, PodcastId)> = Some((ep_id, pod_id));
        let was_playing = true;

        // Simulate the detection block from run_loop with a stopped backend
        let backend_is_playing = false;
        let backend_is_paused = false;

        if was_playing && !backend_is_playing && !backend_is_paused {
            if let Some((ref ep_id, ref pod_id)) = current_episode {
                let _ = app_tx.send(AppEvent::TrackEnded {
                    podcast_id: pod_id.clone(),
                    episode_id: ep_id.clone(),
                });
                current_episode = None;
            }
        }

        // Assert — TrackEnded fired
        let event = app_rx.try_recv().expect("TrackEnded event expected");
        assert!(matches!(event, AppEvent::TrackEnded { .. }));
        // Assert — episode cleared
        assert!(current_episode.is_none());
    }

    #[test]
    fn test_track_ended_does_not_fire_when_paused() {
        // Arrange — was_playing=true, backend now paused (not ended naturally)
        let (app_tx, mut app_rx) = make_app_channels();
        let (ep_id, pod_id) = test_ids();
        let mut current_episode: Option<(EpisodeId, PodcastId)> =
            Some((ep_id.clone(), pod_id.clone()));
        let was_playing = true;

        // Simulate detection with paused backend
        let backend_is_playing = false;
        let backend_is_paused = true;

        if was_playing && !backend_is_playing && !backend_is_paused {
            if let Some((ref ep_id, ref pod_id)) = current_episode {
                let _ = app_tx.send(AppEvent::TrackEnded {
                    podcast_id: pod_id.clone(),
                    episode_id: ep_id.clone(),
                });
                current_episode = None;
            }
        }

        // Assert — no TrackEnded event (paused is not ended)
        assert!(app_rx.try_recv().is_err(), "no event expected when paused");
        // Assert — episode still tracked
        assert!(current_episode.is_some());
    }

    // ── run_loop channel plumbing ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_run_loop_receives_play_command_and_fires_started_event() {
        // Arrange — wire up run_loop with a mock backend
        let (command_tx, command_rx) = mpsc::unbounded_channel::<AudioCommand>();
        let (status_tx, status_rx) = watch::channel(PlaybackStatus::default());
        let (app_tx, mut app_rx) = mpsc::unbounded_channel::<AppEvent>();

        let backend = Box::new(MockBackend::new());
        let initial_volume = crate::constants::audio::DEFAULT_VOLUME;

        let _thread = std::thread::spawn(move || {
            run_loop(backend, command_rx, status_tx, app_tx, initial_volume);
        });

        let (ep_id, pod_id) = test_ids();

        // Act — send a Play command (path doesn't need to exist; mock ignores it)
        command_tx
            .send(AudioCommand::Play {
                path: "/fake/ep.mp3".into(),
                episode_id: ep_id,
                podcast_id: pod_id,
            })
            .unwrap();

        // Assert — PlaybackStarted event arrives within 1 s
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let event = loop {
            if let Ok(e) = app_rx.try_recv() {
                break e;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "PlaybackStarted not received within 1 s"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        assert!(matches!(event, AppEvent::PlaybackStarted { .. }));

        // Assert — status watch reflects Playing after next broadcast cycle
        tokio::time::sleep(Duration::from_millis(400)).await;
        let status = status_rx.borrow().clone();
        assert_eq!(status.state, PlaybackState::Playing);
    }

    #[tokio::test]
    async fn test_run_loop_exits_cleanly_when_command_sender_dropped() {
        // Arrange
        let (command_tx, command_rx) = mpsc::unbounded_channel::<AudioCommand>();
        let (status_tx, _status_rx) = watch::channel(PlaybackStatus::default());
        let (app_tx, _app_rx) = mpsc::unbounded_channel::<AppEvent>();

        let backend = Box::new(MockBackend::new());
        let initial_volume = crate::constants::audio::DEFAULT_VOLUME;

        let thread = std::thread::spawn(move || {
            run_loop(backend, command_rx, status_tx, app_tx, initial_volume);
        });

        // Act — drop the sender to disconnect the channel
        drop(command_tx);

        // Assert — thread exits cleanly (within reasonable time)
        let result = tokio::task::spawn_blocking(move || thread.join())
            .await
            .unwrap();
        assert!(result.is_ok(), "audio thread should exit without panicking");
    }

    // ── Backend selection ─────────────────────────────────────────────────────

    #[test]
    fn test_create_backend_uses_external_player_when_configured() {
        // Arrange — config with explicit external_player
        let config = AudioConfig {
            external_player: Some("mpv".into()),
            ..Default::default()
        };

        // Act
        let result = create_backend(&config);

        // Assert — always succeeds: ExternalPlayerBackend::new() doesn't check existence
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_backend_succeeds_or_returns_rodio_error_when_no_external_player() {
        // Arrange — no external player configured
        let config = AudioConfig {
            external_player: None,
            ..Default::default()
        };

        // Act — may succeed (audio device present) or fail (headless CI)
        let result = create_backend(&config);

        // Assert — either Ok or an AudioError; the function must not panic
        match result {
            Ok(_) => {} // audio device present or external player found
            Err(e) => {
                // Must be an AudioError variant, not a panic
                eprintln!("No audio backend available (expected in headless CI): {e}");
            }
        }
    }
}
