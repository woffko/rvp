//! Mouse handling for the skinned window: what a press, release, double
//! click, drag or wheel turn acts on, and what it does.
//!
//! PotPlayer behaviour reproduced (R1, `docs/r1/mouse-actions.tsv`):
//!
//! * Buttons act on release over the same button; releasing elsewhere
//!   cancels the click.
//! * A right click on a control runs its `AltCmdID` (right click on Previous
//!   is "30 Sec Backward"); elsewhere it is the right-click gesture (the
//!   context menu).
//! * Sliders seek or set the volume on press and while dragging.
//! * The frame edges resize the window; dragging the title bar, an empty
//!   skin area or the video moves it ("prevent window from being dragged
//!   outside of the title bar" is off). The move starts once the pointer
//!   travels a few pixels, so clicks and double clicks stay clicks.
//! * In the video area the bound gestures apply; with
//!   `left_click_video_only` the left click acts nowhere else. A single
//!   click acts at once (section 4: reaction under 50 ms). When it turns into
//!   a double click, a toggling single-click action is undone first, so a
//!   double click leaves play/pause as it was.

use rvp_skin::hit::Hit;

use crate::action::Action;
use crate::chrome::Chrome;
use crate::input::{Bindings, MouseGesture};

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
    /// X1, "back".
    Back,
    /// X2, "forward".
    Forward,
}

/// Window edges as Qt numbers them (`Qt::Edge`), combined for corners.
pub const EDGE_TOP: i32 = 1;
pub const EDGE_LEFT: i32 = 2;
pub const EDGE_RIGHT: i32 = 4;
pub const EDGE_BOTTOM: i32 = 8;
/// Width of the resize border (BlackSpace's frame is 5 px).
const BORDER: i32 = 5;
/// Length of the corner zones along the border.
const CORNER: i32 = 16;
/// Pointer travel that turns a press in the video area into a window move.
const DRAG_DISTANCE: i32 = 4;

/// The pointer shape over the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cursor {
    #[default]
    Arrow,
    SizeHorizontal,
    SizeVertical,
    /// `\`: top-left and bottom-right corners.
    SizeForwardDiagonal,
    /// `/`: top-right and bottom-left corners.
    SizeBackwardDiagonal,
}

/// What the window has to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    Action(Action),
    /// Seek to this fraction of the duration.
    SeekFraction(f64),
    /// Set the volume to this fraction of 100 %.
    VolumeFraction(f64),
    /// Let the window system move the window (`startSystemMove`).
    StartMove,
    /// Let the window system resize the window at these edges.
    StartResize(i32),
}

/// The window's state as far as the pointer is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub width: i32,
    pub height: i32,
    /// False when maximized or fullscreen: no resize border, no moves.
    pub movable: bool,
    /// False in fullscreen while the bars are hidden: everything is video.
    pub chrome_shown: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    Idle,
    Button {
        element: usize,
        command: &'static str,
    },
    Slider {
        node: usize,
        command: &'static str,
    },
    /// A press in the video area (`click`), on the title bar or on an empty
    /// skin area; it moves the window once the pointer travels.
    Press {
        x: i32,
        y: i32,
        click: bool,
    },
}

/// Mouse state of one window.
#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
    drag: Drag,
    /// The toggling action of the last single click in the video area.
    undo: Option<Action>,
    /// The release that ends a double click: no single click for it.
    swallow_release: bool,
    /// Tooltip of the control under the pointer.
    pub tooltip: Option<&'static str>,
    pub cursor: Cursor,
}

impl Default for Pointer {
    fn default() -> Self {
        Self {
            drag: Drag::Idle,
            undo: None,
            swallow_release: false,
            tooltip: None,
            cursor: Cursor::Arrow,
        }
    }
}

