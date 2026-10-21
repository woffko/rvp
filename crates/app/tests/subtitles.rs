//! M13's subtitles on a headless player (no window, no sound): the style
//! and placement settings as mpv's options, and RVP's own work on subtitle
//! files: the browser's edits shown at once, saving with the sync and the
//! frame rate applied, sync changes from the next subtitle on, typed
//! subtitles (Input Subtitles) and edits kept when another file starts.
//! The subtitle files are generated here; the video is a copy of the 60 s
//! fixture in the test's folder, so files named after it stay there.
//!
//! `RVP_TESTDATA` points at `testdata/media` when the test runs elsewhere
//! (the Windows VM).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::subtitles::SubtitleCommand;
use rvp_app::{Action, App, Delay, Settings, UiRequest};
use rvp_engine::{Engine, EngineOptions, PropertyId};
use serde_json::Value;

fn media(name: &str) -> PathBuf {
    let dir = std::env::var_os("RVP_TESTDATA").map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media")),
        PathBuf::from,
    );
    dir.join(name)
}

/// Three subtitles around 30 s, where the tests stand.
const SRT: &str = "1\r\n00:00:29,000 --> 00:00:31,000\r\nfirst line\r\n\r\n\
                   2\r\n00:00:33,000 --> 00:00:35,000\r\nsecond line\r\n\r\n\
                   3\r\n00:00:40,000 --> 00:00:42,000\r\nthird line\r\n";

struct Harness {
    app: App,
    engine: Engine,
    dir: PathBuf,
    video: PathBuf,
    requests: Vec<UiRequest>,
}

impl Harness {
    /// A player paused at 30 s of a copy of the 60 s fixture.
    fn new(name: &str, change: impl FnOnce(&mut Settings)) -> Self {
        let dir = std::env::temp_dir().join(format!("rvp-m13-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("state")).expect("dir");
        let video = dir.join("clip.mkv");
        std::fs::copy(media("fixture-60s.mkv"), &video).expect("copy the fixture");
        let mut settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            // Only the subtitles a test adds.
            subtitle_auto: "no".to_owned(),
            ..Settings::default()
        };
        change(&mut settings);
        let mut harness = Self {
            app: App::new(settings, &dir.join("state")).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
            dir,
            video,
            requests: Vec::new(),
        };
        let file = harness.video.to_string_lossy().into_owned();
        harness
            .app
            .open(&harness.engine, &[file], false)
            .expect("open");
        harness.wait("the video", |app, _| {
            app.model.media_loaded()
                && app
                    .model
                    .double(PropertyId::Duration)
                    .is_some_and(|duration| duration > 30.0)
        });
        harness.engine.set_pause(true).expect("pause");
        // A seek while the file still starts can be refused: again.
        harness.wait("a seek", |_, engine| {
            engine.seek(rvp_engine::Seek::Absolute(30.0)).is_ok()
        });
        harness.wait("30 s", |app, _| {
            app.model
                .double(PropertyId::TimePos)
                .is_some_and(|time| (time - 30.0).abs() < 0.05)
                && app.model.flag(PropertyId::Seeking) == Some(false)
        });
        harness
    }

    /// Drains until `done` holds or 15 s pass; requests are kept.
    fn wait(&mut self, what: &str, done: impl Fn(&App, &Engine) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !done(&self.app, &self.engine) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: time {:?}, sub-text {:?}",
                self.app.model.double(PropertyId::TimePos),
                self.engine.get_text("sub-text")
            );
            std::thread::sleep(Duration::from_millis(5));
            let drained = self.app.drain(&self.engine, Instant::now());
            self.requests.extend(drained.requests);
        }
    }

    /// Writes `text` into the test's folder and shows it.
    fn add_subtitle(&mut self, name: &str, text: &str) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, text).expect("write the subtitles");
        let request = self.app.load_subtitle(&self.engine, &path);
        assert!(matches!(request, Some(UiRequest::Osd(_))), "{request:?}");
        path
    }

    fn sub_text(&mut self, what: &str, text: &str) {
        self.wait(what, |_, engine| {
            engine.get_text("sub-text").as_deref() == Some(text)
        });
    }

    fn browser(&mut self) -> Value {
        serde_json::from_str(&self.app.subtitle_browser_json(&self.engine)).expect("json")
    }

    fn edit(&mut self, op: &str, index: i64, text: &str) -> Option<UiRequest> {
        self.app.subtitle_edit(&self.engine, op, index, text)
    }

    fn set_delay(&mut self, seconds: f64) {
        self.engine
            .set_double("sub-delay", seconds)
            .expect("sub-delay");
        self.wait("the sync", |app, _| {
            app.model
                .double(PropertyId::SubDelay)
                .is_some_and(|delay| (delay - seconds).abs() < 1e-6)
        });
    }

    /// The file of the subtitle track that shows.
    fn shown_file(&self) -> Option<String> {
        let sid: i64 = self.engine.get_text("sid")?.parse().ok()?;
        let list = self.engine.get_json("track-list")?;
        list.as_array()?
            .iter()
            .find(|track| track["type"] == "sub" && track["id"].as_i64() == Some(sid))
            .and_then(|track| track["external-filename"].as_str().map(str::to_owned))
    }

    fn state(&self, name: &str) -> PathBuf {
        self.dir.join("state").join("subtitles").join(name)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read")
}

