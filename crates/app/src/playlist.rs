//! Albums (the playlist panel's tabs) and their entries (M3, D9).
//!
//! RVP's albums are the playlist: mpv plays one entry at a time, and
//! Next, Previous and the end of a file move through the album that plays.
//! Every edit can be undone. The albums live in `playlists.json` in the
//! state directory, written atomically by the storage writer after every
//! change, so a killed process keeps them.

use std::collections::BTreeSet;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage;

/// Undo steps kept.
pub const UNDO_DEPTH: usize = 50;

/// One file or URL in an album.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Entry {
    pub path: String,
    /// A title to show instead of the file name.
    pub title: Option<String>,
    /// Duration in seconds, once known.
    pub duration: Option<f64>,
    /// Opened at least once.
    pub played: bool,
    /// False once it failed to open (`<Can't open>`).
    pub playable: bool,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            path: String::new(),
            title: None,
            duration: None,
            played: false,
            playable: true,
        }
    }
}

/// True for `scheme://...` (a URL, not a file path).
#[must_use]
pub fn is_url(text: &str) -> bool {
    text.split_once("://")
        .is_some_and(|(scheme, _)| scheme.len() > 1 && scheme.chars().all(char::is_alphanumeric))
}

/// A local path as mpv's `path` property reports it, so that the entry
/// that plays can be recognised by it. On Windows mpv turns `/` into `\`
/// and drops `.` and `..` (`C:/a/../b.mkv` plays as `C:\b.mkv`); file
/// dialogs and drops give forward slashes. On Linux the kit's mpv master
/// drops `.` and `..` too (`/a/../b.mkv` plays as `/b.mkv`), while older
/// builds such as Ubuntu 24.04's 0.37 report the path as given; once RVP
/// drops them itself, both report the same. URLs stay as they are.
#[must_use]
pub fn local_path(path: &str) -> String {
    if is_url(path) {
        return path.to_owned();
    }
    let (unified, separator) = if cfg!(windows) {
        (path.replace('/', "\\"), "\\")
    } else {
        (path.to_owned(), "/")
    };
    let mut prefix = String::new();
    let mut parts: Vec<String> = Vec::new();
    for component in Path::new(&unified).components() {
        match component {
            Component::Prefix(drive) => prefix.push_str(&drive.as_os_str().to_string_lossy()),
            Component::RootDir => prefix.push_str(separator),
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() && prefix.is_empty() {
                    parts.push("..".to_owned());
                }
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
        }
    }
    format!("{prefix}{}", parts.join(separator))
}

impl Entry {
    /// An entry for a path or URL (local paths in mpv's form, see
    /// [`local_path`]).
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: local_path(path),
            ..Self::default()
        }
    }

    /// The text the list shows: the title or the file name.
    #[must_use]
    pub fn label(&self) -> &str {
        self.title
            .as_deref()
            .unwrap_or_else(|| crate::view::file_name(&self.path))
    }
}

/// One tab of the playlist panel.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Album {
    pub name: String,
    pub entries: Vec<Entry>,
    /// The entry played last.
    pub current: Option<usize>,
    pub selected: BTreeSet<usize>,
    /// Where a Shift selection starts.
    pub anchor: Option<usize>,
    /// The first row shown.
    pub top: usize,
}

impl Album {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Self::default()
        }
    }

    /// The selected entries, or the current one when nothing is selected.
    #[must_use]
    pub fn targets(&self) -> Vec<usize> {
        if self.selected.is_empty() {
            self.current.into_iter().collect()
        } else {
            self.selected.iter().copied().collect()
        }
    }
}

/// How a click selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    /// Only this entry.
    Only,
    /// Add or remove this entry (Ctrl).
    Toggle,
    /// From the anchor to this entry (Shift).
    Range,
}

/// Where selected entries move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
    Top,
    Bottom,
    /// Before this entry (a drop); `len` is after the last.
    Before(usize),
}

