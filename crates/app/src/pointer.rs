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
//! * A control whose command is `CMD_POPUP_USER_MENU` opens the skin menu
//!   its `UserMenuName` (right click: `AltUserMenuName`) names; one with a
//!   built-in submenu command (`CMD_POPUPMENU_*`) opens that part of the
//!   menu ([`Effect::Popup`]). `CMD_SELECT_PAGE` shows the controls of its
//!   `PageIndex` (the playlist panel's pages).
//! * Sliders act on press and while dragging: the seek bar seeks, the
//!   volume sliders set the volume, the others (the control panel's
//!   equalizer bands and colours) set their value ([`Effect::Slider`]).
//!   Vertical sliders (`Vert='1'`) read the pointer from the bottom up.
//!   The seek bar follows the mouse at once while it is held and seeks
//!   for good where it is let go ([`Effect::SeekRelease`]; owner,
//!   2026-10-15: the bar shows where it is wanted, not where the video
//!   already is).
//! * The frame edges resize the window; dragging the title bar, an empty
//!   skin area or the video moves it ("prevent window from being dragged
//!   outside of the title bar" is off). The move starts once the pointer
//!   travels a few pixels, so clicks and double clicks stay clicks.
//! * In the playlist panel a click selects (Ctrl adds, Shift extends), a
//!   double click plays, dragging selected rows moves them (a line marks
//!   where they land), the wheel scrolls, tabs switch albums and the mark on
//!   the shown tab deletes it; the panel then has the keyboard.
//! * The control panel (F7) is a window of its own with a fixed size: no
//!   resize border and no mouse gestures; dragging its background moves it.
//! * In the video area the bound gestures apply; with
//!   `left_click_video_only` the left click acts nowhere else. A single
//!   click acts at once (section 4: reaction under 50 ms). When it turns into
//!   a double click, a toggling single-click action is undone first, so a
//!   double click leaves play/pause as it was.

use rvp_skin::hit::Hit;

use rvp_skin::playlist::ListHit;

use crate::action::{Action, ListAction};
use crate::chrome::Chrome;
use crate::input::{Bindings, MouseGesture};
use crate::menu::popup_source;
use crate::playlist::Select;

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
    /// The seek bar is held at this fraction of the duration.
    SeekFraction(f64),
    /// The seek bar was let go at this fraction of the duration.
    SeekRelease(f64),
    /// Set the volume to this fraction of 100 %.
    VolumeFraction(f64),
    /// Another slider moved to this fraction of its range (from the left
    /// or the bottom).
    Slider {
        command: &'static str,
        fraction: f64,
    },
    /// Let the window system move the window (`startSystemMove`).
    StartMove,
    /// Let the window system resize the window at these edges.
    StartResize(i32),
    /// Something in the playlist panel.
    List(ListEffect),
    /// Which part of the window gets the keyboard.
    Focus(Focus),
    /// Open a menu at the pointer: a skin menu (`CMD_POPUP_USER_MENU` and
    /// its name) or a built-in submenu (`CMD_POPUPMENU_*`).
    Popup {
        command: &'static str,
        menu: Option<&'static str>,
    },
}

/// What the playlist panel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListEffect {
    Select {
        index: usize,
        how: Select,
    },
    Play(usize),
    ShowAlbum(usize),
    DeleteAlbum(usize),
    /// Scroll by rows (negative: up).
    ScrollBy(i32),
    /// Make this row the first one shown.
    ScrollTo(usize),
    /// Where dragged rows would land, or none.
    DropMarker(Option<usize>),
    /// Move the selected rows before this entry.
    MoveBefore(usize),
    Action(ListAction),
    /// The playlist's context menu.
    Menu,
}

/// Which part of the window gets the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Window,
    Playlist,
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
    /// The control panel's window: a fixed size and no mouse gestures.
    pub panel: bool,
    /// Modifiers held with the button (playlist selection).
    pub modifiers: Modifiers,
}

