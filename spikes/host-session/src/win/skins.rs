//! `host-session skins` (the skin check of 2026-11-04): every skin of
//! PotPlayer's `Skins` folder in PotPlayer and then in RVP, switched in the
//! running player while a video plays, for comparing the layouts and what
//! a switch does to the window. For each skin, in the order of RVP's Skins
//! menu (the files' paths):
//!
//! 1. the window at 1280x720, then the skin opened in the running player
//!    (`PotPlayerMini64.exe SKIN /current`; RVP: a second `rvp.exe` on the
//!    session's channel hands the file over): the window's place before and
//!    after (`switch` lines) and `<player>-NN-<skin>-a-switched.png`;
//! 2. the window at 1280x720 again: `-b-1280x720.png`;
//! 3. a double click on the video for fullscreen, the pointer at the bottom
//!    (`-c-full-bottom.png`) and at the top (`-d-full-top.png`), then a
//!    double click back.
//!
//! When PotPlayer does not take a skin that way (a dialog, or the window
//! looks as before), it is closed, its `LastSkinName` is set and it is
//! started again for that skin and the rest (`m14-host.ps1`'s companion
//! `skins-host.ps1` saves PotPlayer's registry key and playlist before and
//! writes them back after). Only mouse input is injected, as in `m14`; the
//! tool stops before clicking when another window covers the player.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_NOTOPMOST, PostMessageW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WM_CLOSE,
};

use super::m14::{close_rvp, double_click, inside, place, raise, require_shown, rvp_window};
use super::{
    Log, Options, Pot, capture, capture_to, class_name, close_potplayer, monitor_of, move_to,
    rect_text, require_no_potplayer, sleep_ms, start_potplayer, top_windows, window_rect,
    window_text,
};

/// RVP's channel for this session (never the owner's own `rvp-<user>`).
const CHANNEL: &str = "--instance=skins-host";
/// PotPlayer's skin setting.
const SKIN_KEY: &str = r"HKCU\Software\DAUM\PotPlayerMini64\Settings";

pub fn skins(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let potplayer = options.path("--potplayer")?;
    let rvp = options.path("--rvp")?;
    let folder = options.path("--skins")?;
    let video = options.path("--video")?;
    let players = options.get("--players").unwrap_or("potplayer,rvp");
    let only: Vec<&str> = options
        .get("--only")
        .map(|list| list.split(',').collect())
        .unwrap_or_default();
    let mut skins: Vec<PathBuf> = std::fs::read_dir(&folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dsf"))
        })
        .filter(|path| only.is_empty() || only.iter().any(|name| stem(path) == *name))
        .collect();
    // RVP's Skins menu orders them by path.
    skins.sort();
    let mut log = Log::open(&out)?;
    log.line(&format!("skins={}", skins.len()));

    let mut result = Ok(());
    if players.contains("potplayer") {
        require_no_potplayer()?;
        log.line("== PotPlayer");
        result = potplayer_session(&potplayer, &skins, &video, &out, &mut log);
        if let Err(error) = &result {
            log.line(&format!("potplayer_error={error}"));
        }
    }
    if players.contains("rvp") {
        log.line("== RVP");
        let rvp_result = rvp_session(&rvp, &skins, &video, &out, &mut log);
        if let Err(error) = &rvp_result {
            log.line(&format!("rvp_error={error}"));
        }
        result = result.and(rvp_result);
    }
    result
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A sum over the window's top 40 rows: changes when the skin does.
fn title_mark(rect: &RECT) -> u64 {
    let top = RECT {
        bottom: rect.top + 40.min(rect.bottom - rect.top),
        ..*rect
    };
    let (_, _, rgba) = capture(&top);
    rgba.iter()
        .enumerate()
        .map(|(index, &byte)| (index as u64 % 251 + 1) * u64::from(byte))
        .sum()
}

