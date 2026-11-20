//! A Blu-ray menu's graphics (M16): the interactive graphics plane that
//! libbluray draws with its overlay events (`overlay.h`), kept as
//! premultiplied BGRA for mpv's `overlay-add`. HDMV menus come as
//! palette-coded runs (Y, Cr, Cb and transparency, BT.709 on HD planes,
//! BT.601 on SD ones), BD-J menus as ARGB. RVP shows the interactive
//! plane only: mpv draws the subtitles (PG) of the stream itself.
//!
//! The sizes and places come from the disc unchecked (libbluray passes
//! its 16-bit fields on): a plane larger than [`MAX_WIDTH`] x
//! [`MAX_HEIGHT`] stays closed, and a draw or a wipe touches only the
//! part of its rectangle inside the plane. Runs are read row by row (a
//! row ends at an end-of-line run, or once it holds the image's width,
//! with an end-of-line run right after it) for the image's height at
//! most, straight into the plane: the work follows the runs given and
//! the plane, never the sizes the disc claims.

/// libbluray's planes (`bd_overlay_plane_e`).
pub const PLANE_IG: u8 = 1;

/// The largest plane RVP opens (`cpp/bluray.cpp` keeps the same): UHD
/// Blu-ray's video, 3840 x 2160, fits with room.
pub const MAX_WIDTH: usize = 4096;
pub const MAX_HEIGHT: usize = 2304;

/// libbluray's overlay commands (`bd_overlay_cmd_e`).
pub mod command {
    pub const INIT: u8 = 0;
    pub const CLOSE: u8 = 1;
    pub const CLEAR: u8 = 2;
    pub const DRAW: u8 = 3;
    pub const WIPE: u8 = 4;
    pub const HIDE: u8 = 5;
    pub const FLUSH: u8 = 6;
}

/// One overlay event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overlay {
    pub plane: u8,
    pub command: u8,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub image: Image,
}

/// What a draw puts on the plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Image {
    None,
    /// Runs of (length, palette index), a run of 0 ending a line, and the
    /// palette's 256 entries as (Y, Cr, Cb, T).
    Runs {
        runs: Vec<(u16, u8)>,
        palette: Vec<[u8; 4]>,
    },
    /// BD-J's ARGB pixels, `width` x `height`.
    Argb(Vec<u32>),
}

/// The interactive graphics plane.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plane {
    width: usize,
    height: usize,
    /// Premultiplied BGRA, `width` x `height`.
    pixels: Vec<u8>,
    hidden: bool,
}

impl Plane {
    /// The plane's size (0 x 0 until libbluray opens it).
    #[must_use]
    pub fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Applies one event of the interactive plane; true at a flush or a
    /// close, when the plane shows as it is now.
    pub fn apply(&mut self, overlay: &Overlay) -> bool {
        if overlay.plane != PLANE_IG {
            return false;
        }
        match overlay.command {
            command::INIT => {
                let (width, height) = (usize::from(overlay.width), usize::from(overlay.height));
                if width > MAX_WIDTH || height > MAX_HEIGHT {
                    // No disc's video is that large: the plane stays
                    // closed, and what showed goes.
                    *self = Self::default();
                    return true;
                }
                self.width = width;
                self.height = height;
                self.pixels = vec![0; width * height * 4];
                self.hidden = false;
                false
            }
            command::CLOSE => {
                *self = Self::default();
                true
            }
            command::CLEAR => {
                self.pixels.fill(0);
                false
            }
            command::WIPE => {
                self.fill(overlay, |_, _| [0; 4]);
                false
            }
            command::DRAW => {
                self.hidden = false;
                self.draw(overlay);
                false
            }
            command::HIDE => {
                self.hidden = true;
                true
            }
            command::FLUSH => true,
            _ => false,
        }
    }

    fn draw(&mut self, overlay: &Overlay) {
        match &overlay.image {
            Image::None => {}
            Image::Runs { runs, palette } => {
                // The rectangle is the image's: what no run covers is
                // transparent.
                self.fill(overlay, |_, _| [0; 4]);
                let hd = self.height > 576;
                let colours: Vec<[u8; 4]> = palette.iter().map(|e| bgra(*e, hd)).collect();
                self.draw_runs(overlay, runs, &colours);
            }
            Image::Argb(pixels) => {
                let width = usize::from(overlay.width);
                self.fill(overlay, |x, y| {
                    pixels
                        .get(y * width + x)
                        .map_or([0; 4], |&argb| premultiplied(argb))
                });
            }
        }
    }

