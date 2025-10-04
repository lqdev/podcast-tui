// Episode list buffer - displays episodes for a selected podcast
//
// This buffer shows episodes from a podcast and allows playback,
// download, and queue management operations.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{
    download::DownloadManager,
    podcast::{subscription::SubscriptionManager, Episode},
    storage::{JsonStorage, PodcastId, Storage},
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};
use std::sync::Arc;

/// Buffer for displaying episodes from a podcast
pub struct EpisodeListBuffer {
    id: String,
    podcast_name: String,
    pub podcast_id: PodcastId,
    episodes: Vec<Episode>,
    selected_index: Option<usize>,
    focused: bool,
    theme: Theme,
    subscription_manager: Option<Arc<SubscriptionManager<JsonStorage>>>,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,
}

impl EpisodeListBuffer {
    /// Create a new episode list buffer for a podcast
    pub fn new(podcast_name: String, podcast_id: PodcastId) -> Self {
        Self {
            id: format!("episodes-{}", podcast_name.replace(' ', "-").to_lowercase()),
            podcast_name,
            podcast_id,
            episodes: Vec::new(),
            selected_index: None,
            focused: false,
            theme: Theme::default(),
            subscription_manager: None,
            download_manager: None,
        }
    }

    /// Set managers
    pub fn set_managers(
        &mut self,
        subscription_manager: Arc<SubscriptionManager<JsonStorage>>,
        download_manager: Arc<DownloadManager<JsonStorage>>,
    ) {
        self.subscription_manager = Some(subscription_manager);
        self.download_manager = Some(download_manager);
    }

    /// Load episodes for the podcast
    pub async fn load_episodes(&mut self) -> Result<(), String> {
        if let Some(ref manager) = self.subscription_manager {
            match manager.get_podcast(&self.podcast_id).await {
                Ok(_podcast) => {
                    // Load episodes from storage
                    if let Some(ref sm) = self.subscription_manager {
                        match sm.storage.load_episodes(&self.podcast_id).await {
                            Ok(episodes) => {
                                self.episodes = episodes;
                                if !self.episodes.is_empty() && self.selected_index.is_none() {
                                    self.selected_index = Some(0);
                                }
                                Ok(())
                            }
                            Err(e) => Err(e.to_string()),
                        }
                    } else {
                        Err("No subscription manager".to_string())
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("No subscription manager available".to_string())
        }
    }

    /// Set episodes directly (for sync updates from app events)
    pub fn set_episodes(&mut self, episodes: Vec<Episode>) {
        self.episodes = episodes;
        if !self.episodes.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(0);
        }
    }

    /// Get selected episode
    pub fn selected_episode(&self) -> Option<&Episode> {
        self.selected_index.and_then(|i| self.episodes.get(i))
    }

    /// Download selected episode
    pub async fn download_selected(&self) -> Result<(), String> {
        if let (Some(episode), Some(ref dm)) = (self.selected_episode(), &self.download_manager) {
            dm.download_episode(&self.podcast_id, &episode.id)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("No episode selected or download manager unavailable".to_string())
        }
    }

    /// Delete selected episode download
    pub async fn delete_selected(&self) -> Result<(), String> {
        if let (Some(episode), Some(ref dm)) = (self.selected_episode(), &self.download_manager) {
            dm.delete_episode(&self.podcast_id, &episode.id)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("No episode selected or download manager unavailable".to_string())
        }
    }

    /// Move selection up
    fn select_previous(&mut self) {
        if self.episodes.is_empty() {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(i) if i == 0 => Some(self.episodes.len() - 1),
            Some(i) => Some(i - 1),
            None => Some(0),
        };
    }

    /// Move selection down
    fn select_next(&mut self) {
        if self.episodes.is_empty() {
            return;
        }

        self.selected_index = match self.selected_index {
            Some(i) if i >= self.episodes.len() - 1 => Some(0),
            Some(i) => Some(i + 1),
            None => Some(0),
        };
    }

    /// Set the theme for this buffer
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

impl Buffer for EpisodeListBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        format!("Episodes: {}", self.podcast_name)
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Episode List Commands:".to_string(),
            "  C-n, ↓    Next episode".to_string(),
            "  C-p, ↑    Previous episode".to_string(),
            "  Enter     View episode info".to_string(),
            "  D         Download episode".to_string(),
            "  X         Delete downloaded file".to_string(),
            "  m         Mark as played".to_string(),
            "  u         Mark as unplayed".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for EpisodeListBuffer {
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
                if let Some(_index) = self.selected_index {
                    if !self.episodes.is_empty() {
                        // For now, just show episode info
                        if let Some(episode) = self.selected_episode() {
                            UIAction::ShowMinibuffer(format!(
                                "Episode: {} [{}]",
                                episode.title, episode.status
                            ))
                        } else {
                            UIAction::None
                        }
                    } else {
                        UIAction::None
                    }
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToTop => {
                if !self.episodes.is_empty() {
                    self.selected_index = Some(0);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::MoveToBottom => {
                if !self.episodes.is_empty() {
                    self.selected_index = Some(self.episodes.len() - 1);
                    UIAction::Render
                } else {
                    UIAction::None
                }
            }
            UIAction::DownloadEpisode => {
                if let Some(episode) = self.selected_episode() {
                    if episode.is_downloaded() {
                        UIAction::ShowMessage("Episode already downloaded".to_string())
                    } else if matches!(episode.status, crate::podcast::EpisodeStatus::Downloading) {
                        UIAction::ShowMessage("Episode is already downloading".to_string())
                    } else {
                        // Return action to trigger async download
                        UIAction::TriggerDownload {
                            podcast_id: self.podcast_id.clone(),
                            episode_id: episode.id.clone(),
                            episode_title: episode.title.clone(),
                        }
                    }
                } else {
                    UIAction::ShowMessage("No episode selected for download".to_string())
                }
            }
            UIAction::DeleteDownloadedEpisode => {
                if let Some(episode) = self.selected_episode() {
                    if episode.is_downloaded() {
                        UIAction::TriggerDeleteDownload {
                            podcast_id: self.podcast_id.clone(),
                            episode_id: episode.id.clone(),
                            episode_title: episode.title.clone(),
                        }
                    } else {
                        UIAction::ShowMessage("Episode is not downloaded".to_string())
                    }
                } else {
                    UIAction::ShowMessage("No episode selected".to_string())
                }
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .episodes
            .iter()
            .enumerate()
            .map(|(i, episode)| {
                let status_indicator = match episode.status {
                    crate::podcast::EpisodeStatus::New => "○",
                    crate::podcast::EpisodeStatus::Downloaded => "●",
                    crate::podcast::EpisodeStatus::Downloading => "◐",
                    crate::podcast::EpisodeStatus::Played => "✓",
                    crate::podcast::EpisodeStatus::DownloadFailed => "✗",
                };
                let content = format!(" {} {}", status_indicator, episode.title);

                if Some(i) == self.selected_index {
                    ListItem::new(content).style(self.theme.selected_style())
                } else {
                    ListItem::new(content).style(self.theme.text_style())
                }
            })
            .collect();

        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("Episodes: {}", self.podcast_name))
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style()),
            )
            .style(self.theme.text_style());

        frame.render_widget(list, area);

        // Show status
        if self.episodes.is_empty() {
            let empty_msg = "No episodes available.";
            let status_area = Rect {
                x: area.x + 2,
                y: area.y + area.height / 2,
                width: area.width.saturating_sub(4),
                height: 1,
            };

            let status =
                ratatui::widgets::Paragraph::new(empty_msg).style(self.theme.muted_style());
            frame.render_widget(status, status_area);
        } else if let Some(index) = self.selected_index {
            let status_msg = format!(" {} of {} episodes ", index + 1, self.episodes.len());
            let status_area = Rect {
                x: area.x + area.width.saturating_sub(status_msg.len() as u16 + 2),
                y: area.y + area.height - 1,
                width: status_msg.len() as u16,
                height: 1,
            };

            let status =
                ratatui::widgets::Paragraph::new(status_msg).style(self.theme.muted_style());
            frame.render_widget(status, status_area);
        }
    }

    fn title(&self) -> String {
        self.name()
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
    use crate::storage::PodcastId;

    #[test]
    fn test_episode_list_buffer_creation() {
        let podcast_name = "Test Podcast".to_string();
        let podcast_id = PodcastId::new();
        let buffer = EpisodeListBuffer::new(podcast_name.clone(), podcast_id.clone());

        assert_eq!(buffer.id(), "episodes-test-podcast");
        assert_eq!(buffer.name(), "Episodes: Test Podcast");
        assert!(buffer.can_close());
        assert_eq!(buffer.selected_index, None);
        assert_eq!(buffer.podcast_name, podcast_name);
        assert_eq!(buffer.podcast_id, podcast_id);
    }

    #[test]
    fn test_navigation() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        // Add some mock episodes for testing
        buffer.episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Ep1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Ep2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
        ];
        buffer.selected_index = Some(0);

        // Test moving down
        let action = buffer.handle_action(UIAction::MoveDown);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.selected_index, Some(1));

        // Test moving up
        let action = buffer.handle_action(UIAction::MoveUp);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.selected_index, Some(0));
    }

