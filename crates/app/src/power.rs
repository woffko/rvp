//! What happens when playback finishes, and the shutdown timer (M11):
//! RVP exits, or the computer shuts down, sleeps or hibernates after a
//! notice that counts down and can be cancelled ([`Countdown`]).
//!
//! The app only decides; the shell carries out [`UiRequest::Power`] with
//! its platform code. With the dry-run hook (`--power-dry-run=FILE`, FILE
//! only with `RVP_TEST_HOOKS=1`; always on with `--selftest`) the shell
//! appends the action's name to the file instead ([`write_dry_run`]):
//! tests never power a machine off.
//!
//! The timer takes the seconds until it fires (the window's dialog turns
//! a time of day into them, as it knows the local time) and a label for
//! the menu ("at 23:30", "in 1:30:00").

use std::io::{self, Write as _};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::action::{PowerAction, PowerTimer};
use crate::app::{App, UiRequest};

/// What a countdown or the timer ends with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Exit,
    Power(PowerAction),
}

/// A notice that counts down to an ending.
#[derive(Debug, Clone, PartialEq)]
pub struct Countdown {
    pub ending: Ending,
    pub deadline: Instant,
    /// The seconds the notice showed last.
    shown: Option<u64>,
}

/// The shutdown timer.
#[derive(Debug, Clone, PartialEq)]
pub struct Timer {
    pub kind: PowerTimer,
    pub ending: Ending,
    pub deadline: Instant,
    /// How the menu names it: `at 23:30`, `in 1:30:00`.
    pub label: String,
}

/// The countdown and the timer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Power {
    pub countdown: Option<Countdown>,
    pub timer: Option<Timer>,
}

/// The action's name in notices and the dry-run file.
#[must_use]
pub fn action_text(action: PowerAction) -> &'static str {
    match action {
        PowerAction::Shutdown => "shut down",
        PowerAction::Sleep => "sleep",
        PowerAction::Hibernate => "hibernate",
    }
}

/// The dry-run hook: appends the action's name and a newline to `path`.
///
/// # Errors
/// Fails when the file cannot be written.
pub fn write_dry_run(path: &Path, action: PowerAction) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", action_text(action))
}

fn ending_text(ending: Ending) -> String {
    match ending {
        Ending::Exit => "RVP exits".to_owned(),
        Ending::Power(action) => format!("The computer will {}", action_text(action)),
    }
}

impl App {
    /// Starts the notice before an ending (when the file or the album
    /// finished, or the timer fired).
    pub(crate) fn start_countdown(&mut self, ending: Ending, now: Instant) -> UiRequest {
        let seconds = u64::from(self.settings.playback.finished.countdown);
        self.power.countdown = Some(Countdown {
            ending,
            deadline: now + Duration::from_secs(seconds),
            shown: Some(seconds),
        });
        UiRequest::PowerNotice(Some((ending_text(ending), seconds)))
    }

    /// The notice's Cancel: nothing happens.
    pub fn cancel_countdown(&mut self) -> Option<UiRequest> {
        self.power.countdown.take()?;
        Some(UiRequest::PowerNotice(None))
    }

    /// About once a second: the timer fires, the notice counts down, and
    /// at its end the ending is asked for.
    pub fn power_tick(&mut self, now: Instant) -> Vec<UiRequest> {
        let mut requests = Vec::new();
        if let Some(timer) = &self.power.timer
            && now >= timer.deadline
        {
            let ending = timer.ending;
            self.power.timer = None;
            match ending {
                Ending::Exit => {
                    requests.push(UiRequest::Exit);
                    return requests;
                }
                Ending::Power(_) => requests.push(self.start_countdown(ending, now)),
            }
        }
        let Some(countdown) = self.power.countdown.as_mut() else {
            return requests;
        };
        let left = countdown.deadline.saturating_duration_since(now);
        if left.is_zero() {
            let ending = countdown.ending;
            self.power.countdown = None;
            requests.push(UiRequest::PowerNotice(None));
            requests.push(match ending {
                Ending::Exit => UiRequest::Exit,
                Ending::Power(action) => UiRequest::Power(action),
            });
            return requests;
        }
        let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
        if countdown.shown != Some(seconds) {
            countdown.shown = Some(seconds);
            requests.push(UiRequest::PowerNotice(Some((
                ending_text(countdown.ending),
                seconds,
            ))));
        }
        requests
    }

    /// Misc's timer items: one that is set is cancelled, otherwise the
    /// timer's dialog opens.
    pub(crate) fn power_timer_command(&mut self, kind: PowerTimer) -> UiRequest {
        if kind != PowerTimer::Dialog
            && self
                .power
                .timer
                .as_ref()
                .is_some_and(|timer| timer.kind == kind)
        {
            self.power.timer = None;
            return UiRequest::Osd("Shutdown timer cancelled".to_owned());
        }
        UiRequest::PowerTimerDialog(kind)
    }

    /// The timer's dialog's answer: fire in `seconds` with `ending`;
    /// `label` names it in the menu.
    pub fn set_power_timer(
        &mut self,
        kind: PowerTimer,
        seconds: u64,
        exit: bool,
        label: &str,
        now: Instant,
    ) -> UiRequest {
        let ending = if exit {
            Ending::Exit
        } else {
            Ending::Power(self.settings.playback.finished.action)
        };
        let kind = if kind == PowerTimer::Dialog {
            PowerTimer::In
        } else {
            kind
        };
        self.power.timer = Some(Timer {
            kind,
            ending,
            deadline: now + Duration::from_secs(seconds.max(1)),
            label: label.to_owned(),
        });
        UiRequest::Osd(format!("{}: {label}", ending_text(ending)))
    }

    /// The timer items' menu text: `%s` is the timer's label when one of
    /// the kind is set.
    pub(crate) fn timer_label(&self, kind: PowerTimer, text: &str) -> String {
        let label = self
            .power
            .timer
            .as_ref()
            .filter(|timer| timer.kind == kind)
            .map_or("...", |timer| timer.label.as_str());
        text.replace("at %s", &format!("at {label}"))
            .replace("in %s", &format!("in {label}"))
            .replace("%s", label)
    }

    /// The countdown and the timer, for the window.
    #[must_use]
    pub fn power(&self) -> &Power {
        &self.power
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dry_run_hook_writes_the_action() {
        let path = std::env::temp_dir().join(format!("rvp-power-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        write_dry_run(&path, PowerAction::Sleep).expect("write");
        write_dry_run(&path, PowerAction::Hibernate).expect("write");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "sleep\nhibernate\n"
        );
        let _ = std::fs::remove_file(&path);
    }
}
