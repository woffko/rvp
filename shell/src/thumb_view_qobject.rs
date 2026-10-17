//! The `ThumbView` item's bridge; the implementation is in `thumb_view.rs`.
//!
//! This file holds only the bridge, so the `unsafe` the generated code needs
//! is allowed for the file (cxx-qt accepts no attributes on the bridge
//! module itself).

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). `paint` is the one
// `unsafe` entry: Qt calls it with a painter that is valid for the call
// (see `thumb_view.rs`).
#![allow(unsafe_code)]
// The code cxx-qt generates for the custom constructor boxes its result and
// takes its arguments by value.
#![allow(clippy::unnecessary_box_returns, clippy::needless_pass_by_value)]

pub use crate::thumb_view::ThumbViewRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qpainter.h");
        type QPainter = cxx_qt_lib::QPainter;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++" {
        include!(<QtQuick/QQuickPaintedItem>);
        type QQuickPaintedItem;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QQuickPaintedItem]
        #[qproperty(QString, key)]
        #[qproperty(i32, image_width)]
        #[qproperty(i32, image_height)]
        type ThumbView = super::ThumbViewRust;

        /// Paints the thumbnail (render thread, GUI thread blocked).
        #[qinvokable]
        #[cxx_override]
        unsafe fn paint(self: Pin<&mut Self>, painter: *mut QPainter);
    }

    // The generated default constructor passes a QObject* parent, which
    // QQuickPaintedItem does not accept; QML sets the parent item later.
    impl cxx_qt::Constructor<()> for ThumbView {}
}

impl cxx_qt::Constructor<()> for qobject::ThumbView {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_arguments: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new((): ()) -> ThumbViewRust {
        ThumbViewRust::default()
    }
}
