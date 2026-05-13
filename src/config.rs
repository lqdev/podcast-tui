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
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub scrobbling: ScrobblingConfig,
    /// Named device profiles for sync filename customization.
    ///
    /// Empty by default — when no profiles are configured, sync uses the
    /// existing local filename. The filename template engine and its full
    /// token reference land in #208; user-facing documentation lands in
    /// #211. Until then, `filename_template` strings configured here are
    /// inert (recognized by the schema but not yet applied at sync time).
    #[serde(default)]
    pub device_profiles: Vec<DeviceProfile>,
    /// Name of the currently active device profile, if any.
    ///
    /// Must match a `DeviceProfile::name` in `device_profiles`. Use
    /// [`Config::active_device_profile`] to resolve to the profile struct.
    #[serde(default)]
    pub active_device_profile: Option<String>,
}

impl Config {
    /// Resolve the config file path for a given CLI override.
    ///
    /// Returns the path that [`load_or_default`] would read from /
    /// write to. Exposed so callers (e.g. `main.rs`) can hold onto the
    /// path for later runtime persistence without re-implementing the
    /// resolution logic.
    pub fn resolve_config_path(custom_path: Option<&String>) -> Result<PathBuf> {
        match custom_path {
            Some(path) => Ok(PathBuf::from(path)),
            None => Self::default_config_path(),
        }
    }

    /// Load configuration from file or create default
    pub fn load_or_default(custom_path: Option<&String>) -> Result<Self> {
        let config_path = Self::resolve_config_path(custom_path)?;

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

    /// Resolve [`Config::active_device_profile`] (a name) to the matching
    /// [`DeviceProfile`] in [`Config::device_profiles`], if any.
    ///
    /// Returns `None` when no profile is selected, or when the selected name
    /// does not match any configured profile.
    pub fn active_device_profile(&self) -> Option<&DeviceProfile> {
        let name = self.active_device_profile.as_ref()?;
        self.device_profiles.iter().find(|p| &p.name == name)
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

/// Podcast discovery configuration (PodcastIndex.org API).
///
/// Get free API credentials at <https://api.podcastindex.org/>.
/// Leave both fields empty to disable discovery features.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// PodcastIndex API key
    pub podcastindex_api_key: String,
    /// PodcastIndex API secret
    pub podcastindex_api_secret: String,
}

/// Scrobbling configuration for ListenBrainz-compatible server.
///
/// When `enabled` is `false` (the default), a no-op scrobbler is used and no
/// network traffic is generated. Existing `config.json` files without a
/// `"scrobbling"` key will deserialize without error thanks to `#[serde(default)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrobblingConfig {
    /// Whether scrobbling is enabled
    pub enabled: bool,
    /// Server endpoint URL (e.g., "http://localhost:5000")
    pub endpoint: Option<String>,
    /// Authentication token (matches server's SCROBBLER_TOKEN env var)
    pub token: Option<String>,
    /// Logical username label for queue file naming (not sent to the server; default: "default")
    pub username: String,
    /// Minimum % of episode to listen before scrobbling (both thresholds must be met)
    pub min_listen_percent: u8,
    /// Minimum seconds to listen before scrobbling (both thresholds must be met)
    pub min_listen_seconds: u32,
    /// Whether to send playing_now events when playback starts
    pub submit_playing_now: bool,
    /// HTTP timeout in seconds (must never block playback)
    pub timeout_secs: u64,
    /// Maximum pending scrobbles in retry queue
    pub max_retry_queue_size: usize,
    /// Days to keep pending scrobbles before expiring
    pub retry_queue_ttl_days: u32,
}

impl Default for ScrobblingConfig {
    fn default() -> Self {
        use crate::constants::scrobbling;
        Self {
            enabled: false,
            endpoint: None,
            token: None,
            username: "default".to_string(),
            min_listen_percent: scrobbling::DEFAULT_MIN_LISTEN_PERCENT,
            min_listen_seconds: scrobbling::DEFAULT_MIN_LISTEN_SECONDS,
            submit_playing_now: true,
            timeout_secs: scrobbling::SCROBBLE_TIMEOUT.as_secs(),
            max_retry_queue_size: scrobbling::MAX_RETRY_QUEUE_SIZE,
            retry_queue_ttl_days: scrobbling::RETRY_QUEUE_TTL_DAYS,
        }
    }
}

/// Device-specific filename profile for sync.
///
/// Defines how files are named when copied to a target device (e.g., a budget
/// MP3 player like the Innioasis Y1 that ignores ID3 metadata and displays
/// raw filenames). Profiles are pure config — they do not affect how files are
/// stored locally, only how they are written during `sync_to_device`.
///
/// The `filename_template` field is a string with substitution tokens. The
/// template engine (and full token reference) lands in #208; this issue only
/// defines the schema. Until then, profiles configured here are inert.
///
/// # Example
///
/// ```json
/// {
///   "name": "Innioasis Y1",
///   "match_path_contains": "INNIOASIS",
///   "filename_template": "{podcast} - {episode_number:03} - {title}.{ext}",
///   "max_filename_length": 64,
///   "ascii_only": true,
///   "preserve_structure": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceProfile {
    /// Human-readable identifier for the profile. Referenced by
    /// [`Config::active_device_profile`].
    pub name: String,
    /// Optional substring to match against the sync target path for future
    /// auto-selection. Currently informational only — sync still uses
    /// `active_device_profile` to choose a profile.
    #[serde(default)]
    pub match_path_contains: Option<String>,
    /// Filename template containing literal text and substitution tokens.
    ///
    /// Validation is intentionally deferred to the template engine (#208) so
    /// the schema stays permissive: an empty or malformed template
    /// deserializes successfully but will surface a user-friendly error at
    /// sync time. This keeps `config.json` editable without requiring the
    /// user to know every token up front.
    pub filename_template: String,
    /// Maximum length (in bytes) of the rendered filename, excluding any path
    /// separators. The template engine truncates the title segment if the
    /// rendered name exceeds this limit.
    #[serde(default = "default_device_max_filename_length")]
    pub max_filename_length: usize,
    /// If true, transliterate or strip non-ASCII characters in the rendered
    /// filename. Useful for devices that cannot render Unicode.
    #[serde(default)]
    pub ascii_only: bool,
    /// If true (default), preserve the per-podcast subdirectory structure
    /// when writing to the device. If false, all files are flattened into
    /// the device root.
    #[serde(default = "default_true")]
    pub preserve_structure: bool,
}

