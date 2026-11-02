//! M12's capture: PotPlayer's Video > Video Capture menu (the clipboard,
//! Save As, the image formats and file names, consecutive images) and its
//! thumbnail image, a contact sheet from M11's thumbnail core.
//!
//! mpv writes PNG, JPEG and WEBP files itself (`screenshot-to-file`, the
//! format from the extension). BMP files, the clipboard, frames saved at
//! their stored size (aspect correction off) and contact sheets go through
//! Qt in the shell from RGBA pixels: `screenshot-raw` pictures, or the
//! sheet [`compose_sheet`] makes. Nothing here runs on the window's
//! thread: the shell's capture thread asks mpv and writes.
//!
//! Source frames are the video's own pictures (mpv's `video` mode, or
//! `subtitles` with "Include message and subtitles in image"); screen
//! frames are the picture as shown (`window`: scaled, with subtitles and
//! mpv's messages).

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rvp_engine::PropertyId;
use rvp_engine::thumbnail::{FrameRequest, Group, Picture, Ticket};
use serde::{Deserialize, Serialize};

use crate::action::ImageFormat;
use crate::app::{App, UiRequest};

/// Which picture a capture takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shot {
    /// The video's frame at its own size.
    Source,
    /// The frame as the window shows it.
    Screen,
}

/// One of the Video Capture menu's commands (the ones before M12 are
/// `Action::SaveFrame`, `SaveScreen`, `SaveFrameAs` and `SetImageFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaptureCommand {
    /// Copy Current Source/Screen Frame to Clipboard.
    Copy(Shot),
    /// Save Current Screen Frame As... (the source frame's is
    /// `Action::SaveFrameAs`).
    SaveScreenAs,
    ToggleNextToMedia,
    ToggleNameDate,
    ToggleNameElapsed,
    ToggleSubtitles,
    ToggleAspect,
    /// Capture Consecutive Images...
    ConsecutiveDialog,
    /// Start/Stop Consecutive Image Capturing.
    ConsecutiveToggle,
    /// Create Thumbnail Image...
    SheetDialog,
    /// Delete the last saved frame (to the Recycle Bin, after asking).
    DeleteLast,
}

/// The contact sheet's settings (Create Thumbnail Image).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SheetSettings {
    pub columns: u32,
    pub rows: u32,
    /// The sheet's width in pixels.
    pub width: u32,
    /// The file's name, size, duration and streams above the pictures.
    pub info: bool,
    /// Each picture's time in its corner.
    pub times: bool,
    /// The frames at their times, not keyframes near them (slower).
    pub exact: bool,
}

impl Default for SheetSettings {
    fn default() -> Self {
        Self {
            columns: 4,
            rows: 4,
            width: 1280,
            info: true,
            times: true,
            exact: false,
        }
    }
}

/// M12's capture settings (`capture` in `settings.json`; the folder and the
/// format are `image-folder` and `image-format`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// PotPlayer's independent Video Capture options, a check mark each.
#[allow(clippy::struct_excessive_bools)]
pub struct CaptureSettings {
    /// "Store captured images into play folders (if available)".
    pub next_to_media: bool,
    /// "Include system date and time in filename".
    pub name_date: bool,
    /// "Include file elapsed time in filename".
    pub name_elapsed: bool,
    /// "Include message and subtitles in image": source frames with the
    /// subtitles.
    pub with_subtitles: bool,
    /// "Correct aspect ratio when capturing"; off, frames keep their
    /// stored size.
    pub correct_aspect: bool,
    /// JPEG and WEBP quality (percent).
    pub quality: u32,
    /// Consecutive images: seconds of playback between two, and how many.
    pub interval: f64,
    pub count: u32,
    pub sheet: SheetSettings,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            next_to_media: false,
            name_date: false,
            name_elapsed: true,
            with_subtitles: false,
            correct_aspect: true,
            quality: 90,
            interval: 1.0,
            count: 10,
            sheet: SheetSettings::default(),
        }
    }
}

impl CaptureSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let clamp = crate::playback::clamp_number;
        clamp_whole(&mut self.quality, 1, 100, "capture.quality", notes);
        clamp(&mut self.interval, 0.1, 3600.0, "capture.interval", notes);
        clamp_whole(&mut self.count, 1, 10_000, "capture.count", notes);
        let sheet = &mut self.sheet;
        clamp_whole(&mut sheet.columns, 1, 10, "capture.sheet.columns", notes);
        clamp_whole(&mut sheet.rows, 1, 20, "capture.sheet.rows", notes);
        clamp_whole(&mut sheet.width, 320, 7680, "capture.sheet.width", notes);
    }
}

fn clamp_whole(value: &mut u32, low: u32, high: u32, name: &str, notes: &mut Vec<String>) {
    if !(low..=high).contains(value) {
        notes.push(format!("{name} must be {low} to {high}"));
        *value = (*value).clamp(low, high);
    }
}

