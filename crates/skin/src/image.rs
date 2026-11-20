//! RGBA images: decoding skin atlases (PNG, BMP) and encoding PNG output.
//!
//! A picture's size is read from its header and checked against
//! [`crate::limits`] before anything of that size is allocated: a file of
//! a few bytes can ask for any size. A skin's pictures are also counted
//! together ([`Image::decode_within`]).

use std::io::Cursor;

use crate::error::SkinError;
use crate::limits;

/// An 8-bit RGBA image with straight (not premultiplied) alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Image {
    /// A fully transparent image. Sizes are in pixels.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; pixel_count(width, height) * 4],
        }
    }

    /// Wraps RGBA bytes.
    ///
    /// # Errors
    /// Fails when `data` does not hold exactly `width * height * 4` bytes.
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Result<Self, SkinError> {
        if data.len() != pixel_count(width, height) * 4 {
            return Err(SkinError::Image {
                name: "raw".to_owned(),
                reason: format!("{} bytes for {width}x{height}", data.len()),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The RGBA bytes, row by row.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mutable RGBA bytes, row by row.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// The pixel at `(x, y)`; transparent black outside the image.
    #[must_use]
    pub fn pixel(&self, x: i32, y: i32) -> [u8; 4] {
        match self.offset(x, y) {
            Some(offset) => [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ],
            None => [0; 4],
        }
    }

    /// Byte offset of `(x, y)`, or `None` outside the image.
    #[must_use]
    pub fn offset(&self, x: i32, y: i32) -> Option<usize> {
        let x = u32::try_from(x).ok()?;
        let y = u32::try_from(y).ok()?;
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * 4)
    }

    /// Fills the whole image with one colour.
    pub fn fill(&mut self, rgba: [u8; 4]) {
        for pixel in self.data.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&rgba);
        }
    }

    /// The size a PNG or BMP file gives in its header, read without
    /// decoding it; `None` for other files.
    #[must_use]
    pub fn declared_size(bytes: &[u8]) -> Option<(u32, u32)> {
        if bytes.starts_with(b"\x89PNG") {
            let number =
                |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
            (bytes.get(12..16)? == b"IHDR").then_some(())?;
            Some((number(16)?, number(20)?))
        } else if bytes.starts_with(b"BM") {
            // Older headers keep their sizes elsewhere, and do not decode.
            (read_u32(bytes, 14)? >= 40).then_some(())?;
            Some((
                read_u32(bytes, 18)?.cast_signed().unsigned_abs(),
                read_u32(bytes, 22)?.cast_signed().unsigned_abs(),
            ))
        } else {
            None
        }
    }

    /// Decodes a PNG or BMP image; the format is detected from the bytes.
    ///
    /// # Errors
    /// Fails for unsupported formats and corrupt data, and with
    /// [`SkinError::Limit`] for a picture larger than
    /// [`crate::limits::PICTURE_SIDE`] or [`crate::limits::PICTURE_PIXELS`],
    /// which is refused before anything of its size is allocated.
    pub fn decode(name: &str, bytes: &[u8]) -> Result<Self, SkinError> {
        let error = |reason: String| SkinError::Image {
            name: name.to_owned(),
            reason,
        };
        if let Some((width, height)) = Self::declared_size(bytes) {
            limits::picture(name, width, height)?;
        }
        if bytes.starts_with(b"\x89PNG") {
            decode_png(bytes).map_err(error)
        } else if bytes.starts_with(b"BM") {
            decode_bmp(bytes).map_err(error)
        } else {
            Err(error("not a PNG or BMP image".to_owned()))
        }
    }

    /// [`Image::decode`] for one of a skin's pictures, whose pixels are
    /// taken from the skin's `budget`.
    ///
    /// # Errors
    /// As [`Image::decode`], and with [`SkinError::Limit`] when the skin's
    /// pictures together are over [`crate::limits::SKIN_PIXELS`].
    pub fn decode_within(
        name: &str,
        bytes: &[u8],
        budget: &mut limits::Pixels,
    ) -> Result<Self, SkinError> {
        within(name, bytes, budget, || Self::decode(name, bytes))
    }

    /// Encodes the image as an uncompressed 24-bit Windows bitmap
    /// (alpha dropped): classic skins' format, for synthetic fixtures.
    #[must_use]
    pub fn encode_bmp(&self) -> Vec<u8> {
        let (width, height) = (self.width as usize, self.height as usize);
        let stride = (width * 3).div_ceil(4) * 4;
        let size = 54 + stride * height;
        let mut out = vec![0_u8; 54];
        out[0..2].copy_from_slice(b"BM");
        out[2..6].copy_from_slice(&u32::try_from(size).unwrap_or(u32::MAX).to_le_bytes());
        out[10..14].copy_from_slice(&54_u32.to_le_bytes());
        out[14..18].copy_from_slice(&40_u32.to_le_bytes());
        out[18..22].copy_from_slice(&self.width.to_le_bytes());
        out[22..26].copy_from_slice(&self.height.to_le_bytes());
        out[26..28].copy_from_slice(&1_u16.to_le_bytes());
        out[28..30].copy_from_slice(&24_u16.to_le_bytes());
        for row in (0..height).rev() {
            let mut line = vec![0_u8; stride];
            for x in 0..width {
                let at = (row * width + x) * 4;
                line[x * 3..x * 3 + 3].copy_from_slice(&[
                    self.data[at + 2],
                    self.data[at + 1],
                    self.data[at],
                ]);
            }
            out.extend_from_slice(&line);
        }
        out
    }

    /// Encodes the image as PNG.
    ///
    /// # Errors
    /// Fails only when the encoder rejects the image size.
    pub fn encode_png(&self) -> Result<Vec<u8>, SkinError> {
        let error = |reason: png::EncodingError| SkinError::Image {
            name: "output".to_owned(),
            reason: reason.to_string(),
        };
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(error)?;
        writer.write_image_data(&self.data).map_err(error)?;
        writer.finish().map_err(error)?;
        Ok(out)
    }
}

