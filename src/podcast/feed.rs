//! RSS feed parsing and management//! RSS feed parsing and management//! RSS feed parsing and management//! RSS feed parsing and management

//!

//! This module handles RSS/Atom feed parsing for podcast subscriptions.//!



use anyhow::Result;//! This module handles RSS/Atom feed parsing and metadata extraction//!//!

use chrono::{DateTime, Utc};

use feed_rs::parser;//! for podcast subscriptions.

use reqwest::Client;

use serde::{Deserialize, Serialize};//! This module handles RSS/Atom feed parsing and metadata extraction//! This module handles RSS/Atom feed parsing and metadata extraction

use std::time::Duration;

use anyhow::Result;

use crate::podcast::{Episode, EpisodeStatus, Podcast};

use crate::storage::models::PodcastId;use chrono::{DateTime, Utc};//! for podcast subscriptions.//! for podcast subscriptions.

use crate::utils::validation::validate_feed_url;

use feed_rs::parser;

/// RSS feed parser

pub struct FeedParser {use reqwest::Client;

    http_client: Client,

}use serde::{Deserialize, Serialize};



/// Feed metadatause std::time::Duration;use anyhow::Result;use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct FeedMetadata {

    pub title: String,

    pub description: Option<String>,use crate::podcast::{Episode, EpisodeStatus, Podcast};use chrono::{DateTime, Utc};use chrono::{DateTime, Utc};

    pub total_episodes: usize,

}use crate::storage::models::PodcastId;



/// Feed parsing errorsuse crate::utils::validation::validate_feed_url;use feed_rs::parser;use feed_rs::parser;

#[derive(Debug, thiserror::Error)]

pub enum FeedError {

    #[error("Network error: {0}")]

    Network(#[from] reqwest::Error),/// RSS feed parser and manageruse reqwest::Client;use reqwest::Client;



    #[error("Feed parsing failed: {0}")]pub struct FeedParser {

    ParseError(String),

    http_client: Client,use serde::{Deserialize, Serialize};use serde::{Deserialize, Serialize};

    #[error("Feed validation failed: {0}")]

    ValidationError(String),}

}

use std::time::Duration;use std::time::Duration;

impl FeedParser {

    /// Create a new feed parser/// Feed metadata extracted during parsing