    /// Writes the runs of (length, palette index) into the overlay's
    /// rectangle: row by row from its corner, a row ending at a run of 0
    /// or once it holds the overlay's width (an end-of-line run right
    /// after it belongs to it), until the overlay's or the plane's last
    /// row. Runs and parts of runs outside the plane cost a sum each.
    fn draw_runs(&mut self, overlay: &Overlay, runs: &[(u16, u8)], colours: &[[u8; 4]]) {
        let (left, top) = (usize::from(overlay.x), usize::from(overlay.y));
        let width = usize::from(overlay.width);
        let rows = usize::from(overlay.height).min(self.height.saturating_sub(top));
        let mut runs = runs.iter().peekable();
        let (mut x, mut y) = (0, 0);
        while y < rows {
            let Some(&(length, index)) = runs.next() else {
                break;
            };
            let length = usize::from(length);
            if length == 0 {
                (x, y) = (0, y + 1);
                continue;
            }
            // The run's part inside the overlay's width and the plane.
            let from = (left + x).min(self.width);
            let to = (left + (x + length).min(width)).min(self.width);
            if from < to {
                let colour = colours.get(usize::from(index)).copied().unwrap_or([0; 4]);
                let row = (top + y) * self.width;
                let pixels = &mut self.pixels[(row + from) * 4..(row + to) * 4];
                pixels.as_chunks_mut::<4>().0.fill(colour);
            }
            x += length;
            if x >= width {
                runs.next_if(|&&(length, _)| length == 0);
                (x, y) = (0, y + 1);
            }
        }
    }

    /// Sets each pixel of the overlay's rectangle (inside the plane) to
    /// `pixel(x, y)`, with x and y from the rectangle's corner.
    fn fill(&mut self, overlay: &Overlay, pixel: impl Fn(usize, usize) -> [u8; 4]) {
        let (left, top) = (usize::from(overlay.x), usize::from(overlay.y));
        let (x0, y0) = (left.min(self.width), top.min(self.height));
        let x1 = (left + usize::from(overlay.width)).min(self.width);
        let y1 = (top + usize::from(overlay.height)).min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for y in y0..y1 {
            let row = &mut self.pixels[(y * self.width + x0) * 4..(y * self.width + x1) * 4];
            for (x, out) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                *out = pixel(x, y - y0);
            }
        }
    }

    /// What shows: the smallest rectangle around the visible pixels (x, y,
    /// width, height) and its BGRA; `None` when nothing shows.
    #[must_use]
    pub fn picture(&self) -> Option<(usize, usize, usize, usize, Vec<u8>)> {
        if self.hidden || self.width == 0 {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (y, row) in self.pixels.chunks_exact(self.width * 4).enumerate() {
            for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
                if pixel[3] != 0 {
                    x0 = x0.min(x);
                    x1 = x1.max(x + 1);
                    y0 = y0.min(y);
                    y1 = y1.max(y + 1);
                }
            }
        }
        if x1 <= x0 {
            return None;
        }
        let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
        for y in y0..y1 {
            out.extend_from_slice(
                &self.pixels[(y * self.width + x0) * 4..(y * self.width + x1) * 4],
            );
        }
        Some((x0, y0, x1 - x0, y1 - y0, out))
    }
}

/// A palette entry (Y, Cr, Cb, T) in premultiplied BGRA: BT.709 on HD
/// planes, BT.601 on SD ones, limited range.
fn bgra([luma, red_diff, blue_diff, opacity]: [u8; 4], hd: bool) -> [u8; 4] {
    let luma = (f64::from(luma) - 16.0) * 255.0 / 219.0;
    let red_diff = (f64::from(red_diff) - 128.0) * 255.0 / 224.0;
    let blue_diff = (f64::from(blue_diff) - 128.0) * 255.0 / 224.0;
    let (red, green, blue) = if hd {
        (
            luma + 1.5748 * red_diff,
            luma - 0.1873 * blue_diff - 0.4681 * red_diff,
            luma + 1.8556 * blue_diff,
        )
    } else {
        (
            luma + 1.402 * red_diff,
            luma - 0.344_136 * blue_diff - 0.714_136 * red_diff,
            luma + 1.772 * blue_diff,
        )
    };
    let alpha = f64::from(opacity) / 255.0;
    // Rounded and clamped to 0..=255 first.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |value: f64| (value * alpha).round().clamp(0.0, 255.0) as u8;
    [byte(blue), byte(green), byte(red), opacity]
}

