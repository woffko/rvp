//! The menus (M4): PotPlayer's context menu and playlist menu as R1
//! recorded them (`docs/r1/menu-static.tsv`, `docs/r1/menu-playlist.tsv`),
//! limited to what RVP has, and the skin's own menus (`<UserMenu>`, see
//! `rvp_skin::menu`).
//!
//! * An item stays when its command ID maps to an [`Action`] or a
//!   [`ListAction`]; a submenu stays when something in it stays; separators
//!   never start, end or double up. Labels are PotPlayer's, with the seek
//!   and sync steps filled in; the shortcut is the key bound now, the check
//!   mark RVP's state.
//! * Where PotPlayer shows run-time lists, RVP puts its own: the audio,
//!   video and subtitle tracks (in "Select Audio Stream", "Select Video
//!   Stream" and "Add/Select Subtitles"), the chapters ("Chapter/Bookmark"),
//!   the albums ("Album/Favorites"), the equalizer's presets ("Equalizer
//!   Presets"), the output speakers, formats and modes, and mpv's audio
//!   devices ("Audio Renderer", its pass-through and second device).
//!   "Filters" lists what plays the file, as PotPlayer lists its
//!   filter graph there: the source, the video decoder (hardware or
//!   software), the video filters, the video output, the audio decoder, the
//!   audio filters and the audio output, as lines without a command.
//! * A skin menu item names a command (`ID_OPEN_URL`), a built-in submenu
//!   (`CMD_POPUPMENU_PLAY_GOTO`, in place with `NoSubMenu='1'`) or nothing
//!   (a separator). [`popup_source`] says which part of PotPlayer's menu
//!   each `CMD_POPUPMENU_*` stands for. A skin menu RVP has nothing for
//!   shows one disabled line.
//! * Labels say RVP where PotPlayer's say PotPlayer.
//!
//! The shell gets menus as JSON ([`to_json`]) and sends back a command ID
//! ([`Command::id`]).

use std::sync::OnceLock;

use rvp_engine::{Chapter, Engine, PropertyId, Track, TrackKind};
use serde_json::{Value, json};

use crate::action::{Action, ListAction};
use crate::app::{App, UiRequest};
use crate::commands::track_label;

const CONTEXT_TSV: &str = include_str!("../../../docs/r1/menu-static.tsv");
const PLAYLIST_TSV: &str = include_str!("../../../docs/r1/menu-playlist.tsv");
const LIVE_TSV: &str = include_str!("../../../docs/r1/menu-runtime.tsv");

/// Submenus RVP builds from PotPlayer's live menu (`menu-runtime.tsv`),
/// which adds items at run time (M12: WEBP, the desktop sizes, the frame
/// presets, 360° Video Mode and more): the static submenu's path and the
/// titles that lead to the live one.
const LIVE_SUBMENUS: &[(&str, &[&str])] = &[
    ("101_0_11", &["Video"]),
    ("101_0_17", &["Frame Size"]),
    ("101_0_18", &["Aspect Ratio"]),
    ("101_0_19", &["Window Size"]),
];

/// Static items the live menu lacks, kept in front of a live submenu's
/// items: Select Video Stream (R1's live menu was recorded without a
/// file).
const KEPT_STATIC: &[&str] = &["101_0_11_0"];

/// The live Frame Size > Presets submenu: RVP lists its own presets there.
const LIVE_FRAME_PRESETS: u32 = 1_383_396;

/// The live Pixel Shaders > Pre-Resize and Post-Resize submenus: RVP lists
/// its shaders there, in place of PotPlayer's (37502 and 39802 on).
const LIVE_PRE_SHADERS: u32 = 204_269;
const LIVE_POST_SHADERS: u32 = 269_761;
/// The live Video > VapourSynth submenu: RVP's script slots in place of
/// PotPlayer's presets (38400 on).
const LIVE_VAPOURSYNTH: u32 = 531_020;

/// One item of a PotPlayer menu as R1 recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The resource path (`101_0_9_6_0`).
    pub path: String,
    /// The command ID; 0 for submenus and separators.
    pub id: u32,
    pub text: String,
    /// The shortcut PotPlayer's menu shows.
    pub shortcut: String,
    pub separator: bool,
    pub submenu: bool,
    pub children: Vec<Recorded>,
}

fn parse(tsv: &str) -> Vec<Recorded> {
    let mut root: Vec<Recorded> = Vec::new();
    for line in tsv.lines().skip(1) {
        let cells: Vec<&str> = line.split('\t').collect();
        let (Some(path), Some(depth), Some(id), Some(text)) = (
            cells.first(),
            cells.get(1).and_then(|d| d.parse::<usize>().ok()),
            cells.get(2).and_then(|i| i.parse::<u32>().ok()),
            cells.get(3),
        ) else {
            continue;
        };
        let text = text.trim();
        let item = Recorded {
            path: (*path).to_owned(),
            id,
            text: text.replace("&&", "&"),
            shortcut: cells.get(4).map_or("", |s| s.trim()).to_owned(),
            separator: text == "---",
            submenu: cells.get(5) == Some(&"yes"),
            children: Vec::new(),
        };
        insert(&mut root, depth, item);
    }
    root
}

/// Puts an item under the last open submenu `depth` levels down.
fn insert(level: &mut Vec<Recorded>, depth: usize, item: Recorded) {
    match level.last_mut() {
        Some(parent) if depth > 0 => insert(&mut parent.children, depth - 1, item),
        _ => level.push(item),
    }
}

/// PotPlayer's context menu (resource 101).
#[must_use]
pub fn context_tree() -> &'static [Recorded] {
    static TREE: OnceLock<Vec<Recorded>> = OnceLock::new();
    TREE.get_or_init(|| parse(CONTEXT_TSV))
}

/// PotPlayer's playlist menu (resource 111).
#[must_use]
pub fn playlist_tree() -> &'static [Recorded] {
    static TREE: OnceLock<Vec<Recorded>> = OnceLock::new();
    TREE.get_or_init(|| parse(PLAYLIST_TSV))
}

