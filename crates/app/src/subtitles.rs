//! M13: PotPlayer's subtitle options over mpv's (goal O).
//!
//! * PotPlayer's Subtitles page: showing them at start, how far RVP's style
//!   reaches into ASS scripts (`sub-ass-override` and, for the font, the
//!   colour and the letters' scales, `sub-ass-style-overrides`), fonts from RVP's `fonts` folder
//!   (`sub-fonts-dir`), reloading changed external files, the placement of
//!   bitmap subtitles (on the screen or the video: `stretch-image-subs-to-
//!   screen`) and text subtitles (inside the video, `sub-use-margins` off;
//!   forced under it, `sub-ass-force-margins` with `sub-pos` 100 and M12's
//!   bottom margins), the aspect correction of scripts made for anamorphic
//!   video (`sub-ass-vsfilter-aspect-compat`), and removing hearing-impaired
//!   notes and empty events (`sub-filter-sdh`, `sub-filter-regex`).
//! * The Font Style page and the Subtitle Style menu: bold, italic, the
//!   colours with their opacity (`#AARRGGBB`), the outline, shadow and blur,
//!   letter and line spacing, the letters' width and height, the outline or a box behind the text
//!   (`sub-border-style`), the horizontal alignment, the vertical position
//!   and the margins. A number or colour left unset is libmpv's own default:
//!   they differ between Linux's libmpv 0.37 and Windows' (newer), and RVP
//!   leaves each libmpv's look alone until the user changes it.
//! * The Language/Sync/Other page: preferred languages only (`subs-fallback`),
//!   a second subtitle chosen by its own languages (`secondary-sid`, which RVP
//!   picks: mpv has no `secondary-slang`), where it shows
//!   (`secondary-sub-pos`), remembering the chosen tracks per file, the
//!   three sync steps and keeping a sync change for the next files.
//! * The Fade/3D/Location page: the extra folders to look in for subtitle
//!   files (`sub-file-paths`), and the text encoding of files that are not
//!   UTF-8 (`sub-codepage`, which RVP's own reader follows).
//!
//! RVP sets these after libmpv starts (`App::apply_subtitles`), never as
//! start options: libmpv refuses to start with an option it lacks, and
//! Linux's 0.37 lacks `sub-border-style`, `sub-line-spacing` and
//! `secondary-sub-pos` and names the outline `border`. Each option is tried
//! by its current name, then by its older one; one that neither knows is
//! skipped.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rvp_engine::{Engine, PropertyId};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::settings::Settings;

/// `sub-ass-override` values, in the settings' order: PotPlayer's "ASS/SSA
/// subtitle style" from the script's own style to plain text.
pub const ASS_OVERRIDE: &[&str] = &["no", "scale", "yes", "force", "strip"];

/// `sub-border-style` values: an outline (with its shadow), the box of the
/// outline's colour around each line, or one box behind all lines.
pub const BORDER_STYLE: &[&str] = &["outline-and-shadow", "opaque-box", "background-box"];

/// `sub-align-x` values.
pub const ALIGN: &[&str] = &["left", "center", "right"];

/// The frame rates of PotPlayer's Convert Subtitle FPS items.
pub const FPS_CHOICES: &[f64] = &[12.0, 15.0, 23.976, 24.0, 25.0, 29.97, 30.0];

/// The text encodings of subtitle files without a BOM that are not UTF-8
/// (`sub-codepage`; RVP's own reader takes the same names); `auto`: mpv
/// and RVP guess.
pub const ENCODINGS: &[&str] = &[
    "auto",
    "utf-8",
    "windows-1250",
    "windows-1251",
    "windows-1252",
    "windows-1253",
    "windows-1254",
    "windows-1255",
    "windows-1256",
    "windows-1257",
    "windows-1258",
    "iso-8859-2",
    "iso-8859-5",
    "iso-8859-7",
    "koi8-r",
    "koi8-u",
    "shift_jis",
    "euc-jp",
    "euc-kr",
    "gb18030",
    "big5",
];

/// PotPlayer's subtitle options (`subtitles` in `settings.json`; the older
/// `subtitle-*` keys keep the languages, loading, size, font and first sync
/// step).
// Independent options as PotPlayer's pages list them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SubtitleSettings {
    /// Subtitles show when a file starts (`sub-visibility`).
    pub show: bool,
    /// How far RVP's style reaches into ASS scripts (one of
    /// [`ASS_OVERRIDE`]).
    pub ass_override: String,
    /// ASS scripts take RVP's font (`FontName`) and text colour
    /// (`PrimaryColour`) as well.
    pub override_font: bool,
    pub override_color: bool,
    /// Fonts from the `fonts` folder beside the settings (`sub-fonts-dir`).
    pub fonts_folder: bool,
    /// External subtitle files reload when they change on disk.
    pub auto_reload: bool,
    /// Bitmap subtitles placed on the whole screen, black bars included,
    /// rather than on the video (`stretch-image-subs-to-screen`).
    pub bitmap_outside: bool,
    /// Text subtitles kept inside the video (`sub-use-margins` off).
    pub text_inside: bool,
    /// Text subtitles forced under the video, in the black bars or M12's
    /// bottom margin (`sub-ass-force-margins`, `sub-pos` 100).
    pub under_video: bool,
    /// ASS scripts made for anamorphic video keep their shapes
    /// (`sub-ass-vsfilter-aspect-compat`).
    pub fix_aspect: bool,
    /// Notes for the hearing impaired (`[...]`, `(...)`) removed
    /// (`sub-filter-sdh`).
    pub remove_sdh: bool,
    /// Events with nothing but spaces removed (`sub-filter-regex`).
    pub remove_blank: bool,
    pub bold: bool,
    pub italic: bool,
    /// Colours as `#AARRGGBB` (mpv's notation; the alpha is the opacity);
    /// unset: libmpv's default.
    pub color: Option<String>,
    pub outline_color: Option<String>,
    /// The shadow's colour, which is also the box's.
    pub back_color: Option<String>,
    pub outline_size: Option<f64>,
    pub shadow_offset: Option<f64>,
    pub blur: Option<f64>,
    /// Letter spacing (`sub-spacing`) and extra line spacing
    /// (`sub-line-spacing`).
    pub spacing: Option<f64>,
    pub line_spacing: Option<f64>,
    /// The letters' width and height in percent (ASS's `ScaleX` and
    /// `ScaleY`, through `sub-ass-style-overrides`; unset: 100).
    pub scale_x: Option<f64>,
    pub scale_y: Option<f64>,
    /// One of [`BORDER_STYLE`].
    pub border_style: String,
    /// One of [`ALIGN`].
    pub align: String,
    /// The vertical position in percent of the video's height
    /// (`sub-pos`, 100: the bottom).
    pub position: Option<f64>,
    /// The margins at the sides and at the bottom or top
    /// (`sub-margin-x`, `sub-margin-y`).
    pub margin_x: Option<i64>,
    pub margin_y: Option<i64>,
    /// No subtitle is chosen when none has a preferred language
    /// (`subs-fallback=no`).
    pub only_preferred: bool,
    /// A second subtitle shows with the first when a track of these
    /// languages (comma separated) differs from it.
    pub second_auto: bool,
    pub second_languages: String,
    /// Its vertical position (`secondary-sub-pos`); unset: libmpv's (at the
    /// top).
    pub second_position: Option<f64>,
    /// The chosen subtitle tracks come back when a file plays again.
    pub remember_tracks: bool,
    /// The second and third sync steps (Ctrl and Alt with < and >).
    pub sync_step_2: f64,
    pub sync_step_3: f64,
    /// A sync change stays for the next files.
    pub keep_sync: bool,
    /// Extra folders to look in for subtitle files (`sub-file-paths`).
    pub folders: Vec<String>,
    /// One of [`ENCODINGS`].
    pub encoding: String,
    /// Sync steps move only the subtitles after the playback position
    /// (PotPlayer's "Apply sync changes starting from next position";
    /// subtitle files RVP edits only, `crate::subtitle_editor`).
    pub sync_from_next: bool,
    /// The subtitle browser's settings.
    pub browser: crate::subtitle_editor::BrowserSettings,
}

