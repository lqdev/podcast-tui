// PodcastIndex API client for podcast discovery and search
//
// Provides `PodcastIndexClient` which wraps the PodcastIndex.org REST API.
// Authentication uses a SHA-1 hash of (api_key + api_secret + unix_timestamp),
// sent as the `Authorization` header alongside `X-Auth-Key` and `X-Auth-Date`.

use crate::constants::discovery::{
    DEFAULT_TRENDING_COUNT, DISCOVERY_REQUEST_TIMEOUT, MAX_SEARCH_RESULTS,
    PODCASTINDEX_API_BASE_URL,
};
use reqwest::{header::HeaderMap, Client};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::time::{SystemTime, UNIX_EPOCH};

/// Errors that can occur during podcast discovery
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("API credentials not configured. Get a free key at https://api.podcastindex.org/")]
    NotConfigured,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Response parse error: {0}")]
    Parse(String),
}

/// A single podcast result from the PodcastIndex API
#[derive(Debug, Clone, Deserialize)]
pub struct PodcastSearchResult {
    pub title: String,
    pub author: String,
    #[serde(rename = "url")]
    pub feed_url: String,
    pub description: String,
    #[serde(rename = "artwork")]
    pub artwork_url: Option<String>,
    #[serde(default)]
    pub categories: std::collections::HashMap<String, String>,
}

impl PodcastSearchResult {
    /// Returns the category names sorted alphabetically.
    pub fn category_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.categories.values().cloned().collect();
        names.sort();
        names
    }
}

// ---------- API response shapes (private) ------------------------------------

#[derive(Debug, Deserialize)]
struct SearchResponse {
    feeds: Vec<PodcastSearchResult>,
}

#[derive(Debug, Deserialize)]
struct TrendingFeed {
    pub title: String,
    pub author: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    pub artwork: Option<String>,
    #[serde(default)]
    pub categories: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct TrendingResponse {
    feeds: Vec<TrendingFeed>,
}

// ---------- Client -----------------------------------------------------------

/// HTTP client for the PodcastIndex.org API
pub struct PodcastIndexClient {
    client: Client,
    api_key: String,
    api_secret: String,
}

impl std::fmt::Debug for PodcastIndexClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PodcastIndexClient")
            .field("api_key", &"[redacted]")
            .finish()
    }
}

impl PodcastIndexClient {
    /// Create a new client.  Returns `Err(DiscoveryError::NotConfigured)` if
    /// either credential is empty.
    pub fn new(api_key: String, api_secret: String) -> Result<Self, DiscoveryError> {
        if api_key.is_empty() || api_secret.is_empty() {
            return Err(DiscoveryError::NotConfigured);
        }
        let client = Client::builder()
            .timeout(DISCOVERY_REQUEST_TIMEOUT)
            .user_agent(crate::constants::network::USER_AGENT)
            .build()
            .map_err(DiscoveryError::Network)?;
        Ok(Self {
            client,
            api_key,
            api_secret,
        })
    }

    /// Search podcasts by keyword. Returns up to `MAX_SEARCH_RESULTS` results.
    pub async fn search(&self, query: &str) -> Result<Vec<PodcastSearchResult>, DiscoveryError> {
        let encoded = urlencoding_encode(query);
        let url = format!(
            "{}/search/byterm?q={}&max={}",
            PODCASTINDEX_API_BASE_URL, encoded, MAX_SEARCH_RESULTS
        );
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let body: SearchResponse = response
            .json()
            .await
            .map_err(|e| DiscoveryError::Parse(e.to_string()))?;
        Ok(body.feeds)
    }