/// PotPlayer's live context menu (R1's `menu-runtime.tsv`): its items have
/// no resource paths, so each gets `live_` and its line; a submenu's ID is
/// PotPlayer's run-time handle.
#[must_use]
pub fn live_tree() -> &'static [Recorded] {
    static TREE: OnceLock<Vec<Recorded>> = OnceLock::new();
    TREE.get_or_init(|| {
        let mut root: Vec<Recorded> = Vec::new();
        for (line, text) in LIVE_TSV.lines().enumerate().skip(1) {
            let cells: Vec<&str> = text.split('\t').collect();
            let (Some(depth), Some(id), Some(title)) = (
                cells.first().and_then(|d| d.parse::<usize>().ok()),
                cells.get(1).and_then(|i| i.parse::<u32>().ok()),
                cells.get(2),
            ) else {
                continue;
            };
            let title = title.trim();
            let item = Recorded {
                path: format!("live_{line}"),
                id,
                text: title.replace("&&", "&"),
                shortcut: cells.get(3).map_or("", |s| s.trim()).to_owned(),
                separator: title == "---",
                submenu: cells.get(4).is_some_and(|flags| flags.contains("submenu")),
                children: Vec::new(),
            };
            insert(&mut root, depth, item);
        }
        root
    })
}

/// The live submenu that `titles` lead to.
fn find_live(titles: &[&str]) -> Option<&'static Recorded> {
    let mut level = live_tree();
    let mut found = None;
    for title in titles {
        let node = level
            .iter()
            .find(|node| node.submenu && node.text == *title)?;
        level = &node.children;
        found = Some(node);
    }
    found
}

/// The recorded item at a path.
#[must_use]
pub fn find<'a>(tree: &'a [Recorded], path: &str) -> Option<&'a Recorded> {
    tree.iter().find_map(|item| {
        if item.path == path {
            Some(item)
        } else if path.starts_with(&format!("{}_", item.path)) {
            find(&item.children, path)
        } else {
            None
        }
    })
}

/// Every recorded item, depth first.
pub fn walk(tree: &[Recorded], visit: &mut impl FnMut(&Recorded)) {
    for item in tree {
        visit(item);
        walk(&item.children, visit);
    }
}

/// What a built-in submenu command (`CMD_POPUPMENU_*`) of a skin stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSource {
    /// A submenu of PotPlayer's context menu, by path.
    Context(&'static str),
    /// A submenu of PotPlayer's live menu, by the titles leading to it.
    Live(&'static [&'static str]),
    /// A submenu of PotPlayer's playlist menu, by path.
    Playlist(&'static str),
    Tracks(TrackKind),
    Chapters,
    Albums,
    EqualizerPresets,
}

/// The part of PotPlayer's menus a skin's built-in submenu command shows;
/// `None` for the ones RVP has nothing for.
#[must_use]
pub fn popup_source(command: &str) -> Option<PopupSource> {
    Some(match command {
        "CMD_POPUPMENU_ETC" => PopupSource::Context("101_0_15"),
        // PotPlayer's keys J, K and S (M12): RVP's names.
        MenuPopup::THREE_D => PopupSource::Live(&["Video", "3D Video Mode"]),
        MenuPopup::CAPTURE => PopupSource::Live(&["Video", "Video Capture"]),
        MenuPopup::SHADERS => PopupSource::Live(&["Video", "Pixel Shaders"]),
        "CMD_POPUPMENU_PLAY_GOTO" => PopupSource::Context("101_0_9_6"),
        "CMD_POPUPMENU_PLAY_REPEAT" => PopupSource::Context("101_0_9_11"),
        "CMD_POPUPMENU_SIZE" => PopupSource::Context("101_0_19"),
        "CMD_POPUPMENU_FILTER" => PopupSource::Context("101_0_13"),
        "CMD_POPUPMENU_VIDEO_FILTER" => PopupSource::Context("101_0_11"),
        "CMD_POPUPMENU_VIDEO_SHADER" => PopupSource::Context("101_0_11_2"),
        "CMD_POPUPMENU_VIDEO_DEVICE" => PopupSource::Context("101_0_11_1"),
        "CMD_POPUPMENU_DVD" => PopupSource::Context("101_0_8"),
        "CMD_POPUPMENU_AUDIO_FILTER" => PopupSource::Context("101_0_12_18"),
        "CMD_POPUPMENU_CAPTION" => PopupSource::Context("101_0_10"),
        "CMD_POPUPMENU_CAPTION_POS" => PopupSource::Context("101_0_10_9"),
        "CMD_POPUPMENU_CAPTION_SYNC" => PopupSource::Context("101_0_10_10"),
        "ID_CAPTION_SELECT_MENU" => PopupSource::Context("101_0_10_0"),
        "CMD_POPUPMENU_CAPTION_LANG" => PopupSource::Tracks(TrackKind::Subtitle),
        "ID_AUDIO_SELECT_MENU" => PopupSource::Tracks(TrackKind::Audio),
        "CMD_POPUPMENU_PLAY_BOOKMARK" => PopupSource::Chapters,
        "CMD_POPUPMENU_FAVORITE" => PopupSource::Albums,
        "CMD_PLAYLIST_MENU_SORT" => PopupSource::Playlist("111_0_8"),
        "CMD_POPUPMENU_AUDIO_EQULIZER" | "CMD_POPUPMENU_EQULIZER_PRESET" => {
            PopupSource::EqualizerPresets
        }
        _ => return None,
    })
}

/// A submenu a key of PotPlayer's shows at the pointer (M12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuPopup {
    /// J: 3D Video Mode.
    ThreeD,
    /// K: Video Capture.
    Capture,
    /// S: Pixel Shaders.
    Shaders,
}

impl MenuPopup {
    const THREE_D: &'static str = "RVP_POPUP_3D";
    const CAPTURE: &'static str = "RVP_POPUP_CAPTURE";
    const SHADERS: &'static str = "RVP_POPUP_SHADERS";

    /// The popup command the window asks the menu for.
    #[must_use]
    pub fn command(self) -> &'static str {
        match self {
            MenuPopup::ThreeD => Self::THREE_D,
            MenuPopup::Capture => Self::CAPTURE,
            MenuPopup::Shaders => Self::SHADERS,
        }
    }
}

