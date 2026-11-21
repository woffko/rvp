//! The skin a process loads, marked in the state folder: a skin that
//! stops RVP while it loads or draws its first frame is not loaded again at
//! the next start.
//!
//! A skin that fails to load with an error leaves the skin shown and the
//! setting as they were (`App::skin_failed`). What no error path catches (an
//! abort, a stack overflow, the memory running out, a hang the user ends)
//! would otherwise come back at every start once the skin is the setting.
//!
//! * Each RVP process keeps a marker of its own, `skin-load-<pid>.json` in
//!   the state folder, from its start to its end, locked
//!   ([`fs::File::try_lock`]) the whole time: the lock tells the other RVPs
//!   on the folder that it runs. Before a skin file loads for the first time
//!   in the process, the marker names the skin; once the skin has drawn its
//!   first frame, or its load failed with an error, it names none again. A
//!   clean exit removes the marker ([`Marker::remove`]).
//! * At start, before the skin loads, RVP reads the markers no running RVP
//!   holds (their lock is free: their processes ended) and removes them
//!   ([`scan`]). One that still names a skin is a load that never finished:
//!   when that skin is the configured one, RVP starts with RVP Default,
//!   clears the setting and says so (`App::skip_unfinished_skins`); the
//!   skin stays in the Skins menu's and page's list for the session, so it
//!   can be chosen again on purpose.
//! * A second `rvp` that hands its files to a running one loads the skin
//!   too, from its start on (it cannot know yet), and may end in the middle
//!   of the load. Its marker counts only when no other RVP held a marker on
//!   the folder at its start (`alone`), or once its process had a window of
//!   its own (`window`): a second `rvp` always finds the first one's.
//! * The marker is a few bytes rewritten in place, without waiting for the
//!   disk (no flush): written as a skin starts to load, which reads far more
//!   from the disk on the same thread, and kept by the system when the
//!   process ends. A folder whose file system has no locks gets no marker.

use std::fs;
use std::io::{self, Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A marker's file name: `skin-load-<pid>.json`.
const PREFIX: &str = "skin-load-";
const SUFFIX: &str = ".json";

/// What a marker holds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Content {
    /// The skin loading, or shown and not drawn yet.
    skin: Option<String>,
    /// No other RVP held a marker on the folder when this one started.
    alone: bool,
    /// The process has a window of its own (it hands no files over).
    window: bool,
}

impl Content {
    /// The skin of a marker its process left behind, when it counts.
    fn unfinished(self) -> Option<PathBuf> {
        (self.alone || self.window)
            .then_some(self.skin)
            .flatten()
            .map(PathBuf::from)
    }
}

/// What a start finds in the state folder ([`scan`]).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Found {
    /// The skins whose load or first frame never finished, by the markers
    /// of RVPs that ended.
    pub unfinished: Vec<PathBuf>,
    /// Another RVP runs on the folder: its marker is held.
    pub others: bool,
}

/// Whether `path` is a marker's file.
fn is_marker(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(PREFIX))
        .and_then(|rest| rest.strip_suffix(SUFFIX))
        .is_some_and(|pid| !pid.is_empty() && pid.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Reads the markers in `state`. Those whose lock is free belong to RVPs
/// that ended: they are removed, and the skins of those that count are
/// [`Found::unfinished`]. A marker a running RVP holds stays.
#[must_use]
pub fn scan(state: &Path) -> Found {
    let mut found = Found::default();
    let Ok(entries) = fs::read_dir(state) else {
        return found;
    };
    let mut markers: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_marker(path))
        .collect();
    markers.sort();
    for path in markers {
        let Ok(mut file) = fs::File::open(&path) else {
            continue;
        };
        match file.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => {
                found.others = true;
                continue;
            }
            // No locks here: nothing to tell from the file.
            Err(fs::TryLockError::Error(_)) => continue,
        }
        let mut text = String::new();
        let content = file
            .read_to_string(&mut text)
            .ok()
            .and_then(|_| serde_json::from_str::<Content>(&text).ok())
            .unwrap_or_default();
        // Removed while this start holds it: no other start reads it again
        // (Windows removes it once it is closed).
        drop(fs::remove_file(&path));
        if let Some(skin) = content.unfinished()
            && !found.unfinished.contains(&skin)
        {
            found.unfinished.push(skin);
        }
    }
    found
}

/// This process's marker, held (locked) from the start to a clean exit.
/// Dropped without [`Marker::remove`], it stays on disk as a process that
/// ended leaves it.
#[derive(Debug)]
pub struct Marker {
    path: PathBuf,
    file: fs::File,
    content: Content,
}

