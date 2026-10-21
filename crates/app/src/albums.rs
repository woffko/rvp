//! M11's playlist panel commands (PotPlayer's playlist menu and its
//! Playlist page): the playback queue, sorting by more fields, search,
//! titles, renaming, moving and copying files, the clipboard, playlist
//! files, file information, the Recycle Bin, durations found in the
//! background, thumbnail views and a new instance.
//!
//! File operations that may take long (moving and copying files) run on
//! a thread of their own and report through [`App::file_job_done`]; the
//! Recycle Bin is the shell's (Windows' shell file operation), after the
//! user confirmed ([`UiRequest::ConfirmRecycle`]).

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use rvp_engine::Engine;
use serde::{Deserialize, Serialize};

use crate::action::{ListAction, SortField};
use crate::app::{App, UiRequest};
use crate::playlist::{Entry, Move, is_url};

/// The Playlist page's settings (`playlist` in `settings.json`).
// Independent options as PotPlayer's page lists them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ListSettings {
    /// 0: the list; 2 to 7: thumbnails with details on that many rows.
    pub thumbnail_rows: u8,
    /// Thumbnails in 16:9 boxes (otherwise the video's shape).
    pub wide_thumbnails: bool,
    /// Where in a file its thumbnail is taken, in percent.
    pub thumbnail_position: f64,
    /// Durations are found in the background once files are added.
    pub detect_durations: bool,
    /// The Default album and the recent URLs stay across runs.
    pub remember_default: bool,
    /// Choosing an album in a menu plays it at once.
    pub instant_albums: bool,
    /// A file already in the album is not added again.
    pub no_duplicates: bool,
    /// Opening files adds them (otherwise the shown album is cleared
    /// first).
    pub append_on_open: bool,
    /// Dropped files sort by name before they are added.
    pub sort_dropped: bool,
    /// Playing an entry from the panel gives the window the keyboard.
    pub focus_main: bool,
}

impl Default for ListSettings {
    fn default() -> Self {
        Self {
            thumbnail_rows: 0,
            wide_thumbnails: true,
            thumbnail_position: 20.0,
            detect_durations: true,
            remember_default: true,
            instant_albums: true,
            no_duplicates: false,
            append_on_open: true,
            sort_dropped: false,
            focus_main: true,
        }
    }
}

impl ListSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if self.thumbnail_rows == 1 || self.thumbnail_rows > 7 {
            notes.push(format!(
                "playlist.thumbnail-rows must be 0 or 2 to 7: {} used",
                if self.thumbnail_rows == 1 { 2 } else { 7 }
            ));
            self.thumbnail_rows = if self.thumbnail_rows == 1 { 2 } else { 7 };
        }
        if !self.thumbnail_position.is_finite() || !(0.0..=95.0).contains(&self.thumbnail_position)
        {
            notes.push("playlist.thumbnail-position must be 0 to 95: 20 used".to_owned());
            self.thumbnail_position = 20.0;
        }
    }
}

/// A file operation of the playlist panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    Move,
    Copy,
}

/// What a file job did: the files that went (old and new paths) and the
/// errors.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobReport {
    pub moved: Vec<(String, String)>,
    pub copied: usize,
    pub errors: Vec<String>,
}

/// A year (1900 to 2099) in a text, for "By release date".
fn release_year(text: &str) -> Option<u32> {
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(3)).find_map(|at| {
        let window = &bytes[at..at + 4];
        let digits = window.iter().all(u8::is_ascii_digit);
        let alone = (at == 0 || !bytes[at - 1].is_ascii_digit())
            && bytes.get(at + 4).is_none_or(|next| !next.is_ascii_digit());
        let year: u32 = std::str::from_utf8(window).ok()?.parse().ok()?;
        (digits && alone && (1900..2100).contains(&year)).then_some(year)
    })
}