/// What a menu item does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Window(Action),
    List(ListAction),
    /// Select a track (`None`: subtitles off).
    Track(TrackKind, Option<i64>),
    /// Go to a chapter (index).
    Chapter(usize),
    /// Play an album (index).
    Album(usize),
    /// Choose an equalizer preset (index into `equalizer::PRESETS`).
    EqualizerPreset(usize),
    /// Choose the output speakers (index into `speakers::CHOICES`).
    Speakers(usize),
    /// An audio output choice.
    Audio(AudioChoice),
    /// Jump to a bookmark of the playing file (index).
    Bookmark(usize),
    /// Open a favorite (index).
    Favorite(usize),
    /// Open a recent item (index).
    Recent(usize),
    /// A shader of the pre-resize (`false`) or post-resize list's choices
    /// in or out of the list (index into `App::shader_choices`).
    Shader(bool, usize),
    /// A VapourSynth script slot to run (index).
    Script(usize),
    /// Play a disc's title (DVD Control > Move), by its index.
    DiscTitle(usize),
    /// Show a DVD angle (DVD Control > Angle Option).
    DiscAngle(u8),
}

/// An audio output choice of the Audio menu's lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChoice {
    /// The audio device (0: mpv's automatic choice, then [`App::audio_devices`]).
    Device(usize),
    /// The pass-through device (0: the audio device).
    PassthroughDevice(usize),
    /// The second device (0: none).
    SecondDevice(usize),
    /// An output format of [`FORMATS`].
    Format(usize),
    /// A sample rate of `audio::SAMPLE_RATES`.
    Rate(usize),
    /// A resampler quality of [`QUALITIES`].
    Resample(usize),
    /// An output mode of [`MODES`].
    Mode(usize),
    /// "Use Pass-through output when at least 1 chs exist" on or off.
    PassthroughAny,
}

/// Output Format's choices with PotPlayer's texts.
pub const FORMATS: [(crate::audio::SampleFormat, &str); 6] = [
    (crate::audio::SampleFormat::S16, "16-bit (Recommended)"),
    (crate::audio::SampleFormat::S24, "24-bit"),
    (crate::audio::SampleFormat::S32, "32-bit"),
    (crate::audio::SampleFormat::Float, "32-bit (floating point)"),
    (crate::audio::SampleFormat::S16Dither, "16-bit (dithering)"),
    (
        crate::audio::SampleFormat::Same,
        "As the output device takes it",
    ),
];

/// The resampler's qualities with PotPlayer's texts.
pub const QUALITIES: [(crate::audio::ResampleQuality, &str); 3] = [
    (crate::audio::ResampleQuality::High, "High quality"),
    (
        crate::audio::ResampleQuality::Normal,
        "Normal quality (Recommended)",
    ),
    (crate::audio::ResampleQuality::Low, "Low quality"),
];

/// Output Mode's choices with PotPlayer's texts.
pub const MODES: [(crate::audio::OutputMode, &str); 3] = [
    (crate::audio::OutputMode::Pcm, "PCM (Default) (Recommended)"),
    (crate::audio::OutputMode::Passthrough, "Pass-through"),
    (
        crate::audio::OutputMode::Ac3Encode,
        "Pass-through after AC3 re-encoding",
    ),
];

/// An audio device mpv knows: its name and its description.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Device {
    pub name: String,
    pub description: String,
}

impl AudioChoice {
    fn id(self) -> String {
        let (kind, index) = match self {
            AudioChoice::Device(i) => ("d", i),
            AudioChoice::PassthroughDevice(i) => ("p", i),
            AudioChoice::SecondDevice(i) => ("s", i),
            AudioChoice::Format(i) => ("f", i),
            AudioChoice::Rate(i) => ("r", i),
            AudioChoice::Resample(i) => ("q", i),
            AudioChoice::Mode(i) => ("m", i),
            AudioChoice::PassthroughAny => ("a", 0),
        };
        format!("{kind}{index}")
    }

    fn parse(text: &str) -> Option<Self> {
        let (kind, index) = text.split_at_checked(1)?;
        let index: usize = index.parse().ok()?;
        Some(match kind {
            "d" => AudioChoice::Device(index),
            "p" => AudioChoice::PassthroughDevice(index),
            "s" => AudioChoice::SecondDevice(index),
            "f" => AudioChoice::Format(index),
            "r" => AudioChoice::Rate(index),
            "q" => AudioChoice::Resample(index),
            "m" => AudioChoice::Mode(index),
            "a" => AudioChoice::PassthroughAny,
            _ => return None,
        })
    }
}

impl Command {
    /// The ID the shell sends back: `w:play-pause`, `l:select-all`,
    /// `a:2`, `s:no`, `c:3`, `b:1`.
    #[must_use]
    pub fn id(self) -> String {
        match self {
            Command::Window(action) => format!("w:{}", action.name()),
            Command::List(action) => format!("l:{}", action.name()),
            Command::Track(kind, id) => {
                let prefix = match kind {
                    TrackKind::Audio => 'a',
                    TrackKind::Video => 'v',
                    TrackKind::Subtitle => 's',
                };
                format!(
                    "{prefix}:{}",
                    id.map_or_else(|| "no".to_owned(), |id| id.to_string())
                )
            }
            Command::Chapter(index) => format!("c:{index}"),
            Command::Album(index) => format!("b:{index}"),
            Command::EqualizerPreset(index) => format!("e:{index}"),
            Command::Speakers(index) => format!("o:{index}"),
            Command::Audio(choice) => format!("x:{}", choice.id()),
            Command::Bookmark(index) => format!("k:{index}"),
            Command::Favorite(index) => format!("f:{index}"),
            Command::Recent(index) => format!("r:{index}"),
            Command::Shader(post, index) => {
                format!("h:{}{index}", if post { "post" } else { "pre" })
            }
            Command::Script(index) => format!("y:{index}"),
            Command::DiscTitle(title) => format!("t:{title}"),
            Command::DiscAngle(angle) => format!("g:{angle}"),
        }
    }

