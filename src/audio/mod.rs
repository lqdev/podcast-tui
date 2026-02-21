//! Audio playback module — trait abstraction, backends, and coordinator.
//!
//! Architecture:
//! - [`PlaybackBackend`]: synchronous interface, implementations run on a dedicated `std::thread`
//! - [`AudioError`]: domain error type (thiserror)
//! - [`AudioCommand`]: commands from the UI → AudioManager
//! - [`PlaybackStatus`]: broadcast state from AudioManager → UI

use std::path::Path;
use std::time::Duration;

use crate::storage::{EpisodeId, PodcastId};

/// Errors that can occur during audio playback.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio device not found")]
    DeviceNotFound,
    #[error("Failed to decode audio file: {0}")]
    DecodingFailed(String),
    #[error("Seek failed: {0}")]
    SeekFailed(String),
    #[error("External player not found: {0}")]
    ExternalPlayerNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Current playback state.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

/// Commands sent from the UI to the `AudioManager`.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play {
        path: std::path::PathBuf,
        episode_id: EpisodeId,
        podcast_id: PodcastId,
    },
    Pause,
    Resume,
    TogglePlayPause,
    Stop,
    SeekForward(Duration),
    SeekBackward(Duration),
    SetVolume(f32),
    VolumeUp,
    VolumeDown,
}

/// Playback status broadcast from `AudioManager` to the UI.
#[derive(Debug, Clone)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
    pub episode_id: Option<EpisodeId>,
    pub podcast_id: Option<PodcastId>,
    pub position: Option<Duration>,
    pub duration: Option<Duration>,
    pub volume: f32,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self {
            state: PlaybackState::Stopped,
            episode_id: None,
            podcast_id: None,
            position: None,
            duration: None,
            volume: crate::constants::audio::DEFAULT_VOLUME,
        }
    }
}

/// Trait abstracting over different audio playback backends.
///
/// Implementations are expected to run synchronously on a dedicated `std::thread`,
/// not on the tokio async executor, to avoid cpal/tokio deadlock risk.
pub trait PlaybackBackend: Send {
    fn play(&mut self, path: &Path) -> Result<(), AudioError>;
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn seek(&mut self, position: Duration) -> Result<(), AudioError>;
    fn set_volume(&mut self, volume: f32);
    fn position(&self) -> Option<Duration>;
    fn duration(&self) -> Option<Duration>;
    fn is_playing(&self) -> bool;
    fn is_paused(&self) -> bool;
    fn is_stopped(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_status_default_is_stopped_at_default_volume() {
        // Arrange / Act
        let status = PlaybackStatus::default();

        // Assert
        assert_eq!(status.state, PlaybackState::Stopped);
        assert_eq!(status.volume, crate::constants::audio::DEFAULT_VOLUME);
        assert!(status.episode_id.is_none());
        assert!(status.podcast_id.is_none());
        assert!(status.position.is_none());
        assert!(status.duration.is_none());
    }

    #[test]
    fn test_audio_error_display_device_not_found() {
        // Arrange / Act
        let err = AudioError::DeviceNotFound;

        // Assert
        assert_eq!(err.to_string(), "Audio device not found");
    }

    #[test]
    fn test_audio_error_display_decoding_failed() {
        // Arrange / Act
        let err = AudioError::DecodingFailed("bad mp3".to_string());

        // Assert
        assert_eq!(err.to_string(), "Failed to decode audio file: bad mp3");
    }

    #[test]
    fn test_audio_error_display_seek_failed() {
        // Arrange / Act
        let err = AudioError::SeekFailed("sink error".to_string());

        // Assert
        assert_eq!(err.to_string(), "Seek failed: sink error");
    }

    #[test]
    fn test_audio_error_display_external_player_not_found() {
        // Arrange / Act
        let err = AudioError::ExternalPlayerNotFound("vlc".to_string());

        // Assert
        assert_eq!(err.to_string(), "External player not found: vlc");
    }
}
