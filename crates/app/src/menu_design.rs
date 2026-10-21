//! M14: which context menu the right click opens and how it looks (Skins >
//! Context Menu Design and Context Menu Style, and General > Skins' menu
//! options).
//!
//! * Design: RVP's whole menu ("Skin defined/Default": PotPlayer's skins
//!   define no whole menu of their own, so both are RVP's), or a menu file
//!   of RVP's `menus` folder beside the settings or, on Windows, of
//!   PotPlayer's installed `Menus` folder (read only), such as PotPlayer's
//!   Simple popup menu (`rvp_skin::menu::menu_file`).
//! * Style: RVP's look, the system's own menus, or the skin's first or
//!   second style (`rvp_skin::Skin::menu_style`); "Auto select" takes the
//!   skin's first style when it has one. Dark colours when the system is
//!   dark and "Use system dark menu mode" is on, a transparency and a size.
//! * Context Menu Items: top-level items the user hides.

use std::path::{Path, PathBuf};

use crate::app::{App, UiRequest};

/// Design choices that are not menu files.
pub const MENU_SKIN: &str = "skin";
pub const MENU_DEFAULT: &str = "default";
/// The styles, in PotPlayer's order.
pub const MENU_STYLES: &[&str] = &["auto", "system", "skin-1", "skin-2"];
/// PotPlayer's menu files, read only.
const POTPLAYER_MENUS: &str = r"C:\Program Files\DAUM\PotPlayer\Menus";

/// The `.xml` files in `folder`, by lower-case file name (the first found
/// wins).
fn menu_files_in(folder: &Path, out: &mut std::collections::BTreeMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
            && let Some(name) = path.file_name()
        {
            out.entry(name.to_string_lossy().to_lowercase())
                .or_insert(path);
        }
    }
}

impl App {
    /// RVP's folder of context menu files.
    #[must_use]
    pub fn menus_folder(&self) -> PathBuf {
        self.skins_folder().with_file_name("menus")
    }

    /// The menu files to choose from, by file name.
    #[must_use]
    pub fn menu_files(&self) -> Vec<PathBuf> {
        let mut found = std::collections::BTreeMap::new();
        menu_files_in(&self.menus_folder(), &mut found);
        if cfg!(windows) {
            menu_files_in(Path::new(POTPLAYER_MENUS), &mut found);
        }
        found.into_values().collect()
    }

    /// The menu file the design names, read; `None` for RVP's whole menu
    /// or a file that is gone or holds no menu.
    #[must_use]
    pub fn menu_design_file(&self) -> Option<rvp_skin::menu::MenuFile> {
        let design = &self.settings.skins.menu_design;
        if design == MENU_SKIN || design == MENU_DEFAULT {
            return None;
        }
        let path = self.menu_files().into_iter().find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(design))
        })?;
        let bytes = std::fs::read(path).ok()?;
        rvp_skin::menu::menu_file(&rvp_skin::text::decode(&bytes))
    }

    /// Sets the design to `design` (a constant or a menu file's name).
    pub(crate) fn set_menu_design(&mut self, design: &str) -> UiRequest {
        design.clone_into(&mut self.settings.skins.menu_design);
        let shown = match design {
            MENU_SKIN | MENU_DEFAULT => "RVP's",
            name => name,
        };
        self.save_settings();
        UiRequest::Osd(format!("Context menu: {shown}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_list_menu_files_by_name() {
        let dir = std::env::temp_dir().join(format!("rvp-menus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["Short.XML", "a.xml", "notes.txt"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let mut found = std::collections::BTreeMap::new();
        menu_files_in(&dir, &mut found);
        let names: Vec<&String> = found.keys().collect();
        assert_eq!(names, ["a.xml", "short.xml"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
