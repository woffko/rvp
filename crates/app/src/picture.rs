//! M12's picture geometry: PotPlayer's Aspect Ratio and Frame Size menus,
//! and Video > Resize, Extend/Crop, Bottom Margin and the rotation by
//! aspect ratio.
//!
//! * The aspect ratio (`video-aspect-override`, a custom one too) and the
//!   aspect ratio mode: inside the area (`keepaspect`), filling it by
//!   keeping the aspect (`panscan`), stretched, or the frame kept at 0.5x
//!   to 2x of the video's own size (`video-unscaled` with `video-zoom`).
//!   Fullscreen (Stretch) and (Stretch by Keeping AR) bring their mode
//!   while they last, and in fullscreen the chosen mode can wait until the
//!   margin around the video is small. "Adjust AR to Window Size"
//!   stretches the video to the window while it is not fullscreen.
//! * The output device's pixel aspect (`monitorpixelaspect`): square
//!   pixels, a 4:3 or 16:9 television fed this screen's resolution, or the
//!   screen's measured size against its resolution.
//! * The frame (`video-zoom`, `video-scale-x/y`, `video-pan-x/y`), its
//!   presets, and whether the next file keeps it.
//! * Filters RVP puts before its effects, so before SVP's `@svp`:
//!   `@rvpresize` resizes the frame (Video > Resize), `@rvpcrop` crops or
//!   extends it to an aspect ratio or a size at the chosen position, and
//!   `@rvpmargin` adds Video > Bottom Margin's black below the picture,
//!   where text subtitles go. Their expressions read each file's size and
//!   pixel aspect, so they stay the same from file to file.
//! * The window's margins (`video-margin-ratio-*`): Frame Size's screen
//!   margins, and the room "Provide Bottom Margin for Subtitles" keeps
//!   for subtitles: some lines under the video, or the video at the top
//!   (`video-align-y`) so the letterbox is below it.
//! * Rotation by aspect ratio: a video whose orientation is not the
//!   screen's turns by 90 (or 270) degrees.

use std::fmt::Write as _;

use rvp_engine::{Engine, EngineError, PropertyId};
use serde::{Deserialize, Serialize};

use crate::action::{Aspect, Fit};
use crate::app::{App, UiRequest};

/// The labels of the picture's filters, in the order they run.
pub const RESIZE_LABEL: &str = "rvpresize";
pub const CROP_LABEL: &str = "rvpcrop";
pub const MARGIN_LABEL: &str = "rvpmargin";

/// The output device whose pixels the picture is corrected for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputDevice {
    /// Square pixels.
    #[default]
    Monitor,
    /// A 4:3 television showing this screen's resolution.
    Tv4x3,
    /// A 16:9 television showing this screen's resolution.
    Tv16x9,
    /// The screen's measured size against its resolution.
    Auto,
}

/// How Video > Extend/Crop changes the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtendMethod {
    #[default]
    None,
    /// Black bars up to the aspect ratio.
    Extend,
    /// Cut down to the aspect ratio.
    Crop,
    /// Cut or extended to a size in pixels.
    Pixels,
}

/// The room "Provide Bottom Margin for Subtitles" keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubtitleRoom {
    #[default]
    OneLine,
    TwoLines,
    ThreeLines,
    /// The video at the top, the letterbox below it, while subtitles show.
    Available,
    /// The same, with or without subtitles.
    AvailableAlways,
}

/// Screen Rotation's rotation by aspect ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoRotate {
    #[default]
    Off,
    /// 0 degrees when the video's orientation is the screen's, else 90.
    Quarter,
    /// 180 degrees when it is, else 270.
    Half,
}

/// When Video > Resize resizes the frame to its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResizeWhen {
    /// "Do nothing".
    #[default]
    Never,
    /// Only frames smaller than the size grow.
    Smaller,
    /// Only frames larger than the size shrink.
    Larger,
    Always,
}

/// The size Video > Resize resizes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResizeTarget {
    /// The screen's resolution.
    #[default]
    Display,
    /// The video's size times a factor.
    Multiply,
    /// A size in pixels.
    Size,
}

/// A side of the picture (screen margins, moves).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Top,
    Left,
    Bottom,
    Right,
}

/// One of Frame Size's presets: the frame's width and height in percent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FramePreset {
    pub name: String,
    pub width: f64,
    pub height: f64,
}

impl Default for FramePreset {
    fn default() -> Self {
        Self {
            name: "(Default)".to_owned(),
            width: 100.0,
            height: 100.0,
        }
    }
}

/// The screen margins in percent of the video area.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Margins {
    pub top: f64,
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
}

impl Margins {
    fn get_mut(&mut self, side: Side) -> &mut f64 {
        match side {
            Side::Top => &mut self.top,
            Side::Left => &mut self.left,
            Side::Bottom => &mut self.bottom,
            Side::Right => &mut self.right,
        }
    }

    fn is_zero(&self) -> bool {
        [self.top, self.left, self.bottom, self.right]
            .iter()
            .all(|value| value.abs() < f64::EPSILON)
    }
}

/// The frame's zoom, stretch and position (Frame Size), kept from file to
/// file with "Remember video size/position changes".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Frame {
    /// mpv `video-zoom` (log2 of the factor).
    pub zoom: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub pan_x: f64,
    pub pan_y: f64,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            zoom: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

/// Video > Resize's options (Preferences: Video > Resize).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// PotPlayer's Resize page: independent options, a check box each.
#[allow(clippy::struct_excessive_bools)]
pub struct Resize {
    pub when: ResizeWhen,
    pub target: ResizeTarget,
    /// "Multiply by".
    pub factor: f64,
    /// "Specify size".
    pub width: u32,
    pub height: u32,
    /// "Auto calculate vertical size": the height follows the width.
    pub auto_height: bool,
    /// FFmpeg's scaler (`bicubic`, `bilinear`, `lanczos`...).
    pub method: String,
    /// The scaler's parameter (`param0`: bicubic's B, lanczos' width;
    /// 0: FFmpeg's default).
    pub param: f64,
    /// "Aspect ratio": the frame keeps its shape inside the size, or
    /// takes the size.
    pub keep_aspect: bool,
    /// What Smaller and Larger compare with: the display's resolution, or
    /// `condition_size` ("Match size").
    pub condition_display: bool,
    pub condition_size: [u32; 2],
    /// "Interlaced mode": the fields are scaled apart.
    pub interlaced: bool,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            when: ResizeWhen::Never,
            target: ResizeTarget::Display,
            factor: 2.0,
            width: 1920,
            height: 1080,
            auto_height: true,
            method: "bicubic".to_owned(),
            param: 0.0,
            keep_aspect: true,
            condition_display: true,
            condition_size: [1280, 720],
            interlaced: false,
        }
    }
}

/// FFmpeg's scalers for Video > Resize.
pub const RESIZE_METHODS: [&str; 9] = [
    "fast_bilinear",
    "bilinear",
    "bicubic",
    "neighbor",
    "area",
    "gauss",
    "sinc",
    "lanczos",
    "spline",
];

/// M12's picture settings (`picture` in `settings.json`).
// Independent options as PotPlayer's pages list them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Picture {
    /// Aspect Ratio > "Custom (%s)": `width:height` or a number.
    pub custom_aspect: String,
    pub output_device: OutputDevice,
    /// The video takes the window's shape while it is not fullscreen.
    pub adjust_to_window: bool,
    /// In fullscreen the aspect ratio mode applies only while the margin
    /// around the video is below `fullscreen_margin` percent.
    pub fullscreen_mode_limited: bool,
    pub fullscreen_margin: f64,
    pub extend: ExtendMethod,
    /// The ratio Extend/Crop extends or crops to.
    pub extend_ratio: String,
    /// The size "Extend/Crop by pixel" gives the frame.
    pub extend_size: [u32; 2],
    /// "Use video extension before resizing": Extend/Crop before Video >
    /// Resize (after it otherwise).
    pub extend_first: bool,
    /// "Scale only frame height": Extend/Crop by ratio scales the height
    /// to the ratio instead of adding bars or cutting.
    pub extend_scale_height: bool,
    /// Where the picture sits in an extended or cropped frame: the
    /// horizontal and vertical position in percent.
    pub position: [f64; 2],
    /// Video > Bottom Margin: black below the picture, in percent of its
    /// height (0, 5 to 30).
    pub bottom_margin: u32,
    pub bottom_margin_subtitles_only: bool,
    /// Aspect Ratio > "Provide Bottom Margin for Subtitles".
    pub subtitle_margin: bool,
    pub subtitle_room: SubtitleRoom,
    /// The room is made by shrinking the picture (otherwise the letterbox
    /// below the video is used).
    pub subtitle_margin_resize: bool,
    pub subtitle_margin_fullscreen_only: bool,
    /// The screen margins kept with "Remember screen margin changes".
    pub margins: Margins,
    pub margins_fullscreen_only: bool,
    pub remember_margins: bool,
    pub presets: Vec<FramePreset>,
    /// The next file keeps the frame's zoom, stretch and position.
    pub remember_frame: bool,
    /// The frame kept with `remember_frame`.
    pub frame: Frame,
    pub auto_rotate: AutoRotate,
    pub resize: Resize,
    /// The Aspect Ratio page's "AR Mode" and "Override AR": the mode and
    /// the ratio of the last run, which the next starts with.
    pub fit: crate::action::Fit,
    pub aspect: crate::action::Aspect,
}

