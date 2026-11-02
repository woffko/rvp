//! Desktop integration (milestone M8).
//!
//! Linux: MPRIS (`org.mpris.MediaPlayer2` and its `Player` interface) and
//! idle inhibit over the session D-Bus ([`Desktop`]). GNOME's session
//! manager keeps the screen on while video plays, else
//! `org.freedesktop.ScreenSaver`, else the XDG desktop portal (in a
//! sandbox; outside one, GNOME refuses the portal's inhibit). Bus calls
//! run on a thread of the desktop's own, so the window's thread never
//! waits on the bus; MPRIS's method calls arrive on zbus's thread and go
//! to the window through a callback.
//!
//! Windows' media controls need the window's handle and COM calls, which
//! RVP keeps in the shell (`shell/src/media_controls.rs`, AGENTS.md's
//! `unsafe` rule).
//!
//! Linux's counterparts of the shell's Windows calls (goal S): the trash
//! ([`trash`]), power actions through logind ([`power`]), the system's
//! volume and RVP's own stream through PulseAudio's tools ([`pulse`]), and
//! the desktop's recent files ([`recent`]) and the processors' time for the
//! log ([`load`]).

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub mod load;
#[cfg(target_os = "linux")]
pub mod power;
#[cfg(target_os = "linux")]
pub mod pulse;
#[cfg(target_os = "linux")]
pub mod recent;
#[cfg(target_os = "linux")]
pub mod trash;

#[cfg(target_os = "linux")]
pub use linux::{Desktop, Options};
