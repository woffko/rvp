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
    ("General > Playlist", "planned in M11 (goal M)"),
    ("General > Navigation", "planned in M11 (goal M)"),
    ("Playback", "planned in M11 (goal M)"),
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
    ("Device", "planned in M11 (goal M)"),
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
