//! M14's part of `Player`: the skin, its mode (the audio skin) and its
//! colour theme switched while RVP runs, the windows' opacity and limits,
//! the logo over the video area, and the colour theme's dialog
//! (`ColorTheme.qml`).

use std::pin::Pin;

use cxx_qt::Threading;
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
    /// The bar over the video the pointer last showed (chrome pixels).
    pub(super) bar: Option<rvp_skin::Rect>,
    /// "Automatically run at system startup" as last applied (or found at
    /// start).
    pub(super) startup: Option<bool>,
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
                if self
                    .core()
                    .is_some_and(|core| core.app.settings.osd.messages)
                {
                    self.set_osd(QString::from(&text));
                }
            }
            UiRequest::ColorThemeDialog => self.ui_request(request::COLOR_THEME, 0),
            UiRequest::SystemInfo => self.ui_request(request::SYSTEM_INFO, 0),
            // QML takes the language with the General page's look.
            UiRequest::Language(_) => self.sync_general_look(),
            // A preset changed the settings as a whole (M14).
            UiRequest::SettingsChanged(text) => {
                self.as_mut().settings_applied();
                if self
                    .core()
                    .is_some_and(|core| core.app.settings.osd.messages)
                {
                    self.set_osd(QString::from(&text));
                }
            }
            other => self.m16_request(&other),
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
        let touch = want.touch;
        let base = match skin_variant(&source, None, None, touch) {
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
        // The mode's file, or the one the skin's button chose.
        let file = want
            .file
            .as_deref()
            .or_else(|| want.audio.then_some("AudioSkin.xml"));
        let skin = match skin_variant(&source, file, tint, touch) {
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
            .is_some_and(|old| old.audio != want.audio || old.file != want.file);
        let shown = core.chrome.as_ref().map(rvp_app::chrome::Chrome::skin);
        if shown.map(rvp_skin::Skin::id) == Some(skin.id()) {
            core.look.skin_want = Some(want);
            return;
        }
        // Another skin file: the video keeps its size (below).
        let video = core
            .look
            .skin_want
            .as_ref()
            .is_some_and(|old| old.path != want.path)
            .then(|| {
                core.chrome
                    .as_ref()
                    .map(rvp_app::chrome::Chrome::video_rect)
            })
            .flatten();
        show_skin(skin);
        if let Some(chrome) = core.chrome.as_mut() {
            chrome.set_skin(skin);
        }
        // A skin with a spectrum puts its tap into the audio chain.
        core.app.set_skin_spectrum(core.engine, skin.has_visual());
        core.pointer = Pointer::default();
        core.popup = None;
        // The control panel comes back with the new skin when shown.
        let panel = core.panel.take().is_some();
        core.app.skin_shown(want.audio);
        let audio = want.audio;
        core.look.skin_want = Some(want);
        self.as_mut().relayout();
        // The new skin's other windows (M9). Wayland lets no program place
        // its windows: such a skin runs through XWayland, which RVP takes
        // at start (`player::prefer_x11`), so a switch on Wayland says so
        // (the Linux audit's L19).
        self.as_mut().rebuild_extras();
        #[cfg(target_os = "linux")]
        if let Some(note) = wayland_windows_note(skin) {
            self.as_mut().set_osd(QString::from(note));
        }
        if panel && *self.panel_shown() {
            self.as_mut().show_panel(true);
        }
        if mode_changed {
            self.as_mut().size_for_mode(audio);
        } else if let Some(video) = video {
            self.as_mut().keep_video_size(video);
        }
        self.log_skin(audio);
    }

    /// PotPlayer keeps the video's size when the skin changes and builds
    /// the window around it with the new skin's frame, bars and attached
    /// playlist (the skin check's host captures, 2026-11-04: its window
    /// grew and shrank with each skin). "Keep window size" keeps the
    /// window's instead; fullscreen and maximized windows stay as they are.
    fn keep_video_size(self: Pin<&mut Self>, video: rvp_skin::Rect) {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.core() else {
            return;
        };
        if fullscreen
            || core.maximized
            || core.app.settings.skins.keep_size
            || video.w <= 0
            || video.h <= 0
        {
            return;
        }
        let Some(chrome) = core.chrome.as_ref() else {
            return;
        };
        let (width, height) = chrome.chrome_size();
        let request = WindowRequest::Size {
            width: window_length(video.w + width),
            height: window_length(video.h + height),
        };
        self.window_request(&request);
    }

    /// The skin just shown and the window's sizes (the log's Info).
    fn log_skin(&self, audio: bool) {
        if !rvp_app::log::enabled(rvp_app::log::Level::Info) {
            return;
        }
        let Some(core) = self.core() else {
            return;
        };
        let Some(want) = &core.look.skin_want else {
            return;
        };
        let limits = core
            .chrome
            .as_ref()
            .map(|chrome| Mode::Window.limits(chrome.skin()))
            .unwrap_or_default();
        rvp_app::log::write(
            rvp_app::log::Level::Info,
            &format!(
                "skin: {} ({}), window {}x{}, smallest {}x{}{}",
                want.path.as_ref().map_or_else(
                    || "RVP Default".to_owned(),
                    |path| path.display().to_string()
                ),
                want.file.as_deref().unwrap_or(if audio {
                    "AudioSkin.xml"
                } else {
                    "VideoSkin.xml"
                }),
                core.width,
                core.height,
                self.min_width(),
                self.min_height(),
                if limits.force_width || limits.force_height {
                    " (forced)"
                } else {
                    ""
                }
            ),
        );
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
        // A VLC layout's largest size (M9); 0: none.
        let Some(area) = self
            .core()
            .and_then(|core| core.chrome.as_ref())
            .and_then(|chrome| chrome.skin().area("MainSkinArea"))
        else {
            return;
        };
        let largest = |name: &str, least: i32| {
            area.attrs
                .int(name)
                .filter(|value| *value > 0)
                .map_or(0, |value| window_length(value).max(least))
        };
        let (max_width, max_height) = (
            largest("RvpMaxWidth", width),
            largest("RvpMaxHeight", height),
        );
        if *self.max_width() != max_width {
            self.as_mut().set_max_width(max_width);
        }
        if *self.max_height() != max_height {
            self.as_mut().set_max_height(max_height);
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
            // M9: the window holds only the video and the controller.
            || self.controller_without_skin()
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

    /// The misc info line (Scroll Lock) and the OSD's look, once a second
    /// and after a change.
    pub(super) fn tick_misc_info(mut self: Pin<&mut Self>) {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let look = QString::from(&core.app.osd_look().to_string());
        let power = if core.app.settings.osd.misc.power {
            battery()
        } else {
            None
        };
        let line = QString::from(&core.app.misc_info(fullscreen, &core.clock, power));
        // The presets' power condition (M14).
        if core
            .app
            .settings
            .presets
            .list
            .iter()
            .any(|preset| preset.conditions.power.is_some())
        {
            core.app.set_on_battery(on_battery());
        }
        if *self.osd_look() != look {
            self.as_mut().set_osd_look(look);
        }
        if *self.misc_info() != line {
            self.set_misc_info(line);
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
    /// for the bar that shows.
    pub(super) fn show_bars(mut self: Pin<&mut Self>, shown: bool) {
        // An on-screen controller takes the bars' place (M9).
        let shown = shown && !self.controller_active();
        if *self.chrome_shown() != shown {
            self.as_mut().set_chrome_shown(shown);
        }
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let room = if shown && fullscreen && core.app.settings.skins.fullscreen_resize {
            core.look.bar
        } else {
            None
        };
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        if chrome.set_video_room(room) {
            let video = chrome.video_rect();
            self.set_video_rect(to_window(video));
        }
    }

    /// The skin's spectrum bars move on: the bands' levels in, the
    /// `Visual` regions repainted.
    pub(crate) fn tick_spectrum(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some((levels, peaks)) = core.app.spectrum_bars(std::time::Instant::now()) else {
            return;
        };
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        chrome.data.visual = levels;
        chrome.data.visual_peaks = peaks;
        let damage = chrome.refresh();
        self.emit_damage(&damage, false);
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

    pub(crate) fn system_info_json(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        let mpv = core.engine.mpv();
        let text = |name: &str| {
            mpv.get_property::<String>(name)
                .ok()
                .filter(|text| !text.is_empty())
        };
        let int = |name: &str| mpv.get_property::<i64>(name).ok();
        let float = |name: &str| {
            mpv.get_property::<f64>(name)
                .ok()
                .filter(|value| value.is_finite())
        };
        let joined = |parts: Vec<Option<String>>| {
            let parts: Vec<String> = parts.into_iter().flatten().collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        };
        let kbps = |name: &str| float(name).map(|bits| format!("{:.0} kbit/s", bits / 1000.0));
        let rate = |value: Option<f64>, unit: &str| value.map(|value| format!("{value:.3} {unit}"));
        let size = match (int("video-params/w"), int("video-params/h")) {
            (Some(width), Some(height)) => Some(format!("{width}x{height}")),
            _ => None,
        };
        let file_size = int("file-size").map(|bytes| {
            // A size in MiB for reading.
            #[allow(clippy::cast_precision_loss)]
            let mib = bytes as f64 / 1_048_576.0;
            format!("{mib:.1} MiB")
        });
        let time = |name: &str| float(name).map(rvp_app::view::format_time);
        let sections = serde_json::json!([
            {
                "title": "File",
                "rows": [
                    ["Path", text("path")],
                    ["Title", text("media-title")],
                    ["Format", text("file-format")],
                    ["Size", file_size],
                    ["Duration", time("duration")],
                    ["Position", time("time-pos")],
                    ["Chapters", int("chapters").map(|count| count.to_string())],
                ],
            },
            {
                "title": "Video",
                "rows": [
                    ["Codec", text("video-codec")],
                    ["Size", size],
                    ["Pixel format", text("video-params/pixelformat")],
                    ["Colour", joined(vec![
                        text("video-params/colormatrix"),
                        text("video-params/primaries"),
                        text("video-params/gamma"),
                        text("video-params/colorlevels"),
                    ])],
                    ["Frame rate", rate(float("container-fps"), "fps")],
                    ["Output rate", rate(float("estimated-vf-fps"), "fps")],
                    ["Bitrate", kbps("video-bitrate")],
                    ["Hardware decoding", text("hwdec-current")],
                    ["Filters", text("vf")],
                    ["Dropped frames", joined(vec![
                        int("frame-drop-count").map(|count| format!("{count} output")),
                        int("decoder-frame-drop-count").map(|count| format!("{count} decoder")),
                    ])],
                ],
            },
            {
                "title": "Audio",
                "rows": [
                    ["Codec", text("audio-codec")],
                    ["Format", joined(vec![
                        int("audio-params/samplerate").map(|rate| format!("{rate} Hz")),
                        text("audio-params/hr-channels"),
                        text("audio-params/format"),
                    ])],
                    ["Bitrate", kbps("audio-bitrate")],
                    ["Output", joined(vec![text("current-ao"), text("audio-device")])],
                    ["Filters", text("af")],
                ],
            },
            {
                "title": "Rendering",
                "rows": [
                    ["Video output", text("current-vo")],
                    ["Display rate", rate(float("display-fps"), "Hz")],
                    ["Video sync", text("video-sync")],
                    ["Scaler", joined(vec![text("scale"), text("dscale"), text("cscale")])],
                    ["Shaders", text("glsl-shaders")],
                ],
            },
            system_section(text("mpv-version").as_deref(), text("ffmpeg-version").as_deref()),
        ]);
        QString::from(&sections.to_string())
    }

    pub(crate) fn window_attention(mut self: Pin<&mut Self>, kind: i32, on: bool) {
        use rvp_app::general::Attention;
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let attention = match kind {
            0 => Attention::Minimized(on),
            1 => Attention::Focus(on),
            _ => Attention::ScreenLocked(on),
        };
        core.app.window_attention(core.engine, attention);
    }

    pub(crate) fn drop_choice(mut self: Pin<&mut Self>, choice: i32, paths: &QString) {
        match choice {
            0 | 1 => self.open_files(paths, choice == 1),
            _ => {
                for path in paths.to_string().lines().filter(|line| !line.is_empty()) {
                    self.as_mut().load_subtitle(&QString::from(path));
                }
            }
        }
    }

    /// A file started: Windows' jump list or the desktop's recent files
    /// (Linux) list it, and error messages go when the General page says
    /// so (M14).
    pub(super) fn file_started(mut self: Pin<&mut Self>) {
        let Some(core) = self.core() else {
            return;
        };
        let general = &core.app.settings.general;
        if let Some(path) = core.app.model.loaded_path()
            && general.taskbar
            && core.app.settings.open.recent
            && !path.contains("://")
        {
            // Windows' jump list (nothing elsewhere).
            crate::ffi::bridge::rvp_taskbar_recent(path);
            // A file write, so off the window's thread.
            #[cfg(target_os = "linux")]
            {
                let path = std::path::PathBuf::from(path);
                std::thread::spawn(move || {
                    let added = rvp_platform::recent::user_file().and_then(|list| {
                        rvp_platform::recent::add(&list, &path, std::time::SystemTime::now())
                    });
                    if let Err(error) = added {
                        rvp_app::log::report(rvp_app::log::Level::Warning, &error);
                    }
                });
            }
        }
        if general.hide_errors_when_playing && core.app.model.media_loaded() {
            self.as_mut().set_message(QString::default());
        }
    }

    /// Playback closed (RVP idles): "Default window size on closing file"
    /// gives the window its starting size again (M14).
    pub(super) fn file_closed(mut self: Pin<&mut Self>) {
        let fullscreen = *self.fullscreen();
        let (width, height) = (*self.restore_width(), *self.restore_height());
        let Some(core) = self.core() else {
            return;
        };
        let idle = core.app.model.flag(rvp_engine::PropertyId::IdleActive) == Some(true);
        if idle
            && !fullscreen
            && !core.maximized
            && core.app.settings.general.default_size_on_close
            && width > 0
            && height > 0
        {
            self.as_mut()
                .window_request(&WindowRequest::Size { width, height });
        }
    }

    /// The taskbar's progress, once a second: Windows' taskbar button, the
    /// launcher on Linux (the desktop integration's).
    pub(super) fn tick_taskbar(self: Pin<&mut Self>) {
        let Some(core) = self.core() else {
            return;
        };
        let model = &core.app.model;
        let state = if !model.media_loaded() {
            0
        } else if model.playing() {
            1
        } else {
            2
        };
        let progress = match (
            model.double(rvp_engine::PropertyId::TimePos),
            model.double(rvp_engine::PropertyId::Duration),
        ) {
            (Some(time), Some(duration)) if duration > 0.0 => time / duration,
            _ => -1.0,
        };
        // Windows' taskbar (nothing elsewhere).
        crate::ffi::bridge::rvp_taskbar_state(state, progress);
        // The General page's taskbar option, which Windows' taskbar code
        // follows too: off hides the launcher's progress.
        if let Some(desktop) = &core.desktop {
            if core.app.settings.general.taskbar {
                desktop.progress(state, progress);
            } else {
                desktop.progress(0, -1.0);
            }
        }
    }

    /// After a settings change: the media controls, the taskbar and the
    /// start with the session follow the General page.
    pub(super) fn apply_general(mut self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let general = core.app.settings.general.clone();
        // Linux keeps its desktop thread (the screen kept on, the
        // launcher's progress, the lock) and turns MPRIS alone on or off.
        #[cfg(target_os = "linux")]
        if let Some(desktop) = core.desktop.as_mut() {
            desktop.set_media_controls(general.media_controls);
        }
        let wanted = general.media_controls || cfg!(target_os = "linux");
        if wanted && core.desktop.is_none() && core.look.startup.is_some() {
            match crate::desktop::Integration::start(
                thread,
                crate::ffi::bridge::rvp_main_window_id(),
                general.media_controls,
            ) {
                Ok(desktop) => core.desktop = Some(desktop),
                Err(error) => {
                    rvp_app::log::report(rvp_app::log::Level::Warning, &error);
                }
            }
        } else if !wanted {
            core.desktop = None;
        }
        crate::ffi::bridge::rvp_taskbar_start(general.taskbar);
        crate::ffi::bridge::rvp_global_media_keys(core.app.settings.input.global_media_keys);
        if core
            .look
            .startup
            .is_some_and(|applied| applied != general.run_at_startup)
        {
            core.look.startup = Some(general.run_at_startup);
            if let Err(error) = crate::startup::apply(general.run_at_startup) {
                self.show_message(&format!("Run at system startup: {error}"));
            }
        }
    }

    /// The skin's themes, its audio skin and the theme to show, the
    /// windows' opacity, the General and Accessibility pages' options.
    pub(super) fn sync_look(mut self: Pin<&mut Self>) {
        self.as_mut().sync_skin();
        self.as_mut().sync_opacity();
        self.as_mut().sync_general_look();
        self.sync_accessibility();
    }

    /// The Accessibility pages' options for QML and the controller.
    pub(super) fn sync_accessibility(mut self: Pin<&mut Self>) {
        let Some(core) = self.core() else {
            return;
        };
        let accessibility = &core.app.settings.accessibility;
        let stick = match accessibility.xinput_mouse.as_str() {
            "left" => 1,
            "right" => 2,
            _ => 0,
        };
        crate::ffi::bridge::rvp_xinput_start(
            accessibility.xinput,
            accessibility.xinput_focus_only,
            stick,
        );
        let look = QString::from(&accessibility.look().to_string());
        if *self.accessibility_look() != look {
            self.as_mut().set_accessibility_look(look);
        }
    }

    /// What the model says now (M14): whether a video shows, for "Auto
    /// hide main skin", and the text subtitle, for Accessibility.
    pub(super) fn follow_model(
        mut self: Pin<&mut Self>,
        has_video: bool,
        changes: rvp_engine::Changes,
    ) {
        self.as_mut().show_video(has_video);
        self.follow_subtitle(changes.contains(rvp_engine::PropertyId::SubText));
    }

    /// The text subtitle showing now, when it changed.
    pub(super) fn follow_subtitle(mut self: Pin<&mut Self>, changed: bool) {
        if !changed {
            return;
        }
        let text = self
            .core()
            .and_then(|core| core.app.model.text(rvp_engine::PropertyId::SubText))
            .map(QString::from)
            .unwrap_or_default();
        if *self.sub_text() != text {
            self.as_mut().set_sub_text(text);
        }
    }

    pub(crate) fn speak(&self, text: &QString) {
        let Some(core) = self.core() else {
            return;
        };
        let accessibility = &core.app.settings.accessibility;
        crate::ffi::bridge::rvp_tts_speak(
            &text.to_string(),
            &accessibility.voice,
            &accessibility.output,
            i32::from(accessibility.rate),
            i32::from(accessibility.volume),
        );
    }

    // QML invokables, so methods even though they ask the system only.
    #[allow(clippy::unused_self)]
    pub(crate) fn tts_voices_json(&self) -> QString {
        QString::from(&crate::ffi::bridge::rvp_tts_voices())
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn tts_outputs_json(&self) -> QString {
        QString::from(&crate::ffi::bridge::rvp_tts_outputs())
    }

    pub(crate) fn touch_swipe(mut self: Pin<&mut Self>, dx: f64, dy: f64) {
        use rvp_app::accessibility::{SwipeEffect, swipe_effect};
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = match swipe_effect(&core.app.settings.accessibility.swipe, dx, dy) {
            Some(SwipeEffect::Seek(seconds)) => core.app.seek_by(core.engine, seconds),
            Some(SwipeEffect::Volume(steps)) => core.app.volume_steps(core.engine, steps),
            None => None,
        };
        if let Some(request) = request {
            self.handle_request(request);
        }
    }

    pub(crate) fn associations_json(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        let settings = &core.app.settings.associations;
        // Linux: the types the desktop entry in effect lists.
        let (listed, globs) = if cfg!(windows) {
            (std::collections::BTreeSet::new(), String::new())
        } else {
            let entry = linux_desktop_entries()
                .into_iter()
                .find_map(|path| std::fs::read_to_string(path).ok())
                .unwrap_or_default();
            (rvp_app::registration::listed_types(&entry), mime_globs())
        };
        let rows: Vec<serde_json::Value> = settings
            .types()
            .iter()
            .map(|file_type| {
                let registered = if cfg!(windows) {
                    let key = format!(r"Software\Classes\RVP.{}", file_type.extension);
                    crate::ffi::bridge::rvp_registry_has_key(&key)
                } else {
                    rvp_app::registration::listed(&listed, file_type, &globs)
                };
                serde_json::json!({
                    "extension": file_type.extension,
                    "kind": file_type.kind,
                    "description": settings.description(file_type),
                    "on": !settings.off.contains(&file_type.extension),
                    "registered": registered,
                    "extra": settings.extra.contains_key(&file_type.extension),
                })
            })
            .collect();
        QString::from(&serde_json::Value::Array(rows).to_string())
    }

    pub(crate) fn apply_associations(self: Pin<&mut Self>) -> QString {
        use crate::ffi::bridge::{
            rvp_associations_changed, rvp_registry_delete_key, rvp_registry_delete_value,
            rvp_registry_set,
        };
        use rvp_app::registration::{RegistryChange, registration};
        let Some(core) = self.core() else {
            return QString::default();
        };
        if !cfg!(windows) {
            return QString::from(&apply_linux_associations(&core.app.settings.associations));
        }
        let Ok(exe) = std::env::current_exe() else {
            return QString::from("The program's path is unknown.");
        };
        let changes = registration(&core.app.settings.associations, &exe.display().to_string());
        let failed = changes
            .iter()
            .filter(|change| !match change {
                RegistryChange::Set { key, name, value } => rvp_registry_set(key, name, value),
                RegistryChange::DeleteValue { key, name } => rvp_registry_delete_value(key, name),
                RegistryChange::DeleteKey { key } => rvp_registry_delete_key(key),
            })
            .count();
        rvp_associations_changed();
        QString::from(&if failed == 0 {
            format!(
                "Applied: {} registry changes for the current user.",
                changes.len()
            )
        } else {
            format!("{failed} of {} registry changes failed.", changes.len())
        })
    }

    // A QML invokable, so a method even though it asks the system only.
    #[allow(clippy::unused_self)]
    pub(crate) fn use_as_screensaver(&self) -> QString {
        if !cfg!(windows) {
            return QString::from("RVP is a screen saver on Windows only.");
        }
        let Some(saver) = std::env::current_exe()
            .ok()
            .map(|exe| exe.with_file_name("rvp.scr"))
            .filter(|saver| saver.is_file())
        else {
            return QString::from("rvp.scr is missing beside rvp.exe.");
        };
        let desktop = r"Control Panel\Desktop";
        let saver = saver.display().to_string();
        if !(crate::ffi::bridge::rvp_registry_set(desktop, "SCRNSAVE.EXE", &saver)
            && crate::ffi::bridge::rvp_registry_set(desktop, "ScreenSaveActive", "1"))
        {
            return QString::from("Windows' screen saver setting could not be changed.");
        }
        let opened = std::process::Command::new("control.exe")
            .arg("desk.cpl,,@screensaver")
            .spawn()
            .is_ok();
        QString::from(if opened {
            "RVP is the screen saver; Windows' settings set its wait."
        } else {
            "RVP is the screen saver."
        })
    }

    // A QML invokable, so a method even though it asks the system only.
    #[allow(clippy::unused_self)]
    pub(crate) fn rebuild_icon_cache(&self) -> bool {
        if !cfg!(windows) {
            return false;
        }
        let system = std::env::var_os("SystemRoot").map_or_else(
            || std::path::PathBuf::from(r"C:\Windows"),
            std::path::PathBuf::from,
        );
        let mut command = std::process::Command::new(system.join("System32").join("ie4uinit.exe"));
        command.arg("-show");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: no console flashes up.
            command.creation_flags(0x0800_0000);
        }
        command.spawn().is_ok()
    }

    /// The General page's window options for QML.
    pub(super) fn sync_general_look(mut self: Pin<&mut Self>) {
        let Some(core) = self.core() else {
            return;
        };
        let general = &core.app.settings.general;
        let look = serde_json::json!({
            "tray": general.tray,
            "hideWithOwner": general.hide_windows_minimized,
            "resizeCenter": general.resize_from_center,
            "snap": general.snap,
            "dialogOnTop": general.dialog.on_top,
            "dialogFont": general.dialog.font,
            "dialogFontSize": general.dialog.font_size,
            "noFocusAtStart": general.no_focus_at_start,
            "startCenter": general.start.position == "center",
            "language": general.language,
            "startFiles": core.pending.as_ref().is_some_and(|request| !request.files.is_empty()),
        });
        let look = QString::from(&look.to_string());
        if *self.general_look() != look {
            self.as_mut().set_general_look(look);
        }
    }

    pub(crate) fn settings_folder_url(&self, kind: &QString) -> QString {
        let Some(core) = self.core() else {
            return QString::default();
        };
        let folder = match kind.to_string().as_str() {
            "skins" => core.app.skins_folder(),
            "logos" => core.app.logos_folder(),
            "menus" => core.app.menus_folder(),
            // M9: the user's scripts for mpv's on-screen controllers.
            "mpv" => core.app.mpv_folder(),
            "log" => core.app.log_file().parent().map_or_else(
                || core.app.state_dir().to_path_buf(),
                std::path::Path::to_path_buf,
            ),
            _ => return QString::default(),
        };
        if let Err(error) = std::fs::create_dir_all(&folder) {
            rvp_app::log::report(
                rvp_app::log::Level::Warning,
                &format!("cannot make {}: {error}", folder.display()),
            );
            return QString::default();
        }
        QString::from(&file_url(&folder))
    }

    // A QML invokable, so a method even though it reads only the log.
    #[allow(clippy::unused_self)]
    pub(crate) fn log_recent(&self) -> QString {
        QString::from(&rvp_app::log::recent())
    }

    pub(crate) fn calibration_status(&self) -> QString {
        self.core().map_or_else(QString::default, |core| {
            QString::from(&core.app.calibration_status().to_string())
        })
    }

    pub(crate) fn calibrate_now(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.calibrate_now(core.engine);
        }
    }

    pub(crate) fn cancel_calibration(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.cancel_calibration();
        }
    }

    // A QML invokable is a method of its object; the beat is the thread's.
    #[allow(clippy::unused_self)]
    pub(crate) fn window_beat(&self) {
        crate::video::beat();
    }

    pub(crate) fn tensorrt_status(&self) -> QString {
        self.core().map_or_else(QString::default, |core| {
            QString::from(&core.app.tensorrt_status().to_string())
        })
    }

    pub(crate) fn tensorrt_install(mut self: Pin<&mut Self>) {
        let request = self
            .as_mut()
            .core_mut()
            .and_then(|core| core.app.tensorrt_install());
        if let Some(request) = request {
            self.handle_request(request);
        }
    }

    pub(crate) fn tensorrt_cancel(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.tensorrt_cancel();
        }
    }

    pub(crate) fn tensorrt_remove(mut self: Pin<&mut Self>) -> QString {
        let result = self
            .as_mut()
            .core_mut()
            .map_or(Ok(()), |core| core.app.tensorrt_remove(core.engine));
        QString::from(&result.err().unwrap_or_default())
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

/// The battery's charge for the misc info: Windows' power status, or
/// Linux's power supplies.
fn battery() -> Option<String> {
    #[cfg(windows)]
    {
        use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        let mut status = SYSTEM_POWER_STATUS::default();
        // SAFETY: FFI point. GetSystemPowerStatus fills the structure it is
        // given, which lives on this stack frame for the call; a failure
        // leaves it zeroed and is answered with no battery.
        #[allow(unsafe_code)]
        let read = unsafe { GetSystemPowerStatus(&raw mut status) };
        // 128: no system battery; 255: unknown.
        if read.is_err() || status.BatteryFlag & 128 != 0 || status.BatteryLifePercent > 100 {
            return None;
        }
        Some(format!(
            "battery {}%{}",
            status.BatteryLifePercent,
            if status.ACLineStatus == 1 {
                " (charging)"
            } else {
                ""
            }
        ))
    }
    #[cfg(not(windows))]
    {
        rvp_app::osd::battery_from_sysfs(std::path::Path::new("/sys/class/power_supply"))
    }
}

/// The Playback/System Info window's System section: RVP's, mpv's and
/// FFmpeg's versions, the platform, the processor and the battery (Qt's
/// own come from QML).
fn system_section(mpv: Option<&str>, ffmpeg: Option<&str>) -> serde_json::Value {
    let threads = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    serde_json::json!({
        "title": "System",
        "rows": [
            ["RVP", Some(env!("CARGO_PKG_VERSION"))],
            ["mpv", mpv],
            ["FFmpeg", ffmpeg],
            ["Platform", Some(format!("{} {}", std::env::consts::OS, std::env::consts::ARCH))],
            ["Processor threads", Some(threads.to_string())],
            ["Battery", battery()],
        ],
    })
}

/// Whether the system runs on its battery (`None`: it has none, or cannot
/// tell), for the presets' conditions.
fn on_battery() -> Option<bool> {
    #[cfg(windows)]
    {
        use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        let mut status = SYSTEM_POWER_STATUS::default();
        // SAFETY: FFI point. GetSystemPowerStatus fills the structure it is
        // given, which lives on this stack frame for the call; a failure
        // is answered with no battery.
        #[allow(unsafe_code)]
        let read = unsafe { GetSystemPowerStatus(&raw mut status) };
        // 128: no system battery; ACLineStatus 0 offline, 1 online.
        if read.is_err() || status.BatteryFlag & 128 != 0 || status.ACLineStatus > 1 {
            return None;
        }
        Some(status.ACLineStatus == 0)
    }
    #[cfg(not(windows))]
    {
        rvp_app::osd::on_battery_from_sysfs(std::path::Path::new("/sys/class/power_supply"))
    }
}

/// The desktop entries named `rvp.desktop` in the order the desktop takes
/// them: the user's, then the system's (`XDG_DATA_DIRS`).
fn linux_desktop_entries() -> Vec<std::path::PathBuf> {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".local/share"))
        });
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    home.into_iter()
        .chain(system.split(':').map(std::path::PathBuf::from))
        .map(|folder| folder.join("applications/rvp.desktop"))
        .collect()
}

