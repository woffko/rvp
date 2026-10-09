//! Image comparison helpers for golden-image tests and reference checks.

use crate::geometry::Rect;
use crate::image::Image;

/// Composites an image over an opaque background colour.
#[must_use]
pub fn flatten(image: &Image, background: [u8; 3]) -> Image {
    let mut out = image.clone();
    for pixel in out.data_mut().as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        for channel in 0..3 {
            let value = (u32::from(pixel[channel]) * alpha
                + u32::from(background[channel]) * (255 - alpha))
                / 255;
            pixel[channel] = u8::try_from(value).unwrap_or(u8::MAX);
        }
        pixel[3] = 255;
    }
    out
}

/// Resizes with an exact box filter: every output pixel is the area-weighted
/// mean of the input pixels it covers. Used to match screenshots that were
/// scaled down.
#[must_use]
pub fn resize_box(image: &Image, width: u32, height: u32) -> Image {
    let mut out = Image::new(width, height);
    if width == 0 || height == 0 || image.width() == 0 || image.height() == 0 {
        return out;
    }
    let scale_x = f64::from(image.width()) / f64::from(width);
    let scale_y = f64::from(image.height()) / f64::from(height);
    let source = image.data();
    let source_width = image.width() as usize;
    let mut column_weights: Vec<Vec<(usize, f64)>> = Vec::with_capacity(width as usize);
    for x in 0..width {
        column_weights.push(weights(f64::from(x), scale_x, image.width()));
    }
    for y in 0..height {
        let rows = weights(f64::from(y), scale_y, image.height());
        for (x, columns) in column_weights.iter().enumerate() {
            let mut sum = [0.0_f64; 4];
            let mut total = 0.0;
            for &(row, row_weight) in &rows {
                for &(column, column_weight) in columns {
                    let weight = row_weight * column_weight;
                    let offset = (row * source_width + column) * 4;
                    for (accumulator, &value) in sum.iter_mut().zip(&source[offset..offset + 4]) {
                        *accumulator += f64::from(value) * weight;
                    }
                    total += weight;
                }
            }
            let offset = (y as usize * width as usize + x) * 4;
            for (target, accumulated) in out.data_mut()[offset..offset + 4].iter_mut().zip(sum) {
                let value = (accumulated / total).round().clamp(0.0, 255.0);
                // Clamped to 0..=255 above.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let byte = value as u8;
                *target = byte;
            }
        }
    }
    out
}

/// Source pixels covered by output pixel `index` and their overlap.
fn weights(index: f64, scale: f64, limit: u32) -> Vec<(usize, f64)> {
    let start = index * scale;
    let end = start + scale;
    let mut out = Vec::new();
    // Coordinates are small and non-negative.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (first, last) = (start.floor() as u32, (end.ceil() as u32).min(limit));
    for pixel in first..last {
        let overlap = (end.min(f64::from(pixel + 1)) - start.max(f64::from(pixel))).max(0.0);
        if overlap > 0.0 {
            out.push((pixel as usize, overlap));
        }
    }
    out
}

/// Mean absolute difference per channel (0-255) between `a` and `b` over
/// `region`, with `b` shifted by `(dx, dy)`. Pixels outside either image
/// are skipped. Returns `None` when nothing overlaps.
#[must_use]
pub fn mean_abs_diff(a: &Image, b: &Image, region: Rect, dx: i32, dy: i32) -> Option<f64> {
    let mut sum = 0_u64;
    let mut count = 0_u64;
    for y in region.y..region.bottom() {
        for x in region.x..region.right() {
            let (Some(first), Some(second)) = (a.offset(x, y), b.offset(x + dx, y + dy)) else {
                continue;
            };
            for channel in 0..3 {
                sum += u64::from(a.data()[first + channel].abs_diff(b.data()[second + channel]));
            }
            count += 3;
        }
    }
    // Pixel counts stay far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let mean = (count > 0).then(|| sum as f64 / count as f64);
    mean
}

/// A grey-scale picture of the per-pixel difference, amplified four times.
#[must_use]
pub fn diff_image(a: &Image, b: &Image, dx: i32, dy: i32) -> Image {
    let mut out = Image::new(a.width(), a.height());
    let width = i32::try_from(a.width()).unwrap_or(0);
    let height = i32::try_from(a.height()).unwrap_or(0);
    for y in 0..height {
        for x in 0..width {
            let (Some(first), Some(second)) = (a.offset(x, y), b.offset(x + dx, y + dy)) else {
                continue;
            };
            let mut difference = 0_u32;
            for channel in 0..3 {
                difference +=
                    u32::from(a.data()[first + channel].abs_diff(b.data()[second + channel]));
            }
            let value = u8::try_from((difference * 4 / 3).min(255)).unwrap_or(u8::MAX);
            out.data_mut()[first..first + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{mean_abs_diff, resize_box};
    use crate::geometry::Rect;
    use crate::image::Image;

    #[test]
    fn box_resize_averages() {
        let mut image = Image::new(2, 1);
        image
            .data_mut()
            .copy_from_slice(&[0, 0, 0, 255, 200, 100, 50, 255]);
        let small = resize_box(&image, 1, 1);
        assert_eq!(small.pixel(0, 0), [100, 50, 25, 255]);
    }

    #[test]
    fn difference_of_identical_images_is_zero() {
        let image = Image::new(3, 3);
        assert_eq!(
            mean_abs_diff(&image, &image, Rect::new(0, 0, 3, 3), 0, 0),
            Some(0.0)
        );
        assert_eq!(
            mean_abs_diff(&image, &image, Rect::new(0, 0, 3, 3), 5, 5),
            None
        );
    }
}
