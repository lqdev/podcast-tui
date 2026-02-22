// Sync buffer - Device synchronization interface
//
// This buffer provides a UI for syncing downloaded episodes to external devices
// like MP3 players. It shows sync status, history, saved targets, and allows
// picking a sync directory via a ranger-style directory picker.

use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crate::{
    download::{DownloadManager, SyncHistorySummary, SyncProgressEvent, SyncReport},
    storage::JsonStorage,
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

const MAX_SAVED_TARGETS: usize = 5;
const MAX_HISTORY_ENTRIES: usize = 10;

/// Saved sync target with usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTarget {
    pub path: PathBuf,
    pub use_count: u32,
    pub last_used: DateTime<Utc>,
}

/// Persistent sync history entry stored to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentSyncHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub device_path: PathBuf,
    pub summary: SyncHistorySummary,
    pub dry_run: bool,
}

/// In-memory sync history entry (carries full SyncReport for detail view)
#[derive(Debug, Clone)]
pub struct SyncHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub device_path: std::path::PathBuf,
    pub report: SyncReport,
    pub dry_run: bool,
}

/// Entry shown in the directory picker
#[derive(Debug, Clone)]
struct DirectoryEntry {
    path: PathBuf,
    /// Short name for display
    name: String,
    /// Whether the entry can be navigated into
    is_accessible: bool,
    /// True when this entry represents the parent ("..")
    is_parent: bool,
    /// True when this entry is a quick-access shortcut
    is_quick_access: bool,
}

/// Tab in the dry-run preview showing different file categories.
#[derive(Debug, Clone, PartialEq)]
enum PreviewTab {
    ToCopy,
    ToDelete,
    Skipped,
    Errors,
}

impl PreviewTab {
    fn label(&self) -> &str {
        match self {
            PreviewTab::ToCopy => "To Copy",
            PreviewTab::ToDelete => "To Delete",
            PreviewTab::Skipped => "Skipped",
            PreviewTab::Errors => "Errors",
        }
    }

    fn next(&self) -> Self {
        match self {
            PreviewTab::ToCopy => PreviewTab::ToDelete,
            PreviewTab::ToDelete => PreviewTab::Skipped,
            PreviewTab::Skipped => PreviewTab::Errors,
            PreviewTab::Errors => PreviewTab::ToCopy,
        }
    }

    fn prev(&self) -> Self {
        match self {
            PreviewTab::ToCopy => PreviewTab::Errors,
            PreviewTab::ToDelete => PreviewTab::ToCopy,
            PreviewTab::Skipped => PreviewTab::ToDelete,
            PreviewTab::Errors => PreviewTab::Skipped,
        }
    }
}

/// Sync buffer operation mode
enum SyncBufferMode {
    /// Overview: saved targets + sync history
    Overview,
    /// Ranger-style filesystem navigator
    DirectoryPicker {
        current_path: PathBuf,
        /// Flat list: quick-access entries first, then directory contents
        entries: Vec<DirectoryEntry>,
        /// Cursor index within `entries`
        selected: usize,
        /// Scroll offset
        scroll: usize,
    },
    /// Dry-run preview: tabbed file list from a completed dry-run
    DryRunPreview {
        report: SyncReport,
        device_path: PathBuf,
        active_tab: PreviewTab,
        selected: usize,
        scroll: usize,
    },
    /// Live progress view: shows byte-based progress during an active sync
    Progress {
        device_path: PathBuf,
        total_bytes: u64,
        bytes_copied: u64,
        current_file: Option<PathBuf>,
        copied_count: usize,
        deleted_count: usize,
        skipped_count: usize,
        error_count: usize,
        start_time: Instant,
    },
}

/// Buffer for device sync management
pub struct SyncBuffer {
    id: String,
    focused: bool,
    theme: Theme,
    download_manager: Option<Arc<DownloadManager<JsonStorage>>>,

    /// Current UI mode
    mode: SyncBufferMode,

    /// Data directory for persistence (sync_targets.json, sync_history.json)
    data_dir: Option<PathBuf>,

    /// Persisted saved targets (loaded from / written to disk)
    saved_targets: Vec<SyncTarget>,

    /// Currently active sync target path
    active_target: Option<PathBuf>,

    /// Persisted sync history (loaded from / written to disk)
    persistent_history: Vec<PersistentSyncHistoryEntry>,

    /// In-memory history (keeps full SyncReport for detail view)
    sync_history: Vec<SyncHistoryEntry>,

    /// Last sync entry for the status section
    last_sync: Option<SyncHistoryEntry>,

    /// Cursor index across the flat overview list (targets then history)
    selected_index: usize,
}

impl SyncBuffer {
    /// Create a new sync buffer
    pub fn new() -> Self {
        Self {
            id: "sync".to_string(),
            focused: false,
            theme: Theme::default(),
            download_manager: None,
            mode: SyncBufferMode::Overview,
            data_dir: None,
            saved_targets: Vec::new(),
            active_target: None,
            persistent_history: Vec::new(),
            sync_history: Vec::new(),
            last_sync: None,
            selected_index: 0,
        }
    }
}

impl Default for SyncBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncBuffer {
    /// Set download manager
    pub fn set_download_manager(&mut self, dm: Arc<DownloadManager<JsonStorage>>) {
        self.download_manager = Some(dm);
    }

    /// Set data directory and load persisted data
    pub fn set_data_dir(&mut self, dir: PathBuf) {
        self.data_dir = Some(dir);
        self.load_persisted_data();
    }

    /// Set the active sync target (called externally, e.g. after a sync completes)
    pub fn set_active_target(&mut self, path: PathBuf) {
        self.active_target = Some(path);
    }

    /// Return the current active sync target path (if any).
    pub fn active_target(&self) -> Option<&PathBuf> {
        self.active_target.as_ref()
    }

    // ── Persistence ──────────────────────────────────────────────────────────

    /// Load saved targets and history from disk.
    fn load_persisted_data(&mut self) {
        let Some(ref dir) = self.data_dir else {
            return;
        };
        self.saved_targets = Self::load_json(dir.join("sync_targets.json")).unwrap_or_default();
        self.persistent_history =
            Self::load_json(dir.join("sync_history.json")).unwrap_or_default();

        // Restore active target from the most-recently-used saved target
        if self.active_target.is_none() {
            if let Some(t) = self.saved_targets.first() {
                self.active_target = Some(t.path.clone());
            }
        }
    }

