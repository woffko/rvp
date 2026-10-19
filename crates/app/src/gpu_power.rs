//! RVP's GPU power budget (D17, owner 2026-10-19): RVP keeps the graphics
//! card below a share of its rated power by lightening its own work. It
//! never changes the card's settings: the card's power limit stays the
//! system's (or the owner's, for the host's test runs).
//!
//! * While a file plays, the shell reads each card's draw and rated power
//!   once a second on a thread of its own (`shell/src/gpu_power.rs`):
//!   NVIDIA's through NVML (Windows and Linux), AMD's through amdgpu's
//!   hwmon files on Linux ([`amdgpu_samples`]) and through Windows' own
//!   adapter counter on Windows (D3DKMT's adapter performance data, a
//!   share of the card's power; AMD's ADLX SDK is licensed on terms a GPL
//!   program cannot meet). A hung card can hold such a call up, so the
//!   window's thread never makes one: it takes the readings in
//!   [`App::gpu_tick`]. On Linux a card the system has put to sleep is not
//!   read ([`card_awake`], [`nvidia_awake`]): reading it would wake it.
//! * The share is the busiest card's draw over its rated power (its
//!   default power limit, else the one in force), averaged over the last
//!   [`AVERAGE`] readings.
//! * Above the budget for [`Timing::over`], RVP takes one [`Step`] of
//!   those that would change something now, M18's motion smoothing first
//!   (`crate::smoothing`: a lighter preset, Blend, off), then the least
//!   visible of the rest; the next follows after [`Timing::settle`] while
//!   the share stays above.
//! * Below the budget less [`MARGIN`] for the wait (first
//!   [`Timing::under`]), it gives the last step back. A step taken again
//!   within [`Timing::retake`] of being given back doubles the wait, up to
//!   [`Timing::under_max`]; with every step back and the card calm for
//!   [`Timing::under_max`], the wait is the first one again.
//! * The steps change only what mpv gets: the settings keep the user's
//!   choices. Each step and each return shows a notice.
//! * At 100 % the budget is off: every step comes back at once and the
//!   shell stops reading.
//!
//! What a step does is RVP's own work only: SVP and other programs keep
//! the card as busy as they make it.

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rvp_engine::Engine;
use serde_json::Value;

use crate::app::{App, UiRequest};

/// Readings averaged for the share.
pub const AVERAGE: usize = 3;

/// Points below the budget at which steps come back (0.10: 80 % for a
/// 90 % budget).
pub const MARGIN: f64 = 0.10;

/// The budget's range in percent; 100 is off.
pub const BUDGET_MIN: u32 = 50;
pub const BUDGET_OFF: u32 = 100;
/// The owner's default (2026-10-19).
pub const BUDGET_DEFAULT: u32 = 90;

/// The scaler that takes an EWA scaler's place in [`Step::Scalers`]:
/// separable, so far cheaper than the two-dimensional EWA filters.
pub const LIGHT_SCALER: &str = "spline36";

/// What a reading counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Watts,
    /// A share of the card's power in percent (`rated` is 100): Windows'
    /// adapter counter.
    Percent,
}

/// One card's reading.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// The card's name, for the settings window.
    pub name: String,
    /// Its draw now.
    pub draw: f64,
    /// Its rated power: the default power limit, else the one in force.
    pub rated: f64,
    pub unit: Unit,
}

impl Sample {
    /// Draw over rated power; `None` without a rated power.
    #[must_use]
    pub fn share(&self) -> Option<f64> {
        (self.rated > 0.0 && self.draw.is_finite() && self.draw >= 0.0)
            .then(|| self.draw / self.rated)
    }
}

/// The busiest card's share.
#[must_use]
pub fn busiest(samples: &[Sample]) -> Option<f64> {
    samples
        .iter()
        .filter_map(Sample::share)
        .fold(None, |best: Option<f64>, share| {
            Some(best.map_or(share, |best| best.max(share)))
        })
}

/// What RVP lightens ([`Step::ALL`]'s order): M18's smoothing first, its
/// heaviest work, then the rest, the least visible first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    /// Smoothing one preset lighter; RIFE gives way to MVTools.
    SmoothingLighter,
    /// Blend instead of motion-compensated smoothing.
    SmoothingBlend,
    /// Smoothing off.
    SmoothingOff,
    /// EWA scalers (upscaler, downscaler, chroma) become [`LIGHT_SCALER`].
    Scalers,
    /// GPU Super Resolution and RTX Video HDR off (the Direct3D 11 video
    /// processor).
    Processor,
    /// The pixel shaders off.
    Shaders,
    /// The VapourSynth script off.
    Script,
}

