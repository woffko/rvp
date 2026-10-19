//! RVP, the player window (milestones M2 and M3).
//!
//! ```text
//! rvp [/add | /insert | /current | /new] [--skin=PATH] [--mpv-options=name=value,...]
//!     [--selftest] [--instance=NAME] [--out=DIR] [FILE|URL]...
//! ```
//!
//! A running RVP takes the files of a second one (single instance,
//! `rvp_app::instance`): the second one hands them over and exits.
//!
//! Shutdown order (goal D rule 6): closing the window stops playback and
//! waits for mpv to go idle (`Player::shutdown`); the scene graph then frees
//! the render context on the render thread; last, the core is asked to quit
//! and the process exits. The engine itself is never destroyed: the render
//! context borrows it for the life of the process.

// No console window on Windows; the self-test's output still reaches a
// pipe or file that the caller redirects to.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod awake;
mod crash;
mod desktop;
mod devices;
mod ffi;
mod gpu_power;
#[cfg(windows)]
mod media_controls;
mod metrics;
mod player;
mod player_qobject;
mod power;
mod recorder;
mod recycle;
mod skin_view;
mod skin_view_qobject;
mod system_volume;
mod thumb_view;
mod thumb_view_qobject;
mod video;

use std::time::{Duration, Instant};

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use rvp_app::Cli;
use rvp_app::instance::{self, Request};
use rvp_engine::EngineEvent;

fn main() {
    metrics::start();
    let cli = Cli::parse(std::env::args().skip(1));
    install_crash_logs(&cli);
    player::preload_skin();
    ffi::bridge::rvp_setup_graphics();
    let mut app = QGuiApplication::new();
    metrics::mark("gui_application_ms");
    // The window's app ID: the desktop finds rvp.desktop (its icon, MPRIS's
    // DesktopEntry) by it.
    QGuiApplication::set_desktop_file_name(&QString::from("rvp"));
    single_instance(&cli);
    ffi::bridge::rvp_set_c_numeric_locale();
    ffi::bridge::rvp_set_window_icon();
    ffi::bridge::rvp_register_native_types();
    let mut engine = QQmlApplicationEngine::new();
    metrics::mark("qml_engine_ms");
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/Rvp/qml/Main.qml"));
    }
    metrics::mark("qml_loaded_ms");
    if !ffi::bridge::rvp_has_windows() {
        eprintln!("rvp: the window could not be created (see the QML errors above)");
        std::process::exit(2);
    }
    let code = app.as_mut().map_or(1, |app| app.exec());
    if video::in_window() {
        // mpv's window is a child of RVP's video window: mpv lets it go
        // before the windows close.
        finish();
        ffi::bridge::rvp_video_window_close();
        drop(engine);
    } else {
        // Destroys the window; the video item's renderer frees the render
        // context on the render thread.
        drop(engine);
        finish();
    }
    drop(app);
    std::process::exit(code);
}

/// Crash logs in RVP's state folder (M8): panics, then fatal signals or
/// unhandled exceptions. `--crash-test=panic` or `=native` crashes on
/// purpose once they are in place, for the package checks.
fn install_crash_logs(cli: &Cli) {
    if let Ok(dirs) = rvp_app::Dirs::from_env() {
        rvp_app::crash::install_panic_hook(&dirs.state);
        crash::install(&dirs.state);
    }
    match cli.options.get("crash-test").map(String::as_str) {
        Some("panic") => panic!("crash test (--crash-test=panic)"),
        Some("native") => ffi::bridge::rvp_crash_test(),
        _ => {}
    }
}

/// Hands the command line to a running instance and exits, or becomes the
/// instance others hand theirs to.
fn single_instance(cli: &Cli) {
    let Some(channel) = instance::channel(cli) else {
        return;
    };
    // `/new` runs on its own and leaves the channel to the running one.
    if !instance::forwards(cli) {
        return;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let request = Request::from_cli(cli, &cwd);
    if ffi::bridge::rvp_instance_send(&channel, &request.encode()) {
        std::process::exit(0);
    }
    if !ffi::bridge::rvp_instance_listen(&channel) {
        eprintln!("rvp: cannot listen for other instances on {channel}");
    }
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
