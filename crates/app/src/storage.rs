//! Where RVP keeps its files, and crash-safe writes (D9).
//!
//! Every file is written to a temporary file in the same directory, flushed
//! to disk and renamed over the old one, so a crash leaves either the old or
//! the new version, never a partial file. [`Writer`] does this on its own
//! thread: flushing to disk can take tens of milliseconds, and on the GUI
//! thread during playback that showed as dropped video frames.
//!
//! * Each write has a temporary file of its own (`.<name>.<pid>.<n>.tmp`,
//!   created only when no file of that name is there), so several RVP
//!   processes, or threads, writing one file never share one; a failed
//!   write removes its temporary file.
//! * A file that is a symbolic link stays one: the new version goes beside
//!   the file the link points to.
//! * On Windows a rename over a file another program holds open (a virus
//!   scanner, an indexer) fails for a moment: it is tried again a few
//!   times.
//! * Files only this process writes and mpv reads (the VapourSynth scripts,
//!   the subtitle working copies) carry the process's ID in their names
//!   ([`process_file_name`]) and are removed when RVP closes.
//!
//! # Several RVPs on one folder
//!
//! Several RVP processes may run on one settings and state folder
//! ("Multiple instances", `rvp /new`, New Player). Each reads `state.json`,
//! `marks.json`, `playlists.json` and `settings.json` once, at start, and
//! does not take the others' changes while it runs (no watching, no
//! reloading). But one instance never writes another's changes back over:
//! it writes only its own changes onto the newest copy on disk, so nothing
//! either saves is lost and the next start sees all of it. A file is
//! written so once [`Writer::merge_into`] names its rule ([`Merge`]):
//!
//! * The write takes the lock file `.<name>.lock` beside the file
//!   ([`fs::File::lock`], held for that write only: the file itself is
//!   replaced by a rename, so it cannot hold a lock), reads the file again,
//!   puts onto it what this instance changed since its last write (its
//!   content then, the base, against its content now) and writes the result
//!   atomically. Afterwards this instance's own content is the base, not
//!   the merged file: what it has not changed since is never written again,
//!   so its old values never replace the other's newer ones.
//! * While the file is still the one this instance read at start or last
//!   wrote whole, no one else wrote it: the content is written whole, as a
//!   single RVP always wrote it. A missing or damaged file is written whole
//!   too.
//! * A lock that is not free within [`LOCK_WAIT`] (another RVP stalled in
//!   its write, a stalled network folder) does not hold the write up: it
//!   goes on without the lock, merged all the same, and the log says so;
//!   where the file system has no such locks, writes go without them. A
//!   failed write keeps the base, so the next one carries the change again.
//!
//! What counts as one change, file by file (the same one changed in both
//! instances: the last write wins; a removal is a change too, so the other
//! instance's older copy never brings back what this one removed):
//!
//! * `state.json` ([`crate::resume`]) by entry: each file's position, audio
//!   track and subtitle tracks, each skin mode's window; the last file, the
//!   volume, mute, the window and the other single values each alone.
//! * `marks.json` ([`crate::marks`]) by entry: each file's bookmarks and A-B
//!   list together, each favorite by its path; the recent items opened here
//!   since go before the disk's, less those removed here.
//! * `settings.json` ([`crate::presets`]) by setting: each
//!   leaf value this instance changed, added or removed (a key binding is
//!   one; a list's item while the list keeps its length, else the list).
//!   The presets' list by preset name: a preset added, removed or renamed
//!   here (a rename removes the old name) is applied, one changed here gets
//!   the settings changed in it (its conditions and values), one untouched
//!   here is the disk's. The active preset's name and the page's options
//!   are settings like the others, and so are the default preset's values,
//!   the file's top level.
//! * `playlists.json` ([`crate::playlist`]) by album, known by its name (and
//!   its place among albums of the same name, so two albums given one name
//!   in two instances are one album here): an album this instance added,
//!   removed or renamed (the old name removed) is applied, one whose name
//!   or entries it edited is written whole, one it did not touch is the
//!   disk's. The selection, the scroll position and the playback marks
//!   (the current entry, played, durations, the queue) are each instance's
//!   own view: changed alone, they are written while the other instance
//!   left that album's entries as they were, and give way otherwise. The
//!   shown and the playing album are this instance's when it changed them,
//!   else the disk's. Undo and redo stay with each instance.
//! * A list the user orders (albums, presets, favorites) takes the order of
//!   the instance that reordered it; the other's new items go after their
//!   neighbours there.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

/// The configuration directory (settings) and the state directory (resume
/// store, window geometry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub config: PathBuf,
    pub state: PathBuf,
    /// Portable mode: both next to the executable.
    pub portable: bool,
}

/// The file next to the executable that turns on portable mode.
pub const PORTABLE_MARKER: &str = "rvp.portable";

