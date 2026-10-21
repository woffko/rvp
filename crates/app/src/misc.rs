//! M14: the Misc menu's own commands (goal P): the playback's elapsed or
//! total time copied in the format the Format submenu sets (PotPlayer's
//! "Time format", "To the end of time" and "Fill with zeros"), PotPlayer's
//! default hotkeys off, and the window moved by its title bar only.

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// The time formats: hours only when the time has them, minutes and
/// seconds (minutes past 59 as they are), hours, minutes and seconds.
pub const TIME_FORMATS: &[&str] = &["auto", "minutes", "hours"];

/// How copied times are written (`copy-time` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CopyTimeSettings {
    /// One of [`TIME_FORMATS`].
    pub format: String,
    /// Digits after the seconds (0 to 3: tenths to milliseconds).
    pub decimals: u8,
    /// The first field gets a leading zero (`01:05` rather than `1:05`).
    pub zeros: bool,
}

impl Default for CopyTimeSettings {
    fn default() -> Self {
        Self {
            format: "auto".to_owned(),
            decimals: 0,
            zeros: false,
        }
    }
}

impl CopyTimeSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !TIME_FORMATS.contains(&self.format.as_str()) {
            notes.push(format!(
                "copy-time.format {:?} is none of {TIME_FORMATS:?}: auto",
                self.format
            ));
            "auto".clone_into(&mut self.format);
        }
        if self.decimals > 3 {
            notes.push(format!(
                "copy-time.decimals {} is out of 0 to 3: 3",
                self.decimals
            ));
            self.decimals = 3;
        }
    }

    /// `seconds` in this format: `1:02:03`, `62:03`, `01:02:03.450`.
    #[must_use]
    pub fn format(&self, seconds: f64) -> String {
        // Times of a file are far within u64's milliseconds.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let millis = (seconds.max(0.0) * 1000.0).floor() as u64;
        let total_seconds = millis / 1000;
        let (hours, minutes, secs) = (
            total_seconds / 3600,
            total_seconds / 60 % 60,
            total_seconds % 60,
        );
        let width = if self.zeros { 2 } else { 1 };
        let mut text = match self.format.as_str() {
            "minutes" => format!("{:0width$}:{secs:02}", total_seconds / 60),
            "hours" => format!("{hours:0width$}:{minutes:02}:{secs:02}"),
            _ if hours > 0 => format!("{hours:0width$}:{minutes:02}:{secs:02}"),
            _ => format!("{minutes:0width$}:{secs:02}"),
        };
        if self.decimals > 0 {
            let fraction = format!("{:03}", millis % 1000);
            text.push('.');
            text.push_str(&fraction[..usize::from(self.decimals)]);
        }
        text
    }
}

/// A command of Misc > Other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MiscCommand {
    /// Copy the elapsed (`false`) or the total (`true`) time.
    CopyTime(bool),
    /// One of [`TIME_FORMATS`].
    TimeFormat(u8),
    /// Digits after the seconds (0 to 3).
    TimeDecimals(u8),
    TimeZeros,
    /// PotPlayer's default hotkeys off or on (the user's own keys stay).
    DefaultKeys,
    /// Only the title bar moves the window.
    DragTitleOnly,
}

impl App {
    /// Misc > Other's commands.
    pub(crate) fn misc_command(&mut self, command: MiscCommand) -> Option<UiRequest> {
        let settings = &mut self.settings.copy_time;
        let text = match command {
            MiscCommand::CopyTime(total) => {
                let property = if total {
                    rvp_engine::PropertyId::Duration
                } else {
                    rvp_engine::PropertyId::TimePos
                };
                let seconds = self.model.double(property)?;
                return Some(UiRequest::CopyText(settings.format(seconds)));
            }
            MiscCommand::TimeFormat(index) => {
                (*TIME_FORMATS.get(usize::from(index))?).clone_into(&mut settings.format);
                format!("Copied times: {}", settings.format(3723.456))
            }
            MiscCommand::TimeDecimals(decimals) => {
                settings.decimals = decimals.min(3);
                format!("Copied times: {}", settings.format(3723.456))
            }
            MiscCommand::TimeZeros => {
                settings.zeros = !settings.zeros;
                format!("Copied times: {}", settings.format(3723.456))
            }
            MiscCommand::DefaultKeys => {
                let input = &mut self.settings.input;
                input.disable_default_keys = !input.disable_default_keys;
                let off = input.disable_default_keys;
                let (bindings, _) = self.settings.bindings();
                self.bindings = bindings;
                format!("Default hotkeys: {}", if off { "off" } else { "on" })
            }
            MiscCommand::DragTitleOnly => {
                let input = &mut self.settings.input;
                input.drag_title_only = !input.drag_title_only;
                if input.drag_title_only {
                    "Only the title bar moves the window".to_owned()
                } else {
                    "The title bar, the skin and the video move the window".to_owned()
                }
            }
        };
        self.save_settings();
        Some(UiRequest::Osd(text))
    }

    /// The Format submenu's check marks.
    #[must_use]
    pub(crate) fn misc_checked(&self, command: MiscCommand) -> Option<bool> {
        let settings = &self.settings.copy_time;
        match command {
            MiscCommand::CopyTime(_) => None,
            MiscCommand::TimeFormat(index) => {
                Some(TIME_FORMATS.get(usize::from(index)) == Some(&settings.format.as_str()))
            }
            MiscCommand::TimeDecimals(decimals) => Some(settings.decimals == decimals),
            MiscCommand::TimeZeros => Some(settings.zeros),
            MiscCommand::DefaultKeys => Some(self.settings.input.disable_default_keys),
            MiscCommand::DragTitleOnly => Some(self.settings.input.drag_title_only),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_follow_the_format() {
        let mut settings = CopyTimeSettings::default();
        assert_eq!(settings.format(65.4), "1:05");
        assert_eq!(settings.format(3723.456), "1:02:03");
        "minutes".clone_into(&mut settings.format);
        assert_eq!(settings.format(3723.456), "62:03");
        "hours".clone_into(&mut settings.format);
        assert_eq!(settings.format(65.4), "0:01:05");
        settings.zeros = true;
        settings.decimals = 3;
        assert_eq!(settings.format(65.4), "00:01:05.400");
        settings.decimals = 1;
        "auto".clone_into(&mut settings.format);
        assert_eq!(settings.format(65.48), "01:05.4");
    }
}
