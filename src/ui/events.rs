// Event handling system for the UI
//
// This module provides the core event system for handling keyboard input,
// converting them to UI actions, and managing the event loop.

use crossterm::event::{self, Event, KeyEventKind};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

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

            // Use a timeout to prevent hanging
            let poll_timeout = std::cmp::min(timeout, Duration::from_millis(100));

            // Use spawn_blocking to handle the blocking crossterm calls
            let poll_result = tokio::time::timeout(
                Duration::from_millis(200),
                tokio::task::spawn_blocking(move || event::poll(poll_timeout)),
            )
            .await;

            match poll_result {
                Ok(Ok(Ok(true))) => {
                    // Event is available, read it
                    let read_result = tokio::time::timeout(
                        Duration::from_millis(100),
                        tokio::task::spawn_blocking(event::read),
                    )
                    .await;

                    match read_result {
                        Ok(Ok(Ok(crossterm_event))) => {
                            let ui_event = Self::convert_event(crossterm_event);
                            if event_tx.send(ui_event).is_err() {
                                break;
                            }
                        }
                        Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => {
                            // Error or timeout, continue
                        }
                    }
                }
                Ok(Ok(Ok(false))) => {
                    // No event available, continue to tick check
                }
                Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => {
                    // Error or timeout, continue
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                if event_tx.send(UIEvent::Tick).is_err() {
                    break;
                }
                last_tick = Instant::now();
            }

            // Small yield to prevent tight loop
            tokio::task::yield_now().await;
        }
    }

    /// Convert crossterm events to UI events
    fn convert_event(event: Event) -> UIEvent {
        match event {
            Event::Key(key) => {
                // On Windows, crossterm fires both Press and Release events.
                // We only want to handle Press events to avoid duplicate input.
                if key.kind == KeyEventKind::Press {
                    UIEvent::Key(key)
                } else {
                    UIEvent::Tick
                }
            }
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
    PodcastSubscribed {
        podcast: crate::podcast::Podcast,
    },

    /// Podcast subscription failed
    PodcastSubscriptionFailed {
        url: String,
        error: String,
    },

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
    AllPodcastsRefreshed {
        total_new_episodes: usize,
    },

    /// Background buffer data refreshed
    BufferDataRefreshed {
        buffer_type: BufferRefreshType,
        data: BufferRefreshData,
    },

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

    /// Podcast downloads deleted during unsubscribe
    PodcastDownloadsDeleted {
        podcast_id: crate::storage::PodcastId,
        deleted_count: usize,
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

    /// Episode marked as played successfully
    EpisodeMarkedPlayed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    },

    /// Episode mark as played failed
    EpisodeMarkPlayedFailed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        error: String,
    },

    /// Episode marked as unplayed successfully
    EpisodeMarkedUnplayed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
    },

    /// Episode mark as unplayed failed
    EpisodeMarkUnplayedFailed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        error: String,
    },

    /// Episode favorite toggled successfully
    EpisodeFavoriteToggled {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        episode_title: String,
        favorited: bool,
    },

    /// Episode favorite toggle failed
    EpisodeFavoriteToggleFailed {
        podcast_id: crate::storage::PodcastId,
        episode_id: crate::storage::EpisodeId,
        error: String,
    },

    /// Downloads buffer refreshed
    DownloadsRefreshed,

    /// All downloads deleted successfully
    AllDownloadsDeleted {
        deleted_count: usize,
    },

    /// All downloads deletion failed
    AllDownloadsDeletionFailed {
        error: String,
    },

    // OPML Import/Export events
    /// OPML import started
    OpmlImportStarted {
        source: String,
    },

    /// OPML import progress update
    OpmlImportProgress {
        current: usize,
        total: usize,
        status: String,
    },

    /// OPML import completed
    OpmlImportCompleted {
        result: crate::podcast::ImportResult,
        log_path: String,
    },

    /// OPML import failed
    OpmlImportFailed {
        source: String,
        error: String,
    },

    /// OPML export started
    OpmlExportStarted {
        path: String,
    },

    /// OPML export progress update
    OpmlExportProgress {
        status: String,
    },

    /// OPML export completed
    OpmlExportCompleted {
        path: String,
        feed_count: usize,
    },

    /// OPML export failed
    OpmlExportFailed {
        path: String,
        error: String,
    },

    // Playlist events
    PlaylistCreated {
        playlist: crate::playlist::Playlist,
    },
    PlaylistCreationFailed {
        name: String,
        error: String,
    },
    PlaylistDeleted {
        name: String,
    },
    PlaylistDeletionFailed {
        name: String,
        error: String,
    },
    EpisodeAddedToPlaylist {
        playlist_name: String,
        episode_title: String,
    },
    EpisodeAddToPlaylistFailed {
        playlist_name: String,
        episode_title: String,
        error: String,
    },
    EpisodeRemovedFromPlaylist {
        playlist_name: String,
        episode_title: String,
    },
    EpisodeRemoveFromPlaylistFailed {
        playlist_name: String,
        episode_title: String,
        error: String,
    },
    PlaylistReordered {
        name: String,
    },
    PlaylistReorderFailed {
        name: String,
        error: String,
    },
    PlaylistRebuilt {
        name: String,
        rebuilt: usize,
        skipped: usize,
        failed: usize,
    },
    PlaylistRebuildFailed {
        name: String,
        error: String,
    },
    TodayPlaylistRefreshed {
        added: usize,
        removed: usize,
    },
    TodayPlaylistRefreshFailed {
        error: String,
    },

    // Device sync events
    /// Device sync started
    DeviceSyncStarted {
        device_path: std::path::PathBuf,
        dry_run: bool,
        hard_sync: bool,
    },

    /// Device sync completed successfully
    DeviceSyncCompleted {
        device_path: std::path::PathBuf,
        report: crate::download::SyncReport,
        dry_run: bool,
    },

    /// Device sync failed
    DeviceSyncFailed {
        device_path: std::path::PathBuf,
        error: String,
    },

    /// Device sync progress event
    DeviceSyncProgress {
        event: crate::download::SyncProgressEvent,
    },

    /// Download cleanup completed (age-based cleanup)
    DownloadCleanupCompleted {
        deleted_count: usize,
        duration_label: String,
    },

    /// Download cleanup failed
    DownloadCleanupFailed {
        error: String,
    },

    /// Podcast tag added successfully
    PodcastTagAdded {
        podcast_id: crate::storage::PodcastId,
        tag: String,
    },

    /// Podcast tag add failed
    PodcastTagAddFailed {
        podcast_id: crate::storage::PodcastId,
        error: String,
    },

    /// Podcast tag removed successfully
    PodcastTagRemoved {
        podcast_id: crate::storage::PodcastId,
        tag: String,
    },

    /// Podcast tag remove failed
    PodcastTagRemoveFailed {
        podcast_id: crate::storage::PodcastId,
        error: String,
    },

    /// Smart playlist evaluated — episodes computed from filter rules
    SmartPlaylistEvaluated {
        detail_buffer_id: String,
        episodes: Vec<crate::playlist::PlaylistEpisode>,
    },

    /// Smart playlist evaluation failed
    SmartPlaylistEvaluationFailed {
        playlist_name: String,
        error: String,
    },
}

