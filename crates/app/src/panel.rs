//! The control panel (F7): the skin's `ControlBoxSkinArea` in a window of
//! its own, and the values its sliders, labels and toggle buttons show.
//!
//! PotPlayer behaviour reproduced (BlackSpace's markup):
//!
//! * The panel has the area's size (`Width` and `Height`, 300x159 for
//!   BlackSpace) and five pages chosen by its `CMD_SELECT_PAGE` buttons:
//!   audio (the equalizer), video (colour and effects), subtitles,
//!   playback (A-B repeat, jumps, speed and audio sync) and the aspect
//!   ratio. Dragging its background moves it; F7 or its close button
//!   (`ID_CONTROL_BOX_WINDOW`) hides it.
//! * Vertical sliders (`Vert='1'`) fill from the bottom: the equalizer's
//!   ten bands (±12 dB, 0 dB in the middle) and the volume beside them
//!   (`CMD_MASTER_VOLUME`, which sets RVP's volume, not the system's).
//!   Moving a band switches the equalizer on, as choosing a preset does.
//! * The colour sliders (brightness, contrast, saturation, hue) run from
//!   -100 to 100 with 0 in the middle, and their `*_DEFAULT` buttons reset
//!   one value. Moving one switches the colour controls on, as the menu's
//!   steps do.
//! * `DownFlag` buttons look pressed while their command is on: the
//!   equalizer, the video effects, the aspect ratio. This applies to the
//!   main window too (BlackSpace's main area has the equalizer button).
//! * Labels: the equalizer's preset, the subtitle size in percent, the A-B
//!   repeat points.

use rvp_engine::{Engine, PropertyId};
use rvp_skin::{DataModel, SliderValue, UiState};

use crate::action::Action;
use crate::app::{App, UiRequest};
use crate::equalizer::{BANDS, RANGE};
use crate::video::ColorKind;
use crate::view::format_time;

/// The equalizer's band sliders, in the order of [`BANDS`].
pub const BAND_SLIDERS: [&str; 10] = [
    "CMD_EQULIZER_0",
    "CMD_EQULIZER_1",
    "CMD_EQULIZER_2",
    "CMD_EQULIZER_3",
    "CMD_EQULIZER_4",
    "CMD_EQULIZER_5",
    "CMD_EQULIZER_6",
    "CMD_EQULIZER_7",
    "CMD_EQULIZER_8",
    "CMD_EQULIZER_9",
];

/// The colour sliders and what they set.
pub const COLOR_SLIDERS: [(&str, ColorKind); 4] = [
    ("CMD_COLOR_OFFSET", ColorKind::Brightness),
    ("CMD_COLOR_GAIN", ColorKind::Contrast),
    ("CMD_COLOR_SAT", ColorKind::Saturation),
    ("CMD_COLOR_HUE", ColorKind::Hue),
];

/// The largest colour value either way.
const COLOR_RANGE: f64 = 100.0;

/// A value of `-range..=range` as a slider position (the middle is 0).
fn to_fraction(value: f64, range: f64) -> f32 {
    // Slider positions are stored as f32; the loss is far below a pixel.
    #[allow(clippy::cast_possible_truncation)]
    let fraction = f64::midpoint(value / range, 1.0).clamp(0.0, 1.0) as f32;
    fraction
}

/// A slider position as a value of `-range..=range`.
fn from_fraction(fraction: f64, range: f64) -> f64 {
    (fraction.clamp(0.0, 1.0) * 2.0 - 1.0) * range
}

/// `+3.5 dB`, `-12.0 dB`, `0.0 dB`.
fn decibels(gain: f64) -> String {
    if gain > 0.0 {
        format!("+{gain:.1} dB")
    } else {
        format!("{gain:.1} dB")
    }
}

/// `60 Hz`, `1 kHz`, `16 kHz`.
fn frequency(hertz: u32) -> String {
    if hertz >= 1000 {
        format!("{} kHz", hertz / 1000)
    } else {
        format!("{hertz} Hz")
    }
}

