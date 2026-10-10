//! Test helper for `tests/resume.rs`: plays `FILE` headless with the resume
//! store in `STATE_DIR` and prints `time=<seconds>` whenever the position
//! changes, until it is killed or `SECONDS` (default 120) pass.
//!
//! ```text
//! rvp-resume-probe STATE_DIR FILE [SECONDS]
//! ```

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use rvp_app::{App, Settings};
use rvp_engine::{Engine, EngineOptions, PropertyId};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(dir), Some(file)) = (args.first(), args.get(1)) else {
        eprintln!("usage: rvp-resume-probe STATE_DIR FILE [SECONDS]");
        return ExitCode::FAILURE;
    };
    let seconds = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(120);
    match run(Path::new(dir), file, seconds) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("rvp-resume-probe: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(dir: &Path, file: &str, seconds: u64) -> Result<(), String> {
    let mut app = App::new(Settings::default(), dir).map_err(|e| e.to_string())?;
    let engine = Engine::new(&EngineOptions::headless()).map_err(|e| e.to_string())?;
    app.start(&engine, &[file.to_owned()])
        .map_err(|e| e.to_string())?;
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
