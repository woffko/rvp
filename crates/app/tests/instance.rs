//! Single-instance commands (goal F) against the real engine (headless):
//! what a second `rvp /add`, `/insert` or plain `rvp FILE` does in the
//! running one. The socket itself is the shell's (Qt) and is checked by
//! `rvp --selftest --instance=NAME`.

use std::path::Path;
use std::time::{Duration, Instant};

use rvp_app::instance::{Mode, Request, Switches};
use rvp_app::playlist::local_path;
use rvp_app::{App, Cli, Settings};
use rvp_engine::{Engine, EngineOptions, PlayerModel};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);
const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);

fn wait(app: &mut App, engine: &Engine, what: &str, done: impl Fn(&PlayerModel) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(&app.model) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
        app.drain(engine, Instant::now());
    }
}

fn paths(app: &App) -> Vec<&str> {
    app.library()
        .shown()
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect()
}

/// Whether `path` plays (in mpv's form, see `local_path`).
fn plays(model: &PlayerModel, path: &str) -> bool {
    model.loaded_path() == Some(local_path(path).as_str())
}

#[test]
fn add_insert_and_play_requests() {
    let dir = std::env::temp_dir().join(format!("rvp-instance-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let settings = Settings {
        reopen_last_file: false,
        ..Settings::default()
    };
    let mut app = App::new(settings, &dir).expect("app");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    app.open(&engine, &[LONG.to_owned()], false).expect("open");
    wait(&mut app, &engine, "playing", |m| plays(m, LONG));
    let (long, short) = (local_path(LONG), local_path(SHORT));
    let inserted = local_path("/media/inserted.mkv");

    // `rvp /add SHORT`: appended, the film keeps playing.
    let cli = Cli::parse(["/add".to_owned(), SHORT.to_owned()]);
    let add = Request::from_cli(&cli, Path::new("/"));
    assert_eq!(add.mode, Mode::Add);
    app.remote(&engine, &add).expect("add");
    assert_eq!(paths(&app), [&long, &short]);

    // `rvp /insert X`: right below the entry that plays.
    let insert = Request {
        mode: Mode::Insert,
        files: vec!["/media/inserted.mkv".to_owned()],
        switches: Switches::default(),
    };
    app.remote(&engine, &insert).expect("insert");
    assert_eq!(paths(&app), [&long, &inserted, &short]);
    std::thread::sleep(Duration::from_millis(300));
    app.drain(&engine, Instant::now());
    assert!(plays(&app.model, LONG), "still the film");

    // `rvp SHORT` (/current): appended and played.
    let play = Request {
        mode: Mode::Play,
        files: vec![SHORT.to_owned()],
        switches: Switches::default(),
    };
    app.remote(&engine, &play).expect("play");
    assert_eq!(paths(&app).len(), 4);
    wait(&mut app, &engine, "the new entry plays", |m| {
        plays(m, SHORT)
    });
    assert_eq!(app.library().shown().current, Some(3));
    let _ = std::fs::remove_dir_all(&dir);
}
