//! RVP's part of mpv's audio filters (M10), built at once from the
//! settings and the file that plays, in this order, each under its label:
//!
//! 0. `@rvpspectrum`: while a skin with a spectrum shows, a tap that
//!    measures the audio's bands and leaves the audio as it is
//!    (`crate::spectrum`).
//! 1. `@rvpdownmix`: the output speakers (`crate::speakers`): PotPlayer's
//!    mix to the layout with the mixer's levels, stereo spread to the
//!    centre and surround speakers, the LFE redirection, the matrix
//!    encodings, the virtual headphone and surround (FFmpeg's `headphone`
//!    with the head model of `crate::audio::dsp`) or the Dolby decoder.
//! 2. `@rvpchannel`: Both, Left or Right channel, and swapped channels.
//! 3. `@rvppreamp`: AC3 and DTS sources raised.
//! 4. `@rvpeq`: the equalizer (`crate::equalizer`).
//! 5. The effects: `@rvpvoice` (voice removal and emphasis),
//!    `@rvpcrystality`, `@rvpdenoise`, `@rvpreverb` (Freeverb, convolved
//!    with `afir`), `@rvpcrossfeed`.
//! 6. The user's own filters, which RVP keeps as they are.
//! 7. `@rvpnorm`: the normalizer (`crate::normalizer`).
//! 8. `@rvpgains`: the individual channel volumes.
//! 9. `@rvptempo`: the tempo filter speed changes use.
//! 10. `@rvpac3`: AC3 re-encoding for pass-through (mpv's `lavcac3enc`).
//!
//! The effects work before the normalizer, which levels what they made;
//! the channel volumes come after it, so that it does not undo them. The
//! whole chain goes to mpv as one `af` value (`assemble`), so a change
//! rebuilds it once. Graphs are quoted with mpv's `%N%` length prefix,
//! which takes brackets and commas as they are.
//!
//! PotPlayer's effects are its own code; RVP builds them from FFmpeg's
//! filters, fitted to what goal L's host session measured of PotPlayer's
//! at their defaults (`docs/audits/audio.md`, "M10"): voice emphasis and
//! noise reduction are fixed filters there, crossfeed is bs2b, Crystality
//! a rectifier, shelves, extra stereo and a measured echo, Freeverb the
//! original's delays.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::audio::{Audio, OutputMode, StereoChannel, Stretch, dsp};
use crate::equalizer::Equalizer;
use crate::normalizer::Normalizer;
use crate::speakers::{Choice, Mode};

pub const CHANNEL: &str = "rvpchannel";
pub const PREAMP: &str = "rvppreamp";
pub const VOICE: &str = "rvpvoice";
pub const CRYSTALITY: &str = "rvpcrystality";
pub const DENOISE: &str = "rvpdenoise";
pub const REVERB: &str = "rvpreverb";
pub const CROSSFEED: &str = "rvpcrossfeed";
pub const GAINS: &str = "rvpgains";
pub const AC3: &str = "rvpac3";

/// What the chain needs to know about the file that plays.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    /// The audio track's codec (`ac3`, `dts`, `flac`).
    pub codec: Option<String>,
    /// The audio track's channels in FFmpeg's names, when RVP knows its
    /// layout.
    pub channels: Vec<&'static str>,
    /// The file has a video track (not just cover art).
    pub video: bool,
}

/// The impulse responses the chain's graphs read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Responses {
    pub freeverb: Option<dsp::Response>,
    pub crystality: Option<dsp::Response>,
    pub head_model: Option<PathBuf>,
}

/// Everything the chain depends on.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    pub audio: &'a Audio,
    pub speakers: &'a Choice,
    pub equalizer: &'a Equalizer,
    pub normalizer: &'a Normalizer,
    pub channel: StereoChannel,
    /// "Audio Stretching" in percent from 100 (`commands::Session`).
    pub stretching: i32,
    pub source: &'a Source,
    /// The normalizer's current gain.
    pub gain: f64,
    pub responses: &'a Responses,
    /// A skin shows the audio's spectrum (`crate::spectrum`).
    pub spectrum: bool,
}

/// One of RVP's filters: its label and its `af` entry.
pub type Entry = (&'static str, String);

/// An `af` entry for a libavfilter graph.
#[must_use]
pub fn lavfi(label: &str, graph: &str) -> String {
    format!("@{label}:lavfi=graph=%{}%{graph}", graph.len())
}

/// A file name for a graph's `amovie`. FFmpeg takes it apart twice: the
/// graph's parser takes its quotes and escapes off the filter's options,
/// then the options' parser takes its own off the value, where `:`
/// separates options. So `\`, `'` and `:` are escaped for the options
/// first, and the result is quoted for the graph, inside which `[`, `]`,
/// `,` and `;` are plain. On Windows the name has forward slashes, which
/// FFmpeg takes there as well.
#[must_use]
pub fn graph_path(path: &Path) -> String {
    let mut text = path.display().to_string();
    if cfg!(windows) {
        text = text.replace('\\', "/");
    }
    let mut option = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if matches!(c, '\\' | '\'' | ':') {
            option.push('\\');
        }
        option.push(c);
    }
    format!("'{}'", option.replace('\'', "'\\''"))
}

