//! M18's motion smoothing in headless libmpv (no window, no sound): Blend's
//! options, and where a VapourSynth runtime with MVTools is (`VSSCRIPT_PATH`
//! or a system install; skipped elsewhere) the smoothed moving square of
//! `fixture-square.mkv` (twice the frames, the square halfway in the frames
//! between), toggles during playback, SVP's `@svp` untouched and RVP's own
//! script slot taking smoothing's place.
//!
//! `RVP_TESTDATA` points at `testdata/media` when the test runs elsewhere
//! (the Windows VM).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::smoothing::{Engine as SmoothingEngine, Plan, Preset, SmoothingCommand};
use rvp_app::{Action, App, Settings};
use rvp_engine::{Engine, EngineOptions, PropertyId};

fn fixture(name: &str) -> String {
    let dir = std::env::var_os("RVP_TESTDATA").map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media")),
        PathBuf::from,
    );
    dir.join(name).to_string_lossy().into_owned()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-smoothing-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The VSScript library mpv will load: `VSSCRIPT_PATH` or a system install.
fn vsscript() -> Option<PathBuf> {
    std::env::var_os("VSSCRIPT_PATH")
        .map(PathBuf::from)
        .into_iter()
        .chain(
            [
                "/usr/lib/x86_64-linux-gnu/libvapoursynth-script.so.0",
                "/usr/lib/x86_64-linux-gnu/libvapoursynth-script.so",
                "/usr/local/lib/libvapoursynth-script.so",
            ]
            .map(PathBuf::from),
        )
        .find(|path| path.is_file())
}

fn pump(app: &mut App, engine: &Engine, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        app.drain(engine, Instant::now());
    }
}

/// A headless engine and an app playing `file` with smoothing on.
fn playing(dir: &Path, file: &str, engine_choice: SmoothingEngine) -> (Engine, App) {
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        // SVP mode off: its watch is not under test here.
        ipc_server: String::new(),
        hwdec: "auto-safe".to_owned(),
        ..Settings::default()
    };
    settings.smoothing.enabled = true;
    settings.smoothing.engine = engine_choice;
    settings.smoothing.preset = Preset::Quality;
    // The headless player knows no display: twice the source's rate.
    "2x".clone_into(&mut settings.smoothing.target);
    let mut app = App::new(settings, dir).expect("app");
    app.prepare_svp(vsscript());
    app.start(&engine, &[file.to_owned()]).expect("start");
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 0.3 {
        assert!(Instant::now() < deadline, "the fixture plays");
        pump(&mut app, &engine, 20);
    }
    (engine, app)
}

