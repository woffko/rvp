//! RIFE (TensorRT) without RVP's window (M18; `crates/app/src/tensorrt.rs`):
//! installs NVIDIA's files from MIRROR (a folder URL that holds the
//! archives, `file://` too; `-` for the servers RVP downloads from) into
//! FOLDER as RVP does, unless they are there, then measures RVP's own RIFE
//! (TensorRT) script at 720p, 1080p and 1440p with the calibration's core
//! (the first measurement builds the engine, minutes); `install` stops
//! after the installation. `VSSCRIPT_PATH` names RVP's VapourSynth runtime,
//! whose `vs-mlrt` folder the installation takes. The kit's `m18trt` step
//! runs it on the Ubuntu boot.
//!
//! `cargo run --release -p rvp-app --example rife_trt -- FOLDER MIRROR
//! [fast|medium|quality|install]`

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use rvp_app::smoothing::{Engine, Files, Preset, filter_entry, script, tensorrt_package};
use rvp_app::tensorrt::{Event, Installer, Job, archives, installed};
use rvp_engine::calibrate::{Probe, measure};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(folder), Some(mirror)) = (args.first(), args.get(1)) else {
        eprintln!("usage: rife_trt FOLDER MIRROR [fast|medium|quality|install]");
        std::process::exit(2);
    };
    let folder = PathBuf::from(folder);
    let preset = match args.get(2).map(String::as_str) {
        Some("fast") => Preset::Fast,
        Some("quality") => Preset::Quality,
        _ => Preset::Medium,
    };
    let Some(package) = std::env::var_os("VSSCRIPT_PATH")
        .map(PathBuf::from)
        .and_then(|vsscript| tensorrt_package(&vsscript))
    else {
        eprintln!("VSSCRIPT_PATH names no runtime with vs-mlrt");
        std::process::exit(2);
    };
    if installed(&folder).is_none() {
        install(&folder, &package, mirror);
    }
    if let Some(done) = installed(&folder) {
        println!("installation {} for {}", done.version, done.builder);
    }
    if args.get(2).map(String::as_str) == Some("install") {
        return;
    }
    let path = folder.join("rife-trt-check.vpy");
    let files = Files {
        tensorrt: Some((folder.as_path(), package.as_path())),
        ..Files::default()
    };
    std::fs::write(
        &path,
        script(
            Engine::RifeTrt,
            preset,
            (24000, 1001),
            (60, 1),
            &files,
            0,
            None,
        ),
    )
    .expect("script");
    let cancel = AtomicBool::new(false);
    for (width, height) in [(1280, 720), (1920, 1080), (2560, 1440)] {
        let probe = Probe {
            width,
            height,
            rate: (24000, 1001),
            vf: filter_entry(Engine::RifeTrt, preset, (60, 1), &path),
            warm_up: Duration::from_secs(2),
            measure: Duration::from_secs(4),
            extra: Vec::new(),
        };
        match measure(&probe, &cancel, Duration::from_mins(20)) {
            Ok(measurement) => {
                println!(
                    "size={width}x{height} filter_fps={:.3} output_fps={:.1}",
                    measurement.filter_fps,
                    measurement.speed * measurement.filter_fps
                );
                for error in measurement.errors {
                    println!("  mpv: {error}");
                }
            }
            Err(error) => println!("size={width}x{height} error={error}"),
        }
    }
}

/// Installs NVIDIA's files into `folder` as RVP does (`-`: from the
/// archives' own servers); exits on a failure.
fn install(folder: &Path, package: &Path, mirror: &str) {
    let installer = Installer::start(Job {
        folder: folder.to_path_buf(),
        package: package.to_path_buf(),
        archives: archives(),
        mirror: (mirror != "-").then(|| mirror.to_owned()),
        builder: None,
    })
    .expect("installer");
    loop {
        std::thread::sleep(Duration::from_millis(500));
        for event in installer.take() {
            match event {
                Event::Progress { stage, done, total } => {
                    println!("progress {stage:?} {done} of {total}");
                }
                Event::Finished(Ok(done)) => {
                    println!(
                        "installed {} for {} ({} files)",
                        done.version,
                        done.builder,
                        done.files.len()
                    );
                    return;
                }
                Event::Finished(Err(error)) => {
                    println!("install failed: {error}");
                    std::process::exit(1);
                }
                Event::Cancelled => std::process::exit(1),
            }
        }
    }
}
