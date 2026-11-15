//! M12's window commands: PotPlayer's Window Size menu (sizes, moves, the
//! size dialogs, fullscreen on another display) and its Playback > Window
//! Size and Fullscreen Mode pages.
//!
//! The app decides from the settings and the video; the window
//! (`shell/qml/Main.qml`, `windowRequest`) carries a [`WindowRequest`] out
//! with the screen's geometry, which only it knows: sizes keep the video's
//! shape and stay on the screen, moves stop at its edges. Wayland does not
//! let a program move its windows (`docs/coverage/linux.md`).

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// Where a move puts the window, or which way it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Placement {
    Left,
    Right,
    Up,
    Down,
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// One of the Window Size menu's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowCommand {
    Minimum,
    Maximum,
    /// The custom size of the settings.
    Custom,
    /// "Set Custom Window Size...".
    CustomDialog,
    /// One of the four sizes relative to the desktop (Alt+1 to Alt+4).
    Desktop(usize),
    /// "Increase/Decrease Window Size".
    Grow(bool),
    Move(Placement),
    /// "Window Size Control...".
    ControlDialog,
    /// "Fullscreen (on Another Display)".
    FullscreenOtherDisplay,
}

/// What the window does (JSON for `Main.qml`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WindowRequest {
    /// The skin's smallest window.
    Minimum,
    /// As large as the screen's free area, keeping the video's shape.
    Maximum,
    /// This size (the whole window), kept on the screen.
    Size { width: i32, height: i32 },
    /// This place (when known) and size, kept on the screen: a skin
    /// mode's own window (M14).
    Place {
        x: Option<i32>,
        y: Option<i32>,
        width: i32,
        height: i32,
    },
    /// A share of the desktop's width, keeping the video's shape.
    Desktop { percent: f64 },
    /// The size times `factor`, keeping the shape.
    Grow { factor: f64 },
    /// A move by `step` pixels, or to a place on the screen.
    Move { placement: Placement, step: i32 },
    /// The custom size dialog, starting from the custom size.
    SizeDialog { width: u32, height: u32 },
    /// The Window Size Control dialog.
    ControlDialog,
    /// Fullscreen on the next display.
    FullscreenOtherDisplay,
}

/// The window's size for a newly opened video (Playback's "Default window
/// size").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenSize {
    /// The video's size times `open-scale`.
    #[default]
    Video,
    /// The window keeps its size.
    Last,
    /// The custom size.
    Custom,
    /// The first of the desktop sizes.
    Desktop,
}

/// Where fullscreen goes (Fullscreen Mode's "Display").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FullscreenDisplay {
    /// The display the window is on.
    #[default]
    Current,
    Primary,
}

/// M12's window settings (`window` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WindowSettings {
    /// "Custom size" and its dialog: the window's width and height.
    pub custom_size: [u32; 2],
    /// Alt+1 to Alt+4: the window's width in percent of the desktop's.
    pub desktop_sizes: [f64; 4],
    /// Increase and Decrease Window Size: percent a step.
    pub grow_step: f64,
    /// The window's moves: pixels a step.
    pub move_step: u32,
    /// The size a newly opened video gets ("Fit the window to a newly
    /// opened video" must be on).
    pub open_size: OpenSize,
    /// The video's size times this for `OpenSize::Video`.
    pub open_scale: f64,
    /// Only the first video of a run gets the size.
    pub open_first_only: bool,
    /// A video narrower than this many pixels opens at twice its size (0:
    /// never).
    pub double_below: u32,
    pub fullscreen_display: FullscreenDisplay,
    /// The other displays turn black while the window is fullscreen.
    pub black_other_displays: bool,
    /// "Restrict mouse cursor to current display" while fullscreen
    /// (Windows).
    pub restrict_cursor: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            custom_size: [960, 600],
            desktop_sizes: [30.0, 45.0, 60.0, 75.0],
            grow_step: 10.0,
            move_step: 20,
            open_size: OpenSize::Video,
            open_scale: 1.0,
            open_first_only: false,
            double_below: 0,
            fullscreen_display: FullscreenDisplay::Current,
            black_other_displays: false,
            restrict_cursor: false,
        }
    }
}

impl WindowSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let clamp = crate::playback::clamp_number;
        for value in &mut self.custom_size {
            if !(160..=16384).contains(value) {
                notes.push("window.custom-size must be 160 to 16384 pixels".to_owned());
                *value = (*value).clamp(160, 16384);
            }
        }
        for value in &mut self.desktop_sizes {
            clamp(value, 10.0, 100.0, "window.desktop-sizes", notes);
        }
        clamp(&mut self.grow_step, 1.0, 50.0, "window.grow-step", notes);
        if !(1..=500).contains(&self.move_step) {
            notes.push("window.move-step must be 1 to 500 pixels: 20 used".to_owned());
            self.move_step = 20;
        }
        clamp(&mut self.open_scale, 0.25, 4.0, "window.open-scale", notes);
        if self.double_below > 4096 {
            notes.push("window.double-below must be 0 to 4096 pixels".to_owned());
            self.double_below = 4096;
        }
    }
}

