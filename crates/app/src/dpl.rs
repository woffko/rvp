//! Import of PotPlayer playlists (`.dpl`, one file per album) with a
//! mapping from Windows drive letters to Linux paths.
//!
//! The format, as PotPlayer 260819 writes it (UTF-8 with a byte order mark,
//! CRLF lines):
//!
//! ```text
//! DAUMPLAYLIST
//! playname=<the file played last>
//! playtime=<its position in milliseconds>
//! topindex=<the first row shown>
//! saveplaypos=0
//! 1*file*<path or URL>
//! 1*title*<title shown instead of the file name>
//! 1*duration2*<milliseconds>
//! 1*played*1
//! 1*invalid*1          (could not be opened)
//! 1*start*<milliseconds> (where playback resumes)
//! ```
//!
//! `PotPlayerMini64.dpl` is the "Default" album; the others are named after
//! their files and follow it in alphabetical order, as PotPlayer shows them.
//! Folder albums (`basefolder=`) have no entries and import empty. Nothing
//! is ever written back.
//!
//! Paths follow the system (owner decision 2026-10-11): on Windows they stay
//! as PotPlayer wrote them; on Linux every Windows path becomes a Linux path,
//! so that no `C:\...` appears there. A drive in `drive-map` becomes its
//! folder (`V:\Films\a.mkv` → `/mnt/media/Films/a.mkv`), any other drive
//! `/mnt/<letter>` (`C:\x.mkv` → `/mnt/c/x.mkv`), and a network path its
//! mount point (`\\nas\Media\a.mkv` → `/mnt/nas/Media/a.mkv`).
//! URLs stay as they are.
//!
//! A playlist is text from outside: an entry is a file or a network
//! address ([`crate::playlist::admitted`]); what else a file names is
//! left out and counted.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crate::playlist::{Album, Entry, Refused, admitted};

/// How Windows paths become Linux paths (see the module comment). The
/// default maps nothing: Windows keeps its paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DriveMap {
    drives: BTreeMap<char, String>,
    /// Linux: every Windows path becomes a Linux path.
    linux: bool,
}

impl DriveMap {
    /// A mapping from settings: keys like `"V:"` or `"V"`, values Linux
    /// folders. Invalid keys are ignored.
    #[must_use]
    pub fn new(map: &BTreeMap<String, String>) -> Self {
        let drives = map
            .iter()
            .filter_map(|(drive, folder)| {
                let letter = drive.trim().trim_end_matches([':', '\\', '/']);
                let mut chars = letter.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c.is_ascii_alphabetic() => Some((
                        c.to_ascii_uppercase(),
                        folder.trim_end_matches('/').to_owned(),
                    )),
                    _ => None,
                }
            })
            .collect();
        Self {
            drives,
            linux: true,
        }
    }

    /// The path RVP opens for a path in a PotPlayer playlist.
    #[must_use]
    pub fn map(&self, path: &str) -> String {
        if !self.linux {
            return path.to_owned();
        }
        let mut chars = path.chars();
        if let (Some(letter), Some(':'), Some('\\' | '/')) =
            (chars.next(), chars.next(), chars.next())
            && letter.is_ascii_alphabetic()
        {
            let rest = path[3..].replace('\\', "/");
            return match self.drives.get(&letter.to_ascii_uppercase()) {
                Some(folder) => format!("{folder}/{rest}"),
                None => format!("/mnt/{}/{rest}", letter.to_ascii_lowercase()),
            };
        }
        // A network path: `\\server\share\...` is mounted at
        // `/mnt/server/share`.
        if let Some(unc) = path.strip_prefix("\\\\") {
            return format!("/mnt/{}", unc.replace('\\', "/"));
        }
        path.to_owned()
    }
}

/// One imported album and what it remembered about playback.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub album: Album,
    /// Resume positions in seconds, by mapped path.
    pub positions: Vec<(String, f64)>,
    /// The entries left out: neither files nor network addresses.
    pub refused: Refused,
}

#[derive(Default)]
struct Raw {
    file: Option<String>,
    title: Option<String>,
    duration_ms: Option<u64>,
    played: bool,
    invalid: bool,
    start_ms: Option<u64>,
}

