//! M16's part of `Player`: DVD and Blu-ray menus. The app asks for a disc
//! (`App::take_disc_request`); the window opens its navigator
//! (`crate::disc`: libdvdnav or libbluray), loads the disc's segments one
//! after another as each ends, waits out the timed stills, draws the
//! menu's picture (a DVD's sub-picture with its selected button, a
//! Blu-ray's interactive graphics) through mpv's `overlay-add`, and gives
//! the menus the arrows, Enter and the pointer.
//!
//! The navigator's lock is never held while mpv is called: mpv opens a
//! segment on its own thread and the stream's open takes that lock.

use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cxx_qt::Threading;
use rvp_app::UiRequest;
use rvp_app::disc::overlay;
use rvp_app::disc::play::{DiscKind, DiscPlay};
use rvp_app::pointer::Focus;
use rvp_engine::{Engine, EngineEvent, PropertyId};
use serde_json::{Value, json};

use super::Core;
use crate::disc::bluray::BluRay;
use crate::disc::{Disc, End, MenuKey, Navigation, Notice, Notify, Response};
use crate::player_qobject::qobject;

/// mpv's overlay slot for the menus (`overlay-add` takes 0 to 63).
const OVERLAY: &str = "7";

/// The navigator's notices waiting for the window's thread.
const NOTICE_HIGHLIGHT: u8 = 1;
const NOTICE_STREAMS: u8 = 2;

/// Qt's keys the menus take (`Qt::Key`), and the keypad's modifier.
const KEY_RETURN: i32 = 0x0100_0004;
const KEY_ENTER: i32 = 0x0100_0005;
const KEY_LEFT: i32 = 0x0100_0012;
const KEY_UP: i32 = 0x0100_0013;
const KEY_RIGHT: i32 = 0x0100_0014;
const KEY_DOWN: i32 = 0x0100_0015;
const KEYPAD: i32 = 0x2000_0000;

/// The options a segment loads with: mpv opens `rvpdisc://` as a DVD's
/// program stream or a Blu-ray's transport stream.
fn segment_options(kind: DiscKind) -> [(String, String); 2] {
    let format = match kind {
        DiscKind::Dvd => "mpeg",
        DiscKind::Bluray => "mpegts",
    };
    [
        ("demuxer".to_owned(), "lavf".to_owned()),
        ("demuxer-lavf-format".to_owned(), format.to_owned()),
    ]
}

/// The OSD's size and the video's margins in it (`osd-dimensions`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Osd {
    width: f64,
    height: f64,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Osd {
    fn read(engine: &Engine) -> Option<Self> {
        #[allow(clippy::cast_precision_loss)] // OSD sizes are small.
        let get = |name: &str| {
            engine
                .get_int(&format!("osd-dimensions/{name}"))
                .map(|v| v as f64)
        };
        Some(Self {
            width: get("w")?,
            height: get("h")?,
            left: get("ml")?,
            top: get("mt")?,
            right: get("mr")?,
            bottom: get("mb")?,
        })
    }

    /// The video's width and height in OSD pixels.
    fn video(&self) -> (f64, f64) {
        (
            self.width - self.left - self.right,
            self.height - self.top - self.bottom,
        )
    }
}

/// At start: disc menus need libdvdnav (DVD) or libbluray (Blu-ray)
/// beside RVP, and the segments' protocol on the player's core.
pub(super) fn start(app: &mut rvp_app::App, engine: &Engine) {
    let protocol = crate::disc::register(engine);
    let dvd = protocol
        .clone()
        .and_then(|()| crate::disc::available().map(drop));
    let bluray = protocol.and_then(|()| crate::disc::bluray::available().map(drop));
    if let Err(why) = &dvd {
        rvp_app::log::report(rvp_app::log::Level::Warning, &format!("DVD menus: {why}"));
    }
    if let Err(why) = &bluray {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("Blu-ray menus: {why}"),
        );
    }
    app.set_disc_menus_available(dvd.is_ok(), bluray.is_ok());
}

/// A disc that plays with its menus.
pub(super) struct Playing {
    disc: Navigation,
    kind: DiscKind,
    /// The segment mpv plays (`rvpdisc://N`), and whether its end was
    /// handled.
    address: String,
    handled: bool,
    /// A timed still ends then.
    still_until: Option<Instant>,
    /// The picture mpv shows over the video, kept while it shows.
    shown: Option<Vec<u8>>,
    /// The OSD when the picture was last drawn, and the frame's size: the
    /// pointer's place in the frame.
    osd: Option<Osd>,
    frame: (f64, f64),
    /// The window's layout changed: the OSD is read again until then.
    recheck_until: Option<Instant>,
    /// The navigator's notices for the window's thread.
    notices: Arc<AtomicU8>,
    /// A press that clicked a button: its release is the menu's too.
    swallow_release: bool,
}

