//! TOML theme file loader for podcast-tui
//!
//! Loads user-defined themes from `.toml` files and parses them into
//! [`Theme`] values backed by the existing [`ColorScheme`] struct.
//!
//! # Theme File Format
//!
//! ```toml
//! [metadata]
//! name = "Dracula"
//! author = "Zeno Rocha"          # optional
//! description = "A dark theme"   # optional
//! extends = "dark"               # optional: inherit from a bundled theme
//!
//! [colors]
//! background = "#282a36"
//! primary    = "#bd93f9"
//! # Missing fields inherit from the `extends` base (or default dark).
//! ```
//!
//! # Color Formats
//!
//! - **Hex**: `"#rrggbb"` — e.g. `"#ff79c6"` → `Color::Rgb(255, 121, 198)`
//! - **RGB function**: `"rgb(r, g, b)"` — e.g. `"rgb(255, 121, 198)"`
//! - **Indexed**: `"color(n)"` where n is 0–255 — e.g. `"color(141)"`
//! - **Named**: `"Red"`, `"Blue"`, `"White"`, etc. (case-insensitive)
//! - **Reset**: `"reset"` → `Color::Reset`

use std::path::Path;

use ratatui::style::Color;
use serde::Deserialize;
use thiserror::Error;

use crate::ui::themes::{ColorScheme, Theme};

/// Errors that can occur when loading or parsing a theme file.
#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("IO error reading theme file: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error in theme file: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Invalid color '{color}': {reason}")]
    InvalidColor { color: String, reason: String },

    #[error(
        "Unknown base theme '{0}' in extends field — valid names: dark, light, high-contrast, solarized"
    )]
    UnknownBaseTheme(String),
}

/// Top-level structure of a `.toml` theme file.
#[derive(Debug, Deserialize)]
struct ThemeFile {
    metadata: ThemeMetadata,
    #[serde(default)]
    colors: ThemeColors,
}

/// The `[metadata]` section of a theme file.
#[derive(Debug, Deserialize)]
struct ThemeMetadata {
    name: String,
    /// Name of a bundled theme to inherit colors from (`dark`, `light`,
    /// `high-contrast`, or `solarized`). When absent the default dark theme
    /// is used as the base.
    extends: Option<String>,
    // Informational fields stored in the file but not consumed by the app.
    #[allow(dead_code)]
    author: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
}

/// The `[colors]` section of a theme file.
///
/// All fields are optional; any field that is absent inherits its value from
/// the base theme (specified by `extends`, or default dark).
#[derive(Debug, Deserialize, Default)]
struct ThemeColors {
    // Background colors
    background: Option<String>,
    surface: Option<String>,
    overlay: Option<String>,

    // Foreground colors
    text: Option<String>,
    subtext: Option<String>,
    muted: Option<String>,

    // Accent colors
    primary: Option<String>,
    secondary: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,

    // UI element colors
    border: Option<String>,
    border_focused: Option<String>,
    selection: Option<String>,
    cursor: Option<String>,

    // Status colors
    playing: Option<String>,
    paused: Option<String>,
    downloaded: Option<String>,
    downloading: Option<String>,
    queued: Option<String>,

    // Indicator color
    active_indicator: Option<String>,
}

/// Parse a color string into a [`Color`].
///
/// Supported formats:
/// - Hex: `#rrggbb` (e.g. `"#ff79c6"`)
/// - RGB function: `rgb(r, g, b)` (e.g. `"rgb(255, 121, 198)"`)
/// - Indexed: `color(n)` where n is 0–255 (e.g. `"color(141)"`)
/// - Named: `Red`, `Blue`, `White`, `Black`, etc. (case-insensitive)
/// - Reset: `"reset"` → [`Color::Reset`]
pub fn parse_color(s: &str) -> Result<Color, ThemeError> {
    let trimmed = s.trim();
    let lower = trimmed.to_lowercase();
    if trimmed.starts_with('#') {
        parse_hex_color(trimmed)
    } else if lower.starts_with("rgb(") {
        parse_rgb_color(trimmed)
    } else if lower.starts_with("color(") {
        parse_indexed_color(trimmed)
    } else {
        parse_named_color(trimmed)
    }
}

