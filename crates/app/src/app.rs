//! The application core: turns actions into engine commands and window
//! requests, and keeps the resume store in step with playback.
//!
//! It has no Qt dependency; the shell owns the [`Engine`], forwards input as
//! [`Action`]s and carries out the [`UiRequest`]s it gets back.

use std::io;
use std::path::Path;
use std::time::Instant;

use rvp_engine::{
    Changes, EndReason, Engine, EngineError, EngineEvent, EngineOptions, PlayerModel, PropertyId,
    Seek,
};

use crate::action::Action;
use crate::input::Bindings;
use crate::resume::{ResumeStore, WindowGeometry};
use crate::settings::Settings;
use crate::storage::{self, Dirs};

/// Something only the window can do.
#[derive(Debug, Clone, PartialEq)]
pub enum UiRequest {
    OpenFileDialog,
    ToggleFullscreen {
        stretch: bool,
    },
    Minimize,
    MaximizeRestore,
    Exit,
    ContextMenu,
    /// A non-blocking message (section 4: no modal interruptions).
    Message(String),
}

/// Resume-position tracking around file loads.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tracking {
    /// A file is starting; positions are not recorded.
    Loading,
    /// Waiting for the resume seek to reach this position.
    Resuming(f64),
    /// Positions are recorded.
    On,
}

/// The application core.
pub struct App {
    pub settings: Settings,
    pub bindings: Bindings,
    pub model: PlayerModel,
    store: ResumeStore,
    events: Vec<EngineEvent>,
    tracking: Tracking,
    /// Warnings from loading the settings, for the shell to show once.
    pub warnings: Vec<String>,
}

impl App {
    /// Loads `settings.json` (writing the defaults when it is missing) and
    /// the resume store.
    ///
    /// # Errors
    /// Fails on I/O errors other than missing files.
    pub fn load(dirs: &Dirs) -> io::Result<Self> {
        let settings_path = dirs.config.join("settings.json");
        let settings = if let Some(settings) = storage::read_json(&settings_path)? {
            settings
        } else {
            let settings = Settings::default();
            storage::write_json(&settings_path, &settings)?;
            settings
        };
        Self::new(settings, &dirs.state)
    }

    /// # Errors
    /// Fails when the resume store cannot be read.
    pub fn new(settings: Settings, state_dir: &Path) -> io::Result<Self> {
        let (bindings, warnings) = settings.bindings();
        Ok(Self {
            settings,
            bindings,
            model: PlayerModel::new(),
            store: ResumeStore::open(state_dir)?,
            events: Vec::new(),
            tracking: Tracking::On,
            warnings,
        })
    }

    /// The engine options these settings ask for.
    #[must_use]
    pub fn engine_options(&self) -> EngineOptions {
        let mut options = EngineOptions::player();
        options.hwdec.clone_from(&self.settings.hwdec);
        options.ipc_server =
            Some(self.settings.ipc_server.clone()).filter(|server| !server.is_empty());
        options
    }

    #[must_use]
    pub fn store(&self) -> &ResumeStore {
        &self.store
    }

    /// Restores volume and mute, then opens `files`, or without files the
    /// last file paused at its position (`reopen-last-file`).
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn start(&mut self, engine: &Engine, files: &[String]) -> Result<(), EngineError> {
        let state = self.store.state();
        engine.set_volume(state.volume.unwrap_or(100.0))?;
        engine.set_mute(state.mute)?;
        if !files.is_empty() {
            return self.open(engine, files, false);
        }
        if self.settings.reopen_last_file
            && let Some(last) = state.last_file.clone()
        {
            engine.set_pause(true)?;
            return self.open(engine, &[last], false);
        }
        Ok(())
    }

    /// Opens files: the first replaces what plays (or is appended with
    /// `append`), the rest are appended to mpv's playlist.
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn open(
        &mut self,
        engine: &Engine,
        files: &[String],
        append: bool,
    ) -> Result<(), EngineError> {
        for (index, file) in files.iter().enumerate() {
            engine.load(file, append || index > 0)?;
        }
        Ok(())
    }

    /// Carries out an action; window-level actions come back as requests.
    pub fn dispatch(&mut self, engine: &Engine, action: Action) -> Option<UiRequest> {
        let result = match action {
            Action::OpenFiles => return Some(UiRequest::OpenFileDialog),
            Action::Exit => return Some(UiRequest::Exit),
            Action::ToggleFullscreen => {
                return Some(UiRequest::ToggleFullscreen { stretch: false });
            }
            Action::ToggleFullscreenStretch => {
                return Some(UiRequest::ToggleFullscreen { stretch: true });
            }
            Action::Minimize => return Some(UiRequest::Minimize),
            Action::MaximizeRestore => return Some(UiRequest::MaximizeRestore),
            Action::ContextMenu => return Some(UiRequest::ContextMenu),
            Action::PlayPause => {
                if !self.model.media_loaded() {
                    return Some(UiRequest::OpenFileDialog);
                }
                if self.model.flag(PropertyId::EofReached) == Some(true) {
                    // At the end (keep-open), Play starts the file again.
                    engine
                        .seek(Seek::Absolute(0.0))
                        .and_then(|()| engine.set_pause(false))
                } else {
                    engine.toggle_pause()
                }
            }
            Action::CloseMedia => engine.stop(),
            Action::Previous => engine.playlist_step(-1),
            Action::Next => engine.playlist_step(1),
            Action::SeekBackward(step) => {
                self.seek(engine, Seek::Relative(-self.settings.seek_seconds(step)))
            }
            Action::SeekForward(step) => {
                self.seek(engine, Seek::Relative(self.settings.seek_seconds(step)))
            }
            Action::VolumeUp => self.change_volume(engine, self.settings.volume_step),
            Action::VolumeDown => self.change_volume(engine, -self.settings.volume_step),
            Action::ToggleMute => engine.set_mute(self.model.flag(PropertyId::Mute) != Some(true)),
        };
        // Commands that do not apply (no next file, nothing to seek) are
        // ignored, as PotPlayer does.
        drop(result);
        None
    }

