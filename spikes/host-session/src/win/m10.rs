//! `host-session m10` (goal L): PotPlayer's audio settings and output for
//! M10's comparison with RVP (docs/audits/audio.md, "M10").
//!
//! One session, in this order:
//!
//! 1. PotPlayer starts paused on `m10-bands.wav`; its context menu is read
//!    (the checked states: equalizer, normalizer, preset, speakers,
//!    effects) and its audio Preferences pages (combo lists, sliders, edits,
//!    check boxes, the values the labels show), then the dialog closes
//!    without applying anything. Audio Stretching's OSD is captured.
//! 2. Recording the default output device (WASAPI loopback) from here:
//!    the 19 presets, one per pass of `m10-bands.wav` (normalizer off,
//!    equalizer on, volume 100);
//! 3. the six effects, one per pass of `m10-effects.wav` after a pass
//!    without (equalizer and normalizer off);
//! 4. five speaker modes on `m10-51.wav` after a pass of 2.0 Stereo;
//! 5. the normalizer on, a fresh PotPlayer per `block-*` file (its block
//!    for each codec).
//! 6. Everything switched back to the states read in step 1 and the menu
//!    read again (`menu-after.tsv`).
//! 7. With `--rvp` and `--second`: RVP plays `m10-clicks.wav` to the
//!    default device and, as its second device, to the one whose name
//!    contains `--second`, while both are recorded with each buffer's
//!    capture time (QPC), so their offset can be measured.
//!
//! The passes of a file are switched by PotPlayer's playback position
//! (its remote message `0x5004`), in the silence that leads each pass
//! (`tools/audio-audit/make_m10.py`). Settings change only through
//! PotPlayer's own menu commands (`WM_COMMAND`); its registry key and
//! playlist folder are saved before the session and written back after it
//! (by hand, as in the audit).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetDlgCtrlID, GetWindowLongW, HMENU, MN_GETHMENU, PostMessageW, SendMessageW,
    WM_CANCELMODE, WM_CLOSE, WM_COMMAND, WM_KEYDOWN, WM_KEYUP,
};

use super::{
    BM_GETCHECK, CB_GETCOUNT, CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN, CMD_PREFERENCES, Clock,
    Log, Options, POT_EXE, Pot, RVP_EXE, TVGN_CARET, TVM_GETNEXTITEM, TVM_SELECTITEM, WM_USER,
    capture_to, class_name, close_potplayer, control_text, cursor, dump_menu, mouse_flags, move_to,
    open_menus, pot_volume, processes_named, rect_text, require_no_potplayer, send, send_message,
    set_pot_volume, sleep_ms,
    start_potplayer, start_recording, stop_recording, top_windows, tree_items, visible_children,
    window_rect, window_text,
};

const POT_GET_CURRENT_TIME: usize = 0x5004;
const POT_SET_CURRENT_TIME: usize = 0x5005;
const POT_GET_PLAY_STATUS: usize = 0x5006;
const POT_SET_PLAY_STATUS: usize = 0x5007;
const PLAY_PAUSED: isize = 1;
const PLAY_RUNNING: isize = 2;
const WM_CONTEXTMENU: u32 = 0x007B;
const TBM_GETPOS: u32 = WM_USER;
const TBM_GETRANGEMIN: u32 = WM_USER + 1;
const TBM_GETRANGEMAX: u32 = WM_USER + 2;

/// PotPlayer's menu commands (R1, `docs/r1/menu-runtime.tsv`).
const CMD_EQUALIZER: usize = 10095;
const CMD_NORMALIZER: usize = 10009;
const FIRST_PRESET: usize = 36600;
const PRESETS: usize = 19;
const EFFECTS: [(usize, &str); 6] = [
    (10389, "crystality"),
    (10390, "noise-reduction"),
    (10096, "freeverb"),
    (10438, "voice-removal"),
    (10439, "voice-emphasis"),
    (10661, "crossfeed"),
];
const STEREO: usize = 35301;
const SPEAKER_MODES: [(usize, &str); 5] = [
    (35323, "virtual-headphone"),
    (35324, "virtual-surround"),
    (35309, "dolby-surround"),
    (35310, "dolby-pro-logic-ii"),
    (35325, "virtual-dolby-decoder"),
];
const FIRST_SPEAKERS: usize = 35300;
const LAST_SPEAKERS: usize = 35325;
const STRETCH_UP: usize = 11111;
const STRETCH_DOWN: usize = 11110;

