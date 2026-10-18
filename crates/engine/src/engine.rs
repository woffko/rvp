//! The libmpv player: options, observed properties, events and commands.
//!
//! Threading (docs/PLAN.md section 5): libmpv runs its own threads. The
//! wakeup callback must only queue work for the owner's thread; that thread
//! calls [`Engine::drain`], which never blocks. The render context for the
//! video item is created from [`Engine::mpv`] on the render thread.
//!
//! libmpv reads no `mpv.conf` and loads no scripts here: every option RVP
//! relies on is set through the API.

use libmpv2::events::{Event, PropertyData};
use libmpv2::{EndFileReason, Format, Mpv, mpv_end_file_reason};

use crate::model::{Changes, Kind, PlayerModel, PropertyId, Value};

/// Where video goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoOutput {
    /// The libmpv render API (`vo=libmpv`): the shell's video item draws.
    RenderApi,
    /// No video output (`vo=null`), for headless tests.
    Null,
}

/// Where audio goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioOutput {
    /// mpv's automatic choice (`PipeWire` first on Linux when libmpv is built
    /// with it, see goal D rule 5).
    Auto,
    /// No audio output (`ao=null`), for headless tests.
    Null,
}

/// How an [`Engine`] is created.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineOptions {
    pub video: VideoOutput,
    pub audio: AudioOutput,
    /// mpv `hwdec`; SVP needs a copy-back mode (`auto-copy`).
    pub hwdec: String,
    /// mpv `input-ipc-server` (SVP attaches through it; goal C/D S1).
    pub ipc_server: Option<String>,
    /// Further `name=value` options, applied last.
    pub extra: Vec<(String, String)>,
}

impl EngineOptions {
    /// Options for the player window.
    #[must_use]
    pub fn player() -> Self {
        Self {
            video: VideoOutput::RenderApi,
            audio: AudioOutput::Auto,
            hwdec: "auto-copy".to_owned(),
            ipc_server: Some(default_ipc_server().to_owned()),
            extra: Vec::new(),
        }
    }

    /// Options for tests: no video or audio device, no IPC server.
    #[must_use]
    pub fn headless() -> Self {
        Self {
            video: VideoOutput::Null,
            audio: AudioOutput::Null,
            hwdec: "no".to_owned(),
            ipc_server: None,
            extra: Vec::new(),
        }
    }
}

/// The IPC endpoint SVP 4 looks for: a named pipe on Windows, a socket on
/// Linux (docs/spikes/s1-svp-attach.md).
#[must_use]
pub const fn default_ipc_server() -> &'static str {
    if cfg!(windows) {
        "mpvpipe"
    } else {
        "/tmp/mpvsocket"
    }
}

/// Why a file stopped playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    /// Played to the end.
    Eof,
    /// Stopped by a command (stop, loading another file).
    Stop,
    /// The player is quitting.
    Quit,
    /// The file could not be played.
    Error,
    /// The file was a playlist that was expanded.
    Redirect,
}

/// Engine events other than property changes.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    StartFile,
    /// A file is open; `path` is mpv's `path` for it.
    FileLoaded {
        path: String,
    },
    /// The file loaded before (if any) stopped playing.
    EndFile {
        reason: EndReason,
        path: Option<String>,
    },
    /// A seek finished and playback continues from the new position.
    PlaybackRestart,
    /// An error reported by libmpv, for example a file that cannot be opened.
    Error(String),
    Shutdown,
}

/// An engine failure.
/// A frame from [`Engine::screenshot_raw`]: `stride` bytes a row in mpv's
/// `format` (`bgr0` unless asked otherwise), rows top first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub format: String,
    pub data: Vec<u8>,
}

