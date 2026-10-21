//! M14: PotPlayer's settings presets (Preferences > Configuration and
//! Misc > the presets and Manage Presets), goal P.
//!
//! * The default preset is settings.json's own values; each other preset
//!   holds a whole copy of the settings (`values`), kept in settings.json's
//!   `presets` section. While a preset is active RVP runs with its values,
//!   and every change goes into it; the default's values wait meanwhile.
//! * A preset is activated by hand (the Misc menu, the page) or, when it
//!   has conditions, as a file loads whose facts meet all of them (the
//!   first such preset in the list). "Return to previous preset at playback
//!   end" goes back once that file ends.
//! * Activating by hand can reopen the playing file and copy the preset's
//!   values into the default; "Activate default preset at program start"
//!   starts with the default whatever was active.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app::{App, UiRequest};
use crate::settings::Settings;

/// Comparisons of the conditions: =, ≠, <, ≤, >, ≥.
pub const COMPARISONS: &[&str] = &["eq", "ne", "lt", "le", "gt", "ge"];

/// The presets (`presets` in settings.json).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's page.
#[allow(clippy::struct_excessive_bools)]
pub struct PresetsSettings {
    /// The active preset's name; empty: the default.
    pub active: String,
    /// "Reopen current item when preset is manually activated".
    pub reopen: bool,
    /// "Copy preset values into default preset when preset is manually
    /// activated".
    pub copy_to_default: bool,
    /// "Return to previous preset at playback end when preset is auto
    /// activated".
    pub return_at_end: bool,
    /// "Activate default preset at program start".
    pub default_at_start: bool,
    pub list: Vec<Preset>,
}

/// One preset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Preset {
    pub name: String,
    pub conditions: Conditions,
    /// The settings it holds (all but the presets); `null` until it was
    /// first active, when it starts as a copy of the settings then.
    pub values: Value,
}

/// A number and how a file's must compare with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Compare {
    /// One of [`COMPARISONS`].
    pub op: String,
    pub value: f64,
}

impl Default for Compare {
    fn default() -> Self {
        Self {
            op: "eq".to_owned(),
            value: 0.0,
        }
    }
}

// The values are the user's numbers, never NaN (validate drops those).
impl Eq for Compare {}

impl Compare {
    fn holds(&self, actual: f64) -> bool {
        // Equality within a thousandth: rates and aspects are not exact.
        let close = (actual - self.value).abs() < 1e-3;
        match self.op.as_str() {
            "ne" => !close,
            "lt" => actual < self.value && !close,
            "le" => actual < self.value || close,
            "gt" => actual > self.value && !close,
            "ge" => actual > self.value || close,
            _ => close,
        }
    }
}

/// Auto-Preset: what a file must be for the preset to activate (`None`:
/// any).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Conditions {
    /// The file's name, with `*` and `?` (case does not matter).
    pub file_name: Option<String>,
    /// The playing album's name.
    pub album: Option<String>,
    /// "ac" or "battery".
    pub power: Option<String>,
    /// Seconds.
    pub duration: Option<Compare>,
    pub width: Option<Compare>,
    pub height: Option<Compare>,
    /// The video codec (FFmpeg's name or tag, such as h264 or hevc).
    pub fourcc: Option<String>,
    pub fps: Option<Compare>,
    pub aspect: Option<Compare>,
    /// The audio codec.
    pub audio_format: Option<String>,
    pub sample_rate: Option<Compare>,
    pub channels: Option<Compare>,
    /// Only when the file has no video.
    pub audio_only: bool,
    /// Only when the file has subtitles.
    pub subtitles: bool,
}

/// What a loaded file is, for the conditions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    pub file_name: String,
    pub album: String,
    pub on_battery: Option<bool>,
    pub duration: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub codec: Option<String>,
    pub fps: Option<f64>,
    pub aspect: Option<f64>,
    pub audio_codec: Option<String>,
    pub sample_rate: Option<f64>,
    pub channels: Option<f64>,
    pub audio_only: bool,
    pub subtitles: bool,
}

