//! M9: mpv's on-screen controllers (goal K, `docs/spikes/s6-other-skins.md`).
//!
//! A controller is a Lua script that mpv draws over the video: mpv's own
//! OSC, or the user's scripts (uosc, `ModernZ`, ...) from RVP's mpv folder.
//! RVP's skin stays: the controller lies over the skin's video area (in
//! fullscreen it takes the place of the skin's bars), or, "without the
//! skin", the window holds only the video and the controller. The choice
//! takes effect at the next start; "without the skin" at once.
//!
//! * libmpv reads no config folder (`config=no`), so the scripts, their
//!   options and their fonts go to mpv by path: `scripts` lists the
//!   scripts in `<settings>/mpv/scripts`, `script-opts` the lines of
//!   `<settings>/mpv/script-opts/NAME.conf`, `osd-fonts-dir` names
//!   `<settings>/mpv/fonts`. The folder holds what mpv's own does, so a
//!   script's archive (uosc's) unpacks into it as it is. Whole values:
//!   libmpv's `mpv_set_option_string` knows no `-append` forms.
//! * The pointer over the video goes to mpv (`shell/src/player/m9.rs`):
//!   moves as `mouse X Y` in the video's pixels, entering and leaving as
//!   `MOUSE_ENTER` and `MOUSE_LEAVE`, buttons as `keydown`/`keyup`
//!   (controllers act on the release of what was pressed, so a single
//!   press is lost), wheel turns as `keypress`. Clicks that no controller
//!   takes fall to RVP's bindings in mpv's `default` section, which lies
//!   under every section the scripts enable ([`SECTION`], [`section`]):
//!   they send `key-binding` messages to RVP's client, and RVP acts on
//!   them as on its own pointer ([`pointer_message`]).
//! * Keys stay RVP's, except while a controller's menu is open
//!   (`user-data/mpv/console/open`, `user-data/mpv/context-menu/open`,
//!   `user-data/uosc/menu/type`); then they go to mpv ([`mpv_key`]).
//! * mpv's OSC from 0.40 takes its buttons' commands as options:
//!   previous, next, the menu, the volume, fullscreen and the window's
//!   buttons run RVP's actions (`script-message-to main rvp-action NAME`,
//!   [`OSC_COMMANDS`]). Older OSCs and the other scripts set mpv's
//!   `fullscreen`, `window-maximized`, `window-minimized` and `ontop`,
//!   which RVP follows; RVP writes its window's state back, so that their
//!   buttons show it. Their close button runs `quit`, which ends the core:
//!   RVP closes then.
//! * mpv's own window dragging is off (`window-dragging`): under the render
//!   API it does nothing, and under `wid` it would move mpv's child window.
//!   RVP moves its window itself.
//! * Without a video mpv has no OSD, and no controller shows:
//!   `force-window` gives it one under the render API (the OSC's idle
//!   screen stays off: RVP shows its logo).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::input::{Key, KeyChord};
use crate::pointer::{Button, Modifiers};

/// No controller: RVP's skin alone.
pub const KIND_NONE: &str = "";
/// mpv's own OSC.
pub const KIND_OSC: &str = "osc";
/// The user's scripts in RVP's mpv folder.
pub const KIND_SCRIPTS: &str = "scripts";
pub const KINDS: [&str; 3] = [KIND_NONE, KIND_OSC, KIND_SCRIPTS];

/// mpv's input section for RVP's bindings: the bottom one, so it gets the
/// clicks no controller takes (with `input-default-bindings=no` it holds
/// nothing else that acts).
pub const SECTION: &str = "default";
/// The binding name in its `key-binding` messages.
pub const POINTER_BINDING: &str = "rvp-pointer";
/// The message of the commands RVP gives the OSC's buttons.
pub const ACTION_MESSAGE: &str = "rvp-action";
/// The maximize button: maximize or restore, or leave fullscreen.
pub const MAXIMIZE_MESSAGE: &str = "rvp-maximize";

