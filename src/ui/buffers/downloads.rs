// Downloads buffer - shows all download activity and management
//
// This buffer provides a centralized view of all episode downloads,
// their progress, and management options like canceling or retrying.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    download::{DownloadManager, DownloadStatus},
    storage::{EpisodeId, JsonStorage, PodcastId, Storage},
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

use std::sync::Arc;

/// Download entry for tracking downloads
#[derive(Debug, Clone)]
pub struct DownloadEntry {
    pub podcast_id: PodcastId,
    pub episode_id: EpisodeId,
    pub podcast_name: String,
    pub episode_title: String,
    pub status: DownloadStatus,
    pub progress: Option<(u64, u64)>, // (downloaded, total)
    pub error_message: Option<String>,
}

/// Buffer for managing downloads
pub struct DownloadsBuffer {
    id: String,
    downloads: Vec<DownloadEntry>,
    selected_index: Option<usize>,
    focused: bool,
    theme: Theme,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,
    storage: Option<Arc<JsonStorage>>,
}

impl DownloadsBuffer {
    /// Create a new downloads buffer
    pub fn new() -> Self {
        Self {
            id: "downloads".to_string(),
            downloads: Vec::new(),
            selected_index: None,
            focused: false,
            theme: Theme::default(),
            download_manager: None,
            storage: None,
        }
    }

    /// Set managers
    pub fn set_managers(
        &mut self,
        download_manager: Arc<DownloadManager<JsonStorage>>,
        storage: Arc<JsonStorage>,
    ) {
        self.download_manager = Some(download_manager);
        self.storage = Some(storage);
    }

