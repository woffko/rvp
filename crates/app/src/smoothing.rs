//! M18: RVP's own motion smoothing for users without SVP (D16, D18).
//!
//! * Modes: off (the default), Blend (mpv's interpolation: neighbouring
//!   frames blended at the display's rate, no motion estimation) and three
//!   motion-compensated engines: MVTools (CPU) and RIFE (GPU, Vulkan) in
//!   VapourSynth, and `fruc_vulkan` (NVIDIA's optical flow, RTX 30 and later)
//!   in FFmpeg. Turned on, smoothing is MVTools medium up to 1080p and
//!   Blend above it (`max-height`), at the display's rate (owner,
//!   2026-10-19). `docs/spikes/m18-smoothing.md` has the study.
//! * The motion-compensated engines are RVP's labelled filter
//!   [`LABEL`] after RVP's other filters (`App::video_filters`); like them
//!   they need copy-back decoding. `fruc_vulkan` works on frames on the
//!   Vulkan device of mpv's output: mpv uploads them in two filters of
//!   RVP's before it ([`FRUC_LABELS`]), since FFmpeg's own `hwupload` now
//!   wants its device when the graph is parsed, before mpv's lavfi filter
//!   hands it over. Blend is mpv's `interpolation` with
//!   `video-sync=display-resample`; with the OpenGL renderer mpv does not
//!   know the display's rate, so RVP gives it (`display-fps-override`).
//! * Smoothing stays off while SVP is attached (`@svp` in the chain; RVP
//!   never touches SVP's filter), while RVP's own VapourSynth script slot
//!   runs, and when the video already plays at the target rate. In SVP
//!   mode it waits for SVP for `svp::ATTACH_WAIT` after a file starts
//!   while SVP 4 Manager runs: SVP does not attach to a video RVP's filter
//!   already plays. An engine
//!   that cannot run here gives way: RIFE and `fruc_vulkan` to MVTools,
//!   MVTools to Blend. One that runs but leaves the output dropping more
//!   than 5 % of its frames over the page's drop window, or more than half
//!   of them over 5 s, takes a lighter step for the rest of the file (the
//!   owner, 2026-10-20 and 2026-11-06): a lighter preset, then RIFE or
//!   `fruc_vulkan` give way to MVTools, then Blend; RIFE over half gives
//!   way at once (its presets change only its GPU threads). On the host (an RTX
//!   30-series card at its test power limit) RIFE and `fruc_vulkan` medium reach 720p
//!   at 60 fps but not 1080p (RIFE quality at 1080p: 40 to 50 of 60 frames
//!   a second dropped, the card at 99 %), and a minute's drop window held
//!   such a step for a minute before the step at once came in.
//! * A filter that keeps up while the output drops a frame in each second
//!   for 10 s (the state other programs' load left behind until the next
//!   seek, the owner's log of 2026-11-11) gets a seek to where playback is,
//!   at most once a minute, and none again in a file where the drops came
//!   back within that minute (`LagWatch`).
//! * MVTools and RIFE run in the VapourSynth runtime RVP ships (never
//!   SVP's). The scripts end with `core.clear_cache()`: mpv answers the
//!   frames a script asks for while it loads with black ones (MVTools asks
//!   for one in each `Analyse`), and VapourSynth's cache would hand them
//!   out as the first frames after every start and seek.
//! * The GPU power budget's first steps are the smoothing's (D17,
//!   `crate::gpu_power`): a lighter preset (RIFE gives way to MVTools),
//!   then Blend, then off.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_engine::{DemuxVideo, Engine as Mpv, PropertyId};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::gpu_power::Step;

/// The label of RVP's smoothing filter.
pub const LABEL: &str = "rvpsmooth";

/// The labels of the filters before `fruc_vulkan`: to NV12, then mpv's
/// upload to its output's Vulkan device.
pub const FRUC_LABELS: [&str; 2] = ["rvpsmoothnv12", "rvpsmoothvk"];

/// The target rates the settings offer: the display's, twice the
/// source's, or a rate.
pub const TARGETS: &[&str] = &[
    "display", "2x", "48", "50", "60", "72", "90", "100", "120", "144",
];

/// The heights the size limit offers.
pub const MAX_HEIGHTS: &[u32] = &[480, 576, 720, 1080, 1440, 2160];

/// The Motion Smoothing page's choices for `drop-window`: how many seconds
/// of the output's dropped frames are counted before smoothing steps down
/// (0: it never does). Short windows step down for a moment's stutter that
/// the next seconds would not have shown (the owner's host, 2026-11-02).
pub const DROP_WINDOWS: &[u32] = &[0, 6, 10, 20, 30, 60];

/// The Motion Smoothing page's choices for `original-first`: seconds of
/// the original video after a start or a seek before a VapourSynth engine
/// takes over (0: at once). SVP's way (the owner, 2026-11-04): while
/// MVTools or RIFE fill their pipeline, the picture stutters.
pub const ORIGINAL_FIRST: &[u32] = &[0, 1, 2, 3, 5];

/// The Motion Smoothing page's choices for `scene-change`: how a cut
/// shows (the owner, 2026-11-08: RIFE morphed one scene into the next).
/// `off` interpolates across it, as the engines do on their own. `normal`
/// and `high` take a source frame whose next one differs from it by more
/// than [`scene_threshold`] in mean luma for the last of a scene, and
/// repeat it in place of the frames between the two (SVP's "repeat the
/// last frame"). The higher sensitivity catches cuts between alike scenes,
/// and also takes a fast pan or a flash for one more often (one repeated
/// frame there).
pub const SCENE_CHANGES: &[&str] = &["off", "normal", "high"];

/// The mean luma difference between two source frames (0 to 1,
/// VapourSynth's `PlaneStatsDiff`) above which `scene_change` takes them
/// for a cut; `None`: no cut is looked for. 0.1 is `misc.SCDetect`'s,
/// which RVP's VapourSynth (R73, no `misc`) lacks.
#[must_use]
pub fn scene_threshold(scene_change: &str) -> Option<f64> {
    match scene_change {
        "normal" => Some(0.1),
        "high" => Some(0.05),
        _ => None,
    }
}

/// RIFE's model, as the study measured it.
pub const RIFE_MODEL: &str = "rife-v4.6_ensembleFalse";

/// A smoothing engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum Engine {
    #[serde(rename = "blend")]
    Blend,
    #[default]
    #[serde(rename = "mvtools")]
    MvTools,
    #[serde(rename = "rife")]
    Rife,
    /// RIFE through NVIDIA's TensorRT (`crate::tensorrt`).
    #[serde(rename = "rife-trt")]
    RifeTrt,
    #[serde(rename = "fruc-vulkan")]
    FrucVulkan,
}

impl Engine {
    pub const ALL: [Self; 5] = [
        Self::Blend,
        Self::MvTools,
        Self::Rife,
        Self::RifeTrt,
        Self::FrucVulkan,
    ];

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::Blend => "Blend",
            Self::MvTools => "MVTools",
            Self::Rife => "RIFE",
            Self::RifeTrt => "RIFE (TensorRT)",
            Self::FrucVulkan => "NVIDIA Optical Flow (fruc_vulkan)",
        }
    }

    /// The settings name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Blend => "blend",
            Self::MvTools => "mvtools",
            Self::Rife => "rife",
            Self::RifeTrt => "rife-trt",
            Self::FrucVulkan => "fruc-vulkan",
        }
    }

    /// RIFE on ncnn or on TensorRT: presets that change only how much of
    /// the GPU runs at once, and a lighter step gives way to MVTools.
    #[must_use]
    pub fn is_rife(self) -> bool {
        matches!(self, Self::Rife | Self::RifeTrt)
    }
}

/// A quality preset: MVTools' block size and search, RIFE's GPU threads,
/// `fruc_vulkan`'s `perf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    Fast,
    #[default]
    Medium,
    Quality,
}

impl Preset {
    pub const ALL: [Self; 3] = [Self::Fast, Self::Medium, Self::Quality];

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::Fast => "Fast",
            Self::Medium => "Medium",
            Self::Quality => "Quality",
        }
    }

    /// One step lighter; Fast stays.
    #[must_use]
    pub fn lighter(self) -> Self {
        match self {
            Self::Quality => Self::Medium,
            Self::Medium | Self::Fast => Self::Fast,
        }
    }
}

/// The smoothing settings (`smoothing` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SmoothingSettings {
    /// Smoothing is on (Video > Motion Smoothing, Ctrl+Shift+I).
    pub enabled: bool,
    /// The engine when it is on.
    pub engine: Engine,
    pub preset: Preset,
    /// The rate it makes: one of [`TARGETS`].
    pub target: String,
    /// Videos taller than this get Blend instead of motion compensation.
    pub max_height: u32,
    /// Seconds of dropped frames counted before a lighter step (0:
    /// never; [`DROP_WINDOWS`]).
    pub drop_window: u32,
    /// Seconds of the original after a start or a seek ([`ORIGINAL_FIRST`]).
    pub original_first: u32,
    /// How a cut shows: one of [`SCENE_CHANGES`].
    pub scene_change: String,
    /// The original first for RIFE (TensorRT) too (off by default: its
    /// first frames after a start or seek take about a second, which mpv
    /// waits for when they come first; with the original first, motion
    /// resumes at once but about 60 frames drop at the switch, the host's
    /// runs of 2026-11-08).
    pub original_first_tensorrt: bool,
}

impl SmoothingSettings {
    /// The seconds of the original first for `engine` (the scripts'
    /// `original`).
    #[must_use]
    pub fn original_for(&self, engine: Engine) -> u32 {
        if engine == Engine::RifeTrt && !self.original_first_tensorrt {
            0
        } else {
            self.original_first
        }
    }
}

impl Default for SmoothingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: Engine::MvTools,
            preset: Preset::Medium,
            target: "display".to_owned(),
            max_height: 1080,
            drop_window: 20,
            original_first: 1,
            scene_change: "normal".to_owned(),
            original_first_tensorrt: false,
        }
    }
}

impl SmoothingSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !TARGETS.contains(&self.target.as_str()) {
            notes.push(format!(
                "smoothing.target {:?} is not one of {}: display used",
                self.target,
                TARGETS.join(", ")
            ));
            "display".clone_into(&mut self.target);
        }
        if !(240..=4320).contains(&self.max_height) {
            let repaired = self.max_height.clamp(240, 4320);
            notes.push(format!(
                "smoothing.max-height must be 240 to 4320: {repaired} used"
            ));
            self.max_height = repaired;
        }
        if !SCENE_CHANGES.contains(&self.scene_change.as_str()) {
            notes.push(format!(
                "smoothing.scene-change {:?} is not one of {}: normal used",
                self.scene_change,
                SCENE_CHANGES.join(", ")
            ));
            "normal".clone_into(&mut self.scene_change);
        }
        if self.original_first > 30 {
            notes.push(format!(
                "smoothing.original-first must be 0 to 30 seconds: 30 used (was {})",
                self.original_first
            ));
            self.original_first = 30;
        }
        if self.drop_window != 0 && !(3..=600).contains(&self.drop_window) {
            let repaired = self.drop_window.clamp(3, 600);
            notes.push(format!(
                "smoothing.drop-window must be 0 or 3 to 600 seconds: {repaired} used"
            ));
            self.drop_window = repaired;
        }
    }
}

/// What the plan needs to know about the file and the machine.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Facts {
    /// The video's width and height (height 0: no video).
    pub width: u32,
    pub height: u32,
    /// The source's frame rate (0: unknown).
    pub source_fps: f64,
    /// The display's refresh rate (0: unknown).
    pub display_fps: f64,
    /// SVP's filter is in the chain.
    pub svp_attached: bool,
    /// SVP may still attach to the file (`svp::Watch::holds_smoothing`).
    pub svp_waiting: bool,
    /// RVP's VapourSynth script slot runs.
    pub script_slot: bool,
    pub engines: Engines,
    /// What keeps the file's rate by the calibration
    /// (`crate::calibration`; `None`: not calibrated, all may run).
    pub fitting: Option<crate::calibration::Fitting>,
}

