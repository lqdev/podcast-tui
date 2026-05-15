//! Device-side filename template engine.
//!
//! Pure module — no I/O, no `tokio`. Renders a template string into a
//! relative `PathBuf` suitable for placing on a device (MP3 player, USB
//! drive) where the local readable-filename layout doesn't apply.
//!
//! # Token reference
//!
//! | Token              | Substitution                                                       |
//! |--------------------|--------------------------------------------------------------------|
//! | `{podcast}`        | Podcast title, sanitized                                           |
//! | `{podcast_short}`  | Podcast title, sanitized then truncated to 30 chars                |
//! | `{title}`          | Episode title, sanitized                                           |
//! | `{track}`          | Episode number (no padding); empty string if missing               |
//! | `{track:NN}`       | Episode number, zero-padded to N digits (`{track:03}` → `007`)     |
//! | `{episode_number}` | Alias for `{track}`                                                |
//! | `{episode_number:NN}` | Alias for `{track:NN}`                                          |
//! | `{date}`           | Published date, default format `YYYY-MM-DD`                        |
//! | `{date:%fmt}`      | Published date with `chrono` strftime format                       |
//! | `{ext}`            | File extension (e.g. `mp3`) without leading dot                    |
//!
//! Literal `/` in the template becomes a path separator (creates subfolders).
//!
//! # Options
//!
//! * `max_length` — cap (in bytes) applied to **each path segment** after
//!   substitution. Honors UTF-8 boundaries.
//! * `ascii_only` — strip any remaining non-ASCII characters after sanitization.
//!
//! # Example
//!
//! ```ignore
//! use podcast_tui::download::device_template::{render, DeviceFilenameOptions};
//!
//! let path = render(
//!     "{podcast_short}/{track:03} - {title}.{ext}",
//!     &podcast,
//!     &episode,
//!     "mp3",
//!     &DeviceFilenameOptions { max_length: 64, ascii_only: true },
//! )?;
//! ```

use crate::podcast::{Episode, Podcast};
use crate::storage::EpisodeId;
use std::path::{Path, PathBuf};

use super::sanitize::sanitize_filename;

/// Errors that can occur while rendering a device filename template.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemplateError {
    /// The template referenced a token name that is not in the [token reference](self#token-reference).
    #[error("Unknown token: {0}")]
    UnknownToken(String),
    /// The template contains an unbalanced `{` or other syntactically invalid token.
    #[error("Malformed token: {0}")]
    Malformed(String),
}

/// Options controlling how a rendered template is post-processed.
#[derive(Debug, Clone)]
pub struct DeviceFilenameOptions {
    /// Maximum byte length for each path segment after substitution.
    /// Truncation honors UTF-8 character boundaries.
    pub max_length: usize,
    /// If true, strip non-ASCII characters from each segment after sanitization.
    pub ascii_only: bool,
}

/// Maximum length for `{podcast_short}` (in characters).
const PODCAST_SHORT_MAX: usize = 30;

#[derive(Debug, PartialEq, Eq)]
enum Segment<'a> {
    Literal(&'a str),
    Token(&'a str),
}

/// Tokenize a template string into literal and token segments.
fn tokenize(template: &str) -> Result<Vec<Segment<'_>>, TemplateError> {
    let mut out = Vec::new();
    let bytes = template.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() {
        if bytes[cursor] == b'{' {
            // Find the matching '}'
            let close = template[cursor + 1..].find('}').ok_or_else(|| {
                TemplateError::Malformed(format!(
                    "unterminated '{{' at position {} in template",
                    cursor
                ))
            })?;
            let token = &template[cursor + 1..cursor + 1 + close];
            if token.is_empty() {
                return Err(TemplateError::Malformed("empty token '{}'".to_string()));
            }
            if token.contains('{') {
                return Err(TemplateError::Malformed(format!(
                    "nested '{{' inside token '{}'",
                    token
                )));
            }
            out.push(Segment::Token(token));
            cursor += 1 + close + 1;
        } else {
            // Literal until next '{'
            let next = template[cursor..]
                .find('{')
                .map(|n| cursor + n)
                .unwrap_or(bytes.len());
            out.push(Segment::Literal(&template[cursor..next]));
            cursor = next;
        }
    }

    Ok(out)
}

