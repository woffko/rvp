//! The Audio menu's commands (M10) and the audio options mpv takes from
//! RVP's settings: the output, the volume methods, the effects' switches,
//! the sync, the time stretching and recording.
//!
//! * The wave volume is mpv's `ao-volume` (the stream's volume in the
//!   system's mixer, WASAPI's session or PipeWire's stream); the master
//!   volume is the output device's, which only the window can change
//!   ([`UiRequest::SystemVolume`]).
//! * Recording records what RVP plays, after its chain, through the
//!   system ([`UiRequest::Record`]): Windows' process loopback.
//! * The sync is mpv's `audio-delay`. "Never keep" sets the default sync at
//!   every file, "whole runtime" leaves it, "always" also keeps it in the
//!   settings for the next start.

use std::path::PathBuf;
use std::time::Instant;

use rvp_engine::{Engine, EngineEvent, PropertyId};

use crate::app::{App, UiRequest};
use crate::audio::second::Second;
use crate::audio::{OutputMode, SampleFormat, StereoChannel, Stretch, SyncKeep, VolumeMethod};
use crate::settings::Settings;

/// A change of the system's volume, which the window makes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemVolume {
    /// The output device's volume by this many percent.
    MasterStep(f64),
    /// The output device's volume at this fraction (0 to 1).
    MasterSet(f64),
    MasterMute,
}

/// A recording to start or stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Start(PathBuf),
    Stop,
}

/// One of the Audio menu's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudioCommand {
    Channel(StereoChannel),
    SwapChannels,
    Crystality,
    NoiseReduction,
    Freeverb,
    VoiceRemoval,
    VoiceEmphasis,
    Crossfeed,
    Method(VolumeMethod),
    ChannelVolumes,
    MasterUp,
    MasterDown,
    Preamp,
    PreampUp,
    PreampDown,
    SystemMasterUp,
    SystemMasterDown,
    SystemMasterMute,
    SystemWaveUp,
    SystemWaveDown,
    SystemWaveMute,
    SyncDialog,
    Keep(SyncKeep),
    Stretch(Stretch),
    RecordDialog,
    RecordToggle,
    RecordFolderDialog,
    OpenRecordFolder,
    RecordInMediaFolder,
}

/// A step of the individual volume's master and of the pre-amplifier, dB.
const GAIN_STEP: f64 = 1.0;

fn on_off(on: bool) -> &'static str {
    if on { "On" } else { "Off" }
}

impl App {
    /// mpv's options for the audio settings, at start: only those that
    /// differ from mpv's defaults (mpv refuses `audio-format=no`, its own
    /// default).
    #[must_use]
    pub fn audio_options(&self) -> Vec<(String, String)> {
        let audio = &self.settings.audio;
        let mut options: Vec<(&str, String)> = Vec::new();
        if audio.exclusive {
            options.push(("audio-exclusive", "yes".to_owned()));
        }
        let spdif = audio.spdif(None);
        if !spdif.is_empty() {
            options.push(("audio-spdif", spdif));
        }
        if audio.sample_format != SampleFormat::Same {
            options.push(("audio-format", audio.sample_format.mpv().to_owned()));
        }
        if audio.sample_format == SampleFormat::S16Dither {
            options.push(("audio-swresample-o", "dither_method=triangular".to_owned()));
        }
        if audio.sample_rate != 0 {
            options.push(("audio-samplerate", audio.sample_rate.to_string()));
        }
        if audio.resample_quality != crate::audio::ResampleQuality::Normal {
            let (filter_size, phase_shift) = audio.resample_quality.mpv();
            options.push(("audio-resample-filter-size", filter_size.to_string()));
            options.push(("audio-resample-phase-shift", phase_shift.to_string()));
        }
        if audio.stretch == Stretch::Pitch {
            options.push(("audio-pitch-correction", "no".to_owned()));
        }
        let delay = match audio.sync_keep {
            SyncKeep::Always => audio.kept_delay,
            SyncKeep::Never | SyncKeep::Runtime => audio.default_delay / 1000.0,
        };
        if delay.abs() > 1e-9 {
            options.push(("audio-delay", format!("{delay:.3}")));
        }
        if !audio.languages.trim().is_empty() {
            options.push(("alang", audio.languages.trim().to_owned()));
        }
        options
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect()
    }

