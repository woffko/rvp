//! M18's study bench (`docs/spikes/m18-smoothing.md`): runs one motion
//! smoothing engine in a headless libmpv core and measures it.
//!
//! ```text
//! smoothing_bench --input FILE --engine ENGINE [--preset P] [--target T]
//!                 [--mode speed|square|grab] [--seconds S] [--out DIR]
//!                 [--rife DIR] [--rife-model NAME] [--hwdec H]
//!                 [--at SECONDS] [--frames N] [--band Y:H] [--source-fps R]
//!                 [--mpv NAME=VALUE]... [--raw FILE --raw-width W --raw-start T]
//!                 [--vs-frames N]
//! ```
//!
//! Engines: `none`, `minterpolate` (FFmpeg's, in mpv's `lavfi`), `mvtools`
//! and `rife` (VapourSynth scripts the bench writes; libmpv finds
//! VapourSynth through `VSSCRIPT_PATH`, mvtools must be in its plugin
//! folder, `--rife` names the folder of `librife` and its `models`).
//! Presets: `fast`, `medium`, `quality`. The target is `2x` (the default),
//! `2.5x`, or a rate such as `60` or `60000/1001`; a factor multiplies
//! `--source-fps` (default `24000/1001`, the fixture's is `24`).
//!
//! `speed` plays as fast as it can (`untimed`) for `--seconds` and prints
//! how much faster than real time the chain ran (`realtime_ratio`) and so
//! the output rate it can sustain (`capacity_fps`). `square` steps through
//! the output frames from `--at` on a clip of a white square moving over
//! grey (`tools/test-clips/make-clips.sh`, `fixture-square.mkv`), finds the
//! square in the rows of `--band` and compares each frame's left edge with
//! the line through the source frames' edges (`edge_error`), and counts
//! pixels between the background's and the square's brightness
//! (`ghost_ratio`: blended or smeared pixels per square pixel). It writes
//! each frame's band as a PGM file into `--out`. `grab` writes the whole
//! output frames from `--at` as PPM files, to look at (any clip). With
//! `--raw` the output
//! frames come from a file of 8-bit grey bands `--raw-width` wide (an
//! engine outside mpv, such as FFmpeg's `fruc_vulkan`), the first at
//! `--raw-start` seconds, at the target rate.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use rvp_engine::{AudioOutput, Engine, EngineOptions, Image, VideoOutput};

struct Options {
    input: String,
    engine: String,
    preset: String,
    target: String,
    mode: String,
    seconds: f64,
    out: PathBuf,
    rife: Option<PathBuf>,
    rife_model: String,
    hwdec: String,
    at: f64,
    frames: usize,
    band: (usize, usize),
    source_fps: (u64, u64),
    /// Further mpv options (`--mpv NAME=VALUE`).
    mpv: Vec<(String, String)>,
    raw: Option<(PathBuf, usize, f64)>,
    /// mpv's `buffered-frames` and `concurrent-frames` for VapourSynth
    /// (0: 8 and mpv's choice).
    vs_frames: u32,
}

