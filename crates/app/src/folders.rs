//! Opening folders. A folder brings its media files and its subfolders',
//! found on a thread of their own, so that a big tree, a slow network
//! folder or one that links back into itself never holds the window's
//! thread up. The walk reads each entry's own type and looks at what a
//! link names before it follows it; it walks every folder once, by its
//! real path (a linked folder whose real path was walked is left out),
//! goes at most [`DEPTH`] levels down, and over one request stops after
//! [`ENTRIES`] entries or [`FILES`] media files, with one notice that the
//! folder was cut short.
//!
//! What a walk found opens when it ends ([`App::folder_tick`], about ten
//! times a second), as the files would have opened at once. Requests open
//! in the order they came: one that comes while a walk is under way waits
//! for it, and one that plays (not only adds) drops the walks still under
//! way, since what the user chose last plays.

use crate::{App, UiRequest};
use rvp_engine::Engine;
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

/// Levels of subfolders a walk goes down.
pub const DEPTH: usize = 8;
/// Folder entries one request reads, media files or not: a few times a
/// large media library.
pub const ENTRIES: usize = 200_000;
/// Media files one request takes.
pub const FILES: usize = 20_000;

/// File extensions a folder's walk picks up.
const MEDIA_EXTENSIONS: &[&str] = &[
    "3gp", "aac", "ac3", "aiff", "ape", "asf", "avi", "avs", "dts", "flac", "flv", "m2ts", "m4a",
    "m4v", "mka", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "mts", "ogg", "ogm", "ogv", "opus",
    "rm", "rmvb", "ts", "vob", "wav", "webm", "wma", "wmv", "wv",
];

/// One item of a request: a folder to walk, or a file or address that
/// opens as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Item {
    Folder(PathBuf),
    File(String),
}

/// What becomes of the files a walk found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Then {
    /// [`App::open`]'s: they join the shown album and the first plays, or
    /// with `append` they are only added.
    Open { append: bool },
    /// "Add folder": they are added to the shown album.
    Add,
}

/// What a walk found.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Found {
    /// The request's files and addresses in its order, each folder's media
    /// files in its place, sorted by path.
    pub files: Vec<String>,
    /// The request's first folder (the notice when none had media files).
    pub first_folder: Option<PathBuf>,
    /// The folder at which a limit was reached.
    pub cut: Option<PathBuf>,
}

/// How far one request's walk goes (tests take small limits).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    pub depth: usize,
    pub entries: usize,
    pub files: usize,
}

impl Limits {
    pub(crate) const DEFAULT: Self = Self {
        depth: DEPTH,
        entries: ENTRIES,
        files: FILES,
    };
}

/// The folder walks under way, in the order they were asked for.
#[derive(Debug, Default)]
pub(crate) struct Walks {
    queue: VecDeque<Walking>,
}

#[derive(Debug)]
struct Walking {
    then: Then,
    receiver: mpsc::Receiver<Found>,
    stop: Arc<AtomicBool>,
}

