pub mod feed;
pub mod models;
pub mod opml;
pub mod subscription;

// Re-export main types
pub use feed::{FeedError, FeedMetadata, FeedParser};
pub use models::{Episode, EpisodeStatus, Podcast, PodcastSubscription};
pub use opml::{FailedImport, ImportResult, OpmlDocument, OpmlError, OpmlExporter, OpmlParser};
pub use subscription::{SubscriptionError, SubscriptionManager};
