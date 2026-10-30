//! A DVD's sub-pictures (SPU): the menus' buttons and the subtitles. RVP
//! draws a menu's sub-picture itself, with the disc's colours and the
//! selected button's highlight (M16, `docs/spikes/s7-disc-menus.md`), so
//! it reads the sub-picture units from the program stream it hands mpv.
//!
//! * [`Assembler`] collects one stream's units from 2048-byte DVD blocks:
//!   private stream 1's PES packets of sub-stream `0x20 + n`, joined until
//!   the unit's own size.
//! * [`decode`] reads a unit: its control sequences (the colour and
//!   contrast of the four pixel values, the display area, the two fields'
//!   offsets, forced display) and its run-length coded fields.

/// One block of a DVD's program stream.
pub const BLOCK: usize = 2048;

/// A decoded sub-picture: `width` x `height` pixel values (0 to 3) at `x`,
/// `y` of the frame, and what the unit says about them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubPicture {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// Row by row.
    pub pixels: Vec<u8>,
    /// The palette (CLUT) entry of each pixel value.
    pub colours: [u8; 4],
    /// Each pixel value's opacity, 0 (transparent) to 15.
    pub contrast: [u8; 4],
    /// Shown even when subtitles are off (menus are).
    pub forced: bool,
}

/// Why a unit could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpuError {
    Short,
    NoDisplayArea,
    BadOffset,
}

/// Reads one sub-picture unit.
///
/// # Errors
/// Fails when the unit is cut short, has no display area or points past
/// itself.
pub fn decode(unit: &[u8]) -> Result<SubPicture, SpuError> {
    let word = |at: usize| -> Result<usize, SpuError> {
        unit.get(at..at + 2)
            .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
            .ok_or(SpuError::Short)
    };
    let size = word(0)?;
    if size > unit.len() || size < 4 {
        return Err(SpuError::Short);
    }
    let unit = &unit[..size];
    let mut colours = [0u8; 4];
    let mut contrast = [0u8; 4];
    let mut area = None;
    let mut fields = None;
    let mut forced = false;
    let mut at = word(2)?;
    // Each sequence: a date, the next sequence's offset, commands up to
    // 0xFF; the last one points to itself.
    for _ in 0..64 {
        if at + 4 > unit.len() {
            return Err(SpuError::BadOffset);
        }
        let next = word(at + 2)?;
        let mut i = at + 4;
        while i < unit.len() {
            let command = unit[i];
            i += 1;
            let take = |count: usize| unit.get(i..i + count).ok_or(SpuError::Short);
            match command {
                0x00 => forced = true,
                0x01 | 0x02 => {}
                0x03 => {
                    let b = take(2)?;
                    colours = [b[1] & 0x0f, b[1] >> 4, b[0] & 0x0f, b[0] >> 4];
                    i += 2;
                }
                0x04 => {
                    let b = take(2)?;
                    contrast = [b[1] & 0x0f, b[1] >> 4, b[0] & 0x0f, b[0] >> 4];
                    i += 2;
                }
                0x05 => {
                    let b = take(6)?;
                    let x1 = (u16::from(b[0]) << 4) | u16::from(b[1] >> 4);
                    let x2 = (u16::from(b[1] & 0x0f) << 8) | u16::from(b[2]);
                    let y1 = (u16::from(b[3]) << 4) | u16::from(b[4] >> 4);
                    let y2 = (u16::from(b[4] & 0x0f) << 8) | u16::from(b[5]);
                    area = Some((x1, x2, y1, y2));
                    i += 6;
                }
                0x06 => {
                    let b = take(4)?;
                    fields = Some((
                        usize::from(u16::from_be_bytes([b[0], b[1]])),
                        usize::from(u16::from_be_bytes([b[2], b[3]])),
                    ));
                    i += 4;
                }
                // Colour and contrast changes inside the area: its size
                // first, then skipped.
                0x07 => {
                    let b = take(2)?;
                    i += usize::from(u16::from_be_bytes([b[0], b[1]]));
                }
                _ => break,
            }
        }
        if next == at || next >= unit.len() {
            break;
        }
        at = next;
    }
    let (x1, x2, y1, y2) = area.ok_or(SpuError::NoDisplayArea)?;
    let (top, bottom) = fields.ok_or(SpuError::NoDisplayArea)?;
    if x2 < x1 || y2 < y1 {
        return Err(SpuError::NoDisplayArea);
    }
    let width = x2 - x1 + 1;
    let height = y2 - y1 + 1;
    let mut pixels = vec![0u8; usize::from(width) * usize::from(height)];
    let mut readers = [Nibbles::new(unit, top), Nibbles::new(unit, bottom)];
    for (line, row) in pixels.chunks_exact_mut(usize::from(width)).enumerate() {
        let reader = &mut readers[line % 2];
        reader.line(row)?;
    }
    Ok(SubPicture {
        x: x1,
        y: y1,
        width,
        height,
        pixels,
        colours,
        contrast,
        forced,
    })
}

/// A field's run-length codes, read nibble by nibble.
struct Nibbles<'a> {
    data: &'a [u8],
    /// In nibbles.
    at: usize,
}