/// BD-J's ARGB in premultiplied BGRA.
fn premultiplied(argb: u32) -> [u8; 4] {
    let [a, r, g, b] = argb.to_be_bytes();
    let scale = |value: u8| {
        let product = u16::from(value) * u16::from(a) + 127;
        // At most 255 after the division.
        u8::try_from(product / 255).unwrap_or(u8::MAX)
    };
    [scale(b), scale(g), scale(r), a]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay(command: u8, x: u16, y: u16, width: u16, height: u16, image: Image) -> Overlay {
        Overlay {
            plane: PLANE_IG,
            command,
            x,
            y,
            width,
            height,
            image,
        }
    }

    /// Entry 1: white, opaque (BT.709 limited); 2: grey, half transparent.
    fn palette() -> Vec<[u8; 4]> {
        let mut palette = vec![[16, 128, 128, 0]; 256];
        palette[1] = [235, 128, 128, 255];
        palette[2] = [126, 128, 128, 128];
        palette
    }

    #[test]
    fn runs_draw_into_the_plane_and_the_picture_is_their_box() {
        let mut plane = Plane::default();
        assert!(plane.picture().is_none());
        assert!(!plane.apply(&overlay(command::INIT, 0, 0, 1920, 1080, Image::None)));
        // A 4x2 object at 100,50: a white run of 2, two transparent; then
        // a grey run of 4.
        let runs = vec![(2, 1), (2, 0), (0, 0), (4, 2), (0, 0)];
        plane.apply(&overlay(
            command::DRAW,
            100,
            50,
            4,
            2,
            Image::Runs {
                runs,
                palette: palette(),
            },
        ));
        assert!(plane.apply(&overlay(command::FLUSH, 0, 0, 0, 0, Image::None)));
        let (x, y, width, height, pixels) = plane.picture().expect("a picture");
        assert_eq!((x, y, width, height), (100, 50, 4, 2));
        assert_eq!(&pixels[..4], &[255, 255, 255, 255]);
        assert_eq!(&pixels[8..12], &[0, 0, 0, 0]);
        // Grey 126 is 128 in full range, half covered.
        assert_eq!(&pixels[16..20], &[64, 64, 64, 128]);
        // A wipe clears it; hiding hides all.
        plane.apply(&overlay(command::WIPE, 100, 50, 2, 1, Image::None));
        let (x, y, ..) = plane.picture().expect("the grey line");
        assert_eq!((x, y), (100, 51));
        assert!(plane.apply(&overlay(command::HIDE, 0, 0, 0, 0, Image::None)));
        assert!(plane.picture().is_none());
        assert!(plane.apply(&overlay(command::CLOSE, 0, 0, 0, 0, Image::None)));
        assert_eq!(plane.size(), (0, 0));
    }

    fn runs(runs: Vec<(u16, u8)>) -> Image {
        Image::Runs {
            runs,
            palette: palette(),
        }
    }

    #[test]
    fn draws_and_wipes_outside_the_plane_change_nothing() {
        let mut plane = Plane::default();
        plane.apply(&overlay(command::INIT, 0, 0, 1920, 1080, Image::None));
        let white = || runs(vec![(4, 1), (0, 0), (4, 1), (0, 0)]);
        // Right of the plane, with its rows inside it; then below it.
        plane.apply(&overlay(command::DRAW, 5000, 10, 4, 2, white()));
        plane.apply(&overlay(command::DRAW, 10, 2000, 4, 2, white()));
        plane.apply(&overlay(command::WIPE, 5000, 10, 4, 2, Image::None));
        plane.apply(&overlay(command::WIPE, 10, 2000, 4, 2, Image::None));
        let argb = Image::Argb(vec![0xFFFF_FFFF; 8]);
        plane.apply(&overlay(command::DRAW, 1920, 1080, 4, 2, argb));
        assert!(plane.picture().is_none());
        // Across the corner: only the part inside the plane.
        plane.apply(&overlay(command::DRAW, 1918, 1079, 4, 2, white()));
        let (x, y, width, height, _) = plane.picture().expect("the corner");
        assert_eq!((x, y, width, height), (1918, 1079, 2, 1));
        plane.apply(&overlay(
            command::WIPE,
            1917,
            1078,
            60_000,
            60_000,
            Image::None,
        ));
        assert!(plane.picture().is_none());
    }

    #[test]
    fn a_plane_larger_than_any_video_stays_closed() {
        let mut plane = Plane::default();
        plane.apply(&overlay(command::INIT, 0, 0, 720, 480, Image::None));
        plane.apply(&overlay(command::DRAW, 0, 0, 4, 1, runs(vec![(4, 1)])));
        assert!(plane.picture().is_some());
        // The refused plane takes the old one's picture away.
        assert!(plane.apply(&overlay(command::INIT, 0, 0, 65535, 65535, Image::None)));
        assert_eq!(plane.size(), (0, 0));
        assert!(plane.picture().is_none());
        plane.apply(&overlay(command::CLEAR, 0, 0, 0, 0, Image::None));
        plane.apply(&overlay(command::DRAW, 0, 0, 4, 1, runs(vec![(4, 1)])));
        assert!(plane.picture().is_none());
        assert!(!plane.apply(&overlay(command::INIT, 0, 0, 4096, 2304, Image::None)));
        assert_eq!(plane.size(), (4096, 2304));
        plane.apply(&overlay(command::INIT, 0, 0, 4097, 1080, Image::None));
        assert_eq!(plane.size(), (0, 0));
    }

    #[test]
    fn runs_stop_at_the_images_last_row() {
        let mut plane = Plane::default();
        plane.apply(&overlay(command::INIT, 0, 0, 1920, 1080, Image::None));
        // A million ends of line in an image two rows high and as wide as
        // the disc can say, then a run: past its last row.
        let mut markers = vec![(0, 0); 1_000_000];
        markers.push((4, 1));
        plane.apply(&overlay(command::DRAW, 0, 0, 65535, 2, runs(markers)));
        assert!(plane.picture().is_none());
        // A row longer than the image's width is cut there; the next run
        // starts the next row.
        plane.apply(&overlay(
            command::DRAW,
            10,
            20,
            4,
            2,
            runs(vec![(10, 1), (4, 2), (0, 0), (4, 1)]),
        ));
        let (x, y, width, height, pixels) = plane.picture().expect("two rows");
        assert_eq!((x, y, width, height), (10, 20, 4, 2));
        assert_eq!(&pixels[12..16], &[255, 255, 255, 255]);
        assert_eq!(&pixels[16..20], &[64, 64, 64, 128]);
    }

    #[test]
    fn more_pixels_than_the_plane_holds_cost_only_the_plane() {
        let mut plane = Plane::default();
        plane.apply(&overlay(command::INIT, 0, 0, 1920, 1080, Image::None));
        // An image the disc says is 65535 x 65535, its rows given as runs
        // of its width: drawn where it meets the plane.
        let rows = vec![(65535, 1); 2000];
        plane.apply(&overlay(command::DRAW, 100, 50, 65535, 65535, runs(rows)));
        let (x, y, width, height, _) = plane.picture().expect("a picture");
        assert_eq!((x, y, width, height), (100, 50, 1820, 1030));
    }

    #[test]
    fn argb_draws_premultiplied_and_other_planes_are_left_alone() {
        let mut plane = Plane::default();
        plane.apply(&overlay(command::INIT, 0, 0, 720, 480, Image::None));
        plane.apply(&overlay(
            command::DRAW,
            10,
            10,
            2,
            1,
            Image::Argb(vec![0xFF00_FF00, 0x80FF_0000]),
        ));
        let (x, y, width, height, pixels) = plane.picture().expect("a picture");
        assert_eq!((x, y, width, height), (10, 10, 2, 1));
        assert_eq!(pixels, [0, 255, 0, 255, 0, 0, 128, 128]);
        // The subtitles' plane (PG) is mpv's.
        let mut subtitles = overlay(command::CLEAR, 0, 0, 0, 0, Image::None);
        subtitles.plane = 0;
        assert!(!plane.apply(&subtitles));
        assert!(plane.picture().is_some());
    }
}
