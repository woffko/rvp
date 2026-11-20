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
//! The shell carries a [`Request`] over a local socket (a Unix socket or a
//! Windows named pipe, [`channel`]) as one JSON line.
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

/// The channel of this process, where it finds a running instance and
/// listens for the next ones: `Ok(None)` when it neither forwards nor
/// listens (the self-test without `--instance`), an error that says why
/// it has none.
///
/// On Windows it is the name of a named pipe; the shell makes the pipe
/// for the current user alone and talks only to a pipe the current user
/// owns. On Unix it is a socket in a folder of the user's alone
/// ([`socket_path`]): Qt puts a bare name in the shared `/tmp`, where
/// another user could make it first, take the files handed over or keep
/// RVP from listening.
///
/// # Errors
/// On Unix, when no folder of the user's alone can hold the socket, or
/// when its path is longer than a socket's address takes: then there is
/// no channel, rather than one that cannot work.
pub fn channel(cli: &Cli) -> Result<Option<String>, String> {
    let Some(name) = channel_name(cli) else {
        return Ok(None);
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
        let state = crate::Dirs::from_env().ok().map(|dirs| dirs.state);
        // /proc/self belongs to the process's effective user.
        let uid = std::fs::metadata("/proc/self")
            .map_err(|error| format!("cannot find the user: {error}"))?
            .uid();
        let path = socket_path(&name, runtime.as_deref(), state.as_deref(), uid)?;
        path.to_str()
            .map(|path| Some(path.to_owned()))
            .ok_or_else(|| format!("{}: the path is not Unicode", path.display()))
    }
    #[cfg(not(unix))]
    Ok(Some(name))
}

/// The channel's name: `rvp-<user>`, or `rvp-<NAME>` with
/// `--instance=NAME`; `None` for the self-test without `--instance`.
fn channel_name(cli: &Cli) -> Option<String> {
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

/// The folder under RVP's state folder that holds the socket when the
/// runtime folder is not the user's alone.
#[cfg(unix)]
const SOCKET_FOLDER: &str = "channel";

/// The longest socket path RVP's Qt binds: a Unix socket's address holds
/// 108 bytes with the closing NUL, and Qt 6.10 refuses a path of 107.
#[cfg(unix)]
const SOCKET_PATH_MAX: usize = 106;

/// Qt's server binds the socket in a temporary folder beside it first
/// (`XXXXXX/s` in the socket's folder) and then renames it into place: that
/// path is the folder's and these bytes.
#[cfg(unix)]
const BIND_EXTRA: usize = "/XXXXXX/s".len();

/// Where the socket `name` lies for the user `uid`: in `runtime`
/// (`$XDG_RUNTIME_DIR`) when that is a directory of the user's that no
/// one else may enter, not a link to one; otherwise in `state`'s
/// [`SOCKET_FOLDER`], made for the user alone. Every RVP of the user
/// takes the same place by the same rule.
///
/// # Errors
/// When neither folder will do, or the socket's path (or the one Qt binds
/// first) is longer than a socket's address takes.
#[cfg(unix)]
fn socket_path(
    name: &str,
    runtime: Option<&Path>,
    state: Option<&Path>,
    uid: u32,
) -> Result<std::path::PathBuf, String> {
    let folder =
        if let Some(dir) = runtime.filter(|dir| dir.is_absolute() && private_folder(dir, uid)) {
            dir.to_path_buf()
        } else {
            let state = state.ok_or("no runtime folder of the user's alone and no state folder")?;
            own_folder(&state.join(SOCKET_FOLDER), uid)?
        };
    let path = folder.join(name);
    let longest = path
        .as_os_str()
        .len()
        .max(folder.as_os_str().len() + BIND_EXTRA);
    if longest > SOCKET_PATH_MAX {
        return Err(format!(
            "{}: too long for a socket (at most {SOCKET_PATH_MAX} bytes, {} for its folder)",
            path.display(),
            SOCKET_PATH_MAX - BIND_EXTRA
        ));
    }
    Ok(path)
}

/// The mode bits of a folder's group and others.
#[cfg(unix)]
const OTHERS: u32 = 0o077;

/// Whether `dir` is a directory (not a link to one) of the user `uid`'s
/// that no one else may enter.
#[cfg(unix)]
fn private_folder(dir: &Path, uid: u32) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::symlink_metadata(dir)
        .is_ok_and(|meta| meta.is_dir() && meta.uid() == uid && meta.mode() & OTHERS == 0)
}

/// Makes `dir` for the user `uid` alone (0700), or takes the one there
/// when it is the user's; one with wider access is narrowed.
#[cfg(unix)]
fn own_folder(dir: &Path, uid: u32) -> Result<std::path::PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot make {}: {error}", parent.display()))?;
    }
    match std::fs::DirBuilder::new().mode(0o700).create(dir) {
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
            return Err(format!("cannot make {}: {error}", dir.display()));
        }
        _ => {}
    }
    let wider = std::fs::symlink_metadata(dir)
        .is_ok_and(|meta| meta.is_dir() && meta.uid() == uid && meta.mode() & OTHERS != 0);
    if wider {
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    if private_folder(dir, uid) {
        Ok(dir.to_path_buf())
    } else {
        Err(format!(
            "{} is not a folder of the user's alone",
            dir.display()
        ))
    }
}