/// The passes of `make_m10.py`'s files, in milliseconds.
const BANDS_PASS: i64 = 5250;
const EFFECTS_PASS: i64 = 8000;
const SPEAKERS_PASS: i64 = 7500;
/// A pass's setting is sent this long before the pass starts: in the
/// silence that ends the pass before it.
const SWITCH_EARLY: i64 = 150;
/// When PotPlayer reports no position, by the wall clock this long after
/// the pass starts (playback's start lags the clock).
const SWITCH_LATE: i64 = 200;
const BLOCK_FILES: [&str; 8] = [
    "block-mp3.mp3",
    "block-aac.m4a",
    "block-opus.opus",
    "block-pcm16.wav",
    "block-ac3-stereo.mkv",
    "block-eac3.mkv",
    "block-dts.mkv",
    "block-flac44.flac",
];
const BLOCK_SECONDS: u64 = 10;

/// What a page of Preferences shows only it, to find the page.
const AUDIO_PAGES: [(&str, &str); 6] = [
    ("audio", "2nd Audio Renderer:"),
    ("volume-control", "Enable individual volume control"),
    ("equalizer", "Equalizer"),
    (
        "crystality-noise-reduction",
        "Crystallization (operates only in 16-bit sample)",
    ),
    ("normalizer-freeverb", "Normalizer (Auto Volume Control)"),
    ("language-sync-other", "Audio sync control:"),
];

/// The checked items of PotPlayer's context menu, by command.
#[derive(Debug, Clone, Default)]
struct MenuStates {
    checked: Vec<usize>,
}

impl MenuStates {
    fn parse(dump: &str) -> Self {
        let checked = dump
            .lines()
            .skip(1)
            .filter_map(|line| {
                let fields: Vec<&str> = line.split('\t').collect();
                let id: usize = fields.get(1)?.parse().ok()?;
                fields
                    .get(4)
                    .filter(|flags| flags.contains("checked"))
                    .map(|_| id)
            })
            .collect();
        Self { checked }
    }

    fn is_checked(&self, command: usize) -> bool {
        self.checked.contains(&command)
    }

    fn first_checked(&self, range: std::ops::RangeInclusive<usize>) -> Option<usize> {
        self.checked.iter().copied().find(|id| range.contains(id))
    }
}

