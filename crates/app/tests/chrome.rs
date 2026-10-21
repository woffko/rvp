//! The window's skin logic on a small synthetic skin with a generated atlas
//! (no third-party files): the static layer and dynamic regions, damage
//! after changes, and what mouse input does.

use rvp_app::chrome::{Chrome, Mode};
use rvp_app::playback::SeekHover;
use rvp_app::playlist::Playlists;
use rvp_app::pointer::{Button, Context, EDGE_LEFT, Effect, Modifiers, Pointer};
use rvp_app::{Action, Bindings, view};
use rvp_engine::{PlayerModel, PropertyId, Value};
use rvp_skin::{Image, Rect, Skin, SkinArchive};

fn put(image: &mut Image, rect: Rect, color: [u8; 4]) {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            let offset = image.offset(x, y).expect("inside atlas");
            image.data_mut()[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

/// 64x64 atlas: bar sprites, two buttons with four states each, a bitmap
/// font, slider parts and a label colour.
fn atlas() -> Vec<u8> {
    let mut image = Image::new(64, 64);
    put(&mut image, Rect::new(0, 0, 30, 10), [40, 40, 40, 255]);
    put(&mut image, Rect::new(30, 0, 30, 10), [90, 90, 90, 255]);
    for (state, grey) in [10_u8, 20, 30, 40].iter().enumerate() {
        let x = i32::try_from(state).expect("small") * 8;
        put(&mut image, Rect::new(x, 10, 8, 8), [*grey, *grey, 200, 255]);
        // Pause: another four states.
        put(&mut image, Rect::new(x, 40, 8, 8), [200, *grey, *grey, 255]);
    }
    for cell in 0..22_u8 {
        put(
            &mut image,
            Rect::new(i32::from(cell) * 2, 20, 2, 2),
            [cell * 10, 100, 100, 255],
        );
    }
    put(&mut image, Rect::new(0, 30, 10, 2), [1, 100, 1, 255]);
    put(&mut image, Rect::new(0, 32, 10, 2), [2, 150, 2, 255]);
    put(&mut image, Rect::new(0, 34, 10, 2), [3, 50, 3, 255]);
    put(&mut image, Rect::new(12, 30, 2, 4), [250, 250, 0, 255]);
    put(&mut image, Rect::new(60, 60, 1, 1), [123, 45, 67, 255]);
    image.encode_png().expect("encode atlas")
}

const VIDEO_SKIN: &str = "<Skin Version='1.0'>
<MainSkinArea Image='Atlas.png'>
<Control>
\t<Static Align='Top' Height='10' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10' LeftMargin='4' RightMargin='4' AltImageLeft='30' AltImageTop='0' IsTitleBar='1'>
\t\t<Label CmdID='CMD_TITLE_MESSAGE' ColorX='60' ColorY='60' Left='12' Top='1' Width='-32' Height='8' FontSize='8' />
\t\t<Button Tooltip='Maximize' CmdID='CMD_MAX_WINDOW' Left='-20' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button Tooltip='Restore' CmdID='CMD_RESTORE_WINDOW' Left='-20' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button Tooltip='Exit' CmdID='ID_APP_EXIT' Left='-10' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button Tooltip='Open' CmdID='CMD_POPUP_USER_MENU' UserMenuName='OpenMenu' AltCmdID='CMD_POPUPMENU_SIZE' Left='2' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t</Static>
\t<Static Align='Bottom' Height='20' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10' LeftMargin='4' RightMargin='4' AltImageLeft='30' AltImageTop='0'>
\t\t<Slider CmdID='CMD_TIMEBAR_CTRL' Left='12' Top='0' Width='-40' Height='4' ImageWidth='10' ImageHeight='2' ImageLeft='0' ImageTop='32' BackImageLeft='0' BackImageTop='30' AltImageLeft='0' AltImageTop='34' ThumbWidth='2' ThumbHeight='4' ThumbImageLeft='12' ThumbImageTop='30' />
\t\t<Slider CmdID='CMD_VOLUME_CTRL' Left='-26' Top='0' Width='20' Height='4' ImageWidth='10' ImageHeight='2' ImageLeft='0' ImageTop='32' BackImageLeft='0' BackImageTop='30' ThumbWidth='2' ThumbHeight='4' ThumbImageLeft='12' ThumbImageTop='30' />
\t\t<PicLabel CmdID='CMD_CURRENTTIME_MESSAGE' Left='12' Top='12' Width='40' Height='2' ImageLeft='0' ImageTop='20' ImageWidth='44' ImageHeight='2' />
\t\t<Button Tooltip='Play< >Open' CmdID='CMD_MEDIA_PLAY' AltCmdID='ID_OPEN_FILE' Left='0' Top='8' Width='8' Height='8' ImageLeft='0' ImageTop='10' HoriCenter='1' />
\t\t<Button Tooltip='Pause' CmdID='CMD_MEDIA_PAUSE' Left='0' Top='8' Width='8' Height='8' ImageLeft='0' ImageTop='40' HoriCenter='1' />
\t\t<Button Tooltip='Page' CmdID='CMD_SELECT_PAGE' PageIndex='2' Left='2' Top='8' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button Tooltip='Previous< >30 Sec Backward' CmdID='ID_PROCESS_PREV' AltCmdID='ID_PLAYBACK_30SEC_BACK' Left='-20' Top='8' Width='8' Height='8' ImageLeft='0' ImageTop='10' HoriCenter='1' />
\t</Static>
\t<Static Align='Client' Contain='CMD_VIDEO_WINDOW' />
</Control>
</MainSkinArea>
<MainSkinAreaOscInside Image='Atlas.png'>
<Control>
\t<Static Align='Bottom' Height='20' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10'>
\t\t<Button CmdID='CMD_MEDIA_PLAY' Left='0' Top='8' Width='8' Height='8' ImageLeft='0' ImageTop='10' HoriCenter='1' />
\t\t<Button CmdID='CMD_MEDIA_PAUSE' Left='0' Top='8' Width='8' Height='8' ImageLeft='0' ImageTop='10' HoriCenter='1' />
\t</Static>
\t<Static Align='Client' Contain='CMD_VIDEO_WINDOW' />
</Control>
</MainSkinAreaOscInside>
<ControlBoxSkinArea Image='Atlas.png' Width='60' Height='40'>
<Control>
\t<Static Align='Client' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10'>
\t\t<Button CmdID='ID_CONTROL_BOX_WINDOW' Left='-10' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button CmdID='CMD_SELECT_PAGE' PageIndex='0' Left='2' Top='12' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button CmdID='CMD_SELECT_PAGE' PageIndex='1' Left='2' Top='22' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Static State='CMD_PAGE_CONTROL' PageIndex='0' Left='20' Top='10' Width='38' Height='28'>
\t\t\t<Button CmdID='ID_AUDIO_EQULIZER' DownFlag='1' Left='0' Top='0' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t\t<Slider CmdID='CMD_EQULIZER_0' Vert='1' Left='12' Top='2' Width='4' Height='21' ImageWidth='2' ImageHeight='10' ImageLeft='0' ImageTop='32' BackImageLeft='0' BackImageTop='30' ThumbWidth='2' ThumbHeight='0' />
\t\t</Static>
\t\t<Static State='CMD_PAGE_CONTROL' PageIndex='1' Left='20' Top='10' Width='38' Height='28'>
\t\t\t<Slider CmdID='CMD_COLOR_OFFSET' Left='0' Top='4' Width='21' Height='4' ImageWidth='10' ImageHeight='2' ImageLeft='0' ImageTop='32' BackImageLeft='0' BackImageTop='30' ThumbWidth='0' ThumbHeight='4' />
\t\t\t<Button CmdID='CMD_COLOR_OFFSET_DEFAULT' Left='24' Top='2' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t</Static>
\t</Static>
</Control>
</ControlBoxSkinArea>
</Skin>
";

const WIDTH: i32 = 200;
const HEIGHT: i32 = 120;

fn chrome() -> Chrome {
    let archive = SkinArchive::from_files(
        "Synthetic",
        [
            ("VideoSkin.xml", VIDEO_SKIN.as_bytes().to_vec()),
            ("atlas.png", atlas()),
        ],
    );
    let skin = Skin::load(archive, "VideoSkin.xml").expect("load synthetic skin");
    assert!(skin.warnings().is_empty(), "{:?}", skin.warnings());
    let mut chrome = Chrome::new(Box::leak(Box::new(skin)));
    chrome.data = view::data_model(&PlayerModel::new(), &Playlists::default(), 100.0);
    chrome.resize(Mode::Window, WIDTH, HEIGHT).expect("layout");
    chrome
}

fn center(chrome: &Chrome, command: &str) -> (i32, i32) {
    let rect = chrome
        .layout()
        .and_then(|placed| placed.find_command(command))
        .expect(command)
        .rect;
    (rect.x + rect.w / 2, rect.y + rect.h / 2)
}

/// The topmost region at a control's centre.
fn region_of(chrome: &Chrome, command: &str) -> usize {
    let (x, y) = center(chrome, command);
    chrome
        .frame()
        .regions
        .iter()
        .rposition(|region| region.contains(x, y))
        .expect("control lies in a region")
}

fn window() -> Context {
    Context {
        width: WIDTH,
        height: HEIGHT,
        movable: true,
        chrome_shown: true,
        panel: false,
        turn_view: false,
        modifiers: Modifiers::default(),
        options: rvp_app::pointer::PointerOptions::default(),
    }
}

fn playing_model(seconds: f64) -> PlayerModel {
    let mut model = PlayerModel::new();
    model.set(PropertyId::Path, Some(Value::Text("/a/film.mkv".into())));
    model.set_loaded_path(Some("/a/film.mkv".into()));
    model.set(PropertyId::IdleActive, Some(Value::Flag(false)));
    model.set(PropertyId::Pause, Some(Value::Flag(false)));
    model.set(PropertyId::Duration, Some(Value::Double(600.0)));
    model.set(PropertyId::TimePos, Some(Value::Double(seconds)));
    model
}

fn show(chrome: &mut Chrome, model: &PlayerModel) -> rvp_app::chrome::Damage {
    chrome.data = view::data_model(model, &Playlists::default(), 100.0);
    view::apply_playback_state(&mut chrome.state, model);
    chrome.refresh()
}

#[test]
fn regions_cover_every_control() {
    let chrome = chrome();
    let frame = chrome.frame();
    let placed = chrome.layout().expect("layout");
    for node in &placed.nodes {
        if node.command().is_some() && node.slot().is_none() {
            assert!(
                frame
                    .regions
                    .iter()
                    .any(|region| { region.intersect(&node.clip) == node.clip }),
                "{:?} at {} is in no region",
                node.command(),
                node.clip
            );
        }
    }
    assert_eq!(
        chrome.video_rect(),
        Rect::new(0, 10, WIDTH, HEIGHT - 30),
        "the video slot lies between the bars"
    );
}

#[test]
fn a_layout_at_the_same_size_repaints_everything() {
    let mut chrome = chrome();
    show(&mut chrome, &playing_model(5.0));
    // The regions stay the same (a playlist view's new row height asks
    // for a layout at the same size).
    let damage = chrome.resize(Mode::Window, WIDTH, HEIGHT).expect("layout");
    assert!(damage.static_layer && damage.regions_changed);
    assert_eq!(damage.regions.len(), chrome.frame().regions.len());
    let damage = show(&mut chrome, &playing_model(6.0));
    assert_eq!(
        damage.regions,
        vec![region_of(&chrome, "CMD_CURRENTTIME_MESSAGE")]
    );
}

#[test]
fn time_changes_repaint_only_their_region() {
    let mut chrome = chrome();
    show(&mut chrome, &playing_model(5.0));
    let damage = show(&mut chrome, &playing_model(5.4));
    // Same second and the seek thumb on the same pixel: nothing to paint.
    assert!(damage.is_empty(), "{damage:?}");
    let damage = show(&mut chrome, &playing_model(6.0));
    assert!(!damage.static_layer);
    assert!(!damage.regions_changed);
    assert_eq!(
        damage.regions,
        vec![region_of(&chrome, "CMD_CURRENTTIME_MESSAGE")]
    );
}

#[test]
fn hover_and_activation() {
    let mut chrome = chrome();
    show(&mut chrome, &PlayerModel::new());
    let element = chrome
        .layout()
        .and_then(|placed| placed.find_command("CMD_MEDIA_PLAY"))
        .expect("play")
        .element
        .id;
    chrome.state.hover = Some(element);
    let damage = chrome.refresh();
    assert_eq!(damage.regions, vec![region_of(&chrome, "CMD_MEDIA_PLAY")]);
    assert!(!damage.static_layer);
    // An inactive window swaps the bar sprites: the static layer and every
    // region over the bars repaint.
    chrome.state.window_active = false;
    let damage = chrome.refresh();
    assert!(damage.static_layer);
    assert!(damage.regions.contains(&region_of(&chrome, "ID_APP_EXIT")));
}

#[test]
fn playing_swaps_play_for_pause_in_the_same_region() {
    let mut chrome = chrome();
    show(&mut chrome, &PlayerModel::new());
    let play_region = region_of(&chrome, "CMD_MEDIA_PLAY");
    let damage = show(&mut chrome, &playing_model(10.0));
    assert!(!damage.regions_changed);
    assert!(damage.regions.contains(&play_region));
    assert!(
        chrome
            .layout()
            .and_then(|placed| placed.find_command("CMD_MEDIA_PAUSE"))
            .is_some()
    );
}

#[test]
fn buttons_act_on_release_over_the_same_button() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    let (x, y) = center(&chrome, "CMD_MEDIA_PLAY");
    assert!(
        pointer
            .press(&mut chrome, window(), x, y, Button::Left)
            .is_empty()
    );
    assert!(chrome.state.pressed.is_some());
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), x, y, Button::Left),
        vec![Effect::Action(Action::PlayPause)]
    );
    assert_eq!(chrome.state.pressed, None);
    // Released elsewhere: cancelled.
    pointer.press(&mut chrome, window(), x, y, Button::Left);
    pointer.moved(&mut chrome, window(), x, y - 40);
    assert_eq!(chrome.state.pressed, None);
    assert!(
        pointer
            .release(&mut chrome, &bindings, window(), x, y - 40, Button::Left)
            .is_empty()
    );
}