/// `name` against a pattern with `*` and `?`, ignoring case.
fn wildcard(pattern: &str, name: &str) -> bool {
    fn matches(pattern: &[char], name: &[char]) -> bool {
        match (pattern.first(), name.first()) {
            (None, None) => true,
            (Some('*'), _) => {
                matches(&pattern[1..], name) || (!name.is_empty() && matches(pattern, &name[1..]))
            }
            (Some('?'), Some(_)) => matches(&pattern[1..], &name[1..]),
            (Some(p), Some(n)) if p == n => matches(&pattern[1..], &name[1..]),
            _ => false,
        }
    }
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let name: Vec<char> = name.to_lowercase().chars().collect();
    matches(&pattern, &name)
}

impl Conditions {
    /// Whether any condition is set (a preset without one is activated by
    /// hand only).
    #[must_use]
    pub fn any(&self) -> bool {
        *self != Self::default()
    }

    /// Whether `facts` meet every condition set.
    #[must_use]
    pub fn hold(&self, facts: &Facts) -> bool {
        let text = |wanted: &Option<String>, actual: &Option<String>| {
            wanted.as_ref().is_none_or(|wanted| {
                actual
                    .as_ref()
                    .is_some_and(|actual| actual.to_lowercase().contains(&wanted.to_lowercase()))
            })
        };
        let number = |wanted: &Option<Compare>, actual: Option<f64>| {
            wanted
                .as_ref()
                .is_none_or(|wanted| actual.is_some_and(|actual| wanted.holds(actual)))
        };
        self.file_name
            .as_ref()
            .is_none_or(|pattern| wildcard(pattern, &facts.file_name))
            && self
                .album
                .as_ref()
                .is_none_or(|album| album.eq_ignore_ascii_case(&facts.album))
            && self.power.as_ref().is_none_or(|power| {
                facts
                    .on_battery
                    .is_some_and(|battery| battery == (power == "battery"))
            })
            && number(&self.duration, facts.duration)
            && number(&self.width, facts.width)
            && number(&self.height, facts.height)
            && text(&self.fourcc, &facts.codec)
            && number(&self.fps, facts.fps)
            && number(&self.aspect, facts.aspect)
            && text(&self.audio_format, &facts.audio_codec)
            && number(&self.sample_rate, facts.sample_rate)
            && number(&self.channels, facts.channels)
            && (!self.audio_only || facts.audio_only)
            && (!self.subtitles || facts.subtitles)
    }
}

impl PresetsSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let mut names = std::collections::BTreeSet::new();
        let before = self.list.len();
        self.list
            .retain(|preset| !preset.name.trim().is_empty() && names.insert(preset.name.clone()));
        if self.list.len() != before {
            notes.push(
                "presets.list: presets without a name or with a name twice were left out"
                    .to_owned(),
            );
        }
        for preset in &mut self.list {
            let conditions = &mut preset.conditions;
            for compare in [
                &mut conditions.duration,
                &mut conditions.width,
                &mut conditions.height,
                &mut conditions.fps,
                &mut conditions.aspect,
                &mut conditions.sample_rate,
                &mut conditions.channels,
            ] {
                if compare.as_ref().is_some_and(|compare| {
                    !COMPARISONS.contains(&compare.op.as_str()) || !compare.value.is_finite()
                }) {
                    notes.push(format!(
                        "presets: {}'s comparison is not one RVP knows: dropped",
                        preset.name
                    ));
                    *compare = None;
                }
            }
            if conditions
                .power
                .as_ref()
                .is_some_and(|power| power != "ac" && power != "battery")
            {
                conditions.power = None;
            }
            if !preset.values.is_null() && !preset.values.is_object() {
                preset.values = Value::Null;
            }
        }
        if !self.active.is_empty() && !self.list.iter().any(|preset| preset.name == self.active) {
            notes.push(format!(
                "presets.active {:?} is no preset: the default",
                self.active
            ));
            self.active.clear();
        }
    }
}

/// The settings' values without the presets.
fn values(settings: &Settings) -> Value {
    let mut value = serde_json::to_value(settings).unwrap_or(Value::Null);
    if let Some(map) = value.as_object_mut() {
        map.remove("presets");
    }
    value
}

/// `values` with `presets` as the settings to run with.
fn settings_from(values: &Value, presets: &PresetsSettings) -> Settings {
    let mut value = values.clone();
    if let Some(map) = value.as_object_mut() {
        map.insert(
            "presets".to_owned(),
            serde_json::to_value(presets).unwrap_or(Value::Null),
        );
    }
    serde_json::from_value(value).unwrap_or_default()
}

