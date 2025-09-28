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
}
