//! Integration tests for Layer 2 of the incremental-refresh sprint (issue #247):
//! body-hash short-circuit in `SubscriptionManager::refresh_feed_with_options`.
//!
//! These tests verify that when a feed server returns a `200 OK` whose body
//! is byte-identical to the previously fetched body, the refresh skips
//! `load_episodes` + dedup + per-episode save and returns `Ok(vec![])`,
//! while still persisting any new HTTP cache validators.

use anyhow::Result;
use podcast_tui::{
    podcast::{subscription::SubscriptionManager, Podcast},
    storage::{JsonStorage, Storage},
};
use std::sync::Arc;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_RSS_V1: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
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

const TEST_RSS_V2: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Podcast</title>
    <link>http://example.com</link>
    <description>Test feed</description>
    <item>
      <title>Episode 2</title>
      <guid>ep-2</guid>
      <pubDate>Thu, 22 Oct 2015 07:28:00 GMT</pubDate>
      <enclosure url="http://example.com/ep2.mp3" length="456" type="audio/mpeg"/>
    </item>
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
async fn test_first_refresh_stores_hash_and_processes() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // Simulate a server that doesn't honor conditional GET (no ETag, no
    // Last-Modified). Layer 2 must still capture the body hash.
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS_V1))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let podcast = Podcast::new("Test".to_string(), url);
    storage.save_podcast(&podcast).await?;
    assert!(podcast.last_body_hash.is_none());

    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert_eq!(new_eps.len(), 1, "first refresh should process episodes");

    let reloaded = storage.load_podcast(&podcast.id).await?;
    assert!(
        reloaded.last_body_hash.is_some(),
        "first refresh should persist body hash"
    );
    let hash = reloaded.last_body_hash.unwrap();
    assert_eq!(hash.len(), 64, "SHA-256 hex must be 64 chars");
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "hash must be lowercase hex"
    );

    Ok(())
}

#[tokio::test]
async fn test_refresh_short_circuits_on_identical_body() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // Same body, twice. No conditional headers — simulates a server that
    // doesn't implement RFC 7232 but happens to send identical bytes.
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS_V1))
        .expect(2)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let podcast = Podcast::new("Test".to_string(), url);
    storage.save_podcast(&podcast).await?;

    // First refresh: full path, processes the one episode.
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert_eq!(new_eps.len(), 1);

    // Episode persisted to storage.
    let stored = storage.load_episodes(&podcast.id).await?;
    assert_eq!(stored.len(), 1);
    let original_episode_id = stored[0].id.clone();

    // Second refresh: identical body → hash hits → short-circuit.
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert!(
        new_eps.is_empty(),
        "identical body should short-circuit and yield no new episodes"
    );

    // Verify the short-circuit didn't disturb the existing episode (no
    // re-save, no renumber). The episode ID must be identical to before.
    let stored_after = storage.load_episodes(&podcast.id).await?;
    assert_eq!(stored_after.len(), 1);
    assert_eq!(stored_after[0].id, original_episode_id);

    Ok(())
}

#[tokio::test]
async fn test_refresh_processes_changed_body() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // First request gets v1, then subsequent gets v2 (added an episode).
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS_V1))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS_V2))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let podcast = Podcast::new("Test".to_string(), url);
    storage.save_podcast(&podcast).await?;

    // First refresh: 1 episode.
    let _ = sub.refresh_feed(&podcast.id).await?;
    let stored_v1 = storage.load_episodes(&podcast.id).await?;
    assert_eq!(stored_v1.len(), 1);
    let hash_v1 = storage
        .load_podcast(&podcast.id)
        .await?
        .last_body_hash
        .unwrap();

    // Second refresh: different body → hash misses → full path runs.
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert_eq!(new_eps.len(), 1, "one new episode in v2");
    let stored_v2 = storage.load_episodes(&podcast.id).await?;
    assert_eq!(stored_v2.len(), 2);

    // Hash must have advanced.
    let hash_v2 = storage
        .load_podcast(&podcast.id)
        .await?
        .last_body_hash
        .unwrap();
    assert_ne!(hash_v1, hash_v2, "hash must advance when body changes");

    Ok(())
}

#[tokio::test]
async fn test_hard_refresh_bypasses_hash_check() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // Always serve the same body. Hard refresh must still hit the full path
    // (parse + dedup + save) instead of short-circuiting on the hash match.
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_RSS_V1))
        .expect(2)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let mut podcast = Podcast::new("Test".to_string(), url);
    // Pre-populate the matching hash so a non-hard refresh would
    // short-circuit. Hard refresh must ignore it.
    podcast.last_body_hash = Some(podcast_tui::podcast::hash_feed_body(TEST_RSS_V1));
    storage.save_podcast(&podcast).await?;

    // Sanity: a regular refresh would short-circuit (returning Vec::new()).
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert!(
        new_eps.is_empty(),
        "regular refresh with matching hash should short-circuit"
    );

    // Hard refresh must NOT short-circuit — it must run the full path,
    // which discovers the episode is new (storage was empty) and processes
    // it.
    let new_eps = sub.refresh_feed_with_options(&podcast.id, true).await?;
    assert_eq!(
        new_eps.len(),
        1,
        "hard refresh must process episodes even when hash matches"
    );

    Ok(())
}

#[tokio::test]
async fn test_body_hash_short_circuit_still_persists_etag_update() -> Result<()> {
    let (_tmp, storage, sub) = setup().await?;
    let server = MockServer::start().await;

    // Two 200s with the same body but different ETags. The second response
    // hits the hash short-circuit, but the new ETag must still be saved so
    // future refreshes can use Layer 1 (304).
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .set_body_string(TEST_RSS_V1),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v2\"")
                .set_body_string(TEST_RSS_V1),
        )
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/feed.xml", server.uri());
    let podcast = Podcast::new("Test".to_string(), url);
    storage.save_podcast(&podcast).await?;

    // First refresh: stores hash + "v1".
    let _ = sub.refresh_feed(&podcast.id).await?;
    let after_v1 = storage.load_podcast(&podcast.id).await?;
    assert_eq!(after_v1.last_etag.as_deref(), Some("\"v1\""));

    // Second refresh: same body, new ETag header. The conditional GET sends
    // `If-None-Match: "v1"` but our wiremock isn't configured to honor it,
    // so it returns 200 with the body (which then hits the hash
    // short-circuit). Either way, the new ETag must be saved.
    let new_eps = sub.refresh_feed(&podcast.id).await?;
    assert!(new_eps.is_empty(), "short-circuit yields no new episodes");

    let after_v2 = storage.load_podcast(&podcast.id).await?;
    assert_eq!(
        after_v2.last_etag.as_deref(),
        Some("\"v2\""),
        "hash short-circuit must still persist the new ETag"
    );

    Ok(())
}
