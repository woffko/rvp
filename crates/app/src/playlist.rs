//! Albums (the playlist panel's tabs) and their entries (M3, D9).
//!
//! RVP's albums are the playlist: mpv plays one entry at a time, and
//! Next, Previous and the end of a file move through the album that plays.
//! Every edit can be undone. The albums live in `playlists.json` in the
//! state directory, written atomically by the storage writer after every
//! change, so a killed process keeps them. Several RVPs on one state folder
//! merge their saves by album ([`crate::storage`]'s rules,
//! `Playlists::merged`): an edit (what `Library::edit` changes) makes an
//! album this instance's, while selection, scrolling and playback marks
//! (what `Library::update` changes) give way to the other's edits.

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

/// The network schemes a text from outside may name (see [`admitted`]), in
/// lower case. They are the ones mpv itself lets a playlist name, which
/// reach no local file or device: the `safe_protos` of its
/// `stream_lavf.c` with `rtsp`, and curl's `ftp` of `stream_curl.c`. Left
/// out of those are the ones no list of films names (`data`, `gopher`,
/// `httpproxy`, `ipfs`); added is `udp`, the transport under `rtp`, which
/// IPTV lists name.
pub const NETWORK_SCHEMES: &[&str] = &[
    "dav", "davs", "ftp", "ftps", "http", "https", "mms", "mmsh", "mmshttp", "mmst", "rist",
    "rtmp", "rtmpe", "rtmps", "rtmpt", "rtmpte", "rtmpts", "rtp", "rtsp", "rtsps", "srt", "srtp",
    "udp", "webdav", "webdavs",
];

/// The scheme of a text a player may read as `scheme://...`. mpv takes a
/// letter, then letters, digits, `+`, `-` and `.` (its `mp_is_url`); a
/// digit first and `_` count here too, so that nothing a player may take
/// for a scheme passes as a path. One letter is a Windows drive
/// (`C://films`), which [`local_path`] writes as a path.
fn scheme(text: &str) -> Option<&str> {
    let (scheme, _) = text.split_once("://")?;
    (scheme.len() > 1
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+-._".contains(&byte)))
    .then_some(scheme)
}

/// True for an address of one of the [`NETWORK_SCHEMES`], in any case.
#[must_use]
pub fn network_address(text: &str) -> bool {
    scheme(text).is_some_and(|scheme| {
        NETWORK_SCHEMES
            .iter()
            .any(|known| scheme.eq_ignore_ascii_case(known))
    })
}

/// What a text from outside becomes as a playlist entry, or `None` when it
/// may not be one. Outside is whatever RVP did not make itself: a playlist
/// file's line, the clipboard, a dropped link, an address typed into a
/// dialog, another program's request. Such a text may name
///
/// * a local path (but not `-`, which is standard input to mpv),
/// * a `file://` URL of this computer, which becomes its path, or
/// * an address of the [`NETWORK_SCHEMES`], or `ytdl://`, whose rest goes
///   to yt-dlp ([`crate::ytdl`]).
///
/// Every other scheme mpv knows opens something a list or a link should
/// not: capture devices and FFmpeg's filter graphs, which read and write
/// files (`av://`), the process's descriptors (`fd://`), discs
/// (`dvd://`). RVP's own Open menu makes such addresses itself and opens
/// them without asking here, and an album keeps and plays them.
#[must_use]
pub fn admitted(text: &str) -> Option<String> {
    let text = text.trim();
    let Some(scheme) = scheme(text) else {
        return (!text.is_empty() && text != "-").then(|| text.to_owned());
    };
    if scheme.eq_ignore_ascii_case("file") {
        // `file:///path` or `file://localhost/path`: another host's file
        // is not this computer's.
        let rest = &text[scheme.len() + 3..];
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        return rest
            .starts_with('/')
            .then(|| crate::media::file_path(&format!("file://{rest}")));
    }
    (scheme.eq_ignore_ascii_case("ytdl") || network_address(text)).then(|| text.to_owned())
}

/// What the doors left out (see [`admitted`]): how many, and their kinds
/// for one notice however many there were.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Refused {
    count: usize,
    kinds: BTreeSet<String>,
}

