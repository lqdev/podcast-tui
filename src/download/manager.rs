use crate::config::DownloadConfig;
use crate::podcast::{Episode, EpisodeStatus};
use crate::storage::{EpisodeId, PodcastId, Storage};
use anyhow::Result;
use chrono::Datelike;
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
    #[error("Sync error: {0}")]
    Sync(String),
}

/// Device sync error types
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Device path not found or not accessible: {0}")]
    DevicePathInvalid(String),
    #[error("Failed to copy file: {0}")]
    CopyFailed(String),
    #[error("Failed to delete file: {0}")]
    DeleteFailed(String),
}

/// Report of sync operations performed
#[derive(Debug, Clone)]
pub struct SyncReport {
    pub files_copied: Vec<PathBuf>,
    pub files_deleted: Vec<PathBuf>,
    pub files_skipped: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, String)>,
}

impl SyncReport {
    /// Create a new empty sync report
    pub fn new() -> Self {
        Self {
            files_copied: Vec::new(),
            files_deleted: Vec::new(),
            files_skipped: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Check if sync was successful (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get total number of operations performed
    pub fn total_operations(&self) -> usize {
        self.files_copied.len() + self.files_deleted.len() + self.files_skipped.len()
    }
}

impl Default for SyncReport {
    fn default() -> Self {
        Self::new()
    }
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
    config: DownloadConfig,
}

impl<S: Storage> DownloadManager<S> {
    pub fn new(storage: Arc<S>, downloads_dir: PathBuf, config: DownloadConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60)) // Longer timeout for downloads
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(10)) // Handle redirects
            .user_agent("Mozilla/5.0 (compatible; podcast-tui/1.0; +https://github.com/podcast-tui) AppleWebKit/537.36 (KHTML, like Gecko)")
            .build()?;

        Ok(Self {
            storage,
            downloads_dir,
            client,
            config,
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

        // Load podcast information for folder naming
        let podcast = self
            .storage
            .load_podcast(podcast_id)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        // Generate folder name based on configuration
        let folder_name = self.generate_podcast_folder_name(&podcast);
        let podcast_dir = self.downloads_dir.join(folder_name);

        // Create download directory
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
                episode.local_path = Some(file_path.clone());

                // Embed ID3 metadata if configured and file is MP3
                if self.config.embed_id3_metadata
                    && file_path.extension().map_or(false, |ext| ext == "mp3")
                {
                    if let Err(e) = self
                        .embed_id3_metadata(&file_path, &episode, &podcast)
                        .await
                    {
                        // Log the error but don't fail the download
                        eprintln!("Warning: Failed to embed ID3 metadata: {}", e);
                    }
                }
            }
            Err(e) => {
                episode.status = EpisodeStatus::DownloadFailed;
                // Clean up partial file
                let _ = fs::remove_file(&file_path).await;
                self.storage
                    .save_episode(podcast_id, &episode)
                    .await
                    .map_err(|e| DownloadError::Storage(e.to_string()))?;
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

    /// Delete all downloaded episodes for a specific podcast
    /// This is called when unsubscribing from a podcast to clean up downloaded files
    pub async fn delete_podcast_downloads(
        &self,
        podcast_id: &PodcastId,
    ) -> Result<usize, DownloadError> {
        // Load podcast info before deleting episodes (needed for folder cleanup)
        let podcast = self
            .storage
            .load_podcast(podcast_id)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        let folder_name = self.generate_podcast_folder_name(&podcast);

        // Load episodes for this podcast
        let episodes = self
            .storage
            .load_episodes(podcast_id)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        let mut deleted_count = 0;
        let mut failed_count = 0;

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
                                    self.storage.save_episode(podcast_id, &episode).await
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

                        if let Err(_) = self.storage.save_episode(podcast_id, &episode).await {
                            failed_count += 1;
                        }
                    }
                }
            }
        }

        // Try to remove the podcast-specific directory if it exists and is empty
        self.cleanup_podcast_directory_by_name(&folder_name).await?;

        if failed_count > 0 {
            return Err(DownloadError::Storage(format!(
                "Deleted {} files for podcast, but {} operations failed",
                deleted_count, failed_count
            )));
        }

        Ok(deleted_count)
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

    /// Delete downloaded episodes whose files are older than `max_age_days` days.
    ///
    /// Convenience wrapper that converts days to hours and delegates to
    /// `cleanup_old_downloads_hours`. Uses file modification time to determine age.
    /// Returns the number of episodes cleaned up.
    pub async fn cleanup_old_downloads(&self, max_age_days: u32) -> Result<usize, DownloadError> {
        self.cleanup_old_downloads_hours((max_age_days as u64) * 24)
            .await
    }

