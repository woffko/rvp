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
//! The presets are PotPlayer's 19 with PotPlayer's response at the ten
//! bands, as goal L's host session measured them on tones
//! (`docs/audits/audio.md`, M10). PotPlayer's bands are broad, overlapping
//! filters, so a preset's response is not its slider positions (Full Bass
//! & Treble's sliders are 30, 20, 0, -30, -20, 10, 40, 50, 50, 50 of
//! PotPlayer's ±100); RVP's equalizer passes through the same ten points.
//! PotPlayer takes about 1.4 s to switch a preset, so each preset's 60 Hz
//! value comes from the next pass of the session; Techno, the last, has
//! its 60 Hz estimated (about 0.3 of Winamp's value, as the others show).

use std::fmt::Write as _;

use rvp_engine::Engine;
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};
use crate::audio::chain::lavfi;

/// The bands' centre frequencies in Hz.
pub const BANDS: [u32; 10] = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000];
/// The largest boost or cut in dB: PotPlayer's sliders run 0 to 200
/// around 100, and its Full Treble preset reaches +18.5 dB at 6 kHz
/// (goal L's host session).
pub const RANGE: f64 = 20.0;
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

/// PotPlayer's presets in its order (R1's live menu), each as PotPlayer's
/// response at the ten bands, measured in goal L's host session
/// (`docs/audits/audio.md`, M10; see the module comment).
pub const PRESETS: &[Preset] = &[
    preset(DEFAULT_PRESET, [0.0; 10]),
    preset(
        "Classical",
        [0.0, 0.0, 0.0, -0.1, -0.3, -2.5, -7.3, -8.3, -8.1, -7.6],
    ),
    preset("Club", [-0.1, 0.4, 1.3, 2.3, 2.3, 1.8, 1.2, 0.3, 0.2, 0.1]),
    preset(
        "Dance",
        [3.3, 3.9, 2.7, 0.7, -0.1, -2.2, -3.0, -1.8, -1.5, -1.2],
    ),
    preset(
        "Full Bass",
        [3.4, 5.7, 6.0, 4.2, 2.1, -3.4, -7.8, -9.4, -9.2, -8.6],
    ),
    preset(
        "Full Bass & Treble",
        [1.9, 5.7, 0.1, -1.9, -1.2, 5.4, 9.7, 10.0, 9.6, 9.1],
    ),
    preset(
        "Full Treble",
        [-3.1, -4.7, -4.7, -1.4, 2.0, 13.0, 18.5, 17.5, 16.6, 15.6],
    ),
    preset(
        "Laptop",
        [1.3, -4.7, 2.9, 0.0, 0.0, 4.3, 7.7, 8.7, 8.6, 8.3],
    ),
    preset(
        "Large Hall",
        [3.4, 4.9, 4.4, 2.6, 1.0, -2.2, -3.3, -2.4, -2.1, -1.7],
    ),
    preset("Live", [-1.8, 0.0, 1.1, 2.0, 2.1, 2.0, 1.6, 0.9, 0.8, 0.7]),
    preset(
        "Loudness",
        [6.1, 7.8, 6.9, 4.3, 3.1, 7.4, 11.8, 13.0, 12.8, 12.4],
    ),
    preset("Party", [2.0, 7.8, 1.2, 0.4, 0.4, 1.6, 2.8, 3.6, 3.6, 3.6]),
    preset(
        "Pop",
        [-0.6, 1.7, 2.7, 2.8, 1.9, -1.5, -3.0, -2.8, -2.6, -2.3],
    ),
    preset(
        "Reggae",
        [0.0, 1.7, -0.9, -1.8, -0.5, 2.6, 2.6, 0.9, 0.5, 0.3],
    ),
    preset(
        "Rock",
        [2.7, 1.6, -1.5, -3.1, -1.7, 5.1, 8.4, 8.2, 7.8, 7.3],
    ),
    preset("Ska", [-0.7, -1.6, -1.1, 0.1, 1.1, 4.2, 6.4, 6.2, 5.9, 5.4]),
    preset("Soft", [0.9, 0.3, -0.4, -0.9, 0.0, 5.1, 8.5, 8.8, 8.5, 8.1]),
    preset(
        "Soft Rock",
        [1.0, 1.4, 0.9, -0.4, -1.0, -1.0, 0.3, 1.7, 1.9, 2.0],
    ),
    // Techno's 60 Hz is estimated (see the module comment).
    preset(
        "Techno",
        [2.4, 2.1, 0.4, -2.0, -1.7, 2.1, 4.7, 5.2, 5.0, 4.7],
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
        eq.bands[9] = -30.0;
        assert_eq!(
            eq.gain_entry(),
            "entry(60,3.0);entry(170,0.0);entry(310,0.0);entry(600,0.0);entry(1000,0.0);\
             entry(3000,0.0);entry(6000,0.0);entry(12000,0.0);entry(14000,0.0);entry(16000,-20.0)"
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
        assert!(entry.contains(":18b=0.1000"), "{entry}");
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
