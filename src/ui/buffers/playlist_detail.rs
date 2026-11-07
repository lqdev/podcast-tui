use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{
    playlist::{manager::PlaylistManager, EpisodeKey, Playlist, PlaylistId, PlaylistType},
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};
use std::sync::Arc;

pub struct PlaylistDetailBuffer {
    id: String,
    playlist_id: PlaylistId,
    playlist_name: String,
    playlist_type: PlaylistType,
    episodes: Vec<crate::playlist::PlaylistEpisode>,
    list_state: ListState,
    visible_rows: usize,
    focused: bool,
    theme: Theme,
    playlist_manager: Option<Arc<PlaylistManager>>,
    /// True when this buffer holds a smart (dynamic) playlist
    is_smart: bool,
}

impl PlaylistDetailBuffer {
    pub fn new(
        playlist_id: PlaylistId,
        playlist_name: String,
        playlist_type: PlaylistType,
    ) -> Self {
        let id = format!(
            "playlist-{}",
            playlist_name.replace(' ', "-").to_lowercase()
        );
        Self {
            id,
            playlist_id,
            playlist_name,
            playlist_type,
            episodes: Vec::new(),
            list_state: ListState::default(),
            visible_rows: 0,
            focused: false,
            theme: Theme::default(),
            playlist_manager: None,
            is_smart: false,
        }
    }

    pub fn set_playlist_manager(&mut self, manager: Arc<PlaylistManager>) {
        self.playlist_manager = Some(manager);
    }

    pub fn set_playlist(&mut self, playlist: Playlist) {
        self.playlist_type = playlist.playlist_type;
        self.is_smart = playlist.smart_rules.is_some();
        let selected_key = self
            .selected_episode()
            .map(|episode| EpisodeKey::new(episode.podcast_id.clone(), episode.episode_id.clone()));
        let previous_index = self.list_state.selected().unwrap_or(0);
        self.episodes = playlist.episodes;
        self.restore_selected_episode(selected_key, previous_index);
    }

    /// Mark this buffer as holding a smart playlist (dynamic filter result).
    pub fn set_smart(&mut self, smart: bool) {
        self.is_smart = smart;
    }

    /// Populate the buffer with the episodes returned by smart playlist evaluation.
    pub fn set_evaluated_episodes(&mut self, episodes: Vec<crate::playlist::PlaylistEpisode>) {
        let selected_key = self
            .selected_episode()
            .map(|episode| EpisodeKey::new(episode.podcast_id.clone(), episode.episode_id.clone()));
        let previous_index = self.list_state.selected().unwrap_or(0);
        self.episodes = episodes;
        self.restore_selected_episode(selected_key, previous_index);
    }

    fn restore_selected_episode(
        &mut self,
        selected_key: Option<EpisodeKey>,
        previous_index: usize,
    ) {
        if self.episodes.is_empty() {
            self.list_state.select(None);
            self.visible_rows = 0;
        } else {
            let selected = selected_key
                .and_then(|key| {
                    self.episodes.iter().position(|episode| {
                        episode.podcast_id == key.podcast_id && episode.episode_id == key.episode_id
                    })
                })
                .unwrap_or_else(|| previous_index.min(self.episodes.len() - 1));
            self.list_state.select(Some(selected));
            *self.list_state.offset_mut() = self
                .list_state
                .offset()
                .min(self.episodes.len().saturating_sub(1));
        }
    }

    pub fn playlist_id(&self) -> &PlaylistId {
        &self.playlist_id
    }

    pub async fn load_playlist(&mut self) -> Result<(), String> {
        if let Some(manager) = &self.playlist_manager {
            let playlist = manager
                .get_playlist(&self.playlist_id)
                .await
                .map_err(|e| e.to_string())?;
            self.set_playlist(playlist);
        }
        Ok(())
    }

    pub fn selected_episode(&self) -> Option<&crate::playlist::PlaylistEpisode> {
        self.list_state
            .selected()
            .and_then(|index| self.episodes.get(index))
    }

    pub fn select_episode(&mut self, key: &EpisodeKey) -> bool {
        let Some(index) = self.episodes.iter().position(|episode| {
            episode.podcast_id == key.podcast_id && episode.episode_id == key.episode_id
        }) else {
            return false;
        };
        self.list_state.select(Some(index));
        true
    }

    fn select_previous(&mut self) {
        if self.episodes.is_empty() {
            return;
        }
        let selected = match self.list_state.selected() {
            Some(0) => Some(self.episodes.len() - 1),
            Some(index) => Some(index - 1),
            None => Some(0),
        };
        self.list_state.select(selected);
    }

    fn select_next(&mut self) {
        if self.episodes.is_empty() {
            return;
        }
        let selected = match self.list_state.selected() {
            Some(index) if index >= self.episodes.len() - 1 => Some(0),
            Some(index) => Some(index + 1),
            None => Some(0),
        };
        self.list_state.select(selected);
    }

