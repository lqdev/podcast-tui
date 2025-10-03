// Simplified keybinding system for VS Code compatibility
//
// This module provides a simplified keybinding system that avoids 
// complex prefix keys that clash with VS Code.

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

/// Simplified keybinding handler (no complex prefix sequences)
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

    /// Set up default keybindings - Simple and VS Code friendly
    fn setup_default_bindings(&mut self) {
        // Basic navigation
        self.bind_key(KeyChord::ctrl(KeyCode::Char('n')), UIAction::MoveDown);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('p')), UIAction::MoveUp);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('f')), UIAction::MoveRight);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('b')), UIAction::MoveLeft);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('v')), UIAction::PageDown);
        self.bind_key(KeyChord::alt(KeyCode::Char('v')), UIAction::PageUp);

        // Arrow keys (alternative navigation)
        self.bind_key(KeyChord::none(KeyCode::Up), UIAction::MoveUp);
        self.bind_key(KeyChord::none(KeyCode::Down), UIAction::MoveDown);
        self.bind_key(KeyChord::none(KeyCode::Left), UIAction::MoveLeft);
        self.bind_key(KeyChord::none(KeyCode::Right), UIAction::MoveRight);
        self.bind_key(KeyChord::none(KeyCode::PageUp), UIAction::PageUp);
        self.bind_key(KeyChord::none(KeyCode::PageDown), UIAction::PageDown);
        self.bind_key(KeyChord::none(KeyCode::Home), UIAction::MoveToTop);
        self.bind_key(KeyChord::none(KeyCode::End), UIAction::MoveToBottom);

        // Function keys for direct actions (VS Code friendly)
        self.bind_key(KeyChord::none(KeyCode::F(1)), UIAction::ShowHelp);
        self.bind_key(KeyChord::none(KeyCode::F(2)), UIAction::SwitchBuffer("podcast-list".to_string()));
        self.bind_key(KeyChord::none(KeyCode::F(3)), UIAction::SwitchBuffer("*Help*".to_string()));
        self.bind_key(KeyChord::none(KeyCode::F(5)), UIAction::Refresh);

        // Content interaction
        self.bind_key(KeyChord::none(KeyCode::Enter), UIAction::SelectItem);
        self.bind_key(KeyChord::none(KeyCode::Char(' ')), UIAction::SelectItem);

        // Simple letter commands when not in input mode
        self.bind_key(KeyChord::none(KeyCode::Char('a')), UIAction::AddPodcast);
        self.bind_key(KeyChord::none(KeyCode::Char('d')), UIAction::DeletePodcast);
        self.bind_key(KeyChord::none(KeyCode::Char('r')), UIAction::RefreshPodcast);
        self.bind_key(KeyChord::shift(KeyCode::Char('R')), UIAction::RefreshAll);
        self.bind_key(KeyChord::none(KeyCode::Char('q')), UIAction::Quit);
        self.bind_key(KeyChord::none(KeyCode::Char('h')), UIAction::ShowHelp);
        self.bind_key(KeyChord::none(KeyCode::Char('?')), UIAction::ShowHelp);

        // Colon for commands (vi-style)
        self.bind_key(KeyChord::none(KeyCode::Char(':')), UIAction::PromptCommand);

        // Cancel/quit alternatives
        self.bind_key(KeyChord::ctrl(KeyCode::Char('g')), UIAction::ClearMinibuffer);
        self.bind_key(KeyChord::none(KeyCode::Esc), UIAction::ClearMinibuffer);
    }

    /// Bind a single key chord to an action
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

    /// Clear the current key sequence (for compatibility)
    pub fn clear_sequence(&mut self) {
        // No-op in simplified handler
    }

    /// Get the current key sequence as a string for display (for compatibility)
    pub fn current_sequence_string(&self) -> String {
        String::new()
    }

    /// Convert a key chord to its string representation
    fn chord_to_string(&self, chord: &KeyChord) -> String {
        let mut result = String::new();

        if chord.modifiers.contains(KeyModifiers::CONTROL) {
            result.push_str("C-");
        }
        if chord.modifiers.contains(KeyModifiers::ALT) {
            result.push_str("M-");
        }
        if chord.modifiers.contains(KeyModifiers::SHIFT) {
            result.push_str("S-");
        }

        match chord.code {
            KeyCode::Char(c) => result.push(c),
            KeyCode::Enter => result.push_str("RET"),
            KeyCode::Tab => result.push_str("TAB"),
            KeyCode::Backspace => result.push_str("DEL"),
            KeyCode::Delete => result.push_str("DELETE"),
            KeyCode::Insert => result.push_str("INSERT"),
            KeyCode::F(n) => result.push_str(&format!("F{}", n)),
            KeyCode::Up => result.push_str("UP"),
            KeyCode::Down => result.push_str("DOWN"),
            KeyCode::Left => result.push_str("LEFT"),
            KeyCode::Right => result.push_str("RIGHT"),
            KeyCode::Home => result.push_str("HOME"),
            KeyCode::End => result.push_str("END"),
            KeyCode::PageUp => result.push_str("PGUP"),
            KeyCode::PageDown => result.push_str("PGDN"),
            KeyCode::Esc => result.push_str("ESC"),
            _ => result.push_str("?"),
        }

        result
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

        let key_event = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        let action = handler.handle_key(key_event);
        assert_eq!(action, UIAction::MoveDown);
    }

    #[test]
    fn test_chord_to_string() {
        let handler = KeyHandler::new();

        let ctrl_n = KeyChord::ctrl(KeyCode::Char('n'));
        assert_eq!(handler.chord_to_string(&ctrl_n), "C-n");

        let alt_x = KeyChord::alt(KeyCode::Char('x'));
        assert_eq!(handler.chord_to_string(&alt_x), "M-x");

        let f1 = KeyChord::none(KeyCode::F(1));
        assert_eq!(handler.chord_to_string(&f1), "F1");
    }
}