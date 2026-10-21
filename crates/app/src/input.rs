//! Keyboard and mouse bindings with PotPlayer's defaults.
//!
//! Keys are platform-neutral; the shell translates Qt key events. The
//! default keymap is PotPlayer's main-window accelerator table (table 157 in
//! `docs/r1/hotkeys.tsv`, read from PotPlayer's resources in R1): every key
//! whose command has an [`Action`] is bound. The default mouse map is the
//! owner's PotPlayer mouse page (`docs/r1/mouse-actions.tsv`).

use std::collections::BTreeMap;
use std::fmt;

use crate::action::{Action, ListAction, SeekStep};

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
    /// The keyboard's browser back and forward keys.
    BrowserBack,
    BrowserForward,
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

/// Qt keyboard modifier bits (`Qt::KeyboardModifier`).
const QT_SHIFT: i32 = 0x0200_0000;
const QT_CONTROL: i32 = 0x0400_0000;
const QT_ALT: i32 = 0x0800_0000;
const QT_KEYPAD: i32 = 0x2000_0000;

/// The unshifted key of a shifted character on a US keyboard, which is how
/// PotPlayer names keys (`Shift+1`, not `!`).
fn unshifted(c: char) -> Option<char> {
    const PAIRS: [(char, char); 21] = [
        ('!', '1'),
        ('@', '2'),
        ('#', '3'),
        ('$', '4'),
        ('%', '5'),
        ('^', '6'),
        ('&', '7'),
        ('*', '8'),
        ('(', '9'),
        (')', '0'),
        ('_', '-'),
        ('+', '='),
        ('{', '['),
        ('}', ']'),
        ('|', '\\'),
        (':', ';'),
        ('"', '\''),
        ('<', ','),
        ('>', '.'),
        ('?', '/'),
        ('~', '`'),
    ];
    PAIRS
        .iter()
        .find(|&&(shifted, _)| shifted == c)
        .map(|&(_, base)| base)
}

fn qt_special_key(key: i32) -> Option<Key> {
    Some(match key {
        0x0100_0000 => Key::Escape,
        0x0100_0001 | 0x0100_0002 => Key::Tab,
        0x0100_0003 => Key::Backspace,
        0x0100_0004 | 0x0100_0005 => Key::Enter,
        0x0100_0006 => Key::Insert,
        0x0100_0007 => Key::Delete,
        0x0100_0008 => Key::Pause,
        0x0100_0009 => Key::PrintScreen,
        0x0100_0010 => Key::Home,
        0x0100_0011 => Key::End,
        0x0100_0012 => Key::Left,
        0x0100_0013 => Key::Up,
        0x0100_0014 => Key::Right,
        0x0100_0015 => Key::Down,
        0x0100_0016 => Key::PageUp,
        0x0100_0017 => Key::PageDown,
        0x0100_0026 => Key::ScrollLock,
        0x0100_0070 => Key::VolumeDown,
        0x0100_0071 => Key::VolumeMute,
        0x0100_0072 => Key::VolumeUp,
        0x0100_0080 | 0x0100_0085 | 0x0100_0086 => Key::MediaPlayPause,
        0x0100_0081 => Key::MediaStop,
        0x0100_0082 => Key::MediaPrevious,
        0x0100_0083 => Key::MediaNext,
        0x0100_0061 => Key::BrowserBack,
        0x0100_0062 => Key::BrowserForward,
        // F1..F24 (Qt numbers F1 as 0x01000030).
        f if (0x0100_0030..0x0100_0030 + 24).contains(&f) => {
            Key::F(u8::try_from(f - 0x0100_0030 + 1).ok()?)
        }
        _ => return None,
    })
}

