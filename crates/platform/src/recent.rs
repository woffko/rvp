//! The desktop's recent files (M14's "Show recent files in the taskbar",
//! Windows' jump list): freedesktop.org's desktop bookmarks,
//! `$XDG_DATA_HOME/recently-used.xbel`, which GTK's file dialogs and
//! GNOME's Files and Activities read. RVP adds or renews the entry of a
//! file it played, with its MIME type and RVP as the program, and writes
//! the file whole into a temporary file renamed over it, readable by its
//! owner only (0600, as GTK keeps it). Other programs' entries are kept
//! as they are. A file's entry is found by its decoded `href`, so
//! `GLib`'s `&apos;` and a plain `'` are one entry (`GLib` refuses a list
//! with two entries of one file), and RVP writes the attributes escaped
//! as `GLib` does. A list that is a symbolic link (one to `/dev/null`
//! turns the list off) or not a regular file is left alone.

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
    if std::fs::symlink_metadata(list).is_ok_and(|meta| !meta.file_type().is_file()) {
        return Ok(());
    }
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
    write_private(&temporary, updated.as_bytes()).map_err(error)?;
    std::fs::rename(&temporary, list).map_err(|e| {
        drop(std::fs::remove_file(&temporary));
        error(e)
    })
}

/// Writes `bytes` into a new file at `path` that only its owner may read.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    // One left by an earlier process of the same number.
    drop(std::fs::remove_file(path));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)
}

/// `text` with `file`'s entry new at its end (older entries of the same
/// file taken out).
fn with_entry(text: &str, file: &Path, stamp: &str) -> String {
    let uri = file_uri(file);
    let mut text = text.to_owned();
    while let Some(start) = find_entry(&text, &uri)
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

/// Where the bookmark of `uri` starts in `text`: the first `<bookmark`
/// whose decoded `href` is `uri`.
fn find_entry(text: &str, uri: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = text[from..].find("<bookmark") {
        let start = from + at;
        let rest = &text[start + "<bookmark".len()..];
        if rest.starts_with(char::is_whitespace) && href(rest).is_some_and(|href| href == uri) {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

/// The decoded `href` among the attributes that start `tag`.
fn href(tag: &str) -> Option<String> {
    let mut rest = tag;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('>') || rest.starts_with('/') {
            return None;
        }
        let (name, after) = rest.split_once('=')?;
        let after = after.trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let value_and_rest = &after[1..];
        let end = value_and_rest.find(quote)?;
        if name.trim() == "href" {
            return Some(unescape(&value_and_rest[..end]));
        }
        rest = &value_and_rest[end + 1..];
    }
}

/// XML's five named entities and its character references decoded;
/// anything else is kept as it is.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest.find(';').and_then(|end| {
            let name = &rest[1..end];
            let character = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => name
                    .strip_prefix("#x")
                    .map(|hex| u32::from_str_radix(hex, 16))
                    .or_else(|| name.strip_prefix('#').map(str::parse))
                    .and_then(Result::ok)
                    .and_then(char::from_u32),
            };
            character.map(|character| (character, end + 1))
        });
        if let Some((character, length)) = decoded {
            out.push(character);
            rest = &rest[length..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
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

/// Text in an attribute, escaped as `GLib`'s `g_markup_escape_text` does.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

    /// A list of `entries` in a folder of its own, named after `test`.
    fn list_in(test: &str, entries: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("rvp-recent-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let list = dir.join("recently-used.xbel");
        if !entries.is_empty() {
            std::fs::write(
                &list,
                EMPTY.replace("</xbel>", &format!("{entries}</xbel>")),
            )
            .expect("list");
        }
        (dir, list)
    }

    #[test]
    fn an_apostrophe_makes_no_second_entry() {
        // GLib's entry for the file, as GNOME's Files leaves it, and one an
        // older RVP wrote with a plain apostrophe.
        let glib = "  <bookmark href=\"file:///f/it&apos;s%20a%20&amp;%20b%20(1).mkv\" added=\"2026-11-01T09:30:00.123456Z\" modified=\"2026-11-01T09:30:00.123456Z\" visited=\"2026-11-01T09:30:00.123456Z\">\n    <info>\n      <metadata owner=\"http://freedesktop.org\">\n        <mime:mime-type type=\"video/x-matroska\"/>\n        <bookmark:applications>\n          <bookmark:application name=\"Files\" exec=\"&apos;org.gnome.Nautilus %u&apos;\" modified=\"2026-11-01T09:30:00.123456Z\" count=\"1\"/>\n        </bookmark:applications>\n      </metadata>\n    </info>\n  </bookmark>\n";
        let older = "  <bookmark href=\"file:///f/it's%20a%20&amp;%20b%20(1).mkv\" added=\"x\"/>\n";
        let (dir, list) = list_in("apostrophe", &format!("{glib}{older}"));
        let film = Path::new("/f/it's a & b (1).mkv");
        add(&list, film, UNIX_EPOCH).expect("renewed");
        let text = std::fs::read_to_string(&list).expect("read");
        assert_eq!(text.matches("<bookmark ").count(), 1, "{text}");
        assert!(
            text.contains("<bookmark href=\"file:///f/it&apos;s%20a%20&amp;%20b%20(1).mkv\" added=\"1970-01-01T00:00:00.000000Z\""),
            "{text}"
        );
        assert!(!text.contains("2026-11-01"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hrefs_are_decoded() {
        assert_eq!(
            href(" added=\"a\" href='file:///a&amp;b&apos;&#x41;&#66;&bad;' />").as_deref(),
            Some("file:///a&b'AB&bad;")
        );
        assert_eq!(href(" added=\"a\">"), None);
        assert!(find_entry("<bookmarks href=\"file:///a\">", "file:///a").is_none());
    }

    #[test]
    fn the_list_stays_its_owners_alone() {
        use std::os::unix::fs::PermissionsExt as _;
        let mode =
            |list: &Path| std::fs::metadata(list).expect("list").permissions().mode() & 0o777;
        let (dir, list) = list_in("mode", "");
        add(&list, Path::new("/a.mkv"), UNIX_EPOCH).expect("made");
        assert_eq!(mode(&list), 0o600);
        std::fs::set_permissions(&list, std::fs::Permissions::from_mode(0o644)).expect("mode");
        add(&list, Path::new("/b.mkv"), UNIX_EPOCH).expect("added");
        assert_eq!(mode(&list), 0o600);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_list_that_is_a_link_is_left_alone() {
        let (dir, list) = list_in("link", "");
        // The desktop's recent files turned off.
        std::os::unix::fs::symlink("/dev/null", &list).expect("link");
        add(&list, Path::new("/a.mkv"), UNIX_EPOCH).expect("left alone");
        assert_eq!(
            std::fs::read_link(&list).expect("a link"),
            Path::new("/dev/null")
        );
        // A link to a list elsewhere stays a link, and that list as it is.
        let elsewhere = dir.join("elsewhere.xbel");
        std::fs::write(&elsewhere, EMPTY).expect("list");
        std::fs::remove_file(&list).expect("removed");
        std::os::unix::fs::symlink(&elsewhere, &list).expect("link");
        add(&list, Path::new("/a.mkv"), UNIX_EPOCH).expect("left alone");
        assert!(
            std::fs::symlink_metadata(&list)
                .expect("link")
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&elsewhere).expect("read"), EMPTY);
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
