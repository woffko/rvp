//! Spike S2: does libmpv with VapourSynth run a motion-interpolation script
//! unchanged, headless?
//!
//! ```text
//! s2-vapoursynth --script motioninterpolation.vpy [--seconds 10]
//!                [--input av://lavfi:testsrc2=size=1280x720:rate=24000/1001]
//!                [--display-fps 60] [--hwdec no]
//! ```
//!
//! Prints `key=value` lines and exits with 0 when the filtered frame rate
//! reaches at least 90% of the expected rate (the display rate, or the
//! source rate times `--expect-ratio`).

use std::process::ExitCode;
use std::time::{Duration, Instant};

use libmpv2::Mpv;
use libmpv2::events::Event;

struct Options {
    script: String,
    input: String,
    seconds: f64,
    display_fps: f64,
    hwdec: String,
    expect_fps: Option<f64>,
    /// Run the script through a wrapper that restores `vs.get_core()`,
    /// which VapourSynth R6x removed; the script file itself is not changed.
    compat: bool,
}

fn parse_options() -> Result<Options, String> {
    let mut options = Options {
        script: String::new(),
        input: "av://lavfi:testsrc2=size=1280x720:rate=24000/1001".to_owned(),
        seconds: 10.0,
        display_fps: 60.0,
        hwdec: "no".to_owned(),
        expect_fps: None,
        compat: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--script" => options.script = value()?,
            "--input" => options.input = value()?,
            "--seconds" => options.seconds = value()?.parse().map_err(|_| "bad --seconds")?,
            "--display-fps" => {
                options.display_fps = value()?.parse().map_err(|_| "bad --display-fps")?;
            }
            "--hwdec" => options.hwdec = value()?,
            "--compat" => options.compat = true,
            "--expect-fps" => {
                options.expect_fps = Some(value()?.parse().map_err(|_| "bad --expect-fps")?);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.script.is_empty() {
        return Err("--script is required".to_owned());
    }
    Ok(options)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => {
            println!("result=PASS");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("result=FAIL");
            ExitCode::FAILURE
        }
        Err(message) => {
            println!("result=ERROR {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool, String> {
    let options = parse_options()?;
    let original = options.script.replace('\\', "/");
    let script = if options.compat {
        write_compat_wrapper(&original)?
    } else {
        original.clone()
    };
    let mpv = Mpv::with_initializer(|init| {
        init.set_option("vo", "null")?;
        init.set_option("ao", "null")?;
        init.set_option("hwdec", options.hwdec.as_str())?;
        init.set_option("hr-seek-framedrop", "no")?;
        init.set_option("display-fps-override", options.display_fps)?;
        // mpv splits filter arguments at ':'; "%LEN%" quotes a value that
        // contains one, such as a Windows drive letter.
        init.set_option(
            "vf",
            format!("vapoursynth=file=%{}%{script}:buffered-frames=4", script.len()),
        )?;
        init.set_option("msg-level", "all=warn,vapoursynth=v,ffmpeg=error")?;
        init.set_option("terminal", "yes")?;
        Ok(())
    })
    .map_err(|error| format!("mpv initialization failed: {error}"))?;
    let property = |name: &str| mpv.get_property::<String>(name).unwrap_or_default();
    println!("mpv_version={}", property("mpv-version"));
    println!("ffmpeg_version={}", property("ffmpeg-version"));
    println!("script={original}");
    println!("compat_wrapper={}", options.compat);
    println!("input={}", options.input);
    mpv.command("loadfile", &[&options.input])
        .map_err(|error| format!("loadfile failed: {error}"))?;

    let start = Instant::now();
    let deadline = Duration::from_secs_f64(options.seconds + 30.0);
    let mut started: Option<Instant> = None;
    let mut samples: Vec<f64> = Vec::new();
    let mut container_fps = 0.0;
    let mut first_position: Option<(Instant, f64)> = None;
    let mut last_position: Option<(Instant, f64)> = None;
    while start.elapsed() < deadline {
        if let Some(Ok(event)) = mpv.wait_event(0.25) {
            match event {
                Event::PlaybackRestart if started.is_none() => {
                    println!("playback_started_after_s={:.2}", start.elapsed().as_secs_f64());
                    started = Some(Instant::now());
                }
                Event::EndFile(reason) => {
                    println!("end_file={reason:?}");
                    break;
                }
                Event::Shutdown => break,
                _ => {}
            }
        }
        let Some(since) = started else { continue };
        let elapsed = since.elapsed().as_secs_f64();
        if elapsed > 2.0 {
            if let Ok(fps) = mpv.get_property::<f64>("estimated-vf-fps") {
                samples.push(fps);
            }
            container_fps = mpv.get_property::<f64>("container-fps").unwrap_or(container_fps);
            if let Ok(position) = mpv.get_property::<f64>("time-pos") {
                let sample = (Instant::now(), position);
                first_position.get_or_insert(sample);
                last_position = Some(sample);
            }
        }
        if elapsed > options.seconds {
            break;
        }
    }
    println!("vf={}", property("vf"));
    println!("video_params={}", property("video-params"));
    println!("video_out_params={}", property("video-out-params"));
    println!("container_fps={container_fps:.3}");
    println!("time_pos={}", property("time-pos"));
    println!("frame_drop_count={}", property("frame-drop-count"));
    if let (Some((t0, p0)), Some((t1, p1))) = (first_position, last_position) {
        let wall = t1.duration_since(t0).as_secs_f64();
        if wall > 0.0 {
            // 1.0 means the filter chain keeps up with real time.
            println!("realtime_ratio={:.3}", (p1 - p0) / wall);
        }
    }
    if samples.is_empty() {
        println!("estimated_vf_fps=none");
        return Ok(false);
    }
    #[allow(clippy::cast_precision_loss)] // a few hundred samples
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    println!("estimated_vf_fps_mean={mean:.3}");
    println!("estimated_vf_fps_last={:.3}", samples[samples.len() - 1]);
    let expected = options.expect_fps.unwrap_or(options.display_fps);
    println!("expected_fps={expected:.3}");
    Ok(mean >= expected * 0.9)
}

/// Writes a wrapper next to the system temp directory that makes
/// `vs.get_core()` available again and runs the original script in the same
/// globals, so mpv's `video_in`, `container_fps` and `display_fps` reach it.
fn write_compat_wrapper(original: &str) -> Result<String, String> {
    let wrapper = std::env::temp_dir().join("rvp-vs-compat.vpy");
    let escaped = original.replace('\\', "/").replace('\'', "\\'");
    // RVP_VS_PLUGINS lists plugin files to load first (os.pathsep
    // separated), for runtimes without mvtools such as SVP's.
    let text = format!(
        "import os\n\
         import vapoursynth as vs\n\
         if not hasattr(vs, 'get_core'):\n\
         \x20   vs.get_core = lambda *args, **kwargs: vs.core\n\
         for plugin in os.environ.get('RVP_VS_PLUGINS', '').split(os.pathsep):\n\
         \x20   if plugin:\n\
         \x20       vs.core.std.LoadPlugin(plugin)\n\
         with open('{escaped}', encoding='utf-8-sig') as source:\n\
         \x20   exec(compile(source.read(), '{escaped}', 'exec'))\n"
    );
    std::fs::write(&wrapper, text).map_err(|error| format!("cannot write wrapper: {error}"))?;
    Ok(wrapper.to_string_lossy().replace('\\', "/"))
}
