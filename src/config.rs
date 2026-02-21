use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::constants::{audio, downloads, storage, ui};

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Global keybindings — apply in all buffers unless overridden by a context section.
///
/// Each field is a list of key notations (Helix-style: "C-n", "S-Tab", "F1", etc.).
/// Multiple notations can trigger the same action (e.g., `["Up", "k", "C-p"]` for move-up).
/// An empty list means the action has no binding.
///
/// Key notation reference:
/// - Single chars: `q`, `a`, `?`, `/`
/// - Modified: `C-x` (Ctrl), `S-x` (Shift), `A-x` / `M-x` (Alt), `C-S-x` (Ctrl+Shift)
/// - Named keys: `Enter`, `Esc`, `Tab`, `Backspace`, `Delete`, `Space`
/// - Arrow keys: `Up`, `Down`, `Left`, `Right`
/// - Navigation: `Home`, `End`, `PgUp`, `PgDn`
/// - Function keys: `F1`–`F12`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalKeys {
    // ── Navigation ──────────────────────────────────────────────────────────
    pub move_up: Vec<String>,
    pub move_down: Vec<String>,
    pub move_left: Vec<String>,
    pub move_right: Vec<String>,
    pub page_up: Vec<String>,
    pub page_down: Vec<String>,
    pub move_to_top: Vec<String>,
    pub move_to_bottom: Vec<String>,
    pub move_episode_up: Vec<String>,
    pub move_episode_down: Vec<String>,

    // ── Buffer navigation ────────────────────────────────────────────────────
    pub next_buffer: Vec<String>,
    pub prev_buffer: Vec<String>,
    pub close_buffer: Vec<String>,
    pub open_podcast_list: Vec<String>,
    pub open_downloads: Vec<String>,
    pub open_playlists: Vec<String>,
    pub open_sync: Vec<String>,

    // ── Application control ──────────────────────────────────────────────────
    pub quit: Vec<String>,
    pub show_help: Vec<String>,
    pub search: Vec<String>,
    pub clear_filters: Vec<String>,
    pub refresh: Vec<String>,
    pub prompt_command: Vec<String>,
    pub switch_to_buffer: Vec<String>,
    pub list_buffers: Vec<String>,

    // ── Interaction ──────────────────────────────────────────────────────────
    pub select: Vec<String>,
    pub cancel: Vec<String>,

    // ── Podcast management ───────────────────────────────────────────────────
    pub add_podcast: Vec<String>,
    pub delete_podcast: Vec<String>,
    pub refresh_podcast: Vec<String>,
    pub refresh_all: Vec<String>,
    pub hard_refresh_podcast: Vec<String>,

    // ── Episode actions ──────────────────────────────────────────────────────
    pub download_episode: Vec<String>,
    pub delete_downloaded_episode: Vec<String>,
    pub delete_all_downloads: Vec<String>,
    pub mark_played: Vec<String>,
    pub mark_unplayed: Vec<String>,

    // ── Playlist ─────────────────────────────────────────────────────────────
    pub create_playlist: Vec<String>,
    pub add_to_playlist: Vec<String>,

    // ── OPML ─────────────────────────────────────────────────────────────────
    pub import_opml: Vec<String>,
    pub export_opml: Vec<String>,

    // ── Sync ─────────────────────────────────────────────────────────────────
    pub sync_to_device: Vec<String>,

    // ── Tab navigation (e.g., sync dry-run preview tabs) ─────────────────────
    pub prev_tab: Vec<String>,
    pub next_tab: Vec<String>,
}