impl App {
    /// At start, after the settings were read: the active preset's values,
    /// or the default's when the page says so.
    pub(crate) fn start_presets(&mut self) {
        let presets = self.settings.presets.clone();
        if presets.active.is_empty() {
            return;
        }
        if presets.default_at_start {
            self.settings.presets.active.clear();
            return;
        }
        let Some(preset) = presets
            .list
            .iter()
            .find(|preset| preset.name == presets.active)
        else {
            return;
        };
        self.default_values = Some(values(&self.settings));
        if !preset.values.is_null() {
            let mut settings = settings_from(&preset.values, &presets);
            settings.validate();
            self.settings = settings;
        }
    }

    /// What settings.json holds now: the default's values with the
    /// presets, the active one's values taken from the settings running.
    #[must_use]
    pub(crate) fn stored_settings(&self) -> Value {
        let active = &self.settings.presets.active;
        let Some(default_values) = self.default_values.as_ref().filter(|_| !active.is_empty())
        else {
            return serde_json::to_value(&self.settings).unwrap_or(Value::Null);
        };
        let mut presets = self.settings.presets.clone();
        if let Some(preset) = presets
            .list
            .iter_mut()
            .find(|preset| preset.name == *active)
        {
            preset.values = values(&self.settings);
        }
        let mut stored = default_values.clone();
        if let Some(map) = stored.as_object_mut() {
            map.insert(
                "presets".to_owned(),
                serde_json::to_value(&presets).unwrap_or(Value::Null),
            );
        }
        stored
    }

    /// Activates the preset `name` (empty: the default), by hand or for a
    /// file's facts.
    pub fn activate_preset(
        &mut self,
        engine: &rvp_engine::Engine,
        name: &str,
        by_hand: bool,
    ) -> Option<UiRequest> {
        if name == self.settings.presets.active {
            return Some(UiRequest::Osd(format!("Preset: {}", shown(name))));
        }
        if !name.is_empty()
            && !self
                .settings
                .presets
                .list
                .iter()
                .any(|preset| preset.name == name)
        {
            return None;
        }
        // The running values go back where they belong.
        let running = values(&self.settings);
        let mut presets = self.settings.presets.clone();
        if presets.active.is_empty() {
            self.default_values = Some(running.clone());
        } else if let Some(preset) = presets
            .list
            .iter_mut()
            .find(|preset| preset.name == presets.active)
        {
            preset.values = running.clone();
        }
        let target = if name.is_empty() {
            self.default_values.take().unwrap_or(running)
        } else {
            let preset = presets.list.iter_mut().find(|preset| preset.name == name)?;
            if preset.values.is_null() {
                preset.values = running;
            }
            preset.values.clone()
        };
        if by_hand && presets.copy_to_default && !name.is_empty() {
            self.default_values = Some(target.clone());
        }
        name.clone_into(&mut presets.active);
        let reopen = by_hand && presets.reopen && self.model.media_loaded();
        let settings = settings_from(&target, &presets);
        let applied = self.apply_settings(engine, settings);
        for note in applied.notes {
            eprintln!("rvp: preset {name}: {note}");
        }
        if reopen {
            return self.dispatch(engine, crate::Action::Reopen);
        }
        Some(UiRequest::SettingsChanged(format!(
            "Preset: {}",
            shown(name)
        )))
    }

    /// A file loaded: the first preset whose conditions it meets.
    pub(crate) fn auto_preset(&mut self, engine: &rvp_engine::Engine) -> Option<UiRequest> {
        let facts = self.file_facts(engine);
        let found = self
            .settings
            .presets
            .list
            .iter()
            .find(|preset| preset.conditions.any() && preset.conditions.hold(&facts))
            .map(|preset| preset.name.clone())?;
        if found == self.settings.presets.active {
            return None;
        }
        if self.preset_before_auto.is_none() {
            self.preset_before_auto = Some(self.settings.presets.active.clone());
        }
        self.activate_preset(engine, &found, false)
    }

    /// A file ended: back to the preset before the auto-activated one, when
    /// the page says so.
    pub(crate) fn preset_playback_end(&mut self, engine: &rvp_engine::Engine) -> Option<UiRequest> {
        if !self.settings.presets.return_at_end {
            return None;
        }
        let previous = self.preset_before_auto.take()?;
        self.activate_preset(engine, &previous, false)
    }

