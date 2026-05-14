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

/// Build a unique temp path adjacent to `path` so concurrent writers
/// targeting the same destination cannot race on the same temp file.
///
/// The returned path lives in the same directory as `path` (so the
/// subsequent `rename` is on the same filesystem and stays atomic) and
/// embeds a fresh UUID to guarantee uniqueness across concurrent calls.
/// Format: `<original_filename>.<uuid>.tmp` — e.g.,
/// `41f3ed28-cash-daddies.json` → `41f3ed28-cash-daddies.json.<uuid>.tmp`.
fn unique_temp_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let suffix = uuid::Uuid::new_v4().simple();
    parent.join(format!("{base}.{suffix}.tmp"))
}

/// Returns `true` if `name` matches the unique-temp-file naming
/// convention `<original>.<32-hex-uuid>.tmp` produced by
/// [`unique_temp_path`]. Used by startup cleanup to identify orphan
/// temp files left over from a crash mid-write without touching legit
/// `.json` files that happen to share a directory.
fn is_orphan_temp_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".tmp") else {
        return false;
    };
    let Some(idx) = stem.rfind('.') else {
        return false;
    };
    let suffix = &stem[idx + 1..];
    suffix.len() == 32 && suffix.chars().all(|c| c.is_ascii_hexdigit())
}

/// Remove orphan `*.<uuid>.tmp` files left in `dir` from a crashed
/// `atomic_write`. Recurses one level deep to cover nested layouts
/// (`episodes_dir/<podcast_id>/`, `playlists_dir/<name>/`).
///
/// Returns the count of orphans removed. All errors are swallowed —
/// orphan cleanup is best-effort and must never block startup.
async fn cleanup_orphan_temp_files(dir: &Path) -> usize {
    let mut removed = 0;
    let Ok(mut entries) = fs::read_dir(dir).await else {
        return 0;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if file_type.is_file() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if is_orphan_temp_filename(name) && fs::remove_file(&path).await.is_ok() {
                    removed += 1;
                }
            }
        } else if file_type.is_dir() {
            // One level of recursion: episodes_dir/<podcast_id>/
            // and playlists_dir/<name>/ are the only nested write
            // targets that produce uniquely-suffixed temp files.
            if let Ok(mut sub_entries) = fs::read_dir(&path).await {
                while let Ok(Some(sub)) = sub_entries.next_entry().await {
                    let sub_path = sub.path();
                    let Ok(sub_type) = sub.file_type().await else {
                        continue;
                    };
                    if !sub_type.is_file() {
                        continue;
                    }
                    if let Some(name) = sub_path.file_name().and_then(|s| s.to_str()) {
                        if is_orphan_temp_filename(name) && fs::remove_file(&sub_path).await.is_ok()
                        {
                            removed += 1;
                        }
                    }
                }
            }
        }
    }
    removed
}

