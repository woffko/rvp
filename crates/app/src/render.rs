//! M12's rendering choices, mpv's in place of PotPlayer's DirectShow
//! renderers (its Video page and the Pixel Shaders menu's colour items):
//! the downscaler and the chroma scaler (the upscaler is M5's `scale`),
//! HDR tone mapping and peak detection, and the display's ICC profile.
//! All are mpv options it takes while playing.
//!
//! PotPlayer's Pixel Shaders menu turns these on and off: "Enable Display
//! ICC Color Gamut Correction" is the ICC profile, "SMPTE ST 2084 HDR
//! Correction" the tone mapping, and "SMPTE ST 2086 HDR Correction" the
//! file's mastering metadata in place of measured peaks.

use rvp_engine::Engine;
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// mpv's tone mapping curves (`tone-mapping`), the first its default.
pub const TONE_MAPPINGS: &[&str] = &[
    "auto",
    "bt.2390",
    "spline",
    "bt.2446a",
    "st2094-40",
    "st2094-10",
    "hable",
    "mobius",
    "reinhard",
    "clip",
];

/// mpv's scalers for the downscaler and the chroma scaler (empty: mpv's
/// default).
pub const RENDER_SCALERS: &[&str] = &[
    "",
    "bilinear",
    "bicubic",
    "catmull_rom",
    "mitchell",
    "hermite",
    "spline36",
    "lanczos",
    "ewa_lanczos",
    "ewa_lanczossharp",
];

/// The YCbCr matrices one can force (mpv's `colormatrix`); `auto` follows
/// the file's tags.
pub const MATRICES: &[&str] = &[
    "auto",
    "bt.601",
    "bt.709",
    "bt.2020-ncl",
    "smpte-240m",
    "fcc",
];

/// Software deinterlacers RVP runs itself (empty: mpv's own choice).
pub const DEINTERLACERS: &[&str] = &["", "bwdif", "yadif", "w3fdif"];

/// The field order (mpv's `field-dominance`).
pub const FIELD_ORDERS: &[&str] = &["auto", "top", "bottom"];

/// The label of RVP's matrix filter.
pub const MATRIX_LABEL: &str = "rvpmatrix";
/// The label of RVP's own deinterlacer.
pub const DEINTERLACE_LABEL: &str = "rvpdeint";

/// The rendering settings (`render` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's Video and Pixel Shaders pages, a check
// box each.
#[allow(clippy::struct_excessive_bools)]
pub struct RenderSettings {
    /// mpv `dscale` and `cscale` (empty: mpv's default).
    pub dscale: String,
    pub cscale: String,
    /// HDR to SDR: on maps with `tone-mapping`, off clips.
    pub tone_mapping_on: bool,
    pub tone_mapping: String,
    /// Peaks from the file's mastering metadata (ST 2086) rather than
    /// measured from the picture.
    pub hdr_metadata: bool,
    /// The display's ICC profile (`icc-profile-auto`).
    pub icc: bool,
    /// "Use limited (16-235) color range" (`video-output-levels`).
    pub limited_range: bool,
    /// "YCbCr <-> RGB conv. matrix": forced, or `auto`.
    pub matrix: String,
    /// "SDR monitor luminance": the display's peak in nits (`target-peak`;
    /// 0: mpv's choice).
    pub target_peak: u32,
    /// The software deinterlacer and the field order (Deinterlacing).
    pub deinterlacer: String,
    pub field_order: String,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            dscale: String::new(),
            cscale: String::new(),
            tone_mapping_on: true,
            tone_mapping: "auto".to_owned(),
            hdr_metadata: false,
            icc: false,
            limited_range: false,
            matrix: "auto".to_owned(),
            target_peak: 0,
            deinterlacer: String::new(),
            field_order: "auto".to_owned(),
        }
    }
}

