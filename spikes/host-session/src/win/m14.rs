//! `host-session m14` (goal P): PotPlayer's and RVP's skin modes side by
//! side, for M14's comparison. Both players show the same skin (`--skin`,
//! BlackSpace from PotPlayer's Skins folder) on the same files at the same
//! place, one after the other:
//!
//! 1. started with nothing open: the logo (`<player>-1-idle.png`);
//! 2. the video (`--video`) opened at 1280x720 (`-2-video.png`);
//! 3. a double click on the video for fullscreen, the pointer at the
//!    bottom so that the on-screen control shows (`-3-fullscreen.png`),
//!    then at the top (`-3-fullscreen-top.png`) and still in the middle
//!    for 3 s (`-3-fullscreen-middle.png`), another double click back;
//! 4. the audio file (`--audio`): the audio skin (`-4-audio.png`).
//!
//! PotPlayer opens the files in its running window (`/current`); RVP runs
//! on a channel of its own (`--instance=m14-host`) with its settings and
//! state in `--out` (so its own folders stay as they are, and with no IPC
//! server, so SVP does not attach), and a second `rvp.exe` on that channel
//! hands the files over. Each window is kept on top while it is captured
//! (the terminal that runs this tool would otherwise cover it), and the
//! tool clicks and captures only while that window is the one under the
//! points: when another window stays over it (one that is kept on top
//! too), the session stops before it clicks anything; `--players rvp`
//! captures RVP only. Only mouse input is
//! injected (a double click on the video); no setting of either player is
//! changed. PotPlayer saves its window place and adds the files to its
//! playlist and history: `m14-host.ps1` saves its registry key and
//! playlist folder before and writes them back after.

use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GWL_EXSTYLE, GetAncestor, GetWindowLongW, GetWindowThreadProcessId, HWND_NOTOPMOST,
    HWND_TOPMOST, PostMessageW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WM_CLOSE,
    WS_EX_TOPMOST, WindowFromPoint,
};

use super::{
    Log, Options, RVP_EXE, capture_to, class_name, close_potplayer, monitor_of, move_to,
    mouse_flags, processes_named, rect_text, require_no_potplayer, send, sleep_ms,
    start_potplayer, top_windows, window_rect,
};

/// The window's place for the windowed captures.
const SIZE: (i32, i32) = (1280, 720);
/// RVP's channel for this session (never the owner's own `rvp-<user>`).
const CHANNEL: &str = "--instance=m14-host";

