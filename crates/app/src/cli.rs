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

use std::collections::BTreeMap;
use std::ffi::OsString;

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
}

impl Cli {
    /// Parses the arguments after the program name.
    #[must_use]
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Self {
        let mut cli = Self::default();
        let mut only_files = false;
        for arg in args {
            if only_files {
                cli.files.push(arg);
            } else if arg == "--" {
                only_files = true;
            } else if let Some((switch, value)) = potplayer_switch(&arg) {
                cli.options.insert(switch, value);
            } else if let Some(option) = arg.strip_prefix("--") {
                let (name, value) = option.split_once('=').unwrap_or((option, ""));
                cli.options.insert(name.to_owned(), value.to_owned());
            } else {
                cli.files.push(arg);
            }
        }
        cli
    }

    /// RVP's own command line: the process's arguments after its name.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse_os(std::env::args_os().skip(1))
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
    }
}
