//! PotPlayer's audio processing beyond the volume and the normalizer (M10):
//! the output, the volume control, the effects, the sync, the speed's time
//! stretching and recording, as PotPlayer's Audio menu and its Audio
//! Preferences pages have them (`docs/r1/preferences.tsv`).
//!
//! * [`chain`] builds RVP's part of mpv's audio filters, in one order.
//! * [`dsp`] computes the impulse responses some filters convolve with
//!   (Freeverb's reverb, the virtual speakers' head model).
//! * [`commands`] carries out the Audio menu's commands.
//! * [`second`] plays the same sound on a second device.
//!
//! The settings here are `settings.json`'s `audio` object; the speakers,
//! the equalizer and the normalizer keep their own places.

pub mod chain;
pub mod commands;
pub mod dsp;
pub mod second;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What the audio output receives (PotPlayer's Output Mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    /// Decoded samples ("PCM (Default)"; PotPlayer's "LPCM" is the same
    /// to mpv).
    #[default]
    Pcm,
    /// Compressed streams go out as they are (S/PDIF or HDMI
    /// pass-through, `audio-spdif`).
    Passthrough,
    /// Everything is encoded to AC3 and passed through ("Pass-through
    /// after AC3 re-encoding", mpv's `lavcac3enc`).
    Ac3Encode,
}

/// The sample format sent to the output (Output Format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SampleFormat {
    /// What mpv and the device agree on. PotPlayer's default is 16-bit.
    #[default]
    Same,
    S16,
    /// 16-bit with triangular dither.
    S16Dither,
    /// 24 bits, carried in 32 (mpv has no packed 24-bit format).
    S24,
    S32,
    Float,
}

impl SampleFormat {
    /// mpv's `audio-format` (`no`, mpv's own choice, is never set: mpv
    /// refuses it).
    #[must_use]
    pub fn mpv(self) -> &'static str {
        match self {
            SampleFormat::Same => "no",
            SampleFormat::S16 | SampleFormat::S16Dither => "s16",
            SampleFormat::S24 | SampleFormat::S32 => "s32",
            SampleFormat::Float => "float",
        }
    }
}

/// The resampler's quality (Output Format/Resample).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResampleQuality {
    High,
    #[default]
    Normal,
    Low,
}

impl ResampleQuality {
    /// mpv's `audio-resample-filter-size` and `audio-resample-phase-shift`
    /// (normal is mpv's default).
    #[must_use]
    pub fn mpv(self) -> (u32, u32) {
        match self {
            ResampleQuality::High => (32, 12),
            ResampleQuality::Normal => (16, 10),
            ResampleQuality::Low => (8, 8),
        }
    }
}

/// When the output rate applies (the Audio page's "Resample"): always, or
/// only to sources below or above a rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResampleWhen {
    #[default]
    Always,
    Below,
    Above,
}

/// How a file's audio track is chosen ("Selection method").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Selection {
    /// "Prefer specified languages": the languages, then the preferred
    /// stream.
    #[default]
    Languages,
    /// "Prefer specified stream".
    Stream,
    /// "Prefer highest quality ones": the most channels, then the highest
    /// rate, then the highest bitrate.
    Quality,
    /// "Use default selection": the file's own (mpv's choice without the
    /// languages).
    Default,
}

/// The bitrates of "Pass-through after AC3 re-encoding", in kbps
/// (PotPlayer's list; 640 is its default and mpv's).
pub const AC3_BITRATES: [u32; 15] = [
    64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 576, 640,
];

/// How a source is mixed to the speakers (the Audio page's Mixer Settings
/// and Mixer Level). The levels are in dB; the defaults are PotPlayer's mix
/// as the audio audit measured it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Mixer {
    /// Scale the mix down so that it cannot clip (PotPlayer: off).
    pub normalize: bool,
    /// A stereo source also feeds the centre speaker.
    pub expand_center: bool,
    /// A stereo source also feeds the surround speakers.
    pub expand_surround: bool,
    pub front: f64,
    pub center: f64,
    pub surround: f64,
    pub lfe: f64,
}

impl Default for Mixer {
    fn default() -> Self {
        Self {
            normalize: false,
            expand_center: false,
            expand_surround: false,
            front: 0.0,
            center: -3.0103,
            surround: 0.0,
            lfe: -3.0103,
        }
    }
}