impl qobject::Player {
    /// Opens the disc the app asks for, or lets the menus go when another
    /// file plays.
    pub(super) fn follow_disc(mut self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(play) = core.app.take_disc_request() {
            close(core);
            let notices = Arc::new(AtomicU8::new(0));
            let pending = Arc::clone(&notices);
            let thread = Mutex::new(thread);
            // On mpv's demuxer thread: only queue work.
            let notify: Notify = Box::new(move |notice| {
                let bit = match notice {
                    Notice::Highlight => NOTICE_HIGHLIGHT,
                    Notice::Streams => NOTICE_STREAMS,
                    Notice::Position => return,
                };
                if pending.fetch_or(bit, Ordering::AcqRel) == 0
                    && let Ok(thread) = thread.lock()
                {
                    let _ = thread.queue(qobject::Player::disc_notices);
                }
            });
            let opened = match play.kind {
                DiscKind::Dvd => {
                    Disc::open(&play.device, &play.languages, notify).map(Navigation::Dvd)
                }
                DiscKind::Bluray => {
                    let storage = core.app.state_dir().join("bluray");
                    BluRay::open(&play.device, &storage.to_string_lossy(), notify)
                        .map(Navigation::Bluray)
                }
            };
            match opened {
                Ok(disc) => {
                    core.disc = Some(Playing {
                        disc,
                        kind: play.kind,
                        address: String::new(),
                        handled: true,
                        still_until: None,
                        shown: None,
                        osd: None,
                        frame: (0.0, 0.0),
                        recheck_until: None,
                        notices,
                        swallow_release: false,
                    });
                    if play.skip_opening {
                        self.disc_skip_opening();
                    } else {
                        self.disc_advance();
                    }
                }
                Err(error) => self.disc_without_menus(&play, &error),
            }
        } else if core.disc.is_some() && core.app.disc_play().is_none() {
            close(core);
        }
    }

    /// Straight to the disc's menu: the program starts (its first play
    /// runs to its first picture), then the disc's menu is called (a DVD's
    /// title or root menu, a Blu-ray's top menu); a disc without plays from
    /// its start.
    fn disc_skip_opening(mut self: Pin<&mut Self>) {
        let Some(playing) = self.as_mut().core_mut().and_then(|core| core.disc.as_mut()) else {
            return;
        };
        if !playing.disc.prime() {
            let end = playing.disc.ended().unwrap_or(End::Error);
            self.disc_end(end);
        } else if playing.disc.skip_opening() {
            self.disc_advance();
        } else {
            self.load_segment();
        }
    }

