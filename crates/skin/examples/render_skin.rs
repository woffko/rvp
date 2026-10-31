//! Renders a skin area to PNG.
//!
//! ```text
//! cargo run -p rvp-skin --example render_skin -- SKIN.dsf OUT.png WIDTH HEIGHT [DEMO.txt] [AREA]
//! ```
//!
//! `DEMO.txt` uses the format of `rvp_skin::demo`; without it a generic
//! sample is shown. `AREA` defaults to `MainSkinArea`. A classic Winamp
//! skin (`.wsz`, M9) renders through the skin RVP builds from it, with
//! sample texts for its displays.

use std::path::Path;
use std::process::ExitCode;

use rvp_skin::{DataModel, Fonts, Skin, UiState, demo, render_area};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage: render_skin SKIN.dsf OUT.png WIDTH HEIGHT [DEMO.txt] [AREA]");
        return ExitCode::FAILURE;
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("render_skin: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let path = Path::new(&args[0]);
    let classic = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wsz") || ext.eq_ignore_ascii_case("zip"));
    let skin = if classic {
        let winamp = rvp_skin::winamp::WinampSkin::open(path).map_err(|e| e.to_string())?;
        rvp_skin::winamp::build::skin(&winamp).map_err(|e| e.to_string())?
    } else {
        Skin::open(path).map_err(|e| e.to_string())?
    };
    for warning in skin.warnings() {
        eprintln!("warning: {warning}");
    }
    let width: i32 = args[2].parse().map_err(|_| "WIDTH is not a number")?;
    let height: i32 = args[3].parse().map_err(|_| "HEIGHT is not a number")?;
    let (data, state) = match args.get(4).filter(|path| !path.is_empty()) {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let preview = demo::parse(&text)?;
            (preview.data, preview.state)
        }
        None => (DataModel::sample(), UiState::default()),
    };
    let (mut data, mut state) = (data, state);
    if classic {
        for (command, text) in [
            ("RVP:marquee", "1. RVP FIXTURE 60 S (1:00)"),
            ("RVP:time-minutes", "00"),
            ("RVP:time-seconds", "42"),
            ("RVP:kbps", "128"),
            ("RVP:khz", "44"),
            ("RVP:time-short", "0:42"),
            ("RVP:playlist-info", "0:42/1:00"),
        ] {
            data.set_text(command, text);
        }
        data.visual = (0..16_u8)
            .map(|band| 0.2 + 0.05 * f32::from(band))
            .collect();
        for flag in ["RVP:playing", "RVP:stereo", "RVP:no-mono"] {
            state.flags.insert(flag.to_owned());
        }
        state.media_loaded = true;
    }
    let area = args.get(5).map_or("MainSkinArea", String::as_str);
    let mut fonts = Fonts::from_environment();
    let image = render_area(&skin, area, width, height, &state, &data, &mut fonts)
        .map_err(|e| e.to_string())?;
    let png = image.encode_png().map_err(|e| e.to_string())?;
    std::fs::write(&args[1], png).map_err(|e| format!("{}: {e}", args[1]))?;
    Ok(())
}
