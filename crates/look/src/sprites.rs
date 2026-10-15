//! The sprites of RVP Default (`art/skin/sprites.xml`) and how each kind
//! is drawn: every sprite becomes SVG at its place in its atlas, so the
//! atlases are drawn by [`crate::svg`] like the rest of RVP's art. The
//! kinds are described at the top of `sprites.xml`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::font::Font;
use crate::xml::{self, Node};

/// The sprite list.
pub const SPRITES_XML: &str = include_str!("../art/skin/sprites.xml");
/// The icons buttons use.
pub const ICONS_SVG: &str = include_str!("../art/skin/icons.svg");

/// Letter spacing of captions, in font units.
const SPACING: f32 = 2.0;
/// Proportions of the wordmark (`art/logo/wordmark.svg`'s viewBox).
const WORDMARK_ASPECT: f32 = 410.0 / 116.0;

/// One atlas: its file and its sprites in document order.
#[derive(Debug, Clone)]
pub struct Atlas {
    pub file: String,
    pub width: u32,
    /// Set for atlases whose sprites sit at fixed places (`ScrollBar.bmp`).
    pub height: Option<u32>,
    pub sprites: Vec<Sprite>,
}

/// One sprite: a cell (or button state) size, a cell count, and where it
/// must go when the atlas fixes it.
#[derive(Debug, Clone)]
pub struct Sprite {
    pub name: String,
    pub cell_width: u32,
    pub cell_height: u32,
    pub cells: u32,
    pub fixed: Option<(u32, u32)>,
    node: Node,
}

impl Sprite {
    /// The width of all cells.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.cell_width * self.cells
    }

    fn attr(&self, name: &str) -> Option<&str> {
        self.node.attr(name)
    }

    fn text(&self, name: &str) -> Result<&str, String> {
        self.attr(name)
            .ok_or_else(|| format!("sprite {}: {name} is missing", self.name))
    }

    fn number(&self, name: &str) -> Result<f32, String> {
        let text = self.text(name)?;
        text.parse()
            .map_err(|_| format!("sprite {}: {name}={text:?} is not a number", self.name))
    }

    fn number_or(&self, name: &str, default: f32) -> Result<f32, String> {
        if self.attr(name).is_some() {
            self.number(name)
        } else {
            Ok(default)
        }
    }
}

/// Reads the atlases and sizes their sprites (captions are as wide as
/// their text).
///
/// # Errors
/// Reports malformed entries, unknown kinds and repeated names.
pub fn load(font: &Font) -> Result<(Vec<Atlas>, Palette), String> {
    let root = xml::parse(SPRITES_XML)?;
    let mut palette = Palette::default();
    let mut atlases = Vec::new();
    let mut names = BTreeMap::new();
    for node in &root.children {
        match node.name.as_str() {
            "color" => {
                let (Some(name), Some(value)) = (node.attr("name"), node.attr("value")) else {
                    return Err(format!("line {}: <color> needs name and value", node.line));
                };
                palette.colors.insert(name.to_owned(), value.to_owned());
            }
            "atlas" => {
                let file = node.attr("file").ok_or("an atlas without file")?.to_owned();
                let width = attr_u32(node, "width")?;
                let height = node
                    .attr("height")
                    .map(|_| attr_u32(node, "height"))
                    .transpose()?;
                let fixed = node.attr("fixed") == Some("1");
                let mut sprites = Vec::with_capacity(node.children.len());
                for item in &node.children {
                    let sprite = sprite(item, font, fixed)?;
                    if names.insert(sprite.name.clone(), item.line).is_some() {
                        return Err(format!(
                            "line {}: sprite {} is repeated",
                            item.line, sprite.name
                        ));
                    }
                    sprites.push(sprite);
                }
                atlases.push(Atlas {
                    file,
                    width,
                    height,
                    sprites,
                });
            }
            other => return Err(format!("line {}: unexpected <{other}>", node.line)),
        }
    }
    Ok((atlases, palette))
}

