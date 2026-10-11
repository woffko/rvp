//! Usability measurements (docs/PLAN.md section 4): window shown, first
//! video frame, seek to a new frame and input to a visible reaction.
//!
//! A probe starts on the GUI thread and ends on the render thread:
//!
//! * window: process start to the first presented frame;
//! * first frame and seek: start to the first video frame drawn after mpv
//!   reported the playback restart (the frame is presented with the next
//!   swap, a few milliseconds later);
//! * input: the input handler's entry to the first frame presented after
//!   the chrome repainted in response. Qt's own event delivery before the
//!   handler is not included. Input that changes nothing visible within
//!   [`INPUT_LIMIT`] is dropped instead of being matched to a later,
//!   unrelated repaint.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long an input probe may wait for its repaint.
const INPUT_LIMIT: Duration = Duration::from_secs(1);

static START: OnceLock<Instant> = OnceLock::new();
static STATE: Mutex<State> = Mutex::new(State {
    probes: Vec::new(),
    results: Vec::new(),
    window_shown: false,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Waiting for mpv's playback restart.
    Restart,
    /// Waiting for the next video frame.
    VideoFrame,
    /// Waiting for the chrome to repaint.
    Repaint,
    /// Waiting for the next presented frame.
    Swap,
}

struct Probe {
    name: String,
    started: Instant,
    stage: Stage,
}

struct State {
    probes: Vec<Probe>,
    results: Vec<(String, Duration)>,
    window_shown: bool,
}

fn with_state(f: impl FnOnce(&mut State)) {
    if let Ok(mut state) = STATE.lock() {
        f(&mut state);
    }
}

/// Marks the process start.
pub fn start() {
    let _ = START.set(Instant::now());
}

fn started() -> Instant {
    *START.get_or_init(Instant::now)
}

/// Records how long after the process start a startup phase ended
/// (`qml_loaded_ms`, `skin_loaded_ms`, ...).
pub fn mark(name: &str) {
    let elapsed = started().elapsed();
    with_state(|state| state.results.push((name.to_owned(), elapsed)));
}

/// Like [`mark`], for the first time only.
pub fn mark_once(name: &str) {
    let elapsed = started().elapsed();
    with_state(|state| {
        if !state.results.iter().any(|(existing, _)| existing == name) {
            state.results.push((name.to_owned(), elapsed));
        }
    });
}

/// A file was opened from the command line: measure to its first frame.
pub fn first_frame_pending() {
    probe("first_frame_ms", started(), Stage::Restart);
}

/// A seek was issued now.
pub fn seek_issued() {
    probe("seek_ms", Instant::now(), Stage::Restart);
}

/// A key or mouse action was handled now.
pub fn input(name: &str) {
    probe(name, Instant::now(), Stage::Repaint);
}

fn probe(name: &str, started: Instant, stage: Stage) {
    with_state(|state| {
        state.probes.retain(|probe| probe.name != name);
        state.probes.push(Probe {
            name: name.to_owned(),
            started,
            stage,
        });
    });
}

/// mpv reported a playback restart (GUI thread).
pub fn playback_restarted() {
    advance(Stage::Restart, Stage::VideoFrame);
}

/// The chrome was scheduled for repainting (GUI thread).
pub fn chrome_repainted() {
    advance(Stage::Repaint, Stage::Swap);
}

fn advance(from: Stage, to: Stage) {
    with_state(|state| {
        state
            .probes
            .retain(|probe| probe.stage != Stage::Repaint || probe.started.elapsed() < INPUT_LIMIT);
        for probe in &mut state.probes {
            if probe.stage == from {
                probe.stage = to;
            }
        }
    });
}

fn finish(stage: Stage) {
    let now = Instant::now();
    with_state(|state| {
        let (done, waiting): (Vec<Probe>, Vec<Probe>) = std::mem::take(&mut state.probes)
            .into_iter()
            .partition(|probe| probe.stage == stage);
        state.probes = waiting;
        for probe in done {
            state
                .results
                .push((probe.name, now.duration_since(probe.started)));
        }
    });
}

/// mpv drew a frame (render thread).
pub fn video_rendered() {
    finish(Stage::VideoFrame);
}

/// The window presented a frame (render thread).
pub fn frame_swapped() {
    with_state(|state| {
        if !state.window_shown {
            state.window_shown = true;
            state
                .results
                .push(("window_shown_ms".to_owned(), started().elapsed()));
        }
    });
    finish(Stage::Swap);
}

/// Measurements so far, oldest first, as `(name, milliseconds)`.
#[must_use]
pub fn results() -> Vec<(String, f64)> {
    STATE.lock().map_or_else(
        |_| Vec::new(),
        |state| {
            state
                .results
                .iter()
                .map(|(name, duration)| (name.clone(), duration.as_secs_f64() * 1000.0))
                .collect()
        },
    )
}
