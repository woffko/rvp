//! The equalizer (M6): ten bands at the frequencies BlackSpace's control
//! panel shows (60 Hz to 16 kHz), ±12 dB each, and named presets.
//!
//! It is one FFmpeg `firequalizer` in mpv's audio chain under the label
//! `@rvpeq`, put in front of the chain so that it runs before the
//! normalizer (which then levels the equalized sound). Band changes go to
//! the running filter as a command (`af-command`, aimed at the filter's
//! instance name, since mpv reports a failure for the default target
//! `all`), so the chain is not rebuilt and the normalizer keeps its ten
//! seconds of look-ahead; switching the equalizer on or off rebuilds the
//! chain and plays on from the current position, as a normalizer toggle
//! does. Choosing a preset or moving a band switches it on.

use std::fmt::Write as _;

use rvp_engine::{Engine, Seek};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// The bands' centre frequencies in Hz.
pub const BANDS: [u32; 10] = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000];
/// The largest boost or cut in dB.
pub const RANGE: f64 = 12.0;
/// The filter's label in mpv's `af` chain.
pub const LABEL: &str = "rvpeq";
/// The filter's instance name inside the label's graph.
const INSTANCE: &str = "firequalizer@eq";

/// Equalizer settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Equalizer {
    pub enabled: bool,
    /// Gains in dB, one per band of [`BANDS`].
    pub bands: [f64; 10],
    /// The preset the bands came from; empty after a band was changed.
    pub preset: String,
}

impl Default for Equalizer {
    fn default() -> Self {
        Self {
            enabled: false,
            bands: [0.0; 10],
            preset: "Flat".to_owned(),
        }
    }
}

/// A named set of band gains.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    pub bands: [f64; 10],
}

/// RVP's presets (its own curves, named as PotPlayer's are).
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Flat",
        bands: [0.0; 10],
    },
    Preset {
        name: "Bass Boost",
        bands: [6.0, 5.0, 3.5, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    },
    Preset {
        name: "Treble Boost",
        bands: [0.0, 0.0, 0.0, 0.0, 0.0, 1.5, 3.0, 5.0, 5.5, 6.0],
    },
    Preset {
        name: "Bass and Treble",
        bands: [5.5, 4.5, 2.0, 0.0, -1.0, 0.0, 1.5, 4.0, 4.5, 5.0],
    },
    Preset {
        name: "Classical",
        bands: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.5, -4.5, -4.5, -6.0],
    },
    Preset {
        name: "Dance",
        bands: [6.0, 4.5, 1.5, 0.0, 0.0, -2.0, -3.0, -3.0, 0.0, 0.0],
    },
    Preset {
        name: "Headphones",
        bands: [3.0, 6.0, 3.0, -2.0, -1.0, 1.0, 3.0, 5.5, 7.0, 7.5],
    },
    Preset {
        name: "Large Hall",
        bands: [6.5, 6.5, 3.5, 3.5, 0.0, -3.0, -3.0, -3.0, 0.0, 0.0],
    },
    Preset {
        name: "Live",
        bands: [-3.0, 0.0, 2.5, 3.0, 3.5, 3.5, 2.5, 1.5, 1.5, 1.0],
    },
    Preset {
        name: "Pop",
        bands: [-1.0, 3.0, 4.5, 5.0, 3.5, 0.0, -1.0, -1.0, -1.0, -1.0],
    },
    Preset {
        name: "Rock",
        bands: [5.0, 3.0, -3.5, -5.0, -2.0, 2.5, 5.5, 7.0, 7.0, 7.0],
    },
    Preset {
        name: "Soft",
        bands: [3.0, 1.0, 0.0, -1.0, 0.0, 2.5, 5.0, 6.0, 7.0, 7.5],
    },
    Preset {
        name: "Vocal",
        bands: [-2.0, -2.5, 0.0, 3.0, 4.5, 4.5, 3.0, 1.0, 0.0, -1.0],
    },
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

    /// The `af` entry.
    #[must_use]
    pub fn chain_entry(&self) -> String {
        format!(
            "@{LABEL}:lavfi=[{INSTANCE}=gain_entry='{}']",
            self.gain_entry()
        )
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
    /// Adds or removes the equalizer filter to match the settings.
    pub fn apply_equalizer(&self, engine: &Engine) {
        let eq = &self.settings.equalizer;
        let entry = eq.enabled.then(|| eq.chain_entry());
        if let Err(error) = engine.set_audio_filter_first(LABEL, entry.as_deref()) {
            eprintln!("rvp: cannot set the equalizer: {error}");
        }
    }

    /// Switches the equalizer on or off.
    pub fn toggle_equalizer(&mut self, engine: &Engine) -> UiRequest {
        self.settings.equalizer.enabled = !self.settings.equalizer.enabled;
        self.equalizer_chain_changed(engine);
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

    pub(crate) fn equalizer_chain_changed(&self, engine: &Engine) {
        self.apply_equalizer(engine);
        // Rebuilding the chain drops what the normalizer had decoded ahead.
        if self.settings.normalizer.enabled && self.model.media_loaded() {
            drop(engine.seek(Seek::Relative(0.0)));
        }
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
            self.equalizer_chain_changed(engine);
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
            self.equalizer_chain_changed(engine);
        }
        self.save_settings();
    }

    /// Sends the band gains to the running filter.
    pub fn update_equalizer_bands(&self, engine: &Engine) {
        let gains = self.settings.equalizer.gain_entry();
        if engine
            .command("af-command", &[LABEL, "gain_entry", &gains, INSTANCE])
            .is_err()
        {
            // Not running yet (no audio): rebuild it with the new gains.
            self.apply_equalizer(engine);
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
            eq.chain_entry()
                .starts_with("@rvpeq:lavfi=[firequalizer@eq=gain_entry='entry(60,3.0);")
        );
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
        assert_eq!(PRESETS[0].name, "Flat");
    }
}
