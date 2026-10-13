//! M6's video processing in headless libmpv (no window, no sound): RVP's
//! filters in front of the chain with their own labels, SVP's `@svp` left
//! where it is while they come and go during playback, the renderer's
//! colour, rotation and deinterlacing properties, the shader list, and a
//! VapourSynth script slot where a VapourSynth runtime is installed.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::video::{ColorChange, ColorKind, Deinterlace, Rotation, ShaderChange, VideoEffect};
use rvp_app::{Action, App, Settings};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-video-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A headless engine and an app playing the long fixture.
fn playing(dir: &Path, settings: Settings) -> (Engine, App) {
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    // SVP mode off: its watch is not under test here.
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        hwdec: "auto-safe".to_owned(),
        ..settings
    };
    let mut app = App::new(settings, dir).expect("app");
    app.start(&engine, &[LONG.to_owned()]).expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 0.5 {
        assert!(Instant::now() < deadline, "the fixture plays");
        pump(&mut app, &engine, 20);
    }
    (engine, app)
}

fn pump(app: &mut App, engine: &Engine, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        app.drain(engine, Instant::now());
    }
}

/// The labels of mpv's video filter chain, in order.
fn labels(engine: &Engine) -> Vec<String> {
    let chain = engine.get_text("vf").unwrap_or_default();
    chain
        .split(',')
        .filter_map(|entry| entry.strip_prefix('@'))
        .filter_map(|entry| entry.split(':').next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn rvp_filters_go_before_svp_and_leave_it_alone() {
    let dir = scratch("chain");
    let (engine, mut app) = playing(&dir, Settings::default());
    // SVP adds its filter while the file plays (a stand-in here).
    engine
        .command("vf", &["add", "@svp:lavfi=[null]"])
        .expect("vf add");
    let svp = engine.get_text("vf").expect("vf");
    assert_eq!(labels(&engine), ["svp"]);

    assert_eq!(app.effective_hwdec(), "auto-safe");
    app.dispatch(&engine, Action::VideoEffect(VideoEffect::Sharpen));
    pump(&mut app, &engine, 200);
    assert_eq!(labels(&engine), ["rvpsharpen", "svp"]);
    // RVP's filters need the frames in memory: copy-back decoding.
    assert_eq!(app.effective_hwdec(), "auto-copy");
    assert_eq!(engine.get_text("hwdec").as_deref(), Some("auto-copy"));
    // Effects keep their order whatever order they were switched on in.
    app.dispatch(&engine, Action::VideoEffect(VideoEffect::Deblock));
    app.dispatch(&engine, Action::VideoEffect(VideoEffect::FlipHorizontal));
    pump(&mut app, &engine, 200);
    assert_eq!(
        labels(&engine),
        ["rvpdeblock", "rvpsharpen", "rvphflip", "svp"]
    );
    assert!(
        engine
            .get_text("vf")
            .is_some_and(|chain| chain.ends_with(&svp)),
        "SVP's entry is unchanged"
    );
    // Off again, one by one, during playback.
    for effect in [
        VideoEffect::Sharpen,
        VideoEffect::Deblock,
        VideoEffect::FlipHorizontal,
    ] {
        app.dispatch(&engine, Action::VideoEffect(effect));
        pump(&mut app, &engine, 100);
    }
    assert_eq!(engine.get_text("vf").as_deref(), Some(svp.as_str()));
    assert!(app.model.playing(), "still playing");
    assert_eq!(app.effective_hwdec(), "auto-safe");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn colour_rotation_deinterlacing_and_shaders_reach_the_renderer() {
    let dir = scratch("picture");
    let (engine, mut app) = playing(&dir, Settings::default());
    let int = |name: &str| engine.get_double(name).map(f64::round);

    for _ in 0..3 {
        app.dispatch(
            &engine,
            Action::Color(ColorChange::Up(ColorKind::Brightness)),
        );
    }
    app.dispatch(
        &engine,
        Action::Color(ColorChange::Down(ColorKind::Contrast)),
    );
    app.set_color(&engine, ColorKind::Gamma, 40);
    assert_eq!(int("brightness"), Some(3.0));
    assert_eq!(int("contrast"), Some(-1.0));
    assert_eq!(int("gamma"), Some(40.0));
    // Q: off, then the last used values again.
    app.dispatch(&engine, Action::Color(ColorChange::Toggle));
    assert_eq!(int("brightness"), Some(0.0));
    assert_eq!(int("gamma"), Some(0.0));
    app.dispatch(&engine, Action::Color(ColorChange::Toggle));
    assert_eq!(int("brightness"), Some(3.0));
    app.dispatch(
        &engine,
        Action::Color(ColorChange::Reset(ColorKind::Brightness)),
    );
    assert_eq!(int("brightness"), Some(0.0));

    app.dispatch(&engine, Action::Rotate(Rotation::To(90)));
    assert_eq!(int("video-rotate"), Some(90.0));
    app.dispatch(&engine, Action::Rotate(Rotation::Cycle));
    assert_eq!(int("video-rotate"), Some(180.0));
    app.dispatch(&engine, Action::Rotate(Rotation::To(0)));
    assert_eq!(int("video-rotate"), Some(0.0));

    app.dispatch(&engine, Action::SetDeinterlace(Deinterlace::On));
    assert_eq!(engine.get_text("deinterlace").as_deref(), Some("yes"));
    app.dispatch(&engine, Action::SetDeinterlace(Deinterlace::Off));
    assert_eq!(engine.get_text("deinterlace").as_deref(), Some("no"));

    let shader = dir.join("identity.glsl");
    std::fs::write(
        &shader,
        "//!HOOK MAIN\n//!BIND HOOKED\n//!DESC identity\nvec4 hook() { return HOOKED_texOff(0); }\n",
    )
    .expect("write shader");
    app.settings.shaders.files = vec![shader.clone()];
    app.dispatch(&engine, Action::Shaders(ShaderChange::Toggle));
    let list = engine.get_text("glsl-shaders").unwrap_or_default();
    assert!(list.contains("identity.glsl"), "shaders on: {list:?}");
    app.dispatch(&engine, Action::Shaders(ShaderChange::Off));
    assert_eq!(engine.get_text("glsl-shaders").as_deref(), Some(""));
    assert!(app.model.playing(), "still playing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A VSScript library mpv will find: `VSSCRIPT_PATH` or a system install.
fn vapoursynth_available() -> bool {
    std::env::var_os("VSSCRIPT_PATH").is_some_and(|path| Path::new(&path).is_file())
        || [
            "/usr/lib/x86_64-linux-gnu/libvapoursynth-script.so",
            "/usr/local/lib/libvapoursynth-script.so",
            "/usr/lib/libvapoursynth-script.so",
        ]
        .iter()
        .any(|path| Path::new(path).is_file())
}

#[test]
fn a_potplayer_style_script_runs_in_its_slot() {
    if !vapoursynth_available() {
        eprintln!("skipped: no VapourSynth runtime (set VSSCRIPT_PATH to run this test)");
        return;
    }
    let dir = scratch("vpy");
    // PotPlayer's scripts call get_core(), which VapourSynth R73 removed;
    // this one doubles the frame rate so that the result shows.
    let script = dir.join("double.vpy");
    std::fs::write(
        &script,
        "import vapoursynth as vs\n\
         core = vs.get_core()\n\
         clip = video_in\n\
         clip = core.std.Interleave([clip, clip])\n\
         clip = core.std.AssumeFPS(clip, fpsnum=clip.fps.numerator * 2, fpsden=clip.fps.denominator)\n\
         clip.set_output()\n",
    )
    .expect("write script");
    let (engine, mut app) = playing(&dir, Settings::default());
    engine
        .command("vf", &["add", "@svp:lavfi=[null]"])
        .expect("vf add");
    app.settings.vapoursynth.slots = vec![script.clone()];
    app.settings.vapoursynth.script = Some(script);
    app.dispatch(&engine, Action::ToggleVapourSynth);
    assert_eq!(labels(&engine), ["rvpvs", "svp"]);
    pump(&mut app, &engine, 3000);
    let source = engine.get_double("container-fps").unwrap_or_default();
    let filtered = engine.get_double("estimated-vf-fps").unwrap_or_default();
    assert!(
        filtered > source * 1.8,
        "the script runs: {source} fps in, {filtered} fps out"
    );
    app.dispatch(&engine, Action::ToggleVapourSynth);
    assert_eq!(labels(&engine), ["svp"]);
    let _ = std::fs::remove_dir_all(&dir);
}
