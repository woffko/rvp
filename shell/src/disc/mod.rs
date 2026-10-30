//! DVD menus (M16, `docs/spikes/s7-disc-menus.md`): RVP runs the disc's
//! navigation with libdvdnav (`cpp/disc.cpp`) and hands mpv the program
//! stream one segment at a time, as `rvpdisc://N` through libmpv's custom
//! streams (`stream`).
//!
//! * A segment ends at a still (EOF: mpv's decoder drains and, with
//!   keep-open, holds the frame), at a jump of the disc's VM (a new title
//!   set, a menu, a button's command), where its cells go back (a motion
//!   menu that loops, the next program chain: its timestamps start again,
//!   which one mpv file cannot take) and at the disc's end. When mpv
//!   reaches the end ([`Disc::finish`] says why) the window waits for the
//!   user, waits out a timed still, or loads the next segment.
//! * A segment that starts in a title is seekable: byte offsets map to the
//!   title's sectors, so mpv seeks, finds the length and shows the time
//!   as for a file. Its data ends at the title's last sector, and the
//!   navigator reads on past it only when mpv reached the end: mpv's probe
//!   reads up to the end and seeks back, and libdvdnav refuses to seek once
//!   it reached a still or followed a jump.
//! * RVP draws the menus' sub-pictures itself (`rvp_app::disc`): the
//!   disc's colours, and the selected button's own inside its rectangle;
//!   mpv shows no sub-picture track in a menu.

pub(crate) mod bluray;
mod stream;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use rvp_app::disc::overlay::{self, Clut, Highlight};
use rvp_app::disc::spu::{Assembler, BLOCK, SubPicture, decode};

use crate::disc_ffi::bridge::{self as ffi, DvdButton, DvdNavigator, DvdStatus};

pub(crate) use stream::register;

/// The protocol of the segments.
pub(crate) const PROTOCOL: &str = "rvpdisc";

/// libdvdnav's events (`dvdnav_events.h`).
mod event {
    pub const BLOCK_OK: i32 = 0;
    pub const NOP: i32 = 1;
    pub const STILL_FRAME: i32 = 2;
    pub const SPU_STREAM_CHANGE: i32 = 3;
    pub const AUDIO_STREAM_CHANGE: i32 = 4;
    pub const VTS_CHANGE: i32 = 5;
    pub const CELL_CHANGE: i32 = 6;
    pub const NAV_PACKET: i32 = 7;
    pub const STOP: i32 = 8;
    pub const HIGHLIGHT: i32 = 9;
    pub const SPU_CLUT_CHANGE: i32 = 10;
    pub const HOP_CHANNEL: i32 = 12;
    pub const WAIT: i32 = 13;
}

/// libdvdnav's `DVDMenuID_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Menu {
    Title = 2,
    Root = 3,
    Subpicture = 4,
    Audio = 5,
    Angle = 6,
    Part = 7,
}

/// Why a segment ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    /// A still of this many seconds (255: until the user acts).
    Still(u8),
    /// The VM jumped: the next segment follows.
    Jump,
    /// The next segment is prepared ([`Disc::prime`] ran).
    Next,
    /// The disc's program ended.
    Stop,
    /// libdvdnav failed.
    Error,
}

/// What the window hears from the navigator (on mpv's demuxer thread).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Notice {
    /// The menu's picture, the selected button or the colours changed.
    Highlight,
    /// The VM chose other audio or sub-picture streams.
    Streams,
    /// Another title or chapter.
    Position,
}

/// The sink of [`Notice`]s; the shell queues them to the window's thread.
pub(crate) type Notify = Box<dyn Fn(Notice) + Send + Sync>;