fn parse() -> Result<Options, String> {
    let mut options = Options {
        input: String::new(),
        engine: "none".to_owned(),
        preset: "medium".to_owned(),
        target: "2x".to_owned(),
        mode: "speed".to_owned(),
        seconds: 15.0,
        out: std::env::temp_dir().join("rvp-smoothing-bench"),
        rife: None,
        rife_model: "rife-v4.6_ensembleFalse".to_owned(),
        hwdec: "no".to_owned(),
        at: 1.0,
        frames: 12,
        band: (460, 160),
        source_fps: (24_000, 1001),
        mpv: Vec::new(),
        raw: None,
        vs_frames: 0,
    };
    let (mut raw, mut raw_width, mut raw_start) = (None, 0, 0.0);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--input" => options.input = value()?,
            "--engine" => options.engine = value()?,
            "--preset" => options.preset = value()?,
            "--target" => options.target = value()?,
            "--mode" => options.mode = value()?,
            "--seconds" => options.seconds = number(&value()?)?,
            "--out" => options.out = PathBuf::from(value()?),
            "--rife" => options.rife = Some(PathBuf::from(value()?)),
            "--rife-model" => options.rife_model = value()?,
            "--hwdec" => options.hwdec = value()?,
            "--at" => options.at = number(&value()?)?,
            "--frames" => {
                options.frames = value()?.parse().map_err(|_| "bad --frames")?;
            }
            "--band" => {
                let text = value()?;
                let (y, h) = text.split_once(':').ok_or("--band is Y:H")?;
                options.band = (
                    y.parse().map_err(|_| "bad --band")?,
                    h.parse().map_err(|_| "bad --band")?,
                );
            }
            "--source-fps" => options.source_fps = rate(&value()?)?,
            "--raw" => raw = Some(PathBuf::from(value()?)),
            "--vs-frames" => {
                options.vs_frames = value()?.parse().map_err(|_| "bad --vs-frames")?;
            }
            "--raw-width" => raw_width = value()?.parse().map_err(|_| "bad --raw-width")?,
            "--raw-start" => raw_start = number(&value()?)?,
            "--mpv" => {
                let text = value()?;
                let (name, value) = text.split_once('=').ok_or("--mpv is NAME=VALUE")?;
                options.mpv.push((name.to_owned(), value.to_owned()));
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.input.is_empty() {
        return Err("--input is required".to_owned());
    }
    options.raw = raw.map(|path| (path, raw_width, raw_start));
    Ok(options)
}

fn number(text: &str) -> Result<f64, String> {
    text.parse().map_err(|_| format!("{text} is not a number"))
}

/// `60`, `60000/1001` as a fraction.
fn rate(text: &str) -> Result<(u64, u64), String> {
    let bad = || format!("{text} is not a rate");
    match text.split_once('/') {
        Some((num, den)) => Ok((
            num.parse().map_err(|_| bad())?,
            den.parse().map_err(|_| bad())?,
        )),
        None => Ok((text.parse().map_err(|_| bad())?, 1)),
    }
}

fn main() -> ExitCode {
    match parse().and_then(|options| run(&options)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            println!("error={message}");
            ExitCode::FAILURE
        }
    }
}

fn run(options: &Options) -> Result<(), String> {
    std::fs::create_dir_all(&options.out).map_err(|e| e.to_string())?;
    let vf = filter(options)?;
    println!("engine={} preset={}", options.engine, options.preset);
    println!("input={}", options.input);
    println!("vf={vf}");
    match options.mode.as_str() {
        "speed" => speed(options, &vf),
        "square" => square(options, &vf),
        "grab" => grab(options, &vf),
        other => Err(format!("unknown mode {other}")),
    }
}

/// The output rate as a fraction: a factor of the source's or a rate.
fn target(options: &Options) -> Result<(u64, u64), String> {
    let (num, den) = options.source_fps;
    match options.target.strip_suffix('x') {
        Some(factor) => {
            let factor = number(factor)?;
            // A factor such as 2.5 of a rate such as 24000/1001.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let scaled = (num as f64 * factor).round() as u64;
            Ok((scaled, den))
        }
        None => rate(&options.target),
    }
}

/// mpv's `vf` for the engine.
fn filter(options: &Options) -> Result<String, String> {
    let (num, den) = target(options)?;
    match options.engine.as_str() {
        "none" => Ok(String::new()),
        "minterpolate" => {
            let quality = match options.preset.as_str() {
                "fast" => "mc_mode=obmc:me_mode=bilat:me=epzs:vsbmc=0",
                "quality" => "mc_mode=aobmc:me_mode=bidir:me=umh:vsbmc=1",
                _ => "mc_mode=aobmc:me_mode=bidir:me=epzs:vsbmc=1",
            };
            Ok(format!(
                "lavfi=[minterpolate=fps={num}/{den}:mi_mode=mci:{quality}:scd=fdiff]"
            ))
        }
        "mvtools" | "rife" => {
            let script = options.out.join(format!("{}.vpy", options.engine));
            std::fs::write(&script, vapoursynth_script(options, (num, den))?)
                .map_err(|e| format!("{}: {e}", script.display()))?;
            let path = script.to_string_lossy().replace('\\', "/");
            let frames = if options.vs_frames == 0 {
                "buffered-frames=8:concurrent-frames=auto".to_owned()
            } else {
                format!(
                    "buffered-frames={0}:concurrent-frames={0}",
                    options.vs_frames
                )
            };
            Ok(format!("vapoursynth=file=%{}%{path}:{frames}", path.len()))
        }
        other => Err(format!("unknown engine {other}")),
    }
}

