//! End-to-end tests on a small synthetic skin that reproduces every quirk
//! found in BlackSpace, with a generated atlas. No third-party files.

use std::collections::BTreeSet;
use std::path::PathBuf;

use rvp_skin::hit::{Hit, hit_test};
use rvp_skin::{
    DataModel, Fonts, Image, Rect, Skin, SkinArchive, SliderValue, UiState, layout, paint,
    rasterize,
};

/// Atlas colours: every sprite is a solid colour so renders can be checked
/// pixel by pixel.
const TITLE_LEFT: [u8; 4] = [200, 0, 0, 255];
const TITLE_MIDDLE: [u8; 4] = [0, 200, 0, 255];
const TITLE_RIGHT: [u8; 4] = [0, 0, 200, 255];
const TITLE_INACTIVE: [u8; 4] = [90, 90, 90, 255];
const BUTTON_STATES: [[u8; 4]; 4] = [
    [10, 10, 10, 255],
    [20, 20, 20, 255],
    [30, 30, 30, 255],
    [40, 40, 40, 255],
];
const SLIDER_BACK: [u8; 4] = [1, 100, 1, 255];
const SLIDER_FILL: [u8; 4] = [2, 150, 2, 255];
const SLIDER_DISABLED: [u8; 4] = [3, 50, 3, 255];
const SLIDER_THUMB: [u8; 4] = [250, 250, 0, 255];
const LABEL_COLOR: [u8; 4] = [123, 45, 67, 255];

