//!
//! This module contains the main UI application that coordinates
//! all UI components, manages state, and handles the event loop.

use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
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
    constants::ui as ui_constants,
    download::DownloadManager,
    podcast::subscription::SubscriptionManager,
    storage::{JsonStorage, Storage},
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

    /// Storage (reserved for future direct access)
    _storage: Arc<JsonStorage>,

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

    /// Podcast ID pending deletion confirmation
    pending_deletion: Option<crate::storage::PodcastId>,

    /// Whether we're pending bulk deletion confirmation
    pending_bulk_deletion: bool,

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
        storage: Arc<JsonStorage>,
        app_event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> UIResult<Self> {
        let theme = Theme::from_name(&config.ui.theme)?;
        let buffer_manager = BufferManager::new();
        let mut status_bar = StatusBar::new();
        status_bar.set_theme(theme.clone());

        let minibuffer = Minibuffer::new();
        let key_handler = KeyHandler::new();
        let event_handler = UIEventHandler::new(Duration::from_millis(ui_constants::UI_TICK_RATE_MS));

        Ok(Self {
            config,
            theme,
            subscription_manager,
            download_manager,
            _storage: storage,
            buffer_manager,
            status_bar,
            minibuffer,
            key_handler,
            event_handler,
            app_event_tx,
            should_quit: false,
            last_render: Instant::now(),
            frame_count: 0,
            pending_deletion: None,
            pending_bulk_deletion: false,
        })
    }

    /// Create a new UI application with progress reporting
    pub async fn new_with_progress(
        config: Config,
        subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
        download_manager: Arc<DownloadManager<JsonStorage>>,
        storage: Arc<JsonStorage>,
        app_event_tx: mpsc::UnboundedSender<AppEvent>,
        status_tx: mpsc::UnboundedSender<crate::InitStatus>,
    ) -> UIResult<Self> {
        let theme = Theme::from_name(&config.ui.theme)?;
        let mut buffer_manager = BufferManager::new();
        let mut status_bar = StatusBar::new();
        status_bar.set_theme(theme.clone());

        let minibuffer = Minibuffer::new();
        let key_handler = KeyHandler::new();
        let event_handler = UIEventHandler::new(Duration::from_millis(ui_constants::UI_TICK_RATE_MS));

        // Create buffers with progress updates
        status_tx.send(crate::InitStatus::CreatingBuffers).ok();
        buffer_manager.create_help_buffer();
        buffer_manager.create_podcast_list_buffer(subscription_manager.clone());
        buffer_manager.create_downloads_buffer(download_manager.clone(), storage.clone());
        buffer_manager.create_whats_new_buffer(
            subscription_manager.clone(),
            download_manager.clone(),
            config.ui.whats_new_episode_limit,
        );

        // Set initial buffer
        if let Some(buffer_id) = buffer_manager.get_buffer_ids().first() {
            let _ = buffer_manager.switch_to_buffer(&buffer_id.clone());
        }

        // Load data with progress updates
        status_tx.send(crate::InitStatus::LoadingPodcasts).ok();
        if let Some(podcast_buffer) = buffer_manager.get_podcast_list_buffer_mut() {
            let _ = podcast_buffer.load_podcasts().await;
        }

        status_tx.send(crate::InitStatus::LoadingDownloads).ok();
        if let Some(downloads_buffer) = buffer_manager.get_downloads_buffer_mut() {
            let _ = downloads_buffer.refresh_downloads().await;
        }

        status_tx.send(crate::InitStatus::LoadingWhatsNew).ok();
        if let Some(whats_new_buffer) = buffer_manager.get_whats_new_buffer_mut() {
            let _ = whats_new_buffer.load_episodes().await;
        }

        status_tx.send(crate::InitStatus::Complete).ok();

        Ok(Self {
            config,
            theme,
            subscription_manager,
            download_manager,
            _storage: storage,
            buffer_manager,
            status_bar,
            minibuffer,
            key_handler,
            event_handler,
            app_event_tx,
            should_quit: false,
            last_render: Instant::now(),
            frame_count: 0,
            pending_deletion: None,
            pending_bulk_deletion: false,
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

        // Initialize UI state only if buffers weren't already loaded
        if self.buffer_manager.get_buffer_ids().is_empty() {
            self.initialize().await?;
        } else {
            // Buffers already loaded, just update status and show welcome
            self.update_status_bar();
            self.show_message("Welcome to Podcast TUI! Press F1 or ? for help.".to_string());
        }

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
        // Clean up any stuck downloads on startup
        if let Err(e) = self.download_manager.cleanup_stuck_downloads().await {
            self.show_error(format!("Failed to cleanup stuck downloads: {}", e));
        }

        // Create initial buffers
        self.buffer_manager.create_help_buffer();
        self.buffer_manager
            .create_podcast_list_buffer(self.subscription_manager.clone());
        self.buffer_manager.create_downloads_buffer(
            self.download_manager.clone(),
            self.download_manager.storage().clone(),
        );
        self.buffer_manager.create_whats_new_buffer(
            self.subscription_manager.clone(),
            self.download_manager.clone(),
            self.config.ui.whats_new_episode_limit,
        );

        // Set initial buffer
        if let Some(buffer_id) = self.buffer_manager.get_buffer_ids().first() {
            let _ = self.buffer_manager.switch_to_buffer(&buffer_id.clone());
        }

        // Load initial podcast data
        if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
            if let Err(e) = podcast_buffer.load_podcasts().await {
                self.show_error(format!("Failed to load podcasts: {}", e));
            }
        }

        // Load initial downloads data
        if let Some(downloads_buffer) = self.buffer_manager.get_downloads_buffer_mut() {
            if let Err(e) = downloads_buffer.refresh_downloads().await {
                self.show_error(format!("Failed to load downloads: {}", e));
            }
        }

        // Load initial What's New data
        if let Some(whats_new_buffer) = self.buffer_manager.get_whats_new_buffer_mut() {
            if let Err(e) = whats_new_buffer.load_episodes().await {
                self.show_error(format!("Failed to load What's New episodes: {}", e));
            }
        }

        // Update status bar
        self.update_status_bar();

        // Show welcome message
        self.show_message("Welcome to Podcast TUI! Press F1 or ? for help.".to_string());

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
                self.show_command_prompt_with_completion();
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
            UIAction::TabComplete => {
                // Tab completion is handled by the minibuffer directly
                // This action is triggered by Tab key in minibuffer input mode
                Ok(true)
            }
            UIAction::CloseCurrentBuffer => {
                if let Some(current_id) = self.buffer_manager.current_buffer_id() {
                    match self.buffer_manager.remove_buffer(&current_id) {
                        Ok(_) => {
                            self.update_status_bar();
                            self.show_message(format!("Closed buffer: {}", current_id));
                            Ok(true)
                        }
                        Err(e) => {
                            self.show_error(format!("Cannot close buffer: {}", e));
                            Ok(true)
                        }
                    }
                } else {
                    self.show_error("No buffer to close".to_string());
                    Ok(true)
                }
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

                        // Store the podcast ID for deletion confirmation
                        self.pending_deletion = Some(podcast_id);

                        // Show confirmation prompt
                        self.minibuffer.set_content(MinibufferContent::Input {
                            prompt: format!("Delete podcast '{}' (y/n)? ", podcast_title),
                            input: String::new(),
                        });
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
            UIAction::HardRefreshPodcast => {
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Some(podcast) = podcast_buffer.selected_podcast() {
                        let podcast_id = podcast.id.clone();
                        let podcast_title = podcast.title.clone();

                        // Show loading state
                        self.show_message(format!(
                            "Hard refreshing '{}' (re-parsing all episodes)...",
                            podcast_title
                        ));

                        // Trigger async hard refresh
                        self.trigger_async_hard_refresh_single(podcast_id);
                    } else {
                        self.show_error("No podcast selected for hard refresh".to_string());
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
                // Pass to the current buffer to handle
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result_action = current_buffer.handle_action(action);
                    // Handle the result action immediately (non-recursively)
                    match result_action {
                        UIAction::TriggerDownload {
                            podcast_id,
                            episode_id,
                            episode_title,
                        } => {
                            self.show_message(format!("Starting download: {}", episode_title));
                            self.trigger_async_download(podcast_id, episode_id);
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        _ => {}
                    }
                    Ok(true)
                } else {
                    self.show_message("No active buffer to handle download".to_string());
                    Ok(true)
                }
            }
            UIAction::DeleteDownloadedEpisode => {
                // Pass to the current buffer to handle
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result_action = current_buffer.handle_action(action);
                    // Handle the result action immediately (non-recursively)
                    match result_action {
                        UIAction::TriggerDeleteDownload {
                            podcast_id,
                            episode_id,
                            episode_title,
                        } => {
                            self.show_message(format!("Deleting download: {}", episode_title));
                            self.trigger_async_delete_download(podcast_id, episode_id);
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        _ => {}
                    }
                    Ok(true)
                } else {
                    self.show_message("No active buffer to handle delete".to_string());
                    Ok(true)
                }
            }
            UIAction::TriggerDownload {
                podcast_id,
                episode_id,
                episode_title,
            } => {
                self.show_message(format!("Starting download: {}", episode_title));
                self.trigger_async_download(podcast_id, episode_id);
                Ok(true)
            }
            UIAction::TriggerDeleteDownload {
                podcast_id,
                episode_id,
                episode_title,
            } => {
                self.show_message(format!("Deleting download: {}", episode_title));
                self.trigger_async_delete_download(podcast_id, episode_id);
                Ok(true)
            }
            UIAction::TriggerRefreshDownloads => {
                self.show_message("Refreshing downloads...".to_string());
                self.trigger_async_refresh_downloads();
                Ok(true)
            }
            UIAction::DeleteAllDownloads => {
                // Show confirmation prompt
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: "Delete ALL downloaded episodes? This will remove all downloaded files! (y/n) ".to_string(),
                    input: String::new(),
                });
                // Set a flag to indicate we're confirming bulk deletion
                self.pending_bulk_deletion = true;
                Ok(true)
            }
            UIAction::ImportOpml => {
                // Show prompt for file path or URL
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: "Import OPML from (file path or URL): ".to_string(),
                    input: String::new(),
                });
                Ok(true)
            }
            UIAction::ExportOpml => {
                // Show prompt for output path with default
                let default_path = shellexpand::tilde(&self.config.storage.opml_export_directory).to_string();
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: format!("Export to (default: {}): ", default_path),
                    input: String::new(),
                });
                Ok(true)
            }
            UIAction::TriggerOpmlImport { source } => {
                self.show_message(format!("Importing OPML from: {}...", source));
                self.trigger_async_opml_import(source);
                Ok(true)
            }
            UIAction::TriggerOpmlExport { path } => {
                let output_path = if let Some(p) = path {
                    p
                } else {
                    // Use default from config
                    shellexpand::tilde(&self.config.storage.opml_export_directory).to_string()
                };
                self.show_message(format!("Exporting OPML to: {}...", output_path));
                self.trigger_async_opml_export(output_path);
                Ok(true)
            }
            UIAction::Refresh => {
                // Handle F5 refresh - refresh current buffer based on its type
                let current_buffer_id = self.buffer_manager.current_buffer_id();
                if let Some(buffer_id) = current_buffer_id {
                    if buffer_id.starts_with("episodes-") {
                        // If it's an episode list buffer, refresh its episodes
                        if let Some(episode_buffer) = self
                            .buffer_manager
                            .get_episode_list_buffer_mut_by_id(&buffer_id)
                        {
                            let _podcast_id = episode_buffer.podcast_id.clone();
                            let _ = episode_buffer.load_episodes().await;
                            self.show_message("Episode list refreshed".to_string());
                        }
                    } else if buffer_id == "podcast-list" {
                        // If it's the podcast list, refresh podcasts
                        if let Some(podcast_buffer) =
                            self.buffer_manager.get_podcast_list_buffer_mut()
                        {
                            if let Err(e) = podcast_buffer.load_podcasts().await {
                                self.show_error(format!("Failed to refresh podcast list: {}", e));
                            } else {
                                self.show_message("Podcast list refreshed".to_string());
                            }
                        }
                    } else if buffer_id == "downloads" {
                        // If it's the downloads buffer, refresh downloads
                        if let Some(downloads_buffer) =
                            self.buffer_manager.get_downloads_buffer_mut()
                        {
                            if let Err(e) = downloads_buffer.refresh_downloads().await {
                                self.show_error(format!("Failed to refresh downloads: {}", e));
                            } else {
                                self.show_message("Downloads refreshed".to_string());
                            }
                        }
                    } else if buffer_id == "whats-new" {
                        // If it's the What's New buffer, refresh episodes
                        if let Some(whats_new_buffer) =
                            self.buffer_manager.get_whats_new_buffer_mut()
                        {
                            if let Err(e) = whats_new_buffer.load_episodes().await {
                                self.show_error(format!("Failed to refresh What's New: {}", e));
                            } else {
                                self.show_message("What's New refreshed".to_string());
                            }
                        }
                    } else {
                        self.show_message("Refresh not supported for this buffer".to_string());
                    }
                } else {
                    self.show_message("No active buffer to refresh".to_string());
                }
                Ok(true)
            }
            // Buffer-specific actions
            action => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result_action = current_buffer.handle_action(action);
                    // If the buffer returns an action, handle it
                    match result_action {
                        UIAction::SwitchBuffer(buffer_id) => {
                            // Handle buffer switching from buffer list
                            if let Err(_) = self.buffer_manager.switch_to_buffer(&buffer_id) {
                                self.show_error(format!(
                                    "Failed to switch to buffer: {}",
                                    buffer_id
                                ));
                            } else {
                                self.update_status_bar();
                                self.show_message(format!("Switched to buffer: {}", buffer_id));
                            }
                        }
                        UIAction::CloseBuffer(buffer_id) => {
                            // Handle buffer closing from buffer list
                            match self.buffer_manager.remove_buffer(&buffer_id) {
                                Ok(_) => {
                                    self.update_status_bar();
                                    self.show_message(format!("Closed buffer: {}", buffer_id));
                                }
                                Err(e) => {
                                    self.show_error(format!("Cannot close buffer: {}", e));
                                }
                            }
                        }
                        UIAction::OpenEpisodeList {
                            podcast_name,
                            podcast_id,
                        } => {
                            // Handle this specific action directly to avoid recursion
                            let episode_buffer_id = format!(
                                "episodes-{}",
                                podcast_name.replace(' ', "-").to_lowercase()
                            );

                            // Check if buffer already exists
                            if !self
                                .buffer_manager
                                .get_buffer_ids()
                                .contains(&episode_buffer_id)
                            {
                                self.buffer_manager.create_episode_list_buffer(
                                    podcast_name.clone(),
                                    podcast_id.clone(),
                                    self.subscription_manager.clone(),
                                    self.download_manager.clone(),
                                );
                            }

                            // Switch to the buffer
                            let _ = self.buffer_manager.switch_to_buffer(&episode_buffer_id);
                            self.update_status_bar();

                            // Refresh any open buffer list buffers
                            self.refresh_buffer_list_if_open();

                            // Trigger async loading of episodes
                            self.trigger_async_load_episodes(podcast_id, podcast_name.clone());

                            self.show_message(format!("Loading episodes for: {}", podcast_name));
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        UIAction::ShowError(msg) => {
                            self.show_error(msg);
                        }
                        _ => {
                            // Ignore other actions to avoid infinite recursion
                        }
                    }
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
                // Refresh What's New buffer to show new episodes
                self.refresh_whats_new_buffer().await;
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
                // Refresh What's New buffer to show all new episodes
                self.refresh_whats_new_buffer().await;
                if total_new_episodes > 0 {
                    self.show_message(format!(
                        "Refresh completed. Found {} new episode(s) total",
                        total_new_episodes
                    ));
                } else {
                    self.show_message("Refresh completed. No new episodes found".to_string());
                }
            }
            AppEvent::PodcastDeleted {
                podcast_id: _,
                podcast_title,
            } => {
                // Refresh the podcast list
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Err(e) = podcast_buffer.load_podcasts().await {
                        self.show_error(format!("Failed to refresh podcast list: {}", e));
                    }
                }
                self.show_message(format!("Successfully deleted: {}", podcast_title));
            }
            AppEvent::PodcastDownloadsDeleted {
                podcast_id: _,
                deleted_count,
            } => {
                if deleted_count > 0 {
                    self.show_message(format!("Deleted {} downloaded episodes", deleted_count));
                }
            }
            AppEvent::PodcastDeletionFailed {
                podcast_id: _,
                error,
            } => {
                self.show_error(format!("Failed to delete podcast: {}", error));
            }
            AppEvent::EpisodesLoaded {
                podcast_id: _,
                podcast_name,
                episodes,
            } => {
                // Update the episode buffer with the loaded episodes
                if let Some(episode_buffer) = self
                    .buffer_manager
                    .get_episode_list_buffer_mut(&podcast_name)
                {
                    episode_buffer.set_episodes(episodes.clone());
                    self.show_message(format!("Loaded {} episodes", episodes.len()));
                } else {
                    self.show_message(format!("Loaded {} episodes", episodes.len()));
                }
            }
            AppEvent::EpisodesLoadFailed {
                podcast_id: _,
                error,
            } => {
                self.show_error(format!("Failed to load episodes: {}", error));
            }
            AppEvent::EpisodeDownloaded {
                podcast_id,
                episode_id: _,
            } => {
                // Refresh the episode list to show updated status
                self.refresh_episode_buffers(&podcast_id).await;
                // Refresh downloads buffer to show new completed download
                self.refresh_downloads_buffer().await;
                // Refresh What's New buffer to remove downloaded episode
                self.refresh_whats_new_buffer().await;
                self.show_message("Episode download completed successfully".to_string());
            }
            AppEvent::EpisodeDownloadFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Refresh the episode list to show updated status
                self.refresh_episode_buffers(&podcast_id).await;
                // Refresh downloads buffer to show failed download
                self.refresh_downloads_buffer().await;
                self.show_error(format!("Episode download failed: {}", error));
            }
            AppEvent::EpisodeDownloadDeleted {
                podcast_id,
                episode_id: _,
            } => {
                // Refresh the episode list to show updated status
                self.refresh_episode_buffers(&podcast_id).await;
                // Refresh downloads buffer to remove deleted download
                self.refresh_downloads_buffer().await;
                self.show_message("Episode download deleted successfully".to_string());
            }
            AppEvent::EpisodeDownloadDeletionFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Refresh the episode list to show updated status
                self.refresh_episode_buffers(&podcast_id).await;
                // Refresh downloads buffer in case status changed
                self.refresh_downloads_buffer().await;
                self.show_error(format!("Failed to delete episode download: {}", error));
            }
            AppEvent::DownloadsRefreshed => {
                // Refresh the downloads buffer
                self.refresh_downloads_buffer().await;
                self.show_message("Downloads refreshed".to_string());
            }
            AppEvent::AllDownloadsDeleted { deleted_count } => {
                // Refresh all episode buffers and downloads buffer
                self.refresh_all_episode_buffers().await;
                self.refresh_downloads_buffer().await;
                self.show_message(format!(
                    "Successfully deleted {} downloaded episodes and cleaned up downloads folder",
                    deleted_count
                ));
            }
            AppEvent::AllDownloadsDeletionFailed { error } => {
                // Still refresh buffers in case some deletions succeeded
                self.refresh_all_episode_buffers().await;
                self.refresh_downloads_buffer().await;
                self.show_error(format!("Failed to delete all downloads: {}", error));
            }
            AppEvent::OpmlImportStarted { source } => {
                self.show_message(format!("Starting OPML import from: {}...", source));
            }
            AppEvent::OpmlImportProgress { status, .. } => {
                self.show_message(status);
            }
            AppEvent::OpmlImportCompleted { result, log_path } => {
                // Refresh podcast list to show newly imported podcasts
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Err(e) = podcast_buffer.load_podcasts().await {
                        self.show_error(format!("Failed to refresh podcast list: {}", e));
                    }
                }

                // Build summary message
                let mut summary = format!(
                    "Import complete: {} imported, {} skipped",
                    result.imported, result.skipped
                );

                if result.has_failures() {
                    summary.push_str(&format!(", {} failed", result.failed.len()));
                    summary.push_str(&format!("\nSee log: {}", log_path));
                }

                self.show_message(summary);
            }
            AppEvent::OpmlImportFailed { source, error } => {
                self.show_error(format!("OPML import from {} failed: {}", source, error));
            }
            AppEvent::OpmlExportStarted { path } => {
                self.show_message(format!("Starting OPML export to: {}...", path));
            }
            AppEvent::OpmlExportProgress { status } => {
                self.show_message(status);
            }
            AppEvent::OpmlExportCompleted { path, feed_count } => {
                self.show_message(format!(
                    "Successfully exported {} feeds to {}",
                    feed_count, path
                ));
            }
            AppEvent::OpmlExportFailed { path, error } => {
                self.show_error(format!("OPML export to {} failed: {}", path, error));
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
                    let buffer_name = parts[1].to_string();
                    self.switch_to_buffer_by_name(buffer_name)
                } else {
                    self.show_buffer_list();
                    Ok(true)
                }
            }
            "switch-to-buffer" | "switch-buffer" => {
                if parts.len() > 1 {
                    let buffer_name = parts[1].to_string();
                    self.switch_to_buffer_by_name(buffer_name)
                } else {
                    self.prompt_buffer_switch();
                    Ok(true)
                }
            }
            "list-buffers" | "buffers" => {
                self.show_buffer_list();
                Ok(true)
            }
            "close-buffer" | "kill-buffer" => {
                if parts.len() > 1 {
                    let buffer_name = parts[1].to_string();
                    self.close_buffer_by_name(buffer_name)
                } else {
                    // Close current buffer
                    if let Some(current_id) = self.buffer_manager.current_buffer_id() {
                        match self.buffer_manager.remove_buffer(&current_id) {
                            Ok(_) => {
                                self.update_status_bar();
                                self.show_message(format!("Closed buffer: {}", current_id));
                                Ok(true)
                            }
                            Err(e) => {
                                self.show_error(format!("Cannot close buffer: {}", e));
                                Ok(true)
                            }
                        }
                    } else {
                        self.show_error("No buffer to close".to_string());
                        Ok(true)
                    }
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
            "delete-all-downloads" | "clean-downloads" => {
                // Show confirmation prompt for bulk deletion
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: "Delete ALL downloaded episodes? This will remove all downloaded files! (y/n) ".to_string(),
                    input: String::new(),
                });
                self.pending_bulk_deletion = true;
                Ok(true)
            }
            "import-opml" => {
                if parts.len() > 1 {
                    let source = parts[1..].join(" ");
                    self.show_message(format!("Importing OPML from: {}...", source));
                    self.trigger_async_opml_import(source);
                    Ok(true)
                } else {
                    // Prompt for file path/URL
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: "Import OPML from (file path or URL): ".to_string(),
                        input: String::new(),
                    });
                    Ok(true)
                }
            }
            "export-opml" => {
                if parts.len() > 1 {
                    let path = parts[1..].join(" ");
                    self.trigger_async_opml_export(path);
                    Ok(true)
                } else {
                    // Prompt for output path with default
                    let default_path = shellexpand::tilde(&self.config.storage.opml_export_directory).to_string();
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: format!("Export to (default: {}): ", default_path),
                        input: String::new(),
                    });
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
    #[allow(dead_code)]
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
        use crate::ui::buffers::buffer_list::BufferListBuffer;

        let buffer_names = self.buffer_manager.buffer_names();
        let current_id = self.buffer_manager.current_buffer_id();

        // Remove existing buffer list if it exists
        let buffer_list_id = "*Buffer List*".to_string();
        let _ = self.buffer_manager.remove_buffer(&buffer_list_id);

        // Create new buffer list buffer
        let mut buffer_list_buffer = BufferListBuffer::new();
        buffer_list_buffer.set_theme(self.theme.clone());
        buffer_list_buffer.update_buffer_list(buffer_names, current_id.as_ref());

        // Add and switch to buffer list buffer
        if let Ok(_) = self.buffer_manager.add_buffer(Box::new(buffer_list_buffer)) {
            let _ = self.buffer_manager.switch_to_buffer(&buffer_list_id);
        }

        self.update_status_bar();
    }

    /// Switch to buffer by name with smart matching
    fn switch_to_buffer_by_name(&mut self, buffer_name: String) -> UIResult<bool> {
        let buffer_names = self.buffer_manager.buffer_names();

        // Handle common aliases
        let normalized_name = match buffer_name.as_str() {
            "podcasts" | "podcast" | "main" => "podcast-list".to_string(),
            "help" => "*Help*".to_string(),
            "download" | "dl" => "downloads".to_string(),
            "new" | "whats-new" | "latest" => "whats-new".to_string(),
            _ => buffer_name.clone(),
        };

        // Try exact match first (by ID)
        if buffer_names.iter().any(|(id, _)| id == &normalized_name) {
            let _ = self.buffer_manager.switch_to_buffer(&normalized_name);
            self.update_status_bar();
            return Ok(true);
        }

        // Try exact match by display name
        if let Some((id, _)) = buffer_names
            .iter()
            .find(|(_, name)| name == &normalized_name)
        {
            let _ = self.buffer_manager.switch_to_buffer(id);
            self.update_status_bar();
            return Ok(true);
        }

        // Try partial match (case insensitive)
        let lower_search = normalized_name.to_lowercase();
        let matches: Vec<_> = buffer_names
            .iter()
            .filter(|(id, name)| {
                id.to_lowercase().contains(&lower_search)
                    || name.to_lowercase().contains(&lower_search)
            })
            .collect();

        match matches.len() {
            0 => {
                self.show_error(format!("No buffer found matching: {}", buffer_name));
                Ok(true)
            }
            1 => {
                let (id, _) = matches[0];
                let _ = self.buffer_manager.switch_to_buffer(id);
                self.update_status_bar();
                Ok(true)
            }
            _ => {
                let mut msg = format!("Multiple buffers match '{}':\n", buffer_name);
                for (id, name) in matches {
                    if id != name {
                        msg.push_str(&format!("  {} ({})\n", name, id));
                    } else {
                        msg.push_str(&format!("  {}\n", name));
                    }
                }
                self.show_message(msg);
                Ok(true)
            }
        }
    }

    /// Get all available commands for auto-completion
    fn get_available_commands(&self) -> Vec<String> {
        vec![
            // Core commands
            "quit".to_string(),
            "q".to_string(),
            "help".to_string(),
            "h".to_string(),
            // Theme commands
            "theme".to_string(),
            "theme dark".to_string(),
            "theme light".to_string(),
            "theme high-contrast".to_string(),
            "theme solarized".to_string(),
            // Buffer commands
            "buffer".to_string(),
            "b".to_string(),
            "switch-to-buffer".to_string(),
            "switch-buffer".to_string(),
            "list-buffers".to_string(),
            "buffers".to_string(),
            "close-buffer".to_string(),
            "kill-buffer".to_string(),
            // Podcast commands
            "add-podcast".to_string(),
            // Downloads commands
            "delete-all-downloads".to_string(),
            "clean-downloads".to_string(),
            // OPML commands
            "import-opml".to_string(),
            "export-opml".to_string(),
        ]
    }

    /// Update completion mode based on the current input context
    fn update_completion_mode_for_input(&mut self, input: &str) {
        let input_lower = input.to_lowercase();

        // Check if we're typing a buffer-related command that needs buffer name completion
        if input_lower.starts_with("switch-to-buffer ")
            || input_lower.starts_with("switch-buffer ")
            || input_lower.starts_with("buffer ")
            || input_lower.starts_with("b ")
            || input_lower.starts_with("close-buffer ")
            || input_lower.starts_with("kill-buffer ")
        {
            // Get the command part and the buffer name part
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() >= 1 {
                let command_part = parts[0];

                // Get available buffer names (just the names)
                let buffer_names = self.buffer_manager.buffer_completion_names();
                let mut buffer_completions = Vec::new();

                // If there's already a space, we're completing buffer names
                if input.contains(' ') {
                    let buffer_search = if parts.len() > 1 {
                        parts[1..].join(" ").to_lowercase()
                    } else {
                        String::new()
                    };

                    for name in &buffer_names {
                        if buffer_search.is_empty()
                            || name.to_lowercase().starts_with(&buffer_search)
                        {
                            buffer_completions.push(format!("{} {}", command_part, name));
                        }
                    }
                } else {
                    // Still typing the command, offer command completion + buffer suggestions
                    buffer_completions.push(format!("{} ", command_part));
                    for name in &buffer_names {
                        buffer_completions.push(format!("{} {}", command_part, name));
                    }
                }

                // Sort the completions
                buffer_completions.sort();

                // Update the minibuffer with buffer name completions
                if !buffer_completions.is_empty() {
                    // Update the completion candidates to be buffer names instead of all commands
                    self.minibuffer
                        .set_completion_candidates(buffer_completions.clone());

                    // Convert to PromptWithCompletion mode if not already
                    if let Some(current_input) = self.minibuffer.current_input() {
                        let cursor_pos = current_input.len();
                        self.minibuffer
                            .set_content(MinibufferContent::PromptWithCompletion {
                                prompt: "M-x ".to_string(),
                                input: current_input,
                                cursor_pos,
                                completions: buffer_completions,
                                completion_index: None,
                            });
                    }
                }
            }
        }
    }

    /// Get contextual command completions based on current input
    fn get_contextual_command_completions(&self, input: &str) -> Vec<String> {
        let input_lower = input.to_lowercase();
        let mut completions = Vec::new();

        // Basic command completion - match from start
        let base_commands = self.get_available_commands();
        for cmd in &base_commands {
            if cmd.to_lowercase().starts_with(&input_lower) {
                completions.push(cmd.clone());
            }
        }

        // Add contextual completions for specific command patterns
        if input_lower.starts_with("theme ") {
            let theme_part = &input_lower[6..]; // Skip "theme "
            let themes = ["dark", "light", "high-contrast", "solarized"];
            for theme in &themes {
                if theme.starts_with(theme_part) {
                    completions.push(format!("theme {}", theme));
                }
            }
        } else if input_lower.starts_with("buffer ") || input_lower.starts_with("b ") {
            // Add buffer names as completions
            let buffer_names = self.buffer_manager.buffer_names();
            let prefix = if input_lower.starts_with("buffer ") {
                "buffer "
            } else {
                "b "
            };
            let search_term = &input_lower[prefix.len()..];

            for (_, name) in &buffer_names {
                if name.to_lowercase().starts_with(search_term) {
                    completions.push(format!("{}{}", prefix, name));
                }
            }
        } else if input_lower.starts_with("switch-to-buffer ")
            || input_lower.starts_with("switch-buffer ")
        {
            // Add buffer names as completions for switch commands
            let buffer_names = self.buffer_manager.buffer_names();
            let prefix = if input_lower.starts_with("switch-to-buffer ") {
                "switch-to-buffer "
            } else {
                "switch-buffer "
            };
            let search_term = &input_lower[prefix.len()..];

            for (_, name) in &buffer_names {
                if name.to_lowercase().starts_with(search_term) {
                    completions.push(format!("{}{}", prefix, name));
                }
            }
        } else if input_lower.starts_with("close-buffer ")
            || input_lower.starts_with("kill-buffer ")
        {
            // Add buffer names as completions for close commands
            let buffer_names = self.buffer_manager.buffer_names();
            let prefix = if input_lower.starts_with("close-buffer ") {
                "close-buffer "
            } else {
                "kill-buffer "
            };
            let search_term = &input_lower[prefix.len()..];

            for (_, name) in &buffer_names {
                if name.to_lowercase().starts_with(search_term) {
                    completions.push(format!("{}{}", prefix, name));
                }
            }
        } else if input_lower.starts_with("add-podcast ") {
            // For add-podcast, we could suggest common podcast URLs or show a hint
            if input_lower == "add-podcast " {
                completions.push("add-podcast https://".to_string());
            }
        }

        // Remove duplicates and sort
        completions.sort();
        completions.dedup();

        // If no specific matches, fall back to all commands that contain the input
        if completions.is_empty() && !input.is_empty() {
            for cmd in &base_commands {
                if cmd.to_lowercase().contains(&input_lower) {
                    completions.push(cmd.clone());
                }
            }
        }

        completions
    }

    /// Show command prompt with auto-completion
    fn show_command_prompt_with_completion(&mut self) {
        let commands = self.get_available_commands();
        let prompt = "M-x ".to_string();
        self.minibuffer
            .show_prompt_with_completion(prompt, commands);
    }

    /// Prompt for buffer switch with completion hints
    fn prompt_buffer_switch(&mut self) {
        let buffer_names = self.buffer_manager.buffer_names();

        // Create completion candidates from buffer names
        let completions: Vec<String> = buffer_names
            .iter()
            .map(|(id, name)| {
                // Prefer display name, but include ID as fallback
                if id != name {
                    name.clone()
                } else {
                    id.clone()
                }
            })
            .collect();

        let prompt = "Switch to buffer: ".to_string();
        self.minibuffer
            .show_prompt_with_completion(prompt, completions);
    }

    /// Close buffer by name with smart matching
    fn close_buffer_by_name(&mut self, buffer_name: String) -> UIResult<bool> {
        let buffer_names = self.buffer_manager.buffer_names();

        // Try exact match first (by ID)
        if buffer_names.iter().any(|(id, _)| id == &buffer_name) {
            match self.buffer_manager.remove_buffer(&buffer_name) {
                Ok(_) => {
                    self.update_status_bar();
                    self.show_message(format!("Closed buffer: {}", buffer_name));
                    return Ok(true);
                }
                Err(e) => {
                    self.show_error(format!("Cannot close buffer: {}", e));
                    return Ok(true);
                }
            }
        }

        // Try exact match by display name
        if let Some((id, _)) = buffer_names.iter().find(|(_, name)| name == &buffer_name) {
            match self.buffer_manager.remove_buffer(id) {
                Ok(_) => {
                    self.update_status_bar();
                    self.show_message(format!("Closed buffer: {}", buffer_name));
                    return Ok(true);
                }
                Err(e) => {
                    self.show_error(format!("Cannot close buffer: {}", e));
                    return Ok(true);
                }
            }
        }

        // Try partial match (case insensitive)
        let lower_search = buffer_name.to_lowercase();
        let matches: Vec<_> = buffer_names
            .iter()
            .filter(|(id, name)| {
                id.to_lowercase().contains(&lower_search)
                    || name.to_lowercase().contains(&lower_search)
            })
            .collect();

        match matches.len() {
            0 => {
                self.show_error(format!("No buffer found matching: {}", buffer_name));
                Ok(true)
            }
            1 => {
                let (id, _) = matches[0];
                match self.buffer_manager.remove_buffer(id) {
                    Ok(_) => {
                        self.update_status_bar();
                        self.show_message(format!("Closed buffer: {}", buffer_name));
                        Ok(true)
                    }
                    Err(e) => {
                        self.show_error(format!("Cannot close buffer: {}", e));
                        Ok(true)
                    }
                }
            }
            _ => {
                let mut msg = format!("Multiple buffers match '{}':\n", buffer_name);
                for (id, name) in matches {
                    if id != name {
                        msg.push_str(&format!("  {} ({})\n", name, id));
                    } else {
                        msg.push_str(&format!("  {}\n", name));
                    }
                }
                msg.push_str("\nSpecify the exact buffer name to close.");
                self.show_message(msg);
                Ok(true)
            }
        }
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

    /// Trigger async single podcast hard refresh (re-parses all episodes)
    fn trigger_async_hard_refresh_single(&mut self, podcast_id: crate::storage::PodcastId) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();

        tokio::spawn(async move {
            match subscription_manager
                .refresh_feed_with_options(&podcast_id, true)
                .await
            {
                Ok(updated_episodes) => {
                    let _ = app_event_tx.send(AppEvent::PodcastRefreshed {
                        podcast_id: podcast_id_clone,
                        new_episode_count: updated_episodes.len(),
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
                Err(_e) => {
                    // For all refresh, we'll just show a general error
                    // TODO: Add proper error reporting mechanism for TUI
                }
            }
        });
    }

    /// Trigger async episode download
    fn trigger_async_download(
        &mut self,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    ) {
        let download_manager = self.download_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let episode_id_clone = episode_id.clone();

        tokio::spawn(async move {
            match download_manager
                .download_episode(&podcast_id, &episode_id)
                .await
            {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeDownloaded {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeDownloadFailed {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async episode download deletion
    fn trigger_async_delete_download(
        &mut self,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    ) {
        let download_manager = self.download_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let episode_id_clone = episode_id.clone();

        tokio::spawn(async move {
            match download_manager
                .delete_episode(&podcast_id, &episode_id)
                .await
            {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeDownloadDeleted {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeDownloadDeletionFailed {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async downloads refresh
    fn trigger_async_refresh_downloads(&mut self) {
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            // Send a custom event to trigger refresh on the UI thread
            let _ = app_event_tx.send(AppEvent::DownloadsRefreshed);
        });
    }

    /// Trigger async deletion of all downloads
    fn trigger_async_delete_all_downloads(&mut self) {
        let download_manager = self.download_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        // Show immediate feedback
        self.show_message("Deleting all downloaded episodes...".to_string());

        tokio::spawn(async move {
            match download_manager.delete_all_downloads().await {
                Ok(deleted_count) => {
                    let _ = app_event_tx.send(AppEvent::AllDownloadsDeleted { deleted_count });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::AllDownloadsDeletionFailed {
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Refresh episode buffers for a specific podcast
    async fn refresh_episode_buffers(&mut self, podcast_id: &crate::storage::PodcastId) {
        // Find any episode buffers that belong to this podcast and refresh them
        let buffer_ids = self.buffer_manager.get_buffer_ids();
        for buffer_id in buffer_ids {
            if buffer_id.starts_with("episodes-") {
                if let Some(episode_buffer) = self
                    .buffer_manager
                    .get_episode_list_buffer_mut_by_id(&buffer_id)
                {
                    // Check if this buffer belongs to the podcast
                    if &episode_buffer.podcast_id == podcast_id {
                        let _ = episode_buffer.load_episodes().await;
                    }
                }
            }
        }
    }

    /// Refresh downloads buffer
    async fn refresh_downloads_buffer(&mut self) {
        // Find the downloads buffer and refresh it
        let buffer_ids = self.buffer_manager.get_buffer_ids();
        for buffer_id in buffer_ids {
            if buffer_id == "downloads" {
                if let Some(downloads_buffer) = self.buffer_manager.get_downloads_buffer_mut() {
                    let _ = downloads_buffer.refresh_downloads().await;
                }
                break;
            }
        }
    }

    /// Refresh What's New buffer
    async fn refresh_whats_new_buffer(&mut self) {
        // Find the What's New buffer and refresh it
        let buffer_ids = self.buffer_manager.get_buffer_ids();
        for buffer_id in buffer_ids {
            if buffer_id == "whats-new" {
                if let Some(whats_new_buffer) = self.buffer_manager.get_whats_new_buffer_mut() {
                    let _ = whats_new_buffer.load_episodes().await;
                }
                break;
            }
        }
    }

    /// Refresh all episode buffers (used after bulk operations)
    async fn refresh_all_episode_buffers(&mut self) {
        let buffer_ids = self.buffer_manager.get_buffer_ids();
        for buffer_id in buffer_ids {
            if buffer_id.starts_with("episodes-") {
                if let Some(episode_buffer) = self
                    .buffer_manager
                    .get_episode_list_buffer_mut_by_id(&buffer_id)
                {
                    let _ = episode_buffer.load_episodes().await;
                }
            }
        }
    }

    /// Refresh all buffer list buffers when buffers change
    #[allow(dead_code)]
    fn refresh_buffer_lists(&mut self) {
        let _buffer_names = self.buffer_manager.buffer_names();
        let _current_buffer = self.buffer_manager.current_buffer_id();

        // Find and update any buffer list buffers
        let buffer_ids = self.buffer_manager.get_buffer_ids();
        for buffer_id in buffer_ids {
            if buffer_id == "*Buffer List*" {
                if let Some(_buffer) = self.buffer_manager.get_buffer(&buffer_id) {
                    // We need to downcast to update the buffer list
                    // For now, we'll handle this by recreating the buffer when needed
                }
                break;
            }
        }
    }

    /// Refresh buffer list if one is currently open
    fn refresh_buffer_list_if_open(&mut self) {
        let buffer_list_id = "*Buffer List*".to_string();
        if self
            .buffer_manager
            .get_buffer_ids()
            .contains(&buffer_list_id)
        {
            // Only refresh the buffer list contents without switching to it
            use crate::ui::buffers::buffer_list::BufferListBuffer;

            let buffer_names = self.buffer_manager.buffer_names();
            let current_id = self.buffer_manager.current_buffer_id();

            // Remove existing buffer list
            let _ = self.buffer_manager.remove_buffer(&buffer_list_id);

            // Create new buffer list buffer with updated data
            let mut buffer_list_buffer = BufferListBuffer::new();
            buffer_list_buffer.set_theme(self.theme.clone());
            buffer_list_buffer.update_buffer_list(buffer_names, current_id.as_ref());

            // Add back without switching to it
            let _ = self.buffer_manager.add_buffer(Box::new(buffer_list_buffer));
        }
    }

    /// Trigger async podcast deletion
    fn trigger_async_delete_podcast(&mut self, podcast_id: crate::storage::PodcastId) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();

        // Get podcast title for the event
        let podcast_title =
            if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                podcast_buffer
                    .selected_podcast()
                    .map(|p| p.title.clone())
                    .unwrap_or_else(|| "Unknown".to_string())
            } else {
                "Unknown".to_string()
            };

        tokio::spawn(async move {
            match subscription_manager.unsubscribe(&podcast_id).await {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::PodcastDeleted {
                        podcast_id: podcast_id_clone,
                        podcast_title,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PodcastDeletionFailed {
                        podcast_id: podcast_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });

        // Show immediate feedback
        self.show_message("Deleting podcast...".to_string());
    }

    /// Trigger async OPML import
    fn trigger_async_opml_import(&mut self, source: String) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let source_clone = source.clone();

        // Send start event
        let _ = app_event_tx.send(AppEvent::OpmlImportStarted {
            source: source.clone(),
        });

        tokio::spawn(async move {
            // Create progress callback
            let app_event_tx_progress = app_event_tx.clone();
            let progress_callback = move |status: String| {
                let _ = app_event_tx_progress.send(AppEvent::OpmlImportProgress {
                    current: 0,
                    total: 0,
                    status,
                });
            };

            match subscription_manager
                .import_opml(&source, progress_callback)
                .await
            {
                Ok((result, log_path)) => {
                    let _ = app_event_tx.send(AppEvent::OpmlImportCompleted { result, log_path });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::OpmlImportFailed {
                        source: source_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async OPML export
    fn trigger_async_opml_export(&mut self, output_path: String) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let output_path_clone = output_path.clone();

        // Expand tilde and generate filename if needed
        let expanded_path = shellexpand::tilde(&output_path).to_string();
        let final_path = if std::path::Path::new(&expanded_path).is_dir()
            || !expanded_path.contains('.')
        {
            // It's a directory or doesn't have an extension, generate filename
            use chrono::Local;
            let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
            let filename = format!("podcasts-export-{}.opml", timestamp);
            std::path::PathBuf::from(expanded_path).join(filename)
        } else {
            // It's a full file path
            std::path::PathBuf::from(expanded_path)
        };

        let final_path_str = final_path.to_string_lossy().to_string();

        // Send start event
        let _ = app_event_tx.send(AppEvent::OpmlExportStarted {
            path: final_path_str.clone(),
        });

        tokio::spawn(async move {
            // Create progress callback
            let app_event_tx_progress = app_event_tx.clone();
            let progress_callback = move |status: String| {
                let _ = app_event_tx_progress.send(AppEvent::OpmlExportProgress { status });
            };

            match subscription_manager
                .export_opml(&final_path, progress_callback)
                .await
            {
                Ok(feed_count) => {
                    let _ = app_event_tx.send(AppEvent::OpmlExportCompleted {
                        path: final_path_str,
                        feed_count,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::OpmlExportFailed {
                        path: output_path_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async episode loading
    fn trigger_async_load_episodes(
        &mut self,
        podcast_id: crate::storage::PodcastId,
        podcast_name: String,
    ) {
        let subscription_manager = self.subscription_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let podcast_name_clone = podcast_name.clone();

        tokio::spawn(async move {
            match subscription_manager
                .storage
                .load_episodes(&podcast_id)
                .await
            {
                Ok(episodes) => {
                    let _ = app_event_tx.send(AppEvent::EpisodesLoaded {
                        podcast_id: podcast_id_clone,
                        podcast_name: podcast_name_clone,
                        episodes,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodesLoadFailed {
                        podcast_id: podcast_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Handle minibuffer input submission with context
    fn handle_minibuffer_input_with_context(&mut self, input: String, prompt_context: Option<String>) {
        let input = input.trim();

        if input.is_empty() {
            // Handle empty input for specific prompts
            if let Some(prompt) = &prompt_context {
                if prompt.starts_with("Export to") {
                    // Empty input means use default export path
                    let default_path = shellexpand::tilde(&self.config.storage.opml_export_directory).to_string();
                    self.trigger_async_opml_export(default_path);
                    return;
                }
            }
            return;
        }

        // Check context from prompt FIRST (before checking for URLs)
        if let Some(prompt) = &prompt_context {
            if prompt.starts_with("Import OPML from") {
                // This is an OPML import
                self.trigger_async_opml_import(input.to_string());
                return;
            } else if prompt.starts_with("Export to") {
                // This is an OPML export
                self.trigger_async_opml_export(input.to_string());
                return;
            }
        }

        // If no prompt context, fall back to old behavior
        self.handle_minibuffer_input_legacy(input.to_string());
    }

    /// Handle minibuffer input submission (legacy method for backward compatibility)
    fn handle_minibuffer_input(&mut self, input: String) {
        self.handle_minibuffer_input_legacy(input);
    }

    /// Legacy minibuffer input handler
    fn handle_minibuffer_input_legacy(&mut self, input: String) {
        let input = input.trim();

        // Check if this looks like a URL (basic heuristic for podcast addition)
        if input.starts_with("http://") || input.starts_with("https://") {
            self.show_message(format!("Adding podcast: {}...", input));
            self.trigger_async_add_podcast(input.to_string());
        } else if input.to_lowercase() == "y" || input.to_lowercase() == "yes" {
            // Handle podcast deletion confirmation
            if let Some(podcast_id) = self.pending_deletion.take() {
                self.trigger_async_delete_podcast(podcast_id);
            } else if self.pending_bulk_deletion {
                // Handle bulk deletion confirmation
                self.pending_bulk_deletion = false;
                self.trigger_async_delete_all_downloads();
            } else {
                self.show_error("No deletion pending".to_string());
            }
        } else if input.to_lowercase() == "n" || input.to_lowercase() == "no" {
            // Cancel deletion
            if self.pending_deletion.is_some() {
                self.pending_deletion = None;
                self.show_message("Podcast deletion cancelled".to_string());
            } else if self.pending_bulk_deletion {
                self.pending_bulk_deletion = false;
                self.show_message("Bulk deletion cancelled".to_string());
            } else {
                self.show_message("No deletion to cancel".to_string());
            }
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
                // Get the prompt BEFORE submit() clears it
                let prompt = self.minibuffer.current_prompt();
                if let Some(input) = self.minibuffer.submit() {
                    self.handle_minibuffer_input_with_context(input, prompt);
                }
                Ok(true)
            }
            // Tab completion
            (KeyCode::Tab, _) => {
                self.minibuffer.tab_complete();
                Ok(true)
            }
            // Cancel on Ctrl+G or Escape (also clear any pending operations)
            (KeyCode::Esc, _) | (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.minibuffer.clear();
                self.pending_deletion = None;
                self.pending_bulk_deletion = false;
                Ok(true)
            }
            // Backspace
            (KeyCode::Backspace, _) => {
                self.minibuffer.backspace();

                // Update command completion dynamically if in command prompt mode
                if self.minibuffer.is_command_prompt() {
                    if let Some(current_input) = self.minibuffer.current_input() {
                        let commands = self.get_contextual_command_completions(&current_input);
                        self.minibuffer.set_completion_candidates(commands);

                        // If this is a buffer-related command, update to buffer name completion mode
                        self.update_completion_mode_for_input(&current_input);
                    }
                }

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

                // Update command completion dynamically if in command prompt mode
                if self.minibuffer.is_command_prompt() {
                    if let Some(current_input) = self.minibuffer.current_input() {
                        let commands = self.get_contextual_command_completions(&current_input);
                        self.minibuffer.set_completion_candidates(commands);

                        // If this is a buffer-related command, update to buffer name completion mode
                        self.update_completion_mode_for_input(&current_input);
                    }
                }

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
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config {
            ui: UiConfig::default(),
            ..Default::default()
        };

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_dir.path().to_path_buf(),
                DownloadConfig::default(),
            )
            .unwrap(),
        );
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let app = UIApp::new(
            config,
            subscription_manager,
            download_manager.clone(),
            storage.clone(),
            app_event_tx,
        );
        assert!(app.is_ok());

        let app = app.unwrap();
        assert!(!app.should_quit);
        assert_eq!(app.frame_count, 0);
    }

    #[tokio::test]
    async fn test_quit_action() {
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_dir.path().to_path_buf(),
                DownloadConfig::default(),
            )
            .unwrap(),
        );
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app = UIApp::new(
            config,
            subscription_manager,
            download_manager,
            storage,
            app_event_tx,
        )
        .unwrap();

        let result = app.handle_action(UIAction::Quit).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false to indicate stopping
        assert!(app.should_quit);
    }

    #[tokio::test]
    async fn test_show_help_action() {
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_dir.path().to_path_buf(),
                DownloadConfig::default(),
            )
            .unwrap(),
        );
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_dir.path().to_path_buf(),
                DownloadConfig::default(),
            )
            .unwrap(),
        );

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app = UIApp::new(
            config,
            subscription_manager,
            download_manager,
            storage,
            app_event_tx,
        )
        .unwrap();
        app.initialize().await.unwrap();

        let result = app.handle_action(UIAction::ShowHelp).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Should have switched to help buffer
        assert_eq!(app.buffer_manager.current_buffer_name().unwrap(), "*Help*");
    }

    #[tokio::test]
    async fn test_command_execution() {
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_dir.path().to_path_buf(),
                DownloadConfig::default(),
            )
            .unwrap(),
        );
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app = UIApp::new(
            config,
            subscription_manager,
            download_manager,
            storage,
            app_event_tx,
        )
        .unwrap();
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
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();

        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_dir.path().to_path_buf(),
                DownloadConfig::default(),
            )
            .unwrap(),
        );
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));

        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();
        let mut app = UIApp::new(
            config,
            subscription_manager,
            download_manager,
            storage,
            app_event_tx,
        )
        .unwrap();

        let result = app.set_theme_direct("light");
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result = app.set_theme_direct("invalid-theme");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
