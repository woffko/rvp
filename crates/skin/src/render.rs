//! A pure-Rust rasterizer for display lists, used for tests and previews.

use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::image::Image;
use crate::limits;
use crate::paint::{DisplayList, HAlign, Op, TextOp, VAlign};
use crate::skin::Skin;

/// The canvas of a `width` x `height` window: the window's size, cut
/// down to [`limits::PICTURE_SIDE`] a side and to the rows that
/// [`limits::CANVAS_PIXELS`] hold.
fn canvas_size(width: i32, height: i32) -> (u32, u32) {
    let side = |size: i32| {
        u32::try_from(size.max(0))
            .unwrap_or(0)
            .min(limits::PICTURE_SIDE)
    };
    let (width, height) = (side(width), side(height));
    let rows = limits::CANVAS_PIXELS / u64::from(width.max(1));
    (width, height.min(u32::try_from(rows).unwrap_or(u32::MAX)))
}

/// Draws a display list into a new transparent image: the list's window,
/// or as much of it from its top-left corner as a canvas may hold
/// ([`limits::CANVAS_PIXELS`]).
#[must_use]
pub fn rasterize(list: &DisplayList, skin: &Skin, fonts: &mut Fonts) -> Image {
    let (width, height) = canvas_size(list.width, list.height);
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
/// count. A shape of more than [`limits::REGION_RECTS`] rectangles is
/// none (empty, as for a window drawn nowhere: the window stays a
/// rectangle).
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
        // The runs and the rectangles still open from the row above both
        // lie from left to right: one pass pairs them.
        let mut next = Vec::with_capacity(runs.len());
        let mut above = open.into_iter().peekable();
        for (start, length) in runs {
            while let Some(rect) = above.next_if(|rect| rect.0 < start) {
                done.push(rect);
            }
            match above.next_if(|rect| rect.0 == start && rect.2 == length) {
                Some(mut rect) => {
                    rect.3 += 1;
                    next.push(rect);
                }
                None => next.push((start, y, length, 1)),
            }
        }
        done.extend(above);
        open = next;
        if done.len() + open.len() > limits::REGION_RECTS {
            return String::new();
        }
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
    use super::{blend, blit_tiled, canvas_size, opaque_region};
    use crate::archive::SkinArchive;
    use crate::geometry::Rect;
    use crate::image::Image;
    use crate::limits;
    use crate::paint::{DisplayList, Op};
    use crate::skin::Skin;

    #[test]
    fn a_canvas_is_no_larger_than_its_limits() {
        assert_eq!(canvas_size(1920, 1080), (1920, 1080));
        assert_eq!(canvas_size(7680, 4320), (7680, 4320));
        assert_eq!(canvas_size(-5, 10), (0, 10));
        // A window of two thousand million pixels a side: the widest
        // canvas, and the rows that its pixels' limit holds.
        let (width, height) = canvas_size(i32::MAX, i32::MAX);
        assert_eq!(width, limits::PICTURE_SIDE);
        assert_eq!(u64::from(width) * u64::from(height), limits::CANVAS_PIXELS);
        assert_eq!(canvas_size(10_000, 10_000), (10_000, 6710));
    }

    /// A skin whose `c.png` is a checkerboard of single pixels.
    fn checker_skin() -> Skin {
        let mut checker = Image::new(2, 2);
        checker.data_mut()[..4].copy_from_slice(&[255; 4]);
        checker.data_mut()[12..].copy_from_slice(&[255; 4]);
        let files = [
            ("VideoSkin.xml", b"<Skin/>".to_vec()),
            ("c.png", checker.encode_png().unwrap()),
        ];
        Skin::load(SkinArchive::from_files("test", files), "VideoSkin.xml").unwrap()
    }

    fn fill(x: i32, y: i32, w: i32, h: i32) -> Op {
        Op::Fill {
            rect: Rect::new(x, y, w, h),
            color: [9, 9, 9, 255],
        }
    }

    #[test]
    fn a_shape_joins_the_equal_runs_of_its_rows() {
        let skin = checker_skin();
        let region = |ops: Vec<Op>| {
            let list = DisplayList {
                width: 6,
                height: 4,
                ops,
            };
            opaque_region(&list, &skin)
        };
        assert_eq!(region(Vec::new()), "");
        // An L: two rows of two pixels, two of six.
        assert_eq!(
            region(vec![fill(0, 0, 2, 4), fill(2, 2, 4, 2)]),
            "0,0,2,2;0,2,6,2"
        );
        // Two columns that last while a pixel between them comes and goes,
        // and one that starts where another ended.
        assert_eq!(
            region(vec![fill(0, 0, 1, 3), fill(4, 0, 1, 3), fill(2, 1, 1, 1)]),
            "0,0,1,3;2,1,1,1;4,0,1,3"
        );
        assert_eq!(
            region(vec![fill(0, 0, 3, 1), fill(1, 1, 3, 1), fill(0, 2, 3, 2)]),
            "0,0,3,1;0,2,3,2;1,1,3,1"
        );
    }

    #[test]
    fn a_shape_of_too_many_rectangles_is_none() {
        let skin = checker_skin();
        // A checkerboard of single pixels: no run joins the one above it.
        let region = |width: i32, height: i32| {
            let window = Rect::new(0, 0, width, height);
            let list = DisplayList {
                width,
                height,
                ops: vec![Op::Image {
                    atlas: "c.png".to_owned(),
                    src: Rect::new(0, 0, 2, 2),
                    dst: window,
                    clip: window,
                }],
            };
            opaque_region(&list, &skin)
        };
        let small = region(100, 100);
        assert_eq!(small.split(';').count(), 5000);
        assert!(small.starts_with("0,0,1,1;0,2,1,1;"), "{}", &small[..40]);
        // A larger one's rectangles are more than a shape may have.
        let over = i32::try_from(limits::REGION_RECTS / 64 + 1).unwrap();
        assert_eq!(region(128, over), "");
        assert_eq!(region(4096, 4096), "");
    }

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
