//! The normalizer's gain control (TomSteady, `rvp_app::normalizer::Control`)
//! in real time on synthetic audio, headless (`ao=null`: no window, no
//! sound): a quiet tone raises the gain, a loud one cuts it under the
//! ceiling, and mpv's filter applies the gain.
//!
//! A test binary of its own: libmpv routes ffmpeg's log, where the control
//! reads each block's peak, to the first player core of a process only, so
//! no other core may run beside this one.

use std::path::Path;
use std::time::{Duration, Instant};

use rvp_app::normalizer::{Control, LABEL, Normalizer};
use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId};

const RATE: u32 = 48_000;
/// TomSteady's ceiling, 30000 of 32768, in dBFS.
const CEILING_DB: f64 = -0.767;

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

/// Plays until `time-pos` reaches `seconds`.
fn play_to(engine: &Engine, model: &mut PlayerModel, seconds: f64) {
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while model.double(PropertyId::TimePos).unwrap_or(0.0) < seconds {
        assert!(Instant::now() < deadline, "playback stalled: {events:?}");
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

/// The output peak the probe filter after the normalizer measured last
/// (mpv filters a few tenths of a second ahead of `time-pos`).
fn probe_peak_db(engine: &Engine) -> Option<f64> {
    let metadata = engine.get_json("af-metadata/probe")?;
    metadata["lavfi.astats.Overall.Peak_level"]
        .as_str()?
        .parse()
        .ok()
}

#[test]
fn the_gain_follows_tomsteady_in_real_time() {
    let dir = std::env::temp_dir().join(format!("rvp-norm-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let input = dir.join("tones.wav");
    // A quiet tone, then a loud one (peak 29205 of 32768).
    write_tones(&input, &[(5, -30.0), (3, -1.0)]);
    let mut options = EngineOptions::headless();
    options.extra = vec![("vid".to_owned(), "no".to_owned())];
    let engine = Engine::new(&options).expect("libmpv starts");
    // Stepping 5: the gain moves by 1/32 per block, fast enough to see in
    // a few seconds (PotPlayer's 10 moves by 1/1024).
    let normalizer = Normalizer {
        release: 5.0,
        ..Normalizer::default()
    };
    engine
        .set_audio_filter(LABEL, Some(&normalizer.chain_entry(1.0)))
        .expect("af add");
    engine
        .set_audio_filter(
            "probe",
            Some(
                "@probe:lavfi=[astats=metadata=1:reset=1:\
                 measure_perchannel=none:measure_overall=Peak_level]",
            ),
        )
        .expect("af add probe");
    let control = Control::start(&engine, normalizer).expect("gain control");
    engine
        .load(&input.display().to_string(), false)
        .expect("loadfile");
    engine.set_pause(false).expect("play");
    let mut model = PlayerModel::new();

    play_to(&engine, &mut model, 4.0);
    let quiet_gain = control.gain();
    let quiet_out = probe_peak_db(&engine).expect("probe measured the quiet part");
    eprintln!("quiet part: gain {quiet_gain:.3}, output peak {quiet_out:.2} dBFS");
    // About 90 blocks raised by 1/32 each.
    assert!(quiet_gain > 2.5, "gain {quiet_gain}");
    // mpv's filter has the gain (a block or two later).
    let expected = -30.0 + 20.0 * quiet_gain.log10();
    assert!(
        (quiet_out - expected).abs() < 1.5,
        "output {quiet_out} dBFS, expected {expected}"
    );

    play_to(&engine, &mut model, 6.5);
    let loud_gain = control.gain();
    let loud_out = probe_peak_db(&engine).expect("probe measured the loud part");
    eprintln!("loud part: gain {loud_gain:.3}, output peak {loud_out:.2} dBFS");
    // Cut at once under the ceiling (30000 / 29205 = 1.027).
    assert!(loud_gain <= 1.03, "gain {loud_gain}");
    assert!(loud_out <= CEILING_DB + 0.1, "output {loud_out} dBFS");

    drop(control);
    engine.quit().expect("quit");
    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}
