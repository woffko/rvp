//! The equalizer (M6, M10): ten bands at the frequencies BlackSpace's
//! control panel shows (60 Hz to 16 kHz), ±12 dB each, and PotPlayer's
//! presets.
//!
//! It is one FFmpeg `firequalizer` in mpv's audio chain under the label
//! `@rvpeq`, before the effects and the normalizer (which then levels the
//! equalized sound; `crate::audio::chain`). Its filter is 20 ms long
//! (`delay=0.02`, twice FFmpeg's default), which brings the presets' steep
//! low bands to within 1.5 dB of their values (the default's 10 ms missed
//! Full Bass's 60 Hz by 3 dB, `tests/audio_chain.rs`); the sound comes
//! 20 ms late for it, which mpv's sync does not see. Band changes go to the running
//! filter as a command (`af-command`, aimed at the filter's instance name,
//! since mpv reports a failure for the default target `all`), so the chain
//! is not rebuilt and the normalizer keeps its ten seconds of look-ahead;
//! switching the equalizer on or off rebuilds the chain and plays on from
//! the current position, as a normalizer toggle does (in the last moments
//! of a file both wait for the next file, `App::defer_chain_change`).
//! Choosing a preset or moving a band switches it on.
//!
//! PotPlayer's "SuperEQ" is Naoki Shibata's equalizer, which FFmpeg has as
//! `superequalizer` (18 bands from 65 Hz to 20 kHz): with it on, the ten
//! bands are interpolated to its eighteen, and band changes rebuild the
//! chain.
//!
//! The presets are PotPlayer's names; their values are Winamp's presets,
//! whose names PotPlayer's are (Loudness is RVP's own curve), clipped to
//! the bands' range until the audio audit has measured PotPlayer's.

use std::fmt::Write as _;

use rvp_engine::Engine;
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::audio::chain::lavfi;

/// The bands' centre frequencies in Hz.
pub const BANDS: [u32; 10] = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000];
/// The largest boost or cut in dB.
pub const RANGE: f64 = 12.0;
/// The filter's label in mpv's `af` chain.
pub const LABEL: &str = "rvpeq";
/// The filter's instance name inside the label's graph.
const INSTANCE: &str = "firequalizer@eq";
/// `superequalizer`'s band frequencies in Hz.
const SUPER_BANDS: [f64; 18] = [
    65.0, 92.0, 131.0, 185.0, 262.0, 370.0, 523.0, 740.0, 1047.0, 1480.0, 2093.0, 2960.0, 4186.0,
    5920.0, 8372.0, 11840.0, 16744.0, 20000.0,
];

/// Equalizer settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Equalizer {
    pub enabled: bool,
    /// Gains in dB, one per band of [`BANDS`].
    pub bands: [f64; 10],
    /// The preset the bands came from; empty after a band was changed.
    pub preset: String,
    /// PotPlayer's "SuperEQ" (FFmpeg's `superequalizer`).
    pub superequalizer: bool,
}

impl Default for Equalizer {
    fn default() -> Self {
        Self {
            enabled: false,
            bands: [0.0; 10],
            preset: DEFAULT_PRESET.to_owned(),
            superequalizer: false,
        }
    }
}

/// The flat preset's name, PotPlayer's first.
pub const DEFAULT_PRESET: &str = "(Default)";

/// A named set of band gains.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    pub bands: [f64; 10],
}

/// A preset's bands from Winamp's values (dB), clipped to the range.
const fn preset(name: &'static str, bands: [f64; 10]) -> Preset {
    let mut clipped = bands;
    let mut i = 0;
    while i < 10 {
        if clipped[i] > RANGE {
            clipped[i] = RANGE;
        } else if clipped[i] < -RANGE {
            clipped[i] = -RANGE;
        }
        i += 1;
    }
    Preset {
        name,
        bands: clipped,
    }
}

