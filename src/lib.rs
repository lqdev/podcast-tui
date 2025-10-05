pub mod app;
pub mod config;
pub mod download;
pub mod podcast;
pub mod storage;
pub mod ui;
pub mod utils;

// Re-export commonly used types
pub use app::App;
pub use config::Config;
pub use ui::UIApp;

/// Initialization status for splash screen progress
#[derive(Debug, Clone)]
pub enum InitStatus {
    LoadingConfig,
    InitializingStorage,
    CreatingBuffers,
    LoadingPodcasts,
    LoadingDownloads,
    LoadingWhatsNew,
    Complete,
}

impl InitStatus {
    pub fn message(&self) -> &str {
        match self {
            InitStatus::LoadingConfig => "Loading configuration...",
            InitStatus::InitializingStorage => "Initializing storage...",
            InitStatus::CreatingBuffers => "Creating buffers...",
            InitStatus::LoadingPodcasts => "Loading podcasts...",
            InitStatus::LoadingDownloads => "Loading downloads...",
            InitStatus::LoadingWhatsNew => "Loading what's new...",
            InitStatus::Complete => "Ready!",
        }
    }
}
