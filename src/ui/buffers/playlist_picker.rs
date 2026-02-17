use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
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

pub struct PlaylistPickerBuffer {
    id: String,
    focused: bool,
    theme: Theme,
    playlists: Vec<(PlaylistId, String, usize)>,
    selected_index: Option<usize>,
    target_podcast_id: PodcastId,
    target_episode_id: EpisodeId,
}

impl PlaylistPickerBuffer {
    pub fn new(
        playlists: Vec<(PlaylistId, String, usize)>,
        target_podcast_id: PodcastId,
        target_episode_id: EpisodeId,
    ) -> Self {
        Self {
            id: "playlist-picker".to_string(),
            focused: false,
            theme: Theme::default(),
            playlists,
            selected_index: Some(0),
            target_podcast_id,
            target_episode_id,
        }
    }

    fn selected_playlist(&self) -> Option<&(PlaylistId, String, usize)> {
        self.selected_index
            .and_then(|index| self.playlists.get(index))
    }

    fn select_previous(&mut self) {
        if self.playlists.is_empty() {
            return;
        }
        self.selected_index = match self.selected_index {
            Some(0) => Some(self.playlists.len() - 1),
            Some(index) => Some(index - 1),
            None => Some(0),
        };
    }

    fn select_next(&mut self) {
        if self.playlists.is_empty() {
            return;
        }
        self.selected_index = match self.selected_index {
            Some(index) if index >= self.playlists.len() - 1 => Some(0),
            Some(index) => Some(index + 1),
            None => Some(0),
        };
    }
}

impl Buffer for PlaylistPickerBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Playlist Picker".to_string()
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Playlist Picker Commands:".to_string(),
            "  ↑/↓      Navigate playlists".to_string(),
            "  Enter    Add episode to selected playlist".to_string(),
            "  Esc      Cancel".to_string(),
        ]
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
            UIAction::SelectItem => {
                if let Some((playlist_id, _, _)) = self.selected_playlist() {
                    UIAction::TriggerAddToPlaylist {
                        playlist_id: playlist_id.clone(),
                        podcast_id: self.target_podcast_id.clone(),
                        episode_id: self.target_episode_id.clone(),
                    }
                } else {
                    UIAction::ShowError("No playlist selected".to_string())
                }
            }
            UIAction::CreatePlaylist => UIAction::PromptInput("Create playlist: ".to_string()),
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let items: Vec<ListItem> = if self.playlists.is_empty() {
            vec![ListItem::new("No playlists available")]
        } else {
            self.playlists
                .iter()
                .enumerate()
                .map(|(index, (_, name, count))| {
                    let selected = self.selected_index == Some(index);
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

        let list = List::new(items).block(
            Block::default()
                .title("Add to Playlist")
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_style(self.theme.title_style()),
        );
        frame.render_widget(list, area);
    }

    fn title(&self) -> String {
        "Add to Playlist".to_string()
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}