impl Refused {
    /// Kinds a notice names.
    const KINDS: usize = 4;

    /// Counts one text that [`admitted`] refused.
    pub fn add(&mut self, text: &str) {
        self.count += 1;
        // The scheme only: the rest of the text is whatever its writer
        // wished to show.
        let kind = scheme(text.trim()).map_or_else(
            || "standard input".to_owned(),
            |scheme| format!("{}://", scheme.to_ascii_lowercase()),
        );
        if self.kinds.len() < Self::KINDS {
            self.kinds.insert(kind);
        }
    }

    /// Adds what another door left out.
    pub fn merge(&mut self, other: Self) {
        self.count += other.count;
        for kind in other.kinds {
            if self.kinds.len() < Self::KINDS {
                self.kinds.insert(kind);
            }
        }
    }

    /// How many texts were left out.
    #[must_use]
    pub fn count(&self) -> usize {
        self.count
    }

    /// The notice, when anything was left out.
    #[must_use]
    pub fn notice(&self) -> Option<String> {
        let what = match self.count {
            0 => return None,
            1 => "1 address".to_owned(),
            count => format!("{count} addresses"),
        };
        let kinds: Vec<&str> = self.kinds.iter().map(String::as_str).collect();
        Some(format!(
            "Not opened: {what} ({}). Only files and network addresses are taken this way.",
            kinds.join(", ")
        ))
    }
}

/// The texts of `items` that may be entries, as [`admitted`] keeps them,
/// and what was left out.
#[must_use]
pub fn admit_all<S: AsRef<str>>(items: &[S]) -> (Vec<String>, Refused) {
    let mut refused = Refused::default();
    let kept = items
        .iter()
        .filter_map(|item| {
            let kept = admitted(item.as_ref());
            if kept.is_none() {
                refused.add(item.as_ref());
            }
            kept
        })
        .collect();
    (kept, refused)
}

