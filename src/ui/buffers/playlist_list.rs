use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
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
    selected_index: Option<usize>,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
    playlist_manager: Option<Arc<PlaylistManager>>,
}

impl PlaylistListBuffer {
    pub fn new() -> Self {
        Self {
            id: "playlist-list".to_string(),
            playlists: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
            playlist_manager: None,
        }
    }

    pub fn set_playlist_manager(&mut self, manager: Arc<PlaylistManager>) {
        self.playlist_manager = Some(manager);
    }

    pub fn set_playlists(&mut self, playlists: Vec<Playlist>) {
        self.playlists = playlists;
        if self.playlists.is_empty() {
            self.selected_index = None;
            self.scroll_offset = 0;
        } else if self.selected_index.is_none() {
            self.selected_index = Some(0);
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

impl Buffer for PlaylistListBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Playlists".to_string()
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
            UIAction::MoveToTop => {
                if !self.playlists.is_empty() {
                    self.selected_index = Some(0);
                    self.scroll_offset = 0;
                }
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                if !self.playlists.is_empty() {
                    self.selected_index = Some(self.playlists.len() - 1);
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
                    UIAction::ShowError("No playlist selected".to_string())
                }
            }
            UIAction::Refresh => UIAction::RefreshAutoPlaylists,
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
            vec![ListItem::new("No playlists yet")]
        } else {
            self.playlists
                .iter()
                .enumerate()
                .map(|(index, playlist)| {
                    let selected = self.selected_index == Some(index);
                    let playlist_type = match playlist.playlist_type {
                        PlaylistType::User => "User",
                        PlaylistType::AutoGenerated { .. } => "Auto",
                    };
                    let marker = if selected { "► " } else { "  " };
                    let text = format!(
                        "{}{} [{}] ({} episode{})",
                        marker,
                        playlist.name,
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

        let list = List::new(items).block(
            Block::default()
                .title("Playlists")
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_style(self.theme.title_style()),
        );

        frame.render_widget(list, area);
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