/// LFE Redirection: the speakers' low frequencies to the LFE channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LfeRedirect {
    pub enabled: bool,
    /// The other speakers no longer play what goes to the LFE.
    pub remove_low: bool,
    /// The crossover in Hz.
    pub frequency: f64,
    /// The redirected part's gain in dB.
    pub gain: f64,
}

impl Default for LfeRedirect {
    fn default() -> Self {
        Self {
            enabled: false,
            remove_low: false,
            frequency: 80.0,
            gain: 0.0,
        }
    }
}

/// Headphone & Dolby Decoder Settings: the virtual speakers' distance (0
/// to 100: how much of a small room is heard) and the decoded surround
/// channels' delay in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Virtualizer {
    pub distance: f64,
    pub delay: f64,
}

impl Default for Virtualizer {
    fn default() -> Self {
        Self {
            distance: 30.0,
            delay: 20.0,
        }
    }
}

/// What the volume keys and slider change (Playback Volume).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VolumeMethod {
    /// RVP's own volume ("Default volume control").
    #[default]
    Player,
    /// RVP's stream in the system's mixer ("System Wave volume control").
    Wave,
    /// The output device's volume ("System Master volume control").
    Master,
}

/// "Enable individual volume control": a gain per output channel and one
/// for them all, in dB.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ChannelVolumes {
    pub enabled: bool,
    pub master: f64,
    pub left: f64,
    pub right: f64,
    pub center: f64,
    pub back_left: f64,
    pub back_right: f64,
    pub side_left: f64,
    pub side_right: f64,
    pub lfe: f64,
}

impl Default for ChannelVolumes {
    fn default() -> Self {
        Self {
            enabled: false,
            master: 0.0,
            left: 0.0,
            right: 0.0,
            center: 0.0,
            back_left: 0.0,
            back_right: 0.0,
            side_left: 0.0,
            side_right: 0.0,
            lfe: 0.0,
        }
    }
}

impl ChannelVolumes {
    /// The gain in dB of an output channel (FFmpeg's name), the master's
    /// included.
    #[must_use]
    pub fn gain(&self, channel: &str) -> f64 {
        self.master
            + match channel {
                "FL" | "FLC" => self.left,
                "FR" | "FRC" => self.right,
                "FC" => self.center,
                "BL" => self.back_left,
                "BR" => self.back_right,
                "SL" => self.side_left,
                "SR" => self.side_right,
                "LFE" => self.lfe,
                // The back centre between the back pair.
                "BC" => f64::midpoint(self.back_left, self.back_right),
                _ => 0.0,
            }
    }
}

/// "Pre-amplify AC3/DTS Volume to proper level": a gain for those sources,
/// which are mastered quieter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Preamp {
    pub enabled: bool,
    /// dB.
    pub gain: f64,
}

impl Default for Preamp {
    fn default() -> Self {
        Self {
            enabled: false,
            gain: 6.0,
        }
    }
}

/// Crystality (PotPlayer's "Crystallization"): its six levels, 0 to 100,
/// with the defaults of the Crystality plugin PotPlayer names.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Crystality {
    pub enabled: bool,
    pub bandwidth_extender: f64,
    /// The extender's high-pass filter.
    pub extender_hpf: f64,
    pub harmonic_booster: f64,
    pub echo_volume: f64,
    pub echo_feedback: f64,
    pub extra_stereo: f64,
}

impl Default for Crystality {
    fn default() -> Self {
        Self {
            enabled: false,
            bandwidth_extender: 28.0,
            extender_hpf: 3.0,
            harmonic_booster: 43.0,
            echo_volume: 11.0,
            echo_feedback: 30.0,
            extra_stereo: 11.0,
        }
    }
}

/// Noise Reduction: on or off. PotPlayer's threshold (1 to 5000, 100 by
/// default) is not here: goal L's host session measured its filter at the
/// default only (`crate::audio::chain`'s `denoise`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NoiseReduction {
    pub enabled: bool,
}

/// Freeverb (Jezar at Dreampoint's public-domain reverb): its five
/// parameters, 0 to 1; the defaults are PotPlayer's (R1's capture of its
/// Preferences).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Freeverb {
    pub enabled: bool,
    pub room_size: f64,
    pub damping: f64,
    pub wet: f64,
    pub dry: f64,
    pub width: f64,
}

impl Default for Freeverb {
    fn default() -> Self {
        Self {
            enabled: false,
            room_size: 0.5,
            damping: 0.25,
            wet: 0.333,
            dry: 0.75,
            width: 1.0,
        }
    }
}