/// The motion-compensated engines this machine has.
// One flag an engine: a set would hide which engine each test sets.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Engines {
    /// A VapourSynth runtime with MVTools.
    pub mvtools: bool,
    /// RIFE's plugin and model next to the runtime.
    pub rife: bool,
    /// libmpv's FFmpeg has `fruc_vulkan`.
    pub fruc: bool,
    /// RIFE (TensorRT) is installed, and the calibration built and
    /// measured its engine for this card.
    pub rife_trt: bool,
}

/// The GPU power budget's smoothing steps that are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lightened {
    /// One preset lighter (RIFE gives way to MVTools).
    pub lighter: bool,
    /// Blend instead of motion compensation.
    pub blend: bool,
    /// Off.
    pub off: bool,
    /// Steps taken because the output dropped frames while the file
    /// played (`App::smoothing_watch`): a lighter preset, then RIFE or
    /// `fruc_vulkan` give way to MVTools, then Blend.
    pub dropped: u8,
}

/// What runs for the file that plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Nothing; why, for the status.
    Off(&'static str),
    /// mpv's interpolation at the display's rate.
    Blend,
    /// RVP's filter: a motion-compensated engine, its preset and the rate
    /// it makes (a fraction).
    Filter {
        engine: Engine,
        preset: Preset,
        rate: (u64, u64),
    },
}

impl Plan {
    /// The status's text: `MVTools Medium, 59.94 fps`, `Blend`, `Off
    /// (SVP is attached)`.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Plan::Off("off") => "Off".to_owned(),
            Plan::Off(reason) => format!("Off ({reason})"),
            Plan::Blend => "Blend".to_owned(),
            Plan::Filter {
                engine,
                preset,
                rate: (num, den),
            } => {
                // Rates are small fractions, far inside f64's exact range.
                #[allow(clippy::cast_precision_loss)]
                let fps = *num as f64 / *den as f64;
                format!("{} {}, {fps:.2} fps", engine.text(), preset.text())
            }
        }
    }
}

/// A frame rate as a fraction: within 0.05 % of an NTSC rate (half the
/// distance to the whole number) that rate's /1001, near a whole number
/// that number, others to a thousandth.
#[must_use]
pub fn rational(fps: f64) -> (u64, u64) {
    for base in [24_000_u64, 30_000, 48_000, 60_000, 120_000, 144_000] {
        // Small whole numbers; exact in f64.
        #[allow(clippy::cast_precision_loss)]
        let ntsc = base as f64 / 1001.0;
        if (fps - ntsc).abs() < ntsc * 0.0005 {
            return (base, 1001);
        }
    }
    if (fps - fps.round()).abs() < 0.05 {
        // A frame rate is a small positive number.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return (fps.round().max(1.0) as u64, 1);
    }
    // As above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let thousandths = (fps * 1000.0).round().max(1.0) as u64;
    (thousandths, 1000)
}

/// The rate `target` asks for, as a fraction: the display's (twice the
/// source's while the display's is unknown), twice the source's, or a
/// number. `None` without the rates it needs.
#[must_use]
pub fn target_rate(target: &str, source_fps: f64, display_fps: f64) -> Option<(u64, u64)> {
    let double = || {
        (source_fps > 0.0)
            .then(|| rational(source_fps))
            .map(|(n, d)| (n * 2, d))
    };
    match target {
        "display" if display_fps > 1.0 => Some(rational(display_fps)),
        "display" | "2x" => double(),
        number => number
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .map(|n| (n, 1)),
    }
}

/// What runs, from the settings, the file and the machine, and the GPU
/// power budget's steps (the module's documentation).
#[must_use]
pub fn plan(settings: &SmoothingSettings, facts: &Facts, lightened: Lightened) -> Plan {
    if !settings.enabled {
        return Plan::Off("off");
    }
    if facts.height == 0 {
        return Plan::Off("no video");
    }
    if facts.svp_attached {
        return Plan::Off("SVP is attached");
    }
    if facts.svp_waiting {
        return Plan::Off("waiting for SVP");
    }
    if facts.script_slot {
        return Plan::Off("RVP's VapourSynth script runs");
    }
    if lightened.off {
        return Plan::Off("the GPU power budget");
    }
    let Some(rate) = target_rate(&settings.target, facts.source_fps, facts.display_fps) else {
        return Plan::Off("the frame rate is unknown");
    };
    // A small fraction of small numbers.
    #[allow(clippy::cast_precision_loss)]
    let target_fps = rate.0 as f64 / rate.1 as f64;
    if facts.source_fps > 0.0 && target_fps <= facts.source_fps * 1.05 {
        return Plan::Off("the video plays at that rate already");
    }
    if settings.engine == Engine::Blend || lightened.blend || facts.height > settings.max_height {
        return Plan::Blend;
    }
    let mut engine = match settings.engine {
        Engine::FrucVulkan if !facts.engines.fruc => Engine::MvTools,
        // RIFE on ncnn stands in while TensorRT's engine is not ready.
        Engine::RifeTrt if !facts.engines.rife_trt && facts.engines.rife => Engine::Rife,
        Engine::RifeTrt if !facts.engines.rife_trt => Engine::MvTools,
        Engine::Rife if !facts.engines.rife => Engine::MvTools,
        engine => engine,
    };
    let mut preset = settings.preset;
    // The calibration's start: the first step that keeps the rate with
    // room (each step that does not is passed over, here and below).
    let fits = |engine: Engine, preset: Preset| {
        facts
            .fitting
            .is_none_or(|fitting| fitting.fits(engine, preset))
    };
    let fit = |mut step: (Engine, Preset)| {
        while !fits(step.0, step.1) {
            step = step_down(step.0, step.1, settings.preset)?;
        }
        Some(step)
    };
    let Some(start) = fit((engine, preset)) else {
        return Plan::Blend;
    };
    (engine, preset) = start;
    if lightened.lighter {
        if engine.is_rife() {
            engine = Engine::MvTools;
        } else {
            preset = preset.lighter();
        }
        let Some(lighter) = fit((engine, preset)) else {
            return Plan::Blend;
        };
        (engine, preset) = lighter;
    }
    for _ in 0..lightened.dropped {
        let Some(lighter) = step_down(engine, preset, settings.preset).and_then(fit) else {
            return Plan::Blend;
        };
        (engine, preset) = lighter;
    }
    if engine == Engine::MvTools && !facts.engines.mvtools {
        return Plan::Blend;
    }
    Plan::Filter {
        engine,
        preset,
        rate,
    }
}

/// Whether the GPU budget's lighter step changes a filter of `engine` at
/// `preset`, as [`plan`] takes it: RIFE, on Vulkan or TensorRT, gives way
/// to MVTools, the others take a lighter preset down to Fast. With RIFE
/// (TensorRT) at Fast the budget took the step for none and went on to
/// Blend (the Linux audit's M11).
fn lighter_changes(engine: Engine, preset: Preset) -> bool {
    engine.is_rife() || preset != Preset::Fast
}

/// One step down the plan's ladder from `engine` at `preset`: a lighter
/// preset down to Fast, then MVTools at the `wanted` preset but at most
/// Medium (RIFE and `fruc_vulkan` give way), then nothing (Blend).
#[must_use]
fn step_down(engine: Engine, preset: Preset, wanted: Preset) -> Option<(Engine, Preset)> {
    if preset != Preset::Fast {
        Some((engine, preset.lighter()))
    } else if engine != Engine::MvTools {
        Some((Engine::MvTools, wanted.min(Preset::Medium)))
    } else {
        None
    }
}

/// The steps of dropped frames ([`Lightened::dropped`]) beyond
/// `lightened`'s that take the plan past `engine`, as [`plan`] takes them
/// (passing over what the calibration says cannot keep the rate); at most
/// eight.
#[must_use]
pub fn steps_past(
    settings: &SmoothingSettings,
    facts: &Facts,
    lightened: Lightened,
    engine: Engine,
) -> u8 {
    (1..=8)
        .find(|steps| {
            let further = Lightened {
                dropped: lightened.dropped.saturating_add(*steps),
                ..lightened
            };
            !matches!(
                plan(settings, facts, further),
                Plan::Filter { engine: running, .. } if running == engine
            )
        })
        .unwrap_or(8)
}

/// The engines' files a script loads.
#[derive(Debug, Clone, Copy, Default)]
pub struct Files<'a> {
    /// RIFE's ncnn plugin and model.
    pub rife: Option<(&'a Path, &'a Path)>,
    /// MVTools' plugin where the runtime does not load it itself.
    pub mvtools: Option<&'a Path>,
    /// RIFE (TensorRT): the install folder (`crate::tensorrt`: `vstrt.dll`
    /// and NVIDIA's libraries next to it) and vs-mlrt's `vsmlrt.py` in the
    /// package.
    pub tensorrt: Option<(&'a Path, &'a Path)>,
}

/// The shapes of RIFE (TensorRT)'s engine, as SVP builds its own: one
/// engine for every picture from 128x128 to [`TRT_LARGEST`], fastest at
/// 1920x1088.
pub const TRT_LARGEST: (u32, u32) = (2560, 1440);

/// RIFE v4.6 through vs-mlrt's TensorRT backend (fp16, frames padded to
/// multiples of 32 and cropped back, 1080 to 1088): `vsmlrt.RIFE` with the
/// rate's ratio, its engine in the install folder's `engines` (built by
/// `trtexec` the first time, two minutes on the host's RTX 30-series card); the
/// presets are its CUDA streams (1, 2 and 3).
fn rife_trt_script(
    text: &mut String,
    preset: Preset,
    source: (u64, u64),
    rate: (u64, u64),
    folder: &Path,
    scripts: &Path,
) {
    let streams = match preset {
        Preset::Fast => 1,
        Preset::Medium => 2,
        Preset::Quality => 3,
    };
    let _ = write!(
        text,
        "import sys\n\
         from fractions import Fraction\n\
         core.std.LoadPlugin({vstrt})\n\
         if {scripts} not in sys.path:\n    sys.path.insert(0, {scripts})\n\
         import vsmlrt\n\
         matrix = '709' if clip.height > 576 else '170m'\n\
         width, height = clip.width, clip.height\n\
         padded = ((width + 31) // 32 * 32, (height + 31) // 32 * 32)\n\
         rgb = core.resize.Bicubic(clip, format=vs.RGBH, matrix_in_s=matrix)\n\
         if padded != (width, height):\n    \
         rgb = core.std.AddBorders(rgb, right=padded[0] - width, bottom=padded[1] - height)\n\
         backend = vsmlrt.Backend.TRT(fp16=True, device_id=0, num_streams={streams}, \
         static_shape=False, min_shapes=(128, 128), opt_shapes=(1920, 1088), \
         max_shapes=({largest_w}, {largest_h}), output_format=1, engine_folder={engines})\n\
         rgb = vsmlrt.RIFE(rgb, multi=Fraction({num}, {den}), model=vsmlrt.RIFEModel.v4_6, \
         backend=backend, video_player=True)\n\
         if padded != (width, height):\n    \
         rgb = core.std.Crop(rgb, right=padded[0] - width, bottom=padded[1] - height)\n\
         clip = core.resize.Bicubic(rgb, format=clip.format.id, matrix_s=matrix)\n",
        vstrt = python_path(&folder.join(crate::tensorrt::VSTRT)),
        scripts = python_path(scripts),
        engines = python_path(&folder.join("engines")),
        largest_w = TRT_LARGEST.0,
        largest_h = TRT_LARGEST.1,
        num = rate.0 * source.1,
        den = rate.1 * source.0,
    );
}

