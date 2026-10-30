//! A Blu-ray menu's graphics (M16): the interactive graphics plane that
//! libbluray draws with its overlay events (`overlay.h`), kept as
//! premultiplied BGRA for mpv's `overlay-add`. HDMV menus come as
//! palette-coded runs (Y, Cr, Cb and transparency, BT.709 on HD planes,
//! BT.601 on SD ones), BD-J menus as ARGB. RVP shows the interactive
//! plane only: mpv draws the subtitles (PG) of the stream itself.

/// libbluray's planes (`bd_overlay_plane_e`).
pub const PLANE_IG: u8 = 1;

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
                self.width = usize::from(overlay.width);
                self.height = usize::from(overlay.height);
                self.pixels = vec![0; self.width * self.height * 4];
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
                let hd = self.height > 576;
                let colours: Vec<[u8; 4]> = palette.iter().map(|e| bgra(*e, hd)).collect();
                let mut indices =
                    Vec::with_capacity(usize::from(overlay.width) * usize::from(overlay.height));
                let mut line = 0;
                for &(length, colour) in runs {
                    if length == 0 {
                        // The line ends: pad it.
                        line += 1;
                        indices.resize(line * usize::from(overlay.width), 0);
                        continue;
                    }
                    indices.extend(std::iter::repeat_n(colour, usize::from(length)));
                }
                let width = usize::from(overlay.width);
                self.fill(overlay, |x, y| {
                    indices
                        .get(y * width + x)
                        .and_then(|&index| colours.get(usize::from(index)))
                        .copied()
                        .unwrap_or([0; 4])
                });
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

    /// Sets each pixel of the overlay's rectangle (inside the plane) to
    /// `pixel(x, y)`, with x and y from the rectangle's corner.
    fn fill(&mut self, overlay: &Overlay, pixel: impl Fn(usize, usize) -> [u8; 4]) {
        let (x0, y0) = (usize::from(overlay.x), usize::from(overlay.y));
        let x1 = (x0 + usize::from(overlay.width)).min(self.width);
        let y1 = (y0 + usize::from(overlay.height)).min(self.height);
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
