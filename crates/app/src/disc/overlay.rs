//! A menu's sub-picture as RVP draws it over the video (M16): the disc's
//! colours (its CLUT) for each pixel value, and inside the selected
//! button's rectangle the button's own colours and contrast instead, as a
//! DVD player does. The result is premultiplied BGRA, mpv's `overlay-add`
//! format, scaled to the size the video shows at.
//!
//! The disc gives the picture's size and place (a DVD's sub-picture may
//! be 4096 x 4096 over a frame of 352 x 240), so [`place`] scales only
//! the part inside the frame and draws only the part inside the OSD: the
//! result is never larger than the OSD.

use super::spu::SubPicture;

/// A disc's colour table: 16 entries of `0x00YYCrCb`.
pub type Clut = [u32; 16];

/// The selected (or activated) button: its rectangle in frame pixels,
/// both ends included, and its palette as libdvdnav gives it: the four
/// pixel values' colours in bits 16 to 31 and their contrast in bits 0 to
/// 15, four bits each, value 0 lowest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Highlight {
    pub x0: u16,
    pub y0: u16,
    pub x1: u16,
    pub y1: u16,
    pub palette: u32,
}

impl Highlight {
    fn contains(&self, x: u16, y: u16) -> bool {
        (self.x0..=self.x1).contains(&x) && (self.y0..=self.y1).contains(&y)
    }

    fn colour(&self, value: u8) -> u8 {
        ((self.palette >> (16 + 4 * u32::from(value))) & 0xf) as u8
    }

    fn contrast(&self, value: u8) -> u8 {
        ((self.palette >> (4 * u32::from(value))) & 0xf) as u8
    }
}

/// A CLUT entry as premultiplied BGRA with `contrast` (0 to 15): BT.601's
/// limited range, as DVDs use.
fn bgra(entry: u32, contrast: u8) -> [u8; 4] {
    let y = f64::from((entry >> 16) & 0xff) - 16.0;
    let cr = f64::from((entry >> 8) & 0xff) - 128.0;
    let cb = f64::from(entry & 0xff) - 128.0;
    let channel = |v: f64| v.round().clamp(0.0, 255.0);
    let r = channel(1.164 * y + 1.596 * cr);
    let g = channel(1.164 * y - 0.813 * cr - 0.391 * cb);
    let b = channel(1.164 * y + 2.018 * cb);
    let alpha = f64::from(contrast) * 17.0;
    // Premultiplied: each channel at most the alpha (255).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let premultiply = |v: f64| (v * alpha / 255.0).round() as u8;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    [premultiply(b), premultiply(g), premultiply(r), alpha as u8]
}

/// The sub-picture's pixels in premultiplied BGRA (its own width and
/// height), the highlight's colours inside its rectangle.
#[must_use]
pub fn compose(picture: &SubPicture, clut: &Clut, highlight: Option<&Highlight>) -> Vec<u8> {
    let mut out = vec![0u8; picture.pixels.len() * 4];
    let width = usize::from(picture.width);
    if width == 0 {
        return out;
    }
    let rows = picture
        .pixels
        .chunks_exact(width)
        .zip(out.chunks_exact_mut(width * 4));
    for ((row, out_row), y) in rows.zip(picture.y..) {
        let (pixels, _) = out_row.as_chunks_mut::<4>();
        for ((&value, pixel), x) in row.iter().zip(pixels).zip(picture.x..) {
            let value = value & 3;
            let (colour, contrast) = match highlight {
                Some(button) if button.contains(x, y) => {
                    (button.colour(value), button.contrast(value))
                }
                _ => (
                    picture.colours[usize::from(value)],
                    picture.contrast[usize::from(value)],
                ),
            };
            if contrast > 0 {
                *pixel = bgra(clut[usize::from(colour & 0xf)], contrast);
            }
        }
    }
    out
}

/// Where the video shows: its frame's size in pixels, its rectangle on
/// the OSD (left, top, width, height in OSD pixels, as mpv's
/// `osd-dimensions` give it; larger than the OSD when zoomed in) and the
/// OSD's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub frame: (usize, usize),
    pub video: (f64, f64, f64, f64),
    pub osd: (usize, usize),
}