/// The Skins page's controller choice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ControllerSettings {
    /// One of [`KINDS`].
    pub kind: String,
    /// The window shows only the video and the controller.
    pub without_skin: bool,
}

impl ControllerSettings {
    #[must_use]
    pub fn active(&self) -> bool {
        !self.kind.is_empty()
    }

    /// `--controller=KIND` for one run (the self-test); an unknown kind
    /// is left out.
    pub fn choose(&mut self, kind: &str) {
        if KINDS.contains(&kind) {
            kind.clone_into(&mut self.kind);
        }
    }

    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !KINDS.contains(&self.kind.as_str()) {
            notes.push(format!(
                "controller.kind {:?} is not one of {KINDS:?}: none used",
                self.kind
            ));
            self.kind.clear();
        }
    }
}

/// The OSC's button commands RVP gives it (mpv 0.40 and later; older
/// OSCs ignore the options). The rest keep mpv's.
pub const OSC_COMMANDS: [(&str, &str); 16] = [
    ("playlist_prev_mbtn_left_command", "rvp-action previous"),
    (
        "playlist_prev_mbtn_right_command",
        "rvp-action toggle-playlist",
    ),
    ("playlist_next_mbtn_left_command", "rvp-action next"),
    (
        "playlist_next_mbtn_right_command",
        "rvp-action toggle-playlist",
    ),
    ("menu_mbtn_left_command", "rvp-action context-menu"),
    ("play_pause_mbtn_right_command", "rvp-action repeat-cycle"),
    ("volume_mbtn_left_command", "rvp-action toggle-mute"),
    ("volume_wheel_down_command", "rvp-action volume-down"),
    ("volume_wheel_up_command", "rvp-action volume-up"),
    (
        "fullscreen_mbtn_left_command",
        "rvp-action toggle-fullscreen",
    ),
    (
        "fullscreen_mbtn_right_command",
        "rvp-action maximize-restore",
    ),
    ("close_mbtn_left_command", "rvp-action exit"),
    ("minimize_mbtn_left_command", "rvp-action minimize"),
    ("maximize_mbtn_left_command", "rvp-maximize"),
    // mpv's playlist and audio devices are not RVP's.
    ("play_pause_mbtn_mid_command", ""),
    ("volume_mbtn_right_command", ""),
];

/// The user's scripts in `folder` (`<settings>/mpv`): the `.lua` and
/// `.js` files of its `scripts` folder and its folders that hold a
/// `main.lua` or `main.js`, by name.
#[must_use]
pub fn scripts(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder.join("scripts")) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            if path.is_dir() {
                path.join("main.lua").is_file() || path.join("main.js").is_file()
            } else {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("lua") || ext.eq_ignore_ascii_case("js")
                    })
            }
        })
        .collect();
    found.sort();
    found
}

/// The scripts' options from `folder`'s `script-opts/NAME.conf` files, as
/// mpv's `script-opts` entries `NAME-KEY=VALUE`: one `KEY=VALUE` a line,
/// `#` starts a comment line, as mpv reads them.
#[must_use]
pub fn script_opts(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder.join("script-opts")) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "conf"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let (Some(name), Ok(text)) = (
            file.file_stem().and_then(|stem| stem.to_str()),
            std::fs::read_to_string(&file),
        ) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                out.push(format!("{name}-{}={value}", key.trim()));
            }
        }
    }
    out
}