/// Crossfeed for headphones, Bauer's stereophonic-to-binaural (bs2b):
/// each side goes to the other ear below a cut frequency (Hz) at a feed
/// level (dB). PotPlayer's defaults, "Remove low frequencies" 700 Hz and
/// "Gain" 6 dB, are bs2b's "Chu Moy" setting.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Crossfeed {
    pub enabled: bool,
    pub cut: f64,
    pub feed: f64,
}

impl Default for Crossfeed {
    fn default() -> Self {
        Self {
            enabled: false,
            cut: 700.0,
            feed: 6.0,
        }
    }
}

/// Which channels of a stereo track play ("Select Audio Stream": Both,
/// Left channel, Right channel). For this session, as PotPlayer keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum StereoChannel {
    #[default]
    Both,
    Left,
    Right,
}

/// What happens to an audio sync change at the next file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncKeep {
    /// The next file starts at the default sync.
    Never,
    /// It stays until RVP exits.
    #[default]
    Runtime,
    /// It stays, also over restarts.
    Always,
}

/// How speed changes keep the pitch (Playback > Speed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stretch {
    /// "Use high quality audio time stretching": `scaletempo2`.
    #[default]
    HighQuality,
    /// "Use default quality audio time stretching": `scaletempo`.
    DefaultQuality,
    /// "Use audio pitch shifting": the pitch follows the speed.
    Pitch,
}

/// The file format of audio recordings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordFormat {
    #[default]
    Wav,
}

/// Audio Recording's folder options.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Recording {
    /// Where recordings go; unset: `Music/RVP` in the home folder.
    pub folder: Option<PathBuf>,
    /// "Store recorded audios into play folders (if available)".
    pub in_media_folder: bool,
    pub format: RecordFormat,
}

/// RVP's audio settings beyond the speakers, the equalizer and the
/// normalizer (`settings.json`'s `audio`).
// Each flag is an independent option as the settings file stores it.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Audio {
    /// WASAPI's exclusive mode (mpv `audio-exclusive`).
    pub exclusive: bool,
    pub output_mode: OutputMode,
    /// The codecs passed through (mpv `audio-spdif`).
    pub passthrough_codecs: String,
    /// Pass through only sources with at least this many channels ("Use
    /// pass-through output when at least # channels exist").
    pub passthrough_min_channels: Option<u32>,
    /// The device for passed-through sources; empty: the audio device.
    pub passthrough_device: String,
    pub sample_format: SampleFormat,
    /// Hz; 0: as the source ("Same as input").
    pub sample_rate: u32,
    pub resample_quality: ResampleQuality,
    /// The sample rate applies always, or to sources below or above
    /// `resample_limit` Hz.
    pub resample_when: ResampleWhen,
    pub resample_limit: u32,
    /// "Pass-through after AC3 re-encoding"'s bitrate, kbps.
    pub ac3_bitrate: u32,
    pub mixer: Mixer,
    pub lfe_redirect: LfeRedirect,
    pub virtualizer: Virtualizer,
    /// A second device that plays the same sound; empty: none.
    pub second_device: String,
    /// The second device's sync against the first, in milliseconds.
    pub second_delay: f64,
    pub volume_method: VolumeMethod,
    /// With the wave or master method, the other one goes to its maximum
    /// ("Adjust wave volume to max").
    pub wave_to_max: bool,
    /// Holding a volume key speeds the change up.
    pub accelerate: bool,
    /// The volume every start begins with; unset: the last one.
    pub startup_volume: Option<f64>,
    pub channel_volumes: ChannelVolumes,
    pub preamp: Preamp,
    pub crystality: Crystality,
    pub noise_reduction: NoiseReduction,
    pub freeverb: Freeverb,
    pub crossfeed: Crossfeed,
    pub voice_removal: bool,
    pub voice_emphasis: bool,
    pub swap_channels: bool,
    pub sync_keep: SyncKeep,
    /// The audio sync every file starts with, in milliseconds.
    pub default_delay: f64,
    /// A sync kept "always" (`SyncKeep::Always`), in seconds.
    pub kept_delay: f64,
    pub stretch: Stretch,
    pub recording: Recording,
    /// Preferred audio languages, comma separated (mpv `alang`).
    pub languages: String,
    /// Subtitles are turned off when the audio is in one of these
    /// languages ("Turn off subtitles with").
    pub subtitles_off: String,
    pub selection: Selection,
    /// The audio track every file starts with when no language matches
    /// ("Preferred stream", 1 the first); 0: mpv's choice.
    pub preferred_stream: u32,
    /// "Remember audio track selections": a file plays again with the
    /// track chosen in it.
    pub remember_tracks: bool,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            exclusive: false,
            output_mode: OutputMode::Pcm,
            passthrough_codecs: "ac3,eac3,dts,dts-hd,truehd".to_owned(),
            passthrough_min_channels: None,
            passthrough_device: String::new(),
            sample_format: SampleFormat::Same,
            sample_rate: 0,
            resample_quality: ResampleQuality::Normal,
            resample_when: ResampleWhen::Always,
            resample_limit: 48_000,
            ac3_bitrate: 640,
            mixer: Mixer::default(),
            lfe_redirect: LfeRedirect::default(),
            virtualizer: Virtualizer::default(),
            second_device: String::new(),
            second_delay: 0.0,
            volume_method: VolumeMethod::Player,
            wave_to_max: false,
            accelerate: false,
            startup_volume: None,
            channel_volumes: ChannelVolumes::default(),
            preamp: Preamp::default(),
            crystality: Crystality::default(),
            noise_reduction: NoiseReduction::default(),
            freeverb: Freeverb::default(),
            crossfeed: Crossfeed::default(),
            voice_removal: false,
            voice_emphasis: false,
            swap_channels: false,
            sync_keep: SyncKeep::Runtime,
            default_delay: 0.0,
            kept_delay: 0.0,
            stretch: Stretch::HighQuality,
            recording: Recording::default(),
            languages: String::new(),
            subtitles_off: String::new(),
            selection: Selection::Languages,
            preferred_stream: 0,
            remember_tracks: false,
        }
    }
}

