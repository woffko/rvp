//! Checks against the real BlackSpace skin from a local PotPlayer install.
//!
//! The skin is third-party: it is read from `RVP_TEST_SKIN` (default: the
//! PotPlayer folder as seen from WSL) and never stored in this repository.
//! Without it these tests print a note and pass. Renders and the full
//! geometry snapshots go to `RVP_REFERENCE_DIR` (default
//! `~/rvp-reference`), outside the repository; set `RVP_BLESS=1` to
//! rewrite the snapshots.

use std::path::{Path, PathBuf};

use rvp_skin::compare::{flatten, mean_abs_diff, resize_box};
use rvp_skin::{DataModel, Fonts, Image, Rect, Skin, UiState, demo, layout, render_area};

const DEFAULT_SKIN: &str = "/mnt/c/Program Files/DAUM/PotPlayer/Skins/BlackSpace_en.dsf";
const DEFAULT_REFERENCE_DIR: &str = "~/rvp-reference";
const SIZES: [(i32, i32); 3] = [(1280, 720), (1920, 1080), (2560, 1080)];

/// Window size and screenshot alignment found by `compare_reference` for
/// `screenshots/potplayer-classic-idle.png`: the screenshot shows a
/// 2228x906 window scaled by 0.894, offset by (3, 7).
const SCREENSHOT_WINDOW: (i32, i32) = (2228, 906);
const SCREENSHOT_SCALE: f64 = 0.894;
const SCREENSHOT_OFFSET: (i32, i32) = (3, 7);
/// Mean per-channel difference the screenshot comparison must stay below.
const SCREENSHOT_TOLERANCE: f64 = 2.5;

fn skin() -> Option<Skin> {
    let path = std::env::var_os("RVP_TEST_SKIN")
        .map_or_else(|| PathBuf::from(DEFAULT_SKIN), PathBuf::from);
    if !path.exists() {
        eprintln!("skipped: BlackSpace skin not found at {}", path.display());
        return None;
    }
    Some(Skin::open(&path).expect("BlackSpace loads"))
}

fn reference_dir() -> PathBuf {
    std::env::var_os("RVP_REFERENCE_DIR")
        .map_or_else(|| PathBuf::from(DEFAULT_REFERENCE_DIR), PathBuf::from)
}

fn write_png(path: &Path, image: &Image) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create output directory");
    }
    std::fs::write(path, image.encode_png().expect("encode")).expect("write png");
}

#[test]
fn loads_without_warnings() {
    let Some(skin) = skin() else { return };
    assert!(skin.warnings().is_empty(), "{:?}", skin.warnings());
    assert!(skin.area("MainSkinArea").is_some());
    assert!(skin.area("PlaylistSkinArea").is_some());
    assert!(skin.image("Main.png").is_some());
    assert!(skin.image("ScrollBar.bmp").is_some());
}

#[test]
fn classic_layout_geometry() {
    let Some(skin) = skin() else { return };
    let state = UiState::default();
    for (width, height) in SIZES {
        let placed = layout(&skin, "MainSkinArea", width, height, &state).expect("layout");
        // Frame: 32-pixel title bar, 5-pixel borders, 295-pixel playlist with
        // a 5-pixel splitter, a 71-pixel bottom bar under the video only.
        let playlist = placed.slot("CMD_PLAYLIST_TOGGLE").expect("playlist slot");
        assert_eq!(playlist, Rect::new(width - 300, 32, 295, height - 37));
        let video = placed.slot("CMD_VIDEO_WINDOW").expect("video slot");
        assert_eq!(video, Rect::new(5, 32, width - 310, height - 32 - 5 - 71));
        let close = placed
            .find_command("ID_APP_EXIT")
            .expect("exit button")
            .rect;
        assert_eq!(close, Rect::new(width - 40, 2, 30, 28));
        let play = placed
            .find_command("CMD_MEDIA_PLAY")
            .expect("play button")
            .rect;
        assert_eq!(play.x, 5 + (video.w - 51) / 2);
        assert_eq!(play.y, video.bottom() + 16);
        let title = placed
            .find_command("CMD_TITLE_MESSAGE")
            .expect("title")
            .rect;
        assert_eq!(title, Rect::new(70, 7, width - 170, 14));

        let dump = placed.dump();
        let snapshot = reference_dir().join(format!("snapshots/blackspace-{width}x{height}.txt"));
        if std::env::var_os("RVP_BLESS").is_some() || !snapshot.exists() {
            std::fs::create_dir_all(snapshot.parent().expect("parent")).expect("mkdir");
            std::fs::write(&snapshot, &dump).expect("write snapshot");
        }
        let expected = std::fs::read_to_string(&snapshot).expect("read snapshot");
        assert_eq!(
            dump,
            expected,
            "geometry changed; see {}",
            snapshot.display()
        );
    }
}

