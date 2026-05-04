//! Tests for the scrobbling module.
//!
//! Covers: CircuitBreaker state machine, PersistentRetryQueue FIFO/TTL,
//! NoopScrobbler, threshold logic, ScrobbleEvent building, and model serialization.

use podcast_tui::config::ScrobblingConfig;
use podcast_tui::scrobbling::circuit_breaker::{CircuitBreaker, CircuitState};
use podcast_tui::scrobbling::models::*;
use podcast_tui::scrobbling::noop::NoopScrobbler;
use podcast_tui::scrobbling::queue::PersistentRetryQueue;
use podcast_tui::scrobbling::{meets_scrobble_threshold, ScrobbleEvent, Scrobbler};
use tempfile::TempDir;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_event(position_ms: u64, duration_ms: Option<u64>) -> ScrobbleEvent {
    ScrobbleEvent {
        podcast_title: "Test Podcast".to_string(),
        episode_title: "Episode 1".to_string(),
        feed_url: Some("https://example.com/feed.xml".to_string()),
        episode_guid: Some("guid-123".to_string()),
        duration_ms,
        position_ms,
        listened_at: 1700000000,
    }
}

fn default_config() -> ScrobblingConfig {
    ScrobblingConfig::default()
}

// ─── CircuitBreaker ──────────────────────────────────────────────────────────

#[test]
fn test_circuit_breaker_starts_closed() {
    // Arrange
    let cb = CircuitBreaker::new();

    // Act & Assert
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow_request());
}

#[test]
fn test_circuit_breaker_opens_after_threshold_failures() {
    // Arrange
    let cb = CircuitBreaker::new();

    // Act — 5 consecutive failures (threshold)
    for _ in 0..5 {
        cb.record_failure();
    }

    // Assert
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.allow_request());
}

#[test]
fn test_circuit_breaker_stays_closed_below_threshold() {
    // Arrange
    let cb = CircuitBreaker::new();

    // Act — 4 failures (below threshold of 5)
    for _ in 0..4 {
        cb.record_failure();
    }

    // Assert
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow_request());
}

#[test]
fn test_circuit_breaker_success_resets_to_closed() {
    // Arrange
    let cb = CircuitBreaker::new();
    for _ in 0..5 {
        cb.record_failure();
    }
    assert_eq!(cb.state(), CircuitState::Open);

    // Act — record a success (simulating half-open probe)
    cb.record_success();

    // Assert
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow_request());
}

#[test]
fn test_circuit_breaker_success_resets_failure_count() {
    // Arrange
    let cb = CircuitBreaker::new();

    // Act — accumulate some failures, then succeed, then more failures
    for _ in 0..3 {
        cb.record_failure();
    }
    cb.record_success();
    for _ in 0..4 {
        cb.record_failure();
    }

    // Assert — should still be closed (4 failures after reset, threshold is 5)
    assert_eq!(cb.state(), CircuitState::Closed);
}

// ─── PersistentRetryQueue ────────────────────────────────────────────────────

#[test]
fn test_queue_push_and_drain() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let queue = PersistentRetryQueue::new(tmp.path(), 500, 30);

    // Act
    queue.push(make_event(1000, Some(60000)));
    queue.push(make_event(2000, Some(60000)));

    // Assert
    assert_eq!(queue.len(), 2);
    let events = queue.drain();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].position_ms, 1000);
    assert_eq!(events[1].position_ms, 2000);
    assert!(queue.is_empty());
}

#[test]
fn test_queue_fifo_eviction_at_capacity() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let queue = PersistentRetryQueue::new(tmp.path(), 3, 30);

    // Act — push 4 events into a queue with max_size=3
    queue.push(make_event(1000, None));
    queue.push(make_event(2000, None));
    queue.push(make_event(3000, None));
    queue.push(make_event(4000, None));

    // Assert — oldest should be evicted
    assert_eq!(queue.len(), 3);
    let events = queue.drain();
    assert_eq!(events[0].position_ms, 2000);
    assert_eq!(events[1].position_ms, 3000);
    assert_eq!(events[2].position_ms, 4000);
}

