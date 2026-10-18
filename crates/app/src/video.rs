//! RVP's own video processing (M6): colour adjustments, deinterlacing,
//! video effects, rotation, deband, GLSL shaders and VapourSynth script
//! slots.
//!
//! Rules (goal C's rule 4 as changed for M6):
//!
//! * RVP sets only its own video filters, each under its own label
//!   (`@rvp...`) and added and removed by that label. Other filters, SVP's
//!   `@svp` above all, are never replaced, reordered or removed.
//! * RVP's filters run first, in [`VideoEffect::ALL`]'s order with the
//!   VapourSynth script last, so they come before `@svp`: a change removes
//!   RVP's filters and puts them back in front of the chain (`vf pre`).
//! * Deinterlacing is mpv's own (`deinterlace`), which mpv applies before
//!   every user filter. Colour adjustments, deband, rotation and shaders
//!   belong to the renderer and need no filter.
//! * Filters that run on the CPU need the decoded frames in memory: while
//!   one of them, a VapourSynth script or SVP mode is on, decoding uses the
//!   copy-back variant of the chosen hardware decoder
//!   ([`crate::svp::copy_back`]).
//!
//! PotPlayer's `.vpy` scripts call `vs.get_core()`, which VapourSynth R73
//! (SVP's runtime) no longer has (spike S2): RVP runs them through a
//! generated wrapper that defines it, loads the listed plugins (PotPlayer's
//! script needs mvtools, which SVP does not ship) and executes the script in
//! the same globals, so mpv's `video_in`, `container_fps` and `display_fps`
//! reach it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rvp_engine::{Engine, EngineError};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// mpv's deinterlacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Deinterlace {
    /// Interlaced frames only (mpv 0.38 and later; older mpv: off).
    #[default]
    Auto,
    On,
    Off,
}

impl Deinterlace {
    #[must_use]
    pub fn mpv_value(self) -> &'static str {
        match self {
            Deinterlace::Auto => "auto",
            Deinterlace::On => "yes",
            Deinterlace::Off => "no",
        }
    }

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Deinterlace::Auto => "Auto",
            Deinterlace::On => "On",
            Deinterlace::Off => "Off",
        }
    }
}

/// A video effect RVP runs as its own filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoEffect {
    Deblock,
    Denoise,
    Soften,
    Sharpen,
    FlipVertical,
    FlipHorizontal,
    // M12.
    /// Rotation by any angle (PotPlayer's "Rotation (Horizontal/Vertical)").
    Rotate,
    /// Contrast stretched to the frame's own darkest and brightest.
    AutoLevel,
    /// Input and output levels and a gamma.
    Levels,
    /// A denoiser of chosen strength.
    GradualDenoise,
    /// A temporal denoiser ("Temporary Denoise").
    TemporalDenoise,
    /// The luma and chroma planes moved by some pixels.
    Offset,
    MotionBlur,
    /// PotPlayer's `WarpSharp`, approximated by FFmpeg's contrast adaptive
    /// sharpening (`cas`): it sharpens without `WarpSharp`'s edge warping.
    WarpSharp,
}

impl VideoEffect {
    /// Every effect, in the order the filters run: cleaning first, then
    /// levels, then sharpness, then the geometry.
    pub const ALL: [VideoEffect; 14] = [
        VideoEffect::Deblock,
        VideoEffect::Denoise,
        VideoEffect::GradualDenoise,
        VideoEffect::TemporalDenoise,
        VideoEffect::AutoLevel,
        VideoEffect::Levels,
        VideoEffect::Offset,
        VideoEffect::Soften,
        VideoEffect::Sharpen,
        VideoEffect::WarpSharp,
        VideoEffect::MotionBlur,
        VideoEffect::Rotate,
        VideoEffect::FlipVertical,
        VideoEffect::FlipHorizontal,
    ];

    /// The filter's label in mpv's chain.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            VideoEffect::Deblock => "rvpdeblock",
            VideoEffect::Denoise => "rvpdenoise",
            VideoEffect::Soften => "rvpsoften",
            VideoEffect::Sharpen => "rvpsharpen",
            VideoEffect::FlipVertical => "rvpvflip",
            VideoEffect::FlipHorizontal => "rvphflip",
            VideoEffect::Rotate => "rvprotate",
            VideoEffect::AutoLevel => "rvpautolevel",
            VideoEffect::Levels => "rvplevels",
            VideoEffect::GradualDenoise => "rvpgradual",
            VideoEffect::TemporalDenoise => "rvptemporal",
            VideoEffect::Offset => "rvpoffset",
            VideoEffect::MotionBlur => "rvpmotionblur",
            VideoEffect::WarpSharp => "rvpwarpsharp",
        }
    }

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            VideoEffect::Deblock => "Deblock",
            VideoEffect::Denoise => "Denoise 3D",
            VideoEffect::Soften => "Soften",
            VideoEffect::Sharpen => "Sharpen",
            VideoEffect::FlipVertical => "Flip vertical",
            VideoEffect::FlipHorizontal => "Flip horizontal",
            VideoEffect::Rotate => "Rotation",
            VideoEffect::AutoLevel => "Auto level control",
            VideoEffect::Levels => "Level control",
            VideoEffect::GradualDenoise => "Gradual denoise",
            VideoEffect::TemporalDenoise => "Temporary denoise",
            VideoEffect::Offset => "Luma and chroma offset",
            VideoEffect::MotionBlur => "Motion blur",
            VideoEffect::WarpSharp => "WarpSharp",
        }
    }
}

