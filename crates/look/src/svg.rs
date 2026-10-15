//! Renders the SVG subset that RVP's own art is drawn in.
//!
//! RVP's logo, icons and default skin are SVG files in the repository,
//! rasterized here without an SVG library: [`crate::xml`] reads them and
//! `zeno` (already in the tree as `swash`'s glyph rasterizer) fills and
//! strokes their paths. The subset is what that art needs:
//!
//! * elements `svg`, `g`, `defs`, `symbol`, `use`, `path`, `rect`,
//!   `circle`, `ellipse`, `line`, `polyline`, `polygon`, `linearGradient`,
//!   `radialGradient`, `stop`, `clipPath`, `filter` with `feGaussianBlur`,
//!   `title` and `desc`;
//! * presentation attributes, no `style` or CSS: `fill`, `fill-opacity`,
//!   `fill-rule`, `stroke`, `stroke-width`, `stroke-opacity`,
//!   `stroke-linecap`, `stroke-linejoin`, `stroke-miterlimit`, `color`,
//!   `opacity`, `transform`, `clip-path`, `clip-rule`, `filter` and
//!   `display="none"`;
//! * paints `none`, `#rgb`, `#rrggbb`, `black`, `white`, `transparent`,
//!   `currentColor` and `url(#id)`; gradients in `objectBoundingBox` or
//!   `userSpaceOnUse` units, with `gradientTransform`, `href` to another
//!   gradient's stops, and pad spreading. Radial gradients have no focal
//!   point.
//!
//! Anything else is an error instead of a silent difference from what a
//! browser shows for the same file. Compositing is source-over on
//! premultiplied, gamma-encoded values, as browsers do it; group opacity,
//! clipping and blur work on a separate layer.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rvp_skin::Image;
use zeno::{Cap, Fill, Join, Mask, Placement, Point, Stroke, Transform};

use crate::xml::{self, Node};

const ELEMENTS: [&str; 20] = [
    "svg",
    "g",
    "defs",
    "symbol",
    "use",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "linearGradient",
    "radialGradient",
    "stop",
    "clipPath",
    "filter",
    "feGaussianBlur",
    "title",
    "desc",
];

const ATTRIBUTES: [&str; 50] = [
    "id",
    "xmlns",
    "xmlns:xlink",
    "version",
    "width",
    "height",
    "viewBox",
    "preserveAspectRatio",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "fx",
    "fy",
    "d",
    "points",
    "href",
    "xlink:href",
    "transform",
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "opacity",
    "color",
    "clip-path",
    "clip-rule",
    "clipPathUnits",
    "filter",
    "display",
    "gradientUnits",
    "gradientTransform",
    "spreadMethod",
    "offset",
    "stop-color",
    "stop-opacity",
    "stdDeviation",
    "in",
];

/// A parsed and checked SVG document.
#[derive(Debug, Clone)]
pub struct Document {
    root: Node,
}

impl Document {
    /// Parses a document and checks that it stays inside the subset.
    ///
    /// # Errors
    /// Reports malformed XML, unknown elements and attributes, and broken
    /// references.
    pub fn parse(text: &str) -> Result<Self, String> {
        let root = xml::parse(text)?;
        if root.name != "svg" {
            return Err(format!("the root element is <{}>, not <svg>", root.name));
        }
        let mut problem = None;
        let mut ids = BTreeMap::new();
        root.walk(&mut |node| {
            if problem.is_some() {
                return;
            }
            if !ELEMENTS.contains(&node.name.as_str()) {
                problem = Some(format!(
                    "line {}: <{}> is not supported",
                    node.line, node.name
                ));
                return;
            }
            for (key, _) in &node.attrs {
                if !ATTRIBUTES.contains(&key.as_str()) && !key.starts_with("data-") {
                    problem = Some(format!(
                        "line {}: <{}> has the unsupported attribute {key}",
                        node.line, node.name
                    ));
                    return;
                }
            }
            if let Some(id) = node.attr("id")
                && ids.insert(id.to_owned(), node.line).is_some()
            {
                problem = Some(format!("line {}: the id {id} is repeated", node.line));
            }
        });
        if let Some(problem) = problem {
            return Err(problem);
        }
        let document = Self { root };
        document.view_box()?;
        Ok(document)
    }

    /// The `viewBox` (or `0 0 width height`): `[x, y, width, height]`.
    ///
    /// # Errors
    /// Fails when neither gives a positive size.
    pub fn view_box(&self) -> Result<[f32; 4], String> {
        if let Some(text) = self.root.attr("viewBox") {
            let values = numbers(text)?;
            if let [x, y, width, height] = values[..]
                && width > 0.0
                && height > 0.0
            {
                return Ok([x, y, width, height]);
            }
            return Err(format!(
                "the viewBox {text:?} is not four numbers with a size"
            ));
        }
        let width = self.root.attr("width").map(length).transpose()?;
        let height = self.root.attr("height").map(length).transpose()?;
        match (width, height) {
            (Some(width), Some(height)) if width > 0.0 && height > 0.0 => {
                Ok([0.0, 0.0, width, height])
            }
            _ => Err("the document has no viewBox and no size".to_owned()),
        }
    }

