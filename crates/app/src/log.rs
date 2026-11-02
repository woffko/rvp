//! RVP's log (the Log page; `log.level` in settings.json, off by default):
//! what RVP and mpv report, at the level the user chose, in [`FILE_NAME`]
//! in the state folder, for finding out afterwards why playback stuttered,
//! motion smoothing stepped down or a file failed.
//!
//! - Critical: errors (a file that cannot play, settings or state that
//!   cannot be saved, mpv's errors).
//! - Warning: RVP or mpv working around a problem (smoothing stepping
//!   down, the GPU power budget's steps, mpv's warnings).
//! - Info: what happens (files, changed settings, the smoothing's plans,
//!   seconds of playback that dropped frames, mpv's messages).
//! - Debug: every second of playback (the rates, dropped and late frames,
//!   the frames drawn, the processors' load) and mpv's verbose messages.
//!
//! Lines are made where things happen, with the local time then, and a
//! thread of the log's own writes them: no caller waits for the disk.
//! mpv's messages come on a weak client handle of the player's core, read
//! on another thread ([`MpvLog`]), never through the window's thread. At
//! [`ROTATE_AT`] the file moves to [`OLD_FILE_NAME`], when it opens and
//! while it is written.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use rvp_engine::libmpv2::Mpv;
use rvp_engine::libmpv2::events::Event;
use rvp_engine::{Engine, PropertyId};
use serde::{Deserialize, Serialize};

use crate::App;

/// The log's file in the state folder.
pub const FILE_NAME: &str = "rvp.log";
/// The file before the last rotation.
pub const OLD_FILE_NAME: &str = "rvp.old.log";
/// The size at which the file moves aside.
const ROTATE_AT: u64 = 16 * 1024 * 1024;
/// Lines kept for the Log page and self-tests.
const RECENT: usize = 1000;
/// How long [`MpvLog`]'s thread waits for an event before it looks
/// whether it is to stop.
const WAIT: f64 = 0.25;

/// The levels of `log.level`, the first one off.
pub const LEVELS: &[&str] = &["off", "critical", "warning", "info", "debug"];

/// How much a line matters; a level writes its own lines and the ones
/// above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Critical = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
}

impl Level {
    /// The level of a `log.level` value: `None` when the log is off.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "critical" => Some(Self::Critical),
            "warning" => Some(Self::Warning),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Critical => "CRIT ",
            Self::Warning => "WARN ",
            Self::Info => "INFO ",
            Self::Debug => "DEBUG",
        }
    }

    /// mpv's log level that gives this level's messages.
    fn mpv(self) -> &'static str {
        match self {
            Self::Critical => "error",
            Self::Warning => "warn",
            Self::Info => "info",
            Self::Debug => "v",
        }
    }

    /// The level of a message mpv sent at `level`.
    fn of_mpv(level: &str) -> Self {
        match level {
            "fatal" | "error" => Self::Critical,
            "warn" => Self::Warning,
            "info" => Self::Info,
            _ => Self::Debug,
        }
    }
}

/// The Log page (`log` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LogSettings {
    /// One of [`LEVELS`].
    pub level: String,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            level: "off".to_owned(),
        }
    }
}

impl LogSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !LEVELS.contains(&self.level.as_str()) {
            notes.push(format!(
                "log.level {:?} is not one of {}: off used",
                self.level,
                LEVELS.join(", ")
            ));
            "off".clone_into(&mut self.level);
        }
    }
}

/// The level in force (a [`Level`]; 0 while the log is off).
static LEVEL: AtomicU8 = AtomicU8::new(0);
/// The local time's offset from UTC in seconds (the window's clock).
static UTC_OFFSET: AtomicI32 = AtomicI32::new(0);
/// Lines logged, and of them mpv's, since RVP started (self-tests).
static LINES: AtomicU64 = AtomicU64::new(0);
static MPV_LINES: AtomicU64 = AtomicU64::new(0);
/// The writing thread's end of the channel and the file it writes.
static SINK: Mutex<Option<Sink>> = Mutex::new(None);
/// The last [`RECENT`] lines.
static RECENT_LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

