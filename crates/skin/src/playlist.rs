//! The list part of the attached playlist: album tabs, rows and scroll bar.
//!
//! The skin gives colours and sprites in `PlaylistSkinArea` attributes
//! (`TextColorX`, `AlbumTabBtnX`, ...) and the scroll bar in
//! `ScrollBar.bmp`. Row height, fonts, paddings and the file icon are
//! PotPlayer internals; the constants below approximate them from
//! screenshots and are listed as such in the goal status report.
//!
//! [`geometry`] places the tabs, rows and scroll bar once; painting and
//! [`hit`] testing both use it. While rows are dragged, a two-pixel line in
//! the frame colour marks where they will land (`Playlist::drop_before`).

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

/// The parts of the scroll bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scrollbar {
    pub bar: Rect,
    pub up: Rect,
    pub down: Rect,
    /// Between the arrows.
    pub track: Rect,
    pub thumb: Rect,
}

/// Where the parts of the list are, for painting and hit testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub tab_bar: Rect,
    /// One rectangle per album; tabs beyond the bar are clipped away.
    pub tabs: Vec<Rect>,
    /// What the tabs may cover (the bar without the overflow arrow).
    pub tabs_clip: Rect,
    /// The shown album and the delete mark on its tab.
    pub shown: usize,
    pub tab_delete: Option<Rect>,
    /// The arrow at the end of the bar when the tabs do not fit.
    pub tab_overflow: Option<Rect>,
    /// The list below the tabs.
    pub area: Rect,
    /// First entry drawn and the y of its top edge.
    pub first: usize,
    pub first_y: i32,
    /// Right edge of the row text.
    pub text_right: i32,
    /// Full rows that fit.
    pub visible: usize,
    pub scrollbar: Option<Scrollbar>,
    /// `node.clip`: nothing is drawn or hit outside it.
    pub clip: Rect,
}

impl Geometry {
    /// The rectangle of row `index` (it may lie outside the area).
    #[must_use]
    pub fn row(&self, index: usize) -> Rect {
        let offset = i32::try_from(index)
            .unwrap_or(i32::MAX)
            .saturating_sub(i32::try_from(self.first).unwrap_or(0));
        Rect::new(
            self.area.x,
            self.first_y
                .saturating_add(offset.saturating_mul(ROW_HEIGHT)),
            self.text_right - self.area.x + TEXT_RIGHT_PADDING,
            ROW_HEIGHT,
        )
    }

    /// Where rows dropped at `y` land: the index they are inserted before
    /// (0 to `total`).
    #[must_use]
    pub fn insertion_index(&self, y: i32, total: usize) -> usize {
        let relative = y - self.first_y + ROW_HEIGHT / 2;
        let rows = relative.div_euclid(ROW_HEIGHT);
        let index = i64::from(rows) + i64::try_from(self.first).unwrap_or(0);
        usize::try_from(index.max(0)).unwrap_or(0).min(total)
    }
}

/// What a point in the list is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListHit {
    Tab(usize),
    /// The delete mark on the shown album's tab.
    TabDelete(usize),
    TabOverflow,
    /// The tab bar outside any tab.
    TabBar,
    Row(usize),
    /// The list below the last row.
    Empty,
    ScrollUp,
    ScrollDown,
    ScrollThumb,
    /// The track above or below the thumb.
    TrackAbove,
    TrackBelow,
}