/// True for a Windows path that names another computer or a device, not a
/// file on a drive: `\\server\share\...` (also as `\\?\UNC\...`), a device
/// (`\\.\...`) and the other `\\?\` forms but a drive's (`\\?\C:\...`).
/// Either slash counts, as Windows takes both. Opening one reaches for the
/// server with the user's account, so RVP opens it only to play it.
#[must_use]
pub fn windows_network_path(path: &str) -> bool {
    let separator = |byte: u8| byte == b'\\' || byte == b'/';
    let bytes = path.as_bytes();
    if bytes.len() < 3 || !separator(bytes[0]) || !separator(bytes[1]) {
        return false;
    }
    // `\\?\C:\...` is a file on a drive.
    !matches!(
        &bytes[2..],
        [b'?', sep, drive, b':', ..] if separator(*sep) && drive.is_ascii_alphabetic()
    )
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
    /// The playback queue: entries (by path) that play next, in order.
    pub queue: Vec<String>,
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
pub(crate) fn next_random(state: &mut u64) -> u64 {
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
        let (mut lists, read) = match storage::load_json::<Playlists>(&path)? {
            Some((lists, bytes)) => (lists, Some(bytes)),
            None => (Playlists::default(), None),
        };
        lists.repair();
        let writer = storage::Writer::new()?;
        writer.merge_into(&path, read, storage::json_bytes(&lists)?, merge_file);
        Ok(Self {
            lists,
            undo: Vec::new(),
            redo: Vec::new(),
            path,
            writer,
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
            Err(error) => crate::log::report(
                crate::log::Level::Critical,
                &format!("cannot serialize the playlists: {error}"),
            ),
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

impl Library {
    // M11: the queue, durations found in the background, and the edits of
    // the playlist panel's commands.

    /// The first queued entry of the album that plays (or the shown one),
    /// taken off the queue.
    pub fn take_queued(&mut self) -> Option<(usize, usize)> {
        let album = self.lists.playing.unwrap_or(self.lists.shown);
        let mut found = None;
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            while !target.queue.is_empty() {
                let path = target.queue.remove(0);
                if let Some(index) = target
                    .entries
                    .iter()
                    .position(|entry| entry.path == path && entry.playable)
                {
                    found = Some((album, index));
                    break;
                }
            }
            true
        });
        found
    }

    /// Queues entries (each once) of an album.
    pub fn queue_add(&mut self, album: usize, indices: &[usize]) -> bool {
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let mut changed = false;
            for &index in indices {
                if let Some(entry) = target.entries.get(index)
                    && !target.queue.contains(&entry.path)
                {
                    target.queue.push(entry.path.clone());
                    changed = true;
                }
            }
            changed
        })
    }

    pub fn queue_clear(&mut self, album: usize) -> bool {
        self.update(|lists| {
            lists.albums.get_mut(album).is_some_and(|target| {
                let had = !target.queue.is_empty();
                target.queue.clear();
                had
            })
        })
    }

    /// Records a duration found in the background for every entry of the
    /// path.
    pub fn set_duration(&mut self, path: &str, seconds: f64) -> bool {
        self.update(|lists| {
            let mut changed = false;
            for entry in lists
                .albums
                .iter_mut()
                .flat_map(|album| album.entries.iter_mut())
                .filter(|entry| entry.path == path)
            {
                if entry
                    .duration
                    .is_none_or(|known| (known - seconds).abs() > 0.5)
                {
                    entry.duration = Some(seconds);
                    changed = true;
                }
            }
            changed
        })
    }

    /// Puts an album's entries in a new order (`order[i]` is the entry that
    /// goes to row `i`).
    pub fn reorder(&mut self, album: usize, order: &[usize]) -> bool {
        self.edit(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let len = target.entries.len();
            let mut seen = vec![false; len];
            if order.len() != len
                || order
                    .iter()
                    .any(|&index| index >= len || std::mem::replace(&mut seen[index], true))
            {
                return false;
            }
            if order.iter().enumerate().all(|(row, &index)| row == index) {
                return false;
            }
            let entries = std::mem::take(&mut target.entries);
            target.entries = order.iter().map(|&index| entries[index].clone()).collect();
            target.current = target
                .current
                .and_then(|current| order.iter().position(|&index| index == current));
            target.selected.clear();
            target.anchor = None;
            true
        })
    }

    /// Gives entries a title (`None`: the file name again).
    pub fn set_title(&mut self, album: usize, indices: &[usize], title: Option<&str>) -> bool {
        self.edit(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            let title = title.map(str::trim).filter(|title| !title.is_empty());
            let mut changed = false;
            for &index in indices {
                if let Some(entry) = target.entries.get_mut(index)
                    && entry.title.as_deref() != title
                {
                    entry.title = title.map(str::to_owned);
                    changed = true;
                }
            }
            changed
        })
    }

    /// A file was renamed or moved: its entries follow it.
    pub fn replace_path(&mut self, old: &str, new: &str) -> bool {
        let new = local_path(new);
        self.update(|lists| {
            let mut changed = false;
            for album in &mut lists.albums {
                for entry in album.entries.iter_mut().filter(|entry| entry.path == old) {
                    entry.path.clone_from(&new);
                    changed = true;
                }
                for queued in album.queue.iter_mut().filter(|queued| *queued == old) {
                    queued.clone_from(&new);
                }
            }
            changed
        })
    }

    /// Removes the entries an album's predicate picks.
    pub fn remove_where(&mut self, album: usize, pick: impl Fn(usize, &Entry) -> bool) -> bool {
        let Some(target) = self.lists.albums.get(album) else {
            return false;
        };
        let picked: BTreeSet<usize> = target
            .entries
            .iter()
            .enumerate()
            .filter(|(index, entry)| pick(*index, entry))
            .map(|(index, _)| index)
            .collect();
        self.remove(album, &picked)
    }

    /// Removes the entries of these paths from every album (deleted files).
    pub fn remove_paths(&mut self, paths: &[String]) -> bool {
        let mut changed = false;
        for album in 0..self.lists.albums.len() {
            changed |= self.remove_where(album, |_, entry| paths.contains(&entry.path));
        }
        changed
    }

    /// Selects the entries whose title or file name holds `text` (any
    /// case); returns the first.
    pub fn select_matching(&mut self, album: usize, text: &str) -> Option<usize> {
        let needle = text.trim().to_lowercase();
        if needle.is_empty() {
            return None;
        }
        let matches: BTreeSet<usize> = self
            .lists
            .albums
            .get(album)?
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.label().to_lowercase().contains(&needle)
                    || crate::view::file_name(&entry.path)
                        .to_lowercase()
                        .contains(&needle)
            })
            .map(|(index, _)| index)
            .collect();
        let first = matches.iter().next().copied();
        self.update(|lists| {
            let Some(target) = lists.albums.get_mut(album) else {
                return false;
            };
            target.anchor = first;
            target.selected = matches;
            true
        });
        first
    }

    /// Clears the album named "Default" (when the Default album is not to
    /// be remembered).
    pub fn forget_default(&mut self) -> bool {
        let Some(album) = self
            .lists
            .albums
            .iter()
            .position(|album| album.name == "Default")
        else {
            return false;
        };
        self.update(|lists| {
            let target = &mut lists.albums[album];
            let had = !target.entries.is_empty();
            *target = Album::new("Default");
            if lists.playing == Some(album) {
                lists.playing = None;
            }
            had
        })
    }
}