impl Marker {
    /// The marker of the process `pid` in `state`, made after [`scan`]:
    /// `alone` when it found no other RVP.
    ///
    /// # Errors
    /// Fails when the folder cannot hold the file or its file system gives
    /// no lock; no marker is left then.
    pub fn create(state: &Path, pid: u32, alone: bool) -> io::Result<Self> {
        crate::storage::create_private_dir(state)?;
        let path = state.join(format!("{PREFIX}{pid}{SUFFIX}"));
        let file = crate::storage::private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        if let Err(error) = file.try_lock() {
            drop(file);
            drop(fs::remove_file(&path));
            return Err(io::Error::other(error.to_string()));
        }
        let mut marker = Self {
            path,
            file,
            content: Content {
                skin: None,
                alone,
                window: false,
            },
        };
        marker.write()?;
        Ok(marker)
    }

    /// Writes the content in place; no flush (see the module's comment).
    fn write(&mut self) -> io::Result<()> {
        let bytes = serde_json::to_vec(&self.content).map_err(io::Error::other)?;
        self.file.seek(io::SeekFrom::Start(0))?;
        self.file.write_all(&bytes)?;
        self.file
            .set_len(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
    }

    /// Rewrites the marker, saying in the log when it cannot.
    fn update(&mut self) {
        if let Err(error) = self.write() {
            crate::log::report(
                crate::log::Level::Warning,
                &format!(
                    "cannot mark the skin loading in {}: {error}",
                    self.path.display()
                ),
            );
        }
    }

    /// Names the skin that starts to load (`None`: none loads, or the one
    /// shown has drawn).
    pub fn set_skin(&mut self, skin: Option<&Path>) {
        let skin = skin.map(|path| path.to_string_lossy().into_owned());
        if self.content.skin != skin {
            self.content.skin = skin;
            self.update();
        }
    }

    /// The process has a window of its own: it hands no files over.
    pub fn set_window(&mut self) {
        if !self.content.window {
            self.content.window = true;
            self.update();
        }
    }

    /// The skin the marker names.
    #[must_use]
    pub fn skin(&self) -> Option<&str> {
        self.content.skin.as_deref()
    }

    /// A clean exit: the marker goes (removed while still held).
    pub fn remove(self) {
        drop(fs::remove_file(&self.path));
    }

    /// What the file holds, read through the handle that holds its lock
    /// (Windows' locks keep other handles from reading).
    #[cfg(test)]
    fn read_back(&mut self) -> Content {
        let mut text = String::new();
        self.file.seek(io::SeekFrom::Start(0)).expect("seek");
        self.file.read_to_string(&mut text).expect("read");
        serde_json::from_str(&text).expect("json")
    }
}

/// At start: [`scan`], then this process's own [`Marker`] (`None` where the
/// folder cannot hold one). Returns it and the skins whose load never
/// finished.
#[must_use]
pub fn start(state: &Path) -> (Option<Marker>, Vec<PathBuf>) {
    let found = scan(state);
    let marker = match Marker::create(state, std::process::id(), !found.others) {
        Ok(marker) => Some(marker),
        Err(error) => {
            crate::log::report(
                crate::log::Level::Warning,
                &format!("no skin marker in {}: {error}", state.display()),
            );
            None
        }
    };
    (marker, found.unfinished)
}

/// Whether `skin` is `other`, as markers name skins (lossy text).
#[must_use]
pub fn same_skin(skin: &Path, other: &Path) -> bool {
    skin.to_string_lossy() == other.to_string_lossy()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            drop(fs::remove_dir_all(&self.0));
        }
    }

    fn scratch(name: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("rvp-skin-marker-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }

    fn markers(state: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(state)
            .expect("dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(PREFIX))
            .collect();
        names.sort();
        names
    }

    /// What a process that ended left.
    fn left(state: &Path, pid: u32) -> Content {
        let text = fs::read_to_string(state.join(format!("{PREFIX}{pid}{SUFFIX}"))).expect("read");
        serde_json::from_str(&text).expect("json")
    }

    #[test]
    fn a_marker_names_a_skin_from_its_load_to_its_first_frame() {
        let dir = scratch("names");
        let skin = dir.0.join("Some Skin.dsf");
        let mut marker = Marker::create(&dir.0, 7, true).expect("marker");
        assert_eq!(
            marker.read_back(),
            Content {
                skin: None,
                alone: true,
                window: false
            }
        );
        // Before the skin loads.
        marker.set_skin(Some(&skin));
        assert_eq!(marker.skin(), Some(skin.to_string_lossy().as_ref()));
        assert_eq!(
            marker.read_back().skin.as_deref(),
            Some(skin.to_string_lossy().as_ref())
        );
        marker.set_window();
        assert!(marker.read_back().window);
        // A long name, then a short one: the file holds only the last.
        marker.set_skin(Some(&dir.0.join("A much longer name of another skin.dsf")));
        marker.set_skin(Some(Path::new("/s.vlt")));
        assert_eq!(marker.read_back().skin.as_deref(), Some("/s.vlt"));
        // It has drawn.
        marker.set_skin(None);
        assert_eq!(marker.read_back().skin, None);
        // A running RVP's marker is not another's to take.
        let found = scan(&dir.0);
        assert!(found.others);
        assert!(found.unfinished.is_empty());
        assert_eq!(markers(&dir.0), ["skin-load-7.json"]);
        // A clean exit removes it.
        marker.remove();
        assert!(markers(&dir.0).is_empty());
    }

    #[test]
    fn a_start_finds_the_skin_an_ended_rvp_was_loading() {
        let dir = scratch("ended");
        let skin = dir.0.join("Broken.dsf");
        let mut marker = Marker::create(&dir.0, 11, true).expect("marker");
        marker.set_skin(Some(&skin));
        // The process ends without its clean exit: the lock goes, the file
        // stays.
        drop(marker);
        assert_eq!(
            left(&dir.0, 11).skin,
            Some(skin.to_string_lossy().into_owned())
        );
        // An RVP that ended after its skin drew left nothing to find.
        let mut drawn = Marker::create(&dir.0, 12, true).expect("marker");
        drawn.set_skin(Some(&dir.0.join("Fine.dsf")));
        drawn.set_skin(None);
        drop(drawn);
        let found = scan(&dir.0);
        assert_eq!(
            found,
            Found {
                unfinished: vec![skin.clone()],
                others: false
            }
        );
        // Both are gone: the next start finds nothing.
        assert!(markers(&dir.0).is_empty());
        assert_eq!(scan(&dir.0), Found::default());
        // The start's own marker says it was alone.
        let (own, unfinished) = start(&dir.0);
        assert!(unfinished.is_empty());
        let mut own = own.expect("a marker");
        assert!(own.read_back().alone);
        own.remove();
    }

    #[test]
    fn another_running_rvps_marker_is_left_alone() {
        let dir = scratch("running");
        let skin = dir.0.join("Loading.dsf");
        // Another RVP, loading a skin right now.
        let mut other = Marker::create(&dir.0, 21, true).expect("marker");
        other.set_skin(Some(&skin));
        let (own, unfinished) = start(&dir.0);
        assert!(unfinished.is_empty(), "{unfinished:?}");
        let mut own = own.expect("a marker");
        // This start was not alone, and the other's marker is still there.
        assert!(!own.read_back().alone);
        assert_eq!(
            other.read_back().skin.as_deref(),
            Some(skin.to_string_lossy().as_ref())
        );
        own.remove();
        other.remove();
    }

    #[test]
    fn a_start_that_was_not_alone_counts_once_it_had_a_window() {
        let dir = scratch("handed");
        let skin = dir.0.join("Any.dsf");
        // A second `rvp` that handed its files over and ended in the middle
        // of its load.
        let mut handed = Marker::create(&dir.0, 31, false).expect("marker");
        handed.set_skin(Some(&skin));
        drop(handed);
        assert_eq!(scan(&dir.0), Found::default());
        assert!(markers(&dir.0).is_empty());
        // A second RVP of its own window ("Multiple instances").
        let mut own = Marker::create(&dir.0, 32, false).expect("marker");
        own.set_window();
        own.set_skin(Some(&skin));
        drop(own);
        assert_eq!(scan(&dir.0).unfinished, [skin]);
    }

    #[test]
    fn other_files_and_damaged_markers() {
        let dir = scratch("other");
        for name in [
            "skin-load-.json",
            "skin-load-1x.json",
            "settings.json",
            "skin-load-5.txt",
        ] {
            fs::write(dir.0.join(name), "{}").expect("write");
        }
        fs::write(dir.0.join("skin-load-41.json"), "{\"skin\": \"/a.ds").expect("write");
        fs::write(dir.0.join("skin-load-42.json"), "").expect("write");
        assert_eq!(scan(&dir.0), Found::default());
        // Only markers' files are read and removed.
        assert!(
            markers(&dir.0)
                .iter()
                .all(|name| !is_marker(Path::new(name)))
        );
        assert_eq!(fs::read_dir(&dir.0).expect("dir").count(), 4);
        // No folder: nothing.
        assert_eq!(scan(&dir.0.join("missing")), Found::default());
    }

    #[test]
    fn skins_are_compared_as_markers_name_them() {
        assert!(same_skin(Path::new("/s/a.dsf"), Path::new("/s/a.dsf")));
        assert!(!same_skin(Path::new("/s/a.dsf"), Path::new("/s/b.dsf")));
    }
}
