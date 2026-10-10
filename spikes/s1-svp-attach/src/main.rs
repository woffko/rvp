//! Spike S1: does SVP 4 Manager attach to a libmpv-based player that is not
//! called `mpv`?
//!
//! ```text
//! s1-svp-attach --input FILE [--ipc PATH] [--seconds 60] [--hwdec auto-copy]
//! ```
//!
//! Plays the file with the options SVP asks for (`input-ipc-server`,
//! `hwdec=auto-copy`, `hr-seek-framedrop=no`) and reports whether a
//! VapourSynth filter appears in `vf` and whether the filtered frame rate
//! rises above the source rate. Start SVP 4 Manager first.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use libmpv2::Mpv;
use libmpv2::events::Event;

fn default_ipc_path() -> String {
    if cfg!(windows) {
        "mpvpipe".to_owned()
    } else {
        "/tmp/mpvsocket".to_owned()
    }
}

fn main() -> ExitCode {
    let mut input = String::new();
    let mut ipc = default_ipc_path();
    let mut seconds = 60.0_f64;
    let mut hwdec = "auto-copy".to_owned();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().unwrap_or_default();
        match arg.as_str() {
            "--input" => input = value,
            "--ipc" => ipc = value,
            "--seconds" => seconds = value.parse().unwrap_or(seconds),
            "--hwdec" => hwdec = value,
            other => {
                println!("result=ERROR unknown argument {other}");
                return ExitCode::FAILURE;
            }
        }
    }
    if input.is_empty() {
        println!("result=ERROR --input is required");
        return ExitCode::FAILURE;
    }
    let created = Mpv::with_initializer(|init| {
        init.set_option("input-ipc-server", ipc.as_str())?;
        init.set_option("hwdec", hwdec.as_str())?;
        init.set_option("hr-seek-framedrop", "no")?;
        init.set_option("vo", "gpu")?;
        init.set_option("force-window", "yes")?;
        init.set_option("loop-file", "inf")?;
        init.set_option("terminal", "yes")?;
        init.set_option("msg-level", "all=warn")?;
        Ok(())
    });
    let mpv = match created {
        Ok(mpv) => mpv,
        Err(error) => {
            println!("result=ERROR mpv initialization failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let property = |name: &str| mpv.get_property::<String>(name).unwrap_or_default();
    println!("pid={}", std::process::id());
    println!("executable={}", std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
    println!("mpv_version={}", property("mpv-version"));
    println!("ipc={ipc}");
    if let Err(error) = mpv.command("loadfile", &[&input]) {
        println!("result=ERROR loadfile failed: {error}");
        return ExitCode::FAILURE;
    }
    let start = Instant::now();
    let mut last_vf = String::new();
    let mut attached_at: Option<f64> = None;
    let mut best_ratio = 0.0_f64;
    while start.elapsed() < Duration::from_secs_f64(seconds) {
        if let Some(Ok(Event::Shutdown)) = mpv.wait_event(0.5) {
            break;
        }
        let vf = property("vf");
        if vf != last_vf {
            println!("t={:.1}s vf={vf}", start.elapsed().as_secs_f64());
            last_vf = vf.clone();
        }
        let source = mpv.get_property::<f64>("container-fps").unwrap_or(0.0);
        let filtered = mpv.get_property::<f64>("estimated-vf-fps").unwrap_or(0.0);
        if vf.contains("vapoursynth") && attached_at.is_none() {
            attached_at = Some(start.elapsed().as_secs_f64());
        }
        if source > 0.0 {
            best_ratio = best_ratio.max(filtered / source);
        }
    }
    println!("container_fps={}", property("container-fps"));
    println!("estimated_vf_fps={}", property("estimated-vf-fps"));
    println!("best_fps_ratio={best_ratio:.2}");
    match attached_at {
        Some(at) => println!("svp_filter_seen_after_s={at:.1}"),
        None => println!("svp_filter_seen_after_s=never"),
    }
    let pass = attached_at.is_some() && best_ratio > 1.5;
    println!("result={}", if pass { "PASS" } else { "FAIL" });
    if pass { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
