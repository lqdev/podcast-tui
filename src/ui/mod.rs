// UI module - Core user interface components and framework
//
// This module implements an Emacs-style TUI interface with:
// - Buffer-based content management
// - Emacs keybindings and navigation
// - Window management and splitting
// - Command system with minibuffer

pub mod app;
pub mod buffers;
pub mod components;
pub mod events;
pub mod filters;
pub mod key_parser;
pub mod keybindings;
pub mod theme_loader;
pub mod themes;

pub use app::UIApp;
pub use events::{UIEvent, UIEventHandler};
pub use keybindings::KeyHandler;

/// Result type for UI operations
pub type UIResult<T> = Result<T, UIError>;

/// Errors that can occur in the UI system
#[derive(Debug, thiserror::Error)]
pub enum UIError {
    #[error("Rendering error: {0}")]
    Render(String),

    #[error("Buffer not found: {0}")]
    BufferNotFound(String),

    #[error("Invalid buffer operation: {0}")]
    InvalidOperation(String),

    #[error("Keybinding error: {0}")]
    Keybinding(String),

    #[error("Terminal error: {0}")]
    Terminal(#[from] std::io::Error),
}

/// UI action commands that can be executed
#[derive(Debug, Clone, PartialEq)]
pub enum UIAction {
    // Navigation actions
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    PageUp,
    PageDown,
    MoveToTop,
    MoveToBottom,
    MoveEpisodeUp,
    MoveEpisodeDown,

    // Buffer actions
    SwitchBuffer(String),
    CreateBuffer(String),
    CloseBuffer(String),
    NextBuffer,
    PreviousBuffer,

    // Window actions
    SplitHorizontal,
    SplitVertical,
    CloseWindow,
    NextWindow,
    OnlyWindow,

    // Application actions
    Quit,
    Refresh,
    ShowHelp,
    ExecuteCommand(String),
    PromptCommand,

    // Content-specific actions
    SelectItem,
    ExpandItem,
    CollapseItem,

    // Minibuffer actions
    ShowMessage(String),
    ShowError(String),
    ClearMinibuffer,
    ShowMinibuffer(String),
    HideMinibuffer,
    MinibufferInput(String),
    PromptInput(String),
    SubmitInput(String),
    TabComplete,
    CloseCurrentBuffer,

    // Podcast management actions
    AddPodcast,
    /// Delete podcast subscription
    DeletePodcast,
    /// Download current episode
    DownloadEpisode,
    /// Delete downloaded episode file
    DeleteDownloadedEpisode,
    /// Delete all downloaded episodes and clean up downloads folder
    DeleteAllDownloads,
    /// Trigger async download of specific episode
    TriggerDownload {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    },
    /// Trigger async deletion of downloaded episode
    TriggerDeleteDownload {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    },
    /// Open episode list for a podcast
    OpenEpisodeList {
        podcast_name: String,
        podcast_id: crate::storage::PodcastId,
    },
    /// Open episode detail view
    OpenEpisodeDetail {
        episode: Box<crate::podcast::Episode>,
    },
    /// Open episode detail by IDs (used by playlist entries)
    OpenEpisodeDetailById {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    },
    /// Trigger async downloads refresh
    TriggerRefreshDownloads,
    RefreshPodcast,
    RefreshAll,
    /// Hard refresh podcast (re-parse existing episodes)
    HardRefreshPodcast,

    // Playlist actions
    OpenPlaylistList,
    OpenPlaylistDetail {
        playlist_id: crate::playlist::PlaylistId,
        playlist_name: String,
    },
    CreatePlaylist,
    DeletePlaylist,
    AddToPlaylist,
    RefreshAutoPlaylists,
    RebuildPlaylistFiles {
        playlist_id: crate::playlist::PlaylistId,
    },
    TriggerCreatePlaylist {
        name: String,
        description: Option<String>,
    },
    TriggerAddToPlaylist {
        playlist_id: crate::playlist::PlaylistId,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    },
    TriggerRemoveFromPlaylist {
        playlist_id: crate::playlist::PlaylistId,
        episode_id: crate::storage::EpisodeId,
    },
    TriggerDeletePlaylist {
        playlist_id: crate::playlist::PlaylistId,
    },
    TriggerReorderPlaylist {
        playlist_id: crate::playlist::PlaylistId,
        from_idx: usize,
        to_idx: usize,
    },
    SyncPlaylist {
        playlist_id: crate::playlist::PlaylistId,
    },