    /// A file loaded: what the chain needs to know about its audio, the
    /// options that depend on it, and the sync it starts with.
    pub(crate) fn audio_file_loaded(&mut self, engine: &Engine) {
        let codec = engine.get_text("current-tracks/audio/codec");
        let layout = engine.get_text("current-tracks/audio/demux-channels");
        let count = engine
            .get_int("current-tracks/audio/demux-channel-count")
            .and_then(|count| u32::try_from(count).ok());
        let video = engine.get_text("current-tracks/video/codec").is_some()
            && engine.get_text("current-tracks/video/albumart").as_deref() != Some("yes");
        let source = crate::audio::chain::Source {
            codec,
            channels: layout
                .as_deref()
                .map(crate::speakers::layout_channels)
                .unwrap_or_default(),
            video,
        };
        let audio = &self.settings.audio;
        let set = |name: &str, value: &str| {
            if let Err(error) = engine.set_text(name, value) {
                eprintln!("rvp: cannot set {name}: {error}");
            }
        };
        set("audio-spdif", &audio.spdif(count));
        // Pass-through goes to its own device when one is chosen.
        let passes = audio.output_mode == OutputMode::Passthrough
            && !audio.passthrough_device.is_empty()
            && !audio.spdif(count).is_empty()
            && source.codec.as_deref().is_some_and(|codec| {
                audio
                    .passthrough_codecs
                    .split(',')
                    .any(|wanted| wanted.trim().split('-').next() == Some(codec))
            });
        let device = if passes {
            audio.passthrough_device.clone()
        } else {
            self.settings.audio_device.clone()
        };
        set("audio-device", &device);
        if audio.sync_keep == SyncKeep::Never {
            set(
                "audio-delay",
                &format!("{:.3}", audio.default_delay / 1000.0),
            );
        }
        if audio.volume_method == VolumeMethod::Master && audio.wave_to_max {
            set("ao-volume", "100");
        }
        self.choose_audio_track(engine);
        if source != self.audio_source {
            self.audio_source = source;
            self.apply_audio_chain(engine);
        }
    }

    /// The audio track a file starts with: the one chosen in it before
    /// ("Remember audio track selections"), else the preferred stream when
    /// no preferred language matches; subtitles off for the languages the
    /// settings name.
    fn choose_audio_track(&mut self, engine: &Engine) {
        let audio = &self.settings.audio;
        let tracks: Vec<rvp_engine::Track> = engine
            .tracks()
            .into_iter()
            .filter(|track| track.kind == rvp_engine::TrackKind::Audio)
            .collect();
        let path = self
            .model
            .loaded_path()
            .map(str::to_owned)
            .or_else(|| engine.get_text("path"));
        let remembered = audio
            .remember_tracks
            .then(|| {
                path.as_deref()
                    .and_then(|path| self.store().audio_track(path))
            })
            .flatten()
            .filter(|id| tracks.iter().any(|track| track.id == *id));
        let languages: Vec<String> = audio
            .languages
            .split(',')
            .map(|language| language.trim().to_ascii_lowercase())
            .filter(|language| !language.is_empty())
            .collect();
        let language_matches = tracks.iter().any(|track| {
            track
                .language
                .as_deref()
                .is_some_and(|language| languages.contains(&language.to_ascii_lowercase()))
        });
        let preferred = (audio.preferred_stream > 0 && !language_matches)
            .then(|| i64::from(audio.preferred_stream))
            .filter(|id| tracks.iter().any(|track| track.id == *id));
        let chosen = remembered.or(preferred);
        if let Some(id) = chosen {
            drop(engine.set_text("aid", &id.to_string()));
        }
        let off: Vec<String> = audio
            .subtitles_off
            .split(',')
            .map(|language| language.trim().to_ascii_lowercase())
            .filter(|language| !language.is_empty())
            .collect();
        if !off.is_empty() {
            let playing = tracks
                .iter()
                .find(|track| chosen.map_or(track.selected, |id| track.id == id));
            if playing
                .and_then(|track| track.language.as_deref())
                .is_some_and(|language| off.contains(&language.to_ascii_lowercase()))
            {
                drop(engine.set_text("sid", "no"));
            }
        }
    }

