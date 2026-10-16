//! Crash logs (M8). A panic anywhere in RVP writes `crash-<time>.log` to
//! RVP's state folder (version, system, thread, message, place, backtrace)
//! before Rust's own report. The shell adds native crashes to the same
//! folder: a fatal signal on Linux, an unhandled exception with a minidump
//! on Windows (`shell/src/crash.rs`), in files named after the time RVP
//! started ([`report_path`]).

use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// RVP's version and the commit it was built from.
#[must_use]
pub fn version_text() -> String {
    format!("{} ({})", env!("CARGO_PKG_VERSION"), env!("RVP_COMMIT"))
}

/// Year, month, day, hour, minute and second (UTC) of a time.
fn utc(time: SystemTime) -> [u64; 6] {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Howard Hinnant's civil_from_days, for days since 1970 (unsigned:
    // the clock does not go back before it).
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    [year, month, day, rest / 3600, rest % 3600 / 60, rest % 60]
}

/// `crash-YYYYMMDD-HHMMSS<suffix>` in `dir`, the time in UTC.
#[must_use]
pub fn report_path(dir: &Path, time: SystemTime, suffix: &str) -> PathBuf {
    let [year, month, day, hour, minute, second] = utc(time);
    dir.join(format!(
        "crash-{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}{suffix}"
    ))
}

/// Writes a report of every panic to `dir` (created when needed), then
/// lets the hook that was there report it too.
pub fn install_panic_hook(dir: &Path) {
    let dir = dir.to_path_buf();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // A report that cannot be written must not hide the panic.
        drop(write_panic_report(&dir, info));
        previous(info);
    }));
}

fn write_panic_report(dir: &Path, info: &PanicHookInfo<'_>) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = report_path(dir, SystemTime::now(), ".log");
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(no message)".to_owned());
    let place = info.location().map_or_else(
        || "an unknown place".to_owned(),
        |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
    );
    let current = std::thread::current();
    let thread = current.name().unwrap_or("unnamed");
    let text = format!(
        "RVP {} on {} {}\npanic in thread '{thread}' at {place}:\n{message}\n\nbacktrace:\n{}\n",
        version_text(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        Backtrace::force_capture()
    );
    // Appended: two threads may panic within the same second.
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(text.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn report_names_carry_the_utc_time() {
        let dir = Path::new("/state");
        assert_eq!(
            report_path(dir, UNIX_EPOCH, ".log"),
            dir.join("crash-19700101-000000.log")
        );
        // 2026-10-16 12:34:56 UTC.
        let time = UNIX_EPOCH + Duration::from_secs(1_792_154_096);
        assert_eq!(
            report_path(dir, time, "-native.log"),
            dir.join("crash-20261016-123456-native.log")
        );
        // A leap day.
        let time = UNIX_EPOCH + Duration::from_hours(474_768);
        assert_eq!(
            report_path(dir, time, ".dmp"),
            dir.join("crash-20240229-000000.dmp")
        );
    }

    #[test]
    fn the_version_names_the_commit() {
        let text = version_text();
        assert!(text.starts_with(env!("CARGO_PKG_VERSION")), "{text}");
        assert!(text.ends_with(')') && text.contains(" ("), "{text}");
    }
}