    pub fn new() -> Self {

        let http_client = Client::builder()#[derive(Debug, Clone, Serialize, Deserialize)]use url::Url;

            .user_agent("podcast-tui/1.0.0")

            .timeout(Duration::from_secs(30))pub struct FeedMetadata {

            .build()

            .expect("Failed to create HTTP client");    pub title: String,use crate::podcast::{Episode, EpisodeStatus, Podcast};



        Self { http_client }    pub description: Option<String>,

    }

    pub language: Option<String>,use crate::storage::models::PodcastId;use crate::podcast::{Episode, EpisodeStatus, Podcast};

    /// Parse a podcast feed from a URL

    pub async fn parse_feed(&self, feed_url: &str) -> Result<Podcast, FeedError> {    pub author: Option<String>,

        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

    pub image_url: Option<String>,use crate::utils::validation::validate_feed_url;use crate::storage::models::PodcastId;

        let feed_content = self.download_feed(feed_url).await?;

        let feed = parser::parse(feed_content.as_bytes())    pub website_url: Option<String>,

            .map_err(|e| FeedError::ParseError(e.to_string()))?;

    pub last_build_date: Option<DateTime<Utc>>,use crate::utils::{time::parse_duration, validation::validate_feed_url};

        let podcast_id = PodcastId::from_url(feed_url);

    pub total_episodes: usize,

        let title = feed.title.as_ref()

            .map(|t| t.content.clone())}/// RSS feed parser and manager

            .unwrap_or_else(|| "Untitled Podcast".to_string());



        let description = feed.description.as_ref()

            .map(|d| d.content.clone());/// Errors that can occur during feed parsingpub struct FeedParser {/// RSS feed parser and manager



        let author = feed.authors.first()#[derive(Debug, thiserror::Error)]

            .map(|a| a.name.clone());

pub enum FeedError {    http_client: Client,pub struct FeedParser {

        let image_url = feed.logo.as_ref()

            .map(|l| l.uri.clone());    #[error("Invalid feed URL: {0}")]



        let language = feed.language.clone();    InvalidUrl(String),}    http_client: Client,



        let categories: Vec<String> = feed.categories.iter()

            .map(|c| c.term.clone())

            .collect();    #[error("Network error: {0}")]}



        let podcast = Podcast {    Network(#[from] reqwest::Error),

            id: podcast_id,

            title,/// Feed metadata extracted during parsing

            url: feed_url.to_string(),

            description,    #[error("Feed parsing failed: {0}")]

            author,

            image_url,    ParseError(String),#[derive(Debug, Clone, Serialize, Deserialize)]/// Feed metadata extracted during parsing

            language,

            categories,

            explicit: false,

            last_updated: Utc::now(),    #[error("Feed validation failed: {0}")]pub struct FeedMetadata {#[derive(Debug, Clone, Serialize, Deserialize)]

            episodes: Vec::new(),

        };    ValidationError(String),



        Ok(podcast)    pub title: String,pub struct FeedMetadata {

    }

    #[error("No episodes found in feed")]

    /// Get episodes from a feed

    pub async fn get_episodes(&self, feed_url: &str, podcast_id: &PodcastId) -> Result<Vec<Episode>, FeedError> {    NoEpisodes,    pub description: Option<String>,    pub title: String,

        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

}

        let feed_content = self.download_feed(feed_url).await?;

        let feed = parser::parse(feed_content.as_bytes())    pub language: Option<String>,    pub description: Option<String>,

            .map_err(|e| FeedError::ParseError(e.to_string()))?;

impl FeedParser {

        let mut episodes = Vec::new();

        for (index, entry) in feed.entries.iter().enumerate() {    /// Create a new feed parser    pub author: Option<String>,    pub language: Option<String>,

            if let Ok(episode) = self.extract_episode(entry, podcast_id, index) {

                episodes.push(episode);    pub fn new() -> Self {

            }

        }        let http_client = Client::builder()    pub image_url: Option<String>,    pub author: Option<String>,



        Ok(episodes)            .user_agent("podcast-tui/1.0.0")

    }

            .timeout(Duration::from_secs(30))    pub website_url: Option<String>,    pub image_url: Option<String>,

    /// Validate feed URL

    pub async fn validate_feed(&self, feed_url: &str) -> Result<FeedMetadata, FeedError> {            .build()

        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

            .expect("Failed to create HTTP client");    pub last_build_date: Option<DateTime<Utc>>,    pub website_url: Option<String>,

        let feed_content = self.download_feed(feed_url).await?;

        let feed = parser::parse(feed_content.as_bytes())

            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Self { http_client }    pub total_episodes: usize,    pub last_build_date: Option<DateTime<Utc>>,

        Ok(FeedMetadata {

            title: feed.title.as_ref()    }

                .map(|t| t.content.clone())

                .unwrap_or_else(|| "Untitled Podcast".to_string()),}    pub total_episodes: usize,

            description: feed.description.as_ref()

                .map(|d| d.content.clone()),    /// Parse a podcast feed from a URL

            total_episodes: feed.entries.len(),

        })    pub async fn parse_feed(&self, feed_url: &str) -> Result<Podcast, FeedError> {}

    }

        // Validate the URL first

    /// Download feed content

    async fn download_feed(&self, feed_url: &str) -> Result<String, FeedError> {        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;/// Errors that can occur during feed parsing

        let response = self.http_client

            .get(feed_url)

            .send()

            .await?;        // Download the feed#[derive(Debug, thiserror::Error)]/// Errors that can occur during feed parsing



        if !response.status().is_success() {        let feed_content = self.download_feed(feed_url).await?;

            return Err(FeedError::Network(

                reqwest::Error::from(response.error_for_status().unwrap_err())pub enum FeedError {#[derive(Debug, thiserror::Error)]

            ));

        }        // Parse the feed content



        let content = response.text().await?;        let feed = parser::parse(feed_content.as_bytes())    #[error("Invalid feed URL: {0}")]pub enum FeedError {

        Ok(content)

    }            .map_err(|e| FeedError::ParseError(e.to_string()))?;



    /// Extract episode from feed entry    InvalidUrl(String),    #[error("Invalid feed URL: {0}")]

    fn extract_episode(

        &self,        // Create podcast ID from URL

        entry: &feed_rs::model::Entry,

        podcast_id: &PodcastId,        let podcast_id = PodcastId::from_url(feed_url);    InvalidUrl(String),

        index: usize,

    ) -> Result<Episode> {

        let id = entry.id.clone().unwrap_or_else(|| {

            format!("episode-{}-{}", podcast_id.as_str(), index)        // Extract podcast metadata    #[error("Network error: {0}")]

        });

        let title = feed.title.as_ref()

        let title = entry.title.as_ref()

            .map(|t| t.content.clone())            .map(|t| t.content.clone())    Network(#[from] reqwest::Error),    #[error("Network error: {0}")]

            .unwrap_or_else(|| format!("Episode {}", index + 1));

            .unwrap_or_else(|| "Untitled Podcast".to_string());

        let description = entry.summary.as_ref()

            .or_else(|| entry.content.first())    Network(#[from] reqwest::Error),

            .map(|d| d.content.clone());

        let description = feed.description.as_ref()

        let audio_url = entry.links.iter()

            .find(|link| {            .map(|d| d.content.clone());    #[error("Feed parsing failed: {0}")]

                link.media_type.as_ref()

                    .map(|mt| mt.starts_with("audio/"))

                    .unwrap_or(false)

            })        let author = feed.authors.first()    ParseError(String),    #[error("Feed parsing failed: {0}")]

            .map(|link| link.href.clone())

            .unwrap_or_else(|| "".to_string());            .map(|a| a.name.clone());



        let published = entry.published.or(entry.updated).unwrap_or_else(Utc::now);    ParseError(String),



        let file_size = entry.links.iter()        let image_url = feed.logo.as_ref()

            .find(|link| link.length.is_some())

            .and_then(|link| link.length);            .map(|l| l.uri.clone())    #[error("Feed validation failed: {0}")]



        let episode = Episode {            .or_else(|| feed.icon.as_ref().map(|i| i.uri.clone()));

            id: id.into(),

            podcast_id: podcast_id.clone(),    ValidationError(String),    #[error("Feed validation failed: {0}")]

            title,

            description,        let language = feed.language.clone();

            audio_url,

            published,    ValidationError(String),

            duration: None,

            file_size,        let categories: Vec<String> = feed.categories.iter()

            mime_type: entry.links.iter()

                .find(|link| link.media_type.is_some())            .map(|c| c.term.clone())    #[error("No episodes found in feed")]

                .and_then(|link| link.media_type.clone()),

            guid: entry.id.clone(),            .collect();

            link: entry.links.first().map(|l| l.href.clone()),

            image_url: None,    NoEpisodes,    #[error("No episodes found in feed")]

            explicit: false,

            season: None,        // Create the podcast with the actual model structure

            episode_number: None,

            episode_type: None,        let podcast = Podcast {}    NoEpisodes,

            status: EpisodeStatus::New,

            local_path: None,            id: podcast_id,

            last_played_position: None,

            play_count: 0,            title,}

            notes: None,

            chapters: Vec::new(),            url: feed_url.to_string(), // Using 'url' field as it exists in the model

            transcript: None,

        };            description,impl FeedParser {



        Ok(episode)            author,

    }

}            image_url,    /// Create a new feed parserimpl FeedParser {



impl Default for FeedParser {            language,

    fn default() -> Self {

        Self::new()            categories,    pub fn new() -> Self {    /// Create a new feed parser

    }

}            explicit: false, // TODO: Extract from iTunes extensions



#[cfg(test)]            last_updated: Utc::now(),        let http_client = Client::builder()    pub fn new() -> Self {

mod tests {

    use super::*;            episodes: Vec::new(), // Episodes will be added separately



    #[tokio::test]        };            .user_agent("podcast-tui/1.0.0")        let http_client = Client::builder()

    async fn test_feed_parser_creation() {

        let parser = FeedParser::new();

        assert!(parser.http_client.timeout().is_some());

    }        Ok(podcast)            .timeout(Duration::from_secs(30))            .user_agent("podcast-tui/1.0.0")

}
    }

            .build()            .timeout(Duration::from_secs(30))

    /// Get episodes from a feed (for updates)

    pub async fn get_episodes(&self, feed_url: &str, podcast_id: &PodcastId) -> Result<Vec<Episode>, FeedError> {            .expect("Failed to create HTTP client");            .build()

        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

            .expect("Failed to create HTTP client");

        let feed_content = self.download_feed(feed_url).await?;

        let feed = parser::parse(feed_content.as_bytes())        Self { http_client }

            .map_err(|e| FeedError::ParseError(e.to_string()))?;

    }        Self { http_client }

        let mut episodes = Vec::new();

        for (index, entry) in feed.entries.iter().enumerate() {    }

            if let Ok(episode) = self.extract_episode(entry, podcast_id, index) {

                episodes.push(episode);    /// Parse a podcast feed from a URL

            }

        }    pub async fn parse_feed(&self, feed_url: &str) -> Result<Podcast, FeedError> {    /// Parse a podcast feed from a URL



        Ok(episodes)        // Validate the URL first    pub async fn parse_feed(&self, feed_url: &str) -> Result<Podcast, FeedError> {

    }

        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;        // Validate the URL first

    /// Check if a feed URL is valid and accessible

    pub async fn validate_feed(&self, feed_url: &str) -> Result<FeedMetadata, FeedError> {        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        // Download the feed

        let feed_content = self.download_feed(feed_url).await?;

        let feed = parser::parse(feed_content.as_bytes())        let feed_content = self.download_feed(feed_url).await?;        // Download the feed

            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let feed_content = self.download_feed(feed_url).await?;

        Ok(self.extract_feed_metadata(&feed))

    }        // Parse the feed content



    /// Download feed content from URL        let feed = parser::parse(feed_content.as_bytes())        // Parse the feed content

    async fn download_feed(&self, feed_url: &str) -> Result<String, FeedError> {

        let response = self.http_client            .map_err(|e| FeedError::ParseError(e.to_string()))?;        let feed = parser::parse(feed_content.as_bytes())

            .get(feed_url)

            .send()            .map_err(|e| FeedError::ParseError(e.to_string()))?;

            .await?;

        // Create podcast ID from URL

        if !response.status().is_success() {

            return Err(FeedError::Network(        let podcast_id = PodcastId::from_url(feed_url);        // Extract podcast metadata

                reqwest::Error::from(response.error_for_status().unwrap_err())

            ));        let metadata = self.extract_feed_metadata(&feed);

        }

        // Extract podcast metadata

        let content = response.text().await?;

        Ok(content)        let title = feed.title.as_ref()        // Create podcast ID from URL

    }

            .map(|t| t.content.clone())        let podcast_id = PodcastId::from_url(feed_url);

    /// Extract feed metadata

    fn extract_feed_metadata(&self, feed: &feed_rs::model::Feed) -> FeedMetadata {            .unwrap_or_else(|| "Untitled Podcast".to_string());

        FeedMetadata {

            title: feed.title.as_ref()        // Extract episodes

                .map(|t| t.content.clone())

                .unwrap_or_else(|| "Untitled Podcast".to_string()),        let description = feed.description.as_ref()        let mut episodes = Vec::new();

            description: feed.description.as_ref()

                .map(|d| d.content.clone()),            .map(|d| d.content.clone());        for (index, entry) in feed.entries.iter().enumerate() {

            language: feed.language.clone(),

            author: feed.authors.first()            if let Ok(episode) = self.extract_episode(entry, &podcast_id, index) {

                .map(|a| a.name.clone()),

            image_url: feed.logo.as_ref()        let author = feed.authors.first()                episodes.push(episode);

                .map(|l| l.uri.clone())

                .or_else(|| feed.icon.as_ref().map(|i| i.uri.clone())),            .map(|a| a.name.clone());            }

            website_url: feed.links.first()

                .map(|l| l.href.clone()),        }

            last_build_date: feed.updated,

            total_episodes: feed.entries.len(),        let image_url = feed.logo.as_ref()

        }

    }            .map(|l| l.uri.clone())        if episodes.is_empty() {



    /// Extract episode from feed entry            .or_else(|| feed.icon.as_ref().map(|i| i.uri.clone()));            return Err(FeedError::NoEpisodes);

    fn extract_episode(

        &self,        }

        entry: &feed_rs::model::Entry,

        podcast_id: &PodcastId,        let language = feed.language.clone();

        index: usize,

    ) -> Result<Episode> {        // Create the podcast

        let id = entry.id.clone().unwrap_or_else(|| {

            // Generate ID from title and published date if no ID exists        let categories: Vec<String> = feed.categories.iter()        let podcast = Podcast {

            format!("episode-{}-{}", podcast_id.as_str(), index)

        });            .map(|c| c.term.clone())            id: podcast_id,



        let title = entry.title.as_ref()            .collect();            title: metadata.title,

            .map(|t| t.content.clone())

            .unwrap_or_else(|| format!("Episode {}", index + 1));            description: metadata.description,



        let description = entry.summary.as_ref()        // Create the podcast with the actual model structure            author: metadata.author,

            .or_else(|| entry.content.first())

            .map(|d| d.content.clone());        let podcast = Podcast {            language: metadata.language,



        // Find audio enclosure            id: podcast_id,            image_url: metadata.image_url,

        let audio_url = entry.links.iter()

            .find(|link| {            title,            website_url: metadata.website_url,

                link.media_type.as_ref()

                    .map(|mt| mt.starts_with("audio/"))            url: feed_url.to_string(), // Using 'url' field as it exists in the model            feed_url: feed_url.to_string(),

                    .unwrap_or(false)

            })            description,            last_updated: Utc::now(),

            .map(|link| link.href.clone())

            .unwrap_or_else(|| "".to_string());            author,            subscription_date: Utc::now(),



        // Get published date            image_url,            auto_download: false,

        let published = entry.published.or(entry.updated).unwrap_or_else(Utc::now);

            language,            download_limit: None,

        // Get file size from enclosure

        let file_size = entry.links.iter()            categories,        };

            .find(|link| link.length.is_some())

            .and_then(|link| link.length);            explicit: false, // TODO: Extract from iTunes extensions



        // Extract GUID            last_updated: Utc::now(),        Ok(podcast)

        let guid = entry.id.clone();

            episodes: Vec::new(), // Episodes will be added separately    }

        // Extract link

        let link = entry.links.first().map(|l| l.href.clone());        };



        let episode = Episode {    /// Get just the episodes from a feed (for updates)

            id: id.into(),

            podcast_id: podcast_id.clone(),        Ok(podcast)    pub async fn get_episodes(&self, feed_url: &str, podcast_id: &PodcastId) -> Result<Vec<Episode>, FeedError> {

            title,

            description,    }        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

            audio_url,

            published,

            duration: None,

            file_size,    /// Get episodes from a feed (for updates)        let feed_content = self.download_feed(feed_url).await?;

            mime_type: entry.links.iter()

                .find(|link| link.media_type.is_some())    pub async fn get_episodes(&self, feed_url: &str, podcast_id: &PodcastId) -> Result<Vec<Episode>, FeedError> {        let feed = parser::parse(feed_content.as_bytes())

                .and_then(|link| link.media_type.clone()),

            guid,        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;            .map_err(|e| FeedError::ParseError(e.to_string()))?;

            link,

            image_url: None,

            explicit: false,

            season: None,        let feed_content = self.download_feed(feed_url).await?;        let mut episodes = Vec::new();

            episode_number: None,

            episode_type: None,        let feed = parser::parse(feed_content.as_bytes())        for (index, entry) in feed.entries.iter().enumerate() {

            status: EpisodeStatus::New,

            local_path: None,            .map_err(|e| FeedError::ParseError(e.to_string()))?;            if let Ok(episode) = self.extract_episode(entry, podcast_id, index) {

            last_played_position: None,

            play_count: 0,                episodes.push(episode);

            notes: None,

            chapters: Vec::new(),        let mut episodes = Vec::new();            }

            transcript: None,

        };        for (index, entry) in feed.entries.iter().enumerate() {        }



        Ok(episode)            if let Ok(episode) = self.extract_episode(entry, podcast_id, index) {

    }

}                episodes.push(episode);        Ok(episodes)



impl Default for FeedParser {            }    }

    fn default() -> Self {

        Self::new()        }

    }

}    /// Check if a feed URL is valid and accessible



#[cfg(test)]        Ok(episodes)    pub async fn validate_feed(&self, feed_url: &str) -> Result<FeedMetadata, FeedError> {

mod tests {

    use super::*;    }        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;



    #[tokio::test]

    async fn test_feed_parser_creation() {

        let parser = FeedParser::new();    /// Check if a feed URL is valid and accessible        let feed_content = self.download_feed(feed_url).await?;

        assert!(parser.http_client.timeout().is_some());

    }    pub async fn validate_feed(&self, feed_url: &str) -> Result<FeedMetadata, FeedError> {        let feed = parser::parse(feed_content.as_bytes())



    #[tokio::test]        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;            .map_err(|e| FeedError::ParseError(e.to_string()))?;

    async fn test_feed_validation() {

        let parser = FeedParser::new();

        

        // Test invalid URL        let feed_content = self.download_feed(feed_url).await?;        Ok(self.extract_feed_metadata(&feed))

        let result = parser.validate_feed("not-a-url").await;

        assert!(result.is_err());        let feed = parser::parse(feed_content.as_bytes())    }

    }

            .map_err(|e| FeedError::ParseError(e.to_string()))?;

    #[test]

    fn test_feed_metadata_extraction() {    /// Download feed content from URL

        use feed_rs::model::{Feed, Text};

        Ok(self.extract_feed_metadata(&feed))    async fn download_feed(&self, feed_url: &str) -> Result<String, FeedError> {

        let parser = FeedParser::new();

        let mut feed = Feed::default();    }        let response = self.http_client

        feed.title = Some(Text {

            content: "Test Podcast".to_string(),            .get(feed_url)

            ..Default::default()

        });    /// Download feed content from URL            .send()



        let metadata = parser.extract_feed_metadata(&feed);    async fn download_feed(&self, feed_url: &str) -> Result<String, FeedError> {            .await?;

        assert_eq!(metadata.title, "Test Podcast");

        assert_eq!(metadata.total_episodes, 0);        let response = self.http_client

    }

}            .get(feed_url)        if !response.status().is_success() {

            .send()            return Err(FeedError::Network(

            .await?;                reqwest::Error::from(reqwest::ErrorKind::Request)

            ));

        if !response.status().is_success() {        }

            return Err(FeedError::Network(

                reqwest::Error::from(response.error_for_status().unwrap_err())        let content = response.text().await?;

            ));        Ok(content)

        }    }