impl Image {
    /// The red, green and blue of the pixel at (`x`, `y`) of an 8-bit
    /// image (`bgr0`, `bgra`, `rgba`), or `None` outside it.
    #[must_use]
    pub fn rgb(&self, x: usize, y: usize) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = y.checked_mul(self.stride)?.checked_add(x.checked_mul(4)?)?;
        let pixel = self.data.get(at..at + 4)?;
        match self.format.as_str() {
            "bgr0" | "bgra" => Some([pixel[2], pixel[1], pixel[0]]),
            "rgba" | "rgb0" => Some([pixel[0], pixel[1], pixel[2]]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

fn error(context: &str, error: &libmpv2::Error) -> EngineError {
    EngineError(format!("{context}: {error}"))
}

/// A seek target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seek {
    /// Seconds from the start.
    Absolute(f64),
    /// Seconds from the current position.
    Relative(f64),
    /// Fraction of the duration, 0.0 to 1.0.
    Fraction(f64),
}

/// What a track carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackKind {
    Audio,
    Video,
    Subtitle,
}

/// One of a file's tracks (mpv's `track-list`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// mpv's ID within its kind (`aid`, `vid`, `sid`).
    pub id: i64,
    pub kind: TrackKind,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub selected: bool,
    /// Loaded from a separate file (a subtitle added later).
    pub external: bool,
    /// An audio track's channels, sample rate (Hz) and bitrate (bit/s),
    /// as the demuxer reports them.
    pub channels: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bitrate: Option<i64>,
}

/// One chapter (mpv's `chapter-list`).
#[derive(Debug, Clone, PartialEq)]
pub struct Chapter {
    pub title: Option<String>,
    /// Start in seconds.
    pub time: f64,
}

/// The libmpv player.
pub struct Engine {
    mpv: Mpv,
}

impl Engine {
    /// Creates and initializes libmpv and observes the properties of
    /// [`PropertyId`].
    ///
    /// # Errors
    /// Fails when libmpv cannot be created or rejects an option.
    pub fn new(options: &EngineOptions) -> Result<Self, EngineError> {
        let mpv = Mpv::with_initializer(|init| {
            init.set_option(
                "vo",
                match options.video {
                    VideoOutput::RenderApi => "libmpv",
                    VideoOutput::Null => "null",
                },
            )?;
            if options.audio == AudioOutput::Null {
                init.set_option("ao", "null")?;
            }
            init.set_option("hwdec", options.hwdec.as_str())?;
            // SVP needs exact seeks (S1), the window stays on the last frame
            // at the end, and mpv's own input and OSC stay off.
            init.set_option("hr-seek-framedrop", "no")?;
            init.set_option("keep-open", "yes")?;
            init.set_option("idle", "yes")?;
            init.set_option("input-default-bindings", "no")?;
            init.set_option("input-vo-keyboard", "no")?;
            init.set_option("terminal", "no")?;
            // The OSC and the youtube-dl hook are Lua scripts; libmpv built
            // without Lua (the Linux kit's) has neither option, and nothing
            // to turn off.
            for name in ["osc", "ytdl"] {
                drop(init.set_option(name, "no"));
            }
            if let Some(server) = &options.ipc_server {
                init.set_option("input-ipc-server", server.as_str())?;
            }
            for (name, value) in &options.extra {
                init.set_option(name.as_str(), value.as_str())?;
            }
            Ok(())
        })
        .map_err(|e| error("libmpv initialization failed", &e))?;
        for &id in PropertyId::ALL {
            let format = match id.kind() {
                Kind::Flag => Format::Flag,
                Kind::Int => Format::Int64,
                Kind::Double => Format::Double,
                Kind::Text => Format::String,
            };
            mpv.observe_property(id.name(), format, id as u64)
                .map_err(|e| error(id.name(), &e))?;
        }
        Ok(Self { mpv })
    }

    /// The libmpv handle, for the render context and for diagnostics.
    #[must_use]
    pub fn mpv(&self) -> &Mpv {
        &self.mpv
    }

