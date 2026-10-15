//! The normalizer's chain (TomSteady, `rvp_app::normalizer`) on synthetic
//! audio, headless: no window, no sound.
//!
//! `ao=pcm` renders as fast as mpv can, faster than the gain control
//! follows, so these checks hold whatever the gain: the chain is accepted
//! by this platform's ffmpeg, nothing goes above TomSteady's ceiling, and
//! the tempo filter that follows the normalizer in the player changes
//! nothing. The gain control itself is checked in real time in
//! `normalizer_live.rs`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::normalizer::{LABEL, Normalizer, TEMPO_ENTRY, TEMPO_LABEL};
use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId};

const RATE: u32 = 48_000;
/// TomSteady's ceiling, 30000 of 32768, in dBFS.
const CEILING_DB: f64 = -0.767;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-norm-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A 16-bit mono WAV of 440 Hz tones: `(seconds, dBFS peak)` one after the
/// other.
fn write_tones(path: &Path, parts: &[(u32, f64)]) {
    let samples: u32 = parts.iter().map(|(seconds, _)| seconds * RATE).sum();
    let data_len = samples * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    let mut n = 0u32;
    for &(seconds, peak_db) in parts {
        let amplitude = 10f64.powf(peak_db / 20.0);
        for _ in 0..seconds * RATE {
            let phase = f64::from(n) * 440.0 * std::f64::consts::TAU / f64::from(RATE);
            // In range: the amplitude is at most full scale.
            #[allow(clippy::cast_possible_truncation)]
            let sample = (amplitude * phase.sin() * f64::from(i16::MAX)).round() as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
            n += 1;
        }
    }
    std::fs::write(path, bytes).expect("write tones");
}

/// The first channel of a 16-bit PCM WAV as values in -1..1, and its rate.
fn read_wav(path: &Path) -> (Vec<f64>, u32) {
    let bytes = std::fs::read(path).expect("read output");
    assert_eq!(&bytes[0..4], b"RIFF", "a WAV file");
    let mut at = 12;
    let (mut channels, mut rate, mut bits) = (0usize, 0u32, 0u16);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("len")) as usize;
        let body = at + 8;
        if id == b"fmt " {
            channels = usize::from(u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]));
            rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().expect("rate"));
            bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
        } else if id == b"data" {
            assert_eq!(bits, 16, "s16 output");
            // mpv may leave the length open while writing; take what is there.
            let end = if len == 0 || len == usize::MAX {
                bytes.len()
            } else {
                bytes.len().min(body.saturating_add(len))
            };
            let frame = channels * 2;
            let samples = bytes[body..end]
                .chunks_exact(frame)
                .map(|frame| f64::from(i16::from_le_bytes([frame[0], frame[1]])) / 32768.0)
                .collect();
            return (samples, rate);
        }
        at = body + len + (len & 1);
    }
    panic!("no data chunk in {}", path.display());
}

fn rms_db(samples: &[f64]) -> f64 {
    // Sample counts are far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let mean = samples.iter().map(|s| s * s).sum::<f64>() / samples.len() as f64;
    10.0 * mean.log10()
}

fn peak_db(samples: &[f64]) -> f64 {
    20.0 * samples
        .iter()
        .fold(0.0f64, |max, s| max.max(s.abs()))
        .log10()
}

/// Waits until mpv reaches the end of the file (keep-open sets
/// eof-reached), or until `stop` returns true.
fn play_until(
    engine: &Engine,
    model: &mut PlayerModel,
    mut stop: impl FnMut(&PlayerModel) -> bool,
) {
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    while model.flag(PropertyId::EofReached) != Some(true) && !stop(model) {
        assert!(
            Instant::now() < deadline,
            "playback did not end: {events:?}"
        );
        assert!(
            !events.iter().any(|event| matches!(
                event,
                EngineEvent::EndFile {
                    reason: EndReason::Error,
                    ..
                }
            )),
            "playback failed: {events:?}"
        );
        engine.wait_and_drain(0.05, model, &mut events);
    }
}

/// Renders `input` through the normalizer's chain (at gain 1, no control)
/// into a WAV and returns its samples.
fn render(dir: &Path, input: &Path, tempo: bool) -> Vec<f64> {
    let output = dir.join(format!("out-{}.wav", u8::from(tempo)));
    let mut options = EngineOptions::headless();
    options.extra = vec![
        ("ao".to_owned(), "pcm".to_owned()),
        ("ao-pcm-file".to_owned(), output.display().to_string()),
        ("audio-format".to_owned(), "s16".to_owned()),
        ("audio-samplerate".to_owned(), RATE.to_string()),
        ("vid".to_owned(), "no".to_owned()),
    ];
    let engine = Engine::new(&options).expect("libmpv starts");
    engine
        .set_audio_filter(LABEL, Some(&Normalizer::default().chain_entry(1.0)))
        .expect("af add");
    if tempo {
        engine
            .set_audio_filter(TEMPO_LABEL, Some(TEMPO_ENTRY))
            .expect("af add tempo");
    }
    engine
        .load(&input.display().to_string(), false)
        .expect("loadfile");
    engine.set_pause(false).expect("play");
    play_until(&engine, &mut PlayerModel::new(), |_| false);
    engine.quit().expect("quit");
    drop(engine);
    let (samples, rate) = read_wav(&output);
    assert_eq!(rate, RATE);
    samples
}

#[test]
fn the_chain_holds_the_ceiling_and_the_tempo_filter_changes_nothing() {
    let dir = scratch("chain");
    let input = dir.join("tones.wav");
    write_tones(&input, &[(4, -30.0), (4, 0.0)]);
    let second = RATE as usize;
    let quiet = |samples: &[f64]| rms_db(&samples[second..3 * second]);
    let loud = |samples: &[f64]| rms_db(&samples[5 * second..7 * second]);

    let plain = render(&dir, &input, false);
    let with_tempo = render(&dir, &input, true);
    assert!(
        plain.len() + second / 2 >= 8 * second,
        "{} samples",
        plain.len()
    );
    // At gain 1 the quiet tone passes as it is (a sine's RMS is 3 dB below
    // its peak) and the full-scale one is held at the ceiling.
    assert!(
        (quiet(&plain) + 33.0).abs() < 0.3,
        "quiet {}",
        quiet(&plain)
    );
    for (name, samples) in [("plain", &plain), ("with tempo", &with_tempo)] {
        assert!(
            peak_db(samples) <= CEILING_DB + 0.05,
            "{name}: peak {} dBFS",
            peak_db(samples)
        );
    }
    for (part, a, b) in [
        ("quiet", quiet(&plain), quiet(&with_tempo)),
        ("loud", loud(&plain), loud(&with_tempo)),
    ] {
        assert!(
            (a - b).abs() < 0.2,
            "{part} part with the tempo filter: {a} vs {b} dB"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
