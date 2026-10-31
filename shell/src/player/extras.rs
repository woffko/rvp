//! M9's other skin windows: the `RvpWindow='1'` areas of the skin RVP
//! built from a classic Winamp skin (`rvp_skin::winamp::build`): its main
//! window, equalizer and playlist. Each is a `Chrome` of its own in
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
            let shown = before
                .iter()
                .find(|(area, _)| *area == window.area)
                .map_or(window.toggle.is_none(), |(_, shown)| *shown);
            windows.push(json!({
                "area": window.area,
                "width": window.width,
                "height": window.height,
                "dock": window.dock,
                "shown": shown,
                "step": window.step.map(|(w, h)| [w, h]),
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
                resizable: window.step.is_some(),
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
            state
                .flags
                .retain(|flag| !flag.starts_with("RVP:") && !flag.starts_with(WINDOW_COMMAND));
            state.flags.extend(
                main.state
                    .flags
                    .iter()
                    .filter(|flag| flag.starts_with("RVP:"))
                    .cloned(),
            );
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

    /// Shows or hides the window whose toggle is `command` (`RVPWIN:`).
    pub(super) fn toggle_extra(mut self: Pin<&mut Self>, command: &str) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(index) = core
            .extras
            .iter()
            .position(|extra| extra.toggle == Some(command))
        else {
            return;
        };
        let extra = &mut core.extras[index];
        extra.shown = !extra.shown;
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
            .is_ok()
        {
            self.sync_extras(true);
        }
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
        let tooltip = extra.pointer.tooltip.unwrap_or("");
        self.as_mut().set_extra_tooltip(QString::from(tooltip));
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
