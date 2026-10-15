//! The dispatcher's coverage of PotPlayer's commands (M4): which items of
//! PotPlayer's menus (R1) and which commands of a skin RVP carries out, and
//! why the others are not. `tests/coverage.rs` writes the reports to
//! `docs/coverage/` (`RVP_BLESS=1`) and checks that they are current.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rvp_skin::menu::CommandUse;

use crate::action::{Action, ListAction};
use crate::menu::{PopupSource, Recorded, context_tree, playlist_tree, popup_source};

/// Why parts of PotPlayer's context menu (resource 101) are not in RVP, by
/// submenu path; the longest matching path wins.
const CONTEXT_REASONS: &[(&str, &str)] = &[
    ("101_0", "not in RVP yet"),
    (
        "101_0_1",
        "other sources (FTP/WebDAV, devices, TV, DVD, Blu-ray, external audio) are not planned or not in RVP yet; RVP opens files, folders, URLs and subtitles",
    ),
    ("101_0_1_4", "RVP adds opened files to the shown album"),
    (
        "101_0_1_5",
        "PotPlayer's file navigator and Open Box are not planned",
    ),
    (
        "101_0_2",
        "Daum's broadcast and chat services (discontinued) and PotPlayer's screen capture are not planned",
    ),
    ("101_0_3", "favorites: RVP's albums take their place"),
    ("101_0_6", "visualizations are not planned"),
    ("101_0_7", "TV devices are not planned"),
    ("101_0_8", "DVD menus are not planned (chapters are RVP's)"),
    ("101_0_9", "not in RVP yet"),
    (
        "101_0_9_6",
        "keyframe steps, the jump dialog and the scene browsers are not in RVP yet",
    ),
    ("101_0_9_7", "mpv always stretches the audio (scaletempo)"),
    ("101_0_9_8", "skipping intros and endings is not in RVP yet"),
    (
        "101_0_9_9",
        "RVP's albums have no playback queue, thumbnail views, copy and paste or file operations yet",
    ),
    (
        "101_0_9_10",
        "A-B lists, fine point moves and subtitle sections are not in RVP yet",
    ),
    ("101_0_9_12", "seamless playback is not planned"),
    ("101_0_10", "not in RVP yet"),
    ("101_0_10_8", "mpv (libass) renders every subtitle"),
    (
        "101_0_10_9",
        "bold, spacing and sideways moves are not in RVP yet (mpv cannot move subtitles sideways)",
    ),
    (
        "101_0_10_10",
        "frame rate conversion and keeping sync changes are not in RVP yet",
    ),
    ("101_0_10_11", "saving subtitles is not planned"),
    ("101_0_10_12", "3D subtitles are not planned"),
    ("101_0_10_13", "subtitle search is not planned"),
    ("101_0_11", "not in RVP yet"),
    ("101_0_11_1", "RVP renders through libmpv (D4)"),
    (
        "101_0_11_2",
        "mpv's GLSL shaders choose their own stage, so RVP has one shader list",
    ),
    ("101_0_11_3", "3D video is not planned"),
    (
        "101_0_11_4",
        "DirectShow transform filters do not apply to libmpv",
    ),
    (
        "101_0_11_6",
        "mpv scales the video (the scaler is a setting)",
    ),
    (
        "101_0_11_8",
        "extending and cropping the picture is not in RVP yet",
    ),
    ("101_0_11_9", "bottom margins are not in RVP yet"),
    (
        "101_0_11_10",
        "this effect is not in RVP yet (RVP has deblock, denoise 3D, soften, sharpen, the flips and deband)",
    ),
    (
        "101_0_11_11",
        "rotating by the aspect ratio is not in RVP yet",
    ),
    (
        "101_0_11_12",
        "consecutive images, thumbnails, the clipboard, BMP and file name options are not in RVP yet",
    ),
    ("101_0_11_13", "video recording is not planned"),
    (
        "101_0_11_14",
        "AviSynth is not planned; RVP runs VapourSynth scripts (settings, SVP and VapourSynth page)",
    ),
    ("101_0_12", "not in RVP yet"),
    ("101_0_12_1", "the audio device is a setting"),
    ("101_0_12_2", "pass-through is not in RVP yet"),
    ("101_0_12_3", "a second audio device is not planned"),
    ("101_0_12_8", "RVP sets mpv's volume"),
    ("101_0_12_9", "the system volume is not RVP's to change"),
    ("101_0_12_17", "keeping sync changes is not in RVP yet"),
    (
        "101_0_12_18",
        "audio effects other than the normalizer are not in RVP yet",
    ),
    ("101_0_12_19", "audio recording is not planned"),
    ("101_0_13", "DirectShow filters do not apply to libmpv"),
    ("101_0_14", "RVP loads one skin, chosen in the settings"),
    ("101_0_15", "not in RVP yet"),
    ("101_0_15_2", "RVP's interface is English"),
    (
        "101_0_15_4",
        "shutting down, sleeping or hibernating the computer is not planned",
    ),
    ("101_0_15_5", "RVP adds the files it is given"),
    (
        "101_0_15_6",
        "copying names to the clipboard is not in RVP yet",
    ),
    (
        "101_0_17",
        "frame size presets and screen margins are not in RVP yet",
    ),
    (
        "101_0_18",
        "custom ratios, output device correction and margins are not in RVP yet",
    ),
    (
        "101_0_19",
        "other window sizes and window moves are not in RVP yet",
    ),
];

