//! The dispatcher's coverage of PotPlayer's commands (M4): which items of
//! PotPlayer's menus (R1) and which commands of a skin RVP carries out, and
//! why the others are not. `tests/coverage.rs` writes the reports to
//! `docs/coverage/` (`RVP_BLESS=1`) and checks that they are current.

pub mod preferences;

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rvp_skin::menu::CommandUse;

use crate::action::{Action, ListAction};
use crate::menu::{PopupSource, Recorded, context_tree, playlist_tree, popup_source};

/// The owner's decision (2026-10-16) on M13's online part.
pub(crate) const ONLINE_SUBTITLES: &str = "online subtitles: deferred by the owner (2026-10-16)";

/// Why RVP has none of Purfview's faster-whisper builds.
const FASTER_WHISPER: &str = "Purfview's faster-whisper builds come without a licence, as 1.4 GB \
     bundles for NVIDIA's CUDA or the CPU that fetch their models online; whisper.cpp's CUDA, \
     Vulkan and CPU engines run the same models offline (docs/spikes/m13-whisper.md)";

/// Why RVP has no fade for subtitles.
pub(crate) const SUBTITLE_FADE: &str = "libass fades only ASS events that ask for it (\\fad); mpv \
     gives other subtitles no fade, and an ASS copy of a file would lose RVP's style settings";

/// Why parts of PotPlayer's context menu (resource 101) are not in RVP, by
/// submenu path; the longest matching path wins.
const CONTEXT_REASONS: &[(&str, &str)] = &[
    ("101_0", "not in RVP yet"),
    (
        "101_0_1",
        "TV tuners (WDM analog and BDA digital TV) are a non-goal: libmpv has no tuner input",
    ),
    (
        "101_0_1_4",
        "PotPlayer's file navigator is not in RVP (docs/spikes/m11-studies.md): Open File(s), Open URL and Open Folder are the choices",
    ),
    (
        "101_0_1_5",
        "PotPlayer's file navigator and Open Box are not in RVP (docs/spikes/m11-studies.md): the system's file dialogs, Open Folder, favorites and recent items open files",
    ),
    (
        "101_0_2",
        "Daum's broadcast and chat services (discontinued) and PotPlayer's screen capture are not planned",
    ),
    (
        "101_0_6",
        "Windows Media Player's visualization plugins and PotPlayer's floating ball are their programs' own; RVP shows FFmpeg's visualizations",
    ),
    ("101_0_7", "TV devices are not planned"),
    (
        "101_0_8",
        "DVD and Blu-ray menus are M16's (goal R); RVP plays a disc's titles, chapters and streams",
    ),
    ("101_0_9", "not in RVP yet"),
    (
        "101_0_9_12",
        "seamless playback is not in RVP (docs/spikes/m11-studies.md): mpv preloads and joins files only inside its own playlist, and RVP's albums give mpv one entry at a time",
    ),
    ("101_0_10", "not in RVP yet"),
    ("101_0_10_8", "mpv (libass) renders every subtitle"),
    (
        "101_0_10_12",
        "mpv draws subtitles once over the finished picture, not once for each eye (docs/spikes/m12-studies.md)",
    ),
    ("101_0_10_13", ONLINE_SUBTITLES),
    ("101_0_11", "not in RVP yet"),
    (
        "101_0_11_1",
        "DirectShow's video renderers do not apply to libmpv: RVP's renderer (OpenGL in its window or Direct3D 11 in a window of its own, D4) is chosen in Preferences > Video and taken when RVP starts",
    ),
    (
        "101_0_11_2",
        "mpv's shaders always run on the video image only",
    ),
    (
        "101_0_11_3",
        "frame-sequential 3D (shutter glasses, NVIDIA 3D Vision, Windows' stereoscopic 3D) needs a stereo swap chain and the GPU's 3D driver; libmpv's render API draws one picture into Qt's window",
    ),
    (
        "101_0_11_3_13",
        "mpv draws subtitles once over the finished picture, not once for each eye (docs/spikes/m12-studies.md)",
    ),
    (
        "101_0_11_4",
        "DirectShow transform filters do not apply to libmpv",
    ),
    (
        "101_0_11_6",
        "mpv scales the video (the scaler is a setting)",
    ),
    (
        "101_0_11_9",
        "PotPlayer's DirectShow transform filter; RVP's bottom margin is FFmpeg's pad filter, which has no such failure",
    ),
    (
        "101_0_11_14",
        "PotPlayer's AviSynth processing needs a bridge RVP does not have (docs/spikes/m12-studies.md: avsw runs a 32-bit AviSynth in a second process, 8-bit video only); RVP opens .avs files, runs VapourSynth scripts and loads AviSynth plugins in them",
    ),
    ("101_0_13", "DirectShow filters do not apply to libmpv"),
    ("101_0_15", "not in RVP yet"),
    ("101_0_15_2", "RVP's interface is English"),
    (
        "101_0_19",
        "Desktop Mode puts the window into Explorer's desktop layer through an undocumented message, which Windows 11 24H2 changed, and leaves it no keyboard or mouse (docs/spikes/m12-studies.md)",
    ),
];

