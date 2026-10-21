//! The skin as the window shows it: the layout for the current size and
//! mode, the display list, and its split into one static layer and small
//! dynamic regions (goal C rule 1).
//!
//! Every control that can change (buttons, sliders, labels, the playlist)
//! has its own region. A region item paints every operation that touches
//! its region, backgrounds included, so it covers the static layer
//! completely there. Regions may overlap (BlackSpace's Play button reaches
//! one pixel into the seek bar): their items stack in drawing order, and a
//! region whose neighbour changes inside their overlap has changed
//! operations itself, so it repaints too. After a change
//! [`Chrome::refresh`] compares each region's operations with the last
//! painted ones and reports only the regions that differ; the static layer
//! repaints only when an operation outside every region changes (window
//! activation, a new size). Small regions keep repaints small: merged into
//! one, the whole bottom bar repainted with every video frame.

use std::sync::Arc;

use rvp_skin::hit::{Hit, hit_test};
use rvp_skin::layout::Kind;
use rvp_skin::paint::Op;
use rvp_skin::{DataModel, DisplayList, Layout, Rect, Skin, SkinError, UiState, layout, paint};

/// Which skin area the window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The classic window (`MainSkinArea`).
    Window,
    /// Fullscreen: the bars of `MainSkinAreaOscInside` over the video.
    Fullscreen,
    /// The control panel (F7) in a window of its own
    /// (`ControlBoxSkinArea`).
    ControlBox,
    /// The skin's logo over the video area while nothing plays or a file
    /// starts (`<Logo>`, M14).
    Logo,
}

impl Mode {
    #[must_use]
    pub fn area(self) -> &'static str {
        match self {
            Mode::Window => "MainSkinArea",
            Mode::Fullscreen => "MainSkinAreaOscInside",
            Mode::ControlBox => "ControlBoxSkinArea",
            Mode::Logo => "Logo",
        }
    }

    /// The size the mode's area asks for (`Width` and `Height`), when it
    /// sets one: BlackSpace's control panel is 300x159.
    #[must_use]
    pub fn area_size(self, skin: &Skin) -> Option<(i32, i32)> {
        let area = skin.area(self.area())?;
        let size = (area.attrs.int("Width")?, area.attrs.int("Height")?);
        (size.0 > 0 && size.1 > 0).then_some(size)
    }

    /// The window's limits the mode's area sets (M14): `WidthLimit` and
    /// `HeightLimit` are the smallest window (0: no limit of the skin's),
    /// and `ForceMinWidth` or `ForceMinHeight` shrink the window to that
    /// side's smallest size when the skin mode comes (BlackSpace's audio
    /// skin: the bars without the video).
    #[must_use]
    pub fn limits(self, skin: &Skin) -> WindowLimits {
        let Some(area) = skin.area(self.area()) else {
            return WindowLimits::default();
        };
        let attrs = &area.attrs;
        WindowLimits {
            width: attrs.int_or("WidthLimit", 0).max(0),
            height: attrs.int_or("HeightLimit", 0).max(0),
            force_width: attrs.int_or("ForceMinWidth", 0) == 1,
            force_height: attrs.int_or("ForceMinHeight", 0) == 1,
        }
    }
}

/// See [`Mode::limits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowLimits {
    pub width: i32,
    pub height: i32,
    pub force_width: bool,
    pub force_height: bool,
}

/// What the painters need: the display list and the region rectangles,
/// shared with the render thread.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub list: DisplayList,
    pub regions: Vec<Rect>,
}

impl Frame {
    /// Operations a region item paints: every operation that touches the
    /// region (`None` = the static layer: every operation not inside a
    /// region, since region items cover the rest).
    pub fn ops_for(&self, region: Option<usize>) -> impl Iterator<Item = &Op> {
        let area = region.and_then(|index| self.regions.get(index).copied());
        let regions = &self.regions;
        self.list.ops.iter().filter(move |op| {
            let bounds = op_bounds(op);
            match area {
                Some(area) => !bounds.intersect(&area).is_empty(),
                None => region.is_none() && !inside_any(regions, &bounds),
            }
        })
    }
}