    /// Fetch trending podcasts. Returns up to `DEFAULT_TRENDING_COUNT` results.
    pub async fn trending(&self) -> Result<Vec<PodcastSearchResult>, DiscoveryError> {
        let url = format!(
            "{}/podcasts/trending?max={}",
            PODCASTINDEX_API_BASE_URL, DEFAULT_TRENDING_COUNT
        );
        let response = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscoveryError::ApiError { status, message });
        }

        let body: TrendingResponse = response
            .json()
            .await
            .map_err(|e| DiscoveryError::Parse(e.to_string()))?;

        // Normalize TrendingFeed → PodcastSearchResult
        let results = body
            .feeds
            .into_iter()
            .map(|f| PodcastSearchResult {
                title: f.title,
                author: f.author,
                feed_url: f.url,
                description: f.description,
                artwork_url: f.artwork,
                categories: f.categories,
            })
            .collect();
        Ok(results)
    }

    /// Build the PodcastIndex authentication headers.
    ///
    /// PodcastIndex uses a simple timestamp + SHA-1 scheme:
    ///   hash = SHA1(api_key + api_secret + unix_epoch_seconds)
    fn auth_headers(&self) -> HeaderMap {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let raw = format!("{}{}{}", self.api_key, self.api_secret, now);
        let hash = hex_sha1(raw.as_bytes());

        let mut headers = HeaderMap::new();
        // SAFETY: api_key, decimal u64, and hex SHA-1 strings are all valid ASCII,
        // so HeaderValue::from_str() is infallible for these values.
        headers.insert("X-Auth-Key", self.api_key.parse().unwrap());
        headers.insert("X-Auth-Date", now.to_string().parse().unwrap());
        headers.insert("Authorization", hash.parse().unwrap());
        headers
    }
}

// ---------- Helpers ----------------------------------------------------------

/// Compute SHA-1 of `data` and return the lowercase hex string.
fn hex_sha1(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Minimal percent-encoding for query parameters (encode non-alphanumeric chars).
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

// ---------- Tests ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_error_when_key_empty() {
        // Arrange
        // Act
        let result = PodcastIndexClient::new(String::new(), "secret".to_string());
        // Assert
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), DiscoveryError::NotConfigured),
            "expected NotConfigured"
        );
    }

    #[test]
    fn test_new_returns_error_when_secret_empty() {
        // Arrange
        // Act
        let result = PodcastIndexClient::new("key".to_string(), String::new());
        // Assert
        assert!(matches!(result.unwrap_err(), DiscoveryError::NotConfigured));
    }

    #[test]
    fn test_new_succeeds_with_credentials() {
        // Arrange / Act
        let result = PodcastIndexClient::new("key".to_string(), "secret".to_string());
        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_hex_sha1_known_value() {
        // SHA-1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
        let hash = hex_sha1(b"");
        assert_eq!(hash, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn test_hex_sha1_hello_world() {
        // SHA-1("hello world") = 2aae6c35c94fcfb415dbe95f408b9ce91ee846ed
        let hash = hex_sha1(b"hello world");
        assert_eq!(hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }

    #[test]
    fn test_urlencoding_encode_simple() {
        // Arrange
        let input = "hello world";
        // Act
        let encoded = urlencoding_encode(input);
        // Assert
        assert_eq!(encoded, "hello+world");
    }

    #[test]
    fn test_urlencoding_encode_special_chars() {
        // Arrange
        let input = "rust & systems";
        // Act
        let encoded = urlencoding_encode(input);
        // Assert
        assert_eq!(encoded, "rust+%26+systems");
    }

    #[test]
    fn test_urlencoding_encode_alphanumeric_passthrough() {
        // Arrange
        let input = "RustProgramming123";
        // Act / Assert
        assert_eq!(urlencoding_encode(input), "RustProgramming123");
    }

    #[test]
    fn test_podcast_search_result_category_names_sorted() {
        // Arrange
        let mut categories = std::collections::HashMap::new();
        categories.insert("1".to_string(), "Technology".to_string());
        categories.insert("2".to_string(), "Education".to_string());
        let result = PodcastSearchResult {
            title: "Test".to_string(),
            author: "Author".to_string(),
            feed_url: "https://example.com/feed".to_string(),
            description: "Desc".to_string(),
            artwork_url: None,
            categories,
        };
        // Act
        let names = result.category_names();
        // Assert
        assert_eq!(names, vec!["Education", "Technology"]);
    }

    #[test]
    fn test_discovery_error_not_configured_message() {
        let err = DiscoveryError::NotConfigured;
        let msg = err.to_string();
        assert!(msg.contains("credentials not configured"));
    }
}
