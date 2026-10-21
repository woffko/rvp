//! The coverage of PotPlayer's Preferences (R2, M10-M14): every option of
//! R2's pages (`docs/r1/preferences.tsv`), the RVP setting that does what
//! it does (its place in `settings.json`), or why RVP has none.
//! `tests/coverage.rs` writes `docs/coverage/preferences.md` from it.
//!
//! The audio pages are M10's and say for each option why when RVP lacks
//! it; the other pages name the milestone that takes them on.

use std::collections::BTreeMap;
use std::fmt::Write as _;

const PREFERENCES_TSV: &str = include_str!("../../../../docs/r1/preferences.tsv");

/// One option of a Preferences page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageOption {
    pub page: String,
    pub dialog: u32,
    pub control: u32,
    /// `check`, `radio`, `combo`, `slider`, `edit`, `button`.
    pub kind: String,
    pub text: String,
    pub group: String,
}

/// What RVP does with an option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The RVP setting (its place in `settings.json`) or command.
    Setting(&'static str),
    Missing(&'static str),
}

use Status::{Missing, Setting};

const DAUM: &str = "Daum's and Kakao's broadcasting services, discontinued";
const NAVIGATOR: &str = "PotPlayer's file navigator and Open Box are not in RVP \
                         (docs/spikes/m11-studies.md)";
const SEAMLESS: &str = "seamless playback is not in RVP (docs/spikes/m11-studies.md): mpv \
                        preloads and joins files only inside its own playlist";
const SKIN_LIST: &str = "RVP's playlist is the skin's attached panel: it numbers its rows and \
                         takes its font and colours from the skin";
const CAPTURE_DEVICE: &str = "FFmpeg's dshow and gdigrab choose the format and size; RVP opens \
                              the device or screen as it is";
const WINAMP_DSP: &str = "mpv's audio chain runs FFmpeg's filters only: native Winamp DSP \
                          plugins cannot run in it";
const RENDERER: &str = "DirectShow's video renderers do not apply to libmpv: RVP renders with \
                        mpv's renderer, OpenGL in its window or Direct3D 11 in a window of \
                        its own (D4)";
const EXCLUSIVE: &str = "exclusive fullscreen needs a top-level swapchain window; mpv's Direct3D \
                         11 output is a child of RVP's window (D4)";
const DIRECTSHOW: &str = "DirectShow filters do not apply to libmpv: FFmpeg decodes and splits \
                          (the Decoders page chooses hardware decoding per codec)";
const COLOR_CONVERSION: &str = "PotPlayer's own colour conversion (DirectShow); mpv's renderer \
                                converts YCbCr to RGB on the GPU from the file's tags";
const AVISYNTH: &str = "PotPlayer's AviSynth processing needs a bridge RVP does not have \
                        (docs/spikes/m12-studies.md: avsw runs a 32-bit AviSynth in a second \
                        process, 8-bit video only); RVP opens .avs files, runs VapourSynth \
                        scripts and loads AviSynth plugins in them";
const MVC: &str = "FFmpeg decodes only H.264 MVC's base view (docs/spikes/m12-studies.md); 3D \
                   files side by side or top and bottom play through stereo3d";
const SUBTITLES_3D: &str = "mpv draws subtitles once over the finished picture \
                            (docs/spikes/m12-studies.md)";
const WMP: &str = "Windows Media Player's visualization plugins run in WMP's own COM host; RVP \
                   shows FFmpeg's visualizations (visual.mode)";
const DEFAULT_ASPECT: &str = "RVP's idle and audio windows keep the size the user gives them; \
                              with a video the window takes the video's shape (R1 finding 1)";

/// The options RVP has, and the reasons for single ones it lacks, by
/// dialog and control.
const LIBASS: &str = "mpv (libass) renders every subtitle";
const SAMI: &str = "FFmpeg reads a SAMI (.smi) file as one track and decides its line breaks and \
                    times; mpv has no options for them";
const SUBPICTURES: &str = "PotPlayer's subtitle textures and queue (DirectShow); mpv's renderer \
                           draws libass's subtitles with each frame";
const SHOWING_PERIOD: &str =
    "mpv shows each subtitle for its own time, with no shortest or longest";

const ITEMS: &[(u32, u32, Status)] = &[
    // Audio (356).
    (356, 3003, Setting("audio-device")),
    (356, 3016, Setting("audio.passthrough-device")),
    (356, 3017, Setting("audio.second-device")),
    (356, 3013, Setting("output-speakers")),
    (356, 3008, Setting("audio.sample-format")),
    (356, 3006, Setting("audio.sample-rate")),
    (356, 3012, Setting("audio.resample-quality")),
    (356, 3007, Setting("audio.resample-when")),
    (356, 3083, Setting("audio.resample-limit")),
    (356, 3005, Setting("audio.output-mode")),
    (356, 3010, Setting("audio.ac3-bitrate")),
    (356, 3061, Setting("audio.passthrough-min-channels")),
    (356, 3082, Setting("audio.passthrough-min-channels")),
    (
        356,
        3358,
        Missing(
            "PotPlayer's own decoders (DirectShow); RVP decodes with mpv's FFmpeg (the \
             Decoders page chooses hardware decoding per codec)",
        ),
    ),
    (356, 3089, Setting("audio.mixer.normalize")),
    (356, 3094, Setting("audio.mixer.expand-center")),
    (356, 3101, Setting("audio.mixer.expand-surround")),
    (356, 3103, Setting("audio.mixer.front")),
    (356, 3108, Setting("audio.mixer.center")),
    (356, 3112, Setting("audio.mixer.surround")),
    (356, 3362, Setting("audio.mixer.lfe")),
    (356, 3104, Setting("audio.virtualizer.distance")),
    (356, 3109, Setting("audio.virtualizer.delay")),
    (356, 3057, Setting("audio.lfe-redirect.enabled")),
    (356, 3066, Setting("audio.lfe-redirect.remove-low")),
    (356, 3069, Setting("audio.lfe-redirect.frequency")),
    (356, 3077, Setting("audio.lfe-redirect.gain")),
    // Volume Control (390).
    (390, 3008, Setting("audio.volume-method")),
    (390, 3047, Setting("audio.wave-to-max")),
    (390, 3048, Setting("audio.accelerate")),
    (390, 3273, Setting("volume-step")),
    (390, 3274, Setting("volume-max")),
    (390, 3049, Setting("audio.startup-volume")),
    (390, 3275, Setting("audio.startup-volume")),
    (390, 3034, Setting("audio.channel-volumes.enabled")),
    (390, 3079, Setting("audio.channel-volumes.master")),
    (390, 3088, Setting("audio.channel-volumes.master")),
    (390, 3084, Setting("audio.channel-volumes.left")),
    (390, 3095, Setting("audio.channel-volumes.left")),
    (390, 3098, Setting("audio.channel-volumes.right")),
    (390, 3105, Setting("audio.channel-volumes.right")),
    (390, 3108, Setting("audio.channel-volumes.center")),
    (390, 3116, Setting("audio.channel-volumes.center")),
    (390, 3362, Setting("audio.channel-volumes.back-left")),
    (390, 3126, Setting("audio.channel-volumes.back-left")),
    (390, 3130, Setting("audio.channel-volumes.back-right")),
    (390, 3135, Setting("audio.channel-volumes.back-right")),
    (390, 3140, Setting("audio.channel-volumes.side-left")),
    (390, 3143, Setting("audio.channel-volumes.side-left")),
    (390, 3146, Setting("audio.channel-volumes.side-right")),
    (390, 3147, Setting("audio.channel-volumes.side-right")),
    (390, 3153, Setting("audio.channel-volumes.lfe")),
    (390, 3158, Setting("audio.channel-volumes.lfe")),
    (390, 3373, Setting("audio.preamp.enabled")),
    (390, 3374, Setting("audio.preamp.gain")),
    (390, 3375, Setting("audio.preamp.gain")),
    // Equalizer (363).
    (363, 3066, Setting("equalizer.enabled")),
    (363, 3021, Setting("equalizer.preset")),
    (363, 3005, Setting("equalizer.preset")),
    (363, 3072, Setting("equalizer.superequalizer")),
    (363, 3043, Setting("equalizer.bands.0")),
    (363, 3045, Setting("equalizer.bands.1")),
    (363, 3053, Setting("equalizer.bands.2")),
    (363, 3058, Setting("equalizer.bands.3")),
    (363, 3062, Setting("equalizer.bands.4")),
    (363, 3093, Setting("equalizer.bands.5")),
    (363, 3098, Setting("equalizer.bands.6")),
    (363, 3103, Setting("equalizer.bands.7")),
    (363, 3108, Setting("equalizer.bands.8")),
    (363, 3112, Setting("equalizer.bands.9")),
    // Normalizer/Freeverb (387).
    (387, 3035, Missing(DAUM)),
    (387, 3034, Setting("normalizer.enabled")),
    (387, 3036, Setting("normalizer.video-only")),
    (
        387,
        3006,
        Missing(
            "RVP runs TomSteady, PotPlayer's recommended method (D13); the list's other two, \"32-bit simple arithmetic\" and \"32-bit complex arithmetic\" (goal L's host session), are PotPlayer's own and were not measured",
        ),
    ),
    (387, 3047, Setting("normalizer.boost")),
    (387, 3353, Setting("normalizer.level")),
    (387, 3377, Setting("normalizer.release")),
    (387, 3072, Setting("audio.freeverb.enabled")),
    (387, 3079, Setting("audio.freeverb.room-size")),
    (387, 3098, Setting("audio.freeverb.damping")),
    (387, 3103, Setting("audio.freeverb.wet")),
    (387, 3108, Setting("audio.freeverb.dry")),
    (387, 3112, Setting("audio.freeverb.width")),
    // Crystality/Noise Reduction (388).
    (388, 3040, Setting("audio.crystality.enabled")),
    (388, 3130, Setting("audio.crystality.bandwidth-extender")),
    (388, 3098, Setting("audio.crystality.extender-hpf")),
    (388, 3103, Setting("audio.crystality.harmonic-booster")),
    (388, 3108, Setting("audio.crystality.echo-volume")),
    (388, 3112, Setting("audio.crystality.echo-feedback")),
    (388, 3140, Setting("audio.crystality.extra-stereo")),
    (388, 3083, Setting("audio.noise-reduction.enabled")),
    (
        388,
        3362,
        Missing(
            "RVP's noise reduction is PotPlayer's filter as goal L's host session measured it at the default threshold (100); how the threshold changes it was not measured",
        ),
    ),
    (388, 3084, Setting("audio.crossfeed.enabled")),
    (388, 3069, Setting("audio.crossfeed.cut")),
    (388, 3077, Setting("audio.crossfeed.feed")),
    // Audio's Language/Sync/Other (441).
    (441, 3004, Setting("audio.selection")),
    (441, 3069, Setting("audio.languages")),
    (441, 3070, Setting("audio.subtitles-off")),
    (441, 3112, Setting("audio.preferred-stream")),
    (441, 3044, Setting("audio.remember-tracks")),
    (441, 3003, Setting("audio.sync-keep")),
    (441, 3111, Setting("audio-delay-step")),
    (441, 3114, Setting("audio.default-delay")),
    (441, 3113, Setting("audio.second-delay")),
    (441, 3378, Setting("audio.voice-removal")),
    (441, 3379, Setting("audio.voice-emphasis")),
    (441, 3381, Setting("audio.swap-channels")),
    // M11: General > Navigation (443).
    (443, 3075, Missing(NAVIGATOR)),
    (
        443,
        3076,
        Missing("RVP keeps the files it makes in its state folder"),
    ),
    (
        443,
        3077,
        Missing("RVP keeps its albums in its state folder (Open playlist storage folder)"),
    ),
    (
        443,
        3078,
        Missing("speech recognition (Whisper) is not planned"),
    ),
    (443, 3079, Missing("RVP has no web view")),
    (443, 3070, Setting("image-folder")),
    (443, 3034, Setting("capture.next-to-media")),
    (443, 3073, Setting("record.video-folder")),
    (443, 3035, Setting("record.next-to-media")),
    (443, 3071, Setting("audio.recording.folder")),
    (443, 3036, Setting("audio.recording.in-media-folder")),
    (443, 3072, Setting("record.stream-folder")),
    (443, 3074, Missing(DAUM)),
    // General > Playlist (373).
    (373, 3012, Setting("playlist.thumbnail-rows")),
    (373, 3089, Setting("playlist.thumbnail-position")),
    (373, 3021, Missing(SKIN_LIST)),
    (373, 3088, Missing(SKIN_LIST)),
    (373, 3120, Missing(SKIN_LIST)),
    (373, 3104, Missing(SKIN_LIST)),
    (373, 3121, Missing(SKIN_LIST)),
    (
        373,
        3125,
        Missing("RVP's thumbnail views show pictures; an entry without video shows a black box"),
    ),
    (
        373,
        3110,
        Missing("RVP's playlist always shows the durations it knows"),
    ),
    (373, 3123, Missing(SKIN_LIST)),
    (373, 3097, Setting("playlist.remember-default")),
    (
        373,
        3129,
        Missing("sorting by folder is a command in RVP (Sort by > Sort by folder)"),
    ),
    (373, 3115, Setting("open.when-opened")),
    (373, 3117, Setting("playlist.detect-durations")),
    (
        373,
        3118,
        Missing("files dropped on the window join the shown album at its end"),
    ),
    (373, 3122, Setting("playlist.instant-albums")),
    (
        373,
        3126,
        Missing("Open a playlist adds a playlist file's entries to the shown album"),
    ),
    (373, 3119, Setting("playlist.append-on-open")),
    (373, 3124, Setting("playlist.no-duplicates")),
    (373, 3127, Setting("playlist.focus-main")),
    (373, 3128, Setting("playlist.sort-dropped")),
    (
        373,
        3130,
        Missing("RVP deletes local files only (to the Recycle Bin)"),
    ),
    (373, 3242, Setting("playlist-keys")),
    (373, 3330, Setting("playlist-keys")),
    (373, 3024, Setting("playlist-keys")),
    (373, 3027, Setting("playlist-keys")),
    (373, 3028, Setting("playlist-keys")),
    (373, 3022, Setting("playlist-keys")),
    // General > Playlist > Color/View (389): the navigator's look.
    (389, 3072, Setting("playlist.wide-thumbnails")),
    // Playback (358).
    (358, 3005, Setting("window.open-size")),
    (358, 3273, Setting("window.open-scale")),
    (358, 3275, Setting("window.custom-size")),
    (358, 3095, Setting("window.open-first-only")),
    (358, 3007, Missing(DEFAULT_ASPECT)),
    (358, 3274, Missing(DEFAULT_ASPECT)),
    (358, 3276, Missing(DEFAULT_ASPECT)),
    (358, 3014, Setting("audio.stretch")),
    (358, 3277, Setting("playback.speed-step")),
    (358, 3017, Setting("playback.on-failure")),
    (358, 3006, Setting("playback.priority")),
    (358, 3015, Setting("repeat")),
    (358, 3016, Setting("repeat")),
    (358, 3113, Setting("playback.shuffle")),
    (358, 3114, Setting("playback.shuffle")),
    (
        358,
        3094,
        Missing("libmpv opens files on its own threads: the window never waits for one"),
    ),
    (358, 3110, Setting("playback.external-audio")),
    (358, 3068, Setting("playback.folder-neighbours")),
    (358, 3084, Setting("playback.center-window")),
    (358, 3067, Setting("playback.auto-rotate")),
    (358, 3072, Setting("playback.retitle")),
    (358, 3061, Setting("remember-positions")),
    (358, 3071, Setting("playback.remember-audio-positions")),
    (358, 3083, Setting("prevent-screen-saver")),
    (358, 3089, Setting("playback.hide-pointer")),
    (358, 3124, Setting("playback.close-at-end")),
    (358, 3125, Setting("playback.recycle-played")),
    (358, 3066, Setting("playback.seek-preview")),
    (358, 3069, Setting("playback.preview-title")),
    (358, 3070, Setting("playback.preview-time")),
    (358, 3063, Setting("playback.seek-markers")),
    (358, 3062, Setting("playback.seek-tooltip")),
    (358, 3064, Setting("playback.pause-while-seeking")),
    // Playback > Time Jump (364).
    (364, 3267, Setting("seek-seconds.0")),
    (364, 3270, Setting("seek-seconds.1")),
    (364, 3271, Setting("seek-seconds.2")),
    (364, 3272, Setting("seek-seconds.3")),
    (364, 3034, Setting("keyframe-seeks")),
    // Playback > Seamless Playback (379).
    (379, 3012, Missing(SEAMLESS)),
    (379, 3105, Missing(SEAMLESS)),
    (379, 3041, Missing(SEAMLESS)),
    (
        379,
        3040,
        Missing(
            "mpv's cache keeps what it read of a stream, so seeking back works within it; RVP has no timeshift setting",
        ),
    ),
    (
        379,
        3084,
        Missing(
            "mpv's cache keeps what it read of a stream, so seeking back works within it; RVP has no timeshift setting",
        ),
    ),
    (
        379,
        3267,
        Missing(
            "mpv's cache keeps what it read of a stream, so seeking back works within it; RVP has no timeshift setting",
        ),
    ),
    (379, 3315, Setting("playback.audio-folders")),
    (
        379,
        3034,
        Missing("mpv looks in the video's folder first, then in these folders"),
    ),
    (379, 3021, Setting("playback.audio-folders")),
    (379, 3024, Setting("playback.audio-folders")),
    (379, 3022, Setting("playback.audio-folders")),
    (379, 3025, Setting("playback.audio-folders")),
    // Playback > Scene Browser (471): one size, opacity and choice for the
    // three browsers.
    (471, 3088, Setting("scenes.size")),
    (471, 3089, Setting("scenes.size")),
    (471, 3090, Setting("scenes.size")),
    (
        471,
        3004,
        Missing("RVP's browsers scroll with the wheel, the keys and a scroll bar alike"),
    ),
    (
        471,
        3006,
        Missing("RVP's browsers scroll with the wheel, the keys and a scroll bar alike"),
    ),
    (
        471,
        3007,
        Missing("RVP's browsers scroll with the wheel, the keys and a scroll bar alike"),
    ),
    (471, 3008, Setting("scenes.opacity")),
    (471, 3009, Setting("scenes.opacity")),
    (471, 3010, Setting("scenes.opacity")),
    (
        471,
        3046,
        Missing("the browser's background and pictures share its opacity"),
    ),
    (
        471,
        3047,
        Missing("the browser's background and pictures share its opacity"),
    ),
    (
        471,
        3048,
        Missing("the browser's background and pictures share its opacity"),
    ),
    (471, 3003, Setting("scenes.interval")),
    (471, 3034, Setting("scenes.close-on-change")),
    (471, 3036, Setting("scenes.close-on-change")),
    (471, 3039, Setting("scenes.close-on-change")),
    (471, 3038, Setting("scenes.fall-back-to-time")),
    (471, 3041, Setting("scenes.fall-back-to-time")),
    // Device > Screen Capture (428).
    (428, 3047, Setting("devices.capture")),
    (428, 3267, Setting("devices.capture-fps")),
    (428, 3271, Missing(CAPTURE_DEVICE)),
    (428, 3021, Missing(CAPTURE_DEVICE)),
    (428, 3013, Missing(CAPTURE_DEVICE)),
    (
        428,
        3063,
        Missing("RVP captures the whole desktop: capture areas are not in RVP"),
    ),
    (
        428,
        3068,
        Missing("RVP captures the whole desktop: capture areas are not in RVP"),
    ),
    (
        428,
        3064,
        Missing("RVP captures the whole desktop: capture areas are not in RVP"),
    ),
    (
        428,
        3069,
        Missing("RVP captures the whole desktop: capture areas are not in RVP"),
    ),
    (428, 3042, Setting("devices.capture")),
    (
        428,
        3046,
        Missing("FFmpeg's screen grabbers have no sound: a screen capture is silent"),
    ),
    (
        428,
        3051,
        Missing("FFmpeg's screen grabbers have no sound: a screen capture is silent"),
    ),
    (
        428,
        3034,
        Missing("FFmpeg's screen grabbers have no sound: a screen capture is silent"),
    ),
    // Device > Webcam (427).
    (427, 3003, Setting("devices.webcam")),
    (427, 3005, Missing(CAPTURE_DEVICE)),
    (427, 3008, Missing(CAPTURE_DEVICE)),
    (
        427,
        3006,
        Missing("a device opens with its video; its sound is not added"),
    ),
    (
        427,
        3024,
        Missing("Open Webcam/Other Device lists the devices each time it opens"),
    ),
    (
        427,
        3016,
        Missing("a device opens with its video; its sound is not added"),
    ),
    (
        427,
        3019,
        Missing("a device opens with its video; its sound is not added"),
    ),
    (
        427,
        3034,
        Missing("a device opens with its video; its sound is not added"),
    ),
    (
        427,
        3040,
        Missing(
            "mpv's cache keeps what it read, so seeking back works within it; RVP has no timeshift setting",
        ),
    ),
    // Device > DVD/Blu-ray (382).
    (382, 3007, Setting("devices.dvd")),
    (382, 3069, Setting("devices.dvd")),
    (
        382,
        3008,
        Missing("libmpv's dvdnav plays DVDs: DirectShow navigators do not apply"),
    ),
    (
        382,
        3070,
        Missing("libmpv's dvdnav plays DVDs: DirectShow navigators do not apply"),
    ),
    (382, 3003, Missing("DVD menus are M16's (goal R)")),
    (382, 3005, Setting("audio.languages")),
    (382, 3006, Setting("subtitle-languages")),
    (
        382,
        3034,
        Missing("a DVD's subtitles are tracks like any file's (Subtitles menu)"),
    ),
    (382, 3009, Setting("remember-positions")),
    (
        382,
        3010,
        Missing("skipping a disc's opening is M16's (goal R) with its menus"),
    ),
    (
        382,
        3004,
        Missing("libmpv's libbluray plays Blu-rays: there is no other method"),
    ),
    (
        382,
        3267,
        Missing("libbluray's chapters are mpv's: there is no minimum to set"),
    ),
    // Device > CD/DVD-ROM (429).
    (
        429,
        3034,
        Missing("libmpv reads an audio CD's tracks itself (libcdio, when built in)"),
    ),
    (
        429,
        3035,
        Missing("RVP's Open menu has no drive list: the disc dialog shows the system's drives"),
    ),
    (
        429,
        3036,
        Missing("Kernel Streaming devices are DirectShow's: FFmpeg opens dshow devices"),
    ),
    // Winamp DSP Plugins (365).
    (365, 3034, Missing(WINAMP_DSP)),
    (365, 3069, Missing(WINAMP_DSP)),
    (365, 3003, Missing(WINAMP_DSP)),
    (365, 3024, Missing(WINAMP_DSP)),
    (365, 3315, Missing(WINAMP_DSP)),
    // M12: Video (355).
    (355, 3036, Setting("render.renderer")),
    (355, 3081, Missing(RENDERER)),
    (355, 3076, Missing(RENDERER)),
    (355, 3075, Missing(RENDERER)),
    (355, 3073, Setting("scaler")),
    (355, 3074, Missing(RENDERER)),
    (355, 3077, Missing(RENDERER)),
    (355, 3062, Setting("render.ten-bit")),
    (355, 3341, Missing(EXCLUSIVE)),
    (355, 3061, Setting("window.restrict-cursor")),
    (355, 3040, Missing(RENDERER)),
    (355, 3052, Setting("render.limited-range")),
    (355, 3055, Setting("render.super-resolution")),
    (355, 3056, Setting("render.rtx-hdr")),
    (355, 3054, Setting("render.hdr-output")),
    (355, 3048, Setting("video-sync")),
    (
        355,
        3049,
        Missing("Qt's scene graph presents RVP's window in step with the display (vsync)"),
    ),
    (355, 3047, Setting("video-sync")),
    (355, 3344, Missing(RENDERER)),
    (355, 3053, Missing(SUBTITLES_3D)),
    (355, 3343, Missing(RENDERER)),
    (355, 3066, Missing(RENDERER)),
    // Video > Aspect Ratio (432).
    (432, 3012, Setting("picture.fit")),
    (432, 3013, Setting("picture.aspect")),
    (432, 3021, Setting("picture.custom-aspect")),
    (432, 3014, Setting("picture.output-device")),
    (432, 3040, Setting("picture.adjust-to-window")),
    (432, 3043, Setting("picture.fullscreen-mode-limited")),
    (432, 3273, Setting("picture.fullscreen-margin")),
    (432, 3016, Setting("picture.fullscreen-margin")),
    (432, 3041, Setting("picture.subtitle-margin")),
    (432, 3044, Setting("picture.subtitle-margin-resize")),
    (
        432,
        3042,
        Setting("picture.subtitle-margin-fullscreen-only"),
    ),
    (432, 3015, Setting("picture.subtitle-room")),
    // Video > Colorspaces (384).
    (384, 3100, Missing(COLOR_CONVERSION)),
    (384, 3101, Missing(COLOR_CONVERSION)),
    (384, 3139, Missing(COLOR_CONVERSION)),
    (384, 3138, Missing(COLOR_CONVERSION)),
    (384, 3168, Setting("render.matrix")),
    (384, 3176, Missing(COLOR_CONVERSION)),
    (384, 3180, Missing(COLOR_CONVERSION)),
    (384, 3045, Setting("color.brightness")),
    (384, 3322, Setting("color.brightness")),
    (384, 3043, Setting("color.contrast")),
    (384, 3346, Setting("color.contrast")),
    (384, 3058, Setting("color.saturation")),
    (384, 3347, Setting("color.saturation")),
    (384, 3053, Setting("color.hue")),
    (384, 3348, Setting("color.hue")),
    (
        384,
        3083,
        Missing(
            "mpv's renderer applies the colour controls on the GPU: there are no separate hardware controls",
        ),
    ),
    // Video > Resize (369).
    (369, 3003, Setting("picture.resize.when")),
    (369, 3029, Setting("picture.resize")),
    (369, 3005, Setting("picture.resize.method")),
    (369, 3234, Setting("picture.resize.param")),
    (369, 3016, Setting("picture.resize.keep-aspect")),
    (
        369,
        3344,
        Missing(
            "PotPlayer's resizer has a level of its own; FFmpeg's scale filter has its method's parameter (picture.resize.param)",
        ),
    ),
    (369, 3179, Setting("picture.resize.condition-display")),
    (369, 3184, Setting("picture.resize.condition-display")),
    (369, 3267, Setting("picture.resize.condition-size")),
    (369, 3271, Setting("picture.resize.condition-size")),
    (369, 3021, Setting("picture.resize.condition-size")),
    (369, 3004, Setting("picture.resize.condition-size")),
    (369, 3268, Setting("picture.resize.condition-display")),
    (369, 3178, Setting("picture.resize.target")),
    (369, 3183, Setting("picture.resize.target")),
    (369, 3186, Setting("picture.resize.target")),
    (369, 3272, Setting("picture.resize.factor")),
    (369, 3273, Setting("picture.resize.width")),
    (369, 3275, Setting("picture.resize.height")),
    (369, 3024, Setting("picture.resize.width")),
    (369, 3269, Setting("picture.resize.target")),
    (369, 3097, Setting("picture.resize.auto-height")),
    (369, 3013, Setting("picture.resize.interlaced")),
    // Video > Extend/Crop (374).
    (374, 3073, Setting("picture.extend")),
    (374, 3162, Setting("picture.extend")),
    (374, 3104, Setting("picture.extend-first")),
    (374, 3105, Setting("picture.extend-scale-height")),
    (374, 3276, Setting("picture.extend-size")),
    (374, 3277, Setting("picture.extend-size")),
    (374, 3278, Setting("picture.extend-ratio")),
    (374, 3279, Setting("picture.extend-ratio")),
    (374, 3079, Setting("picture.position")),
    (374, 3088, Setting("picture.position")),
    (374, 3084, Setting("picture.position")),
    (374, 3095, Setting("picture.position")),
    (374, 3110, Setting("picture.bottom-margin-subtitles-only")),
    (374, 3100, Setting("picture.bottom-margin")),
    (
        374,
        3111,
        Missing(
            "PotPlayer's DirectShow transform filter; RVP's bottom margin is FFmpeg's pad filter, which has no such failure",
        ),
    ),
    // Video > Deinterlacing (385).
    (385, 3349, Setting("deinterlace")),
    (385, 3176, Setting("render.field-order")),
    (385, 3177, Setting("render.field-order")),
    (385, 3350, Setting("deinterlace")),
    (385, 3180, Setting("deinterlace")),
    (385, 3351, Setting("render.deinterlacer")),
    (385, 3352, Setting("render.field-order")),
    (
        385,
        3107,
        Missing(
            "mpv uses the hardware's own deinterlacer (d3d11vpp, VA-API) with hardware decoding; its method is the driver's",
        ),
    ),
    (385, 3172, Setting("render.field-order")),
    // Video > Levels/Offset (371).
    (371, 3057, Setting("effects")),
    (371, 3061, Setting("video-effects.levels.auto-full-range")),
    (371, 3066, Setting("effects")),
    (371, 3072, Setting("video-effects.levels.full-range")),
    (371, 3083, Setting("video-effects.levels.luma-only")),
    (371, 3276, Setting("video-effects.levels.input")),
    (371, 3277, Setting("video-effects.levels.input")),
    (371, 3078, Setting("video-effects.levels.auto-input")),
    (371, 3279, Setting("video-effects.levels.output")),
    (371, 3282, Setting("video-effects.levels.output")),
    (371, 3239, Setting("video-effects.levels.gamma")),
    (371, 3321, Setting("video-effects.levels.gamma")),
    (371, 3110, Setting("effects")),
    (371, 3354, Setting("video-effects.levels.luma-offset")),
    (371, 3088, Setting("video-effects.levels.luma-offset")),
    (371, 3355, Setting("video-effects.levels.luma-offset")),
    (371, 3095, Setting("video-effects.levels.luma-offset")),
    (371, 3356, Setting("video-effects.levels.chroma-offset")),
    (371, 3254, Setting("video-effects.levels.chroma-offset")),
    (371, 3357, Setting("video-effects.levels.chroma-offset")),
    (371, 3147, Setting("video-effects.levels.chroma-offset")),
    // Video > Effects (386).
    (386, 3061, Setting("effects")),
    (386, 3372, Setting("video-effects.rotate-angle")),
    (
        386,
        3374,
        Missing("FFmpeg's filters thread themselves where they can; there is no switch"),
    ),
    (386, 3149, Setting("effects")),
    (386, 3373, Setting("effects")),
    (386, 3110, Setting("effects")),
    (386, 3359, Setting("video-effects.soften")),
    (386, 3360, Setting("video-effects.soften")),
    (386, 3361, Setting("video-effects.soften")),
    (386, 3115, Setting("effects")),
    (386, 3084, Setting("video-effects.sharpen")),
    (386, 3353, Setting("video-effects.sharpen")),
    (386, 3124, Setting("effects")),
    (386, 3231, Setting("video-effects.deblock")),
    (386, 3138, Setting("effects")),
    (386, 3223, Setting("video-effects.gradual")),
    (386, 3133, Setting("effects")),
    (386, 3332, Setting("video-effects.denoise-quality")),
    (386, 3098, Setting("video-effects.denoise")),
    (386, 3103, Setting("video-effects.denoise")),
    (386, 3362, Setting("video-effects.denoise")),
    (386, 3363, Setting("effects")),
    (386, 3364, Setting("video-effects.temporal")),
    (386, 3227, Setting("video-effects.temporal")),
    (386, 3365, Setting("video-effects.temporal")),
    (386, 3366, Setting("effects")),
    (386, 3367, Setting("video-effects.warpsharp")),
    (
        386,
        3368,
        Missing(
            "WarpSharp is approximated by FFmpeg's cas filter, which has one strength (video-effects.warpsharp)",
        ),
    ),
    (386, 3370, Setting("deband")),
    (386, 3375, Setting("video-effects.deband")),
    (386, 3376, Setting("video-effects.deband")),
    // Video > Pixel Shaders (505).
    (505, 3040, Setting("render.icc")),
    (
        505,
        3041,
        Missing("mpv converts YCgCo video from the file's tags itself"),
    ),
    (505, 3042, Setting("render.tone-mapping-on")),
    (505, 3239, Setting("render.target-peak")),
    (505, 3100, Setting("render.tone-mapping")),
    (505, 3043, Setting("render.hdr-metadata")),
    (505, 3044, Setting("shaders.enabled")),
    (505, 3243, Setting("shaders.files")),
    (505, 3045, Setting("shaders.post-enabled")),
    (
        505,
        3046,
        Missing("mpv's shaders always run on the video image only"),
    ),
    (505, 3244, Setting("shaders.post-files")),
    (505, 3109, Setting("shaders.files")),
    (505, 3110, Setting("shaders.post-files")),
    // Video > AviSynth (452).
    (452, 3036, Missing(AVISYNTH)),
    (452, 3041, Missing(AVISYNTH)),
    (452, 3037, Missing(AVISYNTH)),
    (452, 3038, Missing(AVISYNTH)),
    (452, 3039, Missing(AVISYNTH)),
    (452, 3090, Missing(AVISYNTH)),
    (452, 3034, Missing(AVISYNTH)),
    (452, 3035, Missing(AVISYNTH)),
    (452, 3088, Missing(AVISYNTH)),
    (452, 3089, Missing(AVISYNTH)),
    (452, 3040, Missing(AVISYNTH)),
    (452, 3360, Missing(AVISYNTH)),
    (452, 3069, Missing(AVISYNTH)),
    // Video > VapourSynth (487).
    (487, 3036, Setting("vapoursynth.enabled")),
    (
        487,
        3041,
        Missing("mpv's vapoursynth filter gets every frame and cannot let interlaced ones by"),
    ),
    (
        487,
        3039,
        Missing(
            "mpv's vapoursynth filter starts with the video; how far it reads ahead is vapoursynth.buffered-frames",
        ),
    ),
    (
        487,
        3090,
        Missing(
            "mpv's vapoursynth filter starts with the video; how far it reads ahead is vapoursynth.buffered-frames",
        ),
    ),
    (487, 3034, Setting("vapoursynth.buffered-frames")),
    (487, 3040, Setting("vapoursynth.script")),
    (
        487,
        3360,
        Missing(
            "RVP runs scripts from their files and edits none: the script is the user's own file",
        ),
    ),
    (487, 3069, Setting("vapoursynth.slots")),
    // Video > H.264 MVC 3D (448).
    (448, 3142, Missing(MVC)),
    (448, 3352, Missing(MVC)),
    (448, 3350, Setting("stereo.mode")),
    (448, 3349, Setting("stereo.input")),
    (448, 3088, Setting("stereo.offset")),
    (448, 3089, Setting("stereo.offset")),
    (448, 3138, Setting("stereo.full-resolution")),
    (448, 3139, Setting("stereo.swap")),
    (448, 3351, Setting("stereo.output")),
    (448, 3141, Setting("stereo.auto-detect")),
    // Playback > Fullscreen Mode (456).
    (456, 3008, Setting("window.fullscreen-display")),
    (
        456,
        3009,
        Missing(
            "a fullscreen window covers one display (Qt's and the platform's fullscreen); the others can turn black (window.black-other-displays)",
        ),
    ),
    (
        456,
        3101,
        Missing("a fullscreen window covers one display (Qt's and the platform's fullscreen)"),
    ),
    (
        456,
        3010,
        Missing(
            "RVP leaves the display's mode as it is; video-sync=display-resample matches the video to the display instead",
        ),
    ),
    (
        456,
        3102,
        Missing(
            "the skin lays out its fullscreen chrome; RVP shows the panes the skin shows there",
        ),
    ),
    (456, 3103, Setting("window.black-other-displays")),
    // Playback > Window Size (490).
    (490, 3267, Setting("window.desktop-sizes")),
    (490, 3268, Setting("window.desktop-sizes")),
    (490, 3269, Setting("window.desktop-sizes")),
    (490, 3270, Setting("window.desktop-sizes")),
    (490, 3102, Setting("window.double-below")),
    (490, 3089, Setting("window.double-below")),
    (
        490,
        3103,
        Missing("RVP keeps the window inside the screen's free area at every size it sets"),
    ),
    (
        490,
        3090,
        Missing("RVP keeps the window inside the screen's free area at every size it sets"),
    ),
    (490, 3091, Setting("window.double-below")),
    // General > Skins > Visualization (372).
    (372, 3040, Setting("visual.enabled")),
    (372, 3041, Setting("visual.album-art")),
    (372, 3337, Setting("visual.size")),
    (372, 3338, Setting("visual.rate")),
    (372, 3012, Setting("visual.mode")),
    (372, 3018, Missing(WMP)),
    (372, 3339, Missing(WMP)),
    (372, 3027, Missing(WMP)),
    (372, 3042, Setting("visual.change-every")),
    (372, 3089, Setting("visual.change-every")),
    (372, 3021, Setting("visual.folders")),
    (372, 3024, Setting("visual.folders")),
    (372, 3242, Setting("visual.folders")),
    (372, 3088, Setting("visual.slide-seconds")),
    (372, 3095, Setting("visual.fade-seconds")),
    (372, 3015, Setting("visual.fade-seconds")),
    (372, 3016, Setting("visual.fill")),
    (372, 3034, Setting("visual.random")),
    (372, 3035, Setting("visual.show-name")),
    // Filter Control > Video and Audio Decoder (360, 361).
    (360, 3110, Setting("decoders")),
    (361, 3110, Setting("audio.passthrough-device")),
    // Subtitles (353).
    (
        353,
        3003,
        Missing("DirectShow output surfaces do not apply to libmpv"),
    ),
    (353, 3009, Missing(LIBASS)),
    (353, 3007, Missing(LIBASS)),
    (353, 3008, Setting("subtitles.ass-override")),
    (353, 3041, Setting("subtitles.ass-override")),
    (353, 3042, Setting("subtitle-scale")),
    (353, 3043, Setting("subtitles.override-font")),
    (353, 3044, Setting("subtitles.override-color")),
    (
        353,
        3045,
        Missing("mpv's style overrides always reach every style of a script"),
    ),
    (
        353,
        3046,
        Missing("mpv's style overrides always reach every style of a script"),
    ),
    (353, 3040, Setting("subtitles.show")),
    (
        353,
        3119,
        Missing("DirectShow output surfaces do not apply to libmpv"),
    ),
    (353, 3066, Setting("subtitles.fix-aspect")),
    (
        353,
        3124,
        Missing("mpv justifies every subtitle alike (sub-justify), not by its first character"),
    ),
    (
        353,
        3126,
        Missing("mpv cannot add lines to a subtitle; one-line subtitles sit on the margin"),
    ),
    (353, 3130, Setting("subtitles.fonts-folder")),
    (
        353,
        3129,
        Missing(
            "an online video's subtitles are its own tracks (mpv's ytdl hook); RVP looks for no \
             files beside a URL",
        ),
    ),
    (353, 3131, Setting("subtitle-auto")),
    (353, 3067, Setting("subtitle-auto")),
    (
        353,
        3054,
        Missing("mpv shows closed captions and teletext as decoded; it has no case option"),
    ),
    (353, 3049, Setting("subtitles.auto-reload")),
    (353, 3059, Setting("subtitles.bitmap-outside")),
    (353, 3057, Setting("subtitles.text-inside")),
    (353, 3113, Setting("subtitles.under-video")),
    (
        353,
        3123,
        Missing(
            "mpv moves bitmap subtitles with the subtitle position whenever it overrides \
             styles (subtitles.ass-override other than no)",
        ),
    ),
    (353, 3047, Missing(SAMI)),
    (353, 3048, Missing(SAMI)),
    (353, 3050, Missing(SAMI)),
    (
        353,
        3053,
        Missing("mpv (libass) always animates ASS subtitles"),
    ),
    (
        353,
        3127,
        Missing("mpv shows a subtitle's text as it is; its filter only drops whole subtitles"),
    ),
    (353, 3128, Setting("subtitles.remove-blank")),
    (353, 3132, Setting("subtitles.remove-sdh")),
    (353, 3004, Missing(SUBPICTURES)),
    (
        353,
        3089,
        Missing("mpv scales bitmap and text subtitles by one size (subtitle-scale)"),
    ),
    (353, 3006, Missing(SUBPICTURES)),
    (353, 3088, Missing(SUBPICTURES)),
    // Subtitles > Font Style (383).
    (383, 3021, Setting("subtitle-font")),
    (
        383,
        3022,
        Missing("mpv draws the second subtitle in the first one's style"),
    ),
    (
        383,
        3038,
        Missing("RVP lists the system's fonts (Qt); libass finds glyphs for every alphabet"),
    ),
    (
        383,
        3003,
        Missing("libass reads Unicode text; Windows' GDI charsets do not apply"),
    ),
    (383, 3099, Setting("subtitles.scale-x")),
    (383, 3105, Setting("subtitles.scale-y")),
    (383, 3088, Setting("subtitle-scale")),
    (383, 3077, Setting("subtitles.spacing")),
    (383, 3095, Setting("subtitles.line-spacing")),
    (383, 3096, Missing("libass has no ruby (furigana) text")),
    (383, 3045, Setting("subtitles.color")),
    (383, 3046, Setting("subtitles.outline-color")),
    (383, 3069, Setting("subtitles.outline-size")),
    (383, 3047, Missing("libass draws one outline")),
    (383, 3072, Missing("libass draws one outline")),
    (383, 3048, Setting("subtitles.back-color")),
    (383, 3082, Setting("subtitles.shadow-offset")),
    (
        383,
        3070,
        Missing("libass's edge blur (\\be) has no mpv option; its gaussian blur is subtitles.blur"),
    ),
    (383, 3071, Setting("subtitles.blur")),
    (383, 3036, Setting("subtitles.border-style")),
    (
        383,
        3035,
        Missing(
            "mpv always scales its own subtitles' outline and shadow with the text; ASS scripts say it themselves (ScaledBorderAndShadow)",
        ),
    ),
    (383, 3037, Missing("libass draws one outline")),
    (383, 3005, Setting("subtitles.align")),
    (
        383,
        3271,
        Missing(
            "mpv puts subtitles at an edge or the centre with a side margin (subtitles.align, \
             subtitles.margin-x), not at a horizontal position",
        ),
    ),
    (383, 3253, Setting("subtitles.position")),
    (383, 3267, Setting("subtitles.margin-y")),
    (383, 3247, Setting("subtitles.margin-x")),
    (383, 3251, Setting("subtitles.margin-x")),
    (383, 3272, Setting("subtitles.margin-y")),
    (383, 3034, Setting("subtitles.border-style")),
    (383, 3044, Setting("subtitles.back-color")),
    // Subtitles > Fade/3D/Location (368).
    (368, 3339, Missing(crate::coverage::SUBTITLE_FADE)),
    (368, 3112, Missing(crate::coverage::SUBTITLE_FADE)),
    (368, 3114, Missing(crate::coverage::SUBTITLE_FADE)),
    (368, 3007, Missing(SUBTITLES_3D)),
    (368, 3006, Missing(SUBTITLES_3D)),
    (368, 3116, Missing(SUBTITLES_3D)),
    (368, 3117, Missing(SUBTITLES_3D)),
    (368, 3143, Missing(SUBTITLES_3D)),
    (368, 3142, Missing(SUBTITLES_3D)),
    (368, 3315, Setting("subtitles.folders")),
    (
        368,
        3034,
        Missing(
            "mpv orders the subtitle files it finds by how well their names match, wherever they are",
        ),
    ),
    (368, 3021, Setting("subtitles.folders")),
    (368, 3024, Setting("subtitles.folders")),
    (368, 3022, Setting("subtitles.folders")),
    (368, 3025, Setting("subtitles.folders")),
    // Subtitles > Language/Sync/Other (442).
    (
        442,
        3003,
        Missing(
            "mpv picks a subtitle by language, then external files first, then the default flag",
        ),
    ),
    (442, 3069, Setting("subtitle-languages")),
    (442, 3384, Setting("subtitles.only-preferred")),
    (442, 3070, Setting("subtitles.second-languages")),
    (442, 3117, Setting("subtitles.second-position")),
    (442, 3044, Missing(SAMI)),
    (442, 3061, Missing(SAMI)),
    (442, 3062, Setting("subtitles.second-auto")),
    (442, 3063, Setting("subtitles.remember-tracks")),
    (442, 3111, Setting("subtitle-delay-step")),
    (442, 3112, Setting("subtitles.sync-step-2")),
    (442, 3113, Setting("subtitles.sync-step-3")),
    (442, 3380, Setting("subtitles.keep-sync")),
    (442, 3116, Missing(SHOWING_PERIOD)),
    (442, 3115, Missing(SHOWING_PERIOD)),
    (442, 3321, Missing(SHOWING_PERIOD)),
    (442, 3319, Missing(SHOWING_PERIOD)),
    // Subtitles > Subtitle Browser (478).
    (478, 3004, Setting("subtitles.browser.times")),
    (478, 3003, Setting("subtitles.browser.synced-times")),
    (478, 3008, Setting("subtitles.browser.opacity")),
    (478, 3021, Setting("subtitles.browser.font")),
    (478, 3088, Setting("subtitles.browser.font-size")),
    (478, 3005, Setting("subtitles.browser.lines")),
    (
        478,
        3046,
        Missing(
            "a window with a frame is see-through only as a whole (Qt's window opacity, \
             subtitles.browser.opacity)",
        ),
    ),
    (478, 3036, Setting("subtitles.browser.follow")),
    (
        442,
        3385,
        Missing("mpv (libass) always shows overlapping subtitles together, one above the other"),
    ),
];

/// The milestone or reason for whole pages, by the start of their place;
/// the longest wins.
const PAGES: &[(&str, &str)] = &[
    ("General", "planned in M14 (goal P)"),
    (
        "General > Keyboard",
        "RVP's Keys and Mouse page (M5); the rest in M14 (goal P)",
    ),
    (
        "General > Mouse",
        "RVP's Keys and Mouse page (M5); the rest in M14 (goal P)",
    ),
    ("General > Playlist", "not in RVP"),
    ("General > Playlist > Color/View", NAVIGATOR),
    ("General > Navigation", "not in RVP"),
    ("Playback", "not in RVP"),
    ("Subtitles", "planned in M13 (goal O)"),
    (
        "Subtitles > Online Subtitle Database",
        crate::coverage::ONLINE_SUBTITLES,
    ),
    (
        "Subtitles > Subtitle Translation",
        crate::coverage::ONLINE_SUBTITLES,
    ),
    (
        "Subtitles > Word Searching",
        crate::coverage::ONLINE_SUBTITLES,
    ),
    ("Device", "not in RVP"),
    (
        "Device > Analog TV (WDM)",
        "TV tuners are not planned (non-goals)",
    ),
    (
        "Device > Digital TV (BDA)",
        "TV tuners are not planned (non-goals)",
    ),
    (
        "Device > TV Channels",
        "TV tuners are not planned (non-goals)",
    ),
    ("Device > EPG", "TV tuners are not planned (non-goals)"),
    ("Filter Control", DIRECTSHOW),
    ("Video", RENDERER),
    (
        "Extensions",
        "PotPlayer's AngelScript extensions: with the deferred online subtitles (M13)",
    ),
    ("Accessibility", "planned in M14 (goal P)"),
    (
        "Association",
        "RVP registers its types itself (M8, packaging/associations.tsv); this page's options: M14 (goal P)",
    ),
    ("Configuration", "planned in M14 (goal P)"),
    ("Screen Saver", "planned in M14 (goal P)"),
    ("Kakao TV", DAUM),
];

/// Every option of R2's pages, in their order (group boxes left out).
#[must_use]
pub fn options() -> Vec<PageOption> {
    PREFERENCES_TSV
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            let kind = (*cells.get(3)?).to_owned();
            (kind != "group").then(|| {
                Some(PageOption {
                    page: (*cells.first()?).to_owned(),
                    dialog: cells.get(1)?.parse().ok()?,
                    control: cells.get(2)?.parse().ok()?,
                    kind,
                    text: (*cells.get(4)?).to_owned(),
                    group: cells.get(5).copied().unwrap_or_default().to_owned(),
                })
            })?
        })
        .collect()
}

