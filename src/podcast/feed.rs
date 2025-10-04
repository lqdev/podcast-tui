//! RSS feed parsing and management
//!
//! This module handles RSS/Atom feed parsing and metadata extraction
//! for podcast subscriptions.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::podcast::{Episode, EpisodeStatus, Podcast};
use crate::storage::models::{EpisodeId, PodcastId};
use crate::utils::{time::parse_duration, validation::validate_feed_url};

/// RSS feed parser and manager
pub struct FeedParser {
    http_client: Client,
}

/// Feed metadata extracted during parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedMetadata {
    pub title: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub author: Option<String>,
    pub image_url: Option<String>,
    pub website_url: Option<String>,
    pub last_build_date: Option<DateTime<Utc>>,
    pub total_episodes: usize,
}

/// Errors that can occur during feed parsing
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("Invalid feed URL: {0}")]
    InvalidUrl(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Feed parsing failed: {0}")]
    ParseError(String),

    #[error("Feed validation failed: {0}")]
    ValidationError(String),

    #[error("No episodes found in feed")]
    NoEpisodes,
}

impl FeedParser {
    /// Create a new feed parser
    pub fn new() -> Self {
        let http_client = Client::builder()
            .user_agent("podcast-tui/1.0.0-mvp")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { http_client }
    }

    /// Parse a podcast feed from a URL
    pub async fn parse_feed(&self, feed_url: &str) -> Result<Podcast, FeedError> {
        // Validate the URL first
        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        // Download the feed
        let feed_content = self.download_feed(feed_url).await?;

        // Parse the feed content
        let feed = parser::parse(feed_content.as_bytes())
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        // Create podcast ID from URL
        let podcast_id = PodcastId::from_url(feed_url);

        // Extract metadata
        let metadata = self.extract_feed_metadata(&feed);

        // Extract episodes
        let mut episodes = Vec::new();
        for (index, entry) in feed.entries.iter().enumerate() {
            if let Ok(episode) = self.extract_episode(entry, &podcast_id, index) {
                episodes.push(episode);
            }
        }

        if episodes.is_empty() {
            return Err(FeedError::NoEpisodes);
        }

        // Create the podcast
        let podcast = Podcast {
            id: podcast_id,
            title: metadata.title,
            url: feed_url.to_string(),
            description: metadata.description,
            author: metadata.author,
            image_url: metadata.image_url,
            language: metadata.language,
            categories: Vec::new(), // TODO: Extract from feed
            explicit: false,        // TODO: Extract from iTunes extensions
            last_updated: Utc::now(),
            episodes: Vec::new(), // Episodes IDs will be added as they're saved
        };

        Ok(podcast)
    }

    /// Get just the episodes from a feed (for updates)
    pub async fn get_episodes(
        &self,
        feed_url: &str,
        podcast_id: &PodcastId,
    ) -> Result<Vec<Episode>, FeedError> {
        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        let feed_content = self.download_feed(feed_url).await?;
        let feed = parser::parse(feed_content.as_bytes())
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let mut episodes = Vec::new();
        for (index, entry) in feed.entries.iter().enumerate() {
            if let Ok(episode) = self.extract_episode(entry, podcast_id, index) {
                episodes.push(episode);
            }
        }

        Ok(episodes)
    }

    /// Check if a feed URL is valid and accessible
    pub async fn validate_feed(&self, feed_url: &str) -> Result<FeedMetadata, FeedError> {
        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        let feed_content = self.download_feed(feed_url).await?;
        let feed = parser::parse(feed_content.as_bytes())
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(self.extract_feed_metadata(&feed))
    }

    /// Download feed content from URL
    async fn download_feed(&self, feed_url: &str) -> Result<String, FeedError> {
        let response = self.http_client.get(feed_url).send().await?;

        if !response.status().is_success() {
            return Err(FeedError::Network(reqwest::Error::from(
                response.error_for_status().unwrap_err(),
            )));
        }

        let content = response.text().await?;
        Ok(content)
    }

