//! Text processing utilities
//!
//! This module provides text processing functions including HTML sanitization
//! for RSS feed content.

use regex::Regex;
use std::borrow::Cow;

/// Strip HTML tags from text content
///
/// This function removes HTML tags and decodes common HTML entities,
/// returning clean plain text suitable for display in a TUI.
///
/// # Examples
///
/// ```
/// use podcast_tui::utils::text::strip_html;
///
/// let html = "<p>Hello <strong>world</strong>!</p>";
/// assert_eq!(strip_html(html), "Hello world!");
/// ```
pub fn strip_html(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    // First, remove HTML tags using regex
    // This regex matches any HTML tag: <...>
    let re = Regex::new(r"<[^>]*>").expect("Invalid regex");
    let without_tags = re.replace_all(input, "");

    // Then decode HTML entities (after stripping tags)
    let decoded = decode_html_entities(&without_tags);

    // Clean up excessive whitespace
    let cleaned = clean_whitespace(&decoded);

    cleaned.to_string()
}

/// Decode common HTML entities
///
/// Converts HTML entities like &amp; &lt; &gt; &quot; etc. to their
/// character equivalents.
fn decode_html_entities(input: &str) -> Cow<'_, str> {
    if !input.contains('&') {
        return Cow::Borrowed(input);
    }

    let mut result = input.to_string();

    // Common HTML entities
    let entities = [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&#39;", "'"),
        ("&nbsp;", " "),
        ("&ndash;", "–"),
        ("&mdash;", "—"),
        ("&hellip;", "…"),
        ("&ldquo;", "\u{201C}"), // Left double quotation mark
        ("&rdquo;", "\u{201D}"), // Right double quotation mark
        ("&lsquo;", "\u{2018}"), // Left single quotation mark
        ("&rsquo;", "\u{2019}"), // Right single quotation mark
        ("&bull;", "•"),
        ("&middot;", "·"),
        ("&copy;", "©"),
        ("&reg;", "®"),
        ("&trade;", "™"),
    ];

    for (entity, replacement) in &entities {
        result = result.replace(entity, replacement);
    }

    Cow::Owned(result)
}

/// Clean up excessive whitespace
///
/// Removes extra spaces, tabs, and empty lines while preserving
/// paragraph breaks (double newlines).
fn clean_whitespace(input: &str) -> Cow<'_, str> {
    if input.is_empty() {
        return Cow::Borrowed(input);
    }

    let mut result = String::with_capacity(input.len());
    let mut prev_was_space = false;
    let mut consecutive_newlines = 0;

    for ch in input.chars() {
        match ch {
            '\n' => {
                consecutive_newlines += 1;
                // Allow at most 2 consecutive newlines (one blank line)
                if consecutive_newlines <= 2 {
                    result.push('\n');
                    prev_was_space = true;
                }
            }
            '\r' => {
                // Skip carriage returns, we use \n for newlines
            }
            ' ' | '\t' => {
                if !prev_was_space {
                    result.push(' ');
                    prev_was_space = true;
                }
                consecutive_newlines = 0;
            }
            _ => {
                result.push(ch);
                prev_was_space = false;
                consecutive_newlines = 0;
            }
        }
    }

    // Trim trailing/leading whitespace
    let trimmed = result.trim();
    if trimmed.len() == result.len() {
        Cow::Owned(result)
    } else {
        Cow::Owned(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(strip_html(""), "");
        assert_eq!(strip_html("plain text"), "plain text");
        assert_eq!(strip_html("<p>Hello</p>"), "Hello");
        assert_eq!(
            strip_html("<div>Test <strong>bold</strong> text</div>"),
            "Test bold text"
        );
    }

    #[test]
    fn test_strip_html_complex() {
        let html = r#"<div>In recognition of Hispanic Heritage Month, today's episode is dedicated to George Meléndez Wright, the first Hispanic person to occupy a professional role in the National Park Service.<br>
<br>
To submit a business for the Outsiders Gift Guide, please email <a href="mailto:assistant@npadpodcast.com">assistant@npadpodcast.com</a> by October 22nd :)<br>"#;

        let result = strip_html(html);
        assert!(result.contains("In recognition of Hispanic Heritage Month"));
        assert!(!result.contains("<div>"));
        assert!(!result.contains("<br>"));
        assert!(!result.contains("<a href="));
    }

    #[test]
    fn test_strip_html_nested_tags() {
        let html = "<div><p>Paragraph <strong>with <em>nested</em> tags</strong></p></div>";
        assert_eq!(strip_html(html), "Paragraph with nested tags");
    }

    #[test]
    fn test_decode_html_entities() {
        let text = "Hello &amp; goodbye &lt;world&gt;";
        let result = decode_html_entities(text);
        assert_eq!(result, "Hello & goodbye <world>");
    }

    #[test]
    fn test_decode_html_entities_complex() {
        let text = "She said &ldquo;Hello&rdquo; &ndash; it&rsquo;s great!";
        let result = decode_html_entities(text);
        assert_eq!(
            result,
            "She said \u{201C}Hello\u{201D} \u{2013} it\u{2019}s great!"
        );
    }

    #[test]
    fn test_clean_whitespace() {
        let text = "Hello    world\n\n\nTest";
        let result = clean_whitespace(text);
        assert_eq!(result, "Hello world\n\nTest");
    }

    #[test]
    fn test_clean_whitespace_tabs() {
        let text = "Hello\t\tworld";
        let result = clean_whitespace(text);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_strip_html_with_entities() {
        let html = "<p>Hello &amp; <strong>world</strong> &lt;test&gt;</p>";
        let result = strip_html(html);
        assert_eq!(result, "Hello & world <test>");
    }

    #[test]
    fn test_audioboom_feed_example() {
        let html = r#"<div>In recognition of Hispanic Heritage Month, today's episode is dedicated to George Meléndez Wright, the first Hispanic person to occupy a professional role in the National Park Service. His life was cut tragically short, but his holistic approach to wildlife management in the National Parks has left an indelible mark.<br>
<br>
To submit a business for the Outsiders Gift Guide, please email <a href="mailto:assistant@npadpodcast.com">assistant@npadpodcast.com</a> by October 22nd :)<br>
<strong><br>
Sources:</strong><br>
<br>
Book: George Melendez Wright: The Fight for Wildlife and Wilderness in the National Parks by Jerry Emory<br>
<br>
Articles/Webpages: <a href="https://www.nps.gov/yose/learn/historyculture/wright.htm">National Park Service</a>, <a href="https://www.georgewrightsociety.org/gmw">George Wright Society</a></div>"#;

        let result = strip_html(html);

        // Should contain the main content
        assert!(result.contains("In recognition of Hispanic Heritage Month"));
        assert!(result.contains("George Meléndez Wright"));
        assert!(result.contains("National Park Service"));

        // Should not contain HTML tags
        assert!(!result.contains("<div>"));
        assert!(!result.contains("<br>"));
        assert!(!result.contains("<strong>"));
        assert!(!result.contains("<a href="));

        // Should be reasonably formatted
        assert!(!result.contains("  ")); // No double spaces
    }

    #[test]
    fn test_libsyn_clean_text() {
        // Libsyn feeds often have clean text that should pass through unchanged
        let text = "A 15-minute guided meditation to help you navigate feelings of sadness and low mood with mindfulness and self-compassion.";
        let result = strip_html(text);
        assert_eq!(result, text);
    }
}
