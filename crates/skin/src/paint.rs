//! Turns a layout into a display list: sprite blits, fills and texts.
//!
//! Drawing rules, checked against PotPlayer screenshots of BlackSpace:
//!
//! * A `Static` with an image draws it as a nine-slice: `LeftMargin`,
//!   `RightMargin`, `TopMargin`, `BottomMargin` are fixed borders and the
//!   middle is tiled. Sprites are never scaled; tiling keeps patterned
//!   sprites (the dotted seek bar) intact.
//! * Inactive windows use `AltImageLeft`/`AltImageTop` for statics.
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

use crate::geometry::Rect;
use crate::layout::{Kind, Layout, Node};
use crate::markup::{Attrs, atoi};
use crate::playlist;
use crate::skin::Skin;
use crate::state::{DataModel, SliderValue, UiState};

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
            list.ops.push(Op::Fill {
                rect: node.clip,
                color: [0, 0, 0, 255],
            });
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
    let left = attrs.int_or("ImageLeft", 0);
    let top = attrs.int_or("ImageTop", 0);
    let atlas_width = skin
        .image(node.atlas)
        .map_or(0, |image| i32::try_from(image.width()).unwrap_or(i32::MAX));
    let mut src = Rect::new(left + index * width, top, width, height);
    if src.right() > atlas_width {
        src = Rect::new(left, top, width, height);
    }
    list.image(node.atlas, src, node.rect, node.clip);
}

fn paint_label(
    list: &mut DisplayList,
    skin: &Skin,
    node: &Node<'_>,
    state: &UiState,
    data: &DataModel,
) {
    let attrs = &node.element.attrs;
    let text = match node.command().and_then(|cmd| data.text(cmd)) {
        Some(text) => text.to_owned(),
        None => match attrs.get("Text") {
            Some(text) => skin.strings().resolve(text, state.code_page),
            None => return,
        },
    };
    if text.is_empty() {
        return;
    }
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
        font: FontSpec {
            family: attrs.get("FontName").unwrap_or("Tahoma").to_owned(),
            size_px: f32::from(u16::try_from(attrs.int_or("FontSize", 12)).unwrap_or(12)),
            bold: attrs.flag("FontBold"),
        },
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
    list.image(
        node.atlas,
        Rect::new(left, top, width, height),
        node.rect,
        node.clip,
    );
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
    let thumb_width = attrs.int_or("ThumbWidth", 0);
    let thumb_height = attrs.int_or("ThumbHeight", 0);
    if slider.enabled
        && thumb_width > 0
        && thumb_height > 0
        && (attrs.has("ThumbImageLeft") || attrs.has("ThumbImageTop"))
    {
        let (x, y) = if vertical {
            (
                rect.x + (rect.w - thumb_width) / 2,
                rect.y + rect.h - thumb_height - fraction(rect.h - thumb_height, slider.value),
            )
        } else {
            (
                rect.x + fraction(rect.w - thumb_width, slider.value),
                rect.y + (rect.h - thumb_height) / 2,
            )
        };
        list.image(
            node.atlas,
            Rect::new(
                attrs.int_or("ThumbImageLeft", 0),
                attrs.int_or("ThumbImageTop", 0),
                thumb_width,
                thumb_height,
            ),
            Rect::new(x, y, thumb_width, thumb_height),
            node.clip,
        );
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
