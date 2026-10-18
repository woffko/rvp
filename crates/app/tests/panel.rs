//! The control panel (F7) on BlackSpace's `ControlBoxSkinArea`: its size,
//! pages, slider places and what the pointer does there. The skin is
//! third-party and local only: it is read from `RVP_TEST_SKIN` (default:
//! the PotPlayer folder seen from WSL); without it the test prints a note
//! and passes. Each page is rendered to `RVP_REFERENCE_DIR/renders`
//! (default `~/rvp-reference`) for a look.

use std::path::PathBuf;

use rvp_app::chrome::{Chrome, Mode};
use rvp_app::pointer::{Button, Context, Effect, Modifiers, Pointer};
use rvp_app::{Action, Bindings};
use rvp_skin::{DataModel, Fonts, Rect, Skin, SliderValue, rasterize};

const DEFAULT_SKIN: &str = "/mnt/c/Program Files/DAUM/PotPlayer/Skins/BlackSpace_en.dsf";
const DEFAULT_REFERENCE_DIR: &str = "~/rvp-reference";

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

fn rect(chrome: &Chrome, command: &str) -> Rect {
    chrome
        .layout()
        .and_then(|placed| placed.find_command(command))
        .unwrap_or_else(|| panic!("{command} is shown"))
        .rect
}

fn center(rect: Rect) -> (i32, i32) {
    (rect.x + rect.w / 2, rect.y + rect.h / 2)
}

fn render(chrome: &Chrome, skin: &Skin) -> rvp_skin::Image {
    rasterize(
        &rvp_skin::paint(
            skin,
            chrome.layout().expect("layout"),
            &chrome.state,
            &chrome.data,
        ),
        skin,
        &mut Fonts::from_environment(),
    )
}

/// Clicks a control and returns what the release did.
fn click(chrome: &mut Chrome, pointer: &mut Pointer, ctx: Context, command: &str) -> Vec<Effect> {
    let bindings = Bindings::potplayer_defaults();
    let (x, y) = center(rect(chrome, command));
    pointer.press(chrome, ctx, x, y, Button::Left);
    pointer.release(chrome, &bindings, ctx, x, y, Button::Left)
}

/// Clicks the page button of `page` (the five buttons down the left side).
fn show_page(chrome: &mut Chrome, pointer: &mut Pointer, ctx: Context, page: i32) {
    let bindings = Bindings::potplayer_defaults();
    let (x, y) = (30, 40 + 25 * page);
    pointer.press(chrome, ctx, x, y, Button::Left);
    pointer.release(chrome, &bindings, ctx, x, y, Button::Left);
    chrome.refresh();
    assert_eq!(chrome.state.current_page, page);
}

#[test]
fn blackspace_control_panel() {
    let Some(skin) = skin() else { return };
    let (width, height) = Mode::ControlBox.area_size(skin).expect("panel size");
    assert_eq!((width, height), (300, 159));
    let mut chrome = Chrome::new(skin);
    let mut data = DataModel::default();
    for (command, value) in [
        ("CMD_EQULIZER_0", 0.5),
        ("CMD_EQULIZER_3", 0.8),
        ("CMD_EQULIZER_9", 0.2),
        ("CMD_MASTER_VOLUME", 0.6),
        ("CMD_COLOR_OFFSET", 0.6),
        ("CMD_COLOR_GAIN", 0.5),
        ("CMD_COLOR_SAT", 0.4),
        ("CMD_COLOR_HUE", 0.5),
    ] {
        data.sliders.insert(
            command.to_owned(),
            SliderValue {
                value,
                enabled: true,
            },
        );
    }
    data.set_text("CMD_EQ_PRESET_MESSAGE", "Rock");
    data.set_text("CMD_CAPTION_SIZE_MESSAGE", "100%");
    data.set_text("CMD_REPEAT_START_MESSAGE", "00:01:05");
    chrome.data = data;
    chrome.state.flags.insert("ID_AUDIO_EQULIZER".to_owned());
    chrome
        .resize(Mode::ControlBox, width, height)
        .expect("layout");
    let ctx = Context {
        width,
        height,
        movable: true,
        chrome_shown: true,
        panel: true,
        turn_view: false,
        modifiers: Modifiers::default(),
    };
    let mut pointer = Pointer::default();
    let mut renders = vec![audio_page(&mut chrome, &mut pointer, ctx)];
    show_page(&mut chrome, &mut pointer, ctx, 1);
    renders.push(video_page(&mut chrome, &mut pointer, ctx));
    renders.extend(other_pages(&mut chrome, &mut pointer, ctx));

    let dir = std::env::var_os("RVP_REFERENCE_DIR")
        .map_or_else(|| PathBuf::from(DEFAULT_REFERENCE_DIR), PathBuf::from)
        .join("renders");
    if std::fs::create_dir_all(&dir).is_ok() {
        for (page, image) in renders.iter().enumerate() {
            let path = dir.join(format!("control-panel-{page}.png"));
            if let Ok(png) = image.encode_png() {
                let _ = std::fs::write(&path, png);
            }
        }
    }
}

