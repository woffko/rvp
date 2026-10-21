//! M14's part of `Player`: the skin, its mode (the audio skin) and its
//! colour theme switched while RVP runs, the windows' opacity and limits,
//! the logo over the video area, and the colour theme's dialog
//! (`ColorTheme.qml`).

use std::pin::Pin;

use cxx_qt_lib::QString;
use rvp_app::UiRequest;
use rvp_app::chrome::Mode;
use rvp_app::pointer::Pointer;
use rvp_app::skins::{SkinInfo, ThemeWant};
use rvp_app::window::WindowRequest;

use rvp_app::chrome::Chrome;
use rvp_app::logos::Logo;

use super::{
    LOGO_FRAME, PANEL_ZOOM, SkinSource, ZOOM, file_url, request, show_skin, skin_variant,
    to_window, window_length,
};
use crate::player_qobject::qobject;

/// M14's part of the player's core.
#[derive(Default)]
pub(super) struct Look {
    /// The skin, mode and theme shown.
    pub(super) skin_want: Option<rvp_app::skins::SkinWant>,
    /// The skin's logo, laid out to the video area while it shows, and
    /// what it was last laid out for.
    pub(super) logo: Option<Chrome>,
    pub(super) logo_key: Option<LogoKey>,
    /// When the pointer last moved while the bars show over the video
    /// (Skins > On Screen Control's auto hide).
    pub(super) bars_moved: Option<std::time::Instant>,
}

/// RVP's own logo (`crates/look`), from the shell's resources.
const RVP_LOGO: &str = "qrc:/qt/qml/Rvp/look/logo/symbol.png";

/// What the skin's logo was laid out for: the skin, the logo state, the
/// size and the starting file's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LogoKey {
    skin: u64,
    starting: bool,
    width: i32,
    height: i32,
    message: String,
}

impl qobject::Player {
    pub(super) fn m14_request(mut self: Pin<&mut Self>, request: UiRequest) {
        match request {
            UiRequest::SkinChanged(text) => {
                self.as_mut().sync_skin();
                self.as_mut().sync_opacity();
                self.as_mut().refresh_overlay();
                self.as_mut().sync_logo();
                self.set_osd(QString::from(&text));
            }
            UiRequest::ColorThemeDialog => self.ui_request(request::COLOR_THEME, 0),
            other => eprintln!("rvp: unhandled request {other:?}"),
        }
    }

    /// Shows the skin the application wants now (`App::skin_want`: the
    /// skin, the audio or video skin, the colour theme) when it differs
    /// from the one shown.
    pub(super) fn sync_skin(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if core.chrome.is_none() {
            return;
        }
        let want = core.app.skin_want();
        if core.look.skin_want.as_ref() == Some(&want) {
            return;
        }
        let source = want
            .path
            .clone()
            .map_or(SkinSource::Default, SkinSource::File);
        // The video skin first: its themes and whether it has an audio
        // skin decide the rest.
        let base = match skin_variant(&source, None, None) {
            Ok(skin) => skin,
            Err(error) => {
                // The skin cannot be shown: the one shown stays.
                core.look.skin_want = Some(want);
                self.show_message(&format!("cannot load the skin {error}"));
                return;
            }
        };
        let themes = base.color_themes();
        let (logo, loading_logo) = base.logo_states();
        core.app.set_skin_info(SkinInfo {
            themes: themes.iter().map(|theme| theme.name.clone()).collect(),
            audio: base.has_file("AudioSkin.xml"),
            logo,
            loading_logo,
        });
        let want = core.app.skin_want();
        let tint = match &want.theme {
            ThemeWant::None => None,
            ThemeWant::Custom(tint) => Some(*tint),
            ThemeWant::Named(name) => themes
                .iter()
                .find(|theme| theme.name == *name)
                .map(|theme| theme.tint),
        };
        let file = want.audio.then_some("AudioSkin.xml");
        let skin = match skin_variant(&source, file, tint) {
            Ok(skin) => skin,
            Err(error) => {
                core.look.skin_want = Some(want);
                self.show_message(&format!("cannot load the skin {error}"));
                return;
            }
        };
        let mode_changed = core
            .look
            .skin_want
            .as_ref()
            .is_some_and(|old| old.audio != want.audio);
        let shown = core.chrome.as_ref().map(rvp_app::chrome::Chrome::skin);
        if shown.map(rvp_skin::Skin::id) == Some(skin.id()) {
            core.look.skin_want = Some(want);
            return;
        }
        show_skin(skin);
        if let Some(chrome) = core.chrome.as_mut() {
            chrome.set_skin(skin);
        }
        core.pointer = Pointer::default();
        core.popup = None;
        // The control panel comes back with the new skin when shown.
        let panel = core.panel.take().is_some();
        core.app.skin_shown(want.audio);
        let audio = want.audio;
        core.look.skin_want = Some(want);
        self.as_mut().relayout();
        if panel && *self.panel_shown() {
            self.as_mut().show_panel(true);
        }
        if mode_changed {
            self.as_mut().size_for_mode(audio);
        }
    }

