//! Places skin elements in a window.
//!
//! The model, checked against PotPlayer screenshots of BlackSpace:
//!
//! * Children are placed in document order. `Align='Top'`, `Bottom`, `Left`
//!   and `Right` dock to the matching edge of the space that is still free
//!   and take it away from later siblings; `Client` takes what is left.
//!   Document order matters: in BlackSpace the playlist (docked right before
//!   the bottom bar) runs down to the window border, and the bottom bar only
//!   spans the video.
//! * Undocked children use `Left`, `Top`, `Width`, `Height` relative to the
//!   parent. Negative `Left`/`Top` count from the right/bottom edge.
//!   `Width`/`Height` of zero or less put the right/bottom edge at that
//!   distance from the parent's right/bottom edge (`Width='-100'` ends 100
//!   pixels before it). `HoriCenter`/`VertCenter` centre the element and use
//!   `Left`/`Top` as an offset from the centre.
//! * `ConstSize` overrides `Width` (or `Height`) of docked elements.
//!   `OffsetLeft`, `OffsetTop`, `OffsetWidth`, `OffsetHeight` adjust the final
//!   rectangle without changing the free space.
//! * Hidden elements take no space: paired commands (Play/Pause,
//!   Maximize/Restore), `State` conditions, `Contain` slots,
//!   `AutoVisible` (the command's value, `UiState::auto_value`),
//!   `VisParentMinWidth`, `VisParentMinHeight`, `VisParentMaxWidth` and
//!   `VisParentMaxHeight` (kmp1's small buttons in a narrow window and its
//!   labelled ones in a wide one). `VisMinLeft`, `VisMinWidth` and
//!   `VisMinHeight` hide an element after it is placed (its space is
//!   taken): left of that place, or narrower or lower than that. Controls
//!   of Daum's pop-up windows (`CMD_POPUP_WINDOW`, BlackSpace's audio
//!   skin's "screen partition" button) are hidden as their slots are
//!   (goal P's host session: PotPlayer shows neither).
//! * An attached playlist fills the `Contain='CMD_PLAYLIST_TOGGLE'` slot with
//!   the `CMD_PLAYLIST_HOLDER` part of `PlaylistSkinArea`; the playlist
//!   window's own frame is not drawn.
//! * An area's parts lie in its `<Control>`; the `<Logo>` section (M14)
//!   holds them directly. An element with an `Image` attribute of its own
//!   draws from that picture, and so do its children (Default.dsf's logo
//!   parts); the others draw from their area's.

use std::fmt::Write as _;

use crate::error::SkinError;
use crate::geometry::Rect;
use crate::markup::{Attrs, Element};
use crate::skin::Skin;
use crate::state::UiState;

/// The kinds of controls the layout places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Static,
    Button,
    Label,
    PicLabel,
    Image,
    Slider,
    /// `Visual`: the audio's spectrum as bars (M14).
    Visual,
    /// `RvpGraph`: the equalizer's response curve (M9, classic skins).
    Graph,
    /// `Layout`: an invisible spacer.
    Spacer,
}

impl Kind {
    fn of(element: &Element) -> Option<Self> {
        const KINDS: [(&str, Kind); 9] = [
            ("Static", Kind::Static),
            ("Button", Kind::Button),
            ("Label", Kind::Label),
            ("PicLabel", Kind::PicLabel),
            ("Image", Kind::Image),
            ("Slider", Kind::Slider),
            ("Visual", Kind::Visual),
            ("RvpGraph", Kind::Graph),
            ("Layout", Kind::Spacer),
        ];
        KINDS
            .iter()
            .find(|(name, _)| element.is(name))
            .map(|(_, kind)| *kind)
    }
}

/// Docking mode of an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Top,
    Bottom,
    Left,
    Right,
    Client,
}

