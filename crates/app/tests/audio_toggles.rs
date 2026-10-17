//! M10's effects switched while a file plays and at its very end, as the
//! Audio menu does it (`Action::Audio`): each switch puts its labelled
//! filter in mpv's chain or takes it out and playback goes on; in the last
//! moments of a file the change waits for the next one and the album moves
//! on (the end-of-file rule, `App::defer_chain_change`). Headless libmpv,
//! no window, no sound.

use std::time::{Duration, Instant};

use rvp_app::audio::StereoChannel;
use rvp_app::audio::commands::AudioCommand;
use rvp_app::{Action, App, Settings};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);
const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);

fn pump(app: &mut App, engine: &Engine, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        app.drain(engine, Instant::now());
    }
}

fn settings() -> Settings {
    Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        ..Settings::default()
    }
}

/// Each effect's switch and the label it puts in the chain.
const EFFECTS: [(AudioCommand, &str); 7] = [
    (AudioCommand::Crystality, "@rvpcrystality:"),
    (AudioCommand::NoiseReduction, "@rvpdenoise:"),
    (AudioCommand::Freeverb, "@rvpreverb:"),
    (AudioCommand::VoiceRemoval, "@rvpvoice:"),
    (AudioCommand::Crossfeed, "@rvpcrossfeed:"),
    (AudioCommand::SwapChannels, "@rvpchannel:"),
    (AudioCommand::ChannelVolumes, "@rvpgains:"),
];

#[test]
fn effects_switch_while_a_file_plays() {
    let dir = std::env::temp_dir().join(format!("rvp-toggles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut app = App::new(settings(), &dir).expect("app");
    app.start(&engine, &[LONG.to_owned()]).expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 1.0 {
        assert!(Instant::now() < deadline, "the fixture plays");
        pump(&mut app, &engine, 20);
    }
    for on in [true, false] {
        for (command, label) in EFFECTS {
            let before = app.model.double(PropertyId::TimePos).unwrap_or(0.0);
            let switched = Instant::now();
            app.dispatch(&engine, Action::Audio(command));
            // Playback goes on: a busy machine may take a while.
            let deadline = switched + Duration::from_secs(5);
            while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < before + 0.1 {
                assert!(
                    Instant::now() < deadline,
                    "{command:?}: stopped at {before:.2}"
                );
                pump(&mut app, &engine, 20);
            }
            let af = engine.get_text("af").unwrap_or_default();
            assert_eq!(af.contains(label), on, "{command:?} on={on}: {af}");
            // The normalizer stays in its place, after the effects.
            let norm = af.find("@rvpnorm:").expect("the normalizer");
            if on {
                assert!(
                    af.find(label).is_some_and(|at| at < norm) || label == "@rvpgains:",
                    "{label} before the normalizer: {af}"
                );
            }
            // On from where it was: no seek elsewhere.
            let after = app.model.double(PropertyId::TimePos).unwrap_or(0.0);
            let elapsed = switched.elapsed().as_secs_f64();
            assert!(
                after < before + elapsed + 0.5,
                "{command:?}: from {before:.2} to {after:.2} in {elapsed:.2} s"
            );
            assert_eq!(app.model.flag(PropertyId::Pause), Some(false));
        }
    }
    // Audio Stretching: rubberband in the tempo filter's place, its pitch
    // changed while it plays, then the tempo filter back.
    for (command, rubberband) in [
        (AudioCommand::StretchingUp, true),
        (AudioCommand::StretchingUp, true),
        (AudioCommand::StretchingOff, false),
    ] {
        let before = app.model.double(PropertyId::TimePos).unwrap_or(0.0);
        let switched = Instant::now();
        app.dispatch(&engine, Action::Audio(command));
        let deadline = switched + Duration::from_secs(5);
        while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < before + 0.1 {
            assert!(
                Instant::now() < deadline,
                "{command:?}: stopped at {before:.2}"
            );
            pump(&mut app, &engine, 20);
        }
        let af = engine.get_text("af").unwrap_or_default();
        assert_eq!(
            af.contains("@rvptempo:rubberband"),
            rubberband,
            "{command:?}: {af}"
        );
        let after = app.model.double(PropertyId::TimePos).unwrap_or(0.0);
        let elapsed = switched.elapsed().as_secs_f64();
        assert!(
            after < before + elapsed + 0.5,
            "{command:?}: from {before:.2} to {after:.2} in {elapsed:.2} s"
        );
    }
    // The left channel on both sides, then both again.
    app.dispatch(
        &engine,
        Action::Audio(AudioCommand::Channel(StereoChannel::Left)),
    );
    pump(&mut app, &engine, 300);
    let af = engine.get_text("af").unwrap_or_default();
    assert!(af.contains("@rvpchannel:"), "{af}");
    app.dispatch(
        &engine,
        Action::Audio(AudioCommand::Channel(StereoChannel::Both)),
    );
    pump(&mut app, &engine, 300);
    let af = engine.get_text("af").unwrap_or_default();
    assert!(!af.contains("@rvpchannel:"), "{af}");
    engine.quit().expect("quit");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_effect_switched_at_the_end_waits_for_the_next_file() {
    let dir = std::env::temp_dir().join(format!("rvp-toggles-end-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut app = App::new(settings(), &dir).expect("app");
    app.start(&engine, &[SHORT.to_owned(), LONG.to_owned()])
        .expect("start");
    let playing_long = |app: &App| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-60s.mkv"))
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 2.7 || playing_long(&app) {
        assert!(Instant::now() < deadline, "the short fixture plays");
        pump(&mut app, &engine, 5);
    }
    // mpv reaches the end meanwhile; RVP switches before it sees that.
    std::thread::sleep(Duration::from_millis(400));
    app.dispatch(&engine, Action::Audio(AudioCommand::Crystality));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !playing_long(&app) {
        assert!(
            Instant::now() < deadline,
            "stopped at {:?} of {:?}",
            app.model.double(PropertyId::TimePos),
            app.model.loaded_path()
        );
        pump(&mut app, &engine, 20);
    }
    pump(&mut app, &engine, 300);
    let af = engine.get_text("af").unwrap_or_default();
    assert!(af.contains("@rvpcrystality:"), "the next file has it: {af}");
    engine.quit().expect("quit");
    let _ = std::fs::remove_dir_all(&dir);
}