fn put(image: &mut Image, rect: Rect, color: [u8; 4]) {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            let offset = image.offset(x, y).expect("inside atlas");
            image.data_mut()[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

/// 64x64 atlas; see the constants above for the sprite colours.
fn atlas() -> Vec<u8> {
    let mut image = Image::new(64, 64);
    put(&mut image, Rect::new(0, 0, 4, 10), TITLE_LEFT);
    put(&mut image, Rect::new(4, 0, 22, 10), TITLE_MIDDLE);
    put(&mut image, Rect::new(26, 0, 4, 10), TITLE_RIGHT);
    put(&mut image, Rect::new(30, 0, 30, 10), TITLE_INACTIVE);
    for (state, color) in BUTTON_STATES.iter().enumerate() {
        let x = i32::try_from(state).expect("small") * 8;
        put(&mut image, Rect::new(x, 10, 8, 8), *color);
    }
    // PicLabel bitmap font: 22 cells of 2 pixels for '%' .. ':', each cell
    // filled with grey level 10 * cell.
    for cell in 0..22_u8 {
        let x = i32::from(cell) * 2;
        put(
            &mut image,
            Rect::new(x, 20, 2, 2),
            [cell * 10, cell * 10, cell * 10, 255],
        );
    }
    // MappingDB strip: three 4-pixel entries.
    put(&mut image, Rect::new(0, 24, 4, 2), [70, 0, 0, 255]);
    put(&mut image, Rect::new(4, 24, 4, 2), [0, 70, 0, 255]);
    put(&mut image, Rect::new(8, 24, 4, 2), [0, 0, 70, 255]);
    put(&mut image, Rect::new(0, 30, 10, 2), SLIDER_BACK);
    put(&mut image, Rect::new(0, 32, 10, 2), SLIDER_FILL);
    put(&mut image, Rect::new(0, 34, 10, 2), SLIDER_DISABLED);
    put(&mut image, Rect::new(12, 30, 2, 4), SLIDER_THUMB);
    // A vertical slider's back and fill: 2x10 strips.
    put(&mut image, Rect::new(40, 30, 2, 10), SLIDER_BACK);
    put(&mut image, Rect::new(42, 30, 2, 10), SLIDER_FILL);
    put(&mut image, Rect::new(60, 60, 1, 1), LABEL_COLOR);
    image.encode_png().expect("encode atlas")
}

/// The main skin file. Quirks covered: XML declaration, a line disabled by
/// deleting `<`, a broken quote, an unquoted value, glued attributes, the
/// `< >` tooltip separator, an include with a macro, `#if`/`#else`, docking
/// order, negative coordinates and sizes, centring, `ConstSize`, paired
/// commands, `State` pages, `VisParentMinWidth`, a skin menu on a right
/// click (`AltUserMenuName`, the `<Menu>` section) and a control panel area
/// with a vertical slider.
const VIDEO_SKIN: &str = "<?XML version='1.0' ?>
<Skin Version='1.0'>
<MainSkinArea Image='Atlas.png'>
<Control>
\t<Static Align='Top' Height='10' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10' LeftMargin='4' RightMargin='4' AltImageLeft='30' AltImageTop='0' IsTitleBar='1'>
\t\t<Button Tooltip='Maximize< >Restore' CmdID='CMD_MAX_WINDOW' Left='-20' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button CmdID='CMD_RESTORE_WINDOW' Left='-20' Top='1' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Label CmdID='CMD_TITLE_MESSAGE' ColorX='60' ColorY='60' HoriAlignment='1' Left='2' Top='1' Width='-22' Height='8' FontSize='8' />
\t</Static>
\tStatic Align='Top' Height='50' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10' />
\t<Static Align='Right' Width='1' ConstSize='25' Contain='CMD_PLAYLIST_TOGGLE' />
\t<Static Align='Bottom' Height='12'>
\t\t<Slider CmdID='CMD_TIMEBAR_CTRL' Left='2' Top='0' Width='-30' Height='4' ImageWidth='10' ImageHeight='2' ImageLeft='0' ImageTop='32' BackImageLeft='0' BackImageTop='30' AltImageLeft='0' AltImageTop='34' ThumbWidth='2' ThumbHeight='4' ThumbImageLeft='12' ThumbImageTop='30' />
\t\t<Slider CmdID='CMD_VOLUME_CTRL' Left='-26' Top='0' Width='20' Height='4' ImageWidth='10' ImageHeight='2' ThumbHeight='4 ImageLeft='99' ImageTop='32' BackImageLeft='0' BackImageTop='30' />
\t\t<PicLabel CmdID='CMD_CURRENTTIME_MESSAGE' Left='2' Top='6' Width='20' Height='2' ImageLeft='0' ImageTop='20' ImageWidth='44' ImageHeight='2' HoriAlignment='0' />
\t\t<PicLabel CmdID='CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE' Left='30' Top='6' Width='4' Height='2' ImageLeft='0' ImageTop='24' ImageWidth='4' ImageHeight='2'MappingDB='2,2.1;3,5.1' />
\t\t<Button Tooltip='Play< >Open' CmdID='CMD_MEDIA_PLAY' AltCmdID=ID_OPEN_FILE Left='0' Top='2' Width='8' Height='8' ImageLeft='0' ImageTop='10' HoriCenter='1' />
\t\t<Button CmdID='CMD_MEDIA_PAUSE' Left='0' Top='2' Width='8' Height='8' ImageLeft='0' ImageTop='10' HoriCenter='1' />
\t\t<Button CmdID='ID_AUDIO_EQULIZER' AltCmdID='CMD_POPUP_USER_MENU' AltUserMenuName='EQMenu' DownFlag='1' Left='-12' Top='2' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button CmdID='ID_WIDE_ONLY' Left='40' Top='2' Width='8' Height='8' ImageLeft='0' ImageTop='10' VisParentMinWidth='500' />
\t\t<Static State='CMD_PAGE_CONTROL' PageIndex='4' Left='50' Top='2' Width='8' Height='8' />
\t</Static>
\t#include<Part.txt #KIND=Button>
\t<Static Align='Client' Contain='CMD_VIDEO_WINDOW' />
</Control>
</MainSkinArea>
<ControlBoxSkinArea Image='Atlas.png' Width='20' Height='30'>
<Control>
\t<Static Align='Client'>
\t\t<Slider CmdID='CMD_EQULIZER_0' Vert='1' Left='4' Top='2' Width='2' Height='20' ImageWidth='2' ImageHeight='10' ImageLeft='42' ImageTop='30' BackImageLeft='40' BackImageTop='30' ThumbWidth='2' ThumbHeight='0' />
\t</Static>
</Control>
</ControlBoxSkinArea>
<Menu>
<UserMenu Name='EQMenu' >
\t<MenuItem CmdID='ID_AUDIO_EQULIZER' />
\t<MenuItem CmdID='' />
\t<MenuItem CmdID='CMD_POPUPMENU_AUDIO_EQULIZER' NoSubMenu='1' />
\tMenuItem CmdID='ID_DISABLED_ITEM' />
</UserMenu>
</Menu>
#if<SkinName==VideoSkin>
<Marker Value='video' />
#else
<Marker Value='other' />
#endif
</Skin>
";

/// Included with `#KIND=Button`; stored as UTF-16 LE with a byte order mark.
const PART: &str = "<#KIND CmdID='ID_FROM_INCLUDE' Left='0' Top='0' Width='4' Height='4' />";

fn utf16_with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

fn skin() -> Skin {
    let archive = SkinArchive::from_files(
        "Synthetic",
        [
            ("VideoSkin.xml", VIDEO_SKIN.as_bytes().to_vec()),
            ("part.txt", utf16_with_bom(PART)),
            ("atlas.png", atlas()),
        ],
    );
    let skin = Skin::load(archive, "VideoSkin.xml").expect("load synthetic skin");
    assert!(skin.warnings().is_empty(), "{:?}", skin.warnings());
    skin
}

fn data() -> DataModel {
    let mut data = DataModel::default();
    data.set_text("CMD_CURRENTTIME_MESSAGE", "0:");
    data.set_text("CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE", "5.1");
    data.sliders.insert(
        "CMD_TIMEBAR_CTRL".to_owned(),
        SliderValue {
            value: 0.5,
            enabled: true,
        },
    );
    data
}

fn render(skin: &Skin, state: &UiState) -> Image {
    let placed = layout(skin, "MainSkinArea", 100, 60, state).expect("layout");
    let list = paint(skin, &placed, state, &data());
    rasterize(&list, skin, &mut Fonts::new(Vec::new()))
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

#[test]
fn geometry_matches_snapshot() {
    let skin = skin();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &UiState::default()).expect("layout");
    let actual = placed.dump();
    let path = snapshot_path("synthetic-100x60.txt");
    if std::env::var_os("RVP_BLESS").is_some() {
        std::fs::write(&path, &actual).expect("write snapshot");
    }
    let expected = std::fs::read_to_string(&path).expect("snapshot exists; run with RVP_BLESS=1");
    assert_eq!(actual, expected);
}

#[test]
fn docking_and_coordinates() {
    let skin = skin();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &UiState::default()).expect("layout");
    let rect = |command: &str| placed.find_command(command).expect(command).rect;
    // Docked in document order: title on top, playlist on the right, then
    // the bottom bar under the remaining width only.
    assert_eq!(
        placed.slot("CMD_PLAYLIST_TOGGLE"),
        Some(Rect::new(75, 10, 25, 50))
    );
    assert_eq!(
        placed.slot("CMD_VIDEO_WINDOW"),
        Some(Rect::new(0, 10, 75, 38))
    );
    // Negative Left counts from the right edge; HoriCenter centres.
    assert_eq!(rect("CMD_MAX_WINDOW"), Rect::new(80, 1, 8, 8));
    assert_eq!(rect("CMD_MEDIA_PLAY"), Rect::new(33, 50, 8, 8));
    // Width -30 ends 30 pixels before the parent's right edge.
    assert_eq!(rect("CMD_TIMEBAR_CTRL"), Rect::new(2, 48, 43, 4));
    // The disabled line added nothing and the include expanded its macro.
    assert_eq!(rect("ID_FROM_INCLUDE"), Rect::new(0, 0, 4, 4));
    assert_eq!(
        skin.root()
            .find("Marker")
            .and_then(|m| m.attrs.get("Value")),
        Some("video")
    );
}

#[test]
fn visibility_rules() {
    let skin = skin();
    let mut state = UiState::default();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &state).expect("layout");
    let has = |placed: &rvp_skin::Layout<'_>, command: &str| placed.find_command(command).is_some();
    assert!(has(&placed, "CMD_MAX_WINDOW") && !has(&placed, "CMD_RESTORE_WINDOW"));
    assert!(has(&placed, "CMD_MEDIA_PLAY") && !has(&placed, "CMD_MEDIA_PAUSE"));
    assert!(!has(&placed, "ID_WIDE_ONLY"), "parent is narrower than 500");
    let pages = |placed: &rvp_skin::Layout<'_>| {
        placed
            .nodes
            .iter()
            .filter(|node| node.element.attrs.get("PageIndex") == Some("4"))
            .count()
    };
    assert_eq!(pages(&placed), 0);
    state.playing = true;
    state.maximized = true;
    state.current_page = 4;
    state.playlist_attached = false;
    let placed = layout(&skin, "MainSkinArea", 100, 60, &state).expect("layout");
    assert!(has(&placed, "CMD_RESTORE_WINDOW") && !has(&placed, "CMD_MAX_WINDOW"));
    assert!(has(&placed, "CMD_MEDIA_PAUSE") && !has(&placed, "CMD_MEDIA_PLAY"));
    assert_eq!(pages(&placed), 1);
    assert_eq!(placed.slot("CMD_PLAYLIST_TOGGLE"), None);
    assert_eq!(
        placed.slot("CMD_VIDEO_WINDOW"),
        Some(Rect::new(0, 10, 100, 38))
    );
}

