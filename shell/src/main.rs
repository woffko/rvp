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
mod bluray_ffi;
mod cpu;
mod crash;
mod desktop;
mod devices;
mod disc;
mod disc_ffi;
mod ffi;
mod gpu_power;
mod hardware;
#[cfg(windows)]
mod media_controls;
mod metrics;
mod player;
mod player_qobject;
mod power;
mod process;
mod recorder;
mod recycle;
mod screensaver;
mod skin_view;
mod skin_view_qobject;
mod startup;
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
    // `rvp.scr`: Windows' screen saver starts RVP and waits (M14).
    if let Some(code) = screensaver::launch() {
        std::process::exit(code);
    }
    screensaver::separate_state();
    process::restrict_library_search();
    metrics::start();
    let cli = Cli::from_env();
    install_crash_logs(&cli);
    let settings = player::preload_skin();
    ffi::bridge::rvp_setup_graphics();
    let mut app = QGuiApplication::new();
    metrics::mark("gui_application_ms");
    // The window's app ID: the desktop finds rvp.desktop (its icon, MPRIS's
    // DesktopEntry) by it.
    QGuiApplication::set_desktop_file_name(&QString::from("rvp"));
    single_instance(&cli, &settings);
    ffi::bridge::rvp_set_c_numeric_locale();
    ffi::bridge::rvp_set_window_icon();
    // Windows' counterpart of `FC_LANG` (`player::font_language`): from
    // the next start too.
    ffi::bridge::rvp_language_fonts(&settings.general.language);
    ffi::bridge::rvp_register_native_types();
    let mut engine = QQmlApplicationEngine::new();
    metrics::mark("qml_engine_ms");
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/Rvp/qml/Main.qml"));
    }
    metrics::mark("qml_loaded_ms");
    if !ffi::bridge::rvp_has_windows() {
        rvp_app::log::report(
            rvp_app::log::Level::Critical,
            "the window could not be created (see the QML errors above)",
        );
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
    process::leave(code);
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
/// instance others hand theirs to. With the General page's "Multiple
/// instances" each start runs on its own (M14).
fn single_instance(cli: &Cli, settings: &rvp_app::Settings) {
    let Some(channel) = instance::channel(cli) else {
        return;
    };
    if settings.general.instances == "multiple" && instance::forwards(cli) {
        player::OTHER_INSTANCE.store(
            ffi::bridge::rvp_instance_running(&channel),
            std::sync::atomic::Ordering::Relaxed,
        );
        // The first one keeps the channel; the others run on their own.
        let _ = ffi::bridge::rvp_instance_listen(&channel);
        return;
    }
    // `/new` runs on its own and leaves the channel to the running one.
    if !instance::forwards(cli) {
        return;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let request = Request::from_cli(cli, &cwd);
    if ffi::bridge::rvp_instance_send(&channel, &request.encode()) {
        // This process ends here: the arguments it could not hand over
        // are named on its standard error (a first RVP shows them).
        for notice in cli.notices() {
            rvp_app::log::report(rvp_app::log::Level::Warning, &notice);
        }
        std::process::exit(0);
    }
    if !ffi::bridge::rvp_instance_listen(&channel) {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("cannot listen for other instances on {channel}"),
        );
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
