//! Input binding tests (docs/PLAN.md section 7): every default binding
//! triggers its action, checked against the real engine (headless).

use std::time::{Duration, Instant};

use rvp_app::playlist::local_path;
use rvp_app::{Action, App, Bindings, Key, KeyChord, MouseGesture, SeekStep, Settings, UiRequest};
use rvp_engine::{Engine, EngineOptions, PlayerModel, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);
const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);

fn chord(text: &str) -> KeyChord {
    KeyChord::parse(text).expect("valid key")
}

#[test]
fn potplayer_default_keys() {
    let bindings = Bindings::potplayer_defaults();
    for (key, action) in [
        ("Space", Action::PlayPause),
        ("Enter", Action::ToggleFullscreen),
        ("Alt+Enter", Action::ToggleFullscreen),
        ("Ctrl+Enter", Action::ToggleFullscreenStretch),
        ("Up", Action::VolumeUp),
        ("Down", Action::VolumeDown),
        ("M", Action::ToggleMute),
        ("Left", Action::SeekBackward(SeekStep::Short)),
        ("Right", Action::SeekForward(SeekStep::Short)),
        ("Ctrl+Left", Action::SeekBackward(SeekStep::Medium)),
        ("Shift+Right", Action::SeekForward(SeekStep::Long)),
        ("Ctrl+Alt+Left", Action::SeekBackward(SeekStep::VeryLong)),
        ("PgUp", Action::Previous),
        ("PgDn", Action::Next),
        ("F3", Action::OpenFiles),
        ("Ctrl+O", Action::OpenFiles),
        ("F4", Action::CloseMedia),
        ("F6", Action::TogglePlaylist),
        ("Alt+H", Action::ToggleSubtitles),
    ] {
        assert_eq!(bindings.key(&chord(key)), Some(action), "{key}");
    }
    // Alt+F4 is not an accelerator: the window system closes the window.
    assert_eq!(bindings.key(&chord("Alt+F4")), None);
    assert_eq!(
        bindings.key_for(Action::PlayPause),
        Some(KeyChord::plain(Key::Space))
    );
    assert_eq!(
        bindings.mouse(MouseGesture::LeftClick),
        Some(Action::PlayPause)
    );
    assert_eq!(
        bindings.mouse(MouseGesture::LeftDoubleClick),
        Some(Action::ToggleFullscreen)
    );
    assert_eq!(
        bindings.mouse(MouseGesture::WheelUp),
        Some(Action::VolumeUp)
    );
    assert_eq!(
        bindings.mouse(MouseGesture::X1Click),
        Some(Action::SeekBackward(SeekStep::Short))
    );
    assert!(bindings.left_click_video_only);
}

/// Whether `path` plays (mpv reports it in [`local_path`]'s form).
fn is_file(model: &PlayerModel, path: &str) -> bool {
    model.loaded_path() == Some(local_path(path).as_str())
}

fn audio_filters(model: &PlayerModel) -> &str {
    model.text(PropertyId::AudioFilters).unwrap_or_default()
}

struct Harness {
    app: App,
    engine: Engine,
}