fn db(value: f64) -> f64 {
    10f64.powf(value / 20.0)
}

/// FFmpeg's layout text for channels: `FL+FR+LFE`.
fn layout(channels: &[&str]) -> String {
    channels.join("+")
}

/// A `pan` to the same channels, each made by `expression` (FFmpeg's
/// `pan` syntax: `0.5*FL+0.5*FR`).
fn pan(channels: &[&str], expression: impl Fn(&str) -> String) -> String {
    let mut text = format!("pan={}", layout(channels));
    for channel in channels {
        let _ = write!(text, "|{channel}={}", expression(channel));
    }
    text
}

/// The output's channels: the speakers', or the source's for "Same as
/// input" (unknown until a file plays).
#[must_use]
pub fn output_channels<'a>(speakers: &Choice, source: &'a Source) -> Vec<&'a str> {
    speakers
        .channels()
        .map_or_else(|| source.channels.clone(), <[&str]>::to_vec)
}

/// The mixer's swresample options: the levels for the centre, the
/// surround channels and the LFE (or none), relative to the front, which
/// the mix then gets through a `volume`.
fn mix_options(audio: &Audio, with_lfe: bool) -> String {
    let mixer = &audio.mixer;
    let front = db(mixer.front);
    // swresample gives each side the LFE at -3 dB with lfe_mix_level 1.
    let lfe = if with_lfe {
        db(mixer.lfe + 3.0103)
    } else {
        0.0
    };
    format!(
        "clev={:.6}:slev={:.6}:lfe_mix_level={:.6}:rematrix_maxval={}",
        db(mixer.center) / front,
        db(mixer.surround) / front,
        lfe / front,
        if mixer.normalize { "1" } else { "1000" }
    )
}

fn front_volume(audio: &Audio) -> String {
    if audio.mixer.front.abs() < 1e-9 {
        String::new()
    } else {
        format!(",volume={:.6}", db(audio.mixer.front))
    }
}