/// Frame Size's presets as RVP ships them: zooms and stretches of 10, 20
/// and 30 %. PotPlayer's own values were not recorded (R1).
#[must_use]
pub fn default_presets() -> Vec<FramePreset> {
    let mut presets = vec![FramePreset::default()];
    for (kind, width, height) in [
        ("Zoom", 1.0, 1.0),
        ("Increasing width", 1.0, 0.0),
        ("Increasing Height", 0.0, 1.0),
    ] {
        for step in 1..=3 {
            let grow = f64::from(step) * 10.0;
            presets.push(FramePreset {
                name: format!("{kind} {step}"),
                width: 100.0 + grow * width,
                height: 100.0 + grow * height,
            });
        }
    }
    presets
}

impl Default for Picture {
    fn default() -> Self {
        Self {
            custom_aspect: "21:9".to_owned(),
            output_device: OutputDevice::Monitor,
            adjust_to_window: false,
            fullscreen_mode_limited: false,
            fullscreen_margin: 10.0,
            extend: ExtendMethod::None,
            extend_ratio: "4:3".to_owned(),
            extend_first: false,
            extend_scale_height: false,
            extend_size: [1920, 1080],
            position: [50.0, 50.0],
            bottom_margin: 0,
            bottom_margin_subtitles_only: false,
            subtitle_margin: false,
            subtitle_room: SubtitleRoom::OneLine,
            subtitle_margin_resize: true,
            subtitle_margin_fullscreen_only: false,
            margins: Margins::default(),
            margins_fullscreen_only: false,
            remember_margins: false,
            presets: default_presets(),
            remember_frame: false,
            frame: Frame::default(),
            auto_rotate: AutoRotate::Off,
            resize: Resize::default(),
            fit: crate::action::Fit::KeepAspect,
            aspect: crate::action::Aspect::Default,
        }
    }
}

/// The steps of the picture's commands.
pub const MARGIN_STEP: f64 = 1.0;
pub const POSITION_STEP: f64 = 5.0;
/// The bottom margins of Video > Bottom Margin, in percent.
pub const BOTTOM_MARGINS: [u32; 7] = [0, 5, 10, 15, 20, 25, 30];

/// A ratio written `16:9`, `2.35:1`, `16/9` or `1.85`, between 0.1 and 10.
#[must_use]
pub fn parse_ratio(text: &str) -> Option<f64> {
    let text = text.trim();
    let value = match text.split_once([':', '/', 'x']) {
        Some((width, height)) => {
            let width: f64 = width.trim().parse().ok()?;
            let height: f64 = height.trim().parse().ok()?;
            if height <= 0.0 {
                return None;
            }
            width / height
        }
        None => text.parse().ok()?,
    };
    (value.is_finite() && (0.1..=10.0).contains(&value)).then_some(value)
}

/// A number for FFmpeg's expressions, without needless digits.
fn number(value: f64) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The lavfi graph of Video > Extend/Crop: `None` when it does nothing.
/// `position` is in percent; `ratio` is the display aspect ratio, so the
/// expressions take the file's pixel aspect (`sar`) into account.
#[must_use]
pub fn crop_graph(
    method: ExtendMethod,
    ratio: f64,
    size: [u32; 2],
    position: [f64; 2],
    scale_height: bool,
) -> Option<String> {
    let x = number((position[0] / 100.0).clamp(0.0, 1.0));
    let y = number((position[1] / 100.0).clamp(0.0, 1.0));
    let r = number(ratio);
    match method {
        ExtendMethod::None => None,
        // "Scale only frame height": the height becomes the ratio's.
        ExtendMethod::Crop | ExtendMethod::Extend if scale_height => {
            Some(format!("scale=w=iw:h='round(iw*sar/{r}/2)*2'"))
        }
        // Even sizes (the 4:2:0 formats need them), rounded: the ratio's
        // digits must not cost a row.
        ExtendMethod::Crop => Some(format!(
            "crop=w='if(gt(dar,{r}),min(iw,round(ih*{r}/sar/2)*2),iw)':h='if(gt(dar,{r}),ih,min(ih,round(iw*sar/{r}/2)*2))':x='(iw-ow)*{x}':y='(ih-oh)*{y}'"
        )),
        ExtendMethod::Extend => Some(format!(
            "pad=w='if(lt(dar,{r}),max(iw,round(ih*{r}/sar/2)*2),iw)':h='if(lt(dar,{r}),ih,max(ih,round(iw*sar/{r}/2)*2))':x='(ow-iw)*{x}':y='(oh-ih)*{y}':color=black"
        )),
        ExtendMethod::Pixels => {
            let [width, height] = size;
            Some(format!(
                "crop=w='min(iw,{width})':h='min(ih,{height})':x='(iw-ow)*{x}':y='(ih-oh)*{y}',pad=w='max(iw,{width})':h='max(ih,{height})':x='(ow-iw)*{x}':y='(oh-ih)*{y}':color=black"
            ))
        }
    }
}

/// Video > Bottom Margin's black below the picture: `percent` of its
/// height.
#[must_use]
pub fn margin_graph(percent: u32) -> Option<String> {
    (percent > 0).then(|| {
        format!(
            "pad=w=iw:h='trunc(ih*{}/200)*2':x=0:y=0:color=black",
            100 + percent.min(100)
        )
    })
}

/// Video > Resize's graph; `screen` is the screen's size in pixels.
#[must_use]
pub fn resize_graph(resize: &Resize, screen: Option<(u32, u32)>) -> Option<String> {
    if resize.when == ResizeWhen::Never {
        return None;
    }
    // What Smaller and Larger compare with.
    let limit = || -> Option<(String, String)> {
        if resize.condition_display {
            let (width, height) = screen?;
            Some((number(f64::from(width)), number(f64::from(height))))
        } else {
            let [width, height] = resize.condition_size;
            Some((width.to_string(), height.to_string()))
        }
    };
    let (width, height) = match resize.target {
        ResizeTarget::Display => {
            let (width, height) = screen?;
            (number(f64::from(width)), number(f64::from(height)))
        }
        ResizeTarget::Multiply => {
            let factor = number(resize.factor);
            (format!("iw*{factor}"), format!("ih*{factor}"))
        }
        ResizeTarget::Size => (
            resize.width.to_string(),
            if resize.auto_height {
                format!("{}*ih/iw", resize.width)
            } else {
                resize.height.to_string()
            },
        ),
    };
    // The condition leaves frames that should stay as they are at their
    // own size.
    let condition = match resize.when {
        ResizeWhen::Never => return None,
        ResizeWhen::Always => "1".to_owned(),
        ResizeWhen::Smaller => {
            let (limit_width, limit_height) = limit()?;
            format!("lt(iw,{limit_width})*lt(ih,{limit_height})")
        }
        ResizeWhen::Larger => {
            let (limit_width, limit_height) = limit()?;
            format!("gt(iw,{limit_width})+gt(ih,{limit_height})")
        }
    };
    let method = if RESIZE_METHODS.contains(&resize.method.as_str()) {
        resize.method.as_str()
    } else {
        "bicubic"
    };
    let shape = if resize.keep_aspect {
        ":force_original_aspect_ratio=decrease:force_divisible_by=2"
    } else {
        ""
    };
    let param = if resize.param > 0.0 {
        format!(":param0={}", number(resize.param))
    } else {
        String::new()
    };
    let interlaced = if resize.interlaced { ":interl=1" } else { "" };
    Some(format!(
        "scale=w='if({condition},{width},iw)':h='if({condition},{height},ih)'{shape}:flags={method}{param}{interlaced}"
    ))
}