impl<'a> Nibbles<'a> {
    fn new(data: &'a [u8], byte: usize) -> Self {
        Self { data, at: byte * 2 }
    }

    fn next(&mut self) -> Result<u16, SpuError> {
        let byte = *self.data.get(self.at / 2).ok_or(SpuError::Short)?;
        let nibble = if self.at.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 0x0f
        };
        self.at += 1;
        Ok(u16::from(nibble))
    }

    /// One line: codes of 4, 8, 12 or 16 bits, a run length and a pixel
    /// value; a run of 0 fills the rest of the line; lines end on a byte.
    fn line(&mut self, row: &mut [u8]) -> Result<(), SpuError> {
        let mut x = 0;
        while x < row.len() {
            // A code is longer while its leading nibbles are small.
            let mut code = self.next()?;
            for limit in [0x4, 0x10, 0x40] {
                if code >= limit {
                    break;
                }
                code = (code << 4) | self.next()?;
            }
            let value = (code & 3) as u8;
            let run = usize::from(code >> 2);
            let end = if run == 0 {
                row.len()
            } else {
                (x + run).min(row.len())
            };
            row[x..end].fill(value);
            x = end;
        }
        if self.at % 2 == 1 {
            self.at += 1;
        }
        Ok(())
    }
}

/// Collects one sub-picture stream's units from DVD blocks.
#[derive(Debug, Default)]
pub struct Assembler {
    /// The stream (0 to 31).
    stream: u8,
    unit: Vec<u8>,
    /// The unit's size once its first bytes came.
    size: Option<usize>,
}

impl Assembler {
    #[must_use]
    pub fn new(stream: u8) -> Self {
        Self {
            stream,
            ..Self::default()
        }
    }

    /// Another stream: what was collected goes.
    pub fn set_stream(&mut self, stream: u8) {
        if stream != self.stream {
            self.stream = stream;
            self.unit.clear();
            self.size = None;
        }
    }

    /// A block's PES packets of this stream; the units they complete.
    pub fn push(&mut self, block: &[u8]) -> Vec<Vec<u8>> {
        let mut done = Vec::new();
        for payload in private_stream_1(block) {
            let Some((&sub, data)) = payload.split_first() else {
                continue;
            };
            if sub != 0x20 + self.stream {
                continue;
            }
            if self.size.is_none() {
                self.unit.clear();
                if data.len() < 2 {
                    continue;
                }
                self.size = Some(usize::from(u16::from_be_bytes([data[0], data[1]])));
            }
            self.unit.extend_from_slice(data);
            if let Some(size) = self.size
                && self.unit.len() >= size
            {
                self.unit.truncate(size);
                done.push(std::mem::take(&mut self.unit));
                self.size = None;
            }
        }
        done
    }
}

