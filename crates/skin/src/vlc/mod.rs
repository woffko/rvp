//! VLC's skins (skins2, `.vlt`; M9, `docs/spikes/s6-other-skins.md`):
//! read into a model here and turned into an RVP skin ([`build`]) that
//! RVP's engine lays out, paints and hit-tests.
//!
//! What VLC 3 does, and RVP reproduces (facts from VLC's source, read for
//! the study; no code from it):
//!
//! * A `.vlt` is a gzip'd tar or a zip archive; `theme.xml` is the
//!   theme, found by name in any folder (the shallowest here), and its
//!   folder is the base of every file it names. `\` in names is a folder
//!   separator (darklounge's archive stores it in member names).
//! * Bitmaps are decoded with `alphacolor`'s exact colour made
//!   transparent; an animated bitmap (`nbframes`, frames stacked) shows
//!   its first frame here. `SubBitmap`s are rectangles of their bitmap.
//! * A control whose bitmap, font or action VLC does not know is left
//!   out, as VLC leaves it out; `visible` is an expression ([`expr`]).
//! * Controls are placed in their box (the layout, or the innermost
//!   `Panel`, plus `Group` offsets): their top-left corner tied to the
//!   box's corner `lefttop` names, the bottom-right to `rightbottom`'s;
//!   `xkeepratio`/`ykeepratio` keep the ratio of the free space instead.
//! * The last `Layout` of a window is the one shown first.
//!
//! RVP's own rules: a theme is read within [`crate::limits`] (its
//! archive, its markup, each picture and all of them, each expression),
//! and does not load beyond them. Each `Bitmap` and `SubBitmap` is a
//! picture of its own, however many are made of one file; a font file is
//! kept once. A slider's background is `nbhoriz` by `nbvert` pictures
//! that lie in its bitmap ([`SliderBackground::cell`]): one whose pads
//! put them elsewhere is left out, and the slider shows without it.

pub mod build;
pub mod expr;

use std::collections::BTreeMap;
use std::path::Path;

use crate::archive;
use crate::error::SkinError;
use crate::image::Image;
use crate::limits;
use crate::markup::{Element, parse_within};

/// Which corner of its box a corner of a control is tied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Corner {
    pub right: bool,
    pub bottom: bool,
}

impl Corner {
    fn read(value: Option<&str>) -> Self {
        let value = value.unwrap_or("lefttop");
        Self {
            right: value.starts_with("right"),
            bottom: value.ends_with("bottom"),
        }
    }
}

/// A control's place in its box (the box's design size `box_w`x`box_h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub lefttop: Corner,
    pub rightbottom: Corner,
    pub keep_x: bool,
    pub keep_y: bool,
    pub box_w: i32,
    pub box_h: i32,
}

/// A font of the theme.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The family Qt and RVP's renderer know it by.
    pub family: String,
    /// Pixels.
    pub size: i32,
}

/// A slider's background: a grid of pictures, one per value.
#[derive(Debug, Clone, PartialEq)]
pub struct SliderBackground {
    pub image: String,
    pub columns: i32,
    pub rows: i32,
    pub pad_x: i32,
    pub pad_y: i32,
}

/// One control of a layout, in document order.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    Panel {
        place: Place,
        children: Vec<Control>,
    },
    Image {
        place: Place,
        image: String,
        /// `mosaic`, `scale`, or anything else (fit, keeping the aspect).
        resize: String,
        action: String,
        /// What a double click runs (`action2`).
        double: String,
        visible: Option<String>,
    },
    Button {
        place: Place,
        up: String,
        down: String,
        over: String,
        action: String,
        tooltip: String,
        visible: Option<String>,
    },
    Checkbox {
        place: Place,
        /// Up, down and over of the two sets (state false, true).
        images: [[String; 3]; 2],
        actions: [String; 2],
        tooltips: [String; 2],
        state: String,
        visible: Option<String>,
    },
    Text {
        place: Place,
        font: String,
        text: String,
        color: [u8; 3],
        /// `left`, `center` or `right`.
        alignment: String,
        visible: Option<String>,
    },
    Slider {
        place: Place,
        up: String,
        down: String,
        points: Vec<(i32, i32)>,
        value: String,
        tooltip: String,
        background: Option<SliderBackground>,
        visible: Option<String>,
    },
    Playtree {
        place: Place,
        background: Option<String>,
        fg: [u8; 3],
        play: [u8; 3],
        bg: [u8; 3],
        selection: [u8; 3],
        visible: Option<String>,
    },
    Video {
        place: Place,
        visible: Option<String>,
    },
}

/// One look of a window.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub id: String,
    pub width: i32,
    pub height: i32,
    /// The smallest and largest sizes (equal to the size: fixed).
    pub min: (i32, i32),
    pub max: (i32, i32),
    pub controls: Vec<Control>,
}

