//! User settings (`settings.json` in the configuration directory). Missing
//! fields take their defaults, so older files keep working.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::action::{Action, SeekStep};
use crate::input::{Bindings, KeyChord, MouseGesture};

/// RVP's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Settings {
    /// The `.dsf` skin to load. Unset: PotPlayer's installed BlackSpace on
    /// Windows; on Linux a skin must be configured.
    pub skin: Option<PathBuf>,
    /// mpv `hwdec`.
    pub hwdec: String,
    /// mpv `input-ipc-server` for SVP; empty turns it off.
    pub ipc_server: String,
    /// Seek steps in seconds: Left/Right, Ctrl, Shift, Ctrl+Alt. PotPlayer's
    /// defaults are assumed to be 5 s, 30 s, 1 min and 5 min (to confirm
    /// against the owner's PotPlayer).
    pub seek_seconds: [f64; 4],
    /// Volume step for Up/Down and the wheel, in percent.
    pub volume_step: f64,
    /// Resume every file at its last position (PotPlayer
    /// `RememberPosition=1`).
    pub remember_positions: bool,
    /// Without files on the command line, reopen the last file paused at
    /// its position.
    pub reopen_last_file: bool,
    /// Key binding changes over PotPlayer's defaults: key (`Ctrl+Left`) to
    /// action name (`seek-backward-medium`); an empty name unbinds the key.
    pub keys: BTreeMap<String, String>,
    /// Mouse binding changes: gesture name (`left-click`) to action name.
    pub mouse: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            skin: None,
            hwdec: "auto-copy".to_owned(),
            ipc_server: rvp_engine::default_ipc_server().to_owned(),
            seek_seconds: [5.0, 30.0, 60.0, 300.0],
            volume_step: 5.0,
            remember_positions: true,
            reopen_last_file: true,
            keys: BTreeMap::new(),
            mouse: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Seconds of a seek step.
    #[must_use]
    pub fn seek_seconds(&self, step: SeekStep) -> f64 {
        self.seek_seconds[match step {
            SeekStep::Short => 0,
            SeekStep::Medium => 1,
            SeekStep::Long => 2,
            SeekStep::VeryLong => 3,
        }]
    }

    /// PotPlayer's default bindings with this file's changes applied.
    /// Unknown keys, gestures and action names are returned as warnings.
    #[must_use]
    pub fn bindings(&self) -> (Bindings, Vec<String>) {
        let mut bindings = Bindings::potplayer_defaults();
        let mut warnings = Vec::new();
        for (key, name) in &self.keys {
            let Some(chord) = KeyChord::parse(key) else {
                warnings.push(format!("unknown key {key:?} in settings"));
                continue;
            };
            if name.is_empty() {
                bindings.keys.remove(&chord);
            } else if let Some(action) = Action::from_name(name) {
                bindings.keys.insert(chord, action);
            } else {
                warnings.push(format!("unknown action {name:?} for key {key}"));
            }
        }
        for (gesture, name) in &self.mouse {
            let Some(gesture_value) = MouseGesture::from_name(gesture) else {
                warnings.push(format!("unknown mouse gesture {gesture:?} in settings"));
                continue;
            };
            if name.is_empty() {
                bindings.mouse.remove(&gesture_value);
            } else if let Some(action) = Action::from_name(name) {
                bindings.mouse.insert(gesture_value, action);
            } else {
                warnings.push(format!("unknown action {name:?} for {gesture}"));
            }
        }
        (bindings, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;

    #[test]
    fn missing_fields_take_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "volume-step": 2 }"#).expect("parses");
        assert!((settings.volume_step - 2.0).abs() < f64::EPSILON);
        assert_eq!(settings.hwdec, "auto-copy");
        assert!(settings.remember_positions);
    }

    #[test]
    fn binding_changes_apply_over_defaults() {
        let mut settings = Settings::default();
        settings.keys.insert("Ctrl+Left".into(), "next".into());
        settings.keys.insert("Space".into(), String::new());
        settings.keys.insert("Hyper+Q".into(), "exit".into());
        settings
            .mouse
            .insert("middle-click".into(), "play-pause".into());
        let (bindings, warnings) = settings.bindings();
        let ctrl_left = KeyChord {
            ctrl: true,
            ..KeyChord::plain(Key::Left)
        };
        assert_eq!(bindings.key(&ctrl_left), Some(Action::Next));
        assert_eq!(bindings.key(&KeyChord::plain(Key::Space)), None);
        assert_eq!(
            bindings.mouse(MouseGesture::MiddleClick),
            Some(Action::PlayPause)
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }
}
