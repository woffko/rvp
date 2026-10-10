//! What the user can ask for, and how PotPlayer's command IDs map to it.
//!
//! Two kinds of PotPlayer IDs reach RVP: the symbolic `CmdID` of skin
//! controls (`CMD_MEDIA_PLAY`, `ID_PROCESS_NEXT`) and the numeric IDs of
//! menu commands that the default hotkeys trigger (`10014` = Play/Pause,
//! `docs/r1/hotkeys.tsv`). Both map to [`Action`]. M2 covers what the
//! classic window needs; M4 extends the coverage.

/// One of PotPlayer's four seek steps (Left/Right, with Ctrl, with Shift,
/// with Ctrl+Alt). Their lengths are settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeekStep {
    Short,
    Medium,
    Long,
    VeryLong,
}

/// A user action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Open files through the file dialog.
    OpenFiles,
    /// Stop and unload the current file (PotPlayer "Close Playback").
    CloseMedia,
    /// Quit RVP.
    Exit,
    PlayPause,
    Previous,
    Next,
    SeekBackward(SeekStep),
    SeekForward(SeekStep),
    VolumeUp,
    VolumeDown,
    ToggleMute,
    /// Fullscreen keeping the aspect ratio, or back to the window.
    ToggleFullscreen,
    /// Fullscreen with the video stretched to the screen.
    ToggleFullscreenStretch,
    Minimize,
    MaximizeRestore,
    ContextMenu,
    /// Show or hide the attached playlist panel.
    TogglePlaylist,
    /// Show or hide subtitles.
    ToggleSubtitles,
}

/// Every action with its stable name (settings files use the names).
const NAMES: &[(Action, &str)] = &[
    (Action::OpenFiles, "open-files"),
    (Action::CloseMedia, "close-media"),
    (Action::Exit, "exit"),
    (Action::PlayPause, "play-pause"),
    (Action::Previous, "previous"),
    (Action::Next, "next"),
    (Action::SeekBackward(SeekStep::Short), "seek-backward-short"),
    (Action::SeekForward(SeekStep::Short), "seek-forward-short"),
    (
        Action::SeekBackward(SeekStep::Medium),
        "seek-backward-medium",
    ),
    (Action::SeekForward(SeekStep::Medium), "seek-forward-medium"),
    (Action::SeekBackward(SeekStep::Long), "seek-backward-long"),
    (Action::SeekForward(SeekStep::Long), "seek-forward-long"),
    (
        Action::SeekBackward(SeekStep::VeryLong),
        "seek-backward-very-long",
    ),
    (
        Action::SeekForward(SeekStep::VeryLong),
        "seek-forward-very-long",
    ),
    (Action::VolumeUp, "volume-up"),
    (Action::VolumeDown, "volume-down"),
    (Action::ToggleMute, "toggle-mute"),
    (Action::ToggleFullscreen, "toggle-fullscreen"),
    (Action::ToggleFullscreenStretch, "toggle-fullscreen-stretch"),
    (Action::Minimize, "minimize"),
    (Action::MaximizeRestore, "maximize-restore"),
    (Action::ContextMenu, "context-menu"),
    (Action::TogglePlaylist, "toggle-playlist"),
    (Action::ToggleSubtitles, "toggle-subtitles"),
];

impl Action {
    /// All actions, in a stable order.
    pub fn all() -> impl Iterator<Item = Action> {
        NAMES.iter().map(|&(action, _)| action)
    }