/// The window's size for a video of `video` pixels with `chrome` around
/// it (the skin's bars), as the settings want it; `None` keeps the window
/// as it is. `first` is whether this is the run's first video.
#[must_use]
pub fn open_size(
    settings: &WindowSettings,
    video: (i32, i32),
    chrome: (i32, i32),
    first: bool,
) -> Option<WindowRequest> {
    if settings.open_first_only && !first {
        return None;
    }
    match settings.open_size {
        OpenSize::Last => None,
        OpenSize::Custom => {
            let [width, height] = settings.custom_size;
            Some(WindowRequest::Size {
                width: i32::try_from(width).ok()?,
                height: i32::try_from(height).ok()?,
            })
        }
        OpenSize::Desktop => Some(WindowRequest::Desktop {
            percent: settings.desktop_sizes[0],
        }),
        OpenSize::Video => {
            let small = settings.double_below > 0
                && u32::try_from(video.0).is_ok_and(|width| width < settings.double_below);
            let scale = if small { 2.0 } else { settings.open_scale };
            // Video sizes are far inside f64's exact range and back.
            #[allow(clippy::cast_possible_truncation)]
            let scaled = |pixels: i32| (f64::from(pixels) * scale).round() as i32;
            Some(WindowRequest::Size {
                width: scaled(video.0) + chrome.0,
                height: scaled(video.1) + chrome.1,
            })
        }
    }
}

impl App {
    /// The Window Size menu's commands.
    pub(crate) fn window_command(&self, command: WindowCommand) -> UiRequest {
        let window = &self.settings.window;
        UiRequest::Window(match command {
            WindowCommand::Minimum => WindowRequest::Minimum,
            WindowCommand::Maximum => WindowRequest::Maximum,
            WindowCommand::Custom => {
                let [width, height] = window.custom_size;
                WindowRequest::Size {
                    width: i32::try_from(width).unwrap_or(960),
                    height: i32::try_from(height).unwrap_or(600),
                }
            }
            WindowCommand::CustomDialog => {
                let [width, height] = window.custom_size;
                WindowRequest::SizeDialog { width, height }
            }
            WindowCommand::Desktop(index) => WindowRequest::Desktop {
                percent: window.desktop_sizes.get(index).copied().unwrap_or(50.0),
            },
            WindowCommand::Grow(larger) => {
                let step = window.grow_step / 100.0;
                WindowRequest::Grow {
                    factor: if larger {
                        1.0 + step
                    } else {
                        1.0 / (1.0 + step)
                    },
                }
            }
            WindowCommand::Move(placement) => WindowRequest::Move {
                placement,
                step: i32::try_from(window.move_step).unwrap_or(20),
            },
            WindowCommand::ControlDialog => WindowRequest::ControlDialog,
            WindowCommand::FullscreenOtherDisplay => WindowRequest::FullscreenOtherDisplay,
        })
    }

    /// The custom size dialog's answer (`WIDTHxHEIGHT`): kept, and the
    /// window takes it.
    pub fn window_size_answer(&mut self, text: &str) -> UiRequest {
        let size = text
            .split_once(['x', 'X', '*', ' '])
            .and_then(|(width, height)| {
                Some([width.trim().parse().ok()?, height.trim().parse().ok()?])
            })
            .filter(|[width, height]: &[u32; 2]| {
                (160..=16384).contains(width) && (160..=16384).contains(height)
            });
        let Some(size) = size else {
            return UiRequest::Message(format!("{text:?} is not a window size (960x600)"));
        };
        self.settings.window.custom_size = size;
        self.save_settings();
        self.window_command(WindowCommand::Custom)
    }

    /// The window's size for the video that just loaded (see
    /// [`open_size`]); `chrome` is the skin's bars around the video.
    pub fn open_window_size(
        &mut self,
        video: (i32, i32),
        chrome: (i32, i32),
    ) -> Option<WindowRequest> {
        let first = self.session.sized_videos == 0;
        self.session.sized_videos = self.session.sized_videos.saturating_add(1);
        open_size(&self.settings.window, video, chrome, first)
    }

    /// True while the window is fullscreen (the app's record of it).
    #[must_use]
    pub fn is_fullscreen(&self) -> bool {
        self.picture.fullscreen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_videos_get_the_size_the_settings_say() {
        let mut settings = WindowSettings::default();
        let chrome = (20, 60);
        assert_eq!(
            open_size(&settings, (640, 360), chrome, true),
            Some(WindowRequest::Size {
                width: 660,
                height: 420
            })
        );
        settings.open_scale = 1.5;
        assert_eq!(
            open_size(&settings, (640, 360), chrome, true),
            Some(WindowRequest::Size {
                width: 980,
                height: 600
            })
        );
        // Small videos at twice their size.
        settings.double_below = 400;
        assert_eq!(
            open_size(&settings, (320, 240), chrome, false),
            Some(WindowRequest::Size {
                width: 660,
                height: 540
            })
        );
        settings.open_first_only = true;
        assert_eq!(open_size(&settings, (320, 240), chrome, false), None);
        settings.open_size = OpenSize::Desktop;
        assert_eq!(
            open_size(&settings, (320, 240), chrome, true),
            Some(WindowRequest::Desktop { percent: 30.0 })
        );
        settings.open_size = OpenSize::Last;
        assert_eq!(open_size(&settings, (320, 240), chrome, true), None);
        let json = serde_json::to_string(&WindowRequest::Move {
            placement: Placement::TopLeft,
            step: 20,
        })
        .unwrap_or_default();
        assert_eq!(json, r#"{"kind":"move","placement":"top-left","step":20}"#);
    }
}
