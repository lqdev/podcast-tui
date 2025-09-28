// Minibuffer component - Emacs-style command input area
//
// The minibuffer is used for command input, prompts, and status messages,
// following Emacs conventions for user interaction.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::{
    themes::Theme,
    UIAction, UIComponent,
};

/// Types of minibuffer content
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferContent {
    /// No content (hidden)
    None,
    
    /// Display a message
    Message(String),
    
    /// Display an error
    Error(String),
    
    /// Show a prompt for user input
    Prompt {
        prompt: String,
        input: String,
        cursor_pos: usize,
    },
    
    /// Show command input (M-x)
    Command {
        input: String,
        cursor_pos: usize,
    },
    
    /// Command prompt (alias for Command)
    CommandPrompt,
    
    /// Status message
    Status(String),
    
    /// Hidden state
    Hidden,
}

/// Minibuffer component for command input and messages
pub struct Minibuffer {
    content: MinibufferContent,
    theme: Theme,
    focused: bool,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl Minibuffer {
    /// Create a new minibuffer
    pub fn new() -> Self {
        Self {
            content: MinibufferContent::None,
            theme: Theme::default(),
            focused: false,
            history: Vec::new(),
            history_index: None,
        }
    }
    
    /// Show a simple message
    pub fn show_message(&mut self, message: String) {
        self.content = MinibufferContent::Message(message);
    }
    
    /// Show an error message
    pub fn show_error(&mut self, error: String) {
        self.content = MinibufferContent::Error(error);
    }
    
    /// Show a status message
    pub fn show_status(&mut self, status: String) {
        self.content = MinibufferContent::Status(status);
    }
    
    /// Show a prompt for user input
    pub fn show_prompt(&mut self, prompt: String) {
        self.content = MinibufferContent::Prompt {
            prompt,
            input: String::new(),
            cursor_pos: 0,
        };
        self.focused = true;
    }
    
    /// Show command input prompt (M-x)
    pub fn show_command_prompt(&mut self) {
        self.content = MinibufferContent::Command {
            input: String::new(),
            cursor_pos: 0,
        };
        self.focused = true;
    }
    
    /// Hide the minibuffer
    pub fn hide(&mut self) {
        self.content = MinibufferContent::None;
        self.focused = false;
        self.history_index = None;
    }
    
    /// Check if minibuffer is visible
    pub fn is_visible(&self) -> bool {
        !matches!(self.content, MinibufferContent::None)
    }
    
    /// Clear the minibuffer (alias for hide)
    pub fn clear(&mut self) {
        self.hide();
    }
    
    /// Set the content of the minibuffer
    pub fn set_content(&mut self, content: MinibufferContent) {
        match content {
            MinibufferContent::Message(msg) => self.show_message(msg),
            MinibufferContent::Error(err) => self.show_error(err),
            MinibufferContent::CommandPrompt => self.show_command_prompt(),
            _ => {
                self.content = content;
                self.focused = true;
            }
        }
    }
    
    /// Check if minibuffer is accepting input
    pub fn is_input_mode(&self) -> bool {
        matches!(
            self.content,
            MinibufferContent::Prompt { .. } | MinibufferContent::Command { .. }
        )
    }
    
    /// Add a character to the current input
    pub fn add_char(&mut self, ch: char) {
        match &mut self.content {
            MinibufferContent::Prompt { input, cursor_pos, .. } => {
                input.insert(*cursor_pos, ch);
                *cursor_pos += 1;
            }
            MinibufferContent::Command { input, cursor_pos } => {
                input.insert(*cursor_pos, ch);
                *cursor_pos += 1;
            }
            _ => {}
        }
    }
    
    /// Remove the character before the cursor
    pub fn backspace(&mut self) {
        match &mut self.content {
            MinibufferContent::Prompt { input, cursor_pos, .. } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    input.remove(*cursor_pos);
                }
            }
            MinibufferContent::Command { input, cursor_pos } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    input.remove(*cursor_pos);
                }
            }
            _ => {}
        }
    }
    
    /// Move cursor left
    pub fn cursor_left(&mut self) {
        match &mut self.content {
            MinibufferContent::Prompt { cursor_pos, .. } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            MinibufferContent::Command { cursor_pos, .. } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            _ => {}
        }
    }
    
    /// Move cursor right
    pub fn cursor_right(&mut self) {
        match &mut self.content {
            MinibufferContent::Prompt { input, cursor_pos, .. } => {
                if *cursor_pos < input.len() {
                    *cursor_pos += 1;
                }
            }
            MinibufferContent::Command { input, cursor_pos } => {
                if *cursor_pos < input.len() {
                    *cursor_pos += 1;
                }
            }
            _ => {}
        }
    }
    
    /// Get the current input text
    pub fn current_input(&self) -> Option<String> {
        match &self.content {
            MinibufferContent::Prompt { input, .. } => Some(input.clone()),
            MinibufferContent::Command { input, .. } => Some(input.clone()),
            _ => None,
        }
    }
    
    /// Submit the current input and return the result
    pub fn submit(&mut self) -> Option<String> {
        let input = self.current_input()?;
        
        // Add to history if not empty
        if !input.is_empty() && !self.history.contains(&input) {
            self.history.push(input.clone());
            // Keep history to a reasonable size
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        
        self.hide();
        Some(input)
    }
    
    /// Navigate history up
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        
        match self.history_index {
            None => {
                self.history_index = Some(self.history.len() - 1);
            }
            Some(i) if i > 0 => {
                self.history_index = Some(i - 1);
            }
            _ => return,
        }
        
        if let Some(index) = self.history_index {
            let history_item = self.history[index].clone();
            
            match &mut self.content {
                MinibufferContent::Prompt { input, cursor_pos, .. } => {
                    *input = history_item;
                    *cursor_pos = input.len();
                }
                MinibufferContent::Command { input, cursor_pos } => {
                    *input = history_item;
                    *cursor_pos = input.len();
                }
                _ => {}
            }
        }
    }
    
    /// Navigate history down
    pub fn history_down(&mut self) {
        if let Some(index) = self.history_index {
            if index < self.history.len() - 1 {
                self.history_index = Some(index + 1);
                let history_item = self.history[index + 1].clone();
                
                match &mut self.content {
                    MinibufferContent::Prompt { input, cursor_pos, .. } => {
                        *input = history_item;
                        *cursor_pos = input.len();
                    }
                    MinibufferContent::Command { input, cursor_pos } => {
                        *input = history_item;
                        *cursor_pos = input.len();
                    }
                    _ => {}
                }
            } else {
                // Clear input when going past the end of history
                self.history_index = None;
                match &mut self.content {
                    MinibufferContent::Prompt { input, cursor_pos, .. } => {
                        input.clear();
                        *cursor_pos = 0;
                    }
                    MinibufferContent::Command { input, cursor_pos } => {
                        input.clear();
                        *cursor_pos = 0;
                    }
                    _ => {}
                }
            }
        }
    }
    
    /// Set the theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
    
    /// Get the display text for the minibuffer
    fn display_text(&self) -> String {
        match &self.content {
            MinibufferContent::None => String::new(),
            MinibufferContent::Message(msg) => msg.clone(),
            MinibufferContent::Error(err) => format!("Error: {}", err),
            MinibufferContent::Status(status) => status.clone(),
            MinibufferContent::CommandPrompt => "M-x ".to_string(),
            MinibufferContent::Hidden => String::new(),
            MinibufferContent::Prompt { prompt, input, cursor_pos } => {
                let mut text = format!("{}{}", prompt, input);
                if self.focused && *cursor_pos <= input.len() {
                    // Simple cursor representation
                    if *cursor_pos == input.len() {
                        text.push('█');
                    }
                }
                text
            }
            MinibufferContent::Command { input, cursor_pos } => {
                let mut text = format!("M-x {}", input);
                if self.focused && *cursor_pos <= input.len() {
                    // Simple cursor representation
                    if *cursor_pos == input.len() {
                        text.push('█');
                    }
                }
                text
            }
        }
    }
    
    /// Get the appropriate style for the current content
    fn current_style(&self) -> ratatui::style::Style {
        match &self.content {
            MinibufferContent::Error(_) => self.theme.error_style(),
            MinibufferContent::Status(_) => self.theme.success_style(),
            MinibufferContent::CommandPrompt => self.theme.minibuffer_style(),
            MinibufferContent::Hidden => self.theme.minibuffer_style(),
            _ => self.theme.minibuffer_style(),
        }
    }
}

