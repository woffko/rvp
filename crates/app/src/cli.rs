//! Command line: `rvp [--option[=value]]... [FILE|URL]...`.
//!
//! Options are always written as `--name` or `--name=value`, so a file name
//! never has to be guessed to be an option value. `--` ends the options.
//! PotPlayer's switches (`crate::instance::SWITCHES`: `/add`, `/seek=1800`,
//! `/sub="a b.srt"`, `/cd:D`, any case) are options too, their values with
//! the quotes taken off; after `--` they are file names.
//!
//! RVP's own arguments are read as the system gives them
//! ([`Cli::from_env`]): one that is not Unicode (a file name in another
//! encoding) is taken with its lost characters replaced and named in
//! [`Cli::notices`], and as a file it is left out, since RVP cannot open
//! the file by that name.
//!
//! On Windows a file argument that ends in `"` had a backslash there:
//! the shell fills `"%L"` with a drive's root (`E:\`), and Windows' rules
//! for a command line read `\"` as a quote, so `"E:\"` arrives as `E:"`.
//! No Windows name holds a `"`; the backslash is put back.
//!
//! The test hooks ([`TEST_HOOKS`]) take effect only when the environment
//! has `RVP_TEST_HOOKS=1`: a command line alone does not turn them on.

use std::collections::BTreeMap;
use std::ffi::OsString;

/// The options for tests only, which act on the machine or skip a
/// question: `--power-dry-run=FILE` (the power actions' dry run into
/// FILE), `--crash-test` and `--install-tensorrt` (RIFE (TensorRT)'s
/// download without the consent to NVIDIA's terms).
pub const TEST_HOOKS: &[&str] = &["power-dry-run", "crash-test", "install-tensorrt"];

/// The parsed command line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cli {
    /// Files and URLs to open, in order.
    pub files: Vec<String>,
    /// Options by name, without the leading dashes; flags have an empty value.
    pub options: BTreeMap<String, String>,
    /// The arguments that were not Unicode, their lost characters
    /// replaced (U+FFFD).
    pub not_unicode: Vec<String>,
    /// The test hooks given without `RVP_TEST_HOOKS=1`, as they were
    /// written.
    pub ignored_hooks: Vec<String>,
}

impl Cli {
    /// Parses the arguments after the program name.
    #[must_use]
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Self {
        Self::parse_for(args, cfg!(windows))
    }

    /// [`Cli::parse`] by `windows`' rules or Unix's (tests).
    fn parse_for<I: IntoIterator<Item = String>>(args: I, windows: bool) -> Self {
        let mut cli = Self::default();
        let mut only_files = false;
        for arg in args {
            if only_files {
                cli.files.push(file_argument(arg, windows));
            } else if arg == "--" {
                only_files = true;
            } else if let Some((switch, value)) = potplayer_switch(&arg) {
                cli.options.insert(switch, value);
            } else if let Some(option) = arg.strip_prefix("--") {
                let (name, value) = option.split_once('=').unwrap_or((option, ""));
                cli.options.insert(name.to_owned(), value.to_owned());
            } else {
                cli.files.push(file_argument(arg, windows));
            }
        }
        cli
    }

    /// RVP's own command line: the process's arguments after its name,
    /// the test hooks only with `RVP_TEST_HOOKS=1`.
    #[must_use]
    pub fn from_env() -> Self {
        let mut cli = Self::parse_os(std::env::args_os().skip(1));
        cli.allow_test_hooks(std::env::var_os("RVP_TEST_HOOKS").is_some_and(|value| value == "1"));
        cli
    }

    /// Leaves out the test hooks unless `allowed`, each listed in
    /// `ignored_hooks`. `--power-dry-run` stays a dry run, into the state
    /// folder's file instead of the one it names: an ignored hook never
    /// powers a machine off.
    pub fn allow_test_hooks(&mut self, allowed: bool) {
        if allowed {
            return;
        }
        for hook in TEST_HOOKS {
            let Some(value) = self.options.get_mut(*hook) else {
                continue;
            };
            if *hook == "power-dry-run" {
                if value.is_empty() {
                    continue;
                }
                self.ignored_hooks.push(format!("--{hook}={value}"));
                value.clear();
            } else {
                let written = if value.is_empty() {
                    format!("--{hook}")
                } else {
                    format!("--{hook}={value}")
                };
                self.ignored_hooks.push(written);
                self.options.remove(*hook);
            }
        }
    }

