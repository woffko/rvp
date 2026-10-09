//! Font lookup and glyph rasterization for the headless renderer.
//!
//! Skins name Windows fonts (`Tahoma`, `Arial`). Fonts are searched in
//! `RVP_FONT_DIRS` and then in the usual system font directories, including
//! the Windows font folder as seen from WSL. Missing fonts fall back to a
//! common sans-serif font; with no font at all, text is not drawn.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;

use crate::paint::FontSpec;

/// Font directories searched after `RVP_FONT_DIRS`.
const SYSTEM_FONT_DIRS: [&str; 6] = [
    "C:\\Windows\\Fonts",
    "/mnt/c/Windows/Fonts",
    "/usr/share/fonts",
    "/usr/local/share/fonts",
    "/System/Library/Fonts",
    "/Library/Fonts",
];

/// File names tried for a family, regular and bold.
const KNOWN_FAMILIES: [(&str, &str, &str); 5] = [
    ("tahoma", "tahoma.ttf", "tahomabd.ttf"),
    ("arial", "arial.ttf", "arialbd.ttf"),
    ("segoe ui", "segoeui.ttf", "segoeuib.ttf"),
    ("verdana", "verdana.ttf", "verdanab.ttf"),
    ("gulim", "gulim.ttc", "gulim.ttc"),
];

/// Fallback files when the requested family is not installed.
const FALLBACK_FILES: [&str; 4] = [
    "dejavusans.ttf",
    "liberationsans-regular.ttf",
    "notosans-regular.ttf",
    "arial.ttf",
];

/// A rasterized glyph: an alpha mask placed relative to the pen position
/// on the baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBitmap {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}

/// Line metrics in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineMetrics {
    pub ascent: f32,
    pub descent: f32,
}

struct Face {
    path: PathBuf,
    data: Vec<u8>,
}

/// Fonts found on this machine, loaded on demand.
pub struct Fonts {
    dirs: Vec<PathBuf>,
    files: Option<BTreeMap<String, PathBuf>>,
    faces: Vec<Face>,
    by_spec: BTreeMap<(String, bool), Option<usize>>,
    glyphs: BTreeMap<(usize, u16, u32), Option<GlyphBitmap>>,
    context: ScaleContext,
}

