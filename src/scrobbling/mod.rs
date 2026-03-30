//! ListenBrainz-compatible podcast scrobbling.
//!
//! This module sends listen events to a self-hosted podcast-scrobbler server.
//! It is entirely additive and non-breaking — when disabled, a [`NoopScrobbler`]
//! is used that silently discards all events.

pub mod circuit_breaker;
pub mod client;
pub mod models;
pub mod noop;
pub mod queue;

use async_trait::async_trait;

pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use client::ListenBrainzClient;
pub use noop::NoopScrobbler;
pub use queue::PersistentRetryQueue;

/// A scrobble-worthy playback event.
///
/// Built from the existing `Podcast` + `Episode` models and the current
/// playback position reported by `PlaybackStatus`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScrobbleEvent {
    pub podcast_title: String,
    pub episode_title: String,
    pub feed_url: Option<String>,
    pub episode_guid: Option<String>,
    /// Episode total duration in milliseconds (from `Episode.duration` × 1000)
    pub duration_ms: Option<u64>,
    /// Current playback position in milliseconds
    pub position_ms: u64,
    /// Unix timestamp (seconds) when this listen occurred
    pub listened_at: i64,
}

/// Errors that can occur during scrobbling.
#[derive(Debug, thiserror::Error)]
pub enum ScrobblerError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Server returned error: {status} — {body}")]
    ServerError { status: u16, body: String },
    #[error("Circuit breaker is open")]
    CircuitOpen,
    #[error("Queue I/O error: {0}")]
    QueueIo(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Trait abstracting over scrobbling backends.
///
/// Implementations:
/// - [`ListenBrainzClient`]: real HTTP client with retry queue + circuit breaker
/// - [`NoopScrobbler`]: silent discard (used when scrobbling is disabled)
#[async_trait]
pub trait Scrobbler: Send + Sync {
    /// Notify the server that playback has started (ephemeral, not stored in history).
    async fn playing_now(&self, event: &ScrobbleEvent) -> Result<(), ScrobblerError>;

    /// Submit a completed listen (permanent history record).
    async fn scrobble(&self, event: &ScrobbleEvent) -> Result<(), ScrobblerError>;

    /// Drain pending scrobbles from the retry queue. Returns count of successfully sent.
    async fn flush_pending(&self) -> Result<usize, ScrobblerError>;

    /// Number of scrobbles waiting in the retry queue.
    fn pending_count(&self) -> usize;

    /// Current circuit breaker state.
    fn circuit_state(&self) -> CircuitState;
}

/// Construct the appropriate scrobbler based on config.
///
/// Returns `ListenBrainzClient` when enabled + endpoint configured,
/// `NoopScrobbler` otherwise.
pub fn create_scrobbler(
    config: &crate::config::ScrobblingConfig,
    data_dir: &std::path::Path,
) -> Box<dyn Scrobbler> {
    if config.enabled {
        if let Some(ref endpoint) = config.endpoint {
            match ListenBrainzClient::new(config, endpoint, data_dir) {
                Ok(client) => return Box::new(client),
                Err(e) => {
                    eprintln!(
                        "[scrobbling] Failed to initialize client: {e}. Falling back to noop."
                    );
                }
            }
        } else {
            eprintln!("[scrobbling] Enabled but no endpoint configured. Using noop.");
        }
    }
    Box::new(NoopScrobbler)
}

/// Build a [`ScrobbleEvent`] from podcast/episode metadata.
///
/// Returns `None` if either the podcast or episode cannot be loaded.
pub async fn build_scrobble_event(
    storage: &crate::storage::json::JsonStorage,
    podcast_id: &crate::storage::PodcastId,
    episode_id: &crate::storage::EpisodeId,
    position_ms: u64,
) -> Option<ScrobbleEvent> {
    use crate::storage::Storage;

    let podcast = storage.load_podcast(podcast_id).await.ok()?;
    let episode = storage.load_episode(podcast_id, episode_id).await.ok()?;
    Some(ScrobbleEvent {
        podcast_title: podcast.title,
        episode_title: episode.title,
        feed_url: Some(podcast.url),
        episode_guid: episode.guid,
        duration_ms: episode.duration.map(|d| d as u64 * 1000),
        position_ms,
        listened_at: chrono::Utc::now().timestamp(),
    })
}

/// Check if the scrobble threshold is met (BOTH conditions must be true).
pub fn meets_scrobble_threshold(
    event: &ScrobbleEvent,
    config: &crate::config::ScrobblingConfig,
) -> bool {
    let position_secs = event.position_ms / 1000;
    let meets_time = position_secs >= config.min_listen_seconds as u64;

    let meets_percent = match event.duration_ms {
        Some(d) if d > 0 => {
            let percent = (event.position_ms as f64 / d as f64) * 100.0;
            percent >= config.min_listen_percent as f64
        }
        // If duration unknown, only enforce time threshold
        _ => true,
    };

    meets_time && meets_percent
}
