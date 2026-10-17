//! Command line: `rvp [--option[=value]]... [FILE|URL]...`.
//!
//! Options are always written as `--name` or `--name=value`, so a file name
//! never has to be guessed to be an option value. `--` ends the options.
//! PotPlayer's switches (`crate::instance::SWITCHES`: `/add`, `/seek=1800`,
//! `/sub="a b.srt"`, `/cd:D`, any case) are options too, their values with
//! the quotes taken off; after `--` they are file names.

use std::collections::BTreeMap;

/// The parsed command line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cli {
    /// Files and URLs to open, in order.
    pub files: Vec<String>,
    /// Options by name, without the leading dashes; flags have an empty value.
    pub options: BTreeMap<String, String>,
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

    #[test]
    fn double_dash_ends_options() {
        let cli = parse(&["--", "--weird name.mkv"]);
        assert_eq!(cli.files, ["--weird name.mkv"]);
        assert!(cli.options.is_empty());
    }
}
