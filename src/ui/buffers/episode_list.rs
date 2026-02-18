// Episode list buffer - displays episodes for a selected podcast
//
// This buffer shows episodes from a podcast and allows playback,
// download, and queue management operations.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{
    download::DownloadManager,
    podcast::{subscription::SubscriptionManager, Episode},
    storage::{JsonStorage, PodcastId, Storage},
    ui::{
        buffers::{Buffer, BufferId},
        filters::EpisodeFilter,
        themes::Theme,
        UIAction, UIComponent,
    },
};
use std::sync::Arc;

/// Buffer for displaying episodes from a podcast
pub struct EpisodeListBuffer {
    id: String,
    podcast_name: String,
    pub podcast_id: PodcastId,
    episodes: Vec<Episode>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
    subscription_manager: Option<Arc<SubscriptionManager<JsonStorage>>>,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,
    /// Active filter criteria for this buffer
    filter: EpisodeFilter,
    /// Indices into `episodes` that match the current filter.
    /// When no filter is active, contains all indices 0..episodes.len().
    filtered_indices: Vec<usize>,
}

impl EpisodeListBuffer {
    /// Create a new episode list buffer for a podcast
    pub fn new(podcast_name: String, podcast_id: PodcastId) -> Self {
        Self {
            id: format!("episodes-{}", podcast_name.replace(' ', "-").to_lowercase()),
            podcast_name,
            podcast_id,
            episodes: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
            subscription_manager: None,
            download_manager: None,
            filter: EpisodeFilter::default(),
            filtered_indices: Vec::new(),
        }
    }

    /// Set managers
    pub fn set_managers(
        &mut self,
        subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
        download_manager: Arc<DownloadManager<JsonStorage>>,
    ) {
        self.subscription_manager = Some(subscription_manager);
        self.download_manager = Some(download_manager);
    }

    /// Set configurable duration filter thresholds from user config.
    pub fn set_filter_thresholds(&mut self, short_max_minutes: u32, long_min_minutes: u32) {
        self.filter.set_duration_thresholds(short_max_minutes, long_min_minutes);
    }

    /// Load episodes for the podcast
    pub async fn load_episodes(&mut self) -> Result<(), String> {
        if let Some(ref manager) = self.subscription_manager {
            match manager.get_podcast(&self.podcast_id).await {
                Ok(_podcast) => {
                    // Load episodes from storage
                    if let Some(ref sm) = self.subscription_manager {
                        match sm.storage.load_episodes(&self.podcast_id).await {
                            Ok(mut episodes) => {
                                // Sort episodes by published date in descending order (newest first)
                                episodes.sort_by(|a, b| b.published.cmp(&a.published));

                                self.episodes = episodes;
                                if !self.episodes.is_empty() && self.selected_index.is_none() {
                                    self.selected_index = Some(0);
                                }
                                self.scroll_offset = 0;
                                Ok(())
                            }
                            Err(e) => Err(e.to_string()),
                        }
                    } else {
                        Err("No subscription manager".to_string())
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("No subscription manager available".to_string())
        }
    }

    /// Set episodes for this buffer.
    ///
    /// If a filter is active, it is re-applied so `filtered_indices` stays consistent.
    pub fn set_episodes(&mut self, episodes: Vec<Episode>) {
        // Sort episodes by published date in descending order (newest first)
        let mut sorted_episodes = episodes;
        sorted_episodes.sort_by(|a, b| b.published.cmp(&a.published));

        self.episodes = sorted_episodes;

        // Re-apply filters (this also resets cursor/scroll appropriately)
        self.apply_filters();
    }

    /// Recompute `filtered_indices` based on the current filter state.
    ///
    /// Resets cursor to the first matching item and scroll to 0.
    fn apply_filters(&mut self) {
        self.filtered_indices = self
            .episodes
            .iter()
            .enumerate()
            .filter(|(_, ep)| self.filter.matches(ep))
            .map(|(i, _)| i)
            .collect();

        // Reset cursor to first filtered item
        self.selected_index = if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        };
        self.scroll_offset = 0;
    }

    /// Get the total number of episodes visible after filtering.
    fn visible_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get selected episode, mapping through `filtered_indices`.
    pub fn selected_episode(&self) -> Option<&Episode> {
        self.selected_index
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&actual| self.episodes.get(actual))
    }

    /// Download selected episode
    pub async fn download_selected(&self) -> Result<(), String> {
        if let (Some(episode), Some(ref dm)) = (self.selected_episode(), &self.download_manager) {
            dm.download_episode(&self.podcast_id, &episode.id)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("No episode selected or download manager unavailable".to_string())
        }
    }

    /// Delete selected episode download
    pub async fn delete_selected(&self) -> Result<(), String> {
        if let (Some(episode), Some(ref dm)) = (self.selected_episode(), &self.download_manager) {
            dm.delete_episode(&self.podcast_id, &episode.id)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("No episode selected or download manager unavailable".to_string())
        }
    }

    /// Move selection up (within filtered list)
    fn select_previous(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(0) => Some(count - 1),
            Some(i) => Some(i - 1),
            None => Some(0),
        };

        // Update scroll offset to keep selection visible
        if let Some(selected) = self.selected_index {
            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            }
        }
    }

    /// Move selection down (within filtered list)
    fn select_next(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(i) if i >= count - 1 => Some(0),
            Some(i) => Some(i + 1),
            None => Some(0),
        };

        // Update scroll offset to keep selection visible
        if let Some(selected) = self.selected_index {
            // When moving to beginning of list, reset scroll
            if selected == 0 {
                self.scroll_offset = 0;
            }
        }
    }

    /// Set the theme for this buffer
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