impl Default for SubtitleSettings {
    fn default() -> Self {
        Self {
            show: true,
            ass_override: "scale".to_owned(),
            override_font: false,
            override_color: false,
            fonts_folder: false,
            auto_reload: false,
            bitmap_outside: false,
            text_inside: false,
            under_video: false,
            fix_aspect: true,
            remove_sdh: false,
            remove_blank: false,
            bold: false,
            italic: false,
            color: None,
            outline_color: None,
            back_color: None,
            outline_size: None,
            shadow_offset: None,
            blur: None,
            spacing: None,
            line_spacing: None,
            scale_x: None,
            scale_y: None,
            border_style: "outline-and-shadow".to_owned(),
            align: "center".to_owned(),
            position: None,
            margin_x: None,
            margin_y: None,
            only_preferred: false,
            second_auto: false,
            second_languages: String::new(),
            second_position: None,
            remember_tracks: true,
            sync_step_2: 1.0,
            sync_step_3: 10.0,
            keep_sync: false,
            folders: Vec::new(),
            encoding: "auto".to_owned(),
            sync_from_next: false,
            browser: crate::subtitle_editor::BrowserSettings::default(),
        }
    }
}

/// Whether `text` is a colour mpv takes: `#RRGGBB` or `#AARRGGBB`.
#[must_use]
pub fn is_color(text: &str) -> bool {
    text.strip_prefix('#')
        .is_some_and(|hex| matches!(hex.len(), 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()))
}

/// An ASS colour (`&HAABBGGRR`, alpha 00 opaque) for mpv's `#AARRGGBB`
/// (alpha FF opaque).
#[must_use]
pub fn ass_color(color: &str) -> Option<String> {
    let hex = color.strip_prefix('#')?;
    let (alpha, rgb) = match hex.len() {
        6 => ("FF", hex),
        8 => hex.split_at(2),
        _ => return None,
    };
    let alpha = 255 - u8::from_str_radix(alpha, 16).ok()?;
    let (r, rest) = rgb.split_at(2);
    let (g, b) = rest.split_at(2);
    Some(format!("&H{alpha:02X}{b}{g}{r}").to_uppercase())
}

