//! Golden-image tests (docs/PLAN.md section 7): RVP's idle window, as the
//! application builds it (chrome, data model, hover and pressed states),
//! rendered headless and compared with PotPlayer's R1 captures at the same
//! window sizes.
//!
//! The skin and the captures are third-party and local only: the skin is
//! read from `RVP_TEST_SKIN` (default: the PotPlayer folder seen from WSL),
//! the captures from `RVP_REFERENCE_DIR/r1/session1` (default
//! `~/rvp-reference`). Without them the test prints a note and
//! passes. Masked out: the video area (PotPlayer draws its logo there) and
//! the playlist list (the owner's albums and files). Difference images go
//! to `RVP_REFERENCE_DIR/compare`.

use std::path::PathBuf;

use rvp_app::chrome::{Chrome, Mode};
use rvp_app::view;
use rvp_engine::PlayerModel;
use rvp_skin::compare::{diff_image, flatten};
use rvp_skin::{Fonts, Image, Rect, Skin, rasterize};

const DEFAULT_SKIN: &str = "/mnt/c/Program Files/DAUM/PotPlayer/Skins/BlackSpace_en.dsf";
const DEFAULT_REFERENCE_DIR: &str = "~/rvp-reference";
const SIZES: [(i32, i32); 3] = [(1280, 720), (1920, 1080), (2560, 1080)];
/// Mean difference per channel (0-255) allowed outside the masks.
const TOLERANCE: f64 = 1.5;

fn reference_dir() -> PathBuf {
    std::env::var_os("RVP_REFERENCE_DIR")
        .map_or_else(|| PathBuf::from(DEFAULT_REFERENCE_DIR), PathBuf::from)
}

fn skin() -> Option<&'static Skin> {
    let path = std::env::var_os("RVP_TEST_SKIN")
        .map_or_else(|| PathBuf::from(DEFAULT_SKIN), PathBuf::from);
    if !path.exists() {
        eprintln!("skipped: BlackSpace skin not found at {}", path.display());
        return None;
    }
    let skin = Skin::open(&path).expect("BlackSpace loads");
    Some(Box::leak(Box::new(skin)))
}

/// Mean absolute difference per channel outside `masks`.
fn masked_difference(a: &Image, b: &Image, masks: &[Rect]) -> f64 {
    let width = i32::try_from(a.width().min(b.width())).unwrap_or(0);
    let height = i32::try_from(a.height().min(b.height())).unwrap_or(0);
    let mut sum = 0_u64;
    let mut count = 0_u64;
    for y in 0..height {
        for x in 0..width {
            if masks.iter().any(|mask| mask.contains(x, y)) {
                continue;
            }
            let (first, second) = (a.pixel(x, y), b.pixel(x, y));
            for channel in 0..3 {
                sum += u64::from(first[channel].abs_diff(second[channel]));
            }
            count += 3;
        }
    }
    // Pixel counts stay far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let mean = sum as f64 / count.max(1) as f64;
    mean
}

#[test]
fn idle_window_matches_potplayer_captures() {
    let Some(skin) = skin() else { return };
    let captures = reference_dir().join("r1/session1");
    if !captures.join("idle-1280x720.png").exists() {
        eprintln!("skipped: no R1 captures in {}", captures.display());
        return;
    }
    let mut fonts = Fonts::from_environment();
    let mut failures = Vec::new();
    for (width, height) in SIZES {
        for state in ["", "-hover", "-pressed"] {
            let mut chrome = Chrome::new(skin);
            chrome.data = view::data_model(&PlayerModel::new(), &[]);
            view::apply_playback_state(&mut chrome.state, &PlayerModel::new());
            chrome.resize(Mode::Window, width, height).expect("layout");
            let play = chrome
                .layout()
                .and_then(|placed| placed.find_command("CMD_MEDIA_PLAY"))
                .expect("play button")
                .element
                .id;
            match state {
                "-hover" => chrome.state.hover = Some(play),
                "-pressed" => {
                    chrome.state.hover = Some(play);
                    chrome.state.pressed = Some(play);
                }
                _ => {}
            }
            chrome.refresh();
            let frame = chrome.frame();
            let render = flatten(&rasterize(&frame.list, skin, &mut fonts), [0, 0, 0]);
            let name = format!("idle-{width}x{height}{state}.png");
            let bytes = std::fs::read(captures.join(&name)).expect("capture");
            let capture = flatten(&Image::decode(&name, &bytes).expect("decode"), [0, 0, 0]);
            assert_eq!(
                (capture.width(), capture.height()),
                (render.width(), render.height()),
                "{name}: capture size"
            );
            let placed = chrome.layout().expect("layout");
            let masks: Vec<Rect> = ["CMD_VIDEO_WINDOW", "CMD_PLAY_LIST_ALBUM_TAB"]
                .iter()
                .filter_map(|slot| placed.slot(slot))
                .collect();
            let difference = masked_difference(&render, &capture, &masks);
            eprintln!("{name}: mean difference {difference:.3}");
            let out = reference_dir().join("compare");
            std::fs::create_dir_all(&out).expect("compare directory");
            std::fs::write(
                out.join(format!("golden-diff-{width}x{height}{state}.png")),
                diff_image(&render, &capture, 0, 0)
                    .encode_png()
                    .expect("encode"),
            )
            .expect("write difference");
            if difference >= TOLERANCE {
                failures.push(format!("{name}: {difference:.3}"));
            }
        }
    }
    assert!(failures.is_empty(), "above {TOLERANCE}: {failures:?}");
}