    fn select_page(&mut self, down: bool) {
        let len = self.episodes.len();
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

    fn reorder_selected(&mut self, to_index: usize) -> UIAction {
        let Some(from_index) = self.list_state.selected() else {
            return UIAction::None;
        };
        if to_index >= self.episodes.len() || from_index == to_index {
            return UIAction::None;
        }

        self.episodes.swap(from_index, to_index);
        for (index, episode) in self.episodes.iter_mut().enumerate() {
            episode.order = index + 1;
        }
        self.list_state.select(Some(to_index));
        UIAction::TriggerReorderPlaylist {
            playlist_id: self.playlist_id.clone(),
            from_idx: from_index,
            to_idx: to_index,
        }
    }
}

impl Buffer for PlaylistDetailBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        if self.is_smart {
            format!("⚡ Playlist: {}", self.playlist_name)
        } else {
            format!("Playlist: {}", self.playlist_name)
        }
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
            super::KeyHint::new("X", "remove"),
            super::KeyHint::new("C-↑/↓", "reorder"),
            super::KeyHint::new("m", "played"),
        ]
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Playlist Detail Commands:".to_string(),
            "  ↑/↓      Navigate episodes".to_string(),
            "  Enter    View episode details".to_string(),
            "  X        Remove episode from playlist".to_string(),
            "  Ctrl+↑   Move episode up".to_string(),
            "  Ctrl+↓   Move episode down".to_string(),
            "  r        Refresh Today / rebuild user playlist files".to_string(),
        ]
    }
}

