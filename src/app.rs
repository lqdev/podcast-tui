use crate::ui::UIApp;
use crate::{
    podcast::subscription::SubscriptionManager,
    storage::{JsonStorage, Storage},
    Config,
};
use anyhow::Result;
use std::sync::Arc;

/// Main application state and orchestration
pub struct App {
    config: Config,
    storage: Arc<JsonStorage>,
    subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
    ui: UIApp,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize storage
        let storage = if let Some(data_dir) = &config.storage.data_directory {
            JsonStorage::with_data_dir(data_dir.into())
        } else {
            JsonStorage::new()?
        };

        // Initialize storage directories
        storage.initialize().await?;

        let storage = Arc::new(storage);

        // Create subscription manager
        let subscription_manager = Arc::new(SubscriptionManager::new(storage.clone()));

        // Initialize UI with config and subscription manager
        let ui = UIApp::new(config.clone(), subscription_manager.clone())
            .map_err(|e| anyhow::anyhow!("Failed to initialize UI: {e}"))?;

        Ok(Self {
            config,
            storage,
            subscription_manager,
            ui,
        })
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        println!("Starting Podcast TUI v1.0.0-mvp");
        println!("Storage initialized at: {:?}", self.storage.data_dir);

        // Run the UI application
        self.ui
            .run()
            .await
            .map_err(|e| anyhow::anyhow!("UI error: {e}"))
    }
}
