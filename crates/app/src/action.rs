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
    /// The audio normalizer on or off (PotPlayer: Shift+N).
    ToggleNormalizer,
    /// The normalizer's "Boost audio" on or off.
    ToggleBoost,
    /// The key and mouse editor (F5, PotPlayer's Preferences, until the
    /// settings window of M5).
    KeysAndMouse,
    /// Import PotPlayer's playlists as albums.
    ImportPlaylists,
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
    (Action::ToggleNormalizer, "toggle-normalizer"),
    (Action::ToggleBoost, "toggle-boost"),
    (Action::KeysAndMouse, "keys-and-mouse"),
    (Action::ImportPlaylists, "import-playlists"),
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
            10009 => Action::ToggleNormalizer,
            10010 => Action::KeysAndMouse,
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
            Action::ToggleNormalizer => ("Normalizer", Some("Shift+N")),
            Action::ToggleBoost => ("Normalizer: Boost audio", None),
            Action::KeysAndMouse => ("Keys and Mouse...", Some("F5")),
            Action::ImportPlaylists => ("Import PotPlayer Playlists...", None),
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
                | Action::ToggleNormalizer
                | Action::ToggleBoost
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