impl Step {
    pub const ALL: [Self; 7] = [
        Self::SmoothingLighter,
        Self::SmoothingBlend,
        Self::SmoothingOff,
        Self::Scalers,
        Self::Processor,
        Self::Shaders,
        Self::Script,
    ];

    /// The notice's name of the step taken.
    #[must_use]
    pub fn taken_text(self) -> &'static str {
        match self {
            Self::SmoothingLighter => "lighter motion smoothing",
            Self::SmoothingBlend => "motion smoothing by blending",
            Self::SmoothingOff => "motion smoothing off",
            Self::Scalers => "lighter scalers",
            Self::Processor => "GPU Super Resolution and RTX Video HDR off",
            Self::Shaders => "pixel shaders off",
            Self::Script => "VapourSynth script off",
        }
    }

    /// The notice's name of the step given back.
    #[must_use]
    pub fn back_text(self) -> &'static str {
        match self {
            Self::SmoothingLighter | Self::SmoothingBlend | Self::SmoothingOff => {
                "motion smoothing back"
            }
            Self::Scalers => "the chosen scalers back",
            Self::Processor => "GPU Super Resolution and RTX Video HDR back",
            Self::Shaders => "pixel shaders back",
            Self::Script => "VapourSynth script back",
        }
    }

    /// The settings name (the self-test's and the view's).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::SmoothingLighter => "smoothing-lighter",
            Self::SmoothingBlend => "smoothing-blend",
            Self::SmoothingOff => "smoothing-off",
            Self::Scalers => "scalers",
            Self::Processor => "processor",
            Self::Shaders => "shaders",
            Self::Script => "script",
        }
    }
}

/// The budget's waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Above the budget this long before a step.
    pub over: Duration,
    /// After a step, or a step given back, before the next change.
    pub settle: Duration,
    /// Below the budget less [`MARGIN`] this long before a step comes back
    /// (the first wait).
    pub under: Duration,
    /// The longest wait.
    pub under_max: Duration,
    /// A step taken again this soon after it came back doubles the wait.
    pub retake: Duration,
}

impl Timing {
    pub const DEFAULT: Self = Self {
        over: Duration::from_secs(3),
        settle: Duration::from_secs(5),
        under: Duration::from_mins(1),
        under_max: Duration::from_mins(16),
        retake: Duration::from_mins(2),
    };
    /// The self-test's: the same rules in seconds.
    pub const TEST: Self = Self {
        over: Duration::from_secs(1),
        settle: Duration::from_millis(500),
        under: Duration::from_secs(2),
        under_max: Duration::from_secs(8),
        retake: Duration::from_secs(4),
    };
}

/// A change the governor asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Take(Step),
    Give(Step),
}

/// The budget's rules over the readings (the module's documentation).
#[derive(Debug, Clone)]
pub struct Governor {
    timing: Timing,
    /// The last shares, the oldest overwritten first.
    shares: [f64; AVERAGE],
    filled: u32,
    next: usize,
    /// Since when the average has been above the budget, or below it less
    /// the margin.
    over_since: Option<Instant>,
    under_since: Option<Instant>,
    /// When a step was last taken or given back.
    changed_at: Option<Instant>,
    /// When a step last came back.
    returned_at: Option<Instant>,
    /// The wait below the budget before a step comes back.
    under_wait: Duration,
    /// The steps taken, the last on top.
    taken: Vec<Step>,
}

impl Default for Governor {
    fn default() -> Self {
        Self::new(Timing::DEFAULT)
    }
}

impl Governor {
    #[must_use]
    pub fn new(timing: Timing) -> Self {
        Self {
            timing,
            shares: [0.0; AVERAGE],
            filled: 0,
            next: 0,
            over_since: None,
            under_since: None,
            changed_at: None,
            returned_at: None,
            under_wait: timing.under,
            taken: Vec::with_capacity(Step::ALL.len()),
        }
    }

    /// The steps taken, in the order they were.
    #[must_use]
    pub fn taken(&self) -> &[Step] {
        &self.taken
    }