#[test]
fn broken_quote_drops_the_next_attribute() {
    let skin = skin();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &UiState::default()).expect("layout");
    let volume = placed.find_command("CMD_VOLUME_CTRL").expect("volume");
    assert_eq!(volume.element.attrs.int("ThumbHeight"), Some(4));
    assert_eq!(volume.element.attrs.get("ImageLeft"), None);
    assert_eq!(volume.element.attrs.int("ImageTop"), Some(32));
    let channels = placed
        .find_command("CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE")
        .expect("channels");
    assert_eq!(channels.element.attrs.get("MappingDB"), Some("2,2.1;3,5.1"));
    let play = placed.find_command("CMD_MEDIA_PLAY").expect("play");
    assert_eq!(play.element.attrs.get("AltCmdID"), Some("ID_OPEN_FILE"));
}

#[test]
fn nine_slice_and_inactive_window() {
    let skin = skin();
    let mut state = UiState::default();
    let image = render(&skin, &state);
    assert_eq!(image.pixel(0, 5), TITLE_LEFT);
    assert_eq!(image.pixel(3, 5), TITLE_LEFT);
    assert_eq!(image.pixel(4, 5), TITLE_MIDDLE);
    assert_eq!(image.pixel(50, 5), TITLE_MIDDLE);
    assert_eq!(image.pixel(96, 5), TITLE_RIGHT);
    assert_eq!(image.pixel(99, 5), TITLE_RIGHT);
    state.window_active = false;
    let image = render(&skin, &state);
    assert_eq!(image.pixel(50, 5), TITLE_INACTIVE);
}

