use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{
    playlist::{manager::PlaylistManager, Playlist, PlaylistType},
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};
use std::sync::Arc;

pub struct PlaylistListBuffer {
    id: String,
    playlists: Vec<Playlist>,
    list_state: ListState,
    visible_rows: usize,
    focused: bool,
    theme: Theme,
    playlist_manager: Option<Arc<PlaylistManager>>,
}

impl Default for PlaylistListBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistListBuffer {
    pub fn new() -> Self {
        Self {
            id: "playlist-list".to_string(),
            playlists: Vec::new(),
            list_state: ListState::default(),
            visible_rows: 0,
            focused: false,
            theme: Theme::default(),
            playlist_manager: None,
        }
    }

    pub fn set_playlist_manager(&mut self, manager: Arc<PlaylistManager>) {
        self.playlist_manager = Some(manager);
    }

    pub fn set_playlists(&mut self, playlists: Vec<Playlist>) {
        let selected_id = self
            .list_state
            .selected()
            .and_then(|index| self.playlists.get(index))
            .map(|playlist| playlist.id.clone());
        let previous_index = self.list_state.selected().unwrap_or(0);
        self.playlists = playlists;
        if self.playlists.is_empty() {
            self.list_state.select(None);
            self.visible_rows = 0;
        } else {
            let selected_index = selected_id
                .and_then(|id| self.playlists.iter().position(|playlist| playlist.id == id))
                .unwrap_or_else(|| previous_index.min(self.playlists.len() - 1));
            self.list_state.select(Some(selected_index));
            *self.list_state.offset_mut() = self
                .list_state
                .offset()
                .min(self.playlists.len().saturating_sub(1));
        }
    }

    pub async fn load_playlists(&mut self) -> Result<(), String> {
        if let Some(manager) = &self.playlist_manager {
            let playlists = manager.list_playlists().await.map_err(|e| e.to_string())?;
            self.set_playlists(playlists);
        }
        Ok(())
    }

    pub fn selected_playlist(&self) -> Option<&Playlist> {
        self.list_state
            .selected()
            .and_then(|index| self.playlists.get(index))
    }

    fn select_previous(&mut self) {
        if self.playlists.is_empty() {
            return;
        }

        let selected = match self.list_state.selected() {
            Some(0) => Some(self.playlists.len() - 1),
            Some(index) => Some(index - 1),
            None => Some(0),
        };
        self.list_state.select(selected);
    }

    fn select_next(&mut self) {
        if self.playlists.is_empty() {
            return;
        }

        let selected = match self.list_state.selected() {
            Some(index) if index >= self.playlists.len() - 1 => Some(0),
            Some(index) => Some(index + 1),
            None => Some(0),
        };
        self.list_state.select(selected);
    }

    fn select_page(&mut self, down: bool) {
        let len = self.playlists.len();
        if len == 0 || self.visible_rows == 0 {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let next = if down {
            selected.saturating_add(self.visible_rows).min(len - 1)
        } else {
            selected.saturating_sub(self.visible_rows)
        };
        self.list_state.select(Some(next));
    }
}

impl Buffer for PlaylistListBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Playlists".to_string()
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

    fn key_hints(&self) -> Vec<super::KeyHint> {
        vec![
            super::KeyHint::new("Enter", "open"),
            super::KeyHint::new("c", "create"),
            super::KeyHint::new("d", "delete"),
            super::KeyHint::new("r", "refresh"),
        ]
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Playlist Commands:".to_string(),
            "  ↑/↓      Navigate playlists".to_string(),
            "  Enter    Open playlist".to_string(),
            "  c        Create playlist".to_string(),
            "  d        Delete playlist".to_string(),
            "  r        Refresh Today playlist".to_string(),
            "  F7       Open playlist list".to_string(),
        ]
    }
}

