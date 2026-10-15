//! The application core: turns actions into engine commands and window
//! requests, plays the albums, and keeps the resume store and the albums'
//! file in step with playback.
//!
//! It has no Qt dependency; the shell owns the [`Engine`], forwards input as
//! [`Action`]s and [`ListAction`]s and carries out the [`UiRequest`]s it gets
//! back.
//!
//! mpv plays one album entry at a time: Next, Previous and the end of a file
//! move through the album that plays, skipping entries that could not be
//! opened, which are marked `<Can't open>`.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rvp_engine::{
    Changes, EndReason, Engine, EngineError, EngineEvent, EngineOptions, PlayerModel, PropertyId,
    Seek,
};

use crate::action::{Action, ListAction};
use crate::action::{AfterPlayback, Delay, Repeat};
use crate::commands::Session;
use crate::dpl::{self, DriveMap};
use crate::input::{Bindings, KeyChord};
use crate::instance::{Mode, Request};
use crate::playlist::{Library, Move, Order, Select};
use crate::pointer::ListEffect;
use crate::resume::{EDGE_SECONDS, ResumeStore, WindowGeometry};
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
    /// The settings window, at a page (`input`) or where it was.
    Preferences(Option<&'static str>),
    About,
    /// Ask for a folder to open.
    OpenFolderDialog,
    /// Ask for a URL to open.
    OpenUrlDialog,
    /// Ask for a subtitle file for the playing video.
    OpenSubtitleDialog,
    /// Ask for the folder saved frames go to.
    ImageFolderDialog,
    /// Show a folder in the system's file manager.
    OpenExternal(PathBuf),
    /// Save the playing frame to this file (mpv `screenshot-to-file`, off
    /// the GUI thread); `subtitles` as shown on screen.
    SaveFrame {
        path: PathBuf,
        subtitles: bool,
    },
    /// Resize the window to the video at this scale.
    ScaleWindow(f64),
    /// The window takes the video's shape.
    FitAspect,
    /// Keep the window above others, or not.
    OnTop(bool),
    /// Show or hide the playback information over the video.
    ShowInfo(bool),
    /// Show (`true`) or hide the control panel window (F7).
    ControlPanel(bool),
    /// A save dialog for the current frame, suggesting this file (then
    /// `Player.save_frame_to`).
    SaveFrameDialog(PathBuf),
    /// A font dialog for the subtitles, showing this family (empty: mpv's
    /// default; then `Player.apply_subtitle_font`).
    SubtitleFontDialog(String),
    /// Ask for PotPlayer's playlist folder to import.
    ImportPlaylists,
    /// Ask for files to add to the shown album.
    AddFilesDialog,
    /// Ask for a folder to add to the shown album.
    AddFolderDialog,
    /// Ask for an album name: a new album, or a new name for the shown one.
    AlbumName {
        rename: bool,
        current: String,
    },
    /// A short notice over the video (volume, normalizer).
    Osd(String),
    /// A non-blocking message (section 4: no modal interruptions).
    Message(String),
}

/// A sync step as PotPlayer's menu names it: `50ms`, `0.5 Sec`.
fn sync_step_label(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{:.0}ms", seconds * 1000.0)
    } else {
        format!("{seconds} Sec")
    }
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

/// The last stretch of a file, in seconds, in which changes of RVP's
/// audio filters wait for the next file (`App::defer_chain_change`). The
/// position RVP sees lags playback, and on the Windows VM a change and its
/// resync seek reached mpv after the end when made 0.7 s before it (goal
/// I), so the margin is wide.
const NEAR_END: f64 = 1.5;

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

/// File extensions "Add folder" picks up.
const MEDIA_EXTENSIONS: &[&str] = &[
    "3gp", "aac", "ac3", "aiff", "ape", "asf", "avi", "dts", "flac", "flv", "m2ts", "m4a", "m4v",
    "mka", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "mts", "ogg", "ogm", "ogv", "opus", "rm",
    "rmvb", "ts", "vob", "wav", "webm", "wma", "wmv", "wv",
];

/// The application core.
// Independent flags of the playback state (the end handled, the next entry
// to play, filters waiting near the end, the first start), not a mode.
#[allow(clippy::struct_excessive_bools)]
pub struct App {
    pub settings: Settings,
    pub bindings: Bindings,
    pub model: PlayerModel,
    store: ResumeStore,
    library: Library,
    tracking: Tracking,
    /// The end of the file that plays was handled (moved on or stayed).
    end_handled: bool,
    /// RVP's audio filters changed near the end of a file ([`NEAR_END`])
    /// and wait for the next file or for playback to leave the end
    /// (`App::defer_chain_change`).
    chain_pending: bool,
    /// The entry being loaded is to play: mpv may still carry the pause
    /// keep-open set at the end of the one before (seen on the Windows VM),
    /// so it is unpaused again once loaded.
    play_on_load: bool,
    /// Where settings changed from the window (normalizer) are saved.
    settings_path: Option<PathBuf>,
    writer: storage::Writer,
    /// The albums' file did not exist: nothing was restored.
    first_start: bool,
    /// Warnings from loading the settings, for the shell to show once.
    pub warnings: Vec<String>,
    /// Picture and playback state for this session (M4's commands).
    pub(crate) session: Session,
    /// Where the resume store, the albums and generated files live.
    state_dir: PathBuf,
    /// SVP mode's watch over the file that plays (M6).
    pub(crate) svp: crate::svp::Watch,
}

