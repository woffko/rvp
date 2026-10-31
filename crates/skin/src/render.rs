//! A pure-Rust rasterizer for display lists, used for tests and previews.

use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::image::Image;
use crate::paint::{DisplayList, HAlign, Op, TextOp, VAlign};
use crate::skin::Skin;

/// Draws a display list into a new transparent image.
#[must_use]
pub fn rasterize(list: &DisplayList, skin: &Skin, fonts: &mut Fonts) -> Image {
    let width = u32::try_from(list.width.max(0)).unwrap_or(0);
    let height = u32::try_from(list.height.max(0)).unwrap_or(0);
    let mut canvas = Image::new(width, height);
    for op in &list.ops {
        match op {
            Op::Image {
                atlas,
                src,
                dst,
                clip,
            } => {
                if let Some(atlas) = skin.image(atlas) {
                    blit_tiled(&mut canvas, atlas, *src, *dst, *clip);
                }
            }
            Op::Stretch {
                atlas,
                src,
                dst,
                clip,
            } => {
                if let Some(atlas) = skin.image(atlas) {
                    blit_scaled(&mut canvas, atlas, *src, *dst, *clip);
                }
            }
            Op::Fill { rect, color } => fill(&mut canvas, *rect, *color),
            Op::Text(text) => draw_text(&mut canvas, fonts, text),
        }
    }
    canvas
}