    /// Renders the document into a `width` x `height` image, its viewBox
    /// scaled to fit and centred.
    ///
    /// # Errors
    /// Reports content outside the subset met while drawing (bad numbers,
    /// broken references).
    pub fn render(&self, width: u32, height: u32) -> Result<Image, String> {
        let [x, y, box_width, box_height] = self.view_box()?;
        let (pixels_wide, pixels_high) = (to_f32(to_usize(width)), to_f32(to_usize(height)));
        let scale = (pixels_wide / box_width).min(pixels_high / box_height);
        let dx = (pixels_wide - box_width * scale) / 2.0;
        let dy = (pixels_high - box_height * scale) / 2.0;
        let base = Transform::translation(-x, -y)
            .then_scale(scale, scale)
            .then_translate(dx, dy);
        let mut renderer = Renderer::new(&self.root, width, height);
        let mut canvas = Canvas::new(renderer.width, renderer.height);
        // The root's presentation attributes (`color`, `fill`, ...) apply
        // to everything below it.
        let style = Style::initial().apply(&self.root)?;
        for child in &self.root.children {
            renderer.element(child, &style, base, &mut canvas)?;
        }
        Ok(canvas.into_image(width, height))
    }
}

/// An RGBA colour with straight alpha, components 0 to 1.
type Rgba = [f32; 4];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Paint<'a> {
    None,
    Color(Rgba),
    Current,
    Url(&'a str),
}

/// Inherited presentation attributes.
#[derive(Debug, Clone, Copy)]
struct Style<'a> {
    color: Rgba,
    fill: Paint<'a>,
    fill_opacity: f32,
    fill_rule: Fill,
    stroke: Paint<'a>,
    stroke_opacity: f32,
    stroke_width: f32,
    cap: Cap,
    join: Join,
    miter_limit: f32,
    clip_rule: Fill,
}

impl<'a> Style<'a> {
    fn initial() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            fill: Paint::Color([0.0, 0.0, 0.0, 1.0]),
            fill_opacity: 1.0,
            fill_rule: Fill::NonZero,
            stroke: Paint::None,
            stroke_opacity: 1.0,
            stroke_width: 1.0,
            cap: Cap::Butt,
            join: Join::Miter,
            miter_limit: 4.0,
            clip_rule: Fill::NonZero,
        }
    }

    /// This style with the element's own presentation attributes applied.
    fn apply(mut self, node: &'a Node) -> Result<Self, String> {
        let at = |message: String| format!("line {}: {message}", node.line);
        for (key, value) in &node.attrs {
            let value = value.as_str();
            match key.as_str() {
                "color" => self.color = color(value).map_err(at)?,
                "fill" => self.fill = paint(value).map_err(at)?,
                "fill-opacity" => self.fill_opacity = fraction(value).map_err(at)?,
                "fill-rule" => self.fill_rule = rule(value).map_err(at)?,
                "clip-rule" => self.clip_rule = rule(value).map_err(at)?,
                "stroke" => self.stroke = paint(value).map_err(at)?,
                "stroke-opacity" => self.stroke_opacity = fraction(value).map_err(at)?,
                "stroke-width" => self.stroke_width = length(value).map_err(at)?,
                "stroke-miterlimit" => self.miter_limit = number(value).map_err(at)?,
                "stroke-linecap" => {
                    self.cap = match value {
                        "butt" => Cap::Butt,
                        "round" => Cap::Round,
                        "square" => Cap::Square,
                        other => return Err(at(format!("unknown stroke-linecap {other}"))),
                    };
                }
                "stroke-linejoin" => {
                    self.join = match value {
                        "miter" => Join::Miter,
                        "round" => Join::Round,
                        "bevel" => Join::Bevel,
                        other => return Err(at(format!("unknown stroke-linejoin {other}"))),
                    };
                }
                _ => {}
            }
        }
        Ok(self)
    }
}

fn rule(value: &str) -> Result<Fill, String> {
    match value {
        "nonzero" => Ok(Fill::NonZero),
        "evenodd" => Ok(Fill::EvenOdd),
        other => Err(format!("unknown fill rule {other}")),
    }
}

/// A canvas of premultiplied RGBA values.
struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<Rgba>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0.0; 4]; width * height],
        }
    }

    /// Draws `shader` through a coverage mask placed on the canvas.
    fn fill_mask(&mut self, mask: &[u8], placement: Placement, shader: &Shader) {
        let mask_width = to_usize(placement.width);
        for row in 0..to_usize(placement.height) {
            let Some(y) = offset(placement.top, row, self.height) else {
                continue;
            };
            for column in 0..mask_width {
                let coverage = mask[row * mask_width + column];
                if coverage == 0 {
                    continue;
                }
                let Some(x) = offset(placement.left, column, self.width) else {
                    continue;
                };
                let source = shader.at(x, y);
                let alpha = f32::from(coverage) / 255.0;
                let target = &mut self.pixels[y * self.width + x];
                let keep = 1.0 - source[3] * alpha;
                for channel in 0..4 {
                    target[channel] = source[channel] * alpha + target[channel] * keep;
                }
            }
        }
    }

    /// Draws another canvas over this one.
    fn composite(&mut self, layer: &Canvas, opacity: f32) {
        for (target, source) in self.pixels.iter_mut().zip(&layer.pixels) {
            let keep = 1.0 - source[3] * opacity;
            for channel in 0..4 {
                target[channel] = source[channel] * opacity + target[channel] * keep;
            }
        }
    }

    /// Multiplies every pixel by a coverage value.
    fn clip(&mut self, coverage: &[f32]) {
        for (pixel, &keep) in self.pixels.iter_mut().zip(coverage) {
            for channel in pixel.iter_mut() {
                *channel *= keep;
            }
        }
    }

    /// An approximate Gaussian blur: three box blurs in each direction.
    fn blur(&mut self, sigma: f32) {
        if sigma <= 0.0 {
            return;
        }
        for radius in box_radii(sigma) {
            let mut scratch = vec![[0.0; 4]; self.pixels.len()];
            box_pass(
                &self.pixels,
                &mut scratch,
                self.width,
                self.height,
                radius,
                true,
            );
            box_pass(
                &scratch,
                &mut self.pixels,
                self.width,
                self.height,
                radius,
                false,
            );
        }
    }

    fn into_image(self, width: u32, height: u32) -> Image {
        let mut image = Image::new(width, height);
        for (out, pixel) in image
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&self.pixels)
        {
            let alpha = pixel[3].clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }
            for channel in 0..3 {
                out[channel] = to_byte(pixel[channel] / alpha);
            }
            out[3] = to_byte(alpha);
        }
        image
    }
}

