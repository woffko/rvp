//! Playback engine: the libmpv wrapper and the property model the skin and
//! the application read (milestone M2).
//!
//! This crate has no Qt dependency and no `unsafe` code: it uses the safe
//! API of the patched `libmpv2` crate (`vendor/libmpv2`). Tests run libmpv
//! headless (`vo=null`, `ao=null`) on generated `av://lavfi:` sources.
//! [`thumbnail`] is the second core that renders small pictures (M11).

pub mod calibrate;
pub mod encode;
mod engine;
mod model;
pub mod thumbnail;

pub use engine::{
    AudioOutput, Chapter, DemuxVideo, EndReason, Engine, EngineError, EngineEvent, EngineOptions,
    Image, Seek, Track, TrackKind, VideoOutput, default_ipc_server,
};
pub use libmpv2;
pub use model::{Changes, Kind, PlayerModel, PropertyId, Value};
