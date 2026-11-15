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
    // One update at a time: each started file's thread reads, changes and
    // renames the whole list, and two at once lost one's entry or renamed
    // the same temporary file (the Linux audit's L11).
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let error = |e: std::io::Error| format!("recent files: {}: {e}", list.display());
    let text = match std::fs::read_to_string(list) {
        Ok(text) if text.contains("</xbel>") => text,
        Ok(text) if text.trim().is_empty() => EMPTY.to_owned(),
        // Other programs' entries are not RVP's to drop.
        Ok(_) => {
            return Err(format!(
                "recent files: {}: not a bookmark list, left as it is",
                list.display()
            ));
        }
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
        && let Some(length) = entry_length(&text[start..])
    {
        let mut end = start + length;
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

/// The length of the bookmark at the start of `text`: up to its
/// `</bookmark>`, or its opening tag alone when that ends in `/>` (the
/// next one's end would take the bookmarks between).
fn entry_length(text: &str) -> Option<usize> {
    let tag = text.find('>')? + 1;
    if text[..tag].ends_with("/>") {
        return Some(tag);
    }
    text.find("</bookmark>")
        .map(|length| length + "</bookmark>".len())
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

/// A `file://` URI escaped as `GLib`'s `g_filename_to_uri` does (RFC 3986's
/// characters of a path kept), so GTK's entry and RVP's for one file are
/// the same entry (RVP's `%28` beside GTK's `(` made two).
fn file_uri(file: &Path) -> String {
    use std::os::unix::ffi::OsStrExt as _;
    let mut uri = String::from("file://");
    for &byte in file.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"!$&'()*+,-./:=@_~".contains(&byte) {
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
        let href = "href=\"file:///home/u/%D0%92%D0%B8%D0%B4%D0%B5%D0%BE/a%20&amp;%20b.mkv\"";
        assert_eq!(text.matches(href).count(), 1, "{text}");
        assert!(text.contains("visited=\"1970-01-01T00:00:20.000000Z\""));
        assert!(!text.contains("00:00:10"));
        assert!(text.contains("<mime:mime-type type=\"video/x-matroska\"/>"));
        assert!(text.trim_end().ends_with("</xbel>"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn uris_are_glibs_and_other_entries_stay() {
        assert_eq!(
            file_uri(Path::new("/f/Film (2020) [cut]; #1 100%.mkv")),
            "file:///f/Film%20(2020)%20%5Bcut%5D%3B%20%231%20100%25.mkv"
        );
        let dir = std::env::temp_dir().join(format!("rvp-recent-kept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let list = dir.join("recently-used.xbel");
        // A self-closing entry of the film, then another program's.
        let text = EMPTY.replace(
            "</xbel>",
            "  <bookmark href=\"file:///f/a.mkv\" added=\"x\"/>\n  <bookmark href=\"file:///f/notes.txt\">\n    <info/>\n  </bookmark>\n</xbel>",
        );
        std::fs::write(&list, &text).expect("list");
        add(&list, Path::new("/f/a.mkv"), UNIX_EPOCH).expect("renewed");
        let renewed = std::fs::read_to_string(&list).expect("read");
        assert!(renewed.contains("file:///f/notes.txt"), "{renewed}");
        assert_eq!(renewed.matches("file:///f/a.mkv").count(), 1, "{renewed}");
        // A list RVP cannot read is left as it is.
        std::fs::write(&list, "<xbel version=\"1.0\">truncated").expect("broken");
        assert!(add(&list, Path::new("/f/b.mkv"), UNIX_EPOCH).is_err());
        assert_eq!(
            std::fs::read_to_string(&list).expect("read"),
            "<xbel version=\"1.0\">truncated"
        );
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
