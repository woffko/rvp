//! "RVP Default", RVP's own skin (milestone M7), built from `art/skin/`.
//!
//! The build draws every sprite of `sprites.xml` ([`crate::sprites`]) into
//! its atlas, packs each atlas in rows (tallest sprites first), fills the
//! placeholders of the markup files with the sprites' places, and writes a
//! `.dsf` archive (ZIP) whose bytes depend only on the sources: fixed
//! file order, fixed dates, no compression of the images.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::{Cursor, Write as _};

use rvp_skin::Image;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::font::Font;
use crate::sprites::{self, Atlas, Painter};
use crate::svg::Document;

/// The skin's name, also the archive's file name without `.dsf`.
pub const NAME: &str = "RVP Default";

/// The markup files, in archive order; `VideoSkin.xml` includes the
/// others.
pub const MARKUP: [(&str, &str); 3] = [
    ("VideoSkin.xml", include_str!("../art/skin/VideoSkin.xml")),
    ("TitleBar.txt", include_str!("../art/skin/TitleBar.txt")),
    ("ControlBar.txt", include_str!("../art/skin/ControlBar.txt")),
];

/// A short notice packed with the skin.
const NOTICE: &str = "RVP Default, the default skin of RVP (Rust Video Player).\r\n\
\r\n\
Drawn for RVP and built from its sources (crates/look/art/skin, with the\r\n\
list of every asset in SOURCES.md). It is part of RVP and has RVP's\r\n\
license, the GNU General Public License version 3 or later.\r\n";

/// Space between packed sprites, in pixels.
const GAP: u32 = 2;

/// Where a sprite is in its atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub x: u32,
    pub y: u32,
    /// One cell or button state.
    pub cell_width: u32,
    pub cell_height: u32,
    /// All cells.
    pub width: u32,
}

/// The files of the skin and the sprites' places.
#[derive(Debug, Clone)]
pub struct Built {
    /// `(name, bytes)` in archive order.
    pub files: Vec<(String, Vec<u8>)>,
    pub places: BTreeMap<String, Place>,
    /// The atlases as drawn, for tests and reviews.
    pub atlases: Vec<(String, Image)>,
}

/// Builds the skin's files.
///
/// # Errors
/// Reports problems in the sources: unknown sprites or placeholders,
/// sprites that do not fit or are never used, art outside the SVG subset.
pub fn build() -> Result<Built, String> {
    let font = Font::load()?;
    let (atlases, palette) = sprites::load(&font)?;
    let defs = sprites::defs(&font)?;
    let painter = Painter {
        palette: &palette,
        font: &font,
    };
    let mut places = BTreeMap::new();
    let mut drawn = Vec::with_capacity(atlases.len());
    for atlas in &atlases {
        let (atlas_places, height) = pack(atlas)?;
        let image = draw(atlas, &atlas_places, height, &defs, &painter)?;
        places.extend(atlas_places);
        drawn.push((atlas.file.clone(), image));
    }
    let mut files = Vec::with_capacity(MARKUP.len() + drawn.len() + 1);
    let mut used = BTreeSet::new();
    for (name, text) in MARKUP {
        let filled = fill(name, text, &places, &mut used)?;
        files.push((name.to_owned(), filled.into_bytes()));
    }
    for atlas in &atlases {
        for sprite in &atlas.sprites {
            if atlas.height.is_none() && !used.contains(&sprite.name) {
                return Err(format!("sprite {} is not used by the markup", sprite.name));
            }
        }
    }
    for (file, image) in &drawn {
        let bytes = if has_extension(file, "bmp") {
            bmp(image)
        } else {
            image.encode_png().map_err(|error| error.to_string())?
        };
        files.push((file.clone(), bytes));
    }
    files.push((format!("{NAME}.txt"), NOTICE.as_bytes().to_vec()));
    Ok(Built {
        files,
        places,
        atlases: drawn,
    })
}

