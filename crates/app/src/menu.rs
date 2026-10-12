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
//!   Stream" and "Add/Select Subtitles"), the chapters ("Chapter/Bookmark")
//!   and the albums ("Album/Favorites").
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

use rvp_engine::{Chapter, Engine, Track, TrackKind};
use serde_json::{Value, json};

use crate::action::{Action, ListAction};
use crate::app::{App, UiRequest};
use crate::commands::track_label;

const CONTEXT_TSV: &str = include_str!("../../../docs/r1/menu-static.tsv");
const PLAYLIST_TSV: &str = include_str!("../../../docs/r1/menu-playlist.tsv");

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
    /// A submenu of PotPlayer's playlist menu, by path.
    Playlist(&'static str),
    Tracks(TrackKind),
    Chapters,
    Albums,
}

/// The part of PotPlayer's menus a skin's built-in submenu command shows;
/// `None` for the ones RVP has nothing for.
#[must_use]
pub fn popup_source(command: &str) -> Option<PopupSource> {
    Some(match command {
        "CMD_POPUPMENU_ETC" => PopupSource::Context("101_0_15"),
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
        _ => return None,
    })
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
            PopupSource::Tracks(kind) => ("Tracks".to_owned(), self.tracks(live, kind)),
            PopupSource::Chapters => ("Chapters".to_owned(), self.chapters(live)),
            PopupSource::Albums => ("Albums".to_owned(), self.albums()),
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
                self.play_album(engine, index);
                None
            }
        }
    }

    fn convert(&self, node: &Recorded, live: &Live) -> Option<MenuItem> {
        if node.separator {
            return Some(MenuItem::separator());
        }
        if node.submenu {
            let mut children: Vec<MenuItem> = node
                .children
                .iter()
                .filter_map(|child| self.convert(child, live))
                .collect();
            // RVP's run-time lists where PotPlayer has its own.
            let extra = match node.path.as_str() {
                "101_0_12_0" => self.tracks(live, TrackKind::Audio),
                "101_0_11_0" => self.tracks(live, TrackKind::Video),
                "101_0_10_0" => self.tracks(live, TrackKind::Subtitle),
                "101_0_3" => self.albums(),
                _ => Vec::new(),
            };
            if !extra.is_empty() {
                children.push(MenuItem::separator());
                children.extend(extra);
            }
            return MenuItem::submenu(&node.text, children);
        }
        let text = node.text.replace("PotPlayer", "RVP");
        if let Some(action) = Action::from_potplayer_id(node.id) {
            return Some(self.window_item(action, Some(&text)));
        }
        if let Some(action) = ListAction::from_potplayer_id(node.id) {
            return Some(self.list_item(action, &text));
        }
        // "Chapter/Bookmark" is a list of the file's chapters.
        if node.id == 10324 {
            return MenuItem::submenu(&node.text, self.chapters(live));
        }
        None
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