    /// The window for the skin mode just shown: its own when each mode
    /// keeps one (Skins > "Remember window position & size of each skin
    /// mode"), else, unless the window keeps its size, the smallest size
    /// of the sides the skin forces (BlackSpace's audio skin: its bars).
    fn size_for_mode(mut self: Pin<&mut Self>, audio: bool) {
        let fullscreen = *self.fullscreen();
        let (min_width, min_height) = (*self.min_width(), *self.min_height());
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if fullscreen || core.maximized {
            return;
        }
        let skins = core.app.settings.skins.clone();
        let mut request = None;
        if skins.remember_mode_window {
            if let Some(window) = core.app.store().state().window {
                core.app.remember_mode_window(!audio, window);
            }
            request = core
                .app
                .mode_window(audio)
                .map(|window| WindowRequest::Place {
                    x: window.x,
                    y: window.y,
                    width: window.width,
                    height: window.height,
                });
        }
        if request.is_none() && !skins.keep_size {
            let limits = core
                .chrome
                .as_ref()
                .map(|chrome| Mode::Window.limits(chrome.skin()))
                .unwrap_or_default();
            if limits.force_width || limits.force_height {
                request = Some(WindowRequest::Size {
                    width: if limits.force_width {
                        min_width
                    } else {
                        core.width
                    },
                    height: if limits.force_height {
                        min_height
                    } else {
                        core.height
                    },
                });
            }
        }
        if let Some(request) = request {
            self.window_request(&request);
        }
    }

    /// The window's smallest size from the skin shown: its `WidthLimit`
    /// and `HeightLimit` (320x200 when it sets none and forces no side),
    /// never below the bars around the video.
    pub(super) fn sync_limits(mut self: Pin<&mut Self>) {
        let Some(chrome) = self.core().and_then(|core| core.chrome.as_ref()) else {
            return;
        };
        let limits = Mode::Window.limits(chrome.skin());
        let (bars_width, bars_height) = chrome.chrome_size();
        let base = |limit: i32, forced: bool, default: i32| match (limit, forced) {
            (0, false) => default,
            (0, true) => 1,
            (limit, _) => limit,
        };
        let width = window_length(base(limits.width, limits.force_width, 320).max(bars_width));
        let height = window_length(base(limits.height, limits.force_height, 200).max(bars_height));
        if *self.min_width() != width {
            self.as_mut().set_min_width(width);
        }
        if *self.min_height() != height {
            self.as_mut().set_min_height(height);
        }
    }