struct Sink {
    sender: Option<Sender<String>>,
    thread: Option<std::thread::JoinHandle<()>>,
    path: PathBuf,
}

impl Drop for Sink {
    fn drop(&mut self) {
        // The thread writes what it has, closes the file and ends.
        drop(self.sender.take());
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// True when lines of `level` are written: check it before making a line
/// that costs something.
#[must_use]
pub fn enabled(level: Level) -> bool {
    LEVEL.load(Ordering::Relaxed) >= level as u8
}

/// Logs `text` at `level`, when the log takes that level.
pub fn write(level: Level, text: &str) {
    if !enabled(level) {
        return;
    }
    let line = format!("{} {} {}\n", local_time(), level.tag(), text.trim_end());
    LINES.fetch_add(1, Ordering::Relaxed);
    {
        let mut recent = RECENT_LINES.lock().unwrap_or_else(PoisonError::into_inner);
        if recent.len() == RECENT {
            recent.pop_front();
        }
        recent.push_back(line.trim_end().to_owned());
    }
    let sink = SINK.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(sender) = sink.as_ref().and_then(|sink| sink.sender.as_ref()) {
        // A thread that stopped (a full disk) leaves the line unwritten.
        drop(sender.send(line));
    }
}

/// One of RVP's own messages: to the standard error, as always, and to
/// the log.
pub fn report(level: Level, text: &str) {
    eprintln!("rvp: {text}");
    write(level, text);
}

/// The local time's offset from UTC (the window's clock gives it; without
/// a window the times are UTC).
pub fn set_utc_offset(seconds: i32) {
    UTC_OFFSET.store(seconds, Ordering::Relaxed);
}

/// The last lines logged, oldest first (the Log page).
#[must_use]
pub fn recent() -> String {
    let recent = RECENT_LINES.lock().unwrap_or_else(PoisonError::into_inner);
    recent
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Lines logged since RVP started, and how many of them were mpv's.
#[must_use]
pub fn counts() -> (u64, u64) {
    (
        LINES.load(Ordering::Relaxed),
        MPV_LINES.load(Ordering::Relaxed),
    )
}

/// Logs at `level` into `path` from now on, or nothing (`None`). Returns
/// true when the file was opened now.
///
/// # Errors
/// Fails when the file cannot be opened; the log is off then.
pub fn configure(level: Option<Level>, path: &Path) -> Result<bool, String> {
    let mut sink = SINK.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(level) = level else {
        LEVEL.store(0, Ordering::Relaxed);
        *sink = None;
        return Ok(false);
    };
    let opened = if sink.as_ref().is_some_and(|sink| sink.path == path) {
        false
    } else {
        *sink = None;
        match open(path) {
            Ok(opened) => *sink = Some(opened),
            Err(error) => {
                LEVEL.store(0, Ordering::Relaxed);
                return Err(error);
            }
        }
        true
    };
    LEVEL.store(level as u8, Ordering::Relaxed);
    Ok(opened)
}

fn open(path: &Path) -> Result<Sink, String> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() >= ROTATE_AT) {
        move_aside(path);
    }
    let file = append(path)?;
    let size = file.metadata().map_or(0, |meta| meta.len());
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread_path = path.to_path_buf();
    let thread = std::thread::Builder::new()
        .name("rvp-log".to_owned())
        .spawn(move || run(&receiver, &thread_path, file, size))
        .map_err(|e| format!("the log's thread: {e}"))?;
    Ok(Sink {
        sender: Some(sender),
        thread: Some(thread),
        path: path.to_path_buf(),
    })
}

fn append(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Moves the file to [`OLD_FILE_NAME`] beside it, over the older one.
fn move_aside(path: &Path) {
    drop(std::fs::rename(path, path.with_file_name(OLD_FILE_NAME)));
}

/// The writing thread: each batch of lines, then a flush, so the file is
/// current whenever someone looks; it ends when the log is turned off.
fn run(receiver: &Receiver<String>, path: &Path, file: File, mut size: u64) {
    let mut out = BufWriter::new(file);
    while let Ok(line) = receiver.recv() {
        let mut next = Some(line);
        while let Some(line) = next {
            if out.write_all(line.as_bytes()).is_err() {
                return;
            }
            size += line.len() as u64;
            next = receiver.try_recv().ok();
        }
        if out.flush().is_err() {
            return;
        }
        if size >= ROTATE_AT {
            // Windows moves no open file.
            drop(out);
            move_aside(path);
            let Ok(file) = append(path) else {
                return;
            };
            out = BufWriter::new(file);
            size = 0;
        }
    }
    drop(out.flush());
}

/// Now in local time as `YYYY-MM-DD HH:MM:SS.mmm`.
fn local_time() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = i64::try_from(since.as_millis()).unwrap_or(0)
        + i64::from(UTC_OFFSET.load(Ordering::Relaxed)) * 1000;
    time_text(millis)
}

/// Milliseconds since 1970 as `YYYY-MM-DD HH:MM:SS.mmm`.
fn time_text(millis: i64) -> String {
    let (days, of_day) = (millis.div_euclid(86_400_000), millis.rem_euclid(86_400_000));
    let (year, month, day) = crate::capture::civil_from_days(days);
    let seconds = of_day / 1000;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}.{:03}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        of_day % 1000
    )
}

