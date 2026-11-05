//! Turns a layout into a display list: sprite blits, fills and texts.
//!
//! Drawing rules, checked against PotPlayer screenshots of BlackSpace:
//!
//! * A `Static` with an image draws it as a nine-slice: `LeftMargin`,
//!   `RightMargin`, `TopMargin`, `BottomMargin` are fixed borders and the
//!   middle is tiled. Sprites are never scaled; tiling keeps patterned
//!   sprites (the dotted seek bar) intact.
//! * Inactive windows use `AltImageLeft`/`AltImageTop` for statics.
//! * The video slot (`Contain='CMD_VIDEO_WINDOW'`) is black in the window,
//!   where the skin lies under the video. In `MainSkinAreaOscInside`,
//!   whose bars lie over the video in fullscreen, it draws nothing.
//! * A `Button` or `Image` with `Alignment` (`Default.dsf`, `WindowFrame`)
//!   draws its sprite at the sprite's size inside its place, placed by
//!   `DrawText`'s flags: 1 centre, 2 right, 4 middle, 8 bottom (`5`: the
//!   middle of the place). Without it an `Image` fills its place, tiled;
//!   a `Button` larger than its sprite draws it once in the middle
//!   (Default.dsf's bottom bar: the buttons as tall as the bar, their
//!   pictures centred in PotPlayer's capture), unless it has margins to
//!   stretch by.
//! * Button sprites hold their states side by side, each as wide as the
//!   button: normal, hover, pressed, disabled. A missing state falls back to
//!   normal. Toggle buttons (`DownFlag`) show the pressed state while on;
//!   a `CMD_SELECT_PAGE` toggle is on while its page shows (an RVP
//!   extension for RVP Default's page tabs: BlackSpace's page buttons have
//!   no `DownFlag`).
//!   RVP can give a button a picture of its own ([`Skin::replace_sprite`]):
//!   it draws its logo on title-bar logo buttons this way (decision D10),
//!   with the states side by side from the image's left edge.
//! * `Label` text colour is the atlas pixel at `ColorX`/`ColorY`
//!   (`OverColorX`/`OverColorY` on hover). `HoriAlignment` 0/1/2 is
//!   left/centre/right, `VertAlignment` 0/1/2 is top/centre/bottom.
//! * `PicLabel` draws text with a bitmap font: the sprite is cut into 22
//!   equal cells for the characters `%` to `:`. With `MappingDB` it instead
//!   picks the cell (each `ImageWidth` wide) of the entry that matches the
//!   value; entries are separated by `,`, alternatives by `;`.
//! * `Slider` draws its back image (or `AltImage` when disabled), the fill
//!   image up to the value and the thumb. With no media loaded the seek bar
//!   (`CMD_TIMEBAR_CTRL`) draws nothing at all (R1 idle captures); the
//!   `AltImage` track is for media that cannot be sought (the owner's
//!   screenshot of a loaded, stopped file). A `Vert='1'` slider (the
//!   control panel's equalizer) runs bottom to top: its track is centred
//!   across the width, the sprites are sliced by `TopMargin` and
//!   `BottomMargin`, and the fill grows up from the bottom.
//! * The seek bar's marks (an RVP drawing: PotPlayer's "Show markers of
//!   bookmark/chapter on nav bar") lie over the track and under the thumb,
//!   as tall as the slider: a 1-pixel line per chapter (white) and bookmark
//!   (light blue), 2-pixel lines for the A-B points (amber), and with both
//!   points a faint amber band between them.
//! * A `Visual` (the audio skins' spectrum, M14) draws a bar per
//!   `BarWidth` plus `BarDistance` of its width, each a column of
//!   `LineHeight`-pixel lines `LineDistance` apart, lit from the bottom up
//!   to its band's level, coloured from the atlas pixel at
//!   `BarStartColorX`/`Y` at the bottom to `BarEndColorX`/`Y` at the top
//!   (goal P's host capture: PotPlayer's lit lines over the skin's dimmed
//!   ones); a `PeakHeight` line in `PeakColorX`/`Y` marks each band's
//!   peak. Bars share the bands evenly, low frequencies on the left.
//! * An `Image` marked `IsLogo='1'` (the skin's logo in its `<Logo>`
//!   section, M14) keeps its shape: centred in its place, never larger
//!   than its picture, smaller when the place is smaller. Other `Image`s
//!   fill their place.

use crate::geometry::Rect;
use crate::layout::{Kind, Layout, Node};
use crate::markup::{Attrs, atoi};
use crate::playlist;
use crate::skin::Skin;
use crate::state::{DataModel, MarkKind, SliderValue, UiState};

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// Vertical text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

/// A font request. Sizes are pixel em sizes, as PotPlayer's `FontSize`.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSpec {
    pub family: String,
    pub size_px: f32,
    pub bold: bool,
}

/// A text drawing request; the renderer measures and places it.
#[derive(Debug, Clone, PartialEq)]
pub struct TextOp {
    pub text: String,
    pub font: FontSpec,
    pub color: [u8; 4],
    pub rect: Rect,
    pub clip: Rect,
    pub h_align: HAlign,
    pub v_align: VAlign,
    /// Shorten with `...` when the text does not fit the rectangle.
    pub ellipsis: bool,
    /// Text drawn right-aligned in the same rectangle; the main text is
    /// shortened to leave room for it (playlist durations).
    pub trailing: Option<String>,
}