/// Sample rates the menu offers (PotPlayer's list), 0 being the source's.
pub const SAMPLE_RATES: [u32; 12] = [
    0, 192_000, 96_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000,
];

/// Codecs mpv can pass through (`audio-spdif`).
pub const PASSTHROUGH_CODECS: [&str; 5] = ["ac3", "eac3", "dts", "dts-hd", "truehd"];

fn clamp(value: &mut f64, low: f64, high: f64, name: &str, notes: &mut Vec<String>) {
    let repaired = if value.is_finite() {
        value.clamp(low, high)
    } else {
        low
    };
    if (repaired - *value).abs() > f64::EPSILON || !value.is_finite() {
        notes.push(format!(
            "audio {name} must be {low} to {high}: {repaired} used"
        ));
        *value = repaired;
    }
}

impl Audio {
    /// Brings every value into its range; returns a note per repair.
    pub fn validate(&mut self, notes: &mut Vec<String>) {
        self.validate_output(notes);
        self.validate_levels(notes);
        self.validate_effects(notes);
    }

    fn validate_output(&mut self, notes: &mut Vec<String>) {
        let codecs: Vec<&str> = self
            .passthrough_codecs
            .split(',')
            .map(str::trim)
            .filter(|codec| !codec.is_empty())
            .collect();
        if codecs
            .iter()
            .any(|codec| !PASSTHROUGH_CODECS.contains(codec))
        {
            notes.push(format!(
                "audio passthrough-codecs {:?} must be some of {}: all used",
                self.passthrough_codecs,
                PASSTHROUGH_CODECS.join(",")
            ));
            self.passthrough_codecs = PASSTHROUGH_CODECS.join(",");
        }
        if let Some(channels) = self.passthrough_min_channels
            && !(1..=8).contains(&channels)
        {
            notes.push(format!(
                "audio passthrough-min-channels {channels} must be 1 to 8: off"
            ));
            self.passthrough_min_channels = None;
        }
        for device in [&mut self.passthrough_device, &mut self.second_device] {
            if device.chars().any(char::is_control) || device.len() > 300 {
                notes.push(format!(
                    "audio device {device:?} is not a device name: cleared"
                ));
                device.clear();
            }
        }
        if !SAMPLE_RATES.contains(&self.sample_rate) {
            notes.push(format!(
                "audio sample-rate {} is not one of PotPlayer's: the source's used",
                self.sample_rate
            ));
            self.sample_rate = 0;
        }
        if !(8_000..=192_000).contains(&self.resample_limit) {
            notes.push(format!(
                "audio resample-limit {} must be 8000 to 192000 Hz: 48000 used",
                self.resample_limit
            ));
            self.resample_limit = 48_000;
        }
        if !AC3_BITRATES.contains(&self.ac3_bitrate) {
            notes.push(format!(
                "audio ac3-bitrate {} is not one of PotPlayer's: 640 used",
                self.ac3_bitrate
            ));
            self.ac3_bitrate = 640;
        }
        for (languages, name) in [
            (&mut self.languages, "languages"),
            (&mut self.subtitles_off, "subtitles-off"),
        ] {
            if languages
                .chars()
                .any(|c| !(c.is_ascii_alphanumeric() || ", -_".contains(c)))
            {
                notes.push(format!(
                    "audio {name} {languages:?} must be language codes separated by commas: cleared"
                ));
                languages.clear();
            }
        }
        if self.preferred_stream > 99 {
            notes.push(format!(
                "audio preferred-stream {} must be 0 to 99: 0 used",
                self.preferred_stream
            ));
            self.preferred_stream = 0;
        }
    }

