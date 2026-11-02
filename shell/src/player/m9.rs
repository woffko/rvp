//! M9's part of `Player`: mpv's on-screen controllers
//! (`rvp_app::controller`). Over the video the pointer goes to mpv; the
//! clicks the controller leaves come back through RVP's bindings in mpv's
//! `default` section and act as RVP's own pointer. Outside the video (the
//! skin's bars, title bar and playlist) the pointer stays RVP's. While a
//! controller's menu is open it takes the keys. The window's state goes
//! both ways through mpv's `fullscreen`, `window-maximized`,
//! `window-minimized` and `ontop`, and mpv's `quit` closes RVP.
//!
//! "Without the skin" the window is the video with the skin's bars kept
//! hidden, as in fullscreen; its edges resize it.

use std::pin::Pin;
use std::time::{Duration, Instant};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use rvp_app::controller::{self, KIND_OSC, MAXIMIZE_MESSAGE, PointerInput, SECTION, WindowState};
use rvp_app::pointer::{Button, Modifiers};
use rvp_app::{Action, App, OnTop};
use rvp_engine::{Changes, Engine, EngineEvent, PropertyId};
use serde_json::{Value, json};

use super::{Core, request};
use crate::player_qobject::qobject;

/// The edge of a skinless window that resizes it, in pixels.
const EDGE: f64 = 6.0;

/// Longer than mpv's double-click time (`input-doubleclick-time`, 300 ms).
const DOUBLE_CLICK: Duration = Duration::from_millis(600);

/// A change of mpv's window options this soon after RVP wrote them is
/// RVP's own write coming back, not a controller's request.
const ECHO: Duration = Duration::from_millis(400);

/// Qt's cursor shapes for the edges (`Qt::CursorShape`).
const CURSOR_SIZE_VER: i32 = 5;
const CURSOR_SIZE_HOR: i32 = 6;
const CURSOR_SIZE_BDIAG: i32 = 7;
const CURSOR_SIZE_FDIAG: i32 = 8;

/// The running controller.
#[derive(Debug)]
pub(super) struct Controller {
    /// The kind the core started with (`controller::KINDS`).
    kind: String,
    /// The pointer is over the video, and mpv was told so.
    inside: bool,
    /// Where the pointer is, in the window's pixels.
    at: (f64, f64),
    /// The modifiers of the last press RVP gave mpv.
    modifiers: Modifiers,
    /// mpv's names of the buttons held down in mpv.
    held: Vec<(Button, String)>,
    /// When RVP got the last two presses mpv left: mpv makes a double
    /// click of any two presses, one of them on the controller.
    presses: [Option<Instant>; 2],
    /// The window's state RVP last wrote into mpv's options, and when.
    written: WindowState,
    written_at: Option<Instant>,
}

/// Turns the controller on when the settings ask for it and libmpv can
/// run it: RVP's bindings go into mpv's `default` section. Without Lua
/// (the Linux kit's libmpv) the core has no `osc` option, and RVP keeps
/// its own pointer.
pub(super) fn start(app: &App, engine: &Engine) -> (Option<Controller>, Option<String>) {
    let settings = &app.settings.controller;
    if !settings.active() {
        return (None, None);
    }
    if engine.get_text("osc").is_none() {
        return (
            None,
            Some("mpv's on-screen controllers need a libmpv with Lua".to_owned()),
        );
    }
    let mut message = None;
    if settings.kind != KIND_OSC && controller::scripts(&app.mpv_folder()).is_empty() {
        message = Some(format!(
            "No scripts in {}",
            app.mpv_folder().join("scripts").display()
        ));
    }
    if let Err(error) = engine.command(
        "define-section",
        &[SECTION, &controller::section(), "force"],
    ) {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("the controller's input section: {error}"),
        );
    }
    (
        Some(Controller {
            kind: settings.kind.clone(),
            inside: false,
            at: (0.0, 0.0),
            modifiers: Modifiers::default(),
            held: Vec::new(),
            presses: [None, None],
            written: WindowState::default(),
            written_at: None,
        }),
        message,
    )
}

/// Whether a controller's menu is open: it takes the keys then. Read
/// when a key comes (uosc closes its menu by setting its `user-data` to
/// null, which an observation of the value does not report).
fn menu_open(engine: &Engine) -> bool {
    // `user-data` reads as JSON: `true`, `"menu"`, `null`.
    let shown = |name: &str| match engine.get_json(name) {
        Some(Value::Bool(open)) => open,
        Some(Value::String(kind)) => !kind.is_empty(),
        _ => false,
    };
    shown("user-data/mpv/console/open")
        || shown("user-data/mpv/context-menu/open")
        || shown("user-data/uosc/menu/type")
}