#[test]
fn button_states() {
    let skin = skin();
    let mut state = UiState::default();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &state).expect("layout");
    let play = placed
        .find_command("CMD_MEDIA_PLAY")
        .expect("play")
        .element
        .id;
    assert_eq!(render(&skin, &state).pixel(35, 52), BUTTON_STATES[0]);
    state.hover = Some(play);
    assert_eq!(render(&skin, &state).pixel(35, 52), BUTTON_STATES[1]);
    state.pressed = Some(play);
    assert_eq!(render(&skin, &state).pixel(35, 52), BUTTON_STATES[2]);
    // A DownFlag button shows the pressed state while its command is on.
    let equalizer = Rect::new(63, 50, 8, 8);
    assert_eq!(
        render(&skin, &state).pixel(equalizer.x + 1, equalizer.y + 1),
        BUTTON_STATES[0]
    );
    state.flags = BTreeSet::from(["ID_AUDIO_EQULIZER".to_owned()]);
    assert_eq!(
        render(&skin, &state).pixel(equalizer.x + 1, equalizer.y + 1),
        BUTTON_STATES[2]
    );
}

#[test]
fn bitmap_labels_and_sliders() {
    let skin = skin();
    // The seek bar shows only with media loaded.
    let loaded = UiState {
        media_loaded: true,
        ..UiState::default()
    };
    let image = render(&skin, &loaded);
    // "0:" uses cells 11 and 21 of the bitmap font, 2 pixels each.
    assert_eq!(image.pixel(2, 54), [110, 110, 110, 255]);
    assert_eq!(image.pixel(4, 54), [210, 210, 210, 255]);
    // MappingDB "5.1" is the third entry.
    assert_eq!(image.pixel(30, 54), [0, 0, 70, 255]);
    // Seek bar at 50%: fill on the left half, back image on the right,
    // thumb in the middle.
    assert_eq!(image.pixel(3, 49), SLIDER_FILL);
    assert_eq!(image.pixel(40, 49), SLIDER_BACK);
    assert_eq!(image.pixel(23, 49), SLIDER_THUMB);
    // The volume slider lost ImageLeft, so its fill comes from x = 0.
    let mut data = data();
    data.sliders.insert(
        "CMD_TIMEBAR_CTRL".to_owned(),
        SliderValue {
            value: 0.0,
            enabled: false,
        },
    );
    // Loaded but not seekable: the AltImage track.
    let state = UiState {
        media_loaded: true,
        ..UiState::default()
    };
    let placed = layout(&skin, "MainSkinArea", 100, 60, &state).expect("layout");
    let image = rasterize(
        &paint(&skin, &placed, &state, &data),
        &skin,
        &mut Fonts::new(Vec::new()),
    );
    assert_eq!(image.pixel(10, 49), SLIDER_DISABLED);
    // Nothing loaded: no seek bar at all.
    let state = UiState::default();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &state).expect("layout");
    let list = paint(&skin, &placed, &state, &data);
    let seek_bar = placed
        .find_command("CMD_TIMEBAR_CTRL")
        .expect("seek bar")
        .rect;
    assert!(list.ops.iter().all(|op| match op {
        rvp_skin::paint::Op::Image { dst, .. } => dst.intersect(&seek_bar) != *dst,
        _ => true,
    }));
}

