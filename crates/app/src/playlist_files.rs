//! Playlist files in and out (M11, the playlist's Open a playlist and Save
//! playlist as): PotPlayer's `.dpl` ([`crate::dpl`] reads it), M3U and
//! M3U8 (`#EXTINF:<seconds>,<title>` before each path) and PLS
//! (`File1=`, `Title1=`, `Length1=`). Relative paths in a file are taken
//! from its folder; RVP writes absolute paths, UTF-8, with `.dpl` in
//! PotPlayer's form (a byte order mark and CRLF lines).
//!
//! A playlist file is text from outside: its entries are files and network
//! addresses ([`crate::playlist::admitted`]), and what else it names is
//! left out and counted ([`Refused`]).

use std::fmt::Write as _;
use std::path::Path;

use crate::playlist::{Entry, Refused, admitted, is_url};

/// The kinds of playlist files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Dpl,
    M3u,
    Pls,
}

impl Format {
    /// The format of a file name (M3U for `.m3u` and `.m3u8`).
    #[must_use]
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_string_lossy().to_lowercase();
        Some(match extension.as_str() {
            "dpl" => Format::Dpl,
            "m3u" | "m3u8" => Format::M3u,
            "pls" => Format::Pls,
            _ => return None,
        })
    }
}

/// A path from a playlist file: URLs and absolute paths as they are,
/// relative ones from the file's folder.
fn resolve(item: &str, folder: &Path) -> String {
    let item = item.trim();
    if is_url(item) || Path::new(item).is_absolute() {
        return item.to_owned();
    }
    let item = if cfg!(windows) {
        item.to_owned()
    } else {
        item.replace('\\', "/")
    };
    folder.join(item).to_string_lossy().into_owned()
}

/// Seconds as M3U's whole seconds (-1 when unknown).
fn whole_seconds(duration: Option<f64>) -> i64 {
    // Durations are far inside i64 seconds.
    #[allow(clippy::cast_possible_truncation)]
    duration.map_or(-1, |seconds| seconds.round() as i64)
}

/// Reads a playlist file's entries, and counts the lines that may not be
/// entries.
///
/// # Errors
/// Fails when the file cannot be read or is no playlist file RVP knows.
pub fn read(path: &Path, drives: &crate::dpl::DriveMap) -> Result<(Vec<Entry>, Refused), String> {
    let format =
        Format::of(path).ok_or_else(|| format!("{}: not a playlist file", path.display()))?;
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_start_matches('\u{feff}');
    let folder = path.parent().unwrap_or(Path::new("."));
    Ok(match format {
        Format::Dpl => {
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let imported = crate::dpl::parse(&name, text, drives);
            let entries = imported
                .album
                .entries
                .into_iter()
                .map(|entry| Entry {
                    path: resolve(&entry.path, folder),
                    ..entry
                })
                .collect();
            (entries, imported.refused)
        }
        Format::M3u => read_m3u(text, folder),
        Format::Pls => read_pls(text, folder),
    })
}

fn read_m3u(text: &str, folder: &Path) -> (Vec<Entry>, Refused) {
    let mut entries = Vec::new();
    let mut refused = Refused::default();
    let mut pending: Option<(Option<f64>, Option<String>)> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (length, title) = info.split_once(',').unwrap_or((info, ""));
            let length = length
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|seconds| *seconds > 0.0);
            let title = (!title.trim().is_empty()).then(|| title.trim().to_owned());
            pending = Some((length, title));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        // A line left out takes its `#EXTINF` with it.
        let (duration, title) = pending.take().unwrap_or((None, None));
        let Some(path) = admitted(&resolve(line, folder)) else {
            refused.add(line);
            continue;
        };
        let mut entry = Entry::new(&path);
        entry.duration = duration;
        entry.title = title;
        entries.push(entry);
    }
    (entries, refused)
}

fn read_pls(text: &str, folder: &Path) -> (Vec<Entry>, Refused) {
    let mut files = std::collections::BTreeMap::<u32, Entry>::new();
    let mut refused = Refused::default();
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();
        let split = key.find(|c: char| c.is_ascii_digit());
        let Some((field, number)) = split.map(|at| key.split_at(at)) else {
            continue;
        };
        let Ok(number) = number.parse::<u32>() else {
            continue;
        };
        let entry = files.entry(number).or_insert_with(|| Entry::new(""));
        match field {
            "file" => {
                if let Some(path) = admitted(&resolve(value, folder)) {
                    entry.path = crate::playlist::local_path(&path);
                } else {
                    entry.path.clear();
                    refused.add(value);
                }
            }
            "title" if !value.is_empty() => entry.title = Some(value.to_owned()),
            "length" => {
                entry.duration = value.parse::<f64>().ok().filter(|seconds| *seconds > 0.0);
            }
            _ => {}
        }
    }
    let entries = files
        .into_values()
        .filter(|entry| !entry.path.is_empty())
        .collect();
    (entries, refused)
}

