//! Speed changes with the normalizer on (goal H, carry-over from the
//! Ubuntu boot run). With only the normalizer in the chain, the return to
//! 1x left mpv's A/V offset (`avsync`) above 0.6 s at some moments of a
//! file (one second in, reliably), which a real video output corrects by
//! dropping frames (14 on the Ubuntu boot). RVP now keeps a tempo filter of
//! its own after the normalizer (`rvp_app::normalizer::TEMPO_ENTRY`), so
//! the offset stays at a few hundredths of a second without a seek.
//! Headless libmpv, no window, no sound.

use std::time::{Duration, Instant};

use rvp_app::{Action, App, Settings, Speed};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

/// Lets mpv play for `ms` milliseconds and returns the largest A/V offset
/// it reported meanwhile.
fn play(app: &mut App, engine: &Engine, ms: u64) -> f64 {
    let end = Instant::now() + Duration::from_millis(ms);
    let mut largest: f64 = 0.0;
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        app.drain(engine, Instant::now());
        if let Some(offset) = engine.get_double("avsync") {
            largest = largest.max(offset.abs());
        }
    }
    largest
}

#[test]
fn speed_changes_keep_audio_and_video_together_with_the_normalizer() {
    let dir = std::env::temp_dir().join(format!("rvp-speed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ..Settings::default()
    };
    assert!(settings.normalizer.enabled);
    let mut app = App::new(settings, &dir).expect("app");
    app.apply_normalizer(&engine);
    app.open(&engine, &[LONG.to_owned()], false).expect("open");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 1.0 {
        assert!(Instant::now() < deadline, "the fixture plays");
        play(&mut app, &engine, 20);
    }
    // With the normalizer alone the return to 1x reached 0.60-0.61 s here.
    for change in [Speed::Faster, Speed::Default, Speed::Slower, Speed::Default] {
        app.dispatch(&engine, Action::Speed(change));
        let largest = play(&mut app, &engine, 1500);
        assert!(
            largest < 0.2,
            "after {change:?}: A/V offset up to {largest:.3} s"
        );
    }
    let speed = app.model.double(PropertyId::Speed).unwrap_or_default();
    assert!((speed - 1.0).abs() < 1e-6, "speed {speed}");
    // No seek: the clock ran on through the four changes (6 s of wall
    // time, 0.1x faster and slower for 1.5 s each).
    let time = app.model.double(PropertyId::TimePos).unwrap_or_default();
    assert!(time > 6.5, "played on without seeks: {time}");
    let _ = std::fs::remove_dir_all(&dir);
}
