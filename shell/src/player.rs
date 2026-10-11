//! `Player`: the `QObject` behind the window. It owns the application core,
//! keeps the chrome in step with the engine, turns input into actions and
//! asks QML for what only the window can do.
//!
//! Threading (docs/PLAN.md section 5): mpv's wakeup callback only queues
//! [`qobject::Player::drain`] onto the Qt thread; the render thread is told
//! to repaint through the video item. Chrome repaints go through one path
//! (the `damaged` signal and `Tools.repaint`), which applies them with mpv's
//! next video frame while a video plays (goal C rule 1).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QRectF, QString};
use rvp_app::chrome::{Chrome, Damage, Frame, Mode};
use rvp_app::pointer::{Button, Context, Cursor, Effect, Pointer};
use rvp_app::{Action, App, Cli, Dirs, KeyChord, Settings, UiRequest, WindowGeometry, view};
use rvp_engine::{Engine, EngineEvent, PropertyId};
use rvp_skin::Skin;

use crate::player_qobject::qobject;
use crate::video::ENGINE;

/// `ui_request` kinds.
pub mod request {
    pub const OPEN_FILES: i32 = 1;
    /// Argument 1 enters, 0 leaves fullscreen.
    pub const FULLSCREEN: i32 = 2;
    pub const MINIMIZE: i32 = 3;
    pub const MAXIMIZE_RESTORE: i32 = 4;
    pub const EXIT: i32 = 5;
    pub const CONTEXT_MENU: i32 = 6;
    pub const START_MOVE: i32 = 7;
    /// Argument: `Qt::Edges` bits.
    pub const START_RESIZE: i32 = 8;
}

/// The skin, loaded once for the process (the painters read it on the
/// render thread).
static SKIN: OnceLock<&'static Skin> = OnceLock::new();
/// The frame the painters draw.
static FRAME: Mutex<Option<Arc<Frame>>> = Mutex::new(None);

pub fn skin() -> Option<&'static Skin> {
    SKIN.get().copied()
}

pub fn frame() -> Option<Arc<Frame>> {
    FRAME.lock().ok().and_then(|frame| frame.clone())
}

fn publish(frame: Arc<Frame>) {
    if let Ok(mut slot) = FRAME.lock() {
        *slot = Some(frame);
    }
}

/// Everything that exists once `start` ran.
struct Core {
    app: App,
    engine: &'static Engine,
    chrome: Option<Chrome>,
    pointer: Pointer,
    /// Files to open once the video item is ready; `None` once started.
    pending: Option<Vec<String>>,
    /// mpv's playlist entries.
    playlist: Vec<String>,
    width: i32,
    height: i32,
    maximized: bool,
    stretch: bool,
    /// Resize the window when the next video's size is known.
    fit_pending: bool,
    /// True while a drain is queued (coalesces wakeups).
    drain_queued: Arc<AtomicBool>,
}

/// The Rust state of [`qobject::Player`]; the fields are the properties.
// Each flag is a separate QML property.
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub struct PlayerRust {
    pub(crate) window_title: QString,
    pub(crate) video_rect: QRectF,
    pub(crate) region_count: i32,
    pub(crate) layout_serial: i32,
    pub(crate) tooltip: QString,
    pub(crate) cursor_shape: i32,
    pub(crate) fullscreen: bool,
    pub(crate) chrome_shown: bool,
    pub(crate) message: QString,
    pub(crate) has_skin: bool,
    pub(crate) has_video: bool,
    pub(crate) restore_x: i32,
    pub(crate) restore_y: i32,
    pub(crate) restore_width: i32,
    pub(crate) restore_height: i32,
    pub(crate) restore_position: bool,
    pub(crate) restore_maximized: bool,
    /// The command line, parsed by `start`.
    cli: Cli,
    core: Option<Core>,
}

/// Qt's mouse button numbers (`Qt::MouseButton`).
fn button(value: i32) -> Option<Button> {
    Some(match value {
        1 => Button::Left,
        2 => Button::Right,
        4 => Button::Middle,
        8 => Button::Back,
        16 => Button::Forward,
        _ => return None,
    })
}