    /// The average share of the last readings.
    #[must_use]
    pub fn average(&self) -> Option<f64> {
        let count = self.filled as usize;
        (count > 0).then(|| self.shares[..count].iter().sum::<f64>() / f64::from(self.filled))
    }

    /// Every step back at once (the budget turned off); returns them, the
    /// last taken first.
    pub fn give_all(&mut self) -> Vec<Step> {
        let mut steps = std::mem::take(&mut self.taken);
        steps.reverse();
        *self = Self::new(self.timing);
        steps
    }

    /// One reading: the busiest card's share (`None`: no card read), the
    /// budget (a share, 0.5 to 1.0), the steps that would change something
    /// now (in [`Step::ALL`]'s order) and the time. Returns the change to
    /// make; the caller makes it.
    pub fn update(
        &mut self,
        share: Option<f64>,
        budget: f64,
        available: &[Step],
        now: Instant,
    ) -> Option<Change> {
        let share = share?;
        self.shares[self.next] = share;
        self.next = (self.next + 1) % AVERAGE;
        // AVERAGE is a small constant.
        #[allow(clippy::cast_possible_truncation)]
        let full = AVERAGE as u32;
        self.filled = (self.filled + 1).min(full);
        let average = self.average()?;
        let settled = self
            .changed_at
            .is_none_or(|at| now.duration_since(at) >= self.timing.settle);
        if average > budget {
            self.under_since = None;
            let since = *self.over_since.get_or_insert(now);
            if now.duration_since(since) < self.timing.over || !settled {
                return None;
            }
            let step = *available.iter().find(|step| !self.taken.contains(step))?;
            if self
                .returned_at
                .is_some_and(|at| now.duration_since(at) < self.timing.retake)
            {
                self.under_wait = (self.under_wait * 2).min(self.timing.under_max);
            }
            self.taken.push(step);
            self.changed_at = Some(now);
            return Some(Change::Take(step));
        }
        self.over_since = None;
        if average >= budget - MARGIN {
            self.under_since = None;
            return None;
        }
        let since = *self.under_since.get_or_insert(now);
        let calm = now.duration_since(since);
        if self.taken.is_empty() {
            if calm >= self.timing.under_max {
                self.under_wait = self.timing.under;
            }
            return None;
        }
        if calm < self.under_wait || !settled {
            return None;
        }
        let step = self.taken.pop()?;
        self.returned_at = Some(now);
        self.changed_at = Some(now);
        // The next step waits its own time.
        self.under_since = Some(now);
        Some(Change::Give(step))
    }
}

/// The budget's state in the [`App`].
#[derive(Debug, Default)]
pub(crate) struct Budget {
    /// The shell's readings, while it reads ([`App::gpu_power_listen`]).
    receiver: Option<mpsc::Receiver<Vec<Sample>>>,
    governor: Governor,
    /// The last reading and when it came.
    last: Vec<Sample>,
    last_at: Option<Instant>,
}

impl App {
    /// True while the budget is on: the shell then reads the cards.
    #[must_use]
    pub fn gpu_power_wanted(&self) -> bool {
        self.settings.render.power_budget < BUDGET_OFF
    }

    /// The channel the shell's readings arrive on; `None` when it stops
    /// reading.
    pub fn gpu_power_listen(&mut self, receiver: Option<mpsc::Receiver<Vec<Sample>>>) {
        self.gpu.receiver = receiver;
    }

    /// The self-test's waits ([`Timing::TEST`]).
    pub fn gpu_power_test(&mut self) {
        self.gpu.governor = Governor::new(Timing::TEST);
    }

    /// True while `step` is taken.
    #[must_use]
    pub fn gpu_step_taken(&self, step: Step) -> bool {
        self.gpu.governor.taken().contains(&step)
    }

