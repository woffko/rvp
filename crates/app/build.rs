//! Puts the commit RVP is built from into `RVP_COMMIT`, which the About box,
//! the packages and the crash logs name: `RVP_COMMIT` from the environment
//! (a package built from a source archive), else `git rev-parse`, else
//! "unknown".

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RVP_COMMIT");
    let commit = std::env::var("RVP_COMMIT")
        .ok()
        .filter(|commit| !commit.is_empty())
        .or_else(git_commit)
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=RVP_COMMIT={commit}");
}

/// The short hash of `HEAD`; the build runs again when `HEAD` moves.
fn git_commit() -> Option<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = root.join(".git");
    let head = git.join("HEAD");
    if head.is_file() {
        println!("cargo:rerun-if-changed={}", head.display());
        if let Ok(text) = std::fs::read_to_string(&head)
            && let Some(reference) = text.trim().strip_prefix("ref: ")
        {
            let file = git.join(reference);
            if file.is_file() {
                println!("cargo:rerun-if-changed={}", file.display());
            }
        }
        let packed = git.join("packed-refs");
        if packed.is_file() {
            println!("cargo:rerun-if-changed={}", packed.display());
        }
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "--short=9", "HEAD"])
        .output()
        .ok()?;
    let commit = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (output.status.success() && !commit.is_empty()).then_some(commit)
}
