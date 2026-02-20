// Podcast list buffer - displays subscribed podcasts
//
// This buffer shows the list of subscribed podcasts and allows
// management operations like adding, removing, and refreshing feeds.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    podcast::{subscription::SubscriptionManager, Podcast},
    storage::JsonStorage,
    ui::{
        buffers::{Buffer, BufferId},
        filters::PodcastFilter,
        themes::Theme,
        UIAction, UIComponent,
    },
};
use std::sync::Arc;

/// State of the podcast list buffer
#[derive(Debug, Clone)]
pub enum PodcastListState {
    Loading,
    Ready,
    Error(String),
}
/// Buffer for displaying the podcast list
pub struct PodcastListBuffer {
    id: String,
    podcasts: Vec<Podcast>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
    state: PodcastListState,
    status_message: Option<String>,
    subscription_manager: Option<Arc<SubscriptionManager<JsonStorage>>>,
    filter: PodcastFilter,
    filtered_indices: Vec<usize>,
}

impl PodcastListBuffer {
    /// Create a new podcast list buffer
    pub fn new() -> Self {
        Self {
            id: "podcast-list".to_string(),
            podcasts: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
            state: PodcastListState::Ready,
            status_message: None,
            subscription_manager: None,
            filter: PodcastFilter::default(),
            filtered_indices: Vec::new(),
        }
    }

    /// Set the subscription manager
    pub fn set_subscription_manager(&mut self, manager: Arc<SubscriptionManager<JsonStorage>>) {
        self.subscription_manager = Some(manager);
    }

    /// Load podcasts from storage (for MVP - simplified)
    pub async fn load_podcasts(&mut self) -> Result<(), String> {
        if let Some(ref manager) = self.subscription_manager {
            self.state = PodcastListState::Loading;
            match manager.list_subscriptions().await {
                Ok(podcasts) => {
                    self.set_podcasts(podcasts);
                    self.state = PodcastListState::Ready;
                    Ok(())
                }
                Err(e) => {
                    self.state = PodcastListState::Error(e.to_string());
                    Err(e.to_string())
                }
            }
        } else {
            // For MVP, just show empty list if no manager
            self.set_podcasts(Vec::new());
            Ok(())
        }
    }

    /// Set podcasts to display (for testing/MVP)
    pub fn set_podcasts(&mut self, podcasts: Vec<Podcast>) {
        self.podcasts = podcasts;
        self.state = PodcastListState::Ready;
        self.apply_filters();
    }

    /// Get the currently selected podcast (maps through filtered_indices)
    pub fn selected_podcast(&self) -> Option<&Podcast> {
        self.selected_index
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&actual| self.podcasts.get(actual))
    }

    /// Apply current filter to podcasts, rebuilding filtered_indices
    fn apply_filters(&mut self) {
        self.filtered_indices = self
            .podcasts
            .iter()
            .enumerate()
            .filter(|(_, podcast)| self.filter.matches(podcast))
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

    /// Number of currently visible (filtered) podcasts
    fn visible_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get the number of podcasts
    pub fn podcast_count(&self) -> usize {
        self.podcasts.len()
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
    }

    /// Adjust scroll offset to ensure selected item is visible
    fn adjust_scroll(&mut self, visible_height: usize) {
        if let Some(selected) = self.selected_index {
            // Ensure we have at least one line visible
            if visible_height == 0 {
                return;
            }

            // If selected item is above the visible area, scroll up
            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            }
            // If selected item is below the visible area, scroll down
            else if selected >= self.scroll_offset + visible_height {
                self.scroll_offset = selected.saturating_sub(visible_height - 1);
            }
        }
    }

    /// Set the theme for this buffer
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