    fn load_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_json<T: Serialize>(path: &PathBuf, value: &T) {
        let tmp = path.with_extension("tmp");
        match serde_json::to_string_pretty(value) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&tmp, &json) {
                    eprintln!(
                        "podcast-tui: failed to write sync data to {}: {e}",
                        tmp.display()
                    );
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp, path) {
                    eprintln!(
                        "podcast-tui: failed to persist sync data to {}: {e}",
                        path.display()
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "podcast-tui: failed to serialize sync data for {}: {e}",
                    path.display()
                );
            }
        }
    }

    fn persist_targets(&self) {
        if let Some(ref dir) = self.data_dir {
            Self::save_json(&dir.join("sync_targets.json"), &self.saved_targets);
        }
    }

    fn persist_history(&self) {
        if let Some(ref dir) = self.data_dir {
            Self::save_json(&dir.join("sync_history.json"), &self.persistent_history);
        }
    }

    // ── Sync result handling ─────────────────────────────────────────────────

    /// Record a completed sync operation. Updates saved targets and history.
    pub fn add_sync_result(
        &mut self,
        device_path: std::path::PathBuf,
        report: SyncReport,
        dry_run: bool,
    ) {
        let now = Utc::now();

        // Update or insert into saved targets
        if let Some(existing) = self
            .saved_targets
            .iter_mut()
            .find(|t| t.path == device_path)
        {
            existing.use_count += 1;
            existing.last_used = now;
        } else {
            self.saved_targets.push(SyncTarget {
                path: device_path.clone(),
                use_count: 1,
                last_used: now,
            });
        }

        // Sort by last_used descending and cap at MAX_SAVED_TARGETS
        self.saved_targets
            .sort_by(|a, b| b.last_used.cmp(&a.last_used));
        self.saved_targets.truncate(MAX_SAVED_TARGETS);
        self.persist_targets();

        // Update active target
        self.active_target = Some(device_path.clone());

        // Add to persistent history
        self.persistent_history.insert(
            0,
            PersistentSyncHistoryEntry {
                timestamp: now,
                device_path: device_path.clone(),
                summary: SyncHistorySummary::from(&report),
                dry_run,
            },
        );
        self.persistent_history.truncate(MAX_HISTORY_ENTRIES);
        self.persist_history();

        // In-memory history (full SyncReport)
        let entry = SyncHistoryEntry {
            timestamp: now,
            device_path: device_path.clone(),
            report,
            dry_run,
        };
        self.last_sync = Some(entry.clone());
        self.sync_history.insert(0, entry);
        self.sync_history.truncate(MAX_HISTORY_ENTRIES);
    }

    // ── Overview navigation ──────────────────────────────────────────────────

    /// Total items in the Overview flat list (saved targets + persistent history)
    fn overview_item_count(&self) -> usize {
        self.saved_targets.len() + self.persistent_history.len()
    }

    fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn select_next(&mut self) {
        let max = self.overview_item_count().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    // ── Directory picker ─────────────────────────────────────────────────────

    /// Enter directory picker mode, starting from `active_target` or a platform default.
    pub fn enter_directory_picker(&mut self) {
        let start_path = self
            .active_target
            .clone()
            .unwrap_or_else(Self::platform_default_path);

        let entries = Self::build_entries(&start_path);
        self.mode = SyncBufferMode::DirectoryPicker {
            current_path: start_path,
            entries,
            selected: 0,
            scroll: 0,
        };
    }

    /// Platform-aware starting path for the directory picker.
    fn platform_default_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            // Find first available drive letter
            for letter in b'A'..=b'Z' {
                let p = PathBuf::from(format!("{}:\\", letter as char));
                if p.exists() {
                    return p;
                }
            }
            PathBuf::from("C:\\")
        }
        #[cfg(not(target_os = "windows"))]
        {
            PathBuf::from("/")
        }
    }

    /// Build the flat list of entries for the given directory.
    ///
    /// Layout: quick-access shortcuts first, then `..` (if not at root),
    /// then alphabetically-sorted subdirectories.
    fn build_entries(current: &PathBuf) -> Vec<DirectoryEntry> {
        let mut entries = Vec::new();

        // Quick-access shortcuts
        for path in Self::quick_access_paths() {
            if path.exists() {
                let name = path.to_string_lossy().to_string();
                entries.push(DirectoryEntry {
                    name,
                    path,
                    is_accessible: true,
                    is_parent: false,
                    is_quick_access: true,
                });
            }
        }

        // Parent ".." entry
        if let Some(parent) = current.parent() {
            let parent_path = parent.to_path_buf();
            // Only add ".." when there's a real parent
            if parent_path != *current {
                entries.push(DirectoryEntry {
                    name: "..".to_string(),
                    path: parent_path,
                    is_accessible: true,
                    is_parent: true,
                    is_quick_access: false,
                });
            }
        }

        // Directory contents
        if let Ok(mut read_dir) = std::fs::read_dir(current) {
            let mut dirs: Vec<DirectoryEntry> = Vec::new();
            while let Some(Ok(entry)) = read_dir.next() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                // Skip hidden directories
                if name.starts_with('.') {
                    continue;
                }
                // Check accessibility
                let is_accessible = std::fs::read_dir(&path).is_ok();
                dirs.push(DirectoryEntry {
                    name,
                    path,
                    is_accessible,
                    is_parent: false,
                    is_quick_access: false,
                });
            }
            dirs.sort_by(|a, b| a.name.cmp(&b.name));
            entries.extend(dirs);
        }

        entries
    }

    /// Platform-aware quick-access paths shown at the top of the picker.
    fn quick_access_paths() -> Vec<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            // Enumerate available drive letters
            (b'A'..=b'Z')
                .map(|l| PathBuf::from(format!("{}:\\", l as char)))
                .filter(|p| p.exists())
                .collect()
        }
        #[cfg(target_os = "macos")]
        {
            let mut paths = vec![PathBuf::from("/Volumes")];
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Music"));
            }
            paths
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            // Linux / other Unix
            let mut paths = Vec::new();
            // /media/$USER
            if let Ok(user) = std::env::var("USER") {
                let media_user = PathBuf::from(format!("/media/{}", user));
                if media_user.exists() {
                    paths.push(media_user);
                }
            }
            let mnt = PathBuf::from("/mnt");
            if mnt.exists() {
                paths.push(mnt);
            }
            if let Some(home) = dirs::home_dir() {
                let music = home.join("Music");
                if music.exists() {
                    paths.push(music);
                }
            }
            paths
        }
    }

    /// Navigate the picker into a new directory, rebuilding the entry list.
    fn picker_navigate_to(
        current_path: &mut PathBuf,
        entries: &mut Vec<DirectoryEntry>,
        selected: &mut usize,
        scroll: &mut usize,
        new_path: PathBuf,
    ) {
        *current_path = new_path;
        *entries = Self::build_entries(current_path);
        *selected = 0;
        *scroll = 0;
    }

    // ── Rendering helpers ─────────────────────────────────────────────────────

    fn border_style(&self) -> ratatui::style::Style {
        if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        }
    }

    fn render_overview(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Status / active target
                Constraint::Min(5),    // Flat list (targets + history)
                Constraint::Length(3), // Hints
            ])
            .split(area);

        self.render_overview_status(frame, chunks[0]);
        self.render_overview_list(frame, chunks[1]);
        self.render_overview_hints(frame, chunks[2]);
    }

    fn render_overview_status(&self, frame: &mut Frame, area: Rect) {
        let target_line = self
            .active_target
            .as_ref()
            .map(|p| format!("Active: {}", p.display()))
            .unwrap_or_else(|| "No target set. Press 'p' to pick one.".to_string());

        let last_line = self
            .last_sync
            .as_ref()
            .map(|e| {
                let mode = if e.dry_run { " [DRY]" } else { "" };
                format!(
                    "Last sync{}: {} → copied {}, deleted {}, skipped {}",
                    mode,
                    e.timestamp.format("%Y-%m-%d %H:%M"),
                    e.report.files_copied.len(),
                    e.report.files_deleted.len(),
                    e.report.files_skipped.len()
                )
            })
            .unwrap_or_else(|| {
                self.persistent_history
                    .first()
                    .map(|e| {
                        let mode = if e.dry_run { " [DRY]" } else { "" };
                        format!(
                            "Last sync{}: {} → copied {}, deleted {}, errors {}",
                            mode,
                            e.timestamp.format("%Y-%m-%d %H:%M"),
                            e.summary.files_copied_count,
                            e.summary.files_deleted_count,
                            e.summary.error_count,
                        )
                    })
                    .unwrap_or_else(|| "No syncs yet.".to_string())
            });

        let text = format!("{}\n{}", target_line, last_line);
        let para = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sync Status")
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style())
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
    }

    fn render_overview_list(&self, frame: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();

        // Saved targets section
        if !self.saved_targets.is_empty() {
            items.push(
                ListItem::new(Line::from(vec![Span::styled(
                    "─── Saved Targets (Enter to activate, p to pick new) ───",
                    self.theme.text_style().add_modifier(Modifier::DIM),
                )]))
                .style(self.theme.text_style()),
            );
            for (i, target) in self.saved_targets.iter().enumerate() {
                let active_marker = if self.active_target.as_deref() == Some(&target.path) {
                    "▶ "
                } else {
                    "  "
                };
                let content = format!(
                    "{}{}  (used {}×, last {})",
                    active_marker,
                    target.path.display(),
                    target.use_count,
                    target.last_used.format("%Y-%m-%d"),
                );
                let style = if i == self.selected_index {
                    self.theme.selected_style()
                } else {
                    self.theme.text_style()
                };
                items.push(ListItem::new(content).style(style));
            }
        } else {
            items.push(
                ListItem::new("  No saved targets. Press 'p' to pick a directory.")
                    .style(self.theme.text_style()),
            );
        }

        // History section separator
        items.push(
            ListItem::new(Line::from(vec![Span::styled(
                "─── Sync History ───",
                self.theme.text_style().add_modifier(Modifier::DIM),
            )]))
            .style(self.theme.text_style()),
        );

        if self.persistent_history.is_empty() {
            items.push(ListItem::new("  No sync history yet.").style(self.theme.text_style()));
        } else {
            let target_count = self.saved_targets.len();
            for (i, entry) in self.persistent_history.iter().enumerate() {
                let actual_i = target_count + i;
                let icon = if entry.summary.error_count == 0 {
                    "✓"
                } else {
                    "✗"
                };
                let mode = if entry.dry_run { " [DRY]" } else { "" };
                let content = format!(
                    "{}{} {}  {}  copied:{} deleted:{} errs:{}",
                    icon,
                    mode,
                    entry.timestamp.format("%Y-%m-%d %H:%M"),
                    entry
                        .device_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    entry.summary.files_copied_count,
                    entry.summary.files_deleted_count,
                    entry.summary.error_count,
                );
                let style = if actual_i == self.selected_index {
                    self.theme.selected_style()
                } else {
                    self.theme.text_style()
                };
                items.push(ListItem::new(content).style(style));
            }
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Targets & History")
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(list, area);
    }

    fn render_overview_hints(&self, frame: &mut Frame, area: Rect) {
        let has_targets = !self.saved_targets.is_empty();
        let text = if has_targets {
            "↑↓ navigate  s sync  Enter activate target  d dry-run  p pick dir  r refresh"
        } else {
            "s sync (prompts for path)  d dry-run  p pick directory  r refresh"
        };
        let para = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Actions")
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(para, area);
    }

    fn render_directory_picker(
        &self,
        frame: &mut Frame,
        area: Rect,
        current_path: &Path,
        entries: &[DirectoryEntry],
        selected: usize,
        scroll: usize,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Current path header
                Constraint::Min(5),    // Directory listing
                Constraint::Length(3), // Hints
            ])
            .split(area);

        // Header: current path
        let header = Paragraph::new(format!("  📍 {}", current_path.display()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Sync Target")
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(header, chunks[0]);

        // Listing
        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let end = (scroll + visible_height).min(entries.len());
        let visible = &entries[scroll..end];

        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(vi, entry)| {
                let actual_i = scroll + vi;
                let icon = if entry.is_quick_access {
                    "⚡"
                } else if entry.is_parent {
                    "↑"
                } else if entry.is_accessible {
                    "📁"
                } else {
                    "🔒"
                };
                let suffix = if !entry.is_accessible {
                    " [no access]"
                } else {
                    "/"
                };
                let label = format!("  {} {}{}", icon, entry.name, suffix);
                let style = if actual_i == selected {
                    self.theme.selected_style()
                } else {
                    self.theme.text_style()
                };
                ListItem::new(label).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} items", entries.len()))
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(list, chunks[1]);

        // Hints
        let hints =
            Paragraph::new("↑↓ move  → enter dir  ← parent  Enter select current dir  Esc cancel")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Navigation")
                        .border_style(self.border_style()),
                )
                .style(self.theme.text_style());
        frame.render_widget(hints, chunks[2]);
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

    fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    fn can_close(&self) -> bool {
        true
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Sync Buffer Help".to_string(),
            "".to_string(),
            "Overview:".to_string(),
            "  ↑/↓         Navigate saved targets / history".to_string(),
            "  Enter       Activate selected target (set as next sync destination)".to_string(),
            "  s           Start sync to active target".to_string(),
            "  d           Dry-run preview (show changes without applying)".to_string(),
            "  p           Open directory picker to pick a sync target".to_string(),
            "  r           Refresh / reload sync status".to_string(),
            "".to_string(),
            "Directory Picker:".to_string(),
            "  ↑/↓         Move selection".to_string(),
            "  →           Navigate INTO highlighted directory".to_string(),
            "  ←           Go UP to parent directory".to_string(),
            "  Enter       Select current directory as sync target".to_string(),
            "  Esc         Cancel — return to Overview".to_string(),
            "".to_string(),
            "Dry-Run Preview (after pressing d):".to_string(),
            "  [/]         Cycle between To Copy / To Delete / Skipped / Errors tabs".to_string(),
            "  ↑/↓         Scroll file list within the active tab".to_string(),
            "  Enter / s   Confirm — start real sync to the previewed target".to_string(),
            "  Esc         Cancel — return to Overview without syncing".to_string(),
            "".to_string(),
            "Progress View (during active sync):".to_string(),
            "  (read-only) Byte-based progress bar + live counters + elapsed time".to_string(),
            "              Automatically transitions back to Overview when sync completes"
                .to_string(),
            "".to_string(),
            "  C-h / F1    Show this help".to_string(),
        ]
    }
}