/// The canvas index of `start + step` when it lies in `0..limit`.
fn offset(start: i32, step: usize, limit: usize) -> Option<usize> {
    let index = i64::from(start) + i64::try_from(step).ok()?;
    usize::try_from(index).ok().filter(|&index| index < limit)
}

/// Radii of three box blurs that together approximate a Gaussian with
/// standard deviation `sigma` (the usual construction from box widths).
fn box_radii(sigma: f32) -> [usize; 3] {
    let variance = 12.0 * sigma * sigma;
    let ideal = (variance / 3.0 + 1.0).sqrt();
    // An odd box width at or below the ideal one.
    let mut lower = to_usize_f(ideal.floor()).max(1);
    if lower.is_multiple_of(2) {
        lower -= 1;
    }
    let lower_f = to_f32(lower);
    let narrow_count = ((variance - 3.0 * lower_f * lower_f - 12.0 * lower_f - 9.0)
        / (-4.0 * lower_f - 4.0))
        .round();
    let mut radii = [0; 3];
    for (index, radius) in radii.iter_mut().enumerate() {
        let width = if to_f32(index) < narrow_count {
            lower
        } else {
            lower + 2
        };
        *radius = (width - 1) / 2;
    }
    radii
}

/// One box blur pass along rows (`horizontal`) or columns; pixels beyond
/// the edges count as transparent.
fn box_pass(
    source: &[Rgba],
    target: &mut [Rgba],
    width: usize,
    height: usize,
    radius: usize,
    horizontal: bool,
) {
    let (lines, length) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };
    let index = |line: usize, position: usize| {
        if horizontal {
            line * width + position
        } else {
            position * width + line
        }
    };
    let scale = 1.0 / to_f32(2 * radius + 1);
    for line in 0..lines {
        let mut sum = [0.0_f32; 4];
        for position in 0..radius.min(length) {
            let pixel = source[index(line, position)];
            for channel in 0..4 {
                sum[channel] += pixel[channel];
            }
        }
        for position in 0..length {
            if position + radius < length {
                let pixel = source[index(line, position + radius)];
                for channel in 0..4 {
                    sum[channel] += pixel[channel];
                }
            }
            if position > radius {
                let pixel = source[index(line, position - radius - 1)];
                for channel in 0..4 {
                    sum[channel] -= pixel[channel];
                }
            }
            let out = &mut target[index(line, position)];
            for channel in 0..4 {
                out[channel] = (sum[channel] * scale).max(0.0);
            }
        }
    }
}

/// A resolved paint: a colour, or a gradient evaluated per pixel.
enum Shader {
    Solid(Rgba),
    Linear {
        /// Device pixels to the gradient's own space.
        to_space: Transform,
        start: Point,
        end: Point,
        stops: Vec<(f32, Rgba)>,
    },
    Radial {
        to_space: Transform,
        centre: Point,
        radius: f32,
        stops: Vec<(f32, Rgba)>,
    },
}

impl Shader {
    /// The premultiplied colour at the centre of device pixel `(x, y)`.
    fn at(&self, x: usize, y: usize) -> Rgba {
        let device = Point::new(to_f32(x) + 0.5, to_f32(y) + 0.5);
        match self {
            Self::Solid(color) => *color,
            Self::Linear {
                to_space,
                start,
                end,
                stops,
            } => {
                let point = to_space.transform_point(device);
                let axis = Point::new(end.x - start.x, end.y - start.y);
                let length = axis.x * axis.x + axis.y * axis.y;
                let t = if length > 0.0 {
                    ((point.x - start.x) * axis.x + (point.y - start.y) * axis.y) / length
                } else {
                    1.0
                };
                sample(stops, t)
            }
            Self::Radial {
                to_space,
                centre,
                radius,
                stops,
            } => {
                let point = to_space.transform_point(device);
                let distance = ((point.x - centre.x).powi(2) + (point.y - centre.y).powi(2)).sqrt();
                let t = if *radius > 0.0 {
                    distance / radius
                } else {
                    1.0
                };
                sample(stops, t)
            }
        }
    }
}