#[test]
fn test_queue_requeue_prepends() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let queue = PersistentRetryQueue::new(tmp.path(), 500, 30);
    queue.push(make_event(3000, None));

    // Act — requeue failed events (they go to the front)
    let failed = vec![make_event(1000, None), make_event(2000, None)];
    queue.requeue(failed);

    // Assert
    let events = queue.drain();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].position_ms, 1000);
    assert_eq!(events[1].position_ms, 2000);
    assert_eq!(events[2].position_ms, 3000);
}

#[test]
fn test_queue_persists_to_disk() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    {
        let queue = PersistentRetryQueue::new(tmp.path(), 500, 30);
        queue.push(make_event(42000, Some(120000)));
    }

    // Act — create a new queue from the same directory (simulates restart)
    let queue2 = PersistentRetryQueue::new(tmp.path(), 500, 30);

    // Assert
    assert_eq!(queue2.len(), 1);
    let events = queue2.drain();
    assert_eq!(events[0].position_ms, 42000);
}

#[test]
fn test_queue_empty_on_missing_file() {
    // Arrange
    let tmp = TempDir::new().unwrap();

    // Act — no file exists
    let queue = PersistentRetryQueue::new(tmp.path(), 500, 30);

    // Assert
    assert!(queue.is_empty());
}

// ─── NoopScrobbler ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_noop_scrobbler_playing_now_succeeds() {
    // Arrange
    let scrobbler = NoopScrobbler;
    let event = make_event(0, Some(60000));

    // Act & Assert
    assert!(scrobbler.playing_now(&event).await.is_ok());
}

#[tokio::test]
async fn test_noop_scrobbler_scrobble_succeeds() {
    // Arrange
    let scrobbler = NoopScrobbler;
    let event = make_event(30000, Some(60000));

    // Act & Assert
    assert!(scrobbler.scrobble(&event).await.is_ok());
}

#[tokio::test]
async fn test_noop_scrobbler_flush_returns_zero() {
    // Arrange
    let scrobbler = NoopScrobbler;

    // Act & Assert
    assert_eq!(scrobbler.flush_pending().await.unwrap(), 0);
}

#[test]
fn test_noop_scrobbler_pending_count_is_zero() {
    let scrobbler = NoopScrobbler;
    assert_eq!(scrobbler.pending_count(), 0);
}

#[test]
fn test_noop_scrobbler_circuit_state_is_closed() {
    let scrobbler = NoopScrobbler;
    assert_eq!(scrobbler.circuit_state(), CircuitState::Closed);
}

// ─── meets_scrobble_threshold ────────────────────────────────────────────────

#[test]
fn test_threshold_met_when_both_conditions_satisfied() {
    // Arrange — 50% of 60min = 30min > 5min, and 50% > 25%
    let event = make_event(1_800_000, Some(3_600_000)); // 30 min of 60 min
    let config = default_config();

    // Act & Assert
    assert!(meets_scrobble_threshold(&event, &config));
}

#[test]
fn test_threshold_not_met_when_too_short() {
    // Arrange — 100% of 1min = 1min < 5min threshold
    let event = make_event(60_000, Some(60_000)); // 60s of 60s
    let config = default_config();

    // Act & Assert — meets percent (100%) but not time (60s < 300s)
    assert!(!meets_scrobble_threshold(&event, &config));
}

#[test]
fn test_threshold_not_met_when_percent_too_low() {
    // Arrange — 10% of 90min = 9min > 5min, but 10% < 25%
    let event = make_event(540_000, Some(5_400_000)); // 9 min of 90 min
    let config = default_config();

    // Act & Assert — meets time (9min > 5min) but not percent (10% < 25%)
    assert!(!meets_scrobble_threshold(&event, &config));
}

#[test]
fn test_threshold_with_unknown_duration_only_checks_time() {
    // Arrange — duration unknown, but listened 10 minutes
    let event = make_event(600_000, None); // 10 min, no duration
    let config = default_config();

    // Act & Assert — time threshold met, percent skipped
    assert!(meets_scrobble_threshold(&event, &config));
}

#[test]
fn test_threshold_with_unknown_duration_fails_on_short_listen() {
    // Arrange — duration unknown, listened only 2 minutes
    let event = make_event(120_000, None); // 2 min, no duration
    let config = default_config();

    // Act & Assert — time threshold not met (120s < 300s)
    assert!(!meets_scrobble_threshold(&event, &config));
}