fn attr_u32(node: &Node, name: &str) -> Result<u32, String> {
    let text = node
        .attr(name)
        .ok_or_else(|| format!("line {}: {name} is missing", node.line))?;
    text.parse()
        .map_err(|_| format!("line {}: {name}={text:?} is not a whole number", node.line))
}

fn sprite(node: &Node, font: &Font, fixed_atlas: bool) -> Result<Sprite, String> {
    let name = node
        .attr("name")
        .ok_or_else(|| format!("line {}: a sprite without name", node.line))?
        .to_owned();
    let kind = node.attr("kind").unwrap_or("");
    let size = |key: &str| attr_u32(node, key);
    let measured = |text: &str, size_key: &str| -> Result<u32, String> {
        let size: f32 = node
            .attr(size_key)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("sprite {name}: {size_key} is missing"))?;
        Ok(to_u32((font.width(text, size, SPACING)? + 2.0).ceil()))
    };
    let (cell_width, cell_height, cells) = match kind {
        "bar" | "fill" | "cover" | "panel" | "track" | "thumb" | "icon" | "labels" | "scroll" => {
            (size("width")?, size("height")?, 1)
        }
        "swatch" => (1, 1, 1),
        "button" if node.attr("style") == Some("logo") => {
            let logo: f32 = node
                .attr("logo-height")
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| format!("sprite {name}: logo-height is missing"))?;
            (
                to_u32((logo * WORDMARK_ASPECT).ceil()) + 4,
                size("height")?,
                4,
            )
        }
        "button" => (size("width")?, size("height")?, 4),
        "digits" => (size("width")?, size("height")?, 22),
        "cells" => {
            let count = node.attr("items").unwrap_or("").split(',').count();
            (
                size("width")?,
                size("height")?,
                u32::try_from(count).unwrap_or(1),
            )
        }
        "caption" => {
            let text = node.attr("text").unwrap_or("");
            (measured(text, "size")?, size("height")?, 1)
        }
        "logo" => {
            let height = size("height")?;
            (
                to_u32((to_f32(height) * WORDMARK_ASPECT).ceil()) + 2,
                height,
                1,
            )
        }
        other => {
            return Err(format!(
                "line {}: sprite {name} has the unknown kind {other:?}",
                node.line
            ));
        }
    };
    let fixed = if fixed_atlas {
        Some((size("x")?, size("y")?))
    } else {
        None
    };
    Ok(Sprite {
        name,
        cell_width,
        cell_height,
        cells,
        fixed,
        node: node.clone(),
    })
}

/// Named colours of `sprites.xml`.
#[derive(Debug, Clone, Default)]
pub struct Palette {
    colors: BTreeMap<String, String>,
}

impl Palette {
    /// A colour by name, or the `#rrggbb` text itself.
    ///
    /// # Errors
    /// Fails for an unknown name.
    pub fn color<'a>(&'a self, value: &'a str) -> Result<&'a str, String> {
        if value.starts_with('#') {
            return Ok(value);
        }
        self.colors
            .get(value)
            .map(String::as_str)
            .ok_or_else(|| format!("unknown colour {value:?}"))
    }
}

/// The shared definitions an atlas document needs: the caption font, the
/// icons and the wordmark.
///
/// # Errors
/// Fails when the icon or logo sources are malformed.
pub fn defs(font: &Font) -> Result<String, String> {
    let mut out = String::from(font.defs());
    let icons = xml::parse(ICONS_SVG)?;
    for node in &icons.children {
        if node.name == "defs" {
            for symbol in &node.children {
                symbol.write(&mut out);
            }
        }
    }
    // The wordmark as a symbol; its letters take `color`.
    let wordmark = xml::parse(crate::brand::WORDMARK_SVG)?;
    let view_box = wordmark
        .attr("viewBox")
        .ok_or("the wordmark has no viewBox")?;
    let mut body = String::new();
    for node in &wordmark.children {
        match node.name.as_str() {
            "defs" => {
                for child in &node.children {
                    child.write(&mut out);
                }
            }
            "title" | "desc" => {}
            _ => node.write(&mut body),
        }
    }
    let _ = write!(
        out,
        "<symbol id=\"rvp-wordmark\" viewBox=\"{view_box}\">{body}</symbol>"
    );
    Ok(out)
}