#[test]
fn seek_bar_marks_lie_under_the_thumb() {
    use rvp_skin::paint::Op;
    use rvp_skin::{Mark, MarkKind};
    let skin = skin();
    let state = UiState {
        media_loaded: true,
        ..UiState::default()
    };
    let placed = layout(&skin, "MainSkinArea", 100, 60, &state).expect("layout");
    let bar = placed
        .find_command("CMD_TIMEBAR_CTRL")
        .expect("seek bar")
        .rect;
    let mut data = data();
    data.marks = vec![
        Mark {
            at: 0.0,
            kind: MarkKind::Chapter,
        },
        Mark {
            at: 0.25,
            kind: MarkKind::Bookmark,
        },
        Mark {
            at: 0.5,
            kind: MarkKind::PointA,
        },
        Mark {
            at: 1.0,
            kind: MarkKind::PointB,
        },
    ];
    let list = paint(&skin, &placed, &state, &data);
    let fills: Vec<(Rect, [u8; 4])> = list
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Fill { rect, color } if rect.intersect(&bar) == *rect => Some((*rect, *color)),
            _ => None,
        })
        .collect();
    // 43 pixels wide: positions 0, 11 (0.25 of 42, rounded), 21 and 42;
    // the band from A to B first, then the lines.
    assert_eq!(fills[0].0, Rect::new(bar.x + 21, bar.y, 22, bar.h));
    assert_eq!(fills[1].0, Rect::new(bar.x, bar.y, 1, bar.h));
    assert_eq!(fills[2].0, Rect::new(bar.x + 11, bar.y, 1, bar.h));
    assert_eq!(fills[3].0, Rect::new(bar.x + 21, bar.y, 2, bar.h));
    // B's line is clipped at the bar's end.
    assert_eq!(fills[4].0, Rect::new(bar.x + 42, bar.y, 1, bar.h));
    assert_eq!(fills.len(), 5);
    // The thumb is drawn after the marks.
    let last_mark = list
        .ops
        .iter()
        .rposition(|op| matches!(op, Op::Fill { rect, .. } if rect.intersect(&bar) == *rect))
        .expect("marks");
    let thumb = list
        .ops
        .iter()
        .rposition(
            |op| matches!(op, Op::Image { dst, .. } if dst.w == 2 && dst.intersect(&bar) == *dst),
        )
        .expect("thumb");
    assert!(thumb > last_mark);
}

#[test]
fn vertical_sliders_fill_from_the_bottom() {
    let skin = skin();
    let mut data = DataModel::default();
    data.sliders.insert(
        "CMD_EQULIZER_0".to_owned(),
        SliderValue {
            value: 0.25,
            enabled: true,
        },
    );
    let state = UiState::default();
    let placed = layout(&skin, "ControlBoxSkinArea", 20, 30, &state).expect("layout");
    let slider = placed.find_command("CMD_EQULIZER_0").expect("slider").rect;
    assert_eq!(slider, Rect::new(4, 2, 2, 20));
    let image = rasterize(
        &paint(&skin, &placed, &state, &data),
        &skin,
        &mut Fonts::new(Vec::new()),
    );
    // 25 % of 20 pixels: the bottom 5 rows are filled, the rest is track.
    assert_eq!(image.pixel(4, 21), SLIDER_FILL);
    assert_eq!(image.pixel(5, 17), SLIDER_FILL);
    assert_eq!(image.pixel(4, 16), SLIDER_BACK);
    assert_eq!(image.pixel(4, 2), SLIDER_BACK);
}

#[test]
fn hit_testing() {
    let skin = skin();
    let placed = layout(&skin, "MainSkinArea", 100, 60, &UiState::default()).expect("layout");
    match hit_test(&placed, 84, 4) {
        Some(Hit::Control {
            command, tooltip, ..
        }) => {
            assert_eq!(command, Some("CMD_MAX_WINDOW"));
            assert_eq!(tooltip, Some("Maximize< >Restore"));
        }
        other => panic!("unexpected hit {other:?}"),
    }
    assert!(matches!(
        hit_test(&placed, 50, 4),
        Some(Hit::TitleBar { .. })
    ));
    assert!(matches!(
        hit_test(&placed, 30, 30),
        Some(Hit::Slot {
            name: "CMD_VIDEO_WINDOW",
            ..
        })
    ));
    assert_eq!(hit_test(&placed, 200, 200), None);
}

