use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for podcasts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PodcastId(pub Uuid);

impl PodcastId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for PodcastId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PodcastId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for episodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpisodeId(pub Uuid);

impl EpisodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for EpisodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EpisodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Storage-specific error types
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Podcast not found: {id}")]
    PodcastNotFound { id: PodcastId },

    #[error("Episode not found: {episode_id} in podcast {podcast_id}")]
    EpisodeNotFound {
        podcast_id: PodcastId,
        episode_id: EpisodeId,
    },

    #[error("Directory creation failed: {path}")]
    DirectoryCreation { path: String },

    #[error("File operation failed: {operation} on {path}: {error}")]
    FileOperation {
        operation: String,
        path: String,
        error: String,
    },

    #[error("Storage initialization failed: {reason}")]
    InitializationFailed { reason: String },

    #[error("Backup operation failed: {reason}")]
    BackupFailed { reason: String },

    #[error("Restore operation failed: {reason}")]
    RestoreFailed { reason: String },
}

impl StorageError {
    pub fn file_operation(
        operation: &str,
        path: &std::path::Path,
        error: impl std::fmt::Display,
    ) -> Self {
        Self::FileOperation {
            operation: operation.to_string(),
            path: path.display().to_string(),
            error: error.to_string(),
        }
    }
}
