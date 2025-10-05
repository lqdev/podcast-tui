use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub audio: AudioConfig,
    pub downloads: DownloadConfig,
    pub keybindings: KeybindingConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
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
            volume: 0.8,
            seek_seconds: 30,
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

    // MP3 player compatibility options
    pub use_readable_folders: bool, // Use podcast titles vs UUIDs (default: true)
    pub embed_id3_metadata: bool,   // Add ID3 tags (default: true)
    pub assign_track_numbers: bool, // Auto-assign episode sequence (default: true)
    pub download_artwork: bool,     // Download and embed artwork (default: true)
    pub max_id3_comment_length: usize, // Truncate descriptions (default: 200)
    pub include_episode_numbers: bool, // Add episode numbers to filenames (default: true)
    pub include_dates: bool,        // Add dates to filenames (default: true)
    pub max_filename_length: usize, // Limit for compatibility (default: 150)
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            directory: "~/Downloads/Podcasts".to_string(),
            concurrent_downloads: 3,
            cleanup_after_days: Some(30),
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
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_directory: None, // Use system default
            backup_enabled: true,
            backup_frequency_days: 7,
            max_backups: 5,
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
}

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

        assert_eq!(config.audio.volume, 0.8);
        assert_eq!(config.audio.seek_seconds, 30);
        assert_eq!(config.downloads.concurrent_downloads, 3);
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