/// Why parts of PotPlayer's playlist menu (resource 111) are not in RVP.
const PLAYLIST_REASONS: &[(&str, &str)] = &[
    ("111_0", "not in RVP's albums yet"),
    (
        "111_0_4",
        "RVP saves its albums itself; the import reads PotPlayer's",
    ),
    ("111_0_5", "a playback queue is not in RVP yet"),
    (
        "111_0_6",
        "editing titles, file operations and the clipboard are not in RVP yet",
    ),
    ("111_0_7", "deleting files is not planned"),
    ("111_0_8", "sorting by other fields is not in RVP yet"),
    ("111_0_10", "thumbnail views are not in RVP yet"),
];

/// Reasons for single items.
const ITEM_REASONS: &[(u32, &str)] = &[
    (10187, "Daum's login, discontinued"),
    (10325, "the jump dialog is not in RVP yet"),
    (10792, "single instance: rvp /new opens another window"),
    (10640, "RVP learns durations when files play"),
    (10505, "file information is not in RVP yet"),
    (10800, "thumbnails are not in RVP yet"),
    (10069, "shuffle is not in RVP yet"),
    (
        10419,
        "RVP always remembers positions (the edges are settings)",
    ),
    (
        10121,
        "RVP hides the pointer only in fullscreen (after 2 s)",
    ),
    (10122, "keeping the screen saver off is not in RVP yet"),
    (
        10445,
        "\"When Playback Finished\" covers closing after the list",
    ),
    (10921, "deleting played files is not planned"),
    (10812, "seek bar tooltips and markers are not in RVP yet"),
    (10813, "seek bar tooltips and markers are not in RVP yet"),
    (10814, "seek bar tooltips and markers are not in RVP yet"),
    (10940, "combined subtitles are not in RVP yet"),
    (10228, "subtitle editing is not planned"),
    (10266, "subtitle editing is not planned"),
    (10220, "DirectShow output surfaces do not apply to libmpv"),
    (10221, "DirectShow output surfaces do not apply to libmpv"),
    (10624, "copying to the clipboard is not in RVP yet"),
    (10822, "mpv places subtitles (a setting may come later)"),
    (10823, "mpv places subtitles (a setting may come later)"),
    (10824, "mpv places subtitles (a setting may come later)"),
    (10825, "mpv (libass) always animates ASS subtitles"),
    (10826, "subtitle styles are not in RVP yet"),
    (10827, "subtitle styles are not in RVP yet"),
    (
        37750,
        "playing one channel of a stereo track is not in RVP yet",
    ),
    (
        37751,
        "playing one channel of a stereo track is not in RVP yet",
    ),
    (
        37752,
        "playing one channel of a stereo track is not in RVP yet",
    ),
    (
        20005,
        "output channels and resampling are mpv's (not in RVP yet)",
    ),
    (
        20006,
        "output channels and resampling are mpv's (not in RVP yet)",
    ),
    (
        20007,
        "output channels and resampling are mpv's (not in RVP yet)",
    ),
    (10454, "DirectShow filters do not apply to libmpv"),
    (
        10917,
        "mpv manages colours and HDR itself (target colour space, tone mapping)",
    ),
    (
        11025,
        "mpv manages colours and HDR itself (target colour space, tone mapping)",
    ),
    (
        11026,
        "mpv manages colours and HDR itself (target colour space, tone mapping)",
    ),
    (
        11027,
        "mpv manages colours and HDR itself (target colour space, tone mapping)",
    ),
    (10644, "presets are not in RVP yet"),
    (10968, "stay on top while in fullscreen is not in RVP yet"),
    (10479, "shutting the computer down is not planned"),
    (10784, "shutting the computer down is not planned"),
    (10480, "shutting the computer down is not planned"),
    (10339, "RVP's keys are all editable (Keys and Mouse)"),
    (10807, "recent items are not in RVP yet"),
    (10809, "touch input is not planned"),
    (10442, "other on-screen information is not in RVP yet"),
    (10449, "other on-screen information is not in RVP yet"),
    (10386, "RVP is private and has no website or update service"),
    (10392, "RVP is private and has no website or update service"),
];

