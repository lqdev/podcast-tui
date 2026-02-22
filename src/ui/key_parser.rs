// Key notation parser — converts human-readable strings to KeyChord and back.
//
// Follows Helix/Zellij conventions:
//   C-x      → Ctrl + x
//   S-x      → Shift + x
//   A-x/M-x  → Alt + x
//   C-S-x    → Ctrl + Shift + x
//   F1-F12   → function keys
//   Space/SPC → space bar
//   Named keys: Enter, Tab, Esc, Backspace, Delete, Up, Down, Left, Right,
//               Home, End, PgUp/PageUp, PgDn/PageDown

use crossterm::event::{KeyCode, KeyModifiers, MediaKeyCode};

use crate::ui::keybindings::KeyChord;

/// Error type for key notation parsing.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum KeyParseError {
    #[error("Empty key notation")]
    Empty,
    #[error("Unknown key name: {0}")]
    UnknownKey(String),
    #[error("Invalid modifier: {0}")]
    InvalidModifier(String),
    #[error("Missing key after modifier prefix")]
    MissingKey,
}

/// Parse a human-readable key notation string into a `KeyChord`.
///
/// # Examples
/// ```
/// use podcast_tui::ui::key_parser::parse_key_notation;
/// use crossterm::event::{KeyCode, KeyModifiers};
///
/// let chord = parse_key_notation("C-n").unwrap();
/// assert_eq!(chord.code, KeyCode::Char('n'));
/// assert_eq!(chord.modifiers, KeyModifiers::CONTROL);
/// ```
pub fn parse_key_notation(notation: &str) -> Result<KeyChord, KeyParseError> {
    let notation = notation.trim();
    if notation.is_empty() {
        return Err(KeyParseError::Empty);
    }

    // Split on '-' but be careful: a bare '-' is a valid key.
    // Strategy: walk the notation collecting modifier tokens until we find
    // a token that is a key (not a known modifier letter).
    //
    // Modifier prefixes recognized: C, S, A, M (case-sensitive as per Helix).
    // We split on '-' and check each segment except the last, which is the key.
    // Edge case: a trailing '-' like "C-" means MissingKey.
    // Edge case: bare "-" is a valid single-char key.

    if notation == "-" {
        return Ok(KeyChord::none(KeyCode::Char('-')));
    }

    let parts: Vec<&str> = notation.split('-').collect();

    let mut modifiers = KeyModifiers::NONE;
    let mut key_index = 0;

    // Walk through all parts except the last, parsing modifiers.
    // Stop as soon as we hit something that isn't a modifier letter.
    for (i, &part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part is always the key.
            key_index = i;
            break;
        }
        match part {
            "C" => {
                modifiers |= KeyModifiers::CONTROL;
                key_index = i + 1;
            }
            "S" => {
                modifiers |= KeyModifiers::SHIFT;
                key_index = i + 1;
            }
            "A" | "M" => {
                modifiers |= KeyModifiers::ALT;
                key_index = i + 1;
            }
            _ => {
                // A single uppercase ASCII letter that isn't a known modifier (C/S/A/M)
                // is almost certainly a config typo (e.g., "X-n"). Return a specific error
                // so users get actionable feedback rather than a confusing UnknownKey.
                if part.len() == 1 && part.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    return Err(KeyParseError::InvalidModifier(part.to_string()));
                }
                // Multi-char or non-uppercase token — treat everything from here as the key.
                key_index = i;
                break;
            }
        }
    }

    let key_str = parts[key_index..].join("-");
    if key_str.is_empty() {
        return Err(KeyParseError::MissingKey);
    }

    let code = parse_key_code(&key_str)?;
    Ok(KeyChord::new(modifiers, code))
}