impl UIComponent for SyncBuffer {
    fn handle_action(&mut self, action: UIAction) -> UIAction {
        match &mut self.mode {
            SyncBufferMode::DirectoryPicker {
                ref mut current_path,
                ref mut entries,
                ref mut selected,
                ref mut scroll,
            } => {
                match action {
                    UIAction::MoveUp => {
                        if *selected > 0 {
                            *selected -= 1;
                            if *selected < *scroll {
                                *scroll = *selected;
                            }
                        }
                        UIAction::Render
                    }
                    UIAction::MoveDown => {
                        if *selected + 1 < entries.len() {
                            *selected += 1;
                            let min_scroll = selected.saturating_sub(20);
                            if *scroll < min_scroll {
                                *scroll = min_scroll;
                            }
                        }
                        UIAction::Render
                    }
                    UIAction::PageUp => {
                        *selected = selected.saturating_sub(10);
                        if *selected < *scroll {
                            *scroll = *selected;
                        }
                        UIAction::Render
                    }
                    UIAction::PageDown => {
                        let max = entries.len().saturating_sub(1);
                        *selected = (*selected + 10).min(max);
                        let min_scroll = selected.saturating_sub(20);
                        if *scroll < min_scroll {
                            *scroll = min_scroll;
                        }
                        UIAction::Render
                    }
                    UIAction::MoveRight | UIAction::SelectItem => {
                        // Navigate INTO the highlighted directory (MoveRight),
                        // or if the user pressed Enter and there's a selected entry,
                        // navigate into it IF it's a quick-access entry.
                        // For Enter on the current_path itself, handled below via HideMinibuffer path.
                        let sel = *selected;
                        if let Some(entry) = entries.get(sel).cloned() {
                            if action == UIAction::SelectItem {
                                if entry.is_quick_access || entry.is_parent {
                                    // Navigate to it (don't select as target yet)
                                    if entry.is_accessible {
                                        let new_path = entry.path.clone();
                                        Self::picker_navigate_to(
                                            current_path,
                                            entries,
                                            selected,
                                            scroll,
                                            new_path,
                                        );
                                    } else {
                                        return UIAction::ShowMessage(
                                            "Directory is not accessible".to_string(),
                                        );
                                    }
                                } else {
                                    // Enter on a regular dir entry: SELECT the current_path
                                    let path = current_path.clone();
                                    self.active_target = Some(path.clone());
                                    self.upsert_saved_target(path.clone());
                                    self.mode = SyncBufferMode::Overview;
                                    return UIAction::ShowMessage(format!(
                                        "Sync target set to {}  (use → to enter a subdirectory first, then Enter to select)",
                                        path.display()
                                    ));
                                }
                            } else {
                                // MoveRight → always navigate into entry
                                if entry.is_accessible {
                                    let new_path = entry.path.clone();
                                    Self::picker_navigate_to(
                                        current_path,
                                        entries,
                                        selected,
                                        scroll,
                                        new_path,
                                    );
                                } else {
                                    return UIAction::ShowMessage(
                                        "Directory is not accessible".to_string(),
                                    );
                                }
                            }
                        } else if action == UIAction::SelectItem {
                            // No entries (empty dir): select current_path as target
                            let path = current_path.clone();
                            self.active_target = Some(path.clone());
                            self.upsert_saved_target(path.clone());
                            self.mode = SyncBufferMode::Overview;
                            return UIAction::ShowMessage(format!(
                                "Sync target set to {}",
                                path.display()
                            ));
                        }
                        UIAction::Render
                    }
                    UIAction::MoveLeft => {
                        // Navigate to parent
                        if let Some(parent) = current_path.parent().map(|p| p.to_path_buf()) {
                            if parent != *current_path {
                                let new_path = parent;
                                Self::picker_navigate_to(
                                    current_path,
                                    entries,
                                    selected,
                                    scroll,
                                    new_path,
                                );
                            }
                        }
                        UIAction::Render
                    }
                    UIAction::HideMinibuffer => {
                        // Esc: cancel picker, return to Overview
                        self.mode = SyncBufferMode::Overview;
                        UIAction::Render
                    }
                    _ => UIAction::Render,
                }
            }
            SyncBufferMode::Overview => match action {
                UIAction::MoveUp => {
                    self.select_previous();
                    UIAction::Render
                }
                UIAction::MoveDown => {
                    self.select_next();
                    UIAction::Render
                }
                UIAction::PageUp => {
                    self.selected_index = self.selected_index.saturating_sub(10);
                    UIAction::Render
                }
                UIAction::PageDown => {
                    let max = self.overview_item_count().saturating_sub(1);
                    self.selected_index = (self.selected_index + 10).min(max);
                    UIAction::Render
                }
                UIAction::SelectItem => {
                    let i = self.selected_index;
                    let target_count = self.saved_targets.len();
                    if i < target_count {
                        // Activate the selected saved target
                        let path = self.saved_targets[i].path.clone();
                        self.active_target = Some(path.clone());
                        UIAction::ShowMessage(format!("Active sync target: {}", path.display()))
                    } else {
                        // Show details of selected history entry
                        let history_i = i - target_count;
                        if let Some(entry) = self.persistent_history.get(history_i) {
                            let mode = if entry.dry_run { " [DRY RUN]" } else { "" };
                            let details = format!(
                                "Sync to {} at {}{}\nCopied: {}  Deleted: {}  Skipped: {}  Errors: {}",
                                entry.device_path.display(),
                                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                                mode,
                                entry.summary.files_copied_count,
                                entry.summary.files_deleted_count,
                                entry.summary.files_skipped_count,
                                entry.summary.error_count,
                            );
                            UIAction::ShowMinibuffer(details)
                        } else {
                            UIAction::ShowMessage("No item selected".to_string())
                        }
                    }
                }
                UIAction::SyncToDevice => {
                    if let Some(ref path) = self.active_target.clone() {
                        // Trigger sync directly with the active target — no prompt needed
                        UIAction::TriggerDeviceSync {
                            device_path: path.clone(),
                            delete_orphans: false,
                            dry_run: false,
                        }
                    } else {
                        // No saved target: prompt the user
                        UIAction::PromptInput("Sync to device path: ".to_string())
                    }
                }
                _ => UIAction::None,
            },
            SyncBufferMode::DryRunPreview {
                ref report,
                ref device_path,
                ref mut active_tab,
                ref mut selected,
                ref mut scroll,
            } => match action {
                UIAction::MoveUp => {
                    *selected = selected.saturating_sub(1);
                    if *selected < *scroll {
                        *scroll = *selected;
                    }
                    UIAction::Render
                }
                UIAction::MoveDown => {
                    let max = current_tab_len(report, active_tab).saturating_sub(1);
                    if *selected < max {
                        *selected += 1;
                        let min_scroll = selected.saturating_sub(20);
                        if *scroll < min_scroll {
                            *scroll = min_scroll;
                        }
                    }
                    UIAction::Render
                }
                UIAction::PageUp => {
                    *selected = selected.saturating_sub(10);
                    if *selected < *scroll {
                        *scroll = *selected;
                    }
                    UIAction::Render
                }
                UIAction::PageDown => {
                    let max = current_tab_len(report, active_tab).saturating_sub(1);
                    *selected = (*selected + 10).min(max);
                    let min_scroll = selected.saturating_sub(20);
                    if *scroll < min_scroll {
                        *scroll = min_scroll;
                    }
                    UIAction::Render
                }
                UIAction::PreviousTab => {
                    *active_tab = active_tab.prev();
                    *selected = 0;
                    *scroll = 0;
                    UIAction::Render
                }
                UIAction::NextTab => {
                    *active_tab = active_tab.next();
                    *selected = 0;
                    *scroll = 0;
                    UIAction::Render
                }
                UIAction::SelectItem | UIAction::SyncToDevice => {
                    // Enter or 's' from preview → start real sync
                    let path = device_path.clone();
                    UIAction::TriggerDeviceSync {
                        device_path: path,
                        delete_orphans: false,
                        dry_run: false,
                    }
                }
                UIAction::HideMinibuffer => {
                    // Esc → back to Overview
                    self.mode = SyncBufferMode::Overview;
                    UIAction::Render
                }
                _ => UIAction::Render,
            },
            SyncBufferMode::Progress { .. } => UIAction::Render,
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
        // Extract picker state before the borrow checker makes life difficult
        match &self.mode {
            SyncBufferMode::DirectoryPicker {
                ref current_path,
                ref entries,
                selected,
                scroll,
            } => {
                let cp = current_path.clone();
                let ents: Vec<DirectoryEntry> = entries.clone();
                let sel = *selected;
                let scr = *scroll;
                self.render_directory_picker(frame, area, &cp, &ents, sel, scr);
            }
            SyncBufferMode::DryRunPreview { .. } => {
                self.render_dry_run_preview(frame, area);
            }
            SyncBufferMode::Progress { .. } => {
                self.render_progress(frame, area);
            }
            SyncBufferMode::Overview => {
                self.render_overview(frame, area);
            }
        }
    }
}

impl SyncBuffer {
    /// Insert or update a saved target, capping at MAX_SAVED_TARGETS.
    fn upsert_saved_target(&mut self, path: PathBuf) {
        let now = Utc::now();
        if let Some(existing) = self.saved_targets.iter_mut().find(|t| t.path == path) {
            existing.use_count += 1;
            existing.last_used = now;
        } else {
            self.saved_targets.push(SyncTarget {
                path,
                use_count: 1,
                last_used: now,
            });
        }
        self.saved_targets
            .sort_by(|a, b| b.last_used.cmp(&a.last_used));
        self.saved_targets.truncate(MAX_SAVED_TARGETS);
        self.persist_targets();
    }

