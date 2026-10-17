//! Win32 side of the host-session tool. Everything here is a throwaway
//! spike: raw Win32 calls through windows-sys, no error recovery beyond
//! putting PotPlayer's window back and closing it.

mod m10;

use std::ffi::c_void;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetMonitorInfoW, HDC, HMONITOR,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, ReleaseDC, SRCCOPY, SelectObject,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
// Only mouse input is injected: it goes to the window under the cursor.
// Keyboard input would go to the foreground window, which may be the
// terminal that runs this tool; window messages are used instead.
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GWL_STYLE, GetClassNameW, GetWindowLongW, SMTO_ABORTIFHUNG, SendMessageTimeoutW, GetCursorPos, GetMenuItemCount, GetMenuItemInfoW, GetWindowPlacement,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, HMENU, IsWindowVisible, MENUITEMINFOW,
    MFS_CHECKED, MFS_DISABLED, MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU,
    MN_GETHMENU, PostMessageW, SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetCursorPos, SetWindowPlacement,
    SetWindowPos, WINDOWPLACEMENT, WM_CANCELMODE, WM_CLOSE, WM_COMMAND, WM_KEYDOWN, WM_KEYUP,
};

const POT_CLASS: &str = "PotPlayer64";
/// PotPlayer's "Preferences..." command (menu item 101_0_23, F5).
const CMD_PREFERENCES: usize = 10010;
const POT_EXE: &str = "PotPlayerMini64.exe";

struct Log {
    file: std::fs::File,
}

impl Log {
    fn open(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join("session.log");
        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { file })
    }

    fn line(&mut self, text: &str) {
        println!("{text}");
        let _ = writeln!(self.file, "{text}");
    }
}

fn wide_to_string(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

// ---------------------------------------------------------------- windows

struct Found {
    class: Option<String>,
    pid: Option<u32>,
    windows: Vec<HWND>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> windows_sys::core::BOOL {
    // SAFETY: lparam is the &mut Found passed to EnumWindows below.
    let found = unsafe { &mut *(lparam as *mut Found) };
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    if let Some(class) = &found.class
        && class_name(hwnd) != *class
    {
        return 1;
    }
    if let Some(pid) = found.pid {
        let mut owner = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &raw mut owner) };
        if owner != pid {
            return 1;
        }
    }
    found.windows.push(hwnd);
    1
}

fn top_windows(class: Option<&str>, pid: Option<u32>) -> Vec<HWND> {
    let mut found = Found { class: class.map(str::to_owned), pid, windows: Vec::new() };
    unsafe { EnumWindows(Some(collect), (&raw mut found) as LPARAM) };
    found.windows
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn window_text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn window_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { GetWindowRect(hwnd, &raw mut rect) };
    rect
}

fn rect_text(rect: &RECT) -> String {
    format!("{},{} {}x{}", rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
}

fn placement(hwnd: HWND) -> WINDOWPLACEMENT {
    // SAFETY: WINDOWPLACEMENT is plain data; zero is a valid start value.
    let mut value: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    value.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    unsafe { GetWindowPlacement(hwnd, &raw mut value) };
    value
}

fn placement_text(value: &WINDOWPLACEMENT) -> String {
    format!(
        "show={} flags={} normal={}",
        value.showCmd,
        value.flags,
        rect_text(&value.rcNormalPosition)
    )
}

fn monitor_of(hwnd: HWND) -> MONITORINFO {
    // SAFETY: MONITORINFO is plain data.
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &raw mut info);
    }
    info
}

unsafe extern "system" fn list_monitor(
    monitor: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> windows_sys::core::BOOL {
    // SAFETY: lparam is the &mut Vec passed to EnumDisplayMonitors below.
    let list = unsafe { &mut *(lparam as *mut Vec<MONITORINFO>) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    unsafe { GetMonitorInfoW(monitor, &raw mut info) };
    list.push(info);
    1
}

fn monitors() -> Vec<MONITORINFO> {
    let mut list: Vec<MONITORINFO> = Vec::new();
    unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(list_monitor), (&raw mut list) as LPARAM);
    }
    list
}

fn processes_named(name: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return pids;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snapshot, &raw mut entry);
        while ok != 0 {
            if wide_to_string(&entry.szExeFile).eq_ignore_ascii_case(name) {
                pids.push(entry.th32ProcessID);
            }
            ok = Process32NextW(snapshot, &raw mut entry);
        }
        CloseHandle(snapshot);
    }
    pids
}

// ------------------------------------------------------------------ input

fn send(inputs: &[INPUT]) {
    unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
}

fn mouse_flags(flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn move_to(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y) };
    sleep_ms(60);
}

fn cursor() -> (i32, i32) {
    let mut point = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&raw mut point) };
    (point.x, point.y)
}

// ---------------------------------------------------------------- capture

/// Copies a screen rectangle (what is on screen, including video overlays
/// composed by DWM) to RGBA pixels.
fn capture(rect: &RECT) -> (u32, u32, Vec<u8>) {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(memory, &raw const info, DIB_RGB_COLORS, &raw mut bits, std::ptr::null_mut(), 0);
        let previous = SelectObject(memory, bitmap);
        BitBlt(memory, 0, 0, width, height, screen, rect.left, rect.top, SRCCOPY | CAPTUREBLT);
        if !bits.is_null() {
            let bgra = std::slice::from_raw_parts(bits as *const u8, rgba.len());
            for (out, pixel) in rgba.chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
                out.copy_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
            }
        }
        SelectObject(memory, previous);
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
    (width as u32, height as u32, rgba)
}

