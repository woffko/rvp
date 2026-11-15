//! `Player`: the `QObject` behind the window. It owns the application core,
//! keeps the chrome in step with the engine, turns input into actions and
//! asks QML for what only the window can do.
//!
//! Threading (docs/PLAN.md section 5): mpv's wakeup callback only queues
//! [`qobject::Player::drain`] onto the Qt thread; the render thread is told
//! to repaint through the video item. Chrome repaints go through one path
//! (the `damaged` signal and `Tools.repaint`), which applies them with mpv's
//! next video frame while a video plays (goal C rule 1).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
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
use rvp_skin::{Skin, SkinArchive};

use crate::player_qobject::qobject;
use crate::video::ENGINE;

mod extras;
mod m11;
mod m12;
mod m13;
mod m14;
mod m15;
mod m16;
mod m9;

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
    /// The settings window at `settings_page` (empty: the first page, when
    /// the window showed none before).
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
    /// Let the window system move the control panel's window.
    pub const PANEL_MOVE: i32 = 23;
    /// The save dialog for the current frame (`frame_path` suggests the
    /// file).
    pub const SAVE_FRAME_AS: i32 = 24;
    /// The font dialog for the subtitles (`subtitle_font` is the current
    /// family).
    pub const SUBTITLE_FONT: i32 = 25;
    /// The Adjust Audio Sync dialog; argument: the sync in milliseconds.
    pub const AUDIO_SYNC: i32 = 26;
    /// The save dialog for an audio recording (`record_path` suggests it).
    pub const RECORD_AS: i32 = 27;
    /// The folder dialog for audio recordings.
    pub const RECORD_FOLDER: i32 = 28;
    /// An M11 dialog (`PlaybackDialogs.qml`) described by `dialog` (JSON).
    pub const DIALOG: i32 = 29;
    /// Put `dialog` (text) on the clipboard.
    pub const COPY_TEXT: i32 = 30;
    /// Read the clipboard for `Player.paste_text`; argument: the purpose.
    pub const READ_CLIPBOARD: i32 = 31;
    /// Show the file `dialog` (a path) in the file manager.
    pub const SHOW_FILE: i32 = 32;
    /// The scene browser of `dialog` (JSON, `rvp_app::scenes::Browser`).
    pub const SCENES: i32 = 33;
    /// An M12 dialog (`VideoDialogs.qml`) described by `dialog` (JSON).
    pub const VIDEO_DIALOG: i32 = 34;
    /// A window size or move (`dialog`: `rvp_app::window::WindowRequest`
    /// as JSON).
    pub const WINDOW: i32 = 35;
    /// The slideshow moves by the argument's slides.
    pub const SLIDE: i32 = 36;
    /// The Adjust Subtitle Sync dialog (argument: the sync in ms).
    pub const SUBTITLE_SYNC: i32 = 37;
    /// The subtitle browser (argument 0) or Input Subtitles (1).
    pub const SUBTITLE_EDITOR: i32 = 38;
    /// The colour theme's Customize dialog.
    pub const COLOR_THEME: i32 = 39;
    /// The Playback/System Info window (M14, `SystemInfo.qml`).
    pub const SYSTEM_INFO: i32 = 40;
    /// The dropped files' menu (M14); `dialog` holds the paths.
    pub const DROP_MENU: i32 = 41;
    /// Let one of the skin's other windows (M9) move its docked group;
    /// the argument is its index.
    pub const EXTRA_MOVE: i32 = 42;
    /// Let one resize itself: its index * 100 + `Qt::Edges` bits.
    pub const EXTRA_RESIZE: i32 = 43;
}

/// The skin the main window shows (the painters read it on the render
/// thread). M14 switches it while RVP runs: another skin, the audio skin
/// or a colour theme.
static SKIN: RwLock<Option<&'static Skin>> = RwLock::new(None);

/// One skin as loaded: where it came from, its file and its tint.
#[derive(Debug, Clone, PartialEq)]
struct SkinKey {
    source: SkinSource,
    file: String,
    tint: Option<rvp_skin::theme::Tint>,
    /// The skin's touch layout (`SkinType=Touch`, M14).
    touch: bool,
}

/// Every skin loaded so far. Each lives for the process (layouts and the
/// render thread borrow it), so a switch back costs nothing and the leak
/// is bounded by the skins, modes and themes the user picks.
static LOADED: Mutex<Vec<(SkinKey, &'static Skin)>> = Mutex::new(Vec::new());
/// The frame the painters draw.
static FRAME: Mutex<Option<Arc<Frame>>> = Mutex::new(None);

pub fn skin() -> Option<&'static Skin> {
    SKIN.read().ok().and_then(|slot| *slot)
}

/// The skin the painters draw from now on.
fn show_skin(skin: &'static Skin) {
    if let Ok(mut slot) = SKIN.write() {
        *slot = Some(skin);
    }
}

/// A loaded skin by its key, or `None`.
fn loaded_skin(key: &SkinKey) -> Option<&'static Skin> {
    LOADED
        .lock()
        .ok()?
        .iter()
        .find(|(loaded, _)| loaded == key)
        .map(|(_, skin)| *skin)
}

/// VLC themes' font files (M9) by theme and name: `Some` waits for the
/// window's thread, `None` is Qt's. Qt's font database takes them only once
/// the application exists, and the first skin loads before it does, on a
/// thread of its own.
static FONTS: Mutex<ThemeFonts> = Mutex::new(BTreeMap::new());

type ThemeFonts = BTreeMap<(PathBuf, String), Option<Vec<u8>>>;

/// Keeps `skin` for the process under `key` (on the window's thread), with
/// the fonts its theme brought.
fn keep_skin(key: SkinKey, skin: Skin) -> &'static Skin {
    if let Ok(mut fonts) = FONTS.lock() {
        for ((theme, name), data) in fonts.iter_mut() {
            if let Some(data) = data.take()
                && !crate::ffi::bridge::rvp_add_font(&data)
            {
                rvp_app::log::report(
                    rvp_app::log::Level::Warning,
                    &format!("cannot use the font {name} of {}", theme.display()),
                );
            }
        }
    }
    let skin: &'static Skin = Box::leak(Box::new(skin));
    if let Ok(mut loaded) = LOADED.lock() {
        loaded.push((key, skin));
    }
    skin
}

/// The command line's files, or the screen saver's (M14), muted when its
/// page says so.
fn start_request(cli: &Cli, app: &mut App, engine: &Engine) -> Request {
    let mut request = Request::from_cli(cli, &std::env::current_dir().unwrap_or_default());
    if cli.flag("screensaver") {
        app.start_screensaver(&mut request);
        if app.settings.screensaver.mute {
            drop(engine.set_flag("mute", true));
        }
    }
    request
}

/// The command line's or `RVP_SKIN`'s skin stays until the user picks one
/// (M14).
fn command_line_skin(cli: &Cli, app: &mut App) {
    match skin_source(cli, &app.settings) {
        SkinSource::File(path) if app.settings.skin.as_ref() != Some(&path) => {
            app.set_skin_override(Some(path));
        }
        SkinSource::Default if app.settings.skin.is_some() => app.set_skin_override(None),
        _ => {}
    }
}

/// The skin of `source`'s `file` (`None`: its video skin), tinted when
/// asked: loaded once, then kept.
fn skin_variant(
    source: &SkinSource,
    file: Option<&str>,
    tint: Option<rvp_skin::theme::Tint>,
    touch: bool,
) -> Result<&'static Skin, String> {
    let base_key = |file: &str| SkinKey {
        source: source.clone(),
        file: file.to_owned(),
        tint: None,
        touch,
    };
    let base = if let Some(file) = file {
        if let Some(skin) = loaded_skin(&base_key(file)) {
            skin
        } else {
            keep_skin(base_key(file), open_skin_file(source, Some(file), touch)?)
        }
    } else {
        let found = LOADED.lock().ok().and_then(|loaded| {
            loaded
                .iter()
                .find(|(key, _)| {
                    key.source == *source
                        && key.tint.is_none()
                        && key.touch == touch
                        && rvp_skin::skin::VIDEO_SKIN_FILES.contains(&key.file.as_str())
                })
                .map(|(_, skin)| *skin)
        });
        if let Some(skin) = found {
            skin
        } else {
            let skin = open_skin_file(source, None, touch)?;
            keep_skin(base_key(skin.file()), skin)
        }
    };
    let Some(tint) = tint else {
        return Ok(base);
    };
    let key = SkinKey {
        tint: Some(tint),
        ..base_key(base.file())
    };
    Ok(loaded_skin(&key).unwrap_or_else(|| keep_skin(key, base.tinted(tint))))
}

pub fn frame() -> Option<Arc<Frame>> {
    FRAME.lock().ok().and_then(|frame| frame.clone())
}

fn publish(frame: Arc<Frame>) {
    if let Ok(mut slot) = FRAME.lock() {
        *slot = Some(frame);
    }
}

/// The frame the control panel's painter draws.
static PANEL_FRAME: Mutex<Option<Arc<Frame>>> = Mutex::new(None);

/// The first `SkinView` surface of the skin's other windows (M9).
pub const EXTRA_SURFACE: i32 = extras::SURFACE;

/// The frame of the skin's other window `index` (M9).
pub fn extra_frame(index: usize) -> Option<Arc<Frame>> {
    extras::frame(index)
}

pub fn panel_frame() -> Option<Arc<Frame>> {
    PANEL_FRAME.lock().ok().and_then(|frame| frame.clone())
}

/// Another RVP ran when this one started with "Multiple instances" (M14):
/// with "Use non-overlapping multi instances" its window moves aside.
pub static OTHER_INSTANCE: AtomicBool = AtomicBool::new(false);

/// The main window's and the control panel's skin zoom in percent (M14,
/// Zoom Skinned Windows): their chromes lay out at the window's size over
/// it, their painters scale by it, and their coordinates cross it.
static ZOOM: AtomicU32 = AtomicU32::new(100);
static PANEL_ZOOM: AtomicU32 = AtomicU32::new(100);

pub fn zoom() -> f64 {
    f64::from(ZOOM.load(Ordering::Relaxed).max(10)) / 100.0
}

pub fn panel_zoom() -> f64 {
    f64::from(PANEL_ZOOM.load(Ordering::Relaxed).max(10)) / 100.0
}

/// A main window chrome rectangle in the window's coordinates.
fn to_window(rect: rvp_skin::Rect) -> QRectF {
    let zoom = zoom();
    QRectF::new(
        f64::from(rect.x) * zoom,
        f64::from(rect.y) * zoom,
        f64::from(rect.w) * zoom,
        f64::from(rect.h) * zoom,
    )
}

