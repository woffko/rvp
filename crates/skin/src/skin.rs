//! A loaded skin: archive, preprocessed markup, string table and atlases.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::archive::SkinArchive;
use crate::error::SkinError;
use crate::image::Image;
use crate::lang::StringTable;
use crate::limits;
use crate::markup::{Attrs, Element, parse_within};
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

/// One of a skin's other windows ([`Skin::extra_windows`], M9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraWindow<'a> {
    /// The area's element name.
    pub area: &'a str,
    /// Its first size.
    pub width: i32,
    pub height: i32,
    /// `above` or `below` the main window.
    pub dock: &'a str,
    /// The `RVPWIN:` command that shows and hides it; `None`: always shown.
    pub toggle: Option<&'a str>,
    /// Whether it shows at first (`RvpShown`; without one, when it has no
    /// toggle).
    pub shown: bool,
    /// Its place from the main window's corner (`RvpDock='offset'`).
    pub offset: (i32, i32),
    /// Its smallest and largest sizes (themes write 99999 for a largest
    /// size of "any": the screen bounds it).
    pub min: (i32, i32),
    pub max: (i32, i32),
    /// Sizes go in these steps (a classic playlist's 25 by 29 pixels).
    pub step: Option<(i32, i32)>,
    /// The window's shape: polygons in its pixels (empty: rectangular).
    pub region: Vec<Vec<(i32, i32)>>,
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
    /// Fails when `file` is not in the archive, and with
    /// [`SkinError::Limit`] when the skin is over one of [`crate::limits`]:
    /// its expanded markup, a picture or its pictures together, or a
    /// window of its own.
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
        Self::load_within(archive, file, variables, limits::Pixels::default())
    }

    /// [`Skin::load_with`], with what the skin's pictures may take.
    fn load_within(
        archive: SkinArchive,
        file: &str,
        variables: &[(&str, &str)],
        mut budget: limits::Pixels,
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
        let expanded = expand(&archive, file, &variables)?;
        let mut root = parse_within(&expanded.text, false)?;
        check_windows(&root)?;
        add_fullscreen_bars(&mut root);
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
            match Image::decode_within(name, bytes, &mut budget) {
                Ok(image) => {
                    images.insert(name.to_owned(), image);
                }
                // A picture over a limit is not one that is broken.
                Err(error @ SkinError::Limit(_)) => return Err(error),
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

    /// Whether the skin file draws the audio's spectrum (a `Visual`
    /// element).
    #[must_use]
    pub fn has_visual(&self) -> bool {
        let mut found = false;
        self.root.walk(&mut |element| found |= element.is("Visual"));
        found
    }

    /// The other files of the skin its controls show (the `String` of
    /// `CMD_LOAD_SKIN_XML`: BlackSpace's audio skin links `AudioMini.xml`),
    /// in the order they appear.
    #[must_use]
    pub fn linked_files(&self) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        self.root.walk(&mut |element| {
            let attrs = &element.attrs;
            let loads =
                [attrs.get("CmdID"), attrs.get("AltCmdID")].contains(&Some("CMD_LOAD_SKIN_XML"));
            if let Some(file) = attrs.get("String").filter(|_| loads)
                && !files.iter().any(|known| known.eq_ignore_ascii_case(file))
            {
                files.push(file.to_owned());
            }
        });
        files
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

    /// The skin's windows besides the main one (M9): its `RvpWindow='1'`
    /// areas in document order.
    #[must_use]
    pub fn extra_windows(&self) -> Vec<ExtraWindow<'_>> {
        let mut areas = Vec::new();
        self.root.walk(&mut |element| {
            if element.attrs.flag("RvpWindow") {
                areas.push(element);
            }
        });
        areas
            .into_iter()
            .map(|area| {
                let attrs = &area.attrs;
                let step = attrs.get("RvpResizeStep").and_then(|step| {
                    let (w, h) = step.split_once(',')?;
                    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
                });
                let region = attrs
                    .get("RvpRegion")
                    .map(|text| {
                        text.split(';')
                            .map(|polygon| {
                                polygon
                                    .split_whitespace()
                                    .filter_map(|point| {
                                        let (x, y) = point.split_once(',')?;
                                        Some((x.parse().ok()?, y.parse().ok()?))
                                    })
                                    .collect()
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let (width, height) = (attrs.int_or("Width", 0), attrs.int_or("Height", 0));
                ExtraWindow {
                    area: area.name.as_str(),
                    width,
                    height,
                    dock: attrs.get("RvpDock").unwrap_or("below"),
                    toggle: attrs.get("RvpToggle"),
                    shown: attrs
                        .int("RvpShown")
                        .map_or(!attrs.has("RvpToggle"), |shown| shown != 0),
                    offset: (attrs.int_or("RvpOffsetX", 0), attrs.int_or("RvpOffsetY", 0)),
                    min: (
                        attrs.int("WidthLimit").filter(|w| *w > 0).unwrap_or(width),
                        attrs
                            .int("HeightLimit")
                            .filter(|h| *h > 0)
                            .unwrap_or(height),
                    ),
                    max: (
                        attrs.int("RvpMaxWidth").filter(|w| *w > 0).unwrap_or(width),
                        attrs
                            .int("RvpMaxHeight")
                            .filter(|h| *h > 0)
                            .unwrap_or(height),
                    ),
                    step,
                    region,
                }
            })
            .collect()
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
    /// `RvpLogo='1'` because they already show RVP's logo. RVP draws a
    /// picture for each, so a button larger than
    /// [`limits::LOGO_SIDE`] a side is no logo's place, and only the first
    /// [`limits::LOGO_SLOTS`] count.
    #[must_use]
    pub fn logo_slots(&self) -> Vec<LogoSlot> {
        let mut slots = Vec::new();
        self.root.walk(&mut |element| {
            if !(element.is("Static") && element.attrs.flag("IsTitleBar"))
                || slots.len() >= limits::LOGO_SLOTS
            {
                return;
            }
            for child in &element.children {
                let attrs = &child.attrs;
                let width = attrs.int_or("Width", 0);
                let height = attrs.int_or("Height", 0);
                if child.is("Button")
                    && attrs.get("CmdID") == Some("CMD_POPUPMENU_ETC")
                    && !attrs.flag("RvpLogo")
                    && (1..=limits::LOGO_SIDE).contains(&width)
                    && (1..=limits::LOGO_SIDE).contains(&height)
                    && slots.len() < limits::LOGO_SLOTS
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

/// A skin's own windows (`RvpWindow='1'`, [`Skin::extra_windows`]) are
/// made and drawn at the sizes the skin gives: they are few, and each of
/// a window's size at first (the screen bounds what they grow to).
fn check_windows(root: &Element) -> Result<(), SkinError> {
    let mut count = 0_usize;
    let mut over = None;
    root.walk(&mut |element| {
        if over.is_some() || !element.attrs.flag("RvpWindow") {
            return;
        }
        count += 1;
        if count > limits::EXTRA_WINDOWS {
            over = Some(limits::exceeded(
                "more windows of the skin's own",
                limits::EXTRA_WINDOWS,
            ));
            return;
        }
        let (width, height) = (
            element.attrs.int_or("Width", 0),
            element.attrs.int_or("Height", 0),
        );
        if !limits::window_fits(width, height) {
            over = Some(limits::exceeded(
                &format!("window {} of {width}x{height} pixels", element.name),
                format_args!(
                    "{} a side and {} pixels",
                    limits::WINDOW_SIDE,
                    limits::WINDOW_PIXELS
                ),
            ));
        }
    });
    over.map_or(Ok(()), Err)
}

/// A skin without `MainSkinAreaOscInside` (fourteen of PotPlayer's 22
/// bundled skins: gom, kmp, pot, zoom...) gets one built from its
/// `MainSkinArea`: PotPlayer's fullscreen shows that area's bottom-docked
/// parts across the bottom of the screen over the video, and nothing at
/// the top (the skin check's host captures, 2026-11-04). The parts keep
/// their element IDs (only one layout shows at a time); the new elements
/// get IDs past the document's.
fn add_fullscreen_bars(root: &mut Element) {
    if root.find("MainSkinAreaOscInside").is_some() {
        return;
    }
    let Some(main) = root.find("MainSkinArea") else {
        return;
    };
    let parts = main.child("Control").unwrap_or(main);
    let line = main.line;
    let mut next_id = 0;
    root.walk(&mut |element| next_id = next_id.max(element.id + 1));
    let mut new_element = |name: &str, attrs: &[(&str, &str)]| {
        let mut list = Attrs::default();
        for (key, value) in attrs {
            list.push(key, value);
        }
        let mut element = Element::new(name, list, line);
        element.id = next_id;
        next_id += 1;
        element
    };
    let mut control = new_element("Control", &[]);
    control.children = parts
        .children
        .iter()
        .filter(|part| {
            part.attrs
                .get("Align")
                .is_some_and(|align| align.eq_ignore_ascii_case("Bottom"))
        })
        .cloned()
        .collect();
    control.children.push(new_element(
        "Static",
        &[("Align", "Client"), ("Contain", "CMD_VIDEO_WINDOW")],
    ));
    let mut area = new_element("MainSkinAreaOscInside", &[]);
    area.attrs = main.attrs.clone();
    area.attrs.push("RvpBuilt", "1");
    area.children.push(control);
    insert_after(root, "MainSkinArea", area);
}

/// Puts `element` after the first element named `name`, among its
/// siblings.
fn insert_after(parent: &mut Element, name: &str, element: Element) -> Option<Element> {
    if let Some(index) = parent.children.iter().position(|child| child.is(name)) {
        parent.children.insert(index + 1, element);
        return None;
    }
    let mut element = Some(element);
    for child in &mut parent.children {
        if let Some(left) = element.take() {
            element = insert_after(child, name, left);
        }
    }
    element
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A skin's archive: `markup` as `VideoSkin.xml`, and its other files.
    fn archive(markup: &str, files: &[(&str, Vec<u8>)]) -> SkinArchive {
        let mut members = vec![("VideoSkin.xml".to_owned(), markup.as_bytes().to_vec())];
        members.extend(
            files
                .iter()
                .map(|(name, bytes)| ((*name).to_owned(), bytes.clone())),
        );
        SkinArchive::from_files("test", members)
    }

    fn limit(result: Result<Skin, SkinError>) -> String {
        match result {
            Err(SkinError::Limit(what)) => what,
            Err(other) => panic!("not a limit: {other}"),
            Ok(skin) => panic!("loaded with {} pictures", skin.images.len()),
        }
    }

    #[test]
    fn the_pictures_of_a_skin_share_its_budget() {
        let picture = Image::new(8, 8);
        let files = [
            ("a.png", picture.encode_png().unwrap()),
            ("b.png", picture.encode_png().unwrap()),
            ("c.bmp", picture.encode_bmp()),
            // No picture: its header alone, which gives 4 x 4 pixels.
            (
                "broken.png",
                b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\0\x04\0\0\0\x04\x08\x06\0\0\0".to_vec(),
            ),
        ];
        let load = |pixels: u64| {
            Skin::load_within(
                archive("<Skin/>", &files),
                "VideoSkin.xml",
                &[],
                limits::Pixels::with(pixels),
            )
        };
        // Three pictures of 64 pixels; the broken one is a warning and
        // takes nothing.
        let skin = load(3 * 64).unwrap();
        assert_eq!(skin.images.len(), 3);
        assert_eq!(skin.warnings().len(), 1);
        assert!(limit(load(3 * 64 - 1)).contains("pixels together"));
        // One picture over its own limit: a header of a few bytes.
        let mut huge = files[3].1.clone();
        huge[16..24].copy_from_slice(&[0, 1, 0, 0, 0, 1, 0, 0]);
        let skin = Skin::load(archive("<Skin/>", &[("huge.png", huge)]), "VideoSkin.xml");
        assert!(limit(skin).contains("huge.png"));
    }

    #[test]
    fn a_skin_over_a_markup_limit_does_not_load() {
        let load = |markup: &str| Skin::load(archive(markup, &[]), "VideoSkin.xml");
        assert!(load("<Skin><MainSkinArea/></Skin>").is_ok());
        // Its includes without end.
        let twice = "#include<VideoSkin.xml>\n#include<VideoSkin.xml>";
        assert!(limit(load(twice)).contains("includes"));
        // Its elements nested without end.
        assert!(limit(load(&"<Static>".repeat(1000))).contains("deeper"));
        // The areas RVP adds to a skin without bars for fullscreen do not
        // count against the skin.
        let deepest = "<a>".repeat(limits::MARKUP_DEPTH - 3);
        let markup =
            format!("<Skin><MainSkinArea><Control>{deepest}</Control></MainSkinArea></Skin>");
        assert!(
            load(&markup)
                .unwrap()
                .area("MainSkinAreaOscInside")
                .is_some()
        );
    }

    #[test]
    fn a_skins_own_windows_are_few_and_of_a_windows_size() {
        let window = |n: usize, width: i32, height: i32| {
            format!(
                "<W{n} RvpWindow='1' Width='{width}' Height='{height}' \
                 RvpMaxWidth='99999' RvpMaxHeight='99999'><Control/></W{n}>"
            )
        };
        let load = |windows: &str| {
            Skin::load(
                archive(&format!("<Skin>{windows}</Skin>"), &[]),
                "VideoSkin.xml",
            )
        };
        let many = |count: usize| (0..count).map(|n| window(n, 275, 116)).collect::<String>();
        let skin = load(&many(limits::EXTRA_WINDOWS)).unwrap();
        let windows = skin.extra_windows();
        assert_eq!(windows.len(), limits::EXTRA_WINDOWS);
        // A largest size of "any" is no size the window is made at.
        assert_eq!(windows[0].max, (99_999, 99_999));
        assert!(limit(load(&many(limits::EXTRA_WINDOWS + 1))).contains("windows"));
        // A window is drawn at its first size at once.
        assert!(load(&window(0, limits::WINDOW_SIDE, 1024)).is_ok());
        assert!(limit(load(&window(0, 50_000, 50_000))).contains("50000x50000"));
        assert!(limit(load(&window(0, limits::WINDOW_SIDE + 1, 1))).contains("W0"));
        assert!(limit(load(&window(0, 3000, 3000))).contains("W0"));
    }

    #[test]
    fn logo_buttons_are_of_a_buttons_size_and_few() {
        let button = |width: i32, height: i32| {
            format!("<Button CmdID='CMD_POPUPMENU_ETC' Width='{width}' Height='{height}' />")
        };
        let load = |buttons: &str| {
            let markup = format!(
                "<Skin><MainSkinArea><Control><Static IsTitleBar='1'>{buttons}</Static>\
                 </Control></MainSkinArea></Skin>"
            );
            Skin::load(archive(&markup, &[]), "VideoSkin.xml").unwrap()
        };
        // A button of 30,000 pixels a side is no logo's place: RVP would
        // draw a picture of four times that size for it.
        let buttons = [
            button(30, 16),
            button(30_000, 30_000),
            button(limits::LOGO_SIDE, limits::LOGO_SIDE),
            button(limits::LOGO_SIDE + 1, 16),
            button(16, limits::LOGO_SIDE + 1),
        ]
        .concat();
        let sizes: Vec<(i32, i32)> = load(&buttons)
            .logo_slots()
            .iter()
            .map(|slot| (slot.width, slot.height))
            .collect();
        assert_eq!(sizes, [(30, 16), (limits::LOGO_SIDE, limits::LOGO_SIDE)]);
        // However many buttons a skin has, RVP draws for the first few.
        let skin = load(&button(30, 16).repeat(1000));
        assert_eq!(skin.logo_slots().len(), limits::LOGO_SLOTS);
    }
}
