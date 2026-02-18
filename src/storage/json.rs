use async_trait::async_trait;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::playlist::{Playlist, PlaylistId};
use crate::podcast::{Episode, Podcast};
use crate::storage::{EpisodeId, PodcastId, Storage, StorageError};
use crate::utils::text::strip_html;
use crate::utils::validation::sanitize_playlist_name;

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
pub struct JsonStorage {
    pub data_dir: PathBuf,
    podcasts_dir: PathBuf,
    episodes_dir: PathBuf,
    playlists_dir: PathBuf,
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
        let podcasts_dir = data_dir.join("podcasts");
        let episodes_dir = data_dir.join("episodes");
        let playlists_dir = data_dir.join("Playlists");

        Ok(Self {
            data_dir,
            podcasts_dir,
            episodes_dir,
            playlists_dir,
        })
    }

    /// Create a new JSON storage instance with custom data directory
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        let podcasts_dir = data_dir.join("podcasts");
        let episodes_dir = data_dir.join("episodes");
        let playlists_dir = data_dir.join("Playlists");

        Self {
            data_dir,
            podcasts_dir,
            episodes_dir,
            playlists_dir,
        }
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
}

#[async_trait]
impl Storage for JsonStorage {
    type Error = StorageError;

    async fn save_podcast(&self, podcast: &Podcast) -> Result<(), Self::Error> {
        let path = self.podcast_path(&podcast.id);
        let json = serde_json::to_string_pretty(podcast)?;

        self.atomic_write(&path, &json).await
    }

    async fn load_podcast(&self, id: &PodcastId) -> Result<Podcast, Self::Error> {
        let path = self.podcast_path(id);

        if !path.exists() {
            return Err(StorageError::PodcastNotFound { id: id.clone() });
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| StorageError::file_operation("read", &path, e))?;

        let mut podcast: Podcast = serde_json::from_str(&content)?;

        // Migration: Clean HTML from descriptions for podcasts stored before fix
        if let Some(ref description) = podcast.description {
            if description.contains('<') || description.contains("&lt;") {
                podcast.description = Some(strip_html(description));
            }
        }

        Ok(podcast)
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

        Ok(())
    }

    async fn list_podcasts(&self) -> Result<Vec<PodcastId>, Self::Error> {
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

    async fn podcast_exists(&self, id: &PodcastId) -> Result<bool, Self::Error> {
        Ok(self.podcast_path(id).exists())
    }

    async fn save_episode(
        &self,
        podcast_id: &PodcastId,
        episode: &Episode,
    ) -> Result<(), Self::Error> {
        let path = self.episode_path(podcast_id, &episode.id);
        let json = serde_json::to_string_pretty(episode)?;

        self.atomic_write(&path, &json).await
    }

    async fn load_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<Episode, Self::Error> {
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

        // Migration: Clean HTML from descriptions for episodes stored before fix
        // This ensures existing episodes with HTML get sanitized on load
        if let Some(ref description) = episode.description {
            if description.contains('<') || description.contains("&lt;") {
                episode.description = Some(strip_html(description));
            }
        }

        Ok(episode)
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

        Ok(())
    }

    async fn list_episodes(&self, podcast_id: &PodcastId) -> Result<Vec<EpisodeId>, Self::Error> {
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

    async fn episode_exists(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<bool, Self::Error> {
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
        let episode_ids = self.list_episodes(podcast_id).await?;
        let mut episodes = Vec::with_capacity(episode_ids.len());

        for episode_id in episode_ids {
            let episode = self.load_episode(podcast_id, &episode_id).await?;
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
        self.atomic_write(&metadata_path, &json).await
    }

    async fn load_playlist(&self, id: &PlaylistId) -> Result<Playlist, Self::Error> {
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

        Ok(())
    }

    async fn list_playlists(&self) -> Result<Vec<PlaylistId>, Self::Error> {
        if !self.playlists_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&self.playlists_dir)
            .await
            .map_err(|e| StorageError::file_operation("read_dir", &self.playlists_dir, e))?;

        let mut ids = Vec::new();
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
            ids.push(playlist.id);
        }

        Ok(ids)
    }

    async fn playlist_exists(&self, id: &PlaylistId) -> Result<bool, Self::Error> {
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
}