/// The colour of a gradient at `t` (padded at both ends); stops hold
/// premultiplied colours, so the blend is premultiplied too.
fn sample(stops: &[(f32, Rgba)], t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let Some(first) = stops.first() else {
        return [0.0; 4];
    };
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if t <= to.0 {
            let span = to.0 - from.0;
            let k = if span > 0.0 { (t - from.0) / span } else { 1.0 };
            return std::array::from_fn(|channel| {
                from.1[channel] + (to.1[channel] - from.1[channel]) * k
            });
        }
    }
    stops.last().map_or([0.0; 4], |last| last.1)
}

fn premultiply(color: Rgba, opacity: f32) -> Rgba {
    let alpha = color[3] * opacity;
    [color[0] * alpha, color[1] * alpha, color[2] * alpha, alpha]
}

struct Renderer<'a> {
    ids: BTreeMap<&'a str, &'a Node>,
    width: usize,
    height: usize,
    /// Guards against `use` cycles.
    depth: usize,
}

impl<'a> Renderer<'a> {
    fn new(root: &'a Node, width: u32, height: u32) -> Self {
        let mut ids = BTreeMap::new();
        root.walk(&mut |node| {
            if let Some(id) = node.attr("id") {
                ids.insert(id, node);
            }
        });
        Self {
            ids,
            width: to_usize(width),
            height: to_usize(height),
            depth: 0,
        }
    }