    /// An audio track was chosen in the file that plays.
    pub(crate) fn audio_track_chosen(&mut self, id: i64) {
        if self.settings.audio.remember_tracks
            && let Some(path) = self.model.loaded_path().map(str::to_owned)
        {
            self.store_mut().set_audio_track(&path, id);
        }
    }

    /// Whether an audio command's menu entry is checked.
    #[must_use]
    pub fn audio_checked(&self, command: AudioCommand) -> Option<bool> {
        let audio = &self.settings.audio;
        Some(match command {
            AudioCommand::Channel(channel) => self.session.stereo_channel == channel,
            AudioCommand::SwapChannels => audio.swap_channels,
            AudioCommand::Crystality => audio.crystality.enabled,
            AudioCommand::NoiseReduction => audio.noise_reduction.enabled,
            AudioCommand::Freeverb => audio.freeverb.enabled,
            AudioCommand::VoiceRemoval => audio.voice_removal,
            AudioCommand::VoiceEmphasis => audio.voice_emphasis,
            AudioCommand::Crossfeed => audio.crossfeed.enabled,
            AudioCommand::Method(method) => audio.volume_method == method,
            AudioCommand::ChannelVolumes => audio.channel_volumes.enabled,
            AudioCommand::Preamp => audio.preamp.enabled,
            AudioCommand::Keep(keep) => audio.sync_keep == keep,
            AudioCommand::Stretch(stretch) => audio.stretch == stretch,
            AudioCommand::RecordToggle => self.session.recording.is_some(),
            AudioCommand::RecordInMediaFolder => audio.recording.in_media_folder,
            _ => return None,
        })
    }

    /// Carries out an audio command.
    pub fn audio_command(&mut self, engine: &Engine, command: AudioCommand) -> Option<UiRequest> {
        match command {
            AudioCommand::Channel(_)
            | AudioCommand::SwapChannels
            | AudioCommand::Crystality
            | AudioCommand::NoiseReduction
            | AudioCommand::Freeverb
            | AudioCommand::VoiceRemoval
            | AudioCommand::VoiceEmphasis
            | AudioCommand::Crossfeed => Some(self.effect_command(engine, command)),
            AudioCommand::SyncDialog | AudioCommand::Keep(_) | AudioCommand::Stretch(_) => {
                Some(self.sync_command(engine, command))
            }
            AudioCommand::RecordDialog
            | AudioCommand::RecordToggle
            | AudioCommand::RecordFolderDialog
            | AudioCommand::OpenRecordFolder
            | AudioCommand::RecordInMediaFolder => Some(self.record_command(command)),
            _ => self.volume_command(engine, command),
        }
    }

    /// The channels and the effects.
    fn effect_command(&mut self, engine: &Engine, command: AudioCommand) -> UiRequest {
        match command {
            AudioCommand::Channel(channel) => {
                self.session.stereo_channel = channel;
                self.audio_chain_changed(engine);
                UiRequest::Osd(format!(
                    "Audio channel: {}",
                    match channel {
                        StereoChannel::Both => "Both",
                        StereoChannel::Left => "Left",
                        StereoChannel::Right => "Right",
                    }
                ))
            }
            AudioCommand::SwapChannels => {
                self.toggle_audio(engine, "Swap Stereo Channels", |a| &mut a.swap_channels)
            }
            AudioCommand::Crystality => {
                self.toggle_audio(engine, "Crystality", |a| &mut a.crystality.enabled)
            }
            AudioCommand::NoiseReduction => self.toggle_audio(engine, "Noise Reduction", |a| {
                &mut a.noise_reduction.enabled
            }),
            AudioCommand::Freeverb => {
                self.toggle_audio(engine, "Freeverb", |a| &mut a.freeverb.enabled)
            }
            AudioCommand::VoiceRemoval => {
                self.toggle_audio(engine, "Voice Removal", |a| &mut a.voice_removal)
            }
            AudioCommand::VoiceEmphasis => {
                self.toggle_audio(engine, "Voice Emphasis", |a| &mut a.voice_emphasis)
            }
            _ => self.toggle_audio(engine, "Crossfeed", |a| &mut a.crossfeed.enabled),
        }
    }