/// Qt's cursor shapes (`Qt::CursorShape`).
fn cursor_shape(cursor: Cursor) -> i32 {
    match cursor {
        Cursor::Arrow => 0,
        Cursor::SizeVertical => 5,
        Cursor::SizeHorizontal => 6,
        Cursor::SizeBackwardDiagonal => 7,
        Cursor::SizeForwardDiagonal => 8,
    }
}

fn to_rectf(rect: rvp_skin::Rect) -> QRectF {
    QRectF::new(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.w),
        f64::from(rect.h),
    )
}

/// Window coordinates from QML (fractional on scaled screens) to pixels.
fn pixel(value: f64) -> i32 {
    // Window coordinates are far inside i32.
    #[allow(clippy::cast_possible_truncation)]
    let pixel = value.floor() as i32;
    pixel
}

/// The skin to load: `--skin`, the setting, `RVP_SKIN`, or PotPlayer's
/// installed BlackSpace on Windows.
fn skin_path(cli: &Cli, settings: &Settings) -> Option<PathBuf> {
    cli.value("skin")
        .map(PathBuf::from)
        .or_else(|| settings.skin.clone())
        .or_else(|| std::env::var_os("RVP_SKIN").map(PathBuf::from))
        .or_else(|| {
            cfg!(windows)
                .then(|| PathBuf::from(r"C:\Program Files\DAUM\PotPlayer\Skins\BlackSpace_en.dsf"))
        })
}

type SkinLoad = std::thread::JoinHandle<Result<Skin, String>>;

/// The skin being loaded since the process started, and its path.
static PRELOAD: Mutex<Option<(PathBuf, SkinLoad)>> = Mutex::new(None);

