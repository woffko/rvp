//! PotPlayer's colour themes (M14): a skin lists them under `<ColorTheme>`
//! in its video skin file, and Skins > Color Themes tints the skin's
//! images with one of them.
//!
//! * `<ColorThemeRGB Name Red Green Blue>` adds an offset to each channel
//!   (-255 to 255 each).
//! * `<ColorThemeYUV Name Hue Sat>` (PotPlayer's "HSB" themes) colours the
//!   images: each pixel keeps its lightness and takes the theme's hue
//!   (`Hue` from -128 to 128 is the colour wheel from red round to red)
//!   with `Sat` percent of saturation.
//! * `<ExcludeColorThemeArea Name='Main.png' Left Top Width Height>`
//!   keeps a part of an image out of every theme (in any of the skin's
//!   files).
//!
//! PotPlayer's own formula was not recorded (R1 has no captures of its
//! themes), so these are RVP's reading of the elements: the names match the
//! colours for most of BlackSpace's RGB themes; the HSB ones are RVP's.

use crate::geometry::Rect;
use crate::image::Image;
use crate::markup::Element;

/// How a theme changes the skin's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    /// Offsets added to red, green and blue.
    Rgb([i32; 3]),
    /// A hue (-128 to 128) and a saturation (0 to 100) every pixel takes.
    Hsb { hue: i32, sat: i32 },
}

/// One theme of a skin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorTheme {
    pub name: String,
    pub tint: Tint,
}

/// The themes a skin lists, in its order.
#[must_use]
pub fn themes(root: &Element) -> Vec<ColorTheme> {
    let mut found = Vec::new();
    root.walk(&mut |element| {
        let attrs = &element.attrs;
        let name = attrs.get("Name").unwrap_or("").trim().to_owned();
        let tint = if element.is("ColorThemeRGB") {
            Tint::Rgb([
                attrs.int_or("Red", 0),
                attrs.int_or("Green", 0),
                attrs.int_or("Blue", 0),
            ])
        } else if element.is("ColorThemeYUV") {
            Tint::Hsb {
                hue: attrs.int_or("Hue", 0),
                sat: attrs.int_or("Sat", 0),
            }
        } else {
            return;
        };
        found.push(ColorTheme { name, tint });
    });
    found
}

/// The parts of images every theme leaves alone: the image's name (lower
/// case) and the rectangle.
#[must_use]
pub fn exclusions(root: &Element) -> Vec<(String, Rect)> {
    let mut found = Vec::new();
    root.walk(&mut |element| {
        if !element.is("ExcludeColorThemeArea") {
            return;
        }
        let attrs = &element.attrs;
        let Some(name) = attrs.get("Name") else {
            return;
        };
        found.push((
            name.trim().to_ascii_lowercase(),
            Rect::new(
                attrs.int_or("Left", 0),
                attrs.int_or("Top", 0),
                attrs.int_or("Width", 0),
                attrs.int_or("Height", 0),
            ),
        ));
    });
    found
}

/// The theme's colour for one pixel (alpha stays).
#[must_use]
pub fn tint_pixel(tint: Tint, [r, g, b]: [u8; 3]) -> [u8; 3] {
    match tint {
        Tint::Rgb(offsets) => {
            let shift = |value: u8, offset: i32| {
                // Within 0..=255 after the clamp.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let shifted = (i32::from(value) + offset.clamp(-255, 255)).clamp(0, 255) as u8;
                shifted
            };
            [
                shift(r, offsets[0]),
                shift(g, offsets[1]),
                shift(b, offsets[2]),
            ]
        }
        Tint::Hsb { hue, sat } => {
            let lightness = (f64::from(r.max(g).max(b)) + f64::from(r.min(g).min(b))) / 510.0;
            let hue = f64::from(hue.clamp(-128, 128) + 128) / 256.0;
            let sat = f64::from(sat.clamp(0, 100)) / 100.0;
            hsl_to_rgb(hue, sat, lightness)
        }
    }
}

/// HSL (each 0 to 1) to RGB.
fn hsl_to_rgb(hue: f64, sat: f64, lightness: f64) -> [u8; 3] {
    let q = if lightness < 0.5 {
        lightness * (1.0 + sat)
    } else {
        lightness + sat - lightness * sat
    };
    let p = 2.0 * lightness - q;
    let channel = |offset: f64| {
        let mut t = (hue + offset).rem_euclid(1.0);
        if t < 0.0 {
            t += 1.0;
        }
        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        // Within 0..=255 after the clamp.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let byte = (value * 255.0).round().clamp(0.0, 255.0) as u8;
        byte
    };
    [channel(1.0 / 3.0), channel(0.0), channel(-1.0 / 3.0)]
}

/// Tints `image` but the rectangles of `keep`.
pub fn tint_image(image: &mut Image, tint: Tint, keep: &[Rect]) {
    let width = usize::try_from(image.width()).unwrap_or(0);
    if width == 0 {
        return;
    }
    let inside = |x: usize, y: usize| {
        let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
            return false;
        };
        keep.iter().any(|rect| rect.contains(x, y))
    };
    for (index, pixel) in image
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        if !keep.is_empty() && inside(index % width, index / width) {
            continue;
        }
        let [r, g, b] = tint_pixel(tint, [pixel[0], pixel[1], pixel[2]]);
        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_themes_shift_and_hsb_themes_colour() {
        assert_eq!(
            tint_pixel(Tint::Rgb([50, -20, 300]), [100, 10, 100]),
            [150, 0, 255]
        );
        // Grey keeps its lightness and takes red at full saturation.
        assert_eq!(
            tint_pixel(
                Tint::Hsb {
                    hue: -128,
                    sat: 100
                },
                [128, 128, 128]
            ),
            [255, 1, 1]
        );
        // No saturation: grey stays grey.
        assert_eq!(
            tint_pixel(Tint::Hsb { hue: 40, sat: 0 }, [60, 90, 120]),
            [90, 90, 90]
        );
    }
}