    /// The volume methods, the individual volume, the pre-amplifier and
    /// the system's volume.
    fn volume_command(&mut self, engine: &Engine, command: AudioCommand) -> Option<UiRequest> {
        let notice = |text: String| Some(UiRequest::Osd(text));
        match command {
            AudioCommand::Method(method) => {
                self.settings.audio.volume_method = method;
                self.save_settings();
                notice(
                    match method {
                        VolumeMethod::Player => "Volume: RVP's own",
                        VolumeMethod::Wave => "Volume: the system's wave volume",
                        VolumeMethod::Master => "Volume: the system's master volume",
                    }
                    .to_owned(),
                )
            }
            AudioCommand::ChannelVolumes => {
                Some(self.toggle_audio(engine, "Individual volume control", |a| {
                    &mut a.channel_volumes.enabled
                }))
            }
            AudioCommand::MasterUp | AudioCommand::MasterDown => {
                let step = if command == AudioCommand::MasterUp {
                    GAIN_STEP
                } else {
                    -GAIN_STEP
                };
                let volumes = &mut self.settings.audio.channel_volumes;
                volumes.master = (volumes.master + step).clamp(-40.0, 12.0);
                volumes.enabled = true;
                let master = volumes.master;
                self.audio_chain_changed(engine);
                self.save_settings();
                notice(format!("Master volume: {master:+.0} dB"))
            }
            AudioCommand::Preamp => Some(self.toggle_audio(
                engine,
                "Pre-amplify AC3/DTS Volume to proper level",
                |a| &mut a.preamp.enabled,
            )),
            AudioCommand::PreampUp | AudioCommand::PreampDown => {
                let step = if command == AudioCommand::PreampUp {
                    GAIN_STEP
                } else {
                    -GAIN_STEP
                };
                let preamp = &mut self.settings.audio.preamp;
                preamp.gain = (preamp.gain + step).clamp(0.0, 20.0);
                preamp.enabled = true;
                let gain = preamp.gain;
                self.audio_chain_changed(engine);
                self.save_settings();
                notice(format!("PreAmp: {gain:+.0} dB"))
            }
            AudioCommand::SystemMasterUp => Some(UiRequest::SystemVolume(
                SystemVolume::MasterStep(self.settings.volume_step),
            )),
            AudioCommand::SystemMasterDown => Some(UiRequest::SystemVolume(
                SystemVolume::MasterStep(-self.settings.volume_step),
            )),
            AudioCommand::SystemMasterMute => {
                Some(UiRequest::SystemVolume(SystemVolume::MasterMute))
            }
            AudioCommand::SystemWaveUp => change_wave(engine, self.settings.volume_step),
            AudioCommand::SystemWaveDown => change_wave(engine, -self.settings.volume_step),
            AudioCommand::SystemWaveMute => {
                let mute = engine.get_text("ao-mute").as_deref() != Some("yes");
                engine.set_flag("ao-mute", mute).ok().and_then(|()| {
                    notice(format!("Wave volume: {}", if mute { "mute" } else { "on" }))
                })
            }
            _ => None,
        }
    }

    /// The sync and the time stretching.
    fn sync_command(&mut self, engine: &Engine, command: AudioCommand) -> UiRequest {
        match command {
            AudioCommand::Keep(keep) => {
                self.settings.audio.sync_keep = keep;
                if keep == SyncKeep::Always {
                    self.settings.audio.kept_delay =
                        self.model.double(PropertyId::AudioDelay).unwrap_or(0.0);
                }
                self.save_settings();
                UiRequest::Osd(
                    match keep {
                        SyncKeep::Never => "Audio sync changes: not kept",
                        SyncKeep::Runtime => "Audio sync changes: kept until RVP exits",
                        SyncKeep::Always => "Audio sync changes: always kept",
                    }
                    .to_owned(),
                )
            }
            AudioCommand::Stretch(stretch) => {
                self.settings.audio.stretch = stretch;
                drop(engine.set_flag("audio-pitch-correction", stretch != Stretch::Pitch));
                self.audio_chain_changed(engine);
                self.save_settings();
                UiRequest::Osd(
                    match stretch {
                        Stretch::HighQuality => "Audio time stretching: high quality",
                        Stretch::DefaultQuality => "Audio time stretching: default quality",
                        Stretch::Pitch => "Audio pitch shifting",
                    }
                    .to_owned(),
                )
            }
            _ => {
                UiRequest::AudioSyncDialog(self.model.double(PropertyId::AudioDelay).unwrap_or(0.0))
            }
        }
    }