#[test]
fn skin_menus_and_commands() {
    let skin = skin();
    let menus = skin.user_menus();
    assert_eq!(menus.len(), 1);
    assert_eq!(menus[0].name, "EQMenu");
    let items: Vec<(&str, bool)> = menus[0]
        .items
        .iter()
        .map(|item| (item.command.as_str(), item.inline))
        .collect();
    assert_eq!(
        items,
        [
            ("ID_AUDIO_EQULIZER", false),
            ("", false),
            ("CMD_POPUPMENU_AUDIO_EQULIZER", true)
        ]
    );
    let commands = skin.commands();
    assert!(commands.contains_key("CMD_POPUP_USER_MENU"));
    assert!(commands.contains_key("ID_FROM_INCLUDE"));
    assert!(!commands.contains_key("ID_DISABLED_ITEM"));
    // The line disabled by deleting `<` names no command.
    assert_eq!(commands["ID_AUDIO_EQULIZER"].count, 2);

    // The right click opens the skin menu.
    let placed = layout(&skin, "MainSkinArea", 100, 60, &UiState::default()).expect("layout");
    let button = placed
        .nodes
        .iter()
        .find(|node| node.element.attrs.get("CmdID") == Some("ID_AUDIO_EQULIZER"))
        .expect("equalizer button");
    let (x, y) = (button.rect.x + 1, button.rect.y + 1);
    match hit_test(&placed, x, y) {
        Some(Hit::Control {
            alt_command,
            alt_menu,
            menu,
            page,
            ..
        }) => {
            assert_eq!(alt_command, Some("CMD_POPUP_USER_MENU"));
            assert_eq!(alt_menu, Some("EQMenu"));
            assert_eq!((menu, page), (None, None));
        }
        other => panic!("unexpected hit {other:?}"),
    }
}

/// D10: RVP draws its own picture on a title bar's logo button
/// (`CMD_POPUPMENU_ETC`) and leaves buttons marked `RvpLogo='1'` alone.
#[test]
fn logo_buttons_take_a_replacement_sprite() {
    let markup = "<Skin><MainSkinArea Image='Atlas.png'><Control>
\t<Static Align='Top' Height='10' IsTitleBar='1'>
\t\t<Button CmdID='CMD_POPUPMENU_ETC' Left='2' Top='1' Width='6' Height='8' ImageLeft='0' ImageTop='10' />
\t\t<Button CmdID='CMD_POPUPMENU_ETC' RvpLogo='1' Left='20' Top='1' Width='6' Height='8' ImageLeft='0' ImageTop='10' />
\t</Static>
\t<Static Align='Client'><Button CmdID='CMD_POPUPMENU_ETC' Left='0' Top='0' Width='6' Height='8' ImageLeft='0' ImageTop='10' /></Static>
</Control></MainSkinArea></Skin>";
    let archive = SkinArchive::from_files(
        "Logo",
        [
            ("VideoSkin.xml", markup.as_bytes().to_vec()),
            ("atlas.png", atlas()),
        ],
    );
    let mut skin = Skin::load(archive, "VideoSkin.xml").expect("load");
    let slots = skin.logo_slots();
    assert_eq!(slots.len(), 1, "{slots:?}");
    assert_eq!((slots[0].width, slots[0].height), (6, 8));
    // Normal state red, hover state blue, as RVP's strip would hold them.
    let mut strip = Image::new(24, 8);
    put(&mut strip, Rect::new(0, 0, 6, 8), [255, 0, 0, 255]);
    put(&mut strip, Rect::new(6, 0, 6, 8), [0, 0, 255, 255]);
    skin.replace_sprite(slots[0].element, strip);
    let mut state = UiState::default();
    let draw = |skin: &Skin, state: &UiState| {
        let placed = layout(skin, "MainSkinArea", 40, 20, state).expect("layout");
        let list = paint(skin, &placed, state, &DataModel::default());
        rasterize(&list, skin, &mut Fonts::new(Vec::new()))
    };
    assert_eq!(draw(&skin, &state).pixel(4, 4), [255, 0, 0, 255]);
    // The marked button keeps the atlas sprite (button state colour).
    assert_eq!(draw(&skin, &state).pixel(22, 4), BUTTON_STATES[0]);
    state.hover = Some(slots[0].element);
    assert_eq!(draw(&skin, &state).pixel(4, 4), [0, 0, 255, 255]);
}