    /// The logo over the video area while no video shows: a picture for
    /// QML, or the skin's own `<Logo>` laid out to the video area.
    pub(super) fn sync_logo(mut self: Pin<&mut Self>) {
        if *self.has_video() {
            return;
        }
        let video = self.video_rect().clone();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let logo = core.app.logo();
        let position = QString::from(core.app.settings.skins.logo_position.as_str());
        let (source, skin_logo) = match &logo {
            Logo::Skin(_) => (QString::default(), true),
            Logo::Default => (QString::from(RVP_LOGO), false),
            Logo::File(path) => (QString::from(&file_url(path)), false),
        };
        let mut damaged = false;
        if let (Logo::Skin(starting), Some(skin)) = (&logo, core.chrome.as_ref().map(Chrome::skin))
        {
            // Whole pixels of the video area.
            #[allow(clippy::cast_possible_truncation)]
            let (width, height) = (video.width().round() as i32, video.height().round() as i32);
            let key = LogoKey {
                skin: skin.id(),
                starting: *starting,
                width,
                height,
                message: if *starting {
                    core.app.starting_name()
                } else {
                    String::new()
                },
            };
            if core.look.logo_key.as_ref() != Some(&key) && width > 0 && height > 0 {
                let mut chrome = Chrome::new(skin);
                let state = if *starting {
                    "CMD_LOADING_LOGO"
                } else {
                    "CMD_DEFAULT_LOGO"
                };
                chrome.state.flags.insert(state.to_owned());
                chrome
                    .data
                    .set_text("CMD_LOADING_MESSAGE", key.message.as_str());
                if chrome
                    .resize(rvp_app::chrome::Mode::Logo, width, height)
                    .is_ok()
                {
                    chrome.refresh();
                    if let Ok(mut slot) = LOGO_FRAME.lock() {
                        *slot = Some(chrome.frame());
                    }
                    damaged = true;
                }
                core.look.logo = Some(chrome);
                core.look.logo_key = Some(key);
            }
        } else {
            core.look.logo = None;
            core.look.logo_key = None;
        }
        if *self.logo_source() != source {
            self.as_mut().set_logo_source(source);
        }
        if *self.skin_logo() != skin_logo {
            self.as_mut().set_skin_logo(skin_logo);
        }
        if *self.logo_position() != position {
            self.as_mut().set_logo_position(position);
        }
        if damaged {
            self.logo_damaged();
        }
    }

    /// Whether the skin's bars should lie over the video: in fullscreen,
    /// and in the window with "Auto hide main skin" while a video shows.
    pub(super) fn overlay_wanted(&self) -> bool {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.core() else {
            return fullscreen;
        };
        fullscreen
            || (core.app.settings.skins.auto_hide && *self.has_video() && core.chrome.is_some())
    }

    /// Whether a video shows: "Auto hide main skin" follows it.
    pub(super) fn show_video(mut self: Pin<&mut Self>, has_video: bool) {
        if *self.has_video() == has_video {
            return;
        }
        self.as_mut().set_has_video(has_video);
        if *self.overlay() != self.overlay_wanted() {
            self.relayout();
        }
    }

    /// After a settings change: the bars' look over the video, and a new
    /// layout when the overlay comes or goes.
    pub(super) fn refresh_overlay(mut self: Pin<&mut Self>) {
        let zooms = |core: &super::Core| {
            (
                core.app.settings.skins.zoom,
                core.app.settings.skins.panel_zoom,
            )
        };
        let shown = (
            ZOOM.load(std::sync::atomic::Ordering::Relaxed),
            PANEL_ZOOM.load(std::sync::atomic::Ordering::Relaxed),
        );
        let wanted = self.core().map(zooms);
        let zoomed =
            wanted.is_some_and(|(zoom, panel)| (u32::from(zoom), u32::from(panel)) != shown);
        if *self.overlay() == self.overlay_wanted() && !zoomed {
            self.sync_overlay();
            return;
        }
        self.as_mut().relayout();
        // The control panel comes back at its zoom.
        if zoomed && *self.panel_shown() {
            if let Some(core) = self.as_mut().core_mut() {
                core.panel = None;
            }
            self.show_panel(true);
        }
    }

    /// Sets `overlay` and the bars' look over the video from the settings;
    /// the overlay.
    pub(super) fn sync_overlay(mut self: Pin<&mut Self>) -> bool {
        let overlay = self.overlay_wanted();
        let Some(core) = self.core() else {
            return overlay;
        };
        let skins = &core.app.settings.skins;
        let (opacity, animations) = (f64::from(skins.osc_opacity) / 100.0, skins.animations);
        ZOOM.store(u32::from(skins.zoom), std::sync::atomic::Ordering::Relaxed);
        PANEL_ZOOM.store(
            u32::from(skins.panel_zoom),
            std::sync::atomic::Ordering::Relaxed,
        );
        if *self.overlay() != overlay {
            self.as_mut().set_chrome_shown(false);
            self.as_mut().set_overlay(overlay);
        }
        if (*self.osc_opacity() - opacity).abs() > f64::EPSILON {
            self.as_mut().set_osc_opacity(opacity);
        }
        if *self.animations() != animations {
            self.as_mut().set_animations(animations);
        }
        overlay
    }

