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
const WINDOW_SIZES: &str = "planned in M12 (goal N) with the window sizes";
const CAPTURE_DEVICE: &str = "FFmpeg's dshow and gdigrab choose the format and size; RVP opens \
                              the device or screen as it is";
const WINAMP_DSP: &str = "mpv's audio chain runs FFmpeg's filters only: native Winamp DSP \
                          plugins cannot run in it";

/// The options RVP has, and the reasons for single ones it lacks, by
/// dialog and control.
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
            "PotPlayer's own decoders (DirectShow); RVP decodes with mpv, and M12 adds a \
             decoders page",
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
    (
        443,
        3034,
        Missing("saved frames go to the image folder; M12 (goal N) takes the frame options"),
    ),
    (443, 3073, Missing("video recording is not planned")),
    (443, 3035, Missing("video recording is not planned")),
    (443, 3071, Setting("audio.recording.folder")),
    (443, 3036, Setting("audio.recording.in-media-folder")),
    (443, 3072, Missing("RVP does not save streams")),
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
    (358, 3005, Missing(WINDOW_SIZES)),
    (358, 3273, Missing(WINDOW_SIZES)),
    (358, 3275, Missing(WINDOW_SIZES)),
    (358, 3095, Missing(WINDOW_SIZES)),
    (358, 3007, Missing(WINDOW_SIZES)),
    (358, 3274, Missing(WINDOW_SIZES)),
    (358, 3276, Missing(WINDOW_SIZES)),
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
    ("Playback > Fullscreen Mode", "planned in M12 (goal N)"),
    ("Playback > Window Size", "planned in M12 (goal N)"),
    ("Subtitles", "planned in M13 (goal O)"),
    (
        "Subtitles > Online Subtitle Database",
        "online subtitles: deferred by the owner (2026-10-16)",
    ),
    (
        "Subtitles > Subtitle Translation",
        "online subtitles: deferred by the owner (2026-10-16)",
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
    (
        "Filter Control",
        "DirectShow filters do not apply to libmpv; M12 (goal N) adds a decoders page",
    ),
    ("Video", "planned in M12 (goal N)"),
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