/// shared-mime-info's file patterns (empty without it).
fn mime_globs() -> String {
    std::fs::read_to_string("/usr/share/mime/globs2").unwrap_or_default()
}

/// Apply to System on Linux: the user's `rvp.desktop` lists the types that
/// are on (it takes the place of the packages' entry); the desktop's own
/// settings choose the defaults.
fn apply_linux_associations(settings: &rvp_app::registration::AssociationSettings) -> String {
    // An AppImage runs from a mount that changes: its own file starts it.
    let Some(exe) = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
    else {
        return "The program's path is unknown.".to_owned();
    };
    let Some(entry) = linux_desktop_entries().into_iter().next() else {
        return "The home folder is unknown.".to_owned();
    };
    let text =
        rvp_app::registration::desktop_entry(settings, &exe.display().to_string(), &mime_globs());
    let written = entry
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&entry, &text));
    if let Err(error) = written {
        return format!("{}: {error}", entry.display());
    }
    // The desktop's cache of handlers, where its tool is installed.
    if let Some(folder) = entry.parent() {
        drop(
            std::process::Command::new("update-desktop-database")
                .arg(folder)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status(),
        );
    }
    format!(
        "Applied: {} lists {} types for the current user; the desktop's settings choose the defaults.",
        entry.display(),
        rvp_app::registration::listed_types(&text).len()
    )
}

/// What a skin with windows of its own says when it is chosen while Qt
/// draws on Wayland (the session is Wayland's and RVP did not start
/// through XWayland); `None` otherwise.
#[cfg(target_os = "linux")]
fn wayland_windows_note(skin: &rvp_skin::Skin) -> Option<&'static str> {
    let wayland = std::env::var("QT_QPA_PLATFORM").map_or_else(
        |_| std::env::var_os("WAYLAND_DISPLAY").is_some_and(|display| !display.is_empty()),
        |platform| platform.starts_with("wayland"),
    );
    (wayland && !skin.extra_windows().is_empty()).then_some(
        "This skin's windows need X11: RVP starts through XWayland for it from the next start",
    )
}
