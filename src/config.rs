use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::constants::{audio, downloads, storage, ui};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub audio: AudioConfig,
    pub downloads: DownloadConfig,
    pub keybindings: KeybindingConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    #[serde(default)]
    pub playlist: PlaylistConfig,
}

impl Config {
    /// Load configuration from file or create default
    pub fn load_or_default(custom_path: Option<&String>) -> Result<Self> {
        let config_path = match custom_path {
            Some(path) => PathBuf::from(path),
            None => Self::default_config_path()?,
        };

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let default_config = Self::default();
            default_config.save(&config_path)?;
            Ok(default_config)
        }
    }

    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Get the default configuration file path
    fn default_config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("", "", "podcast-tui")
            .ok_or_else(|| anyhow::anyhow!("Unable to determine config directory"))?;

        Ok(project_dirs.config_dir().join("config.json"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            downloads: DownloadConfig::default(),
            keybindings: KeybindingConfig::default(),
            storage: StorageConfig::default(),
            ui: UiConfig::default(),
            playlist: PlaylistConfig::default(),
        }
    }
}

/// Audio playback configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub volume: f32,
    pub seek_seconds: u32,
    pub external_player: Option<String>,
    pub auto_play_next: bool,
    pub remember_position: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: audio::DEFAULT_VOLUME,
            seek_seconds: audio::SEEK_STEP_SECS as u32,
            external_player: None,
            auto_play_next: false,
            remember_position: true,
        }
    }
}

/// Download management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub directory: String,
    pub concurrent_downloads: usize,
    pub cleanup_after_days: Option<u32>,
    pub auto_download_new: bool,
    pub max_download_size_mb: Option<u32>,

    // MP3 player compatibility options (with defaults for backward compatibility)
    #[serde(default = "default_use_readable_folders")]
    pub use_readable_folders: bool, // Use podcast titles vs UUIDs (default: true)
    #[serde(default = "default_embed_id3_metadata")]
    pub embed_id3_metadata: bool, // Add ID3 tags (default: true)
    #[serde(default = "default_assign_track_numbers")]
    pub assign_track_numbers: bool, // Auto-assign episode sequence (default: true)
    #[serde(default = "default_download_artwork")]
    pub download_artwork: bool, // Download and embed artwork (default: true)
    #[serde(default = "default_max_id3_comment_length")]
    pub max_id3_comment_length: usize, // Truncate descriptions (default: 200)
    #[serde(default = "default_include_episode_numbers")]
    pub include_episode_numbers: bool, // Add episode numbers to filenames (default: true)
    #[serde(default = "default_include_dates")]
    pub include_dates: bool, // Add dates to filenames (default: true)
    #[serde(default = "default_max_filename_length")]
    pub max_filename_length: usize, // Limit for compatibility (default: 150)

    // Device sync options (with defaults for backward compatibility)
    #[serde(default)]
    pub sync_device_path: Option<String>, // Path to sync device (can be overridden at runtime)
    #[serde(default = "default_sync_delete_orphans")]
    pub sync_delete_orphans: bool, // Delete files on device not present on PC (default: true)
    #[serde(default = "default_sync_preserve_structure")]
    pub sync_preserve_structure: bool, // Preserve podcast folder structure (default: true)
    #[serde(default = "default_sync_dry_run")]
    pub sync_dry_run: bool, // Default to dry-run mode for safety (default: false)
    #[serde(default = "default_sync_include_playlists")]
    pub sync_include_playlists: bool, // Include playlists in device sync (default: true)

    // Phase 3 sync options (with defaults for backward compatibility)
    /// If true, pressing 's' (sync) shows a dry-run preview first, requiring confirmation.
    #[serde(default)]
    pub sync_preview_before_sync: bool, // Default: false (immediate sync)
    /// If true, directory picker only shows removable/external drives.
    #[serde(default)]
    pub sync_filter_removable_only: bool, // Default: false (show all directories)
}

