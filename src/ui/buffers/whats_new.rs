// What's New buffer - displays latest episodes from all subscribed podcasts
//
// This buffer aggregates the most recent episodes across all podcasts,
// sorted in reverse chronological order. Users can download episodes directly
// from this view, and episodes are removed once downloaded.

use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::{
    download::DownloadManager,
    podcast::{subscription::SubscriptionManager, Episode, EpisodeStatus},
    storage::{JsonStorage, PodcastId, Storage},
    ui::{
        buffers::{Buffer, BufferId},
        filters::EpisodeFilter,
        themes::Theme,
        UIAction, UIComponent,
    },
    utils::time::format_relative_time,
};
use std::sync::Arc;

/// Aggregated episode with podcast information
#[derive(Debug, Clone)]
pub struct AggregatedEpisode {
    pub podcast_id: PodcastId,
    pub podcast_title: String,
    pub episode: Episode,
}

/// Buffer for displaying latest episodes across all podcasts
pub struct WhatsNewBuffer {
    id: String,
    episodes: Vec<AggregatedEpisode>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
    subscription_manager: Option<Arc<SubscriptionManager<JsonStorage>>>,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,
    max_episodes: usize,
    filter: EpisodeFilter,
    filtered_indices: Vec<usize>,
}

impl WhatsNewBuffer {
    /// Create a new What's New buffer
    pub fn new(max_episodes: usize) -> Self {
        Self {
            id: "whats-new".to_string(),
            episodes: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
            subscription_manager: None,
            download_manager: None,
            max_episodes,
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

    /// Set the theme for this buffer
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Load and aggregate latest episodes from all podcasts
    pub async fn load_episodes(&mut self) -> Result<(), String> {
        if let Some(ref manager) = self.subscription_manager {
            // Get all podcasts
            let podcasts = manager
                .list_subscriptions()
                .await
                .map_err(|e| e.to_string())?;

            let mut all_episodes = Vec::new();

            // Load episodes from each podcast
            for podcast in podcasts {
                // Load episodes for this podcast
                let episodes = match manager.storage.load_episodes(&podcast.id).await {
                    Ok(eps) => eps,
                    Err(_) => continue, // Skip if episodes fail to load
                };

                // Filter out downloaded episodes and aggregate
                for episode in episodes {
                    // Skip downloaded or currently downloading episodes
                    if !episode.is_downloaded()
                        && !matches!(episode.status, EpisodeStatus::Downloading)
                    {
                        all_episodes.push(AggregatedEpisode {
                            podcast_id: podcast.id.clone(),
                            podcast_title: podcast.title.clone(),
                            episode,
                        });
                    }
                }
            }

            // Sort by published date in descending order (newest first)
            all_episodes.sort_by(|a, b| b.episode.published.cmp(&a.episode.published));

            // Limit to max_episodes
            all_episodes.truncate(self.max_episodes);

            // Deduplicate by episode ID (in case same episode appears in multiple feeds)
            let mut seen_ids = std::collections::HashSet::new();
            all_episodes.retain(|agg_ep| seen_ids.insert(agg_ep.episode.id.clone()));

            self.episodes = all_episodes;

            // Apply filters (resets selection + scroll as appropriate)
            self.apply_filters();

            Ok(())
        } else {
            Err("No subscription manager available".to_string())
        }
    }

    /// Get selected episode (maps through filtered_indices)
    pub fn selected_episode(&self) -> Option<&AggregatedEpisode> {
        self.selected_index
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&actual| self.episodes.get(actual))
    }

    /// Apply current filter to episodes, rebuilding filtered_indices
    fn apply_filters(&mut self) {
        self.filtered_indices = self
            .episodes
            .iter()
            .enumerate()
            .filter(|(_, agg_ep)| self.filter.matches(&agg_ep.episode))
            .map(|(i, _)| i)
            .collect();

        // Reset selection when filter changes
        if self.filtered_indices.is_empty() {
            self.selected_index = None;
        } else {
            self.selected_index = Some(0);
        }
        self.scroll_offset = 0;
    }

    /// Number of currently visible (filtered) episodes
    fn visible_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Set episodes data directly (for background refresh)
    pub fn set_episodes(&mut self, episodes: Vec<crate::ui::events::AggregatedEpisode>) {
        // Convert from events::AggregatedEpisode to local AggregatedEpisode format
        self.episodes = episodes
            .into_iter()
            .map(|agg_ep| AggregatedEpisode {
                podcast_id: agg_ep.podcast_id,
                podcast_title: agg_ep.podcast_title,
                episode: agg_ep.episode,
            })
            .collect();

        // Reapply filters (resets selection and scroll)
        self.apply_filters();
    }

    /// Move selection up
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

    /// Move selection down
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
}

impl Buffer for WhatsNewBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "What's New".to_string()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_close(&self) -> bool {
        false // Core buffer, cannot be closed
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "What's New Commands:".to_string(),
            "  C-n, ↓    Next episode".to_string(),
            "  C-p, ↑    Previous episode".to_string(),
            "  Enter     View episode details".to_string(),
            "  D         Download episode".to_string(),
            "  p         Add selected episode to a playlist".to_string(),
            "  /         Search episodes".to_string(),
            "  F6        Clear filters".to_string(),
            "  F5        Refresh episode list".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for WhatsNewBuffer {
    fn has_focus(&self) -> bool {
        self.focused
    }

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
                if self.visible_count() > 0 {
                    self.selected_index = Some(self.visible_count() - 1);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::SelectItem => {
                if let Some(agg_episode) = self.selected_episode() {
                    // Open episode detail buffer
                    UIAction::OpenEpisodeDetail {
                        episode: Box::new(agg_episode.episode.clone()),
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            UIAction::DownloadEpisode => {
                if let Some(agg_episode) = self.selected_episode() {
                    let episode = &agg_episode.episode;

                    if episode.is_downloaded() {
                        UIAction::ShowMessage("Episode already downloaded".to_string())
                    } else if matches!(episode.status, EpisodeStatus::Downloading) {
                        UIAction::ShowMessage("Episode is already downloading".to_string())
                    } else if episode.audio_url.is_empty()
                        && !episode.guid.as_ref().is_some_and(|g| g.starts_with("http"))
                    {
                        UIAction::ShowMessage(
                            "Cannot download: No audio URL available for this episode".to_string(),
                        )
                    } else {
                        // Return action to trigger async download
                        UIAction::TriggerDownload {
                            podcast_id: agg_episode.podcast_id.clone(),
                            episode_id: episode.id.clone(),
                            episode_title: episode.title.clone(),
                        }
                    }
                } else {
                    UIAction::ShowMessage("No episode selected for download".to_string())
                }
            }
            UIAction::MarkPlayed => {
                let result = self
                    .selected_index
                    .and_then(|i| self.filtered_indices.get(i))
                    .copied()
                    .map(|actual_idx| {
                        let agg = &self.episodes[actual_idx];
                        (
                            actual_idx,
                            agg.podcast_id.clone(),
                            agg.episode.id.clone(),
                            agg.episode.title.clone(),
                            agg.episode.is_played(),
                        )
                    });
                match result {
                    Some((_, _, _, _, true)) => {
                        UIAction::ShowMessage("Episode already marked as played".to_string())
                    }
                    Some((actual_idx, podcast_id, episode_id, episode_title, false)) => {
                        self.episodes[actual_idx].episode.mark_played();
                        UIAction::TriggerMarkPlayed {
                            podcast_id,
                            episode_id,
                            episode_title,
                        }
                    }
                    None => UIAction::ShowMessage("No episode selected".to_string()),
                }
            }
            UIAction::MarkUnplayed => {
                let result = self
                    .selected_index
                    .and_then(|i| self.filtered_indices.get(i))
                    .copied()
                    .map(|actual_idx| {
                        let agg = &self.episodes[actual_idx];
                        (
                            actual_idx,
                            agg.podcast_id.clone(),
                            agg.episode.id.clone(),
                            agg.episode.title.clone(),
                            agg.episode.is_played(),
                        )
                    });
                match result {
                    Some((_, _, _, _, false)) => {
                        UIAction::ShowMessage("Episode already marked as unplayed".to_string())
                    }
                    Some((actual_idx, podcast_id, episode_id, episode_title, true)) => {
                        self.episodes[actual_idx].episode.mark_unplayed();
                        UIAction::TriggerMarkUnplayed {
                            podcast_id,
                            episode_id,
                            episode_title,
                        }
                    }
                    None => UIAction::ShowMessage("No episode selected".to_string()),
                }
            }
            UIAction::Search => UIAction::Search,
            UIAction::ApplySearch { query } => {
                self.filter.text_query = if query.is_empty() { None } else { Some(query) };
                self.apply_filters();
                UIAction::Render
            }
            UIAction::ClearFilters => {
                self.filter.clear();
                self.apply_filters();
                UIAction::Render
            }
            UIAction::SetStatusFilter { status } => {
                use crate::ui::filters::parse_status_filter;
                match parse_status_filter(&status) {
                    Some(sf) => {
                        self.filter.status = Some(sf);
                        self.apply_filters();
                        UIAction::Render
                    }
                    None => UIAction::ShowMessage(format!("Unknown status filter: {}", status)),
                }
            }
            UIAction::SetDateRangeFilter { range } => {
                use crate::ui::filters::parse_date_range;
                match parse_date_range(&range) {
                    Some(dr) => {
                        self.filter.date_range = Some(dr);
                        self.apply_filters();
                        UIAction::Render
                    }
                    None => UIAction::ShowError(format!(
                        "Unknown date range: '{}'. Use: today, 12h, 7d, 2w, 1m",
                        range
                    )),
                }
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let filtered_count = self.filtered_indices.len();
        let total_count = self.episodes.len();

        // Build title with filter indicator
        let title = if self.filter.is_active() {
            format!(
                " What's New ({} of {} episodes) [{}] ",
                filtered_count,
                total_count,
                self.filter.description()
            )
        } else {
            format!(" What's New ({} episodes) ", total_count)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if self.focused {
                self.theme.border_focused_style()
            } else {
                self.theme.border_style()
            });

        if self.episodes.is_empty() {
            // Show empty state
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let message = ratatui::widgets::Paragraph::new(
                "No new episodes available.\n\nEpisodes will appear here after refreshing podcasts.\nPress 'R' to refresh all podcasts.",
            )
            .style(self.theme.default_style())
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

            frame.render_widget(message, inner);
            return;
        }

        if filtered_count == 0 && self.filter.is_active() {
            // Filter active but nothing matches
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let message = ratatui::widgets::Paragraph::new(
                "No episodes match the current filter.\n\nPress F6 or run :clear-filters to reset.",
            )
            .style(self.theme.default_style())
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

            frame.render_widget(message, inner);
            return;
        }

        // Calculate visible area
        let inner = block.inner(area);
        let visible_height = inner.height.saturating_sub(1) as usize; // -1 for header

        // Adjust scroll offset to keep selection visible
        if let Some(selected) = self.selected_index {
            if selected >= self.scroll_offset + visible_height {
                self.scroll_offset = selected.saturating_sub(visible_height - 1);
            } else if selected < self.scroll_offset {
                self.scroll_offset = selected;
            }
        }

        // Create table headers
        let header = Row::new(vec![
            Cell::from("Podcast"),
            Cell::from("Episode"),
            Cell::from("Published"),
        ])
        .style(
            Style::default()
                .fg(self.theme.colors.primary)
                .add_modifier(Modifier::BOLD),
        );

        // Create table rows from filtered_indices
        let end_index = (self.scroll_offset + visible_height).min(filtered_count);
        let rows: Vec<Row> = self.filtered_indices[self.scroll_offset..end_index]
            .iter()
            .enumerate()
            .map(|(display_index, &actual_index)| {
                let display_pos = self.scroll_offset + display_index;
                let agg_episode = &self.episodes[actual_index];
                let episode = &agg_episode.episode;
                let style = if Some(display_pos) == self.selected_index {
                    Style::default()
                        .bg(self.theme.colors.selection)
                        .fg(self.theme.colors.text)
                } else {
                    self.theme.default_style()
                };

                // Format published date as relative time
                let published_str = format_relative_time(&episode.published);

                Row::new(vec![
                    Cell::from(truncate_string(&agg_episode.podcast_title, 25)),
                    Cell::from(truncate_string(&episode.title, 65)),
                    Cell::from(published_str),
                ])
                .style(style)
            })
            .collect();

        // Create table with dynamic column widths
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(25), // Podcast
                Constraint::Percentage(60), // Episode (more space!)
                Constraint::Percentage(15), // Published
            ],
        )
        .header(header)
        .block(block)
        .column_spacing(1);

        frame.render_widget(table, area);
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn title(&self) -> String {
        if self.filter.is_active() {
            format!(
                "What's New ({} of {} episodes)",
                self.filtered_indices.len(),
                self.episodes.len()
            )
        } else {
            format!("What's New ({} episodes)", self.episodes.len())
        }
    }
}

/// Truncate a string to a maximum length with ellipsis
/// Uses character-aware truncation to handle multi-byte UTF-8 characters
fn truncate_string(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whats_new_buffer_creation() {
        let buffer = WhatsNewBuffer::new(100);
        assert_eq!(buffer.id(), "whats-new");
        assert_eq!(buffer.name(), "What's New");
        assert!(!buffer.can_close());
        assert_eq!(buffer.max_episodes, 100);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(
            truncate_string("this is a very long string", 10),
            "this is..."
        );
        assert_eq!(truncate_string("exactly10!", 10), "exactly10!");
    }

    #[test]
    fn test_select_item_opens_episode_detail() {
        use crate::podcast::Episode;
        use crate::storage::PodcastId;

        let mut buffer = WhatsNewBuffer::new(100);

        // Add a mock episode
        let podcast_id = PodcastId::new();
        let episode = Episode::new(
            podcast_id.clone(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            chrono::Utc::now(),
        );

        buffer.episodes = vec![AggregatedEpisode {
            podcast_id: podcast_id.clone(),
            podcast_title: "Test Podcast".to_string(),
            episode: episode.clone(),
        }];
        buffer.filtered_indices = vec![0];
        buffer.selected_index = Some(0);

        // Test SelectItem action
        let action = buffer.handle_action(UIAction::SelectItem);

        // Should return OpenEpisodeDetail action
        match action {
            UIAction::OpenEpisodeDetail {
                episode: returned_episode,
            } => {
                assert_eq!(returned_episode.title, "Test Episode");
            }
            _ => panic!("Expected OpenEpisodeDetail action, got {:?}", action),
        }
    }

    #[test]
    fn test_select_item_with_no_selection() {
        let mut buffer = WhatsNewBuffer::new(100);

        // No episodes, no selection
        let action = buffer.handle_action(UIAction::SelectItem);

        // Should return ShowMessage action
        match action {
            UIAction::ShowMessage(msg) => {
                assert_eq!(msg, "No episode selected");
            }
            _ => panic!("Expected ShowMessage action, got {:?}", action),
        }
    }

    #[test]
    fn test_set_episodes_updates_buffer() {
        use crate::podcast::Episode;
        use crate::storage::PodcastId;
        use crate::ui::events::AggregatedEpisode as EventsAggregatedEpisode;

        let mut buffer = WhatsNewBuffer::new(100);

        // Initially empty
        assert_eq!(buffer.episodes.len(), 0);

        // Create some episodes
        let podcast_id = PodcastId::new();
        let episode1 = Episode::new(
            podcast_id.clone(),
            "Episode 1".to_string(),
            "https://example.com/audio1.mp3".to_string(),
            chrono::Utc::now(),
        );
        let episode2 = Episode::new(
            podcast_id.clone(),
            "Episode 2".to_string(),
            "https://example.com/audio2.mp3".to_string(),
            chrono::Utc::now(),
        );

        let agg_episodes = vec![
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Test Podcast".to_string(),
                episode: episode1,
            },
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Test Podcast".to_string(),
                episode: episode2,
            },
        ];

        // Set episodes
        buffer.set_episodes(agg_episodes);

        // Verify episodes were set
        assert_eq!(buffer.episodes.len(), 2);
        assert_eq!(buffer.episodes[0].episode.title, "Episode 1");
        assert_eq!(buffer.episodes[1].episode.title, "Episode 2");

        // Verify selection was set
        assert_eq!(buffer.selected_index, Some(0));

        // Verify scroll offset was reset
        assert_eq!(buffer.scroll_offset, 0);
    }

    #[test]
    fn test_set_episodes_resets_scroll_and_maintains_valid_selection() {
        use crate::podcast::Episode;
        use crate::storage::PodcastId;
        use crate::ui::events::AggregatedEpisode as EventsAggregatedEpisode;

        let mut buffer = WhatsNewBuffer::new(100);

        // Create initial episodes
        let podcast_id = PodcastId::new();
        let episodes: Vec<EventsAggregatedEpisode> = (0..10)
            .map(|i| EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Test Podcast".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    format!("Episode {}", i),
                    format!("https://example.com/audio{}.mp3", i),
                    chrono::Utc::now(),
                ),
            })
            .collect();

        buffer.set_episodes(episodes);

        // Set selection and scroll
        buffer.selected_index = Some(5);
        buffer.scroll_offset = 3;

        // Update with new episodes (fewer than before)
        let new_episodes: Vec<EventsAggregatedEpisode> = (0..3)
            .map(|i| EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Test Podcast".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    format!("New Episode {}", i),
                    format!("https://example.com/new{}.mp3", i),
                    chrono::Utc::now(),
                ),
            })
            .collect();

