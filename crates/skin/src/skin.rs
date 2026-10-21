//! A loaded skin: archive, preprocessed markup, string table and atlases.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::archive::SkinArchive;
use crate::error::SkinError;
use crate::image::Image;
use crate::lang::StringTable;
use crate::markup::{Element, parse_document};
use crate::preprocess::{Expanded, expand};

/// Skin files tried, in order, when the caller does not name one.
pub const VIDEO_SKIN_FILES: [&str; 3] = ["VideoSkin.xml", "ChildSkin.xml", "Skin.xml"];

/// A title-bar button that shows the program's logo: PotPlayer's skins
/// put their player's logo (Daum's, in BlackSpace) on a `CMD_POPUPMENU_ETC`
/// button in a title bar (`IsTitleBar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoSlot {
    /// The button's element id.
    pub element: usize,
    pub width: i32,
    pub height: i32,
}

/// The next skin's [`Skin::id`].
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A skin ready for layout and rendering.
#[derive(Debug, Clone)]
pub struct Skin {
    /// Unique for each loaded or tinted skin in the process: painters key
    /// their image caches by it, so two skins' `Main.png` never mix.
    id: u64,
    archive: SkinArchive,
    file: String,
    expanded: Expanded,
    root: Element,
    strings: StringTable,
    images: BTreeMap<String, Image>,
    /// Buttons drawn from an image RVP added (element id to image name).
    sprites: BTreeMap<usize, String>,
    warnings: Vec<String>,
}

