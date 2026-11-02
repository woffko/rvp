//! What RVP keeps about files besides their positions (M11), in
//! `marks.json` in the state folder:
//!
//! * bookmarks per file (PotPlayer's Chapter/Bookmark list): a time and a
//!   title each, sorted by time;
//! * A-B repeat lists per file (PotPlayer's A-B Repeat List): sections
//!   with a start, an end and a title, sorted by start; kept across runs
//!   only while "Remember A-B Repeat List" is on (the app clears them
//!   otherwise);
//! * favorites (files and folders, PotPlayer's Album/Favorites) and the
//!   recent items (files and URLs opened, newest first, at most
//!   [`RECENT_ITEMS`]).
//!
//! Files are keyed by mpv's `path`, as positions are
//! ([`crate::resume`]). The file is written by the storage writer after
//! every change.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage;

/// Recent items kept.
pub const RECENT_ITEMS: usize = 30;
/// Bookmarks or sections closer than this are the same place.
const SAME_PLACE: f64 = 0.05;

/// A bookmark: a place in a file and its title.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Bookmark {
    pub time: f64,
    pub title: String,
}

impl Default for Bookmark {
    fn default() -> Self {
        Self {
            time: 0.0,
            title: String::new(),
        }
    }
}

/// A section of an A-B repeat list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Section {
    pub start: f64,
    pub end: f64,
    pub title: String,
}

impl Default for Section {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 0.0,
            title: String::new(),
        }
    }
}

/// One file's marks.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FileMarks {
    pub bookmarks: Vec<Bookmark>,
    pub sections: Vec<Section>,
}

impl FileMarks {
    fn is_empty(&self) -> bool {
        self.bookmarks.is_empty() && self.sections.is_empty()
    }
}

/// A favorite file or folder.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Favorite {
    pub path: String,
    pub title: String,
    pub folder: bool,
}

/// Contents of `marks.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Marks {
    pub version: u32,
    pub files: BTreeMap<String, FileMarks>,
    pub favorites: Vec<Favorite>,
    /// Newest first.
    pub recent: Vec<String>,
}

impl Default for Marks {
    fn default() -> Self {
        Self {
            version: 1,
            files: BTreeMap::new(),
            favorites: Vec::new(),
            recent: Vec::new(),
        }
    }
}

/// The marks and their file.
#[derive(Debug)]
pub struct MarkStore {
    path: PathBuf,
    marks: Marks,
    writer: storage::Writer,
}

impl MarkStore {
    /// Opens `marks.json` in `dir`; a missing or damaged file starts empty.
    ///
    /// # Errors
    /// Fails on I/O errors other than a missing file, or when the writer
    /// thread cannot start.
    pub fn open(dir: &Path) -> io::Result<Self> {
        let path = dir.join("marks.json");
        let marks = storage::read_json(&path)?.unwrap_or_default();
        Ok(Self {
            path,
            marks,
            writer: storage::Writer::new()?,
        })
    }

    #[must_use]
    pub fn marks(&self) -> &Marks {
        &self.marks
    }

    /// A file's marks (empty when it has none).
    #[must_use]
    pub fn file(&self, path: &str) -> FileMarks {
        self.marks.files.get(path).cloned().unwrap_or_default()
    }

