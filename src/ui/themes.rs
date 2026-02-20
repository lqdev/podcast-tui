// Theme and styling system for the UI
//
// This module provides color schemes and styling options for the TUI,
// supporting different themes like dark mode, light mode, and high contrast.

use ratatui::style::{Color, Modifier, Style};

use crate::ui::UIError;

/// Available color themes
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeType {
    Default,
    Dark,
    Light,
    HighContrast,
    Solarized,
}

/// Color scheme definition
#[derive(Debug, Clone)]
pub struct ColorScheme {
    // Background colors
    pub background: Color,
    pub surface: Color,
    pub overlay: Color,

    // Foreground colors
    pub text: Color,
    pub subtext: Color,
    pub muted: Color,

    // Accent colors
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,

    // UI element colors
    pub border: Color,
    pub border_focused: Color,
    pub selection: Color,
    pub cursor: Color,

    // Status colors
    pub playing: Color,
    pub paused: Color,
    pub downloaded: Color,
    pub downloading: Color,
    pub queued: Color,

    // Indicator colors
    pub active_indicator: Color,
}

/// Theme manager that provides styling for different UI components
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ColorScheme,
}

impl Theme {
    /// Create a new theme with the specified color scheme
    pub fn new(name: String, colors: ColorScheme) -> Self {
        Self { name, colors }
    }

    /// Get the default dark theme
    pub fn default_dark() -> Self {
        let colors = ColorScheme {
            background: Color::Rgb(16, 20, 24),
            surface: Color::Rgb(24, 30, 36),
            overlay: Color::Rgb(32, 40, 48),

            text: Color::Rgb(220, 225, 230),
            subtext: Color::Rgb(180, 190, 200),
            muted: Color::Rgb(120, 130, 140),

            primary: Color::Rgb(88, 166, 255),
            secondary: Color::Rgb(138, 180, 248),
            success: Color::Rgb(102, 187, 106),
            warning: Color::Rgb(255, 193, 7),
            error: Color::Rgb(244, 67, 54),

            border: Color::Rgb(60, 70, 80),
            border_focused: Color::Rgb(88, 166, 255),
            selection: Color::Rgb(44, 82, 130),
            cursor: Color::Rgb(255, 255, 255),

            playing: Color::Rgb(76, 175, 80),
            paused: Color::Rgb(255, 193, 7),
            downloaded: Color::Rgb(33, 150, 243),
            downloading: Color::Rgb(156, 39, 176),
            queued: Color::Rgb(158, 158, 158),

            active_indicator: Color::Rgb(255, 193, 7),
        };

        Self::new("Default Dark".to_string(), colors)
    }

    /// Get a light theme
    pub fn light() -> Self {
        let colors = ColorScheme {
            background: Color::Rgb(255, 255, 255),
            surface: Color::Rgb(248, 249, 250),
            overlay: Color::Rgb(240, 242, 245),

            text: Color::Rgb(33, 37, 41),
            subtext: Color::Rgb(73, 80, 87),
            muted: Color::Rgb(108, 117, 125),

            primary: Color::Rgb(13, 110, 253),
            secondary: Color::Rgb(108, 117, 125),
            success: Color::Rgb(25, 135, 84),
            warning: Color::Rgb(255, 193, 7),
            error: Color::Rgb(220, 53, 69),

            border: Color::Rgb(206, 212, 218),
            border_focused: Color::Rgb(13, 110, 253),
            selection: Color::Rgb(173, 216, 230),
            cursor: Color::Rgb(0, 0, 0),

            playing: Color::Rgb(25, 135, 84),
            paused: Color::Rgb(255, 193, 7),
            downloaded: Color::Rgb(13, 110, 253),
            downloading: Color::Rgb(111, 66, 193),
            queued: Color::Rgb(108, 117, 125),

            active_indicator: Color::Rgb(255, 193, 7),
        };

        Self::new("Light".to_string(), colors)
    }

    /// Get a high contrast theme for accessibility
    pub fn high_contrast() -> Self {
        let colors = ColorScheme {
            background: Color::Black,
            surface: Color::Rgb(20, 20, 20),
            overlay: Color::Rgb(40, 40, 40),

            text: Color::White,
            subtext: Color::Rgb(220, 220, 220),
            muted: Color::Rgb(180, 180, 180),

            primary: Color::Yellow,
            secondary: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,

            border: Color::White,
            border_focused: Color::Yellow,
            selection: Color::Rgb(100, 100, 0),
            cursor: Color::White,

            playing: Color::Green,
            paused: Color::Yellow,
            downloaded: Color::Cyan,
            downloading: Color::Magenta,
            queued: Color::White,

            active_indicator: Color::Yellow,
        };

        Self::new("High Contrast".to_string(), colors)
    }