/// What the segments' stream (`stream`) reads: the open disc's data, one
/// segment at a time.
pub(crate) trait Segments: Send + Sync {
    /// mpv opens segment `segment` (it opens a file after `loadfile`
    /// returned, so another may be prepared by then): whether it seeks.
    fn bind(&self, segment: u64) -> bool;
    /// Up to `out.len()` bytes of segment `segment`; 0 at its end.
    fn read(&self, segment: u64, out: &mut [u8]) -> usize;
    /// Moves to byte `offset` of a seekable segment.
    fn seek(&self, segment: u64, offset: u64) -> Option<u64>;
    /// A seekable segment's length in bytes.
    fn size(&self, segment: u64) -> Option<u64>;
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The navigator and what one segment has handed over.
struct Navigator {
    dvd: cxx::UniquePtr<DvdNavigator>,
    block: Vec<u8>,
    /// A block not yet handed over, from `taken` on.
    pending: Vec<u8>,
    taken: usize,
    /// This segment handed data over (a jump then ends it).
    delivered: bool,
    ended: Option<End>,
    clut: Clut,
    assembler: Assembler,
    /// The menu's sub-picture, once one came.
    picture: Option<SubPicture>,
    /// Where a seekable segment starts in its title (sectors), and its
    /// length from there.
    start: Option<(u32, u32)>,
    /// A seekable segment handed over its last sector.
    data_end: bool,
    /// The cell this segment plays (libdvdnav's `cellN`).
    cell: i32,
    /// The segment the window prepared last (its address's number); a
    /// stream of an older one reads nothing more.
    segment: u64,
}

impl Navigator {
    fn dvd(&mut self) -> std::pin::Pin<&mut DvdNavigator> {
        self.dvd.pin_mut()
    }

    fn status(&mut self) -> DvdStatus {
        ffi::rvp_dvdnav_status(self.dvd())
    }

    fn in_menu(&mut self) -> bool {
        matches!(self.status().domain, 4 | 8)
    }

    /// A new segment: nothing handed over yet.
    fn begin(&mut self) {
        self.pending.clear();
        self.taken = 0;
        self.delivered = false;
        self.ended = None;
        self.start = None;
        self.data_end = false;
        self.cell = 0;
    }

