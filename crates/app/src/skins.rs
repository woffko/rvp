//! M14: PotPlayer's Skins menu and Skins pages (goal P).
//!
//! * The skin list: RVP Default (built in), the `.dsf` skins (and M9's
//!   classic Winamp `.wsz` and VLC `.vlt` skins) in RVP's `skins` folder
//!   (beside the settings), next to the chosen skin and, on Windows,
//!   PotPlayer's installed `Skins` folder (read only). A choice switches
//!   the skin at once.
//! * Skin modes: a file without video shows the skin's audio skin
//!   (`AudioSkin.xml`) when it has one, others its video skin, unless
//!   switching is turned off ("Disable switching between skin modes"); "Use
//!   video skin at program start" starts in the video skin. With "Remember
//!   window position & size of each skin mode" each mode keeps its own
//!   window; otherwise "Keep window size" keeps the size it has.
//! * Colour themes (`rvp_skin::theme`): none, a random one of the skin's
//!   (picked once per start), a custom one from the Customize dialog (HSB
//!   or RGB), or one the skin names.
//! * Logos: `crate::logos`; the context menu's design and look:
//!   `crate::menu_design`.
//! * On Screen Control: the skin's bars over the video in fullscreen
//!   (`MainSkinAreaOscInside`) can be turned off, made translucent, and
//!   kept or hidden while the pointer rests on them. "Auto hide main skin
//!   while video screen is active" shows the window that way too while a
//!   video plays, and "Auto resize video depending on fullscreen pop-ups"
//!   lets the video make room for the bars while they show.
//! * The skin's times: elapsed and remaining can trade places, and all of
//!   them can run at the playback speed (`App::skin_times`).
//! * The shell loads and tints the skins ([`SkinWant`]) and tells the app
//!   what the skin offers ([`SkinInfo`]).
//! * A skin is chosen in the Skins menu or page, by opening its file (also
//!   one a second `rvp` hands over), by a preset or imported settings, or by
//!   `--skin=PATH`; the start shows the configured one. Each way ends in the
//!   shell's load, which tells the app how it went: a skin that loaded is
//!   the one shown, and a skin the setting names is saved only from then on
//!   ([`App::skin_loaded`]); one that did not load leaves the skin shown and
//!   the setting as they were, with a notice that says why
//!   ([`App::skin_failed`]). What no error catches (an abort, a hang) is
//!   `crate::skin_marker`'s: a start after a load that never finished shows
//!   RVP Default ([`App::skip_unfinished_skins`]).

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::logos::{LOGO_DEFAULT, LOGO_POSITIONS, LOGO_SAME, LOGO_SKIN};
use crate::menu_design::{MENU_DEFAULT, MENU_SKIN, MENU_STYLES};

/// Theme choices that are not a skin's theme names.
pub const THEME_NONE: &str = "";
pub const THEME_RANDOM: &str = "random";
pub const THEME_CUSTOM: &str = "custom";

/// The Customize dialog's theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CustomTheme {
    /// HSB (`hue`, `sat`) rather than RGB offsets.
    pub hsb: bool,
    /// -128 to 128.
    pub hue: i32,
    /// 0 to 100.
    pub sat: i32,
    /// -255 to 255 each.
    pub red: i32,
    pub green: i32,
    pub blue: i32,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            hsb: true,
            hue: -26,
            sat: 60,
            red: 0,
            green: 0,
            blue: 0,
        }
    }
}

impl CustomTheme {
    /// The tint it gives.
    #[must_use]
    pub fn tint(&self) -> rvp_skin::theme::Tint {
        if self.hsb {
            rvp_skin::theme::Tint::Hsb {
                hue: self.hue,
                sat: self.sat,
            }
        } else {
            rvp_skin::theme::Tint::Rgb([self.red, self.green, self.blue])
        }
    }
}

