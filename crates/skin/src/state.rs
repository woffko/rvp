//! What the skin shows: the interface state (window, playback, hover) and
//! the data model (texts, slider values, playlist).

use std::collections::{BTreeMap, BTreeSet};

/// Interface state that changes which elements are visible and how they look.
// Independent flags that mirror PotPlayer's window and playback state.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    /// Inactive windows use the `AltImage*` sprites.
    pub window_active: bool,
    pub maximized: bool,
    /// The window is fullscreen (`State='CMD_ON_FULLSCREEN'`).
    pub fullscreen: bool,
    pub media_loaded: bool,
    /// True while playing; selects the Pause button over the Play button.
    pub playing: bool,
    /// The playlist is attached to the main window and shown.
    pub playlist_attached: bool,
    /// Page shown by `State='CMD_PAGE_CONTROL'` statics (`PageIndex`).
    pub current_page: i32,
    /// Active state names (`CMD_BROADCAST_MODE`, ...) and toggled commands of
    /// `DownFlag` buttons (`ID_AUDIO_EQULIZER`, ...).
    pub flags: BTreeSet<String>,
    /// Command values for `AutoVisible` ([`UiState::auto_value`]): 1 for a
    /// message that has a text, the Always On Top mode's number (1 to 4)
    /// for `ID_ETC_NEXTONTOP`, 1 or -1 for a command that is on or off.
    pub values: BTreeMap<String, i32>,
    /// Element under the mouse pointer.
    pub hover: Option<usize>,
    /// Element held down by the mouse.
    pub pressed: Option<usize>,
    /// Code page for multi-language texts; 0 selects the default (English).
    pub code_page: u32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            window_active: true,
            maximized: false,
            fullscreen: false,
            media_loaded: false,
            playing: false,
            playlist_attached: true,
            current_page: 0,
            flags: BTreeSet::new(),
            values: BTreeMap::new(),
            hover: None,
            pressed: None,
            code_page: 0,
        }
    }
}

impl UiState {
    /// True when `name` is an active state or a toggled command. The
    /// window's and playback's states have names of their own: a file is
    /// open (`CMD_ON_MEDIA_PLAY`, which twelve of PotPlayer's skins give
    /// their video slot), fullscreen, the playlist attached; RVP has none
    /// of Daum's broadcast or VTP modes.
    #[must_use]
    pub fn flag(&self, name: &str) -> bool {
        match name {
            "CMD_ON_MEDIA_PLAY" => self.media_loaded,
            "CMD_ON_FULLSCREEN" => self.fullscreen,
            "CMD_PLAYLIST_ATTACH" => self.playlist_attached,
            "CMD_NOT_VTP_MODE" | "CMD_NOT_BV_MODE" => true,
            _ => self.flags.contains(name),
        }
    }

    /// The value an element's `AutoVisible` is compared with (Default.dsf,
    /// WindowFrame): the command's entry in [`UiState::values`], else 1
    /// while it is on and -1 while off. The element shows while they are
    /// equal: a message's parts while it has a text (`1`), the pin of the
    /// Always On Top mode (`1` to `4`), the on and off pictures (`1`,
    /// `-1`); values RVP never gives (HDR's `10`) hide it.
    #[must_use]
    pub fn auto_value(&self, command: &str) -> i32 {
        self.values
            .get(command)
            .copied()
            .unwrap_or(if self.flag(command) { 1 } else { -1 })
    }

    /// Whether PotPlayer shows a control bound to `command` in this state.
    /// Paired controls share a position and swap: Play/Pause,
    /// Maximize/Restore, playlist open/close.
    #[must_use]
    pub fn command_visible(&self, command: &str) -> bool {
        match command {
            "CMD_MEDIA_PLAY" => !self.playing,
            "CMD_MEDIA_PAUSE" => self.playing,
            "CMD_MAX_WINDOW" => !self.maximized,
            "CMD_RESTORE_WINDOW" => self.maximized,
            "CMD_PLAYLIST_WINDOW_OPEN" | "CMD_ATTACH_WINDOW_OPEN" => !self.playlist_attached,
            "CMD_PLAYLIST_WINDOW_CLOSE" | "CMD_ATTACH_WINDOW_CLOSE" => self.playlist_attached,
            // Daum's pop-up windows, as their slots below.
            "CMD_POPUP_WINDOW" => false,
            _ => true,
        }
    }

    /// Whether a `Contain='...'` slot is shown.
    #[must_use]
    pub fn slot_visible(&self, slot: &str) -> bool {
        match slot {
            "CMD_VIDEO_WINDOW" | "CMD_PLAYLIST_HOLDER" | "CMD_PLAY_LIST_ALBUM_TAB" => true,
            "CMD_PLAYLIST_TOGGLE" | "CMD_CHAT_PLAYLIST_BROADCAST_TOGGLE" => self.playlist_attached,
            // Broadcast pop-ups and Flash controls belong to closed Daum services.
            "CMD_POPUP_WINDOW" | "CMD_FLASH_CONTROL" => false,
            other => self.flag(other),
        }
    }
}

/// Value and availability of a slider (`CMD_TIMEBAR_CTRL`, `CMD_VOLUME_CTRL`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderValue {
    /// Position from 0.0 to 1.0.
    pub value: f32,
    /// Disabled sliders draw their `AltImage` track (no media loaded).
    pub enabled: bool,
}

/// What a mark on the seek bar stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    Chapter,
    Bookmark,
    /// The A-B repeat's points.
    PointA,
    PointB,
}

/// A mark on the seek bar (`CMD_TIMEBAR_CTRL`) at a fraction of the
/// duration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub at: f32,
    pub kind: MarkKind,
}

