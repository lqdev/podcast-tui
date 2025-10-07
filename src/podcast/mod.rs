pub mod feed;
pub mod models;
pub mod opml;
pub mod subscription;

// Re-export main types
pub use models::{Episode, EpisodeStatus, Podcast, PodcastSubscription};
pub use subscription::{SubscriptionError, SubscriptionManager};
pub use feed::{FeedParser, FeedError, FeedMetadata};
pub use opml::{OpmlParser, OpmlExporter, OpmlError, OpmlDocument, ImportResult, FailedImport};
