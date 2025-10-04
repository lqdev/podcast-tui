//! Podcast subscription management

use crate::podcast::{Episode, FeedError, FeedParser, Podcast};
use crate::storage::{PodcastId, Storage};
use chrono::Utc;
use std::sync::Arc;

/// Subscription manager that handles podcast subscriptions
pub struct SubscriptionManager<S: Storage> {
    pub storage: Arc<S>,
    feed_parser: FeedParser,
}

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Podcast not found: {0}")]
    NotFound(String),

    #[error("Feed error: {0}")]
    Feed(#[from] FeedError),

    #[error("Podcast already subscribed: {0}")]
    AlreadySubscribed(String),

    #[error("No new episodes found")]
    NoNewEpisodes,
}

impl<S: Storage> SubscriptionManager<S> {
    /// Create a new subscription manager
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            feed_parser: FeedParser::new(),
        }
    }

    /// Get all subscribed podcasts
    pub async fn list_subscriptions(&self) -> Result<Vec<Podcast>, SubscriptionError> {
        let podcast_ids = self
            .storage
            .list_podcasts()
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        let mut podcasts = Vec::new();

        for id in podcast_ids {
            match self.storage.load_podcast(&id).await {
                Ok(podcast) => podcasts.push(podcast),
                Err(_) => continue,
            }
        }

        // Sort by last updated (newest first)
        podcasts.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
        Ok(podcasts)
    }

    /// Get podcast by ID
    pub async fn get_podcast(&self, podcast_id: &PodcastId) -> Result<Podcast, SubscriptionError> {
        self.storage
            .load_podcast(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))
    }

    /// Subscribe to a new podcast by feed URL
    pub async fn subscribe(&self, feed_url: &str) -> Result<Podcast, SubscriptionError> {
        // Check if already subscribed (prevent duplicates)
        let podcast_id = PodcastId::from_url(feed_url);
        let exists = self
            .storage
            .podcast_exists(&podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        if exists {
            return Err(SubscriptionError::AlreadySubscribed(feed_url.to_string()));
        }

        // Parse the feed and create podcast
        let podcast = self.feed_parser.parse_feed(feed_url).await?;

        // Get episodes for the podcast
        let episodes = self.feed_parser.get_episodes(feed_url, &podcast.id).await?;

        // Save the podcast
        self.storage
            .save_podcast(&podcast)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        // Save all episodes
        for episode in episodes {
            self.storage
                .save_episode(&podcast.id, &episode)
                .await
                .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        }

        Ok(podcast)
    }

    /// Unsubscribe from a podcast
    pub async fn unsubscribe(&self, podcast_id: &PodcastId) -> Result<(), SubscriptionError> {
        // Check if podcast exists
        let exists = self
            .storage
            .podcast_exists(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        if !exists {
            return Err(SubscriptionError::NotFound(podcast_id.to_string()));
        }

        // Delete the podcast (this should cascade to episodes in the storage implementation)
        self.storage
            .delete_podcast(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Refresh a podcast feed and get new episodes
    pub async fn refresh_feed(
        &self,
        podcast_id: &PodcastId,
    ) -> Result<Vec<Episode>, SubscriptionError> {
        // Load the podcast
        let mut podcast = self.get_podcast(podcast_id).await?;

        // Get episodes from the feed
        let feed_episodes = self
            .feed_parser
            .get_episodes(&podcast.url, podcast_id)
            .await?;

        // Load existing episodes to check for duplicates
        let existing_episodes = self
            .storage
            .load_episodes(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        // Filter out episodes we already have
        let mut new_episodes = Vec::new();
        for episode in feed_episodes {
            // Check if episode already exists using multiple strategies
            let is_duplicate = existing_episodes.iter().any(|existing_episode| {
                // Strategy 1: Compare deterministic IDs (based on GUID)
                if episode.id == existing_episode.id {
                    return true;
                }

                // Strategy 2: Compare GUIDs directly if both have them
                if let (Some(ref episode_guid), Some(ref existing_guid)) =
                    (&episode.guid, &existing_episode.guid)
                {
                    if episode_guid == existing_guid {
                        return true;
                    }
                }

                // Strategy 3: Compare audio URLs if both have them and they're not empty
                if !episode.audio_url.is_empty()
                    && !existing_episode.audio_url.is_empty()
                    && episode.audio_url == existing_episode.audio_url
                {
                    return true;
                }

                // Strategy 4: Compare titles and published dates (within 1 minute)
                if episode.title == existing_episode.title
                    && (episode.published - existing_episode.published)
                        .num_seconds()
                        .abs()
                        < 60
                {
                    return true;
                }

                false
            });

            if !is_duplicate {
                new_episodes.push(episode);
            }
        }

        // Save new episodes
        for episode in &new_episodes {
            self.storage
                .save_episode(podcast_id, episode)
                .await
                .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        }

        // Update podcast's last_updated timestamp
        podcast.last_updated = Utc::now();
        self.storage
            .save_podcast(&podcast)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        Ok(new_episodes)
    }

    /// Refresh all subscribed podcasts
    pub async fn refresh_all(&self) -> Result<usize, SubscriptionError> {
        let podcasts = self.list_subscriptions().await?;
        let mut total_new_episodes = 0;

        for podcast in podcasts {
            match self.refresh_feed(&podcast.id).await {
                Ok(new_episodes) => {
                    total_new_episodes += new_episodes.len();
                }
                Err(e) => {
                    // Log error but continue with other podcasts
                    eprintln!("Failed to refresh {}: {}", podcast.title, e);
                }
            }
        }

        Ok(total_new_episodes)
    }

    /// Check if a podcast is already subscribed
    pub async fn is_subscribed(&self, feed_url: &str) -> bool {
        let podcast_id = PodcastId::from_url(feed_url);
        self.storage
            .podcast_exists(&podcast_id)
            .await
            .unwrap_or(false)
    }

    /// Get subscription count
    pub async fn subscription_count(&self) -> Result<usize, SubscriptionError> {
        let podcasts = self
            .storage
            .list_podcasts()
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        Ok(podcasts.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::JsonStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_subscription_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        let manager = SubscriptionManager::new(storage);
        let count = manager.subscription_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        let manager = SubscriptionManager::new(storage);

        // Test is_subscribed for non-existent podcast
        let subscribed = manager.is_subscribed("https://example.com/feed.xml").await;
        assert!(!subscribed);
    }
}
