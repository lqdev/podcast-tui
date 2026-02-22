//! Integration tests for the audio playback subsystem.
//!
//! Tests that require a real audio output device are gated with `#[ignore]` so
//! `cargo test` passes in headless CI environments.  Run with
//! `cargo test --test test_audio -- --ignored` to include them locally.

use std::path::Path;
use std::time::Duration;

use podcast_tui::audio::manager::AudioManager;
use podcast_tui::audio::{AudioCommand, PlaybackState, PlaybackStatus};
use podcast_tui::config::AudioConfig;
use podcast_tui::storage::{EpisodeId, PodcastId};
use podcast_tui::ui::events::AppEvent;
use tokio::sync::{mpsc, watch};

/// Generous timeout for status watch channel updates in tests.
/// The run_loop broadcasts at ~4 Hz (250 ms), so any command result should
/// appear well within this window — even on slow CI runners.
const STATUS_CHANGE_TIMEOUT: Duration = Duration::from_secs(3);

/// A trivially-available external "player" command for CI-safe tests.
/// On Unix, `echo` is a standalone binary. On Windows, `echo` is a
/// shell built-in so `std::process::Command` cannot spawn it; we use
/// `cmd.exe` instead, which is a real executable. The external player
/// backend passes the file path as the sole argument (`cmd <path>`),
/// which exits immediately — sufficient for CI lifecycle tests.
#[cfg(unix)]
fn ci_external_player() -> String {
    "echo".into()
}

#[cfg(windows)]
fn ci_external_player() -> String {
    "cmd".into()
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Write a minimal silent PCM WAV file (44 100 Hz · mono · 16-bit).
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
    bytes.resize(44 + data_size as usize, 0);

    std::fs::write(path, &bytes).expect("failed to write test WAV");
}

/// Wait for a specific `AppEvent` variant within a deadline.
/// Returns the matched event or panics with a descriptive message.
async fn wait_for_event<F>(
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    timeout: Duration,
    description: &str,
    predicate: F,
) -> AppEvent
where
    F: Fn(&AppEvent) -> bool,
{
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Some(event)) if predicate(&event) => return event,
            Ok(Some(_)) => continue, // skip non-matching events
            Ok(None) => panic!("channel closed while waiting for {description}"),
            Err(_) => panic!("{description} not received within {timeout:?}"),
        }
    }
}

/// Wait for the status watch to satisfy `predicate`, with a timeout.
///
/// Polls `watch::Receiver::changed()` in a loop, checking each new value
/// against the predicate.  Returns once matched; panics on timeout.
async fn wait_for_status<F>(
    rx: &mut watch::Receiver<PlaybackStatus>,
    timeout: Duration,
    description: &str,
    predicate: F,
) where
    F: Fn(&PlaybackStatus) -> bool,
{
    if predicate(&rx.borrow_and_update()) {
        return;
    }
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::time::timeout_at(deadline, rx.changed()).await {
            Ok(Ok(())) => {
                if predicate(&rx.borrow_and_update()) {
                    return;
                }
            }
            Ok(Err(_)) => panic!("status channel closed while waiting for {description}"),
            Err(_) => panic!("{description}: condition not met within {timeout:?}"),
        }
    }
}

// ── CI-safe tests (external player backend) ─────────────────────────────────

#[tokio::test]
async fn test_audio_manager_play_and_stop_lifecycle_with_external_player() {
    // Arrange — use a trivially-available external "player"
    let dir = tempfile::TempDir::new().expect("temp dir");
    let wav_path = dir.path().join("test.wav");
    write_test_wav(&wav_path, 1);

    let config = AudioConfig {
        external_player: Some(ci_external_player()),
        ..Default::default()
    };
    let (app_tx, mut app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx).expect("AudioManager should init");
    let mut status_rx = manager.subscribe();

    // Assert — initial status is Stopped
    assert_eq!(status_rx.borrow().state, PlaybackState::Stopped);

    let ep_id = EpisodeId::new();
    let pod_id = PodcastId::new();

    // Act — Play
    manager.send(AudioCommand::Play {
        path: wav_path,
        episode_id: ep_id.clone(),
        podcast_id: pod_id.clone(),
    });

    // Assert — PlaybackStarted event arrives
    let event = wait_for_event(
        &mut app_rx,
        Duration::from_secs(2),
        "PlaybackStarted",
        |e| matches!(e, AppEvent::PlaybackStarted { .. }),
    )
    .await;
    assert!(matches!(event, AppEvent::PlaybackStarted { .. }));

    // Act — Stop
    manager.send(AudioCommand::Stop);

    // Assert — PlaybackStopped event arrives
    let event = wait_for_event(
        &mut app_rx,
        Duration::from_secs(2),
        "PlaybackStopped",
        |e| matches!(e, AppEvent::PlaybackStopped),
    )
    .await;
    assert!(matches!(event, AppEvent::PlaybackStopped));

    // Assert — status watch reflects Stopped after broadcast cycle
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "state → Stopped after stop",
        |s| s.state == PlaybackState::Stopped,
    )
    .await;
}

