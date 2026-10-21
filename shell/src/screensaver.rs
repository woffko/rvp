//! RVP as Windows' screen saver (M14's Screen Saver page): the package's
//! `rvp.scr` is a copy of `rvp.exe`; run as a screen saver it starts the
//! `rvp.exe` beside it and waits for it: `/s` plays the page's media
//! (`--screensaver`), `/c` (or nothing) opens the page
//! (`--screensaver-settings`), and `/p` (the preview in Windows' dialog,
//! which embeds the saver in another program's window) shows nothing.

/// When this process runs as `rvp.scr`: its exit code once `rvp.exe` ran.
pub fn launch() -> Option<i32> {
    let exe = std::env::current_exe().ok()?;
    if !exe
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("scr"))
    {
        return None;
    }
    let first = std::env::args()
        .nth(1)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mode = match first.get(..2) {
        Some("/s" | "-s") => "--screensaver",
        Some("/p" | "-p") => return Some(0),
        _ => "--screensaver-settings",
    };
    let status = std::process::Command::new(exe.with_file_name("rvp.exe"))
        .arg(mode)
        .status();
    Some(status.ok().and_then(|status| status.code()).unwrap_or(1))
}

/// `--screensaver`: RVP keeps a state of its own (its albums and
/// positions), so the saver's files never join the user's. Before any
/// thread starts.
pub fn separate_state() {
    if !std::env::args().any(|arg| arg == "--screensaver") {
        return;
    }
    if let Ok(dirs) = rvp_app::Dirs::from_env() {
        let state = dirs.state.join("screensaver");
        crate::ffi::bridge::rvp_set_env("RVP_STATE_DIR", &state.display().to_string());
    }
}
