//! What the user can ask for, and how PotPlayer's command IDs map to it.
//!
//! Two kinds of PotPlayer IDs reach RVP: the symbolic `CmdID` of skin
//! controls and skin menus (`CMD_MEDIA_PLAY`, `ID_PLAYBACK_5SEC_BACK`) and
//! the numeric IDs of menu items and default hotkeys (`10014` = Play/Pause,
//! `docs/r1`). Both map to [`Action`] through one table ([`ACTIONS`]): the
//! action's settings name, PotPlayer's menu text and the shortcut its menu
//! shows, the numeric IDs and the skin names. What is not mapped is listed
//! with a reason in `docs/coverage/menus.md` (`tests/coverage.rs` keeps it
//! current).

use serde::{Deserialize, Serialize};

use crate::audio::commands::AudioCommand;
use crate::audio::{StereoChannel, Stretch, SyncKeep, VolumeMethod};
use crate::video::{ColorChange, ColorKind, Deinterlace, Rotation, ShaderChange, VideoEffect};

/// One of PotPlayer's four seek steps (Left/Right, with Ctrl, with Shift,
/// with Ctrl+Alt). Their lengths are settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeekStep {
    Short,
    Medium,
    Long,
    VeryLong,
}

/// Where "Jump (to)" goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Jump {
    Start,
    Middle,
    /// 30 seconds before the end.
    BeforeEnd,
    PreviousSubtitle,
    NextSubtitle,
    PreviousChapter,
    NextChapter,
}

/// Playback speed changes (PotPlayer's steps of 0.1x).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Speed {
    /// 1.0x, or back to the last other speed.
    Default,
    Slower,
    Faster,
}

/// A-B section repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbRepeat {
    SetA,
    SetB,
    ClearA,
    ClearB,
    /// Start or stop repeating the section, keeping its points.
    Toggle,
    /// Both points cleared.
    Clear,
}

/// What happens at the end of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Repeat {
    /// On to the next entry, stop after the last.
    #[default]
    Off,
    /// After the last entry, the first again.
    Album,
    /// The same file again.
    Item,
}

/// Audio or subtitle sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Delay {
    Earlier,
    Later,
    Reset,
}

/// Subtitle size, position and alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubtitleStyle {
    Bigger,
    Smaller,
    Up,
    Down,
    DefaultPosition,
    AlignLeft,
    AlignCenter,
    AlignRight,
}

/// The video's aspect ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Aspect {
    /// As the container says (PotPlayer's "Default Display AR").
    #[default]
    Default,
    /// As the video stream says ("Default Original DAR").
    Original,
    R4x3,
    R16x9,
    R16x10,
    R185x1,
    R235x1,
}

/// How the video fills the video area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Fit {
    /// Inside the area, keeping the aspect ratio.
    #[default]
    KeepAspect,
    /// Filling the area, keeping the aspect ratio (the edges are cut).
    Crop,
    /// Filling the area, distorted.
    Stretch,
}

/// Zooming and moving the picture ("Frame Size").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Zoom {
    In,
    Out,
    Wider,
    Narrower,
    Taller,
    Shorter,
    Left,
    Right,
    Up,
    Down,
    ResetSize,
    ResetPosition,
    Reset,
}

/// "Stay on Top".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnTop {
    #[default]
    Never,
    Always,
    WhilePlaying,
}

/// The window's size against the video's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowScale {
    Half,
    Normal,
    OneAndHalf,
    Double,
}

impl WindowScale {
    #[must_use]
    pub fn factor(self) -> f64 {
        match self {
            WindowScale::Half => 0.5,
            WindowScale::Normal => 1.0,
            WindowScale::OneAndHalf => 1.5,
            WindowScale::Double => 2.0,
        }
    }
}

/// "When Playback Finished" (for this session, as in PotPlayer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum AfterPlayback {
    #[default]
    Nothing,
    ExitAfterFile,
    ExitAfterList,
}

/// The format of saved frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageFormat {
    #[default]
    Png,
    Jpeg,
}

/// A page of the settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettingsPage {
    Playback,
    Playlist,
    Audio,
    Video,
    Svp,
    Subtitles,
    Input,
    Skins,
    Shaders,
    AudioVolume,
    Equalizer,
    Normalizer,
    AudioEffects,
    AudioSync,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 14] = [
        SettingsPage::Playback,
        SettingsPage::Playlist,
        SettingsPage::Audio,
        SettingsPage::AudioVolume,
        SettingsPage::Equalizer,
        SettingsPage::Normalizer,
        SettingsPage::AudioEffects,
        SettingsPage::AudioSync,
        SettingsPage::Video,
        SettingsPage::Svp,
        SettingsPage::Subtitles,
        SettingsPage::Input,
        SettingsPage::Skins,
        SettingsPage::Shaders,
    ];

    /// The page's key in the settings window (`shell/qml/Settings.qml`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            SettingsPage::Playback => "playback",
            SettingsPage::Playlist => "playlist",
            SettingsPage::Audio => "audio",
            SettingsPage::Video => "video",
            SettingsPage::Svp => "svp",
            SettingsPage::Subtitles => "subtitles",
            SettingsPage::Input => "input",
            SettingsPage::Skins => "skins",
            SettingsPage::Shaders => "shaders",
            SettingsPage::AudioVolume => "audio-volume",
            SettingsPage::Equalizer => "equalizer",
            SettingsPage::Normalizer => "normalizer",
            SettingsPage::AudioEffects => "audio-effects",
            SettingsPage::AudioSync => "audio-sync",
        }
    }
}

/// A user action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Open files through the file dialog.
    OpenFiles,
    OpenFolder,
    OpenUrl,
    /// Load a subtitle file for the playing video.
    OpenSubtitle,
    /// Choose the subtitle font (a font dialog).
    SubtitleFont,
    /// Open the playing (or last) entry again.
    Reopen,
    /// Stop and unload the current file (PotPlayer "Close Playback").
    CloseMedia,
    /// Quit RVP.
    Exit,
    PlayPause,
    Previous,
    Next,
    SeekBackward(SeekStep),
    SeekForward(SeekStep),
    FrameBack,
    FrameForward,
    JumpTo(Jump),
    Speed(Speed),
    AbRepeat(AbRepeat),
    RepeatCycle,
    SetRepeat(Repeat),
    /// Seek to keyframes (fast) instead of exactly.
    ToggleKeyframeSeeks,
    /// Keep the screen saver off while video plays.
    TogglePreventScreenSaver,
    AfterPlaybackCycle,
    SetAfterPlayback(AfterPlayback),
    VolumeUp,
    VolumeDown,
    ToggleMute,
    /// The audio normalizer on or off (PotPlayer: Shift+N).
    ToggleNormalizer,
    /// The normalizer's "Boost audio" on or off.
    ToggleBoost,
    CycleAudio,
    AudioDelay(Delay),
    /// Show or hide subtitles.
    ToggleSubtitles,
    CycleSubtitles,
    SubtitleDelay(Delay),
    Subtitle(SubtitleStyle),
    CycleVideo,
    AspectCycle,
    SetAspect(Aspect),
    VideoFit(Fit),
    Zoom(Zoom),
    /// Save the current video frame (without subtitles).
    SaveFrame,
    /// Save the current frame as shown (with subtitles).
    SaveScreen,
    SetImageFormat(ImageFormat),
    OpenImageFolder,
    SelectImageFolder,
    /// Fullscreen keeping the aspect ratio, or back to the window.
    ToggleFullscreen,
    /// Fullscreen with the video stretched to the screen.
    ToggleFullscreenStretch,
    Minimize,
    MaximizeRestore,
    OnTopCycle,
    SetOnTop(OnTop),
    WindowScale(WindowScale),
    /// The window takes the video's shape.
    WindowFitAspect,
    ContextMenu,
    /// Show or hide the attached playlist panel.
    TogglePlaylist,
    /// Playback information over the video (Tab).
    PlaybackInfo,
    /// The settings window (F5, PotPlayer's Preferences).
    Preferences,
    /// The settings window's key and mouse page.
    KeysAndMouse,
    /// The settings window at a page.
    Settings(SettingsPage),
    /// Import PotPlayer's playlists as albums.
    ImportPlaylists,
    About,
    /// The control panel window (F7, PotPlayer's control box).
    ControlPanel,
    ToggleEqualizer,
    Color(ColorChange),
    DeinterlaceCycle,
    SetDeinterlace(Deinterlace),
    /// RVP's own video effect filters on or off.
    VideoEffect(VideoEffect),
    ToggleDeband,
    Rotate(Rotation),
    Shaders(ShaderChange),
    /// The active VapourSynth script slot on or off.
    ToggleVapourSynth,
    /// Save the current frame under a chosen name.
    SaveFrameAs,
    /// The Audio menu's processing commands (M10).
    Audio(AudioCommand),
}