/// Resize edges under a point, 0 when none.
#[must_use]
pub fn edges_at(x: i32, y: i32, width: i32, height: i32) -> i32 {
    let near_side = x < BORDER || x >= width - BORDER;
    let near_end = y < BORDER || y >= height - BORDER;
    if !near_side && !near_end {
        return 0;
    }
    let horizontal = if x < BORDER || (near_end && x < CORNER) {
        EDGE_LEFT
    } else if x >= width - BORDER || (near_end && x >= width - CORNER) {
        EDGE_RIGHT
    } else {
        0
    };
    let vertical = if y < BORDER || (near_side && y < CORNER) {
        EDGE_TOP
    } else if y >= height - BORDER || (near_side && y >= height - CORNER) {
        EDGE_BOTTOM
    } else {
        0
    };
    horizontal | vertical
}

fn cursor_for(edges: i32) -> Cursor {
    match edges {
        0 => Cursor::Arrow,
        e if e == EDGE_LEFT | EDGE_TOP || e == EDGE_RIGHT | EDGE_BOTTOM => {
            Cursor::SizeForwardDiagonal
        }
        e if e == EDGE_RIGHT | EDGE_TOP || e == EDGE_LEFT | EDGE_BOTTOM => {
            Cursor::SizeBackwardDiagonal
        }
        EDGE_LEFT | EDGE_RIGHT => Cursor::SizeHorizontal,
        _ => Cursor::SizeVertical,
    }
}