impl KeyChord {
    /// Translates a Qt key event (`QKeyEvent::key()` and `modifiers()`) to
    /// the chords to look up, best match first. A shifted character also
    /// tries the character itself, because PotPlayer's table names some
    /// keys by their shifted character (`<`). Modifier keys alone give none.
    #[must_use]
    pub fn from_qt(key: i32, modifiers: i32) -> Vec<KeyChord> {
        let ctrl = modifiers & QT_CONTROL != 0;
        let alt = modifiers & QT_ALT != 0;
        let shift = modifiers & QT_SHIFT != 0;
        let keypad = modifiers & QT_KEYPAD != 0;
        let chord = |key, shift| KeyChord {
            ctrl,
            alt,
            shift,
            key,
        };
        if let Some(special) = qt_special_key(key) {
            return vec![chord(special, shift)];
        }
        let Some(c) = u32::try_from(key).ok().and_then(char::from_u32) else {
            return Vec::new();
        };
        if keypad {
            let numpad = match c {
                '0'..='9' => c
                    .to_digit(10)
                    .and_then(|d| u8::try_from(d).ok())
                    .map(Key::Numpad),
                '+' => Some(Key::NumpadAdd),
                '-' => Some(Key::NumpadSubtract),
                '*' => Some(Key::NumpadMultiply),
                '/' => Some(Key::NumpadDivide),
                '.' | ',' => Some(Key::NumpadDecimal),
                _ => None,
            };
            if let Some(numpad) = numpad {
                return vec![chord(numpad, shift)];
            }
        }
        match c {
            ' ' => vec![chord(Key::Space, shift)],
            c if c.is_ascii_alphanumeric() => vec![chord(Key::Char(c.to_ascii_uppercase()), shift)],
            c if c.is_ascii_punctuation() => match unshifted(c) {
                Some(base) if shift => {
                    let base_key = if base.is_ascii_digit() {
                        Key::Char(base)
                    } else {
                        Key::Punct(base)
                    };
                    vec![chord(base_key, true), chord(Key::Punct(c), false)]
                }
                _ => vec![chord(Key::Punct(c), shift)],
            },
            _ => Vec::new(),
        }
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
        "BrowserBack" => Key::BrowserBack,
        "BrowserForward" => Key::BrowserForward,
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
    /// The wheel while X1 or X2 is held ("Operate mouse wheel while X
    /// button is pressed down", M14).
    X1WheelUp,
    X1WheelDown,
    X2WheelUp,
    X2WheelDown,
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
    (MouseGesture::X1WheelUp, "x1-wheel-up"),
    (MouseGesture::X1WheelDown, "x1-wheel-down"),
    (MouseGesture::X2WheelUp, "x2-wheel-up"),
    (MouseGesture::X2WheelDown, "x2-wheel-down"),
];

impl MouseGesture {
    /// Every gesture, in the editor's order.
    pub fn all() -> impl Iterator<Item = MouseGesture> {
        GESTURE_NAMES.iter().map(|&(gesture, _)| gesture)
    }

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
    /// Keys while the playlist panel has the keyboard.
    pub list_keys: BTreeMap<KeyChord, ListAction>,
    pub mouse: BTreeMap<MouseGesture, Action>,
    /// PotPlayer: "Operate the action 'Left Button - Single Click' in the
    /// video area only" (on for the owner).
    pub left_click_video_only: bool,
}

/// PotPlayer's Keyboard and Mouse page options (`input` in settings.json,
/// M14).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of two settings pages.
#[allow(clippy::struct_excessive_bools)]
pub struct InputSettings {
    /// Only the user's own keys: PotPlayer's default hotkeys are off.
    pub disable_default_keys: bool,
    /// The keyboard's media keys act in RVP's window (Windows' media
    /// controls take them while RVP's window has no keyboard).
    pub media_keys: bool,
    /// The media keys reach RVP while another window has the keyboard
    /// (Windows' hotkeys; they win over the media controls).
    pub global_media_keys: bool,
    /// The keyboard's volume keys change RVP's volume while its window has
    /// the keyboard.
    pub volume_keys: bool,
    /// The keyboard's back and forward keys go to the previous and next
    /// entry.
    pub navigation_keys: bool,
    /// The left click acts in the video area only.
    pub left_click_video_only: bool,
    /// See [`crate::pointer::PointerOptions`].
    pub drag_title_only: bool,
    pub drag_picture: bool,
    pub drag_subtitles: bool,
    pub x_wheel: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            disable_default_keys: false,
            media_keys: true,
            global_media_keys: false,
            volume_keys: false,
            navigation_keys: false,
            left_click_video_only: true,
            drag_title_only: false,
            drag_picture: false,
            drag_subtitles: false,
            x_wheel: false,
        }
    }
}