/// One row of [`ACTIONS`].
#[derive(Debug, Clone, Copy)]
pub struct ActionInfo {
    pub action: Action,
    /// Stable name for settings files.
    pub name: &'static str,
    /// PotPlayer's menu text; `%s` stands for a step's length.
    pub label: &'static str,
    /// The shortcut PotPlayer's menu shows.
    pub shortcut: Option<&'static str>,
    /// PotPlayer's numeric command IDs.
    pub ids: &'static [u32],
    /// Skin `CmdID`s.
    pub skin: &'static [&'static str],
}

const fn info(
    action: Action,
    name: &'static str,
    label: &'static str,
    shortcut: Option<&'static str>,
    ids: &'static [u32],
    skin: &'static [&'static str],
) -> ActionInfo {
    ActionInfo {
        action,
        name,
        label,
        shortcut,
        ids,
        skin,
    }
}

/// Every action with its names and PotPlayer's IDs, in the order of
/// PotPlayer's menus.
pub const ACTIONS: &[ActionInfo] = &[
    // Open.
    info(
        Action::OpenFiles,
        "open-files",
        "Open File(s)...",
        Some("F3"),
        &[10443, 10158],
        &[
            "CMD_MEDIA_OPEN",
            "ID_FILE_OPEN",
            "ID_OPEN_FILE",
            "ID_BASE_OPEN",
        ],
    ),
    info(
        Action::OpenFolder,
        "open-folder",
        "Open Folder...",
        Some("F2"),
        &[10160],
        &["ID_OPEN_FOLDER"],
    ),
    info(
        Action::OpenUrl,
        "open-url",
        "Open URL...",
        Some("Ctrl+U"),
        &[10159],
        &["ID_OPEN_URL"],
    ),
    info(
        Action::OpenSubtitle,
        "open-subtitle",
        "Load Subtitle...",
        Some("Alt+O"),
        &[10146, 10147],
        &["ID_CAPTION_OPEN"],
    ),
    info(
        Action::Reopen,
        "reopen",
        "Reopen Current/Last Item",
        Some("Ctrl+Y"),
        &[10385],
        &[],
    ),
    info(
        Action::CloseMedia,
        "close-media",
        "Close Playback",
        Some("F4"),
        &[10167],
        &["CMD_MEDIA_STOP", "CMD_MEDIA_CLOSE", "ID_CLOSE_MEDIA"],
    ),
    info(
        Action::Exit,
        "exit",
        "Exit",
        Some("Alt+F4"),
        &[57665],
        &["ID_APP_EXIT"],
    ),
    // Playback.
    info(
        Action::PlayPause,
        "play-pause",
        "Play/Pause Playback",
        Some("Space"),
        &[10014],
        &["CMD_MEDIA_PLAY", "CMD_MEDIA_PAUSE"],
    ),
    info(
        Action::Previous,
        "previous",
        "Play Previous Item",
        Some("PgUp"),
        &[10123, 10067],
        &["ID_PROCESS_PREV", "CMD_MEDIA_PREV"],
    ),
    info(
        Action::Next,
        "next",
        "Play Next Item",
        Some("PgDn"),
        &[10124, 10068],
        &["ID_PROCESS_NEXT", "CMD_MEDIA_NEXT"],
    ),
    info(
        Action::FrameBack,
        "frame-back",
        "Previous frame",
        Some("D"),
        &[10242],
        &[],
    ),
    info(
        Action::FrameForward,
        "frame-forward",
        "Next frame",
        Some("F"),
        &[10241],
        &[],
    ),
    info(
        Action::SeekBackward(SeekStep::Short),
        "seek-backward-short",
        "%s Backward",
        Some("Left"),
        &[10059],
        &["ID_PLAYBACK_5SEC_BACK"],
    ),
    info(
        Action::SeekForward(SeekStep::Short),
        "seek-forward-short",
        "%s Forward",
        Some("Right"),
        &[10060],
        &["ID_PLAYBACK_5SEC_FORW"],
    ),
    info(
        Action::SeekBackward(SeekStep::Medium),
        "seek-backward-medium",
        "%s Backward",
        Some("Ctrl+Left"),
        &[10061],
        &["ID_PLAYBACK_30SEC_BACK"],
    ),
    info(
        Action::SeekForward(SeekStep::Medium),
        "seek-forward-medium",
        "%s Forward",
        Some("Ctrl+Right"),
        &[10062],
        &["ID_PLAYBACK_30SEC_FORW"],
    ),
    info(
        Action::SeekBackward(SeekStep::Long),
        "seek-backward-long",
        "%s Backward",
        Some("Shift+Left"),
        &[10063],
        &["ID_PLAYBACK_1MIN_BACK"],
    ),
    info(
        Action::SeekForward(SeekStep::Long),
        "seek-forward-long",
        "%s Forward",
        Some("Shift+Right"),
        &[10064],
        &["ID_PLAYBACK_1MIN_FORW"],
    ),
    info(
        Action::SeekBackward(SeekStep::VeryLong),
        "seek-backward-very-long",
        "%s Backward",
        Some("Ctrl+Alt+Left"),
        &[10065],
        &["ID_PLAYBACK_5MIN_BACK"],
    ),
    info(
        Action::SeekForward(SeekStep::VeryLong),
        "seek-forward-very-long",
        "%s Forward",
        Some("Ctrl+Alt+Right"),
        &[10066],
        &["ID_PLAYBACK_5MIN_FORW"],
    ),
    info(
        Action::JumpTo(Jump::Start),
        "jump-start",
        "Starting point",
        Some("BackSpace"),
        &[10243],
        &["ID_PLAYBACK_FIRST"],
    ),
    info(
        Action::JumpTo(Jump::Middle),
        "jump-middle",
        "Middle point",
        Some("Ctrl+BackSpace"),
        &[10619],
        &[],
    ),
    info(
        Action::JumpTo(Jump::BeforeEnd),
        "jump-before-end",
        "30 Sec. before ending",
        Some("Shift+BackSpace"),
        &[10620],
        &[],
    ),
    info(
        Action::JumpTo(Jump::PreviousSubtitle),
        "jump-previous-subtitle",
        "Previous subtitle position",
        Some("Home"),
        &[10244],
        &["ID_PLAYBACK_PREV_CAP"],
    ),
    info(
        Action::JumpTo(Jump::NextSubtitle),
        "jump-next-subtitle",
        "Next subtitle position",
        Some("End"),
        &[10245],
        &["ID_PLAYBACK_NEXT_CAP"],
    ),
    info(
        Action::JumpTo(Jump::PreviousChapter),
        "previous-chapter",
        "Previous Chapter",
        None,
        &[10107],
        &[],
    ),
    info(
        Action::JumpTo(Jump::NextChapter),
        "next-chapter",
        "Next Chapter",
        None,
        &[10108],
        &[],
    ),
    info(
        Action::Speed(Speed::Default),
        "speed-default",
        "Default/Last used Playback Speed",
        Some("Z"),
        &[10246],
        &["ID_PLAY_SPEED_NORMAL"],
    ),
    info(
        Action::Speed(Speed::Slower),
        "speed-down",
        "Decrease Playback Speed",
        Some("X"),
        &[10247],
        &["ID_PLAY_SPEED_SLOWER"],
    ),
    info(
        Action::Speed(Speed::Faster),
        "speed-up",
        "Increase Playback Speed",
        Some("C"),
        &[10248],
        &["ID_PLAY_SPEED_FASTER"],
    ),
    info(
        Action::AbRepeat(AbRepeat::SetA),
        "ab-set-a",
        "Set Starting Point (A)",
        Some("["),
        &[10249],
        &["ID_PLAY_REPEAT_START_SET"],
    ),
    info(
        Action::AbRepeat(AbRepeat::SetB),
        "ab-set-b",
        "Set Ending Point (B)",
        Some("]"),
        &[10250],
        &["ID_PLAY_REPEAT_STOP_SET"],
    ),
    info(
        Action::AbRepeat(AbRepeat::ClearA),
        "ab-clear-a",
        "Reset Point A",
        Some("Shift+["),
        &[10251],
        &[],
    ),
    info(
        Action::AbRepeat(AbRepeat::ClearB),
        "ab-clear-b",
        "Reset Point B",
        Some("Shift+]"),
        &[10252],
        &[],
    ),
    info(
        Action::AbRepeat(AbRepeat::Toggle),
        "ab-toggle",
        "Start/Stop A-B Section Repeat",
        Some("\\"),
        &[10253],
        &[],
    ),
    info(
        Action::AbRepeat(AbRepeat::Clear),
        "ab-clear",
        "Reset A-B Section Repeat",
        None,
        &[],
        &["ID_PLAY_REPEAT_RESET"],
    ),
    info(
        Action::RepeatCycle,
        "repeat-cycle",
        "Cycle Repeat Playback",
        None,
        &[10034],
        &[],
    ),
    info(
        Action::SetRepeat(Repeat::Off),
        "repeat-off",
        "Repeat Playback: Disable",
        None,
        &[10495],
        &[],
    ),
    info(
        Action::SetRepeat(Repeat::Album),
        "repeat-album",
        "Enable: Repeat playlist",
        None,
        &[10496],
        &[],
    ),
    info(
        Action::SetRepeat(Repeat::Item),
        "repeat-item",
        "Enable: Repeat playing item",
        None,
        &[10497],
        &[],
    ),
    info(
        Action::ToggleKeyframeSeeks,
        "keyframe-seeks",
        "Use keyframe to jump to a time",
        None,
        &[10265],
        &["ID_KEYFRAME_MOVE"],
    ),
    info(
        Action::TogglePreventScreenSaver,
        "prevent-screen-saver",
        "Prevent screen saver during playback",
        None,
        &[10122],
        &[],
    ),
    info(
        Action::AfterPlaybackCycle,
        "after-playback-cycle",
        "Cycle Playback Finished Action",
        Some("Ctrl+Alt+A"),
        &[10336],
        &[],
    ),
    info(
        Action::SetAfterPlayback(AfterPlayback::Nothing),
        "after-playback-nothing",
        "Do nothing when playback ends",
        None,
        &[],
        &[],
    ),
    info(
        Action::SetAfterPlayback(AfterPlayback::ExitAfterFile),
        "exit-after-file",
        "Exit RVP when playback ends",
        None,
        &[10116],
        &["ID_ETC_CLOSE_PROGRAM_FILE"],
    ),
    info(
        Action::SetAfterPlayback(AfterPlayback::ExitAfterList),
        "exit-after-list",
        "Exit RVP when playlist ends",
        None,
        &[10117],
        &["ID_ETC_CLOSE_PROGRAM_LIST"],
    ),
    // Audio.
    info(
        Action::VolumeUp,
        "volume-up",
        "Increase Volume",
        Some("Up"),
        &[10035],
        &[],
    ),
    info(
        Action::VolumeDown,
        "volume-down",
        "Decrease Volume",
        Some("Down"),
        &[10036],
        &[],
    ),
    info(
        Action::ToggleMute,
        "toggle-mute",
        "Mute",
        Some("M"),
        &[10037],
        &["ID_AUDIO_MUTE"],
    ),
    info(
        Action::ToggleNormalizer,
        "toggle-normalizer",
        "Normalizer",
        Some("Shift+N"),
        &[10009],
        &[],
    ),
    info(
        Action::ToggleBoost,
        "toggle-boost",
        "Normalizer: Boost audio",
        None,
        &[],
        &[],
    ),
    info(
        Action::CycleAudio,
        "cycle-audio",
        "Cycle Audio Stream",
        Some("Alt+A"),
        &[36799],
        &[],
    ),
    info(
        Action::AudioDelay(Delay::Earlier),
        "audio-delay-down",
        "Decrease audio sync by %s",
        Some("Shift+,"),
        &[10291],
        &["ID_AUDIO_FASTER"],
    ),
    info(
        Action::AudioDelay(Delay::Later),
        "audio-delay-up",
        "Increase audio sync by %s",
        Some("Shift+."),
        &[10292],
        &["ID_AUDIO_SLOWER"],
    ),
    info(
        Action::AudioDelay(Delay::Reset),
        "audio-delay-reset",
        "Default Audio Sync",
        Some("Shift+/"),
        &[10293],
        &["ID_AUDIO_NORMAL"],
    ),
    info(
        Action::Audio(AudioCommand::Channel(StereoChannel::Both)),
        "audio-channel-both",
        "Both",
        None,
        &[37750],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Channel(StereoChannel::Left)),
        "audio-channel-left",
        "Left channel",
        None,
        &[37751],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Channel(StereoChannel::Right)),
        "audio-channel-right",
        "Right channel",
        None,
        &[37752],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Method(VolumeMethod::Player)),
        "volume-method-player",
        "Default volume control (Recommended)",
        None,
        &[10393],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Method(VolumeMethod::Wave)),
        "volume-method-wave",
        "System Wave volume control",
        None,
        &[10394],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Method(VolumeMethod::Master)),
        "volume-method-master",
        "System Master volume control",
        None,
        &[10395],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::ChannelVolumes),
        "individual-volume",
        "Enable individual volume control",
        None,
        &[10435],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::MasterUp),
        "master-volume-up",
        "Increase master volume",
        Some("Shift+Up"),
        &[10520],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::MasterDown),
        "master-volume-down",
        "Decrease master volume",
        Some("Shift+Down"),
        &[10521],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Preamp),
        "preamp",
        "Pre-amplify AC3/DTS Volume to proper level",
        None,
        &[10436],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::PreampUp),
        "preamp-up",
        "Increase PreAmp",
        None,
        &[10522],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::PreampDown),
        "preamp-down",
        "Decrease PreAmp",
        None,
        &[10523],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SystemMasterUp),
        "system-master-up",
        "Increase Master Volume",
        Some("Ctrl+Alt+Shift+Up"),
        &[10267],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SystemMasterDown),
        "system-master-down",
        "Decrease Master Volume",
        Some("Ctrl+Alt+Shift+Down"),
        &[10268],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SystemMasterMute),
        "system-master-mute",
        "Mute (Master)",
        Some("Ctrl+Alt+Shift+M"),
        &[10269],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SystemWaveUp),
        "system-wave-up",
        "Increase Wave Volume",
        Some("Ctrl+Alt+Up"),
        &[10270],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SystemWaveDown),
        "system-wave-down",
        "Decrease Wave Volume",
        Some("Ctrl+Alt+Down"),
        &[10271],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SystemWaveMute),
        "system-wave-mute",
        "Mute (Wave)",
        Some("Ctrl+Alt+M"),
        &[10272],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SyncDialog),
        "audio-sync-dialog",
        "Adjust Audio Sync...",
        None,
        &[10878],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Keep(SyncKeep::Never)),
        "audio-sync-keep-never",
        "Do not keep audio sync changes",
        None,
        &[10625],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Keep(SyncKeep::Runtime)),
        "audio-sync-keep-runtime",
        "Keep audio sync changes over whole runtime",
        None,
        &[10626],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Keep(SyncKeep::Always)),
        "audio-sync-keep-always",
        "Always keep audio sync changes",
        None,
        &[10627],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Crystality),
        "crystality",
        "Crystality",
        Some("Shift+C"),
        &[10389],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::NoiseReduction),
        "noise-reduction",
        "Noise Reduction",
        Some("Shift+D"),
        &[10390],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Freeverb),
        "freeverb",
        "Freeverb",
        Some("Shift+R"),
        &[10096],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::VoiceRemoval),
        "voice-removal",
        "Voice Removal",
        Some("Shift+V"),
        &[10438],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::VoiceEmphasis),
        "voice-emphasis",
        "Voice Emphasis",
        Some("Shift+A"),
        &[10439],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::SwapChannels),
        "swap-channels",
        "Swap Stereo Channels",
        Some("Shift+T"),
        &[10660],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Crossfeed),
        "crossfeed",
        "Crossfeed",
        None,
        &[10661],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::RecordDialog),
        "record-audio",
        "Record Audio...",
        Some("Shift+G"),
        &[10226],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::RecordToggle),
        "record-audio-toggle",
        "Start/Stop Audio Recording",
        None,
        &[10798],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::RecordFolderDialog),
        "select-record-folder",
        "Set Audio Storage Folder...",
        None,
        &[10335],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::OpenRecordFolder),
        "open-record-folder",
        "Open Audio Storage Folder",
        None,
        &[10227],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::RecordInMediaFolder),
        "record-in-media-folder",
        "Store recorded audios into play folders (if available)",
        None,
        &[10915],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Stretch(Stretch::DefaultQuality)),
        "stretch-default",
        "Use default quality audio time stretching",
        None,
        &[10396],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Stretch(Stretch::HighQuality)),
        "stretch-high",
        "Use high quality audio time stretching (Recom'd)",
        None,
        &[10397],
        &[],
    ),
    info(
        Action::Audio(AudioCommand::Stretch(Stretch::Pitch)),
        "pitch-shifting",
        "Use audio pitch shifting",
        None,
        &[10398],
        &[],
    ),
    // Subtitles.
    info(
        Action::ToggleSubtitles,
        "toggle-subtitles",
        "Show/Hide Subtitles",
        Some("Alt+H"),
        &[10126],
        &["ID_CAPTION_VISIBLE"],
    ),
    info(
        Action::CycleSubtitles,
        "cycle-subtitles",
        "Cycle Subtitle",
        Some("Alt+L"),
        &[29199],
        &[],
    ),
    info(
        Action::SubtitleDelay(Delay::Earlier),
        "subtitle-delay-down",
        "Decrease Subtitle Sync by %s",
        Some("<"),
        &[10141],
        &["ID_CAPTION_FAST_SYNC"],
    ),
    info(
        Action::SubtitleDelay(Delay::Later),
        "subtitle-delay-up",
        "Increase Subtitle Sync by %s",
        Some(">"),
        &[10140],
        &["ID_CAPTION_SLOW_SYNC"],
    ),
    info(
        Action::SubtitleDelay(Delay::Reset),
        "subtitle-delay-reset",
        "Default Subtitle Sync",
        Some("/"),
        &[10142],
        &["ID_CAPTION_RESET_SYNC"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::Bigger),
        "subtitle-bigger",
        "Increase Subtitle Font Size",
        Some("Alt+PgUp"),
        &[10137],
        &["ID_CAPTION_INC_SIZE"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::Smaller),
        "subtitle-smaller",
        "Decrease Subtitle Font Size",
        Some("Alt+PgDn"),
        &[10138],
        &["ID_CAPTION_DEC_SIZE"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::Up),
        "subtitle-up",
        "Move Subtitles Up",
        Some("Alt+Up"),
        &[10128],
        &["ID_CAPTION_MOVE_UP"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::Down),
        "subtitle-down",
        "Move Subtitles Down",
        Some("Alt+Down"),
        &[10129],
        &["ID_CAPTION_MOVE_DOWN"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::DefaultPosition),
        "subtitle-default-position",
        "Default Subtitle Position",
        Some("Alt+Home"),
        &[10127],
        &[],
    ),
    info(
        Action::Subtitle(SubtitleStyle::AlignLeft),
        "subtitle-align-left",
        "Align Subtitles Left",
        None,
        &[10143],
        &["ID_CAPTION_ALIGN_LEFT"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::AlignCenter),
        "subtitle-align-center",
        "Align Subtitles Center",
        None,
        &[10144],
        &["ID_CAPTION_ALIGN_CENTER"],
    ),
    info(
        Action::Subtitle(SubtitleStyle::AlignRight),
        "subtitle-align-right",
        "Align Subtitles Right",
        None,
        &[10145],
        &["ID_CAPTION_ALIGN_RIGHT"],
    ),
    info(
        Action::SubtitleFont,
        "subtitle-font",
        "Font Settings...",
        Some("Alt+F"),
        &[10139],
        &["ID_CAPTION_FONT_SELECT"],
    ),
    // Video.
    info(
        Action::CycleVideo,
        "cycle-video",
        "Cycle Video Stream",
        None,
        &[44699],
        &[],
    ),
    info(
        Action::AspectCycle,
        "aspect-cycle",
        "Cycle Aspect Ratio",
        Some("Ctrl+F6"),
        &[10261],
        &[],
    ),
    info(
        Action::SetAspect(Aspect::Default),
        "aspect-default",
        "Default Display AR (Recommended)",
        None,
        &[10015],
        &["ID_AUTO_RATIO"],
    ),
    info(
        Action::SetAspect(Aspect::Original),
        "aspect-original",
        "Default Original DAR",
        None,
        &[10016],
        &["ID_SOURCE_RATIO"],
    ),
    info(
        Action::SetAspect(Aspect::R4x3),
        "aspect-4-3",
        "4:3",
        None,
        &[10017],
        &["ID_4_3_RATIO"],
    ),
    info(
        Action::SetAspect(Aspect::R16x9),
        "aspect-16-9",
        "16:9",
        None,
        &[10018],
        &["ID_16_9_RATIO"],
    ),
    info(
        Action::SetAspect(Aspect::R16x10),
        "aspect-16-10",
        "16:10",
        None,
        &[10019],
        &["ID_16_10_RATIO"],
    ),
    info(
        Action::SetAspect(Aspect::R185x1),
        "aspect-1.85",
        "1.85:1",
        None,
        &[10020],
        &["ID_185_1_RATIO"],
    ),
    info(
        Action::SetAspect(Aspect::R235x1),
        "aspect-2.35",
        "2.35:1",
        None,
        &[10021],
        &["ID_235_1_RATIO"],
    ),
    info(
        Action::VideoFit(Fit::KeepAspect),
        "fit-keep-aspect",
        "Keep Aspect Ratio (Recommended)",
        None,
        &[10040],
        &["ID_AR_INSIDE_RATIO"],
    ),
    info(
        Action::VideoFit(Fit::Crop),
        "fit-crop",
        "Stretch by Keeping Aspect Ratio",
        None,
        &[10041],
        &["ID_AR_OUTSIDE_RATIO"],
    ),
    info(
        Action::VideoFit(Fit::Stretch),
        "fit-stretch",
        "Stretch",
        None,
        &[10042],
        &["ID_FIT_RATIO"],
    ),
    info(
        Action::Zoom(Zoom::ResetSize),
        "zoom-reset",
        "Actual/Last used size",
        Some("Num5"),
        &[10058],
        &[],
    ),
    info(
        Action::Zoom(Zoom::In),
        "zoom-in",
        "Zoom In",
        Some("Num9"),
        &[10047],
        &["ID_PANSCAN_INCSIZE"],
    ),
    info(
        Action::Zoom(Zoom::Out),
        "zoom-out",
        "Zoom Out",
        Some("Num1"),
        &[10048],
        &["ID_PANSCAN_DECSIZE"],
    ),
    info(
        Action::Zoom(Zoom::Wider),
        "zoom-wider",
        "Stretch Horizontally",
        Some("Num6"),
        &[10049],
        &["ID_PANSCAN_INCWIDTH"],
    ),
    info(
        Action::Zoom(Zoom::Narrower),
        "zoom-narrower",
        "Compress Horizontally",
        Some("Num4"),
        &[10050],
        &["ID_PANSCAN_DECWIDTH"],
    ),
    info(
        Action::Zoom(Zoom::Taller),
        "zoom-taller",
        "Stretch Vertically",
        Some("Num8"),
        &[10051],
        &["ID_PANSCAN_INCHEIGHT"],
    ),
    info(
        Action::Zoom(Zoom::Shorter),
        "zoom-shorter",
        "Compress Vertically",
        Some("Num2"),
        &[10052],
        &["ID_PANSCAN_DECHEIGHT"],
    ),
    info(
        Action::Zoom(Zoom::Right),
        "pan-right",
        "Move Picture Right",
        Some("Ctrl+Num6"),
        &[10053],
        &["ID_PANSCAN_MOVERIGHT"],
    ),
    info(
        Action::Zoom(Zoom::Left),
        "pan-left",
        "Move Picture Left",
        Some("Ctrl+Num4"),
        &[10054],
        &["ID_PANSCAN_MOVELEFT"],
    ),
    info(
        Action::Zoom(Zoom::Up),
        "pan-up",
        "Move Picture Up",
        Some("Ctrl+Num8"),
        &[10055],
        &["ID_PANSCAN_MOVEUP"],
    ),
    info(
        Action::Zoom(Zoom::Down),
        "pan-down",
        "Move Picture Down",
        Some("Ctrl+Num2"),
        &[10056],
        &["ID_PANSCAN_MOVEDOWN"],
    ),
    info(
        Action::Zoom(Zoom::ResetPosition),
        "pan-reset",
        "Actual Position",
        Some("Ctrl+Num5"),
        &[10057],
        &[],
    ),
    info(
        Action::Zoom(Zoom::Reset),
        "zoom-pan-reset",
        "Reset Size and Position",
        None,
        &[],
        &["ID_PANSCAN_RESET"],
    ),
    info(
        Action::SaveFrame,
        "save-frame",
        "Save Current Source Frame",
        Some("Ctrl+E"),
        &[10224],
        &["ID_VIDEO_IMAGE_CAPTURE"],
    ),
    info(
        Action::SaveScreen,
        "save-screen",
        "Save Current Screen Frame",
        Some("Ctrl+Alt+E"),
        &[10433],
        &[],
    ),
    info(
        Action::SetImageFormat(ImageFormat::Jpeg),
        "image-jpeg",
        "Save images in JPEG format",
        None,
        &[10355],
        &["ID_VIDEO_IMAGE_JPEG"],
    ),
    info(
        Action::SetImageFormat(ImageFormat::Png),
        "image-png",
        "Save images in PNG format",
        None,
        &[10356],
        &["ID_VIDEO_IMAGE_PNG"],
    ),
    info(
        Action::OpenImageFolder,
        "open-image-folder",
        "Open Image Storage Folder",
        None,
        &[10225],
        &["ID_VIDEO_OPEN_FOLDER"],
    ),
    info(
        Action::SelectImageFolder,
        "select-image-folder",
        "Set Image Storage Folder...",
        None,
        &[10334],
        &["ID_VIDEO_SELECT_FOLDER"],
    ),
    // Video processing (M6).
    info(
        Action::DeinterlaceCycle,
        "deinterlace-cycle",
        "Cycle Deinterlacing Mode",
        Some("Ctrl+I"),
        &[10420],
        &[],
    ),
    info(
        Action::SetDeinterlace(Deinterlace::Auto),
        "deinterlace-auto",
        "Auto",
        None,
        &[10421],
        &[],
    ),
    info(
        Action::SetDeinterlace(Deinterlace::On),
        "deinterlace-on",
        "Enabled",
        None,
        &[10422],
        &[],
    ),
    info(
        Action::SetDeinterlace(Deinterlace::Off),
        "deinterlace-off",
        "Disabled",
        None,
        &[10423],
        &[],
    ),
    info(
        Action::VideoEffect(VideoEffect::FlipVertical),
        "flip-vertical",
        "Flip Vertical",
        Some("Ctrl+P"),
        &[10070],
        &[],
    ),
    info(
        Action::VideoEffect(VideoEffect::FlipHorizontal),
        "flip-horizontal",
        "Flip Horizontal",
        Some("Ctrl+Z"),
        &[10071],
        &[],
    ),
    info(
        Action::VideoEffect(VideoEffect::Soften),
        "soften",
        "Soften",
        Some("Ctrl+B"),
        &[10257],
        &["ID_VIDEO_BLUR"],
    ),
    info(
        Action::VideoEffect(VideoEffect::Sharpen),
        "sharpen",
        "Sharpen",
        Some("Ctrl+R"),
        &[10256],
        &["ID_VIDEO_SHARPEN"],
    ),
    info(
        Action::VideoEffect(VideoEffect::Deblock),
        "deblock",
        "Deblock",
        Some("Ctrl+H"),
        &[10218],
        &["ID_VIDEO_POSTPROC"],
    ),
    info(
        Action::VideoEffect(VideoEffect::Denoise),
        "denoise",
        "Denoise 3D",
        Some("Ctrl+N"),
        &[10425],
        &[],
    ),
    info(
        Action::ToggleDeband,
        "deband",
        "Deband",
        None,
        &[10837],
        &[],
    ),
    info(
        Action::Color(ColorChange::Toggle),
        "color-toggle",
        "Disable/Last used Color Controls",
        Some("Q"),
        &[10085],
        &[],
    ),
    info(
        Action::Color(ColorChange::Down(ColorKind::Brightness)),
        "brightness-down",
        "Decrease Brightness by 1%",
        Some("W"),
        &[10086],
        &[],
    ),
    info(
        Action::Color(ColorChange::Up(ColorKind::Brightness)),
        "brightness-up",
        "Increase Brightness by 1%",
        Some("E"),
        &[10087],
        &[],
    ),
    info(
        Action::Color(ColorChange::Down(ColorKind::Contrast)),
        "contrast-down",
        "Decrease Contrast by 1%",
        Some("R"),
        &[10088],
        &[],
    ),
    info(
        Action::Color(ColorChange::Up(ColorKind::Contrast)),
        "contrast-up",
        "Increase Contrast by 1%",
        Some("T"),
        &[10089],
        &[],
    ),
    info(
        Action::Color(ColorChange::Down(ColorKind::Saturation)),
        "saturation-down",
        "Decrease Saturation by 1%",
        Some("Y"),
        &[10090],
        &[],
    ),
    info(
        Action::Color(ColorChange::Up(ColorKind::Saturation)),
        "saturation-up",
        "Increase Saturation by 1%",
        Some("U"),
        &[10091],
        &[],
    ),
    info(
        Action::Color(ColorChange::Down(ColorKind::Hue)),
        "hue-down",
        "Decrease Hue by 1%",
        Some("I"),
        &[10092],
        &[],
    ),
    info(
        Action::Color(ColorChange::Up(ColorKind::Hue)),
        "hue-up",
        "Increase Hue by 1%",
        Some("O"),
        &[10093],
        &[],
    ),
    info(
        Action::Color(ColorChange::Down(ColorKind::Gamma)),
        "gamma-down",
        "Decrease Gamma by 1%",
        None,
        &[],
        &[],
    ),
    info(
        Action::Color(ColorChange::Up(ColorKind::Gamma)),
        "gamma-up",
        "Increase Gamma by 1%",
        None,
        &[],
        &[],
    ),
    info(
        Action::Color(ColorChange::Reset(ColorKind::Brightness)),
        "brightness-reset",
        "Default Brightness",
        None,
        &[],
        &["CMD_COLOR_OFFSET_DEFAULT"],
    ),
    info(
        Action::Color(ColorChange::Reset(ColorKind::Contrast)),
        "contrast-reset",
        "Default Contrast",
        None,
        &[],
        &["CMD_COLOR_GAIN_DEFAULT"],
    ),
    info(
        Action::Color(ColorChange::Reset(ColorKind::Saturation)),
        "saturation-reset",
        "Default Saturation",
        None,
        &[],
        &["CMD_COLOR_SAT_DEFAULT"],
    ),
    info(
        Action::Color(ColorChange::Reset(ColorKind::Hue)),
        "hue-reset",
        "Default Hue",
        None,
        &[],
        &["CMD_COLOR_HUE_DEFAULT"],
    ),
    info(
        Action::Color(ColorChange::Reset(ColorKind::Gamma)),
        "gamma-reset",
        "Default Gamma",
        None,
        &[],
        &[],
    ),
    info(
        Action::Rotate(Rotation::Cycle),
        "rotate-cycle",
        "Cycle Screen Rotation",
        Some("Alt+K"),
        &[10612],
        &[],
    ),
    info(
        Action::Rotate(Rotation::To(0)),
        "rotate-0",
        "Do nothing",
        None,
        &[10613],
        &[],
    ),
    info(
        Action::Rotate(Rotation::To(90)),
        "rotate-90",
        "90 degrees Rotation",
        None,
        &[10614],
        &[],
    ),
    info(
        Action::Rotate(Rotation::To(180)),
        "rotate-180",
        "180 degrees Rotation",
        None,
        &[10615],
        &[],
    ),
    info(
        Action::Rotate(Rotation::To(270)),
        "rotate-270",
        "270 degrees Rotation",
        None,
        &[10616],
        &[],
    ),
    info(
        Action::Shaders(ShaderChange::Toggle),
        "shaders-toggle",
        "Use Pixel Shaders",
        Some("Shift+P"),
        &[10290],
        &[],
    ),
    info(
        Action::Shaders(ShaderChange::On),
        "shaders-on",
        "Enable Shader Stacker",
        None,
        &[37501],
        &[],
    ),
    info(
        Action::Shaders(ShaderChange::Off),
        "shaders-off",
        "Do not use",
        None,
        &[37500],
        &[],
    ),
    info(
        Action::Shaders(ShaderChange::Reload),
        "shaders-reload",
        "Reload Pixel Shader list",
        None,
        &[10289],
        &[],
    ),
    info(
        Action::Shaders(ShaderChange::OpenFolder),
        "shaders-folder",
        "Open Pixel Shader folder",
        None,
        &[10288],
        &[],
    ),
    info(
        Action::ToggleVapourSynth,
        "toggle-vapoursynth",
        "VapourSynth Script",
        None,
        &[],
        &[],
    ),
    info(
        Action::SaveFrameAs,
        "save-frame-as",
        "Save Current Frame As...",
        None,
        &[],
        &["ID_VIDEO_IMAGE_CAPTURE_PROMPT"],
    ),
    info(
        Action::ToggleEqualizer,
        "toggle-equalizer",
        "Use Equalizer",
        Some("Shift+E"),
        &[10095],
        &["ID_AUDIO_EQULIZER"],
    ),
    info(
        Action::ControlPanel,
        "control-panel",
        "Control Panel...",
        Some("F7"),
        &[10383],
        &["ID_CONTROL_BOX_WINDOW"],
    ),
    // Window.
    info(
        Action::ToggleFullscreen,
        "toggle-fullscreen",
        "Fullscreen (Keep AR)",
        Some("Alt+Enter"),
        &[10013],
        &["CMD_FULLSCREEN", "ID_VIEW_FULLSCREEN"],
    ),
    info(
        Action::ToggleFullscreenStretch,
        "toggle-fullscreen-stretch",
        "Fullscreen (Stretch)",
        Some("Ctrl+Enter"),
        &[10308],
        &[],
    ),
    info(
        Action::Minimize,
        "minimize",
        "Minimize (to Taskbar/Tray)",
        None,
        &[10230],
        &["CMD_MIN_WINDOW"],
    ),
    info(
        Action::MaximizeRestore,
        "maximize-restore",
        "Maximize (to Desktop) / Restore",
        Some("6"),
        &[10237, 10238],
        &["CMD_MAX_WINDOW", "CMD_RESTORE_WINDOW"],
    ),
    info(
        Action::WindowScale(WindowScale::Half),
        "window-half",
        "0.5x video output size",
        Some("1"),
        &[10232],
        &[],
    ),
    info(
        Action::WindowScale(WindowScale::Normal),
        "window-normal",
        "1.0x video output size",
        Some("2"),
        &[10233],
        &[],
    ),
    info(
        Action::WindowScale(WindowScale::OneAndHalf),
        "window-one-and-half",
        "1.5x video output size",
        Some("3"),
        &[10234],
        &[],
    ),
    info(
        Action::WindowScale(WindowScale::Double),
        "window-double",
        "2.0x video output size",
        Some("4"),
        &[10235],
        &[],
    ),
    info(
        Action::WindowFitAspect,
        "window-fit-aspect",
        "Fit to video aspect ratio",
        Some("8"),
        &[10841],
        &[],
    ),
    info(
        Action::OnTopCycle,
        "on-top-cycle",
        "Cycle Stay on Top",
        Some("Ctrl+T"),
        &[10337],
        &["ID_ETC_NEXTONTOP"],
    ),
    info(
        Action::SetOnTop(OnTop::Never),
        "on-top-never",
        "Stay on Top: Never",
        None,
        &[],
        &[],
    ),
    info(
        Action::SetOnTop(OnTop::Always),
        "on-top-always",
        "Stay on Top: Always",
        None,
        &[10038],
        &[],
    ),
    info(
        Action::SetOnTop(OnTop::WhilePlaying),
        "on-top-while-playing",
        "Stay on Top: While playing",
        None,
        &[10039],
        &[],
    ),
    info(Action::ContextMenu, "context-menu", "Menu", None, &[], &[]),
    info(
        Action::TogglePlaylist,
        "toggle-playlist",
        "Playlist...",
        Some("F6"),
        &[10011],
        &[
            "ID_PLAYLIST_WINDOW",
            "CMD_ATTACH_WINDOW_OPEN",
            "CMD_ATTACH_WINDOW_CLOSE",
            "CMD_PLAYLIST_WINDOW_OPEN",
            "CMD_PLAYLIST_WINDOW_CLOSE",
            "CMD_ATTACH_PLAYLIST",
        ],
    ),
    info(
        Action::PlaybackInfo,
        "playback-info",
        "Display Full Playback/Audio Tag Info on Screen",
        Some("Tab"),
        &[10351, 10195],
        &["ID_PLAYINFO_OSD", "ID_PLAYINFO_WINDOW"],
    ),
    info(
        Action::Preferences,
        "preferences",
        "Preferences...",
        Some("F5"),
        &[10010, 10970],
        &[],
    ),
    info(
        Action::Settings(SettingsPage::Playback),
        "playback-settings",
        "Playback Settings...",
        None,
        &[10757],
        &[],
    ),
    info(
        Action::Settings(SettingsPage::Playlist),
        "playlist-settings",
        "Playlist Settings...",
        None,
        &[10315],
        &["ID_PLAYLIST_SETTING"],
    ),
    info(
        Action::Settings(SettingsPage::Audio),
        "audio-settings",
        "Audio Processing Settings...",
        None,
        &[10391],
        &["ID_AUDIO_OUT_SETTING"],
    ),
    info(
        Action::Settings(SettingsPage::AudioVolume),
        "volume-settings",
        "Volume control settings...",
        None,
        &[10437],
        &[],
    ),
    info(
        Action::Settings(SettingsPage::AudioSync),
        "audio-sync-settings",
        "Audio Language/Sync/Other Settings...",
        None,
        &[10756],
        &[],
    ),
    info(
        Action::Settings(SettingsPage::Video),
        "video-settings",
        "Video Processing Settings...",
        None,
        &[10363, 10424, 10529],
        &[],
    ),
    info(
        Action::Settings(SettingsPage::Shaders),
        "shader-settings",
        "Stack...",
        None,
        &[10287, 10655],
        &[],
    ),
    info(
        Action::Settings(SettingsPage::Subtitles),
        "subtitle-settings",
        "Subtitle Processing Settings...",
        None,
        &[10760],
        &[],
    ),
    info(
        Action::KeysAndMouse,
        "keys-and-mouse",
        "Keys and Mouse...",
        None,
        &[],
        &[],
    ),
    info(
        Action::ImportPlaylists,
        "import-playlists",
        "Import PotPlayer Playlists...",
        None,
        &[],
        &[],
    ),
    info(
        Action::About,
        "about",
        "About...",
        Some("F1"),
        &[57664],
        &[],
    ),
];