/// [`script`]'s part for cuts ([`SCENE_CHANGES`]): the engine's frames
/// past a cut's last source frame, up to the next scene's first, are that
/// frame repeated; `AssumeFPS` gives them the rate's durations, which mpv's
/// timestamps follow.
fn cut_script(text: &mut String, source: (u64, u64), rate: (u64, u64), threshold: f64) {
    let _ = write!(
        text,
        "scene = core.std.PlaneStats(src, src[1:] + src[-1], plane=0, prop='RvpScene')\n\
         def held(n):\n    \
         return scene[min(n * {} // {}, scene.num_frames - 1)] * (n + 1)\n\
         cuts = core.std.FrameEval(clip, held)\n\
         smooth = clip\n\
         def at_cut(n, f):\n    \
         return cuts if f.props['RvpSceneDiff'] > {threshold} else smooth\n\
         clip = core.std.AssumeFPS(core.std.FrameEval(smooth, at_cut, prop_src=cuts), \
         fpsnum={}, fpsden={})\n",
        rate.1 * source.0,
        rate.0 * source.1,
        rate.0,
        rate.1,
    );
}

/// [`script`]'s part for the original first: its `first` frames are the
/// source's, repeated at the rate.
fn original_script(text: &mut String, source: (u64, u64), rate: (u64, u64), first: u64) {
    let _ = write!(
        text,
        "def original(n):\n    \
         return src[min(n * {} // {}, src.num_frames - 1)] * (n + 1)\n\
         shown = core.std.AssumeFPS(core.std.FrameEval(clip, original), \
         fpsnum={}, fpsden={})\n\
         clip = shown[:{first}] + clip[{first}:]\n",
        rate.1 * source.0,
        rate.0 * source.1,
        rate.0,
        rate.1,
    );
}

/// A Python string literal for a path.
fn python_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// The VapourSynth script of a motion-compensated engine that runs in
/// VapourSynth (MVTools, RIFE) for a source at `source` frames a second
/// (a fraction), making `rate`. `rife` names RIFE's plugin and model,
/// `mvtools` MVTools' plugin where the runtime does not load it itself
/// (RVP's Linux runtime: its compiled-in plugin folder is the build's).
/// `original` is [`SmoothingSettings::original_for`] the engine, `scene` the cut
/// threshold ([`scene_threshold`]).
#[must_use]
pub fn script(
    engine: Engine,
    preset: Preset,
    source: (u64, u64),
    rate: (u64, u64),
    files: &Files<'_>,
    original: u32,
    scene: Option<f64>,
) -> String {
    let (rife, mvtools) = (files.rife, files.mvtools);
    let mut text = format!(
        "# Generated by RVP (M18): motion smoothing.\n\
         import vapoursynth as vs\n\
         core = vs.core\n\
         clip = core.std.AssumeFPS(video_in, fpsnum={}, fpsden={})\n\
         src = clip\n",
        source.0, source.1
    );
    if let (Engine::RifeTrt, Some((folder, scripts))) = (engine, files.tensorrt) {
        rife_trt_script(&mut text, preset, source, rate, folder, scripts);
    } else if let (Engine::Rife, Some((plugin, model))) = (engine, rife) {
        let threads = match preset {
            Preset::Fast => 1,
            Preset::Medium => 2,
            Preset::Quality => 4,
        };
        let _ = write!(
            text,
            "core.std.LoadPlugin({plugin})\n\
                 matrix = '709' if clip.height > 576 else '170m'\n\
                 rgb = core.resize.Bicubic(clip, format=vs.RGBS, matrix_in_s=matrix)\n\
                 rgb = core.rife.RIFE(rgb, model_path={model}, fps_num={num}, fps_den={den}, \
                 gpu_thread={threads})\n\
                 clip = core.resize.Bicubic(rgb, format=clip.format.id, matrix_s=matrix)\n",
            plugin = python_path(plugin),
            model = python_path(model),
            num = rate.0,
            den = rate.1,
        );
    } else {
        let (pel, block, search, flow, recalculate) = match preset {
            Preset::Fast => (1, 32, 2, false, false),
            Preset::Medium => (2, 16, 3, true, false),
            Preset::Quality => (2, 16, 3, true, true),
        };
        if let Some(plugin) = mvtools {
            let _ = writeln!(
                text,
                "if not hasattr(core, 'mv'):\n    core.std.LoadPlugin({})",
                python_path(plugin)
            );
        }
        let _ = write!(
            text,
            "sup = core.mv.Super(clip, pel={pel}, hpad=16, vpad=16, sharp=2, rfilter=4)\n\
                 kw = dict(blksize={block}, overlap={half}, search={search}, truemotion=True)\n\
                 bv = core.mv.Analyse(sup, isb=True, **kw)\n\
                 fv = core.mv.Analyse(sup, isb=False, **kw)\n",
            half = block / 2,
        );
        if recalculate {
            text.push_str(
                "rkw = dict(blksize=8, overlap=4, search=3, truemotion=True)\n\
                     bv = core.mv.Recalculate(sup, bv, **rkw)\n\
                     fv = core.mv.Recalculate(sup, fv, **rkw)\n",
            );
        }
        let _ = if flow {
            writeln!(
                text,
                "clip = core.mv.FlowFPS(clip, sup, bv, fv, num={}, den={}, mask=2, ml=100)",
                rate.0, rate.1
            )
        } else {
            writeln!(
                text,
                "clip = core.mv.BlockFPS(clip, sup, bv, fv, num={}, den={}, mode=3, \
                     thscd1=400, thscd2=130)",
                rate.0, rate.1
            )
        };
    }
    // SVP's way (the owner, 2026-11-04): mpv loads the script again after
    // each start and seek, and for its first `original` seconds the output
    // is the source's frames, repeated at the rate, while the engine fills
    // its pipeline from the frames asked for ahead. Adding the filter later
    // instead dropped 80 frames as mpv's chain changed (the host, 1080p).
    // RIFE (TensorRT) gets none unless the page asks for it
    // (`SmoothingSettings::original_for`).
    if let Some(threshold) = scene {
        cut_script(&mut text, source, rate, threshold);
    }
    let first = rate.0 * u64::from(original) / rate.1.max(1);
    if first > 0 {
        original_script(&mut text, source, rate, first);
    }
    text.push_str(
        "# mpv answers frames asked for while the script loads with black ones;\n\
         # the cache would keep them as the first frames after a start or seek.\n\
         core.clear_cache()\n\
         clip.set_output()\n",
    );
    text
}

/// The `vf` entry of RVP's smoothing filter: VapourSynth on `script`
/// (mpv splits its arguments at `:`, so the path goes with its length), or
/// FFmpeg's `fruc_vulkan`.
#[must_use]
pub fn filter_entry(engine: Engine, preset: Preset, rate: (u64, u64), script: &Path) -> String {
    if engine == Engine::FrucVulkan {
        let perf = match preset {
            Preset::Fast => "fast",
            Preset::Medium => "medium",
            Preset::Quality => "slow",
        };
        // Decoded frames (copy-back, as RVP's other filters have them) go
        // up to the Vulkan device of mpv's output through mpv's own
        // conversion (a format filter asking for Vulkan frames), and the
        // frames it makes stay there for the output to show.
        let [nv12, vulkan] = FRUC_LABELS;
        return format!(
            "@{nv12}:format=fmt=nv12,@{vulkan}:format=fmt=vulkan,\
             @{LABEL}:lavfi=[fruc_vulkan=fps={}/{}:perf={perf}]:hwdec_interop=vulkan",
            rate.0, rate.1
        );
    }
    let path = script.to_string_lossy().replace('\\', "/");
    let entry = format!("@{LABEL}:vapoursynth=file=%{}%{path}", path.len());
    if engine.is_rife() {
        // RIFE keeps its GPU threads busy with the frames in flight.
        format!("{entry}:buffered-frames=8:concurrent-frames=8")
    } else {
        // MVTools spreads over the CPU's threads only as far as frames are
        // asked for at once. 4 at once left 1080p at 2.5x dropping about 48
        // of its 60 frames a second on the host (64 threads); mpv's default,
        // one per logical CPU, dropped one about every 64 frames (the
        // owner's log, 2026-11-03), more buffered frames dropped more. The
        // host's runs of 2026-11-04 (MVTools Quality, 1080p23.976 at 59.94):
        // 12 to 24 at once dropped none, 8 fell short of the rate, 32
        // dropped one in 48 s and 64 dropped 162.
        format!("{entry}:buffered-frames=4:concurrent-frames=16")
    }
}

/// RIFE's plugin and model in the VapourSynth runtime whose VSScript
/// library is `vsscript`: `rife/` next to it.
#[must_use]
pub fn rife_files(vsscript: &Path) -> Option<(PathBuf, PathBuf)> {
    let folder = vsscript.parent()?.join("rife");
    let plugin = folder.join(if cfg!(windows) {
        "librife_windows_x86-64.dll"
    } else {
        "librife_linux_x86-64.so"
    });
    let model = folder.join("models").join(RIFE_MODEL);
    (plugin.is_file() && model.join("flownet.param").is_file()).then_some((plugin, model))
}

/// MVTools' plugin in the `vs-plugins` of the runtime whose VSScript
/// library is `vsscript`, when it is there.
#[must_use]
pub fn mvtools_file(vsscript: &Path) -> Option<PathBuf> {
    let plugin = vsscript
        .parent()?
        .join("vs-plugins")
        .join(if cfg!(windows) {
            "libmvtools.dll"
        } else {
            "libmvtools.so"
        });
    plugin.is_file().then_some(plugin)
}

/// vs-mlrt's folder in the runtime of `vsscript` (`vs-mlrt` next to it),
/// when it has all `crate::tensorrt` needs from the package and
/// `vsmlrt.py`.
#[must_use]
pub fn tensorrt_package(vsscript: &Path) -> Option<PathBuf> {
    let folder = vsscript.parent()?.join("vs-mlrt");
    (folder.join("vsmlrt.py").is_file()
        && crate::tensorrt::PACKAGE_FILES
            .iter()
            .all(|(file, _)| folder.join(file).is_file()))
    .then_some(folder)
}

/// True when the runtime of `vsscript` has MVTools: RVP's own runtime
/// keeps it in `vs-plugins`; another runtime is trusted to have it.
#[must_use]
pub fn has_mvtools(vsscript: &Path) -> bool {
    let Some(folder) = vsscript.parent() else {
        return false;
    };
    let plugins = folder.join("vs-plugins");
    if !plugins.is_dir() {
        return true;
    }
    ["libmvtools.dll", "libmvtools.so"]
        .iter()
        .any(|name| plugins.join(name).is_file())
}

/// The VapourSynth runtime's engines, found once when RVP starts
/// (`App::prepare_svp`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Runtime {
    /// MVTools is there.
    mvtools: bool,
    /// MVTools' plugin in RVP's own runtime (`vs-plugins`).
    pub(crate) mvtools_plugin: Option<PathBuf>,
    /// RIFE's plugin and model.
    rife: Option<(PathBuf, PathBuf)>,
    /// vs-mlrt's files in the package (`vs-mlrt` next to VSScript): its
    /// TensorRT plugin, `trtexec`, `vsmlrt.py` and RIFE's ONNX model.
    tensorrt: Option<PathBuf>,
}

impl Runtime {
    /// What the runtime of `vsscript` has.
    pub(crate) fn probe(vsscript: Option<&Path>) -> Self {
        Self {
            mvtools: vsscript.is_some_and(has_mvtools),
            mvtools_plugin: vsscript.and_then(mvtools_file),
            rife: vsscript.and_then(rife_files),
            tensorrt: vsscript.and_then(tensorrt_package),
        }
    }

    /// The package's vs-mlrt folder (its `vsmlrt.py` and the files
    /// `crate::tensorrt` copies), when it is there.
    pub(crate) fn tensorrt_scripts(&self) -> Option<&Path> {
        self.tensorrt.as_deref()
    }

