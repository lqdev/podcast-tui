use crate::storage::{EpisodeId, PodcastId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a podcast subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Podcast {
    pub id: PodcastId,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub image_url: Option<String>,
    pub language: Option<String>,
    pub categories: Vec<String>,
    pub explicit: bool,
    pub last_updated: DateTime<Utc>,
    pub episodes: Vec<EpisodeId>,
}

impl Podcast {
    /// Create a new podcast from RSS feed metadata
    pub fn new(title: String, url: String) -> Self {
        Self {
            id: PodcastId::new(),
            title,
            url,
            description: None,
            author: None,
            image_url: None,
            language: None,
            categories: Vec::new(),
            explicit: false,
            last_updated: Utc::now(),
            episodes: Vec::new(),
        }
    }

    /// Update the last updated timestamp
    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
    }

    /// Add an episode to this podcast
    pub fn add_episode(&mut self, episode_id: EpisodeId) {
        if !self.episodes.contains(&episode_id) {
            self.episodes.push(episode_id);
        }
    }

    /// Remove an episode from this podcast
    pub fn remove_episode(&mut self, episode_id: &EpisodeId) {
        self.episodes.retain(|id| id != episode_id);
    }
}

/// Represents a podcast episode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub id: EpisodeId,
    pub podcast_id: PodcastId,
    pub title: String,
    pub description: Option<String>,
    pub audio_url: String,
    pub published: DateTime<Utc>,
    pub duration: Option<u32>,  // Duration in seconds
    pub file_size: Option<u64>, // File size in bytes
    pub mime_type: Option<String>,
    pub guid: Option<String>, // RSS GUID for deduplication
    pub link: Option<String>, // Episode webpage link
    pub image_url: Option<String>,
    pub explicit: bool,
    pub season: Option<u32>,
    pub episode_number: Option<u32>,
    pub episode_type: Option<String>, // full, trailer, bonus
    pub status: EpisodeStatus,
    pub local_path: Option<PathBuf>,       // Path to downloaded file
    pub last_played_position: Option<u32>, // Last playback position in seconds
    pub play_count: u32,
    pub notes: Option<String>, // User-added notes
    pub chapters: Vec<Chapter>,
    pub transcript: Option<String>,
    /// Whether this episode has been starred/favorited by the user.
    /// Defaults to false for backward compatibility with existing data files.
    #[serde(default)]
    pub favorited: bool,
}

impl Episode {
    /// Create a new episode from RSS item
    pub fn new(
        podcast_id: PodcastId,
        title: String,
        audio_url: String,
        published: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EpisodeId::new(),
            podcast_id,
            title,
            description: None,
            audio_url,
            published,
            duration: None,
            file_size: None,
            mime_type: None,
            guid: None,
            link: None,
            image_url: None,
            explicit: false,
            season: None,
            episode_number: None,
            episode_type: None,
            status: EpisodeStatus::New,
            local_path: None,
            last_played_position: None,
            play_count: 0,
            notes: None,
            chapters: Vec::new(),
            transcript: None,
            favorited: false,
        }
    }

    /// Check if the episode is marked as a favorite.
    pub fn is_favorited(&self) -> bool {
        self.favorited
    }

    /// Toggle the favorite/starred state of this episode.
    pub fn toggle_favorite(&mut self) {
        self.favorited = !self.favorited;
    }

    /// Check if the episode is downloaded
    pub fn is_downloaded(&self) -> bool {
        matches!(self.status, EpisodeStatus::Downloaded)
            && self.local_path.as_ref().is_some_and(|p| p.exists())
    }

    /// Check if the episode has been played
    pub fn is_played(&self) -> bool {
        matches!(self.status, EpisodeStatus::Played)
    }

    /// Mark episode as played
    pub fn mark_played(&mut self) {
        if !matches!(self.status, EpisodeStatus::Played) {
            self.status = EpisodeStatus::Played;
            self.play_count += 1;
        }
    }

    /// Mark episode as unplayed
    pub fn mark_unplayed(&mut self) {
        if matches!(self.status, EpisodeStatus::Played) {
            self.status = EpisodeStatus::New;
            // Note: We don't reset play_count as it's historical data
        }
    }

    /// Update playback position
    pub fn update_position(&mut self, position: u32) {
        self.last_played_position = Some(position);

        // Auto-mark as played if we're near the end (95% or more)
        if let Some(duration) = self.duration {
            if position as f64 / duration as f64 >= 0.95 {
                self.mark_played();
            }
        }
    }

    /// Get formatted duration string
    pub fn formatted_duration(&self) -> String {
        match self.duration {
            Some(seconds) => {
                let hours = seconds / 3600;
                let minutes = (seconds % 3600) / 60;
                let secs = seconds % 60;

                if hours > 0 {
                    format!("{}:{:02}:{:02}", hours, minutes, secs)
                } else {
                    format!("{}:{:02}", minutes, secs)
                }
            }
            None => "Unknown".to_string(),
        }
    }

    /// Get formatted file size string
    pub fn formatted_file_size(&self) -> String {
        match self.file_size {
            Some(bytes) => {
                const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
                let mut size = bytes as f64;
                let mut unit_index = 0;

                while size >= 1024.0 && unit_index < UNITS.len() - 1 {
                    size /= 1024.0;
                    unit_index += 1;
                }

                format!("{:.1} {}", size, UNITS[unit_index])
            }
            None => "Unknown".to_string(),
        }
    }
}

