// NowPlaying buffer — real-time playback status display.
//
// Shows the currently playing episode, progress bar, volume level, and playback
// state. Updates non-blocking from a `watch::Receiver<PlaybackStatus>` on each
// render tick (~60 fps). The AudioManager (~4 Hz) pushes status into the watch
// channel; the buffer just drains whatever is already there.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Frame,
};
use std::time::Duration;
use tokio::sync::watch;

use crate::{
    audio::{PlaybackState, PlaybackStatus},
    ui::{
        buffers::{Buffer, BufferId},
        themes::Theme,
        UIAction, UIComponent,
    },
};

/// Buffer that displays real-time playback information.
///
/// Updated from a `watch::Receiver<PlaybackStatus>` on each render (non-blocking).
/// When the AudioManager is wired in (#141), call `set_status_rx()` to connect it.
pub struct NowPlayingBuffer {
    id: String,
    /// Cached status snapshot; refreshed from the watch channel on each render.
    status: PlaybackStatus,
    /// Watch channel from AudioManager. Default channel (Stopped) used until #141 wires it.
    status_rx: watch::Receiver<PlaybackStatus>,
    /// Optional display strings set from AppEvent::PlaybackStarted (wired in #141).
    episode_title: Option<String>,
    podcast_name: Option<String>,
    focused: bool,
    theme: Theme,
}

impl NowPlayingBuffer {
    /// Create a new NowPlayingBuffer connected to the given watch receiver.
    pub fn new(status_rx: watch::Receiver<PlaybackStatus>) -> Self {
        let status = status_rx.borrow().clone();
        Self {
            id: "now-playing".to_string(),
            status,
            status_rx,
            episode_title: None,
            podcast_name: None,
            focused: false,
            theme: Theme::default(),
        }
    }

    /// Replace the watch receiver (called by #141 when AudioManager is wired in).
    pub fn set_status_rx(&mut self, rx: watch::Receiver<PlaybackStatus>) {
        self.status = rx.borrow().clone();
        self.status_rx = rx;
    }

    /// Set the human-readable episode title and podcast name for display.
    ///
    /// Called from `AppEvent::PlaybackStarted` in app.rs (wired in #141).
    pub fn set_now_playing_info(
        &mut self,
        episode_title: Option<String>,
        podcast_name: Option<String>,
    ) {
        self.episode_title = episode_title;
        self.podcast_name = podcast_name;
    }

    /// Set the theme for this buffer.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Pull the latest status from the watch channel (non-blocking).
    fn refresh_status(&mut self) {
        if self.status_rx.has_changed().unwrap_or(false) {
            self.status = self.status_rx.borrow_and_update().clone();
        }
        // Also clear display info when stopped (no episode loaded)
        if self.status.state == PlaybackState::Stopped && self.status.episode_id.is_none() {
            self.episode_title = None;
            self.podcast_name = None;
        }
    }

