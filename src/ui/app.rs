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
use futures_util::stream::{self, StreamExt};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tokio::sync::mpsc;

use crate::{
    audio::{AudioCommand, PlaybackStatus},
    config::Config,
    constants::ui as ui_constants,
    download::DownloadManager,
    playlist::{auto_generator::TodayGenerator, manager::PlaylistManager},
    podcast::subscription::SubscriptionManager,
    storage::{JsonStorage, Storage},
    ui::{
        buffers::BufferManager,
        components::{minibuffer::Minibuffer, minibuffer::MinibufferContent, statusbar::StatusBar},
        events::{
            AggregatedEpisode, AppEvent, BufferRefreshData, BufferRefreshType, DownloadEntry,
            UIEvent, UIEventHandler,
        },
        keybindings::KeyHandler,
        theme_loader::ThemeRegistry,
        themes::Theme,
        UIAction, UIComponent, UIError, UIResult,
    },
};
use directories::ProjectDirs;
use std::sync::Arc;

/// The main UI application
pub struct UIApp {
    /// Configuration
    config: Config,

    /// Current theme
    theme: Theme,

    /// Theme registry (bundled + user themes loaded from filesystem)
    theme_registry: ThemeRegistry,

    /// Subscription manager
    subscription_manager: Arc<SubscriptionManager<JsonStorage>>,

    /// Download manager
    download_manager: Arc<DownloadManager<JsonStorage>>,

    /// Playlist manager
    playlist_manager: Arc<PlaylistManager>,

    /// Auto-generated "Today" playlist manager
    today_generator: Arc<TodayGenerator>,

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

    /// Playlist ID pending deletion confirmation
    pending_playlist_deletion: Option<crate::playlist::PlaylistId>,

    /// Whether we're pending bulk deletion confirmation
    pending_bulk_deletion: bool,

    /// Pending cleanup duration in hours (set when user confirms age-based cleanup)
    pending_cleanup_hours: Option<u64>,

    /// Sender for dispatching audio playback commands (None when audio init failed).
    audio_command_tx: Option<mpsc::UnboundedSender<AudioCommand>>,

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
        let playlists_dir = storage.data_dir.join("Playlists");
        let playlist_manager = Arc::new(PlaylistManager::new(
            storage.clone(),
            download_manager.clone(),
            playlists_dir.clone(),
        ));
        let today_generator = Arc::new(TodayGenerator::new(
            storage.clone(),
            download_manager.clone(),
            playlists_dir,
        ));

        let mut theme_registry = ThemeRegistry::new();
        if let Some(project_dirs) = ProjectDirs::from("", "", "podcast-tui") {
            for err in theme_registry.load_user_themes(project_dirs.config_dir()) {
                eprintln!("[themes] Warning: {err}");
            }
        }
        let theme = theme_registry
            .get(&config.ui.theme)
            .cloned()
            .ok_or_else(|| {
                UIError::InvalidOperation(format!("Unknown theme: {}", config.ui.theme))
            })?;
        let buffer_manager = BufferManager::new();
        let mut status_bar = StatusBar::new();
        status_bar.set_theme(theme.clone());

        let minibuffer = Minibuffer::new();
        let key_handler = KeyHandler::from_config(&config.keybindings);

        // Validate keybindings: warn on conflicts, error on unbound critical actions.
        let validation = key_handler.validate();
        for warning in &validation.warnings {
            eprintln!("[keybindings] {warning}");
        }
        if let Some(unbound) = validation
            .unbound_actions
            .iter()
            .find(|u| u.action == UIAction::Quit)
        {
            return Err(UIError::Keybinding(format!(
                "Critical action unbound: {:?} has no key assigned (default was '{}'). \
                 Add a 'quit' entry to your keybindings config.",
                unbound.action, unbound.default_key
            )));
        }

        let event_handler =
            UIEventHandler::new(Duration::from_millis(ui_constants::UI_TICK_RATE_MS));

