//! The skins' spectrum (`rvp_app::spectrum`) in real time on synthetic
//! audio, headless (`ao=null`: no window, no sound): mpv runs the tap's
//! graph, and a 1 kHz tone lights the band around 1 kHz most, with the
//! bands far from it dark.
//!
//! A test binary of its own: libmpv routes ffmpeg's log, where the reader
//! takes the levels, to the first player core of a process only.

use std::path::Path;
use std::time::{Duration, Instant};

use rvp_app::spectrum::{BANDS, Control, LABEL, centres, chain_entry};
use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId};

const RATE: u32 = 48_000;

/// A 16-bit mono WAV of a 1 kHz tone at -6 dBFS.
fn write_tone(path: &Path, seconds: u32) {
    let samples = seconds * RATE;
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
    for n in 0..samples {
        let phase = f64::from(n) * 1000.0 * std::f64::consts::TAU / f64::from(RATE);
        // Half of full scale.
        #[allow(clippy::cast_possible_truncation)]
        let sample = (0.5 * phase.sin() * f64::from(i16::MAX)).round() as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path, bytes).expect("write tone");
}

#[test]
fn a_tone_lights_its_band() {
    let dir = std::env::temp_dir().join(format!("rvp-spectrum-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let input = dir.join("tone.wav");
    write_tone(&input, 6);
    let mut options = EngineOptions::headless();
    options.extra = vec![("vid".to_owned(), "no".to_owned())];
    let engine = Engine::new(&options).expect("libmpv starts");
    engine
        .set_audio_filter(LABEL, Some(&chain_entry()))
        .expect("af add");
    let control = Control::start(&engine).expect("spectrum reader");
    engine
        .load(&input.display().to_string(), false)
        .expect("loadfile");
    engine.set_pause(false).expect("play");
    let mut model = PlayerModel::new();
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut levels = [0.0; BANDS];
    while model.double(PropertyId::TimePos).unwrap_or(0.0) < 3.0 {
        assert!(Instant::now() < deadline, "playback stalled: {events:?}");
        assert!(
            !events.iter().any(|event| matches!(
                event,
                EngineEvent::EndFile {
                    reason: EndReason::Error,
                    ..
                }
            )),
            "playback failed (the graph?): {events:?}"
        );
        engine.wait_and_drain(0.05, &mut model, &mut events);
        levels = control.levels(Instant::now());
    }
    eprintln!("levels {levels:?}");
    let centres = centres();
    let near = (0..BANDS)
        .min_by(|a, b| {
            (centres[*a] / 1000.0)
                .log2()
                .abs()
                .total_cmp(&(centres[*b] / 1000.0).log2().abs())
        })
        .expect("bands");
    let loudest = (0..BANDS)
        .max_by(|a, b| levels[*a].total_cmp(&levels[*b]))
        .expect("bands");
    assert_eq!(loudest, near, "the band near 1 kHz: {levels:?}");
    assert!(levels[near] > 0.5, "{levels:?}");
    assert!(levels[0] < 0.2 && levels[BANDS - 1] < 0.2, "{levels:?}");
    // The audio itself passes: mpv's output keeps playing to the end.
    drop(control);
    engine.quit().expect("quit");
    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}
