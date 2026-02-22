// Help buffer - displays help information and keybindings
//
// This buffer shows context-sensitive help, keybindings, and usage information
// following Emacs conventions for help display.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
    /// Create a new help buffer with minimal fallback content.
    /// In practice, `keybindings_help()` is always used for the F1 buffer.
    pub fn new() -> Self {
        Self::with_content(
            "*Help*".to_string(),
            vec![
                "PODCAST TUI - HELP".to_string(),
                "================".to_string(),
                "".to_string(),
                "Press F1 or ? to view the full keybinding reference.".to_string(),
            ],
        )
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

    /// Create a help buffer for keybindings, built from auto-generated entries.
    ///
    /// `entries` comes from `KeyHandler::generate_help_text()` and reflects the
    /// *actual active bindings*, so the displayed help can never go stale.
    /// Entries are grouped by category with section headers in a logical order.
    pub fn keybindings_help(entries: Vec<(String, String, String)>) -> Self {
        use std::collections::HashMap;

        let mut content = vec![
            "KEYBINDING REFERENCE".to_string(),
            "===================".to_string(),
            "(Generated from your active keybinding configuration)".to_string(),
            "".to_string(),
        ];

        // Defines the display order for category sections.
        // Categories not in this list appear at the end in alphabetical order
        // (future-proofing: new categories appear without requiring changes here).
        let category_order: &[&str] = &[
            "NAVIGATION",
            "BUFFER MANAGEMENT",
            "APPLICATION",
            "PODCAST MANAGEMENT",
            "EPISODE STATUS & SORTING",
            "PLAYLISTS",
            "OPML IMPORT/EXPORT",
            "DEVICE SYNC",
            "AUDIO PLAYBACK",
        ];

        // Group entries by category, preserving (keys, desc) pairs.
        let mut by_category: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (cat, keys, desc) in entries {
            by_category.entry(cat).or_default().push((keys, desc));
        }

        // Render categories in the defined order.
        for &category in category_order {
            if let Some(bindings) = by_category.remove(category) {
                content.push(format!("{}:", category));
                for (keys, desc) in &bindings {
                    content.push(format!("  {:<24} {}", keys, desc));
                }
                content.push("".to_string());
            }
        }

        // Render any remaining categories not in the predefined order (alphabetically).
        let mut remaining: Vec<_> = by_category.into_iter().collect();
        remaining.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (category, bindings) in remaining {
            content.push(format!("{}:", category));
            for (keys, desc) in &bindings {
                content.push(format!("  {:<24} {}", keys, desc));
            }
            content.push("".to_string());
        }

        // Static supplementary section (not auto-generated from keybindings).
        content.push("NOW PLAYING BUFFER (F9):".to_string());
        content.push(
            "  Shows episode title, podcast name, progress bar, volume, and playback state."
                .to_string(),
        );
        content.push(
            "  All playback keys (S-P, +/-, C-←/→) work from any buffer, not just here."
                .to_string(),
        );

        Self::with_content("*Help: Keybindings*".to_string(), content)
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Help Buffer Navigation:".to_string(),
            "  ↑ ↓        Scroll up/down".to_string(),
            "  PageUp/Down  Page navigation".to_string(),
            "  Home/End   Jump to top/bottom".to_string(),
            "  Esc        Close help".to_string(),
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
                // Esc closes help buffer
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
        let entries = vec![
            (
                "APPLICATION".to_string(),
                "q".to_string(),
                "Quit application".to_string(),
            ),
            (
                "APPLICATION".to_string(),
                "F1".to_string(),
                "Show help".to_string(),
            ),
        ];
        let buffer = HelpBuffer::keybindings_help(entries);
        assert_eq!(buffer.name(), "*Help: Keybindings*");
        assert!(!buffer.content.is_empty());
    }

    #[test]
    fn test_keybindings_help_has_section_headers() {
        // Arrange
        let entries = vec![
            (
                "NAVIGATION".to_string(),
                "↑ / k".to_string(),
                "Move up".to_string(),
            ),
            (
                "NAVIGATION".to_string(),
                "↓ / j".to_string(),
                "Move down".to_string(),
            ),
            (
                "APPLICATION".to_string(),
                "q".to_string(),
                "Quit application".to_string(),
            ),
            (
                "AUDIO PLAYBACK".to_string(),
                "S-P".to_string(),
                "Toggle play / pause".to_string(),
            ),
        ];

        // Act
        let buffer = HelpBuffer::keybindings_help(entries);

        // Assert — section headers present in content
        assert!(
            buffer.content.iter().any(|line| line == "NAVIGATION:"),
            "Missing NAVIGATION section header"
        );
        assert!(
            buffer.content.iter().any(|line| line == "APPLICATION:"),
            "Missing APPLICATION section header"
        );
        assert!(
            buffer.content.iter().any(|line| line == "AUDIO PLAYBACK:"),
            "Missing AUDIO PLAYBACK section header"
        );
    }

    #[test]
    fn test_keybindings_help_groups_related_actions() {
        // Arrange — two audio actions and one non-audio action
        let entries = vec![
            (
                "AUDIO PLAYBACK".to_string(),
                "S-P".to_string(),
                "Toggle play / pause".to_string(),
            ),
            (
                "NAVIGATION".to_string(),
                "↑".to_string(),
                "Move up".to_string(),
            ),
            (
                "AUDIO PLAYBACK".to_string(),
                "+".to_string(),
                "Volume up".to_string(),
            ),
        ];

        // Act
        let buffer = HelpBuffer::keybindings_help(entries);

        // Assert — both audio entries appear consecutively after "AUDIO PLAYBACK:" header
        let audio_header_idx = buffer
            .content
            .iter()
            .position(|line| line == "AUDIO PLAYBACK:")
            .expect("AUDIO PLAYBACK header not found");

        let audio_section: Vec<&String> = buffer.content[audio_header_idx + 1..]
            .iter()
            .take_while(|line| !line.is_empty())
            .collect();

        assert_eq!(
            audio_section.len(),
            2,
            "Expected 2 audio bindings in section"
        );
        assert!(
            audio_section
                .iter()
                .any(|l| l.contains("Toggle play / pause")),
            "Missing Toggle play/pause in audio section"
        );
        assert!(
            audio_section.iter().any(|l| l.contains("Volume up")),
            "Missing Volume up in audio section"
        );
    }

    #[test]
    fn test_keybindings_help_section_order() {
        // Arrange — supply AUDIO PLAYBACK and NAVIGATION; NAVIGATION must appear first
        let entries = vec![
            (
                "AUDIO PLAYBACK".to_string(),
                "S-P".to_string(),
                "Toggle play / pause".to_string(),
            ),
            (
                "NAVIGATION".to_string(),
                "↑".to_string(),
                "Move up".to_string(),
            ),
        ];

        // Act
        let buffer = HelpBuffer::keybindings_help(entries);

        // Assert — NAVIGATION header appears before AUDIO PLAYBACK header
        let nav_idx = buffer
            .content
            .iter()
            .position(|line| line == "NAVIGATION:")
            .expect("NAVIGATION header not found");
        let audio_idx = buffer
            .content
            .iter()
            .position(|line| line == "AUDIO PLAYBACK:")
            .expect("AUDIO PLAYBACK header not found");
        assert!(
            nav_idx < audio_idx,
            "NAVIGATION should appear before AUDIO PLAYBACK"
        );
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