/// The labels of mpv's video filter chain, in order.
fn labels(engine: &Engine) -> Vec<String> {
    engine
        .get_text("vf")
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| entry.strip_prefix('@'))
        .filter_map(|entry| entry.split(':').next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn blend_is_mpvs_interpolation_and_gives_the_sync_back() {
    let dir = scratch("blend");
    let (engine, mut app) = playing(&dir, &fixture("fixture-60s.mkv"), SmoothingEngine::Blend);
    assert_eq!(app.smoothing_plan(), Plan::Blend);
    assert_eq!(engine.get_text("interpolation").as_deref(), Some("yes"));
    assert_eq!(
        engine.get_text("video-sync").as_deref(),
        Some("display-resample")
    );
    assert!(!labels(&engine).contains(&"rvpsmooth".to_owned()));
    app.dispatch(&engine, Action::Smoothing(SmoothingCommand::Toggle));
    assert_eq!(app.smoothing_plan(), Plan::Off("off"));
    assert_eq!(engine.get_text("interpolation").as_deref(), Some("no"));
    assert_eq!(engine.get_text("video-sync").as_deref(), Some("audio"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The square's left edge on row 120: the first column whose run of 30
/// white pixels starts there.
fn square_edge(image: &rvp_engine::Image) -> Option<usize> {
    let white = |x: usize| {
        image
            .rgb(x, 120)
            .is_some_and(|pixel| pixel.iter().all(|value| *value > 200))
    };
    (0..image.width.saturating_sub(30)).find(|x| (*x..*x + 30).all(white))
}

#[test]
fn mvtools_doubles_the_frames_and_puts_the_square_halfway() {
    if vsscript().is_none() {
        eprintln!("skipped: no VapourSynth runtime (set VSSCRIPT_PATH to run this test)");
        return;
    }
    let dir = scratch("square");
    let (engine, mut app) = playing(
        &dir,
        &fixture("fixture-square.mkv"),
        SmoothingEngine::MvTools,
    );
    assert!(
        matches!(
            app.smoothing_plan(),
            Plan::Filter {
                engine: SmoothingEngine::MvTools,
                rate: (48, 1),
                ..
            }
        ),
        "{:?}",
        app.smoothing_plan()
    );
    assert_eq!(labels(&engine), ["rvpsmooth"]);
    assert!(app.effective_hwdec().ends_with("copy") || app.effective_hwdec() == "no");
    // Paused where the square moves right (frames 12 to 17 of the source:
    // 8 pixels a frame), then one output frame at a time.
    engine.set_pause(true).expect("pause");
    engine
        .command("seek", &["0.5", "absolute+exact"])
        .expect("seek");
    pump(&mut app, &engine, 1500);
    let mut frames = Vec::new();
    for step in 0..6 {
        if step > 0 {
            let before = engine.get_double("time-pos").unwrap_or(f64::NAN);
            engine.command("frame-step", &[]).expect("frame-step");
            let deadline = Instant::now() + Duration::from_secs(15);
            while engine
                .get_double("time-pos")
                .is_none_or(|time| (time - before).abs() < 1e-6)
            {
                assert!(Instant::now() < deadline, "a frame step");
                pump(&mut app, &engine, 20);
            }
        }
        let time = engine.get_double("time-pos").expect("time");
        let image = engine.screenshot_raw("video").expect("screenshot");
        frames.push((time, square_edge(&image)));
    }
    for pair in frames.windows(2) {
        let ((t0, e0), (t1, e1)) = (pair[0], pair[1]);
        assert!(
            ((t1 - t0) - 1.0 / 48.0).abs() < 0.002,
            "frames 1/48 s apart: {frames:?}"
        );
        let (Some(e0), Some(e1)) = (e0, e1) else {
            panic!("the square is in every frame: {frames:?}");
        };
        assert!(
            e1 > e0 && (e1 - e0).abs_diff(4) <= 1,
            "the square moves 4 pixels a frame (halfway between the source's 8): {frames:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn toggles_leave_svp_alone_and_the_script_slot_takes_over() {
    if vsscript().is_none() {
        eprintln!("skipped: no VapourSynth runtime (set VSSCRIPT_PATH to run this test)");
        return;
    }
    let dir = scratch("toggles");
    let (engine, mut app) = playing(&dir, &fixture("fixture-60s.mkv"), SmoothingEngine::MvTools);
    assert_eq!(labels(&engine), ["rvpsmooth"]);
    app.dispatch(&engine, Action::Smoothing(SmoothingCommand::Toggle));
    assert!(labels(&engine).is_empty());
    // SVP attaches (a stand-in): smoothing stays off and leaves it alone.
    engine
        .command("vf", &["add", "@svp:lavfi=[null]"])
        .expect("vf add");
    let svp = engine.get_text("vf").expect("vf");
    pump(&mut app, &engine, 200);
    app.dispatch(&engine, Action::Smoothing(SmoothingCommand::Toggle));
    pump(&mut app, &engine, 200);
    assert_eq!(app.smoothing_plan(), Plan::Off("SVP is attached"));
    assert_eq!(engine.get_text("vf").as_deref(), Some(svp.as_str()));
    // SVP goes: smoothing comes back by itself.
    engine
        .command("vf", &["remove", "@svp"])
        .expect("vf remove");
    pump(&mut app, &engine, 300);
    assert_eq!(labels(&engine), ["rvpsmooth"]);
    // RVP's own script slot takes smoothing's place.
    let script = dir.join("same.vpy");
    std::fs::write(&script, "video_in.set_output()\n").expect("script");
    app.settings.vapoursynth.slots = vec![script.clone()];
    app.settings.vapoursynth.script = Some(script);
    app.dispatch(&engine, Action::ToggleVapourSynth);
    assert_eq!(labels(&engine), ["rvpvs"]);
    assert_eq!(
        app.smoothing_plan(),
        Plan::Off("RVP's VapourSynth script runs")
    );
    app.dispatch(&engine, Action::ToggleVapourSynth);
    assert_eq!(labels(&engine), ["rvpsmooth"]);
    assert!(app.model.playing(), "still playing");
    let _ = std::fs::remove_dir_all(&dir);
}