/// What has to be repainted after a change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Damage {
    pub static_layer: bool,
    /// Indexes into [`Frame::regions`].
    pub regions: Vec<usize>,
    /// The set of regions changed: the shell recreates its region items.
    pub regions_changed: bool,
}

impl Damage {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.static_layer && self.regions.is_empty() && !self.regions_changed
    }
}

/// The skin state of one window.
pub struct Chrome {
    /// Over the video, the video keeps to the bars' video slot (Skins >
    /// "Auto resize video depending on fullscreen pop-ups" while they show).
    video_in_slot: bool,
    skin: &'static Skin,
    mode: Mode,
    width: i32,
    height: i32,
    layout: Option<Layout<'static>>,
    /// The state the layout was computed for, without hover and pressed.
    layout_state: UiState,
    pub state: UiState,
    pub data: DataModel,
    frame: Arc<Frame>,
    painted_regions: Vec<Vec<Op>>,
    painted_static: Vec<Op>,
}

/// Bounds of what an operation can touch.
fn op_bounds(op: &Op) -> Rect {
    match op {
        Op::Image { dst, clip, .. } => dst.intersect(clip),
        Op::Fill { rect, .. } => *rect,
        Op::Text(text) => text.rect.intersect(&text.clip),
    }
}

fn contains(outer: &Rect, inner: &Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}

fn inside_any(regions: &[Rect], bounds: &Rect) -> bool {
    bounds.is_empty() || regions.iter().any(|region| contains(region, bounds))
}

/// The rectangles of everything that can change, one per control, in
/// drawing order (later ones lie above earlier ones). A region inside a
/// later one is dropped: the later one covers it completely.
fn layered(candidates: &[Rect]) -> Vec<Rect> {
    let mut regions: Vec<Rect> = Vec::new();
    for (index, rect) in candidates.iter().enumerate() {
        let covered = candidates[index + 1..]
            .iter()
            .any(|later| contains(later, rect));
        if !covered && !regions.contains(rect) {
            regions.push(*rect);
        }
    }
    regions
}

fn dynamic_regions(layout: &Layout<'_>) -> Vec<Rect> {
    let candidates: Vec<Rect> = layout
        .nodes
        .iter()
        .filter(|node| match node.kind {
            Kind::Button | Kind::Slider | Kind::Label | Kind::PicLabel => true,
            Kind::Image => node.command().is_some(),
            Kind::Static | Kind::Spacer => node.slot() == Some("CMD_PLAY_LIST_ALBUM_TAB"),
        })
        .map(|node| node.clip)
        .filter(|rect| !rect.is_empty())
        .collect();
    layered(&candidates)
}

/// Interface state without the pointer: what the layout depends on.
fn layout_key(state: &UiState) -> UiState {
    UiState {
        hover: None,
        pressed: None,
        ..state.clone()
    }
}

impl Chrome {
    #[must_use]
    pub fn new(skin: &'static Skin) -> Self {
        Self {
            skin,
            mode: Mode::Window,
            video_in_slot: false,
            width: 0,
            height: 0,
            layout: None,
            layout_state: UiState::default(),
            state: UiState::default(),
            data: DataModel::default(),
            frame: Arc::new(Frame::default()),
            painted_regions: Vec::new(),
            painted_static: Vec::new(),
        }
    }

