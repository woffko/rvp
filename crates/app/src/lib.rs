//! Application logic: commands, input bindings, settings and the resume
//! store (milestone M2; playlists and single instance come in M3).
//!
//! No Qt dependency and no `unsafe` code: everything here runs headless in
//! tests against the real engine.

mod action;
mod app;
mod cli;
mod input;
mod resume;
mod settings;
mod storage;

pub use action::{Action, SeekStep};
pub use app::{App, UiRequest};
pub use cli::Cli;
pub use input::{Bindings, Key, KeyChord, MouseGesture};
pub use resume::{EDGE_SECONDS, Position, ResumeState, ResumeStore, SAVE_INTERVAL, WindowGeometry};
pub use settings::Settings;
pub use storage::{Dirs, read_json, write_atomic, write_json};
