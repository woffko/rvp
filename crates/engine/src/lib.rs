//! Playback engine: the libmpv wrapper and the property model the skin and
//! the application read (milestone M2).
//!
//! This crate has no Qt dependency and no `unsafe` code: it uses the safe
//! API of the patched `libmpv2` crate (`vendor/libmpv2`). Tests run libmpv
//! headless (`vo=null`, `ao=null`) on generated `av://lavfi:` sources.

mod engine;
mod model;

pub use engine::{
    AudioOutput, Chapter, EndReason, Engine, EngineError, EngineEvent, EngineOptions, Seek, Track,
    TrackKind, VideoOutput, default_ipc_server,
};
pub use libmpv2;
pub use model::{Changes, Kind, PlayerModel, PropertyId, Value};
