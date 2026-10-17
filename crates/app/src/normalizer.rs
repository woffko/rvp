//! The audio normalizer: TomSteady, the gain control PotPlayer's normalizer
//! runs ([`crate::tomsteady`], owner decision 2026-10-15; it replaced S5's
//! `dynaudnorm` approximation, `docs/spikes/s5-normalizer.md`).
//!
//! mpv runs no code of RVP's on its audio, so the gain control closes a loop
//! through mpv's filter chain ([`chain_entry`](Normalizer::chain_entry)):
//! ffmpeg cuts the audio into blocks ([`block_for`]: PotPlayer's block for
//! the source as the recordings measured it), `astats` measures each
//! block's peak
//! and `ametadata` prints it to mpv's log. [`Control`] reads it there on a
//! thread of its own (a weak client handle that asks for mpv's log at level
//! `v`), feeds it to TomSteady and sends the gain for the next block to the
//! `volume` filter that follows (`af-command`). In steady playback the gain
//! arrives in time for the next block (measured on libmpv: 1 block after
//! its peak was read, rarely 2), which is when TomSteady applies it too.
//! Only TomSteady's cut of a block that would clip comes one block late;
//! `alimiter` at TomSteady's ceiling holds such a block meanwhile. After a
//! seek mpv filters a few blocks at once, and the gain catches up with them.
//!
//! The gain carries over from file to file and through seeks, as in the
//! plugin; switching the normalizer on starts again at gain 1. A chain that
//! is built again (the equalizer switched, a setting changed) starts at the
//! gain the thread sent last.
//!
//! Speed changes: with only the normalizer in the chain, mpv applies a
//! speed other than 1x with a tempo filter of its own, and with S5's
//! `dynaudnorm` the return to 1x left audio and video about 0.6 s apart,
//! which a real video output paid for with dropped frames (goal H). RVP
//! keeps a `scaletempo2` of its own after the normalizer ([`TEMPO_ENTRY`]),
//! which mpv then uses for the speed: no offset, no dropped frames, no seek.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use rvp_engine::Engine;
use rvp_engine::libmpv2::Mpv;
use rvp_engine::libmpv2::events::Event;
use serde::{Deserialize, Serialize};

use crate::audio::chain::lavfi;
use crate::tomsteady::{STEPPING_RANGE, Tomsteady};

/// The label of RVP's filter in mpv's `af` chain; other filters (SVP's
/// `@svp` is video, but any user filter) are left alone.
pub const LABEL: &str = "rvpnorm";
/// The label of the tempo filter that follows the normalizer.
pub const TEMPO_LABEL: &str = "rvptempo";
/// mpv's pitch-correcting tempo filter, kept in the chain after the
/// normalizer while it is on, so that speed changes retune it.
pub const TEMPO_ENTRY: &str = "@rvptempo:scaletempo2";
/// Samples per block, as PotPlayer runs TomSteady on S5's clips and on a
/// stereo FLAC.
pub const BLOCK: u32 = 2048;

/// PotPlayer's block for a source codec, as the audio audit measured it
/// (finding 2): an AC3 frame (1536 samples), 3072 for a 5.1 PCM WAV, 2048
/// for a stereo FLAC and S5's clips. Other codecs keep 2048 until a
/// recording shows otherwise.
#[must_use]
pub fn block_for(codec: Option<&str>) -> u32 {
    match codec {
        Some("ac3" | "eac3") => 1536,
        Some(pcm) if pcm.starts_with("pcm_") => 3072,
        _ => BLOCK,
    }
}
/// The filter that prints each block's peak, and the start of its lines in
/// mpv's log.
const PEAK_INSTANCE: &str = "ametadata@rvppeak";
const PEAK_LINE: &str = "ametadata@rvppeak: lavfi.astats.Overall.Peak_level=";
/// The filter that applies the gain.
const GAIN_INSTANCE: &str = "volume@rvpgain";
/// TomSteady's ceiling, 30000 of the 16-bit full scale (-0.77 dBFS).
const CEILING: f64 = 30_000.0 / 32_768.0;
/// How long the thread waits for mpv's log before it looks at its stop flag.
const WAIT: f64 = 0.25;

