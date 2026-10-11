//! The audio normalizer with boost (S5, `docs/spikes/s5-normalizer.md`).
//!
//! PotPlayer's normalizer ("Tomsteady 32-bit floating point arithmetic")
//! is approximated with ffmpeg's `dynaudnorm` in mpv's audio filter chain.
//! With PotPlayer's defaults (Level 75 %, Release time 10, Boost on) the
//! chain is `dynaudnorm=f=5000:g=3:p=1.0:m=5`, the closest of about 2300
//! candidates. Boost sets the maximum gain (`m=5`, up to +14 dB; off:
//! `m=1`, the normalizer only lowers loud passages). Level and Release time
//! map to the target peak and the frame length around those defaults; the
//! owner's blind A/B test (`tools/s5/ab_test.py`) decides whether the
//! defaults are close enough.
//!
//! In a player, two things differ from S5's offline measurement. mpv
//! restarts the filter at every seek, and `dynaudnorm` assumes a gain of 1
//! for the frames it has not seen, so the level climbed from the raw level
//! for about ten seconds after each seek; `b=1` (its alternative boundary
//! mode) starts at the computed gain instead. And the filter looks about
//! ten seconds ahead: mpv decodes that much audio before playing on after a
//! seek (about 50 ms more per seek on a local file), and changing the chain
//! during playback dropped that audio, so a toggle seeks to the current
//! position (`App`).

use serde::{Deserialize, Serialize};

/// The label of RVP's filter in mpv's `af` chain; other filters (SVP's
/// `@svp` is video, but any user filter) are left alone.
pub const LABEL: &str = "rvpnorm";

/// Normalizer settings, as PotPlayer's Preferences page names them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Normalizer {
    pub enabled: bool,
    /// "Boost audio": quiet passages may be raised.
    pub boost: bool,
    /// Target level in percent (PotPlayer's "Level"), 10-100.
    pub level: f64,
    /// PotPlayer's "Release time", 1-50: how slowly the gain follows.
    pub release: f64,
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
        }
    }
}

impl Normalizer {
    /// The `dynaudnorm` filter for these settings.
    #[must_use]
    pub fn filter(&self) -> String {
        // Level 75 % is S5's p=1.0 (peak at full scale); lower levels
        // lower the peak target proportionally.
        let peak = (self.level.clamp(10.0, 100.0) / 75.0).min(1.0);
        // Release time 10 is S5's 5000 ms frame; the frame scales with it.
        let frame = (self.release.clamp(1.0, 50.0) * 500.0).round();
        let max_gain = if self.boost { 5 } else { 1 };
        format!("dynaudnorm=f={frame:.0}:g=3:p={peak:.2}:m={max_gain}:b=1")
    }

    /// The `af` entry: label and filter.
    #[must_use]
    pub fn chain_entry(&self) -> String {
        format!("@{LABEL}:lavfi=[{}]", self.filter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_s5_chain() {
        assert_eq!(
            Normalizer::default().filter(),
            "dynaudnorm=f=5000:g=3:p=1.00:m=5:b=1"
        );
        let without_boost = Normalizer {
            boost: false,
            ..Normalizer::default()
        };
        assert_eq!(
            without_boost.filter(),
            "dynaudnorm=f=5000:g=3:p=1.00:m=1:b=1"
        );
        let quieter = Normalizer {
            level: 50.0,
            release: 4.0,
            ..Normalizer::default()
        };
        assert_eq!(quieter.filter(), "dynaudnorm=f=2000:g=3:p=0.67:m=5:b=1");
        assert_eq!(
            Normalizer::default().chain_entry(),
            "@rvpnorm:lavfi=[dynaudnorm=f=5000:g=3:p=1.00:m=5:b=1]"
        );
    }
}
