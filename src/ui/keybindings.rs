// Simple, clash-free keybinding system
//
// This module provides basic keybindings that work in most environments,
// including VS Code terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::config::{GlobalKeys, KeybindingConfig};
use crate::ui::key_parser::parse_key_notation;
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

        // Episode status
        self.bind_key(KeyChord::none(KeyCode::Char('m')), UIAction::MarkPlayed);
        self.bind_key(KeyChord::none(KeyCode::Char('u')), UIAction::MarkUnplayed);

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

    /// Look up the action bound to a key chord, if any.
    pub fn lookup(&self, chord: &KeyChord) -> Option<&UIAction> {
        self.bindings.get(chord)
    }

    /// Build a `KeyHandler` from a `KeybindingConfig`.
    ///
    /// Starts with all default bindings, then applies any non-empty override lists from
    /// `config`. An empty `Vec<String>` for any field means "keep the default binding".
    /// If at least one notation in a non-empty list is valid, the defaults for that action
    /// are replaced with the parsed chords. If all notations in a non-empty list are
    /// invalid, the action keeps its default binding (the list is treated as a no-op).
    pub fn from_config(config: &KeybindingConfig) -> Self {
        let mut handler = Self::new();
        handler.apply_global_overrides(&config.global);
        handler
    }

    /// Parse a slice of notation strings into `KeyChord`s, silently dropping invalid entries.
    fn parse_notations(notations: &[String]) -> Vec<KeyChord> {
        notations
            .iter()
            .filter_map(|s| parse_key_notation(s).ok())
            .collect()
    }

    /// Remove all existing bindings for `action` and replace with `chords`.
    fn rebind_action(&mut self, action: UIAction, chords: Vec<KeyChord>) {
        self.bindings.retain(|_, v| *v != action);
        for chord in chords {
            self.bindings.insert(chord, action.clone());
        }
    }

    /// If `notations` is non-empty, replace all bindings for `action` with parsed chords.
    /// An empty slice is a no-op (preserves the defaults set by `new()`).
    /// If all notations are invalid (parse to no chords), this is also a no-op to avoid
    /// clearing the existing default bindings.
    fn override_binding(&mut self, notations: &[String], action: UIAction) {
        if notations.is_empty() {
            return;
        }
        let chords = Self::parse_notations(notations);
        if chords.is_empty() {
            // All provided notations were invalid — keep existing default bindings.
            return;
        }
        self.rebind_action(action, chords);
    }

    /// Apply non-empty override lists from `GlobalKeys` to the current bindings.
    fn apply_global_overrides(&mut self, keys: &GlobalKeys) {
        // Navigation
        self.override_binding(&keys.move_up, UIAction::MoveUp);
        self.override_binding(&keys.move_down, UIAction::MoveDown);
        self.override_binding(&keys.move_left, UIAction::MoveLeft);
        self.override_binding(&keys.move_right, UIAction::MoveRight);
        self.override_binding(&keys.page_up, UIAction::PageUp);
        self.override_binding(&keys.page_down, UIAction::PageDown);
        self.override_binding(&keys.move_to_top, UIAction::MoveToTop);
        self.override_binding(&keys.move_to_bottom, UIAction::MoveToBottom);
        self.override_binding(&keys.move_episode_up, UIAction::MoveEpisodeUp);
        self.override_binding(&keys.move_episode_down, UIAction::MoveEpisodeDown);

        // Buffer navigation
        self.override_binding(&keys.next_buffer, UIAction::NextBuffer);
        self.override_binding(&keys.prev_buffer, UIAction::PreviousBuffer);
        self.override_binding(&keys.close_buffer, UIAction::CloseCurrentBuffer);
        self.override_binding(
            &keys.open_podcast_list,
            UIAction::SwitchBuffer("podcast-list".to_string()),
        );
        self.override_binding(
            &keys.open_downloads,
            UIAction::SwitchBuffer("downloads".to_string()),
        );
        self.override_binding(&keys.open_playlists, UIAction::OpenPlaylistList);
        self.override_binding(&keys.open_sync, UIAction::SwitchBuffer("sync".to_string()));

        // Application control
        self.override_binding(&keys.quit, UIAction::Quit);
        self.override_binding(&keys.show_help, UIAction::ShowHelp);
        self.override_binding(&keys.search, UIAction::Search);
        self.override_binding(&keys.clear_filters, UIAction::ClearFilters);
        self.override_binding(&keys.refresh, UIAction::Refresh);
        self.override_binding(&keys.prompt_command, UIAction::PromptCommand);
        self.override_binding(
            &keys.switch_to_buffer,
            UIAction::ExecuteCommand("switch-to-buffer".to_string()),
        );
        self.override_binding(
            &keys.list_buffers,
            UIAction::ExecuteCommand("list-buffers".to_string()),
        );

        // Interaction
        self.override_binding(&keys.select, UIAction::SelectItem);
        self.override_binding(&keys.cancel, UIAction::HideMinibuffer);

        // Podcast management
        self.override_binding(&keys.add_podcast, UIAction::AddPodcast);
        self.override_binding(&keys.delete_podcast, UIAction::DeletePodcast);
        self.override_binding(&keys.refresh_podcast, UIAction::RefreshPodcast);
        self.override_binding(&keys.refresh_all, UIAction::RefreshAll);
        self.override_binding(&keys.hard_refresh_podcast, UIAction::HardRefreshPodcast);

        // Episode actions
        self.override_binding(&keys.download_episode, UIAction::DownloadEpisode);
        self.override_binding(
            &keys.delete_downloaded_episode,
            UIAction::DeleteDownloadedEpisode,
        );
        self.override_binding(&keys.delete_all_downloads, UIAction::DeleteAllDownloads);
        self.override_binding(&keys.mark_played, UIAction::MarkPlayed);
        self.override_binding(&keys.mark_unplayed, UIAction::MarkUnplayed);

        // Playlist
        self.override_binding(&keys.create_playlist, UIAction::CreatePlaylist);
        self.override_binding(&keys.add_to_playlist, UIAction::AddToPlaylist);

        // OPML
        self.override_binding(&keys.import_opml, UIAction::ImportOpml);
        self.override_binding(&keys.export_opml, UIAction::ExportOpml);

        // Sync
        self.override_binding(&keys.sync_to_device, UIAction::SyncToDevice);

        // Tab navigation
        self.override_binding(&keys.prev_tab, UIAction::PreviousTab);
        self.override_binding(&keys.next_tab, UIAction::NextTab);
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
    use crate::config::KeybindingConfig;

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

    #[test]
    fn test_m_key_resolves_to_mark_played() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(key), UIAction::MarkPlayed);
    }

    #[test]
    fn test_u_key_resolves_to_mark_unplayed() {
        let mut handler = KeyHandler::new();
        let key = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key(key), UIAction::MarkUnplayed);
    }

    // ── from_config tests ────────────────────────────────────────────────────

    #[test]
    fn test_from_config_defaults_produce_same_bindings_as_new() {
        // Arrange
        let config = KeybindingConfig::default();

        // Act
        let from_conf = KeyHandler::from_config(&config);
        let from_new = KeyHandler::new();

        // Assert — spot-check that default config reproduces the same bindings
        let cases: &[(KeyChord, UIAction)] = &[
            (KeyChord::none(KeyCode::Char('q')), UIAction::Quit),
            (KeyChord::none(KeyCode::F(10)), UIAction::Quit),
            (KeyChord::none(KeyCode::F(1)), UIAction::ShowHelp),
            (KeyChord::none(KeyCode::Up), UIAction::MoveUp),
            (KeyChord::none(KeyCode::Char('j')), UIAction::MoveDown),
            (KeyChord::ctrl(KeyCode::Char('n')), UIAction::MoveDown),
            (KeyChord::shift(KeyCode::BackTab), UIAction::PreviousBuffer),
        ];
        for (chord, expected) in cases {
            assert_eq!(
                from_conf.lookup(chord),
                from_new.lookup(chord),
                "chord {:?} should match between from_config(default) and new()",
                chord
            );
            assert_eq!(from_conf.lookup(chord), Some(expected));
        }
    }

    #[test]
    fn test_from_config_overrides_action_removes_old_bindings() {
        // Arrange — remap quit to C-q only (replaces default q and F10)
        let mut config = KeybindingConfig::default();
        config.global.quit = vec!["C-q".to_string()];

        // Act
        let handler = KeyHandler::from_config(&config);

        // Assert — new chord works
        assert_eq!(
            handler.lookup(&KeyChord::ctrl(KeyCode::Char('q'))),
            Some(&UIAction::Quit)
        );
        // Old defaults are removed
        assert_eq!(handler.lookup(&KeyChord::none(KeyCode::Char('q'))), None);
        assert_eq!(handler.lookup(&KeyChord::none(KeyCode::F(10))), None);
    }

    #[test]
    fn test_from_config_empty_vec_preserves_defaults() {
        // Arrange — empty Vec means "keep the defaults"
        let mut config = KeybindingConfig::default();
        config.global.quit = vec![];

        // Act
        let handler = KeyHandler::from_config(&config);

        // Assert — both default quit chords still work
        assert_eq!(
            handler.lookup(&KeyChord::none(KeyCode::Char('q'))),
            Some(&UIAction::Quit)
        );
        assert_eq!(
            handler.lookup(&KeyChord::none(KeyCode::F(10))),
            Some(&UIAction::Quit)
        );
    }

    #[test]
    fn test_from_config_invalid_notation_skips_gracefully() {
        // Arrange — mix of valid and invalid notations; must not panic
        let mut config = KeybindingConfig::default();
        config.global.quit = vec!["C-q".to_string(), "NOT-VALID-!!!".to_string()];

        // Act
        let handler = KeyHandler::from_config(&config);

        // Assert — valid notation is bound; invalid one is silently skipped
        assert_eq!(
            handler.lookup(&KeyChord::ctrl(KeyCode::Char('q'))),
            Some(&UIAction::Quit)
        );
    }

    #[test]
    fn test_from_config_all_invalid_notations_preserves_defaults() {
        // Arrange — every notation in the list is invalid; must not clear defaults
        let mut config = KeybindingConfig::default();
        config.global.quit = vec!["NOT-VALID".to_string(), "ALSO-BAD".to_string()];

        // Act — must not panic, and must not clear existing default bindings
        let handler = KeyHandler::from_config(&config);

        // Assert — default quit chords preserved (all-invalid list is a no-op)
        assert_eq!(
            handler.lookup(&KeyChord::none(KeyCode::Char('q'))),
            Some(&UIAction::Quit)
        );
        assert_eq!(
            handler.lookup(&KeyChord::none(KeyCode::F(10))),
            Some(&UIAction::Quit)
        );
    }

    #[test]
    fn test_from_config_multiple_chords_all_bound() {
        // Arrange — configure three chords for quit
        let mut config = KeybindingConfig::default();
        config.global.quit = vec!["q".to_string(), "C-q".to_string(), "F10".to_string()];

        // Act
        let handler = KeyHandler::from_config(&config);

        // Assert — all three resolve to Quit
        assert_eq!(
            handler.lookup(&KeyChord::none(KeyCode::Char('q'))),
            Some(&UIAction::Quit)
        );
        assert_eq!(
            handler.lookup(&KeyChord::ctrl(KeyCode::Char('q'))),
            Some(&UIAction::Quit)
        );
        assert_eq!(
            handler.lookup(&KeyChord::none(KeyCode::F(10))),
            Some(&UIAction::Quit)
        );
    }
}