impl SubtitleSettings {
    /// Fixes values out of range or of the wrong form, with a note each.
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let choose = |value: &mut String,
                      allowed: &[&str],
                      fallback: &str,
                      name: &str,
                      notes: &mut Vec<String>| {
            if !allowed.contains(&value.as_str()) {
                notes.push(format!(
                    "subtitles.{name} {value:?} is not one of {allowed:?}: {fallback}"
                ));
                fallback.clone_into(value);
            }
        };
        choose(
            &mut self.ass_override,
            ASS_OVERRIDE,
            "scale",
            "ass-override",
            notes,
        );
        choose(
            &mut self.border_style,
            BORDER_STYLE,
            "outline-and-shadow",
            "border-style",
            notes,
        );
        choose(&mut self.align, ALIGN, "center", "align", notes);
        choose(&mut self.encoding, ENCODINGS, "auto", "encoding", notes);
        for (name, color) in [
            ("color", &mut self.color),
            ("outline-color", &mut self.outline_color),
            ("back-color", &mut self.back_color),
        ] {
            if let Some(text) = color.as_deref()
                && !is_color(text)
            {
                notes.push(format!(
                    "subtitles.{name} {text:?} is not #RRGGBB or #AARRGGBB: libmpv's default"
                ));
                *color = None;
            }
        }
        self.validate_numbers(notes);
        self.folders.retain(|folder| !folder.trim().is_empty());
        self.browser.validate(notes);
    }

    /// The numbers' and languages' part of [`SubtitleSettings::validate`].
    fn validate_numbers(&mut self, notes: &mut Vec<String>) {
        let range =
            |value: &mut Option<f64>, low: f64, high: f64, name: &str, notes: &mut Vec<String>| {
                if let Some(v) = *value {
                    if !v.is_finite() {
                        notes.push(format!(
                            "subtitles.{name} is not a number: libmpv's default"
                        ));
                        *value = None;
                    } else if !(low..=high).contains(&v) {
                        notes.push(format!("subtitles.{name} {v} is out of {low} to {high}"));
                        *value = Some(v.clamp(low, high));
                    }
                }
            };
        range(&mut self.outline_size, 0.0, 20.0, "outline-size", notes);
        range(&mut self.shadow_offset, 0.0, 20.0, "shadow-offset", notes);
        range(&mut self.blur, 0.0, 20.0, "blur", notes);
        range(&mut self.spacing, -10.0, 10.0, "spacing", notes);
        range(&mut self.line_spacing, -100.0, 100.0, "line-spacing", notes);
        range(&mut self.scale_x, 10.0, 400.0, "scale-x", notes);
        range(&mut self.scale_y, 10.0, 400.0, "scale-y", notes);
        range(&mut self.position, 0.0, 150.0, "position", notes);
        range(
            &mut self.second_position,
            0.0,
            150.0,
            "second-position",
            notes,
        );
        for (name, margin) in [
            ("margin-x", &mut self.margin_x),
            ("margin-y", &mut self.margin_y),
        ] {
            if let Some(v) = *margin
                && !(0..=300).contains(&v)
            {
                // libmpv 0.37's range (later ones take more).
                notes.push(format!("subtitles.{name} {v} is out of 0 to 300"));
                *margin = Some(v.clamp(0, 300));
            }
        }
        for (name, step) in [
            ("sync-step-2", &mut self.sync_step_2),
            ("sync-step-3", &mut self.sync_step_3),
        ] {
            if !step.is_finite() || *step <= 0.0 || *step > 600.0 {
                notes.push(format!(
                    "subtitles.{name} {step} is out of 0.001 to 600 s: 1 s"
                ));
                *step = 1.0;
            }
        }
        if !self.second_languages.split(',').all(|code| {
            let code = code.trim();
            code.is_empty() || code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        }) {
            notes.push(format!(
                "subtitles.second-languages {:?} must be language codes separated by commas: cleared",
                self.second_languages
            ));
            self.second_languages.clear();
        }
    }

    /// The `sub-ass-style-overrides` list RVP's font, colour and scales make.
    fn style_overrides(&self, font: &str) -> String {
        let mut overrides = Vec::new();
        if self.override_font && !font.is_empty() {
            overrides.push(format!("FontName={font}"));
        }
        if self.override_color
            && let Some(color) = self.color.as_deref().and_then(ass_color)
        {
            overrides.push(format!("PrimaryColour={color}"));
        }
        // mpv applies the list to the style it makes for text subtitles
        // too.
        for (name, scale) in [("ScaleX", self.scale_x), ("ScaleY", self.scale_y)] {
            if let Some(scale) = scale {
                overrides.push(format!("{name}={scale:.0}"));
            }
        }
        overrides.join(",")
    }

    /// mpv's options for these settings, each with the older names libmpv
    /// 0.37 knows it by, and `None` for libmpv's own default (whose value
    /// `defaults` holds, read at start). `font` is the `subtitle-font`
    /// setting; `config_dir` holds RVP's `fonts` folder.
    #[must_use]
    pub fn mpv_options(
        &self,
        font: &str,
        config_dir: Option<&Path>,
        defaults: &MpvDefaults,
    ) -> Vec<(&'static [&'static str], String)> {
        let yes_no = |on: bool| if on { "yes" } else { "no" }.to_owned();
        let number = |value: Option<f64>, default: &str| {
            value.map_or_else(|| default.to_owned(), |v| format!("{v:.3}"))
        };
        let fonts_dir = config_dir
            .filter(|_| self.fonts_folder)
            .map(|dir| dir.join("fonts").to_string_lossy().into_owned())
            .unwrap_or_default();
        let paths: Vec<String> = self
            .folders
            .iter()
            .map(|folder| folder.trim().replace(',', "\\,"))
            .collect();
        vec![
            (&["sub-ass-override"], self.ass_override.clone()),
            (&["sub-ass-style-overrides"], self.style_overrides(font)),
            (&["sub-fonts-dir"], fonts_dir),
            (
                &["stretch-image-subs-to-screen"],
                yes_no(self.bitmap_outside),
            ),
            (
                &["sub-use-margins"],
                yes_no(!self.text_inside || self.under_video),
            ),
            (&["sub-ass-force-margins"], yes_no(self.under_video)),
            (&["sub-ass-vsfilter-aspect-compat"], yes_no(self.fix_aspect)),
            (&["sub-filter-sdh"], yes_no(self.remove_sdh)),
            (&["sub-bold"], yes_no(self.bold)),
            (&["sub-italic"], yes_no(self.italic)),
            (
                &["sub-color"],
                self.color.clone().unwrap_or_else(|| defaults.color.clone()),
            ),
            (
                &["sub-outline-color", "sub-border-color"],
                self.outline_color
                    .clone()
                    .unwrap_or_else(|| defaults.outline_color.clone()),
            ),
            (
                &["sub-back-color"],
                self.back_color
                    .clone()
                    .unwrap_or_else(|| defaults.back_color.clone()),
            ),
            (
                &["sub-shadow-color"],
                self.back_color
                    .clone()
                    .unwrap_or_else(|| defaults.shadow_color.clone()),
            ),
            (
                &["sub-outline-size", "sub-border-size"],
                number(self.outline_size, &defaults.outline_size),
            ),
            (
                &["sub-shadow-offset"],
                number(self.shadow_offset, &defaults.shadow_offset),
            ),
            (&["sub-blur"], number(self.blur, &defaults.blur)),
            (&["sub-spacing"], number(self.spacing, &defaults.spacing)),
            (
                &["sub-line-spacing", "sub-ass-line-spacing"],
                number(self.line_spacing, &defaults.line_spacing),
            ),
            (&["sub-border-style"], self.border_style.clone()),
            (&["sub-align-x"], self.align.clone()),
            (
                &["sub-pos"],
                if self.under_video {
                    "100".to_owned()
                } else {
                    number(self.position, "100")
                },
            ),
            (
                &["sub-margin-x"],
                self.margin_x
                    .map_or_else(|| defaults.margin_x.clone(), |v| v.to_string()),
            ),
            (
                &["sub-margin-y"],
                self.margin_y
                    .map_or_else(|| defaults.margin_y.clone(), |v| v.to_string()),
            ),
            (
                &["subs-fallback"],
                if self.only_preferred { "no" } else { "default" }.to_owned(),
            ),
            (
                &["secondary-sub-pos"],
                number(self.second_position, &defaults.second_position),
            ),
            (&["sub-file-paths"], paths.join(",")),
            (&["sub-codepage"], self.encoding.clone()),
        ]
    }
}