    fn save(&self) {
        match serde_json::to_vec_pretty(&self.marks) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                self.writer.write(&self.path, bytes);
            }
            Err(error) => crate::log::report(
                crate::log::Level::Critical,
                &format!("cannot serialize the marks: {error}"),
            ),
        }
    }

    /// Changes a file's marks; files left without marks are dropped.
    fn change_file(&mut self, path: &str, change: impl FnOnce(&mut FileMarks) -> bool) -> bool {
        let mut marks = self.file(path);
        if !change(&mut marks) {
            return false;
        }
        marks
            .bookmarks
            .sort_by(|a, b| a.time.total_cmp(&b.time).then(a.title.cmp(&b.title)));
        marks
            .sections
            .sort_by(|a, b| a.start.total_cmp(&b.start).then(a.end.total_cmp(&b.end)));
        if marks.is_empty() {
            self.marks.files.remove(path);
        } else {
            self.marks.files.insert(path.to_owned(), marks);
        }
        self.save();
        true
    }

    /// Adds a bookmark (a bookmark at the same place takes the new title);
    /// returns its index.
    pub fn add_bookmark(&mut self, path: &str, time: f64, title: &str) -> usize {
        let time = time.max(0.0);
        self.change_file(path, |marks| {
            marks
                .bookmarks
                .retain(|bookmark| (bookmark.time - time).abs() >= SAME_PLACE);
            marks.bookmarks.push(Bookmark {
                time,
                title: title.to_owned(),
            });
            true
        });
        self.file(path)
            .bookmarks
            .iter()
            .position(|bookmark| (bookmark.time - time).abs() < SAME_PLACE)
            .unwrap_or(0)
    }

    pub fn remove_bookmark(&mut self, path: &str, index: usize) -> bool {
        self.change_file(path, |marks| {
            (index < marks.bookmarks.len())
                .then(|| marks.bookmarks.remove(index))
                .is_some()
        })
    }

    pub fn rename_bookmark(&mut self, path: &str, index: usize, title: &str) -> bool {
        self.change_file(path, |marks| match marks.bookmarks.get_mut(index) {
            Some(bookmark) if bookmark.title != title => {
                title.clone_into(&mut bookmark.title);
                true
            }
            _ => false,
        })
    }

    pub fn clear_bookmarks(&mut self, path: &str) -> bool {
        self.change_file(path, |marks| {
            let had = !marks.bookmarks.is_empty();
            marks.bookmarks.clear();
            had
        })
    }

    /// Adds a section of the A-B repeat list (the same section is not
    /// added twice); returns its index.
    pub fn add_section(&mut self, path: &str, start: f64, end: f64, title: &str) -> usize {
        let (start, end) = (start.min(end).max(0.0), start.max(end));
        let same = |section: &Section| {
            (section.start - start).abs() < SAME_PLACE && (section.end - end).abs() < SAME_PLACE
        };
        self.change_file(path, |marks| {
            if marks.sections.iter().any(same) {
                return false;
            }
            marks.sections.push(Section {
                start,
                end,
                title: title.to_owned(),
            });
            true
        });
        self.file(path).sections.iter().position(same).unwrap_or(0)
    }

    pub fn remove_section(&mut self, path: &str, index: usize) -> bool {
        self.change_file(path, |marks| {
            (index < marks.sections.len())
                .then(|| marks.sections.remove(index))
                .is_some()
        })
    }

    pub fn rename_section(&mut self, path: &str, index: usize, title: &str) -> bool {
        self.change_file(path, |marks| match marks.sections.get_mut(index) {
            Some(section) if section.title != title => {
                title.clone_into(&mut section.title);
                true
            }
            _ => false,
        })
    }

    /// Drops every file's A-B repeat list ("Remember A-B Repeat List" off).
    pub fn forget_sections(&mut self) -> bool {
        let paths: Vec<String> = self
            .marks
            .files
            .iter()
            .filter(|(_, marks)| !marks.sections.is_empty())
            .map(|(path, _)| path.clone())
            .collect();
        let mut changed = false;
        for path in paths {
            changed |= self.change_file(&path, |marks| {
                marks.sections.clear();
                true
            });
        }
        changed
    }

    /// Adds a favorite (once); true when it was not there.
    pub fn add_favorite(&mut self, path: &str, folder: bool) -> bool {
        if self
            .marks
            .favorites
            .iter()
            .any(|favorite| favorite.path == path)
        {
            return false;
        }
        let title = crate::view::file_name(path.trim_end_matches(['/', '\\'])).to_owned();
        self.marks.favorites.push(Favorite {
            path: path.to_owned(),
            title,
            folder,
        });
        self.save();
        true
    }

    /// Replaces the favorites (the Edit Favorites dialog).
    pub fn set_favorites(&mut self, favorites: Vec<Favorite>) {
        if self.marks.favorites != favorites {
            self.marks.favorites = favorites;
            self.save();
        }
    }

    /// Records an opened file or URL as the newest recent item.
    pub fn add_recent(&mut self, path: &str) {
        if self.marks.recent.first().is_some_and(|first| first == path) {
            return;
        }
        self.marks.recent.retain(|item| item != path);
        self.marks.recent.insert(0, path.to_owned());
        self.marks.recent.truncate(RECENT_ITEMS);
        self.save();
    }

    pub fn clear_recent(&mut self) -> bool {
        if self.marks.recent.is_empty() {
            return false;
        }
        self.marks.recent.clear();
        self.save();
        true
    }

    /// Waits until the file is written (shutdown, tests).
    ///
    /// # Errors
    /// Fails when the last write failed.
    pub fn flush(&self) -> io::Result<()> {
        self.writer.flush();
        match self.writer.take_error() {
            Some(error) => Err(io::Error::other(error)),
            None => Ok(()),
        }
    }

    /// A write error from the background writer, once.
    #[must_use]
    pub fn take_error(&self) -> Option<String> {
        self.writer.take_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(name: &str) -> (MarkStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("rvp-marks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        (MarkStore::open(&dir).expect("open"), dir)
    }

    #[test]
    fn bookmarks_stay_sorted_and_survive_a_restart() {
        let (mut store, dir) = store("bookmarks");
        assert_eq!(store.add_bookmark("/a.mkv", 30.0, "Thirty"), 0);
        assert_eq!(store.add_bookmark("/a.mkv", 10.0, "Ten"), 0);
        assert_eq!(store.add_bookmark("/a.mkv", 20.0, "Twenty"), 1);
        // The same place takes the new title.
        assert_eq!(store.add_bookmark("/a.mkv", 20.01, "Again"), 1);
        assert!(store.rename_bookmark("/a.mkv", 2, "End"));
        assert!(store.remove_bookmark("/a.mkv", 0));
        store.flush().expect("written");
        let store = MarkStore::open(&dir).expect("reopen");
        let titles: Vec<_> = store
            .file("/a.mkv")
            .bookmarks
            .iter()
            .map(|bookmark| bookmark.title.clone())
            .collect();
        assert_eq!(titles, ["Again", "End"]);
        assert!(store.file("/b.mkv").bookmarks.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sections_are_ordered_and_can_be_forgotten() {
        let (mut store, dir) = store("sections");
        store.add_section("/a.mkv", 50.0, 40.0, "Second");
        assert_eq!(store.add_section("/a.mkv", 10.0, 20.0, "First"), 0);
        // Added twice: still one.
        store.add_section("/a.mkv", 10.0, 20.0, "First");
        let sections = store.file("/a.mkv").sections;
        assert_eq!(sections.len(), 2);
        assert!((sections[1].start - 40.0).abs() < 1e-9 && (sections[1].end - 50.0).abs() < 1e-9);
        store.add_bookmark("/a.mkv", 5.0, "Kept");
        assert!(store.forget_sections());
        let marks = store.file("/a.mkv");
        assert!(marks.sections.is_empty());
        assert_eq!(marks.bookmarks.len(), 1);
        assert!(store.remove_bookmark("/a.mkv", 0));
        assert!(
            !store.marks().files.contains_key("/a.mkv"),
            "empty files go"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn favorites_and_recent_items() {
        let (mut store, dir) = store("lists");
        assert!(store.add_favorite("/films/a.mkv", false));
        assert!(!store.add_favorite("/films/a.mkv", false));
        assert!(store.add_favorite("/films/", true));
        assert_eq!(store.marks().favorites[1].title, "films");
        for n in 0..40 {
            store.add_recent(&format!("/r/{n}.mkv"));
        }
        store.add_recent("/r/5.mkv");
        let recent = &store.marks().recent;
        assert_eq!(recent.len(), RECENT_ITEMS);
        assert_eq!(recent[0], "/r/5.mkv");
        assert_eq!(recent[1], "/r/39.mkv");
        assert_eq!(recent.iter().filter(|item| *item == "/r/5.mkv").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