impl InputSettings {
    /// The pointer's options.
    #[must_use]
    pub fn pointer(&self) -> crate::pointer::PointerOptions {
        crate::pointer::PointerOptions {
            drag_title_only: self.drag_title_only,
            drag_picture: self.drag_picture,
            drag_subtitles: self.drag_subtitles,
            x_wheel: self.x_wheel,
        }
    }

    /// The keys these options add over PotPlayer's table (the user's own
    /// keys still win).
    fn extra_keys(&self) -> Vec<(KeyChord, Action)> {
        let mut keys = Vec::new();
        let plain = |key| KeyChord {
            key,
            ctrl: false,
            alt: false,
            shift: false,
        };
        if self.media_keys {
            keys.extend([
                (plain(Key::MediaPlayPause), Action::PlayPause),
                (plain(Key::MediaStop), Action::CloseMedia),
                (plain(Key::MediaNext), Action::Next),
                (plain(Key::MediaPrevious), Action::Previous),
            ]);
        }
        if self.volume_keys {
            keys.extend([
                (plain(Key::VolumeUp), Action::VolumeUp),
                (plain(Key::VolumeDown), Action::VolumeDown),
                (plain(Key::VolumeMute), Action::ToggleMute),
            ]);
        }
        if self.navigation_keys {
            keys.extend([
                (plain(Key::BrowserBack), Action::Previous),
                (plain(Key::BrowserForward), Action::Next),
            ]);
        }
        keys
    }
}

/// PotPlayer's default hotkey table (R1).
const HOTKEYS: &str = include_str!("../../../docs/r1/hotkeys.tsv");

impl Bindings {
    /// PotPlayer's defaults with the Keyboard and Mouse pages' options
    /// applied (see [`InputSettings`]).
    #[must_use]
    pub fn with_options(input: &InputSettings) -> Self {
        let mut bindings = Self::potplayer_defaults();
        if input.disable_default_keys {
            bindings.keys.clear();
        }
        for (chord, action) in input.extra_keys() {
            bindings.keys.entry(chord).or_insert(action);
        }
        bindings.left_click_video_only = input.left_click_video_only;
        bindings
    }

