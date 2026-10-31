//! Classic Winamp skins (`.wsz`, M9; `docs/spikes/s6-other-skins.md`):
//! bitmaps at fixed places in windows of fixed size, turned into an RVP
//! skin ([`build`]) that RVP's own engine lays out, paints and hit-tests.
//!
//! What Winamp 5 does, and RVP reproduces (the facts come from Winamp's
//! released source and Webamp, MIT; no code from either):
//!
//! * A skin is a ZIP archive whose members are found by their base name:
//!   case and folders do not matter, and of two members with the same
//!   name the later one in the archive wins. Only `.bmp`, `.txt` and
//!   `.cur` members count (PNG is a later player's extension).
//! * A missing bitmap falls back, file by file, to the player's own:
//!   RVP's are drawn in code ([`fallback`]), since Winamp's base skin is
//!   Winamp's art. `nums_ex.bmp` takes the place of `numbers.bmp`, and a
//!   skin without `balance.bmp` uses its own `volume.bmp` for the balance.
//! * Bitmaps have no colour key: magenta draws as magenta. A sprite read
//!   beyond a bitmap that is too small is transparent there (Webamp's
//!   reading; skins rely on it for slider thumbs they leave out).
//! * `pledit.txt` gives the playlist's colours and font (`Normal`,
//!   `Current`, `NormalBG`, `SelectedBG`, `mbFG`, `mbBG`, `Font`; hex
//!   with or without `#`, the low 24 bits of the number). `viscolor.txt`
//!   gives the visualizer's 24 colours, one `r,g,b` a line; the first line
//!   without a number ends the list. `region.txt` shapes the main window
//!   and the equalizer (`[Normal]`, `[WindowShade]`, `[Equalizer]`,
//!   `[EqualizerWS]`): `NumPoints` per polygon and their `PointList`,
//!   used only when the counts match.

pub mod build;
pub mod fallback;

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::SkinError;
use crate::image::Image;

/// The bitmaps of a classic skin, by base name without extension.
pub const BITMAPS: [&str; 17] = [
    "main", "titlebar", "cbuttons", "shufrep", "posbar", "volume", "balance", "monoster",
    "playpaus", "numbers", "nums_ex", "text", "eqmain", "eq_ex", "pledit", "video", "gen",
];

/// The playlist's colours and font from `pledit.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistStyle {
    pub normal: [u8; 3],
    pub current: [u8; 3],
    pub normal_bg: [u8; 3],
    pub selected_bg: [u8; 3],
    /// The video window's info line.
    pub mb_fg: [u8; 3],
    pub mb_bg: [u8; 3],
    /// Empty: the system's default.
    pub font: String,
}

impl Default for PlaylistStyle {
    fn default() -> Self {
        Self {
            normal: [0x00, 0xFF, 0x00],
            current: [0xFF, 0xFF, 0xFF],
            normal_bg: [0x00, 0x00, 0x00],
            selected_bg: [0x00, 0x00, 0xC6],
            mb_fg: [0x00, 0xFF, 0x00],
            mb_bg: [0x00, 0x00, 0x00],
            font: String::new(),
        }
    }
}

/// A window's shape: polygons in window pixels (corners of pixels).
pub type Region = Vec<Vec<(i32, i32)>>;

/// A loaded classic skin.
#[derive(Debug, Clone)]
pub struct WinampSkin {
    pub name: String,
    bitmaps: BTreeMap<&'static str, Image>,
    /// The bitmaps the skin lacks (RVP's own are used).
    pub missing: Vec<&'static str>,
    /// The skin has its own video window frame (`video.bmp`).
    pub has_video: bool,
    pub playlist: PlaylistStyle,
    pub vis_colors: [[u8; 3]; 24],
    /// `region.txt`'s shapes by section (`Normal`, `WindowShade`,
    /// `Equalizer`, `EqualizerWS`).
    pub regions: BTreeMap<String, Region>,
    pub warnings: Vec<String>,
}

/// Whether `path` names a classic skin (`.wsz`) rather than a `.dsf`.
#[must_use]
pub fn is_classic(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wsz"))
}

