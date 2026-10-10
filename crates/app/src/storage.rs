//! Where RVP keeps its files, and crash-safe writes (D9).
//!
//! Every file is written to a temporary file in the same directory, flushed
//! to disk and renamed over the old one, so a crash leaves either the old or
//! the new version, never a partial file.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
