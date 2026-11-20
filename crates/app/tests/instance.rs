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

#[test]
fn a_command_line_opens_files_network_and_own_addresses_only() {
    let dir = std::env::temp_dir().join(format!("rvp-instance-doors-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut settings = Settings {
        reopen_last_file: false,
        ..Settings::default()
    };
    // Opening files replaces the album: a request left without files
    // must not empty it.
    settings.playlist.append_on_open = false;
    let mut app = App::new(settings, &dir).expect("app");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    app.open(&engine, &[LONG.to_owned()], false).expect("open");
    wait(&mut app, &engine, "playing", |m| plays(m, LONG));
    let long = local_path(LONG);

    // What a second `rvp` hands over, as the shell carries it out: the
    // switches first, then the request. Nothing of it is a file or a
    // network address, so nothing is added and the film plays on.
    for mode in [Mode::Add, Mode::Insert, Mode::Play] {
        let mut request = Request {
            mode,
            files: vec![
                "av://lavfi:testsrc".to_owned(),
                "FDCLOSE://9".to_owned(),
                "edl://a.mkv".to_owned(),
                "memory://x".to_owned(),
            ],
            switches: Switches {
                sub: Some("av://lavfi:testsrc".to_owned()),
                aud: Some("fd://0".to_owned()),
                ..Switches::default()
            },
        };
        app.prepare_switches(&mut request);
        assert!(request.files.is_empty(), "{mode:?}");
        assert_eq!(request.switches.sub, None, "{mode:?}");
        assert_eq!(request.switches.aud, None, "{mode:?}");
        app.remote(&engine, &request).expect("remote");
        assert_eq!(paths(&app), [&long], "{mode:?}");
        // One notice for all of them, then none.
        assert_eq!(
            app.refusal(),
            Some(rvp_app::UiRequest::Message(
                "Not opened: 6 addresses (av://, edl://, fdclose://, memory://). \
                 Only files and network addresses are taken this way."
                    .to_owned()
            )),
            "{mode:?}"
        );
        assert_eq!(app.refusal(), None);
    }
    std::thread::sleep(Duration::from_millis(300));
    app.drain(&engine, Instant::now());
    assert!(plays(&app.model, LONG), "still the film");

    // The addresses RVP's own Open menu makes stay: "Play in new
    // instance" hands an album's entries over this way.
    let mut own = Request {
        mode: Mode::Add,
        files: vec![
            "dvd://longest//no/such/disc".to_owned(),
            "av://lavfi:testsrc".to_owned(),
            SHORT.to_owned(),
            "cdda://".to_owned(),
        ],
        switches: Switches::default(),
    };
    app.prepare_switches(&mut own);
    app.remote(&engine, &own).expect("add");
    assert_eq!(
        paths(&app),
        [
            long.as_str(),
            "dvd://longest//no/such/disc",
            local_path(SHORT).as_str(),
            "cdda://"
        ]
    );
    assert!(app.refusal().is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_first_start_opens_what_a_handed_over_command_line_would() {
    let dir = std::env::temp_dir().join(format!("rvp-instance-start-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let settings = Settings {
        reopen_last_file: false,
        ..Settings::default()
    };
    let mut app = App::new(settings, &dir).expect("app");
    app.skip_first_import();
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    // `rvp av://lavfi:testsrc SHORT fd://0` with no RVP running.
    let cli = Cli::parse(
        ["av://lavfi:testsrc", SHORT, "fd://0"]
            .iter()
            .map(|&argument| argument.to_owned()),
    );
    let mut request = Request::from_cli(&cli, Path::new("/"));
    app.prepare_switches(&mut request);
    app.start(&engine, &request.files).expect("start");
    assert_eq!(paths(&app), [&local_path(SHORT)]);
    wait(&mut app, &engine, "the file plays", |m| plays(m, SHORT));
    assert!(matches!(
        app.refusal(),
        Some(rvp_app::UiRequest::Message(text)) if text.starts_with("Not opened: 2 addresses (av://, fd://)")
    ));
    let _ = std::fs::remove_dir_all(&dir);
}
