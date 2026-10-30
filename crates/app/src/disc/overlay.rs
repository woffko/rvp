//! A menu's sub-picture as RVP draws it over the video (M16): the disc's
//! colours (its CLUT) for each pixel value, and inside the selected
//! button's rectangle the button's own colours and contrast instead, as a
//! DVD player does. The result is premultiplied BGRA, mpv's `overlay-add`
//! format, scaled to the size the video shows at.

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

/// Scales BGRA of `width` x `height` to `to_width` x `to_height` (nearest
/// pixel: menus are flat colours, and a filter would blur their edges).
#[must_use]
pub fn scale(
    image: &[u8],
    width: usize,
    height: usize,
    to_width: usize,
    to_height: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; to_width * to_height * 4];
    if width == 0 || height == 0 {
        return out;
    }
    for (y, row) in out.chunks_exact_mut(to_width * 4).enumerate() {
        let from_y = (y * height / to_height.max(1)).min(height - 1);
        for (x, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let from_x = (x * width / to_width.max(1)).min(width - 1);
            let at = (from_y * width + from_x) * 4;
            pixel.copy_from_slice(&image[at..at + 4]);
        }
    }
    out
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
        let scaled = scale(&image, 2, 2, 4, 4);
        assert_eq!(scaled.len(), 4 * 4 * 4);
        assert_eq!(&scaled[..4], &[1, 2, 3, 255]);
        assert_eq!(&scaled[4..8], &[1, 2, 3, 255]);
        assert_eq!(&scaled[8..12], &[4, 5, 6, 255]);
        let last = scaled.len() - 4;
        assert_eq!(&scaled[last..], &[10, 11, 12, 255]);
    }
}