impl Action {
    /// All actions, in a stable order.
    pub fn all() -> impl Iterator<Item = Action> {
        ACTIONS.iter().map(|info| info.action)
    }

    /// This action's row of [`ACTIONS`].
    #[must_use]
    pub fn info(self) -> &'static ActionInfo {
        ACTIONS
            .iter()
            .find(|info| info.action == self)
            .unwrap_or(&ACTIONS[0])
    }

    /// The stable name used in settings files.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// The action with this name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Action> {
        ACTIONS
            .iter()
            .find(|info| info.name == name)
            .map(|info| info.action)
    }

    /// The action of a numeric PotPlayer command ID (menu items and the
    /// default hotkeys).
    #[must_use]
    pub fn from_potplayer_id(id: u32) -> Option<Action> {
        ACTIONS
            .iter()
            .find(|info| info.ids.contains(&id))
            .map(|info| info.action)
    }

    /// The action of a skin control's `CmdID` or `AltCmdID` (right click)
    /// or a skin menu item. Sliders, labels and menus are not actions; the
    /// shell handles them directly.
    #[must_use]
    pub fn from_skin_command(command: &str) -> Option<Action> {
        ACTIONS
            .iter()
            .find(|info| info.skin.contains(&command))
            .map(|info| info.action)
    }

    /// PotPlayer's menu text (`docs/r1/menu-static.tsv`) and the shortcut
    /// its menu shows; `%s` stands for a step's length.
    #[must_use]
    pub fn menu_text(self) -> (&'static str, Option<&'static str>) {
        let info = self.info();
        (info.label, info.shortcut)
    }

    /// True for actions that a second invocation undoes (used when a single
    /// click's action has to be taken back because the click became a
    /// double click).
    #[must_use]
    pub fn is_toggle(self) -> bool {
        matches!(
            self,
            Action::PlayPause
                | Action::ToggleMute
                | Action::ToggleFullscreen
                | Action::ToggleFullscreenStretch
                | Action::MaximizeRestore
                | Action::TogglePlaylist
                | Action::ToggleSubtitles
                | Action::ToggleNormalizer
                | Action::ToggleBoost
                | Action::ToggleKeyframeSeeks
                | Action::TogglePreventScreenSaver
                | Action::PlaybackInfo
        )
    }
}

