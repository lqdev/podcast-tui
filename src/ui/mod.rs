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

    // Podcast management actions
    AddPodcast,
    DeletePodcast,
    RefreshPodcast,
    RefreshAll,

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