fn pixel_count(width: u32, height: u32) -> usize {
    width as usize * height as usize
}

/// Decodes one of a skin's pictures with its declared size taken from
/// `budget` first; a picture that does not decode gives its pixels back.
fn within(
    name: &str,
    bytes: &[u8],
    budget: &mut limits::Pixels,
    decode: impl FnOnce() -> Result<Image, SkinError>,
) -> Result<Image, SkinError> {
    let declared = Image::declared_size(bytes);
    if let Some((width, height)) = declared {
        budget.take(name, width, height)?;
    }
    let decoded = decode();
    if let (Err(_), Some((width, height))) = (&decoded, declared) {
        budget.give_back(width, height);
    }
    decoded
}

fn decode_png(bytes: &[u8]) -> Result<Image, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    // The header's size was checked before the decoder started; its own
    // reading of it is checked again before the buffers are made.
    let (width, height) = reader.info().size();
    if !limits::picture_fits(width, height) {
        return Err("image too large".to_owned());
    }
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "image too large".to_owned())?;
    let mut buffer = vec![0; size];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let mut image = Image::new(info.width, info.height);
    let pixels = pixel_count(info.width, info.height);
    let source = &buffer[..info.buffer_size()];
    let target = &mut image.data;
    match info.color_type {
        png::ColorType::Rgba => target.copy_from_slice(&source[..pixels * 4]),
        png::ColorType::Rgb => {
            for (dst, src) in target
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(source.as_chunks::<3>().0)
            {
                dst.copy_from_slice(&[src[0], src[1], src[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for (dst, src) in target
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(source.as_chunks::<2>().0)
            {
                dst.copy_from_slice(&[src[0], src[0], src[0], src[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for (dst, &value) in target.as_chunks_mut::<4>().0.iter_mut().zip(source.iter()) {
                dst.copy_from_slice(&[value, value, value, 255]);
            }
        }
        png::ColorType::Indexed => return Err("palette was not expanded".to_owned()),
    }
    Ok(image)
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Magenta marks transparent pixels in BMP skin images.
const BMP_COLOR_KEY: [u8; 3] = [255, 0, 255];

/// `BI_RGB`, `BI_RLE8`, `BI_RLE4` and `BI_BITFIELDS`.
const BI_RGB: u32 = 0;
const BI_RLE8: u32 = 1;
const BI_RLE4: u32 = 2;
const BI_BITFIELDS: u32 = 3;

/// A bitmap's pixels before the colour key: palette indices or colours.
enum Pixels {
    Indexed {
        palette: Vec<[u8; 3]>,
        index: Vec<u8>,
    },
    Rgb(Vec<[u8; 3]>),
}

impl Image {
    /// Decodes a Windows bitmap without PotPlayer's colour key: classic
    /// Winamp skins draw magenta as magenta (M9).
    ///
    /// # Errors
    /// Fails for unsupported or corrupt bitmaps.
    pub fn decode_opaque_bmp(name: &str, bytes: &[u8]) -> Result<Self, SkinError> {
        if let Some((width, height)) = Self::declared_size(bytes) {
            limits::picture(name, width, height)?;
        }
        decode_bitmap(bytes, false).map_err(|reason| SkinError::Image {
            name: name.to_owned(),
            reason,
        })
    }

    /// [`Image::decode_opaque_bmp`] with the pixels taken from the skin's
    /// `budget`, as [`Image::decode_within`].
    ///
    /// # Errors
    /// As [`Image::decode_within`].
    pub fn decode_opaque_bmp_within(
        name: &str,
        bytes: &[u8],
        budget: &mut limits::Pixels,
    ) -> Result<Self, SkinError> {
        within(name, bytes, budget, || Self::decode_opaque_bmp(name, bytes))
    }
}

fn decode_bmp(bytes: &[u8]) -> Result<Image, String> {
    decode_bitmap(bytes, true)
}

/// Decodes Windows bitmaps as `LoadImage` reads them (M9's classic skins
/// use every variant): 1-, 4-, 8-, 16-, 24- and 32-bit, RLE8 and RLE4,
/// 16- and 32-bit `BI_BITFIELDS`, either row order. 32-bit alpha is
/// ignored, as GDI's `BitBlt` does. `color_key`: magenta is transparent
/// (PotPlayer's skins).
#[allow(clippy::too_many_lines)] // One decoder for every variant of the format.
fn decode_bitmap(bytes: &[u8], color_key: bool) -> Result<Image, String> {
    let short = || "truncated bitmap".to_owned();
    let data_offset = read_u32(bytes, 10).ok_or_else(short)? as usize;
    let header_size = read_u32(bytes, 14).ok_or_else(short)? as usize;
    if header_size < 40 {
        return Err(format!(
            "{header_size}-byte bitmap headers are not supported"
        ));
    }
    let width = read_u32(bytes, 18).ok_or_else(short)?.cast_signed();
    let height = read_u32(bytes, 22).ok_or_else(short)?.cast_signed();
    let bits = read_u16(bytes, 28).ok_or_else(short)?;
    let compression = read_u32(bytes, 30).ok_or_else(short)?;
    let colors_used = read_u32(bytes, 46).ok_or_else(short)? as usize;
    if width <= 0
        || height == 0
        || !limits::picture_fits(width.unsigned_abs(), height.unsigned_abs())
    {
        return Err("empty or oversized bitmap".to_owned());
    }
    let bottom_up = height > 0;
    let width = width.unsigned_abs() as usize;
    let height = height.unsigned_abs() as usize;
    let palette = if bits <= 8 {
        let count = if colors_used == 0 || colors_used > 1 << bits {
            1 << bits
        } else {
            colors_used
        };
        let start = 14 + header_size;
        // A short palette leaves the rest black.
        let available = bytes.len().saturating_sub(start) / 4;
        let table = bytes
            .get(start..start + count.min(available) * 4)
            .ok_or_else(short)?;
        let mut palette: Vec<[u8; 3]> = table
            .as_chunks::<4>()
            .0
            .iter()
            .map(|entry| [entry[2], entry[1], entry[0]])
            .collect();
        palette.resize(1 << bits, [0; 3]);
        palette
    } else {
        Vec::new()
    };
    // Rows as stored, first stored row first.
    let pixels = match (compression, bits) {
        (BI_RLE8, 8) | (BI_RLE4, 4) => Pixels::Indexed {
            palette,
            index: decode_rle(
                bytes.get(data_offset..).ok_or_else(short)?,
                width,
                height,
                bits,
            )?,
        },
        (BI_RGB, 1 | 4 | 8) => {
            let stride = (width * usize::from(bits)).div_ceil(32) * 4;
            let mut index = Vec::with_capacity(width * height);
            for row in 0..height {
                let start = data_offset + row * stride;
                let line = bytes.get(start..start + stride).ok_or_else(short)?;
                let per_byte = 8 / usize::from(bits);
                let mask = u8::MAX >> (8 - bits);
                for x in 0..width {
                    let byte = line[x / per_byte];
                    let shift = 8 - usize::from(bits) * (x % per_byte + 1);
                    index.push((byte >> shift) & mask);
                }
            }
            Pixels::Indexed { palette, index }
        }
        (BI_RGB | BI_BITFIELDS, 16 | 24 | 32) => {
            let masks = if compression == BI_BITFIELDS {
                let at = 14 + 40;
                [
                    read_u32(bytes, at).ok_or_else(short)?,
                    read_u32(bytes, at + 4).ok_or_else(short)?,
                    read_u32(bytes, at + 8).ok_or_else(short)?,
                ]
            } else if bits == 16 {
                [0x7C00, 0x03E0, 0x001F]
            } else {
                [0x00FF_0000, 0x0000_FF00, 0x0000_00FF]
            };
            let size = usize::from(bits / 8);
            let stride = (width * size).div_ceil(4) * 4;
            let mut colors = Vec::with_capacity(width * height);
            for row in 0..height {
                let start = data_offset + row * stride;
                let line = bytes.get(start..start + width * size).ok_or_else(short)?;
                for pixel in line.chunks_exact(size) {
                    let rgb = if bits == 24 {
                        [pixel[2], pixel[1], pixel[0]]
                    } else {
                        let mut value = [0_u8; 4];
                        value[..size].copy_from_slice(pixel);
                        let value = u32::from_le_bytes(value);
                        masks.map(|mask| channel(value, mask))
                    };
                    colors.push(rgb);
                }
            }
            Pixels::Rgb(colors)
        }
        _ => {
            return Err(format!(
                "{bits}-bit bitmaps with compression {compression} are not supported"
            ));
        }
    };
    let width_u32 = u32::try_from(width).map_err(|_| short())?;
    let height_u32 = u32::try_from(height).map_err(|_| short())?;
    let mut image = Image::new(width_u32, height_u32);
    for row in 0..height {
        let stored = if bottom_up { height - 1 - row } else { row };
        let target = &mut image.data[row * width * 4..(row + 1) * width * 4];
        for (x, dst) in target.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let rgb = match &pixels {
                Pixels::Indexed { palette, index } => palette
                    .get(usize::from(index[stored * width + x]))
                    .copied()
                    .unwrap_or([0; 3]),
                Pixels::Rgb(colors) => colors[stored * width + x],
            };
            let alpha = if color_key && rgb == BMP_COLOR_KEY {
                0
            } else {
                u8::MAX
            };
            dst.copy_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
        }
    }
    Ok(image)
}

/// One colour channel of a packed pixel, scaled to 8 bits.
fn channel(value: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    let raw = (value & mask) >> shift;
    // Below 2^32 / 255: the product fits.
    u8::try_from(u64::from(raw) * 255 / u64::from(max)).unwrap_or(u8::MAX)
}

/// Run-length encoded indices (RLE8, RLE4), in stored row order; pixels
/// the runs skip stay index 0.
fn decode_rle(data: &[u8], width: usize, height: usize, bits: u16) -> Result<Vec<u8>, String> {
    let mut index = vec![0_u8; width * height];
    let (mut x, mut y, mut at) = (0_usize, 0_usize, 0_usize);
    let mut put = |x: usize, y: usize, value: u8| {
        if x < width && y < height {
            index[y * width + x] = value;
        }
    };
    let byte = |at: usize| {
        data.get(at)
            .copied()
            .ok_or_else(|| "truncated RLE data".to_owned())
    };
    loop {
        let count = usize::from(byte(at)?);
        let value = byte(at + 1)?;
        at += 2;
        if count > 0 {
            for i in 0..count {
                let pixel = if bits == 8 {
                    value
                } else if i % 2 == 0 {
                    value >> 4
                } else {
                    value & 0x0F
                };
                put(x, y, pixel);
                x += 1;
            }
            continue;
        }
        match value {
            0 => {
                x = 0;
                y += 1;
            }
            1 => break,
            2 => {
                x += usize::from(byte(at)?);
                y += usize::from(byte(at + 1)?);
                at += 2;
            }
            run => {
                let run = usize::from(run);
                let length = if bits == 8 { run } else { run.div_ceil(2) };
                for i in 0..run {
                    let pixel = if bits == 8 {
                        byte(at + i)?
                    } else {
                        let packed = byte(at + i / 2)?;
                        if i % 2 == 0 {
                            packed >> 4
                        } else {
                            packed & 0x0F
                        }
                    };
                    put(x, y, pixel);
                    x += 1;
                }
                // Absolute runs are padded to a word.
                at += length.div_ceil(2) * 2;
            }
        }
        if y >= height {
            break;
        }
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::Image;

    fn bmp_24(width: u32, height: u32, rows_bottom_up: &[&[[u8; 3]]]) -> Vec<u8> {
        let stride = (width as usize * 3).div_ceil(4) * 4;
        let mut bytes = vec![0_u8; 54];
        bytes[0..2].copy_from_slice(b"BM");
        bytes[10..14].copy_from_slice(&54_u32.to_le_bytes());
        bytes[14..18].copy_from_slice(&40_u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&width.to_le_bytes());
        bytes[22..26].copy_from_slice(&height.to_le_bytes());
        bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
        bytes[28..30].copy_from_slice(&24_u16.to_le_bytes());
        for row in rows_bottom_up {
            let mut line = vec![0_u8; stride];
            for (x, rgb) in row.iter().enumerate() {
                line[x * 3..x * 3 + 3].copy_from_slice(&[rgb[2], rgb[1], rgb[0]]);
            }
            bytes.extend_from_slice(&line);
        }
        bytes
    }

    #[test]
    fn png_round_trip() {
        let mut image = Image::new(3, 2);
        image.data_mut()[4..8].copy_from_slice(&[1, 2, 3, 4]);
        let encoded = image.encode_png().expect("encode");
        let decoded = Image::decode("x.png", &encoded).expect("decode");
        assert_eq!(decoded, image);
        assert_eq!(decoded.pixel(1, 0), [1, 2, 3, 4]);
        assert_eq!(decoded.pixel(-1, 0), [0; 4]);
    }

    /// A bitmap with a 40-byte header, a palette and the given pixel data.
    fn bitmap(
        width: i32,
        height: i32,
        bits: u16,
        compression: u32,
        palette: &[[u8; 3]],
        data: &[u8],
    ) -> Vec<u8> {
        let offset = 54 + palette.len() * 4;
        let mut bytes = vec![0_u8; 54];
        bytes[0..2].copy_from_slice(b"BM");
        bytes[10..14].copy_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
        bytes[14..18].copy_from_slice(&40_u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&width.to_le_bytes());
        bytes[22..26].copy_from_slice(&height.to_le_bytes());
        bytes[26..28].copy_from_slice(&1_u16.to_le_bytes());
        bytes[28..30].copy_from_slice(&bits.to_le_bytes());
        bytes[30..34].copy_from_slice(&compression.to_le_bytes());
        bytes[46..50].copy_from_slice(&u32::try_from(palette.len()).unwrap().to_le_bytes());
        for rgb in palette {
            bytes.extend_from_slice(&[rgb[2], rgb[1], rgb[0], 0]);
        }
        bytes.extend_from_slice(data);
        bytes
    }

    #[test]
    fn classic_bitmap_variants_decode() {
        let palette = [[0, 0, 0], [255, 0, 255], [10, 20, 30]];
        // 4-bit, two rows of 3 pixels, top-down (negative height).
        let four = bitmap(3, -2, 4, 0, &palette, &[0x12, 0x00, 0, 0, 0x20, 0x10, 0, 0]);
        let image = Image::decode_opaque_bmp("x.bmp", &four).unwrap();
        assert_eq!(image.pixel(0, 0), [255, 0, 255, 255]);
        assert_eq!(image.pixel(1, 0), [10, 20, 30, 255]);
        assert_eq!(image.pixel(2, 1), [255, 0, 255, 255]);
        // The colour key only for PotPlayer's skins.
        assert_eq!(Image::decode("x.bmp", &four).unwrap().pixel(0, 0)[3], 0);
        // 1-bit.
        let one = bitmap(9, 1, 1, 0, &palette[..2], &[0b1010_0000, 0x80, 0, 0]);
        let image = Image::decode_opaque_bmp("x.bmp", &one).unwrap();
        assert_eq!(image.pixel(0, 0), [255, 0, 255, 255]);
        assert_eq!(image.pixel(1, 0), [0, 0, 0, 255]);
        assert_eq!(image.pixel(8, 0), [255, 0, 255, 255]);
        // RLE8: a run of 3 of index 2, an absolute run of 1 (padded),
        // end of line, then end of bitmap; bottom-up.
        let rle8 = bitmap(4, 2, 8, 1, &palette, &[3, 2, 0, 3, 1, 1, 0, 0, 0, 0, 0, 1]);
        let image = Image::decode_opaque_bmp("x.bmp", &rle8).unwrap();
        assert_eq!(image.pixel(0, 1), [10, 20, 30, 255]);
        assert_eq!(image.pixel(3, 1), [255, 0, 255, 255]);
        assert_eq!(image.pixel(0, 0), [0, 0, 0, 255]);
        // RLE4: four pixels alternating 1 and 2.
        let rle4 = bitmap(4, 1, 4, 2, &palette, &[4, 0x12, 0, 1]);
        let image = Image::decode_opaque_bmp("x.bmp", &rle4).unwrap();
        assert_eq!(image.pixel(0, 0), [255, 0, 255, 255]);
        assert_eq!(image.pixel(1, 0), [10, 20, 30, 255]);
        // 16-bit 5-5-5: pure red.
        let sixteen = bitmap(1, 1, 16, 0, &[], &[0x00, 0x7C, 0, 0]);
        assert_eq!(
            Image::decode_opaque_bmp("x.bmp", &sixteen)
                .unwrap()
                .pixel(0, 0),
            [255, 0, 0, 255]
        );
    }

    #[test]
    fn bmp_rows_are_bottom_up_and_magenta_is_transparent() {
        let bytes = bmp_24(
            2,
            2,
            &[&[[1, 2, 3], [255, 0, 255]], &[[9, 9, 9], [7, 7, 7]]],
        );
        let image = Image::decode("x.bmp", &bytes).expect("decode");
        assert_eq!(image.pixel(0, 0), [9, 9, 9, 255]);
        assert_eq!(image.pixel(0, 1), [1, 2, 3, 255]);
        assert_eq!(image.pixel(1, 1), [255, 0, 255, 0]);
    }

    /// The start of a PNG file: its signature and a header that gives
    /// `width` x `height` pixels of one bit each. No pixel follows.
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[1, 0, 0, 0, 0]);
        bytes
    }

    fn is_limit(result: &Result<Image, crate::SkinError>) -> bool {
        matches!(result, Err(crate::SkinError::Limit(_)))
    }

    #[test]
    fn a_picture_over_the_limit_is_refused_from_its_header() {
        // A few dozen bytes that ask for the memory of a million pixels a
        // side: refused before anything is allocated.
        let huge = png_header(1_000_000, 1_000_000);
        assert_eq!(Image::declared_size(&huge), Some((1_000_000, 1_000_000)));
        assert!(is_limit(&Image::decode("huge.png", &huge)));
        // Within the sides' limit, over the pixels'.
        assert!(is_limit(&Image::decode(
            "wide.png",
            &png_header(8192, 4096)
        )));
        assert!(is_limit(&Image::decode(
            "long.png",
            &png_header(1, crate::limits::PICTURE_SIDE + 1)
        )));
        // The largest picture among the measured skins is one of a size.
        let strip = Image::decode("strip.png", &png_header(377, 8694));
        assert!(matches!(strip, Err(crate::SkinError::Image { .. })));
        // Bitmaps likewise, in either row order and with every decoder.
        let rle = bitmap(30_000, 30_000, 8, 1, &[], &[0, 1]);
        assert!(rle.len() < 64);
        assert!(is_limit(&Image::decode("rle.bmp", &rle)));
        assert!(is_limit(&Image::decode_opaque_bmp("rle.bmp", &rle)));
        let top_down = bitmap(5000, -5000, 24, 0, &[], &[]);
        assert!(is_limit(&Image::decode("down.bmp", &top_down)));
        // A header of the older kind keeps no size where it is looked for.
        let mut old = bitmap(1, 1, 24, 0, &[], &[0, 0, 0, 0]);
        old[14..18].copy_from_slice(&12_u32.to_le_bytes());
        assert_eq!(Image::declared_size(&old), None);
        assert!(matches!(
            Image::decode("old.bmp", &old),
            Err(crate::SkinError::Image { .. })
        ));
    }

    #[test]
    fn a_skins_pictures_are_taken_from_its_budget() {
        let mut picture = Image::new(4, 4);
        picture.fill([1, 2, 3, 255]);
        let png = picture.encode_png().expect("encode");
        let mut budget = crate::limits::Pixels::with(40);
        // Two fit; the third does not.
        assert!(Image::decode_within("a.png", &png, &mut budget).is_ok());
        assert!(Image::decode_within("b.png", &png, &mut budget).is_ok());
        assert!(is_limit(&Image::decode_within("c.png", &png, &mut budget)));
        // A picture that does not decode takes nothing.
        let mut budget = crate::limits::Pixels::with(40);
        for _ in 0..8 {
            let broken = Image::decode_within("broken.png", &png_header(4, 4), &mut budget);
            assert!(matches!(broken, Err(crate::SkinError::Image { .. })));
        }
        let bmp = picture.encode_bmp();
        assert!(Image::decode_opaque_bmp_within("a.bmp", &bmp, &mut budget).is_ok());
        assert!(Image::decode_opaque_bmp_within("b.bmp", &bmp, &mut budget).is_ok());
        assert!(is_limit(&Image::decode_opaque_bmp_within(
            "c.bmp",
            &bmp,
            &mut budget
        )));
    }
}
