//! The equalizer (M6) in headless libmpv: a band changes the level of a
//! tone at its frequency (`ao=pcm`, no sound), and beside the normalizer it
//! runs first, takes band changes without rebuilding the chain, and keeps
//! audio and video together when it is switched on and off.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::equalizer::LABEL;
use rvp_app::normalizer::{LABEL as NORMALIZER, TEMPO_LABEL as TEMPO};
use rvp_app::speakers::LABEL as DOWNMIX;
use rvp_app::{Action, App, Settings};
use rvp_engine::{Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId};

const RATE: u32 = 48_000;
const SECONDS: u32 = 6;
const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-eq-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A 16-bit mono WAV of a 1 kHz tone peaking at -30 dBFS.
fn write_tone(path: &Path) {
    let samples = RATE * SECONDS;
    let data_len = samples * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    let amplitude = 10f64.powf(-30.0 / 20.0);
    for n in 0..samples {
        let phase = f64::from(n) * 1000.0 * std::f64::consts::TAU / f64::from(RATE);
        // In range: the amplitude is below full scale.
        #[allow(clippy::cast_possible_truncation)]
        let sample = (amplitude * phase.sin() * f64::from(i16::MAX)).round() as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path, bytes).expect("write tone");
}

/// RMS level (dBFS) of the middle third of mpv's 16-bit PCM output.
fn middle_level(path: &Path) -> f64 {
    let bytes = std::fs::read(path).expect("read output");
    let data = bytes
        .windows(4)
        .position(|window| window == b"data")
        .expect("data chunk")
        + 8;
    let samples: Vec<f64> = bytes[data..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| f64::from(i16::from_le_bytes(*pair)) / 32768.0)
        .collect();
    let third = samples.len() / 3;
    let middle = &samples[third..2 * third];
    // Sample counts are far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let mean = middle.iter().map(|s| s * s).sum::<f64>() / middle.len().max(1) as f64;
    10.0 * mean.log10()
}

/// Plays the tone with the 1 kHz band at `gain` dB (`None`: equalizer off)
/// into a WAV and returns its level.
fn render(dir: &Path, input: &Path, gain: Option<f64>) -> f64 {
    let name = gain.map_or_else(|| "flat".to_owned(), |gain| format!("{gain:+.0}"));
    let output = dir.join(format!("out-{name}.wav"));
    let mut options = EngineOptions::headless();
    options.extra = vec![
        ("ao".to_owned(), "pcm".to_owned()),
        ("ao-pcm-file".to_owned(), output.display().to_string()),
        ("audio-format".to_owned(), "s16".to_owned()),
        ("audio-samplerate".to_owned(), RATE.to_string()),
        ("audio-channels".to_owned(), "mono".to_owned()),
    ];
    let engine = Engine::new(&options).expect("libmpv starts");
    let mut settings = Settings::default();
    settings.normalizer.enabled = false;
    if let Some(gain) = gain {
        settings.equalizer.enabled = true;
        settings.equalizer.bands[4] = gain;
    }
    let mut app = App::new(settings, &dir.join("state")).expect("app");
    app.apply_audio_chain(&engine);
    engine
        .load(&input.display().to_string(), false)
        .expect("loadfile");
    engine.set_pause(false).expect("play");
    let mut model = PlayerModel::new();
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        engine.wait_and_drain(0.1, &mut model, &mut events);
        if events
            .iter()
            .any(|event| matches!(event, EngineEvent::EndFile { .. }))
            || model.flag(PropertyId::EofReached) == Some(true)
        {
            break;
        }
    }
    drop(engine);
    middle_level(&output)
}

#[test]
fn a_band_changes_the_level_at_its_frequency() {
    let dir = scratch("level");
    let tone = dir.join("tone.wav");
    write_tone(&tone);
    let flat = render(&dir, &tone, None);
    let boosted = render(&dir, &tone, Some(12.0));
    let cut = render(&dir, &tone, Some(-12.0));
    // A sine's RMS is 3 dB below its peak.
    assert!((flat + 33.0).abs() < 1.5, "flat {flat:.1} dBFS");
    assert!(
        (boosted - flat - 12.0).abs() < 2.0,
        "+12 dB band: {flat:.1} -> {boosted:.1} dBFS"
    );
    assert!(
        (flat - cut - 12.0).abs() < 2.0,
        "-12 dB band: {flat:.1} -> {cut:.1} dBFS"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The labels of mpv's audio filter chain, in order.
fn labels(engine: &Engine) -> Vec<String> {
    engine
        .get_text("af")
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| entry.strip_prefix('@'))
        .filter_map(|entry| entry.split(':').next())
        .map(str::to_owned)
        .collect()
}

/// Lets mpv play for `ms` milliseconds and returns the largest A/V offset.
fn play(app: &mut App, engine: &Engine, ms: u64) -> f64 {
    let end = Instant::now() + Duration::from_millis(ms);
    let mut largest: f64 = 0.0;
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        app.drain(engine, Instant::now());
        if let Some(offset) = engine.get_double("avsync") {
            largest = largest.max(offset.abs());
        }
    }
    largest
}

#[test]
fn the_equalizer_runs_before_the_normalizer_and_keeps_sync() {
    let dir = scratch("chain");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        ..Settings::default()
    };
    assert!(settings.normalizer.enabled && !settings.equalizer.enabled);
    let mut app = App::new(settings, &dir).expect("app");
    app.start(&engine, &[LONG.to_owned()]).expect("start");
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.model.double(PropertyId::TimePos).unwrap_or(0.0) < 1.0 {
        assert!(Instant::now() < deadline, "the fixture plays");
        play(&mut app, &engine, 20);
    }
    // PotPlayer's downmix (stereo speakers, the default) comes first.
    assert_eq!(labels(&engine), [DOWNMIX, NORMALIZER, TEMPO]);

    app.dispatch(&engine, Action::ToggleEqualizer);
    let largest = play(&mut app, &engine, 1500);
    assert_eq!(labels(&engine), [DOWNMIX, LABEL, NORMALIZER, TEMPO]);
    assert!(
        largest < 0.2,
        "equalizer on: A/V offset up to {largest:.3} s"
    );

    // Bands change in the running filter: the chain stays as it was.
    let chain = engine.get_text("af");
    for (band, fraction) in [(0, 1.0), (4, 0.25), (9, 0.75)] {
        app.set_slider(&engine, rvp_app::panel::BAND_SLIDERS[band], fraction);
    }
    let largest = play(&mut app, &engine, 1000);
    assert_eq!(engine.get_text("af"), chain, "no rebuild for band changes");
    assert!(
        largest < 0.2,
        "band changes: A/V offset up to {largest:.3} s"
    );
    let bands = app.settings.equalizer.bands;
    assert!(
        (bands[0] - 20.0).abs() < 1e-9 && (bands[4] + 10.0).abs() < 1e-9,
        "{bands:?}"
    );
    assert_eq!(app.settings.equalizer.preset_text(), "Custom");

    app.dispatch(&engine, Action::ToggleEqualizer);
    let largest = play(&mut app, &engine, 1500);
    assert_eq!(labels(&engine), [DOWNMIX, NORMALIZER, TEMPO]);
    assert!(
        largest < 0.2,
        "equalizer off: A/V offset up to {largest:.3} s"
    );
    assert!(app.model.playing(), "still playing");
    let _ = std::fs::remove_dir_all(&dir);
}