/// Level control's options (Preferences: Video > Levels/Offset).
// Independent options as PotPlayer's page lists them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Levels {
    /// Auto level control stretches to 0-255 instead of 16-235.
    pub auto_full_range: bool,
    /// Level control's output may use 0-255 (else 16-235).
    pub full_range: bool,
    /// Only the luma changes; the colours keep their levels.
    pub luma_only: bool,
    /// The input range (0-255); with `auto_input` the frame's own.
    pub input: [f64; 2],
    pub auto_input: bool,
    pub output: [f64; 2],
    /// Gamma correction (0.1 to 10; 1 leaves the middle tones).
    pub gamma: f64,
    /// The luma and chroma planes' offsets in pixels (x, y).
    pub luma_offset: [i32; 2],
    pub chroma_offset: [i32; 2],
}

impl Default for Levels {
    fn default() -> Self {
        Self {
            auto_full_range: false,
            full_range: false,
            luma_only: true,
            input: [16.0, 235.0],
            auto_input: false,
            output: [16.0, 235.0],
            gamma: 1.0,
            luma_offset: [0, 0],
            chroma_offset: [0, 0],
        }
    }
}

/// The effects' strengths (Preferences: Video > Effects).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct EffectOptions {
    /// Rotation in degrees, clockwise (PotPlayer's 90, or 270 with its
    /// "270 degrees").
    pub rotate_angle: f64,
    /// Soften's radius (0.1 to 5), luma and chroma strength (0 to 1).
    pub soften: [f64; 3],
    /// Sharpen's luma and chroma amount (0 to 2).
    pub sharpen: [f64; 2],
    /// Deblock's strength (1 to 10).
    pub deblock: f64,
    /// Gradual denoise's strength (1 to 10).
    pub gradual: f64,
    /// Denoise 3D's luma, chroma and temporal strength (0 to 20).
    pub denoise: [f64; 3],
    /// Denoise 3D with non-local means (slower, finer).
    pub denoise_quality: bool,
    /// The temporal noise reducer's thresholds A and B (0.001 to 0.3) and
    /// the frames it averages (3 to 29, odd).
    pub temporal: [f64; 3],
    /// `WarpSharp`'s depth (0 to 1, `cas` strength).
    pub warpsharp: f64,
    /// Motion blur's frames (2 to 10).
    pub motion_blur: u32,
    /// mpv's deband: threshold (1 to 200) and range (1 to 64).
    pub deband: [f64; 2],
    pub levels: Levels,
}

impl Default for EffectOptions {
    fn default() -> Self {
        Self {
            rotate_angle: 90.0,
            soften: [1.0, 0.5, 0.5],
            sharpen: [0.8, 0.0],
            deblock: 3.0,
            gradual: 3.0,
            denoise: [4.0, 3.0, 6.0],
            denoise_quality: false,
            temporal: [0.02, 0.04, 9.0],
            warpsharp: 0.5,
            motion_blur: 3,
            deband: [48.0, 16.0],
            levels: Levels::default(),
        }
    }
}

