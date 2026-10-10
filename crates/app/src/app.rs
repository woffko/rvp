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
use crate::input::{Bindings, KeyChord};
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
    TogglePlaylist,
    /// A non-blocking message (section 4: no modal interruptions).
    Message(String),
}

/// A seek step as PotPlayer's menu names it: `5 Sec`, `1 Min`.
fn step_label(seconds: f64) -> String {
    // Seek steps are whole seconds in practice; the fraction is shown when
    // a setting has one.
    if seconds >= 60.0 && seconds % 60.0 == 0.0 {
        format!("{} Min", seconds / 60.0)
    } else {
        format!("{seconds} Sec")
    }
}

/// What one drain of the engine produced.
#[derive(Debug, Default)]
pub struct Drained {
    pub changes: Changes,
    pub requests: Vec<UiRequest>,
    pub events: Vec<EngineEvent>,
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

    /// The context menu entry of an action: PotPlayer's text with the seek
    /// step filled in, and the key shown next to it (PotPlayer's key while
    /// it is still bound to the action, otherwise the simplest bound key).
    #[must_use]
    pub fn menu_entry(&self, action: Action) -> (String, String) {
        let (text, shown) = action.menu_text();
        let label = match action {
            Action::SeekBackward(step) | Action::SeekForward(step) => {
                text.replace("%s", &step_label(self.settings.seek_seconds(step)))
            }
            _ => text.to_owned(),
        };
        let shown = shown
            .and_then(KeyChord::parse)
            .filter(|chord| self.bindings.key(chord) == Some(action));
        let shortcut = match (action, shown) {
            // Alt+F4 is the window system's, never in the bindings.
            (Action::Exit, _) => "Alt+F4".to_owned(),
            (_, Some(chord)) => chord.to_string(),
            (_, None) => self
                .bindings
                .key_for(action)
                .map(|chord| chord.to_string())
                .unwrap_or_default(),
        };
        (label, shortcut)
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
            Action::TogglePlaylist => return Some(UiRequest::TogglePlaylist),
            Action::ToggleSubtitles => engine.toggle_subtitles(),
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

    /// Handles everything libmpv has reported: updates the model and the
    /// resume store, and returns the changes, requests for the window and
    /// the events.
    pub fn drain(&mut self, engine: &Engine, now: Instant) -> Drained {
        let mut events = Vec::new();
        let changes = engine.drain(&mut self.model, &mut events);
        let mut requests = Vec::new();
        let mut save_now = false;
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
        Drained {
            changes,
            requests,
            events,
        }
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