impl RenderSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        for (name, value) in [("dscale", &mut self.dscale), ("cscale", &mut self.cscale)] {
            if !RENDER_SCALERS.contains(&value.as_str()) {
                notes.push(format!(
                    "render.{name} {value:?} is not one of mpv's scalers: the default used"
                ));
                value.clear();
            }
        }
        for (name, value, choices, default) in [
            ("matrix", &mut self.matrix, MATRICES, "auto"),
            ("deinterlacer", &mut self.deinterlacer, DEINTERLACERS, ""),
            ("field-order", &mut self.field_order, FIELD_ORDERS, "auto"),
        ] {
            if !choices.contains(&value.as_str()) {
                notes.push(format!(
                    "render.{name} {value:?} is not a choice: {default:?} used"
                ));
                default.clone_into(value);
            }
        }
        if self.target_peak != 0 && !(10..=10_000).contains(&self.target_peak) {
            notes.push("render.target-peak must be 0 or 10 to 10000 nits: 0 used".to_owned());
            self.target_peak = 0;
        }
        if !TONE_MAPPINGS.contains(&self.tone_mapping.as_str()) {
            notes.push(format!(
                "render.tone-mapping {:?} is not one of mpv's curves: auto used",
                self.tone_mapping
            ));
            "auto".clone_into(&mut self.tone_mapping);
        }
    }

    /// mpv's options for these settings, for the engine at start and at
    /// run time.
    #[must_use]
    pub fn mpv_options(&self) -> Vec<(&'static str, String)> {
        let mut options = Vec::new();
        if !self.dscale.is_empty() {
            options.push(("dscale", self.dscale.clone()));
        }
        if !self.cscale.is_empty() {
            options.push(("cscale", self.cscale.clone()));
        }
        let curve = if self.tone_mapping_on {
            self.tone_mapping.clone()
        } else {
            "clip".to_owned()
        };
        options.push(("tone-mapping", curve));
        options.push((
            "hdr-compute-peak",
            if self.hdr_metadata { "no" } else { "auto" }.to_owned(),
        ));
        options.push((
            "icc-profile-auto",
            if self.icc { "yes" } else { "no" }.to_owned(),
        ));
        options.push((
            "video-output-levels",
            if self.limited_range {
                "limited"
            } else {
                "auto"
            }
            .to_owned(),
        ));
        options.push((
            "target-peak",
            if self.target_peak == 0 {
                "auto".to_owned()
            } else {
                self.target_peak.to_string()
            },
        ));
        options.push(("field-dominance", self.field_order.clone()));
        options
    }

    /// RVP's filters for the forced matrix and its own deinterlacer
    /// (`deinterlace`: mpv's setting decides when it runs).
    #[must_use]
    pub fn filters(&self, deinterlace: crate::video::Deinterlace) -> Vec<(String, String)> {
        let mut filters = Vec::new();
        if self.matrix != "auto" {
            filters.push((
                MATRIX_LABEL.to_owned(),
                format!("@{MATRIX_LABEL}:format=colormatrix={}", self.matrix),
            ));
        }
        let frames = match deinterlace {
            crate::video::Deinterlace::Off => None,
            crate::video::Deinterlace::Auto => Some("interlaced"),
            crate::video::Deinterlace::On => Some("all"),
        };
        if let Some(frames) = frames
            && !self.deinterlacer.is_empty()
        {
            filters.push((
                DEINTERLACE_LABEL.to_owned(),
                format!(
                    "@{DEINTERLACE_LABEL}:lavfi=[{}=deint={frames}]",
                    self.deinterlacer
                ),
            ));
        }
        filters
    }
}

/// One of the Pixel Shaders menu's colour items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderToggle {
    /// Enable Display ICC Color Gamut Correction.
    Icc,
    /// Enable SMPTE ST 2084 HDR Correction: tone mapping.
    ToneMapping,
    /// Enable SMPTE ST 2086 HDR Correction: the mastering metadata.
    Metadata,
}

