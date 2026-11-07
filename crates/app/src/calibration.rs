//! Motion smoothing's calibration (M18, the owner, 2026-11-07: "as SVP
//! does"): RVP measures once how many frames a second each of its
//! VapourSynth engines makes on this machine, at each preset and at 720p,
//! 1080p, 1440p and 2160p, and each file then starts at the best step of
//! the smoothing's ladder that keeps the rate with [`HEADROOM`] to spare,
//! instead of stepping down from a preset the machine cannot keep (each
//! step changes mpv's filter chain: about 100 dropped frames on the host).
//!
//! * The measurements run while nothing plays, one at a time on a thread
//!   of their own ([`Runner`]): a core without a window or sound plays a
//!   generated picture through the same script and filter entry as
//!   playback, at the measurement's rate ([`SOURCE_RATE`] to [`RATE`]), as
//!   fast as they go up to `rvp_engine::calibrate::SPEED` times real time.
//!   A file starting stops them; they start again when nothing plays.
//! * Larger pictures are measured only while the last one could keep the
//!   rate, and not when the last one's rate, scaled by the pictures' pixel
//!   counts, falls short of it ([`next`]): RIFE at 2160p took the host's
//!   GPU down on 2026-10-18, and an engine is never pushed far past what
//!   it can do.
//! * The results go to `calibration.json` in the state folder with the
//!   machine's key: the processors, the graphics cards and their drivers,
//!   libmpv, the engines' files and [`VERSION`]. Another key measures again.
//! * NVIDIA Optical Flow (`fruc_vulkan`) needs mpv's Vulkan output and is
//!   not measured: the drop watch decides for it, as before.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use rvp_engine::Engine as Mpv;

use crate::app::{App, UiRequest};
use crate::smoothing::{Engine, Preset};

/// The picture sizes measured, smallest first.
pub const SIZES: [(u32, u32); 5] = [
    (854, 480),
    (1280, 720),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];

/// The engines measured, in order.
pub const ENGINES: [Engine; 2] = [Engine::MvTools, Engine::Rife];

/// The generated picture's rate and the rate the filters make of it: the
/// commonest case, film on a 60 Hz display.
pub const SOURCE_RATE: (u64, u64) = (24000, 1001);
pub const RATE: (u64, u64) = (60, 1);

/// The frames a second an engine must make over the ones a file shows.
pub const HEADROOM: f64 = 1.2;

/// What is measured: a new number measures again (the scripts, their
/// filter entries, the sizes or the rates changed).
pub const VERSION: u32 = 1;

/// The file in the state folder.
pub const FILE_NAME: &str = "calibration.json";

/// A filter whose rate stays below this share over the source's did not
/// run (as the drop watch judges, `smoothing::FilterWatch`).
const RAISED: f64 = 1.2;

/// A larger picture is still measured when the last one's rate, scaled by
/// the pixel counts, reaches this share of what a file needs: engines do
/// a little better on larger pictures than the pixel counts say.
const SCALED_SHARE: f64 = 0.8;

/// One measurement: the frames a second `engine` at `preset` made of a
/// `width` x `height` picture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Point {
    pub engine: Engine,
    pub preset: Preset,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

/// The calibration's results.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Calibration {
    pub version: u32,
    /// The machine it was measured on ([`key`]).
    pub key: String,
    /// When, as `YYYY-MM-DD` (local).
    pub date: String,
    pub points: Vec<Point>,
    /// Engines that did not run here (their filter never raised the rate).
    pub failed: Vec<Engine>,
}

/// Which engines and presets keep a file's rate, for the plan: one bit
/// per engine and preset measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fitting(u8);

impl Fitting {
    fn bit(engine: Engine, preset: Preset) -> Option<u8> {
        let row = ENGINES.iter().position(|known| *known == engine)?;
        let column = Preset::ALL.iter().position(|known| *known == preset)?;
        u8::try_from(row * Preset::ALL.len() + column).ok()
    }

    /// Whether `engine` at `preset` keeps the rate; engines the
    /// calibration does not measure (Blend, NVIDIA Optical Flow) do.
    #[must_use]
    pub fn fits(self, engine: Engine, preset: Preset) -> bool {
        Self::bit(engine, preset).is_none_or(|bit| self.0 & (1 << bit) != 0)
    }

