use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{
    playlist::PlaylistId,
    storage::{EpisodeId, PodcastId},
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

#[derive(Clone, Debug)]
enum PickerMode {
    AddEpisode {
        podcast_id: PodcastId,
        episode_id: EpisodeId,
    },
    EpisodeMembership,
}

pub struct PlaylistPickerBuffer {
    id: String,
    focused: bool,
    theme: Theme,
    playlists: Vec<(PlaylistId, String, usize)>,
    list_state: ListState,
    visible_rows: usize,
    mode: PickerMode,
    origin_buffer_id: BufferId,
    membership_stale: bool,
    membership_error: Option<String>,
}

impl PlaylistPickerBuffer {
    pub fn new(
        playlists: Vec<(PlaylistId, String, usize)>,
        target_podcast_id: PodcastId,
        target_episode_id: EpisodeId,
        origin_buffer_id: BufferId,
    ) -> Self {
        Self::with_mode(
            playlists,
            PickerMode::AddEpisode {
                podcast_id: target_podcast_id,
                episode_id: target_episode_id,
            },
            origin_buffer_id,
            false,
            None,
        )
    }

    pub fn new_episode_membership(
        playlists: Vec<(PlaylistId, String, usize)>,
        origin_buffer_id: BufferId,
        membership_stale: bool,
        membership_error: Option<String>,
    ) -> Self {
        Self::with_mode(
            playlists,
            PickerMode::EpisodeMembership,
            origin_buffer_id,
            membership_stale,
            membership_error,
        )
    }

    fn with_mode(
        playlists: Vec<(PlaylistId, String, usize)>,
        mode: PickerMode,
        origin_buffer_id: BufferId,
        membership_stale: bool,
        membership_error: Option<String>,
    ) -> Self {
        Self {
            id: "playlist-picker".to_string(),
            focused: false,
            theme: Theme::default(),
            playlists,
            list_state: ListState::default().with_selected(Some(0)),
            visible_rows: 0,
            mode,
            origin_buffer_id,
            membership_stale,
            membership_error,
        }
    }

    pub fn selected_playlist(&self) -> Option<&(PlaylistId, String, usize)> {
        self.list_state
            .selected()
            .and_then(|index| self.playlists.get(index))
    }

    pub fn origin_buffer_id(&self) -> &BufferId {
        &self.origin_buffer_id
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

impl Buffer for PlaylistPickerBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        match &self.mode {
            PickerMode::AddEpisode { .. } => "Playlist Picker".to_string(),
            PickerMode::EpisodeMembership => "Episode Playlists".to_string(),
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

    fn help_text(&self) -> Vec<String> {
        match &self.mode {
            PickerMode::AddEpisode { .. } => vec![
                "Playlist Picker Commands:".to_string(),
                "  ↑/↓      Navigate playlists".to_string(),
                "  Enter    Add episode to selected playlist".to_string(),
                "  Esc      Cancel".to_string(),
            ],
            PickerMode::EpisodeMembership => {
                let mut help = vec![
                    "Episode Playlists Commands:".to_string(),
                    "  ↑/↓      Navigate playlists".to_string(),
                    "  Enter    Open selected playlist".to_string(),
                    "  Esc      Return to episode".to_string(),
                ];
                if self.membership_stale {
                    help.push("  !        Membership data may be stale".to_string());
                }
                help
            }
        }
    }
}

impl UIComponent for PlaylistPickerBuffer {
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
                    .select((!self.playlists.is_empty()).then_some(0));
                *self.list_state.offset_mut() = 0;
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                self.list_state.select(
                    (!self.playlists.is_empty()).then_some(self.playlists.len().saturating_sub(1)),
                );
                UIAction::Render
            }
            UIAction::SelectItem => {
                if let Some((playlist_id, name, _)) = self.selected_playlist() {
                    match &self.mode {
                        PickerMode::AddEpisode {
                            podcast_id,
                            episode_id,
                        } => UIAction::TriggerAddToPlaylist {
                            playlist_id: playlist_id.clone(),
                            podcast_id: podcast_id.clone(),
                            episode_id: episode_id.clone(),
                        },
                        PickerMode::EpisodeMembership => UIAction::OpenPlaylistDetail {
                            playlist_id: playlist_id.clone(),
                            playlist_name: name.clone(),
                        },
                    }
                } else {
                    UIAction::ShowMessage("No playlist selected".to_string())
                }
            }
            UIAction::HideMinibuffer => UIAction::CloseBuffer(self.id.clone()),
            UIAction::CreatePlaylist => match &self.mode {
                PickerMode::AddEpisode { .. } => {
                    UIAction::PromptInput("Create playlist: ".to_string())
                }
                PickerMode::EpisodeMembership => UIAction::None,
            },
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
            .title(match &self.mode {
                PickerMode::AddEpisode { .. } => "Add to Playlist".to_string(),
                PickerMode::EpisodeMembership if self.membership_stale => {
                    "Episode Playlists (stale)".to_string()
                }
                PickerMode::EpisodeMembership => "Episode Playlists".to_string(),
            })
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
            let empty_text = match (&self.mode, &self.membership_error) {
                (PickerMode::AddEpisode { .. }, _) => "No playlists available".to_string(),
                (PickerMode::EpisodeMembership, Some(error)) => {
                    format!("Membership unavailable: {error}")
                }
                (PickerMode::EpisodeMembership, None) => {
                    "This episode is not in any playlists".to_string()
                }
            };
            vec![ListItem::new(empty_text)]
        } else {
            self.playlists
                .iter()
                .enumerate()
                .map(|(index, (_, name, count))| {
                    let selected = self.list_state.selected() == Some(index);
                    let marker = if selected { "► " } else { "  " };
                    let text = format!(
                        "{marker}{name} ({count} episode{})",
                        if *count == 1 { "" } else { "s" }
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
        match &self.mode {
            PickerMode::AddEpisode { .. } => "Add to Playlist".to_string(),
            PickerMode::EpisodeMembership => "Episode Playlists".to_string(),
        }
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

    #[test]
    fn test_episode_membership_picker_is_read_only_and_opens_selected_playlist() {
        let playlist_id = PlaylistId::new();
        let mut buffer = PlaylistPickerBuffer::new_episode_membership(
            vec![(playlist_id.clone(), "Favorite Episodes".to_string(), 3)],
            "episode-detail-test".to_string(),
            false,
            None,
        );

        assert_eq!(
            buffer.handle_action(UIAction::SelectItem),
            UIAction::OpenPlaylistDetail {
                playlist_id,
                playlist_name: "Favorite Episodes".to_string(),
            }
        );
        assert_eq!(
            buffer.handle_action(UIAction::CreatePlaylist),
            UIAction::None
        );
        assert_eq!(
            buffer.handle_action(UIAction::HideMinibuffer),
            UIAction::CloseBuffer("playlist-picker".to_string())
        );
    }

    #[test]
    fn test_playlist_picker_scrolls_selected_row_into_view() {
        let playlists = (0..12)
            .map(|index| (PlaylistId::new(), format!("Playlist {index:02}"), index))
            .collect::<Vec<_>>();
        let selected_id = playlists[6].0.clone();
        let mut buffer = PlaylistPickerBuffer::new(
            playlists,
            PodcastId::new(),
            EpisodeId::new(),
            "episodes-test".to_string(),
        );
        for _ in 0..6 {
            buffer.handle_action(UIAction::MoveDown);
        }

        let backend = ratatui::backend::TestBackend::new(50, 6);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                buffer.render(frame, area);
            })
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
            buffer.selected_playlist().map(|playlist| &playlist.0),
            Some(&selected_id)
        );
    }
}
