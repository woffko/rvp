//! What a skin may ask of RVP: one table of limits.
//!
//! A skin is a file from anywhere. Everything in it that costs memory,
//! stack or time has its limit here, and each limit is checked where the
//! number enters, before anything of that size is allocated or walked: a
//! skin over a limit does not load ([`SkinError::Limit`] says which), and
//! a number that only shows while the skin draws (a rectangle's place, a
//! window's shape) is cut down to its limit instead.
//!
//! The values come from measuring, not from guessing: each is several
//! times the largest value among 69 skins (PotPlayer's own 23, and samples
//! of 25 classic Winamp skins and 21 VLC themes; 2026-11-20), and still
//! far below what would trouble a machine. Each constant's comment gives
//! that largest value. Before changing one, measure again:
//!
//! ```text
//! cargo run --release -p rvp-skin --example skin_limits -- FOLDER...
//! ```
//!
//! prints every quantity's largest value among the skins of the folders,
//! the skin it came from and the limit beside it.

use crate::error::SkinError;

// The skin's file and what it unpacks to (`crate::archive`).

/// The bytes of a skin's file. Largest measured: 0.6 MB (BlackSpace).
pub const ARCHIVE_BYTES: u64 = 32 << 20;

/// The files in a skin's archive, or built for another player's skin.
/// Largest measured: 151 (a VLC theme).
pub const ARCHIVE_ENTRIES: usize = 4096;

/// The bytes of one file of the archive, unpacked. Largest measured:
/// 0.8 MB (a VLC theme's picture).
pub const ENTRY_BYTES: u64 = 16 << 20;

/// The bytes of all the archive's files, unpacked; also of the files RVP
/// builds from another player's skin. Largest measured: 2.1 MB (a classic
/// Winamp skin's bitmaps).
pub const UNPACKED_BYTES: u64 = 64 << 20;

// Pictures (`crate::image`).

/// A picture's side, in pixels. Largest measured: 8694 (a VLC theme's
/// strip of 377 x 8694): the side is generous, the pixels are what count.
pub const PICTURE_SIDE: u32 = 32_768;

/// A picture's pixels (four bytes each, decoded). Largest measured:
/// 3.3 million (the same strip).
pub const PICTURE_PIXELS: u64 = 16 << 20;

/// The pixels of all the pictures one skin holds: every picture of a
/// `.dsf`, a classic skin's bitmaps, a VLC theme's bitmaps and their
/// parts, and the pictures RVP builds from them. Largest measured:
/// 9.9 million (PotPlayer's Default).
pub const SKIN_PIXELS: u64 = 64 << 20;

// Markup (`crate::preprocess`, `crate::markup`).

/// The bytes of a skin file's markup after its includes and macros are
/// expanded, and the bytes of the files read for it (each include counts
/// what it reads). Largest measured: 0.16 MB expanded, 0.37 MB read
/// (PotPlayer's Default, whose files are UTF-16).
pub const TEXT_BYTES: usize = 4 << 20;

/// The `#include`s of one skin file. Largest measured: 27.
pub const INCLUDES: usize = 1024;

/// How deep `#include`s nest. A deeper include is left out with a
/// warning, as before there were limits: a file that includes itself ends
/// there. Largest measured: 3.
pub const INCLUDE_DEPTH: usize = 16;

/// The macros one included file is given, its parents' with its own.
/// Largest measured: 2.
pub const MACROS: usize = 64;

/// How deep the markup's elements nest. The tree is walked by functions
/// that call themselves once per level. Largest measured: 9.
pub const MARKUP_DEPTH: usize = 64;

/// The elements of one skin file's markup. Largest measured: 1217
/// (PotPlayer's Default).
pub const MARKUP_ELEMENTS: usize = 16_384;

/// The attributes of one element: each new one is compared with those
/// before it. Largest measured: 66.
pub const ATTRIBUTES: usize = 512;

// Layout and drawing (`crate::layout`, `crate::render`).

/// How far from its window's corner an edge of a rectangle may lie, in
/// pixels; a rectangle beyond is cut there. Largest measured: 1128, with
/// each skin's windows at their own sizes; no screen has 16,384 pixels a
/// side. The bound lies far from anything that shows, and only keeps
/// sums of the skin's numbers exact.
pub const COORDINATE: i32 = 1 << 20;

/// The pixels of a canvas a window is drawn into
/// ([`crate::render::rasterize`], with at most [`PICTURE_SIDE`] a side);
/// a larger window is drawn from its top-left corner as far as they go.
/// A screen of 7680 x 4320 has 33 million.
pub const CANVAS_PIXELS: u64 = 64 << 20;

/// The rectangles of a window's shape ([`crate::render::opaque_region`]);
/// a window whose shape needs more keeps a rectangle's. Largest measured:
/// 598 (a VLC theme's main window).
pub const REGION_RECTS: usize = 8192;

// A skin's windows of its own (`RvpWindow`, built from other players'
// skins).

/// The windows a skin has besides the main one. Largest measured: 5 (a
/// VLC theme).
pub const EXTRA_WINDOWS: usize = 24;

/// The side of such a window at first, in pixels. Largest measured: 810.
pub const WINDOW_SIDE: i32 = 4096;

/// The pixels of such a window at first. Largest measured: 0.28 million.
pub const WINDOW_PIXELS: u64 = 4 << 20;

// VLC's themes (`crate::vlc`).

/// The tokens of one expression of a theme (`visible`, a checkbox's
/// `state`): expressions are read and computed by functions that call
/// themselves. Largest measured: 15.
pub const EXPRESSION_TOKENS: usize = 128;

/// The pictures of a slider's background (`nbhoriz` times `nbvert`).
/// Largest measured: 590.
pub const SLIDER_CELLS: i32 = 4096;