impl Layout {
    /// Whether a `Video` control lies in it.
    #[must_use]
    pub fn has_video(&self) -> bool {
        fn any(controls: &[Control]) -> bool {
            controls.iter().any(|control| match control {
                Control::Video { .. } => true,
                Control::Panel { children, .. } => any(children),
                _ => false,
            })
        }
        any(&self.controls)
    }
}

/// A window of the theme.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub id: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub visible: bool,
    pub layouts: Vec<Layout>,
}

/// A loaded VLC theme.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub images: BTreeMap<String, Image>,
    pub fonts: BTreeMap<String, Font>,
    /// The theme's font files (registered with the system's fonts).
    pub font_files: Vec<(String, Vec<u8>)>,
    pub windows: Vec<Window>,
    pub warnings: Vec<String>,
}

/// What a built VLC skin asks of the window ([`build`]): its conditions
/// (each sets the flag `VLCX:N` while true) and its texts' templates
/// (`VLCTEXT:N`, with VLC's `$` codes).
#[derive(Debug, Clone, Default)]
pub struct Runtime {
    pub conditions: Vec<(usize, expr::Expr)>,
    pub texts: Vec<(usize, String)>,
}

impl Runtime {
    /// Reads them from a skin (empty for other skins).
    #[must_use]
    pub fn of(skin: &crate::Skin) -> Self {
        let mut runtime = Self::default();
        if let Some(list) = skin.area("RvpConditions") {
            for item in &list.children {
                if let (Some(n), Some(text)) = (item.attrs.int("N"), item.attrs.get("X"))
                    && let (Ok(n), Some(expr)) = (usize::try_from(n), expr::Expr::parse(text))
                {
                    runtime.conditions.push((n, expr));
                }
            }
        }
        if let Some(list) = skin.area("RvpTexts") {
            for item in &list.children {
                if let (Some(n), Some(text)) = (item.attrs.int("N"), item.attrs.get("X"))
                    && let Ok(n) = usize::try_from(n)
                {
                    runtime.texts.push((n, text.to_owned()));
                }
            }
        }
        runtime
    }

    /// Sets the conditions' flags in `flags` from VLC's variables
    /// (`var`: a variable's value; unknown ones count as false).
    pub fn set_flags(
        &self,
        flags: &mut std::collections::BTreeSet<String>,
        var: &mut impl FnMut(&str) -> bool,
    ) {
        for (n, expr) in &self.conditions {
            let flag = format!("{}{n}", build::CONDITION);
            if expr.eval(&mut |name| Some(var(name))).unwrap_or(true) {
                flags.insert(flag);
            } else {
                flags.remove(&flag);
            }
        }
    }
}

/// Whether `path` names a VLC skin.
#[must_use]
pub fn is_vlc(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("vlt"))
}

/// The members of a theme's archive: a zip, a gzip'd tar or a tar.
///
/// # Errors
/// Fails when the bytes are none of them, or the archive is over one of
/// [`crate::limits`].
pub fn members(bytes: &[u8]) -> Result<archive::Members, SkinError> {
    if bytes.starts_with(b"PK") {
        archive::zip_members(bytes)
    } else {
        archive::tar_members(bytes)
    }
}

/// `#RRGGBB`.
fn color(value: Option<&str>, default: [u8; 3]) -> [u8; 3] {
    let Some(hex) = value.and_then(|value| value.trim().strip_prefix('#')) else {
        return default;
    };
    match u32::from_str_radix(hex, 16) {
        Ok(number) if hex.len() == 6 => {
            let [_, r, g, b] = number.to_be_bytes();
            [r, g, b]
        }
        _ => default,
    }
}