/// Draws sprites as SVG.
pub struct Painter<'a> {
    pub palette: &'a Palette,
    pub font: &'a Font,
}

/// A rectangle in an atlas, in pixels.
#[derive(Debug, Clone, Copy)]
struct Area {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

impl Area {
    fn right(self) -> f32 {
        self.left + self.width
    }

    fn bottom(self) -> f32 {
        self.top + self.height
    }

    /// The same size `distance` pixels to the right.
    fn shifted(self, distance: f32) -> Self {
        Self {
            left: self.left + distance,
            ..self
        }
    }

    /// Inset by `amount` on every side.
    fn inset(self, amount: f32) -> Self {
        Self {
            left: self.left + amount,
            top: self.top + amount,
            width: self.width - 2.0 * amount,
            height: self.height - 2.0 * amount,
        }
    }
}

/// Colours of one button style in its four states.
struct States {
    fill: [Option<&'static str>; 4],
    border: [Option<&'static str>; 4],
    content: [&'static str; 4],
}

impl Painter<'_> {
    /// Appends the SVG of `sprite` with its top-left corner at `(x, y)`.
    ///
    /// # Errors
    /// Reports missing or wrong attributes and unknown colours.
    pub fn draw(&self, sprite: &Sprite, x: u32, y: u32, out: &mut String) -> Result<(), String> {
        let area = Area {
            left: to_f32(x),
            top: to_f32(y),
            width: to_f32(sprite.cell_width),
            height: to_f32(sprite.cell_height),
        };
        match sprite.attr("kind").unwrap_or("") {
            "bar" => self.bar(sprite, area, out),
            "fill" => self.fill(sprite, area, out),
            "cover" => self.cover(sprite, area, out),
            "panel" => self.panel(sprite, area, out),
            "swatch" => {
                let pixel = Area {
                    width: 1.0,
                    height: 1.0,
                    ..area
                };
                rect(out, pixel, self.color(sprite, "color")?);
                Ok(())
            }
            "button" => {
                for state in 0..4_u32 {
                    self.button(sprite, state, area.shifted(to_f32(state) * area.width), out)?;
                }
                Ok(())
            }
            "track" => self.track(sprite, area, out),
            "thumb" => self.thumb(sprite, area, out),
            "digits" => self.digits(sprite, area, out),
            "cells" => self.cells(sprite, area, out),
            "caption" => {
                let size = sprite.number("size")?;
                let stroke = sprite.number_or("stroke", size * 0.17)?;
                let at = (
                    snap(area.left + 1.0),
                    snap(area.top + (area.height - size) / 2.0),
                );
                let text = self.font.text(
                    sprite.text("text")?,
                    at,
                    size,
                    SPACING,
                    stroke,
                    self.color(sprite, "color")?,
                )?;
                out.push_str(&text);
                Ok(())
            }
            "labels" => self.labels(sprite, area, out),
            "icon" => {
                let size = sprite.number("icon-size")?;
                let stroke = sprite.number_or("stroke", 2.0)?;
                let color = self.color(sprite, "color")?;
                icon(out, sprite.text("icon")?, area, size, stroke, color);
                Ok(())
            }
            "logo" => {
                let _ = write!(
                    out,
                    "<use href=\"#rvp-wordmark\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" color=\"#E3DFD9\"/>",
                    area.left + 1.0,
                    area.top,
                    area.width - 2.0,
                    area.height
                );
                Ok(())
            }
            "scroll" => self.scroll(sprite, area, out),
            other => Err(format!("sprite {}: unknown kind {other:?}", sprite.name)),
        }
    }

    fn color<'s>(&'s self, sprite: &'s Sprite, key: &str) -> Result<&'s str, String> {
        self.palette.color(sprite.text(key)?)
    }

    /// A vertical gradient written as `<linearGradient id="{id}">`.
    fn gradient(out: &mut String, id: &str, units: &str, from: &str, to: &str) {
        let _ = write!(
            out,
            "<linearGradient id=\"{id}\" {units}><stop offset=\"0\" stop-color=\"{from}\"/>\
             <stop offset=\"1\" stop-color=\"{to}\"/></linearGradient>"
        );
    }

    fn bar(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let id = format!("{}-g", sprite.name);
        Self::gradient(
            out,
            &id,
            "x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"",
            self.color(sprite, "top")?,
            self.color(sprite, "bottom")?,
        );
        rect(out, area, &format!("url(#{id})"));
        if sprite.attr("line").is_some() {
            let line = Area {
                height: 1.0,
                ..area
            };
            rect(out, line, self.color(sprite, "line")?);
        }
        if sprite.attr("shade").is_some() {
            let shade = Area {
                top: area.bottom() - 1.0,
                height: 1.0,
                ..area
            };
            rect(out, shade, self.color(sprite, "shade")?);
        }
        Ok(())
    }

    fn fill(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        rect(out, area, self.color(sprite, "color")?);
        let edges = [
            ("left", Area { width: 1.0, ..area }),
            (
                "right",
                Area {
                    left: area.right() - 1.0,
                    width: 1.0,
                    ..area
                },
            ),
            (
                "top",
                Area {
                    height: 1.0,
                    ..area
                },
            ),
            (
                "bottom",
                Area {
                    top: area.bottom() - 1.0,
                    height: 1.0,
                    ..area
                },
            ),
        ];
        for (edge, line) in edges {
            if sprite.attr(edge).is_some() {
                rect(out, line, self.color(sprite, edge)?);
            }
        }
        Ok(())
    }

    /// Rows `offset` and down of a bar `total` high, so that a page of the
    /// bottom bar covers what lies under it with the same gradient.
    fn cover(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let offset = sprite.number("offset")?;
        let total = sprite.number("total")?;
        let id = format!("{}-g", sprite.name);
        let units = format!(
            "gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"{}\" x2=\"0\" y2=\"{}\"",
            area.top - offset,
            area.top - offset + total
        );
        Self::gradient(
            out,
            &id,
            &units,
            self.color(sprite, "top")?,
            self.color(sprite, "bottom")?,
        );
        rect(out, area, &format!("url(#{id})"));
        if sprite.attr("outline").is_some() {
            let radius = sprite.number_or("radius", 4.0)?;
            let fill = if sprite.attr("fill").is_some() {
                self.color(sprite, "fill")?
            } else {
                "none"
            };
            outlined(
                out,
                area.inset(0.5),
                radius,
                fill,
                self.color(sprite, "outline")?,
            );
        }
        Ok(())
    }

    fn panel(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let radius = sprite.number_or("radius", 0.0)?;
        let fill = self.color(sprite, "fill")?;
        let border = self.color(sprite, "border")?;
        if radius > 0.0 {
            outlined(out, area.inset(0.5), radius, fill, border);
            return Ok(());
        }
        rect(out, area, border);
        let inside = area.inset(1.0);
        rect(out, inside, fill);
        if sprite.attr("header").is_some() {
            let header = sprite.number("header")?;
            let id = format!("{}-g", sprite.name);
            Self::gradient(
                out,
                &id,
                "x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"",
                self.color(sprite, "header-top")?,
                self.color(sprite, "header-bottom")?,
            );
            let strip = Area {
                height: header - 2.0,
                ..inside
            };
            rect(out, strip, &format!("url(#{id})"));
            let line = Area {
                top: area.top + header - 1.0,
                height: 1.0,
                ..inside
            };
            rect(out, line, "#0E0F11");
        }
        Ok(())
    }

    fn track(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let thickness = sprite.number("thickness")?;
        let vertical = sprite.attr("vertical") == Some("1");
        let first = self.color(sprite, "color")?;
        let paint = match sprite.attr("color2") {
            Some(second) => {
                let id = format!("{}-g", sprite.name);
                let direction = if vertical {
                    "x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\""
                } else {
                    "x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\""
                };
                Self::gradient(out, &id, direction, self.palette.color(second)?, first);
                format!("url(#{id})")
            }
            None => first.to_owned(),
        };
        let bar = if vertical {
            Area {
                left: area.left + (area.width - thickness) / 2.0,
                width: thickness,
                ..area
            }
        } else {
            Area {
                top: area.top + (area.height - thickness) / 2.0,
                height: thickness,
                ..area
            }
        };
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{paint}\"/>",
            bar.left,
            bar.top,
            bar.width,
            bar.height,
            thickness / 2.0
        );
        Ok(())
    }

    fn thumb(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let fill = self.color(sprite, "color")?;
        let ring = self.color(sprite, "ring")?;
        if sprite.attr("shape") == Some("pill") {
            let inner = area.inset(0.5);
            outlined(out, inner, inner.height / 2.0, fill, ring);
            return Ok(());
        }
        let (cx, cy) = (area.left + area.width / 2.0, area.top + area.height / 2.0);
        let radius = area.width.min(area.height) / 2.0;
        let _ = write!(
            out,
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"#0E0F11\" fill-opacity=\"0.45\"/>\
             <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{}\" fill=\"{fill}\"/>\
             <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{}\" fill=\"{ring}\"/>",
            radius - 1.0,
            (radius / 3.2).max(1.4)
        );
        Ok(())
    }

    /// The 22 cells of a `PicLabel` font: `%` to `:`, each centred.
    fn digits(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let size = sprite.number("size")?;
        let stroke = sprite.number_or("stroke", size * 0.15)?;
        let color = self.color(sprite, "color")?;
        for (index, code) in (u32::from('%')..=u32::from(':')).enumerate() {
            let Some(ch) = char::from_u32(code) else {
                continue;
            };
            let text = ch.to_string();
            let glyph = self.font.width(&text, size, 0.0)?;
            let cell = area.shifted(index_f32(index) * area.width);
            let at = (
                snap(cell.left + (cell.width - glyph) / 2.0),
                snap(cell.top + (cell.height - size) / 2.0),
            );
            out.push_str(&self.font.text(&text, at, size, 0.0, stroke, color)?);
        }
        Ok(())
    }

    fn cells(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let size = sprite.number("size")?;
        let stroke = sprite.number_or("stroke", size * 0.17)?;
        let color = self.color(sprite, "color")?;
        for (index, item) in sprite.text("items")?.split(',').enumerate() {
            let cell = area.shifted(index_f32(index) * area.width);
            if sprite.attr("style") == Some("badge") {
                let badge = Area {
                    left: cell.left + 1.5,
                    top: cell.top + 0.5,
                    width: cell.width - 3.0,
                    height: cell.height - 1.0,
                };
                outlined(out, badge, 2.5, "none", self.color(sprite, "border")?);
            }
            self.centred_text(item, cell, size, stroke, color, out)?;
        }
        Ok(())
    }

    /// Captions centred on the `TEXT@X` positions of `items`.
    fn labels(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        let size = sprite.number("size")?;
        let stroke = sprite.number_or("stroke", size * 0.17)?;
        let color = self.color(sprite, "color")?;
        for item in sprite.text("items")?.split(',') {
            let (text, centre) = item
                .split_once('@')
                .ok_or_else(|| format!("sprite {}: {item:?} is not TEXT@X", sprite.name))?;
            let centre: f32 = centre
                .parse()
                .map_err(|_| format!("sprite {}: {centre:?} is not a number", sprite.name))?;
            let width = self.font.width(text, size, SPACING)?;
            let at = (
                snap(area.left + centre - width / 2.0),
                snap(area.top + (area.height - size) / 2.0),
            );
            out.push_str(&self.font.text(text, at, size, SPACING, stroke, color)?);
        }
        Ok(())
    }

    fn centred_text(
        &self,
        text: &str,
        area: Area,
        size: f32,
        stroke: f32,
        color: &str,
        out: &mut String,
    ) -> Result<(), String> {
        let width = self.font.width(text, size, SPACING)?;
        let at = (
            snap(area.left + (area.width - width) / 2.0),
            snap(area.top + (area.height - size) / 2.0),
        );
        out.push_str(&self.font.text(text, at, size, SPACING, stroke, color)?);
        Ok(())
    }

    /// One state of a button: its shape, then its icon or caption in the
    /// colour the shape leaves for them.
    fn button(
        &self,
        sprite: &Sprite,
        state: u32,
        area: Area,
        out: &mut String,
    ) -> Result<(), String> {
        let index = usize::try_from(state).unwrap_or(0);
        let Some(content) = button_shape(sprite, state, index, area, out)? else {
            return Ok(());
        };
        let icon_name = if state == 2 {
            sprite.attr("icon-on").or_else(|| sprite.attr("icon"))
        } else {
            sprite.attr("icon")
        };
        if let Some(name) = icon_name {
            let size = sprite.number("icon-size")?;
            let stroke = sprite.number_or("stroke", 2.0)?;
            icon(out, name, area, size, stroke, content);
        }
        if let Some(text) = sprite.attr("text") {
            let size = sprite.number("size")?;
            let stroke = sprite.number_or("stroke", size * 0.17)?;
            self.centred_text(text, area, size, stroke, content, out)?;
        }
        Ok(())
    }

    /// `ScrollBar.bmp`'s parts: opaque cells, so no transparent edge
    /// meets the magenta that marks unused pixels.
    fn scroll(&self, sprite: &Sprite, area: Area, out: &mut String) -> Result<(), String> {
        rect(out, area, self.palette.color("list")?);
        match sprite.text("part")? {
            "track" => {
                let rail = Area {
                    left: area.left + 5.0,
                    width: 2.0,
                    ..area
                };
                rect(out, rail, "#24272C");
            }
            "up" => icon(out, "up", area, 12.0, 2.4, "#8E9298"),
            "down" => icon(out, "down", area, 12.0, 2.4, "#8E9298"),
            "thumb" => {
                let thumb = Area {
                    left: area.left + 2.0,
                    width: area.width - 4.0,
                    ..area
                };
                outlined(out, thumb, 3.0, "#4A4F57", "none");
            }
            other => {
                return Err(format!(
                    "sprite {}: unknown scroll part {other:?}",
                    sprite.name
                ));
            }
        }
        Ok(())
    }
}

/// Draws a button state's shape; returns the colour of its icon or
/// caption, or `None` when the style draws everything itself.
fn button_shape(
    sprite: &Sprite,
    state: u32,
    index: usize,
    area: Area,
    out: &mut String,
) -> Result<Option<&'static str>, String> {
    let style = sprite.attr("style").unwrap_or("");
    let content = match style {
        "round" => {
            let (cx, cy) = (area.left + area.width / 2.0, area.top + area.height / 2.0);
            let radius = area.width.min(area.height) / 2.0 - 1.5;
            let fill = ["#2C3036", "#383D45", "#F2741E", "#222428"][index];
            let border = ["#3D424A", "#4B515A", "#FF9B33", "#2A2D32"][index];
            let _ = write!(
                out,
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"{fill}\" stroke=\"{border}\" stroke-width=\"1\"/>"
            );
            ["#E8E5E0", "#FFFFFF", "#FFFFFF", "#5D6168"][index]
        }
        "play" => {
            play_shape(out, &sprite.name, state, index, area);
            if state == 3 { "#6A6E75" } else { "#FFFFFF" }
        }
        "flat" | "close" => flat_shape(out, style == "close", index, area),
        "tool" | "toggle" => boxed_shape(out, style == "toggle", state, index, area),
        "field" => {
            let colors = States {
                fill: [
                    Some("#15171A"),
                    Some("#181A1E"),
                    Some("#15171A"),
                    Some("#15171A"),
                ],
                border: [
                    Some("#3A3F48"),
                    Some("#555B64"),
                    Some("#F2741E"),
                    Some("#2A2D32"),
                ],
                content: ["#9A9EA4", "#E8E5E0", "#FFB547", "#5D6168"],
            };
            rounded(out, area, 3.0, &colors, index);
            let arrow = Area {
                left: area.right() - 18.0,
                width: 16.0,
                ..area
            };
            icon(out, "down", arrow, 12.0, 2.0, colors.content[index]);
            return Ok(None);
        }
        "grip" => {
            let fill = ["#3E434B", "#F2741E", "#FFB547", "#2A2D32"][index];
            let bar = (area.width - 2.0).max(2.0);
            let _ = write!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{bar}\" height=\"{}\" rx=\"{}\" fill=\"{fill}\"/>",
                area.left + (area.width - bar) / 2.0,
                area.top + 4.0,
                area.height - 8.0,
                bar / 2.0
            );
            return Ok(None);
        }
        "tab" => {
            tab_shape(out, state, area);
            return Ok(None);
        }
        "logo" => {
            let logo = sprite.number("logo-height")?;
            let shift = if state == 2 { 1.0 } else { 0.0 };
            let letters = ["#E3DFD9", "#FFFFFF", "#FFB547", "#E3DFD9"][index];
            let opacity = if state == 3 { " opacity=\"0.45\"" } else { "" };
            let _ = write!(
                out,
                "<use href=\"#rvp-wordmark\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{logo}\" color=\"{letters}\"{opacity}/>",
                area.left + 2.0,
                snap_whole(area.top + (area.height - logo) / 2.0) + shift,
                area.width - 4.0
            );
            return Ok(None);
        }
        other => {
            return Err(format!(
                "sprite {}: unknown button style {other:?}",
                sprite.name
            ));
        }
    };
    Ok(Some(content))
}

