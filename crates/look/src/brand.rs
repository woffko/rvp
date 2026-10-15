//! RVP's logo as the program uses it (decision D10): the application icon
//! in PNG sizes, an ICO file and a Windows resource; the symbol for the
//! idle screen; the wordmark for the About box; and the logo RVP draws in
//! place of a loaded skin's title-bar logo.

use rvp_skin::{Image, Skin};

use crate::svg::Document;

/// A play triangle cut into three facets.
pub const SYMBOL_SVG: &str = include_str!("../art/logo/symbol.svg");
/// The symbol on a graphite tile: the application icon.
pub const ICON_SVG: &str = include_str!("../art/logo/icon.svg");
/// The symbol followed by the letters R, V and P.
pub const WORDMARK_SVG: &str = include_str!("../art/logo/wordmark.svg");

/// Letter colour of the wordmark on dark backgrounds (as drawn in the
/// source) and on light ones.
pub const WORDMARK_LIGHT: &str = "#ECE7E1";
pub const WORDMARK_DARK: &str = "#24272C";

/// Sizes in `rvp.ico`: what Windows asks for at 100 % to 250 % scaling.
pub const ICO_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// Renders an SVG source into a `width` x `height` image, fitted and
/// centred.
///
/// # Errors
/// Fails when the source is outside the SVG subset.
pub fn render(svg: &str, width: u32, height: u32) -> Result<Image, String> {
    Document::parse(svg)?.render(width, height)
}

/// The symbol `height` pixels high, as wide as its proportions ask.
///
/// # Errors
/// Fails when the source is outside the SVG subset.
pub fn symbol(height: u32) -> Result<Image, String> {
    let document = Document::parse(SYMBOL_SVG)?;
    let [_, _, width, box_height] = document.view_box()?;
    let pixels_wide = (width * pixel_count(height) / box_height).round();
    document.render(from_f32(pixels_wide), height)
}

/// The application icon at `size` x `size` pixels.
///
/// # Errors
/// Fails when the source is outside the SVG subset.
pub fn icon(size: u32) -> Result<Image, String> {
    render(ICON_SVG, size, size)
}

/// The wordmark `height` pixels high with letters in `color` (`#rrggbb`).
///
/// # Errors
/// Fails when the source is outside the SVG subset.
pub fn wordmark(height: u32, color: &str) -> Result<Image, String> {
    let source = WORDMARK_SVG.replacen(
        &format!("color=\"{WORDMARK_LIGHT}\""),
        &format!("color=\"{color}\""),
        1,
    );
    let document = Document::parse(&source)?;
    let [_, _, width, box_height] = document.view_box()?;
    let pixels_wide = (width * pixel_count(height) / box_height).ceil();
    document.render(from_f32(pixels_wide), height)
}

/// An ICO file holding the icon at [`ICO_SIZES`]: 32-bit bitmaps below
/// 256 pixels (every Windows version reads them), PNG at 256.
///
/// # Errors
/// Fails when an image cannot be drawn or encoded.
pub fn ico() -> Result<Vec<u8>, String> {
    let mut entries: Vec<(u32, Vec<u8>)> = Vec::with_capacity(ICO_SIZES.len());
    for size in ICO_SIZES {
        let image = icon(size)?;
        let data = if size >= 256 {
            image.encode_png().map_err(|error| error.to_string())?
        } else {
            dib(&image)
        };
        entries.push((size, data));
    }
    let count = u16::try_from(entries.len()).map_err(|_| "too many icon sizes")?;
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * u32::from(count);
    for (size, data) in &entries {
        let length = u32::try_from(data.len()).map_err(|_| "icon image too large")?;
        out.extend_from_slice(&directory_entry(*size, length));
        out.extend_from_slice(&offset.to_le_bytes());
        offset += length;
    }
    for (_, data) in &entries {
        out.extend_from_slice(data);
    }
    Ok(out)
}

/// The first 12 bytes of an icon directory entry: size, colours, planes,
/// bits per pixel and the image's length.
fn directory_entry(size: u32, length: u32) -> [u8; 12] {
    // 256 is written as 0.
    let byte = u8::try_from(size).unwrap_or(0);
    let mut entry = [0; 12];
    entry[0] = byte;
    entry[1] = byte;
    entry[4..6].copy_from_slice(&1_u16.to_le_bytes());
    entry[6..8].copy_from_slice(&32_u16.to_le_bytes());
    entry[8..12].copy_from_slice(&length.to_le_bytes());
    entry
}

