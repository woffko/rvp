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
//! * The active preset's name changes only with its values: Rename
//!   ([`App::rename_preset`]) changes the list, the active name and the
//!   preset auto-activation returns to in one change, and a settings change
//!   that would leave the active preset out of the list, or name another,
//!   is refused (`preferences::change`): the values running would go to the
//!   wrong preset. Duplicate copies the values in force
//!   ([`App::duplicate_preset`]).
//! * Export writes what settings.json holds (`App::stored_settings`),
//!   and Import takes a file as a start takes settings.json
//!   ([`App::import_settings`]).
//! * Several RVPs on one settings folder ([`crate::storage`]'s rules,
//!   `merge_settings`): the default preset's values (the file's top
//!   level) and each preset's merge by setting; the list by preset name
//!   (a rename is the old name removed and the new one added), so presets
//!   added or removed in one RVP and values changed in another all stay.
//!   The active preset's name is one setting: the RVP that changed it last
//!   wins, and one that names a preset another RVP deleted makes the next
//!   start run the default (`PresetsSettings::validate`).

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
///
/// One pass with the last star remembered: when the text after it stops
/// matching, that star takes one more character and the match goes on from
/// there. Earlier stars never need another try, so the time is at most the
/// pattern's length times the name's, however many stars the rule has.
fn wildcard(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let name: Vec<char> = name.to_lowercase().chars().collect();
    let (mut p, mut n) = (0, 0);
    // The last star's place in the pattern and the name's place after the
    // characters it took so far.
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        match pattern.get(p) {
            Some('*') => {
                star = Some((p, n));
                p += 1;
            }
            Some(&c) if c == '?' || c == name[n] => {
                p += 1;
                n += 1;
            }
            _ => match star {
                Some((at, taken)) => {
                    star = Some((at, taken + 1));
                    p = at + 1;
                    n = taken + 1;
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
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

/// The settings to run with for `stored` as settings.json holds them (the
/// default's values with the presets), and the default's values while
/// another preset is active.
fn running(stored: Settings) -> (Settings, Option<Value>) {
    let presets = &stored.presets;
    let Some(preset) = presets
        .list
        .iter()
        .find(|preset| !presets.active.is_empty() && preset.name == presets.active)
    else {
        return (stored, None);
    };
    let default_values = Some(values(&stored));
    if preset.values.is_null() {
        return (stored, default_values);
    }
    let mut settings = settings_from(&preset.values, presets);
    settings.validate();
    (settings, default_values)
}

impl App {
    /// At start, after the settings were read: the active preset's values,
    /// or the default's when the page says so.
    pub(crate) fn start_presets(&mut self) {
        if self.settings.presets.default_at_start {
            self.settings.presets.active.clear();
            return;
        }
        let (settings, default_values) = running(std::mem::take(&mut self.settings));
        self.settings = settings;
        self.default_values = default_values;
    }

    /// Makes an imported settings file the settings, as a start takes
    /// settings.json: its own values are the default preset's, and the
    /// preset it names runs with its values.
    pub fn import_settings(
        &mut self,
        engine: &rvp_engine::Engine,
        imported: Settings,
    ) -> crate::preferences::Applied {
        let (settings, default_values) = running(imported);
        self.default_values = default_values;
        self.preset_before_auto = None;
        self.apply_settings(engine, settings)
    }

    /// Writes what settings.json holds to `path` (Export): the default
    /// preset's values with the presets, the active one's values as they
    /// run.
    ///
    /// # Errors
    /// Fails on I/O errors.
    pub fn export_settings(&self, path: &std::path::Path) -> std::io::Result<()> {
        crate::storage::write_json(path, &self.stored_settings())
    }

    /// Renames the preset `old` to `new` (the page's Rename): the list,
    /// the active preset's name and the one auto-activation returns to
    /// change together, so the values running stay with their preset and
    /// the default's with the default.
    ///
    /// # Errors
    /// The reason when `new` is no free name or `old` no preset.
    pub fn rename_preset(
        &mut self,
        engine: &rvp_engine::Engine,
        old: &str,
        new: &str,
    ) -> Result<crate::preferences::Applied, String> {
        let new = new.trim();
        let mut settings = self.settings.clone();
        let presets = &mut settings.presets;
        if new.is_empty() {
            return Err("A preset needs a name".to_owned());
        }
        if presets.list.iter().any(|preset| preset.name == new) {
            return Err(format!("A preset is named {new} already"));
        }
        let preset = presets
            .list
            .iter_mut()
            .find(|preset| preset.name == old)
            .ok_or_else(|| format!("No preset is named {old}"))?;
        new.clone_into(&mut preset.name);
        if presets.active == old {
            new.clone_into(&mut presets.active);
        }
        if self.preset_before_auto.as_deref() == Some(old) {
            self.preset_before_auto = Some(new.to_owned());
        }
        Ok(self.apply_settings(engine, settings))
    }

    /// Adds a copy of the preset `name` (the page's Duplicate) as
    /// "<name> (copy)": its conditions, and its values as they are in
    /// force (the active preset's are the settings running).
    ///
    /// # Errors
    /// The reason when `name` is no preset.
    pub fn duplicate_preset(
        &mut self,
        engine: &rvp_engine::Engine,
        name: &str,
    ) -> Result<crate::preferences::Applied, String> {
        let mut settings = self.settings.clone();
        let presets = &mut settings.presets;
        let mut copy = presets
            .list
            .iter()
            .find(|preset| preset.name == name)
            .cloned()
            .ok_or_else(|| format!("No preset is named {name}"))?;
        if presets.active == name {
            copy.values = values(&self.settings);
        }
        // One more name than there are presets: one of them is free.
        copy.name = (1..=presets.list.len() + 1)
            .map(|number| {
                if number == 1 {
                    format!("{name} (copy)")
                } else {
                    format!("{name} (copy {number})")
                }
            })
            .find(|candidate| !presets.list.iter().any(|preset| preset.name == *candidate))
            .unwrap_or_default();
        presets.list.push(copy);
        Ok(self.apply_settings(engine, settings))
    }

    /// What settings.json holds now: the default's values with the
    /// presets, the active one's values taken from the settings running.
    #[must_use]
    pub(crate) fn stored_settings(&self) -> Value {
        // A skin is saved once the window shows it (`crate::skins`).
        let settings = self.settings_to_save();
        let active = &settings.presets.active;
        let Some(default_values) = self.default_values.as_ref().filter(|_| !active.is_empty())
        else {
            return serde_json::to_value(&*settings).unwrap_or(Value::Null);
        };
        let mut presets = settings.presets.clone();
        if let Some(preset) = presets
            .list
            .iter_mut()
            .find(|preset| preset.name == *active)
        {
            preset.values = values(&settings);
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
            crate::log::report(
                crate::log::Level::Warning,
                &format!("preset {name}: {note}"),
            );
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

/// settings.json as several RVPs write it ([`crate::storage`]): this
/// instance's file `ours`, changed from `base`, put onto `disk` by setting,
/// with the presets' list by preset name (each preset by setting too).
pub(crate) fn merge_settings(disk: Value, base: &Value, ours: &Value) -> Value {
    /// Takes the presets' list out of a file.
    fn take_list(file: &mut Value) -> Option<Vec<Value>> {
        match file.get_mut("presets")?.as_object_mut()?.remove("list")? {
            Value::Array(list) => Some(list),
            _ => None,
        }
    }
    let (mut disk, mut base, mut ours) = (disk, base.clone(), ours.clone());
    let lists = (
        take_list(&mut disk),
        take_list(&mut base),
        take_list(&mut ours),
    );
    let mut merged = crate::storage::merge_value(disk, &base, &ours);
    let list = match lists {
        (Some(disk), Some(base), Some(ours)) => Some(crate::storage::merge_list(
            disk,
            &base,
            &ours,
            |preset| preset["name"].to_string(),
            crate::storage::merge_value,
        )),
        (disk, base, ours) => crate::storage::pick(disk, &base, &ours),
    };
    if let (Some(list), Value::Object(file)) = (list, &mut merged)
        && let Value::Object(presets) = file
            .entry("presets")
            .or_insert_with(|| Value::Object(serde_json::Map::new()))
    {
        presets.insert("list".to_owned(), Value::Array(list));
    }
    merged
}

/// settings.json's rule for several RVPs ([`crate::storage::Merge`]).
pub(crate) fn merge_settings_file(disk: &[u8], base: &[u8], ours: &[u8]) -> Option<Vec<u8>> {
    crate::storage::merge_json(disk, base, ours, merge_settings)
}

/// A preset's name as RVP shows it.
fn shown(name: &str) -> &str {
    if name.is_empty() { "Default" } else { name }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two RVPs' settings: the presets by name (a rename removes the old
    /// one), each preset's values and the rest by setting.
    #[test]
    fn settings_merge_presets_by_name() {
        let preset =
            |name: &str, values: Value| serde_json::json!({ "name": name, "values": values });
        let file = |step: u32, active: &str, list: Vec<Value>| serde_json::json!({ "volume-step": step, "presets": { "active": active, "list": list } });
        let base = file(
            5,
            "",
            vec![
                preset("A", serde_json::json!({ "x": 1, "y": 1 })),
                preset("B", Value::Null),
            ],
        );
        // Here: B renamed C, A's x changed.
        let ours = file(
            5,
            "",
            vec![
                preset("A", serde_json::json!({ "x": 2, "y": 1 })),
                preset("C", Value::Null),
            ],
        );
        // There: A's y changed, D added, A active, the default's step.
        let disk = file(
            9,
            "A",
            vec![
                preset("A", serde_json::json!({ "x": 1, "y": 3 })),
                preset("B", Value::Null),
                preset("D", Value::Null),
            ],
        );
        assert_eq!(
            merge_settings(disk, &base, &ours),
            file(
                9,
                "A",
                vec![
                    preset("A", serde_json::json!({ "x": 2, "y": 3 })),
                    preset("C", Value::Null),
                    preset("D", Value::Null),
                ],
            )
        );
        // A file without presets gets this instance's list.
        let without = serde_json::json!({ "volume-step": 9 });
        let merged = merge_settings(without, &base, &ours);
        assert_eq!(merged["presets"]["list"], ours["presets"]["list"]);
    }

    #[test]
    fn wildcards_match_names() {
        assert!(wildcard("*.MKV", "film.mkv"));
        assert!(wildcard("ep??.mp4", "ep12.mp4"));
        assert!(!wildcard("ep??.mp4", "ep1.mp4"));
        assert!(wildcard("*", ""));
        assert!(wildcard("**", ""));
        assert!(!wildcard("?", ""));
        assert!(!wildcard("", "a"));
        assert!(wildcard("*s01*", "Show S01E02.mkv"));
        assert!(wildcard("a*b*c", "aXbYbZc"));
        assert!(!wildcard("a*b*c", "aXbYbZ"));
        assert!(wildcard("*ab", "aab"));
        assert!(!wildcard("*.mkv", "film.mkv.part"));
        assert!(wildcard("Ü*", "über.mkv"));
    }

    #[test]
    fn many_stars_match_in_linear_time() {
        // A rule of twelve stars against a name of 255 characters that keeps
        // matching the text between them and fails only at its end: trying
        // every split of the name between the stars would never finish.
        let pattern = format!("{}*b", "*a".repeat(11));
        assert_eq!(pattern.matches('*').count(), 12);
        let name = "a".repeat(255);
        let started = std::time::Instant::now();
        assert!(!wildcard(&pattern, &name));
        assert!(wildcard(&pattern, &format!("{name}b")));
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
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
