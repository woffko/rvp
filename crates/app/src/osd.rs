//! M14: PotPlayer's OSD Messages page and its on-screen info (goal P).
//!
//! * RVP's notices ("general info messages") can be turned off, placed by
//!   an alignment and two margins, and sized (fixed, or following the
//!   video's height); they show over the video in RVP's window, or in the
//!   picture through mpv's OSD (`output`), as they always do over the
//!   Direct3D 11 renderer's window.
//! * Misc info (Scroll Lock): a line in the top right corner of the video
//!   with the chosen parts: the time, its percent, the frame, the clock,
//!   the playlist position, the file, the chapter and the battery; shown
//!   (and toggled) only in fullscreen when so set.
//! * Short playback info (Shift+Tab): a notice with the file, the
//!   subtitle, the time, the A-B section and the codecs.
//!
//! The info texts use the second font (`info_font`, also the Tab info's).

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::view::format_time;
use rvp_engine::PropertyId;

/// Where notices show: over the video in RVP's window, or in the picture
/// (mpv's OSD).
pub const OUTPUTS: &[&str] = &["window", "video"];
pub const HORIZONTAL: &[&str] = &["left", "center", "right"];
pub const VERTICAL: &[&str] = &["top", "center", "bottom"];
/// The misc info's time: none, elapsed, remaining, or both.
pub const MISC_TIMES: &[&str] = &["none", "elapsed", "remaining", "both"];
/// The misc info's clock: none, hours and minutes, or with seconds.
pub const MISC_CLOCKS: &[&str] = &["none", "minutes", "seconds"];

/// The OSD Messages page (`osd` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's page.
#[allow(clippy::struct_excessive_bools)]
pub struct OsdSettings {
    /// Show general info messages.
    pub messages: bool,
    /// One of [`OUTPUTS`].
    pub output: String,
    /// One of [`HORIZONTAL`] and of [`VERTICAL`].
    pub h_align: String,
    pub v_align: String,
    /// Pixels from the top (or bottom) and the right (or left) edge of the
    /// video.
    pub top_margin: u16,
    pub right_margin: u16,
    /// The size follows the video's height (`font_size` at 720 lines).
    pub auto_size: bool,
    /// The notices' font (empty: Qt's) and size in pixels.
    pub font: String,
    pub font_size: u16,
    /// The info texts' font and size.
    pub info_font: String,
    pub info_font_size: u16,
    pub misc: MiscInfo,
    pub short: ShortInfo,
}

impl Default for OsdSettings {
    fn default() -> Self {
        Self {
            messages: true,
            output: "window".to_owned(),
            h_align: "left".to_owned(),
            v_align: "top".to_owned(),
            top_margin: 12,
            right_margin: 16,
            auto_size: false,
            font: String::new(),
            font_size: 20,
            info_font: String::new(),
            info_font_size: 14,
            misc: MiscInfo::default(),
            short: ShortInfo::default(),
        }
    }
}

/// Content of Misc Info (Scroll Lock).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[allow(clippy::struct_excessive_bools)]
pub struct MiscInfo {
    /// Display misc info (Scroll Lock toggles it).
    pub show: bool,
    /// Scroll Lock works only in fullscreen.
    pub key_fullscreen_only: bool,
    /// One of [`MISC_TIMES`].
    pub time: String,
    pub percent: bool,
    /// The number of elapsed frames.
    pub frames: bool,
    /// One of [`MISC_CLOCKS`].
    pub clock: String,
    /// The line shows only in fullscreen.
    pub fullscreen_only: bool,
    /// The playlist sequence number.
    pub position: bool,
    pub file: bool,
    /// The chapter.
    pub chapter: bool,
    /// The system's power (the battery).
    pub power: bool,
}

impl Default for MiscInfo {
    fn default() -> Self {
        Self {
            show: false,
            key_fullscreen_only: false,
            time: "both".to_owned(),
            percent: true,
            frames: false,
            clock: "minutes".to_owned(),
            fullscreen_only: false,
            position: true,
            file: false,
            chapter: true,
            power: false,
        }
    }
}

