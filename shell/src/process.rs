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

use std::sync::atomic::{AtomicBool, Ordering};

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
    #[cfg(windows)]
    if loaded & crate::crash::RIFE != 0 {
        use std::io::Write as _;
        use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        drop(std::io::stdout().flush());
        drop(std::io::stderr().flush());
        let exit = u32::try_from(code).unwrap_or(1);
        // SAFETY: FFI point. GetCurrentProcess returns a pseudo handle that
        // needs no closing; TerminateProcess ends this process and does
        // not return when it succeeds. Everything RVP keeps was saved
        // before (`Player::shutdown`, the writer's flush); should it fail,
        // the normal exit below follows.
        #[allow(unsafe_code)]
        unsafe {
            drop(TerminateProcess(GetCurrentProcess(), exit));
        }
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