/// The `.dsf` archive of a built skin.
///
/// # Errors
/// Fails only if the ZIP writer does.
pub fn archive(built: &Built) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in &built.files {
        let text = !is_image(name);
        let options = SimpleFileOptions::default()
            .compression_method(if text {
                CompressionMethod::Deflated
            } else {
                CompressionMethod::Stored
            })
            .last_modified_time(DateTime::default())
            .unix_permissions(0o644);
        writer
            .start_file(name.as_str(), options)
            .map_err(|error| error.to_string())?;
        writer.write_all(bytes).map_err(|error| error.to_string())?;
    }
    let cursor = writer.finish().map_err(|error| error.to_string())?;
    Ok(cursor.into_inner())
}

/// Builds the skin and returns its `.dsf` archive.
///
/// # Errors
/// See [`build`].
pub fn dsf() -> Result<Vec<u8>, String> {
    archive(&build()?)
}

/// Places the sprites of one atlas: rows left to right, the tallest
/// sprites first (document order among equals), or the fixed places of
/// atlases like `ScrollBar.bmp`. Returns the places and the atlas height.
fn pack(atlas: &Atlas) -> Result<(BTreeMap<String, Place>, u32), String> {
    let mut places = BTreeMap::new();
    let place = |sprite: &sprites::Sprite, x: u32, y: u32| Place {
        x,
        y,
        cell_width: sprite.cell_width,
        cell_height: sprite.cell_height,
        width: sprite.width(),
    };
    if let Some(height) = atlas.height {
        for sprite in &atlas.sprites {
            let (x, y) = sprite
                .fixed
                .ok_or_else(|| format!("sprite {} needs x and y", sprite.name))?;
            if x + sprite.width() > atlas.width || y + sprite.cell_height > height {
                return Err(format!(
                    "sprite {} does not fit {}",
                    sprite.name, atlas.file
                ));
            }
            places.insert(sprite.name.clone(), place(sprite, x, y));
        }
        return Ok((places, height));
    }
    let mut order: Vec<&sprites::Sprite> = atlas.sprites.iter().collect();
    order.sort_by_key(|sprite| std::cmp::Reverse(sprite.cell_height));
    let (mut x, mut y, mut row) = (GAP, GAP, 0);
    for sprite in order {
        let width = sprite.width();
        if width + 2 * GAP > atlas.width {
            return Err(format!(
                "sprite {} is wider than {}",
                sprite.name, atlas.file
            ));
        }
        if x + width + GAP > atlas.width {
            x = GAP;
            y += row + GAP;
            row = 0;
        }
        places.insert(sprite.name.clone(), place(sprite, x, y));
        x += width + GAP;
        row = row.max(sprite.cell_height);
    }
    let height = (y + row + GAP).next_multiple_of(4);
    Ok((places, height))
}

/// Draws one atlas: its sprites as one SVG document, rendered at 1:1.
fn draw(
    atlas: &Atlas,
    places: &BTreeMap<String, Place>,
    height: u32,
    defs: &str,
    painter: &Painter<'_>,
) -> Result<Image, String> {
    let width = atlas.width;
    let mut text = String::with_capacity(64 * 1024);
    let _ = write!(
        text,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\"><defs>{defs}</defs>"
    );
    if has_extension(&atlas.file, "bmp") {
        // Magenta marks unused pixels in PotPlayer's bitmaps.
        let _ = write!(
            text,
            "<rect width=\"{width}\" height=\"{height}\" fill=\"#FF00FF\"/>"
        );
    }
    for sprite in &atlas.sprites {
        let place = places[&sprite.name];
        painter.draw(sprite, place.x, place.y, &mut text)?;
    }
    text.push_str("</svg>");
    let document = Document::parse(&text).map_err(|error| format!("{}: {error}", atlas.file))?;
    document
        .render(width, height)
        .map_err(|error| format!("{}: {error}", atlas.file))
}

