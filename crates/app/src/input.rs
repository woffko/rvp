//! Keyboard and mouse bindings with PotPlayer's defaults.
//!
//! Keys are platform-neutral; the shell translates Qt key events. The
//! default keymap is PotPlayer's main-window accelerator table (table 157 in
//! `docs/r1/hotkeys.tsv`, read from PotPlayer's resources in R1): every key
//! whose command has an [`Action`] is bound. The default mouse map is the
//! owner's PotPlayer mouse page (`docs/r1/mouse-actions.tsv`).

use std::collections::BTreeMap;
use std::fmt;

use crate::action::{Action, SeekStep};

/// A key, independent of modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    /// A letter or digit on the main keyboard (upper case for letters).
    Char(char),
    /// A punctuation character (`;`, `[`, and shifted ones such as `{` that
    /// PotPlayer's table names by character).
    Punct(char),
    /// F1 to F24.
    F(u8),
    /// Numeric keypad digit.
    Numpad(u8),
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Insert,
    Delete,
    Enter,
    Space,
    Escape,
    Tab,
    Backspace,
    Pause,
    PrintScreen,
    ScrollLock,
    MediaPlayPause,
    MediaStop,
    MediaNext,
    MediaPrevious,
    VolumeUp,
    VolumeDown,
    VolumeMute,
    /// A Windows virtual-key code without a name here (`VK_17`).
    Other(u16),
}

/// A key with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: Key,
}

impl KeyChord {
    #[must_use]
    pub const fn plain(key: Key) -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            key,
        }
    }

    /// Parses PotPlayer's key names as written in `hotkeys.tsv`
    /// (`Ctrl+Alt+Left`, `Num0`, `Space`, `VK_17`, `;`).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut chord = Self::plain(Key::Space);
        let mut rest = text;
        // Modifiers are prefixes; what remains is the key name, which may
        // itself contain '+' ("Num+").
        loop {
            if let Some(after) = rest.strip_prefix("Ctrl+") {
                chord.ctrl = true;
                rest = after;
            } else if let Some(after) = rest.strip_prefix("Alt+") {
                chord.alt = true;
                rest = after;
            } else if let Some(after) = rest.strip_prefix("Shift+") {
                chord.shift = true;
                rest = after;
            } else {
                break;
            }
        }
        chord.key = parse_key(rest)?;
        Some(chord)
    }
}

fn parse_key(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    if let (Some(single), None) = (chars.next(), chars.next()) {
        return if single.is_ascii_alphanumeric() {
            Some(Key::Char(single.to_ascii_uppercase()))
        } else if single.is_ascii_punctuation() {
            Some(Key::Punct(single))
        } else {
            None
        };
    }
    if let Some(number) = name.strip_prefix('F')
        && let Ok(n) = number.parse::<u8>()
    {
        return (1..=24).contains(&n).then_some(Key::F(n));
    }
    if let Some(code) = name.strip_prefix("VK_") {
        return u16::from_str_radix(code, 16).ok().map(Key::Other);
    }
    if let Some(digit) = name.strip_prefix("Num")
        && let Ok(n) = digit.parse::<u8>()
    {
        return (n <= 9).then_some(Key::Numpad(n));
    }
    Some(match name {
        "Num+" => Key::NumpadAdd,
        "Num-" => Key::NumpadSubtract,
        "Num*" => Key::NumpadMultiply,
        "Num/" => Key::NumpadDivide,
        "Num." => Key::NumpadDecimal,
        "Up" => Key::Up,
        "Down" => Key::Down,
        "Left" => Key::Left,
        "Right" => Key::Right,
        "PgUp" => Key::PageUp,
        "PgDn" => Key::PageDown,
        "Home" => Key::Home,
        "End" => Key::End,
        "Ins" => Key::Insert,
        "Del" => Key::Delete,
        "Enter" => Key::Enter,
        "Space" => Key::Space,
        "Esc" => Key::Escape,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Pause" => Key::Pause,
        "PrintScreen" => Key::PrintScreen,
        "ScrollLock" => Key::ScrollLock,
        "MediaPlayPause" => Key::MediaPlayPause,
        "MediaStop" => Key::MediaStop,
        "MediaNext" => Key::MediaNext,
        "MediaPrev" => Key::MediaPrevious,
        "VolumeUp" => Key::VolumeUp,
        "VolumeDown" => Key::VolumeDown,
        "VolumeMute" => Key::VolumeMute,
        _ => return None,
    })
}