/// A window coordinate in the chrome's (`zoom`: the main window's or the
/// panel's).
fn chrome_pixel(value: f64, zoom: f64) -> i32 {
    pixel(value / zoom)
}

/// A window size in the chrome's pixels.
fn chrome_size(width: i32, height: i32) -> (i32, i32) {
    let zoom = zoom();
    let scale = |value: i32| {
        // Window sizes are far inside i32.
        #[allow(clippy::cast_possible_truncation)]
        let scaled = (f64::from(value) / zoom).round() as i32;
        scaled
    };
    (scale(width), scale(height))
}

/// A chrome length in the window's pixels.
fn window_length(value: i32) -> i32 {
    // Chrome lengths are far inside i32.
    #[allow(clippy::cast_possible_truncation)]
    let scaled = (f64::from(value) * zoom()).round() as i32;
    scaled
}

/// The frame of the skin's logo (M14).
static LOGO_FRAME: Mutex<Option<Arc<Frame>>> = Mutex::new(None);

pub fn logo_frame() -> Option<Arc<Frame>> {
    LOGO_FRAME.lock().ok().and_then(|frame| frame.clone())
}

/// The control panel's window (F7): the skin's `ControlBoxSkinArea` with a
/// pointer of its own.
struct Panel {
    chrome: Chrome,
    pointer: Pointer,
}

/// Everything that exists once `start` ran.
// Its flags are the window's, each on its own: no state machine.
#[allow(clippy::struct_excessive_bools)]
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
    /// The control panel, once it was shown.
    panel: Option<Panel>,
    /// The desktop's media controls (MPRIS, Windows' controls), once the
    /// application started.
    desktop: Option<crate::desktop::Integration>,
    /// Windows' display kept on while video plays.
    awake: crate::awake::KeepAwake,
    /// The audio recording under way.
    recorder: Option<crate::recorder::Recorder>,
    /// Power actions only write their name here (tests, `--selftest`).
    power_dry_run: Option<PathBuf>,
    /// The frame the open Save As dialog saves (M12).
    save_as: rvp_app::capture::Shot,
    /// The slideshow state last given to the window (M12).
    slideshow: bool,
    /// The file the Direct3D 11 renderer's failure was told for (M12).
    native_failed: Option<String>,
    /// Where the GPU power budget's readings come from (D17).
    gpu_readings: GpuReadings,
    /// The processors' load for the log's seconds of playback.
    load: crate::cpu::Load,
    /// The skin, its logo and its bars over the video (M14).
    look: m14::Look,
    /// The control API for remotes (M15).
    remote: m15::Remote,
    /// The DVD that plays with its menus (M16).
    disc: Option<m16::Playing>,
    /// mpv's on-screen controller (M9).
    controller: Option<m9::Controller>,
    /// The skin's other windows (M9: a classic skin's, a VLC theme's).
    extras: Vec<extras::Extra>,
    /// A VLC theme's conditions and texts (M9).
    vlc: m9::VlcCache,
    /// The main window's shape waits for the next tick (M9).
    mask_dirty: bool,
    /// When the file started: the classic skins' title scrolls and their
    /// paused time blinks from here (M9).
    file_clock: Instant,
}

/// Who gives the GPU power budget its readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuReadings {
    /// Nobody: the budget is off, or nothing plays.
    Off,
    /// A reader thread reads the cards (`crate::gpu_power`).
    Cards,
    /// The self-test gives them (`Player.feed_gpu_power`).
    SelfTest,
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

    /// The size the settings give a newly opened video (M12), once its
    /// size is known. A video that opens in a maximized window leaves it
    /// so: fitted once the window was restored, it undid the size a Window
    /// Size command had just given (GNOME maximizes a first window that
    /// covers most of the screen).
    fn take_fit(&mut self) -> Option<rvp_app::window::WindowRequest> {
        if self.fit != Fit::Loaded {
            return None;
        }
        if self.maximized {
            self.fit = Fit::No;
            return None;
        }
        let width = i32::try_from(self.app.model.int(PropertyId::VideoWidth)?).ok();
        let height = i32::try_from(self.app.model.int(PropertyId::VideoHeight)?).ok();
        self.fit = Fit::No;
        let chrome = self.chrome.as_ref().map_or((0, 0), Chrome::chrome_size);
        self.app.open_window_size((width?, height?), chrome)
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
    PropertyId::VoDelayedFrameCount,
    PropertyId::MistimedFrameCount,
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
    pub(crate) extra_windows: QString,
    pub(crate) window_mask: QString,
    pub(crate) max_width: i32,
    pub(crate) max_height: i32,
    pub(crate) extra_tooltip: QString,
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
    pub(crate) panel_shown: bool,
    pub(crate) panel_width: i32,
    pub(crate) panel_height: i32,
    pub(crate) panel_tooltip: QString,
    pub(crate) frame_path: QString,
    pub(crate) subtitle_font: QString,
    pub(crate) panel_has_position: bool,
    pub(crate) panel_x: i32,
    pub(crate) panel_y: i32,
    pub(crate) record_path: QString,
    pub(crate) seek_tip: QString,
    pub(crate) seek_tip_x: i32,
    pub(crate) seek_tip_y: i32,
    pub(crate) seek_tip_shown: bool,
    pub(crate) preview_serial: i32,
    pub(crate) preview_width: i32,
    pub(crate) preview_height: i32,
    pub(crate) seek_picture_serial: i32,
    pub(crate) seek_picture_width: i32,
    pub(crate) seek_picture_height: i32,
    pub(crate) seek_picture_place: QRectF,
    pub(crate) beat_running: bool,
    pub(crate) hide_pointer: bool,
    pub(crate) center_window: bool,
    pub(crate) native_video: bool,
    pub(crate) chrome_bars: QString,
    pub(crate) shown_bar: QRectF,
    pub(crate) video_aspect: f64,
    pub(crate) black_other_displays: bool,
    pub(crate) fullscreen_primary: bool,
    pub(crate) window_opacity: f64,
    pub(crate) panel_opacity: f64,
    pub(crate) min_width: i32,
    pub(crate) min_height: i32,
    pub(crate) logo_source: QString,
    pub(crate) skin_logo: bool,
    pub(crate) logo_position: QString,
    pub(crate) overlay: bool,
    pub(crate) spectrum_shown: bool,
    pub(crate) osc_opacity: f64,
    pub(crate) animations: bool,
    pub(crate) osd_look: QString,
    pub(crate) misc_info: QString,
    pub(crate) general_look: QString,
    pub(crate) accessibility_look: QString,
    pub(crate) sub_text: QString,
    pub(crate) slideshow: QString,
    pub(crate) power_notice: QString,
    pub(crate) power_seconds: i32,
    pub(crate) dialog: QString,
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

/// Qt's keyboard modifiers (`Qt::KeyboardModifiers`) as the pointer's.
fn modifiers(value: i32) -> Modifiers {
    Modifiers {
        ctrl: value & QT_CONTROL != 0,
        shift: value & QT_SHIFT != 0,
        alt: value & QT_ALT != 0,
    }
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
const QT_ALT: i32 = 0x0800_0000;

/// Window coordinates from QML (fractional on scaled screens) to pixels.
fn pixel(value: f64) -> i32 {
    // Window coordinates are far inside i32.
    #[allow(clippy::cast_possible_truncation)]
    let pixel = value.floor() as i32;
    pixel
}

/// RVP Default, RVP's own skin, drawn and packed by build.rs from
/// crates/look/art/skin.
const DEFAULT_SKIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/look/skins/RVP Default.dsf"));

/// Where the skin comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SkinSource {
    File(PathBuf),
    /// RVP Default, built in.
    Default,
}

/// The skin to load: `--skin=PATH`, the setting, `RVP_SKIN`, or RVP
/// Default. `--skin=` with no path asks for RVP Default whatever else is
/// set (the Linux kit's RVP Default check).
fn skin_source(cli: &Cli, settings: &Settings) -> SkinSource {
    if cli.flag("skin") && cli.value("skin").is_none() {
        return SkinSource::Default;
    }
    cli.value("skin")
        .map(PathBuf::from)
        .or_else(|| settings.skin.clone())
        .or_else(|| {
            std::env::var_os("RVP_SKIN")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .map_or(SkinSource::Default, SkinSource::File)
}

type SkinLoad = std::thread::JoinHandle<Result<Skin, String>>;

/// The skin being loaded since the process started, and its source.
static PRELOAD: Mutex<Option<(SkinSource, SkinLoad)>> = Mutex::new(None);

/// The settings ask for the skin's touch layout (M14): the first skin
/// loads as it.
static TOUCH_AT_START: AtomicBool = AtomicBool::new(false);

fn open_skin(source: &SkinSource) -> Result<Skin, String> {
    open_skin_file(source, None, TOUCH_AT_START.load(Ordering::Relaxed))
}

/// A skin's `file` (`None`: its video skin) with RVP's logo (D10).
fn open_skin_file(source: &SkinSource, file: Option<&str>, touch: bool) -> Result<Skin, String> {
    let variables: &[(&str, &str)] = if touch { &[("SkinType", "Touch")] } else { &[] };
    // A classic Winamp skin (M9): RVP builds its own from it; it has no
    // other modes.
    if let SkinSource::File(path) = source
        && rvp_skin::winamp::is_classic(path)
    {
        return rvp_skin::winamp::open(path)
            .map_err(|error| format!("{}: {error}", path.display()));
    }
    // A VLC theme (M9): its fonts join the system's ([`keep_skin`]), and
    // `file` is the main window's layout.
    if let SkinSource::File(path) = source
        && rvp_skin::vlc::is_vlc(path)
    {
        let error = |error: rvp_skin::SkinError| format!("{}: {error}", path.display());
        let theme = rvp_skin::vlc::Theme::open(path).map_err(error)?;
        if let Ok(mut fonts) = FONTS.lock() {
            for (name, data) in &theme.font_files {
                fonts
                    .entry((path.clone(), name.clone()))
                    .or_insert_with(|| Some(data.clone()));
            }
        }
        return rvp_skin::vlc::build::skin(&theme, file).map_err(error);
    }
    let mut skin = match (source, file) {
        (SkinSource::File(path), None) => Skin::open_with(path, variables)
            .map_err(|error| format!("{}: {error}", path.display()))?,
        (SkinSource::File(path), Some(file)) => SkinArchive::open(path)
            .and_then(|archive| Skin::load_with(archive, file, variables))
            .map_err(|error| format!("{}: {error}", path.display()))?,
        (SkinSource::Default, file) => {
            let archive = SkinArchive::from_zip_bytes(rvp_look::skin::NAME, DEFAULT_SKIN.to_vec())
                .map_err(|error| format!("{}: {error}", rvp_look::skin::NAME))?;
            Skin::load_with(archive, file.unwrap_or("VideoSkin.xml"), variables)
                .map_err(|error| format!("{}: {error}", rvp_look::skin::NAME))?
        }
    };
    // D10: RVP's logo in place of the skin's title-bar logo.
    if let Err(error) = rvp_look::brand::brand_skin(&mut skin) {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("cannot draw the logo: {error}"),
        );
    }
    Ok(skin)
}

/// The VapourSynth runtime (VSScript) the process points libmpv at.
static VSSCRIPT: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Points libmpv at a VapourSynth runtime through `VSSCRIPT_PATH`, which
/// it reads when a `vapoursynth` filter starts: the one the settings name,
/// else one already in the environment, else RVP's own next to the
/// executable (goal T: never SVP's; `rvp_app::svp::vsscript_candidates`).
/// Runs before any other thread exists.
fn point_at_vapoursynth(settings: &Settings) {
    let configured = settings
        .vapoursynth
        .library
        .as_deref()
        .filter(|path| path.is_file());
    let from_environment = std::env::var_os("VSSCRIPT_PATH")
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    let chosen = match (configured, from_environment) {
        (None, Some(path)) => Some(path),
        (configured, _) => {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(PathBuf::from));
            let found = rvp_app::svp::find_vsscript(configured, exe_dir.as_deref());
            if let Some(path) = &found {
                crate::ffi::bridge::rvp_set_env("VSSCRIPT_PATH", &path.to_string_lossy());
            }
            found
        }
    };
    let _ = VSSCRIPT.set(chosen);
}

/// Points libmpv at VapourSynth and starts loading the skin on a thread
/// while Qt and QML start (called first in `main`). The settings are read
/// here and again in `start`; both are cheap.
pub fn preload_skin() -> Settings {
    let cli = Cli::parse(std::env::args().skip(1));
    let settings = Dirs::from_env()
        .ok()
        .and_then(|dirs| rvp_app::read_json::<Settings>(&dirs.config.join("settings.json")).ok())
        .flatten()
        .unwrap_or_default();
    point_at_vapoursynth(&settings);
    TOUCH_AT_START.store(settings.accessibility.touch_skin, Ordering::Relaxed);
    // Dialog Box: "Use DPI-Awareness" (M14), before Qt starts and before
    // any other thread does.
    if !settings.general.dialog.dpi_aware {
        crate::ffi::bridge::rvp_set_env("QT_ENABLE_HIGHDPI_SCALING", "0");
    }
    let source = skin_source(&cli, &settings);
    #[cfg(target_os = "linux")]
    prefer_x11(&settings, &source);
    let thread_source = source.clone();
    let spawned = std::thread::Builder::new()
        .name("rvp-skin".to_owned())
        .spawn(move || open_skin(&thread_source));
    if let (Ok(handle), Ok(mut slot)) = (spawned, PRELOAD.lock()) {
        *slot = Some((source, handle));
    }
    settings
}

/// What Wayland lets no program do: a skin with windows of its own (a
/// classic Winamp skin, a VLC theme of more than one window, M9) docks
/// them to the main window and moves them with it, and the Vulkan
/// renderer draws in a child window of RVP's. On a Wayland desktop with
/// XWayland, RVP then runs as an X11 program (`QT_QPA_PLATFORM=xcb`, set
/// before Qt or any thread starts), unless the user chose a platform.
#[cfg(target_os = "linux")]
fn prefer_x11(settings: &Settings, source: &SkinSource) {
    let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    if !set("WAYLAND_DISPLAY") || !set("DISPLAY") || set("QT_QPA_PLATFORM") {
        return;
    }
    let windows = matches!(source, SkinSource::File(path) if rvp_skin::winamp::is_classic(path)
        || (rvp_skin::vlc::is_vlc(path)
            && rvp_skin::vlc::Theme::open(path).is_ok_and(|theme| theme.windows.len() > 1)));
    if windows || settings.render.renderer.native_window() {
        crate::ffi::bridge::rvp_set_env("QT_QPA_PLATFORM", "xcb");
    }
}

fn load_skin(source: &SkinSource) -> Result<&'static Skin, String> {
    let preloaded = PRELOAD
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
        .filter(|(preloaded, _)| preloaded == source);
    let skin = match preloaded {
        Some((_, handle)) => handle
            .join()
            .map_err(|_| "the skin loader stopped".to_owned())??,
        None => open_skin(source)?,
    };
    // The skin lives for the whole process: the layout borrows it and the
    // render thread paints from it.
    let key = SkinKey {
        source: source.clone(),
        file: skin.file().to_owned(),
        tint: None,
        touch: TOUCH_AT_START.load(Ordering::Relaxed),
    };
    let skin = keep_skin(key, skin);
    show_skin(skin);
    Ok(skin)
}

