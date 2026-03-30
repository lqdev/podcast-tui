//! Circuit breaker — prevents hammering a dead server.
//!
//! State machine: Closed → Open (after N failures) → HalfOpen (after cooldown) → Closed/Open.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::constants::scrobbling;

/// Observable circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    consecutive_failures: AtomicU32,
    state: Mutex<InternalState>,
}

enum InternalState {
    Closed,
    Open { opened_at: Instant },
    HalfOpen,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            consecutive_failures: AtomicU32::new(0),
            state: Mutex::new(InternalState::Closed),
        }
    }

    /// Check if a request is allowed through. Transitions Open → HalfOpen if cooldown elapsed.
    pub fn allow_request(&self) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        match *state {
            InternalState::Closed => true,
            InternalState::Open { opened_at } => {
                if opened_at.elapsed() >= scrobbling::CIRCUIT_BREAKER_RESET {
                    *state = InternalState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            InternalState::HalfOpen => true,
        }
    }

    /// Record a successful request. Resets failure count, transitions to Closed.
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        *state = InternalState::Closed;
    }

    /// Record a failed request. Increments failure count, may transition to Open.
    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if failures >= scrobbling::CIRCUIT_BREAKER_FAILURE_THRESHOLD {
            *state = InternalState::Open {
                opened_at: Instant::now(),
            };
        }
    }

    pub fn state(&self) -> CircuitState {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        match *state {
            InternalState::Closed => CircuitState::Closed,
            InternalState::Open { .. } => CircuitState::Open,
            InternalState::HalfOpen => CircuitState::HalfOpen,
        }
    }
}