    fn lookup(&self, reference: &str, node: &Node) -> Result<&'a Node, String> {
        let id = reference
            .strip_prefix('#')
            .ok_or_else(|| format!("line {}: {reference:?} is not a local reference", node.line))?;
        self.ids
            .get(id)
            .copied()
            .ok_or_else(|| format!("line {}: nothing has the id {id}", node.line))
    }

    /// Draws one element and its children.
    fn element(
        &mut self,
        node: &'a Node,
        inherited: &Style<'a>,
        parent: Transform,
        canvas: &mut Canvas,
    ) -> Result<(), String> {
        if node.attr("display") == Some("none") {
            return Ok(());
        }
        if matches!(
            node.name.as_str(),
            "defs"
                | "symbol"
                | "linearGradient"
                | "radialGradient"
                | "stop"
                | "clipPath"
                | "filter"
                | "feGaussianBlur"
                | "title"
                | "desc"
        ) {
            return Ok(());
        }
        let style = inherited.apply(node)?;
        let ctm = match node.attr("transform") {
            Some(text) => transform(text)
                .map_err(|message| format!("line {}: {message}", node.line))?
                .then(&parent),
            None => parent,
        };
        let opacity = node
            .attr("opacity")
            .map(fraction)
            .transpose()?
            .unwrap_or(1.0);
        let clip = node
            .attr("clip-path")
            .map(|value| self.reference(value, node))
            .transpose()?;
        let filter = node
            .attr("filter")
            .map(|value| self.reference(value, node))
            .transpose()?;
        if opacity >= 1.0 && clip.is_none() && filter.is_none() {
            return self.content(node, &style, ctm, canvas);
        }
        let mut layer = Canvas::new(self.width, self.height);
        self.content(node, &style, ctm, &mut layer)?;
        if let Some(filter) = filter {
            apply_filter(filter, ctm, &mut layer)?;
        }
        if let Some(clip) = clip {
            let coverage = self.clip_coverage(clip, ctm)?;
            layer.clip(&coverage);
        }
        canvas.composite(&layer, opacity);
        Ok(())
    }

    /// The element `url(#id)` points at.
    fn reference(&self, value: &str, node: &Node) -> Result<&'a Node, String> {
        let inner = value
            .strip_prefix("url(")
            .and_then(|rest| rest.strip_suffix(')'))
            .ok_or_else(|| format!("line {}: {value:?} is not url(#id)", node.line))?;
        self.lookup(inner.trim(), node)
    }

    fn content(
        &mut self,
        node: &'a Node,
        style: &Style<'a>,
        ctm: Transform,
        canvas: &mut Canvas,
    ) -> Result<(), String> {
        match node.name.as_str() {
            "g" => {
                for child in &node.children {
                    self.element(child, style, ctm, canvas)?;
                }
                Ok(())
            }
            "use" => self.use_element(node, style, ctm, canvas),
            "svg" => Err(format!("line {}: nested <svg> is not supported", node.line)),
            _ => {
                let data = path_data(node)?;
                self.shape(&data, style, ctm, canvas)
            }
        }
    }

    fn use_element(
        &mut self,
        node: &'a Node,
        style: &Style<'a>,
        ctm: Transform,
        canvas: &mut Canvas,
    ) -> Result<(), String> {
        let target_ref = node
            .attr("href")
            .or_else(|| node.attr("xlink:href"))
            .ok_or_else(|| format!("line {}: <use> without href", node.line))?;
        let target = self.lookup(target_ref, node)?;
        if self.depth > 16 {
            return Err(format!("line {}: <use> nests too deeply", node.line));
        }
        let x = attr_length(node, "x", 0.0)?;
        let y = attr_length(node, "y", 0.0)?;
        let mut placed = Transform::translation(x, y).then(&ctm);
        self.depth += 1;
        let result = if target.name == "symbol" {
            if let Some(text) = target.attr("viewBox") {
                let values = numbers(text)?;
                let [bx, by, bw, bh] = values[..] else {
                    return Err(format!("line {}: bad viewBox", target.line));
                };
                let width = attr_length(node, "width", bw)?;
                let height = attr_length(node, "height", bh)?;
                let fit = match target.attr("preserveAspectRatio") {
                    None | Some("xMidYMid meet") => {
                        let scale = (width / bw).min(height / bh);
                        Transform::translation(-bx, -by)
                            .then_scale(scale, scale)
                            .then_translate((width - bw * scale) / 2.0, (height - bh * scale) / 2.0)
                    }
                    Some("none") => {
                        Transform::translation(-bx, -by).then_scale(width / bw, height / bh)
                    }
                    Some(other) => {
                        return Err(format!(
                            "line {}: preserveAspectRatio {other:?} is not supported",
                            target.line
                        ));
                    }
                };
                placed = fit.then(&placed);
            }
            let style = style.apply(target)?;
            let mut outcome = Ok(());
            for child in &target.children {
                outcome = self.element(child, &style, placed, canvas);
                if outcome.is_err() {
                    break;
                }
            }
            outcome
        } else {
            self.element(target, style, placed, canvas)
        };
        self.depth -= 1;
        result
    }

    fn shape(
        &self,
        data: &str,
        style: &Style<'a>,
        ctm: Transform,
        canvas: &mut Canvas,
    ) -> Result<(), String> {
        if data.is_empty() {
            return Ok(());
        }
        let bounds = zeno::bounds(data, Fill::NonZero, None);
        let bbox = [bounds.min.x, bounds.min.y, bounds.width(), bounds.height()];
        if !matches!(style.fill, Paint::None) {
            let shader = self.shader(style.fill, style.color, style.fill_opacity, bbox, ctm)?;
            let (mask, placement) = Mask::new(data)
                .style(style.fill_rule)
                .transform(Some(ctm))
                .render();
            canvas.fill_mask(&mask, placement, &shader);
        }
        if !matches!(style.stroke, Paint::None) && style.stroke_width > 0.0 {
            let shader = self.shader(style.stroke, style.color, style.stroke_opacity, bbox, ctm)?;
            let mut stroke = Stroke::new(style.stroke_width);
            stroke
                .cap(style.cap)
                .join(style.join)
                .miter_limit(style.miter_limit);
            let (mask, placement) = Mask::new(data).style(stroke).transform(Some(ctm)).render();
            canvas.fill_mask(&mask, placement, &shader);
        }
        Ok(())
    }

    fn shader(
        &self,
        paint: Paint<'a>,
        current: Rgba,
        opacity: f32,
        bbox: [f32; 4],
        ctm: Transform,
    ) -> Result<Shader, String> {
        let reference = match paint {
            Paint::None => return Ok(Shader::Solid([0.0; 4])),
            Paint::Color(color) => return Ok(Shader::Solid(premultiply(color, opacity))),
            Paint::Current => return Ok(Shader::Solid(premultiply(current, opacity))),
            Paint::Url(reference) => reference,
        };
        let id = reference.trim_start_matches('#');
        let gradient = self
            .ids
            .get(id)
            .copied()
            .ok_or_else(|| format!("no gradient has the id {id}"))?;
        let at = |message: String| format!("line {}: {message}", gradient.line);
        let stops = self.stops(gradient, opacity, 0)?;
        let units_box = match gradient.attr("gradientUnits") {
            None | Some("objectBoundingBox") => true,
            Some("userSpaceOnUse") => false,
            Some(other) => return Err(at(format!("unknown gradientUnits {other}"))),
        };
        if let Some(spread) = gradient.attr("spreadMethod")
            && spread != "pad"
        {
            return Err(at(format!("spreadMethod {spread} is not supported")));
        }
        // Gradient space to user space, then to device pixels.
        let mut to_device = ctm;
        if units_box {
            to_device =
                Transform::new(bbox[2], 0.0, 0.0, bbox[3], bbox[0], bbox[1]).then(&to_device);
        }
        if let Some(text) = gradient.attr("gradientTransform") {
            to_device = transform(text).map_err(at)?.then(&to_device);
        }
        let to_space = to_device.invert().unwrap_or(Transform::IDENTITY);
        let coordinate = |name: &str, default: f32| -> Result<f32, String> {
            gradient
                .attr(name)
                .map_or(Ok(default), |value| {
                    if units_box {
                        fraction_or_number(value)
                    } else {
                        length(value)
                    }
                })
                .map_err(at)
        };
        match gradient.name.as_str() {
            "linearGradient" => Ok(Shader::Linear {
                to_space,
                start: Point::new(coordinate("x1", 0.0)?, coordinate("y1", 0.0)?),
                end: Point::new(coordinate("x2", 1.0)?, coordinate("y2", 0.0)?),
                stops,
            }),
            "radialGradient" => {
                let centre = Point::new(coordinate("cx", 0.5)?, coordinate("cy", 0.5)?);
                let focus = Point::new(coordinate("fx", centre.x)?, coordinate("fy", centre.y)?);
                if (focus.x - centre.x).abs() > f32::EPSILON
                    || (focus.y - centre.y).abs() > f32::EPSILON
                {
                    return Err(at("a focal point is not supported".to_owned()));
                }
                Ok(Shader::Radial {
                    to_space,
                    centre,
                    radius: coordinate("r", 0.5)?,
                    stops,
                })
            }
            other => Err(at(format!("<{other}> is not a gradient"))),
        }
    }

    /// The gradient's stops (or those of the gradient its `href` names),
    /// premultiplied and with offsets kept in order.
    fn stops(
        &self,
        gradient: &'a Node,
        opacity: f32,
        depth: usize,
    ) -> Result<Vec<(f32, Rgba)>, String> {
        let at = |message: String| format!("line {}: {message}", gradient.line);
        if gradient.children.is_empty()
            && let Some(reference) = gradient
                .attr("href")
                .or_else(|| gradient.attr("xlink:href"))
        {
            if depth > 8 {
                return Err(at("gradient references nest too deeply".to_owned()));
            }
            return self.stops(self.lookup(reference, gradient)?, opacity, depth + 1);
        }
        let mut stops = Vec::with_capacity(gradient.children.len());
        let mut last = 0.0_f32;
        for stop in &gradient.children {
            if stop.name != "stop" {
                return Err(at(format!("<{}> inside a gradient", stop.name)));
            }
            let offset = stop
                .attr("offset")
                .map_or(Ok(0.0), fraction_or_number)
                .map_err(at)?
                .clamp(0.0, 1.0)
                .max(last);
            last = offset;
            let color = color(stop.attr("stop-color").unwrap_or("black")).map_err(at)?;
            let stop_opacity = stop
                .attr("stop-opacity")
                .map_or(Ok(1.0), fraction)
                .map_err(at)?;
            stops.push((offset, premultiply(color, stop_opacity * opacity)));
        }
        if stops.is_empty() {
            return Err(at("a gradient without stops".to_owned()));
        }
        Ok(stops)
    }

    /// The union of a clip path's shapes, as coverage per pixel.
    fn clip_coverage(&self, clip: &'a Node, ctm: Transform) -> Result<Vec<f32>, String> {
        if clip.name != "clipPath" {
            return Err(format!(
                "line {}: <{}> is not a clipPath",
                clip.line, clip.name
            ));
        }
        if let Some(units) = clip.attr("clipPathUnits")
            && units != "userSpaceOnUse"
        {
            return Err(format!(
                "line {}: clipPathUnits {units} is not supported",
                clip.line
            ));
        }
        let mut coverage = vec![0.0_f32; self.width * self.height];
        for shape in &clip.children {
            let style = Style::initial().apply(shape)?;
            let shape_ctm = match shape.attr("transform") {
                Some(text) => transform(text)
                    .map_err(|message| format!("line {}: {message}", shape.line))?
                    .then(&ctm),
                None => ctm,
            };
            let data = path_data(shape)?;
            if data.is_empty() {
                continue;
            }
            let (mask, placement) = Mask::new(data.as_str())
                .style(style.clip_rule)
                .transform(Some(shape_ctm))
                .render();
            let mask_width = to_usize(placement.width);
            for row in 0..to_usize(placement.height) {
                let Some(y) = offset(placement.top, row, self.height) else {
                    continue;
                };
                for column in 0..mask_width {
                    let Some(x) = offset(placement.left, column, self.width) else {
                        continue;
                    };
                    let value = f32::from(mask[row * mask_width + column]) / 255.0;
                    let target = &mut coverage[y * self.width + x];
                    *target = 1.0 - (1.0 - *target) * (1.0 - value);
                }
            }
        }
        Ok(coverage)
    }
}

