//! Speed changes with the normalizer on (goal H, carry-over from the
//! Ubuntu boot run). The normalizer holds about ten seconds of audio, and
//! played at a new speed it put audio and video out of step: after the
//! speed went back to 1x, mpv's A/V offset (`avsync`) stayed above 0.6 s,
//! which a real video output corrects by dropping frames (14 on the Ubuntu
//! boot). RVP now plays on from the current position when the speed
//! changes, so the offset stays at the usual few hundredths of a second.
//! (mpv may still log its one-time desync warning at the instant of the
//! change, before the seek runs; the offset is what matters.) Headless
//! libmpv, no window, no sound.

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
    // Without the resync the second change (back to 1x) reached 0.61 s.
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
    let _ = std::fs::remove_dir_all(&dir);
}