/// Substitute a single token from podcast/episode data.
fn substitute_token(
    token: &str,
    podcast: &Podcast,
    episode: &Episode,
    ext: &str,
) -> Result<String, TemplateError> {
    // Tokens with parameters use ':' as separator.
    let (name, param) = match token.find(':') {
        Some(idx) => (&token[..idx], Some(&token[idx + 1..])),
        None => (token, None),
    };

    match (name, param) {
        ("podcast", None) => Ok(podcast.title.clone()),
        ("podcast_short", None) => {
            // Sanitize first, then truncate the *sanitized* string to 30
            // characters so the doc-promised cap holds even when the
            // sanitizer rewrites length (e.g. `&` → "and").
            Ok(truncate_chars(
                &sanitize_filename(&podcast.title, false),
                PODCAST_SHORT_MAX,
            ))
        }
        ("title", None) => Ok(episode.title.clone()),
        ("track" | "episode_number", None) => Ok(episode
            .episode_number
            .map(|n| n.to_string())
            .unwrap_or_default()),
        ("track" | "episode_number", Some(spec)) => {
            // Spec must be a positive width (NN). Zero is rejected to match
            // the documented behavior of zero-padding (`{track:0}` is meaningless).
            let width: usize = spec.parse().map_err(|_| {
                TemplateError::Malformed(format!(
                    "invalid width '{}' in {{{}:{}}}; expected a positive integer",
                    spec, name, spec
                ))
            })?;
            if width == 0 {
                return Err(TemplateError::Malformed(format!(
                    "invalid width '0' in {{{}:0}}; expected a positive integer",
                    name
                )));
            }
            Ok(episode
                .episode_number
                .map(|n| format!("{:0width$}", n, width = width))
                .unwrap_or_default())
        }
        ("date", None) => Ok(episode.published.format("%Y-%m-%d").to_string()),
        ("date", Some(fmt)) => Ok(episode.published.format(fmt).to_string()),
        ("ext", None) => Ok(ext.trim_start_matches('.').to_string()),
        _ => Err(TemplateError::UnknownToken(name.to_string())),
    }
}

/// Truncate a string to `max` characters (not bytes).
fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Cap a string to `max` bytes, honoring UTF-8 boundaries.
///
/// `max == 0` returns an empty string — callers must enforce the cap even
/// when extension reservation consumes the whole budget.
fn cap_bytes(mut s: String, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.len() <= max {
        return s;
    }
    s.truncate(max);
    while !s.is_char_boundary(s.len()) {
        s.pop();
    }
    s
}

/// Strip any non-ASCII characters from `s`.
fn strip_non_ascii(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii()).collect()
}

/// Render a template into a relative path.
///
/// The returned path is intentionally relative — callers join it onto the
/// device root before performing I/O. See module-level docs for the token
/// reference and behavior of [`DeviceFilenameOptions`].
pub fn render(
    template: &str,
    podcast: &Podcast,
    episode: &Episode,
    ext: &str,
    opts: &DeviceFilenameOptions,
) -> Result<PathBuf, TemplateError> {
    if template.trim().is_empty() {
        return Err(TemplateError::Malformed(
            "template is empty or whitespace-only".to_string(),
        ));
    }

    // 1. Split the *template* on '/' first — only literal slashes in the
    //    template should create subfolders. Slashes that appear inside a
    //    substituted token value (e.g. `{podcast}` of "Foo/Bar") are then
    //    sanitized away within their segment instead of being interpreted
    //    as path separators.
    let template_segments: Vec<&str> = template.split('/').collect();
    let last_idx = template_segments.len().saturating_sub(1);

    let mut path = PathBuf::new();

    for (idx, seg_template) in template_segments.iter().enumerate() {
        // Tokenize and substitute within this segment.
        let parsed = tokenize(seg_template)?;
        let mut rendered = String::new();
        for piece in parsed {
            match piece {
                Segment::Literal(lit) => rendered.push_str(lit),
                Segment::Token(tok) => {
                    let value = substitute_token(tok, podcast, episode, ext)?;
                    rendered.push_str(&value);
                }
            }
        }

        if rendered.is_empty() {
            continue;
        }

        // For the final segment (the file), preserve the extension separately
        // so length capping doesn't eat the ".mp3" suffix.
        let is_last = idx == last_idx;
        let (stem, ext_part) = if is_last {
            split_extension(&rendered)
        } else {
            (rendered.clone(), String::new())
        };

        // Sanitize via shared sanitizer (treats segment as folder when not last).
        let mut sanitized = sanitize_filename(&stem, !is_last);
        if opts.ascii_only {
            sanitized = strip_non_ascii(&sanitized);
            if sanitized.is_empty() {
                sanitized = if is_last { "Episode" } else { "Podcast" }.to_string();
            }
        }

        // Apply per-segment length cap, reserving room for ".ext".
        let reserve = if is_last && !ext_part.is_empty() {
            ext_part.len() + 1 // for the leading '.'
        } else {
            0
        };
        let cap = opts.max_length.saturating_sub(reserve);
        sanitized = cap_bytes(sanitized, cap);

        // If the cap consumed the entire stem (e.g. max_length too small to
        // fit even one stem byte alongside the reserved extension), fall
        // back to a single-character placeholder so we never emit a dotfile
        // like ".mp3".
        if sanitized.is_empty() {
            sanitized = "_".to_string();
        }

        let final_segment = if is_last && !ext_part.is_empty() {
            format!("{}.{}", sanitized, ext_part)
        } else {
            sanitized
        };

        path.push(final_segment);
    }

    if path.as_os_str().is_empty() {
        return Err(TemplateError::Malformed(
            "template rendered to an empty path".to_string(),
        ));
    }

    Ok(path)
}