/// XML's entities in an attribute value (VLC's XML reader decodes them).
fn unescape(value: &str) -> String {
    if !value.contains('&') {
        return value.to_owned();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            break;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = decoded {
            out.push(c);
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// A font file's family name (its name table), when it has one.
fn font_family(data: &[u8]) -> Option<String> {
    let font = swash::FontRef::from_index(data, 0)?;
    let family = font
        .localized_strings()
        .find_by_id(swash::StringId::Family, None)?;
    let name: String = family.chars().collect();
    (!name.is_empty()).then_some(name)
}

/// A dimension: pixels (`N` or `Npx`), or `N%` of `whole`.
fn dimension(value: Option<&str>, whole: i32, default: i32) -> i32 {
    let Some(value) = value.map(str::trim) else {
        return default;
    };
    if let Some(percent) = value.strip_suffix('%') {
        return percent.trim().parse::<f64>().map_or(default, |p| {
            // Percentages of a box: small.
            #[allow(clippy::cast_possible_truncation)]
            let pixels = (p * f64::from(whole) / 100.0).round() as i32;
            pixels
        });
    }
    value
        .trim_end_matches("px")
        .trim()
        .parse::<f64>()
        // Pixel values: small.
        .map_or(default, |p| {
            #[allow(clippy::cast_possible_truncation)]
            let pixels = p as i32;
            pixels
        })
}

/// The reader of one theme.
struct Reader<'a> {
    files: &'a BTreeMap<String, Vec<u8>>,
    base: String,
    images: BTreeMap<String, Image>,
    fonts: BTreeMap<String, Font>,
    font_files: Vec<(String, Vec<u8>)>,
    warnings: Vec<String>,
    /// What the theme's pictures may still take: each `Bitmap` and
    /// `SubBitmap` is a picture of its own, whatever file it comes from.
    budget: limits::Pixels,
}

impl<'a> Reader<'a> {
    /// A file the theme names, relative to its folder.
    fn file(&self, name: &str) -> Option<&'a Vec<u8>> {
        let name = name.replace('\\', "/");
        let path = if self.base.is_empty() {
            name
        } else {
            format!("{}/{name}", self.base)
        };
        self.files.get(&path).or_else(|| {
            self.files
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(&path))
                .map(|(_, bytes)| bytes)
        })
    }

    fn bitmap(&mut self, element: &Element) -> Result<(), SkinError> {
        let attrs = &element.attrs;
        let (Some(id), Some(file)) = (attrs.get("id"), attrs.get("file")) else {
            return Ok(());
        };
        let file = unescape(file);
        let Some(bytes) = self.file(&file) else {
            self.warnings.push(format!("bitmap {id}: no file {file}"));
            return Ok(());
        };
        let mut image = match Image::decode_within(&file, bytes, &mut self.budget) {
            Ok(image) => image,
            // A picture over a limit is not one that is broken.
            Err(error @ SkinError::Limit(_)) => return Err(error),
            Err(error) => {
                self.warnings.push(error.to_string());
                return Ok(());
            }
        };
        let key = color(attrs.get("alphacolor"), [255, 0, 255]);
        for pixel in image.data_mut().as_chunks_mut::<4>().0 {
            if pixel[..3] == key {
                pixel[3] = 0;
            }
        }
        let mut parts = Vec::new();
        for sub in element
            .children
            .iter()
            .filter(|child| child.is("SubBitmap"))
        {
            let sub_attrs = &sub.attrs;
            let Some(sub_id) = sub_attrs.get("id") else {
                continue;
            };
            let (x, y, w, h) = (
                sub_attrs.int_or("x", 0),
                sub_attrs.int_or("y", 0),
                sub_attrs.int_or("width", 0),
                sub_attrs.int_or("height", 0),
            );
            let fits = x >= 0
                && y >= 0
                && w > 0
                && h > 0
                && i64::from(x) + i64::from(w) <= i64::from(image.width())
                && i64::from(y) + i64::from(h) <= i64::from(image.height());
            if !fits {
                continue;
            }
            self.budget
                .take(sub_id, w.unsigned_abs(), h.unsigned_abs())?;
            let crop = crate::compose::crop(&image, x, y, w, h);
            let frames = sub_attrs.int("nbframes").unwrap_or(1).max(1);
            parts.push((sub_id, first_frame(crop, frames, &mut self.budget)?));
        }
        // The bitmap itself, then its parts: of two pictures with one id
        // the later is the one that counts.
        let frames = attrs.int("nbframes").unwrap_or(1).max(1);
        let first = first_frame(image, frames, &mut self.budget)?;
        self.images.insert(id.to_owned(), first);
        for (sub_id, part) in parts {
            self.images.insert(sub_id.to_owned(), part);
        }
        Ok(())
    }

    fn font(&mut self, element: &Element) {
        let attrs = &element.attrs;
        let (Some(id), Some(file)) = (attrs.get("id"), attrs.get("file")) else {
            return;
        };
        let size = attrs.int("size").unwrap_or(12);
        let file = unescape(file);
        let Some(bytes) = self.file(&file) else {
            self.warnings.push(format!("font {id}: no file {file}"));
            return;
        };
        let Some(family) = font_family(bytes) else {
            self.warnings
                .push(format!("font {id}: no family name in {file}"));
            return;
        };
        // One copy of a file, however many fonts are made of it.
        if !self.font_files.iter().any(|(name, _)| *name == file) {
            self.font_files.push((file, bytes.clone()));
        }
        self.fonts.insert(id.to_owned(), Font { family, size });
    }
}

/// The first of `frames` pictures stacked from the top. It takes the
/// whole picture's place in the theme's `budget`.
fn first_frame(image: Image, frames: i32, budget: &mut limits::Pixels) -> Result<Image, SkinError> {
    if frames <= 1 {
        return Ok(image);
    }
    let height = (i32::try_from(image.height()).unwrap_or(0) / frames).max(1);
    budget.take("an animation's frame", image.width(), height.unsigned_abs())?;
    let first = crate::compose::crop(
        &image,
        0,
        0,
        i32::try_from(image.width()).unwrap_or(0),
        height,
    );
    budget.give_back(image.width(), image.height());
    Ok(first)
}

