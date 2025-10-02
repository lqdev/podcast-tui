//! Podcast subscription management

use std::sync::Arc;
use crate::podcast::Podcast;
use crate::storage::{PodcastId, Storage};

/// Subscription manager that handles podcast subscriptions
pub struct SubscriptionManager<S: Storage> {
    storage: Arc<S>,
}

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Podcast not found: {0}")]
    NotFound(String),
}

impl<S: Storage> SubscriptionManager<S> {
    /// Create a new subscription manager
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Get all subscribed podcasts
    pub async fn list_subscriptions(&self) -> Result<Vec<Podcast>, SubscriptionError> {
        let podcast_ids = self.storage
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
}