/// The Skins menu's and pages' settings (`skins` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's Skins pages.
#[allow(clippy::struct_excessive_bools)]
pub struct SkinsSettings {
    /// [`THEME_NONE`], [`THEME_RANDOM`], [`THEME_CUSTOM`] or a theme's name.
    pub color_theme: String,
    pub custom: CustomTheme,
    /// Start in the video skin rather than the mode last shown.
    pub video_skin_at_start: bool,
    /// Stay in one skin mode whatever plays.
    pub lock_mode: bool,
    /// Each mode keeps its window's place and size.
    pub remember_mode_window: bool,
    /// The window keeps its size when the mode changes (off, as in
    /// PotPlayer: its audio skin shrinks the window to the skin's bars,
    /// seen in goal P's host session).
    pub keep_size: bool,
    /// The main window's and the control panel's opacity in percent.
    pub opacity: u8,
    pub panel_opacity: u8,
    /// The logo while nothing plays: [`LOGO_SKIN`], [`LOGO_DEFAULT`] or a
    /// picture's file name (`crate::logos`).
    pub idle_logo: String,
    /// The logo while a file starts: [`LOGO_SAME`] or as `idle_logo`.
    pub start_logo: String,
    /// One of [`LOGO_POSITIONS`].
    pub logo_position: String,
    /// A random picture of the logo folders for each file.
    pub random_logo: bool,
    /// The skin's bars over the video in fullscreen (On Screen Control).
    pub osc: bool,
    /// The bars stay while the pointer rests on them; off: they hide after
    /// [`OSC_HIDE_SECONDS`].
    pub osc_keep_while_over: bool,
    /// The bars' opacity over the video, in percent.
    pub osc_opacity: u8,
    /// The main window's skin and the control panel drawn larger, in
    /// percent (Zoom Skinned Windows; 100 to 300).
    pub zoom: u16,
    pub panel_zoom: u16,
    /// While a video shows in the window, the skin hides and its bars show
    /// over the video when the pointer comes, as in fullscreen.
    pub auto_hide: bool,
    /// In fullscreen the video makes room for the bar that shows. Off by
    /// default: PotPlayer's video keeps its place and size under its bar,
    /// whose own window is opaque (goal P's host captures; RVP gives the
    /// bar a black backdrop, `Main.qml`).
    pub fullscreen_resize: bool,
    /// The bars and the control panel fade in and out.
    pub animations: bool,
    /// The skin's elapsed and remaining times trade places.
    pub reverse_times: bool,
    /// The skin's times run at the playback speed (real time).
    pub speed_times: bool,
    /// The context menu: [`MENU_SKIN`], [`MENU_DEFAULT`] or a menu file's
    /// name (`crate::menu_design`).
    pub menu_design: String,
    /// One of [`MENU_STYLES`].
    pub menu_style: String,
    /// Dark menus while the system is dark.
    pub menu_dark: bool,
    /// The menus' transparency in percent (0 to 90).
    pub menu_transparency: u8,
    /// The menus' text size in percent (80 to 200).
    pub menu_size: u16,
    /// Top-level items of the context menu the user hid, by their text.
    pub menu_hidden: Vec<String>,
}

/// How long the bars stay while the pointer rests on them when they do not
/// stay for good.
pub const OSC_HIDE_SECONDS: u64 = 3;

impl Default for SkinsSettings {
    fn default() -> Self {
        Self {
            color_theme: THEME_NONE.to_owned(),
            custom: CustomTheme::default(),
            video_skin_at_start: true,
            lock_mode: false,
            remember_mode_window: false,
            keep_size: false,
            opacity: 100,
            panel_opacity: 100,
            idle_logo: LOGO_SKIN.to_owned(),
            start_logo: LOGO_SAME.to_owned(),
            logo_position: "center".to_owned(),
            random_logo: false,
            osc: true,
            osc_keep_while_over: true,
            osc_opacity: 100,
            zoom: 100,
            panel_zoom: 100,
            auto_hide: false,
            fullscreen_resize: false,
            animations: true,
            reverse_times: false,
            speed_times: false,
            menu_design: MENU_SKIN.to_owned(),
            menu_style: "auto".to_owned(),
            menu_dark: false,
            menu_transparency: 0,
            menu_size: 100,
            menu_hidden: Vec::new(),
        }
    }
}