    fn set(&mut self, engine: Engine, preset: Preset) {
        if let Some(bit) = Self::bit(engine, preset) {
            self.0 |= 1 << bit;
        }
    }
}

/// The pixels of a picture.
fn pixels(width: u32, height: u32) -> f64 {
    f64::from(width) * f64::from(height)
}

impl Calibration {
    /// Whether it was measured on the machine `key` describes, by this
    /// version of RVP's measurements.
    #[must_use]
    pub fn valid_for(&self, key: &str) -> bool {
        self.version == VERSION && self.key == key
    }

    /// The frames a second `engine` at `preset` makes of a `width` x
    /// `height` picture: the smallest size measured that is at least as
    /// large (no credit for a smaller picture), else the largest measured,
    /// scaled down by the pixel counts. `None` when nothing was measured.
    #[must_use]
    pub fn capacity(&self, engine: Engine, preset: Preset, width: u32, height: u32) -> Option<f64> {
        let wanted = pixels(width, height);
        let mut measured: Vec<&Point> = self
            .points
            .iter()
            .filter(|point| point.engine == engine && point.preset == preset)
            .collect();
        measured.sort_by(|a, b| pixels(a.width, a.height).total_cmp(&pixels(b.width, b.height)));
        if let Some(point) = measured
            .iter()
            .find(|point| pixels(point.width, point.height) >= wanted)
        {
            return Some(point.fps);
        }
        let largest = measured.last()?;
        Some(largest.fps * pixels(largest.width, largest.height) / wanted.max(1.0))
    }

    /// Which engines and presets keep `fps` frames a second of a `width`
    /// x `height` file with [`HEADROOM`].
    #[must_use]
    pub fn fitting(&self, width: u32, height: u32, fps: f64) -> Fitting {
        let mut fitting = Fitting::default();
        for engine in ENGINES {
            for preset in Preset::ALL {
                if !self.failed.contains(&engine)
                    && self
                        .capacity(engine, preset, width, height)
                        .is_some_and(|capacity| capacity >= HEADROOM * fps)
                {
                    fitting.set(engine, preset);
                }
            }
        }
        fitting
    }

    /// The largest measured size each engine and preset keeps at the
    /// measurement's rate, for the settings window and the log:
    /// "MVTools Quality up to 1440p, ...".
    #[must_use]
    pub fn summary(&self) -> String {
        let needed = HEADROOM * rate_fps(RATE);
        let mut parts = Vec::new();
        for engine in ENGINES {
            if self.failed.contains(&engine) {
                parts.push(format!("{} does not run here", engine.text()));
                continue;
            }
            for preset in Preset::ALL {
                let best = SIZES.iter().rev().find(|(width, height)| {
                    self.points.iter().any(|point| {
                        point.engine == engine
                            && point.preset == preset
                            && (point.width, point.height) == (*width, *height)
                            && point.fps >= needed
                    })
                });
                let measured = self
                    .points
                    .iter()
                    .any(|point| point.engine == engine && point.preset == preset);
                if !measured {
                    continue;
                }
                parts.push(match best {
                    Some((_, height)) => {
                        format!("{} {} up to {height}p", engine.text(), preset.text())
                    }
                    None => format!("{} {} below 480p", engine.text(), preset.text()),
                });
            }
        }
        parts.join(", ")
    }
}

/// A fraction's value.
#[must_use]
fn rate_fps(rate: (u64, u64)) -> f64 {
    // Rates are small numbers.
    #[allow(clippy::cast_precision_loss)]
    let fps = rate.0 as f64 / rate.1.max(1) as f64;
    fps
}

/// The next measurement after `done`: engine, preset and size, in the
/// order of [`ENGINES`], `Preset::ALL` and [`SIZES`]. An engine that is
/// missing or `failed` is left out; a larger size is left out once the
/// last one fell short of the rate, or would fall short of it scaled by
/// the pixel counts ([`SCALED_SHARE`]).
#[must_use]
pub fn next(
    done: &[Point],
    available: &[Engine],
    failed: &[Engine],
) -> Option<(Engine, Preset, (u32, u32))> {
    let needed = HEADROOM * rate_fps(RATE);
    for engine in ENGINES {
        if !available.contains(&engine) || failed.contains(&engine) {
            continue;
        }
        for preset in Preset::ALL {
            let mut last: Option<&Point> = None;
            for size in SIZES {
                let measured = done.iter().find(|point| {
                    point.engine == engine
                        && point.preset == preset
                        && (point.width, point.height) == size
                });
                if let Some(point) = measured {
                    last = Some(point);
                    continue;
                }
                if let Some(last) = last {
                    let scaled =
                        last.fps * pixels(last.width, last.height) / pixels(size.0, size.1);
                    if last.fps < needed || scaled < SCALED_SHARE * needed {
                        break;
                    }
                }
                return Some((engine, preset, size));
            }
        }
    }
    None
}