fn vapoursynth_script(options: &Options, (num, den): (u64, u64)) -> Result<String, String> {
    let mut script = format!(
        "import vapoursynth as vs\n\
         from fractions import Fraction\n\
         core = vs.core\n\
         rate = Fraction({num}, {den})\n\
         clip = core.std.AssumeFPS(video_in, fpsnum={source_num}, fpsden={source_den})\n",
        source_num = options.source_fps.0,
        source_den = options.source_fps.1,
    );
    if options.engine == "mvtools" {
        let (pel, block, search, flow, recalculate) = match options.preset.as_str() {
            "fast" => (1, 32, 2, false, false),
            "quality" => (2, 16, 3, true, true),
            _ => (2, 16, 3, true, false),
        };
        let _ = write!(
            script,
            "sup = core.mv.Super(clip, pel={pel}, hpad=16, vpad=16, sharp=2, rfilter=4)\n\
             kw = dict(blksize={block}, overlap={half}, search={search}, truemotion=True)\n\
             bv = core.mv.Analyse(sup, isb=True, **kw)\n\
             fv = core.mv.Analyse(sup, isb=False, **kw)\n",
            half = block / 2,
        );
        if recalculate {
            script.push_str(
                "rkw = dict(blksize=8, overlap=4, search=3, truemotion=True)\n\
                 bv = core.mv.Recalculate(sup, bv, **rkw)\n\
                 fv = core.mv.Recalculate(sup, fv, **rkw)\n",
            );
        }
        if flow {
            script.push_str(
                "clip = core.mv.FlowFPS(clip, sup, bv, fv, num=rate.numerator, \
                 den=rate.denominator, mask=2, ml=100)\n",
            );
        } else {
            script.push_str(
                "clip = core.mv.BlockFPS(clip, sup, bv, fv, num=rate.numerator, \
                 den=rate.denominator, mode=3, thscd1=400, thscd2=130)\n",
            );
        }
    } else {
        let folder = options.rife.as_ref().ok_or("rife needs --rife DIR")?;
        let plugin = folder.join("librife_windows_x86-64.dll");
        let model = folder.join("models").join(&options.rife_model);
        let threads = match options.preset.as_str() {
            "fast" => 1,
            "quality" => 4,
            _ => 2,
        };
        let _ = write!(
            script,
            "core.std.LoadPlugin(r'{plugin}')\n\
             matrix = '709' if clip.height > 576 else '170m'\n\
             rgb = core.resize.Bicubic(clip, format=vs.RGBS, matrix_in_s=matrix)\n\
             rgb = core.rife.RIFE(rgb, model_path=r'{model}', fps_num=rate.numerator, \
             fps_den=rate.denominator, gpu_thread={threads}, uhd=clip.height > 1440)\n\
             clip = core.resize.Bicubic(rgb, format=vs.YUV420P8, matrix_s=matrix)\n",
            plugin = plugin.display(),
            model = model.display(),
        );
    }
    script.push_str("clip.set_output()\n");
    Ok(script)
}

