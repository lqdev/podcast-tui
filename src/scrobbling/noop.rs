//! No-op scrobbler — silently discards all events.
//!
//! Used when scrobbling is disabled in config or when the endpoint is not configured.

use async_trait::async_trait;

use crate::scrobbling::{CircuitState, ScrobbleEvent, Scrobbler, ScrobblerError};

pub struct NoopScrobbler;

#[async_trait]
impl Scrobbler for NoopScrobbler {
    async fn playing_now(&self, _event: &ScrobbleEvent) -> Result<(), ScrobblerError> {
        Ok(())
    }

    async fn scrobble(&self, _event: &ScrobbleEvent) -> Result<(), ScrobblerError> {
        Ok(())
    }

    async fn flush_pending(&self) -> Result<usize, ScrobblerError> {
        Ok(0)
    }

    fn pending_count(&self) -> usize {
        0
    }

    fn circuit_state(&self) -> CircuitState {
        CircuitState::Closed
    }
}
