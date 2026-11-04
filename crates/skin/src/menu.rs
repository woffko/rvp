//! Skin-defined menus and the commands a skin uses.
//!
//! PotPlayer skins define their own small menus in a `<Menu>` section
//! (BlackSpace has 19):
//!
//! ```text
//! <Menu>
//!   <UserMenu Name='OpenMenu2'>
//!     <MenuItem CmdID='ID_BASE_OPEN' />
//!     <MenuItem CmdID='' />                              (a separator)
//!     <MenuItem CmdID='CMD_POPUPMENU_DVD' NoSubMenu='1' />
//!   </UserMenu>
//! </Menu>
//! ```
//!
//! A `CMD_POPUPMENU_*` item stands for one of PotPlayer's built-in submenus:
//! as a submenu entry, or with `NoSubMenu='1'` its items in place. Buttons
//! open a skin menu with `CmdID='CMD_POPUP_USER_MENU' UserMenuName='...'`
//! (left click) or `AltCmdID='CMD_POPUP_USER_MENU' AltUserMenuName='...'`
//! (right click); [`crate::hit::Hit::Control`] carries both names.
//!
//! [`commands`] lists every command the skin's live elements use (`CmdID`,
//! `AltCmdID` and menu items; lines disabled by deleting `<` are not
//! elements), for the dispatcher's coverage report.
//!
//! PotPlayer's `Menus` folder holds whole context menus in the same items
//! ([`menu_file`], M14's Context Menu Design): `<Menu Title>` with
//! `<MenuItem>`s and `<SubMenu Name>` sections. A title's `{$CP0=...$}`
//! part is its English text, and a submenu named `$MENU_VIEW` reads
//! "View".
//!
//! A skin styles context menus with `<CustomPopupMenuDef>` (PotPlayer's
//! "Style #1 (defined in skin)") and `<CustomPopupMenuAlt>` ("Style #2"):
//! points of its main picture give the colours (`TextColorX`/`Y`,
//! `SelTextColor`, `SelColor`, `DimTextColor`, `LineColor`), and the
//! frame's (`FrameX`, `FrameY`, `FrameW`, `FrameH`) middle and left edge
//! the background and border ([`crate::Skin::menu_style`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::markup::Element;

/// One `<UserMenu>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMenu {
    pub name: String,
    pub items: Vec<UserMenuItem>,
}

/// One `<MenuItem>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMenuItem {
    /// The command; empty for a separator.
    pub command: String,
    /// `NoSubMenu='1'`: a built-in submenu's items in place of an entry.
    pub inline: bool,
}

/// The skin's menus in document order.
#[must_use]
pub fn user_menus(root: &Element) -> Vec<UserMenu> {
    let mut menus = Vec::new();
    root.walk(&mut |element| {
        if !element.is("UserMenu") {
            return;
        }
        let items = element
            .children
            .iter()
            .filter(|child| child.is("MenuItem"))
            .map(|item| UserMenuItem {
                command: item.attrs.get("CmdID").unwrap_or("").trim().to_owned(),
                inline: item.attrs.flag("NoSubMenu"),
            })
            .collect();
        menus.push(UserMenu {
            name: element.attrs.get("Name").unwrap_or("").to_owned(),
            items,
        });
    });
    menus
}

/// A skin's context menu colours (`Skin::menu_style`), RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuStyle {
    pub back: [u8; 4],
    pub border: [u8; 4],
    pub text: [u8; 4],
    pub selected_text: [u8; 4],
    pub selected_back: [u8; 4],
    pub dim_text: [u8; 4],
    pub line: [u8; 4],
}

/// A context menu file (PotPlayer's `Menus` folder, M14's Context Menu
/// Design): `<Menu Title>` holds `<MenuItem CmdID>` entries, as a skin's
/// `<UserMenu>` does, and `<SubMenu Name>` sections with more of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuFile {
    /// The `{$CP0=...$}` part of `Title` (its English text), or the whole
    /// title.
    pub title: String,
    pub items: Vec<MenuFileItem>,
}

/// One entry of a [`MenuFile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuFileItem {
    /// A command or, with an empty command, a separator.
    Item(UserMenuItem),
    /// A submenu: `Name='$MENU_VIEW'` reads "View".
    Sub {
        name: String,
        items: Vec<MenuFileItem>,
    },
}

/// Reads a menu file's text; `None` without a `<Menu>`.
#[must_use]
pub fn menu_file(text: &str) -> Option<MenuFile> {
    let root = crate::markup::parse_document(text);
    let menu = root.find("Menu")?;
    Some(MenuFile {
        title: english(menu.attrs.get("Title").unwrap_or("")),
        items: file_items(menu),
    })
}