    /// MVTools is there.
    pub(crate) fn has_mvtools(&self) -> bool {
        self.mvtools
    }

    /// RIFE's plugin and model, when they are there.
    pub(crate) fn rife_files(&self) -> Option<(&Path, &Path)> {
        self.rife
            .as_ref()
            .map(|(plugin, model)| (plugin.as_path(), model.as_path()))
    }
}

/// The smoothing's state in the [`App`].
#[derive(Debug, Default)]
pub(crate) struct Smoothing {
    pub(crate) runtime: Runtime,
    /// libmpv's FFmpeg has `fruc_vulkan`.
    pub(crate) fruc: bool,
    /// The plan mpv got last.
    applied: Option<Plan>,
    /// Blend's options are set.
    blend: bool,
    /// What mpv got last for `hr-seek-framedrop` (`None`: the engine's
    /// "no", `App::apply_seek_framedrop`).
    framedrop: Option<bool>,
    /// The watch over the filter in force (`App::smoothing_watch`).
    watch: FilterWatch,
    /// The watch over a steady frame a second the output drops
    /// (`App::smoothing_watch`).
    lag: LagWatch,
    /// The plan waited for SVP when it was made (`App::drain` re-plans
    /// when that ends).
    pub(crate) svp_hold: bool,
    /// Steps the output's dropped frames took for this file
    /// ([`Lightened::dropped`]).
    dropped: u8,
    /// The engines whose filter raised the rate in this file: a dip after
    /// a seek or a lighter step leaves them in.
    ran: std::collections::BTreeSet<Engine>,
    /// Engines left out for the session that have no flag of their own.
    pub(crate) left_out: std::collections::BTreeSet<Engine>,
    /// The share of the processors other programs took over the last
    /// second (0 to 1; `App::set_processors`), and at the last step down.
    busy: Option<f64>,
    busy_at_step: Option<f64>,
    /// Once mpv holds each file before its first frame for the plan
    /// (`Engine::hold_preloaded`): the video track as the demuxer
    /// describes it while no frame came.
    held: Option<Preloaded>,
}

/// What mpv's hold before a file's first frame told
/// (`App::smoothing_preloaded`).
#[derive(Debug, Clone, Copy, Default)]
struct Preloaded {
    video: Option<DemuxVideo>,
}

impl Smoothing {
    /// mpv starts a file: what the demuxer said of the last one goes.
    pub(crate) fn start_file(&mut self) {
        if let Some(held) = &mut self.held {
            held.video = None;
        }
    }

    /// A file started: the drops of the last one count no longer.
    pub(crate) fn file_started(&mut self) {
        self.dropped = 0;
        self.ran.clear();
        self.watch = FilterWatch::default();
        self.lag = LagWatch::default();
    }
}

/// One second's readings for [`FilterWatch::tick`].
#[derive(Debug, Clone, Copy)]
struct Tick {
    now: Instant,
    playing: bool,
    /// The playback time and speed.
    time: f64,
    speed: f64,
    /// The file's rate and the rate mpv's filters put out.
    source_fps: f64,
    filter_fps: f64,
    /// Frames the output dropped so far.
    dropped: i64,
}

/// What the watch makes of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Wait,
    /// The filter never raised the rate: its engine did not run here.
    LeaveOut,
    /// The output dropped too many of the filter's frames.
    Lighter,
}

/// The watch over a motion-compensated filter since it went in: it has to
/// raise the rate within [`WATCH`] of steady playback, and after that the
/// output's dropped frames are counted over windows of the Motion
/// Smoothing page's length (`drop-window`), more than [`DROP_SHARE`] of
/// the frames making it lighter, and over windows of [`AT_ONCE`], where
/// more than [`AT_ONCE_SHARE`] makes it lighter at once (unless the page
/// never steps down). A seek, a loop or a pause starts the wait again and
/// leaves the windows unjudged; once the filter has raised the rate, a dip
/// after a seek is no failure (mpv's estimate starts again after it).
#[derive(Debug, Default)]
struct FilterWatch {
    /// Steady playback since.
    since: Option<Instant>,
    /// The filter has raised the rate.
    proven: bool,
    /// The last tick's time and playback time.
    last: Option<(Instant, f64)>,
    /// The drop window's start: when, the drops then, the playback time.
    drops_from: Option<(Instant, i64, f64)>,
    /// The short window's start ([`AT_ONCE`]), as `drops_from`.
    at_once_from: Option<(Instant, i64, f64)>,
    /// The window judged at the last tick (the log): the drop window's,
    /// or a short one that made the filter lighter at once.
    judged: Option<Judged>,
}

/// A window of dropped frames the watch judged.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Judged {
    /// The frames the output dropped and the frames the filter made.
    dropped: i64,
    frames: f64,
    /// The window's length.
    seconds: f64,
    /// More than the window's share: a lighter step.
    too_many: bool,
    /// A short window ([`AT_ONCE`]).
    at_once: bool,
}

impl FilterWatch {
    /// One second's readings; `window` is how long dropped frames are
    /// counted (`None`: they never make it lighter).
    fn tick(&mut self, tick: &Tick, window: Option<Duration>) -> Verdict {
        self.judged = None;
        if !tick.playing {
            self.since = None;
            self.drops_from = None;
            self.at_once_from = None;
            self.last = None;
            return Verdict::Wait;
        }
        if let Some((at, time)) = self.last {
            let seconds = tick.now.duration_since(at).as_secs_f64();
            if !played_on(time, tick.time, seconds, tick.speed) {
                self.since = None;
                self.drops_from = None;
                self.at_once_from = None;
            }
        }
        self.last = Some((tick.now, tick.time));
        let since = *self.since.get_or_insert(tick.now);
        if tick.now.duration_since(since) < WATCH || tick.source_fps <= 0.0 {
            return Verdict::Wait;
        }
        if tick.filter_fps <= tick.source_fps * 1.2 {
            return if self.proven {
                Verdict::Wait
            } else {
                Verdict::LeaveOut
            };
        }
        self.proven = true;
        let Some(window) = window else {
            return Verdict::Wait;
        };
        // An engine far beyond this machine does not wait for the drop
        // window (the owner, 2026-11-06).
        if let Some(judged) = judge(&mut self.at_once_from, tick, AT_ONCE, AT_ONCE_SHARE)
            && judged.too_many
        {
            self.judged = Some(Judged {
                at_once: true,
                ..judged
            });
            self.drops_from = None;
            return Verdict::Lighter;
        }
        let Some(judged) = judge(&mut self.drops_from, tick, window, DROP_SHARE) else {
            return Verdict::Wait;
        };
        self.judged = Some(judged);
        if judged.too_many {
            Verdict::Lighter
        } else {
            Verdict::Wait
        }
    }
}

/// The log's line for a window of dropped frames the watch judged.
fn log_judged(judged: Judged, before: &str) {
    crate::log::write(
        if judged.too_many {
            crate::log::Level::Warning
        } else {
            crate::log::Level::Debug
        },
        &format!(
            "smoothing: {before} dropped {} of {:.0} frames over {:.0} s ({})",
            judged.dropped,
            judged.frames,
            judged.seconds,
            if judged.at_once {
                "over half: a lighter step at once"
            } else if judged.too_many {
                "over 5 %: a lighter step"
            } else {
                "kept"
            }
        ),
    );
}

/// The watch over a filter that keeps up while the output stays a frame
/// behind the sound: after other programs had taken the processors, mpv
/// went on dropping a frame in each second until the next seek (the
/// owner's log of 2026-11-11: 57 s with RIFE (TensorRT), five and a half
/// minutes with MVTools). A seek to where playback is lets the filter's
/// frames come in time again.
#[derive(Debug, Default)]
struct LagWatch {
    /// The last tick: the drops so far, the playback time, when.
    last: Option<(i64, f64, Instant)>,
    /// Seconds in a row that dropped one to [`LAG_MOST`] frames.
    steady: u32,
    /// When playback last picked up again.
    resynced: Option<Instant>,
    /// The pattern came back soon after a seek: none again for the file.
    given_up: bool,
}

/// Seconds in a row of a few dropped frames before playback picks up
/// again, and the most a second may drop to count.
const LAG_SECONDS: u32 = 10;
const LAG_MOST: i64 = 3;
/// The pattern back this soon after a seek means seeks do not help here.
const LAG_AGAIN: Duration = Duration::from_secs(60);

/// What the lag watch makes of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lag {
    Wait,
    /// Seek to where playback is.
    PickUp,
    /// The pattern came back after the last seek: left for this file.
    GiveUp,
}

impl LagWatch {
    fn tick(&mut self, tick: &Tick) -> Lag {
        if self.given_up {
            return Lag::Wait;
        }
        let Some((dropped, time, at)) = self.last.replace((tick.dropped, tick.time, tick.now))
        else {
            return Lag::Wait;
        };
        let seconds = tick.now.duration_since(at).as_secs_f64();
        let new = tick.dropped - dropped;
        let steady = tick.playing
            && (tick.speed - 1.0).abs() < 1e-6
            && played_on(time, tick.time, seconds, tick.speed)
            && (1..=LAG_MOST).contains(&new)
            && tick.filter_fps > tick.source_fps * 1.2;
        if !steady {
            self.steady = 0;
            return Lag::Wait;
        }
        self.steady += 1;
        if self.steady < LAG_SECONDS {
            return Lag::Wait;
        }
        self.steady = 0;
        if self
            .resynced
            .is_some_and(|at| tick.now.duration_since(at) < LAG_AGAIN)
        {
            self.given_up = true;
            return Lag::GiveUp;
        }
        self.resynced = Some(tick.now);
        Lag::PickUp
    }
}

/// Once `length` has passed since `from` (a window's start: when, the
/// drops then, the playback time), counts the drops over it against
/// `share` of the filter's frames and starts the next window; `None`
/// while the window runs, and when a seek or a loop left it unjudged.
fn judge(
    from: &mut Option<(Instant, i64, f64)>,
    tick: &Tick,
    length: Duration,
    share: f64,
) -> Option<Judged> {
    let Some((at, at_dropped, at_time)) = *from else {
        *from = Some((tick.now, tick.dropped, tick.time));
        return None;
    };
    let seconds = tick.now.duration_since(at).as_secs_f64();
    if seconds < length.as_secs_f64() {
        return None;
    }
    *from = Some((tick.now, tick.dropped, tick.time));
    if tick.dropped < at_dropped || !played_on(at_time, tick.time, seconds, tick.speed) {
        return None;
    }
    let dropped = tick.dropped - at_dropped;
    Some(Judged {
        dropped,
        frames: tick.filter_fps * seconds,
        seconds,
        too_many: too_many_drops(dropped, tick.filter_fps, seconds, share),
        at_once: false,
    })
}

/// The share of the filter's frames that the output may drop over a drop
/// window before smoothing takes a lighter step: 5 % (the owner,
/// 2026-10-20).
const DROP_SHARE: f64 = 0.05;

/// The short window: more than [`AT_ONCE_SHARE`] of the frames dropped
/// over it make smoothing lighter at once, whatever the drop window (the
/// owner, 2026-11-06: RIFE at 1080p, which the host's card cannot keep up
/// with, otherwise dropped 40 to 50 frames a second for a minute's window
/// at each step).
const AT_ONCE: Duration = Duration::from_secs(5);

/// The share of the frames over [`AT_ONCE`] that makes smoothing lighter
/// at once: half.
const AT_ONCE_SHARE: f64 = 0.5;

/// Other programs' share of the processors at a step down that makes it
/// theirs, and the share under which their load is gone again
/// (`App::smoothing_seek`): the owner's WSL builds took 85 to 100 %.
const BUSY_STEP: f64 = 0.6;
const BUSY_GONE: f64 = 0.3;