impl Default for GlobalKeys {
    fn default() -> Self {
        Self {
            // Navigation — arrow keys + vim aliases + Emacs aliases
            move_up: ["Up", "k", "C-p"].map(String::from).to_vec(),
            move_down: ["Down", "j", "C-n"].map(String::from).to_vec(),
            move_left: ["Left"].map(String::from).to_vec(),
            move_right: ["Right"].map(String::from).to_vec(),
            page_up: ["PgUp"].map(String::from).to_vec(),
            page_down: ["PgDn"].map(String::from).to_vec(),
            move_to_top: ["Home", "g"].map(String::from).to_vec(),
            move_to_bottom: ["End", "S-G"].map(String::from).to_vec(),
            move_episode_up: ["C-Up"].map(String::from).to_vec(),
            move_episode_down: ["C-Down"].map(String::from).to_vec(),

            // Buffer navigation
            next_buffer: ["Tab", "C-PgDn"].map(String::from).to_vec(),
            prev_buffer: ["S-Tab", "C-PgUp"].map(String::from).to_vec(),
            close_buffer: ["C-k"].map(String::from).to_vec(),
            open_podcast_list: ["F2"].map(String::from).to_vec(),
            open_downloads: ["F4"].map(String::from).to_vec(),
            open_playlists: ["F7"].map(String::from).to_vec(),
            open_sync: ["F8"].map(String::from).to_vec(),

            // Application control
            quit: ["q", "F10"].map(String::from).to_vec(),
            show_help: ["F1", "h", "?"].map(String::from).to_vec(),
            search: ["F3", "/"].map(String::from).to_vec(),
            clear_filters: ["F6"].map(String::from).to_vec(),
            refresh: ["F5"].map(String::from).to_vec(),
            prompt_command: [":"].map(String::from).to_vec(),
            switch_to_buffer: ["C-b"].map(String::from).to_vec(),
            list_buffers: ["C-l"].map(String::from).to_vec(),

            // Interaction
            select: ["Enter", "Space"].map(String::from).to_vec(),
            cancel: ["Esc"].map(String::from).to_vec(),

            // Podcast management
            add_podcast: ["a"].map(String::from).to_vec(),
            delete_podcast: ["d"].map(String::from).to_vec(),
            refresh_podcast: ["r"].map(String::from).to_vec(),
            refresh_all: ["S-R"].map(String::from).to_vec(),
            hard_refresh_podcast: ["C-r"].map(String::from).to_vec(),

            // Episode actions
            download_episode: ["S-D"].map(String::from).to_vec(),
            delete_downloaded_episode: ["X"].map(String::from).to_vec(),
            delete_all_downloads: ["C-x"].map(String::from).to_vec(),
            mark_played: ["m"].map(String::from).to_vec(),
            mark_unplayed: ["u"].map(String::from).to_vec(),

            // Playlist
            create_playlist: ["c"].map(String::from).to_vec(),
            add_to_playlist: ["p"].map(String::from).to_vec(),

            // OPML
            import_opml: ["S-A"].map(String::from).to_vec(),
            export_opml: ["S-E"].map(String::from).to_vec(),

            // Sync
            sync_to_device: ["s"].map(String::from).to_vec(),

            // Tab navigation
            prev_tab: ["["].map(String::from).to_vec(),
            next_tab: ["]"].map(String::from).to_vec(),
        }
    }
}

/// Per-context keybinding overrides for the podcast list buffer.
/// An empty `Vec<String>` for any field means "use the global default".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PodcastListKeys {
    pub add_podcast: Vec<String>,
    pub delete_podcast: Vec<String>,
    pub refresh_podcast: Vec<String>,
    pub refresh_all: Vec<String>,
    pub hard_refresh_podcast: Vec<String>,
    pub import_opml: Vec<String>,
    pub export_opml: Vec<String>,
}

/// Per-context keybinding overrides for the episode list buffer.
/// An empty `Vec<String>` for any field means "use the global default".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EpisodeListKeys {
    pub download_episode: Vec<String>,
    pub delete_downloaded_episode: Vec<String>,
    pub delete_all_downloads: Vec<String>,
    pub mark_played: Vec<String>,
    pub mark_unplayed: Vec<String>,
    pub add_to_playlist: Vec<String>,
    pub open_episode_detail: Vec<String>,
}

/// Per-context keybinding overrides for the playlist buffer.
/// An empty `Vec<String>` for any field means "use the global default".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaylistKeys {
    pub create_playlist: Vec<String>,
    pub delete_playlist: Vec<String>,
    pub add_to_playlist: Vec<String>,
}

/// Per-context keybinding overrides for the downloads buffer.
/// An empty `Vec<String>` for any field means "use the global default".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadKeys {
    pub download_episode: Vec<String>,
    pub delete_downloaded_episode: Vec<String>,
    pub delete_all_downloads: Vec<String>,
}

/// Per-context keybinding overrides for the sync buffer.
/// An empty `Vec<String>` for any field means "use the global default".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncKeys {
    pub sync_to_device: Vec<String>,
    pub prev_tab: Vec<String>,
    pub next_tab: Vec<String>,
}