/// Types of buffer refresh operations
#[derive(Debug, Clone)]
pub enum BufferRefreshType {
    /// Refresh podcast list buffer
    PodcastList,
    /// Refresh downloads buffer
    Downloads,
    /// Refresh What's New buffer
    WhatsNew,
    /// Refresh all episode buffers
    AllEpisodeBuffers,
    /// Refresh episode buffers for specific podcast
    EpisodeBuffers {
        podcast_id: crate::storage::PodcastId,
    },
}

/// Buffer refresh data payload
#[derive(Debug, Clone)]
pub enum BufferRefreshData {
    /// Podcast list data
    PodcastList {
        podcasts: Vec<crate::podcast::Podcast>,
    },
    /// Download entries data
    Downloads { downloads: Vec<DownloadEntry> },
    /// What's New episodes data
    WhatsNew { episodes: Vec<AggregatedEpisode> },
    /// Episode list data for specific podcast
    Episodes {
        podcast_id: crate::storage::PodcastId,
        episodes: Vec<crate::podcast::Episode>,
    },
    /// Error occurred during refresh
    Error { message: String },
}

/// Aggregated episode with podcast information (moved from whats_new buffer)
#[derive(Debug, Clone)]
pub struct AggregatedEpisode {
    pub podcast_id: crate::storage::PodcastId,
    pub podcast_title: String,
    pub episode: crate::podcast::Episode,
}

/// Download entry for tracking downloads (moved from downloads buffer)
#[derive(Debug, Clone)]
pub struct DownloadEntry {
    pub podcast_id: crate::storage::PodcastId,
    pub episode_id: crate::storage::EpisodeId,
    pub podcast_name: String,
    pub episode_title: String,
    pub status: crate::download::DownloadStatus,
    pub file_path: Option<std::path::PathBuf>,
    pub file_size: Option<u64>,
}
