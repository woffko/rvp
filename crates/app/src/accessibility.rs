//! M14: PotPlayer's Accessibility pages (goal P): TTS, screen readers,
//! the window title, touch and the Xbox controller.
//!
//! * RVP's notices, text subtitles and the skin's tooltips can be read
//!   aloud (Windows' speech, SAPI: the voice, its output, speed and
//!   volume), announced to screen readers (Qt's accessibility, UI
//!   Automation on Windows) and shown as the window's title.
//! * Touch: a swipe over the video seeks or changes the volume, and the
//!   skin can take its touch layout (`#if<SkinType==Touch>`).
//! * The Xbox controller (Windows' `XInput`) plays, seeks, changes the volume
//!   and the album's entry, and can move the mouse pointer.

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// What a swipe does: seek sideways and volume up and down, one of them,
/// or nothing.
pub const SWIPES: &[&str] = &["seek-volume", "seek", "volume", "none"];
/// The controller's mouse: none, or the left or right stick.
pub const MOUSE_STICKS: &[&str] = &["off", "left", "right"];

/// What one of the outputs takes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Readings {
    /// RVP's notices.
    pub messages: bool,
    /// Text subtitles.
    pub subtitles: bool,
    /// The skin's tooltips.
    pub tooltips: bool,
}

/// The Accessibility pages (`accessibility` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's pages.
#[allow(clippy::struct_excessive_bools)]
pub struct AccessibilitySettings {
    /// Read aloud.
    pub tts: Readings,
    /// The voice and the output (names Windows gives; empty: its defaults).
    pub voice: String,
    pub output: String,
    /// -10 to 10.
    pub rate: i8,
    /// 0 to 100.
    pub volume: u8,
    /// Announced to screen readers.
    pub screen_reader: Readings,
    /// Shown as the window's title.
    pub title: Readings,
    /// Swipes over the video.
    pub touch: bool,
    /// One of [`SWIPES`].
    pub swipe: String,
    /// The skin's touch layout.
    pub touch_skin: bool,
    /// The Xbox controller.
    pub xinput: bool,
    pub xinput_focus_only: bool,
    /// One of [`MOUSE_STICKS`].
    pub xinput_mouse: String,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            tts: Readings::default(),
            voice: String::new(),
            output: String::new(),
            rate: 0,
            volume: 100,
            screen_reader: Readings::default(),
            title: Readings::default(),
            touch: false,
            swipe: "seek-volume".to_owned(),
            touch_skin: false,
            xinput: false,
            xinput_focus_only: true,
            xinput_mouse: "off".to_owned(),
        }
    }
}

impl AccessibilitySettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !(-10..=10).contains(&self.rate) {
            notes.push(format!(
                "accessibility.rate {} is out of -10 to 10: 0",
                self.rate
            ));
            self.rate = 0;
        }
        if self.volume > 100 {
            notes.push(format!(
                "accessibility.volume {} is out of 0 to 100: 100",
                self.volume
            ));
            self.volume = 100;
        }
        for (name, value, known) in [
            ("swipe", &mut self.swipe, SWIPES),
            ("xinput-mouse", &mut self.xinput_mouse, MOUSE_STICKS),
        ] {
            if !known.contains(&value.as_str()) {
                notes.push(format!(
                    "accessibility.{name} {value:?} is none of {known:?}: {}",
                    known[0]
                ));
                known[0].clone_into(value);
            }
        }
    }

    /// The window's look of these options for QML (JSON).
    #[must_use]
    pub fn look(&self) -> serde_json::Value {
        serde_json::json!({
            "tts": self.tts,
            "screenReader": self.screen_reader,
            "title": self.title,
            "touch": self.touch,
            "swipe": self.swipe,
            "xinput": self.xinput,
            "xinputFocusOnly": self.xinput_focus_only,
            "xinputMouse": self.xinput_mouse,
        })
    }
}

/// What a swipe of `dx`, `dy` (a share of the video's width and height)
/// does as `swipe` says: seconds to seek, or volume steps.
#[must_use]
pub fn swipe_effect(swipe: &str, dx: f64, dy: f64) -> Option<SwipeEffect> {
    let sideways = dx.abs() >= dy.abs();
    match (swipe, sideways) {
        ("seek-volume" | "seek", true) if dx.abs() > 0.05 => {
            // A whole width seeks two minutes, in whole seconds.
            Some(SwipeEffect::Seek((dx * 120.0).round()))
        }
        ("seek-volume" | "volume", false) if dy.abs() > 0.05 => {
            // Up is louder; a whole height is twenty steps.
            Some(SwipeEffect::Volume((-dy * 20.0).round()))
        }
        _ => None,
    }
}

/// What a swipe does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwipeEffect {
    Seek(f64),
    Volume(f64),
}

impl App {
    /// A swipe's seek: `seconds` back or forward, with the time shown.
    pub fn seek_by(&mut self, engine: &rvp_engine::Engine, seconds: f64) -> Option<UiRequest> {
        self.seek_to(engine, rvp_engine::Seek::Relative(seconds))
            .ok()?;
        let sign = if seconds < 0.0 { "-" } else { "+" };
        Some(UiRequest::Osd(format!("{sign}{} s", seconds.abs())))
    }

    /// A swipe's volume: `steps` of the volume step up (or down).
    pub fn volume_steps(&mut self, engine: &rvp_engine::Engine, steps: f64) -> Option<UiRequest> {
        let delta = self.volume_step_now() * steps;
        self.change_volume(engine, delta)
    }

    /// Misc > Enable touch feature.
    pub(crate) fn toggle_touch(&mut self) -> UiRequest {
        let accessibility = &mut self.settings.accessibility;
        accessibility.touch = !accessibility.touch;
        let text = format!("Touch: {}", if accessibility.touch { "on" } else { "off" });
        self.save_settings();
        UiRequest::Osd(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swipes_seek_or_change_the_volume() {
        assert_eq!(
            swipe_effect("seek-volume", 0.25, 0.02),
            Some(SwipeEffect::Seek(30.0))
        );
        assert_eq!(
            swipe_effect("seek-volume", 0.01, -0.5),
            Some(SwipeEffect::Volume(10.0))
        );
        assert_eq!(swipe_effect("seek", 0.01, -0.5), None);
        assert_eq!(swipe_effect("volume", 0.5, 0.0), None);
        assert_eq!(swipe_effect("seek-volume", 0.01, 0.01), None);
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = AccessibilitySettings {
            rate: 20,
            swipe: "pinch".to_owned(),
            ..AccessibilitySettings::default()
        };
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(settings.rate, 0);
        assert_eq!(settings.swipe, "seek-volume");
    }
}