/// A capture for the shell's capture thread.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureJob {
    /// mpv's screenshot flags: `video`, `subtitles` or `window`.
    pub mode: &'static str,
    /// The file; `None` puts the picture on the clipboard.
    pub path: Option<PathBuf>,
    /// The stored size the picture is scaled back to (aspect correction
    /// off, and the video's pixels are not square).
    pub stored: Option<(u32, u32)>,
    /// JPEG and WEBP quality.
    pub quality: u32,
}

impl CaptureJob {
    /// True when mpv writes the file itself: PNG, JPEG or WEBP at the size
    /// mpv gives.
    #[must_use]
    pub fn by_mpv(&self) -> bool {
        self.stored.is_none()
            && self.path.as_deref().is_some_and(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "jpeg" | "webp"
                        )
                    })
            })
    }
}

/// A contact sheet ready to be written.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetImage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// RGBA, rows without padding.
    pub rgba: Arc<Vec<u8>>,
    /// The texts Qt draws over it.
    pub texts: Vec<SheetText>,
    pub quality: u32,
}

/// A line of text on the sheet, drawn by the shell (`rvp_save_image`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SheetText {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Pixels.
    pub size: u32,
    /// `left` or `right`.
    pub align: &'static str,
    pub text: String,
}

/// The file extension of a format.
#[must_use]
pub fn extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Webp => "webp",
    }
}

/// The name of a frame's file without the extension: the video's name,
/// then the date and time (`date`, when the settings want it) and the
/// elapsed time, as the settings say.
#[must_use]
pub fn frame_name(settings: &CaptureSettings, stem: &str, elapsed_ms: u64, date: &str) -> String {
    let mut name = stem.to_owned();
    if settings.name_date {
        name.push(' ');
        name.push_str(date);
    }
    if settings.name_elapsed {
        let _ = write!(
            name,
            " {:02}.{:02}.{:02}.{:03}",
            elapsed_ms / 3_600_000,
            elapsed_ms / 60_000 % 60,
            elapsed_ms / 1000 % 60,
            elapsed_ms % 1000
        );
    }
    name
}

/// A file of `folder` named `name` that does not exist yet: `name (2)`
/// and so on when it does.
#[must_use]
pub fn unused_path(folder: &Path, name: &str, extension: &str) -> PathBuf {
    let mut file = folder.join(format!("{name}.{extension}"));
    let mut counter = 2;
    while file.exists() {
        file = folder.join(format!("{name} ({counter}).{extension}"));
        counter += 1;
    }
    file
}

/// `time` as `YYYY-MM-DD HH.MM.SS` in UTC, for when the window gave RVP no
/// clock (headless tests).
#[must_use]
pub fn utc_text(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let rest = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}.{:02}.{:02}",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// The calendar date of a day number counted from 1970-01-01 (Howard
/// Hinnant's `civil_from_days`).
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        year,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

/// RGBA pixels (opaque) of an 8-bit `screenshot-raw` picture.
#[must_use]
pub fn rgba_pixels(image: &rvp_engine::Image) -> Vec<u8> {
    let mut rgba = vec![0; image.width * image.height * 4];
    let bgr = matches!(image.format.as_str(), "bgr0" | "bgra");
    for (y, row) in rgba.chunks_exact_mut(image.width * 4).enumerate() {
        let start = y * image.stride;
        let Some(source) = image.data.get(start..start + image.width * 4) else {
            break;
        };
        for (to, from) in row
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(source.as_chunks::<4>().0)
        {
            *to = if bgr {
                [from[2], from[1], from[0], 255]
            } else {
                [from[0], from[1], from[2], 255]
            };
        }
    }
    rgba
}

/// RGBA pixels scaled to `to` (bilinear): a frame back to its stored size.
#[must_use]
pub fn scale_rgba(rgba: &[u8], from: (u32, u32), to: (u32, u32)) -> Vec<u8> {
    let (width, height) = (from.0 as usize, from.1 as usize);
    let (new_width, new_height) = (to.0 as usize, to.1 as usize);
    let mut out = vec![0; new_width * new_height * 4];
    if width == 0 || height == 0 || rgba.len() < width * height * 4 {
        return out;
    }
    // Pixel coordinates are far inside f64's exact range.
    #[allow(clippy::cast_precision_loss)]
    let ratio = (
        width as f64 / new_width.max(1) as f64,
        height as f64 / new_height.max(1) as f64,
    );
    for (y, row) in out.chunks_exact_mut(new_width * 4).enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let source_y = ((y as f64 + 0.5) * ratio.1 - 0.5).max(0.0);
        // Floors of non-negative coordinates inside the picture.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let top = (source_y as usize).min(height - 1);
        let bottom = (top + 1).min(height - 1);
        #[allow(clippy::cast_precision_loss)]
        let weight_y = source_y - top as f64;
        for (x, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let source_x = ((x as f64 + 0.5) * ratio.0 - 0.5).max(0.0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let left = (source_x as usize).min(width - 1);
            let right = (left + 1).min(width - 1);
            #[allow(clippy::cast_precision_loss)]
            let weight_x = source_x - left as f64;
            for channel in 0..4 {
                let at = |px: usize, py: usize| f64::from(rgba[(py * width + px) * 4 + channel]);
                let upper = at(left, top) * (1.0 - weight_x) + at(right, top) * weight_x;
                let lower = at(left, bottom) * (1.0 - weight_x) + at(right, bottom) * weight_x;
                // A weighted mean of bytes is a byte.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let value = (upper * (1.0 - weight_y) + lower * weight_y).round() as u8;
                pixel[channel] = value;
            }
        }
    }
    out
}

