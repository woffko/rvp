//! RVP's own reader and writer for text subtitles (M13, goal O): `SubRip`
//! (SRT), Advanced `SubStation` Alpha (ASS, and SSA v4) and `WebVTT`, and the
//! model of PotPlayer's subtitle browser and editor (Browse/Edit/Sync
//! Subtitles, Input Subtitles).
//!
//! mpv renders subtitles. RVP reads external text subtitle files itself to
//! save them with the sync applied or at another frame rate (PotPlayer's
//! Subtitle Saving and Convert Subtitle FPS), to show and edit their cues,
//! and to write subtitles that are typed in or made from the audio.
//!
//! * Reading takes a file's bytes: UTF-8 (with or without a BOM), UTF-16
//!   by its BOM, or a legacy code page that the caller names or that
//!   [`decode`] guesses. The format comes from the content, then from the
//!   file name.
//! * Times are milliseconds from the start of the video.
//! * Writing gives UTF-8 text without a BOM: CRLF line ends for SRT and ASS,
//!   which Windows players expect (PotPlayer writes them), and LF for
//!   `WebVTT`, whose specification uses LF.
//! * An ASS file read and written again keeps its header (the script info,
//!   the styles and every other section before the events), the sections
//!   after the events, each event's fields in the order of the events'
//!   `Format:` line, and its `Comment:` events. Other event types
//!   (`Picture:`, `Sound:`, `Movie:`, `Command:`) and blank lines inside the
//!   events are left out.
//! * Converting between formats keeps the timing and the text's lines:
//!   ASS override tags become SRT's and `WebVTT`'s `<i>`, `<b>`, `<u>` and
//!   `<s>` where they match and are dropped otherwise, and the other way
//!   round.

#![forbid(unsafe_code)]

mod ass;
mod decode;
mod document;
mod editor;
mod srt;
mod text;
mod time;
mod vtt;

pub use decode::{decode, resolve_encoding};
pub use document::{AssEvent, Cue, Document, Error, Format, VttCue};
pub use editor::Editor;
pub use time::{format_clock, parse_clock};