/// Opens a classic skin as the RVP skin built from it ([`build`]).
///
/// # Errors
/// Fails when the file cannot be read as a ZIP archive.
pub fn open(path: &Path) -> Result<crate::Skin, SkinError> {
    build::skin(&WinampSkin::open(path)?)
}

/// Lower-cased base name of an archive member.
fn base_name(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase()
}

impl WinampSkin {
    /// Opens a `.wsz` (or `.zip`) file, or a folder of an unpacked skin.
    ///
    /// # Errors
    /// Fails when the file cannot be read or is no ZIP archive.
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let io_error = |source| SkinError::Io {
            path: path.display().to_string(),
            source,
        };
        let mut members = Vec::new();
        if path.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(path)
                .map_err(io_error)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect();
            entries.sort();
            for entry in entries {
                let member = entry
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                members.push((member, std::fs::read(&entry).map_err(io_error)?));
            }
        } else {
            let bytes = std::fs::read(path).map_err(io_error)?;
            let archive_error =
                |error: zip::result::ZipError| SkinError::Archive(error.to_string());
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(archive_error)?;
            for index in 0..archive.len() {
                let mut file = archive.by_index(index).map_err(archive_error)?;
                if file.is_dir() || file.size() > 16 << 20 {
                    continue;
                }
                let member = file.name().map_err(archive_error)?.into_owned();
                let mut content = Vec::with_capacity(usize::try_from(file.size()).unwrap_or(0));
                file.read_to_end(&mut content)
                    .map_err(|error| SkinError::Archive(format!("{member}: {error}")))?;
                members.push((member, content));
            }
        }
        Ok(Self::from_members(&name, members))
    }

    /// A skin from its archive's members in archive order.
    #[must_use]
    pub fn from_members(name: &str, members: Vec<(String, Vec<u8>)>) -> Self {
        // Base names; the later member wins.
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (member, bytes) in members {
            let base = base_name(&member);
            let kept = [".bmp", ".txt", ".cur"]
                .iter()
                .any(|extension| base.ends_with(extension));
            if kept {
                files.insert(base, bytes);
            }
        }
        let mut warnings = Vec::new();
        let mut bitmaps = BTreeMap::new();
        let mut missing = Vec::new();
        for name in BITMAPS {
            let decoded =
                files
                    .get(&format!("{name}.bmp"))
                    .and_then(|bytes| match Image::decode_opaque_bmp(name, bytes) {
                        Ok(image) => Some(image),
                        Err(error) => {
                            warnings.push(error.to_string());
                            None
                        }
                    });
            match decoded {
                Some(image) => {
                    bitmaps.insert(name, image);
                }
                None => missing.push(name),
            }
        }
        let has_video = bitmaps.contains_key("video");
        // `balance.bmp` missing: the skin's own `volume.bmp`.
        if !bitmaps.contains_key("balance")
            && let Some(volume) = bitmaps.get("volume").cloned()
        {
            bitmaps.insert("balance", volume);
            missing.retain(|name| *name != "balance");
        }
        for name in &missing {
            bitmaps.insert(name, fallback::bitmap(name));
        }
        let text = |name: &str| {
            files
                .get(name)
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        };
        let playlist = text("pledit.txt").map_or_else(PlaylistStyle::default, |t| parse_pledit(&t));
        let vis_colors = parse_viscolor(text("viscolor.txt").as_deref().unwrap_or(""));
        let regions = text("region.txt").map_or_else(BTreeMap::new, |t| parse_regions(&t));
        Self {
            name: name.to_owned(),
            bitmaps,
            missing,
            has_video,
            playlist,
            vis_colors,
            regions,
            warnings,
        }
    }

    /// A bitmap by base name (the skin's or RVP's). `numbers` is
    /// `nums_ex.bmp` when the skin has one.
    ///
    /// # Panics
    /// Never: every bitmap of [`BITMAPS`] is the skin's or drawn.
    #[must_use]
    pub fn bitmap(&self, name: &str) -> &Image {
        let name = if name == "numbers" && !self.missing.contains(&"nums_ex") {
            "nums_ex"
        } else {
            name
        };
        self.bitmaps
            .get(name)
            .or_else(|| self.bitmaps.get("main"))
            .expect("every bitmap is loaded or drawn")
    }

    /// Whether the time digits come from `nums_ex.bmp` (with a minus sign).
    #[must_use]
    pub fn extended_numbers(&self) -> bool {
        !self.missing.contains(&"nums_ex")
    }
}