impl UIComponent for Minibuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        if !self.is_input_mode() {
            return UIAction::None;
        }
        
        match action {
            UIAction::MoveLeft => {
                self.cursor_left();
                UIAction::Render
            }
            UIAction::MoveRight => {
                self.cursor_right();
                UIAction::Render
            }
            UIAction::HideMinibuffer => {
                self.hide();
                UIAction::Render
            }
            UIAction::MinibufferInput(text) => {
                for ch in text.chars() {
                    self.add_char(ch);
                }
                UIAction::Render
            }
            _ => UIAction::None,
        }
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.is_visible() {
            return;
        }
        
        let text = self.display_text();
        let style = self.current_style();
        
        let paragraph = Paragraph::new(text)
            .style(style)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(self.theme.border_style()),
            );
        
        frame.render_widget(paragraph, area);
    }
    
    fn title(&self) -> String {
        "Minibuffer".to_string()
    }
    
    fn has_focus(&self) -> bool {
        self.focused
    }
    
    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Default for Minibuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_minibuffer_creation() {
        let minibuffer = Minibuffer::new();
        assert!(!minibuffer.is_visible());
        assert!(!minibuffer.is_input_mode());
        assert!(!minibuffer.has_focus());
    }
    
    #[test]
    fn test_show_message() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.show_message("Test message".to_string());
        
        assert!(minibuffer.is_visible());
        assert!(!minibuffer.is_input_mode());
    }
    
    #[test]
    fn test_show_prompt() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.show_prompt("Enter text: ".to_string());
        
        assert!(minibuffer.is_visible());
        assert!(minibuffer.is_input_mode());
        assert!(minibuffer.has_focus());
    }
    
    #[test]
    fn test_input_handling() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.show_prompt("Test: ".to_string());
        
        // Add some text
        minibuffer.add_char('H');
        minibuffer.add_char('e');
        minibuffer.add_char('l');
        minibuffer.add_char('l');
        minibuffer.add_char('o');
        
        assert_eq!(minibuffer.current_input(), Some("Hello".to_string()));
        
        // Test backspace
        minibuffer.backspace();
        assert_eq!(minibuffer.current_input(), Some("Hell".to_string()));
    }
    
    #[test]
    fn test_submit() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.show_command_prompt();
        
        minibuffer.add_char('q');
        minibuffer.add_char('u');
        minibuffer.add_char('i');
        minibuffer.add_char('t');
        
        let result = minibuffer.submit();
        assert_eq!(result, Some("quit".to_string()));
        assert!(!minibuffer.is_visible());
        assert!(minibuffer.history.contains(&"quit".to_string()));
    }
    
    #[test]
    fn test_history_navigation() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.history = vec!["command1".to_string(), "command2".to_string()];
        
        minibuffer.show_command_prompt();
        
        // Navigate up in history
        minibuffer.history_up();
        assert_eq!(minibuffer.current_input(), Some("command2".to_string()));
        
        minibuffer.history_up();
        assert_eq!(minibuffer.current_input(), Some("command1".to_string()));
        
        // Navigate down
        minibuffer.history_down();
        assert_eq!(minibuffer.current_input(), Some("command2".to_string()));
    }
    
    #[test]
    fn test_cursor_movement() {
        let mut minibuffer = Minibuffer::new();
        minibuffer.show_prompt("Test: ".to_string());
        
        minibuffer.add_char('A');
        minibuffer.add_char('B');
        minibuffer.add_char('C');
        
        // Move cursor left
        minibuffer.cursor_left();
        minibuffer.cursor_left();
        
        // Insert character at cursor position
        minibuffer.add_char('X');
        assert_eq!(minibuffer.current_input(), Some("AXBC".to_string()));
    }
}