        buffer.set_episodes(new_episodes);

        // Verify scroll was reset
        assert_eq!(buffer.scroll_offset, 0);

        // Verify selection resets to first item (apply_filters resets cursor)
        assert_eq!(buffer.selected_index, Some(0));

        // Verify new episodes were set
        assert_eq!(buffer.episodes.len(), 3);
        assert!(buffer.episodes[0].episode.title.starts_with("New Episode"));
    }

    #[test]
    fn test_text_search_filters_episodes() {
        use crate::podcast::Episode;
        use crate::storage::PodcastId;
        use crate::ui::events::AggregatedEpisode as EventsAggregatedEpisode;

        let mut buffer = WhatsNewBuffer::new(100);
        let podcast_id = PodcastId::new();

        let agg_episodes = vec![
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Rust Pod".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    "Rust Programming".to_string(),
                    "url1".to_string(),
                    chrono::Utc::now(),
                ),
            },
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Python Pod".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    "Python Tips".to_string(),
                    "url2".to_string(),
                    chrono::Utc::now(),
                ),
            },
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Rust Pod".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    "Rust async".to_string(),
                    "url3".to_string(),
                    chrono::Utc::now(),
                ),
            },
        ];
        buffer.set_episodes(agg_episodes);
        assert_eq!(buffer.visible_count(), 3);

        buffer.handle_action(UIAction::ApplySearch {
            query: "rust".to_string(),
        });
        assert_eq!(buffer.visible_count(), 2);
        assert!(buffer.filter.is_active());

        let ep = buffer.selected_episode().expect("should have episode");
        assert_eq!(ep.episode.title, "Rust Programming");

        buffer.handle_action(UIAction::MoveDown);
        let ep = buffer.selected_episode().expect("should have episode");
        assert_eq!(ep.episode.title, "Rust async");
    }

    #[test]
    fn test_clear_filters_restores_all() {
        use crate::podcast::Episode;
        use crate::storage::PodcastId;
        use crate::ui::events::AggregatedEpisode as EventsAggregatedEpisode;

        let mut buffer = WhatsNewBuffer::new(100);
        let podcast_id = PodcastId::new();

        let agg_episodes = vec![
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Pod".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    "Ep 1".to_string(),
                    "url1".to_string(),
                    chrono::Utc::now(),
                ),
            },
            EventsAggregatedEpisode {
                podcast_id: podcast_id.clone(),
                podcast_title: "Pod".to_string(),
                episode: Episode::new(
                    podcast_id.clone(),
                    "Ep 2".to_string(),
                    "url2".to_string(),
                    chrono::Utc::now(),
                ),
            },
        ];
        buffer.set_episodes(agg_episodes);

        buffer.handle_action(UIAction::ApplySearch {
            query: "Ep 1".to_string(),
        });
        assert_eq!(buffer.visible_count(), 1);

        buffer.handle_action(UIAction::ClearFilters);
        assert_eq!(buffer.visible_count(), 2);
        assert!(!buffer.filter.is_active());
    }

    #[test]
    fn test_filter_no_match_empty_results() {
        use crate::podcast::Episode;
        use crate::storage::PodcastId;
        use crate::ui::events::AggregatedEpisode as EventsAggregatedEpisode;

        let mut buffer = WhatsNewBuffer::new(100);
        let podcast_id = PodcastId::new();

        buffer.set_episodes(vec![EventsAggregatedEpisode {
            podcast_id: podcast_id.clone(),
            podcast_title: "Pod".to_string(),
            episode: Episode::new(
                podcast_id.clone(),
                "Episode".to_string(),
                "url".to_string(),
                chrono::Utc::now(),
            ),
        }]);

        buffer.handle_action(UIAction::ApplySearch {
            query: "zzzzz".to_string(),
        });
        assert_eq!(buffer.visible_count(), 0);
        assert_eq!(buffer.selected_index, None);
        assert!(buffer.selected_episode().is_none());
    }
}
