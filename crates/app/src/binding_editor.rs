//! The key and mouse editor's model (M3): what each action is bound to,
//! conflicts, reset to PotPlayer's defaults, and the changes over the
//! defaults that `settings.json` stores (`keys`, `playlist-keys`, `mouse`).
//!
//! Window keys and playlist keys are separate scopes: a playlist key only
//! applies while the playlist panel has the keyboard, so the same key in
//! both scopes is not a conflict. Within a scope a key belongs to one
//! action, and a mouse gesture to one action.

use std::collections::BTreeMap;

use crate::action::{Action, ListAction};
use crate::input::{Bindings, KeyChord, MouseGesture};

/// Where a key applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Window,
    Playlist,
}

impl Scope {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Scope::Window => "window",
            Scope::Playlist => "playlist",
        }
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Scope> {
        match name {
            "window" => Some(Scope::Window),
            "playlist" => Some(Scope::Playlist),
            _ => None,
        }
    }
}

/// One line of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub scope: Scope,
    /// The action's settings name.
    pub name: String,
    pub keys: Vec<KeyChord>,
    pub gestures: Vec<MouseGesture>,
}

/// The bindings being edited, and PotPlayer's defaults.
#[derive(Debug, Clone)]
pub struct Editor {
    pub bindings: Bindings,
    defaults: Bindings,
}

impl Editor {
    /// Edits `current`; resets go back to `defaults` (PotPlayer's, with
    /// the Keyboard and Mouse pages' options, `Bindings::with_options`).
    #[must_use]
    pub fn new(current: &Bindings, defaults: &Bindings) -> Self {
        Self {
            bindings: current.clone(),
            defaults: defaults.clone(),
        }
    }

    /// Every action of both scopes with its keys and gestures.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let window = Action::all().map(|action| Row {
            scope: Scope::Window,
            name: action.name().to_owned(),
            keys: self
                .bindings
                .keys
                .iter()
                .filter(|&(_, &bound)| bound == action)
                .map(|(chord, _)| *chord)
                .collect(),
            gestures: self
                .bindings
                .mouse
                .iter()
                .filter(|&(_, &bound)| bound == action)
                .map(|(gesture, _)| *gesture)
                .collect(),
        });
        let playlist = ListAction::all().map(|action| Row {
            scope: Scope::Playlist,
            name: action.name().to_owned(),
            keys: self
                .bindings
                .list_keys
                .iter()
                .filter(|&(_, &bound)| bound == action)
                .map(|(chord, _)| *chord)
                .collect(),
            gestures: Vec::new(),
        });
        window.chain(playlist).collect()
    }

    /// The action a key belongs to in a scope (its settings name).
    #[must_use]
    pub fn key_owner(&self, scope: Scope, chord: &KeyChord) -> Option<&'static str> {
        match scope {
            Scope::Window => self.bindings.keys.get(chord).map(|action| action.name()),
            Scope::Playlist => self
                .bindings
                .list_keys
                .get(chord)
                .map(|action| action.name()),
        }
    }

    /// Binds a key to an action; returns the action that lost it, if any.
    /// Unknown action names change nothing.
    pub fn bind_key(
        &mut self,
        scope: Scope,
        action: &str,
        chord: KeyChord,
    ) -> Option<&'static str> {
        match scope {
            Scope::Window => {
                let action = Action::from_name(action)?;
                self.bindings
                    .keys
                    .insert(chord, action)
                    .filter(|previous| *previous != action)
                    .map(Action::name)
            }
            Scope::Playlist => {
                let action = ListAction::from_name(action)?;
                self.bindings
                    .list_keys
                    .insert(chord, action)
                    .filter(|previous| *previous != action)
                    .map(ListAction::name)
            }
        }
    }

    pub fn unbind_key(&mut self, scope: Scope, chord: &KeyChord) -> bool {
        match scope {
            Scope::Window => self.bindings.keys.remove(chord).is_some(),
            Scope::Playlist => self.bindings.list_keys.remove(chord).is_some(),
        }
    }

    /// The action a mouse gesture is bound to.
    #[must_use]
    pub fn gesture_owner(&self, gesture: MouseGesture) -> Option<&'static str> {
        self.bindings
            .mouse
            .get(&gesture)
            .map(|action| action.name())
    }

    /// Binds a gesture; returns the action that lost it, if any.
    pub fn bind_gesture(&mut self, gesture: MouseGesture, action: &str) -> Option<&'static str> {
        let action = Action::from_name(action)?;
        self.bindings
            .mouse
            .insert(gesture, action)
            .filter(|previous| *previous != action)
            .map(Action::name)
    }

    pub fn unbind_gesture(&mut self, gesture: MouseGesture) -> bool {
        self.bindings.mouse.remove(&gesture).is_some()
    }

    /// Gives an action PotPlayer's keys (and gestures) back: its current
    /// ones go, its default ones return, taken from other actions.
    pub fn reset(&mut self, scope: Scope, action: &str) {
        match scope {
            Scope::Window => {
                let Some(action) = Action::from_name(action) else {
                    return;
                };
                self.bindings.keys.retain(|_, bound| *bound != action);
                self.bindings.mouse.retain(|_, bound| *bound != action);
                for (chord, default) in &self.defaults.keys {
                    if *default == action {
                        self.bindings.keys.insert(*chord, action);
                    }
                }
                for (gesture, default) in &self.defaults.mouse {
                    if *default == action {
                        self.bindings.mouse.insert(*gesture, action);
                    }
                }
            }
            Scope::Playlist => {
                let Some(action) = ListAction::from_name(action) else {
                    return;
                };
                self.bindings.list_keys.retain(|_, bound| *bound != action);
                for (chord, default) in &self.defaults.list_keys {
                    if *default == action {
                        self.bindings.list_keys.insert(*chord, action);
                    }
                }
            }
        }
    }

    /// Everything back to PotPlayer's defaults.
    pub fn reset_all(&mut self) {
        let left_click_video_only = self.bindings.left_click_video_only;
        self.bindings = self.defaults.clone();
        self.bindings.left_click_video_only = left_click_video_only;
    }
}