/// Places the tabs, rows and scroll bar of the list in the slot of `node`
/// (`Contain='CMD_PLAY_LIST_ALBUM_TAB'`).
#[must_use]
pub fn geometry(node: &Node<'_>, playlist: &Playlist) -> Geometry {
    let attrs = &node.area.attrs;
    let rect = node.rect;
    let clip = node.clip;
    let tab_height = attrs.int_or("AlbumTabH", 22);
    let bar = Rect::new(rect.x, rect.y, rect.w, tab_height);
    let bar_clip = bar.intersect(&clip);
    let tab_width = attrs.int_or("AlbumTabBtnW", 70);
    let tab_button_height = attrs.int_or("AlbumTabBtnH", bar.h);
    let step = tab_width + attrs.int_or("AlbumTabBtnMargin", 0);
    let count = i32::try_from(playlist.albums.len()).unwrap_or(0);
    let right_arrow_width = attrs.int_or("AlbumTabRightW", 16);
    let overflow = count.saturating_mul(step) > bar.w;
    let tabs_clip = if overflow {
        Rect::new(bar.x, bar.y, bar.w - right_arrow_width, bar.h).intersect(&clip)
    } else {
        bar_clip
    };
    let tabs: Vec<Rect> = (0..count)
        .map(|index| Rect::new(bar.x + index * step, bar.y, tab_width, tab_button_height))
        .collect();
    let tab_delete = tabs.get(playlist.current_album).map(|tab| {
        let w = attrs.int_or("AlbumTabDelW", 7);
        let h = attrs.int_or("AlbumTabDelH", 6);
        Rect::new(tab.right() - w - 6, tab.y + (tab.h - h) / 2, w, h)
    });
    let tab_overflow = overflow.then(|| {
        Rect::new(
            bar.right() - right_arrow_width,
            bar.y,
            right_arrow_width,
            attrs.int_or("AlbumTabRightH", bar.h),
        )
    });
    let area = Rect::new(rect.x, bar.bottom(), rect.w, rect.h - tab_height);
    let total = playlist.entries.len();
    let visible = usize::try_from((area.h / ROW_HEIGHT).max(0)).unwrap_or(0);
    let needs_scrollbar = total > visible;
    let text_right = if needs_scrollbar {
        area.right() - SCROLLBAR_WIDTH - TEXT_RIGHT_PADDING
    } else {
        area.right() - TEXT_RIGHT_PADDING
    };
    let rows_area = Rect::new(area.x, area.y, area.w, area.h - LIST_BOTTOM_MARGIN);
    let (first, first_y) = first_row(playlist.scroll, rows_area, total);
    let scrollbar = needs_scrollbar.then(|| {
        let offset = area.y - (first_y - ROW_HEIGHT * i32::try_from(first).unwrap_or(0));
        let content = ROW_HEIGHT * i32::try_from(total).unwrap_or(0);
        scrollbar(area, offset, content)
    });
    Geometry {
        tab_bar: bar,
        tabs,
        tabs_clip,
        shown: playlist.current_album,
        tab_delete,
        tab_overflow,
        area,
        first,
        first_y,
        text_right,
        visible,
        scrollbar,
        clip,
    }
}

/// What `(x, y)` is over, if it lies in the list.
#[must_use]
pub fn hit(geometry: &Geometry, total: usize, x: i32, y: i32) -> Option<ListHit> {
    if !geometry.clip.contains(x, y) {
        return None;
    }
    if geometry.tab_bar.contains(x, y) {
        if geometry
            .tab_overflow
            .is_some_and(|arrow| arrow.contains(x, y))
        {
            return Some(ListHit::TabOverflow);
        }
        if !geometry.tabs_clip.contains(x, y) {
            return Some(ListHit::TabBar);
        }
        if geometry
            .tab_delete
            .is_some_and(|mark| expand(mark, 3).contains(x, y))
        {
            return Some(ListHit::TabDelete(geometry.shown));
        }
        return Some(
            match geometry.tabs.iter().position(|tab| tab.contains(x, y)) {
                Some(index) => ListHit::Tab(index),
                None => ListHit::TabBar,
            },
        );
    }
    if !geometry.area.contains(x, y) {
        return None;
    }
    if let Some(bar) = geometry.scrollbar
        && bar.bar.contains(x, y)
    {
        return Some(if bar.up.contains(x, y) {
            ListHit::ScrollUp
        } else if bar.down.contains(x, y) {
            ListHit::ScrollDown
        } else if bar.thumb.contains(x, y) {
            ListHit::ScrollThumb
        } else if y < bar.thumb.y {
            ListHit::TrackAbove
        } else {
            ListHit::TrackBelow
        });
    }
    let relative = y - geometry.first_y;
    let index =
        i64::from(relative.div_euclid(ROW_HEIGHT)) + i64::try_from(geometry.first).unwrap_or(0);
    Some(match usize::try_from(index) {
        Ok(index) if index < total => ListHit::Row(index),
        _ => ListHit::Empty,
    })
}

