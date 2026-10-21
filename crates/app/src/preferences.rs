//! The settings window's logic (M5): checking settings, changing one by
//! its place in the settings file, applying a change to the running player,
//! and importing and exporting the settings file. The window itself is
//! `shell/qml/Settings.qml`; portable mode is [`crate::Dirs`]'s.
//!
//! * Values out of range are brought into range with a note
//!   ([`Settings::validate`]); the file is read with this check at start and
//!   at import, and every change from the window passes it.
//! * A change applies at once: mpv's options are set on the running player
//!   (decoding, output, sync, scaler, audio device, volume limit, the
//!   normalizer, subtitle options and SVP's IPC endpoint), the bindings are
//!   rebuilt, the shell shows another skin or colour theme at once (M14),
//!   and the rest is read where it is used. VapourSynth's library, the
//!   process priority and the renderer wait for the next start.
//! * Import reads a settings file (unknown fields are named and ignored)
//!   and applies it as one change; export writes the current settings.

use std::io;
use std::path::Path;

use rvp_engine::Engine;
use serde_json::{Map, Value};

use crate::app::App;
use crate::settings::Settings;
use crate::storage;

/// mpv `hwdec` values the window offers (others typed into the file stay).
pub const HWDEC: &[&str] = &[
    "no",
    "auto-safe",
    "auto",
    "auto-copy",
    "d3d11va",
    "d3d11va-copy",
    "dxva2-copy",
    "nvdec",
    "nvdec-copy",
    "vaapi",
    "vaapi-copy",
    "vulkan",
    "vulkan-copy",
];

/// mpv `video-sync` modes.
pub const VIDEO_SYNC: &[&str] = &[
    "audio",
    "display-resample",
    "display-resample-vdrop",
    "display-resample-desync",
    "display-tempo",
    "display-vdrop",
    "display-adrop",
    "display-desync",
    "desync",
];

/// mpv `sub-auto` modes.
pub const SUBTITLE_AUTO: &[&str] = &["no", "exact", "fuzzy", "all"];

/// mpv upscalers; empty is mpv's default.
pub const SCALERS: &[&str] = &[
    "",
    "bilinear",
    "bicubic_fast",
    "spline36",
    "lanczos",
    "ewa_lanczos",
    "ewa_lanczossharp",
    "mitchell",
    "catmull_rom",
];

fn clamp(value: &mut f64, low: f64, high: f64, name: &str, notes: &mut Vec<String>) {
    let repaired = if value.is_finite() {
        value.clamp(low, high)
    } else {
        low
    };
    if (repaired - *value).abs() > f64::EPSILON || !value.is_finite() {
        notes.push(format!("{name} must be {low} to {high}: {repaired} used"));
        *value = repaired;
    }
}

fn one_of(
    value: &mut String,
    choices: &[&str],
    fallback: &str,
    name: &str,
    notes: &mut Vec<String>,
) {
    if !choices.contains(&value.as_str()) {
        notes.push(format!(
            "{name} {value:?} is not one of {}: {fallback} used",
            choices.join(", ")
        ));
        fallback.clone_into(value);
    }
}

/// An mpv option value: letters, digits and `-_,.`.
fn plain(value: &mut String, fallback: &str, name: &str, notes: &mut Vec<String>) {
    let ok = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_,.".contains(c));
    if !ok {
        notes.push(format!(
            "{name} {value:?} is not an mpv value: {fallback} used"
        ));
        fallback.clone_into(value);
    }
}

impl Settings {
    /// Brings every value into its range; returns a note per repair.
    pub fn validate(&mut self) -> Vec<String> {
        let mut notes = Vec::new();
        self.validate_numbers(&mut notes);
        self.validate_texts(&mut notes);
        self.validate_subtitle_font(&mut notes);
        self.audio.validate(&mut notes);
        self.playback.validate(&mut notes);
        self.playlist.validate(&mut notes);
        self.scenes.validate(&mut notes);
        self.open.validate(&mut notes);
        self.devices.validate(&mut notes);
        self.picture.validate(&mut notes);
        self.video_effects.validate(&mut notes);
        self.stereo.validate(&mut notes);
        self.vr.validate(&mut notes);
        self.window.validate(&mut notes);
        self.capture.validate(&mut notes);
        self.visual.validate(&mut notes);
        self.render.validate(&mut notes);
        self.decoders.validate(&mut notes);
        self.smoothing.validate(&mut notes);
        self.subtitles.validate(&mut notes);
        self.speech.validate(&mut notes);
        self.copy_time.validate(&mut notes);
        self.skins.validate(&mut notes);
        notes
    }

