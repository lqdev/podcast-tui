// Help buffer - displays help information and keybindings
//
// This buffer shows context-sensitive help, keybindings, and usage information
// following Emacs conventions for help display.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::{
    buffers::{Buffer, BufferId},
    themes::Theme,
    UIAction, UIComponent,
};

/// Help buffer that displays help content
pub struct HelpBuffer {
    id: String,
    title: String,
    content: Vec<String>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
}

impl HelpBuffer {
    /// Create a new help buffer with default content
    pub fn new() -> Self {
        Self::with_content("*Help*".to_string(), Self::default_help_content())
    }

    /// Create a help buffer with custom content
    pub fn with_content(title: String, content: Vec<String>) -> Self {
        Self {
            id: format!("help-{}", uuid::Uuid::new_v4()),
            title,
            content,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
        }
    }

    /// Create a help buffer for keybindings
    pub fn keybindings_help() -> Self {
        Self::with_content(
            "*Help: Keybindings*".to_string(),
            Self::keybindings_content(),
        )
    }

    /// Get default help content
    fn default_help_content() -> Vec<String> {
        vec![
            "PODCAST TUI - HELP".to_string(),
            "================".to_string(),
            "".to_string(),
            "This is a terminal user interface for managing podcasts.".to_string(),
            "It uses simple keybindings that work in any environment.".to_string(),
            "".to_string(),
            "BASIC NAVIGATION:".to_string(),
            "  ↑ ↓ ← →       Move around".to_string(),
            "  Page Up/Down  Page navigation".to_string(),
            "  Home/End      Move to beginning/end".to_string(),
            "".to_string(),
            "BUFFER MANAGEMENT:".to_string(),
            "  Tab/S-Tab     Switch between buffers".to_string(),
            "  F2            Switch to podcast list".to_string(),
            "  F3            Switch to help".to_string(),
            "".to_string(),
            "APPLICATION COMMANDS:".to_string(),
            "  q, F10        Quit application".to_string(),
            "  Esc           Cancel current operation".to_string(),
            "  F5            Refresh current buffer".to_string(),
            "  :             Execute command".to_string(),
            "  h, ?, F1      Show help".to_string(),
            "".to_string(),
            "CONTENT INTERACTION:".to_string(),
            "  Enter, Space  Select/activate item".to_string(),
            "".to_string(),
            "PODCAST MANAGEMENT:".to_string(),
            "  a             Add podcast subscription".to_string(),
            "  d             Delete podcast subscription".to_string(),
            "  r             Refresh selected podcast".to_string(),
            "  R             Refresh all podcasts".to_string(),
            "".to_string(),
            "EPISODE MANAGEMENT (in episode list):".to_string(),
            "  D             Download selected episode".to_string(),
            "  X             Delete downloaded episode file".to_string(),
            "  m             Mark episode as played".to_string(),
            "  u             Mark episode as unplayed".to_string(),
            "".to_string(),
            "DOWNLOAD LOCATION:".to_string(),
            "  Default: ~/Downloads/Podcasts/".to_string(),
            "  Config:  ~/.config/podcast-tui/config.json".to_string(),
            "".to_string(),
            "FUNCTION KEYS (Work everywhere):".to_string(),
            "  F1            Show help".to_string(),
            "  F2            Switch to podcast list".to_string(),
            "  F3            Switch to help buffer".to_string(),
            "  F5            Refresh current buffer".to_string(),
            "  F10           Quit application".to_string(),
            "".to_string(),
            "Note: These keybindings work in VS Code and other environments.".to_string(),
            "      Downloaded episodes are saved to ~/Downloads/Podcasts/ by default.".to_string(),
        ]
    }

    /// Get keybindings-specific help content
    fn keybindings_content() -> Vec<String> {
        vec![
            "KEYBINDING REFERENCE".to_string(),
            "===================".to_string(),
            "".to_string(),
            "PREFIX KEYS:".to_string(),
            "  C-x    File and buffer operations".to_string(),
            "  C-h    Help system".to_string(),
            "  C-c    Application-specific commands".to_string(),
            "".to_string(),
            "MOVEMENT COMMANDS:".to_string(),
            "  C-n    next-line (move down)".to_string(),
            "  C-p    previous-line (move up)".to_string(),
            "  C-f    forward-char (move right)".to_string(),
            "  C-b    backward-char (move left)".to_string(),
            "  C-a    beginning-of-line".to_string(),
            "  C-e    end-of-line".to_string(),
            "  C-v    scroll-down".to_string(),
            "  M-v    scroll-up".to_string(),
            "".to_string(),
            "BUFFER COMMANDS:".to_string(),
            "  C-x b  switch-to-buffer".to_string(),
            "  C-x k  kill-buffer".to_string(),
            "  C-x C-b list-buffers".to_string(),
            "".to_string(),
            "WINDOW COMMANDS:".to_string(),
            "  C-x 0  delete-window".to_string(),
            "  C-x 1  delete-other-windows".to_string(),
            "  C-x 2  split-window-below".to_string(),
            "  C-x 3  split-window-right".to_string(),
            "  C-x o  other-window".to_string(),
            "".to_string(),
            "HELP COMMANDS:".to_string(),
            "  C-h k  describe-key".to_string(),
            "  C-h b  describe-bindings".to_string(),
            "  C-h m  describe-mode".to_string(),
            "".to_string(),
            "UNIVERSAL COMMANDS:".to_string(),
            "  C-g    keyboard-quit".to_string(),
            "  C-x C-c exit-application".to_string(),
            "  M-x    execute-extended-command".to_string(),
        ]
    }

