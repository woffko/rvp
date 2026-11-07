//! Measures how fast mpv's video filters run on a generated picture, with
//! the calibration's core (`rvp_engine::calibrate`): no window, no sound,
//! up to `calibrate::SPEED` times real time.
//!
//! `cargo run --release -p rvp-engine --example measure_vf -- WxH VF
//! [SECONDS [WARM_UP [TIMEOUT]]]` prints `speed=`, `filter_fps=` and
//! `output_fps=` (the frames a second the filters made). The picture is
//! 23.976 frames a second. A VapourSynth filter needs `VSSCRIPT_PATH`
//! (RVP's runtime). The timeout (seconds, 120 by default) covers the
//! filter's start, when an engine of `TensorRT` is built.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use rvp_engine::calibrate::{Probe, measure};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: measure_vf WxH VF [SECONDS [WARM_UP [TIMEOUT]]]";
    let (Some(size), Some(vf)) = (args.first(), args.get(1)) else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let Some((width, height)) = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let number = |index: usize, default: f64| {
        args.get(index)
            .and_then(|text| text.parse::<f64>().ok())
            .unwrap_or(default)
    };
    let probe = Probe {
        width,
        height,
        rate: (24000, 1001),
        vf: vf.clone(),
        warm_up: Duration::from_secs_f64(number(3, 2.0)),
        measure: Duration::from_secs_f64(number(2, 5.0)),
        extra: Vec::new(),
    };
    let cancel = AtomicBool::new(false);
    match measure(&probe, &cancel, Duration::from_secs_f64(number(4, 120.0))) {
        Ok(measurement) => {
            for error in &measurement.errors {
                eprintln!("mpv: {error}");
            }
            println!(
                "size={width}x{height} speed={:.3} filter_fps={:.3} output_fps={:.1}",
                measurement.speed,
                measurement.filter_fps,
                measurement.speed * measurement.filter_fps
            );
        }
        Err(error) => {
            println!("size={width}x{height} error={error}");
            std::process::exit(1);
        }
    }
}