// RVP's logo on a skin's title bars (`Skin::logo_slots`, `rvp-look`).

/// The side of a title-bar button that takes RVP's logo, in pixels: RVP
/// draws its logo at the button's size, four times side by side. A larger
/// button keeps its own picture. Largest measured: 30.
pub const LOGO_SIDE: i32 = 512;

/// The buttons that take RVP's logo; the rest keep their own pictures.
/// Largest measured: 2.
pub const LOGO_SLOTS: usize = 16;

/// The error of a skin that has `what` where at most `limit` may be.
pub(crate) fn exceeded(what: &str, limit: impl std::fmt::Display) -> SkinError {
    SkinError::Limit(format!("{what} (at most {limit})"))
}

/// Whether a picture of this size may be decoded or built.
#[must_use]
pub fn picture_fits(width: u32, height: u32) -> bool {
    width <= PICTURE_SIDE
        && height <= PICTURE_SIDE
        && u64::from(width) * u64::from(height) <= PICTURE_PIXELS
}

/// Checks one picture's size.
///
/// # Errors
/// Fails when a side or the pixel count is over its limit.
pub fn picture(name: &str, width: u32, height: u32) -> Result<(), SkinError> {
    if picture_fits(width, height) {
        return Ok(());
    }
    Err(exceeded(
        &format!("picture {name} of {width}x{height} pixels"),
        format_args!("{PICTURE_SIDE} a side and {PICTURE_PIXELS} pixels"),
    ))
}

/// Whether a skin may give a window of its own this size at first.
#[must_use]
pub fn window_fits(width: i32, height: i32) -> bool {
    width <= WINDOW_SIDE
        && height <= WINDOW_SIDE
        && u64::from(width.max(0).unsigned_abs()) * u64::from(height.max(0).unsigned_abs())
            <= WINDOW_PIXELS
}

/// The pixels the pictures of one skin may still take of [`SKIN_PIXELS`].
#[derive(Debug, Clone)]
pub struct Pixels {
    left: u64,
}

impl Default for Pixels {
    fn default() -> Self {
        Self { left: SKIN_PIXELS }
    }
}

impl Pixels {
    /// A budget with only `pixels` left, for the loaders' tests.
    #[cfg(test)]
    pub(crate) fn with(pixels: u64) -> Self {
        Self {
            left: pixels.min(SKIN_PIXELS),
        }
    }

    /// Takes a picture of `width` x `height` pixels, before it is made.
    ///
    /// # Errors
    /// Fails when the picture is over its own limits, or the skin's
    /// pictures together would be over theirs.
    pub fn take(&mut self, name: &str, width: u32, height: u32) -> Result<(), SkinError> {
        picture(name, width, height)?;
        let pixels = u64::from(width) * u64::from(height);
        match self.left.checked_sub(pixels) {
            Some(left) => {
                self.left = left;
                Ok(())
            }
            None => Err(exceeded(
                &format!("pictures of more pixels together, at {name}"),
                SKIN_PIXELS,
            )),
        }
    }

    /// Gives back a picture that was not kept.
    pub fn give_back(&mut self, width: u32, height: u32) {
        self.left = self
            .left
            .saturating_add(u64::from(width) * u64::from(height))
            .min(SKIN_PIXELS);
    }

    /// The pixels taken so far.
    #[must_use]
    pub fn taken(&self) -> u64 {
        SKIN_PIXELS - self.left
    }
}

/// Runs `run` on a thread with a quarter of the smallest stack RVP's
/// threads have (Windows' 1 MiB): what must not recurse without end is
/// tried where a deep recursion ends the process at once.
#[cfg(test)]
pub(crate) fn on_small_stack<T: Send + 'static>(run: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 10)
        .spawn(run)
        .expect("a thread")
        .join()
        .expect("the thread ends without a panic")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_bounded_by_its_sides_and_its_pixels() {
        assert!(picture_fits(377, 8694));
        assert!(picture_fits(4096, 4096));
        assert!(!picture_fits(PICTURE_SIDE + 1, 1));
        assert!(!picture_fits(1, PICTURE_SIDE + 1));
        assert!(!picture_fits(4097, 4096));
        // The sides' product does not wrap.
        assert!(!picture_fits(u32::MAX, u32::MAX));
        let error = picture("big.png", 5000, 5000).unwrap_err();
        assert!(matches!(error, SkinError::Limit(_)), "{error}");
        assert!(error.to_string().contains("big.png"), "{error}");
    }

    #[test]
    fn a_skins_pictures_share_one_budget() {
        let mut budget = Pixels::default();
        // Four pictures of the largest size fill it.
        for _ in 0..4 {
            budget.take("full.png", 4096, 4096).unwrap();
        }
        assert_eq!(budget.taken(), SKIN_PIXELS);
        let error = budget.take("more.png", 1, 1).unwrap_err();
        assert!(matches!(error, SkinError::Limit(_)), "{error}");
        // A picture that did not decode gives its pixels back.
        budget.give_back(4096, 4096);
        budget.take("again.png", 4096, 4096).unwrap();
        // One picture over its own limit is refused whatever is left.
        let mut budget = Pixels::default();
        assert!(budget.take("wide.png", PICTURE_SIDE + 1, 1).is_err());
        assert_eq!(budget.taken(), 0);
    }

    #[test]
    fn a_skins_own_window_is_bounded() {
        assert!(window_fits(810, 348));
        assert!(window_fits(2048, 2048));
        assert!(!window_fits(WINDOW_SIDE + 1, 1));
        assert!(!window_fits(4096, 4096));
        // A size below zero is no window at all, not a large one.
        assert!(window_fits(-5, 100));
    }
}