#[tokio::test]
async fn test_audio_manager_play_error_fires_playback_error_event() {
    // Arrange — use a non-existent player to trigger a spawn error
    let config = AudioConfig {
        external_player: Some("__nonexistent_player_abc123__".into()),
        ..Default::default()
    };
    let (app_tx, mut app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx).expect("AudioManager should init");

    let ep_id = EpisodeId::new();
    let pod_id = PodcastId::new();

    // Act — Play with a non-existent player → should fire PlaybackError
    let dir = tempfile::TempDir::new().expect("temp dir");
    let fake_path = dir.path().join("fake.mp3");
    manager.send(AudioCommand::Play {
        path: fake_path,
        episode_id: ep_id,
        podcast_id: pod_id,
    });

    // Assert — PlaybackError event arrives
    let event = wait_for_event(&mut app_rx, Duration::from_secs(2), "PlaybackError", |e| {
        matches!(e, AppEvent::PlaybackError { .. })
    })
    .await;
    assert!(matches!(event, AppEvent::PlaybackError { .. }));
}

#[tokio::test]
async fn test_audio_manager_subscribe_returns_initial_stopped_status() {
    // Arrange
    let config = AudioConfig {
        external_player: Some(ci_external_player()),
        volume: 0.6,
        ..Default::default()
    };
    let (app_tx, _app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx).expect("AudioManager should init");

    // Act
    let status_rx = manager.subscribe();
    let status = status_rx.borrow().clone();

    // Assert — initial status reflects config and stopped state
    assert_eq!(status.state, PlaybackState::Stopped);
    assert!((status.volume - 0.6).abs() < f32::EPSILON);
    assert!(status.episode_id.is_none());
    assert!(status.podcast_id.is_none());
    assert!(status.position.is_none());
}

#[tokio::test]
async fn test_audio_manager_clean_shutdown_on_drop() {
    // Arrange
    let config = AudioConfig {
        external_player: Some(ci_external_player()),
        ..Default::default()
    };
    let (app_tx, mut app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx).expect("AudioManager should init");

    // Act — drop triggers thread shutdown
    drop(manager);

    // Assert — drain app_rx until None (proves audio thread dropped its sender)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        match tokio::time::timeout_at(deadline, app_rx.recv()).await {
            Ok(None) => break,       // channel closed = thread exited
            Ok(Some(_)) => continue, // drain remaining events
            Err(_) => panic!("audio thread did not shut down within 2s"),
        }
    }
}

#[tokio::test]
async fn test_audio_manager_volume_commands_update_status() {
    // Arrange — use CI-safe external player
    let config = AudioConfig {
        external_player: Some(ci_external_player()),
        volume: 0.5,
        ..Default::default()
    };
    let (app_tx, _app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx).expect("AudioManager should init");
    let mut status_rx = manager.subscribe();

    // Act — SetVolume
    manager.send(AudioCommand::SetVolume(0.3));
    // Wait for the run_loop to process the command and broadcast
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "volume → 0.3",
        |s| (s.volume - 0.3).abs() < f32::EPSILON,
    )
    .await;

    // Assert
    let status = status_rx.borrow().clone();
    assert!(
        (status.volume - 0.3).abs() < f32::EPSILON,
        "volume should be 0.3, got {}",
        status.volume
    );

    // Act — VolumeUp
    manager.send(AudioCommand::VolumeUp);
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "volume → ~0.35 after VolumeUp",
        |s| (s.volume - 0.35).abs() < 0.01,
    )
    .await;

    // Assert — volume increased by VOLUME_STEP (0.05)
    let status = status_rx.borrow().clone();
    assert!(
        (status.volume - 0.35).abs() < 0.01,
        "volume should be ~0.35, got {}",
        status.volume
    );
}

