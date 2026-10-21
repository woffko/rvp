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
use std::time::{Duration, Instant};

use rvp_engine::{
    Changes, EndReason, Engine, EngineError, EngineEvent, EngineOptions, PlayerModel, PropertyId,
    Seek,
};

use crate::action::{Action, ListAction};
use crate::action::{AfterPlayback, Delay, Fit, Repeat};
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
    /// Fullscreen on or off; `fit` is the aspect ratio mode this kind of
    /// fullscreen brings (stretched, or filled by keeping the aspect).
    ToggleFullscreen {
        fit: Option<Fit>,
    },
    /// A window size or move, or a window dialog (M12).
    Window(crate::window::WindowRequest),
    /// A picture dialog (`custom-aspect`, `extend-size`) with the value it
    /// starts from; the answer goes to [`App::picture_dialog_answer`].
    PictureDialog {
        kind: &'static str,
        value: String,
    },
    Minimize,
    MaximizeRestore,
    Exit,
    /// Bring the window to the front (the media controls' Raise).
    Raise,
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
    /// Capture the playing frame into a file or onto the clipboard (off
    /// the GUI thread, M12's `crate::capture`).
    Capture(crate::capture::CaptureJob),
    /// A save dialog for a stream recording, suggesting this file (then
    /// [`App::start_stream_record`]).
    RecordStreamDialog(PathBuf),
    /// A save dialog for a video recording (then
    /// [`App::start_video_record`]).
    RecordVideoDialog(PathBuf),
    /// Ask for the folder recorded videos go to.
    VideoFolderDialog,
    /// Encode a stretch of the file with a core of its own (Record Video,
    /// off the GUI thread).
    Encode(rvp_engine::encode::EncodeJob),
    /// The slideshow moves by this many slides.
    Slide(i32),
    /// The consecutive images (`consecutive`) or thumbnail image (`sheet`)
    /// dialog, from the capture settings; the answers go to
    /// [`App::consecutive_answer`] and [`App::sheet_answer`].
    CaptureDialog(&'static str),
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
    /// A save dialog for the current source or screen frame, suggesting
    /// this file (then `Player.save_frame_to`).
    SaveFrameDialog {
        path: PathBuf,
        shot: crate::capture::Shot,
    },
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
    /// A notice the user asked for (Shift+Tab's short info, M14): it shows
    /// even with general info messages off.
    Info(String),
    /// The Playback/System Info window (Ctrl+F1, M14).
    SystemInfo,
    /// The settings changed as a whole (M14's presets): the window follows
    /// them as after the settings window; the notice to show.
    SettingsChanged(String),
    /// Dropped files ask what to do (M14's advanced drag and drop).
    DropMenu(Vec<String>),
    /// A change of the system's volume (the master volume).
    SystemVolume(crate::audio::commands::SystemVolume),
    /// The Adjust Audio Sync dialog, at this sync (seconds; then
    /// `Player.set_audio_delay`).
    AudioSyncDialog(f64),
    /// The Adjust Subtitle Sync dialog, with the subtitles' sync in seconds
    /// (M13).
    SubtitleSyncDialog(f64),
    /// The file dialog for a second subtitle (Combine Subtitle).
    CombineSubtitleDialog,
    /// The subtitle browser and editor, or Input Subtitles (`input`; then
    /// `App::subtitle_browser_json` and `App::subtitle_edit`).
    SubtitleEditor {
        input: bool,
    },
    /// The save dialog for the open subtitles, suggesting this file (then
    /// `App::save_subtitle_as`).
    SaveSubtitleDialog(PathBuf),
    /// A Skins menu choice changed the skin, its mode or theme: the shell
    /// shows `App::skin_want` and this notice (M14).
    SkinChanged(String),
    /// The colour theme's Customize dialog.
    ColorThemeDialog,
    /// A save dialog for an audio recording, suggesting this file.
    RecordDialog(PathBuf),
    /// Start or stop recording what RVP plays.
    Record(crate::audio::commands::Record),
    /// Ask for the folder recordings go to.
    RecordFolderDialog,
    /// A non-blocking message (section 4: no modal interruptions).
    Message(String),
    // M11.
    /// The Jump to Time/Frame dialog, at the current time (seconds), with
    /// the duration and the frame rate (0 when unknown).
    JumpDialog {
        time: f64,
        duration: f64,
        fps: f64,
    },
    /// The A-B repeat list's dialog (`Player.sections_json`).
    SectionList,
    /// The bookmark list's dialog (`Player.bookmarks_json`).
    Bookmarks,
    /// Ask for a new bookmark's title (then `Player.add_bookmark`).
    BookmarkTitle {
        time: f64,
        title: String,
    },
    /// The notice before an ending: its text and the seconds left, or
    /// hidden (`None`).
    PowerNotice(Option<(String, u64)>),
    /// Shut down, sleep or hibernate the computer now (the shell's
    /// platform code, or its dry-run hook).
    Power(crate::action::PowerAction),
    /// The shutdown timer's dialog.
    PowerTimerDialog(crate::action::PowerTimer),
    /// Ask whether these files may go to the Recycle Bin (then
    /// `Player.recycle`); `entries` are the album entries to remove after.
    ConfirmRecycle {
        files: Vec<PathBuf>,
        entries: Vec<String>,
    },
    /// Put this text on the clipboard.
    CopyText(String),
    /// Read the clipboard and give its text to `Player.paste_text`, for
    /// the purpose named.
    ReadClipboard(crate::sources::Paste),
    /// A file dialog for an external audio file (then
    /// `Player.load_external_audio`).
    OpenAudioDialog,
    /// A folder or file dialog for a disc: `bluray` for Blu-ray, `files`
    /// for its `.IFO` or `.MPLS` files (then `Player.open_disc`).
    DiscDialog {
        bluray: bool,
        files: bool,
    },
    /// The list of capture devices to choose from (then
    /// `Player.open_device`).
    DeviceDialog(Vec<crate::sources::CaptureDevice>),
    /// The scene browser: its title, the pictures' places and sizes.
    SceneBrowser(crate::scenes::Browser),
    /// The favorites' dialog.
    Favorites,
    /// A dialog of the playlist panel (`kind`: `title`, `rename`,
    /// `search`, `url`, `info`, `save`, `open`, `move`, `copy`), with the
    /// current text.
    ListDialog {
        kind: &'static str,
        text: String,
    },
    /// Start another RVP with these files (`rvp /new`).
    NewInstance(Vec<String>),
    /// Show a file in the system's file manager (selected when it can).
    ShowFile(PathBuf),
    /// The playlist's view changed (list or thumbnails): lay it out again.
    ListView,
    /// A built-in submenu at the pointer (`CMD_POPUPMENU_*`).
    PopupMenu(&'static str),
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

/// How long a released seek bar may show where it was let go before it
/// shows the playback position again, should mpv not get there (a
/// keyframe seek lands before the target).
const SEEK_BAR_HOLD: Duration = Duration::from_secs(2);

/// The seek bar held by the mouse, and after it is let go until the video
/// gets there: the bar and the time show this place, not where the video
/// is, so the bar follows the mouse at once (owner, 2026-10-15).
#[derive(Debug, Clone, Copy, PartialEq)]
struct SeekBarHold {
    fraction: f64,
    /// When the mouse let go.
    released: Option<Instant>,
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

/// File extensions "Add folder" picks up.
const MEDIA_EXTENSIONS: &[&str] = &[
    "3gp", "aac", "ac3", "aiff", "ape", "asf", "avi", "avs", "dts", "flac", "flv", "m2ts", "m4a",
    "m4v", "mka", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "mts", "ogg", "ogm", "ogv", "opus",
    "rm", "rmvb", "ts", "vob", "wav", "webm", "wma", "wmv", "wv",
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
    /// `start` restored the saved volume and mute. Until then mpv's reports
    /// (its defaults) must not replace them: the window starts the app only
    /// once its video output is ready, and mpv reports its volume before.
    started: bool,
    /// RVP's audio filters changed near the end of a file ([`NEAR_END`])
    /// and wait for the next file or for playback to leave the end
    /// (`App::defer_chain_change`).
    pub(crate) chain_pending: bool,
    /// RVP's video filters wait the same way (`App::apply_video_filters`):
    /// a `vf` change in the last moments of a file can leave mpv stuck
    /// before its end (M12).
    pub(crate) video_pending: std::cell::Cell<bool>,
    /// The entry being loaded is to play: mpv may still carry the pause
    /// keep-open set at the end of the one before (seen on the Windows VM),
    /// so it is unpaused again once loaded.
    play_on_load: bool,
    /// Where settings changed from the window (normalizer) are saved.
    pub(crate) settings_path: Option<PathBuf>,
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
    /// The normalizer's gain control (`App::start` starts it).
    pub(crate) normalizer: Option<crate::normalizer::Control>,
    /// What the audio chain knows about the file that plays.
    pub(crate) audio_source: crate::audio::chain::Source,
    /// mpv's audio devices, read at start and when the settings window
    /// asks (asking the system can take long).
    pub audio_devices: Vec<crate::menu::Device>,
    /// The `af` value RVP set last (`App::apply_audio_chain`).
    applied_chain: Option<String>,
    /// Freeverb's response for the settings it was made for: computing it
    /// takes a few milliseconds, which chain changes need not repeat.
    freeverb: Option<crate::audio::dsp::Response>,
    crystality_echo: Option<crate::audio::dsp::Response>,
    /// The second audio device's core (`crate::audio::second`).
    pub(crate) second: Option<crate::audio::second::Second>,
    /// mpv options the second core gets last: the command line's
    /// `--mpv-options` (the self-test's `ao=null`), or a test's.
    pub second_mpv_options: Vec<(String, String)>,
    /// The seek bar while the mouse holds it (`App::drag_seek_bar`).
    seek_bar: Option<SeekBarHold>,
    /// Counts the files loaded: the media controls' track ID (`media`).
    pub(crate) media_track: u64,
    /// M11's playback state (chapters, skips, shuffle).
    pub(crate) play: crate::playback::PlayState,
    /// Bookmarks, A-B lists, favorites and recent items.
    marks: crate::marks::MarkStore,
    /// The thumbnail core's requests.
    pub(crate) thumbs: crate::thumbs::Thumbs,
    /// The countdown before an ending and the shutdown timer.
    pub(crate) power: crate::power::Power,
    /// File moves and copies under way.
    pub(crate) file_jobs: crate::albums::FileJobs,
    /// A yt-dlp resolution under way.
    pub(crate) resolver: crate::ytdl::Resolver,
    /// The picture's state for this run (M12).
    pub(crate) picture: crate::picture::PictureState,
    /// The 3D input found and the 360° view (M12).
    pub(crate) stereo: crate::stereo::StereoState,
    /// Consecutive images and the thumbnail image under way (M12).
    pub(crate) capture: crate::capture::CaptureState,
    /// The stream and video recordings under way (M12).
    pub(crate) record: crate::record::RecordState,
    /// What audio files show (M12).
    pub(crate) visual: crate::visual::VisualState,
    /// The local date and time for file names (the window's clock; UTC
    /// without one).
    pub(crate) clock: Option<fn() -> String>,
    /// mpv's own downscaler and chroma scaler, read before RVP sets its
    /// (`App::apply_render`).
    pub(crate) render_defaults: Option<[String; 2]>,
    /// mpv draws into the Direct3D 11 renderer's window (set by the shell
    /// before the engine starts; `render.renderer` is the wish).
    pub(crate) native_video: bool,
    /// The Direct3D 11 video processor's entry last put in the chain: a
    /// new video or display size changes the chain only when it changes.
    pub(crate) processor_entry: std::cell::RefCell<Option<String>>,
    /// The GPU power budget's readings and steps (D17).
    pub(crate) gpu: crate::gpu_power::Budget,
    /// M18's motion smoothing.
    pub(crate) smoothing: crate::smoothing::Smoothing,
    /// libmpv's defaults of the subtitle options RVP leaves unset, and the
    /// subtitle options mpv has now (`App::apply_subtitles`).
    pub(crate) subtitle_defaults: crate::subtitles::MpvDefaults,
    pub(crate) applied_subtitle_options: std::collections::BTreeMap<&'static str, String>,
    /// The rate the playing file's subtitles were made for (Convert
    /// Subtitle FPS), until the next file.
    pub(crate) subtitle_fps: Option<f64>,
    /// The external subtitle files watched for changes.
    pub(crate) subtitle_files: Vec<crate::subtitles::WatchedFile>,
    /// The subtitle file the browser, the editor and Input Subtitles work
    /// on (`crate::subtitle_editor`).
    pub(crate) subtitle_editor: Option<crate::subtitle_editor::OpenSubtitle>,
    /// Subtitles made from the audio (`crate::speech`).
    pub(crate) speech: crate::speech::Speech,
    /// A Ctrl+Alt drag's sideways travel not yet turned into a step (M14).
    pub(crate) subtitle_drag_x: f64,
    /// What the loaded skin offers, and the random theme picked for this
    /// start (M14, `crate::skins`).
    pub(crate) skin_info: crate::skins::SkinInfo,
    pub(crate) random_theme: Option<usize>,
    /// "Change logo randomly on each loading": this file's pick.
    pub(crate) random_logo: Option<usize>,
    /// What paused playback for the window (M14's General page): bits of
    /// minimized, focus lost and screen locked.
    pub(crate) attention_holds: u8,
    /// M14's presets: the default's values while another preset runs, the
    /// preset to go back to after an auto-activated one, and whether the
    /// system runs on its battery (for the conditions).
    pub(crate) default_values: Option<serde_json::Value>,
    pub(crate) preset_before_auto: Option<String>,
    pub(crate) on_battery: Option<bool>,
    /// The skin the command line or `RVP_SKIN` chose (`Some(None)`: RVP
    /// Default) until the user picks one.
    pub(crate) skin_override: crate::skins::SkinOverride,
    /// The file that plays has no video (the audio skin mode).
    pub(crate) audio_only: bool,
}

impl App {
    /// A file is starting (before it plays): the start logo shows (M14).
    #[must_use]
    pub(crate) fn starting(&self) -> bool {
        self.tracking == Tracking::Loading
    }

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
        app.start_presets();
        Ok(app)
    }

    /// # Errors
    /// Fails when the resume store or the albums cannot be read.
    pub fn new(mut settings: Settings, state_dir: &Path) -> io::Result<Self> {
        let mut warnings = settings.validate();
        let (bindings, binding_warnings) = settings.bindings();
        warnings.extend(binding_warnings);
        let first_start = !state_dir.join("playlists.json").exists();
        let mut marks = crate::marks::MarkStore::open(state_dir)?;
        if !settings.playback.ab.remember {
            marks.forget_sections();
        }
        let mut session = Session::default();
        if settings.playback.finished.remember {
            session.after = settings.playback.finished.after;
        }
        let mut app = Self {
            settings,
            bindings,
            model: PlayerModel::new(),
            store: ResumeStore::open(state_dir)?,
            library: Library::open(state_dir)?,
            tracking: Tracking::On,
            end_handled: false,
            started: false,
            play_on_load: false,
            chain_pending: false,
            video_pending: std::cell::Cell::new(false),
            settings_path: None,
            writer: storage::Writer::new()?,
            first_start,
            warnings,
            session,
            state_dir: state_dir.to_path_buf(),
            svp: crate::svp::Watch::default(),
            normalizer: None,
            audio_source: crate::audio::chain::Source::default(),
            audio_devices: Vec::new(),
            applied_chain: None,
            freeverb: None,
            crystality_echo: None,
            second: None,
            second_mpv_options: Vec::new(),
            seek_bar: None,
            media_track: 0,
            play: crate::playback::PlayState::default(),
            marks,
            thumbs: crate::thumbs::Thumbs::default(),
            power: crate::power::Power::default(),
            file_jobs: crate::albums::FileJobs::default(),
            resolver: crate::ytdl::Resolver::default(),
            picture: crate::picture::PictureState::default(),
            stereo: crate::stereo::StereoState::default(),
            capture: crate::capture::CaptureState::default(),
            record: crate::record::RecordState::default(),
            visual: crate::visual::VisualState::default(),
            clock: None,
            render_defaults: None,
            native_video: false,
            processor_entry: std::cell::RefCell::new(None),
            gpu: crate::gpu_power::Budget::default(),
            smoothing: crate::smoothing::Smoothing::default(),
            subtitle_defaults: crate::subtitles::MpvDefaults::default(),
            applied_subtitle_options: std::collections::BTreeMap::new(),
            subtitle_fps: None,
            subtitle_files: Vec::new(),
            subtitle_editor: None,
            speech: crate::speech::Speech::default(),
            subtitle_drag_x: 0.0,
            skin_info: crate::skins::SkinInfo::default(),
            random_theme: None,
            random_logo: None,
            attention_holds: 0,
            default_values: None,
            preset_before_auto: None,
            on_battery: None,
            skin_override: crate::skins::SkinOverride::Off,
            audio_only: false,
        };
        app.start_picture();
        Ok(app)
    }

    /// Picture and playback state of this session (A-B points and more).
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Bookmarks, A-B lists, favorites and recent items.
    #[must_use]
    pub fn marks(&self) -> &crate::marks::MarkStore {
        &self.marks
    }

    pub fn marks_mut(&mut self) -> &mut crate::marks::MarkStore {
        &mut self.marks
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
            crate::volume::mpv_volume_max(self.settings.volume_max),
        ));
        options
            .extra
            .push(("audio-channels".to_owned(), self.audio_channels()));
        options.extra.extend(self.audio_options());
        options.extra.extend(
            self.settings
                .decoders
                .mpv_options()
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value)),
        );
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
            .extra
            .extend(crate::playback::engine_options(&settings.playback));
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
        if !self.settings.playlist.remember_default {
            self.library.forget_default();
            self.marks.clear_recent();
        }
        let state = self.store.state();
        engine.set_volume(state.volume.unwrap_or(100.0))?;
        engine.set_mute(state.mute)?;
        self.started = true;
        let last_file = state.last_file.clone();
        if let Some(volume) = self.settings.audio.startup_volume {
            engine.set_volume(crate::volume::mpv_volume(volume))?;
        }
        match crate::normalizer::Control::start(engine, self.settings.normalizer) {
            Ok(control) => self.normalizer = Some(control),
            Err(error) => self.warnings.push(format!("Normalizer: {error}")),
        }
        self.configure_normalizer();
        self.apply_audio_chain(engine);
        self.read_audio_devices(engine);
        self.second_device_changed(engine);
        if let Err(error) = self.apply_picture_settings(engine) {
            self.warnings.push(format!("Picture settings: {error}"));
        }
        self.apply_render(engine);
        self.probe_fruc(engine);
        self.start_subtitles(engine);
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
        // A folder brings its media files (and its subfolders').
        let files: Vec<String> = files
            .iter()
            .flat_map(|file| {
                let path = Path::new(file);
                if !crate::playlist::is_url(file) && path.is_dir() {
                    let mut found = Vec::new();
                    collect_media(path, 0, &mut found);
                    found.sort();
                    found
                } else {
                    vec![file.clone()]
                }
            })
            .collect();
        let (files, play) = if append {
            (files, 0)
        } else {
            self.expand_opened(&files)
        };
        if !append && !self.settings.playlist.append_on_open {
            self.library.clear(shown);
        }
        let known = self.library.shown().entries.len();
        let Some(first) = self.add_paths(shown, &files) else {
            // Every file was in the album already: the first of them plays.
            let path = files
                .get(play)
                .map(|path| crate::playlist::local_path(path));
            let existing = path.and_then(|path| {
                self.library
                    .shown()
                    .entries
                    .iter()
                    .position(|entry| entry.path == path)
            });
            return match existing {
                Some(index) if !append => self.play_entry(engine, shown, index, false),
                _ => Ok(()),
            };
        };
        if append {
            return Ok(());
        }
        // With duplicates left out, the file to play may be an older entry.
        let wanted = files
            .get(play)
            .map(|path| crate::playlist::local_path(path));
        let index = wanted
            .and_then(|path| {
                self.library
                    .shown()
                    .entries
                    .iter()
                    .enumerate()
                    .skip(if first >= known { first } else { 0 })
                    .find(|(_, entry)| entry.path == path)
                    .map(|(index, _)| index)
            })
            .unwrap_or(first);
        self.play_entry(engine, shown, index, false)
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
        if self.library.lists().playing != Some(album) {
            self.reset_shuffle();
        }
        self.library.set_playing(album, index);
        self.end_handled = false;
        self.play_on_load = !paused;
        self.tracking = Tracking::Loading;
        self.remember_recent(&path);
        engine.set_pause(paused)?;
        // Streaming sites go through yt-dlp first (crate::ytdl).
        if self.resolve_stream(engine, &path) {
            return Ok(());
        }
        let options = std::mem::take(&mut self.play.next_options);
        // The visualization's graph names this file's track, and a stream
        // recording ends with its file (M12).
        self.clear_visual(engine);
        drop(self.record_file_ended(engine));
        engine.load_with_options(&path, &options)
    }

    /// Moves to the next (`1`) or previous (`-1`) playable entry: the
    /// queue first, in shuffle order when shuffle is on, and on with the
    /// folder's files at the album's ends when that is on.
    fn step(&mut self, engine: &Engine, step: i32) -> Result<(), EngineError> {
        match self.next_entry(step, false) {
            Some((album, index)) => self.play_entry(engine, album, index, false),
            None => Ok(()),
        }
    }

    /// The entry Next (`1`), Previous (`-1`) or the end of a file (`at_end`)
    /// goes to.
    fn next_entry(&mut self, step: i32, at_end: bool) -> Option<(usize, usize)> {
        if step > 0
            && let Some(queued) = self.library.take_queued()
        {
            return Some(queued);
        }
        if self.settings.playback.shuffle {
            let pick = if step > 0 {
                self.shuffle_next()
            } else {
                self.shuffle_previous()
            };
            if pick.is_some() || step < 0 {
                return pick;
            }
            // Every entry played in this round: a new one with Repeat.
            if at_end && self.settings.repeat != Repeat::Album {
                return None;
            }
            self.reset_shuffle();
            return self.shuffle_next();
        }
        self.library
            .neighbour(step)
            .or_else(|| self.folder_neighbour(step))
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
            Action::TogglePreventScreenSaver => Some(self.settings.prevent_screen_saver),
            Action::SetRepeat(repeat) => Some(self.settings.repeat == repeat),
            Action::SetOnTop(mode) => Some(self.settings.on_top == mode),
            Action::SetAfterPlayback(after) => Some(self.session.after == after),
            Action::SetAspect(aspect) => Some(self.session.aspect == aspect),
            Action::VideoFit(fit) => Some(self.session.fit == fit),
            Action::Picture(command) => self.picture_checked(command),
            Action::Stereo(command) => self.stereo_checked(command),
            Action::SetImageFormat(format) => Some(self.settings.image_format == format),
            Action::Capture(command) => self.capture_checked(command),
            Action::Record(command) => self.recording_checked(command),
            Action::Visual(command) => self.visual_checked(command),
            Action::Render(toggle) => Some(self.render_checked(toggle)),
            Action::PlaybackInfo => Some(self.session.info),
            Action::ToggleEqualizer => Some(self.settings.equalizer.enabled),
            Action::Color(crate::video::ColorChange::Toggle) => Some(self.settings.color.enabled),
            Action::SetDeinterlace(mode) => Some(self.settings.deinterlace == mode),
            Action::VideoEffect(effect) => Some(self.settings.effects.contains(&effect)),
            Action::ToggleDeband => Some(self.settings.deband),
            Action::Rotate(crate::video::Rotation::To(degrees)) => {
                Some(self.session.rotation == degrees % 360)
            }
            Action::Shaders(command) => self.shader_checked(command),
            Action::ToggleVapourSynth => Some(self.settings.vapoursynth.enabled),
            Action::Smoothing(command) => Some(self.smoothing_checked(command)),
            Action::Subtitles(command) => self.subtitle_checked(command),
            Action::Speech(command) => self.speech_checked(command),
            Action::Misc(command) => self.misc_checked(command),
            Action::Skins(command) => self.skin_checked(command),
            Action::ControlPanel => Some(self.session.control_panel),
            Action::Audio(command) => self.audio_checked(command),
            other => self
                .playback_checked(other)
                .or_else(|| self.source_checked(other)),
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
            Action::Subtitles(crate::subtitles::SubtitleCommand::DelayBy(_, step)) => {
                let subtitles = &self.settings.subtitles;
                let seconds = if step == 3 {
                    subtitles.sync_step_3
                } else {
                    subtitles.sync_step_2
                };
                text.replace("%s", &sync_step_label(seconds))
            }
            Action::Subtitles(crate::subtitles::SubtitleCommand::Fps(_)) => {
                let video = self.model.double(PropertyId::ContainerFps).unwrap_or(0.0);
                text.replace("%g", &format!("{}", (video * 1000.0).round() / 1000.0))
            }
            Action::Skip(skip) => self.skip_label(skip, text),
            Action::PowerTimer(kind) => self.timer_label(kind, text),
            Action::AbRepeat(crate::action::AbRepeat::Behind) => text.replace(
                "5 Sec.",
                &format!("{} Sec.", self.settings.playback.ab.behind),
            ),
            Action::SetAspect(crate::action::Aspect::Custom) => {
                text.replace("%s", &self.settings.picture.custom_aspect)
            }
            Action::Picture(command) => self.picture_label(command, text),
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
                return Some(UiRequest::ToggleFullscreen { fit: None });
            }
            Action::ToggleFullscreenStretch => {
                return Some(UiRequest::ToggleFullscreen {
                    fit: Some(Fit::Stretch),
                });
            }
            Action::ToggleFullscreenCrop => {
                return Some(UiRequest::ToggleFullscreen {
                    fit: Some(Fit::Crop),
                });
            }
            Action::Minimize => return Some(UiRequest::Minimize),
            Action::MaximizeRestore => return Some(UiRequest::MaximizeRestore),
            Action::ContextMenu => return Some(UiRequest::ContextMenu),
            Action::TogglePlaylist => return Some(UiRequest::TogglePlaylist),
            Action::ImportPlaylists => return Some(UiRequest::ImportPlaylists),
            Action::ToggleSubtitles => engine.toggle_subtitles(),
            Action::ToggleNormalizer => {
                self.settings.normalizer.enabled = !self.settings.normalizer.enabled;
                self.configure_normalizer();
                self.audio_chain_changed(engine);
                return Some(self.normalizer_notice());
            }
            Action::ToggleBoost => {
                self.settings.normalizer.boost = !self.settings.normalizer.boost;
                // Only the gain control needs it: the chain stays.
                self.configure_normalizer();
                return Some(self.normalizer_notice());
            }
            Action::PlayPause => {
                self.forget_attention();
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
            Action::OpenDefault => return Some(self.default_open()),
            Action::SeekBackward(step) => {
                self.seek_to(engine, Seek::Relative(-self.settings.seek_seconds(step)))
            }
            Action::SeekForward(step) => {
                self.seek_to(engine, Seek::Relative(self.settings.seek_seconds(step)))
            }
            Action::VolumeUp => {
                let step = self.volume_step_now();
                return self.change_volume(engine, step);
            }
            Action::VolumeDown => {
                let step = self.volume_step_now();
                return self.change_volume(engine, -step);
            }
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
        if action.is_panel_command() {
            return self.album_command(engine, action, visible);
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
            // The cursor moved above, and M11's commands went to
            // App::album_command.
            _ => false,
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
        let added = engine.command("sub-add", &[&text, "select"]);
        if added.is_ok() {
            self.subtitle_tracks_chosen(engine);
            self.refresh_subtitle_files(engine);
        }
        match added {
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

    /// How PotPlayer's Windows paths become this system's: Windows opens
    /// them as they are, Linux through the drive map setting.
    pub(crate) fn drive_map(&self) -> DriveMap {
        if cfg!(windows) {
            DriveMap::default()
        } else {
            DriveMap::new(&self.settings.drive_map)
        }
    }

    /// Imports PotPlayer's playlists from `folder` (read-only) as albums,
    /// with their resume positions; returns how many albums came in.
    ///
    /// # Errors
    /// Fails when the folder cannot be read.
    pub fn import_playlists(&mut self, folder: &Path) -> io::Result<usize> {
        let imported = dpl::import_dir(folder, &self.drive_map())?;
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

    /// Sets RVP's audio filters (`crate::audio::chain`) for the settings
    /// and the file that plays, with the normalizer starting at the gain it
    /// reached; the user's own filters keep their places. Nothing happens
    /// when the chain is the one RVP set last.
    pub fn apply_audio_chain(&mut self, engine: &Engine) {
        let responses = self.audio_responses();
        let speakers = crate::speakers::choice(&self.settings.output_speakers)
            .unwrap_or(&crate::speakers::CHOICES[1]);
        let inputs = crate::audio::chain::Inputs {
            audio: &self.settings.audio,
            speakers,
            equalizer: &self.settings.equalizer,
            normalizer: &self.settings.normalizer,
            channel: self.session.stereo_channel,
            stretching: self.session.audio_stretching,
            source: &self.audio_source,
            gain: self
                .normalizer
                .as_ref()
                .map_or(1.0, crate::normalizer::Control::gain),
            responses: &responses,
        };
        let (front, back) = crate::audio::chain::build(&inputs);
        let current = engine.get_text("af").unwrap_or_default();
        let text = crate::audio::chain::assemble(&current, &front, &back);
        if let Some(second) = self.second.as_mut() {
            // The second device: the gain from the main core's normalizer,
            // and PCM (no AC3 encoding for pass-through).
            let back: Vec<crate::audio::chain::Entry> = back
                .iter()
                .filter(|(label, _)| *label != crate::audio::chain::AC3)
                .map(|(label, entry)| {
                    if *label == crate::normalizer::LABEL {
                        (
                            *label,
                            crate::normalizer::Normalizer::follower_entry(inputs.gain),
                        )
                    } else {
                        (*label, entry.clone())
                    }
                })
                .collect();
            second.set_chain(&crate::audio::chain::assemble(&current, &front, &back));
        }
        if self.applied_chain.as_deref() == Some(text.as_str()) {
            return;
        }
        match engine.set_text("af", &text) {
            Ok(()) => self.applied_chain = Some(text),
            Err(error) => eprintln!("rvp: cannot set the audio filters: {error}"),
        }
    }

    /// The impulse responses the chain needs now, computed into the state
    /// folder when missing (`crate::audio::dsp`).
    fn audio_responses(&mut self) -> crate::audio::chain::Responses {
        let folder = self.state_dir.join("audio");
        let audio = &self.settings.audio;
        let freeverb = if audio.freeverb.enabled {
            let (room, damping, width) = (
                audio.freeverb.room_size,
                audio.freeverb.damping,
                audio.freeverb.width,
            );
            let path = crate::audio::dsp::freeverb_path(&folder, room, damping, width);
            match &self.freeverb {
                Some(response) if response.path == path && path.is_file() => Some(response.clone()),
                _ => match crate::audio::dsp::freeverb_file(&folder, room, damping, width) {
                    Ok(response) => {
                        self.freeverb = Some(response.clone());
                        Some(response)
                    }
                    Err(error) => {
                        eprintln!("rvp: cannot write Freeverb's response: {error}");
                        None
                    }
                },
            }
        } else {
            None
        };
        let audio = &self.settings.audio;
        let crystality = if audio.crystality.enabled && audio.crystality.echo_volume > 0.0 {
            let (volume, feedback) = (audio.crystality.echo_volume, audio.crystality.echo_feedback);
            let path = crate::audio::dsp::crystality_echo_path(&folder, volume, feedback);
            match &self.crystality_echo {
                Some(response) if response.path == path && path.is_file() => Some(response.clone()),
                _ => match crate::audio::dsp::crystality_echo_file(&folder, volume, feedback) {
                    Ok(response) => {
                        self.crystality_echo = Some(response.clone());
                        Some(response)
                    }
                    Err(error) => {
                        eprintln!("rvp: cannot write Crystality's echo: {error}");
                        None
                    }
                },
            }
        } else {
            None
        };
        let audio = &self.settings.audio;
        let mode = crate::speakers::choice(&self.settings.output_speakers).map(|c| c.mode);
        let distance = match mode {
            Some(crate::speakers::Mode::VirtualHeadphone) => Some(0.0),
            Some(crate::speakers::Mode::VirtualSurround) => Some(audio.virtualizer.distance),
            _ => None,
        };
        let head_model = distance
            .map(|distance| crate::audio::dsp::head_model_file(&folder, distance))
            .and_then(|result| {
                result
                    .map_err(|error| eprintln!("rvp: cannot write the head model: {error}"))
                    .ok()
            });
        crate::audio::chain::Responses {
            freeverb,
            crystality,
            head_model,
        }
    }

    /// RVP's audio filters changed: they are set at once and playback goes
    /// on from here, or near the end of a file they wait for the next one.
    pub(crate) fn audio_chain_changed(&mut self, engine: &Engine) {
        if self.defer_chain_change() {
            return;
        }
        self.apply_audio_chain(engine);
        // Rebuilding the chain drops what mpv had filtered ahead.
        self.resync_audio(engine);
    }

    /// Gives the normalizer's settings to its gain control. Boost, level and
    /// release time need nothing else; switching it on or off changes the
    /// chain too (`App::audio_chain_changed`).
    pub(crate) fn configure_normalizer(&self) {
        if let Some(control) = &self.normalizer {
            control.configure(self.settings.normalizer);
        }
    }

    /// True in the last [`NEAR_END`] seconds of the loaded file or at its
    /// end.
    pub(crate) fn near_end(&self) -> bool {
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
        self.apply_audio_chain(engine);
        if resync {
            self.resync_audio(engine);
        }
    }

    /// Plays on from the current position: the audio chain changed and
    /// the audio mpv had filtered ahead with the old one would put audio and
    /// video out of step. Never near the end (`App::defer_chain_change`).
    pub(crate) fn resync_audio(&self, engine: &Engine) {
        if self.model.media_loaded() && !self.near_end() {
            drop(engine.seek(Seek::Relative(0.0)));
        }
    }

    /// Saves a normalizer change made from a key or menu and tells it.
    fn normalizer_notice(&self) -> UiRequest {
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
        // With a preset active, its values go into it (M14).
        match serde_json::to_vec_pretty(&self.stored_settings()) {
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
        let defaults = Bindings::with_options(&self.settings.input);
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
        let target = self.keep_after_a(target);
        engine.seek_with(target, !self.settings.keyframe_seeks)
    }

    /// The volume keys and the wheel: RVP's volume, or the system's wave or
    /// master volume (the Volume Control page's method).
    pub(crate) fn change_volume(&mut self, engine: &Engine, delta: f64) -> Option<UiRequest> {
        match self.settings.audio.volume_method {
            crate::audio::VolumeMethod::Player => {
                self.set_volume(engine, self.volume_percent() + delta)
            }
            crate::audio::VolumeMethod::Wave => crate::audio::commands::change_wave(engine, delta),
            crate::audio::VolumeMethod::Master => Some(UiRequest::SystemVolume(
                crate::audio::commands::SystemVolume::MasterStep(delta),
            )),
        }
    }

    /// The volume as RVP shows it: PotPlayer's percent
    /// ([`crate::volume`]).
    #[must_use]
    pub fn volume_percent(&self) -> f64 {
        crate::volume::percent(self.model.double(PropertyId::Volume).unwrap_or(100.0))
    }

    /// Sets the volume (PotPlayer's percent), within 0 to the highest
    /// volume, and shows it on the OSD as the keys, the wheel and the
    /// sliders do.
    fn set_volume(&mut self, engine: &Engine, percent: f64) -> Option<UiRequest> {
        let percent = percent.clamp(0.0, self.settings.volume_max.max(100.0));
        engine
            .set_volume(crate::volume::mpv_volume(percent))
            .ok()
            .map(|()| UiRequest::Osd(format!("Volume: {percent:.0} %")))
    }

    /// Seeks to `fraction` (0.0 to 1.0) of the file.
    pub fn seek_to_fraction(&mut self, engine: &Engine, fraction: f64) {
        drop(self.seek_to(engine, Seek::Fraction(fraction)));
    }

    /// The mouse pressed or dragged the seek bar to `fraction`: the bar
    /// shows there at once ([`App::seek_bar_data`]) and the video jumps to
    /// the nearest keyframe meanwhile, which is quick.
    pub fn drag_seek_bar(&mut self, engine: &Engine, fraction: f64) {
        if !self.model.media_loaded() {
            return;
        }
        // "Pause when seeking with mouse": from the press to the release.
        if self.seek_bar.is_none_or(|hold| hold.released.is_some())
            && self.settings.playback.pause_while_seeking
            && self.model.flag(PropertyId::Pause) == Some(false)
            && engine.set_pause(true).is_ok()
        {
            self.play.paused_for_seek = true;
        }
        self.seek_bar = Some(SeekBarHold {
            fraction,
            released: None,
        });
        if matches!(self.tracking, Tracking::Resuming(_)) {
            self.tracking = Tracking::On;
        }
        drop(engine.seek_with(Seek::Fraction(fraction), false));
    }

    /// The mouse let the seek bar go at `fraction`: the last seek (exact
    /// unless keyframe seeks are on); the bar stays there until the video
    /// gets there.
    pub fn release_seek_bar(&mut self, engine: &Engine, fraction: f64, now: Instant) {
        if !self.model.media_loaded() {
            return;
        }
        self.seek_bar = Some(SeekBarHold {
            fraction,
            released: Some(now),
        });
        drop(self.seek_to(engine, Seek::Fraction(fraction)));
        if std::mem::take(&mut self.play.paused_for_seek) {
            drop(engine.set_pause(false));
        }
    }

    /// Shows a held seek bar, and its time, in the skin's data.
    pub fn seek_bar_data(&self, data: &mut rvp_skin::DataModel) {
        let Some(hold) = self.seek_bar else {
            return;
        };
        let Some(duration) = self.model.double(PropertyId::Duration).filter(|d| *d > 0.0) else {
            return;
        };
        let time = hold.fraction * duration;
        data.set_text("CMD_CURRENTTIME_MESSAGE", crate::view::format_time(time));
        data.set_text(
            "CMD_LEFTTIME_MESSAGE",
            crate::view::format_time(duration - time),
        );
        if let Some(slider) = data.sliders.get_mut("CMD_TIMEBAR_CTRL") {
            // f32 is far finer than a pixel of the bar.
            #[allow(clippy::cast_possible_truncation)]
            let value = hold.fraction.clamp(0.0, 1.0) as f32;
            slider.value = value;
        }
    }

    /// Lets a released seek bar show the playback position again once the
    /// video got near the place it was let go, or after [`SEEK_BAR_HOLD`].
    fn settle_seek_bar(&mut self, now: Instant) {
        let Some(SeekBarHold {
            fraction,
            released: Some(released),
        }) = self.seek_bar
        else {
            return;
        };
        let arrived = match (
            self.model.double(PropertyId::TimePos),
            self.model.double(PropertyId::Duration),
        ) {
            (Some(time), Some(duration)) => (time - fraction * duration).abs() < 1.0,
            _ => false,
        };
        if arrived || now.saturating_duration_since(released) > SEEK_BAR_HOLD {
            self.seek_bar = None;
        }
    }

    /// A volume slider was set to `fraction` (0.0 to 1.0) of its length,
    /// which runs to the highest volume.
    pub fn set_volume_fraction(&mut self, engine: &Engine, fraction: f64) -> Option<UiRequest> {
        match self.settings.audio.volume_method {
            crate::audio::VolumeMethod::Player => {
                self.set_volume(engine, fraction * self.settings.volume_max.max(100.0))
            }
            crate::audio::VolumeMethod::Wave => {
                let volume = (fraction * 100.0).clamp(0.0, 100.0);
                engine
                    .set_double("ao-volume", volume)
                    .ok()
                    .map(|()| UiRequest::Osd(format!("Wave volume: {volume:.0} %")))
            }
            crate::audio::VolumeMethod::Master => Some(UiRequest::SystemVolume(
                crate::audio::commands::SystemVolume::MasterSet(fraction.clamp(0.0, 1.0)),
            )),
        }
    }

    /// The engine's events of one drain: what they change, whether the
    /// store is to be saved now, and whether a file failed to open.
    fn handle_events(
        &mut self,
        engine: &Engine,
        events: &[EngineEvent],
        now: Instant,
        requests: &mut Vec<UiRequest>,
    ) -> (bool, bool) {
        let mut save_now = false;
        let mut failed = false;
        for event in events {
            match event {
                EngineEvent::StartFile => {
                    self.tracking = Tracking::Loading;
                    self.next_random_logo();
                    self.seek_bar = None;
                    // A-B points, chapters and skips belong to one file.
                    self.session.point_a = None;
                    self.session.point_b = None;
                    self.session.ab_active = false;
                    self.play.chapters.clear();
                    self.play.skipped_chapter = None;
                    self.play.ending_skipped = false;
                    self.play.subtitle_section = false;
                }
                EngineEvent::FileLoaded { path } => {
                    self.media_track += 1;
                    requests.extend(self.close_subtitle_editor());
                    self.file_loaded(engine, path, now);
                    requests.extend(self.speech_file_loaded(engine));
                    requests.extend(self.auto_preset(engine));
                    save_now = true;
                }
                EngineEvent::EndFile {
                    reason: EndReason::Eof,
                    path: Some(path),
                } => {
                    self.store.clear_position(path);
                    requests.extend(self.preset_playback_end(engine));
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
        (save_now, failed)
    }

    /// Handles everything libmpv has reported: updates the model, the
    /// albums and the resume store, moves on at the end of a file, and
    /// returns the changes, requests for the window and the events.
    pub fn drain(&mut self, engine: &Engine, now: Instant) -> Drained {
        let mut events = Vec::new();
        let changes = engine.drain(&mut self.model, &mut events);
        let mut requests = Vec::new();
        let (mut save_now, failed) = self.handle_events(engine, &events, now, &mut requests);
        if failed {
            // `<Can't open>`, then the next playable entry (or stop).
            self.library.update_current(None, false);
            if self.settings.playback.on_failure == crate::playback::OnFailure::Next {
                drop(self.step(engine, 1));
            }
        }
        if changes.contains(PropertyId::Chapters) {
            self.read_chapters(engine);
        }
        self.picture_changed(engine, changes);
        if changes.any(&[
            PropertyId::VideoHeight,
            PropertyId::ContainerFps,
            PropertyId::VideoFilters,
        ]) && self.model.int(PropertyId::VideoHeight).is_some()
        {
            self.apply_smoothing(engine);
        }
        // The file's stereo metadata comes with its first frame.
        if changes.contains(PropertyId::VideoWidth) && self.model.media_loaded() {
            self.stereo_file_loaded(engine);
        }
        if changes.any(&[PropertyId::SubStart, PropertyId::SubEnd]) {
            requests.extend(self.subtitle_section_arrived(engine));
        }
        if changes.contains(PropertyId::Chapter) && self.chapter_skip(engine) {
            // The last chapter was one to skip: the file is done.
            self.end_handled = true;
            self.end_of_file(engine, now, &mut requests);
        }
        if changes.contains(PropertyId::Duration)
            && let Some(duration) = self.model.double(PropertyId::Duration).filter(|d| *d > 0.0)
        {
            self.library.update_current(Some(duration), true);
        }

        self.follow_second(engine, &events, now);
        self.track_position(engine, now, &mut requests);
        if changes.contains(PropertyId::TimePos) {
            self.capture_tick(&mut requests);
            if let Some(time) = self.model.double(PropertyId::TimePos) {
                self.visual_tick(engine, time);
            }
        }
        self.settle_seek_bar(now);
        if let Some(top) = self.on_top_change() {
            requests.push(UiRequest::OnTop(top));
        }
        requests.extend(self.watch_svp(now));
        requests.extend(self.speech_tick(engine, now));
        // RVP's smoothing starts once SVP had its chance.
        let hold = self.svp.holds_smoothing();
        if hold != self.smoothing.svp_hold {
            self.smoothing.svp_hold = hold;
            self.apply_smoothing(engine);
        }
        if self.started
            && changes.contains(PropertyId::Volume)
            && let Some(volume) = self.model.double(PropertyId::Volume)
        {
            self.store.set_volume(volume);
        }
        if self.started && changes.contains(PropertyId::Mute) {
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
            .or_else(|| self.marks.take_error())
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
        // First: the smoothing waits for SVP from here on, and the last
        // file's dropped frames count no longer.
        self.svp
            .file_started(crate::svp::svp_mode(&self.settings), now);
        self.smoothing.file_started();
        self.subtitle_file_loaded(engine);
        self.audio_file_loaded(engine);
        self.picture_file_loaded(engine);
        self.stereo_file_loaded(engine);
        self.capture_file_loaded();
        self.apply_visual(engine);
        self.apply_pending_chain(engine, false);
        if self.video_pending.get() {
            drop(self.apply_video_filters(engine));
        }
        self.apply_ab(engine);
        self.store.set_last_file(path);
        self.read_chapters(engine);
        self.apply_loop_count(engine);
        if !self.settings.playback.keep_speed {
            drop(engine.set_double("speed", 1.0));
        }
        drop(engine.set_text("play-direction", "forward"));
        if self.settings.playback.retitle
            && let Some(title) = engine.get_text("metadata/by-key/title")
            && !title.trim().is_empty()
            && let Some((album, index)) = self.playing_position()
        {
            self.library.set_title(album, &[index], Some(&title));
        }
        let tracks = engine.tracks();
        let audio_only = self.model.int(PropertyId::VideoWidth).is_none()
            && !tracks
                .iter()
                .any(|track| track.kind == rvp_engine::TrackKind::Video);
        // The skin's audio mode: a cover picture is no video.
        self.audio_only = !tracks
            .iter()
            .any(|track| track.kind == rvp_engine::TrackKind::Video && !track.album_art);
        let remember = if audio_only {
            self.settings.remember_positions && self.settings.playback.remember_audio_positions
        } else {
            self.settings.remember_positions
        };
        // The command line's /seek sets the start itself.
        let given = std::mem::take(&mut self.play.start_given);
        let resume = self.store.position(path).filter(|_| remember && !given);
        let start = if given {
            None
        } else {
            self.intro_skip(resume, engine.get_double("duration"))
                .or(resume)
        };
        self.tracking = match start {
            Some(position) if engine.seek(Seek::Absolute(position)).is_ok() => {
                Tracking::Resuming(position)
            }
            _ => Tracking::On,
        };
    }

    /// The album and index of the entry that plays.
    fn playing_position(&self) -> Option<(usize, usize)> {
        let album = self.library.lists().playing?;
        let (index, _) = self.library.playing()?;
        Some((album, index))
    }

    /// Records the position of the playing file and moves on at its end
    /// (or where "Skip Ending" ends it).
    fn track_position(&mut self, engine: &Engine, now: Instant, requests: &mut Vec<UiRequest>) {
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
        if self.video_pending.get() && !self.near_end() {
            drop(self.apply_video_filters(engine));
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
        if !at_end
            && self.tracking == Tracking::On
            && self.model.media_loaded()
            && self.ending_reached()
        {
            self.end_handled = true;
            self.end_of_file(engine, now, requests);
            return;
        }
        if at_end
            && !self.end_handled
            && self.model.media_loaded()
            && self.tracking != Tracking::Loading
        {
            // keep-open stops on the last frame; the album moves on.
            self.end_handled = true;
            self.end_of_file(engine, now, requests);
        }
    }

    /// The file played to its end: repeat it, move on, start the album
    /// again, exit or power off, as the repeat mode and "When Playback
    /// Finished" say; a played file may go to the Recycle Bin.
    fn end_of_file(&mut self, engine: &Engine, now: Instant, requests: &mut Vec<UiRequest>) {
        let finished = self.model.loaded_path().map(str::to_owned);
        if self.settings.playback.recycle_played
            && let Some(path) = &finished
            && let Some(request) = Self::recycle_request(std::slice::from_ref(path), true)
        {
            requests.push(request);
        }
        match self.session.after {
            AfterPlayback::ExitAfterFile => {
                requests.push(UiRequest::Exit);
                return;
            }
            AfterPlayback::PowerAfterFile => {
                let action = self.settings.playback.finished.action;
                requests.push(self.start_countdown(crate::power::Ending::Power(action), now));
                return;
            }
            // keep-open holds the last frame; the album waits.
            AfterPlayback::Stay => return,
            _ => {}
        }
        if self.settings.repeat == Repeat::Item {
            drop(
                engine
                    .seek(Seek::Absolute(0.0))
                    .and_then(|()| engine.set_pause(false)),
            );
            self.end_handled = false;
            return;
        }
        let next = self.next_entry(1, true).or_else(|| {
            (self.settings.repeat == Repeat::Album && !self.settings.playback.shuffle)
                .then(|| self.library.first_playable())
                .flatten()
        });
        if let Some((album, index)) = next {
            drop(self.play_entry(engine, album, index, false));
            return;
        }
        match self.session.after {
            AfterPlayback::ExitAfterList => requests.push(UiRequest::Exit),
            AfterPlayback::PowerAfterList => {
                let action = self.settings.playback.finished.action;
                requests.push(self.start_countdown(crate::power::Ending::Power(action), now));
            }
            _ if self.settings.playback.close_at_end => drop(engine.stop()),
            _ => {}
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
        // Subtitle edits not saved stay in RVP's folder.
        drop(self.close_subtitle_editor());
        self.stop_speech(true, true);
        self.stop_second();
        drop(engine.stop());
        self.writer.flush();
        self.library.flush()?;
        self.marks.flush()?;
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