/// Normalizer settings, as PotPlayer's Preferences page names them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Normalizer {
    pub enabled: bool,
    /// "Boost audio": quiet passages may be raised.
    pub boost: bool,
    /// Target peak in percent of full scale (PotPlayer's "Level"), 10-100.
    pub level: f64,
    /// PotPlayer's "Release time": TomSteady's stepping, the fractional bits
    /// of its gain. 5-10, the range of PotPlayer's slider (owner,
    /// 2026-10-16) and the plugin's; at 10 the gain moves by 1/1024 per
    /// block.
    pub release: f64,
    /// "Operate only with video playback": music plays as it is.
    pub video_only: bool,
}

impl Default for Normalizer {
    /// The owner's PotPlayer settings (R1): on, Boost on, Level 75 %,
    /// Release time 10.
    fn default() -> Self {
        Self {
            enabled: true,
            boost: true,
            level: 75.0,
            release: 10.0,
            video_only: false,
        }
    }
}

/// `value` rounded to a whole number within `low..=high`.
fn whole(value: f64, low: u32, high: u32) -> u32 {
    let value = value.round().clamp(f64::from(low), f64::from(high));
    // Clamped to a u32 range above, and whole.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = value as u32;
    value
}

impl Normalizer {
    /// The target peak in whole percent.
    #[must_use]
    pub fn level_percent(&self) -> u32 {
        whole(self.level, 1, 100)
    }

    /// TomSteady's stepping for the release time.
    #[must_use]
    pub fn stepping(&self) -> u32 {
        whole(self.release, STEPPING_RANGE.0, STEPPING_RANGE.1)
    }

    /// The `af` entry, its gain starting at `gain`: blocks of `block`
    /// samples, their peak in mpv's log, the gain [`Control`] sets, the
    /// ceiling.
    #[must_use]
    pub fn chain_entry(&self, gain: f64, block: u32) -> String {
        lavfi(
            LABEL,
            &format!(
                "asetnsamples=n={block}:p=0,\
                 astats=metadata=1:reset=1:measure_perchannel=none:measure_overall=Peak_level,\
                 {PEAK_INSTANCE}=mode=print:key=lavfi.astats.Overall.Peak_level,\
                 {GAIN_INSTANCE}=volume={gain:.9}:precision=float,\
                 alimiter=limit={CEILING:.5}:level=0"
            ),
        )
    }
}

/// A block's peak (full scale 1) from a line `ametadata` printed to mpv's
/// log.
fn block_peak(text: &str) -> Option<f64> {
    let decibels: f64 = text.strip_prefix(PEAK_LINE)?.trim().parse().ok()?;
    // A silent block measures -inf.
    Some(if decibels.is_finite() {
        10f64.powf(decibels / 20.0)
    } else {
        0.0
    })
}

/// What the thread and the application share.
#[derive(Debug)]
struct Shared {
    settings: Normalizer,
    /// Start over at gain 1 with the next block.
    restart: bool,
    /// The gain sent last.
    gain: f64,
}

