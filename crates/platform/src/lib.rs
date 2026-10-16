//! Desktop integration (milestone M8).
//!
//! Linux: MPRIS (`org.mpris.MediaPlayer2` and its `Player` interface) and
//! idle inhibit over the session D-Bus ([`Desktop`]). The XDG desktop
//! portal's `Inhibit` keeps the screen on while video plays, with
//! `org.freedesktop.ScreenSaver` where there is no portal. Bus calls run on
//! a thread of the desktop's own, so the window's thread never waits on
//! the bus; MPRIS's method calls arrive on zbus's thread and go to the
//! window through a callback.
//!
//! Windows' media controls need the window's handle and COM calls, which
//! RVP keeps in the shell (`shell/src/media_controls.rs`, AGENTS.md's
//! `unsafe` rule).

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{Desktop, Options};