/// Schema version of the persistent cache index. Bump whenever the on-disk
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

    /// Atomic write operation to prevent data corruption.
    ///
    /// Writes `content` to a unique temp file in the same directory, then
    /// atomically renames it onto `path`. The temp filename includes a
    /// random UUID so concurrent saves of the same target file cannot race
    /// on the same temp path (the cause of the torn-write corruption
    /// observed in production — see issue #230). On rename failure the
    /// temp file is best-effort cleaned up to avoid leaks.
    async fn atomic_write(&self, path: &Path, content: &str) -> Result<(), StorageError> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::file_operation("create_dir_all", parent, e))?;
        }

        let temp_path = unique_temp_path(path);

        // Write the full content to the temp file. If a previous run left a
        // file with the same name (impossible in practice given the UUID,
        // but defensive), `fs::write` truncates and overwrites.
        if let Err(e) = fs::write(&temp_path, content).await {
            // Best-effort cleanup of the partial temp file.
            let _ = fs::remove_file(&temp_path).await;
            return Err(StorageError::file_operation("write_temp", &temp_path, e));
        }

        // Atomically replace the destination. On Windows this uses
        // MoveFileExW with MOVEFILE_REPLACE_EXISTING semantics; on POSIX
        // rename(2) is atomic on the same filesystem.
        if let Err(e) = fs::rename(&temp_path, path).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(StorageError::file_operation("rename", path, e));
        }

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
        match fs::try_exists(&path).await {
            Ok(true) => {}
            Ok(false) => return Ok(None),
            Err(e) => return Err(StorageError::file_operation("stat_cache_index", &path, e)),
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

    /// Rebuild the in-memory snapshot from disk. When caching is enabled
    /// (the default), the freshly built snapshot is also flushed to the
    /// persistent index immediately. When caching is disabled, only the
    /// in-memory snapshot is replaced — there is no persistent index to write.
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

        // Orphan temp-file cleanup runs unconditionally: `atomic_write`
        // (per-record saves and the cache-index flush alike) produces
        // `*.<uuid>.tmp` files in these directories regardless of
        // whether the in-memory cache is enabled. Gating this behind
        // `cache_enabled` would let orphans accumulate forever on
        // installs that have opted out of the cache.
        //
        // The legacy deterministic `cache_index.json.tmp` cleanup is
        // kept for one release cycle so users upgrading from pre-#233
        // still get their old orphan removed. It can be dropped in a
        // later release once the migration window has elapsed.
        let _ = fs::remove_file(self.cache_index_tmp_path()).await;

        // Sweep orphan `*.<uuid>.tmp` files from a crashed write.
        // Post-#233 every `atomic_write` (per-record + cache index)
        // uses a unique suffix that the legacy single-path cleanup
        // above no longer covers, so without this sweep orphans
        // accumulate forever on each crash.
        let mut orphans = 0;
        orphans += cleanup_orphan_temp_files(&self.data_dir).await;
        orphans += cleanup_orphan_temp_files(&self.podcasts_dir).await;
        orphans += cleanup_orphan_temp_files(&self.episodes_dir).await;
        orphans += cleanup_orphan_temp_files(&self.playlists_dir).await;
        if orphans > 0 {
            eprintln!("[storage] removed {orphans} orphan temp file(s) from crashed writes");
        }

        if self.cache_enabled {
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
/// **Concurrency contract**: claims the dirty flag with a `swap(false)`
/// up front, snapshots the cache, then writes. If the write fails, the
/// flag is restored to `true` so the next interval retries. If a writer
/// re-marks dirty *after* the swap but *before* the rename completes,
/// their flag set is preserved — we never overwrite their `true` with
/// `false`. This avoids the race where a concurrent write could be
/// silently dropped from the persistent index.
async fn flush_snapshot(
    data_dir: &Path,
    cache: &Arc<RwLock<Option<CacheSnapshot>>>,
    dirty: &AtomicBool,
) -> Result<(), StorageError> {
    // Atomically claim the dirty flag. If it was already false, another
    // call beat us to it — nothing to do.
    if !dirty.swap(false, Ordering::AcqRel) {
        return Ok(());
    }
    let snap = match cache.read().await.clone() {
        Some(s) => s,
        None => {
            // Nothing to flush. Don't restore the flag — there's literally
            // no cache to persist, so leaving it false is correct.
            return Ok(());
        }
    };
    // From here on, any error path must restore the dirty flag so the
    // next interval retries the flush. Use a small helper closure to keep
    // the restore explicit at every early return.
    let restore_on_err = |e| {
        dirty.store(true, Ordering::Release);
        e
    };
    let json = serde_json::to_vec(&snap).map_err(|e| restore_on_err(StorageError::from(e)))?;
    let final_path = data_dir.join(CACHE_FILE_NAME);
    let tmp_path = unique_temp_path(&final_path);
    if let Err(e) = fs::write(&tmp_path, &json).await {
        let _ = fs::remove_file(&tmp_path).await;
        return Err(restore_on_err(StorageError::file_operation(
            "write_cache_index",
            &tmp_path,
            e,
        )));
    }
    if let Err(e) = fs::rename(&tmp_path, &final_path).await {
        let _ = fs::remove_file(&tmp_path).await;
        return Err(restore_on_err(StorageError::file_operation(
            "rename_cache_index",
            &final_path,
            e,
        )));
    }
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

    /// Regression test for the race documented on `flush_snapshot`: a
    /// writer that sets `dirty = true` between the snapshot clone and the
    /// final atomic update must not have its flag overwritten.
    ///
    /// We can't easily hit the "between clone and rename" window
    /// deterministically, but we can verify the contract: after a
    /// successful `flush_cache_blocking()`, if we immediately mark dirty
    /// again, a *second* flush still sees and persists the new state.
    #[tokio::test]
    async fn test_flush_does_not_clobber_concurrent_dirty_marker() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let storage = JsonStorage::with_data_dir(data_dir.clone());
        storage.initialize().await.unwrap();
        storage.save_podcast(&make_podcast("First")).await.unwrap();
        storage.flush_cache_blocking().await.unwrap();
        assert!(!storage.cache_dirty.load(Ordering::Relaxed));

        // A new write marks dirty; the next flush must see it.
        storage.save_podcast(&make_podcast("Second")).await.unwrap();
        assert!(storage.cache_dirty.load(Ordering::Relaxed));
        storage.flush_cache_blocking().await.unwrap();
        assert!(!storage.cache_dirty.load(Ordering::Relaxed));

        // Reload from disk in a fresh instance; both podcasts must be present.
        let b = JsonStorage::with_data_dir(data_dir);
        b.initialize().await.unwrap();
        let podcasts = b.list_podcasts().await.unwrap();
        assert_eq!(
            podcasts.len(),
            2,
            "second write must have been persisted in the second flush"
        );
    }

    // ─── Atomic-write regression tests (issue #230) ───────────────────────

    #[test]
    fn test_unique_temp_path_is_unique_per_call() {
        let target = Path::new("/tmp/data/podcasts/abc.json");
        let a = unique_temp_path(target);
        let b = unique_temp_path(target);
        assert_ne!(a, b, "two calls must produce distinct temp paths");
        // Sibling of target so rename stays on the same filesystem.
        assert_eq!(a.parent(), target.parent());
        assert_eq!(b.parent(), target.parent());
        // Distinguishable as a temp file.
        assert!(a.to_string_lossy().ends_with(".tmp"));
        assert!(b.to_string_lossy().ends_with(".tmp"));
        // Original filename preserved as the prefix so debugging is easy.
        let a_name = a.file_name().unwrap().to_string_lossy();
        assert!(a_name.starts_with("abc.json."), "got {a_name}");
    }

    /// Direct reproduction of the production bug: a long file is replaced
    /// with shorter content. Before the fix, a torn write could leave the
    /// trailing bytes of the longer file appended after the new shorter
    /// JSON, producing the `trailing characters at line N column M` error
    /// the user hit on launch. After the fix, the rename always replaces
    /// the destination wholesale.
    #[tokio::test]
    async fn test_atomic_write_long_to_short_no_trailing_bytes() {
        let (storage, _td) = create_test_storage();
        storage.initialize().await.unwrap();

        // Save a "long" podcast: many categories blow up the serialized size.
        let mut long = make_podcast("Long Podcast");
        long.categories = (0..500)
            .map(|i| format!("Category number {i:04} with some padding text"))
            .collect();
        storage.save_podcast(&long).await.unwrap();

        let path = storage.podcast_path(&long.id);
        let long_size = std::fs::metadata(&path).unwrap().len();
        assert!(long_size > 1000, "sanity: long podcast should be >1KB");

        // Save a "short" version of the same podcast (same id, no categories).
        let mut short = long.clone();
        short.categories.clear();
        storage.save_podcast(&short).await.unwrap();

        // The file on disk must be exactly the new content, with no
        // trailing bytes from the previous longer write.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        let parsed: Podcast =
            serde_json::from_str(&on_disk).expect("file must be parseable JSON, no trailing bytes");
        assert_eq!(parsed.id, long.id);
        assert!(parsed.categories.is_empty());

        // serde_json must consume the entire file (tail-strict parse).
        let mut de = serde_json::Deserializer::from_str(&on_disk);
        let _: Podcast = serde::Deserialize::deserialize(&mut de).unwrap();
        de.end()
            .expect("no trailing characters after the JSON value");
    }

    /// Concurrent saves of the SAME podcast id used to race on the same
    /// deterministic temp path (`<id>.tmp`), interleaving bytes from
    /// different writers and producing a torn file. With per-call unique
    /// temp paths the final file is always one writer's complete output.
    #[tokio::test]
    async fn test_atomic_write_concurrent_saves_same_target_no_corruption() {
        let (storage, _td) = create_test_storage();
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        let base = make_podcast("Race Target");
        let id = base.id.clone();

        // Spawn many concurrent saves of the same podcast with different
        // payload sizes — the sweet spot for triggering torn-write.
        let mut handles = Vec::new();
        for i in 0..32 {
            let storage = storage.clone();
            let mut p = base.clone();
            p.title = format!("Race Target #{i:03}");
            p.categories = (0..(i * 5))
                .map(|j| format!("cat-{j:04}-padding"))
                .collect();
            handles.push(tokio::spawn(async move { storage.save_podcast(&p).await }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }

        // Whichever writer won the rename race, the resulting file must
        // be a single complete podcast — never a torn mix of two writes.
        let path = storage.podcast_path(&id);
        let on_disk = std::fs::read_to_string(&path).unwrap();
        let _parsed: Podcast = serde_json::from_str(&on_disk)
            .expect("concurrent saves must never produce unparseable JSON");
        let mut de = serde_json::Deserializer::from_str(&on_disk);
        let _: Podcast = serde::Deserialize::deserialize(&mut de).unwrap();
        de.end()
            .expect("no trailing characters after concurrent saves");

        // Loading via the storage API must also succeed (covers cache path).
        let loaded = storage.load_podcast(&id).await.unwrap();
        assert_eq!(loaded.id, id);
    }

    /// Same race but for episodes — they share the atomic_write helper
    /// but go through a different per-record path, so cover them too.
    #[tokio::test]
    async fn test_atomic_write_concurrent_episode_saves_no_corruption() {
        use crate::podcast::Episode;
        let (storage, _td) = create_test_storage();
        storage.initialize().await.unwrap();
        let storage = Arc::new(storage);

        let podcast = make_podcast("Ep Host");
        storage.save_podcast(&podcast).await.unwrap();

        let ep = Episode::new(
            podcast.id.clone(),
            "Concurrent Episode".to_string(),
            "https://example.com/a.mp3".to_string(),
            chrono::Utc::now(),
        );

        let mut handles = Vec::new();
        for i in 0..16 {
            let storage = storage.clone();
            let podcast_id = podcast.id.clone();
            let mut e = ep.clone();
            e.title = format!("Concurrent Episode {i:03}");
            e.description = Some("padding ".repeat(i * 20));
            handles.push(tokio::spawn(async move {
                storage.save_episode(&podcast_id, &e).await
            }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }

        let path = storage.episode_path(&podcast.id, &ep.id);
        let on_disk = std::fs::read_to_string(&path).unwrap();
        let _: Episode = serde_json::from_str(&on_disk)
            .expect("concurrent episode saves must never produce unparseable JSON");
    }

    /// If the final `rename` step fails (e.g., destination is a non-empty
    /// directory on POSIX, or otherwise unreplaceable), `atomic_write` must
    /// (1) return an error, (2) leave the original destination untouched,
    /// and (3) clean up the partial temp file rather than leaking it.
    #[tokio::test]
    async fn test_atomic_write_rename_failure_cleans_up_temp_and_preserves_dest() {
        let (storage, td) = create_test_storage();
        storage.initialize().await.unwrap();

        // Create a destination path that is itself a non-empty directory.
        // `rename(file, non_empty_dir)` fails on both POSIX and Windows.
        let dest = td.path().join("blocked_target.json");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("sentinel.txt"), b"do not touch").unwrap();

        let result = storage.atomic_write(&dest, "{\"new\":true}").await;
        assert!(
            result.is_err(),
            "atomic_write must surface the rename failure as an error"
        );

        // Original destination must be intact (still a directory with the
        // sentinel file inside, untouched by the failed write).
        assert!(dest.is_dir(), "original destination must be preserved");
        let sentinel = std::fs::read_to_string(dest.join("sentinel.txt")).unwrap();
        assert_eq!(sentinel, "do not touch");

        // Temp file must have been cleaned up — no orphan `*.tmp` files
        // should be left in the parent directory.
        let leftover_tmps: Vec<_> = std::fs::read_dir(td.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "tmp")
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            leftover_tmps.is_empty(),
            "rename-failure cleanup must remove the temp file, found: {:?}",
            leftover_tmps.iter().map(|e| e.path()).collect::<Vec<_>>()
        );
    }

    // ─── Orphan temp-file cleanup tests (issue #235) ──────────────────────

    #[test]
    fn test_is_orphan_temp_filename_matches_unique_pattern() {
        // Real output of unique_temp_path uses the simple (no-dash) UUID
        // form, which is exactly 32 lowercase hex chars.
        assert!(is_orphan_temp_filename(
            "abc.json.0123456789abcdef0123456789abcdef.tmp"
        ));
        assert!(is_orphan_temp_filename(
            "cache_index.json.deadbeefdeadbeefdeadbeefdeadbeef.tmp"
        ));
    }

    #[test]
    fn test_is_orphan_temp_filename_rejects_legit_files() {
        assert!(!is_orphan_temp_filename("abc.json"));
        assert!(!is_orphan_temp_filename("abc.tmp"));
        // Legacy short suffix — too short to be a UUID.
        assert!(!is_orphan_temp_filename("cache_index.json.tmp"));
        // Right length but non-hex.
        assert!(!is_orphan_temp_filename(
            "abc.json.zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz.tmp"
        ));
        // Wrong length.
        assert!(!is_orphan_temp_filename("abc.json.beef.tmp"));
        // Wrong extension.
        assert!(!is_orphan_temp_filename(
            "abc.json.0123456789abcdef0123456789abcdef.bak"
        ));
    }

    /// Pre-create an orphan `*.<uuid>.tmp` in each storage directory
    /// (and one nested in the per-podcast episodes subdir) and confirm
    /// `initialize` sweeps them all on the next launch.
    #[tokio::test]
    async fn test_initialize_removes_orphan_temp_files_in_all_dirs() {
        let (storage, _td) = create_test_storage();
        // Need the directory tree to exist before we plant orphans.
        storage.initialize().await.unwrap();

        let uuid = uuid::Uuid::new_v4().simple().to_string();
        let podcast_orphan = storage
            .podcasts_dir
            .join(format!("some-id.json.{uuid}.tmp"));
        let cache_orphan = storage
            .data_dir
            .join(format!("cache_index.json.{uuid}.tmp"));
        let nested_episode_dir = storage.episodes_dir.join("podcast-A");
        std::fs::create_dir_all(&nested_episode_dir).unwrap();
        let episode_orphan = nested_episode_dir.join(format!("ep-1.json.{uuid}.tmp"));
        let nested_playlist_dir = storage.playlists_dir.join("My-Playlist");
        std::fs::create_dir_all(&nested_playlist_dir).unwrap();
        let playlist_orphan = nested_playlist_dir.join(format!("playlist.json.{uuid}.tmp"));

        for p in [
            &podcast_orphan,
            &cache_orphan,
            &episode_orphan,
            &playlist_orphan,
        ] {
            std::fs::write(p, b"partial-write").unwrap();
            assert!(p.exists(), "precondition: orphan must exist before sweep");
        }

        // Re-run initialize on the same data dir to trigger the sweep.
        storage.initialize().await.unwrap();

        for p in [
            &podcast_orphan,
            &cache_orphan,
            &episode_orphan,
            &playlist_orphan,
        ] {
            assert!(
                !p.exists(),
                "orphan must be removed after initialize: {}",
                p.display()
            );
        }
    }

    /// Regression guard: orphan cleanup must never touch legit `*.json`
    /// files or non-matching temp-like files (e.g., the legacy
    /// deterministic `cache_index.json.tmp` is removed by a separate
    /// path, not by this sweep).
    #[tokio::test]
    async fn test_initialize_preserves_non_orphan_files() {
        let (storage, _td) = create_test_storage();
        storage.initialize().await.unwrap();

        // Save a real podcast through the public API so the on-disk
        // file is well-formed and the cache rebuild on the next
        // initialize call doesn't choke on it.
        let podcast = make_podcast("Real Podcast");
        let podcast_id = podcast.id.clone();
        storage.save_podcast(&podcast).await.unwrap();
        let podcast_json = storage.podcasts_dir.join(format!("{podcast_id}.json"));
        assert!(podcast_json.exists(), "precondition: podcast file written");

        // A `.tmp` file that is NOT a unique-suffix orphan — the sweep
        // must leave it alone (it could be a user backup, an in-flight
        // legacy cache temp handled elsewhere, etc.).
        let unrelated_tmp = storage.data_dir.join("user-backup.tmp");
        std::fs::write(&unrelated_tmp, b"keep me").unwrap();

        storage.initialize().await.unwrap();

        assert!(
            podcast_json.exists(),
            "real podcast file must survive sweep"
        );
        assert!(
            unrelated_tmp.exists(),
            "non-uuid `.tmp` file must survive sweep"
        );
    }

    /// Regression guard for the cache-gating bug: orphan temp files are
    /// produced by `atomic_write` regardless of whether the in-memory
    /// cache is enabled, so the startup sweep must run even when the
    /// user has set `storage.cache_enabled = false`.
    #[tokio::test]
    async fn test_initialize_cleans_orphans_with_cache_disabled() {
        let td = tempfile::tempdir().unwrap();
        let storage = JsonStorage::with_data_dir(td.path().to_path_buf()).with_cache(false);
        storage.initialize().await.unwrap();

        let orphan = storage
            .podcasts_dir
            .join(format!("abc.{}.tmp", uuid::Uuid::new_v4().simple()));
        let legacy = storage.data_dir.join("cache_index.json.tmp");
        std::fs::write(&orphan, b"x").unwrap();
        std::fs::write(&legacy, b"x").unwrap();

        storage.initialize().await.unwrap();

        assert!(
            !orphan.exists(),
            "orphan `.uuid.tmp` must be removed even when cache is disabled"
        );
        assert!(
            !legacy.exists(),
            "legacy `cache_index.json.tmp` must be removed even when cache is disabled"
        );
    }
}