/// Parses one `.dpl` file's text into an album called `name`.
#[must_use]
pub fn parse(name: &str, text: &str, map: &DriveMap) -> Imported {
    let mut raws: BTreeMap<u32, Raw> = BTreeMap::new();
    let mut play_name = None;
    let mut play_time_ms = None;
    let mut top = 0;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim_end_matches('\r');
        let mut parts = line.splitn(3, '*');
        if let (Some(number), Some(key), Some(value)) = (parts.next(), parts.next(), parts.next())
            && let Ok(number) = number.parse::<u32>()
        {
            let raw = raws.entry(number).or_default();
            match key {
                "file" => raw.file = Some(value.to_owned()),
                "title" => raw.title = Some(value.to_owned()).filter(|t| !t.is_empty()),
                "duration2" => raw.duration_ms = value.parse().ok(),
                "played" => raw.played = value == "1",
                "invalid" => raw.invalid = value == "1",
                "start" => raw.start_ms = value.parse().ok(),
                _ => {}
            }
        } else if let Some((key, value)) = line.split_once('=') {
            match key {
                "playname" => play_name = Some(value.to_owned()),
                "playtime" => play_time_ms = value.parse::<u64>().ok(),
                "topindex" => top = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    let mut album = Album::new(name);
    let mut positions = Vec::new();
    let mut refused = Refused::default();
    for raw in raws.into_values() {
        let Some(file) = raw.file else {
            continue;
        };
        let Some(path) = admitted(&map.map(&file)) else {
            refused.add(&file);
            continue;
        };
        // Milliseconds as seconds; durations are far below 2^52 ms.
        #[allow(clippy::cast_precision_loss)]
        let seconds = |ms: u64| ms as f64 / 1000.0;
        if Some(&file) == play_name.as_ref() {
            album.current = Some(album.entries.len());
        }
        let resume = raw.start_ms.or_else(|| {
            (Some(&file) == play_name.as_ref())
                .then_some(play_time_ms)
                .flatten()
        });
        if let Some(ms) = resume.filter(|&ms| ms > 0) {
            positions.push((path.clone(), seconds(ms)));
        }
        album.entries.push(Entry {
            path,
            title: raw.title,
            duration: raw.duration_ms.map(seconds),
            played: raw.played,
            playable: !raw.invalid,
        });
    }
    album.top = top;
    Imported {
        album,
        positions,
        refused,
    }
}

/// Reads every `.dpl` in `dir` (read-only): "Default" first, then by name.
///
/// # Errors
/// Fails when the folder cannot be listed; unreadable files are skipped.
pub fn import_dir(dir: &Path, map: &DriveMap) -> io::Result<Vec<Imported>> {
    let mut files: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dpl"))
        })
        .filter_map(|path| {
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            let name = if stem.eq_ignore_ascii_case("PotPlayerMini64")
                || stem.eq_ignore_ascii_case("PotPlayer")
            {
                "Default".to_owned()
            } else {
                stem
            };
            Some((name, path))
        })
        .collect();
    files.sort_by(|a, b| {
        (a.0 != "Default", a.0.to_lowercase()).cmp(&(b.0 != "Default", b.0.to_lowercase()))
    });
    let mut albums = Vec::new();
    for (name, path) in files {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        albums.push(parse(&name, &String::from_utf8_lossy(&bytes), map));
    }
    Ok(albums)
}

