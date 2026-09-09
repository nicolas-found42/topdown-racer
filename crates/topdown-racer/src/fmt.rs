//! Formatting helpers shared by HUD, results, and best-lap projections.

/// Formats a time duration in seconds into `MM:SS.hh`.
pub fn format_time(seconds: f32) -> String {
    let total_hundredths = (seconds.max(0.0) * 100.0).round() as u32;
    let hundredths = total_hundredths % 100;
    let total_seconds = total_hundredths / 100;
    let secs = total_seconds % 60;
    let mins = total_seconds / 60;
    format!("{:02}:{:02}.{:02}", mins, secs, hundredths)
}

/// Formats an optional lap time, showing a placeholder when no lap is recorded.
pub fn format_opt_lap_time(seconds: Option<f32>) -> String {
    match seconds {
        Some(secs) => format_time(secs),
        None => "--:--.--".to_owned(),
    }
}

/// Formats a 1-indexed position with its ordinal suffix.
pub(crate) fn ordinal(position: usize) -> String {
    match position {
        1 => "1st".to_owned(),
        2 => "2nd".to_owned(),
        3 => "3rd".to_owned(),
        n => format!("{n}th"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_renders_minutes_seconds_hundredths() {
        assert_eq!(format_time(18.455), "00:18.46");
        assert_eq!(format_time(0.0), "00:00.00");
        assert_eq!(format_time(61.234), "01:01.23");
        assert_eq!(format_time(-1.0), "00:00.00");
    }

    #[test]
    fn format_opt_lap_time_uses_placeholder_when_absent() {
        assert_eq!(format_opt_lap_time(None), "--:--.--");
        assert_eq!(format_opt_lap_time(Some(18.455)), format_time(18.455));
    }

    #[test]
    fn ordinal_suffixes_the_top_three_then_th() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(12), "12th");
    }
}