/// The thread that keeps the chain's gain at TomSteady's.
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
    pub fn start(engine: &Engine, settings: Normalizer) -> Result<Self, String> {
        let client = engine
            .mpv()
            .create_weak_client(Some("rvp_normalizer"))
            .map_err(|e| format!("normalizer client: {e}"))?;
        client
            .request_log_messages("v")
            .map_err(|e| format!("normalizer log: {e}"))?;
        let shared = Arc::new(Mutex::new(Shared {
            settings,
            restart: true,
            gain: 1.0,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_shared, thread_stop) = (Arc::clone(&shared), Arc::clone(&stop));
        std::thread::Builder::new()
            .name("rvp-normalizer".to_owned())
            .spawn(move || run(&client, &thread_shared, &thread_stop))
            .map_err(|e| format!("normalizer thread: {e}"))?;
        Ok(Self { shared, stop })
    }

    /// New settings for the next block; switching the normalizer on starts
    /// again at gain 1.
    pub fn configure(&self, settings: Normalizer) {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if settings.enabled && !shared.settings.enabled {
            shared.restart = true;
            shared.gain = 1.0;
        }
        shared.settings = settings;
    }

    /// The gain sent last, for a chain built again.
    #[must_use]
    pub fn gain(&self) -> f64 {
        self.shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .gain
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        // The thread notices within `WAIT` and drops its handle.
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The thread: each block's peak in, the next block's gain out.
fn run(client: &Mpv, shared: &Mutex<Shared>, stop: &AtomicBool) {
    let mut agc = Tomsteady::new(75, 10, true);
    let mut applied = None;
    while !stop.load(Ordering::Relaxed) {
        let peak = match client.wait_event(WAIT) {
            Some(Ok(Event::Shutdown)) => return,
            Some(Ok(Event::LogMessage {
                prefix: "ffmpeg",
                text,
                ..
            })) => block_peak(text),
            _ => None,
        };
        let Some(peak) = peak else {
            continue;
        };
        let gain = {
            let mut shared = shared.lock().unwrap_or_else(PoisonError::into_inner);
            let settings = shared.settings;
            let wanted = (
                settings.level_percent(),
                settings.stepping(),
                settings.boost,
            );
            if std::mem::take(&mut shared.restart) {
                agc = Tomsteady::new(wanted.0, wanted.1, wanted.2);
            } else if applied != Some(wanted) {
                agc.set(wanted.0, wanted.1, wanted.2);
            }
            applied = Some(wanted);
            let gain = agc.block(peak);
            shared.gain = gain;
            gain
        };
        // Sent for every block: mpv builds the graph again when the audio
        // format changes, and the new one starts at the entry's gain.
        drop(client.command(
            "af-command",
            &[LABEL, "volume", &format!("{gain:.9}"), GAIN_INSTANCE],
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chain_measures_blocks_and_applies_the_gain() {
        let graph = "asetnsamples=n=2048:p=0,\
             astats=metadata=1:reset=1:measure_perchannel=none:measure_overall=Peak_level,\
             ametadata@rvppeak=mode=print:key=lavfi.astats.Overall.Peak_level,\
             volume@rvpgain=volume=1.000000000:precision=float,\
             alimiter=limit=0.91553:level=0";
        assert_eq!(
            Normalizer::default().chain_entry(1.0, BLOCK),
            format!("@rvpnorm:lavfi=graph=%{}%{graph}", graph.len())
        );
        assert!(
            Normalizer::default()
                .chain_entry(2.5, 1536)
                .contains("volume@rvpgain=volume=2.500000000:")
        );
    }

    #[test]
    fn blocks_follow_the_source() {
        assert_eq!(block_for(Some("ac3")), 1536);
        assert_eq!(block_for(Some("pcm_s16le")), 3072);
        assert_eq!(block_for(Some("flac")), 2048);
        assert_eq!(block_for(None), 2048);
    }

    #[test]
    fn potplayer_settings_map_to_tomsteady() {
        let normalizer = Normalizer::default();
        assert_eq!(
            (normalizer.level_percent(), normalizer.stepping()),
            (75, 10)
        );
        let odd = Normalizer {
            level: 60.4,
            release: 50.0,
            ..Normalizer::default()
        };
        assert_eq!((odd.level_percent(), odd.stepping()), (60, 10));
        let fast = Normalizer {
            release: 1.0,
            ..Normalizer::default()
        };
        assert_eq!(fast.stepping(), 5);
    }

    #[test]
    fn peaks_come_from_ametadata_lines() {
        let peak = block_peak("ametadata@rvppeak: lavfi.astats.Overall.Peak_level=-6.020600\n")
            .expect("a peak");
        assert!((peak - 0.5).abs() < 1e-6, "{peak}");
        assert_eq!(
            block_peak("ametadata@rvppeak: lavfi.astats.Overall.Peak_level=-inf\n"),
            Some(0.0)
        );
        assert_eq!(
            block_peak("ametadata@rvppeak: frame:3 pts:6144 pts_time:0.128\n"),
            None
        );
        assert_eq!(
            block_peak("Parsed_ametadata_2: lavfi.astats.Overall.Peak_level=-1.0\n"),
            None
        );
    }
}
