// Sync buffer - Device synchronization interface
//
// This buffer provides a UI for syncing downloaded episodes to external devices
// like MP3 players. It shows sync status, history, and allows configuration.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    download::{DownloadManager, SyncReport},
    storage::JsonStorage,
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

use std::sync::Arc;

/// Sync history entry
#[derive(Debug, Clone)]
pub struct SyncHistoryEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub device_path: std::path::PathBuf,
    pub report: SyncReport,
    pub dry_run: bool,
}

/// Buffer for device sync management
pub struct SyncBuffer {
    id: String,
    focused: bool,
    theme: Theme,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,

    // Sync state
    last_sync: Option<SyncHistoryEntry>,
    sync_history: Vec<SyncHistoryEntry>,

    // UI state
    selected_index: usize,
    scroll_offset: usize,
}

impl SyncBuffer {
    /// Create a new sync buffer
    pub fn new() -> Self {
        Self {
            id: "sync".to_string(),
            focused: false,
            theme: Theme::default(),
            download_manager: None,
            last_sync: None,
            sync_history: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    /// Set download manager
    pub fn set_download_manager(&mut self, download_manager: Arc<DownloadManager<JsonStorage>>) {
        self.download_manager = Some(download_manager);
    }

    /// Add a sync result to history
    pub fn add_sync_result(
        &mut self,
        device_path: std::path::PathBuf,
        report: SyncReport,
        dry_run: bool,
    ) {
        let entry = SyncHistoryEntry {
            timestamp: chrono::Utc::now(),
            device_path,
            report,
            dry_run,
        };

        self.last_sync = Some(entry.clone());
        self.sync_history.insert(0, entry);

        // Keep only last 10 sync operations
        if self.sync_history.len() > 10 {
            self.sync_history.truncate(10);
        }
    }

    /// Move selection up
    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    fn select_next(&mut self) {
        if self.selected_index < self.sync_history.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Adjust scroll offset to ensure selected item is visible
    fn adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // If selected item is above the visible area, scroll up
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        // If selected item is below the visible area, scroll down
        else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index.saturating_sub(visible_height - 1);
        }
    }
}

impl Buffer for SyncBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Sync".to_string()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Sync Buffer Help".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑/k       Move up in history".to_string(),
            "  ↓/j       Move down in history".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  s         Start sync (prompts for device path)".to_string(),
            "  d         Dry run sync (preview changes)".to_string(),
            "  r         Refresh sync status".to_string(),
            "  Enter     View sync details".to_string(),
            "".to_string(),
            "  C-h       Show help".to_string(),
        ]
    }
}