/// Episode status tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeStatus {
    /// New, not yet downloaded or played
    New,
    /// Currently being downloaded
    Downloading,
    /// Downloaded but not yet played
    Downloaded,
    /// Played (either partially or completely)
    Played,
    /// Download failed
    DownloadFailed,
}

impl std::fmt::Display for EpisodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpisodeStatus::New => write!(f, "New"),
            EpisodeStatus::Downloading => write!(f, "Downloading"),
            EpisodeStatus::Downloaded => write!(f, "Downloaded"),
            EpisodeStatus::Played => write!(f, "Played"),
            EpisodeStatus::DownloadFailed => write!(f, "Failed"),
        }
    }
}

/// Episode chapter information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub start_time: u32, // Start time in seconds
    pub title: String,
    pub image_url: Option<String>,
    pub url: Option<String>, // Chapter link
}

impl Chapter {
    pub fn new(start_time: u32, title: String) -> Self {
        Self {
            start_time,
            title,
            image_url: None,
            url: None,
        }
    }

    /// Get formatted start time string
    pub fn formatted_start_time(&self) -> String {
        let hours = self.start_time / 3600;
        let minutes = (self.start_time % 3600) / 60;
        let seconds = self.start_time % 60;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{}:{:02}", minutes, seconds)
        }
    }
}

/// Simple podcast subscription information for UI lists
#[derive(Debug, Clone)]
pub struct PodcastSubscription {
    pub id: PodcastId,
    pub title: String,
    pub author: Option<String>,
    pub episode_count: usize,
    pub unplayed_count: usize,
    pub last_updated: DateTime<Utc>,
}

impl From<&Podcast> for PodcastSubscription {
    fn from(podcast: &Podcast) -> Self {
        Self {
            id: podcast.id.clone(),
            title: podcast.title.clone(),
            author: podcast.author.clone(),
            episode_count: podcast.episodes.len(),
            unplayed_count: 0, // Would need to be calculated separately
            last_updated: podcast.last_updated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podcast_creation() {
        let podcast = Podcast::new(
            "Test Podcast".to_string(),
            "https://example.com/feed.xml".to_string(),
        );

        assert_eq!(podcast.title, "Test Podcast");
        assert_eq!(podcast.url, "https://example.com/feed.xml");
        assert!(podcast.episodes.is_empty());
        assert!(!podcast.explicit);
    }

    #[test]
    fn test_episode_creation() {
        let podcast_id = PodcastId::new();
        let episode = Episode::new(
            podcast_id.clone(),
            "Test Episode".to_string(),
            "https://example.com/episode.mp3".to_string(),
            Utc::now(),
        );

        assert_eq!(episode.podcast_id, podcast_id);
        assert_eq!(episode.title, "Test Episode");
        assert_eq!(episode.status, EpisodeStatus::New);
        assert_eq!(episode.play_count, 0);
        assert!(!episode.is_played());
        assert!(!episode.is_downloaded());
    }

    #[test]
    fn test_episode_status_updates() {
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test".to_string(),
            "https://example.com/test.mp3".to_string(),
            Utc::now(),
        );

        // Mark as played
        episode.mark_played();
        assert!(episode.is_played());
        assert_eq!(episode.status, EpisodeStatus::Played);
        assert_eq!(episode.play_count, 1);

        // Mark as unplayed
        episode.mark_unplayed();
        assert!(!episode.is_played());
        assert_eq!(episode.status, EpisodeStatus::New);
        assert_eq!(episode.play_count, 1); // Play count persists
    }

    #[test]
    fn test_duration_formatting() {
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test".to_string(),
            "https://example.com/test.mp3".to_string(),
            Utc::now(),
        );

        // Test various durations
        episode.duration = Some(61); // 1:01
        assert_eq!(episode.formatted_duration(), "1:01");

        episode.duration = Some(3661); // 1:01:01
        assert_eq!(episode.formatted_duration(), "1:01:01");

        episode.duration = None;
        assert_eq!(episode.formatted_duration(), "Unknown");
    }