/// What the player core needs for the controller: whether mpv's own
/// scripts load (the OSC's menus use them), and the options, applied only
/// where this libmpv knows them. `native` is RVP's native video window
/// (no `force-window` there: mpv's window would cover RVP's logo).
#[must_use]
pub fn engine_options(
    settings: &ControllerSettings,
    folder: &Path,
    native: bool,
) -> (bool, Vec<(String, String)>) {
    let mut options = Vec::new();
    let mut push = |name: &str, value: String| options.push((name.to_owned(), value));
    match settings.kind.as_str() {
        KIND_OSC => {
            push("osc", "yes".to_owned());
            let mut opts = vec!["osc-idlescreen=no".to_owned()];
            for (name, command) in OSC_COMMANDS {
                let command = if command.is_empty() {
                    String::new()
                } else {
                    format!("script-message-to main {command}")
                };
                opts.push(format!("osc-{name}={command}"));
            }
            push("script-opts", key_value_list(&opts));
        }
        KIND_SCRIPTS => {
            let paths: Vec<String> = scripts(folder)
                .iter()
                .map(|script| script.to_string_lossy().into_owned())
                .collect();
            push("scripts", paths.join(if cfg!(windows) { ";" } else { ":" }));
            let opts = script_opts(folder);
            if !opts.is_empty() {
                push("script-opts", key_value_list(&opts));
            }
            let fonts = folder.join("fonts");
            if fonts.is_dir() {
                push("osd-fonts-dir", fonts.to_string_lossy().into_owned());
            }
        }
        _ => return (false, options),
    }
    push("window-dragging", "no".to_owned());
    if !native {
        push("force-window", "yes".to_owned());
    }
    if settings.without_skin {
        // The window has no frame: the controllers show their window
        // buttons then.
        push("border", "no".to_owned());
        push("title-bar", "no".to_owned());
    }
    (true, options)
}