    #[test]
    fn test_selection_wrapping() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        // Add some mock episodes for testing
        buffer.episodes = vec![
            Episode::new(
                PodcastId::new(),
                "Ep1".to_string(),
                "url1".to_string(),
                chrono::Utc::now(),
            ),
            Episode::new(
                PodcastId::new(),
                "Ep2".to_string(),
                "url2".to_string(),
                chrono::Utc::now(),
            ),
        ];

        // Move to top
        buffer.handle_action(UIAction::MoveToTop);
        assert_eq!(buffer.selected_index, Some(0));

        // Move up from top (should wrap to bottom)
        buffer.handle_action(UIAction::MoveUp);
        assert_eq!(buffer.selected_index, Some(buffer.episodes.len() - 1));

        // Move down from bottom (should wrap to top)
        buffer.handle_action(UIAction::MoveDown);
        assert_eq!(buffer.selected_index, Some(0));
    }

    #[test]
    fn test_episode_selection() {
        let mut buffer = EpisodeListBuffer::new("Test".to_string(), PodcastId::new());
        // Add some mock episodes for testing
        buffer.episodes = vec![Episode::new(
            PodcastId::new(),
            "Ep1".to_string(),
            "url1".to_string(),
            chrono::Utc::now(),
        )];
        buffer.selected_index = Some(0);

        // Select an episode
        let action = buffer.handle_action(UIAction::SelectItem);
        match action {
            UIAction::ShowMinibuffer(msg) => {
                assert!(msg.contains("Episode:"));
                assert!(msg.contains("Ep1"));
            }
            _ => panic!("Expected ShowMinibuffer action"),
        }
    }
}
