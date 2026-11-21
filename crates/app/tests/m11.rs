//! M11's playback, playlist and opening features on the synthetic
//! fixtures, headless (no window, no sound): chapters and their skips,
//! the intro and the ending, subtitle and marked A-B sections, the scene
//! browsers' places, the queue and shuffle, the gap between two album
//! entries (the studies' measurement), the power actions' countdown
//! and timer (the app only asks; nothing powers off), the command line's
//! switches, a DVD folder made with dvdauthor
//! (`tools/test-clips/make-m11-fixtures.sh`), and the thumbnail core's
//! pictures as the app asks for them.
//!
//! `RVP_TESTDATA` points at `testdata/media` when the test runs elsewhere
//! (the Windows VM).

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rvp_app::instance::{Request, Switches};
use rvp_app::thumbs::ThumbEvent;
use rvp_app::{AbRepeat, Action, AfterPlayback, App, PowerAction, PowerTimer, Settings, UiRequest};
use rvp_engine::{Engine, EngineEvent, EngineOptions, PropertyId};

fn media(name: &str) -> String {
    let dir = std::env::var_os("RVP_TESTDATA").map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media")),
        PathBuf::from,
    );
    dir.join(name).to_string_lossy().into_owned()
}

struct Harness {
    app: App,
    engine: Engine,
    dir: PathBuf,
    requests: Vec<UiRequest>,
}