    #[must_use]
    pub fn skin(&self) -> &'static Skin {
        self.skin
    }

    /// Keeps the video in the bars' video slot over the video (`true`) or
    /// gives it the whole window; true when that changed.
    pub fn set_video_in_slot(&mut self, in_slot: bool) -> bool {
        let changed = self.video_in_slot != in_slot;
        self.video_in_slot = in_slot;
        changed
    }

    /// Shows another skin (M14: the skin list, the audio skin, a colour
    /// theme); the state but the pointer's carries over. The next
    /// [`Chrome::resize`] lays it out.
    pub fn set_skin(&mut self, skin: &'static Skin) {
        self.skin = skin;
        self.layout = None;
        self.state.hover = None;
        self.state.pressed = None;
        self.painted_regions.clear();
        self.painted_static.clear();
    }

    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[must_use]
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Lays the skin out for a new size or mode; everything is repainted.
    ///
    /// # Errors
    /// Fails when the skin lacks the mode's area; the old layout is kept.
    pub fn resize(&mut self, mode: Mode, width: i32, height: i32) -> Result<Damage, SkinError> {
        let placed = layout(self.skin, mode.area(), width, height, &self.state)?;
        self.mode = mode;
        self.width = width;
        self.height = height;
        self.layout = Some(placed);
        self.layout_state = layout_key(&self.state);
        self.painted_regions.clear();
        self.painted_static.clear();
        let mut damage = self.repaint();
        damage.regions_changed = true;
        damage.static_layer = true;
        damage.regions = (0..self.frame.regions.len()).collect();
        Ok(damage)
    }

    /// Repaints after `state` or `data` changed and reports what differs.
    pub fn refresh(&mut self) -> Damage {
        if self.layout.is_some() && layout_key(&self.state) != self.layout_state {
            // Visibility depends on the state (Play/Pause, Maximize/Restore).
            if let Ok(placed) = layout(
                self.skin,
                self.mode.area(),
                self.width,
                self.height,
                &self.state,
            ) {
                self.layout = Some(placed);
            }
            self.layout_state = layout_key(&self.state);
        }
        self.repaint()
    }

    fn repaint(&mut self) -> Damage {
        let Some(placed) = &self.layout else {
            return Damage::default();
        };
        let list = paint(self.skin, placed, &self.state, &self.data);
        let regions = dynamic_regions(placed);
        // `resize` forgets what was painted: the same regions count as new.
        let regions_changed =
            regions != self.frame.regions || self.painted_regions.len() != regions.len();
        let frame = Frame { list, regions };
        let mut damage = Damage {
            regions_changed,
            ..Damage::default()
        };
        let static_ops: Vec<Op> = frame.ops_for(None).cloned().collect();
        if regions_changed || static_ops != self.painted_static {
            damage.static_layer = true;
            self.painted_static = static_ops;
        }
        if regions_changed {
            self.painted_regions = vec![Vec::new(); frame.regions.len()];
        }
        for index in 0..frame.regions.len() {
            let ops: Vec<Op> = frame.ops_for(Some(index)).cloned().collect();
            if regions_changed || ops != self.painted_regions[index] {
                damage.regions.push(index);
                self.painted_regions[index] = ops;
            }
        }
        self.frame = Arc::new(frame);
        damage
    }

    /// The current frame for the painters.
    #[must_use]
    pub fn frame(&self) -> Arc<Frame> {
        Arc::clone(&self.frame)
    }

    #[must_use]
    pub fn layout(&self) -> Option<&Layout<'static>> {
        self.layout.as_ref()
    }

    /// The commands of the shown `DownFlag` buttons: they look pressed
    /// while their command is on (the equalizer, a video effect).
    #[must_use]
    pub fn toggle_commands(&self) -> Vec<&'static str> {
        let Some(placed) = &self.layout else {
            return Vec::new();
        };
        placed
            .nodes
            .iter()
            .map(|node| node.element)
            .filter(|element| element.attrs.flag("DownFlag"))
            .filter_map(|element| element.attrs.get("CmdID"))
            .filter(|command| !command.is_empty())
            .collect()
    }

    /// The control, title bar or slot under a point.
    #[must_use]
    pub fn hit(&self, x: i32, y: i32) -> Option<Hit<'static>> {
        self.layout
            .as_ref()
            .and_then(|placed| hit_test(placed, x, y))
    }

    /// The rectangle of a layout node.
    #[must_use]
    pub fn node_rect(&self, node: usize) -> Option<Rect> {
        self.layout
            .as_ref()
            .and_then(|placed| placed.nodes.get(node))
            .map(|node| node.rect)
    }

    /// Where the video goes: the video slot in the window, everything in
    /// fullscreen.
    #[must_use]
    pub fn video_rect(&self) -> Rect {
        match (self.mode, &self.layout) {
            (Mode::Window, Some(placed)) => placed
                .slot("CMD_VIDEO_WINDOW")
                .unwrap_or_else(|| Rect::new(0, 0, self.width, self.height)),
            // The bars' video slot while they show and the video makes
            // room for them (M14).
            (Mode::Fullscreen, Some(placed)) if self.video_in_slot => placed
                .slot("CMD_VIDEO_WINDOW")
                .unwrap_or_else(|| Rect::new(0, 0, self.width, self.height)),
            _ => Rect::new(0, 0, self.width, self.height),
        }
    }

    /// The skin's bars over the video in fullscreen: the top-level parts of
    /// `MainSkinAreaOscInside` (title and control bar), going into a part
    /// that holds the video slot. None in the window, where the video has a
    /// slot of its own. D4's Direct3D 11 window leaves these to the skin.
    #[must_use]
    pub fn bars(&self) -> Vec<Rect> {
        let (Mode::Fullscreen, Some(placed)) = (self.mode, &self.layout) else {
            return Vec::new();
        };
        let nodes = &placed.nodes;
        // The video slot and the parts around it.
        let mut holders = Vec::new();
        let mut next = nodes
            .iter()
            .position(|node| node.slot() == Some("CMD_VIDEO_WINDOW"));
        while let Some(index) = next {
            holders.push(index);
            next = nodes[index].parent;
        }
        let mut bars = Vec::new();
        let mut parents = vec![None];
        while let Some(parent) = parents.pop() {
            for (index, node) in nodes.iter().enumerate() {
                if node.parent != parent || node.clip.is_empty() {
                    continue;
                }
                if holders.first() == Some(&index) {
                    continue;
                }
                if holders.contains(&index) {
                    parents.push(Some(index));
                } else {
                    bars.push(node.clip);
                }
            }
        }
        bars
    }

    /// The size of the chrome around the video slot: window size minus
    /// video size (310x108 for BlackSpace with the playlist).
    #[must_use]
    pub fn chrome_size(&self) -> (i32, i32) {
        let video = self.video_rect();
        (self.width - video.w, self.height - video.h)
    }

    /// The playlist panel's list geometry, when the panel shows.
    #[must_use]
    pub fn list_geometry(&self) -> Option<rvp_skin::playlist::Geometry> {
        let placed = self.layout.as_ref()?;
        let node = placed
            .nodes
            .iter()
            .find(|node| node.slot() == Some("CMD_PLAY_LIST_ALBUM_TAB"))?;
        Some(rvp_skin::playlist::geometry(node, &self.data.playlist))
    }

    /// The bars' rectangles in fullscreen (where the pointer shows them).
    #[must_use]
    pub fn bar_rects(&self) -> Vec<Rect> {
        let Some(placed) = &self.layout else {
            return Vec::new();
        };
        // The video slot is not a bar: over it the bars hide.
        placed
            .nodes
            .iter()
            .filter(|node| {
                node.parent.is_none()
                    && !node.rect.is_empty()
                    && node.slot() != Some("CMD_VIDEO_WINDOW")
            })
            .map(|node| node.rect)
            .collect()
    }
}