    // OPML Import/Export actions
    /// Import podcasts from OPML file or URL
    ImportOpml,
    /// Export subscriptions to OPML file
    ExportOpml,
    /// Trigger async OPML import with source path
    TriggerOpmlImport {
        source: String,
    },
    /// Trigger async OPML export with output path
    TriggerOpmlExport {
        path: Option<String>,
    },

    // Device sync actions
    /// Initiate device sync
    SyncToDevice,
    /// Trigger async device sync with parameters
    TriggerDeviceSync {
        device_path: std::path::PathBuf,
        delete_orphans: bool,
        dry_run: bool,
    },
    /// Cycle to previous tab (e.g. in dry-run preview)
    PreviousTab,
    /// Cycle to next tab (e.g. in dry-run preview)
    NextTab,

    // Sort actions
    /// Cycle to the next sort field (Date → Title → Duration → DownloadStatus → Date)
    CycleSortField,
    /// Toggle sort direction (Ascending ↔ Descending)
    ToggleSortDirection,
    /// Set sort field by name via minibuffer command (date, title, duration, downloaded)
    SetSort {
        field: String,
    },
    /// Set sort direction by name via minibuffer command (asc, desc)
    SetSortDirection {
        direction: String,
    },

    // Search & filter actions
    /// Activate text search in the current buffer (opens minibuffer for input)
    Search,
    /// Apply a text search query to the active buffer
    ApplySearch {
        query: String,
    },
    /// Clear all search/filters in the active buffer
    ClearFilters,
    /// Set a specific status filter via command
    SetStatusFilter {
        status: String,
    },
    /// Set a specific date range filter via command
    SetDateRangeFilter {
        range: String,
    },
    // NOTE: DurationFilter deferred — episode duration data not yet populated
    // from RSS feeds (extract_duration is a stub). See Design Decision #13.
    // SetDurationFilter { duration: String },

    // Tag actions (podcast-level)
    /// Add a tag to the currently selected podcast (optimistic update)
    AddTag {
        tag: String,
    },
    /// Remove a tag from the currently selected podcast (optimistic update)
    RemoveTag {
        tag: String,
    },
    /// Filter the podcast list to only show podcasts with the given tag
    FilterByTag {
        tag: String,
    },
    /// Trigger async persist of add-tag after optimistic in-memory update
    TriggerAddTag {
        podcast_id: crate::storage::PodcastId,
        podcast_title: String,
        tag: String,
    },
    /// Trigger async persist of remove-tag after optimistic in-memory update
    TriggerRemoveTag {
        podcast_id: crate::storage::PodcastId,
        podcast_title: String,
        tag: String,
    },

    // Episode status actions
    /// Mark selected episode as played
    MarkPlayed,
    /// Mark selected episode as unplayed
    MarkUnplayed,
    /// Toggle favorite/starred state on selected episode
    ToggleFavorite,
    /// Trigger async mark episode as played (carries IDs for storage update)
    TriggerMarkPlayed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    },
    /// Trigger async mark episode as unplayed (carries IDs for storage update)
    TriggerMarkUnplayed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    },
    /// Trigger async persist of favorited state after optimistic in-memory toggle
    TriggerToggleFavorite {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
        /// The new favorited state to persist
        favorited: bool,
    },

    // Render request
    Render,

    // No operation
    None,

    /// Subscribe to a podcast directly from a discovery result (provides feed URL)
    SubscribeFromDiscovery {
        feed_url: String,
    },

    // Audio playback actions
    /// Play a downloaded episode through the audio backend
    PlayEpisode {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        path: std::path::PathBuf,
    },
    /// Toggle play/pause for the current track
    TogglePlayPause,
    /// Stop playback and clear the current track
    StopPlayback,
    /// Seek forward by `constants::audio::SEEK_STEP_SECS`
    SeekForward,
    /// Seek backward by `constants::audio::SEEK_STEP_SECS`
    SeekBackward,
    /// Increase volume by `constants::audio::VOLUME_STEP`
    VolumeUp,
    /// Decrease volume by `constants::audio::VOLUME_STEP`
    VolumeDown,
}