type Overrides = BTreeMap<String, String>;

/// The changes of `current` over `defaults` as `settings.json` stores them:
/// key or gesture name to action name, `""` for an unbound default.
#[must_use]
pub fn overrides(defaults: &Bindings, current: &Bindings) -> (Overrides, Overrides, Overrides) {
    fn diff<K: Ord + Copy, V: Copy + PartialEq>(
        defaults: &BTreeMap<K, V>,
        current: &BTreeMap<K, V>,
        key_name: impl Fn(K) -> String,
        value_name: impl Fn(V) -> &'static str,
    ) -> Overrides {
        let mut changes = Overrides::new();
        for (key, value) in current {
            if defaults.get(key) != Some(value) {
                changes.insert(key_name(*key), value_name(*value).to_owned());
            }
        }
        for key in defaults.keys() {
            if !current.contains_key(key) {
                changes.insert(key_name(*key), String::new());
            }
        }
        changes
    }
    (
        diff(
            &defaults.keys,
            &current.keys,
            |chord| chord.to_string(),
            Action::name,
        ),
        diff(
            &defaults.list_keys,
            &current.list_keys,
            |chord| chord.to_string(),
            ListAction::name,
        ),
        diff(
            &defaults.mouse,
            &current.mouse,
            |gesture| gesture.name().to_owned(),
            Action::name,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn chord(text: &str) -> KeyChord {
        KeyChord::parse(text).expect("key")
    }

    #[test]
    fn conflicts_are_reported_and_resolved() {
        let mut editor = Editor::new(
            &Bindings::potplayer_defaults(),
            &Bindings::potplayer_defaults(),
        );
        assert_eq!(
            editor.key_owner(Scope::Window, &chord("Space")),
            Some("play-pause")
        );
        // Space for Next takes it from Play/Pause.
        assert_eq!(
            editor.bind_key(Scope::Window, "next", chord("Space")),
            Some("play-pause")
        );
        assert_eq!(
            editor.key_owner(Scope::Window, &chord("Space")),
            Some("next")
        );
        // The same key in the playlist scope is no conflict.
        assert_eq!(
            editor.key_owner(Scope::Playlist, &chord("Ctrl+A")),
            Some("select-all")
        );
        assert_eq!(
            editor.bind_key(Scope::Window, "exit", chord("Ctrl+Shift+Q")),
            None
        );
        assert_eq!(editor.bind_key(Scope::Window, "no-such", chord("Q")), None);
        assert_eq!(
            editor.bind_gesture(MouseGesture::MiddleClick, "play-pause"),
            Some("toggle-fullscreen")
        );
        // Reset gives Play/Pause its Space back (and its other keys).
        editor.reset(Scope::Window, "play-pause");
        assert_eq!(
            editor.key_owner(Scope::Window, &chord("Space")),
            Some("play-pause")
        );
        assert_eq!(
            editor.gesture_owner(MouseGesture::LeftClick),
            Some("play-pause")
        );
        assert_eq!(editor.gesture_owner(MouseGesture::MiddleClick), None);
        editor.reset_all();
        assert_eq!(
            editor.gesture_owner(MouseGesture::MiddleClick),
            Some("toggle-fullscreen")
        );
    }

    #[test]
    fn overrides_round_trip_through_settings() {
        let defaults = Bindings::with_options(&crate::input::InputSettings::default());
        let mut editor = Editor::new(&defaults, &defaults);
        editor.bind_key(Scope::Window, "next", chord("Space"));
        editor.unbind_key(Scope::Window, &chord("M"));
        editor.bind_key(Scope::Playlist, "undo", chord("Ctrl+U"));
        editor.unbind_gesture(MouseGesture::WheelUp);
        let (keys, playlist_keys, mouse) = overrides(&defaults, &editor.bindings);
        assert_eq!(keys.get("Space").map(String::as_str), Some("next"));
        assert_eq!(keys.get("M").map(String::as_str), Some(""));
        assert_eq!(
            playlist_keys.get("Ctrl+U").map(String::as_str),
            Some("undo")
        );
        assert_eq!(mouse.get("wheel-up").map(String::as_str), Some(""));
        let settings = Settings {
            keys,
            mouse,
            playlist_keys,
            ..Settings::default()
        };
        let (restored, warnings) = settings.bindings();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(restored, editor.bindings);
        let unchanged = overrides(&defaults, &defaults);
        assert!(unchanged.0.is_empty() && unchanged.1.is_empty() && unchanged.2.is_empty());
    }
}