#[test]
fn test_threshold_exact_boundary() {
    // Arrange — exactly at both thresholds: 25% of 20min = 5min = 300s
    let event = make_event(300_000, Some(1_200_000)); // 5 min of 20 min = 25%
    let config = default_config();

    // Act & Assert — both thresholds met (>=)
    assert!(meets_scrobble_threshold(&event, &config));
}

#[test]
fn test_threshold_just_below_time() {
    // Arrange — 25% of 20min but only 299 seconds
    let event = make_event(299_000, Some(1_200_000)); // 4:59 of 20 min
    let config = default_config();

    // Act & Assert — time not met (299s < 300s)
    assert!(!meets_scrobble_threshold(&event, &config));
}

// ─── ScrobbleEvent construction ──────────────────────────────────────────────

#[test]
fn test_scrobble_event_fields() {
    // Arrange & Act
    let event = make_event(1_500_000, Some(3_600_000));

    // Assert
    assert_eq!(event.podcast_title, "Test Podcast");
    assert_eq!(event.episode_title, "Episode 1");
    assert_eq!(
        event.feed_url.as_deref(),
        Some("https://example.com/feed.xml")
    );
    assert_eq!(event.episode_guid.as_deref(), Some("guid-123"));
    assert_eq!(event.duration_ms, Some(3_600_000));
    assert_eq!(event.position_ms, 1_500_000);
    assert_eq!(event.listened_at, 1700000000);
}

// ─── Model serialization ─────────────────────────────────────────────────────

#[test]
fn test_playing_now_request_serialization() {
    // Arrange
    let request = SubmitListensRequest {
        listen_type: ListenType::PlayingNow,
        payload: vec![ListenPayload {
            listened_at: None,
            track_metadata: TrackMetadata {
                artist_name: "Rust in Production".to_string(),
                track_name: "Episode 42: Error Handling".to_string(),
                additional_info: Some(AdditionalInfo {
                    media_player: "podcast-tui".to_string(),
                    podcast_feed_url: Some("https://example.com/feed.xml".to_string()),
                    episode_guid: Some("abc-123".to_string()),
                    duration_ms: Some(3_600_000),
                    position_ms: Some(0),
                    percent_complete: None,
                }),
            },
        }],
    };

    // Act
    let json = serde_json::to_value(&request).unwrap();

    // Assert
    assert_eq!(json["listen_type"], "playing_now");
    assert!(json["payload"][0]["listened_at"].is_null());
    assert_eq!(
        json["payload"][0]["track_metadata"]["artist_name"],
        "Rust in Production"
    );
    assert_eq!(
        json["payload"][0]["track_metadata"]["track_name"],
        "Episode 42: Error Handling"
    );
    assert_eq!(
        json["payload"][0]["track_metadata"]["additional_info"]["media_player"],
        "podcast-tui"
    );
    // percent_complete should be omitted (skip_serializing_if)
    assert!(json["payload"][0]["track_metadata"]["additional_info"]
        .get("percent_complete")
        .is_none());
}

#[test]
fn test_single_listen_request_serialization() {
    // Arrange
    let request = SubmitListensRequest {
        listen_type: ListenType::Single,
        payload: vec![ListenPayload {
            listened_at: Some(1740000000),
            track_metadata: TrackMetadata {
                artist_name: "Rust in Production".to_string(),
                track_name: "Episode 42: Error Handling".to_string(),
                additional_info: Some(AdditionalInfo {
                    media_player: "podcast-tui".to_string(),
                    podcast_feed_url: Some("https://example.com/feed.xml".to_string()),
                    episode_guid: Some("abc-123".to_string()),
                    duration_ms: Some(3_600_000),
                    position_ms: Some(2_700_000),
                    percent_complete: Some(75.0),
                }),
            },
        }],
    };

    // Act
    let json = serde_json::to_value(&request).unwrap();

    // Assert
    assert_eq!(json["listen_type"], "single");
    assert_eq!(json["payload"][0]["listened_at"], 1740000000);
    assert_eq!(
        json["payload"][0]["track_metadata"]["additional_info"]["percent_complete"],
        75.0
    );
    assert_eq!(
        json["payload"][0]["track_metadata"]["additional_info"]["duration_ms"],
        3_600_000
    );
    assert_eq!(
        json["payload"][0]["track_metadata"]["additional_info"]["position_ms"],
        2_700_000
    );
}