/// An action of the playlist panel while it has the keyboard (PotPlayer's
/// playlist accelerators, table 118 in `docs/r1/hotkeys.tsv`, plus undo,
/// redo and the cursor keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ListAction {
    Play,
    SelectAll,
    InvertSelection,
    RemoveSelected,
    RemoveUnselected,
    RemoveAll,
    RemoveMissing,
    AddFiles,
    AddFolder,
    AddAlbum,
    RenameAlbum,
    DeleteAlbum,
    SortAscending,
    SortDescending,
    Shuffle,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    Undo,
    Redo,
    CursorUp,
    CursorDown,
    CursorHome,
    CursorEnd,
    PageUp,
    PageDown,
    /// Hide the attached playlist (F6).
    Close,
}

/// Every playlist action with its settings name and PotPlayer's menu text.
const LIST_NAMES: &[(ListAction, &str, &str)] = &[
    (ListAction::Play, "play", "Play"),
    (ListAction::SelectAll, "select-all", "Select All"),
    (
        ListAction::InvertSelection,
        "invert-selection",
        "Invert Selection",
    ),
    (
        ListAction::RemoveSelected,
        "remove-selected",
        "Remove selected items",
    ),
    (
        ListAction::RemoveUnselected,
        "remove-unselected",
        "Remove unselected items",
    ),
    (ListAction::RemoveAll, "remove-all", "Remove all items"),
    (
        ListAction::RemoveMissing,
        "remove-missing",
        "Remove nonexistent items",
    ),
    (ListAction::AddFiles, "add-files", "Add file(s)..."),
    (ListAction::AddFolder, "add-folder", "Add folder..."),
    (ListAction::AddAlbum, "add-album", "Add a new album..."),
    (ListAction::RenameAlbum, "rename-album", "Edit album..."),
    (ListAction::DeleteAlbum, "delete-album", "Delete album"),
    (
        ListAction::SortAscending,
        "sort-ascending",
        "Ascending order (A to Z)",
    ),
    (
        ListAction::SortDescending,
        "sort-descending",
        "Descending order (Z to A)",
    ),
    (ListAction::Shuffle, "shuffle", "Randomly"),
    (ListAction::MoveUp, "move-up", "Move Up"),
    (ListAction::MoveDown, "move-down", "Move Down"),
    (ListAction::MoveTop, "move-top", "Move Top"),
    (ListAction::MoveBottom, "move-bottom", "Move End"),
    (ListAction::Undo, "undo", "Undo"),
    (ListAction::Redo, "redo", "Redo"),
    (ListAction::CursorUp, "cursor-up", "Select previous item"),
    (ListAction::CursorDown, "cursor-down", "Select next item"),
    (ListAction::CursorHome, "cursor-home", "Select first item"),
    (ListAction::CursorEnd, "cursor-end", "Select last item"),
    (ListAction::PageUp, "page-up", "Page up"),
    (ListAction::PageDown, "page-down", "Page down"),
    (ListAction::Close, "close", "Close Playlist"),
];