#[test]
fn menu_and_page_buttons() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    let (x, y) = center(&chrome, "CMD_POPUP_USER_MENU");
    pointer.press(&mut chrome, window(), x, y, Button::Left);
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), x, y, Button::Left),
        vec![Effect::Popup {
            command: "CMD_POPUP_USER_MENU",
            menu: Some("OpenMenu")
        }]
    );
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), x, y, Button::Right),
        vec![Effect::Popup {
            command: "CMD_POPUPMENU_SIZE",
            menu: None
        }]
    );
    let (x, y) = center(&chrome, "CMD_SELECT_PAGE");
    pointer.press(&mut chrome, window(), x, y, Button::Left);
    assert!(
        pointer
            .release(&mut chrome, &bindings, window(), x, y, Button::Left)
            .is_empty()
    );
    assert_eq!(chrome.state.current_page, 2);
}

#[test]
fn right_click_runs_the_alternative_command_or_opens_the_menu() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    let (x, y) = center(&chrome, "CMD_MEDIA_PLAY");
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), x, y, Button::Right),
        vec![Effect::Action(Action::OpenFiles)]
    );
    let (x, y) = center(&chrome, "ID_PROCESS_PREV");
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), x, y, Button::Right),
        vec![Effect::Action(Action::SeekBackward(
            rvp_app::SeekStep::Medium
        ))]
    );
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), 100, 50, Button::Right),
        vec![Effect::Action(Action::ContextMenu)]
    );
}

