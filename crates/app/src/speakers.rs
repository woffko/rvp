//! The output speakers (PotPlayer's Audio, Output Speakers): which channels
//! RVP plays, and how it mixes a source to them.
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
//! were mixed. RVP therefore puts its own mix, with PotPlayer's levels (the
//! Audio page's mixer, `crate::audio::Mixer`), in front of its chain for
//! every layout (`@rvpdownmix`, `crate::audio::chain`), and gives mpv the
//! layout as `audio-channels`.
//!
//! PotPlayer's notation is front/side/back channels: "3/0/2+LFE" is
//! FL FR FC, no side pair, BL BR, and the LFE. Besides the layouts there
//! are the matrix encodings (Dolby Surround/Pro Logic and Pro Logic II, as
//! swresample writes them), the source's own layout, and three modes that
//! process the sound for the speakers: the virtual headphone and the
//! virtual surround (a head model's impulse responses, `crate::audio::dsp`)
//! and the virtual Dolby decoder (FFmpeg's `surround` upmix to 5.1).

use rvp_engine::Engine;

use crate::app::{App, UiRequest};

/// The label of RVP's downmix in mpv's `af` chain.
pub const LABEL: &str = "rvpdownmix";

/// How a choice makes its channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A layout, mixed by swresample: FFmpeg's name and its channels in
    /// FFmpeg's order.
    Layout(&'static str, &'static [&'static str]),
    /// Stereo with surround matrix-encoded in it (`dolby`, `dplii`), with
    /// or without a separate LFE channel.
    Matrix(&'static str, bool),
    /// The source's layout.
    Same,
    /// Binaural sound for headphones from a head model.
    VirtualHeadphone,
    /// The same, with the virtualizer's room.
    VirtualSurround,
    /// Stereo decoded to 5.1 (FFmpeg's `surround`).
    DolbyDecoder,
}

/// A choice of speakers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// The value in `settings.json`.
    pub value: &'static str,
    /// PotPlayer's text in Output Speakers.
    pub label: &'static str,
    pub mode: Mode,
}

const MONO: &[&str] = &["FC"];
const STEREO: &[&str] = &["FL", "FR"];
const FIVE_ONE: &[&str] = &["FL", "FR", "FC", "LFE", "BL", "BR"];

const fn layout(
    value: &'static str,
    label: &'static str,
    name: &'static str,
    channels: &'static [&'static str],
) -> Choice {
    Choice {
        value,
        label,
        mode: Mode::Layout(name, channels),
    }
}

