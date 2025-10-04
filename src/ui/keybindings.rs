// Simple, clash-free keybinding system
//
// This module provides basic keybindings that work in most environments,
// including VS Code terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::ui::UIAction;

/// Represents a key combination
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
}

impl KeyChord {
    pub fn new(modifiers: KeyModifiers, code: KeyCode) -> Self {
        Self { modifiers, code }
    }

    pub fn ctrl(code: KeyCode) -> Self {
        Self::new(KeyModifiers::CONTROL, code)
    }

    pub fn alt(code: KeyCode) -> Self {
        Self::new(KeyModifiers::ALT, code)
    }

    pub fn shift(code: KeyCode) -> Self {
        Self::new(KeyModifiers::SHIFT, code)
    }

    pub fn none(code: KeyCode) -> Self {
        Self::new(KeyModifiers::NONE, code)
    }
}

impl From<KeyEvent> for KeyChord {
    fn from(key_event: KeyEvent) -> Self {
        Self::new(key_event.modifiers, key_event.code)
    }
}

/// Simple keybinding handler with clash-free keys
pub struct KeyHandler {
    /// Direct key bindings only
    bindings: HashMap<KeyChord, UIAction>,
}

impl KeyHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            bindings: HashMap::new(),
        };

        handler.setup_default_bindings();
        handler
    }

    /// Set up simple, clash-free keybindings
    fn setup_default_bindings(&mut self) {
        // Arrow keys - universal
        self.bind_key(KeyChord::none(KeyCode::Up), UIAction::MoveUp);
        self.bind_key(KeyChord::none(KeyCode::Down), UIAction::MoveDown);
        self.bind_key(KeyChord::none(KeyCode::Left), UIAction::MoveLeft);
        self.bind_key(KeyChord::none(KeyCode::Right), UIAction::MoveRight);
        self.bind_key(KeyChord::none(KeyCode::PageUp), UIAction::PageUp);
        self.bind_key(KeyChord::none(KeyCode::PageDown), UIAction::PageDown);
        self.bind_key(KeyChord::none(KeyCode::Home), UIAction::MoveToTop);
        self.bind_key(KeyChord::none(KeyCode::End), UIAction::MoveToBottom);

        // Function keys - rarely clash
        self.bind_key(KeyChord::none(KeyCode::F(1)), UIAction::ShowHelp);
        self.bind_key(
            KeyChord::none(KeyCode::F(2)),
            UIAction::SwitchBuffer("podcast-list".to_string()),
        );
        self.bind_key(
            KeyChord::none(KeyCode::F(3)),
            UIAction::SwitchBuffer("*Help*".to_string()),
        );
        self.bind_key(
            KeyChord::none(KeyCode::F(4)),
            UIAction::SwitchBuffer("downloads".to_string()),
        );
        self.bind_key(KeyChord::none(KeyCode::F(5)), UIAction::Refresh);
        self.bind_key(KeyChord::none(KeyCode::F(10)), UIAction::Quit);

        // Tab navigation
        self.bind_key(KeyChord::none(KeyCode::Tab), UIAction::NextBuffer);
        self.bind_key(KeyChord::shift(KeyCode::Tab), UIAction::PreviousBuffer);
        // Some terminals send BackTab for Shift+Tab
        self.bind_key(KeyChord::none(KeyCode::BackTab), UIAction::PreviousBuffer);
        self.bind_key(KeyChord::shift(KeyCode::BackTab), UIAction::PreviousBuffer);

        // Alternative buffer navigation (more reliable)
        self.bind_key(KeyChord::ctrl(KeyCode::PageUp), UIAction::PreviousBuffer);
        self.bind_key(KeyChord::ctrl(KeyCode::PageDown), UIAction::NextBuffer);

        // Basic interaction
        self.bind_key(KeyChord::none(KeyCode::Enter), UIAction::SelectItem);
        self.bind_key(KeyChord::none(KeyCode::Char(' ')), UIAction::SelectItem);
        self.bind_key(KeyChord::none(KeyCode::Esc), UIAction::HideMinibuffer);

        // Simple letter commands (when not in input mode)
        self.bind_key(KeyChord::none(KeyCode::Char('a')), UIAction::AddPodcast);
        self.bind_key(KeyChord::none(KeyCode::Char('d')), UIAction::DeletePodcast);
        self.bind_key(KeyChord::none(KeyCode::Char('r')), UIAction::RefreshPodcast);
        self.bind_key(KeyChord::shift(KeyCode::Char('R')), UIAction::RefreshAll);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('r')), UIAction::HardRefreshPodcast);
        self.bind_key(
            KeyChord::shift(KeyCode::Char('D')),
            UIAction::DownloadEpisode,
        );
        self.bind_key(
            KeyChord::shift(KeyCode::Char('X')),
            UIAction::DeleteDownloadedEpisode,
        );
        self.bind_key(KeyChord::none(KeyCode::Char('q')), UIAction::Quit);
        self.bind_key(KeyChord::none(KeyCode::Char('h')), UIAction::ShowHelp);
        self.bind_key(KeyChord::none(KeyCode::Char('?')), UIAction::ShowHelp);
        self.bind_key(KeyChord::none(KeyCode::Char(':')), UIAction::PromptCommand);
    }

    /// Bind a key chord to an action
    pub fn bind_key(&mut self, chord: KeyChord, action: UIAction) {
        self.bindings.insert(chord, action);
    }

    /// Handle a key event and return the corresponding action
    pub fn handle_key(&mut self, key_event: KeyEvent) -> UIAction {
        let chord = KeyChord::from(key_event);

        if let Some(action) = self.bindings.get(&chord) {
            return action.clone();
        }

        UIAction::None
    }

    /// Clear any current key sequence (not needed for simple handler)
    pub fn clear_sequence(&mut self) {
        // No-op for simple handler
    }

    /// Get the current key sequence as a string (empty for simple handler)
    pub fn current_sequence_string(&self) -> String {
        String::new()
    }
}

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_key_binding() {
        let mut handler = KeyHandler::new();

        let key_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        assert_eq!(action, UIAction::MoveUp);
    }

    #[test]
    fn test_function_key() {
        let mut handler = KeyHandler::new();

        let key_event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        assert_eq!(action, UIAction::ShowHelp);
    }
}