impl Align {
    fn read(attrs: &Attrs) -> Self {
        match attrs.get("Align").map(str::trim) {
            Some(value) if value.eq_ignore_ascii_case("Top") => Self::Top,
            Some(value) if value.eq_ignore_ascii_case("Bottom") => Self::Bottom,
            Some(value) if value.eq_ignore_ascii_case("Left") => Self::Left,
            Some(value) if value.eq_ignore_ascii_case("Right") => Self::Right,
            Some(value) if value.eq_ignore_ascii_case("Client") => Self::Client,
            _ => Self::None,
        }
    }
}

/// One placed element.
#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub element: &'a Element,
    pub kind: Kind,
    /// Rectangle in window coordinates.
    pub rect: Rect,
    /// `rect` clipped by every ancestor.
    pub clip: Rect,
    /// Index of the parent node, if any.
    pub parent: Option<usize>,
    pub depth: usize,
    /// Atlas image of the skin area the element belongs to.
    pub atlas: &'a str,
    /// The skin area the element belongs to.
    pub area: &'a Element,
}

impl Node<'_> {
    /// The `CmdID` of the element, if set.
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        self.element
            .attrs
            .get("CmdID")
            .filter(|cmd| !cmd.is_empty())
    }

    /// The `Contain` slot name of the element, if set.
    #[must_use]
    pub fn slot(&self) -> Option<&str> {
        self.element
            .attrs
            .get("Contain")
            .filter(|slot| !slot.is_empty())
    }
}

/// All visible elements of one skin area at one window size, in drawing
/// order (parents before children, earlier siblings first).
#[derive(Debug, Clone)]
pub struct Layout<'a> {
    pub width: i32,
    pub height: i32,
    pub nodes: Vec<Node<'a>>,
}

impl Layout<'_> {
    /// The rectangle of the first visible slot with this `Contain` name.
    #[must_use]
    pub fn slot(&self, name: &str) -> Option<Rect> {
        self.nodes
            .iter()
            .find(|node| node.slot() == Some(name))
            .map(|node| node.rect)
    }

    /// The first visible node bound to `command`.
    #[must_use]
    pub fn find_command(&self, command: &str) -> Option<&Node<'_>> {
        self.nodes
            .iter()
            .find(|node| node.command() == Some(command))
    }

    /// A stable text listing of the layout, one node per line, for
    /// geometry snapshots.
    #[must_use]
    pub fn dump(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "window {}x{}", self.width, self.height);
        for node in &self.nodes {
            let label = node.command().or_else(|| node.slot()).unwrap_or("-");
            let _ = writeln!(
                out,
                "{:indent$}{} {} {}",
                "",
                node.element.name,
                label,
                node.rect,
                indent = node.depth * 2
            );
        }
        out
    }
}

/// Lays out the skin area `area_name` (for example `MainSkinArea`) in a
/// window of `width` x `height` pixels.
///
/// # Errors
/// Fails when the skin has no such area.
pub fn layout<'a>(
    skin: &'a Skin,
    area_name: &str,
    width: i32,
    height: i32,
    state: &UiState,
) -> Result<Layout<'a>, SkinError> {
    let area = skin
        .area(area_name)
        .ok_or_else(|| SkinError::MissingArea(area_name.to_owned()))?;
    let mut builder = Builder {
        skin,
        state,
        nodes: Vec::new(),
    };
    let window = Rect::new(0, 0, width, height);
    let parts = area.child("Control").unwrap_or(area);
    builder.children(parts, window, window, None, 0, skin.area_atlas(area), area);
    Ok(Layout {
        width,
        height,
        nodes: builder.nodes,
    })
}

struct Builder<'a, 's> {
    skin: &'a Skin,
    state: &'s UiState,
    nodes: Vec<Node<'a>>,
}

