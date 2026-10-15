//! The dispatcher's coverage reports (M4, M7) in `docs/coverage/`:
//! PotPlayer's menus as R1 recorded them, RVP Default's commands, and
//! BlackSpace's live commands. The menu and RVP Default reports are always
//! checked; BlackSpace's only when the third-party skin is there
//! (`RVP_TEST_SKIN`, default: the PotPlayer folder seen from WSL).
//! `RVP_BLESS=1` rewrites them.

use std::path::{Path, PathBuf};

use rvp_app::Action;
use rvp_app::coverage::{
    BLACKSPACE_INTRO, DEFAULT_SKIN_INTRO, ItemStatus, commands, item_status, menu_report,
    skin_covered, skin_reason, skin_report,
};
use rvp_app::menu::{context_tree, playlist_tree};
use rvp_skin::{Skin, SkinArchive};

const DEFAULT_SKIN: &str = "/mnt/c/Program Files/DAUM/PotPlayer/Skins/BlackSpace_en.dsf";

fn docs(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/coverage")
        .join(name)
}

fn check(name: &str, report: &str) {
    let path = docs(name);
    if std::env::var_os("RVP_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("docs/coverage")).expect("create docs");
        std::fs::write(&path, report).expect("write the report");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} exists; run with RVP_BLESS=1", path.display()));
    assert!(
        expected == report,
        "{} is out of date; run with RVP_BLESS=1 and review the change",
        path.display()
    );
}

#[test]
fn every_menu_item_is_carried_out_or_explained() {
    for tree in [context_tree(), playlist_tree()] {
        let items = commands(tree);
        assert!(!items.is_empty());
        for (place, item) in items {
            if let ItemStatus::Missing(why) = item_status(item) {
                assert!(!why.is_empty(), "{place} ({}) has no reason", item.id);
            }
        }
    }
    // Spot checks against the action table.
    let find = |id: u32| {
        commands(context_tree())
            .into_iter()
            .find(|(_, item)| item.id == id)
            .map(|(_, item)| item_status(item))
    };
    assert!(
        matches!(find(10014), Some(ItemStatus::Window(_))),
        "Play/Pause"
    );
    assert!(
        matches!(find(10324), Some(ItemStatus::Runtime(_))),
        "chapters"
    );
    assert!(
        matches!(find(10383), Some(ItemStatus::Window(Action::ControlPanel))),
        "control panel"
    );
    assert!(
        matches!(find(20008), Some(ItemStatus::Runtime(_))),
        "equalizer presets"
    );
    assert!(
        matches!(find(10770), Some(ItemStatus::Missing(_))),
        "AviSynth"
    );
}

#[test]
fn the_menu_report_is_current() {
    check("menus.md", &menu_report());
}

#[test]
fn skin_reasons() {
    assert!(skin_covered("CMD_MEDIA_PLAY"));
    assert!(skin_covered("CMD_POPUP_USER_MENU"));
    assert!(skin_covered("CMD_SELECT_PAGE"));
    assert!(skin_covered("CMD_TIMEBAR_CTRL"));
    assert!(skin_covered("CMD_POPUPMENU_PLAY_GOTO"));
    assert!(!skin_covered("CMD_BROADCAST_START"));
    assert!(skin_reason("CMD_BROADCAST_START").contains("Daum"));
    assert!(skin_reason("CMD_CAPTURE1ST_RECORD").contains("capture"));
    assert!(!skin_reason("ID_SOMETHING_NEW").is_empty());
}

#[test]
fn the_blackspace_report_is_current() {
    let path = std::env::var_os("RVP_TEST_SKIN")
        .map_or_else(|| PathBuf::from(DEFAULT_SKIN), PathBuf::from);
    if !path.exists() {
        eprintln!("skipped: BlackSpace skin not found at {}", path.display());
        return;
    }
    let skin = Skin::open(&path).expect("BlackSpace loads");
    let commands = skin.commands();
    assert!(
        commands.len() > 200,
        "BlackSpace names {} commands",
        commands.len()
    );
    let menus = skin.user_menus();
    assert_eq!(menus.len(), 19, "BlackSpace has 19 skin menus");
    check(
        "blackspace.md",
        &skin_report("BlackSpace", BLACKSPACE_INTRO, &commands),
    );
}

/// Commands RVP Default keeps for the classic layout's sake although RVP
/// does not carry them out yet; any other must work.
const DEFAULT_SKIN_GAPS: [&str; 16] = [
    "CMD_POPUPMENU_AUDIO_CHANNEL",
    "CMD_POPUPMENU_AUDIO_SAMPLE",
    "CMD_POPUPMENU_CAPTION_FONT",
    "CMD_POPUPMENU_CAPTION_MARGIN",
    "CMD_POPUPMENU_CAPTION_PARA",
    "ID_CAPTION_MOVE_LEFT",
    "ID_CAPTION_MOVE_RIGHT",
    "ID_ETC_CLOSE_SYSTEM_FILE",
    "ID_ETC_CLOSE_SYSTEM_LIST",
    "ID_ETC_RESTORE_POS",
    "ID_OPEN_DVD",
    "ID_PLAYLIST_SAVE",
    "ID_SKIN_AUTO_HIDE",
    "ID_VIDEO_ADV_CAPTURE",
    "ID_VIDEO_CAPTURE_CAPTION",
    "ID_VIDEO_IMAGE_THUMBNAIL",
];

#[test]
fn the_rvp_default_report_is_current() {
    let bytes = rvp_look::skin::dsf().expect("RVP Default builds");
    let archive =
        SkinArchive::from_zip_bytes(rvp_look::skin::NAME, bytes).expect("a readable archive");
    let skin = Skin::load(archive, "VideoSkin.xml").expect("RVP Default loads");
    let commands = skin.commands();
    let gaps: Vec<&str> = commands
        .keys()
        .map(String::as_str)
        .filter(|command| !skin_covered(command))
        .collect();
    let unexpected: Vec<&&str> = gaps
        .iter()
        .filter(|command| !DEFAULT_SKIN_GAPS.contains(command))
        .collect();
    assert!(
        unexpected.is_empty(),
        "RVP does not carry out {unexpected:?}"
    );
    check(
        "rvp-default.md",
        &skin_report("RVP Default", DEFAULT_SKIN_INTRO, &commands),
    );
}
