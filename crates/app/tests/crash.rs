//! The panic hook's crash log (M8). A test binary of its own: the hook is
//! the process's.

use std::path::PathBuf;

#[test]
fn a_panic_writes_a_crash_log() {
    let dir: PathBuf = std::env::temp_dir().join(format!("rvp-crash-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    rvp_app::crash::install_panic_hook(&dir);
    let result = std::thread::Builder::new()
        .name("crashing".to_owned())
        .spawn(|| panic!("test panic 4711"))
        .expect("thread")
        .join();
    assert!(result.is_err(), "the thread panicked");
    let logs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the crash folder")
        .map(|entry| entry.expect("entry").path())
        .collect();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let name = logs[0]
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    assert!(name.starts_with("crash-"), "{name}");
    assert_eq!(
        logs[0].extension().and_then(|e| e.to_str()),
        Some("log"),
        "{name}"
    );
    let text = std::fs::read_to_string(&logs[0]).expect("log");
    assert!(
        text.starts_with(&format!("RVP {}", rvp_app::crash::version_text())),
        "{text}"
    );
    assert!(text.contains("panic in thread 'crashing' at "), "{text}");
    assert!(text.contains("crash.rs"), "{text}");
    assert!(text.contains("test panic 4711"), "{text}");
    assert!(text.contains("backtrace:"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
