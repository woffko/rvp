//! The media controls' view of playback and their commands (M8: MPRIS,
//! Windows' media controls), and whether the screen stays on, on headless
//! libmpv: no window, no sound.

use std::time::{Duration, Instant};

use rvp_app::media::{MediaCommand, Status};
use rvp_app::{Action, App, Settings, UiRequest};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

fn drain_until(app: &mut App, engine: &Engine, what: &str, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(app) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
        app.drain(engine, Instant::now());
    }
}

#[test]
fn the_media_controls_see_and_steer_playback() {
    let dir = std::env::temp_dir().join(format!("rvp-media-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ..Settings::default()
    };
    let mut app = App::new(settings, &dir).expect("app");
    let idle = app.media_state();
    assert_eq!(idle.status, Status::Stopped);
    assert!(!idle.can_play && idle.url.is_none());

    app.open(&engine, &[LONG.to_owned()], false).expect("open");
    drain_until(&mut app, &engine, "playback", |app| {
        app.media_state().status == Status::Playing && app.media_state().position > 0.3
    });
    let state = app.media_state();
    assert_eq!(state.track, 1);
    assert_eq!(state.title, "fixture-60s.mkv");
    assert!(
        state
            .url
            .as_deref()
            .is_some_and(|url| url.starts_with("file:///") && url.ends_with("/fixture-60s.mkv")),
        "{state:?}"
    );
    assert!(
        state
            .length
            .is_some_and(|length| (length - 60.0).abs() < 1.0)
    );
    assert!(state.can_seek && state.can_play && !state.can_next && !state.can_previous);
    drain_until(&mut app, &engine, "the video's size", |app| {
        app.model.int(PropertyId::VideoWidth).is_some()
    });
    // Video plays: the screen stays on, unless the setting says otherwise.
    assert!(app.keeps_screen_on());
    assert_eq!(
        app.dispatch(&engine, Action::TogglePreventScreenSaver),
        Some(UiRequest::Osd(
            "Prevent screen saver during playback: Off".to_owned()
        ))
    );
    assert!(!app.keeps_screen_on());
    assert_eq!(app.checked(Action::TogglePreventScreenSaver), Some(false));
    app.dispatch(&engine, Action::TogglePreventScreenSaver);
    assert!(app.keeps_screen_on());

    app.media_command(&engine, MediaCommand::Pause);
    drain_until(&mut app, &engine, "paused", |app| {
        app.media_state().status == Status::Paused
    });
    assert!(!app.keeps_screen_on(), "paused: the screen may sleep");
    app.media_command(
        &engine,
        MediaCommand::SetPosition {
            track: 1,
            seconds: 30.0,
        },
    );
    drain_until(&mut app, &engine, "at 30 s", |app| {
        (app.media_state().position - 30.0).abs() < 0.5
    });
    // Another track's position is not this file's.
    app.media_command(
        &engine,
        MediaCommand::SetPosition {
            track: 7,
            seconds: 5.0,
        },
    );
    app.media_command(&engine, MediaCommand::SeekBy(10.0));
    drain_until(&mut app, &engine, "at 40 s", |app| {
        (app.media_state().position - 40.0).abs() < 0.5
    });
    app.media_command(&engine, MediaCommand::SetVolume(0.5));
    drain_until(&mut app, &engine, "volume 50 %", |app| {
        (app.media_state().volume - 0.5).abs() < 1e-6
    });
    app.media_command(&engine, MediaCommand::Play);
    drain_until(&mut app, &engine, "playing again", |app| {
        app.media_state().status == Status::Playing
    });
    assert_eq!(
        app.media_command(&engine, MediaCommand::Raise),
        Some(UiRequest::Raise)
    );
    assert_eq!(
        app.media_command(&engine, MediaCommand::Quit),
        Some(UiRequest::Exit)
    );

    // Stop closes the file, as the skin's stop button; Play starts the
    // album's entry again.
    app.media_command(&engine, MediaCommand::Stop);
    drain_until(&mut app, &engine, "stopped", |app| {
        app.media_state().status == Status::Stopped
    });
    assert!(app.media_state().can_play);
    app.media_command(&engine, MediaCommand::Play);
    drain_until(&mut app, &engine, "the album plays", |app| {
        app.media_state().status == Status::Playing
            && app.model.double(PropertyId::TimePos).is_some()
    });
    assert_eq!(app.media_state().track, 2);
    let _ = std::fs::remove_dir_all(&dir);
}