/// `#RRGGBB`, `RRGGBB` or any hex number: its low 24 bits.
fn parse_color(value: &str) -> Option<[u8; 3]> {
    let digits: String = value
        .trim()
        .trim_start_matches('#')
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .collect();
    if digits.is_empty() {
        return None;
    }
    let tail = &digits[digits.len().saturating_sub(8)..];
    let number = u32::from_str_radix(tail, 16).ok()?;
    let [_, r, g, b] = number.to_be_bytes();
    Some([r, g, b])
}

/// `pledit.txt`: an INI file; keys and sections ignore case, the first
/// of a key wins.
fn parse_pledit(text: &str) -> PlaylistStyle {
    let mut style = PlaylistStyle::default();
    let mut seen: Vec<String> = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.split(']').next())
        {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        if section != "text" {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        let value = value.split(';').next().unwrap_or("").trim();
        let slot = match key.as_str() {
            "normal" => &mut style.normal,
            "current" => &mut style.current,
            "normalbg" => &mut style.normal_bg,
            "selectedbg" => &mut style.selected_bg,
            "mbfg" => &mut style.mb_fg,
            "mbbg" => &mut style.mb_bg,
            "font" => {
                value.clone_into(&mut style.font);
                continue;
            }
            _ => continue,
        };
        if let Some(color) = parse_color(value) {
            *slot = color;
        }
    }
    style
}

/// The visualizer's default colours (Winamp's documented defaults: black,
/// grey dots, the analyzer from red at the top to green at the bottom, the
/// scope in greys, white peaks), used where `viscolor.txt` stops.
#[must_use]
pub fn default_vis_colors() -> [[u8; 3]; 24] {
    let mut colors = [[0_u8; 3]; 24];
    colors[1] = [24, 33, 41];
    for (row, color) in colors[2..18].iter_mut().enumerate() {
        // Row 2 (the top) is red, row 17 (the bottom) green.
        let t = u8::try_from(row * 255 / 15).unwrap_or(u8::MAX);
        *color = [u8::MAX - t / 2, t, 0];
    }
    for (index, color) in colors[18..23].iter_mut().enumerate() {
        let level = u8::try_from(255 - index * 40).unwrap_or(0);
        *color = [level, level, level];
    }
    colors[23] = [255, 255, 255];
    colors
}

/// `viscolor.txt`: line `i` is colour `i` (`r,g,b`, separated by spaces,
/// commas or tabs; the rest of the line is a comment). The first line
/// without a number ends the list.
fn parse_viscolor(text: &str) -> [[u8; 3]; 24] {
    let mut colors = default_vis_colors();
    for (index, line) in text.lines().take(24).enumerate() {
        let mut numbers = Vec::new();
        let mut rest = line.trim_start_matches([' ', ',', '\t']);
        while numbers.len() < 3 {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                break;
            }
            // Winamp stores the value in a byte.
            let value = digits.parse::<u32>().unwrap_or(0) % 256;
            numbers.push(u8::try_from(value).unwrap_or(0));
            rest = rest[digits.len()..].trim_start_matches([' ', ',', '\t']);
        }
        if numbers.is_empty() {
            break;
        }
        numbers.resize(3, 0);
        colors[index] = [numbers[0], numbers[1], numbers[2]];
    }
    colors
}

/// The numbers at the start of a value: separated by spaces, tabs or
/// commas, with an optional `-`; reading stops at anything else.
fn numbers(value: &str) -> Vec<i32> {
    let mut out = Vec::new();
    let mut rest = value.trim_start_matches([' ', ',', '\t']);
    loop {
        let negative = rest.starts_with('-');
        let body = if negative { &rest[1..] } else { rest };
        let digits: String = body.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            break;
        }
        let value: i32 = digits.parse().unwrap_or(0);
        out.push(if negative { -value } else { value });
        rest = body[digits.len()..].trim_start_matches([' ', ',', '\t']);
    }
    out
}