// ── Hardware-dependent tests (require audio device) ──────────────────────────

#[tokio::test]
#[ignore] // Requires audio device — run with `cargo test -- --ignored`
async fn test_full_playback_lifecycle_play_pause_resume_stop() {
    // Arrange
    let dir = tempfile::TempDir::new().expect("temp dir");
    let wav_path = dir.path().join("test.wav");
    write_test_wav(&wav_path, 3); // 3-second WAV

    let config = AudioConfig::default();
    let (app_tx, mut app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx)
        .expect("AudioManager should init — this test requires an audio device");
    let mut status_rx = manager.subscribe();

    let ep_id = EpisodeId::new();
    let pod_id = PodcastId::new();

    // ── Play ────────────────────────────────────────────────────────────
    manager.send(AudioCommand::Play {
        path: wav_path,
        episode_id: ep_id.clone(),
        podcast_id: pod_id.clone(),
    });
    wait_for_event(
        &mut app_rx,
        Duration::from_secs(2),
        "PlaybackStarted",
        |e| matches!(e, AppEvent::PlaybackStarted { .. }),
    )
    .await;
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "state → Playing after play",
        |s| s.state == PlaybackState::Playing,
    )
    .await;

    // ── Pause ───────────────────────────────────────────────────────────
    manager.send(AudioCommand::Pause);
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "state → Paused after pause",
        |s| s.state == PlaybackState::Paused,
    )
    .await;

    // ── Resume ──────────────────────────────────────────────────────────
    manager.send(AudioCommand::Resume);
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "state → Playing after resume",
        |s| s.state == PlaybackState::Playing,
    )
    .await;

    // ── Stop ────────────────────────────────────────────────────────────
    manager.send(AudioCommand::Stop);
    wait_for_event(
        &mut app_rx,
        Duration::from_secs(2),
        "PlaybackStopped",
        |e| matches!(e, AppEvent::PlaybackStopped),
    )
    .await;
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "state → Stopped with episode cleared",
        |s| s.state == PlaybackState::Stopped && s.episode_id.is_none(),
    )
    .await;
}

#[tokio::test]
#[ignore] // Requires audio device — run with `cargo test -- --ignored`
async fn test_full_playback_position_advances_and_resets() {
    // Arrange
    let dir = tempfile::TempDir::new().expect("temp dir");
    let wav_path = dir.path().join("test.wav");
    write_test_wav(&wav_path, 5); // 5-second WAV

    let config = AudioConfig::default();
    let (app_tx, mut app_rx) = mpsc::unbounded_channel::<AppEvent>();
    let manager = AudioManager::new(&config, app_tx)
        .expect("AudioManager should init — this test requires an audio device");
    let mut status_rx = manager.subscribe();

    let ep_id = EpisodeId::new();
    let pod_id = PodcastId::new();

    // Act — Play and wait for position to advance
    manager.send(AudioCommand::Play {
        path: wav_path,
        episode_id: ep_id,
        podcast_id: pod_id,
    });
    wait_for_event(
        &mut app_rx,
        Duration::from_secs(2),
        "PlaybackStarted",
        |e| matches!(e, AppEvent::PlaybackStarted { .. }),
    )
    .await;
    // Wait for real audio to play and position to advance
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "position advanced past 200 ms",
        |s| s.position.is_some_and(|p| p >= Duration::from_millis(200)),
    )
    .await;

    // Assert — position should have advanced
    let status = status_rx.borrow().clone();
    assert!(
        status.position.is_some(),
        "position should be Some while playing"
    );
    let pos = status.position.expect("position is Some");
    assert!(
        pos >= Duration::from_millis(200),
        "position should have advanced, got {pos:?}"
    );

    // Act — Stop
    manager.send(AudioCommand::Stop);
    wait_for_status(
        &mut status_rx,
        STATUS_CHANGE_TIMEOUT,
        "state → Stopped with position cleared",
        |s| s.state == PlaybackState::Stopped && s.position.is_none(),
    )
    .await;

    // Assert — position resets to None when stopped
    let status = status_rx.borrow().clone();
    assert_eq!(status.state, PlaybackState::Stopped);
    assert!(
        status.position.is_none(),
        "position should be None after stop"
    );
}