#[test]
fn hovering_the_seek_bar_asks_for_its_tooltip() {
    let mut chrome = chrome();
    let mut pointer = Pointer::default();
    let rect = chrome
        .layout()
        .and_then(|placed| placed.find_command("CMD_TIMEBAR_CTRL"))
        .expect("seek bar")
        .rect;
    let x = rect.x + (rect.w - 1) / 2;
    match pointer
        .moved(&mut chrome, window(), x, rect.y + 1)
        .as_slice()
    {
        [Effect::SeekHover(Some(hover))] => {
            assert!((hover.fraction - 0.5).abs() < 0.02, "{hover:?}");
            assert_eq!((hover.x, hover.top, hover.held), (x, rect.y, false));
        }
        other => panic!("{other:?}"),
    }
    // The control's own tooltip gives way to the time.
    assert_eq!(pointer.tooltip, None);
    assert_eq!(
        pointer.moved(&mut chrome, window(), 5, 5),
        vec![Effect::SeekHover(None)]
    );
    assert!(pointer.moved(&mut chrome, window(), 6, 5).is_empty());
    pointer.moved(&mut chrome, window(), x, rect.y + 1);
    assert!(
        pointer.leave(&mut chrome),
        "leaving the window leaves the bar"
    );
}

#[test]
fn sliders_follow_the_pointer() {
    let mut chrome = chrome();
    let mut pointer = Pointer::default();
    let rect = chrome
        .layout()
        .and_then(|placed| placed.find_command("CMD_TIMEBAR_CTRL"))
        .expect("seek bar")
        .rect;
    let y = rect.y + 1;
    assert_eq!(
        pointer.press(&mut chrome, window(), rect.x, y, Button::Left),
        vec![Effect::SeekFraction(0.0)]
    );
    // The time tooltip follows the held bar (M11).
    assert_eq!(
        pointer.moved(&mut chrome, window(), rect.right() + 50, y),
        vec![
            Effect::SeekFraction(1.0),
            Effect::SeekHover(Some(SeekHover {
                fraction: 1.0,
                x: rect.right() - 1,
                top: rect.y,
                held: true
            }))
        ]
    );
    // Let go: the seek bar seeks for good there.
    assert_eq!(
        pointer.release(
            &mut chrome,
            &Bindings::potplayer_defaults(),
            window(),
            rect.right() + 50,
            y,
            Button::Left
        ),
        vec![Effect::SeekRelease(1.0)]
    );
    let rect = chrome
        .layout()
        .and_then(|placed| placed.find_command("CMD_VOLUME_CTRL"))
        .expect("volume")
        .rect;
    let mut pointer = Pointer::default();
    let effects = pointer.press(
        &mut chrome,
        window(),
        rect.x + rect.w / 2,
        rect.y + 1,
        Button::Left,
    );
    assert!(
        matches!(effects.as_slice(), [Effect::VolumeFraction(f)] if (f - 0.5).abs() < 0.06),
        "{effects:?}"
    );
}