#[test]
fn style_and_placement_settings_become_mpvs_options() {
    let mut h = Harness::new("style", |_| {});
    let mut settings = h.app.settings.clone();
    let subtitles = &mut settings.subtitles;
    subtitles.bold = true;
    subtitles.color = Some("#FFFFFF00".to_owned());
    subtitles.outline_size = Some(4.0);
    subtitles.align = "left".to_owned();
    subtitles.margin_x = Some(60);
    subtitles.position = Some(80.0);
    subtitles.bitmap_outside = true;
    subtitles.text_inside = true;
    subtitles.remove_sdh = true;
    subtitles.ass_override = "force".to_owned();
    subtitles.override_color = true;
    subtitles.scale_x = Some(120.0);
    subtitles.show = false;
    subtitles.encoding = "windows-1251".to_owned();
    let applied = h.app.apply_settings(&h.engine, settings);
    assert!(applied.notes.is_empty(), "{:?}", applied.notes);
    let option = |engine: &Engine, name: &str| engine.get_text(name).unwrap_or_default();
    assert_eq!(option(&h.engine, "sub-bold"), "yes");
    assert_eq!(option(&h.engine, "sub-align-x"), "left");
    assert_eq!(option(&h.engine, "sub-margin-x"), "60");
    assert_eq!(h.engine.get_double("sub-pos"), Some(80.0));
    assert_eq!(option(&h.engine, "stretch-image-subs-to-screen"), "yes");
    assert_eq!(option(&h.engine, "sub-use-margins"), "no");
    assert_eq!(option(&h.engine, "sub-filter-sdh"), "yes");
    assert_eq!(option(&h.engine, "sub-ass-override"), "force");
    assert_eq!(option(&h.engine, "sub-visibility"), "no");
    assert_eq!(option(&h.engine, "sub-codepage"), "windows-1251");
    // mpv shows colours as #AARRGGBB too.
    assert_eq!(option(&h.engine, "sub-color").to_uppercase(), "#FFFFFF00");
    let outline = h
        .engine
        .get_double("sub-outline-size")
        .or_else(|| h.engine.get_double("sub-border-size"));
    assert_eq!(outline, Some(4.0));
    assert_eq!(
        option(&h.engine, "sub-ass-style-overrides"),
        "PrimaryColour=&H0000FFFF,ScaleX=120"
    );
    // Under the video: margins on and forced, at the bottom.
    let mut settings = h.app.settings.clone();
    settings.subtitles.under_video = true;
    h.app.apply_settings(&h.engine, settings);
    assert_eq!(option(&h.engine, "sub-use-margins"), "yes");
    assert_eq!(option(&h.engine, "sub-ass-force-margins"), "yes");
    assert_eq!(h.engine.get_double("sub-pos"), Some(100.0));
}