impl Harness {
    fn new() -> Self {
        let dir =
            std::env::temp_dir().join(format!("rvp-bindings-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&dir);
        let settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ..Settings::default()
        };
        Self {
            app: App::new(settings, &dir).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
        }
    }

    fn wait(&mut self, what: &str, done: impl Fn(&PlayerModel) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done(&self.app.model) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: loaded {:?}, path {:?}",
                self.app.model.loaded_path(),
                self.app.model.text(PropertyId::Path)
            );
            std::thread::sleep(Duration::from_millis(10));
            self.app.drain(&self.engine, Instant::now());
        }
    }

    fn time(&self) -> f64 {
        self.app
            .model
            .double(PropertyId::TimePos)
            .unwrap_or_default()
    }

    fn dispatch(&mut self, action: Action) -> Option<UiRequest> {
        self.app.dispatch(&self.engine, action)
    }

    /// Carries out `action` once and waits for its expected effect. The
    /// player starts and ends paused at 30 s of the long fixture, at 50 %.
    fn check(&mut self, action: Action) {
        match action {
            Action::PlayPause => {
                assert_eq!(self.dispatch(action), None);
                self.wait("playing", PlayerModel::playing);
                self.dispatch(action);
                self.wait("paused", |m| m.flag(PropertyId::Pause) == Some(true));
            }
            Action::SeekBackward(step) | Action::SeekForward(step) => {
                // Only the short step fits the 60 s fixture both ways.
                if step == SeekStep::Short {
                    let before = self.time();
                    self.dispatch(action);
                    let expected = if matches!(action, Action::SeekForward(_)) {
                        before + 5.0
                    } else {
                        before - 5.0
                    };
                    self.wait("seek step", |m| {
                        (m.double(PropertyId::TimePos).unwrap_or_default() - expected).abs() < 0.3
                    });
                } else {
                    let before = self.time();
                    self.dispatch(action);
                    self.wait("long seek", |m| {
                        (m.double(PropertyId::TimePos).unwrap_or_default() - before).abs() > 4.0
                    });
                    self.engine
                        .seek(rvp_engine::Seek::Absolute(30.0))
                        .expect("seek back");
                    self.wait("back at 30 s", |m| {
                        (m.double(PropertyId::TimePos).unwrap_or_default() - 30.0).abs() < 0.2
                    });
                }
            }
            Action::VolumeUp => {
                self.dispatch(action);
                self.wait("volume up", |m| m.double(PropertyId::Volume) == Some(51.0));
            }
            Action::VolumeDown => {
                self.dispatch(action);
                self.wait("volume down", |m| {
                    m.double(PropertyId::Volume) == Some(50.0)
                });
            }
            Action::ToggleMute => {
                self.dispatch(action);
                self.wait("muted", |m| m.flag(PropertyId::Mute) == Some(true));
                self.dispatch(action);
                self.wait("unmuted", |m| m.flag(PropertyId::Mute) == Some(false));
            }
            // RVP's album is the playlist; mpv plays one entry at a time.
            Action::Next => {
                self.dispatch(action);
                self.wait("next file", |m| is_file(m, SHORT));
            }
            Action::Previous => {
                self.dispatch(Action::Next);
                self.wait("next file", |m| is_file(m, SHORT));
                self.dispatch(action);
                self.wait("previous file", |m| is_file(m, LONG));
            }
            Action::ToggleSubtitles => {
                let before = self.app.model.flag(PropertyId::SubVisibility);
                self.dispatch(action);
                self.wait("subtitles toggled", |m| {
                    m.flag(PropertyId::SubVisibility) == before.map(|on| !on)
                });
                self.dispatch(action);
                self.wait("subtitles back", |m| {
                    m.flag(PropertyId::SubVisibility) == before
                });
            }
            Action::CloseMedia => {}
            _ => self.check_request(action),
        }
    }

    /// Actions that ask the window for something or change the normalizer.
    fn check_request(&mut self, action: Action) {
        match action {
            Action::OpenFiles => assert_eq!(self.dispatch(action), Some(UiRequest::OpenFileDialog)),
            Action::Exit => assert_eq!(self.dispatch(action), Some(UiRequest::Exit)),
            Action::ToggleFullscreen => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ToggleFullscreen { stretch: false })
                );
            }
            Action::ToggleFullscreenStretch => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ToggleFullscreen { stretch: true })
                );
            }
            Action::ContextMenu => assert_eq!(self.dispatch(action), Some(UiRequest::ContextMenu)),
            Action::Minimize => assert_eq!(self.dispatch(action), Some(UiRequest::Minimize)),
            Action::MaximizeRestore => {
                assert_eq!(self.dispatch(action), Some(UiRequest::MaximizeRestore));
            }
            Action::TogglePlaylist => {
                assert_eq!(self.dispatch(action), Some(UiRequest::TogglePlaylist));
            }
            Action::ToggleNormalizer => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Osd("Normalizer: Off, Boost: On".into()))
                );
                self.wait("normalizer off", |m| !audio_filters(m).contains("rvpnorm"));
                self.dispatch(action);
                self.wait("normalizer on", |m| audio_filters(m).contains("rvpnorm"));
            }
            Action::ToggleBoost => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Osd("Normalizer: On, Boost: Off".into()))
                );
                self.wait("boost off", |m| audio_filters(m).contains("m=1"));
                self.dispatch(action);
                self.wait("boost on", |m| audio_filters(m).contains("m=5"));
            }
            Action::KeysAndMouse => {
                assert_eq!(self.dispatch(action), Some(UiRequest::KeysAndMouse));
            }
            Action::ImportPlaylists => {
                assert_eq!(self.dispatch(action), Some(UiRequest::ImportPlaylists));
            }
            _ => unreachable!("{action:?} is checked in check()"),
        }
    }
}