    /// Audio recording.
    fn record_command(&mut self, command: AudioCommand) -> UiRequest {
        match command {
            AudioCommand::RecordDialog => UiRequest::RecordDialog(self.recording_path()),
            AudioCommand::RecordToggle => {
                if self.session.recording.is_some() {
                    self.session.recording = None;
                    UiRequest::Record(Record::Stop)
                } else {
                    let path = self.recording_path();
                    self.session.recording = Some(path.clone());
                    UiRequest::Record(Record::Start(path))
                }
            }
            AudioCommand::RecordFolderDialog => UiRequest::RecordFolderDialog,
            AudioCommand::OpenRecordFolder => {
                let folder = self.recording_folder();
                drop(std::fs::create_dir_all(&folder));
                UiRequest::OpenExternal(folder)
            }
            _ => {
                let recording = &mut self.settings.audio.recording;
                recording.in_media_folder = !recording.in_media_folder;
                let on = recording.in_media_folder;
                self.save_settings();
                UiRequest::Osd(format!(
                    "Store recorded audios into play folders: {}",
                    on_off(on)
                ))
            }
        }
    }

    /// Switches a setting of `settings.audio` and rebuilds the chain.
    fn toggle_audio(
        &mut self,
        engine: &Engine,
        name: &str,
        flag: impl Fn(&mut crate::audio::Audio) -> &mut bool,
    ) -> UiRequest {
        let slot = flag(&mut self.settings.audio);
        *slot = !*slot;
        let on = *slot;
        self.audio_chain_changed(engine);
        self.save_settings();
        UiRequest::Osd(format!("{name}: {}", on_off(on)))
    }

    /// The volume step, faster while a volume key is held ("Accelerate
    /// control speed"): repeats within 250 ms grow the step up to fivefold.
    pub(crate) fn volume_step_now(&mut self) -> f64 {
        let step = self.settings.volume_step;
        if !self.settings.audio.accelerate {
            return step;
        }
        let now = Instant::now();
        let repeated = self
            .session
            .last_volume_key
            .is_some_and(|last| now.saturating_duration_since(last).as_millis() < 250);
        self.session.volume_acceleration = if repeated {
            (self.session.volume_acceleration * 1.25).min(5.0)
        } else {
            1.0
        };
        self.session.last_volume_key = Some(now);
        step * self.session.volume_acceleration
    }

    /// Where recordings go: next to the media when asked and possible,
    /// else the recording folder.
    #[must_use]
    pub fn recording_folder(&self) -> PathBuf {
        let recording = &self.settings.audio.recording;
        if recording.in_media_folder
            && let Some(path) = self.model.loaded_path()
            && let Some(parent) = std::path::Path::new(path).parent()
            && parent.is_dir()
        {
            return parent.to_path_buf();
        }
        recording
            .folder
            .clone()
            .unwrap_or_else(|| dirs_music().join("RVP"))
    }

    /// A new recording's file: the media's name and the time.
    #[must_use]
    pub fn recording_path(&self) -> PathBuf {
        let stem = self
            .model
            .loaded_path()
            .and_then(|path| std::path::Path::new(path).file_stem())
            .map_or_else(
                || "RVP".to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            );
        let seconds = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        // Positions are far below 2^63 seconds.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole = seconds.max(0.0) as u64;
        let mut path = self.recording_folder().join(format!(
            "{stem} {:02}.{:02}.{:02}.wav",
            whole / 3600,
            whole / 60 % 60,
            whole % 60
        ));
        let mut n = 2;
        while path.exists() {
            path.set_file_name(format!(
                "{stem} {:02}.{:02}.{:02} ({n}).wav",
                whole / 3600,
                whole / 60 % 60,
                whole % 60
            ));
            n += 1;
        }
        path
    }