/// The share of the window's height `room` keeps under the video for
/// subtitle lines at mpv's `sub-font-size` 55 (of 720) times `sub_scale`,
/// with mpv's own bottom margin.
#[must_use]
pub fn subtitle_room_ratio(room: SubtitleRoom, sub_scale: f64) -> f64 {
    let lines = match room {
        SubtitleRoom::OneLine => 1.0,
        SubtitleRoom::TwoLines => 2.0,
        SubtitleRoom::ThreeLines => 3.0,
        SubtitleRoom::Available | SubtitleRoom::AvailableAlways => return 0.0,
    };
    ((lines * 55.0 * 1.25 + 22.0) / 720.0 * sub_scale.clamp(0.1, 10.0)).clamp(0.0, 0.45)
}

/// The rotation by aspect ratio for a video of display aspect `aspect`
/// turned by its metadata's `rotation`, on a screen of `screen` pixels.
#[must_use]
pub fn auto_rotation(
    mode: AutoRotate,
    aspect: f64,
    rotation: i64,
    screen: Option<(u32, u32)>,
) -> u16 {
    let (same, other) = match mode {
        AutoRotate::Off => return 0,
        AutoRotate::Quarter => (0, 90),
        AutoRotate::Half => (180, 270),
    };
    let Some((width, height)) = screen else {
        return same;
    };
    let turned = rotation.rem_euclid(180) == 90;
    let video_wide = (aspect >= 1.0) != turned;
    let screen_wide = width >= height;
    if video_wide == screen_wide {
        same
    } else {
        other
    }
}

/// mpv's `monitorpixelaspect` for the output device: the screen is
/// `pixels` (width, height) and measures `millimetres`.
#[must_use]
pub fn monitor_pixel_aspect(
    device: OutputDevice,
    pixels: Option<(u32, u32)>,
    millimetres: Option<(f64, f64)>,
) -> f64 {
    let Some((width, height)) = pixels.filter(|(width, height)| *width > 0 && *height > 0) else {
        return 1.0;
    };
    let resolution = f64::from(width) / f64::from(height);
    let display = match device {
        OutputDevice::Monitor => return 1.0,
        OutputDevice::Tv4x3 => 4.0 / 3.0,
        OutputDevice::Tv16x9 => 16.0 / 9.0,
        OutputDevice::Auto => match millimetres {
            Some((w, h)) if w > 0.0 && h > 0.0 => w / h,
            _ => return 1.0,
        },
    };
    (display / resolution).clamp(0.2, 5.0)
}

/// The share of an area of aspect `area` that a video of aspect `video`
/// leaves black when it fits inside it.
#[must_use]
pub fn keep_aspect_margin(video: f64, area: f64) -> f64 {
    if video <= 0.0 || area <= 0.0 {
        return 0.0;
    }
    if video > area {
        1.0 - area / video
    } else {
        1.0 - video / area
    }
}

/// The screen as the window sees it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Screen {
    /// Pixels.
    pub width: u32,
    pub height: u32,
    /// Its measured size in millimetres, when the system knows it.
    pub width_mm: f64,
    pub height_mm: f64,
    /// Its refresh rate in hertz (0: unknown): M18's target rate and
    /// Blend's rate with the OpenGL renderer.
    pub refresh: f64,
}

impl Screen {
    fn pixels(&self) -> Option<(u32, u32)> {
        (self.width > 0 && self.height > 0).then_some((self.width, self.height))
    }

    fn millimetres(&self) -> Option<(f64, f64)> {
        (self.width_mm > 0.0 && self.height_mm > 0.0).then_some((self.width_mm, self.height_mm))
    }
}

/// One of the picture's commands (PotPlayer's Aspect Ratio and Frame Size
/// menus, Video > Resize, Extend/Crop, Bottom Margin and Screen
/// Rotation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PictureCommand {
    /// "Set Custom Aspect Ratio...".
    CustomAspectDialog,
    /// "Cycle Aspect Ratio Mode".
    ModeCycle,
    OutputDevice(OutputDevice),
    ToggleAdjustToWindow,
    ToggleSubtitleMargin,
    SubtitleRoom(SubtitleRoom),
    /// Extend/Crop's presets: extending or cropping to a ratio.
    ExtendPreset(ExtendMethod, Aspect),
    /// Extend/Crop: "Do nothing".
    ExtendOff,
    /// "Extend by aspect ratio" and "Crop by aspect ratio": the ratio of
    /// the Extend/Crop page.
    ExtendByRatio(ExtendMethod),
    /// "Extend/Crop by pixel": asks for the size.
    ExtendPixelsDialog,
    /// Moves the picture inside an extended or cropped frame.
    Move(Side),
    /// "Reset Video Position".
    ResetPosition,
    BottomMarginCycle,
    BottomMargin(u32),
    ToggleBottomMarginSubtitlesOnly,
    AutoRotate(AutoRotate),
    Preset(usize),
    PresetCycle,
    ToggleRememberFrame,
    /// A screen margin one step larger (`true`) or smaller.
    ScreenMargin(Side, bool),
    ToggleMarginsFullscreenOnly,
    ToggleRememberMargins,
    Resize(ResizeWhen),
}

/// The text of a ratio preset of Extend/Crop and Aspect Ratio.
#[must_use]
pub fn preset_ratio(aspect: Aspect) -> Option<f64> {
    Some(match aspect {
        Aspect::R4x3 => 4.0 / 3.0,
        Aspect::R16x9 => 16.0 / 9.0,
        Aspect::R16x10 => 16.0 / 10.0,
        Aspect::R185x1 => 1.85,
        Aspect::R235x1 => 2.35,
        Aspect::Default | Aspect::Original | Aspect::Custom => return None,
    })
}

impl Picture {
    /// Brings the values into their ranges, with a note per repair.
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        for (value, default, name) in [
            (&mut self.custom_aspect, "21:9", "picture.custom-aspect"),
            (&mut self.extend_ratio, "4:3", "picture.extend-ratio"),
        ] {
            if parse_ratio(value).is_none() {
                notes.push(format!("{name} {value:?} is not a ratio: {default} used"));
                default.clone_into(value);
            }
        }
        for size in &mut self.extend_size {
            if !(16..=16384).contains(size) {
                notes.push(format!(
                    "picture.extend-size must be 16 to 16384 pixels: {} used",
                    (*size).clamp(16, 16384)
                ));
                *size = (*size).clamp(16, 16384);
            }
        }
        for value in &mut self.position {
            crate::playback::clamp_number(value, 0.0, 100.0, "picture.position", notes);
        }
        if !BOTTOM_MARGINS.contains(&self.bottom_margin) {
            let nearest = BOTTOM_MARGINS
                .iter()
                .copied()
                .min_by_key(|margin| margin.abs_diff(self.bottom_margin))
                .unwrap_or(0);
            notes.push(format!(
                "picture.bottom-margin must be one of 0, 5, 10, ..., 30: {nearest} used"
            ));
            self.bottom_margin = nearest;
        }
        crate::playback::clamp_number(
            &mut self.fullscreen_margin,
            0.0,
            50.0,
            "picture.fullscreen-margin",
            notes,
        );
        let margins = &mut self.margins;
        for value in [
            &mut margins.top,
            &mut margins.left,
            &mut margins.bottom,
            &mut margins.right,
        ] {
            crate::playback::clamp_number(value, 0.0, 40.0, "picture.margins", notes);
        }
        if self.presets.is_empty() || self.presets.len() > 20 {
            notes.push("picture.presets must list 1 to 20 presets: RVP's used".to_owned());
            self.presets = default_presets();
        }
        for preset in &mut self.presets {
            crate::playback::clamp_number(&mut preset.width, 10.0, 400.0, "picture.presets", notes);
            crate::playback::clamp_number(
                &mut preset.height,
                10.0,
                400.0,
                "picture.presets",
                notes,
            );
            if preset.name.trim().is_empty() || preset.name.chars().any(char::is_control) {
                notes.push("picture.presets: a name must be one line of text".to_owned());
                preset.name = format!("{:.0} x {:.0} %", preset.width, preset.height);
            }
        }
        let frame = &mut self.frame;
        crate::playback::clamp_number(&mut frame.zoom, -3.0, 3.0, "picture.frame.zoom", notes);
        crate::playback::clamp_number(&mut frame.scale_x, 0.25, 4.0, "picture.frame", notes);
        crate::playback::clamp_number(&mut frame.scale_y, 0.25, 4.0, "picture.frame", notes);
        crate::playback::clamp_number(&mut frame.pan_x, -3.0, 3.0, "picture.frame", notes);
        crate::playback::clamp_number(&mut frame.pan_y, -3.0, 3.0, "picture.frame", notes);
        let resize = &mut self.resize;
        crate::playback::clamp_number(&mut resize.factor, 0.1, 8.0, "picture.resize.factor", notes);
        for value in [&mut resize.width, &mut resize.height] {
            if !(16..=16384).contains(value) {
                notes.push("picture.resize: sizes must be 16 to 16384 pixels".to_owned());
                *value = (*value).clamp(16, 16384);
            }
        }
        if !RESIZE_METHODS.contains(&resize.method.as_str()) {
            notes.push(format!(
                "picture.resize.method {:?} is not one of {}: bicubic used",
                resize.method,
                RESIZE_METHODS.join(", ")
            ));
            "bicubic".clone_into(&mut resize.method);
        }
    }
}