/// PotPlayer's dialogs (class `#32770`) other than its main window:
/// logged, captured and closed.
fn close_dialogs(pid: u32, main: HWND, name: &str, out: &Path, log: &mut Log) -> bool {
    let mut found = false;
    for (index, hwnd) in top_windows(None, Some(pid)).into_iter().enumerate() {
        if hwnd != main && class_name(hwnd) == "#32770" {
            found = true;
            let rect = window_rect(hwnd);
            log.line(&format!("dialog={name} text=\"{}\" {}", window_text(hwnd), rect_text(&rect)));
            capture_to(log, &rect, &out.join(format!("potplayer-dialog-{name}-{index}.png")));
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
    }
    found
}

fn open_in_potplayer(exe: &Path, file: &Path) {
    let _ = Command::new(exe).arg(file).arg("/current").spawn();
}

/// PotPlayer started with `skin` set as its last one, the video opened.
fn restart_potplayer(
    pot: Option<Pot>,
    exe: &Path,
    skin: &Path,
    video: &Path,
    log: &mut Log,
) -> Result<Pot, String> {
    if let Some(pot) = pot {
        close_potplayer(pot, log);
    }
    let name = skin.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let status = Command::new("reg.exe")
        .args(["add", SKIN_KEY, "/v", "LastSkinName", "/t", "REG_SZ", "/d", &name, "/f"])
        .status()
        .map_err(|e| format!("reg.exe: {e}"))?;
    log.line(&format!("potplayer_skin_set={name} status={status}"));
    let pot = start_potplayer(exe, &[], log)?;
    open_in_potplayer(exe, video);
    sleep_ms(5000);
    Ok(pot)
}

fn potplayer_session(
    exe: &Path,
    skins: &[PathBuf],
    video: &Path,
    out: &Path,
    log: &mut Log,
) -> Result<(), String> {
    let mut pot = start_potplayer(exe, &[], log)?;
    open_in_potplayer(exe, video);
    sleep_ms(5000);
    let mut runtime = true;
    let mut result = Ok(());
    for (index, skin) in skins.iter().enumerate() {
        let name = stem(skin);
        let monitor = monitor_of(pot.hwnd).rcMonitor;
        place(pot.hwnd, &monitor);
        let before = window_rect(pot.hwnd);
        if runtime {
            let mark = title_mark(&before);
            open_in_potplayer(exe, skin);
            sleep_ms(3500);
            let dialog = close_dialogs(pot.child.id(), pot.hwnd, &name, out, log);
            let changed = title_mark(&window_rect(pot.hwnd)) != mark;
            log.line(&format!("potplayer_switch_check skin={name} dialog={dialog} changed={changed}"));
            if dialog || !changed {
                runtime = false;
                log.line("potplayer_switch=registry (PotPlayer did not take the opened skin)");
            }
        }
        if !runtime {
            pot = restart_potplayer(Some(pot), exe, skin, video, log)?;
            let monitor = monitor_of(pot.hwnd).rcMonitor;
            place(pot.hwnd, &monitor);
        }
        result = shots("potplayer", index, &name, pot.hwnd, &before, out, log);
        if result.is_err() {
            break;
        }
    }
    unsafe {
        SetWindowPos(pot.hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
    close_potplayer(pot, log);
    result
}

fn rvp_session(
    exe: &Path,
    skins: &[PathBuf],
    video: &Path,
    out: &Path,
    log: &mut Log,
) -> Result<(), String> {
    let config = out.join("rvp-config");
    let state = out.join("rvp-state");
    std::fs::create_dir_all(&config).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    // No IPC server (SVP attaches through one); RVP's log at Info in the
    // state folder.
    std::fs::write(
        config.join("settings.json"),
        "{ \"ipc-server\": \"\", \"log\": { \"level\": \"info\" } }\n",
    )
    .map_err(|e| e.to_string())?;
    let handover = |file: &Path| {
        let _ = Command::new(exe)
            .arg(CHANNEL)
            .arg(file)
            .env("RVP_CONFIG_DIR", &config)
            .env("RVP_STATE_DIR", &state)
            .spawn();
    };
    let mut child: Child = Command::new(exe)
        .arg(CHANNEL)
        .env("RVP_CONFIG_DIR", &config)
        .env("RVP_STATE_DIR", &state)
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    let Some(hwnd) = rvp_window(&child) else {
        close_rvp(&mut child, log);
        return Err("RVP's window did not appear".to_owned());
    };
    handover(video);
    sleep_ms(5000);
    let mut result = Ok(());
    for (index, skin) in skins.iter().enumerate() {
        let name = stem(skin);
        let monitor = monitor_of(hwnd).rcMonitor;
        place(hwnd, &monitor);
        let before = window_rect(hwnd);
        handover(skin);
        sleep_ms(3500);
        result = shots("rvp", index, &name, hwnd, &before, out, log);
        if result.is_err() {
            break;
        }
    }
    close_rvp(&mut child, log);
    result
}

/// The captures of one skin, from the window as the switch left it.
fn shots(
    player: &str,
    index: usize,
    skin: &str,
    hwnd: HWND,
    before: &RECT,
    out: &Path,
    log: &mut Log,
) -> Result<(), String> {
    let monitor = monitor_of(hwnd).rcMonitor;
    let park = (monitor.right - 2, monitor.bottom - 2);
    let file = |step: &str| out.join(format!("{player}-{index:02}-{skin}-{step}.png"));
    raise(hwnd);
    move_to(park.0, park.1);
    sleep_ms(800);
    let after = window_rect(hwnd);
    log.line(&format!(
        "switch player={player} skin={skin} before={} after={}",
        rect_text(before),
        rect_text(&after)
    ));
    require_shown(hwnd, &inside(&after), &format!("{skin} switched"))?;
    capture_to(log, &after, &file("a-switched"));

    place(hwnd, &monitor);
    move_to(park.0, park.1);
    sleep_ms(1200);
    let rect = window_rect(hwnd);
    log.line(&format!("sized player={player} skin={skin} window={}", rect_text(&rect)));
    require_shown(hwnd, &inside(&rect), &format!("{skin} 1280x720"))?;
    capture_to(log, &rect, &file("b-1280x720"));

    // The video's middle: inside the window, above the bars.
    let middle = ((rect.left + rect.right) / 2, rect.top + (rect.bottom - rect.top) / 3);
    require_shown(hwnd, &[middle], &format!("{skin} fullscreen click"))?;
    double_click(middle.0, middle.1);
    sleep_ms(2500);
    log.line(&format!(
        "fullscreen player={player} skin={skin} window={}",
        rect_text(&window_rect(hwnd))
    ));
    move_to((monitor.left + monitor.right) / 2, monitor.bottom - 40);
    sleep_ms(400);
    move_to((monitor.left + monitor.right) / 2 + 10, monitor.bottom - 30);
    raise(hwnd);
    sleep_ms(1500);
    require_shown(hwnd, &inside(&monitor), &format!("{skin} fullscreen bottom"))?;
    capture_to(log, &monitor, &file("c-full-bottom"));
    move_to((monitor.left + monitor.right) / 2, monitor.top + 40);
    sleep_ms(400);
    move_to((monitor.left + monitor.right) / 2 + 10, monitor.top + 30);
    raise(hwnd);
    sleep_ms(1500);
    require_shown(hwnd, &inside(&monitor), &format!("{skin} fullscreen top"))?;
    capture_to(log, &monitor, &file("d-full-top"));
    let back = (
        (monitor.left + monitor.right) / 2,
        monitor.top + (monitor.bottom - monitor.top) / 3,
    );
    require_shown(hwnd, &[back], &format!("{skin} windowed click"))?;
    double_click(back.0, back.1);
    sleep_ms(2500);
    log.line(&format!(
        "windowed player={player} skin={skin} window={}",
        rect_text(&window_rect(hwnd))
    ));
    Ok(())
}
