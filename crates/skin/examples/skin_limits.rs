//! Measures skins against `rvp_skin::limits`.
//!
//! ```text
//! cargo run --release -p rvp-skin --example skin_limits -- [--fingerprint] PATH...
//! ```
//!
//! Each `PATH` is a skin file or a folder searched for `.dsf`, `.wsz` and
//! `.vlt` files. Every skin is loaded as RVP loads it; for each quantity
//! that has a limit the table gives the largest value among the skins, the
//! skin it came from, the limit and how many times the limit is above it.
//! Run it over the skins RVP is checked with before changing a limit.
//!
//! The exit code is 1 when a skin did not load. With `--fingerprint` each
//! skin's line is a hash of everything it shows (its pictures, and every
//! area laid out and painted at three sizes): two runs' lines differ where
//! a change to the engine changed a skin's look.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rvp_skin::markup::Element;
use rvp_skin::vlc::Control;
use rvp_skin::{DataModel, Skin, SkinArchive, SliderValue, UiState, layout, limits, paint};

/// The largest value of each quantity, and the skin that has it.
#[derive(Default)]
struct Largest {
    values: BTreeMap<&'static str, (u64, String)>,
}

impl Largest {
    fn note(&mut self, quantity: &'static str, value: impl TryInto<u64>, skin: &str) {
        let value = value.try_into().unwrap_or(0);
        let entry = self.values.entry(quantity).or_insert((0, String::new()));
        if value > entry.0 || entry.1.is_empty() {
            *entry = (value, skin.to_owned());
        }
    }
}

/// The quantities in the table's order, each with its limit (`None`: a
/// number that has none of its own and is shown for the others' sake).
fn quantities() -> Vec<(&'static str, Option<u64>)> {
    let size = |value: usize| u64::try_from(value).ok();
    let pixels = |value: i32| u64::try_from(value).ok();
    vec![
        ("skin file, bytes", Some(limits::ARCHIVE_BYTES)),
        ("files in the archive", size(limits::ARCHIVE_ENTRIES)),
        ("largest file unpacked, bytes", Some(limits::ENTRY_BYTES)),
        ("all files unpacked, bytes", Some(limits::UNPACKED_BYTES)),
        (
            "picture's longer side",
            Some(u64::from(limits::PICTURE_SIDE)),
        ),
        ("picture, pixels", Some(limits::PICTURE_PIXELS)),
        ("pictures of one skin, pixels", Some(limits::SKIN_PIXELS)),
        ("expanded markup, bytes", size(limits::TEXT_BYTES)),
        ("markup files read, bytes", size(limits::TEXT_BYTES)),
        ("includes", size(limits::INCLUDES)),
        ("include depth", size(limits::INCLUDE_DEPTH)),
        ("macros of one include", size(limits::MACROS)),
        ("markup depth", size(limits::MARKUP_DEPTH)),
        ("markup elements", size(limits::MARKUP_ELEMENTS)),
        ("attributes of one element", size(limits::ATTRIBUTES)),
        ("coordinate in a layout", pixels(limits::COORDINATE)),
        ("expression, tokens", size(limits::EXPRESSION_TOKENS)),
        (
            "slider background, pictures",
            u64::try_from(limits::SLIDER_CELLS).ok(),
        ),
        ("own windows", size(limits::EXTRA_WINDOWS)),
        ("own window's longer side", pixels(limits::WINDOW_SIDE)),
        ("own window, pixels", Some(limits::WINDOW_PIXELS)),
        ("window shape, rectangles", size(limits::REGION_RECTS)),
        ("logo button's longer side", pixels(limits::LOGO_SIDE)),
        ("logo buttons", size(limits::LOGO_SLOTS)),
        ("layouts of a VLC window", None),
        ("font size", None),
    ]
}

/// FNV-1a over everything a skin shows.
struct Hash(u64);

impl Hash {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // A separator, so that joined parts differ from split ones.
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }
}

/// The data a fingerprint paints with: the sample film, a spectrum and
/// the equalizer's bands.
fn sample_data() -> DataModel {
    let mut data = DataModel::sample();
    data.visual = (0..16_u8)
        .map(|band| 0.2 + 0.05 * f32::from(band))
        .collect();
    data.visual_peaks = (0..16_u8)
        .map(|band| 0.3 + 0.04 * f32::from(band))
        .collect();
    for band in 0..10_u8 {
        data.sliders.insert(
            format!("CMD_EQULIZER_{band}"),
            SliderValue {
                value: 0.1 * f32::from(band),
                enabled: true,
            },
        );
    }
    data
}