    #[must_use]
    pub fn parse(id: &str) -> Option<Command> {
        let (kind, value) = id.split_once(':')?;
        let track = |kind| {
            Some(Command::Track(
                kind,
                if value == "no" {
                    None
                } else {
                    Some(value.parse().ok()?)
                },
            ))
        };
        match kind {
            "w" => Action::from_name(value).map(Command::Window),
            "l" => ListAction::from_name(value).map(Command::List),
            "a" => track(TrackKind::Audio),
            "v" => track(TrackKind::Video),
            "s" => track(TrackKind::Subtitle),
            "c" => value.parse().ok().map(Command::Chapter),
            "b" => value.parse().ok().map(Command::Album),
            "e" => value.parse().ok().map(Command::EqualizerPreset),
            "o" => value.parse().ok().map(Command::Speakers),
            "x" => AudioChoice::parse(value).map(Command::Audio),
            "k" => value.parse().ok().map(Command::Bookmark),
            "f" => value.parse().ok().map(Command::Favorite),
            "r" => value.parse().ok().map(Command::Recent),
            "y" => value.parse().ok().map(Command::Script),
            "h" => {
                if let Some(index) = value.strip_prefix("post") {
                    index.parse().ok().map(|index| Command::Shader(true, index))
                } else {
                    let index = value.strip_prefix("pre")?;
                    index
                        .parse()
                        .ok()
                        .map(|index| Command::Shader(false, index))
                }
            }
            "t" => value.parse().ok().map(Command::DiscTitle),
            "g" => value.parse().ok().map(Command::DiscAngle),
            _ => None,
        }
    }
}

/// One entry of a menu RVP shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: String,
    pub command: Option<Command>,
    /// `Some` for items with a check mark.
    pub checked: Option<bool>,
    pub separator: bool,
    /// A submenu when not empty.
    pub children: Vec<MenuItem>,
}

impl MenuItem {
    fn separator() -> Self {
        Self {
            separator: true,
            ..Self::default()
        }
    }

    fn submenu(label: &str, children: Vec<MenuItem>) -> Option<Self> {
        let children = tidy(children);
        (!children.is_empty()).then(|| Self {
            label: label.to_owned(),
            children,
            ..Self::default()
        })
    }
}

/// The filters of an mpv `af` or `vf` value: label and filter name, in
/// order.
fn chain_entries(text: &str) -> Vec<(Option<&str>, &str)> {
    crate::audio::chain::split(text)
        .into_iter()
        .filter_map(|(label, entry)| {
            let rest = label.map_or(entry, |label| &entry[label.len() + 2..]);
            let name = rest.split(['=', ':']).next().unwrap_or("");
            (!name.is_empty()).then_some((label, name))
        })
        .collect()
}

/// What a filter of RVP's is, or mpv's name of another one.
fn filter_text(label: Option<&str>, name: &str) -> String {
    match label {
        Some(crate::equalizer::LABEL) => "Equalizer".to_owned(),
        Some(crate::normalizer::LABEL) => "Normalizer (TomSteady)".to_owned(),
        Some(crate::normalizer::TEMPO_LABEL) => format!("Tempo ({name})"),
        Some(crate::speakers::LABEL) => "Speakers' mix".to_owned(),
        Some(crate::audio::chain::CHANNEL) => "Channels".to_owned(),
        Some(crate::audio::chain::PREAMP) => "Pre-amplifier (AC3/DTS)".to_owned(),
        Some(crate::audio::chain::VOICE) => "Voice removal/emphasis".to_owned(),
        Some(crate::audio::chain::CRYSTALITY) => "Crystality".to_owned(),
        Some(crate::audio::chain::DENOISE) => "Noise reduction".to_owned(),
        Some(crate::audio::chain::REVERB) => "Freeverb".to_owned(),
        Some(crate::audio::chain::CROSSFEED) => "Crossfeed".to_owned(),
        Some(crate::audio::chain::GAINS) => "Individual volume".to_owned(),
        Some(crate::audio::chain::AC3) => "AC3 encoder (pass-through)".to_owned(),
        Some(crate::video::SCRIPT_LABEL) => "VapourSynth script".to_owned(),
        Some("svp") => "SVP".to_owned(),
        Some(label) => crate::video::VideoEffect::ALL
            .iter()
            .find(|effect| effect.label() == label)
            .map_or_else(
                || format!("{name} (@{label})"),
                |effect| effect.text().to_owned(),
            ),
        None => name.to_owned(),
    }
}

/// Drops separators at the start and end and doubled ones.
fn tidy(items: Vec<MenuItem>) -> Vec<MenuItem> {
    let mut out: Vec<MenuItem> = Vec::with_capacity(items.len());
    for item in items {
        if item.separator && out.last().is_none_or(|last| last.separator) {
            continue;
        }
        out.push(item);
    }
    while out.last().is_some_and(|last| last.separator) {
        out.pop();
    }
    out
}

/// A menu as JSON for the shell: `[{"label","shortcut","id","checked",
/// "separator","children"}]`.
#[must_use]
pub fn to_json(items: &[MenuItem]) -> String {
    fn value(items: &[MenuItem]) -> Value {
        Value::Array(
            items
                .iter()
                .map(|item| {
                    json!({
                        "label": item.label,
                        "shortcut": item.shortcut,
                        "id": item.command.map(Command::id).unwrap_or_default(),
                        "checked": item.checked.map_or(-1, i32::from),
                        "separator": item.separator,
                        "children": value(&item.children),
                    })
                })
                .collect(),
        )
    }
    value(items).to_string()
}

/// What RVP knows about the playing file and its albums when a menu opens
/// (read by the shell at that moment).
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub tracks: Vec<Track>,
    pub chapters: Vec<Chapter>,
    pub chapter: Option<i64>,
}

impl App {
    /// PotPlayer's context menu, limited to what RVP has.
    #[must_use]
    pub fn context_menu(&self, live: &Live) -> Vec<MenuItem> {
        tidy(
            context_tree()
                .iter()
                .filter_map(|node| self.convert(node, live))
                .collect(),
        )
    }

    /// PotPlayer's playlist menu, limited to what RVP has, plus RVP's undo,
    /// redo and import.
    #[must_use]
    pub fn playlist_menu(&self, live: &Live) -> Vec<MenuItem> {
        let mut items: Vec<MenuItem> = playlist_tree()
            .iter()
            .filter_map(|node| self.convert(node, live))
            .collect();
        items.push(MenuItem::separator());
        for action in [ListAction::Undo, ListAction::Redo] {
            items.push(self.list_item(action, action.label()));
        }
        items.push(self.window_item(Action::ImportPlaylists, None));
        tidy(items)
    }