impl fmt::Display for KeyChord {
    /// PotPlayer's spelling, as shown in its menus (`Ctrl+Alt+Left`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        match self.key {
            Key::Char(c) | Key::Punct(c) => write!(f, "{c}"),
            Key::F(n) => write!(f, "F{n}"),
            Key::Numpad(n) => write!(f, "Num{n}"),
            Key::Other(code) => write!(f, "VK_{code:02X}"),
            Key::NumpadAdd => f.write_str("Num+"),
            Key::NumpadSubtract => f.write_str("Num-"),
            Key::NumpadMultiply => f.write_str("Num*"),
            Key::NumpadDivide => f.write_str("Num/"),
            Key::NumpadDecimal => f.write_str("Num."),
            Key::PageUp => f.write_str("PgUp"),
            Key::PageDown => f.write_str("PgDn"),
            Key::Insert => f.write_str("Ins"),
            Key::Delete => f.write_str("Del"),
            Key::Escape => f.write_str("Esc"),
            Key::MediaPrevious => f.write_str("MediaPrev"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// A mouse gesture on the window (PotPlayer's mouse page).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseGesture {
    LeftClick,
    LeftDoubleClick,
    RightClick,
    RightDoubleClick,
    MiddleClick,
    MiddleDoubleClick,
    X1Click,
    X1DoubleClick,
    X2Click,
    X2DoubleClick,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

const GESTURE_NAMES: &[(MouseGesture, &str)] = &[
    (MouseGesture::LeftClick, "left-click"),
    (MouseGesture::LeftDoubleClick, "left-double-click"),
    (MouseGesture::RightClick, "right-click"),
    (MouseGesture::RightDoubleClick, "right-double-click"),
    (MouseGesture::MiddleClick, "middle-click"),
    (MouseGesture::MiddleDoubleClick, "middle-double-click"),
    (MouseGesture::X1Click, "x1-click"),
    (MouseGesture::X1DoubleClick, "x1-double-click"),
    (MouseGesture::X2Click, "x2-click"),
    (MouseGesture::X2DoubleClick, "x2-double-click"),
    (MouseGesture::WheelUp, "wheel-up"),
    (MouseGesture::WheelDown, "wheel-down"),
    (MouseGesture::WheelLeft, "wheel-left"),
    (MouseGesture::WheelRight, "wheel-right"),
];

impl MouseGesture {
    /// The name used in settings files.
    #[must_use]
    pub fn name(self) -> &'static str {
        GESTURE_NAMES
            .iter()
            .find(|&&(gesture, _)| gesture == self)
            .map_or("", |&(_, name)| name)
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        GESTURE_NAMES
            .iter()
            .find(|&&(_, candidate)| candidate == name)
            .map(|&(gesture, _)| gesture)
    }
}

/// Key and mouse bindings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bindings {
    pub keys: BTreeMap<KeyChord, Action>,
    pub mouse: BTreeMap<MouseGesture, Action>,
    /// PotPlayer: "Operate the action 'Left Button - Single Click' in the
    /// video area only" (on for the owner).
    pub left_click_video_only: bool,
}

/// PotPlayer's default hotkey table (R1).
const HOTKEYS: &str = include_str!("../../../docs/r1/hotkeys.tsv");

impl Bindings {
    /// PotPlayer's defaults: the main-window accelerators that have an
    /// action in RVP, and the owner's mouse page.
    #[must_use]
    pub fn potplayer_defaults() -> Self {
        let mut keys = BTreeMap::new();
        for line in HOTKEYS.lines().skip(1) {
            let mut cells = line.split('\t');
            let (Some("157"), Some(key), Some(id)) = (cells.next(), cells.next(), cells.next())
            else {
                continue;
            };
            let (Some(chord), Some(action)) = (
                KeyChord::parse(key),
                id.parse().ok().and_then(Action::from_potplayer_id),
            ) else {
                continue;
            };
            keys.insert(chord, action);
        }
        let mouse = BTreeMap::from([
            (MouseGesture::LeftClick, Action::PlayPause),
            (MouseGesture::LeftDoubleClick, Action::ToggleFullscreen),
            (MouseGesture::RightClick, Action::ContextMenu),
            (MouseGesture::MiddleClick, Action::ToggleFullscreen),
            (MouseGesture::WheelUp, Action::VolumeUp),
            (MouseGesture::WheelDown, Action::VolumeDown),
            (MouseGesture::X1Click, Action::SeekBackward(SeekStep::Short)),
            (MouseGesture::X2Click, Action::SeekForward(SeekStep::Short)),
        ]);
        Self {
            keys,
            mouse,
            left_click_video_only: true,
        }
    }

    #[must_use]
    pub fn key(&self, chord: &KeyChord) -> Option<Action> {
        self.keys.get(chord).copied()
    }

    #[must_use]
    pub fn mouse(&self, gesture: MouseGesture) -> Option<Action> {
        self.mouse.get(&gesture).copied()
    }

    /// The first key bound to `action`, for menus ("Play/Pause\tSpace").
    #[must_use]
    pub fn key_for(&self, action: Action) -> Option<KeyChord> {
        self.keys
            .iter()
            .find(|&(_, &bound)| bound == action)
            .map(|(&chord, _)| chord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_potplayer_key_names() {
        let chord = KeyChord::parse("Ctrl+Alt+Left").expect("parses");
        assert!(chord.ctrl && chord.alt && !chord.shift);
        assert_eq!(chord.key, Key::Left);
        assert_eq!(KeyChord::parse("Space").map(|c| c.key), Some(Key::Space));
        assert_eq!(KeyChord::parse("m").map(|c| c.key), Some(Key::Char('M')));
        assert_eq!(KeyChord::parse("Num0").map(|c| c.key), Some(Key::Numpad(0)));
        assert_eq!(KeyChord::parse("Num+").map(|c| c.key), Some(Key::NumpadAdd));
        assert_eq!(
            KeyChord::parse("Ctrl+\\").map(|c| c.key),
            Some(Key::Punct('\\'))
        );
        assert_eq!(
            KeyChord::parse("VK_17").map(|c| c.key),
            Some(Key::Other(0x17))
        );
        assert_eq!(KeyChord::parse("F12").map(|c| c.key), Some(Key::F(12)));
        assert_eq!(KeyChord::parse("Hyper+X"), None);
        assert_eq!(
            KeyChord::parse("Ctrl+Alt+Left")
                .map(|c| c.to_string())
                .as_deref(),
            Some("Ctrl+Alt+Left")
        );
    }

    #[test]
    fn every_main_window_key_parses() {
        let mut total = 0;
        for line in HOTKEYS.lines().skip(1) {
            let cells: Vec<&str> = line.split('\t').collect();
            if cells.first() == Some(&"157") {
                total += 1;
                assert!(
                    KeyChord::parse(cells[1]).is_some(),
                    "cannot parse {}",
                    cells[1]
                );
            }
        }
        assert_eq!(total, 259);
    }
}
