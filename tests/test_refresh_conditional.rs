//! Integration tests for HTTP conditional GET on feed refresh (issue #246).
//!
//! These tests verify the end-to-end behavior of `SubscriptionManager::refresh_feed_with_options`:
//!   - 304 Not Modified short-circuits (no episodes returned, validators preserved).
//!   - 200 OK persists the new ETag/Last-Modified headers to storage.
//!   - Hard refresh bypasses the cache (sends no conditional headers).
//!
//! Wire-level header parsing/sending is covered by unit tests in `src/podcast/feed.rs`.
//! These tests focus on the integration with `SubscriptionManager` and `JsonStorage`.

use anyhow::Result;
use podcast_tui::{
    podcast::{subscription::SubscriptionManager, Podcast},
    storage::{JsonStorage, Storage},
};
use std::sync::Arc;
use tempfile::TempDir;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const TEST_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Podcast</title>
    <link>http://example.com</link>
    <description>Test feed</description>
    <item>
      <title>Episode 1</title>
      <guid>ep-1</guid>
      <pubDate>Wed, 21 Oct 2015 07:28:00 GMT</pubDate>
      <enclosure url="http://example.com/ep1.mp3" length="123" type="audio/mpeg"/>
    </item>
  </channel>
</rss>"#;

async fn setup() -> Result<(
    TempDir,
    Arc<JsonStorage>,
    Arc<SubscriptionManager<JsonStorage>>,
)> {
    let temp_dir = TempDir::new()?;
    let storage = Arc::new(JsonStorage::with_data_dir(temp_dir.path().to_path_buf()));
    storage.initialize().await?;
    let sub = Arc::new(SubscriptionManager::new(storage.clone()));
    Ok((temp_dir, storage, sub))
}

#[tokio::test]
async fn test_refresh_short_circuits_on_304() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // First call: serve a 200 with an ETag so we have something to send back.
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .set_body_string(TEST_RSS),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Subsequent calls: only match when If-None-Match is present, return 304.
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .and(header_exists("If-None-Match"))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let mut podcast = Podcast::new("Test".to_string(), url.clone());
    storage.save_podcast(&podcast).await?;

    // Initial refresh: fetches the feed, persists ETag.
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert_eq!(new_eps.len(), 1);
    podcast = storage.load_podcast(&podcast.id).await?;
    assert_eq!(podcast.last_etag.as_deref(), Some("\"v1\""));

    // Second refresh: server returns 304, refresh returns no new episodes.
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert!(
        new_eps.is_empty(),
        "304 should short-circuit and yield no new episodes"
    );

    // Validators are preserved across the 304 path.
    let podcast = storage.load_podcast(&podcast.id).await?;
    assert_eq!(podcast.last_etag.as_deref(), Some("\"v1\""));

    Ok(())
}

#[tokio::test]
async fn test_refresh_persists_etag_after_200() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"abc123\"")
                .insert_header("Last-Modified", "Wed, 21 Oct 2015 07:28:00 GMT")
                .set_body_string(TEST_RSS),
        )
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let podcast = Podcast::new("Test".to_string(), url);
    storage.save_podcast(&podcast).await?;

    sub.refresh_feed(&podcast.id).await?;

    let reloaded = storage.load_podcast(&podcast.id).await?;
    assert_eq!(reloaded.last_etag.as_deref(), Some("\"abc123\""));
    assert_eq!(
        reloaded.last_modified.as_deref(),
        Some("Wed, 21 Oct 2015 07:28:00 GMT")
    );

    Ok(())
}

#[tokio::test]
async fn test_refresh_preserves_validators_when_200_has_no_headers() -> Result<()> {
    // 200 OK with no ETag/Last-Modified headers must not erase the previously
    // stored validators — some servers omit these headers intermittently and
    // we should keep the last good ones for the next conditional request.
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let mut podcast = Podcast::new("Test".to_string(), url);
    podcast.last_etag = Some("\"keep-me\"".to_string());
    podcast.last_modified = Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string());
    storage.save_podcast(&podcast).await?;

    sub.refresh_feed(&podcast.id).await?;

    let reloaded = storage.load_podcast(&podcast.id).await?;
    assert_eq!(reloaded.last_etag.as_deref(), Some("\"keep-me\""));
    assert_eq!(
        reloaded.last_modified.as_deref(),
        Some("Wed, 21 Oct 2015 07:28:00 GMT")
    );

    Ok(())
}

#[tokio::test]
async fn test_hard_refresh_ignores_stored_etag() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // Match every request and assert at the request-recorder level that
    // no `If-None-Match` is present even though the podcast has one stored.
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let mut podcast = Podcast::new("Test".to_string(), url);
    podcast.last_etag = Some("\"stale\"".to_string());
    podcast.last_modified = Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string());
    storage.save_podcast(&podcast).await?;

    let _ = sub.refresh_feed_with_options(&podcast.id, true).await?;

    // Inspect what wiremock actually received.
    let received: Vec<Request> = server.received_requests().await.unwrap_or_default();
    assert_eq!(received.len(), 1, "exactly one request expected");
    let req = &received[0];
    assert!(
        req.headers.get("If-None-Match").is_none(),
        "hard refresh must not send If-None-Match"
    );
    assert!(
        req.headers.get("If-Modified-Since").is_none(),
        "hard refresh must not send If-Modified-Since"
    );

    Ok(())
}