    /// A font family name: one line of printable text.
    fn validate_subtitle_font(&mut self, notes: &mut Vec<String>) {
        let trimmed = self.subtitle_font.trim();
        if trimmed.len() > 100 || trimmed.chars().any(char::is_control) {
            notes.push(format!(
                "subtitle-font {:?} is not a font family: mpv's default used",
                self.subtitle_font
            ));
            self.subtitle_font.clear();
        } else if trimmed.len() != self.subtitle_font.len() {
            self.subtitle_font = trimmed.to_owned();
        }
    }

    fn validate_numbers(&mut self, notes: &mut Vec<String>) {
        let names = ["seek step 1", "seek step 2", "seek step 3", "seek step 4"];
        for (seconds, name) in self.seek_seconds.iter_mut().zip(names) {
            clamp(seconds, 0.1, 3600.0, name, notes);
        }
        clamp(&mut self.volume_step, 0.1, 50.0, "volume-step", notes);
        clamp(&mut self.volume_max, 100.0, 1000.0, "volume-max", notes);
        if crate::speakers::choice(&self.output_speakers).is_none() {
            notes.push(format!(
                "output-speakers {:?} is not one of RVP's: stereo used",
                self.output_speakers
            ));
            crate::speakers::DEFAULT.clone_into(&mut self.output_speakers);
        }
        clamp(
            &mut self.normalizer.level,
            10.0,
            100.0,
            "normalizer level",
            notes,
        );
        clamp(
            &mut self.normalizer.release,
            5.0,
            10.0,
            "normalizer release time",
            notes,
        );
        clamp(
            &mut self.audio_delay_step,
            0.001,
            10.0,
            "audio-delay-step",
            notes,
        );
        clamp(
            &mut self.subtitle_delay_step,
            0.001,
            60.0,
            "subtitle-delay-step",
            notes,
        );
        clamp(&mut self.subtitle_scale, 0.3, 3.0, "subtitle-scale", notes);
        for kind in crate::video::ColorKind::ALL {
            let value = self.color.get(kind);
            if !(-100..=100).contains(&value) {
                notes.push(format!(
                    "color {} must be -100 to 100: {} used",
                    kind.property(),
                    value.clamp(-100, 100)
                ));
                self.color.set(kind, value);
            }
        }
        let range = crate::equalizer::RANGE;
        for (band, gain) in self.equalizer.bands.iter_mut().enumerate() {
            let mut value = *gain;
            clamp(
                &mut value,
                -range,
                range,
                &format!("equalizer band {}", band + 1),
                notes,
            );
            *gain = value;
        }
        let frames = self.vapoursynth.buffered_frames;
        if !(1..=64).contains(&frames) {
            let repaired = frames.clamp(1, 64);
            notes.push(format!(
                "vapoursynth buffered-frames must be 1 to 64: {repaired} used"
            ));
            self.vapoursynth.buffered_frames = repaired;
        }
    }