/// The areas of a skin: name, own size, and whether its window takes the
/// shape of what is drawn.
fn areas(skin: &Skin) -> Vec<(String, i32, i32, bool)> {
    let mut areas = Vec::new();
    skin.root().walk(&mut |element| {
        let name = element.name.to_ascii_lowercase();
        if name.ends_with("area") || name == "logo" || element.attrs.flag("RvpWindow") {
            areas.push((
                element.name.clone(),
                element.attrs.int_or("Width", 0),
                element.attrs.int_or("Height", 0),
                element.attrs.get("RvpShape") == Some("alpha"),
            ));
        }
    });
    areas
}

/// The size an area is first drawn at: its own, or a usual window's.
fn own_size(width: i32, height: i32) -> (i32, i32) {
    if (1..=4096).contains(&width) && (1..=4096).contains(&height) {
        (width, height)
    } else {
        (1024, 600)
    }
}

/// Adds what `skin` shows to `hash`: its pictures, its windows and every
/// area laid out and painted at three sizes, idle and playing.
fn fingerprint_skin(hash: &mut Hash, skin: &Skin) {
    hash.text(&skin.warnings().join("\n"));
    hash.text(&format!("{:?}", skin.extra_windows()));
    hash.text(&format!("{:?}", skin.logo_slots()));
    for name in skin.archive().names() {
        if let Some(image) = skin.image(name) {
            hash.text(name);
            hash.text(&format!("{}x{}", image.width(), image.height()));
            hash.bytes(image.data());
        }
    }
    let data = sample_data();
    for (area, width, height, alpha) in areas(skin) {
        let own = own_size(width, height);
        for (index, (w, h)) in [own, (640, 360), (1920, 1080)].into_iter().enumerate() {
            for playing in [false, true] {
                let mut state = UiState {
                    media_loaded: playing,
                    playing,
                    ..UiState::default()
                };
                state.values.extend(data.text_values());
                let Ok(placed) = layout(skin, &area, w, h, &state) else {
                    continue;
                };
                hash.text(&area);
                hash.text(&placed.dump());
                let list = paint(skin, &placed, &state, &data);
                hash.text(&format!("{:?}", list.ops));
                if alpha && index == 0 && !playing {
                    hash.text(&rvp_skin::render::opaque_region(&list, skin));
                }
            }
        }
    }
}

/// The depth of the tree below `element` (an element without children: 1).
fn depth(element: &Element) -> usize {
    1 + element.children.iter().map(depth).max().unwrap_or(0)
}

/// Notes the markup's numbers of a tree whose root is the document.
fn measure_markup(largest: &mut Largest, name: &str, root: &Element) {
    let mut elements = 0_usize;
    let mut attributes = 0_usize;
    let mut font = 0_i32;
    root.walk(&mut |element| {
        elements += 1;
        attributes = attributes.max(element.attrs.len());
        font = font
            .max(element.attrs.int_or("FontSize", 0))
            .max(element.attrs.int_or("size", 0));
    });
    // The document itself is no element of the skin.
    largest.note("markup depth", depth(root) - 1, name);
    largest.note("markup elements", elements - 1, name);
    largest.note("attributes of one element", attributes, name);
    largest.note("font size", font, name);
}

/// Notes what one loaded skin file has.
fn measure_skin(largest: &mut Largest, name: &str, skin: &Skin) {
    let expanded = skin.expanded();
    largest.note("expanded markup, bytes", expanded.text.len(), name);
    largest.note("markup files read, bytes", expanded.read, name);
    largest.note("includes", expanded.files.len().saturating_sub(1), name);
    largest.note("include depth", expanded.depth, name);
    largest.note("macros of one include", expanded.macros, name);
    measure_markup(largest, name, skin.root());
    let mut pixels = 0_u64;
    for file in skin.archive().names() {
        if let Some(image) = skin.image(file) {
            pixels += u64::from(image.width()) * u64::from(image.height());
            measure_picture(largest, name, image.width(), image.height());
        }
    }
    largest.note("pictures of one skin, pixels", pixels, name);
    let slots = skin.logo_slots();
    largest.note("logo buttons", slots.len(), name);
    for slot in slots {
        largest.note(
            "logo button's longer side",
            slot.width.max(slot.height),
            name,
        );
    }
    let windows = skin.extra_windows();
    largest.note("own windows", windows.len(), name);
    for window in windows {
        let (width, height) = (window.width, window.height);
        largest.note("own window's longer side", width.max(height), name);
        largest.note(
            "own window, pixels",
            i64::from(width.max(0)) * i64::from(height.max(0)),
            name,
        );
    }
    let data = sample_data();
    let state = UiState::default();
    for (area, width, height, alpha) in areas(skin) {
        let (w, h) = own_size(width, height);
        let Ok(placed) = layout(skin, &area, w, h, &state) else {
            continue;
        };
        for node in &placed.nodes {
            let rect = node.rect;
            let edges = [rect.x, rect.y, rect.right(), rect.bottom()];
            let far = edges.iter().map(|edge| edge.unsigned_abs()).max();
            largest.note("coordinate in a layout", far.unwrap_or(0), name);
        }
        if alpha {
            let list = paint(skin, &placed, &state, &data);
            let region = rvp_skin::render::opaque_region(&list, skin);
            let rects = region.split(';').filter(|rect| !rect.is_empty()).count();
            largest.note("window shape, rectangles", rects, name);
        }
    }
}