/// The largest window size within `window` whose video area (the window
/// minus `chrome`) has the video's aspect ratio (R1 finding 1: PotPlayer
/// keeps the window at the video's shape while a video plays).
#[must_use]
pub fn fit_aspect(window: (i32, i32), chrome: (i32, i32), aspect: f64) -> (i32, i32) {
    let (video_w, video_h) = (window.0 - chrome.0, window.1 - chrome.1);
    if !(aspect.is_finite() && aspect > 0.0) || video_w <= 0 || video_h <= 0 {
        return window;
    }
    // Within a pixel of the shape already: refitting a fitted window must
    // not shrink it pixel by pixel.
    let (w, h) = (f64::from(video_w), f64::from(video_h));
    if (w - h * aspect).abs() <= 1.0 || (h - w / aspect).abs() <= 1.0 {
        return window;
    }
    // Window sizes are far inside i32.
    #[allow(clippy::cast_possible_truncation)]
    if f64::from(video_w) / f64::from(video_h) > aspect {
        let width = (f64::from(video_h) * aspect).round() as i32;
        (width + chrome.0, window.1)
    } else {
        let height = (f64::from(video_w) / aspect).round() as i32;
        (window.0, height + chrome.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_keep_drawing_order_and_drop_covered_ones() {
        let regions = layered(&[
            Rect::new(0, 0, 10, 10),
            Rect::new(5, 5, 10, 10),
            Rect::new(6, 6, 2, 2),
            Rect::new(5, 5, 10, 10),
            Rect::new(40, 0, 5, 5),
        ]);
        // (0,0) stays below the later (5,5); (6,6) and the first (5,5) lie
        // inside the second (5,5), which is drawn after them.
        assert_eq!(
            regions,
            vec![
                Rect::new(0, 0, 10, 10),
                Rect::new(5, 5, 10, 10),
                Rect::new(40, 0, 5, 5)
            ]
        );
        // A small region drawn after a large one stays above it.
        assert_eq!(
            layered(&[Rect::new(0, 0, 20, 20), Rect::new(2, 2, 3, 3)]),
            vec![Rect::new(0, 0, 20, 20), Rect::new(2, 2, 3, 3)]
        );
    }

    #[test]
    fn fitting_keeps_the_video_shape_inside_the_window() {
        // R1: BlackSpace with the playlist, 16:9, 1280 wide: 1280x654.
        assert_eq!(fit_aspect((1280, 720), (310, 108), 16.0 / 9.0), (1280, 654));
        // Too wide for 4:3: the width shrinks.
        assert_eq!(fit_aspect((1000, 400), (100, 100), 4.0 / 3.0), (500, 400));
        assert_eq!(fit_aspect((1000, 400), (100, 100), 0.0), (1000, 400));
        // Refitting a fitted window changes nothing, portrait videos too.
        for (window, aspect) in [
            ((1280, 720), 16.0 / 9.0),
            ((900, 1000), 9.0 / 16.0),
            ((777, 555), 0.4),
            ((640, 999), 2.39),
        ] {
            let once = fit_aspect(window, (310, 108), aspect);
            assert!(once.0 <= window.0 && once.1 <= window.1);
            assert_eq!(
                fit_aspect(once, (310, 108), aspect),
                once,
                "{window:?} {aspect}"
            );
        }
    }

    #[test]
    fn ops_split_between_static_layer_and_regions() {
        let fill = |x, y, w, h| Op::Fill {
            rect: Rect::new(x, y, w, h),
            color: [1, 2, 3, 255],
        };
        let frame = Frame {
            list: DisplayList {
                width: 100,
                height: 100,
                ops: vec![fill(0, 0, 100, 100), fill(10, 10, 5, 5), fill(50, 50, 5, 5)],
            },
            regions: vec![Rect::new(8, 8, 10, 10)],
        };
        let region: Vec<&Op> = frame.ops_for(Some(0)).collect();
        assert_eq!(region.len(), 2, "background and the control");
        let static_layer: Vec<&Op> = frame.ops_for(None).collect();
        assert_eq!(static_layer.len(), 2, "background and the op outside");
    }
}
