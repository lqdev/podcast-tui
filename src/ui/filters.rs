// Filter types and matching logic for episode and podcast filtering
//
// This module provides the core filter infrastructure used by
// EpisodeListBuffer, WhatsNewBuffer, and PodcastListBuffer to
// implement inline filtering (narrowing views in-place).

use crate::podcast::{Episode, EpisodeStatus, Podcast};
use crate::utils::time::parse_cleanup_duration;
use chrono::{DateTime, Utc};

/// Filter criteria for episode lists.
///
/// Combines text search, status, date range, duration, and favorites filters
/// using AND logic: an episode must match all active filters.
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeFilter {
    /// Text search query (matches title, description, notes — case-insensitive)
    pub text_query: Option<String>,

    /// Filter by episode status
    pub status: Option<EpisodeStatusFilter>,

    /// Filter by date range (relative to now)
    pub date_range: Option<DateRangeFilter>,

    /// Filter by duration category
    pub duration: Option<DurationFilter>,

    /// When true, only favorited episodes are shown.
    pub favorites_only: bool,

    /// Configurable threshold: episodes shorter than this (minutes) are "short".
    /// Set from `UiConfig.filter_short_max_minutes`. Default: 15.
    pub short_max_minutes: u32,

    /// Configurable threshold: episodes longer than this (minutes) are "long".
    /// Set from `UiConfig.filter_long_min_minutes`. Default: 45.
    pub long_min_minutes: u32,
}

impl Default for EpisodeFilter {
    fn default() -> Self {
        Self {
            text_query: None,
            status: None,
            date_range: None,
            duration: None,
            favorites_only: false,
            short_max_minutes: DEFAULT_SHORT_MAX_MINUTES,
            long_min_minutes: DEFAULT_LONG_MIN_MINUTES,
        }
    }
}

impl EpisodeFilter {
    /// Set configurable duration thresholds from user config.
    ///
    /// Ensures that `short_max` is strictly less than `long_min` so that the
    /// "medium" duration band remains a logical, non-empty range. If the
    /// provided values are invalid, the thresholds are left unchanged.
    pub fn set_duration_thresholds(&mut self, short_max: u32, long_min: u32) {
        if short_max >= long_min {
            // Invalid configuration; keep existing thresholds.
            return;
        }
        self.short_max_minutes = short_max;
        self.long_min_minutes = long_min;
    }

    /// Check if any filter is active.
    pub fn is_active(&self) -> bool {
        self.text_query.is_some()
            || self.status.is_some()
            || self.date_range.is_some()
            || self.duration.is_some()
            || self.favorites_only
    }

    /// Check if an episode matches all active filters (AND logic).
    pub fn matches(&self, episode: &Episode) -> bool {
        self.matches_text(episode)
            && self.matches_status(episode)
            && self.matches_date_range(episode)
            && self.matches_duration(episode)
            && self.matches_favorites(episode)
    }

    /// Clear all filters.
    pub fn clear(&mut self) {
        self.text_query = None;
        self.status = None;
        self.date_range = None;
        self.duration = None;
        self.favorites_only = false;
    }

    /// Build a human-readable description of active filters for UI display.
    ///
    /// # Examples
    /// - `search: "rust"`
    /// - `status: downloaded, 30d`
    /// - `search: "rust", status: downloaded`
    pub fn description(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref q) = self.text_query {
            parts.push(format!("search: \"{}\"", q));
        }
        if let Some(ref s) = self.status {
            parts.push(format!("status: {}", s));
        }
        if let Some(ref d) = self.date_range {
            parts.push(format!("{}", d));
        }
        if let Some(ref dur) = self.duration {
            parts.push(format!("duration: {}", dur));
        }
        if self.favorites_only {
            parts.push("favorited".to_string());
        }