    /// Takes the shell's latest reading (call it about once a second); at
    /// 100 % every step comes back.
    pub fn gpu_tick(&mut self, engine: &Engine, now: Instant) -> Vec<UiRequest> {
        if !self.gpu_power_wanted() {
            let steps = self.gpu.governor.give_all();
            for step in &steps {
                self.apply_gpu_step(engine, *step);
            }
            return if steps.is_empty() {
                Vec::new()
            } else {
                vec![UiRequest::Osd(
                    "GPU power budget off: RVP's work is as chosen again".to_owned(),
                )]
            };
        }
        let latest = self
            .gpu
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_iter().last());
        match latest {
            Some(samples) => self.gpu_power_sample(engine, samples, now),
            None => Vec::new(),
        }
    }

    /// One reading of the cards: the budget's rules decide, RVP makes the
    /// change and says so. The self-test gives its own readings here.
    pub fn gpu_power_sample(
        &mut self,
        engine: &Engine,
        samples: Vec<Sample>,
        now: Instant,
    ) -> Vec<UiRequest> {
        let share = busiest(&samples);
        self.gpu.last = samples;
        self.gpu.last_at = Some(now);
        let budget = f64::from(self.settings.render.power_budget) / 100.0;
        let available: Vec<Step> = Step::ALL
            .into_iter()
            .filter(|step| self.gpu_step_changes(*step))
            .collect();
        let Some(change) = self.gpu.governor.update(share, budget, &available, now) else {
            return Vec::new();
        };
        let percent = share.map_or(0.0, |share| share * 100.0);
        let text = match change {
            Change::Take(step) => {
                self.apply_gpu_step(engine, step);
                format!(
                    "GPU power {percent:.0} % (budget {} %): {}",
                    self.settings.render.power_budget,
                    step.taken_text()
                )
            }
            Change::Give(step) => {
                self.apply_gpu_step(engine, step);
                format!("GPU power {percent:.0} %: {}", step.back_text())
            }
        };
        vec![UiRequest::Osd(text)]
    }

    /// True when taking `step` now would change what mpv gets. It runs
    /// for every reading on the window's thread: settings only, no files.
    fn gpu_step_changes(&self, step: Step) -> bool {
        match step {
            Step::SmoothingLighter | Step::SmoothingBlend | Step::SmoothingOff => {
                self.smoothing_step_changes(step)
            }
            Step::Scalers => {
                let render = &self.settings.render;
                [&self.settings.scaler, &render.dscale, &render.cscale]
                    .iter()
                    .any(|scaler| is_ewa(scaler))
            }
            Step::Processor => self.processor_wanted(),
            Step::Shaders => [crate::hlsl::Stage::Pre, crate::hlsl::Stage::Post]
                .into_iter()
                .any(|stage| self.shaders_on(stage) && !self.shader_list(stage).is_empty()),
            Step::Script => {
                let vs = &self.settings.vapoursynth;
                vs.enabled && vs.script.is_some()
            }
        }
    }

    /// Sets on mpv what `step` changes, taken or given back.
    fn apply_gpu_step(&mut self, engine: &Engine, step: Step) {
        match step {
            Step::SmoothingLighter | Step::SmoothingBlend | Step::SmoothingOff => {
                self.apply_smoothing(engine);
            }
            Step::Scalers => {
                let scale = if self.settings.scaler.is_empty() {
                    engine
                        .get_text("option-info/scale/default-value")
                        .unwrap_or_else(|| "lanczos".to_owned())
                } else {
                    self.effective_scaler(&self.settings.scaler).to_owned()
                };
                drop(engine.set_text("scale", &scale));
                self.apply_render(engine);
            }
            Step::Processor | Step::Script => {
                self.apply_decoding(engine);
                drop(self.apply_video_filters(engine));
            }
            Step::Shaders => drop(self.apply_shaders(engine)),
        }
    }

    /// A scaler as mpv gets it: [`LIGHT_SCALER`] for an EWA scaler while
    /// [`Step::Scalers`] is taken.
    #[must_use]
    pub fn effective_scaler<'a>(&self, scaler: &'a str) -> &'a str {
        if is_ewa(scaler) && self.gpu_step_taken(Step::Scalers) {
            LIGHT_SCALER
        } else {
            scaler
        }
    }

    /// The budget for the settings window: the cards' last reading and the
    /// steps taken.
    #[must_use]
    pub fn gpu_power_view(&self, now: Instant) -> Value {
        // A reading older than a few seconds says the reader stopped.
        let fresh = self
            .gpu
            .last_at
            .is_some_and(|at| now.duration_since(at) < Duration::from_secs(5));
        let cards: Vec<Value> = if fresh {
            self.gpu
                .last
                .iter()
                .map(|sample| {
                    serde_json::json!({
                        "name": sample.name,
                        "draw": sample.draw,
                        "rated": sample.rated,
                        "unit": match sample.unit {
                            Unit::Watts => "W",
                            Unit::Percent => "%",
                        },
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        serde_json::json!({
            "on": self.gpu_power_wanted(),
            "reading": self.gpu.receiver.is_some(),
            "cards": cards,
            "taken": self.gpu.governor.taken().iter().map(|step| step.name()).collect::<Vec<_>>(),
        })
    }
}

/// True for mpv's EWA (two-dimensional) scalers.
fn is_ewa(scaler: &str) -> bool {
    scaler.starts_with("ewa_")
}

/// The amdgpu cards' readings under `drm` (`/sys/class/drm` on Linux):
/// each `cardN` whose `device/vendor` is AMD's (`0x1002`), whose hwmon
/// folder is amdgpu's and reports power, and that is awake
/// ([`card_awake`]). The draw is `power1_average` (RDNA cards), else
/// `power1_input` (Polaris, Vega and some APUs), the rated power
/// `power1_cap_default` (Linux 5.13), else `power1_cap`; the files count
/// microwatts and anyone may read them. Cards without a cap (APUs, whose
/// figure includes the CPU) are left out.
#[must_use]
pub fn amdgpu_samples(drm: &Path) -> Vec<Sample> {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut cards: Vec<(u32, std::path::PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name.strip_prefix("card")?.parse::<u32>().ok()?;
            Some((number, entry.path()))
        })
        .collect();
    // Card numbers, not the directory's order (deterministic results).
    cards.sort();
    let mut samples = Vec::new();
    for (number, card) in cards {
        let device = card.join("device");
        if read_trimmed(&device.join("vendor")).as_deref() != Some("0x1002") || !card_awake(&device)
        {
            continue;
        }
        let Ok(hwmons) = std::fs::read_dir(device.join("hwmon")) else {
            continue;
        };
        let mut folders: Vec<std::path::PathBuf> =
            hwmons.filter_map(Result::ok).map(|e| e.path()).collect();
        folders.sort();
        for hwmon in folders {
            if read_trimmed(&hwmon.join("name")).as_deref() != Some("amdgpu") {
                continue;
            }
            let micro = |file: &str| {
                read_trimmed(&hwmon.join(file)).and_then(|text| text.parse::<u64>().ok())
            };
            let draw = micro("power1_average").or_else(|| micro("power1_input"));
            let rated = micro("power1_cap_default")
                .or_else(|| micro("power1_cap"))
                .filter(|cap| *cap > 0);
            if let (Some(draw), Some(rated)) = (draw, rated) {
                let name = read_trimmed(&device.join("product_name"))
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| format!("AMD card{number}"));
                samples.push(Sample {
                    name,
                    draw: watts(draw),
                    rated: watts(rated),
                    unit: Unit::Watts,
                });
                break;
            }
        }
    }
    samples
}

/// True unless the system put the PCI `device` to sleep (runtime power
/// management): reading its sensors would wake it, on Linux 6.12 and
/// older, and a reading a second would keep a laptop's card awake. The
/// state file can be read without waking the device; a device without
/// runtime power management counts as awake.
#[must_use]
pub fn card_awake(device: &Path) -> bool {
    !matches!(
        read_trimmed(&device.join("power").join("runtime_status")).as_deref(),
        Some("suspended" | "suspending")
    )
}

/// False when NVIDIA's display devices under `pci`
/// (`/sys/bus/pci/devices` on Linux) are all asleep: then NVML is not
/// called, which would wake them. True when one is awake, and when none
/// is listed (WSL and containers show no PCI device, yet NVML works).
#[must_use]
pub fn nvidia_awake(pci: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(pci) else {
        return true;
    };
    let mut cards = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|device| {
            read_trimmed(&device.join("vendor")).as_deref() == Some("0x10de")
                && read_trimmed(&device.join("class"))
                    .is_some_and(|class| class.starts_with("0x03"))
        });
    let Some(first) = cards.next() else {
        return true;
    };
    card_awake(&first) || cards.any(|device| card_awake(&device))
}

/// A sysfs file's text without its line end.
fn read_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
}