/// One drawing operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Tiles `src` of atlas `atlas` over `dst`, clipped to `clip`.
    Image {
        atlas: String,
        src: Rect,
        dst: Rect,
        clip: Rect,
    },
    /// Scales `src` of atlas `atlas` to `dst`, clipped to `clip` (VLC's
    /// stretched images, M9).
    Stretch {
        atlas: String,
        src: Rect,
        dst: Rect,
        clip: Rect,
    },
    /// Fills a rectangle with a colour (straight alpha).
    Fill {
        rect: Rect,
        color: [u8; 4],
    },
    Text(TextOp),
}

/// The drawing operations for one window, in painting order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayList {
    pub width: i32,
    pub height: i32,
    pub ops: Vec<Op>,
}

impl DisplayList {
    /// Adds a sprite blit; empty rectangles are dropped.
    pub fn image(&mut self, atlas: &str, src: Rect, dst: Rect, clip: Rect) {
        if src.is_empty() || dst.is_empty() || dst.intersect(&clip).is_empty() {
            return;
        }
        self.ops.push(Op::Image {
            atlas: atlas.to_owned(),
            src,
            dst,
            clip,
        });
    }

    /// Adds a nine-slice: fixed borders, tiled middle.
    pub fn nine_slice(&mut self, atlas: &str, src: Rect, margins: [i32; 4], dst: Rect, clip: Rect) {
        let [left, top, right, bottom] = margins;
        let columns = slices(src.x, src.w, left, right, dst.x, dst.w);
        let rows = slices(src.y, src.h, top, bottom, dst.y, dst.h);
        for &(src_y, src_h, dst_y, dst_h) in &rows {
            for &(src_x, src_w, dst_x, dst_w) in &columns {
                self.image(
                    atlas,
                    Rect::new(src_x, src_y, src_w, src_h),
                    Rect::new(dst_x, dst_y, dst_w, dst_h),
                    clip,
                );
            }
        }
    }
}

/// `src`'s size fitted into `place`, keeping its aspect, centred.
fn fit(src: Rect, place: Rect) -> Rect {
    if src.w <= 0 || src.h <= 0 {
        return Rect::new(place.x, place.y, 0, 0);
    }
    let scale = (f64::from(place.w) / f64::from(src.w)).min(f64::from(place.h) / f64::from(src.h));
    // Sizes of a window: small.
    #[allow(clippy::cast_possible_truncation)]
    let (w, h) = (
        (f64::from(src.w) * scale).round() as i32,
        (f64::from(src.h) * scale).round() as i32,
    );
    Rect::new(
        place.x + (place.w - w) / 2,
        place.y + (place.h - h) / 2,
        w,
        h,
    )
}

/// Splits one axis into fixed start, tiled middle and fixed end:
/// `(src_start, src_len, dst_start, dst_len)` triples.
fn slices(
    src_start: i32,
    src_len: i32,
    start_margin: i32,
    end_margin: i32,
    dst_start: i32,
    dst_len: i32,
) -> [(i32, i32, i32, i32); 3] {
    let start = start_margin.clamp(0, src_len).min(dst_len);
    let end = end_margin.clamp(0, src_len - start).min(dst_len - start);
    [
        (src_start, start, dst_start, start),
        (
            src_start + start,
            src_len - start - end,
            dst_start + start,
            dst_len - start - end,
        ),
        (
            src_start + src_len - end,
            end,
            dst_start + dst_len - end,
            end,
        ),
    ]
}

/// File types in the order of PotPlayer's `CMD_PLAY_FILE_TYPE` sprites.
pub const FILE_TYPES: [&str; 15] = [
    "DVD", "CAM", "CAP", "ATV", "DTV", "ASF", "MKV", "REAL", "MP4", "OGG", "AVI", "MPG", "ETC",
    "UCC", "TS",
];

/// Number of cells in a `PicLabel` bitmap font: `%` (0x25) to `:` (0x3A).
const PIC_FONT_CELLS: i32 = 22;
const PIC_FONT_FIRST: u32 = '%' as u32;

/// Builds the display list for a layout.
#[must_use]
pub fn paint(skin: &Skin, layout: &Layout<'_>, state: &UiState, data: &DataModel) -> DisplayList {
    let mut list = DisplayList {
        width: layout.width,
        height: layout.height,
        ops: Vec::with_capacity(layout.nodes.len() * 2),
    };
    for node in &layout.nodes {
        match node.kind {
            Kind::Static => paint_static(&mut list, skin, node, state, data),
            Kind::Button => paint_button(&mut list, skin, node, state),
            Kind::Label => paint_label(&mut list, skin, node, state, data),
            Kind::PicLabel => paint_pic_label(&mut list, node, data),
            Kind::Image => paint_image(&mut list, node, data),
            Kind::Slider => paint_slider(&mut list, node, state, data),
            Kind::Visual => paint_visual(&mut list, skin, node, data),
            Kind::Graph => paint_graph(&mut list, skin, node, data),
            Kind::Spacer => {}
        }
    }
    list
}

fn has_sprite(attrs: &Attrs) -> bool {
    attrs.has("ImageLeft") || attrs.has("ImageTop")
}

fn margins(attrs: &Attrs) -> [i32; 4] {
    [
        attrs.int_or("LeftMargin", 0),
        attrs.int_or("TopMargin", 0),
        attrs.int_or("RightMargin", 0),
        attrs.int_or("BottomMargin", 0),
    ]
}