    /// A recording stopped, or could not start (the window reports it).
    pub fn recording_stopped(&mut self) {
        self.session.recording = None;
    }

    /// A recording started into `path`.
    pub fn recording_started(&mut self, path: PathBuf) {
        self.session.recording = Some(path);
    }

    /// Sets the folder audio recordings go to.
    pub fn set_record_folder(&mut self, folder: &std::path::Path) {
        self.settings.audio.recording.folder = Some(folder.to_path_buf());
        self.save_settings();
    }

    /// Sets the audio sync from the Adjust Audio Sync dialog (seconds).
    pub fn set_audio_delay(&mut self, engine: &Engine, seconds: f64) -> Option<UiRequest> {
        let seconds = seconds.clamp(-10.0, 10.0);
        engine.set_double("audio-delay", seconds).ok()?;
        self.audio_delay_changed(seconds);
        Some(UiRequest::Osd(format!(
            "Audio sync: {}",
            crate::commands::signed_seconds(seconds)
        )))
    }

    /// An audio sync change: "always keep" stores it for the next start.
    pub(crate) fn audio_delay_changed(&mut self, seconds: f64) {
        if self.settings.audio.sync_keep == SyncKeep::Always {
            self.settings.audio.kept_delay = seconds;
            self.save_settings();
        }
    }

    /// The audio part of `App::apply_settings`: mpv's options and the
    /// chain.
    pub(crate) fn apply_audio_settings(
        &mut self,
        engine: &Engine,
        old: &Settings,
        notes: &mut Vec<String>,
    ) {
        let (new, previous) = (&self.settings.audio, &old.audio);
        let mut set = |name: &str, value: &str| {
            if let Err(error) = engine.set_text(name, value) {
                notes.push(format!("cannot set {name}: {error}"));
            }
        };
        if new.exclusive != previous.exclusive {
            set("audio-exclusive", if new.exclusive { "yes" } else { "no" });
        }
        let count = self
            .audio_source
            .channels
            .len()
            .try_into()
            .ok()
            .filter(|count: &u32| *count > 0);
        if new.spdif(count) != previous.spdif(count) {
            set("audio-spdif", &new.spdif(count));
        }
        // mpv takes no value for its own choice back.
        let format_waits =
            new.sample_format != previous.sample_format && new.sample_format == SampleFormat::Same;
        if new.sample_format != previous.sample_format {
            if !format_waits {
                set("audio-format", new.sample_format.mpv());
            }
            set(
                "audio-swresample-o",
                if new.sample_format == SampleFormat::S16Dither {
                    "dither_method=triangular"
                } else {
                    ""
                },
            );
        }
        if new.sample_rate != previous.sample_rate {
            set("audio-samplerate", &new.sample_rate.to_string());
        }
        if new.resample_quality != previous.resample_quality {
            let (size, shift) = new.resample_quality.mpv();
            set("audio-resample-filter-size", &size.to_string());
            set("audio-resample-phase-shift", &shift.to_string());
        }
        if new.stretch != previous.stretch {
            set(
                "audio-pitch-correction",
                if new.stretch == Stretch::Pitch {
                    "no"
                } else {
                    "yes"
                },
            );
        }
        if new.languages != previous.languages {
            set("alang", new.languages.trim());
        }
        let second_device = new.second_device != previous.second_device;
        let second_options = new.stretch != previous.stretch;
        if format_waits {
            notes.push("The output format goes back to the device's at the next start".to_owned());
        }
        let chain = new.mixer != previous.mixer
            || new.lfe_redirect != previous.lfe_redirect
            || new.virtualizer != previous.virtualizer
            || new.channel_volumes != previous.channel_volumes
            || new.preamp != previous.preamp
            || new.crystality != previous.crystality
            || new.noise_reduction != previous.noise_reduction
            || new.freeverb != previous.freeverb
            || new.crossfeed != previous.crossfeed
            || (new.voice_removal, new.voice_emphasis, new.swap_channels)
                != (
                    previous.voice_removal,
                    previous.voice_emphasis,
                    previous.swap_channels,
                )
            || new.stretch != previous.stretch
            || new.output_mode != previous.output_mode
            || self.settings.equalizer.superequalizer != old.equalizer.superequalizer
            || self.settings.normalizer.video_only != old.normalizer.video_only;
        if second_device {
            self.second_device_changed(engine);
        } else if second_options {
            self.update_second_options();
        }
        if chain {
            self.audio_chain_changed(engine);
        }
    }
}