pub(super) fn m10(options: &Options) -> Result<(), String> {
    let out = options.path("--out")?;
    let exe = options.path("--potplayer")?;
    let files = options.path("--files")?;
    let rvp = options.get("--rvp").map(PathBuf::from);
    let second = options.get("--second").map(str::to_owned);
    require_no_potplayer()?;
    let others = processes_named(RVP_EXE);
    if !others.is_empty() {
        return Err(format!(
            "RVP is running (pids {others:?}); close it first, its sound would be recorded"
        ));
    }
    let needed = [
        "m10-bands.wav",
        "m10-effects.wav",
        "m10-51.wav",
        "m10-clicks.wav",
    ];
    for name in needed.iter().chain(BLOCK_FILES.iter()) {
        if !files.join(name).exists() {
            return Err(format!("{} is missing", files.join(name).display()));
        }
    }
    let mut log = Log::open(&out)?;

    // 1. The states and Preferences, paused, before any recording.
    let pot = start_paused(&exe, &files.join("m10-bands.wav"), &mut log)?;
    let before = match read_menu(&pot, &out.join("menu-before.tsv"), &mut log) {
        Ok(states) => states,
        Err(error) => {
            close_potplayer(pot, &mut log);
            return Err(error);
        }
    };
    log.line(&format!("states_before={:?}", before.checked));
    let pages = read_audio_pages(&pot, &out, &mut log);
    if let Err(error) = &pages {
        log.line(&format!("preferences_error={error}"));
    }
    stretch_osd(&pot, &out, &mut log);
    let volume_before = pot_volume(pot.hwnd);
    log.line(&format!("volume_before={volume_before:?}"));

    // 2.-5. Recorded.
    let wav = out.join("m10-potplayer.wav");
    let recorder = start_recording(&wav)?;
    let clock = Clock(Instant::now());
    log.line(&format!(
        "recording={} {}",
        wav.display(),
        recorder.description
    ));
    let mut states = before.clone();
    let result = (|| {
        set_toggle(&pot, &mut states, CMD_NORMALIZER, false, &mut log);
        set_toggle(&pot, &mut states, CMD_EQUALIZER, true, &mut log);
        let mut current = volume_before.unwrap_or(100);
        set_pot_volume(pot.hwnd, 100, &mut current, volume_before.is_some());
        let commands: Vec<(usize, String)> = (0..PRESETS)
            .map(|index| (FIRST_PRESET + index, format!("preset-{index}")))
            .collect();
        play_passes(
            &pot,
            BANDS_PASS,
            &commands,
            &[],
            &clock,
            &mut log,
            "presets",
        )?;
        set_toggle(&pot, &mut states, CMD_EQUALIZER, false, &mut log);
        close_potplayer(pot, &mut log);
        sleep_ms(1500);

        let pot = start_paused(&exe, &files.join("m10-effects.wav"), &mut log)?;
        // The passes switch each effect on and off: all start off.
        for (command, _) in EFFECTS {
            set_toggle(&pot, &mut states, command, false, &mut log);
        }
        let mut commands = vec![(0, "effect-none".to_owned())];
        commands.extend(
            EFFECTS
                .iter()
                .map(|(id, name)| (*id, format!("effect-{name}"))),
        );
        // Each effect is switched off again when its pass ends.
        let toggles: Vec<usize> = EFFECTS.iter().map(|(id, _)| *id).collect();
        play_passes(
            &pot,
            EFFECTS_PASS,
            &commands,
            &toggles,
            &clock,
            &mut log,
            "effects",
        )?;
        close_potplayer(pot, &mut log);
        sleep_ms(1500);

        let pot = start_paused(&exe, &files.join("m10-51.wav"), &mut log)?;
        let mut commands = vec![(STEREO, "speakers-stereo".to_owned())];
        commands.extend(
            SPEAKER_MODES
                .iter()
                .map(|(id, name)| (*id, format!("speakers-{name}"))),
        );
        play_passes(
            &pot,
            SPEAKERS_PASS,
            &commands,
            &[],
            &clock,
            &mut log,
            "speakers",
        )?;
        let speakers = before
            .first_checked(FIRST_SPEAKERS..=LAST_SPEAKERS)
            .unwrap_or(STEREO);
        post_command(&pot, speakers, &mut log);
        close_potplayer(pot, &mut log);
        sleep_ms(1500);

        for name in BLOCK_FILES {
            let pot = start_paused(&exe, &files.join(name), &mut log)?;
            if name == BLOCK_FILES[0] {
                set_toggle(&pot, &mut states, CMD_NORMALIZER, true, &mut log);
            }
            send_message(pot.hwnd, WM_USER, POT_SET_PLAY_STATUS, PLAY_RUNNING);
            log.line(&format!("segment block {name} start t={}", clock.now()));
            sleep_ms(BLOCK_SECONDS * 1000);
            log.line(&format!("segment block {name} end t={}", clock.now()));
            close_potplayer(pot, &mut log);
            sleep_ms(1500);
        }
        Ok::<(), String>(())
    })();
    sleep_ms(500);
    stop_recording(recorder)?;
    log.line(&format!("recorded={} t={}", wav.display(), clock.now()));
    if let Err(error) = &result {
        log.line(&format!("session_error={error}"));
        // A PotPlayer the failed step left open closes before the states
        // are put back.
        close_all(&mut log);
    }

    // 6. Back to the states read first.
    let pot = start_paused(&exe, &files.join("m10-bands.wav"), &mut log)?;
    let mut states = read_menu(&pot, &out.join("menu-between.tsv"), &mut log)?;
    for (command, _) in EFFECTS {
        set_toggle(
            &pot,
            &mut states,
            command,
            before.is_checked(command),
            &mut log,
        );
    }
    set_toggle(
        &pot,
        &mut states,
        CMD_NORMALIZER,
        before.is_checked(CMD_NORMALIZER),
        &mut log,
    );
    if let Some(preset) = before.first_checked(FIRST_PRESET..=FIRST_PRESET + PRESETS - 1) {
        post_command(&pot, preset, &mut log);
    }
    set_toggle(
        &pot,
        &mut states,
        CMD_EQUALIZER,
        before.is_checked(CMD_EQUALIZER),
        &mut log,
    );
    if let Some(volume) = volume_before {
        let mut current = 100;
        let read = set_pot_volume(pot.hwnd, volume, &mut current, true);
        log.line(&format!("volume_restored={read:?}"));
    }
    let after = read_menu(&pot, &out.join("menu-after.tsv"), &mut log)?;
    let differ: Vec<usize> = before
        .checked
        .iter()
        .filter(|id| !after.is_checked(**id))
        .chain(after.checked.iter().filter(|id| !before.is_checked(**id)))
        .copied()
        .collect();
    log.line(&format!(
        "states_after={:?} differ={differ:?}",
        after.checked
    ));
    close_potplayer(pot, &mut log);
    sleep_ms(1500);

    // 7. RVP's second device.
    if let (Some(rvp), Some(second)) = (rvp, second) {
        let result = second_device(&rvp, &second, &files.join("m10-clicks.wav"), &out, &mut log);
        if let Err(error) = &result {
            log.line(&format!("second_device_error={error}"));
        }
    }
    result
}