impl<'a> Builder<'a, '_> {
    #[allow(clippy::too_many_arguments)]
    fn children(
        &mut self,
        container: &'a Element,
        rect: Rect,
        clip: Rect,
        parent: Option<usize>,
        depth: usize,
        atlas: &'a str,
        area: &'a Element,
    ) {
        let mut free = rect;
        for child in &container.children {
            if child.is("Control") {
                self.children(child, rect, clip, parent, depth, atlas, area);
                continue;
            }
            let Some(kind) = Kind::of(child) else {
                continue;
            };
            if !self.visible(child, rect) {
                continue;
            }
            let placed = place(&child.attrs, rect, &mut free);
            if let Some(min_left) = child.attrs.int("VisMinLeft")
                && placed.x - rect.x < min_left
            {
                continue;
            }
            if child
                .attrs
                .int("VisMinWidth")
                .is_some_and(|min| placed.w < min)
                || child
                    .attrs
                    .int("VisMinHeight")
                    .is_some_and(|min| placed.h < min)
            {
                continue;
            }
            let node_clip = placed.intersect(&clip);
            let atlas = child
                .attrs
                .get("Image")
                .filter(|image| !image.trim().is_empty())
                .unwrap_or(atlas);
            let index = self.nodes.len();
            self.nodes.push(Node {
                element: child,
                kind,
                rect: placed,
                clip: node_clip,
                parent,
                depth,
                atlas,
                area,
            });
            let slot = child.attrs.get("Contain").unwrap_or("");
            // The playlist goes into the container; the splitter next to it
            // shares the slot name.
            if (slot == "CMD_PLAYLIST_TOGGLE" || slot == "CMD_CHAT_PLAYLIST_BROADCAST_TOGGLE")
                && !child.attrs.flag("IsSplitter")
            {
                self.attached_playlist(placed, node_clip, index, depth + 1);
            }
            // VLC's panels (M9) are boxes for placing, not clips: their
            // children may lie outside them (`RvpNoClip`).
            let child_clip = if child.attrs.flag("RvpNoClip") {
                clip
            } else {
                node_clip
            };
            self.children(
                child,
                placed,
                child_clip,
                Some(index),
                depth + 1,
                atlas,
                area,
            );
        }
    }

    fn attached_playlist(&mut self, rect: Rect, clip: Rect, parent: usize, depth: usize) {
        let Some(area) = self.skin.area("PlaylistSkinArea") else {
            return;
        };
        let Some(holder) = area.find("Control").and_then(|control| {
            let mut found = None;
            control.walk(&mut |element| {
                if found.is_none() && element.attrs.get("Contain") == Some("CMD_PLAYLIST_HOLDER") {
                    found = Some(element);
                }
            });
            found
        }) else {
            return;
        };
        let atlas = self.skin.area_atlas(area);
        let index = self.nodes.len();
        self.nodes.push(Node {
            element: holder,
            kind: Kind::Static,
            rect,
            clip,
            parent: Some(parent),
            depth,
            atlas,
            area,
        });
        self.children(holder, rect, clip, Some(index), depth + 1, atlas, area);
    }

    fn visible(&self, element: &Element, parent: Rect) -> bool {
        let attrs = &element.attrs;
        if let Some(command) = attrs.get("CmdID")
            && !self.state.command_visible(command)
        {
            return false;
        }
        if let Some(slot) = attrs.get("Contain").filter(|slot| !slot.is_empty())
            && !self.state.slot_visible(slot)
        {
            return false;
        }
        if let Some(states) = attrs
            .get("State")
            .filter(|states| !states.trim().is_empty())
        {
            let any = states.split_whitespace().any(|name| {
                if name == "CMD_PAGE_CONTROL" {
                    attrs.int_or("PageIndex", 0) == self.state.current_page
                } else {
                    self.state.flag(name)
                }
            });
            if !any {
                return false;
            }
        }
        if let Some(auto) = attrs.int("AutoVisible").filter(|auto| *auto != 0)
            && self.state.auto_value(attrs.get("CmdID").unwrap_or("")) != auto
        {
            return false;
        }
        let size = |name: &str| attrs.int(name);
        !(size("VisParentMinWidth").is_some_and(|min| parent.w < min)
            || size("VisParentMinHeight").is_some_and(|min| parent.h < min)
            || size("VisParentMaxWidth").is_some_and(|max| parent.w > max)
            || size("VisParentMaxHeight").is_some_and(|max| parent.h > max))
    }
}

