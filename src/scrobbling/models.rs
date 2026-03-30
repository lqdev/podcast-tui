//! Request and response models for the ListenBrainz API.

use serde::{Deserialize, Serialize};

/// Top-level request body for `POST /1/submit-listens`.
#[derive(Debug, Serialize)]
pub struct SubmitListensRequest {
    pub listen_type: ListenType,
    pub payload: Vec<ListenPayload>,
}

/// The type of listen being submitted.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenType {
    PlayingNow,
    Single,
    Import,
}

/// A single listen event in the payload array.
#[derive(Debug, Serialize)]
pub struct ListenPayload {
    /// Unix timestamp (seconds). Omitted for `playing_now`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listened_at: Option<i64>,
    pub track_metadata: TrackMetadata,
}

/// Track metadata — maps podcast/episode info to ListenBrainz fields.
#[derive(Debug, Serialize)]
pub struct TrackMetadata {
    /// Podcast name (maps to LB "artist")
    pub artist_name: String,
    /// Episode title (maps to LB "track")
    pub track_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_info: Option<AdditionalInfo>,
}

/// Podcast-specific metadata stored in `additional_info`.
#[derive(Debug, Serialize)]
pub struct AdditionalInfo {
    pub media_player: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub podcast_feed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent_complete: Option<f64>,
}

/// Response from `POST /1/submit-listens`.
#[derive(Debug, Deserialize)]
pub struct SubmitListensResponse {
    pub status: String,
}

/// Response from `GET /1/validate-token`.
#[derive(Debug, Deserialize)]
pub struct ValidateTokenResponse {
    pub valid: bool,
}