    /// Switch to DryRunPreview mode with the given report.
    pub fn enter_dry_run_preview(&mut self, device_path: PathBuf, report: SyncReport) {
        self.mode = SyncBufferMode::DryRunPreview {
            report,
            device_path,
            active_tab: PreviewTab::ToCopy,
            selected: 0,
            scroll: 0,
        };
    }

    /// Switch to Progress mode for an in-progress sync.
    pub fn enter_progress_mode(&mut self, device_path: PathBuf) {
        self.mode = SyncBufferMode::Progress {
            device_path,
            total_bytes: 0,
            bytes_copied: 0,
            current_file: None,
            copied_count: 0,
            deleted_count: 0,
            skipped_count: 0,
            error_count: 0,
            start_time: Instant::now(),
        };
    }

    /// Return `true` if currently in Progress mode.
    pub fn is_in_progress_mode(&self) -> bool {
        matches!(self.mode, SyncBufferMode::Progress { .. })
    }

    /// Return `true` if currently in DryRunPreview mode.
    pub fn is_in_dry_run_preview_mode(&self) -> bool {
        matches!(self.mode, SyncBufferMode::DryRunPreview { .. })
    }

    /// Return to Overview mode (e.g., on sync error).
    pub fn reset_to_overview(&mut self) {
        self.mode = SyncBufferMode::Overview;
    }