    #[test]
    fn test_file_size_formatting() {
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test".to_string(),
            "https://example.com/test.mp3".to_string(),
            Utc::now(),
        );

        episode.file_size = Some(1024);
        assert_eq!(episode.formatted_file_size(), "1.0 KB");

        episode.file_size = Some(1024 * 1024);
        assert_eq!(episode.formatted_file_size(), "1.0 MB");

        episode.file_size = None;
        assert_eq!(episode.formatted_file_size(), "Unknown");
    }

    #[test]
    fn test_chapter_formatting() {
        let chapter = Chapter::new(61, "Test Chapter".to_string());
        assert_eq!(chapter.formatted_start_time(), "1:01");

        let chapter = Chapter::new(3661, "Long Chapter".to_string());
        assert_eq!(chapter.formatted_start_time(), "1:01:01");
    }

    #[test]
    fn test_episode_favorited_defaults_to_false() {
        // Arrange
        let episode = Episode::new(
            PodcastId::new(),
            "Test".to_string(),
            "https://example.com/test.mp3".to_string(),
            Utc::now(),
        );

        // Assert
        assert!(!episode.favorited);
        assert!(!episode.is_favorited());
    }

    #[test]
    fn test_episode_toggle_favorite_on_off() {
        // Arrange
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test".to_string(),
            "https://example.com/test.mp3".to_string(),
            Utc::now(),
        );
        assert!(!episode.is_favorited());

        // Act: toggle on
        episode.toggle_favorite();
        // Assert
        assert!(episode.is_favorited());
        assert!(episode.favorited);

        // Act: toggle off
        episode.toggle_favorite();
        // Assert
        assert!(!episode.is_favorited());
        assert!(!episode.favorited);
    }

    #[test]
    fn test_episode_favorited_serde_roundtrip() {
        // Arrange: episode with favorited = true
        let mut episode = Episode::new(
            PodcastId::new(),
            "Starred Episode".to_string(),
            "https://example.com/ep.mp3".to_string(),
            Utc::now(),
        );
        episode.favorited = true;

        // Act: serialize and deserialize
        let json = serde_json::to_string(&episode).unwrap();
        let restored: Episode = serde_json::from_str(&json).unwrap();

        // Assert
        assert!(restored.favorited);
    }

    #[test]
    fn test_episode_favorited_defaults_on_missing_field() {
        // Arrange: JSON without the favorited field (simulates old data files)
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "podcast_id": "00000000-0000-0000-0000-000000000002",
            "title": "Old Episode",
            "description": null,
            "audio_url": "https://example.com/old.mp3",
            "published": "2024-01-01T00:00:00Z",
            "duration": null,
            "file_size": null,
            "mime_type": null,
            "guid": null,
            "link": null,
            "image_url": null,
            "explicit": false,
            "season": null,
            "episode_number": null,
            "episode_type": null,
            "status": "New",
            "local_path": null,
            "last_played_position": null,
            "play_count": 0,
            "notes": null,
            "chapters": [],
            "transcript": null
        }"#;

        // Act
        let episode: Episode = serde_json::from_str(json).unwrap();

        // Assert: missing field defaults to false (backward compatible)
        assert!(!episode.favorited);
    }
}
