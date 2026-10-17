//! The scene browsers (M11, PotPlayer's Browse Scenes): pictures of the
//! file that plays from the thumbnail core, one a place, and a click jumps
//! there.
//!
//! * By time intervals (Shift+Z): every interval of the Scene Browser page,
//!   or with "Auto" (0) the duration in about 24 parts (whole seconds).
//! * By markers (Alt+Z): the chapters and bookmarks, in time order; with
//!   none, the time-based browser when "Prompt for time-based one" is on.
//! * By subtitle intervals: the lines of the selected subtitle when it is
//!   an external text file (`SubRip`, `WebVTT`, ASS/SSA, read by RVP: mpv gives
//!   no list of a track's lines), at most [`MAX_SCENES`].

use std::path::Path;

use rvp_engine::{Engine, PropertyId};
use serde::{Deserialize, Serialize};

use crate::action::Scenes;
use crate::app::{App, UiRequest};

/// Scenes a browser shows at most.
pub const MAX_SCENES: usize = 400;

/// The Scene Browser page's settings (`scenes` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SceneSettings {
    /// The picture's width in pixels (its height follows 16:9).
    pub size: u32,
    /// Seconds between time-based scenes; 0 is "Auto".
    pub interval: f64,
    /// The browser's opacity in percent.
    pub opacity: u32,
    /// The browser closes when a scene is chosen.
    pub close_on_change: bool,
    /// Without markers or subtitle lines, the time-based browser opens.
    pub fall_back_to_time: bool,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            size: 192,
            interval: 0.0,
            opacity: 100,
            close_on_change: true,
            fall_back_to_time: true,
        }
    }
}

impl SceneSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !(64..=480).contains(&self.size) {
            let size = self.size.clamp(64, 480);
            notes.push(format!("scenes.size must be 64 to 480: {size} used"));
            self.size = size;
        }
        if !self.interval.is_finite() || !(0.0..=3600.0).contains(&self.interval) {
            notes.push("scenes.interval must be 0 (auto) to 3600: auto used".to_owned());
            self.interval = 0.0;
        }
        if !(20..=100).contains(&self.opacity) {
            let opacity = self.opacity.clamp(20, 100);
            notes.push(format!("scenes.opacity must be 20 to 100: {opacity} used"));
            self.opacity = opacity;
        }
    }

    /// The pictures' box.
    #[must_use]
    pub fn picture_size(&self) -> (u32, u32) {
        (self.size, self.size * 9 / 16)
    }
}

/// One place of a browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub time: f64,
    pub label: String,
}

/// A browser for the window: its pictures come as
/// [`crate::thumbs::ThumbEvent::Scene`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Browser {
    pub title: String,
    pub scenes: Vec<Scene>,
    pub width: u32,
    pub height: u32,
    pub opacity: u32,
    pub close_on_change: bool,
}

/// Places every `interval` seconds over `duration` (or about 24 with 0).
#[must_use]
pub fn time_scenes(duration: f64, interval: f64) -> Vec<f64> {
    if !(duration.is_finite() && duration > 0.0) {
        return Vec::new();
    }
    let interval = if interval > 0.0 {
        interval
    } else {
        (duration / 24.0).round().max(1.0)
    };
    let mut times = Vec::new();
    let mut time = 0.0;
    // A scene in the last half second would show the end.
    while time < duration - 0.5 && times.len() < MAX_SCENES {
        times.push(time);
        time += interval;
    }
    times
}