impl Skin {
    /// Opens a `.dsf` file (or an unpacked skin directory) and loads its
    /// video skin.
    ///
    /// # Errors
    /// Fails when the archive cannot be read or has no video skin file.
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        Self::open_with(path, &[])
    }

    /// [`Skin::open`] with preprocessor `variables` besides `SkinName`
    /// (`SkinType=Touch` for the touch layout, M14).
    ///
    /// # Errors
    /// As [`Skin::open`].
    pub fn open_with(path: &Path, variables: &[(&str, &str)]) -> Result<Self, SkinError> {
        let archive = SkinArchive::open(path)?;
        let file = VIDEO_SKIN_FILES
            .iter()
            .find(|name| archive.get(name).is_some())
            .ok_or_else(|| SkinError::MissingFile(VIDEO_SKIN_FILES.join(" or ")))?;
        Self::load_with(archive, file, variables)
    }

    /// Loads `file` (for example `VideoSkin.xml`) from an archive.
    ///
    /// Images that cannot be decoded are reported in [`Skin::warnings`] and
    /// render as transparent.
    ///
    /// # Errors
    /// Fails when `file` is not in the archive.
    pub fn load(archive: SkinArchive, file: &str) -> Result<Self, SkinError> {
        Self::load_with(archive, file, &[])
    }

    /// [`Skin::load`] with preprocessor `variables` besides `SkinName`.
    ///
    /// # Errors
    /// As [`Skin::load`].
    pub fn load_with(
        archive: SkinArchive,
        file: &str,
        variables: &[(&str, &str)],
    ) -> Result<Self, SkinError> {
        if archive.get(file).is_none() {
            return Err(SkinError::MissingFile(file.to_owned()));
        }
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        let given = variables;
        let mut variables = BTreeMap::new();
        variables.insert("SkinName".to_owned(), stem.to_owned());
        for (name, value) in given {
            variables.insert((*name).to_owned(), (*value).to_owned());
        }
        let expanded = expand(&archive, file, &variables);
        let root = parse_document(&expanded.text);
        let strings = StringTable::collect(&root);
        let mut warnings = expanded.warnings.clone();
        let mut images = BTreeMap::new();
        for name in archive.names() {
            let is_image = Path::new(name).extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("bmp")
            });
            if !is_image {
                continue;
            }
            let Some(bytes) = archive.get(name) else {
                continue;
            };
            match Image::decode(name, bytes) {
                Ok(image) => {
                    images.insert(name.to_owned(), image);
                }
                Err(error) => warnings.push(error.to_string()),
            }
        }
        Ok(Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            archive,
            file: file.to_owned(),
            expanded,
            root,
            strings,
            images,
            sprites: BTreeMap::new(),
            warnings,
        })
    }

    /// Unique for each loaded or tinted skin of the process.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Whether the archive has `file` (`AudioSkin.xml`, ...).
    #[must_use]
    pub fn has_file(&self, file: &str) -> bool {
        self.archive.get(file).is_some()
    }

    /// The colour themes the skin lists (see [`crate::theme`]).
    #[must_use]
    pub fn color_themes(&self) -> Vec<crate::theme::ColorTheme> {
        crate::theme::themes(&self.root)
    }

    /// A copy with every image tinted (but the parts the skin keeps out of
    /// themes), under a new [`Skin::id`].
    #[must_use]
    pub fn tinted(&self, tint: crate::theme::Tint) -> Self {
        let mut skin = self.clone();
        skin.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let exclusions = crate::theme::exclusions(&self.root);
        for (name, image) in &mut skin.images {
            if name.starts_with("rvp-sprite-") {
                continue;
            }
            let keep: Vec<crate::geometry::Rect> = exclusions
                .iter()
                .filter(|(excluded, _)| excluded == name)
                .map(|(_, rect)| *rect)
                .collect();
            crate::theme::tint_image(image, tint, &keep);
        }
        skin
    }

    /// The skin archive.
    #[must_use]
    pub fn archive(&self) -> &SkinArchive {
        &self.archive
    }

    /// The skin file that was loaded (`VideoSkin.xml`, ...).
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// The preprocessed text with its line map.
    #[must_use]
    pub fn expanded(&self) -> &Expanded {
        &self.expanded
    }

    /// The root of the element tree (`#document`).
    #[must_use]
    pub fn root(&self) -> &Element {
        &self.root
    }

    /// The skin-defined menus (`<UserMenu>`, see [`crate::menu`]).
    #[must_use]
    pub fn user_menus(&self) -> Vec<crate::menu::UserMenu> {
        crate::menu::user_menus(&self.root)
    }

    /// Every command the skin's live elements use.
    #[must_use]
    pub fn commands(&self) -> std::collections::BTreeMap<String, crate::menu::CommandUse> {
        crate::menu::commands(&self.root)
    }

    /// The skin's string table.
    #[must_use]
    pub fn strings(&self) -> &StringTable {
        &self.strings
    }

    /// Problems found while loading: missing includes, broken images.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// A skin area such as `MainSkinArea` or `PlaylistSkinArea`.
    #[must_use]
    pub fn area(&self, name: &str) -> Option<&Element> {
        self.root.find(name)
    }

    /// A decoded image by file name (case-insensitive).
    #[must_use]
    pub fn image(&self, name: &str) -> Option<&Image> {
        self.images.get(&name.trim().to_ascii_lowercase())
    }

    /// The atlas image named by an area's `Image` attribute.
    #[must_use]
    pub fn area_atlas<'a>(&'a self, area: &'a Element) -> &'a str {
        area.attrs.get("Image").unwrap_or("")
    }

    /// The skin's context menu style: `<CustomPopupMenuDef>` (`alt`
    /// false) or `<CustomPopupMenuAlt>`, coloured from points of the main
    /// window's picture (`crate::menu`).
    #[must_use]
    pub fn menu_style(&self, alt: bool) -> Option<crate::menu::MenuStyle> {
        let name = if alt {
            "CustomPopupMenuAlt"
        } else {
            "CustomPopupMenuDef"
        };
        let element = self.root.find(name)?;
        let atlas = self
            .area("MainSkinArea")
            .map_or("", |area| self.area_atlas(area));
        let attrs = &element.attrs;
        let point =
            |x: &str, y: &str| self.atlas_color(atlas, attrs.int_or(x, 0), attrs.int_or(y, 0));
        let (x, y) = (attrs.int_or("FrameX", 0), attrs.int_or("FrameY", 0));
        let (width, height) = (attrs.int_or("FrameW", 0), attrs.int_or("FrameH", 0));
        Some(crate::menu::MenuStyle {
            back: self.atlas_color(atlas, x + width / 2, y + height / 2),
            border: self.atlas_color(atlas, x, y + height / 2),
            text: point("TextColorX", "TextColorY"),
            selected_text: point("SelTextColorX", "SelTextColorY"),
            selected_back: point("SelColorX", "SelColorY"),
            dim_text: point("DimTextColorX", "DimTextColorY"),
            line: point("LineColorX", "LineColorY"),
        })
    }

    /// Whether the skin's `<Logo>` section draws an idle logo (a part
    /// shown in state `CMD_DEFAULT_LOGO`) and one for a starting file
    /// (`CMD_LOADING_LOGO`). Flash logos (`<Logo IdleFileName='Logo.swf'>`)
    /// have no such parts.
    #[must_use]
    pub fn logo_states(&self) -> (bool, bool) {
        let (mut idle, mut loading) = (false, false);
        if let Some(logo) = self.area("Logo") {
            logo.walk(&mut |element| {
                let states = element.attrs.get("State").unwrap_or("");
                for state in states.split_whitespace() {
                    idle |= state == "CMD_DEFAULT_LOGO";
                    loading |= state == "CMD_LOADING_LOGO";
                }
            });
        }
        (idle, loading)
    }

    /// The logo buttons of every title bar, except those a skin marks
    /// `RvpLogo='1'` because they already show RVP's logo.
    #[must_use]
    pub fn logo_slots(&self) -> Vec<LogoSlot> {
        let mut slots = Vec::new();
        self.root.walk(&mut |element| {
            if !(element.is("Static") && element.attrs.flag("IsTitleBar")) {
                return;
            }
            for child in &element.children {
                let attrs = &child.attrs;
                let width = attrs.int_or("Width", 0);
                let height = attrs.int_or("Height", 0);
                if child.is("Button")
                    && attrs.get("CmdID") == Some("CMD_POPUPMENU_ETC")
                    && !attrs.flag("RvpLogo")
                    && width > 0
                    && height > 0
                {
                    slots.push(LogoSlot {
                        element: child.id,
                        width,
                        height,
                    });
                }
            }
        });
        slots
    }

    /// Draws button `element` from `image` instead of its sprite; the image
    /// holds the button states side by side, each as large as the button.
    pub fn replace_sprite(&mut self, element: usize, image: Image) {
        let name = format!("rvp-sprite-{element}");
        self.images.insert(name.clone(), image);
        self.sprites.insert(element, name);
    }

    /// The image that replaces the sprite of button `element`, if any.
    #[must_use]
    pub fn replaced_sprite(&self, element: usize) -> Option<&str> {
        self.sprites.get(&element).map(String::as_str)
    }

    /// The colour of an atlas pixel, used for `ColorX`/`ColorY` attributes.
    #[must_use]
    pub fn atlas_color(&self, atlas: &str, x: i32, y: i32) -> [u8; 4] {
        self.image(atlas)
            .map_or([255, 255, 255, 255], |image| image.pixel(x, y))
    }
}
