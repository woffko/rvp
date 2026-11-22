//! How the process loads libraries and ends.
//!
//! * Libraries come from RVP's folder, System32 and RVP's VapourSynth
//!   runtime only, never from the PATH or the current folder: SVP's
//!   installer puts its mpv folder on the PATH, and libmpv's probe for
//!   ANGLE loaded SVP's `libEGL.dll` into RVP (goal T's study).
//! * After RIFE ran (M18), the RIFE plugin's static destructors (ncnn) call
//!   NVIDIA's Vulkan driver while Windows unloads the process's libraries
//!   and jump to address 0: the plugin never gets ncnn's
//!   `destroy_gpu_instance()`. Once RVP has saved everything, a process
//!   that loaded the plugin ends without unloading its libraries.
//! * On Linux the process ends with `exit()`, which runs the exit handlers
//!   of the libraries still loaded, ncnn's among them after RIFE ran; no
//!   crash there has been seen. A crash while leaving says so in the crash
//!   log, with the plugins loaded (`crate::crash`), and the Linux kit's
//!   `m18exit` closes RVP after RIFE ran to find out.
//! * Once RVP has saved its state as its window closes, the process ends
//!   within [`END_WAIT`] or is ended ([`watch_the_end`]): on 2026-11-22 the
//!   window's thread and mpv's core went on turning after a close, the
//!   process stayed without a window, two processors busy, until it was
//!   ended by hand. The log names the step it was in ([`ending`]).

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use std::time::Duration;

/// How long the process may take to end once RVP saved its state as its
/// window closed: mpv's wait for idle (3 s at most), the window's and
/// mpv's ends (2 s at most for mpv's quit) and the libraries' take a few
/// seconds at most.
pub(crate) const END_WAIT: Duration = Duration::from_secs(15);

/// The steps of the end [`watch_the_end`] can name, in their order.
pub(crate) const ENDING_STEPS: [&str; 6] = [
    "the window closes",
    "mpv quits",
    "the video window closes",
    "the QML engine goes",
    "the application goes",
    "the libraries end",
];

/// The step of the end the process is in (an index of [`ENDING_STEPS`]).
static ENDING: AtomicU8 = AtomicU8::new(0);
/// The code the process ends with (the event loop's, once it returned).
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);
/// `--crash-test=hang` (a test hook): the window's thread stops after the
/// event loop, so that the watch must end the process.
pub(crate) static HANG_AT_EXIT: AtomicBool = AtomicBool::new(false);

/// The end reached `step` of [`ENDING_STEPS`]; `code` is the process's
/// exit code from then on.
pub(crate) fn ending(step: usize, code: Option<i32>) {
    ENDING.store(u8::try_from(step).unwrap_or(u8::MAX), Ordering::SeqCst);
    if let Some(code) = code {
        EXIT_CODE.store(code, Ordering::SeqCst);
    }
}

/// Ends the process [`END_WAIT`] from now unless it ended by then, and
/// says so in the log with the step it was in. Called once RVP saved its
/// state (`Player::shutdown`); once is enough.
pub(crate) fn watch_the_end() {
    static WATCHING: AtomicBool = AtomicBool::new(false);
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("rvp-end".to_owned())
        .spawn(|| {
            std::thread::sleep(END_WAIT);
            let step = ENDING_STEPS
                .get(usize::from(ENDING.load(Ordering::SeqCst)))
                .copied()
                .unwrap_or("its end");
            rvp_app::log::report(
                rvp_app::log::Level::Critical,
                &format!(
                    "RVP had not ended {} s after it saved its state as its window closed ({step}): it ends now",
                    END_WAIT.as_secs()
                ),
            );
            // The log's thread writes the line.
            std::thread::sleep(Duration::from_millis(500));
            terminate(EXIT_CODE.load(Ordering::SeqCst));
        });
    if let Err(error) = spawned {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("the watch over RVP's end did not start: {error}"),
        );
    }
}