/// A picture's place on the contact sheet and the time it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetTile {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub time: f64,
}

/// The contact sheet's geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetLayout {
    pub width: u32,
    pub height: u32,
    /// The space between pictures and around them.
    pub gap: u32,
    /// The height of a line of the header's text, and of the header.
    pub line: u32,
    pub header: u32,
    pub tiles: Vec<SheetTile>,
}

/// The sheet for a video of display aspect `aspect` lasting `duration`
/// seconds, with `lines` lines of information above the pictures. The
/// pictures share the sheet's width; their times split the duration
/// evenly, leaving out the very start and end.
#[must_use]
pub fn sheet_layout(
    settings: &SheetSettings,
    aspect: f64,
    duration: f64,
    lines: u32,
) -> SheetLayout {
    let width = settings.width.max(64);
    let columns = settings.columns.max(1);
    let rows = settings.rows.max(1);
    let gap = (width / 160).max(4);
    let line = (width / 64).max(14);
    let header = if lines > 0 { lines * line + gap * 2 } else { 0 };
    let tile_width = (width.saturating_sub(gap * (columns + 1)) / columns).max(1);
    let aspect = if aspect.is_finite() && aspect > 0.01 {
        aspect
    } else {
        16.0 / 9.0
    };
    // Tile sizes are far inside f64's exact range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let tile_height = ((f64::from(tile_width) / aspect).round() as u32).max(1);
    let count = columns * rows;
    let mut tiles = Vec::with_capacity(count as usize);
    for index in 0..count {
        let (column, row) = (index % columns, index / columns);
        tiles.push(SheetTile {
            x: gap + column * (tile_width + gap),
            y: header + gap + row * (tile_height + gap),
            width: tile_width,
            height: tile_height,
            time: duration.max(0.0) * f64::from(index + 1) / f64::from(count + 1),
        });
    }
    SheetLayout {
        width,
        height: header + gap + rows * (tile_height + gap),
        gap,
        line,
        header,
        tiles,
    }
}

/// The sheet's background.
const SHEET_BACKGROUND: [u8; 4] = [24, 24, 24, 255];

/// The sheet's pixels: each picture centred in its tile (a missing one
/// leaves the tile dark).
#[must_use]
pub fn compose_sheet(layout: &SheetLayout, pictures: &[Option<Arc<Picture>>]) -> Vec<u8> {
    let width = layout.width as usize;
    let mut rgba = SHEET_BACKGROUND.repeat(width * layout.height as usize);
    for (tile, picture) in layout.tiles.iter().zip(pictures) {
        let Some(picture) = picture else {
            continue;
        };
        let shown = (
            picture.width.min(tile.width),
            picture.height.min(tile.height),
        );
        let left = (tile.x + (tile.width - shown.0) / 2) as usize;
        let top = (tile.y + (tile.height - shown.1) / 2) as usize;
        let shown = (shown.0 as usize, shown.1 as usize);
        let source_row = picture.width as usize * 4;
        for y in 0..shown.1 {
            let from = y * source_row;
            let to = ((top + y) * width + left) * 4;
            if let (Some(source), Some(target)) = (
                picture.rgba.get(from..from + shown.0 * 4),
                rgba.get_mut(to..to + shown.0 * 4),
            ) {
                target.copy_from_slice(source);
            }
        }
    }
    rgba
}

/// `seconds` as `H:MM:SS`.
fn clock(seconds: f64) -> String {
    // Media times are far inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = seconds.max(0.0).round() as u64;
    format!("{}:{:02}:{:02}", whole / 3600, whole / 60 % 60, whole % 60)
}

/// The consecutive capture under way.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Consecutive {
    /// The playback time of the next image.
    next: f64,
    /// Images still to take.
    left: u32,
    taken: u32,
}

/// The contact sheet being made: its pictures arrive from the thumbnail
/// core.
#[derive(Debug, Clone)]
pub(crate) struct SheetJob {
    layout: SheetLayout,
    texts: Vec<SheetText>,
    path: PathBuf,
    pictures: Vec<Option<Arc<Picture>>>,
    tickets: BTreeMap<Ticket, usize>,
}

