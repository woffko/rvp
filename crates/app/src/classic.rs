//! M9: what RVP shows in the skins it builds from classic Winamp skins
//! (`rvp_skin::winamp::build`): their `RVP:` texts, states and sliders.
//!
//! * The time is minutes and seconds in two digit labels (the colon is
//!   the skin's), elapsed or, with Skins > "Reverse times", remaining with
//!   a minus sign before it; blank while nothing plays, and blinking while
//!   paused, as Winamp does (half a second each).
//! * The song title runs as Winamp's: `N. Title (m:ss)`; longer than the
//!   31 characters that fit, it scrolls one character every 200 ms with
//!   `  ***  ` between its ends.
//! * The rates: kbps in three characters (from 1000 the hundreds with an
//!   `H`, from 10000 the thousands with a `C`, as Winamp shortens them),
//!   kHz in two.
//! * The states: playing, paused or stopped; mono and stereo lit or not.
//! * Winamp's balance and equalizer preamp have no counterpart in RVP:
//!   their sliders stay in the middle.

use std::time::Duration;

use rvp_engine::PropertyId;
use rvp_skin::{DataModel, SliderValue, UiState};

use crate::app::App;

/// The characters of the title that fit (154 pixels of 5).
const MARQUEE_CHARS: usize = 31;
/// One character every this long.
const MARQUEE_STEP: Duration = Duration::from_millis(200);
/// Paused digits blink at this half period.
const BLINK: Duration = Duration::from_millis(500);

/// The states the classic skins show, all of them, so that one goes off
/// when another comes.
pub const STATES: [&str; 7] = [
    "RVP:playing",
    "RVP:paused",
    "RVP:stopped",
    "RVP:stereo",
    "RVP:no-stereo",
    "RVP:mono",
    "RVP:no-mono",
];

/// `m:ss`, or `h:mm:ss` from an hour.
fn short_time(seconds: f64) -> String {
    // Times of a file: far below u32's range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = seconds.max(0.0).floor() as u64;
    if total >= 3600 {
        format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
    } else {
        format!("{}:{:02}", total / 60, total % 60)
    }
}

/// Winamp's three-character kbps.
fn kbps(bits: f64) -> String {
    // Bit rates are small integers.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let kbps = (bits / 1000.0).round().max(0.0) as u64;
    match kbps {
        0..=999 => kbps.to_string(),
        1000..=9999 => format!("{}H", kbps / 100),
        _ => format!("{}C", (kbps / 1000).min(99)),
    }
}

/// The title as it shows at `elapsed` since the file started.
fn marquee(text: &str, elapsed: Duration) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= MARQUEE_CHARS {
        return text.to_owned();
    }
    let mut loop_text = chars.clone();
    loop_text.extend("  ***  ".chars());
    let steps = elapsed.as_millis() / MARQUEE_STEP.as_millis();
    let offset = usize::try_from(steps % loop_text.len() as u128).unwrap_or(0);
    loop_text
        .iter()
        .cycle()
        .skip(offset)
        .take(MARQUEE_CHARS)
        .collect()
}

