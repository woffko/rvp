//! Blu-ray menus (M16): libbluray runs the disc's HDMV programs (and BD-J
//! with the user's Java) and hands RVP the transport stream, which mpv
//! plays one segment at a time as the DVD's (`super`); libbluray draws the
//! menus' interactive graphics with overlay events, which RVP keeps on a
//! plane (`rvp_app::disc::bluray`) and shows over the video.
//!
//! * libbluray reports a new playlist, a seek of its own, a still and an
//!   error without data: those end a segment that handed data over.
//! * A segment that starts with a playlist is seekable by bytes of the
//!   title (`bd_seek`); its data ends at the title's size, and libbluray
//!   reads on (to the program's next command) only when mpv reached it.
//! * Keys and the pointer go to libbluray while a menu shows; it redraws
//!   the plane at once and runs a button's commands at the next read.

use std::sync::{Arc, Mutex};

use rvp_app::disc::bluray::{Image, Overlay, Plane};

use super::{End, Notice, Notify, PROTOCOL, Picture, Segments, lock, stream};
use crate::bluray_ffi::bridge::{self as ffi, BdEvent, BdInfo, BdOverlay, BlurayNavigator};

/// libbluray reads the stream in aligned units of this many bytes.
const UNIT: usize = 6144;

/// Why an encrypted disc does not play.
const ENCRYPTED: &str = "the disc is encrypted (AACS): it needs the user's own libaacs and keys";

/// Reads without data or an event before the disc counts as waiting.
const IDLE_READS: u32 = 3;

/// libbluray's events (`bd_event_e`).
mod event {
    pub const ERROR: u32 = 1;
    pub const ENCRYPTED: u32 = 3;
    pub const TITLE: u32 = 5;
    pub const PLAYLIST: u32 = 6;
    pub const CHAPTER: u32 = 8;
    pub const AUDIO_STREAM: u32 = 11;
    pub const PG_TEXTST_STREAM: u32 = 13;
    pub const PG_TEXTST: u32 = 17;
    pub const DISCONTINUITY: u32 = 23;
    pub const SEEK: u32 = 24;
    pub const STILL: u32 = 25;
    pub const STILL_TIME: u32 = 26;
    pub const POPUP: u32 = 29;
    pub const MENU: u32 = 30;
}

/// libbluray's keys (`bd_vk_key_e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    Popup = 11,
    Up = 12,
    Down = 13,
    Left = 14,
    Right = 15,
    Enter = 16,
}

/// The navigator and what one segment has handed over.
struct Navigator {
    bd: cxx::UniquePtr<BlurayNavigator>,
    block: Vec<u8>,
    pending: Vec<u8>,
    taken: usize,
    delivered: bool,
    ended: Option<End>,
    /// A playlist began in this segment before its first data.
    playlist_started: bool,
    /// Where a seekable segment starts in its title (bytes), and the
    /// title's size.
    start: Option<(u64, u64)>,
    data_end: bool,
    segment: u64,
    plane: Plane,
    /// The still the disc waits in, until libbluray reads on.
    still: Option<End>,
    /// The title, chapter, menus and streams libbluray reported.
    state: Status,
    /// Why libbluray stopped, where it said.
    error: Option<&'static str>,
}

impl Navigator {
    fn bd(&mut self) -> std::pin::Pin<&mut BlurayNavigator> {
        self.bd.pin_mut()
    }

    fn begin(&mut self) {
        self.pending.clear();
        self.taken = 0;
        self.delivered = false;
        self.ended = None;
        self.playlist_started = false;
        self.start = None;
        self.data_end = false;
    }

    /// The overlay events libbluray queued, onto the plane; a flush tells
    /// the window.
    fn take_overlays(&mut self, notify: &Notify) {
        let overlays = ffi::rvp_bluray_overlays(self.bd());
        let mut shown = false;
        for overlay in &overlays {
            shown |= self.plane.apply(&convert(overlay));
        }
        if shown {
            notify(Notice::Highlight);
        }
    }

