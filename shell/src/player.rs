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
use rvp_app::binding_editor::{Editor, Scope};
use rvp_app::chrome::{Chrome, Damage, Frame, Mode};
use rvp_app::instance::Request;
use rvp_app::menu::{Command, Live, MenuItem, to_json};
use rvp_app::pointer::{Button, Context, Cursor, Effect, Focus, ListEffect, Modifiers, Pointer};
use rvp_app::preferences;
use rvp_app::{
    Action, App, Cli, Dirs, KeyChord, ListAction, MouseGesture, Settings, UiRequest,
    WindowGeometry, view,
};
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
    /// The settings window at `settings_page` (empty: the first page).
    pub const SETTINGS: i32 = 9;
    /// The file dialog, adding to the shown album.
    pub const ADD_FILES: i32 = 10;
    pub const ADD_FOLDER: i32 = 11;
    /// A folder dialog for PotPlayer's playlists.
    pub const IMPORT_PLAYLISTS: i32 = 12;
    pub const LIST_MENU: i32 = 13;
    /// Another instance opened files: bring the window up.
    pub const RAISE: i32 = 14;
    /// The album name dialog (`album_name` holds the current name);
    /// argument 1 renames the shown album.
    pub const ALBUM_NAME: i32 = 15;
    /// A skin menu or built-in submenu at the pointer (`menu_json("skin")`).
    pub const SKIN_MENU: i32 = 16;
    /// The folder dialog for Open Folder.
    pub const OPEN_FOLDER: i32 = 17;
    pub const OPEN_URL: i32 = 18;
    pub const OPEN_SUBTITLE: i32 = 19;
    /// The folder dialog for saved frames.
    pub const IMAGE_FOLDER: i32 = 20;
    /// Open `external_url` with the system (the image folder).
    pub const OPEN_EXTERNAL: i32 = 21;
    pub const ABOUT: i32 = 22;
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
    /// The command line's request, opened once the video item is ready;
    /// `None` once started.
    pending: Option<Request>,
    /// Which part of the window has the keyboard.
    focus: Focus,
    /// While playlist rows are dragged: where they would land.
    drop_before: Option<usize>,
    /// The key and mouse editor's copy of the bindings while it is open.
    editor: Option<Editor>,
    width: i32,
    height: i32,
    maximized: bool,
    stretch: bool,
    /// Resizing the window to the next file's video (`fit-window-to-video`).
    fit: Fit,
    /// True while a drain is queued (coalesces wakeups).
    drain_queued: Arc<AtomicBool>,
    /// Playback updates of the chrome held back by [`CHROME_INTERVAL`].
    throttle: Throttle,
    /// The skin menu a control asked for: its command and menu name.
    popup: Option<(&'static str, Option<&'static str>)>,
    /// The local time for the skin's clock.
    clock: String,
    /// The settings live next to the executable.
    portable: bool,
}

/// Fitting the window to a video the user opened: requested, then the new
/// file loaded (only its video size counts, not the one still playing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    No,
    Requested,
    Loaded,
}

impl Core {
    /// A file the user opens is about to play: fit the window to it.
    fn request_fit(&mut self) {
        self.fit = if self.app.settings.fit_window_to_video {
            Fit::Requested
        } else {
            Fit::No
        };
    }
}

/// When playback last updated the chrome, and whether an update waits.
#[derive(Default)]
struct Throttle {
    last: Option<Instant>,
    pending: bool,
}

/// Playback (the time and the seek bar) updates the chrome at most this
/// often; input and state changes update it at once. Repainting the bottom
/// bar with every frame of a 60 fps video cost 4K60 frames on the owner's
/// desktop.
const CHROME_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Properties that change with every frame or second of playback.
const TICKING: &[PropertyId] = &[
    PropertyId::TimePos,
    PropertyId::EstimatedVfFps,
    PropertyId::VideoBitrate,
    PropertyId::AudioBitrate,
    PropertyId::FrameDropCount,
    PropertyId::DecoderFrameDropCount,
];

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
    pub(crate) osd: QString,
    pub(crate) album_name: QString,
    pub(crate) settings_page: QString,
    pub(crate) on_top: bool,
    pub(crate) info_shown: bool,
    pub(crate) external_url: QString,
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

/// `Qt::KeyboardModifier` bits.
const QT_SHIFT: i32 = 0x0200_0000;
const QT_CONTROL: i32 = 0x0400_0000;

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

/// The application core from the user's directories (and whether they are
/// portable); on failure a temporary state directory and a message saying
/// so.
fn load_app() -> Result<(App, bool, Option<String>), String> {
    let loaded = Dirs::from_env().and_then(|dirs| {
        App::load(&dirs)
            .map(|app| (app, dirs.portable))
            .map_err(|error| format!("cannot read the settings: {error}"))
    });
    match loaded {
        Ok((app, portable)) => Ok((app, portable, None)),
        Err(message) => {
            let fallback = std::env::temp_dir().join("rvp-state");
            let app = App::new(Settings::default(), &fallback).map_err(|e| e.to_string())?;
            Ok((app, false, Some(message)))
        }
    }
}