fn paint_static(
    list: &mut DisplayList,
    skin: &Skin,
    node: &Node<'_>,
    state: &UiState,
    data: &DataModel,
) {
    let attrs = &node.element.attrs;
    match node.slot() {
        Some("CMD_VIDEO_WINDOW") => {
            // Black under the video in the window; in fullscreen the bars
            // of `MainSkinAreaOscInside` lie over the video, so its slot
            // there draws nothing.
            if !node.area.is("MainSkinAreaOscInside") {
                list.ops.push(Op::Fill {
                    rect: node.clip,
                    color: [0, 0, 0, 255],
                });
            }
            return;
        }
        Some("CMD_PLAY_LIST_ALBUM_TAB") => {
            playlist::paint_list(list, skin, node, state, &data.playlist);
            return;
        }
        _ => {}
    }
    let width = attrs.int_or("ImageWidth", 0);
    let height = attrs.int_or("ImageHeight", 0);
    if width <= 0 || height <= 0 {
        return;
    }
    let (x, y) = if !state.window_active && attrs.has("AltImageLeft") {
        (
            attrs.int_or("AltImageLeft", 0),
            attrs.int_or("AltImageTop", 0),
        )
    } else {
        (attrs.int_or("ImageLeft", 0), attrs.int_or("ImageTop", 0))
    };
    // RVP's extension for VLC's images (M9): `RvpStretch='1'` scales the
    // picture to the place, `'fit'` keeps its aspect, centred.
    if let Some(stretch) = attrs.get("RvpStretch") {
        let src = Rect::new(x, y, width, height);
        let dst = if stretch == "fit" {
            fit(src, node.rect)
        } else {
            node.rect
        };
        if !dst.is_empty() && !dst.intersect(&node.clip).is_empty() {
            list.ops.push(Op::Stretch {
                atlas: node.atlas.to_owned(),
                src,
                dst,
                clip: node.clip,
            });
        }
        return;
    }
    list.nine_slice(
        node.atlas,
        Rect::new(x, y, width, height),
        margins(attrs),
        node.rect,
        node.clip,
    );
}

fn paint_button(list: &mut DisplayList, skin: &Skin, node: &Node<'_>, state: &UiState) {
    let attrs = &node.element.attrs;
    let replaced = skin.replaced_sprite(node.element.id);
    if !has_sprite(attrs) && replaced.is_none() {
        return;
    }
    let width = attrs
        .int("ImageWidth")
        .filter(|w| *w > 0)
        .unwrap_or(node.rect.w);
    let height = attrs
        .int("ImageHeight")
        .filter(|h| *h > 0)
        .unwrap_or(node.rect.h);
    let toggled = attrs.flag("DownFlag")
        && node.command().is_some_and(|cmd| {
            if cmd == "CMD_SELECT_PAGE" {
                attrs.int("PageIndex") == Some(state.current_page)
            } else {
                state.flag(cmd)
            }
        });
    let index = if attrs.int("Enabled") == Some(0) {
        3
    } else if state.pressed == Some(node.element.id) || toggled {
        2
    } else {
        i32::from(state.hover == Some(node.element.id))
    };
    if let Some(image) = replaced {
        // RVP's own picture (D10): states side by side from the left edge.
        let src = Rect::new(index * node.rect.w, 0, node.rect.w, node.rect.h);
        list.image(image, src, node.rect, node.clip);
        return;
    }
    // A toggle that is on draws from its own states (`RvpOnImageLeft`,
    // classic skins' lit buttons, M9): normal, hover, pressed.
    let on = toggled && attrs.has("RvpOnImageLeft");
    let (left, top, index) = if on {
        let index = if state.pressed == Some(node.element.id) {
            2
        } else {
            i32::from(state.hover == Some(node.element.id))
        };
        (
            attrs.int_or("RvpOnImageLeft", 0),
            attrs.int_or("RvpOnImageTop", 0),
            index,
        )
    } else {
        (
            attrs.int_or("ImageLeft", 0),
            attrs.int_or("ImageTop", 0),
            index,
        )
    };
    let atlas_width = skin
        .image(node.atlas)
        .map_or(0, |image| i32::try_from(image.width()).unwrap_or(i32::MAX));
    let mut src = Rect::new(left + index * width, top, width, height);
    if src.right() > atlas_width {
        src = Rect::new(left, top, width, height);
    }
    if has_margins(attrs) {
        // A frame for its contents (Default.dsf's codec boxes, sized to
        // their texts with `AutoSize`): its borders fixed, its middle
        // stretched to the button's width, at the sprite's height.
        let place = Rect::new(node.rect.x, node.rect.y, width, height);
        let row = aligned(node.rect, width, height, attrs);
        let dst = Rect::new(place.x, row.y, node.rect.w, height.min(node.rect.h));
        let margins = [
            attrs.int_or("LeftMargin", 0),
            attrs.int_or("TopMargin", 0),
            attrs.int_or("RightMargin", 0),
            attrs.int_or("BottomMargin", 0),
        ];
        list.nine_slice(node.atlas, src, margins, dst, node.clip);
        return;
    }
    let dst = if attrs.has("Alignment") {
        aligned(node.rect, width, height, attrs)
    } else {
        centred(node.rect, width, height)
    };
    list.image(node.atlas, src, dst, node.clip);
}