    /// Reads to the next data block (kept in `pending`), handling the
    /// events on the way; false when the segment ends first.
    fn advance(&mut self, notify: &Notify) -> bool {
        loop {
            if self.ended.is_some() {
                return false;
            }
            let mut length = 0;
            let code = {
                let Navigator { dvd, block, .. } = self;
                ffi::rvp_dvdnav_next(dvd.pin_mut(), block.as_mut_slice(), &mut length)
            };
            let word = |at: usize| {
                i32::from_le_bytes([
                    self.block[at],
                    self.block[at + 1],
                    self.block[at + 2],
                    self.block[at + 3],
                ])
            };
            match code {
                event::BLOCK_OK => {
                    let size = usize::try_from(length).unwrap_or(0).min(BLOCK);
                    let status = self.status();
                    if matches!(status.domain, 4 | 8) {
                        for unit in self.assembler.push(&self.block[..size]) {
                            if let Ok(picture) = decode(&unit) {
                                self.picture = Some(picture);
                                notify(Notice::Highlight);
                            }
                        }
                    }
                    self.pending.clear();
                    self.pending.extend_from_slice(&self.block[..size]);
                    self.taken = 0;
                    self.delivered = true;
                    // The position is the handed block's sector.
                    self.data_end =
                        self.start.is_some() && status.position.saturating_add(1) >= status.length;
                    return true;
                }
                event::STILL_FRAME => {
                    let seconds = u8::try_from(word(0).clamp(0, 255)).unwrap_or(255);
                    self.ended = Some(End::Still(seconds));
                }
                event::SPU_STREAM_CHANGE => {
                    let stream = self.status().spu;
                    self.assembler
                        .set_stream(u8::try_from(stream & 0x1f).unwrap_or(0));
                    notify(Notice::Streams);
                }
                event::AUDIO_STREAM_CHANGE => notify(Notice::Streams),
                event::VTS_CHANGE | event::HOP_CHANNEL => {
                    // A new title set or a jump: a segment that showed
                    // something ends here; a new one goes on.
                    if self.delivered {
                        self.ended = Some(End::Jump);
                    }
                    self.picture = None;
                    notify(Notice::Highlight);
                }
                event::CELL_CHANGE => {
                    // `dvdnav_cell_change_event_t` starts with `cellN`.
                    let cell = word(0);
                    if self.delivered && cell <= self.cell {
                        self.ended = Some(End::Jump);
                    }
                    self.cell = cell;
                    notify(Notice::Position);
                }
                event::STOP => self.ended = Some(End::Stop),
                event::HIGHLIGHT => notify(Notice::Highlight),
                event::SPU_CLUT_CHANGE => {
                    for (entry, bytes) in self.clut.iter_mut().zip(self.block.as_chunks::<4>().0) {
                        *entry = u32::from_le_bytes(*bytes);
                    }
                    notify(Notice::Highlight);
                }
                event::WAIT => {
                    ffi::rvp_dvdnav_wait_skip(self.dvd());
                }
                event::NOP | event::NAV_PACKET => {}
                -1 => self.ended = Some(End::Error),
                _ => {}
            }
        }
    }
}

struct Shared {
    nav: Mutex<Navigator>,
    notify: Notify,
}

impl Shared {
    /// The window prepares the next segment: it reads to the segment's
    /// first data (false when a still or the end comes first), and a
    /// segment that starts in a title becomes seekable.
    fn prime(&self) -> bool {
        let mut nav = lock(&self.nav);
        nav.segment += 1;
        nav.begin();
        if !nav.advance(&self.notify) {
            return false;
        }
        let status = nav.status();
        nav.start = (status.domain == 2 && status.length > status.position)
            .then_some((status.position.saturating_sub(1), status.length));
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

    /// A seekable segment's byte offset: its sector from the segment's
    /// start, then the bytes into it.
    fn seek(&self, segment: u64, offset: u64) -> Option<u64> {
        let mut nav = lock(&self.nav);
        if nav.segment != segment {
            return None;
        }
        let (start, _) = nav.start?;
        let sector = i64::from(start) + i64::try_from(offset / BLOCK as u64).ok()?;
        if !ffi::rvp_dvdnav_sector_search(nav.dvd(), sector) {
            return None;
        }
        nav.pending.clear();
        nav.taken = 0;
        nav.ended = None;
        nav.data_end = false;
        // The search's own jump (HOP_CHANNEL) does not end the segment.
        nav.delivered = false;
        if !nav.advance(&self.notify) {
            return Some(offset);
        }
        nav.taken = usize::try_from(offset % BLOCK as u64)
            .unwrap_or(0)
            .min(nav.pending.len());
        Some(offset)
    }

    fn size(&self, segment: u64) -> Option<u64> {
        let nav = lock(&self.nav);
        if nav.segment != segment {
            return None;
        }
        let (start, length) = nav.start?;
        Some(u64::from(length.saturating_sub(start)) * BLOCK as u64)
    }
}

/// An open disc: its navigator, shared with the segments mpv reads.
pub(crate) struct Disc {
    shared: Arc<Shared>,
}

impl Disc {
    /// Opens a disc, a `VIDEO_TS` folder or an image for its menus, with
    /// the languages its machine prefers for the menus, the audio and the
    /// sub-pictures (two letters, or empty).
    ///
    /// # Errors
    /// Fails without libdvdnav or when it cannot open `path`.
    pub(crate) fn open(
        path: &str,
        languages: &[String; 3],
        notify: Notify,
    ) -> Result<Self, String> {
        let mut error = String::new();
        let [menu, audio, spu] = languages;
        let dvd = ffi::rvp_dvdnav_open(path, menu, audio, spu, &mut error);
        if dvd.is_null() {
            return Err(error);
        }
        let shared = Arc::new(Shared {
            nav: Mutex::new(Navigator {
                dvd,
                block: vec![0; BLOCK],
                pending: Vec::with_capacity(BLOCK),
                taken: 0,
                delivered: false,
                ended: None,
                clut: [0; 16],
                assembler: Assembler::new(0),
                picture: None,
                start: None,
                data_end: false,
                cell: 0,
                segment: 0,
            }),
            notify,
        });
        stream::set_current(Some(Arc::clone(&shared) as Arc<dyn Segments>));
        Ok(Self { shared })
    }

    /// Prepares the next segment: false when a still or the disc's end
    /// comes before any picture ([`Disc::ended`] says which).
    pub(crate) fn prime(&self) -> bool {
        self.shared.prime()
    }

    /// The prepared segment's address for mpv (a new one for each).
    pub(crate) fn segment_address(&self) -> String {
        format!("{PROTOCOL}://{}", lock(&self.shared.nav).segment)
    }