/// A `file:` URL for a local path, for `Qt.openUrlExternally`.
fn file_url(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

/// `{"notes": [...], "restart": bool, "error": "..."}` for the settings
/// window.
fn settings_result(notes: &[String], restart: bool, error: Option<&str>) -> QString {
    let value = serde_json::json!({
        "notes": notes,
        "restart": restart,
        "error": error.unwrap_or(""),
    });
    QString::from(&value.to_string())
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
        let (mut app, portable) = match load_app() {
            Ok((app, portable, message)) => {
                messages.extend(message);
                (app, portable)
            }
            Err(error) => {
                self.show_message(&format!("cannot start: {error}"));
                return;
            }
        };
        messages.extend(app.warnings.iter().cloned());
        if cli.flag("selftest") {
            // A fresh test state must not import the owner's playlists.
            app.skip_first_import();
        }
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
        let cwd = std::env::current_dir().unwrap_or_default();
        let request = Request::from_cli(&cli, &cwd);
        self.as_mut().rust_mut().get_mut().core = Some(Core {
            app,
            engine,
            chrome,
            pointer: Pointer::default(),
            pending: Some(request),
            focus: Focus::Window,
            drop_before: None,
            editor: None,
            width: 0,
            height: 0,
            maximized: false,
            stretch: false,
            fit: Fit::No,
            drain_queued,
            throttle: Throttle::default(),
            popup: None,
            clock: String::new(),
            portable,
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
        let Some(request) = core.pending.take() else {
            return;
        };
        let plays = request.mode == rvp_app::instance::Mode::Play && !request.files.is_empty();
        if plays {
            core.request_fit();
            crate::metrics::first_frame_pending();
        } else {
            // Reopening the last file keeps the restored window size.
            core.fit = Fit::No;
        }
        // `/add` and `/insert` at start: the last state first, then theirs.
        let result = if plays || request.files.is_empty() {
            core.app.start(core.engine, &request.files)
        } else {
            core.app
                .start(core.engine, &[])
                .and_then(|()| core.app.remote(core.engine, &request))
        };
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
        for event in &drained.events {
            match event {
                EngineEvent::PlaybackRestart => crate::metrics::playback_restarted(),
                EngineEvent::FileLoaded { .. } => {
                    crate::metrics::mark_once("file_loaded_ms");
                    if core.fit == Fit::Requested {
                        core.fit = Fit::Loaded;
                    }
                }
                _ => {}
            }
        }
        let new_aspect = drained.changes.contains(PropertyId::VideoAspect) && core.fit == Fit::No;
        let fit = if core.fit == Fit::Loaded
            && !core.maximized
            && let (Some(width), Some(height)) = (
                core.app.model.int(PropertyId::VideoWidth),
                core.app.model.int(PropertyId::VideoHeight),
            ) {
            core.fit = Fit::No;
            let (chrome_width, chrome_height) =
                core.chrome.as_ref().map_or((0, 0), Chrome::chrome_size);
            match (i32::try_from(width), i32::try_from(height)) {
                (Ok(width), Ok(height)) => Some((width + chrome_width, height + chrome_height)),
                _ => None,
            }
        } else {
            None
        };
        let ticking_only = drained.events.is_empty()
            && PropertyId::ALL
                .iter()
                .filter(|id| drained.changes.contains(**id))
                .all(|id| TICKING.contains(id));
        let now = Instant::now();
        let throttle = ticking_only
            && core
                .throttle
                .last
                .is_some_and(|last| now.duration_since(last) < CHROME_INTERVAL);
        if throttle {
            core.throttle.pending = true;
        } else {
            core.throttle = Throttle {
                last: Some(now),
                pending: false,
            };
        }
        let has_video = core.app.model.int(PropertyId::VideoWidth).is_some();
        let title = view::title(&core.app.model, core.app.library().lists());
        let requests = drained.requests;
        self.as_mut().set_has_video(has_video);
        self.as_mut()
            .set_window_title(QString::from(if title.is_empty() {
                "RVP"
            } else {
                title.as_str()
            }));
        for request in requests {
            self.as_mut().handle_request(request);
        }
        if let Some((width, height)) = fit
            && !*self.fullscreen()
        {
            self.as_mut().fit_window(width, height);
        } else if new_aspect {
            self.as_mut().fit_aspect();
        }
        if !throttle {
            self.sync_chrome();
        }
    }

    /// About ten times a second: a playback update held back by
    /// [`CHROME_INTERVAL`] is shown.
    pub(crate) fn flush_chrome(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if !core.throttle.pending {
            return;
        }
        core.throttle = Throttle {
            last: Some(Instant::now()),
            pending: false,
        };
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
        let mut data = view::data_model(&core.app.model, core.app.library().lists());
        data.playlist.drop_before = core.drop_before;
        if !core.clock.is_empty() {
            data.set_text("CMD_SYSTEM_TIME_MESSAGE", core.clock.as_str());
        }
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
            let _ = core.app.apply_picture(core.engine, false);
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

    fn pointer_context(&self, modifiers: Modifiers) -> Option<Context> {
        let core = self.core()?;
        let fullscreen = *self.fullscreen();
        Some(Context {
            width: core.width,
            height: core.height,
            movable: !fullscreen && !core.maximized,
            chrome_shown: core.chrome.is_some() && (!fullscreen || *self.chrome_shown()),
            modifiers,
        })
    }

    pub(crate) fn mouse_press(
        self: Pin<&mut Self>,
        x: f64,
        y: f64,
        button_value: i32,
        modifiers: i32,
    ) {
        let modifiers = Modifiers {
            ctrl: modifiers & QT_CONTROL != 0,
            shift: modifiers & QT_SHIFT != 0,
        };
        self.pointer_event(x, y, button_value, 0, modifiers);
    }

    pub(crate) fn mouse_double(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.pointer_event(x, y, button_value, 1, Modifiers::default());
    }

    pub(crate) fn mouse_release(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.pointer_event(x, y, button_value, 2, Modifiers::default());
    }

    /// `phase`: 0 press, 1 double click, 2 release.
    fn pointer_event(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        button_value: i32,
        phase: u8,
        modifiers: Modifiers,
    ) {
        let Some(button) = button(button_value) else {
            return;
        };
        let Some(ctx) = self.pointer_context(modifiers) else {
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
                Effect::List(effect) => self.as_mut().list_effect(effect),
                Effect::Focus(focus) => {
                    if let Some(core) = self.as_mut().core_mut() {
                        core.focus = focus;
                    }
                }
                Effect::Popup { command, menu } => {
                    if let Some(core) = self.as_mut().core_mut() {
                        core.popup = Some((command, menu));
                    }
                    self.as_mut().ui_request(request::SKIN_MENU, 0);
                }
            }
        }
    }

    /// The rows the playlist panel shows (paging, scrolling).
    fn visible_rows(core: &Core) -> usize {
        core.chrome
            .as_ref()
            .and_then(Chrome::list_geometry)
            .map_or(1, |geometry| geometry.visible.max(1))
    }

    fn list_effect(mut self: Pin<&mut Self>, effect: ListEffect) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        match effect {
            ListEffect::DropMarker(marker) => core.drop_before = marker,
            ListEffect::Menu => self.ui_request(request::LIST_MENU, 0),
            effect => {
                if matches!(effect, ListEffect::Play(_)) {
                    crate::metrics::input("input_ms");
                    core.request_fit();
                }
                let visible = Self::visible_rows(core);
                if let Some(request) = core.app.list_effect(core.engine, effect, visible) {
                    self.handle_request(request);
                }
            }
        }
    }

    /// Carries out a playlist action; `extend` (Shift) selects while the
    /// cursor moves.
    fn perform_list(mut self: Pin<&mut Self>, action: ListAction, extend: bool) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if action == ListAction::Play {
            core.request_fit();
        }
        let visible = Self::visible_rows(core);
        if let Some(request) = core.app.list_action(core.engine, action, extend, visible) {
            self.handle_request(request);
        }
    }

    pub(crate) fn seek_fraction(mut self: Pin<&mut Self>, fraction: f64) {
        if let Some(core) = self.as_mut().core_mut() {
            crate::metrics::seek_issued();
            core.app.seek_to_fraction(core.engine, fraction);
        }
    }

    pub(crate) fn mouse_move(mut self: Pin<&mut Self>, x: f64, y: f64) {
        let Some(ctx) = self.pointer_context(Modifiers::default()) else {
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

    pub(crate) fn mouse_wheel(mut self: Pin<&mut Self>, x: f64, y: f64, delta: i32) {
        let Some(ctx) = self.pointer_context(Modifiers::default()) else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let effects = match core.chrome.as_ref() {
            Some(chrome) => {
                core.pointer
                    .wheel(chrome, &core.app.bindings, ctx, (pixel(x), pixel(y)), delta)
            }
            // Without a skin the whole window is video.
            None => core
                .app
                .bindings
                .mouse(if delta > 0 {
                    MouseGesture::WheelUp
                } else {
                    MouseGesture::WheelDown
                })
                .filter(|_| delta != 0)
                .map(Effect::Action)
                .into_iter()
                .collect(),
        };
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
        // The playlist's keys while it has the keyboard; Shift with a cursor
        // key selects as the cursor moves.
        let list_shown = core
            .chrome
            .as_ref()
            .is_some_and(|chrome| chrome.state.playlist_attached);
        if core.focus == Focus::Playlist && list_shown {
            let list_keys = &core.app.bindings.list_keys;
            let list_action = chords.iter().find_map(|chord| {
                list_keys
                    .get(chord)
                    .map(|action| (*action, false))
                    .or_else(|| {
                        let plain = KeyChord {
                            shift: false,
                            ..*chord
                        };
                        list_keys
                            .get(&plain)
                            .filter(|action| chord.shift && action.is_cursor())
                            .map(|action| (*action, true))
                    })
            });
            if let Some((action, extend)) = list_action {
                crate::metrics::input("input_ms");
                self.as_mut().perform_list(action, extend);
                self.sync_chrome();
                return true;
            }
        }
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

    /// Carries out an action and the window request it may produce.
    fn perform(mut self: Pin<&mut Self>, action: Action) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if matches!(action, Action::SeekBackward(_) | Action::SeekForward(_)) {
            crate::metrics::seek_issued();
        }
        if matches!(action, Action::Previous | Action::Next) {
            core.request_fit();
        }
        let Some(request) = core.app.dispatch(core.engine, action) else {
            return;
        };
        self.handle_request(request);
    }

    /// Carries out what the core asks the window for.
    fn handle_request(mut self: Pin<&mut Self>, request: UiRequest) {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        match request {
            UiRequest::OpenFileDialog => self.ui_request(request::OPEN_FILES, 0),
            UiRequest::ToggleFullscreen { stretch } => {
                let enter = !fullscreen;
                core.stretch = enter && stretch;
                let _ = core.app.apply_picture(core.engine, core.stretch);
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
            UiRequest::Preferences(page) => {
                self.as_mut()
                    .set_settings_page(QString::from(page.unwrap_or("")));
                self.ui_request(request::SETTINGS, 0);
            }
            UiRequest::About => self.ui_request(request::ABOUT, 0),
            UiRequest::OpenFolderDialog => self.ui_request(request::OPEN_FOLDER, 0),
            UiRequest::OpenUrlDialog => self.ui_request(request::OPEN_URL, 0),
            UiRequest::OpenSubtitleDialog => self.ui_request(request::OPEN_SUBTITLE, 0),
            UiRequest::ImageFolderDialog => self.ui_request(request::IMAGE_FOLDER, 0),
            UiRequest::OpenExternal(path) => {
                self.as_mut()
                    .set_external_url(QString::from(&file_url(&path)));
                self.ui_request(request::OPEN_EXTERNAL, 0);
            }
            UiRequest::SaveFrame { path, subtitles } => self.save_frame(path, subtitles),
            UiRequest::ScaleWindow(factor) => self.scale_window(factor),
            UiRequest::FitAspect => self.fit_aspect_now(true),
            UiRequest::OnTop(on) => self.set_on_top(on),
            UiRequest::ShowInfo(shown) => self.set_info_shown(shown),
            UiRequest::ImportPlaylists => match core.app.potplayer_folder() {
                Some(folder) => self.import_from(&folder),
                None => self.ui_request(request::IMPORT_PLAYLISTS, 0),
            },
            UiRequest::AddFilesDialog => self.ui_request(request::ADD_FILES, 0),
            UiRequest::AddFolderDialog => self.ui_request(request::ADD_FOLDER, 0),
            UiRequest::AlbumName { rename, current } => {
                self.as_mut().set_album_name(QString::from(&current));
                self.ui_request(request::ALBUM_NAME, i32::from(rename));
            }
            UiRequest::Osd(text) => self.set_osd(QString::from(&text)),
            UiRequest::Message(text) => self.show_message(&text),
        }
    }

    /// Saves the frame on a thread: mpv encodes the image, which takes
    /// longer than a frame.
    fn save_frame(self: Pin<&mut Self>, path: PathBuf, subtitles: bool) {
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name("rvp-frame".to_owned())
            .spawn(move || {
                let mode = if subtitles { "subtitles" } else { "video" };
                let text = path.to_string_lossy().into_owned();
                let result = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .map_err(|error| error.to_string())
                    .and_then(|()| {
                        ENGINE
                            .get()
                            .ok_or_else(|| "no player".to_owned())?
                            .command("screenshot-to-file", &[&text, mode])
                            .map_err(|error| error.to_string())
                    });
                let notice = match result {
                    Ok(()) => format!("Saved {}", view::file_name(&text)),
                    Err(error) => format!("Cannot save the frame: {error}"),
                };
                let _ = thread.queue(move |player| player.set_osd(QString::from(&notice)));
            });
        if let Err(error) = spawned {
            eprintln!("rvp: cannot save the frame: {error}");
        }
    }

    /// The window at `factor` times the video's size (PotPlayer's "1.0x
    /// video output size"), within the screen.
    fn scale_window(mut self: Pin<&mut Self>, factor: f64) {
        if *self.fullscreen() {
            return;
        }
        let Some(core) = self.core() else {
            return;
        };
        let model = &core.app.model;
        let (Some(height), Some(aspect)) = (
            model.int(PropertyId::VideoHeight),
            model.double(PropertyId::VideoAspect),
        ) else {
            return;
        };
        let (chrome_width, chrome_height) =
            core.chrome.as_ref().map_or((0, 0), Chrome::chrome_size);
        // Video sizes are far inside i32 and f64's exact range.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let (width, height) = (
            (height as f64 * aspect * factor).round() as i32,
            (height as f64 * factor).round() as i32,
        );
        if core.maximized {
            self.as_mut().ui_request(request::MAXIMIZE_RESTORE, 0);
        }
        self.fit_window(width + chrome_width, height + chrome_height);
    }

    fn import_from(mut self: Pin<&mut Self>, folder: &Path) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let text = match core.app.import_playlists(folder) {
            Ok(0) => format!("No PotPlayer playlists in {}", folder.display()),
            Ok(count) => format!("Imported {count} albums from PotPlayer"),
            Err(error) => format!("Cannot import {}: {error}", folder.display()),
        };
        self.as_mut().show_message(&text);
        self.sync_chrome();
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
                pending.files.clear();
            }
            pending.files.extend(files);
            return;
        }
        // Appended files do not play: the window keeps its size.
        if !append {
            core.request_fit();
        }
        let result = core.app.open(core.engine, &files, append);
        if let Err(error) = result {
            self.show_message(&format!("cannot open: {error}"));
        }
    }

    pub(crate) fn run_list_action(mut self: Pin<&mut Self>, name: &QString) {
        if let Some(action) = ListAction::from_name(&name.to_string()) {
            crate::metrics::input("input_ms");
            self.as_mut().perform_list(action, false);
            self.sync_chrome();
        }
    }

    pub(crate) fn menu_json(&self, kind: &QString) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        // Tracks and chapters are read from mpv now, once per menu.
        let live = if core.app.model.media_loaded() {
            Live {
                tracks: core.engine.tracks(),
                chapters: core.engine.chapters(),
                chapter: core.app.model.int(PropertyId::Chapter),
            }
        } else {
            Live::default()
        };
        let items = match kind.to_string().as_str() {
            "context" => Some(core.app.context_menu(&live)),
            "playlist" => Some(core.app.playlist_menu(&live)),
            "skin" => match core.popup {
                Some(("CMD_POPUP_USER_MENU", Some(name))) => skin().and_then(|skin| {
                    skin.user_menus()
                        .iter()
                        .find(|menu| menu.name == name)
                        .map(|menu| core.app.skin_menu(menu, &live))
                }),
                Some((command, _)) => core.app.popup(command, &live).map(|(_, items)| items),
                None => None,
            },
            _ => None,
        };
        let items = items.unwrap_or_else(|| {
            vec![MenuItem {
                label: "Not in RVP yet".to_owned(),
                ..MenuItem::default()
            }]
        });
        QString::from(&to_json(&items))
    }

    pub(crate) fn run_menu_command(mut self: Pin<&mut Self>, id: &QString) {
        let Some(command) = Command::parse(&id.to_string()) else {
            return;
        };
        crate::metrics::input("input_ms");
        match command {
            Command::Window(action) => self.as_mut().perform(action),
            Command::List(action) => self.as_mut().perform_list(action, false),
            other => {
                let Some(core) = self.as_mut().core_mut() else {
                    return;
                };
                if matches!(other, Command::Album(_)) {
                    core.request_fit();
                }
                let visible = Self::visible_rows(core);
                if let Some(request) = core.app.run_menu_command(core.engine, other, visible) {
                    self.as_mut().handle_request(request);
                }
            }
        }
        self.sync_chrome();
    }

    pub(crate) fn settings_view(&self, devices: bool) -> QString {
        let Some(core) = self.core() else {
            return QString::from("{}");
        };
        let view = core.app.settings_view(core.engine, core.portable, devices);
        QString::from(&view.to_string())
    }

    pub(crate) fn set_setting(
        mut self: Pin<&mut Self>,
        path: &QString,
        value: &QString,
    ) -> QString {
        let Some(core) = self.as_mut().core_mut() else {
            return settings_result(&[], false, Some("not started"));
        };
        let value = match serde_json::from_str(&value.to_string()) {
            Ok(value) => value,
            Err(error) => return settings_result(&[], false, Some(&error.to_string())),
        };
        let (changed, mut notes) =
            match preferences::change(&core.app.settings, &path.to_string(), value) {
                Ok(changed) => changed,
                Err(error) => return settings_result(&[], false, Some(&error)),
            };
        let applied = core.app.apply_settings(core.engine, changed);
        notes.extend(applied.notes);
        notes.dedup();
        self.as_mut().settings_applied();
        settings_result(&notes, applied.restart, None)
    }

    pub(crate) fn import_settings(mut self: Pin<&mut Self>, path: &QString) -> QString {
        let Some(core) = self.as_mut().core_mut() else {
            return settings_result(&[], false, Some("not started"));
        };
        let (settings, mut notes) = match preferences::import(Path::new(&path.to_string())) {
            Ok(imported) => imported,
            Err(error) => return settings_result(&[], false, Some(&error)),
        };
        let applied = core.app.apply_settings(core.engine, settings);
        notes.extend(applied.notes);
        notes.dedup();
        self.as_mut().settings_applied();
        settings_result(&notes, applied.restart, None)
    }

    pub(crate) fn export_settings(&self, path: &QString) -> QString {
        let Some(core) = self.core() else {
            return settings_result(&[], false, Some("not started"));
        };
        let path = PathBuf::from(path.to_string());
        match preferences::export(&core.app.settings, &path) {
            Ok(()) => settings_result(&[format!("Saved {}", path.display())], false, None),
            Err(error) => settings_result(&[], false, Some(&error.to_string())),
        }
    }

    /// After a settings change: the window state and the chrome follow.
    fn settings_applied(mut self: Pin<&mut Self>) {
        let on_top = self
            .as_mut()
            .core_mut()
            .and_then(|core| core.app.on_top_change());
        if let Some(on_top) = on_top {
            self.as_mut().set_on_top(on_top);
        }
        self.sync_chrome();
    }

    pub(crate) fn open_folder(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let folder = PathBuf::from(path.to_string());
        core.request_fit();
        match core.app.open_folder(core.engine, &folder) {
            Ok(0) => {
                let text = format!("No media files in {}", folder.display());
                self.as_mut().show_message(&text);
            }
            Ok(_) => {}
            Err(error) => self.as_mut().show_message(&format!("cannot open: {error}")),
        }
        self.sync_chrome();
    }

    pub(crate) fn load_subtitle(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(request) = core
            .app
            .load_subtitle(core.engine, Path::new(&path.to_string()))
        {
            self.handle_request(request);
        }
    }

    pub(crate) fn set_image_folder(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let folder = PathBuf::from(path.to_string());
        core.app.set_image_folder(&folder);
        self.set_osd(QString::from(&format!("Images go to {}", folder.display())));
    }

    pub(crate) fn about_text(&self) -> QString {
        let mpv = self
            .core()
            .and_then(|core| core.engine.get_text("mpv-version"))
            .unwrap_or_default();
        QString::from(&format!(
            "RVP {}\nA PotPlayer-style media player\n{mpv}",
            env!("CARGO_PKG_VERSION")
        ))
    }

    pub(crate) fn info_text(&self) -> QString {
        self.core().map_or_else(QString::default, |core| {
            QString::from(&view::info_text(&core.app.model))
        })
    }

    pub(crate) fn set_clock(mut self: Pin<&mut Self>, text: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let text = text.to_string();
        if core.clock != text {
            core.clock = text;
            self.sync_chrome();
        }
    }

    pub(crate) fn add_folder(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let folder = PathBuf::from(path.to_string());
        let count = core.app.add_folder(&folder);
        if count == 0 {
            let text = format!("No media files in {}", folder.display());
            self.as_mut().show_message(&text);
        }
        self.sync_chrome();
    }

    pub(crate) fn name_album(mut self: Pin<&mut Self>, name: &QString, rename: bool) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.name_album(&name.to_string(), rename);
        }
        self.sync_chrome();
    }

    pub(crate) fn import_playlists(self: Pin<&mut Self>, folder: &QString) {
        let folder = folder.to_string();
        if !folder.is_empty() {
            self.import_from(Path::new(&folder));
        }
    }

    pub(crate) fn remote_message(mut self: Pin<&mut Self>, message: &QString) {
        let Some(request) = Request::decode(&message.to_string()) else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(pending) = core.pending.as_mut() {
            // Not started yet: the files join the command line's.
            pending.files.extend(request.files);
            return;
        }
        if request.mode == rvp_app::instance::Mode::Play && !request.files.is_empty() {
            core.request_fit();
            self.as_mut().ui_request(request::RAISE, 0);
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Err(error) = core.app.remote(core.engine, &request) {
            self.as_mut().show_message(&format!("cannot open: {error}"));
        }
        self.sync_chrome();
    }

    pub(crate) fn fit_aspect(self: Pin<&mut Self>) {
        self.fit_aspect_now(false);
    }

    /// The window takes the video's shape; `force` also when the setting is
    /// off (Window Size > Fit to video aspect ratio).
    fn fit_aspect_now(mut self: Pin<&mut Self>, force: bool) {
        if *self.fullscreen() {
            return;
        }
        let Some(core) = self.core() else {
            return;
        };
        let aspect = core.app.model.double(PropertyId::VideoAspect);
        let (Some(chrome), Some(aspect)) = (core.chrome.as_ref(), aspect) else {
            return;
        };
        if core.maximized
            || !(force || core.app.settings.keep_aspect_window)
            || !core.app.model.media_loaded()
        {
            return;
        }
        let window = (core.width, core.height);
        let fitted = rvp_app::chrome::fit_aspect(window, chrome.chrome_size(), aspect);
        if fitted != window {
            self.as_mut().fit_window(fitted.0, fitted.1);
        }
    }

    pub(crate) fn editor_open(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.editor = Some(Editor::new(&core.app.bindings));
        }
    }

    pub(crate) fn editor_rows(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        let Some(editor) = core.editor.as_ref() else {
            return QString::from("[]");
        };
        let rows: Vec<String> = editor
            .rows()
            .iter()
            .map(|row| {
                let label = Self::action_label(core, row.scope, &row.name);
                let keys: Vec<String> = row
                    .keys
                    .iter()
                    .map(|chord| json_string(&chord.to_string()))
                    .collect();
                let gestures: Vec<String> = row
                    .gestures
                    .iter()
                    .map(|gesture| json_string(gesture.name()))
                    .collect();
                format!(
                    "{{\"scope\":{},\"name\":{},\"label\":{},\"keys\":[{}],\"gestures\":[{}]}}",
                    json_string(row.scope.name()),
                    json_string(&row.name),
                    json_string(&label),
                    keys.join(","),
                    gestures.join(",")
                )
            })
            .collect();
        QString::from(&format!("[{}]", rows.join(",")))
    }

    // A QML invokable, so a method even though the list is fixed.
    #[allow(clippy::unused_self)]
    pub(crate) fn editor_gestures(&self) -> QString {
        let names: Vec<String> = MouseGesture::all()
            .map(|gesture| json_string(gesture.name()))
            .collect();
        QString::from(&format!("[{}]", names.join(",")))
    }

    /// PotPlayer's label of an action of a scope.
    fn action_label(core: &Core, scope: Scope, name: &str) -> String {
        match scope {
            Scope::Window => Action::from_name(name)
                .map(|action| core.app.menu_entry(action).0)
                .unwrap_or_default(),
            Scope::Playlist => ListAction::from_name(name)
                .map(|action| core.app.list_menu_entry(action).0)
                .unwrap_or_default(),
        }
    }

    pub(crate) fn editor_key_owner(&self, scope: &QString, key: i32, modifiers: i32) -> QString {
        let (Some(core), Some(scope)) = (self.core(), Scope::from_name(&scope.to_string())) else {
            return QString::from("{}");
        };
        let (Some(editor), Some(chord)) = (
            core.editor.as_ref(),
            KeyChord::from_qt(key, modifiers).into_iter().next(),
        ) else {
            return QString::from("{}");
        };
        let owner = editor.key_owner(scope, &chord).unwrap_or("");
        QString::from(&format!(
            "{{\"key\":{},\"owner\":{},\"label\":{}}}",
            json_string(&chord.to_string()),
            json_string(owner),
            json_string(&Self::action_label(core, scope, owner))
        ))
    }

    pub(crate) fn editor_gesture_owner(&self, gesture: &QString) -> QString {
        let Some(core) = self.core() else {
            return QString::default();
        };
        let owner = core.editor.as_ref().and_then(|editor| {
            editor.gesture_owner(MouseGesture::from_name(&gesture.to_string())?)
        });
        owner.map_or_else(QString::default, |name| {
            QString::from(&Self::action_label(core, Scope::Window, name))
        })
    }

    fn editor_mut(self: Pin<&mut Self>) -> Option<&mut Editor> {
        self.core_mut().and_then(|core| core.editor.as_mut())
    }

    pub(crate) fn editor_bind_key(
        self: Pin<&mut Self>,
        scope: &QString,
        action: &QString,
        key: i32,
        modifiers: i32,
    ) -> QString {
        let (Some(scope), Some(chord)) = (
            Scope::from_name(&scope.to_string()),
            KeyChord::from_qt(key, modifiers).into_iter().next(),
        ) else {
            return QString::from("{}");
        };
        let Some(editor) = self.editor_mut() else {
            return QString::from("{}");
        };
        let displaced = editor
            .bind_key(scope, &action.to_string(), chord)
            .unwrap_or("");
        QString::from(&format!(
            "{{\"key\":{},\"displaced\":{}}}",
            json_string(&chord.to_string()),
            json_string(displaced)
        ))
    }

    pub(crate) fn editor_unbind_key(self: Pin<&mut Self>, scope: &QString, key: &QString) {
        let (Some(scope), Some(chord)) = (
            Scope::from_name(&scope.to_string()),
            KeyChord::parse(&key.to_string()),
        ) else {
            return;
        };
        if let Some(editor) = self.editor_mut() {
            editor.unbind_key(scope, &chord);
        }
    }

    pub(crate) fn editor_bind_gesture(
        self: Pin<&mut Self>,
        action: &QString,
        gesture: &QString,
    ) -> QString {
        let Some(gesture) = MouseGesture::from_name(&gesture.to_string()) else {
            return QString::default();
        };
        let Some(editor) = self.editor_mut() else {
            return QString::default();
        };
        QString::from(
            editor
                .bind_gesture(gesture, &action.to_string())
                .unwrap_or(""),
        )
    }

    pub(crate) fn editor_unbind_gesture(self: Pin<&mut Self>, gesture: &QString) {
        let Some(gesture) = MouseGesture::from_name(&gesture.to_string()) else {
            return;
        };
        if let Some(editor) = self.editor_mut() {
            editor.unbind_gesture(gesture);
        }
    }

    pub(crate) fn editor_reset(self: Pin<&mut Self>, scope: &QString, action: &QString) {
        let Some(scope) = Scope::from_name(&scope.to_string()) else {
            return;
        };
        if let Some(editor) = self.editor_mut() {
            editor.reset(scope, &action.to_string());
        }
    }

    pub(crate) fn editor_reset_all(self: Pin<&mut Self>) {
        if let Some(editor) = self.editor_mut() {
            editor.reset_all();
        }
    }

    pub(crate) fn editor_close(mut self: Pin<&mut Self>, save: bool) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(editor) = core.editor.take()
            && save
        {
            core.app.set_bindings(editor.bindings);
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

    pub(crate) fn list_rect(&self, what: &QString, index: i32) -> QRectF {
        let geometry = self
            .core()
            .and_then(|core| core.chrome.as_ref())
            .and_then(Chrome::list_geometry);
        let (Some(geometry), Ok(index)) = (geometry, usize::try_from(index)) else {
            return QRectF::default();
        };
        let rect = match what.to_string().as_str() {
            "row" => geometry.row(index),
            "tab" => geometry.tabs.get(index).copied().unwrap_or_default(),
            "area" => geometry.area,
            _ => rvp_skin::Rect::default(),
        };
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
            ("video_width", integer(model.int(PropertyId::VideoWidth))),
            (
                "frame_drop_count",
                integer(model.int(PropertyId::FrameDropCount)),
            ),
            (
                "decoder_frame_drop_count",
                integer(model.int(PropertyId::DecoderFrameDropCount)),
            ),
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
            .into_iter()
            .chain(self.playlist_status(core))
            .map(|(name, value)| format!("\"{name}\":{value}"))
            .collect();
        QString::from(&format!("{{{}}}", body.join(",")))
    }

    /// The albums, the panel and the M3 settings for [`Self::status`].
    fn playlist_status(&self, core: &Core) -> Vec<(&'static str, String)> {
        let model = &core.app.model;
        let number =
            |value: Option<f64>| value.map_or_else(|| "null".to_owned(), |v| format!("{v:.3}"));
        let integer =
            |value: Option<i64>| value.map_or_else(|| "null".to_owned(), |v| v.to_string());
        let text = |value: Option<&str>| value.map_or_else(|| "null".to_owned(), json_string);
        let index = |value: usize| i64::try_from(value).ok();
        let lists = core.app.library().lists();
        let shown = core.app.library().shown();
        let playing = lists.playing.and_then(|album| lists.albums.get(album));
        vec![
            // The album that plays is the playlist (M3).
            (
                "playlist_pos",
                integer(playing.and_then(|a| a.current).and_then(index)),
            ),
            (
                "playlist_count",
                integer(playing.and_then(|a| index(a.entries.len()))),
            ),
            ("albums", integer(index(lists.albums.len()))),
            ("shown_album", integer(index(lists.shown))),
            ("playing_album", integer(lists.playing.and_then(index))),
            ("shown_entries", integer(index(shown.entries.len()))),
            ("selected", integer(index(shown.selected.len()))),
            (
                "cursor",
                integer(core.app.library().cursor(lists.shown).and_then(index)),
            ),
            (
                "shown_first",
                text(shown.entries.first().map(rvp_app::playlist::Entry::label)),
            ),
            ("top", integer(index(shown.top))),
            ("can_undo", core.app.library().can_undo().to_string()),
            (
                "focus",
                text(Some(match core.focus {
                    Focus::Window => "window",
                    Focus::Playlist => "playlist",
                })),
            ),
            ("drop_before", integer(core.drop_before.and_then(index))),
            (
                "normalizer",
                core.app.settings.normalizer.enabled.to_string(),
            ),
            ("boost", core.app.settings.normalizer.boost.to_string()),
            ("af", text(model.text(PropertyId::AudioFilters))),
            (
                "video_aspect",
                number(model.double(PropertyId::VideoAspect)),
            ),
            ("editor_open", core.editor.is_some().to_string()),
            ("osd", text(Some(&self.osd().to_string()))),
            ("volume_step", number(Some(core.app.settings.volume_step))),
            ("speed", number(model.double(PropertyId::Speed))),
            ("sub_scale", number(model.double(PropertyId::SubScale))),
            ("on_top", self.on_top().to_string()),
            ("info_shown", self.info_shown().to_string()),
            ("current_page", {
                let page = core.chrome.as_ref().map(|chrome| chrome.state.current_page);
                integer(page.map(i64::from))
            }),
        ]
    }

    /// Reads mpv's properties synchronously: call it rarely (each read waits
    /// for mpv's core, and on the GUI thread that costs video frames).
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