/// Load a theme from a `.toml` file at `path`.
///
/// If the file's `[metadata]` section includes an `extends` field, the named
/// bundled theme is used as the base and only the fields specified in
/// `[colors]` are overridden. If `extends` is absent the default dark theme
/// is used as the base.
///
/// Returns a [`Theme`] ready for use by the UI.
pub fn load_theme_file(path: &Path) -> Result<Theme, ThemeError> {
    let content = std::fs::read_to_string(path)?;
    let file: ThemeFile = toml::from_str(&content)?;

    let base_colors = match &file.metadata.extends {
        Some(base_name) => {
            Theme::from_name(base_name)
                .map_err(|_| ThemeError::UnknownBaseTheme(base_name.clone()))?
                .colors
        }
        None => Theme::default_dark().colors,
    };

    let colors = apply_colors(base_colors, &file.colors)?;
    Ok(Theme::new(file.metadata.name, colors))
}

/// Apply the `overrides` on top of `base`, returning the merged [`ColorScheme`].
///
/// Fields in `overrides` that are `None` leave the corresponding field in
/// `base` unchanged.
fn apply_colors(mut base: ColorScheme, overrides: &ThemeColors) -> Result<ColorScheme, ThemeError> {
    macro_rules! apply {
        ($field:ident) => {
            if let Some(ref s) = overrides.$field {
                base.$field = parse_color(s)?;
            }
        };
    }

    apply!(background);
    apply!(surface);
    apply!(overlay);
    apply!(text);
    apply!(subtext);
    apply!(muted);
    apply!(primary);
    apply!(secondary);
    apply!(success);
    apply!(warning);
    apply!(error);
    apply!(border);
    apply!(border_focused);
    apply!(selection);
    apply!(cursor);
    apply!(playing);
    apply!(paused);
    apply!(downloaded);
    apply!(downloading);
    apply!(queued);
    apply!(active_indicator);

    Ok(base)
}

fn parse_hex_color(s: &str) -> Result<Color, ThemeError> {
    let hex = s.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(ThemeError::InvalidColor {
            color: s.to_string(),
            reason: "hex colors must be in #rrggbb format (6 hex digits)".to_string(),
        });
    }
    let parse_byte = |digits: &str| -> Result<u8, ThemeError> {
        u8::from_str_radix(digits, 16).map_err(|_| ThemeError::InvalidColor {
            color: s.to_string(),
            reason: format!("'{}' contains invalid hex digits", digits),
        })
    };
    Ok(Color::Rgb(
        parse_byte(&hex[0..2])?,
        parse_byte(&hex[2..4])?,
        parse_byte(&hex[4..6])?,
    ))
}

fn parse_rgb_color(s: &str) -> Result<Color, ThemeError> {
    let lower = s.to_lowercase();
    let inner = lower
        .strip_prefix("rgb(")
        .and_then(|t| t.strip_suffix(')'))
        .ok_or_else(|| ThemeError::InvalidColor {
            color: s.to_string(),
            reason: "expected format rgb(r, g, b)".to_string(),
        })?;

    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 3 {
        return Err(ThemeError::InvalidColor {
            color: s.to_string(),
            reason: "rgb() requires exactly 3 comma-separated components".to_string(),
        });
    }

    let parse_component = |p: &str| -> Result<u8, ThemeError> {
        p.trim()
            .parse::<u8>()
            .map_err(|_| ThemeError::InvalidColor {
                color: s.to_string(),
                reason: format!("component '{}' must be an integer 0–255", p.trim()),
            })
    };

    Ok(Color::Rgb(
        parse_component(parts[0])?,
        parse_component(parts[1])?,
        parse_component(parts[2])?,
    ))
}

