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
pub mod keybindings;
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
        episode: crate::podcast::Episode,
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

    // Render request
    Render,

    // No operation
    None,
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