impl Harness {
    fn new(name: &str, change: impl FnOnce(&mut Settings)) -> Self {
        let dir = std::env::temp_dir().join(format!("rvp-m11-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let mut settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            ..Settings::default()
        };
        change(&mut settings);
        Self {
            app: App::new(settings, &dir).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
            dir,
            requests: Vec::new(),
        }
    }

    fn open(&mut self, files: &[String]) {
        self.app.open(&self.engine, files, false).expect("open");
    }

    /// Drains until `done` holds or 15 s pass; requests are kept.
    fn wait(&mut self, what: &str, done: impl Fn(&App) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !done(&self.app) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: path {:?} time {:?}",
                self.app.model.loaded_path(),
                self.app.model.double(PropertyId::TimePos)
            );
            std::thread::sleep(Duration::from_millis(5));
            let drained = self.app.drain(&self.engine, Instant::now());
            self.requests.extend(drained.requests);
        }
    }

    /// Seeks and waits for mpv's restart after it, so values of the
    /// place before (the time, the subtitle line) are not taken for it.
    fn seek(&mut self, seconds: f64) {
        self.engine
            .seek(rvp_engine::Seek::Absolute(seconds))
            .expect("seek");
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut restarted = false;
        while !restarted
            || !self
                .app
                .model
                .double(PropertyId::TimePos)
                .is_some_and(|time| (time - seconds).abs() < 0.2)
        {
            assert!(Instant::now() < deadline, "timed out seeking to {seconds}");
            std::thread::sleep(Duration::from_millis(5));
            let drained = self.app.drain(&self.engine, Instant::now());
            restarted |= drained
                .events
                .iter()
                .any(|event| matches!(event, EngineEvent::PlaybackRestart));
            self.requests.extend(drained.requests);
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn chapters_mark_the_seek_bar_and_listed_ones_are_skipped() {
    let mut h = Harness::new("chapters", |settings| {
        settings.playback.skip.enabled = true;
        settings.playback.skip.chapters = true;
        settings.playback.skip.intro = false;
        "Part A".clone_into(&mut settings.playback.skip.chapter_names);
    });
    h.open(&[media("fixture-chapters.mkv")]);
    h.wait("chapters", |app| app.seek_marks().len() == 3);
    let marks: Vec<f32> = h.app.seek_marks().iter().map(|mark| mark.at).collect();
    for (mark, at) in marks.iter().zip([0.25_f32, 0.5, 0.75]) {
        assert!((mark - at).abs() < 0.01, "{marks:?}");
    }
    // Into "Part A": on to "Part B".
    h.engine
        .seek(rvp_engine::Seek::Absolute(6.0))
        .expect("seek");
    h.wait("Part A skipped", |app| {
        app.model
            .double(PropertyId::TimePos)
            .is_some_and(|time| time >= 9.95)
    });
}

#[test]
fn keyframe_steps_and_the_jump_dialog_land_where_asked() {
    let mut h = Harness::new("keyframes", |_| {});
    h.open(&[media("fixture-chapters.mkv")]);
    h.wait("the file", |app| {
        app.model.double(PropertyId::Duration).is_some() && app.model.playing()
    });
    h.app.dispatch(&h.engine, Action::PlayPause);
    h.wait("paused", |app| !app.model.playing());
    // There, and done seeking: a relative keyframe step sent while mpv
    // still finishes the seek before it went to the file's end once on
    // CI's runner.
    let near = |wanted: f64| {
        move |app: &App| {
            app.model.flag(PropertyId::Seeking) == Some(false)
                && app
                    .model
                    .double(PropertyId::TimePos)
                    .is_some_and(|time| (time - wanted).abs() < 0.05)
        }
    };
    h.app
        .jump_to_text(&h.engine, "0:00:04.5", false)
        .expect("a time");
    h.wait("the time asked", near(4.5));
    // A keyframe every 2 s.
    h.app
        .dispatch(&h.engine, Action::JumpTo(rvp_app::Jump::NextKeyframe));
    h.wait("the next keyframe", near(6.0));
    h.app
        .dispatch(&h.engine, Action::JumpTo(rvp_app::Jump::PreviousKeyframe));
    h.wait("the keyframe before", near(4.0));
    // Frame 115 at 10 fps.
    h.app.jump_to_text(&h.engine, "115", true).expect("a frame");
    h.wait("the frame asked", near(11.5));
    assert!(h.app.jump_to_text(&h.engine, "1:75", false).is_err());
    // DVD Control > Speed > 1x Backward on a file: the time runs back at
    // about 1x, not in a jump.
    h.app.dispatch(&h.engine, Action::DvdSpeed(-1));
    let started = Instant::now();
    h.wait("backward play", |app| {
        app.model
            .double(PropertyId::TimePos)
            .is_some_and(|time| time < 10.5)
    });
    let took = started.elapsed();
    println!("metric backward_1s_ms={}", took.as_millis());
    assert!(took > Duration::from_millis(600), "{took:?}");
    h.app.dispatch(&h.engine, Action::DvdSpeed(1));
}

#[test]
fn the_intro_and_the_ending_are_skipped() {
    let mut h = Harness::new("ends", |settings| {
        settings.playback.skip.enabled = true;
        settings.playback.skip.intro = true;
        settings.playback.skip.intro_seconds = 2.0;
        settings.playback.skip.ending = true;
        settings.playback.skip.ending_seconds = 15.0;
    });
    h.open(&[media("fixture-chapters.mkv"), media("fixture-60s.mkv")]);
    h.wait("past the intro", |app| {
        app.model
            .double(PropertyId::TimePos)
            .is_some_and(|time| (1.9..4.0).contains(&time))
    });
    // 20 s long: the album moves on once 15 s are left.
    h.wait("the ending skipped", |app| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-60s.mkv"))
    });
}

#[test]
fn subtitle_lines_and_marked_sections_repeat() {
    let mut h = Harness::new("sections", |_| {});
    h.open(&[media("fixture-chapters.mkv")]);
    h.wait("loaded", |app| {
        app.model.double(PropertyId::Duration).is_some()
    });
    h.engine.set_pause(true).expect("pause");
    h.seek(11.5);
    h.wait("the line shows", |app| {
        app.model.double(PropertyId::SubStart).is_some()
    });
    // The third line (11 s to 12.5 s), from mpv's sub-start and sub-end.
    let notice = h
        .app
        .dispatch(&h.engine, Action::AbRepeat(AbRepeat::CurrentSubtitle));
    assert_eq!(
        notice,
        Some(UiRequest::Osd("A-B Repeat: A 0:00:11 B 0:00:13".to_owned()))
    );
    let session = h.app.session();
    assert_eq!((session.point_a, session.point_b), (Some(11.0), Some(12.5)));
    // Fine moves of 0.1 s.
    h.app
        .dispatch(&h.engine, Action::AbRepeat(AbRepeat::MoveBoth(true)));
    let session = h.app.session();
    assert!((session.point_a.unwrap_or(0.0) - 11.1).abs() < 1e-9);
    assert!((session.point_b.unwrap_or(0.0) - 12.6).abs() < 1e-9);
    // The A-B list: the section, and one more before it.
    h.app.section_edit(&h.engine, "add", None, "Third");
    h.app.dispatch(&h.engine, Action::AbRepeat(AbRepeat::Clear));
    h.app
        .dispatch(&h.engine, Action::AbRepeat(AbRepeat::Behind));
    h.app.section_edit(&h.engine, "add", None, "");
    let path = h.app.model.loaded_path().expect("plays").to_owned();
    let sections = h.app.marks().file(&path).sections;
    assert_eq!(sections.len(), 2, "{sections:?}");
    assert!((sections[0].start - 6.5).abs() < 0.2, "{sections:?}");
    h.app.dispatch(&h.engine, Action::AbRepeat(AbRepeat::Clear));
    // The marked section at 11.5 s, then the one before it.
    h.app
        .dispatch(&h.engine, Action::AbRepeat(AbRepeat::CurrentSection));
    assert_eq!(h.app.session().point_a, Some(sections[1].start));
    h.app
        .dispatch(&h.engine, Action::AbRepeat(AbRepeat::PreviousSection));
    assert_eq!(h.app.session().point_a, Some(sections[0].start));
    // The previous subtitle line, reached by a seek first.
    h.app.dispatch(&h.engine, Action::AbRepeat(AbRepeat::Clear));
    h.seek(11.5);
    h.wait("the line shows", |app| {
        app.model.double(PropertyId::SubStart) == Some(11.0)
    });
    h.app
        .dispatch(&h.engine, Action::AbRepeat(AbRepeat::PreviousSubtitle));
    h.wait("the line before", |app| app.session().point_a == Some(6.0));
    assert_eq!(h.app.session().point_b, Some(7.5));
}

#[test]
fn scene_browsers_list_times_markers_and_subtitle_lines() {
    let mut h = Harness::new("scenes", |settings| settings.scenes.interval = 5.0);
    h.open(&[media("fixture-chapters.mkv")]);
    h.wait("video", |app| {
        app.model.int(PropertyId::VideoWidth).is_some()
    });
    h.wait("chapters", |app| app.seek_marks().len() == 3);
    let browser = |h: &mut Harness, kind| match h.app.dispatch(&h.engine, Action::Scenes(kind)) {
        Some(UiRequest::SceneBrowser(browser)) => browser,
        other => panic!("{kind:?}: {other:?}"),
    };
    let times = |browser: &rvp_app::scenes::Browser| -> Vec<f64> {
        browser.scenes.iter().map(|scene| scene.time).collect()
    };
    assert_eq!(
        times(&browser(&mut h, rvp_app::Scenes::Time)),
        [0.0, 5.0, 10.0, 15.0]
    );
    h.app.add_bookmark(7.0, "Mine");
    let markers = browser(&mut h, rvp_app::Scenes::Markers);
    assert_eq!(times(&markers), [0.0, 5.0, 7.0, 10.0, 15.0]);
    assert_eq!(markers.scenes[2].label, "Mine");
    // The subtitle file next to the video (mpv's sub-auto).
    let lines = browser(&mut h, rvp_app::Scenes::Subtitles);
    assert_eq!(times(&lines), [1.0, 6.0, 11.0, 16.0], "{lines:?}");
    assert_eq!(lines.scenes[0].label, "First line");
}

#[test]
fn the_queue_comes_first_and_shuffle_plays_each_once() {
    let mut h = Harness::new("queue", |_| {});
    let files = [
        media("fixture-3s.mkv"),
        media("fixture-60s.mkv"),
        media("fixture-chapters.mkv"),
    ];
    h.open(&files);
    h.wait("first", |app| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-3s.mkv"))
    });
    let album = h.app.library().lists().shown;
    h.app.library_mut().queue_add(album, &[2]);
    h.app.dispatch(&h.engine, Action::Next);
    h.wait("the queued entry", |app| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-chapters.mkv"))
    });
    assert!(h.app.library().shown().queue.is_empty());
    // Shuffle: Next visits the other entries once each, then stops.
    h.app.dispatch(&h.engine, Action::ToggleShuffle);
    let mut seen = vec![h.app.model.loaded_path().unwrap_or_default().to_owned()];
    for _ in 0..2 {
        let before = h.app.model.loaded_path().unwrap_or_default().to_owned();
        h.app.dispatch(&h.engine, Action::Next);
        h.wait("the next shuffled entry", |app| {
            app.model
                .loaded_path()
                .is_some_and(|path| path != before && app.model.media_loaded())
        });
        seen.push(h.app.model.loaded_path().unwrap_or_default().to_owned());
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 3, "{seen:?}");
}