/// Applies a filter (a chain of blurs) to a layer drawn with `ctm`.
fn apply_filter(filter: &Node, ctm: Transform, layer: &mut Canvas) -> Result<(), String> {
    if filter.name != "filter" {
        return Err(format!(
            "line {}: <{}> is not a filter",
            filter.line, filter.name
        ));
    }
    // Device pixels per user unit.
    let scale = ctm.determinant().abs().sqrt();
    for primitive in &filter.children {
        if primitive.name != "feGaussianBlur" {
            return Err(format!(
                "line {}: the filter primitive <{}> is not supported",
                primitive.line, primitive.name
            ));
        }
        if let Some(input) = primitive.attr("in")
            && input != "SourceGraphic"
        {
            return Err(format!(
                "line {}: in={input} is not supported",
                primitive.line
            ));
        }
        let deviation = primitive
            .attr("stdDeviation")
            .map_or(Ok(0.0), number)
            .map_err(|message| format!("line {}: {message}", primitive.line))?;
        layer.blur(deviation * scale);
    }
    Ok(())
}

/// Path data for a shape element, in its user space.
fn path_data(node: &Node) -> Result<String, String> {
    let at = |message: String| format!("line {}: {message}", node.line);
    let get = |name: &str| attr_length(node, name, 0.0);
    match node.name.as_str() {
        "path" => Ok(node.attr("d").unwrap_or("").to_owned()),
        "rect" => {
            let (x, y, width, height) = (get("x")?, get("y")?, get("width")?, get("height")?);
            if width <= 0.0 || height <= 0.0 {
                return Ok(String::new());
            }
            let rx = node.attr("rx").map(length).transpose().map_err(at)?;
            let ry = node.attr("ry").map(length).transpose().map_err(at)?;
            let (rx, ry) = match (rx, ry) {
                (None, None) => (0.0, 0.0),
                (Some(rx), None) => (rx, rx),
                (None, Some(ry)) => (ry, ry),
                (Some(rx), Some(ry)) => (rx, ry),
            };
            Ok(rounded_rect(
                x,
                y,
                width,
                height,
                rx.min(width / 2.0),
                ry.min(height / 2.0),
            ))
        }
        "circle" => {
            let r = get("r")?;
            Ok(ellipse(get("cx")?, get("cy")?, r, r))
        }
        "ellipse" => Ok(ellipse(get("cx")?, get("cy")?, get("rx")?, get("ry")?)),
        "line" => Ok(format!(
            "M{} {} L{} {}",
            get("x1")?,
            get("y1")?,
            get("x2")?,
            get("y2")?
        )),
        "polyline" | "polygon" => {
            let values = numbers(node.attr("points").unwrap_or("")).map_err(at)?;
            if values.len() % 2 != 0 {
                return Err(at("points has an odd count".to_owned()));
            }
            let mut data = String::new();
            for (index, pair) in values.as_chunks::<2>().0.iter().enumerate() {
                let command = if index == 0 { "M" } else { " L" };
                let _ = write!(data, "{command}{} {}", pair[0], pair[1]);
            }
            if node.name == "polygon" && !data.is_empty() {
                data.push_str(" Z");
            }
            Ok(data)
        }
        other => Err(at(format!("<{other}> is not a shape"))),
    }
}