impl Theme {
    /// Opens a `.vlt` file.
    ///
    /// # Errors
    /// Fails when the file cannot be read, holds no archive, or has no
    /// `theme.xml`, and with [`SkinError::Limit`] when the theme is over
    /// one of [`crate::limits`].
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let members = members(&archive::read_file(path)?)?;
        Self::from_members(&name, members)
    }

    /// A theme from its archive's members.
    ///
    /// # Errors
    /// Fails without `theme.xml`, and with [`SkinError::Limit`] when its
    /// markup, a picture or its pictures together, an expression or a
    /// slider's background is over its limit.
    pub fn from_members(name: &str, members: Vec<(String, Vec<u8>)>) -> Result<Self, SkinError> {
        Self::from_members_within(name, members, limits::Pixels::default())
    }

    /// [`Theme::from_members`], with what the theme's pictures may take.
    fn from_members_within(
        name: &str,
        members: Vec<(String, Vec<u8>)>,
        budget: limits::Pixels,
    ) -> Result<Self, SkinError> {
        let files: BTreeMap<String, Vec<u8>> = members
            .into_iter()
            .map(|(member, bytes)| {
                let member = member.replace('\\', "/");
                (member.trim_start_matches("./").to_owned(), bytes)
            })
            .collect();
        let theme_path = files
            .keys()
            .filter(|key| key.rsplit('/').next() == Some("theme.xml"))
            .min_by_key(|key| (key.matches('/').count(), (*key).clone()))
            .cloned()
            .ok_or_else(|| SkinError::MissingFile("theme.xml".to_owned()))?;
        let base = theme_path
            .rsplit_once('/')
            .map_or_else(String::new, |(folder, _)| folder.to_owned());
        let text = &files[&theme_path];
        if text.len() > limits::TEXT_BYTES {
            return Err(limits::exceeded("more bytes of markup", limits::TEXT_BYTES));
        }
        let document = parse_within(&String::from_utf8_lossy(text), true)?;
        let theme = document
            .find("Theme")
            .ok_or_else(|| SkinError::MissingArea("Theme".to_owned()))?;
        let mut reader = Reader {
            files: &files,
            base,
            images: BTreeMap::new(),
            fonts: BTreeMap::new(),
            font_files: Vec::new(),
            warnings: Vec::new(),
            budget,
        };
        reader.fonts.insert(
            "defaultfont".to_owned(),
            Font {
                family: "Sans Serif".to_owned(),
                size: 12,
            },
        );
        for element in &theme.children {
            if element.is("Bitmap") {
                reader.bitmap(element)?;
            } else if element.is("Font") {
                reader.font(element);
            }
        }
        let mut windows = Vec::new();
        for element in theme.children.iter().filter(|child| child.is("Window")) {
            let attrs = &element.attrs;
            let id = attrs.get("id").unwrap_or("").to_owned();
            // A plain boolean: only "true" (the default) shows it.
            let visible = attrs.get("visible").is_none_or(|value| value == "true");
            let layouts = element
                .children
                .iter()
                .filter(|child| child.is("Layout"))
                .map(|layout| read_layout(layout, &reader))
                .collect::<Result<Vec<_>, _>>()?;
            if layouts.is_empty() {
                continue;
            }
            windows.push(Window {
                x: attrs.int("x"),
                y: attrs.int("y"),
                id,
                visible,
                layouts,
            });
        }
        Ok(Self {
            name: name.to_owned(),
            images: reader.images,
            fonts: reader.fonts,
            font_files: reader.font_files,
            windows,
            warnings: reader.warnings,
        })
    }

    /// A picture's size.
    #[must_use]
    pub fn size(&self, image: &str) -> Option<(i32, i32)> {
        self.images.get(image).map(|image| {
            (
                i32::try_from(image.width()).unwrap_or(0),
                i32::try_from(image.height()).unwrap_or(0),
            )
        })
    }
}

fn read_layout(element: &Element, reader: &Reader<'_>) -> Result<Layout, SkinError> {
    let attrs = &element.attrs;
    let width = attrs.int("width").unwrap_or(0).max(1);
    let height = attrs.int("height").unwrap_or(0).max(1);
    // -1 (the default): the size itself, not resizable that way.
    let limit = |name: &str, size: i32| attrs.int(name).filter(|value| *value >= 0).unwrap_or(size);
    let controls = read_controls(element, reader, (0, 0), (width, height))?;
    Ok(Layout {
        id: attrs.get("id").unwrap_or("").to_owned(),
        width,
        height,
        min: (limit("minwidth", width), limit("minheight", height)),
        max: (limit("maxwidth", width), limit("maxheight", height)),
        controls,
    })
}

/// An expression of the theme (`visible`, a checkbox's `state`), which
/// has at most [`limits::EXPRESSION_TOKENS`] tokens.
fn expression(text: Option<String>) -> Result<Option<String>, SkinError> {
    match text {
        Some(text) if expr::token_count(&text) > limits::EXPRESSION_TOKENS => Err(
            limits::exceeded("an expression of more tokens", limits::EXPRESSION_TOKENS),
        ),
        text => Ok(text),
    }
}

