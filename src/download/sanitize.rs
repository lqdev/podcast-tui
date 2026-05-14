//! Cross-platform filename sanitization shared between the download manager
//! and the device template engine.
//!
//! Extracted from `src/download/manager.rs` so the template engine
//! (`src/download/device_template.rs`) can apply the same rules to
//! device-side filenames without duplicating the unicode/punctuation table.
//!
//! The rules below are the canonical sanitizer used everywhere local
//! podcast/episode files are named. They handle:
//!
//! * Windows-prohibited characters (`<`, `>`, `:`, `"`, `/`, `\`, `|`, `?`, `*`)
//! * Control characters (ASCII 0–31)
//! * Common Unicode punctuation (smart quotes, ellipsis, en/em dashes)
//! * Latin-1 / Latin Extended accented characters → ASCII equivalents
//! * Misc symbols (`&` → `and`, `@` → `at`, etc.)
//! * Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`)
//! * Length cap of [`MAX_FILENAME_BYTES`] bytes with UTF-8 boundary safety
//!
//! This module is the canonical filename sanitizer for any file written
//! to the local filesystem (downloads, sync targets, device-side names).
//! For playlist *directory* names with different rules (no Unicode folding,
//! 100-char cap), use [`crate::utils::validation::sanitize_playlist_name`].

use crate::constants::filesystem::MAX_FILENAME_BYTES;

/// Comprehensive cross-platform filename sanitization.
///
/// Empty or whitespace-only input always returns `"Untitled"` regardless of
/// `is_folder`. `is_folder` only affects the fallback when sanitization of a
/// non-empty input *yields* an empty string (e.g. a title that consists
/// entirely of stripped characters): folders fall back to `"Podcast"`,
/// non-folders to `"Episode"`.
pub fn sanitize_filename(input: &str, is_folder: bool) -> String {
    // Step 1: Handle empty or whitespace-only input
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "Untitled".to_string();
    }

    // Step 2: Replace prohibited characters with safe alternatives
    let mut sanitized = String::new();
    for ch in trimmed.chars() {
        match ch {
            // Windows prohibited characters - replace with safe alternatives
            '<' => sanitized.push('('),
            '>' => sanitized.push(')'),
            ':' => sanitized.push('-'), // Common in titles like "Episode 1: Introduction"
            '"' => sanitized.push('\''), // Replace with single quote
            '/' => sanitized.push('-'), // Path separator
            '\\' => sanitized.push('-'), // Windows path separator
            '|' => sanitized.push('-'), // Pipe symbol
            '?' => sanitized.push_str(""), // Remove question marks to avoid confusion
            '*' => sanitized.push_str(""), // Remove wildcards

            // Control characters (ASCII 0-31) - remove entirely
            c if c.is_control() => {} // Skip control characters

            // Unicode quotes and special characters - normalize to ASCII
            '\u{201C}' | '\u{201D}' => sanitized.push('\''), // Smart double quotes to straight quote
            '\u{2018}' | '\u{2019}' => sanitized.push('\''), // Smart single quotes
            '\u{2026}' => sanitized.push_str("..."),         // Ellipsis to three dots
            '\u{2013}' | '\u{2014}' => sanitized.push('-'),  // En/em dash to hyphen

            // Keep safe characters
            c if c.is_ascii_alphanumeric() => sanitized.push(ch),
            ' ' | '-' | '_' | '(' | ')' => sanitized.push(ch),

            // Handle periods carefully
            '.' => {
                // Don't allow leading periods (creates hidden files on Unix)
                if !sanitized.is_empty() {
                    sanitized.push('.');
                }
            }

            // Convert other Unicode to ASCII equivalents or remove
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => sanitized.push('a'),
            'é' | 'è' | 'ê' | 'ë' | 'ē' => sanitized.push('e'),
            'í' | 'ì' | 'î' | 'ï' | 'ī' => sanitized.push('i'),
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' => sanitized.push('o'),
            'ú' | 'ù' | 'û' | 'ü' | 'ū' => sanitized.push('u'),
            'ñ' => sanitized.push('n'),
            'ç' => sanitized.push('c'),
            'ý' | 'ÿ' => sanitized.push('y'),

            // Capital versions
            'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' | 'Ā' => sanitized.push('A'),
            'É' | 'È' | 'Ê' | 'Ë' | 'Ē' => sanitized.push('E'),
            'Í' | 'Ì' | 'Î' | 'Ï' | 'Ī' => sanitized.push('I'),
            'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ō' => sanitized.push('O'),
            'Ú' | 'Ù' | 'Û' | 'Ü' | 'Ū' => sanitized.push('U'),
            'Ñ' => sanitized.push('N'),
            'Ç' => sanitized.push('C'),
            'Ý' | 'Ÿ' => sanitized.push('Y'),

            // Other common symbols - remove or replace
            '&' => sanitized.push_str("and"),
            '@' => sanitized.push_str("at"),
            '%' => sanitized.push_str("percent"),
            '#' => sanitized.push_str("number"),
            '+' => sanitized.push_str("plus"),
            '=' => sanitized.push('-'),

            // Skip other characters
            _ => {}
        }
    }

    // Step 3: Clean up multiple consecutive separators
    //
    // Note: we intentionally do *not* collapse " - " → "-" or " _ " → "_".
    // Those collapses produced inconsistent visual output across episodes and
    // silently rewrote user-supplied template separators (see #232). A bare
    // "--" or "__" run is still collapsed because it usually comes from
    // back-to-back substitutions (e.g. ":" → "-" next to an existing dash).
    let cleaned = sanitized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("--", "-")
        .replace("__", "_");

    // Step 4: Trim and handle edge cases
    let mut final_name = cleaned.trim().to_string();

    // Don't allow names that end with period or space (Windows restriction)
    while final_name.ends_with('.') || final_name.ends_with(' ') {
        final_name.pop();
    }

    // Don't allow names that start with period (creates hidden files)
    while final_name.starts_with('.') {
        final_name = final_name.chars().skip(1).collect();
    }

    // Handle Windows reserved device names
    final_name = handle_reserved_names(final_name);

    // Step 5: Ensure we have something meaningful
    if final_name.trim().is_empty() {
        final_name = if is_folder {
            "Podcast".to_string()
        } else {
            "Episode".to_string()
        };
    }

    // Step 6: Enforce length limit (see `MAX_FILENAME_BYTES` for rationale).
    if final_name.len() > MAX_FILENAME_BYTES {
        // Try to truncate at word boundary
        if let Some(last_space) = final_name[..MAX_FILENAME_BYTES].rfind(' ') {
            final_name.truncate(last_space);
        } else {
            final_name.truncate(MAX_FILENAME_BYTES);
        }

        // Ensure we didn't cut off in the middle of a UTF-8 character
        while !final_name.is_char_boundary(final_name.len()) {
            final_name.pop();
        }
    }

    // Final cleanup
    final_name.trim().to_string()
}

