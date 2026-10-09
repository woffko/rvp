//! RGBA images: decoding skin atlases (PNG, BMP) and encoding PNG output.

use std::io::Cursor;

use crate::error::SkinError;

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

    /// Decodes a PNG or BMP image; the format is detected from the bytes.
    ///
    /// # Errors
    /// Fails for unsupported formats and corrupt data.
    pub fn decode(name: &str, bytes: &[u8]) -> Result<Self, SkinError> {
        let error = |reason: String| SkinError::Image {
            name: name.to_owned(),
            reason,
        };
        if bytes.starts_with(b"\x89PNG") {
            decode_png(bytes).map_err(error)
        } else if bytes.starts_with(b"BM") {
            decode_bmp(bytes).map_err(error)
        } else {
            Err(error("not a PNG or BMP image".to_owned()))
        }
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

fn decode_png(bytes: &[u8]) -> Result<Image, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
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

/// Decodes uncompressed 8-, 24- and 32-bit Windows bitmaps.
fn decode_bmp(bytes: &[u8]) -> Result<Image, String> {
    let short = || "truncated bitmap".to_owned();
    let data_offset = read_u32(bytes, 10).ok_or_else(short)? as usize;
    let header_size = read_u32(bytes, 14).ok_or_else(short)? as usize;
    let width = read_u32(bytes, 18).ok_or_else(short)?.cast_signed();
    let height = read_u32(bytes, 22).ok_or_else(short)?.cast_signed();
    let bits = read_u16(bytes, 28).ok_or_else(short)?;
    let compression = read_u32(bytes, 30).ok_or_else(short)?;
    let colors_used = read_u32(bytes, 46).ok_or_else(short)? as usize;
    if compression != 0 {
        return Err(format!("compression {compression} is not supported"));
    }
    if width <= 0 || height == 0 {
        return Err("empty bitmap".to_owned());
    }
    let bottom_up = height > 0;
    let width = width.unsigned_abs();
    let height = height.unsigned_abs();
    let palette = if bits == 8 {
        let count = if colors_used == 0 { 256 } else { colors_used };
        let start = 14 + header_size;
        let table = bytes.get(start..start + count * 4).ok_or_else(short)?;
        table
            .as_chunks::<4>()
            .0
            .iter()
            .map(|entry| [entry[2], entry[1], entry[0]])
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let bytes_per_pixel = match bits {
        8 => 1,
        24 => 3,
        32 => 4,
        _ => return Err(format!("{bits}-bit bitmaps are not supported")),
    };
    let stride = (width as usize * bytes_per_pixel).div_ceil(4) * 4;
    let mut image = Image::new(width, height);
    for row in 0..height as usize {
        let source_row = if bottom_up {
            height as usize - 1 - row
        } else {
            row
        };
        let start = data_offset + source_row * stride;
        let line = bytes
            .get(start..start + width as usize * bytes_per_pixel)
            .ok_or_else(short)?;
        let target = &mut image.data[row * width as usize * 4..(row + 1) * width as usize * 4];
        for (dst, src) in target
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(line.chunks(bytes_per_pixel))
        {
            let rgb = match bytes_per_pixel {
                1 => palette.get(usize::from(src[0])).copied().unwrap_or([0; 3]),
                _ => [src[2], src[1], src[0]],
            };
            let alpha = if rgb == BMP_COLOR_KEY { 0 } else { u8::MAX };
            dst.copy_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
        }
    }
    Ok(image)
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
}