/// One playlist row.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistEntry {
    pub title: String,
    /// Duration in seconds, when known.
    pub duration: Option<u32>,
    /// False when the file could not be opened (`<Can't open>`).
    pub playable: bool,
    pub played: bool,
    /// The atlas image of the entry's thumbnail (thumbnail views), once
    /// it is there.
    pub thumbnail: Option<String>,
    /// A line of details under the title (thumbnail views).
    pub details: Option<String>,
}

/// The playlist shown in the attached panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Playlist {
    /// Album (tab) names; the selected one is `current_album`.
    pub albums: Vec<String>,
    pub current_album: usize,
    pub entries: Vec<PlaylistEntry>,
    /// Index of the entry being played.
    pub current: Option<usize>,
    /// Selected entries.
    pub selected: BTreeSet<usize>,
    /// Scroll position of the list.
    pub scroll: ListScroll,
    /// While rows are dragged: the entry they will be inserted before
    /// (`entries.len()` for the end).
    pub drop_before: Option<usize>,
    /// Lines of text a row takes: 1 (or 0) for the list, 2 to 7 for
    /// thumbnails with details (PotPlayer's View as).
    pub row_lines: u32,
    /// Thumbnail boxes are 16:9 (otherwise 4:3).
    pub wide_thumbnails: bool,
}

/// Scroll position of the playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListScroll {
    /// The given entry is the first row.
    Entry(usize),
    /// The last entry ends at the bottom edge; the top row may be cut, as
    /// PotPlayer shows a list scrolled to its end.
    #[default]
    End,
}

/// Everything the skin displays besides the video itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DataModel {
    /// Text per command ID, for `Label`, `PicLabel` and `CMD_PLAY_FILE_TYPE`.
    /// `MappingDB` labels compare their entries with this text.
    pub texts: BTreeMap<String, String>,
    pub sliders: BTreeMap<String, SliderValue>,
    pub playlist: Playlist,
    /// Marks on the seek bar: chapters, bookmarks and the A-B points.
    pub marks: Vec<Mark>,
    /// The audio's spectrum for `Visual` elements: each band's level from
    /// 0 to 1, low to high frequencies (empty: no bar lit), and the peaks
    /// the bars fall from.
    pub visual: Vec<f32>,
    pub visual_peaks: Vec<f32>,
}

impl DataModel {
    /// The value 1 for each command whose text is not empty: what
    /// `AutoVisible='1'` asks of a message ([`UiState::values`]).
    #[must_use]
    pub fn text_values(&self) -> BTreeMap<String, i32> {
        self.texts
            .iter()
            .filter(|(_, text)| !text.is_empty())
            .map(|(command, _)| (command.clone(), 1))
            .collect()
    }

    /// Text bound to a command, if any.
    #[must_use]
    pub fn text(&self, command: &str) -> Option<&str> {
        self.texts.get(command).map(String::as_str)
    }

    /// Sets the text of a command.
    pub fn set_text(&mut self, command: &str, text: impl Into<String>) {
        self.texts.insert(command.to_owned(), text.into());
    }

    /// Generic sample content for tests and previews: a two-hour film paused
    /// at the start, and a short playlist.
    #[must_use]
    pub fn sample() -> Self {
        let mut data = Self::default();
        for (command, text) in [
            ("CMD_TITLE_MESSAGE", "[12/12] Sample Film (2024) 1080p.mkv"),
            ("CMD_CURRENTTIME_MESSAGE", "00:00:00"),
            ("CMD_TOTALTIME_MESSAGE", "02:08:49"),
            ("CMD_LEFTTIME_MESSAGE", "02:08:49"),
            ("CMD_SYSTEM_TIME_MESSAGE", "21:30:00"),
            ("CMD_PLAY_FILE_TYPE", "MKV"),
            ("CMD_VIDEO_OUT_FOURCC_MESSAGE", "NV12"),
            ("CMD_VIDEO_BIT_RATE_MESSAGE", "5621"),
            ("CMD_AUDIO_CODEC_TAG_MESSAGE", "0x2000"),
            ("CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE", "5.1"),
            ("CMD_AUDIO_OUT_SAMPLE_RATE_MESSAGE", "48000"),
            ("CMD_AUDIO_BIT_RATE_MESSAGE", "448"),
            ("CMD_VIDEO_CODEC_FPS_MESSAGE", "24"),
            ("CMD_VIDEO_OUT_FPS_MESSAGE", "59.06"),
            ("CMD_VIDEO_CODEC_SIZE_MESSAGE", "1920 800"),
            ("CMD_PERCENTTIME_MESSAGE", "0"),
        ] {
            data.set_text(command, text);
        }
        data.sliders.insert(
            "CMD_TIMEBAR_CTRL".to_owned(),
            SliderValue {
                value: 0.0,
                enabled: false,
            },
        );
        data.sliders.insert(
            "CMD_VOLUME_CTRL".to_owned(),
            SliderValue {
                value: 0.22,
                enabled: true,
            },
        );
        let titles = [
            "Sample Film One.mkv",
            "Another Sample Film (2023) 720p WEB-DL.mkv",
            "Third Sample.avi",
            "A Rather Long Sample Film Title That Does Not Fit The Panel.mkv",
            "Missing File.mkv",
        ];
        for (index, title) in titles.iter().enumerate() {
            let index_u32 = u32::try_from(index).unwrap_or(0);
            data.playlist.entries.push(PlaylistEntry {
                title: (*title).to_owned(),
                duration: Some(5400 + 431 * index_u32),
                playable: index != 4,
                played: index == 0,
                thumbnail: None,
                details: None,
            });
        }
        data.playlist.albums = vec!["Default".to_owned(), "Films".to_owned()];
        data.playlist.current = Some(3);
        data.playlist.selected.insert(3);
        data
    }
}