impl ListAction {
    pub fn all() -> impl Iterator<Item = ListAction> {
        LIST_NAMES.iter().map(|&(action, _, _)| action)
    }

    /// The stable name used in settings files (`playlist-keys`).
    #[must_use]
    pub fn name(self) -> &'static str {
        LIST_NAMES
            .iter()
            .find(|&&(action, _, _)| action == self)
            .map_or("", |&(_, name, _)| name)
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<ListAction> {
        LIST_NAMES
            .iter()
            .find(|&&(_, candidate, _)| candidate == name)
            .map(|&(action, _, _)| action)
    }

    /// PotPlayer's menu text.
    #[must_use]
    pub fn label(self) -> &'static str {
        LIST_NAMES
            .iter()
            .find(|&&(action, _, _)| action == self)
            .map_or("", |&(_, _, label)| label)
    }

    /// The action of a command ID from PotPlayer's playlist accelerators.
    #[must_use]
    pub fn from_potplayer_id(id: u32) -> Option<ListAction> {
        Some(match id {
            10199 => ListAction::Play,
            10204 => ListAction::SelectAll,
            10205 => ListAction::InvertSelection,
            10200 => ListAction::RemoveSelected,
            10212 => ListAction::RemoveUnselected,
            10201 => ListAction::RemoveAll,
            10210 => ListAction::RemoveMissing,
            10196 => ListAction::AddFiles,
            10197 => ListAction::AddFolder,
            10002 => ListAction::AddAlbum,
            10003 => ListAction::RenameAlbum,
            10004 => ListAction::DeleteAlbum,
            10202 => ListAction::SortAscending,
            10203 => ListAction::SortDescending,
            10988 => ListAction::Shuffle,
            // Unnamed in the menus; Ctrl+Up/Down/Home/End move the selected
            // items, like the panel's Move buttons.
            27020 => ListAction::MoveUp,
            27021 => ListAction::MoveDown,
            27022 => ListAction::MoveTop,
            27023 => ListAction::MoveBottom,
            10446 => ListAction::Close,
            _ => return None,
        })
    }

    /// The action of a playlist panel button's `CmdID`.
    #[must_use]
    pub fn from_skin_command(command: &str) -> Option<ListAction> {
        Some(match command {
            "CMD_PLAYLIST_TOP" => ListAction::MoveTop,
            "CMD_PLAYLIST_UP" => ListAction::MoveUp,
            "CMD_PLAYLIST_DOWN" => ListAction::MoveDown,
            "CMD_PLAYLIST_BOTTOM" => ListAction::MoveBottom,
            "CMD_PLAYLIST_ADD" => ListAction::AddFiles,
            "CMD_PLAYLIST_DEL" => ListAction::RemoveSelected,
            _ => return None,
        })
    }

    /// Cursor movements, which Shift turns into extending the selection.
    #[must_use]
    pub fn is_cursor(self) -> bool {
        matches!(
            self,
            ListAction::CursorUp
                | ListAction::CursorDown
                | ListAction::CursorHome
                | ListAction::CursorEnd
                | ListAction::PageUp
                | ListAction::PageDown
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_table_is_consistent() {
        let mut names = BTreeSet::new();
        let mut actions = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut skin = BTreeSet::new();
        for info in ACTIONS {
            assert!(!info.name.is_empty(), "{:?} has no name", info.action);
            assert!(names.insert(info.name), "duplicate name {}", info.name);
            assert!(actions.insert(info.action), "{:?} twice", info.action);
            for id in info.ids {
                assert!(ids.insert(*id), "ID {id} maps to two actions");
            }
            for command in info.skin {
                assert!(skin.insert(*command), "{command} maps to two actions");
            }
            assert_eq!(Action::from_name(info.name), Some(info.action));
            assert_eq!(info.action.info().name, info.name);
        }
        assert_eq!(Action::from_name("no-such-action"), None);
    }

    #[test]
    fn potplayer_ids() {
        assert_eq!(Action::from_potplayer_id(10014), Some(Action::PlayPause));
        assert_eq!(
            Action::from_potplayer_id(10013),
            Some(Action::ToggleFullscreen)
        );
        assert_eq!(
            Action::from_potplayer_id(10061),
            Some(Action::SeekBackward(SeekStep::Medium))
        );
        assert_eq!(
            Action::from_potplayer_id(10248),
            Some(Action::Speed(Speed::Faster))
        );
        assert_eq!(Action::from_potplayer_id(10010), Some(Action::Preferences));
        assert_eq!(Action::from_potplayer_id(1), None);
    }

    #[test]
    fn skin_commands() {
        for (command, action) in [
            ("CMD_MEDIA_PLAY", Action::PlayPause),
            ("ID_PROCESS_PREV", Action::Previous),
            ("ID_PROCESS_NEXT", Action::Next),
            ("ID_AUDIO_MUTE", Action::ToggleMute),
            ("CMD_MIN_WINDOW", Action::Minimize),
            ("CMD_MAX_WINDOW", Action::MaximizeRestore),
            ("ID_APP_EXIT", Action::Exit),
            ("CMD_MEDIA_CLOSE", Action::CloseMedia),
            (
                "ID_PLAYBACK_5MIN_FORW",
                Action::SeekForward(SeekStep::VeryLong),
            ),
            ("ID_16_9_RATIO", Action::SetAspect(Aspect::R16x9)),
            ("ID_VIDEO_IMAGE_CAPTURE", Action::SaveFrame),
        ] {
            assert_eq!(
                Action::from_skin_command(command),
                Some(action),
                "{command}"
            );
        }
        assert_eq!(Action::from_skin_command("CMD_TIMEBAR_CTRL"), None);
    }
}