impl App {
    /// Fills the classic skins' texts, states and sliders: `elapsed` since
    /// the file started (the title's scrolling, the paused digits' blink).
    pub fn classic_data(&self, data: &mut DataModel, state: &mut UiState, elapsed: Duration) {
        let model = &self.model;
        let loaded = model.media_loaded();
        let paused = model.flag(PropertyId::Pause) == Some(true);
        let time = model.double(PropertyId::TimePos).unwrap_or(0.0);
        let duration = model.double(PropertyId::Duration);
        let remaining = self.settings.skins.reverse_times && duration.is_some();
        let shown = if remaining {
            (duration.unwrap_or(0.0) - time).max(0.0)
        } else {
            time
        };
        let blink_off = paused && (elapsed.as_millis() / BLINK.as_millis()) % 2 == 1;
        // Times of a file: far below u64's range.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let seconds = shown.floor() as u64;
        let (sign, minutes, secs) = if !loaded || blink_off {
            (String::new(), String::new(), String::new())
        } else {
            let minutes = seconds / 60;
            let sign = if remaining {
                "-".to_owned()
            } else if minutes >= 100 {
                (minutes / 100 % 10).to_string()
            } else {
                String::new()
            };
            (
                sign,
                format!("{:02}", minutes % 100),
                format!("{:02}", seconds % 60),
            )
        };
        data.set_text("RVP:time-sign", sign);
        data.set_text("RVP:time-minutes", minutes);
        data.set_text("RVP:time-seconds", secs);
        let lists = self.library().lists();
        let number = lists
            .playing
            .and_then(|album| lists.albums.get(album))
            .and_then(|album| album.current)
            .map_or(1, |current| current + 1);
        let title = model.text(PropertyId::MediaTitle).unwrap_or("");
        let text = if loaded {
            match duration {
                Some(total) => format!("{number}. {title} ({})", short_time(total)),
                None => format!("{number}. {title}"),
            }
        } else {
            String::new()
        };
        data.set_text("RVP:marquee", marquee(&text, elapsed));
        let rate = model
            .double(PropertyId::AudioBitrate)
            .filter(|bits| *bits > 0.0)
            .or_else(|| model.double(PropertyId::VideoBitrate));
        data.set_text(
            "RVP:kbps",
            if loaded {
                rate.map(kbps).unwrap_or_default()
            } else {
                String::new()
            },
        );
        let khz = model
            .int(PropertyId::AudioSampleRate)
            .filter(|_| loaded)
            .map(|rate| {
                (rate / 1000)
                    .to_string()
                    .chars()
                    .take(2)
                    .collect::<String>()
            })
            .unwrap_or_default();
        data.set_text("RVP:khz", khz);
        let short = if loaded {
            short_time(time)
        } else {
            String::new()
        };
        data.set_text("RVP:time-short", short.clone());
        let info = match (loaded, duration) {
            (true, Some(total)) => format!("{short}/{}", short_time(total)),
            _ => String::new(),
        };
        data.set_text("RVP:playlist-info", info);
        for (command, value) in [("RVP:preamp", 0.5), ("RVP:balance", 0.5)] {
            data.sliders.insert(
                command.to_owned(),
                SliderValue {
                    value,
                    enabled: true,
                },
            );
        }
        let channels = model.int(PropertyId::AudioChannels).filter(|_| loaded);
        set_states(state, loaded, paused, channels);
    }
}

/// The play state and the channels' lamps.
fn set_states(state: &mut UiState, loaded: bool, paused: bool, channels: Option<i64>) {
    let stereo = channels.is_some_and(|count| count >= 2);
    for (flag, on) in [
        ("RVP:playing", loaded && !paused),
        ("RVP:paused", loaded && paused),
        ("RVP:stopped", !loaded),
        ("RVP:stereo", stereo),
        ("RVP:no-stereo", !stereo),
        ("RVP:mono", channels == Some(1)),
        ("RVP:no-mono", channels != Some(1)),
    ] {
        if on {
            state.flags.insert(flag.to_owned());
        } else {
            state.flags.remove(flag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_shorten_as_winamps() {
        assert_eq!(kbps(128_000.0), "128");
        assert_eq!(kbps(1_411_200.0), "14H");
        assert_eq!(kbps(25_000_000.0), "25C");
    }

    #[test]
    fn long_titles_scroll_one_character_a_step() {
        let short = "1. Short (1:00)";
        assert_eq!(marquee(short, Duration::from_secs(9)), short);
        let long = "1. A title much longer than thirty-one characters (4:00)";
        let start = marquee(long, Duration::ZERO);
        assert_eq!(start.chars().count(), MARQUEE_CHARS);
        assert!(long.starts_with(&start));
        let next = marquee(long, MARQUEE_STEP);
        assert_eq!(
            next,
            long.chars().skip(1).take(MARQUEE_CHARS).collect::<String>()
        );
        // After the whole loop the start comes back.
        let loop_len = long.chars().count() + 7;
        let around = MARQUEE_STEP * u32::try_from(loop_len).unwrap();
        assert_eq!(marquee(long, around), start);
    }

    #[test]
    fn short_times() {
        assert_eq!(short_time(42.9), "0:42");
        assert_eq!(short_time(3725.0), "1:02:05");
    }
}