/// Every action that some default key or mouse gesture is bound to, carried
/// out once with its expected effect.
#[test]
fn every_default_binding_triggers_its_action() {
    let bindings = Bindings::potplayer_defaults();
    let mut bound: Vec<Action> = bindings
        .keys
        .values()
        .chain(bindings.mouse.values())
        .copied()
        .collect();
    bound.sort();
    bound.dedup();
    // Playlist moves and Close Playback change the file: do them last.
    let changes_file =
        |a: &Action| matches!(a, Action::Previous | Action::Next | Action::CloseMedia);
    let (late, early): (Vec<Action>, Vec<Action>) = bound.into_iter().partition(changes_file);

    let mut h = Harness::new();
    h.app
        .open(&h.engine, &[LONG.to_owned(), SHORT.to_owned()], false)
        .expect("open");
    h.wait("loaded", |m| {
        m.media_loaded() && m.double(PropertyId::Duration).is_some()
    });
    h.app.apply_normalizer(&h.engine);
    h.engine.set_pause(true).expect("pause");
    h.engine.set_volume(50.0).expect("volume");
    h.wait("paused at 50 %", |m| {
        m.flag(PropertyId::Pause) == Some(true) && m.double(PropertyId::Volume) == Some(50.0)
    });
    h.engine
        .seek(rvp_engine::Seek::Absolute(30.0))
        .expect("seek");
    h.wait("at 30 s", |m| {
        (m.double(PropertyId::TimePos).unwrap_or_default() - 30.0).abs() < 0.2
    });

    for action in early.into_iter().chain(late) {
        h.check(action);
    }
    // Close Playback last: it unloads the file.
    h.dispatch(Action::CloseMedia);
    h.wait("idle", |m| !m.media_loaded());
    // With nothing loaded, Play plays the album's selected entry ...
    assert_eq!(h.dispatch(Action::PlayPause), None);
    h.wait("playing again", PlayerModel::media_loaded);
    h.dispatch(Action::CloseMedia);
    h.wait("idle again", |m| !m.media_loaded());
    // ... and asks for files when the album is empty.
    h.app
        .list_action(&h.engine, rvp_app::ListAction::RemoveAll, false, 10);
    assert_eq!(
        h.dispatch(Action::PlayPause),
        Some(UiRequest::OpenFileDialog)
    );
}

#[test]
fn menu_shows_potplayer_texts_and_current_keys() {
    let dir = std::env::temp_dir().join(format!("rvp-menu-{}", std::process::id()));
    let app = App::new(Settings::default(), &dir).expect("app");
    let entry = |action| app.menu_entry(action);
    assert_eq!(
        entry(Action::SeekBackward(SeekStep::Medium)),
        ("30 Sec Backward".to_owned(), "Ctrl+Left".to_owned())
    );
    assert_eq!(
        entry(Action::SeekForward(SeekStep::VeryLong)),
        ("5 Min Forward".to_owned(), "Ctrl+Alt+Right".to_owned())
    );
    assert_eq!(
        entry(Action::ToggleFullscreen),
        ("Fullscreen (Keep AR)".to_owned(), "Alt+Enter".to_owned())
    );
    assert_eq!(
        entry(Action::Exit),
        ("Exit".to_owned(), "Alt+F4".to_owned())
    );
    // With PotPlayer's key unbound, the menu shows another bound key.
    let mut settings = Settings::default();
    settings.keys.insert("Alt+Enter".into(), String::new());
    let rebound = App::new(settings, &dir).expect("app");
    assert_eq!(rebound.menu_entry(Action::ToggleFullscreen).1, "Enter");
    let _ = std::fs::remove_dir_all(&dir);
}