fn engine(options: &Options, vf: &str, extra: &[(&str, &str)]) -> Result<Engine, String> {
    let mut settings = EngineOptions {
        video: VideoOutput::Null,
        audio: AudioOutput::Null,
        hwdec: options.hwdec.clone(),
        ipc_server: None,
        extra: Vec::new(),
        optional: Vec::new(),
        builtin_scripts: false,
    };
    for (name, value) in extra {
        settings
            .extra
            .push(((*name).to_owned(), (*value).to_owned()));
    }
    settings.extra.extend(options.mpv.iter().cloned());
    // mpv's log says why a filter failed (`failure`).
    settings.extra.push((
        "log-file".to_owned(),
        options.out.join("mpv.log").to_string_lossy().into_owned(),
    ));
    if !vf.is_empty() {
        settings.extra.push(("vf".to_owned(), vf.to_owned()));
    }
    Engine::new(&settings).map_err(|error| error.to_string())
}

/// The first fatal message or disabled filter in mpv's log.
fn failure(options: &Options) -> Option<String> {
    let log = std::fs::read_to_string(options.out.join("mpv.log")).ok()?;
    log.lines()
        .find(|line| line.contains("][f][") || line.contains("Disabling filter"))
        .map(str::to_owned)
}

fn wait_for(what: &str, timeout: Duration, mut done: impl FnMut() -> bool) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while !done() {
        if Instant::now() > deadline {
            return Err(format!("timed out waiting for {what}"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn speed(options: &Options, vf: &str) -> Result<(), String> {
    // As fast as the chain goes: mpv's top speed without dropping frames
    // (its `untimed` still paced this headless core), and no audio, whose
    // clock would pace it too.
    let player = engine(
        options,
        vf,
        &[
            ("speed", "100"),
            ("framedrop", "no"),
            ("video-sync", "desync"),
            ("aid", "no"),
        ],
    )?;
    player
        .command("loadfile", &[options.input.as_str()])
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    wait_for("playback", Duration::from_secs(60), || {
        player
            .get_double("time-pos")
            .is_some_and(|time| time > 0.05)
    })?;
    println!("started_after_s={:.2}", started.elapsed().as_secs_f64());
    let t0 = Instant::now();
    let p0 = player.get_double("time-pos").unwrap_or(0.0);
    let mut p1 = p0;
    let mut wall = 0.0;
    while t0.elapsed().as_secs_f64() < options.seconds {
        std::thread::sleep(Duration::from_millis(5));
        if let Some(position) = player.get_double("time-pos")
            && position > p1
        {
            p1 = position;
            wall = t0.elapsed().as_secs_f64();
        }
        if player.get_text("eof-reached").as_deref() == Some("yes") {
            break;
        }
    }
    let ratio = (p1 - p0) / wall;
    // The rate asked for: mpv's estimate lags behind a filter's.
    let (num, den) = if vf.is_empty() {
        options.source_fps
    } else {
        target(options)?
    };
    #[allow(clippy::cast_precision_loss)]
    let output = num as f64 / den as f64;
    println!(
        "estimated_vf_fps={:.3}",
        player.get_double("estimated-vf-fps").unwrap_or(0.0)
    );
    println!(
        "video_params={}",
        player.get_text("video-params").unwrap_or_default()
    );
    println!("vf_now={}", player.get_text("vf").unwrap_or_default());
    println!(
        "container_fps={:.3}",
        player.get_double("container-fps").unwrap_or(0.0)
    );
    println!("output_fps={output:.3}");
    println!("measured_s={wall:.2} media_s={:.2}", p1 - p0);
    println!("realtime_ratio={ratio:.3}");
    println!("capacity_fps={:.1}", ratio * output);
    drop(player);
    if let Some(line) = failure(options) {
        println!("error=the filter failed: {line}");
    }
    Ok(())
}

/// The square in an image's band: its left and right edge, its area, and
/// the pixels between background and square brightness.
struct Found {
    left: Option<usize>,
    right: Option<usize>,
    area: usize,
    ghost: usize,
}

/// A pixel coordinate as a number.
fn pixels(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::MAX, f64::from)
}

fn luma(image: &Image, x: usize, y: usize) -> u8 {
    image.rgb(x, y).map_or(0, |[r, g, b]| {
        // Grey and white only: any weighting will do.
        let sum = u16::from(r) + u16::from(g) + u16::from(b);
        u8::try_from(sum / 3).unwrap_or(u8::MAX)
    })
}

fn find(image: &Image, band: (usize, usize)) -> Found {
    let mut found = Found {
        left: None,
        right: None,
        area: 0,
        ghost: 0,
    };
    for y in band.0..(band.0 + band.1).min(image.height) {
        for x in 0..image.width {
            let value = luma(image, x, y);
            if value > 200 {
                found.area += 1;
                found.left = Some(found.left.map_or(x, |left| left.min(x)));
                found.right = Some(found.right.map_or(x, |right| right.max(x)));
            } else if value > 145 {
                found.ghost += 1;
            }
        }
    }
    found
}

fn write_band(image: &Image, band: (usize, usize), path: &Path) -> Result<(), String> {
    let rows = (band.0 + band.1).min(image.height).saturating_sub(band.0);
    let mut data = format!("P5\n{} {rows}\n255\n", image.width).into_bytes();
    for y in band.0..band.0 + rows {
        for x in 0..image.width {
            data.push(luma(image, x, y));
        }
    }
    std::fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

/// Opens the input paused at `--at` with `vf`. `exact` seeks to the frame,
/// which runs every frame from the keyframe before through the filter (too
/// slow for a slow filter far into a film); otherwise playback starts at
/// that keyframe.
fn paused_at(options: &Options, vf: &str, exact: bool) -> Result<Engine, String> {
    let start = format!("{:.3}", options.at);
    let seek = if exact { "yes" } else { "no" };
    let player = engine(
        options,
        vf,
        &[("pause", "yes"), ("start", &start), ("hr-seek", seek)],
    )?;
    player
        .command("loadfile", &[options.input.as_str()])
        .map_err(|e| e.to_string())?;
    wait_for("the first frame", Duration::from_secs(120), || {
        player.get_double("time-pos").is_some() && player.screenshot_raw("video").is_ok()
    })?;
    Ok(player)
}

/// Writes the output frames from `at` as PPM files.
fn grab(options: &Options, vf: &str) -> Result<(), String> {
    let player = paused_at(options, vf, false)?;
    let mut last = f64::NAN;
    for index in 0..options.frames {
        if index > 0 {
            player
                .command("frame-step", &[])
                .map_err(|e| e.to_string())?;
            wait_for("a frame step", Duration::from_secs(30), || {
                player
                    .get_double("time-pos")
                    .is_some_and(|time| (time - last).abs() > 1e-6)
            })?;
        }
        last = player.get_double("time-pos").unwrap_or(f64::NAN);
        let image = player.screenshot_raw("video").map_err(|e| e.to_string())?;
        let mut data = format!("P6\n{} {}\n255\n", image.width, image.height).into_bytes();
        for y in 0..image.height {
            for x in 0..image.width {
                data.extend(image.rgb(x, y).unwrap_or_default());
            }
        }
        let path = options.out.join(format!("grab-{index:03}.ppm"));
        std::fs::write(&path, data).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("frame={index} time={last:.4} file={}", path.display());
    }
    drop(player);
    if let Some(line) = failure(options) {
        println!("error=the filter failed: {line}");
    }
    Ok(())
}

/// Steps through `count` output frames from `at`: each frame's time and
/// what was found in the band.
fn frames(
    options: &Options,
    vf: &str,
    count: usize,
    prefix: &str,
) -> Result<Vec<(f64, Found)>, String> {
    let player = paused_at(options, vf, true)?;
    let mut result = Vec::new();
    let mut last = f64::NAN;
    for index in 0..count {
        if index > 0 {
            player
                .command("frame-step", &[])
                .map_err(|e| e.to_string())?;
            wait_for("a frame step", Duration::from_secs(30), || {
                player
                    .get_double("time-pos")
                    .is_some_and(|time| (time - last).abs() > 1e-6)
            })?;
        }
        let time = player.get_double("time-pos").unwrap_or(f64::NAN);
        last = time;
        let image = player.screenshot_raw("video").map_err(|e| e.to_string())?;
        write_band(
            &image,
            options.band,
            &options.out.join(format!("{prefix}-{index:03}.pgm")),
        )?;
        result.push((time, find(&image, options.band)));
    }
    Ok(result)
}

/// The output frames of a file of grey bands (`--raw`).
fn raw_frames(
    options: &Options,
    path: &Path,
    width: usize,
    start: f64,
) -> Result<Vec<(f64, Found)>, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let size = width * options.band.1;
    if size == 0 {
        return Err("--raw needs --raw-width and --band".to_owned());
    }
    let (num, den) = target(options)?;
    let mut result = Vec::new();
    for (index, band) in data.chunks_exact(size).take(options.frames).enumerate() {
        let image = Image {
            width,
            height: options.band.1,
            stride: width * 4,
            format: "bgr0".to_owned(),
            data: band.iter().flat_map(|&v| [v, v, v, 0]).collect(),
        };
        let band = (0, options.band.1);
        write_band(
            &image,
            band,
            &options.out.join(format!("output-{index:03}.pgm")),
        )?;
        // Frame numbers and rates are small.
        #[allow(clippy::cast_precision_loss)]
        let time = start + index as f64 * den as f64 / num as f64;
        result.push((time, find(&image, band)));
    }
    Ok(result)
}

fn square(options: &Options, vf: &str) -> Result<(), String> {
    // The source's own frames give the line the square's edge follows.
    let source = frames(options, "", options.frames / 2 + 2, "source")?;
    let points: Vec<(f64, f64)> = source
        .iter()
        .filter_map(|(time, found)| found.left.map(|left| (*time, pixels(left))))
        .collect();
    // Few points: their count is far inside f64's exact range.
    #[allow(clippy::cast_precision_loss)]
    let n = points.len() as f64;
    if points.len() < 2 {
        return Err("the square was not found in the source".to_owned());
    }
    let mean_t = points.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_x = points.iter().map(|p| p.1).sum::<f64>() / n;
    let slope = points
        .iter()
        .map(|p| (p.0 - mean_t) * (p.1 - mean_x))
        .sum::<f64>()
        / points.iter().map(|p| (p.0 - mean_t).powi(2)).sum::<f64>();
    println!("source_speed_px_s={slope:.1}");
    let source_area = source.first().map_or(0, |(_, found)| found.area);
    let output = match &options.raw {
        Some((path, width, start)) => raw_frames(options, path, *width, *start)?,
        None => frames(options, vf, options.frames, "output")?,
    };
    let mut error_sum = 0.0;
    let mut error_max: f64 = 0.0;
    let mut ghost_sum = 0;
    let mut measured = 0;
    for (index, (time, found)) in output.iter().enumerate() {
        let expected = mean_x + slope * (time - mean_t);
        let error = found.left.map(|left| pixels(left) - expected);
        println!(
            "frame={index} time={time:.4} left={} right={} area={} ghost={} edge_error={}",
            found.left.map_or("-".to_owned(), |v| v.to_string()),
            found.right.map_or("-".to_owned(), |v| v.to_string()),
            found.area,
            found.ghost,
            error.map_or("-".to_owned(), |e| format!("{e:.1}")),
        );
        if let Some(error) = error {
            error_sum += error.abs();
            error_max = error_max.max(error.abs());
            measured += 1;
        }
        ghost_sum += found.ghost;
    }
    if measured > 0 {
        #[allow(clippy::cast_precision_loss)]
        let mean = error_sum / f64::from(measured);
        println!("edge_error_mean={mean:.2} edge_error_max={error_max:.1}");
    }
    #[allow(clippy::cast_precision_loss)]
    let ghost_ratio = ghost_sum as f64 / (source_area.max(1) * output.len()) as f64;
    println!("ghost_ratio={ghost_ratio:.3}");
    if let Some(line) = failure(options) {
        println!("error=the filter failed: {line}");
    }
    Ok(())
}