/// Sort orders (PotPlayer's playlist keys Ctrl+1, Ctrl+2, Ctrl+9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
    /// Shuffled with this seed.
    Random(u64),
}

/// Contents of `playlists.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Playlists {
    pub version: u32,
    pub albums: Vec<Album>,
    /// The album whose tab is shown.
    pub shown: usize,
    /// The album that plays: Next and Previous move in it.
    pub playing: Option<usize>,
}

impl Default for Playlists {
    fn default() -> Self {
        Self {
            version: 1,
            albums: vec![Album::new("Default")],
            shown: 0,
            playing: None,
        }
    }
}

/// Keeps `index` pointing at the same entry after `removed` (sorted) went.
fn shift_after_removal(index: usize, removed: &BTreeSet<usize>) -> Option<usize> {
    if removed.contains(&index) {
        None
    } else {
        Some(index - removed.range(..index).count())
    }
}

/// A small deterministic generator for shuffling (splitmix64).
fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The albums with undo, redo and their file.
#[derive(Debug)]
pub struct Library {
    lists: Playlists,
    undo: Vec<Playlists>,
    redo: Vec<Playlists>,
    path: PathBuf,
    writer: storage::Writer,
}

impl Library {
    /// Opens `playlists.json` in `dir`; a missing or damaged file starts
    /// with one empty "Default" album (a damaged file is kept as `.bad`).
    ///
    /// # Errors
    /// Fails on I/O errors other than a missing file, or when the writer
    /// thread cannot start.
    pub fn open(dir: &Path) -> io::Result<Self> {
        let path = dir.join("playlists.json");
        let mut lists: Playlists = storage::read_json(&path)?.unwrap_or_default();
        lists.repair();
        Ok(Self {
            lists,
            undo: Vec::new(),
            redo: Vec::new(),
            path,
            writer: storage::Writer::new()?,
        })
    }