impl qobject::Player {
    /// A controller runs (this start's settings).
    pub(super) fn controller_active(&self) -> bool {
        self.core().is_some_and(|core| core.controller.is_some())
    }

    /// The window holds only the video and the controller.
    pub(super) fn controller_without_skin(&self) -> bool {
        self.core().is_some_and(|core| {
            core.controller.is_some() && core.app.settings.controller.without_skin
        })
    }

    /// The point in the OSD's pixels (the video's), when it is over the
    /// video.
    fn controller_osd_point(&self, x: f64, y: f64) -> Option<(i64, i64)> {
        let rect = self.video_rect().clone();
        let core = self.core()?;
        if rect.width() < 1.0
            || rect.height() < 1.0
            || x < rect.x()
            || y < rect.y()
            || x >= rect.x() + rect.width()
            || y >= rect.y() + rect.height()
        {
            return None;
        }
        #[allow(clippy::cast_precision_loss)] // OSD sizes are small.
        let (osd_width, osd_height) = (
            core.app.model.int(PropertyId::OsdWidth).unwrap_or(0) as f64,
            core.app.model.int(PropertyId::OsdHeight).unwrap_or(0) as f64,
        );
        // Before mpv's first frame: the window's own pixels.
        let (sx, sy) = if osd_width > 0.0 && osd_height > 0.0 {
            (osd_width / rect.width(), osd_height / rect.height())
        } else {
            (1.0, 1.0)
        };
        #[allow(clippy::cast_possible_truncation)] // Inside the OSD.
        Some((
            ((x - rect.x()) * sx).floor() as i64,
            ((y - rect.y()) * sy).floor() as i64,
        ))
    }

    /// The edges of a skinless window under the point (`Qt::Edges` bits).
    fn controller_edges(&self, x: f64, y: f64) -> i32 {
        if !self.controller_without_skin()
            || *self.fullscreen()
            || self.core().is_some_and(|core| core.maximized)
        {
            return 0;
        }
        let rect = self.video_rect().clone();
        let mut edges = 0;
        if y < rect.y() + EDGE {
            edges |= 1;
        }
        if x < rect.x() + EDGE {
            edges |= 2;
        }
        if x >= rect.x() + rect.width() - EDGE {
            edges |= 4;
        }
        if y >= rect.y() + rect.height() - EDGE {
            edges |= 8;
        }
        edges
    }

    fn controller_command(&self, name: &str, args: &[&str]) {
        if let Some(core) = self.core()
            && let Err(error) = core.engine.command(name, args)
        {
            rvp_app::log::report(
                rvp_app::log::Level::Warning,
                &format!("controller {name}: {error}"),
            );
        }
    }

    /// The pointer moved: mpv learns where it is over the video, and
    /// when it comes and goes. RVP's own handling goes on after this; the
    /// cursor a skinless window's edge sets comes back.
    pub(super) fn controller_move(mut self: Pin<&mut Self>, x: f64, y: f64) -> Option<i32> {
        if !self.controller_active() {
            return None;
        }
        let point = self.controller_osd_point(x, y);
        let edges = self.controller_edges(x, y);
        let core = self.as_mut().core_mut()?;
        let state = core.controller.as_mut()?;
        state.at = (x, y);
        let was_inside = state.inside;
        state.inside = point.is_some();
        match (was_inside, point) {
            (false, Some(_)) => self.controller_command("keypress", &["MOUSE_ENTER"]),
            (true, None) => self.controller_command("keypress", &["MOUSE_LEAVE"]),
            _ => {}
        }
        if let Some((px, py)) = point {
            self.controller_command("mouse", &[&px.to_string(), &py.to_string()]);
        }
        match edges {
            0 => None,
            1 | 8 => Some(CURSOR_SIZE_VER),
            2 | 4 => Some(CURSOR_SIZE_HOR),
            3 | 12 => Some(CURSOR_SIZE_FDIAG),
            _ => Some(CURSOR_SIZE_BDIAG),
        }
    }

    /// The pointer left the window.
    pub(super) fn controller_leave(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(state) = core.controller.as_mut() else {
            return;
        };
        if state.inside {
            state.inside = false;
            self.controller_command("keypress", &["MOUSE_LEAVE"]);
        }
    }