/// A stereo source spread to the speakers that the mixer's expansions ask
/// for, or `None` when the plain mix will do.
fn expansion(audio: &Audio, channels: &[&'static str], source: &Source) -> Option<String> {
    let mixer = &audio.mixer;
    if source.channels != ["FL", "FR"] {
        return None;
    }
    let centre = mixer.expand_center && channels.contains(&"FC");
    let surround = mixer.expand_surround
        && channels
            .iter()
            .any(|c| matches!(*c, "BL" | "BR" | "SL" | "SR" | "BC"));
    if !centre && !surround {
        return None;
    }
    let front = db(mixer.front);
    let (c, s) = (db(mixer.center) / 2.0, db(mixer.surround));
    Some(pan(channels, |channel| match channel {
        "FL" => format!("{front:.6}*c0"),
        "FR" => format!("{front:.6}*c1"),
        "FC" if centre => format!("{c:.6}*c0+{c:.6}*c1"),
        "BL" | "SL" if surround => format!("{s:.6}*c0"),
        "BR" | "SR" if surround => format!("{s:.6}*c1"),
        "BC" if surround => format!("{:.6}*c0+{:.6}*c1", s / 2.0, s / 2.0),
        _ => "0*c0".to_owned(),
    }))
}

/// The LFE redirection for a layout with an LFE channel: the band below
/// the crossover of every other channel is added to the LFE, and with
/// "remove low frequencies" taken out of them.
fn lfe_redirect(audio: &Audio, channels: &[&str]) -> Option<String> {
    let redirect = &audio.lfe_redirect;
    if !redirect.enabled || !channels.contains(&"LFE") {
        return None;
    }
    let mains: Vec<&str> = channels.iter().copied().filter(|c| *c != "LFE").collect();
    let gain = db(redirect.gain);
    let sum = mains
        .iter()
        .map(|channel| format!("{gain:.6}*{channel}"))
        .collect::<Vec<_>>()
        .join("+");
    let split = format!("acrossover=split={:.0}:order=4th", redirect.frequency);
    Some(if redirect.remove_low {
        format!(
            "{split}[lo][hi];[lo]{}[lfe];[hi]{}[mains];[mains][lfe]amix=inputs=2:normalize=0",
            pan(channels, |c| if c == "LFE" {
                format!("{sum}+LFE")
            } else {
                format!("0*{c}")
            }),
            pan(channels, |c| if c == "LFE" {
                "0*LFE".to_owned()
            } else {
                c.to_owned()
            })
        )
    } else {
        format!(
            "asplit=2[full][x];[x]{split}[lo][hi];[hi]anullsink;[lo]{}[lfe];\
             [full][lfe]amix=inputs=2:normalize=0",
            pan(channels, |c| if c == "LFE" {
                sum.clone()
            } else {
                format!("0*{c}")
            })
        )
    })
}

/// `headphone`'s gain in dB: the filter lowers its output by 3 dB per
/// input channel (its `convert_coeffs`), which this gives back for the
/// eight speakers of 7.1, less 3 dB of headroom for all of them together
/// and 13.5 dB more: PotPlayer's Virtual Headphone and Virtual Surround
/// played a speaker about that much below its 2.0 Stereo (goal L's host
/// session; the LFE less, `HEADPHONE_LFE_GAIN`).
const HEADPHONE_GAIN: f64 = 3.0 * 8.0 - 3.0 - 13.5;
/// `headphone` gives the LFE to both ears as it is, with this gain in dB:
/// PotPlayer's virtual modes played it 4.6 and 7 dB below its 2.0 Stereo,
/// 7.7 dB above the other speakers.
const HEADPHONE_LFE_GAIN: f64 = 7.7;

/// The `@rvpdownmix` graph.
fn downmix(inputs: &Inputs) -> Option<String> {
    let audio = inputs.audio;
    let source = inputs.source;
    let float = "aformat=sample_fmts=flt|fltp";
    match inputs.speakers.mode {
        Mode::Same => None,
        Mode::Layout(name, channels) => {
            let mix = expansion(audio, channels, source).unwrap_or_else(|| {
                format!(
                    "aresample=ochl={name}:{}{}",
                    mix_options(audio, true),
                    front_volume(audio)
                )
            });
            Some(match lfe_redirect(audio, channels) {
                // The redirection's graph starts with a split: the mix goes
                // first under a label.
                Some(redirect) => format!("{float},{mix}[mixed];[mixed]{redirect}"),
                None => format!("{float},{mix}"),
            })
        }
        Mode::Matrix(encoding, lfe) => {
            // PotPlayer's Dolby Surround and Pro Logic II leave the LFE out
            // (goal L's host session); "+LFE" gives it a channel of its own.
            let stereo = format!(
                "aresample=ochl=stereo:matrix_encoding={encoding}:{}{}",
                mix_options(audio, false),
                front_volume(audio)
            );
            if lfe && source.channels.contains(&"LFE") {
                Some(format!(
                    "{float},asplit=2[m][l];[m]{stereo}[st];[l]pan=mono|c0=LFE[lf];\
                     [st][lf]join=inputs=2:channel_layout=2.1:map=0.0-FL|0.1-FR|1.0-LFE"
                ))
            } else {
                Some(format!("{float},{stereo}"))
            }
        }
        Mode::VirtualHeadphone | Mode::VirtualSurround => {
            let path = inputs.responses.head_model.as_ref()?;
            let map = dsp::SPEAKERS
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join("|");
            Some(format!(
                "{float},aresample=ochl=7.1:{},aresample={}[a];amovie=filename={}[h];\
                 [a][h]headphone=map={map}:hrir=multich:gain={HEADPHONE_GAIN:.1}:lfe={HEADPHONE_LFE_GAIN:.1}",
                mix_options(audio, true),
                dsp::RATE,
                graph_path(path)
            ))
        }
        Mode::DolbyDecoder => {
            let delay = audio.virtualizer.delay;
            Some(format!(
                "{float},aresample=ochl=stereo:{},surround=chl_out=5.1,\
                 adelay=delays=0|0|0|0|{delay:.0}|{delay:.0}",
                mix_options(audio, true)
            ))
        }
    }
}

/// The `@rvpchannel` graph: a stereo pair's left or right side on both
/// speakers, the sides swapped, or nothing.
fn channel(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let swap = inputs.audio.swap_channels;
    if (inputs.channel == StereoChannel::Both && !swap)
        || !channels.contains(&"FL")
        || !channels.contains(&"FR")
    {
        return None;
    }
    let (left, right) = match inputs.channel {
        StereoChannel::Both => ("FL", "FR"),
        StereoChannel::Left => ("FL", "FL"),
        StereoChannel::Right => ("FR", "FR"),
    };
    let (left, right) = if swap { (right, left) } else { (left, right) };
    Some(pan(channels, |channel| match channel {
        "FL" => left.to_owned(),
        "FR" => right.to_owned(),
        other => other.to_owned(),
    }))
}

/// The codecs PotPlayer pre-amplifies.
const PREAMP_CODECS: [&str; 3] = ["ac3", "eac3", "dts"];

fn preamp(inputs: &Inputs) -> Option<String> {
    let preamp = &inputs.audio.preamp;
    let codec = inputs.source.codec.as_deref()?;
    (preamp.enabled && PREAMP_CODECS.contains(&codec))
        .then(|| format!("volume={:.2}dB", preamp.gain))
}

/// Voice Emphasis' band: its centre in Hz and its Q.
const VOICE_BAND: f64 = 1000.0;
const VOICE_Q: f64 = 2.9;

/// Voice removal (the centre cancelled: each side less the other) and
/// voice emphasis (the voice's band kept, the rest lowered).
fn voice(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let audio = inputs.audio;
    let mut parts = Vec::new();
    if audio.voice_removal && channels.contains(&"FL") && channels.contains(&"FR") {
        parts.push(pan(channels, |channel| match channel {
            "FL" => "FL-FR".to_owned(),
            "FR" => "FR-FL".to_owned(),
            "FC" => "0*FC".to_owned(),
            other => other.to_owned(),
        }));
    }
    if audio.voice_emphasis {
        // PotPlayer's Voice Emphasis keeps the voice's band: 1 kHz as it
        // was, 500 Hz 13 dB down, 3 kHz 18 dB, 5 kHz 23 dB, white noise
        // 16 dB (goal L's host session): a band-pass at 1 kHz with a Q of
        // 2.9.
        parts.push(format!(
            "bandpass=f={VOICE_BAND:.0}:width_type=q:width={VOICE_Q:.1}"
        ));
    }
    (!parts.is_empty()).then(|| parts.join(","))
}

/// Crystality as PotPlayer's measured in goal L's host session (at its
/// defaults; the other settings scale the parts below):
///
/// * the harmonic booster adds the signal's magnitude, `|x|` (a full-wave
///   rectifier): a tone's even harmonics, 30 dB down for the second at 43;
/// * the bandwidth extender raised every tone 0.6 dB and noise above
///   16 kHz 3 dB more at 28: two high shelves (the extender's HPF, 1 to
///   3, moves the lower one, measured at 3 only);
/// * extra stereo widens the sides (FFmpeg's `extrastereo`);
/// * the echo is [`dsp::crystality_echo`]'s response, convolved on the
///   stereo output.
///
/// All of it at the echo's rate, 48 kHz.
fn crystality(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let c = &inputs.audio.crystality;
    if !c.enabled {
        return None;
    }
    let booster = c.harmonic_booster / 100.0 * HARMONIC_BOOSTER;
    let magnitudes = (0..channels.len())
        .map(|channel| format!("val({channel})+{booster:.5}*abs(val({channel}))"))
        .collect::<Vec<_>>()
        .join("|");
    let extender = c.bandwidth_extender / EXTENDER_DEFAULT;
    // HPF 3 is the measured 300 Hz; each step below it an octave higher.
    let corner = EXTENDER_CORNER * 2f64.powf(3.0 - c.extender_hpf);
    let mut wet = format!(
        "aeval=exprs={magnitudes}:c=same,highpass=f=5:p=1,\
         highshelf=f={corner:.0}:g={:.3},highshelf=f=16000:g={:.3}",
        extender * EXTENDER_GAIN,
        extender * EXTENDER_TOP_GAIN
    );
    let stereo = channels == ["FL", "FR"];
    if stereo && c.extra_stereo > 0.0 {
        // Writing to a String does not fail.
        let _ = write!(
            wet,
            ",extrastereo=m={:.4}",
            1.0 + c.extra_stereo / 100.0 * EXTRA_STEREO
        );
    }
    let echo = inputs
        .responses
        .crystality
        .as_ref()
        .filter(|response| stereo && c.echo_volume > 0.0 && response.levels.len() == 4);
    Some(match echo {
        Some(response) => {
            let l = &response.levels;
            format!(
                "aresample={rate},asplit=2[dry][e];[dry]{wet}[w];\
                 [e]pan=4.0|c0=c0|c1=c1|c2=c0|c3=c1[e4];amovie=filename={file}[ir];[e4][ir]afir[x4];\
                 [x4]pan=stereo|c0={:.6}*c0+{:.6}*c1|c1={:.6}*c2+{:.6}*c3[echo];\
                 [w][echo]amix=inputs=2:normalize=0",
                l[0],
                l[1],
                l[2],
                l[3],
                rate = dsp::RATE,
                file = graph_path(&response.path),
            )
        }
        None => format!("aresample={},{wet}", dsp::RATE),
    })
}

/// The harmonic booster's `|x|` at 100 (0.0745 at PotPlayer's 43).
const HARMONIC_BOOSTER: f64 = 0.173;
/// The bandwidth extender's level where it was measured, its lower
/// shelf's corner (HPF 3) and gain there, and its top shelf's gain, in dB.
const EXTENDER_DEFAULT: f64 = 28.0;
const EXTENDER_CORNER: f64 = 300.0;
const EXTENDER_GAIN: f64 = 0.6;
const EXTENDER_TOP_GAIN: f64 = 3.0;
/// Extra stereo's `extrastereo` factor at 100, less 1: a side tone leaked
/// to the other side 20 dB down at PotPlayer's 11.
const EXTRA_STEREO: f64 = 2.05;

/// Noise reduction as PotPlayer's measured in goal L's host session: 1 kHz
/// and below as they were, 3 kHz 8 dB down, 5 kHz 18 dB, white noise
/// 10 dB, which a low-pass at 1875 Hz with a Q of 0.76 matches within
/// 0.7 dB. That was at PotPlayer's default threshold (100, of 1 to 5000);
/// RVP has no threshold (`crate::audio::NoiseReduction`).
fn denoise(inputs: &Inputs) -> Option<String> {
    let n = &inputs.audio.noise_reduction;
    n.enabled
        .then(|| format!("lowpass=f={DENOISE_CUT:.0}:width_type=q:width={DENOISE_Q:.2}"))
}

/// Noise reduction's low-pass: its corner in Hz and its Q.
const DENOISE_CUT: f64 = 1875.0;
const DENOISE_Q: f64 = 0.76;

/// Freeverb on a stereo output: the sum of both sides through its
/// response (wet), with the response's levels put back, beside the dry
/// sound.
fn reverb(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let reverb = &inputs.audio.freeverb;
    if !reverb.enabled || channels != ["FL", "FR"] {
        return None;
    }
    let response = inputs.responses.freeverb.as_ref()?;
    let wet = reverb.wet * dsp::SCALE_WET;
    Some(format!(
        "aresample={rate},asplit=2[dry][w];[w]pan=stereo|c0=c0+c1|c1=c0+c1[m];\
         amovie=filename={file}[ir];[m][ir]afir[c];\
         [c]pan=stereo|c0={left:.6}*c0|c1={right:.6}*c1[r];\
         [dry]volume={dry:.6}[d];[d][r]amix=inputs=2:normalize=0",
        rate = dsp::RATE,
        file = graph_path(&response.path),
        left = wet * response.levels[0],
        right = wet * response.levels[1],
        dry = reverb.dry * dsp::SCALE_DRY,
    ))
}

/// Crossfeed on a stereo output, bs2b's filters with its formulas: each
/// side gets the other through a one-pole low-pass at the cut frequency,
/// and keeps its own less a low-passed part of it (bs2b's "high boost"),
/// all then raised to make up for the bass the two take away. PotPlayer's
/// is bs2b: at its 700 Hz and 6 dB it matched these within 0.1 dB (goal
/// L's host session).
fn crossfeed(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let c = &inputs.audio.crossfeed;
    if !c.enabled || channels != ["FL", "FR"] {
        return None;
    }
    let low_db = -c.feed * 5.0 / 6.0 - 3.0;
    let high_db = c.feed / 6.0 - 3.0;
    let low = db(low_db);
    let high = 1.0 - db(high_db);
    let high_cut = c.cut * 2f64.powf((low_db - 20.0 * high.log10()) / 12.0);
    let gain = 1.0 / (1.0 - high + low);
    Some(format!(
        "asplit=3[own][part][other];\
         [part]lowpass=f={high_cut:.1}:poles=1,pan=stereo|c0={own:.6}*c0|c1={own:.6}*c1[boost];\
         [other]lowpass=f={cut:.1}:poles=1,pan=stereo|c0={low:.6}*c1|c1={low:.6}*c0[fed];\
         [own][boost][fed]amix=inputs=3:normalize=0,volume={gain:.6}",
        own = -high,
        cut = c.cut,
    ))
}

/// The individual channel volumes.
fn gains(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let volumes = &inputs.audio.channel_volumes;
    if !volumes.enabled || channels.is_empty() {
        return None;
    }
    Some(pan(channels, |channel| {
        format!("{:.6}*{channel}", db(volumes.gain(channel)))
    }))
}

/// The pitch factor of "Audio Stretching" at `percent` from 100.
#[must_use]
pub fn stretching_pitch(percent: i32) -> f64 {
    f64::from(100 + percent) / 100.0
}

/// RVP's filters before the user's, and after them.
#[must_use]
pub fn build(inputs: &Inputs) -> (Vec<Entry>, Vec<Entry>) {
    let channels = output_channels(inputs.speakers, inputs.source);
    let mut front: Vec<Entry> = Vec::new();
    if inputs.spectrum {
        front.push((crate::spectrum::LABEL, crate::spectrum::chain_entry()));
    }
    let mut push = |label: &'static str, graph: Option<String>| {
        if let Some(graph) = graph {
            front.push((label, lavfi(label, &graph)));
        }
    };
    push(crate::speakers::LABEL, downmix(inputs));
    push(CHANNEL, channel(inputs, &channels));
    push(PREAMP, preamp(inputs));
    if inputs.equalizer.enabled {
        front.push((crate::equalizer::LABEL, inputs.equalizer.chain_entry()));
    }
    let mut push = |label: &'static str, graph: Option<String>| {
        if let Some(graph) = graph {
            front.push((label, lavfi(label, &graph)));
        }
    };
    push(VOICE, voice(inputs, &channels));
    push(CRYSTALITY, crystality(inputs, &channels));
    push(DENOISE, denoise(inputs));
    push(REVERB, reverb(inputs, &channels));
    push(CROSSFEED, crossfeed(inputs, &channels));

    let mut back: Vec<Entry> = Vec::new();
    let normalizer = inputs.normalizer;
    let normalize = normalizer.enabled && (inputs.source.video || !normalizer.video_only);
    if normalize {
        back.push((
            crate::normalizer::LABEL,
            normalizer.chain_entry(
                inputs.gain,
                crate::normalizer::block_for(inputs.source.codec.as_deref()),
            ),
        ));
    }
    if let Some(graph) = gains(inputs, &channels) {
        back.push((GAINS, lavfi(GAINS, &graph)));
    }
    // The normalizer needs RVP's tempo filter (crate::normalizer); without
    // it mpv puts in its own scaletempo2 when the speed changes. "Audio
    // Stretching" takes that place with rubberband, which also keeps the
    // speed's pitch.
    if inputs.stretching != 0 {
        back.push((
            crate::normalizer::TEMPO_LABEL,
            format!(
                "@{}:rubberband=pitch-scale={:.4}",
                crate::normalizer::TEMPO_LABEL,
                stretching_pitch(inputs.stretching)
            ),
        ));
    } else {
        match inputs.audio.stretch {
            Stretch::HighQuality if normalize => back.push((
                crate::normalizer::TEMPO_LABEL,
                crate::normalizer::TEMPO_ENTRY.to_owned(),
            )),
            Stretch::DefaultQuality => back.push((
                crate::normalizer::TEMPO_LABEL,
                format!("@{}:scaletempo", crate::normalizer::TEMPO_LABEL),
            )),
            Stretch::HighQuality | Stretch::Pitch => {}
        }
    }
    // "Pass-through after AC3 re-encoding": mpv's AC3 encoder, last.
    if inputs.audio.output_mode == OutputMode::Ac3Encode {
        back.push((
            AC3,
            format!("@{AC3}:lavcac3enc=bitrate={}", inputs.audio.ac3_bitrate),
        ));
    }
    (front, back)
}

