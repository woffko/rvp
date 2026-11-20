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

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

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
fn retry(
    mut step: impl FnMut() -> io::Result<()>,
    tries: u32,
    passing: impl Fn(&io::Error) -> bool,
) -> io::Result<()> {
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
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if let Ok(value) = serde_json::from_slice(&bytes) {
        Ok(Some(value))
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
    let mut bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}

#[derive(Default)]
struct WriterState {
    /// The newest content waiting for each file.
    pending: BTreeMap<PathBuf, Vec<u8>>,
    /// A write is in progress.
    busy: bool,
    /// The last write error, until it is taken.
    error: Option<String>,
    closed: bool,
}

type Shared = Arc<(Mutex<WriterState>, Condvar)>;

/// Writes files atomically ([`write_atomic`]) on a background thread. Only
/// the newest pending content of each file is written; dropping the writer
/// finishes what is pending.
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
            state.busy = true;
            drop(state);
            let result = write_atomic(&path, &bytes);
            state = lock(shared);
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
        let result = retry(
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
            retry(
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
}
