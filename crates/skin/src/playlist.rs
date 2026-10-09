//! The list part of the attached playlist: album tabs, rows and scroll bar.
//!
//! The skin gives colours and sprites in `PlaylistSkinArea` attributes
//! (`TextColorX`, `AlbumTabBtnX`, ...) and the scroll bar in
//! `ScrollBar.bmp`. Row height, fonts, paddings and the file icon are
//! PotPlayer internals; the constants below approximate them from
//! screenshots and are listed as such in the goal status report.

use crate::geometry::Rect;
use crate::layout::Node;
use crate::markup::Attrs;
use crate::paint::{DisplayList, FontSpec, HAlign, Op, TextOp, VAlign};
use crate::skin::Skin;
use crate::state::{ListScroll, Playlist, UiState};

/// Row pitch of the playlist in pixels (approximation).
pub const ROW_HEIGHT: i32 = 20;
/// Font of playlist rows and album tabs: PotPlayer uses the Windows UI
/// font here, not a skin font (approximation).
pub const LIST_FONT: &str = "Segoe UI";
pub const LIST_FONT_PX: f32 = 12.0;
pub const TAB_FONT_PX: f32 = 12.0;
/// Empty space below the last row when the list is scrolled to its end.
pub const LIST_BOTTOM_MARGIN: i32 = 10;
/// Text colour of entries that could not be opened (approximation).
const UNPLAYABLE_TEXT: [u8; 4] = [100, 100, 100, 255];
/// Width of the vertical scroll bar.
pub const SCROLLBAR_WIDTH: i32 = 12;
const ICON_LEFT: i32 = 4;
const ICON_SIZE: i32 = 16;
const TEXT_LEFT: i32 = 24;
const TEXT_RIGHT_PADDING: i32 = 4;
const SCROLLBAR_ATLAS: &str = "ScrollBar.bmp";
/// Sprites in `ScrollBar.bmp`: up arrow, down arrow, thumb, track.
const SCROLL_UP: Rect = Rect {
    x: 72,
    y: 0,
    w: 12,
    h: 12,
};
const SCROLL_DOWN: Rect = Rect {
    x: 72,
    y: 20,
    w: 12,
    h: 12,
};
const SCROLL_THUMB: Rect = Rect {
    x: 126,
    y: 0,
    w: 12,
    h: 19,
};
const SCROLL_TRACK: Rect = Rect {
    x: 0,
    y: 83,
    w: 12,
    h: 30,
};

struct Colors {
    background: [u8; 4],
    selected_background: [u8; 4],
    frame: [u8; 4],
    text: [u8; 4],
    selected_text: [u8; 4],
    current_text: [u8; 4],
    played_text: [u8; 4],
    tab_text: [u8; 4],
    tab_selected_text: [u8; 4],
}

impl Colors {
    fn read(skin: &Skin, atlas: &str, attrs: &Attrs) -> Self {
        let color = |name: &str| {
            let mut rgba = skin.atlas_color(
                atlas,
                attrs.int_or(&format!("{name}X"), 0),
                attrs.int_or(&format!("{name}Y"), 0),
            );
            rgba[3] = 255;
            rgba
        };
        Self {
            background: color("BkgndColor1"),
            selected_background: color("SelBkgndColor"),
            frame: color("FrameColor"),
            text: color("TextColor"),
            selected_text: color("SelTextColor"),
            current_text: color("CurTextColor"),
            played_text: color("PlayedTextColor"),
            tab_text: color("AlbumTabText"),
            tab_selected_text: color("AlbumTabSelText"),
        }
    }
}

fn font(size_px: f32) -> FontSpec {
    FontSpec {
        family: LIST_FONT.to_owned(),
        size_px,
        bold: false,
    }
}