/// The number of measurements [`next`] can still ask for, at most: for
/// the progress shown.
#[must_use]
pub fn most_left(done: &[Point], available: &[Engine], failed: &[Engine]) -> usize {
    let engines = ENGINES
        .iter()
        .filter(|engine| available.contains(engine) && !failed.contains(engine))
        .count();
    (engines * Preset::ALL.len() * SIZES.len()).saturating_sub(done.len())
}

/// The machine's key: `hardware` (the processors, the graphics cards and
/// their drivers, from the shell), libmpv's version and the engines'
/// files (their sizes and times), with [`VERSION`].
#[must_use]
pub fn key(hardware: &str, mpv_version: &str, files: &[PathBuf]) -> String {
    let mut parts = vec![
        format!("v{VERSION}"),
        hardware.trim().to_owned(),
        mpv_version.trim().to_owned(),
    ];
    for file in files {
        let stamp = std::fs::metadata(file).ok().map(|meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |since| since.as_secs());
            format!("{}:{modified}", meta.len())
        });
        parts.push(format!(
            "{}={}",
            file.file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            stamp.unwrap_or_else(|| "missing".to_owned())
        ));
    }
    parts.join("; ")
}

/// Reads the calibration in `folder`; `None` when there is none or it
/// cannot be read.
#[must_use]
pub fn load(folder: &Path) -> Option<Calibration> {
    let text = std::fs::read_to_string(folder.join(FILE_NAME)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes `calibration` to `folder` (a temporary file renamed over the
/// old one).
///
/// # Errors
/// Fails when the file cannot be written.
pub fn save(folder: &Path, calibration: &Calibration) -> std::io::Result<()> {
    std::fs::create_dir_all(folder)?;
    let text = serde_json::to_string_pretty(calibration).map_err(std::io::Error::other)?;
    let temporary = folder.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, folder.join(FILE_NAME))
}

/// What a calibration needs: the machine's key, the engines and their
/// files, where to write its script and results.
#[derive(Debug, Clone)]
pub struct Job {
    pub key: String,
    pub date: String,
    /// The engines this machine has (of [`ENGINES`]).
    pub available: Vec<Engine>,
    /// RIFE's plugin and model, and MVTools' plugin (as the scripts take
    /// them).
    pub rife: Option<(PathBuf, PathBuf)>,
    pub mvtools: Option<PathBuf>,
    /// The state folder: the script goes to its `vapoursynth`, the
    /// results to [`FILE_NAME`].
    pub folder: PathBuf,
    /// How long each measurement warms up and measures.
    pub warm_up: Duration,
    pub measure: Duration,
}

/// What the runner tells the window's thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A measurement starts.
    Started {
        engine: Engine,
        preset: Preset,
        width: u32,
        height: u32,
    },
    /// A measurement: done and the most left.
    Measured {
        point: Point,
        done: usize,
        left: usize,
    },
    /// An engine that did not run, and why.
    Failed { engine: Engine, why: String },
    /// All measured and written (`Err`: why the file was not).
    Finished(Calibration, Result<(), String>),
    /// Stopped: a file started.
    Cancelled,
}

/// The calibration's thread.
pub struct Runner {
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Runner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Runner").finish_non_exhaustive()
    }
}