#[test]
fn a_drag_on_360_video_turns_the_view() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    let vr = Context {
        turn_view: true,
        ..window()
    };
    pointer.press(&mut chrome, vr, 100, 50, Button::Left);
    assert_eq!(
        pointer.moved(&mut chrome, vr, 110, 47),
        vec![Effect::TurnView { dx: 10, dy: -3 }]
    );
    assert_eq!(
        pointer.moved(&mut chrome, vr, 115, 47),
        vec![Effect::TurnView { dx: 5, dy: 0 }]
    );
    // Not a click when it ends.
    assert!(
        pointer
            .release(&mut chrome, &bindings, vr, 115, 47, Button::Left)
            .is_empty()
    );
}

#[test]
fn title_bar_and_video_drag_move_the_window() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    let (x, y) = center(&chrome, "CMD_TITLE_MESSAGE");
    // The title label is not clickable: the title bar under it is.
    pointer.press(&mut chrome, window(), x, y, Button::Left);
    assert_eq!(
        pointer.moved(&mut chrome, window(), x + 10, y),
        vec![Effect::StartMove]
    );
    pointer.release(&mut chrome, &bindings, window(), x + 10, y, Button::Left);
    assert_eq!(
        pointer.double_click(&mut chrome, &bindings, window(), x, y, Button::Left),
        vec![Effect::Action(Action::MaximizeRestore)]
    );
    assert!(
        pointer
            .release(&mut chrome, &bindings, window(), x, y, Button::Left)
            .is_empty()
    );
    // Dragging the video moves the window and is not a click.
    pointer.press(&mut chrome, window(), 100, 50, Button::Left);
    assert_eq!(
        pointer.moved(&mut chrome, window(), 110, 50),
        vec![Effect::StartMove]
    );
    assert!(
        pointer
            .release(&mut chrome, &bindings, window(), 110, 50, Button::Left)
            .is_empty()
    );
    // Not while maximized.
    let maximized = Context {
        movable: false,
        ..window()
    };
    pointer.press(&mut chrome, maximized, 100, 50, Button::Left);
    assert!(pointer.moved(&mut chrome, maximized, 110, 50).is_empty());
}

