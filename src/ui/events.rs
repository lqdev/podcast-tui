// Event handling system for the UI
//
// This module provides the core event system for handling keyboard input,
// converting them to UI actions, and managing the event loop.

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::ui::{UIAction, UIError, UIResult};

/// UI event handler for processing terminal events
#[derive(Clone)]
pub struct UIEventHandler {
    tick_rate: Duration,
}

impl UIEventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Run the event loop, sending events to the provided channel
    pub async fn run(&self, event_tx: mpsc::UnboundedSender<UIEvent>) {
        let mut last_tick = Instant::now();

        loop {
            let timeout = self
                .tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::ZERO);

            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(crossterm_event) => {
                        let ui_event = Self::convert_event(crossterm_event);
                        if event_tx.send(ui_event).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                if event_tx.send(UIEvent::Tick).is_err() {
                    break;
                }
                last_tick = Instant::now();
            }
        }
    }

    /// Convert crossterm events to UI events
    fn convert_event(event: Event) -> UIEvent {
        match event {
            Event::Key(key) => UIEvent::Key(key),
            Event::Mouse(mouse) => UIEvent::Mouse(mouse),
            Event::Resize(w, h) => UIEvent::Resize(w, h),
            _ => UIEvent::Tick,
        }
    }
}

/// UI events that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum UIEvent {
    /// Keyboard input
    Key(crossterm::event::KeyEvent),

    /// Mouse input
    Mouse(crossterm::event::MouseEvent),

    /// Terminal resize
    Resize(u16, u16),

    /// Periodic tick
    Tick,

    /// Application should quit
    Quit,
}

/// Application events for async communication
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Podcast subscription completed successfully
    PodcastSubscribed { podcast: crate::podcast::Podcast },

    /// Podcast subscription failed
    PodcastSubscriptionFailed { url: String, error: String },

    /// Podcast refresh completed
    PodcastRefreshed {
        podcast_id: crate::storage::PodcastId,
        new_episode_count: usize,
    },

    /// Podcast refresh failed
    PodcastRefreshFailed {
        podcast_id: crate::storage::PodcastId,
        error: String,
    },

    /// All podcasts refresh completed
    AllPodcastsRefreshed { total_new_episodes: usize },

    /// Podcast deleted successfully
    PodcastDeleted {
        podcast_id: crate::storage::PodcastId,
        podcast_title: String,
    },

    /// Podcast deletion failed
    PodcastDeletionFailed {
        podcast_id: crate::storage::PodcastId,
        error: String,
    },

    /// Episodes loaded successfully
    EpisodesLoaded {
        podcast_id: crate::storage::PodcastId,
        podcast_name: String,
        episodes: Vec<crate::podcast::Episode>,
    },

    /// Episodes loading failed
    EpisodesLoadFailed {
        podcast_id: crate::storage::PodcastId,
        error: String,
    },

    /// Episode download completed successfully
    EpisodeDownloaded {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    },

    /// Episode download failed
    EpisodeDownloadFailed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        error: String,
    },

    /// Episode download deleted successfully
    EpisodeDownloadDeleted {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
    },

    /// Episode download deletion failed
    EpisodeDownloadDeletionFailed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        error: String,
    },

    /// Downloads buffer refreshed
    DownloadsRefreshed,
}