/// Parse a key name (without modifier prefix) into a `KeyCode`.
fn parse_key_code(key_str: &str) -> Result<KeyCode, KeyParseError> {
    // Single character (case-sensitive — 'q' and 'Q' are distinct keys).
    if key_str.chars().count() == 1 {
        return Ok(KeyCode::Char(
            key_str
                .chars()
                .next()
                // SAFETY: count() == 1 guarantees next() returns Some.
                .expect("chars().next() must exist when count() == 1"),
        ));
    }

    // Named keys and function keys — normalize to lowercase for case-insensitive matching.
    // Single-char keys above are left as-is ('q' and 'Q' are different keys).
    let key_lower = key_str.to_ascii_lowercase();
    match key_lower.as_str() {
        "enter" | "return" => Ok(KeyCode::Enter),
        "tab" => Ok(KeyCode::Tab),
        "backtab" => Ok(KeyCode::BackTab),
        "esc" | "escape" => Ok(KeyCode::Esc),
        "backspace" | "bs" => Ok(KeyCode::Backspace),
        "delete" | "del" => Ok(KeyCode::Delete),
        "space" | "spc" => Ok(KeyCode::Char(' ')),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pgup" | "pageup" => Ok(KeyCode::PageUp),
        "pgdn" | "pagedown" => Ok(KeyCode::PageDown),
        "insert" | "ins" => Ok(KeyCode::Insert),
        "capslock" => Ok(KeyCode::CapsLock),
        "numlock" => Ok(KeyCode::NumLock),
        "scrolllock" => Ok(KeyCode::ScrollLock),
        "printscreen" => Ok(KeyCode::PrintScreen),
        "pause" => Ok(KeyCode::Pause),
        "menu" => Ok(KeyCode::Menu),
        _ => {
            // Function keys: F1-F12 (case-insensitive: "F1", "f1" both work).
            if let Some(n_str) = key_lower.strip_prefix('f') {
                if let Ok(n) = n_str.parse::<u8>() {
                    if (1..=12).contains(&n) {
                        return Ok(KeyCode::F(n));
                    }
                }
            }
            // Preserve original casing in error for user-facing diagnostic.
            Err(KeyParseError::UnknownKey(key_str.to_string()))
        }
    }
}

/// Serialize a `KeyChord` back to its canonical notation string.
///
/// Produces the shortest unambiguous representation.
/// Round-trips with `parse_key_notation` for all supported notations.
pub fn key_to_notation(chord: &KeyChord) -> String {
    let mut parts: Vec<&str> = Vec::new();

    if chord.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("C");
    }
    if chord.modifiers.contains(KeyModifiers::ALT) {
        parts.push("A");
    }
    if chord.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("S");
    }

    let key_str = key_code_to_str(&chord.code);

    if parts.is_empty() {
        key_str
    } else {
        format!("{}-{}", parts.join("-"), key_str)
    }
}

/// Convert a `KeyCode` to its canonical string representation.
fn key_code_to_str(code: &KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::PrintScreen => "PrintScreen".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "Menu".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Media(m) => media_key_to_str(m),
        // Fallback for keys not in our notation (shouldn't appear in practice).
        _ => format!("{code:?}"),
    }
}