/// A number for FFmpeg's options, without needless digits.
fn number(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl EffectOptions {
    /// Brings the strengths into their ranges, with a note per repair.
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let clamp = crate::playback::clamp_number;
        clamp(
            &mut self.rotate_angle,
            -360.0,
            360.0,
            "video-effects.rotate-angle",
            notes,
        );
        clamp(&mut self.soften[0], 0.1, 5.0, "video-effects.soften", notes);
        for value in &mut self.soften[1..] {
            clamp(value, 0.0, 1.0, "video-effects.soften", notes);
        }
        for value in &mut self.sharpen {
            clamp(value, 0.0, 2.0, "video-effects.sharpen", notes);
        }
        clamp(&mut self.deblock, 1.0, 10.0, "video-effects.deblock", notes);
        clamp(&mut self.gradual, 1.0, 10.0, "video-effects.gradual", notes);
        for value in &mut self.denoise {
            clamp(value, 0.0, 20.0, "video-effects.denoise", notes);
        }
        for value in &mut self.temporal[..2] {
            clamp(value, 0.001, 0.3, "video-effects.temporal", notes);
        }
        clamp(
            &mut self.temporal[2],
            3.0,
            29.0,
            "video-effects.temporal",
            notes,
        );
        clamp(
            &mut self.warpsharp,
            0.0,
            1.0,
            "video-effects.warpsharp",
            notes,
        );
        if !(2..=10).contains(&self.motion_blur) {
            notes.push("video-effects.motion-blur must be 2 to 10 frames: 3 used".to_owned());
            self.motion_blur = 3;
        }
        clamp(
            &mut self.deband[0],
            1.0,
            200.0,
            "video-effects.deband",
            notes,
        );
        clamp(
            &mut self.deband[1],
            1.0,
            64.0,
            "video-effects.deband",
            notes,
        );
        let levels = &mut self.levels;
        for value in levels.input.iter_mut().chain(levels.output.iter_mut()) {
            clamp(value, 0.0, 255.0, "video-effects.levels", notes);
        }
        if levels.input[1] <= levels.input[0] || levels.output[1] <= levels.output[0] {
            notes.push("video-effects.levels: a range must rise: 16 to 235 used".to_owned());
            levels.input = [16.0, 235.0];
            levels.output = [16.0, 235.0];
        }
        clamp(
            &mut levels.gamma,
            0.1,
            10.0,
            "video-effects.levels.gamma",
            notes,
        );
        for value in levels
            .luma_offset
            .iter_mut()
            .chain(levels.chroma_offset.iter_mut())
        {
            if value.abs() > 64 {
                notes.push("video-effects.levels: offsets must be -64 to 64 pixels".to_owned());
                *value = (*value).clamp(-64, 64);
            }
        }
    }

    /// The FFmpeg filter graph of an effect at these strengths.
    #[must_use]
    pub fn graph(&self, effect: VideoEffect) -> String {
        match effect {
            VideoEffect::Deblock => {
                let s = self.deblock;
                format!(
                    "deblock=filter=strong:block=8:alpha={}:beta={}:gamma={}:delta={}",
                    number(0.033 * s),
                    number(0.017 * s),
                    number(0.017 * s),
                    number(0.017 * s)
                )
            }
            VideoEffect::Denoise => {
                let [luma, chroma, time] = self.denoise;
                if self.denoise_quality {
                    format!(
                        "nlmeans=s={}:p=7:r=15",
                        number((luma / 2.0).clamp(1.0, 30.0))
                    )
                } else {
                    let chroma_time = if luma > 0.0 {
                        time * chroma / luma
                    } else {
                        time
                    };
                    format!(
                        "hqdn3d={}:{}:{}:{}",
                        number(luma),
                        number(chroma),
                        number(time),
                        number(chroma_time)
                    )
                }
            }
            VideoEffect::GradualDenoise => {
                let s = self.gradual;
                format!(
                    "hqdn3d={}:{}:{}:{}",
                    number(s),
                    number(s * 0.75),
                    number(s * 1.5),
                    number(s * 1.125)
                )
            }
            VideoEffect::TemporalDenoise => {
                let [a, b, frames] = self.temporal;
                // An odd count of frames, as atadenoise wants.
                // The count is 3 to 29 after `validate`.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let frames = (frames.round() as u32) | 1;
                let (a, b) = (number(a), number(b));
                format!("atadenoise=0a={a}:1a={a}:2a={a}:0b={b}:1b={b}:2b={b}:s={frames}")
            }
            VideoEffect::AutoLevel => {
                let (black, white) = if self.levels.auto_full_range {
                    ("black", "white")
                } else {
                    ("0x101010", "0xebebeb")
                };
                format!("normalize=blackpt={black}:whitept={white}:smoothing=24:independence=0")
            }
            VideoEffect::Levels => self.levels_graph(),
            VideoEffect::Offset => self.offset_graph(),
            VideoEffect::Soften => {
                let [radius, luma, chroma] = self.soften;
                format!(
                    "smartblur=lr={r}:ls={}:lt=0:cr={r}:cs={}:ct=0",
                    number(luma),
                    number(chroma),
                    r = number(radius)
                )
            }
            VideoEffect::Sharpen => {
                let [luma, chroma] = self.sharpen;
                format!("unsharp=5:5:{}:5:5:{}", number(luma), number(chroma))
            }
            VideoEffect::WarpSharp => format!("cas=strength={}", number(self.warpsharp)),
            VideoEffect::MotionBlur => format!("tmix=frames={}", self.motion_blur),
            VideoEffect::Rotate => rotate_graph(self.rotate_angle),
            VideoEffect::FlipVertical => "vflip".to_owned(),
            VideoEffect::FlipHorizontal => "hflip".to_owned(),
        }
    }

    fn levels_graph(&self) -> String {
        let levels = &self.levels;
        let [mut out_low, mut out_high] = levels.output;
        if !levels.full_range {
            out_low = out_low.max(16.0);
            out_high = out_high.min(235.0);
        }
        let gamma = number(1.0 / levels.gamma);
        if levels.auto_input {
            // The frame's own range, stretched to the output's.
            let gray = |level: f64| {
                // The level is 0 to 255 after validation.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let byte = level.round() as u8;
                format!("0x{byte:02x}{byte:02x}{byte:02x}")
            };
            return format!(
                "normalize=blackpt={}:whitept={}:smoothing=24:independence=0,lutyuv=y='pow(val/maxval,{gamma})*maxval'",
                gray(out_low),
                gray(out_high)
            );
        }
        let [low, high] = levels.input;
        let curve = format!(
            "pow(clip((val/maxval*255-{})/{},0,1),{gamma})",
            number(low),
            number(high - low)
        );
        let scale = format!(
            "({}+{}*{curve})/255*maxval",
            number(out_low),
            number(out_high - out_low)
        );
        if levels.luma_only {
            format!("lutyuv=y='{scale}'")
        } else {
            format!("lutrgb=r='{scale}':g='{scale}':b='{scale}'")
        }
    }

    fn offset_graph(&self) -> String {
        let [lx, ly] = self.levels.luma_offset;
        let [cx, cy] = self.levels.chroma_offset;
        // The chroma moves against the luma; then the whole picture moves
        // by the luma's offset, black where it leaves the frame.
        let (h, v) = (cx - lx, cy - ly);
        let mut graph = format!("chromashift=cbh={h}:cbv={v}:crh={h}:crv={v}");
        if lx != 0 || ly != 0 {
            let (ax, ay) = (lx.abs(), ly.abs());
            let _ = write!(
                graph,
                ",crop=w=iw-{ax}:h=ih-{ay}:x={}:y={},pad=w=iw+{ax}:h=ih+{ay}:x={}:y={}:color=black",
                (-lx).max(0),
                (-ly).max(0),
                lx.max(0),
                ly.max(0)
            );
        }
        graph
    }
}