fn reason(reasons: &[(&str, &'static str)], path: &str, id: u32) -> &'static str {
    if let Some((_, reason)) = ITEM_REASONS.iter().find(|(item, _)| *item == id) {
        return reason;
    }
    reasons
        .iter()
        .filter(|(prefix, _)| path == *prefix || path.starts_with(&format!("{prefix}_")))
        .max_by_key(|(prefix, _)| prefix.len())
        .map_or("not in RVP yet", |(_, reason)| reason)
}

/// What RVP does with one of PotPlayer's menu items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStatus {
    Window(Action),
    List(ListAction),
    /// RVP's run-time list in place of PotPlayer's.
    Runtime(&'static str),
    Missing(&'static str),
}

/// The status of a recorded menu item with a command (not a submenu or
/// separator).
#[must_use]
pub fn item_status(item: &Recorded) -> ItemStatus {
    if let Some(action) = Action::from_potplayer_id(item.id) {
        return ItemStatus::Window(action);
    }
    if let Some(action) = ListAction::from_potplayer_id(item.id) {
        return ItemStatus::List(action);
    }
    if item.id == 10324 {
        return ItemStatus::Runtime("the file's chapters");
    }
    if item.id == 20008 {
        return ItemStatus::Runtime("the equalizer's presets");
    }
    let reasons = if item.path.starts_with("111") {
        PLAYLIST_REASONS
    } else {
        CONTEXT_REASONS
    };
    ItemStatus::Missing(reason(reasons, &item.path, item.id))
}

/// The items of a recorded menu that carry a command, with their place
/// (`Playback > Jump (to) > Previous frame`).
#[must_use]
pub fn commands(tree: &[Recorded]) -> Vec<(String, &Recorded)> {
    fn visit<'a>(items: &'a [Recorded], place: &str, out: &mut Vec<(String, &'a Recorded)>) {
        for item in items {
            let here = if place.is_empty() {
                item.text.clone()
            } else {
                format!("{place} > {}", item.text)
            };
            if item.submenu {
                visit(&item.children, &here, out);
            } else if !item.separator && item.id != 0 {
                out.push((here, item));
            }
        }
    }
    let mut out = Vec::new();
    visit(tree, "", &mut out);
    out
}

fn menu_section(out: &mut String, title: &str, tree: &[Recorded]) {
    let items = commands(tree);
    let mut done = Vec::new();
    let mut missing: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (place, item) in &items {
        match item_status(item) {
            ItemStatus::Window(action) => done.push((place, item.id, action.name().to_owned())),
            ItemStatus::List(action) => {
                done.push((place, item.id, format!("playlist: {}", action.name())));
            }
            ItemStatus::Runtime(what) => done.push((place, item.id, what.to_owned())),
            ItemStatus::Missing(why) => missing
                .entry(why)
                .or_default()
                .push(format!("{place} ({})", item.id)),
        }
    }
    let _ = writeln!(
        out,
        "\n## {title}\n\n{} items with a command: {} carried out, {} not.\n\n\
         | Item | ID | RVP |\n|------|----|-----|",
        items.len(),
        done.len(),
        items.len() - done.len()
    );
    for (place, id, what) in done {
        let _ = writeln!(out, "| {place} | {id} | `{what}` |");
    }
    let _ = writeln!(out, "\nNot carried out, by reason:\n");
    for (why, places) in missing {
        let _ = writeln!(out, "- **{why}** ({}): {}", places.len(), places.join("; "));
    }
}

/// `docs/coverage/menus.md`.
#[must_use]
pub fn menu_report() -> String {
    let mut out = String::from(
        "# PotPlayer's menus in RVP\n\nGenerated by `cargo test -p rvp-app --test coverage` \
         (`RVP_BLESS=1` rewrites it) from R1's `docs/r1/menu-static.tsv` and \
         `docs/r1/menu-playlist.tsv`: every item with a command, the RVP action that carries it \
         out, or why RVP does not. RVP's menus show the carried-out items in PotPlayer's order \
         (`crates/app/src/menu.rs`).\n",
    );
    menu_section(&mut out, "Context menu (resource 101)", context_tree());
    menu_section(&mut out, "Playlist menu (resource 111)", playlist_tree());
    out
}

/// Skin commands the window shows data with (labels, sliders, images).
pub const DISPLAY: &[(&str, &str)] = &[
    ("CMD_AUDIO_BIT_RATE_MESSAGE", "audio bit rate"),
    ("CMD_AUDIO_CODEC_TAG_MESSAGE", "audio codec"),
    ("CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE", "audio channels"),
    ("CMD_AUDIO_OUT_SAMPLE_RATE_MESSAGE", "audio sample rate"),
    ("CMD_CURRENTTIME_MESSAGE", "time played"),
    ("CMD_LEFTTIME_MESSAGE", "time left"),
    ("CMD_PERCENTTIME_MESSAGE", "percent played"),
    ("CMD_PLAY_FILE_TYPE", "file type"),
    ("CMD_SYSTEM_TIME_MESSAGE", "the clock"),
    ("CMD_TIMEBAR_CTRL", "seek bar"),
    ("CMD_TITLE_MESSAGE", "title"),
    ("CMD_TOTALTIME_MESSAGE", "duration"),
    ("CMD_VIDEO_BIT_RATE_MESSAGE", "video bit rate"),
    ("CMD_VIDEO_CODEC_FOURCC_MESSAGE", "video codec"),
    ("CMD_VIDEO_CODEC_FPS_MESSAGE", "source frame rate"),
    ("CMD_VIDEO_CODEC_SIZE_MESSAGE", "video size"),
    ("CMD_VIDEO_OUT_FOURCC_MESSAGE", "decoder output"),
    ("CMD_VIDEO_OUT_FPS_MESSAGE", "output frame rate"),
    ("CMD_VOLUME_CTRL", "volume slider"),
    (
        "CMD_MASTER_VOLUME",
        "the volume (the control panel's slider; RVP's volume, not the system's)",
    ),
    ("CMD_COLOR_OFFSET", "and sets the brightness (slider)"),
    ("CMD_COLOR_GAIN", "and sets the contrast (slider)"),
    ("CMD_COLOR_SAT", "and sets the saturation (slider)"),
    ("CMD_COLOR_HUE", "and sets the hue (slider)"),
    (
        "CMD_EQULIZER_0",
        "and sets the equalizer's 60 Hz band (slider)",
    ),
    (
        "CMD_EQULIZER_1",
        "and sets the equalizer's 170 Hz band (slider)",
    ),
    (
        "CMD_EQULIZER_2",
        "and sets the equalizer's 310 Hz band (slider)",
    ),
    (
        "CMD_EQULIZER_3",
        "and sets the equalizer's 600 Hz band (slider)",
    ),
    (
        "CMD_EQULIZER_4",
        "and sets the equalizer's 1 kHz band (slider)",
    ),
    (
        "CMD_EQULIZER_5",
        "and sets the equalizer's 3 kHz band (slider)",
    ),
    (
        "CMD_EQULIZER_6",
        "and sets the equalizer's 6 kHz band (slider)",
    ),
    (
        "CMD_EQULIZER_7",
        "and sets the equalizer's 12 kHz band (slider)",
    ),
    (
        "CMD_EQULIZER_8",
        "and sets the equalizer's 14 kHz band (slider)",
    ),
    (
        "CMD_EQULIZER_9",
        "and sets the equalizer's 16 kHz band (slider)",
    ),
    ("CMD_EQ_PRESET_MESSAGE", "the equalizer's preset"),
    ("CMD_CAPTION_SIZE_MESSAGE", "the subtitle size"),
    ("CMD_CAPTION_FONT_MESSAGE", "the subtitle font"),
    ("CMD_REPEAT_START_MESSAGE", "A-B point A"),
    ("CMD_REPEAT_END_MESSAGE", "A-B point B"),
];

/// Why a skin command is not carried out.
#[must_use]
pub fn skin_reason(command: &str) -> &'static str {
    let starts = |prefixes: &[&str]| prefixes.iter().any(|p| command.starts_with(p));
    if starts(&[
        "CMD_BROADCAST",
        "CMD_CHAT",
        "CMD_CAHT",
        "CMD_SERVER_CHAT",
        "CMD_CLIENT_CHAT",
        "CMD_GOOD_",
        "CMD_BAD_",
        "CMD_COPY_BROADCAST",
        "CMD_VIEWER_",
        "CMD_TRANS_",
        "ID_BROADCAST",
        "ID_CHAT",
        "ID_LOGIN",
    ]) || matches!(command, "CMD_ATTACH_BROADCAST" | "CMD_ATTACH_CHAT")
    {
        return "Daum's broadcast and chat services, discontinued";
    }
    if starts(&["CMD_CAPTURE1ST", "CMD_CAPTURE2ND", "CMD_CAPTURE_PIP"]) {
        return "PotPlayer's screen capture areas are not planned";
    }
    match command {
        "ID_VIDEO_MARGIN" => "screen margins are not in RVP yet",
        "ID_SKIN_AUTO_HIDE" => "the skin on/off (floating window) mode is not in RVP yet",
        "ID_OPEN_DVD" => "DVD playback is not planned",
        "ID_ETC_CLOSE_SYSTEM_FILE" | "ID_ETC_CLOSE_SYSTEM_LIST" => {
            "shutting the computer down is not planned"
        }
        "ID_ETC_RESTORE_POS" => "its meaning is not confirmed",
        "ID_VIDEO_IMAGE_THUMBNAIL" | "ID_VIDEO_ADV_CAPTURE" | "ID_VIDEO_CAPTURE_CAPTION" => {
            "thumbnails and advanced capture are not in RVP yet"
        }
        "ID_PLAYLIST_SAVE" => "saving albums as playlist files is not in RVP yet",
        "ID_CAPTION_MOVE_LEFT" | "ID_CAPTION_MOVE_RIGHT" => "mpv cannot move subtitles sideways",
        "CMD_POPUPMENU_CAPTION_FONT" => {
            "the subtitle font is chosen in a font dialog (subtitle-font, Alt+F), not from a menu"
        }
        "CMD_POPUPMENU_AUDIO_CHANNEL" | "CMD_POPUPMENU_AUDIO_SAMPLE" => {
            "output channels and resampling are mpv's (not in RVP yet)"
        }
        "CMD_POPUPMENU_CAPTION_MARGIN" | "CMD_POPUPMENU_CAPTION_PARA" => {
            "subtitle margins and spacing are not in RVP yet"
        }
        _ => "not in RVP yet",
    }
}

/// What RVP does with a skin command.
#[must_use]
pub fn skin_status(command: &str) -> String {
    if let Some(action) = Action::from_skin_command(command) {
        return format!("`{}`", action.name());
    }
    if let Some(action) = ListAction::from_skin_command(command) {
        return format!("playlist: `{}`", action.name());
    }
    if let Some((_, what)) = DISPLAY.iter().find(|(name, _)| *name == command) {
        return format!("shows {what}");
    }
    if command == "CMD_POPUP_USER_MENU" {
        return "opens the skin menu it names".to_owned();
    }
    if command == "CMD_SELECT_PAGE" {
        return "shows the page it names (`PageIndex`)".to_owned();
    }
    if let Some(source) = popup_source(command) {
        return match source {
            PopupSource::Context(path) => format!("menu: PotPlayer's `{path}`"),
            PopupSource::Playlist(path) => format!("menu: PotPlayer's playlist `{path}`"),
            PopupSource::Tracks(_) => "menu: the file's tracks".to_owned(),
            PopupSource::Chapters => "menu: the file's chapters".to_owned(),
            PopupSource::Albums => "menu: the albums".to_owned(),
            PopupSource::EqualizerPresets => "menu: the equalizer's presets".to_owned(),
        };
    }
    format!("not carried out: {}", skin_reason(command))
}

/// True when RVP carries the command out (or shows its data).
#[must_use]
pub fn skin_covered(command: &str) -> bool {
    !skin_status(command).starts_with("not carried out")
}

/// What the report on BlackSpace says before its table.
pub const BLACKSPACE_INTRO: &str = "Generated by `cargo test -p rvp-app --test coverage` \
     (`RVP_BLESS=1`, with `RVP_TEST_SKIN` set) from the skin's live elements: every \
     `CmdID`, `AltCmdID` and menu item, where it appears, and what RVP does with it. \
     RVP shows `MainSkinArea` with `PlaylistSkinArea`, the control panel \
     (`ControlBoxSkinArea`, F7) and the menus; the broadcast, chat and capture areas \
     belong to discontinued services.";

/// What the report on RVP Default says before its table.
pub const DEFAULT_SKIN_INTRO: &str = "Generated by `cargo test -p rvp-app --test coverage` \
     (`RVP_BLESS=1`) from the live elements of RVP Default, RVP's own skin \
     (`crates/look/art/skin`): every `CmdID`, `AltCmdID` and menu item, where it appears, \
     and what RVP does with it. The skin keeps the classic layout's commands and menus; \
     the few RVP does not carry out yet say why.";

/// A skin's coverage report (`docs/coverage/blackspace.md`,
/// `docs/coverage/rvp-default.md`) from its commands.
#[must_use]
pub fn skin_report(skin: &str, intro: &str, commands: &BTreeMap<String, CommandUse>) -> String {
    let covered = commands.keys().filter(|c| skin_covered(c)).count();
    let mut out = format!(
        "# {skin}'s commands in RVP\n\n{intro}\n\n\
         {} commands: {covered} carried out or shown, {} not.\n\n\
         | Command | Where | RVP |\n|---------|-------|-----|\n",
        commands.len(),
        commands.len() - covered
    );
    for (command, used) in commands {
        let areas: Vec<&str> = used.areas.iter().map(String::as_str).collect();
        let _ = writeln!(
            out,
            "| `{command}` | {} | {} |",
            areas.join(", "),
            skin_status(command)
        );
    }
    out
}