        let content = response.text().await?;    /// Extract feed metadata

        Ok(content)    fn extract_feed_metadata(&self, feed: &feed_rs::model::Feed) -> FeedMetadata {

    }        FeedMetadata {

            title: feed.title.as_ref()

    /// Extract feed metadata                .map(|t| t.content.clone())

    fn extract_feed_metadata(&self, feed: &feed_rs::model::Feed) -> FeedMetadata {                .unwrap_or_else(|| "Untitled Podcast".to_string()),

        FeedMetadata {            description: feed.description.as_ref()

            title: feed.title.as_ref()                .map(|d| d.content.clone()),

                .map(|t| t.content.clone())            language: feed.language.clone(),

                .unwrap_or_else(|| "Untitled Podcast".to_string()),            author: feed.authors.first()

            description: feed.description.as_ref()                .map(|a| a.name.clone()),

                .map(|d| d.content.clone()),            image_url: feed.logo.as_ref()

            language: feed.language.clone(),                .map(|l| l.uri.clone())

            author: feed.authors.first()                .or_else(|| feed.icon.as_ref().map(|i| i.uri.clone())),

                .map(|a| a.name.clone()),            website_url: feed.links.first()

            image_url: feed.logo.as_ref()                .map(|l| l.href.clone()),

                .map(|l| l.uri.clone())            last_build_date: feed.updated,

                .or_else(|| feed.icon.as_ref().map(|i| i.uri.clone())),            total_episodes: feed.entries.len(),

            website_url: feed.links.first()        }

                .map(|l| l.href.clone()),    }

            last_build_date: feed.updated,

            total_episodes: feed.entries.len(),    /// Extract episode from feed entry

        }    fn extract_episode(

    }        &self,

