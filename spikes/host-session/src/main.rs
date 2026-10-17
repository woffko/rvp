//! Goal C host-session helpers (Windows only).
//!
//! ```text
//! host-session r1 --potplayer EXE --skin DSF --clip VIDEO --out DIR
//!     [--sizes 1280x720,...] [--states idle,playing|none] [--menu yes|no] [--mouse-page yes|no]
//! host-session s5 --potplayer EXE --out DIR --clip FILE:SECONDS [--clip FILE:SECONDS ...]
//! host-session audio --potplayer EXE --rvp RVP_EXE --files DIR --out DIR [--levels 100,90,...] [--hold-ms N]
//! host-session m10 --potplayer EXE --files DIR --out DIR [--rvp RVP_EXE --second NAME]
//! host-session s3-input --s3 S3_EXE --skin DSF
//! host-session record --out FILE.wav --seconds N
//! host-session check
//! ```
//!
//! `r1` starts PotPlayer, captures its window at each size (idle and
//! playing; plain, hovering the play button and with it pressed), dumps the
//! context menu, reads the mouse page of Preferences, puts the window back
//! where it was and closes PotPlayer. `s5` records the default output device
//! (WASAPI loopback) while PotPlayer plays each clip. `audio` records RVP's
//! and PotPlayer's output of the volume audit's tones: RVP at known volumes
//! with its normalizer on and off, PotPlayer's volume from 100 % down set
//! with its remote messages, and the 5.1 tones in both. `m10` reads
//! PotPlayer's audio settings and records its presets, effects, speaker
//! modes and normalizer blocks for goal L, and RVP's second device
//! (`win/m10.rs`). `s3-input` drags the
//! S3 spike's frameless window with the real mouse. `record` only records;
//! `check` reports monitors, processes and the output device.
//!
//! Only mouse input is injected (it goes to the window under the cursor).
//! Keyboard input would go to the foreground window, which can be the
//! terminal running this tool, so menus, dialogs and commands are driven
//! with window messages instead.

#[cfg(windows)]
mod win;

use std::process::ExitCode;

/// Button and video positions from our own layout of the skin, in window
/// coordinates.
pub struct Targets {
    pub play: (i32, i32),
    pub video: (i32, i32),
}

pub fn skin_targets(skin: &std::path::Path, width: i32, height: i32) -> Result<Targets, String> {
    let skin = rvp_skin::Skin::open(skin).map_err(|e| e.to_string())?;
    let placed = rvp_skin::layout(&skin, "MainSkinArea", width, height, &rvp_skin::UiState::default())
        .map_err(|e| e.to_string())?;
    let centre = |rect: rvp_skin::Rect| (rect.x + rect.w / 2, rect.y + rect.h / 2);
    let play = placed
        .find_command("CMD_MEDIA_PLAY")
        .map(|node| centre(node.rect))
        .ok_or("no CMD_MEDIA_PLAY button in the skin")?;
    let video = placed
        .slot("CMD_VIDEO_WINDOW")
        .or_else(|| placed.find_command("CMD_VIDEO_WINDOW").map(|node| node.rect))
        .map(centre)
        .ok_or("no video slot in the skin")?;
    Ok(Targets { play, video })
}

/// Writes 8-bit RGBA pixels as PNG.
pub fn save_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match win::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            println!("result=ERROR {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() -> ExitCode {
    // The layout part works anywhere; print the targets as a dry run.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [skin, size] = args.as_slice() {
        let (width, height) = size.split_once('x').unwrap_or(("0", "0"));
        match skin_targets(std::path::Path::new(skin), width.parse().unwrap_or(0), height.parse().unwrap_or(0)) {
            Ok(targets) => {
                println!("play={},{} video={},{}", targets.play.0, targets.play.1, targets.video.0, targets.video.1);
                return ExitCode::SUCCESS;
            }
            Err(message) => println!("result=ERROR {message}"),
        }
    } else {
        println!("host-session runs on Windows; on other systems: host-session SKIN.dsf WxH");
    }
    ExitCode::FAILURE
}
