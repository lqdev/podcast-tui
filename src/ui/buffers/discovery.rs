// Discovery buffer - displays podcast search / trending results from PodcastIndex API
//
// Usage:
//   :discover <query>   — search PodcastIndex by keyword
//   :trending           — show trending podcasts
//
// Keybindings (within this buffer):
//   ↑ / k  Move up
//   ↓ / j  Move down
//   Enter  Subscribe to selected podcast
//   Esc    Close buffer

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    podcast::PodcastSearchResult,
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

/// State of the discovery buffer
#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveryState {
    /// Waiting for API response
    Loading,
    /// Results available (may be empty)
    Ready,
    /// API call failed
    Error(String),
}

/// Buffer that displays PodcastIndex search/trending results and lets the user
/// subscribe to a podcast by pressing Enter.
pub struct DiscoveryBuffer {
    id: String,
    /// Human-readable title (e.g. "Search: rust" or "Trending")
    display_title: String,
    results: Vec<PodcastSearchResult>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
    state: DiscoveryState,
    status_message: Option<String>,
}

impl DiscoveryBuffer {
    /// Create a new discovery buffer in loading state.
    ///
    /// `buffer_id` should be unique (e.g. `"discovery-rust"` or `"discovery-trending"`).
    /// `display_title` is shown in the buffer header (e.g. `"Search: rust"`, `"Trending"`).
    pub fn new(buffer_id: String, display_title: String) -> Self {
        Self {
            id: buffer_id,
            display_title,
            results: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
            state: DiscoveryState::Loading,
            status_message: None,
        }
    }

    /// Populate results after a successful API call.
    pub fn set_results(&mut self, results: Vec<PodcastSearchResult>) {
        self.results = results;
        self.state = DiscoveryState::Ready;
        self.selected_index = if self.results.is_empty() {
            None
        } else {
            Some(0)
        };
        self.scroll_offset = 0;
        self.status_message = None;
    }

    /// Transition to the error state.
    pub fn set_error(&mut self, message: String) {
        self.state = DiscoveryState::Error(message);
        self.selected_index = None;
    }

    /// Set a transient status message shown in the footer.
    pub fn set_status_message(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    /// Returns the currently selected result (if any).
    pub fn selected_result(&self) -> Option<&PodcastSearchResult> {
        self.selected_index.and_then(|i| self.results.get(i))
    }

    // ── Navigation helpers ────────────────────────────────────────────────────

    fn move_up(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx > 0 {
                self.selected_index = Some(idx - 1);
                if idx - 1 < self.scroll_offset {
                    self.scroll_offset = idx - 1;
                }
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx + 1 < self.results.len() {
                self.selected_index = Some(idx + 1);
                // We show RESULTS_PER_PAGE rows; scroll when selection would go offscreen
                if idx + 1 >= self.scroll_offset + RESULTS_PER_PAGE {
                    self.scroll_offset = idx + 2 - RESULTS_PER_PAGE;
                }
            }
        }
    }

    fn move_to_top(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = Some(0);
            self.scroll_offset = 0;
        }
    }

    fn move_to_bottom(&mut self) {
        if !self.results.is_empty() {
            let last = self.results.len() - 1;
            self.selected_index = Some(last);
            self.scroll_offset = last.saturating_sub(RESULTS_PER_PAGE - 1);
        }
    }

    // ── Rendering helpers ─────────────────────────────────────────────────────

    fn build_result_items(&self) -> Vec<ListItem<'_>> {
        let visible = self
            .results
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(RESULTS_PER_PAGE);

        visible
            .map(|(i, r)| {
                let selected = self.selected_index == Some(i);
                let prefix = if selected { "▶ " } else { "  " };

                let title_style = if selected {
                    Style::default()
                        .fg(self.theme.colors.primary)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default().fg(self.theme.colors.text)
                };
                let meta_style = Style::default().fg(self.theme.colors.subtext);

                let cats = r.category_names();
                let cat_str = if cats.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", cats.join(", "))
                };

                let desc = if r.description.len() > DESC_MAX_LEN {
                    format!("{}…", &r.description[..DESC_MAX_LEN])
                } else {
                    r.description.clone()
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(prefix.to_string(), title_style),
                        Span::styled(r.title.clone(), title_style),
                    ]),
                    Line::from(Span::styled(
                        format!("   By: {}{}", r.author, cat_str),
                        meta_style,
                    )),
                    Line::from(Span::styled(format!("   {}", desc), meta_style)),
                    Line::from(""),
                ])
            })
            .collect()
    }
}