        parts.join(", ")
    }

    // --- Private matching helpers ---

    fn matches_text(&self, episode: &Episode) -> bool {
        let query = match &self.text_query {
            Some(q) if !q.is_empty() => q.to_lowercase(),
            _ => return true,
        };

        let title_match = episode.title.to_lowercase().contains(&query);
        let desc_match = episode
            .description
            .as_deref()
            .is_some_and(|d| d.to_lowercase().contains(&query));
        let notes_match = episode
            .notes
            .as_deref()
            .is_some_and(|n| n.to_lowercase().contains(&query));

        title_match || desc_match || notes_match
    }

    fn matches_status(&self, episode: &Episode) -> bool {
        match &self.status {
            None => true,
            Some(filter) => filter.matches(&episode.status),
        }
    }

    fn matches_date_range(&self, episode: &Episode) -> bool {
        match &self.date_range {
            None => true,
            Some(range) => range.contains(episode.published),
        }
    }

    fn matches_duration(&self, episode: &Episode) -> bool {
        match &self.duration {
            None => true,
            Some(filter) => filter.matches_with_thresholds(
                episode.duration,
                self.short_max_minutes,
                self.long_min_minutes,
            ),
        }
    }

    fn matches_favorites(&self, episode: &Episode) -> bool {
        !self.favorites_only || episode.favorited
    }
}

/// Status filter options — maps to `EpisodeStatus`.
#[derive(Debug, Clone, PartialEq)]
pub enum EpisodeStatusFilter {
    New,
    Downloaded,
    Played,
    Downloading,
    DownloadFailed,
}

impl EpisodeStatusFilter {
    /// Check if this filter matches the given episode status.
    pub fn matches(&self, status: &EpisodeStatus) -> bool {
        matches!(
            (self, status),
            (EpisodeStatusFilter::New, EpisodeStatus::New)
                | (EpisodeStatusFilter::Downloaded, EpisodeStatus::Downloaded)
                | (EpisodeStatusFilter::Played, EpisodeStatus::Played)
                | (EpisodeStatusFilter::Downloading, EpisodeStatus::Downloading)
                | (
                    EpisodeStatusFilter::DownloadFailed,
                    EpisodeStatus::DownloadFailed
                )
        )
    }
}

impl std::fmt::Display for EpisodeStatusFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::Downloaded => write!(f, "downloaded"),
            Self::Played => write!(f, "played"),
            Self::Downloading => write!(f, "downloading"),
            Self::DownloadFailed => write!(f, "failed"),
        }
    }
}

/// Freeform relative date range filter.
///
/// Uses the same duration syntax as cleanup commands (`12h`, `7d`, `2w`, `1m`)
/// parsed via `parse_cleanup_duration()` from `utils/time.rs`.
///
/// ### Design Decision (#10)
/// Replaced the original fixed enum with this freeform struct to reuse
/// `parse_cleanup_duration()` — one duration syntax across the entire app.
#[derive(Debug, Clone, PartialEq)]
pub struct DateRangeFilter {
    /// Cutoff in hours (from `parse_cleanup_duration`)
    pub hours: u64,
    /// Original input string for display (e.g., "7d", "2w")
    pub display: String,
}

impl DateRangeFilter {
    /// Create a new date range filter from a duration string.
    ///
    /// Accepts the same formats as cleanup: `12h`, `7d`, `2w`, `1m`, `today`.
    pub fn from_input(input: &str) -> Option<Self> {
        let trimmed = input.trim().to_lowercase();

        // "today" is a convenience alias for "1d"
        let (hours, display) = if trimmed == "today" {
            (24u64, "today".to_string())
        } else {
            let h = parse_cleanup_duration(&trimmed)?;
            (h, trimmed)
        };

        Some(Self { hours, display })
    }

    /// Check if a timestamp falls within this date range.
    pub fn contains(&self, date: DateTime<Utc>) -> bool {
        let cutoff = Utc::now() - chrono::Duration::hours(self.hours as i64);
        date >= cutoff
    }
}

impl std::fmt::Display for DateRangeFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

/// Duration filter categories.
///
/// **DEFERRED from user-facing UI** — infrastructure retained for future use.
/// The `:filter-duration` command is not exposed because `FeedParser::extract_duration()`
/// is a stub (returns `None`), so episodes have no duration data to filter on.
/// See Design Decision #13 in `docs/SEARCH_AND_FILTER.md`.
///
/// ### Design Decision (#11)
/// Thresholds are configurable via `UiConfig`:
/// - `filter_short_max_minutes` (default: 15)
/// - `filter_long_min_minutes` (default: 45)
/// - "medium" = everything in between
#[derive(Debug, Clone, PartialEq)]
pub enum DurationFilter {
    /// Under `short_max` minutes (default: 15)
    Short,
    /// Between short_max and long_min thresholds
    Medium,
    /// Over `long_min` minutes (default: 45)
    Long,
}

