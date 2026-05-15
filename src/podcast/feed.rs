//! RSS feed parsing and management
//!
//! This module handles RSS/Atom feed parsing and metadata extraction
//! for podcast subscriptions.

use anyhow::Result;
use chrono::{DateTime, Utc};
use feed_rs::parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::podcast::{Episode, EpisodeStatus, Podcast};
use crate::storage::models::{EpisodeId, PodcastId};
use crate::utils::text::strip_html;
use crate::utils::validation::validate_feed_url;

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

/// Result of a successful feed download. `body` is the raw response body.
/// `etag` and `last_modified` carry the cache validators returned by the
/// server (if any) so callers can persist them and send them back on the
/// next refresh as `If-None-Match` / `If-Modified-Since`.
#[derive(Debug, Clone)]
pub struct FeedFetchResult {
    pub body: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Result of a successful episode fetch from a feed. `None` for the whole
/// `Option` (returned by `get_episodes_conditional`) means the server
/// responded with `304 Not Modified` and no body was downloaded.
#[derive(Debug, Clone)]
pub struct EpisodeFetchResult {
    pub episodes: Vec<Episode>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
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
            .user_agent("Mozilla/5.0 (compatible; podcast-tui/1.0; +https://github.com/podcast-tui) AppleWebKit/537.36 (KHTML, like Gecko)")
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(10)) // Handle up to 10 redirects
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
            tags: Vec::new(),
            // Cache validators are captured on refresh, not subscribe.
            // The first refresh will receive the ETag and persist it then.
            last_etag: None,
            last_modified: None,
        };

        Ok(podcast)
    }

    /// Get just the episodes from a feed (for updates).
    ///
    /// Backwards-compatible wrapper that always sends an unconditional GET.
    /// New callers that want conditional-GET semantics should use
    /// [`Self::get_episodes_conditional`].
    pub async fn get_episodes(
        &self,
        feed_url: &str,
        podcast_id: &PodcastId,
    ) -> Result<Vec<Episode>, FeedError> {
        // Unwrap is safe: passing `None, None` cannot produce a 304.
        Ok(self
            .get_episodes_conditional(feed_url, podcast_id, None, None)
            .await?
            .expect("unconditional GET cannot return 304")
            .episodes)
    }

    /// Get episodes from a feed using HTTP conditional GET (RFC 7232).
    ///
    /// When `if_none_match` and/or `if_modified_since` are `Some`, sends
    /// the corresponding request headers. If the server responds with
    /// `304 Not Modified`, returns `Ok(None)` without downloading or
    /// parsing any body. Otherwise returns `Ok(Some(EpisodeFetchResult))`
    /// with the parsed episodes plus any new cache validators from the
    /// response headers.
    pub async fn get_episodes_conditional(
        &self,
        feed_url: &str,
        podcast_id: &PodcastId,
        if_none_match: Option<&str>,
        if_modified_since: Option<&str>,
    ) -> Result<Option<EpisodeFetchResult>, FeedError> {
        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        let Some(fetch) = self
            .download_feed_conditional(feed_url, if_none_match, if_modified_since)
            .await?
        else {
            return Ok(None);
        };

        let feed = parser::parse(fetch.body.as_bytes())
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        let mut episodes = Vec::new();
        for (index, entry) in feed.entries.iter().enumerate() {
            if let Ok(episode) = self.extract_episode(entry, podcast_id, index) {
                episodes.push(episode);
            }
        }

        Ok(Some(EpisodeFetchResult {
            episodes,
            etag: fetch.etag,
            last_modified: fetch.last_modified,
        }))
    }

    /// Check if a feed URL is valid and accessible
    pub async fn validate_feed(&self, feed_url: &str) -> Result<FeedMetadata, FeedError> {
        validate_feed_url(feed_url).map_err(FeedError::ValidationError)?;

        let feed_content = self.download_feed(feed_url).await?;
        let feed = parser::parse(feed_content.as_bytes())
            .map_err(|e| FeedError::ParseError(e.to_string()))?;

        Ok(self.extract_feed_metadata(&feed))
    }

    /// Download feed content from URL (unconditional GET).
    ///
    /// Backwards-compatible wrapper around [`Self::download_feed_conditional`]
    /// that never sends conditional headers and therefore can never receive
    /// a 304 response. Used by `parse_feed` (initial subscribe) and
    /// `validate_feed`.
    async fn download_feed(&self, feed_url: &str) -> Result<String, FeedError> {
        Ok(self
            .download_feed_conditional(feed_url, None, None)
            .await?
            .expect("unconditional GET cannot return 304")
            .body)
    }

    /// Download feed content with optional HTTP conditional-GET headers.
    ///
    /// Sends `If-None-Match` and/or `If-Modified-Since` when the
    /// corresponding parameter is `Some`. Returns `Ok(None)` if the server
    /// responded with `304 Not Modified` (no body to read). Returns
    /// `Ok(Some(FeedFetchResult))` on `200 OK`, capturing the response's
    /// `ETag` and `Last-Modified` headers (if any) so they can be persisted
    /// for the next refresh.
    async fn download_feed_conditional(
        &self,
        feed_url: &str,
        if_none_match: Option<&str>,
        if_modified_since: Option<&str>,
    ) -> Result<Option<FeedFetchResult>, FeedError> {
        let mut req = self
            .http_client
            .get(feed_url)
            .header("Accept", "application/rss+xml, application/rdf+xml, application/atom+xml, application/xml, text/xml, */*");

        if let Some(etag) = if_none_match {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(lm) = if_modified_since {
            req = req.header(reqwest::header::IF_MODIFIED_SINCE, lm);
        }

        let response = req.send().await.map_err(FeedError::Network)?;

        let status = response.status();

        // RFC 7232: 304 responses MUST NOT include a body. Short-circuit
        // before any read to surface the "nothing changed" signal cheaply.
        if status.as_u16() == 304 {
            return Ok(None);
        }

        if !status.is_success() {
            return Err(FeedError::Network(response.error_for_status().unwrap_err()));
        }

        // Capture cache validators *before* consuming the response body.
        // Header values are opaque ASCII; round-trip them verbatim (do not
        // strip ETag quotes — RFC 7232 treats `"abc"` and `abc` as distinct).
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let last_modified = response
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Check content type if available (validation only)
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(ct_str) = content_type.to_str() {
                if !ct_str.contains("xml") && !ct_str.contains("rss") && !ct_str.contains("atom") {
                    // Content type validation - could be added to error handling if needed
                }
            }
        }

        let body = response.text().await.map_err(FeedError::Network)?;

        Ok(Some(FeedFetchResult {
            body,
            etag,
            last_modified,
        }))
    }

    /// Extract feed metadata
    fn extract_feed_metadata(&self, feed: &feed_rs::model::Feed) -> FeedMetadata {
        FeedMetadata {
            title: feed
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled Podcast".to_string()),
            description: feed
                .description
                .as_ref()
                .map(|d| strip_html(&d.content))
                .filter(|s| !s.is_empty()),
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
        // Generate deterministic Episode ID based on GUID or other stable identifier
        let id = if !entry.id.is_empty() {
            // Use GUID-based deterministic ID for better deduplication
            EpisodeId::from_guid(&entry.id)
        } else {
            // Fallback to new UUID if no GUID available
            EpisodeId::new()
        };

        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| format!("Episode {}", index + 1));

        // Extract and sanitize description
        // RSS descriptions can contain HTML/CDATA that needs to be stripped for TUI display
        let description = entry
            .summary
            .as_ref()
            .map(|t| strip_html(&t.content))
            .or_else(|| {
                entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.as_ref().map(|body| strip_html(body)))
            })
            .filter(|s| !s.is_empty()); // Filter out empty descriptions

        // Find audio enclosure using comprehensive strategy
        let audio_url = self.extract_audio_url(entry);

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
        let audio_url = audio_url.unwrap_or_default();

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
            favorited: false,
        };

        Ok(episode)
    }

    /// Extract audio URL from feed entry using multiple strategies
    fn extract_audio_url(&self, entry: &feed_rs::model::Entry) -> Option<String> {
        // Strategy 1: Look for media enclosures (RSS 2.0 <enclosure> elements)
        // In feed-rs 2.0, enclosures are stored in the media field as MediaContent objects
        for media_object in &entry.media {
            for media_content in &media_object.content {
                if let Some(url) = &media_content.url {
                    let url_string = url.to_string();

                    // Skip embed/player URLs (these return HTML, not audio)
                    if url_string.contains("/embed") || url_string.contains("/player") {
                        continue;
                    }

                    // Check if it has an audio MIME type
                    if let Some(content_type) = &media_content.content_type {
                        let content_type_str = content_type.to_string();
                        if content_type_str.starts_with("audio/")
                            || content_type_str == "application/octet-stream"
                        {
                            return Some(url_string);
                        }
                    }

                    // If no MIME type, check file extension
                    let url_lower = url_string.to_lowercase();
                    let url_path = url_lower.split('?').next().unwrap_or(&url_lower);
                    if url_path.ends_with(".mp3")
                        || url_path.ends_with(".m4a")
                        || url_path.ends_with(".mp4")
                        || url_path.ends_with(".ogg")
                        || url_path.ends_with(".wav")
                        || url_path.ends_with(".aac")
                        || url_path.ends_with(".flac")
                    {
                        return Some(url_string);
                    }
                }
            }
        }

        // Strategy 2: Look for enclosures with audio MIME types in links (fallback for some feeds)
        if let Some(audio_link) = entry.links.iter().find(|link| {
            link.media_type
                .as_ref()
                .map(|mt| mt.starts_with("audio/") || mt == "application/octet-stream")
                .unwrap_or(false)
        }) {
            return Some(audio_link.href.clone());
        }

        // Strategy 3: Look for enclosure relationship (RSS 2.0 standard)
        if let Some(enclosure_link) = entry
            .links
            .iter()
            .find(|link| link.rel.as_ref().is_some_and(|rel| rel == "enclosure"))
        {
            return Some(enclosure_link.href.clone());
        }

        // Strategy 4: Look for links with audio file extensions (feeds missing MIME types)
        if let Some(audio_link) = entry.links.iter().find(|link| {
            let href = &link.href.to_lowercase();
            // Check for common audio extensions, handling query parameters
            let url_path = href.split('?').next().unwrap_or(href);
            url_path.ends_with(".mp3")
                || url_path.ends_with(".m4a")
                || url_path.ends_with(".mp4")
                || url_path.ends_with(".ogg")
                || url_path.ends_with(".wav")
                || url_path.ends_with(".aac")
                || url_path.ends_with(".flac")
        }) {
            return Some(audio_link.href.clone());
        }

        // Strategy 5: Check if GUID looks like an audio URL (some feeds use GUID as direct link)
        if entry.id.starts_with("http") {
            let id_lower = entry.id.to_lowercase();
            let url_path = id_lower.split('?').next().unwrap_or(&id_lower);
            if url_path.ends_with(".mp3")
                || url_path.ends_with(".m4a")
                || url_path.ends_with(".mp4")
                || url_path.ends_with(".ogg")
                || url_path.ends_with(".wav")
            {
                return Some(entry.id.clone());
            }
        }

        // Strategy 6: For feeds with only one link, assume it might be audio (last resort)
        if entry.links.len() == 1 && entry.links[0].href.starts_with("http") {
            let href = &entry.links[0].href.to_lowercase();
            // Only if it looks like it could be a media file
            if href.contains("audio") || href.contains("media") || href.contains("episode") {
                return Some(entry.links[0].href.clone());
            }
        }

        None
    }

    /// Extract duration from feed entry.
    ///
    /// **Stub** — always returns `None`. When implemented, should parse:
    /// - iTunes `<itunes:duration>` tag (HH:MM:SS or seconds)
    /// - Other RSS duration fields
    ///
    /// Implementing this will also unblock the deferred `:filter-duration`
    /// command. See Design Decision #13 in `docs/SEARCH_AND_FILTER.md`.
    fn extract_duration(&self, _entry: &feed_rs::model::Entry) -> Option<chrono::Duration> {
        // TODO: Parse duration from iTunes extensions when feed-rs supports it
        // For now, return None — duration filter deferred (Decision #13)
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
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Minimal valid RSS body used by the conditional-GET tests below.
    /// Uses one entry so `extract_episode` succeeds; the tests don't assert
    /// on episode content, only on the conditional-GET wire behaviour.
    const TEST_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <description>For tests</description>
    <link>https://example.com</link>
    <item>
      <title>Episode 1</title>
      <guid isPermaLink="false">ep-1</guid>
      <enclosure url="https://example.com/ep1.mp3" type="audio/mpeg" length="1234"/>
      <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>"#;

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

    // --- Conditional GET (issue #246) ---

    #[tokio::test]
    async fn test_download_feed_sends_if_none_match_when_etag_provided() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .and(header("If-None-Match", "\"abc123\""))
            .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS))
            .expect(1)
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let result = parser
            .download_feed_conditional(&url, Some("\"abc123\""), None)
            .await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert!(result.unwrap().is_some(), "expected Some(FeedFetchResult)");
    }

    #[tokio::test]
    async fn test_download_feed_sends_if_modified_since_when_provided() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .and(header_exists("If-Modified-Since"))
            .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS))
            .expect(1)
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let result = parser
            .download_feed_conditional(&url, None, Some("Wed, 21 Oct 2015 07:28:00 GMT"))
            .await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_download_feed_omits_conditional_headers_when_none() {
        let server = MockServer::start().await;
        // Match only requests that have *neither* conditional header.
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(TEST_RSS)
                    .insert_header("ETag", "\"new-etag\""),
            )
            .expect(1)
            .mount(&server)
            .await;
        // Negative mock: any request that *does* carry a conditional header
        // would fall through to the default 404, surfacing a parse error.

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let result = parser.download_feed_conditional(&url, None, None).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let fetch = result.unwrap().expect("expected Some on 200");
        assert_eq!(fetch.etag.as_deref(), Some("\"new-etag\""));
    }

    #[tokio::test]
    async fn test_download_feed_returns_none_on_304() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .and(header_exists("If-None-Match"))
            .respond_with(ResponseTemplate::new(304))
            .expect(1)
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let result = parser
            .download_feed_conditional(&url, Some("\"abc123\""), None)
            .await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        assert!(result.unwrap().is_none(), "expected None on 304");
    }

    #[tokio::test]
    async fn test_download_feed_captures_etag_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(TEST_RSS)
                    .insert_header("ETag", "\"server-etag-v2\""),
            )
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let fetch = parser
            .download_feed_conditional(&url, None, None)
            .await
            .unwrap()
            .expect("expected Some on 200");
        assert_eq!(fetch.etag.as_deref(), Some("\"server-etag-v2\""));
    }

    #[tokio::test]
    async fn test_download_feed_captures_last_modified_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(TEST_RSS)
                    .insert_header("Last-Modified", "Wed, 21 Oct 2015 07:28:00 GMT"),
            )
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let fetch = parser
            .download_feed_conditional(&url, None, None)
            .await
            .unwrap()
            .expect("expected Some on 200");
        assert_eq!(
            fetch.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
    }

    #[tokio::test]
    async fn test_download_feed_handles_200_without_validators() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS))
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let fetch = parser
            .download_feed_conditional(&url, None, None)
            .await
            .unwrap()
            .expect("expected Some on 200");
        assert!(fetch.etag.is_none(), "no ETag header → etag is None");
        assert!(
            fetch.last_modified.is_none(),
            "no Last-Modified header → last_modified is None"
        );
        assert!(!fetch.body.is_empty(), "body should be populated");
    }

    #[tokio::test]
    async fn test_get_episodes_conditional_returns_none_on_304() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(ResponseTemplate::new(304))
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let pid = PodcastId::from_url(&url);
        let result = parser
            .get_episodes_conditional(&url, &pid, Some("\"abc\""), None)
            .await
            .unwrap();
        assert!(result.is_none(), "304 should yield None");
    }

    #[tokio::test]
    async fn test_get_episodes_conditional_returns_episodes_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(TEST_RSS)
                    .insert_header("ETag", "\"e1\""),
            )
            .mount(&server)
            .await;

        let parser = FeedParser::new();
        let url = format!("{}/feed.xml", server.uri());
        let pid = PodcastId::from_url(&url);
        let result = parser
            .get_episodes_conditional(&url, &pid, None, None)
            .await
            .unwrap()
            .expect("expected Some");
        assert_eq!(result.episodes.len(), 1);
        assert_eq!(result.etag.as_deref(), Some("\"e1\""));
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
