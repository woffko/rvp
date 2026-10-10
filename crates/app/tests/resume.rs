//! Resume tests (docs/PLAN.md section 7): kill the player at pseudo-random
//! points during playback and check that the saved position is within 5 s
//! of the last one it reached, and that the next start resumes there.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rvp_app::{App, EDGE_SECONDS, ResumeState, SAVE_INTERVAL, Settings, read_json};
use rvp_engine::{Engine, EngineOptions, PropertyId};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-resume-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The single saved position in `dir`'s resume store.
fn saved_position(dir: &Path) -> Option<f64> {
    let state: ResumeState = read_json(&dir.join("state.json")).ok().flatten()?;
    let mut positions = state.positions.values();
    let position = positions.next()?.seconds;
    assert!(positions.next().is_none(), "one file was played");
    Some(position)
}

/// Deterministic pseudo-random delays (linear congruential generator).
struct Delays(u64);

impl Delays {
    fn next_ms(&mut self, low: u64, high: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        low + (self.0 >> 33) % (high - low)
    }
}

/// Runs the probe until it has played `play_ms` after reaching `resume`,
/// kills it and returns (first position after resuming, last position seen).
fn play_and_kill(dir: &Path, resume: Option<f64>, play_ms: u64) -> (f64, f64) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rvp-resume-probe"))
        .arg(dir)
        .arg(FIXTURE)
        .stdout(Stdio::piped())
        .spawn()
        .expect("probe starts");
    let times = Arc::new(Mutex::new(Vec::<f64>::new()));
    let reader = {
        let times = Arc::clone(&times);
        let stdout = child.stdout.take().expect("stdout");
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(value) = line.strip_prefix("time=").and_then(|t| t.parse().ok()) {
                    times.lock().expect("lock").push(value);
                }
            }
        })
    };
    // Wait until playback has reached the resume point (or any position).
    let start_deadline = Instant::now() + Duration::from_secs(20);
    let resumed_at = loop {
        assert!(
            Instant::now() < start_deadline,
            "probe did not start playing"
        );
        let first = times
            .lock()
            .expect("lock")
            .iter()
            .copied()
            .find(|&t| resume.map_or(t > 0.0, |r| t >= r - 1.5));
        if let Some(first) = first {
            break first;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    std::thread::sleep(Duration::from_millis(play_ms));
    let last = times
        .lock()
        .expect("lock")
        .last()
        .copied()
        .unwrap_or_default();
    child.kill().expect("kill");
    let _ = child.wait();
    let _ = reader.join();
    (resumed_at, last)
}

#[test]
fn positions_survive_killed_processes() {
    let dir = scratch("kill");
    let mut delays = Delays(0x5256_5052_4553_554d);
    let mut stored: Option<f64> = None;
    let mut saved_runs = 0;
    for run in 0..6 {
        let play_ms = if run == 0 {
            4500
        } else {
            delays.next_ms(1500, 5000)
        };
        let (resumed_at, last) = play_and_kill(&dir, stored, play_ms);
        if let Some(previous) = stored {
            assert!(
                (resumed_at - previous).abs() < 1.5,
                "run {run}: resumed at {resumed_at}, saved {previous}"
            );
        }
        let Some(saved) = saved_position(&dir) else {
            // Positions near the start are not kept (the file starts over).
            assert!(
                last < EDGE_SECONDS + SAVE_INTERVAL.as_secs_f64() + 0.5,
                "run {run}: nothing saved at {last}"
            );
            stored = None;
            continue;
        };
        assert!(
            saved <= last + 0.5,
            "run {run}: saved {saved} is ahead of {last}"
        );
        assert!(
            last - saved <= 5.0,
            "run {run}: saved {saved} is more than 5 s behind {last}"
        );
        stored = Some(saved);
        saved_runs += 1;
    }
    assert!(saved_runs >= 5, "only {saved_runs} runs saved a position");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clean_restart_resumes_paused_at_the_last_file() {
    let dir = scratch("restart");
    {
        let mut app = App::new(Settings::default(), &dir).expect("app");
        let engine = Engine::new(&EngineOptions::headless()).expect("engine");
        app.start(&engine, &[FIXTURE.to_owned()]).expect("start");
        engine.set_volume(37.0).expect("volume");
        engine.set_pause(false).expect("play");
        let deadline = Instant::now() + Duration::from_secs(20);
        while app.model.double(PropertyId::TimePos).unwrap_or_default() < 6.0 {
            assert!(Instant::now() < deadline, "playback did not reach 6 s");
            std::thread::sleep(Duration::from_millis(20));
            app.drain(&engine, Instant::now());
        }
        app.shutdown(&engine, Instant::now()).expect("shutdown");
    }
    // No files on the command line: the last file opens paused near 6 s.
    let mut app = App::new(Settings::default(), &dir).expect("app");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    app.start(&engine, &[]).expect("start");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(Instant::now() < deadline, "did not resume: {:?}", app.model);
        std::thread::sleep(Duration::from_millis(20));
        app.drain(&engine, Instant::now());
        let time = app.model.double(PropertyId::TimePos).unwrap_or_default();
        if app.model.media_loaded() && time > 5.0 {
            assert!(time < 7.5, "resumed at {time}");
            break;
        }
    }
    assert_eq!(
        app.model.flag(PropertyId::Pause),
        Some(true),
        "starts paused"
    );
    assert_eq!(
        app.model.double(PropertyId::Volume),
        Some(37.0),
        "volume restored"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
