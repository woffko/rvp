//! `SkinView`: a `QQuickPaintedItem` that paints the chrome's display list
//! (D5). In the main window (`surface` 0) with `region_index` -1 it is the
//! static layer and covers the window; otherwise it covers one dynamic
//! region and paints only what touches it (`rvp_app::chrome`). The control
//! panel's window (`surface` 1) and the skin's logo (`surface` 2, M14) are
//! small and paint everything at once.

use std::pin::Pin;

use rvp_app::chrome::Frame;
use rvp_skin::Skin;
use rvp_skin::paint::{HAlign, Op, VAlign};

use crate::ffi::bridge::{
    rvp_atlas_contains, rvp_atlas_insert, rvp_paint_atlas, rvp_paint_fill, rvp_paint_scale,
    rvp_paint_text, rvp_paint_translate,
};
use crate::skin_view_qobject::qobject;

/// The Rust state of [`qobject::SkinView`].
pub struct SkinViewRust {
    pub(crate) region_index: i32,
    pub(crate) surface: i32,
}

fn rect(r: rvp_skin::Rect) -> [i32; 4] {
    [r.x, r.y, r.w, r.h]
}

fn rgba(color: [u8; 4]) -> u32 {
    u32::from_be_bytes(color)
}

/// The C++ image cache's key of a skin's atlas: under the skin's id, so
/// another skin's or theme's image of the same name never shows (M14).
fn atlas_key(skin: &Skin, name: &str) -> String {
    format!("skin/{}/{name}", skin.id())
}

/// Copies an atlas to the C++ image cache on first use.
fn ensure_atlas(skin: &Skin, key: &str, name: &str) -> bool {
    if rvp_atlas_contains(key) {
        return true;
    }
    let Some(image) = skin.image(name) else {
        return false;
    };
    let (Ok(width), Ok(height)) = (i32::try_from(image.width()), i32::try_from(image.height()))
    else {
        return false;
    };
    rvp_atlas_insert(key, image.data(), width, height);
    true
}

fn draw(mut painter: Pin<&mut qobject::QPainter>, skin: &Skin, op: &Op) {
    match op {
        Op::Image {
            atlas,
            src,
            dst,
            clip,
        } => {
            let key = atlas_key(skin, atlas);
            if ensure_atlas(skin, &key, atlas) {
                rvp_paint_atlas(painter.as_mut(), &key, rect(*src), rect(*dst), rect(*clip));
            }
        }
        Op::Fill { rect: area, color } => {
            rvp_paint_fill(painter.as_mut(), rect(*area), rgba(*color));
        }
        Op::Text(text) => {
            // Font sizes are a few pixels.
            #[allow(clippy::cast_possible_truncation)]
            let pixel_size = text.font.size_px.round() as i32;
            let horizontal = match text.h_align {
                HAlign::Left => 0,
                HAlign::Center => 1,
                HAlign::Right => 2,
            };
            let vertical = match text.v_align {
                VAlign::Top => 0,
                VAlign::Center => 1,
                VAlign::Bottom => 2,
            };
            rvp_paint_text(
                painter.as_mut(),
                &text.text,
                text.trailing.as_deref().unwrap_or(""),
                &text.font.family,
                pixel_size,
                text.font.bold,
                rgba(text.color),
                rect(text.rect),
                rect(text.clip),
                horizontal + 3 * vertical,
                text.ellipsis,
            );
        }
    }
}

/// Paints the static layer (`None`) or one region of `frame`.
fn paint_frame(mut painter: Pin<&mut qobject::QPainter>, frame: &Frame, region: Option<usize>) {
    let Some(skin) = crate::player::skin() else {
        return;
    };
    let zoom = crate::player::zoom();
    if (zoom - 1.0).abs() > f64::EPSILON {
        rvp_paint_scale(painter.as_mut(), zoom);
    }
    if let Some(index) = region {
        let Some(area) = frame.regions.get(index) else {
            return;
        };
        rvp_paint_translate(painter.as_mut(), -area.x, -area.y);
    }
    for op in frame.ops_for(region) {
        draw(painter.as_mut(), skin, op);
    }
}

/// Paints every operation of `frame` (the control panel, the logo) at
/// `zoom`.
fn paint_all(mut painter: Pin<&mut qobject::QPainter>, frame: &Frame, zoom: f64) {
    let Some(skin) = crate::player::skin() else {
        return;
    };
    if (zoom - 1.0).abs() > f64::EPSILON {
        rvp_paint_scale(painter.as_mut(), zoom);
    }
    for op in &frame.list.ops {
        draw(painter.as_mut(), skin, op);
    }
}

// SAFETY: the one FFI point of the painter: Qt hands `paint` a raw
// QPainter pointer (owner decision 2026-10-10 allows `unsafe` here).
#[allow(unsafe_code)]
impl qobject::SkinView {
    /// # Safety
    /// `painter` comes from `QQuickPaintedItem::paint` and is valid for the
    /// duration of the call.
    pub(crate) unsafe fn paint(self: Pin<&mut Self>, painter: *mut qobject::QPainter) {
        // SAFETY: Qt passes a valid, exclusively borrowed painter for the
        // duration of paint(); a null pointer is rejected by as_mut.
        let Some(painter) = (unsafe { painter.as_mut() }) else {
            return;
        };
        // SAFETY: the QPainter is a C++ object that Rust never moves; the
        // pin lasts only for this call.
        let painter = unsafe { Pin::new_unchecked(painter) };
        if *self.surface() == 1 {
            if let Some(frame) = crate::player::panel_frame() {
                paint_all(painter, &frame, crate::player::panel_zoom());
            }
            return;
        }
        if *self.surface() == 2 {
            if let Some(frame) = crate::player::logo_frame() {
                paint_all(painter, &frame, 1.0);
            }
            return;
        }
        let Some(frame) = crate::player::frame() else {
            return;
        };
        let region = usize::try_from(*self.region_index()).ok();
        paint_frame(painter, &frame, region);
    }
}