impl Dirs {
    /// `RVP_CONFIG_DIR` and `RVP_STATE_DIR` override the others. Then, when
    /// [`PORTABLE_MARKER`] is next to the executable, portable mode keeps
    /// `settings.json` there and the state in its `state` folder. Otherwise
    /// the platform defaults: on Linux `$XDG_CONFIG_HOME/rvp`
    /// (`~/.config/rvp`) and `$XDG_STATE_HOME/rvp` (`~/.local/state/rvp`),
    /// on Windows `%APPDATA%\RVP` for both.
    ///
    /// # Errors
    /// Fails when no rule gives both directories.
    pub fn from_env() -> Result<Self, String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        Self::resolve(
            |name| std::env::var_os(name),
            exe_dir.as_deref(),
            cfg!(windows),
        )
    }

    /// [`Dirs::from_env`] with the environment, the executable's folder and
    /// the system given (tests).
    ///
    /// # Errors
    /// Fails when no rule gives both directories.
    pub fn resolve(
        var: impl Fn(&str) -> Option<std::ffi::OsString>,
        exe_dir: Option<&Path>,
        windows: bool,
    ) -> Result<Self, String> {
        let var = |name: &str| var(name).filter(|value| !value.is_empty());
        let portable = exe_dir.filter(|dir| dir.join(PORTABLE_MARKER).is_file());
        let (config, state) = if let Some(dir) = portable {
            (Some(dir.to_path_buf()), Some(dir.join("state")))
        } else if windows {
            let base = var("APPDATA").map(|dir| PathBuf::from(dir).join("RVP"));
            (base.clone(), base)
        } else {
            let home = var("HOME").map(PathBuf::from);
            let config = var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| home.as_ref().map(|h| h.join(".config")))
                .map(|dir| dir.join("rvp"));
            let state = var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .or_else(|| home.as_ref().map(|h| h.join(".local/state")))
                .map(|dir| dir.join("rvp"));
            (config, state)
        };
        let config_override = var("RVP_CONFIG_DIR").map(PathBuf::from);
        let state_override = var("RVP_STATE_DIR").map(PathBuf::from);
        let portable = portable.is_some() && config_override.is_none();
        match (config_override.or(config), state_override.or(state)) {
            (Some(config), Some(state)) => Ok(Self {
                config,
                state,
                portable,
            }),
            _ => {
                Err("no configuration directory (set RVP_CONFIG_DIR and RVP_STATE_DIR)".to_owned())
            }
        }
    }

    /// Both directories inside `dir` (tests).
    #[must_use]
    pub fn inside(dir: &Path) -> Self {
        Self {
            config: dir.join("config"),
            state: dir.join("state"),
            portable: false,
        }
    }
}

/// Writes `bytes` to `path` atomically: temporary file, flush to disk,
/// rename over the old file, then flush the directory entry (Linux).
///
/// # Errors
/// Fails on any I/O error; the old file is then left as it was.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace(path, bytes, true, false)
}

/// [`write_atomic`] for a file only its owner may read where the system
/// has such modes (Unix: 0600; on Windows the state folder is the user's
/// own).
///
/// # Errors
/// As [`write_atomic`].
pub fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace(path, bytes, true, true)
}

/// Writes `bytes` to `path` through a temporary file and a rename as
/// [`write_atomic`] does, without the flush to disk: for files written on
/// the window's thread, which waits for no disk (subtitles, the scripts
/// mpv reads), where the newest version may be lost in a crash.
///
/// # Errors
/// As [`write_atomic`].
pub fn write_unflushed(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace(path, bytes, false, false)
}

/// `<stem>-<pid>.<extension>`: the name of a file only this process writes
/// and reads, so that several RVPs on one state folder keep theirs apart.
#[must_use]
pub fn process_file_name(stem: &str, extension: &str) -> String {
    format!("{stem}-{}.{extension}", std::process::id())
}

/// How many names a temporary file tries: a name is taken only by a file
/// a process of the same ID left behind.
const TEMPORARY_NAMES: u32 = 64;

/// How many times a rename is tried while Windows reports the target in
/// use, and the first wait (doubled each time: 150 ms in all).
const RENAME_TRIES: u32 = 5;
const RENAME_WAIT: Duration = Duration::from_millis(10);

/// The temporary files' numbers in this process.
static TEMPORARY_NUMBER: AtomicU64 = AtomicU64::new(0);

fn replace(path: &Path, bytes: &[u8], flush: bool, private: bool) -> io::Result<()> {
    let path = link_target(path);
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    fs::create_dir_all(dir)?;
    let (temporary, mut file) = create_temporary(dir, &path, private)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| if flush { file.sync_all() } else { Ok(()) });
    drop(file);
    if let Err(error) = written.and_then(|()| rename_retrying(&temporary, &path)) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if flush && !cfg!(windows) {
        // Make the rename itself durable; directories cannot be opened for
        // this on Windows, where the rename is already flushed.
        fs::File::open(dir)?.sync_all()?;
    }
    Ok(())
}

/// The file a write replaces: a symbolic link's target, so that the link
/// stays (and the file it names gets the new content).
fn link_target(path: &Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    }
}