    fn validate_texts(&mut self, notes: &mut Vec<String>) {
        plain(&mut self.hwdec, "auto-copy", "hwdec", notes);
        plain(&mut self.audio_device, "auto", "audio-device", notes);
        one_of(
            &mut self.video_sync,
            VIDEO_SYNC,
            "audio",
            "video-sync",
            notes,
        );
        one_of(
            &mut self.subtitle_auto,
            SUBTITLE_AUTO,
            "fuzzy",
            "subtitle-auto",
            notes,
        );
        if !self.scaler.is_empty()
            && !self
                .scaler
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            notes.push(format!(
                "scaler {:?} is not an mpv scaler: mpv's default used",
                self.scaler
            ));
            self.scaler.clear();
        }
        if self
            .subtitle_languages
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || ", -_".contains(c)))
        {
            notes.push(format!(
                "subtitle-languages {:?} must be language codes separated by commas: cleared",
                self.subtitle_languages
            ));
            self.subtitle_languages.clear();
        }
        self.drive_map.retain(|drive, folder| {
            let letter = drive.len() == 2
                && drive.as_bytes()[0].is_ascii_alphabetic()
                && drive.as_bytes()[1] == b':';
            let ok = letter && folder.starts_with('/');
            if !ok {
                notes.push(format!(
                    "drive-map {drive:?} = {folder:?} must map a drive (V:) to a Linux folder: removed"
                ));
            }
            ok
        });
        if let Some(skin) = &self.skin
            && !skin.is_file()
        {
            notes.push(format!("skin {} is not there", skin.display()));
        }
        for file in self.shaders.files.iter().chain(&self.shaders.post_files) {
            if !file.is_file() {
                notes.push(format!("shader {} is not there", file.display()));
            }
        }
        let vs = &self.vapoursynth;
        for file in vs
            .slots
            .iter()
            .chain(&vs.plugins)
            .chain(&vs.avs_plugins)
            .chain(&vs.script)
        {
            if !file.is_file() {
                notes.push(format!("VapourSynth file {} is not there", file.display()));
            }
        }
        if let Some(library) = &vs.library
            && !library.is_file()
        {
            notes.push(format!(
                "VapourSynth library {} is not there",
                library.display()
            ));
        }
    }
}

/// The settings with the value at `path` replaced: `volume-step`,
/// `normalizer.level`, `seek-seconds.1`, `drive-map` (a whole object).
/// Values are checked ([`Settings::validate`]); the notes say what was
/// repaired.
///
/// # Errors
/// Fails when the path names no setting or the value has the wrong type.
pub fn change(
    settings: &Settings,
    path: &str,
    value: Value,
) -> Result<(Settings, Vec<String>), String> {
    let mut root = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    let mut slot = &mut root;
    for part in path.split('.') {
        slot = match slot {
            Value::Object(map) => map
                .get_mut(part)
                .ok_or_else(|| format!("no setting {path:?}"))?,
            Value::Array(items) => part
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get_mut(index))
                .ok_or_else(|| format!("no setting {path:?}"))?,
            _ => return Err(format!("no setting {path:?}")),
        };
    }
    *slot = value;
    let mut changed: Settings =
        serde_json::from_value(root).map_err(|error| format!("{path}: {error}"))?;
    let notes = changed.validate();
    Ok((changed, notes))
}

/// Field names of a JSON object that `known` lacks, with their paths.
fn unknown_fields(value: &Value, known: &Value, place: &str, out: &mut Vec<String>) {
    let (Value::Object(map), Value::Object(known_map)) = (value, known) else {
        return;
    };
    for (key, child) in map {
        let here = if place.is_empty() {
            key.clone()
        } else {
            format!("{place}.{key}")
        };
        match known_map.get(key) {
            None => out.push(here),
            // Maps of the user's own keys (drive-map, keys, mouse) are
            // empty here: they have no fixed fields.
            Some(inner @ Value::Object(fields)) if !fields.is_empty() => {
                unknown_fields(child, inner, &here, out);
            }
            Some(_) => {}
        }
    }
}