/// The picture's state for this run (the rest is in [`Picture`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PictureState {
    /// The window is fullscreen, and the aspect ratio mode its kind of
    /// fullscreen brings.
    pub fullscreen: bool,
    pub fullscreen_fit: Option<Fit>,
    pub screen: Screen,
    /// The screen margins now (the settings' when remembered).
    pub margins: Margins,
    /// The preset applied last.
    pub preset: Option<usize>,
    /// The rotation by aspect ratio now, in degrees.
    pub auto_rotation: u16,
}

/// The aspect ratio mode's name in notices.
#[must_use]
pub fn fit_text(fit: Fit) -> &'static str {
    match fit {
        Fit::KeepAspect => "Keep aspect ratio",
        Fit::Crop => "Stretch by keeping aspect ratio",
        Fit::Stretch => "Stretch",
        Fit::FrameHalf => "Video frame at 0.5x",
        Fit::FrameOne => "Video frame at 1.0x",
        Fit::FrameOneAndHalf => "Video frame at 1.5x",
        Fit::FrameDouble => "Video frame at 2.0x",
    }
}

/// The factor a "Keep Video Frame" mode keeps the frame at.
#[must_use]
pub fn frame_factor(fit: Fit) -> Option<f64> {
    match fit {
        Fit::FrameHalf => Some(0.5),
        Fit::FrameOne => Some(1.0),
        Fit::FrameOneAndHalf => Some(1.5),
        Fit::FrameDouble => Some(2.0),
        Fit::KeepAspect | Fit::Crop | Fit::Stretch => None,
    }
}

const FIT_CYCLE: [Fit; 7] = [
    Fit::KeepAspect,
    Fit::Crop,
    Fit::Stretch,
    Fit::FrameHalf,
    Fit::FrameOne,
    Fit::FrameOneAndHalf,
    Fit::FrameDouble,
];

fn on_off(name: &str, on: bool) -> UiRequest {
    UiRequest::Osd(format!("{name}: {}", if on { "on" } else { "off" }))
}

impl App {
    /// The aspect ratio's override and method for mpv.
    fn aspect_values(&self) -> (String, &'static str) {
        match self.session.aspect {
            Aspect::Default => ("-1".to_owned(), "container"),
            Aspect::Original => ("-1".to_owned(), "bitstream"),
            Aspect::Custom => (
                number(parse_ratio(&self.settings.picture.custom_aspect).unwrap_or(21.0 / 9.0)),
                "container",
            ),
            other => (
                number(preset_ratio(other).unwrap_or(16.0 / 9.0)),
                "container",
            ),
        }
    }

    /// The aspect ratio mode in force: fullscreen's own, else the chosen
    /// one (in fullscreen only while the margin is small, when limited).
    #[must_use]
    pub fn effective_fit(&self) -> Fit {
        let picture = &self.settings.picture;
        let state = &self.picture;
        if state.fullscreen {
            if let Some(fit) = state.fullscreen_fit {
                return fit;
            }
            if picture.fullscreen_mode_limited
                && matches!(self.session.fit, Fit::Crop | Fit::Stretch)
                && let (Some(video), Some((width, height))) = (
                    self.model.double(PropertyId::VideoAspect),
                    state.screen.pixels(),
                )
            {
                let margin = keep_aspect_margin(video, f64::from(width) / f64::from(height));
                if margin * 100.0 >= picture.fullscreen_margin {
                    return Fit::KeepAspect;
                }
            }
        }
        self.session.fit
    }

    /// Sets the aspect ratio, its mode, the frame, the output device's
    /// pixel aspect, the rotation and the window's margins on mpv.
    ///
    /// # Errors
    /// Fails when libmpv rejects a value.
    pub fn apply_picture(&self, engine: &Engine) -> Result<(), EngineError> {
        let session = &self.session;
        let picture = &self.settings.picture;
        let (aspect, method) = self.aspect_values();
        engine.set_text("video-aspect-override", &aspect)?;
        engine.set_text("video-aspect-method", method)?;
        let fit = self.effective_fit();
        let stretched =
            fit == Fit::Stretch || (picture.adjust_to_window && !self.picture.fullscreen);
        engine.set_flag("keepaspect", !stretched)?;
        let panscan = if fit == Fit::Crop { 1.0 } else { 0.0 };
        engine.set_double("panscan", panscan)?;
        let frame = frame_factor(fit);
        engine.set_text("video-unscaled", if frame.is_some() { "yes" } else { "no" })?;
        let zoom = session.zoom + frame.map_or(0.0, f64::log2);
        engine.set_double("video-zoom", zoom)?;
        // Older libmpv builds (Ubuntu 24.04's 0.37) have no video-scale-x/y.
        drop(engine.set_double("video-scale-x", session.scale_x));
        drop(engine.set_double("video-scale-y", session.scale_y));
        engine.set_double("video-pan-x", session.pan_x)?;
        engine.set_double("video-pan-y", session.pan_y)?;
        let state = &self.picture;
        engine.set_double(
            "monitorpixelaspect",
            monitor_pixel_aspect(
                picture.output_device,
                state.screen.pixels(),
                state.screen.millimetres(),
            ),
        )?;
        self.apply_margins(engine)?;
        self.apply_rotation(engine);
        Ok(())
    }

    /// True while a subtitle track is selected.
    fn has_subtitles(&self) -> bool {
        self.model.int(PropertyId::Sid).is_some()
    }

    /// The window's margins: the screen margins and the subtitles' room.
    fn apply_margins(&self, engine: &Engine) -> Result<(), EngineError> {
        let picture = &self.settings.picture;
        let state = &self.picture;
        let screen = if picture.margins_fullscreen_only && !state.fullscreen {
            Margins::default()
        } else {
            state.margins
        };
        let mut bottom = screen.bottom / 100.0;
        let mut align_top = false;
        let room_active = picture.subtitle_margin
            && (!picture.subtitle_margin_fullscreen_only || state.fullscreen)
            && (self.has_subtitles() || picture.subtitle_room == SubtitleRoom::AvailableAlways);
        if room_active {
            match picture.subtitle_room {
                SubtitleRoom::Available | SubtitleRoom::AvailableAlways => align_top = true,
                room if picture.subtitle_margin_resize => {
                    let scale = self.model.double(PropertyId::SubScale).unwrap_or(1.0);
                    bottom += subtitle_room_ratio(room, scale);
                }
                _ => align_top = true,
            }
        }
        engine.set_double("video-margin-ratio-top", screen.top / 100.0)?;
        engine.set_double("video-margin-ratio-left", screen.left / 100.0)?;
        engine.set_double("video-margin-ratio-right", screen.right / 100.0)?;
        engine.set_double("video-margin-ratio-bottom", bottom.min(0.9))?;
        engine.set_double("video-align-y", if align_top { -1.0 } else { 0.0 })?;
        if room_active {
            // Text subtitles go into the room under the video.
            drop(engine.set_flag("sub-use-margins", true));
        }
        Ok(())
    }