/// Split a filename on the last `.` into `(stem, ext)`.
/// If no `.` is present, returns `(name, "")`.
fn split_extension(name: &str) -> (String, String) {
    match name.rfind('.') {
        Some(idx) if idx > 0 && idx < name.len() - 1 => {
            (name[..idx].to_string(), name[idx + 1..].to_string())
        }
        _ => (name.to_string(), String::new()),
    }
}

/// Disambiguate a path on collision by appending the last 6 chars of the
/// episode ID before the extension.
///
/// `Foo.mp3` + `EpisodeId(...abc123)` → `Foo-abc123.mp3`.
pub fn disambiguate(path: &Path, episode_id: &EpisodeId) -> PathBuf {
    let id_str = episode_id.to_string();
    // Use last 6 chars (chars, not bytes — id_str is hex/uuid so ASCII anyway).
    let suffix: String = id_str.chars().rev().take(6).collect::<String>();
    let suffix: String = suffix.chars().rev().collect();

    let parent = path.parent();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let (stem, ext) = split_extension(file_name);
    let new_name = if ext.is_empty() {
        format!("{}-{}", stem, suffix)
    } else {
        format!("{}-{}.{}", stem, suffix, ext)
    };

    match parent {
        Some(p) if !p.as_os_str().is_empty() => p.join(new_name),
        _ => PathBuf::from(new_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podcast::{Episode, EpisodeStatus, Podcast};
    use crate::storage::{EpisodeId, PodcastId};
    use chrono::TimeZone;

    fn make_podcast(title: &str) -> Podcast {
        Podcast {
            id: PodcastId::new(),
            title: title.to_string(),
            url: "https://example.com/feed.xml".to_string(),
            description: None,
            author: None,
            image_url: None,
            language: None,
            categories: vec![],
            explicit: false,
            last_updated: chrono::Utc::now(),
            episodes: vec![],
            tags: vec![],
            last_etag: None,
            last_modified: None,
            last_body_hash: None,
        }
    }

    fn make_episode(title: &str, number: Option<u32>) -> Episode {
        Episode {
            id: EpisodeId::new(),
            podcast_id: PodcastId::new(),
            title: title.to_string(),
            description: None,
            audio_url: "https://example.com/ep.mp3".to_string(),
            published: chrono::Utc.with_ymd_and_hms(2024, 3, 15, 12, 0, 0).unwrap(),
            duration: None,
            file_size: None,
            mime_type: None,
            guid: None,
            link: None,
            image_url: None,
            explicit: false,
            season: None,
            episode_number: number,
            episode_type: None,
            status: EpisodeStatus::New,
            local_path: None,
            last_played_position: None,
            play_count: 0,
            notes: None,
            chapters: vec![],
            transcript: None,
            favorited: false,
        }
    }

    fn default_opts() -> DeviceFilenameOptions {
        DeviceFilenameOptions {
            max_length: 128,
            ascii_only: false,
        }
    }

    #[test]
    fn token_podcast() {
        let p = make_podcast("My Show");
        let e = make_episode("Hello", Some(1));
        let out = render("{podcast}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("My Show.mp3"));
    }

    #[test]
    fn token_podcast_short_truncates_to_30_chars() {
        let p = make_podcast("This is a very long podcast title that exceeds thirty chars");
        let e = make_episode("Ep", Some(1));
        let out = render("{podcast_short}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        // 30 chars of the title (sanitizer is lossless for ASCII alphanumerics + spaces)
        let stem = "This is a very long podcast ti";
        assert_eq!(out, PathBuf::from(format!("{}.mp3", stem)));
    }

    #[test]
    fn token_title() {
        let p = make_podcast("Show");
        let e = make_episode("My Episode", None);
        let out = render("{title}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("My Episode.mp3"));
    }

    #[test]
    fn token_track_unpadded() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(7));
        let out = render("{track}-{title}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("7-Ep.mp3"));
    }

    #[test]
    fn token_track_zero_padded() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(7));
        let out = render("{track:03}-{title}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("007-Ep.mp3"));
    }

    #[test]
    fn token_track_zero_width_rejected() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(7));
        let err = render("{track:0}.{ext}", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn token_episode_number_alias() {
        // `{episode_number}` and `{episode_number:NN}` are aliases for `{track}`
        // and `{track:NN}` so DeviceProfile templates can use the more readable
        // name without breaking the engine.
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(7));
        let out = render(
            "{episode_number:03}-{title}.{ext}",
            &p,
            &e,
            "mp3",
            &default_opts(),
        )
        .unwrap();
        assert_eq!(out, PathBuf::from("007-Ep.mp3"));

        let out = render(
            "{episode_number}-{title}.{ext}",
            &p,
            &e,
            "mp3",
            &default_opts(),
        )
        .unwrap();
        assert_eq!(out, PathBuf::from("7-Ep.mp3"));
    }

    #[test]
    fn token_podcast_short_truncates_after_sanitization() {
        // Verify the doc-promised order: sanitize first, then truncate.
        // `&` → "and" expands the string, so naive raw-truncation could
        // either fall short or exceed 30 chars; sanitize-then-truncate
        // gives a deterministic 30-char cap on the *output*.
        let p = make_podcast("AAA & BBB & CCC & DDD & EEE & FFFFFFF");
        let e = make_episode("Ep", Some(1));
        let out = render("{podcast_short}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        let stem = out.file_stem().unwrap().to_string_lossy().into_owned();
        assert!(
            stem.chars().count() <= 30,
            "expected ≤30 chars after sanitize+truncate, got {} ({:?})",
            stem.chars().count(),
            stem
        );
    }

    #[test]
    fn cap_smaller_than_extension_falls_back_safely() {
        // max_length=3 with "mp3" extension reservation = "mp3" + "." = 4 bytes
        // -> reserved bytes exceed budget; cap_bytes returns "" and the engine
        // falls back to "_" so we never emit a hidden ".mp3" dotfile.
        let p = make_podcast("Show");
        let e = make_episode("Episode Title", Some(1));
        let opts = DeviceFilenameOptions {
            max_length: 3,
            ascii_only: false,
        };
        let out = render("{title}.{ext}", &p, &e, "mp3", &opts).unwrap();
        let file = out.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            !file.starts_with('.'),
            "must not produce a dotfile: {}",
            file
        );
        assert!(file.contains(".mp3"));
    }

    #[test]
    fn token_track_missing_episode_number_renders_empty() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", None);
        // With unpadded {track} and no number, "track" segment is empty -> "-Ep.mp3"
        let out = render("{track}-{title}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("-Ep.mp3"));
    }

    #[test]
    fn token_date_default() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let out = render("{date}-{title}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("2024-03-15-Ep.mp3"));
    }

    #[test]
    fn token_date_custom_format() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let out = render("{date:%Y%m%d}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("20240315.mp3"));
    }

    #[test]
    fn token_ext() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let out = render("{title}.{ext}", &p, &e, "m4a", &default_opts()).unwrap();
        assert_eq!(out, PathBuf::from("Ep.m4a"));
    }

    #[test]
    fn literal_slash_creates_subdirectory() {
        let p = make_podcast("MyShow");
        let e = make_episode("Ep1", Some(1));
        let out = render(
            "{podcast}/{track:02}-{title}.{ext}",
            &p,
            &e,
            "mp3",
            &default_opts(),
        )
        .unwrap();
        assert_eq!(out, PathBuf::from("MyShow").join("01-Ep1.mp3"));
    }

    #[test]
    fn ascii_only_strips_non_ascii() {
        // The shared sanitizer folds Café -> Cafe; 日本 has no folding, so it's
        // dropped by the sanitizer's catch-all already. Use a char that survives
        // the sanitizer to verify ascii_only's independent behavior.
        // U+00B5 MICRO SIGN (µ) is not in the sanitizer's table and gets dropped
        // by the `_ => {}` arm — so it won't reach the ascii_only step. Instead,
        // we verify the documented behavior: any non-ASCII char *that survived
        // sanitization* is stripped. Café is sanitizer-folded to "Cafe".
        let p = make_podcast("Café");
        let e = make_episode("Episode", Some(1));
        let opts = DeviceFilenameOptions {
            max_length: 64,
            ascii_only: true,
        };
        let out = render("{podcast}.{ext}", &p, &e, "mp3", &opts).unwrap();
        assert_eq!(out, PathBuf::from("Cafe.mp3"));
        // And confirm every byte is ASCII.
        let s = out.to_string_lossy();
        assert!(s.is_ascii(), "expected ASCII output, got {}", s);
    }

    #[test]
    fn length_cap_respected_per_segment() {
        let p = make_podcast("PodcastFolder");
        let e = make_episode(
            "An episode with a very long title that should get capped",
            Some(1),
        );
        let opts = DeviceFilenameOptions {
            max_length: 16,
            ascii_only: false,
        };
        let out = render("{podcast}/{title}.{ext}", &p, &e, "mp3", &opts).unwrap();
        // Folder segment capped to 16 bytes (no extension reserve)
        let folder = out.parent().unwrap().to_string_lossy().into_owned();
        assert!(
            folder.len() <= 16,
            "folder too long: {} ({})",
            folder,
            folder.len()
        );
        // File segment: stem capped so total (stem + "." + ext) <= 16
        let file = out.file_name().unwrap().to_string_lossy().into_owned();
        assert!(file.len() <= 16, "file too long: {} ({})", file, file.len());
        assert!(file.ends_with(".mp3"));
    }

    #[test]
    fn unknown_token_returns_error() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let err = render("{foo}.{ext}", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert_eq!(err, TemplateError::UnknownToken("foo".to_string()));
    }

    #[test]
    fn empty_template_returns_error() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let err = render("", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
        let err = render("   ", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn unterminated_token_returns_error() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let err = render("{title.{ext}", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn empty_token_returns_error() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(1));
        let err = render("{}.{ext}", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn invalid_track_width_returns_error() {
        let p = make_podcast("Show");
        let e = make_episode("Ep", Some(7));
        let err = render("{track:abc}.{ext}", &p, &e, "mp3", &default_opts()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn substituted_slash_does_not_create_subdir() {
        // A '/' inside a podcast title must be sanitized away (turned into '-'),
        // not interpreted as a path separator.
        let p = make_podcast("Slash/Show");
        let e = make_episode("Ep", Some(1));
        let out = render("{podcast}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        // No subdirectory created.
        assert_eq!(out.components().count(), 1);
        let s = out.to_string_lossy();
        assert!(!s.contains('/') || std::path::MAIN_SEPARATOR != '/');
    }

    #[test]
    fn sanitization_strips_prohibited_chars() {
        let p = make_podcast("Show");
        let e = make_episode("Ep:1<test>", Some(1));
        let out = render("{title}.{ext}", &p, &e, "mp3", &default_opts()).unwrap();
        let s = out.to_string_lossy();
        assert!(!s.contains(':'));
        assert!(!s.contains('<'));
        assert!(!s.contains('>'));
    }

    #[test]
    fn disambiguate_appends_id_suffix() {
        let id = EpisodeId::new();
        let id_str = id.to_string();
        let suffix: String = id_str.chars().rev().take(6).collect::<String>();
        let suffix: String = suffix.chars().rev().collect();

        let original = PathBuf::from("Foo.mp3");
        let out = disambiguate(&original, &id);
        assert_eq!(out, PathBuf::from(format!("Foo-{}.mp3", suffix)));
    }

    #[test]
    fn disambiguate_distinguishes_different_ids() {
        let id1 = EpisodeId::new();
        let id2 = EpisodeId::new();
        let original = PathBuf::from("Same.mp3");
        let a = disambiguate(&original, &id1);
        let b = disambiguate(&original, &id2);
        assert_ne!(a, b, "different IDs must produce different filenames");
    }

    #[test]
    fn disambiguate_preserves_parent_directory() {
        let id = EpisodeId::new();
        let original = PathBuf::from("MyShow").join("Foo.mp3");
        let out = disambiguate(&original, &id);
        assert_eq!(out.parent(), Some(Path::new("MyShow")));
    }

    #[test]
    fn disambiguate_handles_no_extension() {
        let id = EpisodeId::new();
        let id_str = id.to_string();
        let suffix: String = id_str.chars().rev().take(6).collect::<String>();
        let suffix: String = suffix.chars().rev().collect();

        let original = PathBuf::from("Foo");
        let out = disambiguate(&original, &id);
        assert_eq!(out, PathBuf::from(format!("Foo-{}", suffix)));
    }
}