/// Handle Windows reserved device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9).
///
/// If the name (without extension) matches a reserved name, it is prefixed
/// with `_` to make it safe.
pub fn handle_reserved_names(mut name: String) -> String {
    let upper_name = name.to_uppercase();
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let name_without_ext = if let Some(dot_pos) = upper_name.find('.') {
        &upper_name[..dot_pos]
    } else {
        &upper_name
    };

    if reserved_names.contains(&name_without_ext) {
        name = format!("_{}", name);
    }

    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_sanitization() {
        assert_eq!(sanitize_filename("Hello World", false), "Hello World");
    }

    #[test]
    fn test_strips_path_separators() {
        let out = sanitize_filename("foo/bar\\baz", false);
        assert!(!out.contains('/'));
        assert!(!out.contains('\\'));
    }

    #[test]
    fn test_empty_input_returns_fallback() {
        assert_eq!(sanitize_filename("", false), "Untitled");
        assert_eq!(sanitize_filename("   ", false), "Untitled");
    }

    #[test]
    fn test_reserved_name_prefixed() {
        assert_eq!(handle_reserved_names("CON".to_string()), "_CON");
        assert_eq!(handle_reserved_names("nul.txt".to_string()), "_nul.txt");
        assert_eq!(handle_reserved_names("safe.txt".to_string()), "safe.txt");
    }

    #[test]
    fn test_unicode_accents_folded() {
        assert_eq!(sanitize_filename("Café", false), "Cafe");
    }

    #[test]
    fn test_length_cap_respects_max_filename_bytes() {
        let long = "a".repeat(MAX_FILENAME_BYTES * 2);
        let out = sanitize_filename(&long, false);
        assert!(out.len() <= MAX_FILENAME_BYTES);
    }

    #[test]
    fn test_length_cap_truncates_at_word_boundary() {
        // Build an input with a space just below the cap so the truncation
        // logic prefers the word boundary.
        let prefix = "word ".repeat(MAX_FILENAME_BYTES / 5);
        let input = format!("{prefix}extra-very-long-tail-segment");
        let out = sanitize_filename(&input, false);
        assert!(out.len() <= MAX_FILENAME_BYTES);
        // Should not end mid-word (no trailing partial "tail-segment")
        assert!(!out.ends_with("segment"));
    }

    // ─── #232: dash/underscore separator preservation ──────────────────

    /// User-supplied " - " separators (from titles or templates) must
    /// survive sanitization unchanged. Previously this was collapsed to
    /// "-", producing inconsistent device filenames.
    #[test]
    fn test_dash_separator_spacing_preserved() {
        assert_eq!(sanitize_filename("Foo - Bar", false), "Foo - Bar");
        assert_eq!(
            sanitize_filename("#123 - Episode Title", false),
            "number123 - Episode Title"
        );
    }

    /// Underscore separator with surrounding spaces must also survive.
    #[test]
    fn test_underscore_separator_spacing_preserved() {
        assert_eq!(sanitize_filename("Foo _ Bar", false), "Foo _ Bar");
    }

    /// Multiple consecutive whitespace characters still collapse to a
    /// single space (handled by `split_whitespace()`), but the dash
    /// itself is left alone.
    #[test]
    fn test_multi_space_collapses_but_dash_preserved() {
        assert_eq!(sanitize_filename("Foo  -  Bar", false), "Foo - Bar");
    }

    /// "--" and "__" runs (typically from back-to-back substitutions
    /// like ":" → "-" next to an existing dash) still collapse.
    #[test]
    fn test_double_dash_and_underscore_still_collapse() {
        assert_eq!(sanitize_filename("Foo--Bar", false), "Foo-Bar");
        assert_eq!(sanitize_filename("Foo__Bar", false), "Foo_Bar");
    }

    /// Title-internal ":" → "-" substitution still produces the
    /// expected (slightly awkward but predictable) "- " pattern. This
    /// behavior is unchanged by the #232 fix.
    #[test]
    fn test_colon_substitution_unchanged() {
        assert_eq!(sanitize_filename("Foo: Bar", false), "Foo- Bar");
    }

    /// Template-style render `{date} - {title}` keeps its dash visible
    /// in the final filename instead of being silently collapsed.
    #[test]
    fn test_template_dash_separator_preserved() {
        assert_eq!(
            sanitize_filename("20260417 - Episode Title", false),
            "20260417 - Episode Title"
        );
    }
}
