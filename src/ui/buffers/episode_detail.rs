// Episode detail buffer - displays detailed information about a podcast episode
//
// This buffer shows comprehensive episode details including description,
// metadata, and status information. It is a read-only view created from
// the episode list buffer when pressing Enter on an episode.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{
    podcast::Episode,
    storage::PodcastId,
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

/// Buffer for displaying detailed episode information
pub struct EpisodeDetailBuffer {
    id: String,
    episode_title: String,
    episode: Episode,
    podcast_id: PodcastId,
    scroll_offset: usize,
    focused: bool,
    theme: Theme,
}

impl EpisodeDetailBuffer {
    /// Create a new episode detail buffer
    pub fn new(episode: Episode) -> Self {
        let episode_title = episode.title.clone();
        let id = format!("episode-detail-{}", episode.id);
        let podcast_id = episode.podcast_id.clone();

        Self {
            id,
            episode_title,
            podcast_id,
            episode,
            scroll_offset: 0,
            focused: false,
            theme: Theme::default(),
        }
    }

    /// Set the theme for this buffer
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Generate content lines for display
    fn generate_content(&self) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        // Title section
        lines.push(Line::from(vec![
            Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&self.episode.title),
        ]));
        lines.push(Line::from(""));

        // Published date
        let published_str = self
            .episode
            .published
            .format("%Y-%m-%d %H:%M UTC")
            .to_string();
        lines.push(Line::from(vec![
            Span::styled("Published: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(published_str),
        ]));

        // Status
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", self.episode.status)),
        ]));

        // Duration
        if self.episode.duration.is_some() {
            lines.push(Line::from(vec![
                Span::styled("Duration: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.episode.formatted_duration()),
            ]));
        }

        // File size
        if self.episode.file_size.is_some() {
            lines.push(Line::from(vec![
                Span::styled("File Size: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.episode.formatted_file_size()),
            ]));
        }

        // Episode number and season
        if let Some(season) = self.episode.season {
            lines.push(Line::from(vec![
                Span::styled("Season: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", season)),
            ]));
        }
        if let Some(episode_num) = self.episode.episode_number {
            lines.push(Line::from(vec![
                Span::styled("Episode: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", episode_num)),
            ]));
        }

        // Episode type
        if let Some(ref ep_type) = self.episode.episode_type {
            lines.push(Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(ep_type.clone()),
            ]));
        }

        // Explicit content warning
        if self.episode.explicit {
            lines.push(Line::from(vec![
                Span::styled("Explicit: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("Yes"),
            ]));
        }

        // Link
        if let Some(ref link) = self.episode.link {
            lines.push(Line::from(vec![
                Span::styled("Link: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(link.clone()),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from("─".repeat(60)));

        // Description
        if let Some(ref description) = self.episode.description {
            // Split description into lines for wrapping
            for line in description.lines() {
                if line.trim().is_empty() {
                    lines.push(Line::from(""));
                } else {
                    lines.push(Line::from(line.to_string()));
                }
            }
        } else {
            lines.push(Line::from("No description available."));
        }

        // Transcript (if available)
        if let Some(ref transcript) = self.episode.transcript {
            lines.push(Line::from(""));
            lines.push(Line::from("─".repeat(60)));
            lines.push(Line::from(vec![Span::styled(
                "Transcript:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from("─".repeat(60)));

            for line in transcript.lines() {
                if line.trim().is_empty() {
                    lines.push(Line::from(""));
                } else {
                    lines.push(Line::from(line.to_string()));
                }
            }
        }

        lines
    }

    /// Scroll up
    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down
    fn scroll_down(&mut self, max_lines: usize, visible_height: usize) {
        let max_scroll = max_lines.saturating_sub(visible_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Scroll page up
    fn page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll page down
    fn page_down(&mut self, max_lines: usize, visible_height: usize, page_size: usize) {
        let max_scroll = max_lines.saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_scroll);
    }

    /// Scroll to top
    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to bottom
    fn scroll_to_bottom(&mut self, max_lines: usize, visible_height: usize) {
        let max_scroll = max_lines.saturating_sub(visible_height);
        self.scroll_offset = max_scroll;
    }
}

impl Buffer for EpisodeDetailBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        format!("Episode: {}", self.episode_title)
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Episode Detail Commands:".to_string(),
            "  C-n, ↓    Scroll down".to_string(),
            "  C-p, ↑    Scroll up".to_string(),
            "  Page Down Page down".to_string(),
            "  Page Up   Page up".to_string(),
            "  Home, <   Scroll to top".to_string(),
            "  End, >    Scroll to bottom".to_string(),
            "  D         Download episode".to_string(),
            "  q, C-k    Close buffer".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for EpisodeDetailBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match action {
            UIAction::MoveUp => {
                self.scroll_up();
                UIAction::Render
            }
            UIAction::MoveDown => {
                // Calculate max_lines to properly scroll
                let content = self.generate_content();
                let max_lines = content.len();
                self.scroll_down(max_lines, 20); // Use reasonable visible height for now
                UIAction::Render
            }
            UIAction::PageUp => {
                let page_size = 10; // Default page size
                self.page_up(page_size);
                UIAction::Render
            }
            UIAction::PageDown => {
                // Calculate max_lines to properly scroll
                let content = self.generate_content();
                let max_lines = content.len();
                self.page_down(max_lines, 20, 10); // Use reasonable values
                UIAction::Render
            }
            UIAction::MoveToTop => {
                self.scroll_to_top();
                UIAction::Render
            }
            UIAction::MoveToBottom => {
                // Calculate max_lines to properly scroll
                let content = self.generate_content();
                let max_lines = content.len();
                self.scroll_to_bottom(max_lines, 20); // Use reasonable visible height
                UIAction::Render
            }
            UIAction::DownloadEpisode => {
                if self.episode.is_downloaded() {
                    UIAction::ShowMessage("Episode already downloaded".to_string())
                } else if matches!(
                    self.episode.status,
                    crate::podcast::EpisodeStatus::Downloading
                ) {
                    UIAction::ShowMessage("Episode is already downloading".to_string())
                } else if self.episode.audio_url.is_empty()
                    && !self
                        .episode
                        .guid
                        .as_ref()
                        .map_or(false, |g| g.starts_with("http"))
                {
                    UIAction::ShowMessage(
                        "Cannot download: No audio URL available for this episode".to_string(),
                    )
                } else {
                    // Return action to trigger async download
                    UIAction::TriggerDownload {
                        podcast_id: self.podcast_id.clone(),
                        episode_id: self.episode.id.clone(),
                        episode_title: self.episode.title.clone(),
                    }
                }
            }
            _ => UIAction::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

        // Generate content and calculate scroll limits
        let content = self.generate_content();
        let max_lines = content.len();
        let max_scroll = max_lines.saturating_sub(visible_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(format!("Episode: {}", self.episode_title))
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title_style(self.theme.title_style()),
            )
            .style(self.theme.text_style())
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset as u16, 0));

        frame.render_widget(paragraph, area);

        // Update scroll offset after rendering
        self.scroll_offset = scroll_offset;
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
    use chrono::Utc;

    #[test]
    fn test_episode_detail_buffer_creation() {
        let episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );
        let buffer = EpisodeDetailBuffer::new(episode.clone());

        assert_eq!(buffer.name(), "Episode: Test Episode");
        assert!(buffer.can_close());
        assert_eq!(buffer.episode_title, "Test Episode");
    }

    #[test]
    fn test_scrolling() {
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );
        episode.description = Some("Line 1\nLine 2\nLine 3\nLine 4\nLine 5".to_string());

        let mut buffer = EpisodeDetailBuffer::new(episode);

        // Test scroll down
        buffer.scroll_down(20, 10);
        assert_eq!(buffer.scroll_offset, 1);

        // Test scroll up
        buffer.scroll_up();
        assert_eq!(buffer.scroll_offset, 0);

        // Test scroll to bottom
        buffer.scroll_to_bottom(20, 10);
        assert_eq!(buffer.scroll_offset, 10);

        // Test scroll to top
        buffer.scroll_to_top();
        assert_eq!(buffer.scroll_offset, 0);
    }

    #[test]
    fn test_navigation_actions() {
        let episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );
        let mut buffer = EpisodeDetailBuffer::new(episode);

        // Test move up
        let action = buffer.handle_action(UIAction::MoveUp);
        assert_eq!(action, UIAction::Render);

        // Test move to top
        let action = buffer.handle_action(UIAction::MoveToTop);
        assert_eq!(action, UIAction::Render);
        assert_eq!(buffer.scroll_offset, 0);
    }

    #[test]
    fn test_download_episode_action() {
        let episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );
        let podcast_id = episode.podcast_id.clone();
        let episode_id = episode.id.clone();
        let mut buffer = EpisodeDetailBuffer::new(episode);

        // Test triggering download
        let action = buffer.handle_action(UIAction::DownloadEpisode);
        match action {
            UIAction::TriggerDownload {
                podcast_id: pid,
                episode_id: eid,
                episode_title,
            } => {
                assert_eq!(pid, podcast_id);
                assert_eq!(eid, episode_id);
                assert_eq!(episode_title, "Test Episode");
            }
            _ => panic!("Expected TriggerDownload action"),
        }
    }

    #[test]
    fn test_download_already_downloaded_episode() {
        use std::fs::File;
        use std::io::Write;

        let mut episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );

        // Create a temporary file with unique name to avoid race conditions
        let temp_path = std::env::temp_dir().join(format!(
            "test_episode_download_{}.mp3",
            uuid::Uuid::new_v4()
        ));
        {
            let mut file = File::create(&temp_path).unwrap();
            file.write_all(b"test data").unwrap();
        }

        // Set episode as downloaded with the temporary file
        episode.status = crate::podcast::EpisodeStatus::Downloaded;
        episode.local_path = Some(temp_path.clone());

        let mut buffer = EpisodeDetailBuffer::new(episode);

        // Test attempting to download already downloaded episode
        let action = buffer.handle_action(UIAction::DownloadEpisode);

        // Clean up the temporary file
        let _ = std::fs::remove_file(&temp_path);

        match action {
            UIAction::ShowMessage(msg) => {
                assert_eq!(msg, "Episode already downloaded");
            }
            _ => panic!("Expected ShowMessage action, got {:?}", action),
        }
    }

    #[test]
    fn test_download_episode_no_audio_url() {
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "".to_string(), // Empty audio URL
            Utc::now(),
        );
        episode.guid = None; // No GUID either

        let mut buffer = EpisodeDetailBuffer::new(episode);

        // Test attempting to download episode without audio URL
        let action = buffer.handle_action(UIAction::DownloadEpisode);
        match action {
            UIAction::ShowMessage(msg) => {
                assert!(msg.contains("No audio URL available"));
            }
            _ => panic!("Expected ShowMessage action"),
        }
    }

    #[test]
    fn test_download_episode_already_downloading() {
        let mut episode = Episode::new(
            PodcastId::new(),
            "Test Episode".to_string(),
            "https://example.com/audio.mp3".to_string(),
            Utc::now(),
        );
        // Set episode as currently downloading
        episode.status = crate::podcast::EpisodeStatus::Downloading;

        let mut buffer = EpisodeDetailBuffer::new(episode);

        // Test attempting to download episode that's already downloading
        let action = buffer.handle_action(UIAction::DownloadEpisode);
        match action {
            UIAction::ShowMessage(msg) => {
                assert_eq!(msg, "Episode is already downloading");
            }
            _ => panic!("Expected ShowMessage action"),
        }
    }
}
