//! `ThumbView`: a `QQuickPaintedItem` that paints one picture of the
//! thumbnail core (M11) from the atlas cache, where the window put it under
//! `key` (`thumb/preview`, `thumb/scene/N`): the seek bar's preview and the
//! scene browsers' pictures. QML calls `update()` when a new picture of the
//! key arrives.

use std::pin::Pin;

use cxx_qt_lib::QString;

use crate::ffi::bridge::rvp_paint_atlas;
use crate::thumb_view_qobject::qobject;

/// The Rust state of [`qobject::ThumbView`].
#[derive(Default)]
pub struct ThumbViewRust {
    pub(crate) key: QString,
    pub(crate) image_width: i32,
    pub(crate) image_height: i32,
}

// SAFETY: the one FFI point of the painter: Qt hands `paint` a raw
// QPainter pointer (owner decision 2026-10-10 allows `unsafe` here).
#[allow(unsafe_code)]
impl qobject::ThumbView {
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
        let (width, height) = (*self.image_width(), *self.image_height());
        let key = self.key().to_string();
        if key.is_empty() || width <= 0 || height <= 0 {
            return;
        }
        let area = [0, 0, width, height];
        rvp_paint_atlas(painter, &key, area, area, area);
    }
}