fn capture_to(log: &mut Log, rect: &RECT, path: &Path) {
    let (width, height, rgba) = capture(rect);
    match crate::save_png(path, width, height, &rgba) {
        Ok(()) => log.line(&format!("captured={} {}", path.display(), rect_text(rect))),
        Err(error) => log.line(&format!("capture_failed={} {error}", path.display())),
    }
}

// ------------------------------------------------------------------- menu

fn dump_menu(menu: HMENU, depth: usize, out: &mut String, count: &mut usize) {
    let items = unsafe { GetMenuItemCount(menu) };
    for index in 0..items.max(0) {
        // SAFETY: MENUITEMINFOW is plain data.
        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_SUBMENU | MIIM_STRING;
        unsafe { GetMenuItemInfoW(menu, index as u32, 1, &raw mut info) };
        let mut text = vec![0u16; info.cch as usize + 1];
        info.dwTypeData = text.as_mut_ptr();
        info.cch += 1;
        unsafe { GetMenuItemInfoW(menu, index as u32, 1, &raw mut info) };
        let label = if info.fType & MFT_SEPARATOR != 0 { "---".to_owned() } else { wide_to_string(&text) };
        let (name, shortcut) = label.split_once('\t').unwrap_or((label.as_str(), ""));
        let mut flags = String::new();
        if info.fState & MFS_DISABLED != 0 {
            flags.push_str("disabled ");
        }
        if info.fState & MFS_CHECKED != 0 {
            flags.push_str("checked ");
        }
        if !info.hSubMenu.is_null() {
            flags.push_str("submenu");
        }
        let _ = writeln!(
            out,
            "{depth}\t{}\t{}{name}\t{shortcut}\t{}",
            info.wID,
            "  ".repeat(depth),
            flags.trim_end()
        );
        *count += 1;
        if !info.hSubMenu.is_null() {
            dump_menu(info.hSubMenu, depth + 1, out, count);
        }
    }
}

fn open_menus() -> Vec<HWND> {
    top_windows(Some("#32768"), None)
}

// --------------------------------------------------------------- potplayer

struct Pot {
    child: Child,
    hwnd: HWND,
}

fn start_potplayer(exe: &Path, args: &[&str], log: &mut Log) -> Result<Pot, String> {
    let child = Command::new(exe).args(args).spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
    let pid = child.id();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(&hwnd) = top_windows(Some(POT_CLASS), Some(pid)).first() {
            log.line(&format!("potplayer_started pid={pid} window={}", rect_text(&window_rect(hwnd))));
            sleep_ms(1500);
            return Ok(Pot { child, hwnd });
        }
        sleep_ms(200);
    }
    Err("PotPlayer's main window did not appear".to_owned())
}

fn close_potplayer(mut pot: Pot, log: &mut Log) {
    for hwnd in top_windows(None, Some(pot.child.id())) {
        unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(Some(status)) = pot.child.try_wait() {
            log.line(&format!("potplayer_closed status={status}"));
            return;
        }
        sleep_ms(200);
    }
    log.line("potplayer_close=TIMEOUT (left running; close it by hand)");
}