    /// The stable name used in settings files.
    #[must_use]
    pub fn name(self) -> &'static str {
        NAMES
            .iter()
            .find(|&&(action, _)| action == self)
            .map_or("", |&(_, name)| name)
    }

    /// The action with this name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Action> {
        NAMES
            .iter()
            .find(|&&(_, candidate)| candidate == name)
            .map(|&(action, _)| action)
    }

    /// The action of a numeric PotPlayer command ID (menu commands and the
    /// default hotkeys).
    #[must_use]
    pub fn from_potplayer_id(id: u32) -> Option<Action> {
        Some(match id {
            10013 => Action::ToggleFullscreen,
            10308 => Action::ToggleFullscreenStretch,
            10014 => Action::PlayPause,
            10035 => Action::VolumeUp,
            10036 => Action::VolumeDown,
            10037 => Action::ToggleMute,
            10059 => Action::SeekBackward(SeekStep::Short),
            10060 => Action::SeekForward(SeekStep::Short),
            10061 => Action::SeekBackward(SeekStep::Medium),
            10062 => Action::SeekForward(SeekStep::Medium),
            10063 => Action::SeekBackward(SeekStep::Long),
            10064 => Action::SeekForward(SeekStep::Long),
            10065 => Action::SeekBackward(SeekStep::VeryLong),
            10066 => Action::SeekForward(SeekStep::VeryLong),
            // PgUp/PgDn: previous/next item (or chapter, channel: M4).
            10123 | 10067 => Action::Previous,
            10124 | 10068 => Action::Next,
            10443 | 10158 => Action::OpenFiles,
            10167 => Action::CloseMedia,
            10011 => Action::TogglePlaylist,
            10237 | 10238 => Action::MaximizeRestore,
            10126 => Action::ToggleSubtitles,
            57665 => Action::Exit,
            _ => return None,
        })
    }

    /// The action of a skin control's `CmdID` or `AltCmdID` (right click).
    /// Sliders and labels are not actions; the shell handles them directly.
    #[must_use]
    pub fn from_skin_command(command: &str) -> Option<Action> {
        Some(match command {
            "CMD_MEDIA_PLAY" | "CMD_MEDIA_PAUSE" => Action::PlayPause,
            "ID_PROCESS_PREV" | "CMD_MEDIA_PREV" => Action::Previous,
            "ID_PROCESS_NEXT" | "CMD_MEDIA_NEXT" => Action::Next,
            "CMD_MEDIA_STOP" => Action::CloseMedia,
            "ID_AUDIO_MUTE" => Action::ToggleMute,
            "CMD_MIN_WINDOW" => Action::Minimize,
            "CMD_MAX_WINDOW" | "CMD_RESTORE_WINDOW" => Action::MaximizeRestore,
            "ID_APP_EXIT" => Action::Exit,
            "CMD_FULLSCREEN" | "ID_VIEW_FULLSCREEN" => Action::ToggleFullscreen,
            "CMD_MEDIA_OPEN" | "ID_FILE_OPEN" | "ID_OPEN_FILE" => Action::OpenFiles,
            "ID_PLAYBACK_30SEC_BACK" => Action::SeekBackward(SeekStep::Medium),
            "ID_PLAYBACK_30SEC_FORW" => Action::SeekForward(SeekStep::Medium),
            "ID_PLAYLIST_WINDOW"
            | "CMD_ATTACH_WINDOW_OPEN"
            | "CMD_ATTACH_WINDOW_CLOSE"
            | "CMD_PLAYLIST_WINDOW_OPEN"
            | "CMD_PLAYLIST_WINDOW_CLOSE" => Action::TogglePlaylist,
            "ID_CAPTION_VISIBLE" => Action::ToggleSubtitles,
            _ => return None,
        })
    }

    /// PotPlayer's menu text (`docs/r1/menu-static.tsv`) and the shortcut
    /// its menu shows; `%s` stands for the seek step's length.
    #[must_use]
    pub fn menu_text(self) -> (&'static str, Option<&'static str>) {
        match self {
            Action::OpenFiles => ("Open File(s)...", Some("F3")),
            Action::CloseMedia => ("Close Playback", Some("F4")),
            Action::Exit => ("Exit", Some("Alt+F4")),
            Action::PlayPause => ("Play/Pause Playback", Some("Space")),
            Action::Previous => ("Play Previous Item", Some("PgUp")),
            Action::Next => ("Play Next Item", Some("PgDn")),
            Action::SeekBackward(SeekStep::Short) => ("%s Backward", Some("Left")),
            Action::SeekForward(SeekStep::Short) => ("%s Forward", Some("Right")),
            Action::SeekBackward(SeekStep::Medium) => ("%s Backward", Some("Ctrl+Left")),
            Action::SeekForward(SeekStep::Medium) => ("%s Forward", Some("Ctrl+Right")),
            Action::SeekBackward(SeekStep::Long) => ("%s Backward", Some("Shift+Left")),
            Action::SeekForward(SeekStep::Long) => ("%s Forward", Some("Shift+Right")),
            Action::SeekBackward(SeekStep::VeryLong) => ("%s Backward", Some("Ctrl+Alt+Left")),
            Action::SeekForward(SeekStep::VeryLong) => ("%s Forward", Some("Ctrl+Alt+Right")),
            Action::VolumeUp => ("Increase Volume", Some("Up")),
            Action::VolumeDown => ("Decrease Volume", Some("Down")),
            Action::ToggleMute => ("Mute", Some("M")),
            Action::ToggleFullscreen => ("Fullscreen (Keep AR)", Some("Alt+Enter")),
            Action::ToggleFullscreenStretch => ("Fullscreen (Stretch)", Some("Ctrl+Enter")),
            Action::Minimize => ("Minimize (to Taskbar/Tray)", None),
            Action::MaximizeRestore => ("Maximize (to Desktop) / Restore", Some("6")),
            Action::ContextMenu => ("Menu", None),
            Action::TogglePlaylist => ("Playlist...", Some("F6")),
            Action::ToggleSubtitles => ("Show/Hide Subtitles", Some("Alt+H")),
        }
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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for action in Action::all() {
            let name = action.name();
            assert!(!name.is_empty(), "{action:?} has no name");
            assert!(seen.insert(name), "duplicate name {name}");
            assert_eq!(Action::from_name(name), Some(action));
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
        assert_eq!(Action::from_potplayer_id(1), None);
    }

    #[test]
    fn classic_window_buttons() {
        for (command, action) in [
            ("CMD_MEDIA_PLAY", Action::PlayPause),
            ("ID_PROCESS_PREV", Action::Previous),
            ("ID_PROCESS_NEXT", Action::Next),
            ("ID_AUDIO_MUTE", Action::ToggleMute),
            ("CMD_MIN_WINDOW", Action::Minimize),
            ("CMD_MAX_WINDOW", Action::MaximizeRestore),
            ("ID_APP_EXIT", Action::Exit),
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
