// Status bar component - displays application status information
//
// The status bar shows current buffer information, key hints,
// and application status at the bottom of the screen.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::ui::{
    themes::Theme,
    UIAction, UIComponent,
};

/// Status bar component
pub struct StatusBar {
    theme: Theme,
    buffer_name: String,
    key_sequence: String,
    status_message: String,
    focused: bool,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
            buffer_name: String::new(),
            key_sequence: String::new(),
            status_message: String::new(),
            focused: false,
        }
    }
    
    /// Set the current buffer name
    pub fn set_buffer_name(&mut self, name: String) {
        self.buffer_name = name;
    }
    
    /// Set the current key sequence being typed
    pub fn set_key_sequence(&mut self, sequence: String) {
        self.key_sequence = sequence;
    }
    
    /// Set a status message
    pub fn set_status_message(&mut self, message: String) {
        self.status_message = message;
    }
    
    /// Clear the status message
    pub fn clear_status_message(&mut self) {
        self.status_message.clear();
    }
    
    /// Set the theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
    
    /// Get the left section content (buffer info)
    fn left_content(&self) -> String {
        if self.buffer_name.is_empty() {
            "Podcast TUI".to_string()
        } else {
            format!(" {} ", self.buffer_name)
        }
    }
    
    /// Get the center section content (status message or key sequence)
    fn center_content(&self) -> String {
        if !self.status_message.is_empty() {
            format!(" {} ", self.status_message)
        } else if !self.key_sequence.is_empty() {
            format!(" {} ", self.key_sequence)
        } else {
            String::new()
        }
    }
    
    /// Get the right section content (help hint)
    fn right_content(&self) -> String {
        " C-h for help, C-x C-c to quit ".to_string()
    }
}

impl UIComponent for StatusBar {
    fn handle_action(&mut self, _action: UIAction) -> UIAction {
        // Status bar doesn't handle actions directly
        UIAction::None
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Split the status bar into three sections
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.left_content().len() as u16),
                Constraint::Min(0),
                Constraint::Length(self.right_content().len() as u16),
            ])
            .split(area);
        
        // Left section - buffer name
        let left_text = self.left_content();
        let left_paragraph = Paragraph::new(left_text)
            .style(self.theme.statusbar_style())
            .block(Block::default());
        frame.render_widget(left_paragraph, chunks[0]);
        
        // Center section - status message or key sequence
        let center_text = self.center_content();
        if !center_text.is_empty() {
            let center_style = if !self.status_message.is_empty() {
                self.theme.success_style()
            } else {
                self.theme.warning_style()
            };
            
            let center_paragraph = Paragraph::new(center_text.clone())
                .style(center_style)
                .block(Block::default());
            frame.render_widget(center_paragraph, chunks[1]);
        }
        
        // Right section - help hint
        let right_text = self.right_content();
        let right_paragraph = Paragraph::new(right_text)
            .style(self.theme.muted_style())
            .block(Block::default());
        frame.render_widget(right_paragraph, chunks[2]);
        
        // If there's extra space in the middle, fill it with the status bar background
        if chunks[1].width > center_text.len() as u16 {
            let fill_area = Rect {
                x: chunks[1].x + center_text.len() as u16,
                y: chunks[1].y,
                width: chunks[1].width - center_text.len() as u16,
                height: chunks[1].height,
            };
            
            let fill_paragraph = Paragraph::new("")
                .style(self.theme.statusbar_style())
                .block(Block::default());
            frame.render_widget(fill_paragraph, fill_area);
        }
    }
    
    fn title(&self) -> String {
        "Status Bar".to_string()
    }
    
    fn has_focus(&self) -> bool {
        self.focused
    }
    
    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_status_bar_creation() {
        let status_bar = StatusBar::new();
        assert_eq!(status_bar.title(), "Status Bar");
        assert!(!status_bar.has_focus());
        assert!(status_bar.buffer_name.is_empty());
        assert!(status_bar.status_message.is_empty());
    }
    
    #[test]
    fn test_set_buffer_name() {
        let mut status_bar = StatusBar::new();
        status_bar.set_buffer_name("Test Buffer".to_string());
        assert_eq!(status_bar.buffer_name, "Test Buffer");
        assert_eq!(status_bar.left_content(), " Test Buffer ");
    }
    
    #[test]
    fn test_set_status_message() {
        let mut status_bar = StatusBar::new();
        status_bar.set_status_message("Test message".to_string());
        assert_eq!(status_bar.status_message, "Test message");
        assert_eq!(status_bar.center_content(), " Test message ");
        
        status_bar.clear_status_message();
        assert!(status_bar.status_message.is_empty());
    }
    
    #[test]
    fn test_set_key_sequence() {
        let mut status_bar = StatusBar::new();
        status_bar.set_key_sequence("C-x ".to_string());
        assert_eq!(status_bar.key_sequence, "C-x ");
        assert_eq!(status_bar.center_content(), " C-x  ");
    }
    
    #[test]
    fn test_default_content() {
        let status_bar = StatusBar::new();
        assert_eq!(status_bar.left_content(), "Podcast TUI");
        assert_eq!(status_bar.center_content(), "");
        assert!(status_bar.right_content().contains("C-h for help"));
    }
    
    #[test]
    fn test_priority_of_center_content() {
        let mut status_bar = StatusBar::new();
        
        // Set both status message and key sequence
        status_bar.set_status_message("Status".to_string());
        status_bar.set_key_sequence("C-x ".to_string());
        
        // Status message should take priority
        assert_eq!(status_bar.center_content(), " Status ");
        
        // Clear status message, key sequence should show
        status_bar.clear_status_message();
        assert_eq!(status_bar.center_content(), " C-x  ");
    }
}