/// `rect` grown by `by` pixels on every side (small marks are hard to hit).
fn expand(rect: Rect, by: i32) -> Rect {
    Rect::new(rect.x - by, rect.y - by, rect.w + 2 * by, rect.h + 2 * by)
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
    let geometry = geometry(node, playlist);
    list.ops.push(Op::Fill {
        rect: node.clip,
        color: colors.background,
    });
    paint_tabs(list, atlas, attrs, &colors, &geometry, playlist);
    paint_rows(list, &colors, &geometry, playlist);
}

/// Album tab sprites: normal, hover, selected (drawn pressed), and a
/// fourth variant. The selected tab matches PotPlayer's third sprite.
const TAB_SELECTED_SPRITE: i32 = 2;

fn paint_tabs(
    list: &mut DisplayList,
    atlas: &str,
    attrs: &Attrs,
    colors: &Colors,
    geometry: &Geometry,
    playlist: &Playlist,
) {
    let bar = geometry.tab_bar;
    let bar_clip = bar.intersect(&geometry.clip);
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
    let tabs_clip = geometry.tabs_clip;
    for (index, (name, tab)) in playlist.albums.iter().zip(&geometry.tabs).enumerate() {
        let selected = index == playlist.current_album;
        let sprite = if selected { TAB_SELECTED_SPRITE } else { 0 };
        list.image(
            atlas,
            Rect::new(
                attrs.int_or("AlbumTabBtnX", 0) + sprite * tab.w,
                attrs.int_or("AlbumTabBtnY", 0),
                tab.w,
                tab.h,
            ),
            *tab,
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
            rect: *tab,
            clip: tabs_clip,
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            ellipsis: true,
            trailing: None,
        }));
        if selected && let Some(mark) = geometry.tab_delete {
            list.image(
                atlas,
                Rect::new(
                    attrs.int_or("AlbumTabDelX", 0),
                    attrs.int_or("AlbumTabDelY", 0),
                    mark.w,
                    mark.h,
                ),
                mark,
                tabs_clip,
            );
        }
    }
    if let Some(arrow) = geometry.tab_overflow {
        list.image(
            atlas,
            Rect::new(
                attrs.int_or("AlbumTabRightX", 0),
                attrs.int_or("AlbumTabRightY", 0),
                arrow.w,
                arrow.h,
            ),
            arrow,
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

fn paint_rows(list: &mut DisplayList, colors: &Colors, geometry: &Geometry, playlist: &Playlist) {
    let area = geometry.area;
    let total = playlist.entries.len();
    let digits = total.max(1).to_string().len();
    let area_clip = area.intersect(&geometry.clip);
    for (index, entry) in playlist.entries.iter().enumerate().skip(geometry.first) {
        let row = geometry.row(index);
        if row.y >= area.bottom() {
            break;
        }
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
                row.y,
                geometry.text_right - area.x - TEXT_LEFT,
                ROW_HEIGHT,
            ),
            clip: area_clip,
            h_align: HAlign::Left,
            v_align: VAlign::Center,
            ellipsis: true,
            trailing,
        }));
    }
    if let Some(before) = playlist.drop_before {
        let y = geometry.row(before).y - 1;
        list.ops.push(Op::Fill {
            rect: Rect::new(area.x, y, geometry.text_right - area.x, 2).intersect(&area_clip),
            color: colors.frame,
        });
    }
    if let Some(bar) = geometry.scrollbar {
        paint_scrollbar(list, &bar, area_clip);
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

/// Places the scroll bar; `offset` is the scrolled distance and `content`
/// the full list height, both in pixels.
fn scrollbar(area: Rect, offset: i32, content: i32) -> Scrollbar {
    let bar = Rect::new(
        area.right() - SCROLLBAR_WIDTH,
        area.y,
        SCROLLBAR_WIDTH,
        area.h,
    );
    let up = Rect::new(bar.x, bar.y, SCROLL_UP.w, SCROLL_UP.h);
    let down = Rect::new(
        bar.x,
        bar.bottom() - SCROLL_DOWN.h,
        SCROLL_DOWN.w,
        SCROLL_DOWN.h,
    );
    let track_len = bar.h - SCROLL_UP.h - SCROLL_DOWN.h;
    let content = content.max(1);
    let thumb_len = (track_len * area.h / content).clamp(8, track_len.max(8));
    let range = (content - area.h).max(1);
    let thumb_y = bar.y + SCROLL_UP.h + (track_len - thumb_len) * offset.clamp(0, range) / range;
    Scrollbar {
        bar,
        up,
        down,
        track: Rect::new(bar.x, up.bottom(), bar.w, track_len),
        thumb: Rect::new(bar.x, thumb_y, SCROLL_THUMB.w, thumb_len),
    }
}

fn paint_scrollbar(list: &mut DisplayList, bar: &Scrollbar, clip: Rect) {
    list.nine_slice(SCROLLBAR_ATLAS, SCROLL_TRACK, [0, 0, 0, 0], bar.bar, clip);
    list.image(SCROLLBAR_ATLAS, SCROLL_UP, bar.up, clip);
    list.image(SCROLLBAR_ATLAS, SCROLL_DOWN, bar.down, clip);
    list.nine_slice(SCROLLBAR_ATLAS, SCROLL_THUMB, [0, 3, 0, 3], bar.thumb, clip);
}

#[cfg(test)]
mod tests {
    use super::{Geometry, ListHit, ROW_HEIGHT, first_row, format_duration, hit, scrollbar};
    use crate::geometry::Rect;
    use crate::state::ListScroll;

    /// A 200x(22 + 5 rows) list with 3 tabs, scrolled to entry 2 of 10.
    fn sample() -> Geometry {
        let area = Rect::new(0, 22, 200, 5 * ROW_HEIGHT);
        Geometry {
            tab_bar: Rect::new(0, 0, 200, 22),
            tabs: vec![
                Rect::new(0, 0, 70, 22),
                Rect::new(70, 0, 70, 22),
                Rect::new(140, 0, 70, 22),
            ],
            tabs_clip: Rect::new(0, 0, 184, 22),
            shown: 1,
            tab_delete: Some(Rect::new(127, 8, 7, 6)),
            tab_overflow: Some(Rect::new(184, 0, 16, 22)),
            area,
            first: 2,
            first_y: 22,
            text_right: 184,
            visible: 5,
            scrollbar: Some(scrollbar(area, 2 * ROW_HEIGHT, 10 * ROW_HEIGHT)),
            clip: Rect::new(0, 0, 200, 22 + 5 * ROW_HEIGHT),
        }
    }

    #[test]
    fn hits_tabs_rows_and_the_scroll_bar() {
        let geometry = sample();
        assert_eq!(hit(&geometry, 10, 10, 10), Some(ListHit::Tab(0)));
        assert_eq!(hit(&geometry, 10, 129, 11), Some(ListHit::TabDelete(1)));
        assert_eq!(hit(&geometry, 10, 190, 10), Some(ListHit::TabOverflow));
        // Rows start at entry 2.
        assert_eq!(hit(&geometry, 10, 50, 25), Some(ListHit::Row(2)));
        assert_eq!(
            hit(&geometry, 10, 50, 22 + ROW_HEIGHT + 3),
            Some(ListHit::Row(3))
        );
        assert_eq!(
            hit(&geometry, 4, 50, 22 + 3 * ROW_HEIGHT),
            Some(ListHit::Empty)
        );
        assert_eq!(hit(&geometry, 10, 195, 25), Some(ListHit::ScrollUp));
        assert_eq!(
            hit(&geometry, 10, 195, 22 + 5 * ROW_HEIGHT - 2),
            Some(ListHit::ScrollDown)
        );
        let thumb = geometry.scrollbar.expect("scroll bar").thumb;
        assert_eq!(
            hit(&geometry, 10, 195, thumb.y + 1),
            Some(ListHit::ScrollThumb)
        );
        assert_eq!(
            hit(&geometry, 10, 195, thumb.bottom() + 1),
            Some(ListHit::TrackBelow)
        );
        assert_eq!(hit(&geometry, 10, 250, 25), None);
    }

    #[test]
    fn drops_land_between_rows() {
        let geometry = sample();
        // Upper half of row 2 (the first shown): before 2; lower half: before 3.
        assert_eq!(geometry.insertion_index(23, 10), 2);
        assert_eq!(geometry.insertion_index(22 + ROW_HEIGHT - 2, 10), 3);
        assert_eq!(geometry.insertion_index(-500, 10), 0);
        assert_eq!(geometry.insertion_index(5000, 10), 10);
        assert_eq!(geometry.row(4).y, 22 + 2 * ROW_HEIGHT);
    }

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
