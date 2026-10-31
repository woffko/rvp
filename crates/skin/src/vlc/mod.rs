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

pub mod build;
pub mod expr;

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::SkinError;
use crate::image::Image;
use crate::markup::{Element, parse_xml};

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

/// The members of an archive: zip, gzip'd tar or tar; `None` when it is
/// none of them.
fn members(bytes: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    if bytes.starts_with(b"PK") {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
        let mut out = Vec::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).ok()?;
            if file.is_dir() || file.size() > 32 << 20 {
                continue;
            }
            let name = file.name().ok()?.into_owned();
            let mut content = Vec::new();
            file.read_to_end(&mut content).ok()?;
            out.push((name, content));
        }
        return Some(out);
    }
    let tar = if bytes.starts_with(&[0x1F, 0x8B]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(256 << 20)
            .read_to_end(&mut out)
            .ok()?;
        out
    } else {
        bytes.to_vec()
    };
    read_tar(&tar)
}

/// A tar archive's regular files (ustar and GNU long names).
fn read_tar(bytes: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut long_name: Option<String> = None;
    while at + 512 <= bytes.len() {
        let header = &bytes[at..at + 512];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }
        let field = |from: usize, to: usize| {
            let raw = &header[from..to];
            let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
            String::from_utf8_lossy(&raw[..end]).into_owned()
        };
        let size = usize::from_str_radix(field(124, 136).trim(), 8).ok()?;
        let kind = header[156];
        let mut name = field(0, 100);
        if &header[257..262] == b"ustar" {
            let prefix = field(345, 500);
            if !prefix.is_empty() {
                name = format!("{prefix}/{name}");
            }
        }
        let start = at + 512;
        let end = start.checked_add(size)?;
        let data = bytes.get(start..end)?;
        match kind {
            b'L' => {
                let raw = data.split(|byte| *byte == 0).next().unwrap_or(data);
                long_name = Some(String::from_utf8_lossy(raw).into_owned());
            }
            b'0' | 0 => {
                let name = long_name.take().unwrap_or(name);
                out.push((name, data.to_vec()));
            }
            _ => long_name = None,
        }
        at = start + size.div_ceil(512) * 512;
    }
    Some(out)
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
}

impl Reader<'_> {
    /// A file the theme names, relative to its folder.
    fn file(&self, name: &str) -> Option<&Vec<u8>> {
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

    fn bitmap(&mut self, element: &Element) {
        let attrs = &element.attrs;
        let (Some(id), Some(file)) = (attrs.get("id"), attrs.get("file")) else {
            return;
        };
        let file = unescape(file);
        let Some(bytes) = self.file(&file) else {
            self.warnings.push(format!("bitmap {id}: no file {file}"));
            return;
        };
        let mut image = match Image::decode(&file, bytes) {
            Ok(image) => image,
            Err(error) => {
                self.warnings.push(error.to_string());
                return;
            }
        };
        let key = color(attrs.get("alphacolor"), [255, 0, 255]);
        for pixel in image.data_mut().as_chunks_mut::<4>().0 {
            if pixel[..3] == key {
                pixel[3] = 0;
            }
        }
        let frames = attrs.int("nbframes").unwrap_or(1).max(1);
        let first = first_frame(&image, frames);
        self.images.insert(id.to_owned(), first);
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
            let crop = crate::compose::crop(&image, x, y, w, h);
            let frames = sub_attrs.int("nbframes").unwrap_or(1).max(1);
            self.images
                .insert(sub_id.to_owned(), first_frame(&crop, frames));
        }
    }

    fn font(&mut self, element: &Element) {
        let attrs = &element.attrs;
        let (Some(id), Some(file)) = (attrs.get("id"), attrs.get("file")) else {
            return;
        };
        let size = attrs.int("size").unwrap_or(12);
        let file = unescape(file);
        let Some(bytes) = self.file(&file).cloned() else {
            self.warnings.push(format!("font {id}: no file {file}"));
            return;
        };
        let Some(family) = font_family(&bytes) else {
            self.warnings
                .push(format!("font {id}: no family name in {file}"));
            return;
        };
        self.font_files.push((file, bytes));
        self.fonts.insert(id.to_owned(), Font { family, size });
    }
}