    /// A skin menu (`<UserMenu>`); one disabled line when RVP has nothing
    /// for it, so that the button still answers.
    #[must_use]
    pub fn skin_menu(&self, menu: &rvp_skin::menu::UserMenu, live: &Live) -> Vec<MenuItem> {
        let mut items = Vec::new();
        for item in &menu.items {
            if item.command.is_empty() {
                items.push(MenuItem::separator());
            } else if let Some((title, children)) = self.popup(&item.command, live) {
                if item.inline {
                    items.extend(children);
                } else {
                    items.extend(MenuItem::submenu(&title, children));
                }
            } else if let Some(action) = Action::from_skin_command(&item.command) {
                items.push(self.window_item(action, None));
            } else if let Some(action) = ListAction::from_skin_command(&item.command) {
                items.push(self.list_item(action, action.label()));
            }
        }
        let items = tidy(items);
        if items.is_empty() {
            return vec![MenuItem {
                label: "Not in RVP yet".to_owned(),
                ..MenuItem::default()
            }];
        }
        items
    }

    /// The menu a skin's built-in submenu command shows, with its title;
    /// `None` when RVP has nothing for it.
    #[must_use]
    pub fn popup(&self, command: &str, live: &Live) -> Option<(String, Vec<MenuItem>)> {
        let (title, items) = match popup_source(command)? {
            PopupSource::Context(path) | PopupSource::Playlist(path) => {
                let tree = if path.starts_with("111") {
                    playlist_tree()
                } else {
                    context_tree()
                };
                let node = find(tree, path)?;
                let items = self.convert(node, live)?.children;
                (node.text.clone(), items)
            }
            PopupSource::Live(titles) => {
                let node = find_live(titles)?;
                let items = self.convert(node, live)?.children;
                (node.text.clone(), items)
            }
            PopupSource::Tracks(kind) => ("Tracks".to_owned(), self.tracks(live, kind)),
            PopupSource::Chapters => ("Chapter/Bookmark".to_owned(), self.chapter_menu(live)),
            PopupSource::Albums => ("Albums".to_owned(), self.albums()),
            PopupSource::EqualizerPresets => {
                ("Equalizer Presets".to_owned(), self.equalizer_presets())
            }
        };
        let items = tidy(items);
        (!items.is_empty()).then_some((title, items))
    }

    /// Carries out a menu command.
    pub fn run_menu_command(
        &mut self,
        engine: &Engine,
        command: Command,
        visible_rows: usize,
    ) -> Option<UiRequest> {
        match command {
            Command::Window(action) => self.dispatch(engine, action),
            Command::List(action) => self.list_action(engine, action, false, visible_rows),
            Command::Track(kind, id) => {
                self.select_track(engine, kind, id);
                None
            }
            Command::Chapter(index) => {
                drop(engine.set_text("chapter", &index.to_string()));
                None
            }
            Command::Album(index) => {
                if self.settings.playlist.instant_albums {
                    self.play_album(engine, index);
                } else {
                    self.library_mut().show_album(index);
                }
                None
            }
            Command::Bookmark(index) => self.jump_to_bookmark(engine, index),
            Command::Favorite(index) => {
                let favorite = self.marks().marks().favorites.get(index)?.clone();
                self.open_item(engine, &favorite.path, favorite.folder)
            }
            Command::Recent(index) => {
                let path = self.marks().marks().recent.get(index)?.clone();
                self.open_item(engine, &path, false)
            }
            Command::Script(index) => self.pick_script(engine, index),
            Command::Shader(post, index) => {
                let stage = if post {
                    crate::hlsl::Stage::Post
                } else {
                    crate::hlsl::Stage::Pre
                };
                self.pick_shader(engine, stage, index)
            }
            Command::DiscTitle(index) => self.play_disc_title(engine, index),
            Command::DiscAngle(angle) => self.set_disc_angle(engine, angle),
            Command::EqualizerPreset(index) => self.set_equalizer_preset(engine, index),
            Command::Speakers(index) => self.set_speakers(engine, index),
            Command::Audio(choice) => self.audio_choice(engine, choice),
        }
    }

    fn convert(&self, node: &Recorded, live: &Live) -> Option<MenuItem> {
        if node.separator {
            return Some(MenuItem::separator());
        }
        if node.submenu {
            if node.id == LIVE_FRAME_PRESETS {
                return MenuItem::submenu(&node.text, self.frame_preset_menu(node, live));
            }
            if node.id == LIVE_VAPOURSYNTH {
                return MenuItem::submenu(&node.text, self.script_menu(node, live));
            }
            if node.id == LIVE_PRE_SHADERS || node.id == LIVE_POST_SHADERS {
                let post = node.id == LIVE_POST_SHADERS;
                return MenuItem::submenu(&node.text, self.shader_menu(node, live, post));
            }
            let live_children = LIVE_SUBMENUS
                .iter()
                .find(|(path, _)| *path == node.path)
                .and_then(|(_, titles)| find_live(titles));
            let sources: Vec<&Recorded> = match live_children {
                Some(live_node) => node
                    .children
                    .iter()
                    .filter(|child| KEPT_STATIC.contains(&child.path.as_str()))
                    .chain(&live_node.children)
                    .collect(),
                None => node.children.iter().collect(),
            };
            let mut children: Vec<MenuItem> = sources
                .into_iter()
                .filter_map(|child| self.convert(child, live))
                .collect();
            // RVP's run-time lists where PotPlayer has its own.
            let extra = match node.path.as_str() {
                "101_0_12_0" => self.tracks(live, TrackKind::Audio),
                "101_0_12_1" => self.device_choices(AudioChoice::Device),
                "101_0_12_2" => self.device_choices(AudioChoice::PassthroughDevice),
                "101_0_12_3" => self.device_choices(AudioChoice::SecondDevice),
                "101_0_11_0" => self.tracks(live, TrackKind::Video),
                "101_0_10_0" => self.tracks(live, TrackKind::Subtitle),
                "101_0_3" => self.favorites_menu(),
                "101_0_13" => self.pipeline(live),
                _ => Vec::new(),
            };
            if !extra.is_empty() {
                children.push(MenuItem::separator());
                children.extend(extra);
            }
            return MenuItem::submenu(&node.text, children);
        }
        // "Chapter/Bookmark": the file's chapters and bookmarks, then the
        // bookmark commands (H opens the same list).
        if node.id == 10324 {
            return MenuItem::submenu(&node.text, self.chapter_menu(live));
        }
        let text = node.text.replace("PotPlayer", "RVP");
        if let Some(action) = Action::from_potplayer_id(node.id) {
            return Some(self.window_item(action, Some(&text)));
        }
        if let Some(action) = ListAction::from_potplayer_id(node.id) {
            return Some(self.list_item(action, &text));
        }
        // DVD Control's run-time lists.
        match node.id {
            20001 => return MenuItem::submenu(&node.text, self.disc_title_items()),
            20004 => return MenuItem::submenu(&node.text, self.disc_angle_items()),
            20002 => return MenuItem::submenu(&node.text, self.tracks(live, TrackKind::Audio)),
            20003 => {
                return MenuItem::submenu(&node.text, self.tracks(live, TrackKind::Subtitle));
            }
            _ => {}
        }
        // "Equalizer Presets" lists RVP's presets.
        if node.id == 20008 {
            return MenuItem::submenu(&node.text, self.equalizer_presets());
        }
        // "Output Speakers" lists the speakers RVP can choose.
        if node.id == 20005 {
            return MenuItem::submenu(&node.text, self.speaker_choices());
        }
        if node.id == 20006 {
            return MenuItem::submenu(&node.text, self.format_choices());
        }
        if node.id == 20007 {
            return MenuItem::submenu(&node.text, self.mode_choices());
        }
        None
    }