/// M12's capture state for this run.
#[derive(Debug, Default)]
pub(crate) struct CaptureState {
    pub(crate) consecutive: Option<Consecutive>,
    pub(crate) sheet: Option<SheetJob>,
    /// The album's entries (path, duration) waiting for their contact
    /// sheets, made one after another (M14, the playlist's Create
    /// Thumbnail Image(s)); empty: the sheet is the playing file's.
    pub(crate) sheet_queue: VecDeque<(String, f64)>,
    /// The file the last capture went to (Delete the last saved frame).
    pub(crate) last_saved: Option<PathBuf>,
}

impl App {
    /// The local date and time for file names: the window's clock, or
    /// UTC without one.
    fn date_text(&self) -> String {
        self.clock
            .map_or_else(|| utc_text(SystemTime::now()), |clock| clock())
    }

    /// The folder captured images of the playing file go to: its own
    /// folder with "Store captured images into play folders" (when it is a
    /// local folder), else the image folder.
    pub(crate) fn capture_folder(&self) -> PathBuf {
        if self.settings.capture.next_to_media
            && let Some(path) = self.model.loaded_path()
            && !crate::playlist::is_url(path)
            && let Some(folder) = Path::new(path).parent()
            && folder.is_dir()
        {
            return folder.to_path_buf();
        }
        self.image_folder()
    }

    /// A new file for the current frame, named as the settings say;
    /// `None` without a video.
    pub(crate) fn frame_path(&self) -> Option<PathBuf> {
        let path = self.model.loaded_path()?;
        self.model.int(PropertyId::VideoWidth)?;
        let stem = Path::new(crate::view::file_name(path))
            .file_stem()
            .map_or_else(
                || "frame".to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            );
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        // Milliseconds of a media time are far inside u64.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let millis = (time.max(0.0) * 1000.0).round() as u64;
        let name = frame_name(&self.settings.capture, &stem, millis, &self.date_text());
        Some(unused_path(
            &self.capture_folder(),
            &name,
            extension(self.settings.image_format),
        ))
    }

    /// Gives the app the window's clock for the dates in file names.
    pub fn set_clock(&mut self, clock: fn() -> String) {
        self.clock = Some(clock);
    }

    /// The capture of `shot` into `path` (or the clipboard).
    #[must_use]
    pub fn capture_job(&self, shot: Shot, path: Option<PathBuf>) -> CaptureJob {
        let capture = &self.settings.capture;
        let mode = match shot {
            Shot::Source if capture.with_subtitles => "subtitles",
            Shot::Source => "video",
            Shot::Screen => "window",
        };
        let stored = if capture.correct_aspect || shot == Shot::Screen {
            None
        } else {
            self.stored_size()
        };
        CaptureJob {
            mode,
            path,
            stored,
            quality: capture.quality,
        }
    }

    /// The video's stored size when its pixels are not square (the frame
    /// mpv captures is scaled to the display aspect).
    fn stored_size(&self) -> Option<(u32, u32)> {
        let width = u32::try_from(self.model.int(PropertyId::VideoWidth)?).ok()?;
        let height = u32::try_from(self.model.int(PropertyId::VideoHeight)?).ok()?;
        let aspect = self.model.double(PropertyId::VideoAspect)?;
        let square = f64::from(width) / f64::from(height.max(1));
        ((aspect - square).abs() > 0.01).then_some((width, height))
    }

    /// The Save As dialog for the current source or screen frame.
    pub(crate) fn save_as(&self, shot: Shot) -> Option<UiRequest> {
        let path = self.frame_path()?;
        Some(UiRequest::SaveFrameDialog { path, shot })
    }

    /// Saves the current frame (Ctrl+E, Ctrl+Alt+E).
    pub(crate) fn save_shot(&mut self, shot: Shot) -> Option<UiRequest> {
        let path = self.frame_path()?;
        self.capture.last_saved = Some(path.clone());
        Some(UiRequest::Capture(self.capture_job(shot, Some(path))))
    }

    /// A frame saved through the Save As dialog.
    pub fn frame_saved_as(&mut self, path: &Path) {
        self.capture.last_saved = Some(path.to_path_buf());
    }

