//! Text decoding for skin files.
//!
//! PotPlayer skins mix encodings: UTF-8 with or without a byte order mark,
//! UTF-16 LE, and plain ANSI files. Concatenated files can also carry a byte
//! order mark in the middle (BlackSpace's `SubCommon.txt` does).

/// Decodes a skin text file into a `String` with `\n` line endings and no
/// byte order marks.
#[must_use]
pub fn decode(bytes: &[u8]) -> String {
    let text = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        decode_utf16(rest, u16::from_le_bytes)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        decode_utf16(rest, u16::from_be_bytes)
    } else if looks_like_utf16le(bytes) {
        decode_utf16(bytes, u16::from_le_bytes)
    } else {
        match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => decode_windows_1252(bytes),
        }
    };
    normalize(&text)
}

fn decode_utf16(bytes: &[u8], read: fn([u8; 2]) -> u16) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.as_chunks::<2>().0 {
        units.push(read(*pair));
    }
    String::from_utf16_lossy(&units)
}

/// ASCII text stored as UTF-16 LE without a byte order mark has a zero in
/// every second byte.
fn looks_like_utf16le(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[1] == 0 && bytes[3] == 0 && bytes[0] != 0 && bytes[2] != 0
}

/// The upper half of Windows-1252; 0x80..=0x9F differ from Latin-1.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}', '\u{017D}', '\u{FFFD}',
    '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
];

fn decode_windows_1252(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len());
    for &byte in bytes {
        let ch = match byte {
            0x80..=0x9F => CP1252_HIGH[usize::from(byte - 0x80)],
            _ => char::from(byte),
        };
        text.push(ch);
    }
    text
}

fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\u{FEFF}' => {}
            '\r' => {
                if chars.peek() != Some(&'\n') {
                    out.push('\n');
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn strips_boms_and_carriage_returns() {
        assert_eq!(decode(b"\xEF\xBB\xBFa\r\nb\r\n"), "a\nb\n");
        assert_eq!(decode("x\u{FEFF}y".as_bytes()), "xy");
    }

    #[test]
    fn reads_utf16_with_and_without_bom() {
        assert_eq!(decode(&[0xFF, 0xFE, b'h', 0, b'i', 0]), "hi");
        assert_eq!(decode(&[b'o', 0, b'k', 0]), "ok");
    }

    #[test]
    fn falls_back_to_windows_1252() {
        assert_eq!(decode(&[b'a', 0x80, 0xE9]), "a\u{20AC}\u{E9}");
    }
}