    pub(crate) fn status(&self) -> DvdStatus {
        lock(&self.shared.nav).status()
    }

    /// A menu shows (the video manager's or a title set's).
    pub(crate) fn in_menu(&self) -> bool {
        lock(&self.shared.nav).in_menu()
    }

    /// The selected button (mode 0) or the activated one (1).
    pub(crate) fn button(&self, mode: i32) -> DvdButton {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_button(nav.dvd(), mode)
    }

    /// Moves the selection: 0 up, 1 down, 2 left, 3 right.
    pub(crate) fn select(&self, direction: i32) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_select(nav.dvd(), direction)
    }

    pub(crate) fn activate(&self) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_activate(nav.dvd())
    }

    /// The pointer over the frame (frame pixels); `activate` clicks.
    pub(crate) fn mouse(&self, x: i32, y: i32, activate: bool) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_mouse(nav.dvd(), x, y, activate)
    }

    pub(crate) fn menu(&self, menu: Menu) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_menu(nav.dvd(), menu as i32)
    }

    pub(crate) fn go_up(&self) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_go_up(nav.dvd())
    }

    /// Ends a timed still (or a still the user waits out).
    pub(crate) fn still_skip(&self) -> bool {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_still_skip(nav.dvd())
    }

    /// libdvdnav's last error.
    pub(crate) fn error(&self) -> String {
        let mut nav = lock(&self.shared.nav);
        ffi::rvp_dvdnav_error(nav.dvd())
    }

    /// Why the navigator ended the segment (or stopped before one).
    pub(crate) fn ended(&self) -> Option<End> {
        lock(&self.shared.nav).ended
    }

    /// mpv reached the segment's end: why it ended. After a title's last
    /// sector the navigator reads on now, to a still, the disc's end or the
    /// next segment's data ([`End::Next`]). `None` while the segment has
    /// data left (mpv's end came from elsewhere).
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

    /// The menu's sub-picture as premultiplied BGRA (its rectangle in frame
    /// pixels, then the pixels), with the selected button lit.
    pub(crate) fn menu_picture(&self) -> Option<(u16, u16, u16, u16, Vec<u8>)> {
        let mut nav = lock(&self.shared.nav);
        if !nav.in_menu() {
            return None;
        }
        let button = ffi::rvp_dvdnav_button(nav.dvd(), 0);
        let picture = nav.picture.as_ref()?;
        let highlight = button.valid.then_some(Highlight {
            x0: button.x0,
            y0: button.y0,
            x1: button.x1,
            y1: button.y1,
            palette: button.palette,
        });
        let pixels = overlay::compose(picture, &nav.clut, highlight.as_ref());
        Some((picture.x, picture.y, picture.width, picture.height, pixels))
    }
}

impl Drop for Disc {
    fn drop(&mut self) {
        stream::clear_current(&(Arc::clone(&self.shared) as Arc<dyn Segments>));
    }
}

/// A disc that plays with its menus.
pub(crate) enum Navigation {
    Dvd(Disc),
    Bluray(bluray::BluRay),
}

/// A key of a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
}

/// What a key, the pointer or a menu call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Response {
    /// Nothing: not the menu's.
    Ignored,
    /// The menu's picture changed.
    Redraw,
    /// The disc's program moved on: the next segment follows.
    Advance,
}

/// The menu's picture: its place in frame pixels (a Blu-ray's plane is its
/// frame), premultiplied BGRA, and the frame's size where the disc knows
/// it.
pub(crate) struct Picture {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub frame: Option<(usize, usize)>,
}

/// The streams the disc's program chose, as mpv's source IDs.
pub(crate) struct Streams {
    /// A menu shows: no subtitles (RVP draws the menu).
    pub menu: bool,
    /// The audio stream's source ID, by the stream types it may have.
    pub audio: Vec<i64>,
    pub subtitle: Option<i64>,
}

/// DVD audio in mpv's program stream (lavf's `mpegps`): AC-3, DTS, LPCM
/// and MPEG audio; sub-pictures from 0x20.
const DVD_AUDIO: [i64; 4] = [0x80, 0x88, 0xa0, 0x1c0];
const DVD_SPU: i64 = 0x20;
/// Blu-ray's primary audio and subtitle (PG) PIDs, by stream number.
const BD_AUDIO: i64 = 0x1100;
const BD_SUBTITLE: i64 = 0x1200;