/// A 32-bit icon bitmap: a `BITMAPINFOHEADER` of twice the height, the
/// BGRA rows bottom-up and an all-zero AND mask (the alpha channel
/// decides).
fn dib(image: &Image) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let mask_row = (width.div_ceil(32) * 4) as usize;
    let rows = height as usize;
    let mut out = Vec::with_capacity(40 + image.data().len() + mask_row * rows);
    out.extend_from_slice(&40_u32.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&(height * 2).to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&32_u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    let stride = width as usize * 4;
    for row in image.data().chunks_exact(stride.max(1)).rev() {
        for pixel in row.as_chunks::<4>().0 {
            out.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    out.resize(out.len() + mask_row * rows, 0);
    out
}

/// A Windows resource file (`.res`, linked into `rvp.exe`) with the icon
/// of [`ico`] as the group `IDI_ICON1`, the name Qt's Windows platform
/// looks up for the window class icon.
///
/// # Errors
/// Fails when `ico` is not an ICO file this module wrote.
pub fn res(ico: &[u8]) -> Result<Vec<u8>, String> {
    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;
    let count = usize::from(u16::from_le_bytes(
        ico.get(4..6)
            .ok_or("short ICO")?
            .try_into()
            .map_err(|_| "short ICO")?,
    ));
    let mut out = Vec::new();
    // The empty first resource marks a 32-bit resource file.
    resource(&mut out, Id::Number(0), Id::Number(0), 0, &[]);
    let mut group = Vec::with_capacity(6 + 14 * count);
    group.extend_from_slice(&ico[..6]);
    for index in 0..count {
        let entry = ico
            .get(6 + 16 * index..6 + 16 * (index + 1))
            .ok_or("short ICO directory")?;
        let length = usize::try_from(u32::from_le_bytes(
            entry[8..12].try_into().map_err(|_| "bad entry")?,
        ))
        .map_err(|_| "bad length")?;
        let offset = usize::try_from(u32::from_le_bytes(
            entry[12..16].try_into().map_err(|_| "bad entry")?,
        ))
        .map_err(|_| "bad offset")?;
        let data = ico.get(offset..offset + length).ok_or("short ICO image")?;
        let id = u16::try_from(index + 1).map_err(|_| "too many icons")?;
        resource(&mut out, Id::Number(RT_ICON), Id::Number(id), 0x1010, data);
        group.extend_from_slice(&entry[..12]);
        group.extend_from_slice(&id.to_le_bytes());
    }
    resource(
        &mut out,
        Id::Number(RT_GROUP_ICON),
        Id::Name("IDI_ICON1"),
        0x1030,
        &group,
    );
    Ok(out)
}

#[derive(Clone, Copy)]
enum Id<'a> {
    Number(u16),
    Name(&'a str),
}

fn push_id(out: &mut Vec<u8>, id: &Id<'_>) {
    match id {
        Id::Number(number) => {
            out.extend_from_slice(&0xFFFF_u16.to_le_bytes());
            out.extend_from_slice(&number.to_le_bytes());
        }
        Id::Name(name) => {
            for unit in name.encode_utf16().chain(std::iter::once(0)) {
                out.extend_from_slice(&unit.to_le_bytes());
            }
        }
    }
}

/// Appends one resource: header (type, name, flags, neutral language)
/// and data, both padded to four bytes.
fn resource(out: &mut Vec<u8>, kind: Id<'_>, name: Id<'_>, flags: u16, data: &[u8]) {
    let mut header = Vec::new();
    push_id(&mut header, &kind);
    push_id(&mut header, &name);
    while !header.len().is_multiple_of(4) {
        header.push(0);
    }
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&flags.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    // Resource sizes are far below 4 GiB.
    let data_size = u32::try_from(data.len()).unwrap_or(u32::MAX);
    let header_size = u32::try_from(header.len() + 8).unwrap_or(u32::MAX);
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&header_size.to_le_bytes());
    out.extend_from_slice(&header);
    out.extend_from_slice(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// Puts RVP's symbol in place of the program logo in a loaded skin's
/// title bars (D10): PotPlayer's skins show their player's logo on a
/// `CMD_POPUPMENU_ETC` button there (Daum's, in BlackSpace). The button
/// keeps its place, command and tooltip; only its picture changes, and the
/// `.dsf` file is not touched. Buttons marked `RvpLogo='1'` (RVP Default's
/// own) keep their sprite.
///
/// # Errors
/// Fails only when the logo source cannot be drawn.
pub fn brand_skin(skin: &mut Skin) -> Result<(), String> {
    for slot in skin.logo_slots() {
        let strip = title_logo(slot.width, slot.height)?;
        skin.replace_sprite(slot.element, strip);
    }
    Ok(())
}

/// The symbol fitted into `width` x `height` (with a pixel of room), in
/// four button states side by side: normal, hover (lighter), pressed (one
/// pixel lower, darker) and disabled (grey, faded).
///
/// # Errors
/// Fails when the symbol cannot be drawn.
pub fn title_logo(width: i32, height: i32) -> Result<Image, String> {
    let width = u32::try_from(width.max(1)).unwrap_or(1);
    let height = u32::try_from(height.max(1)).unwrap_or(1);
    let symbol = render(SYMBOL_SVG, width, height.saturating_sub(2).max(1))?;
    let mut strip = Image::new(width * 4, height);
    for state in 0..4 {
        let shift = u32::from(state == 2);
        for y in 0..symbol.height() {
            for x in 0..symbol.width() {
                let [r, g, b, a] = symbol.pixel(to_i32(x), to_i32(y));
                if a == 0 {
                    continue;
                }
                let pixel = match state {
                    1 => [lighten(r), lighten(g), lighten(b), a],
                    2 => [darken(r), darken(g), darken(b), a],
                    3 => {
                        let grey =
                            u8::try_from((u32::from(r) * 3 + u32::from(g) * 6 + u32::from(b)) / 10)
                                .unwrap_or(u8::MAX);
                        [grey, grey, grey, a / 2]
                    }
                    _ => [r, g, b, a],
                };
                let target_y = y + 1 + shift;
                if target_y >= height {
                    continue;
                }
                if let Some(offset) = strip.offset(to_i32(state * width + x), to_i32(target_y)) {
                    strip.data_mut()[offset..offset + 4].copy_from_slice(&pixel);
                }
            }
        }
    }
    Ok(strip)
}

fn lighten(channel: u8) -> u8 {
    u8::try_from(u32::from(channel) + (255 - u32::from(channel)) / 4).unwrap_or(u8::MAX)
}

fn darken(channel: u8) -> u8 {
    u8::try_from(u32::from(channel) * 4 / 5).unwrap_or(u8::MAX)
}

fn to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

// Image sizes are far below 2^24: the conversions are exact.
#[allow(clippy::cast_precision_loss)]
fn pixel_count(value: u32) -> f32 {
    value as f32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn from_f32(value: f32) -> u32 {
    // Callers pass small positive sizes.
    value.max(1.0) as u32
}

#[cfg(test)]
mod tests {
    use super::{ICO_SIZES, ico, icon, res, title_logo, wordmark};

    #[test]
    fn the_icon_has_a_transparent_corner_and_an_orange_symbol() {
        let image = icon(64).expect("icon");
        assert_eq!(image.pixel(0, 0)[3], 0);
        let [r, g, b, a] = image.pixel(34, 32);
        assert!(
            a == 255 && r > 200 && g > 70 && b < 80,
            "{:?}",
            image.pixel(34, 32)
        );
    }

    #[test]
    fn the_ico_and_res_files_hold_every_size() {
        let file = ico().expect("ico");
        assert_eq!(&file[..4], &[0, 0, 1, 0]);
        assert_eq!(usize::from(file[4]), ICO_SIZES.len());
        // The 16-pixel bitmap: 40-byte header, 16x16 BGRA, 16 mask rows of 4 bytes.
        assert_eq!(
            u32::from_le_bytes(file[14..18].try_into().expect("len")),
            40 + 1024 + 64
        );
        let resources = res(&file).expect("res");
        assert_eq!(&resources[..8], &[0, 0, 0, 0, 32, 0, 0, 0]);
        let name: Vec<u8> = "IDI_ICON1"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(
            resources
                .windows(name.len())
                .any(|window| window == name.as_slice())
        );
    }

    #[test]
    fn the_wordmark_and_the_title_logo_have_their_sizes() {
        let word = wordmark(40, "#24272C").expect("wordmark");
        assert_eq!((word.width(), word.height()), (142, 40));
        let mark = super::symbol(144).expect("symbol");
        assert_eq!((mark.width(), mark.height()), (130, 144));
        let strip = title_logo(30, 16).expect("title logo");
        assert_eq!((strip.width(), strip.height()), (120, 16));
        // The pressed state sits one pixel lower than the normal one.
        let first_row =
            |state: i32| (0..16).find(|y| (0..30).any(|x| strip.pixel(state * 30 + x, *y)[3] > 0));
        assert_eq!(first_row(2), first_row(0).map(|row| row + 1));
    }
}