/// The gap seamless playback would close (`docs/spikes/m11-studies.md`):
/// from mpv's end of one entry to the next entry's first frame, with RVP
/// choosing the next entry and loading it with `loadfile replace`.
#[test]
fn the_next_entry_follows_the_end_of_a_file() {
    let mut h = Harness::new("gap", |_| {});
    h.open(&[media("fixture-3s.mkv"), media("fixture-chapters.mkv")]);
    h.wait("the first file", |app| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-3s.mkv"))
            && app.model.double(PropertyId::Duration).is_some()
    });
    h.seek(2.0);
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut ended = None;
    let gap = loop {
        assert!(Instant::now() < deadline, "the next entry did not start");
        std::thread::sleep(Duration::from_millis(1));
        let drained = h.app.drain(&h.engine, Instant::now());
        let second = h
            .app
            .model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-chapters.mkv"));
        if ended.is_none() && !second && h.app.model.flag(PropertyId::EofReached) == Some(true) {
            ended = Some(Instant::now());
        }
        let restarted = drained
            .events
            .iter()
            .any(|event| matches!(event, EngineEvent::PlaybackRestart));
        if let (Some(ended), true, true) = (ended, second, restarted) {
            break ended.elapsed();
        }
    };
    println!("metric next_entry_gap_ms={}", gap.as_millis());
    assert!(gap < Duration::from_secs(2), "{gap:?}");
}

