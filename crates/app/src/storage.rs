//! Where RVP keeps its files, and crash-safe writes (D9).
//!
//! Every file is written to a temporary file in the same directory, flushed
//! to disk and renamed over the old one, so a crash leaves either the old or
//! the new version, never a partial file. [`Writer`] does this on its own
//! thread: flushing to disk can take tens of milliseconds, and on the GUI
//! thread during playback that showed as dropped video frames.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;

/// The configuration directory (settings) and the state directory (resume
/// store, window geometry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub config: PathBuf,
    pub state: PathBuf,
}

impl Dirs {
    /// `RVP_CONFIG_DIR` and `RVP_STATE_DIR` override the platform defaults:
    /// on Linux `$XDG_CONFIG_HOME/rvp` (`~/.config/rvp`) and
    /// `$XDG_STATE_HOME/rvp` (`~/.local/state/rvp`), on Windows
    /// `%APPDATA%\RVP` for both.
    ///
    /// # Errors
    /// Fails when neither the overrides nor the platform variables are set.
    pub fn from_env() -> Result<Self, String> {
        let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let (config, state) = if cfg!(windows) {
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
        let config = var("RVP_CONFIG_DIR").map(PathBuf::from).or(config);
        let state = var("RVP_STATE_DIR").map(PathBuf::from).or(state);
        match (config, state) {
            (Some(config), Some(state)) => Ok(Self { config, state }),
            _ => {
                Err("no configuration directory (set RVP_CONFIG_DIR and RVP_STATE_DIR)".to_owned())
            }
        }
    }

    /// Both directories inside `dir` (tests, portable mode).
    #[must_use]
    pub fn inside(dir: &Path) -> Self {
        Self {
            config: dir.join("config"),
            state: dir.join("state"),
        }
    }
}

/// Writes `bytes` to `path` atomically: temporary file, flush to disk,
/// rename over the old file, then flush the directory entry (Linux).
///
/// # Errors
/// Fails on any I/O error; the old file is then left as it was.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path)?;
    if !cfg!(windows) {
        // Make the rename itself durable; directories cannot be opened for
        // this on Windows, where the rename is already flushed.
        fs::File::open(dir)?.sync_all()?;
    }
    Ok(())
}

/// Reads a JSON file. A missing file is `Ok(None)`; a damaged one is moved
/// aside to `<name>.bad` and also `Ok(None)`, so a bad file never stops RVP.
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
        fs::rename(path, PathBuf::from(bad))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temporary() {
        let dir = scratch("atomic");
        let path = dir.join("a.json");
        write_atomic(&path, b"one").expect("first write");
        write_atomic(&path, b"two").expect("second write");
        assert_eq!(fs::read(&path).expect("read"), b"two");
        assert!(!dir.join("a.json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
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
        let _ = fs::remove_dir_all(&dir);
    }
}
