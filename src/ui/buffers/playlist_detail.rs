use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{
    playlist::{manager::PlaylistManager, Playlist, PlaylistId, PlaylistType},
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
    selected_index: Option<usize>,
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
            selected_index: None,
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
        self.episodes = playlist.episodes;
        if self.episodes.is_empty() {
            self.selected_index = None;
        } else {
            let selected = self.selected_index.unwrap_or(0);
            self.selected_index = Some(selected.min(self.episodes.len() - 1));
        }
    }

    /// Mark this buffer as holding a smart playlist (dynamic filter result).
    pub fn set_smart(&mut self, smart: bool) {
        self.is_smart = smart;
    }

    /// Populate the buffer with the episodes returned by smart playlist evaluation.
    pub fn set_evaluated_episodes(&mut self, episodes: Vec<crate::playlist::PlaylistEpisode>) {
        self.episodes = episodes;
        if self.episodes.is_empty() {
            self.selected_index = None;
        } else {
            let selected = self.selected_index.unwrap_or(0);
            self.selected_index = Some(selected.min(self.episodes.len() - 1));
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

    fn selected_episode(&self) -> Option<&crate::playlist::PlaylistEpisode> {
        self.selected_index
            .and_then(|index| self.episodes.get(index))
    }

    fn select_previous(&mut self) {
        if self.episodes.is_empty() {
            return;
        }
        self.selected_index = match self.selected_index {
            Some(0) => Some(self.episodes.len() - 1),
            Some(index) => Some(index - 1),
            None => Some(0),
        };
    }

    fn select_next(&mut self) {
        if self.episodes.is_empty() {
            return;
        }
        self.selected_index = match self.selected_index {
            Some(index) if index >= self.episodes.len() - 1 => Some(0),
            Some(index) => Some(index + 1),
            None => Some(0),
        };
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
                match self.selected_index {
                    Some(index) if index > 0 => {
                        self.selected_index = Some(index - 1);
                        UIAction::TriggerReorderPlaylist {
                            playlist_id: self.playlist_id.clone(),
                            from_idx: index,
                            to_idx: index - 1,
                        }
                    }
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
                match self.selected_index {
                    Some(index) if index + 1 < self.episodes.len() => {
                        self.selected_index = Some(index + 1);
                        UIAction::TriggerReorderPlaylist {
                            playlist_id: self.playlist_id.clone(),
                            from_idx: index,
                            to_idx: index + 1,
                        }
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

        let items: Vec<ListItem> = if self.episodes.is_empty() {
            vec![ListItem::new("Playlist is empty")]
        } else {
            self.episodes
                .iter()
                .enumerate()
                .map(|(index, episode)| {
                    let selected = self.selected_index == Some(index);
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

        let title = if self.is_smart {
            format!("⚡ Playlist: {}", self.playlist_name)
        } else {
            format!("Playlist: {}", self.playlist_name)
        };
        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_style(self.theme.title_style()),
        );
        frame.render_widget(list, area);
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