    /// Load current downloads from storage
    pub async fn refresh_downloads(&mut self) -> Result<(), String> {
        if let Some(ref storage) = self.storage {
            self.downloads.clear();

            // Load all podcasts and their episodes to find downloading/downloaded ones
            match storage.list_podcasts().await {
                Ok(podcast_ids) => {
                    for podcast_id in podcast_ids {
                        // Load the podcast to get its name
                        if let Ok(podcast) = storage.load_podcast(&podcast_id).await {
                            match storage.load_episodes(&podcast_id).await {
                                Ok(episodes) => {
                                    for episode in episodes {
                                        if matches!(
                                            episode.status,
                                            crate::podcast::EpisodeStatus::Downloading
                                                | crate::podcast::EpisodeStatus::Downloaded
                                                | crate::podcast::EpisodeStatus::DownloadFailed
                                        ) {
                                            let status = match episode.status {
                                                crate::podcast::EpisodeStatus::Downloading => {
                                                    DownloadStatus::InProgress
                                                }
                                                crate::podcast::EpisodeStatus::Downloaded => {
                                                    DownloadStatus::Completed
                                                }
                                                crate::podcast::EpisodeStatus::DownloadFailed => {
                                                    DownloadStatus::Failed(
                                                        "Download failed".to_string(),
                                                    )
                                                }
                                                _ => DownloadStatus::Queued,
                                            };

                                            let entry = DownloadEntry {
                                                podcast_id: podcast.id.clone(),
                                                episode_id: episode.id.clone(),
                                                podcast_name: podcast.title.clone(),
                                                episode_title: episode.title.clone(),
                                                status,
                                                progress: episode.file_size.map(|size| {
                                                    if episode.is_downloaded() {
                                                        (size, size)
                                                    } else {
                                                        (0, size)
                                                    }
                                                }),
                                                error_message: None,
                                            };

                                            self.downloads.push(entry);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Failed to load episodes for {}: {}",
                                        podcast.title, e
                                    )
                                }
                            }
                        }
                    }
                }
                Err(e) => return Err(format!("Failed to load podcasts: {}", e)),
            }

            // Set selection if we have downloads
            if !self.downloads.is_empty() && self.selected_index.is_none() {
                self.selected_index = Some(0);
            }

            Ok(())
        } else {
            Err("No storage available".to_string())
        }
    }

    /// Get selected download entry
    pub fn selected_download(&self) -> Option<&DownloadEntry> {
        self.selected_index.and_then(|i| self.downloads.get(i))
    }

    /// Move selection up
    fn select_previous(&mut self) {
        if let Some(selected) = self.selected_index {
            if selected > 0 {
                self.selected_index = Some(selected - 1);
            }
        }
    }

    /// Move selection down
    fn select_next(&mut self) {
        if let Some(selected) = self.selected_index {
            if selected < self.downloads.len().saturating_sub(1) {
                self.selected_index = Some(selected + 1);
            }
        } else if !self.downloads.is_empty() {
            self.selected_index = Some(0);
        }
    }

    /// Format file size for display
    fn format_progress(&self, progress: Option<(u64, u64)>) -> String {
        match progress {
            Some((downloaded, total)) => {
                let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;
                let total_mb = total as f64 / 1024.0 / 1024.0;
                let percentage = if total > 0 {
                    (downloaded as f64 / total as f64 * 100.0) as u8
                } else {
                    0
                };
                format!("{:.1}/{:.1} MB ({}%)", downloaded_mb, total_mb, percentage)
            }
            None => "Unknown size".to_string(),
        }
    }
}

impl Buffer for DownloadsBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Downloads".to_string()
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Downloads Buffer Help".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/k       Move up".to_string(),
            "  ↓/j       Move down".to_string(),
            "  Enter     View episode details".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  r         Refresh downloads list".to_string(),
            "  X         Delete selected download".to_string(),
            "  c         Cancel/retry download".to_string(),
            "  o         Open downloads folder".to_string(),
            "  C         Clear completed downloads".to_string(),
            "".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for DownloadsBuffer {
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
            UIAction::Refresh => UIAction::TriggerRefreshDownloads,
            UIAction::DeleteDownloadedEpisode => {
                if let Some(download) = self.selected_download() {
                    if matches!(download.status, DownloadStatus::Completed) {
                        UIAction::TriggerDeleteDownload {
                            podcast_id: download.podcast_id.clone(),
                            episode_id: download.episode_id.clone(),
                            episode_title: download.episode_title.clone(),
                        }
                    } else {
                        UIAction::ShowMessage(
                            "Selected item is not a completed download".to_string(),
                        )
                    }
                } else {
                    UIAction::ShowMessage("No download selected".to_string())
                }
            }
            UIAction::SelectItem => {
                if let Some(download) = self.selected_download() {
                    UIAction::ShowMinibuffer(format!(
                        "Download: {} - {} [{}]",
                        download.podcast_name,
                        download.episode_title,
                        match &download.status {
                            DownloadStatus::InProgress => "In Progress",
                            DownloadStatus::Completed => "Completed",
                            DownloadStatus::Failed(_) => "Failed",
                            DownloadStatus::Queued => "Queued",
                        }
                    ))
                } else {
                    UIAction::ShowMessage("No download selected".to_string())
                }
            }
            _ => UIAction::None,
        }
    }

    fn title(&self) -> String {
        format!("Downloads ({})", self.downloads.len())
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        // Main downloads list
        let items: Vec<ListItem> = self
            .downloads
            .iter()
            .enumerate()
            .map(|(i, download)| {
                let status_char = match download.status {
                    DownloadStatus::Queued => "⏳",
                    DownloadStatus::InProgress => "⬇️",
                    DownloadStatus::Completed => "✅",
                    DownloadStatus::Failed(_) => "❌",
                };

                let progress_info = if let DownloadStatus::InProgress = download.status {
                    format!(" [{}]", self.format_progress(download.progress))
                } else {
                    String::new()
                };

                let content = format!(
                    "{} {} - {}{}",
                    status_char, download.podcast_name, download.episode_title, progress_info
                );

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

        let downloads_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Downloads ({})", self.downloads.len()))
                    .border_style(border_style),
            )
            .style(self.theme.text_style());

        frame.render_widget(downloads_list, chunks[0]);

        // Status/help bar
        let status_text = if self.downloads.is_empty() {
            "No downloads found. Press 'r' to refresh.".to_string()
        } else if let Some(download) = self.selected_download() {
            match &download.status {
                DownloadStatus::Failed(msg) => format!("Failed: {}", msg),
                DownloadStatus::InProgress => {
                    "Press 'c' to cancel • 'X' to delete • 'r' to refresh".to_string()
                }
                DownloadStatus::Completed => "Press 'X' to delete • 'o' to open folder".to_string(),
                DownloadStatus::Queued => "Queued for download".to_string(),
            }
        } else {
            "Press 'r' to refresh downloads".to_string()
        };

        let status_paragraph = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .style(self.theme.text_style());

        frame.render_widget(status_paragraph, chunks[1]);
    }
}