/// A slider's background: `nbhoriz` by `nbvert` pictures of its bitmap,
/// `padhoriz` and `padvert` pixels apart. `Ok(None)`: the theme gives
/// none, or one whose pictures do not lie in the bitmap.
fn slider_background(
    back: &Element,
    reader: &Reader<'_>,
) -> Result<Option<SliderBackground>, SkinError> {
    let Some(image) = back.attrs.get("image").map(unescape) else {
        return Ok(None);
    };
    let Some(bitmap) = reader.images.get(&image) else {
        return Ok(None);
    };
    let background = SliderBackground {
        image,
        columns: back.attrs.int_or("nbhoriz", 1).max(1),
        rows: back.attrs.int_or("nbvert", 1).max(1),
        pad_x: back.attrs.int_or("padhoriz", 0),
        pad_y: back.attrs.int_or("padvert", 0),
    };
    let cells = i64::from(background.columns) * i64::from(background.rows);
    if cells > i64::from(limits::SLIDER_CELLS) {
        return Err(limits::exceeded(
            &format!("a slider background of {cells} pictures"),
            limits::SLIDER_CELLS,
        ));
    }
    Ok(background
        .cell(bitmap.width(), bitmap.height())
        .map(|_| background))
}

impl SliderBackground {
    /// The grid in a bitmap of `width` x `height` pixels: the distance
    /// from one picture to the next and a picture's size, each across and
    /// down. `None` when the pictures would not lie in the bitmap (pads
    /// below zero, or more pictures than pixels).
    #[must_use]
    pub fn cell(&self, width: u32, height: u32) -> Option<((i32, i32), (i32, i32))> {
        let axis = |whole: u32, pad: i32, count: i32| {
            let whole = i64::from(whole);
            let (pad, count) = (i64::from(pad), i64::from(count));
            let step = (whole + pad) / count.max(1);
            let size = step - pad;
            let inside = pad >= 0 && size >= 1 && (count - 1) * step + size <= whole;
            if !inside {
                return None;
            }
            Some((i32::try_from(step).ok()?, i32::try_from(size).ok()?))
        };
        let (step_x, width) = axis(width, self.pad_x, self.columns)?;
        let (step_y, height) = axis(height, self.pad_y, self.rows)?;
        Some(((step_x, step_y), (width, height)))
    }
}