impl Album {
    /// What an edit changes: the name and the entries (their files, titles
    /// and order). The rest is the view and the playback marks.
    fn edited_part(&self) -> (&str, Vec<(&str, Option<&str>)>) {
        let entries = self
            .entries
            .iter()
            .map(|entry| (entry.path.as_str(), entry.title.as_deref()))
            .collect();
        (&self.name, entries)
    }

    /// An album this instance changed (`ours`, from `base`) against the
    /// disk's: an edit here makes it ours whole; the view and marks alone
    /// are ours while the other instance left the entries as they were.
    fn merged(disk: Self, base: &Self, ours: &Self) -> Self {
        let known = base.edited_part();
        if ours.edited_part() != known || disk.edited_part() == known {
            ours.clone()
        } else {
            disk
        }
    }
}

/// An album's name and its place among the albums of that name: how
/// several RVPs know it (`Playlists::merged`).
fn album_id(albums: &[Album], index: Option<usize>) -> Option<(String, usize)> {
    let index = index?;
    let album = albums.get(index)?;
    let before = albums[..index]
        .iter()
        .filter(|other| other.name == album.name)
        .count();
    Some((album.name.clone(), before))
}

/// Where the album of [`album_id`] `id` is.
fn album_index(albums: &[Album], id: &(String, usize)) -> Option<usize> {
    albums
        .iter()
        .enumerate()
        .filter(|(_, album)| album.name == id.0)
        .nth(id.1)
        .map(|(index, _)| index)
}

/// `playlists.json`'s rule for several RVPs ([`storage::Merge`]).
fn merge_file(disk: &[u8], base: &[u8], ours: &[u8]) -> Option<Vec<u8>> {
    storage::merge_json(disk, base, ours, Playlists::merged)
}

