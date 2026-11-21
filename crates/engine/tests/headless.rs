//! libmpv in headless mode (`vo=null`, `ao=null`) on the synthetic fixtures
//! in `testdata/media` (`tools/test-clips/make-fixtures.sh`): the property
//! model, commands and events (docs/PLAN.md section 7).

use std::time::{Duration, Instant};

use rvp_engine::{
    EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId, Seek, TrackKind,
};

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
/// 160x90: four coloured quadrants and a white mark at the top left.
const MARKER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-marker.mkv"
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
            events
                .iter()
                .any(|event| matches!(event, EngineEvent::FileLoaded { .. }))
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
    // mpv normalizes the path on Windows (`..`, separators) but not on Linux;
    // the application keys its resume store by mpv's `path` for that reason.
    assert!(
        model
            .text(PropertyId::Path)
            .is_some_and(|path| path.ends_with("fixture-60s.mkv")),
        "path {:?}",
        model.text(PropertyId::Path)
    );
    assert_eq!(model.text(PropertyId::MediaTitle), Some("RVP fixture 60 s"));
}

#[test]
fn a_held_file_waits_before_its_first_frame() {
    let mut player = Player::new();
    player.engine.hold_preloaded().expect("hook");
    player.events.clear();
    player.engine.load(LONG, false).expect("loadfile");
    player.wait("the hold", |_, events| {
        events
            .iter()
            .any(|event| matches!(event, EngineEvent::Preloaded { .. }))
    });
    let Some(EngineEvent::Preloaded { hook, video }) = player
        .events
        .iter()
        .find(|event| matches!(event, EngineEvent::Preloaded { .. }))
        .cloned()
    else {
        unreachable!("waited for it");
    };
    // The demuxer's description, before any frame: no picture yet.
    let video = video.expect("the video track");
    assert_eq!((video.width, video.height), (96, 72));
    assert!((video.fps - 10.0).abs() < 0.01, "fps {}", video.fps);
    std::thread::sleep(Duration::from_millis(300));
    player
        .engine
        .wait_and_drain(0.05, &mut player.model, &mut player.events);
    assert_eq!(player.model.int(PropertyId::VideoWidth), None);
    player.engine.continue_preloaded(hook).expect("continue");
    player.wait("the first frame", |model, _| {
        model.int(PropertyId::VideoWidth) == Some(96)
    });
}

/// RVP closes while mpv holds a file at its hook, the event still waiting
/// in the queue: once the hooks are released, the drains of the shutdown
/// let the file go and mpv stops at once (it waits for a held file
/// whatever `stop` says).
#[test]
fn closing_lets_a_held_file_go() {
    let mut player = Player::new();
    player.engine.hold_preloaded().expect("hook");
    player.wait("idle at start", |model, _| {
        model.flag(PropertyId::IdleActive) == Some(true)
    });
    player.events.clear();
    player.engine.load(LONG, false).expect("loadfile");
    // mpv reaches the hook meanwhile; nothing drains its event.
    std::thread::sleep(Duration::from_millis(300));
    player.engine.release_hooks();
    player.engine.stop().expect("stop");
    let started = Instant::now();
    let stopped = |player: &Player| {
        player.model.flag(PropertyId::IdleActive) == Some(true)
            && player
                .events
                .iter()
                .any(|event| matches!(event, EngineEvent::EndFile { .. }))
    };
    while !stopped(&player) && started.elapsed() < Duration::from_secs(3) {
        player
            .engine
            .wait_and_drain(0.1, &mut player.model, &mut player.events);
    }
    let elapsed = started.elapsed();
    // A hook that came through anyway is let go before the asserts, so a
    // failure ends the test instead of holding mpv's core for good.
    let held: Vec<u64> = player
        .events
        .iter()
        .filter_map(|event| match event {
            EngineEvent::Preloaded { hook, .. } => Some(*hook),
            _ => None,
        })
        .collect();
    for hook in &held {
        drop(player.engine.continue_preloaded(*hook));
    }
    assert!(held.is_empty(), "the held file was reported: {held:?}");
    assert!(
        stopped(&player) && elapsed < Duration::from_secs(1),
        "idle after {elapsed:?}: {:?}",
        player.events
    );
}

/// Messages to RVP's client (a controller script's, M9): one without
/// arguments gives an empty list, one whose argument is not UTF-8 an
/// error, and the next one still comes.
#[test]
fn client_messages_without_arguments_or_utf8_arrive_safely() {
    let mut player = Player::new();
    player.events.clear();
    player
        .engine
        .command("script-message", &[])
        .expect("a message without arguments");
    // `osd-ass-cc/0` is mpv's OSD escape, the single byte 0xFD.
    player
        .engine
        .command("expand-properties", &["script-message", "${osd-ass-cc/0}"])
        .expect("a message that is not UTF-8");
    player
        .engine
        .command("script-message", &["after"])
        .expect("a message");
    let after = EngineEvent::ClientMessage(vec!["after".to_owned()]);
    player.wait("the last message", |_, events| events.contains(&after));
    let messages: Vec<&EngineEvent> = player
        .events
        .iter()
        .filter(|event| matches!(event, EngineEvent::ClientMessage(_) | EngineEvent::Error(_)))
        .collect();
    assert_eq!(
        messages,
        [
            &EngineEvent::ClientMessage(Vec::new()),
            &EngineEvent::Error("InvalidUtf8".to_owned()),
            &after
        ]
    );
}