/// Why parts of PotPlayer's playlist menu (resource 111) are not in RVP.
const PLAYLIST_REASONS: &[(&str, &str)] = &[("111_0", "not in RVP's albums yet")];

/// PotPlayer's ways to draw the on-screen control besides its Direct3D 11
/// one.
const OSC_METHODS: &str = "RVP draws the on-screen control one way, with Qt over the video (the Direct3D 11 item); Direct3D 9 and layered windows are PotPlayer's renderers' ways";

/// Reasons for single items.
const ITEM_REASONS: &[(u32, &str)] = &[
    (10187, "Daum's login, discontinued"),
    (
        10800,
        "thumbnail images of the playlist's entries are M14's (goal P); RVP makes the playing file's (Video Capture > Create Thumbnail Image, Alt+N)",
    ),
    (11096, ONLINE_SUBTITLES),
    (10220, "DirectShow output surfaces do not apply to libmpv"),
    (10221, "DirectShow output surfaces do not apply to libmpv"),
    (10828, "DirectShow output surfaces do not apply to libmpv"),
    (10825, "mpv (libass) always animates ASS subtitles"),
    (10827, SUBTITLE_FADE),
    (
        11384,
        "Const-me's Whisper (DirectCompute, Windows only, MPL-2.0) has had no release since 2023 and reads only older whisper.cpp models; whisper.cpp's Vulkan engine runs on every GPU (docs/spikes/m13-whisper.md)",
    ),
    (11385, FASTER_WHISPER),
    (11386, FASTER_WHISPER),
    (
        10454,
        "DirectShow filters do not apply to libmpv (its functions are FFmpeg's filters in RVP's chain)",
    ),
    (
        11025,
        "mpv converts YCgCo video from the file's tags itself",
    ),
    (10644, "presets are not in RVP yet"),
    (10809, "touch input is not planned"),
    (
        20009,
        "a submenu in PotPlayer's running menu: Skins > Opacity > Main Window lists the percentages (39600-39609)",
    ),
    (10515, OSC_METHODS),
    (10516, OSC_METHODS),
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

/// The menu items PotPlayer fills at run time and RVP fills with its own
/// list.
fn runtime_list(id: u32) -> Option<&'static str> {
    Some(match id {
        20005 => "the output speakers (PotPlayer's 26)",
        20006 => "the output formats, sample rates and resampler qualities",
        20007 => "the output modes",
        20008 => "the equalizer's presets",
        20001 => "the disc's titles (DVD Control > Move, from the disc's title table)",
        20004 => "the DVD title's angles (mpv's dvd-angle)",
        20002 => "the audio tracks",
        20003 => "the subtitle tracks",
        37600 | 37650 | 37700 => "mpv's audio devices",
        _ => return None,
    })
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
    // A submenu RVP fills (its key, H, opens it too).
    if item.id == 10324 {
        return ItemStatus::Runtime("the file's chapters and bookmarks");
    }
    if let Some(action) = Action::from_potplayer_id(item.id) {
        return ItemStatus::Window(action);
    }
    if let Some(action) = ListAction::from_potplayer_id(item.id) {
        return ItemStatus::List(action);
    }
    if let Some(what) = runtime_list(item.id) {
        return ItemStatus::Runtime(what);
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
    runtime_section(&mut out);
    out
}

const RUNTIME_TSV: &str = include_str!("../../../docs/r1/menu-runtime.tsv");

/// Lists PotPlayer builds at run time (devices, presets, files): RVP's
/// counterpart, by the list's place. A list counts once.
const RUNTIME_LISTS: &[(&str, &str)] = &[
    ("Audio > Audio Renderer", "mpv's audio devices"),
    (
        "Audio > Audio Renderer (Pass-through)",
        "mpv's audio devices",
    ),
    ("Audio > Audio Renderer (2nd Device)", "mpv's audio devices"),
    (
        "Audio > Output Speakers",
        "RVP's list of PotPlayer's 26 speaker choices",
    ),
    (
        "Audio > Output Format/Resample",
        "RVP's formats, sample rates and resampler qualities",
    ),
    (
        "Audio > Output Mode",
        "RVP's output modes (PotPlayer's LPCM is PCM to mpv)",
    ),
    ("Audio > Equalizer Presets", "PotPlayer's nineteen presets"),
    ("Album/Favorites", "RVP's albums"),
    (
        "Video > Pixel Shaders > Pre-Resize Pixel Shaders",
        "RVP's shaders: its own set, PotPlayer's translated and the user's",
    ),
    (
        "Video > Pixel Shaders > Post-Resize Pixel Shaders",
        "RVP's shaders: its own set, PotPlayer's translated and the user's",
    ),
    ("Video > VapourSynth", "RVP's VapourSynth script slots"),
    (
        "Subtitles > Create subtitles from audio > Whisper AI engine",
        "RVP's whisper.cpp engines (CPU, BLAS, Vulkan, CUDA)",
    ),
    (
        "Subtitles > Create subtitles from audio > Whisper AI model",
        "whisper.cpp's models (ggml-*.bin in RVP's model folder)",
    ),
    (
        "Subtitles > Create subtitles from audio > Whisper AI language",
        "whisper's languages",
    ),
    (
        "Subtitles > Create subtitles from audio(Real-time) > Whisper AI engine",
        "RVP's whisper.cpp engines (CPU, BLAS, Vulkan, CUDA)",
    ),
    (
        "Subtitles > Create subtitles from audio(Real-time) > Whisper AI model",
        "whisper.cpp's models (ggml-*.bin in RVP's model folder)",
    ),
    (
        "Subtitles > Create subtitles from audio(Real-time) > Whisper AI language",
        "whisper's languages",
    ),
    ("Frame Size > Presets", "RVP's frame size presets"),
    (
        "Skins > Color Themes",
        "the skin's colour themes (its ColorTheme list)",
    ),
    (
        "Skins > Logo (while playback is off)",
        "the pictures of RVP's logo folders, for the idle logo",
    ),
    (
        "Skins > Logo (while starting playback)",
        "the pictures of RVP's logo folders, for the start logo",
    ),
    (
        "Skins > Context Menu Design",
        "the menu files of RVP's menus folder and PotPlayer's",
    ),
];

/// Run-time lists by their IDs, where their place holds other items too.
const RUNTIME_LIST_IDS: &[(std::ops::RangeInclusive<u32>, &str)] = &[(
    44_700..=44_799,
    "RVP's skins: RVP Default and the .dsf files of its skins folder, the chosen skin's folder and PotPlayer's",
)];

/// Why run-time commands are not in RVP, by place; the longest wins.
const RUNTIME_REASONS: &[(&str, &str)] = &[
    (
        "Playback > Speed",
        "RVP stretches with mpv's filters (Speed's three items)",
    ),
    (
        "Playback > Chapter/Bookmark",
        "RVP keeps bookmarks in its state folder (marks.json); PotPlayer's .pbf format was not recorded (R1)",
    ),
    ("Subtitles > Subtitle Translation", ONLINE_SUBTITLES),
    ("Subtitles > Subtitle Searching", ONLINE_SUBTITLES),
    (
        "Subtitles > Subtitle Font Renderer",
        "mpv (libass) renders every subtitle",
    ),
    (
        "Filters",
        "DirectShow filters do not apply to libmpv: FFmpeg decodes and splits (the Decoders page chooses hardware decoding per codec)",
    ),
    (
        "Video > Video Renderer",
        "DirectShow's video renderers do not apply to libmpv: RVP's renderer (OpenGL in its window or Direct3D 11 in a window of its own, D4) is chosen in Preferences > Video and taken when RVP starts",
    ),
    (
        "Video > Pixel Shaders",
        "mpv's shaders always run on the video image only",
    ),
    (
        "Video > 3D Video Mode",
        "FFmpeg decodes only H.264 MVC's base view (docs/spikes/m12-studies.md); 3D files side by side or top and bottom play through stereo3d",
    ),
    (
        "Video > AviSynth",
        "PotPlayer's AviSynth processing needs a bridge RVP does not have (docs/spikes/m12-studies.md): its presets do not run; RVP opens .avs files and runs VapourSynth scripts",
    ),
    ("Misc", "planned in M14 (goal P)"),
    ("Skins", "planned in M14 (goal P)"),
];

/// One command of R1's live menu that the static tree lacks: its place and
/// ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommand {
    pub place: String,
    /// The place of the submenu it is in.
    pub parent: String,
    pub id: u32,
}