/// M14's Mouse page options: only the title bar moves the window, drags
/// with modifiers adjust the picture or the subtitles, and the wheel with
/// X1 held is a gesture of its own whose button click is left out.
#[test]
fn mouse_page_options_change_drags_and_the_wheel() {
    use rvp_app::pointer::{DragKind, Modifiers, PointerOptions};
    let mut chrome = chrome();
    let mut bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    let options = PointerOptions {
        drag_title_only: true,
        drag_picture: true,
        drag_subtitles: true,
        x_wheel: true,
    };
    let ctx = Context {
        options,
        ..window()
    };
    // The video no longer moves the window; the title bar still does.
    pointer.press(&mut chrome, ctx, 100, 50, Button::Left);
    assert!(pointer.moved(&mut chrome, ctx, 110, 50).is_empty());
    pointer.release(&mut chrome, &bindings, ctx, 110, 50, Button::Left);
    let (x, y) = center(&chrome, "CMD_TITLE_MESSAGE");
    pointer.press(&mut chrome, ctx, x, y, Button::Left);
    assert_eq!(
        pointer.moved(&mut chrome, ctx, x + 10, y),
        vec![Effect::StartMove]
    );
    pointer.release(&mut chrome, &bindings, ctx, x + 10, y, Button::Left);
    // Alt pans, Ctrl zooms, Shift stretches, Ctrl+Alt moves subtitles.
    for (modifiers, kind) in [
        (
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
            DragKind::Pan,
        ),
        (
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
            DragKind::Zoom,
        ),
        (
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            DragKind::Stretch,
        ),
        (
            Modifiers {
                ctrl: true,
                alt: true,
                ..Modifiers::default()
            },
            DragKind::Subtitles,
        ),
    ] {
        let held = Context { modifiers, ..ctx };
        pointer.press(&mut chrome, held, 100, 50, Button::Left);
        let effects = pointer.moved(&mut chrome, held, 110, 40);
        assert_eq!(effects.len(), 1, "{kind:?}");
        let Effect::Adjust { kind: got, dx, dy } = effects[0] else {
            panic!("{effects:?}");
        };
        assert_eq!(got, kind);
        assert!(dx > 0.0 && dy < 0.0, "{dx} {dy}");
        // Not a click when it ends.
        assert!(
            pointer
                .release(&mut chrome, &bindings, held, 110, 40, Button::Left)
                .is_empty()
        );
    }
    // The wheel with X1 held: its own gesture, and no X1 click after it.
    bindings.mouse.insert(
        rvp_app::MouseGesture::X1WheelUp,
        Action::SeekForward(rvp_app::SeekStep::Short),
    );
    pointer.press(&mut chrome, ctx, 100, 50, Button::Back);
    assert_eq!(
        pointer.wheel(&chrome, &bindings, ctx, (100, 50), 120),
        vec![Effect::Action(Action::SeekForward(
            rvp_app::SeekStep::Short
        ))]
    );
    assert!(
        pointer
            .release(&mut chrome, &bindings, ctx, 100, 50, Button::Back)
            .is_empty()
    );
    // Without the option the wheel is the plain one.
    pointer.press(&mut chrome, window(), 100, 50, Button::Back);
    assert_eq!(
        pointer.wheel(&chrome, &bindings, window(), (100, 50), 120),
        vec![Effect::Action(Action::VolumeUp)]
    );
}