    /// Scroll the help content
    fn scroll(&mut self, delta: isize) {
        let new_offset = (self.scroll_offset as isize + delta).max(0) as usize;
        self.scroll_offset = new_offset.min(self.content.len().saturating_sub(1));
    }

    /// Set the theme for this buffer
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

impl Buffer for HelpBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.title.clone()
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Help Buffer Navigation:".to_string(),
            "  C-n, ↓    Scroll down".to_string(),
            "  C-p, ↑    Scroll up".to_string(),
            "  C-v       Page down".to_string(),
            "  M-v       Page up".to_string(),
            "  C-g       Close help".to_string(),
        ]
    }
}

impl UIComponent for HelpBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.scroll(-1);
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.scroll(1);
                UIAction::Render
            }
            UIAction::PageUp => {
                self.scroll(-10);
                UIAction::Render
            }
            UIAction::PageDown => {
                self.scroll(10);
                UIAction::Render
            }
            UIAction::MoveToTop => {
                self.scroll_offset = 0;
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                self.scroll_offset = self.content.len().saturating_sub(1);
                UIAction::Render
            }
            UIAction::HideMinibuffer => {
                // C-g closes help buffer
                UIAction::CloseBuffer(self.title.clone())
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate visible content
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        let end_index = (self.scroll_offset + visible_height).min(self.content.len());
        let visible_content: Vec<ListItem> = self.content[self.scroll_offset..end_index]
            .iter()
            .map(|line| ListItem::new(line.as_str()))
            .collect();

        // Create the help widget
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let help_widget = List::new(visible_content)
            .block(
                Block::default()
                    .title(self.title.clone())
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style()),
            )
            .style(self.theme.text_style())
            .highlight_style(self.theme.focused_style());

        frame.render_widget(help_widget, area);

        // Show scroll indicator if content is scrollable
        if self.content.len() > visible_height {
            let scroll_info = format!(" {}/{} ", self.scroll_offset + 1, self.content.len());

            let scroll_area = Rect {
                x: area.x + area.width.saturating_sub(scroll_info.len() as u16 + 2),
                y: area.y,
                width: scroll_info.len() as u16 + 2,
                height: 1,
            };

            let scroll_widget = Paragraph::new(scroll_info).style(self.theme.muted_style());

            frame.render_widget(scroll_widget, scroll_area);
        }
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Default for HelpBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_buffer_creation() {
        let buffer = HelpBuffer::new();
        assert_eq!(buffer.name(), "*Help*");
        assert!(buffer.can_close());
        assert!(!buffer.has_focus());
    }

    #[test]
    fn test_help_buffer_scrolling() {
        let mut buffer = HelpBuffer::new();
        let initial_offset = buffer.scroll_offset;

        // Test scrolling down
        let action = buffer.handle_action(UIAction::MoveDown);
        assert_eq!(action, UIAction::Render);
        assert!(buffer.scroll_offset > initial_offset);

        // Test scrolling up
        let action = buffer.handle_action(UIAction::MoveUp);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.scroll_offset, initial_offset);
    }

    #[test]
    fn test_custom_help_content() {
        let content = vec!["Line 1".to_string(), "Line 2".to_string()];
        let buffer = HelpBuffer::with_content("Custom Help".to_string(), content.clone());

        assert_eq!(buffer.name(), "Custom Help");
        assert_eq!(buffer.content, content);
    }

    #[test]
    fn test_keybindings_help() {
        let buffer = HelpBuffer::keybindings_help();
        assert_eq!(buffer.name(), "*Help: Keybindings*");
        assert!(!buffer.content.is_empty());
    }

    #[test]
    fn test_help_buffer_actions() {
        let mut buffer = HelpBuffer::new();

        // Test page down
        let action = buffer.handle_action(UIAction::PageDown);
        assert_eq!(action, UIAction::Render);

        // Test move to top
        let action = buffer.handle_action(UIAction::MoveToTop);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.scroll_offset, 0);

        // Test close action
        let action = buffer.handle_action(UIAction::HideMinibuffer);
        assert_eq!(action, UIAction::CloseBuffer("*Help*".to_string()));
    }
}
