//! M14: PotPlayer's General, When Starting and Dialog Box pages (goal P):
//! how RVP's windows behave around the desktop.
//!
//! * Instances: a second start hands its files to the running RVP, or
//!   opens a window of its own, moved aside from the others when so set.
//! * The tray icon (Qt's), the taskbar's progress, buttons and jump list
//!   (Windows), the system's media controls and the start with the system.
//! * Pausing while the window is minimized, has lost the focus or the
//!   screen is locked, and playing on when that ends (only what RVP
//!   paused).
//! * Window moves and sizes: from the centre, snapped to the screen's
//!   edges, back to the starting size when a file closes, and where and how
//!   large the window starts.
//! * Tooltips, dropped files' menu, error messages once playback starts,
//!   portrait pictures turned to landscape.
//! * Dialogs: their font and size, kept on top, and Qt's DPI scaling
//!   (taken at start).
//! * The screen saver (`--screensaver`, Windows' `rvp.scr`): the page's
//!   folder or file in fullscreen, repeated, shuffled and muted when so
//!   set, with a state of its own and the settings left as they are.

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// RVP's interface languages: the system's (when RVP has it, else
/// English), English, Russian. RVP's own translations (Qt's .ts files in
/// `shell/i18n`), never PotPlayer's.
pub const LANGUAGES: &[(&str, &str)] = &[("auto", ""), ("en", "English"), ("ru", "Русский")];

/// A second start: hands its files over, or opens its own window.
pub const INSTANCES: &[&str] = &["single", "multiple"];
/// The tray icon: none (the taskbar's button only), next to it, or in its
/// place while the window is minimized.
pub const TRAY: &[&str] = &["taskbar", "both", "minimize-to-tray"];
/// Where the window starts: where it was, centred, or at `x`, `y`.
pub const START_POSITIONS: &[&str] = &["last", "center", "custom"];
/// How large the window starts: as it was, or `width` x `height`.
pub const START_SIZES: &[&str] = &["last", "custom"];

/// The General page (`general` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's pages.
#[allow(clippy::struct_excessive_bools)]
pub struct GeneralSettings {
    /// The interface's language: a code of [`LANGUAGES`].
    pub language: String,
    /// One of [`INSTANCES`].
    pub instances: String,
    /// A new instance's window moves aside from a running one's.
    pub cascade: bool,
    /// One of [`TRAY`].
    pub tray: String,
    /// The control panel hides with the minimized main window.
    pub hide_windows_minimized: bool,
    pub pause_minimized: bool,
    pub pause_focus_lost: bool,
    pub pause_screen_locked: bool,
    /// Size changes keep the window's centre (else its top left corner).
    pub resize_from_center: bool,
    /// A start with files does not take the keyboard from the active
    /// program.
    pub no_focus_at_start: bool,
    /// Dropped files ask what to do (play, add, open as subtitles).
    pub drop_menu: bool,
    /// Error messages go once playback starts.
    pub hide_errors_when_playing: bool,
    /// The skin's tooltips.
    pub tooltips: bool,
    /// Moved windows snap to the screen's edges.
    pub snap: bool,
    /// The taskbar button's progress, playback buttons and jump list
    /// (Windows).
    pub taskbar: bool,
    /// The system's media controls (Windows' and MPRIS).
    pub media_controls: bool,
    /// The window goes back to its starting size when a file closes.
    pub default_size_on_close: bool,
    /// RVP starts with the user's session.
    pub run_at_startup: bool,
    /// Pictures taller than wide turn to landscape.
    pub rotate_portrait_images: bool,
    pub start: StartSettings,
    pub dialog: DialogSettings,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "auto".to_owned(),
            instances: "single".to_owned(),
            cascade: false,
            tray: "taskbar".to_owned(),
            hide_windows_minimized: true,
            pause_minimized: false,
            pause_focus_lost: false,
            pause_screen_locked: false,
            resize_from_center: false,
            no_focus_at_start: false,
            drop_menu: false,
            hide_errors_when_playing: false,
            tooltips: true,
            snap: false,
            taskbar: true,
            media_controls: true,
            default_size_on_close: false,
            run_at_startup: false,
            rotate_portrait_images: false,
            start: StartSettings::default(),
            dialog: DialogSettings::default(),
        }
    }
}

/// When Starting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct StartSettings {
    /// One of [`START_POSITIONS`].
    pub position: String,
    pub x: i32,
    pub y: i32,
    /// One of [`START_SIZES`].
    pub size: String,
    pub width: u32,
    pub height: u32,
    /// Maximized (or minimized) as it was last.
    pub restore_state: bool,
}

impl Default for StartSettings {
    fn default() -> Self {
        Self {
            position: "last".to_owned(),
            x: 100,
            y: 100,
            size: "last".to_owned(),
            width: 960,
            height: 600,
            restore_state: true,
        }
    }
}

/// Dialog Box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DialogSettings {
    /// RVP's own font for its windows (empty: Qt's) and its size in points
    /// (0: Qt's).
    pub font: String,
    pub font_size: u16,
    /// Settings, editors and info windows stay over other programs.
    pub on_top: bool,
    /// Qt scales the windows with the display (taken at start).
    pub dpi_aware: bool,
}