fn default_device_max_filename_length() -> usize {
    128
}

fn default_true() -> bool {
    true
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
/// - Named keys: `Enter`, `Esc`, `Tab`, `BackTab`, `Backspace`, `Delete`, `Space`
/// - Arrow keys: `Up`, `Down`, `Left`, `Right`
/// - Navigation: `Home`, `End`, `PgUp`, `PgDn`
/// - Function keys: `F1`–`F12`
///
/// **Note on uppercase chars:** `G` and `S-G` are distinct — `G` is the literal uppercase
/// character with no modifier, while `S-G` is Shift+G. Unlike Helix, uppercase letters do
/// not imply a Shift modifier; use the `S-` prefix to explicitly require Shift.
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
    pub toggle_favorite: Vec<String>,
    pub cycle_sort_field: Vec<String>,
    pub toggle_sort_direction: Vec<String>,

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

    // ── Audio playback ────────────────────────────────────────────────────────
    pub toggle_play_pause: Vec<String>,
    pub play_episode: Vec<String>,
    pub seek_backward: Vec<String>,
    pub seek_forward: Vec<String>,
    pub volume_up: Vec<String>,
    pub volume_down: Vec<String>,
    pub open_now_playing: Vec<String>,
}

impl Default for GlobalKeys {
    /// Returns an empty `GlobalKeys` — all vecs are empty, meaning "no explicit override;
    /// use the active preset's defaults". This is the correct serde default so that a
    /// user config only needs to specify the bindings they want to change.
    fn default() -> Self {
        Self {
            move_up: vec![],
            move_down: vec![],
            move_left: vec![],
            move_right: vec![],
            page_up: vec![],
            page_down: vec![],
            move_to_top: vec![],
            move_to_bottom: vec![],
            move_episode_up: vec![],
            move_episode_down: vec![],
            next_buffer: vec![],
            prev_buffer: vec![],
            close_buffer: vec![],
            open_podcast_list: vec![],
            open_downloads: vec![],
            open_playlists: vec![],
            open_sync: vec![],
            quit: vec![],
            show_help: vec![],
            search: vec![],
            clear_filters: vec![],
            refresh: vec![],
            prompt_command: vec![],
            switch_to_buffer: vec![],
            list_buffers: vec![],
            select: vec![],
            cancel: vec![],
            add_podcast: vec![],
            delete_podcast: vec![],
            refresh_podcast: vec![],
            refresh_all: vec![],
            hard_refresh_podcast: vec![],
            download_episode: vec![],
            delete_downloaded_episode: vec![],
            delete_all_downloads: vec![],
            mark_played: vec![],
            mark_unplayed: vec![],
            toggle_favorite: vec![],
            cycle_sort_field: vec![],
            toggle_sort_direction: vec![],
            create_playlist: vec![],
            add_to_playlist: vec![],
            import_opml: vec![],
            export_opml: vec![],
            sync_to_device: vec![],
            prev_tab: vec![],
            next_tab: vec![],
            toggle_play_pause: vec![],
            play_episode: vec![],
            seek_backward: vec![],
            seek_forward: vec![],
            volume_up: vec![],
            volume_down: vec![],
            open_now_playing: vec![],
        }
    }
}

