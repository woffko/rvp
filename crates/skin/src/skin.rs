//! A loaded skin: archive, preprocessed markup, string table and atlases.

use std::collections::BTreeMap;
use std::path::Path;

use crate::archive::SkinArchive;
use crate::error::SkinError;
use crate::image::Image;
use crate::lang::StringTable;
use crate::markup::{Element, parse_document};
use crate::preprocess::{Expanded, expand};

/// Skin files tried, in order, when the caller does not name one.
pub const VIDEO_SKIN_FILES: [&str; 3] = ["VideoSkin.xml", "ChildSkin.xml", "Skin.xml"];

/// A skin ready for layout and rendering.
#[derive(Debug)]
pub struct Skin {
    archive: SkinArchive,
    file: String,
    expanded: Expanded,
    root: Element,
    strings: StringTable,
    images: BTreeMap<String, Image>,
    warnings: Vec<String>,
}

impl Skin {
    /// Opens a `.dsf` file (or an unpacked skin directory) and loads its
    /// video skin.
    ///
    /// # Errors
    /// Fails when the archive cannot be read or has no video skin file.
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        let archive = SkinArchive::open(path)?;
        let file = VIDEO_SKIN_FILES
            .iter()
            .find(|name| archive.get(name).is_some())
            .ok_or_else(|| SkinError::MissingFile(VIDEO_SKIN_FILES.join(" or ")))?;
        Self::load(archive, file)
    }

    /// Loads `file` (for example `VideoSkin.xml`) from an archive.
    ///
    /// Images that cannot be decoded are reported in [`Skin::warnings`] and
    /// render as transparent.
    ///
    /// # Errors
    /// Fails when `file` is not in the archive.
    pub fn load(archive: SkinArchive, file: &str) -> Result<Self, SkinError> {
        if archive.get(file).is_none() {
            return Err(SkinError::MissingFile(file.to_owned()));
        }
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        let mut variables = BTreeMap::new();
        variables.insert("SkinName".to_owned(), stem.to_owned());
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
            archive,
            file: file.to_owned(),
            expanded,
            root,
            strings,
            images,
            warnings,
        })
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

    /// The colour of an atlas pixel, used for `ColorX`/`ColorY` attributes.
    #[must_use]
    pub fn atlas_color(&self, atlas: &str, x: i32, y: i32) -> [u8; 4] {
        self.image(atlas)
            .map_or([255, 255, 255, 255], |image| image.pixel(x, y))
    }
}
