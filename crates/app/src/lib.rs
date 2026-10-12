//! Application logic: commands, input bindings, settings and the settings
//! window's checks, the resume store, albums, the normalizer, single
//! instance, menus and their coverage of PotPlayer's, and the skin as the
//! window shows it (milestones M2 to M5).
//!
//! No Qt dependency and no `unsafe` code: everything here runs headless in
//! tests against the real engine. The shell only paints [`chrome::Frame`]s,
//! forwards input and carries out [`UiRequest`]s.

mod action;
mod app;
pub mod binding_editor;
pub mod chrome;
mod cli;
mod commands;
pub mod coverage;
pub mod dpl;
mod input;
pub mod instance;
pub mod menu;
pub mod normalizer;
pub mod playlist;
pub mod pointer;
pub mod preferences;
mod resume;
mod settings;
mod storage;
pub mod view;

pub use action::{
    ACTIONS, AbRepeat, Action, ActionInfo, AfterPlayback, Aspect, Delay, Fit, ImageFormat, Jump,
    ListAction, OnTop, Repeat, SeekStep, SettingsPage, Speed, SubtitleStyle, WindowScale, Zoom,
};
pub use app::{App, Drained, UiRequest};
pub use cli::Cli;
pub use input::{Bindings, Key, KeyChord, MouseGesture};
pub use resume::{EDGE_SECONDS, Position, ResumeState, ResumeStore, SAVE_INTERVAL, WindowGeometry};
pub use settings::Settings;
pub use storage::{Dirs, PORTABLE_MARKER, read_json, write_atomic, write_json};
