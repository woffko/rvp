//! TomSteady, the automatic gain control behind PotPlayer's normalizer
//! (method "Tomsteady 32-bit floating point arithmetic").
//!
//! TomSteady is Tom St Denis's freeware Winamp DSP plugin (2001; he
//! published its source, which is no longer online). Its gain control was
//! reconstructed from version 1.4 of the plugin (owner decision
//! 2026-10-15) and checked against spike S5's recordings of PotPlayer: run
//! over blocks of 2048 samples with a stepping of 10 (PotPlayer's "Release
//! time 10") and a level of 75 %, it follows PotPlayer's gain within 0.1 dB
//! RMS (`docs/spikes/s5-normalizer.md`).
//!
//! The rules, once per block, with levels on the 16-bit scale (full scale
//! 32768) as the plugin has them:
//!
//! * The gain is a fixed-point number, `vol / 2^stepping`, starting at 1.
//! * While the block's peak times the gain is above 30000, the gain drops by
//!   `stepping` units. After 1024 drops the block is left as it was.
//! * A block whose peak after the gain is 10 or less leaves the gain alone.
//! * Any other block asks for one unit up (its peak is at most the level, a
//!   percentage of full scale) or one unit down. The last 12 requests select
//!   one of 4096 counters (0-15, all starting at 7), as in a CPU's branch
//!   predictor; the request counts there and moves the gain only when the
//!   counter then agrees with it: 8 or more for up, 7 or less for down. The
//!   gain does not drop below 2 units.
//! * After 128 raises in a row each raise is `stepping` units.
//! * Without boost the gain does not go above 1.
//!
//! The gain applies to the next block; the drops before a block plays apply
//! to that block, which RVP cannot do inside mpv (`crate::normalizer`).

/// Full scale of the 16-bit levels the rules are written in.
const FULL_SCALE: f64 = 32_768.0;
/// The highest peak a block may have after the gain.
const CLIP: f64 = 30_000.0;
/// Blocks this quiet after the gain leave it alone.
const QUIET: f64 = 10.0;
/// Bits of request history: they select one of the counters.
const HISTORY_BITS: u32 = 12;
const COUNTERS: usize = 1 << HISTORY_BITS;
const COUNTER_START: u8 = 7;
const COUNTER_MAX: u8 = 15;
/// Raises in a row after which a raise is `stepping` units.
const FAST_AFTER: u32 = 128;
/// Gain drops tried for one block before it is left as it was.
const MAX_DROPS: u32 = 1024;

/// The lowest and highest stepping (fractional bits of the gain) the
/// plugin's slider offers.
pub const STEPPING_RANGE: (u32, u32) = (5, 10);

/// One TomSteady gain control.
#[derive(Debug, Clone)]
pub struct Tomsteady {
    /// Target peak in percent of full scale.
    level: u32,
    /// Fractional bits of the gain.
    stepping: u32,
    boost: bool,
    /// The gain in units of `2^-stepping`.
    vol: i64,
    counters: Vec<u8>,
    /// The last requests, newest in the top bit (1 = up).
    history: usize,
    /// Raises in a row.
    raises: u32,
}

impl Tomsteady {
    /// A gain control at gain 1. `level` is in percent (1-100), `stepping`
    /// within [`STEPPING_RANGE`].
    #[must_use]
    pub fn new(level: u32, stepping: u32, boost: bool) -> Self {
        let stepping = stepping.clamp(STEPPING_RANGE.0, STEPPING_RANGE.1);
        Self {
            level: level.clamp(1, 100),
            stepping,
            boost,
            vol: 1 << stepping,
            counters: vec![COUNTER_START; COUNTERS],
            history: 0,
            raises: 0,
        }
    }

    /// The gain the next block gets.
    #[must_use]
    pub fn gain(&self) -> f64 {
        // `vol` stays far below 2^53, where every integer is an f64.
        #[allow(clippy::cast_precision_loss)]
        let vol = self.vol as f64;
        vol / f64::from(1u32 << self.stepping)
    }

    /// New settings, as the plugin takes them from its sliders: a new
    /// stepping keeps the gain, the rest applies from the next block.
    pub fn set(&mut self, level: u32, stepping: u32, boost: bool) {
        let stepping = stepping.clamp(STEPPING_RANGE.0, STEPPING_RANGE.1);
        self.vol = (self.vol << stepping) >> self.stepping;
        self.stepping = stepping;
        self.level = level.clamp(1, 100);
        self.boost = boost;
    }