/// Closes every PotPlayer window, as its close button would.
fn close_all(log: &mut Log) {
    for pid in processes_named(POT_EXE) {
        for hwnd in top_windows(None, Some(pid)) {
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while !processes_named(POT_EXE).is_empty() && Instant::now() < deadline {
        sleep_ms(200);
    }
    log.line(&format!("potplayer_left={:?}", processes_named(POT_EXE)));
}

/// Starts PotPlayer on `file` and pauses it at the start.
fn start_paused(exe: &Path, file: &Path, log: &mut Log) -> Result<Pot, String> {
    let pot = start_potplayer(exe, &[&file.to_string_lossy()], log)?;
    send_message(pot.hwnd, WM_USER, POT_SET_PLAY_STATUS, PLAY_PAUSED);
    send_message(pot.hwnd, WM_USER, POT_SET_CURRENT_TIME, 0);
    sleep_ms(500);
    let status = send_message(pot.hwnd, WM_USER, POT_GET_PLAY_STATUS, 0);
    let time = send_message(pot.hwnd, WM_USER, POT_GET_CURRENT_TIME, 0);
    log.line(&format!(
        "paused file={} status={status} time_ms={time}",
        file.display()
    ));
    Ok(pot)
}

fn post_command(pot: &Pot, command: usize, log: &mut Log) {
    unsafe { PostMessageW(pot.hwnd, WM_COMMAND, command, 0) };
    log.line(&format!("command={command}"));
    sleep_ms(250);
}

/// Sends a toggle's command when its state differs from `on`.
fn set_toggle(pot: &Pot, states: &mut MenuStates, command: usize, on: bool, log: &mut Log) {
    if states.is_checked(command) != on {
        post_command(pot, command, log);
        if on {
            states.checked.push(command);
        } else {
            states.checked.retain(|id| *id != command);
        }
    }
}

/// Plays the file from its start; before pass `index` its command goes
/// (none for 0), and a command in `toggles` goes again when its pass ends.
fn play_passes(
    pot: &Pot,
    pass_ms: i64,
    commands: &[(usize, String)],
    toggles: &[usize],
    clock: &Clock,
    log: &mut Log,
    section: &str,
) -> Result<(), String> {
    let send_pass = |index: usize, log: &mut Log| {
        let (command, name) = &commands[index];
        if *command != 0 {
            unsafe { PostMessageW(pot.hwnd, WM_COMMAND, *command, 0) };
        }
        log.line(&format!(
            "pass {section} {index} {name} command={command} t={}",
            clock.now()
        ));
    };
    send_pass(0, log);
    sleep_ms(300);
    send_message(pot.hwnd, WM_USER, POT_SET_CURRENT_TIME, 0);
    send_message(pot.hwnd, WM_USER, POT_SET_PLAY_STATUS, PLAY_RUNNING);
    log.line(&format!("section {section} start t={}", clock.now()));
    let total = pass_ms * commands.len() as i64;
    let started = Instant::now();
    let deadline = started + Duration::from_millis(total as u64 + 8000);
    let mut next = 1;
    let mut last_time = -1;
    // PotPlayer's position, or the wall clock when it never reports one.
    let mut by_clock = false;
    loop {
        sleep_ms(20);
        let reported = send_message(pot.hwnd, WM_USER, POT_GET_CURRENT_TIME, 0) as i64;
        if !by_clock && last_time <= 0 && reported <= 0 && started.elapsed() > Duration::from_secs(3) {
            by_clock = true;
            log.line(&format!("section {section} position_from=clock (PotPlayer reports {reported})"));
        }
        let time = if by_clock {
            started.elapsed().as_millis() as i64
        } else {
            reported
        };
        if time < last_time - 1000 {
            log.line(&format!("position_jumped from={last_time} to={time}"));
        }
        last_time = last_time.max(time);
        let boundary = pass_ms * next as i64;
        // By the clock playback lags by its start: switch later, still in
        // the silence between the passes.
        let switch = if by_clock { boundary + SWITCH_LATE } else { boundary - SWITCH_EARLY };
        if next < commands.len() && time >= switch {
            let previous = commands[next - 1].0;
            if toggles.contains(&previous) {
                unsafe { PostMessageW(pot.hwnd, WM_COMMAND, previous, 0) };
                log.line(&format!(
                    "pass {section} {} off command={previous} t={}",
                    next - 1,
                    clock.now()
                ));
            }
            send_pass(next, log);
            next += 1;
        }
        if time >= total - 100 || Instant::now() > deadline {
            break;
        }
    }
    let last = commands[commands.len() - 1].0;
    if toggles.contains(&last) {
        unsafe { PostMessageW(pot.hwnd, WM_COMMAND, last, 0) };
        log.line(&format!(
            "pass {section} {} off command={last} t={}",
            commands.len() - 1,
            clock.now()
        ));
    }
    send_message(pot.hwnd, WM_USER, POT_SET_PLAY_STATUS, PLAY_PAUSED);
    log.line(&format!(
        "section {section} end t={} position_ms={last_time}",
        clock.now()
    ));
    if Instant::now() > deadline {
        return Err(format!(
            "{section}: PotPlayer's position did not reach the end ({last_time} ms)"
        ));
    }
    Ok(())
}

/// Opens PotPlayer's context menu with a window message, dumps it with its
/// checked states and closes it.
fn read_menu(pot: &Pot, path: &Path, log: &mut Log) -> Result<MenuStates, String> {
    let rect = window_rect(pot.hwnd);
    let (x, y) = ((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2);
    let point = ((y as u32 & 0xFFFF) << 16) | (x as u32 & 0xFFFF);
    unsafe { PostMessageW(pot.hwnd, WM_CONTEXTMENU, pot.hwnd as usize, point as isize) };
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut menus = open_menus();
    while menus.is_empty() && Instant::now() < deadline {
        sleep_ms(100);
        menus = open_menus();
    }
    if menus.is_empty() {
        // PotPlayer ignored the message: a right click on its window (the
        // pointer goes back where it was).
        let before = cursor();
        move_to(x, y);
        sleep_ms(200);
        send(&[mouse_flags(MOUSEEVENTF_RIGHTDOWN), mouse_flags(MOUSEEVENTF_RIGHTUP)]);
        let deadline = Instant::now() + Duration::from_secs(4);
        while menus.is_empty() && Instant::now() < deadline {
            sleep_ms(100);
            menus = open_menus();
        }
        move_to(before.0, before.1);
        log.line(&format!("menu_by_right_click={}", !menus.is_empty()));
    }
    let result = match menus.first() {
        Some(&menu_window) => {
            let menu = unsafe { SendMessageW(menu_window, MN_GETHMENU, 0, 0) } as HMENU;
            let mut text = String::from("depth\tcommand_id\ttext\tshortcut\tflags\n");
            let mut count = 0;
            dump_menu(menu, 0, &mut text, &mut count);
            std::fs::write(path, &text).map_err(|e| e.to_string())?;
            log.line(&format!("menu={} items={count}", path.display()));
            Ok(MenuStates::parse(&text))
        }
        None => Err("PotPlayer's context menu did not open".to_owned()),
    };
    unsafe { PostMessageW(pot.hwnd, WM_CANCELMODE, 0, 0) };
    sleep_ms(400);
    for _ in 0..4 {
        for menu_window in open_menus() {
            unsafe {
                PostMessageW(menu_window, WM_KEYDOWN, 0x1B, 0);
                PostMessageW(menu_window, WM_KEYUP, 0x1B, 0);
            }
        }
        sleep_ms(250);
    }
    log.line(&format!("menus_left_open={}", open_menus().len()));
    result
}

/// PotPlayer's OSD after Increase Audio Stretching, and after it is undone.
fn stretch_osd(pot: &Pot, out: &Path, log: &mut Log) {
    let rect = window_rect(pot.hwnd);
    unsafe { PostMessageW(pot.hwnd, WM_COMMAND, STRETCH_UP, 0) };
    sleep_ms(350);
    capture_to(log, &rect, &out.join("stretch-up.png"));
    unsafe { PostMessageW(pot.hwnd, WM_COMMAND, STRETCH_DOWN, 0) };
    sleep_ms(350);
    capture_to(log, &rect, &out.join("stretch-down.png"));
}

/// True when a visible control of the dialog shows exactly `mark`.
fn page_has(dialog: HWND, mark: &str) -> bool {
    visible_children(dialog)
        .into_iter()
        .any(|hwnd| control_text(hwnd) == mark)
}

/// The text of the static control left of `rect`, or above it.
fn label_for(statics: &[(HWND, RECT)], rect: &RECT) -> String {
    let left = statics
        .iter()
        .filter(|(_, r)| r.right <= rect.left + 2 && r.top < rect.bottom && r.bottom > rect.top)
        .min_by_key(|(_, r)| rect.left - r.right);
    let above = statics
        .iter()
        .filter(|(_, r)| {
            r.bottom <= rect.top + 2
                && rect.top - r.bottom < 24
                && r.left < rect.right
                && r.right > rect.left
        })
        .min_by_key(|(_, r)| rect.top - r.bottom);
    left.or(above)
        .map(|(hwnd, _)| control_text(*hwnd))
        .unwrap_or_default()
}

/// Every visible control of the dialog with its ID, kind and value.
fn read_controls(dialog: HWND, page: &str) -> String {
    let controls = visible_children(dialog);
    let statics: Vec<(HWND, RECT)> = controls
        .iter()
        .filter(|&&hwnd| class_name(hwnd) == "Static")
        .map(|&hwnd| (hwnd, window_rect(hwnd)))
        .collect();
    let mut out = String::new();
    for &hwnd in &controls {
        let class = class_name(hwnd);
        let id = unsafe { GetDlgCtrlID(hwnd) };
        let rect = window_rect(hwnd);
        let row = match class.as_str() {
            "Static" => {
                let text = control_text(hwnd);
                if text.is_empty() {
                    continue;
                }
                format!("static\t{text}\t\t")
            }
            "Button" => {
                let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } & 0xF;
                let text = control_text(hwnd);
                if matches!(style, 2 | 3 | 5 | 6) {
                    format!("check\t{text}\t{}\t", send_message(hwnd, BM_GETCHECK, 0, 0))
                } else if matches!(style, 4 | 9) {
                    format!("radio\t{text}\t{}\t", send_message(hwnd, BM_GETCHECK, 0, 0))
                } else if style == 7 {
                    format!("group\t{text}\t\t")
                } else {
                    format!("button\t{text}\t\t")
                }
            }
            "ComboBox" => {
                let count = send_message(hwnd, CB_GETCOUNT, 0, 0).max(0) as usize;
                let selected = send_message(hwnd, CB_GETCURSEL, 0, 0);
                let mut entries = Vec::new();
                for index in 0..count {
                    let length = send_message(hwnd, CB_GETLBTEXTLEN, index, 0).max(0) as usize;
                    let mut buffer = vec![0u16; length + 1];
                    send_message(hwnd, CB_GETLBTEXT, index, buffer.as_mut_ptr() as isize);
                    entries.push(super::wide_to_string(&buffer));
                }
                format!(
                    "combo\t{}\t{selected}\t{}",
                    label_for(&statics, &rect),
                    entries.join(" | ")
                )
            }
            "msctls_trackbar32" => format!(
                "slider\t{}\t{}\t{}..{}",
                label_for(&statics, &rect),
                send_message(hwnd, TBM_GETPOS, 0, 0),
                send_message(hwnd, TBM_GETRANGEMIN, 0, 0),
                send_message(hwnd, TBM_GETRANGEMAX, 0, 0)
            ),
            "Edit" => format!(
                "edit\t{}\t{}\t",
                label_for(&statics, &rect),
                control_text(hwnd)
            ),
            _ => continue,
        };
        let _ = writeln!(out, "{page}\t{id}\t{row}");
    }
    out
}

/// Opens Preferences, reads each audio page and closes the dialog without
/// applying anything, the page tree put back as it was.
fn read_audio_pages(pot: &Pot, out: &Path, log: &mut Log) -> Result<(), String> {
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
    log.line(&format!(
        "preferences=\"{}\" {}",
        window_text(dialog),
        rect_text(&window_rect(dialog))
    ));
    sleep_ms(1200);
    let result = (|| {
        let tree = visible_children(dialog)
            .into_iter()
            .find(|&hwnd| class_name(hwnd) == "SysTreeView32")
            .ok_or("no page tree in the dialog")?;
        let original = send_message(tree, TVM_GETNEXTITEM, TVGN_CARET, 0);
        let mut table = String::from("page\tcontrol\tkind\ttext\tvalue\textra\n");
        let mut found = Vec::new();
        for &root in &tree_items(tree, None) {
            for item in std::iter::once(root).chain(tree_items(tree, Some(root))) {
                send_message(tree, TVM_SELECTITEM, TVGN_CARET, item);
                sleep_ms(350);
                for (page, mark) in AUDIO_PAGES {
                    if !found.contains(&page) && page_has(dialog, mark) {
                        found.push(page);
                        table.push_str(&read_controls(dialog, page));
                        capture_to(
                            log,
                            &window_rect(dialog),
                            &out.join(format!("preferences-{page}.png")),
                        );
                    }
                }
            }
            if found.len() == AUDIO_PAGES.len() {
                break;
            }
        }
        std::fs::write(out.join("preferences-audio.tsv"), &table).map_err(|e| e.to_string())?;
        log.line(&format!(
            "audio_pages={found:?} rows={}",
            table.lines().count() - 1
        ));
        send_message(tree, TVM_SELECTITEM, TVGN_CARET, original);
        sleep_ms(300);
        Ok::<(), String>(())
    })();
    unsafe { PostMessageW(dialog, WM_CLOSE, 0, 0) };
    sleep_ms(800);
    log.line(&format!(
        "preferences_closed={}",
        !top_windows(None, Some(pid)).contains(&dialog)
    ));
    result
}

/// A loopback recording of one device with each buffer's capture time.
struct TimedRecorder {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    times: Arc<Mutex<Vec<(u64, u128)>>>,
    times_path: PathBuf,
}

fn record_device(device: &cpal::Device, path: &Path) -> Result<TimedRecorder, String> {
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    if config.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("{:?} is not f32", config.sample_format()));
    }
    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let writer =
        hound::WavWriter::create(path, spec).map_err(|e| format!("{}: {e}", path.display()))?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let times = Arc::new(Mutex::new(Vec::new()));
    let (sink, stamps) = (Arc::clone(&writer), Arc::clone(&times));
    let channels = u64::from(config.channels());
    let mut frames = 0u64;
    let stream = device
        .build_input_stream::<f32, _, _>(
            config.config(),
            move |data: &[f32], info: &cpal::InputCallbackInfo| {
                let nanos = info
                    .timestamp()
                    .capture
                    .duration_since(cpal::StreamInstant::ZERO)
                    .as_nanos();
                if let Ok(mut list) = stamps.lock() {
                    list.push((frames, nanos));
                }
                frames += data.len() as u64 / channels;
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
    Ok(TimedRecorder {
        stream,
        writer,
        times,
        times_path: path.with_extension("times.tsv"),
    })
}

fn stop_device(recorder: TimedRecorder) -> Result<(), String> {
    drop(recorder.stream);
    if let Some(writer) = recorder.writer.lock().map_err(|e| e.to_string())?.take() {
        writer.finalize().map_err(|e| e.to_string())?;
    }
    let times = recorder.times.lock().map_err(|e| e.to_string())?;
    let mut text = String::from("frame\tcapture_ns\n");
    for (frame, nanos) in times.iter() {
        let _ = writeln!(text, "{frame}\t{nanos}");
    }
    std::fs::write(&recorder.times_path, text).map_err(|e| e.to_string())
}

/// RVP plays the clicks to the default device and to its second device;
/// both are recorded with capture times.
fn second_device(
    rvp: &Path,
    second: &str,
    clicks: &Path,
    out: &Path,
    log: &mut Log,
) -> Result<(), String> {
    let host = cpal::default_host();
    let default = host
        .default_output_device()
        .ok_or("no default output device")?;
    let default_id = default
        .id()
        .map(|id| id.id().to_owned())
        .unwrap_or_default();
    let mut chosen = None;
    for device in host.output_devices().map_err(|e| e.to_string())? {
        let name = device
            .description()
            .map(|d| d.to_string())
            .unwrap_or_default();
        let id = device.id().map(|id| id.id().to_owned()).unwrap_or_default();
        log.line(&format!("output_device \"{name}\" id={id}"));
        if chosen.is_none()
            && id != default_id
            && name.to_lowercase().contains(&second.to_lowercase())
        {
            chosen = Some((device, name, id));
        }
    }
    let (device, name, endpoint) =
        chosen.ok_or(format!("no other output device named like \"{second}\""))?;
    log.line(&format!("second_device=\"{name}\" endpoint={endpoint}"));
    let home = out.join("rvp-second");
    let (config, state) = (home.join("config"), home.join("state"));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&config).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    let settings = format!(
        "{{ \"ipc-server\": \"\", \"normalizer\": {{ \"enabled\": false }}, \"audio\": {{ \"second-device\": \"wasapi/{endpoint}\" }} }}\n"
    );
    std::fs::write(config.join("settings.json"), settings).map_err(|e| e.to_string())?;
    std::fs::write(
        state.join("state.json"),
        "{ \"version\": 1, \"volume\": 100 }\n",
    )
    .map_err(|e| e.to_string())?;
    let first = record_device(&default, &out.join("second-default.wav"))?;
    let other = record_device(&device, &out.join("second-other.wav"))?;
    sleep_ms(500);
    let mut child = Command::new(rvp)
        .args(["--instance=m10-second", "--skin="])
        .arg(clicks)
        .env("RVP_CONFIG_DIR", &config)
        .env("RVP_STATE_DIR", &state)
        .spawn()
        .map_err(|e| format!("{}: {e}", rvp.display()))?;
    log.line("rvp_second start");
    sleep_ms(36_000);
    for hwnd in top_windows(None, Some(child.id())) {
        if class_name(hwnd).contains("QWindow") {
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut closed = false;
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            log.line(&format!("rvp_second closed status={status}"));
            closed = true;
            break;
        }
        sleep_ms(200);
    }
    if !closed {
        let _ = child.kill();
        log.line("rvp_second close=TIMEOUT (stopped)");
    }
    sleep_ms(500);
    stop_device(first)?;
    stop_device(other)?;
    log.line(&format!(
        "recorded={} and second-other.wav",
        out.join("second-default.wav").display()
    ));
    Ok(())
}