impl GlobalKeys {
    /// Returns the built-in default preset: arrow keys + Vim aliases (`hjkl`) +
    /// Emacs aliases (`C-n`/`C-p`). This matches the hard-coded bindings set up
    /// by `KeyHandler::new()`.
    pub fn default_preset() -> Self {
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
            prev_buffer: ["S-Tab", "BackTab", "S-BackTab", "C-PgUp"]
                .map(String::from)
                .to_vec(),
            close_buffer: ["C-k"].map(String::from).to_vec(),
            open_podcast_list: ["F2"].map(String::from).to_vec(),
            open_downloads: ["F4"].map(String::from).to_vec(),
            open_playlists: ["F7"].map(String::from).to_vec(),
            open_sync: ["F8"].map(String::from).to_vec(),

            // Application control
            quit: ["q", "F10"].map(String::from).to_vec(),
            show_help: ["F1", "h", "?", "S-?"].map(String::from).to_vec(),
            search: ["F3", "/"].map(String::from).to_vec(),
            clear_filters: ["F6"].map(String::from).to_vec(),
            refresh: ["F5"].map(String::from).to_vec(),
            prompt_command: [":", "S-:"].map(String::from).to_vec(),
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
            delete_downloaded_episode: ["X", "S-X"].map(String::from).to_vec(),
            delete_all_downloads: ["C-x"].map(String::from).to_vec(),
            mark_played: ["m"].map(String::from).to_vec(),
            mark_unplayed: ["u"].map(String::from).to_vec(),
            toggle_favorite: ["*", "S-*"].map(String::from).to_vec(),
            cycle_sort_field: ["o"].map(String::from).to_vec(),
            toggle_sort_direction: ["S-O"].map(String::from).to_vec(),

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

            // Audio playback — keys chosen to avoid displacing any existing default binding.
            // 'P' (S-P / Shift+P) is mnemonic for Play/Pause; lowercase 'p' is AddToPlaylist.
            // S-Enter plays the selected episode (Enter opens detail; S-Enter = play).
            toggle_play_pause: ["S-P"].map(String::from).to_vec(),
            play_episode: ["S-Enter"].map(String::from).to_vec(),
            seek_backward: ["C-Left"].map(String::from).to_vec(),
            seek_forward: ["C-Right"].map(String::from).to_vec(),
            volume_up: ["+", "="].map(String::from).to_vec(),
            volume_down: ["-"].map(String::from).to_vec(),
            open_now_playing: ["F9"].map(String::from).to_vec(),
        }
    }

    /// Returns the Vim preset: `hjkl` navigation, no Emacs `C-n`/`C-p` aliases.
    /// `h` is used for `move_left`, so it is removed from `show_help`.
    /// All other bindings are identical to the default preset.
    pub fn vim_preset() -> Self {
        Self {
            move_up: ["Up", "k"].map(String::from).to_vec(),
            move_down: ["Down", "j"].map(String::from).to_vec(),
            move_left: ["Left", "h"].map(String::from).to_vec(),
            move_right: ["Right", "l"].map(String::from).to_vec(),
            // Remove 'h' from show_help since it is used for move_left in vim
            show_help: ["F1", "?", "S-?"].map(String::from).to_vec(),
            ..Self::default_preset()
        }
    }

    /// Returns the Emacs preset: `C-n`/`C-p` navigation, no Vim `j`/`k` aliases.
    /// All other bindings are identical to the default preset.
    pub fn emacs_preset() -> Self {
        Self {
            move_up: ["Up", "C-p"].map(String::from).to_vec(),
            move_down: ["Down", "C-n"].map(String::from).to_vec(),
            ..Self::default_preset()
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
///
/// # Preset selection
///
/// Set `preset` to `"vim"` or `"emacs"` to choose a navigation style. The `global` section
/// then contains only your *overrides* on top of the preset — leave a field empty (or absent)
/// to inherit the preset's default for that action.
///
/// | Preset | Navigation |
/// |--------|-----------|
/// | `"default"` | Arrow keys + `hjkl` (Vim) + `C-n`/`C-p` (Emacs) |
/// | `"vim"` | `hjkl` + arrow keys; no `C-n`/`C-p` |
/// | `"emacs"` | `C-n`/`C-p` + arrow keys; no `j`/`k` |
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    /// Base keybinding preset: `"default"` (the empty string also means default), `"vim"`,
    /// or `"emacs"`. Unrecognised values fall back to `"default"`.
    #[serde(default)]
    pub preset: String,
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
    /// When true (default), `JsonStorage` keeps an in-memory snapshot of
    /// podcasts/episodes/playlists so repeated reads within a session do not
    /// re-scan the data directory. Set to `false` to disable and behave
    /// exactly like older versions (every read hits disk).
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
}

fn default_opml_export_directory() -> String {
    "~/Documents/podcast-exports".to_string()
}

fn default_cache_enabled() -> bool {
    true
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_directory: None, // Use system default
            backup_enabled: true,
            backup_frequency_days: 7,
            max_backups: storage::MAX_BACKUPS as u32,
            opml_export_directory: default_opml_export_directory(),
            cache_enabled: default_cache_enabled(),
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
    /// Buffer to open on application startup. Valid values:
    /// `"help"`, `"podcast-list"`, `"downloads"`, `"sync"`,
    /// `"playlist-list"`, `"whats-new"`, `"now-playing"`.
    /// Unknown values fall back to `"help"` with a warning.
    #[serde(default = "default_startup_buffer")]
    pub startup_buffer: String,
}

// Default function for serde
fn default_whats_new_episode_limit() -> usize {
    ui::DEFAULT_WHATS_NEW_LIMIT
}

fn default_startup_buffer() -> String {
    "help".to_string()
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
            startup_buffer: default_startup_buffer(),
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
        // global.quit is empty by default (no explicit override — the preset provides defaults)
        assert!(config.keybindings.global.quit.is_empty());
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
        // Arrange / Act — use default_preset() which has all the bindings
        let keys = GlobalKeys::default_preset();

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
        assert!(!keys.clear_filters.is_empty()); // F6 → ClearFilters
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
        assert!(!keys.toggle_favorite.is_empty());
        assert!(!keys.create_playlist.is_empty());
        assert!(!keys.add_to_playlist.is_empty());
        assert!(!keys.import_opml.is_empty());
        assert!(!keys.export_opml.is_empty());
        assert!(!keys.sync_to_device.is_empty());
        assert!(!keys.prev_tab.is_empty());
        assert!(!keys.next_tab.is_empty());

        // Audio playback — seek/volume/now-playing/toggle have defaults; play_episode bound to S-Enter
        assert!(!keys.toggle_play_pause.is_empty()); // S-P
        assert!(!keys.play_episode.is_empty()); // S-Enter (Shift+Enter)
        assert!(!keys.seek_backward.is_empty());
        assert!(!keys.seek_forward.is_empty());
        assert!(!keys.volume_up.is_empty());
        assert!(!keys.volume_down.is_empty());
        assert!(!keys.open_now_playing.is_empty());

        // GlobalKeys::default() returns empty vecs (no explicit override = use preset)
        let empty = GlobalKeys::default();
        assert!(empty.move_up.is_empty());
        assert!(empty.quit.is_empty());
    }

    #[test]
    fn test_keybinding_config_default_matches_keybindings() {
        // Arrange / Act — default_preset() returns the full default binding set
        let keys = GlobalKeys::default_preset();

        // Assert — verify defaults match the hardcoded bindings in keybindings.rs
        assert!(keys.move_up.contains(&"Up".to_string()));
        assert!(keys.move_up.contains(&"k".to_string()));
        assert!(keys.move_up.contains(&"C-p".to_string()));
        assert!(keys.move_down.contains(&"Down".to_string()));
        assert!(keys.move_down.contains(&"j".to_string()));
        assert!(keys.move_down.contains(&"C-n".to_string()));
        assert_eq!(keys.move_left, vec!["Left"]);
        assert_eq!(keys.move_right, vec!["Right"]);
        assert_eq!(keys.page_up, vec!["PgUp"]);
        assert_eq!(keys.page_down, vec!["PgDn"]);
        assert!(keys.move_to_top.contains(&"Home".to_string()));
        assert!(keys.move_to_top.contains(&"g".to_string()));
        assert!(keys.move_to_bottom.contains(&"End".to_string()));
        assert!(keys.move_to_bottom.contains(&"S-G".to_string()));
        assert_eq!(keys.move_episode_up, vec!["C-Up"]);
        assert_eq!(keys.move_episode_down, vec!["C-Down"]);
        assert!(keys.next_buffer.contains(&"Tab".to_string()));
        assert!(keys.next_buffer.contains(&"C-PgDn".to_string()));
        assert!(keys.prev_buffer.contains(&"S-Tab".to_string()));
        assert!(keys.prev_buffer.contains(&"BackTab".to_string()));
        assert!(keys.prev_buffer.contains(&"S-BackTab".to_string()));
        assert!(keys.prev_buffer.contains(&"C-PgUp".to_string()));
        assert_eq!(keys.close_buffer, vec!["C-k"]);
        assert_eq!(keys.open_podcast_list, vec!["F2"]);
        assert_eq!(keys.open_downloads, vec!["F4"]);
        assert_eq!(keys.open_playlists, vec!["F7"]);
        assert_eq!(keys.open_sync, vec!["F8"]);
        assert!(keys.quit.contains(&"q".to_string()));
        assert!(keys.quit.contains(&"F10".to_string()));
        assert!(keys.show_help.contains(&"F1".to_string()));
        assert!(keys.show_help.contains(&"h".to_string()));
        assert!(keys.show_help.contains(&"?".to_string()));
        assert!(keys.show_help.contains(&"S-?".to_string()));
        assert!(keys.search.contains(&"F3".to_string()));
        assert!(keys.search.contains(&"/".to_string()));
        assert_eq!(keys.clear_filters, vec!["F6"]);
        assert_eq!(keys.refresh, vec!["F5"]);
        assert!(keys.prompt_command.contains(&":".to_string()));
        assert!(keys.prompt_command.contains(&"S-:".to_string()));
        assert_eq!(keys.switch_to_buffer, vec!["C-b"]);
        assert_eq!(keys.list_buffers, vec!["C-l"]);
        assert!(keys.select.contains(&"Enter".to_string()));
        assert!(keys.select.contains(&"Space".to_string()));
        assert_eq!(keys.cancel, vec!["Esc"]);
        assert_eq!(keys.add_podcast, vec!["a"]);
        assert_eq!(keys.delete_podcast, vec!["d"]);
        assert_eq!(keys.refresh_podcast, vec!["r"]);
        assert_eq!(keys.refresh_all, vec!["S-R"]);
        assert_eq!(keys.hard_refresh_podcast, vec!["C-r"]);
        assert_eq!(keys.download_episode, vec!["S-D"]);
        assert!(keys.delete_downloaded_episode.contains(&"X".to_string()));
        assert!(keys.delete_downloaded_episode.contains(&"S-X".to_string()));
        assert_eq!(keys.delete_all_downloads, vec!["C-x"]);
        assert_eq!(keys.mark_played, vec!["m"]);
        assert_eq!(keys.mark_unplayed, vec!["u"]);
        assert_eq!(keys.toggle_favorite, vec!["*", "S-*"]);
        assert_eq!(keys.create_playlist, vec!["c"]);
        assert_eq!(keys.add_to_playlist, vec!["p"]);
        assert_eq!(keys.import_opml, vec!["S-A"]);
        assert_eq!(keys.export_opml, vec!["S-E"]);
        assert_eq!(keys.sync_to_device, vec!["s"]);
        assert_eq!(keys.prev_tab, vec!["["]);
        assert_eq!(keys.next_tab, vec!["]"]);

        // Audio playback — non-displacing defaults
        assert_eq!(keys.toggle_play_pause, vec!["S-P"]); // P (Shift+P), mnemonic for Play/Pause
        assert_eq!(keys.play_episode, vec!["S-Enter"]); // Shift+Enter; plain Enter = SelectItem
        assert_eq!(keys.seek_backward, vec!["C-Left"]);
        assert_eq!(keys.seek_forward, vec!["C-Right"]);
        assert!(keys.volume_up.contains(&"+".to_string()));
        assert!(keys.volume_up.contains(&"=".to_string()));
        assert_eq!(keys.volume_down, vec!["-"]);
        assert_eq!(keys.open_now_playing, vec!["F9"]);
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
        // Assert — unspecified fields are empty vecs (meaning "use preset default")
        assert!(config.global.move_up.is_empty());
        assert!(config.global.mark_played.is_empty());
    }

    #[test]
    fn test_keybinding_config_empty_keybindings_gets_defaults() {
        // Arrange — no keybindings section at all
        let json = r#"{}"#;

        // Act
        let config: KeybindingConfig = serde_json::from_str(json).expect("deserialize empty");

        // Assert — global section has empty vecs (override-only semantics)
        assert!(config.global.quit.is_empty());
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

    #[test]
    fn test_device_profile_defaults_for_empty_config() {
        // Arrange / Act
        let config = Config::default();

        // Assert — fresh config has no profiles configured and none active
        assert!(config.device_profiles.is_empty());
        assert!(config.active_device_profile.is_none());
        assert!(config.active_device_profile().is_none());
    }

    #[test]
    fn test_device_profile_roundtrip_serialization() {
        // Arrange — config with two device profiles
        let config = Config {
            device_profiles: vec![
                DeviceProfile {
                    name: "Innioasis Y1".to_string(),
                    match_path_contains: Some("INNIOASIS".to_string()),
                    filename_template: "{podcast} - {episode_number:03} - {title}.{ext}"
                        .to_string(),
                    max_filename_length: 64,
                    ascii_only: true,
                    preserve_structure: false,
                },
                DeviceProfile {
                    name: "Generic USB".to_string(),
                    match_path_contains: None,
                    filename_template: "{title}.{ext}".to_string(),
                    max_filename_length: 128,
                    ascii_only: false,
                    preserve_structure: true,
                },
            ],
            active_device_profile: Some("Innioasis Y1".to_string()),
            ..Config::default()
        };

        // Act
        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let restored: Config = serde_json::from_str(&json).expect("deserialize");

        // Assert
        assert_eq!(restored.device_profiles.len(), 2);
        assert_eq!(restored.device_profiles, config.device_profiles);
        assert_eq!(
            restored.active_device_profile.as_deref(),
            Some("Innioasis Y1")
        );
    }

    #[test]
    fn test_device_profile_backward_compat_no_field() {
        // Arrange — serialize a Config without device_profiles by stripping
        // those fields from the JSON. This simulates a config.json written by
        // an earlier version of podcast-tui that predates #207.
        let original = Config::default();
        let mut value = serde_json::to_value(&original).expect("to_value");
        let map = value
            .as_object_mut()
            .expect("config should serialize as object");
        map.remove("device_profiles");
        map.remove("active_device_profile");
        let json = serde_json::to_string(&value).expect("to_string");

        // Act
        let restored: Config = serde_json::from_str(&json).expect("deserialize legacy config");

        // Assert — defaults applied without error
        assert!(restored.device_profiles.is_empty());
        assert!(restored.active_device_profile.is_none());
    }

    #[test]
    fn test_device_profile_partial_fields_use_defaults() {
        // Arrange — profile JSON omits max_filename_length, ascii_only, preserve_structure
        let json = r#"{
            "name": "Minimal",
            "filename_template": "{title}.{ext}"
        }"#;

        // Act
        let profile: DeviceProfile =
            serde_json::from_str(json).expect("deserialize partial profile");

        // Assert — schema defaults apply
        assert_eq!(profile.name, "Minimal");
        assert_eq!(profile.filename_template, "{title}.{ext}");
        assert_eq!(profile.max_filename_length, 128);
        assert!(!profile.ascii_only);
        assert!(profile.preserve_structure); // default_true
        assert!(profile.match_path_contains.is_none());
    }

    #[test]
    fn test_active_device_profile_resolves_by_name() {
        // Arrange
        let mut config = Config {
            device_profiles: vec![
                DeviceProfile {
                    name: "A".to_string(),
                    match_path_contains: None,
                    filename_template: "a.{ext}".to_string(),
                    max_filename_length: 128,
                    ascii_only: false,
                    preserve_structure: true,
                },
                DeviceProfile {
                    name: "B".to_string(),
                    match_path_contains: None,
                    filename_template: "b.{ext}".to_string(),
                    max_filename_length: 128,
                    ascii_only: false,
                    preserve_structure: true,
                },
            ],
            ..Config::default()
        };

        // Act / Assert — selecting an existing profile resolves
        config.active_device_profile = Some("B".to_string());
        let active = config.active_device_profile().expect("B should resolve");
        assert_eq!(active.name, "B");
        assert_eq!(active.filename_template, "b.{ext}");

        // Act / Assert — selecting an unknown profile returns None
        config.active_device_profile = Some("does-not-exist".to_string());
        assert!(config.active_device_profile().is_none());

        // Act / Assert — None when no profile selected
        config.active_device_profile = None;
        assert!(config.active_device_profile().is_none());
    }

    #[test]
    fn test_device_profile_empty_strings_deserialize_successfully() {
        // The schema is intentionally permissive: validation lives in the
        // template engine (#208), so empty name / empty template still
        // deserialize. They will surface a user-friendly error at sync time.
        let json = r#"{"name": "", "filename_template": ""}"#;
        let profile: DeviceProfile = serde_json::from_str(json).expect("deserialize empty fields");
        assert_eq!(profile.name, "");
        assert_eq!(profile.filename_template, "");
    }
}