/// A new temporary file beside `path`, under a name no file has (a name
/// that is taken, by a file or a link, is never opened).
fn create_temporary(dir: &Path, path: &Path, private: bool) -> io::Result<(PathBuf, fs::File)> {
    // The start of the file's name says whose a leftover is; long names
    // are cut so the temporary one stays within the system's limit.
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut end = name.len().min(64);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        if private {
            options.mode(0o600);
        }
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut taken = None;
    for _ in 0..TEMPORARY_NAMES {
        let number = TEMPORARY_NUMBER.fetch_add(1, Ordering::Relaxed);
        let temporary = dir.join(format!(
            ".{}.{}.{number}.tmp",
            &name[..end],
            std::process::id()
        ));
        match options.open(&temporary) {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => taken = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(taken.unwrap_or_else(|| io::ErrorKind::AlreadyExists.into()))
}

fn rename_retrying(from: &Path, to: &Path) -> io::Result<()> {
    retry(|| fs::rename(from, to), RENAME_TRIES, in_use)
}

/// Runs `step` until it succeeds, fails for another reason than
/// `passing`, or ran `tries` times; the waits between double.
fn retry<T>(
    mut step: impl FnMut() -> io::Result<T>,
    tries: u32,
    passing: impl Fn(&io::Error) -> bool,
) -> io::Result<T> {
    let mut wait = RENAME_WAIT;
    let mut attempt = 1;
    loop {
        match step() {
            Err(error) if attempt < tries && passing(&error) => {
                std::thread::sleep(wait);
                wait *= 2;
                attempt += 1;
            }
            result => return result,
        }
    }
}

/// Windows' errors for a file another program holds open: access denied,
/// a sharing or a lock violation.
fn in_use(error: &io::Error) -> bool {
    cfg!(windows) && matches!(error.raw_os_error(), Some(5 | 32 | 33))
}

/// Reads a JSON file. A missing file is `Ok(None)`; a damaged one is moved
/// aside to `<name>.bad` when it can be and also `Ok(None)`, so a bad file
/// never stops RVP.
///
/// # Errors
/// Fails on I/O errors other than a missing file.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    Ok(load_json(path)?.map(|(value, _)| value))
}

/// [`read_json`] with the bytes the value came from (for
/// [`Writer::merge_into`]).
///
/// # Errors
/// Fails on I/O errors other than a missing file.
pub fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<Option<(T, Vec<u8>)>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if let Ok(value) = serde_json::from_slice(&bytes) {
        Ok(Some((value, bytes)))
    } else {
        let mut bad = path.as_os_str().to_owned();
        bad.push(".bad");
        // A file that cannot be moved (another RVP moved it first, the
        // folder is read-only, the name is taken) is no reason to stop:
        // the next save replaces it.
        let _ = fs::rename(path, PathBuf::from(bad));
        Ok(None)
    }
}

/// Writes a value as pretty JSON, atomically.
///
/// # Errors
/// Fails on serialization or I/O errors.
pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    write_atomic(path, &json_bytes(value)?)
}

/// A value as RVP's JSON files hold it: pretty, with a final new line.
///
/// # Errors
/// Fails when the value cannot be serialized.
pub fn json_bytes<T: serde::Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// A file's rule for several RVPs (module comment): the file on disk now
/// (`disk`) with this instance's changes put onto it, which are its
/// content now (`ours`) against its content at its last write (`base`).
/// `None` when `disk` is no such file (damaged): `ours` is written whole.
pub type Merge = fn(disk: &[u8], base: &[u8], ours: &[u8]) -> Option<Vec<u8>>;

/// How long a write waits for the lock another RVP holds while it writes
/// the same file. Such a write takes milliseconds; a lock held this long
/// belongs to a process or a disk that is stuck.
pub const LOCK_WAIT: Duration = Duration::from_secs(2);

/// A file several RVPs write ([`Writer::merge_into`]).
struct Merging {
    merge: Merge,
    /// This instance's content when it last wrote the file (or read it).
    base: Vec<u8>,
    /// The file as this instance read it or last wrote it whole: while the
    /// file is still that, it holds no one else's changes.
    own: Option<Vec<u8>>,
}

#[derive(Default)]
struct WriterState {
    /// The newest content waiting for each file.
    pending: BTreeMap<PathBuf, Vec<u8>>,
    /// The files written by merging, and how.
    merging: BTreeMap<PathBuf, Merging>,
    /// A write is in progress.
    busy: bool,
    /// The last write error, until it is taken.
    error: Option<String>,
    closed: bool,
}

type Shared = Arc<(Mutex<WriterState>, Condvar)>;

/// Writes files atomically ([`write_atomic`]) on a background thread. Only
/// the newest pending content of each file is written; dropping the writer
/// finishes what is pending. A file several RVPs share is merged first
/// ([`Writer::merge_into`]).
pub struct Writer {
    shared: Shared,
    thread: Option<JoinHandle<()>>,
}

fn lock(shared: &Shared) -> std::sync::MutexGuard<'_, WriterState> {
    shared.0.lock().unwrap_or_else(PoisonError::into_inner)
}

