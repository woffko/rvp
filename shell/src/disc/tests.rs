//! DVD and Blu-ray menus on the synthetic discs
//! (`tools/test-clips/make-m16-dvd.sh`, `make-m16-bluray.py`), headless:
//! libmpv without outputs plays the segments RVP's navigators hand it. The
//! discs are `RVP_DVD_MENU` (the folder with `VIDEO_TS`) and `RVP_BD_MENU`
//! (the folder with `BDMV`); each test passes with a note without its disc
//! or its library.

use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use rvp_engine::{Engine, EngineOptions, PlayerModel, PropertyId};

use super::bluray::{self, BluRay, Key};
use super::{Disc, End, Menu, Navigation, Notice, Notify, Picture, available, register};

/// One disc at a time: the segments' stream reads the process's one
/// open disc.
static ONE_DISC: Mutex<()> = Mutex::new(());

struct Harness {
    engine: Engine,
    model: PlayerModel,
    nav: Navigation,
    notices: Receiver<Notice>,
    /// The segment mpv was asked to play.
    address: String,
}

/// A headless player core with the segments' protocol, and the notices'
/// sink.
fn start() -> (Engine, Notify, Receiver<Notice>) {
    // The caller holds `ONE_DISC`.
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    engine.set_text("keep-open", "yes").expect("keep-open");
    if let Some(log) = std::env::var_os("RVP_MPV_LOG") {
        engine
            .set_text("log-file", &log.to_string_lossy())
            .expect("log");
        engine.set_text("msg-level", "all=debug").expect("log");
    }
    register(&engine).expect("protocol");
    let (sender, notices) = mpsc::channel();
    let sender = Mutex::new(sender);
    let notify: Notify = Box::new(move |notice| {
        let _ = sender.lock().map(|s| s.send(notice));
    });
    (engine, notify, notices)
}

impl Harness {
    fn new(engine: Engine, nav: Navigation, notices: Receiver<Notice>) -> Self {
        Self {
            engine,
            model: PlayerModel::new(),
            nav,
            notices,
            address: String::new(),
        }
    }

    fn dvd(&self) -> &Disc {
        match &self.nav {
            Navigation::Dvd(disc) => disc,
            Navigation::Bluray(_) => panic!("not a DVD"),
        }
    }

    fn bd(&self) -> &BluRay {
        match &self.nav {
            Navigation::Bluray(disc) => disc,
            Navigation::Dvd(_) => panic!("not a Blu-ray"),
        }
    }

    fn load_next(&mut self) {
        assert!(
            self.nav.prime(),
            "the next segment has a picture: {:?}",
            self.nav.ended()
        );
        self.load_primed();
    }

    /// Loads the segment the navigator prepared.
    fn load_primed(&mut self) {
        self.address = self.nav.segment_address();
        let format = match self.nav {
            Navigation::Dvd(_) => "mpeg",
            Navigation::Bluray(_) => "mpegts",
        };
        let options = [
            ("demuxer".to_owned(), "lavf".to_owned()),
            ("demuxer-lavf-format".to_owned(), format.to_owned()),
        ];
        self.engine
            .load_with_options(&self.address, &options)
            .expect("loadfile");
        // keep-open paused at the last segment's end.
        self.engine.set_pause(false).expect("play");
    }

