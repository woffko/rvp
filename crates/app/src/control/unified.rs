//! RVP's Unified Remote remote (`remote/unified-remote/RVP`, M15) put into
//! Unified Remote Server's custom remotes by the Remote Control page's Add
//! to Unified Remote, on both systems (Windows' installer copies it too).
//! The packages carry the remote in `share/rvp/remote/unified-remote/RVP`,
//! a Windows bundle beside `rvp.exe`. The server reads custom remotes from
//! `~/.urserver/remotes/custom` on Linux (its `urserver-start`) and from
//! `%ProgramData%\Unified Remote\Remotes\Custom` on Windows, and reads them
//! again when its manager is asked to.
//!
//! The copy's `settings.prop` names this RVP's port and its token's file
//! (`token_file`): the remote finds a portable RVP's token, or one in a
//! state folder of its own, and the token itself is copied nowhere. A
//! folder of the remote's name that holds another remote, or a link or
//! junction on the way, is left alone: `%ProgramData%` is shared by the
//! computer's users, and a file is replaced by a new one, never written
//! through.

use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The remote's folder below a package's `share` and beside a Windows
/// bundle's `rvp.exe`.
const REMOTE: &str = "remote/unified-remote/RVP";
/// The remote's folder among the custom remotes.
const FOLDER: &str = "RVP";
/// The line of `meta.prop` that makes a folder RVP's remote.
const META_ID: &str = "meta.id: RVP.Player";
/// The remote's settings, which RVP writes for itself.
const SETTINGS: &str = "settings.prop";
/// Unified Remote Server's manager.
const MANAGER_PORT: u16 = 9510;

/// The remote's folder for RVP at `exe`: `../share/rvp/remote/...` in the
/// packages, `remote/...` beside a bundle.
#[must_use]
pub fn remote_source(exe: &Path) -> Option<PathBuf> {
    let folder = exe.parent()?;
    [
        folder.join("../share/rvp").join(REMOTE),
        folder.join(REMOTE),
    ]
    .into_iter()
    .find(|candidate| candidate.join("meta.prop").is_file())
}

/// This computer's Unified Remote Server: its data folder, which is there
/// once the server is installed (Windows) or has run for this user
/// (Linux), and its custom remotes below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFolders {
    pub data: PathBuf,
    pub custom: PathBuf,
}

impl ServerFolders {
    /// The folders of this system's server; `None` when the system names no
    /// home (Linux) or program data (Windows) folder.
    #[must_use]
    pub fn of_this_system() -> Option<Self> {
        #[cfg(windows)]
        {
            let data = PathBuf::from(std::env::var_os("ProgramData")?).join("Unified Remote");
            let custom = data.join("Remotes").join("Custom");
            Some(Self { data, custom })
        }
        #[cfg(not(windows))]
        {
            let data = PathBuf::from(std::env::var_os("HOME")?).join(".urserver");
            let custom = data.join("remotes").join("custom");
            Some(Self { data, custom })
        }
    }
}

/// The remote's settings for RVP on `port` with its token in `token_file`
/// (with `/` on Windows too, which its file functions take; no `\` to
/// escape).
#[must_use]
pub fn settings(port: u16, token_file: &Path) -> String {
    let file = token_file.to_string_lossy().replace('\\', "/");
    format!("port: {port}\ntoken: \ntoken_file: {file}\n")
}

/// Copies the remote from `source` into the custom remotes of `server`,
/// with [`settings`] for `port` and `token_file`; returns where it went.
///
/// # Errors
/// Fails when the server's data folder is missing, a folder on the way is
/// a link or no folder, the remote's folder holds another remote, or a
/// file cannot be written.
pub fn install(
    source: &Path,
    server: &ServerFolders,
    port: u16,
    token_file: &Path,
) -> Result<PathBuf, String> {
    if !server.data.is_dir() {
        return Err(if cfg!(windows) {
            format!(
                "Unified Remote Server is not installed ({} is missing)",
                server.data.display()
            )
        } else {
            format!(
                "Unified Remote Server has not run for this user ({} is missing)",
                server.data.display()
            )
        });
    }
    let target = server.custom.join(FOLDER);
    // Every folder below the data folder, the remote's own last.
    let below = target.strip_prefix(&server.data).map_err(|_| {
        format!(
            "{} is not below {}",
            target.display(),
            server.data.display()
        )
    })?;
    let mut folder = server.data.clone();
    for part in below.components() {
        folder.push(part);
        match std::fs::symlink_metadata(&folder) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!("{} is a link: left alone", folder.display()));
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(format!("{} is no folder: left alone", folder.display()));
            }
            Ok(_) => {}
            Err(_) => std::fs::create_dir(&folder)
                .map_err(|error| format!("{}: {error}", folder.display()))?,
        }
    }
    if let Ok(meta) = std::fs::read_to_string(target.join("meta.prop"))
        && !meta.lines().any(|line| line.trim() == META_ID)
    {
        return Err(format!(
            "{} holds another remote: left alone",
            target.display()
        ));
    }
    let entries = std::fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", source.display()))?;
        let name = entry.file_name();
        if name == SETTINGS || !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let bytes =
            std::fs::read(entry.path()).map_err(|e| format!("{}: {e}", entry.path().display()))?;
        replace(&target.join(&name), &bytes)?;
    }
    replace(
        &target.join(SETTINGS),
        settings(port, token_file).as_bytes(),
    )?;
    Ok(target)
}

/// Puts a new file with `bytes` at `path`: what was there goes first (a
/// link goes, its target stays as it was), and the new file is made only
/// where none is, so nothing is written through a link put there between.
fn replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let error = |e: std::io::Error| format!("{}: {e}", path.display());
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => {
            return Err(format!("{} is a folder: left alone", path.display()));
        }
        Ok(_) => std::fs::remove_file(path).map_err(error)?,
        Err(_) => {}
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(error)?;
    file.write_all(bytes).map_err(error)
}

