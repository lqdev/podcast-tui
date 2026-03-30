//! ListenBrainz HTTP client with resilience (circuit breaker + retry queue).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::config::ScrobblingConfig;
use crate::scrobbling::circuit_breaker::{CircuitBreaker, CircuitState};
use crate::scrobbling::models::*;
use crate::scrobbling::queue::PersistentRetryQueue;
use crate::scrobbling::{ScrobbleEvent, Scrobbler, ScrobblerError};

pub struct ListenBrainzClient {
    client: Client,
    endpoint: String,
    token: Option<String>,
    queue: Arc<PersistentRetryQueue>,
    breaker: Arc<CircuitBreaker>,
}

impl ListenBrainzClient {
    pub fn new(
        config: &ScrobblingConfig,
        endpoint: &str,
        data_dir: &std::path::Path,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent(crate::constants::network::USER_AGENT)
            .build()?;

        let queue = Arc::new(PersistentRetryQueue::new(
            data_dir,
            config.max_retry_queue_size,
            config.retry_queue_ttl_days,
        ));

        Ok(Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            token: config.token.clone(),
            queue,
            breaker: Arc::new(CircuitBreaker::new()),
        })
    }

    /// Build the `Authorization: Token <token>` header.
    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Token {t}"))
    }

    /// POST to /1/submit-listens.
    async fn submit(
        &self,
        body: &SubmitListensRequest,
    ) -> Result<SubmitListensResponse, ScrobblerError> {
        if !self.breaker.allow_request() {
            return Err(ScrobblerError::CircuitOpen);
        }

        let url = format!("{}/1/submit-listens", self.endpoint);
        let mut req = self.client.post(&url).json(body);
        if let Some(ref auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if resp.status().is_success() {
                    self.breaker.record_success();
                    let parsed: SubmitListensResponse = resp.json().await?;
                    Ok(parsed)
                } else {
                    self.breaker.record_failure();
                    let body = resp.text().await.unwrap_or_default();
                    Err(ScrobblerError::ServerError { status, body })
                }
            }
            Err(e) => {
                self.breaker.record_failure();
                Err(ScrobblerError::Http(e))
            }
        }
    }

    /// Convert a ScrobbleEvent into a `single` listen request.
    fn build_single_request(event: &ScrobbleEvent) -> SubmitListensRequest {
        let percent = event.duration_ms.map(|d| {
            if d > 0 {
                (event.position_ms as f64 / d as f64) * 100.0
            } else {
                0.0
            }
        });

        SubmitListensRequest {
            listen_type: ListenType::Single,
            payload: vec![ListenPayload {
                listened_at: Some(event.listened_at),
                track_metadata: TrackMetadata {
                    artist_name: event.podcast_title.clone(),
                    track_name: event.episode_title.clone(),
                    additional_info: Some(AdditionalInfo {
                        media_player: "podcast-tui".to_string(),
                        podcast_feed_url: event.feed_url.clone(),
                        episode_guid: event.episode_guid.clone(),
                        duration_ms: event.duration_ms,
                        position_ms: Some(event.position_ms),
                        percent_complete: percent,
                    }),
                },
            }],
        }
    }

    /// Convert a ScrobbleEvent into a `playing_now` request.
    fn build_playing_now_request(event: &ScrobbleEvent) -> SubmitListensRequest {
        SubmitListensRequest {
            listen_type: ListenType::PlayingNow,
            payload: vec![ListenPayload {
                listened_at: None,
                track_metadata: TrackMetadata {
                    artist_name: event.podcast_title.clone(),
                    track_name: event.episode_title.clone(),
                    additional_info: Some(AdditionalInfo {
                        media_player: "podcast-tui".to_string(),
                        podcast_feed_url: event.feed_url.clone(),
                        episode_guid: event.episode_guid.clone(),
                        duration_ms: event.duration_ms,
                        position_ms: Some(event.position_ms),
                        percent_complete: None,
                    }),
                },
            }],
        }
    }
}

#[async_trait]
impl Scrobbler for ListenBrainzClient {
    async fn playing_now(&self, event: &ScrobbleEvent) -> Result<(), ScrobblerError> {
        let body = Self::build_playing_now_request(event);
        self.submit(&body).await.map(|_| ())
    }

    async fn scrobble(&self, event: &ScrobbleEvent) -> Result<(), ScrobblerError> {
        let body = Self::build_single_request(event);
        match self.submit(&body).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Queue for retry on failure
                self.queue.push(event.clone());
                Err(e)
            }
        }
    }

    async fn flush_pending(&self) -> Result<usize, ScrobblerError> {
        if self.queue.is_empty() {
            return Ok(0);
        }
        if !self.breaker.allow_request() {
            return Err(ScrobblerError::CircuitOpen);
        }

        let events = self.queue.drain();
        let mut sent = 0usize;
        let mut failed = Vec::new();

        for event in events {
            let body = Self::build_single_request(&event);
            match self.submit(&body).await {
                Ok(_) => sent += 1,
                Err(_) => {
                    failed.push(event);
                    break; // Stop on first failure (circuit breaker will handle)
                }
            }
        }

        if !failed.is_empty() {
            self.queue.requeue(failed);
        }

        Ok(sent)
    }

    fn pending_count(&self) -> usize {
        self.queue.len()
    }

    fn circuit_state(&self) -> CircuitState {
        self.breaker.state()
    }
}