impl SkinsSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let custom = &mut self.custom;
        for (name, value, low, high) in [
            ("hue", &mut custom.hue, -128, 128),
            ("sat", &mut custom.sat, 0, 100),
            ("red", &mut custom.red, -255, 255),
            ("green", &mut custom.green, -255, 255),
            ("blue", &mut custom.blue, -255, 255),
        ] {
            if !(low..=high).contains(value) {
                notes.push(format!(
                    "skins.custom.{name} {value} is out of {low} to {high}"
                ));
                *value = (*value).clamp(low, high);
            }
        }
        if self.idle_logo == LOGO_SAME || self.idle_logo.trim().is_empty() {
            notes.push(format!(
                "skins.idle-logo {:?} is no logo: {LOGO_SKIN}",
                self.idle_logo
            ));
            LOGO_SKIN.clone_into(&mut self.idle_logo);
        }
        if self.start_logo.trim().is_empty() {
            notes.push("skins.start-logo is empty: same".to_owned());
            LOGO_SAME.clone_into(&mut self.start_logo);
        }
        if self.menu_design.trim().is_empty() {
            notes.push("skins.menu-design is empty: skin".to_owned());
            MENU_SKIN.clone_into(&mut self.menu_design);
        }
        if !MENU_STYLES.contains(&self.menu_style.as_str()) {
            notes.push(format!(
                "skins.menu-style {:?} is none of {MENU_STYLES:?}: auto",
                self.menu_style
            ));
            "auto".clone_into(&mut self.menu_style);
        }
        if self.menu_transparency > 90 {
            notes.push(format!(
                "skins.menu-transparency {} is out of 0 to 90: 90",
                self.menu_transparency
            ));
            self.menu_transparency = 90;
        }
        for (name, value) in [
            ("zoom", &mut self.zoom),
            ("panel-zoom", &mut self.panel_zoom),
        ] {
            if !(100..=300).contains(value) {
                notes.push(format!("skins.{name} {value} is out of 100 to 300: 100"));
                *value = 100;
            }
        }
        if !(80..=200).contains(&self.menu_size) {
            notes.push(format!(
                "skins.menu-size {} is out of 80 to 200: 100",
                self.menu_size
            ));
            self.menu_size = 100;
        }
        if !LOGO_POSITIONS.contains(&self.logo_position.as_str()) {
            notes.push(format!(
                "skins.logo-position {:?} is none of {LOGO_POSITIONS:?}: center",
                self.logo_position
            ));
            "center".clone_into(&mut self.logo_position);
        }
        for (name, value) in [
            ("opacity", &mut self.opacity),
            ("panel-opacity", &mut self.panel_opacity),
            ("osc-opacity", &mut self.osc_opacity),
        ] {
            if !(10..=100).contains(value) {
                notes.push(format!("skins.{name} {value} is out of 10 to 100: 100"));
                *value = 100;
            }
        }
    }
}

/// The skin that shows in place of the setting's until the user picks a
/// skin: the command line's or `RVP_SKIN`'s, or the one shown when the
/// skin asked for did not load.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SkinOverride {
    /// The setting's skin shows.
    #[default]
    Off,
    /// RVP Default (`None`) or a `.dsf` file.
    Skin(Option<PathBuf>),
}

/// The skin the window shows and the skin setting as saved.
#[derive(Debug, Clone, Default)]
pub(crate) struct ShownSkin {
    /// The skin the window shows (`None`: RVP Default).
    path: Option<PathBuf>,
    /// The skin setting as saved: the configured one at start, then each
    /// one the window showed from the setting. Another skin the setting
    /// names is saved once the window shows it, so the settings never name
    /// a skin that did not load.
    kept: Option<PathBuf>,
    /// The configured skin this start left out (`crate::skin_marker`):
    /// listed for the session, to be chosen again on purpose.
    skipped: Option<PathBuf>,
    /// The skin of the last Skins menu choice or opened skin file did not
    /// load.
    refused: bool,
}

impl ShownSkin {
    /// The configured `skin`, shown and saved.
    pub(crate) fn new(skin: Option<PathBuf>) -> Self {
        Self {
            path: skin.clone(),
            kept: skin,
            ..Self::default()
        }
    }
}

/// A skin's name for notices: its file's, or RVP Default.
fn skin_name(skin: Option<&Path>) -> String {
    skin.map_or_else(
        || "RVP Default".to_owned(),
        |path| crate::view::file_name(&path.to_string_lossy()).to_owned(),
    )
}

/// What the loaded skin offers (from the shell).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkinInfo {
    /// Its colour themes' names, in its order.
    pub themes: Vec<String>,
    /// It has an audio skin (`AudioSkin.xml`).
    pub audio: bool,
    /// Its `<Logo>` section draws an idle logo (`CMD_DEFAULT_LOGO`) or one
    /// for a starting file (`CMD_LOADING_LOGO`).
    pub logo: bool,
    pub loading_logo: bool,
}

/// The skin the window should show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinWant {
    /// The `.dsf` file; `None`: RVP Default.
    pub path: Option<PathBuf>,
    /// The audio skin rather than the video skin.
    pub audio: bool,
    pub theme: ThemeWant,
    /// The skin's touch layout (Accessibility > Touch, M14).
    pub touch: bool,
    /// Another file of the skin its own button chose for this mode
    /// (`CMD_LOAD_SKIN_XML`: BlackSpace's Mini Audio Mode); `None`:
    /// `VideoSkin.xml` or `AudioSkin.xml`.
    pub file: Option<String>,
}

/// A skin file chosen by the skin's own button, for one skin and mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkinFile {
    path: Option<PathBuf>,
    audio: bool,
    file: String,
}

/// The colour theme the window should show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeWant {
    None,
    /// One of the skin's themes, by name.
    Named(String),
    Custom(rvp_skin::theme::Tint),
}