impl Playlists {
    /// This instance's albums `ours`, changed from `base`, put onto `disk`,
    /// the newest file on disk ([`crate::storage`]): by album, each known
    /// by its name; the shown and the playing album are this instance's
    /// when it changed them, else the disk's.
    fn merged(disk: Self, base: &Self, ours: &Self) -> Self {
        // Which album: this instance's choice when it changed it, else the
        // disk's (and the other one when the chosen album is gone).
        let choose = |ours_at, base_at, disk_at| {
            let mine = album_id(&ours.albums, ours_at);
            let theirs = album_id(&disk.albums, disk_at);
            if mine == album_id(&base.albums, base_at) {
                (theirs, mine)
            } else {
                (mine, theirs)
            }
        };
        let shown = choose(Some(ours.shown), Some(base.shown), Some(disk.shown));
        let (playing, _) = choose(ours.playing, base.playing, disk.playing);
        let albums = storage::merge_list(
            disk.albums,
            &base.albums,
            &ours.albums,
            |album| album.name.clone(),
            Album::merged,
        );
        let find = |id: Option<(String, usize)>| id.and_then(|id| album_index(&albums, &id));
        let mut merged = Self {
            version: ours.version,
            shown: find(shown.0).or_else(|| find(shown.1)).unwrap_or(0),
            playing: find(playing),
            albums,
        };
        merged.repair();
        merged
    }

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
            let paths: BTreeSet<&str> = album.entries.iter().map(|e| e.path.as_str()).collect();
            album.queue.retain(|queued| paths.contains(queued.as_str()));
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
    fn network_addresses_are_admitted_in_any_case() {
        for scheme in NETWORK_SCHEMES {
            let address = format!("{scheme}://example.org/live/1");
            assert_eq!(admitted(&address).as_deref(), Some(address.as_str()));
            assert!(network_address(&address), "{address}");
            let upper = format!("{}://example.org/live/1", scheme.to_uppercase());
            assert_eq!(admitted(&upper).as_deref(), Some(upper.as_str()));
        }
        // Around the text only the blanks go.
        assert_eq!(
            admitted("  https://example.org/a b.mkv\t").as_deref(),
            Some("https://example.org/a b.mkv")
        );
        // yt-dlp's own prefix, which is no network address of an answer.
        assert_eq!(
            admitted("ytdl://example.org/watch").as_deref(),
            Some("ytdl://example.org/watch")
        );
        assert!(!network_address("ytdl://example.org/watch"));
        assert!(!network_address("/films/a.mkv"));
    }

    #[test]
    fn other_schemes_are_refused() {
        // Every other protocol of mpv and FFmpeg: devices and filter
        // graphs, descriptors, discs, mpv's own containers of addresses,
        // FFmpeg's raw and file protocols.
        for scheme in [
            "av",
            "avdevice",
            "fd",
            "fdclose",
            "lavf",
            "ffmpeg",
            "dvd",
            "dvdnav",
            "bd",
            "br",
            "bluray",
            "cdda",
            "dvb",
            "dvda",
            "rvpdisc",
            "edl",
            "mf",
            "memory",
            "hex",
            "slice",
            "appending",
            "archive",
            "concat",
            "null",
            "env",
            "mpv",
            "aes-ctr",
            "data",
            "gopher",
            "gophers",
            "httpproxy",
            "ipfs",
            "ipns",
            "tcp",
            "tls",
            "unix",
            "smb",
            "sftp",
            "pipe",
            "crypto",
            "subfile",
            "cache",
            "async",
            "md5",
            "tee",
            "hls+http",
            "x-unknown",
            "a_b",
            "3gp",
        ] {
            for text in [
                format!("{scheme}://lavfi:testsrc"),
                format!("{}://9", scheme.to_uppercase()),
            ] {
                assert_eq!(admitted(&text), None, "{text}");
                assert!(!network_address(&text), "{text}");
            }
        }
        assert_eq!(admitted("Av://lavfi:testsrc"), None);
        // Standard input, and nothing.
        assert_eq!(admitted("-"), None);
        assert_eq!(admitted(" - "), None);
        assert_eq!(admitted(""), None);
    }

    #[test]
    fn local_paths_are_admitted() {
        for path in [
            "/films/a.mkv",
            "films/a.mkv",
            "a.mkv",
            // A drive is no scheme, with either slash, nor with two.
            r"C:\films\a.mkv",
            "C:/films/a.mkv",
            "C://films/a.mkv",
            r"\\nas\films\a.mkv",
            "//nas/films/a.mkv",
            // `://` after a folder is part of a name.
            "/films/av://a.mkv",
            r"C:\films\av://a.mkv",
            "./-",
            // No scheme a player reads: its letters are not ASCII.
            "фильм://a.mkv",
        ] {
            assert_eq!(admitted(path).as_deref(), Some(path), "{path}");
        }
    }