/// What RVP does with an option.
#[must_use]
pub fn status(option: &PageOption) -> Status {
    if let Some((_, _, status)) = ITEMS
        .iter()
        .find(|(dialog, control, _)| *dialog == option.dialog && *control == option.control)
    {
        return *status;
    }
    Missing(
        PAGES
            .iter()
            .filter(|(page, _)| {
                option.page == *page || option.page.starts_with(&format!("{page} > "))
            })
            .max_by_key(|(page, _)| page.len())
            .map_or("not in RVP yet", |(_, reason)| reason),
    )
}

/// `docs/coverage/preferences.md`.
#[must_use]
pub fn report() -> String {
    let options = options();
    let covered = options
        .iter()
        .filter(|option| matches!(status(option), Setting(_)))
        .count();
    let mut out = format!(
        "# PotPlayer's Preferences in RVP\n\nGenerated by `cargo test -p rvp-app --test coverage` \
         (`RVP_BLESS=1` rewrites it) from R2's `docs/r1/preferences.tsv`: every option of \
         PotPlayer's Preferences pages, the RVP setting that does it (its place in \
         `settings.json`), or why RVP has none.\n\n{} options: {covered} in RVP, {} not.\n",
        options.len(),
        options.len() - covered
    );
    let mut pages: BTreeMap<(usize, &str), Vec<&PageOption>> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for option in &options {
        if !order.contains(&option.page.as_str()) {
            order.push(&option.page);
        }
        let index = order
            .iter()
            .position(|page| *page == option.page)
            .unwrap_or_default();
        pages.entry((index, &option.page)).or_default().push(option);
    }
    for ((_, page), list) in pages {
        let dialog = list.first().map_or(0, |option| option.dialog);
        let _ = writeln!(
            out,
            "\n## {page} (dialog {dialog})\n\n| Option | Kind | Group | RVP |\n|--------|------|-------|-----|"
        );
        for option in list {
            let text = if option.text.is_empty() {
                format!("(unnamed {})", option.kind)
            } else {
                option.text.replace('|', "/")
            };
            let rvp = match status(option) {
                Setting(place) => format!("`{place}`"),
                Missing(why) => format!("not: {why}"),
            };
            let _ = writeln!(
                out,
                "| {text} | {} | {} | {rvp} |",
                option.kind,
                option.group.replace('|', "/")
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_option_is_on_a_page_and_named_once() {
        let options = options();
        assert!(options.len() > 900, "{}", options.len());
        for (dialog, control, _) in ITEMS {
            assert!(
                options
                    .iter()
                    .any(|option| option.dialog == *dialog && option.control == *control),
                "{dialog}/{control} is not in preferences.tsv"
            );
        }
        let mut keys: Vec<(u32, u32)> = ITEMS.iter().map(|(d, c, _)| (*d, *c)).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), ITEMS.len(), "an option is listed twice");
    }

    #[test]
    fn the_audio_pages_say_why_for_every_option_rvp_lacks() {
        for option in options() {
            if (option.page == "Audio" || option.page.starts_with("Audio >"))
                && let Missing(why) = status(&option)
            {
                assert!(
                    !why.contains("not in RVP yet") && !why.starts_with("planned"),
                    "{} / {} ({}): {why}",
                    option.page,
                    option.text,
                    option.control
                );
            }
        }
    }
}