    /// Delete downloaded episodes whose files are older than `max_age_hours` hours.
    ///
    /// Uses file modification time to determine age. Episodes whose files are
    /// older than the cutoff are deleted and their status is reset to `New`.
    /// Returns the number of episodes cleaned up.
    pub async fn cleanup_old_downloads_hours(
        &self,
        max_age_hours: u64,
    ) -> Result<usize, DownloadError> {
        if max_age_hours == 0 {
            return Err(DownloadError::Storage(
                "max_age_hours must be greater than 0".to_string(),
            ));
        }

        let seconds = max_age_hours
            .checked_mul(3600)
            .ok_or_else(|| DownloadError::Storage("max_age_hours is too large".to_string()))?;

        let duration = std::time::Duration::from_secs(seconds);
        let cutoff = std::time::SystemTime::now()
            .checked_sub(duration)
            .ok_or_else(|| {
                DownloadError::Storage(
                    "max_age_hours is too large relative to system time".to_string(),
                )
            })?;

        let mut deleted_count: usize = 0;
        let mut failed_count: usize = 0;

        let podcast_ids = self
            .storage
            .list_podcasts()
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        for podcast_id in &podcast_ids {
            let episodes = self
                .storage
                .load_episodes(podcast_id)
                .await
                .map_err(|e| DownloadError::Storage(e.to_string()))?;

            for mut episode in episodes {
                if !matches!(episode.status, EpisodeStatus::Downloaded) {
                    continue;
                }
                if let Some(ref local_path) = episode.local_path {
                    if local_path.exists() {
                        if let Ok(metadata) = fs::metadata(local_path).await {
                            if let Ok(modified) = metadata.modified() {
                                if modified < cutoff {
                                    // File is old enough — delete it
                                    match fs::remove_file(local_path).await {
                                        Ok(_) => {
                                            episode.status = EpisodeStatus::New;
                                            episode.local_path = None;
                                            if let Err(_) = self
                                                .storage
                                                .save_episode(podcast_id, &episode)
                                                .await
                                            {
                                                failed_count += 1;
                                            }
                                            deleted_count += 1;
                                        }
                                        Err(_) => {
                                            failed_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // File doesn't exist, but episode thinks it's downloaded
                        // Clean up the stale status
                        episode.status = EpisodeStatus::New;
                        episode.local_path = None;
                        if let Err(_) = self.storage.save_episode(podcast_id, &episode).await {
                            failed_count += 1;
                        }
                    }
                }
            }
        }

        // Clean up any empty directories left behind
        self.cleanup_empty_directories().await?;

        if failed_count > 0 {
            return Err(DownloadError::Storage(format!(
                "Cleaned up {} files, but {} operations failed",
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

    /// Clean up podcast-specific directory after deleting its episodes
    #[allow(dead_code)]
    async fn cleanup_podcast_directory(&self, podcast_id: &PodcastId) -> Result<(), DownloadError> {
        if !self.downloads_dir.exists() {
            return Ok(());
        }

        // Load podcast to get folder name
        let podcast = self
            .storage
            .load_podcast(podcast_id)
            .await
            .map_err(|e| DownloadError::Storage(e.to_string()))?;

        let folder_name = self.generate_podcast_folder_name(&podcast);
        self.cleanup_podcast_directory_by_name(&folder_name).await
    }

    /// Clean up podcast-specific directory by folder name
    async fn cleanup_podcast_directory_by_name(
        &self,
        folder_name: &str,
    ) -> Result<(), DownloadError> {
        if !self.downloads_dir.exists() {
            return Ok(());
        }

        let podcast_dir = self.downloads_dir.join(folder_name);

        if podcast_dir.exists() {
            // Check if directory is empty
            match fs::read_dir(&podcast_dir).await {
                Ok(mut entries) => {
                    if entries.next_entry().await?.is_none() {
                        // Directory is empty, remove it
                        let _ = fs::remove_dir(&podcast_dir).await;
                    }
                }
                Err(_) => {
                    // Directory doesn't exist or can't be read, ignore
                }
            }
        }

        Ok(())
    }

    /// Simple file download implementation
    async fn download_file(&self, url: &str, path: &Path) -> Result<(), DownloadError> {
        let response = self.client.get(url).send().await?;

        // Check if the response is successful, otherwise error_for_status will return an error
        let response = response.error_for_status()?;

        // Get content type to verify it's actually audio
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("unknown");

        // Reject downloads that are not audio files
        // This catches cases where servers return HTML error pages with 200 OK status
        let is_audio = content_type.starts_with("audio/") 
            || content_type == "application/octet-stream"
            || content_type.starts_with("video/") // Some podcasts use video MIME types
            || content_type == "binary/octet-stream"
            || content_type == "unknown"; // Allow unknown content type as fallback

        if !is_audio && content_type.contains("html") {
            return Err(DownloadError::InvalidPath(format!(
                "Server returned HTML instead of audio file (Content-Type: {}). The audio URL may be invalid or the file may have been removed.",
                content_type
            )));
        }

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
        let mut filename_parts = Vec::new();

        // Add episode number if configured and available
        if self.config.include_episode_numbers {
            if let Some(episode_num) = episode.episode_number {
                filename_parts.push(format!("{:03}", episode_num));
            }
        }

        // Add date if configured
        if self.config.include_dates {
            let date_str = episode.published.format("%Y-%m-%d").to_string();
            filename_parts.push(date_str);
        }

        // Clean and add episode title using robust sanitization
        let title = self.sanitize_filename(&episode.title, false);

        filename_parts.push(title);

        // Join parts with underscores
        let mut base_filename = filename_parts.join("_");

        // Truncate if too long (reserve space for extension)
        let max_base_len = self.config.max_filename_length.saturating_sub(4); // Reserve for .mp3
        if base_filename.len() > max_base_len {
            base_filename.truncate(max_base_len);
            // Ensure we don't cut in the middle of a UTF-8 character
            while !base_filename.is_char_boundary(base_filename.len()) {
                base_filename.pop();
            }
        }

        // Determine extension from audio URL
        let extension = if !episode.audio_url.is_empty() {
            episode
                .audio_url
                .split('.')
                .next_back() // Use next_back instead of last for DoubleEndedIterator
                .and_then(|ext| {
                    let ext = ext.split('?').next().unwrap_or(ext); // Remove query params
                    match ext.to_lowercase().as_str() {
                        "mp3" | "m4a" | "aac" | "ogg" | "wav" | "flac" => Some(ext.to_lowercase()),
                        _ => None,
                    }
                })
                .unwrap_or_else(|| "mp3".to_string())
        } else {
            // Fallback: try to get extension from GUID if it looks like a URL
            episode
                .guid
                .as_ref()
                .and_then(|guid| {
                    guid.split('.').next_back().and_then(|ext| {
                        let ext = ext.split('?').next().unwrap_or(ext);
                        match ext.to_lowercase().as_str() {
                            "mp3" | "m4a" | "aac" | "ogg" | "wav" | "flac" => {
                                Some(ext.to_lowercase())
                            }
                            _ => None,
                        }
                    })
                })
                .unwrap_or_else(|| "mp3".to_string())
        };

        Ok(format!("{}.{}", base_filename, extension))
    }

    /// Generate podcast folder name based on configuration with robust cross-platform sanitization
    fn generate_podcast_folder_name(&self, podcast: &crate::podcast::Podcast) -> String {
        if self.config.use_readable_folders {
            self.sanitize_filename(&podcast.title, true)
        } else {
            // Use UUID for guaranteed uniqueness
            podcast.id.to_string()
        }
    }

    /// Comprehensive cross-platform filename sanitization
    /// Based on research of Windows, macOS, and Linux compatibility requirements
    fn sanitize_filename(&self, input: &str, is_folder: bool) -> String {
        // Step 1: Handle empty or whitespace-only input
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return "Untitled".to_string();
        }

        // Step 2: Replace prohibited characters with safe alternatives
        let mut sanitized = String::new();
        for ch in trimmed.chars() {
            match ch {
                // Windows prohibited characters - replace with safe alternatives
                '<' => sanitized.push('('),
                '>' => sanitized.push(')'),
                ':' => sanitized.push('-'), // Common in titles like "Episode 1: Introduction"
                '"' => sanitized.push('\''), // Replace with single quote
                '/' => sanitized.push('-'), // Path separator
                '\\' => sanitized.push('-'), // Windows path separator
                '|' => sanitized.push('-'), // Pipe symbol
                '?' => sanitized.push_str(""), // Remove question marks to avoid confusion
                '*' => sanitized.push_str(""), // Remove wildcards

                // Control characters (ASCII 0-31) - remove entirely
                c if c.is_control() => {} // Skip control characters

                // Unicode quotes and special characters - normalize to ASCII
                '\u{201C}' | '\u{201D}' => sanitized.push('\''), // Smart double quotes to straight quote
                '\u{2018}' | '\u{2019}' => sanitized.push('\''), // Smart single quotes
                '\u{2026}' => sanitized.push_str("..."),         // Ellipsis to three dots
                '\u{2013}' | '\u{2014}' => sanitized.push('-'),  // En/em dash to hyphen

                // Keep safe characters
                c if c.is_ascii_alphanumeric() => sanitized.push(ch),
                ' ' | '-' | '_' | '(' | ')' => sanitized.push(ch),

                // Handle periods carefully
                '.' => {
                    // Don't allow leading periods (creates hidden files on Unix)
                    if !sanitized.is_empty() {
                        sanitized.push('.');
                    }
                }

                // Convert other Unicode to ASCII equivalents or remove
                'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => sanitized.push('a'),
                'é' | 'è' | 'ê' | 'ë' | 'ē' => sanitized.push('e'),
                'í' | 'ì' | 'î' | 'ï' | 'ī' => sanitized.push('i'),
                'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' => sanitized.push('o'),
                'ú' | 'ù' | 'û' | 'ü' | 'ū' => sanitized.push('u'),
                'ñ' => sanitized.push('n'),
                'ç' => sanitized.push('c'),
                'ý' | 'ÿ' => sanitized.push('y'),

                // Capital versions
                'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' | 'Ā' => sanitized.push('A'),
                'É' | 'È' | 'Ê' | 'Ë' | 'Ē' => sanitized.push('E'),
                'Í' | 'Ì' | 'Î' | 'Ï' | 'Ī' => sanitized.push('I'),
                'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ō' => sanitized.push('O'),
                'Ú' | 'Ù' | 'Û' | 'Ü' | 'Ū' => sanitized.push('U'),
                'Ñ' => sanitized.push('N'),
                'Ç' => sanitized.push('C'),
                'Ý' | 'Ÿ' => sanitized.push('Y'),

                // Other common symbols - remove or replace
                '&' => sanitized.push_str("and"),
                '@' => sanitized.push_str("at"),
                '%' => sanitized.push_str("percent"),
                '#' => sanitized.push_str("number"),
                '+' => sanitized.push_str("plus"),
                '=' => sanitized.push('-'),

                // Skip other characters
                _ => {}
            }
        }

        // Step 3: Clean up multiple consecutive separators
        let cleaned = sanitized
            .split_whitespace() // Split on whitespace
            .collect::<Vec<_>>() // Collect into vector
            .join(" ") // Rejoin with single spaces
            .replace("  ", " ") // Remove any remaining double spaces
            .replace("--", "-") // Remove double hyphens
            .replace("__", "_") // Remove double underscores
            .replace(" - ", "-") // Clean up spaced hyphens
            .replace(" _ ", "_"); // Clean up spaced underscores

        // Step 4: Trim and handle edge cases
        let mut final_name = cleaned.trim().to_string();

        // Don't allow names that end with period or space (Windows restriction)
        while final_name.ends_with('.') || final_name.ends_with(' ') {
            final_name.pop();
        }

        // Don't allow names that start with period (creates hidden files)
        while final_name.starts_with('.') {
            final_name = final_name.chars().skip(1).collect();
        }

        // Handle Windows reserved device names
        final_name = self.handle_reserved_names(final_name);

        // Step 5: Ensure we have something meaningful
        if final_name.trim().is_empty() {
            final_name = if is_folder {
                "Podcast".to_string()
            } else {
                "Episode".to_string()
            };
        }

        // Step 6: Enforce length limits (140 chars for safety across all systems)
        if final_name.len() > 140 {
            // Try to truncate at word boundary
            if let Some(last_space) = final_name[..140].rfind(' ') {
                final_name.truncate(last_space);
            } else {
                final_name.truncate(140);
            }

            // Ensure we didn't cut off in the middle of a UTF-8 character
            while !final_name.is_char_boundary(final_name.len()) {
                final_name.pop();
            }
        }

        // Final cleanup
        final_name.trim().to_string()
    }

    /// Handle Windows reserved device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
    fn handle_reserved_names(&self, mut name: String) -> String {
        let upper_name = name.to_uppercase();
        let reserved_names = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        // Check if the name (without extension) is a reserved name
        let name_without_ext = if let Some(dot_pos) = upper_name.find('.') {
            &upper_name[..dot_pos]
        } else {
            &upper_name
        };

        if reserved_names.contains(&name_without_ext) {
            // Add underscore prefix to make it safe
            name = format!("_{}", name);
        }

        name
    }

    /// Embed ID3 metadata into an MP3 file
    async fn embed_id3_metadata(
        &self,
        file_path: &Path,
        episode: &Episode,
        podcast: &crate::podcast::Podcast,
    ) -> Result<(), DownloadError> {
        use id3::{Tag, TagLike};

        // Create or load existing tag
        let mut tag = Tag::read_from_path(file_path).unwrap_or_default();

        // Set basic metadata
        tag.set_title(&episode.title);
        tag.set_artist(&podcast.title);
        tag.set_album(&podcast.title);
        tag.set_genre("Podcast");

        // Set track number if available (use episode_number)
        if let Some(episode_num) = episode.episode_number {
            tag.set_track(episode_num);
        }

        // Set year from publication date
        let year = episode.published.year();
        if year > 0 {
            tag.set_year(year);
        }

        // Set comment with description (truncated if necessary)
        if let Some(ref description) = episode.description {
            let comment = if description.len() > self.config.max_id3_comment_length {
                let mut truncated = description
                    .chars()
                    .take(self.config.max_id3_comment_length.saturating_sub(3))
                    .collect::<String>();
                truncated.push_str("...");
                truncated
            } else {
                description.clone()
            };
            // Use add_frame with a Comment frame
            let comment_frame = id3::frame::Comment {
                lang: "eng".to_string(),
                description: "".to_string(),
                text: comment,
            };
            tag.add_frame(comment_frame);
        }

        // Download and embed artwork if configured
        if self.config.download_artwork {
            if let Some(ref artwork_url) = podcast.image_url {
                if let Ok(artwork_data) = self.download_artwork(artwork_url).await {
                    let picture = id3::frame::Picture {
                        mime_type: artwork_data.0,
                        picture_type: id3::frame::PictureType::CoverFront,
                        description: "Cover".to_string(),
                        data: artwork_data.1,
                    };
                    tag.add_frame(picture);
                }
            }
        }

        // Write the tag back to the file
        tag.write_to_path(file_path, id3::Version::Id3v23)
            .map_err(|e| DownloadError::InvalidPath(format!("Failed to write ID3 tags: {}", e)))?;

        Ok(())
    }

    /// Download artwork and return MIME type and data
    async fn download_artwork(&self, url: &str) -> Result<(String, Vec<u8>), DownloadError> {
        let response = self.client.get(url).send().await?;

        // Check if the response is successful
        let response = response.error_for_status()?;

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let data = response.bytes().await?.to_vec();

        // Validate it's actually an image and convert if needed
        let (final_mime_type, final_data) = match image::load_from_memory(&data) {
            Ok(img) => {
                // Convert to JPEG for maximum compatibility
                let mut jpeg_data = Vec::new();
                img.write_to(
                    &mut std::io::Cursor::new(&mut jpeg_data),
                    image::ImageFormat::Jpeg,
                )
                .map_err(|e| {
                    DownloadError::InvalidPath(format!("Failed to convert image: {}", e))
                })?;
                ("image/jpeg".to_string(), jpeg_data)
            }
            Err(_) => {
                // If we can't decode it as an image, return as-is
                (content_type, data)
            }
        };

        Ok((final_mime_type, final_data))
    }

    /// Sync downloaded episodes to a device using metadata comparison
    ///
    /// # Arguments
    /// * `device_path` - Path to the device/directory to sync to
    /// * `delete_orphans` - Whether to delete files on device not present on PC
    /// * `dry_run` - If true, only report what would be done without making changes
    ///
    /// # Returns
    /// A `SyncReport` detailing all operations performed or that would be performed
    pub async fn sync_to_device(
        &self,
        device_path: PathBuf,
        playlists_dir: Option<PathBuf>,
        delete_orphans: bool,
        dry_run: bool,
    ) -> Result<SyncReport, SyncError> {
        let mut report = SyncReport::new();

        // Validate device path exists and is accessible
        if !device_path.exists() {
            return Err(SyncError::DevicePathInvalid(format!(
                "Path does not exist: {}",
                device_path.display()
            )));
        }

        if !device_path.is_dir() {
            return Err(SyncError::DevicePathInvalid(format!(
                "Path is not a directory: {}",
                device_path.display()
            )));
        }

        // Test write access by trying to create and remove a temporary file
        let test_file = device_path.join(".podcast-tui-sync-test");
        if let Err(e) = fs::write(&test_file, b"test").await {
            return Err(SyncError::DevicePathInvalid(format!(
                "Path is not writable: {} ({})",
                device_path.display(),
                e
            )));
        }
        let _ = fs::remove_file(&test_file).await;

        // Step 1: Build a map of all downloaded episodes on PC
        let mut pc_files: std::collections::HashMap<PathBuf, (PathBuf, u64)> =
            std::collections::HashMap::new();

        // Scan downloads directory with Podcasts/ prefix
        self.scan_directory_with_prefix(&self.downloads_dir, Path::new("Podcasts"), &mut pc_files)
            .await?;

        // Scan playlists directory with Playlists/ prefix
        if let Some(playlists_dir) = &playlists_dir {
            if playlists_dir.exists() {
                self.scan_playlists_for_sync(playlists_dir, &mut pc_files)
                    .await?;
            }
        }

        // Step 2: Build a map of all files on the device
        let mut device_files: std::collections::HashMap<PathBuf, (PathBuf, u64)> =
            std::collections::HashMap::new();

        self.scan_directory(&device_path, &mut device_files).await?;

        // Step 3: Determine what needs to be copied (new or changed files)
        for (relative_path, (source_path, source_size)) in &pc_files {
            if let Some((device_file_path, device_size)) = device_files.get(relative_path) {
                // File exists on device - check if size matches
                if source_size == device_size {
                    // File is identical (by metadata), skip
                    report.files_skipped.push(relative_path.clone());
                } else {
                    // File size differs, needs update
                    if !dry_run {
                        match self
                            .copy_file_to_device(source_path, device_file_path)
                            .await
                        {
                            Ok(_) => report.files_copied.push(relative_path.clone()),
                            Err(e) => report
                                .errors
                                .push((relative_path.clone(), format!("Copy failed: {}", e))),
                        }
                    } else {
                        report.files_copied.push(relative_path.clone());
                    }
                }
            } else {
                // File doesn't exist on device, needs to be copied
                let target_path = device_path.join(relative_path);

                // Create parent directories if needed
                if let Some(parent) = target_path.parent() {
                    if !dry_run {
                        if let Err(e) = fs::create_dir_all(parent).await {
                            report.errors.push((
                                relative_path.clone(),
                                format!("Failed to create directory: {}", e),
                            ));
                            continue;
                        }
                    }
                }

                if !dry_run {
                    match self.copy_file_to_device(source_path, &target_path).await {
                        Ok(_) => report.files_copied.push(relative_path.clone()),
                        Err(e) => report
                            .errors
                            .push((relative_path.clone(), format!("Copy failed: {}", e))),
                    }
                } else {
                    report.files_copied.push(relative_path.clone());
                }
            }
        }

        // Step 4: Delete orphan files on device (files not present on PC)
        if delete_orphans {
            for (relative_path, (device_file_path, _)) in &device_files {
                if !pc_files.contains_key(relative_path) {
                    // File exists on device but not on PC, delete it
                    if !dry_run {
                        match fs::remove_file(device_file_path).await {
                            Ok(_) => report.files_deleted.push(relative_path.clone()),
                            Err(e) => report
                                .errors
                                .push((relative_path.clone(), format!("Delete failed: {}", e))),
                        }
                    } else {
                        report.files_deleted.push(relative_path.clone());
                    }
                }
            }

            // Clean up empty directories on device (only if not dry run)
            if !dry_run {
                let _ = self.cleanup_empty_directories_in(&device_path).await;
            }
        }

        Ok(report)
    }

    /// Recursively scan a directory and build a map of relative paths to (absolute path, file size)
    ///
    /// # Arguments
    /// * `root_path` - The original root path for computing relative paths
    /// * `current_path` - The current directory being scanned (for recursion)
    /// * `files` - The map to populate with file information
    fn scan_directory_impl<'a>(
        &'a self,
        root_path: &'a Path,
        current_path: &'a Path,
        files: &'a mut std::collections::HashMap<PathBuf, (PathBuf, u64)>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SyncError>> + 'a + Send>>
    {
        Box::pin(async move {
            let mut entries = fs::read_dir(current_path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let metadata = entry.metadata().await?;

                if metadata.is_file() {
                    // Only include audio files
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if matches!(
                            ext_str.as_str(),
                            "mp3" | "m4a" | "aac" | "ogg" | "wav" | "flac"
                        ) {
                            // Calculate relative path from root
                            let relative_path = path
                                .strip_prefix(root_path)
                                .map_err(|e| {
                                    SyncError::Io(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        format!("Failed to compute relative path: {}", e),
                                    ))
                                })?
                                .to_path_buf();

                            files.insert(relative_path, (path.clone(), metadata.len()));
                        }
                    }
                } else if metadata.is_dir() {
                    // Recursively scan subdirectories
                    self.scan_directory_impl(root_path, &path, files).await?;
                }
            }

            Ok(())
        })
    }

    /// Wrapper for scan_directory_impl that uses the same path for root and current
    fn scan_directory<'a>(
        &'a self,
        base_path: &'a Path,
        files: &'a mut std::collections::HashMap<PathBuf, (PathBuf, u64)>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SyncError>> + 'a + Send>>
    {
        Box::pin(async move { self.scan_directory_impl(base_path, base_path, files).await })
    }

    /// Scan a directory and prefix all discovered relative paths.
    fn scan_directory_with_prefix<'a>(
        &'a self,
        source_dir: &'a Path,
        prefix: &'a Path,
        files: &'a mut std::collections::HashMap<PathBuf, (PathBuf, u64)>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SyncError>> + 'a + Send>>
    {
        Box::pin(async move {
            if !source_dir.exists() {
                return Ok(());
            }

            let mut raw_files: std::collections::HashMap<PathBuf, (PathBuf, u64)> =
                std::collections::HashMap::new();
            self.scan_directory_impl(source_dir, source_dir, &mut raw_files)
                .await?;

            for (relative_path, (absolute_path, size)) in raw_files {
                let prefixed = prefix.join(relative_path);
                files.insert(prefixed, (absolute_path, size));
            }

            Ok(())
        })
    }

    /// Scan playlist audio directories and map files to Playlists/{playlist_name}/...
    fn scan_playlists_for_sync<'a>(
        &'a self,
        playlists_dir: &'a Path,
        files: &'a mut std::collections::HashMap<PathBuf, (PathBuf, u64)>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SyncError>> + 'a + Send>>
    {
        Box::pin(async move {
            let mut entries = fs::read_dir(playlists_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let playlist_dir = entry.path();
                if !playlist_dir.is_dir() {
                    continue;
                }

                let playlist_name = playlist_dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if playlist_name.is_empty() {
                    continue;
                }

                let audio_dir = playlist_dir.join("audio");
                if !audio_dir.exists() {
                    continue;
                }

                let mut playlist_files: std::collections::HashMap<PathBuf, (PathBuf, u64)> =
                    std::collections::HashMap::new();
                self.scan_directory_impl(&audio_dir, &audio_dir, &mut playlist_files)
                    .await?;
                if playlist_files.is_empty() {
                    continue;
                }

                let prefix = Path::new("Playlists").join(playlist_name);
                for (relative_path, (absolute_path, size)) in playlist_files {
                    files.insert(prefix.join(relative_path), (absolute_path, size));
                }
            }

            Ok(())
        })
    }

    /// Copy a file from source to destination with error handling
    async fn copy_file_to_device(
        &self,
        source_path: &Path,
        dest_path: &Path,
    ) -> Result<(), SyncError> {
        // Use tokio's fs::copy which is more efficient
        fs::copy(source_path, dest_path)
            .await
            .map_err(|e| SyncError::CopyFailed(format!("{}: {}", dest_path.display(), e)))?;
        Ok(())
    }

    /// Clean up empty directories within a given path
    fn cleanup_empty_directories_in<'a>(
        &'a self,
        base_path: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SyncError>> + 'a + Send>>
    {
        Box::pin(async move {
            let mut entries = fs::read_dir(base_path).await?;
            let mut subdirs = Vec::new();

            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    subdirs.push(entry.path());
                }
            }

            // Recursively clean subdirectories first
            for subdir in subdirs {
                let _ = self.cleanup_empty_directories_in(&subdir).await;

                // After cleaning subdirectory, check if it's now empty
                if let Ok(mut sub_entries) = fs::read_dir(&subdir).await {
                    if sub_entries.next_entry().await?.is_none() {
                        // Directory is empty, remove it
                        let _ = fs::remove_dir(&subdir).await;
                    }
                }
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DownloadConfig;
    use crate::podcast::Episode;
    use crate::storage::{JsonStorage, PodcastId};
    use chrono::Utc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_generate_filename() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        let episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/episode.mp3".to_string(),
            Utc::now(),
        );

        let filename = manager.generate_filename(&episode).unwrap();
        // With default config, it includes dates and preserves title formatting
        assert!(filename.contains("Test Episode"));
        assert!(filename.ends_with(".mp3"));
        // Should include date in YYYY-MM-DD format
        assert!(filename.contains(&Utc::now().format("%Y-%m-%d").to_string()));
    }

    #[tokio::test]
    async fn test_sync_report_creation() {
        let report = SyncReport::new();
        assert_eq!(report.files_copied.len(), 0);
        assert_eq!(report.files_deleted.len(), 0);
        assert_eq!(report.files_skipped.len(), 0);
        assert_eq!(report.errors.len(), 0);
        assert!(report.is_success());
        assert_eq!(report.total_operations(), 0);
    }

    #[tokio::test]
    async fn test_sync_to_device_invalid_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        let invalid_path = PathBuf::from("/nonexistent/path");
        let result = manager
            .sync_to_device(invalid_path, None, true, false)
            .await;

        assert!(result.is_err());
        match result {
            Err(SyncError::DevicePathInvalid(_)) => {
                // Expected error
            }
            _ => panic!("Expected DevicePathInvalid error"),
        }
    }

    #[tokio::test]
    async fn test_sync_to_device_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        // Create a test audio file in downloads
        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        let test_file = podcast_dir.join("episode1.mp3");
        fs::write(&test_file, b"test audio content").await.unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        // Create device directory
        let device_path = temp_dir.path().join("device");
        fs::create_dir_all(&device_path).await.unwrap();

        // Run sync in dry-run mode
        let report = manager
            .sync_to_device(device_path.clone(), None, false, true)
            .await
            .unwrap();

        // Verify dry-run results
        assert_eq!(report.files_copied.len(), 1);
        assert_eq!(report.files_deleted.len(), 0);
        assert_eq!(report.errors.len(), 0);
        assert!(report.is_success());

        // Verify no files were actually copied
        let device_podcast_dir = device_path.join("Podcasts").join("Test Podcast");
        assert!(!device_podcast_dir.exists());
    }

    #[tokio::test]
    async fn test_sync_to_device_copy_new_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        // Create test audio files in downloads
        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        let test_file1 = podcast_dir.join("episode1.mp3");
        let test_file2 = podcast_dir.join("episode2.mp3");
        fs::write(&test_file1, b"test audio content 1")
            .await
            .unwrap();
        fs::write(&test_file2, b"test audio content 2")
            .await
            .unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        // Create device directory
        let device_path = temp_dir.path().join("device");
        fs::create_dir_all(&device_path).await.unwrap();

        // Run sync
        let report = manager
            .sync_to_device(device_path.clone(), None, false, false)
            .await
            .unwrap();

        // Verify sync results
        assert_eq!(report.files_copied.len(), 2);
        assert_eq!(report.files_deleted.len(), 0);
        assert_eq!(report.errors.len(), 0);
        assert!(report.is_success());

        // Verify files were actually copied
        let device_podcast_dir = device_path.join("Podcasts").join("Test Podcast");
        assert!(device_podcast_dir.exists());
        assert!(device_podcast_dir.join("episode1.mp3").exists());
        assert!(device_podcast_dir.join("episode2.mp3").exists());
    }

    #[tokio::test]
    async fn test_sync_to_device_skip_identical_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        // Create test audio file in downloads
        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        let test_file = podcast_dir.join("episode1.mp3");
        let content = b"test audio content";
        fs::write(&test_file, content).await.unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        // Create device directory with same file
        let device_path = temp_dir.path().join("device");
        let device_podcast_dir = device_path.join("Podcasts").join("Test Podcast");
        fs::create_dir_all(&device_podcast_dir).await.unwrap();
        let device_file = device_podcast_dir.join("episode1.mp3");
        fs::write(&device_file, content).await.unwrap();

        // Run sync
        let report = manager
            .sync_to_device(device_path.clone(), None, false, false)
            .await
            .unwrap();

        // Verify file was skipped
        assert_eq!(report.files_copied.len(), 0);
        assert_eq!(report.files_skipped.len(), 1);
        assert_eq!(report.files_deleted.len(), 0);
        assert_eq!(report.errors.len(), 0);
        assert!(report.is_success());
    }

    #[tokio::test]
    async fn test_sync_to_device_delete_orphans() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        // Create test audio file in downloads
        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        let test_file = podcast_dir.join("episode1.mp3");
        fs::write(&test_file, b"test audio content").await.unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        // Create device directory with extra file not on PC
        let device_path = temp_dir.path().join("device");
        let device_podcast_dir = device_path.join("Podcasts").join("Test Podcast");
        fs::create_dir_all(&device_podcast_dir).await.unwrap();
        let orphan_file = device_podcast_dir.join("old_episode.mp3");
        fs::write(&orphan_file, b"old content").await.unwrap();

        // Run sync with delete_orphans=true
        let report = manager
            .sync_to_device(device_path.clone(), None, true, false)
            .await
            .unwrap();

        // Verify orphan was deleted
        assert_eq!(report.files_copied.len(), 1); // episode1.mp3 copied
        assert_eq!(report.files_deleted.len(), 1); // old_episode.mp3 deleted
        assert_eq!(report.errors.len(), 0);
        assert!(report.is_success());
        assert!(!orphan_file.exists());
    }

    #[tokio::test]
    async fn test_sync_to_device_update_changed_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        // Create test audio file in downloads
        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        let test_file = podcast_dir.join("episode1.mp3");
        let new_content = b"updated audio content with more data";
        fs::write(&test_file, new_content).await.unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        // Create device directory with old version of file (different size)
        let device_path = temp_dir.path().join("device");
        let device_podcast_dir = device_path.join("Podcasts").join("Test Podcast");
        fs::create_dir_all(&device_podcast_dir).await.unwrap();
        let device_file = device_podcast_dir.join("episode1.mp3");
        fs::write(&device_file, b"old content").await.unwrap();

        // Run sync
        let report = manager
            .sync_to_device(device_path.clone(), None, false, false)
            .await
            .unwrap();

        // Verify file was updated
        assert_eq!(report.files_copied.len(), 1);
        assert_eq!(report.files_skipped.len(), 0);
        assert_eq!(report.errors.len(), 0);
        assert!(report.is_success());

        // Verify content was updated
        let device_content = fs::read(&device_file).await.unwrap();
        assert_eq!(device_content, new_content);
    }

    #[tokio::test]
    async fn test_sync_with_playlists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let playlists_dir = temp_dir.path().join("Playlists");
        fs::create_dir_all(&downloads_dir).await.unwrap();
        fs::create_dir_all(&playlists_dir).await.unwrap();

        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        fs::write(podcast_dir.join("episode1.mp3"), b"download")
            .await
            .unwrap();

        let playlist_audio_dir = playlists_dir.join("Morning Commute").join("audio");
        fs::create_dir_all(&playlist_audio_dir).await.unwrap();
        fs::write(playlist_audio_dir.join("001-episode.mp3"), b"playlist")
            .await
            .unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();
        let device_path = temp_dir.path().join("device");
        fs::create_dir_all(&device_path).await.unwrap();

        let report = manager
            .sync_to_device(device_path.clone(), Some(playlists_dir), false, false)
            .await
            .unwrap();

        assert_eq!(report.files_copied.len(), 2);
        assert!(device_path
            .join("Podcasts")
            .join("Test Podcast")
            .join("episode1.mp3")
            .exists());
        assert!(device_path
            .join("Playlists")
            .join("Morning Commute")
            .join("001-episode.mp3")
            .exists());
    }