/// The controls of a box: `offset` from `Group`s, the box's size `size`.
// One arm per VLC element, each reading its attributes as VLC's builder
// does: splitting it would scatter the element list.
#[allow(clippy::too_many_lines)]
fn read_controls(
    element: &Element,
    reader: &Reader<'_>,
    offset: (i32, i32),
    size: (i32, i32),
) -> Result<Vec<Control>, SkinError> {
    let mut out = Vec::new();
    for child in &element.children {
        let attrs = &child.attrs;
        let get = |name: &str| attrs.get(name).map(unescape);
        let place = |w: i32, h: i32| Place {
            x: offset
                .0
                .saturating_add(dimension(attrs.get("x"), size.0, 0)),
            y: offset
                .1
                .saturating_add(dimension(attrs.get("y"), size.1, 0)),
            w,
            h,
            lefttop: Corner::read(attrs.get("lefttop")),
            rightbottom: Corner::read(attrs.get("rightbottom")),
            keep_x: attrs.get("xkeepratio") == Some("true"),
            keep_y: attrs.get("ykeepratio") == Some("true"),
            box_w: size.0,
            box_h: size.1,
        };
        let visible = expression(get("visible"))?;
        let image_size = |id: Option<String>| {
            id.and_then(|id| {
                reader
                    .images
                    .get(&id)
                    .map(|image| (image.width(), image.height()))
            })
        };
        let known = |id: &str| reader.images.contains_key(id);
        match child.name.to_ascii_lowercase().as_str() {
            "group" => {
                let x = attrs.int_or("x", 0);
                let y = attrs.int_or("y", 0);
                out.extend(read_controls(
                    child,
                    reader,
                    (offset.0.saturating_add(x), offset.1.saturating_add(y)),
                    size,
                )?);
            }
            "panel" => {
                let w = dimension(attrs.get("width"), size.0, 0);
                let h = dimension(attrs.get("height"), size.1, 0);
                let children = read_controls(child, reader, (0, 0), (w, h))?;
                out.push(Control::Panel {
                    place: place(w, h),
                    children,
                });
            }
            "image" => {
                let Some(image) = get("image").filter(|id| known(id)) else {
                    continue;
                };
                let (bw, bh) = image_size(Some(image.clone())).unwrap_or((0, 0));
                let w = attrs
                    .int("width")
                    .filter(|w| *w > 0)
                    .unwrap_or(i32::try_from(bw).unwrap_or(0));
                let h = attrs
                    .int("height")
                    .filter(|h| *h > 0)
                    .unwrap_or(i32::try_from(bh).unwrap_or(0));
                out.push(Control::Image {
                    place: place(w, h),
                    image,
                    resize: get("resize").unwrap_or_else(|| "mosaic".to_owned()),
                    action: get("action").unwrap_or_default(),
                    double: get("action2").unwrap_or_default(),
                    visible,
                });
            }
            "button" => {
                let up = get("up").unwrap_or_default();
                let down = get("down").unwrap_or_else(|| up.clone());
                let over = get("over").unwrap_or_else(|| up.clone());
                if ![&up, &down, &over].iter().all(|id| known(id)) {
                    continue;
                }
                let (w, h) = image_size(Some(up.clone())).unwrap_or((0, 0));
                out.push(Control::Button {
                    place: place(i32::try_from(w).unwrap_or(0), i32::try_from(h).unwrap_or(0)),
                    up,
                    down,
                    over,
                    action: get("action").unwrap_or_default(),
                    tooltip: get("tooltiptext").unwrap_or_default(),
                    visible,
                });
            }
            "checkbox" => {
                let set = |n: u8| -> [String; 3] {
                    let up = get(&format!("up{n}")).unwrap_or_default();
                    let down = get(&format!("down{n}")).unwrap_or_else(|| up.clone());
                    let over = get(&format!("over{n}")).unwrap_or_else(|| up.clone());
                    [up, down, over]
                };
                let images = [set(1), set(2)];
                let Some(state) = expression(get("state"))? else {
                    continue;
                };
                if !images.iter().flatten().all(|id| known(id)) {
                    continue;
                }
                let (w, h) = image_size(Some(images[0][0].clone())).unwrap_or((0, 0));
                out.push(Control::Checkbox {
                    place: place(i32::try_from(w).unwrap_or(0), i32::try_from(h).unwrap_or(0)),
                    images,
                    actions: [
                        get("action1").unwrap_or_default(),
                        get("action2").unwrap_or_default(),
                    ],
                    tooltips: [
                        get("tooltiptext1").unwrap_or_default(),
                        get("tooltiptext2").unwrap_or_default(),
                    ],
                    state,
                    visible,
                });
            }
            "text" => {
                let font = get("font").unwrap_or_else(|| "defaultfont".to_owned());
                let Some(size_px) = reader.fonts.get(&font).map(|font| font.size) else {
                    continue;
                };
                let w = attrs.int("width").unwrap_or(0);
                // The font's line height.
                let h = size_px.saturating_add(size_px / 4).saturating_add(1);
                out.push(Control::Text {
                    place: place(w, h),
                    font,
                    text: get("text").unwrap_or_default(),
                    color: color(attrs.get("color"), [0, 0, 0]),
                    alignment: get("alignment").unwrap_or_else(|| "left".to_owned()),
                    visible,
                });
            }
            "slider" => {
                let up = get("up").unwrap_or_default();
                let down = get("down").unwrap_or_else(|| up.clone());
                if !known(&up) || !known(&down) {
                    continue;
                }
                let points = parse_points(attrs.get("points").unwrap_or(""));
                if points.len() < 2 {
                    continue;
                }
                let background = match child.child("SliderBackground") {
                    Some(back) => slider_background(back, reader)?,
                    None => None,
                };
                let max_x = points
                    .iter()
                    .map(|p| p.0)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                let max_y = points
                    .iter()
                    .map(|p| p.1)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                out.push(Control::Slider {
                    place: place(
                        attrs.int("width").filter(|w| *w > 0).unwrap_or(max_x),
                        attrs.int("height").filter(|h| *h > 0).unwrap_or(max_y),
                    ),
                    up,
                    down,
                    points,
                    value: get("value").unwrap_or_default(),
                    tooltip: get("tooltiptext").unwrap_or_default(),
                    background,
                    visible,
                });
            }
            "playtree" | "playlist" => {
                let w = dimension(attrs.get("width"), size.0, 0);
                let h = dimension(attrs.get("height"), size.1, 0);
                out.push(Control::Playtree {
                    place: place(w, h),
                    background: get("bgimage").filter(|id| known(id)),
                    fg: color(attrs.get("fgcolor"), [0, 0, 0]),
                    play: color(attrs.get("playcolor"), [255, 0, 0]),
                    bg: color(attrs.get("bgcolor1"), [255, 255, 255]),
                    selection: color(attrs.get("selcolor"), [0, 0, 255]),
                    visible,
                });
            }
            "video" => {
                let w = dimension(attrs.get("width"), size.0, 0);
                let h = dimension(attrs.get("height"), size.1, 0);
                if w > 0 && h > 0 {
                    out.push(Control::Video {
                        place: place(w, h),
                        visible,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// `(x,y),(x,y),...`.
fn parse_points(text: &str) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for part in text.split(')') {
        let part = part.trim_start_matches([',', ' ']).trim_start_matches('(');
        if let Some((x, y)) = part.split_once(',')
            && let (Ok(x), Ok(y)) = (x.trim().parse(), y.trim().parse())
        {
            out.push((x, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    #[test]
    fn entities_and_points() {
        assert_eq!(unescape("a &amp; b &#65;&#x42;"), "a & b AB");
        assert_eq!(parse_points("(0,11),(350,11)"), vec![(0, 11), (350, 11)]);
        assert_eq!(dimension(Some("50%"), 300, 0), 150);
        assert_eq!(dimension(Some("12px"), 300, 0), 12);
        assert_eq!(color(Some("#1A2B3C"), [0; 3]), [0x1A, 0x2B, 0x3C]);
    }

    #[test]
    fn a_tar_archive_reads() {
        let mut tar = vec![0_u8; 512];
        tar[..9].copy_from_slice(b"theme.xml");
        let body = b"<Theme version='2.0'></Theme>";
        let size = format!("{:011o}\0", body.len());
        tar[124..136].copy_from_slice(size.as_bytes());
        tar[156] = b'0';
        tar.extend_from_slice(body);
        tar.resize(tar.len().div_ceil(512) * 512 + 1024, 0);
        let members = members(&tar).unwrap();
        assert_eq!(members, vec![("theme.xml".to_owned(), body.to_vec())]);
    }

    /// A theme of `xml` (inside `<Theme>`) and its files; every picture
    /// named `N.png` is one of `N` x `N` pixels.
    pub(crate) fn theme_members(xml: &str, files: &[&str]) -> Vec<(String, Vec<u8>)> {
        let mut members = vec![(
            "theme.xml".to_owned(),
            format!("<Theme version='2.0'>{xml}</Theme>").into_bytes(),
        )];
        for file in files {
            let side: u32 = file.trim_end_matches(".png").parse().unwrap();
            let mut picture = Image::new(side, side);
            picture.fill([10, 20, 30, 255]);
            members.push(((*file).to_owned(), picture.encode_png().unwrap()));
        }
        members
    }

    fn limit(result: Result<Theme, SkinError>) -> String {
        match result {
            Err(SkinError::Limit(what)) => what,
            Err(other) => panic!("not a limit: {other}"),
            Ok(theme) => panic!("loaded with {} pictures", theme.images.len()),
        }
    }

    /// A window with one slider whose background is `background`.
    fn slider_theme(background: &str) -> Result<Theme, SkinError> {
        let xml = format!(
            "<Bitmap id='back' file='64.png'/><Bitmap id='thumb' file='8.png'/>\
             <Window id='main'><Layout id='l' width='100' height='50'>\
             <Slider up='thumb' points='(0,4),(60,4)' value='volume'>\
             <SliderBackground image='back' {background}/></Slider></Layout></Window>"
        );
        Theme::from_members("test", theme_members(&xml, &["64.png", "8.png"]))
    }

    fn background(theme: &Theme) -> Option<&SliderBackground> {
        match &theme.windows[0].layouts[0].controls[0] {
            Control::Slider { background, .. } => background.as_ref(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_slider_background_is_a_grid_within_its_bitmap() {
        // Ten thousand million pictures to cut, from one line of a theme.
        let bomb = slider_theme("nbhoriz='100000' nbvert='100000'");
        assert!(limit(bomb).contains("slider background"));
        // As many as a background may have: one pixel each.
        let most = slider_theme("nbhoriz='64' nbvert='64'").unwrap();
        assert_eq!(
            background(&most).unwrap().cell(64, 64),
            Some(((1, 1), (1, 1)))
        );
        let over = slider_theme("nbhoriz='65' nbvert='64'");
        assert!(limit(over).contains("4160"));
        // Pictures that would not lie in the bitmap: a pad below zero
        // makes each a million pixels wide; more of them than pixels have
        // no size. The slider keeps its thumb and has no background.
        let wide = slider_theme("nbhoriz='2' padhoriz='-1000000'").unwrap();
        assert!(background(&wide).is_none());
        let none = slider_theme("nbhoriz='100' nbvert='1'").unwrap();
        assert!(background(&none).is_none());
        // VLC's own grids: 31 by 7 pictures, and pads between them.
        let grid = SliderBackground {
            image: String::new(),
            columns: 31,
            rows: 7,
            pad_x: 0,
            pad_y: 0,
        };
        assert_eq!(grid.cell(310, 70), Some(((10, 10), (10, 10))));
        let padded = SliderBackground {
            columns: 3,
            rows: 1,
            pad_x: 2,
            ..grid
        };
        assert_eq!(padded.cell(28, 5), Some(((10, 5), (8, 5))));
    }

    #[test]
    fn an_expression_of_a_theme_is_bounded() {
        let tokens = |count: usize| "true".to_owned() + &" and true".repeat((count - 1) / 2);
        let image = |visible: &str| {
            let xml = format!(
                "<Bitmap id='b' file='8.png'/><Window id='main'>\
                 <Layout id='l' width='100' height='50'>\
                 <Image image='b' visible='{visible}'/></Layout></Window>"
            );
            Theme::from_members("test", theme_members(&xml, &["8.png"]))
        };
        assert_eq!(
            expr::token_count(&tokens(limits::EXPRESSION_TOKENS + 1)),
            129
        );
        assert!(image(&tokens(limits::EXPRESSION_TOKENS - 1)).is_ok());
        assert!(limit(image(&tokens(limits::EXPRESSION_TOKENS + 1))).contains("expression"));
        let checkbox = |state: &str| {
            let xml = format!(
                "<Bitmap id='b' file='8.png'/><Window id='main'>\
                 <Layout id='l' width='100' height='50'>\
                 <Checkbox up1='b' up2='b' state='{state}'/></Layout></Window>"
            );
            Theme::from_members("test", theme_members(&xml, &["8.png"]))
        };
        assert!(checkbox("vlc.isMute").is_ok());
        assert!(limit(checkbox(&"not ".repeat(100_000))).contains("expression"));
    }

    #[test]
    fn the_markup_of_a_theme_is_bounded() {
        let padding = " ".repeat(limits::TEXT_BYTES);
        let large = Theme::from_members("test", theme_members(&padding, &[]));
        assert!(limit(large).contains("bytes of markup"));
        let nested = "<Panel>".repeat(1000);
        let xml = format!("<Window id='main'><Layout id='l'>{nested}</Layout></Window>");
        let deep = Theme::from_members("test", theme_members(&xml, &[]));
        assert!(limit(deep).contains("deeper"));
    }

    #[test]
    fn the_pictures_of_a_theme_share_its_budget() {
        let load = |xml: &str, pixels: u64| {
            Theme::from_members_within(
                "test",
                theme_members(xml, &["8.png"]),
                limits::Pixels::with(pixels),
            )
        };
        // Every `Bitmap` is a picture of its own, whatever file it names.
        let three = "<Bitmap id='a' file='8.png'/><Bitmap id='b' file='8.png'/>\
                     <Bitmap id='c' file='8.png'/>";
        assert_eq!(load(three, 3 * 64).unwrap().images.len(), 3);
        assert!(limit(load(three, 3 * 64 - 1)).contains("pixels together"));
        // So is each part of one.
        let parts = "<Bitmap id='a' file='8.png'><SubBitmap id='b' x='0' y='0' width='4' \
                     height='4'/><SubBitmap id='c' x='4' y='4' width='4' height='2'/></Bitmap>";
        assert_eq!(load(parts, 64 + 16 + 8).unwrap().images.len(), 3);
        assert!(limit(load(parts, 64 + 16 + 8 - 1)).contains("pixels together"));
        // An animation keeps its first frame alone.
        let frames = "<Bitmap id='a' file='8.png' nbframes='2'/><Bitmap id='b' file='8.png'/>";
        let theme = load(frames, 64 + 32).unwrap();
        assert_eq!(theme.size("a"), Some((8, 4)));
        assert_eq!(theme.size("b"), Some((8, 8)));
        assert!(limit(load(frames, 64 + 32 - 1)).contains("pixels together"));
    }

    /// The smallest font file that has a family name: a table directory
    /// with a `name` table of one Windows name.
    fn font_file(family: &str) -> Vec<u8> {
        let name: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let mut table = vec![0, 0, 0, 1, 0, 18];
        // Windows, Unicode, US English, the family; the string's length
        // and offset.
        table.extend_from_slice(&[0, 3, 0, 1, 4, 9, 0, 1]);
        table.extend_from_slice(&u16::try_from(name.len()).unwrap().to_be_bytes());
        table.extend_from_slice(&[0, 0]);
        table.extend_from_slice(&name);
        let mut file = vec![0, 1, 0, 0, 0, 1, 0, 16, 0, 0, 0, 0];
        file.extend_from_slice(b"name");
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&28_u32.to_be_bytes());
        file.extend_from_slice(&u32::try_from(table.len()).unwrap().to_be_bytes());
        file.extend_from_slice(&table);
        file
    }

    #[test]
    fn a_font_file_is_kept_once_however_many_fonts_name_it() {
        let mut fonts = String::new();
        for n in 0..100 {
            let size = 8 + n % 4;
            let _ = write!(fonts, "<Font id='f{n}' file='font.ttf' size='{size}'/>");
        }
        let mut members = theme_members(&fonts, &[]);
        members.push(("font.ttf".to_owned(), font_file("Test Sans")));
        let theme = Theme::from_members("test", members).unwrap();
        assert!(theme.warnings.is_empty(), "{:?}", theme.warnings);
        // The default font and the hundred of the theme.
        assert_eq!(theme.fonts.len(), 101);
        assert_eq!(theme.fonts["f5"].family, "Test Sans");
        assert_eq!(theme.fonts["f5"].size, 9);
        assert_eq!(theme.font_files.len(), 1);
    }
}