impl Default for DialogSettings {
    fn default() -> Self {
        Self {
            font: String::new(),
            font_size: 0,
            on_top: false,
            dpi_aware: true,
        }
    }
}

impl GeneralSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let mut choose = |name: &str, value: &mut String, known: &[&str]| {
            if !known.contains(&value.as_str()) {
                notes.push(format!(
                    "general.{name} {value:?} is none of {known:?}: {}",
                    known[0]
                ));
                known[0].clone_into(value);
            }
        };
        let languages: Vec<&str> = LANGUAGES.iter().map(|(code, _)| *code).collect();
        choose("language", &mut self.language, &languages);
        choose("instances", &mut self.instances, INSTANCES);
        choose("tray", &mut self.tray, TRAY);
        choose("start.position", &mut self.start.position, START_POSITIONS);
        choose("start.size", &mut self.start.size, START_SIZES);
        let start = &mut self.start;
        if !(200..=16_384).contains(&start.width) || !(120..=16_384).contains(&start.height) {
            notes.push(format!(
                "general.start size {}x{} is out of 200x120 to 16384x16384: 960x600",
                start.width, start.height
            ));
            (start.width, start.height) = (960, 600);
        }
        if self.dialog.font_size > 72 {
            notes.push(format!(
                "general.dialog.font-size {} is out of 0 to 72: 0",
                self.dialog.font_size
            ));
            self.dialog.font_size = 0;
        }
    }
}

/// Where and how large the window starts (`None`: as it was), and whether
/// it restores maximized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartWindow {
    pub position: Option<(i32, i32)>,
    pub size: Option<(u32, u32)>,
    pub restore_state: bool,
}

/// The Screen Saver page (`screensaver` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ScreenSaverSettings {
    /// A folder or a file; empty: RVP's logo.
    pub media: String,
    pub shuffle: bool,
    pub mute: bool,
}

impl Default for ScreenSaverSettings {
    fn default() -> Self {
        Self {
            media: String::new(),
            shuffle: true,
            mute: true,
        }
    }
}

/// Why the window asks RVP to pause or play on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attention {
    Minimized(bool),
    Focus(bool),
    ScreenLocked(bool),
}

impl App {
    /// The window was minimized or restored, lost or got the focus, or the
    /// screen was locked or unlocked: pauses as the General page says, and
    /// plays on what it paused once nothing holds it.
    pub fn window_attention(&mut self, engine: &rvp_engine::Engine, attention: Attention) {
        let general = &self.settings.general;
        let holds = &mut self.attention_holds;
        let (bit, held, wanted) = match attention {
            Attention::Minimized(on) => (1, on, general.pause_minimized),
            Attention::Focus(focused) => (2, !focused, general.pause_focus_lost),
            Attention::ScreenLocked(on) => (4, on, general.pause_screen_locked),
        };
        if held {
            // Paused already by the window, or playing now: pause (once).
            if wanted
                && (*holds != 0 || self.model.playing())
                && (*holds != 0 || engine.set_pause(true).is_ok())
            {
                *holds |= bit;
            }
        } else if *holds & bit != 0 {
            *holds &= !bit;
            if *holds == 0 {
                drop(engine.set_pause(false));
            }
        }
    }

    /// The user paused or played: what the window paused is theirs now.
    pub(crate) fn forget_attention(&mut self) {
        self.attention_holds = 0;
    }

    /// The window's start as When Starting sets it.
    #[must_use]
    pub fn start_window(&self) -> StartWindow {
        let start = &self.settings.general.start;
        StartWindow {
            position: (start.position == "custom").then_some((start.x, start.y)),
            size: (start.size == "custom").then_some((start.width, start.height)),
            restore_state: start.restore_state,
        }
    }

    /// "Auto rotate based on image dimensions" for the file playing: a
    /// picture turns to the screen's orientation as rotation by aspect ratio
    /// turns videos.
    #[must_use]
    pub(crate) fn turns_pictures(&self) -> bool {
        self.settings.general.rotate_portrait_images
            && self
                .model
                .text(rvp_engine::PropertyId::FileFormat)
                .is_some_and(|format| format.ends_with("_pipe") || format == "image2")
    }

    /// RVP starts as the screen saver: the page's media, repeated (and
    /// shuffled) in the start's request; the settings are not saved.
    pub fn start_screensaver(&mut self, request: &mut crate::instance::Request) {
        let saver = self.settings.screensaver.clone();
        self.settings_path = None;
        self.settings.repeat = crate::Repeat::Album;
        self.settings.playback.shuffle = saver.shuffle;
        request.mode = crate::instance::Mode::Play;
        request.files = if saver.media.trim().is_empty() {
            Vec::new()
        } else {
            vec![saver.media]
        };
    }

    /// Dropped files, asked what to do with (`drop_menu`): the choices'
    /// request.
    #[must_use]
    pub fn drop_request(&self, paths: Vec<String>) -> Option<UiRequest> {
        self.settings
            .general
            .drop_menu
            .then_some(UiRequest::DropMenu(paths))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_checked() {
        let mut settings = GeneralSettings {
            instances: "many".to_owned(),
            ..GeneralSettings::default()
        };
        settings.start.width = 10;
        settings.dialog.font_size = 300;
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert_eq!(settings.instances, "single");
        assert_eq!(settings.start.width, 960);
        assert_eq!(settings.dialog.font_size, 0);
    }
}
