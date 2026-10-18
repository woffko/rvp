//! Application logic: commands, input bindings, settings and the settings
//! window's checks, the resume store, albums, the normalizer, single
//! instance, menus and their coverage of PotPlayer's, and the skin as the
//! window shows it (milestones M2 to M5).
//!
//! No Qt dependency and no `unsafe` code: everything here runs headless in
//! tests against the real engine. The shell only paints [`chrome::Frame`]s,
//! forwards input and carries out [`UiRequest`]s.

mod action;
pub mod albums;
mod app;
pub mod associations;
pub mod audio;
pub mod binding_editor;
pub mod chrome;
mod cli;
mod commands;
pub mod coverage;
pub mod crash;
pub mod dpl;
pub mod equalizer;
mod input;
pub mod instance;
pub mod marks;
pub mod media;
pub mod menu;
pub mod normalizer;
pub mod panel;
pub mod picture;
pub mod playback;
pub mod playlist;
pub mod playlist_files;
pub mod pointer;
pub mod power;
pub mod preferences;
mod resume;
pub mod scenes;
mod settings;
pub mod sources;
pub mod speakers;
pub mod stereo;
mod storage;
pub mod svp;
pub mod thumbs;
pub mod tomsteady;
pub mod video;
pub mod view;
pub mod volume;
pub mod window;
pub mod ytdl;

pub use action::{
    ACTIONS, AbRepeat, Action, ActionInfo, AfterPlayback, Aspect, BookmarkCommand, CopyName,
    DefaultOpen, Delay, Fit, ImageFormat, Jump, ListAction, OnTop, PowerAction, PowerTimer, Repeat,
    Scenes, SeekStep, SettingsPage, Skip, SortField, Source, Speed, SubtitleStyle, WhenOpened,
    WindowScale, Zoom,
};
pub use app::{App, Drained, UiRequest};
pub use cli::Cli;
pub use commands::Session;
pub use input::{Bindings, Key, KeyChord, MouseGesture};
pub use resume::{EDGE_SECONDS, Position, ResumeState, ResumeStore, SAVE_INTERVAL, WindowGeometry};
pub use settings::Settings;
pub use storage::{Dirs, PORTABLE_MARKER, read_json, write_atomic, write_json};