impl App {
    /// Reads mpv's audio devices (`audio-device-list`) for the menus.
    pub fn read_audio_devices(&mut self, engine: &Engine) {
        if let Some(serde_json::Value::Array(list)) = engine.get_json("audio-device-list") {
            self.audio_devices = list
                .iter()
                .filter_map(|device| {
                    Some(crate::menu::Device {
                        name: device.get("name")?.as_str()?.to_owned(),
                        description: device
                            .get("description")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    })
                })
                .collect();
        }
    }

    /// Carries out a choice of the Audio menu's output lists.
    pub fn audio_choice(
        &mut self,
        engine: &Engine,
        choice: crate::menu::AudioChoice,
    ) -> Option<UiRequest> {
        use crate::menu::AudioChoice;
        let device = |index: usize| {
            index
                .checked_sub(1)
                .and_then(|index| self.audio_devices.get(index))
                .map(|device| (device.name.clone(), device.description.clone()))
        };
        let text = match choice {
            AudioChoice::Device(index) => {
                let (name, description) =
                    device(index).unwrap_or_else(|| ("auto".to_owned(), "Auto select".to_owned()));
                self.settings.audio_device.clone_from(&name);
                drop(engine.set_text("audio-device", &name));
                format!("Audio Renderer: {description}")
            }
            AudioChoice::PassthroughDevice(index) => {
                let (name, description) = device(index)
                    .unwrap_or_else(|| (String::new(), "Default Audio Device".to_owned()));
                self.settings.audio.passthrough_device = name;
                format!("Pass-through Renderer: {description}")
            }
            AudioChoice::SecondDevice(index) => {
                let (name, description) =
                    device(index).unwrap_or_else(|| (String::new(), "Disable".to_owned()));
                self.settings.audio.second_device = name;
                format!("2nd Audio Renderer: {description}")
            }
            AudioChoice::Format(index) => {
                let (format, label) = crate::menu::FORMATS.get(index)?;
                self.settings.audio.sample_format = *format;
                if *format != SampleFormat::Same {
                    drop(engine.set_text("audio-format", format.mpv()));
                }
                drop(engine.set_text(
                    "audio-swresample-o",
                    if *format == SampleFormat::S16Dither {
                        "dither_method=triangular"
                    } else {
                        ""
                    },
                ));
                format!("Output Format: {label}")
            }
            AudioChoice::Rate(index) => {
                let rate = *crate::audio::SAMPLE_RATES.get(index)?;
                self.settings.audio.sample_rate = rate;
                drop(engine.set_text("audio-samplerate", &rate.to_string()));
                if rate == 0 {
                    "Sample rate: same as input".to_owned()
                } else {
                    format!("Sample rate: {rate}Hz")
                }
            }
            AudioChoice::Resample(index) => {
                let (quality, label) = crate::menu::QUALITIES.get(index)?;
                self.settings.audio.resample_quality = *quality;
                let (size, shift) = quality.mpv();
                drop(engine.set_text("audio-resample-filter-size", &size.to_string()));
                drop(engine.set_text("audio-resample-phase-shift", &shift.to_string()));
                format!("Resample: {label}")
            }
            AudioChoice::Mode(index) => {
                let (mode, label) = crate::menu::MODES.get(index)?;
                self.settings.audio.output_mode = *mode;
                self.output_mode_changed(engine);
                format!("Output Mode: {label}")
            }
            AudioChoice::PassthroughAny => {
                let audio = &mut self.settings.audio;
                audio.passthrough_min_channels = if audio.passthrough_min_channels.is_some() {
                    None
                } else {
                    Some(3)
                };
                let any = audio.passthrough_min_channels.is_none();
                self.output_mode_changed(engine);
                format!(
                    "Pass-through: {}",
                    if any {
                        "with any source"
                    } else {
                        "with 3 channels or more"
                    }
                )
            }
        };
        if matches!(choice, AudioChoice::SecondDevice(_)) {
            self.second_device_changed(engine);
        }
        self.save_settings();
        Some(UiRequest::Osd(text))
    }

