//! Building RVP skins in memory from other players' formats (M9): pictures
//! cut and joined, and markup in PotPlayer's format written element by
//! element (`crate::winamp::build`, `crate::vlc::build`).

use std::borrow::Borrow;
use std::fmt::Write as _;

use crate::image::Image;

/// Copies `w`x`h` from (`x`, `y`) of `image`; beyond its edges the copy is
/// transparent.
#[must_use]
pub fn crop(image: &Image, x: i32, y: i32, w: i32, h: i32) -> Image {
    let (width, height) = (
        u32::try_from(w.max(1)).unwrap_or(1),
        u32::try_from(h.max(1)).unwrap_or(1),
    );
    let mut out = Image::new(width, height);
    for row in 0..h.max(0) {
        for column in 0..w.max(0) {
            let pixel = image.pixel(x + column, y + row);
            let at = (usize::try_from(row).unwrap_or(0) * width as usize
                + usize::try_from(column).unwrap_or(0))
                * 4;
            out.data_mut()[at..at + 4].copy_from_slice(&pixel);
        }
    }
    out
}

/// The size of [`join`]'s picture; `None` when its sides are no numbers
/// a picture can have.
#[must_use]
pub fn joined_size<I: Borrow<Image>>(parts: &[I], vertical: bool) -> Option<(u32, u32)> {
    let sum = |side: fn(&Image) -> u32| {
        parts
            .iter()
            .try_fold(0_u32, |sum, part| sum.checked_add(side(part.borrow())))
    };
    let most = |side: fn(&Image) -> u32| parts.iter().map(|part| side(part.borrow())).max();
    Some(if vertical {
        (most(Image::width).unwrap_or(1), sum(Image::height)?.max(1))
    } else {
        (sum(Image::width)?.max(1), most(Image::height).unwrap_or(1))
    })
}

/// Pictures side by side (`vertical`: stacked), top-left aligned. The
/// parts are RVP's own or checked ([`joined_size`], [`crate::limits`]);
/// parts whose sizes do not add up give an empty picture.
#[must_use]
pub fn join<I: Borrow<Image>>(parts: &[I], vertical: bool) -> Image {
    let Some((width, height)) = joined_size(parts, vertical) else {
        return Image::new(1, 1);
    };
    let mut out = Image::new(width, height);
    let mut offset = 0_u32;
    for part in parts {
        let part = part.borrow();
        for row in 0..part.height() {
            for column in 0..part.width() {
                let (x, y) = if vertical {
                    (column, offset + row)
                } else {
                    (offset + column, row)
                };
                let pixel = part.pixel(
                    i32::try_from(column).unwrap_or(0),
                    i32::try_from(row).unwrap_or(0),
                );
                let at = (y as usize * width as usize + x as usize) * 4;
                out.data_mut()[at..at + 4].copy_from_slice(&pixel);
            }
        }
        offset += if vertical {
            part.height()
        } else {
            part.width()
        };
    }
    out
}

/// One element of the markup.
#[derive(Clone)]
pub struct El {
    name: &'static str,
    attrs: Vec<(String, String)>,
    children: Vec<El>,
}

impl El {
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    // Numbers and texts alike, by value: the builder's calls stay short.
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn a(mut self, name: &str, value: impl ToString) -> Self {
        // The markup has no escapes: quotes and brackets leave values.
        let value: String = value
            .to_string()
            .chars()
            .filter(|c| !matches!(c, '\'' | '<' | '>'))
            .collect();
        self.attrs.push((name.to_owned(), value));
        self
    }

    #[must_use]
    pub fn rect(self, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.a("Left", x).a("Top", y).a("Width", w).a("Height", h)
    }

    #[must_use]
    pub fn image(self, file: &str, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.a("Image", file)
            .a("ImageLeft", x)
            .a("ImageTop", y)
            .a("ImageWidth", w)
            .a("ImageHeight", h)
    }

    #[must_use]
    pub fn child(mut self, child: El) -> Self {
        self.children.push(child);
        self
    }

    pub fn write(&self, out: &mut String, depth: usize) {
        let indent = "\t".repeat(depth);
        let _ = write!(out, "{indent}<{}", self.name);
        for (name, value) in &self.attrs {
            let _ = write!(out, " {name}='{value}'");
        }
        if self.children.is_empty() {
            out.push_str(" />\n");
            return;
        }
        out.push_str(">\n");
        for child in &self.children {
            child.write(out, depth + 1);
        }
        let _ = writeln!(out, "{indent}</{}>", self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::{join, joined_size};
    use crate::image::Image;

    #[test]
    fn joined_pictures_are_as_long_as_all_and_as_wide_as_the_widest() {
        let mut first = Image::new(2, 3);
        first.fill([1, 1, 1, 255]);
        let mut second = Image::new(4, 1);
        second.fill([2, 2, 2, 255]);
        let parts = [first, second];
        assert_eq!(joined_size(&parts, false), Some((6, 3)));
        assert_eq!(joined_size(&parts, true), Some((4, 4)));
        // Borrowed parts join as owned ones do.
        let borrowed: Vec<&Image> = parts.iter().collect();
        let side = join(&borrowed, false);
        assert_eq!(side, join(&parts, false));
        assert_eq!((side.width(), side.height()), (6, 3));
        assert_eq!(side.pixel(1, 2), [1, 1, 1, 255]);
        assert_eq!(side.pixel(2, 0), [2, 2, 2, 255]);
        assert_eq!(side.pixel(2, 1), [0; 4]);
        let stacked = join(&parts, true);
        assert_eq!(stacked.pixel(3, 3), [2, 2, 2, 255]);
        let none: [Image; 0] = [];
        assert_eq!(joined_size(&none, true), Some((1, 1)));
    }
}
