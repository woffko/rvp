//! M9's other skin windows: the `RvpWindow='1'` areas of the skin RVP
//! built from a classic Winamp skin (`rvp_skin::winamp::build`: its main
//! window, equalizer and playlist) or from a VLC theme's windows beside the
//! one with the video (`rvp_skin::vlc::build`). Each is a `Chrome` of its own in
//! `Mode::Extra`, painted whole (`SkinView` surfaces from 10 on), with its
//! own pointer; QML places them docked to the main window
//! (`SkinWindow.qml`) and moves them with it. A window's toggle command
//! (`RVPWIN:`) shows and hides it; the main window's `RVP:` texts and
//! states reach them all.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt_lib::QString;
use rvp_app::chrome::{Chrome, Frame, Mode};
use rvp_app::pointer::{Context, Effect, Modifiers, Pointer, WINDOW_COMMAND};
use rvp_app::view;
use serde_json::json;

use super::{button, chrome_pixel, request};
use crate::player_qobject::qobject;

/// The first `SkinView` surface of the extra windows.
pub const SURFACE: i32 = 10;

/// The frames the extra windows' painters draw, by window.
static FRAMES: Mutex<Vec<Option<Arc<Frame>>>> = Mutex::new(Vec::new());

/// The frame of extra window `index`.
pub fn frame(index: usize) -> Option<Arc<Frame>> {
    FRAMES
        .lock()
        .ok()
        .and_then(|frames| frames.get(index).cloned().flatten())
}

/// One of the skin's other windows.
pub(super) struct Extra {
    area: &'static str,
    chrome: Chrome,
    pointer: Pointer,
    shown: bool,
    toggle: Option<&'static str>,
    /// Its rows show the playlist (it gets the albums' data).
    lists: bool,
    resizable: bool,
}

impl Extra {
    /// The window's id in a VLC theme (its toggle's `RVPWIN:` name), when
    /// it shows.
    pub(super) fn shown_id(&self) -> Option<&'static str> {
        self.toggle
            .filter(|_| self.shown)
            .and_then(|toggle| toggle.strip_prefix(WINDOW_COMMAND))
    }
}