/// A sliced sprite: fixed borders and a stretched middle.
fn has_margins(attrs: &Attrs) -> bool {
    ["LeftMargin", "RightMargin", "TopMargin", "BottomMargin"]
        .iter()
        .any(|name| attrs.int(name).is_some_and(|margin| margin > 0))
}

/// A `width` x `height` sprite in the middle of `place` when the place is
/// larger; the place itself otherwise.
fn centred(place: Rect, width: i32, height: i32) -> Rect {
    if width >= place.w && height >= place.h {
        return place;
    }
    let (width, height) = (width.min(place.w), height.min(place.h));
    Rect::new(
        place.x + (place.w - width) / 2,
        place.y + (place.h - height) / 2,
        width,
        height,
    )
}

/// The place of a `width` x `height` sprite inside `place` by the
/// element's `Alignment` (`DrawText`'s flags), or `place` itself without one.
fn aligned(place: Rect, width: i32, height: i32, attrs: &Attrs) -> Rect {
    let Some(flags) = attrs.int("Alignment") else {
        return place;
    };
    let x = if flags & 1 != 0 {
        place.x + (place.w - width) / 2
    } else if flags & 2 != 0 {
        place.right() - width
    } else {
        place.x
    };
    let y = if flags & 4 != 0 {
        place.y + (place.h - height) / 2
    } else if flags & 8 != 0 {
        place.bottom() - height
    } else {
        place.y
    };
    Rect::new(x, y, width, height)
}

/// A label's text: its command's, else its own `Text`; `None` when empty.
pub(crate) fn label_text(
    skin: &Skin,
    attrs: &Attrs,
    state: &UiState,
    data: &DataModel,
) -> Option<String> {
    let command = attrs
        .get("CmdID")
        .map(str::trim)
        .filter(|cmd| !cmd.is_empty());
    let text = match command.and_then(|cmd| data.text(cmd)) {
        Some(text) => text.to_owned(),
        None => skin.strings().resolve(attrs.get("Text")?, state.code_page),
    };
    (!text.is_empty()).then_some(text)
}

/// A label's font.
pub(crate) fn label_font(attrs: &Attrs) -> FontSpec {
    FontSpec {
        family: attrs.get("FontName").unwrap_or("Tahoma").to_owned(),
        size_px: f32::from(u16::try_from(attrs.int_or("FontSize", 12)).unwrap_or(12)),
        bold: attrs.flag("FontBold"),
    }
}

fn paint_label(
    list: &mut DisplayList,
    skin: &Skin,
    node: &Node<'_>,
    state: &UiState,
    data: &DataModel,
) {
    let attrs = &node.element.attrs;
    let Some(text) = label_text(skin, attrs, state, data) else {
        return;
    };
    let hovered = state.hover == Some(node.element.id);
    let (color_x, color_y) = if hovered && attrs.has("OverColorX") {
        (attrs.int_or("OverColorX", 0), attrs.int_or("OverColorY", 0))
    } else {
        (attrs.int_or("ColorX", 0), attrs.int_or("ColorY", 0))
    };
    let mut color = skin.atlas_color(node.atlas, color_x, color_y);
    color[3] = 255;
    list.ops.push(Op::Text(TextOp {
        text,
        font: label_font(attrs),
        color,
        rect: node.rect,
        clip: node.clip,
        h_align: match attrs.int_or("HoriAlignment", 0) {
            1 => HAlign::Center,
            2 => HAlign::Right,
            _ => HAlign::Left,
        },
        v_align: match attrs.int_or("VertAlignment", 0) {
            1 => VAlign::Center,
            2 => VAlign::Bottom,
            _ => VAlign::Top,
        },
        ellipsis: false,
        trailing: None,
    }));
}

/// Index of the `MappingDB` entry that matches `value`.
#[must_use]
pub fn mapping_index(mapping: &str, value: &str) -> Option<usize> {
    let value = value.trim();
    let entries: Vec<&str> = mapping.split(',').collect();
    for (index, entry) in entries.iter().enumerate() {
        if entry
            .split(';')
            .any(|alternative| alternative.trim().eq_ignore_ascii_case(value))
        {
            return Some(index);
        }
    }
    if value.len() > 2 && value[..2].eq_ignore_ascii_case("0x") {
        let wanted = atoi(value);
        for (index, entry) in entries.iter().enumerate() {
            if entry.split(';').any(|alternative| {
                let alternative = alternative.trim();
                alternative.len() > 2
                    && alternative[..2].eq_ignore_ascii_case("0x")
                    && atoi(alternative) == wanted
            }) {
                return Some(index);
            }
        }
    }
    None
}