    /// The navigator cannot open the disc: it plays as mpv's `dvd://` or
    /// `bd://`.
    fn disc_without_menus(mut self: Pin<&mut Self>, play: &DiscPlay, error: &str) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let result = core.engine.load(&play.entry, false);
        let message = match result {
            Ok(()) => format!("Disc menus: {error}; the disc plays without them"),
            Err(load) => format!("Disc menus: {error}; {load}"),
        };
        self.set_osd(cxx_qt_lib::QString::from(&message));
    }

    /// Loads the disc's next segment, or acts on the end that comes before
    /// it.
    fn disc_advance(mut self: Pin<&mut Self>) {
        let Some(playing) = self.as_mut().core_mut().and_then(|core| core.disc.as_mut()) else {
            return;
        };
        if playing.disc.prime() {
            self.load_segment();
        } else {
            let end = playing.disc.ended().unwrap_or(End::Error);
            self.disc_end(end);
        }
    }

    /// Loads the segment the navigator prepared.
    fn load_segment(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        playing.still_until = None;
        playing.address = playing.disc.segment_address();
        playing.handled = false;
        let loaded = core
            .engine
            .load_with_options(&playing.address, &segment_options(playing.kind))
            // keep-open paused at the last segment's end.
            .and_then(|()| core.engine.set_pause(false));
        match loaded {
            Ok(()) => self.draw_disc_menu(),
            Err(error) => {
                rvp_app::log::report(
                    rvp_app::log::Level::Warning,
                    &format!("disc segment: {error}"),
                );
                self.disc_end(End::Error);
            }
        }
    }

    /// A segment ended (or the navigator stopped before one began).
    fn disc_end(mut self: Pin<&mut Self>, end: End) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        match end {
            // A menu or a picture until the user acts.
            End::Still(255) => self.draw_disc_menu(),
            End::Still(seconds) => {
                playing.still_until =
                    Some(Instant::now() + Duration::from_secs(u64::from(seconds)));
                self.draw_disc_menu();
            }
            End::Jump => self.disc_advance(),
            End::Next => self.load_segment(),
            End::Stop | End::Error => {
                if end == End::Error {
                    rvp_app::log::report(
                        rvp_app::log::Level::Critical,
                        &format!("the disc's navigation failed: {}", playing.disc.error()),
                    );
                }
                close(core);
                let requests = core.app.disc_finished(core.engine, Instant::now());
                for request in requests {
                    self.as_mut().handle_request(request);
                }
            }
        }
    }

    /// After a drain: a segment that loaded takes the disc's streams, and
    /// one mpv played to its end hands on to the next.
    pub(super) fn disc_drained(mut self: Pin<&mut Self>, events: &[EngineEvent]) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        if core.app.model.loaded_path() != Some(playing.address.as_str()) {
            return;
        }
        if events
            .iter()
            .any(|event| matches!(event, EngineEvent::FileLoaded { .. }))
        {
            apply_streams(core);
            self.as_mut().draw_disc_menu();
        }
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        if playing.handled || core.app.model.flag(PropertyId::EofReached) != Some(true) {
            return;
        }
        let Some(end) = playing.disc.finish() else {
            return;
        };
        playing.handled = true;
        self.disc_end(end);
    }

    /// The navigator's notices, on the window's thread.
    pub(super) fn disc_notices(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_ref() else {
            return;
        };
        let notices = playing.notices.swap(0, Ordering::AcqRel);
        if notices & NOTICE_STREAMS != 0 {
            apply_streams(core);
        }
        if notices & NOTICE_HIGHLIGHT != 0 {
            self.draw_disc_menu();
        }
    }

    /// About ten times a second: a timed still that ran out goes on, and
    /// after a layout change the menu's picture follows the video's new
    /// place (mpv sizes its OSD with the next frame it draws).
    pub(super) fn disc_tick(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        let now = Instant::now();
        if playing.still_until.is_some_and(|until| now >= until) {
            playing.still_until = None;
            playing.disc.still_skip();
            self.disc_advance();
            return;
        }
        // A Blu-ray's program runs on while it waits (timers, BD-J).
        if playing.handled && playing.still_until.is_none() && playing.disc.poll() {
            self.load_segment();
            return;
        }
        let Some(until) = playing.recheck_until else {
            return;
        };
        if now >= until {
            playing.recheck_until = None;
        }
        if playing.shown.is_some() && Osd::read(core.engine) != playing.osd {
            self.draw_disc_menu();
        }
    }

    /// The window's layout changed (its size, the bars over the video).
    pub(super) fn disc_layout_changed(self: Pin<&mut Self>) {
        if let Some(playing) = self.core_mut().and_then(|core| core.disc.as_mut()) {
            playing.recheck_until = Some(Instant::now() + Duration::from_secs(1));
        }
    }

    /// Shows the menu's sub-picture with its selected button over the
    /// video, scaled to the video's place in the OSD; removes it outside
    /// menus.
    pub(super) fn draw_disc_menu(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        let video = (
            core.app.model.int(PropertyId::VideoWidth),
            core.app.model.int(PropertyId::VideoHeight),
        );
        let Some(picture) = playing.disc.picture() else {
            remove_overlay(core.engine, playing);
            return;
        };
        let frame = match (picture.frame, video) {
            (Some(frame), _) => frame,
            (None, (Some(width), Some(height))) => (
                usize::try_from(width).unwrap_or(0),
                usize::try_from(height).unwrap_or(0),
            ),
            (None, _) => return,
        };
        let Some(osd) = Osd::read(core.engine) else {
            return;
        };
        let (video_width, video_height) = osd.video();
        if video_width < 1.0 || video_height < 1.0 || frame.0 == 0 || frame.1 == 0 {
            return;
        }
        // OSD sizes: small positive numbers.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let to_osd = |value: f64| value.round().max(0.0) as usize;
        let view = overlay::View {
            frame,
            video: (osd.left, osd.top, video_width, video_height),
            osd: (to_osd(osd.width), to_osd(osd.height)),
        };
        let Some(placed) = overlay::place(
            &picture.pixels,
            (picture.x, picture.y),
            (picture.width, picture.height),
            &view,
        ) else {
            remove_overlay(core.engine, playing);
            return;
        };
        let (left, top, to_width, to_height) =
            (placed.left, placed.top, placed.width, placed.height);
        let scaled = placed.pixels;
        // mpv reads the pixels at this address while the command runs;
        // `shown` keeps them until the overlay is replaced or removed.
        let address = format!("&{}", scaled.as_ptr() as usize);
        let args = [
            OVERLAY.to_owned(),
            left.to_string(),
            top.to_string(),
            address,
            "0".to_owned(),
            "bgra".to_owned(),
            to_width.to_string(),
            to_height.to_string(),
            (to_width * 4).to_string(),
        ];
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        if let Err(error) = core.engine.command("overlay-add", &args) {
            rvp_app::log::report(
                rvp_app::log::Level::Warning,
                &format!("disc menu picture: {error}"),
            );
            return;
        }
        playing.shown = Some(scaled);
        playing.osd = Some(osd);
        // Frame sizes are small.
        #[allow(clippy::cast_precision_loss)]
        let frame = (frame.0 as f64, frame.1 as f64);
        playing.frame = frame;
    }

    /// Arrows and Enter in a menu with buttons; Enter also ends a still.
    /// True when the key was the disc's.
    pub(super) fn disc_key(mut self: Pin<&mut Self>, key: i32, modifiers: i32) -> bool {
        let Some(core) = self.as_mut().core_mut() else {
            return false;
        };
        let Some(playing) = core.disc.as_mut() else {
            return false;
        };
        if core.focus != Focus::Window || modifiers & !KEYPAD != 0 {
            return false;
        }
        let menu_key = match key {
            KEY_UP => MenuKey::Up,
            KEY_DOWN => MenuKey::Down,
            KEY_LEFT => MenuKey::Left,
            KEY_RIGHT => MenuKey::Right,
            KEY_RETURN | KEY_ENTER => MenuKey::Enter,
            _ => return false,
        };
        if playing.disc.menu_keys() {
            let response = playing.disc.key(menu_key);
            self.disc_respond(response);
            return true;
        }
        if menu_key == MenuKey::Enter && (playing.still_until.is_some() || playing.handled) {
            playing.disc.still_skip();
            self.disc_advance();
            return true;
        }
        false
    }

    /// The pointer over a menu (window coordinates): a move selects the
    /// button under it, a left press clicks it. True when a press was the
    /// menu's.
    pub(super) fn disc_pointer(mut self: Pin<&mut Self>, x: f64, y: f64, press: bool) -> bool {
        let rect = self.video_rect().clone();
        let Some(core) = self.as_mut().core_mut() else {
            return false;
        };
        let Some(playing) = core.disc.as_mut() else {
            return false;
        };
        let Some(osd) = playing.osd else {
            return false;
        };
        if playing.shown.is_none() || rect.width() < 1.0 || rect.height() < 1.0 {
            return false;
        }
        let (video_width, video_height) = osd.video();
        let to_frame = |value: f64,
                        origin: f64,
                        size: f64,
                        osd_size: f64,
                        margin: f64,
                        video: f64,
                        frame: f64| {
            ((value - origin) / size * osd_size - margin) / video * frame
        };
        let fx = to_frame(
            x,
            rect.x(),
            rect.width(),
            osd.width,
            osd.left,
            video_width,
            playing.frame.0,
        );
        let fy = to_frame(
            y,
            rect.y(),
            rect.height(),
            osd.height,
            osd.top,
            video_height,
            playing.frame.1,
        );
        if !(0.0..playing.frame.0).contains(&fx) || !(0.0..playing.frame.1).contains(&fy) {
            return false;
        }
        // Inside the frame: small positive numbers.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (fx, fy) = (fx as usize, fy as usize);
        let response = playing.disc.mouse(fx, fy, press);
        if press && response == Response::Advance {
            playing.swallow_release = true;
        }
        let clicked = response == Response::Advance;
        self.disc_respond(response);
        clicked
    }

    /// Carries out what the menu did: draws its new picture, or goes on to
    /// the next segment.
    fn disc_respond(self: Pin<&mut Self>, response: Response) {
        match response {
            Response::Ignored => {}
            Response::Redraw => self.draw_disc_menu(),
            Response::Advance => self.disc_advance(),
        }
    }

    /// The release of a press that clicked a menu button.
    pub(super) fn disc_release(self: Pin<&mut Self>) -> bool {
        let Some(core) = self.core_mut() else {
            return false;
        };
        core.disc
            .as_mut()
            .is_some_and(|playing| std::mem::take(&mut playing.swallow_release))
    }

    pub(super) fn m16_request(mut self: Pin<&mut Self>, request: &UiRequest) {
        let &UiRequest::DiscMenu(menu) = request else {
            rvp_app::log::report(
                rvp_app::log::Level::Warning,
                &format!("unhandled request {request:?}"),
            );
            return;
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(playing) = core.disc.as_mut() else {
            return;
        };
        match playing.disc.menu(menu) {
            Response::Ignored => {
                self.set_osd(cxx_qt_lib::QString::from("The disc has no such menu here"));
            }
            response => self.disc_respond(response),
        }
    }
}