#[test]
fn renders_classic_window() {
    let Some(skin) = skin() else { return };
    let mut fonts = Fonts::from_environment();
    let data = DataModel::sample();
    let state = UiState::default();
    for (width, height) in SIZES {
        let image = render_area(
            &skin,
            "MainSkinArea",
            width,
            height,
            &state,
            &data,
            &mut fonts,
        )
        .expect("render");
        assert_eq!(
            (image.width(), image.height()),
            (width.unsigned_abs(), height.unsigned_abs())
        );
        // The window frame is opaque where the skin draws it.
        assert_eq!(image.pixel(width / 2, 1)[3], 255);
        write_png(
            &reference_dir().join(format!("renders/blackspace-classic-{width}x{height}.png")),
            &image,
        );
    }
}

/// Compares a render with the owner's PotPlayer screenshot when both the
/// screenshot and the matching preview content are present locally.
#[test]
fn matches_potplayer_screenshot() {
    let Some(skin) = skin() else { return };
    let directory = reference_dir();
    let screenshot_path = directory.join("screenshots/potplayer-classic-idle.png");
    let demo_path = directory.join("demo-screenshot.txt");
    if !screenshot_path.exists() || !demo_path.exists() {
        eprintln!(
            "skipped: no reference screenshot or demo content in {}",
            directory.display()
        );
        return;
    }
    let preview =
        demo::parse(&std::fs::read_to_string(&demo_path).expect("demo")).expect("parse demo");
    let (width, height) = SCREENSHOT_WINDOW;
    let mut fonts = Fonts::from_environment();
    let render = render_area(
        &skin,
        "MainSkinArea",
        width,
        height,
        &preview.state,
        &preview.data,
        &mut fonts,
    )
    .expect("render");
    write_png(
        &directory.join("renders/blackspace-screenshot-match-2228x906.png"),
        &render,
    );
    let bytes = std::fs::read(&screenshot_path).expect("screenshot");
    let reference = flatten(
        &Image::decode("screenshot", &bytes).expect("decode"),
        [0, 0, 0],
    );
    let scaled = |value: i32| {
        // Window sizes are a few thousand pixels.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let result = (f64::from(value) * SCREENSHOT_SCALE).round() as u32;
        result
    };
    let small = resize_box(
        &flatten(&render, [48, 10, 36]),
        scaled(width),
        scaled(height),
    );
    let region = Rect::new(
        0,
        0,
        i32::try_from(small.width()).unwrap_or(0),
        i32::try_from(small.height()).unwrap_or(0),
    );
    let difference = mean_abs_diff(
        &small,
        &reference,
        region,
        SCREENSHOT_OFFSET.0,
        SCREENSHOT_OFFSET.1,
    )
    .expect("images overlap");
    eprintln!("mean difference to the PotPlayer screenshot: {difference:.2}");
    assert!(
        difference < SCREENSHOT_TOLERANCE,
        "difference {difference:.2}"
    );
}
