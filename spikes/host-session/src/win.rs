//! Win32 side of the host-session tool. Everything here is a throwaway
//! spike: raw Win32 calls through windows-sys, no error recovery beyond
//! putting PotPlayer's window back and closing it.

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
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput, VK_ESCAPE, VK_F5,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetCursorPos, GetMenuItemCount, GetMenuItemInfoW, GetWindowPlacement,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, HMENU, IsWindowVisible, MENUITEMINFOW,
    MFS_CHECKED, MFS_DISABLED, MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU,
    MN_GETHMENU, PostMessageW, SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetCursorPos, SetWindowPlacement,
    SetWindowPos, WINDOWPLACEMENT, WM_CLOSE,
};

const POT_CLASS: &str = "PotPlayer64";
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

fn key_input(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn move_to(x: i32, y: i32) {
    unsafe { SetCursorPos(x, y) };
    sleep_ms(60);
}

fn press_key(vk: u16) {
    send(&[key_input(vk, false), key_input(vk, true)]);
    sleep_ms(150);
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
            log.line(&format!("unexpected_dialog=\"{}\" (closed with Esc)", window_text(hwnd)));
            press_key(VK_ESCAPE);
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
    let mouse_script = options.get("--mouse-script").map(PathBuf::from);
    let sizes = parse_sizes(options.get("--sizes").unwrap_or("1280x720,1920x1080,2560x1080"));
    let mut log = Log::open(&out)?;
    require_no_potplayer()?;
    let pot = start_potplayer(&exe, &[], &mut log)?;
    let result = r1_session(&pot, &exe, &skin, &clip, &sizes, &out, &mut log)
        .and_then(|()| match &mouse_script {
            Some(script) => read_mouse_page(&pot, script, &out, &mut log),
            None => Ok(()),
        });
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
    for state in ["idle", "playing"] {
        if state == "playing" {
            let clip_text = clip.to_string_lossy().into_owned();
            let _ = Command::new(exe).args([clip_text.as_str(), "/current"]).spawn();
            log.line(&format!("opened={clip_text}"));
            sleep_ms(4000);
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
            let targets = crate::skin_targets(skin, width, height)?;
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
    // Context menu (playing state, last size): right click on the video.
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
    for _ in 0..4 {
        press_key(VK_ESCAPE);
    }
    log.line(&format!("menus_left_open={}", open_menus().len()));
    move_to(park.0, park.1);
    let _ = cursor();
    restore(log);
    Ok(())
}

/// Opens Preferences (F5), lets the UI Automation script read the mouse
/// page, captures it and closes the dialog without applying anything.
fn read_mouse_page(pot: &Pot, script: &Path, out: &Path, log: &mut Log) -> Result<(), String> {
    let pid = pot.child.id();
    let before = top_windows(None, Some(pid));
    press_key(VK_F5);
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
    let table = out.join("mouse-actions.tsv");
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .args(["-Hwnd", &(dialog as isize).to_string(), "-Out"])
        .arg(&table)
        .status()
        .map_err(|e| e.to_string())?;
    log.line(&format!("mouse_script={status} table={}", table.display()));
    capture_to(log, &window_rect(dialog), &out.join("preferences-mouse.png"));
    unsafe { PostMessageW(dialog, WM_CLOSE, 0, 0) };
    sleep_ms(800);
    let still_open = top_windows(None, Some(pid)).contains(&dialog);
    if still_open {
        press_key(VK_ESCAPE);
        sleep_ms(500);
    }
    log.line(&format!("preferences_closed={}", !top_windows(None, Some(pid)).contains(&dialog)));
    Ok(())
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
    let (command, rest) = args.split_first().ok_or("usage: host-session r1|s5|record|check ...")?;
    let options = Options::parse(rest);
    match command.as_str() {
        "r1" => r1(&options),
        "s5" => s5(&options),
        "record" => record(&options),
        "check" => check(),
        other => Err(format!("unknown command {other}")),
    }
}