    /// `video-rotate`: the screen rotation plus the rotation by aspect
    /// ratio; "no" keeps the metadata's out when that is off.
    pub(crate) fn apply_rotation(&self, engine: &Engine) {
        let total = (self.session.rotation + self.picture.auto_rotation) % 360;
        let value = if total == 0 && !self.settings.playback.auto_rotate {
            "no".to_owned()
        } else {
            total.to_string()
        };
        // An integer choice: mpv takes it as text, not as a double.
        drop(engine.set_text("video-rotate", &value));
    }

    /// The picture's filters (resize, extend/crop, bottom margin), as
    /// `(label, vf entry)` in the order they run.
    #[must_use]
    pub fn picture_filters(&self) -> Vec<(String, String)> {
        let picture = &self.settings.picture;
        let mut filters = Vec::new();
        if let Some(graph) = resize_graph(&picture.resize, self.picture.screen.pixels()) {
            filters.push((RESIZE_LABEL, graph));
        }
        let ratio = parse_ratio(&picture.extend_ratio).unwrap_or(4.0 / 3.0);
        if let Some(graph) = crop_graph(
            picture.extend,
            ratio,
            picture.extend_size,
            picture.position,
            picture.extend_scale_height,
        ) {
            // "Use video extension before resizing".
            let at = if picture.extend_first {
                0
            } else {
                filters.len()
            };
            filters.insert(at, (CROP_LABEL, graph));
        }
        let margin_wanted = !picture.bottom_margin_subtitles_only || self.has_subtitles();
        if margin_wanted && let Some(graph) = margin_graph(picture.bottom_margin) {
            filters.push((MARGIN_LABEL, graph));
        }
        filters
            .into_iter()
            .map(|(label, graph)| (label.to_owned(), format!("@{label}:lavfi=[{graph}]")))
            .collect()
    }

    /// The window became fullscreen (with the aspect ratio mode its kind
    /// brings) or left it.
    pub fn set_fullscreen(&mut self, engine: &Engine, fullscreen: bool, fit: Option<Fit>) {
        let state = &mut self.picture;
        state.fullscreen = fullscreen;
        state.fullscreen_fit = if fullscreen { fit } else { None };
        drop(self.apply_picture(engine));
    }

    /// The screen the window is on (its pixels and measured size).
    pub fn set_screen(&mut self, engine: &Engine, screen: Screen) {
        if self.picture.screen == screen {
            return;
        }
        let resize_changes = self.settings.picture.resize.target == ResizeTarget::Display
            && self.settings.picture.resize.when != ResizeWhen::Never;
        if (screen.refresh - self.picture.screen.refresh).abs() >= 0.001
            || (screen.width, screen.height)
                != (self.picture.screen.width, self.picture.screen.height)
        {
            crate::log::write(
                crate::log::Level::Info,
                &format!(
                    "screen: {}x{}, {:.3} Hz",
                    screen.width, screen.height, screen.refresh
                ),
            );
        }
        self.picture.screen = screen;
        self.update_auto_rotation();
        drop(self.apply_picture(engine));
        if resize_changes || self.processor_changed() {
            drop(self.apply_video_filters(engine));
        }
        // The display's rate is smoothing's target.
        self.apply_smoothing(engine);
    }

    /// The fullscreen state and the kind of fullscreen, for the status.
    #[must_use]
    pub fn fullscreen_fit(&self) -> Option<Fit> {
        self.picture.fullscreen_fit
    }

    /// Works out the rotation by aspect ratio for the video that plays.
    fn update_auto_rotation(&mut self) -> bool {
        let aspect = self
            .model
            .double(PropertyId::VideoAspect)
            .unwrap_or(16.0 / 9.0);
        let rotation = self.model.int(PropertyId::VideoRotation).unwrap_or(0);
        // Pictures turn the same way with the General page's option (M14).
        let mode = match self.settings.picture.auto_rotate {
            AutoRotate::Off if self.turns_pictures() => AutoRotate::Quarter,
            mode => mode,
        };
        let angle = auto_rotation(mode, aspect, rotation, self.picture.screen.pixels());
        let changed = angle != self.picture.auto_rotation;
        self.picture.auto_rotation = angle;
        changed
    }

    /// A new file loaded: the frame starts afresh unless remembered.
    pub(crate) fn picture_file_loaded(&mut self, engine: &Engine) {
        let picture = &self.settings.picture;
        let frame = if picture.remember_frame {
            picture.frame
        } else {
            Frame::default()
        };
        let session = &mut self.session;
        session.zoom = frame.zoom;
        session.scale_x = frame.scale_x;
        session.scale_y = frame.scale_y;
        session.pan_x = frame.pan_x;
        session.pan_y = frame.pan_y;
        if !picture.remember_frame {
            self.picture.preset = None;
        }
        self.update_auto_rotation();
        drop(self.apply_picture(engine));
    }

    /// mpv's properties changed: the rotation by aspect ratio follows the
    /// video, and the parts that wait for subtitles follow the track.
    pub(crate) fn picture_changed(&mut self, engine: &Engine, changes: rvp_engine::Changes) {
        if changes.any(&[PropertyId::VideoAspect, PropertyId::VideoRotation])
            && self.update_auto_rotation()
        {
            self.apply_rotation(engine);
        }
        // Super resolution's scale follows the video's height.
        if changes.contains(PropertyId::VideoHeight) && self.processor_changed() {
            drop(self.apply_video_filters(engine));
        }
        if changes.contains(PropertyId::Sid) {
            let picture = &self.settings.picture;
            if picture.subtitle_margin && picture.subtitle_room != SubtitleRoom::AvailableAlways {
                drop(self.apply_margins(engine));
            }
            if picture.bottom_margin > 0 && picture.bottom_margin_subtitles_only {
                drop(self.apply_video_filters(engine));
            }
        }
    }

    /// A drag with modifiers on the video (PotPlayer's Mouse page, M14):
    /// `dx` and `dy` are fractions of the window's size since the last
    /// move. Alt pans, Ctrl zooms (up is larger), Shift stretches (right
    /// is wider, up is taller), Ctrl+Alt moves the subtitles.
    pub fn drag_adjust(
        &mut self,
        engine: &Engine,
        kind: crate::pointer::DragKind,
        dx: f64,
        dy: f64,
    ) -> Option<UiRequest> {
        use crate::pointer::DragKind;
        if !self.model.media_loaded() {
            return None;
        }
        let session = &mut self.session;
        let text = match kind {
            DragKind::Pan => {
                session.pan_x = (session.pan_x + dx).clamp(-3.0, 3.0);
                session.pan_y = (session.pan_y + dy).clamp(-3.0, 3.0);
                format!("Position: {:+.2}, {:+.2}", session.pan_x, session.pan_y)
            }
            DragKind::Zoom => {
                session.zoom = (session.zoom - 2.0 * dy).clamp(-3.0, 3.0);
                format!("Zoom: {:.0} %", 2f64.powf(session.zoom) * 100.0)
            }
            DragKind::Stretch => {
                session.scale_x = (session.scale_x * 2f64.powf(2.0 * dx)).clamp(0.1, 10.0);
                session.scale_y = (session.scale_y * 2f64.powf(-2.0 * dy)).clamp(0.1, 10.0);
                format!(
                    "Stretch: {:.0} % x {:.0} %",
                    session.scale_x * 100.0,
                    session.scale_y * 100.0
                )
            }
            DragKind::Subtitles => return Some(self.drag_subtitles(engine, dx, dy)),
        };
        drop(self.apply_picture(engine));
        self.remember_frame();
        Some(UiRequest::Osd(text))
    }

    /// Keeps the frame for the next file when it is remembered.
    pub(crate) fn remember_frame(&mut self) {
        if !self.settings.picture.remember_frame {
            return;
        }
        let session = &self.session;
        self.settings.picture.frame = Frame {
            zoom: session.zoom,
            scale_x: session.scale_x,
            scale_y: session.scale_y,
            pan_x: session.pan_x,
            pan_y: session.pan_y,
        };
        self.save_settings();
    }