/// The live menu's commands whose IDs are in neither static tree.
#[must_use]
pub fn runtime_commands() -> Vec<RuntimeCommand> {
    let mut known = std::collections::BTreeSet::new();
    for tree in [context_tree(), playlist_tree()] {
        crate::menu::walk(tree, &mut |item| {
            known.insert(item.id);
        });
    }
    let mut path: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for line in RUNTIME_TSV.lines().skip(1) {
        let cells: Vec<&str> = line.split('\t').collect();
        let (Some(depth), Some(id), Some(text)) = (
            cells.first().and_then(|d| d.parse::<usize>().ok()),
            cells.get(1).and_then(|i| i.parse::<u32>().ok()),
            cells.get(2),
        ) else {
            continue;
        };
        let text = text.trim().replace("&&", "&");
        path.truncate(depth);
        path.push(text.clone());
        let submenu = cells.get(4).is_some_and(|flags| flags.contains("submenu"));
        if submenu || id == 0 || text == "---" || known.contains(&id) {
            continue;
        }
        out.push(RuntimeCommand {
            place: path.join(" > "),
            parent: path[..path.len() - 1].join(" > "),
            id,
        });
    }
    out
}

/// What RVP does with a run-time command.
#[must_use]
pub fn runtime_status(command: &RuntimeCommand) -> ItemStatus {
    if let Some(action) = Action::from_potplayer_id(command.id) {
        return ItemStatus::Window(action);
    }
    if let Some(action) = ListAction::from_potplayer_id(command.id) {
        return ItemStatus::List(action);
    }
    if let Some((_, reason)) = ITEM_REASONS.iter().find(|(id, _)| *id == command.id) {
        return ItemStatus::Missing(reason);
    }
    if let Some((_, what)) = RUNTIME_LISTS.iter().find(|(place, _)| {
        command.parent == *place || command.parent.starts_with(&format!("{place} > "))
    }) {
        return ItemStatus::Runtime(what);
    }
    if let Some((_, what)) = RUNTIME_LIST_IDS
        .iter()
        .find(|(ids, _)| ids.contains(&command.id))
    {
        return ItemStatus::Runtime(what);
    }
    ItemStatus::Missing(
        RUNTIME_REASONS
            .iter()
            .filter(|(place, _)| {
                command.place == *place || command.place.starts_with(&format!("{place} > "))
            })
            .max_by_key(|(place, _)| place.len())
            .map_or("not in RVP yet", |(_, reason)| reason),
    )
}

