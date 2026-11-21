//! A libmpv core whose options are refused at start (a refused option, a
//! failed calibration, encode or thumbnail core) leaves nothing behind:
//! its handle and its core's thread go. Alone in its binary: it counts the
//! process's threads, which other tests' cores would change.

#![cfg(target_os = "linux")]

use std::time::{Duration, Instant};

use rvp_engine::{Engine, EngineOptions};

/// The process's threads now.
fn threads() -> usize {
    std::fs::read_dir("/proc/self/task").map_or(0, Iterator::count)
}

fn refused() -> EngineOptions {
    let mut options = EngineOptions::headless();
    options
        .extra
        .push(("rvp-no-such-option".to_owned(), "1".to_owned()));
    options
}

#[test]
fn a_refused_start_leaves_no_thread() {
    // Threads libmpv starts once per process come with the first cores.
    drop(Engine::new(&EngineOptions::headless()).expect("libmpv starts"));
    assert!(Engine::new(&refused()).is_err(), "the option is refused");
    let before = threads();
    for _ in 0..4 {
        assert!(Engine::new(&refused()).is_err());
    }
    // A thread that ends may take a moment to leave the list.
    let deadline = Instant::now() + Duration::from_secs(2);
    while threads() > before && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(threads(), before, "threads left by refused starts");
}
