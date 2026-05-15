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

        // Hard refresh bypasses the HTTP cache: send no conditional headers
        // so the server always returns a full body and we re-process every
        // entry (e.g., to pick up corrected metadata or descriptions).
        let (etag_in, lm_in) = if hard_refresh {
            (None, None)
        } else {
            (
                podcast.last_etag.as_deref(),
                podcast.last_modified.as_deref(),
            )
        };

        // Get episodes from the feed using a conditional GET. `None` means
        // the server responded 304 Not Modified — we can skip parse, dedup,
        // renumber, and per-episode save entirely.
        let fetch_result = self
            .feed_parser
            .get_episodes_conditional(&podcast.url, podcast_id, etag_in, lm_in)
            .await?;

        let crate::podcast::EpisodeFetchResult {
            episodes: feed_episodes,
            etag: new_etag,
            last_modified: new_lm,
        } = match fetch_result {
            Some(r) => r,
            None => {
                // 304 path: nothing changed upstream. Bump last_updated so
                // the UI shows a recent refresh time, but do not touch
                // episodes. Cache validators stay as they are.
                podcast.last_updated = Utc::now();
                self.storage
                    .save_podcast(&podcast)
                    .await
                    .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
                return Ok(Vec::new());
            }
        };

        // 200 path: capture the new cache validators before we save the
        // podcast at the bottom of this function. Servers may return
        // either, both, or neither header — only overwrite when present.
        if new_etag.is_some() {
            podcast.last_etag = new_etag;
        }
        if new_lm.is_some() {
            podcast.last_modified = new_lm;
        }

        // Load existing episodes once. We use this both for dedup and as the
        // source of truth for chronological renumbering.
        let existing_episodes = self
            .storage
            .load_episodes(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        // Classify each feed episode as truly new vs. an update of an
        // existing one (via the multi-strategy dedup below).
        let mut new_episodes: Vec<Episode> = Vec::new();
        let mut updated_episodes: Vec<Episode> = Vec::new();

        for episode in feed_episodes {
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

        // Build the unified list: start from existing, apply hard-refresh
        // updates in place, then append truly new episodes. This is the
        // *complete* episode set we'll renumber against.
        let mut unified: Vec<Episode> = existing_episodes.clone();
        for upd in &updated_episodes {
            if let Some(pos) = unified.iter().position(|e| e.id == upd.id) {
                unified[pos] = upd.clone();
            }
        }
        for new_ep in &new_episodes {
            unified.push(new_ep.clone());
        }

        // Renumber 1..N in chronological order on every refresh. Doing this
        // unconditionally (rather than only on hard_refresh) is what fixes
        // the snowballing-track-numbers bug (#231): the previous incremental
        // path used `max_track + index` and compounded every time dedup
        // misclassified a re-published episode as "new", producing
        // `episode_number` values 60-70x the real episode count.
        // Tie-break on episode id so podcasts with multiple episodes sharing
        // the same `published` timestamp produce a stable, deterministic order
        // (avoids repeated re-numbering on every refresh — see PR #236 review).
        unified.sort_by(|a, b| {
            a.published
                .cmp(&b.published)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        for (index, episode) in unified.iter_mut().enumerate() {
            episode.episode_number = Some((index + 1) as u32);
        }

        // Save anything that changed: every truly-new episode, every
        // hard-refresh-updated episode, and any existing episode whose
        // `episode_number` differs from what's on disk (renumbering can
        // shift older episodes' positions when older items are added late).
        let new_ids: std::collections::HashSet<_> =
            new_episodes.iter().map(|e| e.id.clone()).collect();
        let updated_ids: std::collections::HashSet<_> =
            updated_episodes.iter().map(|e| e.id.clone()).collect();
        let existing_by_id: std::collections::HashMap<_, _> = existing_episodes
            .iter()
            .map(|e| (e.id.clone(), e))
            .collect();

        // Re-derive new_episodes and updated_episodes from the unified
        // (renumbered) list so callers see the corrected episode_number
        // on each returned record.
        let mut new_with_tracks = Vec::new();
        let mut updated_with_tracks = Vec::new();

        for ep in &unified {
            let needs_save = if new_ids.contains(&ep.id) {
                new_with_tracks.push(ep.clone());
                true
            } else if updated_ids.contains(&ep.id) {
                updated_with_tracks.push(ep.clone());
                true
            } else {
                // Existing episode that wasn't touched by this feed pull —
                // save only if its track number actually changed.
                existing_by_id
                    .get(&ep.id)
                    .map(|orig| orig.episode_number != ep.episode_number)
                    .unwrap_or(false)
            };

            if needs_save {
                self.storage
                    .save_episode(podcast_id, ep)
                    .await
                    .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
            }
        }

        // Combine new and updated episodes for return value
        let mut all_changes = new_with_tracks;
        all_changes.extend(updated_with_tracks);

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

    /// Renumber a podcast's episodes 1..N in chronological order if and
    /// only if the current numbering is broken. Returns the count of
    /// episodes whose `episode_number` was changed.
    ///
    /// "Broken" means any of:
    /// - At least one episode has `episode_number == None` while at least
    ///   one other has `Some(_)`. (Sub-#231 podcasts had this — older
    ///   episodes were left null while newer ones were numbered.)
    /// - `max(episode_number) > episode_count`. The pre-fix incremental
    ///   refresh path used `max + index` and snowballed this far above
    ///   the actual count — real user data: JRE #2486 had
    ///   `episode_number: 168025` against ~2,500 real episodes.
    /// - The numbers don't form a dense, gap-free `1..=N` after sorting
    ///   chronologically by `published`. (Catches less-extreme drift.)
    ///
    /// On any of those, every affected episode is re-saved with the
    /// corrected number. Cheap O(N) check; only writes the episodes
    /// whose number actually changes.
    ///
    /// Used by the one-time startup migration after upgrade. Idempotent:
    /// a second call on the same podcast is effectively a no-op.
    pub async fn renumber_podcast_episodes(
        &self,
        podcast_id: &PodcastId,
    ) -> Result<usize, SubscriptionError> {
        let mut episodes = self
            .storage
            .load_episodes(podcast_id)
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        if episodes.is_empty() {
            return Ok(0);
        }

        // Sort chronologically once, with episode id as a deterministic
        // tie-breaker so episodes sharing a `published` timestamp produce
        // stable numbering across runs (see PR #236 review).
        episodes.sort_by(|a, b| {
            a.published
                .cmp(&b.published)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });

        let already_correct = episodes
            .iter()
            .enumerate()
            .all(|(i, e)| e.episode_number == Some((i + 1) as u32));

        if already_correct {
            return Ok(0);
        }

        let mut changed = 0usize;
        for (index, episode) in episodes.iter_mut().enumerate() {
            let new_number = Some((index + 1) as u32);
            if episode.episode_number != new_number {
                episode.episode_number = new_number;
                self.storage
                    .save_episode(podcast_id, episode)
                    .await
                    .map_err(|e| SubscriptionError::Storage(e.to_string()))?;
                changed += 1;
            }
        }

        Ok(changed)
    }

    /// One-time startup migration: scan every subscribed podcast and
    /// renumber any whose `episode_number` field is in the broken state
    /// described in `renumber_podcast_episodes`. Returns
    /// `(podcasts_renumbered, total_episodes_renumbered)`.
    ///
    /// Cheap on healthy data (one O(N) scan per podcast, no writes); only
    /// touches disk for podcasts that actually need fixing.
    pub async fn migrate_episode_numbering(&self) -> Result<(usize, usize), SubscriptionError> {
        let podcast_ids = self
            .storage
            .list_podcasts()
            .await
            .map_err(|e| SubscriptionError::Storage(e.to_string()))?;

        let mut podcasts_renumbered = 0usize;
        let mut total_episodes_renumbered = 0usize;

        for podcast_id in podcast_ids {
            let changed = self.renumber_podcast_episodes(&podcast_id).await?;
            if changed > 0 {
                podcasts_renumbered += 1;
                total_episodes_renumbered += changed;
            }
        }

        Ok((podcasts_renumbered, total_episodes_renumbered))
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
            .ok_or_else(|| {
                SubscriptionError::Storage("Cannot determine data directory".to_string())
            })?
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

            let feed_title = outline.title.as_deref().unwrap_or(&outline.text);
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
                    // Check if this is an "already subscribed" error
                    // If so, treat it as a skip rather than a failure (defensive programming)
                    let error_msg = e.to_string();
                    if error_msg.contains("already subscribed")
                        || error_msg.contains("AlreadySubscribed")
                    {
                        progress_callback(format!(
                            "⊘ Skipped [{}/{}]: {} (already subscribed)",
                            current, total_feeds, feed_title
                        ));

                        log_content.push_str(&format!(
                            "[{}] [{}/{}] ⊘ Skipped (already subscribed, caught by subscribe)\n",
                            Local::now().format("%H:%M:%S"),
                            current,
                            total_feeds
                        ));

                        result.skipped += 1;
                    } else {
                        // Genuine error - add to failed list
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

    // ─── Renumber tests for #231 ────────────────────────────────────────

    fn make_podcast(title: &str) -> Podcast {
        Podcast::new(
            format!("https://example.com/{}.rss", title.to_lowercase()),
            title.to_string(),
        )
    }

    fn make_episode(podcast_id: &PodcastId, title: &str, days_ago: i64) -> Episode {
        Episode::new(
            podcast_id.clone(),
            title.to_string(),
            format!("https://example.com/{}.mp3", title.to_lowercase()),
            Utc::now() - chrono::Duration::days(days_ago),
        )
    }

    async fn renumber_test_setup() -> (
        SubscriptionManager<JsonStorage>,
        TempDir,
        Podcast,
        Vec<Episode>,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        let podcast = make_podcast("Test Show");
        storage.save_podcast(&podcast).await.unwrap();

        // 5 episodes: oldest first by days_ago
        let episodes = vec![
            make_episode(&podcast.id, "Ep1 oldest", 50),
            make_episode(&podcast.id, "Ep2", 40),
            make_episode(&podcast.id, "Ep3", 30),
            make_episode(&podcast.id, "Ep4", 20),
            make_episode(&podcast.id, "Ep5 newest", 10),
        ];
        let manager = SubscriptionManager::new(storage);
        (manager, temp_dir, podcast, episodes)
    }

    /// Healthy podcast with all episodes already numbered 1..N must be
    /// a no-op (no writes, returns 0).
    #[tokio::test]
    async fn test_renumber_noop_when_already_dense() {
        let (manager, _td, podcast, mut episodes) = renumber_test_setup().await;
        for (i, ep) in episodes.iter_mut().enumerate() {
            ep.episode_number = Some((i + 1) as u32);
            manager.storage.save_episode(&podcast.id, ep).await.unwrap();
        }

        let changed = manager
            .renumber_podcast_episodes(&podcast.id)
            .await
            .unwrap();
        assert_eq!(changed, 0, "dense 1..N must be a no-op");
    }

    /// JRE-style snowballed numbers (max far exceeds count) must be
    /// reset to dense 1..N.
    #[tokio::test]
    async fn test_renumber_fixes_snowballed_numbers() {
        let (manager, _td, podcast, mut episodes) = renumber_test_setup().await;
        // Simulate the bug: 5 episodes with bogus high numbers.
        for (i, ep) in episodes.iter_mut().enumerate() {
            ep.episode_number = Some(168000 + i as u32);
            manager.storage.save_episode(&podcast.id, ep).await.unwrap();
        }

        let changed = manager
            .renumber_podcast_episodes(&podcast.id)
            .await
            .unwrap();
        assert_eq!(changed, 5, "all 5 episodes should be renumbered");

        // Verify the on-disk state is now dense 1..N in chronological order.
        let mut after = manager.storage.load_episodes(&podcast.id).await.unwrap();
        after.sort_by_key(|e| e.published);
        for (i, ep) in after.iter().enumerate() {
            assert_eq!(
                ep.episode_number,
                Some((i + 1) as u32),
                "episode at chronological index {i} must have number {}",
                i + 1
            );
        }
    }

    /// Mixed null + Some state (some old episodes never got numbers,
    /// newer ones were assigned). Renumber must produce dense 1..N
    /// with no nulls.
    #[tokio::test]
    async fn test_renumber_fixes_mixed_null_and_some() {
        let (manager, _td, podcast, mut episodes) = renumber_test_setup().await;
        // First 3 are null, last 2 have small (but wrong) numbers.
        episodes[0].episode_number = None;
        episodes[1].episode_number = None;
        episodes[2].episode_number = None;
        episodes[3].episode_number = Some(100);
        episodes[4].episode_number = Some(101);
        for ep in &episodes {
            manager.storage.save_episode(&podcast.id, ep).await.unwrap();
        }

        let changed = manager
            .renumber_podcast_episodes(&podcast.id)
            .await
            .unwrap();
        assert_eq!(changed, 5);

        let mut after = manager.storage.load_episodes(&podcast.id).await.unwrap();
        after.sort_by_key(|e| e.published);
        for (i, ep) in after.iter().enumerate() {
            assert_eq!(ep.episode_number, Some((i + 1) as u32));
        }
        assert!(
            after.iter().all(|e| e.episode_number.is_some()),
            "no nulls allowed after renumber"
        );
    }

    /// Renumber must be idempotent: running it twice on the same broken
    /// state changes 0 episodes the second time.
    #[tokio::test]
    async fn test_renumber_is_idempotent() {
        let (manager, _td, podcast, mut episodes) = renumber_test_setup().await;
        for ep in &mut episodes {
            ep.episode_number = Some(99999);
            manager.storage.save_episode(&podcast.id, ep).await.unwrap();
        }

        let first = manager
            .renumber_podcast_episodes(&podcast.id)
            .await
            .unwrap();
        assert!(first > 0);
        let second = manager
            .renumber_podcast_episodes(&podcast.id)
            .await
            .unwrap();
        assert_eq!(second, 0, "second call must be a no-op");
    }

    /// Empty podcast: no episodes, no error, no writes.
    #[tokio::test]
    async fn test_renumber_empty_podcast() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        let podcast = make_podcast("Empty");
        storage.save_podcast(&podcast).await.unwrap();

        let manager = SubscriptionManager::new(storage);
        let changed = manager
            .renumber_podcast_episodes(&podcast.id)
            .await
            .unwrap();
        assert_eq!(changed, 0);
    }

    /// migrate_episode_numbering walks every podcast and reports
    /// aggregate counts.
    #[tokio::test]
    async fn test_migrate_aggregates_across_podcasts() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf());
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        // Healthy podcast: 3 episodes, already numbered 1..3.
        let healthy = make_podcast("Healthy");
        storage.save_podcast(&healthy).await.unwrap();
        let mut healthy_eps = [
            make_episode(&healthy.id, "h1", 30),
            make_episode(&healthy.id, "h2", 20),
            make_episode(&healthy.id, "h3", 10),
        ];
        for (i, ep) in healthy_eps.iter_mut().enumerate() {
            ep.episode_number = Some((i + 1) as u32);
            storage.save_episode(&healthy.id, ep).await.unwrap();
        }

        // Broken podcast: 4 episodes with snowballed numbers.
        let broken = make_podcast("Broken");
        storage.save_podcast(&broken).await.unwrap();
        let mut broken_eps = [
            make_episode(&broken.id, "b1", 40),
            make_episode(&broken.id, "b2", 30),
            make_episode(&broken.id, "b3", 20),
            make_episode(&broken.id, "b4", 10),
        ];
        for (i, ep) in broken_eps.iter_mut().enumerate() {
            ep.episode_number = Some(50000 + i as u32);
            storage.save_episode(&broken.id, ep).await.unwrap();
        }

        let manager = SubscriptionManager::new(storage);
        let (podcasts_fixed, episodes_fixed) = manager.migrate_episode_numbering().await.unwrap();
        assert_eq!(podcasts_fixed, 1, "only the broken podcast counts");
        assert_eq!(episodes_fixed, 4, "all 4 broken episodes renumbered");
    }
}
