//! Application-wide constants
//!
//! This module centralizes magic numbers and configuration defaults used throughout
//! the application, making them easier to maintain and understand.

use std::time::Duration;

/// Network-related constants
pub mod network {
    use super::*;

    /// Default timeout for HTTP requests (feed fetching, metadata)
    pub const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

    /// Timeout for episode downloads (longer for large files)
    pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

    /// Timeout for feed refresh operations
    pub const FEED_REFRESH_TIMEOUT: Duration = Duration::from_secs(60);

    /// Maximum number of redirects to follow
    pub const MAX_REDIRECTS: usize = 10;

    /// User agent string for HTTP requests
    pub const USER_AGENT: &str = concat!("podcast-tui/", env!("CARGO_PKG_VERSION"));
}

/// File system-related constants
pub mod filesystem {
    /// Maximum filename length (cross-platform safe)
    pub const MAX_FILENAME_LENGTH: usize = 255;

    /// Maximum path length (varies by platform, this is a safe minimum)
    pub const MAX_PATH_LENGTH: usize = 4096;

    /// Default permissions for created directories (Unix only)
    #[cfg(unix)]
    pub const DEFAULT_DIR_PERMISSIONS: u32 = 0o755;

    /// Default permissions for created files (Unix only)
    #[cfg(unix)]
    pub const DEFAULT_FILE_PERMISSIONS: u32 = 0o644;
}

/// Download configuration constants
pub mod downloads {
    /// Default number of concurrent downloads
    pub const DEFAULT_CONCURRENT_DOWNLOADS: usize = 3;

    /// Maximum number of concurrent downloads allowed
    pub const MAX_CONCURRENT_DOWNLOADS: usize = 10;

    /// Minimum number of concurrent downloads (must be at least 1)
    pub const MIN_CONCURRENT_DOWNLOADS: usize = 1;

    /// Default chunk size for streaming downloads (8KB)
    pub const CHUNK_SIZE: usize = 8192;

    /// Number of retry attempts for failed downloads
    pub const MAX_DOWNLOAD_RETRIES: usize = 3;

    /// Delay between retry attempts (exponential backoff base)
    pub const RETRY_DELAY_MS: u64 = 1000;

    /// Default device path used when no sync_device_path is configured
    pub const DEFAULT_SYNC_DEVICE_PATH: &str = "/mnt/mp3player";
}

/// UI configuration constants
pub mod ui {
    use super::*;

    /// Default number of episodes to show in "What's New" buffer
    pub const DEFAULT_WHATS_NEW_LIMIT: usize = 50;

    /// Maximum number of episodes in "What's New" buffer
    pub const MAX_WHATS_NEW_LIMIT: usize = 200;

    /// Minimum number of episodes in "What's New" buffer
    pub const MIN_WHATS_NEW_LIMIT: usize = 10;

    /// Default theme name
    pub const DEFAULT_THEME: &str = "dark";

    /// Tick rate for UI event loop (milliseconds)
    pub const UI_TICK_RATE_MS: u64 = 250;

    /// Frame rate cap for rendering (milliseconds between frames)
    pub const MIN_FRAME_INTERVAL_MS: u64 = 16; // ~60 FPS

    /// Status message display duration (milliseconds)
    pub const STATUS_MESSAGE_DURATION: Duration = Duration::from_secs(3);

    /// Minibuffer history size
    pub const MINIBUFFER_HISTORY_SIZE: usize = 100;
}

/// Storage-related constants
pub mod storage {
    /// Number of days to keep old data before cleanup
    pub const DEFAULT_CLEANUP_AFTER_DAYS: usize = 30;

    /// Maximum number of days for cleanup (prevent immediate deletion)
    pub const MAX_CLEANUP_DAYS: usize = 365;

    /// Minimum number of days for cleanup
    pub const MIN_CLEANUP_DAYS: usize = 1;

    /// Temporary file suffix for atomic writes
    pub const TEMP_FILE_SUFFIX: &str = ".tmp";

    /// Backup file suffix
    pub const BACKUP_FILE_SUFFIX: &str = ".bak";

    /// Maximum number of backups to keep
    pub const MAX_BACKUPS: usize = 5;
}

/// Podcast feed constants
pub mod feed {
    use super::*;

    /// Default interval for feed refresh (hours)
    pub const DEFAULT_REFRESH_INTERVAL_HOURS: u64 = 24;

    /// Minimum refresh interval to prevent hammering servers (hours)
    pub const MIN_REFRESH_INTERVAL_HOURS: u64 = 1;

    /// Maximum age of feed cache before forcing refresh (hours)
    pub const MAX_CACHE_AGE_HOURS: u64 = 168; // 1 week

    /// Timeout for feed parsing operations
    pub const PARSE_TIMEOUT: Duration = Duration::from_secs(30);

    /// Maximum number of episodes to keep per podcast (0 = unlimited)
    pub const DEFAULT_MAX_EPISODES_PER_PODCAST: usize = 0;
}