/// `00:01:02.500` (`SubRip` uses a comma), or `0:01:02.50` (ASS).
fn subtitle_time(text: &str) -> Option<f64> {
    let text = text.trim().replace(',', ".");
    let parts: Vec<&str> = text.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [hours, minutes, seconds] => (
            hours.parse::<f64>().ok()?,
            minutes.parse::<f64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        [minutes, seconds] => (
            0.0,
            minutes.parse::<f64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        _ => return None,
    };
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// The lines of a text subtitle file: their starts and texts, in time
/// order (`SubRip`, `WebVTT`, ASS and SSA).
#[must_use]
pub fn subtitle_lines(text: &str) -> Vec<(f64, String)> {
    let mut lines = Vec::new();
    let mut rows = text.lines().peekable();
    while let Some(row) = rows.next() {
        let row = row.trim_start_matches('\u{feff}').trim();
        if let Some(rest) = row.strip_prefix("Dialogue:") {
            // Layer, Start, End, Style, Name, MarginL, MarginR, MarginV,
            // Effect, Text.
            let fields: Vec<&str> = rest.splitn(10, ',').collect();
            if let (Some(start), Some(text)) =
                (fields.get(1).and_then(|t| subtitle_time(t)), fields.get(9))
            {
                lines.push((start, clean_text(text)));
            }
            continue;
        }
        if let Some((start, _)) = row.split_once("-->")
            && let Some(start) = subtitle_time(start)
        {
            let mut text = Vec::new();
            while let Some(next) = rows.peek() {
                if next.trim().is_empty() {
                    break;
                }
                text.push(next.trim().to_owned());
                rows.next();
            }
            lines.push((start, clean_text(&text.join(" "))));
        }
    }
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    lines.dedup_by(|a, b| (a.0 - b.0).abs() < 0.001);
    lines
}

/// Subtitle text without ASS overrides (`{\i1}`), line breaks (`\N`) and
/// HTML tags.
fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0_u32;
    for c in text.replace("\\N", " ").replace("\\n", " ").chars() {
        match c {
            '{' | '<' => depth += 1,
            '}' | '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl App {
    /// Opens a scene browser of the file that plays.
    pub(crate) fn open_scenes(&mut self, engine: &Engine, kind: Scenes) -> Option<UiRequest> {
        let path = self.model.loaded_path()?.to_owned();
        if self.model.int(PropertyId::VideoWidth).is_none() || crate::playlist::is_url(&path) {
            return Some(UiRequest::Osd(
                "Scene browsers need a video file".to_owned(),
            ));
        }
        let duration = self.model.double(PropertyId::Duration).unwrap_or(0.0);
        let settings = self.settings.scenes.clone();
        let fall_back = |title: &str, why: Option<&str>| -> (String, Vec<Scene>) {
            if why.is_none() || settings.fall_back_to_time {
                (
                    match why {
                        Some(why) => format!("Scenes by time intervals ({why})"),
                        None => title.to_owned(),
                    },
                    time_scenes(duration, settings.interval)
                        .into_iter()
                        .map(|time| Scene {
                            time,
                            label: crate::view::format_time(time),
                        })
                        .collect(),
                )
            } else {
                (title.to_owned(), Vec::new())
            }
        };
        let (title, scenes) = match kind {
            Scenes::Time => fall_back("Scenes by time intervals", None),
            Scenes::Markers => {
                let mut scenes: Vec<Scene> = self
                    .play
                    .chapters
                    .iter()
                    .enumerate()
                    .map(|(index, chapter)| Scene {
                        time: chapter.time,
                        label: match &chapter.title {
                            Some(title) if !title.is_empty() => title.clone(),
                            _ => format!("Chapter {}", index + 1),
                        },
                    })
                    .collect();
                scenes.extend(
                    self.marks()
                        .file(&path)
                        .bookmarks
                        .into_iter()
                        .map(|bookmark| Scene {
                            time: bookmark.time,
                            label: bookmark.title,
                        }),
                );
                scenes.sort_by(|a, b| a.time.total_cmp(&b.time));
                if scenes.is_empty() {
                    fall_back("Scenes by markers", Some("no markers"))
                } else {
                    ("Scenes by markers".to_owned(), scenes)
                }
            }
            Scenes::Subtitles => {
                let lines = Self::external_subtitle_lines(engine);
                if lines.is_empty() {
                    fall_back("Scenes by subtitle intervals", Some("no subtitle file"))
                } else {
                    (
                        "Scenes by subtitle intervals".to_owned(),
                        lines
                            .into_iter()
                            .take(MAX_SCENES)
                            .map(|(time, label)| Scene { time, label })
                            .collect(),
                    )
                }
            }
        };
        if scenes.is_empty() {
            return Some(UiRequest::Osd(format!("{title}: nothing to show")));
        }
        let (width, height) = settings.picture_size();
        let times: Vec<(usize, f64)> = scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| (index, scene.time))
            .collect();
        self.request_scenes(&path, &times, (width, height));
        Some(UiRequest::SceneBrowser(Browser {
            title,
            scenes,
            width,
            height,
            opacity: settings.opacity,
            close_on_change: settings.close_on_change,
        }))
    }

    /// The lines of the selected subtitle when it is an external text
    /// file.
    fn external_subtitle_lines(engine: &Engine) -> Vec<(f64, String)> {
        let Some(list) = engine.get_json("track-list") else {
            return Vec::new();
        };
        let file = list.as_array().into_iter().flatten().find_map(|track| {
            (track["type"] == "sub" && track["selected"] == true)
                .then(|| track["external-filename"].as_str().map(str::to_owned))
                .flatten()
        });
        let Some(file) = file else {
            return Vec::new();
        };
        match std::fs::read(Path::new(&file)) {
            Ok(bytes) => subtitle_lines(&String::from_utf8_lossy(&bytes)),
            Err(_) => Vec::new(),
        }
    }

    /// A scene was chosen: playback jumps there.
    pub fn scene_chosen(&mut self, engine: &Engine, time: f64) {
        drop(self.seek_to(engine, rvp_engine::Seek::Absolute(time)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_scenes_split_the_duration() {
        assert_eq!(time_scenes(60.0, 20.0), [0.0, 20.0, 40.0]);
        let auto = time_scenes(600.0, 0.0);
        assert_eq!(auto.len(), 24);
        assert!((auto[1] - 25.0).abs() < 1e-9);
        assert!(time_scenes(0.0, 10.0).is_empty());
        assert_eq!(time_scenes(1e9, 1.0).len(), MAX_SCENES);
    }

    #[test]
    fn subtitle_files_give_their_lines() {
        let srt = "1\n00:00:01,500 --> 00:00:03,000\n<i>Hello</i>\nthere\n\n2\n00:01:00,000 --> 00:01:02,000\nAgain\n";
        assert_eq!(
            subtitle_lines(srt),
            [(1.5, "Hello there".to_owned()), (60.0, "Again".to_owned())]
        );
        let vtt = "WEBVTT\n\n00:02.000 --> 00:04.000\nShort form\n";
        assert_eq!(subtitle_lines(vtt), [(2.0, "Short form".to_owned())]);
        let ass = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:05.25,0:00:06.00,Default,,0,0,0,,{\\i1}One,\\Ntwo\n";
        assert_eq!(subtitle_lines(ass), [(5.25, "One, two".to_owned())]);
    }
}