/// The first of `frames` pictures stacked from the top.
fn first_frame(image: &Image, frames: i32) -> Image {
    if frames <= 1 {
        return image.clone();
    }
    let height = i32::try_from(image.height()).unwrap_or(0) / frames;
    crate::compose::crop(
        image,
        0,
        0,
        i32::try_from(image.width()).unwrap_or(0),
        height.max(1),
    )
}

impl Theme {
    /// Opens a `.vlt` file.
    ///
    /// # Errors
    /// Fails when the file cannot be read, holds no archive, or has no
    /// `theme.xml`.
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let bytes = std::fs::read(path).map_err(|source| SkinError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let members = members(&bytes)
            .ok_or_else(|| SkinError::Archive("neither a zip nor a tar archive".to_owned()))?;
        Self::from_members(&name, members)
    }

    /// A theme from its archive's members.
    ///
    /// # Errors
    /// Fails without `theme.xml`.
    pub fn from_members(name: &str, members: Vec<(String, Vec<u8>)>) -> Result<Self, SkinError> {
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
        let text = String::from_utf8_lossy(&files[&theme_path]).into_owned();
        let document = parse_xml(&text);
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
                reader.bitmap(element);
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
                .collect::<Vec<_>>();
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

fn read_layout(element: &Element, reader: &Reader<'_>) -> Layout {
    let attrs = &element.attrs;
    let width = attrs.int("width").unwrap_or(0).max(1);
    let height = attrs.int("height").unwrap_or(0).max(1);
    // -1 (the default): the size itself, not resizable that way.
    let limit = |name: &str, size: i32| attrs.int(name).filter(|value| *value >= 0).unwrap_or(size);
    let controls = read_controls(element, reader, (0, 0), (width, height));
    Layout {
        id: attrs.get("id").unwrap_or("").to_owned(),
        width,
        height,
        min: (limit("minwidth", width), limit("minheight", height)),
        max: (limit("maxwidth", width), limit("maxheight", height)),
        controls,
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
) -> Vec<Control> {
    let mut out = Vec::new();
    for child in &element.children {
        let attrs = &child.attrs;
        let get = |name: &str| attrs.get(name).map(unescape);
        let place = |w: i32, h: i32| Place {
            x: offset.0 + dimension(attrs.get("x"), size.0, 0),
            y: offset.1 + dimension(attrs.get("y"), size.1, 0),
            w,
            h,
            lefttop: Corner::read(attrs.get("lefttop")),
            rightbottom: Corner::read(attrs.get("rightbottom")),
            keep_x: attrs.get("xkeepratio") == Some("true"),
            keep_y: attrs.get("ykeepratio") == Some("true"),
            box_w: size.0,
            box_h: size.1,
        };
        let visible = get("visible");
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
                    (offset.0 + x, offset.1 + y),
                    size,
                ));
            }
            "panel" => {
                let w = dimension(attrs.get("width"), size.0, 0);
                let h = dimension(attrs.get("height"), size.1, 0);
                let children = read_controls(child, reader, (0, 0), (w, h));
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
                let Some(state) = get("state") else {
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
                let h = size_px + size_px / 4 + 1;
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
                let background = child.child("SliderBackground").and_then(|back| {
                    let image = back.attrs.get("image").map(unescape)?;
                    known(&image).then(|| SliderBackground {
                        image,
                        columns: back.attrs.int_or("nbhoriz", 1).max(1),
                        rows: back.attrs.int_or("nbvert", 1).max(1),
                        pad_x: back.attrs.int_or("padhoriz", 0),
                        pad_y: back.attrs.int_or("padvert", 0),
                    })
                });
                let max_x = points.iter().map(|p| p.0).max().unwrap_or(0) + 1;
                let max_y = points.iter().map(|p| p.1).max().unwrap_or(0) + 1;
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
    out
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
        let members = read_tar(&tar).unwrap();
        assert_eq!(members, vec![("theme.xml".to_owned(), body.to_vec())]);
    }
}