/// PotPlayer's presets, in its order (R1's live menu).
pub const PRESETS: &[Preset] = &[
    preset(DEFAULT_PRESET, [0.0; 10]),
    preset(
        "Classical",
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -7.2, -7.2, -7.2, -9.6],
    ),
    preset("Club", [0.0, 0.0, 8.0, 5.6, 5.6, 5.6, 3.2, 0.0, 0.0, 0.0]),
    preset(
        "Dance",
        [9.6, 7.2, 2.4, 0.0, 0.0, -5.6, -7.2, -7.2, 0.0, 0.0],
    ),
    preset(
        "Full Bass",
        [-8.0, 9.6, 9.6, 5.6, 1.6, -4.0, -8.0, -10.4, -11.2, -11.2],
    ),
    preset(
        "Full Bass & Treble",
        [7.2, 5.6, 0.0, -7.2, -4.8, 1.6, 8.0, 11.2, 12.0, 12.0],
    ),
    preset(
        "Full Treble",
        [-9.6, -9.6, -9.6, -4.0, 2.4, 11.2, 16.0, 16.0, 16.0, 16.8],
    ),
    preset(
        "Laptop",
        [4.8, 11.2, 5.6, -3.2, -2.4, 1.6, 4.8, 9.6, 12.8, 14.4],
    ),
    preset(
        "Large Hall",
        [10.4, 10.4, 5.6, 5.6, 0.0, -4.8, -4.8, -4.8, 0.0, 0.0],
    ),
    preset("Live", [-4.8, 0.0, 4.0, 5.6, 5.6, 5.6, 4.0, 2.4, 2.4, 2.4]),
    preset(
        "Loudness",
        [6.0, 4.0, 1.0, 0.0, -1.0, 0.0, 1.0, 3.0, 4.0, 5.0],
    ),
    preset("Party", [7.2, 7.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 7.2, 7.2]),
    preset(
        "Pop",
        [-1.6, 4.8, 7.2, 8.0, 5.6, 0.0, -2.4, -2.4, -1.6, -1.6],
    ),
    preset(
        "Reggae",
        [0.0, 0.0, 0.0, -5.6, 0.0, 6.4, 6.4, 0.0, 0.0, 0.0],
    ),
    preset(
        "Rock",
        [8.0, 4.8, -5.6, -8.0, -3.2, 4.0, 8.8, 11.2, 11.2, 11.2],
    ),
    preset(
        "Ska",
        [-2.4, -4.8, -4.0, 0.0, 4.0, 5.6, 8.8, 9.6, 11.2, 9.6],
    ),
    preset(
        "Soft",
        [4.8, 1.6, 0.0, -2.4, 0.0, 4.0, 8.0, 9.6, 11.2, 12.0],
    ),
    preset(
        "Soft Rock",
        [4.0, 4.0, 2.4, 0.0, -4.0, -5.6, -3.2, 0.0, 2.4, 8.8],
    ),
    preset(
        "Techno",
        [8.0, 5.6, 0.0, -5.6, -4.8, 0.0, 8.0, 9.6, 9.6, 8.8],
    ),
];

impl Equalizer {
    /// `firequalizer`'s `gain_entry`: the bands as `entry(Hz,dB)` points.
    #[must_use]
    pub fn gain_entry(&self) -> String {
        let mut text = String::new();
        for (frequency, gain) in BANDS.iter().zip(self.bands) {
            if !text.is_empty() {
                text.push(';');
            }
            let _ = write!(text, "entry({frequency},{:.1})", gain.clamp(-RANGE, RANGE));
        }
        text
    }

    /// A band's gain at `frequency`, interpolated between the bands on a
    /// logarithmic frequency scale and held beyond the outer ones.
    #[must_use]
    pub fn gain_at(&self, frequency: f64) -> f64 {
        let bands = BANDS.map(f64::from);
        if frequency <= bands[0] {
            return self.bands[0];
        }
        for i in 1..bands.len() {
            if frequency <= bands[i] {
                let t = (frequency.ln() - bands[i - 1].ln()) / (bands[i].ln() - bands[i - 1].ln());
                return self.bands[i - 1] + t * (self.bands[i] - self.bands[i - 1]);
            }
        }
        self.bands[bands.len() - 1]
    }

    /// The `af` entry: `firequalizer`, or `superequalizer` with the bands
    /// interpolated to its eighteen (gains as factors).
    #[must_use]
    pub fn chain_entry(&self) -> String {
        if self.superequalizer {
            let mut graph = String::from("superequalizer=");
            for (index, frequency) in SUPER_BANDS.iter().enumerate() {
                let gain = self.gain_at(*frequency).clamp(-RANGE, RANGE);
                let factor = 10f64.powf(gain / 20.0);
                if index > 0 {
                    graph.push(':');
                }
                let _ = write!(graph, "{}b={factor:.4}", index + 1);
            }
            lavfi(LABEL, &graph)
        } else {
            lavfi(
                LABEL,
                &format!("{INSTANCE}=delay=0.02:gain_entry='{}'", self.gain_entry()),
            )
        }
    }