    /// Drains mpv until it reaches the segment's end (keep-open); why the
    /// segment ended.
    fn until_end(&mut self, what: &str) -> End {
        // A real disc's opening can be long.
        let deadline = Instant::now() + Duration::from_secs(180);
        let mut events = Vec::new();
        loop {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {events:?}"
            );
            self.engine
                .wait_and_drain(0.05, &mut self.model, &mut events);
            // The new segment's own end, not the last one's.
            if self.model.loaded_path() == Some(self.address.as_str())
                && self.model.flag(PropertyId::EofReached) == Some(true)
                && let Some(end) = self.nav.finish()
            {
                return end;
            }
        }
    }

    /// Drains mpv until the segment ends (why) or plays past `seconds`
    /// (`None`).
    fn until_end_or(&mut self, seconds: f64) -> Option<End> {
        let deadline = Instant::now() + Duration::from_secs(180);
        let mut events = Vec::new();
        loop {
            assert!(Instant::now() < deadline, "timed out: {events:?}");
            self.engine
                .wait_and_drain(0.05, &mut self.model, &mut events);
            if self.model.loaded_path() != Some(self.address.as_str()) {
                continue;
            }
            if self.model.flag(PropertyId::EofReached) == Some(true)
                && let Some(end) = self.nav.finish()
            {
                return Some(end);
            }
            if self.model.double(PropertyId::TimePos).unwrap_or(0.0) >= seconds {
                return None;
            }
        }
    }

    /// Drains mpv until the time passes `seconds`.
    fn until_time(&mut self, seconds: f64) {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut events = Vec::new();
        while self.model.loaded_path() != Some(self.address.as_str())
            || self.model.double(PropertyId::TimePos).unwrap_or(0.0) < seconds
        {
            assert!(
                Instant::now() < deadline,
                "timed out before {seconds} s: time {:?} events {events:?} notices {:?}",
                self.model.double(PropertyId::TimePos),
                self.notices.try_iter().collect::<Vec<_>>()
            );
            self.engine
                .wait_and_drain(0.05, &mut self.model, &mut events);
        }
    }
}

#[test]
fn the_synthetic_disc_menus_navigate() {
    let Some(path) = std::env::var_os("RVP_DVD_MENU") else {
        eprintln!("note: RVP_DVD_MENU is not set (tools/test-clips/make-m16-dvd.sh); skipped");
        return;
    };
    match available() {
        Ok(version) => eprintln!("libdvdnav {version}"),
        Err(why) => {
            eprintln!("note: {why}; skipped");
            return;
        }
    }
    let _one = ONE_DISC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (engine, notify, notices) = start();
    let disc =
        Disc::open(&path.to_string_lossy(), &Default::default(), notify).expect("the disc opens");
    let mut h = Harness::new(engine, Navigation::Dvd(disc), notices);

    // First play goes to the main menu: an endless still with the first
    // of three buttons selected and the menu's picture decoded.
    h.load_next();
    assert_eq!(h.until_end("the main menu's still"), End::Still(255));
    assert!(h.dvd().in_menu(), "{:?}", h.dvd().status());
    assert_eq!(h.dvd().status().domain, 4, "the video manager's menu");
    let button = h.dvd().button(0);
    assert!(button.valid, "{button:?}");
    assert_eq!(
        (button.button, button.x0, button.y0),
        (1, 80, 190),
        "{button:?}"
    );
    let (x, y, width, height, pixels) = h.dvd().menu_picture().expect("the menu's sub-picture");
    assert!(
        width > 0 && height > 0 && pixels.len() == usize::from(width) * usize::from(height) * 4
    );
    assert!(x <= 80 && y <= 190, "{x},{y}");
    // The selection moves with the arrows and the pointer.
    assert!(h.dvd().select(3));
    assert_eq!(h.dvd().button(0).button, 2);
    assert!(h.dvd().select(2));
    assert_eq!(h.dvd().button(0).button, 1);
    assert!(h.dvd().mouse(500, 230, false));
    assert_eq!(h.dvd().button(0).button, 3);
    assert!(h.dvd().mouse(100, 230, false));
    assert_eq!(h.dvd().button(0).button, 1);

    // Play: title 1 plays, ends in a 3 s still, then the main menu again.
    assert!(h.dvd().activate());
    h.load_next();
    h.until_time(1.0);
    let status = h.dvd().status();
    assert_eq!((status.domain, status.title), (2, 1), "{status:?}");
    assert!(!h.dvd().in_menu());
    assert_eq!(h.until_end("title 1's still"), End::Still(3));
    assert!(h.dvd().still_skip());
    h.load_next();
    assert_eq!(h.until_end("the main menu again"), End::Still(255));
    assert_eq!(h.dvd().status().domain, 4);

    // A click on the second button opens the title set's chapter menu.
    assert!(h.dvd().mouse(300, 230, true));
    h.load_next();
    assert_eq!(h.until_end("the chapter menu"), End::Still(255));
    let status = h.dvd().status();
    assert_eq!(status.domain, 8, "{status:?}");
    // Its second button plays chapter 2 (from 4 s).
    assert!(h.dvd().select(3));
    assert!(h.dvd().activate());
    h.load_next();
    h.until_time(0.5);
    let status = h.dvd().status();
    assert_eq!((status.title, status.part), (1, 2), "{status:?}");

    // From the title, the menu calls: the title menu is the main one.
    assert!(h.dvd().menu(Menu::Title));
    h.load_next();
    assert_eq!(h.until_end("the title menu"), End::Still(255));
    assert_eq!(h.dvd().status().domain, 4);
    h.engine.quit().expect("quit");
}

