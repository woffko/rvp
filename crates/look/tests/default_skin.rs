//! RVP Default as RVP draws it: the archive loads without warnings, its
//! layout matches the geometry snapshots, and its renders match the golden
//! images at three window sizes, for the control panel's pages and for the
//! fullscreen bars. The art is RVP's own, so the snapshots and golden
//! images are committed (`tests/snapshots`, `tests/golden`); `RVP_BLESS=1`
//! rewrites them after a deliberate change. Renders carry no text: they
//! use no fonts, so they do not depend on the fonts of the machine.

use std::path::PathBuf;

use rvp_look::skin;
use rvp_skin::compare::flatten;
use rvp_skin::demo::{self, Demo};
use rvp_skin::{Fonts, Image, Skin, SkinArchive, layout, render_area};

const SIZES: [(i32, i32); 3] = [(960, 540), (1280, 720), (1920, 1080)];
const PANEL: (i32, i32) = (320, 178);
/// The largest difference of a colour channel two renders of the same
/// skin may show: drawing on another system can round a pixel apart.
const TOLERANCE: u8 = 3;

fn built() -> Skin {
    let bytes = skin::dsf().expect("RVP Default builds");
    let archive = SkinArchive::from_zip_bytes(skin::NAME, bytes).expect("a readable archive");
    Skin::load(archive, "VideoSkin.xml").expect("RVP Default loads")
}

fn preview() -> Demo {
    demo::parse(include_str!("demo.txt")).expect("demo content")
}

fn path(folder: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(folder)
        .join(name)
}

fn bless() -> bool {
    std::env::var_os("RVP_BLESS").is_some()
}

/// Every layout the tests check: (name, area, width, height, page).
fn views() -> Vec<(String, &'static str, i32, i32, i32)> {
    let mut views: Vec<(String, &'static str, i32, i32, i32)> = SIZES
        .iter()
        .map(|&(w, h)| (format!("main-{w}x{h}"), "MainSkinArea", w, h, 0))
        .collect();
    views.push((
        "fullscreen-1280x720".to_owned(),
        "MainSkinAreaOscInside",
        1280,
        720,
        0,
    ));
    views.push((
        "main-video-page-1280x720".to_owned(),
        "MainSkinArea",
        1280,
        720,
        4,
    ));
    for page in 0..5 {
        views.push((
            format!("control-panel-{page}"),
            "ControlBoxSkinArea",
            PANEL.0,
            PANEL.1,
            page,
        ));
    }
    views
}

#[test]
fn loads_without_warnings() {
    let skin = built();
    assert!(skin.warnings().is_empty(), "{:?}", skin.warnings());
    for area in [
        "MainSkinArea",
        "MainSkinAreaOscInside",
        "PlaylistSkinArea",
        "ControlBoxSkinArea",
    ] {
        assert!(skin.area(area).is_some(), "{area}");
    }
    for image in ["Main.png", "PlayList.png", "Control.png", "ScrollBar.bmp"] {
        assert!(skin.image(image).is_some(), "{image}");
    }
    // Its own logo button is marked, so RVP leaves it as drawn.
    assert!(skin.logo_slots().is_empty());
}

#[test]
fn the_archive_is_reproducible() {
    assert_eq!(skin::dsf().expect("first"), skin::dsf().expect("second"));
}

#[test]
fn geometry_matches_the_snapshots() {
    let skin = built();
    let preview = preview();
    for (name, area, width, height, page) in views() {
        let mut state = preview.state.clone();
        state.current_page = page;
        let placed = layout(&skin, area, width, height, &state).expect(area);
        let actual = placed.dump();
        let file = path("snapshots", &format!("{name}.txt"));
        if bless() {
            std::fs::create_dir_all(file.parent().expect("folder")).expect("snapshot folder");
            std::fs::write(&file, &actual).expect("write snapshot");
        }
        let expected = std::fs::read_to_string(&file)
            .unwrap_or_else(|_| panic!("{} is missing; run with RVP_BLESS=1", file.display()));
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn renders_match_the_golden_images() {
    let skin = built();
    let preview = preview();
    let mut fonts = Fonts::new(Vec::new());
    for (name, area, width, height, page) in views() {
        let mut state = preview.state.clone();
        state.current_page = page;
        let image = render_area(
            &skin,
            area,
            width,
            height,
            &state,
            &preview.data,
            &mut fonts,
        )
        .expect(area);
        let file = path("golden", &format!("{name}.png"));
        if bless() {
            std::fs::create_dir_all(file.parent().expect("folder")).expect("golden folder");
            std::fs::write(&file, image.encode_png().expect("encode")).expect("write golden");
        }
        let bytes = std::fs::read(&file)
            .unwrap_or_else(|_| panic!("{} is missing; run with RVP_BLESS=1", file.display()));
        let golden = Image::decode(&name, &bytes).expect("decode golden");
        assert_eq!(
            (golden.width(), golden.height()),
            (image.width(), image.height()),
            "{name}"
        );
        let (actual, expected) = (flatten(&image, [0, 0, 0]), flatten(&golden, [0, 0, 0]));
        let worst = actual
            .data()
            .iter()
            .zip(expected.data())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        assert!(worst <= TOLERANCE, "{name}: a channel differs by {worst}");
    }
}