        Ok(Self {
            config,
            theme,
            theme_registry,
            subscription_manager,
            download_manager,
            playlist_manager,
            today_generator,
            _storage: storage,
            buffer_manager,
            status_bar,
            minibuffer,
            key_handler,
            event_handler,
            app_event_tx,
            should_quit: false,
            audio_command_tx: None,
            last_render: Instant::now(),
            frame_count: 0,
            pending_deletion: None,
            pending_playlist_deletion: None,
            pending_bulk_deletion: false,
            pending_cleanup_hours: None,
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
        let playlists_dir = storage.data_dir.join("Playlists");
        let playlist_manager = Arc::new(PlaylistManager::new(
            storage.clone(),
            download_manager.clone(),
            playlists_dir.clone(),
        ));
        let today_generator = Arc::new(TodayGenerator::new(
            storage.clone(),
            download_manager.clone(),
            playlists_dir,
        ));

        let mut theme_registry = ThemeRegistry::new();
        if let Some(project_dirs) = ProjectDirs::from("", "", "podcast-tui") {
            for err in theme_registry.load_user_themes(project_dirs.config_dir()) {
                eprintln!("[themes] Warning: {err}");
            }
        }
        let theme = theme_registry
            .get(&config.ui.theme)
            .cloned()
            .ok_or_else(|| {
                UIError::InvalidOperation(format!("Unknown theme: {}", config.ui.theme))
            })?;
        let mut buffer_manager = BufferManager::new();
        let mut status_bar = StatusBar::new();
        status_bar.set_theme(theme.clone());

        let minibuffer = Minibuffer::new();
        let key_handler = KeyHandler::from_config(&config.keybindings);

        // Validate keybindings: warn on conflicts, error on unbound critical actions.
        let validation = key_handler.validate();
        for warning in &validation.warnings {
            eprintln!("[keybindings] {warning}");
        }
        if let Some(unbound) = validation
            .unbound_actions
            .iter()
            .find(|u| u.action == UIAction::Quit)
        {
            return Err(UIError::Keybinding(format!(
                "Critical action unbound: {:?} has no key assigned (default was '{}'). \
                 Add a 'quit' entry to your keybindings config.",
                unbound.action, unbound.default_key
            )));
        }

        let event_handler =
            UIEventHandler::new(Duration::from_millis(ui_constants::UI_TICK_RATE_MS));

        // Create buffers with progress updates
        status_tx.send(crate::InitStatus::CreatingBuffers).ok();
        buffer_manager.create_help_buffer(key_handler.generate_help_text());
        buffer_manager.create_podcast_list_buffer(subscription_manager.clone());
        buffer_manager.create_downloads_buffer(download_manager.clone(), storage.clone());
        buffer_manager.create_sync_buffer(download_manager.clone(), storage.data_dir.clone());
        buffer_manager.create_playlist_list_buffer(playlist_manager.clone());
        buffer_manager.create_whats_new_buffer(
            subscription_manager.clone(),
            download_manager.clone(),
            config.ui.whats_new_episode_limit,
        );
        buffer_manager.create_now_playing_buffer();

        // Set initial buffer
        if let Some(buffer_id) = buffer_manager.get_buffer_ids().first() {
            let _ = buffer_manager.switch_to_buffer(&buffer_id.clone());
        }

        // Skip loading data during initialization - it will be loaded asynchronously in the background
        // after the UI is displayed to provide instant startup
        status_tx.send(crate::InitStatus::Complete).ok();

        Ok(Self {
            config,
            theme,
            theme_registry,
            subscription_manager,
            download_manager,
            playlist_manager,
            today_generator,
            _storage: storage,
            buffer_manager,
            status_bar,
            minibuffer,
            key_handler,
            event_handler,
            app_event_tx,
            should_quit: false,
            audio_command_tx: None,
            last_render: Instant::now(),
            frame_count: 0,
            pending_deletion: None,
            pending_playlist_deletion: None,
            pending_bulk_deletion: false,
            pending_cleanup_hours: None,
        })
    }

    /// Wire the audio command sender into the app.
    ///
    /// Called from `App::run()` after `AudioManager` is initialised. All
    /// playback `UIAction` handlers check this before sending commands.
    pub fn set_audio_command_tx(&mut self, tx: mpsc::UnboundedSender<AudioCommand>) {
        self.audio_command_tx = Some(tx);
    }

    /// Replace the app event sender after construction (used when wiring AudioManager).
    pub fn set_app_event_tx(&mut self, tx: mpsc::UnboundedSender<AppEvent>) {
        self.app_event_tx = tx;
    }

    /// Run the UI application
    pub async fn run(
        &mut self,
        mut app_event_rx: mpsc::UnboundedReceiver<AppEvent>,
        mut playback_status_rx: Option<tokio::sync::watch::Receiver<PlaybackStatus>>,
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

            // Trigger background loading of buffer data (non-blocking)
            self.trigger_background_refresh(crate::ui::events::BufferRefreshType::PodcastList);
            self.trigger_background_refresh(crate::ui::events::BufferRefreshType::Downloads);
            self.trigger_background_refresh(crate::ui::events::BufferRefreshType::WhatsNew);
            self.trigger_async_refresh_today();

            // Defer download cleanup to background — don't block the first render
            let dm = self.download_manager.clone();
            let cleanup_days = self.config.downloads.cleanup_after_days;
            let app_event_tx = self.app_event_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = dm.cleanup_stuck_downloads().await {
                    let _ = app_event_tx.send(AppEvent::DownloadCleanupFailed {
                        error: format!("Stuck download cleanup failed: {e}"),
                    });
                }
                if let Some(days) = cleanup_days {
                    if days > 0 {
                        match dm.cleanup_old_downloads(days).await {
                            Ok(0) => {}
                            Ok(count) => {
                                let _ = app_event_tx.send(AppEvent::DownloadCleanupCompleted {
                                    deleted_count: count,
                                    duration_label: format!("{days} days"),
                                });
                            }
                            Err(e) => {
                                let _ = app_event_tx.send(AppEvent::DownloadCleanupFailed {
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                }
            });
        }

        // Wire the AudioManager status receiver into the NowPlaying buffer so it
        // receives live playback state updates (~4 Hz from the audio thread).
        if let Some(rx) = playback_status_rx.as_ref() {
            self.buffer_manager.set_now_playing_status_rx(rx.clone());
        }

        // Perform initial render to display UI immediately (before event loop)
        terminal
            .draw(|f| self.render(f))
            .map_err(|e| UIError::Render(e.to_string()))?;

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
                                    self.show_error(format!("Could not process input event: {}", e));
                                }
                            }
                        }
                        None => break Ok(()), // Channel closed
                    }
                }
                // Handle incoming app events (from async tasks)
                app_event = app_event_rx.recv() => {
                    if let Some(event) = app_event {
                        if let Err(e) = self.handle_app_event(event).await {
                            self.show_error(format!(
                                "Could not process background event: {}",
                                e
                            ));
                        }
                    }
                }
                // Playback status changed — NowPlaying buffer reads from its own
                // watch::Receiver in render(); this branch just triggers a re-render.
                _ = async {
                    match playback_status_rx.as_mut() {
                        Some(rx) => { let _ = rx.changed().await; }
                        None => std::future::pending::<()>().await,
                    }
                } => {
                    // Status updated; fall through to render below.
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
            self.show_error(format!("Could not clean up stuck downloads: {}", e));
        }

        // Auto-cleanup old downloads on startup if configured
        if let Some(days) = self.config.downloads.cleanup_after_days {
            if days > 0 {
                match self.download_manager.cleanup_old_downloads(days).await {
                    Ok(0) => {} // Nothing to clean, stay silent
                    Ok(count) => {
                        self.show_message(format!(
                            "Auto-cleanup: deleted {} episode(s) older than {} days",
                            count, days
                        ));
                    }
                    Err(e) => {
                        self.show_error(format!("Could not complete auto-cleanup: {}", e));
                    }
                }
            }
        }

        // Create initial buffers
        self.buffer_manager
            .create_help_buffer(self.key_handler.generate_help_text());
        self.buffer_manager
            .create_podcast_list_buffer(self.subscription_manager.clone());
        self.buffer_manager.create_downloads_buffer(
            self.download_manager.clone(),
            self.download_manager.storage().clone(),
        );
        self.buffer_manager.create_sync_buffer(
            self.download_manager.clone(),
            self._storage.data_dir.clone(),
        );
        self.buffer_manager.create_whats_new_buffer(
            self.subscription_manager.clone(),
            self.download_manager.clone(),
            self.config.ui.whats_new_episode_limit,
        );
        self.buffer_manager
            .create_playlist_list_buffer(self.playlist_manager.clone());
        self.buffer_manager.create_now_playing_buffer();

        // Set initial buffer
        if let Some(buffer_id) = self.buffer_manager.get_buffer_ids().first() {
            let _ = self.buffer_manager.switch_to_buffer(&buffer_id.clone());
        }

        // Update status bar
        self.update_status_bar();

        // Show welcome message
        self.show_message("Welcome to Podcast TUI! Press F1 or ? for help.".to_string());

        // Trigger background loading of buffer data (non-blocking)
        self.trigger_background_refresh(crate::ui::events::BufferRefreshType::PodcastList);
        self.trigger_background_refresh(crate::ui::events::BufferRefreshType::Downloads);
        self.trigger_background_refresh(crate::ui::events::BufferRefreshType::WhatsNew);
        self.trigger_async_refresh_today();

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
                // Try to find existing help buffer by name, or create a new one
                let mut help_id = self
                    .buffer_manager
                    .find_buffer_id_by_name("*Help: Keybindings*");
                if help_id.is_none() {
                    let entries = self.key_handler.generate_help_text();
                    self.buffer_manager.create_help_buffer(entries);
                    help_id = self
                        .buffer_manager
                        .find_buffer_id_by_name("*Help: Keybindings*");
                }
                if let Some(id) = help_id {
                    let _ = self.buffer_manager.switch_to_buffer(&id);
                }
                self.update_status_bar();
                Ok(true)
            }
            UIAction::SwitchBuffer(name) => {
                // Try to find buffer by name first, then fall back to ID
                let buffer_id = self
                    .buffer_manager
                    .find_buffer_id_by_name(&name)
                    .unwrap_or_else(|| name.clone());
                if self.buffer_manager.switch_to_buffer(&buffer_id).is_err() {
                    self.show_error(format!("Could not switch to buffer: {}", name));
                }
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
                    self.show_message("No buffer to close".to_string());
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
            UIAction::CreatePlaylist => {
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: "Create playlist: ".to_string(),
                    input: String::new(),
                });
                Ok(true)
            }
            UIAction::DeletePodcast => {
                if self.buffer_manager.current_buffer_id().as_deref() == Some("sync") {
                    // 'd' means delete — nothing to delete in the sync buffer
                    self.show_message("Nothing to delete in the sync buffer".to_string());
                    return Ok(true);
                }

                if self.buffer_manager.current_buffer_id().as_deref() == Some("playlist-list") {
                    if let Some(playlist_buffer) =
                        self.buffer_manager.get_playlist_list_buffer_mut()
                    {
                        if let Some(playlist) = playlist_buffer.selected_playlist() {
                            if matches!(
                                playlist.playlist_type,
                                crate::playlist::PlaylistType::AutoGenerated { .. }
                            ) {
                                self.show_message(
                                    "Auto-generated playlists are managed automatically"
                                        .to_string(),
                                );
                            } else {
                                self.pending_playlist_deletion = Some(playlist.id.clone());
                                self.minibuffer.set_content(MinibufferContent::Input {
                                    prompt: format!("Delete playlist '{}' (y/n)? ", playlist.name),
                                    input: String::new(),
                                });
                            }
                        } else {
                            self.show_message("No playlist selected for deletion".to_string());
                        }
                    }
                    return Ok(true);
                }

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
                        self.show_message("No podcast selected for deletion".to_string());
                    }
                } else {
                    self.show_error("Podcast list not available".to_string());
                }
                Ok(true)
            }
            UIAction::RefreshPodcast => {
                if self.buffer_manager.current_buffer_id().as_deref() == Some("sync") {
                    // 'r' in sync buffer just re-renders the current state
                    return Ok(true);
                }

                if let Some(current_id) = self.buffer_manager.current_buffer_id() {
                    if current_id.starts_with("playlist-") && current_id != "playlist-list" {
                        let result_action = if let Some(detail_buffer) = self
                            .buffer_manager
                            .get_playlist_detail_buffer_mut_by_id(&current_id)
                        {
                            detail_buffer.handle_action(UIAction::RefreshPodcast)
                        } else {
                            UIAction::ShowError("Playlist detail not available".to_string())
                        };

                        match result_action {
                            UIAction::RefreshAutoPlaylists => {
                                self.show_message("Refreshing Today playlist...".to_string());
                                self.trigger_async_refresh_today();
                            }
                            UIAction::RebuildPlaylistFiles { playlist_id } => {
                                self.show_message("Rebuilding playlist files...".to_string());
                                self.trigger_async_rebuild_playlist(playlist_id);
                            }
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {}
                        }
                        return Ok(true);
                    }
                }

                if self.buffer_manager.current_buffer_id().as_deref() == Some("playlist-list") {
                    self.trigger_async_refresh_today();
                    return Ok(true);
                }

                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    if let Some(podcast) = podcast_buffer.selected_podcast() {
                        let podcast_id = podcast.id.clone();
                        let podcast_title = podcast.title.clone();

                        // Show loading state
                        self.show_message(format!("Refreshing '{}'...", podcast_title));

                        // Trigger async refresh
                        self.trigger_async_refresh_single(podcast_id);
                    } else {
                        self.show_message("No podcast selected for refresh".to_string());
                    }
                } else {
                    self.show_error("Podcast list not available".to_string());
                }
                Ok(true)
            }
            UIAction::OpenPlaylistList => {
                let list_id = "playlist-list".to_string();
                if !self.buffer_manager.get_buffer_ids().contains(&list_id) {
                    self.buffer_manager
                        .create_playlist_list_buffer(self.playlist_manager.clone());
                }
                let _ = self.buffer_manager.switch_to_buffer(&list_id);
                self.update_status_bar();
                self.load_playlists_into_buffer().await;
                Ok(true)
            }
            UIAction::OpenPlaylistDetail {
                playlist_id,
                playlist_name,
            } => {
                match self.playlist_manager.get_playlist(&playlist_id).await {
                    Ok(playlist) => {
                        let detail_id = format!(
                            "playlist-{}",
                            playlist_name.replace(' ', "-").to_lowercase()
                        );
                        if !self.buffer_manager.get_buffer_ids().contains(&detail_id) {
                            self.buffer_manager.create_playlist_detail_buffer(
                                playlist_id.clone(),
                                playlist_name.clone(),
                                playlist.playlist_type.clone(),
                                self.playlist_manager.clone(),
                            );
                        }
                        if playlist.is_smart() {
                            // For smart playlists, show an empty placeholder then populate
                            // asynchronously via evaluate.
                            if let Some(detail_buffer) = self
                                .buffer_manager
                                .get_playlist_detail_buffer_mut_by_id(&detail_id)
                            {
                                detail_buffer.set_smart(true);
                            }
                            self.trigger_async_evaluate_smart_playlist(
                                playlist_id,
                                playlist_name,
                                detail_id.clone(),
                            );
                        } else if let Some(detail_buffer) = self
                            .buffer_manager
                            .get_playlist_detail_buffer_mut_by_id(&detail_id)
                        {
                            detail_buffer.set_playlist(playlist);
                        }
                        let _ = self.buffer_manager.switch_to_buffer(&detail_id);
                        self.update_status_bar();
                    }
                    Err(e) => self.show_error(format!("Could not open playlist: {}", e)),
                }
                Ok(true)
            }
            UIAction::AddToPlaylist => {
                // When in the sync buffer, 'p' opens the directory picker instead
                if self.buffer_manager.current_buffer_id().as_deref() == Some("sync") {
                    if let Some(sync_buffer) = self.buffer_manager.get_sync_buffer_mut() {
                        sync_buffer.enter_directory_picker();
                    }
                    return Ok(true);
                }

                if let Some(current_id) = self.buffer_manager.current_buffer_id() {
                    if !self.add_to_playlist_supported_in_buffer(&current_id) {
                        self.show_message(
                            "Add-to-playlist is available from episode lists, episode detail, and What's New".to_string(),
                        );
                        return Ok(true);
                    }

                    if let Some((podcast_id, episode_id)) =
                        self.resolve_add_to_playlist_selection(&current_id)
                    {
                        match self.playlist_manager.list_playlists().await {
                            Ok(playlists) => {
                                let options: Vec<_> = playlists
                                    .into_iter()
                                    .filter_map(|playlist| {
                                        if matches!(
                                            playlist.playlist_type,
                                            crate::playlist::PlaylistType::User
                                        ) {
                                            Some((
                                                playlist.id,
                                                playlist.name,
                                                playlist.episodes.len(),
                                            ))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                if options.is_empty() {
                                    self.show_error(
                                        "No user playlists available; create one first".to_string(),
                                    );
                                } else {
                                    let _ = self
                                        .buffer_manager
                                        .remove_buffer(&"playlist-picker".to_string());
                                    self.buffer_manager.create_playlist_picker_buffer(
                                        options, podcast_id, episode_id,
                                    );
                                    let _ = self
                                        .buffer_manager
                                        .switch_to_buffer(&"playlist-picker".to_string());
                                    self.update_status_bar();
                                }
                            }
                            Err(e) => self.show_error(format!("Could not list playlists: {}", e)),
                        }
                    } else {
                        self.show_message("No episode selected".to_string());
                    }
                }
                Ok(true)
            }
            UIAction::RefreshAutoPlaylists => {
                self.trigger_async_refresh_today();
                Ok(true)
            }
            UIAction::RebuildPlaylistFiles { playlist_id } => {
                self.show_message("Rebuilding playlist files...".to_string());
                self.trigger_async_rebuild_playlist(playlist_id);
                Ok(true)
            }
            UIAction::TriggerCreatePlaylist { name, description } => {
                self.trigger_async_create_playlist(name, description);
                Ok(true)
            }
            UIAction::TriggerAddToPlaylist {
                playlist_id,
                podcast_id,
                episode_id,
            } => {
                if self.buffer_manager.current_buffer_id().as_deref() == Some("playlist-picker") {
                    let _ = self
                        .buffer_manager
                        .remove_buffer(&"playlist-picker".to_string());
                    self.update_status_bar();
                }
                self.show_message("Downloading and adding to playlist...".to_string());
                self.trigger_async_add_to_playlist(playlist_id, podcast_id, episode_id);
                Ok(true)
            }
            UIAction::TriggerRemoveFromPlaylist {
                playlist_id,
                episode_id,
            } => {
                self.trigger_async_remove_from_playlist(playlist_id, episode_id);
                Ok(true)
            }
            UIAction::TriggerDeletePlaylist { playlist_id } => {
                self.trigger_async_delete_playlist(playlist_id);
                Ok(true)
            }
            UIAction::TriggerReorderPlaylist {
                playlist_id,
                from_idx,
                to_idx,
            } => {
                self.trigger_async_reorder_playlist(playlist_id, from_idx, to_idx);
                Ok(true)
            }
            UIAction::SyncPlaylist { .. } => {
                let default_path = self.get_default_sync_path();
                self.trigger_async_device_sync(default_path, false, false, false);
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
                        self.show_message("No podcast selected for hard refresh".to_string());
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
                // 'D' in sync buffer → dry-run preview with active target (or prompt if none)
                if self.buffer_manager.current_buffer_id().as_deref() == Some("sync") {
                    if let Some(active_path) = self
                        .buffer_manager
                        .get_sync_buffer_mut()
                        .and_then(|b| b.active_target().cloned())
                    {
                        let path_str = active_path.to_string_lossy().to_string();
                        self.trigger_async_device_sync(path_str, false, true, false);
                    } else {
                        let default_path = self.get_default_sync_path();
                        self.minibuffer.set_content(MinibufferContent::Input {
                            prompt: format!("Dry run sync to (default: {}): ", default_path),
                            input: String::new(),
                        });
                    }
                    return Ok(true);
                }
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
                        UIAction::TriggerRemoveFromPlaylist {
                            playlist_id,
                            episode_id,
                        } => {
                            self.trigger_async_remove_from_playlist(playlist_id, episode_id);
                        }
                        UIAction::ShowError(msg) => {
                            self.show_error(msg);
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
            UIAction::MarkPlayed => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result_action = current_buffer.handle_action(action);
                    match result_action {
                        UIAction::TriggerMarkPlayed {
                            podcast_id,
                            episode_id,
                            episode_title,
                        } => {
                            self.trigger_async_mark_played(podcast_id, episode_id, episode_title);
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        _ => {}
                    }
                    Ok(true)
                } else {
                    self.show_message("No active buffer".to_string());
                    Ok(true)
                }
            }
            UIAction::MarkUnplayed => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result_action = current_buffer.handle_action(action);
                    match result_action {
                        UIAction::TriggerMarkUnplayed {
                            podcast_id,
                            episode_id,
                            episode_title,
                        } => {
                            self.trigger_async_mark_unplayed(podcast_id, episode_id, episode_title);
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        _ => {}
                    }
                    Ok(true)
                } else {
                    self.show_message("No active buffer".to_string());
                    Ok(true)
                }
            }
            UIAction::TriggerMarkPlayed {
                podcast_id,
                episode_id,
                episode_title,
            } => {
                self.trigger_async_mark_played(podcast_id, episode_id, episode_title);
                Ok(true)
            }
            UIAction::TriggerMarkUnplayed {
                podcast_id,
                episode_id,
                episode_title,
            } => {
                self.trigger_async_mark_unplayed(podcast_id, episode_id, episode_title);
                Ok(true)
            }
            UIAction::ToggleFavorite => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result_action = current_buffer.handle_action(action);
                    match result_action {
                        UIAction::TriggerToggleFavorite {
                            podcast_id,
                            episode_id,
                            ref episode_title,
                            favorited,
                        } => {
                            let msg = if favorited {
                                format!("★ Favorited: {}", episode_title)
                            } else {
                                format!("Unfavorited: {}", episode_title)
                            };
                            self.show_message(msg);
                            self.trigger_async_toggle_favorite(
                                podcast_id,
                                episode_id,
                                episode_title.clone(),
                                favorited,
                            );
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        _ => {}
                    }
                    Ok(true)
                } else {
                    self.show_message("No active buffer".to_string());
                    Ok(true)
                }
            }
            UIAction::TriggerToggleFavorite {
                podcast_id,
                episode_id,
                episode_title,
                favorited,
            } => {
                self.trigger_async_toggle_favorite(
                    podcast_id,
                    episode_id,
                    episode_title,
                    favorited,
                );
                Ok(true)
            }
            UIAction::CycleSortField | UIAction::ToggleSortDirection => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    current_buffer.handle_action(action);
                }
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
                let default_path =
                    shellexpand::tilde(&self.config.storage.opml_export_directory).to_string();
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
            UIAction::Search => {
                // Open minibuffer with search prompt
                self.minibuffer.set_content(MinibufferContent::Input {
                    prompt: "Search: ".to_string(),
                    input: String::new(),
                });
                Ok(true)
            }
            UIAction::ApplySearch { query } => {
                // Dispatch directly to the active buffer
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    current_buffer.handle_action(UIAction::ApplySearch { query });
                }
                Ok(true)
            }
            UIAction::ClearFilters => {
                // Dispatch to the active buffer
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    current_buffer.handle_action(UIAction::ClearFilters);
                }
                self.show_message("Filters cleared".to_string());
                Ok(true)
            }
            UIAction::SetStatusFilter { status } => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result = current_buffer.handle_action(UIAction::SetStatusFilter { status });
                    match result {
                        UIAction::ShowMessage(msg) => self.show_message(msg),
                        UIAction::ShowError(msg) => self.show_error(msg),
                        _ => {}
                    }
                }
                Ok(true)
            }
            UIAction::SetDateRangeFilter { range } => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result =
                        current_buffer.handle_action(UIAction::SetDateRangeFilter { range });
                    match result {
                        UIAction::ShowMessage(msg) => self.show_message(msg),
                        UIAction::ShowError(msg) => self.show_error(msg),
                        _ => {}
                    }
                }
                Ok(true)
            }
            UIAction::Refresh => {
                // Handle F5 refresh - refresh current buffer based on its type
                let current_buffer_id = self.buffer_manager.current_buffer_id();
                if let Some(buffer_id) = current_buffer_id {
                    if buffer_id.starts_with("episodes-") {
                        // If it's an episode list buffer, trigger background refresh of its episodes
                        if let Some(episode_buffer) = self
                            .buffer_manager
                            .get_episode_list_buffer_mut_by_id(&buffer_id)
                        {
                            let podcast_id = episode_buffer.podcast_id.clone();
                            self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers {
                                podcast_id,
                            });
                            self.show_message("Refreshing episode list...".to_string());
                        }
                    } else if buffer_id == "podcast-list" {
                        // If it's the podcast list, trigger background refresh of podcasts
                        self.trigger_background_refresh(BufferRefreshType::PodcastList);
                        self.show_message("Refreshing podcast list...".to_string());
                    } else if buffer_id == "downloads" {
                        // If it's the downloads buffer, trigger background refresh of downloads
                        self.trigger_background_refresh(BufferRefreshType::Downloads);
                        self.show_message("Refreshing downloads...".to_string());
                    } else if buffer_id == "playlist-list" {
                        self.trigger_async_refresh_today();
                        self.show_message("Refreshing playlists...".to_string());
                    } else if buffer_id.starts_with("playlist-") {
                        let result_action = if let Some(detail_buffer) = self
                            .buffer_manager
                            .get_playlist_detail_buffer_mut_by_id(&buffer_id)
                        {
                            detail_buffer.handle_action(UIAction::Refresh)
                        } else {
                            UIAction::ShowError("Playlist detail not available".to_string())
                        };
                        match result_action {
                            UIAction::RefreshAutoPlaylists => {
                                self.show_message("Refreshing Today playlist...".to_string());
                                self.trigger_async_refresh_today();
                            }
                            UIAction::RebuildPlaylistFiles { playlist_id } => {
                                self.show_message("Rebuilding playlist files...".to_string());
                                self.trigger_async_rebuild_playlist(playlist_id);
                            }
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {}
                        }
                    } else if buffer_id == "whats-new" {
                        // If it's the What's New buffer, trigger background refresh of episodes
                        self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                        self.show_message("Refreshing What's New...".to_string());
                    } else {
                        self.show_message("Refresh not supported for this buffer".to_string());
                    }
                } else {
                    self.show_message("No active buffer to refresh".to_string());
                }
                Ok(true)
            }
            UIAction::SubscribeFromDiscovery { feed_url } => {
                self.show_message(format!("Subscribing to {}…", feed_url));
                self.trigger_async_add_podcast(feed_url);
                Ok(true)
            }
            // Audio playback — pass to buffer to resolve the selected episode, then dispatch
            // to AudioManager.
            UIAction::PlayEpisode { .. } => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result = current_buffer.handle_action(action);
                    match result {
                        UIAction::PlayEpisode {
                            podcast_id,
                            episode_id,
                            path,
                        } => {
                            if let Some(ref tx) = self.audio_command_tx {
                                let _ = tx.send(AudioCommand::Play {
                                    path,
                                    episode_id,
                                    podcast_id,
                                });
                            } else {
                                self.show_error(
                                    crate::constants::audio::UNAVAILABLE_ERROR.to_string(),
                                );
                            }
                        }
                        UIAction::ShowError(msg) => self.show_error(msg),
                        _ => {}
                    }
                }
                Ok(true)
            }
            // Simple playback controls forwarded to AudioManager
            UIAction::TogglePlayPause => {
                // Guard: S-P in minibuffer input mode is a typed character, not a command.
                if self.minibuffer.is_input_mode() {
                    return Ok(true);
                }
                if let Some(ref tx) = self.audio_command_tx {
                    let _ = tx.send(AudioCommand::TogglePlayPause);
                } else {
                    self.show_error(crate::constants::audio::UNAVAILABLE_ERROR.to_string());
                }
                Ok(true)
            }
            UIAction::StopPlayback => {
                if let Some(ref tx) = self.audio_command_tx {
                    let _ = tx.send(AudioCommand::Stop);
                }
                Ok(true)
            }
            UIAction::SeekForward => {
                if let Some(ref tx) = self.audio_command_tx {
                    let _ = tx.send(AudioCommand::SeekForward(Duration::from_secs(
                        crate::constants::audio::SEEK_STEP_SECS,
                    )));
                }
                Ok(true)
            }
            UIAction::SeekBackward => {
                if let Some(ref tx) = self.audio_command_tx {
                    let _ = tx.send(AudioCommand::SeekBackward(Duration::from_secs(
                        crate::constants::audio::SEEK_STEP_SECS,
                    )));
                }
                Ok(true)
            }
            UIAction::VolumeUp => {
                if let Some(ref tx) = self.audio_command_tx {
                    let _ = tx.send(AudioCommand::VolumeUp);
                }
                Ok(true)
            }
            UIAction::VolumeDown => {
                if let Some(ref tx) = self.audio_command_tx {
                    let _ = tx.send(AudioCommand::VolumeDown);
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
                            if self.buffer_manager.switch_to_buffer(&buffer_id).is_err() {
                                self.show_error(format!(
                                    "Could not switch to buffer: {}",
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
                        UIAction::OpenEpisodeDetail { episode } => {
                            self.open_episode_detail_buffer(*episode);
                        }
                        UIAction::OpenEpisodeDetailById {
                            podcast_id,
                            episode_id,
                        } => match self._storage.load_episode(&podcast_id, &episode_id).await {
                            Ok(episode) => self.open_episode_detail_buffer(episode),
                            Err(e) => {
                                self.show_error(format!("Could not open episode details: {}", e))
                            }
                        },
                        UIAction::OpenPlaylistDetail {
                            playlist_id,
                            playlist_name,
                        } => match self.playlist_manager.get_playlist(&playlist_id).await {
                            Ok(playlist) => {
                                let detail_id = format!(
                                    "playlist-{}",
                                    playlist_name.replace(' ', "-").to_lowercase()
                                );
                                if !self.buffer_manager.get_buffer_ids().contains(&detail_id) {
                                    self.buffer_manager.create_playlist_detail_buffer(
                                        playlist_id.clone(),
                                        playlist_name.clone(),
                                        playlist.playlist_type.clone(),
                                        self.playlist_manager.clone(),
                                    );
                                }
                                if let Some(detail_buffer) = self
                                    .buffer_manager
                                    .get_playlist_detail_buffer_mut_by_id(&detail_id)
                                {
                                    detail_buffer.set_playlist(playlist);
                                }
                                let _ = self.buffer_manager.switch_to_buffer(&detail_id);
                                self.update_status_bar();
                            }
                            Err(e) => self.show_error(format!("Could not open playlist: {}", e)),
                        },
                        UIAction::TriggerDeletePlaylist { playlist_id } => {
                            self.trigger_async_delete_playlist(playlist_id);
                        }
                        UIAction::TriggerAddToPlaylist {
                            playlist_id,
                            podcast_id,
                            episode_id,
                        } => {
                            self.trigger_async_add_to_playlist(playlist_id, podcast_id, episode_id);
                        }
                        UIAction::TriggerRemoveFromPlaylist {
                            playlist_id,
                            episode_id,
                        } => {
                            self.trigger_async_remove_from_playlist(playlist_id, episode_id);
                        }
                        UIAction::TriggerReorderPlaylist {
                            playlist_id,
                            from_idx,
                            to_idx,
                        } => {
                            self.trigger_async_reorder_playlist(playlist_id, from_idx, to_idx);
                        }
                        UIAction::RefreshAutoPlaylists => {
                            self.trigger_async_refresh_today();
                        }
                        UIAction::ShowMessage(msg) => {
                            self.show_message(msg);
                        }
                        UIAction::ShowError(msg) => {
                            self.show_error(msg);
                        }
                        UIAction::Search => {
                            // Buffer bubbled up Search — open the minibuffer prompt
                            self.minibuffer.set_content(MinibufferContent::Input {
                                prompt: "Search: ".to_string(),
                                input: String::new(),
                            });
                        }
                        UIAction::PromptInput(prompt) => {
                            // Buffer wants to prompt the user for input
                            self.minibuffer.set_content(MinibufferContent::Input {
                                prompt,
                                input: String::new(),
                            });
                        }
                        UIAction::TriggerDeviceSync {
                            device_path,
                            delete_orphans,
                            mut dry_run,
                        } => {
                            // Buffer has an active target and wants to sync directly (no prompt).
                            // If sync_preview_before_sync is set and we're NOT already coming from
                            // the DryRunPreview mode, convert to a dry-run first.
                            let is_from_preview = self
                                .buffer_manager
                                .get_sync_buffer_mut()
                                .map(|b| b.is_in_dry_run_preview_mode())
                                .unwrap_or(false);
                            if !dry_run
                                && self.config.downloads.sync_preview_before_sync
                                && !is_from_preview
                            {
                                dry_run = true;
                            }
                            let path_str = device_path.to_string_lossy().to_string();
                            self.trigger_async_device_sync(
                                path_str,
                                delete_orphans,
                                dry_run,
                                false,
                            );
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
                // Trigger background refresh of podcast list
                self.trigger_background_refresh(BufferRefreshType::PodcastList);
                self.show_message(format!("Successfully subscribed to: {}", podcast.title));
            }
            AppEvent::PodcastSubscriptionFailed { url: _, error } => {
                self.show_error(format!("Could not subscribe to podcast: {}", error));
            }
            AppEvent::PodcastRefreshed {
                podcast_id: _,
                new_episode_count,
            } => {
                // Trigger background refresh of buffers
                self.trigger_background_refresh(BufferRefreshType::PodcastList);
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
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
                self.show_error(format!("Could not refresh podcast feed: {}", error));
            }
            AppEvent::AllPodcastsRefreshed { total_new_episodes } => {
                // Trigger background refresh of buffers
                self.trigger_background_refresh(BufferRefreshType::PodcastList);
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                if total_new_episodes > 0 {
                    self.show_message(format!(
                        "Podcast refresh completed. Found {} new episode(s). Updating buffers...",
                        total_new_episodes
                    ));
                } else {
                    self.show_message(
                        "Podcast refresh completed. No new episodes found".to_string(),
                    );
                }
            }
            AppEvent::BufferDataRefreshed { buffer_type, data } => {
                // Update buffer data without blocking the UI thread
                self.handle_buffer_data_refresh(buffer_type, data);
            }
            AppEvent::PodcastDeleted {
                podcast_id: _,
                podcast_title,
            } => {
                // Trigger background refresh of podcast list
                self.trigger_background_refresh(BufferRefreshType::PodcastList);
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
                self.show_error(format!("Could not delete podcast: {}", error));
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
                self.show_error(format!("Could not load episodes: {}", error));
            }
            AppEvent::EpisodeDownloaded {
                podcast_id,
                episode_id: _,
            } => {
                // Trigger background refresh of buffers
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                self.show_message("Episode download completed successfully".to_string());
            }
            AppEvent::EpisodeDownloadFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Trigger background refresh of buffers
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_error(format!("Episode download failed: {}", error));
            }
            AppEvent::EpisodeDownloadDeleted {
                podcast_id,
                episode_id: _,
            } => {
                // Trigger background refresh of buffers
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_message("Episode download deleted successfully".to_string());
            }
            AppEvent::EpisodeDownloadDeletionFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Trigger background refresh of buffers
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_error(format!("Could not delete episode download: {}", error));
            }
            AppEvent::EpisodeMarkedPlayed {
                podcast_id,
                episode_id: _,
                episode_title,
            } => {
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                self.show_message(format!("Marked as played: {}", episode_title));
            }
            AppEvent::EpisodeMarkPlayedFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Refresh buffers to revert optimistic UI update
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                self.show_error(format!("Could not mark episode as played: {}", error));
            }
            AppEvent::EpisodeMarkedUnplayed {
                podcast_id,
                episode_id: _,
                episode_title,
            } => {
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                self.show_message(format!("Marked as unplayed: {}", episode_title));
            }
            AppEvent::EpisodeMarkUnplayedFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Refresh buffers to revert optimistic UI update
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                self.show_error(format!("Could not mark episode as unplayed: {}", error));
            }
            AppEvent::EpisodeFavoriteToggled {
                podcast_id: _,
                episode_id: _,
                episode_title: _,
                favorited: _,
            } => {
                // Optimistic update already applied; nothing further needed
            }
            AppEvent::EpisodeFavoriteToggleFailed {
                podcast_id,
                episode_id: _,
                error,
            } => {
                // Refresh buffers to revert optimistic UI update
                self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers { podcast_id });
                self.trigger_background_refresh(BufferRefreshType::WhatsNew);
                self.show_error(format!("Could not save favorite: {}", error));
            }
            AppEvent::DownloadsRefreshed => {
                // Trigger background refresh of downloads buffer
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_message("Downloads refreshed".to_string());
            }
            AppEvent::AllDownloadsDeleted { deleted_count } => {
                // Trigger background refresh of all episode buffers and downloads buffer
                self.trigger_background_refresh(BufferRefreshType::AllEpisodeBuffers);
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_message(format!(
                    "Successfully deleted {} downloaded episodes and cleaned up downloads folder",
                    deleted_count
                ));
            }
            AppEvent::AllDownloadsDeletionFailed { error } => {
                // Still trigger background refresh of buffers in case some deletions succeeded
                self.trigger_background_refresh(BufferRefreshType::AllEpisodeBuffers);
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_error(format!("Could not delete all downloads: {}", error));
            }
            AppEvent::OpmlImportStarted { source } => {
                self.show_message(format!("Starting OPML import from: {}...", source));
            }
            AppEvent::OpmlImportProgress { status, .. } => {
                self.show_message(status);
            }
            AppEvent::OpmlImportCompleted { result, log_path } => {
                // Trigger background refresh of podcast list to show newly imported podcasts
                self.trigger_background_refresh(BufferRefreshType::PodcastList);

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
            AppEvent::OpmlImportFailed { source: _, error } => {
                self.show_error(format!("Could not import OPML: {}", error));
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
            AppEvent::OpmlExportFailed { path: _, error } => {
                self.show_error(format!("Could not export OPML: {}", error));
            }
            AppEvent::PlaylistCreated { playlist } => {
                self.load_playlists_into_buffer().await;
                self.show_message(format!("Playlist created: {}", playlist.name));
            }
            AppEvent::PlaylistCreationFailed { name, error } => {
                self.show_error(format!("Could not create playlist '{}': {}", name, error));
            }
            AppEvent::PlaylistDeleted { name } => {
                self.load_playlists_into_buffer().await;
                self.show_message(format!("Playlist deleted: {}", name));
            }
            AppEvent::PlaylistDeletionFailed { name, error } => {
                self.show_error(format!("Could not delete playlist '{}': {}", name, error));
            }
            AppEvent::EpisodeAddedToPlaylist {
                playlist_name,
                episode_title,
            } => {
                self.load_playlists_into_buffer().await;
                self.refresh_open_playlist_detail_buffers().await;
                self.trigger_background_refresh(BufferRefreshType::AllEpisodeBuffers);
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                if self.buffer_manager.current_buffer_id().as_deref() == Some("playlist-picker") {
                    let _ = self
                        .buffer_manager
                        .remove_buffer(&"playlist-picker".to_string());
                    self.update_status_bar();
                }
                self.show_message(format!(
                    "Added '{}' to playlist '{}'",
                    episode_title, playlist_name
                ));
            }
            AppEvent::EpisodeAddToPlaylistFailed {
                playlist_name,
                episode_title,
                error,
            } => {
                if self.buffer_manager.current_buffer_id().as_deref() == Some("playlist-picker") {
                    let _ = self
                        .buffer_manager
                        .remove_buffer(&"playlist-picker".to_string());
                    self.update_status_bar();
                }
                self.show_error(format!(
                    "Could not add '{}' to playlist '{}': {}",
                    episode_title, playlist_name, error
                ));
            }
            AppEvent::EpisodeRemovedFromPlaylist {
                playlist_name,
                episode_title,
            } => {
                self.load_playlists_into_buffer().await;
                self.refresh_open_playlist_detail_buffers().await;
                self.show_message(format!(
                    "Removed '{}' from playlist '{}'",
                    episode_title, playlist_name
                ));
            }
            AppEvent::EpisodeRemoveFromPlaylistFailed {
                playlist_name,
                episode_title,
                error,
            } => {
                self.show_error(format!(
                    "Could not remove '{}' from playlist '{}': {}",
                    episode_title, playlist_name, error
                ));
            }
            AppEvent::PlaylistReordered { name } => {
                self.load_playlists_into_buffer().await;
                self.refresh_open_playlist_detail_buffers().await;
                self.show_message(format!("Playlist reordered: {}", name));
            }
            AppEvent::PlaylistReorderFailed { name, error } => {
                self.show_error(format!("Could not reorder playlist '{}': {}", name, error));
            }
            AppEvent::PlaylistRebuilt {
                name,
                rebuilt,
                skipped,
                failed,
            } => {
                self.load_playlists_into_buffer().await;
                self.refresh_open_playlist_detail_buffers().await;
                self.show_message(format!(
                    "Playlist rebuilt '{}': {} rebuilt, {} skipped, {} failed",
                    name, rebuilt, skipped, failed
                ));
            }
            AppEvent::PlaylistRebuildFailed { name, error } => {
                self.show_error(format!("Could not rebuild playlist '{}': {}", name, error));
            }
            AppEvent::TodayPlaylistRefreshed { added, removed } => {
                self.load_playlists_into_buffer().await;
                self.refresh_open_playlist_detail_buffers().await;
                self.show_message(format!(
                    "Today playlist refreshed: {} added, {} removed",
                    added, removed
                ));
            }
            AppEvent::TodayPlaylistRefreshFailed { error } => {
                self.show_error(format!("Could not refresh Today playlist: {}", error));
            }
            AppEvent::DeviceSyncStarted {
                device_path,
                dry_run,
                hard_sync,
            } => {
                let mode = if dry_run { " (dry run)" } else { "" };
                let sync_kind = if hard_sync { "hard " } else { "" };
                self.show_message(format!(
                    "Starting {}device sync to {}{}...",
                    sync_kind,
                    device_path.display(),
                    mode
                ));
            }
            AppEvent::DeviceSyncCompleted {
                device_path,
                report,
                dry_run,
            } => {
                if dry_run {
                    // Enter dry-run preview mode so the user can review and confirm
                    if let Some(sync_buffer) = self.buffer_manager.get_sync_buffer_mut() {
                        sync_buffer.enter_dry_run_preview(device_path.clone(), report.clone());
                    }
                    let summary = if report.is_success() {
                        format!(
                            "Dry run: {} to copy, {} to delete, {} skip — press Enter/s to confirm",
                            report.files_copied.len(),
                            report.files_deleted.len(),
                            report.files_skipped.len()
                        )
                    } else {
                        format!(
                            "Dry run: {} to copy, {} to delete, {} skip ({} errors)",
                            report.files_copied.len(),
                            report.files_deleted.len(),
                            report.files_skipped.len(),
                            report.errors.len()
                        )
                    };
                    self.show_message(summary);
                } else {
                    // Real sync completed — update history, return to overview
                    if let Some(sync_buffer) = self.buffer_manager.get_sync_buffer_mut() {
                        sync_buffer.add_sync_result(device_path.clone(), report.clone(), dry_run);
                        sync_buffer.reset_to_overview();
                    }

                    let summary = if report.is_success() {
                        format!(
                            "Sync complete: {} copied, {} deleted, {} skipped",
                            report.files_copied.len(),
                            report.files_deleted.len(),
                            report.files_skipped.len()
                        )
                    } else {
                        format!(
                            "Sync complete with {} errors: {} copied, {} deleted, {} skipped",
                            report.errors.len(),
                            report.files_copied.len(),
                            report.files_deleted.len(),
                            report.files_skipped.len()
                        )
                    };
                    self.load_playlists_into_buffer().await;
                    self.show_message(summary);
                }
            }
            AppEvent::DeviceSyncProgress { event } => {
                if let Some(sync_buffer) = self.buffer_manager.get_sync_buffer_mut() {
                    let complete = sync_buffer.handle_progress_event(event);
                    if complete {
                        // Complete event will be followed by DeviceSyncCompleted; nothing extra needed
                    }
                }
            }
            AppEvent::DeviceSyncFailed {
                device_path: _,
                error,
            } => {
                // On failure, return sync buffer to Overview
                if let Some(sync_buffer) = self.buffer_manager.get_sync_buffer_mut() {
                    // enter_progress_mode is only called for real syncs; reset to overview on error
                    if sync_buffer.is_in_progress_mode() {
                        sync_buffer.reset_to_overview();
                    }
                }
                self.show_error(format!("Could not sync device: {}", error));
            }
            AppEvent::DownloadCleanupCompleted {
                deleted_count,
                duration_label,
            } => {
                self.trigger_background_refresh(BufferRefreshType::AllEpisodeBuffers);
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                if deleted_count > 0 {
                    self.show_message(format!(
                        "Cleaned up {} episode(s) older than {}",
                        deleted_count, duration_label
                    ));
                } else {
                    self.show_message(format!(
                        "No downloaded episodes older than {}",
                        duration_label
                    ));
                }
            }
            AppEvent::DownloadCleanupFailed { error } => {
                self.trigger_background_refresh(BufferRefreshType::AllEpisodeBuffers);
                self.trigger_background_refresh(BufferRefreshType::Downloads);
                self.show_error(format!("Could not clean up downloads: {}", error));
            }
            AppEvent::PodcastTagAdded {
                podcast_id: _,
                tag: _,
            } => {
                // Optimistic update already applied; nothing further needed
            }
            AppEvent::PodcastTagAddFailed {
                podcast_id: _,
                error,
            } => {
                // Revert optimistic update by refreshing from storage
                self.trigger_background_refresh(BufferRefreshType::PodcastList);
                self.show_error(format!("Could not add tag: {}", error));
            }
            AppEvent::PodcastTagRemoved {
                podcast_id: _,
                tag: _,
            } => {
                // Optimistic update already applied; nothing further needed
            }
            AppEvent::PodcastTagRemoveFailed {
                podcast_id: _,
                error,
            } => {
                // Revert optimistic update by refreshing from storage
                self.trigger_background_refresh(BufferRefreshType::PodcastList);
                self.show_error(format!("Could not remove tag: {}", error));
            }
            AppEvent::SmartPlaylistEvaluated {
                detail_buffer_id,
                episodes,
            } => {
                if let Some(detail_buffer) = self
                    .buffer_manager
                    .get_playlist_detail_buffer_mut_by_id(&detail_buffer_id)
                {
                    detail_buffer.set_evaluated_episodes(episodes);
                }
            }
            AppEvent::SmartPlaylistEvaluationFailed {
                playlist_name,
                error,
            } => {
                self.show_error(format!(
                    "Smart playlist '{}' failed: {}",
                    playlist_name, error
                ));
            }
            AppEvent::DiscoveryResultsLoaded {
                buffer_id,
                results,
                title: _,
            } => {
                if let Some(buf) = self
                    .buffer_manager
                    .get_discovery_buffer_mut_by_id(&buffer_id)
                {
                    buf.set_results(results);
                }
                self.update_status_bar();
            }
            AppEvent::DiscoveryLoadFailed { buffer_id, error } => {
                if let Some(buf) = self
                    .buffer_manager
                    .get_discovery_buffer_mut_by_id(&buffer_id)
                {
                    buf.set_error(error.clone());
                }
                self.show_error(format!("Discovery failed: {}", error));
            }
            AppEvent::PlaybackStarted {
                podcast_id,
                episode_id,
            } => {
                // Look up episode title and podcast name for the NowPlaying buffer.
                let episode_title = self
                    ._storage
                    .load_episode(&podcast_id, &episode_id)
                    .await
                    .ok()
                    .map(|ep| ep.title);
                let podcast_name = self
                    ._storage
                    .load_podcast(&podcast_id)
                    .await
                    .ok()
                    .map(|p| p.title);
                self.buffer_manager
                    .set_now_playing_info(episode_title, podcast_name);
                self.show_message("Now playing…".to_string());
            }
            AppEvent::PlaybackStopped => {
                self.show_message("Playback stopped".to_string());
            }
            AppEvent::TrackEnded {
                podcast_id,
                episode_id,
            } => {
                // Persist completion: update position to full duration (auto-marks played at ≥95%).
                match self._storage.load_episode(&podcast_id, &episode_id).await {
                    Ok(mut episode) => {
                        if let Some(duration) = episode.duration {
                            episode.update_position(duration);
                        } else {
                            // No duration recorded — mark as played without position update.
                            episode.mark_played();
                        }
                        if let Err(e) = self._storage.save_episode(&podcast_id, &episode).await {
                            eprintln!("[audio] Failed to save episode after track end: {e}");
                        }
                        // Refresh episode buffers so played status is reflected immediately.
                        self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers {
                            podcast_id: podcast_id.clone(),
                        });
                        self.trigger_background_refresh(BufferRefreshType::AllEpisodeBuffers);
                    }
                    Err(e) => {
                        eprintln!("[audio] Failed to load episode for track-end update: {e}");
                    }
                }
                self.show_message("Finished playing episode".to_string());
            }
            AppEvent::PlaybackError { error } => {
                self.show_error(format!("Playback error: {}", error));
            }
        }
        Ok(())
    }

    /// Execute a command directly without recursion
    fn execute_command_direct(&mut self, command: String) -> UIResult<bool> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "quit" | "q" => {
                self.should_quit = true;
                Ok(false)
            }
            "help" | "h" => {
                let mut help_id = self
                    .buffer_manager
                    .find_buffer_id_by_name("*Help: Keybindings*");
                if help_id.is_none() {
                    let entries = self.key_handler.generate_help_text();
                    self.buffer_manager.create_help_buffer(entries);
                    help_id = self
                        .buffer_manager
                        .find_buffer_id_by_name("*Help: Keybindings*");
                }
                if let Some(id) = help_id {
                    let _ = self.buffer_manager.switch_to_buffer(&id);
                }
                self.update_status_bar();
                Ok(true)
            }
            "theme" => {
                if parts.len() > 1 {
                    self.set_theme_direct(parts[1])
                } else {
                    let names = self.theme_registry.list_names().join(", ");
                    self.show_error(format!("Usage: theme <name> ({})", names));
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
                        self.show_message("No buffer to close".to_string());
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
                    let default_path =
                        shellexpand::tilde(&self.config.storage.opml_export_directory).to_string();
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: format!("Export to (default: {}): ", default_path),
                        input: String::new(),
                    });
                    Ok(true)
                }
            }
            "sync" | "sync-device" => {
                let (device_path, hard_sync) = Self::parse_sync_command_args(&parts[1..]);
                if let Some(device_path) = device_path {
                    self.trigger_async_device_sync(device_path, false, false, hard_sync);
                    Ok(true)
                } else if hard_sync {
                    let default_path = self.get_default_sync_path();
                    self.trigger_async_device_sync(default_path, false, false, true);
                    Ok(true)
                } else {
                    // Prompt for device path with default from config
                    let default_path = self.get_default_sync_path();
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: format!("Sync to device path (default: {}): ", default_path),
                        input: String::new(),
                    });
                    Ok(true)
                }
            }
            "sync-dry-run" | "sync-preview" => {
                let (device_path, hard_sync) = Self::parse_sync_command_args(&parts[1..]);
                if let Some(device_path) = device_path {
                    self.trigger_async_device_sync(device_path, false, true, hard_sync);
                    Ok(true)
                } else if hard_sync {
                    let default_path = self.get_default_sync_path();
                    self.trigger_async_device_sync(default_path, false, true, true);
                    Ok(true)
                } else {
                    // Prompt for device path
                    let default_path = self.get_default_sync_path();
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: format!("Dry run sync to (default: {}): ", default_path),
                        input: String::new(),
                    });
                    Ok(true)
                }
            }
            "playlists" => {
                let list_id = "playlist-list".to_string();
                if !self.buffer_manager.get_buffer_ids().contains(&list_id) {
                    self.buffer_manager
                        .create_playlist_list_buffer(self.playlist_manager.clone());
                }
                let _ = self.buffer_manager.switch_to_buffer(&list_id);
                self.update_status_bar();
                self.trigger_async_refresh_today();
                Ok(true)
            }
            "playlist-create" | "playlist-new" => {
                if parts.len() > 1 {
                    let name = parts[1..].join(" ");
                    self.trigger_async_create_playlist(name, None);
                } else {
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: "Create playlist: ".to_string(),
                        input: String::new(),
                    });
                }
                Ok(true)
            }
            "smart-playlist" => {
                if parts.len() < 2 {
                    self.show_error(
                        "Usage: :smart-playlist <name> [--filter <spec>...] [--sort <field>] [--limit <n>]".to_string(),
                    );
                    return Ok(true);
                }
                // Allow multi-word names: consume tokens until the first --flag
                let flags_start_idx = parts[2..]
                    .iter()
                    .position(|p| p.starts_with("--"))
                    .map(|i| i + 2)
                    .unwrap_or(parts.len());
                let name = parts[1..flags_start_idx].join(" ");
                let args = &parts[flags_start_idx..];
                match crate::ui::app::parse_smart_playlist_args(args) {
                    Ok(rule) => self.trigger_async_create_smart_playlist(name, rule),
                    Err(msg) => self.show_error(msg),
                }
                Ok(true)
            }
            "discover" | "search-podcasts" => {
                if parts.len() < 2 {
                    self.show_error("Usage: :discover <search term>".to_string());
                    return Ok(true);
                }
                let query = parts[1..].join(" ");
                let buffer_id = format!("discovery-{}", query.replace(' ', "-").to_lowercase());
                let display_title = format!("Search: {}", query);
                // Reuse existing buffer if query is the same, otherwise create fresh
                if self.buffer_manager.get_buffer(&buffer_id).is_none() {
                    self.buffer_manager
                        .create_discovery_buffer(buffer_id.clone(), display_title.clone());
                }
                let _ = self.buffer_manager.switch_to_buffer(&buffer_id);
                self.trigger_async_discover(query, buffer_id);
                self.update_status_bar();
                Ok(true)
            }
            "trending" => {
                let buffer_id = "discovery-trending".to_string();
                let display_title = "Trending".to_string();
                if self.buffer_manager.get_buffer(&buffer_id).is_none() {
                    self.buffer_manager
                        .create_discovery_buffer(buffer_id.clone(), display_title.clone());
                }
                let _ = self.buffer_manager.switch_to_buffer(&buffer_id);
                self.trigger_async_trending(buffer_id);
                self.update_status_bar();
                Ok(true)
            }
            "playlist-delete" => {
                if parts.len() > 1 {
                    let name = parts[1..].join(" ");
                    self.trigger_async_delete_playlist_by_name(name);
                } else {
                    self.show_error("Usage: playlist-delete <name>".to_string());
                }
                Ok(true)
            }
            "playlist-refresh" => {
                self.trigger_async_refresh_today();
                Ok(true)
            }
            "playlist-sync" => {
                let default_path = self.get_default_sync_path();
                self.trigger_async_device_sync(default_path, false, false, false);
                Ok(true)
            }
            "clean-older-than" | "cleanup" => {
                if parts.len() > 1 {
                    let duration_str = parts[1];
                    if let Some(total_hours) =
                        crate::utils::time::parse_cleanup_duration(duration_str)
                    {
                        let label = crate::utils::time::format_cleanup_duration(total_hours);
                        self.minibuffer.set_content(MinibufferContent::Input {
                            prompt: format!(
                                "Delete downloaded episodes older than {}? (y/n) ",
                                label
                            ),
                            input: String::new(),
                        });
                        self.pending_cleanup_hours = Some(total_hours);
                    } else {
                        self.show_error(format!(
                            "Invalid duration: '{}'. Use e.g., 7d, 2w, 1m, 12h",
                            duration_str
                        ));
                    }
                    Ok(true)
                } else {
                    // Prompt for duration
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: "Delete downloads older than (e.g., 7d, 2w, 1m, 12h): ".to_string(),
                        input: String::new(),
                    });
                    Ok(true)
                }
            }
            "search" => {
                if parts.len() > 1 {
                    let query = parts[1..].join(" ");
                    if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                        let result = current_buffer.handle_action(UIAction::ApplySearch { query });
                        if let UIAction::ShowMessage(msg) = result {
                            self.show_message(msg);
                        }
                    }
                    Ok(true)
                } else {
                    // Open search prompt
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: "Search: ".to_string(),
                        input: String::new(),
                    });
                    Ok(true)
                }
            }
            "filter-status" => {
                if parts.len() > 1 {
                    let status = parts[1].to_string();
                    if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                        let result =
                            current_buffer.handle_action(UIAction::SetStatusFilter { status });
                        match result {
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {}
                        }
                    }
                    Ok(true)
                } else {
                    self.show_error(
                        "Usage: filter-status <status> (new, downloaded, played, downloading, failed, favorited)"
                            .to_string(),
                    );
                    Ok(true)
                }
            }
            "filter-date" => {
                if parts.len() > 1 {
                    let range = parts[1].to_string();
                    if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                        let result =
                            current_buffer.handle_action(UIAction::SetDateRangeFilter { range });
                        match result {
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {}
                        }
                    }
                    Ok(true)
                } else {
                    self.show_error(
                        "Usage: filter-date <range> (today, 12h, 7d, 2w, 1m)".to_string(),
                    );
                    Ok(true)
                }
            }
            "tag" => {
                if parts.len() > 1 {
                    let tag = parts[1..].join(" ");
                    if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut()
                    {
                        let result = podcast_buffer.handle_action(UIAction::AddTag { tag });
                        match result {
                            UIAction::TriggerAddTag {
                                podcast_id,
                                podcast_title,
                                tag,
                            } => {
                                self.show_message(format!(
                                    "Tagged '{}' with \"{}\"",
                                    podcast_title, tag
                                ));
                                self.trigger_async_add_tag(podcast_id, tag);
                            }
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {
                                self.show_error(
                                    "Could not add tag. Is a podcast selected?".to_string(),
                                );
                            }
                        }
                    } else {
                        self.show_error("Podcast list not available".to_string());
                    }
                    Ok(true)
                } else {
                    self.show_error("Usage: tag <name>".to_string());
                    Ok(true)
                }
            }
            "untag" => {
                if parts.len() > 1 {
                    let tag = parts[1..].join(" ");
                    if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut()
                    {
                        let result = podcast_buffer.handle_action(UIAction::RemoveTag { tag });
                        match result {
                            UIAction::TriggerRemoveTag {
                                podcast_id,
                                podcast_title,
                                tag,
                            } => {
                                self.show_message(format!(
                                    "Removed tag \"{}\" from '{}'",
                                    tag, podcast_title
                                ));
                                self.trigger_async_remove_tag(podcast_id, tag);
                            }
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {
                                self.show_error(
                                    "Could not remove tag. Is a podcast selected?".to_string(),
                                );
                            }
                        }
                    } else {
                        self.show_error("Podcast list not available".to_string());
                    }
                    Ok(true)
                } else {
                    self.show_error("Usage: untag <name>".to_string());
                    Ok(true)
                }
            }
            "tags" => {
                // Collect all unique tags from in-memory podcast list
                let tags: Vec<String> = if let Some(podcast_buffer) =
                    self.buffer_manager.get_podcast_list_buffer_mut()
                {
                    let mut all_tags: Vec<String> = podcast_buffer
                        .podcasts()
                        .iter()
                        .flat_map(|p| p.tags.iter().cloned())
                        .collect();
                    all_tags.sort_unstable();
                    all_tags.dedup();
                    all_tags
                } else {
                    Vec::new()
                };

                if tags.is_empty() {
                    self.show_message(
                        "No tags defined. Use :tag <name> to tag a podcast.".to_string(),
                    );
                } else {
                    self.show_message(format!("Tags: {}", tags.join(", ")));
                }
                Ok(true)
            }
            "filter-tag" => {
                if parts.len() > 1 {
                    let tag = parts[1..].join(" ");
                    if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut()
                    {
                        let result = podcast_buffer
                            .handle_action(UIAction::FilterByTag { tag: tag.clone() });
                        match result {
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {}
                        }
                        // Switch to podcast list so user sees the filter applied
                        let _ = self
                            .buffer_manager
                            .switch_to_buffer(&"podcast-list".to_string());
                    } else {
                        self.show_error("Podcast list not available".to_string());
                    }
                    Ok(true)
                } else {
                    self.show_error("Usage: filter-tag <tag>".to_string());
                    Ok(true)
                }
            }
            "clear-filters" | "widen" => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    current_buffer.handle_action(UIAction::ClearFilters);
                }
                self.show_message("Filters cleared".to_string());
                Ok(true)
            }
            "sort" => {
                if parts.len() > 1 {
                    let field = parts[1].to_string();
                    if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                        let result = current_buffer.handle_action(UIAction::SetSort { field });
                        match result {
                            UIAction::ShowMessage(msg) => self.show_message(msg),
                            UIAction::ShowError(msg) => self.show_error(msg),
                            _ => {}
                        }
                    }
                    Ok(true)
                } else {
                    self.show_error(
                        "Usage: sort <field> (date, title, duration, downloaded)".to_string(),
                    );
                    Ok(true)
                }
            }
            "sort-asc" => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result = current_buffer.handle_action(UIAction::SetSortDirection {
                        direction: "asc".to_string(),
                    });
                    match result {
                        UIAction::ShowMessage(msg) => self.show_message(msg),
                        UIAction::ShowError(msg) => self.show_error(msg),
                        _ => {}
                    }
                }
                Ok(true)
            }
            "sort-desc" => {
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    let result = current_buffer.handle_action(UIAction::SetSortDirection {
                        direction: "desc".to_string(),
                    });
                    match result {
                        UIAction::ShowMessage(msg) => self.show_message(msg),
                        UIAction::ShowError(msg) => self.show_error(msg),
                        _ => {}
                    }
                }
                Ok(true)
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
        match self.theme_registry.get(theme_name).cloned() {
            Some(new_theme) => {
                self.theme = new_theme.clone();
                self.status_bar.set_theme(new_theme);
                self.show_message(format!("Theme changed to: {}", theme_name));
                Ok(true)
            }
            None => {
                self.show_error(format!("Unknown theme: {}", theme_name));
                Ok(true)
            }
        }
    }

    /// Set the application theme (direct version)
    fn set_theme_direct(&mut self, theme_name: &str) -> UIResult<bool> {
        match self.theme_registry.get(theme_name).cloned() {
            Some(new_theme) => {
                self.theme = new_theme.clone();
                self.buffer_manager.set_theme_all(&new_theme);
                self.minibuffer.set_theme(new_theme.clone());
                self.status_bar.set_theme(new_theme);
                self.show_message(format!("Theme changed to: {}", theme_name));
                Ok(true)
            }
            None => {
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
        if self
            .buffer_manager
            .add_buffer(Box::new(buffer_list_buffer))
            .is_ok()
        {
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
            "help" => "*Help: Keybindings*".to_string(),
            "download" | "dl" => "downloads".to_string(),
            "new" | "whats-new" | "latest" => "whats-new".to_string(),
            "sync" | "device-sync" => "sync".to_string(),
            "playlist" | "playlists" => "playlist-list".to_string(),
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
        let mut commands = vec![
            // Core commands
            "quit".to_string(),
            "q".to_string(),
            "help".to_string(),
            "h".to_string(),
            // Theme commands
            "theme".to_string(),
        ];
        // Add one completion entry per registered theme name
        for name in self.theme_registry.list_names() {
            commands.push(format!("theme {}", name));
        }
        commands.extend([
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
            // Sync commands
            "sync".to_string(),
            "sync-device".to_string(),
            "sync-dry-run".to_string(),
            "sync-preview".to_string(),
            // Playlist commands
            "playlists".to_string(),
            "playlist-create".to_string(),
            "playlist-new".to_string(),
            "playlist-delete".to_string(),
            "playlist-refresh".to_string(),
            "playlist-sync".to_string(),
            "smart-playlist".to_string(),
            // Discovery commands
            "discover".to_string(),
            "search-podcasts".to_string(),
            "trending".to_string(),
            // Cleanup commands
            "clean-older-than".to_string(),
            "cleanup".to_string(),
            // Search & filter commands
            "search".to_string(),
            "filter-status".to_string(),
            "filter-status new".to_string(),
            "filter-status downloaded".to_string(),
            "filter-status played".to_string(),
            "filter-status downloading".to_string(),
            "filter-status failed".to_string(),
            "filter-status favorited".to_string(),
            "filter-date".to_string(),
            "filter-date today".to_string(),
            "filter-date 1d".to_string(),
            "filter-date 7d".to_string(),
            "filter-date 2w".to_string(),
            "filter-date 1m".to_string(),
            "clear-filters".to_string(),
            "widen".to_string(),
            // Sort commands
            "sort".to_string(),
            "sort date".to_string(),
            "sort title".to_string(),
            "sort duration".to_string(),
            "sort downloaded".to_string(),
            "sort-asc".to_string(),
            "sort-desc".to_string(),
        ]);
        commands
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
            if !parts.is_empty() {
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
        if let Some(theme_part) = input_lower.strip_prefix("theme ") {
            for name in self.theme_registry.list_names() {
                if name.starts_with(theme_part) {
                    completions.push(format!("theme {}", name));
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

    fn open_episode_detail_buffer(&mut self, episode: crate::podcast::Episode) {
        self.buffer_manager
            .create_episode_detail_buffer(episode.clone());
        let episode_buffer_id = format!("episode-detail-{}", episode.id);
        let _ = self.buffer_manager.switch_to_buffer(&episode_buffer_id);
        self.update_status_bar();
        self.refresh_buffer_list_if_open();
    }

    fn add_to_playlist_supported_in_buffer(&self, buffer_id: &str) -> bool {
        buffer_id.starts_with("episodes-")
            || buffer_id.starts_with("episode-detail-")
            || buffer_id == "whats-new"
    }

    fn resolve_add_to_playlist_selection(
        &mut self,
        buffer_id: &str,
    ) -> Option<(crate::storage::PodcastId, crate::storage::EpisodeId)> {
        if buffer_id.starts_with("episodes-") {
            if let Some(episode_buffer) = self
                .buffer_manager
                .get_episode_list_buffer_mut_by_id(buffer_id)
            {
                if let Some(episode) = episode_buffer.selected_episode() {
                    return Some((episode_buffer.podcast_id.clone(), episode.id.clone()));
                }
            }
        } else if buffer_id == "whats-new" {
            if let Some(whats_new_buffer) = self.buffer_manager.get_whats_new_buffer_mut() {
                if let Some(agg_episode) = whats_new_buffer.selected_episode() {
                    return Some((
                        agg_episode.podcast_id.clone(),
                        agg_episode.episode.id.clone(),
                    ));
                }
            }
        } else if buffer_id.starts_with("episode-detail-") {
            if let Some(episode_detail_buffer) = self
                .buffer_manager
                .get_episode_detail_buffer_mut_by_id(buffer_id)
            {
                return Some((
                    episode_detail_buffer.podcast_id().clone(),
                    episode_detail_buffer.episode_id().clone(),
                ));
            }
        }

        None
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

    /// Trigger async mark episode as played
    fn trigger_async_mark_played(
        &mut self,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    ) {
        let storage = self._storage.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let episode_id_clone = episode_id.clone();

        tokio::spawn(async move {
            match storage.load_episode(&podcast_id, &episode_id).await {
                Ok(mut episode) => {
                    episode.mark_played();
                    match storage.save_episode(&podcast_id, &episode).await {
                        Ok(()) => {
                            let _ = app_event_tx.send(AppEvent::EpisodeMarkedPlayed {
                                podcast_id: podcast_id_clone,
                                episode_id: episode_id_clone,
                                episode_title,
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::EpisodeMarkPlayedFailed {
                                podcast_id: podcast_id_clone,
                                episode_id: episode_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeMarkPlayedFailed {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async mark episode as unplayed
    fn trigger_async_mark_unplayed(
        &mut self,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    ) {
        let storage = self._storage.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let episode_id_clone = episode_id.clone();

        tokio::spawn(async move {
            match storage.load_episode(&podcast_id, &episode_id).await {
                Ok(mut episode) => {
                    episode.mark_unplayed();
                    match storage.save_episode(&podcast_id, &episode).await {
                        Ok(()) => {
                            let _ = app_event_tx.send(AppEvent::EpisodeMarkedUnplayed {
                                podcast_id: podcast_id_clone,
                                episode_id: episode_id_clone,
                                episode_title,
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::EpisodeMarkUnplayedFailed {
                                podcast_id: podcast_id_clone,
                                episode_id: episode_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeMarkUnplayedFailed {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async persist of the favorited state for an episode
    fn trigger_async_toggle_favorite(
        &mut self,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
        favorited: bool,
    ) {
        let storage = self._storage.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let episode_id_clone = episode_id.clone();

        tokio::spawn(async move {
            match storage.load_episode(&podcast_id, &episode_id).await {
                Ok(mut episode) => {
                    episode.favorited = favorited;
                    match storage.save_episode(&podcast_id, &episode).await {
                        Ok(()) => {
                            let _ = app_event_tx.send(AppEvent::EpisodeFavoriteToggled {
                                podcast_id: podcast_id_clone,
                                episode_id: episode_id_clone,
                                episode_title,
                                favorited,
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::EpisodeFavoriteToggleFailed {
                                podcast_id: podcast_id_clone,
                                episode_id: episode_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeFavoriteToggleFailed {
                        podcast_id: podcast_id_clone,
                        episode_id: episode_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async persist of adding a tag to a podcast
    fn trigger_async_add_tag(&mut self, podcast_id: crate::storage::PodcastId, tag: String) {
        let storage = self._storage.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let tag_clone = tag.clone();

        tokio::spawn(async move {
            match storage.load_podcast(&podcast_id).await {
                Ok(mut podcast) => {
                    podcast.add_tag(&tag);
                    match storage.save_podcast(&podcast).await {
                        Ok(()) => {
                            let _ = app_event_tx.send(AppEvent::PodcastTagAdded {
                                podcast_id: podcast_id_clone,
                                tag: tag_clone,
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::PodcastTagAddFailed {
                                podcast_id: podcast_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PodcastTagAddFailed {
                        podcast_id: podcast_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async persist of removing a tag from a podcast
    fn trigger_async_remove_tag(&mut self, podcast_id: crate::storage::PodcastId, tag: String) {
        let storage = self._storage.clone();
        let app_event_tx = self.app_event_tx.clone();
        let podcast_id_clone = podcast_id.clone();
        let tag_clone = tag.clone();

        tokio::spawn(async move {
            match storage.load_podcast(&podcast_id).await {
                Ok(mut podcast) => {
                    podcast.remove_tag(&tag);
                    match storage.save_podcast(&podcast).await {
                        Ok(()) => {
                            let _ = app_event_tx.send(AppEvent::PodcastTagRemoved {
                                podcast_id: podcast_id_clone,
                                tag: tag_clone,
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::PodcastTagRemoveFailed {
                                podcast_id: podcast_id_clone,
                                error: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PodcastTagRemoveFailed {
                        podcast_id: podcast_id_clone,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger async cleanup of old downloads
    fn trigger_async_cleanup_downloads(&mut self, max_age_hours: u64, duration_label: String) {
        let download_manager = self.download_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let label = duration_label.clone();

        self.show_message(format!(
            "Cleaning up downloads older than {}...",
            duration_label
        ));

        tokio::spawn(async move {
            match download_manager
                .cleanup_old_downloads_hours(max_age_hours)
                .await
            {
                Ok(deleted_count) => {
                    let _ = app_event_tx.send(AppEvent::DownloadCleanupCompleted {
                        deleted_count,
                        duration_label: label,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::DownloadCleanupFailed {
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

    fn trigger_async_create_playlist(&mut self, name: String, description: Option<String>) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let name_for_error = name.clone();

        tokio::spawn(async move {
            match playlist_manager.create_playlist(&name, description).await {
                Ok(playlist) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistCreated { playlist });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistCreationFailed {
                        name: name_for_error,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_create_smart_playlist(
        &mut self,
        name: String,
        rule: crate::playlist::models::SmartPlaylistRule,
    ) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let name_for_error = name.clone();

        tokio::spawn(async move {
            match playlist_manager
                .create_smart_playlist(&name, None, rule)
                .await
            {
                Ok(playlist) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistCreated { playlist });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistCreationFailed {
                        name: name_for_error,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_evaluate_smart_playlist(
        &mut self,
        playlist_id: crate::playlist::PlaylistId,
        playlist_name: String,
        detail_buffer_id: String,
    ) {
        let storage = self._storage.clone();
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            // Load the playlist to get its rule
            let playlist = match playlist_manager.get_playlist(&playlist_id).await {
                Ok(p) => p,
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::SmartPlaylistEvaluationFailed {
                        playlist_name,
                        error: e.to_string(),
                    });
                    return;
                }
            };
            let rule = match playlist.smart_rules {
                Some(r) => r,
                None => return, // not a smart playlist, nothing to do
            };

            // Load all podcasts and episodes
            let podcast_ids = match storage.list_podcasts().await {
                Ok(ids) => ids,
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::SmartPlaylistEvaluationFailed {
                        playlist_name,
                        error: e.to_string(),
                    });
                    return;
                }
            };
            let mut all_episodes = Vec::new();
            let mut all_podcasts = Vec::new();
            for pid in &podcast_ids {
                match storage.load_podcast(pid).await {
                    Ok(podcast) => all_podcasts.push(podcast),
                    Err(e) => eprintln!(
                        "Warning: failed to load podcast '{}' for smart playlist evaluation: {}",
                        pid, e
                    ),
                }
                match storage.load_episodes(pid).await {
                    Ok(episodes) => all_episodes.extend(episodes),
                    Err(e) => eprintln!(
                        "Warning: failed to load episodes for podcast '{}' in smart playlist evaluation: {}",
                        pid, e
                    ),
                }
            }

            let raw = rule.evaluate(&all_episodes, &all_podcasts);
            // Convert Episode → PlaylistEpisode for display
            let episodes: Vec<crate::playlist::PlaylistEpisode> = raw
                .into_iter()
                .enumerate()
                .map(|(idx, ep)| crate::playlist::PlaylistEpisode {
                    podcast_id: ep.podcast_id.clone(),
                    episode_id: ep.id.clone(),
                    episode_title: Some(ep.title.clone()),
                    added_at: chrono::Utc::now(),
                    order: idx + 1,
                    file_synced: false,
                    filename: None,
                })
                .collect();
            let _ = app_event_tx.send(AppEvent::SmartPlaylistEvaluated {
                detail_buffer_id,
                episodes,
            });
        });
    }

    /// Trigger an async PodcastIndex search and load results into the discovery buffer.
    fn trigger_async_discover(&mut self, query: String, buffer_id: String) {
        let app_event_tx = self.app_event_tx.clone();
        let api_key = self.config.discovery.podcastindex_api_key.clone();
        let api_secret = self.config.discovery.podcastindex_api_secret.clone();
        let display_title = format!("Search: {}", query);

        tokio::spawn(async move {
            let client = match crate::podcast::PodcastIndexClient::new(api_key, api_secret) {
                Ok(c) => c,
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::DiscoveryLoadFailed {
                        buffer_id,
                        error: e.to_string(),
                    });
                    return;
                }
            };
            match client.search(&query).await {
                Ok(results) => {
                    let _ = app_event_tx.send(AppEvent::DiscoveryResultsLoaded {
                        buffer_id,
                        results,
                        title: display_title,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::DiscoveryLoadFailed {
                        buffer_id,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    /// Trigger an async PodcastIndex trending fetch and load results into the discovery buffer.
    fn trigger_async_trending(&mut self, buffer_id: String) {
        let app_event_tx = self.app_event_tx.clone();
        let api_key = self.config.discovery.podcastindex_api_key.clone();
        let api_secret = self.config.discovery.podcastindex_api_secret.clone();

        tokio::spawn(async move {
            let client = match crate::podcast::PodcastIndexClient::new(api_key, api_secret) {
                Ok(c) => c,
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::DiscoveryLoadFailed {
                        buffer_id,
                        error: e.to_string(),
                    });
                    return;
                }
            };
            match client.trending().await {
                Ok(results) => {
                    let _ = app_event_tx.send(AppEvent::DiscoveryResultsLoaded {
                        buffer_id,
                        results,
                        title: "Trending".to_string(),
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::DiscoveryLoadFailed {
                        buffer_id,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_delete_playlist(&mut self, playlist_id: crate::playlist::PlaylistId) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            let playlist_name = playlist_manager
                .get_playlist(&playlist_id)
                .await
                .map(|playlist| playlist.name)
                .unwrap_or_else(|_| playlist_id.to_string());

            match playlist_manager.delete_playlist(&playlist_id).await {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistDeleted {
                        name: playlist_name,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistDeletionFailed {
                        name: playlist_name,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_delete_playlist_by_name(&mut self, playlist_name: String) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let lookup_name = playlist_name.clone();

        tokio::spawn(async move {
            match playlist_manager.get_playlist_by_name(&lookup_name).await {
                Ok(playlist) => match playlist_manager.delete_playlist(&playlist.id).await {
                    Ok(_) => {
                        let _ = app_event_tx.send(AppEvent::PlaylistDeleted {
                            name: playlist.name,
                        });
                    }
                    Err(e) => {
                        let _ = app_event_tx.send(AppEvent::PlaylistDeletionFailed {
                            name: playlist.name,
                            error: e.to_string(),
                        });
                    }
                },
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistDeletionFailed {
                        name: playlist_name,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_add_to_playlist(
        &mut self,
        playlist_id: crate::playlist::PlaylistId,
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    ) {
        let playlist_manager = self.playlist_manager.clone();
        let storage = self._storage.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            let playlist_name = playlist_manager
                .get_playlist(&playlist_id)
                .await
                .map(|playlist| playlist.name)
                .unwrap_or_else(|_| playlist_id.to_string());
            let episode_title = storage
                .load_episode(&podcast_id, &episode_id)
                .await
                .map(|episode| episode.title)
                .unwrap_or_else(|_| episode_id.to_string());

            match playlist_manager
                .add_episode_to_playlist(&playlist_id, &podcast_id, &episode_id)
                .await
            {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeAddedToPlaylist {
                        playlist_name,
                        episode_title,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeAddToPlaylistFailed {
                        playlist_name,
                        episode_title,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_remove_from_playlist(
        &mut self,
        playlist_id: crate::playlist::PlaylistId,
        episode_id: crate::storage::EpisodeId,
    ) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            let playlist_name = playlist_manager
                .get_playlist(&playlist_id)
                .await
                .map(|playlist| playlist.name)
                .unwrap_or_else(|_| playlist_id.to_string());

            match playlist_manager
                .remove_episode_from_playlist(&playlist_id, &episode_id)
                .await
            {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeRemovedFromPlaylist {
                        playlist_name,
                        episode_title: episode_id.to_string(),
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::EpisodeRemoveFromPlaylistFailed {
                        playlist_name,
                        episode_title: episode_id.to_string(),
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_reorder_playlist(
        &mut self,
        playlist_id: crate::playlist::PlaylistId,
        from_idx: usize,
        to_idx: usize,
    ) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            let playlist_name = playlist_manager
                .get_playlist(&playlist_id)
                .await
                .map(|playlist| playlist.name)
                .unwrap_or_else(|_| playlist_id.to_string());
            match playlist_manager
                .reorder_episode(&playlist_id, from_idx, to_idx)
                .await
            {
                Ok(_) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistReordered {
                        name: playlist_name,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistReorderFailed {
                        name: playlist_name,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_rebuild_playlist(&mut self, playlist_id: crate::playlist::PlaylistId) {
        let playlist_manager = self.playlist_manager.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            let playlist_name = playlist_manager
                .get_playlist(&playlist_id)
                .await
                .map(|playlist| playlist.name)
                .unwrap_or_else(|_| playlist_id.to_string());

            match playlist_manager.rebuild_playlist_files(&playlist_id).await {
                Ok(result) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistRebuilt {
                        name: playlist_name,
                        rebuilt: result.rebuilt,
                        skipped: result.skipped,
                        failed: result.failed,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::PlaylistRebuildFailed {
                        name: playlist_name,
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    fn trigger_async_refresh_today(&mut self) {
        let today_generator = self.today_generator.clone();
        let app_event_tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            match today_generator.refresh().await {
                Ok(result) => {
                    let _ = app_event_tx.send(AppEvent::TodayPlaylistRefreshed {
                        added: result.added,
                        removed: result.removed,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::TodayPlaylistRefreshFailed {
                        error: e.to_string(),
                    });
                }
            }
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

    /// Get the default sync device path from config, falling back to the constant default
    fn get_default_sync_path(&self) -> String {
        self.config
            .downloads
            .sync_device_path
            .as_ref()
            .map(|p| shellexpand::tilde(p).to_string())
            .unwrap_or_else(|| crate::constants::downloads::DEFAULT_SYNC_DEVICE_PATH.to_string())
    }

    /// Parse sync command arguments, extracting optional --hard and optional device path.
    fn parse_sync_command_args(args: &[&str]) -> (Option<String>, bool) {
        let mut hard_sync = false;
        let mut path_parts = Vec::new();

        for arg in args {
            if *arg == "--hard" {
                hard_sync = true;
            } else {
                path_parts.push(*arg);
            }
        }

        if path_parts.is_empty() {
            (None, hard_sync)
        } else {
            (Some(path_parts.join(" ")), hard_sync)
        }
    }

    /// Trigger async device sync
    fn trigger_async_device_sync(
        &mut self,
        device_path_str: String,
        delete_orphans: bool,
        dry_run: bool,
        hard_sync: bool,
    ) {
        let download_manager = self.download_manager.clone();
        let app_event_tx = self.app_event_tx.clone();
        let playlists_dir = if self.config.downloads.sync_include_playlists {
            Some(self._storage.data_dir.join("Playlists"))
        } else {
            None
        };

        // Expand tilde and convert to PathBuf
        let expanded_path = shellexpand::tilde(&device_path_str).to_string();
        let device_path = std::path::PathBuf::from(expanded_path);
        let device_path_clone = device_path.clone();

        // Use config defaults if not specified
        let delete_orphans = if delete_orphans {
            true
        } else {
            self.config.downloads.sync_delete_orphans
        };

        // Send start event
        let _ = app_event_tx.send(AppEvent::DeviceSyncStarted {
            device_path: device_path.clone(),
            dry_run,
            hard_sync,
        });

        // For real syncs (not dry-run): create a progress channel and enter Progress mode.
        let progress_tx = if !dry_run {
            if let Some(sync_buffer) = self.buffer_manager.get_sync_buffer_mut() {
                sync_buffer.enter_progress_mode(device_path.clone());
            }
            let (tx, mut rx) =
                tokio::sync::mpsc::unbounded_channel::<crate::download::SyncProgressEvent>();
            // Relay progress events to app_event_tx
            let relay_tx = app_event_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = relay_tx.send(AppEvent::DeviceSyncProgress { event });
                }
            });
            Some(tx)
        } else {
            None
        };

        tokio::spawn(async move {
            match download_manager
                .sync_to_device(
                    device_path.clone(),
                    playlists_dir,
                    delete_orphans,
                    dry_run,
                    hard_sync,
                    progress_tx,
                )
                .await
            {
                Ok(report) => {
                    let _ = app_event_tx.send(AppEvent::DeviceSyncCompleted {
                        device_path,
                        report,
                        dry_run,
                    });
                }
                Err(e) => {
                    let _ = app_event_tx.send(AppEvent::DeviceSyncFailed {
                        device_path: device_path_clone,
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
        let final_path =
            if std::path::Path::new(&expanded_path).is_dir() || !expanded_path.contains('.') {
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

    /// Trigger background buffer refresh to avoid blocking UI thread
    fn trigger_background_refresh(&mut self, refresh_type: BufferRefreshType) {
        match refresh_type.clone() {
            BufferRefreshType::PodcastList => {
                let subscription_manager = self.subscription_manager.clone();
                let app_event_tx = self.app_event_tx.clone();

                tokio::spawn(async move {
                    match subscription_manager.storage.list_podcasts().await {
                        Ok(podcast_ids) => {
                            let storage = subscription_manager.storage.clone();
                            let mut indexed: Vec<(usize, _)> =
                                stream::iter(podcast_ids.into_iter().enumerate())
                                    .map(|(idx, id)| {
                                        let storage = storage.clone();
                                        async move {
                                            storage.load_podcast(&id).await.ok().map(|p| (idx, p))
                                        }
                                    })
                                    .buffer_unordered(ui_constants::REFRESH_IO_CONCURRENCY)
                                    .filter_map(|r| async { r })
                                    .collect()
                                    .await;
                            indexed.sort_unstable_by_key(|(idx, _)| *idx);
                            let podcasts: Vec<_> = indexed.into_iter().map(|(_, p)| p).collect();
                            let _ = app_event_tx.send(AppEvent::BufferDataRefreshed {
                                buffer_type: BufferRefreshType::PodcastList,
                                data: BufferRefreshData::PodcastList { podcasts },
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::BufferDataRefreshed {
                                buffer_type: BufferRefreshType::PodcastList,
                                data: BufferRefreshData::Error {
                                    message: e.to_string(),
                                },
                            });
                        }
                    }
                });
            }
            BufferRefreshType::Downloads => {
                let download_manager = self.download_manager.clone();
                let app_event_tx = self.app_event_tx.clone();

                tokio::spawn(async move {
                    // Load all (podcast, episodes) pairs concurrently
                    let downloads = if let Ok(podcast_ids) =
                        download_manager.storage().list_podcasts().await
                    {
                        let dm = download_manager.clone();
                        let mut podcast_pairs: Vec<_> =
                            stream::iter(podcast_ids.into_iter().enumerate())
                                .map(|(idx, podcast_id)| {
                                    let dm = dm.clone();
                                    async move {
                                        let podcast =
                                            dm.storage().load_podcast(&podcast_id).await.ok()?;
                                        let episodes =
                                            dm.storage().load_episodes(&podcast_id).await.ok()?;
                                        Some((idx, podcast_id, podcast, episodes))
                                    }
                                })
                                .buffer_unordered(ui_constants::REFRESH_IO_CONCURRENCY)
                                .filter_map(|r| async { r })
                                .collect()
                                .await;
                        podcast_pairs.sort_unstable_by_key(|(idx, ..)| *idx);

                        // Collect candidates (filter without I/O), then fetch metadata concurrently
                        let mut candidates: Vec<(
                            usize,
                            crate::storage::PodcastId,
                            crate::podcast::Podcast,
                            crate::podcast::Episode,
                        )> = Vec::new();
                        let mut candidate_idx = 0usize;
                        for (_, podcast_id, podcast, episodes) in podcast_pairs {
                            for episode in episodes {
                                if episode.is_downloaded()
                                    || matches!(
                                        episode.status,
                                        crate::podcast::EpisodeStatus::Downloading
                                            | crate::podcast::EpisodeStatus::DownloadFailed
                                    )
                                {
                                    candidates.push((
                                        candidate_idx,
                                        podcast_id.clone(),
                                        podcast.clone(),
                                        episode,
                                    ));
                                    candidate_idx += 1;
                                }
                            }
                        }

                        // Fetch file sizes concurrently with bounded concurrency; sort to restore order
                        let mut indexed: Vec<(usize, DownloadEntry)> = stream::iter(candidates)
                            .map(|(idx, podcast_id, podcast, episode)| async move {
                                let status = match episode.status {
                                    crate::podcast::EpisodeStatus::Downloaded => {
                                        crate::download::DownloadStatus::Completed
                                    }
                                    crate::podcast::EpisodeStatus::Downloading => {
                                        crate::download::DownloadStatus::InProgress
                                    }
                                    crate::podcast::EpisodeStatus::DownloadFailed => {
                                        crate::download::DownloadStatus::Failed(
                                            "Download failed".to_string(),
                                        )
                                    }
                                    _ => return None,
                                };
                                let file_size = match episode.local_path.as_ref() {
                                    Some(path) => {
                                        tokio::fs::metadata(path).await.ok().map(|m| m.len())
                                    }
                                    None => None,
                                };
                                Some((
                                    idx,
                                    DownloadEntry {
                                        podcast_id,
                                        episode_id: episode.id.clone(),
                                        podcast_name: podcast.title.clone(),
                                        episode_title: episode.title.clone(),
                                        status,
                                        file_path: episode.local_path.clone(),
                                        file_size,
                                    },
                                ))
                            })
                            .buffer_unordered(ui_constants::REFRESH_IO_CONCURRENCY)
                            .filter_map(|r| async { r })
                            .collect()
                            .await;
                        indexed.sort_unstable_by_key(|(idx, _)| *idx);
                        indexed.into_iter().map(|(_, entry)| entry).collect()
                    } else {
                        Vec::new()
                    };

                    let _ = app_event_tx.send(AppEvent::BufferDataRefreshed {
                        buffer_type: BufferRefreshType::Downloads,
                        data: BufferRefreshData::Downloads { downloads },
                    });
                });
            }
            BufferRefreshType::WhatsNew => {
                let subscription_manager = self.subscription_manager.clone();
                let app_event_tx = self.app_event_tx.clone();
                let episode_limit = self.config.ui.whats_new_episode_limit;

                tokio::spawn(async move {
                    // Load What's New episodes data in background
                    let mut all_episodes = Vec::new();

                    if let Ok(podcast_ids) = subscription_manager.storage.list_podcasts().await {
                        let storage = subscription_manager.storage.clone();
                        let podcast_pairs: Vec<_> = stream::iter(podcast_ids)
                            .map(|podcast_id| {
                                let storage = storage.clone();
                                async move {
                                    let podcast = storage.load_podcast(&podcast_id).await.ok()?;
                                    let episodes = storage.load_episodes(&podcast_id).await.ok()?;
                                    Some((podcast_id, podcast, episodes))
                                }
                            })
                            .buffer_unordered(ui_constants::REFRESH_IO_CONCURRENCY)
                            .filter_map(|r| async { r })
                            .collect()
                            .await;

                        // Process results (CPU only, no I/O)
                        for (podcast_id, podcast, episodes) in podcast_pairs {
                            for episode in episodes {
                                // Only show episodes that aren't downloaded or downloading
                                if !episode.is_downloaded()
                                    && !matches!(
                                        episode.status,
                                        crate::podcast::EpisodeStatus::Downloading
                                    )
                                {
                                    all_episodes.push(AggregatedEpisode {
                                        podcast_id: podcast_id.clone(),
                                        podcast_title: podcast.title.clone(),
                                        episode,
                                    });
                                }
                            }
                        }
                    }

                    // Sort by publication date (newest first)
                    all_episodes.sort_by(|a, b| b.episode.published.cmp(&a.episode.published));

                    // Apply episode limit
                    all_episodes.truncate(episode_limit);

                    let _ = app_event_tx.send(AppEvent::BufferDataRefreshed {
                        buffer_type: BufferRefreshType::WhatsNew,
                        data: BufferRefreshData::WhatsNew {
                            episodes: all_episodes,
                        },
                    });
                });
            }
            BufferRefreshType::EpisodeBuffers { podcast_id } => {
                let subscription_manager = self.subscription_manager.clone();
                let app_event_tx = self.app_event_tx.clone();

                tokio::spawn(async move {
                    match subscription_manager
                        .storage
                        .load_episodes(&podcast_id)
                        .await
                    {
                        Ok(episodes) => {
                            let _ = app_event_tx.send(AppEvent::BufferDataRefreshed {
                                buffer_type: BufferRefreshType::EpisodeBuffers {
                                    podcast_id: podcast_id.clone(),
                                },
                                data: BufferRefreshData::Episodes {
                                    podcast_id,
                                    episodes,
                                },
                            });
                        }
                        Err(e) => {
                            let _ = app_event_tx.send(AppEvent::BufferDataRefreshed {
                                buffer_type: BufferRefreshType::EpisodeBuffers { podcast_id },
                                data: BufferRefreshData::Error {
                                    message: e.to_string(),
                                },
                            });
                        }
                    }
                });
            }
            BufferRefreshType::AllEpisodeBuffers => {
                // Collect podcast IDs from all open episode buffers, then
                // trigger an individual refresh for each one.
                let buffer_ids = self.buffer_manager.get_buffer_ids();
                let mut podcast_ids: Vec<crate::storage::PodcastId> = Vec::new();
                for buffer_id in &buffer_ids {
                    if buffer_id.starts_with("episodes-") {
                        if let Some(episode_buffer) = self
                            .buffer_manager
                            .get_episode_list_buffer_mut_by_id(buffer_id)
                        {
                            podcast_ids.push(episode_buffer.podcast_id.clone());
                        }
                    }
                }
                for podcast_id in podcast_ids {
                    self.trigger_background_refresh(BufferRefreshType::EpisodeBuffers {
                        podcast_id,
                    });
                }
            }
        }
    }

    /// Handle buffer data refresh by updating buffers with pre-loaded data
    fn handle_buffer_data_refresh(
        &mut self,
        buffer_type: BufferRefreshType,
        data: BufferRefreshData,
    ) {
        match (buffer_type, data) {
            (BufferRefreshType::PodcastList, BufferRefreshData::PodcastList { podcasts }) => {
                if let Some(podcast_buffer) = self.buffer_manager.get_podcast_list_buffer_mut() {
                    podcast_buffer.set_podcasts(podcasts);
                }
            }
            (BufferRefreshType::Downloads, BufferRefreshData::Downloads { downloads }) => {
                if let Some(downloads_buffer) = self.buffer_manager.get_downloads_buffer_mut() {
                    downloads_buffer.set_downloads(downloads);
                }
            }
            (BufferRefreshType::WhatsNew, BufferRefreshData::WhatsNew { episodes }) => {
                if let Some(whats_new_buffer) = self.buffer_manager.get_whats_new_buffer_mut() {
                    let episode_count = episodes.len();
                    whats_new_buffer.set_episodes(episodes);

                    // Show message only if we're currently viewing the What's New buffer
                    if let Some(active_id) = self.buffer_manager.active_buffer_id() {
                        if active_id == "whats-new" {
                            self.show_message(format!(
                                "What's New updated with {} episode(s)",
                                episode_count
                            ));
                        }
                    }
                }
            }
            (
                BufferRefreshType::EpisodeBuffers { podcast_id },
                BufferRefreshData::Episodes { episodes, .. },
            ) => {
                let buffer_ids = self.buffer_manager.get_buffer_ids();
                for buffer_id in buffer_ids {
                    if buffer_id.starts_with("episodes-") {
                        if let Some(episode_buffer) = self
                            .buffer_manager
                            .get_episode_list_buffer_mut_by_id(&buffer_id)
                        {
                            // Check if this buffer belongs to the podcast
                            if episode_buffer.podcast_id == podcast_id {
                                episode_buffer.set_episodes(episodes.clone());
                            }
                        }
                    }
                }
            }
            (_, BufferRefreshData::Error { message }) => {
                self.show_error(format!("Could not refresh buffer: {}", message));
            }
            _ => {
                // Mismatched buffer type and data, ignore
            }
        }
    }

    async fn load_playlists_into_buffer(&mut self) {
        match self.playlist_manager.list_playlists().await {
            Ok(playlists) => {
                if let Some(buffer) = self.buffer_manager.get_playlist_list_buffer_mut() {
                    buffer.set_playlists(playlists);
                }
            }
            Err(e) => self.show_error(format!("Could not load playlists: {}", e)),
        }
    }

    async fn refresh_open_playlist_detail_buffers(&mut self) {
        let mut details_to_refresh: Vec<(String, crate::playlist::PlaylistId)> = Vec::new();
        for buffer_id in self.buffer_manager.get_buffer_ids() {
            if !buffer_id.starts_with("playlist-") || buffer_id == "playlist-list" {
                continue;
            }
            if let Some(detail_buffer) = self
                .buffer_manager
                .get_playlist_detail_buffer_mut_by_id(&buffer_id)
            {
                details_to_refresh.push((buffer_id, detail_buffer.playlist_id().clone()));
            }
        }

        for (buffer_id, playlist_id) in details_to_refresh {
            if let Ok(playlist) = self.playlist_manager.get_playlist(&playlist_id).await {
                if let Some(detail_buffer) = self
                    .buffer_manager
                    .get_playlist_detail_buffer_mut_by_id(&buffer_id)
                {
                    detail_buffer.set_playlist(playlist);
                }
            }
        }
    }

    /// Handle minibuffer input submission with context
    fn handle_minibuffer_input_with_context(
        &mut self,
        input: String,
        prompt_context: Option<String>,
    ) {
        let input = input.trim();

        if input.is_empty() {
            // Handle empty input for specific prompts
            if let Some(prompt) = &prompt_context {
                if prompt.starts_with("Export to") {
                    // Empty input means use default export path
                    let default_path =
                        shellexpand::tilde(&self.config.storage.opml_export_directory).to_string();
                    self.trigger_async_opml_export(default_path);
                    return;
                } else if prompt.starts_with("Sync to device path") {
                    // Empty input means use default sync path
                    let default_path = self.get_default_sync_path();
                    self.trigger_async_device_sync(default_path, false, false, false);
                    return;
                } else if prompt.starts_with("Dry run sync to") {
                    // Empty input means use default sync path for dry run
                    let default_path = self.get_default_sync_path();
                    self.trigger_async_device_sync(default_path, false, true, false);
                    return;
                }
            }
            return;
        }

        // Check context from prompt FIRST (before checking for URLs)
        if let Some(prompt) = &prompt_context {
            if prompt.starts_with("Search:") {
                // This is a search query — dispatch to active buffer
                if let Some(current_buffer) = self.buffer_manager.current_buffer_mut() {
                    current_buffer.handle_action(UIAction::ApplySearch {
                        query: input.to_string(),
                    });
                }
                return;
            } else if prompt.starts_with("Import OPML from") {
                // This is an OPML import
                self.trigger_async_opml_import(input.to_string());
                return;
            } else if prompt.starts_with("Export to") {
                // This is an OPML export
                self.trigger_async_opml_export(input.to_string());
                return;
            } else if prompt.starts_with("Sync to device path") {
                // This is a device sync
                let args: Vec<&str> = input.split_whitespace().collect();
                let (device_path, hard_sync) = Self::parse_sync_command_args(&args);
                let device_path = device_path.unwrap_or_else(|| self.get_default_sync_path());
                self.trigger_async_device_sync(device_path, false, false, hard_sync);
                return;
            } else if prompt.starts_with("Dry run sync to") {
                // This is a device sync dry run
                let args: Vec<&str> = input.split_whitespace().collect();
                let (device_path, hard_sync) = Self::parse_sync_command_args(&args);
                let device_path = device_path.unwrap_or_else(|| self.get_default_sync_path());
                self.trigger_async_device_sync(device_path, false, true, hard_sync);
                return;
            } else if prompt.starts_with("Create playlist:") {
                self.trigger_async_create_playlist(input.to_string(), None);
                return;
            } else if prompt.starts_with("Delete downloaded episodes older than") {
                // This is a cleanup confirmation (y/n)
                if input.to_lowercase() == "y" || input.to_lowercase() == "yes" {
                    if let Some(hours) = self.pending_cleanup_hours.take() {
                        let label = crate::utils::time::format_cleanup_duration(hours);
                        self.trigger_async_cleanup_downloads(hours, label);
                    }
                } else {
                    self.pending_cleanup_hours = None;
                    self.show_message("Download cleanup cancelled".to_string());
                }
                return;
            } else if prompt.starts_with("Delete downloads older than") {
                // This is a duration input prompt (no argument was provided)
                if let Some(total_hours) = crate::utils::time::parse_cleanup_duration(input) {
                    let label = crate::utils::time::format_cleanup_duration(total_hours);
                    self.minibuffer.set_content(MinibufferContent::Input {
                        prompt: format!("Delete downloaded episodes older than {}? (y/n) ", label),
                        input: String::new(),
                    });
                    self.pending_cleanup_hours = Some(total_hours);
                } else {
                    self.show_error(format!(
                        "Invalid duration: '{}'. Use e.g., 7d, 2w, 1m, 12h",
                        input
                    ));
                }
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
            } else if let Some(playlist_id) = self.pending_playlist_deletion.take() {
                self.trigger_async_delete_playlist(playlist_id);
            } else if self.pending_bulk_deletion {
                // Handle bulk deletion confirmation
                self.pending_bulk_deletion = false;
                self.trigger_async_delete_all_downloads();
            } else {
                self.show_message("No deletion pending".to_string());
            }
        } else if input.to_lowercase() == "n" || input.to_lowercase() == "no" {
            // Cancel deletion
            if self.pending_deletion.is_some() {
                self.pending_deletion = None;
                self.show_message("Podcast deletion cancelled".to_string());
            } else if self.pending_playlist_deletion.is_some() {
                self.pending_playlist_deletion = None;
                self.show_message("Playlist deletion cancelled".to_string());
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
                if self.buffer_manager.switch_to_buffer(buffer_id).is_err() {
                    self.show_error(format!("Could not switch to buffer: {}", buffer_name));
                } else {
                    self.update_status_bar();
                    self.show_message(format!("Switched to buffer: {}", buffer_name));
                }
            } else {
                // Treat as a command
                let _ = self.execute_command_direct(input.to_string());
            }
        }

        // Only clear the minibuffer if execute_command_direct didn't set up a new prompt.
        // Commands like clean-older-than and delete-all-downloads set the minibuffer to
        // a confirmation prompt, which would be wiped out by an unconditional clear().
        if !self.minibuffer.is_input_mode() {
            self.minibuffer.clear();
        }
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
                self.pending_playlist_deletion = None;
                self.pending_bulk_deletion = false;
                self.pending_cleanup_hours = None;
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

/// Parse `:smart-playlist` command arguments into a `SmartPlaylistRule`.
///
/// Supported flags:
/// - `--filter <spec>` — one or more; multiple filters are AND-ed
/// - `--sort <field>`  — `date-asc` | `date-desc` | `title-asc` | `title-desc`
/// - `--limit <n>`     — maximum episodes to return
///
/// Filter specs: `downloaded`, `favorited`, `played`, `unplayed`,
///               `tag:<name>`, `podcast:<id>`, `newer-than:<days>`
pub(crate) fn parse_smart_playlist_args(
    args: &[&str],
) -> Result<crate::playlist::models::SmartPlaylistRule, String> {
    use crate::playlist::models::{SmartFilter, SmartPlaylistRule, SmartSort};

    let mut filters: Vec<SmartFilter> = Vec::new();
    let mut sort: Option<SmartSort> = None;
    let mut limit: Option<usize> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--filter" => {
                i += 1;
                if i >= args.len() {
                    return Err("--filter requires a value".to_string());
                }
                let spec = args[i];
                let filter = parse_filter_spec(spec)?;
                filters.push(filter);
            }
            "--sort" => {
                i += 1;
                if i >= args.len() {
                    return Err("--sort requires a value".to_string());
                }
                sort = Some(parse_sort_spec(args[i])?);
            }
            "--limit" => {
                i += 1;
                if i >= args.len() {
                    return Err("--limit requires a value".to_string());
                }
                limit =
                    Some(args[i].parse::<usize>().map_err(|_| {
                        format!("--limit value must be a number, got '{}'", args[i])
                    })?);
            }
            other => {
                return Err(format!(
                    "Unknown argument '{}'. Use --filter, --sort, or --limit",
                    other
                ));
            }
        }
        i += 1;
    }

    let filter = match filters.len() {
        0 => SmartFilter::Downloaded, // sensible default
        1 => filters.remove(0),
        _ => SmartFilter::And(filters),
    };

    Ok(SmartPlaylistRule {
        filter,
        sort,
        limit,
    })
}

fn parse_filter_spec(spec: &str) -> Result<crate::playlist::models::SmartFilter, String> {
    use crate::playlist::models::SmartFilter;
    match spec {
        "downloaded" => Ok(SmartFilter::Downloaded),
        "favorited" => Ok(SmartFilter::Favorited),
        "played" => Ok(SmartFilter::Played),
        "unplayed" => Ok(SmartFilter::Unplayed),
        other => {
            if let Some(tag) = other.strip_prefix("tag:") {
                Ok(SmartFilter::Tag(tag.to_string()))
            } else if let Some(id) = other.strip_prefix("podcast:") {
                Ok(SmartFilter::Podcast(id.to_string()))
            } else if let Some(days_str) = other.strip_prefix("newer-than:") {
                let days = days_str.parse::<u32>().map_err(|_| {
                    format!(
                        "newer-than value must be a number of days, got '{}'",
                        days_str
                    )
                })?;
                Ok(SmartFilter::NewerThan(days))
            } else {
                Err(format!(
                    "Unknown filter '{}'. Valid: downloaded, favorited, played, unplayed, tag:<name>, podcast:<id>, newer-than:<days>",
                    other
                ))
            }
        }
    }
}

fn parse_sort_spec(spec: &str) -> Result<crate::playlist::models::SmartSort, String> {
    use crate::playlist::models::{SmartSort, SmartSortDirection, SmartSortField};
    match spec {
        "date-desc" => Ok(SmartSort {
            field: SmartSortField::Date,
            direction: SmartSortDirection::Descending,
        }),
        "date-asc" => Ok(SmartSort {
            field: SmartSortField::Date,
            direction: SmartSortDirection::Ascending,
        }),
        "title-asc" => Ok(SmartSort {
            field: SmartSortField::Title,
            direction: SmartSortDirection::Ascending,
        }),
        "title-desc" => Ok(SmartSort {
            field: SmartSortField::Title,
            direction: SmartSortDirection::Descending,
        }),
        "duration-asc" => Ok(SmartSort {
            field: SmartSortField::Duration,
            direction: SmartSortDirection::Ascending,
        }),
        "duration-desc" => Ok(SmartSort {
            field: SmartSortField::Duration,
            direction: SmartSortDirection::Descending,
        }),
        other => Err(format!(
            "Unknown sort '{}'. Valid: date-desc, date-asc, title-asc, title-desc, duration-asc, duration-desc",
            other
        )),
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
        assert_eq!(
            app.buffer_manager.current_buffer_name().unwrap(),
            "*Help: Keybindings*"
        );
    }

    #[tokio::test]
    async fn test_show_help_reopens_after_close() {
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

        // Open help buffer
        let result = app.handle_action(UIAction::ShowHelp).await;
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_name().unwrap(),
            "*Help: Keybindings*"
        );

        // Close the help buffer
        let help_id = app.buffer_manager.current_buffer_id().unwrap();
        let _ = app.buffer_manager.remove_buffer(&help_id);

        // Verify help buffer is gone
        let has_help = app
            .buffer_manager
            .buffer_names()
            .iter()
            .any(|(_, name)| name == "*Help: Keybindings*");
        assert!(!has_help, "Help buffer should be removed");

        // Reopen help buffer via ShowHelp action
        let result = app.handle_action(UIAction::ShowHelp).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Should have recreated and switched to help buffer
        assert_eq!(
            app.buffer_manager.current_buffer_name().unwrap(),
            "*Help: Keybindings*"
        );
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
        // Verify theme propagated to app
        assert_eq!(app.theme.name, "Light");

        let result = app.set_theme_direct("invalid-theme");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_add_to_playlist_opens_picker_from_supported_buffers() {
        use crate::config::DownloadConfig;
        use crate::podcast::Episode;
        use crate::storage::JsonStorage;
        use chrono::Utc;
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

        app.playlist_manager
            .create_playlist("Issue55 Playlist", None)
            .await
            .unwrap();

        let podcast_id = crate::storage::PodcastId::new();
        let episode = Episode::new(
            podcast_id.clone(),
            "Issue55 Episode".to_string(),
            "https://example.com/issue55.mp3".to_string(),
            Utc::now(),
        );

        let episode_buffer_id = "episodes-issue55-podcast".to_string();
        app.buffer_manager.create_episode_list_buffer(
            "Issue55 Podcast".to_string(),
            podcast_id.clone(),
            app.subscription_manager.clone(),
            app.download_manager.clone(),
        );
        if let Some(episode_buffer) = app
            .buffer_manager
            .get_episode_list_buffer_mut_by_id(&episode_buffer_id)
        {
            episode_buffer.set_episodes(vec![episode.clone()]);
        }
        app.buffer_manager
            .switch_to_buffer(&episode_buffer_id)
            .unwrap();
        let result = app.handle_action(UIAction::AddToPlaylist).await;
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some("playlist-picker".to_string())
        );

        if let Some(whats_new_buffer) = app.buffer_manager.get_whats_new_buffer_mut() {
            whats_new_buffer.set_episodes(vec![crate::ui::events::AggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Issue55 Podcast".to_string(),
                episode: episode.clone(),
            }]);
        }
        app.buffer_manager
            .switch_to_buffer(&"whats-new".to_string())
            .unwrap();
        let result = app.handle_action(UIAction::AddToPlaylist).await;
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some("playlist-picker".to_string())
        );

        let episode_detail_id = format!("episode-detail-{}", episode.id);
        app.buffer_manager
            .create_episode_detail_buffer(episode.clone());
        app.buffer_manager
            .switch_to_buffer(&episode_detail_id)
            .unwrap();
        let result = app.handle_action(UIAction::AddToPlaylist).await;
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some("playlist-picker".to_string())
        );
    }

    #[tokio::test]
    async fn test_add_to_playlist_in_unsupported_buffer_shows_info_message() {
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

        app.buffer_manager
            .switch_to_buffer(&"podcast-list".to_string())
            .unwrap();
        let result = app.handle_action(UIAction::AddToPlaylist).await;
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some("podcast-list".to_string())
        );
        assert_ne!(
            app.buffer_manager.current_buffer_id(),
            Some("playlist-picker".to_string())
        );
        assert!(app.minibuffer.is_visible());
    }

    #[tokio::test]
    async fn test_playlist_detail_select_item_opens_episode_detail() {
        use crate::config::DownloadConfig;
        use crate::playlist::{Playlist, PlaylistEpisode, PlaylistId, PlaylistType};
        use crate::podcast::{Episode, Podcast};
        use crate::storage::JsonStorage;
        use chrono::Utc;
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
            storage.clone(),
            app_event_tx,
        )
        .unwrap();
        app.initialize().await.unwrap();

        let mut podcast = Podcast::new(
            "Test Podcast".to_string(),
            "https://example.com/feed.xml".to_string(),
        );
        let podcast_id = podcast.id.clone();
        storage.save_podcast(&podcast).await.unwrap();

        let episode = Episode::new(
            podcast_id.clone(),
            "Episode 1".to_string(),
            "https://example.com/ep1.mp3".to_string(),
            Utc::now(),
        );
        let episode_id = episode.id.clone();
        podcast.add_episode(episode_id.clone());
        storage.save_episode(&podcast_id, &episode).await.unwrap();
        storage.save_podcast(&podcast).await.unwrap();

        let playlist_id = PlaylistId::new();
        let playlist_name = "Test Playlist".to_string();
        let detail_id = format!(
            "playlist-{}",
            playlist_name.replace(' ', "-").to_lowercase()
        );
        let playlist_manager = app.playlist_manager.clone();
        app.buffer_manager.create_playlist_detail_buffer(
            playlist_id.clone(),
            playlist_name.clone(),
            PlaylistType::User,
            playlist_manager,
        );
        if let Some(detail_buffer) = app
            .buffer_manager
            .get_playlist_detail_buffer_mut_by_id(&detail_id)
        {
            detail_buffer.set_playlist(Playlist {
                id: playlist_id,
                name: playlist_name,
                description: None,
                playlist_type: PlaylistType::User,
                episodes: vec![PlaylistEpisode {
                    podcast_id: podcast_id.clone(),
                    episode_id: episode_id.clone(),
                    episode_title: Some("Episode 1".to_string()),
                    added_at: Utc::now(),
                    order: 1,
                    file_synced: false,
                    filename: None,
                }],
                created: Utc::now(),
                last_updated: Utc::now(),
                smart_rules: None,
            });
        }
        app.buffer_manager.switch_to_buffer(&detail_id).unwrap();

        let result = app.handle_action(UIAction::SelectItem).await;
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some(format!("episode-detail-{}", episode_id))
        );
    }

    #[tokio::test]
    async fn test_playlist_detail_select_item_with_missing_episode_shows_error() {
        use crate::config::DownloadConfig;
        use crate::playlist::{Playlist, PlaylistEpisode, PlaylistId, PlaylistType};
        use crate::podcast::Podcast;
        use crate::storage::JsonStorage;
        use chrono::Utc;
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
            storage.clone(),
            app_event_tx,
        )
        .unwrap();
        app.initialize().await.unwrap();

        let podcast = Podcast::new(
            "Test Podcast".to_string(),
            "https://example.com/feed.xml".to_string(),
        );
        let podcast_id = podcast.id.clone();
        storage.save_podcast(&podcast).await.unwrap();
        let missing_episode_id = crate::storage::EpisodeId::new();

        let playlist_id = PlaylistId::new();
        let playlist_name = "Broken Playlist".to_string();
        let detail_id = format!(
            "playlist-{}",
            playlist_name.replace(' ', "-").to_lowercase()
        );
        let playlist_manager = app.playlist_manager.clone();
        app.buffer_manager.create_playlist_detail_buffer(
            playlist_id.clone(),
            playlist_name.clone(),
            PlaylistType::User,
            playlist_manager,
        );
        if let Some(detail_buffer) = app
            .buffer_manager
            .get_playlist_detail_buffer_mut_by_id(&detail_id)
        {
            detail_buffer.set_playlist(Playlist {
                id: playlist_id,
                name: playlist_name,
                description: None,
                playlist_type: PlaylistType::User,
                episodes: vec![PlaylistEpisode {
                    podcast_id,
                    episode_id: missing_episode_id,
                    episode_title: Some("Missing".to_string()),
                    added_at: Utc::now(),
                    order: 1,
                    file_synced: false,
                    filename: None,
                }],
                created: Utc::now(),
                last_updated: Utc::now(),
                smart_rules: None,
            });
        }
        app.buffer_manager.switch_to_buffer(&detail_id).unwrap();

        let result = app.handle_action(UIAction::SelectItem).await;
        assert!(result.is_ok());
        assert_eq!(app.buffer_manager.current_buffer_id(), Some(detail_id));
        assert!(app.minibuffer.is_visible());
    }

    /// Helper: build a minimal UIApp with the sync buffer registered and active.
    async fn make_app_with_sync_buffer() -> UIApp {
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.keep();

        let storage = Arc::new(JsonStorage::with_data_dir(temp_path.clone()));
        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_path.clone(),
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
            download_manager.clone(),
            storage,
            app_event_tx,
        )
        .unwrap();
        app.initialize().await.unwrap();

        // Switch to the sync buffer (now created by initialize())
        app.buffer_manager
            .switch_to_buffer(&"sync".to_string())
            .unwrap();

        app
    }

    #[tokio::test]
    async fn test_prompt_input_from_sync_buffer_opens_minibuffer() {
        // Arrange
        let mut app = make_app_with_sync_buffer().await;

        // Act — SyncToDevice falls through to catch-all → sync buffer returns
        // PromptInput → new PromptInput arm opens the minibuffer
        let result = app.handle_action(UIAction::SyncToDevice).await;

        // Assert
        assert!(result.is_ok());
        assert!(
            app.minibuffer.is_input_mode(),
            "Minibuffer should be in input mode after SyncToDevice"
        );
        assert!(
            app.minibuffer
                .current_prompt()
                .map(|p| p.starts_with("Sync to device path"))
                .unwrap_or(false),
            "Prompt should start with 'Sync to device path'"
        );
    }

    #[tokio::test]
    async fn test_download_episode_in_sync_buffer_opens_dry_run_prompt() {
        // Arrange — 'D' maps to DownloadEpisode; sync buffer context guard intercepts it
        let mut app = make_app_with_sync_buffer().await;

        // Act
        let result = app.handle_action(UIAction::DownloadEpisode).await;

        // Assert — should open dry-run prompt, not download-episode flow
        assert!(result.is_ok());
        assert!(
            app.minibuffer.is_input_mode(),
            "Minibuffer should be in input mode after 'D' in sync buffer"
        );
        assert!(
            app.minibuffer
                .current_prompt()
                .map(|p| p.starts_with("Dry run sync to"))
                .unwrap_or(false),
            "Prompt should start with 'Dry run sync to'"
        );
    }

    #[tokio::test]
    async fn test_delete_podcast_in_sync_buffer_shows_nothing_to_delete() {
        // Arrange — 'd' now consistently means delete; sync buffer has nothing to delete
        let mut app = make_app_with_sync_buffer().await;

        // Act
        let result = app.handle_action(UIAction::DeletePodcast).await;

        // Assert — should show info message, not open a dry-run prompt
        assert!(result.is_ok());
        assert!(
            !app.minibuffer.is_input_mode(),
            "Minibuffer should NOT enter input mode for 'd' in sync buffer"
        );
        // Verify show_message was actually called (not a silent no-op)
        assert!(
            app.minibuffer.is_visible(),
            "Minibuffer should be visible (showing 'nothing to delete' message)"
        );
    }

    #[tokio::test]
    async fn test_refresh_podcast_in_sync_buffer_is_noop() {
        // Arrange — 'r' maps to RefreshPodcast; sync buffer context guard short-circuits it
        let mut app = make_app_with_sync_buffer().await;

        // Act
        let result = app.handle_action(UIAction::RefreshPodcast).await;

        // Assert — should return cleanly without touching the podcast refresh flow
        assert!(result.is_ok());
        assert!(result.unwrap(), "Should return true (continue running)");
        // Minibuffer should not have been opened for a prompt
        assert!(
            !app.minibuffer.is_input_mode(),
            "Minibuffer should NOT enter input mode for 'r' in sync buffer"
        );
        // Buffer should still be sync
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some("sync".to_string())
        );
    }

    #[tokio::test]
    async fn test_initialize_creates_sync_buffer() {
        // Arrange — build UIApp via new() which leaves BufferManager empty,
        // then call initialize() (the real startup path from App::run)
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.keep();

        let storage = Arc::new(JsonStorage::with_data_dir(temp_path.clone()));
        let download_manager = Arc::new(
            DownloadManager::new(storage.clone(), temp_path, DownloadConfig::default()).unwrap(),
        );
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));
        let (app_event_tx, _rx) = mpsc::unbounded_channel();
        let mut app = UIApp::new(
            config,
            subscription_manager,
            download_manager,
            storage,
            app_event_tx,
        )
        .unwrap();

        // Act
        app.initialize().await.unwrap();

        // Assert — sync buffer must be registered and switchable
        let buffer_ids = app.buffer_manager.get_buffer_ids();
        assert!(
            buffer_ids.contains(&"sync".to_string()),
            "initialize() must create the sync buffer; got: {:?}",
            buffer_ids
        );
        assert!(
            app.buffer_manager
                .switch_to_buffer(&"sync".to_string())
                .is_ok(),
            "Should be able to switch to sync buffer after initialize()"
        );
    }

    // ── parse_smart_playlist_args / parse_filter_spec / parse_sort_spec ────

    #[test]
    fn test_parse_smart_playlist_args_empty_defaults_to_downloaded() {
        // Arrange
        let args: &[&str] = &[];
        // Act
        let rule = parse_smart_playlist_args(args).unwrap();
        // Assert
        use crate::playlist::models::SmartFilter;
        assert_eq!(rule.filter, SmartFilter::Downloaded);
        assert!(rule.sort.is_none());
        assert!(rule.limit.is_none());
    }

    #[test]
    fn test_parse_smart_playlist_args_single_filter_used_directly() {
        // Arrange / Act
        let rule = parse_smart_playlist_args(&["--filter", "favorited"]).unwrap();
        // Assert
        use crate::playlist::models::SmartFilter;
        assert_eq!(rule.filter, SmartFilter::Favorited);
    }

    #[test]
    fn test_parse_smart_playlist_args_multiple_filters_become_and() {
        // Arrange / Act
        let rule = parse_smart_playlist_args(&["--filter", "downloaded", "--filter", "favorited"])
            .unwrap();
        // Assert
        use crate::playlist::models::SmartFilter;
        assert_eq!(
            rule.filter,
            SmartFilter::And(vec![SmartFilter::Downloaded, SmartFilter::Favorited])
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_sort_date_desc() {
        use crate::playlist::models::{SmartSort, SmartSortDirection, SmartSortField};
        let rule = parse_smart_playlist_args(&["--sort", "date-desc"]).unwrap();
        assert_eq!(
            rule.sort,
            Some(SmartSort {
                field: SmartSortField::Date,
                direction: SmartSortDirection::Descending,
            })
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_sort_duration_asc() {
        use crate::playlist::models::{SmartSort, SmartSortDirection, SmartSortField};
        let rule = parse_smart_playlist_args(&["--sort", "duration-asc"]).unwrap();
        assert_eq!(
            rule.sort,
            Some(SmartSort {
                field: SmartSortField::Duration,
                direction: SmartSortDirection::Ascending,
            })
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_limit_valid() {
        // Arrange / Act
        let rule = parse_smart_playlist_args(&["--limit", "25"]).unwrap();
        // Assert
        assert_eq!(rule.limit, Some(25));
    }

    #[test]
    fn test_parse_smart_playlist_args_limit_invalid_returns_error() {
        // Arrange / Act
        let err = parse_smart_playlist_args(&["--limit", "abc"]).unwrap_err();
        // Assert
        assert!(
            err.contains("number"),
            "Error should mention 'number': {err}"
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_unknown_flag_returns_error() {
        let err = parse_smart_playlist_args(&["--unknown", "foo"]).unwrap_err();
        assert!(
            err.contains("Unknown argument"),
            "Error should say 'Unknown argument': {err}"
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_filter_tag() {
        use crate::playlist::models::SmartFilter;
        let rule = parse_smart_playlist_args(&["--filter", "tag:rust"]).unwrap();
        assert_eq!(rule.filter, SmartFilter::Tag("rust".to_string()));
    }

    #[test]
    fn test_parse_smart_playlist_args_filter_newer_than_valid() {
        use crate::playlist::models::SmartFilter;
        let rule = parse_smart_playlist_args(&["--filter", "newer-than:7"]).unwrap();
        assert_eq!(rule.filter, SmartFilter::NewerThan(7));
    }

    #[test]
    fn test_parse_smart_playlist_args_filter_newer_than_invalid() {
        let err = parse_smart_playlist_args(&["--filter", "newer-than:notanumber"]).unwrap_err();
        assert!(
            err.contains("number of days"),
            "Error should mention 'number of days': {err}"
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_filter_unknown_returns_error() {
        let err = parse_smart_playlist_args(&["--filter", "bogus"]).unwrap_err();
        assert!(
            err.contains("Unknown filter"),
            "Error should say 'Unknown filter': {err}"
        );
    }

    #[test]
    fn test_parse_smart_playlist_args_all_sort_variants() {
        for spec in &[
            "date-asc",
            "date-desc",
            "title-asc",
            "title-desc",
            "duration-asc",
            "duration-desc",
        ] {
            let result = parse_smart_playlist_args(&["--sort", spec]);
            assert!(
                result.is_ok(),
                "Sort spec '{spec}' should parse successfully"
            );
        }
    }

    #[test]
    fn test_parse_smart_playlist_args_sort_unknown_returns_error() {
        let err = parse_smart_playlist_args(&["--sort", "random"]).unwrap_err();
        assert!(
            err.contains("Unknown sort"),
            "Error should say 'Unknown sort': {err}"
        );
    }

    // ── AudioManager wiring tests (#141) ─────────────────────────────────────

    /// Creates a UIApp backed by real storage (initialized).
    /// Returns both the app and the storage so tests can pre-populate data.
    async fn make_test_app_with_storage() -> (UIApp, Arc<crate::storage::JsonStorage>) {
        use crate::config::DownloadConfig;
        use crate::storage::JsonStorage;
        use tempfile::TempDir;

        let config = Config::default();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.keep();

        let storage = Arc::new(JsonStorage::with_data_dir(temp_path.clone()));
        storage.initialize().await.unwrap();

        let download_manager = Arc::new(
            DownloadManager::new(
                storage.clone(),
                temp_path.clone(),
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
            storage.clone(),
            app_event_tx,
        )
        .unwrap();
        app.initialize().await.unwrap();

        (app, storage)
    }

    async fn make_test_app() -> UIApp {
        make_test_app_with_storage().await.0
    }

    #[tokio::test]
    async fn test_handle_action_toggle_play_pause_sends_command_when_audio_available() {
        // Arrange
        let mut app = make_test_app().await;
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<AudioCommand>();
        app.set_audio_command_tx(audio_tx);

        // Act
        let result = app.handle_action(UIAction::TogglePlayPause).await;

        // Assert
        assert!(result.is_ok());
        let cmd = audio_rx
            .try_recv()
            .expect("Expected AudioCommand to be sent");
        assert!(
            matches!(cmd, AudioCommand::TogglePlayPause),
            "Expected TogglePlayPause, got {cmd:?}"
        );
    }

    #[tokio::test]
    async fn test_handle_action_toggle_play_pause_blocked_in_minibuffer_input_mode() {
        // Arrange
        let mut app = make_test_app().await;
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<AudioCommand>();
        app.set_audio_command_tx(audio_tx);
        app.minibuffer.show_prompt("test: ".to_string());
        assert!(app.minibuffer.is_input_mode());

        // Act
        let result = app.handle_action(UIAction::TogglePlayPause).await;

        // Assert — no command should be dispatched while minibuffer is in input mode
        assert!(result.is_ok());
        assert!(
            audio_rx.try_recv().is_err(),
            "No AudioCommand should be sent when minibuffer is in input mode"
        );
    }

    #[tokio::test]
    async fn test_handle_action_stop_playback_sends_stop_command() {
        // Arrange
        let mut app = make_test_app().await;
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<AudioCommand>();
        app.set_audio_command_tx(audio_tx);

        // Act
        let result = app.handle_action(UIAction::StopPlayback).await;

        // Assert
        assert!(result.is_ok());
        let cmd = audio_rx.try_recv().expect("Expected AudioCommand::Stop");
        assert!(
            matches!(cmd, AudioCommand::Stop),
            "Expected Stop, got {cmd:?}"
        );
    }

    #[tokio::test]
    async fn test_handle_action_volume_up_sends_volume_up_command() {
        // Arrange
        let mut app = make_test_app().await;
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<AudioCommand>();
        app.set_audio_command_tx(audio_tx);

        // Act
        let result = app.handle_action(UIAction::VolumeUp).await;

        // Assert
        assert!(result.is_ok());
        let cmd = audio_rx
            .try_recv()
            .expect("Expected AudioCommand::VolumeUp");
        assert!(
            matches!(cmd, AudioCommand::VolumeUp),
            "Expected VolumeUp, got {cmd:?}"
        );
    }

    #[tokio::test]
    async fn test_handle_app_event_track_ended_marks_episode_played_and_persists() {
        use crate::podcast::{Episode, Podcast};
        use crate::storage::Storage;
        use chrono::Utc;

        // Arrange
        let (mut app, storage) = make_test_app_with_storage().await;

        let mut podcast = Podcast::new(
            "Test Podcast".to_string(),
            "http://example.com/feed.xml".to_string(),
        );
        let mut episode = Episode::new(
            podcast.id.clone(),
            "Test Episode".to_string(),
            "http://example.com/ep1.mp3".to_string(),
            Utc::now(),
        );
        episode.duration = Some(3600);
        podcast.episodes.push(episode.id.clone());
        storage.save_podcast(&podcast).await.unwrap(); // unwrap OK — test setup
        storage.save_episode(&podcast.id, &episode).await.unwrap(); // unwrap OK — test setup

        // Act
        app.handle_app_event(AppEvent::TrackEnded {
            podcast_id: podcast.id.clone(),
            episode_id: episode.id.clone(),
        })
        .await
        .unwrap(); // unwrap OK — testing success path

        // Assert — reload from storage and verify persistence
        let saved = storage
            .load_episode(&podcast.id, &episode.id)
            .await
            .unwrap(); // unwrap OK — episode was saved above
        assert!(saved.is_played(), "Episode should be marked as played");
        assert_eq!(
            saved.last_played_position,
            Some(3600),
            "Position should be set to full duration"
        );
        assert_eq!(saved.play_count, 1, "play_count should be incremented to 1");
    }

    #[tokio::test]
    async fn test_handle_app_event_track_ended_marks_played_when_no_duration() {
        use crate::podcast::{Episode, Podcast};
        use crate::storage::Storage;
        use chrono::Utc;

        // Arrange
        let (mut app, storage) = make_test_app_with_storage().await;

        let mut podcast = Podcast::new(
            "Test Podcast".to_string(),
            "http://example.com/feed.xml".to_string(),
        );
        let episode = Episode::new(
            podcast.id.clone(),
            "Test Episode No Duration".to_string(),
            "http://example.com/ep2.mp3".to_string(),
            Utc::now(),
        );
        // No duration set — episode.duration is None
        podcast.episodes.push(episode.id.clone());
        storage.save_podcast(&podcast).await.unwrap(); // unwrap OK — test setup
        storage.save_episode(&podcast.id, &episode).await.unwrap(); // unwrap OK — test setup

        // Act
        app.handle_app_event(AppEvent::TrackEnded {
            podcast_id: podcast.id.clone(),
            episode_id: episode.id.clone(),
        })
        .await
        .unwrap(); // unwrap OK — testing success path

        // Assert — mark_played() is called when no duration is available
        let saved = storage
            .load_episode(&podcast.id, &episode.id)
            .await
            .unwrap(); // unwrap OK — episode was saved above
        assert!(
            saved.is_played(),
            "Episode with no duration should still be marked played"
        );
        assert_eq!(saved.play_count, 1);
    }

    #[tokio::test]
    async fn test_handle_app_event_playback_started_sets_now_playing_info() {
        use crate::podcast::{Episode, Podcast};
        use crate::storage::Storage;
        use chrono::Utc;

        // Arrange
        let (mut app, storage) = make_test_app_with_storage().await;

        let mut podcast = Podcast::new(
            "My Podcast".to_string(),
            "http://example.com/feed.xml".to_string(),
        );
        let episode = Episode::new(
            podcast.id.clone(),
            "My Episode".to_string(),
            "http://example.com/ep1.mp3".to_string(),
            Utc::now(),
        );
        podcast.episodes.push(episode.id.clone());
        storage.save_podcast(&podcast).await.unwrap(); // unwrap OK — test setup
        storage.save_episode(&podcast.id, &episode).await.unwrap(); // unwrap OK — test setup

        // Act
        app.handle_app_event(AppEvent::PlaybackStarted {
            podcast_id: podcast.id.clone(),
            episode_id: episode.id.clone(),
        })
        .await
        .unwrap(); // unwrap OK — testing success path

        // Assert — NowPlaying buffer should display the episode and podcast names
        let buf = app
            .buffer_manager
            .get_now_playing_buffer_mut()
            .expect("NowPlaying buffer should exist after initialize()");
        assert_eq!(buf.episode_title(), Some("My Episode"));
        assert_eq!(buf.podcast_name(), Some("My Podcast"));
    }

    #[tokio::test]
    async fn test_handle_app_event_playback_error_shows_error_message() {
        // Arrange
        let mut app = make_test_app().await;

        // Act
        app.handle_app_event(AppEvent::PlaybackError {
            error: "device unavailable".to_string(),
        })
        .await
        .unwrap(); // unwrap OK — testing success path

        // Assert
        assert!(
            app.minibuffer.is_visible(),
            "Minibuffer should show error after PlaybackError"
        );
        assert!(
            !app.minibuffer.is_input_mode(),
            "Minibuffer should be in message/error display mode, not input mode"
        );
    }

    #[tokio::test]
    async fn test_tag_command_routes_to_podcast_list_not_active_buffer() {
        use crate::podcast::Podcast;

        // Arrange — seed podcast list buffer, then make episode list the active buffer
        let mut app = make_test_app().await;
        let podcast = Podcast::new(
            "Test Podcast".to_string(),
            "https://example.com/feed.xml".to_string(),
        );
        let podcast_id = podcast.id.clone();
        if let Some(buf) = app.buffer_manager.get_podcast_list_buffer_mut() {
            buf.set_podcasts(vec![podcast]);
        }
        app.buffer_manager.create_episode_list_buffer(
            "Test Podcast".to_string(),
            podcast_id.clone(),
            app.subscription_manager.clone(),
            app.download_manager.clone(),
        );
        // Switch to an episode list so podcast-list is NOT the active buffer
        let _ = app
            .buffer_manager
            .switch_to_buffer(&format!("episodes-{}", podcast_id));

        // Act
        let result = app.execute_command_direct("tag tech".to_string());

        // Assert — command succeeded and tag was added to the podcast in the podcast list buffer
        assert!(result.is_ok());
        if let Some(buf) = app.buffer_manager.get_podcast_list_buffer_mut() {
            let tagged = buf
                .podcasts()
                .iter()
                .any(|p| p.id == podcast_id && p.has_tag("tech"));
            assert!(
                tagged,
                "Expected podcast to have tag 'tech' after :tag command"
            );
        }
    }

    #[tokio::test]
    async fn test_untag_command_routes_to_podcast_list_not_active_buffer() {
        use crate::podcast::Podcast;

        // Arrange — seed podcast list with a tagged podcast, make episode list active
        let mut app = make_test_app().await;
        let mut podcast = Podcast::new(
            "Test Podcast".to_string(),
            "https://example.com/feed.xml".to_string(),
        );
        podcast.add_tag("tech");
        let podcast_id = podcast.id.clone();
        if let Some(buf) = app.buffer_manager.get_podcast_list_buffer_mut() {
            buf.set_podcasts(vec![podcast]);
        }
        app.buffer_manager.create_episode_list_buffer(
            "Test Podcast".to_string(),
            podcast_id.clone(),
            app.subscription_manager.clone(),
            app.download_manager.clone(),
        );
        // Switch to episode list so podcast-list is NOT the active buffer
        let _ = app
            .buffer_manager
            .switch_to_buffer(&format!("episodes-{}", podcast_id));

        // Act
        let result = app.execute_command_direct("untag tech".to_string());

        // Assert — tag was removed from the podcast in the podcast list buffer
        assert!(result.is_ok());
        if let Some(buf) = app.buffer_manager.get_podcast_list_buffer_mut() {
            let still_tagged = buf
                .podcasts()
                .iter()
                .any(|p| p.id == podcast_id && p.has_tag("tech"));
            assert!(
                !still_tagged,
                "Expected 'tech' tag to be removed after :untag command"
            );
        }
    }

    #[tokio::test]
    async fn test_filter_tag_routes_to_podcast_list_and_switches_buffer() {
        use crate::podcast::Podcast;

        // Arrange — 2 podcasts: one tagged "tech", one untagged; episode list is active
        let mut app = make_test_app().await;
        let mut tagged_podcast = Podcast::new(
            "Tech Podcast".to_string(),
            "https://example.com/tech.xml".to_string(),
        );
        tagged_podcast.add_tag("tech");
        let tagged_id = tagged_podcast.id.clone();

        let other_podcast = Podcast::new(
            "News Podcast".to_string(),
            "https://example.com/news.xml".to_string(),
        );
        if let Some(buf) = app.buffer_manager.get_podcast_list_buffer_mut() {
            buf.set_podcasts(vec![tagged_podcast, other_podcast]);
        }
        app.buffer_manager.create_episode_list_buffer(
            "Tech Podcast".to_string(),
            tagged_id.clone(),
            app.subscription_manager.clone(),
            app.download_manager.clone(),
        );
        // Switch to episode list so podcast-list is NOT the active buffer
        let _ = app
            .buffer_manager
            .switch_to_buffer(&format!("episodes-{}", tagged_id));
        assert_ne!(
            app.buffer_manager.current_buffer_id(),
            Some("podcast-list".to_string()),
            "Precondition: episode list should be active, not podcast list"
        );

        // Act
        let result = app.execute_command_direct("filter-tag tech".to_string());

        // Assert — command succeeded AND we switched to podcast-list
        assert!(result.is_ok());
        assert_eq!(
            app.buffer_manager.current_buffer_id(),
            Some("podcast-list".to_string()),
            "Expected :filter-tag to switch to podcast list buffer"
        );
        // Only the tagged podcast should be visible after filter
        if let Some(buf) = app.buffer_manager.get_podcast_list_buffer_mut() {
            // selected_podcast() returns Some only if a filtered result exists
            let selected = buf.selected_podcast().map(|p| p.id.clone());
            assert_eq!(
                selected,
                Some(tagged_id),
                "Expected the tech-tagged podcast to be selected after filter"
            );
            // The other (untagged) podcast should not be visible — verify it has the tag
            let selected_has_tag = buf
                .selected_podcast()
                .map(|p| p.has_tag("tech"))
                .unwrap_or(false);
            assert!(
                selected_has_tag,
                "Selected podcast should have the 'tech' tag after :filter-tag tech"
            );
        }
    }

    #[tokio::test]
    async fn test_tag_command_shows_error_when_no_podcast_selected() {
        // Arrange — empty podcast list (no selection possible)
        let mut app = make_test_app().await;
        // podcast list buffer is created by UIApp::new() but is empty by default

        // Act
        let result = app.execute_command_direct("tag tech".to_string());

        // Assert — returns Ok but shows error in minibuffer (no silent failure)
        assert!(result.is_ok());
        assert!(
            app.minibuffer.is_visible(),
            "Expected error message in minibuffer when no podcast is selected"
        );
        assert!(
            !app.minibuffer.is_input_mode(),
            "Minibuffer should be in error display mode, not input mode"
        );
    }

    #[tokio::test]
    async fn test_untag_command_shows_error_when_no_podcast_selected() {
        // Arrange — empty podcast list (no selection possible)
        let mut app = make_test_app().await;

        // Act
        let result = app.execute_command_direct("untag tech".to_string());

        // Assert — returns Ok but shows error in minibuffer (no silent failure)
        assert!(result.is_ok());
        assert!(
            app.minibuffer.is_visible(),
            "Expected error message in minibuffer when no podcast is selected for :untag"
        );
        assert!(
            !app.minibuffer.is_input_mode(),
            "Minibuffer should be in error display mode, not input mode"
        );
    }

    // ── Issue #171: PlayEpisode default keybinding / TogglePlayPause feedback ──

    #[tokio::test]
    async fn test_toggle_play_pause_shows_error_when_audio_none() {
        // Arrange — audio_command_tx is None (audio init failed / not available)
        let mut app = make_test_app().await;
        // Do NOT call set_audio_command_tx — leave audio_command_tx as None

        // Act
        let result = app.handle_action(UIAction::TogglePlayPause).await;

        // Assert — returns Ok(true) and shows an error in the minibuffer
        assert!(result.is_ok());
        assert!(
            result.unwrap(),
            "handle_action should return true (continue running)"
        );
        assert!(
            app.minibuffer.is_visible(),
            "Minibuffer should show error when audio is unavailable"
        );
        assert!(
            !app.minibuffer.is_input_mode(),
            "Minibuffer should be in error display mode, not input mode"
        );
        // Verify the exact error message so we catch silent regressions if the
        // string changes (e.g. a different branch fires instead)
        let text = app.minibuffer.text_content();
        assert!(
            text.contains(crate::constants::audio::UNAVAILABLE_ERROR),
            "Minibuffer should contain the audio-unavailable error, got: {text:?}"
        );
    }
}
