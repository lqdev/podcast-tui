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
    download::DownloadManager,
    podcast::subscription::SubscriptionManager,
    storage::JsonStorage,
    ui::{
        buffers::BufferManager,
        components::{minibuffer::Minibuffer, minibuffer::MinibufferContent, statusbar::StatusBar},
        events::{AppEvent, UIEvent, UIEventHandler},
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

    /// Download manager
    download_manager: Arc<DownloadManager<JsonStorage>>,

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

    /// App event sender for async communication
    app_event_tx: mpsc::UnboundedSender<AppEvent>,

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
        download_manager: Arc<DownloadManager<JsonStorage>>,
        app_event_tx: mpsc::UnboundedSender<AppEvent>,
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
            download_manager,
            buffer_manager,
            status_bar,
            minibuffer,
            key_handler,
            event_handler,
            app_event_tx,
            should_quit: false,
            last_render: Instant::now(),
            frame_count: 0,
        })
    }

    /// Run the UI application
    pub async fn run(
        &mut self,
        mut app_event_rx: mpsc::UnboundedReceiver<AppEvent>,
    ) -> UIResult<()> {
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
            // Wait for events or timeout
            tokio::select! {
                // Handle incoming UI events
                ui_event = event_rx.recv() => {
                    match ui_event {
                        Some(event) => {
                            match self.handle_event(event).await {
                                Ok(should_continue) => {
                                    if !should_continue {
                                        break Ok(());
                                    }
                                }
                                Err(e) => {
                                    self.show_error(format!("Event handling error: {}", e));
                                }
                            }
                        }
                        None => break Ok(()), // Channel closed
                    }
                }
                // Handle incoming app events (from async tasks)
                app_event = app_event_rx.recv() => {
                    match app_event {
                        Some(event) => {
                            if let Err(e) = self.handle_app_event(event).await {
                                self.show_error(format!("App event handling error: {}", e));
                            }
                        }
                        None => {} // Channel closed, continue
                    }
                }
                // Render timeout
                _ = tokio::time::sleep(Duration::from_millis(16)) => {
                    // Continue to rendering
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
                // Check if minibuffer is in input mode and handle input
                if self.minibuffer.is_input_mode() {
                    return self.handle_minibuffer_key(key_event).await;
                }

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
            UIAction::ShowMinibuffer(prompt) => {
                // If the prompt ends with a space, it's a key sequence indicator, not an input prompt
                if prompt.trim_end().ends_with("C-x")
                    || prompt.trim_end().ends_with("C-c")
                    || prompt.trim_end().ends_with("C-h")
                {
                    self.show_message(prompt);
                } else {
                    // This is an actual input prompt
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt,
                        input: String::new(),
                    });
                }
                Ok(true)
            }
            UIAction::SubmitInput(input) => {
                self.handle_minibuffer_input(input);
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
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Some(podcast) = podcast_buffer.selected_podcast() {
                        let podcast_id = podcast.id.clone();
                        let podcast_title = podcast.title.clone();

                        // Show confirmation prompt
                        self.minibuffer.set_content(MinibufferContent::Input {
                            prompt: format!("Delete podcast '{}' (y/n)? ", podcast_title),
                            input: String::new(),
                        });
                        // Store the podcast ID for deletion confirmation
                        // For MVP, we'll implement a simple confirmation flow
                    } else {
                        self.show_error("No podcast selected for deletion".to_string());
                    }
                } else {
                    self.show_error("Podcast list not available".to_string());
                }
                Ok(true)
            }
            UIAction::RefreshPodcast => {
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Some(podcast) = podcast_buffer.selected_podcast() {
                        let podcast_id = podcast.id.clone();
                        let podcast_title = podcast.title.clone();

                        // Show loading state
                        self.show_message(format!("Refreshing '{}'...", podcast_title));

                        // Trigger async refresh
                        self.trigger_async_refresh_single(podcast_id);
                    } else {
                        self.show_error("No podcast selected for refresh".to_string());
                    }
                } else {
                    self.show_error("Podcast list not available".to_string());
                }
                Ok(true)
            }
            UIAction::RefreshAll => {
                self.show_message("Refreshing all podcasts...".to_string());
                self.trigger_async_refresh_all();
                Ok(true)
            }
            UIAction::DownloadEpisode => {
                self.show_message("Download functionality not yet integrated into UI".to_string());
                Ok(true)
            }
            UIAction::DeleteDownloadedEpisode => {
                self.show_message(
                    "Delete download functionality not yet integrated into UI".to_string(),
                );
                Ok(true)
            }
            UIAction::OpenEpisodeList {
                podcast_name,
                podcast_id,
            } => {
                // Create and switch to episode list buffer
                self.buffer_manager.create_episode_list_buffer(
                    podcast_name.clone(),
                    podcast_id,
                    self.subscription_manager.clone(),
                    self.download_manager.clone(),
                );

                // Switch to the new buffer
                let episode_buffer_id =
                    format!("episodes-{}", podcast_name.replace(' ', "-").to_lowercase());
                let _ = self.buffer_manager.switch_to_buffer(&episode_buffer_id);
                self.update_status_bar();
                self.show_message(format!("Opened episodes for: {}", podcast_name));
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

    /// Handle app events from async tasks
    async fn handle_app_event(&mut self, event: AppEvent) -> UIResult<()> {
        match event {
            AppEvent::PodcastSubscribed { podcast } => {
                // Refresh the podcast list
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Err(e) = podcast_buffer.load_podcasts().await {
                        self.show_error(format!("Failed to refresh podcast list: {}", e));
                    }
                }
                self.show_message(format!("Successfully subscribed to: {}", podcast.title));
            }
            AppEvent::PodcastSubscriptionFailed { url, error } => {
                self.show_error(format!("Failed to subscribe to {}: {}", url, error));
            }
            AppEvent::PodcastRefreshed {
                podcast_id: _,
                new_episode_count,
            } => {
                // Refresh the podcast list (or specific podcast view)
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Err(e) = podcast_buffer.load_podcasts().await {
                        self.show_error(format!("Failed to refresh podcast list: {}", e));
                    }
                }
                if new_episode_count > 0 {
                    self.show_message(format!("Found {} new episode(s)", new_episode_count));
                } else {
                    self.show_message("No new episodes found".to_string());
                }
            }
            AppEvent::PodcastRefreshFailed {
                podcast_id: _,
                error,
            } => {
                self.show_error(format!("Failed to refresh podcast: {}", error));
            }
            AppEvent::AllPodcastsRefreshed { total_new_episodes } => {
                // Refresh the podcast list
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Err(e) = podcast_buffer.load_podcasts().await {
                        self.show_error(format!("Failed to refresh podcast list: {}", e));
                    }
                }
                if total_new_episodes > 0 {
                    self.show_message(format!(
                        "Refresh completed. Found {} new episode(s) total",
                        total_new_episodes
                    ));
                } else {
                    self.show_message("Refresh completed. No new episodes found".to_string());
                }
            }
        }
        Ok(())
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
                    self.show_message(format!("Adding podcast: {}...", url));
                    self.trigger_async_add_podcast(url);
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

    /// Trigger async podcast addition
    fn trigger_async_add_podcast(&mut self, url: String) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let url_clone = url.clone();

        // Spawn async task to add the podcast
        tokio::spawn(async move {
            match subscription_manager.subscribe(&url_clone).await {
                Ok(podcast) => {
                    // Send success event back to UI
                    let _ = app_event_tx.send(AppEvent::PodcastSubscribed { podcast });
                }
                Err(e) => {
                    // Send error event back to UI
                    let _ = app_event_tx.send(AppEvent::PodcastSubscriptionFailed {
                        url: url_clone.clone(),
                        error: e.to_string(),
                    });
                }
            }
        });

        // Show immediate feedback
        self.show_message(format!("Adding podcast: {}...", url));
    }

    /// Trigger async single podcast refresh
    fn trigger_async_refresh_single(&mut self, podcast_id: crate::storage::PodcastId) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();

        tokio::spawn(async move {
            match subscription_manager.refresh_feed(&podcast_id).await {
                Ok(new_episodes) => {
                    let _ = app_event_tx.send(AppEvent::PodcastRefreshed {
                        podcast_id: podcast_id_clone,
                        new_episode_count: new_episodes.len(),
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PodcastRefreshFailed {
                        podcast_id: podcast_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async refresh of all podcasts
    fn trigger_async_refresh_all(&mut self) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            match subscription_manager.refresh_all().await {
                Ok(total_new_episodes) => {
                    let _ =
                        app_event_tx.send(AppEvent::AllPodcastsRefreshed { total_new_episodes });
                }
                Err(e) => {
                    // For all refresh, we'll just show a general error
                    eprintln!("Failed to refresh podcasts: {}", e);
                }
            }
        });
    }

    /// Handle minibuffer input submission
    fn handle_minibuffer_input(&mut self, input: String) {
        let input = input.trim();

        if input.is_empty() {
            self.minibuffer.clear();
            return;
        }

        // Check if this looks like a URL (basic heuristic for podcast addition)
        if input.starts_with("http://") || input.starts_with("https://") {
            self.show_message(format!("Adding podcast: {}...", input));
            self.trigger_async_add_podcast(input.to_string());
        } else if input.to_lowercase() == "y" || input.to_lowercase() == "yes" {
            self.show_message("Podcast deletion confirmed (not implemented in MVP)".to_string());
        } else if input.to_lowercase() == "n" || input.to_lowercase() == "no" {
            self.show_message("Podcast deletion cancelled".to_string());
        } else {
            // Check if this is a buffer name
            let buffer_names = self.buffer_manager.buffer_names();
            let matching_buffer = buffer_names.iter().find(|(_, name)| {
                name.to_lowercase().contains(&input.to_lowercase())
                    || name.to_lowercase() == input.to_lowercase()
            });

            if let Some((buffer_id, buffer_name)) = matching_buffer {
                if let Err(_) = self.buffer_manager.switch_to_buffer(&buffer_id) {
                    self.show_error(format!("Failed to switch to buffer: {}", buffer_name));
                } else {
                    self.update_status_bar();
                    self.show_message(format!("Switched to buffer: {}", buffer_name));
                }
            } else {
                // Treat as a command
                let _ = self.execute_command_direct(input.to_string());
            }
        }

        self.minibuffer.clear();
    }

    /// Handle key events when minibuffer is in input mode
    async fn handle_minibuffer_key(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> UIResult<bool> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key_event.code, key_event.modifiers) {
            // Submit input on Enter
            (KeyCode::Enter, _) => {
                if let Some(input) = self.minibuffer.submit() {
                    self.handle_minibuffer_input(input);
                }
                Ok(true)
            }
            // Cancel on Ctrl+G or Escape
            (KeyCode::Esc, _) | (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.minibuffer.clear();
                Ok(true)
            }
            // Backspace
            (KeyCode::Backspace, _) => {
                self.minibuffer.backspace();
                Ok(true)
            }
            // Cursor movement
            (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.minibuffer.cursor_left();
                Ok(true)
            }
            (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.minibuffer.cursor_right();
                Ok(true)
            }
            // History navigation
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.minibuffer.history_up();
                Ok(true)
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.minibuffer.history_down();
                Ok(true)
            }
            // Regular character input
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.minibuffer.add_char(c);
                Ok(true)
            }
            // Ignore other keys in input mode
            _ => Ok(true),
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
                Constraint::Length(2), // Minibuffer (1 for border + 1 for text)
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
    use crate::{config::UiConfig, download::DownloadManager};

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
        let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));
        let download_manager =
            Arc::new(DownloadManager::new(storage, temp_dir.path().to_path_buf()).unwrap());

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let app = UIApp::new(config, subscription_manager, download_manager, app_event_tx);
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
        let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));
        let download_manager =
            Arc::new(DownloadManager::new(storage, temp_dir.path().to_path_buf()).unwrap());

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app =
            UIApp::new(config, subscription_manager, download_manager, app_event_tx).unwrap();

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
        let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));
        let download_manager =
            Arc::new(DownloadManager::new(storage, temp_dir.path().to_path_buf()).unwrap());

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app =
            UIApp::new(config, subscription_manager, download_manager, app_event_tx).unwrap();
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
        let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));
        let download_manager =
            Arc::new(DownloadManager::new(storage, temp_dir.path().to_path_buf()).unwrap());

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app =
            UIApp::new(config, subscription_manager, download_manager, app_event_tx).unwrap();
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
        let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));
        let download_manager =
            Arc::new(DownloadManager::new(storage, temp_dir.path().to_path_buf()).unwrap());

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app =
            UIApp::new(config, subscription_manager, download_manager, app_event_tx).unwrap();

        let result = app.set_theme_direct("light");
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result = app.set_theme_direct("invalid-theme");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