/// Flags go to mpv and come back as its C `int` (not a one-byte `bool`).
#[test]
fn flags_are_read_and_written_whole() {
    let player = Player::new();
    let mpv = player.engine.mpv();
    for value in [true, false, true] {
        mpv.set_property("pause", value).expect("set");
        assert_eq!(mpv.get_property::<bool>("pause"), Ok(value));
        let text = mpv.get_property::<String>("pause");
        assert_eq!(text.as_deref(), Ok(if value { "yes" } else { "no" }));
    }
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
        events.iter().any(|event| {
            matches!(event, EngineEvent::EndFile { reason: EndReason::Stop, path: Some(path) } if path.ends_with("fixture-3s.mkv"))
        })
    });
    assert_eq!(player.model.loaded_path(), None);
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

#[test]
fn tracks_chapters_and_plain_properties() {
    let mut player = Player::new();
    player.load(LONG);
    let tracks = player.engine.tracks();
    let kinds: Vec<(TrackKind, i64, Option<&str>, bool)> = tracks
        .iter()
        .map(|t| (t.kind, t.id, t.codec.as_deref(), t.selected))
        .collect();
    assert_eq!(
        kinds,
        [
            (TrackKind::Video, 1, Some("h264"), true),
            (TrackKind::Audio, 1, Some("opus"), true)
        ]
    );
    assert!(player.engine.chapters().is_empty());
    assert_eq!(player.engine.get_int("chapters"), Some(0));

    player.engine.set_text("aid", "no").expect("audio off");
    player.wait("audio off", |model, _| {
        model.int(PropertyId::Chapters).is_some()
    });
    assert!(
        player
            .engine
            .tracks()
            .iter()
            .all(|t| t.kind != TrackKind::Audio || !t.selected)
    );
    player.engine.set_text("aid", "1").expect("audio on");

    player.engine.set_double("speed", 1.5).expect("speed");
    player.wait("speed", |model, _| {
        model.double(PropertyId::Speed) == Some(1.5)
    });
    assert_eq!(player.engine.get_double("speed"), Some(1.5));
    player
        .engine
        .set_double("sub-delay", -0.5)
        .expect("sub-delay");
    player.wait("sub-delay", |model, _| {
        model.double(PropertyId::SubDelay) == Some(-0.5)
    });
    player
        .engine
        .set_flag("keepaspect", false)
        .expect("keepaspect");
    assert_eq!(player.engine.get_text("keepaspect").as_deref(), Some("no"));
    assert!(player.engine.set_double("no-such-property", 1.0).is_err());
    assert_eq!(player.engine.get_text("no-such-property"), None);
    assert!(player.engine.command("no-such-command", &[]).is_err());

    // A-B loop points are plain properties too.
    player.engine.set_double("ab-loop-a", 10.0).expect("a");
    player.engine.set_double("ab-loop-b", 12.0).expect("b");
    player.wait("A-B", |model, _| {
        model.double(PropertyId::AbLoopA) == Some(10.0)
            && model.double(PropertyId::AbLoopB) == Some(12.0)
    });
    player.engine.set_text("ab-loop-a", "no").expect("clear a");
    player.wait("A cleared", |model, _| {
        model.double(PropertyId::AbLoopA).is_none()
    });
}

#[test]
fn per_file_options_reach_mpv_with_commas_intact() {
    let mut player = Player::new();
    player.events.clear();
    player
        .engine
        .load_with_options(
            SHORT,
            &[("force-media-title".to_owned(), "One, two=three".to_owned())],
        )
        .expect("loadfile with options");
    player.wait("file loaded", |_, events| {
        events
            .iter()
            .any(|event| matches!(event, EngineEvent::FileLoaded { .. }))
    });
    assert_eq!(
        player.engine.get_text("media-title").as_deref(),
        Some("One, two=three")
    );
    // A file loaded without them takes its own title again.
    player.load(LONG);
    assert_eq!(
        player.engine.get_text("media-title").as_deref(),
        Some("RVP fixture 60 s")
    );
    let (major, minor) = player.engine.version();
    assert!(
        major > 0 || minor >= 30,
        "mpv-version read: {major}.{minor}"
    );
    let protocols = player.engine.protocols();
    assert!(protocols.iter().any(|p| p == "file"), "{protocols:?}");
    println!("status protocols={}", protocols.join(","));
}

#[test]
fn screenshot_raw_gives_the_frames_pixels() {
    let mut player = Player::new();
    player.load(MARKER);
    player.wait("a frame shown", |model, _| {
        model
            .double(PropertyId::TimePos)
            .is_some_and(|time| time > 0.2)
    });
    let image = player.engine.screenshot_raw("video").expect("a frame");
    assert_eq!((image.width, image.height), (160, 90));
    assert!(image.stride >= 640, "{}", image.stride);
    let close = |at: (usize, usize), want: [u8; 3]| {
        let got = image.rgb(at.0, at.1).expect("inside");
        assert!(
            got.iter().zip(want).all(|(a, b)| a.abs_diff(b) < 40),
            "{at:?}: {got:?} for {want:?}"
        );
    };
    close((15, 12), [255, 255, 255]);
    close((60, 30), [0xDC, 0x28, 0x28]);
    close((120, 30), [0x28, 0xC8, 0x3C]);
    close((40, 70), [0x28, 0x46, 0xDC]);
    close((120, 70), [0xE6, 0xD2, 0x28]);
    assert_eq!(image.rgb(160, 0), None);
}