/// Asks the server on this computer to read its remotes again (its
/// manager's `/system/reload`); false when it does not answer.
#[must_use]
pub fn reload() -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, MANAGER_PORT));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_secs(1)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let request = format!(
        "GET /system/reload HTTP/1.1\r\nHost: 127.0.0.1:{MANAGER_PORT}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut answer = [0u8; 16];
    stream
        .read(&mut answer)
        .is_ok_and(|read| answer[..read].starts_with(b"HTTP/1.1 2"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-unified-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// A package's remote and the server's folders in `dir`.
    fn package(dir: &Path) -> (PathBuf, ServerFolders) {
        let bin = dir.join("opt/rvp/bin");
        let source = dir.join("opt/rvp/share/rvp").join(REMOTE);
        std::fs::create_dir_all(&bin).expect("bin");
        std::fs::create_dir_all(&source).expect("source");
        for name in ["remote.lua", "layout.xml"] {
            std::fs::write(source.join(name), name).expect("file");
        }
        std::fs::write(
            source.join("meta.prop"),
            format!("{META_ID}\nmeta.name: RVP\n"),
        )
        .expect("meta");
        std::fs::write(
            source.join(SETTINGS),
            "port: 48931\ntoken: \ntoken_file: \n",
        )
        .expect("settings");
        let found = remote_source(&bin.join("rvp")).expect("found");
        assert!(found.ends_with(REMOTE));
        let data = dir.join("home/.urserver");
        let custom = data.join("remotes/custom");
        (found, ServerFolders { data, custom })
    }

    #[test]
    fn the_remote_goes_into_the_custom_remotes_with_this_rvps_settings() {
        let dir = scratch("copy");
        let (source, server) = package(&dir);
        let token = dir.join("state/remote-token");
        let refused = install(&source, &server, 48931, &token).expect_err("the server has not run");
        assert!(refused.contains("is missing"), "{refused}");
        std::fs::create_dir_all(&server.data).expect("data");
        let target = install(&source, &server, 48940, &token).expect("installed");
        assert_eq!(target, server.custom.join(FOLDER));
        assert_eq!(
            std::fs::read_to_string(target.join("remote.lua")).expect("lua"),
            "remote.lua"
        );
        // The settings are this RVP's: its port and token file, no token.
        assert_eq!(
            std::fs::read_to_string(target.join(SETTINGS)).expect("settings"),
            format!("port: 48940\ntoken: \ntoken_file: {}\n", token.display())
        );
        // Again over its own copy: the remote's files are new, the user's
        // other files stay.
        std::fs::write(target.join("remote.lua"), "old").expect("old");
        std::fs::write(target.join("layout.xml.bak"), "kept").expect("kept");
        install(&source, &server, 48931, &token).expect("again");
        assert_eq!(
            std::fs::read_to_string(target.join("remote.lua")).expect("lua"),
            "remote.lua"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("layout.xml.bak")).expect("bak"),
            "kept"
        );
        assert!(remote_source(&dir.join("elsewhere/rvp")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_windows_token_file_is_named_with_slashes() {
        assert_eq!(
            settings(48931, Path::new(r"C:\Users\Name\RVP\state\remote-token")),
            "port: 48931\ntoken: \ntoken_file: C:/Users/Name/RVP/state/remote-token\n"
        );
    }

    #[test]
    fn another_remotes_folder_is_left_alone() {
        let dir = scratch("foreign");
        let (source, server) = package(&dir);
        let target = server.custom.join(FOLDER);
        std::fs::create_dir_all(&target).expect("target");
        std::fs::write(target.join("meta.prop"), "meta.id: Someone.Else\n").expect("meta");
        std::fs::write(target.join("remote.lua"), "theirs").expect("theirs");
        let refused = install(&source, &server, 48931, &dir.join("t")).expect_err("left alone");
        assert!(refused.contains("another remote"), "{refused}");
        assert_eq!(
            std::fs::read_to_string(target.join("remote.lua")).expect("lua"),
            "theirs"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn links_on_the_way_and_in_the_folder_are_not_followed() {
        let dir = scratch("links");
        let (source, server) = package(&dir);
        std::fs::create_dir_all(&server.data).expect("data");
        // The custom remotes' folder is a link: nothing is written there.
        let elsewhere = dir.join("elsewhere");
        std::fs::create_dir_all(&elsewhere).expect("elsewhere");
        std::fs::create_dir_all(server.custom.parent().expect("remotes")).expect("remotes");
        std::os::unix::fs::symlink(&elsewhere, &server.custom).expect("link");
        let refused = install(&source, &server, 48931, &dir.join("t")).expect_err("a link");
        assert!(refused.contains("is a link"), "{refused}");
        assert_eq!(std::fs::read_dir(&elsewhere).expect("read").count(), 0);
        std::fs::remove_file(&server.custom).expect("unlink");
        // A file of the remote is a link: the link goes, its target stays.
        let target = server.custom.join(FOLDER);
        std::fs::create_dir_all(&target).expect("target");
        let victim = dir.join("victim");
        std::fs::write(&victim, "untouched").expect("victim");
        std::os::unix::fs::symlink(&victim, target.join("remote.lua")).expect("link");
        install(&source, &server, 48931, &dir.join("t")).expect("installed");
        assert_eq!(
            std::fs::read_to_string(&victim).expect("victim"),
            "untouched"
        );
        assert!(
            !std::fs::symlink_metadata(target.join("remote.lua"))
                .expect("lua")
                .file_type()
                .is_symlink()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