impl App {
    /// Sets the rendering options on mpv (start and settings changes); a
    /// scaler set back to the default gets mpv's own, read at start.
    pub(crate) fn apply_render(&mut self, engine: &Engine) {
        if self.render_defaults.is_none() {
            self.render_defaults = Some([
                engine.get_text("dscale").unwrap_or_default(),
                engine.get_text("cscale").unwrap_or_default(),
            ]);
        }
        for (name, value) in self.settings.render.mpv_options() {
            drop(engine.set_text(name, &value));
        }
        let render = &self.settings.render;
        if let Some([dscale, cscale]) = &self.render_defaults {
            for (name, value, default) in [
                ("dscale", &render.dscale, dscale),
                ("cscale", &render.cscale, cscale),
            ] {
                if value.is_empty() {
                    drop(engine.set_text(name, default));
                }
            }
        }
    }

    /// The Pixel Shaders menu's colour items.
    pub(crate) fn render_toggle(&mut self, engine: &Engine, toggle: RenderToggle) -> UiRequest {
        let render = &mut self.settings.render;
        let (name, on) = match toggle {
            RenderToggle::Icc => {
                render.icc = !render.icc;
                ("Display ICC profile", render.icc)
            }
            RenderToggle::ToneMapping => {
                render.tone_mapping_on = !render.tone_mapping_on;
                ("HDR tone mapping (ST 2084)", render.tone_mapping_on)
            }
            RenderToggle::Metadata => {
                render.hdr_metadata = !render.hdr_metadata;
                ("HDR mastering metadata (ST 2086)", render.hdr_metadata)
            }
        };
        self.save_settings();
        self.apply_render(engine);
        UiRequest::Osd(format!("{name}: {}", if on { "on" } else { "off" }))
    }

    /// The check mark of a colour item.
    pub(crate) fn render_checked(&self, toggle: RenderToggle) -> bool {
        let render = &self.settings.render;
        match toggle {
            RenderToggle::Icc => render.icc,
            RenderToggle::ToneMapping => render.tone_mapping_on,
            RenderToggle::Metadata => render.hdr_metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_become_mpvs_options() {
        let mut settings = RenderSettings::default();
        assert_eq!(
            settings.mpv_options(),
            vec![
                ("tone-mapping", "auto".to_owned()),
                ("hdr-compute-peak", "auto".to_owned()),
                ("icc-profile-auto", "no".to_owned()),
                ("video-output-levels", "auto".to_owned()),
                ("target-peak", "auto".to_owned()),
                ("field-dominance", "auto".to_owned()),
            ]
        );
        assert!(settings.filters(crate::video::Deinterlace::On).is_empty());
        settings.matrix = "bt.601".to_owned();
        settings.deinterlacer = "bwdif".to_owned();
        assert_eq!(
            settings.filters(crate::video::Deinterlace::Auto),
            vec![
                (
                    "rvpmatrix".to_owned(),
                    "@rvpmatrix:format=colormatrix=bt.601".to_owned()
                ),
                (
                    "rvpdeint".to_owned(),
                    "@rvpdeint:lavfi=[bwdif=deint=interlaced]".to_owned()
                ),
            ]
        );
        settings.matrix = "auto".to_owned();
        settings.deinterlacer.clear();
        settings.dscale = "mitchell".to_owned();
        settings.tone_mapping_on = false;
        settings.hdr_metadata = true;
        settings.icc = true;
        let options = settings.mpv_options();
        assert!(options.contains(&("dscale", "mitchell".to_owned())));
        assert!(options.contains(&("tone-mapping", "clip".to_owned())));
        assert!(options.contains(&("hdr-compute-peak", "no".to_owned())));
        assert!(options.contains(&("icc-profile-auto", "yes".to_owned())));
        let mut notes = Vec::new();
        settings.cscale = "nonsense".to_owned();
        settings.validate(&mut notes);
        assert_eq!(settings.cscale, "");
        assert_eq!(notes.len(), 1);
    }
}