    #[tokio::test]
    async fn test_sync_skips_empty_playlists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let playlists_dir = temp_dir.path().join("Playlists");
        fs::create_dir_all(&downloads_dir).await.unwrap();
        fs::create_dir_all(playlists_dir.join("Empty Playlist"))
            .await
            .unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();
        let device_path = temp_dir.path().join("device");
        fs::create_dir_all(&device_path).await.unwrap();

        let report = manager
            .sync_to_device(device_path.clone(), Some(playlists_dir), false, false)
            .await
            .unwrap();

        assert_eq!(report.files_copied.len(), 0);
        assert!(!device_path
            .join("Playlists")
            .join("Empty Playlist")
            .exists());
    }

    #[tokio::test]
    async fn test_sync_playlists_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let playlists_dir = temp_dir.path().join("Playlists");
        fs::create_dir_all(&downloads_dir).await.unwrap();
        fs::create_dir_all(&playlists_dir).await.unwrap();

        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        fs::write(podcast_dir.join("episode1.mp3"), b"download")
            .await
            .unwrap();

        let playlist_audio_dir = playlists_dir.join("Morning Commute").join("audio");
        fs::create_dir_all(&playlist_audio_dir).await.unwrap();
        fs::write(playlist_audio_dir.join("001-episode.mp3"), b"playlist")
            .await
            .unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();
        let device_path = temp_dir.path().join("device");
        fs::create_dir_all(&device_path).await.unwrap();

        let report = manager
            .sync_to_device(device_path.clone(), None, false, false)
            .await
            .unwrap();

        assert_eq!(report.files_copied.len(), 1);
        assert!(device_path.join("Podcasts").join("Test Podcast").exists());
        assert!(!device_path.join("Playlists").exists());
    }

    #[tokio::test]
    async fn test_sync_orphan_deletion_with_playlists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let playlists_dir = temp_dir.path().join("Playlists");
        fs::create_dir_all(&downloads_dir).await.unwrap();
        fs::create_dir_all(&playlists_dir).await.unwrap();

        let podcast_dir = downloads_dir.join("Test Podcast");
        fs::create_dir_all(&podcast_dir).await.unwrap();
        fs::write(podcast_dir.join("episode1.mp3"), b"download")
            .await
            .unwrap();

        let playlist_audio_dir = playlists_dir.join("Morning Commute").join("audio");
        fs::create_dir_all(&playlist_audio_dir).await.unwrap();
        fs::write(playlist_audio_dir.join("001-episode.mp3"), b"playlist")
            .await
            .unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();
        let device_path = temp_dir.path().join("device");
        let orphan_playlist_dir = device_path.join("Playlists").join("Old Playlist");
        fs::create_dir_all(&orphan_playlist_dir).await.unwrap();
        let orphan_file = orphan_playlist_dir.join("001-old.mp3");
        fs::write(&orphan_file, b"orphan").await.unwrap();

        let report = manager
            .sync_to_device(device_path.clone(), Some(playlists_dir), true, false)
            .await
            .unwrap();

        assert_eq!(report.files_deleted.len(), 1);
        assert!(!orphan_file.exists());
        assert!(device_path
            .join("Playlists")
            .join("Morning Commute")
            .join("001-episode.mp3")
            .exists());
    }

    // -----------------------------------------------------------------------
    // Tests for cleanup_old_downloads / cleanup_old_downloads_hours
    // -----------------------------------------------------------------------

    /// Helper: set a file's modification time to `age` in the past.
    fn set_file_mtime_age(path: &std::path::Path, age: std::time::Duration) {
        use std::fs::FileTimes;
        let past = std::time::SystemTime::now() - age;
        let times = FileTimes::new().set_modified(past);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(times)
            .unwrap();
    }

    /// Helper: create a podcast, an episode marked as Downloaded with a real
    /// file on disk, and save both to storage. Returns (podcast_id, episode).
    async fn setup_downloaded_episode(
        storage: &Arc<JsonStorage>,
        downloads_dir: &std::path::Path,
        podcast_title: &str,
        episode_title: &str,
        filename: &str,
    ) -> (PodcastId, Episode) {
        use crate::podcast::Podcast;

        let podcast = Podcast::new(
            podcast_title.to_string(),
            "https://example.com/feed".to_string(),
        );
        let podcast_id = podcast.id.clone();
        storage.save_podcast(&podcast).await.unwrap();

        let episode_file = downloads_dir.join(podcast_title).join(filename);
        fs::create_dir_all(episode_file.parent().unwrap())
            .await
            .unwrap();
        fs::write(&episode_file, b"fake audio data").await.unwrap();

        let mut episode = Episode::new(
            podcast_id.clone(),
            episode_title.to_string(),
            format!("https://example.com/{}", filename),
            Utc::now(),
        );
        episode.status = EpisodeStatus::Downloaded;
        episode.local_path = Some(episode_file);
        storage.save_episode(&podcast_id, &episode).await.unwrap();

        (podcast_id, episode)
    }

    #[tokio::test]
    async fn test_cleanup_deletes_old_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager = DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            DownloadConfig::default(),
        )
        .unwrap();

        // Create two episodes under the same podcast
        let (podcast_id, old_ep) =
            setup_downloaded_episode(&storage, &downloads_dir, "TestPod", "Old Ep", "old.mp3")
                .await;

        // Create second episode reusing the same podcast_id
        let new_file = downloads_dir.join("TestPod").join("new.mp3");
        fs::write(&new_file, b"new audio").await.unwrap();
        let mut new_ep = Episode::new(
            podcast_id.clone(),
            "New Ep".to_string(),
            "https://example.com/new.mp3".to_string(),
            Utc::now(),
        );
        new_ep.status = EpisodeStatus::Downloaded;
        new_ep.local_path = Some(new_file.clone());
        storage.save_episode(&podcast_id, &new_ep).await.unwrap();

        // Make old episode file 10 days old; new episode stays fresh
        let ten_days = std::time::Duration::from_secs(10 * 24 * 3600);
        set_file_mtime_age(old_ep.local_path.as_ref().unwrap(), ten_days);

        // Cleanup with 7-day cutoff (168 hours)
        let result = manager.cleanup_old_downloads_hours(7 * 24).await;
        assert!(result.is_ok(), "cleanup should succeed: {:?}", result);
        assert_eq!(result.unwrap(), 1, "should have deleted exactly 1 file");

        // Old file should be gone
        assert!(!old_ep.local_path.as_ref().unwrap().exists());

        // New file should still exist
        assert!(new_file.exists());

        // Verify episode statuses persisted
        let episodes = storage.load_episodes(&podcast_id).await.unwrap();
        let old_saved = episodes.iter().find(|e| e.id == old_ep.id).unwrap();
        assert_eq!(old_saved.status, EpisodeStatus::New);
        assert!(old_saved.local_path.is_none());

        let new_saved = episodes.iter().find(|e| e.id == new_ep.id).unwrap();
        assert_eq!(new_saved.status, EpisodeStatus::Downloaded);
        assert!(new_saved.local_path.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_preserves_new_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager = DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            DownloadConfig::default(),
        )
        .unwrap();

        let (podcast_id, episode) = setup_downloaded_episode(
            &storage,
            &downloads_dir,
            "FreshPod",
            "Fresh Ep",
            "fresh.mp3",
        )
        .await;

        // File was just created — well within a 24-hour cutoff
        let result = manager.cleanup_old_downloads_hours(24).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0, "no files should be deleted");

        // File still exists, status unchanged
        assert!(episode.local_path.as_ref().unwrap().exists());
        let episodes = storage.load_episodes(&podcast_id).await.unwrap();
        let saved = episodes.iter().find(|e| e.id == episode.id).unwrap();
        assert_eq!(saved.status, EpisodeStatus::Downloaded);
        assert!(saved.local_path.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_resets_stale_status() {
        use crate::podcast::Podcast;

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager = DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            DownloadConfig::default(),
        )
        .unwrap();

        // Create podcast and episode that claims to be Downloaded,
        // but the file does NOT exist on disk.
        let podcast = Podcast::new(
            "StalePod".to_string(),
            "https://example.com/feed".to_string(),
        );
        let podcast_id = podcast.id.clone();
        storage.save_podcast(&podcast).await.unwrap();

        let phantom_path = downloads_dir.join("StalePod").join("ghost.mp3");
        let mut episode = Episode::new(
            podcast_id.clone(),
            "Ghost Episode".to_string(),
            "https://example.com/ghost.mp3".to_string(),
            Utc::now(),
        );
        episode.status = EpisodeStatus::Downloaded;
        episode.local_path = Some(phantom_path.clone());
        storage.save_episode(&podcast_id, &episode).await.unwrap();

        // Sanity: file truly doesn't exist
        assert!(!phantom_path.exists());

        let result = manager.cleanup_old_downloads_hours(24).await;
        assert!(result.is_ok());

        // Episode status should be reset to New
        let episodes = storage.load_episodes(&podcast_id).await.unwrap();
        let saved = episodes.iter().find(|e| e.id == episode.id).unwrap();
        assert_eq!(saved.status, EpisodeStatus::New);
        assert!(saved.local_path.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_skips_non_downloaded_episodes() {
        use crate::podcast::Podcast;

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager = DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            DownloadConfig::default(),
        )
        .unwrap();

        let podcast = Podcast::new(
            "SkipPod".to_string(),
            "https://example.com/feed".to_string(),
        );
        let podcast_id = podcast.id.clone();
        storage.save_podcast(&podcast).await.unwrap();

        // Create episodes with non-Downloaded statuses
        let statuses = [
            EpisodeStatus::New,
            EpisodeStatus::Downloading,
            EpisodeStatus::DownloadFailed,
        ];

        let mut episode_ids = Vec::new();
        for (i, status) in statuses.iter().enumerate() {
            let mut ep = Episode::new(
                podcast_id.clone(),
                format!("Episode {}", i),
                format!("https://example.com/ep{}.mp3", i),
                Utc::now(),
            );
            ep.status = status.clone();
            episode_ids.push((ep.id.clone(), status.clone()));
            storage.save_episode(&podcast_id, &ep).await.unwrap();
        }

        let result = manager.cleanup_old_downloads_hours(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify all episodes are untouched (match by ID, not order)
        let episodes = storage.load_episodes(&podcast_id).await.unwrap();
        for (id, expected_status) in &episode_ids {
            let ep = episodes.iter().find(|e| &e.id == id).unwrap();
            assert_eq!(&ep.status, expected_status);
        }
    }

    #[tokio::test]
    async fn test_cleanup_rejects_zero_hours() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        let result = manager.cleanup_old_downloads_hours(0).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("must be greater than 0"),
            "expected 'must be greater than 0' but got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_cleanup_rejects_overflow() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        let result = manager.cleanup_old_downloads_hours(u64::MAX).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("too large"),
            "expected 'too large' but got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_cleanup_empty_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager =
            DownloadManager::new(storage, downloads_dir, DownloadConfig::default()).unwrap();

        let result = manager.cleanup_old_downloads_hours(24).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_removes_empty_directories() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager = DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            DownloadConfig::default(),
        )
        .unwrap();

        // Create a single episode — the only file in its podcast directory
        let (podcast_id, episode) =
            setup_downloaded_episode(&storage, &downloads_dir, "LonelyPod", "Solo Ep", "solo.mp3")
                .await;

        // Make the file 10 days old
        let ten_days = std::time::Duration::from_secs(10 * 24 * 3600);
        set_file_mtime_age(episode.local_path.as_ref().unwrap(), ten_days);

        let podcast_dir = downloads_dir.join("LonelyPod");
        assert!(
            podcast_dir.exists(),
            "podcast dir should exist before cleanup"
        );

        let result = manager.cleanup_old_downloads_hours(7 * 24).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // The podcast subdirectory should have been cleaned up
        assert!(
            !podcast_dir.exists(),
            "empty podcast directory should be removed after cleanup"
        );

        // Verify episode status
        let episodes = storage.load_episodes(&podcast_id).await.unwrap();
        let saved = episodes.iter().find(|e| e.id == episode.id).unwrap();
        assert_eq!(saved.status, EpisodeStatus::New);
        assert!(saved.local_path.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_old_downloads_wrapper() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let downloads_dir = temp_dir.path().join("downloads");
        fs::create_dir_all(&downloads_dir).await.unwrap();

        let manager = DownloadManager::new(
            storage.clone(),
            downloads_dir.clone(),
            DownloadConfig::default(),
        )
        .unwrap();

        // Create an episode with mtime 10 days ago
        let (_podcast_id, episode) =
            setup_downloaded_episode(&storage, &downloads_dir, "WrapperPod", "Old Ep", "old.mp3")
                .await;

        let ten_days = std::time::Duration::from_secs(10 * 24 * 3600);
        set_file_mtime_age(episode.local_path.as_ref().unwrap(), ten_days);

        // Use the days-based wrapper: 7 days = 168 hours
        let result = manager.cleanup_old_downloads(7).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            1,
            "wrapper should correctly convert days to hours"
        );

        // Confirm file is deleted
        assert!(!episode.local_path.as_ref().unwrap().exists());
    }
}