fn dismiss_dialogs(pid: u32, main: HWND, log: &mut Log) {
    for hwnd in top_windows(None, Some(pid)) {
        if hwnd != main && class_name(hwnd) == "#32770" {
            log.line(&format!("unexpected_dialog=\"{}\" (closed)", window_text(hwnd)));
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
    }
}

fn require_no_potplayer() -> Result<(), String> {
    let running = processes_named(POT_EXE);
    if running.is_empty() {
        Ok(())
    } else {
        Err(format!("PotPlayer is running (pids {running:?}); close it first"))
    }
}

fn require_dpi(hwnd: HWND) -> Result<(), String> {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 96 { Ok(()) } else { Err(format!("display scaling is {}%, not 100%", dpi * 100 / 96)) }
}

// --------------------------------------------------------------------- r1

fn parse_sizes(text: &str) -> Vec<(i32, i32)> {
    text.split(',')
        .filter_map(|size| {
            let (w, h) = size.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .collect()
}

fn r1(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let exe = options.path("--potplayer")?;
    let skin = options.path("--skin")?;
    let clip = options.path("--clip")?;
    let mouse_page = options.get("--mouse-page").unwrap_or("yes") == "yes";
    let states: Vec<&str> =
        options.get("--states").unwrap_or("idle,playing").split(',').filter(|s| !s.is_empty() && *s != "none").collect();
    let menu = options.get("--menu").unwrap_or("yes") == "yes";
    let sizes = parse_sizes(options.get("--sizes").unwrap_or("1280x720,1920x1080,2560x1080"));
    let mut log = Log::open(&out)?;
    require_no_potplayer()?;
    let pot = start_potplayer(&exe, &[], &mut log)?;
    let result = r1_session(&pot, &exe, &skin, &clip, &sizes, &states, menu, &out, &mut log)
        .and_then(|()| if mouse_page { read_mouse_page(&pot, &out, &mut log) } else { Ok(()) });
    if let Err(error) = &result {
        log.line(&format!("r1_error={error}"));
    }
    close_potplayer(pot, &mut log);
    result
}

fn r1_session(
    pot: &Pot,
    exe: &Path,
    skin: &Path,
    clip: &Path,
    sizes: &[(i32, i32)],
    states: &[&str],
    menu: bool,
    out: &Path,
    log: &mut Log,
) -> Result<(), String> {
    let hwnd = pot.hwnd;
    let pid = pot.child.id();
    let saved = placement(hwnd);
    log.line(&format!("saved_placement={}", placement_text(&saved)));
    let restore = |log: &mut Log| {
        unsafe { SetWindowPlacement(hwnd, &raw const saved) };
        sleep_ms(500);
        let now = placement(hwnd);
        let same = now.rcNormalPosition.left == saved.rcNormalPosition.left
            && now.rcNormalPosition.top == saved.rcNormalPosition.top
            && now.rcNormalPosition.right == saved.rcNormalPosition.right
            && now.rcNormalPosition.bottom == saved.rcNormalPosition.bottom
            && now.showCmd == saved.showCmd;
        log.line(&format!("restored_placement={} same={same}", placement_text(&now)));
    };
    if let Err(error) = require_dpi(hwnd) {
        restore(log);
        return Err(error);
    }
    let monitor = monitor_of(hwnd).rcMonitor;
    log.line(&format!("monitor={}", rect_text(&monitor)));
    let park = (monitor.right - 2, monitor.bottom - 2);
    for &state in states {
        if state == "playing" {
            let clip_text = clip.to_string_lossy().into_owned();
            let _ = Command::new(exe).args([clip_text.as_str(), "/current"]).spawn();
            log.line(&format!("opened={clip_text}"));
            // SVP attaches and PotPlayer resizes the window to the video;
            // wait until that and SVP's on-screen message are over.
            sleep_ms(12000);
            let instances = processes_named(POT_EXE);
            if instances.len() > 1 {
                log.line(&format!("warning=the clip opened in another PotPlayer instance {instances:?}"));
            }
        }
        for &(width, height) in sizes {
            let x = monitor.left.max((monitor.right - width).min(monitor.left + 20));
            let y = monitor.top.max((monitor.bottom - height).min(monitor.top + 20));
            unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE) };
            move_to(park.0, park.1);
            sleep_ms(1500);
            let rect = window_rect(hwnd);
            log.line(&format!("size={width}x{height} window={}", rect_text(&rect)));
            // PotPlayer may keep the video's aspect ratio while playing, so
            // lay out at the size it actually took.
            let targets = crate::skin_targets(skin, rect.right - rect.left, rect.bottom - rect.top)?;
            let name = |suffix: &str| out.join(format!("{state}-{width}x{height}{suffix}.png"));
            capture_to(log, &rect, &name(""));
            let play = (rect.left + targets.play.0, rect.top + targets.play.1);
            move_to(play.0, play.1);
            sleep_ms(800);
            capture_to(log, &rect, &name("-hover"));
            // Press the play button, capture, then leave the button before
            // releasing so that the click is cancelled.
            send(&[mouse_flags(MOUSEEVENTF_LEFTDOWN)]);
            sleep_ms(400);
            capture_to(log, &rect, &name("-pressed"));
            move_to(rect.left + targets.video.0, rect.top + targets.video.1);
            sleep_ms(200);
            send(&[mouse_flags(MOUSEEVENTF_LEFTUP)]);
            sleep_ms(500);
            dismiss_dialogs(pid, hwnd, log);
            move_to(park.0, park.1);
        }
    }
    if !menu {
        restore(log);
        return Ok(());
    }
    // Context menu (last state and size): right click on the video.
    let rect = window_rect(hwnd);
    let targets = crate::skin_targets(skin, rect.right - rect.left, rect.bottom - rect.top)?;
    move_to(rect.left + targets.video.0, rect.top + targets.video.1);
    send(&[mouse_flags(MOUSEEVENTF_RIGHTDOWN), mouse_flags(MOUSEEVENTF_RIGHTUP)]);
    sleep_ms(1200);
    let menus = open_menus();
    if let Some(&menu_window) = menus.first() {
        let menu = unsafe { SendMessageW(menu_window, MN_GETHMENU, 0, 0) } as HMENU;
        let mut text = String::from("depth\tcommand_id\ttext\tshortcut\tflags\n");
        let mut count = 0;
        dump_menu(menu, 0, &mut text, &mut count);
        std::fs::write(out.join("menu-runtime.tsv"), text).map_err(|e| e.to_string())?;
        log.line(&format!("menu_items={count}"));
        capture_to(log, &window_rect(menu_window), &out.join("menu-top.png"));
    } else {
        log.line("menu=FAIL no popup menu window");
    }
    // WM_CANCELMODE ends the menu's modal loop in the owner window.
    unsafe { PostMessageW(hwnd, WM_CANCELMODE, 0, 0) };
    sleep_ms(500);
    // Fallback: Esc posted to each menu window (a message to that window,
    // not keyboard input, so it cannot reach another program).
    for _ in 0..4 {
        for menu_window in open_menus() {
            unsafe {
                PostMessageW(menu_window, WM_KEYDOWN, 0x1B, 0);
                PostMessageW(menu_window, WM_KEYUP, 0x1B, 0);
            }
        }
        sleep_ms(300);
    }
    log.line(&format!("menus_left_open={}", open_menus().len()));
    move_to(park.0, park.1);
    let _ = cursor();
    restore(log);
    Ok(())
}

// Common-control messages used on PotPlayer's Preferences dialog. Only
// messages without pointers (tree items are opaque handles) or messages the
// system marshals between processes (text into our own buffer) are sent, so
// nothing is written into PotPlayer's memory.
const WM_GETTEXT: u32 = 0x000D;
const WM_GETTEXTLENGTH: u32 = 0x000E;
const BM_GETCHECK: u32 = 0x00F0;
const CB_GETCOUNT: u32 = 0x0146;
const CB_GETCURSEL: u32 = 0x0147;
const CB_GETLBTEXT: u32 = 0x0148;
const CB_GETLBTEXTLEN: u32 = 0x0149;
const TVM_EXPAND: u32 = 0x1102;
const TVM_GETNEXTITEM: u32 = 0x110A;
const TVM_SELECTITEM: u32 = 0x110B;
const TVM_GETITEMSTATE: u32 = 0x1127;
const TVGN_ROOT: usize = 0x0;
const TVGN_NEXT: usize = 0x1;
const TVGN_CHILD: usize = 0x4;
const TVGN_CARET: usize = 0x9;
const TVE_COLLAPSE: usize = 0x1;
const TVIS_EXPANDED: isize = 0x20;
/// Text that appears only on PotPlayer's mouse page.
const MOUSE_PAGE_MARK: &str = "Left Button - Single Click";

fn send_message(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(hwnd, message, wparam, lparam, SMTO_ABORTIFHUNG, 2000, &raw mut result);
    }
    result as isize
}