/// Page 0: the volume, then ten bands 16 pixels apart.
fn audio_page(chrome: &mut Chrome, pointer: &mut Pointer, ctx: Context) -> rvp_skin::Image {
    let chrome = &mut *chrome;
    let bindings = Bindings::potplayer_defaults();
    let skin = chrome.skin();
    assert_eq!(rect(chrome, "CMD_MASTER_VOLUME"), Rect::new(83, 82, 14, 57));
    for band in 0..10 {
        let x = 104 + 16 * band;
        assert_eq!(
            rect(chrome, &format!("CMD_EQULIZER_{band}")),
            Rect::new(x, 82, 14, 57)
        );
    }
    let band = rect(chrome, "CMD_EQULIZER_3");
    assert_eq!(
        pointer.press(chrome, ctx, band.x + 7, band.bottom() - 1, Button::Left),
        vec![Effect::Slider {
            command: "CMD_EQULIZER_3",
            fraction: 0.0
        }]
    );
    assert_eq!(
        pointer.release(
            chrome,
            &bindings,
            ctx,
            band.x + 7,
            band.y + 28,
            Button::Left
        ),
        vec![Effect::Slider {
            command: "CMD_EQULIZER_3",
            fraction: 0.5
        }]
    );
    let (x, y) = center(rect(chrome, "CMD_MASTER_VOLUME"));
    assert!(matches!(
        pointer.press(chrome, ctx, x, y, Button::Left).as_slice(),
        [Effect::VolumeFraction(fraction)] if (fraction - 0.5).abs() < 0.02
    ));
    pointer.release(chrome, &bindings, ctx, x, y, Button::Left);
    let toggles = chrome.toggle_commands();
    assert!(toggles.contains(&"ID_AUDIO_EQULIZER"), "{toggles:?}");
    assert_eq!(
        click(chrome, pointer, ctx, "CMD_POPUPMENU_EQULIZER_PRESET"),
        vec![Effect::Popup {
            command: "CMD_POPUPMENU_EQULIZER_PRESET",
            menu: None
        }]
    );
    render(chrome, skin)
}

/// Page 1: colours with their reset buttons, the video effects.
fn video_page(chrome: &mut Chrome, pointer: &mut Pointer, ctx: Context) -> rvp_skin::Image {
    let bindings = Bindings::potplayer_defaults();
    assert_eq!(
        rect(chrome, "CMD_COLOR_OFFSET"),
        Rect::new(103, 79, 140, 12)
    );
    assert_eq!(rect(chrome, "CMD_COLOR_HUE"), Rect::new(103, 124, 140, 12));
    let slider = rect(chrome, "CMD_COLOR_SAT");
    assert_eq!(
        pointer.press(chrome, ctx, slider.right() - 1, slider.y + 6, Button::Left),
        vec![Effect::Slider {
            command: "CMD_COLOR_SAT",
            fraction: 1.0
        }]
    );
    pointer.release(
        chrome,
        &bindings,
        ctx,
        slider.right() - 1,
        slider.y + 6,
        Button::Left,
    );
    for (command, action) in [
        ("ID_VIDEO_SHARPEN", "sharpen"),
        ("CMD_COLOR_HUE_DEFAULT", "hue-reset"),
        ("ID_VIDEO_IMAGE_CAPTURE_PROMPT", "save-frame-as"),
    ] {
        assert_eq!(
            click(chrome, pointer, ctx, command),
            vec![Effect::Action(Action::from_name(action).expect(action))],
            "{command}"
        );
    }
    render(chrome, chrome.skin())
}

/// Pages 2 to 4 (subtitles, playback and the aspect ratio), the close
/// button and a move from the logo strip.
fn other_pages(chrome: &mut Chrome, pointer: &mut Pointer, ctx: Context) -> Vec<rvp_skin::Image> {
    let mut renders = Vec::new();
    for (page, command, action) in [
        (2, "ID_CAPTION_SLOW_SYNC", "subtitle-delay-up"),
        (2, "ID_CAPTION_INC_SIZE", "subtitle-bigger"),
        (3, "ID_AUDIO_FASTER", "audio-delay-down"),
        (3, "ID_PLAY_REPEAT_START_SET", "ab-set-a"),
        (4, "ID_PANSCAN_INCSIZE", "zoom-in"),
    ] {
        if chrome.state.current_page != page {
            show_page(chrome, pointer, ctx, page);
            renders.push(render(chrome, chrome.skin()));
        }
        assert_eq!(
            click(chrome, pointer, ctx, command),
            vec![Effect::Action(Action::from_name(action).expect(action))],
            "{command}"
        );
    }
    assert_eq!(
        rect(chrome, "ID_CONTROL_BOX_WINDOW"),
        Rect::new(259, 3, 35, 22)
    );
    assert_eq!(
        click(chrome, pointer, ctx, "ID_CONTROL_BOX_WINDOW"),
        vec![Effect::Action(Action::ControlPanel)]
    );
    assert!(pointer.press(chrome, ctx, 120, 10, Button::Left).is_empty());
    assert_eq!(pointer.moved(chrome, ctx, 130, 12), vec![Effect::StartMove]);
    renders
}