impl Runner {
    /// Starts measuring `job` on a thread of its own; returns at once.
    ///
    /// # Errors
    /// Fails when the thread cannot start.
    pub fn start(job: Job) -> std::io::Result<Self> {
        let (sender, events) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancel);
        let thread = std::thread::Builder::new()
            .name("rvp-calibration".to_owned())
            .spawn(move || run(&job, &stop, &sender))?;
        Ok(Self {
            events,
            cancel,
            thread: Some(thread),
        })
    }

    /// The events since the last call.
    #[must_use]
    pub fn take(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Asks the thread to stop; the measurement in progress ends within a
    /// twentieth of a second.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// The thread: measures until [`next`] has nothing left, then writes the
/// results.
fn run(job: &Job, cancel: &AtomicBool, sender: &Sender<Event>) {
    let script = job.folder.join("vapoursynth").join("rvp-calibrate.vpy");
    let mut calibration = Calibration {
        version: VERSION,
        key: job.key.clone(),
        date: job.date.clone(),
        points: Vec::new(),
        failed: Vec::new(),
    };
    let rife = job
        .rife
        .as_ref()
        .map(|(plugin, model)| (plugin.as_path(), model.as_path()));
    while let Some((engine, preset, (width, height))) =
        next(&calibration.points, &job.available, &calibration.failed)
    {
        if cancel.load(Ordering::Relaxed) {
            drop(sender.send(Event::Cancelled));
            return;
        }
        let text = crate::smoothing::script(
            engine,
            preset,
            SOURCE_RATE,
            RATE,
            rife,
            job.mvtools.as_deref(),
            0,
        );
        if let Err(error) = script
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&script, text))
        {
            drop(sender.send(Event::Failed {
                engine,
                why: format!("the script cannot be written: {error}"),
            }));
            calibration.failed.push(engine);
            continue;
        }
        drop(sender.send(Event::Started {
            engine,
            preset,
            width,
            height,
        }));
        let probe = rvp_engine::calibrate::Probe {
            width,
            height,
            rate: SOURCE_RATE,
            vf: crate::smoothing::filter_entry(engine, preset, RATE, &script),
            warm_up: job.warm_up,
            measure: job.measure,
            extra: Vec::new(),
        };
        match rvp_engine::calibrate::measure(&probe, cancel, Duration::from_secs(60)) {
            Ok(measurement) if measurement.filter_fps >= RAISED * rate_fps(SOURCE_RATE) => {
                let point = Point {
                    engine,
                    preset,
                    width,
                    height,
                    fps: measurement.speed * measurement.filter_fps,
                };
                calibration.points.push(point.clone());
                drop(sender.send(Event::Measured {
                    point,
                    done: calibration.points.len(),
                    left: most_left(&calibration.points, &job.available, &calibration.failed),
                }));
            }
            Ok(measurement) => {
                calibration.failed.push(engine);
                drop(sender.send(Event::Failed {
                    engine,
                    why: format!(
                        "its filter did not raise the rate ({:.3} fps)",
                        measurement.filter_fps
                    ),
                }));
            }
            Err(_) if cancel.load(Ordering::Relaxed) => {
                drop(sender.send(Event::Cancelled));
                return;
            }
            Err(error) => {
                calibration.failed.push(engine);
                drop(sender.send(Event::Failed {
                    engine,
                    why: error.to_string(),
                }));
            }
        }
    }
    let written = save(&job.folder, &calibration).map_err(|error| error.to_string());
    drop(sender.send(Event::Finished(calibration, written)));
}

/// How long nothing has to play before a calibration starts on its own.
const IDLE: Duration = Duration::from_secs(5);

/// The calibration's state in the [`App`].
#[derive(Debug, Default)]
pub(crate) struct State {
    /// The machine's description from the shell (`None` until told).
    hardware: Option<String>,
    /// libmpv's version, read when the player's core started.
    mpv_version: String,
    /// The results in force: measured on this machine.
    results: Option<Calibration>,
    /// The thread while it measures, its progress (done, most left) and
    /// the measurement it is at.
    runner: Option<Runner>,
    progress: (usize, usize),
    current: Option<String>,
    /// Since when nothing plays.
    idle_since: Option<Instant>,
    /// The calibration window asked for a run.
    asked: bool,
    /// The calibration window cancelled one: no run on its own until
    /// asked again.
    declined: bool,
    /// Runs only when asked (self-tests).
    manual: bool,
}

use std::time::Instant;

impl App {
    /// The player's core started: libmpv's version is part of the key.
    pub(crate) fn calibration_started(&mut self, engine: &Mpv) {
        self.calibration.mpv_version = engine.get_text("mpv-version").unwrap_or_default();
    }

