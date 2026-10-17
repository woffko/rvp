//! M10's audio chain on a real player (headless libmpv, `ao=pcm`: no
//! window, no sound): each stage, built by `rvp_app::audio::chain`, does
//! to synthetic tones what PotPlayer's function does (levels per channel),
//! and every effect's graph runs.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::audio::chain::{Inputs, Responses, Source, assemble, build};
use rvp_app::audio::{Audio, StereoChannel, dsp};
use rvp_app::equalizer::Equalizer;
use rvp_app::normalizer::Normalizer;
use rvp_app::speakers::choice;
use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel};

const RATE: u32 = 48_000;
/// Silence before and after the tones: mpv may cut a little of either end.
const PAD: u32 = RATE;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-chain-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A 32-bit float WAV of `channels` channels, `frames` long after a pad,
/// each sample from `sample(frame, channel)`, a pad after.
fn write_wav(path: &Path, channels: u16, frames: u32, sample: impl Fn(u32, u16) -> f32) {
    let total = PAD + frames + PAD;
    let data_len = total * u32::from(channels) * 4;
    let extensible = channels > 2;
    let fmt_len: u32 = if extensible { 40 } else { 16 };
    let mut bytes = Vec::with_capacity(60 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(20 + fmt_len + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&fmt_len.to_le_bytes());
    bytes.extend_from_slice(&(if extensible { 0xFFFEu16 } else { 3 }).to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(channels) * 4).to_le_bytes());
    bytes.extend_from_slice(&(channels * 4).to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    if extensible {
        bytes.extend_from_slice(&22u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        // FL FR FC LFE SL SR.
        bytes.extend_from_slice(&0x60Fu32.to_le_bytes());
        // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.
        bytes.extend_from_slice(&[
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38,
            0x9B, 0x71,
        ]);
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for n in 0..total {
        for channel in 0..channels {
            let value = if (PAD..PAD + frames).contains(&n) {
                sample(n - PAD, channel)
            } else {
                0.0
            };
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    std::fs::write(path, bytes).expect("write the source");
}

fn tone(frequency: f64, amplitude: f64, n: u32) -> f32 {
    let phase = f64::from(n) * frequency * std::f64::consts::TAU / f64::from(RATE);
    // A sine below full scale fits f32.
    #[allow(clippy::cast_possible_truncation)]
    let value = (amplitude * phase.sin()) as f32;
    value
}

/// The frames of mpv's float output, per channel.
fn read_output(path: &Path) -> Vec<Vec<f64>> {
    let bytes = std::fs::read(path).expect("read output");
    let mut at = 12;
    let mut channels = 0usize;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("len")) as usize;
        let body = at + 8;
        if id == b"fmt " {
            channels = usize::from(u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]));
        } else if id == b"data" {
            let end = if len == 0 || len == usize::MAX {
                bytes.len()
            } else {
                bytes.len().min(body.saturating_add(len))
            };
            let floats: Vec<f64> = bytes[body..end]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f64::from(f32::from_le_bytes(*b)))
                .collect();
            return (0..channels)
                .map(|c| floats.iter().skip(c).step_by(channels).copied().collect())
                .collect();
        }
        at = body + len + (len & 1);
    }
    panic!("no data in {}", path.display());
}

/// Settings for one render: the chain's inputs.
struct Case<'a> {
    audio: Audio,
    speakers: &'a str,
    source: Source,
    channel: StereoChannel,
    equalizer: Equalizer,
    responses: Responses,
}

