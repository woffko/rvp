//! M14: PotPlayer's logos (Skins > Logo and General > Skins > Logo): the
//! picture over the video area while nothing plays ("while playback is
//! off") and while a file starts.
//!
//! * "[Skin defined]": the skin's own logo, drawn by the skin from its
//!   `<Logo>` section (`State='CMD_DEFAULT_LOGO'`, and `CMD_LOADING_LOGO`
//!   while a file starts). PotPlayer's Flash logos (`<Logo
//!   IdleFileName='Logo.swf'>`, BlackSpace's) cannot show, since Qt plays no
//!   Flash: RVP's logo shows instead, as it does for skins without a logo.
//! * "<Default logo>": RVP's logo.
//! * The pictures (PNG, JPEG, BMP, GIF) of RVP's `logos` folder beside the
//!   settings and, on Windows, of PotPlayer's installed `Logos` folder
//!   (read only), chosen by file name; "Change logo randomly on each
//!   loading" picks one of them for each file instead.
//! * Its place in the video area: centred, in a corner, or fitted to it.

use std::path::{Path, PathBuf};

use crate::app::{App, UiRequest};

/// The logo choices that are not pictures.
pub const LOGO_SKIN: &str = "skin";
pub const LOGO_DEFAULT: &str = "default";
/// The start logo is the idle one.
pub const LOGO_SAME: &str = "same";
/// Where the logo shows in the video area.
pub const LOGO_POSITIONS: &[&str] = &[
    "center",
    "top-left",
    "top-right",
    "bottom-left",
    "bottom-right",
    "fit",
];
/// The pictures a logo folder offers (what Qt's image readers take).
const PICTURES: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif"];
/// PotPlayer's logo folder, read only.
const POTPLAYER_LOGOS: &str = r"C:\Program Files\DAUM\PotPlayer\Logos";

/// What shows over the video area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Logo {
    /// The skin draws its logo: the one for a starting file (`true`) or
    /// the idle one.
    Skin(bool),
    /// RVP's logo.
    Default,
    /// A picture.
    File(PathBuf),
}

/// The pictures in `folder`, by lower-case file name (the first found
/// wins).
fn pictures_in(folder: &Path, out: &mut std::collections::BTreeMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let picture = path
            .extension()
            .is_some_and(|ext| PICTURES.iter().any(|known| ext.eq_ignore_ascii_case(known)));
        if picture && let Some(name) = path.file_name() {
            out.entry(name.to_string_lossy().to_lowercase())
                .or_insert(path);
        }
    }
}

impl App {
    /// RVP's logo folder (beside the settings).
    #[must_use]
    pub fn logos_folder(&self) -> PathBuf {
        self.skins_folder().with_file_name("logos")
    }

    /// The pictures to choose from, by file name.
    #[must_use]
    pub fn logo_list(&self) -> Vec<PathBuf> {
        let mut found = std::collections::BTreeMap::new();
        pictures_in(&self.logos_folder(), &mut found);
        if cfg!(windows) {
            pictures_in(Path::new(POTPLAYER_LOGOS), &mut found);
        }
        found.into_values().collect()
    }

    /// The logo to show now: the start logo while a file starts, else the
    /// idle one.
    #[must_use]
    pub fn logo(&mut self) -> Logo {
        let starting = self.starting();
        let skins = &self.settings.skins;
        let choice = if starting && skins.start_logo != LOGO_SAME {
            skins.start_logo.clone()
        } else {
            skins.idle_logo.clone()
        };
        if skins.random_logo {
            let list = self.logo_list();
            if !list.is_empty() {
                let index = *self.random_logo.get_or_insert_with(|| {
                    // Once per file: the clock's nanoseconds.
                    let nanos = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |time| time.subsec_nanos());
                    usize::try_from(nanos).unwrap_or(0)
                });
                return Logo::File(list[index % list.len()].clone());
            }
        }
        match choice.as_str() {
            LOGO_SKIN => {
                let info = &self.skin_info;
                if starting && info.loading_logo {
                    Logo::Skin(true)
                } else if info.logo {
                    Logo::Skin(false)
                } else {
                    Logo::Default
                }
            }
            LOGO_DEFAULT | LOGO_SAME => Logo::Default,
            name => self
                .logo_list()
                .into_iter()
                .find(|path| {
                    path.file_name()
                        .is_some_and(|file| file.to_string_lossy().eq_ignore_ascii_case(name))
                })
                .map_or(Logo::Default, Logo::File),
        }
    }

    /// The name of the file that starts, for the skin's loading logo
    /// (`CMD_LOADING_MESSAGE`).
    #[must_use]
    pub fn starting_name(&self) -> String {
        self.library()
            .playing()
            .map(|(_, entry)| {
                entry
                    .title
                    .clone()
                    .unwrap_or_else(|| crate::view::file_name(&entry.path).to_owned())
            })
            .unwrap_or_default()
    }

    /// A file starts: "Change logo randomly" picks again.
    pub(crate) fn next_random_logo(&mut self) {
        self.random_logo = None;
    }

    /// Sets the idle (`false`) or start logo to `choice` (a constant or a
    /// picture's file name).
    pub(crate) fn set_logo(&mut self, start: bool, choice: &str) -> UiRequest {
        let skins = &mut self.settings.skins;
        let target = if start {
            &mut skins.start_logo
        } else {
            &mut skins.idle_logo
        };
        choice.clone_into(target);
        let shown = match choice {
            LOGO_SKIN => "the skin's",
            LOGO_DEFAULT => "RVP's",
            LOGO_SAME => "the idle logo",
            name => name,
        };
        let text = format!(
            "Logo {}: {shown}",
            if start {
                "while starting"
            } else {
                "while idle"
            }
        );
        self.save_settings();
        UiRequest::SkinChanged(text)
    }

    /// Whether the idle or start logo is `choice`.
    #[must_use]
    pub(crate) fn logo_is(&self, start: bool, choice: &str) -> bool {
        let skins = &self.settings.skins;
        let current = if start {
            &skins.start_logo
        } else {
            &skins.idle_logo
        };
        !skins.random_logo && current.eq_ignore_ascii_case(choice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_list_pictures_by_name() {
        let dir = std::env::temp_dir().join(format!("rvp-logos-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["b.PNG", "a.jpg", "notes.txt"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let mut found = std::collections::BTreeMap::new();
        pictures_in(&dir, &mut found);
        let names: Vec<&String> = found.keys().collect();
        assert_eq!(names, ["a.jpg", "b.png"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
