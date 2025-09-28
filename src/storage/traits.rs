use crate::podcast::{Episode, Podcast};
use crate::storage::models::{EpisodeId, PodcastId, StorageError};
use anyhow::Result;
use async_trait::async_trait;

/// Abstract storage trait for podcast data persistence
///
/// This trait provides an abstraction layer over data storage,
/// allowing for different backend implementations (JSON, SQLite, etc.)
/// while maintaining a consistent interface for the application logic.
#[async_trait]
pub trait Storage: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // Podcast operations
    async fn save_podcast(&self, podcast: &Podcast) -> Result<(), Self::Error>;
    async fn load_podcast(&self, id: &PodcastId) -> Result<Podcast, Self::Error>;
    async fn delete_podcast(&self, id: &PodcastId) -> Result<(), Self::Error>;
    async fn list_podcasts(&self) -> Result<Vec<PodcastId>, Self::Error>;
    async fn podcast_exists(&self, id: &PodcastId) -> Result<bool, Self::Error>;

    // Episode operations
    async fn save_episode(
        &self,
        podcast_id: &PodcastId,
        episode: &Episode,
    ) -> Result<(), Self::Error>;
    async fn load_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<Episode, Self::Error>;
    async fn delete_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<(), Self::Error>;
    async fn list_episodes(&self, podcast_id: &PodcastId) -> Result<Vec<EpisodeId>, Self::Error>;
    async fn episode_exists(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<bool, Self::Error>;

    // Batch operations for performance
    async fn save_episodes(
        &self,
        podcast_id: &PodcastId,
        episodes: &[Episode],
    ) -> Result<(), Self::Error>;
    async fn load_episodes(&self, podcast_id: &PodcastId) -> Result<Vec<Episode>, Self::Error>;

    // Storage management
    async fn initialize(&self) -> Result<(), Self::Error>;
    async fn backup(&self, path: &std::path::Path) -> Result<(), Self::Error>;
    async fn restore(&self, path: &std::path::Path) -> Result<(), Self::Error>;
    async fn cleanup(&self) -> Result<(), Self::Error>;
}

/// Convenience type for boxed storage implementations
pub type BoxedStorage = Box<dyn Storage<Error = StorageError>>;