/// The payloads (from the sub-stream byte on) of a block's private stream
/// 1 PES packets.
fn private_stream_1(block: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    // The pack header: 14 bytes and its stuffing (MPEG-2).
    if block.len() < 14 || block[..4] != [0, 0, 1, 0xBA] {
        return out;
    }
    let mut at = 14 + usize::from(block[13] & 7);
    while at + 6 <= block.len() {
        if block[at..at + 3] != [0, 0, 1] {
            break;
        }
        let id = block[at + 3];
        let length = usize::from(u16::from_be_bytes([block[at + 4], block[at + 5]]));
        let end = (at + 6 + length).min(block.len());
        if id == 0xBD && at + 9 <= end {
            // The MPEG-2 PES header: flags, flags, its data's length.
            let start = at + 9 + usize::from(block[at + 8]);
            if start < end {
                out.push(&block[start..end]);
            }
        }
        at = end;
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn be16(value: usize) -> [u8; 2] {
        u16::try_from(value).expect("fits 16 bits").to_be_bytes()
    }

    /// Two 12-bit numbers in three bytes (the display area's command).
    fn pack12(first: u16, second: u16) -> [u8; 3] {
        let [_, a, b, c] = ((u32::from(first) << 12) | u32::from(second)).to_be_bytes();
        [a, b, c]
    }

    /// Run-length codes for one line of pixel values, padded to a byte.
    fn encode_line(row: &[u8], out: &mut Vec<u8>) {
        let mut nibbles: Vec<u8> = Vec::new();
        let mut x = 0;
        while x < row.len() {
            let value = row[x];
            let mut run = 1;
            while x + run < row.len() && row[x + run] == value && run < 255 {
                run += 1;
            }
            let code = if x + run == row.len() {
                u16::from(value)
            } else {
                (u16::try_from(run).expect("a run fits") << 2) | u16::from(value)
            };
            let digits = if x + run == row.len() {
                4
            } else if code < 0x10 {
                1
            } else if code < 0x40 {
                2
            } else if code < 0x100 {
                3
            } else {
                4
            };
            for d in (0..digits).rev() {
                nibbles.push(u8::try_from((code >> (4 * d)) & 0xf).expect("a nibble"));
            }
            x += run;
        }
        if nibbles.len() % 2 == 1 {
            nibbles.push(0);
        }
        for pair in nibbles.chunks(2) {
            out.push((pair[0] << 4) | pair[1]);
        }
    }

    /// A unit for `pixels` (width x height) at x, y with the given
    /// colours and contrast, forced.
    pub(crate) fn encode(
        x: u16,
        y: u16,
        width: u16,
        pixels: &[u8],
        colours: [u8; 4],
        contrast: [u8; 4],
    ) -> Vec<u8> {
        let height = pixels.len() / usize::from(width);
        let mut top = Vec::new();
        let mut bottom = Vec::new();
        for (line, row) in pixels.chunks(usize::from(width)).enumerate() {
            encode_line(row, if line % 2 == 0 { &mut top } else { &mut bottom });
        }
        let top_at = 4;
        let bottom_at = top_at + top.len();
        let control_at = bottom_at + bottom.len();
        let x2 = x + width - 1;
        let y2 = y + u16::try_from(height).expect("the height fits") - 1;
        let mut control = vec![0, 0];
        control.extend_from_slice(&be16(control_at));
        control.push(0x00);
        control.push(0x01);
        control.extend_from_slice(&[
            0x03,
            (colours[3] << 4) | colours[2],
            (colours[1] << 4) | colours[0],
        ]);
        control.extend_from_slice(&[
            0x04,
            (contrast[3] << 4) | contrast[2],
            (contrast[1] << 4) | contrast[0],
        ]);
        control.push(0x05);
        control.extend_from_slice(&pack12(x, x2));
        control.extend_from_slice(&pack12(y, y2));
        control.push(0x06);
        control.extend_from_slice(&be16(top_at));
        control.extend_from_slice(&be16(bottom_at));
        control.push(0xFF);
        let size = control_at + control.len();
        let mut unit = Vec::with_capacity(size);
        unit.extend_from_slice(&be16(size));
        unit.extend_from_slice(&be16(control_at));
        unit.extend_from_slice(&top);
        unit.extend_from_slice(&bottom);
        unit.extend_from_slice(&control);
        unit
    }

    /// A frame of `width` x `height` with a frame of value 1 around it and
    /// value 2 inside a smaller box.
    pub(crate) fn sample(width: u16, height: u16) -> Vec<u8> {
        let (w, h) = (usize::from(width), usize::from(height));
        let mut pixels = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                let inner = x >= 3 && x < w - 3 && y >= 3 && y < h - 3;
                pixels[y * w + x] = if edge {
                    1
                } else if inner {
                    2
                } else {
                    0
                };
            }
        }
        pixels
    }

    #[test]
    fn units_decode_to_their_pixels_colours_and_place() {
        let pixels = sample(40, 12);
        let unit = encode(80, 190, 40, &pixels, [0, 5, 6, 7], [0, 15, 8, 0]);
        let decoded = decode(&unit).expect("decodes");
        assert_eq!(
            (decoded.x, decoded.y, decoded.width, decoded.height),
            (80, 190, 40, 12)
        );
        assert_eq!(decoded.pixels, pixels);
        assert_eq!(decoded.colours, [0, 5, 6, 7]);
        assert_eq!(decoded.contrast, [0, 15, 8, 0]);
        assert!(decoded.forced);
    }

    #[test]
    fn broken_units_are_refused() {
        let pixels = sample(16, 4);
        let unit = encode(0, 0, 16, &pixels, [0, 1, 2, 3], [0, 15, 15, 15]);
        assert_eq!(decode(&unit[..3]), Err(SpuError::Short));
        let mut wrong = unit.clone();
        wrong[2] = 0xff;
        assert!(decode(&wrong).is_err());
    }

    /// A pack with one private stream 1 PES packet of sub-stream `sub`.
    fn block(sub: u8, payload: &[u8]) -> Vec<u8> {
        let mut block = vec![0, 0, 1, 0xBA, 0x44, 0, 4, 0, 4, 1, 0x01, 0x89, 0xC3, 0xF8];
        let length = 3 + 1 + payload.len();
        block.extend_from_slice(&[0, 0, 1, 0xBD]);
        block.extend_from_slice(&be16(length));
        block.extend_from_slice(&[0x81, 0x00, 0x00, sub]);
        block.extend_from_slice(payload);
        block
    }

    #[test]
    fn units_are_joined_across_blocks_of_their_stream() {
        let pixels = sample(64, 40);
        let unit = encode(0, 0, 64, &pixels, [0, 1, 2, 3], [0, 15, 15, 15]);
        let (first, rest) = unit.split_at(unit.len() / 2);
        let mut assembler = Assembler::new(0);
        assert!(
            assembler.push(&block(0x21, &unit)).is_empty(),
            "another stream"
        );
        assert!(assembler.push(&block(0x20, first)).is_empty());
        let done = assembler.push(&block(0x20, rest));
        assert_eq!(done, vec![unit.clone()]);
        assert_eq!(decode(&done[0]).expect("decodes").pixels, pixels);
        assembler.set_stream(1);
        assert_eq!(assembler.push(&block(0x21, &unit)), vec![unit]);
    }
}