/// Reads a settings file to import: missing fields take their defaults,
/// unknown ones are named in the notes, values are checked.
///
/// # Errors
/// Fails when the file cannot be read or is not a settings object.
pub fn import(path: &Path) -> Result<(Settings, Vec<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{}: not a settings file", path.display()));
    }
    let known = serde_json::to_value(Settings::default()).map_err(|e| e.to_string())?;
    let mut unknown = Vec::new();
    // Only fixed fields count (drive-map holds the user's own keys).
    let known_fixed = match &known {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let value = if matches!(
                        key.as_str(),
                        "normalizer"
                            | "audio"
                            | "playback"
                            | "playlist"
                            | "scenes"
                            | "open"
                            | "devices"
                            | "smoothing"
                            | "subtitles"
                            | "speech"
                            | "input"
                            | "copy-time"
                            | "skins"
                    ) {
                        value.clone()
                    } else if value.is_object() {
                        Value::Object(Map::new())
                    } else {
                        value.clone()
                    };
                    (key.clone(), value)
                })
                .collect(),
        ),
        other => other.clone(),
    };
    unknown_fields(&value, &known_fixed, "", &mut unknown);
    let mut settings: Settings =
        serde_json::from_value(value).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut notes: Vec<String> = unknown
        .into_iter()
        .map(|field| format!("unknown setting {field:?} ignored"))
        .collect();
    notes.extend(settings.validate());
    Ok((settings, notes))
}

/// Writes the settings to `path` (pretty JSON, as `settings.json`).
///
/// # Errors
/// Fails on I/O errors.
pub fn export(settings: &Settings, path: &Path) -> io::Result<()> {
    storage::write_json(path, settings)
}

/// What applying settings did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// Repairs and binding warnings.
    pub notes: Vec<String>,
    /// A change waits for the next start (VapourSynth's library, the
    /// priority, the renderer).
    pub restart: bool,
}

impl App {
    /// Makes `new` the settings: sets what changed on the running player,
    /// rebuilds the bindings and saves the file.
    pub fn apply_settings(&mut self, engine: &Engine, mut new: Settings) -> Applied {
        let mut applied = Applied {
            notes: new.validate(),
            restart: false,
        };
        let old = std::mem::replace(&mut self.settings, new.clone());
        let mut set = |name: &str, value: &str| {
            if let Err(error) = engine.set_text(name, value) {
                applied.notes.push(format!("cannot set {name}: {error}"));
            }
        };
        let mut endpoint_changed = None;
        if old.ipc_server != new.ipc_server {
            // Another process may hold the endpoint (see crate::svp).
            let endpoint = crate::svp::choose_endpoint(
                &new.ipc_server,
                std::process::id(),
                crate::svp::endpoint_in_use,
            );
            if let Some(endpoint) = &endpoint {
                crate::svp::prepare_endpoint(endpoint);
            }
            set("input-ipc-server", endpoint.as_deref().unwrap_or(""));
            endpoint_changed = Some(endpoint);
        }
        if (old.volume_max - new.volume_max).abs() > f64::EPSILON {
            set("volume-max", &crate::volume::mpv_volume_max(new.volume_max));
        }
        if old.audio_device != new.audio_device {
            set("audio-device", &new.audio_device);
        }
        if old.video_sync != new.video_sync {
            // Blend keeps display-resample while it runs.
            let video_sync = self.effective_video_sync().to_owned();
            set("video-sync", &video_sync);
        }
        if old.scaler != new.scaler {
            let scaler = if new.scaler.is_empty() {
                engine
                    .get_text("option-info/scale/default-value")
                    .unwrap_or_else(|| "lanczos".to_owned())
            } else {
                // The GPU power budget's lighter scaler while it is taken.
                self.effective_scaler(&new.scaler).to_owned()
            };
            set("scale", &scaler);
        }
        self.subtitle_settings_changed(engine, &old, &mut applied.notes);
        if old.skin != new.skin {
            // The user's skin now, not the command line's (M14).
            self.skin_override = crate::skins::SkinOverride::Off;
        }
        if old.normalizer.enabled != new.normalizer.enabled {
            self.configure_normalizer();
            // As a toggle does: play on from here with the new chain.
            self.audio_chain_changed(engine);
        } else if old.normalizer != new.normalizer {
            self.configure_normalizer();
        }
        if (&old.keys, &old.mouse, &old.playlist_keys, &old.input)
            != (&new.keys, &new.mouse, &new.playlist_keys, &new.input)
        {
            let (bindings, warnings) = new.bindings();
            self.bindings = bindings;
            applied.notes.extend(warnings);
        }
        if old.on_top != new.on_top {
            // The window gets the new state with the next update.
            self.session.on_top = None;
        }
        if let Some(endpoint) = endpoint_changed {
            self.svp.endpoint_changed(endpoint);
            if self.model.media_loaded() {
                self.svp.file_started(
                    crate::svp::svp_mode(&self.settings),
                    std::time::Instant::now(),
                );
            }
        }
        if old.output_speakers != new.output_speakers {
            self.speakers_changed(engine);
        }
        if old.smoothing != self.settings.smoothing || old.vapoursynth != self.settings.vapoursynth
        {
            self.apply_smoothing(engine);
        }
        self.apply_audio_settings(engine, &old, &mut applied.notes);
        self.apply_m6_settings(engine, &old, &mut applied);
        self.apply_playback_settings(engine, &old);
        applied.restart = taken_at_start(&old, &new);
        self.save_settings();
        applied
    }

