use url::Url;

/// Validate if a string is a valid URL
pub fn is_valid_url(url_str: &str) -> bool {
    Url::parse(url_str).is_ok()
}

/// Check if a URL is a valid feed URL (RSS/Atom)
pub fn is_valid_feed_url(url: &str) -> bool {
    is_valid_url(url) && (url.starts_with("http://") || url.starts_with("https://"))
}

/// Validate a feed URL and return a Result
pub fn validate_feed_url(url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("Feed URL cannot be empty".to_string());
    }

    if !is_valid_feed_url(url) {
        return Err("Invalid feed URL format".to_string());
    }

    Ok(())
}

/// Validate episode title (must not be empty, reasonable length)
pub fn is_valid_episode_title(title: &str) -> bool {
    !title.trim().is_empty() && title.len() <= 500
}

/// Validate podcast title (must not be empty, reasonable length)
pub fn is_valid_podcast_title(title: &str) -> bool {
    !title.trim().is_empty() && title.len() <= 200
}

// NOTE: A previous `sanitize_filename` helper lived here but was unused
// and superseded by [`crate::download::sanitize::sanitize_filename`], the
// canonical cross-platform sanitizer for any file written to disk.
// Use that for download paths, sync targets, and device-side filenames.
// Use [`sanitize_playlist_name`] (below) only for playlist directory names,
// which have a distinct contract (100-char cap, underscore replacement,
// no Unicode folding).

/// Sanitize playlist names for safe filesystem directory usage.
///
/// Distinct from [`crate::download::sanitize::sanitize_filename`]: this
/// helper is tailored for playlist *directories* (shorter cap, simpler
/// replacement strategy, no Unicode folding). For audio file names, use
/// the download sanitizer instead.
pub fn sanitize_playlist_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "Untitled".to_string();
    }

    let mut sanitized = String::new();
    for ch in trimmed.chars() {
        match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => sanitized.push('-'),
            c if c.is_control() => {}
            c if c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '(' | ')') => {
                sanitized.push(c)
            }
            _ => sanitized.push('_'),
        }
    }

    let normalized: String = sanitized
        .trim()
        .trim_matches(|c: char| c == '.' || c == '-' || c == ' ')
        .chars()
        .take(100)
        .collect();

    if normalized.is_empty() {
        "Untitled".to_string()
    } else {
        normalized
    }
}

/// Validate audio file extension
pub fn is_supported_audio_format(filename: &str) -> bool {
    let supported_extensions = ["mp3", "m4a", "aac", "ogg", "wav", "flac"];

    if let Some(extension) = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        supported_extensions.contains(&extension.to_lowercase().as_str())
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://example.com/feed.xml"));
        assert!(!is_valid_url("not a url"));
        assert!(!is_valid_url(""));
    }

    #[test]
    fn test_feed_url_validation() {
        assert!(is_valid_feed_url("https://example.com/feed.xml"));
        assert!(is_valid_feed_url("http://example.com/rss"));
        assert!(!is_valid_feed_url("ftp://example.com/feed.xml"));
        assert!(!is_valid_feed_url("not a url"));
    }

    #[test]
    fn test_title_validation() {
        assert!(is_valid_episode_title("Valid Episode Title"));
        assert!(!is_valid_episode_title(""));
        assert!(!is_valid_episode_title("   "));

        assert!(is_valid_podcast_title("Valid Podcast"));
        assert!(!is_valid_podcast_title(""));
    }

    #[test]
    fn test_audio_format_validation() {
        assert!(is_supported_audio_format("episode.mp3"));
        assert!(is_supported_audio_format("episode.M4A"));
        assert!(is_supported_audio_format("episode.ogg"));
        assert!(!is_supported_audio_format("episode.txt"));
        assert!(!is_supported_audio_format("no_extension"));
    }

    #[test]
    fn test_playlist_name_sanitization() {
        assert_eq!(sanitize_playlist_name("Morning Commute"), "Morning Commute");
        assert_eq!(sanitize_playlist_name("My: Playlist?"), "My- Playlist");
        assert_eq!(sanitize_playlist_name("   "), "Untitled");
        assert_eq!(sanitize_playlist_name("***"), "Untitled");
    }
}
