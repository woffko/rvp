//! The output speakers' downmix (`rvp_app::speakers`) on a synthetic 5.1
//! file, headless (`ao=pcm`: no window, no sound): stereo is PotPlayer's
//! mix (front and surround channels at their level, centre and LFE at
//! -3 dB, not scaled down), where mpv's own mix puts the surround
//! channels at -3 dB and drops the LFE.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::audio::Audio;
use rvp_app::audio::chain::{Inputs, Responses, Source, build};
use rvp_app::equalizer::Equalizer;
use rvp_app::normalizer::Normalizer;
use rvp_app::speakers::{LABEL, choice};
use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel};

const RATE: u32 = 48_000;
/// Channels of the source, in WAV order: FL FR FC LFE SL SR.
const CHANNELS: usize = 6;
/// After a second of silence each channel plays alone for a second, then
/// half a second is silent; the file ends with a second and a half of
/// silence. (mpv may cut up to a second of what `ao=pcm` writes; the
/// silences keep that off the bursts.)
const HEAD: u32 = RATE;
const BURST: u32 = RATE;
const GAP: u32 = RATE / 2;
const TAIL: u32 = RATE * 3 / 2;
/// The tones' peak: -6 dBFS, so that no mix clips.
const AMPLITUDE: f64 = 0.5;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-speakers-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A 16-bit 5.1 WAV (`WAVE_FORMAT_EXTENSIBLE`, side surround channels):
/// a 1 kHz tone (the LFE 50 Hz) in one channel after the other.
fn write_surround(path: &Path) {
    let frames = HEAD + (BURST + GAP) * u32::try_from(CHANNELS).expect("6") + TAIL;
    let data_len = frames * 2 * u32::try_from(CHANNELS).expect("6");
    let mut bytes = Vec::with_capacity(68 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(60 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&0xFFFEu16.to_le_bytes()); // extensible
    bytes.extend_from_slice(&6u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 12).to_le_bytes());
    bytes.extend_from_slice(&12u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(&22u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    // FL FR FC LFE SL SR.
    bytes.extend_from_slice(&0x60Fu32.to_le_bytes());
    // KSDATAFORMAT_SUBTYPE_PCM.
    bytes.extend_from_slice(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ]);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.resize(bytes.len() + HEAD as usize * 2 * CHANNELS, 0);
    for channel in 0..CHANNELS {
        let frequency = if channel == 3 { 50.0 } else { 1000.0 };
        let length = if channel + 1 == CHANNELS {
            BURST + GAP + TAIL
        } else {
            BURST + GAP
        };
        for n in 0..length {
            let value = if n < BURST {
                let phase = f64::from(n) * frequency * std::f64::consts::TAU / f64::from(RATE);
                AMPLITUDE * phase.sin()
            } else {
                0.0
            };
            for slot in 0..CHANNELS {
                // In range: the amplitude is below full scale.
                #[allow(clippy::cast_possible_truncation)]
                let sample = if slot == channel {
                    (value * f64::from(i16::MAX)).round() as i16
                } else {
                    0
                };
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }
    }
    std::fs::write(path, bytes).expect("write the 5.1 file");
}

/// The frames of a stereo 16-bit PCM WAV as values in -1..1.
fn read_stereo(path: &Path) -> Vec<[f64; 2]> {
    let bytes = std::fs::read(path).expect("read output");
    assert_eq!(&bytes[0..4], b"RIFF", "a WAV file");
    let mut at = 12;
    let mut channels = 0usize;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("len")) as usize;
        let body = at + 8;
        if id == b"fmt " {
            channels = usize::from(u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]));
        } else if id == b"data" {
            assert_eq!(channels, 2, "a stereo output");
            // mpv may leave the length open while writing; take what is there.
            let end = if len == 0 || len == usize::MAX {
                bytes.len()
            } else {
                bytes.len().min(body.saturating_add(len))
            };
            let sample = |pair: &[u8]| f64::from(i16::from_le_bytes([pair[0], pair[1]])) / 32768.0;
            let (frames, _) = bytes[body..end].as_chunks::<4>();
            return frames
                .iter()
                .map(|frame| [sample(&frame[0..2]), sample(&frame[2..4])])
                .collect();
        }
        at = body + len + (len & 1);
    }
    panic!("no data chunk in {}", path.display());
}