    /// The file the albums live in.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn lists(&self) -> &Playlists {
        &self.lists
    }

    #[must_use]
    pub fn shown(&self) -> &Album {
        &self.lists.albums[self.lists.shown]
    }

    /// The album that plays and its current entry.
    #[must_use]
    pub fn playing(&self) -> Option<(usize, &Entry)> {
        let album = self.lists.albums.get(self.lists.playing?)?;
        let index = album.current?;
        album.entries.get(index).map(|entry| (index, entry))
    }

    /// Queues the file for writing.
    fn save(&self) {
        match serde_json::to_vec_pretty(&self.lists) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                self.writer.write(&self.path, bytes);
            }
            Err(error) => eprintln!("rvp: cannot serialize the playlists: {error}"),
        }
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

    /// Changes that undo can take back.
    fn edit(&mut self, change: impl FnOnce(&mut Playlists) -> bool) -> bool {
        let before = self.lists.clone();
        if change(&mut self.lists) {
            self.lists.repair();
            self.undo.push(before);
            if self.undo.len() > UNDO_DEPTH {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.save();
            true
        } else {
            false
        }
    }

    /// Changes that are not edits (selection, scrolling, playback marks).
    fn update(&mut self, change: impl FnOnce(&mut Playlists) -> bool) -> bool {
        let changed = change(&mut self.lists);
        if changed {
            self.lists.repair();
            self.save();
        }
        changed
    }

    /// Takes back the last edit.
    pub fn undo(&mut self) -> bool {
        let Some(before) = self.undo.pop() else {
            return false;
        };
        let after = self.restore(before);
        self.redo.push(after);
        true
    }

    /// Repeats an undone edit.
    pub fn redo(&mut self) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        let before = self.restore(after);
        self.undo.push(before);
        true
    }

    /// Puts a snapshot back and returns what it replaced. Undo restores
    /// entries, not playback: the entry that plays stays marked.
    fn restore(&mut self, snapshot: Playlists) -> Playlists {
        let playing = self.lists.playing.and_then(|album| {
            let target = self.lists.albums.get(album)?;
            let entry = target.entries.get(target.current?)?;
            Some((album, entry.path.clone()))
        });
        let replaced = std::mem::replace(&mut self.lists, snapshot);
        self.lists.repair();
        if let Some((album, path)) = playing
            && let Some(target) = self.lists.albums.get_mut(album)
            && let Some(index) = target.entries.iter().position(|entry| entry.path == path)
        {
            target.current = Some(index);
            self.lists.playing = Some(album);
        }
        self.save();
        replaced
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    // Albums.

    /// Adds an album after the shown one and shows it.
    pub fn add_album(&mut self, name: &str) -> usize {
        let at = self.lists.shown + 1;
        self.edit(|lists| {
            lists.albums.insert(at, Album::new(name));
            if let Some(playing) = lists.playing.as_mut()
                && *playing >= at
            {
                *playing += 1;
            }
            lists.shown = at;
            true
        });
        at
    }

    pub fn rename_album(&mut self, album: usize, name: &str) -> bool {
        self.edit(|lists| match lists.albums.get_mut(album) {
            Some(target) if target.name != name && !name.trim().is_empty() => {
                name.trim().clone_into(&mut target.name);
                true
            }
            _ => false,
        })
    }

    /// Removes an album (the last one is emptied instead).
    pub fn remove_album(&mut self, album: usize) -> bool {
        self.edit(|lists| {
            if album >= lists.albums.len() {
                return false;
            }
            if lists.albums.len() == 1 {
                lists.albums[0] = Album::new(&lists.albums[0].name);
                lists.playing = None;
                return true;
            }
            lists.albums.remove(album);
            lists.playing = match lists.playing {
                Some(playing) if playing == album => None,
                Some(playing) if playing > album => Some(playing - 1),
                other => other,
            };
            if lists.shown >= album && lists.shown > 0 {
                lists.shown -= 1;
            }
            true
        })
    }

    /// Shows an album's tab (not an edit).
    pub fn show_album(&mut self, album: usize) -> bool {
        self.update(|lists| {
            if album < lists.albums.len() && lists.shown != album {
                lists.shown = album;
                true
            } else {
                false
            }
        })
    }

    /// Adds a whole album, for example from a PotPlayer playlist. An empty
    /// album of the same name (a fresh "Default") takes its entries
    /// instead; returns where the album is.
    pub fn import_album(&mut self, album: Album) -> usize {
        let empty_twin = self
            .lists
            .albums
            .iter()
            .position(|existing| existing.name == album.name && existing.entries.is_empty());
        let at = empty_twin.unwrap_or(self.lists.albums.len());
        self.edit(|lists| {
            if empty_twin.is_some() {
                lists.albums[at] = album;
            } else {
                lists.albums.push(album);
            }
            true
        });
        at
    }

    // Entries.

    /// Inserts paths before `at` (`None`: at the end); returns where the
    /// first went.
    pub fn insert(&mut self, album: usize, at: Option<usize>, paths: &[String]) -> Option<usize> {
        let mut first = None;
        self.edit(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            if paths.is_empty() {
                return false;
            }
            let at = at.unwrap_or(target.entries.len()).min(target.entries.len());
            target
                .entries
                .splice(at..at, paths.iter().map(|path| Entry::new(path)));
            let count = paths.len();
            if let Some(current) = target.current.as_mut()
                && *current >= at
            {
                *current += count;
            }
            target.selected = (at..at + count).collect();
            target.anchor = Some(at);
            first = Some(at);
            true
        });
        first
    }

    /// Removes entries.
    pub fn remove(&mut self, album: usize, indices: &BTreeSet<usize>) -> bool {
        self.edit(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let removed: BTreeSet<usize> = indices
                .iter()
                .copied()
                .filter(|&index| index < target.entries.len())
                .collect();
            if removed.is_empty() {
                return false;
            }
            let mut index = 0;
            target.entries.retain(|_| {
                let keep = !removed.contains(&index);
                index += 1;
                keep
            });
            target.current = target
                .current
                .and_then(|current| shift_after_removal(current, &removed));
            let first_removed = removed.iter().next().copied().unwrap_or(0);
            target.selected.clear();
            if !target.entries.is_empty() {
                target
                    .selected
                    .insert(first_removed.min(target.entries.len() - 1));
            }
            target.anchor = target.selected.iter().next().copied();
            true
        })
    }

    /// Removes the selected entries.
    pub fn remove_selected(&mut self, album: usize) -> bool {
        let selected = self.lists.albums.get(album).map(|a| a.selected.clone());
        selected.is_some_and(|selected| self.remove(album, &selected))
    }

    /// Removes everything that is not selected.
    pub fn remove_unselected(&mut self, album: usize) -> bool {
        let Some(target) = self.lists.albums.get(album) else {
            return false;
        };
        let others = (0..target.entries.len())
            .filter(|index| !target.selected.contains(index))
            .collect();
        self.remove(album, &others)
    }

    /// Removes every entry.
    pub fn clear(&mut self, album: usize) -> bool {
        let Some(target) = self.lists.albums.get(album) else {
            return false;
        };
        let all = (0..target.entries.len()).collect();
        self.remove(album, &all)
    }

    /// Removes local files that no longer exist (URLs stay).
    pub fn remove_missing(&mut self, album: usize) -> bool {
        let Some(target) = self.lists.albums.get(album) else {
            return false;
        };
        let missing = target
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.path.contains("://") && !Path::new(&entry.path).exists())
            .map(|(index, _)| index)
            .collect();
        self.remove(album, &missing)
    }

    /// Moves the selected entries.
    pub fn move_selected(&mut self, album: usize, to: Move) -> bool {
        self.edit(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let len = target.entries.len();
            let selected: Vec<usize> = target
                .selected
                .iter()
                .copied()
                .filter(|&index| index < len)
                .collect();
            let (Some(&first), Some(&last)) = (selected.first(), selected.last()) else {
                return false;
            };
            // Position in the list without the moved entries.
            let remaining = len - selected.len();
            let before_count = |index: usize| selected.iter().filter(|&&s| s < index).count();
            let destination = match to {
                Move::Up => first.saturating_sub(1),
                Move::Down => (first + 1).min(remaining),
                Move::Top => 0,
                Move::Bottom => remaining,
                Move::Before(index) => index.min(len) - before_count(index.min(len)),
            };
            if destination == first && last - first + 1 == selected.len() {
                return false;
            }
            let current = target.current;
            let moved: Vec<(usize, Entry)> = selected
                .iter()
                .map(|&index| (index, target.entries[index].clone()))
                .collect();
            let mut rest: Vec<(usize, Entry)> = target
                .entries
                .drain(..)
                .enumerate()
                .filter(|(index, _)| !target.selected.contains(index))
                .collect();
            let tail = rest.split_off(destination.min(rest.len()));
            let order: Vec<(usize, Entry)> = rest.into_iter().chain(moved).chain(tail).collect();
            target.current =
                current.and_then(|current| order.iter().position(|(old, _)| *old == current));
            target.entries = order.into_iter().map(|(_, entry)| entry).collect();
            target.selected = (destination..destination + selected.len()).collect();
            target.anchor = Some(destination);
            true
        })
    }

    /// Sorts the album by file name, or shuffles it.
    pub fn sort(&mut self, album: usize, order: Order) -> bool {
        self.edit(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            if target.entries.len() < 2 {
                return false;
            }
            let current = target.current;
            let mut indexed: Vec<(usize, Entry)> = target.entries.drain(..).enumerate().collect();
            match order {
                Order::Ascending => indexed.sort_by(|a, b| {
                    a.1.label()
                        .to_lowercase()
                        .cmp(&b.1.label().to_lowercase())
                        .then(a.0.cmp(&b.0))
                }),
                Order::Descending => indexed.sort_by(|a, b| {
                    b.1.label()
                        .to_lowercase()
                        .cmp(&a.1.label().to_lowercase())
                        .then(a.0.cmp(&b.0))
                }),
                Order::Random(seed) => {
                    let mut state = seed;
                    for i in (1..indexed.len()).rev() {
                        let bound = u64::try_from(i + 1).unwrap_or(u64::MAX);
                        let j = usize::try_from(next_random(&mut state) % bound).unwrap_or(0);
                        indexed.swap(i, j);
                    }
                }
            }
            target.current =
                current.and_then(|current| indexed.iter().position(|(old, _)| *old == current));
            target.entries = indexed.into_iter().map(|(_, entry)| entry).collect();
            target.selected.clear();
            target.anchor = None;
            true
        })
    }

    // Selection and scrolling (not edits).

    pub fn select(&mut self, album: usize, index: usize, how: Select) -> bool {
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            if index >= target.entries.len() {
                return false;
            }
            let before = target.selected.clone();
            match how {
                Select::Only => {
                    target.selected = BTreeSet::from([index]);
                    target.anchor = Some(index);
                }
                Select::Toggle => {
                    if !target.selected.remove(&index) {
                        target.selected.insert(index);
                    }
                    target.anchor = Some(index);
                }
                Select::Range => {
                    let anchor = target.anchor.unwrap_or(index);
                    target.selected = (anchor.min(index)..=anchor.max(index)).collect();
                }
            }
            target.selected != before
        })
    }

    pub fn select_all(&mut self, album: usize) -> bool {
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let all: BTreeSet<usize> = (0..target.entries.len()).collect();
            let changed = target.selected != all;
            target.selected = all;
            changed
        })
    }

    pub fn invert_selection(&mut self, album: usize) -> bool {
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            target.selected = (0..target.entries.len())
                .filter(|index| !target.selected.contains(index))
                .collect();
            true
        })
    }

    pub fn clear_selection(&mut self, album: usize) -> bool {
        self.update(|lists| {
            lists.albums.get_mut(album).is_some_and(|target| {
                let changed = !target.selected.is_empty();
                target.selected.clear();
                changed
            })
        })
    }

    /// The selected entry the cursor keys move from.
    #[must_use]
    pub fn cursor(&self, album: usize) -> Option<usize> {
        let target = self.lists.albums.get(album)?;
        target
            .anchor
            .filter(|anchor| target.selected.contains(anchor))
            .or_else(|| target.selected.iter().next().copied())
            .or(target.current)
    }

    /// Scrolls so that `top` is the first row (clamped to the list).
    pub fn scroll_to(&mut self, album: usize, top: usize, visible: usize) -> bool {
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let max_top = target.entries.len().saturating_sub(visible.max(1));
            let top = top.min(max_top);
            let changed = target.top != top;
            target.top = top;
            changed
        })
    }

    /// Scrolls the least needed to show `index`.
    pub fn reveal(&mut self, album: usize, index: usize, visible: usize) -> bool {
        let Some(target) = self.lists.albums.get(album) else {
            return false;
        };
        let visible = visible.max(1);
        let top = if index < target.top {
            index
        } else if index >= target.top + visible {
            index + 1 - visible
        } else {
            return false;
        };
        self.scroll_to(album, top, visible)
    }

    // Playback.

    /// Marks an entry as the one that plays.
    pub fn set_playing(&mut self, album: usize, index: usize) -> bool {
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let Some(entry) = target.entries.get_mut(index) else {
                return false;
            };
            entry.played = true;
            entry.playable = true;
            target.current = Some(index);
            lists.playing = Some(album);
            true
        })
    }

    /// Records facts learnt while playing the current entry.
    pub fn update_current(&mut self, duration: Option<f64>, playable: bool) -> bool {
        self.update(|lists| {
            let Some(album) = lists.playing else {
                return false;
            };
            let Some(entry) = lists.albums.get_mut(album).and_then(|target| {
                target
                    .current
                    .and_then(|index| target.entries.get_mut(index))
            }) else {
                return false;
            };
            let duration = duration.or(entry.duration);
            let changed = entry.duration != duration || entry.playable != playable;
            entry.duration = duration;
            entry.playable = playable;
            changed
        })
    }

    /// The first playable entry of the album that plays (Repeat playlist).
    #[must_use]
    pub fn first_playable(&self) -> Option<(usize, usize)> {
        let album_index = self.lists.playing?;
        let album = self.lists.albums.get(album_index)?;
        album
            .entries
            .iter()
            .position(|entry| entry.playable)
            .map(|index| (album_index, index))
    }

    /// The next (`step` 1) or previous (-1) playable entry of the album that
    /// plays.
    #[must_use]
    pub fn neighbour(&self, step: i32) -> Option<(usize, usize)> {
        let album_index = self.lists.playing?;
        let album = self.lists.albums.get(album_index)?;
        let current = album.current?;
        let mut index = current;
        loop {
            index = if step >= 0 {
                index.checked_add(1)?
            } else {
                index.checked_sub(1)?
            };
            let entry = album.entries.get(index)?;
            if entry.playable {
                return Some((album_index, index));
            }
        }
    }
}