    /// Render the stopped / idle state.
    fn render_stopped(&self, frame: &mut Frame, inner: Rect) {
        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  ⏹ No episode playing",
                self.theme.muted_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Open an episode list and press S-P or ⏯ to start playback.",
                self.theme.muted_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  S-P / ⏯  Play/pause  •  +/−  Volume",
                self.theme.help_style(),
            )]),
        ];

        let paragraph = Paragraph::new(lines)
            .style(self.theme.text_style())
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    }

    /// Render the playing / paused state (episode info, progress, volume, hints).
    fn render_playing(&self, frame: &mut Frame, inner: Rect) {
        // We need at least 8 rows to show everything; fall back to compact if smaller.
        if inner.height < 6 {
            self.render_compact(frame, inner);
            return;
        }

        // ── Layout ──────────────────────────────────────────────────────────
        // [0] info section: podcast name, episode title, blank, state + volume  (4 rows)
        // [1] blank                                                              (1 row)
        // [2] progress gauge                                                     (1 row)
        // [3] spacer (fills remaining space)
        // [4] keybinding hints                                                   (1 row)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(inner);

        // ── Info section ────────────────────────────────────────────────────
        let podcast_line = {
            let name = self.podcast_name.as_deref().unwrap_or("—");
            Line::from(vec![
                Span::styled("  📻 ", self.theme.muted_style()),
                Span::styled(name, self.theme.text_style()),
            ])
        };

        let episode_line = {
            let title = self.episode_title.as_deref().unwrap_or("—");
            Line::from(vec![
                Span::styled("  🎵 ", self.theme.muted_style()),
                Span::styled(title, self.theme.primary_style()),
            ])
        };

        let (state_icon, state_text, state_style) = match self.status.state {
            PlaybackState::Playing => ("▶", "Playing", self.theme.success_style()),
            PlaybackState::Paused => ("⏸", "Paused", self.theme.warning_style()),
            PlaybackState::Stopped => ("⏹", "Stopped", self.theme.muted_style()),
        };

        let volume_pct = (self.status.volume * 100.0).round() as u8;
        let filled = (self.status.volume * 10.0).round() as usize;
        let empty = 10usize.saturating_sub(filled);
        let volume_bar = format!(
            "Volume: {}{}  {}%",
            "█".repeat(filled),
            "░".repeat(empty),
            volume_pct
        );

        let state_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(state_icon, state_style.add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(state_text, state_style),
            Span::raw("    "),
            Span::styled(volume_bar, self.theme.subtext_style()),
        ]);

        let info_lines = vec![podcast_line, episode_line, Line::from(""), state_line];
        frame.render_widget(
            Paragraph::new(info_lines).style(self.theme.text_style()),
            chunks[0],
        );

        // ── Progress gauge ───────────────────────────────────────────────────
        let (ratio, label) = progress_ratio_and_label(self.status.position, self.status.duration);
        let gauge = Gauge::default()
            .gauge_style(self.theme.primary_style())
            .ratio(ratio)
            .label(label);
        frame.render_widget(gauge, chunks[2]);

        // ── Keybinding hints ─────────────────────────────────────────────────
        let hints_line = if self.status.position.is_some() {
            "  S-P: play/pause  •  C-←/→: seek ±10s  •  +/−: volume  •  F9: now playing"
        } else {
            "  S-P: play/pause  •  +/−: volume  •  External player active"
        };
        frame.render_widget(
            Paragraph::new(hints_line).style(self.theme.help_style()),
            chunks[4],
        );
    }

    /// Compact rendering for very small areas.
    fn render_compact(&self, frame: &mut Frame, inner: Rect) {
        let (state_icon, state_text, _) = match self.status.state {
            PlaybackState::Playing => ("▶", "Playing", ()),
            PlaybackState::Paused => ("⏸", "Paused", ()),
            PlaybackState::Stopped => ("⏹", "Stopped", ()),
        };
        let title = self.episode_title.as_deref().unwrap_or("No episode");
        let lines = vec![
            Line::from(vec![
                Span::raw(state_icon),
                Span::raw(" "),
                Span::raw(state_text),
            ]),
            Line::from(title.to_string()),
        ];
        frame.render_widget(Paragraph::new(lines).style(self.theme.text_style()), inner);
    }
}

// ── UIComponent impl ─────────────────────────────────────────────────────────

