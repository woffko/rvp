//! The freedesktop.org trash (the Trash specification 1.0) for M11's
//! played files and the playlist's Delete Files, after the user confirmed
//! in RVP's own dialog. A file on the home folder's filesystem goes to
//! `$XDG_DATA_HOME/Trash`; one on another filesystem to that filesystem's
//! `.Trash/$uid` (when the administrator made `.Trash` a sticky folder, not
//! a link) or else `.Trash-$uid`. Each gets a `.trashinfo` file that file
//! managers restore it from. Files are renamed, never copied: a trash on
//! the file's own filesystem is always chosen.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Component, Path, PathBuf};

/// The sticky bit of a folder's mode.
const STICKY: u32 = 0o1000;
/// Names tried for a file before giving up (`name`, `name.2`, ...).
const MAX_NAMES: u32 = 10_000;

/// The user's trashes.
#[derive(Debug, Clone)]
pub struct Trash {
    /// `$XDG_DATA_HOME/Trash`.
    home: PathBuf,
    /// The user ID the other filesystems' trashes are named by.
    uid: u32,
}

impl Trash {
    /// The trashes of the user running RVP.
    ///
    /// # Errors
    /// Fails when neither `XDG_DATA_HOME` nor `HOME` names a folder.
    pub fn for_user() -> Result<Self, String> {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .ok_or_else(|| "the trash: no home folder".to_owned())?;
        // /proc/self belongs to the process's effective user.
        let uid = fs::metadata("/proc/self")
            .map_err(|error| format!("the trash: {error}"))?
            .uid();
        Ok(Self::new(data.join("Trash"), uid))
    }

    /// Trashes with the home trash at `home` (tests).
    #[must_use]
    pub fn new(home: PathBuf, uid: u32) -> Self {
        Self { home, uid }
    }

    /// Moves `file` into the trash of its filesystem; `deleted` is the local
    /// time, `YYYY-MM-DDThh:mm:ss`. Returns where it went.
    ///
    /// # Errors
    /// Fails when the file is missing, or no trash on its filesystem can
    /// be made or written.
    pub fn send(&self, file: &Path, deleted: &str) -> Result<PathBuf, String> {
        let error = |e: io::Error| format!("the trash: {}: {e}", file.display());
        let file = full_path(file).map_err(error)?;
        let device = fs::symlink_metadata(&file).map_err(error)?.dev();
        let (trash, shown) = if device_of(&self.home).map_err(error)? == device {
            (self.home.clone(), file.clone())
        } else {
            let top = top_folder(&file, device).map_err(error)?;
            let trash = self.top_trash(&top);
            // A trash at the filesystem's top keeps paths relative to it, so
            // they stay right wherever the filesystem is mounted.
            let shown = file
                .strip_prefix(&top)
                .map_or_else(|_| file.clone(), Path::to_path_buf);
            (trash, shown)
        };
        let files = trash.join("files");
        let info = trash.join("info");
        for folder in [&trash, &files, &info] {
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(folder)
                .map_err(error)?;
            // A folder another user made, or a link, would hand them the
            // file: the user's own folders only, as the specification asks
            // and gio checks (the Linux audit's L12).
            self.owned(folder)
                .map_err(|why| format!("the trash: {}: {why}", folder.display()))?;
        }
        let name = file
            .file_name()
            .ok_or_else(|| format!("the trash: {} has no name", file.display()))?
            .to_string_lossy()
            .into_owned();
        let text = format!(
            "[Trash Info]\nPath={}\nDeletionDate={deleted}\n",
            encode_path(&shown)
        );
        for number in 1..=MAX_NAMES {
            let stored = if number == 1 {
                name.clone()
            } else {
                format!("{name}.{number}")
            };
            let info_file = info.join(format!("{stored}.trashinfo"));
            // The info file is created first and exclusively: it reserves
            // the name, as the specification asks.
            let mut written = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&info_file)
            {
                Ok(written) => written,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(error(e)),
            };
            let target = files.join(&stored);
            if fs::symlink_metadata(&target).is_ok() {
                drop(written);
                let _ = fs::remove_file(&info_file);
                continue;
            }
            let moved = written
                .write_all(text.as_bytes())
                .and_then(|()| written.sync_all())
                .and_then(|()| fs::rename(&file, &target));
            if let Err(e) = moved {
                drop(written);
                let _ = fs::remove_file(&info_file);
                return Err(error(e));
            }
            return Ok(target);
        }
        Err(format!("the trash: no free name for {}", file.display()))
    }

    /// The trash at the top of another filesystem: `.Trash/$uid` when the
    /// administrator made `.Trash` (a sticky folder, not a link), else
    /// `.Trash-$uid`.
    fn top_trash(&self, top: &Path) -> PathBuf {
        let shared = top.join(".Trash");
        if let Ok(meta) = fs::symlink_metadata(&shared)
            && meta.is_dir()
            && meta.permissions().mode() & STICKY != 0
        {
            let own = shared.join(self.uid.to_string());
            if (DirBuilder::new().mode(0o700).create(&own).is_ok() || own.is_dir())
                && self.owned(&own).is_ok()
            {
                return own;
            }
        }
        top.join(format!(".Trash-{}", self.uid))
    }

    /// Whether `folder` is a folder of the user's own, not a link.
    fn owned(&self, folder: &Path) -> Result<(), &'static str> {
        let meta = fs::symlink_metadata(folder).map_err(|_| "cannot be read")?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            Err("not a folder (a link?)")
        } else if meta.uid() != self.uid {
            Err("another user's folder")
        } else {
            Ok(())
        }
    }
}

