//! The text encodings of subtitle files.
//!
//! A BOM names the encoding (UTF-8, UTF-16 LE or BE). Without one the bytes
//! are UTF-8 when they are valid UTF-8, else in the code page the caller
//! names (the user's setting, like mpv's `sub-codepage`), else a guess.
//! Text with more ASCII letters than high bytes is Western
//! (`windows-1252`). Otherwise it is Cyrillic `windows-1251` when nearly all
//! high bytes are in that code page's letter range, else the first of
//! `EUC-KR` (CP949, PotPlayer's home), `Shift_JIS`, `GBK` and `Big5` that
//! reads the bytes without an error, else `windows-1252`.

use encoding_rs::{BIG5, EUC_KR, Encoding, GBK, SHIFT_JIS, UTF_8, WINDOWS_1251, WINDOWS_1252};

use crate::Error;

/// The encoding a label names: the WHATWG labels (`utf-8`, `windows-1251`,
/// `cp1251`, `euc-kr`, `shift_jis`, `gbk`, `big5`, `koi8-r`...) and the code
/// page numbers players use (`cp949`, `cp932`, `cp936`, `cp950`), in any
/// case. `None` for `auto`, an empty label or one RVP does not know.
#[must_use]
pub fn resolve_encoding(label: &str) -> Option<&'static Encoding> {
    let label = label.trim().to_ascii_lowercase();
    let alias = match label.as_str() {
        "cp949" | "uhc" => "euc-kr",
        "cp932" => "shift_jis",
        "cp936" => "gbk",
        "cp950" => "big5",
        "utf8" => "utf-8",
        other => other,
    };
    Encoding::for_label(alias.as_bytes())
}

/// The text of a file's bytes and the name of the encoding read
/// (`UTF-8`, `UTF-16LE`, `windows-1251`...); `label` as for
/// [`resolve_encoding`], `auto` or empty to guess. Bytes the encoding has no
/// character for become U+FFFD.
///
/// # Errors
/// [`Error::UnknownEncoding`] for a label RVP does not know.
pub fn decode(bytes: &[u8], label: &str) -> Result<(String, &'static str), Error> {
    if let Some((encoding, bom)) = Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[bom..]);
        return Ok((text.into_owned(), encoding.name()));
    }
    let named = label.trim();
    let encoding = if named.is_empty() || named.eq_ignore_ascii_case("auto") {
        guess(bytes)
    } else {
        resolve_encoding(named).ok_or_else(|| Error::UnknownEncoding(named.to_owned()))?
    };
    let (text, _) = encoding.decode_without_bom_handling(bytes);
    Ok((text.into_owned(), encoding.name()))
}

/// The encoding of bytes without a BOM (the module's documentation).
fn guess(bytes: &[u8]) -> &'static Encoding {
    if std::str::from_utf8(bytes).is_ok() {
        return UTF_8;
    }
    let high = bytes.iter().filter(|b| **b >= 0x80).count();
    let ascii_letters = bytes.iter().filter(|b| b.is_ascii_alphabetic()).count();
    // Western text is mostly ASCII letters with a few accented ones (which
    // the multibyte code pages may well read without an error); Russian
    // and CJK text is mostly high bytes.
    if high <= ascii_letters {
        return WINDOWS_1252;
    }
    // Russian in windows-1251: nearly all high bytes are letters (0xC0 to
    // 0xFF); CJK text has many below 0xC0.
    let cyrillic = bytes.iter().filter(|b| **b >= 0xC0).count();
    if cyrillic * 10 >= high * 9 {
        return WINDOWS_1251;
    }
    for encoding in [EUC_KR, SHIFT_JIS, GBK, BIG5] {
        if encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
            .is_some()
        {
            return encoding;
        }
    }
    WINDOWS_1252
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_include_players_code_page_numbers() {
        assert_eq!(resolve_encoding("CP949"), Some(EUC_KR));
        assert_eq!(resolve_encoding("cp1251"), Some(WINDOWS_1251));
        assert_eq!(resolve_encoding("Windows-1251"), Some(WINDOWS_1251));
        assert_eq!(resolve_encoding("cp932"), Some(SHIFT_JIS));
        assert_eq!(resolve_encoding("auto"), None);
        assert_eq!(resolve_encoding("klingon"), None);
    }

    #[test]
    fn the_guess_tells_cyrillic_korean_and_western_apart() {
        let (russian, _, _) = WINDOWS_1251.encode("Привет, как дела? Всё хорошо.");
        assert_eq!(guess(&russian), WINDOWS_1251);
        let (korean, _, _) = EUC_KR.encode("안녕하세요. 반갑습니다.");
        assert_eq!(guess(&korean), EUC_KR);
        let (french, _, _) = WINDOWS_1252.encode("Ça va très bien, merci beaucoup.");
        assert_eq!(guess(&french), WINDOWS_1252);
        assert_eq!(guess("plain".as_bytes()), UTF_8);
    }
}