impl qobject::Player {
    /// The skin's other windows, made again for a new skin; a window keeps
    /// whether it showed when the new skin has one of the same area.
    pub(super) fn rebuild_extras(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(skin) = core.chrome.as_ref().map(Chrome::skin) else {
            return;
        };
        let before: Vec<(&'static str, bool)> = core
            .extras
            .iter()
            .map(|extra| (extra.area, extra.shown))
            .collect();
        let mut extras = Vec::new();
        let mut windows = Vec::new();
        for window in skin.extra_windows() {
            let mut chrome = Chrome::new(skin);
            if chrome
                .resize(Mode::Extra(window.area), window.width, window.height)
                .is_err()
            {
                continue;
            }
            let lists = chrome.list_geometry().is_some();
            let mask = shape(&chrome, window.area);
            let shown = before
                .iter()
                .find(|(area, _)| *area == window.area)
                .map_or(window.shown, |(_, shown)| *shown);
            windows.push(json!({
                "area": window.area,
                "width": window.width,
                "height": window.height,
                "dock": window.dock,
                "shown": shown,
                "step": window.step.map(|(w, h)| [w, h]),
                "offset": [window.offset.0, window.offset.1],
                "min": [window.min.0, window.min.1],
                "max": [window.max.0, window.max.1],
                "mask": mask,
                "region": window
                    .region
                    .iter()
                    .map(|polygon| {
                        polygon
                            .iter()
                            .map(|(x, y)| format!("{x},{y}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(";"),
            }));
            extras.push(Extra {
                area: window.area,
                chrome,
                pointer: Pointer::default(),
                shown,
                toggle: window.toggle,
                lists,
                resizable: window.step.is_some() || window.min != window.max,
            });
        }
        core.extras = extras;
        if let Ok(mut frames) = FRAMES.lock() {
            *frames = vec![None; core.extras.len()];
        }
        self.as_mut().set_extra_windows(QString::from(
            &serde_json::Value::Array(windows).to_string(),
        ));
        self.sync_extras(true);
    }

    /// Recomputes the shown extra windows from the main window's data;
    /// those that changed repaint.
    pub(super) fn sync_extras(mut self: Pin<&mut Self>, force: bool) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(main) = core.chrome.as_ref() else {
            return;
        };
        if core.extras.is_empty() {
            return;
        }
        let shown: Vec<&'static str> = core
            .extras
            .iter()
            .filter(|extra| extra.shown)
            .filter_map(|extra| extra.toggle)
            .collect();
        let mut damaged = Vec::new();
        for (index, extra) in core.extras.iter_mut().enumerate() {
            if !extra.shown && !force {
                continue;
            }
            let mut data = rvp_skin::DataModel {
                texts: main.data.texts.clone(),
                sliders: main.data.sliders.clone(),
                marks: main.data.marks.clone(),
                visual: main.data.visual.clone(),
                visual_peaks: main.data.visual_peaks.clone(),
                ..rvp_skin::DataModel::default()
            };
            if extra.lists {
                data.playlist = main.data.playlist.clone();
            }
            if data != extra.chrome.data {
                extra.chrome.data = data;
            }
            let state = &mut extra.chrome.state;
            view::apply_playback_state(state, &core.app.model);
            // The main window's states: a classic skin's (`RVP:`) and a VLC
            // theme's conditions (`VLCX:`).
            let shared = |flag: &String| flag.starts_with("RVP:") || flag.starts_with("VLCX:");
            state
                .flags
                .retain(|flag| !shared(flag) && !flag.starts_with(WINDOW_COMMAND));
            state
                .flags
                .extend(main.state.flags.iter().filter(|flag| shared(flag)).cloned());
            state
                .flags
                .extend(shown.iter().map(|toggle| (*toggle).to_owned()));
            let toggles = extra.chrome.toggle_commands();
            core.app.toggle_flags(&mut extra.chrome.state, toggles);
            let damage = extra.chrome.refresh();
            if damage.is_empty() && !force {
                continue;
            }
            if let Ok(mut frames) = FRAMES.lock()
                && let Some(slot) = frames.get_mut(index)
            {
                *slot = Some(extra.chrome.frame());
            }
            damaged.push(index);
        }
        for index in damaged {
            self.as_mut()
                .extra_damaged(i32::try_from(index).unwrap_or(0));
        }
    }

    /// Shows or hides the window whose toggle is `command` (`RVPWIN:`);
    /// `RVPWIN:ID:show` and `:hide` only show or hide (VLC's).
    pub(super) fn toggle_extra(mut self: Pin<&mut Self>, command: &str) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (toggle, wanted) = match command.rsplit_once(':') {
            Some((toggle, "show")) => (toggle, Some(true)),
            Some((toggle, "hide")) => (toggle, Some(false)),
            _ => (command, None),
        };
        let Some(index) = core
            .extras
            .iter()
            .position(|extra| extra.toggle == Some(toggle))
        else {
            return;
        };
        let extra = &mut core.extras[index];
        extra.shown = wanted.unwrap_or(!extra.shown);
        let shown = extra.shown;
        let windows = self.extra_windows().to_string();
        if let Ok(mut list) = serde_json::from_str::<serde_json::Value>(&windows)
            && let Some(window) = list.get_mut(index)
        {
            window["shown"] = json!(shown);
            self.as_mut()
                .set_extra_windows(QString::from(&list.to_string()));
        }
        self.as_mut().sync_extras(true);
        self.sync_chrome();
    }

    /// A window's new size (the resizable playlist): its layout follows.
    pub(crate) fn extra_resized(mut self: Pin<&mut Self>, index: i32, width: i32, height: i32) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(extra) = usize::try_from(index)
            .ok()
            .and_then(|i| core.extras.get_mut(i))
        else {
            return;
        };
        if extra.chrome.size() == (width, height) {
            return;
        }
        if extra
            .chrome
            .resize(Mode::Extra(extra.area), width, height)
            .is_err()
        {
            return;
        }
        let mask = shape(&extra.chrome, extra.area);
        let windows = self.extra_windows().to_string();
        if let Ok(mut list) = serde_json::from_str::<serde_json::Value>(&windows)
            && let Some(window) = usize::try_from(index).ok().and_then(|i| list.get_mut(i))
            && window["mask"] != json!(mask)
        {
            window["mask"] = json!(mask);
            self.as_mut()
                .set_extra_windows(QString::from(&list.to_string()));
        }
        self.sync_extras(true);
    }

    /// The window system closed a window: it stays hidden.
    pub(crate) fn extra_closed(mut self: Pin<&mut Self>, index: i32) {
        let toggle = self.core().and_then(|core| {
            let extra = core.extras.get(usize::try_from(index).ok()?)?;
            extra.shown.then_some(extra.toggle).flatten()
        });
        if let Some(toggle) = toggle {
            self.as_mut().toggle_extra(toggle);
        }
    }

    fn extra_context(extra: &Extra, modifiers: Modifiers) -> Context {
        let (width, height) = extra.chrome.size();
        Context {
            width,
            height,
            movable: true,
            chrome_shown: true,
            // Only the resizable windows take their edges.
            panel: !extra.resizable,
            turn_view: false,
            modifiers,
            options: rvp_app::pointer::PointerOptions::default(),
        }
    }

    /// `phase`: 0 press, 1 double click, 2 release (as `pointer_event`).
    fn extra_pointer(
        mut self: Pin<&mut Self>,
        index: i32,
        x: f64,
        y: f64,
        button_value: i32,
        phase: u8,
        modifiers: Modifiers,
    ) {
        let Some(button) = button(button_value) else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(extra) = usize::try_from(index)
            .ok()
            .and_then(|i| core.extras.get_mut(i))
        else {
            return;
        };
        let ctx = Self::extra_context(extra, modifiers);
        let (x, y) = (chrome_pixel(x, 1.0), chrome_pixel(y, 1.0));
        let bindings = &core.app.bindings;
        let effects = match phase {
            0 => extra.pointer.press(&mut extra.chrome, ctx, x, y, button),
            1 => extra
                .pointer
                .double_click(&mut extra.chrome, bindings, ctx, x, y, button),
            _ => extra
                .pointer
                .release(&mut extra.chrome, bindings, ctx, x, y, button),
        };
        self.as_mut().apply_extra_effects(index, &effects);
        self.as_mut().sync_extras(false);
        self.sync_chrome();
    }

    /// Moves and resizes go to the window's own QML; the rest is as in the
    /// main window.
    fn apply_extra_effects(mut self: Pin<&mut Self>, index: i32, effects: &[Effect]) {
        for effect in effects {
            match effect {
                Effect::StartMove => self.as_mut().ui_request(request::EXTRA_MOVE, index),
                Effect::StartResize(edges) => {
                    self.as_mut()
                        .ui_request(request::EXTRA_RESIZE, index * 100 + edges);
                }
                Effect::Focus(_) => {}
                other => self.as_mut().apply_effects(std::slice::from_ref(other)),
            }
        }
    }

    pub(crate) fn extra_press(
        self: Pin<&mut Self>,
        index: i32,
        x: f64,
        y: f64,
        button_value: i32,
        modifiers: i32,
    ) {
        let modifiers = super::modifiers(modifiers);
        self.extra_pointer(index, x, y, button_value, 0, modifiers);
    }

    pub(crate) fn extra_double(
        self: Pin<&mut Self>,
        index: i32,
        x: f64,
        y: f64,
        button_value: i32,
    ) {
        self.extra_pointer(index, x, y, button_value, 1, Modifiers::default());
    }

    pub(crate) fn extra_release(
        self: Pin<&mut Self>,
        index: i32,
        x: f64,
        y: f64,
        button_value: i32,
    ) {
        self.extra_pointer(index, x, y, button_value, 2, Modifiers::default());
    }

    pub(crate) fn extra_move(mut self: Pin<&mut Self>, index: i32, x: f64, y: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(extra) = usize::try_from(index)
            .ok()
            .and_then(|i| core.extras.get_mut(i))
        else {
            return;
        };
        let ctx = Self::extra_context(extra, Modifiers::default());
        let effects = extra.pointer.moved(
            &mut extra.chrome,
            ctx,
            chrome_pixel(x, 1.0),
            chrome_pixel(y, 1.0),
        );
        let tooltip = core
            .vlc
            .tooltip(&core.app, extra.pointer.tooltip.unwrap_or(""));
        self.as_mut().set_extra_tooltip(QString::from(&tooltip));
        self.as_mut().apply_extra_effects(index, &effects);
        self.sync_extras(false);
    }

    pub(crate) fn extra_wheel(mut self: Pin<&mut Self>, index: i32, x: f64, y: f64, delta: i32) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(extra) = usize::try_from(index)
            .ok()
            .and_then(|i| core.extras.get_mut(i))
        else {
            return;
        };
        let ctx = Self::extra_context(extra, Modifiers::default());
        let effects = extra.pointer.wheel(
            &extra.chrome,
            &core.app.bindings,
            ctx,
            (chrome_pixel(x, 1.0), chrome_pixel(y, 1.0)),
            delta,
        );
        self.as_mut().apply_effects(&effects);
        self.sync_chrome();
    }

    pub(crate) fn extra_leave(mut self: Pin<&mut Self>, index: i32) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(extra) = usize::try_from(index)
            .ok()
            .and_then(|i| core.extras.get_mut(i))
        {
            extra.pointer.leave(&mut extra.chrome);
        }
        self.as_mut().set_extra_tooltip(QString::default());
        self.sync_extras(false);
    }