fn paint_pic_label(list: &mut DisplayList, node: &Node<'_>, data: &DataModel) {
    let attrs = &node.element.attrs;
    let Some(text) = node.command().and_then(|cmd| data.text(cmd)) else {
        return;
    };
    let left = attrs.int_or("ImageLeft", 0);
    let top = attrs.int_or("ImageTop", 0);
    let width = attrs.int_or("ImageWidth", 0);
    let height = attrs.int_or("ImageHeight", 0);
    if width <= 0 || height <= 0 {
        return;
    }
    if let Some(mapping) = attrs.get("MappingDB") {
        if let Some(index) = mapping_index(mapping, text) {
            let index = i32::try_from(index).unwrap_or(0);
            list.image(
                node.atlas,
                Rect::new(left + index * width, top, width, height),
                Rect::new(node.rect.x, node.rect.y, width, height),
                node.clip,
            );
        }
        return;
    }
    // RVP's extension for classic skins (M9): `RvpChars` lists the
    // characters of the cells as Unicode numbers (`65,66,...`: the markup
    // has no escapes for quotes), each cell `RvpCellWidth` wide; others
    // draw the cell of a space.
    if let Some(chars) = attrs.get("RvpChars") {
        let cell = attrs.int_or("RvpCellWidth", 5).max(1);
        let cells: Vec<char> = chars
            .split(',')
            .filter_map(|code| code.trim().parse::<u32>().ok())
            .filter_map(char::from_u32)
            .collect();
        let blank = cells.iter().position(|c| *c == ' ');
        let total = cell * i32::try_from(text.chars().count()).unwrap_or(0);
        let mut x = match attrs.int_or("HoriAlignment", 0) {
            1 => node.rect.x + (node.rect.w - total) / 2,
            2 => node.rect.right() - total,
            _ => node.rect.x,
        };
        for ch in text.chars() {
            let upper = ch.to_uppercase().next().unwrap_or(ch);
            let index = cells.iter().position(|c| *c == upper).or(blank);
            if let Some(index) = index {
                let index = i32::try_from(index).unwrap_or(0);
                list.image(
                    node.atlas,
                    Rect::new(left + index * cell, top, cell, height),
                    Rect::new(x, node.rect.y, cell, height),
                    node.clip,
                );
            }
            x += cell;
        }
        return;
    }
    // Cells are `ImageWidth / 22` pixels wide, rounded down; BlackSpace's
    // 134-pixel bitrate font has 6-pixel cells and two spare pixels.
    let cell = width / PIC_FONT_CELLS;
    let total = cell * i32::try_from(text.chars().count()).unwrap_or(0);
    let mut x = match attrs.int_or("HoriAlignment", 0) {
        1 => node.rect.x + (node.rect.w - total) / 2,
        2 => node.rect.right() - total,
        _ => node.rect.x,
    };
    for ch in text.chars() {
        match pic_cell(ch) {
            Some(index) => {
                list.image(
                    node.atlas,
                    Rect::new(left + index * cell, top, cell, height),
                    Rect::new(x, node.rect.y, cell, height),
                    node.clip,
                );
                x += cell;
            }
            None => x += cell,
        }
    }
}

fn pic_cell(ch: char) -> Option<i32> {
    let code = u32::from(ch);
    if (PIC_FONT_FIRST..PIC_FONT_FIRST + 22).contains(&code) {
        i32::try_from(code - PIC_FONT_FIRST).ok()
    } else {
        None
    }
}

fn paint_image(list: &mut DisplayList, node: &Node<'_>, data: &DataModel) {
    let attrs = &node.element.attrs;
    if !has_sprite(attrs) {
        return;
    }
    let left = attrs.int_or("ImageLeft", 0);
    let top = attrs.int_or("ImageTop", 0);
    let width = attrs
        .int("ImageWidth")
        .filter(|w| *w > 0)
        .unwrap_or(node.rect.w);
    let height = attrs
        .int("ImageHeight")
        .filter(|h| *h > 0)
        .unwrap_or(node.rect.h);
    if node.command() == Some("CMD_PLAY_FILE_TYPE") {
        let Some(kind) = data.text("CMD_PLAY_FILE_TYPE") else {
            return;
        };
        let index = FILE_TYPES
            .iter()
            .position(|name| name.eq_ignore_ascii_case(kind))
            .or_else(|| FILE_TYPES.iter().position(|name| *name == "ETC"))
            .unwrap_or(0);
        let index = i32::try_from(index).unwrap_or(0);
        list.image(
            node.atlas,
            Rect::new(left + index * width, top, width, height),
            Rect::new(node.rect.x, node.rect.y, width, height),
            node.clip,
        );
        return;
    }
    let dst = if attrs.flag("IsLogo") {
        fit_logo(node.rect, width, height)
    } else {
        aligned(node.rect, width, height, attrs)
    };
    list.image(
        node.atlas,
        Rect::new(left, top, width, height),
        dst,
        node.clip,
    );
}

/// A `width` x `height` logo centred in `place`, shrunk to fit it.
fn fit_logo(place: Rect, width: i32, height: i32) -> Rect {
    if width <= 0 || height <= 0 || place.w <= 0 || place.h <= 0 {
        return place;
    }
    let scale = (f64::from(place.w) / f64::from(width))
        .min(f64::from(place.h) / f64::from(height))
        .min(1.0);
    // At most the picture's size: within i32.
    #[allow(clippy::cast_possible_truncation)]
    let (w, h) = (
        (f64::from(width) * scale).round() as i32,
        (f64::from(height) * scale).round() as i32,
    );
    Rect::new(
        place.x + (place.w - w) / 2,
        place.y + (place.h - h) / 2,
        w,
        h,
    )
}