/// mpv's messages for the log: a weak client handle of the player's core,
/// read on a thread of its own at the log's level.
#[derive(Debug)]
pub(crate) struct MpvLog {
    level: Level,
    stop: Arc<AtomicBool>,
}

impl MpvLog {
    /// Starts reading `engine`'s messages of `level` and above.
    ///
    /// # Errors
    /// Fails when libmpv refuses the handle or its log messages.
    pub(crate) fn start(engine: &Engine, level: Level) -> Result<Self, String> {
        let client = engine
            .mpv()
            .create_weak_client(Some("rvp_log"))
            .map_err(|e| format!("the log's mpv client: {e}"))?;
        client
            .request_log_messages(level.mpv())
            .map_err(|e| format!("mpv's log: {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("rvp-mpv-log".to_owned())
            .spawn(move || read_mpv(&client, &thread_stop))
            .map_err(|e| format!("the mpv log's thread: {e}"))?;
        Ok(Self { level, stop })
    }
}

impl Drop for MpvLog {
    fn drop(&mut self) {
        // The thread notices within `WAIT` and drops its handle (mpv waits
        // for weak handles when it ends a core).
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn read_mpv(client: &Mpv, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        match client.wait_event(WAIT) {
            Some(Ok(Event::Shutdown)) => return,
            Some(Ok(Event::LogMessage {
                prefix,
                level,
                text,
                ..
            })) => {
                // RVP's own meters (the normalizer's and the spectrum's
                // blocks), many a second.
                if prefix == "ffmpeg" && text.starts_with("ametadata@rvp") {
                    continue;
                }
                MPV_LINES.fetch_add(1, Ordering::Relaxed);
                write(Level::of_mpv(level), &format!("mpv [{prefix}] {text}"));
            }
            _ => {}
        }
    }
}

/// The frame counters of the file that plays, as last logged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    dropped: i64,
    decoder: i64,
    late: i64,
    mistimed: i64,
}

impl Counts {
    /// The counts since `before`; a counter that went back started again
    /// (a new file) and counts from zero.
    fn since(self, before: Self) -> Self {
        let delta = |now: i64, then: i64| if now >= then { now - then } else { now };
        Self {
            dropped: delta(self.dropped, before.dropped),
            decoder: delta(self.decoder, before.decoder),
            late: delta(self.late, before.late),
            mistimed: delta(self.mistimed, before.mistimed),
        }
    }
}

/// The shell's readings of the last second for [`App::log_second`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Second {
    /// Video frames the window drew and the longest time between two of
    /// them in milliseconds (the OpenGL renderer: mpv draws into its own
    /// window with the others).
    pub drawn: Option<(u32, u32)>,
    /// The share of the processors' time that was busy, and that RVP took
    /// (0 to 1).
    pub processors: Option<(f64, f64)>,
}

/// The log's state in the [`App`].
#[derive(Debug, Default)]
pub(crate) struct LogState {
    /// mpv's messages ([`MpvLog`]).
    mpv: Option<MpvLog>,
    /// The counters as the last second left them.
    counts: Option<Counts>,
    /// The file's video was described.
    video_told: bool,
}

impl App {
    /// The Log page's level in force: opens or closes the file, and asks
    /// mpv for its messages of that level.
    pub(crate) fn apply_log(&mut self, engine: &Engine) {
        let level = Level::parse(&self.settings.log.level);
        let path = self.state_dir().join(FILE_NAME);
        match configure(level, &path) {
            Ok(true) => self.log_opened(),
            Ok(false) => {}
            Err(error) => {
                self.warnings.push(format!("Log: {error}"));
                return;
            }
        }
        let Some(level) = level else {
            self.log.mpv = None;
            return;
        };
        if self.log.mpv.as_ref().map(|mpv| mpv.level) != Some(level) {
            self.log.mpv = None;
            match MpvLog::start(engine, level) {
                Ok(mpv) => self.log.mpv = Some(mpv),
                Err(error) => report(Level::Warning, &error),
            }
        }
    }