    #[test]
    fn file_urls_become_this_computers_paths() {
        let (url, path) = if cfg!(windows) {
            ("file:///C:/films/a%20b.mkv", "C:/films/a b.mkv")
        } else {
            ("file:///films/a%20b.mkv", "/films/a b.mkv")
        };
        assert_eq!(admitted(url).as_deref(), Some(path));
        assert_eq!(
            admitted(&url.replace("file:", "FILE:")).as_deref(),
            Some(path)
        );
        assert_eq!(
            admitted("file://localhost/films/a.mkv").as_deref(),
            Some("/films/a.mkv")
        );
        // Another host's file, and what is no path after the scheme.
        assert_eq!(admitted("file://nas/films/a.mkv"), None);
        assert_eq!(admitted("file://-"), None);
        assert_eq!(admitted("file://av://lavfi:testsrc"), None);
        assert_eq!(admitted("file://"), None);
        // A path that hides a scheme stays a path: no player reads one
        // after a slash, and `local_path` takes the double slash away.
        let hidden = admitted("file:///av%3A//lavfi:testsrc").expect("a path");
        assert_eq!(hidden, "/av://lavfi:testsrc");
        assert_eq!(admitted(&hidden).as_deref(), Some(hidden.as_str()));
        assert!(!Entry::new(&hidden).path.contains("://"));
    }

    #[test]
    fn what_is_left_out_makes_one_notice() {
        let (kept, refused) = admit_all(&[
            "/films/a.mkv",
            "av://lavfi:testsrc",
            "https://example.org/v",
            "AV://v4l2:/dev/video0",
            "fdclose://9",
            "-",
            "file:///films/b.mkv",
        ]);
        assert_eq!(
            kept,
            ["/films/a.mkv", "https://example.org/v", "/films/b.mkv"]
        );
        assert_eq!(refused.count(), 4);
        // The kinds only: nothing else of a refused text is shown.
        assert_eq!(
            refused.notice().as_deref(),
            Some(
                "Not opened: 4 addresses (av://, fdclose://, standard input). \
                 Only files and network addresses are taken this way."
            )
        );
        let (_, one) = admit_all(&["dvd://1"]);
        assert_eq!(
            one.notice().as_deref(),
            Some(
                "Not opened: 1 address (dvd://). \
                 Only files and network addresses are taken this way."
            )
        );
        let (kept, none) = admit_all(&["/films/a.mkv"]);
        assert_eq!((kept.len(), none.notice()), (1, None));
        // A long list names a few kinds and counts the rest.
        let many: Vec<String> = (0..500).map(|n| format!("x{n}://y")).collect();
        let (kept, mut refused) = admit_all(&many);
        assert!(kept.is_empty());
        refused.merge(one);
        assert_eq!(refused.count(), 501);
        let notice = refused.notice().expect("a notice");
        assert!(
            notice.starts_with("Not opened: 501 addresses ("),
            "{notice}"
        );
        assert!(notice.len() < 160, "{notice}");
    }

    #[test]
    fn windows_paths_of_other_computers_and_devices() {
        for path in [
            r"\\server\share\a.mkv",
            "//server/share/a.mkv",
            r"\/server\share\a.mkv",
            r"\\?\UNC\server\share\a.mkv",
            r"\\.\pipe\name",
            r"\\.\PhysicalDrive0",
            r"\\?\GLOBALROOT\Device\Mup\server\share\a.mkv",
            r"\\?\Volume{0}\a.mkv",
        ] {
            assert!(windows_network_path(path), "{path}");
        }
        for path in [
            r"C:\films\a.mkv",
            "C:/films/a.mkv",
            r"\\?\C:\films\a.mkv",
            r"\films\a.mkv",
            "/films/a.mkv",
            "films/a.mkv",
            r"\\",
            "",
        ] {
            assert!(!windows_network_path(path), "{path}");
        }
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