fn control_text(hwnd: HWND) -> String {
    let length = send_message(hwnd, WM_GETTEXTLENGTH, 0, 0).max(0) as usize;
    let mut buffer = vec![0u16; length + 1];
    send_message(hwnd, WM_GETTEXT, buffer.len(), buffer.as_mut_ptr() as isize);
    wide_to_string(&buffer)
}

unsafe extern "system" fn collect_child(hwnd: HWND, lparam: LPARAM) -> windows_sys::core::BOOL {
    // SAFETY: lparam is the &mut Vec passed to EnumChildWindows below.
    let list = unsafe { &mut *(lparam as *mut Vec<HWND>) };
    list.push(hwnd);
    1
}

/// All visible descendants of `parent`.
fn visible_children(parent: HWND) -> Vec<HWND> {
    let mut list: Vec<HWND> = Vec::new();
    unsafe { EnumChildWindows(parent, Some(collect_child), (&raw mut list) as LPARAM) };
    list.retain(|&hwnd| unsafe { IsWindowVisible(hwnd) } != 0);
    list
}

fn page_shows(dialog: HWND, mark: &str) -> bool {
    visible_children(dialog).into_iter().any(|hwnd| control_text(hwnd).contains(mark))
}

fn tree_items(tree: HWND, parent: Option<isize>) -> Vec<isize> {
    let mut items = Vec::new();
    let mut item = match parent {
        None => send_message(tree, TVM_GETNEXTITEM, TVGN_ROOT, 0),
        Some(parent) => send_message(tree, TVM_GETNEXTITEM, TVGN_CHILD, parent),
    };
    while item != 0 {
        items.push(item);
        item = send_message(tree, TVM_GETNEXTITEM, TVGN_NEXT, item);
    }
    items
}

fn read_combos(dialog: HWND) -> String {
    let controls = visible_children(dialog);
    let statics: Vec<(HWND, RECT)> = controls
        .iter()
        .filter(|&&hwnd| class_name(hwnd) == "Static")
        .map(|&hwnd| (hwnd, window_rect(hwnd)))
        .collect();
    let mut out = String::from("label\tselected\tselected_index\tentries\n");
    for &combo in controls.iter().filter(|&&hwnd| class_name(hwnd) == "ComboBox") {
        let rect = window_rect(combo);
        let label = statics
            .iter()
            .filter(|(_, r)| r.right <= rect.left + 2 && r.top < rect.bottom && r.bottom > rect.top)
            .min_by_key(|(_, r)| rect.left - r.right)
            .map(|(hwnd, _)| control_text(*hwnd))
            .unwrap_or_default();
        let count = send_message(combo, CB_GETCOUNT, 0, 0).max(0) as usize;
        let selected = send_message(combo, CB_GETCURSEL, 0, 0);
        let mut entries = Vec::new();
        for index in 0..count {
            let length = send_message(combo, CB_GETLBTEXTLEN, index, 0).max(0) as usize;
            let mut buffer = vec![0u16; length + 1];
            send_message(combo, CB_GETLBTEXT, index, buffer.as_mut_ptr() as isize);
            entries.push(wide_to_string(&buffer));
        }
        let chosen = usize::try_from(selected).ok().and_then(|i| entries.get(i)).cloned().unwrap_or_default();
        let _ = writeln!(out, "{label}\t{chosen}\t{selected}\t{}", entries.join(" | "));
    }
    for &button in controls.iter().filter(|&&hwnd| class_name(hwnd) == "Button") {
        let style = unsafe { GetWindowLongW(button, GWL_STYLE) } & 0xF;
        if matches!(style, 2 | 3 | 5 | 6) {
            let state = send_message(button, BM_GETCHECK, 0, 0);
            let _ = writeln!(out, "checkbox\t{}\t{state}", control_text(button));
        }
    }
    out
}

