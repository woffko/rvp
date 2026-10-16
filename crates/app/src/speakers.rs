//! The output speakers (PotPlayer's Audio, Output Speakers): which channels
//! RVP plays, and how it mixes a source with more channels down.
//!
//! PotPlayer's default is "2/0/0 - 2.0 Stereo (Recommended)": it mixes
//! every source down to stereo, the front channels at their level, the
//! centre and the LFE at -3 dB, the surround channels at their level,
//! without scaling the sum down, and its normalizer then works on that
//! mix. The audio audit (`docs/audits/audio.md`) measured this on a 5.1
//! tone: TomSteady's model on the mix matches the recording within
//! 0.02 dB. mpv mixes down after RVP's filters, with the surround channels
//! at -3 dB and no LFE: film dialogue in the centre came out 3 dB quieter
//! than PotPlayer's, and the normalizer measured the channels before they
//! were mixed. For stereo RVP therefore puts PotPlayer's mix in front of
//! its chain (`@rvpdownmix`, swresample with those levels). The other
//! choices leave the channels to mpv (`audio-channels`).

use rvp_engine::Engine;

use crate::app::{App, UiRequest};

/// The label of RVP's downmix in mpv's `af` chain.
pub const LABEL: &str = "rvpdownmix";

/// PotPlayer's mix to stereo: swresample's matrix with the centre at
/// -3 dB, the surround channels at 0 dB and the LFE at -3 dB on each side
/// (swresample halves the LFE's power between the two), in float and not
/// scaled down (`rematrix_maxval`).
const STEREO_ENTRY: &str = "@rvpdownmix:lavfi=[aformat=sample_fmts=flt|fltp,\
     aresample=ochl=stereo:clev=0.707107:slev=1:lfe_mix_level=1:rematrix_maxval=1000]";

/// A choice of speakers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// The value in `settings.json`.
    pub value: &'static str,
    /// PotPlayer's text in Output Speakers.
    pub label: &'static str,
    /// mpv's `audio-channels`.
    pub channels: &'static str,
}

/// The choices, in PotPlayer's order; stereo is the default.
pub const CHOICES: [Choice; 6] = [
    Choice {
        value: "mono",
        label: "1/0/0 - 1.0 Mono",
        channels: "mono",
    },
    Choice {
        value: "stereo",
        label: "2/0/0 - 2.0 Stereo (Recommended)",
        channels: "stereo",
    },
    Choice {
        value: "2.1",
        label: "2/0/0+LFE - 2.1 Stereo",
        channels: "2.1",
    },
    Choice {
        value: "5.1",
        label: "3/0/2+LFE - 5.1 Channels",
        channels: "5.1",
    },
    Choice {
        value: "7.1",
        label: "3/2/2+LFE - 7.1 Channels",
        channels: "7.1",
    },
    Choice {
        value: "same",
        label: "Same as input",
        channels: "auto-safe",
    },
];

/// The default choice's value.
pub const DEFAULT: &str = "stereo";

/// The choice with this value.
#[must_use]
pub fn choice(value: &str) -> Option<&'static Choice> {
    CHOICES.iter().find(|choice| choice.value == value)
}

/// The `af` entry for a choice: PotPlayer's mix for stereo, none for the
/// others.
#[must_use]
pub fn chain_entry(value: &str) -> Option<&'static str> {
    (value == DEFAULT).then_some(STEREO_ENTRY)
}

impl App {
    /// mpv's `audio-channels` for the speakers chosen.
    #[must_use]
    pub fn audio_channels(&self) -> &'static str {
        choice(&self.settings.output_speakers).map_or("stereo", |choice| choice.channels)
    }

    /// Puts the downmix of the speakers chosen in front of the audio chain
    /// (after the equalizer, which goes in front too) or takes it out.
    pub fn apply_speakers(&self, engine: &Engine) {
        let entry = chain_entry(&self.settings.output_speakers);
        if let Err(error) = engine.set_audio_filter_first(LABEL, entry) {
            eprintln!("rvp: cannot set the downmix: {error}");
        }
    }

    /// Chooses the speakers by their index in [`CHOICES`] (the menu).
    pub fn set_speakers(&mut self, engine: &Engine, index: usize) -> Option<UiRequest> {
        let choice = CHOICES.get(index)?;
        if self.settings.output_speakers != choice.value {
            choice.value.clone_into(&mut self.settings.output_speakers);
            self.speakers_changed(engine);
            self.save_settings();
        }
        Some(UiRequest::Osd(format!("Output Speakers: {}", choice.label)))
    }

    /// The speakers changed: mpv's channels and the downmix follow, at once
    /// or with the next file near the end of this one.
    pub(crate) fn speakers_changed(&mut self, engine: &Engine) {
        if let Err(error) = engine.set_text("audio-channels", self.audio_channels()) {
            eprintln!("rvp: cannot set the speakers: {error}");
        }
        if self.defer_chain_change() {
            return;
        }
        self.apply_speakers(engine);
        self.resync_audio(engine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_is_potplayers_mix_and_the_default() {
        assert_eq!(choice(DEFAULT).map(|c| c.channels), Some("stereo"));
        let entry = chain_entry("stereo").expect("an entry");
        assert!(entry.starts_with("@rvpdownmix:lavfi=["));
        assert!(entry.contains("clev=0.707107:slev=1:lfe_mix_level=1:rematrix_maxval=1000"));
        for other in ["mono", "2.1", "5.1", "7.1", "same"] {
            assert_eq!(chain_entry(other), None, "{other}");
        }
        assert_eq!(choice("same").map(|c| c.channels), Some("auto-safe"));
        assert!(choice("quad").is_none());
    }
}