pub fn m14(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let potplayer = options.path("--potplayer")?;
    let rvp = options.path("--rvp")?;
    let skin = options.path("--skin")?;
    let video = options.path("--video")?;
    let audio = options.path("--audio")?;
    let players = options.get("--players").unwrap_or("potplayer,rvp");
    let mut log = Log::open(&out)?;
    if !processes_named(RVP_EXE).is_empty() {
        log.line("note=another RVP runs; this session's RVP uses its own channel");
    }

    let mut result = Ok(());
    if players.contains("potplayer") {
        require_no_potplayer()?;
        log.line("== PotPlayer");
        let pot = start_potplayer(&potplayer, &[], &mut log)?;
        let pot_window = pot.hwnd;
        let open_pot = |file: &Path| {
            let _ = Command::new(&potplayer).arg(file).arg("/current").spawn();
        };
        result = session("potplayer", pot_window, &out, &video, &audio, &open_pot, &mut log);
        if let Err(error) = &result {
            log.line(&format!("potplayer_error={error}"));
        }
        unsafe {
            SetWindowPos(pot_window, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        close_potplayer(pot, &mut log);
    }
    if !players.contains("rvp") {
        return result;
    }

    log.line("== RVP");
    let config = out.join("rvp-config");
    let state = out.join("rvp-state");
    std::fs::create_dir_all(&config).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    // No IPC server: SVP attaches to RVP only through one.
    std::fs::write(config.join("settings.json"), "{ \"ipc-server\": \"\" }\n")
        .map_err(|e| e.to_string())?;
    let mut child = Command::new(&rvp)
        .arg(CHANNEL)
        .arg(format!("--skin={}", skin.display()))
        .env("RVP_CONFIG_DIR", &config)
        .env("RVP_STATE_DIR", &state)
        .spawn()
        .map_err(|e| format!("{}: {e}", rvp.display()))?;
    let rvp_result = match rvp_window(&child) {
        Some(window) => {
            let open_rvp = |file: &Path| {
                let _ = Command::new(&rvp)
                    .arg(CHANNEL)
                    .arg(file)
                    .env("RVP_CONFIG_DIR", &config)
                    .env("RVP_STATE_DIR", &state)
                    .spawn();
            };
            session("rvp", window, &out, &video, &audio, &open_rvp, &mut log)
        }
        None => Err("RVP's window did not appear".to_owned()),
    };
    if let Err(error) = &rvp_result {
        log.line(&format!("rvp_error={error}"));
    }
    close_rvp(&mut child, &mut log);
    result.and(rvp_result)
}

/// RVP's main window, once it shows.
fn rvp_window(child: &Child) -> Option<HWND> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(&hwnd) = top_windows(None, Some(child.id()))
            .iter()
            .find(|&&hwnd| class_name(hwnd).contains("QWindow"))
        {
            sleep_ms(1500);
            return Some(hwnd);
        }
        sleep_ms(200);
    }
    None
}

