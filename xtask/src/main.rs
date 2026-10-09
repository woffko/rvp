//! Build automation for the RVP workspace.
//!
//! `cargo xtask ci` runs the same checks as the GitHub Actions workflow:
//! formatting, clippy with warnings denied, the test suite and `cargo deny`.

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

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    println!("+ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
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