// Default functions for serde
fn default_use_readable_folders() -> bool {
    true
}
fn default_embed_id3_metadata() -> bool {
    true
}
fn default_assign_track_numbers() -> bool {
    true
}
fn default_download_artwork() -> bool {
    true
}
fn default_max_id3_comment_length() -> usize {
    200
}
fn default_include_episode_numbers() -> bool {
    true
}
fn default_include_dates() -> bool {
    true
}
fn default_max_filename_length() -> usize {
    150
}
fn default_sync_delete_orphans() -> bool {
    true
}
fn default_sync_preserve_structure() -> bool {
    true
}
fn default_sync_dry_run() -> bool {
    false
}
fn default_sync_include_playlists() -> bool {
    true
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            directory: "~/Downloads/Podcasts".to_string(),
            concurrent_downloads: downloads::DEFAULT_CONCURRENT_DOWNLOADS,
            cleanup_after_days: Some(storage::DEFAULT_CLEANUP_AFTER_DAYS as u32),
            auto_download_new: false,
            max_download_size_mb: Some(500), // 500MB limit

            // MP3 player optimized defaults
            use_readable_folders: true,
            embed_id3_metadata: true,
            assign_track_numbers: true,
            download_artwork: true,
            max_id3_comment_length: 200,
            include_episode_numbers: true,
            include_dates: true,
            max_filename_length: 150,

            // Device sync defaults
            sync_device_path: None,
            sync_delete_orphans: true,
            sync_preserve_structure: true,
            sync_dry_run: false,
            sync_include_playlists: true,
            sync_preview_before_sync: false,
            sync_filter_removable_only: false,
        }
    }
}

/// Playlist management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistConfig {
    /// Refresh policy for the auto-generated "Today" playlist.
    #[serde(default = "default_today_refresh_policy")]
    pub today_refresh_policy: String, // "daily", "on_launch", "manual"
    /// Auto-download episodes when adding to playlists.
    #[serde(default = "default_auto_download_on_add")]
    pub auto_download_on_add: bool,
    /// Download retries when adding episodes to playlists.
    #[serde(default = "default_playlist_download_retries")]
    pub download_retries: u32,
}

fn default_today_refresh_policy() -> String {
    "daily".to_string()
}
fn default_auto_download_on_add() -> bool {
    true
}
fn default_playlist_download_retries() -> u32 {
    3
}

impl Default for PlaylistConfig {
    fn default() -> Self {
        Self {
            today_refresh_policy: default_today_refresh_policy(),
            auto_download_on_add: default_auto_download_on_add(),
            download_retries: default_playlist_download_retries(),
        }
    }
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    pub play_pause: String,
    pub stop: String,
    pub next_episode: String,
    pub prev_episode: String,
    pub seek_forward: String,
    pub seek_backward: String,
    pub volume_up: String,
    pub volume_down: String,
    pub add_podcast: String,
    pub refresh_feeds: String,
    pub refresh_all_feeds: String,
    pub download_episode: String,
    pub delete_episode: String,
    pub toggle_played: String,
    pub add_note: String,
    pub quit: String,
    pub help: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            play_pause: "SPC".to_string(),
            stop: "s".to_string(),
            next_episode: "n".to_string(),
            prev_episode: "p".to_string(),
            seek_forward: "f".to_string(),
            seek_backward: "b".to_string(),
            volume_up: "+".to_string(),
            volume_down: "-".to_string(),
            add_podcast: "a".to_string(),
            refresh_feeds: "r".to_string(),
            refresh_all_feeds: "R".to_string(),
            download_episode: "D".to_string(),
            delete_episode: "X".to_string(),
            toggle_played: "m".to_string(),
            add_note: "N".to_string(),
            quit: "q".to_string(),
            help: "C-h ?".to_string(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_directory: Option<String>,
    pub backup_enabled: bool,
    pub backup_frequency_days: u32,
    pub max_backups: u32,
    #[serde(default = "default_opml_export_directory")]
    pub opml_export_directory: String,
}

fn default_opml_export_directory() -> String {
    "~/Documents/podcast-exports".to_string()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_directory: None, // Use system default
            backup_enabled: true,
            backup_frequency_days: 7,
            max_backups: storage::MAX_BACKUPS as u32,
            opml_export_directory: default_opml_export_directory(),
        }
    }
}

/// User interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub show_progress_bar: bool,
    pub show_episode_numbers: bool,
    pub date_format: String,
    pub time_format: String,
    pub compact_mode: bool,
    pub mouse_support: bool,

    // What's New buffer settings
    #[serde(default = "default_whats_new_episode_limit")]
    pub whats_new_episode_limit: usize,
    // NOTE: Duration filter config (filter_short_max_minutes, filter_long_min_minutes)
    // deferred until episode duration data is populated from RSS feeds.
    // See Design Decision #13 in docs/SEARCH_AND_FILTER.md.
}

