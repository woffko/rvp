//! The album moves on at the end of a file even when the audio chain or
//! the speed changes right then (goal H, found by the Windows VM's
//! self-test): a resync seek that reached mpv after the end but before RVP
//! saw it left mpv's `eof-reached` false for good, and playback stopped on
//! the last frame. And it moves on by one entry even when RVP looks again
//! before mpv reports the next file (goal I, also found there): the old
//! file's end, still in the model, moved it on a second time. Video
//! filters and the decoder wait at the end the same way, and reach mpv
//! when the next file starts, also on a clip shorter than the wait.
//! Headless libmpv, no window, no sound.

use std::time::{Duration, Instant};

use rvp_app::video::VideoEffect;
use rvp_app::{Action, App, Repeat, Settings, Speed};
use rvp_engine::encode::{EncodeJob, encode, encoders};
use rvp_engine::{Engine, EngineEvent, EngineOptions, PropertyId};

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
    // The downmix of the stereo speakers stays in front of the equalizer.
    change_at_the_end("equalizer", 2.7, Action::ToggleEqualizer, |af| {
        af.starts_with("@rvpdownmix:")
            && af
                .find("@rvpeq:")
                .is_some_and(|eq| af.find("@rvpnorm:").is_some_and(|norm| eq < norm))
    });
}

#[test]
fn the_album_moves_on_when_the_speed_changes_at_the_end() {
    change_at_the_end("speed", 2.7, Action::Speed(Speed::Faster), |af| {
        af.contains("@rvpnorm:")
    });
}

/// Settings for the video cases: a decoder whose copy-back variant
/// differs, so that a filter needing frames in memory changes it.
fn video_settings() -> Settings {
    Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        hwdec: "auto-safe".to_owned(),
        ..Settings::default()
    }
}

/// A video filter and the decoder it needs wait together at the end of a
/// file: mpv answers a decoder change with a restart of the decoder and
/// an exact seek, which is what the wait avoids. Both reach mpv once the
/// next file plays.
#[test]
fn the_decoder_waits_with_the_video_filters_at_the_end() {
    let dir = std::env::temp_dir().join(format!("rvp-end-decoder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut app = App::new(video_settings(), &dir).expect("app");
    app.start(&engine, &[SHORT.to_owned(), LONG.to_owned()])
        .expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 2.7 || playing_long(&app) {
        assert!(Instant::now() < deadline, "the short fixture plays");
        pump(&mut app, &engine, 5);
    }
    std::thread::sleep(Duration::from_millis(400));
    app.dispatch(&engine, Action::VideoEffect(VideoEffect::Sharpen));
    let decoder_at_end = engine.get_text("hwdec");
    let filters_at_end = engine.get_text("vf").unwrap_or_default();
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
    assert_eq!(decoder_at_end.as_deref(), Some("auto-safe"));
    assert!(!filters_at_end.contains("@rvpsharpen:"), "{filters_at_end}");
    assert_eq!(engine.get_text("hwdec").as_deref(), Some("auto-copy"));
    let filters = engine.get_text("vf").unwrap_or_default();
    assert!(filters.contains("@rvpsharpen:"), "{filters}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A clip shorter than the wait at the end: a video filter changed while
/// it plays reaches mpv when it starts again (the album of one repeats),
/// and the album goes on.
#[test]
fn a_video_filter_changed_on_a_clip_shorter_than_the_wait_comes_at_its_next_start() {
    let dir = std::env::temp_dir().join(format!("rvp-end-clip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    // Encoded by a core of its own, after the player's (FFmpeg's log stays
    // with the first core).
    let clip = dir.join("clip-1s.mkv");
    let job = EncodeJob {
        source: SHORT.to_owned(),
        output: clip.clone(),
        start: 0.0,
        end: 1.0,
        vf: String::new(),
        extra: encoders("mkv"),
    };
    encode(&job, Duration::from_secs(60)).expect("the clip");
    let clip = clip.display().to_string();
    let mut app = App::new(
        Settings {
            repeat: Repeat::Album,
            ..video_settings()
        },
        &dir.join("state"),
    )
    .expect("app");
    app.start(&engine, std::slice::from_ref(&clip))
        .expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 0.2 {
        assert!(Instant::now() < deadline, "the clip plays");
        pump(&mut app, &engine, 5);
    }
    let duration = app.model.double(PropertyId::Duration).unwrap_or(0.0);
    assert!(duration < 1.5, "a clip shorter than the wait: {duration}");
    app.dispatch(&engine, Action::VideoEffect(VideoEffect::Sharpen));
    let filters_now = engine.get_text("vf").unwrap_or_default();
    assert!(!filters_now.contains("@rvpsharpen:"), "{filters_now}");
    // The filter at each start after the change: the first has it, and
    // the album goes on with it.
    let mut starts: Vec<bool> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while starts.len() < 3 {
        assert!(
            Instant::now() < deadline,
            "starts with the filter {starts:?}; at {:?} of {:?}, vf {:?}",
            app.model.double(PropertyId::TimePos),
            app.model.loaded_path(),
            engine.get_text("vf")
        );
        std::thread::sleep(Duration::from_millis(5));
        let drained = app.drain(&engine, Instant::now());
        for event in &drained.events {
            if matches!(event, EngineEvent::FileLoaded { .. }) {
                starts.push(
                    engine
                        .get_text("vf")
                        .is_some_and(|vf| vf.contains("@rvpsharpen:")),
                );
            }
        }
    }
    assert!(starts.iter().all(|filter| *filter), "{starts:?}");
    assert_eq!(engine.get_text("hwdec").as_deref(), Some("auto-copy"));
    let _ = std::fs::remove_dir_all(&dir);
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