/// Content of Short Playback Info (Shift+Tab).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[allow(clippy::struct_excessive_bools)]
pub struct ShortInfo {
    pub file: bool,
    pub subtitle: bool,
    pub time: bool,
    /// The marked (A-B) section.
    pub section: bool,
    pub codec: bool,
}

impl Default for ShortInfo {
    fn default() -> Self {
        Self {
            file: true,
            subtitle: true,
            time: true,
            section: true,
            codec: true,
        }
    }
}

impl OsdSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let mut choose = |name: &str, value: &mut String, known: &[&str]| {
            if !known.contains(&value.as_str()) {
                notes.push(format!(
                    "osd.{name} {value:?} is none of {known:?}: {}",
                    known[0]
                ));
                known[0].clone_into(value);
            }
        };
        choose("output", &mut self.output, OUTPUTS);
        choose("h-align", &mut self.h_align, HORIZONTAL);
        choose("v-align", &mut self.v_align, VERTICAL);
        choose("misc.time", &mut self.misc.time, MISC_TIMES);
        choose("misc.clock", &mut self.misc.clock, MISC_CLOCKS);
        for (name, value, default) in [
            ("font-size", &mut self.font_size, 20),
            ("info-font-size", &mut self.info_font_size, 14),
        ] {
            if !(8..=96).contains(value) {
                notes.push(format!("osd.{name} {value} is out of 8 to 96: {default}"));
                *value = default;
            }
        }
        for (name, value) in [
            ("top-margin", &mut self.top_margin),
            ("right-margin", &mut self.right_margin),
        ] {
            if *value > 500 {
                notes.push(format!("osd.{name} {value} is out of 0 to 500: 500"));
                *value = 500;
            }
        }
    }
}

/// The parts of the misc info line, as the window knows them now.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MiscParts {
    pub time: f64,
    pub duration: Option<f64>,
    pub fps: Option<f64>,
    /// The local time (`HH:MM:SS`).
    pub clock: String,
    /// The playlist position and count.
    pub position: Option<(usize, usize)>,
    pub file: String,
    /// The chapter (from 0) and the count.
    pub chapter: Option<(i64, i64)>,
    /// The battery's text, when the system has one.
    pub power: Option<String>,
}

/// The misc info line of `parts` as `misc` chooses.
#[must_use]
pub fn misc_line(misc: &MiscInfo, parts: &MiscParts) -> String {
    let mut out = Vec::new();
    let remaining = parts
        .duration
        .map(|duration| (duration - parts.time).max(0.0));
    match (misc.time.as_str(), remaining) {
        ("remaining", Some(left)) => out.push(format!("-{}", format_time(left))),
        ("both", Some(left)) => out.push(format!(
            "{} / -{}",
            format_time(parts.time),
            format_time(left)
        )),
        // Without a duration, both is the elapsed time.
        ("elapsed" | "both", _) => out.push(format_time(parts.time)),
        _ => {}
    }
    if misc.percent
        && let Some(duration) = parts.duration.filter(|duration| *duration > 0.0)
    {
        out.push(format!(
            "{:.1}%",
            (parts.time / duration * 100.0).clamp(0.0, 100.0)
        ));
    }
    if misc.frames
        && let Some(fps) = parts.fps.filter(|fps| *fps > 0.0)
    {
        // Frame numbers are far inside u64.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame = (parts.time * fps).floor().max(0.0) as u64;
        out.push(format!("frame {frame}"));
    }
    match misc.clock.as_str() {
        "minutes" => out.push(parts.clock.chars().take(5).collect()),
        "seconds" => out.push(parts.clock.clone()),
        _ => {}
    }
    if misc.position
        && let Some((index, count)) = parts.position
    {
        out.push(format!("[{}/{count}]", index + 1));
    }
    if misc.file && !parts.file.is_empty() {
        out.push(parts.file.clone());
    }
    if misc.chapter
        && let Some((chapter, count)) = parts.chapter.filter(|(_, count)| *count > 0)
    {
        out.push(format!("chapter {}/{count}", chapter + 1));
    }
    if misc.power
        && let Some(power) = &parts.power
    {
        out.push(power.clone());
    }
    out.retain(|part| !part.is_empty());
    out.join("  ")
}