impl std::fmt::Debug for Fonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fonts")
            .field("dirs", &self.dirs)
            .field(
                "faces",
                &self.faces.iter().map(|face| &face.path).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl Fonts {
    /// Searches only the given directories.
    #[must_use]
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self {
            dirs,
            files: None,
            faces: Vec::new(),
            by_spec: BTreeMap::new(),
            glyphs: BTreeMap::new(),
            context: ScaleContext::new(),
        }
    }

    /// Searches `RVP_FONT_DIRS` first, then the system font directories.
    #[must_use]
    pub fn from_environment() -> Self {
        let mut dirs: Vec<PathBuf> = std::env::var_os("RVP_FONT_DIRS")
            .map(|value| std::env::split_paths(&value).collect())
            .unwrap_or_default();
        dirs.extend(SYSTEM_FONT_DIRS.iter().map(PathBuf::from));
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(Path::new(&home).join(".local/share/fonts"));
        }
        Self::new(dirs)
    }

    /// The file a font request resolves to, if any.
    pub fn resolve(&mut self, spec: &FontSpec) -> Option<&Path> {
        let index = self.face(spec)?;
        Some(&self.faces[index].path)
    }

    /// Finds and loads the face for a request; returns its index.
    pub fn face(&mut self, spec: &FontSpec) -> Option<usize> {
        let key = (spec.family.to_ascii_lowercase(), spec.bold);
        if let Some(found) = self.by_spec.get(&key) {
            return *found;
        }
        let found = self.load(&key.0, key.1);
        self.by_spec.insert(key, found);
        found
    }

    fn load(&mut self, family: &str, bold: bool) -> Option<usize> {
        let mut candidates: Vec<String> = Vec::new();
        if let Some((_, regular, bold_file)) =
            KNOWN_FAMILIES.iter().find(|(name, _, _)| *name == family)
        {
            if bold {
                candidates.push((*bold_file).to_owned());
            }
            candidates.push((*regular).to_owned());
        }
        candidates.push(format!("{}.ttf", family.replace(' ', "")));
        candidates.extend(FALLBACK_FILES.iter().map(|name| (*name).to_owned()));
        let files = self
            .files
            .get_or_insert_with(|| index_font_files(&self.dirs));
        let path = candidates
            .iter()
            .find_map(|name| files.get(name.as_str()).cloned())?;
        if let Some(existing) = self.faces.iter().position(|face| face.path == path) {
            return Some(existing);
        }
        let data = std::fs::read(&path).ok()?;
        FontRef::from_index(&data, 0)?;
        self.faces.push(Face { path, data });
        Some(self.faces.len() - 1)
    }

    /// Ascent and descent of a face at a pixel size.
    #[must_use]
    pub fn line_metrics(&self, face: usize, size_px: f32) -> LineMetrics {
        match self.font(face) {
            Some(font) => {
                let metrics = font.metrics(&[]).scale(size_px);
                LineMetrics {
                    ascent: metrics.ascent,
                    descent: metrics.descent,
                }
            }
            None => LineMetrics {
                ascent: size_px,
                descent: 0.0,
            },
        }
    }

    /// Glyph IDs and rounded advances for `text`.
    #[must_use]
    pub fn shape(&self, face: usize, size_px: f32, text: &str) -> Vec<(u16, i32)> {
        let Some(font) = self.font(face) else {
            return Vec::new();
        };
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(&[]).scale(size_px);
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            let glyph = charmap.map(ch);
            let advance = metrics.advance_width(glyph).round();
            // Advances of UI text are a few dozen pixels.
            #[allow(clippy::cast_possible_truncation)]
            glyphs.push((glyph, advance as i32));
        }
        glyphs
    }

    /// The rasterized glyph, cached per face, glyph and size.
    pub fn glyph(&mut self, face: usize, glyph: u16, size_px: f32) -> Option<&GlyphBitmap> {
        let key = (face, glyph, size_px.to_bits());
        if !self.glyphs.contains_key(&key) {
            let bitmap = self.rasterize(face, glyph, size_px);
            self.glyphs.insert(key, bitmap);
        }
        self.glyphs.get(&key).and_then(Option::as_ref)
    }

    fn rasterize(&mut self, face: usize, glyph: u16, size_px: f32) -> Option<GlyphBitmap> {
        let data = &self.faces.get(face)?.data;
        let font = FontRef::from_index(data, 0)?;
        let mut scaler = self.context.builder(font).size(size_px).hint(true).build();
        let image = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .render(&mut scaler, glyph)?;
        Some(GlyphBitmap {
            left: image.placement.left,
            top: image.placement.top,
            width: image.placement.width,
            height: image.placement.height,
            alpha: image.data,
        })
    }

    fn font(&self, face: usize) -> Option<FontRef<'_>> {
        FontRef::from_index(&self.faces.get(face)?.data, 0)
    }
}

/// Maps lower-case font file names to paths, searching up to four levels
/// deep. Earlier directories win; entries are visited in sorted order so the
/// result does not depend on the file system.
fn index_font_files(dirs: &[PathBuf]) -> BTreeMap<String, PathBuf> {
    let mut files = BTreeMap::new();
    for root in dirs {
        let mut pending: Vec<(PathBuf, u32)> = vec![(root.clone(), 0)];
        while let Some((dir, depth)) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
            paths.sort();
            for path in paths.into_iter().rev() {
                if path.is_dir() {
                    if depth < 4 {
                        pending.push((path, depth + 1));
                    }
                    continue;
                }
                let Some(name) = path.file_name() else {
                    continue;
                };
                let is_font = path.extension().is_some_and(|extension| {
                    ["ttf", "ttc", "otf"]
                        .iter()
                        .any(|known| extension.eq_ignore_ascii_case(known))
                });
                if is_font {
                    let name = name.to_string_lossy().to_ascii_lowercase();
                    files.entry(name).or_insert(path);
                }
            }
        }
    }
    files
}
