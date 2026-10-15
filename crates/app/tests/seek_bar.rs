//! The seek bar while the mouse holds it (owner, 2026-10-15): it shows the
//! place it is held at once, not where the video is, and once let go it
//! stays there until the video gets there. Headless libmpv, no window, no
//! sound.

use std::time::{Duration, Instant};

use rvp_app::{App, Settings, view};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

/// The seek bar's position and the current time as the skin would show
/// them.
fn shown(app: &App) -> (f32, String) {
    let mut data = view::data_model(&app.model, app.library().lists());
    app.seek_bar_data(&mut data);
    (
        data.sliders["CMD_TIMEBAR_CTRL"].value,
        data.texts["CMD_CURRENTTIME_MESSAGE"].clone(),
    )
}

fn drain_until(app: &mut App, engine: &Engine, what: &str, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(app) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
        app.drain(engine, Instant::now());
    }
}

#[test]
fn the_seek_bar_shows_where_it_is_held() {
    let dir = std::env::temp_dir().join(format!("rvp-seek-bar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ..Settings::default()
    };
    let mut app = App::new(settings, &dir).expect("app");
    app.open(&engine, &[LONG.to_owned()], false).expect("open");
    drain_until(&mut app, &engine, "the fixture plays", |app| {
        app.model.double(PropertyId::TimePos).unwrap_or(0.0) > 0.5
            && app.model.double(PropertyId::Duration).is_some()
    });
    let duration = app.model.double(PropertyId::Duration).expect("duration");

    // Pressed and dragged: the bar is where the mouse is, before mpv seeks.
    app.drag_seek_bar(&engine, 0.5);
    let (bar, time) = shown(&app);
    assert!((bar - 0.5).abs() < 1e-6, "bar {bar}");
    assert_eq!(time, view::format_time(0.5 * duration));
    app.drag_seek_bar(&engine, 0.75);
    assert!((shown(&app).0 - 0.75).abs() < 1e-6);
    // The video's own position does not move the held bar.
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(10));
        app.drain(&engine, Instant::now());
        assert!((shown(&app).0 - 0.75).abs() < 1e-6, "{:?}", shown(&app));
    }

    // Let go: it stays there until the video gets there, then follows
    // playback again.
    app.release_seek_bar(&engine, 0.75, Instant::now());
    assert!((shown(&app).0 - 0.75).abs() < 1e-6);
    drain_until(&mut app, &engine, "the video at the release point", |app| {
        app.model
            .double(PropertyId::TimePos)
            .is_some_and(|time| (time - 0.75 * duration).abs() < 1.0)
    });
    drain_until(&mut app, &engine, "playback moves the bar again", |app| {
        let time = app.model.double(PropertyId::TimePos).unwrap_or(0.0);
        // Single-precision bar, as the skin stores it.
        #[allow(clippy::cast_possible_truncation)]
        let playing = (time / duration) as f32;
        (shown(app).0 - playing).abs() < 1e-6
    });
    let _ = std::fs::remove_dir_all(&dir);
}
