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
}