    /// A Pixel Shaders list: "Do not use", "Enable Shader Stacker" and
    /// "Stack..." as PotPlayer has them, with RVP's shaders between them;
    /// a shader's check mark says it is in the list.
    fn shader_menu(&self, node: &Recorded, live: &Live, post: bool) -> Vec<MenuItem> {
        let stage = if post {
            crate::hlsl::Stage::Post
        } else {
            crate::hlsl::Stage::Pre
        };
        let list = if post { 39_802..39_900 } else { 37_502..37_600 };
        let mut items = Vec::new();
        let mut listed = false;
        for child in &node.children {
            if list.contains(&child.id) {
                if !listed {
                    listed = true;
                    for (index, entry) in self.shader_choices(stage).into_iter().enumerate() {
                        items.push(MenuItem {
                            label: entry.name.clone(),
                            command: Some(Command::Shader(post, index)),
                            checked: Some(self.shader_picked(stage, &entry.path)),
                            ..MenuItem::default()
                        });
                    }
                }
                continue;
            }
            items.extend(self.convert(child, live));
        }
        items
    }

    /// Video > VapourSynth: its commands, then RVP's script slots (the one
    /// that runs checked).
    fn script_menu(&self, node: &Recorded, live: &Live) -> Vec<MenuItem> {
        let mut items: Vec<MenuItem> = node
            .children
            .iter()
            .filter(|child| !(38_400..38_500).contains(&child.id))
            .filter_map(|child| self.convert(child, live))
            .collect();
        let vs = &self.settings.vapoursynth;
        for (index, slot) in vs.slots.iter().enumerate() {
            items.push(MenuItem {
                label: crate::view::file_name(&slot.to_string_lossy()).to_owned(),
                command: Some(Command::Script(index)),
                checked: Some(vs.enabled && vs.script.as_ref() == Some(slot)),
                ..MenuItem::default()
            });
        }
        items
    }

    /// Frame Size > Presets: Add/Edit and the cycle as PotPlayer has them,
    /// then RVP's presets.
    fn frame_preset_menu(&self, node: &Recorded, live: &Live) -> Vec<MenuItem> {
        let mut items: Vec<MenuItem> = node
            .children
            .iter()
            .filter(|child| !(37_400..37_499).contains(&child.id))
            .filter_map(|child| self.convert(child, live))
            .collect();
        for index in 0..self.settings.picture.presets.len() {
            let action = Action::Picture(crate::picture::PictureCommand::Preset(index));
            if Action::from_name(action.name()) == Some(action) {
                items.push(self.window_item(action, Some("")));
            }
        }
        items
    }

    fn window_item(&self, action: Action, text: Option<&str>) -> MenuItem {
        let (label, shortcut) = self.menu_entry(action);
        let label = text.map_or(label, |text| self.fill_step(action, text));
        MenuItem {
            label,
            shortcut,
            command: Some(Command::Window(action)),
            checked: self.checked(action),
            ..MenuItem::default()
        }
    }

    fn list_item(&self, action: ListAction, text: &str) -> MenuItem {
        MenuItem {
            label: text.to_owned(),
            shortcut: self.list_menu_entry(action).1,
            command: Some(Command::List(action)),
            ..MenuItem::default()
        }
    }

    fn tracks(&self, live: &Live, kind: TrackKind) -> Vec<MenuItem> {
        let mut items: Vec<MenuItem> = live
            .tracks
            .iter()
            .filter(|track| track.kind == kind)
            .map(|track| MenuItem {
                label: track_label(track),
                command: Some(Command::Track(kind, Some(track.id))),
                checked: Some(track.selected),
                ..MenuItem::default()
            })
            .collect();
        if kind == TrackKind::Subtitle && !items.is_empty() {
            let any = live
                .tracks
                .iter()
                .any(|track| track.kind == kind && track.selected);
            items.push(MenuItem {
                label: "No subtitles".to_owned(),
                command: Some(Command::Track(kind, None)),
                checked: Some(!any),
                ..MenuItem::default()
            });
        }
        let _ = self;
        items
    }