    /// PotPlayer's defaults: the main-window accelerators that have an
    /// action in RVP, and the owner's mouse page.
    #[must_use]
    pub fn potplayer_defaults() -> Self {
        let mut keys = BTreeMap::new();
        let mut list_keys = BTreeMap::new();
        for line in HOTKEYS.lines().skip(1) {
            let mut cells = line.split('\t');
            let (Some(table), Some(key), Some(id)) = (cells.next(), cells.next(), cells.next())
            else {
                continue;
            };
            let (Some(chord), Ok(id)) = (KeyChord::parse(key), id.parse::<u32>()) else {
                continue;
            };
            match table {
                // A key listed twice (Ctrl+\: 10592 and 10596) keeps the
                // first, the one PotPlayer's menu shows it with.
                "157" => {
                    if let Some(action) = Action::from_potplayer_id(id) {
                        keys.entry(chord).or_insert(action);
                    }
                }
                "118" => {
                    if let Some(action) = ListAction::from_potplayer_id(id) {
                        list_keys.insert(chord, action);
                    }
                }
                _ => {}
            }
        }
        // RVP's own: M18's motion smoothing on or off (free in PotPlayer).
        if let Some(chord) = KeyChord::parse("Ctrl+Shift+I") {
            keys.entry(chord).or_insert(Action::Smoothing(
                crate::smoothing::SmoothingCommand::Toggle,
            ));
        }
        // Not in PotPlayer's table: undo, redo and moving the selection.
        for (key, action) in [
            ("Ctrl+Z", ListAction::Undo),
            ("Ctrl+Y", ListAction::Redo),
            ("Up", ListAction::CursorUp),
            ("Down", ListAction::CursorDown),
            ("Home", ListAction::CursorHome),
            ("End", ListAction::CursorEnd),
            ("PgUp", ListAction::PageUp),
            ("PgDn", ListAction::PageDown),
        ] {
            if let Some(chord) = KeyChord::parse(key) {
                list_keys.entry(chord).or_insert(action);
            }
        }
        // PotPlayer's menu names Insert beside Ctrl+\ for "Repeat current
        // subtitle section"; its table lacks it.
        if let Some(chord) = KeyChord::parse("Ins") {
            keys.entry(chord)
                .or_insert(Action::AbRepeat(crate::action::AbRepeat::CurrentSubtitle));
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
            list_keys,
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

    /// The key a menu shows for `action` ("Play/Pause\tSpace"): the
    /// simplest bound key, preferring keys without modifiers and keys on
    /// the main keyboard over media and keypad keys.
    #[must_use]
    pub fn key_for(&self, action: Action) -> Option<KeyChord> {
        self.keys
            .iter()
            .filter(|&(_, &bound)| bound == action)
            .map(|(&chord, _)| chord)
            .min_by_key(|chord| {
                let unusual = matches!(
                    chord.key,
                    Key::Numpad(_)
                        | Key::NumpadAdd
                        | Key::NumpadSubtract
                        | Key::NumpadMultiply
                        | Key::NumpadDivide
                        | Key::NumpadDecimal
                        | Key::MediaPlayPause
                        | Key::MediaStop
                        | Key::MediaNext
                        | Key::MediaPrevious
                        | Key::VolumeUp
                        | Key::VolumeDown
                        | Key::VolumeMute
                        | Key::BrowserBack
                        | Key::BrowserForward
                        | Key::Other(_)
                );
                let modifiers = u8::from(chord.ctrl) + u8::from(chord.alt) + u8::from(chord.shift);
                (unusual, modifiers, *chord)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qt_keys_translate_to_potplayer_chords() {
        const CTRL: i32 = 0x0400_0000;
        const SHIFT: i32 = 0x0200_0000;
        const KEYPAD: i32 = 0x2000_0000;
        let first = |key, modifiers| KeyChord::from_qt(key, modifiers).first().copied();
        assert_eq!(first(0x20, 0), Some(KeyChord::plain(Key::Space)));
        assert_eq!(first(0x41, 0), Some(KeyChord::plain(Key::Char('A'))));
        assert_eq!(first(0x0100_0012, CTRL), KeyChord::parse("Ctrl+Left"));
        assert_eq!(first(0x0100_0004, 0), KeyChord::parse("Enter"));
        assert_eq!(first(0x0100_0005, KEYPAD), KeyChord::parse("Enter"));
        assert_eq!(first(0x0100_0032, 0), KeyChord::parse("F3"));
        assert_eq!(first(0x35, KEYPAD), KeyChord::parse("Num5"));
        assert_eq!(first(0x2b, KEYPAD), KeyChord::parse("Num+"));
        assert_eq!(first(0x0100_0086, 0), KeyChord::parse("MediaPlayPause"));
        // Shift+1 arrives as '!'.
        assert_eq!(first(0x21, SHIFT), KeyChord::parse("Shift+1"));
        // Shift+, arrives as '<', which PotPlayer's table also names.
        assert_eq!(
            KeyChord::from_qt(0x3c, SHIFT),
            vec![
                KeyChord::parse("Shift+,").expect("chord"),
                KeyChord::parse("<").expect("chord")
            ]
        );
        // A modifier key alone.
        assert!(KeyChord::from_qt(0x0100_0021, CTRL).is_empty());
    }

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