impl UIComponent for NowPlayingBuffer {
    fn handle_action(&mut self, _action: UIAction) -> UIAction {
        // Global keybinding handler catches all playback actions (S-P, C-←, +/−).
        // This buffer does not consume any actions — it just displays state.
        UIAction::None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Non-blocking status refresh from the watch channel.
        self.refresh_status();

        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else {
            self.theme.border_style()
        };

        let block = Block::default()
            .title("Now Playing")
            .borders(Borders::ALL)
            .border_style(border_style)
            .title_style(self.theme.title_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.status.state {
            PlaybackState::Stopped if self.status.episode_id.is_none() => {
                self.render_stopped(frame, inner);
            }
            _ => {
                self.render_playing(frame, inner);
            }
        }
    }

    fn title(&self) -> String {
        "Now Playing".to_string()
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

// ── Buffer impl ──────────────────────────────────────────────────────────────

impl Buffer for NowPlayingBuffer {
    fn id(&self) -> BufferId {
        self.id.clone()
    }

    fn name(&self) -> String {
        "Now Playing".to_string()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn can_close(&self) -> bool {
        // Persistent buffer — not closeable with C-k.
        false
    }

    fn help_text(&self) -> Vec<String> {
        vec![
            "Now Playing Buffer:".to_string(),
            "  S-P / ⏯ / ⏵    Toggle play / pause".to_string(),
            "  C-← / C-→      Seek backward / forward 10s".to_string(),
            "  + / =           Volume up".to_string(),
            "  -               Volume down".to_string(),
            "  (All playback keys work from any buffer)".to_string(),
        ]
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Format a `Duration` as "m:ss" or "h:mm:ss".
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Compute progress ratio (0.0–1.0) and a "pos / dur" label string.
///
/// Returns `(0.0, "—")` when position or duration is unavailable.
fn progress_ratio_and_label(
    position: Option<Duration>,
    duration: Option<Duration>,
) -> (f64, String) {
    match (position, duration) {
        (Some(pos), Some(dur)) if !dur.is_zero() => {
            let ratio = (pos.as_secs_f64() / dur.as_secs_f64()).clamp(0.0, 1.0);
            let label = format!("{} / {}", format_duration(pos), format_duration(dur));
            (ratio, label)
        }
        (Some(pos), None) => (0.0, format!("{} / —", format_duration(pos))),
        _ => (0.0, "— / —".to_string()),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::PlaybackState;

    fn make_buffer() -> NowPlayingBuffer {
        let (_tx, rx) = watch::channel(PlaybackStatus::default());
        NowPlayingBuffer::new(rx)
    }

    #[test]
    fn test_now_playing_buffer_title_returns_now_playing() {
        // Arrange
        let buffer = make_buffer();

        // Act / Assert
        assert_eq!(buffer.title(), "Now Playing");
        assert_eq!(buffer.name(), "Now Playing");
        assert_eq!(buffer.id(), "now-playing");
    }

    #[test]
    fn test_now_playing_buffer_initial_state_is_stopped() {
        // Arrange / Act
        let buffer = make_buffer();

        // Assert — default status is Stopped
        assert_eq!(buffer.status.state, PlaybackState::Stopped);
        assert!(buffer.status.episode_id.is_none());
    }

    #[test]
    fn test_now_playing_buffer_cannot_be_closed() {
        // Arrange / Act
        let buffer = make_buffer();

        // Assert — NowPlaying is a persistent buffer
        assert!(!buffer.can_close());
    }

    #[test]
    fn test_now_playing_buffer_handle_action_returns_none() {
        // Arrange
        let mut buffer = make_buffer();

        // Act — all actions should be pass-through (global handler catches them)
        let result = buffer.handle_action(UIAction::TogglePlayPause);

        // Assert
        assert_eq!(result, UIAction::None);
    }

    #[test]
    fn test_now_playing_buffer_formats_duration_correctly() {
        // Arrange / Act / Assert — seconds only
        assert_eq!(format_duration(Duration::from_secs(75)), "1:15");
        assert_eq!(format_duration(Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(Duration::from_secs(59)), "0:59");
        // Hours
        assert_eq!(format_duration(Duration::from_secs(3661)), "1:01:01");
        assert_eq!(format_duration(Duration::from_secs(7322)), "2:02:02");
    }

    #[test]
    fn test_now_playing_buffer_progress_ratio_clamped() {
        // Arrange — position > duration (shouldn't happen but must not exceed 1.0)
        let pos = Duration::from_secs(100);
        let dur = Duration::from_secs(50);

        // Act
        let (ratio, _) = progress_ratio_and_label(Some(pos), Some(dur));

        // Assert — clamped to 1.0
        assert!((ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_now_playing_buffer_progress_ratio_zero_for_no_data() {
        // Arrange / Act — no position or duration
        let (ratio, label) = progress_ratio_and_label(None, None);

        // Assert
        assert!((ratio - 0.0).abs() < f64::EPSILON);
        assert_eq!(label, "— / —");
    }

    #[test]
    fn test_now_playing_buffer_progress_ratio_normal() {
        // Arrange
        let pos = Duration::from_secs(30);
        let dur = Duration::from_secs(60);

        // Act
        let (ratio, label) = progress_ratio_and_label(Some(pos), Some(dur));

        // Assert
        assert!((ratio - 0.5).abs() < 1e-9);
        assert_eq!(label, "0:30 / 1:00");
    }

    #[test]
    fn test_now_playing_buffer_status_rx_update() {
        // Arrange
        let (tx, rx) = watch::channel(PlaybackStatus::default());
        let mut buffer = NowPlayingBuffer::new(rx);

        // Act — update the watch channel with new status
        let new_status = PlaybackStatus {
            state: PlaybackState::Playing,
            volume: 0.7,
            ..PlaybackStatus::default()
        };
        tx.send(new_status.clone()).unwrap();
        buffer.refresh_status();

        // Assert — buffer picked up new status
        assert_eq!(buffer.status.state, PlaybackState::Playing);
        assert!((buffer.status.volume - 0.7_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_now_playing_buffer_set_now_playing_info() {
        // Arrange
        let mut buffer = make_buffer();

        // Act
        buffer.set_now_playing_info(
            Some("Episode Title".to_string()),
            Some("Podcast Name".to_string()),
        );

        // Assert
        assert_eq!(buffer.episode_title.as_deref(), Some("Episode Title"));
        assert_eq!(buffer.podcast_name.as_deref(), Some("Podcast Name"));
    }

    #[test]
    fn test_now_playing_buffer_info_cleared_when_stopped() {
        // Arrange
        let (tx, rx) = watch::channel(PlaybackStatus::default());
        let mut buffer = NowPlayingBuffer::new(rx);
        buffer.set_now_playing_info(Some("Episode".to_string()), Some("Podcast".to_string()));

        // Act — send a Stopped status with no episode_id
        tx.send(PlaybackStatus {
            state: PlaybackState::Stopped,
            episode_id: None,
            ..PlaybackStatus::default()
        })
        .unwrap();
        buffer.refresh_status();

        // Assert — display info cleared
        assert!(buffer.episode_title.is_none());
        assert!(buffer.podcast_name.is_none());
    }
}