        entry: &feed_rs::model::Entry,

    /// Extract episode from feed entry        podcast_id: &PodcastId,

    fn extract_episode(        index: usize,

        &self,    ) -> Result<Episode> {

        entry: &feed_rs::model::Entry,        let id = entry.id.clone().unwrap_or_else(|| {

        podcast_id: &PodcastId,            // Generate ID from title and published date if no ID exists

        index: usize,            format!("episode-{}-{}", podcast_id.as_str(), index)

    ) -> Result<Episode> {        });

        let id = entry.id.clone().unwrap_or_else(|| {

            // Generate ID from title and published date if no ID exists        let title = entry.title.as_ref()

            format!("episode-{}-{}", podcast_id.as_str(), index)            .map(|t| t.content.clone())

        });            .unwrap_or_else(|| format!("Episode {}", index + 1));



        let title = entry.title.as_ref()        let description = entry.summary.as_ref()

            .map(|t| t.content.clone())            .or_else(|| entry.content.first())

            .unwrap_or_else(|| format!("Episode {}", index + 1));            .map(|d| d.content.clone());



        let description = entry.summary.as_ref()        // Find audio enclosure

            .or_else(|| entry.content.first())        let audio_url = entry.links.iter()

            .map(|d| d.content.clone());            .find(|link| {

                link.media_type.as_ref()

        // Find audio enclosure                    .map(|mt| mt.starts_with("audio/"))

        let audio_url = entry.links.iter()                    .unwrap_or(false)

            .find(|link| {            })

                link.media_type.as_ref()            .map(|link| link.href.clone());

                    .map(|mt| mt.starts_with("audio/"))

                    .unwrap_or(false)        // Parse duration from iTunes extension or other sources

            })        let duration = self.extract_duration(entry);

            .map(|link| link.href.clone())

            .unwrap_or_else(|| "".to_string()); // Use empty string if no audio URL found        // Get file size from enclosure

        let file_size = entry.links.iter()

        // Get published date            .find(|link| link.length.is_some())

        let published = entry.published.or(entry.updated).unwrap_or_else(Utc::now);            .and_then(|link| link.length);



        // Get file size from enclosure        let episode = Episode {

        let file_size = entry.links.iter()            id: id.into(),

            .find(|link| link.length.is_some())            podcast_id: podcast_id.clone(),

            .and_then(|link| link.length);            title,

            description,

        // Extract GUID            audio_url,

        let guid = entry.id.clone();            published_date: entry.published.or(entry.updated),

            duration,

        // Extract link            file_size,

        let link = entry.links.first().map(|l| l.href.clone());            status: EpisodeStatus::New,

            file_path: None,

        // Extract duration (simplified - would need iTunes extension parsing for full support)            played_duration: None,

        let duration = None; // TODO: Parse from iTunes extensions            notes: None,

            chapters: None,

        let episode = Episode {            transcript: None,

            id: id.into(),        };

            podcast_id: podcast_id.clone(),

            title,        Ok(episode)

            description,    }