/// The filters of an mpv `af` or `vf` value (`@label:name=...,...`), each
/// with its label and its text. Values may be quoted as `%N%` followed by
/// N bytes, or in brackets; a count past the end ends the text (it may come
/// from a label, which mpv prints as it was given).
#[must_use]
pub fn split(text: &str) -> Vec<(Option<&str>, &str)> {
    let bytes = text.as_bytes();
    let mut entries = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i <= bytes.len() {
        if i == bytes.len() || bytes[i] == b',' {
            let entry = &text[start..i];
            let label = entry
                .strip_prefix('@')
                .and_then(|e| e.split_once(':'))
                .map(|(label, _)| label);
            if !entry.is_empty() {
                entries.push((label, entry));
            }
            start = i + 1;
            i += 1;
        } else if bytes[i] == b'%'
            && let Some(len) = text[i + 1..].find('%')
            && let Ok(count) = text[i + 1..i + 1 + len].parse::<usize>()
        {
            i = i
                .saturating_add(len + 2)
                .saturating_add(count)
                .min(bytes.len());
        } else if bytes[i] == b'[' {
            i = text[i..].find(']').map_or(bytes.len(), |end| i + end + 1);
        } else {
            i += 1;
        }
    }
    entries
}

/// Whether a label is one of RVP's (`@rvp...`).
#[must_use]
pub fn is_rvps(label: Option<&str>) -> bool {
    label.is_some_and(|label| label.starts_with("rvp"))
}