    /// Extract feed metadata
    fn extract_feed_metadata(&self, feed: &feed_rs::model::Feed) -> FeedMetadata {
        FeedMetadata {
            title: feed
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled Podcast".to_string()),
            description: feed.description.as_ref().map(|d| d.content.clone()),
            language: feed.language.clone(),
            author: feed.authors.first().map(|a| a.name.clone()),
            image_url: feed
                .logo
                .as_ref()
                .map(|l| l.uri.clone())
                .or_else(|| feed.icon.as_ref().map(|i| i.uri.clone())),
            website_url: feed.links.first().map(|l| l.href.clone()),
            last_build_date: feed.updated,
            total_episodes: feed.entries.len(),
        }
    }

    /// Extract episode from feed entry
    fn extract_episode(
        &self,
        entry: &feed_rs::model::Entry,
        podcast_id: &PodcastId,
        index: usize,
    ) -> Result<Episode> {
        // Generate a new Episode ID
        // We could try to make it deterministic based on GUID, but for MVP just use new UUID
        let id = EpisodeId::new();

        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| format!("Episode {}", index + 1));

        let description = entry
            .summary
            .as_ref()
            .map(|t| t.content.clone())
            .or_else(|| {
                entry
                    .content
                    .as_ref()
                    .map(|c| c.body.clone().unwrap_or_default())
            });

        // Find audio enclosure
        let audio_url = entry
            .links
            .iter()
            .find(|link| {
                link.media_type
                    .as_ref()
                    .map(|mt| mt.starts_with("audio/"))
                    .unwrap_or(false)
            })
            .map(|link| link.href.clone());

        // Parse duration from iTunes extension or other sources
        let duration = self.extract_duration(entry);

        // Get file size from enclosure
        let file_size = entry
            .links
            .iter()
            .find(|link| link.length.is_some())
            .and_then(|link| link.length);

        // Get published date
        let published = entry.published.or(entry.updated).unwrap_or_else(Utc::now);

        // Convert duration to seconds if present
        let duration_secs = duration.map(|d| d.num_seconds() as u32);

        // Create the audio URL - use empty string if not found, will be validated at download time
        let audio_url = audio_url.unwrap_or_else(String::new);

        let episode = Episode {
            id,
            podcast_id: podcast_id.clone(),
            title,
            description,
            audio_url,
            published,
            duration: duration_secs,
            file_size,
            mime_type: entry
                .links
                .iter()
                .find(|link| link.media_type.is_some())
                .and_then(|link| link.media_type.clone()),
            guid: if entry.id.is_empty() {
                None
            } else {
                Some(entry.id.clone())
            },
            link: entry.links.first().map(|l| l.href.clone()),
            image_url: None, // TODO: Extract from entry if available
            explicit: false, // TODO: Extract from iTunes extensions
            season: None,
            episode_number: None,
            episode_type: None,
            status: EpisodeStatus::New,
            local_path: None,
            last_played_position: None,
            play_count: 0,
            notes: None,
            chapters: Vec::new(),
            transcript: None,
        };

        Ok(episode)
    }

    /// Extract duration from feed entry
    fn extract_duration(&self, _entry: &feed_rs::model::Entry) -> Option<chrono::Duration> {
        // TODO: Parse duration from iTunes extensions when feed-rs supports it
        // For now, return None
        None
    }
}

impl Default for FeedParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feed_parser_creation() {
        let parser = FeedParser::new();
        // Just test that it creates successfully
        assert_eq!(
            parser
                .http_client
                .get("https://example.com")
                .build()
                .unwrap()
                .url()
                .as_str(),
            "https://example.com/"
        );
    }

    #[tokio::test]
    async fn test_feed_validation() {
        let parser = FeedParser::new();

        // Test invalid URL
        let result = parser.validate_feed("not-a-url").await;
        assert!(result.is_err());

        // Note: Testing with real feeds requires network access
        // For unit tests, we'd want to mock the HTTP client
    }

    // Commented out test that depends on Feed::default() which isn't available
    /*
    #[test]
    fn test_feed_metadata_extraction() {
        use feed_rs::model::{Feed, Text};

        let parser = FeedParser::new();
        let mut feed = Feed::default();
        feed.title = Some(Text {
            content: "Test Podcast".to_string(),
            ..Default::default()
        });

        let metadata = parser.extract_feed_metadata(&feed);
        assert_eq!(metadata.title, "Test Podcast");
        assert_eq!(metadata.total_episodes, 0);
    }
    */
}