    /// Update Progress mode state from an incoming `SyncProgressEvent`.
    ///
    /// Returns `true` if the sync has completed (caller should handle transition).
    pub fn handle_progress_event(&mut self, event: SyncProgressEvent) -> bool {
        if let SyncBufferMode::Progress {
            ref mut total_bytes,
            ref mut bytes_copied,
            ref mut current_file,
            ref mut copied_count,
            ref mut deleted_count,
            ref mut skipped_count,
            ref mut error_count,
            ..
        } = self.mode
        {
            match event {
                SyncProgressEvent::ScanComplete {
                    total_bytes: tb, ..
                } => {
                    *total_bytes = tb;
                }
                SyncProgressEvent::FileCopied { path, bytes } => {
                    *bytes_copied += bytes;
                    *copied_count += 1;
                    *current_file = Some(path);
                }
                SyncProgressEvent::FileDeleted { .. } => {
                    *deleted_count += 1;
                }
                SyncProgressEvent::FileSkipped { .. } => {
                    *skipped_count += 1;
                }
                SyncProgressEvent::Error { .. } => {
                    *error_count += 1;
                }
                SyncProgressEvent::Complete { .. } => {
                    return true;
                }
            }
        }
        false
    }

    // ── Dry-run preview rendering ─────────────────────────────────────────────

