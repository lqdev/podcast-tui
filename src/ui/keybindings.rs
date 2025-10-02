// Emacs-style keybinding system
//
// This module provides a comprehensive keybinding system that mimics Emacs
// keybindings, including support for prefix keys and customizable bindings.

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

/// Represents a sequence of key chords (for prefix keys like C-x)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence {
    pub chords: Vec<KeyChord>,
}

impl KeySequence {
    pub fn new(chords: Vec<KeyChord>) -> Self {
        Self { chords }
    }

    pub fn single(chord: KeyChord) -> Self {
        Self::new(vec![chord])
    }

    pub fn from_chord_and_key(prefix: KeyChord, key: KeyChord) -> Self {
        Self::new(vec![prefix, key])
    }
}

/// Emacs-style keybinding handler with support for prefix keys
pub struct KeyHandler {
    /// Direct key bindings (no prefix)
    direct_bindings: HashMap<KeyChord, UIAction>,

    /// Prefix key bindings (C-x, C-c, etc.)
    prefix_bindings: HashMap<KeySequence, UIAction>,

    /// Current prefix sequence being built
    current_sequence: Vec<KeyChord>,

    /// Known prefix keys
    prefix_keys: HashMap<KeyChord, String>,
}

impl KeyHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            direct_bindings: HashMap::new(),
            prefix_bindings: HashMap::new(),
            current_sequence: Vec::new(),
            prefix_keys: HashMap::new(),
        };

        handler.setup_default_bindings();
        handler
    }

    /// Set up default Emacs-style keybindings
    fn setup_default_bindings(&mut self) {
        // Define prefix keys
        self.add_prefix_key(KeyChord::ctrl(KeyCode::Char('x')), "C-x");
        self.add_prefix_key(KeyChord::ctrl(KeyCode::Char('c')), "C-c");
        self.add_prefix_key(KeyChord::ctrl(KeyCode::Char('h')), "C-h");

        // Basic navigation
        self.bind_key(KeyChord::ctrl(KeyCode::Char('n')), UIAction::MoveDown);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('p')), UIAction::MoveUp);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('f')), UIAction::MoveRight);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('b')), UIAction::MoveLeft);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('v')), UIAction::PageDown);
        self.bind_key(KeyChord::alt(KeyCode::Char('v')), UIAction::PageUp);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('a')), UIAction::MoveToTop);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('e')), UIAction::MoveToBottom);

        // Arrow keys (alternative navigation)
        self.bind_key(KeyChord::none(KeyCode::Up), UIAction::MoveUp);
        self.bind_key(KeyChord::none(KeyCode::Down), UIAction::MoveDown);
        self.bind_key(KeyChord::none(KeyCode::Left), UIAction::MoveLeft);
        self.bind_key(KeyChord::none(KeyCode::Right), UIAction::MoveRight);
        self.bind_key(KeyChord::none(KeyCode::PageUp), UIAction::PageUp);
        self.bind_key(KeyChord::none(KeyCode::PageDown), UIAction::PageDown);
        self.bind_key(KeyChord::none(KeyCode::Home), UIAction::MoveToTop);
        self.bind_key(KeyChord::none(KeyCode::End), UIAction::MoveToBottom);

        // Cancel/quit
        self.bind_key(KeyChord::ctrl(KeyCode::Char('g')), UIAction::HideMinibuffer);

        // Buffer switching
        self.bind_key(KeyChord::ctrl(KeyCode::Tab), UIAction::NextBuffer);
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('b')),
            ),
            UIAction::ShowMinibuffer("Switch buffer: ".to_string()),
        );

        // Buffer management (C-x prefix)
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::ctrl(KeyCode::Char('c')),
            ),
            UIAction::Quit,
        );

        // Window management (C-x prefix)
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('1')),
            ),
            UIAction::OnlyWindow,
        );
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('2')),
            ),
            UIAction::SplitHorizontal,
        );
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('3')),
            ),
            UIAction::SplitVertical,
        );
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('o')),
            ),
            UIAction::NextWindow,
        );
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('0')),
            ),
            UIAction::CloseWindow,
        );

        // Help system (C-h prefix)
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('h')),
                KeyChord::none(KeyCode::Char('k')),
            ),
            UIAction::ShowHelp,
        );

        // Direct help
        self.bind_key(KeyChord::none(KeyCode::F(1)), UIAction::ShowHelp);

        // Command execution
        self.bind_key(KeyChord::alt(KeyCode::Char('x')), UIAction::PromptCommand);

        // Content interaction
        self.bind_key(KeyChord::none(KeyCode::Enter), UIAction::SelectItem);
        self.bind_key(KeyChord::none(KeyCode::Char(' ')), UIAction::SelectItem);

        // Refresh
        self.bind_key(KeyChord::ctrl(KeyCode::Char('l')), UIAction::Refresh);

        // Podcast management
        self.bind_sequence(
            KeySequence::from_chord_and_key(
                KeyChord::ctrl(KeyCode::Char('x')),
                KeyChord::none(KeyCode::Char('a')),
            ),
            UIAction::AddPodcast,
        );
        self.bind_key(KeyChord::none(KeyCode::Char('d')), UIAction::DeletePodcast);
        self.bind_key(KeyChord::none(KeyCode::Char('r')), UIAction::RefreshPodcast);
        self.bind_key(KeyChord::shift(KeyCode::Char('R')), UIAction::RefreshAll);
        self.bind_key(KeyChord::none(KeyCode::F(5)), UIAction::Refresh);
        self.bind_key(KeyChord::none(KeyCode::Char('r')), UIAction::Refresh);
        self.bind_key(KeyChord::shift(KeyCode::Char('R')), UIAction::RefreshAll);

        // Podcast management
        self.bind_key(KeyChord::none(KeyCode::Char('a')), UIAction::AddPodcast);
        self.bind_key(KeyChord::none(KeyCode::Char('d')), UIAction::DeletePodcast);
    }

    /// Add a prefix key
    pub fn add_prefix_key(&mut self, chord: KeyChord, description: &str) {
        self.prefix_keys.insert(chord, description.to_string());
    }

    /// Bind a single key chord to an action
    pub fn bind_key(&mut self, chord: KeyChord, action: UIAction) {
        self.direct_bindings.insert(chord, action);
    }

    /// Bind a key sequence to an action
    pub fn bind_sequence(&mut self, sequence: KeySequence, action: UIAction) {
        self.prefix_bindings.insert(sequence, action);
    }

    /// Handle a key event and return the corresponding action
    pub fn handle_key(&mut self, key_event: KeyEvent) -> UIAction {
        let chord = KeyChord::from(key_event);

        // Add this chord to the current sequence
        self.current_sequence.push(chord.clone());

        // Check if we have a complete sequence match
        let current_seq = KeySequence::new(self.current_sequence.clone());
        if let Some(action) = self.prefix_bindings.get(&current_seq) {
            // Found a match, clear the sequence and return the action
            self.current_sequence.clear();
            return action.clone();
        }

        // Check if this could be the start of a longer sequence
        let is_potential_prefix = self
            .prefix_bindings
            .keys()
            .any(|seq| seq.chords.starts_with(&self.current_sequence));

        if is_potential_prefix {
            // This could be part of a longer sequence, wait for more keys
            if self.prefix_keys.contains_key(&chord) {
                // Show that we're waiting for the next key
                return UIAction::ShowMinibuffer(format!(
                    "{} ",
                    self.prefix_keys.get(&chord).unwrap()
                ));
            }
            return UIAction::None;
        }

        // No prefix match, check direct bindings for the last key
        if let Some(action) = self.direct_bindings.get(&chord) {
            self.current_sequence.clear();
            return action.clone();
        }

        // No match found, clear sequence and try the key as a direct binding
        self.current_sequence.clear();
        if let Some(action) = self.direct_bindings.get(&chord) {
            return action.clone();
        }

        UIAction::None
    }

    /// Clear the current key sequence (useful for cancellation)
    pub fn clear_sequence(&mut self) {
        self.current_sequence.clear();
    }

    /// Get the current key sequence as a string for display
    pub fn current_sequence_string(&self) -> String {
        if self.current_sequence.is_empty() {
            String::new()
        } else {
            self.current_sequence
                .iter()
                .map(|chord| self.chord_to_string(chord))
                .collect::<Vec<_>>()
                .join(" ")
        }
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
    fn test_prefix_key_sequence() {
        let mut handler = KeyHandler::new();

        // First key of C-x C-c sequence
        let key1 = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let action1 = handler.handle_key(key1);
        assert_eq!(action1, UIAction::ShowMinibuffer("C-x ".to_string()));

        // Second key of C-x C-c sequence
        let key2 = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action2 = handler.handle_key(key2);
        assert_eq!(action2, UIAction::Quit);
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

    #[test]
    fn test_sequence_clearing() {
        let mut handler = KeyHandler::new();

        // Start a prefix sequence
        let key1 = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        handler.handle_key(key1);
        assert!(!handler.current_sequence.is_empty());

        // Clear the sequence
        handler.clear_sequence();
        assert!(handler.current_sequence.is_empty());
    }
}
