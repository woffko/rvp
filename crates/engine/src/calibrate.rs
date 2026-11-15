//! How fast video filters run (M18's calibration, the owner, 2026-11-07):
//! a core of its own plays a generated picture (FFmpeg's `testsrc2`
//! through mpv's `av://lavfi:`, which needs no file) at a size and rate,
//! without a window or sound (`vo=null`), through the filters given, at
//! [`SPEED`] times real time: as fast as they go up to that. The playback
//! time it covers over a stretch of wall time, after a warm-up, says how
//! many frames a second the filters make; mpv's estimate of the filters'
//! rate says whether they ran at all (mpv drops a filter that fails and
//! plays on without it). `untimed` would have no cap, but mpv does not
//! keep that estimate up to date without timing (the source's rate stays).
//!
//! The core must be created after the player's: libmpv gives FFmpeg's log
//! to the first core of a process (the normalizer reads it there). It runs
//! on the caller's thread until the measurement ends: never the window's.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use libmpv2::Mpv;
use libmpv2::events::Event;

use crate::engine::EngineError;

/// The playback speed of a measurement: the most it can tell. RVP needs
/// to know whether filters keep the rate with some room, and how much
/// room, to guess at larger pictures.
pub const SPEED: f64 = 8.0;

/// One measurement: the picture's size and rate, the filters, and how
/// long to warm up and to measure.
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    pub width: u32,
    pub height: u32,
    /// The generated picture's frame rate (numerator, denominator).
    pub rate: (u64, u64),
    /// mpv `vf` (RVP's own filters).
    pub vf: String,
    pub warm_up: Duration,
    pub measure: Duration,
    /// Further options, applied last (the player's own, such as RVP's
    /// VapourSynth runtime's).
    pub extra: Vec<(String, String)>,
}

/// What a probe measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// Playback seconds per wall second while measured (about [`SPEED`]
    /// when the filters keep up with it).
    pub speed: f64,
    /// mpv's estimate of the rate the filters put out (frames a second of
    /// playback time; the source's rate when they did not run).
    pub filter_fps: f64,
    /// mpv's first error messages meanwhile: why a filter did not run.
    pub errors: Vec<String>,
}

/// The generated picture for `probe`: `testsrc2` (moving colour bars, a
/// gradient and counters) for as long as no measurement lasts.
#[must_use]
pub fn source(probe: &Probe) -> String {
    format!(
        "av://lavfi:testsrc2=size={}x{}:rate={}/{}:duration=3600",
        probe.width, probe.height, probe.rate.0, probe.rate.1
    )
}

/// Measures `probe`. `cancel` stops it early (an error, nothing measured);
/// `timeout` bounds the whole run, the core's start included.
///
/// # Errors
/// Fails when the core cannot start, the picture does not play, the run
/// is cancelled or `timeout` passes.
pub fn measure(
    probe: &Probe,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Measurement, EngineError> {
    let fail =
        |context: &str, error: &dyn std::fmt::Display| EngineError(format!("{context}: {error}"));
    let mpv = Mpv::with_initializer(|init| {
        init.set_option("vo", "null")?;
        init.set_option("speed", SPEED)?;
        init.set_option("ao", "null")?;
        init.set_option("aid", "no")?;
        init.set_option("sid", "no")?;
        init.set_option("hwdec", "no")?;
        init.set_option("vf", probe.vf.as_str())?;
        init.set_option("osd-level", "0")?;
        init.set_option("input-default-bindings", "no")?;
        init.set_option("terminal", "no")?;
        for name in ["osc", "ytdl", "load-scripts"]
            .into_iter()
            .chain(crate::engine::BUILTIN_SCRIPTS)
        {
            drop(init.set_option(name, "no"));
        }
        for (name, value) in &probe.extra {
            init.set_option(name.as_str(), value.as_str())?;
        }
        Ok(())
    })
    .map_err(|error| fail("calibration core", &error))?;
    // mpv's errors say why a filter or the picture failed.
    drop(mpv.request_log_messages("error"));
    let result = run(&mpv, probe, cancel, timeout);
    drop(mpv.command("quit", &[]));
    // The core unloads the file, and the script's filters with it (vs-mlrt's
    // TensorRT engine, RIFE's Vulkan device), before it says it shuts down:
    // waited for, they are gone when this returns. Dropped at once, the
    // core tore them down on its own thread while the process exited, and
    // TensorRT's library crashed in its exit (the kit's m18trt, goal U).
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(Ok(Event::Shutdown) | Err(_)) = mpv.wait_event(0.1) {
            break;
        }
    }
    result
}

/// The measurement on a started core.
fn run(
    mpv: &Mpv,
    probe: &Probe,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Measurement, EngineError> {
    let fail =
        |context: &str, error: &dyn std::fmt::Display| EngineError(format!("{context}: {error}"));
    mpv.command("loadfile", &[source(probe).as_str()])
        .map_err(|error| fail("loadfile", &error))?;
    let deadline = Instant::now() + timeout;
    let mut errors: Vec<String> = Vec::new();
    let mut playing_since: Option<Instant> = None;
    let mut start: Option<(Instant, f64)> = None;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(EngineError("calibration cancelled".to_owned()));
        }
        if Instant::now() >= deadline {
            return Err(EngineError(format!(
                "calibration timed out: {}",
                errors.join("; ")
            )));
        }
        match mpv.wait_event(0.05) {
            Some(Ok(Event::PlaybackRestart)) => {
                playing_since.get_or_insert_with(Instant::now);
            }
            Some(Ok(Event::EndFile(reason))) => {
                return Err(EngineError(format!(
                    "the picture did not play (end {reason}): {}",
                    errors.join("; ")
                )));
            }
            Some(Ok(Event::LogMessage { prefix, text, .. })) => {
                if errors.len() < 8 {
                    errors.push(format!("[{prefix}] {}", text.trim()));
                }
            }
            Some(Ok(Event::Shutdown)) => {
                return Err(EngineError("the calibration core quit".to_owned()));
            }
            Some(Err(error)) => return Err(fail("calibration", &error)),
            _ => {}
        }
        let Some(since) = playing_since else {
            continue;
        };
        let now = Instant::now();
        match start {
            None if now.duration_since(since) >= probe.warm_up => {
                if let Ok(time) = mpv.get_property::<f64>("time-pos") {
                    start = Some((now, time));
                }
            }
            Some((at, from)) if now.duration_since(at) >= probe.measure => {
                let time = mpv
                    .get_property::<f64>("time-pos")
                    .map_err(|error| fail("time-pos", &error))?;
                let filter_fps = mpv.get_property::<f64>("estimated-vf-fps").unwrap_or(0.0);
                let seconds = now.duration_since(at).as_secs_f64();
                return Ok(Measurement {
                    speed: (time - from) / seconds,
                    filter_fps,
                    errors,
                });
            }
            _ => {}
        }
    }
}