impl App {
    /// Loads `settings.json` (writing the defaults when it is missing), the
    /// resume store and the albums.
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
        let mut app = Self::new(settings, &dirs.state)?;
        app.settings_path = Some(settings_path);
        Ok(app)
    }

    /// # Errors
    /// Fails when the resume store or the albums cannot be read.
    pub fn new(mut settings: Settings, state_dir: &Path) -> io::Result<Self> {
        let mut warnings = settings.validate();
        let (bindings, binding_warnings) = settings.bindings();
        warnings.extend(binding_warnings);
        let first_start = !state_dir.join("playlists.json").exists();
        Ok(Self {
            settings,
            bindings,
            model: PlayerModel::new(),
            store: ResumeStore::open(state_dir)?,
            library: Library::open(state_dir)?,
            tracking: Tracking::On,
            end_handled: false,
            play_on_load: false,
            chain_pending: false,
            settings_path: None,
            writer: storage::Writer::new()?,
            first_start,
            warnings,
            session: Session::default(),
            state_dir: state_dir.to_path_buf(),
            svp: crate::svp::Watch::default(),
        })
    }

    /// The engine options these settings ask for.
    #[must_use]
    pub fn engine_options(&self) -> EngineOptions {
        let mut options = EngineOptions::player();
        options.hwdec = self.effective_hwdec();
        options.ipc_server = self.svp.endpoint.clone().or_else(|| {
            Some(self.settings.ipc_server.trim().to_owned()).filter(|server| !server.is_empty())
        });
        options.extra.push((
            "volume-max".to_owned(),
            format!("{:.0}", self.settings.volume_max.max(100.0)),
        ));
        let settings = &self.settings;
        for (name, value) in [
            ("audio-device", settings.audio_device.as_str()),
            ("video-sync", settings.video_sync.as_str()),
            ("scale", settings.scaler.as_str()),
            ("slang", settings.subtitle_languages.as_str()),
            ("sub-auto", settings.subtitle_auto.as_str()),
        ] {
            if !value.trim().is_empty() {
                options
                    .extra
                    .push((name.to_owned(), value.trim().to_owned()));
            }
        }
        options.extra.push((
            "sub-scale".to_owned(),
            format!("{:.2}", settings.subtitle_scale),
        ));
        if !settings.subtitle_font.is_empty() {
            options
                .extra
                .push(("sub-font".to_owned(), settings.subtitle_font.clone()));
        }
        options
    }

    /// Where the settings are saved (`None` in tests that give settings
    /// directly).
    #[must_use]
    pub fn settings_path(&self) -> Option<&Path> {
        self.settings_path.as_deref()
    }

    /// The state folder (resume store, albums, generated files).
    #[must_use]
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    #[must_use]
    pub fn store(&self) -> &ResumeStore {
        &self.store
    }

    pub(crate) fn store_mut(&mut self) -> &mut ResumeStore {
        &mut self.store
    }

    /// No import of PotPlayer's playlists at this start (the self-test
    /// must not work on the owner's albums).
    pub fn skip_first_import(&mut self) {
        self.first_start = false;
    }

    #[must_use]
    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Changes to the albums that the panel makes directly (selection,
    /// scrolling, tabs, dropped rows).
    pub fn library_mut(&mut self) -> &mut Library {
        &mut self.library
    }

    /// Restores volume, mute and the normalizer, then opens `files`, or
    /// without files the entry that played last, paused at its position
    /// (`reopen-last-file`). On the first start, PotPlayer's playlists are
    /// imported when their folder is known.
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn start(&mut self, engine: &Engine, files: &[String]) -> Result<(), EngineError> {
        let state = self.store.state();
        engine.set_volume(state.volume.unwrap_or(100.0))?;
        engine.set_mute(state.mute)?;
        let last_file = state.last_file.clone();
        self.apply_normalizer(engine);
        self.apply_equalizer(engine);
        if let Err(error) = self.apply_picture_settings(engine) {
            self.warnings.push(format!("Picture settings: {error}"));
        }
        if let Err(error) = self.apply_video_filters(engine) {
            self.warnings.push(format!("Video filters: {error}"));
        }
        if self.first_start
            && let Some(folder) = self.potplayer_folder()
            && let Ok(count) = self.import_playlists(&folder)
            && count > 0
        {
            self.warnings
                .push(format!("Imported {count} albums from PotPlayer"));
        }
        if !files.is_empty() {
            return self.open(engine, files, false);
        }
        if !self.settings.reopen_last_file {
            return Ok(());
        }
        if let (Some(album), Some((index, _))) =
            (self.library.lists().playing, self.library.playing())
        {
            return self.play_entry(engine, album, index, true);
        }
        // From M2's state: the last file without an album.
        if let Some(last) = last_file {
            let shown = self.library.lists().shown;
            if let Some(index) = self.library.insert(shown, None, &[last]) {
                return self.play_entry(engine, shown, index, true);
            }
        }
        Ok(())
    }

    /// Opens files: they are added to the shown album and the first one
    /// plays, or with `append` they are only added.
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn open(
        &mut self,
        engine: &Engine,
        files: &[String],
        append: bool,
    ) -> Result<(), EngineError> {
        let shown = self.library.lists().shown;
        let Some(first) = self.library.insert(shown, None, files) else {
            return Ok(());
        };
        if append {
            Ok(())
        } else {
            self.play_entry(engine, shown, first, false)
        }
    }

    /// Adds files after the entry that plays (PotPlayer's `/insert`).
    pub fn insert_after_current(&mut self, files: &[String]) {
        let lists = self.library.lists();
        let (album, at) = match (lists.playing, self.library.playing()) {
            (Some(album), Some((index, _))) => (album, Some(index + 1)),
            _ => (lists.shown, None),
        };
        self.library.insert(album, at, files);
    }

    /// Carries out a second instance's request (or this one's own `/add`
    /// or `/insert` at start).
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn remote(&mut self, engine: &Engine, request: &Request) -> Result<(), EngineError> {
        match request.mode {
            Mode::Play => self.open(engine, &request.files, false),
            Mode::Add => self.open(engine, &request.files, true),
            Mode::Insert => {
                self.insert_after_current(&request.files);
                Ok(())
            }
        }
    }

    /// Carries out what the pointer did in the playlist panel; `visible`
    /// is the number of rows the panel shows. The drop marker and the menu
    /// are the window's.
    pub fn list_effect(
        &mut self,
        engine: &Engine,
        effect: ListEffect,
        visible: usize,
    ) -> Option<UiRequest> {
        let album = self.library.lists().shown;
        match effect {
            ListEffect::Select { index, how } => {
                self.library.select(album, index, how);
            }
            ListEffect::Play(index) => {
                self.library.select(album, index, Select::Only);
                if let Err(error) = self.play_entry(engine, album, index, false) {
                    return Some(UiRequest::Message(format!("Cannot play: {error}")));
                }
            }
            ListEffect::ShowAlbum(target) => {
                self.library.show_album(target);
            }
            ListEffect::DeleteAlbum(target) => {
                self.library.remove_album(target);
            }
            ListEffect::ScrollBy(rows) => {
                let top = self.library.shown().top;
                let top = if rows < 0 {
                    top.saturating_sub(rows.unsigned_abs() as usize)
                } else {
                    top.saturating_add(rows.unsigned_abs() as usize)
                };
                self.library.scroll_to(album, top, visible);
            }
            ListEffect::ScrollTo(top) => {
                self.library.scroll_to(album, top, visible);
            }
            ListEffect::MoveBefore(index) => {
                self.library.move_selected(album, Move::Before(index));
            }
            ListEffect::Action(action) => return self.list_action(engine, action, false, visible),
            ListEffect::DropMarker(_) | ListEffect::Menu => {}
        }
        None
    }

    /// Plays an album entry; `paused` opens it paused (reopening at start).
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn play_entry(
        &mut self,
        engine: &Engine,
        album: usize,
        index: usize,
        paused: bool,
    ) -> Result<(), EngineError> {
        let Some(path) = self
            .library
            .lists()
            .albums
            .get(album)
            .and_then(|target| target.entries.get(index))
            .map(|entry| entry.path.clone())
        else {
            return Ok(());
        };
        self.library.set_playing(album, index);
        self.end_handled = false;
        self.play_on_load = !paused;
        self.tracking = Tracking::Loading;
        engine.set_pause(paused)?;
        engine.load(&path, false)
    }

    /// Moves to the next (`1`) or previous (`-1`) playable entry.
    fn step(&mut self, engine: &Engine, step: i32) -> Result<(), EngineError> {
        match self.library.neighbour(step) {
            Some((album, index)) => self.play_entry(engine, album, index, false),
            None => Ok(()),
        }
    }

    /// Whether an action's menu entry shows a check mark, and if so its
    /// state.
    #[must_use]
    pub fn checked(&self, action: Action) -> Option<bool> {
        match action {
            Action::ToggleNormalizer => Some(self.settings.normalizer.enabled),
            Action::ToggleBoost => Some(self.settings.normalizer.boost),
            Action::ToggleMute => Some(self.model.flag(PropertyId::Mute) == Some(true)),
            Action::ToggleSubtitles => {
                Some(self.model.flag(PropertyId::SubVisibility) != Some(false))
            }
            Action::ToggleKeyframeSeeks => Some(self.settings.keyframe_seeks),
            Action::SetRepeat(repeat) => Some(self.settings.repeat == repeat),
            Action::SetOnTop(mode) => Some(self.settings.on_top == mode),
            Action::SetAfterPlayback(after) => Some(self.session.after == after),
            Action::SetAspect(aspect) => Some(self.session.aspect == aspect),
            Action::VideoFit(fit) => Some(self.session.fit == fit),
            Action::SetImageFormat(format) => Some(self.settings.image_format == format),
            Action::PlaybackInfo => Some(self.session.info),
            Action::ToggleEqualizer => Some(self.settings.equalizer.enabled),
            Action::Color(crate::video::ColorChange::Toggle) => Some(self.settings.color.enabled),
            Action::SetDeinterlace(mode) => Some(self.settings.deinterlace == mode),
            Action::VideoEffect(effect) => Some(self.settings.effects.contains(&effect)),
            Action::ToggleDeband => Some(self.settings.deband),
            Action::Rotate(crate::video::Rotation::To(degrees)) => {
                Some(self.session.rotation == degrees % 360)
            }
            Action::Shaders(
                crate::video::ShaderChange::Toggle | crate::video::ShaderChange::On,
            ) => Some(self.settings.shaders.enabled),
            Action::Shaders(crate::video::ShaderChange::Off) => {
                Some(!self.settings.shaders.enabled)
            }
            Action::ToggleVapourSynth => Some(self.settings.vapoursynth.enabled),
            Action::ControlPanel => Some(self.session.control_panel),
            _ => None,
        }
    }

    /// The context menu entry of an action: PotPlayer's text with the seek
    /// step filled in, and the key shown next to it (PotPlayer's key while
    /// it is still bound to the action, otherwise the simplest bound key).
    #[must_use]
    pub fn menu_entry(&self, action: Action) -> (String, String) {
        let (text, shown) = action.menu_text();
        let label = self.fill_step(action, text);
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

    /// PotPlayer's menu text with the step's length in place of `%s`
    /// (`%s Backward` → `30 Sec Backward`).
    #[must_use]
    pub fn fill_step(&self, action: Action, text: &str) -> String {
        match action {
            Action::SeekBackward(step) | Action::SeekForward(step) => {
                text.replace("%s", &step_label(self.settings.seek_seconds(step)))
            }
            Action::AudioDelay(Delay::Earlier | Delay::Later) => {
                text.replace("%s", &sync_step_label(self.settings.audio_delay_step))
            }
            Action::SubtitleDelay(Delay::Earlier | Delay::Later) => {
                text.replace("%s", &sync_step_label(self.settings.subtitle_delay_step))
            }
            _ => text.to_owned(),
        }
        .trim()
        .to_owned()
    }

    /// Shows an album and plays its current (or first playable) entry.
    pub fn play_album(&mut self, engine: &Engine, album: usize) {
        self.library.show_album(album);
        let target = self.library.lists().albums.get(album).and_then(|target| {
            target
                .current
                .or_else(|| target.entries.iter().position(|entry| entry.playable))
        });
        if let Some(index) = target {
            drop(self.play_entry(engine, album, index, false));
        }
    }

    /// The playlist menu entry of a playlist action and its bound key.
    #[must_use]
    pub fn list_menu_entry(&self, action: ListAction) -> (String, String) {
        let shortcut = self
            .bindings
            .list_keys
            .iter()
            .filter(|&(_, &bound)| bound == action)
            .map(|(chord, _)| *chord)
            .min_by_key(|chord| (u8::from(chord.ctrl) + u8::from(chord.alt), *chord))
            .map(|chord| chord.to_string())
            .unwrap_or_default();
        (action.label().to_owned(), shortcut)
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
            Action::ImportPlaylists => return Some(UiRequest::ImportPlaylists),
            Action::ToggleSubtitles => engine.toggle_subtitles(),
            Action::ToggleNormalizer => {
                self.settings.normalizer.enabled = !self.settings.normalizer.enabled;
                return Some(self.normalizer_changed(engine));
            }
            Action::ToggleBoost => {
                self.settings.normalizer.boost = !self.settings.normalizer.boost;
                return Some(self.normalizer_changed(engine));
            }
            Action::PlayPause => {
                if !self.model.media_loaded() {
                    return self.play_shown(engine);
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
            Action::Previous => self.step(engine, -1),
            Action::Next => self.step(engine, 1),
            Action::SeekBackward(step) => {
                self.seek_to(engine, Seek::Relative(-self.settings.seek_seconds(step)))
            }
            Action::SeekForward(step) => {
                self.seek_to(engine, Seek::Relative(self.settings.seek_seconds(step)))
            }
            Action::VolumeUp => return self.change_volume(engine, self.settings.volume_step),
            Action::VolumeDown => return self.change_volume(engine, -self.settings.volume_step),
            Action::ToggleMute => {
                let mute = self.model.flag(PropertyId::Mute) != Some(true);
                return engine
                    .set_mute(mute)
                    .ok()
                    .map(|()| UiRequest::Osd(if mute { "Mute" } else { "Sound on" }.to_owned()));
            }
            other => return self.command(engine, other),
        };
        // Commands that do not apply (nothing to seek) are ignored, as
        // PotPlayer does.
        drop(result);
        None
    }

    /// Play with nothing loaded: the shown album's selected or current
    /// entry, else the file dialog.
    fn play_shown(&mut self, engine: &Engine) -> Option<UiRequest> {
        let shown = self.library.lists().shown;
        let target = self
            .library
            .cursor(shown)
            .or_else(|| (!self.library.shown().entries.is_empty()).then_some(0));
        match target {
            Some(index) => {
                drop(self.play_entry(engine, shown, index, false));
                None
            }
            None => Some(UiRequest::OpenFileDialog),
        }
    }

    /// Carries out a playlist action on the shown album. `extend` (Shift)
    /// turns cursor moves into selecting; `visible` is the number of rows
    /// the panel shows (for paging and scrolling).
    pub fn list_action(
        &mut self,
        engine: &Engine,
        action: ListAction,
        extend: bool,
        visible: usize,
    ) -> Option<UiRequest> {
        let album = self.library.lists().shown;
        let len = self.library.shown().entries.len();
        let cursor = self.library.cursor(album);
        let page = visible.max(1);
        let target = match action {
            ListAction::CursorUp => Some(cursor.map_or(0, |c| c.saturating_sub(1))),
            ListAction::CursorDown => Some(cursor.map_or(0, |c| c + 1)),
            ListAction::CursorHome => Some(0),
            ListAction::CursorEnd => Some(len.saturating_sub(1)),
            ListAction::PageUp => Some(cursor.map_or(0, |c| c.saturating_sub(page))),
            ListAction::PageDown => Some(cursor.map_or(0, |c| c + page)),
            _ => None,
        };
        if let Some(index) = target {
            if len > 0 {
                let index = index.min(len - 1);
                let how = if extend { Select::Range } else { Select::Only };
                self.library.select(album, index, how);
                self.library.reveal(album, index, visible);
            }
            return None;
        }
        // Whether the album changed is already in the library's state.
        let _changed = match action {
            ListAction::Play => {
                cursor.is_some_and(|index| self.play_entry(engine, album, index, false).is_ok())
            }
            ListAction::SelectAll => self.library.select_all(album),
            ListAction::InvertSelection => self.library.invert_selection(album),
            ListAction::RemoveSelected => self.library.remove_selected(album),
            ListAction::RemoveUnselected => self.library.remove_unselected(album),
            ListAction::RemoveAll => self.library.clear(album),
            ListAction::RemoveMissing => self.library.remove_missing(album),
            ListAction::SortAscending => self.library.sort(album, Order::Ascending),
            ListAction::SortDescending => self.library.sort(album, Order::Descending),
            ListAction::Shuffle => {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |elapsed| elapsed.as_secs());
                self.library.sort(album, Order::Random(seed))
            }
            ListAction::MoveUp => self.library.move_selected(album, Move::Up),
            ListAction::MoveDown => self.library.move_selected(album, Move::Down),
            ListAction::MoveTop => self.library.move_selected(album, Move::Top),
            ListAction::MoveBottom => self.library.move_selected(album, Move::Bottom),
            ListAction::Undo => self.library.undo(),
            ListAction::Redo => self.library.redo(),
            ListAction::DeleteAlbum => self.library.remove_album(album),
            ListAction::AddFiles => return Some(UiRequest::AddFilesDialog),
            ListAction::AddFolder => return Some(UiRequest::AddFolderDialog),
            ListAction::AddAlbum => {
                return Some(UiRequest::AlbumName {
                    rename: false,
                    current: String::new(),
                });
            }
            ListAction::RenameAlbum => {
                return Some(UiRequest::AlbumName {
                    rename: true,
                    current: self.library.shown().name.clone(),
                });
            }
            ListAction::Close => return Some(UiRequest::TogglePlaylist),
            ListAction::CursorUp
            | ListAction::CursorDown
            | ListAction::CursorHome
            | ListAction::CursorEnd
            | ListAction::PageUp
            | ListAction::PageDown => false,
        };
        None
    }

    /// Names a new album, or renames the shown one.
    pub fn name_album(&mut self, name: &str, rename: bool) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if rename {
            let shown = self.library.lists().shown;
            self.library.rename_album(shown, name);
        } else {
            self.library.add_album(name);
        }
    }

    /// Adds the media files in `folder` and its subfolders to the shown
    /// album, sorted by path.
    pub fn add_folder(&mut self, folder: &Path) -> usize {
        let mut files = Vec::new();
        collect_media(folder, 0, &mut files);
        files.sort();
        let shown = self.library.lists().shown;
        self.library.insert(shown, None, &files);
        files.len()
    }

    /// Opens the media files in `folder` and its subfolders (PotPlayer's
    /// "Open Folder"): they join the shown album and the first one plays.
    /// Returns how many there were.
    ///
    /// # Errors
    /// Fails when libmpv rejects a command.
    pub fn open_folder(&mut self, engine: &Engine, folder: &Path) -> Result<usize, EngineError> {
        let mut files = Vec::new();
        collect_media(folder, 0, &mut files);
        files.sort();
        if !files.is_empty() {
            self.open(engine, &files, false)?;
        }
        Ok(files.len())
    }

    /// Loads a subtitle file for the playing video and shows it.
    pub fn load_subtitle(&mut self, engine: &Engine, path: &Path) -> Option<UiRequest> {
        if !self.model.media_loaded() {
            return Some(UiRequest::Osd("Open a video first".to_owned()));
        }
        let text = path.to_string_lossy();
        match engine.command("sub-add", &[&text, "select"]) {
            Ok(()) => Some(UiRequest::Osd(format!(
                "Subtitles: {}",
                crate::view::file_name(&text)
            ))),
            Err(error) => Some(UiRequest::Message(format!("Cannot load {text}: {error}"))),
        }
    }

    /// Where PotPlayer's playlists are: the setting, else on Windows
    /// `%APPDATA%\PotPlayerMini64\Playlist`.
    #[must_use]
    pub fn potplayer_folder(&self) -> Option<PathBuf> {
        self.settings
            .potplayer_playlists
            .clone()
            .filter(|folder| folder.is_dir())
            .or_else(dpl::potplayer_folder)
    }

    /// Imports PotPlayer's playlists from `folder` (read-only) as albums,
    /// with their resume positions; returns how many albums came in.
    ///
    /// # Errors
    /// Fails when the folder cannot be read.
    pub fn import_playlists(&mut self, folder: &Path) -> io::Result<usize> {
        // Windows opens PotPlayer's paths as they are.
        let map = if cfg!(windows) {
            DriveMap::default()
        } else {
            DriveMap::new(&self.settings.drive_map)
        };
        let imported = dpl::import_dir(folder, &map)?;
        let count = imported.len();
        let mut first = None;
        for item in imported {
            let mut album = item.album;
            let fills_empty = self
                .library
                .lists()
                .albums
                .iter()
                .any(|existing| existing.name == album.name && existing.entries.is_empty());
            if !fills_empty {
                album.name = self.unique_album_name(&album.name);
            }
            let index = self.library.import_album(album);
            first.get_or_insert(index);
            for (path, seconds) in item.positions {
                if seconds >= EDGE_SECONDS {
                    self.store.set_position(&path, seconds, None);
                }
            }
        }
        if let Some(first) = first {
            self.library.show_album(first);
        }
        drop(self.store.save(Instant::now()));
        Ok(count)
    }

    fn unique_album_name(&self, name: &str) -> String {
        let taken = |candidate: &str| {
            self.library
                .lists()
                .albums
                .iter()
                .any(|album| album.name == candidate)
        };
        if !taken(name) {
            return name.to_owned();
        }
        // There are fewer albums than numbers to try.
        let albums = self.library.lists().albums.len();
        (2..=albums + 1)
            .map(|n| format!("{name} ({n})"))
            .find(|candidate| !taken(candidate))
            .unwrap_or_else(|| name.to_owned())
    }

    /// Applies the normalizer settings to mpv's audio filters: the
    /// normalizer, then the tempo filter that keeps speed changes from
    /// putting audio and video out of step (`crate::normalizer`).
    pub fn apply_normalizer(&self, engine: &Engine) {
        let enabled = self.settings.normalizer.enabled;
        let entry = enabled.then(|| self.settings.normalizer.chain_entry());
        if let Err(error) = engine.set_audio_filter(crate::normalizer::LABEL, entry.as_deref()) {
            eprintln!("rvp: cannot set the normalizer: {error}");
        }
        // Re-added after the normalizer, so it stays last.
        let tempo = enabled.then_some(crate::normalizer::TEMPO_ENTRY);
        if let Err(error) = engine.set_audio_filter(crate::normalizer::TEMPO_LABEL, tempo) {
            eprintln!("rvp: cannot set the tempo filter: {error}");
        }
    }

    /// True in the last [`NEAR_END`] seconds of the loaded file or at its
    /// end.
    fn near_end(&self) -> bool {
        if !self.model.media_loaded() {
            return false;
        }
        if self.model.flag(PropertyId::EofReached) == Some(true) {
            return true;
        }
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        self.model
            .double(PropertyId::Duration)
            .is_some_and(|duration| duration - time < NEAR_END)
    }

    /// Whether a change of RVP's audio filters waits: a chain change or a
    /// seek that reaches mpv at the very end of a file can leave its
    /// `eof-reached` false for good (seen on the Windows VM), and the album
    /// would not move on. The filters are applied when the next file starts
    /// or playback leaves the end.
    pub(crate) fn defer_chain_change(&mut self) -> bool {
        if self.near_end() {
            self.chain_pending = true;
        }
        self.chain_pending
    }

    /// Applies the audio filters a change left waiting; `resync` plays on
    /// from the current position (not needed when a file starts).
    fn apply_pending_chain(&mut self, engine: &Engine, resync: bool) {
        if !std::mem::take(&mut self.chain_pending) {
            return;
        }
        self.apply_normalizer(engine);
        self.apply_equalizer(engine);
        if resync {
            self.resync_audio(engine);
        }
    }

    /// Plays on from the current position: the audio chain changed and
    /// the audio mpv had decoded ahead for the old one (the normalizer looks
    /// ten seconds ahead) would put audio and video out of step. Never near
    /// the end (`App::defer_chain_change`).
    pub(crate) fn resync_audio(&self, engine: &Engine) {
        if self.model.media_loaded() && !self.near_end() {
            drop(engine.seek(Seek::Relative(0.0)));
        }
    }

    fn normalizer_changed(&mut self, engine: &Engine) -> UiRequest {
        if !self.defer_chain_change() {
            self.apply_normalizer(engine);
            self.resync_audio(engine);
        }
        self.save_settings();
        let normalizer = self.settings.normalizer;
        let on_off = |on: bool| if on { "On" } else { "Off" };
        UiRequest::Osd(format!(
            "Normalizer: {}, Boost: {}",
            on_off(normalizer.enabled),
            on_off(normalizer.boost)
        ))
    }

    /// Writes `settings.json` (on the writer thread) after a change made in
    /// the window.
    pub fn save_settings(&self) {
        let Some(path) = &self.settings_path else {
            return;
        };
        match serde_json::to_vec_pretty(&self.settings) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                self.writer.write(path, bytes);
            }
            Err(error) => eprintln!("rvp: cannot serialize the settings: {error}"),
        }
    }

    /// Replaces the bindings (from the key and mouse editor) and saves them
    /// as changes over PotPlayer's defaults.
    pub fn set_bindings(&mut self, bindings: Bindings) {
        let defaults = Bindings::potplayer_defaults();
        let (keys, list_keys, mouse) = crate::binding_editor::overrides(&defaults, &bindings);
        self.settings.keys = keys;
        self.settings.playlist_keys = list_keys;
        self.settings.mouse = mouse;
        self.bindings = bindings;
        self.save_settings();
    }

    pub(crate) fn seek_to(&mut self, engine: &Engine, target: Seek) -> Result<(), EngineError> {
        if !self.model.media_loaded() {
            return Ok(());
        }
        // A user seek ends any pending resume.
        if matches!(self.tracking, Tracking::Resuming(_)) {
            self.tracking = Tracking::On;
        }
        engine.seek_with(target, !self.settings.keyframe_seeks)
    }

    fn change_volume(&mut self, engine: &Engine, delta: f64) -> Option<UiRequest> {
        let current = self.model.double(PropertyId::Volume).unwrap_or(100.0);
        let volume = (current + delta).clamp(0.0, self.settings.volume_max.max(100.0));
        engine
            .set_volume(volume)
            .ok()
            .map(|()| UiRequest::Osd(format!("Volume: {volume:.0} %")))
    }

    /// The seek bar was clicked or dragged to `fraction` (0.0 to 1.0).
    pub fn seek_to_fraction(&mut self, engine: &Engine, fraction: f64) {
        drop(self.seek_to(engine, Seek::Fraction(fraction)));
    }

    /// The volume slider was set to `fraction` (0.0 to 1.0 of 100 %).
    pub fn set_volume_fraction(&mut self, engine: &Engine, fraction: f64) {
        drop(engine.set_volume((fraction * 100.0).clamp(0.0, 100.0)));
    }

    /// Handles everything libmpv has reported: updates the model, the
    /// albums and the resume store, moves on at the end of a file, and
    /// returns the changes, requests for the window and the events.
    pub fn drain(&mut self, engine: &Engine, now: Instant) -> Drained {
        let mut events = Vec::new();
        let changes = engine.drain(&mut self.model, &mut events);
        let mut requests = Vec::new();
        let mut save_now = false;
        let mut failed = false;
        for event in &events {
            match event {
                EngineEvent::StartFile => {
                    self.tracking = Tracking::Loading;
                    // A-B points belong to one file.
                    self.session.point_a = None;
                    self.session.point_b = None;
                    self.session.ab_active = false;
                }
                EngineEvent::FileLoaded { path } => {
                    self.file_loaded(engine, path, now);
                    save_now = true;
                }
                EngineEvent::EndFile {
                    reason: EndReason::Eof,
                    path: Some(path),
                } => {
                    self.store.clear_position(path);
                    save_now = true;
                }
                EngineEvent::EndFile {
                    reason: EndReason::Error,
                    ..
                } => {
                    failed = true;
                    save_now = true;
                }
                EngineEvent::EndFile { .. } => save_now = true,
                EngineEvent::Error(message) => {
                    requests.push(UiRequest::Message(format!("Cannot play: {message}")));
                }
                EngineEvent::PlaybackRestart | EngineEvent::Shutdown => {}
            }
        }
        if failed {
            // `<Can't open>`, then the next playable entry.
            self.library.update_current(None, false);
            drop(self.step(engine, 1));
        }
        if changes.contains(PropertyId::Duration)
            && let Some(duration) = self.model.double(PropertyId::Duration).filter(|d| *d > 0.0)
        {
            self.library.update_current(Some(duration), true);
        }

        requests.extend(self.track_position(engine));
        if let Some(top) = self.on_top_change() {
            requests.push(UiRequest::OnTop(top));
        }
        requests.extend(self.watch_svp(now));
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
        let failure = saved
            .err()
            .map(|error| error.to_string())
            .or_else(|| self.store.take_error())
            .or_else(|| self.library.take_error())
            .or_else(|| self.writer.take_error());
        if let Some(error) = failure {
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

    /// A file is open: A-B points, SVP's watch, the last file, and the
    /// resume position.
    fn file_loaded(&mut self, engine: &Engine, path: &str, now: Instant) {
        if std::mem::take(&mut self.play_on_load) {
            drop(engine.set_pause(false));
        }
        self.apply_pending_chain(engine, false);
        self.apply_ab(engine);
        self.svp
            .file_started(crate::svp::svp_mode(&self.settings), now);
        self.store.set_last_file(path);
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

    /// Records the position of the playing file and moves on at its end.
    fn track_position(&mut self, engine: &Engine) -> Option<UiRequest> {
        let time = self.model.double(PropertyId::TimePos);
        if let (Tracking::Resuming(target), Some(time)) = (self.tracking, time)
            && (time - target).abs() < 2.0
        {
            self.tracking = Tracking::On;
        }
        let at_end = self.model.flag(PropertyId::EofReached) == Some(true);
        if !at_end {
            // Playback left the end (a seek back): the next end counts.
            self.end_handled = false;
        }
        if self.chain_pending && !self.near_end() {
            self.apply_pending_chain(engine, true);
        }
        if self.tracking == Tracking::On
            && let (Some(path), Some(time)) = (self.model.loaded_path(), time)
        {
            if at_end {
                self.store.clear_position(path);
            } else {
                let duration = self.model.double(PropertyId::Duration);
                self.store.set_position(path, time, duration);
            }
        }
        // While the next file loads, the model still holds the old one's
        // end until mpv reports the load: moving on again then would skip
        // an entry (goal I, seen on the Windows VM).
        if at_end
            && !self.end_handled
            && self.model.media_loaded()
            && self.tracking != Tracking::Loading
        {
            // keep-open stops on the last frame; the album moves on.
            self.end_handled = true;
            return self.end_of_file(engine);
        }
        None
    }

    /// The file played to its end: repeat it, move on, start the album
    /// again or exit, as the repeat mode and "When Playback Finished" say.
    fn end_of_file(&mut self, engine: &Engine) -> Option<UiRequest> {
        if self.session.after == AfterPlayback::ExitAfterFile {
            return Some(UiRequest::Exit);
        }
        if self.settings.repeat == Repeat::Item {
            drop(
                engine
                    .seek(Seek::Absolute(0.0))
                    .and_then(|()| engine.set_pause(false)),
            );
            self.end_handled = false;
            return None;
        }
        let next = self.library.neighbour(1).or_else(|| {
            (self.settings.repeat == Repeat::Album)
                .then(|| self.library.first_playable())
                .flatten()
        });
        match next {
            Some((album, index)) => {
                drop(self.play_entry(engine, album, index, false));
                None
            }
            None if self.session.after == AfterPlayback::ExitAfterList => Some(UiRequest::Exit),
            None => None,
        }
    }

    /// Saves a pending change once the save interval has passed; call it
    /// about once a second.
    ///
    /// # Errors
    /// Fails on I/O errors.
    pub fn tick(&mut self, now: Instant) -> io::Result<()> {
        self.store.save_if_due(now)?;
        match self.store.take_error() {
            Some(error) => Err(io::Error::other(error)),
            None => Ok(()),
        }
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
        self.writer.flush();
        self.library.flush()?;
        self.store.save_and_wait(now)
    }
}

/// Media files under `folder`, at most eight levels deep.
fn collect_media(folder: &Path, depth: usize, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if depth < 8 {
                collect_media(&path, depth + 1, files);
            }
        } else if path.extension().is_some_and(|ext| {
            MEDIA_EXTENSIONS
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        }) {
            files.push(path.to_string_lossy().into_owned());
        }
    }
}