/// Audio playback constants (for future Sprint 4)
#[allow(dead_code)]
pub mod audio {
    /// Default volume level (0.0 to 1.0)
    pub const DEFAULT_VOLUME: f32 = 0.8;

    /// Volume adjustment step
    pub const VOLUME_STEP: f32 = 0.05;

    /// Seek step forward/backward (seconds)
    pub const SEEK_STEP_SECS: u64 = 10;

    /// Long seek step (for Shift+arrow) (seconds)
    pub const LONG_SEEK_STEP_SECS: u64 = 60;

    /// Audio buffer size
    pub const AUDIO_BUFFER_SIZE: usize = 4096;

    /// Crossfade duration between tracks (milliseconds)
    pub const CROSSFADE_DURATION_MS: u64 = 1000;
}

/// OPML import/export constants
pub mod opml {
    use super::*;

    /// Maximum number of feeds to import in parallel
    pub const MAX_PARALLEL_IMPORTS: usize = 5;

    /// Timeout for individual feed import during OPML
    pub const IMPORT_TIMEOUT: Duration = Duration::from_secs(60);

    /// Maximum OPML file size (10 MB)
    pub const MAX_OPML_FILE_SIZE: usize = 10 * 1024 * 1024;

    /// Default OPML version
    pub const OPML_VERSION: &str = "2.0";
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_constants_are_valid() {
        use super::{audio, downloads, feed, network, opml, storage, ui};
        // Network constants
        assert!(network::HTTP_TIMEOUT.as_secs() > 0);
        assert!(network::DOWNLOAD_TIMEOUT > network::HTTP_TIMEOUT);
        assert!(network::MAX_REDIRECTS > 0);
        assert!(!network::USER_AGENT.is_empty());

        // Download constants
        assert!(downloads::MIN_CONCURRENT_DOWNLOADS > 0);
        assert!(downloads::DEFAULT_CONCURRENT_DOWNLOADS >= downloads::MIN_CONCURRENT_DOWNLOADS);
        assert!(downloads::DEFAULT_CONCURRENT_DOWNLOADS <= downloads::MAX_CONCURRENT_DOWNLOADS);
        assert!(downloads::CHUNK_SIZE > 0);
        assert!(downloads::MAX_DOWNLOAD_RETRIES > 0);

        // UI constants
        assert!(ui::MIN_WHATS_NEW_LIMIT > 0);
        assert!(ui::DEFAULT_WHATS_NEW_LIMIT >= ui::MIN_WHATS_NEW_LIMIT);
        assert!(ui::DEFAULT_WHATS_NEW_LIMIT <= ui::MAX_WHATS_NEW_LIMIT);
        assert!(!ui::DEFAULT_THEME.is_empty());
        assert!(ui::UI_TICK_RATE_MS > 0);
        assert!(ui::MIN_FRAME_INTERVAL_MS > 0);

        // Storage constants
        assert!(storage::MIN_CLEANUP_DAYS > 0);
        assert!(storage::DEFAULT_CLEANUP_AFTER_DAYS >= storage::MIN_CLEANUP_DAYS);
        assert!(storage::DEFAULT_CLEANUP_AFTER_DAYS <= storage::MAX_CLEANUP_DAYS);
        assert!(!storage::TEMP_FILE_SUFFIX.is_empty());
        assert!(!storage::BACKUP_FILE_SUFFIX.is_empty());

        // Feed constants
        assert!(feed::MIN_REFRESH_INTERVAL_HOURS > 0);
        assert!(feed::DEFAULT_REFRESH_INTERVAL_HOURS >= feed::MIN_REFRESH_INTERVAL_HOURS);
        assert!(feed::PARSE_TIMEOUT.as_secs() > 0);

        // Audio constants
        assert!(audio::DEFAULT_VOLUME >= 0.0 && audio::DEFAULT_VOLUME <= 1.0);
        assert!(audio::VOLUME_STEP > 0.0);
        assert!(audio::SEEK_STEP_SECS > 0);
        assert!(audio::AUDIO_BUFFER_SIZE > 0);

        // OPML constants
        assert!(opml::MAX_PARALLEL_IMPORTS > 0);
        assert!(opml::IMPORT_TIMEOUT.as_secs() > 0);
        assert!(opml::MAX_OPML_FILE_SIZE > 0);
        assert!(!opml::OPML_VERSION.is_empty());
    }

    #[test]
    fn test_filesystem_constants() {
        use super::filesystem;

        assert!(filesystem::MAX_FILENAME_LENGTH == 255); // Cross-platform standard
        assert!(filesystem::MAX_PATH_LENGTH >= 4096);

        #[cfg(unix)]
        {
            assert!(filesystem::DEFAULT_DIR_PERMISSIONS == 0o755);
            assert!(filesystem::DEFAULT_FILE_PERMISSIONS == 0o644);
        }
    }
}
