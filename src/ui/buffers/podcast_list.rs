// Podcast list buffer - displays subscribed podcasts
//
// This buffer shows the list of subscribed podcasts and allows
// management operations like adding, removing, and refreshing feeds.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    podcast::{subscription::SubscriptionManager, Podcast},
    storage::JsonStorage,
    ui::{
        buffers::{Buffer, BufferId},
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
        if !self.podcasts.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(0);
        }
    }

    /// Get the currently selected podcast
    pub fn selected_podcast(&self) -> Option<&Podcast> {
        self.selected_index.and_then(|i| self.podcasts.get(i))
    }

    /// Get the number of podcasts
    pub fn podcast_count(&self) -> usize {
        self.podcasts.len()
    }

    /// Move selection up
    fn select_previous(&mut self) {
        if self.podcasts.is_empty() {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(i) if i == 0 => Some(self.podcasts.len() - 1),
            Some(i) => Some(i - 1),
            None => Some(0),
        };
    }

    /// Move selection down
    fn select_next(&mut self) {
        if self.podcasts.is_empty() {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(i) if i >= self.podcasts.len() - 1 => Some(0),
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
                if self.podcasts.is_empty() {
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
                if self.podcasts.is_empty() {
                    return UIAction::None;
                }
                
                // Move down by 10 items or to the bottom
                if let Some(current) = self.selected_index {
                    self.selected_index = Some((current + 10).min(self.podcasts.len() - 1));
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
                if !self.podcasts.is_empty() {
                    self.selected_index = Some(0);
                    self.scroll_offset = 0;
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToBottom => {
                if !self.podcasts.is_empty() {
                    self.selected_index = Some(self.podcasts.len() - 1);
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
                    .border_style(Style::default().fg(Color::Red))
                    .title_style(self.theme.title_style());

                let error_text = Paragraph::new(error.as_str())
                    .block(error_block)
                    .style(Style::default().fg(Color::Red))
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
                } else {
                    // Calculate visible height (subtract 2 for borders)
                    let visible_height = chunks[0].height.saturating_sub(2) as usize;
                    
                    // Adjust scroll to keep selected item visible
                    self.adjust_scroll(visible_height);

                    // Calculate the range of items to display
                    let end_index = (self.scroll_offset + visible_height).min(self.podcasts.len());
                    let visible_podcasts = &self.podcasts[self.scroll_offset..end_index];

                    let items: Vec<ListItem> = visible_podcasts
                        .iter()
                        .enumerate()
                        .map(|(visible_i, podcast)| {
                            let actual_i = self.scroll_offset + visible_i;
                            let title = podcast.title.clone();
                            let author = podcast.author.as_deref().unwrap_or("Unknown");
                            let episode_count = ""; // TODO: Add episode count when available

                            let content = format!("  {} - {}{}", title, author, episode_count);
                            let style = if Some(actual_i) == self.selected_index {
                                self.theme.selected_style()
                            } else {
                                self.theme.text_style()
                            };

                            ListItem::new(Line::from(vec![Span::styled(content, style)]))
                        })
                        .collect();

                    let list = List::new(items)
                        .block(
                            Block::default()
                                .title("Podcasts")
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
                        format!(" {} of {} podcasts ", index + 1, self.podcasts.len())
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
}