impl App {
    /// Adds the control panel's values to a skin data model. The main
    /// window's model gets them too: skins may place these controls there.
    pub fn panel_data(&self, data: &mut DataModel) {
        let eq = &self.settings.equalizer;
        for (command, gain) in BAND_SLIDERS.iter().zip(eq.bands) {
            data.sliders.insert(
                (*command).to_owned(),
                SliderValue {
                    value: to_fraction(gain, RANGE),
                    enabled: true,
                },
            );
        }
        for (command, kind) in COLOR_SLIDERS {
            data.sliders.insert(
                command.to_owned(),
                SliderValue {
                    value: to_fraction(f64::from(self.settings.color.get(kind)), COLOR_RANGE),
                    enabled: true,
                },
            );
        }
        if let Some(volume) = data.sliders.get("CMD_VOLUME_CTRL").copied() {
            data.sliders.insert("CMD_MASTER_VOLUME".to_owned(), volume);
        }
        data.set_text("CMD_EQ_PRESET_MESSAGE", eq.preset_text());
        let scale = self
            .model
            .double(PropertyId::SubScale)
            .unwrap_or(self.settings.subtitle_scale);
        data.set_text("CMD_CAPTION_SIZE_MESSAGE", format!("{:.0}%", scale * 100.0));
        let font = self.settings.subtitle_font.as_str();
        data.set_text(
            "CMD_CAPTION_FONT_MESSAGE",
            if font.is_empty() { "Default" } else { font },
        );
        let point = |seconds: Option<f64>| seconds.map_or_else(String::new, format_time);
        data.set_text("CMD_REPEAT_START_MESSAGE", point(self.session.point_a));
        data.set_text("CMD_REPEAT_END_MESSAGE", point(self.session.point_b));
    }

    /// Sets the flags of `DownFlag` buttons (`commands`, see
    /// [`crate::chrome::Chrome::toggle_commands`]) from their actions'
    /// states.
    pub fn toggle_flags<'a>(
        &self,
        state: &mut UiState,
        commands: impl IntoIterator<Item = &'a str>,
    ) {
        for command in commands {
            let Some(action) = Action::from_skin_command(command) else {
                continue;
            };
            match self.checked(action) {
                Some(true) => {
                    state.flags.insert(command.to_owned());
                }
                Some(false) => {
                    state.flags.remove(command);
                }
                None => {}
            }
        }
    }

    /// Sets the subtitle font chosen in the font dialog (empty: mpv's
    /// default).
    pub fn set_subtitle_font(&mut self, engine: &Engine, family: &str) -> UiRequest {
        let mut changed = self.settings.clone();
        family.clone_into(&mut changed.subtitle_font);
        let applied = self.apply_settings(engine, changed);
        let shown = if self.settings.subtitle_font.is_empty() {
            "default"
        } else {
            self.settings.subtitle_font.as_str()
        };
        match applied.notes.into_iter().next() {
            Some(note) => UiRequest::Message(note),
            None => UiRequest::Osd(format!("Subtitle font: {shown}")),
        }
    }

    /// The panel's window moved (screen coordinates): it opens there next
    /// time.
    pub fn set_panel_position(&mut self, x: i32, y: i32) {
        self.store_mut().set_panel((x, y));
    }

    /// Where the panel's window was last, when known.
    #[must_use]
    pub fn panel_position(&self) -> Option<(i32, i32)> {
        self.store().state().panel
    }

    /// The panel's window closed by other means than F7 or its close
    /// button (the window system).
    pub fn panel_closed(&mut self) {
        self.session.control_panel = false;
    }

    /// A slider other than the seek bar and the volume moved to `fraction`
    /// of its range: an equalizer band or a colour.
    pub fn set_slider(
        &mut self,
        engine: &Engine,
        command: &str,
        fraction: f64,
    ) -> Option<UiRequest> {
        if let Some(band) = BAND_SLIDERS.iter().position(|slider| *slider == command) {
            self.set_equalizer_band(engine, band, from_fraction(fraction, RANGE));
            let gain = self.settings.equalizer.bands[band];
            return Some(UiRequest::Osd(format!(
                "Equalizer {}: {}",
                frequency(BANDS[band]),
                decibels(gain)
            )));
        }
        let (_, kind) = COLOR_SLIDERS
            .iter()
            .find(|(slider, _)| *slider == command)?;
        // The range is ±100.
        #[allow(clippy::cast_possible_truncation)]
        let value = from_fraction(fraction, COLOR_RANGE).round() as i32;
        self.set_color(engine, *kind, value);
        Some(UiRequest::Osd(format!(
            "{}: {}",
            kind.text(),
            self.settings.color.get(*kind)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_positions_and_values_meet_in_the_middle() {
        assert!((to_fraction(0.0, RANGE) - 0.5).abs() < 1e-6);
        assert!((to_fraction(12.0, RANGE) - 1.0).abs() < 1e-6);
        assert!((to_fraction(-30.0, RANGE)).abs() < 1e-6);
        assert!((from_fraction(0.5, RANGE)).abs() < 1e-9);
        assert!((from_fraction(1.0, COLOR_RANGE) - 100.0).abs() < 1e-9);
        assert!((from_fraction(-1.0, COLOR_RANGE) + 100.0).abs() < 1e-9);
        assert_eq!(decibels(3.5), "+3.5 dB");
        assert_eq!(decibels(0.0), "0.0 dB");
        assert_eq!(decibels(-12.0), "-12.0 dB");
        assert_eq!(frequency(600), "600 Hz");
        assert_eq!(frequency(16000), "16 kHz");
    }
}