/// Rotation by `degrees`: the lossless filters for quarter turns, else
/// `rotate` with a frame large enough for the turned picture.
#[must_use]
pub fn rotate_graph(degrees: f64) -> String {
    let turned = degrees.rem_euclid(360.0);
    if (turned - 90.0).abs() < 0.01 {
        "transpose=clock".to_owned()
    } else if (turned - 180.0).abs() < 0.01 {
        "hflip,vflip".to_owned()
    } else if (turned - 270.0).abs() < 0.01 {
        "transpose=cclock".to_owned()
    } else if turned < 0.01 || turned > 359.99 {
        "null".to_owned()
    } else {
        let a = format!("{}*PI/180", number(turned));
        format!("rotate={a}:ow='rotw({a})':oh='roth({a})':c=black")
    }
}

/// The label of the VapourSynth script slot's filter.
pub const SCRIPT_LABEL: &str = "rvpvs";

/// One of the colour adjustments (mpv's renderer properties, -100 to 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorKind {
    Brightness,
    Contrast,
    Saturation,
    Hue,
    Gamma,
}

impl ColorKind {
    pub const ALL: [ColorKind; 5] = [
        ColorKind::Brightness,
        ColorKind::Contrast,
        ColorKind::Saturation,
        ColorKind::Hue,
        ColorKind::Gamma,
    ];

    #[must_use]
    pub fn property(self) -> &'static str {
        match self {
            ColorKind::Brightness => "brightness",
            ColorKind::Contrast => "contrast",
            ColorKind::Saturation => "saturation",
            ColorKind::Hue => "hue",
            ColorKind::Gamma => "gamma",
        }
    }

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            ColorKind::Brightness => "Brightness",
            ColorKind::Contrast => "Contrast",
            ColorKind::Saturation => "Saturation",
            ColorKind::Hue => "Hue",
            ColorKind::Gamma => "Gamma",
        }
    }
}

/// A colour control change (PotPlayer's 1 % steps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorChange {
    Down(ColorKind),
    Up(ColorKind),
    Reset(ColorKind),
    /// "Disable/Last used Color Controls": all adjustments off and back.
    Toggle,
}

/// Colour adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Color {
    /// Off: the picture as decoded; the values are kept for "last used".
    pub enabled: bool,
    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
    pub hue: i32,
    pub gamma: i32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            enabled: true,
            brightness: 0,
            contrast: 0,
            saturation: 0,
            hue: 0,
            gamma: 0,
        }
    }
}

impl Color {
    #[must_use]
    pub fn get(&self, kind: ColorKind) -> i32 {
        match kind {
            ColorKind::Brightness => self.brightness,
            ColorKind::Contrast => self.contrast,
            ColorKind::Saturation => self.saturation,
            ColorKind::Hue => self.hue,
            ColorKind::Gamma => self.gamma,
        }
    }

    pub fn set(&mut self, kind: ColorKind, value: i32) {
        let value = value.clamp(-100, 100);
        match kind {
            ColorKind::Brightness => self.brightness = value,
            ColorKind::Contrast => self.contrast = value,
            ColorKind::Saturation => self.saturation = value,
            ColorKind::Hue => self.hue = value,
            ColorKind::Gamma => self.gamma = value,
        }
    }

    /// The value mpv gets: the setting, or 0 while the controls are off.
    #[must_use]
    pub fn applied(&self, kind: ColorKind) -> i32 {
        if self.enabled { self.get(kind) } else { 0 }
    }
}