    /// Starts, changes or stops the second audio device's core
    /// (`crate::audio::second`) after `audio.second-device` changed.
    pub(crate) fn second_device_changed(&mut self, engine: &Engine) {
        let wanted = self.settings.audio.second_device.clone();
        let current = self.second.as_ref().map(Second::device);
        if current == Some(wanted.as_str()) || (current.is_none() && wanted.is_empty()) {
            return;
        }
        self.stop_second();
        if wanted.is_empty() {
            return;
        }
        match Second::start(&wanted, &self.second_options()) {
            Ok(second) => {
                if let Some(control) = &self.normalizer {
                    control.follow(Some(second.engine()));
                }
                self.second = Some(second);
                self.apply_audio_chain(engine);
                self.follow_second(engine, &[], Instant::now());
            }
            Err(error) => eprintln!("rvp: cannot start the second audio device: {error}"),
        }
    }

    /// Stops the second device's core; the normalizer's thread lets go of
    /// it first (`normalizer::Control::follow`).
    pub(crate) fn stop_second(&mut self) {
        if let Some(control) = &self.normalizer {
            control.follow(None);
        }
        self.second = None;
    }

    /// The options of the main core's the second core shares: the
    /// speakers and the speed's pitch correction.
    fn second_options(&self) -> Vec<(String, String)> {
        let pitch = if self.settings.audio.stretch == Stretch::Pitch {
            "no"
        } else {
            "yes"
        };
        let mut options = vec![
            ("audio-channels".to_owned(), self.audio_channels()),
            ("audio-pitch-correction".to_owned(), pitch.to_owned()),
        ];
        options.extend(self.second_mpv_options.iter().cloned());
        options
    }

    /// Gives a running second core the main core's shared options again.
    pub(crate) fn update_second_options(&self) {
        if let Some(second) = &self.second {
            for (name, value) in self.second_options() {
                drop(second.engine().set_text(&name, &value));
            }
        }
    }

    /// The second core follows the main core; `events` are what the main
    /// core's last drain brought.
    pub(crate) fn follow_second(&mut self, engine: &Engine, events: &[EngineEvent], now: Instant) {
        let delay = self.settings.audio.second_delay / 1000.0;
        if let Some(second) = self.second.as_mut() {
            second.follow(engine, &self.model, events, delay, now);
        }
    }

    /// The shell's tick: the second core follows while the main core sends
    /// no events (paused, or between files).
    pub fn tick_second(&mut self, engine: &Engine, now: Instant) {
        self.follow_second(engine, &[], now);
    }

    /// The second device's core, while one plays.
    #[must_use]
    pub fn second(&self) -> Option<&Second> {
        self.second.as_ref()
    }

    /// The output mode changed: pass-through for the file that plays, and
    /// the AC3 encoder in the chain.
    fn output_mode_changed(&mut self, engine: &Engine) {
        let count = u32::try_from(self.audio_source.channels.len())
            .ok()
            .filter(|count| *count > 0);
        drop(engine.set_text("audio-spdif", &self.settings.audio.spdif(count)));
        self.audio_chain_changed(engine);
    }
}

/// The wave volume (mpv's `ao-volume`) by `delta` percent.
pub(crate) fn change_wave(engine: &Engine, delta: f64) -> Option<UiRequest> {
    let current = engine.get_double("ao-volume")?;
    let volume = (current + delta).clamp(0.0, 100.0);
    engine
        .set_double("ao-volume", volume)
        .ok()
        .map(|()| UiRequest::Osd(format!("Wave volume: {volume:.0} %")))
}

/// The user's music folder.
fn dirs_music() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map_or_else(std::env::temp_dir, PathBuf::from);
    home.join("Music")
}