    fn render_dry_run_preview(&self, frame: &mut Frame, area: Rect) {
        let (report, device_path, active_tab, selected, scroll) =
            if let SyncBufferMode::DryRunPreview {
                ref report,
                ref device_path,
                ref active_tab,
                selected,
                scroll,
            } = self.mode
            {
                (report, device_path, active_tab, selected, scroll)
            } else {
                return;
            };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Summary bar
                Constraint::Length(3), // Tab bar
                Constraint::Min(5),    // File list
                Constraint::Length(3), // Hints
            ])
            .split(area);

        // Summary bar
        let total_copy_bytes: u64 = report
            .files_copied
            .iter()
            .filter_map(|p| report.file_sizes.get(p))
            .sum();
        let summary = format!(
            "📋 {} to copy ({})  ·  🗑️  {} to delete  ·  ⏭ {} skip  ·  ⚠ {} errors",
            report.files_copied.len(),
            format_bytes(total_copy_bytes),
            report.files_deleted.len(),
            report.files_skipped.len(),
            report.errors.len(),
        );
        let summary_para = Paragraph::new(summary)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Sync Preview → {}", device_path.display()))
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(summary_para, chunks[0]);

        // Tab bar
        let tabs = [
            PreviewTab::ToCopy,
            PreviewTab::ToDelete,
            PreviewTab::Skipped,
            PreviewTab::Errors,
        ];
        let tab_spans: Vec<Span> = tabs
            .iter()
            .flat_map(|tab| {
                let label = if tab == active_tab {
                    format!("[{} ▾]", tab.label())
                } else {
                    format!("[{}]", tab.label())
                };
                let style = if tab == active_tab {
                    self.theme
                        .text_style()
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED)
                } else {
                    self.theme.text_style().add_modifier(Modifier::DIM)
                };
                vec![Span::styled(label, style), Span::raw("  ")]
            })
            .collect();
        let tab_line = Line::from(tab_spans);
        let tab_para = Paragraph::new(tab_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("[/] switch tab")
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(tab_para, chunks[1]);

        // File list for active tab
        let (entries, total_bytes) = match active_tab {
            PreviewTab::ToCopy => {
                let total: u64 = report
                    .files_copied
                    .iter()
                    .filter_map(|p| report.file_sizes.get(p))
                    .sum();
                let items: Vec<String> = report
                    .files_copied
                    .iter()
                    .map(|p| {
                        let size = report
                            .file_sizes
                            .get(p)
                            .map(|b| format_bytes(*b))
                            .unwrap_or_default();
                        format!("📄 {}  {}", p.display(), size)
                    })
                    .collect();
                (items, Some(total))
            }
            PreviewTab::ToDelete => (
                report
                    .files_deleted
                    .iter()
                    .map(|p| format!("🗑️  {}", p.display()))
                    .collect(),
                None,
            ),
            PreviewTab::Skipped => (
                report
                    .files_skipped
                    .iter()
                    .map(|p| format!("⏭  {}", p.display()))
                    .collect(),
                None,
            ),
            PreviewTab::Errors => (
                report
                    .errors
                    .iter()
                    .map(|(p, e)| format!("⚠  {}  {}", p.display(), e))
                    .collect(),
                None,
            ),
        };

        let visible_height = chunks[2].height.saturating_sub(2) as usize;
        let end = (scroll + visible_height).min(entries.len());
        let visible = if entries.is_empty() {
            vec!["  (none)".to_string()]
        } else {
            entries[scroll..end].to_vec()
        };
        let list_items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(vi, line)| {
                let actual_i = scroll + vi;
                let style = if actual_i == selected {
                    self.theme.selected_style()
                } else {
                    self.theme.text_style()
                };
                ListItem::new(line.as_str()).style(style)
            })
            .collect();
        let footer = if let Some(tb) = total_bytes {
            format!("Total: {}  ({} files)", format_bytes(tb), entries.len())
        } else {
            format!("{} files", entries.len())
        };
        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(footer)
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(list, chunks[2]);

        // Hints
        let hints =
            Paragraph::new("↑↓/j/k scroll  [/] cycle tabs  Enter/s confirm & sync  Esc cancel")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Actions")
                        .border_style(self.border_style()),
                )
                .style(self.theme.text_style());
        frame.render_widget(hints, chunks[3]);
    }

    // ── Progress view rendering ──────────────────────────────────────────────

    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        let (
            device_path,
            total_bytes,
            bytes_copied,
            current_file,
            copied,
            deleted,
            skipped,
            errors,
            start_time,
        ) = if let SyncBufferMode::Progress {
            ref device_path,
            total_bytes,
            bytes_copied,
            ref current_file,
            copied_count,
            deleted_count,
            skipped_count,
            error_count,
            ref start_time,
        } = self.mode
        {
            (
                device_path,
                total_bytes,
                bytes_copied,
                current_file,
                copied_count,
                deleted_count,
                skipped_count,
                error_count,
                start_time,
            )
        } else {
            return;
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Progress bar
                Constraint::Length(3), // Current file
                Constraint::Length(3), // Counters
                Constraint::Min(1),    // Status / padding
            ])
            .split(area);

        // Progress bar
        let pct = if total_bytes > 0 {
            ((bytes_copied as f64 / total_bytes as f64) * 100.0).min(100.0) as u16
        } else {
            0
        };
        let bar_label = format!(
            "{}%   {} / {}",
            pct,
            format_bytes(bytes_copied),
            format_bytes(total_bytes)
        );
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Syncing → {}", device_path.display()))
                    .border_style(self.border_style()),
            )
            .gauge_style(self.theme.selected_style())
            .percent(pct)
            .label(bar_label);
        frame.render_widget(gauge, chunks[0]);

        // Current file
        let current_label = current_file
            .as_ref()
            .map(|p| format!("Currently:  {}", p.display()))
            .unwrap_or_else(|| "Scanning…".to_string());
        let current_para = Paragraph::new(current_label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style());
        frame.render_widget(current_para, chunks[1]);

        // Counters + elapsed
        let elapsed = start_time.elapsed();
        let elapsed_str = if elapsed.as_secs() >= 3600 {
            format!(
                "{}h {}m {}s",
                elapsed.as_secs() / 3600,
                (elapsed.as_secs() % 3600) / 60,
                elapsed.as_secs() % 60
            )
        } else {
            format!("{}m {}s", elapsed.as_secs() / 60, elapsed.as_secs() % 60)
        };
        let counters = format!(
            "✅ Copied: {}    🗑️ Deleted: {}    ⏭ Skipped: {}    ❌ Errors: {}    Elapsed: {}",
            copied, deleted, skipped, errors, elapsed_str
        );
        let counters_para = Paragraph::new(counters)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style())
            .wrap(Wrap { trim: true });
        frame.render_widget(counters_para, chunks[2]);

        // Status
        let status_para = Paragraph::new("  (sync in progress…)")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.border_style()),
            )
            .style(self.theme.text_style().add_modifier(Modifier::DIM));
        frame.render_widget(status_para, chunks[3]);
    }
}