/// libmpv's own defaults of the options RVP leaves unset, read when it
/// starts (they differ between libmpv versions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpvDefaults {
    pub color: String,
    pub outline_color: String,
    pub back_color: String,
    pub shadow_color: String,
    pub outline_size: String,
    pub shadow_offset: String,
    pub blur: String,
    pub spacing: String,
    pub line_spacing: String,
    pub margin_x: String,
    pub margin_y: String,
    pub second_position: String,
}

impl Default for MpvDefaults {
    /// mpv 0.41's, for tests and before libmpv answers.
    fn default() -> Self {
        Self {
            color: "#FFFFFFFF".to_owned(),
            outline_color: "#FF000000".to_owned(),
            back_color: "#AF000000".to_owned(),
            shadow_color: "#AF000000".to_owned(),
            outline_size: "1.650".to_owned(),
            shadow_offset: "0.000".to_owned(),
            blur: "0.000".to_owned(),
            spacing: "0.000".to_owned(),
            line_spacing: "0.000".to_owned(),
            margin_x: "25".to_owned(),
            margin_y: "22".to_owned(),
            second_position: "0.000".to_owned(),
        }
    }
}

impl MpvDefaults {
    /// The defaults as the settings window shows them: by the setting's
    /// key, numbers as numbers.
    #[must_use]
    pub fn view(&self) -> serde_json::Value {
        let number = |text: &str| {
            text.parse::<f64>()
                .map_or(serde_json::Value::Null, serde_json::Value::from)
        };
        serde_json::json!({
            "color": self.color,
            "outline-color": self.outline_color,
            "back-color": self.back_color,
            "outline-size": number(&self.outline_size),
            "shadow-offset": number(&self.shadow_offset),
            "blur": number(&self.blur),
            "spacing": number(&self.spacing),
            "line-spacing": number(&self.line_spacing),
            "margin-x": number(&self.margin_x),
            "margin-y": number(&self.margin_y),
            "second-position": number(&self.second_position),
        })
    }

    /// Reads libmpv's defaults (`option-info/NAME/default-value`), each by
    /// the first name it knows; the built-in values stand in for the rest.
    #[must_use]
    pub fn read(engine: &Engine) -> Self {
        let fallback = Self::default();
        let read = |names: &[&str], fallback: String| {
            names
                .iter()
                .find_map(|name| engine.get_text(&format!("option-info/{name}/default-value")))
                .unwrap_or(fallback)
        };
        Self {
            color: read(&["sub-color"], fallback.color),
            outline_color: read(
                &["sub-outline-color", "sub-border-color"],
                fallback.outline_color,
            ),
            back_color: read(&["sub-back-color"], fallback.back_color),
            shadow_color: read(
                &["sub-shadow-color", "sub-back-color"],
                fallback.shadow_color,
            ),
            outline_size: read(
                &["sub-outline-size", "sub-border-size"],
                fallback.outline_size,
            ),
            shadow_offset: read(&["sub-shadow-offset"], fallback.shadow_offset),
            blur: read(&["sub-blur"], fallback.blur),
            spacing: read(&["sub-spacing"], fallback.spacing),
            line_spacing: read(
                &["sub-line-spacing", "sub-ass-line-spacing"],
                fallback.line_spacing,
            ),
            margin_x: read(&["sub-margin-x"], fallback.margin_x),
            margin_y: read(&["sub-margin-y"], fallback.margin_y),
            second_position: read(&["secondary-sub-pos"], fallback.second_position),
        }
    }
}

/// The second subtitle for `tracks`: the first track of the first of
/// `languages` (comma separated) that has one, other than the subtitle
/// shown first.
#[must_use]
pub fn second_subtitle(tracks: &[rvp_engine::Track], languages: &str) -> Option<i64> {
    languages
        .split(',')
        .map(str::trim)
        .filter(|language| !language.is_empty())
        .find_map(|language| {
            tracks
                .iter()
                .filter(|track| track.kind == rvp_engine::TrackKind::Subtitle)
                .filter(|track| !(track.selected && track.main_selection != Some(1)))
                .find(|track| {
                    track
                        .language
                        .as_deref()
                        .is_some_and(|lang| lang.eq_ignore_ascii_case(language))
                })
                .map(|track| track.id)
        })
}

/// An external subtitle file mpv shows, and when it last changed
/// ("Auto reload subtitles on file changes").
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchedFile {
    id: i64,
    path: PathBuf,
    modified: Option<SystemTime>,
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// The external subtitle files of the file that plays (one read of
/// `track-list`).
fn external_subtitles(engine: &Engine) -> Vec<WatchedFile> {
    let Some(list) = engine.get_json("track-list") else {
        return Vec::new();
    };
    list.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|track| track["type"] == "sub" && track["external"].as_bool() == Some(true))
        .filter_map(|track| {
            let path = PathBuf::from(track["external-filename"].as_str()?);
            Some(WatchedFile {
                id: track["id"].as_i64()?,
                modified: modified(&path),
                path,
            })
        })
        .collect()
}

/// The regular expression of empty events (`sub-filter-regex`).
pub const BLANK_EVENT: &str = r"^\s*$";