fn runtime_section(out: &mut String) {
    let commands = runtime_commands();
    let mut done = Vec::new();
    let mut lists: BTreeMap<&str, (usize, &str)> = BTreeMap::new();
    let mut missing: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for command in &commands {
        match runtime_status(command) {
            ItemStatus::Window(action) => done.push((command, action.name().to_owned())),
            ItemStatus::List(action) => {
                done.push((command, format!("playlist: {}", action.name())));
            }
            ItemStatus::Runtime(what) => {
                let list = RUNTIME_LISTS
                    .iter()
                    .find(|(_, w)| *w == what)
                    .map_or(command.parent.as_str(), |(place, _)| place);
                lists.entry(list).or_insert((0, what)).0 += 1;
            }
            ItemStatus::Missing(why) => missing
                .entry(why)
                .or_default()
                .push(format!("{} ({})", command.place, command.id)),
        }
    }
    let _ = writeln!(
        out,
        "\n## Run-time commands (R1's live menu)\n\n{} commands of the live menu \
         (`docs/r1/menu-runtime.tsv`) are in neither static tree: {} carried out, {} in \
         lists RVP fills itself (each list counts as one row), {} not.\n\n\
         | Item | ID | RVP |\n|------|----|-----|",
        commands.len(),
        done.len(),
        lists.values().map(|(count, _)| count).sum::<usize>(),
        missing.values().map(Vec::len).sum::<usize>()
    );
    for (command, what) in done {
        let _ = writeln!(out, "| {} | {} | `{what}` |", command.place, command.id);
    }
    for (place, (count, what)) in &lists {
        let _ = writeln!(out, "| {place} ({count} entries) | list | {what} |");
    }
    let _ = writeln!(out, "\nNot carried out, by reason:\n");
    for (why, places) in missing {
        let _ = writeln!(out, "- **{why}** ({}): {}", places.len(), places.join("; "));
    }
}