    /// M6's part of [`App::apply_settings`]: decoding, video filters, the
    /// renderer's picture settings and the equalizer.
    fn apply_m6_settings(&mut self, engine: &Engine, old: &Settings, applied: &mut Applied) {
        let new = &self.settings;
        let filters = old.effects != new.effects
            || old.vapoursynth != new.vapoursynth
            || old.video_effects != new.video_effects;
        if filters || old.hwdec != new.hwdec || old.ipc_server != new.ipc_server {
            self.apply_decoding(engine);
        }
        if filters && let Err(error) = self.apply_video_filters(engine) {
            applied.notes.push(error);
        }
        let new = &self.settings;
        if (old.color != new.color
            || old.deinterlace != new.deinterlace
            || old.deband != new.deband
            || old
                .video_effects
                .deband
                .iter()
                .zip(new.video_effects.deband)
                .any(|(a, b)| (a - b).abs() > 1e-9)
            || old.shaders != new.shaders)
            && let Err(error) = self.apply_picture_settings(engine)
        {
            applied.notes.push(format!("picture settings: {error}"));
        }
        self.apply_m12_settings(engine, old);
        let new = &self.settings;
        if old.equalizer.enabled != new.equalizer.enabled {
            self.audio_chain_changed(engine);
        } else if new.equalizer.enabled
            && old
                .equalizer
                .bands
                .iter()
                .zip(new.equalizer.bands)
                .any(|(a, b)| (a - b).abs() > 1e-9)
        {
            self.update_equalizer_bands(engine);
        }
    }

