//! Subtitle clock times and the timing operations of PotPlayer's Subtitle
//! Saving (Apply Subtitle Sync) and Convert Subtitle FPS.

use crate::Cue;

/// A clock time in milliseconds: `[h:]mm:ss` with a fraction after `.` or
/// `,` (SRT's `00:01:02,500`, `WebVTT`'s `01:02.500`, ASS's `0:01:02.50`).
/// The fraction is decimal: `.5` and `.50` are 500 ms; digits after the
/// third are ignored.
#[must_use]
pub fn parse_clock(text: &str) -> Option<i64> {
    let text = text.trim();
    let (clock, fraction) = match text.rfind(['.', ',']) {
        Some(at) => (&text[..at], &text[at + 1..]),
        None => (text, ""),
    };
    let parts: Vec<&str> = clock.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [h, m, s] => (*h, *m, *s),
        [m, s] => ("0", *m, *s),
        _ => return None,
    };
    let number = |part: &str| -> Option<i64> {
        let part = part.trim();
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let (hours, minutes, seconds) = (number(hours)?, number(minutes)?, number(seconds)?);
    if minutes >= 60 || seconds >= 60 {
        return None;
    }
    let mut millis = 0;
    if !fraction.is_empty() {
        if !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut scale = 100;
        for digit in fraction.bytes().take(3) {
            millis += i64::from(digit - b'0') * scale;
            scale /= 10;
        }
    }
    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + millis)
}

/// A clock time as SRT (`separator` `,`) or `WebVTT` (`.`) writes it:
/// `HH:MM:SS,mmm`, with more hour digits when needed. Negative times are 0.
#[must_use]
pub fn format_clock(ms: i64, separator: char) -> String {
    let ms = ms.max(0);
    let (hours, rest) = (ms / 3_600_000, ms % 3_600_000);
    format!(
        "{hours:02}:{:02}:{:02}{separator}{:03}",
        rest / 60_000,
        rest % 60_000 / 1000,
        rest % 1000
    )
}

/// A clock time as ASS writes it: `h:mm:ss.cc`, rounded to the nearest
/// centisecond. Negative times are 0.
pub(crate) fn format_ass(ms: i64) -> String {
    let cs = (ms.max(0) + 5) / 10;
    let (hours, rest) = (cs / 360_000, cs % 360_000);
    format!(
        "{hours}:{:02}:{:02}.{:02}",
        rest / 6000,
        rest % 6000 / 100,
        rest % 100
    )
}

/// [`crate::Document::shift`].
pub(crate) fn shift(cues: &mut Vec<Cue>, delay_ms: i64) {
    cues.retain_mut(|cue| {
        cue.start_ms = cue.start_ms.saturating_add(delay_ms).max(0);
        cue.end_ms = cue.end_ms.saturating_add(delay_ms);
        cue.end_ms > 0
    });
}

/// [`crate::Document::scale`].
pub(crate) fn scale(cues: &mut [Cue], from_fps: f64, to_fps: f64) {
    let valid = |fps: f64| fps.is_finite() && fps > 0.0;
    if !valid(from_fps) || !valid(to_fps) {
        return;
    }
    let factor = from_fps / to_fps;
    // Subtitle times stay far below 2^53 ms, where f64 is exact, and the
    // product is rounded back to whole milliseconds.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let scaled = |ms: i64| (ms as f64 * factor).round() as i64;
    for cue in cues {
        cue.start_ms = scaled(cue.start_ms);
        cue.end_ms = scaled(cue.end_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clocks_read_every_format_and_write_srt_vtt_and_ass() {
        assert_eq!(parse_clock("00:01:02,500"), Some(62_500));
        assert_eq!(parse_clock("01:02.5"), Some(62_500));
        assert_eq!(parse_clock("0:01:02.05"), Some(62_050));
        assert_eq!(parse_clock("1:00:00.000"), Some(3_600_000));
        assert_eq!(parse_clock("12:00:00.1239"), Some(43_200_123));
        assert_eq!(parse_clock("00:61:00,000"), None);
        assert_eq!(parse_clock("aa:00:00,000"), None);
        assert_eq!(format_clock(62_500, ','), "00:01:02,500");
        assert_eq!(format_clock(100 * 3_600_000 + 1, '.'), "100:00:00.001");
        assert_eq!(format_clock(-5, ','), "00:00:00,000");
        assert_eq!(format_ass(62_504), "0:01:02.50");
        assert_eq!(format_ass(62_505), "0:01:02.51");
        assert_eq!(format_ass(36_000_000), "10:00:00.00");
    }
}
