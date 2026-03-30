//! Persistent retry queue for failed scrobbles.
//!
//! Stores events as a JSON array in `pending_scrobbles.json`.
//! FIFO eviction at capacity, TTL-based expiry on load.
//!
//! Uses `std::fs` (not `tokio::fs`) intentionally — methods are synchronous
//! and called inside a `std::sync::Mutex` lock. The file is small (≤500 entries)
//! so the brief blocking is acceptable.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::scrobbling::ScrobbleEvent;

pub struct PersistentRetryQueue {
    path: PathBuf,
    events: Mutex<Vec<TimestampedEvent>>,
    max_size: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TimestampedEvent {
    queued_at: i64,
    event: ScrobbleEvent,
}

impl PersistentRetryQueue {
    pub fn new(data_dir: &std::path::Path, max_size: usize, ttl_days: u32) -> Self {
        let path = data_dir.join("pending_scrobbles.json");
        let events = Self::load_from_disk(&path, ttl_days);
        Self {
            path,
            events: Mutex::new(events),
            max_size,
        }
    }

    /// Push an event onto the queue. Evicts oldest if at capacity.
    pub fn push(&self, event: ScrobbleEvent) {
        let mut events = self.events.lock().unwrap();
        if events.len() >= self.max_size {
            events.remove(0); // FIFO eviction
        }
        events.push(TimestampedEvent {
            queued_at: chrono::Utc::now().timestamp(),
            event,
        });
        let _ = Self::save_to_disk(&self.path, &events);
    }

    /// Take all events out of the queue (drains it).
    pub fn drain(&self) -> Vec<ScrobbleEvent> {
        let mut events = self.events.lock().unwrap();
        let drained: Vec<ScrobbleEvent> = events.iter().map(|e| e.event.clone()).collect();
        events.clear();
        let _ = Self::save_to_disk(&self.path, &events);
        drained
    }

    /// Re-enqueue events that failed to send (prepend to front).
    pub fn requeue(&self, failed: Vec<ScrobbleEvent>) {
        let mut events = self.events.lock().unwrap();
        let mut requeued: Vec<TimestampedEvent> = failed
            .into_iter()
            .map(|event| TimestampedEvent {
                queued_at: chrono::Utc::now().timestamp(),
                event,
            })
            .collect();
        requeued.extend(events.drain(..));
        // Trim to max_size (keep newest)
        while requeued.len() > self.max_size {
            requeued.remove(0);
        }
        *events = requeued;
        let _ = Self::save_to_disk(&self.path, &events);
    }

    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn load_from_disk(path: &std::path::Path, ttl_days: u32) -> Vec<TimestampedEvent> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let events: Vec<TimestampedEvent> = match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        let cutoff = chrono::Utc::now().timestamp() - (ttl_days as i64 * 86400);
        events
            .into_iter()
            .filter(|e| e.queued_at >= cutoff)
            .collect()
    }

    fn save_to_disk(
        path: &std::path::Path,
        events: &[TimestampedEvent],
    ) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(events)
            .map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}