    /// The picture's commands.
    pub(crate) fn picture_command(
        &mut self,
        engine: &Engine,
        command: PictureCommand,
    ) -> Option<UiRequest> {
        match command {
            PictureCommand::CustomAspectDialog => {
                return Some(UiRequest::PictureDialog {
                    kind: "custom-aspect",
                    value: self.settings.picture.custom_aspect.clone(),
                });
            }
            PictureCommand::ExtendPixelsDialog => {
                let [width, height] = self.settings.picture.extend_size;
                return Some(UiRequest::PictureDialog {
                    kind: "extend-size",
                    value: format!("{width}x{height}"),
                });
            }
            PictureCommand::ModeCycle => {
                let index = FIT_CYCLE
                    .iter()
                    .position(|fit| *fit == self.session.fit)
                    .map_or(0, |index| (index + 1) % FIT_CYCLE.len());
                self.session.fit = FIT_CYCLE[index];
                self.remember_aspect();
                drop(self.apply_picture(engine));
                return Some(UiRequest::Osd(fit_text(self.session.fit).to_owned()));
            }
            PictureCommand::Preset(index) => return self.apply_preset(engine, index),
            PictureCommand::PresetCycle => {
                let count = self.settings.picture.presets.len();
                let next = self.picture.preset.map_or(1, |index| index + 1) % count.max(1);
                return self.apply_preset(engine, next);
            }
            PictureCommand::ScreenMargin(side, larger) => {
                let state = &mut self.picture;
                let value = state.margins.get_mut(side);
                *value =
                    (*value + if larger { MARGIN_STEP } else { -MARGIN_STEP }).clamp(0.0, 40.0);
                let margins = state.margins;
                if self.settings.picture.remember_margins {
                    self.settings.picture.margins = margins;
                    self.save_settings();
                }
                drop(self.apply_margins(engine));
                return Some(UiRequest::Osd(format!(
                    "Screen margins: top {:.0} %, left {:.0} %, bottom {:.0} %, right {:.0} %",
                    margins.top, margins.left, margins.bottom, margins.right
                )));
            }
            PictureCommand::Move(side) => return Some(self.move_picture(engine, Some(side))),
            PictureCommand::ResetPosition => return Some(self.move_picture(engine, None)),
            _ => {}
        }
        let notice = self.picture_setting(command)?;
        self.save_settings();
        drop(self.apply_picture(engine));
        drop(self.apply_video_filters(engine));
        self.apply_decoding(engine);
        Some(notice)
    }

    /// The commands that change a setting: the change and its notice.
    fn picture_setting(&mut self, command: PictureCommand) -> Option<UiRequest> {
        let picture = &mut self.settings.picture;
        Some(match command {
            PictureCommand::OutputDevice(device) => {
                picture.output_device = device;
                UiRequest::Osd(format!(
                    "Output device: {}",
                    match device {
                        OutputDevice::Monitor => "monitor",
                        OutputDevice::Tv4x3 => "TV 4:3",
                        OutputDevice::Tv16x9 => "TV 16:9",
                        OutputDevice::Auto => "detected",
                    }
                ))
            }
            PictureCommand::ToggleAdjustToWindow => {
                picture.adjust_to_window = !picture.adjust_to_window;
                on_off(
                    "Adjust aspect ratio to the window",
                    picture.adjust_to_window,
                )
            }
            PictureCommand::ToggleSubtitleMargin => {
                picture.subtitle_margin = !picture.subtitle_margin;
                on_off("Bottom margin for subtitles", picture.subtitle_margin)
            }
            PictureCommand::SubtitleRoom(room) => {
                picture.subtitle_room = room;
                picture.subtitle_margin = true;
                UiRequest::Osd(format!("Bottom margin for subtitles: {}", room_text(room)))
            }
            PictureCommand::Resize(when) => {
                picture.resize.when = when;
                UiRequest::Osd(format!(
                    "Resize: {}",
                    match when {
                        ResizeWhen::Never => "off",
                        ResizeWhen::Smaller => "when smaller",
                        ResizeWhen::Larger => "when larger",
                        ResizeWhen::Always => "always",
                    }
                ))
            }
            other => return self.picture_frame_setting(other),
        })
    }

    /// The rest of [`App::picture_setting`]: Extend/Crop, Video > Bottom
    /// Margin, the rotation by aspect ratio, Frame Size's options and
    /// Video > Resize.
    fn picture_frame_setting(&mut self, command: PictureCommand) -> Option<UiRequest> {
        let picture = &mut self.settings.picture;
        Some(match command {
            PictureCommand::ExtendPreset(method, aspect) => {
                picture.extend = method;
                crate::commands::aspect_text(aspect).clone_into(&mut picture.extend_ratio);
                UiRequest::Osd(format!(
                    "{} to {}",
                    extend_text(method),
                    crate::commands::aspect_text(aspect)
                ))
            }
            PictureCommand::ExtendOff => {
                picture.extend = ExtendMethod::None;
                UiRequest::Osd("Extend/Crop: off".to_owned())
            }
            PictureCommand::ExtendByRatio(method) => {
                picture.extend = method;
                UiRequest::Osd(format!(
                    "{} to {}",
                    extend_text(method),
                    picture.extend_ratio
                ))
            }
            PictureCommand::BottomMarginCycle => {
                let index = BOTTOM_MARGINS
                    .iter()
                    .position(|margin| *margin == picture.bottom_margin)
                    .map_or(0, |index| (index + 1) % BOTTOM_MARGINS.len());
                picture.bottom_margin = BOTTOM_MARGINS[index];
                margin_notice(picture.bottom_margin)
            }
            PictureCommand::BottomMargin(percent) => {
                picture.bottom_margin = percent;
                margin_notice(percent)
            }
            PictureCommand::ToggleBottomMarginSubtitlesOnly => {
                picture.bottom_margin_subtitles_only = !picture.bottom_margin_subtitles_only;
                on_off(
                    "Bottom margin only with subtitles",
                    picture.bottom_margin_subtitles_only,
                )
            }
            PictureCommand::AutoRotate(mode) => {
                picture.auto_rotate = if picture.auto_rotate == mode {
                    AutoRotate::Off
                } else {
                    mode
                };
                let mode = picture.auto_rotate;
                self.update_auto_rotation();
                UiRequest::Osd(match mode {
                    AutoRotate::Off => "Rotation by aspect ratio: off".to_owned(),
                    AutoRotate::Quarter => "Rotation by aspect ratio: 0/90°".to_owned(),
                    AutoRotate::Half => "Rotation by aspect ratio: 180/270°".to_owned(),
                })
            }
            PictureCommand::ToggleRememberFrame => {
                picture.remember_frame = !picture.remember_frame;
                let on = picture.remember_frame;
                self.remember_frame();
                on_off("Remember video size/position changes", on)
            }
            PictureCommand::ToggleMarginsFullscreenOnly => {
                picture.margins_fullscreen_only = !picture.margins_fullscreen_only;
                on_off(
                    "Margin changes only in fullscreen",
                    picture.margins_fullscreen_only,
                )
            }
            PictureCommand::ToggleRememberMargins => {
                picture.remember_margins = !picture.remember_margins;
                if picture.remember_margins {
                    picture.margins = self.picture.margins;
                }
                on_off("Remember screen margin changes", picture.remember_margins)
            }
            PictureCommand::CustomAspectDialog
            | PictureCommand::ExtendPixelsDialog
            | PictureCommand::ModeCycle
            | PictureCommand::Preset(_)
            | PictureCommand::PresetCycle
            | PictureCommand::ScreenMargin(..)
            | PictureCommand::Move(_)
            | PictureCommand::ResetPosition
            | PictureCommand::OutputDevice(_)
            | PictureCommand::ToggleAdjustToWindow
            | PictureCommand::ToggleSubtitleMargin
            | PictureCommand::SubtitleRoom(_)
            | PictureCommand::Resize(_) => return None,
        })
    }

    fn apply_preset(&mut self, engine: &Engine, index: usize) -> Option<UiRequest> {
        let preset = self.settings.picture.presets.get(index)?.clone();
        let session = &mut self.session;
        session.zoom = 0.0;
        session.scale_x = preset.width / 100.0;
        session.scale_y = preset.height / 100.0;
        self.picture.preset = Some(index);
        self.remember_frame();
        drop(self.apply_picture(engine));
        Some(UiRequest::Osd(format!("Frame size: {}", preset.name)))
    }

