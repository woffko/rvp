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
//!   they need copy-back decoding. Blend is mpv's `interpolation` with
//!   `video-sync=display-resample`; with the OpenGL renderer mpv does not
//!   know the display's rate, so RVP gives it (`display-fps-override`).
//! * Smoothing stays off while SVP is attached (`@svp` in the chain; RVP
//!   never touches SVP's filter), while RVP's own VapourSynth script slot
//!   runs, and when the video already plays at the target rate. An engine
//!   that cannot run here gives way: RIFE and `fruc_vulkan` to MVTools,
//!   MVTools to Blend.
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

use rvp_engine::{Engine as Mpv, PropertyId};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::gpu_power::Step;

/// The label of RVP's smoothing filter.
pub const LABEL: &str = "rvpsmooth";

/// The target rates the settings offer: the display's, twice the
/// source's, or a rate.
pub const TARGETS: &[&str] = &[
    "display", "2x", "48", "50", "60", "72", "90", "100", "120", "144",
];

/// The heights the size limit offers.
pub const MAX_HEIGHTS: &[u32] = &[480, 576, 720, 1080, 1440, 2160];

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
    #[serde(rename = "fruc-vulkan")]
    FrucVulkan,
}

impl Engine {
    pub const ALL: [Self; 4] = [Self::Blend, Self::MvTools, Self::Rife, Self::FrucVulkan];

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::Blend => "Blend",
            Self::MvTools => "MVTools",
            Self::Rife => "RIFE",
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
            Self::FrucVulkan => "fruc-vulkan",
        }
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
}

impl Default for SmoothingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: Engine::MvTools,
            preset: Preset::Medium,
            target: "display".to_owned(),
            max_height: 1080,
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
    }
}

/// What the plan needs to know about the file and the machine.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Facts {
    /// The video's height (0: no video).
    pub height: u32,
    /// The source's frame rate (0: unknown).
    pub source_fps: f64,
    /// The display's refresh rate (0: unknown).
    pub display_fps: f64,
    /// SVP's filter is in the chain.
    pub svp_attached: bool,
    /// RVP's VapourSynth script slot runs.
    pub script_slot: bool,
    pub engines: Engines,
}