/// A command of the Skins menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkinCommand {
    /// Colour themes: none, random, the custom one.
    ThemeNone,
    ThemeRandom,
    ThemeCustom,
    /// The custom theme is HSB (`true`) or RGB.
    ThemeKind(bool),
    /// The Customize dialog.
    Customize,
    /// The main window's (`false`) or control panel's opacity, in tens of
    /// percent (1 to 10).
    Opacity(bool, u8),
    VideoSkinAtStart,
    LockMode,
    RememberModeWindow,
    /// The idle logo: the skin's (0) or RVP's (1).
    IdleLogo(u8),
    /// The start logo: the idle one (0), the skin's (1) or RVP's (2).
    StartLogo(u8),
    /// On Screen Control on or off.
    Osc(bool),
    AutoHide,
    FullscreenResize,
    /// Context Menu Design: the skin's (0) or RVP's (1), both RVP's whole
    /// menu.
    MenuDesign(u8),
    /// Context Menu Style: an index of [`MENU_STYLES`].
    MenuStyle(u8),
    MenuDark,
}

/// A run-time list choice of the Skins menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinChoice {
    /// A skin of [`App::skin_list`].
    Skin(usize),
    /// A colour theme of the skin, by its index.
    Theme(usize),
    /// A picture of [`App::logo_list`] as the idle or the start logo.
    IdleLogo(usize),
    StartLogo(usize),
    /// A file of [`App::menu_files`] as the context menu.
    MenuFile(usize),
}

impl SkinChoice {
    /// The menu's ID: `s3`, `t5`.
    #[must_use]
    pub fn id(self) -> String {
        match self {
            Self::Skin(index) => format!("s{index}"),
            Self::Theme(index) => format!("t{index}"),
            Self::IdleLogo(index) => format!("i{index}"),
            Self::StartLogo(index) => format!("l{index}"),
            Self::MenuFile(index) => format!("m{index}"),
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (kind, index) = text.split_at_checked(1)?;
        let index = index.parse().ok()?;
        match kind {
            "s" => Some(Self::Skin(index)),
            "t" => Some(Self::Theme(index)),
            "i" => Some(Self::IdleLogo(index)),
            "l" => Some(Self::StartLogo(index)),
            "m" => Some(Self::MenuFile(index)),
            _ => None,
        }
    }
}

/// Flips `flag` and says so.
fn toggle(flag: &mut bool, what: &str) -> String {
    *flag = !*flag;
    format!("{what}: {}", if *flag { "on" } else { "off" })
}

/// The skin files in `folder` ([`is_skin_file`]).
fn skins_in(folder: &Path, out: &mut std::collections::BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_skin_file(&path) {
            out.insert(path);
        }
    }
}

/// A skin file: PotPlayer's `.dsf`, a classic Winamp skin (`.wsz`) or a
/// VLC theme (`.vlt`, M9). `.dsf` is also DSD audio (Sony's DSD Stream
/// File, which starts with `DSD `): a `.dsf` is a skin only as a folder or
/// a ZIP archive (`PK`), and the audio plays.
#[must_use]
pub fn is_skin_file(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    if ext.eq_ignore_ascii_case("wsz") || ext.eq_ignore_ascii_case("vlt") {
        return true;
    }
    ext.eq_ignore_ascii_case("dsf") && (path.is_dir() || starts_as_zip(path))
}

/// Whether the file starts as a ZIP archive does.
fn starts_as_zip(path: &Path) -> bool {
    use std::io::Read as _;
    let mut start = [0_u8; 2];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut start))
        .is_ok_and(|()| start == *b"PK")
}

impl App {
    /// RVP's skins folder (beside the settings).
    #[must_use]
    pub fn skins_folder(&self) -> PathBuf {
        self.settings_path()
            .and_then(Path::parent)
            .map_or_else(|| self.state_dir().join("skins"), |dir| dir.join("skins"))
    }

    /// The skins to choose from: `None` is RVP Default, then the `.dsf`
    /// files by path: RVP's skins folder, the folders of the skin set and
    /// of the one the command line chose, and PotPlayer's on Windows.
    #[must_use]
    pub fn skin_list(&self) -> Vec<Option<PathBuf>> {
        let mut found = std::collections::BTreeSet::new();
        skins_in(&self.skins_folder(), &mut found);
        let chosen = [
            self.settings.skin.as_ref(),
            match &self.skin_override {
                SkinOverride::Skin(skin) => skin.as_ref(),
                SkinOverride::Off => None,
            },
            self.shown_skin.skipped.as_ref(),
        ];
        for skin in chosen.into_iter().flatten() {
            if let Some(folder) = skin.parent() {
                skins_in(folder, &mut found);
            }
            found.insert(skin.clone());
        }
        if cfg!(windows) {
            skins_in(
                Path::new(r"C:\Program Files\DAUM\PotPlayer\Skins"),
                &mut found,
            );
        }
        std::iter::once(None)
            .chain(found.into_iter().map(Some))
            .collect()
    }

