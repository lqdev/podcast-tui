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

    /// Create a PodcastId from a feed URL by hashing it
    pub fn from_url(url: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Create a deterministic UUID from the hash
        // This ensures the same URL always generates the same ID
        let uuid = Uuid::from_u64_pair(hash, hash);
        Self(uuid)
    }

    /// Get the string representation of the ID
    pub fn as_str(&self) -> &str {
        // This is a workaround - we return the Display formatted string
        // For a proper implementation, we'd store the string separately
        // or use a different ID type
        unsafe {
            std::mem::transmute(Box::leak(Box::new(self.0.to_string())).as_str())
        }
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

    /// Create an EpisodeId from a GUID by hashing it
    /// This ensures the same GUID always generates the same episode ID for deduplication
    pub fn from_guid(guid: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        guid.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Create a deterministic UUID from the hash
        let uuid = Uuid::from_u64_pair(hash, hash);
        Self(uuid)
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