/// A command of the info items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OsdCommand {
    /// Display Misc Info on Screen (Scroll Lock).
    MiscInfo,
    /// Show Short Playback Info on Screen (Shift+Tab).
    ShortInfo,
}

impl App {
    /// The info items' commands.
    pub(crate) fn osd_command(
        &mut self,
        engine: &rvp_engine::Engine,
        command: OsdCommand,
    ) -> Option<UiRequest> {
        match command {
            OsdCommand::MiscInfo => {
                let fullscreen = self.picture.fullscreen;
                let misc = &mut self.settings.osd.misc;
                if misc.key_fullscreen_only && !fullscreen {
                    return None;
                }
                misc.show = !misc.show;
                let text = format!("Misc info: {}", if misc.show { "on" } else { "off" });
                self.save_settings();
                Some(UiRequest::Osd(text))
            }
            OsdCommand::ShortInfo => Some(UiRequest::Info(self.short_info(engine))),
        }
    }

    /// The misc info line for the window now (empty when it does not
    /// show).
    #[must_use]
    pub fn misc_info(&self, fullscreen: bool, clock: &str, power: Option<String>) -> String {
        let misc = &self.settings.osd.misc;
        if !misc.show || (misc.fullscreen_only && !fullscreen) || !self.model.media_loaded() {
            return String::new();
        }
        let lists = self.library().lists();
        let parts = MiscParts {
            time: self.model.double(PropertyId::TimePos).unwrap_or(0.0),
            duration: self.model.double(PropertyId::Duration),
            fps: self.model.double(PropertyId::ContainerFps),
            clock: clock.to_owned(),
            position: lists.playing.and_then(|album| {
                let album = lists.albums.get(album)?;
                Some((album.current?, album.entries.len()))
            }),
            file: self
                .model
                .loaded_path()
                .map(|path| crate::view::file_name(path).to_owned())
                .unwrap_or_default(),
            chapter: self.model.int(PropertyId::Chapter).and_then(|chapter| {
                self.model
                    .int(PropertyId::Chapters)
                    .map(|count| (chapter, count))
            }),
            power,
        };
        misc_line(misc, &parts)
    }

    /// The short playback info (Shift+Tab): its parts, a line each.
    fn short_info(&self, engine: &rvp_engine::Engine) -> String {
        let short = &self.settings.osd.short;
        let model = &self.model;
        let Some(path) = model.loaded_path() else {
            return "No file".to_owned();
        };
        let mut lines = Vec::new();
        if short.file {
            lines.push(crate::view::file_name(path).to_owned());
        }
        if short.subtitle {
            let subtitle = engine
                .tracks()
                .into_iter()
                .find(|track| track.kind == rvp_engine::TrackKind::Subtitle && track.selected)
                .map_or_else(
                    || "Subtitles: none".to_owned(),
                    |track| format!("Subtitles: {}", crate::commands::track_label(&track)),
                );
            lines.push(subtitle);
        }
        if short.time {
            lines.push(format!(
                "{} / {}",
                format_time(model.double(PropertyId::TimePos).unwrap_or(0.0)),
                format_time(model.double(PropertyId::Duration).unwrap_or(0.0))
            ));
        }
        if short.section
            && let (Some(a), Some(b)) = (
                model.double(PropertyId::AbLoopA),
                model.double(PropertyId::AbLoopB),
            )
        {
            lines.push(format!("A-B: {} - {}", format_time(a), format_time(b)));
        }
        if short.codec {
            let mut codecs = Vec::new();
            if let Some(video) = model.text(PropertyId::VideoFormat) {
                let size = match (
                    model.int(PropertyId::VideoWidth),
                    model.int(PropertyId::VideoHeight),
                ) {
                    (Some(width), Some(height)) => format!(" {width}x{height}"),
                    _ => String::new(),
                };
                codecs.push(format!("{video}{size}"));
            }
            if let Some(audio) = model.text(PropertyId::AudioCodecName) {
                codecs.push(audio.to_owned());
            }
            if !codecs.is_empty() {
                lines.push(codecs.join(", "));
            }
        }
        lines.join("\n")
    }