/// What the command line changes in the application before it starts.
fn prepare_app(cli: &Cli, app: &mut App) {
    if cli.flag("selftest") {
        // A fresh test state must not import the owner's playlists.
        app.skip_first_import();
    }
    // Self-tests measure the smoothing engines only when asked;
    // `--calibrate` measures as soon as nothing plays.
    app.set_calibration_manual(cli.flag("selftest"));
    if cli.flag("calibrate") {
        app.calibrate_at_start();
    }
    // M9: `--controller=KIND` for this run (the self-test).
    if let Some(kind) = cli.value("controller") {
        app.settings.controller.choose(kind);
    }
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
    let (mut app, portable, message) = match loaded {
        Ok((app, portable)) => (app, portable, None),
        Err(message) => {
            // The user's runtime folder (their own; a shared /tmp/rvp-state
            // could be another user's), else one of this process's.
            let fallback = std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .filter(|dir| cfg!(unix) && dir.is_absolute() && dir.is_dir())
                .map_or_else(
                    || std::env::temp_dir().join(format!("rvp-state-{}", std::process::id())),
                    |dir| dir.join("rvp-state"),
                );
            let app = App::new(Settings::default(), &fallback).map_err(|e| e.to_string())?;
            (app, false, Some(message))
        }
    };
    // Captured images' names carry the local date (M12).
    app.set_clock(crate::ffi::bridge::rvp_local_time);
    // Misc > Interface Language lists the translations built in (M19).
    app.set_interface_languages(
        serde_json::from_str(&crate::ffi::bridge::rvp_interface_languages()).unwrap_or_default(),
    );
    // The skins' `AutoSize` labels take the widths the window draws them at.
    rvp_skin::layout::set_text_measure(qt_text_width);
    Ok((app, portable, message))
}

/// A text's width in the font the window draws it with (`AutoSize`).
fn qt_text_width(font: &rvp_skin::paint::FontSpec, text: &str) -> i32 {
    // Font sizes are a few pixels.
    #[allow(clippy::cast_possible_truncation)]
    let pixel_size = font.size_px.round() as i32;
    crate::ffi::bridge::rvp_text_width(text, &font.family, pixel_size, font.bold)
}

/// A `file:` URL for a local path, for QML (`Qt.openUrlExternally`, the
/// dialogs' folders): escaped, so `#`, `?` and `%` in a name stay part of
/// it (the Linux audit's L14).
fn file_url(path: &Path) -> String {
    rvp_app::media::file_url(&path.to_string_lossy())
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

/// The chrome with the configured skin, or with RVP Default when that
/// skin cannot be loaded (the reason goes to `messages`).
fn load_chrome(cli: &Cli, settings: &Settings, messages: &mut Vec<String>) -> Option<Chrome> {
    let source = skin_source(cli, settings);
    match load_skin(&source) {
        Ok(skin) => return Some(Chrome::new(skin)),
        Err(error) => messages.push(if source == SkinSource::Default {
            format!("cannot load the skin {error}")
        } else {
            format!("cannot load the skin {error}; RVP Default is shown instead")
        }),
    }
    if source == SkinSource::Default {
        return None;
    }
    match load_skin(&SkinSource::Default) {
        Ok(skin) => Some(Chrome::new(skin)),
        Err(error) => {
            messages.push(format!("cannot load the skin {error}"));
            None
        }
    }
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
/// The player core's options: the settings', D4's renderer and the command
/// line's mpv options last. The renderer is taken here, at start: the
/// native window (Direct3D 11 or Vulkan) when the settings ask for it and
/// it can be made (Windows), the render API otherwise.
fn engine_options(
    app: &mut App,
    cli: &Cli,
    messages: &mut Vec<String>,
) -> rvp_engine::EngineOptions {
    let mut window = None;
    if app.settings.render.renderer.native_window() {
        let wid = crate::ffi::bridge::rvp_video_window_create();
        window = i64::try_from(wid).ok().filter(|wid| *wid != 0);
        if window.is_none() {
            messages.push("The video window could not be made: OpenGL is used".to_owned());
        }
    }
    app.set_native_video(window.is_some());
    let mut options = app.engine_options();
    if let Some(wid) = window {
        crate::video::use_window();
        options.video = rvp_engine::VideoOutput::Window(wid);
        options.extra.extend(
            app.settings
                .render
                .window_options()
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value)),
        );
    }
    options.extra.extend(extra_mpv_options(cli));
    options
}