/// The screen rotation in degrees (mpv's `video-rotate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rotation {
    Cycle,
    To(u16),
}

/// The pixel shaders (mpv's `glsl-shaders`): PotPlayer's pre-resize and
/// post-resize lists, each on or off with its files in order
/// (`crate::shaders`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Shaders {
    /// The pre-resize list (M6's one list).
    pub enabled: bool,
    pub files: Vec<PathBuf>,
    /// The post-resize list.
    pub post_enabled: bool,
    pub post_files: Vec<PathBuf>,
    /// Further folders of shaders for the lists.
    pub folders: Vec<PathBuf>,
}

/// VapourSynth script slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct VapourSynth {
    /// The active script runs.
    pub enabled: bool,
    /// The scripts to choose from (PotPlayer's `.vpy` files work).
    pub slots: Vec<PathBuf>,
    /// The slot that runs; unset: none.
    pub script: Option<PathBuf>,
    /// Plugins loaded before the script (mvtools for PotPlayer's).
    pub plugins: Vec<PathBuf>,
    /// AviSynth plugins loaded before the script through VapourSynth's
    /// `avs.LoadPlugin` (Windows; M12).
    pub avs_plugins: Vec<PathBuf>,
    /// The VSScript library; unset: SVP's runtime when installed, else the
    /// system's (read at start).
    pub library: Option<PathBuf>,
    /// mpv's `buffered-frames` for the filter.
    pub buffered_frames: u32,
}

impl Default for VapourSynth {
    fn default() -> Self {
        Self {
            enabled: false,
            slots: Vec::new(),
            script: None,
            plugins: Vec::new(),
            avs_plugins: Vec::new(),
            library: None,
            buffered_frames: 4,
        }
    }
}

/// A Python string literal for a path.
fn python_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// The wrapper that runs `script` with today's VapourSynth (see the module
/// documentation).
#[must_use]
pub fn script_wrapper(script: &Path, plugins: &[PathBuf], avs_plugins: &[PathBuf]) -> String {
    let script = python_string(&script.to_string_lossy());
    let mut text = String::from(
        "# Generated by RVP: runs a PotPlayer VapourSynth script with the\n\
         # VapourSynth in use (R73 removed get_core) and loads its plugins.\n\
         import vapoursynth as vs\n\
         if not hasattr(vs, 'get_core'):\n\
         \x20   vs.get_core = lambda *args, **kwargs: vs.core\n",
    );
    for plugin in plugins {
        let _ = write!(
            text,
            "try:\n    vs.core.std.LoadPlugin({})\nexcept vs.Error:\n    pass\n",
            python_string(&plugin.to_string_lossy())
        );
    }
    // VapourSynth's AviSynth compatibility exists on Windows only.
    for plugin in avs_plugins {
        let _ = write!(
            text,
            "try:\n    vs.core.avs.LoadPlugin({})\nexcept (vs.Error, AttributeError):\n    pass\n",
            python_string(&plugin.to_string_lossy())
        );
    }
    let _ = write!(
        text,
        "with open({script}, encoding='utf-8-sig') as source:\n\
         \x20   exec(compile(source.read(), {script}, 'exec'))\n"
    );
    text
}

/// The `vf` entry for the VapourSynth filter on `wrapper`: mpv splits its
/// arguments at `:`, so the path is given with its length (`%LEN%`).
#[must_use]
pub fn script_entry(wrapper: &Path, buffered_frames: u32) -> String {
    let path = wrapper.to_string_lossy().replace('\\', "/");
    format!(
        "@{SCRIPT_LABEL}:vapoursynth=file=%{}%{path}:buffered-frames={}",
        path.len(),
        buffered_frames.clamp(1, 64)
    )
}

impl App {
    /// RVP's video filters, in the order they run.
    #[must_use]
    pub fn video_filters(&self) -> Vec<(String, String)> {
        // The forced matrix and RVP's deinterlacer on the source, 3D and
        // 360° next, then the picture's geometry, the effects and the
        // script.
        let mut filters = self.settings.render.filters(self.settings.deinterlace);
        filters.extend(self.stereo_filters());
        filters.extend(self.picture_filters());
        filters.extend(
            VideoEffect::ALL
                .iter()
                .filter(|effect| self.settings.effects.contains(effect))
                .map(|effect| {
                    (
                        effect.label().to_owned(),
                        format!(
                            "@{}:lavfi=[{}]",
                            effect.label(),
                            self.settings.video_effects.graph(*effect)
                        ),
                    )
                }),
        );
        let vs = &self.settings.vapoursynth;
        if vs.enabled && vs.script.is_some() {
            let wrapper = self.state_dir().join("vapoursynth").join("rvp-script.vpy");
            filters.push((
                SCRIPT_LABEL.to_owned(),
                script_entry(&wrapper, vs.buffered_frames),
            ));
        }
        filters
    }

    /// True while a filter that needs decoded frames in memory runs.
    #[must_use]
    pub fn needs_copy_back(&self) -> bool {
        !self.video_filters().is_empty() || crate::svp::svp_mode(&self.settings)
    }