    /// Where a self-test can click the control bound to `command`: a point
    /// of it where the hit test finds it (VLC's buttons take only their
    /// opaque pixels), in the main window (`window` -1, its pixels) or in a
    /// shown other window (its index). Without one, `bound` says whether the
    /// skin has such a control at all.
    pub(crate) fn control_point(&self, command: &QString) -> QString {
        const STEPS: i32 = 16;
        let command = command.to_string();
        let Some(core) = self.core() else {
            return QString::from("{}");
        };
        let main = core
            .chrome
            .as_ref()
            .map(|chrome| (-1, chrome, super::zoom()));
        let others = core
            .extras
            .iter()
            .enumerate()
            .filter(|(_, extra)| extra.shown)
            .map(|(index, extra)| (i32::try_from(index).unwrap_or(-1), &extra.chrome, 1.0));
        for (window, chrome, zoom) in main.into_iter().chain(others) {
            let Some(placed) = chrome.layout() else {
                continue;
            };
            for node in placed
                .nodes
                .iter()
                .filter(|node| node.command() == Some(&command))
            {
                let rect = node.rect;
                // From the middle out: the window's edges resize it.
                let mut points: Vec<(i32, i32)> = (1..STEPS)
                    .flat_map(|row| (1..STEPS).map(move |column| (column, row)))
                    .collect();
                points.sort_by_key(|&(column, row)| {
                    (column * 2 - STEPS).pow(2) + (row * 2 - STEPS).pow(2)
                });
                for (column, row) in points {
                    let (x, y) = (
                        rect.x + rect.w * column / STEPS,
                        rect.y + rect.h * row / STEPS,
                    );
                    if let Some(rvp_skin::hit::Hit::Control {
                        command: Some(found),
                        ..
                    }) = chrome.hit(x, y)
                        && found == command
                    {
                        let value = json!({
                            "window": window,
                            "x": f64::from(x) * zoom,
                            "y": f64::from(y) * zoom,
                        });
                        return QString::from(&value.to_string());
                    }
                }
            }
        }
        let mut bound = false;
        if let Some(chrome) = &core.chrome {
            chrome.skin().root().walk(&mut |element| {
                bound |= element.attrs.get("CmdID") == Some(command.as_str());
            });
        }
        QString::from(&json!({ "bound": bound }).to_string())
    }
}