/// `region.txt`: per section `NumPoints` (points per polygon) and
/// `PointList` (x, y pairs); a section counts only when the counts match.
fn parse_regions(text: &str) -> BTreeMap<String, Region> {
    let mut sections: BTreeMap<String, (Vec<i32>, Vec<i32>)> = BTreeMap::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.split(']').next())
        {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let entry = sections.entry(section.clone()).or_default();
        match key.trim().to_ascii_lowercase().as_str() {
            "numpoints" if entry.0.is_empty() => entry.0 = numbers(value),
            "pointlist" if entry.1.is_empty() => entry.1 = numbers(value),
            _ => {}
        }
    }
    let mut regions = BTreeMap::new();
    for (name, (counts, points)) in sections {
        let total: i32 = counts.iter().sum();
        let canonical = match name.as_str() {
            "normal" => "Normal",
            "windowshade" => "WindowShade",
            "equalizer" => "Equalizer",
            "equalizerws" => "EqualizerWS",
            _ => continue,
        };
        if counts.is_empty()
            || points.len() % 2 != 0
            || usize::try_from(total).ok() != Some(points.len() / 2)
            || counts.iter().any(|count| *count < 0)
        {
            continue;
        }
        let mut polygons = Vec::new();
        let mut pairs = points.as_chunks::<2>().0.iter().map(|[x, y]| (*x, *y));
        for count in counts {
            let polygon: Vec<(i32, i32)> = pairs
                .by_ref()
                .take(usize::try_from(count).unwrap_or(0))
                .collect();
            if polygon.len() >= 3 {
                polygons.push(polygon);
            }
        }
        regions.insert(canonical.to_owned(), polygons);
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_are_found_by_base_name_and_the_later_wins() {
        let main = fallback::bitmap("main").encode_bmp();
        let mut other = fallback::bitmap("main");
        other.fill([1, 2, 3, 255]);
        let skin = WinampSkin::from_members(
            "test",
            vec![
                ("Skin/MAIN.BMP".to_owned(), main),
                ("main.bmp".to_owned(), other.encode_bmp()),
                ("readme.png".to_owned(), vec![1]),
            ],
        );
        assert_eq!(skin.bitmap("main").pixel(0, 0), [1, 2, 3, 255]);
        assert!(!skin.missing.contains(&"main"));
        assert!(skin.missing.contains(&"cbuttons"));
        assert!(!skin.has_video);
        // RVP's own art for the rest, of the standard sizes.
        assert_eq!(skin.bitmap("cbuttons").width(), 136);
    }

    #[test]
    fn balance_falls_back_to_the_skins_volume() {
        let mut volume = fallback::bitmap("volume");
        volume.fill([9, 9, 9, 255]);
        let skin =
            WinampSkin::from_members("test", vec![("volume.bmp".to_owned(), volume.encode_bmp())]);
        assert_eq!(skin.bitmap("balance").pixel(0, 0), [9, 9, 9, 255]);
        assert!(!skin.missing.contains(&"balance"));
    }

    #[test]
    fn text_files_are_read_as_winamp_reads_them() {
        let style = parse_pledit(
            "[Text]\nnormal=#00FF00\nCurrent=FFFFFF\nNORMALBG=#000010\nNormalBG=#FFFFFF\nselectedbg=1A2B3C\nFont=Tahoma\n",
        );
        assert_eq!(style.normal, [0, 255, 0]);
        assert_eq!(style.normal_bg, [0, 0, 0x10]);
        assert_eq!(style.selected_bg, [0x1A, 0x2B, 0x3C]);
        assert_eq!(style.font, "Tahoma");
        let colors = parse_viscolor("0,0,0 // background\n 24, 33,41\n\n255,255,255\n");
        assert_eq!(colors[1], [24, 33, 41]);
        // An empty line ends the list.
        assert_eq!(colors[3], default_vis_colors()[3]);
        let regions = parse_regions(
            "[Normal]\nNumPoints=4, 3 ;comment\nPointList=0,0, 275,0, 275,116, 0,116, 1,1, 5,1, 1,5\n[Equalizer]\nNumPoints=4\nPointList=0,0, 1,1\n",
        );
        assert_eq!(regions["Normal"].len(), 2);
        assert_eq!(regions["Normal"][1], vec![(1, 1), (5, 1), (1, 5)]);
        assert!(!regions.contains_key("Equalizer"));
    }
}
