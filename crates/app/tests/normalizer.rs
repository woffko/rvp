//! The normalizer chain on synthetic audio (S5, docs/PLAN.md goal F): a
//! quiet tone followed by a loud one goes through mpv with RVP's `af` entry
//! and `ao=pcm`; Boost raises the quiet part, without Boost the normalizer
//! only lowers, nothing clips, and the tempo filter that follows the
//! normalizer in the player leaves the levels as they were.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::normalizer::{LABEL, Normalizer, TEMPO_ENTRY, TEMPO_LABEL};
use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId};

const RATE: u32 = 48_000;
/// Seconds of each tone.
const PART: u32 = 40;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-norm-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A 16-bit mono WAV: 440 Hz at -30 dBFS for `PART` seconds, then at
/// -6 dBFS for `PART` seconds.
fn write_tones(path: &Path) {
    let samples = RATE * PART * 2;
    let mut bytes = Vec::with_capacity(44 + samples as usize * 2);
    let data_len = samples * 2;
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
    for n in 0..samples {
        let amplitude = if n < RATE * PART {
            10f64.powf(-30.0 / 20.0)
        } else {
            10f64.powf(-6.0 / 20.0)
        };
        let phase = f64::from(n) * 440.0 * std::f64::consts::TAU / f64::from(RATE);
        // In range: the amplitude is below full scale.
        #[allow(clippy::cast_possible_truncation)]
        let sample = (amplitude * phase.sin() * f64::from(i16::MAX)).round() as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
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
            let end = bytes.len().min(body.saturating_add(len).max(body));
            let end = if len == 0 || len == usize::MAX {
                bytes.len()
            } else {
                end
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

/// Plays `input` through the normalizer into a WAV and returns the RMS
/// level (dBFS) of the settled middle of each tone and the peak.
fn render(dir: &Path, input: &Path, normalizer: &Normalizer, tempo: bool) -> (f64, f64, f64) {
    let output = dir.join(format!(
        "out-{}-{}.wav",
        u8::from(normalizer.boost),
        u8::from(tempo)
    ));
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
        .set_audio_filter(LABEL, Some(&normalizer.chain_entry()))
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
    let mut model = PlayerModel::new();
    let mut events = Vec::new();
    // keep-open: the end sets eof-reached.
    let deadline = Instant::now() + Duration::from_secs(120);
    while model.flag(PropertyId::EofReached) != Some(true) {
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
        engine.wait_and_drain(0.1, &mut model, &mut events);
    }
    engine.quit().expect("quit");
    drop(engine);
    let (samples, rate) = read_wav(&output);
    assert_eq!(rate, RATE);
    let second = RATE as usize;
    let part = PART as usize * second;
    assert!(
        samples.len() + second / 2 >= 2 * part,
        "{} samples",
        samples.len()
    );
    // The middle 10 s of each tone, three 5 s frames away from the edges:
    // dynaudnorm's minimum and Gaussian filters (g=3) have settled there.
    let middle = |start: usize| &samples[start + 15 * second..start + 25 * second];
    let quiet = rms_db(middle(0));
    let loud = rms_db(middle(part));
    let peak = samples.iter().fold(0.0f64, |max, s| max.max(s.abs()));
    (quiet, loud, 20.0 * peak.log10())
}

#[test]
fn boost_raises_quiet_passages_and_nothing_clips() {
    let dir = scratch("chain");
    let input = dir.join("tones.wav");
    write_tones(&input);
    // A sine's RMS is 3 dB below its peak.
    let (quiet_in, loud_in) = (-33.0, -9.0);

    let boost = render(&dir, &input, &Normalizer::default(), false);
    let without = render(
        &dir,
        &input,
        &Normalizer {
            boost: false,
            ..Normalizer::default()
        },
        false,
    );
    // The tempo filter RVP keeps after the normalizer changes nothing at
    // 1x (goal H).
    let with_tempo = render(&dir, &input, &Normalizer::default(), true);
    for (part, a, b) in [
        ("quiet", boost.0, with_tempo.0),
        ("loud", boost.1, with_tempo.1),
    ] {
        assert!(
            (a - b).abs() < 0.2,
            "{part} part with the tempo filter: {a} vs {b} dB"
        );
    }
    eprintln!("boost: {boost:?}, without boost: {without:?}");
    // Boost: the quiet tone gets the maximum gain, 5x (+14 dB); the loud
    // one (peak -6 dBFS) is raised towards a full-scale peak (up to +6 dB).
    assert!(
        (boost.0 - quiet_in - 14.0).abs() < 1.0,
        "quiet part raised 14 dB: {boost:?}"
    );
    assert!(
        (4.5..=6.0).contains(&(boost.1 - loud_in)),
        "loud part raised to the peak: {boost:?}"
    );
    // Without Boost the gain stays at or below 1.
    assert!(
        (without.0 - quiet_in).abs() < 0.5,
        "quiet part kept: {without:?}"
    );
    assert!(
        (without.1 - loud_in).abs() < 0.5,
        "loud part kept: {without:?}"
    );
    for (name, peak) in [("boost", boost.2), ("without", without.2)] {
        assert!(peak <= 0.0, "{name}: peak {peak} dBFS");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