impl Buffer for EpisodeListBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        format!("Episodes: {}", self.podcast_name)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Episode List Commands:".to_string(),
            "  C-n, ↓    Next episode".to_string(),
            "  C-p, ↑    Previous episode".to_string(),
            "  Enter     View episode details".to_string(),
            "  D         Download episode".to_string(),
            "  X         Delete downloaded file".to_string(),
            "  m         Mark as played".to_string(),
            "  u         Mark as unplayed".to_string(),
            "  /         Search episodes".to_string(),
            "  F6        Clear all filters".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for EpisodeListBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.select_previous();
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.select_next();
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(_index) = self.selected_index {
                    if !self.episodes.is_empty() {
                        // Open episode detail buffer
                        if let Some(episode) = self.selected_episode() {
                            UIAction::OpenEpisodeDetail {
                                episode: episode.clone(),
                            }
                        } else {
                            UIAction::None
                        }
                    } else {
                        UIAction::None
                    }
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToTop => {
                if self.visible_count() > 0 {
                    self.selected_index = Some(0);
                    self.scroll_offset = 0;
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToBottom => {
                let count = self.visible_count();
                if count > 0 {
                    self.selected_index = Some(count - 1);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::DownloadEpisode => {
                if let Some(episode) = self.selected_episode() {
                    if episode.is_downloaded() {
                        UIAction::ShowMessage("Episode already downloaded".to_string())
                    } else if matches!(episode.status, crate::podcast::EpisodeStatus::Downloading) {
                        UIAction::ShowMessage("Episode is already downloading".to_string())
                    } else if episode.audio_url.is_empty()
                        && !episode
                            .guid
                            .as_ref()
                            .map_or(false, |g| g.starts_with("http"))
                    {
                        UIAction::ShowMessage(
                            "Cannot download: No audio URL available for this episode".to_string(),
                        )
                    } else {
                        // Return action to trigger async download
                        UIAction::TriggerDownload {
                            podcast_id: self.podcast_id.clone(),
                            episode_id: episode.id.clone(),
                            episode_title: episode.title.clone(),
                        }
                    }
                } else {
                    UIAction::ShowMessage("No episode selected for download".to_string())
                }
            }
            UIAction::DeleteDownloadedEpisode => {
                if let Some(episode) = self.selected_episode() {
                    if episode.is_downloaded() {
                        UIAction::TriggerDeleteDownload {
                            podcast_id: self.podcast_id.clone(),
                            episode_id: episode.id.clone(),
                            episode_title: episode.title.clone(),
                        }
                    } else {
                        UIAction::ShowMessage("Episode is not downloaded".to_string())
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            // --- Search & Filter actions ---
            UIAction::Search => {
                // Bubble up to UIApp which will open the minibuffer prompt
                UIAction::Search
            }
            UIAction::ApplySearch { query } => {
                self.filter.text_query = if query.is_empty() {
                    None
                } else {
                    Some(query)
                };
                self.apply_filters();
                UIAction::Render
            }
            UIAction::ClearFilters => {
                if self.filter.is_active() {
                    self.filter.clear();
                    self.apply_filters();
                    UIAction::ShowMessage("Filters cleared".to_string())
                } else {
                    UIAction::ShowMessage("No active filters".to_string())
                }
            }
            UIAction::SetStatusFilter { status } => {
                use crate::ui::filters::parse_status_filter;
                if let Some(sf) = parse_status_filter(&status) {
                    self.filter.status = Some(sf);
                    self.apply_filters();
                    UIAction::Render
                } else {
                    UIAction::ShowError(format!("Unknown status: '{}'. Use: new, downloaded, played, downloading, failed", status))
                }
            }
            UIAction::SetDateRangeFilter { range } => {
                use crate::ui::filters::parse_date_range;
                if let Some(dr) = parse_date_range(&range) {
                    self.filter.date_range = Some(dr);
                    self.apply_filters();
                    UIAction::Render
                } else {
                    UIAction::ShowError(format!("Unknown date range: '{}'. Use: today, 12h, 7d, 2w, 1m", range))
                }
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let filtered_count = self.filtered_indices.len();
        let total_count = self.episodes.len();

        // Calculate visible area and viewport
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

        // Ensure selected item is visible in viewport
        if let Some(selected) = self.selected_index {
            let viewport_end = self.scroll_offset + visible_height;

            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            } else if selected >= viewport_end {
                self.scroll_offset = selected.saturating_sub(visible_height - 1);
            }
        }

        // Build list items from filtered_indices
        let end_index = (self.scroll_offset + visible_height).min(filtered_count);
        let items: Vec<ListItem> = if filtered_count == 0 {
            Vec::new()
        } else {
            self.filtered_indices[self.scroll_offset..end_index]
                .iter()
                .enumerate()
                .map(|(display_index, &actual_ep_index)| {
                    let display_pos = self.scroll_offset + display_index;
                    let episode = &self.episodes[actual_ep_index];

                    let status_indicator = match episode.status {
                        crate::podcast::EpisodeStatus::New => {
                            if episode.audio_url.is_empty()
                                && !episode
                                    .guid
                                    .as_ref()
                                    .map_or(false, |g| g.starts_with("http"))
                            {
                                "⚠"
                            } else {
                                "○"
                            }
                        }
                        crate::podcast::EpisodeStatus::Downloaded => "●",
                        crate::podcast::EpisodeStatus::Downloading => "◐",
                        crate::podcast::EpisodeStatus::Played => "✓",
                        crate::podcast::EpisodeStatus::DownloadFailed => "✗",
                    };

                    let title_with_info = if episode.audio_url.is_empty()
                        && !episode
                            .guid
                            .as_ref()
                            .map_or(false, |g| g.starts_with("http"))
                        && episode.status == crate::podcast::EpisodeStatus::New
                    {
                        format!("{} (no audio URL)", episode.title)
                    } else {
                        episode.title.clone()
                    };

                    let content = format!(" {} {}", status_indicator, title_with_info);

                    if Some(display_pos) == self.selected_index {
                        ListItem::new(content).style(self.theme.selected_style())
                    } else {
                        ListItem::new(content).style(self.theme.text_style())
                    }
                })
                .collect()
        };

        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        // Build title with filter indicator
        let title = if self.filter.is_active() {
            format!(
                "Episodes: {} [{}]",
                self.podcast_name,
                self.filter.description()
            )
        } else {
            format!("Episodes: {}", self.podcast_name)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style()),
            )
            .style(self.theme.text_style());

        frame.render_widget(list, area);

        // Show status / empty state
        if self.episodes.is_empty() {
            let empty_msg = "No episodes available.";
            let status_area = Rect {
                x: area.x + 2,
                y: area.y + area.height / 2,
                width: area.width.saturating_sub(4),
                height: 1,
            };
            let status =
                ratatui::widgets::Paragraph::new(empty_msg).style(self.theme.muted_style());
            frame.render_widget(status, status_area);
        } else if filtered_count == 0 && self.filter.is_active() {
            // Filter active but nothing matches
            let empty_msg = "No episodes match the current filter. Press F6 or :clear-filters to reset.";
            let status_area = Rect {
                x: area.x + 2,
                y: area.y + area.height / 2,
                width: area.width.saturating_sub(4),
                height: 1,
            };
            let status =
                ratatui::widgets::Paragraph::new(empty_msg).style(self.theme.muted_style());
            frame.render_widget(status, status_area);
        } else if let Some(index) = self.selected_index {
            let status_msg = if self.filter.is_active() {
                format!(
                    " {} of {} matching ({} total) ",
                    index + 1,
                    filtered_count,
                    total_count
                )
            } else {
                format!(" {} of {} episodes ", index + 1, total_count)
            };
            let status_area = Rect {
                x: area.x + area.width.saturating_sub(status_msg.len() as u16 + 2),
                y: area.y + area.height - 1,
                width: status_msg.len() as u16,
                height: 1,
            };
            let status =
                ratatui::widgets::Paragraph::new(status_msg).style(self.theme.muted_style());
            frame.render_widget(status, status_area);
        }
    }

    fn title(&self) -> String {
        self.name()
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::PodcastId;

    #[test]
    fn test_episode_list_buffer_creation() {
        let podcast_name = "Test Podcast".to_string();
        let podcast_id = PodcastId::new();
        let buffer = EpisodeListBuffer::new(podcast_name.clone(), podcast_id.clone());

        assert_eq!(buffer.id(), "episodes-test-podcast");
        assert_eq!(buffer.name(), "Episodes: Test Podcast");
        assert!(buffer.can_close());
        assert_eq!(buffer.selected_index, None);
        assert_eq!(buffer.podcast_name, podcast_name);
        assert_eq!(buffer.podcast_id, podcast_id);
    }

    #[test]
    fn test_navigation() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        // Add some mock episodes using set_episodes
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Ep1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Ep2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
        ];
        buffer.set_episodes(episodes);

        // Test moving down
        let action = buffer.handle_action(UIAction::MoveDown);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.selected_index, Some(1));

        // Test moving up
        let action = buffer.handle_action(UIAction::MoveUp);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.selected_index, Some(0));
    }

    #[test]
    fn test_selection_wrapping() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        // Add some mock episodes using set_episodes
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Ep1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Ep2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
        ];
        buffer.set_episodes(episodes);

        // Move to top
        buffer.handle_action(UIAction::MoveToTop);
        assert_eq!(buffer.selected_index, Some(0));

        // Move up from top (should wrap to bottom)
        buffer.handle_action(UIAction::MoveUp);
        assert_eq!(buffer.selected_index, Some(buffer.visible_count() - 1));

        // Move down from bottom (should wrap to top)
        buffer.handle_action(UIAction::MoveDown);
        assert_eq!(buffer.selected_index, Some(0));
    }

    #[test]
    fn test_episode_selection() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        // Add some mock episodes using set_episodes
        let episodes = vec![Episode::new(
            PodcastId::new(),
            "Ep1".to_string(),
            "url1".to_string(),
            chrono::Utc::now(),
        )];
        buffer.set_episodes(episodes);

        // Select an episode - should now open episode detail
        let action = buffer.handle_action(UIAction::SelectItem);
        match action {
            UIAction::OpenEpisodeDetail { episode } => {
                assert_eq!(episode.title, "Ep1");
            }
            _ => panic!("Expected OpenEpisodeDetail action"),
        }
    }

    #[test]
    fn test_cursor_position_reset_after_set_episodes() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());

        // Create initial episodes
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Episode 1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Episode 2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Episode 3".to_string(),
                "url3".to_string(),
                chrono::Utc::now(),
            ),
        ];

        // Set initial episodes
        buffer.set_episodes(episodes.clone());

        // Move cursor to third episode (index 2)
        buffer.selected_index = Some(2);
        buffer.scroll_offset = 1;

        // Simulate updating episodes (filter reapply resets cursor)
        buffer.set_episodes(episodes.clone());

        // Cursor resets to 0 when filter is reapplied (no filter active)
        assert_eq!(
            buffer.selected_index,
            Some(0),
            "Cursor resets to first item after set_episodes"
        );

        // Scroll offset resets
        assert_eq!(buffer.scroll_offset, 0, "Scroll offset resets after set_episodes");
    }

    #[test]
    fn test_cursor_position_adjusted_when_episodes_decrease() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());

        // Create initial episodes
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Episode 1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Episode 2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Episode 3".to_string(),
                "url3".to_string(),
                chrono::Utc::now(),
            ),
        ];

        // Set initial episodes and move cursor to last episode
        buffer.set_episodes(episodes);
        buffer.selected_index = Some(2);

        // Update with fewer episodes
        let fewer_episodes = vec![Episode::new(
            PodcastId::new(),
            "Episode 1".to_string(),
            "url1".to_string(),
            chrono::Utc::now(),
        )];

        buffer.set_episodes(fewer_episodes);

        // Cursor should be adjusted to last valid index
        assert_eq!(
            buffer.selected_index,
            Some(0),
            "Cursor should be adjusted to last valid index"
        );
    }

    #[test]
    fn test_scroll_offset_reset_when_out_of_bounds() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());

        // Create initial episodes
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Episode 1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Episode 2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Episode 3".to_string(),
                "url3".to_string(),
                chrono::Utc::now(),
            ),
        ];

        buffer.set_episodes(episodes);
        buffer.scroll_offset = 2;

        // Update with single episode
        let single_episode = vec![Episode::new(
            PodcastId::new(),
            "Episode 1".to_string(),
            "url1".to_string(),
            chrono::Utc::now(),
        )];

        buffer.set_episodes(single_episode);

        // Scroll offset should be reset to 0
        assert_eq!(
            buffer.scroll_offset, 0,
            "Scroll offset should be reset when out of bounds"
        );
    }

    #[test]
    fn test_text_search_filters_episodes() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        let now = chrono::Utc::now();
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Rust Programming".to_string(),
                "url1".to_string(),
                now - chrono::Duration::hours(2),
            ),
            Episode::new(
                PodcastId::new(),
                "Python Tips".to_string(),
                "url2".to_string(),
                now - chrono::Duration::hours(1),
            ),
            Episode::new(
                PodcastId::new(),
                "Rust async".to_string(),
                "url3".to_string(),
                now,
            ),
        ];
        buffer.set_episodes(episodes);
        // After sorting by date desc: Rust async, Python Tips, Rust Programming
        assert_eq!(buffer.visible_count(), 3);

        // Apply search filter
        buffer.handle_action(UIAction::ApplySearch {
            query: "rust".to_string(),
        });
        assert_eq!(buffer.visible_count(), 2);
        assert!(buffer.filter.is_active());

        // Navigation should only move through filtered items
        assert_eq!(buffer.selected_index, Some(0));
        let ep = buffer.selected_episode().expect("should have episode");
        assert_eq!(ep.title, "Rust async"); // newest first

        buffer.handle_action(UIAction::MoveDown);
        let ep = buffer.selected_episode().expect("should have episode");
        assert_eq!(ep.title, "Rust Programming"); // oldest last
    }

    #[test]
    fn test_clear_filters_restores_all() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Ep 1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Ep 2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
        ];
        buffer.set_episodes(episodes);

        // Apply filter
        buffer.handle_action(UIAction::ApplySearch {
            query: "Ep 1".to_string(),
        });
        assert_eq!(buffer.visible_count(), 1);

        // Clear filters
        buffer.handle_action(UIAction::ClearFilters);
        assert_eq!(buffer.visible_count(), 2);
        assert!(!buffer.filter.is_active());
    }

    #[test]
    fn test_status_filter() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        let ep1 = Episode::new(
            PodcastId::new(),
            "New Episode".to_string(),
            "url1".to_string(),
            chrono::Utc::now(),
        );
        let mut ep2 = Episode::new(
            PodcastId::new(),
            "Downloaded Episode".to_string(),
            "url2".to_string(),
            chrono::Utc::now(),
        );
        ep2.status = crate::podcast::EpisodeStatus::Downloaded;
        let episodes = vec![ep1, ep2];
        buffer.set_episodes(episodes);
        assert_eq!(buffer.visible_count(), 2);

        // Filter by status
        buffer.handle_action(UIAction::SetStatusFilter {
            status: "downloaded".to_string(),
        });
        assert_eq!(buffer.visible_count(), 1);
        let ep = buffer.selected_episode().expect("should have episode");
        assert_eq!(ep.title, "Downloaded Episode");
    }

    #[test]
    fn test_empty_search_shows_all() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        let episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Ep 1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Ep 2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
        ];
        buffer.set_episodes(episodes);

        // Apply empty search
        buffer.handle_action(UIAction::ApplySearch {
            query: "".to_string(),
        });
        assert_eq!(buffer.visible_count(), 2);
        assert!(!buffer.filter.is_active());
    }

    #[test]
    fn test_filter_no_match_empty_results() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        let episodes = vec![Episode::new(
            PodcastId::new(),
            "Ep 1".to_string(),
            "url1".to_string(),
            chrono::Utc::now(),
        )];
        buffer.set_episodes(episodes);

        // Apply search that matches nothing
        buffer.handle_action(UIAction::ApplySearch {
            query: "zzzzz".to_string(),
        });
        assert_eq!(buffer.visible_count(), 0);
        assert_eq!(buffer.selected_index, None);
        assert!(buffer.selected_episode().is_none());
    }

    #[test]
    fn test_search_bubbles_up() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        let action = buffer.handle_action(UIAction::Search);
        // Search should bubble up to the app for minibuffer handling
        assert_eq!(action, UIAction::Search);
    }
}