/// The opaque parts of a display list (alpha above 0) as rectangles, as
/// `x,y,w,h;...` for a window's shape (M9: VLC's windows are the union of
/// their drawn pixels). Equal runs of neighbouring rows join; texts do not
/// count.
#[must_use]
pub fn opaque_region(list: &DisplayList, skin: &Skin) -> String {
    let image = rasterize(list, skin, &mut Fonts::new(Vec::new()));
    let (width, height) = (image.width() as usize, image.height() as usize);
    let data = image.data();
    let mut open: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut done: Vec<(usize, usize, usize, usize)> = Vec::new();
    for y in 0..height {
        let mut runs = Vec::new();
        let mut x = 0;
        while x < width {
            if data[(y * width + x) * 4 + 3] == 0 {
                x += 1;
                continue;
            }
            let start = x;
            while x < width && data[(y * width + x) * 4 + 3] != 0 {
                x += 1;
            }
            runs.push((start, x - start));
        }
        let mut next = Vec::with_capacity(runs.len());
        for (start, length) in runs {
            match open
                .iter()
                .position(|(x, _, w, _)| *x == start && *w == length)
            {
                Some(index) => {
                    let mut rect = open.swap_remove(index);
                    rect.3 += 1;
                    next.push(rect);
                }
                None => next.push((start, y, length, 1)),
            }
        }
        done.append(&mut open);
        open = next;
    }
    done.append(&mut open);
    done.sort_unstable();
    done.iter()
        .map(|(x, y, w, h)| format!("{x},{y},{w},{h}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn bounds(image: &Image) -> Rect {
    Rect::new(
        0,
        0,
        i32::try_from(image.width()).unwrap_or(i32::MAX),
        i32::try_from(image.height()).unwrap_or(i32::MAX),
    )
}

/// Byte range of pixels `x..x + count` in row `y`. Callers clip first.
fn span(image: &Image, x: i32, y: i32, count: i32) -> std::ops::Range<usize> {
    debug_assert!(x >= 0 && y >= 0 && count >= 0);
    let width = image.width() as usize;
    let start = (y.unsigned_abs() as usize * width + x.unsigned_abs() as usize) * 4;
    start..start + count.unsigned_abs() as usize * 4
}

/// Source-over blending of straight-alpha pixels; `coverage` scales the
/// source alpha (255 = full).
fn blend(dst: &mut [u8; 4], src: [u8; 4], coverage: u8) {
    let alpha = u32::from(src[3]) * u32::from(coverage) / 255;
    if alpha == 0 {
        return;
    }
    if alpha == 255 {
        *dst = [src[0], src[1], src[2], 255];
        return;
    }
    let dst_alpha = u32::from(dst[3]) * (255 - alpha) / 255;
    let out_alpha = alpha + dst_alpha;
    for channel in 0..3 {
        let value =
            (u32::from(src[channel]) * alpha + u32::from(dst[channel]) * dst_alpha) / out_alpha;
        dst[channel] = u8::try_from(value).unwrap_or(u8::MAX);
    }
    dst[3] = u8::try_from(out_alpha).unwrap_or(u8::MAX);
}

/// Tiles `src` of `atlas` over `dst`, clipped to `clip` and the canvas.
/// Parts of `src` outside the atlas are transparent.
/// Scales `src` to `dst` (nearest pixel), clipped.
fn blit_scaled(canvas: &mut Image, atlas: &Image, src: Rect, dst: Rect, clip: Rect) {
    let area = dst.intersect(&clip).intersect(&bounds(canvas));
    if area.is_empty() || src.is_empty() || dst.is_empty() {
        return;
    }
    for y in area.y..area.bottom() {
        let source_y = src.y + (y - dst.y) * src.h / dst.h;
        for x in area.x..area.right() {
            let source_x = src.x + (x - dst.x) * src.w / dst.w;
            let pixel = atlas.pixel(source_x, source_y);
            let target = span(canvas, x, y, 1);
            if let Some(dst_pixel) = canvas.data_mut()[target].as_chunks_mut::<4>().0.first_mut() {
                blend(dst_pixel, pixel, 255);
            }
        }
    }
}

fn blit_tiled(canvas: &mut Image, atlas: &Image, src: Rect, dst: Rect, clip: Rect) {
    let area = dst.intersect(&clip).intersect(&bounds(canvas));
    let atlas_bounds = bounds(atlas);
    if area.is_empty() || src.is_empty() {
        return;
    }
    for y in area.y..area.bottom() {
        let source_y = src.y + (y - dst.y).rem_euclid(src.h);
        if source_y < 0 || source_y >= atlas_bounds.h {
            continue;
        }
        let mut x = area.x;
        while x < area.right() {
            let tile_offset = (x - dst.x).rem_euclid(src.w);
            let run = (src.w - tile_offset).min(area.right() - x);
            let source_x = src.x + tile_offset;
            let first = source_x.max(0);
            let last = (source_x + run).min(atlas_bounds.w);
            if first < last {
                let count = last - first;
                let target_x = x + (first - source_x);
                let source = span(atlas, first, source_y, count);
                let target = span(canvas, target_x, y, count);
                let (source_data, target_data) = (atlas.data(), canvas.data_mut());
                for (dst_pixel, src_pixel) in target_data[target]
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(source_data[source].as_chunks::<4>().0)
                {
                    blend(dst_pixel, *src_pixel, 255);
                }
            }
            x += run;
        }
    }
}

fn fill(canvas: &mut Image, rect: Rect, color: [u8; 4]) {
    let area = rect.intersect(&bounds(canvas));
    if area.is_empty() {
        return;
    }
    for y in area.y..area.bottom() {
        let range = span(canvas, area.x, y, area.w);
        for pixel in canvas.data_mut()[range].as_chunks_mut::<4>().0 {
            blend(pixel, color, 255);
        }
    }
}

fn text_width(glyphs: &[(u16, i32)]) -> i32 {
    glyphs.iter().map(|(_, advance)| advance).sum()
}

/// Removes characters from the end until the text plus `...` fits.
fn ellipsize(fonts: &Fonts, face: usize, size: f32, text: &str, width: i32) -> Vec<(u16, i32)> {
    let glyphs = fonts.shape(face, size, text);
    if text_width(&glyphs) <= width {
        return glyphs;
    }
    // `shape` returns one glyph per character, so indices line up.
    let chars: Vec<char> = text.chars().collect();
    let dots = fonts.shape(face, size, "...");
    let budget = width - text_width(&dots);
    let mut count = glyphs.len();
    let mut used = text_width(&glyphs);
    while count > 0 && used > budget {
        count -= 1;
        used -= glyphs[count].1;
    }
    while count > 0 && chars[count - 1].is_whitespace() {
        count -= 1;
    }
    let mut kept = glyphs[..count].to_vec();
    kept.extend(dots);
    kept
}

/// Space between a text and its right-aligned trailing text.
const TRAILING_GAP: i32 = 4;

fn draw_text(canvas: &mut Image, fonts: &mut Fonts, op: &TextOp) {
    let Some(face) = fonts.face(&op.font) else {
        return;
    };
    let size = op.font.size_px;
    let trailing = op
        .trailing
        .as_deref()
        .map(|text| fonts.shape(face, size, text))
        .unwrap_or_default();
    let reserved = if trailing.is_empty() {
        0
    } else {
        text_width(&trailing) + TRAILING_GAP
    };
    let available = op.rect.w - reserved;
    let glyphs = if op.ellipsis {
        ellipsize(fonts, face, size, &op.text, available)
    } else {
        fonts.shape(face, size, &op.text)
    };
    let width = text_width(&glyphs);
    let metrics = fonts.line_metrics(face, size);
    // Pixel metrics of UI fonts are small; rounding to i32 is exact enough.
    #[allow(clippy::cast_possible_truncation)]
    let (ascent, descent) = (
        metrics.ascent.round() as i32,
        metrics.descent.round() as i32,
    );
    let pen = match op.h_align {
        HAlign::Left => op.rect.x,
        HAlign::Center => op.rect.x + (available - width) / 2,
        HAlign::Right => op.rect.x + available - width,
    };
    let baseline = match op.v_align {
        VAlign::Top => op.rect.y + ascent,
        VAlign::Center => op.rect.y + (op.rect.h - ascent - descent) / 2 + ascent,
        VAlign::Bottom => op.rect.bottom() - descent,
    };
    let clip = op.clip.intersect(&bounds(canvas));
    let run = GlyphRun {
        face,
        size,
        baseline,
        color: op.color,
        clip,
    };
    draw_run(canvas, fonts, &run, &glyphs, pen);
    if !trailing.is_empty() {
        let trailing_pen = op.rect.right() - text_width(&trailing);
        draw_run(canvas, fonts, &run, &trailing, trailing_pen);
    }
}

struct GlyphRun {
    face: usize,
    size: f32,
    baseline: i32,
    color: [u8; 4],
    clip: Rect,
}

fn draw_run(
    canvas: &mut Image,
    fonts: &mut Fonts,
    run: &GlyphRun,
    glyphs: &[(u16, i32)],
    mut pen: i32,
) {
    for &(glyph, advance) in glyphs {
        if let Some(bitmap) = fonts.glyph(run.face, glyph, run.size) {
            draw_glyph(canvas, bitmap, pen, run.baseline, run.color, run.clip);
        }
        pen += advance;
    }
}

fn draw_glyph(
    canvas: &mut Image,
    bitmap: &crate::fonts::GlyphBitmap,
    pen: i32,
    baseline: i32,
    color: [u8; 4],
    clip: Rect,
) {
    let width = i32::try_from(bitmap.width).unwrap_or(0);
    let height = i32::try_from(bitmap.height).unwrap_or(0);
    let placed = Rect::new(pen + bitmap.left, baseline - bitmap.top, width, height);
    let area = placed.intersect(&clip);
    if area.is_empty() {
        return;
    }
    let row_length = bitmap.width as usize;
    for y in area.y..area.bottom() {
        let mask_row = (y - placed.y).unsigned_abs() as usize * row_length;
        let mask_start = mask_row + (area.x - placed.x).unsigned_abs() as usize;
        let mask = &bitmap.alpha[mask_start..mask_start + area.w.unsigned_abs() as usize];
        let range = span(canvas, area.x, y, area.w);
        for (pixel, &coverage) in canvas.data_mut()[range]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(mask)
        {
            blend(pixel, color, coverage);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{blend, blit_tiled};
    use crate::geometry::Rect;
    use crate::image::Image;

    #[test]
    fn blending() {
        let mut pixel = [0, 0, 0, 255];
        blend(&mut pixel, [255, 255, 255, 255], 128);
        assert_eq!(pixel, [128, 128, 128, 255]);
        let mut transparent = [0, 0, 0, 0];
        blend(&mut transparent, [200, 100, 50, 128], 255);
        assert_eq!(transparent, [200, 100, 50, 128]);
    }

    #[test]
    fn tiles_and_clips() {
        let mut atlas = Image::new(4, 1);
        atlas
            .data_mut()
            .copy_from_slice(&[1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255]);
        let mut canvas = Image::new(6, 1);
        blit_tiled(
            &mut canvas,
            &atlas,
            Rect::new(1, 0, 2, 1),
            Rect::new(0, 0, 6, 1),
            Rect::new(1, 0, 4, 1),
        );
        let reds: Vec<u8> = canvas
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[0])
            .collect();
        assert_eq!(reds, [0, 3, 2, 3, 2, 0]);
    }
}