impl Buffer for PodcastListBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Podcasts".to_string()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_close(&self) -> bool {
        false // Main podcast list shouldn't be closeable
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Podcast List Commands:".to_string(),
            "  C-n, ↓    Next podcast".to_string(),
            "  C-p, ↑    Previous podcast".to_string(),
            "  Enter     View episodes".to_string(),
            "  a         Add podcast".to_string(),
            "  d         Delete podcast".to_string(),
            "  /         Search podcasts".to_string(),
            "  F6        Clear filters".to_string(),
            "  r         Refresh feeds".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for PodcastListBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.select_previous();
                // Scroll adjustment happens in render based on area size
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.select_next();
                // Scroll adjustment happens in render based on area size
                UIAction::Render
            }
            UIAction::PageUp => {
                let count = self.visible_count();
                if count == 0 {
                    return UIAction::None;
                }

                // Move up by 10 items or to the top
                if let Some(current) = self.selected_index {
                    self.selected_index = Some(current.saturating_sub(10));
                } else {
                    self.selected_index = Some(0);
                }
                UIAction::Render
            }
            UIAction::PageDown => {
                let count = self.visible_count();
                if count == 0 {
                    return UIAction::None;
                }

                // Move down by 10 items or to the bottom
                if let Some(current) = self.selected_index {
                    self.selected_index = Some((current + 10).min(count - 1));
                } else {
                    self.selected_index = Some(0);
                }
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(podcast) = self.selected_podcast() {
                    // Create action to open episode list for this podcast
                    UIAction::OpenEpisodeList {
                        podcast_name: podcast.title.clone(),
                        podcast_id: podcast.id.clone(),
                    }
                } else {
                    UIAction::ShowMessage("No podcast selected".to_string())
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
                if self.visible_count() > 0 {
                    self.selected_index = Some(self.visible_count() - 1);
                    // Scroll adjustment happens in render based on area size
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::Refresh => {
                // For now, just show a message. Actual refresh would be async
                if self.selected_podcast().is_some() {
                    UIAction::ShowMessage(
                        "Refresh selected podcast (not implemented yet)".to_string(),
                    )
                } else {
                    UIAction::ShowMessage("No podcast selected to refresh".to_string())
                }
            }
            UIAction::Search => UIAction::Search,
            UIAction::ApplySearch { query } => {
                self.filter.text_query = if query.is_empty() { None } else { Some(query) };
                self.apply_filters();
                UIAction::Render
            }
            UIAction::ClearFilters => {
                self.filter = PodcastFilter::default();
                self.apply_filters();
                UIAction::Render
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        // Create the main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Main content
                Constraint::Length(1), // Status line
            ])
            .split(area);

        match &self.state {
            PodcastListState::Loading => {
                let loading_block = Block::default()
                    .title("Podcasts - Loading...")
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style());

                let loading_text = Paragraph::new("Loading podcasts...")
                    .block(loading_block)
                    .style(self.theme.text_style())
                    .wrap(Wrap { trim: true });

                frame.render_widget(loading_text, chunks[0]);
            }
            PodcastListState::Error(error) => {
                let error_block = Block::default()
                    .title("Podcasts - Error")
                    .borders(Borders::ALL)
                    .border_style(self.theme.error_style())
                    .title_style(self.theme.title_style());

                let error_text = Paragraph::new(error.as_str())
                    .block(error_block)
                    .style(self.theme.error_style())
                    .wrap(Wrap { trim: true });

                frame.render_widget(error_text, chunks[0]);
            }
            PodcastListState::Ready => {
                if self.podcasts.is_empty() {
                    let empty_block = Block::default()
                        .title("Podcasts")
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .title_style(self.theme.title_style());

                    let empty_text = Paragraph::new(
                        "No podcasts subscribed.\n\nPress 'a' to add a podcast feed.",
                    )
                    .block(empty_block)
                    .style(self.theme.muted_style())
                    .wrap(Wrap { trim: true });

                    frame.render_widget(empty_text, chunks[0]);
                } else if self.visible_count() == 0 && self.filter.is_active() {
                    // Filter active but nothing matches
                    let title = format!("Podcasts [{}]", self.filter.description());
                    let filter_block = Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .title_style(self.theme.title_style());

                    let filter_text = Paragraph::new(
                        "No podcasts match the current filter.\n\nPress F6 or run :clear-filters to reset.",
                    )
                    .block(filter_block)
                    .style(self.theme.muted_style())
                    .wrap(Wrap { trim: true });

                    frame.render_widget(filter_text, chunks[0]);
                } else {
                    let filtered_count = self.visible_count();

                    // Calculate visible height (subtract 2 for borders)
                    let visible_height = chunks[0].height.saturating_sub(2) as usize;

                    // Adjust scroll to keep selected item visible
                    self.adjust_scroll(visible_height);

                    // Calculate the range of items to display
                    let end_index = (self.scroll_offset + visible_height).min(filtered_count);

                    let items: Vec<ListItem> = self.filtered_indices[self.scroll_offset..end_index]
                        .iter()
                        .enumerate()
                        .map(|(display_index, &actual_index)| {
                            let display_pos = self.scroll_offset + display_index;
                            let podcast = &self.podcasts[actual_index];
                            let title = podcast.title.clone();
                            let author = podcast.author.as_deref().unwrap_or("Unknown");
                            let episode_count = ""; // TODO: Add episode count when available

                            let content = format!("  {} - {}{}", title, author, episode_count);
                            let style = if Some(display_pos) == self.selected_index {
                                self.theme.selected_style()
                            } else {
                                self.theme.text_style()
                            };

                            ListItem::new(Line::from(vec![Span::styled(content, style)]))
                        })
                        .collect();

                    // Build title with filter indicator
                    let block_title = if self.filter.is_active() {
                        format!("Podcasts [{}]", self.filter.description())
                    } else {
                        "Podcasts".to_string()
                    };

                    let list = List::new(items)
                        .block(
                            Block::default()
                                .title(block_title)
                                .borders(Borders::ALL)
                                .border_style(border_style)
                                .title_style(self.theme.title_style()),
                        )
                        .style(self.theme.text_style());

                    frame.render_widget(list, chunks[0]);
                }
            }
        }

        // Render status line
        let status_text = if let Some(msg) = &self.status_message {
            msg.clone()
        } else {
            match &self.state {
                PodcastListState::Ready if !self.podcasts.is_empty() => {
                    if let Some(index) = self.selected_index {
                        if self.filter.is_active() {
                            format!(
                                " {} of {} matching ({} total) ",
                                index + 1,
                                self.visible_count(),
                                self.podcasts.len()
                            )
                        } else {
                            format!(" {} of {} podcasts ", index + 1, self.podcasts.len())
                        }
                    } else if self.filter.is_active() {
                        format!(
                            " 0 of {} matching ({} total) ",
                            self.visible_count(),
                            self.podcasts.len()
                        )
                    } else {
                        format!(" {} podcasts ", self.podcasts.len())
                    }
                }
                _ => String::new(),
            }
        };

        if !status_text.is_empty() {
            let status_paragraph = Paragraph::new(status_text).style(self.theme.muted_style());
            frame.render_widget(status_paragraph, chunks[1]);
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

impl Default for PodcastListBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podcast_list_buffer_creation() {
        let buffer = PodcastListBuffer::new();
        assert_eq!(buffer.id(), "podcast-list");
        assert_eq!(buffer.name(), "Podcasts");
        assert!(!buffer.can_close());
        assert_eq!(buffer.selected_index, None); // Empty buffer has no selection
        assert_eq!(buffer.podcast_count(), 0);
    }

    #[test]
    fn test_set_podcasts() {
        let mut buffer = PodcastListBuffer::new();

        // Create mock podcasts
        let podcasts = vec![
            Podcast::new("Podcast 1".to_string(), "http://example.com/1".to_string()),
            Podcast::new("Podcast 2".to_string(), "http://example.com/2".to_string()),
        ];

        buffer.set_podcasts(podcasts);
        assert_eq!(buffer.podcast_count(), 2);
        assert_eq!(buffer.selected_index, Some(0)); // Auto-selects first
    }

    #[test]
    fn test_navigation_with_podcasts() {
        let mut buffer = PodcastListBuffer::new();

        // Add mock podcasts
        let podcasts = vec![
            Podcast::new("Podcast 1".to_string(), "http://example.com/1".to_string()),
            Podcast::new("Podcast 2".to_string(), "http://example.com/2".to_string()),
            Podcast::new("Podcast 3".to_string(), "http://example.com/3".to_string()),
        ];
        buffer.set_podcasts(podcasts);

        // Test moving down
        let action = buffer.handle_action(UIAction::MoveDown);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.selected_index, Some(1));

        // Test moving down again
        buffer.handle_action(UIAction::MoveDown);
        assert_eq!(buffer.selected_index, Some(2));

        // Test moving up
        let action = buffer.handle_action(UIAction::MoveUp);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.selected_index, Some(1));
    }

    #[test]
    fn test_text_search_filters_podcasts() {
        let mut buffer = PodcastListBuffer::new();
        let podcasts = vec![
            Podcast::new("Rust Radio".to_string(), "http://example.com/1".to_string()),
            Podcast::new(
                "Python Bytes".to_string(),
                "http://example.com/2".to_string(),
            ),
            Podcast::new(
                "Rust in Production".to_string(),
                "http://example.com/3".to_string(),
            ),
        ];
        buffer.set_podcasts(podcasts);
        assert_eq!(buffer.visible_count(), 3);

        buffer.handle_action(UIAction::ApplySearch {
            query: "rust".to_string(),
        });
        assert_eq!(buffer.visible_count(), 2);
        assert!(buffer.filter.is_active());

        let podcast = buffer.selected_podcast().expect("should have podcast");
        assert_eq!(podcast.title, "Rust Radio");

        buffer.handle_action(UIAction::MoveDown);
        let podcast = buffer.selected_podcast().expect("should have podcast");
        assert_eq!(podcast.title, "Rust in Production");
    }

    #[test]
    fn test_clear_podcast_filter() {
        let mut buffer = PodcastListBuffer::new();
        let podcasts = vec![
            Podcast::new("Alpha Show".to_string(), "http://example.com/a".to_string()),
            Podcast::new("Beta Show".to_string(), "http://example.com/b".to_string()),
        ];
        buffer.set_podcasts(podcasts);

        buffer.handle_action(UIAction::ApplySearch {
            query: "alpha".to_string(),
        });
        assert_eq!(buffer.visible_count(), 1);

        buffer.handle_action(UIAction::ClearFilters);
        assert_eq!(buffer.visible_count(), 2);
        assert!(!buffer.filter.is_active());
    }

    #[test]
    fn test_podcast_filter_no_matches() {
        let mut buffer = PodcastListBuffer::new();
        let podcasts = vec![Podcast::new(
            "My Podcast".to_string(),
            "http://example.com/1".to_string(),
        )];
        buffer.set_podcasts(podcasts);

        buffer.handle_action(UIAction::ApplySearch {
            query: "zzzzz".to_string(),
        });
        assert_eq!(buffer.visible_count(), 0);
        assert_eq!(buffer.selected_index, None);
        assert!(buffer.selected_podcast().is_none());
    }
}