/// Opens Preferences (its menu command), finds the mouse page in the page
/// tree, reads its combo boxes and check boxes, captures it, puts the tree
/// back as it was and closes the dialog without applying anything.
fn read_mouse_page(pot: &Pot, out: &Path, log: &mut Log) -> Result<(), String> {
    let pid = pot.child.id();
    let before = top_windows(None, Some(pid));
    unsafe { PostMessageW(pot.hwnd, WM_COMMAND, CMD_PREFERENCES, 0) };
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut dialog = None;
    while Instant::now() < deadline && dialog.is_none() {
        sleep_ms(250);
        dialog = top_windows(None, Some(pid))
            .into_iter()
            .find(|hwnd| !before.contains(hwnd) && class_name(*hwnd) == "#32770");
    }
    let dialog = dialog.ok_or("the Preferences dialog did not open")?;
    log.line(&format!("preferences=\"{}\" {}", window_text(dialog), rect_text(&window_rect(dialog))));
    sleep_ms(1500);
    capture_to(log, &window_rect(dialog), &out.join("preferences-opened.png"));
    let result = (|| {
        let tree = visible_children(dialog)
            .into_iter()
            .find(|&hwnd| class_name(hwnd) == "SysTreeView32")
            .ok_or("no page tree in the dialog")?;
        let original = send_message(tree, TVM_GETNEXTITEM, TVGN_CARET, 0);
        let roots = tree_items(tree, None);
        let expanded_before: Vec<bool> = roots
            .iter()
            .map(|&root| send_message(tree, TVM_GETITEMSTATE, root as usize, TVIS_EXPANDED) & TVIS_EXPANDED != 0)
            .collect();
        let mut found = false;
        'search: for &root in &roots {
            for item in std::iter::once(root).chain(tree_items(tree, Some(root))) {
                send_message(tree, TVM_SELECTITEM, TVGN_CARET, item);
                sleep_ms(400);
                if page_shows(dialog, MOUSE_PAGE_MARK) {
                    found = true;
                    break 'search;
                }
            }
        }
        if found {
            let table = read_combos(dialog);
            std::fs::write(out.join("mouse-actions.tsv"), &table).map_err(|e| e.to_string())?;
            log.line(&format!("mouse_page=found rows={}", table.lines().count() - 1));
            capture_to(log, &window_rect(dialog), &out.join("preferences-mouse.png"));
        } else {
            log.line("mouse_page=NOT FOUND");
        }
        send_message(tree, TVM_SELECTITEM, TVGN_CARET, original);
        sleep_ms(400);
        for (&root, &was_expanded) in roots.iter().zip(&expanded_before) {
            let expanded = send_message(tree, TVM_GETITEMSTATE, root as usize, TVIS_EXPANDED) & TVIS_EXPANDED != 0;
            if expanded && !was_expanded {
                send_message(tree, TVM_EXPAND, TVE_COLLAPSE, root);
            }
        }
        sleep_ms(300);
        let now = send_message(tree, TVM_GETNEXTITEM, TVGN_CARET, 0);
        log.line(&format!("page_restored={}", now == original));
        capture_to(log, &window_rect(dialog), &out.join("preferences-restored.png"));
        Ok::<(), String>(())
    })();
    unsafe { PostMessageW(dialog, WM_CLOSE, 0, 0) };
    sleep_ms(800);
    log.line(&format!("preferences_closed={}", !top_windows(None, Some(pid)).contains(&dialog)));
    result
}

// --------------------------------------------------------------- s3-input

/// Left-drags with the real mouse from `start` by `delta` (screen pixels).
fn drag(start: (i32, i32), delta: (i32, i32)) {
    move_to(start.0, start.1);
    send(&[mouse_flags(MOUSEEVENTF_LEFTDOWN)]);
    sleep_ms(150);
    for step in 1..=10 {
        move_to(start.0 + delta.0 * step / 10, start.1 + delta.1 * step / 10);
        sleep_ms(30);
    }
    sleep_ms(200);
    send(&[mouse_flags(MOUSEEVENTF_LEFTUP)]);
    sleep_ms(600);
}