/// A picture placed on the OSD: its corner and size in OSD pixels and its
/// BGRA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

/// The BGRA `image` of `width` x `height` at `x`, `y` in frame pixels as
/// it shows on the OSD: the part inside the frame, scaled to the video's
/// size (nearest pixel: menus are flat colours, and a filter would blur
/// their edges), of which the part inside the OSD. `None` when no part
/// shows.
#[must_use]
pub fn place(
    image: &[u8],
    (x, y): (usize, usize),
    (width, height): (usize, usize),
    view: &View,
) -> Option<Placed> {
    let (left, top, video_width, video_height) = view.video;
    if image.len() < width.checked_mul(height)?.checked_mul(4)?
        || !(video_width >= 1.0 && video_height >= 1.0)
    {
        return None;
    }
    let (first_column, columns) = axis(x, width, view.frame.0, left, video_width, view.osd.0)?;
    let (first_row, rows) = axis(y, height, view.frame.1, top, video_height, view.osd.1)?;
    let mut pixels = Vec::with_capacity(columns.len() * rows.len() * 4);
    for &row in &rows {
        let line = &image[row * width * 4..(row + 1) * width * 4];
        for &column in &columns {
            pixels.extend_from_slice(&line[column * 4..column * 4 + 4]);
        }
    }
    Some(Placed {
        left: first_column,
        top: first_row,
        width: columns.len(),
        height: rows.len(),
        pixels,
    })
}

