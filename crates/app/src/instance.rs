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
//! PotPlayer's other switches that apply (R2's `command-line.tsv`) travel
//! with the request ([`Switches`]): the start time, subtitles, audio, the
//! title and HTTP headers for the first file, `/same` (the others are its
//! streams), `/sort` and `/randomize`, and the dialogs and sources that
//! open at once (`/filedlg`, `/urldlg`, `/folderdlg`, `/cap`, `/cam`,
//! `/dvd`, `/cd`, `/autoplay`, `/clipboard`); `coverage::command_line`
//! says why the rest do not apply.
//! The shell carries a [`Request`] over a local socket (Qt's
//! `QLocalServer`: a Unix socket or a Windows named pipe) as one JSON line.
//! The self-test neither forwards nor listens unless `--instance=NAME`
//! names a separate channel, so it never talks to the owner's player.
//!
//! What a command line names is checked the same way whether this process
//! plays it or a running one does ([`Request::admit`]): files and network
//! addresses, and the disc and capture addresses RVP's own Open menu
//! makes, which "Play in new instance" hands over. `-` is standard input,
//! which only the process that was given it can read: a command line that
//! names it is not handed over ([`forwards`]), and a request does not
//! carry it.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Cli;
use crate::playlist::{Refused, admitted};

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
    /// PotPlayer's other switches.
    #[serde(default)]
    pub switches: Switches,
}

/// PotPlayer's switches besides the mode, as the command line gave them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Switches {
    /// `/seek=time`: where the first file starts (`hh:mm:ss.ms` or
    /// seconds).
    pub seek: Option<String>,
    /// `/sub="file"`: a subtitle for the first file (absolute).
    pub sub: Option<String>,
    /// `/aud="file"`: an audio file for the first file (absolute).
    pub aud: Option<String>,
    pub title: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub headers: Option<String>,
    /// `/same`: the files after the first are its streams.
    pub same: bool,
    pub sort: bool,
    pub randomize: bool,
    /// What opens at once: `filedlg`, `urldlg`, `folderdlg`, `cap`, `cam`,
    /// `dvd`, `cd`, `autoplay` or `clipboard`.
    pub open: Option<String>,
    /// `/cd:D`: the drive.
    pub drive: Option<String>,
    /// `/config="name"`: the settings preset to use (M14's Configuration).
    #[serde(default)]
    pub config: Option<String>,
}

/// PotPlayer's switches RVP reads (as options: `/add` is the flag `add`).
/// `simple`, `atv` and `dtv` are read to say why they do nothing.
pub const SWITCHES: &[&str] = &[
    "add",
    "insert",
    "current",
    "new",
    "filedlg",
    "urldlg",
    "folderdlg",
    "simple",
    "cap",
    "cam",
    "atv",
    "dtv",
    "dvd",
    "cd",
    "autoplay",
    "same",
    "sort",
    "randomize",
    "clipboard",
    "seek",
    "sub",
    "aud",
    "user_agent",
    "referer",
    "headers",
    "title",
    "config",
];

/// The switches that open something at once.
const OPENERS: &[&str] = &[
    "filedlg",
    "urldlg",
    "folderdlg",
    "cap",
    "cam",
    "dvd",
    "cd",
    "autoplay",
    "clipboard",
];