fn rounded_rect(x: f32, y: f32, width: f32, height: f32, rx: f32, ry: f32) -> String {
    if rx <= 0.0 || ry <= 0.0 {
        return format!("M{x} {y} H{} V{} H{x} Z", x + width, y + height);
    }
    let right = x + width;
    let bottom = y + height;
    format!(
        "M{} {y} H{} A{rx} {ry} 0 0 1 {right} {} V{} A{rx} {ry} 0 0 1 {} {bottom} H{} \
         A{rx} {ry} 0 0 1 {x} {} V{} A{rx} {ry} 0 0 1 {} {y} Z",
        x + rx,
        right - rx,
        y + ry,
        bottom - ry,
        right - rx,
        x + rx,
        bottom - ry,
        y + ry,
        x + rx,
    )
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> String {
    if rx <= 0.0 || ry <= 0.0 {
        return String::new();
    }
    format!(
        "M{} {cy} A{rx} {ry} 0 1 0 {} {cy} A{rx} {ry} 0 1 0 {} {cy} Z",
        cx - rx,
        cx + rx,
        cx - rx
    )
}

fn attr_length(node: &Node, name: &str, default: f32) -> Result<f32, String> {
    node.attr(name)
        .map_or(Ok(default), length)
        .map_err(|message| format!("line {}: {message}", node.line))
}

/// A transform list such as `translate(4 2) rotate(45)`.
fn transform(text: &str) -> Result<Transform, String> {
    let mut result = Transform::IDENTITY;
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest
            .find('(')
            .ok_or_else(|| format!("bad transform {text:?}"))?;
        let close = rest
            .find(')')
            .ok_or_else(|| format!("bad transform {text:?}"))?;
        let name = rest[..open].trim().trim_start_matches(',').trim();
        let args = numbers(&rest[open + 1..close])?;
        let step = match (name, args.as_slice()) {
            ("translate", [x]) => Transform::translation(*x, 0.0),
            ("translate", [x, y]) => Transform::translation(*x, *y),
            ("scale", [s]) => Transform::scale(*s, *s),
            ("scale", [x, y]) => Transform::scale(*x, *y),
            ("rotate", [angle]) => Transform::rotation(zeno::Angle::from_degrees(*angle)),
            ("rotate", [angle, x, y]) => Transform::translation(-x, -y)
                .then(&Transform::rotation(zeno::Angle::from_degrees(*angle)))
                .then_translate(*x, *y),
            ("matrix", [xx, xy, yx, yy, dx, dy]) => Transform::new(*xx, *xy, *yx, *yy, *dx, *dy),
            _ => return Err(format!("unsupported transform {name}({args:?})")),
        };
        // Later functions apply to points first.
        result = step.then(&result);
        rest = rest[close + 1..].trim_start();
    }
    Ok(result)
}

/// Numbers separated by blanks and/or commas.
fn numbers(text: &str) -> Result<Vec<f32>, String> {
    text.split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(number)
        .collect()
}

fn number(text: &str) -> Result<f32, String> {
    let value: f32 = text
        .trim()
        .parse()
        .map_err(|_| format!("{text:?} is not a number"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{text:?} is not a finite number"))
    }
}

/// A length in user units (`px` allowed).
fn length(text: &str) -> Result<f32, String> {
    number(text.trim().trim_end_matches("px"))
}

/// A number or a percentage, as a fraction (`50%` is 0.5).
fn fraction_or_number(text: &str) -> Result<f32, String> {
    let text = text.trim();
    match text.strip_suffix('%') {
        Some(percent) => Ok(number(percent)? / 100.0),
        None => number(text),
    }
}

/// An opacity: a number or a percentage, clamped to 0..1.
fn fraction(text: &str) -> Result<f32, String> {
    Ok(fraction_or_number(text)?.clamp(0.0, 1.0))
}

fn paint(text: &str) -> Result<Paint<'_>, String> {
    let text = text.trim();
    if text == "none" {
        return Ok(Paint::None);
    }
    if text == "currentColor" {
        return Ok(Paint::Current);
    }
    if let Some(inner) = text
        .strip_prefix("url(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return Ok(Paint::Url(inner.trim()));
    }
    color(text).map(Paint::Color)
}

