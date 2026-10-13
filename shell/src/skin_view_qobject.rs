//! The `SkinView` item's bridge; the implementation is in `skin_view.rs`.
//!
//! This file holds only the bridge, so the `unsafe` the generated code needs
//! is allowed for the file (cxx-qt accepts no attributes on the bridge
//! module itself).

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). `paint` is the one
// `unsafe` entry: Qt calls it with a painter that is valid for the call
// (see `skin_view.rs`).
#![allow(unsafe_code)]
// The code cxx-qt generates for the custom constructor boxes its result and
// takes its arguments by value.
#![allow(clippy::unnecessary_box_returns, clippy::needless_pass_by_value)]

pub use crate::skin_view::SkinViewRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qpainter.h");
        type QPainter = cxx_qt_lib::QPainter;
    }

    unsafe extern "C++" {
        include!(<QtQuick/QQuickPaintedItem>);
        type QQuickPaintedItem;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QQuickPaintedItem]
        #[qproperty(i32, region_index)]
        #[qproperty(i32, surface)]
        type SkinView = super::SkinViewRust;

        /// Paints the current frame (render thread, GUI thread blocked).
        #[qinvokable]
        #[cxx_override]
        unsafe fn paint(self: Pin<&mut Self>, painter: *mut QPainter);
    }

    // The generated default constructor passes a QObject* parent, which
    // QQuickPaintedItem does not accept; QML sets the parent item later.
    impl cxx_qt::Constructor<()> for SkinView {}
}

impl cxx_qt::Constructor<()> for qobject::SkinView {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_arguments: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new((): ()) -> SkinViewRust {
        SkinViewRust {
            region_index: -1,
            surface: 0,
        }
    }
}
