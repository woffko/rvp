//! Single instance (M3): a second `rvp` hands its files to the running one
//! and exits. The switches are PotPlayer's (its `CmdLine64.txt`):
//!
//! * `/add`: append to the playlist (the shown album), do not play.
//! * `/insert`: insert just below the entry that plays, or append when
//!   nothing plays; do not play.
//! * `/current` (the default): play in the running instance.
//! * `/new`: start a separate instance.
//!
//! They may also be written `--add`, `--insert`, `--current` and `--new`.
//! The shell carries a [`Request`] over a local socket (Qt's
//! `QLocalServer`: a Unix socket or a Windows named pipe) as one JSON line.
//! The self-test neither forwards nor listens unless `--instance=NAME`
//! names a separate channel, so it never talks to the owner's player.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Cli;

/// What the running instance does with the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Play them (`/current`, or no switch).
    Play,
    /// Append them (`/add`).
    Add,
    /// Insert them after the entry that plays (`/insert`).
    Insert,
}

/// One message from a second instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub mode: Mode,
    /// Absolute paths and URLs.
    pub files: Vec<String>,
}

/// The switches as options (`/add` is stored as the flag `add`).
pub const SWITCHES: &[&str] = &["add", "insert", "current", "new"];

impl Request {
    /// The request of a command line; relative paths are made absolute
    /// against `cwd` (the running instance has its own).
    #[must_use]
    pub fn from_cli(cli: &Cli, cwd: &Path) -> Self {
        let mode = if cli.flag("add") {
            Mode::Add
        } else if cli.flag("insert") {
            Mode::Insert
        } else {
            Mode::Play
        };
        let files = cli.files.iter().map(|file| absolute(file, cwd)).collect();
        Self { mode, files }
    }

    /// One line of JSON.
    #[must_use]
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    #[must_use]
    pub fn decode(line: &str) -> Option<Self> {
        serde_json::from_str(line.trim()).ok()
    }
}

/// URLs and absolute paths stay; anything else is joined to `cwd`.
fn absolute(file: &str, cwd: &Path) -> String {
    let is_url = file
        .split_once("://")
        .is_some_and(|(scheme, _)| scheme.len() > 1 && scheme.chars().all(char::is_alphanumeric));
    if is_url || Path::new(file).is_absolute() {
        file.to_owned()
    } else {
        cwd.join(file).display().to_string()
    }
}

/// The channel name: `rvp-<user>`, or `rvp-<NAME>` with `--instance=NAME`.
/// `None` when this process neither forwards nor listens: the self-test
/// without `--instance`.
#[must_use]
pub fn channel(cli: &Cli) -> Option<String> {
    let suffix = match cli.value("instance") {
        Some(name) => name.to_owned(),
        None if cli.flag("selftest") => return None,
        None => std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_owned()),
    };
    let clean: String = suffix
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    Some(format!("rvp-{clean}"))
}

/// Whether this process hands its files to a running instance: not with
/// `/new`, and not without a channel.
#[must_use]
pub fn forwards(cli: &Cli) -> bool {
    !cli.flag("new") && channel(cli).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Cli {
        Cli::parse(args.iter().map(|&arg| arg.to_owned()))
    }

    #[test]
    fn switches_select_the_mode() {
        let cwd = Path::new("/home/u/films");
        let play = Request::from_cli(&cli(&["a.mkv", "/m/b.mkv"]), cwd);
        assert_eq!(play.mode, Mode::Play);
        assert_eq!(play.files, ["/home/u/films/a.mkv", "/m/b.mkv"]);
        assert_eq!(
            Request::from_cli(&cli(&["/add", "a.mkv"]), cwd).mode,
            Mode::Add
        );
        assert_eq!(
            Request::from_cli(&cli(&["--insert", "https://x.org/v.ts"]), cwd),
            Request {
                mode: Mode::Insert,
                files: vec!["https://x.org/v.ts".to_owned()],
            }
        );
        assert_eq!(
            Request::from_cli(&cli(&["/CURRENT", "a.mkv"]), cwd).mode,
            Mode::Play
        );
        // After `--` a file may be called /add.
        let escaped = Request::from_cli(&cli(&["--", "/add"]), cwd);
        assert_eq!(
            (escaped.mode, escaped.files),
            (Mode::Play, vec!["/add".to_owned()])
        );
    }

    #[test]
    fn requests_round_trip_as_one_line() {
        let request = Request {
            mode: Mode::Add,
            files: vec!["/a b/\"c\".mkv".to_owned(), "C:\\d\ne.mkv".to_owned()],
        };
        let line = request.encode();
        assert!(!line.contains('\n'));
        assert_eq!(Request::decode(&line), Some(request));
        assert_eq!(Request::decode("not json"), None);
    }

    #[test]
    fn channels_keep_the_self_test_apart() {
        assert_eq!(channel(&cli(&["--selftest"])), None);
        assert!(!forwards(&cli(&["--selftest", "a.mkv"])));
        assert_eq!(
            channel(&cli(&["--selftest", "--instance=t 1"])).as_deref(),
            Some("rvp-t_1")
        );
        assert!(channel(&cli(&["a.mkv"])).is_some_and(|name| name.starts_with("rvp-")));
        assert!(!forwards(&cli(&["/new", "a.mkv"])));
        assert!(forwards(&cli(&["a.mkv"])));
    }
}