/// A real disc (`RVP_DVD_REAL`, a `VIDEO_TS` folder; the owner's, never
/// committed): its opening plays (timed stills waited out) to a menu with
/// buttons, the selected button's action starts something that plays.
#[test]
fn a_real_disc_reaches_its_menu() {
    let Some(path) = std::env::var_os("RVP_DVD_REAL") else {
        eprintln!("note: RVP_DVD_REAL is not set; skipped");
        return;
    };
    if let Err(why) = available() {
        eprintln!("note: {why}; skipped");
        return;
    }
    let _one = ONE_DISC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (engine, notify, notices) = start();
    let disc =
        Disc::open(&path.to_string_lossy(), &Default::default(), notify).expect("the disc opens");
    let mut h = Harness::new(engine, Navigation::Dvd(disc), notices);
    let mut menu = false;
    // The next segment is prepared already (`End::Next`).
    let mut primed = false;
    for _ in 0..40 {
        if !std::mem::take(&mut primed) && !h.nav.prime() {
            match h.nav.ended() {
                Some(End::Still(255)) => break,
                Some(End::Still(_)) => {
                    h.dvd().still_skip();
                    continue;
                }
                other => panic!("the disc ended: {other:?}"),
            }
        }
        h.load_primed();
        let end = h.until_end_or(3.0);
        let status = h.dvd().status();
        eprintln!("segment {} ({end:?}): {status:?}", h.address);
        // A menu with buttons and its picture, still or moving (a moving
        // one loops: each loop is a segment).
        if h.dvd().in_menu() && h.dvd().button(0).valid && h.dvd().menu_picture().is_some() {
            menu = true;
            break;
        }
        let end = end.unwrap_or_else(|| h.until_end("a segment of the opening"));
        match end {
            End::Still(255) => break,
            End::Still(_) => {
                h.dvd().still_skip();
            }
            End::Next => primed = true,
            End::Jump => {}
            End::Stop | End::Error => panic!("the disc ended: {end:?}"),
        }
    }
    assert!(menu, "no menu with buttons: {:?}", h.dvd().status());
    let button = h.dvd().button(0);
    eprintln!("menu {:?}, button {button:?}", h.dvd().status());
    assert!(h.dvd().menu_picture().is_some(), "the menu's picture");
    assert!(h.dvd().activate());
    h.load_next();
    h.until_time(2.0);
    eprintln!("after the button: {:?}", h.dvd().status());
    h.engine.quit().expect("quit");
}

/// The colour of a pixel of the menu's picture, at plane pixel `x`, `y`.
fn colour_at(picture: &Picture, x: usize, y: usize) -> [u8; 4] {
    let at = ((y - picture.y) * picture.width + (x - picture.x)) * 4;
    picture.pixels[at..at + 4].try_into().expect("a pixel")
}

/// Yellow (the selected button's frame) and grey (the others').
fn yellow(pixel: [u8; 4]) -> bool {
    pixel[0] < 60 && pixel[1] > 200 && pixel[2] > 200 && pixel[3] == 255
}

