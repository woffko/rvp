//! Multi-language strings and the skin string table.
//!
//! Skin texts can hold one variant per code page:
//! `{$CP949=Korean$}{$CP936=Chinese$}{$CP0=English$}`. `CP0` is the default.
//! Texts that start with `$` refer to a `<StringTable Name='$X' String='...'>`
//! entry.

use std::collections::BTreeMap;

use crate::markup::Element;

/// Picks the variant for `code_page` from a multi-language string, falling
/// back to `CP0` and then to the first variant. Plain strings are returned
/// unchanged.
#[must_use]
pub fn select_variant(text: &str, code_page: u32) -> String {
    if !text.contains("{$") {
        return text.to_owned();
    }
    let wanted = format!("CP{code_page}");
    let mut fallback: Option<&str> = None;
    let mut first: Option<&str> = None;
    let mut rest = text;
    while let Some(start) = rest.find("{$") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("$}") else {
            break;
        };
        let segment = &after[..end];
        rest = &after[end + 2..];
        let Some((tag, value)) = segment.split_once('=') else {
            continue;
        };
        if tag.eq_ignore_ascii_case(&wanted) {
            return value.to_owned();
        }
        if tag.eq_ignore_ascii_case("CP0") {
            fallback = Some(value);
        }
        first.get_or_insert(value);
    }
    fallback.or(first).unwrap_or(text).to_owned()
}

/// `<StringTable>` entries of a skin, keyed by name including the leading `$`.
#[derive(Debug, Clone, Default)]
pub struct StringTable {
    entries: BTreeMap<String, String>,
}

impl StringTable {
    /// Collects every `StringTable` element below `root`.
    #[must_use]
    pub fn collect(root: &Element) -> Self {
        let mut entries = BTreeMap::new();
        root.walk(&mut |element| {
            if element.is("StringTable")
                && let (Some(name), Some(value)) =
                    (element.attrs.get("Name"), element.attrs.get("String"))
            {
                entries
                    .entry(name.to_owned())
                    .or_insert_with(|| value.to_owned());
            }
        });
        Self { entries }
    }

    /// Resolves a skin text for display: string table references and
    /// multi-language variants. Unknown references stay as written.
    #[must_use]
    pub fn resolve(&self, text: &str, code_page: u32) -> String {
        let raw = if text.starts_with('$') {
            self.entries.get(text).map_or(text, String::as_str)
        } else {
            text
        };
        select_variant(raw, code_page)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{StringTable, select_variant};
    use crate::markup::parse_document;

    #[test]
    fn selects_code_page_variants() {
        let text = "{$CP949=Korean$}{$CP0=Default Skin$}{$CP1251=Russian$}";
        assert_eq!(select_variant(text, 949), "Korean");
        assert_eq!(select_variant(text, 1251), "Russian");
        assert_eq!(select_variant(text, 1252), "Default Skin");
        assert_eq!(select_variant("{$CP949=Only$}", 0), "Only");
        assert_eq!(select_variant("Plain", 0), "Plain");
    }

    #[test]
    fn resolves_string_table_references() {
        let doc = parse_document("<StringTable Name='$PLAY' String='{$CP949=K$}{$CP0=Play$}' />");
        let table = StringTable::collect(&doc);
        assert_eq!(table.resolve("$PLAY", 0), "Play");
        assert_eq!(table.resolve("$MISSING", 0), "$MISSING");
        assert_eq!(table.resolve("Literal", 0), "Literal");
    }
}