fn writer_loop(shared: &Shared) {
    let mut state = lock(shared);
    loop {
        if let Some(path) = state.pending.keys().next().cloned() {
            let bytes = state.pending.remove(&path).unwrap_or_default();
            let mut merging = state.merging.remove(&path);
            state.busy = true;
            drop(state);
            let result = match merging.as_mut() {
                Some(merging) => write_merged(&path, &bytes, merging),
                None => write_atomic(&path, &bytes),
            };
            state = lock(shared);
            if let Some(merging) = merging {
                state.merging.entry(path.clone()).or_insert(merging);
            }
            state.busy = false;
            if let Err(error) = result {
                state.error = Some(format!("{}: {error}", path.display()));
            }
            shared.1.notify_all();
        } else if state.closed {
            return;
        } else {
            state = shared.1.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Writes `ours` to a file several RVPs share: under its lock, onto the
/// file as it is now (module comment).
fn write_merged(path: &Path, ours: &[u8], merging: &mut Merging) -> io::Result<()> {
    let _lock = take_lock(path);
    let read = retry(|| fs::read(path), RENAME_TRIES, in_use);
    let merged = match read {
        Ok(disk) if merging.own.as_deref() != Some(disk.as_slice()) => {
            (merging.merge)(&disk, &merging.base, ours)
        }
        Ok(_) => None,
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let written = merged.as_deref().unwrap_or(ours);
    write_atomic(path, written)?;
    merging.own = (written == ours).then(|| ours.to_vec());
    ours.clone_into(&mut merging.base);
    Ok(())
}

/// The lock file of a shared file: `.<name>.lock` beside it.
fn lock_path(path: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(path.file_name().unwrap_or_default());
    name.push(".lock");
    path.with_file_name(name)
}

/// Takes the lock of a shared file, waiting at most [`LOCK_WAIT`] for
/// another RVP's write. `None` when the write goes on without it: the
/// wait ran out (logged), or the folder or its file system gives no lock.
/// The lock is released when the file is dropped.
fn take_lock(path: &Path) -> Option<fs::File> {
    let lock = lock_path(path);
    if let Some(dir) = lock.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .ok()?;
    let deadline = Instant::now() + LOCK_WAIT;
    let mut wait = Duration::from_millis(1);
    loop {
        match file.try_lock() {
            Ok(()) => return Some(file),
            Err(fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(wait);
                wait = (wait * 2).min(Duration::from_millis(50));
            }
            Err(fs::TryLockError::WouldBlock) => {
                crate::log::report(
                    crate::log::Level::Warning,
                    &format!(
                        "{} stayed locked by another RVP for {} s: written without the lock",
                        path.display(),
                        LOCK_WAIT.as_secs()
                    ),
                );
                return None;
            }
            Err(fs::TryLockError::Error(_)) => return None,
        }
    }
}

/// `ours` when this instance changed the value (it differs from `base`),
/// else the disk's.
pub(crate) fn pick<T: PartialEq + Clone>(disk: T, base: &T, ours: &T) -> T {
    if ours == base { disk } else { ours.clone() }
}

/// A map merged by entry: the entries this instance added, changed or
/// removed (against `base`) put onto `disk`; the rest stay the disk's.
pub(crate) fn merge_map<K: Ord + Clone, V: PartialEq + Clone>(
    mut disk: BTreeMap<K, V>,
    base: &BTreeMap<K, V>,
    ours: &BTreeMap<K, V>,
) -> BTreeMap<K, V> {
    for key in base.keys().filter(|key| !ours.contains_key(*key)) {
        disk.remove(key);
    }
    for (key, value) in ours {
        if base.get(key) != Some(value) {
            disk.insert(key.clone(), value.clone());
        }
    }
    disk
}

/// A list merged by item, each known by its `key` (a key that comes twice
/// by its place among them): an item this instance added or removed
/// (against `base`) is added or removed on `disk`, one it changed is
/// `item(disk's, base's, ours)` (or `ours` again when the other instance
/// removed it), and the rest stay the disk's, the other's new items too.
/// The order is the disk's with this instance's new items after their
/// neighbours, or this instance's with the other's new items so, when this
/// instance reordered the items both have.
pub(crate) fn merge_list<T: PartialEq + Clone, K: Ord + Clone>(
    disk: Vec<T>,
    base: &[T],
    ours: &[T],
    key: impl Fn(&T) -> K,
    item: impl Fn(T, &T, &T) -> T,
) -> Vec<T> {
    if ours == base {
        return disk;
    }
    let ids = |list: &[T]| -> Vec<(K, usize)> {
        let mut seen: BTreeMap<K, usize> = BTreeMap::new();
        list.iter()
            .map(|value| {
                let key = key(value);
                let count = seen.entry(key.clone()).or_default();
                *count += 1;
                (key, *count - 1)
            })
            .collect()
    };
    let (base_ids, ours_ids, disk_ids) = (ids(base), ids(ours), ids(&disk));
    let in_base: BTreeMap<&(K, usize), &T> = base_ids.iter().zip(base).collect();
    let in_ours: BTreeSet<&(K, usize)> = ours_ids.iter().collect();
    let mut on_disk: BTreeMap<&(K, usize), T> = disk_ids.iter().zip(disk).collect();
    let mut merged: BTreeMap<&(K, usize), T> = BTreeMap::new();
    for (id, value) in ours_ids.iter().zip(ours) {
        let disk = on_disk.remove(id);
        let kept = match in_base.get(id) {
            // Untouched here: the disk's, or gone with the other's removal.
            Some(known) if *known == value => disk,
            Some(known) => Some(match disk {
                Some(disk) => item(disk, known, value),
                None => value.clone(),
            }),
            None => Some(value.clone()),
        };
        if let Some(kept) = kept {
            merged.insert(id, kept);
        }
    }
    // What is left on disk and not in `base` is the other's new; what is
    // in `base` was removed here.
    merged.extend(
        on_disk
            .into_iter()
            .filter(|(id, _)| !in_base.contains_key(id)),
    );
    let both = |ids: &[(K, usize)]| -> Vec<(K, usize)> {
        ids.iter()
            .filter(|id| in_base.contains_key(id) && in_ours.contains(id))
            .cloned()
            .collect()
    };
    let (frame, others) = if both(&ours_ids) == both(&base_ids) {
        (&disk_ids, &ours_ids)
    } else {
        (&ours_ids, &disk_ids)
    };
    let mut order: Vec<&(K, usize)> = frame.iter().filter(|id| merged.contains_key(id)).collect();
    for (at, id) in others.iter().enumerate() {
        if !merged.contains_key(id) || order.contains(&id) {
            continue;
        }
        let after = others[..at]
            .iter()
            .rev()
            .find_map(|before| order.iter().position(|placed| *placed == before));
        order.insert(after.map_or(0, |place| place + 1), id);
    }
    order
        .into_iter()
        .filter_map(|id| merged.remove(id))
        .collect()
}

/// JSON merged by key, all the way down: the keys this instance added,
/// changed or removed (against `base`) put onto `disk`. A list of one
/// length on all three sides merges item by item (the settings window
/// names `seek-seconds.0`); any other value that differs (a number, a list
/// that grew) is one value.
pub(crate) fn merge_value(disk: Value, base: &Value, ours: &Value) -> Value {
    if ours == base {
        return disk;
    }
    match (disk, base, ours) {
        (Value::Object(mut disk), Value::Object(base), Value::Object(ours)) => {
            disk.retain(|key, _| !base.contains_key(key) || ours.contains_key(key));
            for (key, value) in ours {
                match (base.get(key), disk.get_mut(key)) {
                    (Some(known), _) if known == value => {}
                    (Some(known), Some(slot)) => *slot = merge_value(slot.take(), known, value),
                    _ => {
                        disk.insert(key.clone(), value.clone());
                    }
                }
            }
            Value::Object(disk)
        }
        (Value::Array(disk), Value::Array(base), Value::Array(ours))
            if disk.len() == base.len() && base.len() == ours.len() =>
        {
            Value::Array(
                disk.into_iter()
                    .zip(base.iter().zip(ours))
                    .map(|(disk, (base, ours))| merge_value(disk, base, ours))
                    .collect(),
            )
        }
        _ => ours.clone(),
    }
}

/// A [`Merge`] for a JSON file of `T`: `merge(disk, base, ours)` on the
/// parsed values, written as [`json_bytes`] writes them. `None` when one of
/// them is no such file.
pub(crate) fn merge_json<T: serde::Serialize + serde::de::DeserializeOwned>(
    disk: &[u8],
    base: &[u8],
    ours: &[u8],
    merge: impl FnOnce(T, &T, &T) -> T,
) -> Option<Vec<u8>> {
    let disk = serde_json::from_slice(disk).ok()?;
    let base = serde_json::from_slice(base).ok()?;
    let ours = serde_json::from_slice(ours).ok()?;
    json_bytes(&merge(disk, &base, &ours)).ok()
}

impl Writer {
    /// Starts the writer thread.
    ///
    /// # Errors
    /// Fails when the thread cannot be started.
    pub fn new() -> io::Result<Self> {
        let shared: Shared = Arc::new((Mutex::new(WriterState::default()), Condvar::new()));
        let worker = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("rvp-writer".to_owned())
            .spawn(move || writer_loop(&worker))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    /// Writes to `path` from now on put this instance's changes onto the
    /// file as it is then, by `merge` (module comment). `read` is the file
    /// as this instance read it (`None`: missing or damaged), `base` this
    /// instance's content then, in the form its writes take.
    pub fn merge_into(&self, path: &Path, read: Option<Vec<u8>>, base: Vec<u8>, merge: Merge) {
        lock(&self.shared).merging.insert(
            path.to_owned(),
            Merging {
                merge,
                base,
                own: read,
            },
        );
    }

    /// Queues `bytes` for `path`, replacing content still waiting for it.
    pub fn write(&self, path: &Path, bytes: Vec<u8>) {
        lock(&self.shared).pending.insert(path.to_owned(), bytes);
        self.shared.1.notify_all();
    }

    /// Waits until everything queued so far is on disk.
    pub fn flush(&self) {
        let mut state = lock(&self.shared);
        while !state.pending.is_empty() || state.busy {
            state = self
                .shared
                .1
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// The last write error, once.
    #[must_use]
    pub fn take_error(&self) -> Option<String> {
        lock(&self.shared).error.take()
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        lock(&self.shared).closed = true;
        self.shared.1.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl std::fmt::Debug for Writer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Writer").finish_non_exhaustive()
    }
}

/// A user folder of the desktop's for RVP's own files: on Linux the one
/// `user-dirs.dirs` names (`key`, such as `XDG_PICTURES_DIR`: the user's
/// language's, `~/Изображения`), else `~/<english>` (Windows:
/// `%USERPROFILE%\\<english>`). The owner's answer to the Linux audit's
/// L15 (2026-11-14).
#[must_use]
pub fn user_folder(key: &str, english: &str) -> PathBuf {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map_or_else(std::env::temp_dir, PathBuf::from);
    if cfg!(unix) {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        if let Ok(text) = fs::read_to_string(config.join("user-dirs.dirs"))
            && let Some(folder) = user_dirs_entry(&text, key, &home)
        {
            return folder;
        }
    }
    home.join(english)
}

/// `key`'s folder in a `user-dirs.dirs` text (`XDG_VIDEOS_DIR="$HOME/Видео"`);
/// `None` when it is missing, relative, or the home folder itself (which
/// xdg-user-dirs writes for "none").
fn user_dirs_entry(text: &str, key: &str, home: &Path) -> Option<PathBuf> {
    let value = text.lines().find_map(|line| {
        line.trim()
            .strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })?;
    let value = value.trim().strip_prefix('"')?.strip_suffix('"')?;
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(value),
    };
    (path.is_absolute() && path != home).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // user-dirs.dirs is Linux's: its paths are Unix paths.
    #[cfg(unix)]
    #[test]
    fn user_folders_come_from_user_dirs() {
        let text = "# written by xdg-user-dirs-update\nXDG_DESKTOP_DIR=\"$HOME/Рабочий стол\"\nXDG_VIDEOS_DIR=\"$HOME/Видео\"\nXDG_MUSIC_DIR=\"$HOME/\"\nXDG_PICTURES_DIR=\"/data/Фото\"\n";
        let home = Path::new("/home/u");
        assert_eq!(
            user_dirs_entry(text, "XDG_VIDEOS_DIR", home),
            Some(PathBuf::from("/home/u/Видео"))
        );
        assert_eq!(
            user_dirs_entry(text, "XDG_PICTURES_DIR", home),
            Some(PathBuf::from("/data/Фото"))
        );
        // The home folder itself means none; a missing key too.
        assert_eq!(user_dirs_entry(text, "XDG_MUSIC_DIR", home), None);
        assert_eq!(user_dirs_entry(text, "XDG_TEMPLATES_DIR", home), None);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    /// The names in `dir`, sorted.
    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("list")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temporary() {
        let dir = scratch("atomic");
        let path = dir.join("a.json");
        write_atomic(&path, b"one").expect("first write");
        write_atomic(&path, b"two").expect("second write");
        write_unflushed(&path, b"three").expect("third write");
        assert_eq!(fs::read(&path).expect("read"), b"three");
        assert_eq!(names(&dir), ["a.json"]);
        // A failed write leaves its temporary file behind neither: a
        // directory where the file should be.
        fs::create_dir_all(dir.join("b.json").join("inside")).expect("blocker");
        assert!(write_atomic(&dir.join("b.json"), b"x").is_err());
        assert_eq!(names(&dir), ["a.json", "b.json"]);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Several processes (here threads) write one file at once: each
    /// write has its own temporary file, so none fails and the file is
    /// always one whole version.
    #[test]
    fn writers_of_one_file_do_not_share_a_temporary_file() {
        let dir = scratch("writers");
        let path = dir.join("state.json");
        let writers: Vec<_> = (0..6u8)
            .map(|writer| {
                let path = path.clone();
                std::thread::spawn(move || {
                    // Different lengths: a shorter version written over a
                    // longer one would keep the longer one's tail.
                    let bytes = vec![b'a' + writer; 4096 * (usize::from(writer) + 1)];
                    for _ in 0..20 {
                        write_atomic(&path, &bytes)?;
                        let read = fs::read(&path)?;
                        let first = read.first().copied().unwrap_or_default();
                        let whole = first >= b'a'
                            && read.len() == 4096 * usize::from(first - b'a' + 1)
                            && read.iter().all(|byte| *byte == first);
                        if !whole {
                            return Err(io::Error::other(format!(
                                "a mixed file of {} bytes",
                                read.len()
                            )));
                        }
                    }
                    Ok::<(), io::Error>(())
                })
            })
            .collect();
        for writer in writers {
            writer.join().expect("writer").expect("writes");
        }
        assert_eq!(names(&dir), ["state.json"]);
        let _ = fs::remove_dir_all(&dir);
    }

    // Links need a privilege on Windows.
    #[cfg(unix)]
    #[test]
    fn a_linked_file_stays_a_link() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = scratch("link");
        let real = dir.join("real");
        fs::create_dir_all(&real).expect("dir");
        write_atomic(&real.join("settings.json"), b"old").expect("write");
        let link = dir.join("settings.json");
        std::os::unix::fs::symlink(real.join("settings.json"), &link).expect("link");
        write_atomic(&link, b"new").expect("write through the link");
        assert!(
            fs::symlink_metadata(&link)
                .expect("meta")
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(real.join("settings.json")).expect("read"), b"new");
        assert_eq!(names(&dir), ["real", "settings.json"]);
        assert_eq!(names(&real), ["settings.json"]);
        // The owner-only kind.
        write_private(&dir.join("token"), b"secret").expect("private");
        let mode = fs::metadata(dir.join("token"))
            .expect("meta")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_step_is_tried_again_only_while_its_error_passes() {
        let mut calls = 0;
        let busy = || io::Error::from(io::ErrorKind::ResourceBusy);
        let result = retry(
            || {
                calls += 1;
                if calls < 3 { Err(busy()) } else { Ok(()) }
            },
            5,
            |error| error.kind() == io::ErrorKind::ResourceBusy,
        );
        assert!(result.is_ok());
        assert_eq!(calls, 3);
        // Another error, or too many tries: the error.
        let mut calls = 0;
        let result: io::Result<()> = retry(
            || {
                calls += 1;
                Err(io::Error::from(io::ErrorKind::NotFound))
            },
            5,
            |error| error.kind() == io::ErrorKind::ResourceBusy,
        );
        assert_eq!(
            result.map_err(|error| error.kind()),
            Err(io::ErrorKind::NotFound)
        );
        assert_eq!(calls, 1);
        let mut calls = 0;
        assert!(
            retry::<()>(
                || {
                    calls += 1;
                    Err(busy())
                },
                5,
                |error| error.kind() == io::ErrorKind::ResourceBusy,
            )
            .is_err()
        );
        assert_eq!(calls, 5);
        // Windows' sharing errors pass there, and nothing elsewhere.
        assert_eq!(in_use(&io::Error::from_raw_os_error(32)), cfg!(windows));
        assert!(!in_use(&io::Error::from(io::ErrorKind::NotFound)));
    }

    #[test]
    fn writer_keeps_the_newest_content_and_reports_errors() {
        let dir = scratch("writer");
        let path = dir.join("state.json");
        let writer = Writer::new().expect("writer");
        for round in 0..50 {
            writer.write(&path, format!("{round}").into_bytes());
        }
        writer.flush();
        assert_eq!(fs::read(&path).expect("read"), b"49");
        assert_eq!(writer.take_error(), None);
        // A directory where the file should be: the write fails, the error
        // is kept until taken.
        fs::create_dir_all(dir.join("blocked.json")).expect("blocker");
        writer.write(&dir.join("blocked.json"), b"x".to_vec());
        writer.flush();
        assert!(writer.take_error().is_some());
        assert_eq!(writer.take_error(), None);
        drop(writer);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_json_is_moved_aside() {
        let dir = scratch("damaged");
        let path = dir.join("state.json");
        write_atomic(&path, b"{ not json").expect("write");
        let value: Option<serde_json::Value> = read_json(&path).expect("read");
        assert!(value.is_none());
        assert!(dir.join("state.json.bad").exists());
        assert!(!path.exists());
        let missing: Option<serde_json::Value> = read_json(&dir.join("none.json")).expect("read");
        assert!(missing.is_none());
        // A damaged file that cannot be moved aside (its `.bad` name is a
        // folder with something in it) does not stop the start either.
        write_atomic(&path, b"{ damaged again").expect("write");
        fs::remove_file(dir.join("state.json.bad")).expect("remove");
        fs::create_dir_all(dir.join("state.json.bad").join("inside")).expect("blocker");
        let value: Option<serde_json::Value> = read_json(&path).expect("read");
        assert!(value.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn maps_values_and_lists_merge_by_entry() {
        let map = |pairs: &[(&str, u32)]| -> BTreeMap<String, u32> {
            pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
        };
        // Here: a changed, b removed, d added; there: c added, e changed.
        let base = map(&[("a", 1), ("b", 2), ("e", 5)]);
        let ours = map(&[("a", 10), ("d", 4), ("e", 5)]);
        let disk = map(&[("a", 1), ("b", 2), ("c", 3), ("e", 50)]);
        assert_eq!(
            merge_map(disk, &base, &ours),
            map(&[("a", 10), ("c", 3), ("d", 4), ("e", 50)])
        );
        assert_eq!(pick(1, &2, &2), 1);
        assert_eq!(pick(1, &2, &3), 3);

        let base = serde_json::json!({ "a": 1, "b": { "c": 2, "d": 3 }, "l": [1, 2], "s": [1, 2], "gone": 0 });
        let ours = serde_json::json!({ "a": 1, "b": { "c": 20, "d": 3 }, "l": [1], "s": [10, 2], "new": 7 });
        let disk = serde_json::json!({ "a": 9, "b": { "c": 2, "d": 30 }, "l": [1, 2, 3], "s": [1, 20], "gone": 0, "x": 1 });
        assert_eq!(
            merge_value(disk, &base, &ours),
            serde_json::json!({ "a": 9, "b": { "c": 20, "d": 30 }, "l": [1], "s": [10, 20], "new": 7, "x": 1 })
        );

        // Items by key: here "b" changed, "c" removed, "n" added after "a";
        // there "a" changed, "b" changed, "t" added at the end.
        let item = |key: &str, value: u32| (key.to_owned(), value);
        let base = [item("a", 1), item("b", 2), item("c", 3)];
        let ours = [item("a", 1), item("n", 9), item("b", 20)];
        let disk = vec![item("a", 10), item("b", 21), item("c", 3), item("t", 5)];
        let merged = merge_list(
            disk.clone(),
            &base,
            &ours,
            |(key, _)| key.clone(),
            |_, _, ours| ours.clone(),
        );
        assert_eq!(
            merged,
            [item("a", 10), item("n", 9), item("b", 20), item("t", 5)]
        );
        // Untouched here: the disk's as it is.
        assert_eq!(
            merge_list(
                disk.clone(),
                &base,
                &base,
                |(key, _)| key.clone(),
                |d, _, _| d
            ),
            disk
        );
        // Reordered here: this order, the other's new item after its
        // neighbour; an item changed here that the other removed is back.
        let ours = [item("c", 30), item("b", 2), item("a", 1)];
        let disk = vec![item("a", 1), item("t", 5), item("b", 2)];
        let merged = merge_list(
            disk,
            &base,
            &ours,
            |(key, _)| key.clone(),
            |_, _, ours| ours.clone(),
        );
        assert_eq!(
            merged,
            [item("c", 30), item("b", 2), item("a", 1), item("t", 5)]
        );
    }

    /// The numbers' file of the tests below: a map merged by entry.
    fn merge_numbers(disk: &[u8], base: &[u8], ours: &[u8]) -> Option<Vec<u8>> {
        merge_json(
            disk,
            base,
            ours,
            |disk: BTreeMap<String, u32>, base, ours| merge_map(disk, base, ours),
        )
    }

    fn numbers(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    fn read_numbers(path: &Path) -> BTreeMap<String, u32> {
        read_json(path).expect("read").expect("there")
    }

    /// A writer as an RVP that read `path` at start sets it up.
    fn instance(path: &Path) -> (Writer, BTreeMap<String, u32>) {
        let (start, read) = match load_json(path).expect("read") {
            Some((start, bytes)) => (start, Some(bytes)),
            None => (BTreeMap::new(), None),
        };
        let writer = Writer::new().expect("writer");
        writer.merge_into(
            path,
            read,
            json_bytes(&start).expect("bytes"),
            merge_numbers,
        );
        (writer, start)
    }

    fn save(writer: &Writer, path: &Path, numbers: &BTreeMap<String, u32>) {
        writer.write(path, json_bytes(numbers).expect("bytes"));
        writer.flush();
        assert_eq!(writer.take_error(), None);
    }

    /// Two RVPs on one file: each writes its own changes onto the other's,
    /// in either order, and a removal stays removed.
    #[test]
    fn two_instances_keep_each_others_changes() {
        let dir = scratch("instances");
        let path = dir.join("numbers.json");
        // Not in RVP's form: the first write of one instance alone is its
        // content whole, as before.
        fs::create_dir_all(&dir).expect("dir");
        fs::write(&path, br#"{"a":1,"b":2,"old":0}"#).expect("write");
        let (one, mut mine) = instance(&path);
        let (two, mut theirs) = instance(&path);
        mine.insert("a".into(), 10);
        mine.remove("b");
        save(&one, &path, &mine);
        assert_eq!(
            fs::read(&path).expect("read"),
            json_bytes(&mine).expect("bytes")
        );
        theirs.insert("c".into(), 3);
        save(&two, &path, &theirs);
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 10), ("c", 3), ("old", 0)])
        );
        mine.insert("d".into(), 4);
        save(&one, &path, &mine);
        // The same entry changed in both: the last write wins.
        theirs.insert("a".into(), 20);
        theirs.remove("old");
        save(&two, &path, &theirs);
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 20), ("c", 3), ("d", 4)])
        );
        mine.insert("e".into(), 5);
        save(&one, &path, &mine);
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 20), ("c", 3), ("d", 4), ("e", 5)])
        );
        // One alone again (the other stopped): still merged, as the file
        // holds the other's changes.
        mine.insert("f".into(), 6);
        save(&one, &path, &mine);
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 20), ("c", 3), ("d", 4), ("e", 5), ("f", 6)])
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_damaged_file_takes_the_whole_copy() {
        let dir = scratch("whole");
        let path = dir.join("numbers.json");
        fs::create_dir_all(&dir).expect("dir");
        fs::write(&path, br#"{"a":1}"#).expect("write");
        let (writer, mut mine) = instance(&path);
        let (other, mut theirs) = instance(&path);
        theirs.insert("t".into(), 1);
        save(&other, &path, &theirs);
        fs::write(&path, b"{ damaged").expect("damage");
        mine.insert("b".into(), 2);
        save(&writer, &path, &mine);
        assert_eq!(
            fs::read(&path).expect("read"),
            json_bytes(&mine).expect("bytes")
        );
        fs::remove_file(&path).expect("remove");
        mine.insert("c".into(), 3);
        save(&writer, &path, &mine);
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 1), ("b", 2), ("c", 3)])
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Another RVP's write holds the lock: the write waits for it and goes
    /// onto what that one wrote; a lock never freed delays it by
    /// [`LOCK_WAIT`] and no more.
    #[test]
    fn a_held_lock_delays_the_write_and_loses_nothing() {
        let dir = scratch("lock");
        let path = dir.join("numbers.json");
        fs::create_dir_all(&dir).expect("dir");
        fs::write(&path, br#"{"a":1}"#).expect("write");
        let (writer, mut mine) = instance(&path);
        let held = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path(&path))
            .expect("lock file");
        held.lock().expect("lock");
        let other = {
            let path = path.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                // Written at once, with no flush: a flush on a busy disk
                // can outlast the wait for the lock, and the test is about
                // a lock that is freed in time.
                fs::write(&path, br#"{"a":1,"t":9}"#).expect("write");
                drop(held);
            })
        };
        let start = Instant::now();
        mine.insert("b".into(), 2);
        save(&writer, &path, &mine);
        assert!(start.elapsed() >= Duration::from_millis(300));
        other.join().expect("other");
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 1), ("b", 2), ("t", 9)])
        );

        let held = fs::File::open(lock_path(&path)).expect("lock file");
        held.lock().expect("lock");
        let start = Instant::now();
        mine.insert("c".into(), 3);
        save(&writer, &path, &mine);
        let waited = start.elapsed();
        // The upper bound only says the wait ends: the write after it
        // flushes, which a busy disk makes slow.
        assert!(waited >= LOCK_WAIT && waited < LOCK_WAIT * 10, "{waited:?}");
        assert_eq!(
            read_numbers(&path),
            numbers(&[("a", 1), ("b", 2), ("c", 3), ("t", 9)])
        );
        drop(held);
        let _ = fs::remove_dir_all(&dir);
    }
}