/// A command of the Subtitles menu (M13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubtitleCommand {
    Bold,
    /// Sideways moves (see [`horizontal_step`]).
    MoveLeft,
    MoveRight,
    /// Letter spacing.
    Wider,
    Narrower,
    /// Line spacing.
    LinesApart,
    LinesCloser,
    OpaqueBox,
    TextInside,
    BitmapOutside,
    UnderVideo,
    /// The Adjust Subtitle Sync dialog, and the same dialog for the frame
    /// rate.
    SyncDialog,
    FpsDialog,
    /// The second (2) or third (3) sync step.
    DelayBy(crate::action::Delay, u8),
    /// The sync that shows the next or the previous subtitle now
    /// (`sub-step`).
    SyncNext,
    SyncPrevious,
    KeepSync,
    /// The subtitles were made for this rate ([`FPS_CHOICES`]).
    Fps(u8),
    Copy,
    /// A second subtitle file shown with the first.
    Combine,
    /// The subtitle browser and editor (Alt+E) and Input Subtitles (Alt+P).
    Browser,
    Input,
    /// Subtitle Saving: the sync written into the file that shows, the
    /// file named after the video, and the save dialog.
    ApplySync,
    SaveAsVideoName,
    SaveAs,
    /// Sync steps move only the subtitles after the playback position.
    SyncFromNext,
}

/// The margin step of sideways moves and their widest margin, in mpv's
/// units (pixels of a 720-line picture).
const MARGIN_STEP: i64 = 20;
const MARGIN_MAX: i64 = 300;

/// One sideways move from `align` and `margin`: mpv anchors subtitles to
/// the left edge, the centre or the right edge (`sub-align-x`) and keeps a
/// margin from that edge (`sub-margin-x`), so a move to the right goes from
/// the left edge with a growing margin to the centre, then to the right
/// edge with a shrinking one (and back the other way).
#[must_use]
pub fn horizontal_step(align: &str, margin: i64, right: bool) -> (&'static str, i64) {
    let margin = (margin / MARGIN_STEP * MARGIN_STEP).clamp(0, MARGIN_MAX);
    match (align, right) {
        ("left", true) | ("right", false) if margin >= MARGIN_MAX => ("center", MARGIN_MAX),
        ("left", true) => ("left", margin + MARGIN_STEP),
        ("left", false) => ("left", (margin - MARGIN_STEP).max(0)),
        ("right", false) => ("right", margin + MARGIN_STEP),
        ("right", true) => ("right", (margin - MARGIN_STEP).max(0)),
        (_, true) => ("right", MARGIN_MAX),
        (_, false) => ("left", MARGIN_MAX),
    }
}

impl App {
    /// Gives mpv the subtitle options (the module's documentation), each by
    /// the first name this libmpv takes; the options libmpv lacks are left
    /// out.
    pub(crate) fn apply_subtitles(&mut self, engine: &Engine) {
        let config_dir = self
            .settings_path()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        let options = self.settings.subtitles.mpv_options(
            &self.settings.subtitle_font,
            config_dir.as_deref(),
            &self.subtitle_defaults,
        );
        for (names, value) in options {
            if self.applied_subtitle_options.get(names[0]) == Some(&value) {
                continue;
            }
            if names
                .iter()
                .any(|name| engine.set_text(name, &value).is_ok())
            {
                self.applied_subtitle_options.insert(names[0], value);
            }
        }
        // Empty events go through mpv's regex filter, which RVP shares
        // with nothing else.
        let blank = self.settings.subtitles.remove_blank;
        let applied = self
            .applied_subtitle_options
            .get("sub-filter-regex-enable")
            .map(String::as_str);
        let wanted = if blank { "yes" } else { "no" };
        if applied != Some(wanted) {
            let list = if blank { BLANK_EVENT } else { "" };
            if engine.set_text("sub-filter-regex", list).is_ok()
                && engine.set_text("sub-filter-regex-enable", wanted).is_ok()
            {
                self.applied_subtitle_options
                    .insert("sub-filter-regex-enable", wanted.to_owned());
            }
        }
    }

    /// The subtitle part of [`App::apply_settings`]: the settings changed
    /// from `old`.
    pub(crate) fn subtitle_settings_changed(
        &mut self,
        engine: &Engine,
        old: &Settings,
        notes: &mut Vec<String>,
    ) {
        let new = &self.settings;
        let mut set = |name: &str, value: &str| {
            if let Err(error) = engine.set_text(name, value) {
                notes.push(format!("cannot set {name}: {error}"));
            }
        };
        if old.subtitle_languages != new.subtitle_languages {
            set("slang", &new.subtitle_languages);
        }
        if old.subtitle_auto != new.subtitle_auto {
            set("sub-auto", &new.subtitle_auto);
        }
        if (old.subtitle_scale - new.subtitle_scale).abs() > f64::EPSILON {
            set("sub-scale", &format!("{:.2}", new.subtitle_scale));
        }
        if old.subtitle_font != new.subtitle_font {
            let font = if new.subtitle_font.is_empty() {
                engine
                    .get_text("option-info/sub-font/default-value")
                    .unwrap_or_else(|| "sans-serif".to_owned())
            } else {
                new.subtitle_font.clone()
            };
            set("sub-font", &font);
        }
        if old.subtitles.show != new.subtitles.show {
            drop(engine.set_flag("sub-visibility", new.subtitles.show));
        }
        let reload = old.subtitles.auto_reload != new.subtitles.auto_reload;
        if old.subtitles != new.subtitles || old.subtitle_font != new.subtitle_font {
            self.apply_subtitles(engine);
        }
        if reload {
            self.refresh_subtitle_files(engine);
        }
    }

    /// The subtitle options at start: libmpv's defaults first, then RVP's
    /// options and whether subtitles show.
    pub(crate) fn start_subtitles(&mut self, engine: &Engine) {
        self.subtitle_defaults = MpvDefaults::read(engine);
        self.applied_subtitle_options.clear();
        self.apply_subtitles(engine);
        drop(engine.set_flag("sub-visibility", self.settings.subtitles.show));
    }