impl Switches {
    fn from_cli(cli: &Cli, cwd: &Path) -> Self {
        let text = |name: &str| cli.value(name).map(str::to_owned);
        Self {
            seek: text("seek"),
            sub: cli.value("sub").map(|file| absolute(file, cwd)),
            aud: cli.value("aud").map(|file| absolute(file, cwd)),
            title: text("title"),
            user_agent: text("user_agent"),
            referer: text("referer"),
            headers: text("headers"),
            same: cli.flag("same"),
            sort: cli.flag("sort"),
            randomize: cli.flag("randomize"),
            open: OPENERS
                .iter()
                .find(|name| cli.flag(name))
                .map(|name| (*name).to_owned()),
            drive: text("cd"),
            config: text("config"),
        }
    }
}

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
        Self {
            mode,
            files,
            switches: Switches::from_cli(cli, cwd),
        }
    }

    /// One line of JSON.
    #[must_use]
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A request as it arrives. Standard input is its sender's, not this
    /// process's: `-` is no file of it.
    #[must_use]
    pub fn decode(line: &str) -> Option<Self> {
        let mut request: Self = serde_json::from_str(line.trim()).ok()?;
        request.files.retain(|file| file != STDIN);
        Some(request)
    }

    /// Leaves out what a command line may not name, a first start's and
    /// one handed over alike, and returns it for the notice. A file stays
    /// when it is a path or a network address
    /// ([`crate::playlist::admitted`]), an address of RVP's own Open menu
    /// ([`crate::sources::own_address`]) or standard input; `/sub` and
    /// `/aud` name files or network addresses.
    pub fn admit(&mut self) -> Refused {
        let mut refused = Refused::default();
        self.files = std::mem::take(&mut self.files)
            .into_iter()
            .filter_map(|file| {
                if file == STDIN || crate::sources::own_address(&file) {
                    return Some(file);
                }
                let kept = admitted(&file);
                if kept.is_none() {
                    refused.add(&file);
                }
                kept
            })
            .collect();
        for extra in [&mut self.switches.sub, &mut self.switches.aud] {
            if let Some(file) = extra.take() {
                *extra = admitted(&file);
                if extra.is_none() {
                    refused.add(&file);
                }
            }
        }
        refused
    }
}

/// Standard input on a command line.
const STDIN: &str = "-";

/// URLs, absolute paths and `-` (standard input) stay; anything else is
/// joined to `cwd`.
fn absolute(file: &str, cwd: &Path) -> String {
    if file == STDIN || crate::playlist::is_url(file) || Path::new(file).is_absolute() {
        file.to_owned()
    } else {
        cwd.join(file).display().to_string()
    }
}

/// The channel name: `rvp-<user>`, or `rvp-<NAME>` with `--instance=NAME`.
/// `None` when this process neither forwards nor listens: the self-test
/// without `--instance`. On Unix it lies in the user's runtime folder
/// (`$XDG_RUNTIME_DIR`, the user's alone): Qt puts a bare name in the
/// shared `/tmp`, where another user could make it first, take the files
/// handed over or keep RVP from listening (the Linux audit's M5).
#[must_use]
pub fn channel(cli: &Cli) -> Option<String> {
    let runtime = if cfg!(unix) {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .filter(|dir| dir.is_absolute() && dir.is_dir())
    } else {
        None
    };
    channel_in(cli, runtime.as_deref())
}

/// [`channel`] in `runtime`, a folder of the user's own, when given.
fn channel_in(cli: &Cli, runtime: Option<&Path>) -> Option<String> {
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
    let name = format!("rvp-{clean}");
    Some(runtime.map_or(name.clone(), |dir| dir.join(&name).display().to_string()))
}