    fn seek(&mut self, engine: &Engine, target: Seek) -> Result<(), EngineError> {
        if !self.model.media_loaded() {
            return Ok(());
        }
        // A user seek ends any pending resume.
        if matches!(self.tracking, Tracking::Resuming(_)) {
            self.tracking = Tracking::On;
        }
        engine.seek(target)
    }

    fn change_volume(&mut self, engine: &Engine, delta: f64) -> Result<(), EngineError> {
        let current = self.model.double(PropertyId::Volume).unwrap_or(100.0);
        engine.set_volume((current + delta).clamp(0.0, 100.0))
    }

    /// The seek bar was clicked or dragged to `fraction` (0.0 to 1.0).
    pub fn seek_to_fraction(&mut self, engine: &Engine, fraction: f64) {
        drop(self.seek(engine, Seek::Fraction(fraction)));
    }

    /// The volume slider was set to `fraction` (0.0 to 1.0 of 100 %).
    pub fn set_volume_fraction(&mut self, engine: &Engine, fraction: f64) {
        drop(engine.set_volume((fraction * 100.0).clamp(0.0, 100.0)));
    }

    /// Handles everything libmpv has reported: updates the model, the resume
    /// store and returns requests for the window.
    pub fn drain(&mut self, engine: &Engine, now: Instant) -> (Changes, Vec<UiRequest>) {
        let changes = engine.drain(&mut self.model, &mut self.events);
        let mut requests = Vec::new();
        let mut save_now = false;
        let events = std::mem::take(&mut self.events);
        for event in &events {
            match event {
                EngineEvent::StartFile => self.tracking = Tracking::Loading,
                EngineEvent::FileLoaded { path } => {
                    self.store.set_last_file(path);
                    save_now = true;
                    let resume = self
                        .store
                        .position(path)
                        .filter(|_| self.settings.remember_positions);
                    self.tracking = match resume {
                        Some(position) if engine.seek(Seek::Absolute(position)).is_ok() => {
                            Tracking::Resuming(position)
                        }
                        _ => Tracking::On,
                    };
                }
                EngineEvent::EndFile {
                    reason: EndReason::Eof,
                    path: Some(path),
                } => {
                    self.store.clear_position(path);
                    save_now = true;
                }
                EngineEvent::EndFile { .. } => save_now = true,
                EngineEvent::Error(message) => {
                    requests.push(UiRequest::Message(format!("Cannot play: {message}")));
                }
                EngineEvent::PlaybackRestart | EngineEvent::Shutdown => {}
            }
        }
        self.events = events;
        self.events.clear();

        let time = self.model.double(PropertyId::TimePos);
        if let (Tracking::Resuming(target), Some(time)) = (self.tracking, time)
            && (time - target).abs() < 2.0
        {
            self.tracking = Tracking::On;
        }
        if self.tracking == Tracking::On
            && let (Some(path), Some(time)) = (self.model.loaded_path(), time)
        {
            if self.model.flag(PropertyId::EofReached) == Some(true) {
                self.store.clear_position(path);
            } else {
                let duration = self.model.double(PropertyId::Duration);
                self.store.set_position(path, time, duration);
            }
        }
        if changes.contains(PropertyId::Volume)
            && let Some(volume) = self.model.double(PropertyId::Volume)
        {
            self.store.set_volume(volume);
        }
        if changes.contains(PropertyId::Mute) {
            self.store
                .set_mute(self.model.flag(PropertyId::Mute) == Some(true));
        }
        if changes.any(&[PropertyId::Pause, PropertyId::EofReached]) {
            save_now = true;
        }
        let saved = if save_now {
            self.store.save(now)
        } else {
            self.store.save_if_due(now)
        };
        if let Err(error) = saved {
            requests.push(UiRequest::Message(format!(
                "Cannot save the playback state: {error}"
            )));
        }
        (changes, requests)
    }

    /// Saves a pending change once the save interval has passed; call it
    /// about once a second.
    ///
    /// # Errors
    /// Fails on I/O errors.
    pub fn tick(&mut self, now: Instant) -> io::Result<()> {
        self.store.save_if_due(now)
    }

    /// The window moved, resized or changed state.
    pub fn set_window(&mut self, window: WindowGeometry) {
        self.store.set_window(window);
    }

    /// Saves everything and stops playback before libmpv shuts down (goal D
    /// rule 6).
    ///
    /// # Errors
    /// Fails on I/O errors.
    pub fn shutdown(&mut self, engine: &Engine, now: Instant) -> io::Result<()> {
        drop(engine.stop());
        self.store.save(now)
    }
}