/// Microwatts as watts.
fn watts(micro: u64) -> f64 {
    // A card's power in microwatts is far inside f64's exact range.
    #[allow(clippy::cast_precision_loss)]
    let micro = micro as f64;
    micro / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The steps outside smoothing, which these tests run through.
    const STEPS: [Step; 4] = [Step::Scalers, Step::Processor, Step::Shaders, Step::Script];

    fn at(start: Instant, seconds: f64) -> Instant {
        start + Duration::from_secs_f64(seconds)
    }

    #[test]
    fn steps_are_taken_above_the_budget_one_at_a_time() {
        let start = Instant::now();
        let mut governor = Governor::new(Timing::DEFAULT);
        let mut changes = Vec::new();
        // Two readings at half power, then the card at 99 %; from 13 s on
        // only the script would change anything.
        let readings = [(0, 0.5), (1, 0.5)]
            .into_iter()
            .chain((2..30).map(|second| (second, 0.99)));
        for (second, share) in readings {
            let available: &[Step] = if second < 13 { &STEPS } else { &[Step::Script] };
            let now = at(start, f64::from(second));
            if let Some(change) = governor.update(Some(share), 0.9, available, now) {
                changes.push((second, change));
            }
        }
        // The average of three readings is above the budget from 4 s on;
        // the first step comes 3 s (`over`) later, each next one 5 s
        // (`settle`) after the one before, of the steps that would change
        // something.
        assert_eq!(
            changes,
            [
                (7, Change::Take(Step::Scalers)),
                (12, Change::Take(Step::Processor)),
                (17, Change::Take(Step::Script)),
            ]
        );
        assert_eq!(
            governor.taken(),
            [Step::Scalers, Step::Processor, Step::Script]
        );
    }

    #[test]
    fn steps_come_back_below_the_margin_and_waits_grow_after_retakes() {
        let start = Instant::now();
        let mut governor = Governor::new(Timing::DEFAULT);
        let all = STEPS;
        let mut now = 0.0;
        let mut feed = |governor: &mut Governor, share: f64, seconds: f64| {
            let mut changes = Vec::new();
            let end = now + seconds;
            while now < end {
                now += 1.0;
                if let Some(change) = governor.update(Some(share), 0.9, &all, at(start, now)) {
                    changes.push((now, change));
                }
            }
            changes
        };
        assert_eq!(
            feed(&mut governor, 0.99, 10.0),
            [
                (4.0, Change::Take(Step::Scalers)),
                (9.0, Change::Take(Step::Processor))
            ]
        );
        // Between the margin and the budget nothing comes back.
        assert!(feed(&mut governor, 0.85, 120.0).is_empty());
        // Below 80 %: the average gets there after two readings, then the
        // first wait (60 s) gives the last step back, and 60 s more the
        // one before.
        let back = feed(&mut governor, 0.5, 130.0);
        assert_eq!(back.len(), 2, "{back:?}");
        assert_eq!(back[0].1, Change::Give(Step::Processor));
        assert_eq!(back[1].1, Change::Give(Step::Scalers));
        assert!((back[1].0 - back[0].0 - 60.0).abs() < 1.5, "{back:?}");
        // Taken again soon after: the wait doubles to 120 s.
        let again = feed(&mut governor, 0.99, 6.0);
        assert_eq!(again.len(), 1, "{again:?}");
        assert_eq!(again[0].1, Change::Take(Step::Scalers));
        let slow = feed(&mut governor, 0.5, 125.0);
        assert_eq!(slow.len(), 1, "{slow:?}");
        assert!(slow[0].0 - again[0].0 > 119.0, "{slow:?} after {again:?}");
        // Calm for `under_max` with every step back: the first wait again.
        assert!(feed(&mut governor, 0.5, 970.0).is_empty());
        assert_eq!(feed(&mut governor, 0.99, 6.0).len(), 1);
        let quick = feed(&mut governor, 0.5, 70.0);
        assert_eq!(quick.len(), 1, "{quick:?}");
    }

    #[test]
    fn no_reading_changes_nothing_and_off_gives_everything_back() {
        let start = Instant::now();
        let mut governor = Governor::new(Timing::TEST);
        assert_eq!(governor.update(None, 0.9, &STEPS, start), None);
        assert_eq!(governor.average(), None);
        // The self-test's waits: a step 1 s after the first reading above,
        // the next 0.5 s later.
        for tenth in 0..=15 {
            governor.update(Some(1.0), 0.9, &STEPS, at(start, f64::from(tenth) / 10.0));
        }
        assert_eq!(governor.taken(), [Step::Scalers, Step::Processor]);
        assert_eq!(governor.give_all(), [Step::Processor, Step::Scalers]);
        assert!(governor.taken().is_empty());
    }

    #[test]
    fn the_busiest_card_counts() {
        let card = |draw: f64, rated: f64| Sample {
            name: "card".to_owned(),
            draw,
            rated,
            unit: Unit::Watts,
        };
        assert_eq!(busiest(&[]), None);
        assert_eq!(busiest(&[card(100.0, 0.0)]), None);
        let share = busiest(&[card(35.0, 350.0), card(150.0, 200.0), card(f64::NAN, 100.0)]);
        assert!(
            share.is_some_and(|share| (share - 0.75).abs() < 1e-9),
            "{share:?}"
        );
    }

    #[test]
    fn amdgpu_cards_are_read_from_their_hwmon_files() {
        let root = std::env::temp_dir().join(format!("rvp-amdgpu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, text).expect("write");
        };
        // An RDNA 2 card on an older kernel: average and the default cap.
        write("card1/device/vendor", "0x1002\n");
        write("card1/device/power/runtime_status", "active\n");
        write("card1/device/hwmon/hwmon3/name", "amdgpu\n");
        write("card1/device/product_name", "Radeon RX 6800\n");
        write("card1/device/hwmon/hwmon3/power1_average", "203000000\n");
        write(
            "card1/device/hwmon/hwmon3/power1_cap_default",
            "250000000\n",
        );
        write("card1/device/hwmon/hwmon3/power1_cap", "203000000\n");
        // A card on a newer kernel with only the input and the cap in force.
        write("card0/device/vendor", "0x1002\n");
        write("card0/device/hwmon/hwmon2/name", "amdgpu\n");
        write("card0/device/hwmon/hwmon2/power1_input", "35500000\n");
        write("card0/device/hwmon/hwmon2/power1_cap", "355000000\n");
        // NVIDIA's card, a connector, a card without a sensor, a laptop's
        // card asleep and another driver's hwmon folder.
        write("card2/device/vendor", "0x10de\n");
        write("card0-DP-1/status", "connected\n");
        write("card3/device/vendor", "0x1002\n");
        write("card3/device/hwmon/hwmon5/name", "amdgpu\n");
        write("card3/device/hwmon/hwmon5/temp1_input", "40000\n");
        write("card4/device/vendor", "0x1002\n");
        write("card4/device/power/runtime_status", "suspended\n");
        write("card4/device/hwmon/hwmon6/name", "amdgpu\n");
        write("card4/device/hwmon/hwmon6/power1_average", "1000000\n");
        write("card4/device/hwmon/hwmon6/power1_cap", "100000000\n");
        write("card5/device/vendor", "0x1002\n");
        write("card5/device/hwmon/hwmon7/name", "nct6775\n");
        write("card5/device/hwmon/hwmon7/power1_average", "1000000\n");
        write("card5/device/hwmon/hwmon7/power1_cap", "100000000\n");
        let samples = amdgpu_samples(&root);
        assert_eq!(
            samples,
            [
                Sample {
                    name: "AMD card0".to_owned(),
                    draw: 35.5,
                    rated: 355.0,
                    unit: Unit::Watts,
                },
                Sample {
                    name: "Radeon RX 6800".to_owned(),
                    draw: 203.0,
                    rated: 250.0,
                    unit: Unit::Watts,
                },
            ]
        );
        assert!(amdgpu_samples(&root.join("missing")).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nvml_is_not_called_while_every_nvidia_card_sleeps() {
        let root = std::env::temp_dir().join(format!("rvp-pci-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, text).expect("write");
        };
        // PCI addresses without their colons, which Windows refuses in
        // names. No NVIDIA device listed (WSL): NVML may know one.
        write("0000-00-02.0/vendor", "0x8086\n");
        write("0000-00-02.0/class", "0x030000\n");
        assert!(nvidia_awake(&root));
        // A laptop's NVIDIA card asleep; its audio function does not count.
        write("0000-01-00.0/vendor", "0x10de\n");
        write("0000-01-00.0/class", "0x030000\n");
        write("0000-01-00.0/power/runtime_status", "suspended\n");
        write("0000-01-00.1/vendor", "0x10de\n");
        write("0000-01-00.1/class", "0x040300\n");
        write("0000-01-00.1/power/runtime_status", "active\n");
        assert!(!nvidia_awake(&root));
        write("0000-01-00.0/power/runtime_status", "active\n");
        assert!(nvidia_awake(&root));
        assert!(nvidia_awake(&root.join("missing")));
        let _ = std::fs::remove_dir_all(&root);
    }
}