    /// Puts RVP's video filters in front of mpv's chain, in their order,
    /// leaving every other filter where it is. In the last moments of a
    /// file they wait for the next one or for playback to leave the end:
    /// a `vf` change there can leave mpv stuck before the end (seen with
    /// libmpv 0.37: the time stops, `eof-reached` never comes), as an audio
    /// chain change can (`App::defer_chain_change`).
    ///
    /// # Errors
    /// Fails when the VapourSynth wrapper cannot be written or libmpv
    /// rejects a filter.
    pub fn apply_video_filters(&self, engine: &Engine) -> Result<(), String> {
        if self.near_end() {
            self.video_pending.set(true);
            return Ok(());
        }
        self.video_pending.set(false);
        let vs = &self.settings.vapoursynth;
        if vs.enabled
            && let Some(script) = &vs.script
        {
            let wrapper = self.state_dir().join("vapoursynth").join("rvp-script.vpy");
            crate::storage::write_atomic(
                &wrapper,
                script_wrapper(script, &vs.plugins, &vs.avs_plugins).as_bytes(),
            )
            .map_err(|error| format!("{}: {error}", wrapper.display()))?;
        }
        // Only labels in the chain: mpv logs a warning for the others.
        let chain = engine.get_text("vf").unwrap_or_default();
        let labels = [
            crate::render::MATRIX_LABEL,
            crate::render::DEINTERLACE_LABEL,
            crate::stereo::STEREO_LABEL,
            crate::stereo::VR_LABEL,
            crate::picture::RESIZE_LABEL,
            crate::picture::CROP_LABEL,
            crate::picture::MARGIN_LABEL,
        ]
        .into_iter()
        .chain(VideoEffect::ALL.iter().map(|effect| effect.label()))
        .chain([SCRIPT_LABEL]);
        for label in labels {
            if chain.contains(&format!("@{label}:")) {
                drop(engine.remove_video_filter(label));
            }
        }
        for (_, entry) in self.video_filters().iter().rev() {
            engine
                .prepend_video_filter(entry)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Sets the renderer's colour, deinterlacing, deband and shaders.
    ///
    /// # Errors
    /// Fails when libmpv rejects a value (deinterlacing's `auto` falls back
    /// to off on mpv before 0.38).
    pub fn apply_picture_settings(&self, engine: &Engine) -> Result<(), EngineError> {
        let color = &self.settings.color;
        for kind in ColorKind::ALL {
            engine.set_double(kind.property(), f64::from(color.applied(kind)))?;
        }
        // RVP's own deinterlacer takes mpv's place (`render.deinterlacer`).
        let deinterlace = if self.settings.render.deinterlacer.is_empty() {
            self.settings.deinterlace.mpv_value()
        } else {
            "no"
        };
        if engine.set_text("deinterlace", deinterlace).is_err() {
            engine.set_text("deinterlace", "no")?;
        }
        // Only the gpu renderers have deband; the headless output has not.
        drop(engine.set_flag("deband", self.settings.deband));
        let [threshold, range] = self.settings.video_effects.deband;
        drop(engine.set_double("deband-threshold", threshold));
        drop(engine.set_double("deband-range", range));
        // Shaders that cannot run are named when a list changes
        // (`App::shader_command`).
        self.apply_shaders(engine).map(drop)
    }

    /// Sets mpv's shader list from the settings' two lists; returns the
    /// shaders left out, with why (`App::shader_files`).
    ///
    /// # Errors
    /// Fails when libmpv rejects the list.
    pub fn apply_shaders(&self, engine: &Engine) -> Result<Vec<String>, EngineError> {
        let (files, refused) = self.shader_files();
        engine.command("change-list", &["glsl-shaders", "clr", ""])?;
        for file in &files {
            engine.command(
                "change-list",
                &["glsl-shaders", "append", &file.to_string_lossy()],
            )?;
        }
        Ok(refused)
    }

    /// Where RVP keeps shader files by default: `shaders` next to the
    /// settings, or in the state folder.
    #[must_use]
    pub fn shader_folder(&self) -> PathBuf {
        self.settings_path().and_then(Path::parent).map_or_else(
            || self.state_dir().join("shaders"),
            |dir| dir.join("shaders"),
        )
    }

    /// The video commands of M6.
    pub(crate) fn video_command(
        &mut self,
        engine: &Engine,
        action: crate::action::Action,
    ) -> Option<UiRequest> {
        use crate::action::Action;
        match action {
            Action::Color(change) => Some(self.change_color(engine, change)),
            Action::DeinterlaceCycle => {
                let next = match self.settings.deinterlace {
                    Deinterlace::Auto => Deinterlace::On,
                    Deinterlace::On => Deinterlace::Off,
                    Deinterlace::Off => Deinterlace::Auto,
                };
                Some(self.set_deinterlace(engine, next))
            }
            Action::SetDeinterlace(mode) => Some(self.set_deinterlace(engine, mode)),
            Action::VideoEffect(effect) => Some(self.toggle_effect(engine, effect)),
            Action::ToggleDeband => {
                self.settings.deband = !self.settings.deband;
                drop(engine.set_flag("deband", self.settings.deband));
                self.save_settings();
                Some(on_off("Deband", self.settings.deband))
            }
            Action::Rotate(rotation) => Some(self.rotate(engine, rotation)),
            Action::Shaders(command) => Some(self.shader_command(engine, command)),
            Action::ToggleVapourSynth => Some(self.toggle_script(engine)),
            Action::OpenScriptFolder => {
                let vs = &self.settings.vapoursynth;
                let folder = vs
                    .script
                    .as_ref()
                    .or_else(|| vs.slots.first())
                    .and_then(|script| script.parent())
                    .map_or_else(|| self.state_dir().join("vapoursynth"), Path::to_path_buf);
                drop(std::fs::create_dir_all(&folder));
                Some(UiRequest::OpenExternal(folder))
            }
            _ => None,
        }
    }

    fn change_color(&mut self, engine: &Engine, change: ColorChange) -> UiRequest {
        let color = &mut self.settings.color;
        let text = match change {
            ColorChange::Toggle => {
                color.enabled = !color.enabled;
                if color.enabled {
                    "Color controls: last used".to_owned()
                } else {
                    "Color controls: off".to_owned()
                }
            }
            ColorChange::Down(kind) | ColorChange::Up(kind) | ColorChange::Reset(kind) => {
                let value = match change {
                    ColorChange::Down(_) => color.get(kind) - 1,
                    ColorChange::Up(_) => color.get(kind) + 1,
                    _ => 0,
                };
                color.set(kind, value);
                color.enabled = true;
                format!("{}: {}", kind.text(), color.get(kind))
            }
        };
        for kind in ColorKind::ALL {
            drop(engine.set_double(
                kind.property(),
                f64::from(self.settings.color.applied(kind)),
            ));
        }
        self.save_settings();
        UiRequest::Osd(text)
    }

    /// Sets one colour adjustment from a slider (-100 to 100).
    pub fn set_color(&mut self, engine: &Engine, kind: ColorKind, value: i32) {
        self.settings.color.set(kind, value);
        self.settings.color.enabled = true;
        for kind in ColorKind::ALL {
            drop(engine.set_double(
                kind.property(),
                f64::from(self.settings.color.applied(kind)),
            ));
        }
        self.save_settings();
    }

    fn set_deinterlace(&mut self, engine: &Engine, mode: Deinterlace) -> UiRequest {
        self.settings.deinterlace = mode;
        if engine.set_text("deinterlace", mode.mpv_value()).is_err() {
            drop(engine.set_text("deinterlace", "no"));
        }
        self.save_settings();
        UiRequest::Osd(format!("Deinterlacing: {}", mode.text()))
    }

    fn toggle_effect(&mut self, engine: &Engine, effect: VideoEffect) -> UiRequest {
        let on = !self.settings.effects.contains(&effect);
        if on {
            self.settings.effects.insert(effect);
        } else {
            self.settings.effects.remove(&effect);
        }
        self.save_settings();
        self.apply_decoding(engine);
        match self.apply_video_filters(engine) {
            Ok(()) => on_off(effect.text(), on),
            Err(error) => UiRequest::Message(format!("{}: {error}", effect.text())),
        }
    }

    fn rotate(&mut self, engine: &Engine, rotation: Rotation) -> UiRequest {
        let current = self.session.rotation;
        let next = match rotation {
            Rotation::Cycle => (current + 90) % 360,
            Rotation::To(degrees) => degrees % 360,
        };
        self.session.rotation = next;
        // An integer choice: mpv takes it as text, not as a double.
        drop(engine.set_text("video-rotate", &next.to_string()));
        UiRequest::Osd(format!("Rotation: {next}°"))
    }

    /// Chooses the script slot `index` and runs it (the VapourSynth
    /// menu's list).
    pub fn pick_script(&mut self, engine: &Engine, index: usize) -> Option<UiRequest> {
        let vs = &mut self.settings.vapoursynth;
        let script = vs.slots.get(index)?.clone();
        let name = crate::view::file_name(&script.to_string_lossy()).to_owned();
        vs.script = Some(script);
        vs.enabled = true;
        self.save_settings();
        self.apply_decoding(engine);
        Some(match self.apply_video_filters(engine) {
            Ok(()) => UiRequest::Osd(format!("VapourSynth: {name}")),
            Err(error) => UiRequest::Message(format!("VapourSynth: {error}")),
        })
    }

    fn toggle_script(&mut self, engine: &Engine) -> UiRequest {
        let vs = &mut self.settings.vapoursynth;
        if vs.script.is_none() {
            return UiRequest::Osd("VapourSynth: no script chosen (settings)".to_owned());
        }
        vs.enabled = !vs.enabled;
        let on = vs.enabled;
        self.save_settings();
        self.apply_decoding(engine);
        match self.apply_video_filters(engine) {
            Ok(()) => on_off("VapourSynth script", on),
            Err(error) => UiRequest::Message(format!("VapourSynth: {error}")),
        }
    }

    /// The decoder mpv uses: the setting, or its copy-back variant while a
    /// CPU filter, a script or SVP mode needs frames in memory.
    #[must_use]
    pub fn effective_hwdec(&self) -> String {
        if self.needs_copy_back() {
            crate::svp::copy_back(&self.settings.hwdec)
        } else {
            self.settings.hwdec.clone()
        }
    }

    /// Sets mpv's decoder when it should change.
    pub fn apply_decoding(&self, engine: &Engine) {
        let hwdec = self.effective_hwdec();
        if engine.get_text("hwdec").as_deref() != Some(hwdec.as_str()) {
            drop(engine.set_text("hwdec", &hwdec));
        }
    }
}

fn on_off(name: &str, on: bool) -> UiRequest {
    UiRequest::Osd(format!("{name}: {}", if on { "on" } else { "off" }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_defines_get_core_loads_plugins_and_runs_the_script() {
        let text = script_wrapper(
            Path::new("C:\\Program Files\\DAUM\\PotPlayer\\x's.vpy"),
            &[PathBuf::from("/opt/mvtools/libmvtools.so")],
            &[PathBuf::from("C:\\AviSynth\\plugins64\\RgTools.dll")],
        );
        assert!(
            text.contains("vs.core.avs.LoadPlugin('C:\\\\AviSynth\\\\plugins64\\\\RgTools.dll')")
        );
        assert!(text.contains("vs.get_core = lambda"));
        assert!(text.contains("vs.core.std.LoadPlugin('/opt/mvtools/libmvtools.so')"));
        assert!(text.contains(
            "open('C:\\\\Program Files\\\\DAUM\\\\PotPlayer\\\\x\\'s.vpy', encoding='utf-8-sig')"
        ));
        assert!(text.contains("exec(compile(source.read()"));
    }

    #[test]
    fn script_entries_quote_the_path() {
        let entry = script_entry(Path::new("C:\\state\\rvp-script.vpy"), 4);
        assert_eq!(
            entry,
            "@rvpvs:vapoursynth=file=%23%C:/state/rvp-script.vpy:buffered-frames=4"
        );
    }

    #[test]
    fn effect_graphs_follow_their_strengths() {
        assert_eq!(rotate_graph(90.0), "transpose=clock");
        assert_eq!(rotate_graph(-90.0), "transpose=cclock");
        assert_eq!(rotate_graph(180.0), "hflip,vflip");
        assert_eq!(rotate_graph(0.0), "null");
        assert!(rotate_graph(30.0).starts_with("rotate=30*PI/180:ow='rotw(30*PI/180)'"));
        let mut options = EffectOptions::default();
        assert_eq!(options.graph(VideoEffect::Denoise), "hqdn3d=4:3:6:4.5");
        options.denoise_quality = true;
        assert_eq!(options.graph(VideoEffect::Denoise), "nlmeans=s=2:p=7:r=15");
        assert_eq!(
            options.graph(VideoEffect::TemporalDenoise),
            "atadenoise=0a=0.02:1a=0.02:2a=0.02:0b=0.04:1b=0.04:2b=0.04:s=9"
        );
        assert_eq!(options.graph(VideoEffect::MotionBlur), "tmix=frames=3");
        assert!(
            options
                .graph(VideoEffect::Levels)
                .starts_with("lutyuv=y='(16+219*pow(")
        );
        options.levels.luma_only = false;
        assert!(options.graph(VideoEffect::Levels).starts_with("lutrgb=r="));
        options.levels.luma_offset = [2, -1];
        options.levels.chroma_offset = [3, 0];
        assert_eq!(
            options.graph(VideoEffect::Offset),
            "chromashift=cbh=1:cbv=1:crh=1:crv=1,crop=w=iw-2:h=ih-1:x=0:y=1,pad=w=iw+2:h=ih+1:x=2:y=0:color=black"
        );
        let mut notes = Vec::new();
        options.motion_blur = 40;
        options.levels.input = [200.0, 100.0];
        options.validate(&mut notes);
        assert_eq!(options.motion_blur, 3);
        assert!((options.levels.input[0] - 16.0).abs() < f64::EPSILON);
        assert!((options.levels.input[1] - 235.0).abs() < f64::EPSILON);
        assert_eq!(notes.len(), 2, "{notes:?}");
    }

    #[test]
    fn colour_values_stay_in_range_and_switch_off_together() {
        let mut color = Color::default();
        color.set(ColorKind::Hue, 250);
        assert_eq!(color.get(ColorKind::Hue), 100);
        color.set(ColorKind::Brightness, -7);
        color.enabled = false;
        assert_eq!(color.applied(ColorKind::Brightness), 0);
        color.enabled = true;
        assert_eq!(color.applied(ColorKind::Brightness), -7);
    }
}