/// A page button with `DownFlag` shows its pressed state while its page
/// shows (RVP Default's control panel tabs).
#[test]
fn page_toggles_show_the_current_page() {
    let markup = "<Skin><ControlBoxSkinArea Image='Atlas.png' Width='20' Height='10'><Control>
\t<Button CmdID='CMD_SELECT_PAGE' PageIndex='0' DownFlag='1' Left='0' Top='0' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
\t<Button CmdID='CMD_SELECT_PAGE' PageIndex='1' DownFlag='1' Left='10' Top='0' Width='8' Height='8' ImageLeft='0' ImageTop='10' />
</Control></ControlBoxSkinArea></Skin>";
    let archive = SkinArchive::from_files(
        "Pages",
        [
            ("VideoSkin.xml", markup.as_bytes().to_vec()),
            ("atlas.png", atlas()),
        ],
    );
    let skin = Skin::load(archive, "VideoSkin.xml").expect("load");
    let mut state = UiState::default();
    let draw = |state: &UiState| {
        let placed = layout(&skin, "ControlBoxSkinArea", 20, 10, state).expect("layout");
        let list = paint(&skin, &placed, state, &DataModel::default());
        rasterize(&list, &skin, &mut Fonts::new(Vec::new()))
    };
    let image = draw(&state);
    assert_eq!(image.pixel(1, 1), BUTTON_STATES[2]);
    assert_eq!(image.pixel(11, 1), BUTTON_STATES[0]);
    state.current_page = 1;
    let image = draw(&state);
    assert_eq!(image.pixel(1, 1), BUTTON_STATES[0]);
    assert_eq!(image.pixel(11, 1), BUTTON_STATES[2]);
}

/// The video slot is black under the video in the window and draws
/// nothing in `MainSkinAreaOscInside`, whose bars lie over the video in
/// fullscreen.
#[test]
fn the_fullscreen_video_slot_draws_nothing() {
    let markup = "<Skin><MainSkinArea Image='Atlas.png'><Control>
\t<Static Align='Client' Contain='CMD_VIDEO_WINDOW' />
</Control></MainSkinArea><MainSkinAreaOscInside Image='Atlas.png'><Control>
\t<Static Align='Bottom' Height='4' ImageLeft='0' ImageTop='0' ImageWidth='30' ImageHeight='10' />
\t<Static Align='Client' Contain='CMD_VIDEO_WINDOW' />
</Control></MainSkinAreaOscInside></Skin>";
    let archive = SkinArchive::from_files(
        "Slot",
        [
            ("VideoSkin.xml", markup.as_bytes().to_vec()),
            ("atlas.png", atlas()),
        ],
    );
    let skin = Skin::load(archive, "VideoSkin.xml").expect("load");
    let state = UiState::default();
    let draw = |area: &str| {
        let placed = layout(&skin, area, 20, 20, &state).expect("layout");
        let list = paint(&skin, &placed, &state, &DataModel::default());
        rasterize(&list, &skin, &mut Fonts::new(Vec::new()))
    };
    assert_eq!(draw("MainSkinArea").pixel(10, 10), [0, 0, 0, 255]);
    let fullscreen = draw("MainSkinAreaOscInside");
    assert_eq!(fullscreen.pixel(10, 10)[3], 0, "the video shows through");
    assert_eq!(fullscreen.pixel(1, 18), TITLE_LEFT, "the bar still draws");
}

/// A skin's `<Logo>` section: its idle part shows in state
/// `CMD_DEFAULT_LOGO`, and an `IsLogo` picture keeps its shape, centred
/// and never enlarged.
#[test]
fn logo_pictures_keep_their_shape() {
    let markup = "<Skin><Logo>
\t<Static Align='Client' State='CMD_DEFAULT_LOGO'>
\t\t<Image Image='Atlas.png' Align='Client' ImageLeft='0' ImageTop='10' ImageWidth='8' ImageHeight='4' IsLogo='1' />
\t</Static>
\t<Static Align='Client' State='CMD_LOADING_LOGO' />
</Logo></Skin>";
    let archive = SkinArchive::from_files(
        "Logo",
        [
            ("VideoSkin.xml", markup.as_bytes().to_vec()),
            ("atlas.png", atlas()),
        ],
    );
    let skin = Skin::load(archive, "VideoSkin.xml").expect("load");
    assert_eq!(skin.logo_states(), (true, true));
    let draw = |state: &UiState, width: i32, height: i32| {
        let placed = layout(&skin, "Logo", width, height, state).expect("layout");
        let list = paint(&skin, &placed, state, &DataModel::default());
        rasterize(&list, &skin, &mut Fonts::new(Vec::new()))
    };
    let mut state = UiState::default();
    // Without its state the logo does not show.
    assert_eq!(draw(&state, 20, 20).pixel(10, 10)[3], 0);
    state.flags.insert("CMD_DEFAULT_LOGO".to_owned());
    // An 8x4 picture in 20x20: centred at its size (6..14, 8..12).
    let image = draw(&state, 20, 20);
    assert_eq!(image.pixel(10, 10), BUTTON_STATES[0]);
    assert_eq!(image.pixel(10, 6)[3], 0, "above the picture");
    assert_eq!(image.pixel(4, 10)[3], 0, "left of the picture");
    // In 4x20 it shrinks to 4x2, keeping its shape.
    let image = draw(&state, 4, 20);
    assert_ne!(image.pixel(1, 10)[3], 0);
    assert_eq!(image.pixel(1, 7)[3], 0, "above the shrunk picture");
}