/// Keybinding configuration — structured by context.
///
/// `global` covers all 60+ bindable actions with defaults matching the built-in bindings.
/// Buffer-specific sections (`podcast_list`, `episode_list`, etc.) are optional: when
/// `None` (the default), the global bindings apply. When present, the non-empty fields
/// override the corresponding global bindings for that buffer context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    pub global: GlobalKeys,
    #[serde(default)]
    pub podcast_list: Option<PodcastListKeys>,
    #[serde(default)]
    pub episode_list: Option<EpisodeListKeys>,
    #[serde(default)]
    pub playlist: Option<PlaylistKeys>,
    #[serde(default)]
    pub downloads: Option<DownloadKeys>,
    #[serde(default)]
    pub sync: Option<SyncKeys>,
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
        assert!(config.keybindings.global.quit.contains(&"q".to_string()));
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
  "keybindings": {},
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
            original_config.keybindings.global.quit,
            loaded_config.keybindings.global.quit
        );
    }

    // ── KeybindingConfig tests ───────────────────────────────────────────────

    #[test]
    fn test_keybinding_config_default_global_covers_all_actions() {
        // Arrange / Act
        let keys = GlobalKeys::default();

        // Assert — spot-check every action group has at least one binding
        assert!(!keys.move_up.is_empty());
        assert!(!keys.move_down.is_empty());
        assert!(!keys.move_left.is_empty());
        assert!(!keys.move_right.is_empty());
        assert!(!keys.page_up.is_empty());
        assert!(!keys.page_down.is_empty());
        assert!(!keys.move_to_top.is_empty());
        assert!(!keys.move_to_bottom.is_empty());
        assert!(!keys.move_episode_up.is_empty());
        assert!(!keys.move_episode_down.is_empty());
        assert!(!keys.next_buffer.is_empty());
        assert!(!keys.prev_buffer.is_empty());
        assert!(!keys.close_buffer.is_empty());
        assert!(!keys.open_podcast_list.is_empty());
        assert!(!keys.open_downloads.is_empty());
        assert!(!keys.open_playlists.is_empty());
        assert!(!keys.open_sync.is_empty());
        assert!(!keys.quit.is_empty());
        assert!(!keys.show_help.is_empty());
        assert!(!keys.search.is_empty());
        assert!(!keys.clear_filters.is_empty());
        assert!(!keys.refresh.is_empty());
        assert!(!keys.prompt_command.is_empty());
        assert!(!keys.switch_to_buffer.is_empty());
        assert!(!keys.list_buffers.is_empty());
        assert!(!keys.select.is_empty());
        assert!(!keys.cancel.is_empty());
        assert!(!keys.add_podcast.is_empty());
        assert!(!keys.delete_podcast.is_empty());
        assert!(!keys.refresh_podcast.is_empty());
        assert!(!keys.refresh_all.is_empty());
        assert!(!keys.hard_refresh_podcast.is_empty());
        assert!(!keys.download_episode.is_empty());
        assert!(!keys.delete_downloaded_episode.is_empty());
        assert!(!keys.delete_all_downloads.is_empty());
        assert!(!keys.mark_played.is_empty());
        assert!(!keys.mark_unplayed.is_empty());
        assert!(!keys.create_playlist.is_empty());
        assert!(!keys.add_to_playlist.is_empty());
        assert!(!keys.import_opml.is_empty());
        assert!(!keys.export_opml.is_empty());
        assert!(!keys.sync_to_device.is_empty());
        assert!(!keys.prev_tab.is_empty());
        assert!(!keys.next_tab.is_empty());
    }

    #[test]
    fn test_keybinding_config_default_matches_keybindings() {
        // Arrange / Act
        let keys = GlobalKeys::default();

        // Assert — verify defaults match the hardcoded bindings in keybindings.rs
        assert!(keys.move_up.contains(&"Up".to_string()));
        assert!(keys.move_up.contains(&"k".to_string()));
        assert!(keys.move_up.contains(&"C-p".to_string()));
        assert!(keys.move_down.contains(&"Down".to_string()));
        assert!(keys.move_down.contains(&"j".to_string()));
        assert!(keys.move_down.contains(&"C-n".to_string()));
        assert!(keys.move_to_top.contains(&"Home".to_string()));
        assert!(keys.move_to_top.contains(&"g".to_string()));
        assert!(keys.move_to_bottom.contains(&"End".to_string()));
        assert!(keys.move_to_bottom.contains(&"S-G".to_string()));
        assert!(keys.next_buffer.contains(&"Tab".to_string()));
        assert!(keys.prev_buffer.contains(&"S-Tab".to_string()));
        assert!(keys.quit.contains(&"q".to_string()));
        assert!(keys.quit.contains(&"F10".to_string()));
        assert!(keys.show_help.contains(&"F1".to_string()));
        assert!(keys.show_help.contains(&"h".to_string()));
        assert!(keys.show_help.contains(&"?".to_string()));
        assert!(keys.search.contains(&"F3".to_string()));
        assert!(keys.search.contains(&"/".to_string()));
        assert_eq!(keys.open_podcast_list, vec!["F2"]);
        assert_eq!(keys.open_downloads, vec!["F4"]);
        assert_eq!(keys.open_playlists, vec!["F7"]);
        assert_eq!(keys.open_sync, vec!["F8"]);
        assert_eq!(keys.add_podcast, vec!["a"]);
        assert_eq!(keys.refresh_all, vec!["S-R"]);
        assert_eq!(keys.download_episode, vec!["S-D"]);
        assert_eq!(keys.mark_played, vec!["m"]);
        assert_eq!(keys.mark_unplayed, vec!["u"]);
        assert_eq!(keys.import_opml, vec!["S-A"]);
        assert_eq!(keys.export_opml, vec!["S-E"]);
        assert_eq!(keys.sync_to_device, vec!["s"]);
    }

    #[test]
    fn test_keybinding_config_roundtrip_serialization() {
        // Arrange
        let config = KeybindingConfig::default();

        // Act — serialize then deserialize
        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let restored: KeybindingConfig = serde_json::from_str(&json).expect("deserialize");

        // Assert — roundtrip preserves all global keys
        assert_eq!(config.global.quit, restored.global.quit);
        assert_eq!(config.global.move_up, restored.global.move_up);
        assert_eq!(config.global.mark_played, restored.global.mark_played);
        assert_eq!(
            config.global.download_episode,
            restored.global.download_episode
        );
        assert_eq!(
            config.podcast_list.is_none(),
            restored.podcast_list.is_none()
        );
    }

    #[test]
    fn test_keybinding_config_partial_json_fills_in_defaults() {
        // Arrange — partial JSON: only quit is overridden
        let json = r#"{"global": {"quit": ["C-q"]}}"#;

        // Act
        let config: KeybindingConfig = serde_json::from_str(json).expect("deserialize partial");

        // Assert — overridden field is as specified
        assert_eq!(config.global.quit, vec!["C-q"]);
        // Assert — unspecified fields fill in from GlobalKeys::default()
        assert_eq!(config.global.move_up, GlobalKeys::default().move_up);
        assert_eq!(config.global.mark_played, GlobalKeys::default().mark_played);
    }

    #[test]
    fn test_keybinding_config_empty_keybindings_gets_defaults() {
        // Arrange — no keybindings section at all
        let json = r#"{}"#;

        // Act
        let config: KeybindingConfig = serde_json::from_str(json).expect("deserialize empty");

        // Assert — global section uses full defaults
        assert_eq!(config.global.quit, vec!["q", "F10"]);
        assert!(config.podcast_list.is_none());
        assert!(config.episode_list.is_none());
    }

    #[test]
    fn test_keybinding_config_buffer_sections_default_to_none() {
        // Arrange / Act
        let config = KeybindingConfig::default();

        // Assert — no buffer-specific overrides by default
        assert!(config.podcast_list.is_none());
        assert!(config.episode_list.is_none());
        assert!(config.playlist.is_none());
        assert!(config.downloads.is_none());
        assert!(config.sync.is_none());
    }

    #[test]
    fn test_keybinding_config_buffer_section_partial_override() {
        // Arrange — only episode_list section provided, with partial fields
        let json = r#"{"episode_list": {"mark_played": ["M"]}}"#;

        // Act
        let config: KeybindingConfig = serde_json::from_str(json).expect("deserialize");

        // Assert — episode_list section is present with specified override
        let ep = config.episode_list.expect("episode_list should be Some");
        assert_eq!(ep.mark_played, vec!["M"]);
        // Unspecified fields within the section are empty (= use global default)
        assert!(ep.download_episode.is_empty());
    }
}