impl UIAction {
    /// Returns a short human-readable description of this action for help text generation.
    /// Returns an empty string for internal/trigger actions that are not user-bindable.
    pub fn description(&self) -> &str {
        match self {
            // Navigation
            UIAction::MoveUp => "Move up",
            UIAction::MoveDown => "Move down",
            UIAction::MoveLeft => "Move left",
            UIAction::MoveRight => "Move right",
            UIAction::PageUp => "Page up",
            UIAction::PageDown => "Page down",
            UIAction::MoveToTop => "Move to top",
            UIAction::MoveToBottom => "Move to bottom",
            UIAction::MoveEpisodeUp => "Reorder episode up",
            UIAction::MoveEpisodeDown => "Reorder episode down",
            // Buffer management
            UIAction::NextBuffer => "Next buffer",
            UIAction::PreviousBuffer => "Previous buffer",
            UIAction::CloseCurrentBuffer => "Close current buffer",
            UIAction::SwitchBuffer(name) => match name.as_str() {
                "podcast-list" => "Switch to podcast list",
                "downloads" => "Switch to downloads",
                "sync" => "Switch to sync",
                _ => "Switch to buffer",
            },
            // Application
            UIAction::Quit => "Quit application",
            UIAction::ShowHelp => "Show help",
            UIAction::Search => "Search",
            UIAction::ClearFilters => "Clear filters",
            UIAction::Refresh => "Refresh current buffer",
            UIAction::PromptCommand => "Enter command",
            UIAction::SelectItem => "Select / activate item",
            UIAction::HideMinibuffer => "Cancel / close minibuffer",
            UIAction::ExecuteCommand(cmd) => match cmd.as_str() {
                "switch-to-buffer" => "Switch to buffer by name",
                "list-buffers" => "List all buffers",
                _ => "",
            },
            // Podcast management
            UIAction::AddPodcast => "Add podcast subscription",
            UIAction::DeletePodcast => "Delete podcast / playlist",
            UIAction::DownloadEpisode => "Download selected episode",
            UIAction::DeleteDownloadedEpisode => "Delete downloaded episode file",
            UIAction::DeleteAllDownloads => "Delete all downloaded episodes",
            UIAction::RefreshPodcast => "Refresh selected podcast",
            UIAction::RefreshAll => "Refresh all podcasts",
            UIAction::HardRefreshPodcast => "Hard refresh podcast (re-parse episodes)",
            // Episode status
            UIAction::MarkPlayed => "Mark episode as played",
            UIAction::MarkUnplayed => "Mark episode as unplayed",
            UIAction::ToggleFavorite => "Toggle episode favorite (★)",
            UIAction::CycleSortField => "Cycle sort field",
            UIAction::ToggleSortDirection => "Toggle sort direction",
            // Playlists
            UIAction::OpenPlaylistList => "Switch to playlists",
            UIAction::CreatePlaylist => "Create playlist",
            UIAction::AddToPlaylist => "Add episode to playlist",
            // OPML
            UIAction::ImportOpml => "Import OPML",
            UIAction::ExportOpml => "Export OPML",
            // Sync / tabs
            UIAction::SyncToDevice => "Sync to device",
            UIAction::PreviousTab => "Previous tab",
            UIAction::NextTab => "Next tab",
            // Audio playback
            UIAction::PlayEpisode { .. } => "Play selected episode",
            UIAction::TogglePlayPause => "Toggle play / pause",
            UIAction::StopPlayback => "Stop playback",
            UIAction::SeekForward => "Seek forward",
            UIAction::SeekBackward => "Seek backward",
            UIAction::VolumeUp => "Volume up",
            UIAction::VolumeDown => "Volume down",
            // Internal / trigger actions — not shown in help
            _ => "",
        }
    }
}

/// Trait for UI components that can handle events and render themselves
pub trait UIComponent {
    /// Handle a UI action and return the resulting action
    fn handle_action(&mut self, action: UIAction) -> UIAction;

    /// Render the component to the given area
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect);

    /// Get the component's title for display
    fn title(&self) -> String;

    /// Check if this component should have focus
    fn has_focus(&self) -> bool;

    /// Set focus state for this component
    fn set_focus(&mut self, focused: bool);
}