    /// What plays the file, from the source to the outputs ("Filters").
    fn pipeline(&self, live: &Live) -> Vec<MenuItem> {
        if !self.model.media_loaded() {
            return Vec::new();
        }
        let info = |label: String| MenuItem {
            label,
            ..MenuItem::default()
        };
        let codec = |kind: TrackKind| {
            live.tracks
                .iter()
                .find(|track| track.kind == kind && track.selected)
                .and_then(|track| track.codec.clone())
        };
        let mut lines = Vec::new();
        if let Some(format) = self.model.text(PropertyId::FileFormat) {
            lines.push(info(format!("Source: {format}")));
        }
        if let Some(video) = codec(TrackKind::Video) {
            let decoding = match self.model.text(PropertyId::HwdecCurrent) {
                Some(hwdec) if !hwdec.is_empty() && hwdec != "no" => {
                    format!("hardware, {hwdec}")
                }
                _ => "software".to_owned(),
            };
            lines.push(info(format!("Video decoder: {video} ({decoding})")));
            for (label, name) in
                chain_entries(self.model.text(PropertyId::VideoFilters).unwrap_or(""))
            {
                lines.push(info(format!("Video filter: {}", filter_text(label, name))));
            }
            lines.push(info("Video output: libmpv render API (OpenGL)".to_owned()));
        }
        let audio = codec(TrackKind::Audio).or_else(|| {
            self.model
                .text(PropertyId::AudioCodecName)
                .map(str::to_owned)
        });
        if let Some(audio) = audio {
            lines.push(info(format!("Audio decoder: {audio}")));
            for (label, name) in
                chain_entries(self.model.text(PropertyId::AudioFilters).unwrap_or(""))
            {
                lines.push(info(format!("Audio filter: {}", filter_text(label, name))));
            }
            if let Some(output) = self.model.text(PropertyId::CurrentAo) {
                lines.push(info(format!("Audio output: {output}")));
            }
        }
        lines
    }