/// The text of a playlist file for `entries`.
#[must_use]
pub fn write(format: Format, entries: &[Entry]) -> String {
    let mut out = String::new();
    match format {
        Format::Dpl => {
            out.push('\u{feff}');
            out.push_str("DAUMPLAYLIST\r\n");
            for (index, entry) in entries.iter().enumerate() {
                let number = index + 1;
                let _ = write!(out, "{number}*file*{}\r\n", entry.path);
                if let Some(title) = &entry.title {
                    let _ = write!(out, "{number}*title*{title}\r\n");
                }
                if let Some(duration) = entry.duration {
                    // Milliseconds of a duration are far inside i64.
                    #[allow(clippy::cast_possible_truncation)]
                    let millis = (duration * 1000.0).round() as i64;
                    let _ = write!(out, "{number}*duration2*{millis}\r\n");
                }
                if entry.played {
                    let _ = write!(out, "{number}*played*1\r\n");
                }
            }
        }
        Format::M3u => {
            out.push_str("#EXTM3U\n");
            for entry in entries {
                let _ = writeln!(
                    out,
                    "#EXTINF:{},{}",
                    whole_seconds(entry.duration),
                    entry.label()
                );
                let _ = writeln!(out, "{}", entry.path);
            }
        }
        Format::Pls => {
            out.push_str("[playlist]\n");
            for (index, entry) in entries.iter().enumerate() {
                let number = index + 1;
                let _ = writeln!(out, "File{number}={}", entry.path);
                let _ = writeln!(out, "Title{number}={}", entry.label());
                let _ = writeln!(out, "Length{number}={}", whole_seconds(entry.duration));
            }
            let _ = writeln!(out, "NumberOfEntries={}", entries.len());
            out.push_str("Version=2\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An absolute path of this system (`/films` has no drive on Windows).
    const FILM: &str = if cfg!(windows) {
        "C:\\films\\a.mkv"
    } else {
        "/films/a.mkv"
    };

    fn entries() -> Vec<Entry> {
        let mut first = Entry::new(FILM);
        first.title = Some("Film A".to_owned());
        first.duration = Some(90.4);
        let mut second = Entry::new("https://example.com/b.mp4");
        second.played = true;
        vec![first, second]
    }

    #[test]
    fn every_format_round_trips() {
        let dir = std::env::temp_dir().join(format!("rvp-lists-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        for (name, format) in [
            ("list.dpl", Format::Dpl),
            ("list.m3u8", Format::M3u),
            ("list.pls", Format::Pls),
        ] {
            let path = dir.join(name);
            std::fs::write(&path, write(format, &entries())).expect("write");
            let (back, refused) = read(&path, &crate::dpl::DriveMap::default()).expect("read");
            assert_eq!(refused.count(), 0, "{name}");
            assert_eq!(back.len(), 2, "{name}");
            assert_eq!(back[0].path, Entry::new(FILM).path, "{name}");
            assert_eq!(back[0].title.as_deref(), Some("Film A"), "{name}");
            let duration = back[0].duration.expect("duration");
            assert!((duration - 90.0).abs() <= 0.5, "{name}: {duration}");
            assert_eq!(back[1].path, "https://example.com/b.mp4", "{name}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_paths_come_from_the_folder() {
        let dir = std::env::temp_dir().join(format!("rvp-relative-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("list.m3u");
        std::fs::write(&path, "#EXTM3U\nsub/one.mkv\n# a comment\n\ntwo.mp3\n").expect("write");
        let (entries, _) = read(&path, &crate::dpl::DriveMap::default()).expect("read");
        let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
        let expected = |path: std::path::PathBuf| Entry::new(&path.to_string_lossy()).path;
        assert_eq!(
            paths,
            [
                expected(dir.join("sub").join("one.mkv")),
                expected(dir.join("two.mp3"))
            ]
        );
        assert!(read(&dir.join("list.txt"), &crate::dpl::DriveMap::default()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_files_and_network_addresses_become_entries() {
        let dir = std::env::temp_dir().join(format!("rvp-admitted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let film = Entry::new(FILM).path;
        let url = if cfg!(windows) {
            "file:///C:/films/a.mkv"
        } else {
            "file:///films/a.mkv"
        };
        // Each list names the film (as a path and as a file URL), a
        // stream, and three addresses no list may name; the titles of
        // those go with them.
        let lists = [
            (
                "list.m3u8",
                format!(
                    "#EXTM3U\n#EXTINF:5,Camera\nav://v4l2:/dev/video0\n{FILM}\n\
                     #EXTINF:7,Graph\nAV://lavfi:testsrc\n{url}\nfdclose://9\n\
                     #EXTINF:-1,Live\nrtsp://example.org/live\n"
                ),
            ),
            (
                "list.pls",
                format!(
                    "[playlist]\nFile1=av://v4l2:/dev/video0\nTitle1=Camera\nFile2={FILM}\n\
                     File3=AV://lavfi:testsrc\nTitle3=Graph\nFile4={url}\nFile5=fdclose://9\n\
                     File6=rtsp://example.org/live\nTitle6=Live\nNumberOfEntries=6\n"
                ),
            ),
            (
                "list.dpl",
                format!(
                    "\u{feff}DAUMPLAYLIST\r\n1*file*av://v4l2:/dev/video0\r\n1*title*Camera\r\n\
                     2*file*{FILM}\r\n3*file*AV://lavfi:testsrc\r\n3*title*Graph\r\n\
                     4*file*{url}\r\n5*file*fdclose://9\r\n\
                     6*file*rtsp://example.org/live\r\n6*title*Live\r\n"
                ),
            ),
        ];
        for (name, text) in lists {
            let path = dir.join(name);
            std::fs::write(&path, text).expect("write");
            let (entries, refused) = read(&path, &crate::dpl::DriveMap::default()).expect("read");
            let shown: Vec<(String, Option<&str>)> = entries
                .iter()
                .map(|entry| (Entry::new(&entry.path).path, entry.title.as_deref()))
                .collect();
            assert_eq!(
                shown,
                [
                    (film.clone(), None),
                    (film.clone(), None),
                    ("rtsp://example.org/live".to_owned(), Some("Live")),
                ],
                "{name}"
            );
            assert_eq!(refused.count(), 3, "{name}");
            assert_eq!(
                refused.notice().as_deref(),
                Some(
                    "Not opened: 3 addresses (av://, fdclose://). \
                     Only files and network addresses are taken this way."
                ),
                "{name}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