#[test]
fn power_actions_count_down_and_the_timer_can_be_cancelled() {
    let mut h = Harness::new("power", |settings| settings.playback.finished.countdown = 5);
    h.open(&[media("fixture-3s.mkv")]);
    h.app
        .dispatch(&h.engine, Action::SetPowerAction(PowerAction::Hibernate));
    h.app.dispatch(
        &h.engine,
        Action::SetAfterPlayback(AfterPlayback::PowerAfterFile),
    );
    h.wait("the end", |app| app.power().countdown.is_some());
    assert!(h.requests.iter().any(|request| matches!(
        request,
        UiRequest::PowerNotice(Some((text, 5))) if text.contains("hibernate")
    )));
    let start = Instant::now();
    assert!(
        h.app
            .power_tick(start + Duration::from_secs(2))
            .iter()
            .all(|request| matches!(request, UiRequest::PowerNotice(Some(_)))),
        "still counting"
    );
    let ended = h.app.power_tick(start + Duration::from_secs(6));
    assert!(
        ended.contains(&UiRequest::Power(PowerAction::Hibernate)),
        "{ended:?}"
    );
    assert!(h.app.power().countdown.is_none());
    // The timer starts the notice; Cancel stops it.
    let now = Instant::now();
    h.app
        .set_power_timer(PowerTimer::In, 60, false, "1:00", now);
    assert!(h.app.power_tick(now + Duration::from_secs(30)).is_empty());
    let started = h.app.power_tick(now + Duration::from_secs(61));
    assert!(
        matches!(started.as_slice(), [UiRequest::PowerNotice(Some(_))]),
        "{started:?}"
    );
    assert_eq!(h.app.cancel_countdown(), Some(UiRequest::PowerNotice(None)));
    assert!(h.app.power_tick(now + Duration::from_secs(120)).is_empty());
    // A timer that exits RVP.
    h.app.set_power_timer(PowerTimer::At, 1, true, "now", now);
    assert_eq!(
        h.app.power_tick(now + Duration::from_secs(2)),
        [UiRequest::Exit]
    );
    // A timer fires a second from now at the soonest and at its limit at
    // the latest (so far ahead an `Instant` would not exist).
    let deadline = |app: &App| app.power().timer.as_ref().map(|timer| timer.deadline);
    h.app.set_power_timer(PowerTimer::In, 0, false, "0:00", now);
    assert_eq!(deadline(&h.app), Some(now + Duration::from_secs(1)));
    h.app
        .set_power_timer(PowerTimer::In, u64::MAX, false, "far", now);
    let limit = Duration::from_secs(u64::from(rvp_app::power::TIMER_LIMIT));
    assert_eq!(deadline(&h.app), Some(now + limit));
}