fn extra_mpv_options(cli: &Cli) -> Vec<(String, String)> {
    cli.value("mpv-options")
        .unwrap_or("")
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

/// How many rectangles the main window's shape has (M9; the self-test).
static WINDOW_MASK_LEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl qobject::Player {
    fn core(&self) -> Option<&Core> {
        self.rust().core.as_ref()
    }

    fn core_mut(self: Pin<&mut Self>) -> Option<&mut Core> {
        // SAFETY-free projection: PlayerRust is Unpin.
        self.rust_mut().get_mut().core.as_mut()
    }

    fn show_message(mut self: Pin<&mut Self>, text: &str) {
        rvp_app::log::report(rvp_app::log::Level::Warning, text);
        self.as_mut().set_message(QString::from(text));
    }

    /// The window's and the control panel's saved places; When Starting's
    /// place and size override the window's (M14).
    fn restore_places(mut self: Pin<&mut Self>, app: &App) {
        let rvp_app::general::StartWindow {
            position,
            size,
            restore_state,
        } = app.start_window();
        let saved = app.store().state().window;
        let window = if position.is_none() && size.is_none() {
            saved
        } else {
            let mut window = saved.unwrap_or(WindowGeometry {
                x: None,
                y: None,
                width: 960,
                height: 600,
                maximized: false,
            });
            if let Some((width, height)) = size {
                window.width = i32::try_from(width).unwrap_or(960);
                window.height = i32::try_from(height).unwrap_or(600);
            }
            if let Some((x, y)) = position {
                (window.x, window.y) = (Some(x), Some(y));
            }
            Some(window)
        };
        if let Some(mut window) = window {
            window.maximized &= restore_state;
            if app.settings.general.start.position == "center" {
                (window.x, window.y) = (None, None);
            }
            // "Use non-overlapping multi instances": aside from the other.
            if app.settings.general.cascade && OTHER_INSTANCE.load(Ordering::Relaxed) {
                window.x = window.x.map(|x| x + 48);
                window.y = window.y.map(|y| y + 48);
            }
            self.as_mut().restore_window(window);
        }
        if let Some((x, y)) = app.panel_position() {
            self.as_mut().set_panel_x(x);
            self.as_mut().set_panel_y(y);
            self.as_mut().set_panel_has_position(true);
        }
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
        crate::metrics::mark("settings_loaded_ms");
        prepare_app(&cli, &mut app);
        let chrome = load_chrome(&cli, &app.settings, &mut messages);
        command_line_skin(&cli, &mut app);
        crate::metrics::mark("skin_loaded_ms");

        crate::ffi::bridge::rvp_set_c_numeric_locale();
        app.prepare_svp(VSSCRIPT.get().cloned().flatten());
        let options = engine_options(&mut app, &cli, &mut messages);
        app.second_mpv_options = extra_mpv_options(&cli);
        let Some(drain_queued) = self.as_mut().start_engine(&options, &mut app) else {
            return;
        };
        crate::metrics::mark("engine_ready_ms");
        messages.extend(m11::start_thumbnails(&mut app, self.qt_thread()));
        let Some(engine) = ENGINE.get() else {
            return;
        };
        let (controller, message) = m9::start(&app, engine);
        messages.extend(message);
        let ready_thread = Mutex::new(self.qt_thread());
        crate::video::on_ready(move || {
            if let Ok(thread) = ready_thread.lock() {
                let _ = thread.queue(qobject::Player::render_ready);
            }
        });

        self.as_mut().restore_places(&app);
        self.as_mut().set_has_skin(chrome.is_some());
        self.as_mut().set_native_video(crate::video::in_window());
        self.as_mut().set_window_title(QString::from("RVP"));
        let request = start_request(&cli, &mut app, engine);
        let power_dry_run = m11::power_dry_run(&cli, app.state_dir());
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
            fit: Fit::No,
            drain_queued,
            throttle: Throttle::default(),
            popup: None,
            clock: String::new(),
            portable,
            panel: None,
            desktop: None,
            awake: crate::awake::KeepAwake::default(),
            recorder: None,
            power_dry_run,
            save_as: rvp_app::capture::Shot::Source,
            slideshow: false,
            native_failed: None,
            gpu_readings: GpuReadings::Off,
            load: crate::cpu::Load::default(),
            look: m14::Look::default(),
            remote: m15::Remote::default(),
            disc: None,
            controller,
            extras: Vec::new(),
            vlc: m9::VlcCache::default(),
            mask_dirty: true,
            file_clock: Instant::now(),
        });
        self.as_mut().rebuild_extras();
        self.as_mut().sync_look();
        if let Some(message) = messages.first() {
            self.as_mut().show_message(message);
        }
        if crate::video::READY.load(Ordering::Acquire) {
            self.render_ready();
        }
    }

    /// Starts libmpv's player core and its wakeups; `None` when it cannot
    /// start. The flag is true while a drain is queued.
    fn start_engine(
        self: Pin<&mut Self>,
        options: &rvp_engine::EngineOptions,
        app: &mut App,
    ) -> Option<Arc<AtomicBool>> {
        let mut engine = match Engine::new(options) {
            Ok(engine) => engine,
            Err(error) => {
                self.show_message(&format!("cannot start libmpv: {error}"));
                return None;
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
        app.engine_started(&engine);
        m16::start(app, &engine);
        ENGINE.set(engine).ok()?;
        Some(drain_queued)
    }

    /// The window's last geometry, for QML to restore.
    fn restore_window(mut self: Pin<&mut Self>, window: WindowGeometry) {
        self.as_mut().set_restore_width(window.width);
        self.as_mut().set_restore_height(window.height);
        if let (Some(x), Some(y)) = (window.x, window.y) {
            self.as_mut().set_restore_x(x);
            self.as_mut().set_restore_y(y);
            self.as_mut().set_restore_position(true);
        }
        self.set_restore_maximized(window.maximized);
    }

    /// The render context exists: open the files.
    fn render_ready(mut self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(mut request) = core.pending.take() else {
            return;
        };
        // PotPlayer's switches (M11): the files' order and options first.
        core.app.prepare_switches(&mut request);
        // A skin file among them is the skin, not a file to play.
        let skin = core.app.take_skin_files(&mut request.files);
        // The desktop's media controls follow the application from here,
        // unless the General page turns them off; the taskbar's buttons
        // and progress (M14). On Linux the same thread keeps the screen
        // on, shows the launcher's progress and reports the lock, with the
        // media controls off too (the Linux audit's M2).
        let media_controls = core.app.settings.general.media_controls;
        if media_controls || cfg!(target_os = "linux") {
            match crate::desktop::Integration::start(
                thread,
                crate::ffi::bridge::rvp_main_window_id(),
                media_controls,
            ) {
                Ok(desktop) => core.desktop = Some(desktop),
                Err(error) => {
                    rvp_app::log::report(rvp_app::log::Level::Warning, &error);
                }
            }
        }
        crate::ffi::bridge::rvp_taskbar_start(core.app.settings.general.taskbar);
        crate::ffi::bridge::rvp_global_media_keys(core.app.settings.input.global_media_keys);
        core.look.startup = Some(core.app.settings.general.run_at_startup);
        let plays = request.mode == rvp_app::instance::Mode::Play && !request.files.is_empty();
        if plays {
            core.request_fit();
            crate::metrics::first_frame_pending();
        } else {
            // Reopening the last file keeps the restored window size.
            core.fit = Fit::No;
        }
        // The log's times are local (it opens in `App::start`).
        rvp_app::log::set_utc_offset(crate::ffi::bridge::rvp_utc_offset());
        // `/add` and `/insert` at start: the last state first, then theirs.
        let result = if plays || request.files.is_empty() {
            core.app.start(core.engine, &request.files)
        } else {
            core.app
                .start(core.engine, &[])
                .and_then(|()| core.app.remote(core.engine, &request))
        };
        // The calibration's key (after the start: libmpv's version).
        core.app.set_hardware(crate::hardware::describe());
        // `--install-tensorrt`: RIFE (TensorRT)'s download at once, NVIDIA's
        // terms accepted by asking for it (tests and scripts); a failure is
        // in the log.
        if Cli::parse(std::env::args().skip(1)).flag("install-tensorrt") {
            drop(core.app.tensorrt_install());
        }
        let action = core.app.switch_action(core.engine, &request);
        if let Err(error) = result {
            self.as_mut().show_message(&format!("cannot open: {error}"));
        }
        // The control API follows the application from here (M15).
        self.as_mut().apply_remote();
        if let Some(skin) = skin {
            self.as_mut().sync_skin();
            self.as_mut().handle_request(skin);
        }
        if let Some(action) = action {
            self.handle_request(action);
        }
    }

    /// Queued by mpv's wakeup callback.
    fn drain(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.drain_queued.store(false, Ordering::Release);
        let drained = core.app.drain(core.engine, Instant::now());
        core.show_drain(&drained);
        let hides_pointer = core.hides_pointer();
        let center = core.app.settings.playback.center_window;
        let file_changed = drained.events.iter().any(|event| {
            matches!(
                event,
                EngineEvent::FileLoaded { .. } | EngineEvent::EndFile { .. }
            )
        });
        for event in &drained.events {
            match event {
                EngineEvent::PlaybackRestart => crate::metrics::playback_restarted(),
                // mpv asks for no new frame once its output is gone: the
                // item draws again, the empty picture (black) in place of
                // the closed file's last frame under the logo.
                EngineEvent::VideoReconfig => crate::ffi::bridge::rvp_video_request_update(),
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
        let fit = core.take_fit();
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
        self.as_mut().follow_model(has_video, drained.changes);
        self.as_mut().set_hide_pointer(hides_pointer);
        self.as_mut().set_center_window(center);
        self.as_mut().sync_window_settings();
        self.as_mut()
            .set_window_title(QString::from(if title.is_empty() {
                "RVP"
            } else {
                title.as_str()
            }));
        for request in requests {
            self.as_mut().handle_request(request);
        }
        self.as_mut().disc_drained(&drained.events);
        self.as_mut().m9_drained(&drained.events, drained.changes);
        // A file with or without video: the audio or video skin (M14).
        if file_changed {
            self.as_mut().sync_skin();
            self.as_mut().file_started();
            self.as_mut().file_closed();
        }
        if let Some(request) = fit
            && !*self.fullscreen()
        {
            match request {
                rvp_app::window::WindowRequest::Size { width, height } => {
                    self.as_mut().fit_window(width, height);
                }
                other => self.as_mut().window_request(&other),
            }
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
        self.as_mut().disc_tick();
        self.as_mut().sync_window_mask();
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
        self.as_mut().follow_disc();
        self.as_mut().controller_tick();
        self.as_mut().publish_remote();
        let (fullscreen, on_top) = (*self.fullscreen(), *self.on_top());
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let synced =
            core.app.model.playing() && core.app.model.int(PropertyId::VideoWidth).is_some();
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        let mut data = view::data_model(
            &core.app.model,
            core.app.library().lists(),
            core.app.settings.volume_max,
        );
        core.app.seek_bar_data(&mut data);
        core.app.list_view_data(&mut data);
        data.marks = core.app.seek_marks();
        data.playlist.drop_before = core.drop_before;
        if !core.clock.is_empty() {
            data.set_text("CMD_SYSTEM_TIME_MESSAGE", core.clock.as_str());
        }
        core.app.panel_data(&mut data);
        // A classic skin's texts and states, a VLC theme's (M9).
        if !core.extras.is_empty() {
            core.app
                .classic_data(&mut data, &mut chrome.state, core.file_clock.elapsed());
        }
        let window = (fullscreen, on_top, core.maximized);
        m9::vlc_data(
            &core.app,
            &mut core.vlc,
            &core.extras,
            chrome.skin(),
            window,
            &mut data,
            &mut chrome.state,
        );
        // The spectrum's bars move on their own tick (`tick_spectrum`).
        data.visual = std::mem::take(&mut chrome.data.visual);
        data.visual_peaks = std::mem::take(&mut chrome.data.visual_peaks);
        if data != chrome.data {
            chrome.data = data;
        }
        let spectrum = core.app.skin_spectrum() && core.app.model.media_loaded();
        view::apply_playback_state(&mut chrome.state, &core.app.model);
        chrome.state.fullscreen = fullscreen;
        view::apply_auto_values(
            &mut chrome.state,
            &chrome.data,
            &core.app.model,
            core.app.settings.on_top,
            on_top,
        );
        let toggles = chrome.toggle_commands();
        core.app.toggle_flags(&mut chrome.state, toggles);
        let damage = chrome.refresh();
        // The thumbnail view's rows ask for their pictures (M11), the last
        // one even when it shows in part.
        if let Some(geometry) = chrome.list_geometry()
            && !geometry.area.is_empty()
        {
            core.app
                .request_list_thumbnails(geometry.first, geometry.visible + 1);
        }
        self.as_mut().emit_damage(&damage, synced);
        if *self.spectrum_shown() != spectrum {
            self.as_mut().set_spectrum_shown(spectrum);
        }
        self.as_mut().sync_panel(false);
        self.as_mut().sync_extras(false);
        self.sync_logo();
    }

    /// A classic skin's displays move on their own (M9): its title
    /// scrolls and its paused time blinks; QML calls this a few times a
    /// second while the skin shows.
    pub(crate) fn classic_tick(self: Pin<&mut Self>) {
        if self.core().is_some_and(|core| !core.extras.is_empty()) {
            self.sync_chrome();
        }
    }

    /// Recomputes the control panel while it shows; it repaints whole.
    fn sync_panel(mut self: Pin<&mut Self>, force: bool) {
        if !*self.panel_shown() {
            return;
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(panel) = core.panel.as_mut() else {
            return;
        };
        let mut data = rvp_skin::DataModel::default();
        core.app.panel_data(&mut data);
        if data != panel.chrome.data {
            panel.chrome.data = data;
        }
        view::apply_playback_state(&mut panel.chrome.state, &core.app.model);
        let toggles = panel.chrome.toggle_commands();
        core.app.toggle_flags(&mut panel.chrome.state, toggles);
        let damage = panel.chrome.refresh();
        if damage.is_empty() && !force {
            return;
        }
        if let Ok(mut slot) = PANEL_FRAME.lock() {
            *slot = Some(panel.chrome.frame());
        }
        self.panel_damaged();
    }

    /// Shows or hides the control panel; it is laid out the first time.
    fn show_panel(mut self: Pin<&mut Self>, shown: bool) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if shown && core.panel.is_none() {
            core.panel = core.chrome.as_ref().and_then(|main| {
                let skin = main.skin();
                let (width, height) = Mode::ControlBox.area_size(skin)?;
                let mut chrome = Chrome::new(skin);
                chrome.resize(Mode::ControlBox, width, height).ok()?;
                Some(Panel {
                    chrome,
                    pointer: Pointer::default(),
                })
            });
            if core.panel.is_none() {
                core.app.panel_closed();
                self.set_osd(QString::from("The skin has no control panel"));
                return;
            }
        }
        let (width, height) = core
            .panel
            .as_ref()
            .map_or((0, 0), |panel| panel.chrome.size());
        // The panel's zoom (M14).
        #[allow(clippy::cast_possible_truncation)]
        let (width, height) = (
            (f64::from(width) * panel_zoom()).round() as i32,
            (f64::from(height) * panel_zoom()).round() as i32,
        );
        self.as_mut().set_panel_width(width);
        self.as_mut().set_panel_height(height);
        self.as_mut().set_panel_shown(shown);
        self.sync_panel(true);
    }

    /// `phase`: 0 press, 1 double click, 2 release (as `pointer_event`).
    fn panel_pointer(mut self: Pin<&mut Self>, x: f64, y: f64, button_value: i32, phase: u8) {
        let Some(button) = button(button_value) else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(panel) = core.panel.as_mut() else {
            return;
        };
        let ctx = Self::panel_context(panel);
        let (x, y) = (chrome_pixel(x, panel_zoom()), chrome_pixel(y, panel_zoom()));
        let bindings = &core.app.bindings;
        let effects = match phase {
            0 => panel.pointer.press(&mut panel.chrome, ctx, x, y, button),
            1 => panel
                .pointer
                .double_click(&mut panel.chrome, bindings, ctx, x, y, button),
            _ => panel
                .pointer
                .release(&mut panel.chrome, bindings, ctx, x, y, button),
        };
        self.as_mut().apply_panel_effects(&effects);
        self.sync_chrome();
    }

    fn panel_context(panel: &Panel) -> Context {
        let (width, height) = panel.chrome.size();
        Context {
            width,
            height,
            movable: true,
            chrome_shown: true,
            panel: true,
            turn_view: false,
            modifiers: Modifiers::default(),
            options: rvp_app::pointer::PointerOptions::default(),
        }
    }

    /// The control panel moves its own window; the rest is as in the main
    /// window.
    fn apply_panel_effects(mut self: Pin<&mut Self>, effects: &[Effect]) {
        for effect in effects {
            match effect {
                Effect::StartMove => self.as_mut().ui_request(request::PANEL_MOVE, 0),
                Effect::Focus(_) => {}
                other => self.as_mut().apply_effects(std::slice::from_ref(other)),
            }
        }
    }

    pub(crate) fn panel_press(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.panel_pointer(x, y, button_value, 0);
    }

    pub(crate) fn panel_double(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.panel_pointer(x, y, button_value, 1);
    }

    pub(crate) fn panel_release(self: Pin<&mut Self>, x: f64, y: f64, button_value: i32) {
        self.panel_pointer(x, y, button_value, 2);
    }

    pub(crate) fn panel_move(mut self: Pin<&mut Self>, x: f64, y: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(panel) = core.panel.as_mut() else {
            return;
        };
        let ctx = Self::panel_context(panel);
        let effects = panel.pointer.moved(
            &mut panel.chrome,
            ctx,
            chrome_pixel(x, panel_zoom()),
            chrome_pixel(y, panel_zoom()),
        );
        let tooltip = panel.pointer.tooltip.unwrap_or("");
        self.as_mut().set_panel_tooltip(QString::from(tooltip));
        self.as_mut().apply_panel_effects(&effects);
        self.sync_chrome();
    }

    pub(crate) fn panel_leave(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(panel) = core.panel.as_mut() {
            panel.pointer.leave(&mut panel.chrome);
        }
        self.as_mut().set_panel_tooltip(QString::default());
        self.sync_panel(false);
    }

    pub(crate) fn panel_closed(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.panel_closed();
        }
        self.set_panel_shown(false);
    }

    pub(crate) fn panel_rect(&self, command: &QString) -> QRectF {
        let command = command.to_string();
        let (command, page) = match command.split_once(':') {
            Some((command, page)) => (command.to_owned(), page.parse::<i32>().ok()),
            None => (command, None),
        };
        self.core()
            .and_then(|core| core.panel.as_ref())
            .and_then(|panel| {
                panel.chrome.layout()?.nodes.iter().find(|node| {
                    node.command() == Some(command.as_str())
                        && page.is_none_or(|page| node.element.attrs.int("PageIndex") == Some(page))
                })
            })
            .map_or_else(QRectF::default, |node| {
                let zoom = panel_zoom();
                let rect = node.rect;
                QRectF::new(
                    f64::from(rect.x) * zoom,
                    f64::from(rect.y) * zoom,
                    f64::from(rect.w) * zoom,
                    f64::from(rect.h) * zoom,
                )
            })
    }

    pub(crate) fn apply_subtitle_font(mut self: Pin<&mut Self>, family: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core.app.set_subtitle_font(core.engine, &family.to_string());
        self.as_mut().handle_request(request);
        self.sync_chrome();
    }

    pub(crate) fn settings_page_shown(mut self: Pin<&mut Self>, key: &QString) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.settings_page_shown(&key.to_string());
        }
    }

    pub(crate) fn panel_moved(mut self: Pin<&mut Self>, x: i32, y: i32) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.set_panel_position(x, y);
        }
    }

    pub(crate) fn save_frame_to(mut self: Pin<&mut Self>, path: &QString) {
        let path = path.to_string();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if !path.is_empty() {
            let path = PathBuf::from(path);
            core.app.frame_saved_as(&path);
            let job = core.app.capture_job(core.save_as, Some(path));
            self.capture(job);
        }
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
        let bars = chrome
            .bars()
            .iter()
            .map(|bar| {
                let bar = to_window(*bar);
                format!("[{},{},{},{}]", bar.x(), bar.y(), bar.width(), bar.height())
            })
            .collect::<Vec<_>>()
            .join(",");
        if damage.regions_changed {
            let serial = self.layout_serial().wrapping_add(1);
            self.as_mut().set_layout_serial(serial);
            self.as_mut().set_region_count(count);
        }
        self.as_mut()
            .set_chrome_bars(QString::from(&format!("[{bars}]")));
        self.as_mut().set_video_rect(to_window(video));
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
        self.as_mut().disc_layout_changed();
        let overlay = self.as_mut().sync_overlay();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (width, height) = (core.width, core.height);
        let Some(chrome) = core.chrome.as_mut() else {
            let rect = rvp_skin::Rect::new(0, 0, width, height);
            self.set_video_rect(to_rectf(rect));
            return;
        };
        let mode = if overlay {
            Mode::Fullscreen
        } else {
            Mode::Window
        };
        let (chrome_width, chrome_height) = chrome_size(width, height);
        let damage = match chrome.resize(mode, chrome_width, chrome_height) {
            Ok(damage) => damage,
            Err(error) => {
                let text = format!("skin layout failed: {error}");
                self.show_message(&text);
                return;
            }
        };
        let synced = false;
        self.as_mut().emit_damage(&damage, synced);
        if !overlay {
            self.as_mut().sync_limits();
        }
        if let Some(core) = self.as_mut().core_mut() {
            core.mask_dirty = true;
        }
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
                .is_some_and(|c| c.size() == chrome_size(width, height))
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
        if let Some(core) = self.as_mut().core_mut() {
            // RVP's own requests chose the kind of fullscreen already.
            let fit = if fullscreen {
                core.app.fullscreen_fit()
            } else {
                None
            };
            core.app.set_fullscreen(core.engine, fullscreen, fit);
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
        // The chrome's size: the window's over the skin's zoom.
        let (width, height) = chrome_size(core.width, core.height);
        Some(Context {
            width,
            height,
            movable: !fullscreen && !core.maximized,
            chrome_shown: core.chrome.is_some() && (!*self.overlay() || *self.chrome_shown()),
            panel: false,
            turn_view: core.app.vr_active(),
            modifiers,
            options: core.app.settings.input.pointer(),
        })
    }

    pub(crate) fn mouse_press(
        self: Pin<&mut Self>,
        x: f64,
        y: f64,
        button_value: i32,
        modifiers: i32,
    ) {
        let modifiers = self::modifiers(modifiers);
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
        // A DVD menu's buttons under the pointer (M16).
        if button == Button::Left
            && match phase {
                0 => self.as_mut().disc_pointer(x, y, true),
                2 => self.as_mut().disc_release(),
                _ => false,
            }
        {
            return;
        }
        // An on-screen controller over the video (M9).
        if self
            .as_mut()
            .controller_button(x, y, button, phase, modifiers)
        {
            self.sync_chrome();
            return;
        }
        self.rvp_pointer(x, y, button, phase, modifiers);
    }

    /// RVP's own pointer: the skin's controls, the window's moves and the
    /// video's gestures. `phase` as for [`Self::pointer_event`].
    fn rvp_pointer(
        mut self: Pin<&mut Self>,
        x: f64,
        y: f64,
        button: Button,
        phase: u8,
        modifiers: Modifiers,
    ) {
        let Some(ctx) = self.pointer_context(modifiers) else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (x, y) = (chrome_pixel(x, zoom()), chrome_pixel(y, zoom()));
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
                Effect::SeekFraction(fraction) => {
                    if let Some(core) = self.as_mut().core_mut() {
                        crate::metrics::seek_issued();
                        core.app.drag_seek_bar(core.engine, fraction);
                    }
                }
                Effect::SeekRelease(fraction) => {
                    if let Some(core) = self.as_mut().core_mut() {
                        crate::metrics::seek_issued();
                        core.app
                            .release_seek_bar(core.engine, fraction, Instant::now());
                    }
                }
                Effect::VolumeFraction(fraction) => {
                    if let Some(core) = self.as_mut().core_mut()
                        && let Some(request) = core.app.set_volume_fraction(core.engine, fraction)
                    {
                        self.as_mut().handle_request(request);
                    }
                }
                Effect::Slider { command, fraction } => {
                    if let Some(core) = self.as_mut().core_mut()
                        && let Some(request) = core.app.set_slider(core.engine, command, fraction)
                    {
                        self.as_mut().handle_request(request);
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
                Effect::SkinFile(file) => {
                    if let Some(core) = self.as_mut().core_mut() {
                        core.app.load_skin_file(file);
                    }
                    self.as_mut().sync_skin();
                }
                Effect::SeekHover(hover) => self.as_mut().seek_hover(hover),
                Effect::TurnView { dx, dy } => {
                    if let Some(core) = self.as_mut().core_mut() {
                        core.app.turn_vr(core.engine, f64::from(dx), f64::from(dy));
                    }
                }
                Effect::Window(command) => self.as_mut().toggle_extra(command),
                Effect::Adjust { kind, dx, dy } => {
                    let request = self
                        .as_mut()
                        .core_mut()
                        .and_then(|core| core.app.drag_adjust(core.engine, kind, dx, dy));
                    if let Some(request) = request {
                        self.as_mut().handle_request(request);
                    }
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
                let plays = matches!(effect, ListEffect::Play(_));
                if plays {
                    crate::metrics::input("input_ms");
                    core.request_fit();
                }
                let visible = Self::visible_rows(core);
                let request = core.app.list_effect(core.engine, effect, visible);
                // "Move focus to main win on manual play" (M11).
                if plays && core.app.settings.playlist.focus_main {
                    core.focus = Focus::Window;
                }
                if let Some(request) = request {
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
        let request = core.app.list_action(core.engine, action, extend, visible);
        if action == ListAction::Play && core.app.settings.playlist.focus_main {
            core.focus = Focus::Window;
        }
        if let Some(request) = request {
            self.handle_request(request);
        }
    }

    /// A command from the desktop's media controls (MPRIS, Windows'
    /// controls), queued to the window's thread.
    pub(crate) fn media_command(mut self: Pin<&mut Self>, command: rvp_app::media::MediaCommand) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(request) = core.app.media_command(core.engine, command) {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    pub(crate) fn seek_fraction(mut self: Pin<&mut Self>, fraction: f64) {
        if let Some(core) = self.as_mut().core_mut() {
            crate::metrics::seek_issued();
            core.app.seek_to_fraction(core.engine, fraction);
        }
    }

    pub(crate) fn mouse_move(mut self: Pin<&mut Self>, x: f64, y: f64) {
        self.as_mut().disc_pointer(x, y, false);
        let edge_cursor = self.as_mut().controller_move(x, y);
        let controller = self.controller_active();
        let Some(ctx) = self.pointer_context(Modifiers::default()) else {
            return;
        };
        let overlay = *self.overlay();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let (x, y) = (chrome_pixel(x, zoom()), chrome_pixel(y, zoom()));
        let Some(chrome) = core.chrome.as_mut() else {
            return;
        };
        let bar = chrome
            .bar_rects()
            .into_iter()
            .find(|rect| rect.contains(x, y));
        let effects = core.pointer.moved(chrome, ctx, x, y);
        let tooltip = core
            .vlc
            .tooltip(&core.app, core.pointer.tooltip.unwrap_or(""));
        let cursor = cursor_shape(core.pointer.cursor);
        let dragging = core.pointer.dragging();
        let osc = core.app.settings.skins.osc;
        // General: "Show tooltips" (M14).
        let tooltip = if core.app.settings.general.tooltips {
            tooltip.as_str()
        } else {
            ""
        };
        core.look.bars_moved = Some(Instant::now());
        self.as_mut().set_tooltip(QString::from(tooltip));
        self.as_mut()
            .set_cursor_shape(edge_cursor.unwrap_or(cursor));
        // A controller takes the place of the skin's bars (M9).
        if overlay && !controller {
            // The bar the pointer is on shows (auto-hide), alone, as in
            // PotPlayer's fullscreen: the title bar at the top edge, the
            // control bar at the bottom; a drag keeps its bar. Nothing
            // shows when On Screen Control is off.
            if let Some(bar) = bar
                && !dragging
            {
                if let Some(core) = self.as_mut().core_mut() {
                    core.look.bar = Some(bar);
                }
                self.as_mut().set_shown_bar(to_window(bar));
            }
            self.as_mut().show_bars(osc && (bar.is_some() || dragging));
        }
        self.as_mut().apply_effects(&effects);
        self.sync_chrome();
    }

    pub(crate) fn mouse_wheel(self: Pin<&mut Self>, x: f64, y: f64, delta: i32) {
        // An on-screen controller over the video (M9).
        if self.controller_wheel(x, y, delta) {
            return;
        }
        self.rvp_wheel(x, y, delta);
    }

    /// RVP's own wheel: the skin's controls or the bindings.
    fn rvp_wheel(mut self: Pin<&mut Self>, x: f64, y: f64, delta: i32) {
        let Some(ctx) = self.pointer_context(Modifiers::default()) else {
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let effects = match core.chrome.as_ref() {
            Some(chrome) => core.pointer.wheel(
                chrome,
                &core.app.bindings,
                ctx,
                (chrome_pixel(x, zoom()), chrome_pixel(y, zoom())),
                delta,
            ),
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
        self.as_mut().controller_leave();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let left_bar = core
            .chrome
            .as_mut()
            .is_some_and(|chrome| core.pointer.leave(chrome));
        if left_bar {
            self.as_mut().seek_hover(None);
        }
        self.as_mut().set_tooltip(QString::default());
        if *self.overlay() {
            self.as_mut().show_bars(false);
        }
        self.sync_chrome();
    }

    pub(crate) fn key(mut self: Pin<&mut Self>, key: i32, modifiers: i32) -> bool {
        // A DVD menu's arrows and Enter (M16).
        if self.as_mut().disc_key(key, modifiers) {
            self.sync_chrome();
            return true;
        }
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
            UiRequest::ToggleFullscreen { fit } => {
                let enter = !fullscreen;
                core.app.set_fullscreen(core.engine, enter, fit);
                self.as_mut().set_chrome_shown(false);
                self.as_mut().set_fullscreen(enter);
                // The bars lie over the video from now; the layout follows
                // with the window's new size.
                self.as_mut().sync_overlay();
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
            UiRequest::Raise => self.ui_request(request::RAISE, 0),
            UiRequest::ContextMenu => self.ui_request(request::CONTEXT_MENU, 0),
            UiRequest::TogglePlaylist => {
                if let Some(chrome) = core.chrome.as_mut() {
                    chrome.state.playlist_attached = !chrome.state.playlist_attached;
                }
                self.relayout();
            }
            UiRequest::Preferences(page) => {
                // Preferences alone opens on the page shown last.
                let page = page
                    .or_else(|| core.app.last_settings_page())
                    .map(QString::from)
                    .unwrap_or_default();
                self.as_mut().set_settings_page(page);
                self.ui_request(request::SETTINGS, 0);
            }
            UiRequest::About => self.ui_request(request::ABOUT, 0),
            UiRequest::OpenFolderDialog => self.ui_request(request::OPEN_FOLDER, 0),
            UiRequest::OpenUrlDialog => self.ui_request(request::OPEN_URL, 0),
            UiRequest::OpenSubtitleDialog => self.ui_request(request::OPEN_SUBTITLE, 0),
            UiRequest::CombineSubtitleDialog => self.ui_request(request::OPEN_SUBTITLE, 1),
            UiRequest::SubtitleSyncDialog(seconds) => {
                // Syncs are within ±10 min.
                #[allow(clippy::cast_possible_truncation)]
                let milliseconds = (seconds * 1000.0).round() as i32;
                self.ui_request(request::SUBTITLE_SYNC, milliseconds);
            }
            UiRequest::ImageFolderDialog => self.ui_request(request::IMAGE_FOLDER, 0),
            UiRequest::OpenExternal(path) => {
                self.as_mut()
                    .set_external_url(QString::from(&file_url(&path)));
                self.ui_request(request::OPEN_EXTERNAL, 0);
            }
            UiRequest::Capture(job) => self.capture(job),
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
            UiRequest::ControlPanel(shown) => self.show_panel(shown),
            UiRequest::SubtitleFontDialog(family) => {
                self.as_mut().set_subtitle_font(QString::from(&family));
                self.ui_request(request::SUBTITLE_FONT, 0);
            }
            UiRequest::SaveFrameDialog { path, shot } => {
                core.save_as = shot;
                self.as_mut()
                    .set_frame_path(QString::from(&file_url(&path)));
                self.ui_request(request::SAVE_FRAME_AS, 0);
            }
            // General info messages can be turned off (M14); asked-for
            // info shows anyway.
            UiRequest::Osd(text) => {
                rvp_app::log::write(rvp_app::log::Level::Debug, &format!("osd: {text}"));
                if core.app.settings.osd.messages {
                    self.as_mut().set_osd(QString::from(&text));
                }
                // Scroll Lock's line shows at once.
                self.tick_misc_info();
            }
            UiRequest::Info(text) => self.set_osd(QString::from(&text)),
            UiRequest::Message(text) => self.show_message(&text),
            request @ (UiRequest::SystemVolume(_)
            | UiRequest::AudioSyncDialog(_)
            | UiRequest::RecordDialog(_)
            | UiRequest::RecordFolderDialog
            | UiRequest::Record(_)) => self.audio_request(request),
            other => self.m11_request(other),
        }
    }

    /// The requests of the Audio menu (M10).
    fn audio_request(mut self: Pin<&mut Self>, request: UiRequest) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        match request {
            UiRequest::SystemVolume(change) => {
                #[cfg(target_os = "linux")]
                crate::system_volume::change_later(self.qt_thread(), change);
                #[cfg(not(target_os = "linux"))]
                {
                    let text = crate::system_volume::change(change).unwrap_or_else(|error| error);
                    self.set_osd(QString::from(&text));
                }
            }
            UiRequest::AudioSyncDialog(seconds) => {
                // Syncs are within ±10 s.
                #[allow(clippy::cast_possible_truncation)]
                let milliseconds = (seconds * 1000.0).round() as i32;
                self.ui_request(request::AUDIO_SYNC, milliseconds);
            }
            UiRequest::RecordDialog(path) => {
                self.as_mut()
                    .set_record_path(QString::from(&file_url(&path)));
                self.ui_request(request::RECORD_AS, 0);
            }
            UiRequest::RecordFolderDialog => self.ui_request(request::RECORD_FOLDER, 0),
            UiRequest::Record(rvp_app::audio::commands::Record::Start(path)) => {
                self.record_to(path);
            }
            UiRequest::Record(rvp_app::audio::commands::Record::Stop) => {
                let text = match core.recorder.take() {
                    Some(recorder) => match recorder.stop() {
                        Ok(seconds) => format!("Audio recording stopped: {seconds:.1} s"),
                        Err(error) => error,
                    },
                    None => "No audio recording".to_owned(),
                };
                self.set_osd(QString::from(&text));
            }
            _ => {}
        }
    }

    /// Starts recording what RVP plays into `path`.
    fn record_to(mut self: Pin<&mut Self>, path: PathBuf) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        // One recording at a time.
        if let Some(previous) = core.recorder.take() {
            drop(previous.stop());
        }
        let text = match crate::recorder::Recorder::start(path) {
            Ok(recorder) => {
                let text = format!("Audio recording: {}", recorder.path().display());
                core.app.recording_started(recorder.path().to_path_buf());
                core.recorder = Some(recorder);
                text
            }
            Err(error) => {
                core.app.recording_stopped();
                error
            }
        };
        self.set_osd(QString::from(&text));
    }

    /// The Adjust Subtitle Sync dialog set the sync (seconds).
    pub(crate) fn set_subtitle_delay(mut self: Pin<&mut Self>, seconds: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core.app.set_subtitle_delay(core.engine, seconds);
        self.handle_request(request);
    }

    /// The Adjust Subtitle Sync dialog chose the rate the subtitles were
    /// made for (0: the video's).
    pub(crate) fn set_subtitle_fps(mut self: Pin<&mut Self>, fps: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core
            .app
            .set_subtitle_fps(core.engine, (fps > 0.0).then_some(fps));
        self.handle_request(request);
    }

    /// The rate the subtitles were made for, for the dialog (0: the
    /// video's).
    pub(crate) fn subtitle_fps(&self) -> f64 {
        self.core()
            .and_then(|core| core.app.subtitle_fps())
            .unwrap_or(0.0)
    }

    /// The Combine Subtitle dialog chose a second subtitle file.
    pub(crate) fn combine_subtitle(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core
            .app
            .combine_subtitle(core.engine, Path::new(&path.to_string()));
        self.handle_request(request);
    }

    /// The Adjust Audio Sync dialog set the sync (seconds).
    pub(crate) fn set_audio_delay(mut self: Pin<&mut Self>, seconds: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(request) = core.app.set_audio_delay(core.engine, seconds) {
            self.handle_request(request);
        }
    }

    /// The save dialog chose where to record.
    pub(crate) fn start_recording(self: Pin<&mut Self>, path: &QString) {
        self.record_to(PathBuf::from(path.to_string()));
    }

    /// Sets the folder audio recordings go to.
    pub(crate) fn set_record_folder(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let folder = PathBuf::from(path.to_string());
        core.app.set_record_folder(&folder);
        self.set_osd(QString::from(&format!(
            "Audio recordings go to {}",
            folder.display()
        )));
    }

    /// Saves the frame on a thread: mpv encodes the image, which takes
    /// longer than a frame.
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
            // Context Menu Items (M14): RVP's whole menu.
            "context-all" => Some(core.app.full_context_menu(&live)),
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

    pub(crate) fn reset_settings(mut self: Pin<&mut Self>, paths: &QString) -> QString {
        let Some(core) = self.as_mut().core_mut() else {
            return settings_result(&[], false, Some("not started"));
        };
        let paths: Vec<String> = match serde_json::from_str(&paths.to_string()) {
            Ok(paths) => paths,
            Err(error) => return settings_result(&[], false, Some(&error.to_string())),
        };
        let (changed, mut notes) = match preferences::reset(&core.app.settings, &paths) {
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
        self.as_mut().sync_window_settings();
        // Another skin, mode or theme, the windows' opacity, the bars
        // over the video, the logo and the General page (M14).
        self.as_mut().sync_look();
        self.as_mut().refresh_overlay();
        self.as_mut().tick_misc_info();
        self.as_mut().apply_general();
        self.as_mut().apply_remote();
        self.sync_chrome();
    }

    /// The video's shape, M12's fullscreen settings and the slideshow for
    /// `Main.qml`.
    fn sync_window_settings(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let shown = core.app.slideshow_shown();
        if shown != core.slideshow {
            core.slideshow = shown;
            let json = core.app.slideshow_json();
            self.as_mut().set_slideshow(QString::from(&json));
        }
        let Some(core) = self.core() else {
            return;
        };
        let window = &core.app.settings.window;
        let aspect = core
            .app
            .model
            .double(PropertyId::VideoAspect)
            .unwrap_or(0.0);
        let black = window.black_other_displays;
        let primary = window.fullscreen_display == rvp_app::window::FullscreenDisplay::Primary;
        self.as_mut().set_video_aspect(aspect);
        self.as_mut().set_black_other_displays(black);
        self.as_mut().set_fullscreen_primary(primary);
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
            rvp_app::crash::version_text()
        ))
    }

    pub(crate) fn mpv_osd(&self, text: &QString, milliseconds: i32) {
        if let Some(core) = self.core() {
            let text = text.to_string();
            drop(
                core.engine
                    .command("show-text", &[text.as_str(), &milliseconds.to_string()]),
            );
        }
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
        let Some(mut request) = Request::decode(&message.to_string()) else {
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
        // A skin file is the skin (PotPlayer applies an opened `.dsf`);
        // a request with nothing else leaves the playlist as it is.
        if let Some(skin) = core.app.take_skin_files(&mut request.files) {
            self.as_mut().sync_skin();
            self.as_mut().handle_request(skin);
            if request.files.is_empty() {
                self.sync_chrome();
                return;
            }
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if request.mode == rvp_app::instance::Mode::Play && !request.files.is_empty() {
            core.request_fit();
            self.as_mut().ui_request(request::RAISE, 0);
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.app.prepare_switches(&mut request);
        let result = core.app.remote(core.engine, &request);
        let action = core.app.switch_action(core.engine, &request);
        if let Err(error) = result {
            self.as_mut().show_message(&format!("cannot open: {error}"));
        }
        if let Some(action) = action {
            self.as_mut().handle_request(action);
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
        let (bars_width, bars_height) = chrome.chrome_size();
        let bars = (window_length(bars_width), window_length(bars_height));
        let fitted = rvp_app::chrome::fit_aspect(window, bars, aspect);
        if fitted != window {
            self.as_mut().fit_window(fitted.0, fitted.1);
        }
    }

    pub(crate) fn editor_open(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.editor = Some(Editor::new(
                &core.app.bindings,
                &rvp_app::Bindings::with_options(&core.app.settings.input),
            ));
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
        to_window(rect)
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
        to_window(rect)
    }

    pub(crate) fn tick(mut self: Pin<&mut Self>) {
        let started = Instant::now();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.app.tick_second(core.engine, Instant::now());
        let watched = core.app.smoothing_watch(core.engine, Instant::now());
        let calibrated = core.app.calibration_tick(core.engine, Instant::now());
        let installed = core.app.tensorrt_tick(core.engine);
        let reloaded = core.app.watch_subtitle_files(core.engine);
        let svp = core.app.svp_tick(core.engine, Instant::now());
        let speech = core.app.speech_tick(core.engine, Instant::now());
        if let Err(error) = core.app.tick(Instant::now()) {
            let text = format!("cannot save the playback state: {error}");
            self.as_mut().show_message(&text);
        }
        for request in [watched, calibrated, installed, reloaded, svp]
            .into_iter()
            .flatten()
            .chain(speech)
        {
            self.as_mut().handle_request(request);
        }
        self.as_mut().tick_m11();
        self.as_mut().tick_bars();
        self.as_mut().tick_misc_info();
        self.as_mut().tick_taskbar();
        self.as_mut().tick_gpu_power();
        self.as_mut().tick_log(started.elapsed());
        self.check_native_video();
    }

    /// The log's second of playback: mpv's frame counters with the frames
    /// the window drew, the processors' load and, at Debug, the window
    /// thread's second (`work`: this tick's time so far) (the Log page).
    fn tick_log(mut self: Pin<&mut Self>, work: std::time::Duration) {
        let beating = *self.beat_running();
        let pause = crate::video::take_pause(beating);
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let playing = core.app.model.playing();
        let beat = playing && rvp_app::log::enabled(rvp_app::log::Level::Debug);
        if beat != beating {
            self.as_mut().set_beat_running(beat);
        }
        let Some(core) = self.core_mut() else {
            return;
        };
        let window =
            pause.map(|pause| (u32::try_from(work.as_millis()).unwrap_or(u32::MAX), pause));
        let drawn = crate::video::take_drawn(core.app.model.playing());
        // Every second: smoothing's steps down judge other programs' load.
        let processors = core.load.sample();
        core.app.set_processors(processors);
        if !rvp_app::log::enabled(rvp_app::log::Level::Info) {
            return;
        }
        rvp_app::log::set_utc_offset(crate::ffi::bridge::rvp_utc_offset());
        core.app.log_second(&rvp_app::log::Second {
            drawn,
            processors,
            window,
        });
    }

    /// The GPU power budget (D17): the cards are read while it is on (or
    /// the log takes Debug, which gets them every 3 s), a file plays (a
    /// reading a second would keep a laptop's sleeping card awake) and no
    /// self-test gives the readings; its steps and returns are carried out
    /// and logged.
    fn tick_gpu_power(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        // For the budget, or for the log at Debug.
        let wanted = core.app.gpu_readings_wanted() && core.app.model.playing();
        match core.gpu_readings {
            GpuReadings::Off if wanted => {
                let (sender, receiver) = std::sync::mpsc::channel();
                core.app.gpu_power_listen(Some(receiver));
                crate::gpu_power::start(sender);
                core.gpu_readings = GpuReadings::Cards;
            }
            GpuReadings::Cards if !wanted => {
                // The reader ends at its next reading.
                core.app.gpu_power_listen(None);
                core.gpu_readings = GpuReadings::Off;
            }
            _ => {}
        }
        let requests = core.app.gpu_tick(core.engine, Instant::now());
        self.gpu_power_requests(requests);
    }

    /// The budget's notices: shown and logged (test runs show what it did).
    fn gpu_power_requests(mut self: Pin<&mut Self>, requests: Vec<UiRequest>) {
        for request in requests {
            if let UiRequest::Osd(text) = &request {
                rvp_app::log::report(rvp_app::log::Level::Warning, text);
            }
            self.as_mut().handle_request(request);
        }
    }

    pub(crate) fn gpu_power_json(&self) -> QString {
        self.core().map_or_else(
            || QString::from("{}"),
            |core| QString::from(&core.app.gpu_power_view(Instant::now()).to_string()),
        )
    }

    /// A self-test's reading for the budget: from the first one on, the
    /// cards are no longer read and the budget waits seconds, not minutes.
    pub(crate) fn feed_gpu_power(mut self: Pin<&mut Self>, draw: f64, rated: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if core.gpu_readings != GpuReadings::SelfTest {
            core.app.gpu_power_listen(None);
            core.app.gpu_power_test();
            core.gpu_readings = GpuReadings::SelfTest;
        }
        let sample = rvp_app::gpu_power::Sample {
            name: "self-test".to_owned(),
            draw,
            rated,
            unit: rvp_app::gpu_power::Unit::Watts,
            state: rvp_app::gpu_power::CardState::default(),
        };
        let requests = core
            .app
            .gpu_power_sample(core.engine, vec![sample], Instant::now());
        self.gpu_power_requests(requests);
    }

    /// The Direct3D 11 renderer's window shows nothing when mpv cannot make
    /// its output (an old GPU, no swapchain): a playing file with a video
    /// track but no video says so, once per file.
    fn check_native_video(mut self: Pin<&mut Self>) {
        if !crate::video::in_window() {
            return;
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let model = &core.app.model;
        let path = model.loaded_path().map(str::to_owned);
        if path.is_none()
            || path == core.native_failed
            || model.int(PropertyId::VideoWidth).is_some()
            || model
                .double(PropertyId::TimePos)
                .is_none_or(|time| time < 2.0)
        {
            return;
        }
        let video = core
            .engine
            .tracks()
            .iter()
            .any(|track| track.kind == rvp_engine::TrackKind::Video && !track.album_art);
        if video {
            core.native_failed = path;
            self.show_message(
                "mpv's output could not start in RVP's video window: choose OpenGL in Preferences > Video",
            );
        }
    }

    pub(crate) fn shutdown(mut self: Pin<&mut Self>) {
        crate::video::CLOSING.store(true, Ordering::Release);
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Err(error) = core.app.shutdown(core.engine, Instant::now()) {
            rvp_app::log::report(
                rvp_app::log::Level::Critical,
                &format!("cannot save the playback state: {error}"),
            );
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
        let number = |value: Option<f64>| {
            value
                .filter(|v| v.is_finite())
                .map_or_else(|| "null".to_owned(), |v| format!("{v:.3}"))
        };
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
            ("volume", number(Some(core.app.volume_percent()))),
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
            (
                "stretch",
                (core.app.fullscreen_fit() == Some(rvp_app::Fit::Stretch)).to_string(),
            ),
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
            (
                "skin_file",
                text(core.chrome.as_ref().map(|chrome| chrome.skin().file())),
            ),
            (
                "skin_id",
                integer(
                    core.chrome
                        .as_ref()
                        .and_then(|chrome| i64::try_from(chrome.skin().id()).ok()),
                ),
            ),
            (
                "skin_forces_height",
                core.chrome
                    .as_ref()
                    .is_some_and(|chrome| Mode::Window.limits(chrome.skin()).force_height)
                    .to_string(),
            ),
            // The skin's spectrum: its highest bar now (M14).
            (
                "spectrum",
                core.chrome
                    .as_ref()
                    .map(|chrome| {
                        chrome
                            .data
                            .visual
                            .iter()
                            .fold(0.0_f32, |high, level| high.max(*level))
                    })
                    .map_or_else(|| "null".to_owned(), |high| format!("{high:.3}")),
            ),
        ];
        let body: Vec<String> = fields
            .into_iter()
            .chain(self.playlist_status(core))
            .chain(m11::status(self, core))
            .chain(m16::status(self, core))
            .chain(m9::status(core))
            .map(|(name, value)| format!("\"{name}\":{value}"))
            .collect();
        QString::from(&format!("{{{}}}", body.join(",")))
    }

    /// The albums, the panel and the M3 settings for [`Self::status`].
    fn playlist_status(&self, core: &Core) -> Vec<(&'static str, String)> {
        let model = &core.app.model;
        let number = |value: Option<f64>| {
            value
                .filter(|v| v.is_finite())
                .map_or_else(|| "null".to_owned(), |v| format!("{v:.3}"))
        };
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
            ("volume_max", number(Some(core.app.settings.volume_max))),
            ("speed", number(model.double(PropertyId::Speed))),
            (
                "container_fps",
                number(model.double(PropertyId::ContainerFps)),
            ),
            (
                "estimated_vf_fps",
                number(model.double(PropertyId::EstimatedVfFps)),
            ),
            ("sub_scale", number(model.double(PropertyId::SubScale))),
            ("on_top", self.on_top().to_string()),
            ("info_shown", self.info_shown().to_string()),
            ("current_page", {
                let page = core.chrome.as_ref().map(|chrome| chrome.state.current_page);
                integer(page.map(i64::from))
            }),
        ]
        .into_iter()
        .chain(Self::m6_status(core))
        .chain([
            ("panel_shown", self.panel_shown().to_string()),
            ("recording", core.recorder.is_some().to_string()),
            ("message", text(Some(&self.message().to_string()))),
            ("log_lines", rvp_app::log::counts().0.to_string()),
            (
                "log_open",
                text(
                    rvp_app::log::path()
                        .as_deref()
                        .and_then(std::path::Path::to_str),
                ),
            ),
            ("log_mpv_lines", rvp_app::log::counts().1.to_string()),
        ])
        .collect()
    }

    /// M6's part of [`Self::status`]: the control panel, colours, the
    /// equalizer, shaders and SVP.
    fn m6_status(core: &Core) -> Vec<(&'static str, String)> {
        let settings = &core.app.settings;
        let text = |value: &str| json_string(value);
        let bands: Vec<String> = settings
            .equalizer
            .bands
            .iter()
            .map(|gain| format!("{gain:.1}"))
            .collect();
        let mut fields = vec![
            (
                "panel_page",
                core.panel.as_ref().map_or_else(
                    || "null".to_owned(),
                    |panel| panel.chrome.state.current_page.to_string(),
                ),
            ),
            ("eq_enabled", settings.equalizer.enabled.to_string()),
            ("eq_bands", format!("[{}]", bands.join(","))),
            ("color_enabled", settings.color.enabled.to_string()),
            ("shaders_enabled", settings.shaders.enabled.to_string()),
            ("svp_status", text(&core.app.svp_status())),
            // M18: what smooths the video now.
            ("smoothing", text(&core.app.smoothing_plan().text())),
        ];
        let second = core.app.second();
        fields.push((
            "second_device",
            text(second.map_or("", rvp_app::audio::second::Second::device)),
        ));
        fields.push((
            "second_playing",
            second
                .is_some_and(rvp_app::audio::second::Second::playing)
                .to_string(),
        ));
        fields.push((
            "second_offset_ms",
            second
                .and_then(rvp_app::audio::second::Second::offset)
                .map_or_else(|| "null".to_owned(), |s| format!("{:.1}", s * 1000.0)),
        ));
        for kind in rvp_app::video::ColorKind::ALL {
            fields.push((kind.property(), settings.color.applied(kind).to_string()));
        }
        fields
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
        // JSON has no NaN or infinity (a rate right after a filter goes).
        let float = |name: &str| {
            mpv.get_property::<f64>(name)
                .ok()
                .filter(|value| value.is_finite())
                .map_or_else(|| "null".to_owned(), |value| format!("{value:.4}"))
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
            ("avsync", float("avsync")),
            ("current_vo", text("current-vo")),
            ("current_ao", text("current-ao")),
            ("hwdec_current", text("hwdec-current")),
            ("video_sync", text("video-sync")),
            ("hr_seek_framedrop", text("hr-seek-framedrop")),
            ("vf", text("vf")),
            ("af", text("af")),
            ("glsl_shaders", text("glsl-shaders")),
            ("scale", text("scale")),
            ("dscale", text("dscale")),
            ("cscale", text("cscale")),
            ("deinterlace", text("deinterlace")),
            ("video_rotate", count("video-rotate")),
            ("brightness", count("brightness")),
            ("contrast", count("contrast")),
            ("saturation", count("saturation")),
            ("hue", count("hue")),
            ("gamma", count("gamma")),
            ("width", count("video-params/w")),
            ("height", count("video-params/h")),
            // M13's subtitles.
            ("sid", text("sid")),
            ("secondary_sid", text("secondary-sid")),
            ("sub_text", text("sub-text")),
            ("sub_delay", float("sub-delay")),
            ("sub_bold", text("sub-bold")),
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
