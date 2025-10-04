use crate::podcast::{Episode, EpisodeStatus};
use crate::storage::{EpisodeId, PodcastId, Storage};
use anyhow::Result;
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Invalid file path: {0}")]
    InvalidPath(String),
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub episode_id: EpisodeId,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    InProgress,
    Completed,
    Failed(String),
}

/// Simple download manager for MVP
pub struct DownloadManager<S: Storage> {
    storage: Arc<S>,
    downloads_dir: PathBuf,
    client: reqwest::Client,
}

impl<S: Storage> DownloadManager<S> {
    pub fn new(storage: Arc<S>, downloads_dir: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60)) // Longer timeout for downloads
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(10)) // Handle redirects
            .user_agent("podcast-tui/1.0.0-mvp (like FeedReader)")
            .build()?;

        Ok(Self {
            storage,
            downloads_dir,
            client,
        })
    }

    /// Get a reference to the storage
    pub fn storage(&self) -> &Arc<S> {
        &self.storage
    }

    /// Clean up stuck downloads on startup - resets episodes stuck in "Downloading" status
    /// when there's no actual download happening
    pub async fn cleanup_stuck_downloads(&self) -> Result<(), DownloadError> {
        // Load all podcast IDs
        let podcast_ids = self
            .storage
            .list_podcasts()
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        for podcast_id in podcast_ids {
            // Load episodes for this podcast
            let episodes = self
                .storage
                .load_episodes(&podcast_id)
                .await
                .map_err(|e| DownloadError::Storage(e.to_string()))?;

            for mut episode in episodes {
                // Reset stuck "Downloading" episodes back to "New" status
                if matches!(episode.status, EpisodeStatus::Downloading) {
                    // Check if the file actually exists and is complete
                    let should_reset = if let Some(ref local_path) = episode.local_path {
                        !local_path.exists()
                    } else {
                        true // No local path means definitely not downloaded
                    };

                    if should_reset {
                        episode.status = EpisodeStatus::New;
                        episode.local_path = None;

                        self.storage
                            .save_episode(&podcast_id, &episode)
                            .await
                            .map_err(|e| DownloadError::Storage(e.to_string()))?;
                    }
                }
            }
        }

        Ok(())
    }
    /// Download an episode (simple implementation)
    pub async fn download_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<(), DownloadError> {
        // Load episode from storage
        let mut episode = self
            .storage
            .load_episode(podcast_id, episode_id)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        // Create download directory
        let podcast_dir = self.downloads_dir.join(podcast_id.to_string());
        fs::create_dir_all(&podcast_dir).await?;

        // Generate filename
        let filename = self.generate_filename(&episode)?;
        let file_path = podcast_dir.join(&filename);

        // Skip if already downloaded
        if file_path.exists() {
            episode.local_path = Some(file_path);
            episode.status = EpisodeStatus::Downloaded;
            self.storage
                .save_episode(podcast_id, &episode)
                .await
                .map_err(|e| DownloadError::Storage(e.to_string()))?;
            return Ok(());
        }

        // Update status to downloading
        episode.status = EpisodeStatus::Downloading;
        self.storage
            .save_episode(podcast_id, &episode)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        // Get the audio URL - if empty, try using the GUID as fallback
        let audio_url = if episode.audio_url.is_empty() {
            // Check if GUID looks like a URL and use it as fallback
            episode
                .guid
                .as_ref()
                .filter(|guid| guid.starts_with("http"))
                .map(|s| s.as_str())
                .unwrap_or("")
        } else {
            &episode.audio_url
        };

        if audio_url.is_empty() {
            // Mark episode as failed with specific reason
            episode.status = EpisodeStatus::DownloadFailed;
            self.storage
                .save_episode(podcast_id, &episode)
                .await
                .map_err(|e| DownloadError::Storage(e.to_string()))?;

            return Err(DownloadError::InvalidPath(
                "No audio URL found for episode".to_string(),
            ));
        }

        // Download the file
        match self.download_file(audio_url, &file_path).await {
            Ok(_) => {
                episode.status = EpisodeStatus::Downloaded;
                episode.local_path = Some(file_path);
            }
            Err(e) => {
                episode.status = EpisodeStatus::DownloadFailed;
                // Clean up partial file
                let _ = fs::remove_file(&file_path).await;
                return Err(e);
            }
        }

        // Save updated episode
        self.storage
            .save_episode(podcast_id, &episode)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Delete downloaded episode file
    pub async fn delete_episode(
        &self,
        podcast_id: &PodcastId,
        episode_id: &EpisodeId,
    ) -> Result<(), DownloadError> {
        let mut episode = self
            .storage
            .load_episode(podcast_id, episode_id)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        if let Some(ref local_path) = episode.local_path {
            if local_path.exists() {
                fs::remove_file(local_path).await?;
            }
            episode.local_path = None;
            episode.status = if episode.status == EpisodeStatus::Downloaded {
                EpisodeStatus::New
            } else {
                episode.status
            };

            self.storage
                .save_episode(podcast_id, &episode)
                .await
                .map_err(|e| DownloadError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// Delete all downloaded episodes and clean up the downloads folder
    pub async fn delete_all_downloads(&self) -> Result<usize, DownloadError> {
        // Load all podcast IDs
        let podcast_ids = self
            .storage
            .list_podcasts()
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for podcast_id in podcast_ids {
            // Load episodes for this podcast
            let episodes = self
                .storage
                .load_episodes(&podcast_id)
                .await
                .map_err(|e| DownloadError::Storage(e.to_string()))?;

            for mut episode in episodes {
                // Only process downloaded episodes
                if matches!(episode.status, EpisodeStatus::Downloaded) {
                    if let Some(ref local_path) = episode.local_path {
                        // Try to delete the file
                        if local_path.exists() {
                            match fs::remove_file(local_path).await {
                                Ok(_) => {
                                    deleted_count += 1;
                                    // Update episode status
                                    episode.local_path = None;
                                    episode.status = EpisodeStatus::New;

                                    // Save updated episode
                                    if let Err(_) =
                                        self.storage.save_episode(&podcast_id, &episode).await
                                    {
                                        failed_count += 1;
                                    }
                                }
                                Err(_) => {
                                    failed_count += 1;
                                }
                            }
                        } else {
                            // File doesn't exist, but episode thinks it's downloaded
                            // Clean up the status
                            episode.local_path = None;
                            episode.status = EpisodeStatus::New;

                            if let Err(_) = self.storage.save_episode(&podcast_id, &episode).await {
                                failed_count += 1;
                            }
                        }
                    }
                }
            }
        }

        // Clean up empty directories in downloads folder
        self.cleanup_empty_directories().await?;

        if failed_count > 0 {
            return Err(DownloadError::Storage(format!(
                "Deleted {} files, but {} operations failed",
                deleted_count, failed_count
            )));
        }

        Ok(deleted_count)
    }

    /// Clean up empty podcast directories in the downloads folder
    async fn cleanup_empty_directories(&self) -> Result<(), DownloadError> {
        if !self.downloads_dir.exists() {
            return Ok(());
        }

        let mut dir_entries = fs::read_dir(&self.downloads_dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let dir_path = entry.path();

                // Check if directory is empty
                let mut subdir_entries = fs::read_dir(&dir_path).await?;
                if subdir_entries.next_entry().await?.is_none() {
                    // Directory is empty, remove it
                    let _ = fs::remove_dir(&dir_path).await;
                }
            }
        }

        Ok(())
    }

    /// Simple file download implementation
    async fn download_file(&self, url: &str, path: &Path) -> Result<(), DownloadError> {
        let response = self.client.get(url).send().await?;
        let mut file = fs::File::create(path).await?;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
        }

        file.sync_all().await?;
        Ok(())
    }

    /// Generate safe filename for episode
    fn generate_filename(&self, episode: &Episode) -> Result<String, DownloadError> {
        // Sanitize title for filename
        let safe_title = episode
            .title
            .chars()
            .filter(|c| c.is_alphanumeric() || " -_".contains(*c))
            .collect::<String>()
            .trim()
            .replace(' ', "_");

        // Get extension from URL or default to mp3
        let extension = if !episode.audio_url.is_empty() {
            episode
                .audio_url
                .split('.')
                .last()
                .and_then(|ext| {
                    let ext = ext.split('?').next().unwrap_or(ext);
                    if ["mp3", "m4a", "ogg", "wav"].contains(&ext) {
                        Some(ext)
                    } else {
                        None
                    }
                })
                .unwrap_or("mp3")
        } else {
            // If no audio URL, check GUID for extension
            episode
                .guid
                .as_ref()
                .and_then(|guid| {
                    guid.split('.').last().and_then(|ext| {
                        let ext = ext.split('?').next().unwrap_or(ext);
                        if ["mp3", "m4a", "ogg", "wav"].contains(&ext) {
                            Some(ext)
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or("mp3")
        };

        let filename = format!("{}_{}.{}", episode.id, safe_title, extension);

        // Ensure filename isn't too long
        if filename.len() > 200 {
            Ok(format!("{}.{}", episode.id, extension))
        } else {
            Ok(filename)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podcast::Episode;
    use crate::storage::{JsonStorage, PodcastId};
    use chrono::Utc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_generate_filename() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let manager = DownloadManager::new(storage, downloads_dir).unwrap();

        let episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/episode.mp3".to_string(),
            Utc::now(),
        );

        let filename = manager.generate_filename(&episode).unwrap();
        assert!(filename.contains("Test_Episode"));
        assert!(filename.ends_with(".mp3"));
    }
}