    /// The Video Capture menu's M12 commands.
    pub(crate) fn capture_command(&mut self, command: CaptureCommand) -> Option<UiRequest> {
        let capture = &mut self.settings.capture;
        let toggled = |name: &str, on: bool| format!("{name}: {}", if on { "on" } else { "off" });
        let notice = match command {
            CaptureCommand::Copy(shot) => {
                self.model.int(PropertyId::VideoWidth)?;
                return Some(UiRequest::Capture(self.capture_job(shot, None)));
            }
            CaptureCommand::SaveScreenAs => return self.save_as(Shot::Screen),
            CaptureCommand::ToggleNextToMedia => {
                capture.next_to_media = !capture.next_to_media;
                toggled("Store images in play folders", capture.next_to_media)
            }
            CaptureCommand::ToggleNameDate => {
                capture.name_date = !capture.name_date;
                toggled("Date and time in file names", capture.name_date)
            }
            CaptureCommand::ToggleNameElapsed => {
                capture.name_elapsed = !capture.name_elapsed;
                toggled("Elapsed time in file names", capture.name_elapsed)
            }
            CaptureCommand::ToggleSubtitles => {
                capture.with_subtitles = !capture.with_subtitles;
                toggled("Subtitles in images", capture.with_subtitles)
            }
            CaptureCommand::ToggleAspect => {
                capture.correct_aspect = !capture.correct_aspect;
                toggled(
                    "Correct aspect ratio when capturing",
                    capture.correct_aspect,
                )
            }
            CaptureCommand::ConsecutiveDialog => {
                return Some(UiRequest::CaptureDialog("consecutive"));
            }
            CaptureCommand::ConsecutiveToggle => {
                return Some(UiRequest::Osd(self.toggle_consecutive()));
            }
            CaptureCommand::SheetDialog => {
                self.model.loaded_path()?;
                self.capture.sheet_queue.clear();
                return Some(UiRequest::CaptureDialog("sheet"));
            }
            CaptureCommand::DeleteLast => {
                return Some(match self.capture.last_saved.take() {
                    Some(path) if path.is_file() => UiRequest::ConfirmRecycle {
                        files: vec![path],
                        entries: Vec::new(),
                    },
                    _ => UiRequest::Osd("No saved frame to delete".to_owned()),
                });
            }
        };
        self.save_settings();
        Some(UiRequest::Osd(notice))
    }

    /// The check mark of a capture command.
    pub(crate) fn capture_checked(&self, command: CaptureCommand) -> Option<bool> {
        let capture = &self.settings.capture;
        match command {
            CaptureCommand::ToggleNextToMedia => Some(capture.next_to_media),
            CaptureCommand::ToggleNameDate => Some(capture.name_date),
            CaptureCommand::ToggleNameElapsed => Some(capture.name_elapsed),
            CaptureCommand::ToggleSubtitles => Some(capture.with_subtitles),
            CaptureCommand::ToggleAspect => Some(capture.correct_aspect),
            CaptureCommand::ConsecutiveToggle => Some(self.capture.consecutive.is_some()),
            _ => None,
        }
    }

    /// Starts consecutive images from now, or stops them.
    fn toggle_consecutive(&mut self) -> String {
        if let Some(run) = self.capture.consecutive.take() {
            return format!("Consecutive capture stopped ({} images)", run.taken);
        }
        if self.model.int(PropertyId::VideoWidth).is_none() {
            return "Consecutive capture needs a video".to_owned();
        }
        let now = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        self.capture.consecutive = Some(Consecutive {
            next: now,
            left: self.settings.capture.count,
            taken: 0,
        });
        let capture = &self.settings.capture;
        format!(
            "Consecutive capture: {} images, every {} s",
            capture.count, capture.interval
        )
    }

