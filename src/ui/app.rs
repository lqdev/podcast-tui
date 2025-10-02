//!
//! This module contains the main UI application that coordinates
//! all UI components, manages state, and handles the event loop.

use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tokio::sync::mpsc;

use crate::{
    config::Config,
    podcast::subscription::SubscriptionManager,
    storage::JsonStorage,
    ui::{
        buffers::BufferManager,
        components::{minibuffer::Minibuffer, minibuffer::MinibufferContent, statusbar::StatusBar},
        events::{UIEvent, UIEventHandler},
        keybindings::KeyHandler,
        themes::Theme,
        UIAction, UIComponent, UIError, UIResult,
    },
};
use std::sync::Arc;

/// The main UI application
pub struct UIApp {
    /// Configuration
    config: Config,

    /// Current theme
    theme: Theme,

    /// Subscription manager
    subscription_manager: Arc<SubscriptionManager<JsonStorage>>,

    /// Buffer manager
    buffer_manager: BufferManager,

    /// Status bar component
    status_bar: StatusBar,

    /// Minibuffer component
    minibuffer: Minibuffer,

    /// Keybinding handler
    key_handler: KeyHandler,

    /// Event handler
    event_handler: UIEventHandler,

    /// Whether the application should quit
    should_quit: bool,

    /// Last render time for performance tracking
    last_render: Instant,

    /// Frame counter for debugging
    frame_count: u64,
}

impl UIApp {
    /// Create a new UI application
    pub fn new(
        config: Config,
        subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
    ) -> UIResult<Self> {
        let theme = Theme::from_name(&config.ui.theme)?;
        let buffer_manager = BufferManager::new();
        let mut status_bar = StatusBar::new();
        status_bar.set_theme(theme.clone());

        let minibuffer = Minibuffer::new();
        let key_handler = KeyHandler::new();
        let event_handler = UIEventHandler::new(Duration::from_millis(250)); // 250ms tick rate

        Ok(Self {
            config,
            theme,
            subscription_manager,
            buffer_manager,
            status_bar,
            minibuffer,
            key_handler,
            event_handler,
            should_quit: false,
            last_render: Instant::now(),
            frame_count: 0,
        })
    }

    /// Run the UI application
    pub async fn run(&mut self) -> UIResult<()> {
        // Initialize terminal
        enable_raw_mode().map_err(UIError::Terminal)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(UIError::Terminal)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(UIError::Terminal)?;

        // Create event channel
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        // Start event handler
        let event_handler = self.event_handler.clone();
        tokio::spawn(async move {
            event_handler.run(event_tx).await;
        });

        // Initialize UI state
        self.initialize().await?;

        // Main event loop
        let result = loop {
            // Handle events
            while let Ok(ui_event) = event_rx.try_recv() {
                match self.handle_event(ui_event).await {
                    Ok(should_continue) => {
                        if !should_continue {
                            break;
                        }
                    }
                    Err(e) => {
                        self.show_error(format!("Event handling error: {}", e));
                    }
                }
            }

            // Check if we should quit
            if self.should_quit {
                break Ok(());
            }

            // Render the UI
            match terminal.draw(|f| self.render(f)) {
                Ok(_) => {
                    self.frame_count += 1;
                    self.last_render = Instant::now();
                }
                Err(e) => break Err(UIError::Render(e.to_string())),
            }

            // Small delay to prevent excessive CPU usage
            tokio::time::sleep(Duration::from_millis(16)).await; // ~60 FPS
        };

        // Cleanup terminal
        disable_raw_mode().map_err(UIError::Terminal)?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(UIError::Terminal)?;
        terminal.show_cursor().map_err(UIError::Terminal)?;

        result
    }

    /// Initialize the UI application
    async fn initialize(&mut self) -> UIResult<()> {
        // Create initial buffers
        self.buffer_manager.create_help_buffer();
        self.buffer_manager
            .create_podcast_list_buffer(self.subscription_manager.clone());

        // Set initial buffer
        if let Some(buffer_id) = self.buffer_manager.get_buffer_ids().first() {
            self.buffer_manager.switch_to_buffer(&buffer_id.clone());
        }

        // Load initial podcast data
        if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
            if let Err(e) = podcast_buffer.load_podcasts().await {
                self.show_error(format!("Failed to load podcasts: {}", e));
            }
        }

        // Update status bar
        self.update_status_bar();

        // Show welcome message
        self.show_message("Welcome to Podcast TUI! Press C-h for help.".to_string());