#[test]
fn test_minimal_request_omits_optional_fields() {
    // Arrange — no additional_info
    let request = SubmitListensRequest {
        listen_type: ListenType::Single,
        payload: vec![ListenPayload {
            listened_at: Some(1740000000),
            track_metadata: TrackMetadata {
                artist_name: "My Podcast".to_string(),
                track_name: "Ep 1".to_string(),
                additional_info: None,
            },
        }],
    };

    // Act
    let json = serde_json::to_value(&request).unwrap();

    // Assert — additional_info should be omitted entirely
    assert!(json["payload"][0]["track_metadata"]
        .get("additional_info")
        .is_none());
}

// ─── ScrobblingConfig defaults ───────────────────────────────────────────────

#[test]
fn test_scrobbling_config_defaults() {
    // Arrange & Act
    let config = ScrobblingConfig::default();

    // Assert
    assert!(!config.enabled);
    assert!(config.endpoint.is_none());
    assert!(config.token.is_none());
    assert_eq!(config.username, "default");
    assert_eq!(config.min_listen_percent, 25);
    assert_eq!(config.min_listen_seconds, 300);
    assert!(config.submit_playing_now);
    assert_eq!(config.timeout_secs, 5);
    assert_eq!(config.max_retry_queue_size, 500);
    assert_eq!(config.retry_queue_ttl_days, 30);
}

#[test]
fn test_scrobbling_config_backward_compat() {
    // Arrange — serialize a default Config, then strip the scrobbling key to simulate
    // an existing user's config.json that was created before scrobbling existed.
    let default_config = podcast_tui::config::Config::default();
    let mut json_value: serde_json::Value = serde_json::to_value(&default_config).unwrap();
    json_value.as_object_mut().unwrap().remove("scrobbling");
    let json = serde_json::to_string(&json_value).unwrap();

    // Act — deserialize without the scrobbling key
    let config: podcast_tui::config::Config = serde_json::from_str(&json).unwrap();

    // Assert — scrobbling should use defaults
    assert!(!config.scrobbling.enabled);
    assert!(config.scrobbling.endpoint.is_none());
}

// ─── create_scrobbler factory ────────────────────────────────────────────────

#[test]
fn test_create_scrobbler_disabled_returns_noop() {
    // Arrange
    let config = ScrobblingConfig::default(); // enabled: false
    let tmp = TempDir::new().unwrap();

    // Act
    let scrobbler = podcast_tui::scrobbling::create_scrobbler(&config, tmp.path());

    // Assert — should be noop (pending_count always 0, circuit always closed)
    assert_eq!(scrobbler.pending_count(), 0);
    assert_eq!(scrobbler.circuit_state(), CircuitState::Closed);
}

#[test]
fn test_create_scrobbler_enabled_no_endpoint_returns_noop() {
    // Arrange
    let config = ScrobblingConfig {
        enabled: true,
        // endpoint is None
        ..Default::default()
    };
    let tmp = TempDir::new().unwrap();

    // Act
    let scrobbler = podcast_tui::scrobbling::create_scrobbler(&config, tmp.path());

    // Assert — noop because no endpoint
    assert_eq!(scrobbler.pending_count(), 0);
}

#[test]
fn test_create_scrobbler_enabled_with_endpoint_returns_real_client() {
    // Arrange
    let config = ScrobblingConfig {
        enabled: true,
        endpoint: Some("http://localhost:5000".to_string()),
        ..Default::default()
    };
    let tmp = TempDir::new().unwrap();

    // Act
    let scrobbler = podcast_tui::scrobbling::create_scrobbler(&config, tmp.path());

    // Assert — should be a real client (circuit starts closed, queue empty)
    assert_eq!(scrobbler.pending_count(), 0);
    assert_eq!(scrobbler.circuit_state(), CircuitState::Closed);
}
