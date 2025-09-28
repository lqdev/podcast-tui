// Podcast list buffer - displays subscribed podcasts
//
// This buffer shows the list of subscribed podcasts and allows
// management operations like adding, removing, and refreshing feeds.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::ui::{
    buffers::{Buffer, BufferId},
    themes::Theme,
    UIAction, UIComponent,
};

/// Buffer for displaying the podcast list
pub struct PodcastListBuffer {
    id: String,
    podcasts: Vec<String>, // Placeholder - will use proper Podcast model in Sprint 2
    selected_index: Option<usize>,
    focused: bool,
    theme: Theme,
}

impl PodcastListBuffer {
    /// Create a new podcast list buffer
    pub fn new() -> Self {
        Self {
            id: "podcast-list".to_string(),
            podcasts: vec![
                "Example Podcast 1".to_string(),
                "Example Podcast 2".to_string(),
                "Example Podcast 3".to_string(),
            ],
            selected_index: Some(0),
            focused: false,
            theme: Theme::default(),
        }
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
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.select_next();
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(index) = self.selected_index {
                    if index < self.podcasts.len() {
                        // For now, just show a placeholder action
                        // In Sprint 2, this will open the episode list
                        UIAction::ShowMinibuffer(format!(
                            "Would open episodes for: {}",
                            self.podcasts[index]
                        ))
                    } else {
                        UIAction::None
                    }
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToTop => {
                if !self.podcasts.is_empty() {
                    self.selected_index = Some(0);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToBottom => {
                if !self.podcasts.is_empty() {
                    self.selected_index = Some(self.podcasts.len() - 1);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            _ => UIAction::None,
        }
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.podcasts
            .iter()
            .enumerate()
            .map(|(i, podcast)| {
                let content = format!(" {}", podcast);
                if Some(i) == self.selected_index {
                    ListItem::new(content).style(self.theme.selected_style())
                } else {
                    ListItem::new(content).style(self.theme.text_style())
                }
            })
            .collect();
        
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title("Podcasts")
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style()),
            )
            .style(self.theme.text_style());
        
        frame.render_widget(list, area);
        
        // Show status
        if self.podcasts.is_empty() {
            let empty_msg = "No podcasts subscribed. Press 'a' to add podcasts.";
            let status_area = Rect {
                x: area.x + 2,
                y: area.y + area.height / 2,
                width: area.width.saturating_sub(4),
                height: 1,
            };
            
            let status = ratatui::widgets::Paragraph::new(empty_msg)
                .style(self.theme.muted_style());
            frame.render_widget(status, status_area);
        } else if let Some(index) = self.selected_index {
            let status_msg = format!(" {} of {} podcasts ", index + 1, self.podcasts.len());
            let status_area = Rect {
                x: area.x + area.width.saturating_sub(status_msg.len() as u16 + 2),
                y: area.y + area.height - 1,
                width: status_msg.len() as u16,
                height: 1,
            };
            
            let status = ratatui::widgets::Paragraph::new(status_msg)
                .style(self.theme.muted_style());
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
        assert_eq!(buffer.selected_index, Some(0));
    }
    
    #[test]
    fn test_navigation() {
        let mut buffer = PodcastListBuffer::new();
        
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
        let mut buffer = PodcastListBuffer::new();
        
        // Move to top
        buffer.handle_action(UIAction::MoveToTop);
        assert_eq!(buffer.selected_index, Some(0));
        
        // Move up from top (should wrap to bottom)
        buffer.handle_action(UIAction::MoveUp);
        assert_eq!(buffer.selected_index, Some(buffer.podcasts.len() - 1));
        
        // Move down from bottom (should wrap to top)
        buffer.handle_action(UIAction::MoveDown);
        assert_eq!(buffer.selected_index, Some(0));
    }
}