impl Navigation {
    pub(crate) fn prime(&self) -> bool {
        match self {
            Self::Dvd(disc) => disc.prime(),
            Self::Bluray(disc) => disc.prime(),
        }
    }

    pub(crate) fn segment_address(&self) -> String {
        match self {
            Self::Dvd(disc) => disc.segment_address(),
            Self::Bluray(disc) => disc.segment_address(),
        }
    }

    pub(crate) fn ended(&self) -> Option<End> {
        match self {
            Self::Dvd(disc) => disc.ended(),
            Self::Bluray(disc) => disc.ended(),
        }
    }

    pub(crate) fn finish(&self) -> Option<End> {
        match self {
            Self::Dvd(disc) => disc.finish(),
            Self::Bluray(disc) => disc.finish(),
        }
    }

    pub(crate) fn still_skip(&self) -> bool {
        match self {
            Self::Dvd(disc) => disc.still_skip(),
            Self::Bluray(disc) => disc.still_skip(),
        }
    }

    /// Straight to the disc's menu (after the first play started): true
    /// when the program jumped.
    pub(crate) fn skip_opening(&self) -> bool {
        match self {
            Self::Dvd(disc) => disc.menu(Menu::Title) || disc.menu(Menu::Root),
            Self::Bluray(disc) => disc.top_menu(),
        }
    }

    /// A menu with buttons shows: the arrows, Enter and the pointer are
    /// its.
    pub(crate) fn menu_keys(&self) -> bool {
        match self {
            Self::Dvd(disc) => disc.in_menu() && disc.button(0).valid,
            Self::Bluray(disc) => disc.menu_shown(),
        }
    }

    pub(crate) fn key(&self, key: MenuKey) -> Response {
        match self {
            Self::Dvd(disc) => {
                let direction = match key {
                    MenuKey::Up => 0,
                    MenuKey::Down => 1,
                    MenuKey::Left => 2,
                    MenuKey::Right => 3,
                    MenuKey::Enter => {
                        return if disc.activate() {
                            Response::Advance
                        } else {
                            Response::Ignored
                        };
                    }
                };
                if disc.select(direction) {
                    Response::Redraw
                } else {
                    Response::Ignored
                }
            }
            Self::Bluray(disc) => {
                let (code, response) = match key {
                    MenuKey::Up => (bluray::Key::Up, Response::Redraw),
                    MenuKey::Down => (bluray::Key::Down, Response::Redraw),
                    MenuKey::Left => (bluray::Key::Left, Response::Redraw),
                    MenuKey::Right => (bluray::Key::Right, Response::Redraw),
                    MenuKey::Enter => (bluray::Key::Enter, Response::Advance),
                };
                if disc.key(code) {
                    response
                } else {
                    Response::Ignored
                }
            }
        }
    }