    /// The consecutive capture dialog's answer (JSON: `interval`, `count`):
    /// kept, and the capture starts.
    pub fn consecutive_answer(&mut self, json: &str) -> Option<UiRequest> {
        let value: serde_json::Value = serde_json::from_str(json).ok()?;
        let capture = &mut self.settings.capture;
        if let Some(interval) = value.get("interval").and_then(serde_json::Value::as_f64) {
            capture.interval = interval;
        }
        if let Some(count) = value
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| u32::try_from(count).ok())
        {
            capture.count = count;
        }
        let mut notes = Vec::new();
        capture.validate(&mut notes);
        self.save_settings();
        self.capture.consecutive = None;
        Some(UiRequest::Osd(self.toggle_consecutive()))
    }

    /// Takes the next consecutive image when playback reached its time.
    pub(crate) fn capture_tick(&mut self, requests: &mut Vec<UiRequest>) {
        let Some(run) = &self.capture.consecutive else {
            return;
        };
        let Some(time) = self.model.double(PropertyId::TimePos) else {
            return;
        };
        if time + 0.001 < run.next {
            return;
        }
        let Some(request) = self.save_shot(Shot::Source) else {
            self.capture.consecutive = None;
            return;
        };
        requests.push(request);
        let interval = self.settings.capture.interval;
        let Some(run) = &mut self.capture.consecutive else {
            return;
        };
        run.taken += 1;
        run.left = run.left.saturating_sub(1);
        run.next = time + interval;
        if run.left == 0 {
            let taken = run.taken;
            self.capture.consecutive = None;
            requests.push(UiRequest::Osd(format!(
                "Consecutive capture done ({taken} images)"
            )));
        }
    }

    /// A new file stops consecutive images (PotPlayer captures one file).
    pub(crate) fn capture_file_loaded(&mut self) {
        self.capture.consecutive = None;
    }

    /// The thumbnail image dialog's answer (JSON with the sheet's
    /// settings): kept, and the pictures are asked for.
    pub fn sheet_answer(&mut self, json: &str) -> Option<UiRequest> {
        let sheet: SheetSettings = serde_json::from_str(json).ok()?;
        self.settings.capture.sheet = sheet;
        let mut notes = Vec::new();
        self.settings.capture.validate(&mut notes);
        self.save_settings();
        if self.capture.sheet_queue.is_empty() {
            return Some(UiRequest::Osd(self.start_sheet()));
        }
        let count = self.capture.sheet_queue.len();
        self.next_queued_sheet();
        Some(UiRequest::Osd(format!(
            "Making {count} thumbnail image(s), one after another"
        )))
    }

    /// The playlist's Create Thumbnail Image(s) for `targets` (entries of
    /// the shown album): the local files whose duration is known get a
    /// sheet each, with the dialog's settings.
    pub(crate) fn sheets_for_entries(&mut self, targets: &[usize]) -> UiRequest {
        let entries = &self.library().shown().entries;
        let queue: VecDeque<(String, f64)> = targets
            .iter()
            .filter_map(|&index| entries.get(index))
            .filter(|entry| !crate::playlist::is_url(&entry.path))
            .filter_map(|entry| {
                let duration = entry.duration.filter(|duration| *duration > 0.0)?;
                Some((entry.path.clone(), duration))
            })
            .collect();
        if queue.is_empty() {
            return UiRequest::Osd(
                "Thumbnail images need local files with known durations (Detect file durations)"
                    .to_owned(),
            );
        }
        self.capture.sheet_queue = queue;
        UiRequest::CaptureDialog("sheet")
    }

    /// Starts the next queued entry's sheet, if any.
    fn next_queued_sheet(&mut self) {
        while let Some((path, duration)) = self.capture.sheet_queue.pop_front() {
            if self.thumbs.core().is_none() {
                self.capture.sheet_queue.clear();
                return;
            }
            // An entry that is not a file any more is passed over.
            if !Path::new(&path).is_file() {
                continue;
            }
            let settings = self.settings.capture.sheet.clone();
            let info = if settings.info {
                file_lines(&path, duration)
            } else {
                Vec::new()
            };
            // The entry's shape is unknown until it is opened: 16:9 tiles,
            // each picture centred in its own.
            self.begin_sheet(&path, duration, 16.0 / 9.0, &info);
            return;
        }
    }

    /// Lays the sheet out and asks the thumbnail core for its pictures.
    fn start_sheet(&mut self) -> String {
        let Some(path) = self.model.loaded_path().map(str::to_owned) else {
            return "No file plays".to_owned();
        };
        let Some(duration) = self.model.double(PropertyId::Duration).filter(|d| *d > 0.0) else {
            return "The thumbnail image needs the file's duration".to_owned();
        };
        if self.thumbs.core().is_none() {
            return "The thumbnail image needs the thumbnail core, which is not running".to_owned();
        }
        let info = if self.settings.capture.sheet.info {
            self.sheet_info(&path, duration)
        } else {
            Vec::new()
        };
        let aspect = self
            .model
            .double(PropertyId::VideoAspect)
            .unwrap_or(16.0 / 9.0);
        self.begin_sheet(&path, duration, aspect, &info)
    }

    /// Lays out the sheet of `path` and asks the thumbnail core for its
    /// pictures.
    fn begin_sheet(&mut self, path: &str, duration: f64, aspect: f64, info: &[String]) -> String {
        let settings = self.settings.capture.sheet.clone();
        let lines = u32::try_from(info.len()).unwrap_or(0);
        let layout = sheet_layout(&settings, aspect, duration, lines);
        let texts = sheet_texts(&layout, info, settings.times);
        let stem = Path::new(crate::view::file_name(path))
            .file_stem()
            .map_or_else(
                || "thumbnails".to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            );
        let format = match self.settings.image_format {
            // Qt writes the sheet; it has no WEBP writer.
            ImageFormat::Webp => ImageFormat::Png,
            other => other,
        };
        // A queued entry's sheet goes beside it with "Store captured
        // images into play folders", as the playing file's does.
        let folder = match Path::new(path).parent() {
            Some(folder)
                if self.settings.capture.next_to_media
                    && !crate::playlist::is_url(path)
                    && folder.is_dir() =>
            {
                folder.to_path_buf()
            }
            _ => self.image_folder(),
        };
        let file = unused_path(&folder, &stem, extension(format));
        let mut tickets = BTreeMap::new();
        if let Some(core) = self.thumbs.core() {
            core.cancel(Group::Sheet);
            for (index, tile) in layout.tiles.iter().enumerate() {
                let ticket = core.request(FrameRequest {
                    path: path.to_owned(),
                    time: tile.time,
                    max_width: tile.width,
                    max_height: tile.height,
                    fill: false,
                    exact: settings.exact,
                    group: Group::Sheet,
                });
                tickets.insert(ticket, index);
            }
        }
        let count = layout.tiles.len();
        self.capture.sheet = Some(SheetJob {
            layout,
            texts,
            path: file,
            pictures: vec![None; count],
            tickets,
        });
        format!("Making the thumbnail image ({count} pictures)")
    }

    /// The header's lines: the file, its size and duration, its streams.
    fn sheet_info(&self, path: &str, duration: f64) -> Vec<String> {
        let model = &self.model;
        let mut lines = file_lines(path, duration);
        if let (Some(width), Some(height)) = (
            model.int(PropertyId::VideoWidth),
            model.int(PropertyId::VideoHeight),
        ) {
            let codec = model.text(PropertyId::VideoFormat).unwrap_or("video");
            let fps = model
                .double(PropertyId::ContainerFps)
                .map_or_else(String::new, |fps| format!(", {fps:.3} fps"));
            lines.push(format!("Video: {codec}, {width}x{height}{fps}"));
        }
        if let Some(codec) = model.text(PropertyId::AudioCodecName) {
            let rate = model
                .int(PropertyId::AudioSampleRate)
                .map_or_else(String::new, |rate| format!(", {rate} Hz"));
            let channels = match model.int(PropertyId::AudioChannels) {
                Some(1) => ", mono".to_owned(),
                Some(2) => ", stereo".to_owned(),
                Some(channels) => format!(", {channels} channels"),
                None => String::new(),
            };
            lines.push(format!("Audio: {codec}{rate}{channels}"));
        }
        lines
    }

    /// Takes a sheet picture from the thumbnail core (`None`: it failed);
    /// when the last one came, the sheet is ready to be written.
    pub(crate) fn sheet_picture(
        &mut self,
        ticket: Ticket,
        picture: Option<Arc<Picture>>,
    ) -> Option<SheetImage> {
        let job = self.capture.sheet.as_mut()?;
        let index = job.tickets.remove(&ticket)?;
        if let Some(slot) = job.pictures.get_mut(index) {
            *slot = picture;
        }
        if !job.tickets.is_empty() {
            return None;
        }
        let job = self.capture.sheet.take()?;
        let rgba = compose_sheet(&job.layout, &job.pictures);
        self.next_queued_sheet();
        Some(SheetImage {
            path: job.path,
            width: job.layout.width,
            height: job.layout.height,
            rgba: Arc::new(rgba),
            texts: job.texts,
            quality: self.settings.capture.quality,
        })
    }
}