/// A context menu file: its English title, items, separators and
/// submenus; and a skin's menu style, coloured from its picture.
#[test]
fn menu_files_and_skin_menu_styles() {
    use rvp_skin::menu::{MenuFileItem, menu_file};
    let file = menu_file(
        "<?XML version='1.0' ?>
<Menu Title=\"{$CP949=x$}{$CP0=Short menu$}\">
\t<MenuItem CmdID='ID_PLAY_PAUSE' />
\t<MenuItem CmdID='' />
\t<SubMenu Name='$MENU_VIEW'>
\t\t<MenuItem CmdID='CMD_POPUPMENU_SKIN' NoSubMenu='1' />
\t</SubMenu>
</Menu>",
    )
    .expect("a menu");
    assert_eq!(file.title, "Short menu");
    assert_eq!(file.items.len(), 3);
    let MenuFileItem::Item(first) = &file.items[0] else {
        panic!("{:?}", file.items[0]);
    };
    assert_eq!(first.command, "ID_PLAY_PAUSE");
    let MenuFileItem::Sub { name, items } = &file.items[2] else {
        panic!("{:?}", file.items[2]);
    };
    assert_eq!(name, "View");
    assert!(matches!(&items[0], MenuFileItem::Item(item) if item.inline));
    assert_eq!(menu_file("<Skin />"), None);

    // The frame's middle is the background; the points give the colours.
    let markup = "<Skin><MainSkinArea Image='Atlas.png'><Control /></MainSkinArea>
<Menu><CustomPopupMenuDef FrameX='0' FrameY='10' FrameW='16' FrameH='8' TextColorX='60' TextColorY='60'
 SelTextColorX='0' SelTextColorY='30' SelColorX='0' SelColorY='32' DimTextColorX='0' DimTextColorY='34'
 LineColorX='12' LineColorY='30' /></Menu></Skin>";
    let archive = SkinArchive::from_files(
        "Menus",
        [
            ("VideoSkin.xml", markup.as_bytes().to_vec()),
            ("atlas.png", atlas()),
        ],
    );
    let skin = Skin::load(archive, "VideoSkin.xml").expect("load");
    let style = skin.menu_style(false).expect("a style");
    assert_eq!(style.back, BUTTON_STATES[1]);
    assert_eq!(style.border, BUTTON_STATES[0]);
    assert_eq!(style.text, LABEL_COLOR);
    assert_eq!(style.selected_text, SLIDER_BACK);
    assert_eq!(style.selected_back, SLIDER_FILL);
    assert_eq!(style.dim_text, SLIDER_DISABLED);
    assert_eq!(style.line, SLIDER_THUMB);
    assert_eq!(skin.menu_style(true), None);
}

/// Preprocessor variables besides `SkinName`: `SkinType=Touch` takes a
/// skin's touch layout (M14).
#[test]
fn the_touch_layout_comes_from_skin_type() {
    let markup = "<Skin>
#if<SkinType==Touch>
<Marker Value='touch' />
#else
<Marker Value='mouse' />
#endif
</Skin>";
    let load = |variables: &[(&str, &str)]| {
        let archive = SkinArchive::from_files(
            "Touch",
            [
                ("VideoSkin.xml", markup.as_bytes().to_vec()),
                ("atlas.png", atlas()),
            ],
        );
        let skin = Skin::load_with(archive, "VideoSkin.xml", variables).expect("load");
        skin.root()
            .find("Marker")
            .and_then(|marker| marker.attrs.get("Value"))
            .map(str::to_owned)
    };
    assert_eq!(load(&[]).as_deref(), Some("mouse"));
    assert_eq!(load(&[("SkinType", "Touch")]).as_deref(), Some("touch"));
}
