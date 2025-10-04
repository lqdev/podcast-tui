// Minibuffer component - Emacs-style command input area
//
// The minibuffer is used for command input, prompts, and status messages,
// following Emacs conventions for user interaction.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::{themes::Theme, UIAction, UIComponent};

/// Types of minibuffer content
#[derive(Debug, Clone, PartialEq)]
pub enum MinibufferContent {
    /// No content (hidden)
    None,

    /// Display a message
    Message(String),

    /// Display an error
    Error(String),

    /// Show a prompt for user input (simple version)
    Input { prompt: String, input: String },

    /// Show a prompt for user input (full control)
    Prompt {
        prompt: String,
        input: String,
        cursor_pos: usize,
    },

    /// Show a prompt with completion support
    PromptWithCompletion {
        prompt: String,
        input: String,
        cursor_pos: usize,
        completions: Vec<String>,
        completion_index: Option<usize>,
    },

    /// Show command input (M-x)
    Command { input: String, cursor_pos: usize },

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
    /// Current completion candidates
    completion_candidates: Vec<String>,
    /// Current completion prefix
    completion_prefix: String,
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
            completion_candidates: Vec::new(),
            completion_prefix: String::new(),
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
            MinibufferContent::Input { .. }
                | MinibufferContent::Prompt { .. }
                | MinibufferContent::PromptWithCompletion { .. }
                | MinibufferContent::Command { .. }
        )
    }

    /// Add a character to the current input
    pub fn add_char(&mut self, ch: char) {
        match &mut self.content {
            MinibufferContent::Input { input, .. } => {
                input.push(ch);
            }
            MinibufferContent::Prompt {
                input, cursor_pos, ..
            } => {
                input.insert(*cursor_pos, ch);
                *cursor_pos += 1;
            }
            MinibufferContent::PromptWithCompletion {
                input,
                cursor_pos,
                completion_index,
                ..
            } => {
                input.insert(*cursor_pos, ch);
                *cursor_pos += 1;
                *completion_index = None; // Reset completion when typing
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
            MinibufferContent::Input { input, .. } => {
                input.pop();
            }
            MinibufferContent::Prompt {
                input, cursor_pos, ..
            } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    input.remove(*cursor_pos);
                }
            }
            MinibufferContent::PromptWithCompletion {
                input,
                cursor_pos,
                completion_index,
                ..
            } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                    input.remove(*cursor_pos);
                    *completion_index = None; // Reset completion when editing
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
            MinibufferContent::PromptWithCompletion { cursor_pos, .. } => {
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
            MinibufferContent::Prompt {
                input, cursor_pos, ..
            } => {
                if *cursor_pos < input.len() {
                    *cursor_pos += 1;
                }
            }
            MinibufferContent::PromptWithCompletion {
                input, cursor_pos, ..
            } => {
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
            MinibufferContent::Input { input, .. } => Some(input.clone()),
            MinibufferContent::Prompt { input, .. } => Some(input.clone()),
            MinibufferContent::PromptWithCompletion { input, .. } => Some(input.clone()),
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
                MinibufferContent::Prompt {
                    input, cursor_pos, ..
                } => {
                    *input = history_item;
                    *cursor_pos = input.len();
                }
                MinibufferContent::PromptWithCompletion {
                    input,
                    cursor_pos,
                    completion_index,
                    ..
                } => {
                    *input = history_item;
                    *cursor_pos = input.len();
                    *completion_index = None;
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
                    MinibufferContent::Prompt {
                        input, cursor_pos, ..
                    } => {
                        *input = history_item;
                        *cursor_pos = input.len();
                    }
                    MinibufferContent::PromptWithCompletion {
                        input,
                        cursor_pos,
                        completion_index,
                        ..
                    } => {
                        *input = history_item;
                        *cursor_pos = input.len();
                        *completion_index = None;
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
                    MinibufferContent::Prompt {
                        input, cursor_pos, ..
                    } => {
                        input.clear();
                        *cursor_pos = 0;
                    }
                    MinibufferContent::PromptWithCompletion {
                        input,
                        cursor_pos,
                        completion_index,
                        ..
                    } => {
                        input.clear();
                        *cursor_pos = 0;
                        *completion_index = None;
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

    /// Show a prompt with completion support
    pub fn show_prompt_with_completion(&mut self, prompt: String, completions: Vec<String>) {
        self.completion_candidates = completions.clone();
        self.completion_prefix = String::new();
        self.content = MinibufferContent::PromptWithCompletion {
            prompt,
            input: String::new(),
            cursor_pos: 0,
            completions,
            completion_index: None,
        };
        self.focused = true;
    }

    /// Handle tab completion
    pub fn tab_complete(&mut self) {
        match &mut self.content {
            MinibufferContent::PromptWithCompletion {
                input,
                completions,
                completion_index,
                cursor_pos,
                ..
            } => {
                // Filter completions based on current input
                let filtered: Vec<String> = if input.is_empty() {
                    // If input is empty, show all completions
                    completions.clone()
                } else {
                    // For buffer name completion, we need to handle partial command+buffer patterns
                    completions
                        .iter()
                        .filter(|completion| {
                            let completion_lower = completion.to_lowercase();
                            let input_lower = input.to_lowercase();

                            // Direct starts_with match
                            if completion_lower.starts_with(&input_lower) {
                                return true;
                            }

                            // For buffer commands, also check if the buffer name part matches
                            if let Some(space_pos) = completion.rfind(' ') {
                                let buffer_name = &completion[space_pos + 1..];
                                if let Some(input_space_pos) = input.rfind(' ') {
                                    let input_buffer_part = &input[input_space_pos + 1..];
                                    if buffer_name
                                        .to_lowercase()
                                        .starts_with(&input_buffer_part.to_lowercase())
                                    {
                                        return true;
                                    }
                                }
                            }

                            false
                        })
                        .cloned()
                        .collect()
                };

                if filtered.is_empty() {
                    return;
                }

                match completion_index {
                    None => {
                        // Start completion with first match
                        *completion_index = Some(0);
                        *input = filtered[0].clone();
                        *cursor_pos = input.len();
                    }
                    Some(current_index) => {
                        // Cycle to next completion
                        let next_index = (*current_index + 1) % filtered.len();
                        *completion_index = Some(next_index);
                        *input = filtered[next_index].clone();
                        *cursor_pos = input.len();
                    }
                }
            }
            _ => {
                // For other prompt types, try to complete from candidates
                if let Some(current_input) = self.current_input() {
                    let matches: Vec<String> = self
                        .completion_candidates
                        .iter()
                        .filter(|candidate| {
                            candidate
                                .to_lowercase()
                                .starts_with(&current_input.to_lowercase())
                        })
                        .cloned()
                        .collect();

                    if let Some(first_match) = matches.first() {
                        // Replace current input with first match
                        match &mut self.content {
                            MinibufferContent::Prompt {
                                input, cursor_pos, ..
                            } => {
                                *input = first_match.clone();
                                *cursor_pos = input.len();
                            }
                            MinibufferContent::Command { input, cursor_pos } => {
                                *input = first_match.clone();
                                *cursor_pos = input.len();
                            }
                            MinibufferContent::Input { input, .. } => {
                                *input = first_match.clone();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Set completion candidates
    pub fn set_completion_candidates(&mut self, candidates: Vec<String>) {
        self.completion_candidates = candidates;
    }

    /// Get current completion candidates  
    pub fn get_completion_candidates(&self) -> &[String] {
        &self.completion_candidates
    }

    /// Check if minibuffer is in command prompt mode (M-x)
    pub fn is_command_prompt(&self) -> bool {
        match &self.content {
            MinibufferContent::PromptWithCompletion { prompt, .. } => prompt == "M-x ",
            MinibufferContent::Command { .. } => true,
            _ => false,
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
            MinibufferContent::Error(err) => format!("Error: {err}"),
            MinibufferContent::Status(status) => status.clone(),
            MinibufferContent::CommandPrompt => "M-x ".to_string(),
            MinibufferContent::Hidden => String::new(),
            MinibufferContent::Input { prompt, input } => {
                format!("{prompt}{input}█")
            }
            MinibufferContent::Prompt {
                prompt,
                input,
                cursor_pos,
            } => {
                let mut text = format!("{prompt}{input}");
                if self.focused && *cursor_pos <= input.len() {
                    // Simple cursor representation
                    if *cursor_pos == input.len() {
                        text.push('█');
                    }
                }
                text
            }
            MinibufferContent::PromptWithCompletion {
                prompt,
                input,
                cursor_pos,
                completions,
                completion_index,
            } => {
                let mut text = format!("{prompt}{input}");
                if self.focused && *cursor_pos <= input.len() {
                    // Simple cursor representation
                    if *cursor_pos == input.len() {
                        text.push('█');
                    }
                }

                // Add completion hint if available
                if let Some(index) = completion_index {
                    if let Some(completion) = completions.get(*index) {
                        // Show the remaining part of the completion
                        if completion.starts_with(input) {
                            let suffix = &completion[input.len()..];
                            text.push_str(&format!(" [{}]", suffix));
                        } else {
                            text.push_str(&format!(" [{}]", completion));
                        }
                    }
                }
                text
            }
            MinibufferContent::Command { input, cursor_pos } => {
                let mut text = format!("M-x {input}");
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
            UIAction::TabComplete => {
                self.tab_complete();
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

        let paragraph = Paragraph::new(text).style(style).block(
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
