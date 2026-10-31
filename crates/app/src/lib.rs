//! Application logic: commands, input bindings, settings and the settings
//! window's checks, the resume store, albums, the normalizer, single
//! instance, menus and their coverage of PotPlayer's, and the skin as the
//! window shows it (milestones M2 to M5).
//!
//! No Qt dependency and no `unsafe` code: everything here runs headless in
//! tests against the real engine. The shell only paints [`chrome::Frame`]s,
//! forwards input and carries out [`UiRequest`]s.

pub mod accessibility;
mod action;
pub mod albums;
mod app;
pub mod associations;
pub mod audio;
pub mod binding_editor;
pub mod capture;
pub mod chrome;
pub mod classic;
mod cli;
mod commands;
pub mod control;
pub mod controller;
pub mod coverage;
pub mod crash;
pub mod decoders;
pub mod disc;
pub mod dpl;
pub mod equalizer;
pub mod general;
pub mod gpu_power;
pub mod hlsl;
mod input;
pub mod instance;
pub mod logos;
pub mod marks;
pub mod media;
pub mod menu;
pub mod menu_design;
pub mod misc;
pub mod normalizer;
pub mod osd;
pub mod panel;
pub mod picture;
pub mod playback;
pub mod playlist;
pub mod playlist_files;
pub mod pointer;
pub mod power;
pub mod preferences;
pub mod presets;
pub mod record;
pub mod registration;
pub mod render;
mod resume;
pub mod scenes;
mod settings;
pub mod shaders;
pub mod skins;
pub mod smoothing;
pub mod sources;
pub mod speakers;
pub mod spectrum;
pub mod speech;
pub mod stereo;
mod storage;
pub mod subtitle_editor;
pub mod subtitles;
pub mod svp;
pub mod thumbs;
pub mod tomsteady;
pub mod video;
pub mod view;
pub mod visual;
pub mod vlc;
pub mod volume;
pub mod window;
pub mod ytdl;

pub use action::{
    ACTIONS, AbRepeat, Action, ActionInfo, AfterPlayback, Aspect, BookmarkCommand, CopyName,
    DefaultOpen, Delay, Fit, ImageFormat, Jump, ListAction, OnTop, PowerAction, PowerTimer,
    RVP_COMMAND, Repeat, Scenes, SeekStep, SettingsPage, Skip, SortField, Source, Speed,
    SubtitleStyle, WhenOpened, WindowScale, Zoom,
};
pub use app::{App, Drained, UiRequest};
pub use cli::Cli;
pub use commands::Session;
pub use input::{Bindings, Key, KeyChord, MouseGesture};
pub use resume::{EDGE_SECONDS, Position, ResumeState, ResumeStore, SAVE_INTERVAL, WindowGeometry};
pub use settings::Settings;
pub use storage::{Dirs, PORTABLE_MARKER, read_json, write_atomic, write_json};
