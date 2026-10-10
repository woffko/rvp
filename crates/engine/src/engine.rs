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
    FileLoaded,
    EndFile(EndReason),
    /// A seek finished and playback continues from the new position.
    PlaybackRestart,
    /// An error reported by libmpv, for example a file that cannot be opened.
    Error(String),
    Shutdown,
}

/// An engine failure.
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
            init.set_option("osc", "no")?;
            init.set_option("input-default-bindings", "no")?;
            init.set_option("input-vo-keyboard", "no")?;
            init.set_option("ytdl", "no")?;
            init.set_option("terminal", "no")?;
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
            Ok(Event::FileLoaded) => events.push(EngineEvent::FileLoaded),
            Ok(Event::EndFile(reason)) => events.push(EngineEvent::EndFile(end_reason(reason))),
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
        let (amount, mode) = match target {
            Seek::Absolute(seconds) => (seconds, "absolute+exact"),
            Seek::Relative(seconds) => (seconds, "relative+exact"),
            Seek::Fraction(fraction) => {
                (fraction.clamp(0.0, 1.0) * 100.0, "absolute-percent+exact")
            }
        };
        self.mpv
            .command("seek", &[&format!("{amount:.3}"), mode])
            .map_err(|e| error("seek", &e))
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

fn end_reason(reason: EndFileReason) -> EndReason {
    match reason {
        mpv_end_file_reason::Eof => EndReason::Eof,
        mpv_end_file_reason::Quit => EndReason::Quit,
        mpv_end_file_reason::Error => EndReason::Error,
        mpv_end_file_reason::Redirect => EndReason::Redirect,
        _ => EndReason::Stop,
    }
}