/// Format a byte count as a human-readable string (e.g. "1.5 MB").
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Return the number of items in the active tab (for scroll clamping).
fn current_tab_len(report: &SyncReport, tab: &PreviewTab) -> usize {
    match tab {
        PreviewTab::ToCopy => report.files_copied.len(),
        PreviewTab::ToDelete => report.files_deleted.len(),
        PreviewTab::Skipped => report.files_skipped.len(),
        PreviewTab::Errors => report.errors.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{UIAction, UIComponent};

    // ── SyncTarget serialization ──────────────────────────────────────────────

    #[test]
    fn test_sync_target_serialization_roundtrip() {
        // Arrange
        let target = SyncTarget {
            path: PathBuf::from("/media/usb"),
            use_count: 5,
            last_used: Utc::now(),
        };

        // Act
        let json = serde_json::to_string(&target).unwrap();
        let restored: SyncTarget = serde_json::from_str(&json).unwrap();

        // Assert
        assert_eq!(restored.path, target.path);
        assert_eq!(restored.use_count, target.use_count);
    }

    #[test]
    fn test_persistent_history_serialization_roundtrip() {
        // Arrange
        let entry = PersistentSyncHistoryEntry {
            timestamp: Utc::now(),
            device_path: PathBuf::from("/mnt/device"),
            summary: SyncHistorySummary {
                files_copied_count: 3,
                files_deleted_count: 1,
                files_skipped_count: 10,
                error_count: 0,
                errors: vec![],
            },
            dry_run: false,
        };

        // Act
        let json = serde_json::to_string(&entry).unwrap();
        let restored: PersistentSyncHistoryEntry = serde_json::from_str(&json).unwrap();

        // Assert
        assert_eq!(restored.device_path, entry.device_path);
        assert_eq!(restored.summary.files_copied_count, 3);
        assert!(!restored.dry_run);
    }

    // ── Overview navigation ───────────────────────────────────────────────────

    #[test]
    fn test_sync_buffer_navigation_returns_render() {
        // Arrange
        let mut buffer = SyncBuffer::new();

        // Act / Assert
        assert_eq!(buffer.handle_action(UIAction::MoveUp), UIAction::Render);
        assert_eq!(buffer.handle_action(UIAction::MoveDown), UIAction::Render);
        assert_eq!(buffer.handle_action(UIAction::PageUp), UIAction::Render);
        assert_eq!(buffer.handle_action(UIAction::PageDown), UIAction::Render);
    }

    #[test]
    fn test_select_item_on_saved_target_activates_it() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let path = PathBuf::from("/media/usb");
        buffer.saved_targets.push(SyncTarget {
            path: path.clone(),
            use_count: 1,
            last_used: Utc::now(),
        });
        buffer.selected_index = 0;

        // Act
        let action = buffer.handle_action(UIAction::SelectItem);

        // Assert — should show a ShowMessage with the path and set active_target
        assert_eq!(buffer.active_target, Some(path.clone()));
        match action {
            UIAction::ShowMessage(msg) => assert!(msg.contains("usb")),
            other => panic!("Expected ShowMessage, got {:?}", other),
        }
    }

    #[test]
    fn test_select_item_empty_list_returns_message() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.selected_index = 0;

        // Act
        let action = buffer.handle_action(UIAction::SelectItem);

        // Assert — no targets, no history → "No item selected"
        assert_eq!(
            action,
            UIAction::ShowMessage("No item selected".to_string())
        );
    }

    // ── SyncToDevice routing ──────────────────────────────────────────────────

    #[test]
    fn test_sync_to_device_with_active_target_returns_trigger() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let path = PathBuf::from("/media/player");
        buffer.active_target = Some(path.clone());

        // Act
        let action = buffer.handle_action(UIAction::SyncToDevice);

        // Assert — should trigger sync directly (no prompt)
        match action {
            UIAction::TriggerDeviceSync {
                device_path,
                dry_run,
                ..
            } => {
                assert_eq!(device_path, path);
                assert!(!dry_run);
            }
            other => panic!("Expected TriggerDeviceSync, got {:?}", other),
        }
    }

    #[test]
    fn test_sync_to_device_without_active_target_prompts() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        assert!(buffer.active_target.is_none());

        // Act
        let action = buffer.handle_action(UIAction::SyncToDevice);

        // Assert — no active target → prompt the user
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

    // ── add_sync_result ───────────────────────────────────────────────────────

    #[test]
    fn test_add_sync_result_creates_saved_target() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let path = PathBuf::from("/media/usb");
        let report = SyncReport::new();

        // Act
        buffer.add_sync_result(path.clone(), report, false);

        // Assert
        assert_eq!(buffer.saved_targets.len(), 1);
        assert_eq!(buffer.saved_targets[0].path, path);
        assert_eq!(buffer.saved_targets[0].use_count, 1);
        assert_eq!(buffer.active_target, Some(path));
    }

    #[test]
    fn test_add_sync_result_increments_use_count_for_existing_target() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let path = PathBuf::from("/media/usb");
        let report = SyncReport::new();

        // Act
        buffer.add_sync_result(path.clone(), report.clone(), false);
        buffer.add_sync_result(path.clone(), report, false);

        // Assert
        assert_eq!(buffer.saved_targets.len(), 1);
        assert_eq!(buffer.saved_targets[0].use_count, 2);
    }

    #[test]
    fn test_add_sync_result_caps_targets_at_max() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let report = SyncReport::new();

        // Act — add 7 different targets
        for i in 0..7usize {
            buffer.add_sync_result(
                PathBuf::from(format!("/media/usb{}", i)),
                report.clone(),
                false,
            );
        }

        // Assert — capped at MAX_SAVED_TARGETS (5)
        assert_eq!(buffer.saved_targets.len(), MAX_SAVED_TARGETS);
    }

    #[test]
    fn test_add_sync_result_records_persistent_history() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let path = PathBuf::from("/media/usb");
        let report = SyncReport::new();

        // Act
        buffer.add_sync_result(path, report, true);

        // Assert
        assert_eq!(buffer.persistent_history.len(), 1);
        assert!(buffer.persistent_history[0].dry_run);
    }

    // ── Directory picker mode ─────────────────────────────────────────────────

    #[test]
    fn test_picker_esc_returns_to_overview() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_directory_picker();
        assert!(matches!(
            buffer.mode,
            SyncBufferMode::DirectoryPicker { .. }
        ));

        // Act
        let action = buffer.handle_action(UIAction::HideMinibuffer);

        // Assert
        assert_eq!(action, UIAction::Render);
        assert!(matches!(buffer.mode, SyncBufferMode::Overview));
    }

    // ── PreviewTab cycling ────────────────────────────────────────────────────

    #[test]
    fn test_preview_tab_cycle_next_wraps() {
        // Arrange / Act / Assert — full forward cycle
        assert_eq!(PreviewTab::ToCopy.next(), PreviewTab::ToDelete);
        assert_eq!(PreviewTab::ToDelete.next(), PreviewTab::Skipped);
        assert_eq!(PreviewTab::Skipped.next(), PreviewTab::Errors);
        assert_eq!(PreviewTab::Errors.next(), PreviewTab::ToCopy);
    }

    #[test]
    fn test_preview_tab_cycle_prev_wraps() {
        // Arrange / Act / Assert — full backward cycle
        assert_eq!(PreviewTab::ToCopy.prev(), PreviewTab::Errors);
        assert_eq!(PreviewTab::Errors.prev(), PreviewTab::Skipped);
        assert_eq!(PreviewTab::Skipped.prev(), PreviewTab::ToDelete);
        assert_eq!(PreviewTab::ToDelete.prev(), PreviewTab::ToCopy);
    }

    #[test]
    fn test_dry_run_preview_mode_tab_cycling() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_dry_run_preview(PathBuf::from("/media/usb"), SyncReport::new());
        assert!(matches!(
            buffer.mode,
            SyncBufferMode::DryRunPreview {
                active_tab: PreviewTab::ToCopy,
                ..
            }
        ));

        // Act — advance tabs via NextTab
        buffer.handle_action(UIAction::NextTab);
        assert!(matches!(
            buffer.mode,
            SyncBufferMode::DryRunPreview {
                active_tab: PreviewTab::ToDelete,
                ..
            }
        ));

        // Act — go back via PreviousTab
        buffer.handle_action(UIAction::PreviousTab);
        assert!(matches!(
            buffer.mode,
            SyncBufferMode::DryRunPreview {
                active_tab: PreviewTab::ToCopy,
                ..
            }
        ));
    }

    #[test]
    fn test_dry_run_preview_esc_returns_to_overview() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_dry_run_preview(PathBuf::from("/media/usb"), SyncReport::new());

        // Act
        let action = buffer.handle_action(UIAction::HideMinibuffer);

        // Assert
        assert_eq!(action, UIAction::Render);
        assert!(matches!(buffer.mode, SyncBufferMode::Overview));
    }

    #[test]
    fn test_dry_run_preview_enter_triggers_sync() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        let path = PathBuf::from("/media/usb");
        buffer.enter_dry_run_preview(path.clone(), SyncReport::new());

        // Act — pressing Enter from preview should start real sync
        let action = buffer.handle_action(UIAction::SelectItem);

        // Assert
        match action {
            UIAction::TriggerDeviceSync {
                device_path,
                dry_run,
                ..
            } => {
                assert_eq!(device_path, path);
                assert!(!dry_run, "Confirming from preview must use dry_run=false");
            }
            other => panic!("Expected TriggerDeviceSync, got {:?}", other),
        }
    }

    // ── Progress mode ─────────────────────────────────────────────────────────

    #[test]
    fn test_progress_mode_enter_and_mode_flag() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        assert!(!buffer.is_in_progress_mode());

        // Act
        buffer.enter_progress_mode(PathBuf::from("/media/usb"));

        // Assert
        assert!(buffer.is_in_progress_mode());
    }

    #[test]
    fn test_progress_handle_scan_complete_sets_total_bytes() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_progress_mode(PathBuf::from("/media/usb"));

        // Act
        let complete = buffer.handle_progress_event(SyncProgressEvent::ScanComplete {
            total_bytes: 1_000_000,
            total_files: 5,
        });

        // Assert
        assert!(!complete, "ScanComplete should not signal completion");
        if let SyncBufferMode::Progress { total_bytes, .. } = &buffer.mode {
            assert_eq!(*total_bytes, 1_000_000);
        } else {
            panic!("Expected Progress mode");
        }
    }

    #[test]
    fn test_progress_handle_file_copied_increments_counters() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_progress_mode(PathBuf::from("/media/usb"));

        // Act
        buffer.handle_progress_event(SyncProgressEvent::FileCopied {
            path: PathBuf::from("Podcasts/test.mp3"),
            bytes: 500,
        });

        // Assert
        if let SyncBufferMode::Progress {
            bytes_copied,
            copied_count,
            ..
        } = &buffer.mode
        {
            assert_eq!(*bytes_copied, 500);
            assert_eq!(*copied_count, 1);
        } else {
            panic!("Expected Progress mode");
        }
    }

    #[test]
    fn test_progress_handle_complete_returns_true() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_progress_mode(PathBuf::from("/media/usb"));

        // Act
        let complete = buffer.handle_progress_event(SyncProgressEvent::Complete {
            report: SyncReport::new(),
        });

        // Assert
        assert!(complete, "Complete event should return true");
    }

    #[test]
    fn test_progress_reset_to_overview() {
        // Arrange
        let mut buffer = SyncBuffer::new();
        buffer.enter_progress_mode(PathBuf::from("/media/usb"));
        assert!(buffer.is_in_progress_mode());

        // Act
        buffer.reset_to_overview();

        // Assert
        assert!(matches!(buffer.mode, SyncBufferMode::Overview));
        assert!(!buffer.is_in_progress_mode());
    }

    // ── format_bytes ──────────────────────────────────────────────────────────

    #[test]
    fn test_format_bytes_sub_kb() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_kb_range() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_format_bytes_mb_range() {
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
    }

    #[test]
    fn test_format_bytes_gb_range() {
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    }
}