/// Default threshold: episodes shorter than this (in minutes) are "short".
pub const DEFAULT_SHORT_MAX_MINUTES: u32 = 15;

/// Default threshold: episodes longer than this (in minutes) are "long".
pub const DEFAULT_LONG_MIN_MINUTES: u32 = 45;

impl DurationFilter {
    /// Check if an episode's duration falls within this category.
    ///
    /// Uses default thresholds (short < 15min, medium 15-45min, long > 45min).
    /// For configurable thresholds, use `matches_with_thresholds`.
    ///
    /// Episodes with no duration (`None`) do not match any duration filter.
    pub fn matches(&self, duration: Option<u32>) -> bool {
        self.matches_with_thresholds(
            duration,
            DEFAULT_SHORT_MAX_MINUTES,
            DEFAULT_LONG_MIN_MINUTES,
        )
    }

    /// Check if an episode's duration falls within this category
    /// using the given thresholds.
    pub fn matches_with_thresholds(
        &self,
        duration: Option<u32>,
        short_max: u32,
        long_min: u32,
    ) -> bool {
        match duration {
            None => false,
            Some(secs) => {
                let minutes = secs / 60;
                match self {
                    Self::Short => minutes < short_max,
                    Self::Medium => (short_max..=long_min).contains(&minutes),
                    Self::Long => minutes > long_min,
                }
            }
        }
    }
}

impl std::fmt::Display for DurationFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Short => write!(f, "short"),
            Self::Medium => write!(f, "medium"),
            Self::Long => write!(f, "long"),
        }
    }
}

/// Filter for the podcast list (text search only).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PodcastFilter {
    /// Text search query (matches title, author, description — case-insensitive)
    pub text_query: Option<String>,
}

impl PodcastFilter {
    /// Check if the filter is active.
    pub fn is_active(&self) -> bool {
        self.text_query.is_some()
    }

    /// Check if a podcast matches this filter.
    pub fn matches(&self, podcast: &Podcast) -> bool {
        let query = match &self.text_query {
            Some(q) if !q.is_empty() => q.to_lowercase(),
            _ => return true,
        };

        let title_match = podcast.title.to_lowercase().contains(&query);
        let author_match = podcast
            .author
            .as_deref()
            .is_some_and(|a| a.to_lowercase().contains(&query));
        let desc_match = podcast
            .description
            .as_deref()
            .is_some_and(|d| d.to_lowercase().contains(&query));

        title_match || author_match || desc_match
    }

    /// Build a human-readable description of the active filter.
    pub fn description(&self) -> String {
        match &self.text_query {
            Some(q) => format!("search: \"{}\"", q),
            None => String::new(),
        }
    }

    /// Clear the filter.
    pub fn clear(&mut self) {
        self.text_query = None;
    }
}

// --- Parsing helpers for command input ---

/// Parse a status filter string from minibuffer/command input.
///
/// Accepts: `new`, `downloaded`, `played`, `downloading`, `failed`
pub fn parse_status_filter(s: &str) -> Option<EpisodeStatusFilter> {
    match s.trim().to_lowercase().as_str() {
        "new" => Some(EpisodeStatusFilter::New),
        "downloaded" => Some(EpisodeStatusFilter::Downloaded),
        "played" => Some(EpisodeStatusFilter::Played),
        "downloading" => Some(EpisodeStatusFilter::Downloading),
        "failed" | "download-failed" => Some(EpisodeStatusFilter::DownloadFailed),
        _ => None,
    }
}

/// Parse a date range filter string from minibuffer/command input.
///
/// Uses the same duration syntax as cleanup commands: `12h`, `7d`, `2w`, `1m`.
/// Also accepts `today` as a convenience alias for `1d`.
///
/// ### Design Decision (#10)
/// Reuses `parse_cleanup_duration()` from `utils/time.rs` for consistency
/// with the `:cleanup <duration>` command.
pub fn parse_date_range(s: &str) -> Option<DateRangeFilter> {
    DateRangeFilter::from_input(s)
}