        Ok(())
    }

    /// Handle a UI event
    async fn handle_event(&mut self, event: UIEvent) -> UIResult<bool> {
        match event {
            UIEvent::Key(key_event) => {
                // Handle key event through keybinding system
                let action = self.key_handler.handle_key(key_event);
                self.handle_action(action).await
            }
            UIEvent::Mouse(_) => {
                // Mouse events not implemented for MVP
                Ok(true)
            }
            UIEvent::Resize(_, _) => {
                // Terminal was resized, just continue
                Ok(true)
            }
            UIEvent::Tick => {
                // Periodic tick event
                self.handle_tick().await
            }
            UIEvent::Quit => {
                self.should_quit = true;
                Ok(false)
            }
        }
    }

    /// Handle a UI action
    async fn handle_action(&mut self, action: UIAction) -> UIResult<bool> {
        match action {
            UIAction::None => Ok(true),
            UIAction::Quit => {
                self.should_quit = true;
                Ok(false)
            }
            UIAction::ShowHelp => {
                let _ = self.buffer_manager.switch_to_buffer(&"*help*".to_string());
                self.update_status_bar();
                Ok(true)
            }
            UIAction::SwitchBuffer(name) => {
                let _ = self.buffer_manager.switch_to_buffer(&name);
                self.update_status_bar();
                Ok(true)
            }
            UIAction::NextBuffer => {
                let _ = self.buffer_manager.next_buffer();
                self.update_status_bar();
                Ok(true)
            }
            UIAction::PreviousBuffer => {
                let _ = self.buffer_manager.previous_buffer();
                self.update_status_bar();
                Ok(true)
            }
            UIAction::ShowMessage(msg) => {
                self.show_message(msg);
                Ok(true)
            }
            UIAction::ShowError(msg) => {
                self.show_error(msg);
                Ok(true)
            }
            UIAction::PromptCommand => {
                self.minibuffer
                    .set_content(MinibufferContent::CommandPrompt);
                Ok(true)
            }
            UIAction::ExecuteCommand(cmd) => {
                // Handle command execution directly without recursion
                self.execute_command_direct(cmd)
            }
            UIAction::ClearMinibuffer => {
                self.minibuffer.clear();
                Ok(true)
            }
            UIAction::AddPodcast => {
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: "Add podcast URL: ".to_string(),
                    input: String::new(),
                });
                Ok(true)
            }
            UIAction::DeletePodcast => {
                // For MVP, just show a message that action is triggered
                // The actual async operation would need a different architecture
                self.show_message(
                    "Delete subscription action triggered (async operation not implemented in MVP)"
                        .to_string(),
                );
                Ok(true)
            }
            UIAction::RefreshPodcast => {
                // For MVP, just show a message that action is triggered
                self.show_message(
                    "Refresh podcast action triggered (async operation not implemented in MVP)"
                        .to_string(),
                );
                Ok(true)
            }
            UIAction::RefreshAll => {
                // For MVP, just show a message that action is triggered
                self.show_message("Refresh all podcasts action triggered (async operation not implemented in MVP)".to_string());
                Ok(true)
            }
            // Buffer-specific actions
            action => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    current_buffer.handle_action(action);
                }
                Ok(true)
            }
        }
    }

    /// Handle periodic tick
    async fn handle_tick(&mut self) -> UIResult<bool> {
        // Update status bar with current key sequence
        let sequence = self.key_handler.current_sequence_string();
        if !sequence.is_empty() {
            self.status_bar.set_key_sequence(sequence);
        } else {
            self.status_bar.set_key_sequence(String::new());
        }

        Ok(true)
    }

    /// Execute a command directly without recursion
    fn execute_command_direct(&mut self, command: String) -> UIResult<bool> {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "quit" | "q" => {
                self.should_quit = true;
                Ok(false)
            }
            "help" | "h" => {
                let _ = self.buffer_manager.switch_to_buffer(&"*help*".to_string());
                self.update_status_bar();
                Ok(true)
            }
            "theme" => {
                if parts.len() > 1 {
                    self.set_theme_direct(parts[1])
                } else {
                    self.show_error(
                        "Usage: theme <name> (dark, light, high-contrast, solarized)".to_string(),
                    );
                    Ok(true)
                }
            }
            "buffer" | "b" => {
                if parts.len() > 1 {
                    let _ = self.buffer_manager.switch_to_buffer(&parts[1].to_string());
                    self.update_status_bar();
                    Ok(true)
                } else {
                    self.show_buffer_list();
                    Ok(true)
                }
            }
            "add-podcast" => {
                if parts.len() > 1 {
                    let url = parts[1].to_string();
                    self.show_message(format!(
                        "Add podcast URL: {} (async operation not implemented in MVP)",
                        url
                    ));
                    Ok(true)
                } else {
                    self.show_error("Usage: add-podcast <URL>".to_string());
                    Ok(true)
                }
            }
            _ => {
                self.show_error(format!("Unknown command: {}", parts[0]));
                Ok(true)
            }
        }
    }

    /// Set the application theme
    async fn set_theme(&mut self, theme_name: &str) -> UIResult<bool> {
        match Theme::from_name(theme_name) {
            Ok(new_theme) => {
                self.theme = new_theme.clone();
                self.status_bar.set_theme(new_theme);
                self.show_message(format!("Theme changed to: {}", theme_name));
                Ok(true)
            }
            Err(_) => {
                self.show_error(format!("Unknown theme: {}", theme_name));
                Ok(true)
            }
        }
    }

    /// Set the application theme (direct version)
    fn set_theme_direct(&mut self, theme_name: &str) -> UIResult<bool> {
        match Theme::from_name(theme_name) {
            Ok(new_theme) => {
                self.theme = new_theme.clone();
                self.status_bar.set_theme(new_theme);
                self.show_message(format!("Theme changed to: {}", theme_name));
                Ok(true)
            }
            Err(_) => {
                self.show_error(format!("Unknown theme: {}", theme_name));
                Ok(true)
            }
        }
    }

    /// Show list of available buffers
    fn show_buffer_list(&mut self) {
        let buffer_ids = self.buffer_manager.get_buffer_ids();
        let current = self
            .buffer_manager
            .current_buffer_name()
            .unwrap_or_default();

        let mut message = "Available buffers:\n".to_string();
        for id in buffer_ids {
            if id == current {
                message.push_str(&format!("* {}\n", id));
            } else {
                message.push_str(&format!("  {}\n", id));
            }
        }

        self.show_message(message);
    }

    /// Show a message in the minibuffer
    fn show_message(&mut self, message: String) {
        self.minibuffer
            .set_content(MinibufferContent::Message(message));
        self.status_bar.clear_status_message();
    }

    /// Show an error in the minibuffer
    fn show_error(&mut self, error: String) {
        self.minibuffer.set_content(MinibufferContent::Error(error));
        self.status_bar.clear_status_message();
    }

    /// Update the status bar with current state
    fn update_status_bar(&mut self) {
        if let Some(buffer_name) = self.buffer_manager.current_buffer_name() {
            self.status_bar.set_buffer_name(buffer_name);
        }
    }

    /// Render the UI
    fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create layout: main area + minibuffer + status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Minibuffer
                Constraint::Length(1), // Status bar
            ])
            .split(size);

        // Render main content area
        self.render_main_content(frame, chunks[0]);

        // Render minibuffer
        self.minibuffer.render(frame, chunks[1]);

        // Render status bar
        self.status_bar.render(frame, chunks[2]);
    }

    /// Render the main content area
    fn render_main_content(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
            current_buffer.render(frame, area);
        } else {
            // No buffer selected, show empty area
            let block = Block::default()
                .borders(Borders::ALL)
                .title("No Buffer")
                .style(self.theme.default_style());
            frame.render_widget(block, area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UiConfig;

    #[tokio::test]
    async fn test_ui_app_creation() {
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config {
            ui: UiConfig::default(),
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let subscription_manager = Arc::new(SubscriptionManager::new(storage));

        let app = UIApp::new(config, subscription_manager);
        assert!(app.is_ok());

        let app = app.unwrap();
        assert!(!app.should_quit);
        assert_eq!(app.frame_count, 0);
    }

    #[tokio::test]
    async fn test_quit_action() {
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let subscription_manager = Arc::new(SubscriptionManager::new(storage));

        let mut app = UIApp::new(config, subscription_manager).unwrap();

        let result = app.handle_action(UIAction::Quit).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false to indicate stopping
        assert!(app.should_quit);
    }

    #[tokio::test]
    async fn test_show_help_action() {
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let subscription_manager = Arc::new(SubscriptionManager::new(storage));

        let mut app = UIApp::new(config, subscription_manager).unwrap();
        app.initialize().await.unwrap();

        let result = app.handle_action(UIAction::ShowHelp).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Should have switched to help buffer
        assert_eq!(app.buffer_manager.current_buffer_name().unwrap(), "*Help*");
    }

    #[tokio::test]
    async fn test_command_execution() {
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let subscription_manager = Arc::new(SubscriptionManager::new(storage));

        let mut app = UIApp::new(config, subscription_manager).unwrap();
        app.initialize().await.unwrap();

        // Test quit command
        let result = app.execute_command_direct("quit".to_string());
        assert!(result.is_ok());
        assert!(!result.unwrap());
        assert!(app.should_quit);

        // Reset for next test
        app.should_quit = false;

        // Test help command
        let result = app.execute_command_direct("help".to_string());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_theme_setting() {
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let subscription_manager = Arc::new(SubscriptionManager::new(storage));

        let mut app = UIApp::new(config, subscription_manager).unwrap();

        let result = app.set_theme_direct("light");
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result = app.set_theme_direct("invalid-theme");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