/// The disc's state for `Player::status` (the self-test): the machine's
/// place, the selected button, the menu's picture, the stills, the
/// streams mpv plays, and where frame pixels lie in the window (`x0` +
/// `sx` * x).
pub(super) fn status(player: &qobject::Player, core: &Core) -> Vec<(&'static str, String)> {
    let disc = core.disc.as_ref().map_or(Value::Null, |playing| {
        let rect = player.video_rect();
        let window = playing.osd.map(|osd| {
            let (video_width, video_height) = osd.video();
            let (sx, sy) = (
                rect.width() / osd.width * video_width / playing.frame.0,
                rect.height() / osd.height * video_height / playing.frame.1,
            );
            json!({
                "x0": rect.x() + osd.left * rect.width() / osd.width,
                "y0": rect.y() + osd.top * rect.height() / osd.height,
                "sx": sx,
                "sy": sy,
            })
        });
        let mut value = playing.disc.status_json();
        if let Some(fields) = value.as_object_mut() {
            fields.insert("picture".to_owned(), json!(playing.shown.is_some()));
            // The menu's picture changes (a Blu-ray's selection shows only
            // there): FNV-1a of the pixels mpv shows.
            let digest = playing.shown.as_ref().map(|pixels| {
                pixels
                    .iter()
                    .fold(0xcbf2_9ce4_8422_2325_u64, |hash, &byte| {
                        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
                    })
            });
            fields.insert(
                "picture_digest".to_owned(),
                json!(digest.map(|d| format!("{d:016x}"))),
            );
            fields.insert("still".to_owned(), json!(playing.handled));
            fields.insert(
                "timed_still".to_owned(),
                json!(playing.still_until.is_some()),
            );
            fields.insert("segment".to_owned(), json!(playing.address));
            fields.insert(
                "audio_source".to_owned(),
                json!(core.engine.get_int("current-tracks/audio/src-id")),
            );
            fields.insert(
                "sub_source".to_owned(),
                json!(core.engine.get_int("current-tracks/sub/src-id")),
            );
            fields.insert("window".to_owned(), json!(window));
        }
        value
    });
    vec![
        ("disc", disc.to_string()),
        ("disc_menus", crate::disc::available().is_ok().to_string()),
        (
            "bluray_menus",
            crate::disc::bluray::available().is_ok().to_string(),
        ),
    ]
}