/// The choices, in PotPlayer's order; stereo is the default.
pub const CHOICES: [Choice; 26] = [
    layout("mono", "1/0/0 - 1.0 Mono", "mono", MONO),
    layout(
        "stereo",
        "2/0/0 - 2.0 Stereo (Recommended)",
        "stereo",
        STEREO,
    ),
    layout("3.0", "3/0/0 - 3.0 Channels", "3.0", &["FL", "FR", "FC"]),
    layout(
        "3.0-surround",
        "2/0/1 - 3.0 Surround",
        "3.0(back)",
        &["FL", "FR", "BC"],
    ),
    layout(
        "4.0-surround",
        "3/0/1 - 4.0 Surround",
        "4.0",
        &["FL", "FR", "FC", "BC"],
    ),
    layout(
        "4.0",
        "2/0/2 - 4.0 Channels",
        "quad",
        &["FL", "FR", "BL", "BR"],
    ),
    layout(
        "5.0",
        "3/0/2 - 5.0 Channels",
        "5.0",
        &["FL", "FR", "FC", "BL", "BR"],
    ),
    layout(
        "6.0",
        "3/2/1 - 6.0 Channels",
        "6.0",
        &["FL", "FR", "FC", "BC", "SL", "SR"],
    ),
    layout(
        "7.0",
        "3/2/2 - 7.0 Channels",
        "7.0",
        &["FL", "FR", "FC", "BL", "BR", "SL", "SR"],
    ),
    Choice {
        value: "dolby",
        label: "Dolby Surround/Pro Logic",
        mode: Mode::Matrix("dolby", false),
    },
    Choice {
        value: "dplii",
        label: "Dolby Pro Logic II",
        mode: Mode::Matrix("dplii", false),
    },
    layout("1.1", "1/0/0+LFE - 1.1 Mono", "FC+LFE", &["FC", "LFE"]),
    layout("2.1", "2/0/0+LFE - 2.1 Stereo", "2.1", &["FL", "FR", "LFE"]),
    layout(
        "3.1",
        "3/0/0+LFE - 3.1 Channels",
        "3.1",
        &["FL", "FR", "FC", "LFE"],
    ),
    layout(
        "3.1-surround",
        "2/0/1+LFE - 3.1 Surround",
        "FL+FR+LFE+BC",
        &["FL", "FR", "LFE", "BC"],
    ),
    layout(
        "4.1-surround",
        "3/0/1+LFE - 4.1 Surround",
        "4.1",
        &["FL", "FR", "FC", "LFE", "BC"],
    ),
    layout(
        "4.1",
        "2/0/2+LFE - 4.1 Channels",
        "FL+FR+LFE+BL+BR",
        &["FL", "FR", "LFE", "BL", "BR"],
    ),
    layout("5.1", "3/0/2+LFE - 5.1 Channels", "5.1", FIVE_ONE),
    layout(
        "6.1",
        "3/2/1+LFE - 6.1 Channels",
        "6.1",
        &["FL", "FR", "FC", "LFE", "BC", "SL", "SR"],
    ),
    layout(
        "7.1",
        "3/2/2+LFE - 7.1 Channels",
        "7.1",
        &["FL", "FR", "FC", "LFE", "BL", "BR", "SL", "SR"],
    ),
    Choice {
        value: "dolby-lfe",
        label: "Dolby Surround/Pro Logic+LFE",
        mode: Mode::Matrix("dolby", true),
    },
    Choice {
        value: "dplii-lfe",
        label: "Dolby Pro Logic II+LFE",
        mode: Mode::Matrix("dplii", true),
    },
    Choice {
        value: "same",
        label: "Same as input",
        mode: Mode::Same,
    },
    Choice {
        value: "virtual-headphone",
        label: "Virtual Headphone",
        mode: Mode::VirtualHeadphone,
    },
    Choice {
        value: "virtual-surround",
        label: "Virtual Surround (HRTF)",
        mode: Mode::VirtualSurround,
    },
    Choice {
        value: "dolby-decoder",
        label: "Virtual Dolby Decoder",
        mode: Mode::DolbyDecoder,
    },
];

/// The default choice's value.
pub const DEFAULT: &str = "stereo";

/// The choice with this value.
#[must_use]
pub fn choice(value: &str) -> Option<&'static Choice> {
    CHOICES.iter().find(|choice| choice.value == value)
}

impl Choice {
    /// mpv's `audio-channels` for this choice: an mpv layout name or an
    /// explicit list (`fl-fr-lfe-bc`).
    #[must_use]
    pub fn audio_channels(&self) -> String {
        match self.mode {
            Mode::Layout(_, channels) => match self.value {
                "mono" | "stereo" | "2.1" | "3.0" | "3.0-surround" | "3.1" | "4.0"
                | "4.0-surround" | "5.0" | "5.1" | "6.0" | "6.1" | "7.0" | "7.1" => {
                    mpv_name(self.value)
                }
                _ => channels
                    .iter()
                    .map(|channel| channel.to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join("-"),
            },
            Mode::Matrix(_, false) | Mode::VirtualHeadphone | Mode::VirtualSurround => {
                "stereo".to_owned()
            }
            Mode::Matrix(_, true) => "2.1".to_owned(),
            Mode::DolbyDecoder => "5.1".to_owned(),
            Mode::Same => "auto-safe".to_owned(),
        }
    }

    /// The output's channels in FFmpeg's order, when the choice fixes
    /// them (not for "Same as input").
    #[must_use]
    pub fn channels(&self) -> Option<&'static [&'static str]> {
        match self.mode {
            Mode::Layout(_, channels) => Some(channels),
            Mode::Matrix(_, false) | Mode::VirtualHeadphone | Mode::VirtualSurround => Some(STEREO),
            Mode::Matrix(_, true) => Some(&["FL", "FR", "LFE"]),
            Mode::DolbyDecoder => Some(FIVE_ONE),
            Mode::Same => None,
        }
    }
}