    fn chapters(&self, live: &Live) -> Vec<MenuItem> {
        let _ = self;
        live.chapters
            .iter()
            .enumerate()
            .map(|(index, chapter)| {
                // Chapter times are far inside u64.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let seconds = chapter.time.max(0.0).round() as u64;
                let time = format!(
                    "{}:{:02}:{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                );
                // The start time goes in the shortcut column.
                MenuItem {
                    label: match &chapter.title {
                        Some(title) if !title.is_empty() => title.clone(),
                        _ => format!("Chapter {}", index + 1),
                    },
                    shortcut: time,
                    command: Some(Command::Chapter(index)),
                    checked: Some(live.chapter == i64::try_from(index).ok()),
                    ..MenuItem::default()
                }
            })
            .collect()
    }

    fn equalizer_presets(&self) -> Vec<MenuItem> {
        let eq = &self.settings.equalizer;
        crate::equalizer::PRESETS
            .iter()
            .enumerate()
            .map(|(index, preset)| MenuItem {
                label: preset.name.to_owned(),
                command: Some(Command::EqualizerPreset(index)),
                checked: Some(eq.enabled && eq.preset == preset.name),
                ..MenuItem::default()
            })
            .collect()
    }

    fn speaker_choices(&self) -> Vec<MenuItem> {
        crate::speakers::CHOICES
            .iter()
            .enumerate()
            .map(|(index, choice)| MenuItem {
                label: choice.label.to_owned(),
                command: Some(Command::Speakers(index)),
                checked: Some(self.settings.output_speakers == choice.value),
                ..MenuItem::default()
            })
            .collect()
    }

    /// mpv's audio devices for a device list; the first entry is the
    /// list's default (automatic, the audio device, none).
    fn device_choices(&self, make: fn(usize) -> AudioChoice) -> Vec<MenuItem> {
        let audio = &self.settings.audio;
        let (first, current) = match make(0) {
            AudioChoice::Device(_) => (
                "Auto select (Recommended)",
                self.settings.audio_device.as_str(),
            ),
            AudioChoice::PassthroughDevice(_) => (
                "Default Audio Device (Recommended)",
                audio.passthrough_device.as_str(),
            ),
            _ => ("Disable (Recommended)", audio.second_device.as_str()),
        };
        let first_value = if matches!(make(0), AudioChoice::Device(_)) {
            "auto"
        } else {
            ""
        };
        let mut items = vec![MenuItem {
            label: first.to_owned(),
            command: Some(Command::Audio(make(0))),
            checked: Some(current == first_value),
            ..MenuItem::default()
        }];
        items.extend(
            self.audio_devices
                .iter()
                .enumerate()
                .filter(|(_, device)| device.name != "auto")
                .map(|(index, device)| MenuItem {
                    label: device.description.clone(),
                    command: Some(Command::Audio(make(index + 1))),
                    checked: Some(current == device.name),
                    ..MenuItem::default()
                }),
        );
        items
    }

    fn format_choices(&self) -> Vec<MenuItem> {
        let audio = &self.settings.audio;
        let mut items: Vec<MenuItem> = FORMATS
            .iter()
            .enumerate()
            .map(|(index, (format, label))| MenuItem {
                label: (*label).to_owned(),
                command: Some(Command::Audio(AudioChoice::Format(index))),
                checked: Some(audio.sample_format == *format),
                ..MenuItem::default()
            })
            .collect();
        items.push(MenuItem::separator());
        items.extend(
            crate::audio::SAMPLE_RATES
                .iter()
                .enumerate()
                .map(|(index, rate)| MenuItem {
                    label: if *rate == 0 {
                        "Same as input (Recommended)".to_owned()
                    } else {
                        format!("{rate}Hz")
                    },
                    command: Some(Command::Audio(AudioChoice::Rate(index))),
                    checked: Some(audio.sample_rate == *rate),
                    ..MenuItem::default()
                }),
        );
        items.push(MenuItem::separator());
        items.extend(
            QUALITIES
                .iter()
                .enumerate()
                .map(|(index, (quality, label))| MenuItem {
                    label: (*label).to_owned(),
                    command: Some(Command::Audio(AudioChoice::Resample(index))),
                    checked: Some(audio.resample_quality == *quality),
                    ..MenuItem::default()
                }),
        );
        items
    }

    fn mode_choices(&self) -> Vec<MenuItem> {
        let audio = &self.settings.audio;
        let mut items: Vec<MenuItem> = MODES
            .iter()
            .enumerate()
            .map(|(index, (mode, label))| MenuItem {
                label: (*label).to_owned(),
                command: Some(Command::Audio(AudioChoice::Mode(index))),
                checked: Some(audio.output_mode == *mode),
                ..MenuItem::default()
            })
            .collect();
        items.push(MenuItem::separator());
        items.push(MenuItem {
            label: "Use Pass-through output when at least 1 chs exist".to_owned(),
            command: Some(Command::Audio(AudioChoice::PassthroughAny)),
            checked: Some(audio.passthrough_min_channels.is_none()),
            ..MenuItem::default()
        });
        items
    }

    /// Chapter/Bookmark: the chapters, the bookmarks (when they are to be
    /// listed) and the bookmark commands.
    fn chapter_menu(&self, live: &Live) -> Vec<MenuItem> {
        let mut items = self.chapters(live);
        if self.settings.playback.bookmarks.in_menu
            && let Some(path) = self.model.loaded_path()
        {
            let bookmarks = self.marks().file(path).bookmarks;
            if !bookmarks.is_empty() {
                items.push(MenuItem::separator());
            }
            items.extend(
                bookmarks
                    .into_iter()
                    .enumerate()
                    .map(|(index, bookmark)| MenuItem {
                        label: bookmark.title,
                        shortcut: crate::view::format_time(bookmark.time),
                        command: Some(Command::Bookmark(index)),
                        ..MenuItem::default()
                    }),
            );
        }
        items.push(MenuItem::separator());
        for command in [
            crate::action::BookmarkCommand::Add,
            crate::action::BookmarkCommand::Edit,
        ] {
            items.push(self.window_item(Action::Bookmark(command), None));
        }
        items.push(MenuItem::separator());
        for command in [
            crate::action::BookmarkCommand::Thumbnails,
            crate::action::BookmarkCommand::AutoDelete,
            crate::action::BookmarkCommand::Prompt,
            crate::action::BookmarkCommand::ShowInMenu,
        ] {
            items.push(self.window_item(Action::Bookmark(command), None));
        }
        items
    }

    /// A disc's titles (DVD Control > Move).
    fn disc_title_items(&self) -> Vec<MenuItem> {
        let playing = self.model.loaded_path().unwrap_or_default();
        self.disc_titles()
            .into_iter()
            .enumerate()
            .map(|(index, (label, address))| MenuItem {
                label,
                command: Some(Command::DiscTitle(index)),
                checked: Some(address == playing),
                ..MenuItem::default()
            })
            .collect()
    }

    /// A DVD title's angles (DVD Control > Angle Option).
    fn disc_angle_items(&self) -> Vec<MenuItem> {
        (1..=self.disc_angles())
            .map(|angle| MenuItem {
                label: format!("Angle {angle}"),
                command: Some(Command::DiscAngle(angle)),
                ..MenuItem::default()
            })
            .collect()
    }

    /// Album/Favorites: the albums, the favorites and the recent items.
    fn favorites_menu(&self) -> Vec<MenuItem> {
        let mut items = self.albums();
        let marks = self.marks().marks();
        if !marks.favorites.is_empty() {
            items.push(MenuItem::separator());
            items.extend(
                marks
                    .favorites
                    .iter()
                    .enumerate()
                    .map(|(index, favorite)| MenuItem {
                        label: if favorite.folder {
                            format!("{} (folder)", favorite.title)
                        } else {
                            favorite.title.clone()
                        },
                        command: Some(Command::Favorite(index)),
                        ..MenuItem::default()
                    }),
            );
        }
        if self.settings.open.recent && !marks.recent.is_empty() {
            let recent: Vec<MenuItem> = marks
                .recent
                .iter()
                .enumerate()
                .map(|(index, path)| MenuItem {
                    label: crate::view::file_name(path).to_owned(),
                    command: Some(Command::Recent(index)),
                    ..MenuItem::default()
                })
                .collect();
            items.push(MenuItem::separator());
            items.extend(MenuItem::submenu("Recent Items", recent));
        }
        items
    }

    fn albums(&self) -> Vec<MenuItem> {
        let lists = self.library().lists();
        lists
            .albums
            .iter()
            .enumerate()
            .map(|(index, album)| MenuItem {
                label: album.name.clone(),
                command: Some(Command::Album(index)),
                checked: Some(lists.playing == Some(index)),
                ..MenuItem::default()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recorded_trees_parse() {
        let tree = context_tree();
        assert_eq!(tree.first().map(|item| item.id), Some(10443));
        let mut count = 0;
        walk(tree, &mut |_| count += 1);
        assert_eq!(count, 894, "menu-static.tsv has 894 items");
        let jump = find(tree, "101_0_9_6").expect("Jump (to)");
        assert_eq!(jump.text, "Jump (to)");
        assert_eq!(jump.children[0].id, 10242);
        let playlist = playlist_tree();
        let mut count = 0;
        walk(playlist, &mut |_| count += 1);
        assert_eq!(count, 72);
        assert_eq!(
            find(playlist, "111_0_6").map(|n| n.text.as_str()),
            Some("Add & Edit")
        );
    }

    #[test]
    fn command_ids_round_trip() {
        for command in [
            Command::Window(Action::PlayPause),
            Command::List(ListAction::SelectAll),
            Command::Track(TrackKind::Audio, Some(2)),
            Command::Track(TrackKind::Subtitle, None),
            Command::Chapter(3),
            Command::Album(1),
            Command::EqualizerPreset(4),
        ] {
            assert_eq!(Command::parse(&command.id()), Some(command), "{command:?}");
        }
        assert_eq!(Command::parse("w:no-such"), None);
        assert_eq!(Command::parse("garbage"), None);
    }

    #[test]
    fn separators_are_tidied() {
        let item = |label: &str| MenuItem {
            label: label.to_owned(),
            ..MenuItem::default()
        };
        let items = tidy(vec![
            MenuItem::separator(),
            item("a"),
            MenuItem::separator(),
            MenuItem::separator(),
            item("b"),
            MenuItem::separator(),
        ]);
        let labels: Vec<&str> = items
            .iter()
            .map(|i| if i.separator { "-" } else { i.label.as_str() })
            .collect();
        assert_eq!(labels, ["a", "-", "b"]);
        assert!(MenuItem::submenu("x", vec![MenuItem::separator()]).is_none());
    }
}