    /// The Subtitles menu's commands (M13); the notice says what changed.
    pub(crate) fn subtitle_command(
        &mut self,
        engine: &Engine,
        command: SubtitleCommand,
    ) -> Option<UiRequest> {
        match command {
            SubtitleCommand::SyncDialog | SubtitleCommand::FpsDialog => {
                Some(UiRequest::SubtitleSyncDialog(
                    self.model.double(PropertyId::SubDelay).unwrap_or(0.0),
                ))
            }
            SubtitleCommand::DelayBy(delay, step) => {
                let subtitles = &self.settings.subtitles;
                let step = if step == 3 {
                    subtitles.sync_step_3
                } else {
                    subtitles.sync_step_2
                };
                let signed = if delay == crate::action::Delay::Earlier {
                    -step
                } else {
                    step
                };
                if let Some(request) = self.sync_from_next(engine, signed) {
                    return Some(request);
                }
                self.delay(engine, "sub-delay", PropertyId::SubDelay, delay, step)
                    .map(|value| {
                        UiRequest::Osd(format!(
                            "Subtitle sync: {}",
                            crate::commands::signed_seconds(value)
                        ))
                    })
            }
            SubtitleCommand::SyncNext | SubtitleCommand::SyncPrevious => {
                let next = command == SubtitleCommand::SyncNext;
                let before = engine.get_double("sub-delay").unwrap_or(0.0);
                let moved = engine
                    .command("sub-step", &[if next { "1" } else { "-1" }])
                    .is_ok();
                let after = engine.get_double("sub-delay").unwrap_or(before);
                Some(UiRequest::Osd(if moved && (after - before).abs() > 1e-6 {
                    format!("Subtitle sync: {}", crate::commands::signed_seconds(after))
                } else if next {
                    "No next subtitle to sync".to_owned()
                } else {
                    "No previous subtitle to sync".to_owned()
                }))
            }
            SubtitleCommand::Fps(index) => {
                let fps = FPS_CHOICES.get(usize::from(index)).copied()?;
                // The checked rate again goes back to the video's.
                let same = self
                    .subtitle_fps
                    .is_some_and(|chosen| (chosen - fps).abs() < 1e-6);
                Some(self.set_subtitle_fps(engine, (!same).then_some(fps)))
            }
            SubtitleCommand::Copy => Some(Self::copy_subtitle(engine)),
            SubtitleCommand::Browser => Some(UiRequest::SubtitleEditor { input: false }),
            SubtitleCommand::Input => Some(UiRequest::SubtitleEditor { input: true }),
            SubtitleCommand::ApplySync => Some(self.apply_subtitle_sync(engine)),
            SubtitleCommand::SaveAsVideoName => Some(self.save_subtitle_as_video_name(engine)),
            SubtitleCommand::SaveAs => Some(self.save_subtitle_dialog(engine)),
            SubtitleCommand::Combine => Some(if self.model.media_loaded() {
                UiRequest::CombineSubtitleDialog
            } else {
                UiRequest::Osd("Open a video first".to_owned())
            }),
            _ => {
                let text = self.change_subtitle_setting(command)?;
                self.save_settings();
                self.apply_subtitles(engine);
                Some(UiRequest::Osd(text))
            }
        }
    }

    /// The Subtitles menu's toggles and steps of RVP's settings: changes
    /// the setting and says what changed.
    fn change_subtitle_setting(&mut self, command: SubtitleCommand) -> Option<String> {
        let defaults_margin = self.subtitle_defaults.margin_x.parse::<i64>().unwrap_or(25);
        let subtitles = &mut self.settings.subtitles;
        let on_off = |on: bool| if on { "on" } else { "off" };
        let text = match command {
            SubtitleCommand::Bold => {
                subtitles.bold = !subtitles.bold;
                format!("Bold subtitles: {}", on_off(subtitles.bold))
            }
            SubtitleCommand::MoveLeft | SubtitleCommand::MoveRight => {
                let margin = subtitles.margin_x.unwrap_or(defaults_margin);
                let (align, margin) = horizontal_step(
                    &subtitles.align,
                    margin,
                    command == SubtitleCommand::MoveRight,
                );
                align.clone_into(&mut subtitles.align);
                subtitles.margin_x = (align != "center").then_some(margin);
                match align {
                    "center" => "Subtitles: centred".to_owned(),
                    side => format!("Subtitles: {margin} from the {side} edge"),
                }
            }
            SubtitleCommand::Wider | SubtitleCommand::Narrower => {
                let step = if command == SubtitleCommand::Wider {
                    0.5
                } else {
                    -0.5
                };
                let current = subtitles
                    .spacing
                    .unwrap_or_else(|| self.subtitle_defaults.spacing.parse().unwrap_or(0.0));
                let next = (current + step).clamp(-10.0, 10.0);
                subtitles.spacing = Some(next);
                format!("Letter spacing: {next}")
            }
            SubtitleCommand::LinesApart | SubtitleCommand::LinesCloser => {
                let step = if command == SubtitleCommand::LinesApart {
                    2.0
                } else {
                    -2.0
                };
                let current = subtitles
                    .line_spacing
                    .unwrap_or_else(|| self.subtitle_defaults.line_spacing.parse().unwrap_or(0.0));
                let next = (current + step).clamp(-100.0, 100.0);
                subtitles.line_spacing = Some(next);
                format!("Line spacing: {next}")
            }
            SubtitleCommand::OpaqueBox => {
                let boxed = subtitles.border_style != "opaque-box";
                if boxed {
                    "opaque-box"
                } else {
                    "outline-and-shadow"
                }
                .clone_into(&mut subtitles.border_style);
                format!("Opaque box behind subtitles: {}", on_off(boxed))
            }
            SubtitleCommand::TextInside => {
                subtitles.text_inside = !subtitles.text_inside;
                format!(
                    "Text subtitles inside the video: {}",
                    on_off(subtitles.text_inside)
                )
            }
            SubtitleCommand::BitmapOutside => {
                subtitles.bitmap_outside = !subtitles.bitmap_outside;
                format!(
                    "Bitmap subtitles on the whole screen: {}",
                    on_off(subtitles.bitmap_outside)
                )
            }
            SubtitleCommand::UnderVideo => {
                subtitles.under_video = !subtitles.under_video;
                format!(
                    "Subtitles under the video: {}",
                    on_off(subtitles.under_video)
                )
            }
            SubtitleCommand::KeepSync => {
                subtitles.keep_sync = !subtitles.keep_sync;
                format!(
                    "Keep subtitle sync for the next files: {}",
                    on_off(subtitles.keep_sync)
                )
            }
            SubtitleCommand::SyncFromNext => {
                subtitles.sync_from_next = !subtitles.sync_from_next;
                format!(
                    "Sync changes from the next subtitle on: {}",
                    on_off(subtitles.sync_from_next)
                )
            }
            _ => return None,
        };
        Some(text)
    }