/// The new order of `entries` sorted by `field` (stable: equal keys keep
/// their order).
#[must_use]
pub fn sorted_order(entries: &[Entry], field: SortField) -> Vec<usize> {
    let name = |entry: &Entry| crate::view::file_name(&entry.path).to_lowercase();
    let extension = |entry: &Entry| {
        Path::new(&entry.path)
            .extension()
            .map(|extension| extension.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    let metadata = |entry: &Entry| {
        (!is_url(&entry.path))
            .then(|| std::fs::metadata(&entry.path).ok())
            .flatten()
    };
    let mut keyed: Vec<(usize, (u64, String))> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let key = match field {
                SortField::Title => (0, entry.label().to_lowercase()),
                SortField::Name => (0, name(entry)),
                SortField::Extension => (0, format!("{}\u{0}{}", extension(entry), name(entry))),
                SortField::Size => (metadata(entry).map_or(u64::MAX, |m| m.len()), name(entry)),
                SortField::Duration => (
                    // Durations in milliseconds are far inside u64.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    entry
                        .duration
                        .map_or(u64::MAX, |seconds| (seconds.max(0.0) * 1000.0) as u64),
                    name(entry),
                ),
                SortField::Date => (
                    metadata(entry)
                        .and_then(|m| m.modified().ok())
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(u64::MAX, |since| since.as_secs()),
                    name(entry),
                ),
                SortField::Folder => {
                    let folder = Path::new(&entry.path)
                        .parent()
                        .map(|parent| parent.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    (0, format!("{folder}\u{0}{}", name(entry)))
                }
                SortField::Release => (
                    u64::from(release_year(entry.label()).unwrap_or(u32::MAX)),
                    entry.label().to_lowercase(),
                ),
            };
            (index, key)
        })
        .collect();
    keyed.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    keyed.into_iter().map(|(index, _)| index).collect()
}

/// Moves or copies files into `folder` (a file of the same name there is
/// not overwritten); moving across drives copies, then deletes.
#[must_use]
pub fn run_file_job(files: &[PathBuf], folder: &Path, operation: FileOperation) -> JobReport {
    let mut report = JobReport::default();
    for file in files {
        let Some(name) = file.file_name() else {
            continue;
        };
        let target = folder.join(name);
        if target.exists() {
            report
                .errors
                .push(format!("{} is there already", target.display()));
            continue;
        }
        let result = match operation {
            FileOperation::Copy => std::fs::copy(file, &target).map(|_| ()),
            FileOperation::Move => std::fs::rename(file, &target)
                .or_else(|_| std::fs::copy(file, &target).and_then(|_| std::fs::remove_file(file))),
        };
        match result {
            Ok(()) => match operation {
                FileOperation::Copy => report.copied += 1,
                FileOperation::Move => report.moved.push((
                    file.to_string_lossy().into_owned(),
                    target.to_string_lossy().into_owned(),
                )),
            },
            Err(error) => report.errors.push(format!("{}: {error}", file.display())),
        }
    }
    report
}

/// The file jobs under way.
#[derive(Debug, Default)]
pub(crate) struct FileJobs {
    results: Vec<mpsc::Receiver<JobReport>>,
}

impl App {
    /// The playlist panel's M11 commands ([`App::list_action`] has the
    /// rest).
    pub(crate) fn album_command(
        &mut self,
        engine: &Engine,
        action: ListAction,
        visible: usize,
    ) -> Option<UiRequest> {
        match action {
            ListAction::PlayNewInstance
            | ListAction::OpenStorageFolder
            | ListAction::OpenPlaylist
            | ListAction::SavePlaylist
            | ListAction::OpenLocation
            | ListAction::RenameFile
            | ListAction::DeleteFiles
            | ListAction::DeleteRelated
            | ListAction::MoveFiles
            | ListAction::MoveRelated
            | ListAction::CopyFiles
            | ListAction::CopyRelated
            | ListAction::FileInfo
            | ListAction::ThumbnailImages => self.album_file_command(engine, action),
            ListAction::ToggleInstantAlbums
            | ListAction::ToggleRememberDefault
            | ListAction::View(_)
            | ListAction::ReloadThumbnails
            | ListAction::ToggleWideThumbnails
            | ListAction::DetectDurations => self.album_view_command(action),
            _ => self.album_edit_command(action, visible),
        }
    }

    /// The selected entries (or the current one) and their paths.
    fn targets_and_paths(&self) -> (Vec<usize>, Vec<String>) {
        let targets = self.library().shown().targets();
        let paths = targets
            .iter()
            .filter_map(|&index| self.library().shown().entries.get(index))
            .map(|entry| entry.path.clone())
            .collect();
        (targets, paths)
    }

    /// Commands on the entries' files and playlist files.
    fn album_file_command(&mut self, engine: &Engine, action: ListAction) -> Option<UiRequest> {
        let (targets, paths) = self.targets_and_paths();
        match action {
            ListAction::PlayNewInstance => {
                (!paths.is_empty()).then_some(UiRequest::NewInstance(paths))
            }
            ListAction::OpenStorageFolder => {
                Some(UiRequest::OpenExternal(self.state_dir().to_path_buf()))
            }
            ListAction::OpenPlaylist => Some(UiRequest::ListDialog {
                kind: "open",
                text: String::new(),
            }),
            ListAction::SavePlaylist => Some(UiRequest::ListDialog {
                kind: "save",
                text: format!("{}.m3u8", self.library().shown().name),
            }),
            ListAction::OpenLocation => {
                let path = paths.first().filter(|path| !is_url(path))?;
                Some(UiRequest::ShowFile(PathBuf::from(path)))
            }
            ListAction::RenameFile => {
                let path = paths.first().filter(|path| !is_url(path))?;
                Some(UiRequest::ListDialog {
                    kind: "rename",
                    text: crate::view::file_name(path).to_owned(),
                })
            }
            ListAction::DeleteFiles | ListAction::DeleteRelated => {
                Self::recycle_request(&paths, action == ListAction::DeleteRelated)
            }
            ListAction::MoveFiles
            | ListAction::MoveRelated
            | ListAction::CopyFiles
            | ListAction::CopyRelated => Some(UiRequest::ListDialog {
                kind: match action {
                    ListAction::MoveFiles => "move",
                    ListAction::MoveRelated => "move-related",
                    ListAction::CopyFiles => "copy",
                    _ => "copy-related",
                },
                text: String::new(),
            }),
            ListAction::FileInfo => {
                let index = *targets.first()?;
                Some(UiRequest::ListDialog {
                    kind: "info",
                    text: self.file_info(engine, index),
                })
            }
            ListAction::ThumbnailImages => Some(self.sheets_for_entries(&targets)),
            _ => None,
        }
    }

    /// The view's and the panel's options.
    fn album_view_command(&mut self, action: ListAction) -> Option<UiRequest> {
        match action {
            ListAction::ToggleInstantAlbums => {
                let list = &mut self.settings.playlist;
                list.instant_albums = !list.instant_albums;
                let on = list.instant_albums;
                Some(self.toggled("Start albums from menus at once", on))
            }
            ListAction::ToggleRememberDefault => {
                let list = &mut self.settings.playlist;
                list.remember_default = !list.remember_default;
                let on = list.remember_default;
                Some(self.toggled("Remember default playlist/recent URL items", on))
            }
            ListAction::View(rows) => {
                self.settings.playlist.thumbnail_rows = rows;
                self.save_settings();
                Some(UiRequest::ListView)
            }
            ListAction::ReloadThumbnails => {
                self.cancel_entry_thumbnails();
                Some(UiRequest::ListView)
            }
            ListAction::ToggleWideThumbnails => {
                let list = &mut self.settings.playlist;
                list.wide_thumbnails = !list.wide_thumbnails;
                self.save_settings();
                self.cancel_entry_thumbnails();
                Some(UiRequest::ListView)
            }
            ListAction::DetectDurations => {
                let all: Vec<String> = self
                    .library()
                    .shown()
                    .entries
                    .iter()
                    .filter(|entry| !is_url(&entry.path))
                    .map(|entry| entry.path.clone())
                    .collect();
                self.redetect_durations(&all);
                Some(UiRequest::Osd(format!("Detecting {} durations", all.len())))
            }
            _ => None,
        }
    }

    /// Edits of the shown album: the queue, titles, search, the clipboard,
    /// removals, sorting and moves.
    fn album_edit_command(&mut self, action: ListAction, visible: usize) -> Option<UiRequest> {
        let album = self.library().lists().shown;
        let (targets, paths) = self.targets_and_paths();
        match action {
            ListAction::QueueAdd => {
                self.library_mut().queue_add(album, &targets);
                let count = self.library().shown().queue.len();
                Some(UiRequest::Osd(format!("Playback queue: {count}")))
            }
            ListAction::QueueClear => {
                self.library_mut().queue_clear(album);
                Some(UiRequest::Osd("Playback queue cleared".to_owned()))
            }
            ListAction::AddUrl => Some(UiRequest::ListDialog {
                kind: "url",
                text: String::new(),
            }),
            ListAction::EditTitle => {
                let index = *targets.first()?;
                let entry = self.library().shown().entries.get(index)?;
                Some(UiRequest::ListDialog {
                    kind: "title",
                    text: entry.label().to_owned(),
                })
            }
            ListAction::Search => Some(UiRequest::ListDialog {
                kind: "search",
                text: String::new(),
            }),
            ListAction::Copy | ListAction::CopyPaths => {
                (!paths.is_empty()).then(|| UiRequest::CopyText(paths.join("\n")))
            }
            ListAction::Paste => Some(UiRequest::ReadClipboard(crate::sources::Paste::List)),
            ListAction::PasteBelow => {
                Some(UiRequest::ReadClipboard(crate::sources::Paste::ListBelow))
            }
            ListAction::PasteRecent => {
                let recent = self.marks().marks().recent.clone();
                self.library_mut().insert(album, None, &recent);
                None
            }
            ListAction::AddPlayingFolder => {
                let (_, entry) = self.library().playing()?;
                let folder = Path::new(&entry.path).parent()?.to_path_buf();
                let files: Vec<String> = crate::sources::folder_media(&folder)
                    .into_iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                self.add_paths(album, &files);
                None
            }
            ListAction::RemovePlayed => {
                self.library_mut()
                    .remove_where(album, |_, entry| entry.played);
                None
            }
            ListAction::RemoveDuplicates => {
                let mut seen = std::collections::BTreeSet::new();
                let duplicates: Vec<bool> = self
                    .library()
                    .shown()
                    .entries
                    .iter()
                    .map(|entry| !seen.insert(entry.path.clone()))
                    .collect();
                self.library_mut()
                    .remove_where(album, |index, _| duplicates.get(index) == Some(&true));
                None
            }
            ListAction::SortBy(field) => {
                let order = sorted_order(&self.library().shown().entries, field);
                self.library_mut().reorder(album, &order);
                None
            }
            ListAction::MoveTopOfPage => {
                let top = self.library().shown().top;
                self.library_mut().move_selected(album, Move::Before(top));
                None
            }
            ListAction::MoveBottomOfPage => {
                let top = self.library().shown().top;
                let end = (top + visible.max(1)).min(self.library().shown().entries.len());
                self.library_mut().move_selected(album, Move::Before(end));
                None
            }
            ListAction::MoveBelowPlaying => {
                let lists = self.library().lists();
                let below = (lists.playing == Some(album))
                    .then(|| lists.albums[album].current)
                    .flatten()?;
                self.library_mut()
                    .move_selected(album, Move::Before(below + 1));
                None
            }
            _ => None,
        }
    }

    /// Adds paths to an album (once each when "Do not add duplicate
    /// files" is on); durations are found in the background.
    pub(crate) fn add_paths(&mut self, album: usize, paths: &[String]) -> Option<usize> {
        let paths: Vec<String> = if self.settings.playlist.no_duplicates {
            let known: std::collections::BTreeSet<String> = self
                .library()
                .lists()
                .albums
                .get(album)
                .map(|target| target.entries.iter().map(|e| e.path.clone()).collect())
                .unwrap_or_default();
            paths
                .iter()
                .filter(|path| !known.contains(&crate::playlist::local_path(path)))
                .cloned()
                .collect()
        } else {
            paths.to_vec()
        };
        let first = self.library_mut().insert(album, None, &paths);
        if self.settings.playlist.detect_durations {
            let local: Vec<String> = paths
                .iter()
                .filter(|path| !is_url(path))
                .map(|path| crate::playlist::local_path(path))
                .collect();
            self.detect_durations(&local);
        }
        first
    }

    /// A playlist dialog's answer (`kind` as in [`UiRequest::ListDialog`]).
    pub fn list_dialog_answer(
        &mut self,
        engine: &Engine,
        kind: &str,
        text: &str,
    ) -> Option<UiRequest> {
        let album = self.library().lists().shown;
        let targets = self.library().shown().targets();
        match kind {
            "title" => {
                let first = targets.first().copied()?;
                self.library_mut().set_title(album, &[first], Some(text));
                None
            }
            "search" => {
                let found = self.library_mut().select_matching(album, text);
                match found {
                    Some(index) => {
                        self.library_mut().reveal(album, index, 10);
                        None
                    }
                    None => Some(UiRequest::Osd(format!("Not found: {text}"))),
                }
            }
            "url" => {
                let items: Vec<String> = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect();
                self.add_paths(album, &items);
                None
            }
            "rename" => self.rename_entry(engine, text),
            "open" => match crate::playlist_files::read(Path::new(text), &self.drive_map()) {
                Ok(entries) => {
                    let paths: Vec<String> =
                        entries.iter().map(|entry| entry.path.clone()).collect();
                    let first = self.add_paths(album, &paths)?;
                    for (offset, entry) in entries.iter().enumerate() {
                        if entry.title.is_some() {
                            self.library_mut().set_title(
                                album,
                                &[first + offset],
                                entry.title.as_deref(),
                            );
                        }
                    }
                    None
                }
                Err(error) => Some(UiRequest::Message(error)),
            },
            "save" => {
                let path = Path::new(text);
                let Some(format) = crate::playlist_files::Format::of(path) else {
                    return Some(UiRequest::Message(format!(
                        "{text}: save as .m3u8, .m3u, .pls or .dpl"
                    )));
                };
                let body = crate::playlist_files::write(format, &self.library().shown().entries);
                Some(match crate::storage::write_atomic(path, body.as_bytes()) {
                    Ok(()) => UiRequest::Osd(format!("Saved {}", crate::view::file_name(text))),
                    Err(error) => UiRequest::Message(format!("Cannot save {text}: {error}")),
                })
            }
            "move" | "move-related" | "copy" | "copy-related" => {
                let operation = if kind.starts_with("move") {
                    FileOperation::Move
                } else {
                    FileOperation::Copy
                };
                let related = kind.ends_with("related");
                self.start_file_job(&targets, Path::new(text), operation, related)
            }
            _ => None,
        }
    }

    /// Renames the selected entry's file (in its folder) and the entries.
    fn rename_entry(&mut self, engine: &Engine, name: &str) -> Option<UiRequest> {
        let index = self.library().shown().targets().first().copied()?;
        let old = self.library().shown().entries.get(index)?.path.clone();
        let name = name.trim();
        if name.is_empty() || name.contains(['/', '\\']) {
            return Some(UiRequest::Message(format!("{name:?} is not a file name")));
        }
        let new = Path::new(&old).with_file_name(name);
        if new.exists() {
            return Some(UiRequest::Message(format!(
                "{} is there already",
                new.display()
            )));
        }
        let playing = self.model.loaded_path() == Some(old.as_str());
        if playing {
            // An open file cannot be renamed on Windows.
            drop(engine.stop());
        }
        match std::fs::rename(&old, &new) {
            Ok(()) => {
                let new = new.to_string_lossy().into_owned();
                self.library_mut().replace_path(&old, &new);
                None
            }
            Err(error) => Some(UiRequest::Message(format!("Cannot rename {old}: {error}"))),
        }
    }

    /// Asks to send files (and with `related` their subtitles and other
    /// files of the same name) to the Recycle Bin.
    pub(crate) fn recycle_request(paths: &[String], related: bool) -> Option<UiRequest> {
        let mut files: Vec<PathBuf> = Vec::new();
        for path in paths.iter().filter(|path| !is_url(path)) {
            let file = PathBuf::from(path);
            if related {
                files.extend(crate::sources::related_files(&file));
            }
            files.push(file);
        }
        files.retain(|file| file.exists());
        (!files.is_empty()).then(|| UiRequest::ConfirmRecycle {
            files,
            entries: paths.to_vec(),
        })
    }

    /// The user confirmed the Recycle Bin: playback of these files stops
    /// first (Windows cannot delete an open file).
    pub fn prepare_recycle(&mut self, engine: &Engine, files: &[PathBuf]) {
        let playing = self.model.loaded_path().map(PathBuf::from);
        if playing.is_some_and(|playing| files.contains(&playing)) {
            drop(engine.stop());
        }
    }

    /// The shell sent files to the Recycle Bin: their entries go.
    pub fn recycled(&mut self, entries: &[String]) {
        self.library_mut().remove_paths(entries);
    }

    /// Starts moving or copying the selected entries' files.
    fn start_file_job(
        &mut self,
        targets: &[usize],
        folder: &Path,
        operation: FileOperation,
        related: bool,
    ) -> Option<UiRequest> {
        if !folder.is_dir() {
            return Some(UiRequest::Message(format!(
                "{} is not a folder",
                folder.display()
            )));
        }
        let mut files = Vec::new();
        for &index in targets {
            let Some(entry) = self.library().shown().entries.get(index) else {
                continue;
            };
            if is_url(&entry.path) {
                continue;
            }
            let file = PathBuf::from(&entry.path);
            if related {
                files.extend(crate::sources::related_files(&file));
            }
            files.push(file);
        }
        if files.is_empty() {
            return None;
        }
        let (sender, receiver) = mpsc::channel();
        let folder = folder.to_path_buf();
        let started = std::thread::Builder::new()
            .name("rvp-files".to_owned())
            .spawn(move || {
                drop(sender.send(run_file_job(&files, &folder, operation)));
            });
        match started {
            Ok(_) => {
                self.file_jobs.results.push(receiver);
                Some(UiRequest::Osd("Working on the files...".to_owned()))
            }
            Err(error) => Some(UiRequest::Message(format!("Cannot start: {error}"))),
        }
    }

    /// About once a second: finished file jobs update the albums.
    pub fn file_job_done(&mut self) -> Vec<UiRequest> {
        let mut requests = Vec::new();
        let mut reports = Vec::new();
        self.file_jobs
            .results
            .retain(|receiver| match receiver.try_recv() {
                Ok(report) => {
                    reports.push(report);
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                Err(mpsc::TryRecvError::Disconnected) => false,
            });
        for report in reports {
            for (old, new) in &report.moved {
                self.library_mut().replace_path(old, new);
            }
            let mut text = match (report.moved.len(), report.copied) {
                (0, copied) => format!("Copied {copied} files"),
                (moved, _) => format!("Moved {moved} files"),
            };
            if report.errors.is_empty() {
                requests.push(UiRequest::Osd(text));
            } else {
                text = format!("{text}; {}", report.errors.join("; "));
                requests.push(UiRequest::Message(text));
            }
        }
        requests
    }

    /// File information for an entry of the shown album: its file and,
    /// for the one that plays, its tracks.
    fn file_info(&self, engine: &Engine, index: usize) -> String {
        let Some(entry) = self.library().shown().entries.get(index) else {
            return String::new();
        };
        let mut lines = vec![
            format!("Title: {}", entry.label()),
            format!("Path: {}", entry.path),
        ];
        if let Some(duration) = entry.duration {
            lines.push(format!("Duration: {}", crate::view::format_time(duration)));
        }
        if !is_url(&entry.path)
            && let Ok(metadata) = std::fs::metadata(&entry.path)
        {
            lines.push(format!("Size: {} bytes", metadata.len()));
        }
        if self.model.loaded_path() == Some(entry.path.as_str()) {
            for track in engine.tracks() {
                lines.push(format!(
                    "{:?} track {}: {}",
                    track.kind,
                    track.id,
                    crate::commands::track_label(&track)
                ));
            }
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, title: Option<&str>, duration: Option<f64>) -> Entry {
        let mut entry = Entry::new(path);
        entry.title = title.map(str::to_owned);
        entry.duration = duration;
        entry
    }

    #[test]
    fn sorting_by_fields_is_stable() {
        let entries = vec![
            entry("/b/z.mp3", Some("Gamma (2001)"), Some(30.0)),
            entry("/a/y.mkv", Some("alpha 1999"), None),
            entry("/a/x.mkv", Some("Beta"), Some(10.0)),
        ];
        assert_eq!(sorted_order(&entries, SortField::Title), [1, 2, 0]);
        assert_eq!(sorted_order(&entries, SortField::Name), [2, 1, 0]);
        assert_eq!(sorted_order(&entries, SortField::Extension), [2, 1, 0]);
        assert_eq!(sorted_order(&entries, SortField::Duration), [2, 0, 1]);
        assert_eq!(sorted_order(&entries, SortField::Folder), [2, 1, 0]);
        assert_eq!(sorted_order(&entries, SortField::Release), [1, 0, 2]);
    }

    #[test]
    fn years_are_found_alone() {
        assert_eq!(release_year("Film (2001)"), Some(2001));
        assert_eq!(release_year("1999.mkv"), Some(1999));
        assert_eq!(release_year("12001"), None);
        assert_eq!(release_year("3000 BC"), None);
    }

    #[test]
    fn file_jobs_move_and_copy() {
        let dir = std::env::temp_dir().join(format!("rvp-jobs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (from, to) = (dir.join("from"), dir.join("to"));
        std::fs::create_dir_all(&from).expect("from");
        std::fs::create_dir_all(&to).expect("to");
        std::fs::write(from.join("a.mkv"), b"a").expect("a");
        std::fs::write(from.join("b.mkv"), b"b").expect("b");
        std::fs::write(to.join("b.mkv"), b"there").expect("b there");
        let report = run_file_job(
            &[from.join("a.mkv"), from.join("b.mkv")],
            &to,
            FileOperation::Copy,
        );
        assert_eq!(report.copied, 1);
        assert_eq!(report.errors.len(), 1, "{report:?}");
        std::fs::remove_file(to.join("a.mkv")).expect("copied");
        let report = run_file_job(&[from.join("a.mkv")], &to, FileOperation::Move);
        assert_eq!(report.moved.len(), 1);
        assert!(!from.join("a.mkv").exists() && to.join("a.mkv").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