    /// The machine's description (the processors, the graphics cards and
    /// their drivers), told by the shell once at start; the results
    /// measured on this machine are read from the state folder.
    pub fn set_hardware(&mut self, hardware: String) {
        self.calibration.hardware = Some(hardware);
        let key = self.calibration_key();
        self.calibration.results = key.and_then(|key| {
            load(self.state_dir()).filter(|calibration| calibration.valid_for(&key))
        });
        if let Some(results) = &self.calibration.results {
            crate::log::write(
                crate::log::Level::Info,
                &format!("calibration of {}: {}", results.date, results.summary()),
            );
        }
    }

    /// A self-test measures only when asked (`--calibrate`).
    pub fn set_calibration_manual(&mut self, manual: bool) {
        self.calibration.manual = manual;
    }

    /// The key of this machine's calibration: `None` until the shell has
    /// described it.
    fn calibration_key(&self) -> Option<String> {
        let hardware = self.calibration.hardware.as_deref()?;
        let runtime = &self.smoothing.runtime;
        let mut files: Vec<PathBuf> = Vec::new();
        files.extend(runtime.mvtools_plugin.iter().cloned());
        if let Some((plugin, model)) = runtime.rife_files() {
            files.push(plugin.to_path_buf());
            files.push(model.to_path_buf());
        }
        Some(key(hardware, &self.calibration.mpv_version, &files))
    }

    /// The engines a calibration can measure here.
    fn calibration_engines(&self) -> Vec<Engine> {
        let runtime = &self.smoothing.runtime;
        ENGINES
            .into_iter()
            .filter(|engine| match engine {
                Engine::MvTools => runtime.has_mvtools(),
                Engine::Rife => runtime.rife_files().is_some(),
                _ => false,
            })
            .collect()
    }

    /// What keeps the rate of a `width` x `height` file by the
    /// calibration, at the rate smoothing's target asks for (`None`: not
    /// calibrated, or nothing to judge).
    pub(crate) fn calibration_fitting(
        &self,
        width: u32,
        height: u32,
        source_fps: f64,
        display_fps: f64,
    ) -> Option<Fitting> {
        let results = self.calibration.results.as_ref()?;
        let rate = crate::smoothing::target_rate(
            &self.settings.smoothing.target,
            source_fps,
            display_fps,
        )?;
        (height > 0).then(|| results.fitting(width.max(1), height, rate_fps(rate)))
    }

    /// Whether a calibration is to run: smoothing is on, an engine can be
    /// measured, and this machine has none (or the settings window asked).
    fn calibration_due(&self) -> bool {
        let state = &self.calibration;
        state.hardware.is_some()
            && self.settings.smoothing.enabled
            && !self.calibration_engines().is_empty()
            && (state.asked || (!state.manual && !state.declined && state.results.is_none()))
    }

    /// The calibration window's Start: pauses playback and measures at the
    /// next tick.
    pub fn calibrate_now(&mut self, engine: &Mpv) {
        self.calibration.asked = true;
        self.calibration.declined = false;
        if self.model.playing() {
            drop(engine.set_pause(true));
        }
    }

    /// The command line's `--calibrate`: measures as soon as nothing plays.
    pub fn calibrate_at_start(&mut self) {
        self.calibration.asked = true;
    }

    /// The calibration window's Cancel: stops the run; none starts on its
    /// own until Start.
    pub fn cancel_calibration(&mut self) {
        self.calibration.asked = false;
        self.calibration.declined = true;
        if let Some(runner) = &self.calibration.runner {
            runner.cancel();
        }
    }

    /// Once a second: the runner's news, a stop when a file plays, a
    /// start after a moment when nothing does. A finished calibration
    /// plans the file that is open again.
    pub fn calibration_tick(&mut self, engine: &Mpv, now: Instant) -> Option<UiRequest> {
        let events = self
            .calibration
            .runner
            .as_ref()
            .map(Runner::take)
            .unwrap_or_default();
        let mut finished = false;
        for event in events {
            finished |= self.calibration_event(event);
        }
        if finished {
            self.apply_smoothing(engine);
            return Some(UiRequest::Osd(format!(
                "Motion smoothing calibrated: {}",
                self.calibration
                    .results
                    .as_ref()
                    .map(Calibration::summary)
                    .unwrap_or_default()
            )));
        }
        if self.model.playing() {
            self.calibration.idle_since = None;
            if let Some(runner) = &self.calibration.runner {
                runner.cancel();
            }
            return None;
        }
        let since = *self.calibration.idle_since.get_or_insert(now);
        if self.calibration.runner.is_some()
            || !self.calibration_due()
            || (!self.calibration.asked && now.duration_since(since) < IDLE)
        {
            return None;
        }
        self.start_calibration()
    }