impl<'a> Case<'a> {
    fn new(speakers: &'a str, channels: Vec<&'static str>) -> Case<'a> {
        Case {
            audio: Audio::default(),
            speakers,
            source: Source {
                channels,
                ..Source::default()
            },
            channel: StereoChannel::Both,
            equalizer: Equalizer::default(),
            responses: Responses::default(),
        }
    }

    /// The `af` value: RVP's chain without the normalizer (it would level
    /// what the tests measure).
    fn af(&self) -> String {
        let normalizer = Normalizer {
            enabled: false,
            ..Normalizer::default()
        };
        let inputs = Inputs {
            audio: &self.audio,
            speakers: choice(self.speakers).expect("speakers"),
            equalizer: &self.equalizer,
            normalizer: &normalizer,
            channel: self.channel,
            source: &self.source,
            gain: 1.0,
            responses: &self.responses,
        };
        let (front, back) = build(&inputs);
        assemble("", &front, &back)
    }

    /// Renders `input` through the chain; per output channel.
    fn render(&self, dir: &Path, input: &Path, name: &str) -> Vec<Vec<f64>> {
        let output = dir.join(format!("out-{name}.wav"));
        let speakers = choice(self.speakers).expect("speakers");
        let mut options = EngineOptions::headless();
        options.extra = vec![
            ("keep-open".to_owned(), "no".to_owned()),
            ("ao".to_owned(), "pcm".to_owned()),
            ("ao-pcm-file".to_owned(), output.display().to_string()),
            ("audio-format".to_owned(), "float".to_owned()),
            ("audio-samplerate".to_owned(), RATE.to_string()),
            ("audio-channels".to_owned(), speakers.audio_channels()),
            ("vid".to_owned(), "no".to_owned()),
        ];
        let engine = Engine::new(&options).expect("libmpv starts");
        engine.set_text("af", &self.af()).expect("af");
        engine
            .load(&input.display().to_string(), false)
            .expect("loadfile");
        engine.set_pause(false).expect("play");
        let mut model = PlayerModel::new();
        let mut events = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(60);
        'playing: loop {
            engine.wait_and_drain(0.05, &mut model, &mut events);
            for event in events.drain(..) {
                match event {
                    EngineEvent::EndFile {
                        reason: EndReason::Eof,
                        ..
                    } => break 'playing,
                    EngineEvent::EndFile {
                        reason: EndReason::Error,
                        ..
                    } => panic!("{name}: playback failed"),
                    _ => {}
                }
            }
            assert!(Instant::now() < deadline, "{name}: playback did not end");
        }
        engine.quit().expect("quit");
        drop(engine);
        read_output(&output)
    }
}

/// RMS in dB of a channel between `from` and `to` seconds after its first
/// sound (any channel's), the edges of the tone left out.
fn level(channels: &[Vec<f64>], channel: usize, from: f64, to: f64) -> f64 {
    let onset = (0..channels[0].len())
        .find(|&n| channels.iter().any(|c| c[n].abs() > 1e-3))
        .expect("some sound");
    // Seconds within a few minutes.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (start, end) = (
        onset + (from * f64::from(RATE)) as usize,
        onset + (to * f64::from(RATE)) as usize,
    );
    let part =
        &channels[channel][start.min(channels[channel].len())..end.min(channels[channel].len())];
    // Sample counts are far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let mean = part.iter().map(|s| s * s).sum::<f64>() / part.len().max(1) as f64;
    10.0 * mean.max(1e-20).log10()
}

/// A sine's RMS at amplitude 0.5: -9.03 dBFS.
const HALF: f64 = -9.0309;