/// What the pointer is over.
enum Target {
    Edge(i32),
    Control(Hit<'static>),
    TitleBar,
    Video,
    /// Skin without a control (frame, bar backgrounds, playlist).
    Background,
}

fn target(chrome: &Chrome, ctx: Context, x: i32, y: i32) -> Target {
    if !ctx.chrome_shown {
        return Target::Video;
    }
    if ctx.movable {
        let edges = edges_at(x, y, ctx.width, ctx.height);
        if edges != 0 {
            return Target::Edge(edges);
        }
    }
    match chrome.hit(x, y) {
        Some(hit @ Hit::Control { .. }) => Target::Control(hit),
        Some(Hit::TitleBar { .. }) => Target::TitleBar,
        Some(Hit::Slot {
            name: "CMD_VIDEO_WINDOW",
            ..
        }) => Target::Video,
        // Fullscreen bars cover only part of the screen: the rest is video.
        None if chrome.mode() == crate::chrome::Mode::Fullscreen => Target::Video,
        _ => Target::Background,
    }
}

fn slider_effect(chrome: &Chrome, node: usize, command: &str, x: i32) -> Option<Effect> {
    let rect = chrome.node_rect(node)?;
    let fraction = if rect.w > 1 {
        (f64::from(x - rect.x) / f64::from(rect.w - 1)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    match command {
        "CMD_TIMEBAR_CTRL" => Some(Effect::SeekFraction(fraction)),
        "CMD_VOLUME_CTRL" => Some(Effect::VolumeFraction(fraction)),
        _ => None,
    }
}

fn gesture(button: Button, double: bool) -> MouseGesture {
    match (button, double) {
        (Button::Left, false) => MouseGesture::LeftClick,
        (Button::Left, true) => MouseGesture::LeftDoubleClick,
        (Button::Right, false) => MouseGesture::RightClick,
        (Button::Right, true) => MouseGesture::RightDoubleClick,
        (Button::Middle, false) => MouseGesture::MiddleClick,
        (Button::Middle, true) => MouseGesture::MiddleDoubleClick,
        (Button::Back, false) => MouseGesture::X1Click,
        (Button::Back, true) => MouseGesture::X1DoubleClick,
        (Button::Forward, false) => MouseGesture::X2Click,
        (Button::Forward, true) => MouseGesture::X2DoubleClick,
    }
}

impl Pointer {
    /// A button went down.
    pub fn press(
        &mut self,
        chrome: &mut Chrome,
        ctx: Context,
        x: i32,
        y: i32,
        button: Button,
    ) -> Vec<Effect> {
        self.swallow_release = false;
        if button != Button::Left {
            return Vec::new();
        }
        match target(chrome, ctx, x, y) {
            Target::Edge(edges) => vec![Effect::StartResize(edges)],
            // The window moves once the pointer travels, so a double click
            // on the title bar is not taken for a move.
            Target::TitleBar | Target::Background => {
                self.drag = Drag::Press { x, y, click: false };
                Vec::new()
            }
            Target::Control(Hit::Control {
                node,
                element,
                command,
                ..
            }) => {
                let Some(command) = command else {
                    return Vec::new();
                };
                chrome.state.pressed = Some(element);
                let is_slider = chrome
                    .layout()
                    .and_then(|placed| placed.nodes.get(node))
                    .is_some_and(|n| n.kind == rvp_skin::layout::Kind::Slider);
                if is_slider {
                    self.drag = Drag::Slider { node, command };
                    slider_effect(chrome, node, command, x)
                        .into_iter()
                        .collect()
                } else {
                    self.drag = Drag::Button { element, command };
                    Vec::new()
                }
            }
            Target::Control(_) => Vec::new(),
            Target::Video => {
                self.drag = Drag::Press { x, y, click: true };
                Vec::new()
            }
        }
    }

    /// The second press of a double click (it replaces the press).
    pub fn double_click(
        &mut self,
        chrome: &mut Chrome,
        bindings: &Bindings,
        ctx: Context,
        x: i32,
        y: i32,
        button: Button,
    ) -> Vec<Effect> {
        match target(chrome, ctx, x, y) {
            Target::TitleBar if button == Button::Left => {
                self.swallow_release = true;
                vec![Effect::Action(Action::MaximizeRestore)]
            }
            Target::Video => {
                self.swallow_release = true;
                self.drag = Drag::Idle;
                let mut effects = Vec::new();
                if let Some(undo) = self.undo.take() {
                    effects.push(Effect::Action(undo));
                }
                if let Some(action) = bindings.mouse(gesture(button, true)) {
                    effects.push(Effect::Action(action));
                }
                effects
            }
            // Controls take the second click as another click.
            _ => self.press(chrome, ctx, x, y, button),
        }
    }

    /// The pointer moved (with or without a button down).
    pub fn moved(&mut self, chrome: &mut Chrome, ctx: Context, x: i32, y: i32) -> Vec<Effect> {
        match self.drag {
            Drag::Slider { node, command } => {
                return slider_effect(chrome, node, command, x)
                    .into_iter()
                    .collect();
            }
            Drag::Button { element, .. } => {
                let over = matches!(
                    chrome.hit(x, y),
                    Some(Hit::Control { element: e, .. }) if e == element
                );
                chrome.state.pressed = over.then_some(element);
                return Vec::new();
            }
            Drag::Press { x: x0, y: y0, .. } => {
                if (x - x0).abs() + (y - y0).abs() >= DRAG_DISTANCE {
                    self.drag = Drag::Idle;
                    if ctx.movable {
                        return vec![Effect::StartMove];
                    }
                }
                return Vec::new();
            }
            Drag::Idle => {}
        }
        let (hover, tooltip, cursor) = match target(chrome, ctx, x, y) {
            Target::Edge(edges) => (None, None, cursor_for(edges)),
            Target::Control(Hit::Control {
                element, tooltip, ..
            }) => (Some(element), tooltip, Cursor::Arrow),
            _ => (None, None, Cursor::Arrow),
        };
        chrome.state.hover = hover;
        self.tooltip = tooltip;
        self.cursor = cursor;
        Vec::new()
    }

    /// A button went up.
    pub fn release(
        &mut self,
        chrome: &mut Chrome,
        bindings: &Bindings,
        ctx: Context,
        x: i32,
        y: i32,
        button: Button,
    ) -> Vec<Effect> {
        let drag = std::mem::replace(&mut self.drag, Drag::Idle);
        chrome.state.pressed = None;
        if std::mem::take(&mut self.swallow_release) {
            return Vec::new();
        }
        match button {
            Button::Left => match drag {
                Drag::Button { element, command } => {
                    let over = matches!(
                        chrome.hit(x, y),
                        Some(Hit::Control { element: e, .. }) if e == element
                    );
                    match Action::from_skin_command(command) {
                        Some(action) if over => vec![Effect::Action(action)],
                        _ => Vec::new(),
                    }
                }
                Drag::Slider { node, command } => slider_effect(chrome, node, command, x)
                    .into_iter()
                    .collect(),
                Drag::Press { click: true, .. } => {
                    let action = bindings.mouse(MouseGesture::LeftClick);
                    self.undo = action.filter(|action| action.is_toggle());
                    action.map(Effect::Action).into_iter().collect()
                }
                Drag::Press { click: false, .. } | Drag::Idle => Vec::new(),
            },
            Button::Right => {
                if let Target::Control(Hit::Control { alt_command, .. }) = target(chrome, ctx, x, y)
                    && let Some(action) = alt_command.and_then(Action::from_skin_command)
                {
                    return vec![Effect::Action(action)];
                }
                Self::gesture(chrome, bindings, ctx, x, y, button)
            }
            _ => Self::gesture(chrome, bindings, ctx, x, y, button),
        }
    }

    fn gesture(
        chrome: &Chrome,
        bindings: &Bindings,
        ctx: Context,
        x: i32,
        y: i32,
        button: Button,
    ) -> Vec<Effect> {
        let in_video = matches!(target(chrome, ctx, x, y), Target::Video);
        if button == Button::Left && bindings.left_click_video_only && !in_video {
            return Vec::new();
        }
        bindings
            .mouse(gesture(button, false))
            .map(Effect::Action)
            .into_iter()
            .collect()
    }

    /// The wheel turned; `delta` is positive away from the user.
    #[must_use]
    pub fn wheel(&self, bindings: &Bindings, delta: i32) -> Vec<Effect> {
        let gesture = match delta.signum() {
            1 => MouseGesture::WheelUp,
            -1 => MouseGesture::WheelDown,
            _ => return Vec::new(),
        };
        bindings
            .mouse(gesture)
            .map(Effect::Action)
            .into_iter()
            .collect()
    }

    /// The pointer left the window.
    pub fn leave(&mut self, chrome: &mut Chrome) {
        if matches!(self.drag, Drag::Idle) {
            chrome.state.hover = None;
            self.tooltip = None;
            self.cursor = Cursor::Arrow;
        }
    }

    /// True while a slider or button is held.
    #[must_use]
    pub fn dragging(&self) -> bool {
        !matches!(self.drag, Drag::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_and_corners() {
        let (w, h) = (1280, 720);
        assert_eq!(edges_at(640, 360, w, h), 0);
        assert_eq!(edges_at(2, 360, w, h), EDGE_LEFT);
        assert_eq!(edges_at(1278, 360, w, h), EDGE_RIGHT);
        assert_eq!(edges_at(640, 1, w, h), EDGE_TOP);
        assert_eq!(edges_at(640, 718, w, h), EDGE_BOTTOM);
        assert_eq!(edges_at(10, 2, w, h), EDGE_LEFT | EDGE_TOP);
        assert_eq!(edges_at(1277, 710, w, h), EDGE_RIGHT | EDGE_BOTTOM);
        assert_eq!(
            cursor_for(EDGE_LEFT | EDGE_TOP),
            Cursor::SizeForwardDiagonal
        );
        assert_eq!(
            cursor_for(EDGE_RIGHT | EDGE_TOP),
            Cursor::SizeBackwardDiagonal
        );
        assert_eq!(cursor_for(EDGE_BOTTOM), Cursor::SizeVertical);
    }

    #[test]
    fn wheel_follows_bindings() {
        let bindings = Bindings::potplayer_defaults();
        let pointer = Pointer::default();
        assert_eq!(
            pointer.wheel(&bindings, 120),
            vec![Effect::Action(Action::VolumeUp)]
        );
        assert_eq!(
            pointer.wheel(&bindings, -120),
            vec![Effect::Action(Action::VolumeDown)]
        );
        assert!(pointer.wheel(&bindings, 0).is_empty());
    }
}