    fn validate_levels(&mut self, notes: &mut Vec<String>) {
        let mixer = &mut self.mixer;
        for (value, name) in [
            (&mut mixer.front, "mixer front"),
            (&mut mixer.center, "mixer center"),
            (&mut mixer.surround, "mixer surround"),
            (&mut mixer.lfe, "mixer lfe"),
        ] {
            clamp(value, -30.0, 12.0, name, notes);
        }
        clamp(
            &mut self.lfe_redirect.frequency,
            20.0,
            300.0,
            "lfe-redirect frequency",
            notes,
        );
        clamp(
            &mut self.lfe_redirect.gain,
            -20.0,
            20.0,
            "lfe-redirect gain",
            notes,
        );
        clamp(
            &mut self.virtualizer.distance,
            0.0,
            100.0,
            "virtualizer distance",
            notes,
        );
        clamp(
            &mut self.virtualizer.delay,
            0.0,
            100.0,
            "virtualizer delay",
            notes,
        );
        clamp(
            &mut self.second_delay,
            -2000.0,
            2000.0,
            "second-delay",
            notes,
        );
        if let Some(volume) = &mut self.startup_volume {
            clamp(volume, 0.0, 1000.0, "startup-volume", notes);
        }
        let gains = &mut self.channel_volumes;
        for (value, name) in [
            (&mut gains.master, "channel-volumes master"),
            (&mut gains.left, "channel-volumes left"),
            (&mut gains.right, "channel-volumes right"),
            (&mut gains.center, "channel-volumes center"),
            (&mut gains.back_left, "channel-volumes back-left"),
            (&mut gains.back_right, "channel-volumes back-right"),
            (&mut gains.side_left, "channel-volumes side-left"),
            (&mut gains.side_right, "channel-volumes side-right"),
            (&mut gains.lfe, "channel-volumes lfe"),
        ] {
            clamp(value, -40.0, 12.0, name, notes);
        }
        clamp(&mut self.preamp.gain, 0.0, 20.0, "preamp gain", notes);
        clamp(
            &mut self.default_delay,
            -10_000.0,
            10_000.0,
            "default-delay",
            notes,
        );
        clamp(&mut self.kept_delay, -10.0, 10.0, "kept-delay", notes);
    }

    fn validate_effects(&mut self, notes: &mut Vec<String>) {
        let crystality = &mut self.crystality;
        for (value, name) in [
            (
                &mut crystality.bandwidth_extender,
                "crystality bandwidth-extender",
            ),
            (
                &mut crystality.harmonic_booster,
                "crystality harmonic-booster",
            ),
            (&mut crystality.echo_volume, "crystality echo-volume"),
            (&mut crystality.echo_feedback, "crystality echo-feedback"),
            (&mut crystality.extra_stereo, "crystality extra-stereo"),
        ] {
            clamp(value, 0.0, 100.0, name, notes);
        }
        clamp(
            &mut crystality.extender_hpf,
            1.0,
            3.0,
            "crystality extender-hpf",
            notes,
        );
        let reverb = &mut self.freeverb;
        for (value, name) in [
            (&mut reverb.room_size, "freeverb room-size"),
            (&mut reverb.damping, "freeverb damping"),
            (&mut reverb.wet, "freeverb wet"),
            (&mut reverb.dry, "freeverb dry"),
            (&mut reverb.width, "freeverb width"),
        ] {
            clamp(value, 0.0, 1.0, name, notes);
        }
        clamp(
            &mut self.crossfeed.cut,
            300.0,
            2000.0,
            "crossfeed cut",
            notes,
        );
        clamp(&mut self.crossfeed.feed, 1.0, 15.0, "crossfeed feed", notes);
    }