/// One direction of [`place`]: the picture's `length` pixels from `start`
/// on a frame of `frame` pixels, shown at `offset` with `size` OSD pixels
/// for the whole frame, on an OSD of `osd` pixels. The first OSD pixel
/// the picture covers and, for each OSD pixel from there, the picture's
/// pixel that shows in it.
fn axis(
    start: usize,
    length: usize,
    frame: usize,
    offset: f64,
    size: f64,
    osd: usize,
) -> Option<(usize, Vec<usize>)> {
    let (from, to) = (start.min(frame), start.saturating_add(length).min(frame));
    if from >= to {
        return None;
    }
    // Frames and OSDs are a few thousand pixels: exact in a double.
    #[allow(clippy::cast_precision_loss)]
    let (from_f, to_f, frame_f, osd_f) = (from as f64, to as f64, frame as f64, osd as f64);
    let scale = size / frame_f;
    let (begins, ends) = (offset + from_f * scale, offset + to_f * scale);
    if ends <= 0.0 || begins >= osd_f {
        return None;
    }
    // Rounded and clamped at 0 first; the cast saturates above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixel = |value: f64| value.round().max(0.0) as usize;
    let first = pixel(begins).min(osd);
    // At least one pixel.
    let last = pixel(ends).max(first + 1).min(osd);
    if first >= last {
        return None;
    }
    let map = (first..last)
        .map(|at| {
            // The frame pixel under this OSD pixel's centre.
            #[allow(clippy::cast_precision_loss)]
            let centre = at as f64 + 0.5;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let source = ((centre - offset) / scale).floor().max(0.0) as usize;
            source.clamp(from, to - 1) - start
        })
        .collect();
    Some((first, map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disc::spu::{decode, tests::encode, tests::sample};

    /// Black, white, yellow and orange in 0x00YYCrCb (BT.601).
    const CLUT: Clut = [
        0x0010_8080,
        0x00EB_8080,
        0x00D2_9210,
        0x00A0_C030,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];

    #[test]
    fn colours_follow_the_clut_and_the_highlight_inside_its_rectangle() {
        // A 40x12 sub-picture at 80, 190: its frame pixels (value 1)
        // white, the inner box (value 2) black at half contrast.
        let pixels = sample(40, 12);
        let picture =
            decode(&encode(80, 190, 40, &pixels, [0, 1, 0, 0], [0, 15, 8, 0])).expect("decodes");
        let plain = compose(&picture, &CLUT, None);
        // The corner pixel: white (limited range's 235 is full white),
        // opaque.
        assert_eq!(&plain[..4], &[255, 255, 255, 255]);
        // Background (value 0): transparent.
        // Row 1, column 1 of 40.
        let at = (40 + 1) * 4;
        assert_eq!(&plain[at..at + 4], &[0, 0, 0, 0]);
        // The highlight turns value 1 to entry 2 (yellow), opaque, in the
        // left half only.
        let button = Highlight {
            x0: 80,
            y0: 190,
            x1: 99,
            y1: 201,
            palette: 0x0020_00f0,
        };
        let lit = compose(&picture, &CLUT, Some(&button));
        assert_eq!(&lit[..4], &bgra(CLUT[2], 15));
        assert_ne!(&lit[..4], &plain[..4]);
        let right = 39 * 4;
        assert_eq!(
            &lit[right..right + 4],
            &plain[right..right + 4],
            "outside the button"
        );
        // Inside the rectangle, value 2 has no contrast in the highlight.
        let inner = (5 * 40 + 5) * 4;
        assert_eq!(&lit[inner..inner + 4], &[0, 0, 0, 0]);
        assert_ne!(&plain[inner..inner + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn scaling_keeps_flat_colours() {
        let image: Vec<u8> = [
            [1, 2, 3, 255],
            [4, 5, 6, 255],
            [7, 8, 9, 255],
            [10, 11, 12, 255],
        ]
        .concat();
        let view = View {
            frame: (2, 2),
            video: (0.0, 0.0, 4.0, 4.0),
            osd: (4, 4),
        };
        let placed = place(&image, (0, 0), (2, 2), &view).expect("placed");
        assert_eq!(
            (placed.left, placed.top, placed.width, placed.height),
            (0, 0, 4, 4)
        );
        let scaled = placed.pixels;
        assert_eq!(scaled.len(), 4 * 4 * 4);
        assert_eq!(&scaled[..4], &[1, 2, 3, 255]);
        assert_eq!(&scaled[4..8], &[1, 2, 3, 255]);
        assert_eq!(&scaled[8..12], &[4, 5, 6, 255]);
        let last = scaled.len() - 4;
        assert_eq!(&scaled[last..], &[10, 11, 12, 255]);
    }

    #[test]
    fn a_picture_larger_than_its_frame_is_cut_to_it_before_scaling() {
        // A 4096 x 4096 sub-picture over a 352 x 240 frame, the video
        // shown 3168 x 2160 in the middle of a 3840 x 2160 screen.
        let image = vec![255u8; 4096 * 4096 * 4];
        let view = View {
            frame: (352, 240),
            video: (336.0, 0.0, 3168.0, 2160.0),
            osd: (3840, 2160),
        };
        let placed = place(&image, (0, 0), (4096, 4096), &view).expect("placed");
        assert_eq!(
            (placed.left, placed.top, placed.width, placed.height),
            (336, 0, 3168, 2160)
        );
        assert_eq!(placed.pixels.len(), 3168 * 2160 * 4);
        // Outside the frame: nothing shows.
        assert_eq!(place(&image[..4], (352, 0), (1, 1), &view), None);
        assert_eq!(place(&image[..4], (0, 240), (1, 1), &view), None);
    }

    #[test]
    fn a_zoomed_video_draws_only_the_osds_part() {
        // A 720 x 480 frame zoomed four times around the middle of a
        // 1440 x 960 OSD.
        let mut image = vec![0u8; 720 * 480 * 4];
        let at = (120 * 720 + 180) * 4;
        image[at..at + 4].copy_from_slice(&[1, 2, 3, 255]);
        let view = View {
            frame: (720, 480),
            video: (-720.0, -480.0, 2880.0, 1920.0),
            osd: (1440, 960),
        };
        let placed = place(&image, (0, 0), (720, 480), &view).expect("placed");
        assert_eq!(
            (placed.left, placed.top, placed.width, placed.height),
            (0, 0, 1440, 960)
        );
        // The OSD's corner shows frame pixel 180, 120.
        assert_eq!(&placed.pixels[..4], &[1, 2, 3, 255]);
        // A picture left of the OSD does not show.
        assert_eq!(place(&image[..16], (0, 200), (2, 2), &view), None);
    }
}
