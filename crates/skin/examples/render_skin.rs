//! Renders a skin area to PNG.
//!
//! ```text
//! cargo run -p rvp-skin --example render_skin -- SKIN.dsf OUT.png WIDTH HEIGHT [DEMO.txt] [AREA]
//! ```
//!
//! `DEMO.txt` uses the format of `rvp_skin::demo`; without it a generic
//! sample is shown. `AREA` defaults to `MainSkinArea`.

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
    let skin = Skin::open(Path::new(&args[0])).map_err(|e| e.to_string())?;
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
    let area = args.get(5).map_or("MainSkinArea", String::as_str);
    let mut fonts = Fonts::from_environment();
    let image = render_area(&skin, area, width, height, &state, &data, &mut fonts)
        .map_err(|e| e.to_string())?;
    let png = image.encode_png().map_err(|e| e.to_string())?;
    std::fs::write(&args[1], png).map_err(|e| format!("{}: {e}", args[1]))?;
    Ok(())
}
