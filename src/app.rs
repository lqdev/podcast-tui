use crate::ui::app::UiApp;
use crate::{
    storage::{JsonStorage, Storage},
    Config,
};
use anyhow::Result;
use std::sync::Arc;

/// Main application state and orchestration
pub struct App {
    config: Config,
    storage: Arc<JsonStorage>,
    ui: UiApp,
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
        let ui = UiApp::new();

        Ok(Self {
            config,
            storage,
            ui,
        })
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        println!("Starting Podcast TUI v1.0.0-mvp");
        println!("Storage initialized at: {:?}", self.storage.data_dir);

        self.ui.run().await
    }
}