/// PotPlayer's playlist folder on Windows (`%APPDATA%\PotPlayerMini64\Playlist`).
#[must_use]
pub fn potplayer_folder() -> Option<std::path::PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let folder = std::path::PathBuf::from(std::env::var_os("APPDATA")?)
        .join("PotPlayerMini64")
        .join("Playlist");
    folder.is_dir().then_some(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> DriveMap {
        DriveMap::new(&BTreeMap::from([
            ("V:".to_owned(), "/mnt/media".to_owned()),
            ("a".to_owned(), "/mnt/archive/".to_owned()),
            ("bad key".to_owned(), "/x".to_owned()),
        ]))
    }

    #[test]
    fn drive_letters_map_to_folders() {
        let map = map();
        assert_eq!(map.map(r"V:\Films\A b.mkv"), "/mnt/media/Films/A b.mkv");
        assert_eq!(map.map(r"v:\x.mkv"), "/mnt/media/x.mkv");
        assert_eq!(
            map.map(r"A:\Series\s01\e01.mkv"),
            "/mnt/archive/Series/s01/e01.mkv"
        );
        // Unmapped drives and network paths still become Linux paths.
        assert_eq!(map.map(r"C:\Users\x.mkv"), "/mnt/c/Users/x.mkv");
        assert_eq!(
            map.map(r"\\nas\Media\Films\a.mkv"),
            "/mnt/nas/Media/Films/a.mkv"
        );
        assert_eq!(
            map.map("https://example.org/v.mkv"),
            "https://example.org/v.mkv"
        );
        assert_eq!(map.map("/already/linux.mkv"), "/already/linux.mkv");
        assert_eq!(
            DriveMap::default().map(r"V:\Films\a.mkv"),
            r"V:\Films\a.mkv"
        );
    }

    #[test]
    fn parses_entries_marks_and_positions() {
        // Synthetic, in PotPlayer's layout: BOM, CRLF, gaps in the numbers.
        let text = "\u{feff}DAUMPLAYLIST\r\nplayname=V:\\F\\two.mkv\r\nplaytime=125500\r\n\
                    topindex=1\r\nsaveplaypos=0\r\n\
                    1*file*V:\\F\\one.mkv\r\n1*duration2*5400000\r\n1*played*1\r\n\
                    2*file*V:\\F\\two.mkv\r\n2*duration2*7729000\r\n\
                    3*file*V:\\F\\gone.avi\r\n3*invalid*1\r\n\
                    5*file*https://example.org/live.ts\r\n5*title*Live\r\n5*start*61000\r\n";
        let imported = parse("Films", text, &map());
        let album = &imported.album;
        assert_eq!(album.name, "Films");
        assert_eq!(album.entries.len(), 4);
        assert_eq!(album.entries[0].path, "/mnt/media/F/one.mkv");
        assert_eq!(album.entries[0].duration, Some(5400.0));
        assert!(album.entries[0].played);
        assert!(!album.entries[2].playable);
        assert_eq!(album.entries[3].title.as_deref(), Some("Live"));
        assert_eq!(album.current, Some(1));
        assert_eq!(album.top, 1);
        assert_eq!(
            imported.positions,
            vec![
                ("/mnt/media/F/two.mkv".to_owned(), 125.5),
                ("https://example.org/live.ts".to_owned(), 61.0)
            ]
        );
    }

    #[test]
    fn only_files_and_network_addresses_are_imported() {
        let text = "\u{feff}DAUMPLAYLIST\r\nplayname=V:\\F\\two.mkv\r\nplaytime=5000\r\n\
                    1*file*av://dshow:video=Camera\r\n1*title*Camera\r\n1*start*9000\r\n\
                    2*file*V:\\F\\one.mkv\r\n\
                    3*file*DVD://1/D:\r\n\
                    4*file*V:\\F\\two.mkv\r\n\
                    5*file*fd://0\r\n\
                    6*file*mms://example.org/radio\r\n";
        let imported = parse("Mixed", text, &map());
        let paths: Vec<&str> = imported
            .album
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "/mnt/media/F/one.mkv",
                "/mnt/media/F/two.mkv",
                "mms://example.org/radio"
            ]
        );
        // The entry that played is found among those that stayed, and a
        // left-out entry leaves no position behind.
        assert_eq!(imported.album.current, Some(1));
        assert_eq!(
            imported.positions,
            vec![("/mnt/media/F/two.mkv".to_owned(), 5.0)]
        );
        assert_eq!(imported.refused.count(), 3);
        assert_eq!(
            imported.refused.notice().as_deref(),
            Some(
                "Not opened: 3 addresses (av://, dvd://, fd://). \
                 Only files and network addresses are taken this way."
            )
        );
    }

    #[test]
    fn imports_a_folder_with_default_first() {
        let dir = std::env::temp_dir().join(format!("rvp-dpl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let write = |name: &str, body: &str| {
            std::fs::write(dir.join(name), format!("\u{feff}DAUMPLAYLIST\r\n{body}"))
                .expect("write");
        };
        write("series.dpl", "1*file*V:\\s.mkv\r\n");
        write("PotPlayerMini64.dpl", "1*file*V:\\d.mkv\r\n");
        write("Every.dpl", "basefolder=\\\r\nworkfolder=\\\r\n");
        std::fs::write(dir.join("notes.txt"), "not a playlist").expect("write");
        let albums = import_dir(&dir, &map()).expect("import");
        let names: Vec<&str> = albums.iter().map(|a| a.album.name.as_str()).collect();
        assert_eq!(names, ["Default", "Every", "series"]);
        assert!(albums[1].album.entries.is_empty());
        assert_eq!(albums[2].album.entries[0].path, "/mnt/media/s.mkv");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
