//! The menus RVP shows (M4): PotPlayer's trees limited to what RVP has,
//! with the current keys and check marks, RVP's run-time lists, and the
//! skin's own menus (synthetic here; BlackSpace's when `RVP_TEST_SKIN` or
//! the PotPlayer folder seen from WSL has it).

use std::path::PathBuf;

use rvp_app::menu::{Command, Live, MenuItem, to_json};
use rvp_app::{Action, App, Repeat, Settings};
use rvp_engine::{Chapter, PropertyId, Track, TrackKind, Value};
use rvp_skin::menu::{UserMenu, UserMenuItem};

const DEFAULT_SKIN: &str = "/mnt/c/Program Files/DAUM/PotPlayer/Skins/BlackSpace_en.dsf";

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn new_app(name: &str, settings: Settings) -> (App, Scratch) {
    let dir = std::env::temp_dir().join(format!("rvp-menus-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (App::new(settings, &dir).expect("app"), Scratch(dir))
}

/// The item at a path of labels.
fn item<'a>(items: &'a [MenuItem], path: &[&str]) -> &'a MenuItem {
    let (first, rest) = path.split_first().expect("a path");
    let found = items
        .iter()
        .find(|item| item.label == *first)
        .unwrap_or_else(|| panic!("no {first:?} in {:?}", labels(items)));
    if rest.is_empty() {
        found
    } else {
        item(&found.children, rest)
    }
}