/// Ends the process at once with `code`: no library's exit handlers or
/// destructors run, nothing more is saved.
fn terminate(code: i32) -> ! {
    use std::io::Write as _;
    drop(std::io::stdout().flush());
    drop(std::io::stderr().flush());
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        let exit = u32::try_from(code).unwrap_or(1);
        // SAFETY: FFI point. GetCurrentProcess returns a pseudo handle that
        // needs no closing; TerminateProcess ends this process and does
        // not return when it succeeds; should it fail, the abort below
        // ends it.
        #[allow(unsafe_code)]
        unsafe {
            drop(TerminateProcess(GetCurrentProcess(), exit));
        }
    }
    #[cfg(unix)]
    {
        // SAFETY: FFI point. _exit takes the code by value, ends the
        // process at once without running exit handlers, and does not
        // return.
        #[allow(unsafe_code)]
        unsafe {
            libc::_exit(code)
        }
    }
    #[allow(unreachable_code)]
    {
        std::process::abort()
    }
}

/// Restricts where libraries are looked for (see the module's
/// documentation); first thing in `main`, before any library loads.
pub(crate) fn restrict_library_search() {
    #[cfg(windows)]
    {
        use windows::Win32::System::LibraryLoader::{
            AddDllDirectory, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, SetDefaultDllDirectories,
        };
        // SAFETY: FFI point. SetDefaultDllDirectories takes flags by value
        // and only changes how this process looks for libraries from now
        // on; a failure leaves the system's search order.
        #[allow(unsafe_code)]
        let restricted = unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) };
        if restricted.is_err() {
            rvp_app::log::report(
                rvp_app::log::Level::Warning,
                "cannot restrict where libraries load from",
            );
            return;
        }
        let runtime = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("vapoursynth")))
            .filter(|dir| dir.is_dir());
        if let Some(runtime) = runtime {
            let wide: Vec<u16> = runtime
                .as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain([0])
                .collect();
            // SAFETY: FFI point. `wide` is a NUL-terminated UTF-16 path
            // that outlives the call; Windows copies it. The cookie it
            // returns stays valid for the process's life and is not kept.
            #[allow(unsafe_code)]
            let cookie = unsafe { AddDllDirectory(windows::core::PCWSTR(wide.as_ptr())) };
            if cookie.is_null() {
                rvp_app::log::report(
                    rvp_app::log::Level::Warning,
                    &format!("cannot add {} to the library search", runtime.display()),
                );
            }
        }
    }
}

/// `--crash-test=exit` (a test hook): crash natively in [`leave`], once the
/// crash log would say RVP was exiting.
pub(crate) static CRASH_AT_EXIT: AtomicBool = AtomicBool::new(false);

/// Ends the process with `code` (see the module's documentation).
pub(crate) fn leave(code: i32) -> ! {
    let loaded = crate::crash::leaving(crate::crash::EXITING);
    if CRASH_AT_EXIT.load(Ordering::Relaxed) {
        crate::ffi::bridge::rvp_crash_test();
    }
    // A clean exit: the skin marker goes (`rvp_app::skin_marker`).
    crate::player::marker_exit();
    ending(ENDING_STEPS.len() - 1, Some(code));
    // Everything RVP keeps was saved before (`Player::shutdown`, the
    // writer's flush).
    #[cfg(windows)]
    if loaded & crate::crash::RIFE != 0 {
        terminate(code);
    }
    #[cfg(not(windows))]
    let _ = loaded;
    std::process::exit(code)
}

/// The smoothing plugins whose libraries are loaded now, as bits of
/// `crate::crash::PLUGINS`' places.
pub(crate) fn loaded_plugins() -> u8 {
    let mut loaded = 0;
    #[cfg(windows)]
    for (bit, (name, _, _)) in crate::crash::PLUGINS.iter().enumerate() {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
        // SAFETY: FFI point. GetModuleHandleW only looks a NUL-terminated
        // name that outlives the call up among the loaded modules; the
        // handle is not kept or used.
        #[allow(unsafe_code)]
        let handle = unsafe { GetModuleHandleW(windows::core::PCWSTR(wide.as_ptr())) };
        if handle.is_ok() {
            loaded |= 1 << bit;
        }
    }
    #[cfg(target_os = "linux")]
    if let Ok(maps) = std::fs::read("/proc/self/maps") {
        for (bit, (_, name, _)) in crate::crash::PLUGINS.iter().enumerate() {
            let path = format!("/{name}");
            if maps.windows(path.len()).any(|part| part == path.as_bytes()) {
                loaded |= 1 << bit;
            }
        }
    }
    loaded
}