    /// The settings window's data: the settings, the choices it offers
    /// and where the file is. With `devices` the audio devices mpv knows
    /// are read too (when the window opens: mpv asks the system, which can
    /// take long enough to cost video frames).
    #[must_use]
    pub fn settings_view(&self, engine: &Engine, portable: bool, devices: bool) -> Value {
        let devices: Value = devices
            .then(|| engine.get_json("audio-device-list"))
            .flatten()
            .unwrap_or_else(|| Value::Array(Vec::new()));
        let skins: Vec<String> = self
            .skin_choices()
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        serde_json::json!({
            "settings": self.settings,
            "choices": {
                "hwdec": HWDEC,
                "video-sync": VIDEO_SYNC,
                "subtitle-auto": SUBTITLE_AUTO,
                "scaler": SCALERS,
                "resize-method": crate::picture::RESIZE_METHODS,
                "render-scaler": crate::render::RENDER_SCALERS,
                "matrix": crate::render::MATRICES,
                "deinterlacer": crate::render::DEINTERLACERS,
                "field-order": crate::render::FIELD_ORDERS,
                "tone-mapping": crate::render::TONE_MAPPINGS,
                // Direct3D 11 is Windows' only.
                "super-resolution": crate::render::SUPER_RESOLUTIONS,
                "renderer": if cfg!(windows) {
                    crate::render::RENDERERS
                } else {
                    &crate::render::RENDERERS[..1]
                },
                "hardware-codecs": crate::decoders::HARDWARE_CODECS
                    .iter()
                    .map(|(codec, text)| serde_json::json!({ "codec": codec, "text": text }))
                    .collect::<Vec<_>>(),
                "shader-choices": self
                    .shader_catalogue()
                    .iter()
                    .map(|entry| serde_json::json!({
                        "name": entry.name,
                        "path": entry.path,
                        "stage": match entry.stage {
                            Some(crate::hlsl::Stage::Pre) => "pre",
                            Some(crate::hlsl::Stage::Post) => "post",
                            None => "any",
                        },
                    }))
                    .collect::<Vec<_>>(),
                "potplayer-shaders": self.potplayer_shader_folder(),
                "audio-device": devices,
                "skin": skins,
                "output-speakers": crate::speakers::CHOICES
                    .iter()
                    .map(|choice| serde_json::json!({ "value": choice.value, "label": choice.label }))
                    .collect::<Vec<_>>(),
                "sample-rate": crate::audio::SAMPLE_RATES,
                "equalizer-preset": crate::equalizer::PRESETS
                    .iter()
                    .map(|preset| preset.name)
                    .collect::<Vec<_>>(),
                "smoothing-target": crate::smoothing::TARGETS,
                "smoothing-max-height": crate::smoothing::MAX_HEIGHTS,
                "subtitles.ass-override": crate::subtitles::ASS_OVERRIDE,
                "subtitles.border-style": crate::subtitles::BORDER_STYLE,
                "subtitles.align": crate::subtitles::ALIGN,
                "subtitles.encoding": crate::subtitles::ENCODINGS,
                "subtitles.browser.times": crate::subtitle_editor::BROWSER_TIMES,
                "speech.engine": crate::speech::ENGINES.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
                "speech.model": crate::speech::MODELS,
                "speech.language": crate::speech::LANGUAGES.iter().map(|(code, _)| *code).collect::<Vec<_>>(),
                "speech.auto": crate::speech::AUTO_MODES,
                "speech.realtime": crate::speech::REALTIME_MODES,
            },
            "file": self.settings_path().map(|path| path.display().to_string()),
            "portable": portable,
            "default-ipc": rvp_engine::default_ipc_server(),
            "windows": cfg!(windows),
            "svp_status": self.svp_status(),
            "svp_endpoint": self.svp.endpoint,
            "vapoursynth_runtime": self.svp.vapoursynth.as_ref().map(|path| path.display().to_string()),
            "shader_folder": self.shader_folder().display().to_string(),
            "smoothing": self.smoothing_plan().text(),
            "smoothing_engines": self.smoothing_engines(),
            // libmpv's own values of the subtitle options left unset.
            "subtitle_defaults": self.subtitle_defaults.view(),
            "fonts_folder": self
                .settings_path()
                .and_then(std::path::Path::parent)
                .map(|dir| dir.join("fonts").display().to_string()),
            "speech": self.speech_view(),
            "skin_themes": self.skin_themes(),
            "skin_shown": self.chosen_skin().map(|path| path.display().to_string()),
            "skins_folder": self.skins_folder().display().to_string(),
            "logo_list": self
                .logo_list()
                .iter()
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            "logos_folder": self.logos_folder().display().to_string(),
        })
    }

    /// The skins to choose from (`App::skin_list`, RVP Default left out).
    #[must_use]
    pub fn skin_choices(&self) -> Vec<std::path::PathBuf> {
        self.skin_list().into_iter().flatten().collect()
    }
}