            audio_url,

            published,    /// Extract duration from feed entry

            duration,    fn extract_duration(&self, entry: &feed_rs::model::Entry) -> Option<chrono::Duration> {

            file_size,        // Try to find duration in iTunes extensions

            mime_type: entry.links.iter()        for extension in &entry.extensions {

                .find(|link| link.media_type.is_some())            if extension.name == "duration" {

                .and_then(|link| link.media_type.clone()),                if let Some(duration_str) = extension.value.as_ref() {

            guid,                    if let Ok(duration) = parse_duration(duration_str) {

            link,                        return Some(duration);

            image_url: None, // TODO: Extract from entry if available                    }

            explicit: false, // TODO: Extract from iTunes extensions                }

            season: None,    // TODO: Extract from iTunes extensions              }

            episode_number: None, // TODO: Extract from iTunes extensions        }

            episode_type: None,   // TODO: Extract from iTunes extensions

            status: EpisodeStatus::New,        None

            local_path: None,    }

            last_played_position: None,}

            play_count: 0,

            notes: None,impl Default for FeedParser {

            chapters: Vec::new(),    fn default() -> Self {

            transcript: None,        Self::new()

        };    }

}

        Ok(episode)

    }#[cfg(test)]

}mod tests {

    use super::*;

impl Default for FeedParser {

    fn default() -> Self {    #[tokio::test]

        Self::new()    async fn test_feed_parser_creation() {

    }        let parser = FeedParser::new();

}        assert!(parser.http_client.timeout().is_some());

    }

#[cfg(test)]

mod tests {    #[tokio::test]

    use super::*;    async fn test_feed_validation() {

        let parser = FeedParser::new();

    #[tokio::test]        

    async fn test_feed_parser_creation() {        // Test invalid URL

        let parser = FeedParser::new();        let result = parser.validate_feed("not-a-url").await;

        assert!(parser.http_client.timeout().is_some());        assert!(result.is_err());

    }

        // Note: Testing with real feeds requires network access

    #[tokio::test]        // For unit tests, we'd want to mock the HTTP client

    async fn test_feed_validation() {    }

        let parser = FeedParser::new();

            #[test]

        // Test invalid URL    fn test_feed_metadata_extraction() {

        let result = parser.validate_feed("not-a-url").await;        use feed_rs::model::{Feed, Text};

        assert!(result.is_err());

        let parser = FeedParser::new();

        // Note: Testing with real feeds requires network access        let mut feed = Feed::default();

        // For unit tests, we'd want to mock the HTTP client        feed.title = Some(Text {

    }            content: "Test Podcast".to_string(),

            ..Default::default()

    #[test]        });

    fn test_feed_metadata_extraction() {

        use feed_rs::model::{Feed, Text};        let metadata = parser.extract_feed_metadata(&feed);

        assert_eq!(metadata.title, "Test Podcast");

        let parser = FeedParser::new();        assert_eq!(metadata.total_episodes, 0);

        let mut feed = Feed::default();    }

        feed.title = Some(Text {}
            content: "Test Podcast".to_string(),
            ..Default::default()
        });

        let metadata = parser.extract_feed_metadata(&feed);
        assert_eq!(metadata.title, "Test Podcast");
        assert_eq!(metadata.total_episodes, 0);
    }
}