impl UIComponent for PlaylistListBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.select_previous();
                UIAction::Render
            }
            UIAction::MoveDown => {
                self.select_next();
                UIAction::Render
            }
            UIAction::PageUp => {
                self.select_page(false);
                UIAction::Render
            }
            UIAction::PageDown => {
                self.select_page(true);
                UIAction::Render
            }
            UIAction::MoveToTop => {
                if !self.playlists.is_empty() {
                    self.list_state.select(Some(0));
                    *self.list_state.offset_mut() = 0;
                }
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                if !self.playlists.is_empty() {
                    self.list_state.select(Some(self.playlists.len() - 1));
                }
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(playlist) = self.selected_playlist() {
                    UIAction::OpenPlaylistDetail {
                        playlist_id: playlist.id.clone(),
                        playlist_name: playlist.name.clone(),
                    }
                } else {
                    UIAction::ShowMessage("No playlist selected".to_string())
                }
            }
            UIAction::CreatePlaylist => UIAction::PromptInput("Create playlist: ".to_string()),
            UIAction::DeletePlaylist => {
                if let Some(playlist) = self.selected_playlist() {
                    UIAction::TriggerDeletePlaylist {
                        playlist_id: playlist.id.clone(),
                    }
                } else {
                    UIAction::ShowMessage("No playlist selected".to_string())
                }
            }
            UIAction::Refresh | UIAction::RefreshPodcast => UIAction::RefreshAutoPlaylists,
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let block = Block::default()
            .title("Playlists")
            .borders(Borders::ALL)
            .border_style(border_style)
            .title_style(self.theme.title_style());
        let inner = block.inner(area);
        self.visible_rows = inner.height as usize;

        if inner.width == 0 || inner.height == 0 {
            frame.render_widget(block, area);
            return;
        }

        let items: Vec<ListItem> = if self.playlists.is_empty() {
            self.list_state.select(None);
            vec![ListItem::new("No playlists yet")]
        } else {
            self.playlists
                .iter()
                .enumerate()
                .map(|(index, playlist)| {
                    let selected = self.list_state.selected() == Some(index);
                    let playlist_type = if playlist.is_smart() {
                        "Smart"
                    } else {
                        match playlist.playlist_type {
                            PlaylistType::User => "User",
                            PlaylistType::AutoGenerated { .. } => "Auto",
                        }
                    };
                    let name = if playlist.is_smart() {
                        format!("⚡ {}", playlist.name)
                    } else {
                        playlist.name.clone()
                    };
                    let marker = if selected { "► " } else { "  " };
                    let text = format!(
                        "{}{} [{}] ({} episode{})",
                        marker,
                        name,
                        playlist_type,
                        playlist.episodes.len(),
                        if playlist.episodes.len() == 1 {
                            ""
                        } else {
                            "s"
                        }
                    );
                    if selected {
                        ListItem::new(text).style(self.theme.selected_style())
                    } else {
                        ListItem::new(text).style(self.theme.text_style())
                    }
                })
                .collect()
        };

        let list = List::new(items).block(block);

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn title(&self) -> String {
        "Playlists".to_string()
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::{PlaylistEpisode, PlaylistId};
    use chrono::Utc;

    fn make_playlist(name: &str) -> Playlist {
        Playlist {
            id: PlaylistId::new(),
            name: name.to_string(),
            description: None,
            playlist_type: PlaylistType::User,
            episodes: vec![PlaylistEpisode {
                podcast_id: crate::storage::PodcastId::new(),
                episode_id: crate::storage::EpisodeId::new(),
                episode_title: None,
                added_at: Utc::now(),
                order: 1,
                file_synced: false,
                filename: None,
            }],
            created: Utc::now(),
            last_updated: Utc::now(),
            smart_rules: None,
        }
    }

    #[test]
    fn test_playlist_list_scrolls_selected_row_into_view() {
        let mut buffer = PlaylistListBuffer::new();
        let playlists = (0..12)
            .map(|index| make_playlist(&format!("Playlist {index:02}")))
            .collect::<Vec<_>>();
        let selected_id = playlists[6].id.clone();
        buffer.set_playlists(playlists);
        for _ in 0..6 {
            buffer.handle_action(UIAction::MoveDown);
        }

        let backend = ratatui::backend::TestBackend::new(50, 6);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| buffer.render(frame, frame.area()))
            .unwrap();

        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("Playlist 06"));
        assert!(!screen.contains("Playlist 00"));
        assert_eq!(
            buffer.selected_playlist().map(|playlist| &playlist.id),
            Some(&selected_id)
        );
    }

    #[test]
    fn test_playlist_list_page_navigation_uses_visible_height() {
        let mut buffer = PlaylistListBuffer::new();
        buffer.set_playlists(
            (0..12)
                .map(|index| make_playlist(&format!("Playlist {index:02}")))
                .collect(),
        );
        let backend = ratatui::backend::TestBackend::new(50, 6);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| buffer.render(frame, frame.area()))
            .unwrap();

        buffer.handle_action(UIAction::PageDown);
        assert_eq!(buffer.list_state.selected(), Some(4));
        buffer.handle_action(UIAction::MoveToBottom);
        assert_eq!(buffer.list_state.selected(), Some(11));
        buffer.handle_action(UIAction::PageDown);
        assert_eq!(buffer.list_state.selected(), Some(11));
    }

    #[test]
    fn test_r_and_f5_both_refresh_auto_playlists() {
        // `r` (RefreshPodcast) and F5 (Refresh) must both refresh auto-playlists.
        // The hint bar advertises `r` to stay buffer-specific and match the buffer
        // help text ("r  Refresh Today playlist").
        let mut buffer = PlaylistListBuffer::new();
        assert_eq!(
            buffer.handle_action(UIAction::RefreshPodcast),
            UIAction::RefreshAutoPlaylists
        );
        assert_eq!(
            buffer.handle_action(UIAction::Refresh),
            UIAction::RefreshAutoPlaylists
        );
    }

    #[test]
    fn test_key_hints_use_buffer_specific_refresh_key() {
        let buffer = PlaylistListBuffer::new();
        let hints = buffer.key_hints();
        assert!(hints.iter().any(|h| h.key == "r"));
        assert!(!hints.iter().any(|h| h.key == "F5"));
    }
}