/// Whether `dropped` frames over `seconds` of a filter that makes `rate`
/// frames a second are more than `share` of them.
#[must_use]
fn too_many_drops(dropped: i64, rate: f64, seconds: f64, share: f64) -> bool {
    // Frame counts of a few seconds.
    #[allow(clippy::cast_precision_loss)]
    let dropped = dropped as f64;
    dropped > share * rate * seconds
}

/// Whether playback went on from `from` to `to` (playback times) over
/// `seconds` at `speed` without a seek: forward, and not further than it
/// could play. A machine that cannot keep up plays less, which is no
/// reason to leave its drops unjudged.
#[must_use]
fn played_on(from: f64, to: f64, seconds: f64, speed: f64) -> bool {
    let advanced = to - from;
    (0.0..=seconds * speed + 1.0).contains(&advanced)
}

/// How long a motion-compensated filter plays before its rate is looked
/// at (`App::smoothing_watch`).
const WATCH: Duration = Duration::from_secs(6);

/// A command of Video > Motion Smoothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SmoothingCommand {
    /// On or off (Ctrl+Shift+I).
    Toggle,
    Engine(Engine),
    Preset(Preset),
    /// The display's rate (`true`) or twice the video's.
    DisplayRate(bool),
}

impl App {
    /// What the plan knows about the file that plays and this machine.
    fn smoothing_facts(&self) -> Facts {
        let vs = &self.settings.vapoursynth;
        let size = |id| {
            self.model
                .int(id)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0)
        };
        // The decoded size and rate once a frame came, the demuxer's before
        // (`App::smoothing_preloaded`).
        let preloaded = self.smoothing.held.and_then(|held| held.video);
        let decoded = (size(PropertyId::VideoWidth), size(PropertyId::VideoHeight));
        let (width, height) = match preloaded {
            Some(video) if decoded.0 == 0 || decoded.1 == 0 => (video.width, video.height),
            _ => decoded,
        };
        let source_fps = self
            .model
            .double(PropertyId::ContainerFps)
            .filter(|fps| *fps > 0.0)
            .or(preloaded.map(|video| video.fps))
            .unwrap_or(0.0);
        let display_fps = self.picture.screen.refresh;
        Facts {
            width,
            height,
            source_fps,
            display_fps,
            svp_attached: self
                .model
                .text(PropertyId::VideoFilters)
                .is_some_and(|vf| vf.contains("@svp")),
            svp_waiting: self.svp.holds_smoothing(),
            script_slot: vs.enabled && vs.script.is_some() && !self.gpu_step_taken(Step::Script),
            engines: Engines {
                mvtools: self.smoothing.runtime.mvtools,
                rife: self.smoothing.runtime.rife.is_some(),
                // Its frames stay on the Vulkan device of mpv's output.
                fruc: self.smoothing.fruc
                    && self.native_video
                    && self.settings.render.renderer == crate::render::Renderer::Vulkan,
                rife_trt: self.rife_trt_ready(),
            },
            fitting: self.calibration_fitting(width, height, source_fps, display_fps),
        }
    }

    /// The GPU power budget's smoothing steps that are taken.
    fn smoothing_lightened(&self) -> Lightened {
        Lightened {
            lighter: self.gpu_step_taken(Step::SmoothingLighter),
            blend: self.gpu_step_taken(Step::SmoothingBlend),
            off: self.gpu_step_taken(Step::SmoothingOff),
            dropped: self.smoothing.dropped,
        }
    }

    /// What runs for the file that plays.
    #[must_use]
    pub fn smoothing_plan(&self) -> Plan {
        plan(
            &self.settings.smoothing,
            &self.smoothing_facts(),
            self.smoothing_lightened(),
        )
    }

    /// The script mpv reads again at every start and seek: this process's
    /// own, as each RVP plays its own file.
    pub(crate) fn smoothing_script_path(&self) -> PathBuf {
        self.state_dir()
            .join("vapoursynth")
            .join(crate::storage::process_file_name("rvp-smooth", "vpy"))
    }

    /// RVP's smoothing filter while a motion-compensated engine runs
    /// (`App::video_filters`).
    pub(crate) fn smoothing_filter(&self) -> Option<(String, String)> {
        match self.smoothing_plan() {
            Plan::Filter {
                engine,
                preset,
                rate,
            } => Some((
                LABEL.to_owned(),
                filter_entry(engine, preset, rate, &self.smoothing_script_path()),
            )),
            Plan::Off(_) | Plan::Blend => None,
        }
    }

    /// Writes the VapourSynth script of the plan in force
    /// (`App::apply_video_filters`).
    pub(crate) fn write_smoothing_script(&self) -> Result<(), String> {
        let Plan::Filter {
            engine,
            preset,
            rate,
        } = self.smoothing_plan()
        else {
            return Ok(());
        };
        if engine == Engine::FrucVulkan {
            return Ok(());
        }
        let source = rational(
            self.model
                .double(PropertyId::ContainerFps)
                .filter(|fps| *fps > 0.0)
                .unwrap_or(24000.0 / 1001.0),
        );
        let folder = self.tensorrt_folder();
        let files = Files {
            rife: self.smoothing.runtime.rife_files(),
            mvtools: self.smoothing.runtime.mvtools_plugin.as_deref(),
            tensorrt: self
                .smoothing
                .runtime
                .tensorrt_scripts()
                .map(|scripts| (folder.as_path(), scripts)),
        };
        let path = self.smoothing_script_path();
        let smoothing = &self.settings.smoothing;
        let scene = scene_threshold(&smoothing.scene_change);
        crate::storage::write_unflushed(
            &path,
            script(
                engine,
                preset,
                source,
                rate,
                &files,
                smoothing.original_for(engine),
                scene,
            )
            .as_bytes(),
        )
        .map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Asks mpv to hold each file before its first frame while smoothing
    /// may run a filter (on, and not Blend), so the filter is in place as
    /// playback starts: added after it, it stopped mpv's core while the
    /// script loaded as the sound played on, and the video then dropped
    /// what it was behind (the host's runs of 2026-11-08: 44 to 62 frames
    /// at each start). Without smoothing the start does not wait for the
    /// window's thread, which is busy while RVP starts.
    pub(crate) fn hold_preloaded(&mut self, engine: &Mpv) {
        let smoothing = &self.settings.smoothing;
        if self.smoothing.held.is_some() || !smoothing.enabled || smoothing.engine == Engine::Blend
        {
            return;
        }
        match engine.hold_preloaded() {
            Ok(()) => self.smoothing.held = Some(Preloaded::default()),
            Err(error) => crate::log::report(
                crate::log::Level::Warning,
                &format!("smoothing: no filter before playback starts: {error}"),
            ),
        }
    }

    /// A file is open and mpv waits before its first frame: the plan from
    /// the demuxer's video track, then mpv goes on. The file starts here
    /// for SVP's wait and the drop watch, as it does again when it is
    /// loaded.
    pub(crate) fn smoothing_preloaded(
        &mut self,
        engine: &Mpv,
        hook: u64,
        video: Option<DemuxVideo>,
        now: Instant,
    ) {
        self.svp
            .file_started(crate::svp::svp_mode(&self.settings), now);
        self.smoothing.file_started();
        self.smoothing.held = Some(Preloaded { video });
        self.apply_smoothing(engine);
        if let Err(error) = engine.continue_preloaded(hook) {
            crate::log::report(crate::log::Level::Warning, &format!("smoothing: {error}"));
        }
    }

    /// mpv's `video-sync`: display-resample while Blend runs, else the
    /// setting.
    #[must_use]
    pub fn effective_video_sync(&self) -> &str {
        if self.smoothing.blend {
            "display-resample"
        } else {
            &self.settings.video_sync
        }
    }

    /// Gives mpv the plan in force when it changed: Blend's options and
    /// RVP's filter (with the decoder it needs). Returns the new plan.
    pub(crate) fn apply_smoothing(&mut self, engine: &Mpv) -> Option<Plan> {
        let plan = self.smoothing_plan();
        self.apply_seek_framedrop(engine, &plan);
        if self.smoothing.applied.as_ref() == Some(&plan) {
            return None;
        }
        let blend = plan == Plan::Blend;
        if blend != self.smoothing.blend {
            self.smoothing.blend = blend;
            drop(engine.set_text("interpolation", if blend { "yes" } else { "no" }));
            drop(engine.set_text("video-sync", self.effective_video_sync()));
            if blend {
                drop(engine.set_text("tscale", "oversample"));
            }
            // mpv knows the display's rate only when it owns the window.
            let refresh = self.picture.screen.refresh;
            let fps = if blend && !self.native_video && refresh > 1.0 {
                format!("{refresh:.3}")
            } else {
                "0".to_owned()
            };
            drop(engine.set_text("display-fps-override", &fps));
        }
        let filter_before = matches!(self.smoothing.applied, Some(Plan::Filter { .. }));
        let filter_now = matches!(plan, Plan::Filter { .. });
        crate::log::write(
            crate::log::Level::Info,
            &format!(
                "smoothing: {} -> {}",
                self.smoothing
                    .applied
                    .as_ref()
                    .map_or_else(|| "nothing yet".to_owned(), Plan::text),
                plan.text()
            ),
        );
        self.smoothing.applied = Some(plan.clone());
        self.smoothing.watch = FilterWatch::default();
        if filter_before || filter_now {
            self.apply_decoding(engine);
            drop(self.apply_video_filters(engine));
        }
        Some(plan)
    }

    /// A motion-compensated engine's filter is what mpv got last.
    pub(crate) fn smoothing_filter_in_force(&self) -> bool {
        matches!(self.smoothing.applied, Some(Plan::Filter { .. }))
    }

    /// The lag watch's tick ([`LagWatch`]): a seek to where playback is,
    /// once the output dropped a frame in each second for a while.
    fn smoothing_lag(&mut self, engine: &Mpv, tick: &Tick, before: &str) {
        match self.smoothing.lag.tick(tick) {
            Lag::Wait => {}
            Lag::PickUp => {
                crate::log::write(
                    crate::log::Level::Info,
                    &format!(
                        "smoothing: {before} kept up, but the output dropped a frame in each \
                         second for {LAG_SECONDS} s: playback picks up from here"
                    ),
                );
                self.resync_audio(engine);
            }
            Lag::GiveUp => crate::log::write(
                crate::log::Level::Info,
                &format!(
                    "smoothing: {before}: the dropped frames came back after the seek; \
                     left as they are for this file"
                ),
            ),
        }
    }

    /// Exact seeks skip the frames before the target, before the video
    /// filters, while the Playback page allows it
    /// (`Playback::seek_skip_before_target`) and SVP's filter cannot be
    /// there: RVP's smoothing filter runs (which the plan never runs beside
    /// SVP's), or SVP mode is off. Every frame from the keyframe on went
    /// through the engine, up to 4.5 s with RIFE. The engine starts mpv
    /// with "no" for SVP's filter, which needs them.
    pub(crate) fn apply_seek_framedrop(&mut self, engine: &Mpv, plan: &Plan) {
        let skip = self.settings.playback.seek_skip_before_target
            && (matches!(plan, Plan::Filter { .. }) || !crate::svp::svp_mode(&self.settings));
        if self.smoothing.framedrop.unwrap_or(false) != skip {
            self.smoothing.framedrop = Some(skip);
            drop(engine.set_text("hr-seek-framedrop", if skip { "yes" } else { "no" }));
        }
    }

    /// Video > Motion Smoothing's commands; the notice says what runs.
    pub(crate) fn smoothing_command(
        &mut self,
        engine: &Mpv,
        command: SmoothingCommand,
    ) -> UiRequest {
        let smoothing = &mut self.settings.smoothing;
        match command {
            SmoothingCommand::Toggle => smoothing.enabled = !smoothing.enabled,
            SmoothingCommand::Engine(chosen) => {
                smoothing.engine = chosen;
                smoothing.enabled = true;
            }
            SmoothingCommand::Preset(preset) => smoothing.preset = preset,
            SmoothingCommand::DisplayRate(display) => {
                let target = if display { "display" } else { "2x" };
                target.clone_into(&mut smoothing.target);
            }
        }
        self.save_settings();
        self.hold_preloaded(engine);
        self.apply_smoothing(engine);
        UiRequest::Osd(format!(
            "Motion smoothing: {}",
            self.smoothing_plan().text()
        ))
    }

    /// The check mark of a Motion Smoothing item.
    pub(crate) fn smoothing_checked(&self, command: SmoothingCommand) -> bool {
        let smoothing = &self.settings.smoothing;
        match command {
            SmoothingCommand::Toggle => smoothing.enabled,
            SmoothingCommand::Engine(engine) => smoothing.engine == engine,
            SmoothingCommand::Preset(preset) => smoothing.preset == preset,
            SmoothingCommand::DisplayRate(display) => (smoothing.target == "display") == display,
        }
    }

    /// Looks once, while nothing plays, whether libmpv's FFmpeg has
    /// `fruc_vulkan`: mpv takes libavfilter's filters by name and refuses
    /// unknown ones when they are added, before any frame.
    pub(crate) fn probe_fruc(&mut self, engine: &Mpv) {
        if !self.settings.render.renderer.native_window() {
            return;
        }
        self.smoothing.fruc = engine
            .command("vf", &["add", "@rvpprobe:lavfi-fruc_vulkan"])
            .is_ok();
        if self.smoothing.fruc {
            drop(engine.command("vf", &["remove", "@rvpprobe"]));
        }
    }

    /// Once a second, the watch over the filter in force
    /// ([`FilterWatch`]): an engine whose filter never raised the rate did
    /// not start here (RIFE or `fruc_vulkan` without their GPU, MVTools
    /// without its plugin; mpv drops a filter that fails) and is left out
    /// for the session; one that leaves the output dropping more than
    /// [`DROP_SHARE`] of its frames over the drop window, or more than
    /// [`AT_ONCE_SHARE`] over [`AT_ONCE`], takes one step lighter for the
    /// rest of the file. Either way the next plan runs, with a notice. A
    /// filter that keeps up while the output drops a frame in each second
    /// gets a seek to where playback is ([`LagWatch`]).
    pub fn smoothing_watch(&mut self, engine: &Mpv, now: Instant) -> Option<UiRequest> {
        let Some(Plan::Filter {
            engine: running, ..
        }) = self.smoothing.applied.clone()
        else {
            self.smoothing.watch = FilterWatch::default();
            self.smoothing.lag.last = None;
            self.smoothing.lag.steady = 0;
            return None;
        };
        let tick = Tick {
            now,
            playing: self.model.playing(),
            time: self.model.double(PropertyId::TimePos).unwrap_or(0.0),
            speed: self.model.double(PropertyId::Speed).unwrap_or(1.0),
            source_fps: self.model.double(PropertyId::ContainerFps).unwrap_or(0.0),
            filter_fps: self.model.double(PropertyId::EstimatedVfFps).unwrap_or(0.0),
            dropped: self.model.int(PropertyId::FrameDropCount).unwrap_or(0),
        };
        let before = self.smoothing_plan().text();
        self.smoothing_lag(engine, &tick, &before);
        let window = match self.settings.smoothing.drop_window {
            0 => None,
            seconds => Some(Duration::from_secs(u64::from(seconds))),
        };
        let verdict = self.smoothing.watch.tick(&tick, window);
        if self.smoothing.watch.proven {
            self.smoothing.ran.insert(running);
        }
        if let Some(judged) = self.smoothing.watch.judged {
            log_judged(judged, &before);
        }
        match verdict {
            Verdict::Wait => return None,
            Verdict::Lighter => {
                let at_once = self
                    .smoothing
                    .watch
                    .judged
                    .is_some_and(|judged| judged.at_once);
                // RIFE's presets change only its GPU threads (the host's
                // RIFE at 1080p dropped as many frames at Medium as at
                // Quality): one far beyond the card gives way to MVTools.
                let steps = if at_once && running.is_rife() {
                    steps_past(
                        &self.settings.smoothing,
                        &self.smoothing_facts(),
                        self.smoothing_lightened(),
                        running,
                    )
                } else {
                    1
                };
                self.smoothing.dropped = self.smoothing.dropped.saturating_add(steps);
                self.smoothing.busy_at_step = self.smoothing.busy;
                self.apply_smoothing(engine);
                return Some(UiRequest::Osd(format!(
                    "Motion smoothing: {before} dropped frames here; {}",
                    self.smoothing_plan().text()
                )));
            }
            Verdict::LeaveOut if self.smoothing.ran.contains(&running) => return None,
            Verdict::LeaveOut => {}
        }
        match running {
            Engine::FrucVulkan => self.smoothing.fruc = false,
            Engine::Rife => self.smoothing.runtime.rife = None,
            Engine::RifeTrt => {
                self.smoothing.left_out.insert(Engine::RifeTrt);
            }
            Engine::MvTools => self.smoothing.runtime.mvtools = false,
            Engine::Blend => return None,
        }
        crate::log::write(
            crate::log::Level::Warning,
            &format!(
                "smoothing: {} did not raise the rate ({:.3} fps in, {:.3} out): left out for the session",
                running.text(),
                tick.source_fps,
                tick.filter_fps
            ),
        );
        self.apply_smoothing(engine);
        Some(UiRequest::Osd(format!(
            "Motion smoothing: {} did not run here; {}",
            running.text(),
            self.smoothing_plan().text()
        )))
    }

    /// The processors' load over the last second, from the shell: all of
    /// it and RVP's share (0 to 1).
    pub fn set_processors(&mut self, processors: Option<(f64, f64)>) {
        self.smoothing.busy = processors.map(|(all, own)| (all - own).max(0.0));
    }

    /// A seek started: steps down that other programs' load took come back
    /// once that load is gone, while mpv restarts the filter anyway (the
    /// owner, 2026-11-07); steps the engine took on its own stay.
    pub(crate) fn smoothing_seek(&mut self, engine: &Mpv) {
        let smoothing = &self.smoothing;
        let (Some(then), Some(now)) = (smoothing.busy_at_step, smoothing.busy) else {
            return;
        };
        if smoothing.dropped == 0 || then < BUSY_STEP || now > BUSY_GONE {
            return;
        }
        crate::log::write(
            crate::log::Level::Info,
            &format!(
                "smoothing: other programs' load is gone ({:.0} % of the processors, {:.0} % at the step): the steps down come back at the seek",
                now * 100.0,
                then * 100.0
            ),
        );
        self.smoothing.dropped = 0;
        self.smoothing.busy_at_step = None;
        self.apply_smoothing(engine);
    }

    /// Which engines this machine has, for the settings window.
    #[must_use]
    pub fn smoothing_engines(&self) -> serde_json::Value {
        serde_json::json!({
            "mvtools": self.smoothing.runtime.mvtools,
            "rife": self.smoothing.runtime.rife.is_some(),
            "fruc": self.smoothing.fruc,
            "rife-trt": self.rife_trt_ready(),
        })
    }

    /// True when taking a smoothing step of the GPU power budget now
    /// would change what runs.
    pub(crate) fn smoothing_step_changes(&self, step: Step) -> bool {
        match (step, self.smoothing_plan()) {
            (Step::SmoothingLighter, Plan::Filter { engine, preset, .. }) => {
                lighter_changes(engine, preset)
            }
            (Step::SmoothingBlend, Plan::Filter { .. })
            | (Step::SmoothingOff, Plan::Filter { .. } | Plan::Blend) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lighter_step_takes_both_rifes_to_mvtools() {
        for engine in [Engine::Rife, Engine::RifeTrt] {
            assert!(lighter_changes(engine, Preset::Fast), "{engine:?}");
        }
        assert!(!lighter_changes(Engine::MvTools, Preset::Fast));
        assert!(lighter_changes(Engine::MvTools, Preset::Medium));
    }

    fn facts() -> Facts {
        Facts {
            height: 1080,
            source_fps: 24000.0 / 1001.0,
            display_fps: 60.0,
            engines: Engines {
                mvtools: true,
                rife: true,
                fruc: true,
                rife_trt: false,
            },
            ..Facts::default()
        }
    }

    fn on(engine: Engine) -> SmoothingSettings {
        SmoothingSettings {
            enabled: true,
            engine,
            ..SmoothingSettings::default()
        }
    }

    #[test]
    fn rates_become_fractions() {
        assert_eq!(rational(23.976), (24000, 1001));
        assert_eq!(rational(24000.0 / 1001.0), (24000, 1001));
        assert_eq!(rational(59.94), (60000, 1001));
        assert_eq!(rational(59.951), (60000, 1001));
        assert_eq!(rational(60.0), (60, 1));
        assert_eq!(rational(143.98), (144, 1));
        assert_eq!(rational(25.0), (25, 1));
        assert_eq!(rational(75.2), (75200, 1000));
        assert_eq!(target_rate("display", 23.976, 60.0), Some((60, 1)));
        assert_eq!(target_rate("display", 23.976, 0.0), Some((48000, 1001)));
        assert_eq!(target_rate("2x", 25.0, 60.0), Some((50, 1)));
        assert_eq!(target_rate("2x", 0.0, 60.0), None);
        assert_eq!(target_rate("120", 0.0, 0.0), Some((120, 1)));
    }

    #[test]
    fn the_plan_follows_the_owners_defaults_and_gives_way() {
        // Off by default.
        assert_eq!(
            plan(
                &SmoothingSettings::default(),
                &facts(),
                Lightened::default()
            ),
            Plan::Off("off")
        );
        // On: MVTools medium at the display's rate up to 1080p, Blend above.
        let settings = on(Engine::MvTools);
        assert_eq!(
            plan(&settings, &facts(), Lightened::default()),
            Plan::Filter {
                engine: Engine::MvTools,
                preset: Preset::Medium,
                rate: (60, 1)
            }
        );
        let tall = Facts {
            height: 2160,
            ..facts()
        };
        assert_eq!(plan(&settings, &tall, Lightened::default()), Plan::Blend);
        // SVP attached, RVP's own script, or nothing to gain: off.
        for (changed, reason) in [
            (
                Facts {
                    svp_attached: true,
                    ..facts()
                },
                "SVP is attached",
            ),
            (
                Facts {
                    svp_waiting: true,
                    ..facts()
                },
                "waiting for SVP",
            ),
            (
                Facts {
                    script_slot: true,
                    ..facts()
                },
                "RVP's VapourSynth script runs",
            ),
            (
                Facts {
                    source_fps: 59.94,
                    ..facts()
                },
                "the video plays at that rate already",
            ),
            (
                Facts {
                    height: 0,
                    ..facts()
                },
                "no video",
            ),
        ] {
            assert_eq!(
                plan(&settings, &changed, Lightened::default()),
                Plan::Off(reason)
            );
        }
        // Engines that cannot run give way.
        let bare = Facts {
            engines: Engines {
                mvtools: true,
                rife: false,
                fruc: false,
                rife_trt: false,
            },
            ..facts()
        };
        for engine in [Engine::Rife, Engine::FrucVulkan] {
            assert!(matches!(
                plan(&on(engine), &bare, Lightened::default()),
                Plan::Filter {
                    engine: Engine::MvTools,
                    ..
                }
            ));
        }
        let none = Facts {
            engines: Engines::default(),
            ..bare
        };
        assert_eq!(
            plan(&on(Engine::Rife), &none, Lightened::default()),
            Plan::Blend
        );
        assert_eq!(
            plan(&on(Engine::Blend), &facts(), Lightened::default()),
            Plan::Blend
        );
    }

    #[test]
    fn the_budgets_steps_lighten_then_blend_then_stop() {
        let quality = SmoothingSettings {
            preset: Preset::Quality,
            ..on(Engine::MvTools)
        };
        let lighter = Lightened {
            lighter: true,
            ..Lightened::default()
        };
        assert!(matches!(
            plan(&quality, &facts(), lighter),
            Plan::Filter {
                preset: Preset::Medium,
                ..
            }
        ));
        assert!(matches!(
            plan(&on(Engine::Rife), &facts(), lighter),
            Plan::Filter {
                engine: Engine::MvTools,
                preset: Preset::Medium,
                ..
            }
        ));
        let blend = Lightened {
            blend: true,
            ..lighter
        };
        assert_eq!(plan(&quality, &facts(), blend), Plan::Blend);
        let off = Lightened { off: true, ..blend };
        assert_eq!(
            plan(&quality, &facts(), off),
            Plan::Off("the GPU power budget")
        );
    }

    #[test]
    fn dropped_frames_lighten_the_preset_then_the_engine_then_blend() {
        let quality = |engine| SmoothingSettings {
            preset: Preset::Quality,
            ..on(engine)
        };
        let dropped = |steps| Lightened {
            dropped: steps,
            ..Lightened::default()
        };
        let machine = Facts {
            engines: Engines {
                mvtools: true,
                rife: true,
                fruc: true,
                rife_trt: false,
            },
            ..facts()
        };
        let filter = |engine, preset| Plan::Filter {
            engine,
            preset,
            rate: (60, 1),
        };
        // A RIFE far beyond the card gives way to MVTools at once from
        // the preset it runs at.
        for (taken, running) in [(0, Preset::Quality), (1, Preset::Medium), (2, Preset::Fast)] {
            assert_eq!(
                plan(&quality(Engine::Rife), &machine, dropped(taken)),
                filter(Engine::Rife, running)
            );
            let past = steps_past(
                &quality(Engine::Rife),
                &machine,
                dropped(taken),
                Engine::Rife,
            );
            assert_eq!(
                plan(&quality(Engine::Rife), &machine, dropped(taken + past)),
                filter(Engine::MvTools, Preset::Medium)
            );
        }
        for gpu in [Engine::Rife, Engine::FrucVulkan] {
            let steps: Vec<Plan> = (0..6)
                .map(|n| plan(&quality(gpu), &machine, dropped(n)))
                .collect();
            assert_eq!(
                steps,
                [
                    filter(gpu, Preset::Quality),
                    filter(gpu, Preset::Medium),
                    filter(gpu, Preset::Fast),
                    filter(Engine::MvTools, Preset::Medium),
                    filter(Engine::MvTools, Preset::Fast),
                    Plan::Blend,
                ]
            );
        }
        assert_eq!(
            plan(&on(Engine::MvTools), &machine, dropped(1)),
            filter(Engine::MvTools, Preset::Fast)
        );
        assert_eq!(
            plan(&on(Engine::MvTools), &machine, dropped(2)),
            Plan::Blend
        );
        // 5 % of 60 frames a second over 6 s: 18 frames.
        assert!(!too_many_drops(18, 60.0, 6.0, DROP_SHARE));
        assert!(too_many_drops(19, 60.0, 6.0, DROP_SHARE));
        assert!(!too_many_drops(150, 60.0, 5.0, AT_ONCE_SHARE));
        assert!(too_many_drops(151, 60.0, 5.0, AT_ONCE_SHARE));
        // A seek between leaves the drops unjudged; a machine that plays
        // slower than real time is judged.
        assert!(played_on(10.0, 16.1, 6.0, 1.0));
        assert!(played_on(10.0, 13.0, 6.0, 1.0));
        assert!(played_on(10.0, 19.0, 6.0, 1.5));
        assert!(!played_on(10.0, 40.0, 6.0, 1.0));
        assert!(!played_on(10.0, 5.0, 6.0, 1.0));
    }

    /// Ticks once a second from `start`: playback time `time(n)`, the
    /// filter's rate `fps(n)` and `dropped(n)` frames dropped so far, for
    /// the seconds `seconds`; the verdicts that are not waits.
    fn ticks(
        watch: &mut FilterWatch,
        window: Option<u64>,
        seconds: std::ops::Range<u64>,
        time: impl Fn(u64) -> f64,
        fps: impl Fn(u64) -> f64,
        dropped: impl Fn(u64) -> i64,
    ) -> Vec<(u64, Verdict)> {
        let start = Instant::now();
        seconds
            .filter_map(|n| {
                let tick = Tick {
                    now: start + Duration::from_secs(n),
                    playing: true,
                    time: time(n),
                    speed: 1.0,
                    source_fps: 24.0,
                    filter_fps: fps(n),
                    dropped: dropped(n),
                };
                let verdict = watch.tick(&tick, window.map(Duration::from_secs));
                (verdict != Verdict::Wait).then_some((n, verdict))
            })
            .collect()
    }

    #[test]
    fn the_filter_watch_leaves_out_lightens_and_forgives_seeks() {
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let steady = |n: u64| n as f64;
        // A filter that never raises the rate is left out after 6 s.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(&mut watch, Some(6), 0..8, steady, |_| 24.0, |_| 0),
            [(6, Verdict::LeaveOut), (7, Verdict::LeaveOut)]
        );
        // One that raised it is not, when a loop starts mpv's estimate
        // again (the host, 2026-10-20: MVTools went at a clip's end).
        let mut watch = FilterWatch::default();
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let looping = |n: u64| (n % 10) as f64;
        let low_after_loop = |n: u64| if n % 10 < 2 { 24.0 } else { 60.0 };
        assert_eq!(
            ticks(&mut watch, Some(6), 0..30, looping, low_after_loop, |_| 0),
            []
        );
        // A seek during the first 6 s starts the wait again.
        let mut watch = FilterWatch::default();
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let seek_at_3 = |n: u64| if n < 3 { n as f64 } else { 100.0 + n as f64 };
        assert_eq!(
            ticks(&mut watch, Some(6), 0..10, seek_at_3, |_| 24.0, |_| 0),
            [(9, Verdict::LeaveOut)]
        );
        // 10 dropped frames a second at 60: lighter after 6 s and a window.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(
                &mut watch,
                Some(6),
                0..13,
                steady,
                |_| 60.0,
                |n| 10 * i64::try_from(n).unwrap_or(0)
            ),
            [(12, Verdict::Lighter)]
        );
        // A slow machine plays less than real time and is still judged.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(
                &mut watch,
                Some(6),
                0..13,
                |n| 0.6 * steady(n),
                |_| 60.0,
                |n| 10 * i64::try_from(n).unwrap_or(0)
            ),
            [(12, Verdict::Lighter)]
        );
        // A loop inside a window leaves it unjudged; the next one counts.
        let mut watch = FilterWatch::default();
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let loop_at_9 = |n: u64| if n < 9 { n as f64 } else { (n - 9) as f64 };
        assert_eq!(
            ticks(
                &mut watch,
                Some(6),
                0..22,
                loop_at_9,
                |_| 60.0,
                |n| 10 * i64::try_from(n).unwrap_or(0)
            ),
            [(21, Verdict::Lighter)]
        );
    }

    #[test]
    fn a_hopeless_engine_steps_down_at_once() {
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let steady = |n: u64| n as f64;
        let per_second = |frames: i64| move |n: u64| frames * i64::try_from(n).unwrap_or(0);
        // RIFE at 1080p on the host: 45 of 60 frames a second dropped.
        // With a minute's drop window it steps down 5 s after it proved
        // itself, not a minute later.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(
                &mut watch,
                Some(60),
                0..12,
                steady,
                |_| 60.0,
                per_second(45)
            ),
            [(11, Verdict::Lighter)]
        );
        let judged = watch.judged.expect("judged");
        assert!(judged.at_once && judged.too_many);
        assert_eq!((judged.dropped, judged.seconds), (225, 5.0));
        // A third of the frames waits for the drop window.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(
                &mut watch,
                Some(60),
                0..67,
                steady,
                |_| 60.0,
                per_second(20)
            ),
            [(66, Verdict::Lighter)]
        );
        assert!(!watch.judged.expect("judged").at_once);
        // "Never" never steps down, however many frames drop.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(&mut watch, None, 0..70, steady, |_| 60.0, per_second(45)),
            []
        );
        // A seek inside the short window leaves it unjudged: the wait
        // starts again at the seek, and the next short window counts.
        let mut watch = FilterWatch::default();
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let seek_at_9 = |n: u64| if n < 9 { n as f64 } else { 100.0 + n as f64 };
        assert_eq!(
            ticks(
                &mut watch,
                Some(60),
                0..21,
                seek_at_9,
                |_| 60.0,
                per_second(45)
            ),
            [(20, Verdict::Lighter)]
        );
    }

    #[test]
    fn the_original_frames_come_first_after_each_start() {
        // A second at 60 frames a second of the 24000/1001 source: output
        // frame n shows source frame n * 1 * 24000 / (60 * 1001).
        let first = script(
            Engine::MvTools,
            Preset::Medium,
            (24000, 1001),
            (60, 1),
            &Files::default(),
            1,
            None,
        );
        assert!(first.contains("src = clip\n"));
        assert!(
            first.contains("return src[min(n * 24000 // 60060, src.num_frames - 1)] * (n + 1)")
        );
        assert!(first.contains("fpsnum=60, fpsden=1)\nclip = shown[:60] + clip[60:]\n"));
        assert!(first.ends_with("core.clear_cache()\nclip.set_output()\n"));
        let at_once = script(
            Engine::MvTools,
            Preset::Medium,
            (24000, 1001),
            (60, 1),
            &Files::default(),
            0,
            None,
        );
        assert!(!at_once.contains("FrameEval"));
        // RIFE (TensorRT) starts with its own frames unless the page asks
        // for the original first.
        let mut settings = on(Engine::RifeTrt);
        assert_eq!(settings.original_for(Engine::RifeTrt), 0);
        assert_eq!(settings.original_for(Engine::MvTools), 1);
        settings.original_first_tensorrt = true;
        assert_eq!(settings.original_for(Engine::RifeTrt), 1);
        let tensorrt = |original| {
            script(
                Engine::RifeTrt,
                Preset::Medium,
                (24000, 1001),
                (60, 1),
                &Files {
                    tensorrt: Some((Path::new("/trt"), Path::new("/vs-mlrt"))),
                    ..Files::default()
                },
                original,
                None,
            )
        };
        assert!(tensorrt(0).contains("vsmlrt.RIFE(") && !tensorrt(0).contains("FrameEval"));
        assert!(tensorrt(1).contains("vsmlrt.RIFE(") && tensorrt(1).contains("FrameEval"));
        let mut settings = on(Engine::MvTools);
        settings.original_first = 90;
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!((settings.original_first, notes.len()), (30, 1));
    }

    #[test]
    fn a_frame_dropped_each_second_picks_playback_up_once_a_minute_at_most() {
        let start = Instant::now();
        let tick = |n: u64, time: f64, dropped: i64| Tick {
            now: start + Duration::from_secs(n),
            playing: true,
            time,
            speed: 1.0,
            source_fps: 23.976,
            filter_fps: 59.94,
            dropped,
        };
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let run = |watch: &mut LagWatch, seconds: std::ops::Range<u64>, per_second: i64| {
            seconds
                .filter_map(|n| {
                    let lag = watch.tick(&tick(
                        n,
                        n as f64,
                        i64::try_from(n).unwrap_or(0) * per_second,
                    ));
                    (lag != Lag::Wait).then_some((n, lag))
                })
                .collect::<Vec<_>>()
        };
        // A frame each second: after 10 s in a row, a seek; the pattern back
        // within a minute: left for the file.
        let mut watch = LagWatch::default();
        assert_eq!(
            run(&mut watch, 0..40, 1),
            [(10, Lag::PickUp), (20, Lag::GiveUp)]
        );
        assert!(run(&mut watch, 40..100, 1).is_empty());
        // Back after more than a minute: another seek.
        let mut watch = LagWatch::default();
        assert_eq!(run(&mut watch, 0..11, 1), [(10, Lag::PickUp)]);
        assert!(run(&mut watch, 11..12, 0).is_empty());
        assert_eq!(run(&mut watch, 80..91, 1), [(90, Lag::PickUp)]);
        // Smooth playback, or a burst the steps judge, is left alone.
        let mut watch = LagWatch::default();
        assert!(run(&mut watch, 0..60, 0).is_empty());
        assert!(run(&mut watch, 60..120, 20).is_empty());
        // A seek in between starts the count again: 7 steady seconds, a
        // jump, then 9 more are not 10 in a row; the 10th is.
        let mut watch = LagWatch::default();
        assert!(run(&mut watch, 0..8, 1).is_empty());
        let after_seek = |n: u64| {
            tick(
                n,
                300.0 + f64::from(u32::try_from(n).unwrap_or(0)),
                i64::try_from(n).unwrap_or(0),
            )
        };
        assert_eq!(watch.tick(&after_seek(8)), Lag::Wait);
        for n in 9..18 {
            assert_eq!(watch.tick(&after_seek(n)), Lag::Wait, "second {n}");
        }
        assert_eq!(watch.tick(&after_seek(18)), Lag::PickUp);
        // A pause between: not playing breaks the count too.
        let mut watch = LagWatch::default();
        assert!(run(&mut watch, 0..8, 1).is_empty());
        assert_eq!(
            watch.tick(&Tick {
                playing: false,
                ..tick(8, 8.0, 8)
            }),
            Lag::Wait
        );
        assert_eq!(watch.steady, 0);
        // A filter that does not keep up is the steps' (not a seek's).
        let mut watch = LagWatch::default();
        for n in 0..30 {
            let slow = Tick {
                filter_fps: 24.0,
                ..tick(
                    n,
                    f64::from(u32::try_from(n).unwrap_or(0)),
                    i64::try_from(n).unwrap_or(0),
                )
            };
            assert_eq!(watch.tick(&slow), Lag::Wait);
        }
    }

    #[test]
    fn a_longer_drop_window_forgives_a_moments_stutter() {
        #[allow(clippy::cast_precision_loss)] // seconds of a test
        let steady = |n: u64| n as f64;
        // 40 frames dropped in second 10 (two thirds of a second at 60):
        // over 5 % of a 6 s window, under 5 % of a 20 s one (60 frames).
        let burst = |n: u64| if n < 10 { 0 } else { 40 };
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(&mut watch, Some(6), 0..40, steady, |_| 60.0, burst),
            [(12, Verdict::Lighter)]
        );
        assert_eq!(watch.judged, None);
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(&mut watch, Some(20), 0..40, steady, |_| 60.0, burst),
            []
        );
        assert_eq!(watch.judged, None);
        // Steady drops of 10 a second still step down, after 6 s and 20.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(
                &mut watch,
                Some(20),
                0..27,
                steady,
                |_| 60.0,
                |n| 10 * i64::try_from(n).unwrap_or(0)
            ),
            [(26, Verdict::Lighter)]
        );
        assert_eq!(
            watch.judged,
            Some(Judged {
                dropped: 200,
                frames: 1200.0,
                seconds: 20.0,
                too_many: true,
                at_once: false,
            })
        );
        // Never: the filter is still left out when it does not run.
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(
                &mut watch,
                None,
                0..40,
                steady,
                |_| 60.0,
                |n| 10 * i64::try_from(n).unwrap_or(0)
            ),
            []
        );
        let mut watch = FilterWatch::default();
        assert_eq!(
            ticks(&mut watch, None, 0..7, steady, |_| 24.0, |_| 0),
            [(6, Verdict::LeaveOut)]
        );
    }

    #[test]
    fn rife_through_tensorrt_runs_vs_mlrt_and_stands_in_while_not_ready() {
        let folder = Path::new(r"C:\Users\me\AppData\Local\RVP\tensorrt");
        let scripts = Path::new(r"C:\RVP\vapoursynth\vs-mlrt");
        let text = script(
            Engine::RifeTrt,
            Preset::Medium,
            (24000, 1001),
            (60000, 1001),
            &Files {
                tensorrt: Some((folder, scripts)),
                ..Files::default()
            },
            0,
            None,
        );
        let vstrt = format!(
            "core.std.LoadPlugin({})",
            python_path(&folder.join(crate::tensorrt::VSTRT))
        );
        assert!(text.contains(&vstrt), "{text}");
        let path = format!("sys.path.insert(0, {})", python_path(scripts));
        assert!(text.contains(&path), "{text}");
        // 59.94 out of 23.976: vsmlrt.RIFE takes the ratio, 5/2.
        assert!(
            text.contains("multi=Fraction(60060000, 24024000)"),
            "{text}"
        );
        assert!(text.contains("num_streams=2"), "{text}");
        assert!(text.contains("max_shapes=(2560, 1440)"), "{text}");
        let engines = format!("engine_folder={}", python_path(&folder.join("engines")));
        assert!(text.contains(&engines), "{text}");
        assert!(text.ends_with("core.clear_cache()\nclip.set_output()\n"));
        assert!(
            filter_entry(
                Engine::RifeTrt,
                Preset::Medium,
                (60, 1),
                Path::new("/s.vpy")
            )
            .ends_with(":buffered-frames=8:concurrent-frames=8")
        );
        // Not ready (not installed, or its engine not built yet): RIFE on
        // ncnn stands in, else MVTools; ready, it runs; the budget's lighter
        // step gives way to MVTools, as RIFE's does.
        let settings = on(Engine::RifeTrt);
        let with = |rife, rife_trt| Facts {
            engines: Engines {
                mvtools: true,
                rife,
                fruc: false,
                rife_trt,
            },
            ..facts()
        };
        let engine_of = |plan: Plan| match plan {
            Plan::Filter { engine, .. } => Some(engine),
            _ => None,
        };
        assert_eq!(
            engine_of(plan(&settings, &with(true, false), Lightened::default())),
            Some(Engine::Rife)
        );
        assert_eq!(
            engine_of(plan(&settings, &with(false, false), Lightened::default())),
            Some(Engine::MvTools)
        );
        assert_eq!(
            engine_of(plan(&settings, &with(false, true), Lightened::default())),
            Some(Engine::RifeTrt)
        );
        let lighter = Lightened {
            lighter: true,
            ..Lightened::default()
        };
        assert_eq!(
            engine_of(plan(&settings, &with(false, true), lighter)),
            Some(Engine::MvTools)
        );
    }

    #[test]
    fn cuts_repeat_the_last_frame_of_a_scene() {
        assert_eq!(scene_threshold("off"), None);
        assert_eq!(scene_threshold("normal"), Some(0.1));
        assert_eq!(scene_threshold("high"), Some(0.05));
        let text = script(
            Engine::MvTools,
            Preset::Medium,
            (24000, 1001),
            (60000, 1001),
            &Files::default(),
            1,
            Some(0.1),
        );
        // Each source frame against the next, in mean luma.
        assert!(text.contains("PlaneStats(src, src[1:] + src[-1], plane=0, prop='RvpScene')"));
        // Output frame n shows source frame n * 24000 / 60000 at a cut.
        assert!(text.contains(
            "return scene[min(n * 24024000 // 60060000, scene.num_frames - 1)] * (n + 1)"
        ));
        assert!(text.contains("return cuts if f.props['RvpSceneDiff'] > 0.1 else smooth"));
        // Repeated frames take the rate's durations (mpv times frames by them).
        assert!(text.contains("prop_src=cuts), fpsnum=60000, fpsden=1001)\n"));
        // Before the original's first second, which shows the source anyway.
        let cuts = text.find("at_cut").unwrap_or(usize::MAX);
        assert!(cuts < text.find("def original").unwrap_or(0));
        let off = script(
            Engine::MvTools,
            Preset::Medium,
            (24000, 1001),
            (60000, 1001),
            &Files::default(),
            1,
            None,
        );
        assert!(!off.contains("RvpScene"));
        let mut settings = on(Engine::MvTools);
        assert_eq!(settings.scene_change, "normal");
        settings.scene_change = "always".to_owned();
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!((settings.scene_change.as_str(), notes.len()), ("normal", 1));
    }

    #[test]
    fn scripts_clear_the_cache_and_make_the_rate() {
        let mvtools = script(
            Engine::MvTools,
            Preset::Quality,
            (24000, 1001),
            (60, 1),
            &Files::default(),
            0,
            None,
        );
        assert!(mvtools.contains("AssumeFPS(video_in, fpsnum=24000, fpsden=1001)"));
        assert!(!mvtools.contains("LoadPlugin"));
        assert!(mvtools.contains("blksize=16, overlap=8, search=3"));
        assert!(mvtools.contains("core.mv.Recalculate(sup, bv"));
        assert!(mvtools.contains("FlowFPS(clip, sup, bv, fv, num=60, den=1"));
        assert!(mvtools.ends_with("core.clear_cache()\nclip.set_output()\n"));
        let fast = script(
            Engine::MvTools,
            Preset::Fast,
            (25, 1),
            (50, 1),
            &Files {
                mvtools: Some(Path::new(
                    "/opt/rvp/bin/vapoursynth/vs-plugins/libmvtools.so",
                )),
                ..Files::default()
            },
            0,
            None,
        );
        assert!(fast.contains("BlockFPS(clip, sup, bv, fv, num=50, den=1, mode=3"));
        assert!(fast.contains(
            "if not hasattr(core, 'mv'):\n    core.std.LoadPlugin('/opt/rvp/bin/vapoursynth/vs-plugins/libmvtools.so')\n"
        ));
        let plugin = Path::new(r"C:\RVP\vapoursynth\rife\librife_windows_x86-64.dll");
        let model = Path::new(r"C:\RVP\vapoursynth\rife\models\it's");
        let rife = script(
            Engine::Rife,
            Preset::Quality,
            (24000, 1001),
            (60000, 1001),
            &Files {
                rife: Some((plugin, model)),
                ..Files::default()
            },
            0,
            None,
        );
        assert!(rife.contains(
            r"core.std.LoadPlugin('C:\\RVP\\vapoursynth\\rife\\librife_windows_x86-64.dll')"
        ));
        assert!(rife.contains(r"model_path='C:\\RVP\\vapoursynth\\rife\\models\\it\'s'"));
        assert!(rife.contains("fps_num=60000, fps_den=1001, gpu_thread=4"));
        assert!(rife.contains("format=clip.format.id"));
        assert!(rife.contains("core.clear_cache()"));
        assert_eq!(
            filter_entry(
                Engine::MvTools,
                Preset::Medium,
                (60, 1),
                Path::new(r"C:\s\a.vpy")
            ),
            "@rvpsmooth:vapoursynth=file=%10%C:/s/a.vpy:buffered-frames=4:concurrent-frames=16"
        );
        assert_eq!(
            filter_entry(
                Engine::FrucVulkan,
                Preset::Quality,
                (60, 1),
                Path::new("unused")
            ),
            "@rvpsmoothnv12:format=fmt=nv12,@rvpsmoothvk:format=fmt=vulkan,\
             @rvpsmooth:lavfi=[fruc_vulkan=fps=60/1:perf=slow]:hwdec_interop=vulkan"
        );
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = SmoothingSettings {
            target: "fast".to_owned(),
            max_height: 100,
            ..SmoothingSettings::default()
        };
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(settings.target, "display");
        assert_eq!(settings.max_height, 240);
        assert_eq!(notes.len(), 2);
        let json = serde_json::to_value(SmoothingSettings::default()).expect("json");
        assert_eq!(json["engine"], "mvtools");
        assert_eq!(json["preset"], "medium");
        assert_eq!(json["max-height"], 1080);
        let parsed: SmoothingSettings =
            serde_json::from_str(r#"{"enabled": true, "engine": "fruc-vulkan"}"#).expect("parse");
        assert_eq!(parsed.engine, Engine::FrucVulkan);
        assert!(parsed.enabled);
    }
}
