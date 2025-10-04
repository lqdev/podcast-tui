// Downloads buffer - shows all download activity and management// Downloads buffer - shows all download activity and management

////

// This buffer provides a centralized view of all episode downloads,// This buffer provides a centralized view of all episode downloads,

// their progress, and management options like canceling or retrying.// their progress, and management options like canceling or retrying.



use ratatui::{use ratatui::{

    layout::{Constraint, Direction, Layout, Rect},    layout::{Constraint, Direction, Layout, Rect},

    widgets::{Block, Borders, List, ListItem, Paragraph},    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},

    Frame,    Frame,

};};



use crate::{use crate::{

    download::{DownloadManager, DownloadStatus},    download::{DownloadManager, DownloadStatus},

    podcast::Episode,    podcast::Episode,

    storage::{EpisodeId, JsonStorage, PodcastId, Storage},    storage::{EpisodeId, JsonStorage, PodcastId, Storage},

    ui::{    ui::{

        buffers::{Buffer, BufferId},        buffers::{Buffer, BufferId},

        themes::Theme,        themes::Theme,

        UIAction, UIComponent,        UIAction, UIComponent,

    },    },

};};

use std::sync::Arc;use std::sync::Arc;



/// Download entry for tracking downloads/// Download entry for tracking downloads

#[derive(Debug, Clone)]#[derive(Debug, Clone)]

pub struct DownloadEntry {pub struct DownloadEntry {

    pub podcast_id: PodcastId,    pub podcast_id: PodcastId,

    pub episode_id: EpisodeId,    pub episode_id: EpisodeId,

    pub podcast_name: String,    pub podcast_name: String,

    pub episode_title: String,    pub episode_title: String,

    pub status: DownloadStatus,    pub status: DownloadStatus,

    pub progress: Option<(u64, u64)>, // (downloaded, total)    pub progress: Option<(u64, u64)>, // (downloaded, total)

    pub error_message: Option<String>,    pub error_message: Option<String>,

}}



/// Buffer for managing downloads/// Buffer for managing downloads

pub struct DownloadsBuffer {pub struct DownloadsBuffer {

    id: String,    id: String,

    downloads: Vec<DownloadEntry>,    downloads: Vec<DownloadEntry>,

    selected_index: Option<usize>,    selected_index: Option<usize>,

    focused: bool,    focused: bool,

    theme: Theme,    theme: Theme,

    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,

    storage: Option<Arc<JsonStorage>>,    storage: Option<Arc<JsonStorage>>,

}}



impl DownloadsBuffer {impl DownloadsBuffer {

    /// Create a new downloads buffer    /// Create a new downloads buffer

    pub fn new() -> Self {    pub fn new() -> Self {

        Self {        Self {

            id: "downloads".to_string(),            id: "downloads".to_string(),

            downloads: Vec::new(),            downloads: Vec::new(),

            selected_index: None,            selected_index: None,

            focused: false,            focused: false,

            theme: Theme::default(),            theme: Theme::default(),

            download_manager: None,            download_manager: None,

            storage: None,            storage: None,

        }        }

    }    }



    /// Set managers    /// Set managers

    pub fn set_managers(    pub fn set_managers(

        &mut self,        &mut self,

        download_manager: Arc<DownloadManager<JsonStorage>>,        download_manager: Arc<DownloadManager<JsonStorage>>,

        storage: Arc<JsonStorage>,        storage: Arc<JsonStorage>,

    ) {    ) {

        self.download_manager = Some(download_manager);        self.download_manager = Some(download_manager);

        self.storage = Some(storage);        self.storage = Some(storage);

    }    }



    /// Load current downloads from storage    /// Load current downloads from storage