/// A `Visual`: the spectrum's bars, each a column of `LineHeight` lines
/// `LineDistance` apart, `BarWidth` wide and `BarDistance` apart, lit from
/// the bottom up to its band's level with colours that run from the atlas
/// pixel at `BarStartColorX`/`Y` (bottom) to `BarEndColorX`/`Y` (top); a
/// `PeakHeight` line in `PeakColorX`/`Y` marks each band's peak.
fn paint_visual(list: &mut DisplayList, skin: &Skin, node: &Node<'_>, data: &DataModel) {
    if data.visual.is_empty() {
        return;
    }
    let attrs = &node.element.attrs;
    let rect = node.rect;
    let bar_width = attrs.int_or("BarWidth", 4).max(1);
    let bar_gap = attrs.int_or("BarDistance", 1).max(0);
    let line_height = attrs.int_or("LineHeight", 2).max(1);
    let line_gap = attrs.int_or("LineDistance", 1).max(0);
    let peak_height = attrs.int_or("PeakHeight", 0).max(0);
    let color = |name: &str| {
        let mut color = skin.atlas_color(
            node.atlas,
            attrs.int_or(&format!("{name}ColorX"), 0),
            attrs.int_or(&format!("{name}ColorY"), 0),
        );
        color[3] = 255;
        color
    };
    let (start, end, peak) = (color("BarStart"), color("BarEnd"), color("Peak"));
    let bars = (rect.w + bar_gap) / (bar_width + bar_gap);
    let lines = (rect.h + line_gap) / (line_height + line_gap);
    if bars <= 0 || lines <= 0 {
        return;
    }
    let band = |values: &[f32], bar: i32| -> f32 {
        // Bars share the bands evenly, low frequencies on the left.
        let count = i32::try_from(values.len()).unwrap_or(i32::MAX);
        let index = usize::try_from(bar * count / bars).unwrap_or(0);
        values.get(index).copied().unwrap_or(0.0).clamp(0.0, 1.0)
    };
    let lit_lines = |level: f32| {
        // At most a few dozen lines: f32 holds them exactly.
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let count = (level * lines as f32).round() as i32;
        count.clamp(0, lines)
    };
    let line_top = |line: i32| rect.y + rect.h - (line + 1) * line_height - line * line_gap;
    for bar in 0..bars {
        let x = rect.x + bar * (bar_width + bar_gap);
        for line in 0..lit_lines(band(&data.visual, bar)) {
            let mix = if lines > 1 {
                f64::from(line) / f64::from(lines - 1)
            } else {
                0.0
            };
            let channel = |from: u8, to: u8| {
                // A blend of two bytes stays a byte.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let value =
                    (f64::from(from) + (f64::from(to) - f64::from(from)) * mix).round() as u8;
                value
            };
            let color = if attrs.has("RvpRowColorsX") {
                // A colour per line from the bottom (classic skins'
                // `viscolor.txt`, M9).
                let mut color = skin.atlas_color(
                    node.atlas,
                    attrs.int_or("RvpRowColorsX", 0) + line,
                    attrs.int_or("RvpRowColorsY", 0),
                );
                color[3] = 255;
                color
            } else {
                [
                    channel(start[0], end[0]),
                    channel(start[1], end[1]),
                    channel(start[2], end[2]),
                    255,
                ]
            };
            let line_rect =
                Rect::new(x, line_top(line), bar_width, line_height).intersect(&node.clip);
            if !line_rect.is_empty() {
                list.ops.push(Op::Fill {
                    rect: line_rect,
                    color,
                });
            }
        }
        if peak_height > 0 && !data.visual_peaks.is_empty() {
            let top = lit_lines(band(&data.visual_peaks, bar));
            if top > 0 {
                let y = line_top(top - 1);
                let peak_rect = Rect::new(x, y, bar_width, peak_height).intersect(&node.clip);
                if !peak_rect.is_empty() {
                    list.ops.push(Op::Fill {
                        rect: peak_rect,
                        color: peak,
                    });
                }
            }
        }
    }
}

/// The equalizer bands' sliders (`CMD_EQULIZER_0` to `_9`) and the
/// preamp's (`RVP:preamp`).
const GRAPH_BANDS: [&str; 10] = [
    "CMD_EQULIZER_0",
    "CMD_EQULIZER_1",
    "CMD_EQULIZER_2",
    "CMD_EQULIZER_3",
    "CMD_EQULIZER_4",
    "CMD_EQULIZER_5",
    "CMD_EQULIZER_6",
    "CMD_EQULIZER_7",
    "CMD_EQULIZER_8",
    "CMD_EQULIZER_9",
];

/// A Kochanek-Bartels spline (tension 0.1, no continuity or bias) through
/// `keys` at `t` (key index, fractional), as classic Winamp draws its
/// equalizer's curve.
fn kb_spline(keys: &[f64], t: f64) -> f64 {
    let last = keys.len().saturating_sub(1);
    if last == 0 {
        return keys.first().copied().unwrap_or(0.0);
    }
    // Small non-negative numbers: the cast keeps them.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i = (t.floor().max(0.0) as usize).min(last - 1);
    let s = t - f64::from(u32::try_from(i).unwrap_or(0));
    let key = |k: isize| {
        let k = k.clamp(0, isize::try_from(last).unwrap_or(isize::MAX));
        keys[usize::try_from(k).unwrap_or(0)]
    };
    let i = isize::try_from(i).unwrap_or(0);
    let (p0, p1, p2, p3) = (key(i - 1), key(i), key(i + 1), key(i + 2));
    let tension = 0.1;
    let m1 = (1.0 - tension) * (p2 - p0) / 2.0;
    let m2 = (1.0 - tension) * (p3 - p1) / 2.0;
    let (s2, s3) = (s * s, s * s * s);
    (2.0 * s3 - 3.0 * s2 + 1.0) * p1
        + (s3 - 2.0 * s2 + s) * m1
        + (-2.0 * s3 + 3.0 * s2) * p2
        + (s3 - s2) * m2
}

