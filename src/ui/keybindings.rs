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
        self.bind_key(KeyChord::ctrl(KeyCode::Up), UIAction::MoveEpisodeUp);
        self.bind_key(KeyChord::ctrl(KeyCode::Down), UIAction::MoveEpisodeDown);

        // Vim-style navigation aliases
        self.bind_key(KeyChord::none(KeyCode::Char('j')), UIAction::MoveDown);
        self.bind_key(KeyChord::none(KeyCode::Char('k')), UIAction::MoveUp);
        self.bind_key(KeyChord::none(KeyCode::Char('g')), UIAction::MoveToTop);
        self.bind_key(KeyChord::shift(KeyCode::Char('G')), UIAction::MoveToBottom);

        // Emacs-style navigation aliases (C-n/C-p globally, not just minibuffer)
        self.bind_key(KeyChord::ctrl(KeyCode::Char('n')), UIAction::MoveDown);
        self.bind_key(KeyChord::ctrl(KeyCode::Char('p')), UIAction::MoveUp);

        // Function keys - rarely clash
        self.bind_key(KeyChord::none(KeyCode::F(1)), UIAction::ShowHelp);
        self.bind_key(
            KeyChord::none(KeyCode::F(2)),
            UIAction::SwitchBuffer("podcast-list".to_string()),
        );
        self.bind_key(KeyChord::none(KeyCode::F(3)), UIAction::Search);
        self.bind_key(
            KeyChord::none(KeyCode::F(4)),
            UIAction::SwitchBuffer("downloads".to_string()),
        );
        self.bind_key(KeyChord::none(KeyCode::F(5)), UIAction::Refresh);
        self.bind_key(KeyChord::none(KeyCode::F(7)), UIAction::OpenPlaylistList);
        self.bind_key(
            KeyChord::none(KeyCode::F(8)),
            UIAction::SwitchBuffer("sync".to_string()),
        );
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
        self.bind_key(KeyChord::none(KeyCode::Char('s')), UIAction::SyncToDevice);
        self.bind_key(KeyChord::none(KeyCode::Char('c')), UIAction::CreatePlaylist);
        self.bind_key(KeyChord::none(KeyCode::Char('p')), UIAction::AddToPlaylist);
        self.bind_key(KeyChord::none(KeyCode::Char('r')), UIAction::RefreshPodcast);
        self.bind_key(KeyChord::shift(KeyCode::Char('R')), UIAction::RefreshAll);
        self.bind_key(
            KeyChord::ctrl(KeyCode::Char('r')),
            UIAction::HardRefreshPodcast,
        );
        self.bind_key(
            KeyChord::shift(KeyCode::Char('D')),
            UIAction::DownloadEpisode,
        );
        self.bind_key(
            KeyChord::shift(KeyCode::Char('X')),
            UIAction::DeleteDownloadedEpisode,
        );
        self.bind_key(
            KeyChord::none(KeyCode::Char('X')),
            UIAction::DeleteDownloadedEpisode,
        );
        self.bind_key(
            KeyChord::ctrl(KeyCode::Char('x')),
            UIAction::DeleteAllDownloads,
        );
        self.bind_key(KeyChord::none(KeyCode::Char('q')), UIAction::Quit);
        self.bind_key(KeyChord::none(KeyCode::Char('h')), UIAction::ShowHelp);
        // Bind '?' without modifiers (crossterm handles the shift automatically for the char)
        self.bind_key(KeyChord::none(KeyCode::Char('?')), UIAction::ShowHelp);
        // Also bind with shift modifier in case some terminals report it that way
        self.bind_key(KeyChord::shift(KeyCode::Char('?')), UIAction::ShowHelp);
        // Bind ':' without modifiers (crossterm handles the shift automatically for the char)
        self.bind_key(KeyChord::none(KeyCode::Char(':')), UIAction::PromptCommand);
        // Also bind with shift modifier in case some terminals report it that way
        self.bind_key(KeyChord::shift(KeyCode::Char(':')), UIAction::PromptCommand);

        // Buffer switching (Emacs-style)
        self.bind_key(
            KeyChord::ctrl(KeyCode::Char('b')),
            UIAction::ExecuteCommand("switch-to-buffer".to_string()),
        );

        // List buffers
        self.bind_key(
            KeyChord::ctrl(KeyCode::Char('l')),
            UIAction::ExecuteCommand("list-buffers".to_string()),
        );

        // Close current buffer
        self.bind_key(
            KeyChord::ctrl(KeyCode::Char('k')),
            UIAction::CloseCurrentBuffer,
        );

        // OPML Import/Export
        self.bind_key(KeyChord::shift(KeyCode::Char('A')), UIAction::ImportOpml);
        self.bind_key(KeyChord::shift(KeyCode::Char('E')), UIAction::ExportOpml);

        // Search and filter
        self.bind_key(KeyChord::none(KeyCode::Char('/')), UIAction::Search);
        self.bind_key(KeyChord::none(KeyCode::F(6)), UIAction::ClearFilters);

        // Tab cycling (for dry-run preview mode)
        self.bind_key(KeyChord::none(KeyCode::Char('[')), UIAction::PreviousTab);
        self.bind_key(KeyChord::none(KeyCode::Char(']')), UIAction::NextTab);
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

    #[test]
    fn test_playlist_function_key() {
        let mut handler = KeyHandler::new();

        let key_event = KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        assert_eq!(action, UIAction::OpenPlaylistList);
    }

    #[test]
    fn test_sync_function_key_f8() {
        let mut handler = KeyHandler::new();

        let key_event = KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        assert_eq!(action, UIAction::SwitchBuffer("sync".to_string()));
    }

    #[test]
    fn test_s_key_triggers_sync_to_device() {
        let mut handler = KeyHandler::new();

        let key_event = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        let action = handler.handle_key(key_event);
        assert_eq!(action, UIAction::SyncToDevice);
    }

    #[test]
    fn test_vim_navigation_j_moves_down() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(key), UIAction::MoveDown);
    }

    #[test]
    fn test_vim_navigation_k_moves_up() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(key), UIAction::MoveUp);
    }

    #[test]
    fn test_vim_navigation_g_moves_to_top() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(key), UIAction::MoveToTop);
    }

    #[test]
    fn test_vim_navigation_shift_g_moves_to_bottom() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(handler.handle_key(key), UIAction::MoveToBottom);
    }

    #[test]
    fn test_ctrl_n_moves_down() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key(key), UIAction::MoveDown);
    }

    #[test]
    fn test_ctrl_p_moves_up() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key(key), UIAction::MoveUp);
    }
}