/// A window's shape when its area asks for one (`RvpShape='alpha'`: a
/// VLC theme's drawn pixels), as `rvp_skin::render::opaque_region` gives
/// it; empty: rectangular.
pub(super) fn shape(chrome: &Chrome, area: &str) -> String {
    let skin = chrome.skin();
    if skin.area(area).and_then(|area| area.attrs.get("RvpShape")) != Some("alpha") {
        return String::new();
    }
    rvp_skin::render::opaque_region(&chrome.frame().list, skin)
}

/// Rectangles `x,y,w,h;...` scaled by the skin's zoom.
fn scaled(rects: &str, zoom: f64) -> String {
    if (zoom - 1.0).abs() < f64::EPSILON {
        return rects.to_owned();
    }
    rects
        .split(';')
        .filter(|rect| !rect.is_empty())
        .map(|rect| {
            rect.split(',')
                .map(|value| {
                    // Window pixels: small.
                    #[allow(clippy::cast_possible_truncation)]
                    let scaled = (value.parse::<f64>().unwrap_or(0.0) * zoom).round() as i64;
                    scaled.to_string()
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join(";")
}

impl qobject::Player {
    /// The main window's shape after a new layout, once a tick (M9).
    pub(super) fn sync_window_mask(mut self: Pin<&mut Self>) {
        let overlay = *self.overlay();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if !std::mem::take(&mut core.mask_dirty) {
            return;
        }
        // The video alone fills the screen in fullscreen.
        let mask = match core.chrome.as_ref() {
            Some(chrome) if !overlay => scaled(&shape(chrome, "MainSkinArea"), super::zoom()),
            _ => String::new(),
        };
        super::WINDOW_MASK_LEN.store(
            mask.split(';').filter(|rect| !rect.is_empty()).count(),
            std::sync::atomic::Ordering::Relaxed,
        );
        if self.window_mask().to_string() != mask {
            self.set_window_mask(QString::from(&mask));
        }
    }
}

/// The extra windows' part of `Player::status` (the self-test reads it).
pub(super) fn status(core: &super::Core) -> Vec<(&'static str, String)> {
    let windows: Vec<serde_json::Value> = core
        .extras
        .iter()
        .map(|extra| {
            let (width, height) = extra.chrome.size();
            json!({
                "area": extra.area,
                "shown": extra.shown,
                "width": width,
                "height": height,
                "hover": extra.chrome.state.hover,
            })
        })
        .collect();
    vec![("extras", serde_json::Value::Array(windows).to_string())]
}