fn measure_picture(largest: &mut Largest, name: &str, width: u32, height: u32) {
    largest.note("picture's longer side", width.max(height), name);
    largest.note(
        "picture, pixels",
        u64::from(width) * u64::from(height),
        name,
    );
}

/// Notes what an archive's members are.
fn measure_members(largest: &mut Largest, name: &str, members: &[(String, Vec<u8>)]) {
    largest.note("files in the archive", members.len(), name);
    let sizes = members.iter().map(|(_, bytes)| bytes.len());
    largest.note(
        "largest file unpacked, bytes",
        sizes.clone().max().unwrap_or(0),
        name,
    );
    largest.note("all files unpacked, bytes", sizes.sum::<usize>(), name);
}

/// Notes an expression's length.
fn measure_expression(largest: &mut Largest, name: &str, text: Option<&str>) {
    if let Some(text) = text {
        let tokens = rvp_skin::vlc::expr::token_count(text);
        largest.note("expression, tokens", tokens, name);
    }
}

/// Notes a VLC theme's expressions and slider backgrounds.
fn measure_controls(largest: &mut Largest, name: &str, controls: &[Control]) {
    for control in controls {
        match control {
            Control::Panel { children, .. } => measure_controls(largest, name, children),
            Control::Checkbox { state, visible, .. } => {
                measure_expression(largest, name, Some(state));
                measure_expression(largest, name, visible.as_deref());
            }
            Control::Slider {
                background,
                visible,
                ..
            } => {
                measure_expression(largest, name, visible.as_deref());
                if let Some(background) = background {
                    let cells = i64::from(background.columns) * i64::from(background.rows);
                    largest.note("slider background, pictures", cells, name);
                }
            }
            Control::Image { visible, .. }
            | Control::Button { visible, .. }
            | Control::Text { visible, .. }
            | Control::Playtree { visible, .. }
            | Control::Video { visible, .. } => {
                measure_expression(largest, name, visible.as_deref());
            }
        }
    }
}

