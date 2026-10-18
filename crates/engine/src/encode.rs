//! Encoding a stretch of a file with a core of its own (M12's Record
//! Video): mpv's encoding mode (`o`) decodes the file from `start` to
//! `end`, runs the video filters given and writes the result, the format
//! from the file's extension. `docs/spikes/m12-studies.md` has the study.
//!
//! The core must be created after the player's (libmpv gives FFmpeg's log
//! to the first core of a process; the normalizer reads it there). It runs
//! on the caller's thread until the file is written: never the window's.
//! The file is complete once the core has quit (its `Shutdown` event).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use libmpv2::Mpv;
use libmpv2::events::Event;
use libmpv2::mpv_end_file_reason;

use crate::engine::EngineError;

/// A stretch of a file to encode.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodeJob {
    /// The file or address, as the player opened it.
    pub source: String,
    pub output: PathBuf,
    /// Seconds from the start.
    pub start: f64,
    pub end: f64,
    /// mpv `vf` (RVP's own filters; never SVP's).
    pub vf: String,
    /// Further options (the encoders), applied last.
    pub extra: Vec<(String, String)>,
}

/// The encoders RVP gives a file of this extension: H.264 and AAC in
/// Matroska, MP4 and the others FFmpeg picks for the format.
#[must_use]
pub fn encoders(extension: &str) -> Vec<(String, String)> {
    let pair = |name: &str, value: &str| (name.to_owned(), value.to_owned());
    match extension.to_ascii_lowercase().as_str() {
        "mkv" | "mp4" | "mov" => vec![
            pair("ovc", "libx264"),
            pair("ovcopts", "crf=20,preset=medium"),
            pair("oac", "aac"),
            pair("oacopts", "b=192k"),
        ],
        "webm" => vec![
            pair("ovc", "libvpx-vp9"),
            pair("ovcopts", "crf=32,b=0"),
            pair("oac", "libopus"),
        ],
        _ => Vec::new(),
    }
}

/// Encodes `job`; returns once the file is written.
///
/// # Errors
/// Fails when the core cannot start, the file cannot be opened or encoded,
/// or `timeout` passes.
pub fn encode(job: &EncodeJob, timeout: Duration) -> Result<(), EngineError> {
    let fail =
        |context: &str, error: &dyn std::fmt::Display| EngineError(format!("{context}: {error}"));
    let output = job.output.to_string_lossy().into_owned();
    let mpv = Mpv::with_initializer(|init| {
        init.set_option("o", output.as_str())?;
        init.set_option("start", format!("{:.3}", job.start).as_str())?;
        init.set_option("end", format!("{:.3}", job.end).as_str())?;
        init.set_option("vf", job.vf.as_str())?;
        init.set_option("hwdec", "no")?;
        init.set_option("osd-level", "0")?;
        init.set_option("input-default-bindings", "no")?;
        init.set_option("terminal", "no")?;
        for name in ["osc", "ytdl", "load-scripts"] {
            drop(init.set_option(name, "no"));
        }
        for (name, value) in &job.extra {
            init.set_option(name.as_str(), value.as_str())?;
        }
        Ok(())
    })
    .map_err(|error| fail("encode core", &error))?;
    // mpv's errors say why an encoding ended early.
    drop(mpv.request_log_messages("error"));
    mpv.command("loadfile", &[job.source.as_str()])
        .map_err(|error| fail("loadfile", &error))?;
    let deadline = Instant::now() + timeout;
    let mut errors: Vec<String> = Vec::new();
    loop {
        match mpv.wait_event(0.1) {
            Some(Ok(Event::EndFile(reason))) => {
                // mpv finishes the file when its core quits. Dropping the
                // handle is not enough: with a script's client still alive
                // (Windows' libmpv) the core quits after `mpv_destroy`
                // returns, and the file is empty until then.
                drop(mpv.command("quit", &[]));
                while Instant::now() < deadline {
                    if let Some(Ok(Event::Shutdown)) = mpv.wait_event(0.1) {
                        break;
                    }
                }
                drop(mpv);
                return if reason == mpv_end_file_reason::Eof {
                    Ok(())
                } else {
                    Err(EngineError(format!(
                        "the file could not be encoded (end {reason}): {}",
                        errors.join("; ")
                    )))
                };
            }
            Some(Ok(Event::LogMessage { prefix, text, .. })) => {
                if errors.len() < 8 {
                    errors.push(format!("[{prefix}] {}", text.trim()));
                }
            }
            Some(Ok(Event::Shutdown)) => return Ok(()),
            Some(Err(error)) => return Err(fail("encoding", &error)),
            _ => {}
        }
        if Instant::now() >= deadline {
            return Err(EngineError("encoding timed out".to_owned()));
        }
    }
}
