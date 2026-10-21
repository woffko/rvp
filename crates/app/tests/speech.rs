//! M13's subtitles made from the audio on a headless player (no window,
//! no sound): whisper.cpp's command line on a synthetic speech clip with
//! the tiny model, ahead of time and in real time. Nothing goes online.
//!
//! Each test is skipped, with a note, when a part is missing:
//! * `RVP_WHISPER_CLI`: whisper.cpp's `whisper-cli` (`tools/whisper/`);
//! * `RVP_WHISPER_MODELS`: a folder with `ggml-tiny.bin`, which the user
//!   downloads (never committed);
//! * the clip: `RVP_SPEECH_CLIP` (a WAV file), or one made here with
//!   espeak-ng (`RVP_ESPEAK` names it, else `espeak-ng` on the PATH; its
//!   data through `ESPEAK_DATA_PATH` when it is not installed).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use rvp_app::speech::SpeechCommand;
use rvp_app::{Action, App, Settings, UiRequest};
use rvp_engine::{Engine, EngineOptions, PropertyId};

/// What the clip says.
const SPEECH: &str = "Hello world. This is a test of subtitles made from speech.";

/// The clip, made into `dir` when espeak-ng is there; `None` with a note
/// when a part is missing.
fn prepare(dir: &Path) -> Option<PathBuf> {
    let present = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let Some(models) = present("RVP_WHISPER_MODELS").map(PathBuf::from) else {
        eprintln!("note: RVP_WHISPER_MODELS is not set; skipped");
        return None;
    };
    if present("RVP_WHISPER_CLI").is_none() || !models.join("ggml-tiny.bin").is_file() {
        eprintln!("note: whisper-cli or ggml-tiny.bin is missing; skipped");
        return None;
    }
    std::fs::create_dir_all(dir).expect("dir");
    let clip = dir.join("speech.wav");
    if let Some(given) = present("RVP_SPEECH_CLIP") {
        std::fs::copy(given, &clip).expect("copy the clip");
        return Some(clip);
    }
    let espeak = present("RVP_ESPEAK").map_or_else(|| PathBuf::from("espeak-ng"), PathBuf::from);
    let made = Command::new(&espeak)
        .arg("-w")
        .arg(&clip)
        .arg(SPEECH)
        .status()
        .is_ok_and(|status| status.success());
    if made {
        Some(clip)
    } else {
        eprintln!("note: no speech clip (RVP_SPEECH_CLIP, or espeak-ng); skipped");
        None
    }
}

struct Harness {
    app: App,
    engine: Engine,
    dir: PathBuf,
    requests: Vec<UiRequest>,
}

impl Harness {
    fn new(name: &str, change: impl FnOnce(&mut Settings)) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("rvp-speech-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let clip = prepare(&dir)?;
        std::fs::create_dir_all(dir.join("state")).expect("dir");
        let mut settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            subtitle_auto: "no".to_owned(),
            ..Settings::default()
        };
        for recognizer in [
            &mut settings.speech.file,
            &mut settings.speech.realtime_recognizer,
        ] {
            "cpu".clone_into(&mut recognizer.engine);
            "tiny".clone_into(&mut recognizer.model);
            "en".clone_into(&mut recognizer.language);
        }
        change(&mut settings);
        let mut harness = Self {
            app: App::new(settings, &dir.join("state")).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
            dir,
            requests: Vec::new(),
        };
        let file = clip.to_string_lossy().into_owned();
        harness
            .app
            .open(&harness.engine, &[file], false)
            .expect("open");
        harness.wait("the clip", Duration::from_secs(15), |h| {
            h.app.model.media_loaded()
                && h.app
                    .model
                    .double(PropertyId::Duration)
                    .is_some_and(|duration| duration > 0.0)
        });
        // Paused at once: the clip is a few seconds long.
        harness.engine.set_pause(true).expect("pause");
        Some(harness)
    }

    fn wait(&mut self, what: &str, timeout: Duration, done: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + timeout;
        while !done(self) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {:?}",
                self.requests
            );
            std::thread::sleep(Duration::from_millis(20));
            let drained = self.app.drain(&self.engine, Instant::now());
            self.requests.extend(drained.requests);
        }
    }

    /// The titles of the subtitle tracks.
    fn track_titles(&self) -> Vec<String> {
        self.engine
            .tracks()
            .into_iter()
            .filter(|track| track.kind == rvp_engine::TrackKind::Subtitle)
            .filter_map(|track| track.title)
            .collect()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn the_whole_file_becomes_subtitles_next_to_it() {
    let Some(mut h) = Harness::new("file", |_| {}) else {
        return;
    };
    match h
        .app
        .dispatch(&h.engine, Action::Speech(SpeechCommand::Start))
    {
        Some(UiRequest::Osd(text)) => {
            assert_eq!(text, "Subtitles from audio: extracting the audio");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        h.app.checked(Action::Speech(SpeechCommand::Start)),
        Some(true)
    );
    h.wait("the subtitles", Duration::from_mins(3), |h| {
        h.requests.iter().any(|request| {
            matches!(request, UiRequest::Osd(text) if text.contains(" lines in speech.en.srt"))
                || matches!(request, UiRequest::Message(_))
        })
    });
    let srt = std::fs::read_to_string(h.dir.join("speech.en.srt")).expect("the SRT file");
    assert!(srt.to_lowercase().contains("hello"), "{srt}");
    assert!(srt.starts_with("1\r\n00:00:0"), "{srt}");
    h.wait("the track", Duration::from_secs(5), |h| {
        h.track_titles()
            .iter()
            .any(|title| title == "Subtitles from audio (whisper)")
    });
    assert_eq!(
        h.app.checked(Action::Speech(SpeechCommand::Start)),
        Some(false)
    );
}

#[test]
fn real_time_subtitles_follow_the_playback_position() {
    let Some(mut h) = Harness::new("realtime", |settings| {
        "always".clone_into(&mut settings.speech.realtime);
    }) else {
        return;
    };
    h.wait("real-time subtitles", Duration::from_mins(3), |h| {
        h.track_titles()
            .iter()
            .any(|title| title == "Real-time subtitles (whisper)")
            || h.requests
                .iter()
                .any(|request| matches!(request, UiRequest::Message(_)))
    });
    let srt = std::fs::read_to_string(h.dir.join("state/subtitles/realtime.srt")).expect("the SRT");
    assert!(srt.to_lowercase().contains("hello"), "{srt}");
    assert!(
        h.app.model.double(PropertyId::Duration).unwrap_or_default() < 30.0,
        "one piece"
    );
}
