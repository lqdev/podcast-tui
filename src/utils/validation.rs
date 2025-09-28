use url::Url;

/// Validate if a string is a valid URL
pub fn is_valid_url(url_str: &str) -> bool {
    Url::parse(url_str).is_ok()
}

/// Validate if a string is a valid RSS/podcast feed URL
pub fn is_valid_feed_url(url_str: &str) -> bool {
    if !is_valid_url(url_str) {
        return false;
    }

    // Basic scheme validation - must be HTTP or HTTPS
    if let Ok(url) = Url::parse(url_str) {
        matches!(url.scheme(), "http" | "https")
    } else {
        false
    }
}

/// Validate episode title (must not be empty, reasonable length)
pub fn is_valid_episode_title(title: &str) -> bool {
    !title.trim().is_empty() && title.len() <= 500
}

/// Validate podcast title (must not be empty, reasonable length)
pub fn is_valid_podcast_title(title: &str) -> bool {
    !title.trim().is_empty() && title.len() <= 200
}

/// Clean and validate a filename for safe filesystem usage
pub fn sanitize_filename(filename: &str) -> String {
    // Remove or replace characters that are problematic in filenames
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*', '/', '\\'];
    let mut sanitized = filename.to_string();

    for invalid_char in invalid_chars {
        sanitized = sanitized.replace(invalid_char, "_");
    }

    // Trim whitespace and limit length
    sanitized.trim().chars().take(255).collect()
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
    fn test_filename_sanitization() {
        assert_eq!(
            sanitize_filename("Normal Filename.mp3"),
            "Normal Filename.mp3"
        );
        assert_eq!(sanitize_filename("File<>:Name|?.mp3"), "File___Name__.mp3");
        assert_eq!(sanitize_filename("  Trimmed  "), "Trimmed");
    }

    #[test]
    fn test_audio_format_validation() {
        assert!(is_supported_audio_format("episode.mp3"));
        assert!(is_supported_audio_format("episode.M4A"));
        assert!(is_supported_audio_format("episode.ogg"));
        assert!(!is_supported_audio_format("episode.txt"));
        assert!(!is_supported_audio_format("no_extension"));
    }
}