    /// The first lines of a log: RVP, the machine and what plays video.
    fn log_opened(&self) {
        let offset = UTC_OFFSET.load(Ordering::Relaxed);
        write(
            Level::Critical,
            &format!(
                "RVP {} on {} {}, {} processors; times are UTC{}{:02}:{:02}; level {}",
                crate::crash::version_text(),
                std::env::consts::OS,
                std::env::consts::ARCH,
                std::thread::available_parallelism().map_or(0, std::num::NonZero::get),
                if offset < 0 { '-' } else { '+' },
                offset.unsigned_abs() / 3600,
                offset.unsigned_abs() / 60 % 60,
                self.settings.log.level
            ),
        );
        let settings = &self.settings;
        let smoothing = &settings.smoothing;
        write(
            Level::Info,
            &format!(
                "video: renderer {:?} ({}), hwdec {}, video-sync {}, screen {:.3} Hz; smoothing {}: {} {} at {}, motion compensation up to {}p, steps down after {}; here: {}",
                settings.render.renderer,
                if self.native_video {
                    "mpv's own window"
                } else {
                    "drawn by the window"
                },
                settings.hwdec,
                settings.video_sync,
                self.picture.screen.refresh,
                if smoothing.enabled { "on" } else { "off" },
                smoothing.engine.text(),
                smoothing.preset.text(),
                smoothing.target,
                smoothing.max_height,
                match smoothing.drop_window {
                    0 => "never".to_owned(),
                    seconds => format!("{seconds} s of dropped frames"),
                },
                self.smoothing_engines()
            ),
        );
    }

    /// Changed settings, each path with its old and new value (Info).
    pub(crate) fn log_settings_changed(old: &crate::Settings, new: &crate::Settings) {
        if !enabled(Level::Info) {
            return;
        }
        let (Ok(old), Ok(new)) = (serde_json::to_value(old), serde_json::to_value(new)) else {
            return;
        };
        let mut changed = Vec::new();
        changed_paths(&old, &new, "", &mut changed);
        for (path, before, after) in changed {
            write(Level::Info, &format!("setting {path}: {before} -> {after}"));
        }
    }

    /// The engine's events of a drain worth a line: files, their ends and
    /// errors.
    pub(crate) fn log_events(&mut self, events: &[rvp_engine::EngineEvent]) {
        use rvp_engine::{EndReason, EngineEvent};
        if !enabled(Level::Critical) {
            return;
        }
        for event in events {
            match event {
                EngineEvent::FileLoaded { path } => {
                    self.log.counts = None;
                    self.log.video_told = false;
                    write(Level::Info, &format!("file: {path}"));
                }
                EngineEvent::EndFile {
                    reason: EndReason::Error,
                    path,
                } => write(
                    Level::Critical,
                    &format!("cannot play {}", path.as_deref().unwrap_or("the file")),
                ),
                EngineEvent::EndFile { reason, path } => write(
                    Level::Info,
                    &format!(
                        "end of {} ({reason:?}): {} frames dropped, {} by the decoder",
                        path.as_deref().unwrap_or("the file"),
                        self.model.int(PropertyId::FrameDropCount).unwrap_or(0),
                        self.model
                            .int(PropertyId::DecoderFrameDropCount)
                            .unwrap_or(0)
                    ),
                ),
                EngineEvent::Error(message) => {
                    write(Level::Critical, &format!("cannot play: {message}"));
                }
                _ => {}
            }
        }
    }