    /// Sets the function libmpv calls (on its own thread) when events are
    /// waiting. It must only schedule a call to [`Engine::drain`].
    pub fn set_wakeup_callback<F: Fn() + Send + 'static>(&mut self, callback: F) {
        self.mpv.set_wakeup_callback(callback);
    }

    /// Handles all waiting events without blocking: property changes go into
    /// `model` and are returned as a change set, other events are appended to
    /// `events`.
    pub fn drain(&self, model: &mut PlayerModel, events: &mut Vec<EngineEvent>) -> Changes {
        let mut changes = Changes::default();
        while self.handle_next(0.0, model, events, &mut changes) {}
        changes
    }

    /// Waits up to `timeout` seconds for one event, then drains the rest.
    /// For headless use; the shell uses the wakeup callback instead.
    pub fn wait_and_drain(
        &self,
        timeout: f64,
        model: &mut PlayerModel,
        events: &mut Vec<EngineEvent>,
    ) -> Changes {
        let mut changes = Changes::default();
        if self.handle_next(timeout, model, events, &mut changes) {
            while self.handle_next(0.0, model, events, &mut changes) {}
        }
        changes
    }

    /// Handles one event; false when the queue was empty.
    fn handle_next(
        &self,
        timeout: f64,
        model: &mut PlayerModel,
        events: &mut Vec<EngineEvent>,
        changes: &mut Changes,
    ) -> bool {
        let Some(event) = self.mpv.wait_event(timeout) else {
            return false;
        };
        match event {
            Ok(Event::PropertyChange {
                change,
                reply_userdata,
                ..
            }) => {
                if let Some(id) = PropertyId::from_userdata(reply_userdata) {
                    let value = match change {
                        PropertyData::Flag(value) => Value::Flag(value),
                        PropertyData::Int64(value) => Value::Int(value),
                        PropertyData::Double(value) => Value::Double(value),
                        PropertyData::Str(value) | PropertyData::OsdStr(value) => {
                            Value::Text(value.to_owned())
                        }
                    };
                    if model.set(id, Some(value)) {
                        changes.insert(id);
                    }
                }
            }
            Ok(Event::PropertyUnavailable { reply_userdata, .. }) => {
                if let Some(id) = PropertyId::from_userdata(reply_userdata)
                    && model.set(id, None)
                {
                    changes.insert(id);
                }
            }
            Ok(Event::StartFile) => events.push(EngineEvent::StartFile),
            Ok(Event::FileLoaded) => {
                // Asked directly: the observed `path` may arrive in a later
                // batch than this event.
                let path = self
                    .mpv
                    .get_property::<String>("path")
                    .ok()
                    .or_else(|| model.text(PropertyId::Path).map(str::to_owned))
                    .unwrap_or_default();
                model.set_loaded_path(Some(path.clone()));
                events.push(EngineEvent::FileLoaded { path });
            }
            Ok(Event::EndFile(reason)) => {
                let path = model.set_loaded_path(None);
                events.push(EngineEvent::EndFile {
                    reason: end_reason(reason),
                    path,
                });
            }
            Ok(Event::PlaybackRestart) => events.push(EngineEvent::PlaybackRestart),
            Ok(Event::Shutdown) => events.push(EngineEvent::Shutdown),
            Ok(_) => {}
            // libmpv2 reports an end-file with an error code (an unplayable
            // file) and failed asynchronous replies this way.
            Err(e) => events.push(EngineEvent::Error(e.to_string())),
        }
        true
    }

    /// Opens a file or URL. `append` adds it to mpv's playlist instead of
    /// replacing the playlist; the first appended file starts playing when
    /// nothing plays.
    ///
    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn load(&self, target: &str, append: bool) -> Result<(), EngineError> {
        let mode = if append { "append-play" } else { "replace" };
        self.mpv
            .command("loadfile", &[target, mode])
            .map_err(|e| error("loadfile", &e))
    }

    /// Opens a file or URL in place of the one that plays, with options
    /// for this file only (`name=value`, as mpv's `loadfile` takes them:
    /// `http-header-fields`, `force-media-title`).
    ///
    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn load_with_options(
        &self,
        target: &str,
        options: &[(String, String)],
    ) -> Result<(), EngineError> {
        if options.is_empty() {
            return self.load(target, false);
        }
        // Commas inside a value would split it: mpv takes `%N%` lengths.
        let options = options
            .iter()
            .map(|(name, value)| format!("{name}=%{}%{value}", value.len()))
            .collect::<Vec<_>>()
            .join(",");
        // mpv 0.38 put an index before the options.
        let result = if self.version() >= (0, 38) {
            self.mpv
                .command("loadfile", &[target, "replace", "-1", &options])
        } else {
            self.mpv.command("loadfile", &[target, "replace", &options])
        };
        result.map_err(|e| error("loadfile", &e))
    }

    /// libmpv's version as (major, minor), from `mpv-version` (`mpv
    /// 0.37.0`, `mpv v0.40.0-12-gabcdef`); (0, 0) when unknown.
    #[must_use]
    pub fn version(&self) -> (u32, u32) {
        self.get_text("mpv-version")
            .as_deref()
            .map_or((0, 0), parse_version)
    }

    /// The protocols libmpv opens (`protocol-list`: `dvd`, `bd`, `cdda`,
    /// `av`, `http`...).
    #[must_use]
    pub fn protocols(&self) -> Vec<String> {
        // A string list: mpv gives it as text separated by commas.
        self.get_text("protocol-list")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn set_pause(&self, pause: bool) -> Result<(), EngineError> {
        self.mpv
            .set_property("pause", pause)
            .map_err(|e| error("pause", &e))
    }

    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn toggle_pause(&self) -> Result<(), EngineError> {
        self.mpv
            .command("cycle", &["pause"])
            .map_err(|e| error("cycle pause", &e))
    }

    /// Seeks exactly (SVP needs exact seeks).
    ///
    /// # Errors
    /// Fails when libmpv rejects the command (for example with nothing loaded).
    pub fn seek(&self, target: Seek) -> Result<(), EngineError> {
        self.seek_with(target, true)
    }

    /// Seeks exactly, or to a keyframe near the target (fast, but up to a
    /// few seconds off).
    ///
    /// # Errors
    /// Fails when libmpv rejects the command (for example with nothing loaded).
    pub fn seek_with(&self, target: Seek, exact: bool) -> Result<(), EngineError> {
        let precision = if exact { "exact" } else { "keyframes" };
        let (amount, mode) = match target {
            Seek::Absolute(seconds) => (seconds, "absolute"),
            Seek::Relative(seconds) => (seconds, "relative"),
            Seek::Fraction(fraction) => (fraction.clamp(0.0, 1.0) * 100.0, "absolute-percent"),
        };
        self.mpv
            .command(
                "seek",
                &[&format!("{amount:.3}"), &format!("{mode}+{precision}")],
            )
            .map_err(|e| error("seek", &e))
    }

    /// Runs an mpv command (`frame-step`, `sub-seek -1`, ...).
    ///
    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn command(&self, name: &str, args: &[&str]) -> Result<(), EngineError> {
        self.mpv.command(name, args).map_err(|e| error(name, &e))
    }

    /// Sets an mpv property or option.
    ///
    /// # Errors
    /// Fails when libmpv rejects the value.
    pub fn set_double(&self, name: &str, value: f64) -> Result<(), EngineError> {
        self.mpv
            .set_property(name, value)
            .map_err(|e| error(name, &e))
    }

    /// Sets an mpv property or option.
    ///
    /// # Errors
    /// Fails when libmpv rejects the value.
    pub fn set_text(&self, name: &str, value: &str) -> Result<(), EngineError> {
        self.mpv
            .set_property(name, value)
            .map_err(|e| error(name, &e))
    }

    /// Sets an mpv property or option.
    ///
    /// # Errors
    /// Fails when libmpv rejects the value.
    pub fn set_flag(&self, name: &str, value: bool) -> Result<(), EngineError> {
        self.mpv
            .set_property(name, value)
            .map_err(|e| error(name, &e))
    }

    /// Reads a property now. This waits for mpv's core: for menus and
    /// dialogs, never in a loop during playback.
    #[must_use]
    pub fn get_double(&self, name: &str) -> Option<f64> {
        self.mpv.get_property::<f64>(name).ok()
    }

    /// Reads a property now (see [`Engine::get_double`]).
    #[must_use]
    pub fn get_int(&self, name: &str) -> Option<i64> {
        self.mpv.get_property::<i64>(name).ok()
    }

    /// Reads a property now (see [`Engine::get_double`]).
    #[must_use]
    pub fn get_text(&self, name: &str) -> Option<String> {
        self.mpv.get_property::<String>(name).ok()
    }

    /// The file's audio, video and subtitle tracks (read now, for menus):
    /// `track-list` in one read, which mpv gives as JSON text.
    #[must_use]
    pub fn tracks(&self) -> Vec<Track> {
        let Some(list) = self.get_json("track-list") else {
            return Vec::new();
        };
        let text = |value: &serde_json::Value| value.as_str().map(str::to_owned);
        list.as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|track| {
                let kind = match track["type"].as_str()? {
                    "audio" => TrackKind::Audio,
                    "video" => TrackKind::Video,
                    "sub" => TrackKind::Subtitle,
                    _ => return None,
                };
                Some(Track {
                    id: track["id"].as_i64()?,
                    kind,
                    title: text(&track["title"]),
                    language: text(&track["lang"]),
                    codec: text(&track["codec"]),
                    selected: track["selected"].as_bool().unwrap_or(false),
                    external: track["external"].as_bool().unwrap_or(false),
                    channels: track["demux-channel-count"].as_i64(),
                    sample_rate: track["demux-samplerate"].as_i64(),
                    bitrate: track["demux-bitrate"].as_i64(),
                })
            })
            .collect()
    }

    /// The file's chapters (read now, for menus), in one read.
    #[must_use]
    pub fn chapters(&self) -> Vec<Chapter> {
        let Some(list) = self.get_json("chapter-list") else {
            return Vec::new();
        };
        list.as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|chapter| Chapter {
                title: chapter["title"].as_str().map(str::to_owned),
                time: chapter["time"].as_f64().unwrap_or(0.0),
            })
            .collect()
    }

    /// A node property (lists, maps) as JSON, which mpv gives for them as
    /// text.
    #[must_use]
    pub fn get_json(&self, name: &str) -> Option<serde_json::Value> {
        serde_json::from_str(&self.get_text(name)?).ok()
    }

    /// The frame as `screenshot-raw` gives it: `flags` is `video` (the
    /// source frame after the filters), `subtitles` or `window` (as the
    /// screen shows it).
    ///
    /// # Errors
    /// Fails without a frame, or when libmpv refuses.
    pub fn screenshot_raw(&self, flags: &str) -> Result<Image, EngineError> {
        let raw = self
            .mpv
            .screenshot_raw(flags)
            .map_err(|e| error("screenshot-raw", &e))?;
        Ok(Image {
            width: raw.width,
            height: raw.height,
            stride: raw.stride,
            format: raw.format,
            data: raw.data,
        })
    }

    /// Sets the volume in percent (100 is unchanged).
    ///
    /// # Errors
    /// Fails when libmpv rejects the value.
    pub fn set_volume(&self, percent: f64) -> Result<(), EngineError> {
        self.mpv
            .set_property("volume", percent)
            .map_err(|e| error("volume", &e))
    }

    /// # Errors
    /// Fails when libmpv rejects the value.
    pub fn set_mute(&self, mute: bool) -> Result<(), EngineError> {
        self.mpv
            .set_property("mute", mute)
            .map_err(|e| error("mute", &e))
    }

    /// Sets RVP's audio filter with this label (`Some("@label:filter")`) or
    /// removes it (`None`). Other filters in the chain are left alone.
    ///
    /// # Errors
    /// Fails when libmpv rejects the filter.
    pub fn set_audio_filter(&self, label: &str, entry: Option<&str>) -> Result<(), EngineError> {
        // Removing a filter that is not there fails; that is fine.
        drop(self.mpv.command("af", &["remove", &format!("@{label}")]));
        match entry {
            Some(entry) => self
                .mpv
                .command("af", &["add", entry])
                .map_err(|e| error("af add", &e)),
            None => Ok(()),
        }
    }

    /// Puts an audio filter (`@label:...`) in front of the chain, replacing
    /// the one with the same label; `None` removes it. Other filters keep
    /// their places.
    ///
    /// # Errors
    /// Fails when libmpv rejects the filter.
    pub fn set_audio_filter_first(
        &self,
        label: &str,
        entry: Option<&str>,
    ) -> Result<(), EngineError> {
        drop(self.mpv.command("af", &["remove", &format!("@{label}")]));
        match entry {
            Some(entry) => self
                .mpv
                .command("af", &["pre", entry])
                .map_err(|e| error("af pre", &e)),
            None => Ok(()),
        }
    }

    /// Removes the video filter with this label (`vf remove @label`); other
    /// filters keep their places.
    ///
    /// # Errors
    /// Fails when no filter has the label.
    pub fn remove_video_filter(&self, label: &str) -> Result<(), EngineError> {
        self.mpv
            .command("vf", &["remove", &format!("@{label}")])
            .map_err(|e| error("vf remove", &e))
    }

    /// Puts a video filter (`@label:...`) in front of the chain (`vf pre`).
    ///
    /// # Errors
    /// Fails when libmpv rejects the filter.
    pub fn prepend_video_filter(&self, entry: &str) -> Result<(), EngineError> {
        self.mpv
            .command("vf", &["pre", entry])
            .map_err(|e| error("vf pre", &e))
    }

    /// Shows or hides subtitles (`sub-visibility`).
    ///
    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn toggle_subtitles(&self) -> Result<(), EngineError> {
        self.mpv
            .command("cycle", &["sub-visibility"])
            .map_err(|e| error("cycle sub-visibility", &e))
    }

    /// Next (`1`) or previous (`-1`) entry of mpv's playlist.
    ///
    /// # Errors
    /// Fails when there is no such entry.
    pub fn playlist_step(&self, step: i32) -> Result<(), EngineError> {
        let command = if step >= 0 {
            "playlist-next"
        } else {
            "playlist-prev"
        };
        self.mpv
            .command(command, &["weak"])
            .map_err(|e| error(command, &e))
    }

    /// The entries of mpv's playlist (file names or URLs as loaded), in
    /// order. Entries that disappear while reading are skipped.
    #[must_use]
    pub fn playlist(&self) -> Vec<String> {
        let count = self.mpv.get_property::<i64>("playlist-count").unwrap_or(0);
        (0..count)
            .filter_map(|index| {
                self.mpv
                    .get_property::<String>(&format!("playlist/{index}/filename"))
                    .ok()
            })
            .collect()
    }

    /// Keeps the video's aspect ratio (`true`) or stretches it to the
    /// output (PotPlayer's "Fullscreen (Stretch)").
    ///
    /// # Errors
    /// Fails when libmpv rejects the value.
    pub fn set_keep_aspect(&self, keep: bool) -> Result<(), EngineError> {
        self.mpv
            .set_property("keepaspect", keep)
            .map_err(|e| error("keepaspect", &e))
    }

    /// Stops playback and clears the playlist.
    ///
    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn stop(&self) -> Result<(), EngineError> {
        self.mpv.command("stop", &[]).map_err(|e| error("stop", &e))
    }

    /// Asks the core to shut down; a [`EngineEvent::Shutdown`] follows. Stop
    /// playback first (goal D rule 6).
    ///
    /// # Errors
    /// Fails when libmpv rejects the command.
    pub fn quit(&self) -> Result<(), EngineError> {
        self.mpv.command("quit", &[]).map_err(|e| error("quit", &e))
    }
}

/// (major, minor) of an `mpv-version` text.
fn parse_version(text: &str) -> (u32, u32) {
    let digits = text
        .trim_start_matches("mpv")
        .trim_start()
        .trim_start_matches('v');
    let mut parts = digits
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

fn end_reason(reason: EndFileReason) -> EndReason {
    match reason {
        mpv_end_file_reason::Eof => EndReason::Eof,
        mpv_end_file_reason::Quit => EndReason::Quit,
        mpv_end_file_reason::Error => EndReason::Error,
        mpv_end_file_reason::Redirect => EndReason::Redirect,
        _ => EndReason::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn versions_read_from_mpv_version() {
        assert_eq!(parse_version("mpv 0.37.0"), (0, 37));
        assert_eq!(parse_version("mpv v0.40.0-12-gabcdef"), (0, 40));
        assert_eq!(parse_version("mpv 0.38.0+git"), (0, 38));
        assert_eq!(parse_version("something"), (0, 0));
    }
}