    /// The loaded file's facts.
    fn file_facts(&self, engine: &rvp_engine::Engine) -> Facts {
        use rvp_engine::PropertyId;
        let model = &self.model;
        let lists = self.library().lists();
        let int = |id| {
            model.int(id).map(|value| {
                // Sizes, rates and counts are far inside f64's integers.
                #[allow(clippy::cast_precision_loss)]
                let value = value as f64;
                value
            })
        };
        Facts {
            file_name: model
                .loaded_path()
                .map(|path| crate::view::file_name(path).to_owned())
                .unwrap_or_default(),
            album: lists
                .playing
                .and_then(|album| lists.albums.get(album))
                .map(|album| album.name.clone())
                .unwrap_or_default(),
            on_battery: self.on_battery,
            duration: model.double(PropertyId::Duration),
            width: int(PropertyId::VideoWidth),
            height: int(PropertyId::VideoHeight),
            codec: model.text(PropertyId::VideoFormat).map(str::to_owned),
            fps: model.double(PropertyId::ContainerFps),
            aspect: model.double(PropertyId::VideoAspect),
            audio_codec: model.text(PropertyId::AudioCodecName).map(str::to_owned),
            sample_rate: int(PropertyId::AudioSampleRate),
            channels: int(PropertyId::AudioChannels),
            audio_only: self.audio_only,
            subtitles: engine
                .tracks()
                .iter()
                .any(|track| track.kind == rvp_engine::TrackKind::Subtitle),
        }
    }

    /// The system runs on its battery (`Some(true)`), on AC power, or
    /// cannot tell (the window reads it).
    pub fn set_on_battery(&mut self, on_battery: Option<bool>) {
        self.on_battery = on_battery;
    }
}

/// A preset's name as RVP shows it.
fn shown(name: &str) -> &str {
    if name.is_empty() { "Default" } else { name }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_names() {
        assert!(wildcard("*.MKV", "film.mkv"));
        assert!(wildcard("ep??.mp4", "ep12.mp4"));
        assert!(!wildcard("ep??.mp4", "ep1.mp4"));
        assert!(wildcard("*", ""));
    }

    #[test]
    fn conditions_hold_for_matching_files() {
        let facts = Facts {
            file_name: "Show S01E02.mkv".to_owned(),
            album: "Series".to_owned(),
            on_battery: Some(true),
            duration: Some(1500.0),
            width: Some(1920.0),
            height: Some(1080.0),
            codec: Some("hevc".to_owned()),
            fps: Some(23.976),
            aspect: Some(1.778),
            audio_codec: Some("aac".to_owned()),
            sample_rate: Some(48_000.0),
            channels: Some(2.0),
            audio_only: false,
            subtitles: true,
        };
        let mut conditions = Conditions::default();
        assert!(!conditions.any());
        conditions.file_name = Some("*s01*".to_owned());
        conditions.power = Some("battery".to_owned());
        conditions.height = Some(Compare {
            op: "ge".to_owned(),
            value: 1080.0,
        });
        conditions.fps = Some(Compare {
            op: "eq".to_owned(),
            value: 23.976,
        });
        conditions.fourcc = Some("HEVC".to_owned());
        conditions.subtitles = true;
        assert!(conditions.any());
        assert!(conditions.hold(&facts));
        conditions.width = Some(Compare {
            op: "lt".to_owned(),
            value: 1920.0,
        });
        assert!(!conditions.hold(&facts));
        conditions.width = None;
        conditions.audio_only = true;
        assert!(!conditions.hold(&facts));
    }

    #[test]
    fn settings_are_checked() {
        let mut presets = PresetsSettings {
            active: "gone".to_owned(),
            list: vec![
                Preset {
                    name: "a".to_owned(),
                    ..Preset::default()
                },
                Preset {
                    name: "a".to_owned(),
                    ..Preset::default()
                },
                Preset::default(),
            ],
            ..PresetsSettings::default()
        };
        presets.list[0].conditions.fps = Some(Compare {
            op: "about".to_owned(),
            value: 1.0,
        });
        let mut notes = Vec::new();
        presets.validate(&mut notes);
        assert_eq!(presets.list.len(), 1);
        assert_eq!(presets.list[0].conditions.fps, None);
        assert!(presets.active.is_empty());
        assert_eq!(notes.len(), 3, "{notes:?}");
    }
}
