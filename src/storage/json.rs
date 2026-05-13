use async_trait::async_trait;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;

use crate::playlist::{Playlist, PlaylistId};
use crate::podcast::{Episode, Podcast};
use crate::storage::{EpisodeId, PodcastId, Storage, StorageError};
use crate::utils::text::strip_html;
use crate::utils::validation::sanitize_playlist_name;

/// Schema version for the in-memory cache snapshot. Bumped when the snapshot
/// shape changes incompatibly. The persistent index file rejects older
/// versions and rebuilds from disk.
pub(crate) const CACHE_SCHEMA_VERSION: u32 = 1;

/// File name of the persistent cache index, stored in `data_dir`.
pub(crate) const CACHE_FILE_NAME: &str = "cache_index.json";

/// Background flush interval. The flush task only writes when the cache
/// is dirty, so this is a worst-case write frequency.
const CACHE_FLUSH_INTERVAL: Duration = Duration::from_secs(5);

/// In-memory snapshot of the storage data set.
///
/// The snapshot is the authoritative read source when caching is enabled;
/// individual JSON files on disk remain the persistent source of truth and
/// every mutation is written to disk *before* the snapshot is updated.
///
/// A `None` snapshot indicates the cache has not been initialised yet (lazy
/// build on first access). The fields are visible to the rest of the
/// `storage` module so a future persistent-index loader can construct
/// snapshots directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CacheSnapshot {
    pub(crate) schema_version: u32,
    pub(crate) last_updated: chrono::DateTime<chrono::Utc>,
    pub(crate) podcasts: HashMap<PodcastId, Podcast>,
    pub(crate) episodes: HashMap<PodcastId, Vec<Episode>>,
    pub(crate) playlists: HashMap<PlaylistId, Playlist>,
}

impl CacheSnapshot {
    pub(crate) fn empty() -> Self {
        Self {
            schema_version: CACHE_SCHEMA_VERSION,
            last_updated: chrono::Utc::now(),
            podcasts: HashMap::new(),
            episodes: HashMap::new(),
            playlists: HashMap::new(),
        }
    }
}

/// JSON-based file storage implementation
///
/// This implementation stores data in JSON files on the filesystem,
/// organized in a directory structure for efficient access and management.
///
/// Directory Structure:
/// ```text
/// ~/.local/share/podcast-tui/
/// ├── podcasts/
/// │   └── {podcast-id}.json
/// └── episodes/
///     └── {podcast-id}/
///         └── {episode-id}.json
/// ```
///
/// ## Caching
///
/// When `cache_enabled` is true (the default), an in-memory snapshot of the
/// data is built lazily on first read and kept in sync with disk on every
/// write. Subsequent reads in the same session return cached values without
/// re-scanning the data directory. Disk remains the source of truth: writes
/// land on disk first and the cache is updated only after a successful write.
///
/// The `cache_dirty` flag is set whenever the in-memory snapshot diverges
/// from the on-disk persistent index. It is read by the background flush
/// task added in the persistent-cache follow-up issue.
pub struct JsonStorage {
    pub data_dir: PathBuf,
    podcasts_dir: PathBuf,
    episodes_dir: PathBuf,
    playlists_dir: PathBuf,
    cache_enabled: bool,
    cache: Arc<RwLock<Option<CacheSnapshot>>>,
    cache_dirty: Arc<AtomicBool>,
}

impl JsonStorage {
    /// Create a new JSON storage instance
    ///
    /// Uses the system's standard application data directory.
    /// On Linux: ~/.local/share/podcast-tui/
    /// On Windows: %APPDATA%/podcast-tui/
    /// On macOS: ~/Library/Application Support/podcast-tui/
    pub fn new() -> Result<Self, StorageError> {
        let project_dirs = ProjectDirs::from("", "", "podcast-tui").ok_or_else(|| {
            StorageError::InitializationFailed {
                reason: "Unable to determine application data directory".to_string(),
            }
        })?;

        let data_dir = project_dirs.data_dir().to_path_buf();
        Ok(Self::build(data_dir, true))
    }