#[test]
fn edits_show_at_once_and_save_writes_the_sync_into_the_file() {
    let mut h = Harness::new("edit", |_| {});
    let file = h.add_subtitle("clip.srt", SRT);
    h.sub_text("the first subtitle", "first line");
    let browser = h.browser();
    assert_eq!(browser["error"], "");
    assert_eq!(browser["name"], "clip.srt");
    assert_eq!(browser["cues"].as_array().map(Vec::len), Some(3));
    assert_eq!(h.app.subtitle_cue_now(), 0);

    // An edit shows at once, through RVP's working copy.
    assert_eq!(h.edit("text", 0, "edited line"), None);
    h.sub_text("the edited subtitle", "edited line");
    assert!(h.browser()["dirty"].as_bool().unwrap_or_default());
    assert_eq!(
        h.shown_file(),
        Some(h.state("editing.srt").to_string_lossy().into_owned())
    );
    assert_eq!(h.edit("undo", -1, ""), None);
    h.sub_text("the undone edit", "first line");
    assert_eq!(h.edit("redo", -1, ""), None);
    h.sub_text("the edit again", "edited line");

    // Save: the times as they show (+1.5 s), into the file mpv shows
    // again, with the old file kept and the sync back at none.
    h.set_delay(1.5);
    match h.edit("save", -1, "") {
        Some(UiRequest::Osd(text)) => {
            assert_eq!(text, "Saved clip.srt with the sync (+1.5 s)");
        }
        other => panic!("{other:?}"),
    }
    let saved = read(&file);
    assert!(
        saved.starts_with("1\r\n00:00:30,500 --> 00:00:32,500\r\nedited line\r\n"),
        "{saved}"
    );
    assert!(
        saved.contains("00:00:34,500 --> 00:00:36,500\r\nsecond line"),
        "{saved}"
    );
    assert_eq!(read(&h.state("backup/clip.srt")), SRT);
    assert_eq!(h.engine.get_double("sub-delay"), Some(0.0));
    assert_eq!(h.shown_file(), Some(file.to_string_lossy().into_owned()));
    assert!(!h.browser()["dirty"].as_bool().unwrap_or(true));
    // 30.5 s: the saved subtitle shows without a sync.
    h.engine
        .seek(rvp_engine::Seek::Absolute(31.0))
        .expect("seek");
    h.sub_text("the saved subtitle", "edited line");
}

#[test]
fn the_frame_rate_conversion_is_saved_with_the_times() {
    let mut h = Harness::new("fps", |_| {});
    h.add_subtitle("clip.srt", SRT);
    h.sub_text("the first subtitle", "first line");
    let video = h
        .app
        .model
        .double(PropertyId::ContainerFps)
        .expect("the fixture's rate");
    // Made for 2 x the video's rate: shown at twice the times.
    h.app.set_subtitle_fps(&h.engine, Some(video * 2.0));
    h.browser();
    let target = h.dir.join("converted.srt");
    match h.app.save_subtitle_as(&h.engine, &target) {
        UiRequest::Osd(text) => assert_eq!(text, "Saved converted.srt"),
        other => panic!("{other:?}"),
    }
    let saved = read(&target);
    assert!(
        saved.starts_with("1\r\n00:00:58,000 --> 00:01:02,000\r\nfirst line"),
        "{saved}"
    );
    // Saved elsewhere: the track and its conversion stay.
    assert!(h.app.subtitle_fps().is_some());
}

