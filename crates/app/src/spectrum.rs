//! The audio skins' spectrum (M14): a skin's `Visual` element (BlackSpace's
//! audio skin and fourteen more of PotPlayer's) draws the audio's bands as
//! bars (`rvp_skin`'s paint rules).
//!
//! mpv hands RVP no samples, so the bands come through its filter chain, as
//! the normalizer's peaks do: `@rvpspectrum` ([`chain_entry`]) splits off a
//! mono copy of the audio in blocks of [`BLOCK`] samples, filters it into
//! [`BANDS`] bands spaced evenly in octaves (three `bandpass` filters in a
//! row each: one alone leaks a tone into every band), merges them into
//! one stream of as many channels (`amerge`) and lets `astats` measure each
//! channel's RMS level, which `ametadata` prints to mpv's log; the copy ends
//! in `anullsink`, and the audio itself passes untouched. [`Control`] reads
//! the lines on a thread of its own (a weak client that asks for mpv's log
//! at level `v`) and keeps each band's last level. The window's thread
//! takes them a few dozen times a second ([`Bars::tick`]), when the bars
//! rise at once and fall at [`FALL_PER_SECOND`], their peaks slower.
//!
//! The tap is in the chain only while a skin with a `Visual` element shows
//! (`App::set_skin_visual`), so files with video pay nothing for it.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rvp_engine::Engine;
use rvp_engine::libmpv2::Mpv;
use rvp_engine::libmpv2::events::Event;

use crate::audio::chain::lavfi;

/// The label of the tap in mpv's `af` chain.
pub const LABEL: &str = "rvpspectrum";
/// The bands, from [`LOW`] to [`HIGH`] Hz.
pub const BANDS: usize = 16;
const LOW: f64 = 50.0;
const HIGH: f64 = 16_000.0;
/// Samples per measurement: about 23 a second at 48 kHz.
pub const BLOCK: u32 = 2048;
/// The levels a bar spans, in dB of full scale: below [`FLOOR_DB`] it is
/// dark, from [`TOP_DB`] full.
const FLOOR_DB: f64 = -66.0;
const TOP_DB: f64 = -12.0;
/// How fast a bar and a peak fall, in bar heights a second.
pub const FALL_PER_SECOND: f32 = 2.5;
const PEAK_FALL_PER_SECOND: f32 = 0.8;
/// Levels older than this are silence (paused, stopped, between files).
const STALE: Duration = Duration::from_millis(250);
/// The filter that prints the levels, and the start of its lines.
const LINE: &str = "ametadata@rvpspectrum: lavfi.astats.";
const WAIT: f64 = 0.25;

/// The bands' centre frequencies, low to high.
#[must_use]
pub fn centres() -> [f64; BANDS] {
    let mut centres = [0.0; BANDS];
    let steps = f64::from(u32::try_from(BANDS - 1).unwrap_or(1));
    for (index, centre) in (0u32..).zip(centres.iter_mut()) {
        *centre = LOW * (HIGH / LOW).powf(f64::from(index) / steps);
    }
    centres
}

/// The tap's `af` entry.
#[must_use]
pub fn chain_entry() -> String {
    let width = (HIGH / LOW).log2() / f64::from(u32::try_from(BANDS - 1).unwrap_or(1));
    let mut graph = format!(
        "asplit=2[rvpout][rvpv];[rvpv]aformat=channel_layouts=mono,asetnsamples=n={BLOCK}:p=0,asplit={BANDS}"
    );
    for band in 0..BANDS {
        let _ = write!(graph, "[rvpb{band}]");
    }
    graph.push(';');
    for (band, centre) in centres().iter().enumerate() {
        let filter = format!("bandpass=f={centre:.1}:width_type=o:width={width:.3}");
        let _ = write!(graph, "[rvpb{band}]{filter},{filter},{filter}[rvpc{band}];");
    }
    for band in 0..BANDS {
        let _ = write!(graph, "[rvpc{band}]");
    }
    let _ = write!(
        graph,
        "amerge=inputs={BANDS},astats=metadata=1:reset=1:measure_perchannel=RMS_level:measure_overall=none,\
         ametadata@rvpspectrum=mode=print,anullsink"
    );
    lavfi(LABEL, &graph)
}

