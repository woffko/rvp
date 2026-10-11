//! Test helper for `tests/resume.rs`, run headless with the state in
//! `STATE_DIR`:
//!
//! ```text
//! rvp-resume-probe STATE_DIR FILE [SECONDS]
//! rvp-resume-probe --album STATE_DIR FILE1 FILE2
//! rvp-resume-probe --edits STATE_DIR COUNT [PAUSE_MS]
//! ```
//!
//! The first form plays `FILE` and prints `time=<seconds>` whenever the
//! position changes, until it is killed or `SECONDS` (default 120) pass.
//! `--album` does the same in a second album holding both files, playing
//! the second one. `--edits` makes `COUNT` scripted playlist edits, prints
//! `edit=<n>` after each and pauses `PAUSE_MS` between them; with a pause
//! of 0 it flushes at the end (the expected state after `COUNT` edits).

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use rvp_app::playlist::{Library, Move};
use rvp_app::{Action, App, Settings};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const USAGE: &str = "usage: rvp-resume-probe [--album | --edits] STATE_DIR ...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("--edits") => match (args.get(1), args.get(2).and_then(|n| n.parse().ok())) {
            (Some(dir), Some(count)) => {
                let pause = args.get(3).and_then(|ms| ms.parse().ok()).unwrap_or(0);
                edits(Path::new(dir), count, pause)
            }
            _ => Err(USAGE.to_owned()),
        },
        Some("--album") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(dir), Some(first), Some(second)) => {
                play(Path::new(dir), &[first.clone(), second.clone()], 120)
            }
            _ => Err(USAGE.to_owned()),
        },
        _ => match (args.first(), args.get(1)) {
            (Some(dir), Some(file)) => {
                let seconds = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(120);
                play(Path::new(dir), std::slice::from_ref(file), seconds)
            }
            _ => Err(USAGE.to_owned()),
        },
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("rvp-resume-probe: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Edit `n` of the script: albums, inserts, moves and undo.
fn scripted_edit(library: &mut Library, n: usize) {
    let shown = library.lists().shown;
    match n % 5 {
        0 => {
            library.add_album(&format!("Album {n}"));
        }
        1 => {
            library.insert(shown, None, &[format!("/media/{n:03}.mkv")]);
        }
        2 => {
            library.insert(
                shown,
                Some(0),
                &[format!("/media/{n:03}a.mkv"), format!("/media/{n:03}b.mkv")],
            );
        }
        3 => {
            // The two entries edit 2 inserted (and selected) go last.
            library.move_selected(shown, Move::Bottom);
        }
        _ => {
            library.undo();
        }
    }
}

fn edits(dir: &Path, count: usize, pause_ms: u64) -> Result<(), String> {
    let mut library = Library::open(dir).map_err(|e| e.to_string())?;
    let mut out = std::io::stdout();
    for n in 1..=count {
        scripted_edit(&mut library, n);
        if pause_ms > 0 {
            writeln!(out, "edit={n}").map_err(|e| e.to_string())?;
            out.flush().map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(pause_ms));
        }
    }
    library.flush().map_err(|e| e.to_string())
}

fn play(dir: &Path, files: &[String], seconds: u64) -> Result<(), String> {
    let mut app = App::new(Settings::default(), dir).map_err(|e| e.to_string())?;
    let engine = Engine::new(&EngineOptions::headless()).map_err(|e| e.to_string())?;
    if let [file] = files {
        app.start(&engine, std::slice::from_ref(file))
            .map_err(|e| e.to_string())?;
    } else {
        // A second album with both files; the second one plays.
        app.start(&engine, &[]).map_err(|e| e.to_string())?;
        app.name_album("Second", false);
        app.open(&engine, files, false).map_err(|e| e.to_string())?;
        app.dispatch(&engine, Action::Next);
    }
    engine.set_pause(false).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut last = None;
    let mut out = std::io::stdout();
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
        app.drain(&engine, Instant::now());
        let time = app.model.double(PropertyId::TimePos);
        if time != last
            && let Some(time) = time
        {
            writeln!(out, "time={time:.3}").map_err(|e| e.to_string())?;
            out.flush().map_err(|e| e.to_string())?;
            last = Some(time);
        }
    }
    app.shutdown(&engine, Instant::now())
        .map_err(|e| e.to_string())
}