/// A sheet's first lines: the file, its size and its duration.
fn file_lines(path: &str, duration: f64) -> Vec<String> {
    let mut lines = vec![format!("File: {}", crate::view::file_name(path))];
    let size = std::fs::metadata(path).map(|meta| meta.len()).ok();
    lines.push(match size {
        // A file size shown in MB.
        #[allow(clippy::cast_precision_loss)]
        Some(bytes) => format!(
            "Size: {:.1} MB ({bytes} bytes), duration: {}",
            bytes as f64 / 1_048_576.0,
            clock(duration)
        ),
        None => format!("Duration: {}", clock(duration)),
    });
    lines
}

/// The header's lines and each picture's time, placed on the sheet.
fn sheet_texts(layout: &SheetLayout, info: &[String], times: bool) -> Vec<SheetText> {
    let gap = i32::try_from(layout.gap).unwrap_or(4);
    let line = i32::try_from(layout.line).unwrap_or(14);
    let width = i32::try_from(layout.width).unwrap_or(0);
    let mut texts = Vec::new();
    for (row, text) in (0..).zip(info) {
        texts.push(SheetText {
            x: gap,
            y: gap + row * line,
            width: width - 2 * gap,
            height: line,
            size: layout.line * 3 / 4,
            align: "left",
            text: text.clone(),
        });
    }
    if times {
        for tile in &layout.tiles {
            let size = (tile.height / 8).clamp(10, 32);
            let height = i32::try_from(size * 4 / 3).unwrap_or(14);
            texts.push(SheetText {
                x: i32::try_from(tile.x).unwrap_or(0),
                y: i32::try_from(tile.y + tile.height).unwrap_or(0) - height - 2,
                width: i32::try_from(tile.width).unwrap_or(0) - 4,
                height,
                size,
                align: "right",
                text: clock(tile.time),
            });
        }
    }
    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_names_follow_the_settings() {
        let mut settings = CaptureSettings::default();
        let date = "2026-10-18 15.30.45";
        assert_eq!(
            frame_name(&settings, "clip", 65_250, date),
            "clip 00.01.05.250"
        );
        settings.name_date = true;
        assert_eq!(
            frame_name(&settings, "clip", 3_723_004, date),
            "clip 2026-10-18 15.30.45 01.02.03.004"
        );
        settings.name_elapsed = false;
        assert_eq!(
            frame_name(&settings, "clip", 0, date),
            "clip 2026-10-18 15.30.45"
        );
        settings.name_date = false;
        assert_eq!(frame_name(&settings, "clip", 0, date), "clip");
    }

    #[test]
    fn utc_dates_are_the_calendars() {
        let at = |seconds: u64| utc_text(UNIX_EPOCH + std::time::Duration::from_secs(seconds));
        assert_eq!(at(0), "1970-01-01 00.00.00");
        // 2000-02-29 12:34:56 and 2026-10-18 00:00:00 UTC.
        assert_eq!(at(951_827_696), "2000-02-29 12.34.56");
        assert_eq!(at(1_792_281_600), "2026-10-18 00.00.00");
    }

    #[test]
    fn sheets_share_the_width_and_split_the_duration() {
        let settings = SheetSettings {
            columns: 4,
            rows: 3,
            width: 1280,
            ..SheetSettings::default()
        };
        let layout = sheet_layout(&settings, 16.0 / 9.0, 130.0, 4);
        assert_eq!((layout.gap, layout.line), (8, 20));
        assert_eq!(layout.header, 4 * 20 + 16);
        assert_eq!(layout.tiles.len(), 12);
        let first = &layout.tiles[0];
        assert_eq!(
            (first.x, first.y, first.width, first.height),
            (8, 104, 310, 174)
        );
        let last = &layout.tiles[11];
        assert_eq!(last.x + last.width + layout.gap, layout.width);
        assert_eq!(last.y + last.height + layout.gap, layout.height);
        // No tile overlaps the next one in its row or below it.
        for pair in layout.tiles.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            assert!(b.x >= a.x + a.width || b.y >= a.y + a.height, "{a:?} {b:?}");
            assert!(b.time > a.time);
        }
        assert!((first.time - 10.0).abs() < 1e-9);
        assert!((last.time - 120.0).abs() < 1e-9);
        // Without information the pictures start at the top.
        let bare = sheet_layout(&settings, 4.0 / 3.0, 130.0, 0);
        assert_eq!(
            (bare.header, bare.tiles[0].y, bare.tiles[0].height),
            (0, 8, 233)
        );
    }

    #[test]
    fn pictures_sit_in_the_middle_of_their_tiles() {
        let settings = SheetSettings {
            columns: 2,
            rows: 1,
            width: 340,
            ..SheetSettings::default()
        };
        let layout = sheet_layout(&settings, 2.0, 10.0, 0);
        assert_eq!(layout.gap, 4);
        assert_eq!((layout.tiles[0].width, layout.tiles[0].height), (164, 82));
        let picture = Picture {
            path: String::new(),
            time: 1.0,
            shown: 1.0,
            width: 101,
            height: 81,
            rgba: [255, 0, 0, 255].repeat(101 * 81),
        };
        let rgba = compose_sheet(&layout, &[Some(Arc::new(picture)), None]);
        let pixel = |x: usize, y: usize| &rgba[(y * 340 + x) * 4..(y * 340 + x) * 4 + 4];
        // The 101 pixels wide picture starts 31 pixels into its tile.
        assert_eq!(pixel(4 + 30, 40), SHEET_BACKGROUND);
        assert_eq!(pixel(4 + 31, 40), [255, 0, 0, 255]);
        assert_eq!(pixel(4 + 131, 40), [255, 0, 0, 255]);
        assert_eq!(pixel(4 + 132, 40), SHEET_BACKGROUND);
        // The second tile has no picture.
        assert_eq!(pixel(4 + 164 + 4 + 80, 40), SHEET_BACKGROUND);
    }

    #[test]
    fn frames_scale_back_to_their_stored_size() {
        // Two columns, black then white, widened to four and back.
        let rgba = [[0, 0, 0, 255], [255, 255, 255, 255]].concat();
        let wide = scale_rgba(&rgba, (2, 1), (4, 1));
        assert_eq!(wide.len(), 16);
        assert_eq!(&wide[0..4], &[0, 0, 0, 255]);
        assert_eq!(&wide[12..16], &[255, 255, 255, 255]);
        assert_eq!(scale_rgba(&wide, (4, 1), (2, 1))[0], 32);
    }

    #[test]
    fn jobs_go_to_mpv_only_for_its_formats() {
        let job = |path: &str, stored| CaptureJob {
            mode: "video",
            path: Some(PathBuf::from(path)),
            stored,
            quality: 90,
        };
        assert!(job("a.png", None).by_mpv());
        assert!(job("a.JPG", None).by_mpv());
        assert!(job("a.webp", None).by_mpv());
        assert!(!job("a.bmp", None).by_mpv());
        assert!(!job("a.png", Some((720, 480))).by_mpv());
        let clipboard = CaptureJob {
            path: None,
            ..job("a.png", None)
        };
        assert!(!clipboard.by_mpv());
    }
}
