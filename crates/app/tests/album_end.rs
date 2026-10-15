//! The album moves on at the end of a file even when the audio chain or
//! the speed changes right then (goal H, found by the Windows VM's
//! self-test): a resync seek that reached mpv after the end but before RVP
//! saw it left mpv's `eof-reached` false for good, and playback stopped on
//! the last frame. And it moves on by one entry even when RVP looks again
//! before mpv reports the next file (goal I, also found there): the old
//! file's end, still in the model, moved it on a second time. Headless
//! libmpv, no window, no sound.

use std::time::{Duration, Instant};

use rvp_app::{Action, App, Settings, Speed};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);
const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);
const MARKER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-marker.mkv"
);

fn pump(app: &mut App, engine: &Engine, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        app.drain(engine, Instant::now());
    }
}

fn playing_long(app: &App) -> bool {
    app.model
        .loaded_path()
        .is_some_and(|path| path.ends_with("fixture-60s.mkv"))
}

/// Plays the short fixture before the long one; once it is past
/// `seconds`, mpv plays on unwatched for a moment (it reaches the end),
/// then `action` runs before RVP sees what happened. `chain` checks mpv's
/// audio filters once the next file plays: a change made at the end waits
/// for it.
fn change_at_the_end(name: &str, seconds: f64, action: Action, chain: impl Fn(&str) -> bool) {
    let dir = std::env::temp_dir().join(format!("rvp-end-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        ..Settings::default()
    };
    assert!(settings.normalizer.enabled);
    let mut app = App::new(settings, &dir).expect("app");
    app.start(&engine, &[SHORT.to_owned(), LONG.to_owned()])
        .expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < seconds || playing_long(&app) {
        assert!(Instant::now() < deadline, "{name}: the short fixture plays");
        pump(&mut app, &engine, 5);
    }
    std::thread::sleep(Duration::from_millis(400));
    app.dispatch(&engine, action);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !playing_long(&app) {
        assert!(
            Instant::now() < deadline,
            "{name}: stopped at {:?} of {:?}",
            app.model.double(PropertyId::TimePos),
            app.model.loaded_path()
        );
        pump(&mut app, &engine, 20);
    }
    pump(&mut app, &engine, 300);
    let filters = engine.get_text("af").unwrap_or_default();
    assert!(chain(&filters), "{name}: audio filters {filters:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_album_moves_on_when_the_normalizer_toggles_at_the_end() {
    change_at_the_end("normalizer", 2.7, Action::ToggleNormalizer, |af| {
        !af.contains("@rvpnorm:") && !af.contains("@rvptempo:")
    });
}

#[test]
fn the_album_moves_on_when_the_equalizer_toggles_at_the_end() {
    change_at_the_end("equalizer", 2.7, Action::ToggleEqualizer, |af| {
        af.starts_with("@rvpeq:") && af.contains("@rvpnorm:")
    });
}

#[test]
fn the_album_moves_on_when_the_speed_changes_at_the_end() {
    change_at_the_end("speed", 2.7, Action::Speed(Speed::Faster), |af| {
        af.contains("@rvpnorm:")
    });
}

#[test]
fn the_album_moves_on_by_one_entry_when_rvp_looks_before_the_load() {
    let dir = std::env::temp_dir().join(format!("rvp-end-one-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        ..Settings::default()
    };
    let mut app = App::new(settings, &dir).expect("app");
    app.start(
        &engine,
        &[SHORT.to_owned(), LONG.to_owned(), MARKER.to_owned()],
    )
    .expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 2.5 || playing_long(&app) {
        assert!(Instant::now() < deadline, "the short fixture plays");
        pump(&mut app, &engine, 5);
    }
    // Without a pause between looks, RVP sees the model again before mpv
    // reports that the next file started.
    let end = Instant::now() + Duration::from_millis(2500);
    while Instant::now() < end {
        app.drain(&engine, Instant::now());
    }
    pump(&mut app, &engine, 300);
    assert!(
        playing_long(&app),
        "skipped to {:?}",
        app.model.loaded_path()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