/// Keyboard modifiers held with a mouse button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    Idle,
    Button {
        element: usize,
        command: &'static str,
        menu: Option<&'static str>,
        page: Option<i32>,
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
    /// A press on a playlist row: a drag moves the selection; without one,
    /// the release selects only this row (`select_on_release`).
    Rows {
        y: i32,
        index: usize,
        moving: bool,
        select_on_release: bool,
    },
    /// The scroll bar thumb, grabbed this far below its top.
    Thumb {
        grab: i32,
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
    List(ListHit),
    /// Skin without a control (frame, bar backgrounds, playlist).
    Background,
}

/// Skin commands the pointer handles itself: menus and pages.
fn is_control_command(command: &str) -> bool {
    command == "CMD_POPUP_USER_MENU"
        || command == "CMD_SELECT_PAGE"
        || command.starts_with("CMD_POPUPMENU_")
        || popup_source(command).is_some()
}

/// A menu or page command of a skin control.
fn control_effect(
    chrome: &mut Chrome,
    command: &'static str,
    menu: Option<&'static str>,
    page: Option<i32>,
) -> Vec<Effect> {
    if command == "CMD_SELECT_PAGE" {
        if let Some(page) = page {
            chrome.state.current_page = page;
        }
        return Vec::new();
    }
    if is_control_command(command) {
        return vec![Effect::Popup { command, menu }];
    }
    Vec::new()
}

fn target(chrome: &Chrome, ctx: Context, x: i32, y: i32) -> Target {
    if !ctx.chrome_shown {
        return Target::Video;
    }
    if ctx.movable && !ctx.panel {
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
        Some(Hit::Slot {
            name: "CMD_PLAY_LIST_ALBUM_TAB",
            ..
        }) => chrome
            .list_geometry()
            .and_then(|geometry| {
                rvp_skin::playlist::hit(&geometry, chrome.data.playlist.entries.len(), x, y)
            })
            .map_or(Target::Background, Target::List),
        // Fullscreen bars cover only part of the screen: the rest is video.
        None if chrome.mode() == crate::chrome::Mode::Fullscreen => Target::Video,
        _ => Target::Background,
    }
}

fn slider_effect(
    chrome: &Chrome,
    node: usize,
    command: &'static str,
    x: i32,
    y: i32,
    released: bool,
) -> Option<Effect> {
    let node = chrome.layout()?.nodes.get(node)?;
    let rect = node.rect;
    let fraction = if node.element.attrs.flag("Vert") {
        if rect.h > 1 {
            (f64::from(rect.bottom() - 1 - y) / f64::from(rect.h - 1)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    } else if rect.w > 1 {
        (f64::from(x - rect.x) / f64::from(rect.w - 1)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some(match command {
        "CMD_TIMEBAR_CTRL" if released => Effect::SeekRelease(fraction),
        "CMD_TIMEBAR_CTRL" => Effect::SeekFraction(fraction),
        "CMD_VOLUME_CTRL" | "CMD_MASTER_VOLUME" => Effect::VolumeFraction(fraction),
        _ => Effect::Slider { command, fraction },
    })
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
                menu,
                page,
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
                    slider_effect(chrome, node, command, x, y, false)
                        .into_iter()
                        .collect()
                } else {
                    self.drag = Drag::Button {
                        element,
                        command,
                        menu,
                        page,
                    };
                    Vec::new()
                }
            }
            Target::Control(_) => Vec::new(),
            Target::Video => {
                self.drag = Drag::Press { x, y, click: true };
                vec![Effect::Focus(Focus::Window)]
            }
            Target::List(hit) => {
                let mut effects = vec![Effect::Focus(Focus::Playlist)];
                effects.extend(self.press_list(chrome, ctx, hit, y));
                effects
            }
        }
    }

    fn press_list(&mut self, chrome: &Chrome, ctx: Context, hit: ListHit, y: i32) -> Vec<Effect> {
        let list = &chrome.data.playlist;
        let visible = chrome
            .list_geometry()
            .map_or(1, |geometry| geometry.visible.max(1));
        let page = i32::try_from(visible).unwrap_or(1);
        let effect = match hit {
            ListHit::Row(index) => {
                let plain = ctx.modifiers == Modifiers::default();
                let selected = list.selected.contains(&index);
                self.drag = Drag::Rows {
                    y,
                    index,
                    moving: false,
                    select_on_release: plain && selected,
                };
                if plain && selected {
                    // Keep a multiple selection for dragging; the release
                    // narrows it to this row.
                    return Vec::new();
                }
                let how = if ctx.modifiers.shift {
                    Select::Range
                } else if ctx.modifiers.ctrl {
                    Select::Toggle
                } else {
                    Select::Only
                };
                ListEffect::Select { index, how }
            }
            ListHit::Tab(album) => ListEffect::ShowAlbum(album),
            ListHit::TabDelete(album) => ListEffect::DeleteAlbum(album),
            ListHit::TabOverflow => {
                ListEffect::ShowAlbum((list.current_album + 1) % list.albums.len().max(1))
            }
            ListHit::ScrollUp => ListEffect::ScrollBy(-1),
            ListHit::ScrollDown => ListEffect::ScrollBy(1),
            ListHit::TrackAbove => ListEffect::ScrollBy(-page),
            ListHit::TrackBelow => ListEffect::ScrollBy(page),
            ListHit::ScrollThumb => {
                let top = chrome
                    .list_geometry()
                    .and_then(|geometry| geometry.scrollbar)
                    .map_or(y, |bar| bar.thumb.y);
                self.drag = Drag::Thumb { grab: y - top };
                return Vec::new();
            }
            ListHit::TabBar | ListHit::Empty => return Vec::new(),
        };
        vec![Effect::List(effect)]
    }

    fn move_list(&mut self, chrome: &Chrome, y: i32) -> Vec<Effect> {
        let Some(geometry) = chrome.list_geometry() else {
            return Vec::new();
        };
        let total = chrome.data.playlist.entries.len();
        match &mut self.drag {
            Drag::Rows {
                y: start, moving, ..
            } => {
                if !*moving && (y - *start).abs() < DRAG_DISTANCE {
                    return Vec::new();
                }
                *moving = true;
                vec![Effect::List(ListEffect::DropMarker(Some(
                    geometry.insertion_index(y, total),
                )))]
            }
            Drag::Thumb { grab } => {
                let Some(bar) = geometry.scrollbar else {
                    return Vec::new();
                };
                let travel = (bar.track.h - bar.thumb.h).max(1);
                let rows = total.saturating_sub(geometry.visible);
                let offset = (y - *grab - bar.track.y).clamp(0, travel);
                let rows_i = i64::try_from(rows).unwrap_or(0);
                let top = rows_i * i64::from(offset) / i64::from(travel);
                vec![Effect::List(ListEffect::ScrollTo(
                    usize::try_from(top).unwrap_or(0),
                ))]
            }
            _ => Vec::new(),
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
            Target::List(ListHit::Row(index)) if button == Button::Left => {
                self.swallow_release = true;
                self.drag = Drag::Idle;
                vec![Effect::List(ListEffect::Play(index))]
            }
            Target::List(ListHit::Empty) if button == Button::Left => {
                self.swallow_release = true;
                vec![Effect::List(ListEffect::Action(ListAction::AddFiles))]
            }
            Target::List(ListHit::TabBar) if button == Button::Left => {
                self.swallow_release = true;
                vec![Effect::List(ListEffect::Action(ListAction::AddAlbum))]
            }
            // Controls take the second click as another click.
            _ => self.press(chrome, ctx, x, y, button),
        }
    }

    /// The pointer moved (with or without a button down).
    pub fn moved(&mut self, chrome: &mut Chrome, ctx: Context, x: i32, y: i32) -> Vec<Effect> {
        match self.drag {
            Drag::Slider { node, command } => {
                return slider_effect(chrome, node, command, x, y, false)
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
            Drag::Rows { .. } | Drag::Thumb { .. } => return self.move_list(chrome, y),
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
                Drag::Button {
                    element,
                    command,
                    menu,
                    page,
                } => {
                    let over = matches!(
                        chrome.hit(x, y),
                        Some(Hit::Control { element: e, .. }) if e == element
                    );
                    if !over {
                        return Vec::new();
                    }
                    if let Some(action) = Action::from_skin_command(command) {
                        vec![Effect::Action(action)]
                    } else if let Some(action) = ListAction::from_skin_command(command) {
                        vec![
                            Effect::Focus(Focus::Playlist),
                            Effect::List(ListEffect::Action(action)),
                        ]
                    } else {
                        control_effect(chrome, command, menu, page)
                    }
                }
                Drag::Rows {
                    index,
                    moving,
                    select_on_release,
                    ..
                } => {
                    if moving {
                        let total = chrome.data.playlist.entries.len();
                        let to = chrome
                            .list_geometry()
                            .map_or(index, |geometry| geometry.insertion_index(y, total));
                        vec![
                            Effect::List(ListEffect::DropMarker(None)),
                            Effect::List(ListEffect::MoveBefore(to)),
                        ]
                    } else if select_on_release {
                        vec![Effect::List(ListEffect::Select {
                            index,
                            how: Select::Only,
                        })]
                    } else {
                        Vec::new()
                    }
                }
                Drag::Slider { node, command } => slider_effect(chrome, node, command, x, y, true)
                    .into_iter()
                    .collect(),
                Drag::Press { click: true, .. } => {
                    let action = bindings.mouse(MouseGesture::LeftClick);
                    self.undo = action.filter(|action| action.is_toggle());
                    action.map(Effect::Action).into_iter().collect()
                }
                Drag::Press { click: false, .. } | Drag::Thumb { .. } | Drag::Idle => Vec::new(),
            },
            Button::Right => match target(chrome, ctx, x, y) {
                Target::Control(Hit::Control {
                    alt_command: Some(command),
                    alt_menu,
                    page,
                    ..
                }) if Action::from_skin_command(command).is_some()
                    || is_control_command(command) =>
                {
                    match Action::from_skin_command(command) {
                        Some(action) => vec![Effect::Action(action)],
                        None => control_effect(chrome, command, alt_menu, page),
                    }
                }
                Target::List(hit) => {
                    let mut effects = vec![Effect::Focus(Focus::Playlist)];
                    if let ListHit::Row(index) = hit
                        && !chrome.data.playlist.selected.contains(&index)
                    {
                        effects.push(Effect::List(ListEffect::Select {
                            index,
                            how: Select::Only,
                        }));
                    }
                    effects.push(Effect::List(ListEffect::Menu));
                    effects
                }
                _ => Self::gesture(chrome, bindings, ctx, x, y, button),
            },
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
        if ctx.panel || (button == Button::Left && bindings.left_click_video_only && !in_video) {
            return Vec::new();
        }
        bindings
            .mouse(gesture(button, false))
            .map(Effect::Action)
            .into_iter()
            .collect()
    }

    /// The wheel turned at `(x, y)`; `delta` is positive away from the
    /// user. Over the playlist it scrolls the list.
    #[must_use]
    pub fn wheel(
        &self,
        chrome: &Chrome,
        bindings: &Bindings,
        ctx: Context,
        (x, y): (i32, i32),
        delta: i32,
    ) -> Vec<Effect> {
        if delta != 0 && matches!(target(chrome, ctx, x, y), Target::List(_)) {
            return vec![Effect::List(ListEffect::ScrollBy(-3 * delta.signum()))];
        }
        if ctx.panel {
            return Vec::new();
        }
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
    fn gestures_map_buttons_and_clicks() {
        assert_eq!(gesture(Button::Left, true), MouseGesture::LeftDoubleClick);
        assert_eq!(gesture(Button::Forward, false), MouseGesture::X2Click);
        assert_eq!(gesture(Button::Middle, false), MouseGesture::MiddleClick);
    }
}