    /// A press (`phase` 0), double click (1) or release (2): over the
    /// video it goes to mpv, which hands back what its controller leaves
    /// ([`Self::controller_drained`]). True when it went to mpv.
    pub(super) fn controller_button(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        button: Button,
        phase: u8,
        modifiers: Modifiers,
    ) -> bool {
        if !self.controller_active() {
            return false;
        }
        let point = self.controller_osd_point(x, y);
        let over = point.is_some();
        let edges = if phase == 0 && button == Button::Left {
            self.controller_edges(x, y)
        } else {
            0
        };
        if edges != 0 {
            self.ui_request(request::START_RESIZE, edges);
            return true;
        }
        let Some(core) = self.as_mut().core_mut() else {
            return false;
        };
        let Some(state) = core.controller.as_mut() else {
            return false;
        };
        match phase {
            0 => {
                if !over {
                    return false;
                }
                let name = controller::button_name(button, modifiers);
                state.modifiers = modifiers;
                state.at = (x, y);
                state.held.retain(|(held, _)| *held != button);
                state.held.push((button, name.clone()));
                // mpv acts where it last saw the pointer.
                if let Some((px, py)) = point {
                    self.controller_command("mouse", &[&px.to_string(), &py.to_string()]);
                }
                self.controller_command("keydown", &[&name]);
                true
            }
            // mpv makes its own double clicks from two presses.
            1 => over,
            _ => {
                let Some(index) = state.held.iter().position(|(held, _)| *held == button) else {
                    return false;
                };
                let (_, name) = state.held.remove(index);
                self.controller_command("keyup", &[&name]);
                true
            }
        }
    }

    /// A wheel turn over the video goes to mpv.
    pub(super) fn controller_wheel(&self, x: f64, y: f64, delta: i32) -> bool {
        if !self.controller_active() || delta == 0 || self.controller_osd_point(x, y).is_none() {
            return false;
        }
        let name = if delta > 0 { "WHEEL_UP" } else { "WHEEL_DOWN" };
        self.controller_command("keypress", &[name]);
        true
    }

    /// A key, while a controller's menu is open, goes to mpv.
    pub(crate) fn controller_key(&self, key: i32, modifiers: i32, text: &QString) -> bool {
        let Some(core) = self.core() else {
            return false;
        };
        if core.controller.is_none() || !menu_open(core.engine) {
            return false;
        }
        let Some(name) = controller::mpv_key(key, modifiers, &text.to_string()) else {
            return false;
        };
        self.controller_command("keypress", &[&name]);
        true
    }

    /// An mpv command for the self-test, as a controller's script would
    /// run it: only while a self-test runs.
    pub(crate) fn selftest_mpv(&self, command: &QString) -> bool {
        if !self.rust().cli.flag("selftest") {
            return false;
        }
        let Ok(args) = serde_json::from_str::<Vec<String>>(&command.to_string()) else {
            return false;
        };
        let Some((name, rest)) = args.split_first() else {
            return false;
        };
        let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
        self.core()
            .is_some_and(|core| core.engine.command(name, &rest).is_ok())
    }

    /// M9's part of a drain: the controller's events, and the classic
    /// skins' clock from a new file on.
    pub(super) fn m9_drained(mut self: Pin<&mut Self>, events: &[EngineEvent], changes: Changes) {
        if events
            .iter()
            .any(|event| matches!(event, EngineEvent::FileLoaded { .. }))
            && let Some(core) = self.as_mut().core_mut()
        {
            core.file_clock = Instant::now();
        }
        self.controller_drained(events, changes);
    }

    /// mpv's events for the controller: the clicks it left, the commands
    /// RVP gave the OSC's buttons, its window requests and `quit`.
    pub(super) fn controller_drained(
        mut self: Pin<&mut Self>,
        events: &[EngineEvent],
        changes: Changes,
    ) {
        if !self.controller_active() {
            return;
        }
        for event in events {
            match event {
                EngineEvent::ClientMessage(args) => {
                    if let Some(input) = controller::pointer_message(args) {
                        self.as_mut().controller_pointer(input);
                    } else if let Some(command) = controller::command_message(args) {
                        self.as_mut().controller_run(command);
                    }
                }
                // A controller's close button runs mpv's `quit`: the core
                // is gone, and RVP closes.
                EngineEvent::Shutdown => self.as_mut().perform(Action::Exit),
                _ => {}
            }
        }
        let window = [
            PropertyId::Fullscreen,
            PropertyId::WindowMaximized,
            PropertyId::WindowMinimized,
            PropertyId::OnTop,
        ];
        if window.iter().any(|id| changes.contains(*id)) {
            self.controller_window_request();
        }
    }