fn grey(pixel: [u8; 4]) -> bool {
    (100..160).contains(&pixel[0]) && pixel[0] == pixel[1] && pixel[1] == pixel[2]
}

/// The synthetic Blu-ray's buttons: frames at x = 320, 820, 1320, y = 700.
const BD_BUTTONS: [usize; 3] = [320, 820, 1320];
const BD_ROW: usize = 700;

impl Harness {
    /// Which of the Blu-ray's three buttons has the yellow frame.
    fn selected_bd_button(&self) -> Option<usize> {
        let picture = self.bd().picture()?;
        BD_BUTTONS
            .iter()
            .position(|&x| yellow(colour_at(&picture, x + 2, BD_ROW + 2)))
            .map(|index| index + 1)
    }
}

#[test]
fn the_synthetic_bluray_menus_navigate() {
    let Some(path) = std::env::var_os("RVP_BD_MENU") else {
        eprintln!("note: RVP_BD_MENU is not set (tools/test-clips/make-m16-bluray.py); skipped");
        return;
    };
    match bluray::available() {
        Ok(version) => eprintln!("libbluray {version}"),
        Err(why) => {
            eprintln!("note: {why}; skipped");
            return;
        }
    }
    let storage = std::env::temp_dir().join(format!("rvp-bluray-test-{}", std::process::id()));
    let _one = ONE_DISC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (engine, notify, notices) = start();
    let disc = BluRay::open(&path.to_string_lossy(), &storage.to_string_lossy(), notify)
        .expect("the disc opens");
    let mut h = Harness::new(engine, Navigation::Bluray(disc), notices);

    // First play: the menu's playlist ends in an endless still, its
    // interactive graphics show three buttons, the first selected.
    h.load_next();
    assert_eq!(h.until_end("the menu's still"), End::Still(255));
    assert!(h.bd().menu_shown());
    let picture = h.bd().picture().expect("the menu's graphics");
    assert_eq!(picture.frame, Some((1920, 1080)));
    assert_eq!(
        (picture.x, picture.y, picture.width, picture.height),
        (320, 700, 1280, 160)
    );
    assert!(
        grey(colour_at(&picture, 822, 702)),
        "{:?}",
        colour_at(&picture, 822, 702)
    );
    assert_eq!(h.selected_bd_button(), Some(1));
    // The selection moves with the arrows and the pointer.
    assert!(h.bd().key(Key::Right));
    assert_eq!(h.selected_bd_button(), Some(2));
    assert!(h.bd().mouse(1400, 780, false));
    assert_eq!(h.selected_bd_button(), Some(3));

    // A click on the first button plays title 1 with the first audio
    // stream; the menu's graphics go.
    assert!(h.bd().mouse(400, 780, true));
    h.load_next();
    h.until_time(1.0);
    let status = h.bd().status();
    assert_eq!(status.title, 1, "{status:?}");
    assert!(h.bd().picture().is_none());
    assert_eq!(h.nav.streams().audio, [0x1100]);

    // The top menu during the title; its third button chooses the second
    // audio stream, then plays title 1.
    assert!(h.bd().top_menu());
    h.load_next();
    assert_eq!(h.until_end("the top menu"), End::Still(255));
    assert!(h.bd().mouse(1400, 780, true));
    h.load_next();
    h.until_time(0.5);
    assert_eq!(h.nav.streams().audio, [0x1101], "{:?}", h.bd().status());

    // At the title's end its program goes back to the menu.
    assert_eq!(h.until_end("title 1's end"), End::Next);
    h.load_primed();
    assert_eq!(h.until_end("the menu again"), End::Still(255));
    assert!(h.bd().menu_shown());

    // The second button: title 2, title 1's playlist from its second
    // chapter.
    assert!(h.bd().mouse(900, 780, true));
    h.load_next();
    h.until_time(0.3);
    let status = h.bd().status();
    assert_eq!((status.title, status.chapter), (2, 2), "{status:?}");
    h.engine.quit().expect("quit");
    let _ = std::fs::remove_dir_all(storage);
}
