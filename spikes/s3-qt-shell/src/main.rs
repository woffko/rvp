//! Spike S3: a frameless cxx-qt/QML window with a skin painted from Rust,
//! an mpv OpenGL video item and a context menu larger than the window.
//!
//! ```text
//! s3-qt-shell [--skin BlackSpace_en.dsf] [--video URL] [--hwdec no]
//!             [--selftest] [--out DIR]
//! ```

mod ffi;
mod mpv;
mod skin_item;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    ffi::bridge::rvp_setup_graphics();
    let mut app = QGuiApplication::new();
    ffi::bridge::rvp_register_native_types();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/RvpSpike/qml/Main.qml"));
    }
    let code = app.as_mut().map_or(1, |app| app.exec());
    std::process::exit(code);
}