    /// A click or wheel turn the controller left: RVP's pointer, where
    /// the pointer is.
    fn controller_pointer(mut self: Pin<&mut Self>, input: PointerInput) {
        let Some(state) = self
            .as_mut()
            .core_mut()
            .and_then(|core| core.controller.as_mut())
        else {
            return;
        };
        let ((x, y), modifiers) = (state.at, state.modifiers);
        match input {
            PointerInput::Press(button) => {
                state.presses = [state.presses[1], Some(Instant::now())];
                self.rvp_pointer(x, y, button, 0, modifiers);
            }
            PointerInput::Double(button) => {
                // Both presses must have been RVP's.
                if state.presses[0].is_some_and(|at| at.elapsed() < DOUBLE_CLICK) {
                    self.rvp_pointer(x, y, button, 1, modifiers);
                }
            }
            PointerInput::Release(button) | PointerInput::Cancel(button) => {
                self.rvp_pointer(x, y, button, 2, modifiers);
            }
            PointerInput::Wheel(notches) => self.rvp_wheel(x, y, notches * 120),
        }
    }

    /// A command RVP gave the OSC's buttons.
    fn controller_run(mut self: Pin<&mut Self>, command: &str) {
        let action = if command == MAXIMIZE_MESSAGE {
            Some(if *self.fullscreen() {
                Action::ToggleFullscreen
            } else {
                Action::MaximizeRestore
            })
        } else {
            Action::from_name(command)
        };
        if let Some(action) = action {
            self.as_mut().perform(action);
        }
        self.sync_chrome();
    }

    /// RVP's window as mpv's options describe it.
    fn controller_window_state(&self) -> WindowState {
        WindowState {
            fullscreen: *self.fullscreen(),
            maximized: self.core().is_some_and(|core| core.maximized),
            minimized: false,
            on_top: *self.on_top(),
        }
    }

    /// A controller changed mpv's window options: RVP's window follows.
    fn controller_window_request(mut self: Pin<&mut Self>) {
        let real = self.controller_window_state();
        let Some(core) = self.core() else {
            return;
        };
        let Some(state) = core.controller.as_ref() else {
            return;
        };
        if state.written_at.is_some_and(|at| at.elapsed() < ECHO) {
            return;
        }
        let model = &core.app.model;
        let asked = WindowState {
            fullscreen: model.flag(PropertyId::Fullscreen) == Some(true),
            maximized: model.flag(PropertyId::WindowMaximized) == Some(true),
            minimized: model.flag(PropertyId::WindowMinimized) == Some(true),
            on_top: model.flag(PropertyId::OnTop) == Some(true),
        };
        let written = state.written;
        if asked.fullscreen != written.fullscreen && asked.fullscreen != real.fullscreen {
            self.as_mut().perform(Action::ToggleFullscreen);
        }
        if asked.maximized != written.maximized
            && asked.maximized != real.maximized
            && !real.fullscreen
        {
            self.as_mut().perform(Action::MaximizeRestore);
        }
        if asked.minimized && !written.minimized {
            self.as_mut().perform(Action::Minimize);
        }
        if asked.on_top != written.on_top && asked.on_top != real.on_top {
            self.as_mut().perform(Action::SetOnTop(if asked.on_top {
                OnTop::Always
            } else {
                OnTop::Never
            }));
        }
        // The window's state goes back, minimized at once restored in
        // mpv's eyes: the next click minimizes again.
        if let Some(state) = self
            .as_mut()
            .core_mut()
            .and_then(|core| core.controller.as_mut())
        {
            state.written.minimized = asked.minimized;
        }
        self.controller_tick();
    }

    /// Writes RVP's window state into mpv's options when it changed, so
    /// that the controllers' buttons show it. Each sync of the chrome.
    pub(super) fn controller_tick(mut self: Pin<&mut Self>) {
        let real = self.controller_window_state();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let engine = core.engine;
        let Some(state) = core.controller.as_mut() else {
            return;
        };
        if state.written == real {
            return;
        }
        for (name, now, before) in [
            ("fullscreen", real.fullscreen, state.written.fullscreen),
            ("window-maximized", real.maximized, state.written.maximized),
            ("window-minimized", real.minimized, state.written.minimized),
            ("ontop", real.on_top, state.written.on_top),
        ] {
            if now != before
                && let Err(error) = engine.set_flag(name, now)
            {
                rvp_app::log::report(
                    rvp_app::log::Level::Warning,
                    &format!("controller {name}: {error}"),
                );
            }
        }
        state.written = real;
        state.written_at = Some(Instant::now());
    }
}

