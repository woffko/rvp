//! Command line: `rvp [--option[=value]]... [FILE|URL]...`.
//!
//! Options are always written as `--name` or `--name=value`, so a file name
//! never has to be guessed to be an option value. `--` ends the options.
//! PotPlayer's instance switches (`/add`, `/insert`, `/current`, `/new`,
//! any case) are flags too; after `--` they are file names.

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
            } else if let Some(switch) = arg
                .strip_prefix('/')
                .map(str::to_ascii_lowercase)
                .filter(|name| crate::instance::SWITCHES.contains(&name.as_str()))
            {
                cli.options.insert(switch, String::new());
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
    fn double_dash_ends_options() {
        let cli = parse(&["--", "--weird name.mkv"]);
        assert_eq!(cli.files, ["--weird name.mkv"]);
        assert!(cli.options.is_empty());
    }
}