fn open_skin(path: &Path) -> Result<Skin, String> {
    Skin::open(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// Starts loading the skin on a thread while Qt and QML start (called first
/// in `main`). The settings are read here and again in `start`; both are
/// cheap.
pub fn preload_skin() {
    let cli = Cli::parse(std::env::args().skip(1));
    let settings = Dirs::from_env()
        .ok()
        .and_then(|dirs| rvp_app::read_json::<Settings>(&dirs.config.join("settings.json")).ok())
        .flatten()
        .unwrap_or_default();
    let Some(path) = skin_path(&cli, &settings) else {
        return;
    };
    let thread_path = path.clone();
    let spawned = std::thread::Builder::new()
        .name("rvp-skin".to_owned())
        .spawn(move || open_skin(&thread_path));
    if let (Ok(handle), Ok(mut slot)) = (spawned, PRELOAD.lock()) {
        *slot = Some((path, handle));
    }
}

fn load_skin(path: &Path) -> Result<&'static Skin, String> {
    let preloaded = PRELOAD
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
        .filter(|(preloaded_path, _)| preloaded_path == path);
    let skin = match preloaded {
        Some((_, handle)) => handle
            .join()
            .map_err(|_| "the skin loader stopped".to_owned())??,
        None => open_skin(path)?,
    };
    // The skin lives for the whole process: the layout borrows it and the
    // render thread paints from it.
    let skin: &'static Skin = Box::leak(Box::new(skin));
    let _ = SKIN.set(skin);
    Ok(skin)
}

/// The application core from the user's directories; on failure a
/// temporary state directory and a message saying so.
fn load_app() -> Result<(App, Option<String>), String> {
    let loaded = Dirs::from_env().and_then(|dirs| {
        App::load(&dirs).map_err(|error| format!("cannot read the settings: {error}"))
    });
    match loaded {
        Ok(app) => Ok((app, None)),
        Err(message) => {
            let fallback = std::env::temp_dir().join("rvp-state");
            let app = App::new(Settings::default(), &fallback).map_err(|e| e.to_string())?;
            Ok((app, Some(message)))
        }
    }
}

fn load_chrome(cli: &Cli, settings: &Settings) -> Result<Chrome, String> {
    let path = skin_path(cli, settings).ok_or_else(|| {
        "no skin configured: set \"skin\" in settings.json or pass --skin=PATH".to_owned()
    })?;
    let skin = load_skin(&path).map_err(|error| format!("cannot load the skin {error}"))?;
    Ok(Chrome::new(skin))
}

/// A JSON string literal (for the self-test's status lines).
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `--mpv-options=name=value,name=value` for experiments and test runs.
fn extra_mpv_options(cli: &Cli) -> Vec<(String, String)> {
    cli.value("mpv-options")
        .unwrap_or("")
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

impl qobject::Player {
    fn core(&self) -> Option<&Core> {
        self.rust().core.as_ref()
    }

    fn core_mut(self: Pin<&mut Self>) -> Option<&mut Core> {
        // SAFETY-free projection: PlayerRust is Unpin.
        self.rust_mut().get_mut().core.as_mut()
    }

    fn show_message(mut self: Pin<&mut Self>, text: &str) {
        eprintln!("rvp: {text}");
        self.as_mut().set_message(QString::from(text));
    }

    pub(crate) fn start(mut self: Pin<&mut Self>) {
        if self.core().is_some() {
            return;
        }
        // First, so that --selftest runs (and reports) even when the start
        // fails below.
        let cli = Cli::parse(std::env::args().skip(1));
        self.as_mut().rust_mut().get_mut().cli = cli.clone();
        let mut messages = Vec::new();
        let app = match load_app() {
            Ok((app, message)) => {
                messages.extend(message);
                app
            }
            Err(error) => {
                self.show_message(&format!("cannot start: {error}"));
                return;
            }
        };
        messages.extend(app.warnings.iter().cloned());
        crate::metrics::mark("settings_loaded_ms");
        let chrome = match load_chrome(&cli, &app.settings) {
            Ok(chrome) => Some(chrome),
            Err(error) => {
                messages.push(error);
                None
            }
        };
        crate::metrics::mark("skin_loaded_ms");

        crate::ffi::bridge::rvp_set_c_numeric_locale();
        let mut options = app.engine_options();
        options.extra.extend(extra_mpv_options(&cli));
        let mut engine = match Engine::new(&options) {
            Ok(engine) => engine,
            Err(error) => {
                self.show_message(&format!("cannot start libmpv: {error}"));
                return;
            }
        };
        let drain_queued = Arc::new(AtomicBool::new(false));
        let thread = self.qt_thread();
        let queued = Arc::clone(&drain_queued);
        // On an mpv thread: only queue work (section 5).
        engine.set_wakeup_callback(move || {
            if !queued.swap(true, Ordering::AcqRel) {
                let _ = thread.queue(qobject::Player::drain);
            }
        });
        if ENGINE.set(engine).is_err() {
            return;
        }
        crate::metrics::mark("engine_ready_ms");
        let Some(engine) = ENGINE.get() else {
            return;
        };
        let ready_thread = Mutex::new(self.qt_thread());
        crate::video::on_ready(move || {
            if let Ok(thread) = ready_thread.lock() {
                let _ = thread.queue(qobject::Player::render_ready);
            }
        });

        if let Some(window) = app.store().state().window {
            self.as_mut().set_restore_width(window.width);
            self.as_mut().set_restore_height(window.height);
            if let (Some(x), Some(y)) = (window.x, window.y) {
                self.as_mut().set_restore_x(x);
                self.as_mut().set_restore_y(y);
                self.as_mut().set_restore_position(true);
            }
            self.as_mut().set_restore_maximized(window.maximized);
        }
        self.as_mut().set_has_skin(chrome.is_some());
        self.as_mut().set_window_title(QString::from("RVP"));
        let files = cli.files.clone();
        self.as_mut().rust_mut().get_mut().core = Some(Core {
            app,
            engine,
            chrome,
            pointer: Pointer::default(),
            pending: Some(files),
            playlist: Vec::new(),
            width: 0,
            height: 0,
            maximized: false,
            stretch: false,
            fit_pending: false,
            drain_queued,
        });
        if let Some(message) = messages.first() {
            self.as_mut().show_message(message);
        }
        if crate::video::READY.load(Ordering::Acquire) {
            self.render_ready();
        }
    }

    /// The render context exists: open the files.
    fn render_ready(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(files) = core.pending.take() else {
            return;
        };
        if files.is_empty() {
            // Reopening the last file keeps the restored window size.
            core.fit_pending = false;
        } else {
            core.fit_pending = core.app.settings.fit_window_to_video;
            crate::metrics::first_frame_pending();
        }
        let result = core.app.start(core.engine, &files);
        if let Err(error) = result {
            self.show_message(&format!("cannot open: {error}"));
        }
    }

    /// Queued by mpv's wakeup callback.
    fn drain(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.drain_queued.store(false, Ordering::Release);
        let drained = core.app.drain(core.engine, Instant::now());
        let mut playlist_changed = drained.changes.contains(PropertyId::PlaylistCount);
        for event in &drained.events {
            match event {
                EngineEvent::PlaybackRestart => crate::metrics::playback_restarted(),
                EngineEvent::FileLoaded { .. } => {
                    crate::metrics::mark_once("file_loaded_ms");
                    playlist_changed = true;
                }
                EngineEvent::StartFile => playlist_changed = true,
                _ => {}
            }
        }
        if playlist_changed {
            core.playlist = core.engine.playlist();
        }
        let fit = if core.fit_pending
            && !core.maximized
            && let (Some(width), Some(height)) = (
                core.app.model.int(PropertyId::VideoWidth),
                core.app.model.int(PropertyId::VideoHeight),
            ) {
            core.fit_pending = false;
            let (chrome_width, chrome_height) =
                core.chrome.as_ref().map_or((0, 0), Chrome::chrome_size);
            match (i32::try_from(width), i32::try_from(height)) {
                (Ok(width), Ok(height)) => Some((width + chrome_width, height + chrome_height)),
                _ => None,
            }
        } else {
            None
        };
        let has_video = core.app.model.int(PropertyId::VideoWidth).is_some();
        let title = view::title(&core.app.model);
        let messages: Vec<String> = drained
            .requests
            .into_iter()
            .filter_map(|request| match request {
                UiRequest::Message(text) => Some(text),
                _ => None,
            })
            .collect();
        self.as_mut().set_has_video(has_video);
        self.as_mut()
            .set_window_title(QString::from(if title.is_empty() {
                "RVP"
            } else {
                title.as_str()
            }));
        for message in messages {
            self.as_mut().show_message(&message);
        }
        if let Some((width, height)) = fit
            && !*self.fullscreen()
        {
            self.as_mut().fit_window(width, height);
        }
        self.sync_chrome();
    }

    /// Recomputes the chrome from the model and signals what to repaint.
    fn sync_chrome(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let synced =
            core.app.model.playing() && core.app.model.int(PropertyId::VideoWidth).is_some();
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        let data = view::data_model(&core.app.model, &core.playlist);
        if data != chrome.data {
            chrome.data = data;
        }
        view::apply_playback_state(&mut chrome.state, &core.app.model);
        let damage = chrome.refresh();
        self.emit_damage(&damage, synced);
    }

    fn emit_damage(mut self: Pin<&mut Self>, damage: &Damage, synced: bool) {
        if damage.is_empty() {
            return;
        }
        let Some(core) = self.core() else {
            return;
        };
        let Some(chrome) = core.chrome.as_ref() else {
            return;
        };
        let frame = chrome.frame();
        let count = i32::try_from(frame.regions.len()).unwrap_or(i32::MAX);
        let video = chrome.video_rect();
        publish(frame);
        if damage.regions_changed {
            let serial = self.layout_serial().wrapping_add(1);
            self.as_mut().set_layout_serial(serial);
            self.as_mut().set_region_count(count);
        }
        self.as_mut().set_video_rect(to_rectf(video));
        let regions = damage
            .regions
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        crate::metrics::chrome_repainted();
        self.damaged(damage.static_layer, QString::from(&regions), synced);
    }

    fn relayout(mut self: Pin<&mut Self>) {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (width, height) = (core.width, core.height);
        let Some(chrome) = core.chrome.as_mut() else {
            let rect = rvp_skin::Rect::new(0, 0, width, height);
            self.set_video_rect(to_rectf(rect));
            return;
        };
        let mode = if fullscreen {
            Mode::Fullscreen
        } else {
            Mode::Window
        };
        let damage = match chrome.resize(mode, width, height) {
            Ok(damage) => damage,
            Err(error) => {
                let text = format!("skin layout failed: {error}");
                self.show_message(&text);
                return;
            }
        };
        let synced = false;
        self.as_mut().emit_damage(&damage, synced);
        self.sync_chrome();
    }

    pub(crate) fn resized(mut self: Pin<&mut Self>, width: i32, height: i32) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if (core.width, core.height) == (width, height)
            && core
                .chrome
                .as_ref()
                .is_some_and(|c| c.size() == (width, height))
        {
            return;
        }
        core.width = width;
        core.height = height;
        self.relayout();
    }

    pub(crate) fn window_state(mut self: Pin<&mut Self>, maximized: bool, active: bool) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.maximized = maximized;
        if let Some(chrome) = core.chrome.as_mut() {
            chrome.state.maximized = maximized;
            chrome.state.window_active = active;
        }
        self.sync_chrome();
    }

    pub(crate) fn window_fullscreen(mut self: Pin<&mut Self>, fullscreen: bool) {
        if *self.fullscreen() == fullscreen {
            return;
        }
        if let Some(core) = self.as_mut().core_mut()
            && !fullscreen
            && core.stretch
        {
            core.stretch = false;
            let _ = core.engine.set_keep_aspect(true);
        }
        self.as_mut().set_chrome_shown(false);
        self.as_mut().set_fullscreen(fullscreen);
        self.relayout();
    }

    pub(crate) fn window_geometry(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        has_position: bool,
    ) {
        if *self.fullscreen() {
            return;
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let maximized = core.maximized;
        let previous = core.app.store().state().window;
        // While maximized, keep the last normal size and position.
        let geometry = match previous {
            Some(previous) if maximized => WindowGeometry {
                maximized: true,
                ..previous
            },
            _ => WindowGeometry {
                x: has_position.then_some(x),
                y: has_position.then_some(y),
                width,
                height,
                maximized,
            },
        };
        core.app.set_window(geometry);
    }

    fn pointer_context(&self) -> Option<Context> {
        let core = self.core()?;
        let fullscreen = *self.fullscreen();
        Some(Context {
            width: core.width,
            height: core.height,
            movable: !fullscreen && !core.maximized,
            chrome_shown: core.chrome.is_some() && (!fullscreen || *self.chrome_shown()),
        })
    }

    pub(crate) fn mouse_press(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.pointer_event(x, y, button_value, 0);
    }

    pub(crate) fn mouse_double(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.pointer_event(x, y, button_value, 1);
    }

    pub(crate) fn mouse_release(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.pointer_event(x, y, button_value, 2);
    }

    /// `phase`: 0 press, 1 double click, 2 release.
    fn pointer_event(mut self: Pin<&mut Self>, x: f64, y: f64, button_value: i32, phase: u8) {
        let Some(button) = button(button_value) else {
            return;
        };
        let Some(ctx) = self.pointer_context() else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (x, y) = (pixel(x), pixel(y));
        let effects = match (core.chrome.as_mut(), phase) {
            (Some(chrome), 0) => core.pointer.press(chrome, ctx, x, y, button),
            (Some(chrome), 1) => {
                core.pointer
                    .double_click(chrome, &core.app.bindings, ctx, x, y, button)
            }
            (Some(chrome), _) => {
                core.pointer
                    .release(chrome, &core.app.bindings, ctx, x, y, button)
            }
            // Without a skin the whole window is video.
            (None, 1) => core
                .app
                .bindings
                .mouse(match button {
                    Button::Left => rvp_app::MouseGesture::LeftDoubleClick,
                    _ => return,
                })
                .map(Effect::Action)
                .into_iter()
                .collect(),
            (None, 2) => match button {
                Button::Right => core
                    .app
                    .bindings
                    .mouse(rvp_app::MouseGesture::RightClick)
                    .map(Effect::Action)
                    .into_iter()
                    .collect(),
                _ => Vec::new(),
            },
            (None, _) => Vec::new(),
        };
        self.as_mut().apply_effects(&effects);
        self.sync_chrome();
    }

    fn apply_effects(mut self: Pin<&mut Self>, effects: &[Effect]) {
        for effect in effects {
            match *effect {
                Effect::Action(action) => self.as_mut().perform(action),
                Effect::SeekFraction(fraction) => self.as_mut().seek_fraction(fraction),
                Effect::VolumeFraction(fraction) => {
                    if let Some(core) = self.as_mut().core_mut() {
                        core.app.set_volume_fraction(core.engine, fraction);
                    }
                }
                Effect::StartMove => self.as_mut().ui_request(request::START_MOVE, 0),
                Effect::StartResize(edges) => {
                    self.as_mut().ui_request(request::START_RESIZE, edges);
                }
            }
        }
    }

    pub(crate) fn seek_fraction(mut self: Pin<&mut Self>, fraction: f64) {
        if let Some(core) = self.as_mut().core_mut() {
            crate::metrics::seek_issued();
            core.app.seek_to_fraction(core.engine, fraction);
        }
    }

    pub(crate) fn mouse_move(mut self: Pin<&mut Self>, x: f64, y: f64) {
        let Some(ctx) = self.pointer_context() else {
            return;
        };
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (x, y) = (pixel(x), pixel(y));
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        let over_bars = chrome.bar_rects().iter().any(|rect| rect.contains(x, y));
        let effects = core.pointer.moved(chrome, ctx, x, y);
        let tooltip = core.pointer.tooltip.unwrap_or("");
        let cursor = cursor_shape(core.pointer.cursor);
        let dragging = core.pointer.dragging();
        self.as_mut().set_tooltip(QString::from(tooltip));
        self.as_mut().set_cursor_shape(cursor);
        if fullscreen {
            // The bars show while the pointer is over them (auto-hide).
            self.as_mut().set_chrome_shown(over_bars || dragging);
        }
        self.as_mut().apply_effects(&effects);
        self.sync_chrome();
    }

    pub(crate) fn mouse_wheel(mut self: Pin<&mut Self>, delta: i32) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let effects = core.pointer.wheel(&core.app.bindings, delta);
        self.as_mut().apply_effects(&effects);
        self.sync_chrome();
    }

    pub(crate) fn mouse_leave(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(chrome) = core.chrome.as_mut() {
            core.pointer.leave(chrome);
        }
        self.as_mut().set_tooltip(QString::default());
        if *self.fullscreen() {
            self.as_mut().set_chrome_shown(false);
        }
        self.sync_chrome();
    }

    pub(crate) fn key(mut self: Pin<&mut Self>, key: i32, modifiers: i32) -> bool {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.core() else {
            return false;
        };
        let chords = KeyChord::from_qt(key, modifiers);
        let action = chords.iter().find_map(|chord| core.app.bindings.key(chord));
        match action {
            Some(action) => {
                crate::metrics::input("input_ms");
                self.as_mut().perform(action);
                self.sync_chrome();
                true
            }
            // Esc leaves fullscreen when nothing else uses it.
            None if fullscreen
                && chords
                    .first()
                    .is_some_and(|chord| *chord == KeyChord::plain(rvp_app::Key::Escape)) =>
            {
                self.as_mut().perform(Action::ToggleFullscreen);
                true
            }
            None => false,
        }
    }

    pub(crate) fn run_action(mut self: Pin<&mut Self>, name: &QString) {
        if let Some(action) = Action::from_name(&name.to_string()) {
            crate::metrics::input("input_ms");
            self.as_mut().perform(action);
            self.sync_chrome();
        }
    }

    pub(crate) fn menu_label(&self, name: &QString) -> QString {
        let (Some(core), Some(action)) = (self.core(), Action::from_name(&name.to_string())) else {
            return QString::default();
        };
        QString::from(&core.app.menu_entry(action).0)
    }

    pub(crate) fn menu_shortcut(&self, name: &QString) -> QString {
        let (Some(core), Some(action)) = (self.core(), Action::from_name(&name.to_string())) else {
            return QString::default();
        };
        QString::from(&core.app.menu_entry(action).1)
    }

    /// Carries out an action and the window request it may produce.
    fn perform(mut self: Pin<&mut Self>, action: Action) {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if matches!(action, Action::SeekBackward(_) | Action::SeekForward(_)) {
            crate::metrics::seek_issued();
        }
        if matches!(action, Action::Previous | Action::Next) {
            core.fit_pending = core.app.settings.fit_window_to_video;
        }
        let Some(request) = core.app.dispatch(core.engine, action) else {
            return;
        };
        match request {
            UiRequest::OpenFileDialog => self.ui_request(request::OPEN_FILES, 0),
            UiRequest::ToggleFullscreen { stretch } => {
                let enter = !fullscreen;
                let keep_aspect = !(enter && stretch);
                core.stretch = enter && stretch;
                let _ = core.engine.set_keep_aspect(keep_aspect);
                self.as_mut().set_chrome_shown(false);
                self.as_mut().set_fullscreen(enter);
                self.as_mut()
                    .ui_request(request::FULLSCREEN, i32::from(enter));
            }
            UiRequest::Minimize => self.ui_request(request::MINIMIZE, 0),
            UiRequest::MaximizeRestore => {
                if fullscreen {
                    self.as_mut().perform(Action::ToggleFullscreen);
                } else {
                    self.ui_request(request::MAXIMIZE_RESTORE, 0);
                }
            }
            UiRequest::Exit => self.ui_request(request::EXIT, 0),
            UiRequest::ContextMenu => self.ui_request(request::CONTEXT_MENU, 0),
            UiRequest::TogglePlaylist => {
                if let Some(chrome) = core.chrome.as_mut() {
                    chrome.state.playlist_attached = !chrome.state.playlist_attached;
                }
                self.relayout();
            }
            UiRequest::Message(text) => self.show_message(&text),
        }
    }

    pub(crate) fn open_files(mut self: Pin<&mut Self>, paths: &QString, append: bool) {
        let files: Vec<String> = paths
            .to_string()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        if files.is_empty() {
            return;
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(pending) = core.pending.as_mut() {
            // Not ready yet: open them with the start.
            if !append {
                pending.clear();
            }
            pending.extend(files);
            return;
        }
        core.fit_pending = core.app.settings.fit_window_to_video;
        let result = core.app.open(core.engine, &files, append);
        if let Err(error) = result {
            self.show_message(&format!("cannot open: {error}"));
        }
    }

    // A QML invokable, so a method even though it reads only the shared
    // frame.
    #[allow(clippy::unused_self)]
    pub(crate) fn region_rect(&self, index: i32, _serial: i32) -> QRectF {
        let frame = frame();
        let rect = usize::try_from(index)
            .ok()
            .and_then(|index| {
                frame
                    .as_ref()
                    .and_then(|frame| frame.regions.get(index).copied())
            })
            .unwrap_or_default();
        to_rectf(rect)
    }

    pub(crate) fn tick(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Err(error) = core.app.tick(Instant::now()) {
            let text = format!("cannot save the playback state: {error}");
            self.show_message(&text);
        }
    }

    pub(crate) fn shutdown(mut self: Pin<&mut Self>) {
        crate::video::CLOSING.store(true, Ordering::Release);
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Err(error) = core.app.shutdown(core.engine, Instant::now()) {
            eprintln!("rvp: cannot save the playback state: {error}");
        }
        // Let mpv unload the file and its filter chain (SVP's included)
        // before the process goes on to shut libmpv down.
        let deadline = Instant::now() + std::time::Duration::from_secs(3);
        let mut events = Vec::new();
        while core.app.model.flag(PropertyId::IdleActive) != Some(true) && Instant::now() < deadline
        {
            core.engine
                .wait_and_drain(0.1, &mut core.app.model, &mut events);
        }
    }

    // A QML invokable, so a method even though it reads only the shared
    // measurements.
    #[allow(clippy::unused_self)]
    pub(crate) fn metrics(&self) -> QString {
        let lines: Vec<String> = crate::metrics::results()
            .iter()
            .map(|(name, ms)| format!("{name}={ms:.1}"))
            .collect();
        QString::from(&lines.join("\n"))
    }

    pub(crate) fn status(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("{}");
        };
        let model = &core.app.model;
        let number =
            |value: Option<f64>| value.map_or_else(|| "null".to_owned(), |v| format!("{v:.3}"));
        let integer =
            |value: Option<i64>| value.map_or_else(|| "null".to_owned(), |v| v.to_string());
        let flag = |value: Option<bool>| value.map_or_else(|| "null".to_owned(), |v| v.to_string());
        let text = |value: Option<&str>| value.map_or_else(|| "null".to_owned(), json_string);
        let fields = [
            ("time", number(model.double(PropertyId::TimePos))),
            ("duration", number(model.double(PropertyId::Duration))),
            ("pause", flag(model.flag(PropertyId::Pause))),
            ("playing", model.playing().to_string()),
            ("loaded", model.media_loaded().to_string()),
            ("volume", number(model.double(PropertyId::Volume))),
            ("mute", flag(model.flag(PropertyId::Mute))),
            ("playlist_pos", integer(model.int(PropertyId::PlaylistPos))),
            (
                "playlist_count",
                integer(model.int(PropertyId::PlaylistCount)),
            ),
            ("video_width", integer(model.int(PropertyId::VideoWidth))),
            ("path", text(model.loaded_path())),
            ("current_ao", text(model.text(PropertyId::CurrentAo))),
            ("hwdec_current", text(model.text(PropertyId::HwdecCurrent))),
            ("vf", text(model.text(PropertyId::VideoFilters))),
            ("fullscreen", self.fullscreen().to_string()),
            ("maximized", core.maximized.to_string()),
            ("stretch", core.stretch.to_string()),
            (
                "playlist_attached",
                core.chrome
                    .as_ref()
                    .is_some_and(|chrome| chrome.state.playlist_attached)
                    .to_string(),
            ),
            (
                "hover",
                integer(
                    core.chrome
                        .as_ref()
                        .and_then(|chrome| chrome.state.hover)
                        .and_then(|element| i64::try_from(element).ok()),
                ),
            ),
            ("tooltip", text(Some(&self.tooltip().to_string()))),
            ("state_file", text(core.app.store().path().to_str())),
        ];
        let body: Vec<String> = fields
            .iter()
            .map(|(name, value)| format!("\"{name}\":{value}"))
            .collect();
        QString::from(&format!("{{{}}}", body.join(",")))
    }

    pub(crate) fn diagnostics(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("{}");
        };
        let mpv = core.engine.mpv();
        let count = |name: &str| {
            mpv.get_property::<i64>(name)
                .map_or_else(|_| "null".to_owned(), |value| value.to_string())
        };
        let float = |name: &str| {
            mpv.get_property::<f64>(name)
                .map_or_else(|_| "null".to_owned(), |value| format!("{value:.4}"))
        };
        let text = |name: &str| {
            mpv.get_property::<String>(name)
                .map_or_else(|_| "null".to_owned(), |value| json_string(&value))
        };
        let fields = [
            ("frame_drop_count", count("frame-drop-count")),
            (
                "decoder_frame_drop_count",
                count("decoder-frame-drop-count"),
            ),
            ("vo_delayed_frame_count", count("vo-delayed-frame-count")),
            ("mistimed_frame_count", count("mistimed-frame-count")),
            ("estimated_vf_fps", float("estimated-vf-fps")),
            ("container_fps", float("container-fps")),
            ("display_fps", float("display-fps")),
            ("estimated_display_fps", float("estimated-display-fps")),
            ("vsync_jitter", float("vsync-jitter")),
            ("current_vo", text("current-vo")),
            ("current_ao", text("current-ao")),
            ("hwdec_current", text("hwdec-current")),
            ("video_sync", text("video-sync")),
            ("vf", text("vf")),
            ("width", count("video-params/w")),
            ("height", count("video-params/h")),
        ];
        let body: Vec<String> = fields
            .iter()
            .map(|(name, value)| format!("\"{name}\":{value}"))
            .collect();
        QString::from(&format!("{{{}}}", body.join(",")))
    }

    pub(crate) fn has_option(&self, name: &QString) -> bool {
        self.rust().cli.flag(&name.to_string())
    }

    pub(crate) fn option(&self, name: &QString) -> QString {
        self.rust()
            .cli
            .value(&name.to_string())
            .map(QString::from)
            .unwrap_or_default()
    }
}