/// Lets the disc go: no picture over the video.
fn close(core: &mut Core) {
    if let Some(mut playing) = core.disc.take() {
        remove_overlay(core.engine, &mut playing);
    }
}

fn remove_overlay(engine: &Engine, playing: &mut Playing) {
    if playing.shown.take().is_some() {
        drop(engine.command("overlay-remove", &[OVERLAY]));
    }
}

/// The disc's program chose the audio and subtitle streams: mpv plays
/// those (no subtitles in a menu: RVP draws the menu itself).
fn apply_streams(core: &Core) {
    let Some(playing) = core.disc.as_ref() else {
        return;
    };
    let streams = playing.disc.streams();
    let engine = core.engine;
    if streams.menu {
        drop(engine.set_text("sid", "no"));
        return;
    }
    if let Some(id) = streams
        .audio
        .iter()
        .find_map(|&source| track_with_source(engine, "audio", source))
    {
        drop(engine.set_text("aid", &id.to_string()));
    }
    let sid = streams
        .subtitle
        .and_then(|source| track_with_source(engine, "sub", source));
    drop(engine.set_text(
        "sid",
        &sid.map_or_else(|| "no".to_owned(), |id| id.to_string()),
    ));
}

/// mpv's ID of the `kind` track whose source ID is `source`.
fn track_with_source(engine: &Engine, kind: &str, source: i64) -> Option<i64> {
    let count = engine.get_int("track-list/count")?;
    (0..count).find_map(|index| {
        let matches = engine
            .get_text(&format!("track-list/{index}/type"))
            .as_deref()
            == Some(kind)
            && engine.get_int(&format!("track-list/{index}/src-id")) == Some(source);
        if matches {
            engine.get_int(&format!("track-list/{index}/id"))
        } else {
            None
        }
    })
}
