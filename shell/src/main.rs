//! RVP, the player window (milestone M2).
//!
//! ```text
//! rvp [--skin=PATH] [--mpv-options=name=value,...] [--selftest] [--out=DIR] [FILE|URL]...
//! ```
//!
//! Shutdown order (goal D rule 6): closing the window stops playback and
//! waits for mpv to go idle (`Player::shutdown`); the scene graph then frees
//! the render context on the render thread; last, the core is asked to quit
//! and the process exits. The engine itself is never destroyed: the render
//! context borrows it for the life of the process.

mod ffi;
mod metrics;
mod player;
mod player_qobject;
mod skin_view;
mod skin_view_qobject;
mod video;

use std::time::{Duration, Instant};

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use rvp_engine::EngineEvent;

fn main() {
    metrics::start();
    ffi::bridge::rvp_setup_graphics();
    let mut app = QGuiApplication::new();
    ffi::bridge::rvp_set_c_numeric_locale();
    ffi::bridge::rvp_register_native_types();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/Rvp/qml/Main.qml"));
    }
    if !ffi::bridge::rvp_has_windows() {
        eprintln!("rvp: the window could not be created (see the QML errors above)");
        std::process::exit(2);
    }
    let code = app.as_mut().map_or(1, |app| app.exec());
    // Destroys the window; the video item's renderer frees the render
    // context on the render thread.
    drop(engine);
    finish();
    drop(app);
    std::process::exit(code);
}

/// Lets the mpv core shut down after playback stopped.
fn finish() {
    let Some(engine) = video::ENGINE.get() else {
        return;
    };
    if engine.quit().is_err() {
        return;
    }
    let mut model = rvp_engine::PlayerModel::new();
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !events.contains(&EngineEvent::Shutdown) {
        engine.wait_and_drain(0.1, &mut model, &mut events);
    }
}
