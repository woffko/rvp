//! User settings (`settings.json` in the configuration directory). Missing
//! fields take their defaults, so older files keep working.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::action::{Action, ImageFormat, ListAction, OnTop, Repeat, SeekStep};
use crate::equalizer::Equalizer;
use crate::input::{Bindings, KeyChord, MouseGesture};
use crate::normalizer::Normalizer;
use crate::video::{Color, Deinterlace, Shaders, VapourSynth, VideoEffect};

/// RVP's settings.
// Each flag is an independent option as the settings file stores it.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Settings {
    /// The `.dsf` skin to load. Unset: RVP Default, the skin built into
    /// RVP.
    pub skin: Option<PathBuf>,
    /// mpv `hwdec`.
    pub hwdec: String,
    /// mpv `input-ipc-server` for SVP; empty turns it off.
    pub ipc_server: String,
    /// Seek steps in seconds: Left/Right, Ctrl, Shift, Ctrl+Alt. PotPlayer's
    /// menu shows them only as `%s`; 5 s, 30 s, 1 min and 5 min were kept
    /// by the owner (2026-10-11).
    pub seek_seconds: [f64; 4],
    /// Volume step for Up/Down and the wheel, in percent (1 %, owner
    /// decision 2026-10-11, matching the owner's `AudioVolumeUnit=1`).
    pub volume_step: f64,
    /// Resume every file at its last position (PotPlayer
    /// `RememberPosition=1`).
    pub remember_positions: bool,
    /// Without files on the command line, reopen the last file paused at
    /// its position.
    pub reopen_last_file: bool,
    /// Resize the window to show a newly opened video at 100 %, within the
    /// screen (PotPlayer's behaviour, R1 finding 2).
    pub fit_window_to_video: bool,
    /// While a video plays, resizing keeps the video's aspect ratio (R1
    /// finding 1).
    pub keep_aspect_window: bool,
    /// Keep the screen saver and the display's power saving off while video
    /// plays (PotPlayer's "Prevent screen saver during playback").
    pub prevent_screen_saver: bool,
    /// The highest volume in percent; above 100 the sound is amplified.
    pub volume_max: f64,
    /// The audio normalizer (S5).
    pub normalizer: Normalizer,
    /// Windows drive letters in imported PotPlayer playlists and the Linux
    /// folders they stand for (`"V:": "/mnt/media"`); other drives become
    /// `/mnt/<letter>`. Not used on Windows, where paths stay as they are.
    pub drive_map: BTreeMap<String, String>,
    /// Where PotPlayer keeps its playlists; on Windows the default is
    /// `%APPDATA%\PotPlayerMini64\Playlist`.
    pub potplayer_playlists: Option<PathBuf>,
    /// Key binding changes over PotPlayer's defaults: key (`Ctrl+Left`) to
    /// action name (`seek-backward-medium`); an empty name unbinds the key.
    pub keys: BTreeMap<String, String>,
    /// Mouse binding changes: gesture name (`left-click`) to action name.
    pub mouse: BTreeMap<String, String>,
    /// Playlist key changes (while the panel has the keyboard): key to
    /// playlist action name (`select-all`).
    pub playlist_keys: BTreeMap<String, String>,
    /// Seek to keyframes instead of exactly (PotPlayer's "Use keyframe to
    /// jump to a time"; off: the owner keeps exact seeks, 2026-10-11).
    pub keyframe_seeks: bool,
    /// What happens at the end of a file (PotPlayer's "Repeat Playback").
    pub repeat: Repeat,
    /// "Stay on Top".
    pub on_top: OnTop,
    /// Where saved frames go; unset: `Pictures/RVP` in the home folder.
    pub image_folder: Option<PathBuf>,
    pub image_format: ImageFormat,
    /// Audio sync step in seconds (Shift+, and Shift+.).
    pub audio_delay_step: f64,
    /// Subtitle sync step in seconds (< and >).
    pub subtitle_delay_step: f64,
    /// mpv `audio-device` (`auto`: the system's default output).
    pub audio_device: String,
    /// PotPlayer's Output Speakers (`crate::speakers`): `stereo`
    /// (PotPlayer's mix, the default), `mono`, `2.1`, `5.1`, `7.1` or
    /// `same` (as the source).
    pub output_speakers: String,
    /// mpv `video-sync` (`audio`: the video follows the audio clock).
    pub video_sync: String,
    /// mpv `scale` (the upscaler); empty: mpv's default.
    pub scaler: String,
    /// Preferred subtitle languages, comma separated (mpv `slang`).
    pub subtitle_languages: String,
    /// Load subtitle files next to the video (mpv `sub-auto`: `no`,
    /// `exact`, `fuzzy` or `all`).
    pub subtitle_auto: String,
    /// Subtitle size (mpv `sub-scale`).
    pub subtitle_scale: f64,
    /// Subtitle font family (mpv `sub-font`); empty: mpv's default.
    pub subtitle_font: String,
    /// Colour adjustments (brightness, contrast, saturation, hue, gamma).
    pub color: Color,
    /// mpv's deinterlacing.
    pub deinterlace: Deinterlace,
    /// RVP's video effects that are on (their own filters, M6).
    pub effects: BTreeSet<VideoEffect>,
    /// mpv's debanding (the renderer's).
    pub deband: bool,
    /// GLSL shaders.
    pub shaders: Shaders,
    /// VapourSynth script slots and runtime.
    pub vapoursynth: VapourSynth,
    /// The ten-band equalizer.
    pub equalizer: Equalizer,
    /// The rest of the audio processing (M10).
    pub audio: crate::audio::Audio,
    /// M11's playback options (seek bar, skips, A-B repeat, bookmarks,
    /// when playback finishes).
    pub playback: crate::playback::Playback,
    /// The playlist panel's options (M11).
    pub playlist: crate::albums::ListSettings,
    /// The scene browsers (M11).
    pub scenes: crate::scenes::SceneSettings,
    /// The Open menu's options (M11).
    pub open: crate::sources::OpenSettings,
    /// Capture devices and discs (M11).
    pub devices: crate::sources::Devices,
    /// The picture's geometry (M12).
    pub picture: crate::picture::Picture,
    /// The video effects' strengths (M12).
    pub video_effects: crate::video::EffectOptions,
    /// 3D video (M12).
    pub stereo: crate::stereo::Stereo,
    /// 360° video (M12).
    pub vr: crate::stereo::Vr,
    /// The window's sizes and moves (M12).
    pub window: crate::window::WindowSettings,
    /// M12's capture options (file names, consecutive images, the
    /// thumbnail image).
    pub capture: crate::capture::CaptureSettings,
    /// M12's stream and video recording folders.
    pub record: crate::record::RecordSettings,
    /// M12's visualizations for audio files.
    pub visual: crate::visual::VisualSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            skin: None,
            hwdec: "auto-copy".to_owned(),
            ipc_server: rvp_engine::default_ipc_server().to_owned(),
            seek_seconds: [5.0, 30.0, 60.0, 300.0],
            volume_step: 1.0,
            remember_positions: true,
            reopen_last_file: true,
            fit_window_to_video: true,
            keep_aspect_window: true,
            prevent_screen_saver: true,
            volume_max: 200.0,
            normalizer: Normalizer::default(),
            drive_map: BTreeMap::from([("V:".to_owned(), "/mnt/media".to_owned())]),
            potplayer_playlists: None,
            keys: BTreeMap::new(),
            mouse: BTreeMap::new(),
            playlist_keys: BTreeMap::new(),
            keyframe_seeks: false,
            repeat: Repeat::Off,
            on_top: OnTop::Never,
            image_folder: None,
            image_format: ImageFormat::Png,
            audio_delay_step: 0.05,
            subtitle_delay_step: 0.5,
            audio_device: "auto".to_owned(),
            output_speakers: crate::speakers::DEFAULT.to_owned(),
            video_sync: "audio".to_owned(),
            scaler: String::new(),
            subtitle_languages: String::new(),
            subtitle_auto: "fuzzy".to_owned(),
            subtitle_scale: 1.0,
            subtitle_font: String::new(),
            color: Color::default(),
            deinterlace: Deinterlace::default(),
            effects: BTreeSet::new(),
            deband: false,
            shaders: Shaders::default(),
            vapoursynth: VapourSynth::default(),
            equalizer: Equalizer::default(),
            audio: crate::audio::Audio::default(),
            playback: crate::playback::Playback::default(),
            playlist: crate::albums::ListSettings::default(),
            scenes: crate::scenes::SceneSettings::default(),
            open: crate::sources::OpenSettings::default(),
            devices: crate::sources::Devices::default(),
            picture: crate::picture::Picture::default(),
            video_effects: crate::video::EffectOptions::default(),
            stereo: crate::stereo::Stereo::default(),
            vr: crate::stereo::Vr::default(),
            window: crate::window::WindowSettings::default(),
            capture: crate::capture::CaptureSettings::default(),
            record: crate::record::RecordSettings::default(),
            visual: crate::visual::VisualSettings::default(),
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
        for (key, name) in &self.playlist_keys {
            let Some(chord) = KeyChord::parse(key) else {
                warnings.push(format!("unknown key {key:?} in playlist-keys"));
                continue;
            };
            if name.is_empty() {
                bindings.list_keys.remove(&chord);
            } else if let Some(action) = ListAction::from_name(name) {
                bindings.list_keys.insert(chord, action);
            } else {
                warnings.push(format!("unknown playlist action {name:?} for key {key}"));
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