    /// Starts the runner; a notice when it cannot start.
    fn start_calibration(&mut self) -> Option<UiRequest> {
        let key = self.calibration_key()?;
        let runtime = &self.smoothing.runtime;
        let job = Job {
            key,
            date: crate::log::local_time().chars().take(10).collect(),
            available: self.calibration_engines(),
            rife: runtime
                .rife_files()
                .map(|(plugin, model)| (plugin.to_path_buf(), model.to_path_buf())),
            mvtools: runtime.mvtools_plugin.clone(),
            folder: self.state_dir().to_path_buf(),
            warm_up: Duration::from_millis(1500),
            measure: Duration::from_millis(2500),
        };
        self.calibration.asked = false;
        match Runner::start(job) {
            Ok(runner) => {
                crate::log::write(
                    crate::log::Level::Info,
                    "calibration: measuring the smoothing engines while nothing plays",
                );
                self.calibration.runner = Some(runner);
                self.calibration.progress = (0, 0);
                Some(UiRequest::Osd(
                    "Motion smoothing: calibrating while nothing plays".to_owned(),
                ))
            }
            Err(error) => {
                let text = format!("Motion smoothing: the calibration cannot start: {error}");
                crate::log::report(crate::log::Level::Warning, &text);
                Some(UiRequest::Message(text))
            }
        }
    }

    /// One event of the runner; `true` when the calibration finished.
    fn calibration_event(&mut self, event: Event) -> bool {
        match event {
            Event::Started {
                engine,
                preset,
                width,
                height,
            } => {
                self.calibration.current = Some(format!(
                    "{} {} {width}x{height}",
                    engine.text(),
                    preset.text()
                ));
                false
            }
            Event::Measured { point, done, left } => {
                crate::log::write(
                    crate::log::Level::Info,
                    &format!(
                        "calibration: {} {} at {}x{}: {:.0} frames a second",
                        point.engine.text(),
                        point.preset.text(),
                        point.width,
                        point.height,
                        point.fps
                    ),
                );
                self.calibration.progress = (done, left);
                false
            }
            Event::Failed { engine, why } => {
                crate::log::write(
                    crate::log::Level::Warning,
                    &format!("calibration: {} did not run: {why}", engine.text()),
                );
                false
            }
            Event::Finished(calibration, written) => {
                if let Err(error) = written {
                    crate::log::report(
                        crate::log::Level::Warning,
                        &format!("calibration: {FILE_NAME} cannot be written: {error}"),
                    );
                }
                crate::log::write(
                    crate::log::Level::Info,
                    &format!("calibration: done: {}", calibration.summary()),
                );
                self.calibration.results = Some(calibration);
                self.calibration.runner = None;
                self.calibration.current = None;
                true
            }
            Event::Cancelled => {
                crate::log::write(crate::log::Level::Info, "calibration: stopped");
                self.calibration.runner = None;
                self.calibration.current = None;
                false
            }
        }
    }