    /// The shell shows the audio (`true`) or video skin: kept for the next
    /// start while a file plays.
    pub fn skin_shown(&mut self, audio: bool) {
        if self.model.media_loaded() {
            self.store_mut().set_audio_skin(audio);
        }
    }

    /// The window the audio (`true`) or video skin mode had, when each mode
    /// keeps its own.
    #[must_use]
    pub fn mode_window(&self, audio: bool) -> Option<crate::resume::WindowGeometry> {
        self.store()
            .mode_window(if audio { "audio" } else { "video" })
    }

    /// Keeps the window of a skin mode for its next time.
    pub fn remember_mode_window(&mut self, audio: bool, window: crate::resume::WindowGeometry) {
        self.store_mut()
            .set_mode_window(if audio { "audio" } else { "video" }, window);
    }

    /// The skin to show: the command line's until the user picks one, else
    /// the setting's (`None`: RVP Default).
    #[must_use]
    pub fn chosen_skin(&self) -> Option<PathBuf> {
        match &self.skin_override {
            SkinOverride::Skin(skin) => skin.clone(),
            SkinOverride::Off => self.settings.skin.clone(),
        }
    }

    /// At start, the skin the window shows (`None`: RVP Default): the
    /// command line's or `RVP_SKIN`'s, the setting's, or RVP Default when
    /// the one asked for did not load. It is the skin chosen until the user
    /// picks one; the setting stays.
    pub fn skin_started(&mut self, shown: Option<PathBuf>) {
        self.skin_override = if self.settings.skin == shown {
            SkinOverride::Off
        } else {
            SkinOverride::Skin(shown.clone())
        };
        self.shown_skin.path = shown;
    }

    /// At start, the skins whose load or first frame never finished
    /// (`crate::skin_marker`), before the skin loads: the configured one
    /// is not loaded, its setting is cleared, and it stays in the skin list
    /// for this session to be chosen again on purpose. Returns the notices.
    pub fn skip_unfinished_skins(&mut self, unfinished: &[PathBuf]) -> Vec<String> {
        let mut notices = Vec::new();
        for skin in unfinished {
            let name = skin_name(Some(skin));
            let configured = self
                .settings
                .skin
                .as_deref()
                .is_some_and(|configured| crate::skin_marker::same_skin(configured, skin));
            if configured {
                self.shown_skin.skipped = self.settings.skin.take();
                self.shown_skin.kept = None;
                self.shown_skin.path = None;
                self.save_settings();
                notices.push(format!(
                    "RVP closed while the skin {name} was loading last time: RVP Default is \
                     shown instead (choose the skin again in the Skins menu to try it)"
                ));
            } else {
                notices.push(format!(
                    "RVP closed while the skin {name} was loading last time"
                ));
            }
        }
        notices
    }

    /// The skin the window shows (`None`: RVP Default).
    #[must_use]
    pub fn skin_showing(&self) -> Option<&Path> {
        self.shown_skin.path.as_deref()
    }

    /// The window shows the skin it wanted ([`App::skin_want`]'s `path`):
    /// one the setting names is saved from now on.
    pub fn skin_loaded(&mut self, path: Option<&Path>) {
        self.shown_skin.path = path.map(Path::to_path_buf);
        if self.settings.skin.as_deref() == path && self.shown_skin.kept != self.settings.skin {
            self.shown_skin.kept.clone_from(&self.settings.skin);
            self.save_settings();
        }
    }

    /// The window could not load the skin `path` it wanted (`error` says
    /// why): the skin shown stays, and so does the setting the choice
    /// replaced; the skin shown is the one chosen. Returns the notice.
    pub fn skin_failed(&mut self, path: Option<&Path>, error: &str) -> String {
        let shown = self.shown_skin.path.clone();
        if self.settings.skin.as_deref() == path {
            self.settings.skin.clone_from(&self.shown_skin.kept);
        }
        self.skin_override = if self.settings.skin == shown {
            SkinOverride::Off
        } else {
            SkinOverride::Skin(shown.clone())
        };
        self.shown_skin.refused = true;
        format!(
            "cannot load the skin {error}; {} stays",
            skin_name(shown.as_deref())
        )
    }

    /// Whether the skin of the last Skins menu choice or opened skin file
    /// did not load (asked once, for its notice).
    pub fn take_skin_refused(&mut self) -> bool {
        std::mem::take(&mut self.shown_skin.refused)
    }

