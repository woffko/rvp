//! Finds the control under a point.

use crate::layout::{Kind, Layout};

/// What a mouse event at a point would act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hit<'a> {
    /// A control with a command: buttons, sliders, clickable images and
    /// labels.
    Control {
        /// Index of the node in the layout.
        node: usize,
        /// Element ID, as used by `UiState::hover` and `UiState::pressed`.
        element: usize,
        /// Left-click command.
        command: Option<&'a str>,
        /// Right-click command (`AltCmdID`).
        alt_command: Option<&'a str>,
        /// Tooltip as written in the skin (PotPlayer shows `< >` as is).
        tooltip: Option<&'a str>,
        /// The skin menu a `CMD_POPUP_USER_MENU` command opens
        /// (`UserMenuName`), see [`crate::menu`].
        menu: Option<&'a str>,
        /// The same for the right click (`AltUserMenuName`).
        alt_menu: Option<&'a str>,
        /// The skin file a `CMD_LOAD_SKIN_XML` command shows (`String`).
        file: Option<&'a str>,
        /// The page a `CMD_SELECT_PAGE` control shows (`PageIndex`).
        page: Option<i32>,
    },
    /// A `IsTitleBar` static: dragging moves the window.
    TitleBar { node: usize },
    /// A `Contain` slot such as the video window or the playlist.
    Slot { node: usize, name: &'a str },
}

/// Returns the topmost interactive element at `(x, y)`.
#[must_use]
pub fn hit_test<'a>(layout: &Layout<'a>, x: i32, y: i32) -> Option<Hit<'a>> {
    for (index, node) in layout.nodes.iter().enumerate().rev() {
        if !node.clip.contains(x, y) {
            continue;
        }
        let attrs = &node.element.attrs;
        let command = attrs.get("CmdID").filter(|cmd| !cmd.is_empty());
        let interactive = match node.kind {
            Kind::Button | Kind::Slider => true,
            Kind::Image | Kind::Label | Kind::PicLabel => {
                command.is_some() && attrs.flag("Enabled")
            }
            Kind::Static | Kind::Spacer | Kind::Visual => false,
        };
        if interactive {
            return Some(Hit::Control {
                node: index,
                element: node.element.id,
                command,
                alt_command: attrs.get("AltCmdID").filter(|cmd| !cmd.is_empty()),
                tooltip: attrs.get("Tooltip").filter(|tip| !tip.is_empty()),
                menu: attrs.get("UserMenuName").filter(|name| !name.is_empty()),
                alt_menu: attrs.get("AltUserMenuName").filter(|name| !name.is_empty()),
                file: attrs.get("String").filter(|name| !name.is_empty()),
                page: attrs.int("PageIndex"),
            });
        }
        if let Some(name) = attrs.get("Contain").filter(|slot| !slot.is_empty()) {
            return Some(Hit::Slot { node: index, name });
        }
        if attrs.flag("IsTitleBar") {
            return Some(Hit::TitleBar { node: index });
        }
    }
    None
}