/// Whether this process hands its files to a running instance: not with
/// `/new`, not without a channel, and not when a file is standard input,
/// which a running instance cannot read: this process plays it.
#[must_use]
pub fn forwards(cli: &Cli) -> bool {
    !cli.flag("new") && !cli.files.iter().any(|file| file == STDIN) && channel(cli).is_some()
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
        // Relative paths join the sender's folder (with `\` on Windows).
        let joined = cwd.join("a.mkv").display().to_string();
        assert_eq!(play.files, [joined.as_str(), "/m/b.mkv"]);
        assert_eq!(
            Request::from_cli(&cli(&["/add", "a.mkv"]), cwd).mode,
            Mode::Add
        );
        assert_eq!(
            Request::from_cli(&cli(&["--insert", "https://x.org/v.ts"]), cwd),
            Request {
                mode: Mode::Insert,
                files: vec!["https://x.org/v.ts".to_owned()],
                switches: Switches::default(),
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
    fn potplayer_switches_travel_with_the_request() {
        let cwd = Path::new("/home/u/films");
        let request = Request::from_cli(
            &cli(&[
                "/seek=1800",
                "/sub=s.srt",
                "/same",
                "/dvd",
                "/title=T",
                "a.mkv",
            ]),
            cwd,
        );
        let switches = &request.switches;
        assert_eq!(switches.seek.as_deref(), Some("1800"));
        assert_eq!(
            switches.sub.as_deref(),
            Some(cwd.join("s.srt").display().to_string().as_str())
        );
        assert!(switches.same);
        assert_eq!(switches.open.as_deref(), Some("dvd"));
        assert_eq!(switches.title.as_deref(), Some("T"));
        // An older instance's line has no switches.
        let old = Request::decode(r#"{"mode":"play","files":["/a.mkv"]}"#).expect("decodes");
        assert_eq!(old.switches, Switches::default());
    }

    #[test]
    fn requests_round_trip_as_one_line() {
        let request = Request {
            mode: Mode::Add,
            files: vec!["/a b/\"c\".mkv".to_owned(), "C:\\d\ne.mkv".to_owned()],
            switches: Switches {
                seek: Some("90".to_owned()),
                same: true,
                ..Switches::default()
            },
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
            channel_in(&cli(&["--selftest", "--instance=t 1"]), None).as_deref(),
            Some("rvp-t_1")
        );
        // In the user's runtime folder where there is one.
        assert_eq!(
            channel_in(&cli(&["--instance=t 1"]), Some(Path::new("/run/user/1000"))),
            Some(
                Path::new("/run/user/1000")
                    .join("rvp-t_1")
                    .display()
                    .to_string()
            )
        );
        assert!(channel(&cli(&["a.mkv"])).is_some_and(|name| {
            Path::new(&name)
                .file_name()
                .is_some_and(|file| file.to_string_lossy().starts_with("rvp-"))
        }));
        assert!(!forwards(&cli(&["/new", "a.mkv"])));
        assert!(forwards(&cli(&["a.mkv"])));
    }

    #[test]
    fn standard_input_stays_with_its_process() {
        // A running instance cannot read this process's input: the
        // command line that names it is not handed over.
        assert!(!forwards(&cli(&["-"])));
        assert!(!forwards(&cli(&["/add", "a.mkv", "-"])));
        assert!(forwards(&cli(&["a.mkv", "--", "-b.mkv"])));
        // It is this process's own file, kept as it is.
        let cwd = Path::new("/home/u/films");
        let mut own = Request::from_cli(&cli(&["-"]), cwd);
        assert_eq!(own.files, ["-"]);
        assert_eq!(own.admit().count(), 0);
        assert_eq!(own.files, ["-"]);
        // A request that names it anyway arrives without it.
        let arrived =
            Request::decode(r#"{"mode":"play","files":["-","/a.mkv","-"]}"#).expect("decodes");
        assert_eq!(arrived.files, ["/a.mkv"]);
    }

    #[test]
    fn a_command_line_names_files_network_and_own_addresses() {
        let cwd = Path::new("/home/u/films");
        let mut request = Request::from_cli(
            &cli(&[
                "/add",
                "/sub=av://lavfi:subtitles",
                "/aud=https://example.org/a.flac",
                "/m/a.mkv",
                "av://lavfi:testsrc",
                "https://example.org/v",
                "FD://0",
                "dvd://longest//discs/film",
                "av://v4l2:/dev/video0",
                "edl://a.mkv",
            ]),
            cwd,
        );
        let refused = request.admit();
        // Files and network addresses, and what the Open menu itself
        // makes ("Play in new instance" hands such entries over).
        assert_eq!(
            request.files,
            [
                "/m/a.mkv",
                "https://example.org/v",
                "dvd://longest//discs/film",
                "av://v4l2:/dev/video0"
            ]
        );
        assert_eq!(request.switches.sub, None);
        assert_eq!(
            request.switches.aud.as_deref(),
            Some("https://example.org/a.flac")
        );
        assert_eq!(refused.count(), 4);
        assert_eq!(
            refused.notice().as_deref(),
            Some(
                "Not opened: 4 addresses (av://, edl://, fd://). \
                 Only files and network addresses are taken this way."
            )
        );
        // The same request, handed over, is left with the same files.
        let mut handed = Request::decode(
            &Request::from_cli(&cli(&["av://lavfi:testsrc", "/m/a.mkv", "cdda://"]), cwd).encode(),
        )
        .expect("decodes");
        assert_eq!(handed.admit().count(), 1);
        assert_eq!(handed.files, ["/m/a.mkv", "cdda://"]);
    }
}