#[test]
fn command_line_switches_set_the_first_file() {
    let mut h = Harness::new("switches", |_| {});
    // The stream and the subtitle have mpv's path-list separator in their
    // names (`:` on Linux, `;` on Windows): joined into one list value,
    // each split in two and did not load (the Linux audit's M1).
    let separator = if cfg!(windows) { ";" } else { ":" };
    let folder = std::env::temp_dir().join(format!("rvp-m11-switches-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("folder");
    let stream = folder.join(format!("same{separator} stream.mkv"));
    let subtitle = folder.join(format!("sub{separator} title.srt"));
    std::fs::copy(media("fixture-3s.mkv"), &stream).expect("stream");
    std::fs::copy(media("fixture-chapters.srt"), &subtitle).expect("subtitle");
    let mut request = Request {
        mode: rvp_app::instance::Mode::Play,
        files: vec![
            media("fixture-chapters.mkv"),
            stream.to_string_lossy().into_owned(),
        ],
        switches: Switches {
            seek: Some("0:00:08".to_owned()),
            title: Some("Switched".to_owned()),
            sub: Some(subtitle.to_string_lossy().into_owned()),
            same: true,
            ..Switches::default()
        },
    };
    h.app.prepare_switches(&mut request);
    // `/same`: the second file is the first's stream, not an entry.
    assert_eq!(request.files.len(), 1);
    h.app.start(&h.engine, &request.files).expect("start");
    h.wait("started at 8 s", |app| {
        app.model
            .double(PropertyId::TimePos)
            .is_some_and(|time| (7.5..9.5).contains(&time))
    });
    assert_eq!(
        h.engine.get_text("media-title").as_deref(),
        Some("Switched")
    );
    let audio = h
        .engine
        .tracks()
        .iter()
        .filter(|track| track.kind == rvp_engine::TrackKind::Audio)
        .count();
    assert_eq!(audio, 2, "the second file's audio joined");
    // The /sub file itself (mpv may also load the fixture's own .srt).
    let tracks = h.engine.tracks();
    assert!(
        tracks
            .iter()
            .any(|track| track.kind == rvp_engine::TrackKind::Subtitle
                && track.external
                && track
                    .title
                    .as_deref()
                    .is_some_and(|title| title.contains("title.srt"))),
        "the /sub file loaded: {tracks:?}"
    );
    assert_eq!(h.app.library().shown().entries.len(), 1);
    drop(std::fs::remove_dir_all(&folder));
}

#[test]
fn a_dvd_folder_plays_its_titles() {
    let mut h = Harness::new("dvd", |_| {});
    let dvd = PathBuf::from(media("dvd"));
    if !h
        .engine
        .protocols()
        .iter()
        .any(|protocol| protocol == "dvd")
    {
        println!("note: this libmpv has no DVD support");
        return;
    }
    h.app.open_disc(&h.engine, &dvd, false);
    h.wait("the DVD", |app| {
        app.model.media_loaded() && app.model.double(PropertyId::Duration).is_some()
    });
    let path = h.app.model.loaded_path().expect("plays").to_owned();
    assert!(path.starts_with("dvd://"), "{path}");
    let titles = h.app.disc_titles();
    assert_eq!(titles.len(), 2, "{titles:?}");
    assert_eq!(titles[0].0, "Title 1 (2 chapters)");
    assert_eq!(h.app.disc_angles(), 1);
    // The longest title: 8 s with chapters at 0 and 4 s.
    let duration = h.app.model.double(PropertyId::Duration).unwrap_or(0.0);
    assert!((duration - 8.0).abs() < 0.5, "{duration}");
    assert_eq!(h.engine.chapters().len(), 2);
    // From the .IFO too.
    let ifo = dvd.join("VIDEO_TS").join("VIDEO_TS.IFO");
    assert_eq!(rvp_app::sources::disc_address(&ifo, false), path);
    // The second title (6 s, one chapter) in the same album entry; mpv
    // counts titles from 0.
    h.app.play_disc_title(&h.engine, 1);
    h.wait("the second title", |app| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.starts_with("dvd://1/"))
            && app
                .model
                .double(PropertyId::Duration)
                .is_some_and(|duration| (duration - 6.0).abs() < 0.5)
    });
    assert_eq!(h.engine.chapters().len(), 1);
    assert_eq!(h.app.library().shown().entries.len(), 1);
    // mpv's backward play stalls on a disc's stream: refused there.
    assert_eq!(
        h.app.dispatch(&h.engine, Action::DvdSpeed(-1)),
        Some(UiRequest::Osd("Backward play: not on discs".to_owned()))
    );
}