    fn event(&mut self, event: BdEvent, notify: &Notify) {
        let param = event.param;
        match event.event {
            event::PLAYLIST => {
                if self.delivered {
                    self.ended = Some(End::Jump);
                } else {
                    self.playlist_started = true;
                }
                self.still = None;
            }
            event::SEEK | event::DISCONTINUITY if self.delivered => self.ended = Some(End::Jump),
            event::STILL_TIME => {
                let seconds = u8::try_from(param).unwrap_or(254).min(254);
                let end = End::Still(if param == 0 { 255 } else { seconds });
                self.still = Some(end);
                self.ended = Some(end);
            }
            event::STILL if param != 0 => {
                self.still = Some(End::Still(255));
                self.ended = Some(End::Still(255));
            }
            event::STILL => self.still = None,
            event::ENCRYPTED => {
                self.error = Some(ENCRYPTED);
                self.ended = Some(End::Error);
            }
            event::ERROR => self.ended = Some(End::Error),
            event::MENU => {
                self.state.menu = param != 0;
                notify(Notice::Highlight);
            }
            event::POPUP => self.state.popup = param != 0,
            event::AUDIO_STREAM => {
                self.state.audio = param;
                notify(Notice::Streams);
            }
            event::PG_TEXTST_STREAM => {
                self.state.subtitle = param;
                notify(Notice::Streams);
            }
            event::PG_TEXTST => {
                self.state.subtitle_shown = param != 0;
                notify(Notice::Streams);
            }
            event::TITLE => {
                self.state.title = param;
                notify(Notice::Position);
            }
            event::CHAPTER => {
                self.state.chapter = param;
                notify(Notice::Position);
            }
            _ => {}
        }
    }

    /// Reads to the next data (kept in `pending`), handling the events on
    /// the way; false when the segment ends first, or the disc waits (a
    /// still, a menu without video).
    fn advance(&mut self, notify: &Notify) -> bool {
        let mut idle = 0;
        loop {
            if self.ended.is_some() {
                return false;
            }
            let mut events = Vec::new();
            let length = {
                let Navigator { bd, block, .. } = self;
                ffi::rvp_bluray_read(bd.pin_mut(), block.as_mut_slice(), &mut events)
            };
            self.take_overlays(notify);
            for event in &events {
                self.event(*event, notify);
            }
            if length < 0 {
                self.ended = Some(End::Error);
                return false;
            }
            let size = usize::try_from(length).unwrap_or(0).min(UNIT);
            if size > 0 {
                self.pending.clear();
                self.pending.extend_from_slice(&self.block[..size]);
                self.taken = 0;
                self.delivered = true;
                self.still = None;
                if let Some((_, title)) = self.start {
                    self.data_end = ffi::rvp_bluray_tell(self.bd()) >= title;
                }
                return true;
            }
            if events.is_empty() {
                idle += 1;
                if idle >= IDLE_READS {
                    // Nothing new: the disc waits (in its still, or a menu).
                    self.ended = Some(self.still.unwrap_or(End::Still(255)));
                }
            }
        }
    }
}

/// libbluray's event as the plane's.
fn convert(overlay: &BdOverlay) -> Overlay {
    let image = if overlay.argb {
        Image::Argb(overlay.data.clone())
    } else if overlay.data.is_empty() {
        Image::None
    } else {
        Image::Runs {
            // Packed by `cpp/bluray.cpp`: length << 16 | colour (0 to 255).
            runs: overlay
                .data
                .iter()
                .map(|&run| {
                    let [length_high, length_low, _, colour] = run.to_be_bytes();
                    (u16::from_be_bytes([length_high, length_low]), colour)
                })
                .collect(),
            palette: overlay
                .palette
                .iter()
                .map(|entry| entry.to_be_bytes())
                .collect(),
        }
    };
    Overlay {
        plane: overlay.plane,
        command: overlay.command,
        x: overlay.x,
        y: overlay.y,
        width: overlay.width,
        height: overlay.height,
        image,
    }
}

struct Shared {
    nav: Mutex<Navigator>,
    notify: Notify,
}