    /// Takes one block's peak (its largest absolute sample before the gain,
    /// full scale 1) and returns the gain for the next block.
    pub fn block(&mut self, peak: f64) -> f64 {
        let step = i64::from(self.stepping);
        self.vol = self.vol.max(1);
        let peak = peak * FULL_SCALE;
        let mut drops = 0;
        let out = loop {
            let out = self.gain() * peak;
            if out <= CLIP {
                break out;
            }
            self.vol = if self.vol > step { self.vol - step } else { 1 };
            drops += 1;
            if drops >= MAX_DROPS {
                // The plugin leaves such a block as it was.
                return self.gain();
            }
        };
        if out > QUIET {
            let counter = &mut self.counters[self.history];
            if out <= f64::from(self.level << 15) * 0.01 {
                *counter = (*counter + 1).min(COUNTER_MAX);
                let agrees = *counter > COUNTER_START;
                self.history = (self.history | COUNTERS) >> 1;
                if agrees {
                    self.raises = self.raises.saturating_add(1);
                    self.vol += if self.raises >= FAST_AFTER { step } else { 1 };
                }
            } else {
                *counter = counter.saturating_sub(1);
                let agrees = *counter <= COUNTER_START;
                self.history >>= 1;
                self.raises = 0;
                if agrees && self.vol >= 2 {
                    self.vol -= 1;
                }
            }
        }
        if !self.boost {
            self.vol = self.vol.min(1 << self.stepping);
        }
        self.gain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Block peaks (16-bit units) of a fixed sequence: speech-like, loud,
    /// quiet, silent, louder, full scale and very quiet stretches.
    fn peaks() -> Vec<u32> {
        const SEGMENTS: [(usize, u32, u32); 7] = [
            (600, 500, 2000),
            (100, 28_000, 32_767),
            (800, 100, 300),
            (100, 0, 0),
            (800, 2000, 10_000),
            (100, 32_767, 32_768),
            (1500, 20, 60),
        ];
        let mut x: u32 = 12_345;
        let mut peaks = Vec::new();
        for (blocks, low, high) in SEGMENTS {
            for _ in 0..blocks {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                peaks.push(low + (x >> 8) % (high - low + 1));
            }
        }
        peaks
    }

    fn fnv(values: &[i64]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for value in values {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    }

    fn run(level: u32, stepping: u32, boost: bool) -> Vec<i64> {
        let mut agc = Tomsteady::new(level, stepping, boost);
        peaks()
            .into_iter()
            .map(|peak| {
                agc.block(f64::from(peak) / FULL_SCALE);
                agc.vol
            })
            .collect()
    }

    /// The units after each block equal those of the reference model the
    /// reconstruction was checked with against PotPlayer (S5's recordings).
    #[test]
    fn follows_the_reference_model() {
        let picks = [
            0, 100, 599, 600, 650, 699, 1000, 1499, 1550, 1600, 2000, 2399, 2400, 2450, 2499, 3000,
            3999,
        ];
        let cases: [(u32, u32, bool, u64, [i64; 17]); 4] = [
            (
                75,
                10,
                true,
                0xcdf4_7976_4bfe_44f7,
                [
                    1025, 1125, 5881, 1041, 901, 859, 2726, 7716, 7716, 5676, 3108, 3130, 929, 879,
                    830, 4687, 14677,
                ],
            ),
            (
                75,
                10,
                false,
                0x01d0_6d6b_45ac_1eb1,
                [
                    1024, 1024, 1024, 1024, 904, 859, 1024, 1024, 1024, 1024, 1024, 1024, 934, 884,
                    835, 1024, 1024,
                ],
            ),
            (
                60,
                5,
                true,
                0xc2d9_192a_d40f_499c,
                [
                    33, 133, 467, 31, 21, 20, 1002, 2385, 2385, 174, 102, 94, 28, 19, 20, 2011,
                    7006,
                ],
            ),
            (
                90,
                8,
                true,
                0x7a95_bef9_e0bb_6cf8,
                [
                    257, 357, 3830, 255, 241, 237, 1770, 5762, 5762, 1418, 795, 828, 229, 230, 231,
                    3349, 11341,
                ],
            ),
        ];
        for (level, stepping, boost, hash, expected) in cases {
            let vols = run(level, stepping, boost);
            let picked: Vec<i64> = picks.iter().map(|&i| vols[i]).collect();
            assert_eq!(
                picked, expected,
                "level {level} stepping {stepping} boost {boost}"
            );
            assert_eq!(
                fnv(&vols),
                hash,
                "level {level} stepping {stepping} boost {boost}"
            );
        }
    }

    #[test]
    fn loud_blocks_drop_the_gain_at_once() {
        let mut agc = Tomsteady::new(75, 10, true);
        let gain = agc.block(1.0);
        assert!(gain * FULL_SCALE <= CLIP, "{gain}");
        assert!(
            gain * FULL_SCALE > CLIP - 10.0 * FULL_SCALE / 1024.0,
            "{gain}"
        );
    }

    #[test]
    fn quiet_blocks_and_silence_leave_the_gain() {
        let mut agc = Tomsteady::new(75, 10, true);
        assert!((agc.block(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((agc.block(10.0 / FULL_SCALE) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn raises_speed_up_after_128_in_a_row() {
        let mut agc = Tomsteady::new(75, 10, true);
        for _ in 0..127 {
            agc.block(0.01);
        }
        assert_eq!(agc.vol, 1024 + 127);
        agc.block(0.01);
        assert_eq!(agc.vol, 1024 + 127 + 10);
    }

    #[test]
    fn without_boost_the_gain_stays_at_most_one() {
        let mut agc = Tomsteady::new(75, 10, false);
        for _ in 0..1000 {
            assert!(agc.block(0.01) <= 1.0);
        }
        agc.set(75, 10, true);
        agc.block(0.01);
        assert!(agc.gain() > 1.0);
    }

    #[test]
    fn a_new_stepping_keeps_the_gain() {
        let mut agc = Tomsteady::new(75, 10, true);
        for _ in 0..300 {
            agc.block(0.01);
        }
        let before = agc.gain();
        agc.set(75, 8, true);
        assert!(
            (agc.gain() - before).abs() < 1.0 / 256.0,
            "{before} {}",
            agc.gain()
        );
    }
}