/// Parse a duration filter string from minibuffer/command input.
///
/// Accepts: `short`, `medium`, `long`
pub fn parse_duration_filter(s: &str) -> Option<DurationFilter> {
    match s.trim().to_lowercase().as_str() {
        "short" => Some(DurationFilter::Short),
        "medium" | "med" => Some(DurationFilter::Medium),
        "long" => Some(DurationFilter::Long),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::PodcastId;

    /// Helper to create a test episode with given fields.
    fn make_episode(title: &str, status: EpisodeStatus, duration: Option<u32>) -> Episode {
        let mut ep = Episode::new(
            PodcastId::new(),
            title.to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );
        ep.status = status;
        ep.duration = duration;
        ep
    }

    fn make_episode_with_date(
        title: &str,
        published: DateTime<Utc>,
        duration: Option<u32>,
    ) -> Episode {
        let mut ep = Episode::new(
            PodcastId::new(),
            title.to_string(),
            "https://example.com/audio.mp3".to_string(),
            published,
        );
        ep.duration = duration;
        ep
    }

    // --- EpisodeFilter tests ---

    #[test]
    fn test_default_filter_is_inactive() {
        let filter = EpisodeFilter::default();
        assert!(!filter.is_active());
    }

    #[test]
    fn test_inactive_filter_matches_everything() {
        let filter = EpisodeFilter::default();
        let ep = make_episode("Any Title", EpisodeStatus::New, None);
        assert!(filter.matches(&ep));
    }

    #[test]
    fn test_text_search_case_insensitive() {
        let filter = EpisodeFilter {
            text_query: Some("RUST".to_string()),
            ..Default::default()
        };
        let ep = make_episode("Learning Rust Basics", EpisodeStatus::New, None);
        assert!(filter.matches(&ep));
    }

    #[test]
    fn test_text_search_in_description() {
        let mut ep = make_episode("Episode 1", EpisodeStatus::New, None);
        ep.description = Some("This episode covers Rust programming".to_string());

        let filter = EpisodeFilter {
            text_query: Some("rust".to_string()),
            ..Default::default()
        };
        assert!(filter.matches(&ep));
    }

    #[test]
    fn test_text_search_in_notes() {
        let mut ep = make_episode("Episode 1", EpisodeStatus::New, None);
        ep.notes = Some("Great episode about Rust".to_string());

        let filter = EpisodeFilter {
            text_query: Some("rust".to_string()),
            ..Default::default()
        };
        assert!(filter.matches(&ep));
    }

    #[test]
    fn test_text_search_no_match() {
        let filter = EpisodeFilter {
            text_query: Some("xyzzy".to_string()),
            ..Default::default()
        };
        let ep = make_episode("Normal Episode", EpisodeStatus::New, None);
        assert!(!filter.matches(&ep));
    }

    #[test]
    fn test_status_filter_downloaded() {
        let filter = EpisodeFilter {
            status: Some(EpisodeStatusFilter::Downloaded),
            ..Default::default()
        };

        let downloaded = make_episode("Ep1", EpisodeStatus::Downloaded, None);
        let new_ep = make_episode("Ep2", EpisodeStatus::New, None);

        assert!(filter.matches(&downloaded));
        assert!(!filter.matches(&new_ep));
    }

    #[test]
    fn test_status_filter_new() {
        let filter = EpisodeFilter {
            status: Some(EpisodeStatusFilter::New),
            ..Default::default()
        };
        let ep = make_episode("Ep", EpisodeStatus::New, None);
        assert!(filter.matches(&ep));

        let played = make_episode("Ep", EpisodeStatus::Played, None);
        assert!(!filter.matches(&played));
    }

    #[test]
    fn test_duration_filter_short() {
        let filter = EpisodeFilter {
            duration: Some(DurationFilter::Short),
            ..Default::default()
        };
        // 10 minutes = 600 seconds → short
        assert!(filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(600))));
        // 20 minutes → not short
        assert!(!filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(1200))));
        // No duration → doesn't match
        assert!(!filter.matches(&make_episode("Ep", EpisodeStatus::New, None)));
    }

    #[test]
    fn test_duration_filter_medium() {
        let filter = EpisodeFilter {
            duration: Some(DurationFilter::Medium),
            ..Default::default()
        };
        // 30 minutes = 1800s → medium
        assert!(filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(1800))));
        // 10 minutes → not medium
        assert!(!filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(600))));
        // 60 minutes → not medium
        assert!(!filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(3600))));
    }

    #[test]
    fn test_duration_filter_long() {
        let filter = EpisodeFilter {
            duration: Some(DurationFilter::Long),
            ..Default::default()
        };
        // 60 minutes → long
        assert!(filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(3600))));
        // 30 minutes → not long
        assert!(!filter.matches(&make_episode("Ep", EpisodeStatus::New, Some(1800))));
    }

    #[test]
    fn test_date_range_filter_today() {
        let filter = EpisodeFilter {
            date_range: DateRangeFilter::from_input("today"),
            ..Default::default()
        };

        // Episode published now → matches today
        let recent = make_episode_with_date("Ep", Utc::now(), None);
        assert!(filter.matches(&recent));

        // Episode published 2 days ago → doesn't match today
        let old = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(2), None);
        assert!(!filter.matches(&old));
    }

    #[test]
    fn test_date_range_filter_7d() {
        let filter = EpisodeFilter {
            date_range: DateRangeFilter::from_input("7d"),
            ..Default::default()
        };

        let recent = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(3), None);
        assert!(filter.matches(&recent));

        let old = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(10), None);
        assert!(!filter.matches(&old));
    }

    #[test]
    fn test_date_range_filter_2w() {
        let filter = EpisodeFilter {
            date_range: DateRangeFilter::from_input("2w"),
            ..Default::default()
        };

        let recent = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(10), None);
        assert!(filter.matches(&recent));

        let old = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(20), None);
        assert!(!filter.matches(&old));
    }

    #[test]
    fn test_date_range_filter_1m() {
        let filter = EpisodeFilter {
            date_range: DateRangeFilter::from_input("1m"),
            ..Default::default()
        };

        let recent = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(15), None);
        assert!(filter.matches(&recent));

        let old = make_episode_with_date("Ep", Utc::now() - chrono::Duration::days(45), None);
        assert!(!filter.matches(&old));
    }

    #[test]
    fn test_combined_filters_and_logic() {
        let filter = EpisodeFilter {
            text_query: Some("rust".to_string()),
            status: Some(EpisodeStatusFilter::Downloaded),
            ..Default::default()
        };

        // Matches text AND status
        let matching = make_episode("Learning Rust", EpisodeStatus::Downloaded, None);
        assert!(filter.matches(&matching));

        // Matches text but not status
        let text_only = make_episode("Learning Rust", EpisodeStatus::New, None);
        assert!(!filter.matches(&text_only));

        // Matches status but not text
        let status_only = make_episode("Learning Go", EpisodeStatus::Downloaded, None);
        assert!(!filter.matches(&status_only));
    }

    #[test]
    fn test_filter_description() {
        let filter = EpisodeFilter {
            text_query: Some("rust".to_string()),
            status: Some(EpisodeStatusFilter::Downloaded),
            ..Default::default()
        };
        assert_eq!(filter.description(), "search: \"rust\", status: downloaded");
    }

    #[test]
    fn test_filter_description_empty() {
        let filter = EpisodeFilter::default();
        assert_eq!(filter.description(), "");
    }

    #[test]
    fn test_filter_clear() {
        let mut filter = EpisodeFilter {
            text_query: Some("rust".to_string()),
            status: Some(EpisodeStatusFilter::Downloaded),
            date_range: DateRangeFilter::from_input("7d"),
            duration: Some(DurationFilter::Short),
            ..Default::default()
        };
        filter.clear();
        assert!(!filter.is_active());
        assert_eq!(filter, EpisodeFilter::default());
    }

    // --- Parse function tests ---

    #[test]
    fn test_parse_status_filter() {
        assert_eq!(
            parse_status_filter("downloaded"),
            Some(EpisodeStatusFilter::Downloaded)
        );
        assert_eq!(parse_status_filter("NEW"), Some(EpisodeStatusFilter::New));
        assert_eq!(
            parse_status_filter("played"),
            Some(EpisodeStatusFilter::Played)
        );
        assert_eq!(
            parse_status_filter("downloading"),
            Some(EpisodeStatusFilter::Downloading)
        );
        assert_eq!(
            parse_status_filter("failed"),
            Some(EpisodeStatusFilter::DownloadFailed)
        );
        assert_eq!(parse_status_filter("invalid"), None);
    }

    #[test]
    fn test_parse_date_range() {
        // "today" alias → 24 hours
        let today = parse_date_range("today").unwrap();
        assert_eq!(today.hours, 24);
        assert_eq!(today.display, "today");

        // Standard duration syntax (same as cleanup commands)
        let d7 = parse_date_range("7d").unwrap();
        assert_eq!(d7.hours, 168); // 7 * 24

        let w2 = parse_date_range("2w").unwrap();
        assert_eq!(w2.hours, 336); // 14 * 24

        let m1 = parse_date_range("1m").unwrap();
        assert_eq!(m1.hours, 720); // 30 * 24

        let h12 = parse_date_range("12h").unwrap();
        assert_eq!(h12.hours, 12);

        // Invalid
        assert!(parse_date_range("invalid").is_none());
        assert!(parse_date_range("0d").is_none());
        assert!(parse_date_range("").is_none());
    }

    #[test]
    fn test_parse_duration_filter() {
        assert_eq!(parse_duration_filter("short"), Some(DurationFilter::Short));
        assert_eq!(
            parse_duration_filter("medium"),
            Some(DurationFilter::Medium)
        );
        assert_eq!(parse_duration_filter("long"), Some(DurationFilter::Long));
        assert_eq!(parse_duration_filter("med"), Some(DurationFilter::Medium));
        assert_eq!(parse_duration_filter("invalid"), None);
    }

    // --- PodcastFilter tests ---

    #[test]
    fn test_podcast_filter_inactive_matches_all() {
        let filter = PodcastFilter::default();
        let podcast = Podcast::new("Any Podcast".to_string(), "http://example.com".to_string());
        assert!(filter.matches(&podcast));
    }

    #[test]
    fn test_podcast_filter_title_match() {
        let filter = PodcastFilter {
            text_query: Some("rust".to_string()),
        };
        let podcast = Podcast::new(
            "Rustacean Station".to_string(),
            "http://example.com".to_string(),
        );
        assert!(filter.matches(&podcast));
    }

    #[test]
    fn test_podcast_filter_author_match() {
        let filter = PodcastFilter {
            text_query: Some("chris".to_string()),
        };
        let mut podcast =
            Podcast::new("Some Podcast".to_string(), "http://example.com".to_string());
        podcast.author = Some("Chris Krycho".to_string());
        assert!(filter.matches(&podcast));
    }

    #[test]
    fn test_podcast_filter_no_match() {
        let filter = PodcastFilter {
            text_query: Some("xyzzy".to_string()),
        };
        let podcast = Podcast::new(
            "Normal Podcast".to_string(),
            "http://example.com".to_string(),
        );
        assert!(!filter.matches(&podcast));
    }

    #[test]
    fn test_podcast_filter_description() {
        let filter = PodcastFilter {
            text_query: Some("rust".to_string()),
        };
        assert_eq!(filter.description(), "search: \"rust\"");
    }

    #[test]
    fn test_podcast_filter_clear() {
        let mut filter = PodcastFilter {
            text_query: Some("rust".to_string()),
        };
        filter.clear();
        assert!(!filter.is_active());
    }

    // --- EpisodeStatusFilter matching tests ---

    #[test]
    fn test_episode_status_filter_all_variants() {
        assert!(EpisodeStatusFilter::New.matches(&EpisodeStatus::New));
        assert!(!EpisodeStatusFilter::New.matches(&EpisodeStatus::Downloaded));

        assert!(EpisodeStatusFilter::Downloaded.matches(&EpisodeStatus::Downloaded));
        assert!(!EpisodeStatusFilter::Downloaded.matches(&EpisodeStatus::New));

        assert!(EpisodeStatusFilter::Played.matches(&EpisodeStatus::Played));
        assert!(!EpisodeStatusFilter::Played.matches(&EpisodeStatus::New));

        assert!(EpisodeStatusFilter::Downloading.matches(&EpisodeStatus::Downloading));
        assert!(!EpisodeStatusFilter::Downloading.matches(&EpisodeStatus::New));

        assert!(EpisodeStatusFilter::DownloadFailed.matches(&EpisodeStatus::DownloadFailed));
        assert!(!EpisodeStatusFilter::DownloadFailed.matches(&EpisodeStatus::New));
    }

    // --- Duration boundary tests ---

    #[test]
    fn test_duration_boundary_15_minutes() {
        // 14 minutes 59 seconds = 899s → short
        assert!(DurationFilter::Short.matches(Some(899)));
        // 15 minutes = 900s → medium (15 min ÷ 60 = 15, which is in 15..=45)
        assert!(DurationFilter::Medium.matches(Some(900)));
        assert!(!DurationFilter::Short.matches(Some(900)));
    }

    #[test]
    fn test_duration_boundary_45_minutes() {
        // 45 minutes = 2700s → medium (45 min is in 15..=45)
        assert!(DurationFilter::Medium.matches(Some(2700)));
        assert!(!DurationFilter::Long.matches(Some(2700)));
        // 46 minutes = 2760s → long
        assert!(DurationFilter::Long.matches(Some(2760)));
        assert!(!DurationFilter::Medium.matches(Some(2760)));
    }

    // --- Duration threshold validation tests ---

    #[test]
    fn test_set_duration_thresholds_valid() {
        let mut filter = EpisodeFilter::default();
        filter.set_duration_thresholds(10, 50);
        assert_eq!(filter.short_max_minutes, 10);
        assert_eq!(filter.long_min_minutes, 50);
    }

    #[test]
    fn test_set_duration_thresholds_invalid_equal() {
        let mut filter = EpisodeFilter::default();
        filter.set_duration_thresholds(30, 30);
        // Should keep defaults when short_max == long_min
        assert_eq!(filter.short_max_minutes, DEFAULT_SHORT_MAX_MINUTES);
        assert_eq!(filter.long_min_minutes, DEFAULT_LONG_MIN_MINUTES);
    }

    #[test]
    fn test_set_duration_thresholds_invalid_reversed() {
        let mut filter = EpisodeFilter::default();
        filter.set_duration_thresholds(60, 10);
        // Should keep defaults when short_max > long_min
        assert_eq!(filter.short_max_minutes, DEFAULT_SHORT_MAX_MINUTES);
        assert_eq!(filter.long_min_minutes, DEFAULT_LONG_MIN_MINUTES);
    }

    // --- Favorites filter tests ---

    #[test]
    fn test_favorites_filter_inactive_matches_all() {
        let filter = EpisodeFilter::default();
        let mut ep = make_episode("Ep", EpisodeStatus::New, None);
        ep.favorited = false;
        assert!(filter.matches(&ep));

        ep.favorited = true;
        assert!(filter.matches(&ep));
    }

    #[test]
    fn test_favorites_filter_active_returns_only_favorited() {
        let filter = EpisodeFilter {
            favorites_only: true,
            ..Default::default()
        };

        let mut ep_fav = make_episode("Favorited", EpisodeStatus::New, None);
        ep_fav.favorited = true;
        assert!(filter.matches(&ep_fav));

        let ep_not_fav = make_episode("Not Favorited", EpisodeStatus::New, None);
        assert!(!filter.matches(&ep_not_fav));
    }

    #[test]
    fn test_favorites_filter_is_active() {
        let mut filter = EpisodeFilter::default();
        assert!(!filter.is_active());

        filter.favorites_only = true;
        assert!(filter.is_active());
    }

    #[test]
    fn test_favorites_filter_clear() {
        let mut filter = EpisodeFilter {
            favorites_only: true,
            ..Default::default()
        };
        filter.clear();
        assert!(!filter.favorites_only);
        assert!(!filter.is_active());
    }

    #[test]
    fn test_favorites_filter_description() {
        let filter = EpisodeFilter {
            favorites_only: true,
            ..Default::default()
        };
        assert_eq!(filter.description(), "favorited");
    }

    #[test]
    fn test_favorites_filter_combined_with_status() {
        let filter = EpisodeFilter {
            status: Some(EpisodeStatusFilter::Downloaded),
            favorites_only: true,
            ..Default::default()
        };

        let mut ep = make_episode("Ep", EpisodeStatus::Downloaded, None);
        ep.favorited = true;
        assert!(filter.matches(&ep)); // downloaded AND favorited

        ep.favorited = false;
        assert!(!filter.matches(&ep)); // downloaded but NOT favorited

        let mut ep2 = make_episode("Ep2", EpisodeStatus::New, None);
        ep2.favorited = true;
        assert!(!filter.matches(&ep2)); // favorited but NOT downloaded
    }
}