/// A VLC theme's conditions and texts (`rvp_skin::vlc::Runtime`), read
/// once per skin.
#[derive(Debug, Default)]
pub(super) struct VlcCache {
    skin: u64,
    runtime: Option<rvp_skin::vlc::Runtime>,
}

impl VlcCache {
    /// A tooltip with VLC's text codes filled in, when a VLC theme shows.
    pub(super) fn tooltip(&self, app: &App, tooltip: &str) -> String {
        if self.runtime.is_some() {
            app.vlc_text(tooltip)
        } else {
            tooltip.to_owned()
        }
    }

    /// How many conditions the theme has.
    pub(super) fn conditions(&self) -> usize {
        self.runtime
            .as_ref()
            .map_or(0, |runtime| runtime.conditions.len())
    }
}

/// Fills a VLC theme's flags and texts for the main window: `fullscreen`,
/// `on_top` and `maximized` are the window's.
#[allow(clippy::too_many_arguments)] // The window's state, field by field.
pub(super) fn vlc_data(
    app: &App,
    cache: &mut VlcCache,
    extras: &[super::extras::Extra],
    skin: &rvp_skin::Skin,
    window: (bool, bool, bool),
    data: &mut rvp_skin::DataModel,
    state: &mut rvp_skin::UiState,
) {
    if cache.skin != skin.id() {
        let runtime = rvp_skin::vlc::Runtime::of(skin);
        cache.skin = skin.id();
        cache.runtime =
            (!runtime.conditions.is_empty() || !runtime.texts.is_empty()).then_some(runtime);
    }
    let Some(runtime) = cache.runtime.as_ref() else {
        return;
    };
    let area = skin.area("MainSkinArea");
    let attr = |name: &str| {
        area.and_then(|area| area.attrs.get(name))
            .unwrap_or("")
            .to_owned()
    };
    let (fullscreen, on_top, maximized) = window;
    let window = rvp_app::vlc::WindowState {
        fullscreen,
        on_top,
        maximized,
        main: attr("RvpWindowId"),
        shown: extras
            .iter()
            .filter_map(super::extras::Extra::shown_id)
            .map(str::to_owned)
            .collect(),
        layout: attr("RvpLayout"),
    };
    runtime.set_flags(&mut state.flags, &mut |name| {
        app.vlc_variable(name, &window) == Some(true)
    });
    for (n, template) in &runtime.texts {
        data.set_text(
            &format!("{}{n}", rvp_skin::vlc::build::TEXT),
            app.vlc_text(template),
        );
    }
}

/// M9's part of `Player::status` (the self-test reads it): the controller
/// and the skin's other windows.
pub(super) fn status(core: &Core) -> Vec<(&'static str, String)> {
    let controller = core.controller.as_ref().map_or(Value::Null, |state| {
        json!({
            "kind": state.kind,
            "inside": state.inside,
            "menu_open": menu_open(core.engine),
            "osc": core.engine.get_text("osc"),
            "scripts": controller::scripts(&core.app.mpv_folder()).len(),
            "held": state.held.len(),
        })
    });
    let mut fields = vec![("controller", controller.to_string())];
    fields.extend(super::extras::status(core));
    // A VLC theme's conditions that hold and its texts.
    let vlc = core.chrome.as_ref().map_or(Value::Null, |chrome| {
        let on = chrome
            .state
            .flags
            .iter()
            .filter(|flag| flag.starts_with(rvp_skin::vlc::build::CONDITION))
            .count();
        let texts: Vec<&str> = chrome
            .data
            .texts
            .iter()
            .filter(|(command, _)| command.starts_with(rvp_skin::vlc::build::TEXT))
            .map(|(_, text)| text.as_str())
            .collect();
        json!({
            "conditions": core.vlc.conditions(),
            "on": on,
            "texts": texts,
        })
    });
    fields.push(("vlc", vlc.to_string()));
    fields.push((
        "window_mask",
        json!(super::WINDOW_MASK_LEN.load(std::sync::atomic::Ordering::Relaxed)).to_string(),
    ));
    fields
}
