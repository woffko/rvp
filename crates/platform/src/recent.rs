//! The desktop's recent files (M14's "Show recent files in the taskbar",
//! Windows' jump list): freedesktop.org's desktop bookmarks,
//! `$XDG_DATA_HOME/recently-used.xbel`, which GTK's file dialogs and
//! GNOME's Files and Activities read. RVP adds or renews the entry of a
//! file it played, with its MIME type and RVP as the program, and writes
//! the file whole into a temporary file renamed over it. Other programs'
//! entries are kept as they are.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const EMPTY: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xbel version=\"1.0\"\n      xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\"\n      xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\"\n>\n</xbel>\n";

/// The user's `recently-used.xbel`.
///
/// # Errors
/// Fails when neither `XDG_DATA_HOME` nor `HOME` names a folder.
pub fn user_file() -> Result<PathBuf, String> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|data| data.join("recently-used.xbel"))
        .ok_or_else(|| "recent files: no home folder".to_owned())
}

/// Adds `file` (an absolute path) to the bookmarks in `list`, or renews
/// its entry.
///
/// # Errors
/// Fails when the list cannot be read or written.
pub fn add(list: &Path, file: &Path, now: SystemTime) -> Result<(), String> {
    let error = |e: std::io::Error| format!("recent files: {}: {e}", list.display());
    let text = match std::fs::read_to_string(list) {
        Ok(text) if text.contains("</xbel>") => text,
        Ok(_) => EMPTY.to_owned(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => EMPTY.to_owned(),
        Err(e) => return Err(error(e)),
    };
    let updated = with_entry(&text, file, &stamp(now));
    if let Some(folder) = list.parent() {
        std::fs::create_dir_all(folder).map_err(error)?;
    }
    let temporary = list.with_extension(format!("xbel.rvp-{}", std::process::id()));
    std::fs::write(&temporary, updated).map_err(error)?;
    std::fs::rename(&temporary, list).map_err(|e| {
        drop(std::fs::remove_file(&temporary));
        error(e)
    })
}

/// `text` with `file`'s entry new at its end (an older entry of the same
/// file taken out).
fn with_entry(text: &str, file: &Path, stamp: &str) -> String {
    let uri = file_uri(file);
    let opening = format!("<bookmark href=\"{}\"", escape(&uri));
    let mut text = text.to_owned();
    if let Some(start) = text.find(&opening)
        && let Some(length) = text[start..].find("</bookmark>")
    {
        let mut end = start + length + "</bookmark>".len();
        // The line's indent before and its end after go too.
        let line_start = text[..start].rfind('\n').map_or(start, |n| n + 1);
        if text[line_start..start].trim().is_empty() {
            if text[end..].starts_with('\n') {
                end += 1;
            }
            text.replace_range(line_start..end, "");
        } else {
            text.replace_range(start..end, "");
        }
    }
    let mime = mime_type(file);
    let mut entry = String::new();
    let _ = write!(
        entry,
        "  <bookmark href=\"{}\" added=\"{stamp}\" modified=\"{stamp}\" visited=\"{stamp}\">\n    <info>\n      <metadata owner=\"http://freedesktop.org\">\n        <mime:mime-type type=\"{mime}\"/>\n        <bookmark:applications>\n          <bookmark:application name=\"RVP\" exec=\"&apos;rvp %u&apos;\" modified=\"{stamp}\" count=\"1\"/>\n        </bookmark:applications>\n      </metadata>\n    </info>\n  </bookmark>\n",
        escape(&uri)
    );
    let end = text.rfind("</xbel>").unwrap_or(text.len());
    text.insert_str(end, &entry);
    text
}

/// The file's MIME type by its extension (RVP's registered types), else
/// `application/octet-stream`.
fn mime_type(file: &Path) -> &'static str {
    let extension = file
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    rvp_app::associations::associations()
        .into_iter()
        .find(|association| association.extension == extension)
        .and_then(|association| association.mime_types.first().copied())
        .unwrap_or("application/octet-stream")
}

/// A `file://` URI, its path escaped as RFC 3986 asks.
fn file_uri(file: &Path) -> String {
    use std::os::unix::ffi::OsStrExt as _;
    let mut uri = String::from("file://");
    for &byte in file.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-_.~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The time as the bookmarks write it: UTC, `YYYY-MM-DDThh:mm:ss.ffffffZ`.
fn stamp(now: SystemTime) -> String {
    let since = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = since.as_secs();
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Howard Hinnant's civil_from_days.
    let z = i64::try_from(days).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:06}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60,
        since.subsec_micros()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn times_are_utc() {
        let time = UNIX_EPOCH + Duration::from_micros(1_793_525_400_123_456);
        assert_eq!(stamp(time), "2026-11-01T09:30:00.123456Z");
        let leap = UNIX_EPOCH + Duration::from_secs(1_709_251_199);
        assert_eq!(stamp(leap), "2024-02-29T23:59:59.000000Z");
        assert_eq!(stamp(UNIX_EPOCH), "1970-01-01T00:00:00.000000Z");
    }

    #[test]
    fn an_entry_is_added_and_renewed() {
        let dir = std::env::temp_dir().join(format!("rvp-recent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let list = dir.join("recently-used.xbel");
        let other = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xbel version=\"1.0\"\n      xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\"\n      xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\"\n>\n  <bookmark href=\"file:///home/u/notes.txt\" added=\"2026-01-01T00:00:00Z\" modified=\"2026-01-01T00:00:00Z\" visited=\"2026-01-01T00:00:00Z\">\n    <info/>\n  </bookmark>\n</xbel>\n";
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(&list, other).expect("list");
        let film = Path::new("/home/u/Видео/a & b.mkv");
        add(&list, film, UNIX_EPOCH + Duration::from_secs(10)).expect("added");
        add(&list, film, UNIX_EPOCH + Duration::from_secs(20)).expect("renewed");
        let text = std::fs::read_to_string(&list).expect("read");
        assert!(text.contains("href=\"file:///home/u/notes.txt\""));
        let href = "href=\"file:///home/u/%D0%92%D0%B8%D0%B4%D0%B5%D0%BE/a%20%26%20b.mkv\"";
        assert_eq!(text.matches(href).count(), 1, "{text}");
        assert!(text.contains("visited=\"1970-01-01T00:00:20.000000Z\""));
        assert!(!text.contains("00:00:10"));
        assert!(text.contains("<mime:mime-type type=\"video/x-matroska\"/>"));
        assert!(text.trim_end().ends_with("</xbel>"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_list_is_made() {
        let dir = std::env::temp_dir().join(format!("rvp-recent-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let list = dir.join("recently-used.xbel");
        add(&list, Path::new("/a.mp3"), UNIX_EPOCH).expect("added");
        let text = std::fs::read_to_string(&list).expect("read");
        assert!(text.starts_with("<?xml"));
        assert!(text.contains("href=\"file:///a.mp3\""));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