    /// The calibration for the settings window: its state, date, the
    /// largest size each engine and preset keeps at 60 frames a second
    /// (`height` 0: below 480p), the engines that did not run, and the
    /// progress of a run.
    #[must_use]
    pub fn calibration_status(&self) -> serde_json::Value {
        let state = &self.calibration;
        let mut rows = Vec::new();
        let mut failed = Vec::new();
        if let Some(results) = &state.results {
            let needed = HEADROOM * rate_fps(RATE);
            for engine in ENGINES {
                if results.failed.contains(&engine) {
                    failed.push(engine.text());
                    continue;
                }
                for preset in Preset::ALL {
                    let points: Vec<&Point> = results
                        .points
                        .iter()
                        .filter(|point| point.engine == engine && point.preset == preset)
                        .collect();
                    if points.is_empty() {
                        continue;
                    }
                    let height = points
                        .iter()
                        .filter(|point| point.fps >= needed)
                        .map(|point| point.height)
                        .max()
                        .unwrap_or(0);
                    rows.push(serde_json::json!({
                        "engine": engine.text(),
                        "preset": preset.text(),
                        "height": height,
                    }));
                }
            }
        }
        let status = if state.runner.is_some() {
            "running"
        } else if state.results.is_some() {
            "measured"
        } else if self.calibration_engines().is_empty() {
            "unavailable"
        } else if state.declined {
            "cancelled"
        } else {
            "waiting"
        };
        serde_json::json!({
            "state": status,
            "date": state.results.as_ref().map(|results| results.date.clone()),
            "rows": rows,
            "failed": failed,
            "done": state.progress.0,
            "left": state.progress.1,
            "current": state.current,
            "playing": self.model.playing(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(engine: Engine, preset: Preset, size: (u32, u32), fps: f64) -> Point {
        Point {
            engine,
            preset,
            width: size.0,
            height: size.1,
            fps,
        }
    }

    /// The measurements [`next`] asks for when each engine and preset
    /// makes `fps(engine, preset, size)` frames a second.
    fn order(
        available: &[Engine],
        fps: impl Fn(Engine, Preset, (u32, u32)) -> f64,
    ) -> Vec<(Engine, Preset, u32)> {
        let mut done = Vec::new();
        let mut asked = Vec::new();
        while let Some((engine, preset, size)) = next(&done, available, &[]) {
            asked.push((engine, preset, size.1));
            done.push(point(engine, preset, size, fps(engine, preset, size)));
            assert!(asked.len() <= 24, "{asked:?}");
        }
        asked
    }

    #[test]
    fn larger_pictures_are_measured_only_while_they_could_keep_the_rate() {
        // The host's RIFE: 113 frames a second at 720p, about 2.25 times
        // fewer at 1080p (50 scaled, under 0.8 of the 72 needed): 1080p is
        // not measured, nor anything larger.
        let rife =
            |_: Engine, _: Preset, size: (u32, u32)| 113.0 * 921_600.0 / pixels(size.0, size.1);
        assert_eq!(
            order(&[Engine::Rife], rife),
            [
                (Engine::Rife, Preset::Fast, 480),
                (Engine::Rife, Preset::Fast, 720),
                (Engine::Rife, Preset::Medium, 480),
                (Engine::Rife, Preset::Medium, 720),
                (Engine::Rife, Preset::Quality, 480),
                (Engine::Rife, Preset::Quality, 720),
            ]
        );
        // The test VM's two processors: about 80 frames a second at 480p,
        // 36 scaled to 720p, under 0.8 of the 72 needed: each preset stops
        // at 480p.
        let vm = |_: Engine, _: Preset, size: (u32, u32)| 80.0 * 409_920.0 / pixels(size.0, size.1);
        assert_eq!(order(&[Engine::MvTools], vm).len(), 3);
        // A strong CPU: 600 at 720p goes on to 2160p, measured once
        // there although its 67 falls short of 72: 1440p's 150, scaled by
        // the pixels, reached 0.8 of it.
        let mvtools =
            |_: Engine, _: Preset, size: (u32, u32)| 600.0 * 921_600.0 / pixels(size.0, size.1);
        let asked = order(&[Engine::MvTools], mvtools);
        assert_eq!(asked.len(), 15);
        assert!(asked.contains(&(Engine::MvTools, Preset::Quality, 2160)));
        // Nothing is asked of a missing engine, nor of a failed one.
        assert_eq!(next(&[], &[], &[]), None);
        assert_eq!(next(&[], &[Engine::Rife], &[Engine::Rife]), None);
    }

    #[test]
    fn a_file_fits_what_the_next_larger_measured_size_keeps() {
        let calibration = Calibration {
            version: VERSION,
            key: "machine".to_owned(),
            date: "2026-11-07".to_owned(),
            points: vec![
                point(Engine::MvTools, Preset::Quality, (1280, 720), 300.0),
                point(Engine::MvTools, Preset::Quality, (1920, 1080), 140.0),
                point(Engine::MvTools, Preset::Quality, (2560, 1440), 70.0),
                point(Engine::Rife, Preset::Quality, (1280, 720), 113.0),
            ],
            failed: Vec::new(),
        };
        // A 1920x800 film counts as 1080p (no credit for being smaller).
        assert_eq!(
            calibration.capacity(Engine::MvTools, Preset::Quality, 1920, 800),
            Some(140.0)
        );
        // Larger than all measured: the largest, scaled by the pixels.
        let scaled = calibration
            .capacity(Engine::MvTools, Preset::Quality, 3840, 2160)
            .expect("scaled");
        assert!((scaled - 70.0 * 3_686_400.0 / 8_294_400.0).abs() < 1e-9);
        let fitting = calibration.fitting(1920, 1080, 59.94);
        assert!(fitting.fits(Engine::MvTools, Preset::Quality));
        assert!(!fitting.fits(Engine::Rife, Preset::Quality));
        // Not measured: does not fit; not calibrated at all: fits.
        assert!(!fitting.fits(Engine::MvTools, Preset::Fast));
        assert!(fitting.fits(Engine::FrucVulkan, Preset::Quality));
        assert!(fitting.fits(Engine::Blend, Preset::Fast));
        // 120 Hz asks for 144 frames a second at 1080p: 140 is short.
        assert!(
            !calibration
                .fitting(1920, 1080, 120.0)
                .fits(Engine::MvTools, Preset::Quality)
        );
        assert!(
            calibration
                .fitting(1280, 720, 120.0)
                .fits(Engine::MvTools, Preset::Quality)
        );
        assert_eq!(
            calibration.summary(),
            "MVTools Quality up to 1080p, RIFE Quality up to 720p"
        );
        // Another machine or version needs a new calibration.
        assert!(calibration.valid_for("machine"));
        assert!(!calibration.valid_for("other"));
        assert!(
            !Calibration {
                version: VERSION + 1,
                ..calibration.clone()
            }
            .valid_for("machine")
        );
    }

    #[test]
    fn a_failed_engine_fits_nothing_and_says_so() {
        let calibration = Calibration {
            version: VERSION,
            key: "machine".to_owned(),
            points: vec![point(Engine::MvTools, Preset::Fast, (1280, 720), 500.0)],
            failed: vec![Engine::Rife],
            ..Calibration::default()
        };
        let fitting = calibration.fitting(1280, 720, 60.0);
        assert!(fitting.fits(Engine::MvTools, Preset::Fast));
        assert!(!fitting.fits(Engine::Rife, Preset::Fast));
        assert_eq!(
            calibration.summary(),
            "MVTools Fast up to 720p, RIFE does not run here"
        );
    }

    #[test]
    fn the_results_are_kept_in_the_state_folder() {
        let folder = std::env::temp_dir().join(format!("rvp-calibration-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&folder));
        assert_eq!(load(&folder), None);
        let calibration = Calibration {
            version: VERSION,
            key: "machine".to_owned(),
            date: "2026-11-07".to_owned(),
            points: vec![point(Engine::Rife, Preset::Medium, (1920, 1080), 61.5)],
            failed: vec![Engine::MvTools],
        };
        save(&folder, &calibration).expect("save");
        assert_eq!(load(&folder), Some(calibration));
        drop(std::fs::remove_dir_all(&folder));
    }

    #[test]
    fn the_key_changes_with_the_machine_and_the_engines() {
        let folder =
            std::env::temp_dir().join(format!("rvp-calibration-key-{}", std::process::id()));
        drop(std::fs::create_dir_all(&folder));
        let file = folder.join("plugin.dll");
        std::fs::write(&file, b"one").expect("write");
        let first = key("CPU A; GPU B 1.0", "mpv 0.41", std::slice::from_ref(&file));
        assert_eq!(
            first,
            key("CPU A; GPU B 1.0", "mpv 0.41", std::slice::from_ref(&file))
        );
        assert_ne!(
            first,
            key("CPU A; GPU B 2.0", "mpv 0.41", std::slice::from_ref(&file))
        );
        std::fs::write(&file, b"longer").expect("write");
        assert_ne!(
            first,
            key("CPU A; GPU B 1.0", "mpv 0.41", std::slice::from_ref(&file))
        );
        assert!(key("x", "y", &[folder.join("gone")]).ends_with("gone=missing"));
        drop(std::fs::remove_dir_all(&folder));
    }
}