    /// A line for each test hook left out ([`Cli::allow_test_hooks`]).
    #[must_use]
    pub fn hook_notices(&self) -> Vec<String> {
        self.ignored_hooks
            .iter()
            .map(|hook| {
                if hook.starts_with("--power-dry-run=") {
                    format!(
                        "{hook}: a test option, taken only with RVP_TEST_HOOKS=1; \
                         the dry run goes to the state folder's power-dry-run.txt"
                    )
                } else {
                    format!("{hook}: a test option, ignored without RVP_TEST_HOOKS=1")
                }
            })
            .collect()
    }

    /// Parses arguments as the system gives them: one that is not Unicode
    /// is taken lossily and listed in `not_unicode`; as a file it is left
    /// out.
    #[must_use]
    pub fn parse_os<I: IntoIterator<Item = OsString>>(args: I) -> Self {
        let mut kept = Vec::new();
        let mut not_unicode = Vec::new();
        let mut only_files = false;
        for arg in args {
            match arg.into_string() {
                Ok(arg) => {
                    only_files |= arg == "--";
                    kept.push(arg);
                }
                Err(arg) => {
                    let arg = arg.to_string_lossy().into_owned();
                    let option =
                        !only_files && (arg.starts_with("--") || potplayer_switch(&arg).is_some());
                    not_unicode.push(arg.clone());
                    if option {
                        kept.push(arg);
                    }
                }
            }
        }
        let mut cli = Self::parse(kept);
        cli.not_unicode = not_unicode;
        cli
    }

    /// What the user should know about the command line: each argument
    /// that was not Unicode.
    #[must_use]
    pub fn notices(&self) -> Vec<String> {
        self.not_unicode
            .iter()
            .map(|arg| format!("cannot open {arg}: its name is not Unicode"))
            .collect()
    }

    #[must_use]
    pub fn flag(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }

    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.options
            .get(name)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }
}

/// A file argument as RVP opens it: on Windows (`windows`) a trailing
/// `"` that stands for a backslash is one again (the module comment).
fn file_argument(arg: String, windows: bool) -> String {
    if !windows || crate::playlist::is_url(&arg) {
        return arg;
    }
    match arg.strip_suffix('"') {
        Some(path) if !path.contains('"') => format!("{path}\\"),
        _ => arg,
    }
}

