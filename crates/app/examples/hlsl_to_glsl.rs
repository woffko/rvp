//! Translates MPC-HC pixel shaders (PotPlayer's `PxShader` files) into mpv
//! hooks the way RVP does when they load (`rvp_app::hlsl`), to look at the
//! result or to try it in mpv.
//!
//! `cargo run -p rvp-app --example hlsl_to_glsl -- IN... OUT_DIR [pre|post]`

use std::path::{Path, PathBuf};

use rvp_app::hlsl::{Stage, declared_name, translate};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let stage = match args.last().map(String::as_str) {
        Some("pre") => {
            args.pop();
            Stage::Pre
        }
        Some("post") => {
            args.pop();
            Stage::Post
        }
        _ => Stage::Post,
    };
    let Some(out) = args.pop().map(PathBuf::from) else {
        eprintln!("usage: hlsl_to_glsl IN... OUT_DIR [pre|post]");
        std::process::exit(2);
    };
    if let Err(error) = std::fs::create_dir_all(&out) {
        eprintln!("{}: {error}", out.display());
        std::process::exit(1);
    }
    let mut failed = 0;
    for input in &args {
        let path = Path::new(input);
        let stem = path
            .file_stem()
            .map_or_else(|| "shader".to_owned(), |s| s.to_string_lossy().into_owned());
        let result = std::fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|source| {
                let name = declared_name(&source).unwrap_or_else(|| stem.clone());
                translate(&source, stage, &name)
            });
        match result {
            Ok(glsl) => {
                let target = out.join(format!("{stem}.glsl"));
                match std::fs::write(&target, glsl) {
                    Ok(()) => println!("ok {}", target.display()),
                    Err(error) => {
                        failed += 1;
                        println!("fail {stem}: {error}");
                    }
                }
            }
            Err(reason) => {
                failed += 1;
                println!("fail {stem}: {reason}");
            }
        }
    }
    if failed > 0 {
        std::process::exit(1);
    }
}