impl UIComponent for SyncBuffer {
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
                // Move up by 10 items or to the top
                self.selected_index = self.selected_index.saturating_sub(10);
                UIAction::Render
            }
            UIAction::PageDown => {
                // Move down by 10 items or to the bottom
                let max = self.sync_history.len().saturating_sub(1);
                self.selected_index = (self.selected_index + 10).min(max);
                UIAction::Render
            }
            UIAction::SyncToDevice => {
                // Prompt for device path — prefix must match handle_minibuffer_input_with_context
                UIAction::PromptInput("Sync to device path: ".to_string())
            }
            UIAction::SelectItem => {
                if let Some(entry) = self.sync_history.get(self.selected_index) {
                    let details = format!(
                        "Sync to {} at {}\nCopied: {}, Deleted: {}, Skipped: {}, Errors: {}",
                        entry.device_path.display(),
                        entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        entry.report.files_copied.len(),
                        entry.report.files_deleted.len(),
                        entry.report.files_skipped.len(),
                        entry.report.errors.len()
                    );
                    UIAction::ShowMinibuffer(details)
                } else {
                    UIAction::ShowMessage("No sync history selected".to_string())
                }
            }
            _ => UIAction::None,
        }
    }

    fn title(&self) -> String {
        "Device Sync".to_string()
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
            .constraints([
                Constraint::Length(5), // Status
                Constraint::Min(5),    // History
                Constraint::Length(3), // Actions
            ])
            .split(area);

        // Status section
        let status_text = if let Some(ref entry) = self.last_sync {
            let mode = if entry.dry_run { " [DRY RUN]" } else { "" };
            format!(
                "Last Sync: {}{}\nDevice: {}\nResult: {} copied, {} deleted, {} skipped, {} errors",
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                mode,
                entry.device_path.display(),
                entry.report.files_copied.len(),
                entry.report.files_deleted.len(),
                entry.report.files_skipped.len(),
                entry.report.errors.len()
            )
        } else {
            "No sync performed yet.\nPress 's' to start a sync or 'd' for a dry run.".to_string()
        };

        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let status_paragraph = Paragraph::new(status_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sync Status")
                    .border_style(border_style),
            )
            .style(self.theme.text_style())
            .wrap(Wrap { trim: true });

        frame.render_widget(status_paragraph, chunks[0]);

        // History section
        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        self.adjust_scroll(visible_height);

        let end_index = (self.scroll_offset + visible_height).min(self.sync_history.len());
        let visible_history = &self.sync_history[self.scroll_offset..end_index];

        let items: Vec<ListItem> = visible_history
            .iter()
            .enumerate()
            .map(|(visible_i, entry)| {
                let actual_i = self.scroll_offset + visible_i;
                let status_icon = if entry.report.is_success() {
                    "✅"
                } else {
                    "❌"
                };
                let mode = if entry.dry_run { " [DRY]" } else { "" };

                let content = format!(
                    "{}{} {} - {} → {} copied, {} deleted",
                    status_icon,
                    mode,
                    entry.timestamp.format("%Y-%m-%d %H:%M"),
                    entry
                        .device_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    entry.report.files_copied.len(),
                    entry.report.files_deleted.len()
                );

                if actual_i == self.selected_index {
                    ListItem::new(content).style(self.theme.selected_style())
                } else {
                    ListItem::new(content).style(self.theme.text_style())
                }
            })
            .collect();

        let history_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Sync History ({})", self.sync_history.len()))
                    .border_style(border_style),
            )
            .style(self.theme.text_style());

        frame.render_widget(history_list, chunks[1]);

        // Actions section
        let actions_text = if self.sync_history.is_empty() {
            "Press 's' to start sync • 'd' for dry run • 'C-h' for help"
        } else {
            "↑↓ navigate • Enter to view details • 's' to sync • 'd' for dry run"
        };

        let actions_paragraph = Paragraph::new(actions_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Actions")
                    .border_style(border_style),
            )
            .style(self.theme.text_style());

        frame.render_widget(actions_paragraph, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{UIAction, UIComponent};

    #[test]
    fn test_sync_to_device_returns_prompt_input() {
        // Arrange
        let mut buffer = SyncBuffer::new();

        // Act
        let action = buffer.handle_action(UIAction::SyncToDevice);

        // Assert — prompt must start with "Sync to device path" for context handler
        match action {
            UIAction::PromptInput(prompt) => {
                assert!(
                    prompt.starts_with("Sync to device path"),
                    "Prompt must start with 'Sync to device path', got: {}",
                    prompt
                );
            }
            other => panic!("Expected PromptInput, got {:?}", other),
        }
    }

    #[test]
    fn test_sync_buffer_navigation() {
        // Arrange
        let mut buffer = SyncBuffer::new();

        // Act / Assert — navigation returns Render
        assert_eq!(buffer.handle_action(UIAction::MoveUp), UIAction::Render);
        assert_eq!(buffer.handle_action(UIAction::MoveDown), UIAction::Render);
        assert_eq!(buffer.handle_action(UIAction::PageUp), UIAction::Render);
        assert_eq!(buffer.handle_action(UIAction::PageDown), UIAction::Render);
    }

    #[test]
    fn test_sync_buffer_select_item_empty_history() {
        // Arrange
        let mut buffer = SyncBuffer::new();

        // Act
        let action = buffer.handle_action(UIAction::SelectItem);

        // Assert — no history, should show message
        assert_eq!(
            action,
            UIAction::ShowMessage("No sync history selected".to_string())
        );
    }
}