#[test]
fn channels_select_and_swap() {
    let dir = scratch("channels");
    let input = dir.join("left.wav");
    // A tone in the left channel only.
    write_wav(&input, 2, RATE * 2, |n, c| {
        if c == 0 { tone(1000.0, 0.5, n) } else { 0.0 }
    });
    let mut case = Case::new("stereo", vec!["FL", "FR"]);
    case.channel = StereoChannel::Left;
    let both_left = case.render(&dir, &input, "left");
    assert!((level(&both_left, 0, 0.2, 1.8) - HALF).abs() < 0.2);
    assert!(
        (level(&both_left, 1, 0.2, 1.8) - HALF).abs() < 0.2,
        "the left side on both"
    );
    case.channel = StereoChannel::Both;
    case.audio.swap_channels = true;
    let swapped = case.render(&dir, &input, "swap");
    assert!(level(&swapped, 0, 0.2, 1.8) < -80.0, "left now silent");
    assert!(
        (level(&swapped, 1, 0.2, 1.8) - HALF).abs() < 0.2,
        "on the right"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn voice_removal_cancels_the_centre_and_keeps_the_sides() {
    let dir = scratch("voice");
    let input = dir.join("centre.wav");
    // A centred tone, then a tone in the left side only.
    write_wav(&input, 2, RATE * 4, |n, c| {
        if n < RATE * 2 {
            tone(1000.0, 0.5, n)
        } else if c == 0 {
            tone(500.0, 0.5, n)
        } else {
            0.0
        }
    });
    let mut case = Case::new("stereo", vec!["FL", "FR"]);
    case.audio.voice_removal = true;
    let out = case.render(&dir, &input, "removal");
    // The onset is the side tone's (the centred one cancels).
    let side = level(&out, 0, 0.2, 1.8);
    assert!((side - HALF).abs() < 0.3, "the side tone stays: {side}");
    let centre_left = {
        let start = PAD as usize + (RATE as usize / 5);
        let part = &out[0][start..start + RATE as usize];
        // Sample counts are far below 2^52.
        #[allow(clippy::cast_precision_loss)]
        let mean = part.iter().map(|s| s * s).sum::<f64>() / part.len() as f64;
        10.0 * mean.max(1e-20).log10()
    };
    assert!(centre_left < -60.0, "the centre cancels: {centre_left}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ac3_and_dts_are_pre_amplified() {
    let dir = scratch("preamp");
    let input = dir.join("tone.wav");
    write_wav(&input, 2, RATE * 2, |n, _| tone(1000.0, 0.25, n));
    let mut case = Case::new("stereo", vec!["FL", "FR"]);
    case.audio.preamp.enabled = true;
    case.source.codec = Some("flac".to_owned());
    let flac = level(&case.render(&dir, &input, "flac"), 0, 0.2, 1.8);
    case.source.codec = Some("ac3".to_owned());
    let ac3 = level(&case.render(&dir, &input, "ac3"), 0, 0.2, 1.8);
    assert!(
        (ac3 - flac - 6.0).abs() < 0.2,
        "AC3 {ac3} against FLAC {flac}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn freeverb_rings_after_the_sound() {
    let dir = scratch("freeverb");
    let input = dir.join("burst.wav");
    // A tenth of a second, then two seconds of silence.
    write_wav(&input, 2, RATE * 2, |n, _| {
        if n < RATE / 10 {
            tone(800.0, 0.5, n)
        } else {
            0.0
        }
    });
    let mut case = Case::new("stereo", vec!["FL", "FR"]);
    let dry = case.render(&dir, &input, "dry");
    case.audio.freeverb.enabled = true;
    case.responses.freeverb =
        Some(dsp::freeverb_file(&dir, 0.5, 0.25, 1.0).expect("Freeverb's response"));
    let wet = case.render(&dir, &input, "wet");
    let tail_dry = level(&dry, 0, 0.3, 0.8);
    let tail_wet = level(&wet, 0, 0.3, 0.8);
    assert!(tail_dry < -80.0, "no tail dry: {tail_dry}");
    assert!(tail_wet > -60.0, "a reverb tail: {tail_wet}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A 5.1 source with a tone in channel `which` (FL FR FC LFE SL SR).
fn surround(path: &Path, which: u16, frequency: f64) {
    write_wav(path, 6, RATE * 2, |n, c| {
        if c == which {
            tone(frequency, 0.5, n)
        } else {
            0.0
        }
    });
}

#[test]
fn the_virtual_headphone_puts_the_front_left_speaker_to_the_left() {
    let dir = scratch("headphone");
    let input = dir.join("front-left.wav");
    // The head shadows high sounds most.
    surround(&input, 0, 4000.0);
    let mut case = Case::new(
        "virtual-headphone",
        vec!["FL", "FR", "FC", "LFE", "SL", "SR"],
    );
    case.responses.head_model = Some(dsp::head_model_file(&dir, 0.0).expect("head model"));
    let out = case.render(&dir, &input, "headphone");
    let (left, right) = (level(&out, 0, 0.2, 1.8), level(&out, 1, 0.2, 1.8));
    assert!(left > right + 6.0, "near ear {left} dB, far ear {right} dB");
    // The speaker's level, less the 3 dB of headroom, plus the near ear's
    // gain (about 2.4 dB at 4 kHz).
    assert!(
        (left - (HALF - 3.0 + 2.4)).abs() < 1.5,
        "near ear {left} dB"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pro_logic_ii_puts_the_surround_in_both_sides() {
    let dir = scratch("dplii");
    let input = dir.join("surround-left.wav");
    surround(&input, 4, 1000.0);
    let plain = Case::new("stereo", vec!["FL", "FR", "FC", "LFE", "SL", "SR"]);
    let stereo = plain.render(&dir, &input, "stereo");
    assert!(
        level(&stereo, 1, 0.2, 1.8) < -80.0,
        "plain stereo: SL only left"
    );
    let matrix = Case::new("dplii", vec!["FL", "FR", "FC", "LFE", "SL", "SR"]);
    let encoded = matrix.render(&dir, &input, "dplii");
    assert!(
        level(&encoded, 1, 0.2, 1.8) > -30.0,
        "matrix-encoded: some on the right ({})",
        level(&encoded, 1, 0.2, 1.8)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stereo_spreads_to_the_centre_and_the_lfe_takes_the_bass() {
    let dir = scratch("spread");
    let input = dir.join("stereo.wav");
    write_wav(&input, 2, RATE * 2, |n, _| tone(40.0, 0.5, n));
    let mut case = Case::new("5.1", vec!["FL", "FR"]);
    case.audio.mixer.expand_center = true;
    case.audio.lfe_redirect.enabled = true;
    case.audio.lfe_redirect.remove_low = true;
    let out = case.render(&dir, &input, "spread");
    // 5.1: FL FR FC LFE BL BR.
    let (front, centre, lfe) = (
        level(&out, 0, 0.3, 1.7),
        level(&out, 2, 0.3, 1.7),
        level(&out, 3, 0.3, 1.7),
    );
    assert!(lfe > -15.0, "the bass in the LFE: {lfe}");
    assert!(
        front < lfe - 10.0,
        "taken out of the front: {front} against {lfe}"
    );
    assert!(centre < lfe - 10.0, "and of the centre: {centre}");
    // At 1 kHz the centre plays the spread stereo.
    let input = dir.join("mid.wav");
    write_wav(&input, 2, RATE * 2, |n, _| tone(1000.0, 0.5, n));
    case.audio.lfe_redirect.enabled = false;
    let out = case.render(&dir, &input, "centre");
    assert!(level(&out, 2, 0.2, 1.8) > -15.0, "the centre plays");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn individual_volume_changes_one_channel() {
    let dir = scratch("gains");
    let input = dir.join("tone.wav");
    write_wav(&input, 2, RATE * 2, |n, _| tone(1000.0, 0.5, n));
    let mut case = Case::new("stereo", vec!["FL", "FR"]);
    case.audio.channel_volumes.enabled = true;
    case.audio.channel_volumes.left = -6.0;
    let out = case.render(&dir, &input, "gains");
    let (left, right) = (level(&out, 0, 0.2, 1.8), level(&out, 1, 0.2, 1.8));
    assert!(
        (right - left - 6.0).abs() < 0.2,
        "left {left} right {right}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Turns one effect on.
type Switch = fn(&mut Case<'_>);

#[test]
fn every_effect_plays() {
    let dir = scratch("effects");
    let input = dir.join("music.wav");
    // Two tones, unequal sides, a little noise.
    write_wav(&input, 2, RATE * 2, |n, c| {
        let noise =
            f32::from(u16::try_from((n * 7919 + u32::from(c) * 104_729) % 1000).expect("small"))
                / 1e5;
        tone(440.0, 0.3, n) + tone(if c == 0 { 3000.0 } else { 5000.0 }, 0.1, n) + noise
    });
    let plain = Case::new("stereo", vec!["FL", "FR"]);
    let reference = level(&plain.render(&dir, &input, "plain"), 0, 0.2, 1.8);
    let effects: [(&str, Switch); 6] = [
        ("crystality", |c| c.audio.crystality.enabled = true),
        ("noise-reduction", |c| {
            c.audio.noise_reduction.enabled = true;
        }),
        ("crossfeed", |c| c.audio.crossfeed.enabled = true),
        ("voice-emphasis", |c| c.audio.voice_emphasis = true),
        ("superequalizer", |c| {
            c.equalizer.enabled = true;
            c.equalizer.superequalizer = true;
            c.equalizer.bands = [3.0; 10];
        }),
        ("dolby-decoder", |c| c.speakers = "dolby-decoder"),
    ];
    for (name, switch) in effects {
        let mut case = Case::new("stereo", vec!["FL", "FR"]);
        switch(&mut case);
        let out = case.render(&dir, &input, name);
        let got = level(&out, 0, 0.2, 1.8);
        assert!(
            (got - reference).abs() < 12.0,
            "{name}: {got} dB against {reference} dB (chain {})",
            case.af()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