const COMMAND_LINE_TSV: &str = include_str!("../../../docs/r1/command-line.tsv");

/// What RVP does with each of PotPlayer's command-line switches (R2), by
/// the switch as R2 wrote it.
const COMMAND_LINE: &[(&str, Result<&str, &str>)] = &[
    (
        "\"content\"",
        Ok(
            "files, folders and URLs open in the shown album (When Files Opened applies); wildcards and URL titles after a backslash are not read",
        ),
    ),
    ("-", Ok("standard input plays (mpv's `-`)")),
    ("/filedlg", Ok("the Open File(s) dialog")),
    ("/urldlg", Ok("the Open URL dialog")),
    ("/folderdlg", Ok("the Open Folder dialog")),
    (
        "/simple",
        Err("PotPlayer's file navigator is not in RVP (docs/spikes/m11-studies.md)"),
    ),
    ("/cap", Ok("the screen capture")),
    (
        "/cam",
        Ok("the webcam or device (the device setting, or a list)"),
    ),
    ("/atv[:ch]", Err("TV tuners are a non-goal")),
    ("/dtv[:ch[-idx]]", Err("TV tuners are a non-goal")),
    (
        "/dvd",
        Ok("the DVD (the device setting, or the disc dialog)"),
    ),
    ("/cd[:drive]", Ok("the audio CD (cdda://) of the drive")),
    ("/add", Ok("appends to the shown album")),
    ("/insert", Ok("inserts below the entry that plays")),
    ("/autoplay", Ok("the last played item plays")),
    (
        "/same",
        Ok("the files after the first are its streams (external files)"),
    ),
    ("/sort", Ok("the files sort by name first")),
    ("/randomize", Ok("the files are shuffled first")),
    ("/new", Ok("another RVP plays them")),
    ("/current", Ok("the running RVP plays them")),
    ("/clipboard", Ok("the clipboard's files and URLs open")),
    (
        "/seek=time",
        Ok("the first file starts there (mpv's start)"),
    ),
    (
        "/sub=\"subfile\"",
        Ok("the first file's subtitle (mpv's sub-files)"),
    ),
    (
        "/aud=\"audiofile\"",
        Ok("the first file's audio (mpv's audio-files)"),
    ),
    ("/user_agent=\"ua\"", Ok("the first file's user agent")),
    ("/referer=\"ref\"", Ok("the first file's referer")),
    (
        "/headers=\"hdrs\"",
        Ok("the first file's HTTP header fields"),
    ),
    (
        "/title=\"title\"",
        Ok("the first file's title (mpv's force-media-title)"),
    ),
    ("config=\"name\"", Err("presets are M14's (goal P)")),
];

/// What RVP does with a switch of R2's list; `None` for one RVP has not
/// sorted out (the test fails on it).
#[must_use]
pub fn command_line_status(switch: &str) -> Option<Result<&'static str, &'static str>> {
    COMMAND_LINE
        .iter()
        .find(|(name, _)| *name == switch)
        .map(|(_, status)| *status)
}

/// The switches of R2's `docs/r1/command-line.tsv`.
#[must_use]
pub fn command_line_switches() -> Vec<&'static str> {
    COMMAND_LINE_TSV
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').next())
        .filter(|switch| !switch.is_empty())
        .collect()
}

/// `docs/coverage/command-line.md`.
#[must_use]
pub fn command_line_report() -> String {
    let switches = command_line_switches();
    let done = switches
        .iter()
        .filter(|switch| matches!(command_line_status(switch), Some(Ok(_))))
        .count();
    let mut out = format!(
        "# PotPlayer's command line in RVP\n\nGenerated by `cargo test -p rvp-app --test coverage` \
         (`RVP_BLESS=1` rewrites it) from R2's `docs/r1/command-line.tsv` \
         (`crates/app/src/instance.rs` reads them).\n\n{} switches: {done} in RVP, {} not.\n\n\
         | Switch | RVP |\n|--------|-----|\n",
        switches.len(),
        switches.len() - done
    );
    for switch in switches {
        let what = match command_line_status(switch) {
            Some(Ok(what)) => what.to_owned(),
            Some(Err(why)) => format!("not: {why}"),
            None => "not: not sorted out".to_owned(),
        };
        let _ = writeln!(out, "| `{switch}` | {what} |");
    }
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
            PopupSource::Live(titles) => format!("menu: PotPlayer's live {}", titles.join(" > ")),
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