    /// Moves the picture in an extended or cropped frame (`None`: back to
    /// the middle).
    fn move_picture(&mut self, engine: &Engine, side: Option<Side>) -> UiRequest {
        let position = &mut self.settings.picture.position;
        match side {
            None => *position = [50.0, 50.0],
            Some(Side::Left) => position[0] -= POSITION_STEP,
            Some(Side::Right) => position[0] += POSITION_STEP,
            Some(Side::Top) => position[1] -= POSITION_STEP,
            Some(Side::Bottom) => position[1] += POSITION_STEP,
        }
        for value in position.iter_mut() {
            *value = value.clamp(0.0, 100.0);
        }
        let [x, y] = *position;
        self.save_settings();
        drop(self.apply_video_filters(engine));
        UiRequest::Osd(format!("Video position: {x:.0} %, {y:.0} %"))
    }

    /// The answer of a picture dialog (`custom-aspect`: a ratio,
    /// `extend-size`: `WIDTHxHEIGHT`).
    pub fn picture_dialog_answer(
        &mut self,
        engine: &Engine,
        kind: &str,
        text: &str,
    ) -> Option<UiRequest> {
        match kind {
            "custom-aspect" => {
                if parse_ratio(text).is_none() {
                    return Some(UiRequest::Message(format!(
                        "{text:?} is not an aspect ratio (16:9, 2.35:1 or 1.85)"
                    )));
                }
                text.trim()
                    .clone_into(&mut self.settings.picture.custom_aspect);
                self.session.aspect = Aspect::Custom;
                self.save_settings();
                self.remember_aspect();
                drop(self.apply_picture(engine));
                Some(UiRequest::Osd(format!("Aspect ratio: {}", text.trim())))
            }
            "extend-size" => {
                let size = text
                    .split_once(['x', 'X', '*', ' '])
                    .and_then(|(width, height)| {
                        Some([width.trim().parse().ok()?, height.trim().parse().ok()?])
                    })
                    .filter(|[width, height]: &[u32; 2]| {
                        (16..=16384).contains(width) && (16..=16384).contains(height)
                    });
                let Some(size) = size else {
                    return Some(UiRequest::Message(format!(
                        "{text:?} is not a size (1920x1080)"
                    )));
                };
                let picture = &mut self.settings.picture;
                picture.extend_size = size;
                picture.extend = ExtendMethod::Pixels;
                self.save_settings();
                self.apply_decoding(engine);
                drop(self.apply_video_filters(engine));
                Some(UiRequest::Osd(format!(
                    "Extend/Crop to {}x{}",
                    size[0], size[1]
                )))
            }
            _ => None,
        }
    }

    /// Whether a picture command's menu entry is checked.
    #[must_use]
    pub fn picture_checked(&self, command: PictureCommand) -> Option<bool> {
        let picture = &self.settings.picture;
        Some(match command {
            PictureCommand::OutputDevice(device) => picture.output_device == device,
            PictureCommand::ToggleAdjustToWindow => picture.adjust_to_window,
            PictureCommand::ToggleSubtitleMargin => picture.subtitle_margin,
            PictureCommand::SubtitleRoom(room) => picture.subtitle_room == room,
            PictureCommand::ExtendPreset(method, aspect) => {
                picture.extend == method
                    && parse_ratio(&picture.extend_ratio)
                        .zip(preset_ratio(aspect))
                        .is_some_and(|(a, b)| (a - b).abs() < 0.005)
            }
            PictureCommand::ExtendOff => picture.extend == ExtendMethod::None,
            PictureCommand::ExtendByRatio(method) => picture.extend == method,
            PictureCommand::ExtendPixelsDialog => picture.extend == ExtendMethod::Pixels,
            PictureCommand::BottomMargin(percent) => picture.bottom_margin == percent,
            PictureCommand::ToggleBottomMarginSubtitlesOnly => picture.bottom_margin_subtitles_only,
            PictureCommand::AutoRotate(mode) => picture.auto_rotate == mode,
            PictureCommand::Preset(index) => self.picture.preset == Some(index),
            PictureCommand::ToggleRememberFrame => picture.remember_frame,
            PictureCommand::ToggleMarginsFullscreenOnly => picture.margins_fullscreen_only,
            PictureCommand::ToggleRememberMargins => picture.remember_margins,
            PictureCommand::Resize(when) => picture.resize.when == when,
            PictureCommand::CustomAspectDialog
            | PictureCommand::ModeCycle
            | PictureCommand::Move(_)
            | PictureCommand::ResetPosition
            | PictureCommand::BottomMarginCycle
            | PictureCommand::PresetCycle
            | PictureCommand::ScreenMargin(..) => return None,
        })
    }

    /// A picture command's menu text: PotPlayer's, with the custom ratio,
    /// the Extend/Crop ratio or the preset's name filled in.
    #[must_use]
    pub fn picture_label(&self, command: PictureCommand, text: &str) -> String {
        let picture = &self.settings.picture;
        match command {
            PictureCommand::ExtendByRatio(method) => {
                format!(
                    "{} by aspect ratio ({})",
                    extend_text(method),
                    picture.extend_ratio
                )
            }
            PictureCommand::Preset(index) => picture
                .presets
                .get(index)
                .map_or_else(|| text.to_owned(), |preset| preset.name.clone()),
            _ => text.to_owned(),
        }
    }

    /// Starts this run's picture state from the settings.
    pub(crate) fn start_picture(&mut self) {
        if self.settings.picture.remember_margins {
            self.picture.margins = self.settings.picture.margins;
        }
        self.session.fit = self.settings.picture.fit;
        self.session.aspect = self.settings.picture.aspect;
    }

    /// Keeps the aspect ratio mode and ratio for the next run.
    pub(crate) fn remember_aspect(&mut self) {
        let picture = &mut self.settings.picture;
        if picture.fit != self.session.fit || picture.aspect != self.session.aspect {
            picture.fit = self.session.fit;
            picture.aspect = self.session.aspect;
            self.save_settings();
        }
    }

    /// The parts of [`App::apply_settings`] for the picture.
    pub(crate) fn apply_picture_settings_change(&mut self, engine: &Engine, old: &crate::Settings) {
        let (new, old) = (&self.settings.picture, &old.picture);
        if new == old {
            return;
        }
        // Margins typed on the settings page apply at once.
        if new.margins != old.margins {
            self.picture.margins = new.margins;
        }
        let filters = new.resize != old.resize
            || new.extend != old.extend
            || new.extend_ratio != old.extend_ratio
            || new.extend_size != old.extend_size
            || differs(&new.position, &old.position)
            || new.bottom_margin != old.bottom_margin
            || new.bottom_margin_subtitles_only != old.bottom_margin_subtitles_only;
        self.update_auto_rotation();
        drop(self.apply_picture(engine));
        if filters {
            self.apply_decoding(engine);
            drop(self.apply_video_filters(engine));
        }
    }

    /// The screen margins now (the self-test reads them).
    #[must_use]
    pub fn screen_margins(&self) -> Margins {
        self.picture.margins
    }

    /// The rotation by aspect ratio now, in degrees.
    #[must_use]
    pub fn auto_rotation_now(&self) -> u16 {
        self.picture.auto_rotation
    }

    /// The custom aspect ratio's text for "Custom (%s)".
    #[must_use]
    pub fn custom_aspect(&self) -> &str {
        &self.settings.picture.custom_aspect
    }

    /// The status' description of the picture's margins, for the self-test.
    #[must_use]
    pub fn margins_text(&self) -> String {
        let margins = self.picture.margins;
        let mut text = String::new();
        if !margins.is_zero() {
            let _ = write!(
                text,
                "{:.0}/{:.0}/{:.0}/{:.0}",
                margins.top, margins.left, margins.bottom, margins.right
            );
        }
        text
    }
}

/// True when two lists of numbers are not the same.
fn differs(a: &[f64], b: &[f64]) -> bool {
    a.len() != b.len() || a.iter().zip(b).any(|(a, b)| (a - b).abs() > 1e-9)
}

fn room_text(room: SubtitleRoom) -> &'static str {
    match room {
        SubtitleRoom::OneLine => "1 line",
        SubtitleRoom::TwoLines => "2 lines",
        SubtitleRoom::ThreeLines => "3 lines",
        SubtitleRoom::Available => "the space below the video",
        SubtitleRoom::AvailableAlways => "the space below the video, always",
    }
}

fn extend_text(method: ExtendMethod) -> &'static str {
    match method {
        ExtendMethod::None => "Do nothing",
        ExtendMethod::Extend => "Extend",
        ExtendMethod::Crop => "Crop",
        ExtendMethod::Pixels => "Extend/Crop",
    }
}

