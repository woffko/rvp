//! The control API's token: 32 random bytes from the system (as 64 hex
//! digits), made once per installation and kept in the state folder,
//! readable by the user only where the system has such modes. It never
//! goes into logs; the Remote page shows it and can renew it.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

/// The token's file in the state folder.
pub const FILE: &str = "remote-token";

/// The token's length in hex digits.
const LENGTH: usize = 64;

fn path(state_dir: &Path) -> PathBuf {
    state_dir.join(FILE)
}

/// The installation's token, made on first use (and again when the file
/// holds something else).
///
/// # Errors
/// Fails when the system gives no random bytes or the file cannot be
/// written.
pub fn load_or_create(state_dir: &Path) -> io::Result<String> {
    if let Ok(text) = std::fs::read_to_string(path(state_dir)) {
        let text = text.trim();
        if valid(text) {
            return Ok(text.to_owned());
        }
    }
    renew(state_dir)
}

/// A new token in place of the old one: clients that had the old one are
/// refused from now on.
///
/// # Errors
/// As [`load_or_create`].
pub fn renew(state_dir: &Path) -> io::Result<String> {
    let mut bytes = [0u8; LENGTH / 2];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    let token = bytes
        .iter()
        .fold(String::with_capacity(LENGTH), |mut text, byte| {
            // Writing to a String does not fail.
            let _ = write!(text, "{byte:02x}");
            text
        });
    std::fs::create_dir_all(state_dir)?;
    let target = path(state_dir);
    let temporary = state_dir.join(format!("{FILE}.new"));
    write_private(&temporary, &token)?;
    std::fs::rename(&temporary, &target)?;
    Ok(token)
}

/// Writes a file only its owner may read where the system has such modes
/// (on Windows the state folder is the user's own).
fn write_private(path: &Path, text: &str) -> io::Result<()> {
    let _ = std::fs::remove_file(path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    io::Write::write_all(&mut file, text.as_bytes())?;
    file.sync_all()
}

fn valid(text: &str) -> bool {
    text.len() == LENGTH && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Compares a presented token with the installation's in a time that does
/// not depend on where they differ.
#[must_use]
pub fn matches(presented: &str, token: &str) -> bool {
    let (a, b) = (presented.as_bytes(), token.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_made_once_and_renewed_on_demand() {
        let dir = std::env::temp_dir().join(format!("rvp-token-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = load_or_create(&dir).expect("token");
        assert!(valid(&first), "{} hex digits", first.len());
        assert_eq!(load_or_create(&dir).expect("token"), first);
        let renewed = renew(&dir).expect("renewed");
        assert_ne!(renewed, first);
        assert_eq!(load_or_create(&dir).expect("token"), renewed);
        // A damaged file gets a new token.
        std::fs::write(dir.join(FILE), "not a token").expect("write");
        let replaced = load_or_create(&dir).expect("token");
        assert!(valid(&replaced) && replaced != renewed);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(FILE))
                .expect("meta")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tokens_match_only_in_full() {
        assert!(matches("abcd", "abcd"));
        assert!(!matches("abce", "abcd"));
        assert!(!matches("abc", "abcd"));
        assert!(!matches("", "abcd"));
    }
}