    pub async fn refresh_downloads(&mut self) -> Result<(), String> {    pub async fn refresh_downloads(&mut self) -> Result<(), String> {

        if let Some(ref storage) = self.storage {        if let Some(ref storage) = self.storage {

            self.downloads.clear();            self.downloads.clear();



            // Load all podcasts and their episodes to find downloading/downloaded ones            // Load all podcasts and their episodes to find downloading/downloaded ones

            match storage.load_podcasts().await {            match storage.load_podcasts().await {

                Ok(podcasts) => {                Ok(podcasts) => {

                    for podcast in podcasts {                    for podcast in podcasts {

                        match storage.load_episodes(&podcast.id).await {                        match storage.load_episodes(&podcast.id).await {

                            Ok(episodes) => {                            Ok(episodes) => {

                                for episode in episodes {                                for episode in episodes {

                                    if matches!(                                    if matches!(

                                        episode.status,                                        episode.status,

                                        crate::podcast::EpisodeStatus::Downloading                                        crate::podcast::EpisodeStatus::Downloading

                                            | crate::podcast::EpisodeStatus::Downloaded                                            | crate::podcast::EpisodeStatus::Downloaded

                                            | crate::podcast::EpisodeStatus::DownloadFailed                                            | crate::podcast::EpisodeStatus::DownloadFailed

                                    ) {                                    ) {

                                        let status = match episode.status {                                        let status = match episode.status {

                                            crate::podcast::EpisodeStatus::Downloading => {                                            crate::podcast::EpisodeStatus::Downloading => {

                                                DownloadStatus::InProgress                                                DownloadStatus::InProgress

                                            }                                            }

                                            crate::podcast::EpisodeStatus::Downloaded => {                                            crate::podcast::EpisodeStatus::Downloaded => {

                                                DownloadStatus::Completed                                                DownloadStatus::Completed

                                            }                                            }

                                            crate::podcast::EpisodeStatus::DownloadFailed => {                                            crate::podcast::EpisodeStatus::DownloadFailed => {

                                                DownloadStatus::Failed("Download failed".to_string())                                                DownloadStatus::Failed("Download failed".to_string())

                                            }                                            }

                                            _ => DownloadStatus::Queued,                                            _ => DownloadStatus::Queued,

                                        };                                        };



                                        let entry = DownloadEntry {                                        let entry = DownloadEntry {

                                            podcast_id: podcast.id.clone(),                                            podcast_id: podcast.id.clone(),

                                            episode_id: episode.id.clone(),                                            episode_id: episode.id.clone(),

                                            podcast_name: podcast.title.clone(),                                            podcast_name: podcast.title.clone(),

                                            episode_title: episode.title.clone(),                                            episode_title: episode.title.clone(),

                                            status,                                            status,

                                            progress: episode.file_size.map(|size| {                                            progress: episode.file_size.map(|size| {

                                                if episode.is_downloaded() {                                                if episode.is_downloaded() {

                                                    (size, size)                                                    (size, size)

                                                } else {                                                } else {

                                                    (0, size)                                                    (0, size)

                                                }                                                }

                                            }),                                            }),

                                            error_message: None,                                            error_message: None,

                                        };                                        };



                                        self.downloads.push(entry);                                        self.downloads.push(entry);

                                    }                                    }

                                }                                }

                            }                            }

                            Err(e) => eprintln!("Failed to load episodes for {}: {}", podcast.title, e),                            Err(e) => eprintln!(\"Failed to load episodes for {}: {}\", podcast.title, e),

                        }                        }

                    }                    }

                }                }

                Err(e) => return Err(format!("Failed to load podcasts: {}", e)),                Err(e) => return Err(format!(\"Failed to load podcasts: {}\", e)),

            }            }



            // Set selection if we have downloads            // Set selection if we have downloads

            if !self.downloads.is_empty() && self.selected_index.is_none() {            if !self.downloads.is_empty() && self.selected_index.is_none() {

                self.selected_index = Some(0);                self.selected_index = Some(0);

            }            }



            Ok(())            Ok(())

        } else {        } else {

            Err("No storage available".to_string())            Err(\"No storage available\".to_string())

        }        }

    }    }



    /// Get selected download entry    /// Get selected download entry

    pub fn selected_download(&self) -> Option<&DownloadEntry> {    pub fn selected_download(&self) -> Option<&DownloadEntry> {

        self.selected_index        self.selected_index

            .and_then(|i| self.downloads.get(i))            .and_then(|i| self.downloads.get(i))

    }    }



    /// Move selection up    /// Move selection up

    fn select_previous(&mut self) {    fn select_previous(&mut self) {

        if let Some(selected) = self.selected_index {        if let Some(selected) = self.selected_index {

            if selected > 0 {            if selected > 0 {

                self.selected_index = Some(selected - 1);                self.selected_index = Some(selected - 1);

            }            }

        }        }

    }    }



    /// Move selection down    /// Move selection down

    fn select_next(&mut self) {    fn select_next(&mut self) {

        if let Some(selected) = self.selected_index {        if let Some(selected) = self.selected_index {

            if selected < self.downloads.len().saturating_sub(1) {            if selected < self.downloads.len().saturating_sub(1) {

                self.selected_index = Some(selected + 1);                self.selected_index = Some(selected + 1);

            }            }

        } else if !self.downloads.is_empty() {        } else if !self.downloads.is_empty() {

            self.selected_index = Some(0);            self.selected_index = Some(0);

        }        }

    }    }



    /// Format file size for display    /// Format file size for display

    fn format_progress(&self, progress: Option<(u64, u64)>) -> String {    fn format_progress(&self, progress: Option<(u64, u64)>) -> String {

        match progress {        match progress {

            Some((downloaded, total)) => {            Some((downloaded, total)) => {

                let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;                let downloaded_mb = downloaded as f64 / 1024.0 / 1024.0;

                let total_mb = total as f64 / 1024.0 / 1024.0;                let total_mb = total as f64 / 1024.0 / 1024.0;

                let percentage = if total > 0 {                let percentage = if total > 0 {

                    (downloaded as f64 / total as f64 * 100.0) as u8                    (downloaded as f64 / total as f64 * 100.0) as u8

                } else {                } else {

                    0                    0

                };                };

                format!("{:.1}/{:.1} MB ({}%)", downloaded_mb, total_mb, percentage)                format!(\"{:.1}/{:.1} MB ({}%)\", downloaded_mb, total_mb, percentage)

            }            }

            None => "Unknown size".to_string(),            None => \"Unknown size\".to_string(),

        }        }

    }    }

}}



impl Buffer for DownloadsBuffer {impl Buffer for DownloadsBuffer {

    fn id(&self) -> &BufferId {    fn id(&self) -> &BufferId {

        &self.id        &self.id

    }    }



    fn set_focused(&mut self, focused: bool) {    fn set_focused(&mut self, focused: bool) {

        self.focused = focused;        self.focused = focused;

    }    }



    fn is_focused(&self) -> bool {    fn is_focused(&self) -> bool {

        self.focused        self.focused

    }    }



    fn help_text(&self) -> Vec<String> {    fn help_text(&self) -> Vec<String> {

        vec![        vec![

            "Downloads Buffer Help".to_string(),            \"Downloads Buffer Help\".to_string(),

            "".to_string(),            \"\".to_string(),

            "Navigation:".to_string(),            \"Navigation:\".to_string(),

            "  ↑/k       Move up".to_string(),            \"  ↑/k       Move up\".to_string(),

            "  ↓/j       Move down".to_string(),            \"  ↓/j       Move down\".to_string(),

            "  Enter     View episode details".to_string(),            \"  Enter     View episode details\".to_string(),

            "".to_string(),            \"\".to_string(),

            "Actions:".to_string(),            \"Actions:\".to_string(),

            "  r         Refresh downloads list".to_string(),            \"  r         Refresh downloads list\".to_string(),

            "  d         Delete selected download".to_string(),            \"  d         Delete selected download\".to_string(),

            "  c         Cancel/retry download".to_string(),            \"  c         Cancel/retry download\".to_string(),

            "  o         Open downloads folder".to_string(),            \"  o         Open downloads folder\".to_string(),

            "  C         Clear completed downloads".to_string(),            \"  C         Clear completed downloads\".to_string(),

            "".to_string(),            \"\".to_string(),

            "  C-h       Show help".to_string(),            \"  C-h       Show help\".to_string(),

        ]        ]

    }    }

}}



impl UIComponent for DownloadsBuffer {impl UIComponent for DownloadsBuffer {

    fn handle_action(&mut self, action: UIAction) -> UIAction {    fn handle_action(&mut self, action: UIAction) -> UIAction {

        match action {        match action {

            UIAction::MoveUp => {            UIAction::MoveUp => {

                self.select_previous();                self.select_previous();

                UIAction::Render                UIAction::Render

            }            }

            UIAction::MoveDown => {            UIAction::MoveDown => {

                self.select_next();                self.select_next();

                UIAction::Render                UIAction::Render

            }            }

            UIAction::Refresh => {            UIAction::Refresh => {

                UIAction::TriggerRefreshDownloads                UIAction::TriggerRefreshDownloads

            }            }

            UIAction::SelectItem => {            UIAction::SelectItem => {

                if let Some(download) = self.selected_download() {                if let Some(download) = self.selected_download() {

                    UIAction::ShowMinibuffer(format!(                    UIAction::ShowMinibuffer(format!(

                        "Download: {} - {} [{}]",                        \"Download: {} - {} [{}]\",

                        download.podcast_name, download.episode_title,                         download.podcast_name, download.episode_title, 

                        match &download.status {                        match &download.status {

                            DownloadStatus::InProgress => "In Progress",                            DownloadStatus::InProgress => \"In Progress\",

                            DownloadStatus::Completed => "Completed",                            DownloadStatus::Completed => \"Completed\",

                            DownloadStatus::Failed(_) => "Failed",                            DownloadStatus::Failed(_) => \"Failed\",

                            DownloadStatus::Queued => "Queued",                            DownloadStatus::Queued => \"Queued\",

                        }                        }

                    ))                    ))

                } else {                } else {

                    UIAction::ShowMessage("No download selected".to_string())                    UIAction::ShowMessage(\"No download selected\".to_string())

                }                }

            }            }

            _ => UIAction::None,            _ => UIAction::None,

        }        }

    }    }



    fn render(&mut self, frame: &mut Frame, area: Rect) {    fn render(&mut self, frame: &mut Frame, area: Rect) {

        let chunks = Layout::default()        let chunks = Layout::default()

            .direction(Direction::Vertical)            .direction(Direction::Vertical)

            .constraints([Constraint::Min(3), Constraint::Length(3)])            .constraints([Constraint::Min(3), Constraint::Length(3)])

            .split(area);            .split(area);



        // Main downloads list        // Main downloads list

        let items: Vec<ListItem> = self        let items: Vec<ListItem> = self

            .downloads            .downloads

            .iter()            .iter()

            .enumerate()            .enumerate()

            .map(|(i, download)| {            .map(|(i, download)| {

                let status_char = match download.status {                let status_char = match download.status {

                    DownloadStatus::Queued => "⏳",                    DownloadStatus::Queued => \"⏳\",

                    DownloadStatus::InProgress => "⬇️",                    DownloadStatus::InProgress => \"⬇️\",

                    DownloadStatus::Completed => "✅",                    DownloadStatus::Completed => \"✅\",

                    DownloadStatus::Failed(_) => "❌",                    DownloadStatus::Failed(_) => \"❌\",

                };                };



                let progress_info = if let DownloadStatus::InProgress = download.status {                let progress_info = if let DownloadStatus::InProgress = download.status {

                    format!(" [{}]", self.format_progress(download.progress))                    format!(\" [{}]\", self.format_progress(download.progress))

                } else {                } else {

                    String::new()                    String::new()

                };                };



                let content = format!(                let content = format!(

                    "{} {} - {}{}",                    \"{} {} - {}{}\",

                    status_char, download.podcast_name, download.episode_title, progress_info                    status_char, download.podcast_name, download.episode_title, progress_info

                );                );



                if Some(i) == self.selected_index {                if Some(i) == self.selected_index {

                    ListItem::new(content).style(self.theme.selected_style())                    ListItem::new(content).style(self.theme.selected_style())

                } else {                } else {

                    ListItem::new(content).style(self.theme.text_style())                    ListItem::new(content).style(self.theme.text_style())

                }                }

            })            })

            .collect();            .collect();



        let border_style = if self.focused {        let border_style = if self.focused {

            self.theme.border_focused_style()            self.theme.border_focused_style()

        } else {        } else {

            self.theme.border_style()            self.theme.border_style()

        };        };



        let downloads_list = List::new(items)        let downloads_list = List::new(items)

            .block(            .block(

                Block::default()                Block::default()

                    .borders(Borders::ALL)                    .borders(Borders::ALL)

                    .title(format!("Downloads ({})", self.downloads.len()))                    .title(format!(\"Downloads ({})\", self.downloads.len()))

                    .border_style(border_style),                    .border_style(border_style),

            )            )

            .style(self.theme.text_style());            .style(self.theme.text_style());



        frame.render_widget(downloads_list, chunks[0]);        frame.render_widget(downloads_list, chunks[0]);



        // Status/help bar        // Status/help bar

        let status_text = if self.downloads.is_empty() {        let status_text = if self.downloads.is_empty() {

            "No downloads found. Press 'r' to refresh.".to_string()            \"No downloads found. Press 'r' to refresh.\".to_string()

        } else if let Some(download) = self.selected_download() {        } else if let Some(download) = self.selected_download() {

            match &download.status {            match &download.status {

                DownloadStatus::Failed(msg) => format!("Failed: {}", msg),                DownloadStatus::Failed(msg) => format!(\"Failed: {}\", msg),

                DownloadStatus::InProgress => "Press 'c' to cancel • 'd' to delete • 'r' to refresh".to_string(),                DownloadStatus::InProgress => \"Press 'c' to cancel • 'd' to delete • 'r' to refresh\".to_string(),

                DownloadStatus::Completed => "Press 'd' to delete • 'o' to open folder".to_string(),                DownloadStatus::Completed => \"Press 'd' to delete • 'o' to open folder\".to_string(),

                DownloadStatus::Queued => "Queued for download".to_string(),                DownloadStatus::Queued => \"Queued for download\".to_string(),

            }            }

        } else {        } else {

            "Press 'r' to refresh downloads".to_string()            \"Press 'r' to refresh downloads\".to_string()

        };        };



        let status_paragraph = Paragraph::new(status_text)        let status_paragraph = Paragraph::new(status_text)

            .block(Block::default().borders(Borders::ALL).title("Actions"))            .block(Block::default().borders(Borders::ALL).title(\"Actions\"))

            .style(self.theme.text_style());            .style(self.theme.text_style());



        frame.render_widget(status_paragraph, chunks[1]);        frame.render_widget(status_paragraph, chunks[1]);

    }    }

}}