fn close_rvp(child: &mut Child, log: &mut Log) {
    for hwnd in top_windows(None, Some(child.id())) {
        if class_name(hwnd).contains("QWindow") {
            unsafe {
                SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            log.line(&format!("rvp_closed status={status}"));
            return;
        }
        sleep_ms(200);
    }
    let _ = child.kill();
    log.line("rvp_close=TIMEOUT (stopped)");
}

/// Puts the window at the session's place, on top.
fn place(hwnd: HWND, monitor: &RECT) {
    let x = monitor.left + 40;
    let y = monitor.top + 40;
    unsafe { SetWindowPos(hwnd, HWND_TOPMOST, x, y, SIZE.0, SIZE.1, SWP_NOACTIVATE) };
    sleep_ms(1500);
}

/// Keeps the window over the others (without taking the keyboard), as it
/// is now.
fn raise(hwnd: HWND) {
    unsafe {
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
    sleep_ms(300);
}

/// The top-level window under the point when it belongs to another
/// process than `hwnd` (a player's own popups, such as PotPlayer's
/// fullscreen control bar, are fine): its class, process and whether it
/// is kept on top.
fn covering(hwnd: HWND, x: i32, y: i32) -> Option<String> {
    let root = unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) };
    let mut player = 0;
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &raw mut player);
        GetWindowThreadProcessId(root, &raw mut pid);
    }
    if pid == player {
        return None;
    }
    let topmost = unsafe { GetWindowLongW(root, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST != 0;
    Some(format!("class={} pid={pid} topmost={topmost}", class_name(root)))
}

/// Fails unless the player's window is the one under every point, before
/// a click or a capture.
fn require_shown(hwnd: HWND, points: &[(i32, i32)], step: &str) -> Result<(), String> {
    for &(x, y) in points {
        if let Some(other) = covering(hwnd, x, y) {
            return Err(format!(
                "{step}: another window is over the player at {x},{y} ({other}); stopped before clicking or capturing"
            ));
        }
    }
    Ok(())
}

/// The middle and the corners of a rectangle, 16 pixels inside.
fn inside(rect: &RECT) -> [(i32, i32); 5] {
    [
        ((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2),
        (rect.left + 16, rect.top + 16),
        (rect.right - 17, rect.top + 16),
        (rect.left + 16, rect.bottom - 17),
        (rect.right - 17, rect.bottom - 17),
    ]
}

fn double_click(x: i32, y: i32) {
    move_to(x, y);
    let click = [mouse_flags(MOUSEEVENTF_LEFTDOWN), mouse_flags(MOUSEEVENTF_LEFTUP)];
    send(&click);
    sleep_ms(80);
    send(&click);
}

/// The four captures of one player.
fn session(
    name: &str,
    hwnd: HWND,
    out: &Path,
    video: &Path,
    audio: &Path,
    open: &dyn Fn(&Path),
    log: &mut Log,
) -> Result<(), String> {
    let monitor = monitor_of(hwnd).rcMonitor;
    log.line(&format!("{name}_monitor={}", rect_text(&monitor)));
    let park = (monitor.right - 2, monitor.bottom - 2);
    let shot = |log: &mut Log, step: &str, rect: &RECT| {
        capture_to(log, rect, &out.join(format!("{name}-{step}.png")));
    };

    place(hwnd, &monitor);
    move_to(park.0, park.1);
    sleep_ms(800);
    let rect = window_rect(hwnd);
    require_shown(hwnd, &inside(&rect), "1-idle")?;
    shot(log, "1-idle", &rect);

    open(video);
    log.line(&format!("{name}_opened={}", video.display()));
    sleep_ms(6000);
    place(hwnd, &monitor);
    move_to(park.0, park.1);
    sleep_ms(1500);
    let rect = window_rect(hwnd);
    log.line(&format!("{name}_video_window={}", rect_text(&rect)));
    require_shown(hwnd, &inside(&rect), "2-video")?;
    shot(log, "2-video", &rect);

    // The video's middle: inside the window, above the bars.
    let middle = ((rect.left + rect.right) / 2, rect.top + (rect.bottom - rect.top) / 3);
    raise(hwnd);
    require_shown(hwnd, &[middle], "fullscreen click")?;
    double_click(middle.0, middle.1);
    sleep_ms(2500);
    let full = window_rect(hwnd);
    log.line(&format!("{name}_fullscreen_window={}", rect_text(&full)));
    // The on-screen control shows while the pointer is over the bottom.
    move_to((monitor.left + monitor.right) / 2, monitor.bottom - 40);
    sleep_ms(400);
    move_to((monitor.left + monitor.right) / 2 + 10, monitor.bottom - 30);
    raise(hwnd);
    sleep_ms(1500);
    require_shown(hwnd, &inside(&monitor), "3-fullscreen")?;
    shot(log, "3-fullscreen", &monitor);
    move_to((monitor.left + monitor.right) / 2, monitor.top + 40);
    sleep_ms(400);
    move_to((monitor.left + monitor.right) / 2 + 10, monitor.top + 30);
    raise(hwnd);
    sleep_ms(1500);
    require_shown(hwnd, &inside(&monitor), "3-fullscreen-top")?;
    shot(log, "3-fullscreen-top", &monitor);
    move_to((monitor.left + monitor.right) / 2, (monitor.top + monitor.bottom) / 2);
    sleep_ms(3000);
    require_shown(hwnd, &inside(&monitor), "3-fullscreen-middle")?;
    shot(log, "3-fullscreen-middle", &monitor);
    let back = (
        (monitor.left + monitor.right) / 2,
        monitor.top + (monitor.bottom - monitor.top) / 3,
    );
    require_shown(hwnd, &[back], "windowed click")?;
    double_click(back.0, back.1);
    sleep_ms(2500);
    log.line(&format!("{name}_back_window={}", rect_text(&window_rect(hwnd))));

    open(audio);
    log.line(&format!("{name}_opened={}", audio.display()));
    sleep_ms(5000);
    raise(hwnd);
    move_to(park.0, park.1);
    sleep_ms(800);
    let rect = window_rect(hwnd);
    log.line(&format!("{name}_audio_window={}", rect_text(&rect)));
    require_shown(hwnd, &inside(&rect), "4-audio")?;
    shot(log, "4-audio", &rect);
    Ok(())
}