/// A flat button: no shape until hovered or pressed; `close` turns red.
fn flat_shape(out: &mut String, close: bool, index: usize, area: Area) -> &'static str {
    let fill = if close {
        [None, Some("#D4471C"), Some("#A8361A"), None]
    } else {
        [None, Some("#33373E"), Some("#26292E"), None]
    };
    if let Some(fill) = fill[index] {
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{fill}\"/>",
            area.left + 2.0,
            area.top + 2.0,
            area.width - 4.0,
            area.height - 4.0
        );
    }
    if close {
        ["#AEB1B6", "#FFFFFF", "#FFFFFF", "#5D6168"][index]
    } else {
        ["#AEB1B6", "#F4F1EC", "#FFB547", "#5D6168"][index]
    }
}

/// A tool button (solid, accent when pressed) or a toggle (outlined, an
/// accent tint while on).
fn boxed_shape(
    out: &mut String,
    toggle: bool,
    state: u32,
    index: usize,
    area: Area,
) -> &'static str {
    if !toggle {
        let colors = States {
            fill: [
                Some("#2A2E34"),
                Some("#343940"),
                Some("#F2741E"),
                Some("#212327"),
            ],
            border: [
                Some("#3C4149"),
                Some("#4C525B"),
                Some("#FF9B33"),
                Some("#2A2D32"),
            ],
            content: ["#E8E5E0", "#FFFFFF", "#1A1C20", "#5D6168"],
        };
        rounded(out, area, 4.0, &colors, index);
        return colors.content[index];
    }
    let colors = States {
        fill: [None, Some("#262A2F"), Some("#F2741E"), None],
        border: [
            Some("#3E434B"),
            Some("#5A606A"),
            Some("#F2741E"),
            Some("#2A2D32"),
        ],
        content: ["#9A9EA4", "#E8E5E0", "#FFB547", "#5D6168"],
    };
    rounded(out, area, 3.0, &colors, index);
    if state == 2 {
        // The "on" fill: the accent above, darkened to a tint.
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"#1A1C20\" fill-opacity=\"0.82\"/>",
            area.left + 1.0,
            area.top + 1.0,
            area.width - 2.0,
            area.height - 2.0
        );
    }
    colors.content[index]
}

