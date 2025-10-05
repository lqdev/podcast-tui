use crate::ui::UIApp;
use crate::{
    download::DownloadManager,
    podcast::subscription::SubscriptionManager,
    storage::{JsonStorage, Storage},
    Config,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Main application state and orchestration
pub struct App {
    config: Config,
    storage: Arc<JsonStorage>,
    subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
    download_manager: Arc<DownloadManager<JsonStorage>>,
    ui: UIApp,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        Self::new_with_progress(config, tokio::sync::mpsc::unbounded_channel().0).await
    }

    /// Create a new application instance with progress reporting
    pub async fn new_with_progress(
        config: Config,
        status_tx: tokio::sync::mpsc::UnboundedSender<crate::InitStatus>,
    ) -> Result<Self> {
        // Initialize storage
        status_tx.send(crate::InitStatus::InitializingStorage).ok();
        let storage = if let Some(data_dir) = &config.storage.data_directory {
            JsonStorage::with_data_dir(data_dir.into())
        } else {
            JsonStorage::new()?
        };

        // Initialize storage directories
        storage.initialize().await?;
        let storage = Arc::new(storage);

        // Create download manager with configured downloads directory
        let downloads_dir = shellexpand::tilde(&config.downloads.directory)
            .into_owned()
            .into();
        let download_manager = Arc::new(DownloadManager::new(
            storage.clone(),
            downloads_dir,
            config.downloads.clone(),
        )?);

        // Create subscription manager with download manager for automatic cleanup
        let subscription_manager = Arc::new(SubscriptionManager::with_download_manager(
            storage.clone(),
            download_manager.clone(),
        ));

        // Create app event channel for async communication
        let (app_event_tx, _app_event_rx) = mpsc::unbounded_channel();

        // Initialize UI with config and managers (with progress updates)
        status_tx.send(crate::InitStatus::CreatingBuffers).ok();
        let ui = UIApp::new_with_progress(
            config.clone(),
            subscription_manager.clone(),
            download_manager.clone(),
            storage.clone(),
            app_event_tx,
            status_tx,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize UI: {e}"))?;

        Ok(Self {
            config,
            storage,
            subscription_manager,
            download_manager,
            ui,
        })
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        // Create app event channel for async communication
        let (app_event_tx, app_event_rx) = mpsc::unbounded_channel();

        // Recreate UI with the new app event sender
        self.ui = UIApp::new(
            self.config.clone(),
            self.subscription_manager.clone(),
            self.download_manager.clone(),
            self.storage.clone(),
            app_event_tx,
        )
        .map_err(|e| anyhow::anyhow!("Failed to initialize UI: {e}"))?;

        // Run the UI application with the app event receiver
        self.ui
            .run(app_event_rx)
            .await
            .map_err(|e| anyhow::anyhow!("UI error: {e}"))
    }
}