    /// The text the `CMD_EQ_PRESET_MESSAGE` label shows.
    #[must_use]
    pub fn preset_text(&self) -> &str {
        if self.preset.is_empty() {
            "Custom"
        } else {
            &self.preset
        }
    }
}

impl App {
    /// Switches the equalizer on or off.
    pub fn toggle_equalizer(&mut self, engine: &Engine) -> UiRequest {
        self.settings.equalizer.enabled = !self.settings.equalizer.enabled;
        self.audio_chain_changed(engine);
        self.save_settings();
        UiRequest::Osd(format!(
            "Equalizer: {}",
            if self.settings.equalizer.enabled {
                self.settings.equalizer.preset_text().to_owned()
            } else {
                "off".to_owned()
            }
        ))
    }

    /// Chooses a preset by its index in [`PRESETS`].
    pub fn set_equalizer_preset(&mut self, engine: &Engine, index: usize) -> Option<UiRequest> {
        let preset = PRESETS.get(index)?;
        let eq = &mut self.settings.equalizer;
        eq.bands = preset.bands;
        preset.name.clone_into(&mut eq.preset);
        let was_on = eq.enabled;
        eq.enabled = true;
        if was_on {
            self.update_equalizer_bands(engine);
        } else {
            self.audio_chain_changed(engine);
        }
        self.save_settings();
        Some(UiRequest::Osd(format!("Equalizer: {}", preset.name)))
    }

    /// Sets one band (dB) from a slider and switches the equalizer on.
    pub fn set_equalizer_band(&mut self, engine: &Engine, band: usize, gain: f64) {
        let eq = &mut self.settings.equalizer;
        let Some(slot) = eq.bands.get_mut(band) else {
            return;
        };
        *slot = (gain.clamp(-RANGE, RANGE) * 10.0).round() / 10.0;
        eq.preset.clear();
        let was_on = eq.enabled;
        eq.enabled = true;
        if was_on {
            self.update_equalizer_bands(engine);
        } else {
            self.audio_chain_changed(engine);
        }
        self.save_settings();
    }

    /// Sends the band gains to the running filter (`superequalizer` takes
    /// no commands: its chain is rebuilt).
    pub fn update_equalizer_bands(&mut self, engine: &Engine) {
        let gains = self.settings.equalizer.gain_entry();
        if (self.settings.equalizer.superequalizer
            || engine
                .command("af-command", &[LABEL, "gain_entry", &gains, INSTANCE])
                .is_err())
            && !self.defer_chain_change()
        {
            // Not running yet (no audio), or SuperEQ: rebuild the chain.
            self.audio_chain_changed(engine);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_lists_every_band() {
        let mut eq = Equalizer::default();
        eq.bands[0] = 3.0;
        eq.bands[9] = -20.0;
        assert_eq!(
            eq.gain_entry(),
            "entry(60,3.0);entry(170,0.0);entry(310,0.0);entry(600,0.0);entry(1000,0.0);\
             entry(3000,0.0);entry(6000,0.0);entry(12000,0.0);entry(14000,0.0);entry(16000,-12.0)"
        );
        assert!(
            eq.chain_entry().contains("lavfi=graph=%"),
            "{}",
            eq.chain_entry()
        );
        assert!(
            eq.chain_entry()
                .contains("%firequalizer@eq=delay=0.02:gain_entry='entry(60,3.0);")
        );
        eq.superequalizer = true;
        let entry = eq.chain_entry();
        assert!(entry.contains("superequalizer=1b=1.3755:"), "{entry}");
        assert!(entry.contains(":18b=0.2512"), "{entry}");
    }

    #[test]
    fn bands_interpolate_on_a_log_scale() {
        let mut eq = Equalizer::default();
        eq.bands[4] = 6.0;
        assert!((eq.gain_at(1000.0) - 6.0).abs() < 1e-12);
        assert!((eq.gain_at(20.0)).abs() < 1e-12);
        let between = eq.gain_at((600.0f64 * 1000.0).sqrt());
        assert!((between - 3.0).abs() < 1e-9, "{between}");
    }

    #[test]
    fn presets_have_unique_names_and_stay_in_range() {
        let mut names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PRESETS.len());
        assert!(
            PRESETS
                .iter()
                .all(|p| p.bands.iter().all(|g| g.abs() <= RANGE))
        );
        assert_eq!(PRESETS[0].name, DEFAULT_PRESET);
        assert_eq!(PRESETS.len(), 19, "PotPlayer's nineteen");
        assert!(PRESETS[0].bands.iter().all(|g| *g == 0.0));
    }
}