/// The big play button: a gradient disc over a soft shadow, with a faint
/// inner rim.
fn play_shape(out: &mut String, name: &str, state: u32, index: usize, area: Area) {
    let (top, bottom) = [
        ("#FFA244", "#E8661A"),
        ("#FFB65A", "#F2741E"),
        ("#E8661A", "#B63A12"),
        ("#3A3E45", "#2C3036"),
    ][index];
    let (cx, cy) = (area.left + area.width / 2.0, area.top + area.height / 2.0);
    let radius = area.width.min(area.height) / 2.0 - 2.0;
    let id = format!("{name}-{state}");
    let _ = write!(
        out,
        "<linearGradient id=\"{id}\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\">\
         <stop offset=\"0\" stop-color=\"{top}\"/><stop offset=\"1\" stop-color=\"{bottom}\"/></linearGradient>\
         <circle cx=\"{cx}\" cy=\"{}\" r=\"{}\" fill=\"#0B0C0E\" fill-opacity=\"0.55\"/>\
         <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"url(#{id})\"/>\
         <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{}\" fill=\"none\" stroke=\"#FFFFFF\" stroke-opacity=\"0.18\" stroke-width=\"1\"/>",
        cy + 1.0,
        radius + 1.0,
        radius - 0.5
    );
}

/// An album tab: plain with a separator, lighter on hover, raised with an
/// accent line when selected (the engine uses cells 0 and 2).
fn tab_shape(out: &mut String, state: u32, area: Area) {
    match state {
        1 => rect(
            out,
            Area {
                height: area.height - 1.0,
                ..area
            },
            "#22252A",
        ),
        2 => {
            rect(out, area, "#26292F");
            rect(
                out,
                Area {
                    height: 2.0,
                    ..area
                },
                "#F2741E",
            );
        }
        _ => rect(
            out,
            Area {
                left: area.right() - 1.0,
                top: area.top + 6.0,
                width: 1.0,
                height: area.height - 12.0,
            },
            "#2E3238",
        ),
    }
}

fn rect(out: &mut String, area: Area, fill: &str) {
    let _ = write!(
        out,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\"/>",
        area.left, area.top, area.width, area.height
    );
}

/// A rounded rectangle with a one-pixel border.
fn outlined(out: &mut String, area: Area, radius: f32, fill: &str, border: &str) {
    let _ = write!(
        out,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" fill=\"{fill}\" stroke=\"{border}\" stroke-width=\"1\"/>",
        area.left, area.top, area.width, area.height
    );
}

/// A rounded rectangle with a one-pixel border inside `area`, in a
/// state's colours.
fn rounded(out: &mut String, area: Area, radius: f32, colors: &States, index: usize) {
    outlined(
        out,
        area.inset(0.5),
        radius,
        colors.fill[index].unwrap_or("none"),
        colors.border[index].unwrap_or("none"),
    );
}

/// An icon of `size` pixels centred in `area` on whole pixels.
fn icon(out: &mut String, name: &str, area: Area, size: f32, stroke: f32, color: &str) {
    let _ = write!(
        out,
        "<use href=\"#icon-{name}\" x=\"{}\" y=\"{}\" width=\"{size}\" height=\"{size}\" color=\"{color}\" stroke-width=\"{stroke}\"/>",
        snap_whole(area.left + (area.width - size) / 2.0),
        snap_whole(area.top + (area.height - size) / 2.0)
    );
}

/// A coordinate on a pixel centre, where one-pixel strokes are sharpest.
fn snap(value: f32) -> f32 {
    value.floor() + 0.5
}

fn snap_whole(value: f32) -> f32 {
    value.round()
}

// Sprite sizes and cell indices are a few hundred at most: the
// conversion is exact.
#[allow(clippy::cast_precision_loss)]
fn to_f32(value: u32) -> f32 {
    value as f32
}

fn index_f32(index: usize) -> f32 {
    to_f32(u32::try_from(index).unwrap_or(u32::MAX))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_u32(value: f32) -> u32 {
    // Callers pass small positive sizes.
    value.max(0.0) as u32
}