/// Computes the rectangle of one element and updates the free space.
fn place(attrs: &Attrs, parent: Rect, free: &mut Rect) -> Rect {
    let docked_width = attrs
        .int("ConstSize")
        .unwrap_or_else(|| attrs.int_or("Width", 0));
    let docked_height = attrs
        .int("ConstSize")
        .unwrap_or_else(|| attrs.int_or("Height", 0));
    let mut rect = match Align::read(attrs) {
        Align::Top => {
            let h = docked_height.clamp(0, free.h);
            let rect = Rect::new(free.x, free.y, free.w, h);
            *free = Rect::new(free.x, free.y + h, free.w, free.h - h);
            rect
        }
        Align::Bottom => {
            let h = docked_height.clamp(0, free.h);
            let rect = Rect::new(free.x, free.bottom() - h, free.w, h);
            *free = Rect::new(free.x, free.y, free.w, free.h - h);
            rect
        }
        Align::Left => {
            let w = docked_width.clamp(0, free.w);
            let rect = Rect::new(free.x, free.y, w, free.h);
            *free = Rect::new(free.x + w, free.y, free.w - w, free.h);
            rect
        }
        Align::Right => {
            let w = docked_width.clamp(0, free.w);
            let rect = Rect::new(free.right() - w, free.y, w, free.h);
            *free = Rect::new(free.x, free.y, free.w - w, free.h);
            rect
        }
        Align::Client => *free,
        Align::None => {
            let (x, w) = axis(
                attrs.int_or("Left", 0),
                attrs.int_or("Width", 0),
                parent.w,
                attrs.flag("HoriCenter"),
            );
            let (y, h) = axis(
                attrs.int_or("Top", 0),
                attrs.int_or("Height", 0),
                parent.h,
                attrs.flag("VertCenter"),
            );
            // VLC's corners (M9): an edge tied to the parent's far edge,
            // at a signed distance from it, even outside the parent.
            let (x, w) = far(attrs, "RvpLeftFar", "RvpRightFar", parent.w, x, w);
            let (y, h) = far(attrs, "RvpTopFar", "RvpBottomFar", parent.h, y, h);
            Rect::new(parent.x + x, parent.y + y, w, h)
        }
    };
    rect = Rect::new(
        rect.x + attrs.int_or("OffsetLeft", 0),
        rect.y + attrs.int_or("OffsetTop", 0),
        rect.w + attrs.int_or("OffsetWidth", 0),
        rect.h + attrs.int_or("OffsetHeight", 0),
    );
    rect
}

/// One axis with VLC's far edges (`RvpLeftFar` and the like): the start
/// or end at that signed distance from the parent's far edge.
fn far(
    attrs: &crate::markup::Attrs,
    start: &str,
    end: &str,
    parent: i32,
    offset: i32,
    size: i32,
) -> (i32, i32) {
    let begin = attrs
        .int(start)
        .map_or(offset, |distance| parent + distance);
    let finish = attrs
        .int(end)
        .map_or(begin + size, |distance| parent + distance);
    if attrs.has(start) || attrs.has(end) {
        (begin, (finish - begin).max(0))
    } else {
        (offset, size)
    }
}

/// Resolves one axis of an undocked element: returns (offset, size).
fn axis(start: i32, size: i32, parent: i32, centered: bool) -> (i32, i32) {
    let offset = if centered && size > 0 {
        (parent - size) / 2 + start
    } else if start < 0 {
        parent + start
    } else {
        start
    };
    let size = if size > 0 {
        size
    } else {
        parent + size - offset
    };
    (offset, size)
}

#[cfg(test)]
mod tests {
    use super::axis;

    #[test]
    fn negative_coordinates_count_from_the_far_edge() {
        assert_eq!(axis(-100, 30, 1000, false), (900, 30));
        assert_eq!(axis(70, -100, 1000, false), (70, 830));
        assert_eq!(axis(7, -127, 1000, false), (7, 866));
        assert_eq!(axis(0, 0, 1000, false), (0, 1000));
        assert_eq!(axis(-54, 35, 1000, true), (428, 35));
    }
}
