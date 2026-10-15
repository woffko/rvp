//! RVP Caps, the caption font of RVP Default (`art/skin/font.svg`):
//! monoline capitals, digits and signs that the skin's sprites stroke at
//! any size, so captions and time digits are vector art like the rest.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::xml;

/// The font source.
pub const FONT_SVG: &str = include_str!("../art/skin/font.svg");

/// Glyph height in font units (the cap height).
const CAP_UNITS: f32 = 10.0;

/// Glyphs by character: their element id and advance in font units.
#[derive(Debug, Clone)]
pub struct Font {
    glyphs: BTreeMap<char, (String, f32)>,
    defs: String,
}

impl Font {
    /// Reads the glyphs from [`FONT_SVG`].
    ///
    /// # Errors
    /// Fails when the source is malformed or a glyph lacks its character
    /// or advance.
    pub fn load() -> Result<Self, String> {
        let root = xml::parse(FONT_SVG)?;
        let defs = root
            .children
            .iter()
            .find(|node| node.name == "defs")
            .ok_or("font.svg has no <defs>")?;
        let mut glyphs = BTreeMap::new();
        let mut text = String::new();
        for node in &defs.children {
            let id = node.attr("id").unwrap_or("");
            if !id.starts_with("glyph-") {
                continue;
            }
            let mut chars = node.attr("data-char").unwrap_or("").chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                return Err(format!("line {}: {id} needs one data-char", node.line));
            };
            let advance: f32 = node
                .attr("data-advance")
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| format!("line {}: {id} needs data-advance", node.line))?;
            glyphs.insert(ch, (id.to_owned(), advance));
            node.write(&mut text);
        }
        Ok(Self { glyphs, defs: text })
    }

    /// The glyph definitions, for the `<defs>` of a document that draws
    /// captions.
    #[must_use]
    pub fn defs(&self) -> &str {
        &self.defs
    }

    /// The width in pixels of `text` with cap height `size` and `spacing`
    /// font units between glyphs.
    ///
    /// # Errors
    /// Fails when the font has no glyph for a character.
    pub fn width(&self, text: &str, size: f32, spacing: f32) -> Result<f32, String> {
        let mut units = 0.0;
        for (index, ch) in text.chars().enumerate() {
            let (_, advance) = self.glyph(ch)?;
            if index > 0 {
                units += spacing;
            }
            units += advance;
        }
        Ok(units * size / CAP_UNITS)
    }

    /// SVG that strokes `text` with its top-left corner at `(x, y)`, cap
    /// height `size` and a stroke `stroke` pixels wide in `color`.
    ///
    /// # Errors
    /// Fails when the font has no glyph for a character.
    pub fn text(
        &self,
        text: &str,
        at: (f32, f32),
        size: f32,
        spacing: f32,
        stroke: f32,
        color: &str,
    ) -> Result<String, String> {
        let scale = size / CAP_UNITS;
        let mut out = String::new();
        let _ = write!(
            out,
            "<g fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" stroke-linecap=\"round\" \
             stroke-linejoin=\"round\" transform=\"translate({} {}) scale({scale})\">",
            stroke / scale,
            at.0,
            at.1
        );
        let mut pen = 0.0;
        for (index, ch) in text.chars().enumerate() {
            let (id, advance) = self.glyph(ch)?;
            if index > 0 {
                pen += spacing;
            }
            let _ = write!(out, "<use href=\"#{id}\" x=\"{pen}\"/>");
            pen += advance;
        }
        out.push_str("</g>");
        Ok(out)
    }

    fn glyph(&self, ch: char) -> Result<(&str, f32), String> {
        self.glyphs
            .get(&ch)
            .map(|(id, advance)| (id.as_str(), *advance))
            .ok_or_else(|| format!("the caption font has no glyph for {ch:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::Font;

    #[test]
    fn has_the_pic_label_cells_and_measures_text() {
        let font = Font::load().expect("font");
        for code in u32::from('%')..=u32::from(':') {
            let ch = char::from_u32(code).expect("char");
            assert!(font.width(&ch.to_string(), 10.0, 0.0).is_ok(), "{ch}");
        }
        // Two 6-unit digits and 2 units of spacing at cap height 10.
        let width = font.width("12", 10.0, 2.0).expect("width");
        assert!((width - 14.0).abs() < 1e-4, "{width}");
        assert!(font.text("A-1", (0.0, 0.0), 8.0, 2.0, 1.2, "#fff").is_ok());
        assert!(font.width("a", 10.0, 0.0).is_err());
    }
}
