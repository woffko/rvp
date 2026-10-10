//! Build automation for the RVP workspace.
//!
//! `cargo xtask ci` runs the same checks as the GitHub Actions workflow:
//! formatting, clippy with warnings denied, the test suite and `cargo deny`.
//!
//! The engine links libmpv and the shell links Qt 6: when `MPV_LIB_DIR` is
//! set (a libmpv outside the system paths), and when `QMAKE` names a Qt
//! outside them, their library folders are added to the loader path for
//! the tests.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let result = match args.next().as_deref() {
        Some("ci") => ci(),
        Some("help") | None => {
            println!(
                "xtask commands:\n  ci     run fmt, clippy, tests and cargo deny as CI does\n  help   show this message"
            );
            Ok(())
        }
        Some(other) => Err(format!(
            "unknown xtask command '{other}' (try `cargo xtask help`)"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The folder `qmake -query` reports for `property` (`QT_INSTALL_LIBS`).
fn qt_folder(property: &str) -> Option<PathBuf> {
    let qmake = std::env::var_os("QMAKE")?;
    let output = Command::new(qmake)
        .args(["-query", property])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    Some(PathBuf::from(text.trim())).filter(|path| !path.as_os_str().is_empty())
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    println!("+ {program} {}", args.join(" "));
    let mut command = Command::new(program);
    command.args(args);
    let mut paths: Vec<PathBuf> = std::env::var_os("MPV_LIB_DIR")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    // Qt's DLLs are in bin on Windows, its shared libraries in lib.
    paths.extend(qt_folder(if cfg!(windows) {
        "QT_INSTALL_BINS"
    } else {
        "QT_INSTALL_LIBS"
    }));
    if !paths.is_empty() {
        let variable = if cfg!(windows) {
            "PATH"
        } else {
            "LD_LIBRARY_PATH"
        };
        if let Some(existing) = std::env::var_os(variable) {
            paths.extend(std::env::split_paths(&existing));
        }
        let joined = std::env::join_paths(paths).map_err(|error| error.to_string())?;
        command.env(variable, joined);
    }
    let status = command
        .status()
        .map_err(|error| format!("cannot start {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed with {status}", args.join(" ")))
    }
}

fn ci() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace", "--locked"])?;
    run("cargo", &["deny", "--locked", "check"])?;
    println!("xtask ci: all checks passed");
    Ok(())
}