/// mpv's names of the layouts it knows by name: PotPlayer's 4.0 Channels
/// is mpv's `quad`, its 3.0 Surround `3.0(back)`.
fn mpv_name(value: &str) -> String {
    match value {
        "3.0-surround" => "3.0(back)".to_owned(),
        "4.0" => "quad".to_owned(),
        "4.0-surround" => "4.0".to_owned(),
        other => other.to_owned(),
    }
}

/// The channels of a layout mpv names (`5.1(side)`), in FFmpeg's order;
/// empty when RVP does not know the name.
#[must_use]
pub fn layout_channels(name: &str) -> Vec<&'static str> {
    let channels: &[&str] = match name {
        "mono" | "1.0" => MONO,
        "stereo" | "2.0" => STEREO,
        "2.1" => &["FL", "FR", "LFE"],
        "3.0" => &["FL", "FR", "FC"],
        "3.0(back)" => &["FL", "FR", "BC"],
        "4.0" => &["FL", "FR", "FC", "BC"],
        "quad" => &["FL", "FR", "BL", "BR"],
        "quad(side)" => &["FL", "FR", "SL", "SR"],
        "3.1" => &["FL", "FR", "FC", "LFE"],
        "5.0" => &["FL", "FR", "FC", "BL", "BR"],
        "5.0(side)" => &["FL", "FR", "FC", "SL", "SR"],
        "4.1" => &["FL", "FR", "FC", "LFE", "BC"],
        "5.1" => FIVE_ONE,
        "5.1(side)" => &["FL", "FR", "FC", "LFE", "SL", "SR"],
        "6.0" => &["FL", "FR", "FC", "BC", "SL", "SR"],
        "6.1" => &["FL", "FR", "FC", "LFE", "BC", "SL", "SR"],
        "7.0" => &["FL", "FR", "FC", "BL", "BR", "SL", "SR"],
        "7.1" => &["FL", "FR", "FC", "LFE", "BL", "BR", "SL", "SR"],
        _ => &[],
    };
    channels.to_vec()
}

impl App {
    /// mpv's `audio-channels` for the speakers chosen.
    #[must_use]
    pub fn audio_channels(&self) -> String {
        choice(&self.settings.output_speakers)
            .map_or_else(|| "stereo".to_owned(), Choice::audio_channels)
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

    /// The speakers changed: mpv's channels and the chain follow, at once
    /// or with the next file near the end of this one.
    pub(crate) fn speakers_changed(&mut self, engine: &Engine) {
        if let Err(error) = engine.set_text("audio-channels", &self.audio_channels()) {
            eprintln!("rvp: cannot set the speakers: {error}");
        }
        self.audio_chain_changed(engine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potplayers_choices_in_its_order() {
        assert_eq!(CHOICES.len(), 26);
        assert_eq!(CHOICES[1].value, DEFAULT);
        assert_eq!(CHOICES[1].audio_channels(), "stereo");
        let mut values: Vec<&str> = CHOICES.iter().map(|c| c.value).collect();
        values.sort_unstable();
        values.dedup();
        assert_eq!(values.len(), CHOICES.len(), "unique values");
        let channels = |value: &str| choice(value).map(Choice::audio_channels);
        assert_eq!(channels("4.0").as_deref(), Some("quad"));
        assert_eq!(channels("3.0-surround").as_deref(), Some("3.0(back)"));
        assert_eq!(channels("3.1-surround").as_deref(), Some("fl-fr-lfe-bc"));
        assert_eq!(channels("1.1").as_deref(), Some("fc-lfe"));
        assert_eq!(channels("dplii").as_deref(), Some("stereo"));
        assert_eq!(channels("dolby-lfe").as_deref(), Some("2.1"));
        assert_eq!(channels("dolby-decoder").as_deref(), Some("5.1"));
        assert_eq!(channels("same").as_deref(), Some("auto-safe"));
        assert!(choice("quad").is_none());
        for choice in &CHOICES {
            if let Mode::Layout(_, list) = choice.mode {
                assert_eq!(choice.channels(), Some(list));
            }
        }
    }

    #[test]
    fn mpvs_layout_names_have_their_channels() {
        assert_eq!(
            layout_channels("5.1(side)"),
            ["FL", "FR", "FC", "LFE", "SL", "SR"]
        );
        assert_eq!(layout_channels("stereo"), ["FL", "FR"]);
        assert!(layout_channels("22.2").is_empty());
    }
}