#[test]
fn the_thumbnail_core_answers_the_apps_requests() {
    let mut h = Harness::new("thumbs", |_| {});
    let (sender, receiver) = mpsc::channel();
    let notify = std::sync::Mutex::new(sender);
    h.app
        .start_thumbnails(
            rvp_engine::thumbnail::ThumbnailOptions::default(),
            move || {
                if let Ok(sender) = notify.lock() {
                    let _ = sender.send(());
                }
            },
        )
        .expect("thumbnails");
    let path = media("fixture-chapters.mkv");
    h.app
        .request_scenes(&path, &[(0, 2.0), (1, 12.0)], (160, 90));
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut scenes = Vec::new();
    while scenes.len() < 2 {
        assert!(Instant::now() < deadline, "scenes: {scenes:?}");
        let _ = receiver.recv_timeout(Duration::from_millis(100));
        for event in h.app.thumbnail_events() {
            if let ThumbEvent::Scene { index, picture } = event {
                assert_eq!(
                    (picture.width, picture.height),
                    (160, 90),
                    "boxes are filled"
                );
                scenes.push(index);
            }
        }
    }
    scenes.sort_unstable();
    assert_eq!(scenes, [0, 1]);
    let stats = h.app.thumbnail_stats().expect("running");
    assert_ne!(stats.render_thread, Some(std::thread::current().id()));
    println!(
        "metric app_scene_ms={}",
        stats.last_render.unwrap_or_default().as_millis()
    );
}
