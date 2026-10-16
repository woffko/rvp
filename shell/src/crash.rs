//! Native crash logs (M8), beside the panic logs of `rvp_app::crash`: a
//! fatal signal on Unix, an unhandled exception on Windows (with a
//! minidump) write a line to `crash-<start>-native.log` in RVP's state
//! folder, named after the time RVP started. Everything the handlers need
//! is prepared at start; the handlers themselves only open, write and
//! close files (FFI points, AGENTS.md).

use std::path::Path;
use std::sync::OnceLock;
use std::time::SystemTime;

use rvp_app::crash::version_text;

/// The first words of the line a crash writes, before the signal or the
/// exception code.
static HEADER: OnceLock<Vec<u8>> = OnceLock::new();

/// Writes `value` in `base` (10 or 16) into `buffer`; the digits used.
fn digits(mut value: u64, base: u64, buffer: &mut [u8; 20]) -> &[u8] {
    let mut start = buffer.len();
    loop {
        start -= 1;
        let digit = u8::try_from(value % base).unwrap_or(0);
        buffer[start] = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        };
        value /= base;
        if value == 0 || start == 0 {
            break;
        }
    }
    &buffer[start..]
}

#[cfg(unix)]
mod imp {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::Path;
    use std::sync::OnceLock;
    use std::time::SystemTime;

    use super::{HEADER, digits};

    static LOG: OnceLock<CString> = OnceLock::new();

    const SIGNALS: [libc::c_int; 5] = [
        libc::SIGSEGV,
        libc::SIGBUS,
        libc::SIGILL,
        libc::SIGFPE,
        libc::SIGABRT,
    ];

    pub(super) fn install(dir: &Path, start: SystemTime) {
        let path = rvp_app::crash::report_path(dir, start, "-native.log");
        let Ok(path) = CString::new(path.into_os_string().into_vec()) else {
            return;
        };
        let _ = LOG.set(path);
        for signal in SIGNALS {
            // SAFETY: FFI point. A zeroed `sigaction` is a valid empty one;
            // the handler only makes async-signal-safe calls, and
            // SA_RESETHAND gives the signal its default action again, so
            // the handler's re-raise ends the process as before.
            #[allow(unsafe_code)]
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = handler as *const () as usize;
                action.sa_flags = libc::SA_RESETHAND | libc::SA_ONSTACK;
                libc::sigemptyset(&raw mut action.sa_mask);
                libc::sigaction(signal, &raw const action, std::ptr::null_mut());
            }
        }
    }

    extern "C" fn handler(signal: libc::c_int) {
        let mut buffer = [0u8; 20];
        let number = digits(u64::try_from(signal).unwrap_or(0), 10, &mut buffer);
        if let (Some(path), Some(header)) = (LOG.get(), HEADER.get()) {
            // SAFETY: FFI point. open, write and close are
            // async-signal-safe; the path and the header were prepared
            // before any handler could run, and every buffer outlives its
            // call.
            #[allow(unsafe_code)]
            unsafe {
                let file = libc::open(
                    path.as_ptr(),
                    libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND | libc::O_CLOEXEC,
                    0o644,
                );
                if file >= 0 {
                    let text = b"fatal signal ";
                    libc::write(file, header.as_ptr().cast(), header.len());
                    libc::write(file, text.as_ptr().cast(), text.len());
                    libc::write(file, number.as_ptr().cast(), number.len());
                    libc::write(file, b"\n".as_ptr().cast(), 1);
                    libc::close(file);
                }
            }
        }
        // SAFETY: FFI point: raise is async-signal-safe; the default action
        // (restored by SA_RESETHAND) ends the process.
        #[allow(unsafe_code)]
        unsafe {
            libc::raise(signal);
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::OnceLock;
    use std::time::SystemTime;

    use windows::Win32::Foundation::{CloseHandle, GENERIC_WRITE};
    use windows::Win32::Storage::FileSystem::{
        CREATE_ALWAYS, CreateFileW, FILE_APPEND_DATA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ,
        OPEN_ALWAYS, WriteFile,
    };
    use windows::Win32::System::Diagnostics::Debug::{
        EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION, MiniDumpNormal, MiniDumpWithThreadInfo,
        MiniDumpWriteDump, SetUnhandledExceptionFilter,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
    };
    use windows::core::PCWSTR;

    use super::{HEADER, digits};

    static LOG: OnceLock<Vec<u16>> = OnceLock::new();
    static DUMP: OnceLock<Vec<u16>> = OnceLock::new();

    /// Ends the process after the filter, without Windows' crash dialog.
    const EXCEPTION_EXECUTE_HANDLER: i32 = 1;

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    pub(super) fn install(dir: &Path, start: SystemTime) {
        let _ = LOG.set(wide(&rvp_app::crash::report_path(
            dir,
            start,
            "-native.log",
        )));
        let _ = DUMP.set(wide(&rvp_app::crash::report_path(
            dir,
            start,
            "-native.dmp",
        )));
        // SAFETY: FFI point: registers a function of the required
        // signature as the process's last-chance exception filter.
        #[allow(unsafe_code)]
        unsafe {
            SetUnhandledExceptionFilter(Some(filter));
        }
    }

    /// Writes the crash line and a minidump, then ends the process.
    #[allow(unsafe_code)]
    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        let (Some(log), Some(dump), Some(header)) = (LOG.get(), DUMP.get(), HEADER.get()) else {
            return EXCEPTION_EXECUTE_HANDLER;
        };
        // SAFETY: FFI point. Windows passes the filter valid exception
        // pointers for the duration of the call; the paths and the header
        // were prepared before any exception, and each handle is closed
        // once written.
        unsafe {
            let code = info
                .as_ref()
                .and_then(|info| info.ExceptionRecord.as_ref())
                .map_or(0, |record| record.ExceptionCode.0.cast_unsigned());
            let mut buffer = [0u8; 20];
            let code = digits(u64::from(code), 16, &mut buffer);
            if let Ok(file) = CreateFileW(
                PCWSTR(log.as_ptr()),
                FILE_APPEND_DATA.0,
                FILE_SHARE_READ,
                None,
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                for part in [header.as_slice(), b"unhandled exception 0x", code, b"\r\n"] {
                    let _ = WriteFile(file, Some(part), None, None);
                }
                let _ = CloseHandle(file);
            }
            if let Ok(file) = CreateFileW(
                PCWSTR(dump.as_ptr()),
                GENERIC_WRITE.0,
                FILE_SHARE_READ,
                None,
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                let exception = MINIDUMP_EXCEPTION_INFORMATION {
                    ThreadId: GetCurrentThreadId(),
                    ExceptionPointers: info.cast_mut(),
                    ClientPointers: false.into(),
                };
                let _ = MiniDumpWriteDump(
                    GetCurrentProcess(),
                    GetCurrentProcessId(),
                    file,
                    MiniDumpNormal | MiniDumpWithThreadInfo,
                    Some(&raw const exception),
                    None,
                    None,
                );
                let _ = CloseHandle(file);
            }
        }
        EXCEPTION_EXECUTE_HANDLER
    }
}

/// Installs the native crash handlers, writing to `dir` (created here).
pub(crate) fn install(dir: &Path) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let _ = HEADER.set(
        format!(
            "RVP {} on {} {}: ",
            version_text(),
            std::env::consts::OS,
            std::env::consts::ARCH
        )
        .into_bytes(),
    );
    // Named after the start: the handlers cannot format a time.
    #[cfg(any(unix, windows))]
    imp::install(dir, SystemTime::now());
}
