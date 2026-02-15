use chrono::{DateTime, Duration, Utc};

/// Format duration in seconds to HH:MM:SS or MM:SS format
pub fn format_duration(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

/// Parse duration string (HH:MM:SS or MM:SS) to seconds
pub fn parse_duration(duration_str: &str) -> Option<u32> {
    let parts: Vec<&str> = duration_str.split(':').collect();

    match parts.len() {
        2 => {
            // MM:SS format
            let minutes: u32 = parts[0].parse().ok()?;
            let seconds: u32 = parts[1].parse().ok()?;
            Some(minutes * 60 + seconds)
        }
        3 => {
            // HH:MM:SS format
            let hours: u32 = parts[0].parse().ok()?;
            let minutes: u32 = parts[1].parse().ok()?;
            let seconds: u32 = parts[2].parse().ok()?;
            Some(hours * 3600 + minutes * 60 + seconds)
        }
        _ => None,
    }
}

/// Get a human-readable "time ago" string
pub fn time_ago(datetime: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(datetime);

    if duration < Duration::minutes(1) {
        "just now".to_string()
    } else if duration < Duration::hours(1) {
        format!("{}m ago", duration.num_minutes())
    } else if duration < Duration::days(1) {
        format!("{}h ago", duration.num_hours())
    } else if duration < Duration::days(7) {
        format!("{}d ago", duration.num_days())
    } else if duration < Duration::days(30) {
        format!("{}w ago", duration.num_weeks())
    } else if duration < Duration::days(365) {
        format!("{}mo ago", duration.num_days() / 30)
    } else {
        format!("{}y ago", duration.num_days() / 365)
    }
}

/// Alias for time_ago - formats a DateTime as a relative time string
pub fn format_relative_time(datetime: &DateTime<Utc>) -> String {
    time_ago(*datetime)
}

/// Parse a human-readable duration string into total hours.
///
/// Supported formats: `12h` (hours), `7d` (days), `2w` (weeks), `1m` (months = 30 days)
/// If no suffix, defaults to days.
///
/// Returns `None` if the input is invalid or zero.
pub fn parse_cleanup_duration(input: &str) -> Option<u64> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return None;
    }

    let (num_str, multiplier_hours) = if input.ends_with('h') {
        (&input[..input.len() - 1], 1u64)
    } else if input.ends_with('d') {
        (&input[..input.len() - 1], 24u64)
    } else if input.ends_with('w') {
        (&input[..input.len() - 1], 24 * 7)
    } else if input.ends_with('m') {
        (&input[..input.len() - 1], 24 * 30)
    } else {
        // Default to days if no suffix
        (input.as_str(), 24u64)
    };

    let value: u64 = num_str.parse().ok()?;
    if value == 0 {
        return None;
    }

    let total_hours = value.checked_mul(multiplier_hours)?;

    // Validate: must be >= 1h and <= 365 days (8760 hours)
    if total_hours > 8760 {
        return None;
    }

    Some(total_hours)
}

/// Format a duration in hours back to human-readable form for display.
pub fn format_cleanup_duration(total_hours: u64) -> String {
    if total_hours.is_multiple_of(24 * 30) && total_hours / (24 * 30) > 0 {
        let months = total_hours / (24 * 30);
        if months == 1 {
            "1 month".to_string()
        } else {
            format!("{} months", months)
        }
    } else if total_hours.is_multiple_of(24 * 7) && total_hours / (24 * 7) > 0 {
        let weeks = total_hours / (24 * 7);
        if weeks == 1 {
            "1 week".to_string()
        } else {
            format!("{} weeks", weeks)
        }
    } else if total_hours.is_multiple_of(24) && total_hours / 24 > 0 {
        let days = total_hours / 24;
        if days == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", days)
        }
    } else if total_hours == 1 {
        "1 hour".to_string()
    } else {
        format!("{} hours", total_hours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(61), "1:01");
        assert_eq!(format_duration(3661), "1:01:01");
        assert_eq!(format_duration(30), "0:30");
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1:01"), Some(61));
        assert_eq!(parse_duration("1:01:01"), Some(3661));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_parse_cleanup_duration_hours() {
        assert_eq!(parse_cleanup_duration("12h"), Some(12));
        assert_eq!(parse_cleanup_duration("1h"), Some(1));
        assert_eq!(parse_cleanup_duration("24h"), Some(24));
    }

    #[test]
    fn test_parse_cleanup_duration_days() {
        assert_eq!(parse_cleanup_duration("7d"), Some(168));
        assert_eq!(parse_cleanup_duration("1d"), Some(24));
        assert_eq!(parse_cleanup_duration("30d"), Some(720));
    }

    #[test]
    fn test_parse_cleanup_duration_weeks() {
        assert_eq!(parse_cleanup_duration("2w"), Some(336));
        assert_eq!(parse_cleanup_duration("1w"), Some(168));
        assert_eq!(parse_cleanup_duration("4w"), Some(672));
    }

    #[test]
    fn test_parse_cleanup_duration_months() {
        assert_eq!(parse_cleanup_duration("1m"), Some(720));
        assert_eq!(parse_cleanup_duration("6m"), Some(4320));
    }

    #[test]
    fn test_parse_cleanup_duration_default_days() {
        // No suffix defaults to days
        assert_eq!(parse_cleanup_duration("30"), Some(720));
        assert_eq!(parse_cleanup_duration("7"), Some(168));
    }

    #[test]
    fn test_parse_cleanup_duration_invalid() {
        assert_eq!(parse_cleanup_duration("0d"), None);
        assert_eq!(parse_cleanup_duration(""), None);
        assert_eq!(parse_cleanup_duration("abc"), None);
        assert_eq!(parse_cleanup_duration("0h"), None);
        // Over 365 days
        assert_eq!(parse_cleanup_duration("366d"), None);
    }

    #[test]
    fn test_parse_cleanup_duration_case_insensitive() {
        assert_eq!(parse_cleanup_duration("7D"), Some(168));
        assert_eq!(parse_cleanup_duration("2W"), Some(336));
        assert_eq!(parse_cleanup_duration("1M"), Some(720));
        assert_eq!(parse_cleanup_duration("12H"), Some(12));
    }

    #[test]
    fn test_parse_cleanup_duration_whitespace() {
        assert_eq!(parse_cleanup_duration("  7d  "), Some(168));
        assert_eq!(parse_cleanup_duration(" 2w "), Some(336));
    }

    #[test]
    fn test_format_cleanup_duration() {
        assert_eq!(format_cleanup_duration(1), "1 hour");
        assert_eq!(format_cleanup_duration(12), "12 hours");
        assert_eq!(format_cleanup_duration(24), "1 day");
        assert_eq!(format_cleanup_duration(168), "1 week");
        assert_eq!(format_cleanup_duration(336), "2 weeks");
        assert_eq!(format_cleanup_duration(720), "1 month");
        assert_eq!(format_cleanup_duration(1440), "2 months");
        assert_eq!(format_cleanup_duration(48), "2 days");
    }
}