impl Walks {
    /// Whether a request waits for its walk.
    pub(crate) fn walking(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Drops the requests under way and stops their walks.
    pub(crate) fn drop_all(&mut self) {
        for walking in self.queue.drain(..) {
            walking.stop.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for Walks {
    fn drop(&mut self) {
        self.drop_all();
    }
}

/// The request's items with each folder's media files in its place.
pub(crate) fn walk(items: &[Item], limits: Limits, stop: &AtomicBool) -> Found {
    let mut walker = Walker {
        limits,
        stop,
        entries: 0,
        files: 0,
        visited: BTreeSet::new(),
        cut: false,
    };
    let mut found = Found::default();
    for item in items {
        match item {
            Item::File(file) => found.files.push(file.clone()),
            Item::Folder(folder) => {
                found.first_folder.get_or_insert_with(|| folder.clone());
                if walker.cut {
                    continue;
                }
                let real = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.clone());
                let mut files = Vec::new();
                walker.visit(folder, &real, 0, &mut files);
                files.sort();
                found.files.extend(files);
                if walker.cut {
                    found.cut = Some(folder.clone());
                }
            }
        }
    }
    found
}

struct Walker<'a> {
    limits: Limits,
    stop: &'a AtomicBool,
    /// Entries read so far, over the whole request.
    entries: usize,
    /// Media files taken so far.
    files: usize,
    /// The real paths of the folders walked.
    visited: BTreeSet<PathBuf>,
    /// A limit was reached: the walk ends.
    cut: bool,
}

impl Walker<'_> {
    /// Takes the media files of `folder`, whose real path is `real`, and of
    /// its subfolders.
    fn visit(&mut self, folder: &Path, real: &Path, depth: usize, files: &mut Vec<String>) {
        if !self.visited.insert(real.to_path_buf()) {
            return;
        }
        let Ok(entries) = std::fs::read_dir(folder) else {
            return;
        };
        for entry in entries {
            if self.cut || self.stop.load(Ordering::Relaxed) {
                return;
            }
            self.entries += 1;
            if self.entries > self.limits.entries {
                self.cut = true;
                return;
            }
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            // A plain folder's real path is its parent's and its name; a
            // link's is looked up (it may lead anywhere, back here too).
            let (is_folder, is_file, real_path) = if kind.is_symlink() {
                match std::fs::metadata(&path) {
                    Ok(named) if named.is_dir() => (true, false, std::fs::canonicalize(&path).ok()),
                    Ok(named) => (false, named.is_file(), None),
                    Err(_) => continue,
                }
            } else {
                (
                    kind.is_dir(),
                    kind.is_file(),
                    Some(real.join(entry.file_name())),
                )
            };
            if is_folder {
                if depth < self.limits.depth
                    && let Some(real_path) = real_path
                {
                    self.visit(&path, &real_path, depth + 1, files);
                }
            } else if is_file && is_media(&path) {
                if self.files == self.limits.files {
                    self.cut = true;
                    return;
                }
                self.files += 1;
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
}

fn is_media(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        MEDIA_EXTENSIONS
            .iter()
            .any(|known| ext.eq_ignore_ascii_case(known))
    })
}

/// Walks `items` on a thread of its own; with no folder among them, or
/// no thread to be had, here (within the same limits).
fn start(items: &[Item], stop: &Arc<AtomicBool>) -> mpsc::Receiver<Found> {
    let (sender, receiver) = mpsc::channel();
    if items.iter().any(|item| matches!(item, Item::Folder(_))) {
        let (worker, worker_stop, worker_sender) =
            (items.to_vec(), Arc::clone(stop), sender.clone());
        let started = std::thread::Builder::new()
            .name("rvp-folders".to_owned())
            .spawn(move || {
                drop(worker_sender.send(walk(&worker, Limits::DEFAULT, &worker_stop)));
            });
        if started.is_ok() {
            return receiver;
        }
    }
    drop(sender.send(walk(items, Limits::DEFAULT, stop)));
    receiver
}

impl App {
    /// Finds the media files of `items`' folders, then does `then` with
    /// them ([`App::folder_tick`]).
    pub(crate) fn walk_folders(&mut self, items: &[Item], then: Then) {
        let stop = Arc::new(AtomicBool::new(false));
        let receiver = start(items, &stop);
        self.folders.queue.push_back(Walking {
            then,
            receiver,
            stop,
        });
    }

    /// Opens the media files in `folder` and its subfolders (PotPlayer's
    /// "Open Folder"): they join the shown album and the first one plays,
    /// once they are found ([`App::folder_tick`]).
    pub fn open_folder(&mut self, folder: &Path) {
        self.folders.drop_all();
        self.walk_folders(
            &[Item::Folder(folder.to_path_buf())],
            Then::Open { append: false },
        );
    }

    /// Adds the media files in `folder` and its subfolders to the shown
    /// album, sorted by path, once they are found ([`App::folder_tick`]).
    pub fn add_folder(&mut self, folder: &Path) {
        self.walk_folders(&[Item::Folder(folder.to_path_buf())], Then::Add);
    }

    /// Whether opened folders are still being read.
    #[must_use]
    pub fn walking_folders(&self) -> bool {
        self.folders.walking()
    }

    /// About ten times a second: the requests whose walks ended, in their
    /// order, open what they found. `None` when none ended.
    pub fn folder_tick(&mut self, engine: &Engine) -> Option<Vec<UiRequest>> {
        let mut requests = None::<Vec<UiRequest>>;
        while let Some(walking) = self.folders.queue.front() {
            let found = match walking.receiver.try_recv() {
                Ok(found) => Some(found),
                Err(mpsc::TryRecvError::Empty) => break,
                // The walk's thread ended without an answer.
                Err(mpsc::TryRecvError::Disconnected) => None,
            };
            let then = walking.then;
            self.folders.queue.pop_front();
            let done = requests.get_or_insert_with(Vec::new);
            if let Some(found) = found {
                done.extend(self.open_found_files(engine, then, &found));
            }
        }
        requests
    }

    fn open_found_files(&mut self, engine: &Engine, then: Then, found: &Found) -> Vec<UiRequest> {
        let mut requests = Vec::new();
        if let Some(folder) = &found.cut {
            requests.push(UiRequest::Message(format!(
                "{} holds more than RVP reads at once ({FILES} media files, {ENTRIES} \
                 entries): the rest of it was left out",
                folder.display()
            )));
        }
        if found.files.is_empty() {
            if let Some(folder) = &found.first_folder {
                requests.push(UiRequest::Message(format!(
                    "No media files in {}",
                    folder.display()
                )));
            }
            return requests;
        }
        match then {
            Then::Open { append } => {
                if let Err(error) = self.open_files(engine, &found.files, append) {
                    requests.push(UiRequest::Message(format!("Cannot open: {error}")));
                }
            }
            Then::Add => {
                let shown = self.library().lists().shown;
                self.library_mut().insert(shown, None, &found.files);
            }
        }
        requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder of the test's own.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-folders-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("folder");
        dir
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("folder");
        }
        std::fs::write(path, b"").expect("file");
    }

    fn text(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    fn walked(items: &[Item], limits: Limits) -> Found {
        walk(items, limits, &AtomicBool::new(false))
    }

    #[test]
    fn folders_bring_their_media_files_in_place() {
        let dir = folder("place");
        touch(&dir.join("b.mkv"));
        touch(&dir.join("a.MP4"));
        touch(&dir.join("notes.txt"));
        touch(&dir.join("sub").join("c.flac"));
        let found = walked(
            &[
                Item::File("first.mkv".to_owned()),
                Item::Folder(dir.clone()),
                Item::File("https://example.org/last".to_owned()),
            ],
            Limits::DEFAULT,
        );
        assert_eq!(
            found.files,
            [
                "first.mkv".to_owned(),
                text(&dir.join("a.MP4")),
                text(&dir.join("b.mkv")),
                text(&dir.join("sub").join("c.flac")),
                "https://example.org/last".to_owned(),
            ]
        );
        assert_eq!(found.first_folder, Some(dir.clone()));
        assert_eq!(found.cut, None);
        // Eight levels down at most.
        let mut deep = dir.join("deep");
        for level in 0..10 {
            touch(&deep.join(format!("level{level}.mkv")));
            deep = deep.join("next");
        }
        let found = walked(&[Item::Folder(dir.join("deep"))], Limits::DEFAULT);
        assert_eq!(found.files.len(), DEPTH + 1, "{:?}", found.files);
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn a_walk_stops_at_its_limits() {
        let dir = folder("limits");
        for index in 0..10 {
            touch(&dir.join(format!("{index}.mkv")));
            touch(&dir.join(format!("{index}.txt")));
        }
        let limits = |entries, files| Limits {
            depth: DEPTH,
            entries,
            files,
        };
        // Exactly at the limits: nothing is cut.
        let found = walked(&[Item::Folder(dir.clone())], limits(20, 10));
        assert_eq!((found.files.len(), found.cut.as_ref()), (10, None));
        // One file over.
        let found = walked(&[Item::Folder(dir.clone())], limits(20, 9));
        assert_eq!((found.files.len(), found.cut.as_ref()), (9, Some(&dir)));
        // One entry over.
        let found = walked(&[Item::Folder(dir.clone())], limits(19, 10));
        assert_eq!(found.cut.as_ref(), Some(&dir));
        assert!(found.files.len() <= 10);
        // The limits hold over the whole request: a second folder is left
        // out once the first reached one, its files and addresses are not.
        let other = folder("limits-other");
        touch(&other.join("x.mkv"));
        let found = walked(
            &[
                Item::Folder(dir.clone()),
                Item::Folder(other.clone()),
                Item::File("y.mkv".to_owned()),
            ],
            limits(100, 5),
        );
        assert_eq!(found.files.len(), 6);
        assert_eq!(found.files.last().map(String::as_str), Some("y.mkv"));
        assert_eq!(found.cut.as_ref(), Some(&dir));
        // A stopped walk ends at once.
        let found = walk(
            &[Item::Folder(dir.clone())],
            Limits::DEFAULT,
            &AtomicBool::new(true),
        );
        assert!(found.files.is_empty());
        drop(std::fs::remove_dir_all(&dir));
        drop(std::fs::remove_dir_all(&other));
    }

    #[cfg(unix)]
    #[test]
    fn linked_folders_are_walked_once() {
        let dir = folder("links");
        touch(&dir.join("a.mkv"));
        touch(&dir.join("sub").join("b.mkv"));
        let outside = folder("links-outside");
        touch(&outside.join("c.mkv"));
        // A link to the folder itself, one back to it from below, a second
        // link to a folder walked already, and one to a folder elsewhere.
        std::os::unix::fs::symlink(&dir, dir.join("self")).expect("link");
        std::os::unix::fs::symlink("..", dir.join("sub").join("up")).expect("link");
        std::os::unix::fs::symlink(dir.join("sub"), dir.join("same")).expect("link");
        std::os::unix::fs::symlink(&outside, dir.join("there")).expect("link");
        // A link to a file counts as the file; one that leads nowhere is
        // left out.
        std::os::unix::fs::symlink(outside.join("c.mkv"), dir.join("d.mkv")).expect("link");
        std::os::unix::fs::symlink(dir.join("missing.mkv"), dir.join("e.mkv")).expect("link");
        let found = walked(&[Item::Folder(dir.clone())], Limits::DEFAULT);
        let names: Vec<String> = found
            .files
            .iter()
            .map(|file| {
                Path::new(file)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            ["a.mkv", "b.mkv", "c.mkv", "d.mkv"],
            "{:?}",
            found.files
        );
        assert_eq!(found.cut, None);
        // Linked to the root: walked once, never through its links again.
        let found = walked(&[Item::Folder(dir.join("self"))], Limits::DEFAULT);
        assert_eq!(found.files.len(), 4, "{:?}", found.files);
        drop(std::fs::remove_dir_all(&dir));
        drop(std::fs::remove_dir_all(&outside));
    }

    /// A headless player with nothing to restore.
    fn player(name: &str) -> (App, Engine, PathBuf) {
        let dir = folder(&format!("app-{name}"));
        let engine = Engine::new(&rvp_engine::EngineOptions::headless()).expect("engine");
        let settings = crate::Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            ..crate::Settings::default()
        };
        let app = App::new(settings, &dir.join("state")).expect("app");
        (app, engine, dir)
    }

    /// Ticks until every request opened; what they asked of the window.
    fn wait(app: &mut App, engine: &Engine) -> Vec<UiRequest> {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut requests = Vec::new();
        while std::time::Instant::now() < end {
            requests.extend(app.folder_tick(engine).unwrap_or_default());
            if !app.walking_folders() {
                return requests;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the walk did not end");
    }

    fn paths(app: &App) -> Vec<String> {
        app.library()
            .shown()
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect()
    }

    #[test]
    fn opened_folders_open_when_their_walk_ends() {
        let (mut app, engine, dir) = player("open");
        let media = dir.join("media");
        touch(&media.join("2.mkv"));
        touch(&media.join("1.mkv"));
        app.open(&engine, &[text(&media)], false).expect("open");
        // Nothing was read on this thread.
        assert!(app.walking_folders());
        assert!(paths(&app).is_empty());
        let requests = wait(&mut app, &engine);
        assert!(requests.is_empty(), "{requests:?}");
        assert_eq!(
            paths(&app),
            [
                crate::playlist::local_path(&text(&media.join("1.mkv"))),
                crate::playlist::local_path(&text(&media.join("2.mkv"))),
            ]
        );

        // A folder without media files says so and leaves the album alone.
        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).expect("folder");
        app.open_folder(&empty);
        let requests = wait(&mut app, &engine);
        assert!(
            matches!(&requests[..], [UiRequest::Message(text)] if text.starts_with("No media files")),
            "{requests:?}"
        );
        assert_eq!(paths(&app).len(), 2);

        // "Add folder" adds; a file appended meanwhile waits its turn.
        let more = dir.join("more");
        touch(&more.join("3.mkv"));
        let fourth = crate::playlist::local_path(&text(&dir.join("4.mkv")));
        app.add_folder(&more);
        app.open(&engine, std::slice::from_ref(&fourth), true)
            .expect("open");
        assert_eq!(paths(&app).len(), 2);
        wait(&mut app, &engine);
        let listed = paths(&app);
        assert_eq!(listed.len(), 4, "{listed:?}");
        assert!(listed[2].ends_with("3.mkv"), "{listed:?}");
        assert_eq!(listed[3], fourth);

        // A file that plays drops a walk still to open.
        let fifth = crate::playlist::local_path(&text(&dir.join("5.mkv")));
        app.open_folder(&more);
        app.open(&engine, std::slice::from_ref(&fifth), false)
            .expect("open");
        assert!(!app.walking_folders());
        assert!(paths(&app).contains(&fifth));

        // A walk cut short says so once, and what it found is added.
        let cut = Found {
            files: vec![text(&more.join("3.mkv"))],
            first_folder: Some(more.clone()),
            cut: Some(more.clone()),
        };
        let requests = app.open_found_files(&engine, Then::Add, &cut);
        assert!(
            matches!(&requests[..], [UiRequest::Message(text)] if text.contains("left out")),
            "{requests:?}"
        );
        assert!(
            paths(&app)
                .last()
                .is_some_and(|path| path.ends_with("3.mkv"))
        );
        drop(engine);
        drop(app);
        drop(std::fs::remove_dir_all(&dir));
    }
}