    /// Create a new JSON storage instance with custom data directory
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        Self::build(data_dir, true)
    }

    /// Builder: enable or disable the in-memory cache.
    ///
    /// Defaults to enabled. Disabling makes every read hit disk, matching
    /// the behaviour of versions prior to the cache.
    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    fn build(data_dir: PathBuf, cache_enabled: bool) -> Self {
        let podcasts_dir = data_dir.join("podcasts");
        let episodes_dir = data_dir.join("episodes");
        let playlists_dir = data_dir.join("Playlists");

        Self {
            data_dir,
            podcasts_dir,
            episodes_dir,
            playlists_dir,
            cache_enabled,
            cache: Arc::new(RwLock::new(None)),
            cache_dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Whether the in-memory cache is enabled for this instance.
    pub fn cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    /// True when the in-memory snapshot has changed since the last persistent
    /// flush. Used by the persistent-index follow-up issue.
    pub fn cache_is_dirty(&self) -> bool {
        self.cache_dirty.load(Ordering::Relaxed)
    }

    /// Get the file path for a podcast
    fn podcast_path(&self, id: &PodcastId) -> PathBuf {
        self.podcasts_dir.join(format!("{}.json", id))
    }

    /// Get the directory path for podcast episodes
    fn podcast_episodes_dir(&self, podcast_id: &PodcastId) -> PathBuf {
        self.episodes_dir.join(podcast_id.to_string())
    }

    /// Get the file path for an episode
    fn episode_path(&self, podcast_id: &PodcastId, episode_id: &EpisodeId) -> PathBuf {
        self.podcast_episodes_dir(podcast_id)
            .join(format!("{}.json", episode_id))
    }

    /// Get the directory path for a playlist by name.
    fn playlist_dir_by_name(&self, name: &str) -> PathBuf {
        self.playlists_dir.join(sanitize_playlist_name(name))
    }

    /// Get the metadata path for a playlist by name.
    fn playlist_metadata_path_by_name(&self, name: &str) -> PathBuf {
        self.playlist_dir_by_name(name).join("playlist.json")
    }

    /// Find playlist metadata path by playlist ID.
    async fn find_playlist_metadata_path_by_id(
        &self,
        id: &PlaylistId,
    ) -> Result<Option<PathBuf>, StorageError> {
        if !self.playlists_dir.exists() {
            return Ok(None);
        }

        let mut entries = fs::read_dir(&self.playlists_dir)
            .await
            .map_err(|e| StorageError::file_operation("read_dir", &self.playlists_dir, e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::file_operation("read_dir_entry", &self.playlists_dir, e))?
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let metadata_path = path.join("playlist.json");
            if !metadata_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| StorageError::file_operation("read", &metadata_path, e))?;
            let playlist: Playlist = serde_json::from_str(&content)?;

            if playlist.id == *id {
                return Ok(Some(metadata_path));
            }
        }

        Ok(None)
    }

    /// Atomic write operation to prevent data corruption
    async fn atomic_write(&self, path: &Path, content: &str) -> Result<(), StorageError> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::file_operation("create_dir_all", parent, e))?;
        }

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content)
            .await
            .map_err(|e| StorageError::file_operation("write_temp", &temp_path, e))?;

        // Atomically move to final location
        fs::rename(&temp_path, path)
            .await
            .map_err(|e| StorageError::file_operation("rename", path, e))?;

        Ok(())
    }

    // ─── Cache helpers ──────────────────────────────────────────────────

    /// Mark the in-memory snapshot as out-of-sync with the persistent index.
    fn mark_dirty(&self) {
        if self.cache_enabled {
            self.cache_dirty.store(true, Ordering::Relaxed);
        }
    }

    /// Path to the persistent cache index file.
    fn cache_index_path(&self) -> PathBuf {
        self.data_dir.join(CACHE_FILE_NAME)
    }

    /// Path to the in-flight cache index temp file.
    fn cache_index_tmp_path(&self) -> PathBuf {
        self.cache_index_path().with_extension("json.tmp")
    }

    /// Read the persistent cache index from disk if it exists. Returns
    /// `Ok(None)` when the index file is absent (clean first launch).
    /// Bubbles up I/O and parse errors so the caller can decide whether to
    /// rebuild from disk.
    async fn load_cache_from_disk(&self) -> Result<Option<CacheSnapshot>, StorageError> {
        let path = self.cache_index_path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path)
            .await
            .map_err(|e| StorageError::file_operation("read_cache_index", &path, e))?;
        let snap: CacheSnapshot = serde_json::from_slice(&bytes)?;
        Ok(Some(snap))
    }

    /// Spawn the background flush task. The task is fire-and-forget; it
    /// terminates with the tokio runtime. No abort handle is stored because
    /// `JsonStorage` is `Arc`-shared throughout the app lifetime.
    fn spawn_flush_task(&self) {
        let cache = self.cache.clone();
        let dirty = self.cache_dirty.clone();
        let data_dir = self.data_dir.clone();
        tokio::spawn(async move {
            // Wait one full interval before the first attempt so tests
            // (which init then immediately exit) don't trip the flush.
            let start = tokio::time::Instant::now() + CACHE_FLUSH_INTERVAL;
            let mut interval = tokio::time::interval_at(start, CACHE_FLUSH_INTERVAL);
            loop {
                interval.tick().await;
                if !dirty.load(Ordering::Relaxed) {
                    continue;
                }
                if let Err(e) = flush_snapshot(&data_dir, &cache, &dirty).await {
                    eprintln!("[cache] background flush failed: {e}");
                }
            }
        });
    }

    /// Block until any pending in-memory changes are written to the
    /// persistent index. Safe to call when caching is disabled (no-op).
    /// Used by graceful shutdown.
    pub async fn flush_cache_blocking(&self) -> Result<(), StorageError> {
        if !self.cache_enabled {
            return Ok(());
        }
        flush_snapshot(&self.data_dir, &self.cache, &self.cache_dirty).await
    }

    /// Rebuild the in-memory snapshot from disk and immediately flush it.
    /// Returns `(podcast_count, episode_count)` for user feedback.
    /// Used by `:cache-rebuild`.
    pub async fn rebuild_cache(&self) -> Result<(usize, usize), StorageError> {
        let snap = self.build_snapshot_from_disk().await?;
        let podcast_count = snap.podcasts.len();
        let episode_count: usize = snap.episodes.values().map(|v| v.len()).sum();
        *self.cache.write().await = Some(snap);
        self.cache_dirty.store(true, Ordering::Relaxed);
        if self.cache_enabled {
            flush_snapshot(&self.data_dir, &self.cache, &self.cache_dirty).await?;
        }
        Ok((podcast_count, episode_count))
    }

    /// Lazily build the in-memory snapshot from disk on first access.
    ///
    /// Subsequent calls are a cheap `Option` check while holding only a
    /// read lock. The snapshot is built outside any lock, then installed
    /// under a brief write lock — concurrent first-access calls will each
    /// build their own snapshot and the last writer wins; that is correct
    /// because `build_snapshot_from_disk` is idempotent.
    async fn ensure_cache_initialized(&self) -> Result<(), StorageError> {
        if !self.cache_enabled {
            return Ok(());
        }
        if self.cache.read().await.is_some() {
            return Ok(());
        }
        let snapshot = self.build_snapshot_from_disk().await?;
        let mut guard = self.cache.write().await;
        if guard.is_none() {
            *guard = Some(snapshot);
        }
        Ok(())
    }

    /// Scan disk and build a fresh snapshot. Used for first-access lazy
    /// init and (in the persistent-cache follow-up) for `:cache-rebuild`.
    async fn build_snapshot_from_disk(&self) -> Result<CacheSnapshot, StorageError> {
        let mut snap = CacheSnapshot::empty();

        let podcast_ids = self.list_podcasts_from_disk().await?;
        for pid in &podcast_ids {
            let podcast = self.load_podcast_from_disk(pid).await?;
            snap.podcasts.insert(pid.clone(), podcast);

            let episode_ids = self.list_episode_ids_from_disk(pid).await?;
            let mut eps = Vec::with_capacity(episode_ids.len());
            for eid in episode_ids {
                eps.push(self.load_episode_from_disk(pid, &eid).await?);
            }
            snap.episodes.insert(pid.clone(), eps);
        }

        for playlist in self.list_playlists_from_disk().await? {
            snap.playlists.insert(playlist.id.clone(), playlist);
        }

        Ok(snap)
    }

    // ─── Uncached disk readers (used by build_snapshot_from_disk and as
    //     fallback when the cache is disabled) ──────────────────────────

    async fn load_podcast_from_disk(&self, id: &PodcastId) -> Result<Podcast, StorageError> {
        let path = self.podcast_path(id);
        if !path.exists() {
            return Err(StorageError::PodcastNotFound { id: id.clone() });
        }
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| StorageError::file_operation("read", &path, e))?;
        let mut podcast: Podcast = serde_json::from_str(&content)?;
        if let Some(ref description) = podcast.description {
            if description.contains('<') || description.contains("&lt;") {
                podcast.description = Some(strip_html(description));
            }
        }
        Ok(podcast)
    }

    async fn list_podcasts_from_disk(&self) -> Result<Vec<PodcastId>, StorageError> {
        if !self.podcasts_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&self.podcasts_dir)
            .await
            .map_err(|e| StorageError::file_operation("read_dir", &self.podcasts_dir, e))?;
        let mut ids = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::file_operation("read_dir_entry", &self.podcasts_dir, e))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let filename = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                    StorageError::FileOperation {
                        operation: "parse_filename".to_string(),
                        path: path.display().to_string(),
                        error: "Invalid filename".to_string(),
                    }
                })?;
                let id =
                    PodcastId::from_string(filename).map_err(|e| StorageError::FileOperation {
                        operation: "parse_uuid".to_string(),
                        path: path.display().to_string(),
                        error: e.to_string(),
                    })?;
                ids.push(id);
            }
        }
        Ok(ids)
    }

    async fn load_episode_from_disk(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<Episode, StorageError> {
        let path = self.episode_path(podcast_id, episode_id);
        if !path.exists() {
            return Err(StorageError::EpisodeNotFound {
                podcast_id: podcast_id.clone(),
                episode_id: episode_id.clone(),
            });
        }
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| StorageError::file_operation("read", &path, e))?;
        let mut episode: Episode = serde_json::from_str(&content)?;
        if let Some(ref description) = episode.description {
            if description.contains('<') || description.contains("&lt;") {
                episode.description = Some(strip_html(description));
            }
        }
        Ok(episode)
    }

    async fn list_episode_ids_from_disk(
        &self,
        podcast_id: &PodcastId,
    ) -> Result<Vec<EpisodeId>, StorageError> {
        let episodes_dir = self.podcast_episodes_dir(podcast_id);
        if !episodes_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&episodes_dir)
            .await
            .map_err(|e| StorageError::file_operation("read_dir", &episodes_dir, e))?;
        let mut ids = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::file_operation("read_dir_entry", &episodes_dir, e))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let filename = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                    StorageError::FileOperation {
                        operation: "parse_filename".to_string(),
                        path: path.display().to_string(),
                        error: "Invalid filename".to_string(),
                    }
                })?;
                let id =
                    EpisodeId::from_string(filename).map_err(|e| StorageError::FileOperation {
                        operation: "parse_uuid".to_string(),
                        path: path.display().to_string(),
                        error: e.to_string(),
                    })?;
                ids.push(id);
            }
        }
        Ok(ids)
    }

    async fn list_playlists_from_disk(&self) -> Result<Vec<Playlist>, StorageError> {
        if !self.playlists_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&self.playlists_dir)
            .await
            .map_err(|e| StorageError::file_operation("read_dir", &self.playlists_dir, e))?;
        let mut out = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::file_operation("read_dir_entry", &self.playlists_dir, e))?
        {
            let playlist_dir = entry.path();
            if !playlist_dir.is_dir() {
                continue;
            }
            let metadata_path = playlist_dir.join("playlist.json");
            if !metadata_path.exists() {
                continue;
            }
            let content = fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| StorageError::file_operation("read", &metadata_path, e))?;
            let playlist: Playlist = serde_json::from_str(&content)?;
            out.push(playlist);
        }
        Ok(out)
    }
}