/// The whole `af` value: RVP's front filters, the user's from `current`,
/// RVP's back filters.
#[must_use]
pub fn assemble(current: &str, front: &[Entry], back: &[Entry]) -> String {
    let user = split(current)
        .into_iter()
        .filter(|(label, _)| !is_rvps(*label))
        .map(|(_, entry)| entry.to_owned());
    front
        .iter()
        .map(|(_, entry)| entry.clone())
        .chain(user)
        .chain(back.iter().map(|(_, entry)| entry.clone()))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speakers::choice;

    fn inputs<'a>(
        audio: &'a Audio,
        speakers: &'a str,
        source: &'a Source,
        responses: &'a Responses,
        equalizer: &'a Equalizer,
        normalizer: &'a Normalizer,
    ) -> Inputs<'a> {
        Inputs {
            audio,
            speakers: choice(speakers).expect("a choice"),
            equalizer,
            normalizer,
            channel: StereoChannel::Both,
            stretching: 0,
            source,
            gain: 1.0,
            responses,
            spectrum: false,
        }
    }

    fn labels(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|(label, _)| *label).collect()
    }

    #[test]
    fn the_default_chain_is_potplayers_mix_and_the_normalizer() {
        let audio = Audio::default();
        let source = Source {
            video: true,
            ..Source::default()
        };
        let (eq, norm, responses) = (
            Equalizer::default(),
            Normalizer::default(),
            Responses::default(),
        );
        let (front, back) = build(&inputs(&audio, "stereo", &source, &responses, &eq, &norm));
        assert_eq!(labels(&front), ["rvpdownmix"]);
        assert_eq!(labels(&back), ["rvpnorm", "rvptempo"]);
        assert!(front[0].1.contains(
            "aresample=ochl=stereo:clev=0.707107:slev=1.000000:lfe_mix_level=1.000000:\
             rematrix_maxval=1000"
        ));
        // The audit's mix: unchanged from M8's.
        assert!(front[0].1.starts_with("@rvpdownmix:lavfi=graph=%"));
    }

    #[test]
    fn every_stage_in_its_order() {
        let mut audio = Audio {
            swap_channels: true,
            voice_removal: true,
            stretch: Stretch::DefaultQuality,
            ..Audio::default()
        };
        audio.preamp.enabled = true;
        audio.crystality.enabled = true;
        audio.noise_reduction.enabled = true;
        audio.freeverb.enabled = true;
        audio.crossfeed.enabled = true;
        audio.channel_volumes.enabled = true;
        let source = Source {
            codec: Some("ac3".to_owned()),
            channels: vec!["FL", "FR", "FC", "LFE", "SL", "SR"],
            video: true,
        };
        let responses = Responses {
            freeverb: Some(dsp::Response {
                path: PathBuf::from("/tmp/fv.wav"),
                levels: vec![20.0, 20.0],
            }),
            crystality: None,
            head_model: None,
        };
        let eq = Equalizer {
            enabled: true,
            ..Equalizer::default()
        };
        let norm = Normalizer::default();
        let (front, back) = build(&inputs(&audio, "stereo", &source, &responses, &eq, &norm));
        assert_eq!(
            labels(&front),
            [
                "rvpdownmix",
                "rvpchannel",
                "rvppreamp",
                "rvpeq",
                "rvpvoice",
                "rvpcrystality",
                "rvpdenoise",
                "rvpreverb",
                "rvpcrossfeed"
            ]
        );
        assert_eq!(labels(&back), ["rvpnorm", "rvpgains", "rvptempo"]);
        assert!(back[2].1.ends_with(":scaletempo"));
        // AC3 is pre-amplified, FLAC is not.
        let flac = Source {
            codec: Some("flac".to_owned()),
            ..source.clone()
        };
        let (front, _) = build(&inputs(&audio, "stereo", &flac, &responses, &eq, &norm));
        assert!(!labels(&front).contains(&"rvppreamp"));
    }

    #[test]
    fn layouts_matrices_and_virtual_speakers() {
        let audio = Audio::default();
        let (eq, norm) = (Equalizer::default(), Normalizer::default());
        let surround = Source {
            channels: vec!["FL", "FR", "FC", "LFE", "SL", "SR"],
            ..Source::default()
        };
        let none = Responses::default();
        let graph = |speakers: &str, responses: &Responses| {
            build(&inputs(&audio, speakers, &surround, responses, &eq, &norm))
                .0
                .first()
                .map(|(_, entry)| entry.clone())
                .unwrap_or_default()
        };
        assert!(graph("5.1", &none).contains("aresample=ochl=5.1:"));
        assert!(graph("3.1-surround", &none).contains("ochl=FL+FR+LFE+BC:"));
        assert!(graph("dplii", &none).contains("matrix_encoding=dplii"));
        assert!(graph("dolby-lfe", &none).contains("join=inputs=2:channel_layout=2.1"));
        assert!(graph("dolby-decoder", &none).contains("surround=chl_out=5.1"));
        assert_eq!(graph("same", &none), "");
        // The virtual speakers need their responses.
        assert_eq!(graph("virtual-headphone", &none), "");
        let (path, quoted) = if cfg!(windows) {
            ("C:\\Users\\a b\\head.wav", "'C\\:/Users/a b/head.wav'")
        } else {
            ("/home/a b/head.wav", "'/home/a b/head.wav'")
        };
        let responses = Responses {
            freeverb: None,
            crystality: None,
            head_model: Some(PathBuf::from(path)),
        };
        let virtual_graph = graph("virtual-headphone", &responses);
        assert!(virtual_graph.contains(&format!("amovie=filename={quoted}[h]")));
        assert!(virtual_graph.contains("headphone=map=FL|FR|FC|LFE|BL|BR|SL|SR:hrir=multich"));
    }

    #[test]
    fn stereo_spreads_and_the_lfe_takes_the_bass() {
        let mut audio = Audio::default();
        audio.mixer.expand_center = true;
        audio.mixer.expand_surround = true;
        audio.lfe_redirect.enabled = true;
        let (eq, norm, responses) = (
            Equalizer::default(),
            Normalizer::default(),
            Responses::default(),
        );
        let stereo = Source {
            channels: vec!["FL", "FR"],
            ..Source::default()
        };
        let (front, _) = build(&inputs(&audio, "5.1", &stereo, &responses, &eq, &norm));
        let graph = &front[0].1;
        assert!(graph.contains("FC=0.353553*c0+0.353553*c1"), "{graph}");
        assert!(graph.contains("BL=1.000000*c0"), "{graph}");
        assert!(graph.contains("acrossover=split=80"), "{graph}");
        assert!(graph.contains("anullsink"), "{graph}");
    }

    #[test]
    fn channels_select_and_swap() {
        let mut audio = Audio::default();
        let (eq, norm, responses) = (
            Equalizer::default(),
            Normalizer::default(),
            Responses::default(),
        );
        let source = Source::default();
        let mut i = inputs(&audio, "stereo", &source, &responses, &eq, &norm);
        assert_eq!(channel(&i, &["FL", "FR"]), None);
        i.channel = StereoChannel::Left;
        assert_eq!(
            channel(&i, &["FL", "FR"]).as_deref(),
            Some("pan=FL+FR|FL=FL|FR=FL")
        );
        audio.swap_channels = true;
        let mut i = inputs(&audio, "stereo", &source, &responses, &eq, &norm);
        assert_eq!(
            channel(&i, &["FL", "FR"]).as_deref(),
            Some("pan=FL+FR|FL=FR|FR=FL")
        );
        i.channel = StereoChannel::Right;
        assert_eq!(
            channel(&i, &["FL", "FR", "LFE"]).as_deref(),
            Some("pan=FL+FR+LFE|FL=FR|FR=FR|LFE=LFE")
        );
    }

    #[test]
    fn the_users_filters_stay_between_rvps() {
        let current = "@rvpdownmix:lavfi=graph=%3%a,b,@mine:lavfi=graph=%8%volume=2,\
                       @rvpnorm:lavfi=[volume=1],@other:scaletempo";
        let entries = split(current);
        assert_eq!(entries.len(), 4, "{entries:?}");
        assert_eq!(entries[1], (Some("mine"), "@mine:lavfi=graph=%8%volume=2"));
        let front = vec![("rvpeq", "@rvpeq:lavfi=graph=%1%x".to_owned())];
        let back = vec![("rvpnorm", "@rvpnorm:lavfi=graph=%1%y".to_owned())];
        assert_eq!(
            assemble(current, &front, &back),
            "@rvpeq:lavfi=graph=%1%x,@mine:lavfi=graph=%8%volume=2,@other:scaletempo,\
             @rvpnorm:lavfi=graph=%1%y"
        );
    }

    /// FFmpeg's `av_get_token` (libavutil/avstring.c), which both the
    /// graph's parser and the filter options' use: white space skipped
    /// before, `\` takes the next character as it is, `'` quotes up to the
    /// next `'`, unquoted white space dropped at the end; it stops at a
    /// character of `term`. Returns the token and the text after it.
    fn get_token<'a>(text: &'a str, term: &str) -> (String, &'a str) {
        const WHITE: [char; 4] = [' ', '\n', '\t', '\r'];
        let text = text.trim_start_matches(WHITE);
        let mut chars = text.char_indices().peekable();
        let (mut out, mut kept) = (Vec::new(), 0);
        let mut rest = text.len();
        while let Some((at, c)) = chars.next() {
            if term.contains(c) {
                rest = at;
                break;
            }
            if c == '\\' && chars.peek().is_some() {
                out.extend(chars.next().map(|(_, c)| c));
                kept = out.len();
            } else if c == '\'' {
                let mut closed = false;
                for (_, c) in chars.by_ref() {
                    if c == '\'' {
                        closed = true;
                        break;
                    }
                    out.push(c);
                }
                if closed {
                    kept = out.len();
                }
            } else {
                out.push(c);
            }
        }
        while out.len() > kept && out.last().is_some_and(|c| WHITE.contains(c)) {
            out.pop();
        }
        (out.into_iter().collect(), &text[rest..])
    }

    #[test]
    fn graph_paths_come_through_both_of_ffmpegs_parsers() {
        for path in [
            "/home/it's/freeverb.wav",
            "/home/a: b's 'x':/freeverb.wav",
            "/home/''/[x],y;z/freeverb.wav",
            "/home/back\\slash\\'/freeverb.wav",
            "C:\\Users\\O'Brien\\AppData\\Local\\RVP\\audio\\freeverb.wav",
        ] {
            let graph = format!("filename={}[ir];[m][ir]afir", graph_path(Path::new(path)));
            // The graph's parser: the filter's options, up to its outputs.
            let (options, after) = get_token(&graph, "[],;");
            assert_eq!(after, "[ir];[m][ir]afir", "{path}");
            // The options' parser: one value, the whole name.
            let value = options.strip_prefix("filename=").expect("the option");
            let (name, after) = get_token(value, ":");
            let wanted = if cfg!(windows) {
                path.replace('\\', "/")
            } else {
                path.to_owned()
            };
            assert_eq!(name, wanted, "{graph}");
            assert_eq!(after, "", "{path}: one option");
        }
    }

    #[test]
    fn a_count_past_the_end_ends_the_text() {
        // mpv prints a label as it was given, `%` and all.
        let entries = split("@%18446744073709551594%:lavfi=[anull],@b:x");
        assert_eq!(entries.len(), 1, "{entries:?}");
        assert_eq!(split("%99%abc"), [(None, "%99%abc")]);
    }

    #[test]
    fn the_normalizer_may_wait_for_video() {
        let audio = Audio::default();
        let (eq, responses) = (Equalizer::default(), Responses::default());
        let norm = Normalizer {
            video_only: true,
            ..Normalizer::default()
        };
        let music = Source::default();
        let (_, back) = build(&inputs(&audio, "stereo", &music, &responses, &eq, &norm));
        assert!(back.is_empty(), "{back:?}");
        let film = Source {
            video: true,
            ..Source::default()
        };
        let (_, back) = build(&inputs(&audio, "stereo", &film, &responses, &eq, &norm));
        assert_eq!(labels(&back), ["rvpnorm", "rvptempo"]);
    }
}