/// True when a change waits for RVP's next start: VapourSynth's
/// library, the process priority, the renderer and its swapchain format.
fn taken_at_start(old: &Settings, new: &Settings) -> bool {
    old.vapoursynth.library != new.vapoursynth.library
        || old.playback.priority != new.playback.priority
        || old.render.renderer != new.render.renderer
        || old.render.ten_bit != new.render.ten_bit
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn values_out_of_range_are_repaired() {
        let mut settings = Settings {
            volume_step: 0.0,
            volume_max: 5000.0,
            seek_seconds: [5.0, f64::NAN, 60.0, 300.0],
            video_sync: "sometimes".to_owned(),
            hwdec: "auto; rm".to_owned(),
            subtitle_languages: "en,ru".to_owned(),
            output_speakers: "quad".to_owned(),
            ..Settings::default()
        };
        settings
            .drive_map
            .insert("Viid".to_owned(), "/mnt/x".to_owned());
        settings
            .drive_map
            .insert("W:".to_owned(), "relative".to_owned());
        let notes = settings.validate();
        assert!((settings.volume_step - 0.1).abs() < 1e-9);
        assert!((settings.volume_max - 1000.0).abs() < 1e-9);
        assert_eq!(settings.output_speakers, "stereo");
        assert!(
            notes.iter().any(|note| note.contains("output-speakers")),
            "{notes:?}"
        );
        assert!((settings.seek_seconds[1] - 0.1).abs() < 1e-9);
        assert_eq!(settings.video_sync, "audio");
        assert_eq!(settings.hwdec, "auto-copy");
        assert_eq!(settings.subtitle_languages, "en,ru");
        assert_eq!(
            settings.drive_map.keys().collect::<Vec<_>>(),
            ["V:"],
            "{notes:?}"
        );
        assert_eq!(notes.len(), 8, "{notes:?}");
        // The defaults are valid.
        assert!(Settings::default().validate().is_empty());
    }

    #[test]
    fn one_setting_changes_by_its_path() {
        let settings = Settings::default();
        let (changed, notes) = change(&settings, "volume-step", json!(2.5)).expect("change");
        assert!((changed.volume_step - 2.5).abs() < 1e-9);
        assert!(notes.is_empty());
        let (changed, _) = change(&settings, "normalizer.level", json!(60)).expect("nested");
        assert!((changed.normalizer.level - 60.0).abs() < 1e-9);
        let (changed, _) = change(&settings, "seek-seconds.1", json!(10)).expect("array");
        assert!((changed.seek_seconds[1] - 10.0).abs() < 1e-9);
        let (changed, notes) = change(&settings, "subtitle-scale", json!(9)).expect("repaired");
        assert!((changed.subtitle_scale - 3.0).abs() < 1e-9);
        assert_eq!(notes.len(), 1);
        let (changed, _) =
            change(&settings, "drive-map", json!({"X:": "/mnt/x"})).expect("whole map");
        assert_eq!(changed.drive_map.len(), 1);
        assert!(change(&settings, "no-such", json!(1)).is_err());
        assert!(change(&settings, "seek-seconds.9", json!(1)).is_err());
        assert!(change(&settings, "volume-step", json!("loud")).is_err());
    }

    #[test]
    fn import_and_export_round_trip() {
        let dir = std::env::temp_dir().join(format!("rvp-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut settings = Settings {
            volume_step: 3.0,
            ..Settings::default()
        };
        settings.keys.insert("Ctrl+B".into(), "frame-back".into());
        let file = dir.join("exported.json");
        export(&settings, &file).expect("export");
        let (back, notes) = import(&file).expect("import");
        assert_eq!(back, settings);
        assert!(notes.is_empty(), "{notes:?}");

        // Unknown fields are named; bad values repaired; the rest default.
        let file = dir.join("old.json");
        std::fs::write(
            &file,
            r#"{"volume-step": 0, "colour": "red", "normalizer": {"level": 50, "shape": 1},
                "drive-map": {"Q:": "/mnt/q"}}"#,
        )
        .expect("write");
        let (imported, notes) = import(&file).expect("import");
        assert!((imported.volume_step - 0.1).abs() < 1e-9);
        assert!((imported.normalizer.level - 50.0).abs() < 1e-9);
        assert_eq!(imported.hwdec, "auto-copy");
        assert_eq!(
            imported.drive_map.get("Q:").map(String::as_str),
            Some("/mnt/q")
        );
        assert!(notes.iter().any(|n| n.contains("\"colour\"")), "{notes:?}");
        assert!(
            notes.iter().any(|n| n.contains("\"normalizer.shape\"")),
            "{notes:?}"
        );
        assert_eq!(notes.len(), 3, "{notes:?}");

        std::fs::write(&file, "[1, 2]").expect("write");
        assert!(import(&file).is_err());
        std::fs::write(&file, "{ not json").expect("write");
        assert!(import(&file).is_err());
        assert!(import(&dir.join("missing.json")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
