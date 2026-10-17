//! The second audio device (`rvp_app::audio::second`): a second core plays
//! the main core's file and follows its seeks, pause and speed, with the
//! delay set for the device. Headless libmpv, both cores on `ao=null`
//! (which keeps time like a device): no window, no sound.

use std::time::{Duration, Instant};

use rvp_app::{App, Settings};
use rvp_engine::{Engine, EngineOptions, PropertyId, Seek};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

/// Drains both cores for `ms` milliseconds, as the shell would.
fn run(app: &mut App, engine: &Engine, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
        app.drain(engine, Instant::now());
        app.tick_second(engine, Instant::now());
    }
}

/// Runs until `done` holds, at most `seconds`.
fn wait_for(
    app: &mut App,
    engine: &Engine,
    seconds: u64,
    what: &str,
    done: impl Fn(&App, &Engine) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while !done(app, engine) {
        assert!(Instant::now() < deadline, "{what}");
        run(app, engine, 50);
    }
}

/// The main core's and the second core's audio positions.
fn positions(app: &App, engine: &Engine) -> (f64, f64) {
    let second = app.second().expect("a second core").engine();
    (
        engine.get_double("audio-pts").unwrap_or(f64::NAN),
        second.get_double("audio-pts").unwrap_or(f64::NAN),
    )
}

#[test]
fn the_second_device_follows_the_main_core() {
    let dir = std::env::temp_dir().join(format!("rvp-second-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ..Settings::default()
    };
    let mut app = App::new(settings, &dir).expect("app");
    app.second_mpv_options = vec![("ao".to_owned(), "null".to_owned())];
    app.apply_audio_chain(&engine);
    let mut settings = app.settings.clone();
    settings.audio.second_device = "auto".to_owned();
    let applied = app.apply_settings(&engine, settings);
    assert!(applied.notes.is_empty(), "{:?}", applied.notes);
    assert_eq!(
        app.second().map(rvp_app::audio::second::Second::device),
        Some("auto")
    );

    // Its chain is the main core's, the normalizer's measurement left out.
    let chain = app
        .second()
        .and_then(|s| s.engine().get_text("af"))
        .unwrap_or_default();
    assert!(chain.contains("@rvpnorm") && chain.contains("volume@rvpgain"));
    assert!(!chain.contains("rvppeak"), "no measurement: {chain}");

    app.open(&engine, &[LONG.to_owned()], false).expect("open");
    wait_for(&mut app, &engine, 15, "the second core plays", |app, _| {
        app.second()
            .is_some_and(|s| s.playing() && s.offset().is_some())
    });
    run(&mut app, &engine, 2500);
    let (main, second) = positions(&app, &engine);
    assert!(
        (second - main).abs() < 0.06,
        "main {main:.3}, second {second:.3}"
    );

    // A seek: it follows.
    engine.seek(Seek::Absolute(30.0)).expect("seek");
    run(&mut app, &engine, 2500);
    let (main, second) = positions(&app, &engine);
    assert!(main > 30.0, "the main core seeked: {main:.3}");
    assert!(
        (second - main).abs() < 0.06,
        "main {main:.3}, second {second:.3}"
    );

    // Pause and speed are mirrored.
    engine.set_pause(true).expect("pause");
    run(&mut app, &engine, 500);
    let second = app.second().expect("second").engine();
    assert_eq!(second.get_text("pause").as_deref(), Some("yes"));
    engine.set_pause(false).expect("play");
    engine.set_double("speed", 1.5).expect("speed");
    run(&mut app, &engine, 2000);
    let second = app.second().expect("second").engine();
    let speed = second.get_double("speed").unwrap_or_default();
    assert!(
        (speed - 1.5).abs() < 0.04,
        "the second core's speed {speed}"
    );
    engine.set_double("speed", 1.0).expect("speed");

    // A delay for the device: it plays that much later.
    let mut settings = app.settings.clone();
    settings.audio.second_delay = 400.0;
    app.apply_settings(&engine, settings);
    wait_for(
        &mut app,
        &engine,
        10,
        "the delay is taken",
        |app, engine| {
            let (main, second) = positions(app, engine);
            (main - second - 0.4).abs() < 0.06 && app.model.flag(PropertyId::Pause) == Some(false)
        },
    );
    run(&mut app, &engine, 2000);
    let (main, second) = positions(&app, &engine);
    assert!(
        (main - second - 0.4).abs() < 0.06,
        "main {main:.3}, second {second:.3}"
    );

    // Off again.
    let mut settings = app.settings.clone();
    settings.audio.second_device = String::new();
    app.apply_settings(&engine, settings);
    assert!(app.second().is_none());
    engine.quit().expect("quit");
    let _ = std::fs::remove_dir_all(&dir);
}