/// `RvpGraph` (classic skins' equalizer window, M9): the bands' curve as
/// one-pixel columns, each pixel coloured from the atlas column at
/// `ColorsX`/`ColorsY` by its row, joined vertically; the preamp as a line
/// from `PreampImageLeft`/`Top` (+12 dB at the bottom, as Winamp draws it).
fn paint_graph(list: &mut DisplayList, skin: &Skin, node: &Node<'_>, data: &DataModel) {
    let attrs = &node.element.attrs;
    let rect = node.rect;
    if rect.w < 2 || rect.h < 2 {
        return;
    }
    let value = |command: &str| {
        f64::from(
            data.sliders
                .get(command)
                .map_or(0.5, |slider| slider.value.clamp(0.0, 1.0)),
        )
    };
    let rows = f64::from(rect.h - 1);
    if attrs.has("PreampImageLeft") {
        // Fractions of the height: small.
        #[allow(clippy::cast_possible_truncation)]
        let y = rect.y + (value("RVP:preamp") * rows).round() as i32;
        list.image(
            node.atlas,
            Rect::new(
                attrs.int_or("PreampImageLeft", 0),
                attrs.int_or("PreampImageTop", 0),
                rect.w,
                1,
            ),
            Rect::new(rect.x, y, rect.w, 1),
            node.clip,
        );
    }
    // Each band's height from the top: +12 dB at the top row.
    let mut keys: Vec<f64> = GRAPH_BANDS
        .iter()
        .map(|band| (1.0 - value(band)) * rows)
        .collect();
    // The end keys doubled, as Winamp does.
    keys.insert(0, keys[0]);
    keys.push(keys[keys.len() - 1]);
    let color = |row: i32| {
        let mut color = skin.atlas_color(
            node.atlas,
            attrs.int_or("ColorsX", 0),
            attrs.int_or("ColorsY", 0) + row,
        );
        color[3] = 255;
        color
    };
    let span = f64::from(rect.w - 1);
    let mut previous: Option<i32> = None;
    for x in 0..rect.w {
        let t = 1.0 + f64::from(x) / span * 9.0;
        // Within the graph's rows.
        #[allow(clippy::cast_possible_truncation)]
        let y = kb_spline(&keys, t).round().clamp(0.0, rows) as i32;
        let (from, to) = previous.map_or((y, y), |p| (p.min(y), p.max(y)));
        for row in from..=to {
            let pixel = Rect::new(rect.x + x, rect.y + row, 1, 1).intersect(&node.clip);
            if !pixel.is_empty() {
                list.ops.push(Op::Fill {
                    rect: pixel,
                    color: color(row),
                });
            }
        }
        previous = Some(y);
    }
}

fn paint_slider(list: &mut DisplayList, node: &Node<'_>, state: &UiState, data: &DataModel) {
    let attrs = &node.element.attrs;
    let command = node.command().unwrap_or("");
    if command == "CMD_TIMEBAR_CTRL" && !state.media_loaded {
        return;
    }
    let slider = data.sliders.get(command).copied().unwrap_or(SliderValue {
        value: 0.0,
        enabled: state.media_loaded,
    });
    let width = attrs.int_or("ImageWidth", 0);
    let height = attrs.int_or("ImageHeight", 0);
    let rect = node.rect;
    let vertical = attrs.flag("Vert");
    // RVP's extension for classic and VLC skins (M9): a picture per value,
    // `RvpFrames` of them stacked from `BackImageTop` down, no fill, drawn
    // `RvpFrameOffsetX`/`Y` from the slider's corner.
    if let Some(frames) = attrs.int("RvpFrames").filter(|frames| *frames > 1) {
        let index = fraction(frames - 1, slider.value);
        list.image(
            node.atlas,
            Rect::new(
                attrs.int_or("BackImageLeft", 0),
                attrs.int_or("BackImageTop", 0) + index * height,
                width,
                height,
            ),
            Rect::new(
                rect.x + attrs.int_or("RvpFrameOffsetX", 0),
                rect.y + attrs.int_or("RvpFrameOffsetY", 0),
                width,
                height,
            ),
            node.clip,
        );
        if slider.enabled {
            paint_thumb(list, node, slider.value, state);
        }
        return;
    }
    if width > 0 && height > 0 {
        let [left_margin, top_margin, right_margin, bottom_margin] = margins(attrs);
        let (track, slice) = if vertical {
            (
                Rect::new(rect.x + (rect.w - width) / 2, rect.y, width, rect.h),
                [0, top_margin, 0, bottom_margin],
            )
        } else {
            (
                Rect::new(rect.x, rect.y + (rect.h - height) / 2, rect.w, height),
                [left_margin, 0, right_margin, 0],
            )
        };
        let back = if !slider.enabled && attrs.has("AltImageLeft") {
            Some((
                attrs.int_or("AltImageLeft", 0),
                attrs.int_or("AltImageTop", 0),
            ))
        } else if attrs.has("BackImageLeft") || attrs.has("BackImageTop") {
            Some((
                attrs.int_or("BackImageLeft", 0),
                attrs.int_or("BackImageTop", 0),
            ))
        } else {
            None
        };
        if let Some((x, y)) = back {
            list.nine_slice(
                node.atlas,
                Rect::new(x, y, width, height),
                slice,
                track,
                node.clip,
            );
        }
        if slider.enabled && has_sprite(attrs) {
            let fill = if vertical {
                let filled = fraction(rect.h, slider.value);
                Rect::new(track.x, track.y + track.h - filled, track.w, filled)
            } else {
                Rect::new(track.x, track.y, fraction(rect.w, slider.value), track.h)
            };
            list.nine_slice(
                node.atlas,
                Rect::new(
                    attrs.int_or("ImageLeft", 0),
                    attrs.int_or("ImageTop", 0),
                    width,
                    height,
                ),
                slice,
                fill,
                node.clip,
            );
        }
    }
    if command == "CMD_TIMEBAR_CTRL" && slider.enabled && !vertical {
        paint_marks(list, node, data);
    }
    if slider.enabled {
        paint_thumb(list, node, slider.value, state);
    }
}