/// RVP's downmix entry for the stereo speakers and a 5.1 source, as the
/// chain builds it with the default settings.
fn stereo_downmix() -> String {
    let audio = Audio::default();
    let source = Source {
        channels: vec!["FL", "FR", "FC", "LFE", "SL", "SR"],
        ..Source::default()
    };
    let (equalizer, normalizer, responses) = (
        Equalizer::default(),
        Normalizer::default(),
        Responses::default(),
    );
    let inputs = Inputs {
        audio: &audio,
        speakers: choice("stereo").expect("stereo"),
        equalizer: &equalizer,
        normalizer: &normalizer,
        channel: rvp_app::audio::StereoChannel::Both,
        stretching: 0,
        source: &source,
        gain: 1.0,
        responses: &responses,
    };
    let (front, _) = build(&inputs);
    front
        .into_iter()
        .find(|(label, _)| *label == LABEL)
        .map(|(_, entry)| entry)
        .expect("the downmix")
}

/// Renders `input` to stereo through `entry` (none: mpv's own mix).
/// Without `keep-open` mpv drains its output before it ends the file.
fn render(dir: &Path, input: &Path, entry: Option<&str>, name: &str) -> Vec<[f64; 2]> {
    let output = dir.join(format!("out-{name}.wav"));
    let mut options = EngineOptions::headless();
    options.extra = vec![
        ("keep-open".to_owned(), "no".to_owned()),
        ("ao".to_owned(), "pcm".to_owned()),
        ("ao-pcm-file".to_owned(), output.display().to_string()),
        ("audio-format".to_owned(), "s16".to_owned()),
        ("audio-samplerate".to_owned(), RATE.to_string()),
        ("audio-channels".to_owned(), "stereo".to_owned()),
        ("vid".to_owned(), "no".to_owned()),
    ];
    let engine = Engine::new(&options).expect("libmpv starts");
    if let Some(entry) = entry {
        engine
            .set_audio_filter_first(LABEL, Some(entry))
            .expect("af pre");
    }
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
                } => panic!("playback failed: {event:?}"),
                _ => {}
            }
        }
        assert!(Instant::now() < deadline, "playback did not end");
    }
    engine.quit().expect("quit");
    drop(engine);
    read_stereo(&output)
}

/// Left and right RMS in dB of each channel's burst (its middle half). The
/// bursts are found from the first one's start, as mpv may drop a little
/// of the file's start.
fn burst_levels(frames: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let period = (BURST + GAP) as usize;
    let onset = frames
        .iter()
        .position(|frame| frame[0].abs() > 0.01 || frame[1].abs() > 0.01)
        .expect("the first burst");
    let first = onset;
    assert!(
        frames.len() >= first + (CHANNELS - 1) * period + BURST as usize,
        "{} frames rendered, the bursts start at {first}",
        frames.len()
    );
    (0..CHANNELS)
        .map(|channel| {
            let start = first + channel * period + BURST as usize / 4;
            let part = &frames[start..start + BURST as usize / 2];
            let level = |side: usize| {
                // Sample counts are far below 2^52.
                #[allow(clippy::cast_precision_loss)]
                let mean = part.iter().map(|f| f[side] * f[side]).sum::<f64>() / part.len() as f64;
                10.0 * mean.max(1e-20).log10()
            };
            [level(0), level(1)]
        })
        .collect()
}

#[test]
fn stereo_mixes_down_as_potplayer_and_unlike_mpv() {
    let dir = scratch("mix");
    let input = dir.join("surround.wav");
    write_surround(&input);
    // A sine's RMS is 3.01 dB below its peak: one channel alone gives
    // -9.03 dB, at -3 dB it gives -12.04 dB.
    let alone = 20.0 * AMPLITUDE.log10() - 3.0103;
    let minus3 = alone - 3.0103;
    let silent = -100.0;

    let downmix = stereo_downmix();
    let potplayer = burst_levels(&render(&dir, &input, Some(&downmix), "rvp"));
    let expected = [
        ("front left", [alone, silent]),
        ("front right", [silent, alone]),
        ("centre", [minus3, minus3]),
        ("LFE", [minus3, minus3]),
        ("surround left", [alone, silent]),
        ("surround right", [silent, alone]),
    ];
    for ((name, want), got) in expected.iter().zip(&potplayer) {
        for side in 0..2 {
            if want[side] <= silent {
                assert!(got[side] < -60.0, "{name}: side {side} {} dB", got[side]);
            } else {
                assert!(
                    (got[side] - want[side]).abs() < 0.1,
                    "{name}: side {side} {} dB, PotPlayer's {} dB",
                    got[side],
                    want[side]
                );
            }
        }
    }

    // mpv's own mix: the surround channels at -3 dB, no LFE.
    let mpv = burst_levels(&render(&dir, &input, None, "mpv"));
    assert!(
        (mpv[4][0] - minus3).abs() < 0.1,
        "mpv's surround {} dB",
        mpv[4][0]
    );
    assert!(mpv[3][0] < -60.0, "mpv's LFE {} dB", mpv[3][0]);
    let _ = std::fs::remove_dir_all(&dir);
}
