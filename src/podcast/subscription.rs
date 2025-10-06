//! Podcast subscription management

use crate::download::DownloadManager;
use crate::podcast::{Episode, FeedError, FeedParser, Podcast};
use crate::storage::{PodcastId, Storage};
use chrono::Utc;
use std::sync::Arc;

/// Subscription manager that handles podcast subscriptions
pub struct SubscriptionManager<S: Storage> {
    pub storage: Arc<S>,
    feed_parser: FeedParser,
    download_manager: Option<Arc<DownloadManager<S>>>,
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

    #[error("OPML error: {0}")]
    Opml(#[from] crate::podcast::OpmlError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl<S: Storage> SubscriptionManager<S> {
    /// Create a new subscription manager
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            feed_parser: FeedParser::new(),
            download_manager: None,
        }
    }

    /// Create a new subscription manager with download manager for automatic cleanup
    pub fn with_download_manager(
        storage: Arc<S>,
        download_manager: Arc<DownloadManager<S>>,
    ) -> Self {
        Self {
            storage,
            feed_parser: FeedParser::new(),
            download_manager: Some(download_manager),
        }
    }

    /// Set the download manager for automatic cleanup during unsubscribe
    pub fn set_download_manager(&mut self, download_manager: Arc<DownloadManager<S>>) {
        self.download_manager = Some(download_manager);
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
    /// This will also delete all downloaded episodes for the podcast
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

        // Delete all downloaded episodes for this podcast if download manager is available
        if let Some(ref download_manager) = self.download_manager {
            if let Err(e) = download_manager.delete_podcast_downloads(podcast_id).await {
                // Log the error but don't fail the unsubscribe operation
                eprintln!("Warning: Failed to delete some downloaded episodes: {}", e);
            }
        }

        // Delete the podcast (this should cascade to episodes in the storage implementation)
        self.storage
            .delete_podcast(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Refresh a podcast feed and get new episodes
    /// If hard_refresh is true, existing episodes will be updated with new data
    pub async fn refresh_feed(
        &self,
        podcast_id: &PodcastId,
    ) -> Result<Vec<Episode>, SubscriptionError> {
        self.refresh_feed_with_options(podcast_id, false).await
    }

    /// Refresh a podcast feed with options
    /// If hard_refresh is true, existing episodes will be updated with new data
    pub async fn refresh_feed_with_options(
        &self,
        podcast_id: &PodcastId,
        hard_refresh: bool,
    ) -> Result<Vec<Episode>, SubscriptionError> {
        // Load the podcast
        let mut podcast = self.get_podcast(podcast_id).await?;

        // Get episodes from the feed
        let feed_episodes = self
            .feed_parser
            .get_episodes(&podcast.url, podcast_id)
            .await?;

        // Assign track numbers to episodes
        let episodes_with_tracks = self
            .assign_track_numbers(podcast_id, feed_episodes, hard_refresh)
            .await?;

        // Load existing episodes to check for duplicates
        let existing_episodes = self
            .storage
            .load_episodes(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        // Filter out episodes we already have, or update existing ones if hard_refresh
        let mut new_episodes = Vec::new();
        let mut updated_episodes = Vec::new();

        for episode in episodes_with_tracks {
            // Check if episode already exists using multiple strategies
            let existing_episode = existing_episodes.iter().find(|existing_episode| {
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

            if let Some(existing) = existing_episode {
                if hard_refresh {
                    // Update existing episode with new data (preserving user-specific fields)
                    let mut updated_episode = episode.clone();
                    updated_episode.id = existing.id.clone(); // Keep the same ID
                    updated_episode.status = existing.status.clone(); // Preserve download status
                    updated_episode.local_path = existing.local_path.clone(); // Preserve local file
                    updated_episode.last_played_position = existing.last_played_position; // Preserve playback position
                    updated_episode.play_count = existing.play_count; // Preserve play count
                    updated_episode.notes = existing.notes.clone(); // Preserve user notes

                    updated_episodes.push(updated_episode);
                }
                // If not hard refresh, skip existing episodes (current behavior)
            } else {
                // Truly new episode
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

        // Save updated episodes (for hard refresh)
        for episode in &updated_episodes {
            self.storage
                .save_episode(podcast_id, episode)
                .await
                .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
        }

        // Combine new and updated episodes for return value
        let mut all_changes = new_episodes.clone();
        all_changes.extend(updated_episodes);

        // Update podcast's last_updated timestamp
        podcast.last_updated = Utc::now();
        self.storage
            .save_podcast(&podcast)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        Ok(all_changes)
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
                Err(_e) => {
                    // Log error but continue with other podcasts
                    // TODO: Add proper error reporting mechanism
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

    /// Assign track numbers to episodes based on chronological order
    async fn assign_track_numbers(
        &self,
        podcast_id: &PodcastId,
        mut new_episodes: Vec<Episode>,
        hard_refresh: bool,
    ) -> Result<Vec<Episode>, SubscriptionError> {
        if hard_refresh {
            // Renumber all episodes for consistency
            let existing_episodes = self
                .storage
                .load_episodes(podcast_id)
                .await
                .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

            // Combine and deduplicate by ID
            let mut all_episodes = existing_episodes;
            for new_ep in new_episodes {
                if !all_episodes.iter().any(|existing| existing.id == new_ep.id) {
                    all_episodes.push(new_ep);
                }
            }

            // Sort chronologically (oldest first for track numbering)
            all_episodes.sort_by_key(|e| e.published);

            // Assign sequential track numbers
            for (index, episode) in all_episodes.iter_mut().enumerate() {
                episode.episode_number = Some((index + 1) as u32);
            }

            new_episodes = all_episodes;
        } else {
            // Only assign track numbers to new episodes
            let existing_episodes = self
                .storage
                .load_episodes(podcast_id)
                .await
                .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

            // Find highest existing track number
            let max_track = existing_episodes
                .iter()
                .filter_map(|e| e.episode_number)
                .max()
                .unwrap_or(0);

            // Sort new episodes chronologically (oldest first)
            new_episodes.sort_by_key(|e| e.published);

            // Assign sequential track numbers starting after max
            for (index, episode) in new_episodes.iter_mut().enumerate() {
                episode.episode_number = Some(max_track + (index + 1) as u32);
            }
        }

        Ok(new_episodes)
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

    /// Import podcasts from OPML file or URL
    ///
    /// Non-destructive import that skips duplicates and processes feeds sequentially.
    /// Returns detailed statistics about the import operation.
    ///
    /// # Arguments
    ///
    /// * `source` - File path or HTTP(S) URL to OPML file
    /// * `progress_callback` - Callback function for progress updates
    ///
    /// # Returns
    ///
    /// ImportResult with statistics (total, imported, skipped, failed)
    pub async fn import_opml<F>(
        &self,
        source: &str,
        progress_callback: F,
    ) -> Result<(crate::podcast::ImportResult, String), SubscriptionError>
    where
        F: Fn(String) + Send + Sync,
    {
        use crate::podcast::{FailedImport, ImportResult, OpmlParser};
        use chrono::Local;

        progress_callback("Validating OPML file...".to_string());

        // Parse and validate OPML
        let parser = OpmlParser::new();
        let document = parser.parse(source).await?;

        let total_feeds = document.outlines.len();
        progress_callback(format!("Found {} feeds in OPML", total_feeds));

        // Create log file
        let log_dir = dirs::data_local_dir()
            .ok_or_else(|| SubscriptionError::Storage("Cannot determine data directory".to_string()))?
            .join("podcast-tui")
            .join("logs");

        tokio::fs::create_dir_all(&log_dir).await?;

        let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
        let log_path = log_dir.join(format!("opml-import-{}.log", timestamp));
        let log_path_str = log_path.to_string_lossy().to_string();

        let mut log_content = format!(
            "OPML Import Log\nStarted: {}\nSource: {}\n\n=== Processing ===\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            source
        );

        let mut result = ImportResult::new(total_feeds);

        // Process feeds sequentially
        for (index, outline) in document.outlines.iter().enumerate() {
            let feed_url = match outline.feed_url() {
                Some(url) => url,
                None => {
                    // Skip outlines without feed URLs
                    continue;
                }
            };

            let feed_title = outline.title.as_deref().or(Some(&outline.text)).unwrap();
            let current = index + 1;

            progress_callback(format!(
                "Importing [{}/{}]: {}...",
                current, total_feeds, feed_title
            ));

            log_content.push_str(&format!(
                "[{}] [{}/{}] Importing: {} ({})\n",
                Local::now().format("%H:%M:%S"),
                current,
                total_feeds,
                feed_title,
                feed_url
            ));

            // Check if already subscribed
            if self.is_subscribed(feed_url).await {
                progress_callback(format!(
                    "⊘ Skipped [{}/{}]: {} (already subscribed)",
                    current, total_feeds, feed_title
                ));

                log_content.push_str(&format!(
                    "[{}] [{}/{}] ⊘ Skipped (already subscribed)\n",
                    Local::now().format("%H:%M:%S"),
                    current,
                    total_feeds
                ));

                result.skipped += 1;
                continue;
            }

            // Attempt to subscribe
            match self.subscribe(feed_url).await {
                Ok(_) => {
                    progress_callback(format!(
                        "✓ Imported [{}/{}]: {}",
                        current, total_feeds, feed_title
                    ));

                    log_content.push_str(&format!(
                        "[{}] [{}/{}] ✓ Success\n",
                        Local::now().format("%H:%M:%S"),
                        current,
                        total_feeds
                    ));

                    result.imported += 1;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    progress_callback(format!(
                        "✗ Failed [{}/{}]: {} - {}",
                        current, total_feeds, feed_title, error_msg
                    ));

                    log_content.push_str(&format!(
                        "[{}] [{}/{}] ✗ Failed: {}\n",
                        Local::now().format("%H:%M:%S"),
                        current,
                        total_feeds,
                        error_msg
                    ));

                    result.failed.push(FailedImport {
                        url: feed_url.to_string(),
                        title: Some(feed_title.to_string()),
                        error: error_msg,
                    });
                }
            }
        }

        // Write summary to log
        log_content.push_str(&format!(
            "\n=== Summary ===\nCompleted: {}\nTotal feeds: {}\nImported: {}\nSkipped: {}\nFailed: {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            result.total_feeds,
            result.imported,
            result.skipped,
            result.failed.len()
        ));

        if result.has_failures() {
            log_content.push_str("\n=== Failed Imports ===\n");
            for (i, failure) in result.failed.iter().enumerate() {
                log_content.push_str(&format!(
                    "{}. {} ({})\n   Error: {}\n\n",
                    i + 1,
                    failure.title.as_deref().unwrap_or("Unknown"),
                    failure.url,
                    failure.error
                ));
            }
        }

        // Write log file
        tokio::fs::write(&log_path, log_content).await?;

        Ok((result, log_path_str))
    }

    /// Export all subscriptions to OPML file
    ///
    /// Generates a valid OPML 2.0 document with all current subscriptions.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path where OPML file should be written
    /// * `progress_callback` - Callback function for progress updates
    ///
    /// # Returns
    ///
    /// Number of feeds exported
    pub async fn export_opml<F>(
        &self,
        output_path: &std::path::Path,
        progress_callback: F,
    ) -> Result<usize, SubscriptionError>
    where
        F: Fn(String) + Send + Sync,
    {
        use crate::podcast::OpmlExporter;

        progress_callback("Loading subscriptions...".to_string());

        // Load all podcasts
        let podcasts = self.list_subscriptions().await?;
        let feed_count = podcasts.len();

        progress_callback(format!("Generating OPML ({} feeds)...", feed_count));

        // Generate and write OPML
        let exporter = OpmlExporter::new();
        exporter.export(&podcasts, output_path).await?;

        progress_callback("Writing to file...".to_string());

        Ok(feed_count)
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