#[test]
fn video_clicks_follow_the_mouse_bindings() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    let mut pointer = Pointer::default();
    pointer.press(&mut chrome, window(), 100, 50, Button::Left);
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), 100, 50, Button::Left),
        vec![Effect::Action(Action::PlayPause)]
    );
    // The second click of a double click undoes play/pause, then goes
    // fullscreen; its release does nothing.
    pointer.press(&mut chrome, window(), 100, 50, Button::Left);
    assert_eq!(
        pointer.double_click(&mut chrome, &bindings, window(), 100, 50, Button::Left),
        vec![
            Effect::Action(Action::PlayPause),
            Effect::Action(Action::ToggleFullscreen)
        ]
    );
    assert!(
        pointer
            .release(&mut chrome, &bindings, window(), 100, 50, Button::Left)
            .is_empty()
    );
    // Middle click and the side buttons.
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), 100, 50, Button::Middle),
        vec![Effect::Action(Action::ToggleFullscreen)]
    );
    assert_eq!(
        pointer.release(&mut chrome, &bindings, window(), 100, 50, Button::Forward),
        vec![Effect::Action(Action::SeekForward(
            rvp_app::SeekStep::Short
        ))]
    );
    // A left click on an empty part of a bar does nothing (video area only).
    pointer.press(&mut chrome, window(), 60, 117, Button::Left);
    assert!(
        pointer
            .release(&mut chrome, &bindings, window(), 60, 117, Button::Left)
            .is_empty()
    );
}

