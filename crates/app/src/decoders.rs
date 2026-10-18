//! M12's decoders page, in place of PotPlayer's DirectShow filter
//! priorities (Filter Control): which codecs mpv decodes in hardware
//! (`hwdec-codecs`; the mode stays M5's `hwdec`), the AV1 software decoder
//! (`vd`) and the decoders' threads (`vd-lavc-threads`). mpv takes them for
//! the next decoder it opens; RVP reopens the video's at once.

use serde::{Deserialize, Serialize};

/// The codecs the page offers for hardware decoding, with their names:
/// FFmpeg's codec, the text.
pub const HARDWARE_CODECS: &[(&str, &str)] = &[
    ("h264", "H.264 / AVC"),
    ("hevc", "H.265 / HEVC"),
    ("av1", "AV1"),
    ("vp9", "VP9"),
    ("vp8", "VP8"),
    ("mpeg2video", "MPEG-2"),
    ("vc1", "VC-1 / WMV3"),
    ("mpeg4", "MPEG-4 Part 2 (XviD, DivX)"),
    ("prores", "ProRes"),
];

/// The AV1 software decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Av1Decoder {
    /// `VideoLAN`'s dav1d (`libdav1d`), the faster one.
    #[default]
    Dav1d,
    /// FFmpeg's own (`av1`).
    Ffmpeg,
}

/// The decoders' settings (`decoders` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DecoderSettings {
    /// FFmpeg's codecs mpv may decode in hardware.
    pub hardware: Vec<String>,
    pub av1: Av1Decoder,
    /// Decoder threads (0: as many as the CPU has, mpv's own choice).
    pub threads: u32,
}

impl Default for DecoderSettings {
    fn default() -> Self {
        Self {
            // mpv's own default list, sorted as the settings keep it.
            hardware: ["av1", "h264", "hevc", "prores", "vc1", "vp8", "vp9"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            av1: Av1Decoder::Dav1d,
            threads: 0,
        }
    }
}

impl DecoderSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let before = self.hardware.len();
        self.hardware
            .retain(|codec| HARDWARE_CODECS.iter().any(|(name, _)| name == codec));
        self.hardware.sort();
        self.hardware.dedup();
        if self.hardware.len() < before {
            notes.push("decoders.hardware lists codecs RVP does not know: left out".to_owned());
        }
        if self.threads > 64 {
            notes.push("decoders.threads must be 0 to 64".to_owned());
            self.threads = 64;
        }
    }

    /// mpv's options for these settings.
    #[must_use]
    pub fn mpv_options(&self) -> Vec<(&'static str, String)> {
        let hardware = if self.hardware.is_empty() {
            // An empty list would let mpv choose every codec.
            "none".to_owned()
        } else {
            self.hardware.join(",")
        };
        vec![
            ("hwdec-codecs", hardware),
            (
                "vd",
                match self.av1 {
                    // The trailing comma lets mpv fall back to the others.
                    Av1Decoder::Dav1d => "libdav1d,".to_owned(),
                    Av1Decoder::Ffmpeg => "av1,".to_owned(),
                },
            ),
            ("vd-lavc-threads", self.threads.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_becomes_mpvs_options() {
        let mut settings = DecoderSettings::default();
        assert_eq!(
            settings.mpv_options()[0],
            (
                "hwdec-codecs",
                "av1,h264,hevc,prores,vc1,vp8,vp9".to_owned()
            )
        );
        settings.hardware = vec!["hevc".to_owned(), "x".to_owned(), "h264".to_owned()];
        settings.av1 = Av1Decoder::Ffmpeg;
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 1);
        assert_eq!(
            settings.mpv_options(),
            vec![
                ("hwdec-codecs", "h264,hevc".to_owned()),
                ("vd", "av1,".to_owned()),
                ("vd-lavc-threads", "0".to_owned()),
            ]
        );
        settings.hardware.clear();
        assert_eq!(settings.mpv_options()[0].1, "none");
    }
}