#[test]
fn sync_steps_from_the_next_subtitle_leave_the_earlier_ones() {
    let mut h = Harness::new("next", |settings| {
        settings.subtitles.sync_from_next = true;
        settings.subtitle_delay_step = 0.5;
    });
    h.add_subtitle("clip.srt", SRT);
    h.sub_text("the first subtitle", "first line");
    match h
        .app
        .dispatch(&h.engine, Action::SubtitleDelay(Delay::Later))
    {
        Some(UiRequest::Osd(text)) => {
            assert_eq!(text, "Subtitle sync from the next subtitle: +500 ms");
        }
        other => panic!("{other:?}"),
    }
    let browser = h.browser();
    let starts: Vec<i64> = browser["cues"]
        .as_array()
        .expect("cues")
        .iter()
        .filter_map(|cue| cue["start"].as_i64())
        .collect();
    assert_eq!(starts, [29_000, 33_500, 40_500]);
    assert_eq!(h.engine.get_double("sub-delay"), Some(0.0));
    // The third step too, and the menu shows the option checked.
    match h.app.dispatch(
        &h.engine,
        Action::Subtitles(SubtitleCommand::DelayBy(Delay::Earlier, 3)),
    ) {
        Some(UiRequest::Osd(text)) => {
            assert_eq!(text, "Subtitle sync from the next subtitle: -10.0 s");
        }
        other => panic!("{other:?}"),
    }
    let browser = h.browser();
    let starts: Vec<i64> = browser["cues"]
        .as_array()
        .expect("cues")
        .iter()
        .filter_map(|cue| cue["start"].as_i64())
        .collect();
    // The later two moved 10 s back; the second now comes first.
    assert_eq!(starts, [23_500, 29_000, 30_500]);
    assert_eq!(browser["cues"][0]["text"], "second line");
    assert_eq!(
        h.app
            .checked(Action::Subtitles(SubtitleCommand::SyncFromNext)),
        Some(true)
    );
}

#[test]
fn typed_subtitles_show_and_are_saved_as_a_new_file() {
    let mut h = Harness::new("input", |_| {});
    assert_eq!(h.browser()["error"], "No subtitle shows");
    assert_eq!(h.edit("input-begin", -1, ""), None);
    match h.edit("input", -1, "typed\nline") {
        Some(UiRequest::Osd(text)) => assert_eq!(text, "Subtitle added at 00:00:30.000"),
        other => panic!("{other:?}"),
    }
    h.sub_text("the typed subtitle", "typed\nline");
    assert_eq!(h.browser()["name"], "Typed subtitles");
    // Save names the video's file and shows it.
    match h.app.dispatch(
        &h.engine,
        Action::Subtitles(SubtitleCommand::SaveAsVideoName),
    ) {
        Some(UiRequest::Osd(text)) => assert_eq!(text, "Saved clip.srt"),
        other => panic!("{other:?}"),
    }
    let saved = h.dir.join("clip.srt");
    assert_eq!(
        read(&saved),
        "1\r\n00:00:30,000 --> 00:00:33,000\r\ntyped\r\nline\r\n\r\n"
    );
    assert_eq!(h.shown_file(), Some(saved.to_string_lossy().into_owned()));
    assert_eq!(h.browser()["name"], "clip.srt");
}

#[test]
fn unsaved_edits_are_kept_when_another_file_starts() {
    let mut h = Harness::new("unsaved", |_| {});
    h.add_subtitle("clip.srt", SRT);
    h.sub_text("the first subtitle", "first line");
    h.browser();
    assert_eq!(h.edit("delete", 2, ""), None);
    let other = h.dir.join("other.mkv");
    std::fs::copy(media("fixture-3s.mkv"), &other).expect("copy");
    h.app
        .open(&h.engine, &[other.to_string_lossy().into_owned()], false)
        .expect("open");
    let kept = h.state("unsaved-clip.srt");
    let message = format!("Unsaved subtitle edits are kept in {}", kept.display());
    h.wait("the next file", |app, _| {
        app.model
            .loaded_path()
            .is_some_and(|path| path.ends_with("other.mkv"))
    });
    assert!(
        h.requests
            .iter()
            .any(|request| *request == UiRequest::Message(message.clone())),
        "{:?}",
        h.requests
    );
    let unsaved = read(&kept);
    assert!(unsaved.contains("second line") && !unsaved.contains("third line"));
}