    /// mpv's `audio-samplerate` for a source of `source_rate` Hz (unknown:
    /// the setting as it is): the chosen rate when it applies, else 0 (the
    /// source's own).
    #[must_use]
    pub fn output_rate(&self, source_rate: Option<u32>) -> u32 {
        let applies = match (self.resample_when, source_rate) {
            (ResampleWhen::Always, _) | (_, None) => true,
            (ResampleWhen::Below, Some(rate)) => rate < self.resample_limit,
            (ResampleWhen::Above, Some(rate)) => rate > self.resample_limit,
        };
        if applies { self.sample_rate } else { 0 }
    }

    /// mpv's `audio-spdif` for a source with `channels` channels.
    #[must_use]
    pub fn spdif(&self, channels: Option<u32>) -> String {
        match self.output_mode {
            OutputMode::Pcm => String::new(),
            OutputMode::Ac3Encode => "ac3".to_owned(),
            OutputMode::Passthrough => {
                let enough = match (self.passthrough_min_channels, channels) {
                    (Some(least), Some(channels)) => channels >= least,
                    _ => true,
                };
                if enough {
                    self.passthrough_codecs.clone()
                } else {
                    String::new()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_output_rate_follows_the_resample_rule() {
        let mut audio = Audio {
            sample_rate: 48_000,
            ..Audio::default()
        };
        assert_eq!(audio.output_rate(Some(44_100)), 48_000);
        audio.resample_when = ResampleWhen::Below;
        assert_eq!(audio.output_rate(Some(44_100)), 48_000);
        assert_eq!(audio.output_rate(Some(96_000)), 0);
        audio.resample_when = ResampleWhen::Above;
        assert_eq!(audio.output_rate(Some(44_100)), 0);
        assert_eq!(audio.output_rate(Some(96_000)), 48_000);
        assert_eq!(audio.output_rate(None), 48_000);
        audio.sample_rate = 0;
        assert_eq!(audio.output_rate(Some(96_000)), 0);
    }

    #[test]
    fn the_defaults_are_valid_and_out_of_range_values_are_repaired() {
        let mut notes = Vec::new();
        let mut audio = Audio::default();
        audio.validate(&mut notes);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(audio, Audio::default());

        let mut audio = Audio {
            passthrough_codecs: "ac3,mp3".to_owned(),
            passthrough_min_channels: Some(12),
            sample_rate: 12_345,
            resample_limit: 5,
            ac3_bitrate: 700,
            languages: "en;rm".to_owned(),
            ..Audio::default()
        };
        audio.freeverb.room_size = 3.0;
        audio.crossfeed.cut = f64::NAN;
        audio.validate(&mut notes);
        assert_eq!(audio.passthrough_codecs, "ac3,eac3,dts,dts-hd,truehd");
        assert_eq!(audio.passthrough_min_channels, None);
        assert_eq!(audio.sample_rate, 0);
        assert!(audio.languages.is_empty());
        assert!((audio.freeverb.room_size - 1.0).abs() < 1e-12);
        assert!((audio.crossfeed.cut - 300.0).abs() < 1e-12);
        assert_eq!((audio.resample_limit, audio.ac3_bitrate), (48_000, 640));
        assert_eq!(notes.len(), 8, "{notes:?}");
    }

    #[test]
    fn pass_through_follows_the_mode_and_the_channels() {
        let mut audio = Audio::default();
        assert_eq!(audio.spdif(Some(6)), "");
        audio.output_mode = OutputMode::Passthrough;
        assert_eq!(audio.spdif(Some(2)), "ac3,eac3,dts,dts-hd,truehd");
        audio.passthrough_min_channels = Some(6);
        assert_eq!(audio.spdif(Some(2)), "");
        assert_eq!(audio.spdif(Some(6)), "ac3,eac3,dts,dts-hd,truehd");
        audio.output_mode = OutputMode::Ac3Encode;
        assert_eq!(audio.spdif(Some(2)), "ac3");
    }

    #[test]
    fn channel_volumes_add_the_master() {
        let gains = ChannelVolumes {
            master: -2.0,
            left: 1.0,
            lfe: 3.0,
            ..ChannelVolumes::default()
        };
        assert!((gains.gain("FL") + 1.0).abs() < 1e-12);
        assert!((gains.gain("LFE") - 1.0).abs() < 1e-12);
        assert!((gains.gain("FR") + 2.0).abs() < 1e-12);
    }
}