    /// The video's properties once a file shows them, and each change of
    /// mpv's video filters (Info).
    pub(crate) fn log_video(&mut self, changes: rvp_engine::Changes) {
        if !enabled(Level::Info) {
            return;
        }
        let model = &self.model;
        if !self.log.video_told
            && let (Some(width), Some(height), Some(fps)) = (
                model.int(PropertyId::VideoWidth),
                model.int(PropertyId::VideoHeight),
                model.double(PropertyId::ContainerFps),
            )
        {
            self.log.video_told = true;
            write(
                Level::Info,
                &format!(
                    "video: {width}x{height}, {fps:.3} fps, decoded by {}",
                    model
                        .text(PropertyId::HwdecCurrent)
                        .filter(|hwdec| !hwdec.is_empty() && *hwdec != "no")
                        .unwrap_or("the processor")
                ),
            );
        }
        if changes.contains(PropertyId::VideoFilters) {
            write(
                Level::Info,
                &format!(
                    "video filters: {}",
                    model.text(PropertyId::VideoFilters).unwrap_or("")
                ),
            );
        }
    }

    /// A second of playback: every one at Debug, those that dropped or
    /// delayed frames at Info. `second` has the shell's readings.
    pub fn log_second(&mut self, second: &Second) {
        if !enabled(Level::Info) || !self.model.playing() {
            return;
        }
        let model = &self.model;
        let counts = Counts {
            dropped: model.int(PropertyId::FrameDropCount).unwrap_or(0),
            decoder: model.int(PropertyId::DecoderFrameDropCount).unwrap_or(0),
            late: model.int(PropertyId::VoDelayedFrameCount).unwrap_or(0),
            mistimed: model.int(PropertyId::MistimedFrameCount).unwrap_or(0),
        };
        let delta = counts.since(self.log.counts.unwrap_or(counts));
        self.log.counts = Some(counts);
        let level = if delta.dropped > 0 || delta.decoder > 0 || delta.late > 0 {
            Level::Info
        } else {
            Level::Debug
        };
        if !enabled(level) {
            return;
        }
        let mut line = format!(
            "playback {:.1} s: {:.3} fps in, {:.3} out; dropped {} ({} in all), by the decoder {}, late {}, mistimed {}",
            model.double(PropertyId::TimePos).unwrap_or(0.0),
            model.double(PropertyId::ContainerFps).unwrap_or(0.0),
            model.double(PropertyId::EstimatedVfFps).unwrap_or(0.0),
            delta.dropped,
            counts.dropped,
            delta.decoder,
            delta.late,
            delta.mistimed
        );
        if let Some((drawn, gap)) = second.drawn {
            let _ = write!(line, "; drawn {drawn}, longest gap {gap} ms");
        }
        if let Some((busy, ours)) = second.processors {
            let _ = write!(
                line,
                "; processors {:.0} % busy, RVP {:.0} %",
                busy * 100.0,
                ours * 100.0
            );
        }
        write(level, &line);
    }
}

/// The paths whose values differ between `old` and `new` (settings as
/// JSON), with both values.
fn changed_paths(
    old: &serde_json::Value,
    new: &serde_json::Value,
    prefix: &str,
    out: &mut Vec<(String, String, String)>,
) {
    use serde_json::Value;
    if old == new {
        return;
    }
    if let (Value::Object(old), Value::Object(new)) = (old, new) {
        let keys: std::collections::BTreeSet<&String> = old.keys().chain(new.keys()).collect();
        for key in keys {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            changed_paths(
                old.get(key).unwrap_or(&Value::Null),
                new.get(key).unwrap_or(&Value::Null),
                &path,
                out,
            );
        }
        return;
    }
    let shown = |value: &Value| {
        let text = value.to_string();
        match text.char_indices().nth(200) {
            Some((end, _)) => format!("{}…", &text[..end]),
            None => text,
        }
    };
    out.push((prefix.to_owned(), shown(old), shown(new)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_parse_and_map_to_mpvs() {
        assert_eq!(Level::parse("off"), None);
        assert_eq!(Level::parse("debug"), Some(Level::Debug));
        assert!(Level::Debug > Level::Info && Level::Warning > Level::Critical);
        assert_eq!(Level::of_mpv("fatal"), Level::Critical);
        assert_eq!(Level::of_mpv("warn"), Level::Warning);
        assert_eq!(Level::of_mpv("v"), Level::Debug);
        assert_eq!(Level::Info.mpv(), "info");
        let mut settings = LogSettings {
            level: "loud".to_owned(),
        };
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(settings.level, "off");
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn times_are_local_with_milliseconds() {
        assert_eq!(time_text(0), "1970-01-01 00:00:00.000");
        // 2026-11-02 18:17:58.123 (UTC+2) as UTC plus two hours.
        assert_eq!(
            time_text(1_793_636_278_123 + 7_200_000),
            "2026-11-02 18:17:58.123"
        );
        assert_eq!(time_text(-1), "1969-12-31 23:59:59.999");
    }

    #[test]
    fn counts_start_again_with_a_file() {
        let before = Counts {
            dropped: 10,
            decoder: 2,
            late: 5,
            mistimed: 0,
        };
        let now = Counts {
            dropped: 13,
            decoder: 1,
            late: 5,
            mistimed: 0,
        };
        assert_eq!(
            now.since(before),
            Counts {
                dropped: 3,
                decoder: 1,
                late: 0,
                mistimed: 0
            }
        );
    }

    #[test]
    fn changed_settings_name_their_paths() {
        let old = serde_json::json!({ "smoothing": { "engine": "mvtools", "preset": "medium" }, "hwdec": "auto" });
        let new = serde_json::json!({ "smoothing": { "engine": "rife", "preset": "medium" }, "hwdec": "auto", "log": { "level": "info" } });
        let mut changed = Vec::new();
        changed_paths(&old, &new, "", &mut changed);
        assert_eq!(
            changed,
            [
                (
                    "log".to_owned(),
                    "null".to_owned(),
                    "{\"level\":\"info\"}".to_owned()
                ),
                (
                    "smoothing.engine".to_owned(),
                    "\"mvtools\"".to_owned(),
                    "\"rife\"".to_owned()
                ),
            ]
        );
    }

    /// The file, its levels and its rotation, in one test: the log is one
    /// per process.
    #[test]
    fn the_file_takes_its_levels_and_moves_aside_when_large() {
        let folder = std::env::temp_dir().join(format!("rvp-log-test-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&folder));
        let path = folder.join(FILE_NAME);
        assert_eq!(configure(Some(Level::Warning), &path), Ok(true));
        assert_eq!(configure(Some(Level::Warning), &path), Ok(false));
        write(Level::Warning, "kept");
        write(Level::Info, "not at warning");
        assert!(enabled(Level::Critical) && !enabled(Level::Info));
        // Off: the thread writes what it has and ends.
        assert_eq!(configure(None, &path), Ok(false));
        write(Level::Critical, "after off");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(text.contains(" WARN  kept\n"), "{text}");
        assert!(!text.contains("not at warning") && !text.contains("after off"));
        assert!(recent().ends_with("WARN  kept"));
        // A file at the size moves aside when it opens (Windows sets no
        // length through a handle that only appends).
        let file = File::options().write(true).open(&path).expect("log");
        file.set_len(ROTATE_AT).expect("size");
        drop(file);
        assert_eq!(configure(Some(Level::Debug), &path), Ok(true));
        write(Level::Debug, "fresh");
        assert_eq!(configure(None, &path), Ok(false));
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(text.lines().count(), 1, "{text}");
        assert!(text.ends_with(" DEBUG fresh\n"), "{text}");
        assert!(folder.join(OLD_FILE_NAME).is_file());
        drop(std::fs::remove_dir_all(&folder));
    }
}