// Default function for serde
fn default_whats_new_episode_limit() -> usize {
    ui::DEFAULT_WHATS_NEW_LIMIT
}

// NOTE: Duration filter default fns removed — deferred until extract_duration is implemented.
// See Design Decision #13.

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            show_progress_bar: true,
            show_episode_numbers: true,
            date_format: "%Y-%m-%d".to_string(),
            time_format: "%H:%M:%S".to_string(),
            compact_mode: false,
            mouse_support: true,
            whats_new_episode_limit: ui::DEFAULT_WHATS_NEW_LIMIT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert_eq!(config.audio.volume, audio::DEFAULT_VOLUME);
        assert_eq!(config.audio.seek_seconds, audio::SEEK_STEP_SECS as u32);
        assert_eq!(
            config.downloads.concurrent_downloads,
            downloads::DEFAULT_CONCURRENT_DOWNLOADS
        );
        assert!(config.downloads.sync_include_playlists);
        assert_eq!(config.playlist.today_refresh_policy, "daily");
        assert_eq!(config.keybindings.play_pause, "SPC");
        assert_eq!(config.ui.theme, "default");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).expect("Failed to serialize config");

        let deserialized: Config =
            serde_json::from_str(&json).expect("Failed to deserialize config");

        assert_eq!(config.audio.volume, deserialized.audio.volume);
        assert_eq!(
            config.downloads.concurrent_downloads,
            deserialized.downloads.concurrent_downloads
        );
    }

    #[test]
    fn test_config_backward_compat_playlist_defaults() {
        let legacy_json = r#"{
  "audio": {
    "volume": 0.8,
    "seek_seconds": 10,
    "external_player": null,
    "auto_play_next": false,
    "remember_position": true
  },
  "downloads": {
    "directory": "~/Downloads/Podcasts",
    "concurrent_downloads": 3,
    "cleanup_after_days": 30,
    "auto_download_new": false,
    "max_download_size_mb": 500
  },
  "keybindings": {
    "play_pause": "SPC",
    "stop": "s",
    "next_episode": "n",
    "prev_episode": "p",
    "seek_forward": "f",
    "seek_backward": "b",
    "volume_up": "+",
    "volume_down": "-",
    "add_podcast": "a",
    "refresh_feeds": "r",
    "refresh_all_feeds": "R",
    "download_episode": "D",
    "delete_episode": "X",
    "toggle_played": "m",
    "add_note": "N",
    "quit": "q",
    "help": "C-h ?"
  },
  "storage": {
    "data_directory": null,
    "backup_enabled": true,
    "backup_frequency_days": 7,
    "max_backups": 5,
    "opml_export_directory": "~/Documents/podcast-exports"
  },
  "ui": {
    "theme": "default",
    "show_progress_bar": true,
    "show_episode_numbers": true,
    "date_format": "%Y-%m-%d",
    "time_format": "%H:%M:%S",
    "compact_mode": false,
    "mouse_support": true,
    "whats_new_episode_limit": 50
  }
}"#;

        let config: Config = serde_json::from_str(legacy_json).expect("Legacy config should parse");
        assert_eq!(config.playlist.today_refresh_policy, "daily");
        assert!(config.downloads.sync_include_playlists);
        // Phase 3 fields should default to false when absent from legacy config
        assert!(!config.downloads.sync_preview_before_sync);
        assert!(!config.downloads.sync_filter_removable_only);
    }

    #[test]
    fn test_config_phase3_sync_fields_default() {
        // Arrange / Act
        let config = Config::default();

        // Assert — new fields default to false (non-breaking)
        assert!(!config.downloads.sync_preview_before_sync);
        assert!(!config.downloads.sync_filter_removable_only);
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test-config.json");

        let original_config = Config::default();
        original_config
            .save(&config_path)
            .expect("Failed to save config");

        assert!(config_path.exists());

        // Modify to test loading
        let loaded_config = {
            let content =
                std::fs::read_to_string(&config_path).expect("Failed to read config file");
            serde_json::from_str::<Config>(&content).expect("Failed to parse config")
        };

        assert_eq!(original_config.audio.volume, loaded_config.audio.volume);
        assert_eq!(
            original_config.keybindings.play_pause,
            loaded_config.keybindings.play_pause
        );
    }
}