fn margin_notice(percent: u32) -> UiRequest {
    UiRequest::Osd(if percent == 0 {
        "Bottom margin: off".to_owned()
    } else {
        format!("Bottom margin: {percent} %")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_parse_in_every_form() {
        assert!((parse_ratio("16:9").unwrap_or_default() - 16.0 / 9.0).abs() < 1e-9);
        assert!((parse_ratio("2.35:1").unwrap_or_default() - 2.35).abs() < 1e-9);
        assert!((parse_ratio(" 21/9 ").unwrap_or_default() - 21.0 / 9.0).abs() < 1e-9);
        assert!((parse_ratio("1.85").unwrap_or_default() - 1.85).abs() < 1e-9);
        assert_eq!(parse_ratio("4:0"), None);
        assert_eq!(parse_ratio("wide"), None);
        assert_eq!(parse_ratio("100:1"), None);
    }

    #[test]
    fn crop_and_extend_graphs_read_the_files_shape() {
        assert_eq!(
            crop_graph(ExtendMethod::None, 1.0, [1, 1], [50.0, 50.0], false),
            None
        );
        let crop = crop_graph(ExtendMethod::Crop, 4.0 / 3.0, [0, 0], [50.0, 50.0], false)
            .unwrap_or_default();
        assert!(crop.starts_with("crop=w='if(gt(dar,1.333333)"), "{crop}");
        assert!(crop.contains(":x='(iw-ow)*0.5'"), "{crop}");
        let pad =
            crop_graph(ExtendMethod::Extend, 2.35, [0, 0], [0.0, 100.0], false).unwrap_or_default();
        assert!(pad.starts_with("pad=w='if(lt(dar,2.35)"), "{pad}");
        assert!(
            pad.contains(":x='(ow-iw)*0':y='(oh-ih)*1':color=black"),
            "{pad}"
        );
        let pixels = crop_graph(ExtendMethod::Pixels, 1.0, [640, 480], [50.0, 50.0], false)
            .unwrap_or_default();
        assert!(pixels.contains("crop=w='min(iw,640)'") && pixels.contains("pad=w='max(iw,640)'"));
        // "Scale only frame height".
        assert_eq!(
            crop_graph(ExtendMethod::Extend, 4.0 / 3.0, [0, 0], [50.0, 50.0], true).as_deref(),
            Some("scale=w=iw:h='round(iw*sar/1.333333/2)*2'")
        );
        assert_eq!(margin_graph(0), None);
        assert_eq!(
            margin_graph(10).as_deref(),
            Some("pad=w=iw:h='trunc(ih*110/200)*2':x=0:y=0:color=black")
        );
    }

    #[test]
    fn resizing_follows_its_condition_and_target() {
        let mut resize = Resize::default();
        assert_eq!(resize_graph(&resize, Some((1920, 1080))), None);
        resize.when = ResizeWhen::Smaller;
        let graph = resize_graph(&resize, Some((1920, 1080))).unwrap_or_default();
        assert!(
            graph.starts_with("scale=w='if(lt(iw,1920)*lt(ih,1080),1920,iw)'"),
            "{graph}"
        );
        assert!(
            graph.ends_with("force_divisible_by=2:flags=bicubic"),
            "{graph}"
        );
        // The screen's size is not known yet: nothing to resize to.
        assert_eq!(resize_graph(&resize, None), None);
        resize.when = ResizeWhen::Always;
        resize.target = ResizeTarget::Multiply;
        resize.factor = 1.5;
        let graph = resize_graph(&resize, None).unwrap_or_default();
        assert!(graph.starts_with("scale=w='if(1,iw*1.5,iw)'"), "{graph}");
        resize.target = ResizeTarget::Size;
        resize.width = 1280;
        resize.method = "lanczos".to_owned();
        let graph = resize_graph(&resize, None).unwrap_or_default();
        assert!(graph.contains("h='if(1,1280*ih/iw,ih)'") && graph.ends_with("flags=lanczos"));
        // A size of its own to compare with, the scaler's parameter, the
        // fields apart, and the frame taking the size.
        resize.when = ResizeWhen::Larger;
        resize.condition_display = false;
        resize.condition_size = [1280, 720];
        resize.param = 3.0;
        resize.interlaced = true;
        resize.keep_aspect = false;
        let graph = resize_graph(&resize, None).unwrap_or_default();
        assert!(
            graph.starts_with("scale=w='if(gt(iw,1280)+gt(ih,720),1280,iw)'"),
            "{graph}"
        );
        assert!(
            graph.ends_with("':flags=lanczos:param0=3:interl=1"),
            "{graph}"
        );
    }

    #[test]
    fn rotation_by_aspect_ratio_turns_videos_of_the_other_orientation() {
        let landscape = Some((1920, 1080));
        assert_eq!(auto_rotation(AutoRotate::Off, 0.5, 0, landscape), 0);
        assert_eq!(
            auto_rotation(AutoRotate::Quarter, 16.0 / 9.0, 0, landscape),
            0
        );
        assert_eq!(
            auto_rotation(AutoRotate::Quarter, 9.0 / 16.0, 0, landscape),
            90
        );
        // A wide video turned by its metadata stands upright.
        assert_eq!(
            auto_rotation(AutoRotate::Quarter, 16.0 / 9.0, 90, landscape),
            90
        );
        assert_eq!(
            auto_rotation(AutoRotate::Half, 16.0 / 9.0, 0, landscape),
            180
        );
        assert_eq!(
            auto_rotation(AutoRotate::Half, 9.0 / 16.0, 0, landscape),
            270
        );
        assert_eq!(
            auto_rotation(AutoRotate::Quarter, 16.0 / 9.0, 0, Some((1080, 1920))),
            90
        );
    }

    #[test]
    fn the_output_device_corrects_the_pixel_aspect() {
        let screen = Some((1920, 1080));
        assert!((monitor_pixel_aspect(OutputDevice::Monitor, screen, None) - 1.0).abs() < 1e-9);
        // A 4:3 television showing 1920x1080: its pixels are narrow.
        let tv = monitor_pixel_aspect(OutputDevice::Tv4x3, screen, None);
        assert!((tv - 0.75).abs() < 1e-9, "{tv}");
        let tv = monitor_pixel_aspect(OutputDevice::Tv16x9, screen, None);
        assert!((tv - 1.0).abs() < 1e-9, "{tv}");
        let measured = monitor_pixel_aspect(OutputDevice::Auto, screen, Some((600.0, 340.0)));
        assert!((measured - (600.0 / 340.0) / (16.0 / 9.0)).abs() < 1e-9);
        assert!((monitor_pixel_aspect(OutputDevice::Auto, screen, None) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn subtitle_room_grows_with_lines_and_size() {
        let one = subtitle_room_ratio(SubtitleRoom::OneLine, 1.0);
        let three = subtitle_room_ratio(SubtitleRoom::ThreeLines, 1.0);
        assert!(one > 0.1 && one < 0.15, "{one}");
        assert!(three > 2.0 * one, "{three}");
        assert!(subtitle_room_ratio(SubtitleRoom::ThreeLines, 3.0) <= 0.45);
        assert!(subtitle_room_ratio(SubtitleRoom::Available, 1.0).abs() < f64::EPSILON);
        assert!((keep_aspect_margin(2.35, 16.0 / 9.0) - (1.0 - (16.0 / 9.0) / 2.35)).abs() < 1e-9);
        assert!(keep_aspect_margin(16.0 / 9.0, 16.0 / 9.0).abs() < 1e-9);
    }

    #[test]
    fn the_presets_and_values_are_repaired() {
        let presets = default_presets();
        assert_eq!(presets.len(), 10);
        assert_eq!(presets[3].name, "Zoom 3");
        assert!((presets[3].width - 130.0).abs() < f64::EPSILON);
        assert!((presets[6].height - 100.0).abs() < f64::EPSILON);
        assert_eq!(presets[9].name, "Increasing Height 3");
        let mut picture = Picture {
            custom_aspect: "wide".to_owned(),
            bottom_margin: 12,
            position: [150.0, -3.0],
            ..Picture::default()
        };
        let mut notes = Vec::new();
        picture.validate(&mut notes);
        assert_eq!(picture.custom_aspect, "21:9");
        assert_eq!(picture.bottom_margin, 10);
        assert!(!differs(&picture.position, &[100.0, 0.0]));
        assert_eq!(notes.len(), 4, "{notes:?}");
    }
}