/// Replaces `{name.x}` style placeholders with the sprites' numbers and
/// records the sprites used.
fn fill(
    file: &str,
    text: &str,
    places: &BTreeMap<String, Place>,
    used: &mut BTreeSet<String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let line = text.len() - rest.len() + open;
        let at = || format!("{file} line {}", text[..line].matches('\n').count() + 1);
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| format!("{}: an unclosed brace", at()))?;
        let key = &after[..close];
        let (name, property) = key
            .rsplit_once('.')
            .ok_or_else(|| format!("{}: {{{key}}} is not NAME.PROPERTY", at()))?;
        let place = places
            .get(name)
            .ok_or_else(|| format!("{}: no sprite {name}", at()))?;
        let value = match property {
            "x" => place.x,
            "y" => place.y,
            "w" => place.cell_width,
            "h" => place.cell_height,
            "tw" => place.width,
            other => return Err(format!("{}: unknown property {other}", at())),
        };
        let _ = write!(out, "{value}");
        used.insert(name.to_owned());
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn has_extension(name: &str, extension: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

fn is_image(name: &str) -> bool {
    has_extension(name, "png") || has_extension(name, "bmp")
}

/// A 24-bit bottom-up Windows bitmap; pixels less than half opaque become
/// magenta, which PotPlayer's skins use for transparency.
fn bmp(image: &Image) -> Vec<u8> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let stride = (width * 3).next_multiple_of(4);
    let size = 54 + stride * height;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&u32::try_from(size).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&54_u32.to_le_bytes());
    out.extend_from_slice(&40_u32.to_le_bytes());
    out.extend_from_slice(&image.width().to_le_bytes());
    out.extend_from_slice(&image.height().to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&24_u16.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(
        &u32::try_from(stride * height)
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    out.extend_from_slice(&2835_u32.to_le_bytes());
    out.extend_from_slice(&2835_u32.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    for row in image.data().chunks_exact((width * 4).max(1)).rev() {
        let start = out.len();
        for pixel in row.as_chunks::<4>().0 {
            if pixel[3] < 128 {
                out.extend_from_slice(&[255, 0, 255]);
            } else {
                out.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
            }
        }
        out.resize(start + stride, 0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Place, fill, pack};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn placeholders_take_the_sprites_numbers() {
        let mut places = BTreeMap::new();
        places.insert(
            "play".to_owned(),
            Place {
                x: 4,
                y: 6,
                cell_width: 48,
                cell_height: 40,
                width: 192,
            },
        );
        let mut used = BTreeSet::new();
        let text = "<Button ImageLeft='{play.x}' ImageTop='{play.y}' W='{play.w}' T='{play.tw}' />";
        let filled = fill("t", text, &places, &mut used).expect("fill");
        assert_eq!(
            filled,
            "<Button ImageLeft='4' ImageTop='6' W='48' T='192' />"
        );
        assert!(used.contains("play"));
        assert!(fill("t", "{nothing.x}", &places, &mut used).is_err());
        assert!(fill("t", "{play.z}", &places, &mut used).is_err());
    }

    #[test]
    fn packing_keeps_sprites_apart_and_inside() {
        let font = crate::font::Font::load().expect("font");
        let (atlases, _) = crate::sprites::load(&font).expect("sprites");
        for atlas in &atlases {
            let (places, height) = pack(atlas).expect("pack");
            let rects: Vec<(u32, u32, u32, u32)> = places
                .values()
                .map(|place| (place.x, place.y, place.width, place.cell_height))
                .collect();
            for (index, a) in rects.iter().enumerate() {
                assert!(
                    a.0 + a.2 <= atlas.width && a.1 + a.3 <= height,
                    "{}",
                    atlas.file
                );
                for b in &rects[index + 1..] {
                    let apart = a.0 + a.2 <= b.0
                        || b.0 + b.2 <= a.0
                        || a.1 + a.3 <= b.1
                        || b.1 + b.3 <= a.1;
                    assert!(apart, "{}: {a:?} and {b:?} overlap", atlas.file);
                }
            }
        }
    }
}
