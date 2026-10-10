//! libmpv in headless mode (`vo=null`, `ao=null`) on the synthetic fixtures
//! in `testdata/media` (`tools/test-clips/make-fixtures.sh`): the property
//! model, commands and events (docs/PLAN.md section 7).

use std::time::{Duration, Instant};

use rvp_engine::{EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId, Seek};

/// 96x72, 10 fps H.264 with an Opus tone; 60 s long.
const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);
/// The same, 3 s long.
const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);

struct Player {
    engine: Engine,
    model: PlayerModel,
    events: Vec<EngineEvent>,
}

impl Player {
    fn new() -> Self {
        Self {
            engine: Engine::new(&EngineOptions::headless()).expect("libmpv starts"),
            model: PlayerModel::new(),
            events: Vec::new(),
        }
    }

    /// Pumps events until `done` holds or 10 s pass.
    fn wait(&mut self, what: &str, done: impl Fn(&PlayerModel, &[EngineEvent]) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done(&self.model, &self.events) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {:?}",
                self.events
            );
            self.engine
                .wait_and_drain(0.05, &mut self.model, &mut self.events);
        }
    }

    fn load(&mut self, target: &str) {
        self.events.clear();
        self.engine.load(target, false).expect("loadfile");
        self.wait("file loaded", |_, events| {
            events.contains(&EngineEvent::FileLoaded)
        });
        self.wait("duration", |model, _| {
            model.double(PropertyId::Duration).is_some()
        });
    }
}

#[test]
fn loading_fills_the_model() {
    let mut player = Player::new();
    player.wait("idle", |model, _| {
        model.flag(PropertyId::IdleActive) == Some(true)
    });
    player.load(LONG);
    player.wait("video size", |model, _| {
        model.int(PropertyId::VideoWidth).is_some()
    });
    player.wait("audio", |model, _| {
        model.int(PropertyId::AudioSampleRate).is_some()
    });
    let model = &player.model;
    assert_eq!(model.int(PropertyId::VideoWidth), Some(96));
    assert_eq!(model.int(PropertyId::VideoHeight), Some(72));
    assert_eq!(model.int(PropertyId::PlaylistCount), Some(1));
    assert_eq!(model.int(PropertyId::PlaylistPos), Some(0));
    assert_eq!(model.text(PropertyId::VideoFormat), Some("h264"));
    assert_eq!(model.text(PropertyId::AudioCodecName), Some("opus"));
    assert_eq!(model.int(PropertyId::AudioChannels), Some(1));
    let duration = model.double(PropertyId::Duration).unwrap_or_default();
    assert!((duration - 60.0).abs() < 0.1, "duration {duration}");
    assert!(model.media_loaded());
    assert_eq!(model.text(PropertyId::Path), Some(LONG));
    assert_eq!(model.text(PropertyId::MediaTitle), Some("RVP fixture 60 s"));
}

#[test]
fn pause_seek_and_time() {
    let mut player = Player::new();
    player.load(LONG);
    player.engine.set_pause(false).expect("unpause");
    player.wait("playing", |model, _| model.playing());
    player.wait("time advances", |model, _| {
        model.double(PropertyId::TimePos).unwrap_or_default() > 0.5
    });
    player.engine.set_pause(true).expect("pause");
    player.wait("paused", |model, _| {
        model.flag(PropertyId::Pause) == Some(true)
    });
    assert!(!player.model.playing());

    player.events.clear();
    player.engine.seek(Seek::Absolute(12.0)).expect("seek");
    player.wait("seek done", |_, events| {
        events.contains(&EngineEvent::PlaybackRestart)
    });
    player.wait("time near 12", |model, _| {
        (model.double(PropertyId::TimePos).unwrap_or_default() - 12.0).abs() < 0.15
    });

    player.events.clear();
    player
        .engine
        .seek(Seek::Relative(-5.0))
        .expect("relative seek");
    player.wait("seek back", |model, _| {
        (model.double(PropertyId::TimePos).unwrap_or_default() - 7.0).abs() < 0.15
    });

    player
        .engine
        .seek(Seek::Fraction(0.5))
        .expect("fraction seek");
    player.wait("half way", |model, _| {
        (model.double(PropertyId::TimePos).unwrap_or_default() - 30.0).abs() < 0.15
    });
}

#[test]
fn volume_and_mute() {
    let mut player = Player::new();
    player.engine.set_volume(37.0).expect("volume");
    player.wait("volume", |model, _| {
        model.double(PropertyId::Volume) == Some(37.0)
    });
    player.engine.set_mute(true).expect("mute");
    player.wait("mute", |model, _| {
        model.flag(PropertyId::Mute) == Some(true)
    });
    player.engine.set_mute(false).expect("unmute");
    player.wait("unmute", |model, _| {
        model.flag(PropertyId::Mute) == Some(false)
    });
}

#[test]
fn playlist_next_and_previous() {
    let mut player = Player::new();
    player.load(LONG);
    player.engine.load(SHORT, true).expect("append");
    player.wait("two entries", |model, _| {
        model.int(PropertyId::PlaylistCount) == Some(2)
    });
    player.engine.playlist_step(1).expect("next");
    player.wait("second entry", |model, _| {
        model.int(PropertyId::PlaylistPos) == Some(1)
    });
    player.wait("second duration", |model, _| {
        model
            .double(PropertyId::Duration)
            .is_some_and(|d| (d - 3.0).abs() < 0.1)
    });
    player.engine.playlist_step(-1).expect("previous");
    player.wait("first entry", |model, _| {
        model.int(PropertyId::PlaylistPos) == Some(0)
    });
    assert!(
        player.engine.playlist_step(-1).is_err(),
        "no entry before the first"
    );
}

/// The vendored libmpv2 fix: properties that lose their value after `stop`
/// are reported, and later events are still delivered.
#[test]
fn stop_clears_unavailable_properties() {
    let mut player = Player::new();
    player.load(SHORT);
    player.events.clear();
    player.engine.stop().expect("stop");
    player.wait("end of file", |_, events| {
        events.contains(&EngineEvent::EndFile(EndReason::Stop))
    });
    player.wait("duration cleared", |model, _| {
        model.double(PropertyId::Duration).is_none()
    });
    player.wait("idle", |model, _| {
        model.flag(PropertyId::IdleActive) == Some(true)
    });
    assert!(!player.model.media_loaded());
    // Events after the unavailable ones still arrive.
    player.load(SHORT);
}

#[test]
fn end_of_file_keeps_the_last_frame() {
    let mut player = Player::new();
    player.load(SHORT);
    player.engine.set_pause(false).expect("play");
    player.wait("eof", |model, _| {
        model.flag(PropertyId::EofReached) == Some(true)
    });
    // keep-open: the file stays loaded on its last frame and no longer
    // counts as playing.
    assert!(player.model.media_loaded());
    assert!(!player.model.playing());
}

#[test]
fn unplayable_files_report_an_error() {
    let mut player = Player::new();
    player.events.clear();
    player
        .engine
        .load("/nonexistent/rvp-test-missing.mkv", false)
        .expect("loadfile is accepted");
    player.wait("error", |_, events| {
        events
            .iter()
            .any(|event| matches!(event, EngineEvent::Error(_)))
    });
}