    /// The check marks of the Subtitles menu's toggles.
    pub(crate) fn subtitle_checked(&self, command: SubtitleCommand) -> Option<bool> {
        let subtitles = &self.settings.subtitles;
        match command {
            SubtitleCommand::Bold => Some(subtitles.bold),
            SubtitleCommand::OpaqueBox => Some(subtitles.border_style == "opaque-box"),
            SubtitleCommand::TextInside => Some(subtitles.text_inside),
            SubtitleCommand::BitmapOutside => Some(subtitles.bitmap_outside),
            SubtitleCommand::UnderVideo => Some(subtitles.under_video),
            SubtitleCommand::KeepSync => Some(subtitles.keep_sync),
            SubtitleCommand::SyncFromNext => Some(subtitles.sync_from_next),
            SubtitleCommand::Fps(index) => {
                let fps = FPS_CHOICES.get(usize::from(index)).copied()?;
                Some(
                    self.subtitle_fps
                        .is_some_and(|chosen| (chosen - fps).abs() < 1e-6),
                )
            }
            _ => None,
        }
    }

    /// The rate the subtitles were made for (`sub-fps`; `None`: the
    /// video's), for this file.
    pub fn set_subtitle_fps(&mut self, engine: &Engine, fps: Option<f64>) -> UiRequest {
        let value = fps.map_or_else(|| "0".to_owned(), |fps| format!("{fps}"));
        if let Err(error) = engine.set_text("sub-fps", &value) {
            return UiRequest::Message(format!("cannot set the subtitles' frame rate: {error}"));
        }
        self.subtitle_fps = fps;
        match fps {
            Some(fps) => {
                let video = self.model.double(PropertyId::ContainerFps).unwrap_or(0.0);
                UiRequest::Osd(format!(
                    "Subtitles made for {fps} fps, played at {video:.3}"
                ))
            }
            None => UiRequest::Osd("Subtitles at the video's frame rate".to_owned()),
        }
    }

    /// The Adjust Subtitle Sync dialog's sync (seconds, within ±10 min).
    pub fn set_subtitle_delay(&mut self, engine: &Engine, seconds: f64) -> UiRequest {
        let seconds = seconds.clamp(-600.0, 600.0);
        match engine.set_double("sub-delay", seconds) {
            Ok(()) => UiRequest::Osd(format!(
                "Subtitle sync: {}",
                crate::commands::signed_seconds(seconds)
            )),
            Err(error) => UiRequest::Message(format!("cannot set the subtitle sync: {error}")),
        }
    }

    /// The rate the playing file's subtitles were made for (`None`: the
    /// video's).
    #[must_use]
    pub fn subtitle_fps(&self) -> Option<f64> {
        self.subtitle_fps
    }

    /// A file started: the frame rate conversion is the last file's, and so
    /// is the sync unless it is kept; the subtitles chosen last time in the
    /// file come back, or a second subtitle of the second languages shows.
    pub(crate) fn subtitle_file_loaded(&mut self, engine: &Engine) {
        if self.subtitle_fps.take().is_some() {
            drop(engine.set_text("sub-fps", "0"));
        }
        let subtitles = &self.settings.subtitles;
        if !subtitles.keep_sync {
            drop(engine.set_double("sub-delay", 0.0));
        }
        let remembered = self
            .model
            .loaded_path()
            .filter(|_| subtitles.remember_tracks)
            .and_then(|path| self.store().subtitle_tracks(path));
        let tracks: Vec<rvp_engine::Track> = engine
            .tracks()
            .into_iter()
            .filter(|track| track.kind == rvp_engine::TrackKind::Subtitle)
            .collect();
        let exists = |id: i64| tracks.iter().any(|track| track.id == id);
        self.refresh_subtitle_files(engine);
        let subtitles = &self.settings.subtitles;
        if let Some((first, second)) = remembered {
            let text = |id: Option<i64>| {
                id.filter(|id| exists(*id))
                    .map_or_else(|| "no".to_owned(), |id| id.to_string())
            };
            if first.is_none_or(exists) {
                drop(engine.set_text("sid", &text(first)));
            }
            if second.is_none_or(exists) {
                drop(engine.set_text("secondary-sid", &text(second)));
            }
            return;
        }
        drop(engine.set_text("secondary-sid", "no"));
        if let Some(id) =
            second_subtitle(&tracks, &subtitles.second_languages).filter(|_| subtitles.second_auto)
        {
            drop(engine.set_text("secondary-sid", &id.to_string()));
        }
    }

    /// Looks at the external subtitle files again: after a file starts or a
    /// subtitle is added.
    pub(crate) fn refresh_subtitle_files(&mut self, engine: &Engine) {
        self.subtitle_files = if self.settings.subtitles.auto_reload {
            external_subtitles(engine)
        } else {
            Vec::new()
        };
    }

    /// Once a second: reloads an external subtitle file that changed on
    /// disk (`sub-reload`; mpv adds the track again under a new ID).
    pub fn watch_subtitle_files(&mut self, engine: &Engine) -> Option<UiRequest> {
        if !self.settings.subtitles.auto_reload {
            return None;
        }
        let changed = self.subtitle_files.iter().position(|file| {
            let now = modified(&file.path);
            now.is_some() && now != file.modified
        })?;
        let file = self.subtitle_files[changed].clone();
        drop(engine.command("sub-reload", &[&file.id.to_string()]));
        self.refresh_subtitle_files(engine);
        Some(UiRequest::Osd(format!(
            "Subtitles reloaded: {}",
            crate::view::file_name(&file.path.to_string_lossy())
        )))
    }