    /// The Tab info's, misc info's and notices' look for `Main.qml`
    /// (JSON).
    #[must_use]
    pub fn osd_look(&self) -> serde_json::Value {
        let osd = &self.settings.osd;
        serde_json::json!({
            "messages": osd.messages,
            "video": osd.output == "video",
            "h": osd.h_align,
            "v": osd.v_align,
            "top": osd.top_margin,
            "right": osd.right_margin,
            "auto": osd.auto_size,
            "font": osd.font,
            "size": osd.font_size,
            "infoFont": osd.info_font,
            "infoSize": osd.info_font_size,
        })
    }
}

/// The battery from Linux's `/sys/class/power_supply` (`root`): its
/// charge and whether it charges; `None` without a battery.
#[must_use]
pub fn battery_from_sysfs(root: &std::path::Path) -> Option<String> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut supplies: Vec<std::path::PathBuf> =
        entries.flatten().map(|entry| entry.path()).collect();
    supplies.sort();
    let read = |path: &std::path::Path, name: &str| {
        std::fs::read_to_string(path.join(name))
            .ok()
            .map(|text| text.trim().to_owned())
    };
    let battery = supplies
        .iter()
        .find(|path| read(path, "type").as_deref() == Some("Battery"))?;
    let capacity = read(battery, "capacity")?;
    let charging = read(battery, "status").is_some_and(|status| status == "Charging");
    Some(format!(
        "battery {capacity}%{}",
        if charging { " (charging)" } else { "" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_misc_line_holds_the_chosen_parts() {
        let parts = MiscParts {
            time: 65.0,
            duration: Some(130.0),
            fps: Some(25.0),
            clock: "21:05:09".to_owned(),
            position: Some((1, 3)),
            file: "a.mkv".to_owned(),
            chapter: Some((0, 4)),
            power: Some("battery 80%".to_owned()),
        };
        let mut misc = MiscInfo::default();
        assert_eq!(
            misc_line(&misc, &parts),
            "00:01:05 / -00:01:05  50.0%  21:05  [2/3]  chapter 1/4"
        );
        misc.time = "remaining".to_owned();
        misc.percent = false;
        misc.frames = true;
        misc.clock = "seconds".to_owned();
        misc.position = false;
        misc.file = true;
        misc.chapter = false;
        misc.power = true;
        assert_eq!(
            misc_line(&misc, &parts),
            "-00:01:05  frame 1625  21:05:09  a.mkv  battery 80%"
        );
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = OsdSettings {
            output: "screen".to_owned(),
            font_size: 2,
            ..OsdSettings::default()
        };
        settings.misc.clock = "never".to_owned();
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert_eq!(settings.output, "window");
        assert_eq!(settings.font_size, 20);
        assert_eq!(settings.misc.clock, "none");
    }

    #[test]
    fn the_battery_comes_from_sysfs() {
        let root = std::env::temp_dir().join(format!("rvp-power-{}", std::process::id()));
        let battery = root.join("BAT0");
        let mains = root.join("AC");
        std::fs::create_dir_all(&battery).unwrap();
        std::fs::create_dir_all(&mains).unwrap();
        std::fs::write(mains.join("type"), "Mains\n").unwrap();
        assert_eq!(battery_from_sysfs(&root), None);
        std::fs::write(battery.join("type"), "Battery\n").unwrap();
        std::fs::write(battery.join("capacity"), "57\n").unwrap();
        std::fs::write(battery.join("status"), "Charging\n").unwrap();
        assert_eq!(
            battery_from_sysfs(&root).as_deref(),
            Some("battery 57% (charging)")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