/// S3 on Windows with real input: the frameless window moves when its
/// title area is dragged, resizes from its corner grip, and its context
/// menu is a separate window larger than the window.
fn s3_input(options: &Options) -> Result<(), String> {
    let exe = options.path("--s3")?;
    let skin = options.path("--skin")?;
    let child = Command::new(&exe)
        .args(["--skin", &skin.to_string_lossy(), "--width", "440", "--height", "77"])
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    let pid = child.id();
    let mut child = child;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut main = None;
        while Instant::now() < deadline && main.is_none() {
            sleep_ms(250);
            main = top_windows(None, Some(pid)).into_iter().find(|&hwnd| window_text(hwnd) == "RVP spike S3");
        }
        let main = main.ok_or("the S3 window did not appear")?;
        sleep_ms(1500);
        let start = window_rect(main);
        println!("window={}", rect_text(&start));
        drag((start.left + 150, start.top + 12), (80, 60));
        let moved = window_rect(main);
        let (dx, dy) = (moved.left - start.left, moved.top - start.top);
        let same_size = moved.right - moved.left == start.right - start.left;
        println!(
            "input_check_move={} moved={dx},{dy} expected=80,60 window={}",
            if (dx, dy) == (80, 60) && same_size { "PASS" } else { "FAIL" },
            rect_text(&moved)
        );
        drag((moved.right - 6, moved.bottom - 6), (100, 80));
        let resized = window_rect(main);
        let (dw, dh) = (
            (resized.right - resized.left) - (moved.right - moved.left),
            (resized.bottom - resized.top) - (moved.bottom - moved.top),
        );
        println!(
            "input_check_resize={} grown={dw},{dh} expected=100,80 window={}",
            if (dw - 100).abs() <= 2 && (dh - 80).abs() <= 2 { "PASS" } else { "FAIL" },
            rect_text(&resized)
        );
        let before = top_windows(None, Some(pid));
        move_to(resized.left + 200, resized.top + 50);
        send(&[mouse_flags(MOUSEEVENTF_RIGHTDOWN), mouse_flags(MOUSEEVENTF_RIGHTUP)]);
        sleep_ms(1200);
        let menu = top_windows(None, Some(pid)).into_iter().find(|hwnd| !before.contains(hwnd));
        match menu {
            Some(menu) => {
                let rect = window_rect(menu);
                let main_height = resized.bottom - resized.top;
                let ok = rect.bottom - rect.top > main_height;
                println!(
                    "input_check_menu={} menu={} class={} main_height={main_height}",
                    if ok { "PASS" } else { "FAIL" },
                    rect_text(&rect),
                    class_name(menu)
                );
            }
            None => println!("input_check_menu=FAIL no menu window"),
        }
        Ok::<(), String>(())
    })();
    // The spike has no clean shutdown path (see docs/spikes/s3-qt-shell.md).
    let _ = child.kill();
    let _ = child.wait();
    move_to(0, 0);
    result
}

// --------------------------------------------------------------------- s5

struct Recorder {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    description: String,
}

fn start_recording(path: &Path) -> Result<Recorder, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no default output device")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let name = device.description().map(|d| d.to_string()).unwrap_or_default();
    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let writer = hound::WavWriter::create(path, spec).map_err(|e| format!("{}: {e}", path.display()))?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let sink = Arc::clone(&writer);
    let format = config.sample_format();
    if format != cpal::SampleFormat::F32 {
        return Err(format!("output device format {format:?} is not f32"));
    }
    let stream = device
        .build_input_stream::<f32, _, _>(
            config.config(),
            move |data: &[f32], _| {
                if let Ok(mut guard) = sink.lock()
                    && let Some(writer) = guard.as_mut()
                {
                    for &sample in data {
                        let _ = writer.write_sample(sample);
                    }
                }
            },
            |error| eprintln!("loopback error: {error}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let description = format!("device=\"{name}\" rate={} channels={} format=f32", spec.sample_rate, spec.channels);
    Ok(Recorder { stream, writer, description })
}