fn file_items(element: &Element) -> Vec<MenuFileItem> {
    element
        .children
        .iter()
        .filter_map(|child| {
            if child.is("MenuItem") {
                Some(MenuFileItem::Item(UserMenuItem {
                    command: child.attrs.get("CmdID").unwrap_or("").trim().to_owned(),
                    inline: child.attrs.flag("NoSubMenu"),
                }))
            } else if child.is("SubMenu") {
                Some(MenuFileItem::Sub {
                    name: submenu_name(child.attrs.get("Name").unwrap_or("")),
                    items: file_items(child),
                })
            } else {
                None
            }
        })
        .collect()
}

/// `{$CP949=...$}{$CP0=Simple popup menu$}` reads "Simple popup menu".
fn english(title: &str) -> String {
    title
        .split("{$")
        .find_map(|part| part.strip_prefix("CP0="))
        .map_or(title, |text| text.split("$}").next().unwrap_or(text))
        .trim()
        .to_owned()
}

/// `$MENU_VIEW` reads "View"; other names stay.
fn submenu_name(name: &str) -> String {
    let Some(key) = name.trim().strip_prefix("$MENU_") else {
        return name.trim().to_owned();
    };
    let lower = key.replace('_', " ").to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Where a command appears.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandUse {
    /// Element names (`Button`, `Slider`, `MenuItem`, ...).
    pub kinds: BTreeSet<String>,
    /// The skin areas (`MainSkinArea`, ...) or `Menu`.
    pub areas: BTreeSet<String>,
    /// How many attributes name it.
    pub count: usize,
}

/// Every command the skin's live elements name, with where it appears.
#[must_use]
pub fn commands(root: &Element) -> BTreeMap<String, CommandUse> {
    let mut found = BTreeMap::new();
    collect(root, "", &mut found);
    found
}

fn collect(element: &Element, area: &str, found: &mut BTreeMap<String, CommandUse>) {
    // RVP's own copies (the fullscreen bars of a skin without them) count
    // once, where the skin has them.
    if element.attrs.flag("RvpBuilt") {
        return;
    }
    let area = if element.name.ends_with("Area") || element.is("Menu") {
        element.name.as_str()
    } else {
        area
    };
    for attribute in ["CmdID", "AltCmdID"] {
        if let Some(command) = element.attrs.get(attribute).map(str::trim)
            && !command.is_empty()
        {
            let entry: &mut CommandUse = found.entry(command.to_owned()).or_default();
            entry.kinds.insert(element.name.clone());
            if !area.is_empty() {
                entry.areas.insert(area.to_owned());
            }
            entry.count += 1;
        }
    }
    for child in &element.children {
        collect(child, area, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::parse_document;

    #[test]
    fn menus_items_and_separators() {
        let root = parse_document(
            "<Skin><MainSkinArea><Button CmdID='CMD_POPUP_USER_MENU' UserMenuName='Open' \
             AltCmdID='ID_X' /></MainSkinArea>\
             <Menu><UserMenu Name='Open'><MenuItem CmdID='ID_BASE_OPEN' /><MenuItem CmdID='' />\
             <MenuItem CmdID='CMD_POPUPMENU_DVD' NoSubMenu='1' />\
             MenuItem CmdID='ID_DISABLED' /></UserMenu>\
             <UserMenu Name='Empty'></UserMenu></Menu></Skin>",
        );
        let menus = user_menus(&root);
        assert_eq!(menus.len(), 2);
        assert_eq!(menus[0].name, "Open");
        assert_eq!(
            menus[0].items,
            vec![
                UserMenuItem {
                    command: "ID_BASE_OPEN".into(),
                    inline: false
                },
                UserMenuItem {
                    command: String::new(),
                    inline: false
                },
                UserMenuItem {
                    command: "CMD_POPUPMENU_DVD".into(),
                    inline: true
                },
            ]
        );
        let used = commands(&root);
        let names: Vec<&str> = used.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "CMD_POPUPMENU_DVD",
                "CMD_POPUP_USER_MENU",
                "ID_BASE_OPEN",
                "ID_X"
            ]
        );
        let popup = &used["CMD_POPUP_USER_MENU"];
        assert_eq!(popup.kinds, BTreeSet::from(["Button".to_owned()]));
        assert_eq!(popup.areas, BTreeSet::from(["MainSkinArea".to_owned()]));
        assert_eq!(
            used["ID_BASE_OPEN"].areas,
            BTreeSet::from(["Menu".to_owned()])
        );
    }
}