    /// Get a Solarized-inspired theme
    pub fn solarized() -> Self {
        let colors = ColorScheme {
            background: Color::Rgb(0, 43, 54), // base03
            surface: Color::Rgb(7, 54, 66),    // base02
            overlay: Color::Rgb(88, 110, 117), // base01

            text: Color::Rgb(131, 148, 150),    // base0
            subtext: Color::Rgb(147, 161, 161), // base1
            muted: Color::Rgb(101, 123, 131),   // base00

            primary: Color::Rgb(38, 139, 210),   // blue
            secondary: Color::Rgb(42, 161, 152), // cyan
            success: Color::Rgb(133, 153, 0),    // green
            warning: Color::Rgb(181, 137, 0),    // yellow
            error: Color::Rgb(220, 50, 47),      // red

            border: Color::Rgb(88, 110, 117),         // base01
            border_focused: Color::Rgb(38, 139, 210), // blue
            selection: Color::Rgb(7, 54, 66),         // base02
            cursor: Color::Rgb(147, 161, 161),        // base1

            playing: Color::Rgb(133, 153, 0),      // green
            paused: Color::Rgb(181, 137, 0),       // yellow
            downloaded: Color::Rgb(38, 139, 210),  // blue
            downloading: Color::Rgb(211, 54, 130), // magenta
            queued: Color::Rgb(101, 123, 131),     // base00

            active_indicator: Color::Rgb(181, 137, 0), // yellow
        };

        Self::new("Solarized".to_string(), colors)
    }

    /// Create a theme from a name
    pub fn from_name(name: &str) -> Result<Self, UIError> {
        match name.to_lowercase().as_str() {
            "dark" | "default" => Ok(Self::default_dark()),
            "light" => Ok(Self::light()),
            "high-contrast" | "high_contrast" => Ok(Self::high_contrast()),
            "solarized" => Ok(Self::solarized()),
            _ => Err(UIError::InvalidOperation(format!(
                "Unknown theme: {}",
                name
            ))),
        }
    }

    /// Get default style
    pub fn default_style(&self) -> Style {
        Style::default()
            .bg(self.colors.background)
            .fg(self.colors.text)
    }

    /// Get the theme by type
    pub fn by_type(theme_type: ThemeType) -> Self {
        match theme_type {
            ThemeType::Default => Self::default_dark(),
            ThemeType::Dark => Self::default_dark(),
            ThemeType::Light => Self::light(),
            ThemeType::HighContrast => Self::high_contrast(),
            ThemeType::Solarized => Self::solarized(),
        }
    }

    // Style getters for different UI components

    /// Style for normal text
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.colors.text)
    }

    /// Style for secondary/subtitle text
    pub fn subtext_style(&self) -> Style {
        Style::default().fg(self.colors.subtext)
    }

    /// Style for muted/disabled text
    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.colors.muted)
    }

    /// Style for focused/active elements
    pub fn focused_style(&self) -> Style {
        Style::default()
            .fg(self.colors.text)
            .bg(self.colors.selection)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for selected items
    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.colors.text)
            .bg(self.colors.selection)
    }

    /// Style for borders
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.colors.border)
    }

    /// Style for focused borders
    pub fn border_focused_style(&self) -> Style {
        Style::default().fg(self.colors.border_focused)
    }

    /// Style for primary buttons/actions
    pub fn primary_style(&self) -> Style {
        Style::default()
            .fg(self.colors.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for success messages
    pub fn success_style(&self) -> Style {
        Style::default().fg(self.colors.success)
    }

    /// Style for warning messages
    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.colors.warning)
    }

    /// Style for error messages
    pub fn error_style(&self) -> Style {
        Style::default()
            .fg(self.colors.error)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for status indicators
    pub fn status_style(&self, status: &str) -> Style {
        match status.to_lowercase().as_str() {
            "playing" => Style::default().fg(self.colors.playing),
            "paused" => Style::default().fg(self.colors.paused),
            "downloaded" => Style::default().fg(self.colors.downloaded),
            "downloading" => Style::default().fg(self.colors.downloading),
            "queued" => Style::default().fg(self.colors.queued),
            _ => self.text_style(),
        }
    }

    /// Style for help text
    pub fn help_style(&self) -> Style {
        Style::default()
            .fg(self.colors.subtext)
            .add_modifier(Modifier::ITALIC)
    }

    /// Style for titles/headers
    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.colors.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for the minibuffer
    pub fn minibuffer_style(&self) -> Style {
        Style::default()
            .fg(self.colors.text)
            .bg(self.colors.overlay)
    }

    /// Style for the status bar
    pub fn statusbar_style(&self) -> Style {
        Style::default()
            .fg(self.colors.text)
            .bg(self.colors.surface)
    }

    /// Style for active buffer/item indicators
    pub fn active_indicator_style(&self) -> Style {
        Style::default().fg(self.colors.active_indicator)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_creation() {
        let theme = Theme::default();
        assert_eq!(theme.name, "Default Dark");
    }

    #[test]
    fn test_theme_by_type() {
        let dark = Theme::by_type(ThemeType::Dark);
        assert_eq!(dark.name, "Default Dark");

        let light = Theme::by_type(ThemeType::Light);
        assert_eq!(light.name, "Light");

        let high_contrast = Theme::by_type(ThemeType::HighContrast);
        assert_eq!(high_contrast.name, "High Contrast");
    }

    #[test]
    fn test_style_methods() {
        let theme = Theme::default();

        // Test that styles are created without panicking
        let _ = theme.text_style();
        let _ = theme.focused_style();
        let _ = theme.error_style();
        let _ = theme.status_style("playing");
        let _ = theme.active_indicator_style();
    }

    #[test]
    fn test_status_styles() {
        let theme = Theme::default();

        let playing_style = theme.status_style("playing");
        let paused_style = theme.status_style("paused");
        let unknown_style = theme.status_style("unknown");

        // Verify that different statuses produce different styles
        assert_ne!(playing_style.fg, paused_style.fg);
        assert_eq!(unknown_style.fg, theme.text_style().fg);
    }
}
