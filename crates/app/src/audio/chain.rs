//! RVP's part of mpv's audio filters (M10), built at once from the
//! settings and the file that plays, in this order, each under its label:
//!
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
//! PotPlayer's effects are its own code; where FFmpeg has no equal RVP
//! builds them from FFmpeg's filters (Crystality, noise reduction,
//! crossfeed), and the audio audit compares the results with PotPlayer's.

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
    pub source: &'a Source,
    /// The normalizer's current gain.
    pub gain: f64,
    pub responses: &'a Responses,
}

/// One of RVP's filters: its label and its `af` entry.
pub type Entry = (&'static str, String);

/// An `af` entry for a libavfilter graph.
#[must_use]
pub fn lavfi(label: &str, graph: &str) -> String {
    format!("@{label}:lavfi=graph=%{}%{graph}", graph.len())
}

/// A file name for a graph's `amovie`: forward slashes, `:` escaped for the
/// filter's options, quoted for the graph.
#[must_use]
pub fn graph_path(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    let escaped = text.replace(':', "\\:");
    format!("'{}'", escaped.replace('\'', "'\\''"))
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
/// surround channels and the LFE, relative to the front, which the mix
/// then gets through a `volume`.
fn mix_options(audio: &Audio) -> String {
    let mixer = &audio.mixer;
    let front = db(mixer.front);
    // swresample gives each side the LFE at -3 dB with lfe_mix_level 1.
    let lfe = db(mixer.lfe + 3.0103);
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
/// eight speakers of 7.1, less 3 dB of headroom for all of them together.
const HEADPHONE_GAIN: i32 = 3 * 8 - 3;

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
                    mix_options(audio),
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
            let stereo = format!(
                "aresample=ochl=stereo:matrix_encoding={encoding}:{}{}",
                mix_options(audio),
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
                 [a][h]headphone=map={map}:hrir=multich:gain={HEADPHONE_GAIN}",
                mix_options(audio),
                dsp::RATE,
                graph_path(path)
            ))
        }
        Mode::DolbyDecoder => {
            let delay = audio.virtualizer.delay;
            Some(format!(
                "{float},aresample=ochl=stereo:{},surround=chl_out=5.1,\
                 adelay=delays=0|0|0|0|{delay:.0}|{delay:.0}",
                mix_options(audio)
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

/// Voice removal (the centre cancelled: each side less the other) and
/// voice emphasis (the voice band raised).
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
        parts.push("equalizer=f=1500:t=o:w=2:g=6".to_owned());
    }
    (!parts.is_empty()).then(|| parts.join(","))
}

/// Crystality from FFmpeg's filters: the bandwidth extender and its
/// high-pass as `aexciter`, the harmonic booster as `crystalizer`, the
/// echo as `aecho`, extra stereo (stereo only) as `extrastereo`.
fn crystality(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let c = &inputs.audio.crystality;
    if !c.enabled {
        return None;
    }
    let mut parts = vec![
        format!(
            "aexciter=amount={:.3}:drive=8.5:freq={:.0}:blend=0",
            (c.bandwidth_extender / 100.0 * 3.0).max(0.001),
            (2000.0 + c.extender_hpf * 100.0).min(12_000.0)
        ),
        format!("crystalizer=i={:.3}", c.harmonic_booster / 100.0 * 4.0),
    ];
    if c.echo_volume > 0.0 {
        let echo = c.echo_volume / 100.0;
        let feedback = c.echo_feedback / 100.0;
        parts.push(format!(
            "aecho=in_gain=1:out_gain={:.4}:delays=20|40:decays={:.4}|{:.4}",
            1.0 / (1.0 + echo + echo * feedback),
            echo,
            (echo * feedback).max(0.0001)
        ));
    }
    if c.extra_stereo > 0.0 && channels == ["FL", "FR"] {
        parts.push(format!(
            "extrastereo=m={:.3}",
            1.0 + c.extra_stereo / 100.0 * 2.5
        ));
    }
    Some(parts.join(","))
}

/// Noise reduction: FFmpeg's FFT denoiser with its noise floor from the
/// threshold.
fn denoise(inputs: &Inputs) -> Option<String> {
    let n = &inputs.audio.noise_reduction;
    n.enabled.then(|| {
        format!(
            "afftdn=nr=12:nf={:.1}:tn=1",
            (-80.0 + n.threshold * 0.6).min(-20.0)
        )
    })
}

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

/// Crossfeed on a stereo output, FFmpeg's `crossfeed` set from the cut
/// frequency and feed level.
fn crossfeed(inputs: &Inputs, channels: &[&str]) -> Option<String> {
    let c = &inputs.audio.crossfeed;
    (c.enabled && channels == ["FL", "FR"]).then(|| {
        format!(
            "crossfeed=strength={:.3}:range={:.3}",
            (c.feed / 15.0).clamp(0.0, 1.0),
            ((c.cut - 300.0) / 1700.0).clamp(0.0, 1.0)
        )
    })
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

/// RVP's filters before the user's, and after them.
#[must_use]
pub fn build(inputs: &Inputs) -> (Vec<Entry>, Vec<Entry>) {
    let channels = output_channels(inputs.speakers, inputs.source);
    let mut front: Vec<Entry> = Vec::new();
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
    // it mpv puts in its own scaletempo2 when the speed changes.
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
    // "Pass-through after AC3 re-encoding": mpv's AC3 encoder, last.
    if inputs.audio.output_mode == OutputMode::Ac3Encode {
        back.push((AC3, format!("@{AC3}:lavcac3enc")));
    }
    (front, back)
}

/// The filters of an mpv `af` or `vf` value (`@label:name=...,...`), each
/// with its label and its text. Values may be quoted as `%N%` followed by
/// N bytes, or in brackets.
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
            i = (i + len + 2 + count).min(bytes.len());
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
            source,
            gain: 1.0,
            responses,
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
                levels: [20.0, 20.0],
            }),
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
        let responses = Responses {
            freeverb: None,
            head_model: Some(PathBuf::from("C:\\Users\\a b\\head.wav")),
        };
        let virtual_graph = graph("virtual-headphone", &responses);
        assert!(virtual_graph.contains("amovie=filename='C\\:/Users/a b/head.wav'"));
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