impl Playlists {
    /// Keeps indexes in range after loading or editing.
    fn repair(&mut self) {
        if self.albums.is_empty() {
            self.albums.push(Album::new("Default"));
        }
        self.shown = self.shown.min(self.albums.len() - 1);
        if self
            .playing
            .is_some_and(|playing| playing >= self.albums.len())
        {
            self.playing = None;
        }
        for album in &mut self.albums {
            let len = album.entries.len();
            album.current = album.current.filter(|&current| current < len);
            album.selected.retain(|&index| index < len);
            album.anchor = album.anchor.filter(|&anchor| anchor < len);
            album.top = album.top.min(len.saturating_sub(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-playlist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| format!("/m/{name}.mkv")).collect()
    }

    fn labels(library: &Library) -> Vec<String> {
        library
            .shown()
            .entries
            .iter()
            .map(|entry| entry.label().trim_end_matches(".mkv").to_owned())
            .collect()
    }

    #[test]
    fn edits_undo_and_redo() {
        let dir = scratch("undo");
        let mut library = Library::open(&dir).expect("open");
        library.insert(0, None, &paths(&["a", "b", "c", "d"]));
        library.set_playing(0, 2);
        library.select(0, 1, Select::Only);
        library.select(0, 3, Select::Toggle);
        assert!(library.remove_selected(0));
        assert_eq!(labels(&library), ["a", "c"]);
        assert_eq!(library.shown().current, Some(1), "c still plays");
        assert!(library.undo());
        assert_eq!(labels(&library), ["a", "b", "c", "d"]);
        assert!(library.redo());
        assert_eq!(labels(&library), ["a", "c"]);
        assert!(!library.redo());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn moves_keep_the_current_entry() {
        let dir = scratch("move");
        let mut library = Library::open(&dir).expect("open");
        library.insert(0, None, &paths(&["a", "b", "c", "d", "e"]));
        library.set_playing(0, 3);
        library.select(0, 0, Select::Only);
        library.select(0, 1, Select::Range);
        // a and b dropped before e.
        assert!(library.move_selected(0, Move::Before(4)));
        assert_eq!(labels(&library), ["c", "d", "a", "b", "e"]);
        assert_eq!(library.shown().current, Some(1), "d still plays");
        assert_eq!(library.shown().selected, BTreeSet::from([2, 3]));
        assert!(library.move_selected(0, Move::Top));
        assert_eq!(labels(&library), ["a", "b", "c", "d", "e"]);
        assert!(library.move_selected(0, Move::Down));
        assert_eq!(labels(&library), ["c", "a", "b", "d", "e"]);
        assert!(library.move_selected(0, Move::Bottom));
        assert_eq!(labels(&library), ["c", "d", "e", "a", "b"]);
        assert!(!library.move_selected(0, Move::Bottom), "already there");
        assert!(library.move_selected(0, Move::Up));
        assert_eq!(labels(&library), ["c", "d", "a", "b", "e"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn albums_sorting_and_neighbours() {
        let dir = scratch("albums");
        let mut library = Library::open(&dir).expect("open");
        library.insert(0, None, &paths(&["b", "a", "c"]));
        let films = library.add_album("Films");
        assert_eq!(library.lists().shown, films);
        library.insert(films, None, &paths(&["x", "y", "z"]));
        library.set_playing(films, 0);
        library.lists.albums[films].entries[1].playable = false;
        assert_eq!(library.neighbour(1), Some((films, 2)), "skips y");
        assert_eq!(library.neighbour(-1), None);
        assert!(library.show_album(0));
        assert!(library.sort(0, Order::Ascending));
        assert_eq!(labels(&library), ["a", "b", "c"]);
        assert!(library.sort(0, Order::Descending));
        assert_eq!(labels(&library), ["c", "b", "a"]);
        assert!(library.sort(0, Order::Random(7)));
        let mut shuffled = labels(&library);
        shuffled.sort();
        assert_eq!(shuffled, ["a", "b", "c"]);
        assert!(library.remove_album(0));
        assert_eq!(library.lists().albums.len(), 1);
        assert_eq!(library.lists().playing, Some(0), "Films moved down");
        assert!(library.undo());
        assert_eq!(library.lists().albums.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selection_and_scrolling() {
        let dir = scratch("select");
        let mut library = Library::open(&dir).expect("open");
        library.insert(0, None, &paths(&["a", "b", "c", "d", "e", "f"]));
        library.select(0, 1, Select::Only);
        library.select(0, 4, Select::Range);
        assert_eq!(library.shown().selected, BTreeSet::from([1, 2, 3, 4]));
        library.invert_selection(0);
        assert_eq!(library.shown().selected, BTreeSet::from([0, 5]));
        library.select_all(0);
        assert_eq!(library.shown().selected.len(), 6);
        assert!(library.reveal(0, 5, 3));
        assert_eq!(library.shown().top, 3);
        assert!(
            !library.scroll_to(0, 100, 3),
            "clamped to the end, where it is"
        );
        assert!(library.scroll_to(0, 1, 3));
        assert_eq!(library.shown().top, 1);
        assert_eq!(library.undo.len(), 1, "only the insert is an edit");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn local_paths_take_mpvs_form() {
        assert_eq!(
            local_path("https://x.org/a/../v.ts"),
            "https://x.org/a/../v.ts"
        );
        if cfg!(windows) {
            assert_eq!(local_path("C:/Films/a/../b.mkv"), r"C:\Films\b.mkv");
            assert_eq!(local_path(r"C:\Films\.\b.mkv"), r"C:\Films\b.mkv");
            assert_eq!(
                local_path("/home/u/rvp/crates/app/../../t/f.mkv"),
                r"\home\u\rvp\t\f.mkv"
            );
            assert_eq!(local_path(r"\\nas\Media\x\..\y.mkv"), r"\\nas\Media\y.mkv");
        } else {
            assert_eq!(local_path("/a/../b.mkv"), "/b.mkv");
            assert_eq!(local_path("/home/u/./films//a.mkv"), "/home/u/films/a.mkv");
        }
        assert_eq!(Entry::new("C:/x.mkv").path, local_path("C:/x.mkv"));
    }

    #[test]
    fn the_file_survives_a_reopen() {
        let dir = scratch("file");
        let mut library = Library::open(&dir).expect("open");
        library.insert(0, None, &paths(&["a", "b"]));
        library.add_album("Series");
        library.set_playing(0, 1);
        library.flush().expect("flush");
        let reopened = Library::open(&dir).expect("reopen");
        assert_eq!(reopened.lists(), library.lists());
        assert_eq!(reopened.playing().map(|(index, _)| index), Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