#[async_trait]
impl Storage for JsonStorage {
    type Error = StorageError;

    async fn save_podcast(&self, podcast: &Podcast) -> Result<(), Self::Error> {
        let path = self.podcast_path(&podcast.id);
        let json = serde_json::to_string_pretty(podcast)?;

        self.atomic_write(&path, &json).await?;

        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            let mut guard = self.cache.write().await;
            if let Some(snap) = guard.as_mut() {
                snap.podcasts.insert(podcast.id.clone(), podcast.clone());
                snap.episodes.entry(podcast.id.clone()).or_default();
                snap.last_updated = chrono::Utc::now();
            }
            drop(guard);
            self.mark_dirty();
        }
        Ok(())
    }

    async fn load_podcast(&self, id: &PodcastId) -> Result<Podcast, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return snap
                    .podcasts
                    .get(id)
                    .cloned()
                    .ok_or_else(|| StorageError::PodcastNotFound { id: id.clone() });
            }
        }
        self.load_podcast_from_disk(id).await
    }

    async fn delete_podcast(&self, id: &PodcastId) -> Result<(), Self::Error> {
        let path = self.podcast_path(id);

        if !path.exists() {
            return Err(StorageError::PodcastNotFound { id: id.clone() });
        }

        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::file_operation("delete", &path, e))?;

        // Also remove episodes directory if it exists
        let episodes_dir = self.podcast_episodes_dir(id);
        if episodes_dir.exists() {
            fs::remove_dir_all(&episodes_dir)
                .await
                .map_err(|e| StorageError::file_operation("remove_dir_all", &episodes_dir, e))?;
        }

        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            let mut guard = self.cache.write().await;
            if let Some(snap) = guard.as_mut() {
                snap.podcasts.remove(id);
                snap.episodes.remove(id);
                snap.last_updated = chrono::Utc::now();
            }
            drop(guard);
            self.mark_dirty();
        }

        Ok(())
    }

    async fn list_podcasts(&self) -> Result<Vec<PodcastId>, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap.podcasts.keys().cloned().collect());
            }
        }
        self.list_podcasts_from_disk().await
    }

    async fn podcast_exists(&self, id: &PodcastId) -> Result<bool, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap.podcasts.contains_key(id));
            }
        }
        Ok(self.podcast_path(id).exists())
    }

    async fn save_episode(
        &self,
        podcast_id: &PodcastId,
        episode: &Episode,
    ) -> Result<(), Self::Error> {
        let path = self.episode_path(podcast_id, &episode.id);
        let json = serde_json::to_string_pretty(episode)?;

        self.atomic_write(&path, &json).await?;

        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            let mut guard = self.cache.write().await;
            if let Some(snap) = guard.as_mut() {
                let list = snap.episodes.entry(podcast_id.clone()).or_default();
                if let Some(pos) = list.iter().position(|e| e.id == episode.id) {
                    list[pos] = episode.clone();
                } else {
                    list.push(episode.clone());
                }
                snap.last_updated = chrono::Utc::now();
            }
            drop(guard);
            self.mark_dirty();
        }
        Ok(())
    }

    async fn load_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<Episode, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                if let Some(eps) = snap.episodes.get(podcast_id) {
                    return eps
                        .iter()
                        .find(|e| e.id == *episode_id)
                        .cloned()
                        .ok_or_else(|| StorageError::EpisodeNotFound {
                            podcast_id: podcast_id.clone(),
                            episode_id: episode_id.clone(),
                        });
                }
                return Err(StorageError::EpisodeNotFound {
                    podcast_id: podcast_id.clone(),
                    episode_id: episode_id.clone(),
                });
            }
        }
        self.load_episode_from_disk(podcast_id, episode_id).await
    }

    async fn delete_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<(), Self::Error> {
        let path = self.episode_path(podcast_id, episode_id);

        if !path.exists() {
            return Err(StorageError::EpisodeNotFound {
                podcast_id: podcast_id.clone(),
                episode_id: episode_id.clone(),
            });
        }

        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::file_operation("delete", &path, e))?;

        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            let mut guard = self.cache.write().await;
            if let Some(snap) = guard.as_mut() {
                if let Some(list) = snap.episodes.get_mut(podcast_id) {
                    list.retain(|e| e.id != *episode_id);
                }
                snap.last_updated = chrono::Utc::now();
            }
            drop(guard);
            self.mark_dirty();
        }

        Ok(())
    }

    async fn list_episodes(&self, podcast_id: &PodcastId) -> Result<Vec<EpisodeId>, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap
                    .episodes
                    .get(podcast_id)
                    .map(|eps| eps.iter().map(|e| e.id.clone()).collect())
                    .unwrap_or_default());
            }
        }
        self.list_episode_ids_from_disk(podcast_id).await
    }

    async fn episode_exists(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<bool, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap
                    .episodes
                    .get(podcast_id)
                    .is_some_and(|eps| eps.iter().any(|e| e.id == *episode_id)));
            }
        }
        Ok(self.episode_path(podcast_id, episode_id).exists())
    }

    async fn save_episodes(
        &self,
        podcast_id: &PodcastId,
        episodes: &[Episode],
    ) -> Result<(), Self::Error> {
        // Create episodes directory for this podcast if it doesn't exist
        let episodes_dir = self.podcast_episodes_dir(podcast_id);
        fs::create_dir_all(&episodes_dir)
            .await
            .map_err(|e| StorageError::file_operation("create_dir_all", &episodes_dir, e))?;

        // Save all episodes
        for episode in episodes {
            self.save_episode(podcast_id, episode).await?;
        }

        Ok(())
    }

    async fn load_episodes(&self, podcast_id: &PodcastId) -> Result<Vec<Episode>, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap.episodes.get(podcast_id).cloned().unwrap_or_default());
            }
        }
        let episode_ids = self.list_episode_ids_from_disk(podcast_id).await?;
        let mut episodes = Vec::with_capacity(episode_ids.len());

        for episode_id in episode_ids {
            let episode = self.load_episode_from_disk(podcast_id, &episode_id).await?;
            episodes.push(episode);
        }

        Ok(episodes)
    }

    async fn save_playlist(&self, playlist: &Playlist) -> Result<(), Self::Error> {
        let playlist_dir = self.playlist_dir_by_name(&playlist.name);
        let metadata_path = self.playlist_metadata_path_by_name(&playlist.name);
        let audio_dir = playlist_dir.join("audio");

        if let Some(existing_metadata_path) =
            self.find_playlist_metadata_path_by_id(&playlist.id).await?
        {
            let existing_dir =
                existing_metadata_path
                    .parent()
                    .ok_or_else(|| StorageError::FileOperation {
                        operation: "find_parent".to_string(),
                        path: existing_metadata_path.display().to_string(),
                        error: "Missing parent directory".to_string(),
                    })?;

            if existing_dir != playlist_dir {
                if playlist_dir.exists() {
                    return Err(StorageError::FileOperation {
                        operation: "rename_playlist_dir".to_string(),
                        path: playlist_dir.display().to_string(),
                        error: "Target directory already exists".to_string(),
                    });
                }

                fs::rename(existing_dir, &playlist_dir)
                    .await
                    .map_err(|e| StorageError::file_operation("rename", &playlist_dir, e))?;
            }
        } else if metadata_path.exists() {
            let existing_content = fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| StorageError::file_operation("read", &metadata_path, e))?;
            let existing: Playlist = serde_json::from_str(&existing_content)?;
            if existing.id != playlist.id {
                return Err(StorageError::FileOperation {
                    operation: "save_playlist".to_string(),
                    path: metadata_path.display().to_string(),
                    error: format!("Playlist name '{}' already exists", playlist.name),
                });
            }
        }

        fs::create_dir_all(&audio_dir)
            .await
            .map_err(|e| StorageError::file_operation("create_dir_all", &audio_dir, e))?;

        let json = serde_json::to_string_pretty(playlist)?;
        self.atomic_write(&metadata_path, &json).await?;

        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            let mut guard = self.cache.write().await;
            if let Some(snap) = guard.as_mut() {
                snap.playlists.insert(playlist.id.clone(), playlist.clone());
                snap.last_updated = chrono::Utc::now();
            }
            drop(guard);
            self.mark_dirty();
        }
        Ok(())
    }

    async fn load_playlist(&self, id: &PlaylistId) -> Result<Playlist, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return snap
                    .playlists
                    .get(id)
                    .cloned()
                    .ok_or_else(|| StorageError::PlaylistNotFound { id: id.to_string() });
            }
        }
        let metadata_path = self
            .find_playlist_metadata_path_by_id(id)
            .await?
            .ok_or_else(|| StorageError::PlaylistNotFound { id: id.to_string() })?;

        let content = fs::read_to_string(&metadata_path)
            .await
            .map_err(|e| StorageError::file_operation("read", &metadata_path, e))?;
        let playlist = serde_json::from_str(&content)?;
        Ok(playlist)
    }

    async fn delete_playlist(&self, id: &PlaylistId) -> Result<(), Self::Error> {
        let metadata_path = self
            .find_playlist_metadata_path_by_id(id)
            .await?
            .ok_or_else(|| StorageError::PlaylistNotFound { id: id.to_string() })?;
        let playlist_dir = metadata_path
            .parent()
            .ok_or_else(|| StorageError::FileOperation {
                operation: "find_parent".to_string(),
                path: metadata_path.display().to_string(),
                error: "Missing parent directory".to_string(),
            })?;

        fs::remove_dir_all(playlist_dir)
            .await
            .map_err(|e| StorageError::file_operation("remove_dir_all", playlist_dir, e))?;

        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            let mut guard = self.cache.write().await;
            if let Some(snap) = guard.as_mut() {
                snap.playlists.remove(id);
                snap.last_updated = chrono::Utc::now();
            }
            drop(guard);
            self.mark_dirty();
        }

        Ok(())
    }

    async fn list_playlists(&self) -> Result<Vec<PlaylistId>, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap.playlists.keys().cloned().collect());
            }
        }
        Ok(self
            .list_playlists_from_disk()
            .await?
            .into_iter()
            .map(|p| p.id)
            .collect())
    }

    async fn playlist_exists(&self, id: &PlaylistId) -> Result<bool, Self::Error> {
        if self.cache_enabled {
            self.ensure_cache_initialized().await?;
            if let Some(snap) = self.cache.read().await.as_ref() {
                return Ok(snap.playlists.contains_key(id));
            }
        }
        Ok(self.find_playlist_metadata_path_by_id(id).await?.is_some())
    }

    async fn initialize(&self) -> Result<(), Self::Error> {
        let legacy_playlists_dir = self.data_dir.join("playlists");
        if legacy_playlists_dir.exists() && !self.playlists_dir.exists() {
            fs::rename(&legacy_playlists_dir, &self.playlists_dir)
                .await
                .map_err(|e| {
                    StorageError::file_operation(
                        "rename",
                        &legacy_playlists_dir,
                        std::io::Error::new(
                            e.kind(),
                            format!(
                                "{} -> {} ({})",
                                legacy_playlists_dir.display(),
                                self.playlists_dir.display(),
                                e
                            ),
                        ),
                    )
                })?;
        }

        // Create data directories
        for dir in [
            &self.data_dir,
            &self.podcasts_dir,
            &self.episodes_dir,
            &self.playlists_dir,
        ] {
            fs::create_dir_all(dir)
                .await
                .map_err(|e| StorageError::file_operation("create_dir_all", dir, e))?;
        }

        if self.cache_enabled {
            // Clean up any stale tmp file left over from a crash mid-flush.
            // Safe to ignore errors — absence is the expected case.
            let _ = fs::remove_file(self.cache_index_tmp_path()).await;

            match self.load_cache_from_disk().await {
                Ok(Some(snap)) if snap.schema_version == CACHE_SCHEMA_VERSION => {
                    *self.cache.write().await = Some(snap);
                }
                Ok(Some(snap)) => {
                    eprintln!(
                        "[cache] Index schema {} != expected {}; rebuilding from disk",
                        snap.schema_version, CACHE_SCHEMA_VERSION
                    );
                    let snap = self.build_snapshot_from_disk().await?;
                    *self.cache.write().await = Some(snap);
                    self.cache_dirty.store(true, Ordering::Relaxed);
                }
                Ok(None) => {
                    let snap = self.build_snapshot_from_disk().await?;
                    *self.cache.write().await = Some(snap);
                    self.cache_dirty.store(true, Ordering::Relaxed);
                }
                Err(e) => {
                    eprintln!("[cache] Could not load index ({e}); rebuilding from disk");
                    let snap = self.build_snapshot_from_disk().await?;
                    *self.cache.write().await = Some(snap);
                    self.cache_dirty.store(true, Ordering::Relaxed);
                }
            }

            self.spawn_flush_task();
        }

        Ok(())
    }

    async fn backup(&self, _path: &Path) -> Result<(), Self::Error> {
        // TODO: Implement backup functionality
        // For now, return an error indicating it's not implemented
        Err(StorageError::BackupFailed {
            reason: "Backup functionality not yet implemented".to_string(),
        })
    }

    async fn restore(&self, _path: &Path) -> Result<(), Self::Error> {
        // TODO: Implement restore functionality
        // For now, return an error indicating it's not implemented
        Err(StorageError::RestoreFailed {
            reason: "Restore functionality not yet implemented".to_string(),
        })
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        // TODO: Implement cleanup functionality (remove orphaned files, etc.)
        // For now, this is a no-op
        Ok(())
    }
}