#[test]
fn edges_resize_and_show_resize_cursors() {
    let mut chrome = chrome();
    let mut pointer = Pointer::default();
    assert_eq!(
        pointer.press(&mut chrome, window(), 1, 60, Button::Left),
        vec![Effect::StartResize(EDGE_LEFT)]
    );
    pointer.moved(&mut chrome, window(), 1, 60);
    assert_eq!(pointer.cursor, rvp_app::pointer::Cursor::SizeHorizontal);
    let (x, y) = center(&chrome, "ID_APP_EXIT");
    pointer.moved(&mut chrome, window(), x, y);
    assert_eq!(pointer.cursor, rvp_app::pointer::Cursor::Arrow);
    assert_eq!(pointer.tooltip, Some("Exit"));
}

#[test]
fn fullscreen_uses_the_overlay_bars() {
    let mut chrome = chrome();
    let bindings = Bindings::potplayer_defaults();
    chrome
        .resize(Mode::Fullscreen, WIDTH, HEIGHT)
        .expect("overlay layout");
    assert_eq!(chrome.video_rect(), Rect::new(0, 0, WIDTH, HEIGHT));
    // The bars lie over the video: a video slot there is neither a bar
    // (the pointer over the video would show the bars) nor black.
    assert_eq!(chrome.bar_rects(), vec![Rect::new(0, 100, WIDTH, 20)]);
    let frame = chrome.frame();
    assert!(
        !frame.list.ops.iter().any(|op| matches!(
            op,
            rvp_skin::paint::Op::Fill { rect, .. } if rect.intersect(&Rect::new(0, 0, WIDTH, 100)).w > 0
        )),
        "something covers the video: {:?}",
        frame.list.ops
    );
    let (x, y) = center(&chrome, "CMD_MEDIA_PLAY");
    let mut pointer = Pointer::default();
    let shown = Context {
        movable: false,
        ..window()
    };
    pointer.press(&mut chrome, shown, x, y, Button::Left);
    assert_eq!(
        pointer.release(&mut chrome, &bindings, shown, x, y, Button::Left),
        vec![Effect::Action(Action::PlayPause)]
    );
    // Bars hidden: the same spot is video.
    let hidden = Context {
        chrome_shown: false,
        ..shown
    };
    assert_eq!(
        pointer.double_click(&mut chrome, &bindings, hidden, x, y, Button::Left),
        vec![Effect::Action(Action::ToggleFullscreen)]
    );
}

