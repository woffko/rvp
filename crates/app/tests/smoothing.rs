//! M18's motion smoothing in headless libmpv (no window, no sound): Blend's
//! options, and where a VapourSynth runtime with MVTools is (`VSSCRIPT_PATH`
//! or a system install; skipped elsewhere) the smoothed moving square of
//! `fixture-square.mkv` (twice the frames, the square halfway in the frames
//! between), the picture after exact seeks in `fixture-square-b.mkv`
//! (non-reference B-frames) and the original before a seek's target,
//! toggles during playback, SVP's `@svp` untouched and RVP's own script
//! slot taking smoothing's place.
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

/// A headless engine and an app playing `file` with smoothing on: the
/// filter's frames at once (they are under test; the original shown first
/// after a start or a seek has its own test and the self-test).
fn playing(dir: &Path, file: &str, engine_choice: SmoothingEngine) -> (Engine, App) {
    playing_with(dir, file, engine_choice, 0)
}

/// [`playing`] with `original_first` seconds of the original first.
fn playing_with(
    dir: &Path,
    file: &str,
    engine_choice: SmoothingEngine,
    original_first: u32,
) -> (Engine, App) {
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
    settings.smoothing.original_first = original_first;
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

/// The square's left edge at `time` of the source (make-m18-fixtures.sh:
/// its frame `n`, counted from 1, has it at `abs(mod(8 n + 240, 480) -
/// 240) + 20`; between two frames, in between).
fn square_at(time: f64) -> f64 {
    let n = time * 24.0 + 1.0;
    ((8.0 * n + 240.0).rem_euclid(480.0) - 240.0).abs() + 20.0
}

/// An exact seek into the middle of a group of pictures. mpv decodes from
/// the keyframe on, and with `hr-seek-framedrop` none of the non-reference
/// frames before the target, while the others still go through the
/// filters. mpv's `vapoursynth` filter times its frames by counting them,
/// so the picture then is as many frames ahead of its timestamps, and of
/// the sound, as were left out (the owner's film, 2026-11-21: a seek 2.8 s
/// past a keyframe). `fixture-square-b.mkv` is one group with three such
/// frames in four: the picture after a seek must be its own time's.
#[test]
fn an_exact_seek_shows_the_picture_of_its_time() {
    if vsscript().is_none() {
        eprintln!("skipped: no VapourSynth runtime (set VSSCRIPT_PATH to run this test)");
        return;
    }
    let dir = scratch("seek");
    let (engine, mut app) = playing(
        &dir,
        &fixture("fixture-square-b.mkv"),
        SmoothingEngine::MvTools,
    );
    assert_eq!(labels(&engine), ["rvpsmooth"]);
    engine.set_pause(true).expect("pause");
    // Away from where the square turns (every 1.25 s from 1.208 s on), and
    // back again: every seek starts at the file's one keyframe.
    for target in [4.2, 1.9, 7.9] {
        let took = seek_exact(&engine, &mut app, target);
        eprintln!("the seek to {target} through the engine took {took:?}");
        let time = engine.get_double("time-pos").expect("time");
        let image = engine.screenshot_raw("video").expect("screenshot");
        // A column of a 320-pixel picture: exact as a float.
        let edge = square_edge(&image).and_then(|edge| u32::try_from(edge).ok());
        let expected = square_at(time);
        assert!(
            edge.is_some_and(|edge| (f64::from(edge) - expected).abs() <= 3.0),
            "after a seek to {target} the picture at {time} is that time's: \
             the square's edge at {edge:?}, that time's at {expected:.1}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Seeks exactly to `target` while paused and waits until mpv shows it.
fn seek_exact(engine: &Engine, app: &mut App, target: f64) -> Duration {
    let start = Instant::now();
    engine
        .command("seek", &[&target.to_string(), "absolute+exact"])
        .expect("seek");
    let deadline = start + Duration::from_secs(60);
    let mut settled = 0;
    while settled < 10 {
        assert!(Instant::now() < deadline, "the seek to {target} ends");
        pump(app, engine, 30);
        let near = engine
            .get_double("time-pos")
            .is_some_and(|time| (time - target).abs() < 0.1);
        let seeking = engine.get_text("seeking").as_deref() != Some("no");
        settled = if near && !seeking { settled + 1 } else { 0 };
    }
    start.elapsed()
}

/// The square's left edge in the frame that shows and in the `steps`
/// output frames after it (paused: one frame step each).
fn edges(engine: &Engine, app: &mut App, steps: usize) -> Vec<i64> {
    let mut found = Vec::new();
    for step in 0..=steps {
        if step > 0 {
            let before = engine.get_double("time-pos").unwrap_or(f64::NAN);
            engine.command("frame-step", &[]).expect("frame-step");
            let deadline = Instant::now() + Duration::from_secs(15);
            while engine
                .get_double("time-pos")
                .is_none_or(|time| (time - before).abs() < 1e-6)
            {
                assert!(Instant::now() < deadline, "a frame step");
                pump(app, engine, 20);
            }
        }
        let image = engine.screenshot_raw("video").expect("screenshot");
        let edge = square_edge(&image).and_then(|edge| i64::try_from(edge).ok());
        found.push(edge.expect("the square is in the frame"));
    }
    found
}

/// With the original first, the frames mpv plays through the filter before
/// a seek's target are the source's as well (a copy each: through the
/// engine a seek took seconds), and the engine takes over once playback
/// runs at the target and the original's second is over.
#[test]
fn after_a_seek_the_original_comes_until_playback_runs() {
    if vsscript().is_none() {
        eprintln!("skipped: no VapourSynth runtime (set VSSCRIPT_PATH to run this test)");
        return;
    }
    let dir = scratch("sign");
    let (engine, mut app) = playing_with(
        &dir,
        &fixture("fixture-square-b.mkv"),
        SmoothingEngine::MvTools,
        1,
    );
    assert_eq!(labels(&engine), ["rvpsmooth"]);
    engine.set_pause(true).expect("pause");
    let took = seek_exact(&engine, &mut app, 4.2);
    eprintln!("the seek with the original before its target took {took:?}");
    // The picture is its time's: a frame of the source, so up to one
    // output frame (4 pixels) behind where an engine's frame would be.
    let time = engine.get_double("time-pos").expect("time");
    let at_target = edges(&engine, &mut app, 4);
    assert!(
        (f64::from(i32::try_from(at_target[0]).expect("a column")) - square_at(time)).abs() <= 9.0,
        "the picture at {time} is that time's: {at_target:?}, that time's at {:.1}",
        square_at(time)
    );
    // Paused at the target, the frames are the source's, each twice.
    assert!(
        at_target
            .windows(2)
            .all(|pair| [0, 8].contains(&(pair[1] - pair[0]).abs())),
        "the original's frames at the target: {at_target:?}"
    );
    // Playing on, the engine's frames come after the original's second:
    // the square moves 4 pixels a frame (it turns at 6.208 and 7.458 s).
    engine.set_pause(false).expect("play");
    let deadline = Instant::now() + Duration::from_secs(60);
    while engine.get_double("time-pos").unwrap_or(0.0) < 6.5 {
        assert!(Instant::now() < deadline, "the fixture plays on");
        pump(&mut app, &engine, 20);
    }
    engine.set_pause(true).expect("pause");
    pump(&mut app, &engine, 500);
    let later = edges(&engine, &mut app, 5);
    assert!(
        later
            .windows(2)
            .all(|pair| ((pair[1] - pair[0]).abs() - 4).abs() <= 1),
        "the engine's frames once playback runs: {later:?}"
    );
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

/// The calibration (`rvp_app::calibration`) where a VapourSynth runtime
/// with MVTools is: asked for while nothing plays, it measures MVTools'
/// presets on a generated picture (each at least at the smallest size), writes `calibration.json`, and the next
/// file starts at the first preset it says keeps the rate.
#[test]
fn the_calibration_measures_mvtools_and_the_plan_follows_it() {
    if vsscript().is_none() {
        eprintln!("skipped: no VapourSynth runtime (set VSSCRIPT_PATH to run this test)");
        return;
    }
    let dir = scratch("calibration");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        hwdec: "auto-safe".to_owned(),
        ..Settings::default()
    };
    settings.smoothing.enabled = true;
    settings.smoothing.engine = SmoothingEngine::MvTools;
    settings.smoothing.preset = Preset::Quality;
    "2x".clone_into(&mut settings.smoothing.target);
    settings.smoothing.original_first = 0;
    let mut app = App::new(settings, &dir).expect("app");
    app.prepare_svp(vsscript());
    app.start(&engine, &[]).expect("start");
    app.set_hardware("the test's machine".to_owned());
    assert_eq!(app.calibration_status()["state"], "waiting");
    app.calibrate_now(&engine);
    let deadline = Instant::now() + Duration::from_secs(300);
    while app.calibration_status()["state"] != "measured" {
        assert!(
            Instant::now() < deadline,
            "the calibration did not finish: {}",
            app.calibration_status()
        );
        app.drain(&engine, Instant::now());
        drop(app.calibration_tick(&engine, Instant::now()));
        std::thread::sleep(Duration::from_millis(100));
    }
    let calibration = rvp_app::calibration::load(&dir).expect("calibration.json");
    assert!(
        calibration.key.contains("the test's machine"),
        "{calibration:?}"
    );
    // RIFE may not run (the test VM has no GPU); MVTools has to.
    assert!(
        !calibration.failed.contains(&SmoothingEngine::MvTools),
        "{calibration:?}"
    );
    let measured: Vec<_> = calibration
        .points
        .iter()
        .filter(|point| {
            point.engine == SmoothingEngine::MvTools
                && (point.width, point.height) == rvp_app::calibration::SIZES[0]
        })
        .collect();
    assert_eq!(measured.len(), 3, "{calibration:?}");
    assert!(
        measured.iter().all(|point| point.fps > 1.0),
        "{calibration:?}"
    );
    let rows = app.calibration_status()["rows"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty(), "{}", app.calibration_status());
    // The next file starts at the first MVTools preset that keeps twice
    // its rate with room, by the calibration, or with Blend.
    app.open(&engine, &[fixture("fixture-square.mkv")], false)
        .expect("open");
    let deadline = Instant::now() + Duration::from_secs(20);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 0.3 {
        assert!(Instant::now() < deadline, "the fixture did not play");
        pump(&mut app, &engine, 50);
    }
    let width = u32::try_from(app.model.int(PropertyId::VideoWidth).unwrap_or(0)).unwrap_or(0);
    let height = u32::try_from(app.model.int(PropertyId::VideoHeight).unwrap_or(0)).unwrap_or(0);
    let fps = 2.0 * app.model.double(PropertyId::ContainerFps).unwrap_or(0.0);
    let fitting = calibration.fitting(width, height, fps);
    let expected = [Preset::Quality, Preset::Medium, Preset::Fast]
        .into_iter()
        .find(|preset| fitting.fits(SmoothingEngine::MvTools, *preset));
    match (app.smoothing_plan(), expected) {
        (Plan::Filter { engine, preset, .. }, Some(wanted)) => {
            assert_eq!((engine, preset), (SmoothingEngine::MvTools, wanted));
        }
        (Plan::Blend, None) => {}
        (plan, wanted) => panic!("plan {plan:?}, calibration {wanted:?}: {calibration:?}"),
    }
}