/// Convert a `MediaKeyCode` to a human-readable Unicode symbol string.
fn media_key_to_str(code: &MediaKeyCode) -> String {
    match code {
        MediaKeyCode::Play => "⏵".to_string(),
        MediaKeyCode::Pause => "⏸".to_string(),
        MediaKeyCode::PlayPause => "⏯".to_string(),
        MediaKeyCode::Stop => "⏹".to_string(),
        MediaKeyCode::FastForward => "⏩".to_string(),
        MediaKeyCode::Rewind => "⏪".to_string(),
        MediaKeyCode::TrackNext => "⏭".to_string(),
        MediaKeyCode::TrackPrevious => "⏮".to_string(),
        MediaKeyCode::RaiseVolume => "🔊".to_string(),
        MediaKeyCode::LowerVolume => "🔉".to_string(),
        MediaKeyCode::MuteVolume => "🔇".to_string(),
        _ => format!("{code:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    // ── Single character keys ────────────────────────────────────────────────

    #[test]
    fn test_parse_single_lowercase_char_returns_no_modifiers() {
        let chord = parse_key_notation("q").unwrap();
        assert_eq!(chord.code, KeyCode::Char('q'));
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_single_uppercase_char_returns_no_modifiers() {
        // Uppercase literal — no S- prefix means no SHIFT modifier.
        let chord = parse_key_notation("Q").unwrap();
        assert_eq!(chord.code, KeyCode::Char('Q'));
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_question_mark_returns_char() {
        let chord = parse_key_notation("?").unwrap();
        assert_eq!(chord.code, KeyCode::Char('?'));
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_digit_returns_char() {
        let chord = parse_key_notation("3").unwrap();
        assert_eq!(chord.code, KeyCode::Char('3'));
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    // ── Modifier prefixes ────────────────────────────────────────────────────

    #[test]
    fn test_parse_ctrl_modifier_returns_control_flag() {
        let chord = parse_key_notation("C-n").unwrap();
        assert_eq!(chord.code, KeyCode::Char('n'));
        assert_eq!(chord.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_parse_shift_modifier_returns_shift_flag() {
        let chord = parse_key_notation("S-Tab").unwrap();
        assert_eq!(chord.code, KeyCode::Tab);
        assert_eq!(chord.modifiers, KeyModifiers::SHIFT);
    }

    #[test]
    fn test_parse_alt_modifier_a_prefix_returns_alt_flag() {
        let chord = parse_key_notation("A-x").unwrap();
        assert_eq!(chord.code, KeyCode::Char('x'));
        assert_eq!(chord.modifiers, KeyModifiers::ALT);
    }

    #[test]
    fn test_parse_alt_modifier_m_prefix_returns_alt_flag() {
        let chord = parse_key_notation("M-x").unwrap();
        assert_eq!(chord.code, KeyCode::Char('x'));
        assert_eq!(chord.modifiers, KeyModifiers::ALT);
    }

    #[test]
    fn test_parse_ctrl_shift_combined_modifiers() {
        let chord = parse_key_notation("C-S-d").unwrap();
        assert_eq!(chord.code, KeyCode::Char('d'));
        assert!(chord.modifiers.contains(KeyModifiers::CONTROL));
        assert!(chord.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_parse_ctrl_alt_combined_modifiers() {
        let chord = parse_key_notation("C-A-r").unwrap();
        assert_eq!(chord.code, KeyCode::Char('r'));
        assert!(chord.modifiers.contains(KeyModifiers::CONTROL));
        assert!(chord.modifiers.contains(KeyModifiers::ALT));
    }

    // ── Named keys ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_enter_returns_enter_keycode() {
        let chord = parse_key_notation("Enter").unwrap();
        assert_eq!(chord.code, KeyCode::Enter);
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_esc_returns_esc_keycode() {
        let chord = parse_key_notation("Esc").unwrap();
        assert_eq!(chord.code, KeyCode::Esc);
    }

    #[test]
    fn test_parse_tab_returns_tab_keycode() {
        let chord = parse_key_notation("Tab").unwrap();
        assert_eq!(chord.code, KeyCode::Tab);
    }

    #[test]
    fn test_parse_backtab_returns_backtab_keycode() {
        let chord = parse_key_notation("BackTab").unwrap();
        assert_eq!(chord.code, KeyCode::BackTab);
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_shift_backtab_returns_backtab_with_shift() {
        let chord = parse_key_notation("S-BackTab").unwrap();
        assert_eq!(chord.code, KeyCode::BackTab);
        assert_eq!(chord.modifiers, KeyModifiers::SHIFT);
    }

    #[test]
    fn test_roundtrip_backtab() {
        let chord = parse_key_notation("BackTab").unwrap();
        assert_eq!(key_to_notation(&chord), "BackTab");
    }

    #[test]
    fn test_parse_backspace_returns_backspace_keycode() {
        let chord = parse_key_notation("Backspace").unwrap();
        assert_eq!(chord.code, KeyCode::Backspace);
    }

    #[test]
    fn test_parse_space_alias_returns_space_char() {
        let chord = parse_key_notation("Space").unwrap();
        assert_eq!(chord.code, KeyCode::Char(' '));
    }

    #[test]
    fn test_parse_spc_alias_returns_space_char() {
        let chord = parse_key_notation("SPC").unwrap();
        assert_eq!(chord.code, KeyCode::Char(' '));
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(parse_key_notation("Up").unwrap().code, KeyCode::Up);
        assert_eq!(parse_key_notation("Down").unwrap().code, KeyCode::Down);
        assert_eq!(parse_key_notation("Left").unwrap().code, KeyCode::Left);
        assert_eq!(parse_key_notation("Right").unwrap().code, KeyCode::Right);
    }

    #[test]
    fn test_parse_navigation_keys() {
        assert_eq!(parse_key_notation("Home").unwrap().code, KeyCode::Home);
        assert_eq!(parse_key_notation("End").unwrap().code, KeyCode::End);
        assert_eq!(parse_key_notation("PgUp").unwrap().code, KeyCode::PageUp);
        assert_eq!(parse_key_notation("PgDn").unwrap().code, KeyCode::PageDown);
        assert_eq!(parse_key_notation("PageUp").unwrap().code, KeyCode::PageUp);
        assert_eq!(
            parse_key_notation("PageDown").unwrap().code,
            KeyCode::PageDown
        );
    }

    // ── Function keys ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_function_key_f1() {
        let chord = parse_key_notation("F1").unwrap();
        assert_eq!(chord.code, KeyCode::F(1));
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_function_key_f12() {
        let chord = parse_key_notation("F12").unwrap();
        assert_eq!(chord.code, KeyCode::F(12));
    }

    #[test]
    fn test_parse_ctrl_function_key() {
        let chord = parse_key_notation("C-F3").unwrap();
        assert_eq!(chord.code, KeyCode::F(3));
        assert_eq!(chord.modifiers, KeyModifiers::CONTROL);
    }

    // ── Serialization ────────────────────────────────────────────────────────

    #[test]
    fn test_key_to_notation_single_char() {
        let chord = KeyChord::none(KeyCode::Char('q'));
        assert_eq!(key_to_notation(&chord), "q");
    }

    #[test]
    fn test_key_to_notation_ctrl_char() {
        let chord = KeyChord::ctrl(KeyCode::Char('n'));
        assert_eq!(key_to_notation(&chord), "C-n");
    }

    #[test]
    fn test_key_to_notation_function_key() {
        let chord = KeyChord::none(KeyCode::F(3));
        assert_eq!(key_to_notation(&chord), "F3");
    }

    #[test]
    fn test_key_to_notation_space() {
        let chord = KeyChord::none(KeyCode::Char(' '));
        assert_eq!(key_to_notation(&chord), "Space");
    }

    // ── Round-trip (parse → serialize → parse) ───────────────────────────────

    #[test]
    fn test_roundtrip_ctrl_char() {
        let original = "C-n";
        let chord = parse_key_notation(original).unwrap();
        assert_eq!(key_to_notation(&chord), original);
    }

    #[test]
    fn test_roundtrip_shift_tab() {
        let original = "S-Tab";
        let chord = parse_key_notation(original).unwrap();
        assert_eq!(key_to_notation(&chord), original);
    }

    #[test]
    fn test_roundtrip_ctrl_shift() {
        let original = "C-S-d";
        let chord = parse_key_notation(original).unwrap();
        assert_eq!(key_to_notation(&chord), original);
    }

    #[test]
    fn test_roundtrip_function_key() {
        let original = "F1";
        let chord = parse_key_notation(original).unwrap();
        assert_eq!(key_to_notation(&chord), original);
    }

    #[test]
    fn test_roundtrip_named_key_enter() {
        let original = "Enter";
        let chord = parse_key_notation(original).unwrap();
        assert_eq!(key_to_notation(&chord), original);
    }

    // ── Edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_bare_hyphen_returns_char() {
        let chord = parse_key_notation("-").unwrap();
        assert_eq!(chord.code, KeyCode::Char('-'));
        assert_eq!(chord.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_leading_trailing_whitespace_is_trimmed() {
        let chord = parse_key_notation("  C-n  ").unwrap();
        assert_eq!(chord.code, KeyCode::Char('n'));
        assert_eq!(chord.modifiers, KeyModifiers::CONTROL);
    }

    // ── Error cases ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_empty_string_returns_error() {
        let err = parse_key_notation("").unwrap_err();
        assert_eq!(err, KeyParseError::Empty);
    }

    #[test]
    fn test_parse_whitespace_only_returns_error() {
        let err = parse_key_notation("   ").unwrap_err();
        assert_eq!(err, KeyParseError::Empty);
    }

    #[test]
    fn test_parse_unknown_key_returns_error() {
        let err = parse_key_notation("C-FOOBAR").unwrap_err();
        assert!(matches!(err, KeyParseError::UnknownKey(_)));
    }

    #[test]
    fn test_parse_unknown_named_key_returns_error() {
        let err = parse_key_notation("Flibbertigibbet").unwrap_err();
        assert!(matches!(err, KeyParseError::UnknownKey(_)));
    }

    #[test]
    fn test_parse_f0_out_of_range_returns_error() {
        // F0 is not in 1-12 range.
        let err = parse_key_notation("F0").unwrap_err();
        assert!(matches!(err, KeyParseError::UnknownKey(_)));
    }

    #[test]
    fn test_parse_f13_out_of_range_returns_error() {
        // F13 is not in 1-12 range.
        let err = parse_key_notation("F13").unwrap_err();
        assert!(matches!(err, KeyParseError::UnknownKey(_)));
    }

    #[test]
    fn test_parse_unknown_modifier_prefix_returns_invalid_modifier() {
        // Single uppercase letter that isn't C/S/A/M should be reported as an
        // invalid modifier, not silently swallowed into an UnknownKey error.
        let err = parse_key_notation("X-n").unwrap_err();
        assert!(matches!(err, KeyParseError::InvalidModifier(ref s) if s == "X"));
    }

    #[test]
    fn test_parse_unknown_modifier_prefix_n_returns_invalid_modifier() {
        // 'N' is a common capitalisation typo for 'n' (the key), used as a modifier.
        let err = parse_key_notation("N-x").unwrap_err();
        assert!(matches!(err, KeyParseError::InvalidModifier(_)));
    }

    // ── Case-insensitive named keys ──────────────────────────────────────────

    #[test]
    fn test_parse_named_key_lowercase_enter_is_accepted() {
        // Config files may use lowercase; the parser must accept it.
        let chord = parse_key_notation("enter").unwrap();
        assert_eq!(chord.code, KeyCode::Enter);
    }

    #[test]
    fn test_parse_named_key_uppercase_enter_is_accepted() {
        let chord = parse_key_notation("ENTER").unwrap();
        assert_eq!(chord.code, KeyCode::Enter);
    }

    #[test]
    fn test_parse_named_key_lowercase_esc_is_accepted() {
        let chord = parse_key_notation("esc").unwrap();
        assert_eq!(chord.code, KeyCode::Esc);
    }

    #[test]
    fn test_parse_function_key_lowercase_f_prefix_is_accepted() {
        // "f1" should parse identically to "F1".
        let chord = parse_key_notation("f1").unwrap();
        assert_eq!(chord.code, KeyCode::F(1));
    }

    #[test]
    fn test_parse_function_key_uppercase_preserves_existing_behaviour() {
        // Canonical uppercase "F3" must still work after case-insensitive change.
        let chord = parse_key_notation("F3").unwrap();
        assert_eq!(chord.code, KeyCode::F(3));
    }
}
