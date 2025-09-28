use anyhow::Result;
use crate::{Config, storage::{JsonStorage, Storage}};
use crate::ui::UIApp;
use std::sync::Arc;

/// Main application state and orchestration
pub struct App {
    config: Config,
    storage: Arc<JsonStorage>,
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
        
        // Initialize UI with config
        let ui = UIApp::new(config.clone()).map_err(|e| anyhow::anyhow!("Failed to initialize UI: {}", e))?;
        
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
        
        // Run the UI application
        self.ui.run().await.map_err(|e| anyhow::anyhow!("UI error: {}", e))
    }
}