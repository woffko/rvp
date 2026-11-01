//! RVP's Unified Remote remote (`remote/unified-remote/RVP`, M15) put into
//! Unified Remote Server's custom remotes on Linux (goal S): the packages
//! carry the remote in `share/rvp/remote/unified-remote/RVP`, and the
//! Remote Control page's button copies it into `~/.urserver/remotes/custom`,
//! the folder Unified Remote Server for Linux reads them from (its
//! `urserver-start`), then asks the server to read its remotes again. On
//! Windows the installer copies it.

use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The remote's folder below a package's `share` and beside a Windows
/// bundle's `rvp.exe`.
const REMOTE: &str = "remote/unified-remote/RVP";
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

/// Copies the remote from `source` into the custom remotes of the server
/// whose data folder is `urserver` (`~/.urserver`); returns where it went.
///
/// # Errors
/// Fails when the server has not run for this user (no data folder) or a
/// file cannot be copied.
pub fn install(source: &Path, urserver: &Path) -> Result<PathBuf, String> {
    if !urserver.is_dir() {
        return Err(format!(
            "Unified Remote Server has not run for this user ({} is missing)",
            urserver.display()
        ));
    }
    let target = urserver.join("remotes/custom/RVP");
    let error = |e: std::io::Error| format!("{}: {e}", target.display());
    std::fs::create_dir_all(&target).map_err(error)?;
    for entry in std::fs::read_dir(source).map_err(|e| format!("{}: {e}", source.display()))? {
        let entry = entry.map_err(error)?;
        if entry.file_type().map_err(error)?.is_file() {
            std::fs::copy(entry.path(), target.join(entry.file_name())).map_err(error)?;
        }
    }
    Ok(target)
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

    #[test]
    fn the_remote_is_copied_into_the_custom_remotes() {
        let dir = std::env::temp_dir().join(format!("rvp-unified-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bin = dir.join("opt/rvp/bin");
        let source = dir.join("opt/rvp/share/rvp").join(REMOTE);
        std::fs::create_dir_all(&bin).expect("bin");
        std::fs::create_dir_all(&source).expect("source");
        for name in ["meta.prop", "remote.lua", "layout.xml"] {
            std::fs::write(source.join(name), name).expect("file");
        }
        let found = remote_source(&bin.join("rvp")).expect("found");
        assert!(found.ends_with(REMOTE));
        let home = dir.join("home/.urserver");
        assert!(install(&found, &home).is_err(), "the server has not run");
        std::fs::create_dir_all(&home).expect("home");
        let target = install(&found, &home).expect("installed");
        assert_eq!(target, home.join("remotes/custom/RVP"));
        assert_eq!(
            std::fs::read_to_string(target.join("remote.lua")).expect("lua"),
            "remote.lua"
        );
        assert!(remote_source(&dir.join("elsewhere/rvp")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