    /// Remembers the subtitles chosen now in the playing file ("Remember
    /// subtitle track selections").
    pub(crate) fn subtitle_tracks_chosen(&mut self, engine: &Engine) {
        if !self.settings.subtitles.remember_tracks {
            return;
        }
        let Some(path) = self.model.loaded_path().map(str::to_owned) else {
            return;
        };
        let id = |name: &str| {
            engine
                .get_text(name)
                .and_then(|text| text.parse::<i64>().ok())
        };
        let (first, second) = (id("sid"), id("secondary-sid"));
        self.store_mut().set_subtitle_tracks(&path, first, second);
    }

    /// Loads a second subtitle file and shows it with the first
    /// (PotPlayer's Combine Subtitle).
    pub fn combine_subtitle(&mut self, engine: &Engine, path: &Path) -> UiRequest {
        let text = path.to_string_lossy();
        if let Err(error) = engine.command("sub-add", &[&text, "auto"]) {
            return UiRequest::Message(format!("Cannot load {text}: {error}"));
        }
        let added = engine
            .tracks()
            .into_iter()
            .filter(|track| track.kind == rvp_engine::TrackKind::Subtitle && track.external)
            .map(|track| track.id)
            .max();
        let request = match added.map(|id| engine.set_text("secondary-sid", &id.to_string())) {
            Some(Ok(())) => UiRequest::Osd(format!(
                "Second subtitles: {}",
                crate::view::file_name(&text)
            )),
            _ => UiRequest::Message(format!("Cannot show {text} as second subtitles")),
        };
        self.subtitle_tracks_chosen(engine);
        self.refresh_subtitle_files(engine);
        request
    }

    /// The subtitle that shows now, for the clipboard (PotPlayer's "Copy
    /// current subtitle to clipboard").
    pub(crate) fn copy_subtitle(engine: &Engine) -> UiRequest {
        let text = engine
            .get_text("sub-text")
            .unwrap_or_default()
            .trim()
            .to_owned();
        if text.is_empty() {
            UiRequest::Osd("No subtitle to copy".to_owned())
        } else {
            UiRequest::CopyText(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_become_ass_colours() {
        assert!(is_color("#FFFFFF"));
        assert!(is_color("#80FF0000"));
        assert!(!is_color("FFFFFF"));
        assert!(!is_color("#FFF"));
        assert_eq!(ass_color("#FF0000").as_deref(), Some("&H000000FF"));
        // mpv's alpha 0x80 (half) is ASS's 0x7F; opaque is 00 in ASS.
        assert_eq!(ass_color("#800000FF").as_deref(), Some("&H7FFF0000"));
        assert_eq!(ass_color("#FFFFFFFF").as_deref(), Some("&H00FFFFFF"));
    }

    #[test]
    fn unset_values_are_libmpvs_and_the_options_follow_the_settings() {
        let defaults = MpvDefaults::default();
        let settings = SubtitleSettings::default();
        let options = settings.mpv_options("", None, &defaults);
        let value = |name: &str| {
            options
                .iter()
                .find(|(names, _)| names[0] == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(value("sub-outline-size").as_deref(), Some("1.650"));
        assert_eq!(value("sub-color").as_deref(), Some("#FFFFFFFF"));
        assert_eq!(value("sub-use-margins").as_deref(), Some("yes"));
        assert_eq!(value("sub-pos").as_deref(), Some("100"));
        assert_eq!(value("sub-fonts-dir").as_deref(), Some(""));
        assert_eq!(value("sub-ass-style-overrides").as_deref(), Some(""));

        let changed = SubtitleSettings {
            text_inside: true,
            bold: true,
            color: Some("#FFFFFF00".to_owned()),
            override_color: true,
            override_font: true,
            fonts_folder: true,
            position: Some(90.0),
            scale_x: Some(120.0),
            folders: vec!["subs".to_owned(), "a,b".to_owned()],
            ..SubtitleSettings::default()
        };
        let options = changed.mpv_options("Arial", Some(Path::new("/c")), &defaults);
        let value = |name: &str| {
            options
                .iter()
                .find(|(names, _)| names[0] == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(value("sub-use-margins").as_deref(), Some("no"));
        assert_eq!(value("sub-bold").as_deref(), Some("yes"));
        assert_eq!(value("sub-pos").as_deref(), Some("90.000"));
        assert_eq!(
            value("sub-ass-style-overrides").as_deref(),
            Some("FontName=Arial,PrimaryColour=&H0000FFFF,ScaleX=120")
        );
        assert_eq!(
            value("sub-fonts-dir"),
            Some(Path::new("/c").join("fonts").to_string_lossy().into_owned())
        );
        assert_eq!(value("sub-file-paths").as_deref(), Some("subs,a\\,b"));
        // Under the video: margins on, forced, at the bottom.
        let under = SubtitleSettings {
            text_inside: true,
            under_video: true,
            position: Some(50.0),
            ..SubtitleSettings::default()
        };
        let options = under.mpv_options("", None, &defaults);
        let value = |name: &str| {
            options
                .iter()
                .find(|(names, _)| names[0] == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(value("sub-use-margins").as_deref(), Some("yes"));
        assert_eq!(value("sub-ass-force-margins").as_deref(), Some("yes"));
        assert_eq!(value("sub-pos").as_deref(), Some("100"));
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = SubtitleSettings {
            ass_override: "always".to_owned(),
            color: Some("white".to_owned()),
            outline_size: Some(99.0),
            margin_x: Some(-3),
            sync_step_2: 0.0,
            second_languages: "en; ru".to_owned(),
            folders: vec![String::new(), "subs".to_owned()],
            ..SubtitleSettings::default()
        };
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(settings.ass_override, "scale");
        assert_eq!(settings.color, None);
        assert_eq!(settings.outline_size, Some(20.0));
        assert_eq!(settings.margin_x, Some(0));
        assert!((settings.sync_step_2 - 1.0).abs() < f64::EPSILON);
        assert!(settings.second_languages.is_empty());
        assert_eq!(settings.folders, ["subs"]);
        assert_eq!(notes.len(), 6, "{notes:?}");
    }
}
