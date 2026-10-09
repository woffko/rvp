//! Compares a render with a reference screenshot that may be scaled down.
//!
//! ```text
//! cargo run --release -p rvp-skin --example compare_reference -- RENDER.png REFERENCE.png OUT_PREFIX
//! ```
//!
//! Finds the scale and offset that best align the render with the
//! reference, prints the mean difference per region and writes zoomed
//! side-by-side crops (`OUT_PREFIX-<region>.png`, render on the left).

use std::path::Path;
use std::process::ExitCode;

use rvp_skin::compare::{flatten, mean_abs_diff, resize_box};
use rvp_skin::{Image, Rect};

/// Regions in render coordinates: name and fractions of the window
/// (left, top, width, height) or absolute pixels when given as pixels.
const REGIONS: [(&str, Region); 9] = [
    ("title-left", Region::Pixels(0, 0, 400, 40)),
    ("title-right", Region::FromRight(400, 0, 400, 40)),
    ("bottom-left", Region::FromBottom(0, 80, 700, 80)),
    ("bottom-center", Region::CenterBottom(600, 80)),
    ("bottom-right", Region::FromRightBottom(640, 80, 640, 80)),
    ("playlist-top", Region::FromRight(310, 30, 310, 160)),
    ("playlist-middle", Region::FromRight(310, 600, 310, 160)),
    (
        "playlist-bottom",
        Region::FromRightBottom(310, 180, 310, 180),
    ),
    ("video", Region::Pixels(200, 200, 600, 300)),
];

#[derive(Clone, Copy)]
enum Region {
    Pixels(i32, i32, i32, i32),
    FromRight(i32, i32, i32, i32),
    FromBottom(i32, i32, i32, i32),
    FromRightBottom(i32, i32, i32, i32),
    CenterBottom(i32, i32),
}

impl Region {
    fn rect(self, width: i32, height: i32) -> Rect {
        match self {
            Self::Pixels(x, y, w, h) => Rect::new(x, y, w, h),
            Self::FromRight(right, y, w, h) => Rect::new(width - right, y, w, h),
            Self::FromBottom(x, bottom, w, h) => Rect::new(x, height - bottom, w, h),
            Self::FromRightBottom(right, bottom, w, h) => {
                Rect::new(width - right, height - bottom, w, h)
            }
            Self::CenterBottom(w, h) => Rect::new((width - w) / 2, height - h, w, h),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: compare_reference RENDER.png REFERENCE.png OUT_PREFIX");
        return ExitCode::FAILURE;
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("compare_reference: {message}");
            ExitCode::FAILURE
        }
    }
}

fn load(path: &str) -> Result<Image, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    Image::decode(path, &bytes).map_err(|e| e.to_string())
}

fn scaled(value: i32, scale: f64) -> i32 {
    // Window sizes are a few thousand pixels.
    #[allow(clippy::cast_possible_truncation)]
    let result = (f64::from(value) * scale).round() as i32;
    result
}

fn sampled_diff(a: &Image, b: &Image, dx: i32, dy: i32, step: i32) -> f64 {
    let width = i32::try_from(a.width()).unwrap_or(0);
    let height = i32::try_from(a.height()).unwrap_or(0);
    let mut sum = 0.0;
    let mut count = 0.0;
    let mut y = 0;
    while y < height {
        if let Some(value) = mean_abs_diff(a, b, Rect::new(0, y, width, 1), dx, dy) {
            sum += value;
            count += 1.0;
        }
        y += step;
    }
    if count > 0.0 { sum / count } else { f64::MAX }
}

fn run(args: &[String]) -> Result<(), String> {
    let render = flatten(&load(&args[0])?, [48, 10, 36]);
    let reference = flatten(&load(&args[1])?, [0, 0, 0]);
    let render_width = i32::try_from(render.width()).unwrap_or(0);
    let render_height = i32::try_from(render.height()).unwrap_or(0);
    let mut best = (f64::MAX, 1.0, 0, 0, render.clone());
    for step in 0..=20 {
        let scale = 0.880 + f64::from(step) * 0.001;
        let small = resize_box(
            &render,
            u32::try_from(scaled(render_width, scale)).unwrap_or(1),
            u32::try_from(scaled(render_height, scale)).unwrap_or(1),
        );
        for dx in 0..=10 {
            for dy in 0..=18 {
                let value = sampled_diff(&small, &reference, dx, dy, 3);
                if value < best.0 {
                    best = (value, scale, dx, dy, small.clone());
                }
            }
        }
    }
    let (overall, scale, dx, dy, small) = best;
    println!("best scale {scale:.3} offset ({dx}, {dy}) mean difference {overall:.2}");
    for (name, region) in REGIONS {
        let full = region.rect(render_width, render_height);
        let rect = Rect::new(
            scaled(full.x, scale),
            scaled(full.y, scale),
            scaled(full.w, scale),
            scaled(full.h, scale),
        );
        let value = mean_abs_diff(&small, &reference, rect, dx, dy).unwrap_or(f64::NAN);
        println!("{name:16} {value:6.2}");
        write_pair(
            &small,
            &reference,
            rect,
            dx,
            dy,
            &format!("{}-{name}.png", args[2]),
        )?;
    }
    Ok(())
}

/// Writes render and reference crops side by side, three times enlarged.
fn write_pair(
    render: &Image,
    reference: &Image,
    rect: Rect,
    dx: i32,
    dy: i32,
    path: &str,
) -> Result<(), String> {
    const ZOOM: i32 = 3;
    let width = rect.w * 2 + 4;
    let mut out = Image::new(
        u32::try_from(width * ZOOM).unwrap_or(1),
        u32::try_from(rect.h * ZOOM).unwrap_or(1),
    );
    out.fill([255, 0, 255, 255]);
    for y in 0..rect.h * ZOOM {
        for x in 0..width * ZOOM {
            let (column, row) = (x / ZOOM, y / ZOOM);
            let pixel = if column < rect.w {
                render.pixel(rect.x + column, rect.y + row)
            } else if column >= rect.w + 4 {
                reference.pixel(rect.x + column - rect.w - 4 + dx, rect.y + row + dy)
            } else {
                continue;
            };
            if let Some(offset) = out.offset(x, y) {
                out.data_mut()[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
    }
    let png = out.encode_png().map_err(|e| e.to_string())?;
    std::fs::write(Path::new(path), png).map_err(|e| format!("{path}: {e}"))
}