fn parse_indexed_color(s: &str) -> Result<Color, ThemeError> {
    let lower = s.to_lowercase();
    let inner = lower
        .strip_prefix("color(")
        .and_then(|t| t.strip_suffix(')'))
        .ok_or_else(|| ThemeError::InvalidColor {
            color: s.to_string(),
            reason: "expected format color(n) where n is 0–255".to_string(),
        })?;

    inner
        .trim()
        .parse::<u8>()
        .map(Color::Indexed)
        .map_err(|_| ThemeError::InvalidColor {
            color: s.to_string(),
            reason: format!("'{}' is not a valid color index (0–255)", inner.trim()),
        })
}

fn parse_named_color(s: &str) -> Result<Color, ThemeError> {
    match s.to_lowercase().as_str() {
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "dark_gray" | "darkgrey" | "dark_grey" => Ok(Color::DarkGray),
        "lightred" | "light_red" => Ok(Color::LightRed),
        "lightgreen" | "light_green" => Ok(Color::LightGreen),
        "lightyellow" | "light_yellow" => Ok(Color::LightYellow),
        "lightblue" | "light_blue" => Ok(Color::LightBlue),
        "lightmagenta" | "light_magenta" => Ok(Color::LightMagenta),
        "lightcyan" | "light_cyan" => Ok(Color::LightCyan),
        "white" => Ok(Color::White),
        "reset" => Ok(Color::Reset),
        _ => Err(ThemeError::InvalidColor {
            color: s.to_string(),
            reason: "unknown color name — use hex (#rrggbb), rgb(r,g,b), color(n), or a named \
                     color (black, red, green, yellow, blue, magenta, cyan, white, etc.)"
                .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── parse_color tests ────────────────────────────────────────────────────

    #[test]
    fn test_parse_hex_color_standard() {
        // Arrange / Act / Assert
        assert_eq!(parse_color("#ff79c6").unwrap(), Color::Rgb(255, 121, 198));
    }

    #[test]
    fn test_parse_hex_color_lowercase() {
        assert_eq!(parse_color("#282a36").unwrap(), Color::Rgb(40, 42, 54));
    }

    #[test]
    fn test_parse_hex_color_uppercase() {
        assert_eq!(parse_color("#FF0000").unwrap(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_rgb_color_with_spaces() {
        assert_eq!(
            parse_color("rgb(255, 121, 198)").unwrap(),
            Color::Rgb(255, 121, 198)
        );
    }

    #[test]
    fn test_parse_rgb_color_no_spaces() {
        assert_eq!(
            parse_color("rgb(10,20,30)").unwrap(),
            Color::Rgb(10, 20, 30)
        );
    }

    #[test]
    fn test_parse_indexed_color() {
        assert_eq!(parse_color("color(141)").unwrap(), Color::Indexed(141));
    }

    #[test]
    fn test_parse_indexed_color_zero() {
        assert_eq!(parse_color("color(0)").unwrap(), Color::Indexed(0));
    }

    #[test]
    fn test_parse_named_color_red() {
        assert_eq!(parse_color("Red").unwrap(), Color::Red);
    }

    #[test]
    fn test_parse_named_color_case_insensitive() {
        assert_eq!(parse_color("red").unwrap(), Color::Red);
        assert_eq!(parse_color("RED").unwrap(), Color::Red);
        assert_eq!(parse_color("White").unwrap(), Color::White);
        assert_eq!(parse_color("CYAN").unwrap(), Color::Cyan);
    }

    #[test]
    fn test_parse_reset_color() {
        assert_eq!(parse_color("reset").unwrap(), Color::Reset);
        assert_eq!(parse_color("Reset").unwrap(), Color::Reset);
    }

    #[test]
    fn test_parse_dark_gray_variants() {
        assert_eq!(parse_color("darkgray").unwrap(), Color::DarkGray);
        assert_eq!(parse_color("dark_gray").unwrap(), Color::DarkGray);
        assert_eq!(parse_color("DarkGray").unwrap(), Color::DarkGray);
        assert_eq!(parse_color("darkgrey").unwrap(), Color::DarkGray);
    }

    #[test]
    fn test_parse_light_color_variants() {
        assert_eq!(parse_color("lightred").unwrap(), Color::LightRed);
        assert_eq!(parse_color("light_red").unwrap(), Color::LightRed);
        assert_eq!(parse_color("LightBlue").unwrap(), Color::LightBlue);
    }

    #[test]
    fn test_parse_gray_and_grey_both_work() {
        assert_eq!(parse_color("gray").unwrap(), Color::Gray);
        assert_eq!(parse_color("grey").unwrap(), Color::Gray);
    }

    #[test]
    fn test_invalid_color_returns_error_with_value_in_message() {
        // Arrange
        let input = "not-a-color";

        // Act
        let result = parse_color(input);

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not-a-color"),
            "error should mention the bad value: {err}"
        );
    }

    #[test]
    fn test_invalid_hex_wrong_length_returns_error() {
        assert!(parse_color("#fff").is_err());
        assert!(parse_color("#fffffff").is_err());
    }

    #[test]
    fn test_invalid_rgb_component_out_of_range_returns_error() {
        assert!(parse_color("rgb(256, 0, 0)").is_err());
    }

    #[test]
    fn test_invalid_rgb_wrong_component_count_returns_error() {
        assert!(parse_color("rgb(255, 0)").is_err());
        assert!(parse_color("rgb(255, 0, 0, 0)").is_err());
    }

    // ── load_theme_file tests ────────────────────────────────────────────────

    #[test]
    fn test_load_theme_file_complete() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.toml");
        let content = r##"
[metadata]
name = "Test Theme"

[colors]
background = "#000000"
surface    = "#111111"
overlay    = "#222222"
text       = "#ffffff"
subtext    = "#dddddd"
muted      = "#aaaaaa"
primary    = "#ff0000"
secondary  = "#00ff00"
success    = "#00ff00"
warning    = "#ffff00"
error      = "#ff0000"
border          = "#555555"
border_focused  = "#ff0000"
selection       = "#333333"
cursor          = "#ffffff"
playing         = "#00ff00"
paused          = "#ffff00"
downloaded      = "#0000ff"
downloading     = "#ff00ff"
queued          = "#888888"
active_indicator = "#ffff00"
"##;
        std::fs::write(&path, content).unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();

        // Assert
        assert_eq!(theme.name, "Test Theme");
        assert_eq!(theme.colors.background, Color::Rgb(0, 0, 0));
        assert_eq!(theme.colors.text, Color::Rgb(255, 255, 255));
        assert_eq!(theme.colors.primary, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_load_theme_file_partial_inherits_dark_defaults() {
        // Arrange: override only primary; everything else should come from dark theme
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("partial.toml");
        std::fs::write(
            &path,
            r##"
[metadata]
name = "Partial Theme"

[colors]
primary = "#ff79c6"
"##,
        )
        .unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();
        let dark = Theme::default_dark();

        // Assert: overridden field is updated
        assert_eq!(theme.colors.primary, Color::Rgb(255, 121, 198));
        // Assert: non-overridden fields match the dark theme defaults
        assert_eq!(theme.colors.background, dark.colors.background);
        assert_eq!(theme.colors.text, dark.colors.text);
        assert_eq!(theme.colors.error, dark.colors.error);
    }

    #[test]
    fn test_load_theme_file_with_extends_solarized() {
        // Arrange: extend solarized, override one color
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("extended.toml");
        std::fs::write(
            &path,
            r##"
[metadata]
name = "My Solarized"
extends = "solarized"

[colors]
primary = "#ff79c6"
"##,
        )
        .unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();
        let solarized = Theme::solarized();

        // Assert: overridden field is updated
        assert_eq!(theme.colors.primary, Color::Rgb(255, 121, 198));
        // Assert: non-overridden fields inherit solarized values
        assert_eq!(theme.colors.background, solarized.colors.background);
        assert_eq!(theme.colors.error, solarized.colors.error);
    }

    #[test]
    fn test_load_theme_file_extends_all_bundled_themes() {
        // Arrange / Act / Assert: all four bundled base names must resolve
        let dir = TempDir::new().unwrap();
        for base in &["dark", "light", "high-contrast", "solarized"] {
            let path = dir.path().join(format!("{base}.toml"));
            std::fs::write(
                &path,
                format!("[metadata]\nname = \"test\"\nextends = \"{base}\"\n"),
            )
            .unwrap();
            let result = load_theme_file(&path);
            assert!(
                result.is_ok(),
                "extends = '{base}' should be valid: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn test_load_theme_file_invalid_color_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad_color.toml");
        std::fs::write(
            &path,
            r#"
[metadata]
name = "Bad Theme"

[colors]
primary = "not-a-valid-color"
"#,
        )
        .unwrap();

        // Act
        let result = load_theme_file(&path);

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not-a-valid-color"),
            "error should mention the bad value: {err}"
        );
    }

    #[test]
    fn test_load_theme_file_unknown_extends_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad_extends.toml");
        std::fs::write(
            &path,
            r#"
[metadata]
name = "Bad Extends"
extends = "nonexistent-theme"
"#,
        )
        .unwrap();

        // Act
        let result = load_theme_file(&path);

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent-theme"),
            "error should mention the unknown theme: {err}"
        );
    }

    #[test]
    fn test_load_theme_file_with_optional_metadata_fields() {
        // Arrange: file has author and description (should parse without error)
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("meta.toml");
        std::fs::write(
            &path,
            r##"
[metadata]
name = "Dracula"
author = "Zeno Rocha"
description = "A dark theme for all occasions"

[colors]
background = "#282a36"
"##,
        )
        .unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();

        // Assert
        assert_eq!(theme.name, "Dracula");
        assert_eq!(theme.colors.background, Color::Rgb(40, 42, 54));
    }

    #[test]
    fn test_load_theme_file_with_named_colors() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("named.toml");
        std::fs::write(
            &path,
            r#"
[metadata]
name = "Named Colors"

[colors]
primary  = "Yellow"
error    = "Red"
success  = "Green"
border   = "gray"
"#,
        )
        .unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();

        // Assert
        assert_eq!(theme.colors.primary, Color::Yellow);
        assert_eq!(theme.colors.error, Color::Red);
        assert_eq!(theme.colors.success, Color::Green);
        assert_eq!(theme.colors.border, Color::Gray);
    }

    #[test]
    fn test_load_theme_file_with_rgb_and_indexed_colors() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mixed.toml");
        std::fs::write(
            &path,
            r#"
[metadata]
name = "Mixed Formats"

[colors]
primary  = "rgb(189, 147, 249)"
secondary = "color(141)"
"#,
        )
        .unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();

        // Assert
        assert_eq!(theme.colors.primary, Color::Rgb(189, 147, 249));
        assert_eq!(theme.colors.secondary, Color::Indexed(141));
    }

    #[test]
    fn test_load_theme_file_missing_colors_section_uses_dark_defaults() {
        // Arrange: no [colors] section at all
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("no_colors.toml");
        std::fs::write(
            &path,
            r#"
[metadata]
name = "Minimal"
"#,
        )
        .unwrap();

        // Act
        let theme = load_theme_file(&path).unwrap();
        let dark = Theme::default_dark();

        // Assert: all colors match dark defaults
        assert_eq!(theme.colors.background, dark.colors.background);
        assert_eq!(theme.colors.primary, dark.colors.primary);
    }
}