    /// The pointer over the menu (frame pixels); `click` activates the
    /// button under it.
    pub(crate) fn mouse(&self, x: usize, y: usize, click: bool) -> Response {
        let (Ok(x16), Ok(y16)) = (u16::try_from(x), u16::try_from(y)) else {
            return Response::Ignored;
        };
        match self {
            Self::Dvd(disc) => {
                let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
                    return Response::Ignored;
                };
                if click {
                    return if disc.mouse(x, y, true) {
                        Response::Advance
                    } else {
                        Response::Ignored
                    };
                }
                let before = disc.button(0).button;
                if disc.mouse(x, y, false) && disc.button(0).button != before {
                    Response::Redraw
                } else {
                    Response::Ignored
                }
            }
            Self::Bluray(disc) => match (disc.mouse(x16, y16, click), click) {
                (true, true) => Response::Advance,
                (true, false) => Response::Redraw,
                (false, _) => Response::Ignored,
            },
        }
    }

    /// One of DVD Control's menus. A Blu-ray has a top menu (Title and
    /// Root Menu) and a pop-up menu (the others); no previous menu.
    pub(crate) fn menu(&self, menu: rvp_app::disc::play::DiscMenu) -> Response {
        use rvp_app::disc::play::DiscMenu;
        match self {
            Self::Dvd(disc) => {
                let done = match menu {
                    DiscMenu::Previous => disc.go_up(),
                    DiscMenu::Root => disc.menu(Menu::Root),
                    DiscMenu::Title => disc.menu(Menu::Title),
                    DiscMenu::Chapter => disc.menu(Menu::Part),
                    DiscMenu::Audio => disc.menu(Menu::Audio),
                    DiscMenu::Subtitle => disc.menu(Menu::Subpicture),
                    DiscMenu::Angle => disc.menu(Menu::Angle),
                };
                if done {
                    Response::Advance
                } else {
                    Response::Ignored
                }
            }
            Self::Bluray(disc) => match menu {
                DiscMenu::Title | DiscMenu::Root if disc.top_menu() => Response::Advance,
                DiscMenu::Chapter | DiscMenu::Audio | DiscMenu::Subtitle | DiscMenu::Angle
                    if disc.key(bluray::Key::Popup) =>
                {
                    Response::Redraw
                }
                _ => Response::Ignored,
            },
        }
    }

    pub(crate) fn picture(&self) -> Option<Picture> {
        match self {
            Self::Dvd(disc) => {
                let (x, y, width, height, pixels) = disc.menu_picture()?;
                Some(Picture {
                    x: usize::from(x),
                    y: usize::from(y),
                    width: usize::from(width),
                    height: usize::from(height),
                    pixels,
                    frame: None,
                })
            }
            Self::Bluray(disc) => disc.picture(),
        }
    }

    /// While the disc waits: true when its program moved on by itself (a
    /// Blu-ray's timers, BD-J).
    pub(crate) fn poll(&self) -> bool {
        match self {
            Self::Dvd(_) => false,
            Self::Bluray(disc) => disc.poll(),
        }
    }

    pub(crate) fn streams(&self) -> Streams {
        match self {
            Self::Dvd(disc) => {
                let status = disc.status();
                // A negative sub-picture stream: none, or one shown only
                // by its forced pictures.
                Streams {
                    menu: matches!(status.domain, 4 | 8),
                    audio: if status.audio >= 0 {
                        DVD_AUDIO
                            .iter()
                            .map(|base| base + i64::from(status.audio))
                            .collect()
                    } else {
                        Vec::new()
                    },
                    subtitle: (status.spu >= 0).then(|| DVD_SPU + i64::from(status.spu)),
                }
            }
            Self::Bluray(disc) => {
                let status = disc.status();
                Streams {
                    menu: status.menu,
                    audio: if (1..=32).contains(&status.audio) {
                        vec![BD_AUDIO + i64::from(status.audio) - 1]
                    } else {
                        Vec::new()
                    },
                    subtitle: (status.subtitle_shown && (1..=255).contains(&status.subtitle))
                        .then(|| BD_SUBTITLE + i64::from(status.subtitle) - 1),
                }
            }
        }
    }

    /// The disc's state for the self-test.
    pub(crate) fn status_json(&self) -> serde_json::Value {
        match self {
            Self::Dvd(disc) => {
                let status = disc.status();
                let button = disc.button(0);
                serde_json::json!({
                    "kind": "dvd",
                    "domain": status.domain,
                    "title": status.title,
                    "part": status.part,
                    "button": button.valid.then_some(button.button),
                })
            }
            Self::Bluray(disc) => {
                let status = disc.status();
                serde_json::json!({
                    "kind": "bluray",
                    "title": status.title,
                    "part": status.chapter,
                    "menu": status.menu,
                    "popup": status.popup,
                })
            }
        }
    }

    /// Why the navigation failed.
    pub(crate) fn error(&self) -> String {
        match self {
            Self::Dvd(disc) => disc.error(),
            Self::Bluray(disc) => disc.error().unwrap_or("libbluray stopped").to_owned(),
        }
    }
}

/// Whether libdvdnav loads here, with its version or why not.
pub(crate) fn available() -> Result<String, String> {
    let mut message = String::new();
    if ffi::rvp_dvdnav_load(&mut message) {
        Ok(message)
    } else {
        Err(message)
    }
}

#[cfg(test)]
mod tests;