fn color(text: &str) -> Result<Rgba, String> {
    let text = text.trim();
    match text {
        "black" => return Ok([0.0, 0.0, 0.0, 1.0]),
        "white" => return Ok([1.0, 1.0, 1.0, 1.0]),
        "transparent" => return Ok([0.0; 4]),
        _ => {}
    }
    let hex = text
        .strip_prefix('#')
        .ok_or_else(|| format!("{text:?} is not a supported colour"))?;
    let digit = |index: usize| -> Result<f32, String> {
        let value = u8::from_str_radix(&hex[index..=index], 16)
            .map_err(|_| format!("{text:?} is not a colour"))?;
        Ok(f32::from(value))
    };
    match hex.len() {
        3 => Ok([
            digit(0)? * 17.0 / 255.0,
            digit(1)? * 17.0 / 255.0,
            digit(2)? * 17.0 / 255.0,
            1.0,
        ]),
        6 => Ok([
            (digit(0)? * 16.0 + digit(1)?) / 255.0,
            (digit(2)? * 16.0 + digit(3)?) / 255.0,
            (digit(4)? * 16.0 + digit(5)?) / 255.0,
            1.0,
        ]),
        _ => Err(format!("{text:?} is not a colour")),
    }
}

// Sizes and indices here are canvas dimensions, far below 2^24, so the
// conversion to f32 is exact.
#[allow(clippy::cast_precision_loss)]
fn to_f32(value: usize) -> f32 {
    value as f32
}

fn to_usize(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_usize_f(value: f32) -> usize {
    // Callers pass small non-negative values (blur radii).
    value as usize
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(value: f32) -> u8 {
    // Clamped to 0..=255 first, so the cast is exact.
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::Document;

    fn render(text: &str, size: u32) -> rvp_skin::Image {
        Document::parse(text)
            .expect("parse")
            .render(size, size)
            .expect("render")
    }

    #[test]
    fn fills_a_square_with_a_solid_colour() {
        let image = render(
            "<svg viewBox='0 0 4 4'><rect x='1' y='1' width='2' height='2' fill='#f80'/></svg>",
            4,
        );
        assert_eq!(image.pixel(0, 0), [0, 0, 0, 0]);
        assert_eq!(image.pixel(1, 1), [255, 136, 0, 255]);
        assert_eq!(image.pixel(2, 2), [255, 136, 0, 255]);
        assert_eq!(image.pixel(3, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn scales_the_view_box_and_blends_opacity() {
        let image = render(
            "<svg viewBox='0 0 2 2'><rect width='2' height='2' fill='#000'/>\
             <g opacity='0.5'><rect width='1' height='2' fill='#fff'/></g></svg>",
            8,
        );
        assert_eq!(image.pixel(1, 4), [128, 128, 128, 255]);
        assert_eq!(image.pixel(6, 4), [0, 0, 0, 255]);
    }

    #[test]
    fn draws_gradients_symbols_and_current_colour() {
        let image = render(
            "<svg viewBox='0 0 10 10'><defs>\
             <linearGradient id='g' x1='0' y1='0' x2='1' y2='0'>\
             <stop offset='0' stop-color='#000'/><stop offset='1' stop-color='#fff'/></linearGradient>\
             <symbol id='s' viewBox='0 0 2 2'><rect width='2' height='2' fill='currentColor'/></symbol>\
             </defs><rect width='10' height='5' fill='url(#g)'/>\
             <use href='#s' x='0' y='5' width='5' height='5' color='#0f0'/></svg>",
            10,
        );
        let left = image.pixel(0, 2)[0];
        let right = image.pixel(9, 2)[0];
        assert!(left < 20 && right > 235, "{left} {right}");
        assert_eq!(image.pixel(2, 7), [0, 255, 0, 255]);
        assert_eq!(image.pixel(7, 7)[3], 0);
    }

    #[test]
    fn clips_and_blurs_on_a_layer() {
        let image = render(
            "<svg viewBox='0 0 20 20'><defs><clipPath id='c'><rect width='10' height='20'/></clipPath>\
             <filter id='f'><feGaussianBlur stdDeviation='2'/></filter></defs>\
             <rect width='20' height='20' fill='#fff' clip-path='url(#c)'/>\
             <rect x='14' y='14' width='2' height='2' fill='#f00' filter='url(#f)'/></svg>",
            20,
        );
        assert_eq!(image.pixel(5, 5), [255, 255, 255, 255]);
        assert_eq!(image.pixel(15, 5)[3], 0);
        let glow = image.pixel(15, 12);
        assert!(glow[3] > 0 && glow[3] < 128, "{glow:?}");
    }

    #[test]
    fn rejects_what_it_cannot_draw() {
        for (text, needle) in [
            (
                "<svg viewBox='0 0 1 1'><text>RVP</text></svg>",
                "<text> is not supported",
            ),
            (
                "<svg viewBox='0 0 1 1'><rect style='fill:red'/></svg>",
                "style",
            ),
            ("<g/>", "not <svg>"),
            (
                "<svg viewBox='0 0 1 1'><rect id='a'/><rect id='a'/></svg>",
                "repeated",
            ),
        ] {
            let error = Document::parse(text).expect_err(text);
            assert!(error.contains(needle), "{text}: {error}");
        }
        let bad_paint =
            Document::parse("<svg viewBox='0 0 1 1'><rect width='1' height='1' fill='red'/></svg>")
                .expect("parse")
                .render(1, 1)
                .expect_err("red");
        assert!(bad_paint.contains("colour"), "{bad_paint}");
    }
}