impl Shared {
    fn prime(&self) -> bool {
        let mut nav = lock(&self.nav);
        nav.segment += 1;
        nav.begin();
        if !nav.advance(&self.notify) {
            return false;
        }
        if nav.playlist_started {
            let title = ffi::rvp_bluray_title_size(nav.bd());
            let at = ffi::rvp_bluray_tell(nav.bd()).saturating_sub(nav.pending.len() as u64);
            nav.start = (title > at).then_some((at, title));
        }
        true
    }
}

impl Segments for Shared {
    fn bind(&self, segment: u64) -> bool {
        let nav = lock(&self.nav);
        nav.segment == segment && nav.start.is_some()
    }

    fn read(&self, segment: u64, out: &mut [u8]) -> usize {
        let mut nav = lock(&self.nav);
        if nav.segment != segment {
            return 0;
        }
        loop {
            if nav.taken < nav.pending.len() {
                let count = out.len().min(nav.pending.len() - nav.taken);
                let from = nav.taken;
                out[..count].copy_from_slice(&nav.pending[from..from + count]);
                nav.taken += count;
                return count;
            }
            if nav.data_end || !nav.advance(&self.notify) {
                return 0;
            }
        }
    }

    /// libbluray seeks to the access point at or before the byte; the
    /// bytes up to it are read past.
    fn seek(&self, segment: u64, offset: u64) -> Option<u64> {
        let mut nav = lock(&self.nav);
        if nav.segment != segment {
            return None;
        }
        let (start, _) = nav.start?;
        let target = start + offset;
        let reached = u64::try_from(ffi::rvp_bluray_seek(nav.bd(), target)).ok()?;
        nav.pending.clear();
        nav.taken = 0;
        nav.ended = None;
        nav.data_end = false;
        // libbluray's own SEEK event does not end the segment.
        nav.delivered = false;
        let mut skip = target.saturating_sub(reached);
        while nav.advance(&self.notify) {
            let length = nav.pending.len() as u64;
            if skip < length {
                nav.taken = usize::try_from(skip).unwrap_or(0);
                break;
            }
            skip -= length;
            nav.pending.clear();
        }
        Some(offset)
    }

    fn size(&self, segment: u64) -> Option<u64> {
        let nav = lock(&self.nav);
        if nav.segment != segment {
            return None;
        }
        let (start, title) = nav.start?;
        Some(title - start)
    }
}

/// Where an open Blu-ray is, as libbluray's events said: the disc's title
/// (0 the top menu, 0xffff the first play), the chapter, the menus and the
/// streams.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Status {
    pub title: u32,
    pub chapter: u32,
    pub menu: bool,
    pub popup: bool,
    /// Primary audio stream (1 to 32), subtitle stream (1 to 255) and
    /// whether subtitles show.
    pub audio: u32,
    pub subtitle: u32,
    pub subtitle_shown: bool,
}

/// An open Blu-ray: its navigator, shared with the segments mpv reads.
pub(crate) struct BluRay {
    shared: Arc<Shared>,
}

impl BluRay {
    /// Opens a disc, a folder holding `BDMV` or an image, and starts its
    /// first play; BD-J keeps its files under `storage`.
    ///
    /// # Errors
    /// Fails without libbluray, when it cannot open `path`, or when the
    /// disc is encrypted and the user's own libaacs and keys cannot open
    /// it.
    pub(crate) fn open(path: &str, storage: &str, notify: Notify) -> Result<Self, String> {
        let mut error = String::new();
        let mut bd = ffi::rvp_bluray_open(path, storage, &mut error);
        if bd.is_null() {
            return Err(error);
        }
        let info: BdInfo = ffi::rvp_bluray_info(bd.pin_mut());
        if !info.bluray {
            return Err("no Blu-ray there".to_owned());
        }
        if info.aacs && !info.aacs_handled {
            return Err(ENCRYPTED.to_owned());
        }
        if !info.menus || !(info.first_play || info.top_menu) {
            return Err("the disc has no menus".to_owned());
        }
        if info.hdmv_titles == 0 && !info.bdj_handled {
            return Err("the disc's menus are BD-J: they need Java".to_owned());
        }
        if !ffi::rvp_bluray_play(bd.pin_mut()) {
            return Err("libbluray cannot start the disc".to_owned());
        }
        let shared = Arc::new(Shared {
            nav: Mutex::new(Navigator {
                bd,
                block: vec![0; UNIT],
                pending: Vec::with_capacity(UNIT),
                taken: 0,
                delivered: false,
                ended: None,
                playlist_started: false,
                start: None,
                data_end: false,
                segment: 0,
                plane: Plane::default(),
                still: None,
                state: Status::default(),
                error: None,
            }),
            notify,
        });
        stream::set_current(Some(Arc::clone(&shared) as Arc<dyn Segments>));
        Ok(Self { shared })
    }