/// The file's absolute path with its folders' links resolved; the file
/// itself, a link included, is what goes to the trash.
fn full_path(file: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(file)?;
    let name = absolute
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    let folder = absolute
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .canonicalize()?;
    Ok(folder.join(name))
}

/// The device of `path`, or of its nearest folder that exists (the home
/// trash may not exist yet).
fn device_of(path: &Path) -> io::Result<u64> {
    let mut current = path;
    loop {
        match fs::metadata(current) {
            Ok(meta) => return Ok(meta.dev()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                current = current.parent().ok_or(e)?;
            }
            Err(e) => return Err(e),
        }
    }
}

/// The topmost folder above `file` on the same device: its filesystem's
/// mount point.
fn top_folder(file: &Path, device: u64) -> io::Result<PathBuf> {
    let mut top = file
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .to_path_buf();
    while let Some(parent) = top.parent() {
        if fs::metadata(parent)?.dev() != device {
            break;
        }
        top = parent.to_path_buf();
    }
    Ok(top)
}

/// A path in a `.trashinfo` file: escaped as a URI path (RFC 2396), its
/// separators kept.
fn encode_path(path: &Path) -> String {
    use std::fmt::Write as _;
    use std::os::unix::ffi::OsStrExt as _;
    let mut text = String::new();
    let absolute = path.has_root();
    for (index, component) in path
        .components()
        .filter(|c| !matches!(c, Component::RootDir))
        .enumerate()
    {
        if index > 0 || absolute {
            text.push('/');
        }
        for &byte in component.as_os_str().as_bytes() {
            if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
                text.push(char::from(byte));
            } else {
                let _ = write!(text, "%{byte:02X}");
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test's own user: the trash takes only its folders.
    fn uid() -> u32 {
        fs::metadata("/proc/self").expect("/proc/self").uid()
    }

    #[test]
    fn only_the_users_own_folders_are_taken() {
        let dir = scratch("owned");
        let trash = Trash::new(dir.join("Trash"), uid());
        let folder = dir.join("folder");
        fs::create_dir_all(&folder).expect("folder");
        assert_eq!(trash.owned(&folder), Ok(()));
        let link = dir.join("link");
        std::os::unix::fs::symlink(&folder, &link).expect("link");
        assert_eq!(trash.owned(&link), Err("not a folder (a link?)"));
        let other = Trash::new(dir.join("Trash"), uid() + 1);
        assert_eq!(other.owned(&folder), Err("another user's folder"));
        // A file sent through a linked trash fails rather than leaving.
        let file = dir.join("film.mkv");
        fs::write(&file, b"video").expect("file");
        let target = dir.join("elsewhere");
        fs::create_dir_all(&target).expect("elsewhere");
        std::os::unix::fs::symlink(&target, dir.join("Linked")).expect("trash link");
        let linked = Trash::new(dir.join("Linked"), uid());
        assert!(linked.send(&file, "2026-11-15T08:00:00").is_err());
        assert!(file.exists(), "the file stays");
        let _ = fs::remove_dir_all(&dir);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-trash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch folder");
        dir
    }

    #[test]
    fn a_file_goes_to_the_home_trash_with_its_info() {
        let dir = scratch("home");
        let trash = Trash::new(dir.join("data/Trash"), uid());
        let file = dir.join("a film #1.mkv");
        fs::write(&file, b"video").expect("file");
        let target = trash.send(&file, "2026-11-01T10:20:30").expect("trashed");
        assert!(!file.exists());
        assert_eq!(target, dir.join("data/Trash/files/a film #1.mkv"));
        assert_eq!(fs::read(&target).expect("moved"), b"video");
        let info =
            fs::read_to_string(dir.join("data/Trash/info/a film #1.mkv.trashinfo")).expect("info");
        let shown = dir.canonicalize().expect("folder").join("a film #1.mkv");
        assert!(encode_path(&shown).ends_with("/a%20film%20%231.mkv"));
        assert_eq!(
            info,
            format!(
                "[Trash Info]\nPath={}\nDeletionDate=2026-11-01T10:20:30\n",
                encode_path(&shown)
            )
        );
        let mode = fs::metadata(dir.join("data/Trash/files"))
            .expect("files")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_file_of_the_same_name_gets_a_new_one() {
        let dir = scratch("names");
        let trash = Trash::new(dir.join("Trash"), uid());
        for round in 0..3 {
            let folder = dir.join(format!("folder{round}"));
            fs::create_dir_all(&folder).expect("folder");
            fs::write(folder.join("clip.mp4"), [round]).expect("file");
            trash
                .send(&folder.join("clip.mp4"), "2026-11-01T10:20:30")
                .expect("trashed");
        }
        for (name, byte) in [("clip.mp4", 0), ("clip.mp4.2", 1), ("clip.mp4.3", 2)] {
            assert_eq!(
                fs::read(dir.join("Trash/files").join(name)).expect("stored"),
                [byte]
            );
            assert!(
                dir.join("Trash/info")
                    .join(format!("{name}.trashinfo"))
                    .is_file()
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_file_fails_and_leaves_no_info() {
        let dir = scratch("missing");
        let trash = Trash::new(dir.join("Trash"), uid());
        assert!(
            trash
                .send(&dir.join("gone.mkv"), "2026-11-01T10:20:30")
                .is_err()
        );
        assert!(!dir.join("Trash/info/gone.mkv.trashinfo").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_are_escaped_as_uris() {
        assert_eq!(
            encode_path(Path::new("/home/u/Видео/a b.mkv")),
            "/home/u/%D0%92%D0%B8%D0%B4%D0%B5%D0%BE/a%20b.mkv"
        );
        assert_eq!(encode_path(Path::new("films/x%y.mkv")), "films/x%25y.mkv");
    }

    #[test]
    fn the_top_folder_is_the_highest_on_the_same_device() {
        let dir = scratch("top");
        let file = dir.join("f");
        fs::write(&file, b"").expect("file");
        let device = fs::metadata(&file).expect("meta").dev();
        let top = top_folder(&file, device).expect("top");
        assert_eq!(fs::metadata(&top).expect("top meta").dev(), device);
        if let Some(parent) = top.parent() {
            assert_ne!(fs::metadata(parent).expect("parent").dev(), device);
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