/// Loads the skin at `path` as RVP does, notes its numbers and returns
/// its fingerprint.
fn measure(largest: &mut Largest, path: &Path, name: &str) -> Result<u64, String> {
    let error = |error: rvp_skin::SkinError| error.to_string();
    let mut hash = Hash::new();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let bytes = rvp_skin::archive::read_file(path).map_err(error)?;
    largest.note("skin file, bytes", bytes.len(), name);
    match extension.as_str() {
        "wsz" => {
            let members = rvp_skin::archive::zip_members(&bytes).map_err(error)?;
            measure_members(largest, name, &members);
            let classic = rvp_skin::winamp::WinampSkin::open(path).map_err(error)?;
            let mut pixels = 0_u64;
            for bitmap in rvp_skin::winamp::BITMAPS {
                if !classic.missing.contains(&bitmap) {
                    let image = classic.bitmap(bitmap);
                    pixels += u64::from(image.width()) * u64::from(image.height());
                    measure_picture(largest, name, image.width(), image.height());
                }
            }
            largest.note("pictures of one skin, pixels", pixels, name);
            hash.text(&classic.warnings.join("\n"));
            hash.text(&format!("{:?}", classic.missing));
            hash.text(&format!("{:?}", classic.regions));
            let skin = rvp_skin::winamp::build::skin(&classic).map_err(error)?;
            measure_skin(largest, name, &skin);
            fingerprint_skin(&mut hash, &skin);
        }
        "vlt" => {
            let members = rvp_skin::vlc::members(&bytes).map_err(error)?;
            measure_members(largest, name, &members);
            let theme_xml = members
                .iter()
                .filter(|(member, _)| member.rsplit(['/', '\\']).next() == Some("theme.xml"))
                .min_by_key(|(member, _)| member.matches(['/', '\\']).count());
            if let Some((_, text)) = theme_xml {
                largest.note("expanded markup, bytes", text.len(), name);
                let document = rvp_skin::markup::parse_xml(&String::from_utf8_lossy(text));
                measure_markup(largest, name, &document);
            }
            let theme = rvp_skin::vlc::Theme::open(path).map_err(error)?;
            let mut pixels = 0_u64;
            for image in theme.images.values() {
                pixels += u64::from(image.width()) * u64::from(image.height());
                measure_picture(largest, name, image.width(), image.height());
            }
            largest.note("pictures of one skin, pixels", pixels, name);
            for window in &theme.windows {
                largest.note("layouts of a VLC window", window.layouts.len(), name);
                for layout in &window.layouts {
                    measure_controls(largest, name, &layout.controls);
                }
            }
            hash.text(&theme.warnings.join("\n"));
            let members = rvp_skin::vlc::build::members(&theme).map_err(error)?;
            let files: Vec<String> = members
                .iter()
                .map(|(name, _)| name.clone())
                .filter(|name| is_markup(name))
                .collect();
            let archive = SkinArchive::from_files(&theme.name, members);
            for file in files {
                let skin = Skin::load(archive.clone(), &file).map_err(error)?;
                measure_skin(largest, name, &skin);
                hash.text(&file);
                fingerprint_skin(&mut hash, &skin);
            }
        }
        _ => {
            let members = rvp_skin::archive::zip_members(&bytes).map_err(error)?;
            measure_members(largest, name, &members);
            let archive = SkinArchive::open(path).map_err(error)?;
            let files: Vec<String> = archive
                .names()
                .filter(|name| is_markup(name) && !name.contains('/'))
                .map(str::to_owned)
                .collect();
            for file in files {
                let skin = Skin::load(archive.clone(), &file).map_err(error)?;
                measure_skin(largest, name, &skin);
                hash.text(&file);
                fingerprint_skin(&mut hash, &skin);
            }
        }
    }
    Ok(hash.0)
}

/// Whether an archive's member is a skin's markup file.
fn is_markup(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
}

/// The skin files under `path`, sorted.
fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        let mut entries: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        for entry in entries {
            collect(&entry, out);
        }
    } else if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["dsf", "wsz", "vlt"]
                .iter()
                .any(|kind| extension.eq_ignore_ascii_case(kind))
        })
    {
        out.push(path.to_path_buf());
    }
}

fn main() -> ExitCode {
    let mut fingerprints = false;
    let mut paths = Vec::new();
    for argument in std::env::args().skip(1) {
        if argument == "--fingerprint" {
            fingerprints = true;
        } else {
            collect(Path::new(&argument), &mut paths);
        }
    }
    if paths.is_empty() {
        eprintln!("usage: skin_limits [--fingerprint] PATH...");
        return ExitCode::FAILURE;
    }
    let mut largest = Largest::default();
    let mut failed = 0_usize;
    for path in &paths {
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        match measure(&mut largest, path, &name) {
            Ok(hash) if fingerprints => println!("{hash:016x} {name}"),
            Ok(_) => {}
            Err(error) => {
                failed += 1;
                println!("FAILED {name}: {error}");
            }
        }
    }
    if !fingerprints {
        println!(
            "{:<30} {:>12} {:>12} {:>8}  skin",
            "quantity", "largest", "limit", "times"
        );
        for (quantity, limit) in quantities() {
            let (value, skin) = largest
                .values
                .get(quantity)
                .cloned()
                .unwrap_or((0, String::new()));
            let (limit_text, times) = match limit {
                Some(limit) if value > 0 => {
                    // A ratio for reading: the precision of f64 is plenty.
                    #[allow(clippy::cast_precision_loss)]
                    let times = limit as f64 / value as f64;
                    (limit.to_string(), format!("{times:.1}"))
                }
                Some(limit) => (limit.to_string(), "-".to_owned()),
                None => ("-".to_owned(), "-".to_owned()),
            };
            println!("{quantity:<30} {value:>12} {limit_text:>12} {times:>8}  {skin}");
        }
        println!(
            "{} skins, {} loaded, {failed} failed",
            paths.len(),
            paths.len() - failed
        );
    }
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
