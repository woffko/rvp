//! Application logic: commands, input bindings, settings, the resume
//! store, albums, the normalizer, single instance, and the skin as the
//! window shows it (milestones M2 and M3).
//!
//! No Qt dependency and no `unsafe` code: everything here runs headless in
//! tests against the real engine. The shell only paints [`chrome::Frame`]s,
//! forwards input and carries out [`UiRequest`]s.

mod action;
mod app;
pub mod binding_editor;
pub mod chrome;
mod cli;
pub mod dpl;
mod input;
pub mod instance;
pub mod normalizer;
pub mod playlist;
pub mod pointer;
mod resume;
mod settings;
mod storage;
pub mod view;

pub use action::{Action, ListAction, SeekStep};
pub use app::{App, Drained, UiRequest};
pub use cli::Cli;
pub use input::{Bindings, Key, KeyChord, MouseGesture};
pub use resume::{EDGE_SECONDS, Position, ResumeState, ResumeStore, SAVE_INTERVAL, WindowGeometry};
pub use settings::Settings;
pub use storage::{Dirs, read_json, write_atomic, write_json};