#[test]
fn the_control_panel_has_its_own_window_pages_and_sliders() {
    let mut chrome = chrome();
    let skin = chrome.skin();
    assert_eq!(Mode::ControlBox.area_size(skin), Some((60, 40)));
    assert_eq!(Mode::Window.area_size(skin), None);
    chrome.resize(Mode::ControlBox, 60, 40).expect("layout");
    let bindings = Bindings::potplayer_defaults();
    let panel = Context {
        width: 60,
        height: 40,
        panel: true,
        turn_view: false,
        ..window()
    };
    let mut pointer = Pointer::default();
    // Page 0: the equalizer's band reads the pointer from the bottom up.
    let slider = chrome
        .layout()
        .and_then(|placed| placed.find_command("CMD_EQULIZER_0"))
        .expect("band slider")
        .rect;
    assert_eq!(slider, Rect::new(32, 12, 4, 21));
    assert_eq!(
        pointer.press(&mut chrome, panel, 33, slider.bottom() - 1, Button::Left),
        vec![Effect::Slider {
            command: "CMD_EQULIZER_0",
            fraction: 0.0
        }]
    );
    assert_eq!(
        pointer.moved(&mut chrome, panel, 40, slider.y + 5),
        vec![Effect::Slider {
            command: "CMD_EQULIZER_0",
            fraction: 0.75
        }]
    );
    assert_eq!(
        pointer.release(
            &mut chrome,
            &bindings,
            panel,
            40,
            slider.y - 3,
            Button::Left
        ),
        vec![Effect::Slider {
            command: "CMD_EQULIZER_0",
            fraction: 1.0
        }]
    );
    assert_eq!(chrome.toggle_commands(), vec!["ID_AUDIO_EQULIZER"]);
    // The second page button shows the colour page.
    pointer.press(&mut chrome, panel, 5, 25, Button::Left);
    assert!(
        pointer
            .release(&mut chrome, &bindings, panel, 5, 25, Button::Left)
            .is_empty()
    );
    assert_eq!(chrome.state.current_page, 1);
    chrome.refresh();
    assert!(chrome.toggle_commands().is_empty());
    let (x, y) = center(&chrome, "CMD_COLOR_OFFSET");
    assert_eq!(
        pointer.press(&mut chrome, panel, x, y, Button::Left),
        vec![Effect::Slider {
            command: "CMD_COLOR_OFFSET",
            fraction: 0.5
        }]
    );
    pointer.release(&mut chrome, &bindings, panel, x, y, Button::Left);
    let (x, y) = center(&chrome, "CMD_COLOR_OFFSET_DEFAULT");
    pointer.press(&mut chrome, panel, x, y, Button::Left);
    assert_eq!(
        pointer.release(&mut chrome, &bindings, panel, x, y, Button::Left),
        vec![Effect::Action(Action::Color(
            rvp_app::video::ColorChange::Reset(rvp_app::video::ColorKind::Brightness)
        ))]
    );
    // The close button hides the panel.
    let (x, y) = center(&chrome, "ID_CONTROL_BOX_WINDOW");
    pointer.press(&mut chrome, panel, x, y, Button::Left);
    assert_eq!(
        pointer.release(&mut chrome, &bindings, panel, x, y, Button::Left),
        vec![Effect::Action(Action::ControlPanel)]
    );
    // No resize border: the corner moves the panel like its background,
    // and a right click is no gesture there.
    assert!(
        pointer
            .press(&mut chrome, panel, 1, 1, Button::Left)
            .is_empty()
    );
    assert_eq!(
        pointer.moved(&mut chrome, panel, 9, 1),
        vec![Effect::StartMove]
    );
    pointer.release(&mut chrome, &bindings, panel, 9, 1, Button::Left);
    pointer.press(&mut chrome, panel, 15, 30, Button::Right);
    assert!(
        pointer
            .release(&mut chrome, &bindings, panel, 15, 30, Button::Right)
            .is_empty()
    );
}

#[test]
fn fullscreen_bars_leave_the_video_slot_out() {
    let mut chrome = chrome();
    assert!(chrome.bars().is_empty(), "the window has a video slot");
    chrome
        .resize(Mode::Fullscreen, WIDTH, HEIGHT)
        .expect("layout");
    // The control bar, not the client part that holds the video.
    assert_eq!(chrome.bars(), vec![Rect::new(0, HEIGHT - 20, WIDTH, 20)]);
}