/// Number of results visible at once (each takes 4 lines).
const RESULTS_PER_PAGE: usize = 8;
/// Max description characters shown inline.
const DESC_MAX_LEN: usize = 120;

// ── UIComponent ───────────────────────────────────────────────────────────────

impl UIComponent for DiscoveryBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.move_up();
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.move_down();
                UIAction::Render
            }
            UIAction::MoveToTop => {
                self.move_to_top();
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                self.move_to_bottom();
                UIAction::Render
            }
            UIAction::PageUp => {
                for _ in 0..RESULTS_PER_PAGE {
                    self.move_up();
                }
                UIAction::Render
            }
            UIAction::PageDown => {
                for _ in 0..RESULTS_PER_PAGE {
                    self.move_down();
                }
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(result) = self.selected_result() {
                    UIAction::SubscribeFromDiscovery {
                        feed_url: result.feed_url.clone(),
                    }
                } else {
                    UIAction::None
                }
            }
            UIAction::CloseCurrentBuffer => UIAction::CloseBuffer(self.id.clone()),
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Min(0),    // results / message
                Constraint::Length(2), // footer hints
            ])
            .split(area);

        // Header
        let header_text = match &self.state {
            DiscoveryState::Loading => {
                format!("Discover Podcasts — {} (loading…)", self.display_title)
            }
            DiscoveryState::Ready => format!(
                "Discover Podcasts — {} ({} result{})",
                self.display_title,
                self.results.len(),
                if self.results.len() == 1 { "" } else { "s" }
            ),
            DiscoveryState::Error(_) => format!("Discover Podcasts — {}", self.display_title),
        };
        let header = Paragraph::new(header_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if self.focused {
                        self.theme.colors.border_focused
                    } else {
                        self.theme.colors.border
                    })),
            )
            .style(Style::default().fg(self.theme.colors.text));
        frame.render_widget(header, chunks[0]);

        // Results / state
        match &self.state {
            DiscoveryState::Loading => {
                let loading = Paragraph::new("Searching PodcastIndex…")
                    .block(Block::default().borders(Borders::NONE))
                    .style(Style::default().fg(self.theme.colors.subtext));
                frame.render_widget(loading, chunks[1]);
            }
            DiscoveryState::Error(msg) => {
                let error_msg = msg.clone();
                let error = Paragraph::new(error_msg)
                    .block(Block::default().borders(Borders::NONE))
                    .style(Style::default().fg(self.theme.colors.error))
                    .wrap(Wrap { trim: true });
                frame.render_widget(error, chunks[1]);
            }
            DiscoveryState::Ready if self.results.is_empty() => {
                let empty = Paragraph::new("No results found.")
                    .block(Block::default().borders(Borders::NONE))
                    .style(Style::default().fg(self.theme.colors.subtext));
                frame.render_widget(empty, chunks[1]);
            }
            DiscoveryState::Ready => {
                let items = self.build_result_items();
                let list = List::new(items)
                    .block(Block::default().borders(Borders::NONE))
                    .style(Style::default().fg(self.theme.colors.text));
                frame.render_widget(list, chunks[1]);
            }
        }

        // Footer hints / status
        let footer_text = if let Some(ref msg) = self.status_message {
            msg.clone()
        } else {
            "[Enter] Subscribe   [↑↓ / j k] Navigate   [Esc] Close".to_string()
        };
        let footer =
            Paragraph::new(footer_text).style(Style::default().fg(self.theme.colors.subtext));
        frame.render_widget(footer, chunks[2]);
    }

    fn title(&self) -> String {
        format!("*Discover: {}*", self.display_title)
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

// ── Buffer ─────────────────────────────────────────────────────────────────────

impl Buffer for DiscoveryBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.title()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "DISCOVERY BUFFER".to_string(),
            "".to_string(),
            "  ↑ / k     Move up".to_string(),
            "  ↓ / j     Move down".to_string(),
            "  Enter     Subscribe to selected podcast".to_string(),
            "  Esc       Close buffer".to_string(),
            "".to_string(),
            "Commands:".to_string(),
            "  :discover <query>   Search PodcastIndex".to_string(),
            "  :trending           Show trending podcasts".to_string(),
        ]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(title: &str, feed_url: &str) -> PodcastSearchResult {
        PodcastSearchResult {
            title: title.to_string(),
            author: "Author".to_string(),
            feed_url: feed_url.to_string(),
            description: "Description".to_string(),
            artwork_url: None,
            categories: Default::default(),
        }
    }

    #[test]
    fn test_discovery_buffer_new_is_loading() {
        // Arrange / Act
        let buf = DiscoveryBuffer::new("discovery-test".to_string(), "test".to_string());
        // Assert
        assert_eq!(buf.state, DiscoveryState::Loading);
        assert!(buf.selected_index.is_none());
    }

    #[test]
    fn test_set_results_selects_first() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        let results = vec![
            make_result("Podcast A", "https://a.com/feed"),
            make_result("Podcast B", "https://b.com/feed"),
        ];
        // Act
        buf.set_results(results);
        // Assert
        assert_eq!(buf.state, DiscoveryState::Ready);
        assert_eq!(buf.selected_index, Some(0));
        assert_eq!(buf.results.len(), 2);
    }

    #[test]
    fn test_set_results_empty_no_selection() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        // Act
        buf.set_results(vec![]);
        // Assert
        assert_eq!(buf.state, DiscoveryState::Ready);
        assert!(buf.selected_index.is_none());
    }

    #[test]
    fn test_set_error_transitions_state() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        // Act
        buf.set_error("API down".to_string());
        // Assert
        assert_eq!(buf.state, DiscoveryState::Error("API down".to_string()));
    }

    #[test]
    fn test_move_down_increments_selection() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        buf.set_results(vec![
            make_result("A", "https://a.com/feed"),
            make_result("B", "https://b.com/feed"),
        ]);
        // Act
        buf.move_down();
        // Assert
        assert_eq!(buf.selected_index, Some(1));
    }

    #[test]
    fn test_move_down_does_not_exceed_bounds() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        buf.set_results(vec![make_result("A", "https://a.com/feed")]);
        // Act
        buf.move_down();
        // Assert: stays at 0
        assert_eq!(buf.selected_index, Some(0));
    }

    #[test]
    fn test_move_up_decrements_selection() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        buf.set_results(vec![
            make_result("A", "https://a.com/feed"),
            make_result("B", "https://b.com/feed"),
        ]);
        buf.move_down(); // now at 1
                         // Act
        buf.move_up();
        // Assert
        assert_eq!(buf.selected_index, Some(0));
    }

    #[test]
    fn test_move_to_top_and_bottom() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        buf.set_results(vec![
            make_result("A", "https://a.com/feed"),
            make_result("B", "https://b.com/feed"),
            make_result("C", "https://c.com/feed"),
        ]);
        // Act
        buf.move_to_bottom();
        assert_eq!(buf.selected_index, Some(2));
        buf.move_to_top();
        assert_eq!(buf.selected_index, Some(0));
    }

    #[test]
    fn test_select_item_action_returns_subscribe() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        buf.set_results(vec![make_result("A", "https://a.com/feed")]);
        // Act
        let action = buf.handle_action(UIAction::SelectItem);
        // Assert
        assert!(matches!(
            action,
            UIAction::SubscribeFromDiscovery { feed_url } if feed_url == "https://a.com/feed"
        ));
    }

    #[test]
    fn test_select_item_with_no_results_returns_none() {
        // Arrange
        let mut buf = DiscoveryBuffer::new("id".to_string(), "test".to_string());
        buf.set_results(vec![]);
        // Act
        let action = buf.handle_action(UIAction::SelectItem);
        // Assert
        assert_eq!(action, UIAction::None);
    }

    #[test]
    fn test_buffer_id_and_name() {
        // Arrange
        let buf = DiscoveryBuffer::new("discovery-rust".to_string(), "rust".to_string());
        // Assert
        assert_eq!(buf.id(), "discovery-rust");
        assert_eq!(buf.name(), "*Discover: rust*");
    }
}