    /// Shows or hides the bars over the video; in fullscreen with "Auto
    /// resize video depending on fullscreen pop-ups" the video makes room
    /// for them while they show.
    pub(super) fn show_bars(mut self: Pin<&mut Self>, shown: bool) {
        if *self.chrome_shown() != shown {
            self.as_mut().set_chrome_shown(shown);
        }
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let room = shown && fullscreen && core.app.settings.skins.fullscreen_resize;
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        if chrome.set_video_in_slot(room) {
            let video = chrome.video_rect();
            self.set_video_rect(to_window(video));
        }
    }

    /// The bars hide after a while with the pointer resting on them when
    /// On Screen Control does not keep them.
    pub(super) fn tick_bars(mut self: Pin<&mut Self>) {
        if !*self.overlay() || !*self.chrome_shown() {
            return;
        }
        let Some(core) = self.core() else {
            return;
        };
        let rested = core.look.bars_moved.is_none_or(|moved| {
            moved.elapsed() >= std::time::Duration::from_secs(rvp_app::skins::OSC_HIDE_SECONDS)
        });
        if !core.app.settings.skins.osc_keep_while_over && rested && !core.pointer.dragging() {
            self.as_mut().show_bars(false);
        }
    }

    pub(crate) fn menu_look_json(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("{}");
        };
        let skins = &core.app.settings.skins;
        let style = |alt: bool| {
            core.chrome
                .as_ref()
                .and_then(|chrome| chrome.skin().menu_style(alt))
        };
        let style = match skins.menu_style.as_str() {
            "system" => None,
            "skin-2" => style(true).or_else(|| style(false)),
            // "auto" and "skin-1".
            _ => style(false),
        };
        let hex = |[r, g, b, _]: [u8; 4]| format!("#{r:02x}{g:02x}{b:02x}");
        let colors = style.map(|style| {
            serde_json::json!({
                "back": hex(style.back),
                "border": hex(style.border),
                "text": hex(style.text),
                "selectedText": hex(style.selected_text),
                "selectedBack": hex(style.selected_back),
                "dimText": hex(style.dim_text),
                "line": hex(style.line),
            })
        });
        let look = serde_json::json!({
            "native": skins.menu_style == "system",
            "dark": skins.menu_dark,
            "opacity": 1.0 - f64::from(skins.menu_transparency) / 100.0,
            "scale": f64::from(skins.menu_size) / 100.0,
            "colors": colors,
        });
        QString::from(&look.to_string())
    }

    pub(crate) fn settings_folder_url(&self, kind: &QString) -> QString {
        let Some(core) = self.core() else {
            return QString::default();
        };
        let folder = match kind.to_string().as_str() {
            "skins" => core.app.skins_folder(),
            "logos" => core.app.logos_folder(),
            "menus" => core.app.menus_folder(),
            _ => return QString::default(),
        };
        if let Err(error) = std::fs::create_dir_all(&folder) {
            eprintln!("rvp: cannot make {}: {error}", folder.display());
            return QString::default();
        }
        QString::from(&file_url(&folder))
    }

    /// The main window's and the control panel's opacity (Skins >
    /// Opacity).
    pub(super) fn sync_opacity(mut self: Pin<&mut Self>) {
        let Some(core) = self.core() else {
            return;
        };
        let skins = &core.app.settings.skins;
        let (window, panel) = (
            f64::from(skins.opacity) / 100.0,
            f64::from(skins.panel_opacity) / 100.0,
        );
        if (*self.window_opacity() - window).abs() > f64::EPSILON {
            self.as_mut().set_window_opacity(window);
        }
        if (*self.panel_opacity() - panel).abs() > f64::EPSILON {
            self.as_mut().set_panel_opacity(panel);
        }
    }
}