/// Whether this process hands its files to a running instance: not with
/// `/new`, not without a channel name, and not when a file is standard
/// input, which a running instance cannot read: this process plays it.
#[must_use]
pub fn forwards(cli: &Cli) -> bool {
    !cli.flag("new") && !cli.files.iter().any(|file| file == STDIN) && channel_name(cli).is_some()
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
        assert_eq!(channel(&cli(&["--selftest"])), Ok(None));
        assert!(!forwards(&cli(&["--selftest", "a.mkv"])));
        assert_eq!(
            channel_name(&cli(&["--selftest", "--instance=t 1"])).as_deref(),
            Some("rvp-t_1")
        );
        assert!(channel_name(&cli(&["a.mkv"])).is_some_and(|name| name.starts_with("rvp-")));
        assert!(!forwards(&cli(&["/new", "a.mkv"])));
        assert!(forwards(&cli(&["a.mkv"])));
    }

    /// A folder of its own for a test, removed first.
    #[cfg(unix)]
    fn scratch(test: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-channel-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch folder");
        dir
    }

    #[cfg(unix)]
    fn uid() -> u32 {
        use std::os::unix::fs::MetadataExt as _;
        std::fs::metadata("/proc/self").expect("/proc/self").uid()
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::symlink_metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn the_socket_lies_in_a_folder_of_the_users_alone() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = scratch("folder");
        let set_mode = |path: &Path, mode: u32| {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
        };
        let runtime = dir.join("run");
        std::fs::create_dir(&runtime).expect("runtime");
        set_mode(&runtime, 0o700);
        let state = dir.join("state");
        let fallback = state.join(SOCKET_FOLDER).join("rvp-t");
        // The runtime folder when it is the user's alone.
        assert_eq!(
            socket_path("rvp-t", Some(&runtime), Some(&state), uid()),
            Ok(runtime.join("rvp-t"))
        );
        assert!(!state.exists(), "the state folder is left alone");
        // One others may enter: the state's folder, made 0700.
        set_mode(&runtime, 0o755);
        assert_eq!(
            socket_path("rvp-t", Some(&runtime), Some(&state), uid()),
            Ok(fallback.clone())
        );
        assert_eq!(mode(&state.join(SOCKET_FOLDER)), 0o700);
        set_mode(&runtime, 0o700);
        // Another user's, a link to one, a relative one, none at all.
        assert_eq!(
            socket_path("rvp-t", Some(&runtime), Some(&state), uid() + 1),
            Err(format!(
                "{} is not a folder of the user's alone",
                state.join(SOCKET_FOLDER).display()
            ))
        );
        let link = dir.join("link");
        std::os::unix::fs::symlink(&runtime, &link).expect("link");
        for runtime in [Some(link.as_path()), Some(Path::new("run")), None] {
            assert_eq!(
                socket_path("rvp-t", runtime, Some(&state), uid()),
                Ok(fallback.clone())
            );
        }
        // The state's folder, made earlier with wider access, is narrowed.
        set_mode(&state.join(SOCKET_FOLDER), 0o755);
        assert_eq!(
            socket_path("rvp-t", None, Some(&state), uid()),
            Ok(fallback.clone())
        );
        assert_eq!(mode(&state.join(SOCKET_FOLDER)), 0o700);
        // A link in the state's place is not followed.
        let linked_state = dir.join("linked-state");
        std::fs::create_dir(&linked_state).expect("state");
        std::os::unix::fs::symlink(&runtime, linked_state.join(SOCKET_FOLDER)).expect("link");
        assert!(socket_path("rvp-t", None, Some(&linked_state), uid()).is_err());
        assert!(socket_path("rvp-t", None, None, uid()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_socket_path_too_long_gives_no_channel() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = scratch("long");
        let base = dir.as_os_str().len() + 1;
        if base + 40 > SOCKET_PATH_MAX {
            eprintln!("note: the temporary folder's path is too long for this test");
            return;
        }
        let private = |letter: &str, length: usize| {
            let folder = dir.join(letter.repeat(length - base));
            std::fs::create_dir(&folder).expect("folder");
            std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700))
                .expect("chmod");
            folder
        };
        // A long name: the socket's own path is the longer one. Exactly
        // the longest Qt binds, then one byte more.
        let name = "rvp-longer";
        let fitting = private("a", SOCKET_PATH_MAX - 1 - name.len());
        let fits = socket_path(name, Some(&fitting), None, uid()).expect("fits");
        assert_eq!(fits.as_os_str().len(), SOCKET_PATH_MAX);
        let longer = private("b", SOCKET_PATH_MAX - name.len());
        let error = socket_path(name, Some(&longer), None, uid()).expect_err("too long");
        assert!(
            error.ends_with("too long for a socket (at most 106 bytes, 97 for its folder)"),
            "{error}"
        );
        // A short name: the path Qt binds first in the socket's folder is
        // the longer one.
        let fitting = private("c", SOCKET_PATH_MAX - BIND_EXTRA);
        assert!(socket_path("rvp-t", Some(&fitting), None, uid()).is_ok());
        let longer = private("d", SOCKET_PATH_MAX - BIND_EXTRA + 1);
        assert!(socket_path("rvp-t", Some(&longer), None, uid()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
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