    /// The settings as saved: a skin the setting names is saved once the
    /// window shows it ([`App::skin_loaded`]).
    pub(crate) fn settings_to_save(&self) -> Cow<'_, crate::Settings> {
        if self.settings.skin == self.shown_skin.kept {
            Cow::Borrowed(&self.settings)
        } else {
            Cow::Owned(crate::Settings {
                skin: self.shown_skin.kept.clone(),
                ..self.settings.clone()
            })
        }
    }

    /// The shell loaded a skin: what it offers.
    pub fn set_skin_info(&mut self, info: SkinInfo) {
        if info.themes != self.skin_info.themes {
            self.random_theme = None;
        }
        self.skin_info = info;
    }

    /// The themes of the loaded skin.
    #[must_use]
    pub fn skin_themes(&self) -> &[String] {
        &self.skin_info.themes
    }

    /// The skin's times: elapsed and remaining trade places, and all of
    /// them run at the playback speed, as the Skins page sets.
    pub fn skin_times(&self, data: &mut rvp_skin::DataModel) {
        let skins = &self.settings.skins;
        if skins.speed_times {
            let speed = self
                .model
                .double(rvp_engine::PropertyId::Speed)
                .filter(|speed| *speed > 0.0)
                .unwrap_or(1.0);
            if (speed - 1.0).abs() > 1e-9 {
                let time = self
                    .model
                    .double(rvp_engine::PropertyId::TimePos)
                    .unwrap_or(0.0);
                let duration = self
                    .model
                    .double(rvp_engine::PropertyId::Duration)
                    .filter(|duration| *duration > 0.0)
                    .unwrap_or(0.0);
                let format = crate::view::format_time;
                data.set_text("CMD_CURRENTTIME_MESSAGE", format(time / speed));
                data.set_text("CMD_TOTALTIME_MESSAGE", format(duration / speed));
                data.set_text(
                    "CMD_LEFTTIME_MESSAGE",
                    format((duration - time).max(0.0) / speed),
                );
            }
        }
        if skins.reverse_times {
            let elapsed = data.text("CMD_CURRENTTIME_MESSAGE").map(str::to_owned);
            let left = data.text("CMD_LEFTTIME_MESSAGE").map(str::to_owned);
            if let (Some(elapsed), Some(left)) = (elapsed, left) {
                data.set_text("CMD_CURRENTTIME_MESSAGE", left);
                data.set_text("CMD_LEFTTIME_MESSAGE", elapsed);
            }
        }
    }

    /// The skin the window should show now.
    #[must_use]
    pub fn skin_want(&mut self) -> SkinWant {
        let skins = &self.settings.skins;
        let theme = match skins.color_theme.as_str() {
            THEME_NONE => ThemeWant::None,
            THEME_CUSTOM => ThemeWant::Custom(skins.custom.tint()),
            THEME_RANDOM => {
                let count = self.skin_info.themes.len();
                if count == 0 {
                    ThemeWant::None
                } else {
                    let index = *self.random_theme.get_or_insert_with(|| {
                        // Once per start: the clock's nanoseconds.
                        let nanos = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |time| time.subsec_nanos());
                        usize::try_from(nanos).unwrap_or(0) % count
                    });
                    ThemeWant::Named(self.skin_info.themes[index % count].clone())
                }
            }
            name => ThemeWant::Named(name.to_owned()),
        };
        let path = self.chosen_skin();
        let audio = self.skin_mode_audio();
        // A file the skin's button chose stays for its skin and mode.
        let file = self
            .skin_file
            .as_ref()
            .filter(|chosen| chosen.path == path && chosen.audio == audio)
            .map(|chosen| chosen.file.clone());
        SkinWant {
            path,
            audio,
            theme,
            touch: self.settings.accessibility.touch_skin,
            file,
        }
    }

    /// A skin's `CMD_LOAD_SKIN_XML` button: its file shows for the skin
    /// mode now shown (the mode's own file again clears the choice). Only
    /// a plain `.xml` name of the skin is taken.
    pub fn load_skin_file(&mut self, file: &str) {
        let plain = Path::new(file)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
            && !file.contains(['/', '\\']);
        if !plain {
            return;
        }
        let audio = self.skin_mode_audio();
        let own = if audio {
            "AudioSkin.xml"
        } else {
            "VideoSkin.xml"
        };
        self.skin_file = (!file.eq_ignore_ascii_case(own)).then(|| SkinFile {
            path: self.chosen_skin(),
            audio,
            file: file.to_owned(),
        });
    }

    /// Whether the audio skin should show: a file without video plays, the
    /// skin has one and switching is on; at the start the video skin when
    /// so set.
    fn skin_mode_audio(&self) -> bool {
        let skins = &self.settings.skins;
        if !self.skin_info.audio || skins.lock_mode {
            return false;
        }
        if !self.model.media_loaded() {
            return !skins.video_skin_at_start && self.store().state().audio_skin;
        }
        self.audio_only
    }

    /// The Skins menu's commands; `SkinChanged` asks the shell to show the
    /// skin wanted now.
    pub(crate) fn skin_command(&mut self, command: SkinCommand) -> UiRequest {
        self.shown_skin.refused = false;
        let skins = &mut self.settings.skins;
        let text = match command {
            SkinCommand::ThemeNone => {
                THEME_NONE.clone_into(&mut skins.color_theme);
                "Color theme: none".to_owned()
            }
            SkinCommand::ThemeRandom => {
                THEME_RANDOM.clone_into(&mut skins.color_theme);
                self.random_theme = None;
                "Color theme: random".to_owned()
            }
            SkinCommand::ThemeCustom => {
                THEME_CUSTOM.clone_into(&mut skins.color_theme);
                "Color theme: custom".to_owned()
            }
            SkinCommand::ThemeKind(hsb) => {
                skins.custom.hsb = hsb;
                THEME_CUSTOM.clone_into(&mut skins.color_theme);
                format!("Custom color theme: {}", if hsb { "HSB" } else { "RGB" })
            }
            SkinCommand::Customize => return UiRequest::ColorThemeDialog,
            SkinCommand::Opacity(panel, tens) => {
                let percent = tens.clamp(1, 10) * 10;
                if panel {
                    skins.panel_opacity = percent;
                } else {
                    skins.opacity = percent;
                }
                format!(
                    "{} opacity: {percent} %",
                    if panel { "Control panel" } else { "Window" }
                )
            }
            SkinCommand::VideoSkinAtStart => {
                toggle(&mut skins.video_skin_at_start, "Video skin at start")
            }
            SkinCommand::LockMode => toggle(&mut skins.lock_mode, "One skin mode for every file"),
            SkinCommand::RememberModeWindow => {
                toggle(&mut skins.remember_mode_window, "Window per skin mode")
            }
            SkinCommand::IdleLogo(choice) => {
                skins.random_logo = false;
                let choice = if choice == 0 { LOGO_SKIN } else { LOGO_DEFAULT };
                return self.set_logo(false, choice);
            }
            SkinCommand::StartLogo(choice) => {
                skins.random_logo = false;
                let choice = [LOGO_SAME, LOGO_SKIN, LOGO_DEFAULT][usize::from(choice.min(2))];
                return self.set_logo(true, choice);
            }
            SkinCommand::Osc(on) => {
                skins.osc = on;
                format!("On screen control: {}", if on { "on" } else { "off" })
            }
            SkinCommand::AutoHide => {
                toggle(&mut skins.auto_hide, "Auto hide the skin over the video")
            }
            SkinCommand::FullscreenResize => toggle(
                &mut skins.fullscreen_resize,
                "Room for the bars in fullscreen",
            ),
            SkinCommand::MenuDesign(choice) => {
                return self.set_menu_design(if choice == 0 { MENU_SKIN } else { MENU_DEFAULT });
            }
            SkinCommand::MenuStyle(index) => {
                let style = MENU_STYLES.get(usize::from(index)).unwrap_or(&"auto");
                (*style).clone_into(&mut skins.menu_style);
                format!("Context menu style: {style}")
            }
            SkinCommand::MenuDark => toggle(&mut skins.menu_dark, "Dark menus with a dark system"),
        };
        self.save_settings();
        UiRequest::SkinChanged(text)
    }

    /// The Skins menu's check marks.
    #[must_use]
    pub(crate) fn skin_checked(&self, command: SkinCommand) -> Option<bool> {
        let skins = &self.settings.skins;
        match command {
            SkinCommand::ThemeNone => Some(skins.color_theme == THEME_NONE),
            SkinCommand::ThemeRandom => Some(skins.color_theme == THEME_RANDOM),
            SkinCommand::ThemeCustom => Some(skins.color_theme == THEME_CUSTOM),
            SkinCommand::ThemeKind(hsb) => Some(skins.custom.hsb == hsb),
            SkinCommand::Customize => None,
            SkinCommand::Opacity(panel, tens) => {
                let percent = if panel {
                    skins.panel_opacity
                } else {
                    skins.opacity
                };
                Some(percent == tens.clamp(1, 10) * 10)
            }
            SkinCommand::VideoSkinAtStart => Some(skins.video_skin_at_start),
            SkinCommand::LockMode => Some(skins.lock_mode),
            SkinCommand::RememberModeWindow => Some(skins.remember_mode_window),
            SkinCommand::IdleLogo(choice) => {
                Some(self.logo_is(false, if choice == 0 { LOGO_SKIN } else { LOGO_DEFAULT }))
            }
            SkinCommand::StartLogo(choice) => Some(self.logo_is(
                true,
                [LOGO_SAME, LOGO_SKIN, LOGO_DEFAULT][usize::from(choice.min(2))],
            )),
            SkinCommand::Osc(on) => Some(skins.osc == on),
            SkinCommand::AutoHide => Some(skins.auto_hide),
            SkinCommand::FullscreenResize => Some(skins.fullscreen_resize),
            SkinCommand::MenuDesign(choice) => {
                Some(skins.menu_design == if choice == 0 { MENU_SKIN } else { MENU_DEFAULT })
            }
            SkinCommand::MenuStyle(index) => {
                Some(MENU_STYLES.get(usize::from(index)) == Some(&skins.menu_style.as_str()))
            }
            SkinCommand::MenuDark => Some(skins.menu_dark),
        }
    }

    /// Skin files among files opened (the command line's or a second
    /// instance's) are no media: the last one becomes the skin, as if
    /// chosen in the Skins menu, and they leave the list. PotPlayer applies
    /// a `.dsf` that is opened too. As a menu choice, the setting is saved
    /// once the window shows the skin.
    pub fn take_skin_files(&mut self, files: &mut Vec<String>) -> Option<UiRequest> {
        let mut skins = files
            .iter()
            .filter(|file| !crate::playlist::is_url(file) && is_skin_file(Path::new(file)))
            .cloned()
            .collect::<Vec<_>>();
        let skin = skins.pop()?;
        files.retain(|file| !skins.contains(file) && *file != skin);
        // Absolute, as the Skins menu lists them (no `\\?\` prefix of
        // `canonicalize` on Windows).
        let path = std::path::absolute(&skin).unwrap_or_else(|_| PathBuf::from(&skin));
        let name = skin_name(Some(&path));
        self.settings.skin = Some(path);
        self.skin_override = SkinOverride::Off;
        self.shown_skin.refused = false;
        Some(UiRequest::SkinChanged(format!("Skin: {name}")))
    }

    /// A choice of the Skins menu's lists.
    pub(crate) fn skin_choice(&mut self, choice: SkinChoice) -> Option<UiRequest> {
        self.shown_skin.refused = false;
        let text = match choice {
            SkinChoice::Skin(index) => {
                let skin = self.skin_list().get(index)?.clone();
                let name = skin_name(skin.as_deref());
                self.settings.skin = skin;
                self.skin_override = SkinOverride::Off;
                format!("Skin: {name}")
            }
            SkinChoice::Theme(index) => {
                let name = self.skin_info.themes.get(index)?.clone();
                name.clone_into(&mut self.settings.skins.color_theme);
                format!("Color theme: {name}")
            }
            SkinChoice::IdleLogo(index) | SkinChoice::StartLogo(index) => {
                let path = self.logo_list().get(index)?.clone();
                let name = path.file_name()?.to_string_lossy().into_owned();
                self.settings.skins.random_logo = false;
                return Some(self.set_logo(matches!(choice, SkinChoice::StartLogo(_)), &name));
            }
            SkinChoice::MenuFile(index) => {
                let path = self.menu_files().get(index)?.clone();
                let name = path.file_name()?.to_string_lossy().into_owned();
                return Some(self.set_menu_design(&name));
            }
        };
        self.save_settings();
        Some(UiRequest::SkinChanged(text))
    }

    /// Whether a list choice is the one shown.
    #[must_use]
    pub fn skin_choice_checked(&self, choice: SkinChoice) -> bool {
        match choice {
            SkinChoice::Skin(index) => self
                .skin_list()
                .get(index)
                .is_some_and(|skin| *skin == self.chosen_skin()),
            SkinChoice::Theme(index) => self
                .skin_info
                .themes
                .get(index)
                .is_some_and(|name| *name == self.settings.skins.color_theme),
            SkinChoice::MenuFile(index) => self.menu_files().get(index).is_some_and(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .eq_ignore_ascii_case(&self.settings.skins.menu_design)
                })
            }),
            SkinChoice::IdleLogo(index) | SkinChoice::StartLogo(index) => {
                self.logo_list().get(index).is_some_and(|path| {
                    path.file_name().is_some_and(|name| {
                        self.logo_is(
                            matches!(choice, SkinChoice::StartLogo(_)),
                            &name.to_string_lossy(),
                        )
                    })
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_have_menu_ids() {
        for choice in [
            SkinChoice::Skin(0),
            SkinChoice::Theme(12),
            SkinChoice::IdleLogo(1),
            SkinChoice::StartLogo(2),
        ] {
            assert_eq!(SkinChoice::parse(&choice.id()), Some(choice));
        }
        assert_eq!(SkinChoice::parse("x1"), None);
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = SkinsSettings {
            opacity: 5,
            ..SkinsSettings::default()
        };
        settings.custom.hue = 300;
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(settings.opacity, 100);
        assert_eq!(settings.custom.hue, 128);
    }
}
