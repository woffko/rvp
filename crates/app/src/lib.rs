//! Application logic: commands, input bindings, settings, the resume
//! store, and the skin as the window shows it (milestone M2; playlists and
//! single instance come in M3).
//!
//! No Qt dependency and no `unsafe` code: everything here runs headless in
//! tests against the real engine. The shell only paints [`chrome::Frame`]s,
//! forwards input and carries out [`UiRequest`]s.

mod action;
mod app;
pub mod chrome;
mod cli;
mod input;
pub mod pointer;
mod resume;
mod settings;
mod storage;
pub mod view;

pub use action::{Action, SeekStep};
pub use app::{App, Drained, UiRequest};
pub use cli::Cli;
pub use input::{Bindings, Key, KeyChord, MouseGesture};
pub use resume::{EDGE_SECONDS, Position, ResumeState, ResumeStore, SAVE_INTERVAL, WindowGeometry};
pub use settings::Settings;
pub use storage::{Dirs, read_json, write_atomic, write_json};