/// One of PotPlayer's switches: its name (lower case) and value, if `arg`
/// is one. `/name`, `/name=value` and `/name:value` (only `/cd:D` uses the
/// colon); the value may be quoted.
fn potplayer_switch(arg: &str) -> Option<(String, String)> {
    let rest = arg.strip_prefix('/')?;
    let split = rest.find(['=', ':']);
    let (name, value) = match split {
        Some(at) => (&rest[..at], &rest[at + 1..]),
        None => (rest, ""),
    };
    let name = name.to_ascii_lowercase();
    if !crate::instance::SWITCHES.contains(&name.as_str()) {
        return None;
    }
    // `/C:\films` on Windows is a path, not the switch `c`; only `cd`
    // takes a colon.
    if split.is_some_and(|at| rest.as_bytes()[at] == b':') && name != "cd" {
        return None;
    }
    let value = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(value);
    Some((name, value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse(args.iter().map(|&arg| arg.to_owned()))
    }

    #[test]
    fn files_and_options() {
        let cli = parse(&[
            "--skin=/s/BlackSpace_en.dsf",
            "a.mkv",
            "--selftest",
            "b.mkv",
        ]);
        assert_eq!(cli.files, ["a.mkv", "b.mkv"]);
        assert_eq!(cli.value("skin"), Some("/s/BlackSpace_en.dsf"));
        assert!(cli.flag("selftest"));
        assert_eq!(cli.value("selftest"), None);
    }

    #[test]
    fn potplayer_switches_take_values() {
        let cli = parse(&[
            "/SEEK=0:30:00",
            "/sub=\"a b.srt\"",
            "/cd:D",
            "/title=Film",
            "/clipboard",
            "/nothing=1",
            "a.mkv",
        ]);
        assert_eq!(cli.value("seek"), Some("0:30:00"));
        assert_eq!(cli.value("sub"), Some("a b.srt"));
        assert_eq!(cli.value("cd"), Some("D"));
        assert_eq!(cli.value("title"), Some("Film"));
        assert!(cli.flag("clipboard"));
        // Not a switch: a file name.
        assert_eq!(cli.files, ["/nothing=1", "a.mkv"]);
    }

    /// An argument with `before` and `after` around a character the
    /// system's encoding has but Unicode lacks.
    fn not_unicode(before: &str, after: &str) -> OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt as _;
            // Latin-1's "é": not UTF-8.
            OsString::from_vec([before.as_bytes(), b"\xe9", after.as_bytes()].concat())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt as _;
            // An unpaired surrogate.
            let wide: Vec<u16> = before
                .encode_utf16()
                .chain([0xd800])
                .chain(after.encode_utf16())
                .collect();
            OsString::from_wide(&wide)
        }
    }

    #[test]
    fn arguments_that_are_not_unicode_do_not_stop_the_start() {
        let cli = Cli::parse_os([
            OsString::from("a.mkv"),
            not_unicode("caf", ".mkv"),
            not_unicode("--skin=/s/", ".dsf"),
            OsString::from("--selftest"),
            OsString::from("--"),
            not_unicode("--", ".mkv"),
        ]);
        // The files RVP can open by their names, and the options.
        assert_eq!(cli.files, ["a.mkv"]);
        assert!(cli.flag("selftest"));
        assert_eq!(cli.value("skin"), Some("/s/\u{fffd}.dsf"));
        assert_eq!(
            cli.not_unicode,
            [
                "caf\u{fffd}.mkv",
                "--skin=/s/\u{fffd}.dsf",
                "--\u{fffd}.mkv"
            ]
        );
        assert_eq!(
            cli.notices()[0],
            "cannot open caf\u{fffd}.mkv: its name is not Unicode"
        );
        // Unicode arguments parse as from strings.
        let plain = Cli::parse_os(["a.mkv", "--selftest"].map(OsString::from));
        assert_eq!(plain, parse(&["a.mkv", "--selftest"]));
        assert!(plain.notices().is_empty());
    }

    #[test]
    fn double_dash_ends_options() {
        let cli = parse(&["--", "--weird name.mkv"]);
        assert_eq!(cli.files, ["--weird name.mkv"]);
        assert!(cli.options.is_empty());
        // Where the launchers put PotPlayer's switches: before `--`.
        let cli = parse(&["/add", "--", "/add", "--selftest"]);
        assert!(cli.flag("add"));
        assert_eq!(cli.files, ["/add", "--selftest"]);
    }

    #[test]
    fn a_drive_root_keeps_its_backslash_on_windows() {
        let windows = |args: &[&str]| Cli::parse_for(args.iter().map(|&arg| arg.to_owned()), true);
        // `"rvp.exe" -- "E:\"` and `"rvp.exe" "C:\films\"` as Windows
        // splits them.
        assert_eq!(windows(&["--", "E:\""]).files, ["E:\\"]);
        assert_eq!(windows(&["C:\\films\""]).files, ["C:\\films\\"]);
        assert_eq!(windows(&["C:\\a.mkv"]).files, ["C:\\a.mkv"]);
        // A URL, and an argument with another quote, stay as they came.
        assert_eq!(
            windows(&["https://example.org/a\"", "a\"b\""]).files,
            ["https://example.org/a\"", "a\"b\""]
        );
        // Elsewhere a name may end in a quote.
        assert_eq!(parse_unix(&["say \"hi\""]).files, ["say \"hi\""]);
    }

    fn parse_unix(args: &[&str]) -> Cli {
        Cli::parse_for(args.iter().map(|&arg| arg.to_owned()), false)
    }

    #[test]
    fn test_hooks_need_the_variable() {
        let args = [
            "--crash-test=panic",
            "--install-tensorrt",
            "--power-dry-run=/x/dry.txt",
            "--selftest",
            "a.mkv",
        ];
        let mut allowed = parse(&args);
        allowed.allow_test_hooks(true);
        assert_eq!(allowed, parse(&args));
        assert_eq!(allowed.value("crash-test"), Some("panic"));
        assert_eq!(allowed.value("power-dry-run"), Some("/x/dry.txt"));
        let mut left_out = parse(&args);
        left_out.allow_test_hooks(false);
        assert!(!left_out.flag("crash-test"));
        assert!(!left_out.flag("install-tensorrt"));
        // Still a dry run, into the state folder's file.
        assert!(left_out.flag("power-dry-run"));
        assert_eq!(left_out.value("power-dry-run"), None);
        assert!(left_out.flag("selftest"));
        assert_eq!(left_out.files, ["a.mkv"]);
        assert_eq!(
            left_out.ignored_hooks,
            [
                "--power-dry-run=/x/dry.txt",
                "--crash-test=panic",
                "--install-tensorrt"
            ]
        );
        assert_eq!(
            left_out.hook_notices()[..2],
            [
                "--power-dry-run=/x/dry.txt: a test option, taken only with RVP_TEST_HOOKS=1; \
                 the dry run goes to the state folder's power-dry-run.txt",
                "--crash-test=panic: a test option, ignored without RVP_TEST_HOOKS=1"
            ]
        );
        // A plain dry run asks for nothing a test alone may do.
        let mut plain = parse(&["--power-dry-run"]);
        plain.allow_test_hooks(false);
        assert!(plain.flag("power-dry-run"));
        assert!(plain.hook_notices().is_empty());
    }
}