/// A band's level from 0 to 1 from an `ametadata` line, with its index.
fn band_level(text: &str) -> Option<(usize, f32)> {
    let rest = text.strip_prefix(LINE)?;
    let (channel, rest) = rest.split_once('.')?;
    let value = rest.strip_prefix("RMS_level=")?.trim();
    let band = channel.parse::<usize>().ok()?.checked_sub(1)?;
    if band >= BANDS {
        return None;
    }
    let decibels: f64 = value.parse().ok()?;
    let level = if decibels.is_finite() {
        ((decibels - FLOOR_DB) / (TOP_DB - FLOOR_DB)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // A fraction from 0 to 1 fits f32.
    #[allow(clippy::cast_possible_truncation)]
    Some((band, level as f32))
}

/// What the thread hands the window's thread.
#[derive(Debug)]
struct Shared {
    levels: [f32; BANDS],
    at: Option<Instant>,
}

/// The thread that reads the bands from mpv's log.
#[derive(Debug)]
pub struct Control {
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
}

impl Control {
    /// Starts the thread on a weak client handle of `engine`'s player.
    ///
    /// # Errors
    /// Fails when libmpv refuses the handle or its log messages.
    pub fn start(engine: &Engine) -> Result<Self, String> {
        let client = engine
            .mpv()
            .create_weak_client(Some("rvp_spectrum"))
            .map_err(|e| format!("spectrum client: {e}"))?;
        client
            .request_log_messages("v")
            .map_err(|e| format!("spectrum log: {e}"))?;
        let shared = Arc::new(Mutex::new(Shared {
            levels: [0.0; BANDS],
            at: None,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_shared, thread_stop) = (Arc::clone(&shared), Arc::clone(&stop));
        std::thread::Builder::new()
            .name("rvp-spectrum".to_owned())
            .spawn(move || run(&client, &thread_shared, &thread_stop))
            .map_err(|e| format!("spectrum thread: {e}"))?;
        Ok(Self { shared, stop })
    }

    /// The bands' last levels, or silence when none came lately.
    #[must_use]
    pub fn levels(&self, now: Instant) -> [f32; BANDS] {
        let shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        match shared.at {
            Some(at) if now.saturating_duration_since(at) < STALE => shared.levels,
            _ => [0.0; BANDS],
        }
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        // The thread notices within `WAIT` and drops its handle.
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn run(client: &Mpv, shared: &Mutex<Shared>, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        let level = match client.wait_event(WAIT) {
            Some(Ok(Event::Shutdown)) => return,
            Some(Ok(Event::LogMessage {
                prefix: "ffmpeg",
                text,
                ..
            })) => band_level(text),
            _ => None,
        };
        if let Some((band, level)) = level {
            let mut shared = shared.lock().unwrap_or_else(PoisonError::into_inner);
            shared.levels[band] = level;
            shared.at = Some(Instant::now());
        }
    }
}

/// The bars as they show: they rise to a band's level at once and fall
/// slowly, and each keeps a peak that falls slower still.
#[derive(Debug, Clone, Default)]
pub struct Bars {
    pub levels: Vec<f32>,
    pub peaks: Vec<f32>,
    last: Option<Instant>,
}

impl Bars {
    /// The bars at `now` for the bands' `measured` levels; true when they
    /// changed.
    pub fn tick(&mut self, measured: &[f32], now: Instant) -> bool {
        // A tick's seconds stay far inside f32's precision.
        #[allow(clippy::cast_possible_truncation)]
        let seconds = self
            .last
            .map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f64()
            })
            .min(1.0) as f32;
        self.last = Some(now);
        if self.levels.len() != measured.len() {
            self.levels = vec![0.0; measured.len()];
            self.peaks = vec![0.0; measured.len()];
        }
        let before = (self.levels.clone(), self.peaks.clone());
        for ((shown, peak), new) in self.levels.iter_mut().zip(&mut self.peaks).zip(measured) {
            *shown = new.max(*shown - FALL_PER_SECOND * seconds).max(0.0);
            *peak = shown.max(*peak - PEAK_FALL_PER_SECOND * seconds);
        }
        (self.levels.clone(), self.peaks.clone()) != before
    }

    /// All bars dark.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_come_from_ametadata_lines() {
        assert_eq!(
            band_level("ametadata@rvpspectrum: lavfi.astats.1.RMS_level=-12.000000\n"),
            Some((0, 1.0))
        );
        let (band, level) = band_level("ametadata@rvpspectrum: lavfi.astats.16.RMS_level=-39.0\n")
            .expect("a level");
        assert_eq!(band, 15);
        assert!((level - 0.5).abs() < 1e-6, "{level}");
        assert_eq!(
            band_level("ametadata@rvpspectrum: lavfi.astats.3.RMS_level=-inf\n"),
            Some((2, 0.0))
        );
        assert_eq!(
            band_level("ametadata@rvpspectrum: frame:3 pts:6144 pts_time:0.128\n"),
            None
        );
        assert_eq!(
            band_level("ametadata@rvpspectrum: lavfi.astats.17.RMS_level=-1\n"),
            None
        );
        assert_eq!(
            band_level("ametadata@rvppeak: lavfi.astats.Overall.Peak_level=-6\n"),
            None
        );
    }

    #[test]
    fn the_graph_measures_every_band() {
        let entry = chain_entry();
        assert!(entry.starts_with("@rvpspectrum:lavfi=graph=%"), "{entry}");
        assert!(entry.contains(&format!("amerge=inputs={BANDS}")));
        assert_eq!(entry.matches("bandpass=").count(), 3 * BANDS);
        assert!(entry.ends_with("anullsink"));
        let centres = centres();
        assert!((centres[0] - LOW).abs() < 1e-9 && (centres[BANDS - 1] - HIGH).abs() < 1e-6);
    }

    #[test]
    fn bars_rise_at_once_and_fall_slowly() {
        let mut bars = Bars::default();
        let start = Instant::now();
        assert!(bars.tick(&[1.0, 0.5], start));
        assert_eq!(bars.levels, [1.0, 0.5]);
        // A tenth of a second of silence: 0.25 lower; the peaks 0.08.
        bars.tick(&[0.0, 0.0], start + Duration::from_millis(100));
        assert!((bars.levels[0] - 0.75).abs() < 1e-5, "{:?}", bars.levels);
        assert!((bars.peaks[0] - 0.92).abs() < 1e-5, "{:?}", bars.peaks);
        // Long enough: all dark, and nothing changes any more.
        bars.tick(&[0.0, 0.0], start + Duration::from_secs(3));
        bars.tick(&[0.0, 0.0], start + Duration::from_secs(4));
        assert!(!bars.tick(&[0.0, 0.0], start + Duration::from_secs(5)));
        assert_eq!(bars.levels, [0.0, 0.0]);
    }
}