/// Formats seconds as `hh:mm:ss`.
#[must_use]
pub fn format_duration(seconds: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// Paints the album tabs and the list into the slot of `node`
/// (`Contain='CMD_PLAY_LIST_ALBUM_TAB'`).
pub fn paint_list(
    list: &mut DisplayList,
    skin: &Skin,
    node: &Node<'_>,
    _state: &UiState,
    playlist: &Playlist,
) {
    let attrs = &node.area.attrs;
    let atlas = node.atlas;
    let colors = Colors::read(skin, atlas, attrs);
    let rect = node.rect;
    let clip = node.clip;
    list.ops.push(Op::Fill {
        rect: clip,
        color: colors.background,
    });
    let tab_height = attrs.int_or("AlbumTabH", 22);
    let tabs = Rect::new(rect.x, rect.y, rect.w, tab_height);
    paint_tabs(list, atlas, attrs, &colors, tabs, clip, playlist);
    let rows = Rect::new(rect.x, tabs.bottom(), rect.w, rect.h - tab_height);
    paint_rows(list, &colors, rows, clip, playlist);
}

/// Album tab sprites: normal, hover, selected (drawn pressed), and a
/// fourth variant. The selected tab matches PotPlayer's third sprite.
const TAB_SELECTED_SPRITE: i32 = 2;

fn paint_tabs(
    list: &mut DisplayList,
    atlas: &str,
    attrs: &Attrs,
    colors: &Colors,
    bar: Rect,
    clip: Rect,
    playlist: &Playlist,
) {
    let bar_clip = bar.intersect(&clip);
    list.nine_slice(
        atlas,
        Rect::new(
            attrs.int_or("AlbumTabX", 0),
            attrs.int_or("AlbumTabY", 0),
            attrs.int_or("AlbumTabW", 0),
            bar.h,
        ),
        [5, 0, 5, 0],
        bar,
        bar_clip,
    );
    let tab_width = attrs.int_or("AlbumTabBtnW", 70);
    let tab_height = attrs.int_or("AlbumTabBtnH", bar.h);
    let step = tab_width + attrs.int_or("AlbumTabBtnMargin", 0);
    let count = i32::try_from(playlist.albums.len()).unwrap_or(0);
    let right_arrow_width = attrs.int_or("AlbumTabRightW", 16);
    let overflow = count * step > bar.w;
    let tabs_clip = if overflow {
        Rect::new(bar.x, bar.y, bar.w - right_arrow_width, bar.h).intersect(&clip)
    } else {
        bar_clip
    };
    let mut x = bar.x;
    for (index, name) in playlist.albums.iter().enumerate() {
        let selected = index == playlist.current_album;
        let sprite = if selected { TAB_SELECTED_SPRITE } else { 0 };
        let tab = Rect::new(x, bar.y, tab_width, tab_height);
        list.image(
            atlas,
            Rect::new(
                attrs.int_or("AlbumTabBtnX", 0) + sprite * tab_width,
                attrs.int_or("AlbumTabBtnY", 0),
                tab_width,
                tab_height,
            ),
            tab,
            tabs_clip,
        );
        list.ops.push(Op::Text(TextOp {
            text: name.clone(),
            font: font(TAB_FONT_PX),
            color: if selected {
                colors.tab_selected_text
            } else {
                colors.tab_text
            },
            rect: tab,
            clip: tabs_clip,
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            ellipsis: true,
            trailing: None,
        }));
        if selected {
            let w = attrs.int_or("AlbumTabDelW", 7);
            let h = attrs.int_or("AlbumTabDelH", 6);
            list.image(
                atlas,
                Rect::new(
                    attrs.int_or("AlbumTabDelX", 0),
                    attrs.int_or("AlbumTabDelY", 0),
                    w,
                    h,
                ),
                Rect::new(tab.right() - w - 6, tab.y + (tab.h - h) / 2, w, h),
                tabs_clip,
            );
        }
        x += step;
    }
    if overflow {
        let w = right_arrow_width;
        let h = attrs.int_or("AlbumTabRightH", bar.h);
        list.image(
            atlas,
            Rect::new(
                attrs.int_or("AlbumTabRightX", 0),
                attrs.int_or("AlbumTabRightY", 0),
                w,
                h,
            ),
            Rect::new(bar.right() - w, bar.y, w, h),
            bar_clip,
        );
    }
}

/// First entry drawn and the y of its top edge.
fn first_row(scroll: ListScroll, area: Rect, total: usize) -> (usize, i32) {
    match scroll {
        ListScroll::Entry(index) => (index.min(total), area.y),
        ListScroll::End => {
            let total_height = i32::try_from(total)
                .unwrap_or(i32::MAX / ROW_HEIGHT)
                .saturating_mul(ROW_HEIGHT);
            if total_height <= area.h {
                return (0, area.y);
            }
            let top = area.bottom() - total_height;
            let skipped = (area.y - top) / ROW_HEIGHT;
            (
                usize::try_from(skipped).unwrap_or(0),
                top + skipped * ROW_HEIGHT,
            )
        }
    }
}

fn paint_rows(
    list: &mut DisplayList,
    colors: &Colors,
    area: Rect,
    clip: Rect,
    playlist: &Playlist,
) {
    let total = playlist.entries.len();
    let visible = usize::try_from((area.h / ROW_HEIGHT).max(0)).unwrap_or(0);
    let needs_scrollbar = total > visible;
    let text_right = if needs_scrollbar {
        area.right() - SCROLLBAR_WIDTH - TEXT_RIGHT_PADDING
    } else {
        area.right() - TEXT_RIGHT_PADDING
    };
    let digits = total.max(1).to_string().len();
    let area_clip = area.intersect(&clip);
    let rows_area = Rect::new(area.x, area.y, area.w, area.h - LIST_BOTTOM_MARGIN);
    let (first, first_y) = first_row(playlist.scroll, rows_area, total);
    let mut y = first_y;
    for (index, entry) in playlist.entries.iter().enumerate().skip(first) {
        if y >= area.bottom() {
            break;
        }
        let row = Rect::new(
            area.x,
            y,
            text_right - area.x + TEXT_RIGHT_PADDING,
            ROW_HEIGHT,
        );
        let selected = playlist.selected.contains(&index);
        if selected {
            list.ops.push(Op::Fill {
                rect: row.intersect(&area_clip),
                color: colors.selected_background,
            });
            paint_frame(list, row, area_clip, colors.frame);
        }
        let color = if !entry.playable {
            UNPLAYABLE_TEXT
        } else if Some(index) == playlist.current {
            colors.current_text
        } else if selected {
            colors.selected_text
        } else if entry.played {
            colors.played_text
        } else {
            colors.text
        };
        paint_icon(list, row, area_clip);
        let title = format!("{:0digits$}. {}", index + 1, entry.title);
        let trailing = if entry.playable {
            entry.duration.map(format_duration)
        } else {
            Some("<Can't open>".to_owned())
        };
        list.ops.push(Op::Text(TextOp {
            text: title,
            font: font(LIST_FONT_PX),
            color,
            rect: Rect::new(
                area.x + TEXT_LEFT,
                y,
                text_right - area.x - TEXT_LEFT,
                ROW_HEIGHT,
            ),
            clip: area_clip,
            h_align: HAlign::Left,
            v_align: VAlign::Center,
            ellipsis: true,
            trailing,
        }));
        y += ROW_HEIGHT;
    }
    if needs_scrollbar {
        let offset = area.y - (first_y - ROW_HEIGHT * i32::try_from(first).unwrap_or(0));
        let content = ROW_HEIGHT * i32::try_from(total).unwrap_or(0);
        paint_scrollbar(list, area, area_clip, offset, content);
    }
}

/// A one-pixel outline inside `rect`.
fn paint_frame(list: &mut DisplayList, rect: Rect, clip: Rect, color: [u8; 4]) {
    let edges = [
        Rect::new(rect.x, rect.y, rect.w, 1),
        Rect::new(rect.x, rect.bottom() - 1, rect.w, 1),
        Rect::new(rect.x, rect.y, 1, rect.h),
        Rect::new(rect.right() - 1, rect.y, 1, rect.h),
    ];
    for edge in edges {
        list.ops.push(Op::Fill {
            rect: edge.intersect(&clip),
            color,
        });
    }
}

/// A stand-in for PotPlayer's media file icon (it is not part of the skin):
/// a white page with a folded corner and an orange ring.
fn paint_icon(list: &mut DisplayList, row: Rect, clip: Rect) {
    let x = row.x + ICON_LEFT + 2;
    let y = row.y + (row.h - ICON_SIZE) / 2;
    let white = [238, 238, 238, 255];
    let parts = [
        (Rect::new(x, y, 9, ICON_SIZE), white),
        (Rect::new(x + 9, y + 3, 3, ICON_SIZE - 3), white),
        (Rect::new(x + 9, y, 1, 3), [160, 160, 160, 255]),
        (Rect::new(x + 3, y + 6, 6, 6), [226, 98, 32, 255]),
        (Rect::new(x + 5, y + 8, 2, 2), [30, 30, 30, 255]),
    ];
    for (rect, color) in parts {
        list.ops.push(Op::Fill {
            rect: rect.intersect(&clip),
            color,
        });
    }
}

/// Draws the scroll bar; `offset` is the scrolled distance and `content`
/// the full list height, both in pixels.
fn paint_scrollbar(list: &mut DisplayList, area: Rect, clip: Rect, offset: i32, content: i32) {
    let bar = Rect::new(
        area.right() - SCROLLBAR_WIDTH,
        area.y,
        SCROLLBAR_WIDTH,
        area.h,
    );
    list.nine_slice(SCROLLBAR_ATLAS, SCROLL_TRACK, [0, 0, 0, 0], bar, clip);
    list.image(
        SCROLLBAR_ATLAS,
        SCROLL_UP,
        Rect::new(bar.x, bar.y, SCROLL_UP.w, SCROLL_UP.h),
        clip,
    );
    list.image(
        SCROLLBAR_ATLAS,
        SCROLL_DOWN,
        Rect::new(
            bar.x,
            bar.bottom() - SCROLL_DOWN.h,
            SCROLL_DOWN.w,
            SCROLL_DOWN.h,
        ),
        clip,
    );
    let track = bar.h - SCROLL_UP.h - SCROLL_DOWN.h;
    let content = content.max(1);
    let thumb_len = (track * area.h / content).clamp(8, track.max(8));
    let range = (content - area.h).max(1);
    let thumb_y = bar.y + SCROLL_UP.h + (track - thumb_len) * offset.clamp(0, range) / range;
    list.nine_slice(
        SCROLLBAR_ATLAS,
        SCROLL_THUMB,
        [0, 3, 0, 3],
        Rect::new(bar.x, thumb_y, SCROLL_THUMB.w, thumb_len),
        clip,
    );
}

#[cfg(test)]
mod tests {
    use super::{ROW_HEIGHT, first_row, format_duration};
    use crate::geometry::Rect;
    use crate::state::ListScroll;

    #[test]
    fn durations() {
        assert_eq!(format_duration(7729), "02:08:49");
        assert_eq!(format_duration(59), "00:00:59");
    }

    #[test]
    fn scrolled_to_end_cuts_the_top_row() {
        let area = Rect::new(0, 100, 200, 5 * ROW_HEIGHT + 7);
        assert_eq!(first_row(ListScroll::End, area, 3), (0, 100));
        assert_eq!(first_row(ListScroll::End, area, 10), (4, 100 - 13));
        assert_eq!(first_row(ListScroll::Entry(2), area, 10), (2, 100));
    }
}
