//! Renders a file through RVP's whole audio chain as the player builds it
//! (goal L's comparison with PotPlayer, `docs/audits/audio.md`): headless
//! libmpv with `ao=pcm` (no window, no sound), the settings from a
//! `settings.json`, the normalizer off (its gain control follows mpv's log
//! in real time; `ao=pcm` renders faster). The output is 32-bit float at
//! 48 kHz with the speakers' channels.
//!
//! ```text
//! cargo run --release -p rvp-app --example render_chain -- IN OUT.wav [SETTINGS.json]
//! ```

use std::time::{Duration, Instant};

use rvp_app::{App, Settings};
use rvp_engine::{AudioOutput, EndReason, Engine, EngineEvent, VideoOutput};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        return Err("usage: render_chain IN OUT.wav [SETTINGS.json]".to_owned());
    };
    let mut settings: Settings = match args.get(2) {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?
        }
        None => Settings::default(),
    };
    settings.normalizer.enabled = false;
    settings.reopen_last_file = false;
    settings.remember_positions = false;
    settings.ipc_server = String::new();
    let notes = settings.validate();
    if !notes.is_empty() {
        eprintln!("settings: {notes:?}");
    }
    let dir = std::env::temp_dir().join(format!("rvp-render-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = App::new(settings, &dir).map_err(|e| e.to_string())?;
    let mut options = app.engine_options();
    options.video = VideoOutput::Null;
    options.audio = AudioOutput::Null;
    "no".clone_into(&mut options.hwdec);
    options.ipc_server = None;
    options.extra.extend(
        [
            ("ao", "pcm"),
            ("ao-pcm-file", output.as_str()),
            ("audio-format", "float"),
            ("audio-samplerate", "48000"),
            ("keep-open", "no"),
            ("vid", "no"),
        ]
        .map(|(name, value)| (name.to_owned(), value.to_owned())),
    );
    let engine = Engine::new(&options).map_err(|e| e.to_string())?;
    app.apply_audio_chain(&engine);
    app.open(&engine, std::slice::from_ref(input), false)
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(600);
    let mut chain = String::new();
    'playing: loop {
        let drained = app.drain(&engine, Instant::now());
        for event in &drained.events {
            match event {
                EngineEvent::FileLoaded { .. } => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                EngineEvent::EndFile {
                    reason: EndReason::Eof,
                    ..
                } => break 'playing,
                EngineEvent::EndFile {
                    reason: EndReason::Error,
                    ..
                } => return Err(format!("{input}: playback failed")),
                _ => {}
            }
        }
        if chain.is_empty() {
            chain = engine.get_text("af").unwrap_or_default();
        }
        if Instant::now() > deadline {
            return Err("playback did not end".to_owned());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("af={chain}");
    drop(engine.quit());
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