fn stop_recording(recorder: Recorder) -> Result<(), String> {
    drop(recorder.stream);
    let writer = recorder.writer.lock().map_err(|e| e.to_string())?.take();
    match writer {
        Some(writer) => writer.finalize().map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

fn s5(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let exe = options.path("--potplayer")?;
    let mut log = Log::open(&out)?;
    require_no_potplayer()?;
    for clip in options.all("--clip") {
        let (file, seconds) = clip.rsplit_once(':').ok_or("--clip needs FILE:SECONDS")?;
        let seconds: u64 = seconds.parse().map_err(|_| "--clip needs FILE:SECONDS")?;
        let stem = Path::new(file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let wav = out.join(format!("potplayer-{stem}.wav"));
        let recorder = start_recording(&wav)?;
        log.line(&format!("recording={} {}", wav.display(), recorder.description));
        sleep_ms(1000);
        let started = Instant::now();
        let pot = start_potplayer(&exe, &[file], &mut log)?;
        log.line(&format!("playing={file} seconds={seconds}"));
        while started.elapsed() < Duration::from_secs(seconds + 3) {
            sleep_ms(250);
        }
        close_potplayer(pot, &mut log);
        sleep_ms(500);
        stop_recording(recorder)?;
        log.line(&format!("recorded={}", wav.display()));
        sleep_ms(2000);
    }
    Ok(())
}

// ------------------------------------------------------------------ audio

/// PotPlayer's remote messages, as SVP and remote controls use them:
/// `SendMessage(window, WM_USER, command, value)`.
const WM_USER: u32 = 0x0400;
const POT_GET_VOLUME: usize = 0x5000;
const POT_SET_VOLUME: usize = 0x5001;
/// PotPlayer's "Increase Volume" and "Decrease Volume" commands (R1), one
/// `AudioVolumeUnit` (the owner's 1 %) each.
const CMD_VOLUME_UP: usize = 10035;
const CMD_VOLUME_DOWN: usize = 10036;
const RVP_EXE: &str = "rvp.exe";

/// The session's clock: seconds since the recording started, for the log.
struct Clock(Instant);

impl Clock {
    fn now(&self) -> String {
        format!("{:.3}", self.0.elapsed().as_secs_f64())
    }
}

/// PotPlayer's volume (0-100), or None when it does not answer.
fn pot_volume(hwnd: HWND) -> Option<isize> {
    let value = send_message(hwnd, WM_USER, POT_GET_VOLUME, 0);
    (0..=1000).contains(&value).then_some(value)
}

/// Sets PotPlayer's volume; returns what PotPlayer reports afterwards.
/// Without the remote messages it steps with the volume commands from
/// `current`.
fn set_pot_volume(hwnd: HWND, target: isize, current: &mut isize, remote: bool) -> Option<isize> {
    if remote {
        send_message(hwnd, WM_USER, POT_SET_VOLUME, target);
        sleep_ms(50);
        let read = pot_volume(hwnd);
        if let Some(read) = read {
            *current = read;
        }
        return read;
    }
    let (command, step) = if target > *current { (CMD_VOLUME_UP, 1) } else { (CMD_VOLUME_DOWN, -1) };
    while *current != target {
        unsafe { PostMessageW(hwnd, WM_COMMAND, command, 0) };
        *current += step;
        sleep_ms(15);
    }
    None
}

/// Runs RVP with settings and state of its own for `seconds`, then closes
/// its windows as their close buttons would.
fn rvp_run(rvp: &Path, home: &Path, settings: &str, volume: Option<f64>, file: &Path, seconds: u64, clock: &Clock, log: &mut Log, name: &str) -> Result<(), String> {
    let config = home.join(name).join("config");
    let state = home.join(name).join("state");
    let _ = std::fs::remove_dir_all(home.join(name));
    std::fs::create_dir_all(&config).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    std::fs::write(config.join("settings.json"), settings).map_err(|e| e.to_string())?;
    if let Some(volume) = volume {
        std::fs::write(state.join("state.json"), format!("{{ \"version\": 1, \"volume\": {volume} }}\n"))
            .map_err(|e| e.to_string())?;
    }
    let mut child = Command::new(rvp)
        .args(["--instance=audio-audit", "--skin="])
        .arg(file)
        .env("RVP_CONFIG_DIR", &config)
        .env("RVP_STATE_DIR", &state)
        .spawn()
        .map_err(|e| format!("{}: {e}", rvp.display()))?;
    log.line(&format!("segment {name} start t={} file={}", clock.now(), file.display()));
    sleep_ms(seconds * 1000);
    log.line(&format!("segment {name} end t={}", clock.now()));
    let windows: Vec<HWND> = top_windows(None, Some(child.id()))
        .into_iter()
        .filter(|&hwnd| class_name(hwnd).contains("QWindow"))
        .collect();
    for &hwnd in &windows {
        unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            log.line(&format!("rvp_closed {name} windows={} status={status}", windows.len()));
            sleep_ms(1000);
            return Ok(());
        }
        sleep_ms(200);
    }
    let _ = child.kill();
    log.line(&format!("rvp_close=TIMEOUT {name} (stopped)"));
    Ok(())
}

/// Plays `file` in PotPlayer at volume 100 for `seconds`, then puts the
/// volume back to what PotPlayer had.
fn pot_play(exe: &Path, file: &Path, seconds: u64, remote: bool, clock: &Clock, log: &mut Log, name: &str) -> Result<(), String> {
    let pot = start_potplayer(exe, &[&file.to_string_lossy()], log)?;
    let before = pot_volume(pot.hwnd);
    let mut current = before.unwrap_or(22);
    let read = set_pot_volume(pot.hwnd, 100, &mut current, remote);
    log.line(&format!("segment {name} start t={} volume_before={before:?} set=100 read={read:?}", clock.now()));
    sleep_ms(seconds * 1000);
    log.line(&format!("segment {name} end t={}", clock.now()));
    let restored = set_pot_volume(pot.hwnd, before.unwrap_or(22), &mut current, remote);
    log.line(&format!("potplayer_volume_restored read={restored:?}"));
    sleep_ms(300);
    close_potplayer(pot, log);
    sleep_ms(1500);
    Ok(())
}

/// Records the default output device while RVP and PotPlayer play the test
/// files: RVP's references (normalizer on and off, volume 100, 50 and 22),
/// PotPlayer's volume from 100 % down (its normalizer as the owner has it),
/// and both players on the 5.1 files. The log gives each segment's time.
fn audio(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let exe = options.path("--potplayer")?;
    let rvp = options.path("--rvp")?;
    let files = options.path("--files")?;
    let hold: u64 = options.get("--hold-ms").unwrap_or("2500").parse().map_err(|_| "bad --hold-ms")?;
    let levels: Vec<isize> = options
        .get("--levels")
        .unwrap_or("100,90,80,70,60,50,40,30,25,22,20,15,10,7,5,3,2,1,0,150,200")
        .split(',')
        .map(|v| v.trim().parse().map_err(|_| format!("bad level {v}")))
        .collect::<Result<_, _>>()?;
    require_no_potplayer()?;
    let others = processes_named(RVP_EXE);
    if !others.is_empty() {
        return Err(format!("RVP is running (pids {others:?}); close it first, its sound would be recorded"));
    }
    let mut log = Log::open(&out)?;
    let stereo = files.join("tone-stereo.flac");
    let surround = files.join("tone-51.wav");
    let ac3 = files.join("tone-51-ac3.mkv");
    for file in [&stereo, &surround, &ac3] {
        if !file.exists() {
            return Err(format!("{} is missing", file.display()));
        }
    }
    let wav = out.join("audio.wav");
    let recorder = start_recording(&wav)?;
    let clock = Clock(Instant::now());
    log.line(&format!("recording={} {}", wav.display(), recorder.description));
    let home = out.join("rvp");
    let on = "{ \"ipc-server\": \"\" }\n";
    let off = "{ \"ipc-server\": \"\", \"normalizer\": { \"enabled\": false } }\n";
    sleep_ms(1000);
    rvp_run(&rvp, &home, on, None, &stereo, 7, &clock, &mut log, "rvp-on-100")?;
    rvp_run(&rvp, &home, off, None, &stereo, 7, &clock, &mut log, "rvp-off-100")?;
    rvp_run(&rvp, &home, off, Some(50.0), &stereo, 7, &clock, &mut log, "rvp-off-50")?;
    rvp_run(&rvp, &home, off, Some(22.0), &stereo, 7, &clock, &mut log, "rvp-off-22")?;
    rvp_run(&rvp, &home, on, None, &surround, 25, &clock, &mut log, "rvp-on-51")?;
    rvp_run(&rvp, &home, on, None, &ac3, 25, &clock, &mut log, "rvp-on-51-ac3")?;

    // PotPlayer's volume, its normalizer as the owner has it.
    let pot = start_potplayer(&exe, &[&stereo.to_string_lossy()], &mut log)?;
    let before = pot_volume(pot.hwnd);
    let remote = before.is_some();
    log.line(&format!("potplayer_volume_before={before:?} remote_messages={remote}"));
    let mut current = before.unwrap_or(22);
    if !remote {
        // Down to 0 with the volume commands, then up the levels in order.
        set_pot_volume(pot.hwnd, 0, &mut current, false);
        current = 0;
    }
    sleep_ms(1500);
    let mut sweep = levels.clone();
    if !remote {
        sweep.retain(|&v| v <= 100);
        sweep.sort_unstable();
    }
    for level in sweep {
        let read = set_pot_volume(pot.hwnd, level, &mut current, remote);
        log.line(&format!("segment pot-stereo-{level} start t={} set={level} read={read:?}", clock.now()));
        sleep_ms(hold);
        log.line(&format!("segment pot-stereo-{level} end t={}", clock.now()));
    }
    let restored = set_pot_volume(pot.hwnd, before.unwrap_or(22), &mut current, remote);
    log.line(&format!("potplayer_volume_restored read={restored:?}"));
    sleep_ms(300);
    close_potplayer(pot, &mut log);
    sleep_ms(1500);
    pot_play(&exe, &surround, 25, remote, &clock, &mut log, "pot-on-51")?;
    pot_play(&exe, &ac3, 25, remote, &clock, &mut log, "pot-on-51-ac3")?;
    sleep_ms(500);
    stop_recording(recorder)?;
    log.line(&format!("recorded={} t={}", wav.display(), clock.now()));
    Ok(())
}

// ------------------------------------------------------------------ other

fn record(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let seconds: u64 = options.get("--seconds").unwrap_or("5").parse().map_err(|_| "bad --seconds")?;
    let recorder = start_recording(&out)?;
    println!("recording={} {}", out.display(), recorder.description);
    std::thread::sleep(Duration::from_secs(seconds));
    stop_recording(recorder)?;
    println!("recorded={}", out.display());
    Ok(())
}

fn check() -> Result<(), String> {
    for (index, info) in monitors().iter().enumerate() {
        println!("monitor{index}={} work={}", rect_text(&info.rcMonitor), rect_text(&info.rcWork));
    }
    println!("potplayer_pids={:?}", processes_named(POT_EXE));
    println!("svp_pids={:?}", processes_named("SVPManager.exe"));
    if let Ok(device) = cpal::default_host().default_output_device().ok_or(()) {
        let name = device.description().map(|d| d.to_string()).unwrap_or_default();
        let config = device.default_output_config().map(|c| format!("{c:?}")).unwrap_or_default();
        println!("output_device=\"{name}\" {config}");
    }
    Ok(())
}

struct Options {
    pairs: Vec<(String, String)>,
}

impl Options {
    fn parse(args: &[String]) -> Self {
        let mut pairs = Vec::new();
        let mut iter = args.iter();
        while let Some(key) = iter.next() {
            if let Some(value) = iter.next() {
                pairs.push((key.clone(), value.clone()));
            }
        }
        Self { pairs }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn all(&self, key: &str) -> Vec<&str> {
        self.pairs.iter().filter(|(k, _)| k == key).map(|(_, v)| v.as_str()).collect()
    }

    fn path(&self, key: &str) -> Result<PathBuf, String> {
        self.get(key).map(PathBuf::from).ok_or_else(|| format!("{key} is required"))
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let (command, rest) = args.split_first().ok_or("usage: host-session r1|s5|audio|m10|record|check ...")?;
    let options = Options::parse(rest);
    match command.as_str() {
        "r1" => r1(&options),
        "s5" => s5(&options),
        "audio" => audio(&options),
        "m10" => m10::m10(&options),
        "record" => record(&options),
        "check" => check(),
        "s3-input" => s3_input(&options),
        other => Err(format!("unknown command {other}")),
    }
}