impl UIComponent for PlaylistDetailBuffer {
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
                self.list_state
                    .select((!self.episodes.is_empty()).then_some(0));
                *self.list_state.offset_mut() = 0;
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                self.list_state.select(
                    (!self.episodes.is_empty()).then_some(self.episodes.len().saturating_sub(1)),
                );
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some(episode) = self.selected_episode() {
                    UIAction::OpenEpisodeDetailById {
                        podcast_id: episode.podcast_id.clone(),
                        episode_id: episode.episode_id.clone(),
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            UIAction::DeleteDownloadedEpisode => {
                if matches!(self.playlist_type, PlaylistType::AutoGenerated { .. }) {
                    return UIAction::ShowMessage(
                        "Auto-generated playlists are managed automatically".to_string(),
                    );
                }
                if self.is_smart {
                    return UIAction::ShowMessage(
                        "Smart playlists are managed automatically".to_string(),
                    );
                }

                if let Some(episode) = self.selected_episode() {
                    UIAction::TriggerRemoveFromPlaylist {
                        playlist_id: self.playlist_id.clone(),
                        episode_id: episode.episode_id.clone(),
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            UIAction::MoveEpisodeUp => {
                if matches!(self.playlist_type, PlaylistType::AutoGenerated { .. }) {
                    return UIAction::ShowMessage(
                        "Auto-generated playlists cannot be reordered".to_string(),
                    );
                }
                if self.is_smart {
                    return UIAction::ShowMessage(
                        "Smart playlists cannot be reordered".to_string(),
                    );
                }
                match self.list_state.selected() {
                    Some(index) if index > 0 => self.reorder_selected(index - 1),
                    _ => UIAction::None,
                }
            }
            UIAction::MoveEpisodeDown => {
                if matches!(self.playlist_type, PlaylistType::AutoGenerated { .. }) {
                    return UIAction::ShowMessage(
                        "Auto-generated playlists cannot be reordered".to_string(),
                    );
                }
                if self.is_smart {
                    return UIAction::ShowMessage(
                        "Smart playlists cannot be reordered".to_string(),
                    );
                }
                match self.list_state.selected() {
                    Some(index) if index + 1 < self.episodes.len() => {
                        self.reorder_selected(index + 1)
                    }
                    _ => UIAction::None,
                }
            }
            UIAction::Refresh | UIAction::RefreshPodcast => {
                if matches!(self.playlist_type, PlaylistType::AutoGenerated { .. }) {
                    UIAction::RefreshAutoPlaylists
                } else {
                    UIAction::RebuildPlaylistFiles {
                        playlist_id: self.playlist_id.clone(),
                    }
                }
            }
            UIAction::MarkPlayed => {
                if let Some(episode) = self.selected_episode() {
                    let title = episode
                        .episode_title
                        .clone()
                        .or_else(|| episode.filename.clone())
                        .unwrap_or_else(|| episode.episode_id.to_string());
                    UIAction::TriggerMarkPlayed {
                        podcast_id: episode.podcast_id.clone(),
                        episode_id: episode.episode_id.clone(),
                        episode_title: title,
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            UIAction::MarkUnplayed => {
                if let Some(episode) = self.selected_episode() {
                    let title = episode
                        .episode_title
                        .clone()
                        .or_else(|| episode.filename.clone())
                        .unwrap_or_else(|| episode.episode_id.to_string());
                    UIAction::TriggerMarkUnplayed {
                        podcast_id: episode.podcast_id.clone(),
                        episode_id: episode.episode_id.clone(),
                        episode_title: title,
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let title = if self.is_smart {
            format!("⚡ Playlist: {}", self.playlist_name)
        } else {
            format!("Playlist: {}", self.playlist_name)
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style)
            .title_style(self.theme.title_style());
        let inner = block.inner(area);
        self.visible_rows = inner.height as usize;

        if inner.width == 0 || inner.height == 0 {
            frame.render_widget(block, area);
            return;
        }

        let items: Vec<ListItem> = if self.episodes.is_empty() {
            self.list_state.select(None);
            vec![ListItem::new("Playlist is empty")]
        } else {
            self.episodes
                .iter()
                .enumerate()
                .map(|(index, episode)| {
                    let selected = self.list_state.selected() == Some(index);
                    let marker = if selected { "► " } else { "  " };
                    let display_name = episode
                        .episode_title
                        .clone()
                        .or_else(|| episode.filename.clone())
                        .unwrap_or_else(|| episode.episode_id.to_string());
                    let text = format!("{marker}{:03} {}", episode.order, display_name);
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
        format!("Playlist: {}", self.playlist_name)
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
    use crate::storage::{EpisodeId, PodcastId};
    use chrono::Utc;

    fn make_episode(index: usize) -> crate::playlist::PlaylistEpisode {
        crate::playlist::PlaylistEpisode {
            podcast_id: PodcastId::new(),
            episode_id: EpisodeId::new(),
            episode_title: Some(format!("Episode {index:02}")),
            added_at: Utc::now(),
            order: index + 1,
            file_synced: false,
            filename: None,
        }
    }

    #[test]
    fn test_playlist_detail_scrolls_selected_row_into_view() {
        let playlist_id = PlaylistId::new();
        let mut buffer = PlaylistDetailBuffer::new(
            playlist_id.clone(),
            "Long Playlist".to_string(),
            PlaylistType::User,
        );
        let episodes = (0..12).map(make_episode).collect::<Vec<_>>();
        let selected_episode = episodes[6].clone();
        buffer.set_playlist(Playlist {
            id: playlist_id.clone(),
            name: "Long Playlist".to_string(),
            description: None,
            playlist_type: PlaylistType::User,
            episodes,
            created: Utc::now(),
            last_updated: Utc::now(),
            smart_rules: None,
        });
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
        assert!(screen.contains("Episode 06"));
        assert!(!screen.contains("Episode 00"));
        assert_eq!(
            buffer.selected_episode().map(|episode| &episode.episode_id),
            Some(&selected_episode.episode_id)
        );
    }

    #[test]
    fn test_select_item_opens_episode_detail_by_id() {
        let playlist_id = PlaylistId::new();
        let mut buffer = PlaylistDetailBuffer::new(
            playlist_id.clone(),
            "Test Playlist".to_string(),
            PlaylistType::User,
        );
        let podcast_id = PodcastId::new();
        let episode_id = EpisodeId::new();
        buffer.set_playlist(Playlist {
            id: playlist_id,
            name: "Test Playlist".to_string(),
            description: None,
            playlist_type: PlaylistType::User,
            episodes: vec![crate::playlist::PlaylistEpisode {
                podcast_id: podcast_id.clone(),
                episode_id: episode_id.clone(),
                episode_title: Some("Episode 1".to_string()),
                added_at: Utc::now(),
                order: 1,
                file_synced: false,
                filename: None,
            }],
            created: Utc::now(),
            last_updated: Utc::now(),
            smart_rules: None,
        });

        let action = buffer.handle_action(UIAction::SelectItem);
        match action {
            UIAction::OpenEpisodeDetailById {
                podcast_id: actual_podcast_id,
                episode_id: actual_episode_id,
            } => {
                assert_eq!(actual_podcast_id, podcast_id);
                assert_eq!(actual_episode_id, episode_id);
            }
            _ => panic!("Expected OpenEpisodeDetailById action"),
        }
    }

    #[test]
    fn test_select_item_without_selection_shows_message() {
        let playlist_id = PlaylistId::new();
        let mut buffer = PlaylistDetailBuffer::new(
            playlist_id.clone(),
            "Empty Playlist".to_string(),
            PlaylistType::User,
        );
        buffer.set_playlist(Playlist {
            id: playlist_id,
            name: "Empty Playlist".to_string(),
            description: None,
            playlist_type: PlaylistType::User,
            episodes: Vec::new(),
            created: Utc::now(),
            last_updated: Utc::now(),
            smart_rules: None,
        });

        let action = buffer.handle_action(UIAction::SelectItem);
        assert_eq!(
            action,
            UIAction::ShowMessage("No episode selected".to_string())
        );
    }
}