/// The motion-compensated engines this machine has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Engines {
    /// A VapourSynth runtime with MVTools.
    pub mvtools: bool,
    /// RIFE's plugin and model next to the runtime.
    pub rife: bool,
    /// libmpv's FFmpeg has `fruc_vulkan`.
    pub fruc: bool,
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
        Engine::Rife if !facts.engines.rife => Engine::MvTools,
        engine => engine,
    };
    let mut preset = settings.preset;
    if lightened.lighter {
        if engine == Engine::Rife {
            engine = Engine::MvTools;
        } else {
            preset = preset.lighter();
        }
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
/// (a fraction), making `rate`. `rife` names RIFE's plugin and model.
#[must_use]
pub fn script(
    engine: Engine,
    preset: Preset,
    source: (u64, u64),
    rate: (u64, u64),
    rife: Option<(&Path, &Path)>,
) -> String {
    let mut text = format!(
        "# Generated by RVP (M18): motion smoothing.\n\
         import vapoursynth as vs\n\
         core = vs.core\n\
         clip = core.std.AssumeFPS(video_in, fpsnum={}, fpsden={})\n",
        source.0, source.1
    );
    if let (Engine::Rife, Some((plugin, model))) = (engine, rife) {
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
        return format!(
            "@{LABEL}:lavfi=[hwupload,fruc_vulkan=fps={}/{}:perf={perf},hwdownload,format=nv12]",
            rate.0, rate.1
        );
    }
    let path = script.to_string_lossy().replace('\\', "/");
    // RIFE keeps its GPU threads busy with the frames in flight.
    let frames = if engine == Engine::Rife { 8 } else { 4 };
    format!(
        "@{LABEL}:vapoursynth=file=%{}%{path}:buffered-frames={frames}:concurrent-frames={frames}",
        path.len()
    )
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
    /// RIFE's plugin and model.
    rife: Option<(PathBuf, PathBuf)>,
}

impl Runtime {
    /// What the runtime of `vsscript` has.
    pub(crate) fn probe(vsscript: Option<&Path>) -> Self {
        Self {
            mvtools: vsscript.is_some_and(has_mvtools),
            rife: vsscript.and_then(rife_files),
        }
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
}

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
        Facts {
            height: self
                .model
                .int(PropertyId::VideoHeight)
                .and_then(|height| u32::try_from(height).ok())
                .unwrap_or(0),
            source_fps: self.model.double(PropertyId::ContainerFps).unwrap_or(0.0),
            display_fps: self.picture.screen.refresh,
            svp_attached: self
                .model
                .text(PropertyId::VideoFilters)
                .is_some_and(|vf| vf.contains("@svp")),
            script_slot: vs.enabled && vs.script.is_some() && !self.gpu_step_taken(Step::Script),
            engines: Engines {
                mvtools: self.smoothing.runtime.mvtools,
                rife: self.smoothing.runtime.rife.is_some(),
                fruc: self.smoothing.fruc,
            },
        }
    }

    /// The GPU power budget's smoothing steps that are taken.
    fn smoothing_lightened(&self) -> Lightened {
        Lightened {
            lighter: self.gpu_step_taken(Step::SmoothingLighter),
            blend: self.gpu_step_taken(Step::SmoothingBlend),
            off: self.gpu_step_taken(Step::SmoothingOff),
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

    fn smoothing_script_path(&self) -> PathBuf {
        self.state_dir().join("vapoursynth").join("rvp-smooth.vpy")
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
        let rife = self
            .smoothing
            .runtime
            .rife
            .as_ref()
            .map(|(plugin, model)| (plugin.as_path(), model.as_path()));
        let path = self.smoothing_script_path();
        crate::storage::write_atomic(&path, script(engine, preset, source, rate, rife).as_bytes())
            .map_err(|error| format!("{}: {error}", path.display()))
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
        self.smoothing.applied = Some(plan.clone());
        if filter_before || filter_now {
            self.apply_decoding(engine);
            drop(self.apply_video_filters(engine));
        }
        Some(plan)
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

    /// Which engines this machine has, for the settings window.
    #[must_use]
    pub fn smoothing_engines(&self) -> serde_json::Value {
        serde_json::json!({
            "mvtools": self.smoothing.runtime.mvtools,
            "rife": self.smoothing.runtime.rife.is_some(),
            "fruc": self.smoothing.fruc,
        })
    }

    /// True when taking a smoothing step of the GPU power budget now
    /// would change what runs.
    pub(crate) fn smoothing_step_changes(&self, step: Step) -> bool {
        match (step, self.smoothing_plan()) {
            (Step::SmoothingLighter, Plan::Filter { engine, preset, .. }) => {
                engine == Engine::Rife || preset != Preset::Fast
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

    fn facts() -> Facts {
        Facts {
            height: 1080,
            source_fps: 24000.0 / 1001.0,
            display_fps: 60.0,
            engines: Engines {
                mvtools: true,
                rife: true,
                fruc: true,
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
    fn scripts_clear_the_cache_and_make_the_rate() {
        let mvtools = script(
            Engine::MvTools,
            Preset::Quality,
            (24000, 1001),
            (60, 1),
            None,
        );
        assert!(mvtools.contains("AssumeFPS(video_in, fpsnum=24000, fpsden=1001)"));
        assert!(mvtools.contains("blksize=16, overlap=8, search=3"));
        assert!(mvtools.contains("core.mv.Recalculate(sup, bv"));
        assert!(mvtools.contains("FlowFPS(clip, sup, bv, fv, num=60, den=1"));
        assert!(mvtools.ends_with("core.clear_cache()\nclip.set_output()\n"));
        let fast = script(Engine::MvTools, Preset::Fast, (25, 1), (50, 1), None);
        assert!(fast.contains("BlockFPS(clip, sup, bv, fv, num=50, den=1, mode=3"));
        let plugin = Path::new(r"C:\RVP\vapoursynth\rife\librife_windows_x86-64.dll");
        let model = Path::new(r"C:\RVP\vapoursynth\rife\models\it's");
        let rife = script(
            Engine::Rife,
            Preset::Quality,
            (24000, 1001),
            (60000, 1001),
            Some((plugin, model)),
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
            "@rvpsmooth:vapoursynth=file=%10%C:/s/a.vpy:buffered-frames=4:concurrent-frames=4"
        );
        assert_eq!(
            filter_entry(
                Engine::FrucVulkan,
                Preset::Quality,
                (60, 1),
                Path::new("unused")
            ),
            "@rvpsmooth:lavfi=[hwupload,fruc_vulkan=fps=60/1:perf=slow,hwdownload,format=nv12]"
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