fn labels(items: &[MenuItem]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

fn walk(items: &[MenuItem], visit: &mut impl FnMut(&MenuItem)) {
    for item in items {
        visit(item);
        walk(&item.children, visit);
    }
}

/// Separators never start, end or double up; submenus are not empty;
/// every command survives the shell's round trip; labels say RVP (except
/// where they mean PotPlayer).
fn assert_tidy(items: &[MenuItem]) {
    assert!(!items.is_empty());
    assert!(!items[0].separator && !items[items.len() - 1].separator);
    for pair in items.windows(2) {
        assert!(!(pair[0].separator && pair[1].separator));
    }
    for item in items {
        if item.command != Some(Command::Window(Action::ImportPlaylists)) {
            assert!(!item.label.contains("PotPlayer"), "{}", item.label);
        }
        if let Some(command) = item.command {
            assert_eq!(Command::parse(&command.id()), Some(command));
        }
        if !item.children.is_empty() {
            assert_tidy(&item.children);
        }
    }
}

#[test]
fn the_context_menu_follows_potplayer_with_rvps_keys_and_state() {
    let (app, _dir) = new_app("context", Settings::default());
    let menu = app.context_menu(&Live::default());
    assert_tidy(&menu);
    assert_eq!(
        labels(&menu),
        [
            "Open File(s)...",
            "Open",
            // Record Stream (M12) is in Tools and in TV, as in PotPlayer.
            "Tools",
            "Album/Favorites",
            "Close Playback",
            "",
            "Visualization",
            "TV (WDM/BDA)",
            "DVD Control",
            "Playback",
            "Subtitles",
            "Video",
            "Audio",
            "Misc",
            "",
            "Frame Size",
            "Aspect Ratio",
            "Window Size",
            "Fullscreen (Keep AR)",
            "Fullscreen (Stretch)",
            "",
            "Preferences...",
            "Playlist...",
            "Control Panel...",
            "Playback/System Info...",
            "About...",
            "",
            "Exit",
        ]
    );
    let frame = item(&menu, &["Playback", "Jump (to)", "Previous frame"]);
    assert_eq!(frame.shortcut, "D");
    assert_eq!(frame.command, Some(Command::Window(Action::FrameBack)));
    // Steps fill PotPlayer's %s.
    item(
        &menu,
        &[
            "Subtitles",
            "Subtitle Sync",
            "Increase Subtitle Sync by 500ms",
        ],
    );
    item(
        &menu,
        &["Audio", "Audio Sync", "Decrease audio sync by 50ms"],
    );
    // Check marks show RVP's state.
    let repeat =
        |menu: &[MenuItem], label| item(menu, &["Playback", "Repeat Playback", label]).checked;
    assert_eq!(repeat(&menu, "Disable"), Some(true));
    assert_eq!(repeat(&menu, "Enable: Repeat playlist"), Some(false));
    assert_eq!(
        item(&menu, &["Audio", "Audio Effects", "Normalizer"]).checked,
        Some(true)
    );
    assert_eq!(
        item(
            &menu,
            &[
                "Misc",
                "When Playback Finished",
                "Exit RVP when playback ends"
            ]
        )
        .checked,
        Some(false)
    );
    // RVP's albums stand in for PotPlayer's favorites.
    assert_eq!(
        item(&menu, &["Album/Favorites", "Default"]).command,
        Some(Command::Album(0))
    );
    // Nothing PotPlayer has but RVP does not.
    let mut commands = 0;
    walk(&menu, &mut |item| {
        if item.command.is_some() {
            commands += 1;
        } else {
            assert!(
                item.separator || !item.children.is_empty(),
                "{}",
                item.label
            );
        }
    });
    assert!(commands > 120, "{commands} commands");

    // Other keys and states show at once.
    let mut settings = Settings::default();
    settings.keys.insert("D".into(), String::new());
    settings.keys.insert("Ctrl+B".into(), "frame-back".into());
    settings.repeat = Repeat::Album;
    let (app, _dir2) = new_app("context-rebound", settings);
    let menu = app.context_menu(&Live::default());
    assert_eq!(
        item(&menu, &["Playback", "Jump (to)", "Previous frame"]).shortcut,
        "Ctrl+B"
    );
    assert_eq!(repeat(&menu, "Disable"), Some(false));
    assert_eq!(repeat(&menu, "Enable: Repeat playlist"), Some(true));
}

fn track(id: i64, kind: TrackKind, language: &str, selected: bool) -> Track {
    Track {
        id,
        kind,
        title: None,
        language: Some(language.to_owned()),
        codec: Some("aac".to_owned()),
        selected,
        main_selection: None,
        external: false,
        channels: None,
        sample_rate: None,
        bitrate: None,
        album_art: false,
    }
}

#[test]
fn run_time_lists_take_potplayers_place() {
    let (app, _dir) = new_app("live", Settings::default());
    let live = Live {
        tracks: vec![
            track(1, TrackKind::Audio, "eng", true),
            track(2, TrackKind::Audio, "rus", false),
            track(1, TrackKind::Subtitle, "eng", false),
        ],
        chapters: vec![
            Chapter {
                title: Some("Opening".to_owned()),
                time: 0.0,
            },
            Chapter {
                title: None,
                time: 95.0,
            },
        ],
        chapter: Some(1),
    };
    let menu = app.context_menu(&live);
    assert_tidy(&menu);
    let audio = item(&menu, &["Audio", "Select Audio Stream"]);
    let russian = item(&audio.children, &["rus (aac)"]);
    assert_eq!(
        russian.command,
        Some(Command::Track(TrackKind::Audio, Some(2)))
    );
    assert_eq!(russian.checked, Some(false));
    assert_eq!(item(&audio.children, &["eng (aac)"]).checked, Some(true));
    let subtitles = item(&menu, &["Subtitles", "Add/Select Subtitles"]);
    let off = item(&subtitles.children, &["No subtitles"]);
    assert_eq!(off.command, Some(Command::Track(TrackKind::Subtitle, None)));
    assert_eq!(off.checked, Some(true));
    let chapters = item(&menu, &["Playback", "Chapter/Bookmark"]);
    // The chapters, then the bookmark commands (M11).
    assert_eq!(labels(&chapters.children[..2]), ["Opening", "Chapter 2"]);
    assert_eq!(chapters.children[3].label, "Add Bookmark");
    assert_eq!(chapters.children[1].shortcut, "0:01:35");
    assert_eq!(chapters.children[1].checked, Some(true));
    assert_eq!(chapters.children[1].command, Some(Command::Chapter(1)));
    // No file: no chapters, only the bookmark commands.
    let idle = app.context_menu(&Live::default());
    let idle_chapters = item(&idle, &["Playback", "Chapter/Bookmark"]);
    assert_eq!(idle_chapters.children[0].label, "Add Bookmark");
    assert!(
        idle_chapters
            .children
            .iter()
            .all(|item| !matches!(item.command, Some(Command::Chapter(_))))
    );
}

#[test]
fn filters_list_what_plays_the_file() {
    let (mut app, _dir) = new_app("filters", Settings::default());
    // mpv quotes values with commas as %N% plus N bytes, or in brackets.
    let sharpen = "unsharp=5:5:0.8:5:5:0.0";
    let script = "C:/svp/svp.vpy";
    let normalizer = "asetnsamples=n=2048:p=0,astats=metadata=1,volume@rvpgain=volume=1.5";
    let state = [
        (PropertyId::IdleActive, Value::Flag(false)),
        (PropertyId::Path, Value::Text("/films/film.mkv".to_owned())),
        (PropertyId::FileFormat, Value::Text("mkv".to_owned())),
        (
            PropertyId::HwdecCurrent,
            Value::Text("d3d11va-copy".to_owned()),
        ),
        (
            PropertyId::VideoFilters,
            Value::Text(format!(
                "@rvpsharpen:lavfi=graph=%{}%{sharpen},@svp:vapoursynth=file=%{}%{script}:buffered-frames=4",
                sharpen.len(),
                script.len()
            )),
        ),
        (
            PropertyId::AudioFilters,
            Value::Text(format!(
                "@rvpeq:lavfi=graph=[firequalizer@eq=gain_entry='entry(60,3.0);entry(170,0)'],\
                 @rvpnorm:lavfi=graph=%{}%{normalizer},@rvptempo:scaletempo2",
                normalizer.len()
            )),
        ),
        (PropertyId::CurrentAo, Value::Text("wasapi".to_owned())),
    ];
    for (id, value) in state {
        app.model.set(id, Some(value));
    }
    let mut video = track(1, TrackKind::Video, "und", true);
    video.codec = Some("h264".to_owned());
    let live = Live {
        tracks: vec![video, track(1, TrackKind::Audio, "eng", true)],
        ..Live::default()
    };
    let menu = app.context_menu(&live);
    assert_tidy(&menu);
    let filters = item(&menu, &["Filters"]);
    assert_eq!(
        labels(&filters.children),
        [
            "Source: mkv",
            "Video decoder: h264 (hardware, d3d11va-copy)",
            "Video filter: Sharpen",
            "Video filter: SVP",
            "Video output: libmpv render API (OpenGL)",
            "Audio decoder: aac",
            "Audio filter: Equalizer",
            "Audio filter: Normalizer (TomSteady)",
            "Audio filter: Tempo (scaletempo2)",
            "Audio output: wasapi",
        ]
    );
    // Information only, as PotPlayer's lines without a property page.
    assert!(filters.children.iter().all(|line| line.command.is_none()));
    // Nothing plays: no Filters menu.
    app.model
        .set(PropertyId::IdleActive, Some(Value::Flag(true)));
    let idle = app.context_menu(&Live::default());
    assert!(idle.iter().all(|item| item.label != "Filters"));
}

#[test]
fn the_playlist_menu_adds_undo_redo_and_import() {
    let (app, _dir) = new_app("playlist", Settings::default());
    let menu = app.playlist_menu(&Live::default());
    assert_tidy(&menu);
    assert_eq!(menu[0].label, "Play");
    let tail: Vec<&str> = labels(&menu).into_iter().rev().take(3).collect();
    assert_eq!(tail, ["Import PotPlayer Playlists...", "Redo", "Undo"]);
    assert_eq!(item(&menu, &["Undo"]).shortcut, "Ctrl+Z");
    assert_eq!(
        item(&menu, &["Remove", "Remove selected items"]).shortcut,
        "Del"
    );
}

#[test]
fn skin_menus_name_commands_and_builtin_submenus() {
    let (app, _dir) = new_app("skin", Settings::default());
    let entry = |command: &str, inline: bool| UserMenuItem {
        command: command.to_owned(),
        inline,
    };
    let menu = UserMenu {
        name: "Test".to_owned(),
        items: vec![
            entry("ID_OPEN_URL", false),
            entry("", false),
            entry("", false),
            entry("CMD_POPUPMENU_PLAY_REPEAT", false),
            entry("CMD_POPUPMENU_DVD", true),
            entry("CMD_POPUPMENU_FILTER", false),
            entry("ID_NO_SUCH_COMMAND", false),
            entry("", false),
        ],
    };
    let items = app.skin_menu(&menu, &Live::default());
    assert_tidy(&items);
    assert_eq!(
        labels(&items),
        [
            "Open URL...",
            "",
            "Repeat Playback",
            "Previous Chapter",
            "Next Chapter",
            "",
            "Speed"
        ]
    );
    assert_eq!(items[0].shortcut, "Ctrl+U");
    assert_eq!(
        item(&items, &["Repeat Playback", "Disable"]).checked,
        Some(true)
    );
    // The equalizer's presets (M6), the one in use checked.
    let eq = UserMenu {
        name: "EQ".to_owned(),
        items: vec![entry("CMD_POPUPMENU_AUDIO_EQULIZER", true)],
    };
    let items = app.skin_menu(&eq, &Live::default());
    assert_eq!(items.len(), rvp_app::equalizer::PRESETS.len());
    assert_eq!(items[0].label, "(Default)");
    assert_eq!(items[0].checked, Some(false), "the equalizer is off");
    // A menu RVP has nothing for still answers.
    let empty = UserMenu {
        name: "Fonts".to_owned(),
        items: vec![entry("CMD_POPUPMENU_CAPTION_FONT", true)],
    };
    let items = app.skin_menu(&empty, &Live::default());
    assert_eq!(labels(&items), ["Not in RVP yet"]);
    assert_eq!(items[0].command, None);
    // Built-in submenus on their own (buttons with CMD_POPUPMENU_*).
    let (title, items) = app
        .popup("CMD_POPUPMENU_SIZE", &Live::default())
        .expect("window size");
    assert_eq!(title, "Window Size");
    assert_eq!(items[0].label, "Minimize (to Taskbar/Tray)");
    assert!(
        app.popup("CMD_POPUPMENU_FILTER", &Live::default())
            .is_none()
    );
    // Without a file the bookmark menu has its commands only.
    let (title, items) = app
        .popup("CMD_POPUPMENU_PLAY_BOOKMARK", &Live::default())
        .expect("bookmarks");
    assert_eq!(title, "Chapter/Bookmark");
    assert_eq!(items[0].label, "Add Bookmark");
}

#[test]
fn menus_reach_the_shell_as_json() {
    let (app, _dir) = new_app("json", Settings::default());
    let menu = app.playlist_menu(&Live::default());
    let json: serde_json::Value = serde_json::from_str(&to_json(&menu)).expect("JSON");
    let first = &json[0];
    assert_eq!(first["label"], "Play");
    assert_eq!(first["id"], "l:play");
    assert_eq!(first["checked"], -1);
    assert_eq!(json[1]["label"], "Play within a new instance");
    assert_eq!(json[2]["separator"], true);
    let album = json
        .as_array()
        .and_then(|items| items.iter().find(|item| item["label"] == "Album"))
        .expect("Album");
    assert_eq!(album["id"], "");
    assert!(album["children"].as_array().is_some_and(|c| !c.is_empty()));
}

#[test]
fn blackspace_menus_build() {
    let path = std::env::var_os("RVP_TEST_SKIN")
        .map_or_else(|| PathBuf::from(DEFAULT_SKIN), PathBuf::from);
    if !path.exists() {
        eprintln!("skipped: BlackSpace skin not found at {}", path.display());
        return;
    }
    let skin = rvp_skin::Skin::open(&path).expect("BlackSpace loads");
    let (app, _dir) = new_app("blackspace", Settings::default());
    let menus = skin.user_menus();
    assert_eq!(menus.len(), 19);
    for menu in &menus {
        let items = app.skin_menu(menu, &Live::default());
        assert_tidy(&items);
        // Every command RVP carries out is in the menu.
        for entry in &menu.items {
            if let Some(action) = Action::from_skin_command(&entry.command) {
                let mut found = false;
                walk(&items, &mut |item| {
                    found |= item.command == Some(Command::Window(action));
                });
                assert!(found, "{}: {}", menu.name, entry.command);
            }
        }
    }
    let open = menus
        .iter()
        .find(|m| m.name == "OpenMenu2")
        .expect("OpenMenu2");
    assert_eq!(
        labels(&app.skin_menu(open, &Live::default())),
        [
            "Open File(s)...",
            "Close Playback",
            "",
            "Open Folder...",
            "Open URL...",
            "Load Subtitle..."
        ]
    );
}