impl Default for JsonStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create JsonStorage with default configuration")
    }
}

/// Atomically write the cache snapshot to disk via temp-file + rename.
///
/// Free function so the spawned background task does not need to hold a
/// reference to `&JsonStorage` (which would couple task lifetime to the
/// instance and complicate the async `move` closure).
///
/// Clears the dirty flag only on a successful rename — a failed write
/// leaves the cache dirty so the next flush retries.
async fn flush_snapshot(
    data_dir: &Path,
    cache: &Arc<RwLock<Option<CacheSnapshot>>>,
    dirty: &AtomicBool,
) -> Result<(), StorageError> {
    if !dirty.load(Ordering::Relaxed) {
        return Ok(());
    }
    let snap = match cache.read().await.clone() {
        Some(s) => s,
        None => {
            // Nothing to flush; clear the flag so we don't busy-loop.
            dirty.store(false, Ordering::Relaxed);
            return Ok(());
        }
    };
    let json = serde_json::to_vec(&snap)?;
    let final_path = data_dir.join(CACHE_FILE_NAME);
    let tmp_path = final_path.with_extension("json.tmp");
    fs::write(&tmp_path, &json)
        .await
        .map_err(|e| StorageError::file_operation("write_cache_index", &tmp_path, e))?;
    fs::rename(&tmp_path, &final_path)
        .await
        .map_err(|e| StorageError::file_operation("rename_cache_index", &final_path, e))?;
    dirty.store(false, Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::{Playlist, PlaylistEpisode, PlaylistId, PlaylistType};
    use crate::podcast::Podcast;
    use tempfile::TempDir;

    fn create_test_storage() -> (JsonStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf());
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_storage_initialization() {
        let (storage, _temp_dir) = create_test_storage();

        let result = storage.initialize().await;
        assert!(result.is_ok());

        assert!(storage.podcasts_dir.exists());
        assert!(storage.episodes_dir.exists());
        assert!(storage.playlists_dir.exists());
    }

    #[tokio::test]
    async fn test_podcast_crud_operations() {
        let (storage, _temp_dir) = create_test_storage();
        storage
            .initialize()
            .await
            .expect("Failed to initialize storage");

        // Create a test podcast
        let podcast = Podcast {
            id: PodcastId::new(),
            title: "Test Podcast".to_string(),
            url: "https://example.com/feed.xml".to_string(),
            description: Some("A test podcast".to_string()),
            author: Some("Test Author".to_string()),
            image_url: None,
            language: None,
            categories: Vec::new(),
            explicit: false,
            last_updated: chrono::Utc::now(),
            episodes: Vec::new(),
            tags: Vec::new(),
        };

        // Save podcast
        let result = storage.save_podcast(&podcast).await;
        assert!(result.is_ok());

        // Check if podcast exists
        let exists = storage
            .podcast_exists(&podcast.id)
            .await
            .expect("Failed to check existence");
        assert!(exists);

        // Load podcast
        let loaded_podcast = storage
            .load_podcast(&podcast.id)
            .await
            .expect("Failed to load podcast");
        assert_eq!(loaded_podcast.id, podcast.id);
        assert_eq!(loaded_podcast.title, podcast.title);

        // List podcasts
        let podcast_ids = storage
            .list_podcasts()
            .await
            .expect("Failed to list podcasts");
        assert_eq!(podcast_ids.len(), 1);
        assert_eq!(podcast_ids[0], podcast.id);

        // Delete podcast
        let result = storage.delete_podcast(&podcast.id).await;
        assert!(result.is_ok());

        // Verify deletion
        let exists = storage
            .podcast_exists(&podcast.id)
            .await
            .expect("Failed to check existence");
        assert!(!exists);
    }

    // Additional tests would go here for episode operations, error handling, etc.

    fn create_test_playlist(name: &str) -> Playlist {
        Playlist {
            id: PlaylistId::new(),
            name: name.to_string(),
            description: Some("Test playlist".to_string()),
            playlist_type: PlaylistType::User,
            episodes: vec![PlaylistEpisode {
                podcast_id: PodcastId::new(),
                episode_id: EpisodeId::new(),
                episode_title: None,
                added_at: chrono::Utc::now(),
                order: 0,
                file_synced: false,
                filename: None,
            }],
            created: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
            smart_rules: None,
        }
    }

    #[tokio::test]
    async fn test_save_load_playlist() {
        let (storage, _temp_dir) = create_test_storage();
        storage
            .initialize()
            .await
            .expect("Failed to initialize storage");

        let playlist = create_test_playlist("Morning Commute");
        storage
            .save_playlist(&playlist)
            .await
            .expect("Failed to save playlist");

        let loaded = storage
            .load_playlist(&playlist.id)
            .await
            .expect("Failed to load playlist");
        assert_eq!(loaded.id, playlist.id);
        assert_eq!(loaded.name, playlist.name);
        assert_eq!(loaded.episodes.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_playlist() {
        let (storage, _temp_dir) = create_test_storage();
        storage
            .initialize()
            .await
            .expect("Failed to initialize storage");

        let playlist = create_test_playlist("Delete Me");
        storage
            .save_playlist(&playlist)
            .await
            .expect("Failed to save playlist");

        storage
            .delete_playlist(&playlist.id)
            .await
            .expect("Failed to delete playlist");

        let exists = storage
            .playlist_exists(&playlist.id)
            .await
            .expect("Failed to check playlist existence");
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_list_playlists() {
        let (storage, _temp_dir) = create_test_storage();
        storage
            .initialize()
            .await
            .expect("Failed to initialize storage");

        let playlist1 = create_test_playlist("P1");
        let playlist2 = create_test_playlist("P2");
        storage
            .save_playlist(&playlist1)
            .await
            .expect("Failed to save first playlist");
        storage
            .save_playlist(&playlist2)
            .await
            .expect("Failed to save second playlist");

        let playlists = storage
            .list_playlists()
            .await
            .expect("Failed to list playlists");
        assert_eq!(playlists.len(), 2);
        assert!(playlists.contains(&playlist1.id));
        assert!(playlists.contains(&playlist2.id));
    }

    #[tokio::test]
    async fn test_playlist_exists() {
        let (storage, _temp_dir) = create_test_storage();
        storage
            .initialize()
            .await
            .expect("Failed to initialize storage");

        let playlist = create_test_playlist("Exists Test");
        let missing_id = PlaylistId::new();

        storage
            .save_playlist(&playlist)
            .await
            .expect("Failed to save playlist");

        let exists = storage
            .playlist_exists(&playlist.id)
            .await
            .expect("Failed to check existing playlist");
        let missing = storage
            .playlist_exists(&missing_id)
            .await
            .expect("Failed to check missing playlist");

        assert!(exists);
        assert!(!missing);
    }

    // ---- Cache layer tests (issue #204) ----

    fn make_test_podcast(title: &str) -> Podcast {
        Podcast {
            id: PodcastId::new(),
            title: title.to_string(),
            url: format!("https://example.com/{}.xml", title),
            description: Some(format!("desc for {}", title)),
            author: Some("Test".to_string()),
            image_url: None,
            language: None,
            categories: Vec::new(),
            explicit: false,
            last_updated: chrono::Utc::now(),
            episodes: Vec::new(),
            tags: Vec::new(),
        }
    }

    fn make_test_episode(podcast_id: &PodcastId, title: &str) -> Episode {
        Episode::new(
            podcast_id.clone(),
            title.to_string(),
            format!("https://example.com/{}.mp3", title),
            chrono::Utc::now(),
        )
    }

    /// Save podcast + episode, then delete the underlying JSON files. Cache should
    /// still serve the data, proving reads come from the cache snapshot.
    #[tokio::test]
    async fn test_cache_round_trip() {
        let (storage, _temp_dir) = create_test_storage();
        storage.initialize().await.unwrap();

        let podcast = make_test_podcast("rt");
        storage.save_podcast(&podcast).await.unwrap();
        let episode = make_test_episode(&podcast.id, "ep1");
        storage.save_episode(&podcast.id, &episode).await.unwrap();

        // Sneak underneath the cache and remove the disk files.
        let podcast_file = storage.podcasts_dir.join(format!("{}.json", podcast.id));
        let episode_file = storage
            .episodes_dir
            .join(podcast.id.to_string())
            .join(format!("{}.json", episode.id));
        std::fs::remove_file(&podcast_file).unwrap();
        std::fs::remove_file(&episode_file).unwrap();

        // Cache must still return the data.
        let loaded = storage.load_podcast(&podcast.id).await.unwrap();
        assert_eq!(loaded.id, podcast.id);
        let eps = storage.load_episodes(&podcast.id).await.unwrap();
        assert_eq!(eps.len(), 1);
        assert_eq!(eps[0].id, episode.id);
        assert!(storage.cache_is_dirty());
    }

    /// With cache disabled, behavior matches the pre-cache implementation: deleting
    /// the underlying file makes the data inaccessible.
    #[tokio::test]
    async fn test_cache_disabled_passthrough() {
        let temp_dir = TempDir::new().unwrap();
        let storage = JsonStorage::with_data_dir(temp_dir.path().to_path_buf()).with_cache(false);
        storage.initialize().await.unwrap();
        assert!(!storage.cache_enabled());

        let podcast = make_test_podcast("disabled");
        storage.save_podcast(&podcast).await.unwrap();

        let podcast_file = storage.podcasts_dir.join(format!("{}.json", podcast.id));
        std::fs::remove_file(&podcast_file).unwrap();

        let result = storage.load_podcast(&podcast.id).await;
        assert!(matches!(result, Err(StorageError::PodcastNotFound { .. })));
    }

    /// Deleting through the storage API must invalidate the cache entry too.
    #[tokio::test]
    async fn test_cache_invalidates_on_delete() {
        let (storage, _temp_dir) = create_test_storage();
        storage.initialize().await.unwrap();

        let podcast = make_test_podcast("inv");
        storage.save_podcast(&podcast).await.unwrap();
        let episode = make_test_episode(&podcast.id, "ep");
        storage.save_episode(&podcast.id, &episode).await.unwrap();

        // Prime the cache via reads.
        let _ = storage.load_podcast(&podcast.id).await.unwrap();
        let _ = storage.load_episodes(&podcast.id).await.unwrap();

        storage
            .delete_episode(&podcast.id, &episode.id)
            .await
            .unwrap();
        let exists = storage
            .episode_exists(&podcast.id, &episode.id)
            .await
            .unwrap();
        assert!(!exists);
        let load_err = storage.load_episode(&podcast.id, &episode.id).await;
        assert!(matches!(
            load_err,
            Err(StorageError::EpisodeNotFound { .. })
        ));

        storage.delete_podcast(&podcast.id).await.unwrap();
        let exists = storage.podcast_exists(&podcast.id).await.unwrap();
        assert!(!exists);
    }

    /// Concurrent writes must all land in both the cache and on disk; nothing is lost.
    #[tokio::test]
    async fn test_cache_concurrent_writes() {
        let (storage, _temp_dir) = create_test_storage();
        storage.initialize().await.unwrap();

        let podcast = make_test_podcast("concurrent");
        storage.save_podcast(&podcast).await.unwrap();

        let storage = Arc::new(storage);
        let podcast_id = podcast.id.clone();
        let mut handles = Vec::new();
        for i in 0..50 {
            let storage = storage.clone();
            let podcast_id = podcast_id.clone();
            handles.push(tokio::spawn(async move {
                let ep = make_test_episode(&podcast_id, &format!("ep{}", i));
                storage.save_episode(&podcast_id, &ep).await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        let eps = storage.load_episodes(&podcast_id).await.unwrap();
        assert_eq!(eps.len(), 50, "all concurrent writes should be cached");

        // And on disk too.
        let disk_dir = storage.episodes_dir.join(podcast_id.to_string());
        let count = std::fs::read_dir(&disk_dir).unwrap().count();
        assert_eq!(count, 50, "all concurrent writes should be persisted");
    }

    fn make_podcast(title: &str) -> Podcast {
        Podcast {
            id: PodcastId::new(),
            title: title.to_string(),
            url: format!("https://example.com/{title}.xml"),
            description: None,
            author: None,
            image_url: None,
            language: None,
            categories: Vec::new(),
            explicit: false,
            last_updated: chrono::Utc::now(),
            episodes: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_cache_persists_across_instances() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Instance A: write data, then explicitly flush.
        {
            let a = JsonStorage::with_data_dir(data_dir.clone());
            a.initialize().await.unwrap();
            a.save_podcast(&make_podcast("Alpha")).await.unwrap();
            a.save_podcast(&make_podcast("Beta")).await.unwrap();
            a.flush_cache_blocking().await.unwrap();

            // The persistent index should exist on disk now.
            assert!(
                data_dir.join(CACHE_FILE_NAME).exists(),
                "cache_index.json should be written by flush_cache_blocking"
            );
        }

        // Instance B: should load index from disk (no rebuild).
        let b = JsonStorage::with_data_dir(data_dir.clone());
        b.initialize().await.unwrap();
        // After init via index load, dirty stays false (no rebuild happened).
        assert!(
            !b.cache_dirty.load(Ordering::Relaxed),
            "loading a valid index should not mark dirty"
        );
        let podcasts = b.list_podcasts().await.unwrap();
        assert_eq!(podcasts.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_schema_mismatch_rebuilds() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Seed disk with one podcast via a clean storage instance.
        {
            let a = JsonStorage::with_data_dir(data_dir.clone());
            a.initialize().await.unwrap();
            a.save_podcast(&make_podcast("Gamma")).await.unwrap();
            a.flush_cache_blocking().await.unwrap();
        }

        // Hand-write a cache index with a bogus schema version.
        let bad = serde_json::json!({
            "schema_version": 999,
            "last_updated": chrono::Utc::now(),
            "podcasts": {},
            "episodes": {},
            "playlists": {},
        });
        std::fs::write(
            data_dir.join(CACHE_FILE_NAME),
            serde_json::to_vec(&bad).unwrap(),
        )
        .unwrap();

        // Initialize a new instance: must rebuild from disk and overwrite the bad index.
        let b = JsonStorage::with_data_dir(data_dir.clone());
        b.initialize().await.unwrap();
        let podcasts = b.list_podcasts().await.unwrap();
        assert_eq!(
            podcasts.len(),
            1,
            "rebuild should recover the saved podcast"
        );
        assert!(
            b.cache_dirty.load(Ordering::Relaxed),
            "schema mismatch should mark cache dirty for re-flush"
        );
    }

    #[tokio::test]
    async fn test_cache_atomic_flush_ignores_stale_tmp() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(&data_dir).unwrap();

        // Pre-create a stale .tmp file (simulating a crash mid-flush).
        let tmp_path = data_dir.join("cache_index.json.tmp");
        std::fs::write(&tmp_path, b"garbage that should be ignored").unwrap();

        // Initialize: should clean up the stale tmp without crashing.
        let storage = JsonStorage::with_data_dir(data_dir.clone());
        storage.initialize().await.unwrap();
        assert!(
            !tmp_path.exists(),
            "stale tmp file should be removed on initialize"
        );

        // A subsequent successful flush should produce a clean index.
        storage.save_podcast(&make_podcast("Delta")).await.unwrap();
        storage.flush_cache_blocking().await.unwrap();
        assert!(data_dir.join(CACHE_FILE_NAME).exists());
        assert!(!tmp_path.exists(), "tmp should not linger after rename");
    }

    #[tokio::test]
    async fn test_cache_corrupt_index_rebuilds() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Seed real data and a valid index.
        {
            let a = JsonStorage::with_data_dir(data_dir.clone());
            a.initialize().await.unwrap();
            a.save_podcast(&make_podcast("Epsilon")).await.unwrap();
            a.flush_cache_blocking().await.unwrap();
        }

        // Corrupt the index file with garbage.
        std::fs::write(data_dir.join(CACHE_FILE_NAME), b"{not valid json at all").unwrap();

        // Initialize must not error; should rebuild from disk.
        let b = JsonStorage::with_data_dir(data_dir.clone());
        b.initialize()
            .await
            .expect("init should recover from a corrupt index");
        let podcasts = b.list_podcasts().await.unwrap();
        assert_eq!(podcasts.len(), 1, "rebuild should recover saved podcast");
        assert!(
            b.cache_dirty.load(Ordering::Relaxed),
            "corrupt index should mark cache dirty"
        );
    }

    #[tokio::test]
    async fn test_rebuild_cache_returns_counts() {
        let (storage, _td) = create_test_storage();
        storage.initialize().await.unwrap();
        let p = make_podcast("Zeta");
        let pid = p.id.clone();
        storage.save_podcast(&p).await.unwrap();
        for i in 0..3 {
            let e = make_test_episode(&pid, &format!("ep {i}"));
            storage.save_episode(&pid, &e).await.unwrap();
        }
        let (pcount, ecount) = storage.rebuild_cache().await.unwrap();
        assert_eq!(pcount, 1);
        assert_eq!(ecount, 3);
    }
}
