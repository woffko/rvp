//! The volume as RVP shows it and as mpv takes it.
//!
//! PotPlayer's volume scales the sound by `(p / 100)²` up to 100 % (40 ·
//! log10(p / 100) dB) and by `p / 100` above it, where the output clips at
//! full scale. The audio audit (`docs/audits/audio.md`) measured its
//! output at 21 levels: from 3 % to 80 % it is within 0.05 dB of the
//! square, and a clipped tone at 150 % and 200 % matches the linear gain
//! within 0.1 dB. mpv scales by `(volume / 100)³`, so with mpv's number
//! RVP was 13 dB quieter than PotPlayer at 22 % (owner, 2026-10-16:
//! PotPlayer's 20 % sounded like RVP's 40 %).
//!
//! RVP shows and takes PotPlayer's percent: the keys, the wheel, the
//! sliders, the OSD, the highest volume in the settings and MPRIS. mpv gets
//! the volume of the same gain: `100 · (p / 100)^(2/3)` up to 100 %,
//! `100 · (p / 100)^(1/3)` above. The state file keeps mpv's volume, so a
//! volume saved before keeps its loudness and now shows PotPlayer's number
//! for it.

/// mpv's `volume` for a volume RVP shows, in percent.
#[must_use]
pub fn mpv_volume(percent: f64) -> f64 {
    let ratio = percent.max(0.0) / 100.0;
    let exponent = if ratio <= 1.0 { 2.0 / 3.0 } else { 1.0 / 3.0 };
    100.0 * ratio.powf(exponent)
}

/// mpv's `volume-max` for the highest volume RVP shows: a thousandth above
/// [`mpv_volume`] of it, so that the highest volume itself is in range
/// (mpv keeps both as 32-bit floats and refuses a volume above the
/// maximum: three decimals of the exact value fell just below it).
#[must_use]
pub fn mpv_volume_max(percent: f64) -> String {
    format!("{:.3}", mpv_volume(percent.max(100.0)) + 0.001)
}

/// The volume RVP shows, in percent, for mpv's `volume`. Rounded to a
/// thousandth, so that steps of whole percents stay whole.
#[must_use]
pub fn percent(mpv_volume: f64) -> f64 {
    let ratio = mpv_volume.max(0.0) / 100.0;
    let exponent = if ratio <= 1.0 { 1.5 } else { 3.0 };
    (100.0 * ratio.powf(exponent) * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gain in dB a volume RVP shows gives through mpv's cube law.
    fn gain_db(shown: f64) -> f64 {
        60.0 * (mpv_volume(shown) / 100.0).log10()
    }

    #[test]
    fn the_shown_percent_has_potplayers_gain() {
        // PotPlayer's measured output against its 100 %, in dB.
        for (shown, potplayer) in [
            (80.0, -3.88),
            (50.0, -12.04),
            (22.0, -26.30),
            (10.0, -40.0),
            (3.0, -60.92),
        ] {
            assert!(
                (gain_db(shown) - potplayer).abs() < 0.02,
                "{shown} %: {} dB against {potplayer} dB",
                gain_db(shown)
            );
        }
        assert!((mpv_volume(100.0) - 100.0).abs() < 1e-9);
        // Above 100 % the gain is linear: +3.52 dB at 150 %, +6.02 dB at
        // 200 % (before PotPlayer's output clips).
        assert!((gain_db(150.0) - 3.52).abs() < 0.01);
        assert!((gain_db(200.0) - 6.02).abs() < 0.01);
        assert!(mpv_volume(0.0).abs() < 1e-12);
        assert!(mpv_volume(-5.0).abs() < 1e-12);
    }

    #[test]
    fn percent_and_mpv_volume_round_trip() {
        for shown in [0.0, 1.0, 22.0, 41.0, 99.0, 100.0, 150.0, 1000.0] {
            assert!((percent(mpv_volume(shown)) - shown).abs() < 1e-9, "{shown}");
        }
        // The owner's saved mpv volume of 41 keeps its loudness: it shows as
        // PotPlayer's 26 %.
        assert!((percent(41.0) - 26.253).abs() < 1e-9, "{}", percent(41.0));
    }
}