/// `KEY=VALUE` entries as one mpv key-value list: each value quoted as
/// `%BYTES%VALUE`, so that commas and spaces stay in it.
fn key_value_list(entries: &[String]) -> String {
    entries
        .iter()
        .map(|entry| {
            let (key, value) = entry.split_once('=').unwrap_or((entry, ""));
            format!("{key}=%{}%{value}", value.len())
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// mpv's names of the pointer's buttons and wheel turns.
const BUTTONS: [&str; 10] = [
    "MBTN_LEFT",
    "MBTN_LEFT_DBL",
    "MBTN_RIGHT",
    "MBTN_MID",
    "MBTN_BACK",
    "MBTN_FORWARD",
    "WHEEL_UP",
    "WHEEL_DOWN",
    "WHEEL_LEFT",
    "WHEEL_RIGHT",
];

/// RVP's bindings for [`SECTION`] (`define-section default TEXT force`):
/// every button and wheel turn, with every combination of modifiers (mpv
/// matches them exactly), sent to RVP's client.
#[must_use]
pub fn section() -> String {
    let mut out = String::new();
    for mask in 0..16u8 {
        let mut prefix = String::new();
        for (bit, name) in ["shift+", "ctrl+", "alt+", "meta+"].iter().enumerate() {
            if mask & (1 << bit) != 0 {
                prefix.push_str(name);
            }
        }
        for button in BUTTONS {
            let _ = writeln!(
                out,
                "{prefix}{button} script-binding main/{POINTER_BINDING}"
            );
        }
    }
    out
}

/// What a button or wheel turn did, from mpv's `key-binding` message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerInput {
    Press(Button),
    Release(Button),
    /// The press was taken from RVP (mpv started its own drag): no click.
    Cancel(Button),
    Double(Button),
    /// Wheel up (positive) or down, in notches.
    Wheel(i32),
}

/// Reads a message of RVP's input section: `key-binding`, the binding
/// name, the state (`d`own, `u`p, `p`ress, `r`epeat; `m` for the pointer;
/// `c` when cancelled) and the key's name with its modifiers.
#[must_use]
pub fn pointer_message(args: &[String]) -> Option<PointerInput> {
    let [kind, name, state, key, ..] = args else {
        return None;
    };
    if kind != "key-binding" || name != POINTER_BINDING {
        return None;
    }
    let key = key.rsplit('+').next().unwrap_or(key).to_ascii_uppercase();
    let cancelled = state.as_bytes().get(2) == Some(&b'c');
    let phase = state.as_bytes().first().copied();
    let button = |name: &str| match name {
        "MBTN_LEFT" | "MBTN_LEFT_DBL" => Some(Button::Left),
        "MBTN_RIGHT" | "MBTN_RIGHT_DBL" => Some(Button::Right),
        "MBTN_MID" | "MBTN_MID_DBL" => Some(Button::Middle),
        "MBTN_BACK" => Some(Button::Back),
        "MBTN_FORWARD" => Some(Button::Forward),
        _ => None,
    };
    match key.as_str() {
        "WHEEL_UP" => return Some(PointerInput::Wheel(1)),
        "WHEEL_DOWN" => return Some(PointerInput::Wheel(-1)),
        "WHEEL_LEFT" | "WHEEL_RIGHT" => return None,
        _ => {}
    }
    let pressed = button(&key)?;
    if key.ends_with("_DBL") {
        return Some(PointerInput::Double(pressed));
    }
    match (phase, cancelled) {
        (Some(b'd'), _) => Some(PointerInput::Press(pressed)),
        (Some(b'u'), true) => Some(PointerInput::Cancel(pressed)),
        (Some(b'u'), false) => Some(PointerInput::Release(pressed)),
        _ => None,
    }
}

/// A command RVP gave the OSC's buttons: the action's settings name, or
/// [`MAXIMIZE_MESSAGE`].
#[must_use]
pub fn command_message(args: &[String]) -> Option<&str> {
    match args {
        [kind, action, ..] if kind == ACTION_MESSAGE => Some(action),
        [kind, ..] if kind == MAXIMIZE_MESSAGE => Some(MAXIMIZE_MESSAGE),
        _ => None,
    }
}

/// mpv's name of a pointer button, with the modifiers held.
#[must_use]
pub fn button_name(button: Button, modifiers: Modifiers) -> String {
    let name = match button {
        Button::Left => "MBTN_LEFT",
        Button::Right => "MBTN_RIGHT",
        Button::Middle => "MBTN_MID",
        Button::Back => "MBTN_BACK",
        Button::Forward => "MBTN_FORWARD",
    };
    format!(
        "{}{name}",
        modifier_prefix(modifiers.shift, modifiers.ctrl, modifiers.alt)
    )
}

fn modifier_prefix(shift: bool, ctrl: bool, alt: bool) -> String {
    let mut prefix = String::new();
    for (held, name) in [(shift, "shift+"), (ctrl, "ctrl+"), (alt, "alt+")] {
        if held {
            prefix.push_str(name);
        }
    }
    prefix
}

/// mpv's name of a key from a Qt key event, for a controller's menu:
/// typed text as it is (type to search), other keys by mpv's names.
#[must_use]
pub fn mpv_key(key: i32, modifiers: i32, text: &str) -> Option<String> {
    let chord = KeyChord::from_qt(key, modifiers).into_iter().next()?;
    let mut chars = text.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && !c.is_control()
        && c != ' '
        && !chord.ctrl
        && !chord.alt
    {
        return Some(match c {
            '#' => "SHARP".to_owned(),
            c => c.to_string(),
        });
    }
    let name = match chord.key {
        Key::Char(c) => c.to_ascii_lowercase().to_string(),
        Key::Punct('#') => "SHARP".to_owned(),
        Key::Punct(c) => c.to_string(),
        Key::F(n) => format!("F{n}"),
        Key::Numpad(n) => format!("KP{n}"),
        Key::NumpadAdd => "KP_ADD".to_owned(),
        Key::NumpadSubtract => "KP_SUBTRACT".to_owned(),
        Key::NumpadMultiply => "KP_MULTIPLY".to_owned(),
        Key::NumpadDivide => "KP_DIVIDE".to_owned(),
        Key::NumpadDecimal => "KP_DEC".to_owned(),
        Key::Up => "UP".to_owned(),
        Key::Down => "DOWN".to_owned(),
        Key::Left => "LEFT".to_owned(),
        Key::Right => "RIGHT".to_owned(),
        Key::PageUp => "PGUP".to_owned(),
        Key::PageDown => "PGDWN".to_owned(),
        Key::Home => "HOME".to_owned(),
        Key::End => "END".to_owned(),
        Key::Insert => "INS".to_owned(),
        Key::Delete => "DEL".to_owned(),
        Key::Enter => "ENTER".to_owned(),
        Key::Space => "SPACE".to_owned(),
        Key::Escape => "ESC".to_owned(),
        Key::Tab => "TAB".to_owned(),
        Key::Backspace => "BS".to_owned(),
        _ => return None,
    };
    Some(format!(
        "{}{name}",
        modifier_prefix(chord.shift, chord.ctrl, chord.alt)
    ))
}

/// The window state RVP and the controllers share through mpv's options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // mpv's four window options.
pub struct WindowState {
    pub fullscreen: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub on_top: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn messages_of_rvps_section_become_pointer_input() {
        let message = |state: &str, key: &str| {
            pointer_message(&strings(&["key-binding", POINTER_BINDING, state, key, ""]))
        };
        // mpv 0.37 sends two letters, later ones three.
        assert_eq!(
            message("dm", "MBTN_LEFT"),
            Some(PointerInput::Press(Button::Left))
        );
        assert_eq!(
            message("um-", "MBTN_LEFT"),
            Some(PointerInput::Release(Button::Left))
        );
        assert_eq!(
            message("umc", "MBTN_LEFT"),
            Some(PointerInput::Cancel(Button::Left))
        );
        assert_eq!(
            message("p-", "MBTN_LEFT_DBL"),
            Some(PointerInput::Double(Button::Left))
        );
        assert_eq!(
            message("dm", "shift+MBTN_RIGHT"),
            Some(PointerInput::Press(Button::Right))
        );
        assert_eq!(message("pm", "WHEEL_DOWN"), Some(PointerInput::Wheel(-1)));
        assert_eq!(message("pm", "Ctrl+WHEEL_UP"), Some(PointerInput::Wheel(1)));
        assert_eq!(message("pm", "WHEEL_LEFT"), None);
        assert_eq!(
            pointer_message(&strings(&["key-binding", "other", "dm", "MBTN_LEFT", ""])),
            None
        );
    }

    #[test]
    fn the_section_binds_every_button_with_every_modifier() {
        let section = section();
        assert_eq!(section.lines().count(), 160);
        assert!(
            section
                .lines()
                .any(|line| line == "MBTN_LEFT script-binding main/rvp-pointer")
        );
        assert!(section.contains("shift+ctrl+alt+meta+WHEEL_DOWN script-binding main/rvp-pointer"));
    }

    #[test]
    fn commands_for_rvp_are_recognised() {
        assert_eq!(
            command_message(&strings(&["rvp-action", "next"])),
            Some("next")
        );
        assert_eq!(
            command_message(&strings(&["rvp-maximize"])),
            Some(MAXIMIZE_MESSAGE)
        );
        assert_eq!(command_message(&strings(&["osc-visibility", "auto"])), None);
    }

    #[test]
    fn keys_get_mpvs_names() {
        const SHIFT: i32 = 0x0200_0000;
        const CTRL: i32 = 0x0400_0000;
        assert_eq!(mpv_key(0x0100_0013, 0, "").as_deref(), Some("UP"));
        assert_eq!(mpv_key(0x0100_0004, 0, "\r").as_deref(), Some("ENTER"));
        assert_eq!(mpv_key(0x0100_0000, 0, "\u{1b}").as_deref(), Some("ESC"));
        assert_eq!(mpv_key(i32::from(b'A'), 0, "a").as_deref(), Some("a"));
        assert_eq!(mpv_key(i32::from(b'A'), SHIFT, "A").as_deref(), Some("A"));
        assert_eq!(
            mpv_key(i32::from(b'A'), CTRL, "\u{1}").as_deref(),
            Some("ctrl+a")
        );
        assert_eq!(mpv_key(0x20, 0, " ").as_deref(), Some("SPACE"));
        assert_eq!(mpv_key(0x0100_0031, 0, "").as_deref(), Some("F2"));
        assert_eq!(
            mpv_key(i32::from(b'#'), SHIFT, "#").as_deref(),
            Some("SHARP")
        );
        assert_eq!(mpv_key(0x0100_0020, SHIFT, "").as_deref(), None);
    }

    #[test]
    fn buttons_carry_their_modifiers() {
        let modifiers = Modifiers {
            ctrl: true,
            shift: true,
            alt: false,
        };
        assert_eq!(button_name(Button::Left, modifiers), "shift+ctrl+MBTN_LEFT");
        assert_eq!(
            button_name(Button::Middle, Modifiers::default()),
            "MBTN_MID"
        );
    }

    #[test]
    fn the_osc_gets_rvps_commands_and_scripts_their_files() {
        let folder = std::env::temp_dir().join(format!("rvp-controller-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("scripts/uosc")).unwrap();
        std::fs::create_dir_all(folder.join("scripts/empty")).unwrap();
        std::fs::create_dir_all(folder.join("script-opts")).unwrap();
        std::fs::create_dir_all(folder.join("fonts")).unwrap();
        std::fs::write(folder.join("scripts/uosc/main.lua"), "").unwrap();
        std::fs::write(folder.join("scripts/modernz.lua"), "").unwrap();
        std::fs::write(folder.join("scripts/readme.txt"), "").unwrap();
        std::fs::write(
            folder.join("script-opts/uosc.conf"),
            "# comment\ntimeline_size=40\n\ncontrols=menu,gap\n",
        )
        .unwrap();

        let none = ControllerSettings::default();
        assert_eq!(engine_options(&none, &folder, false), (false, Vec::new()));

        let osc = ControllerSettings {
            kind: KIND_OSC.to_owned(),
            without_skin: true,
        };
        let (builtin, options) = engine_options(&osc, &folder, true);
        assert!(builtin);
        let has = |name: &str, value: &str| options.iter().any(|(n, v)| n == name && v == value);
        assert!(has("osc", "yes"));
        let opts = &options
            .iter()
            .find(|(name, _)| name == "script-opts")
            .unwrap()
            .1;
        assert!(opts.starts_with("osc-idlescreen=%2%no,"));
        assert!(
            opts.contains(
                ",osc-close_mbtn_left_command=%38%script-message-to main rvp-action exit,"
            )
        );
        assert!(opts.ends_with(",osc-volume_mbtn_right_command=%0%"));
        assert!(has("border", "no"));
        assert!(has("window-dragging", "no"));
        assert!(!options.iter().any(|(name, _)| name == "force-window"));

        let scripts = ControllerSettings {
            kind: KIND_SCRIPTS.to_owned(),
            without_skin: false,
        };
        let (_, options) = engine_options(&scripts, &folder, false);
        let list = &options
            .iter()
            .find(|(name, _)| name == "scripts")
            .unwrap()
            .1;
        let separator = if cfg!(windows) { ';' } else { ':' };
        let listed: Vec<&str> = list.split(separator).collect();
        assert_eq!(listed.len(), 2);
        assert!(listed[0].ends_with("modernz.lua"));
        assert!(listed[1].ends_with("uosc"));
        assert!(options.contains(&(
            "script-opts".to_owned(),
            "uosc-timeline_size=%2%40,uosc-controls=%8%menu,gap".to_owned()
        )));
        assert!(
            options
                .iter()
                .any(|(name, value)| name == "osd-fonts-dir" && value.ends_with("fonts"))
        );
        assert!(options.contains(&("force-window".to_owned(), "yes".to_owned())));
        assert!(
            !options
                .iter()
                .any(|(name, _)| name == "osc" || name == "border")
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn an_unknown_kind_is_turned_off() {
        let mut settings = ControllerSettings {
            kind: "winamp".to_owned(),
            without_skin: false,
        };
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert!(settings.kind.is_empty());
        assert_eq!(notes.len(), 1);
    }
}