/// A slider's thumb at `value`.
fn paint_thumb(list: &mut DisplayList, node: &Node<'_>, value: f32, state: &UiState) {
    let attrs = &node.element.attrs;
    // The thumb while it is held (`ThumbPressedImageLeft`, RVP's for
    // classic skins, M9).
    let pressed = state.pressed == Some(node.element.id) && attrs.has("ThumbPressedImageLeft");
    let (thumb_left, thumb_top) = if pressed {
        ("ThumbPressedImageLeft", "ThumbPressedImageTop")
    } else {
        ("ThumbImageLeft", "ThumbImageTop")
    };
    let rect = node.rect;
    let vertical = attrs.flag("Vert");
    let thumb_width = attrs.int_or("ThumbWidth", 0);
    let thumb_height = attrs.int_or("ThumbHeight", 0);
    if thumb_width > 0
        && thumb_height > 0
        && (attrs.has("ThumbImageLeft") || attrs.has("ThumbImageTop"))
    {
        let (x, y) = if vertical {
            (
                rect.x + (rect.w - thumb_width) / 2,
                rect.y + rect.h - thumb_height - fraction(rect.h - thumb_height, value),
            )
        } else {
            (
                rect.x + fraction(rect.w - thumb_width, value),
                rect.y + (rect.h - thumb_height) / 2,
            )
        };
        list.image(
            node.atlas,
            Rect::new(
                attrs.int_or(thumb_left, 0),
                attrs.int_or(thumb_top, 0),
                thumb_width,
                thumb_height,
            ),
            Rect::new(x, y, thumb_width, thumb_height),
            node.clip,
        );
    }
}

/// The seek bar's marks (see the module comment).
fn paint_marks(list: &mut DisplayList, node: &Node<'_>, data: &DataModel) {
    let rect = node.rect;
    let at = |value: f32| rect.x + fraction(rect.w - 1, value);
    let point = |kind: MarkKind| data.marks.iter().find(|mark| mark.kind == kind);
    if let (Some(a), Some(b)) = (point(MarkKind::PointA), point(MarkKind::PointB)) {
        let (start, end) = (at(a.at.min(b.at)), at(a.at.max(b.at)));
        let band = Rect::new(start, rect.y, end - start + 1, rect.h).intersect(&node.clip);
        if !band.is_empty() {
            list.ops.push(Op::Fill {
                rect: band,
                color: [255, 190, 0, 70],
            });
        }
    }
    for mark in &data.marks {
        let (width, color) = match mark.kind {
            MarkKind::Chapter => (1, [255, 255, 255, 150]),
            MarkKind::Bookmark => (1, [120, 200, 255, 220]),
            MarkKind::PointA | MarkKind::PointB => (2, [255, 190, 0, 230]),
        };
        let line = Rect::new(at(mark.at), rect.y, width, rect.h).intersect(&node.clip);
        if !line.is_empty() {
            list.ops.push(Op::Fill { rect: line, color });
        }
    }
}

/// `length * value` rounded, with `value` clamped to 0..=1.
fn fraction(length: i32, value: f32) -> i32 {
    let value = f64::from(value.clamp(0.0, 1.0));
    let scaled = (f64::from(length.max(0)) * value).round();
    // `scaled` lies within 0..=length, so it fits in i32.
    #[allow(clippy::cast_possible_truncation)]
    let result = scaled as i32;
    result
}

#[cfg(test)]
mod tests {
    use super::{mapping_index, slices};

    #[test]
    fn mapping_entries_and_alternatives() {
        let mapping = "YUY2, RGB, DXVA, NV12, YV12 , DV";
        assert_eq!(mapping_index(mapping, "NV12"), Some(3));
        assert_eq!(mapping_index(mapping, "yv12"), Some(4));
        assert_eq!(mapping_index(mapping, "HEVC"), None);
        let channels = "1,2,2.1;3,3.1;4,4.1;5,5.1;6,6.1;7,7.1;8";
        assert_eq!(mapping_index(channels, "5.1"), Some(5));
        assert_eq!(mapping_index(channels, "5"), Some(4));
        let tags = "0x55;0x1D61346D,0x50;0x80,0x30;0x2000";
        assert_eq!(mapping_index(tags, "0x2000"), Some(2));
        assert_eq!(mapping_index(tags, "0x0050"), Some(1));
    }

    #[test]
    fn nine_slice_axis() {
        assert_eq!(
            slices(0, 410, 20, 20, 100, 1000),
            [(0, 20, 100, 20), (20, 370, 120, 960), (390, 20, 1080, 20)]
        );
        assert_eq!(
            slices(0, 10, 8, 8, 0, 6),
            [(0, 6, 0, 6), (6, 4, 6, 0), (10, 0, 6, 0)]
        );
    }
}
