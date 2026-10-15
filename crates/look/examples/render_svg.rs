//! Renders an SVG file of RVP's art to PNG: `render_svg IN.svg OUT.png SIZE`
//! (the larger side, in pixels).

use std::process::ExitCode;

use rvp_look::svg::Document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output, size] = args.as_slice() else {
        eprintln!("usage: render_svg IN.svg OUT.png SIZE");
        return ExitCode::FAILURE;
    };
    match render(input, output, size) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("render_svg: {message}");
            ExitCode::FAILURE
        }
    }
}

fn render(input: &str, output: &str, size: &str) -> Result<(), String> {
    let size: f32 = size.parse().map_err(|_| format!("bad size {size}"))?;
    let text = std::fs::read_to_string(input).map_err(|error| format!("{input}: {error}"))?;
    let document = Document::parse(&text).map_err(|error| format!("{input}: {error}"))?;
    let [_, _, width, height] = document.view_box()?;
    let scale = size / width.max(height);
    let image = document.render(pixels(width * scale), pixels(height * scale))?;
    let png = image.encode_png().map_err(|error| error.to_string())?;
    std::fs::write(output, png).map_err(|error| format!("{output}: {error}"))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn pixels(value: f32) -> u32 {
    // Sizes on the command line are small and positive.
    value.round().max(1.0) as u32
}