    /// Prepares the next segment: false when a still or the disc's end
    /// comes first ([`BluRay::ended`]).
    pub(crate) fn prime(&self) -> bool {
        self.shared.prime()
    }

    pub(crate) fn segment_address(&self) -> String {
        format!("{PROTOCOL}://{}", lock(&self.shared.nav).segment)
    }

    pub(crate) fn ended(&self) -> Option<End> {
        lock(&self.shared.nav).ended
    }

    /// As the DVD's `finish`: why the segment ended once mpv reached its
    /// end, reading past a title's end only now.
    pub(crate) fn finish(&self) -> Option<End> {
        let nav = lock(&self.shared.nav);
        if nav.ended.is_some() || !nav.data_end {
            return nav.ended;
        }
        drop(nav);
        if self.prime() {
            Some(End::Next)
        } else {
            Some(self.ended().unwrap_or(End::Error))
        }
    }

    pub(crate) fn still_skip(&self) -> bool {
        let mut nav = lock(&self.shared.nav);
        nav.still = None;
        ffi::rvp_bluray_skip_still(nav.bd())
    }

    /// A menu shows (always on or pop-up): keys and the pointer are the
    /// menu's.
    pub(crate) fn menu_shown(&self) -> bool {
        let nav = lock(&self.shared.nav);
        nav.state.menu || nav.plane.picture().is_some()
    }

    /// A key for the menu; libbluray redraws the plane at once.
    pub(crate) fn key(&self, key: Key) -> bool {
        let mut nav = lock(&self.shared.nav);
        let done = ffi::rvp_bluray_key(nav.bd(), key as u32);
        nav.take_overlays(&self.shared.notify);
        done
    }

    /// The pointer over the plane (plane pixels); `activate` clicks. True
    /// when it is over a button.
    pub(crate) fn mouse(&self, x: u16, y: u16, activate: bool) -> bool {
        let mut nav = lock(&self.shared.nav);
        let over = ffi::rvp_bluray_mouse(nav.bd(), x, y, activate);
        nav.take_overlays(&self.shared.notify);
        over
    }

    /// The disc's top menu.
    pub(crate) fn top_menu(&self) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_bluray_menu(nav.bd())
    }

    /// The plane's visible part, in plane pixels (the plane is the
    /// frame).
    pub(crate) fn picture(&self) -> Option<Picture> {
        let nav = lock(&self.shared.nav);
        let (x, y, width, height, pixels) = nav.plane.picture()?;
        Some(Picture {
            x,
            y,
            width,
            height,
            pixels,
            frame: Some(nav.plane.size()),
        })
    }

    pub(crate) fn status(&self) -> Status {
        lock(&self.shared.nav).state
    }

    /// While the disc waits (a still, a menu), libbluray runs its timers
    /// and BD-J at each read: true when it moved on to new data, which a
    /// new segment then plays.
    pub(crate) fn poll(&self) -> bool {
        let nav = lock(&self.shared.nav);
        if nav.ended.is_none() {
            return false;
        }
        drop(nav);
        self.prime()
    }

    /// Why libbluray stopped, where it said.
    pub(crate) fn error(&self) -> Option<&'static str> {
        lock(&self.shared.nav).error
    }
}

impl Drop for BluRay {
    fn drop(&mut self) {
        stream::clear_current(&(Arc::clone(&self.shared) as Arc<dyn Segments>));
    }
}

/// Whether libbluray loads here, with its version or why not.
pub(crate) fn available() -> Result<String, String> {
    let mut message = String::new();
    if ffi::rvp_bluray_load(&mut message) {
        Ok(message)
    } else {
        Err(message)
    }
}
