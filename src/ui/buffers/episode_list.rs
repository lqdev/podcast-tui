// Episode list buffer - displays episodes for a selected podcast
//
// This buffer shows episodes from a podcast and allows playback,
// download, and queue management operations.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::ui::{
    buffers::{Buffer, BufferId},
    themes::Theme,
    UIAction, UIComponent,
};

/// Buffer for displaying episodes from a podcast
pub struct EpisodeListBuffer {
    id: String,
    podcast_name: String,
    episodes: Vec<String>, // Placeholder - will use proper Episode model in Sprint 2
    selected_index: Option<usize>,
    focused: bool,
    theme: Theme,
}

impl EpisodeListBuffer {
    /// Create a new episode list buffer for a podcast
    pub fn new(podcast_name: String) -> Self {
        let episodes = vec![
            "Episode 1: Introduction".to_string(),
            "Episode 2: Getting Started".to_string(),
            "Episode 3: Advanced Topics".to_string(),
            "Episode 4: Q&A Session".to_string(),
            "Episode 5: Wrap Up".to_string(),
        ];
        
        Self {
            id: format!("episodes-{}", podcast_name.replace(' ', "-").to_lowercase()),
            podcast_name,
            episodes,
            selected_index: Some(0),
            focused: false,
            theme: Theme::default(),
        }
    }
    
    /// Move selection up
    fn select_previous(&mut self) {
        if self.episodes.is_empty() {
            return;
        }
        
        self.selected_index = match self.selected_index {
            Some(i) if i == 0 => Some(self.episodes.len() - 1),
            Some(i) => Some(i - 1),
            None => Some(0),
        };
    }
    
    /// Move selection down
    fn select_next(&mut self) {
        if self.episodes.is_empty() {
            return;
        }
        
        self.selected_index = match self.selected_index {
            Some(i) if i >= self.episodes.len() - 1 => Some(0),
            Some(i) => Some(i + 1),
            None => Some(0),
        };
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
    
    fn can_close(&self) -> bool {
        true
    }
    
    fn help_text(&self) -> Vec<String> {
        vec![
            "Episode List Commands:".to_string(),
            "  C-n, ↓    Next episode".to_string(),
            "  C-p, ↑    Previous episode".to_string(),
            "  Enter     Play episode".to_string(),
            "  Space     Play/pause episode".to_string(),
            "  d         Download episode".to_string(),
            "  q         Add to queue".to_string(),
            "  m         Mark as played".to_string(),
            "  u         Mark as unplayed".to_string(),
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
                if let Some(index) = self.selected_index {
                    if index < self.episodes.len() {
                        // For now, just show a placeholder action
                        // In Sprint 3, this will start playback
                        UIAction::ShowMinibuffer(format!(
                            "Would play: {}",
                            self.episodes[index]
                        ))
                    } else {
                        UIAction::None
                    }
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToTop => {
                if !self.episodes.is_empty() {
                    self.selected_index = Some(0);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToBottom => {
                if !self.episodes.is_empty() {
                    self.selected_index = Some(self.episodes.len() - 1);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            _ => UIAction::None,
        }
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.episodes
            .iter()
            .enumerate()
            .map(|(i, episode)| {
                let status_indicator = "●"; // Placeholder for play status
                let content = format!(" {} {}", status_indicator, episode);
                
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
                    .title(format!("Episodes: {}", self.podcast_name))
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style()),
            )
            .style(self.theme.text_style());
        
        frame.render_widget(list, area);
        
        // Show status
        if self.episodes.is_empty() {
            let empty_msg = "No episodes available.";
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
            let status_msg = format!(" {} of {} episodes ", index + 1, self.episodes.len());
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_episode_list_buffer_creation() {
        let podcast_name = "Test Podcast".to_string();
        let buffer = EpisodeListBuffer::new(podcast_name.clone());
        
        assert_eq!(buffer.id(), "episodes-test-podcast");
        assert_eq!(buffer.name(), "Episodes: Test Podcast");
        assert!(buffer.can_close());
        assert_eq!(buffer.selected_index, Some(0));
        assert_eq!(buffer.podcast_name, podcast_name);
    }
    
    #[test]
    fn test_navigation() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string());
        
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
        let mut buffer = EpisodeListBuffer::new("Test".to_string());
        
        // Move to top
        buffer.handle_action(UIAction::MoveToTop);
        assert_eq!(buffer.selected_index, Some(0));
        
        // Move up from top (should wrap to bottom)
        buffer.handle_action(UIAction::MoveUp);
        assert_eq!(buffer.selected_index, Some(buffer.episodes.len() - 1));
        
        // Move down from bottom (should wrap to top)
        buffer.handle_action(UIAction::MoveDown);
        assert_eq!(buffer.selected_index, Some(0));
    }
    
    #[test]
    fn test_episode_selection() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string());
        
        // Select an episode
        let action = buffer.handle_action(UIAction::SelectItem);
        match action {
            UIAction::ShowMinibuffer(msg) => {
                assert!(msg.contains("Would play:"));
                assert!(msg.contains("Episode 1"));
            }
            _ => panic!("Expected ShowMinibuffer action"),
        }
    }
}