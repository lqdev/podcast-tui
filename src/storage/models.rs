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
        unsafe { std::mem::transmute(Box::leak(Box::new(self.0.to_string())).as_str()) }
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
    #[error("Could not access podcast data.")]
    Io(#[from] std::io::Error),

    #[error("Podcast data could not be read.")]
    Serialization(#[from] serde_json::Error),

    #[error("Podcast could not be found.")]
    PodcastNotFound { id: PodcastId },

    #[error("Episode could not be found.")]
    EpisodeNotFound {
        podcast_id: PodcastId,
        episode_id: EpisodeId,
    },

    #[error("Playlist could not be found.")]
    PlaylistNotFound { id: String },

    #[error("Could not prepare storage directory.")]
    DirectoryCreation { path: String },

    #[error("Could not update podcast files.")]
    FileOperation {
        operation: String,
        path: String,
        error: String,
    },

    #[error("Could not update playlist files.")]
    PlaylistFileOperation {
        operation: String,
        path: String,
        error: String,
    },

    #[error("Could not initialize podcast storage.")]
    InitializationFailed { reason: String },

    #[error("Could not create a storage backup.")]
    BackupFailed { reason: String },

    #[error("Could not restore from backup.")]
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

    pub fn technical_details(&self) -> String {
        match self {
            Self::Io(error) => format!("IO error: {}", error),
            Self::Serialization(error) => format!("Serialization error: {}", error),
            Self::PodcastNotFound { id } => format!("Podcast not found: {}", id),
            Self::EpisodeNotFound {
                podcast_id,
                episode_id,
            } => format!(
                "Episode not found: {} in podcast {}",
                episode_id, podcast_id
            ),
            Self::PlaylistNotFound { id } => format!("Playlist not found: {}", id),
            Self::DirectoryCreation { path } => format!("Directory creation failed: {}", path),
            Self::FileOperation {
                operation,
                path,
                error,
            } => format!(
                "File operation failed: {} on {}: {}",
                operation, path, error
            ),
            Self::PlaylistFileOperation {
                operation,
                path,
                error,
            } => format!(
                "Playlist file operation failed: {} on {}: {}",
                operation, path, error
            ),
            Self::InitializationFailed { reason } => {
                format!("Storage initialization failed: {}", reason)
            }
            Self::BackupFailed { reason } => format!("Backup operation failed: {}", reason),
            Self::RestoreFailed { reason } => format!("Restore operation failed: {}", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_error_display_messages_are_user_friendly() {
        let podcast_id = PodcastId::from_string("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let episode_id = EpisodeId::from_string("123e4567-e89b-12d3-a456-426614174001").unwrap();
        let serialization_error =
            serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let io_error = std::io::Error::other("permission denied");

        let cases = vec![
            (
                StorageError::Io(io_error),
                "Could not access podcast data.".to_string(),
            ),
            (
                StorageError::Serialization(serialization_error),
                "Podcast data could not be read.".to_string(),
            ),
            (
                StorageError::PodcastNotFound {
                    id: podcast_id.clone(),
                },
                "Podcast could not be found.".to_string(),
            ),
            (
                StorageError::EpisodeNotFound {
                    podcast_id: podcast_id.clone(),
                    episode_id: episode_id.clone(),
                },
                "Episode could not be found.".to_string(),
            ),
            (
                StorageError::PlaylistNotFound {
                    id: "playlist-id".to_string(),
                },
                "Playlist could not be found.".to_string(),
            ),
            (
                StorageError::DirectoryCreation {
                    path: "C:\\podcast-tui\\data".to_string(),
                },
                "Could not prepare storage directory.".to_string(),
            ),
            (
                StorageError::FileOperation {
                    operation: "delete".to_string(),
                    path: "C:\\podcast-tui\\data\\podcasts.json".to_string(),
                    error: "permission denied".to_string(),
                },
                "Could not update podcast files.".to_string(),
            ),
            (
                StorageError::PlaylistFileOperation {
                    operation: "write".to_string(),
                    path: "C:\\podcast-tui\\data\\playlists.json".to_string(),
                    error: "permission denied".to_string(),
                },
                "Could not update playlist files.".to_string(),
            ),
            (
                StorageError::InitializationFailed {
                    reason: "missing config".to_string(),
                },
                "Could not initialize podcast storage.".to_string(),
            ),
            (
                StorageError::BackupFailed {
                    reason: "disk full".to_string(),
                },
                "Could not create a storage backup.".to_string(),
            ),
            (
                StorageError::RestoreFailed {
                    reason: "invalid archive".to_string(),
                },
                "Could not restore from backup.".to_string(),
            ),
        ];

        for (error, expected) in cases {
            let message = error.to_string();
            assert_eq!(message, expected);
            assert!(!message.contains(&podcast_id.to_string()));
            assert!(!message.contains(&episode_id.to_string()));
            assert!(!message.contains("C:\\"));
        }
    }

    #[test]
    fn storage_error_technical_details_preserve_internal_fields() {
        let podcast_id = PodcastId::from_string("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let episode_id = EpisodeId::from_string("123e4567-e89b-12d3-a456-426614174001").unwrap();

        let file_error = StorageError::FileOperation {
            operation: "delete".to_string(),
            path: "C:\\podcast-tui\\data\\podcasts.json".to_string(),
            error: "permission denied".to_string(),
        };
        let details = file_error.technical_details();
        assert!(details.contains("delete"));
        assert!(details.contains("C:\\podcast-tui\\data\\podcasts.json"));
        assert!(details.contains("permission denied"));

        let episode_error = StorageError::EpisodeNotFound {
            podcast_id: podcast_id.clone(),
            episode_id: episode_id.clone(),
        };
        let details = episode_error.technical_details();
        assert!(details.contains(&podcast_id.to_string()));
        assert!(details.contains(&episode_id.to_string()));
    }
}
