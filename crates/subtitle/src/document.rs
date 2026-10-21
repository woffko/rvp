//! The subtitle model: a document of timed cues, read from and written to
//! SRT, ASS and `WebVTT`.

use std::fmt;

use crate::{ass, decode, srt, vtt};

/// A text subtitle format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Format {
    /// `SubRip` (`.srt`).
    Srt,
    /// Advanced `SubStation` Alpha (`.ass`), and `SubStation` Alpha v4
    /// (`.ssa`), which is read the same way.
    Ass,
    /// `WebVTT` (`.vtt`).
    WebVtt,
}

impl Format {
    /// The format a file name's extension names, if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Format> {
        let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
        match extension.as_str() {
            "srt" => Some(Format::Srt),
            "ass" | "ssa" => Some(Format::Ass),
            "vtt" | "webvtt" => Some(Format::WebVtt),
            _ => None,
        }
    }

    /// The extension files of this format get, without the dot.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Format::Srt => "srt",
            Format::Ass => "ass",
            Format::WebVtt => "vtt",
        }
    }
}

/// An ASS event's fields beyond a cue's times, style and text, as read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssEvent {
    /// A `Comment:` event (not shown) rather than a `Dialogue:` one.
    pub comment: bool,
    /// The event's fields in the order of the events' `Format:` line, as
    /// read. Writing takes `Start`, `End`, `Style` and `Text` from the cue.
    pub fields: Vec<String>,
}

/// A `WebVTT` cue's identifier and settings, as read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VttCue {
    /// The identifier line before the timing; empty when there is none.
    pub id: String,
    /// The cue settings after the timing (`line:0 align:start`...).
    pub settings: String,
}

/// One subtitle: shown from `start_ms` up to `end_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    pub start_ms: i64,
    pub end_ms: i64,
    /// The lines, joined by `'\n'`. ASS cues keep their override tags
    /// (`{\i1}`); SRT and `WebVTT` cues their tags (`<i>`).
    pub text: String,
    /// The ASS style; `None` for SRT and `WebVTT` cues.
    pub style: Option<String>,
    /// The other ASS fields, for cues read from ASS.
    pub ass: Option<AssEvent>,
    /// The `WebVTT` identifier and settings, for cues read from `WebVTT`.
    pub vtt: Option<VttCue>,
}

impl Cue {
    /// A cue with text only.
    #[must_use]
    pub fn new(start_ms: i64, end_ms: i64, text: impl Into<String>) -> Cue {
        Cue {
            start_ms,
            end_ms,
            text: text.into(),
            style: None,
            ass: None,
            vtt: None,
        }
    }

    /// An ASS `Comment:` event: kept in the file, never shown.
    #[must_use]
    pub fn is_comment(&self) -> bool {
        self.ass.as_ref().is_some_and(|event| event.comment)
    }

    /// Shown at `ms`: from its start up to, not including, its end.
    #[must_use]
    pub fn shows_at(&self, ms: i64) -> bool {
        !self.is_comment() && self.start_ms <= ms && ms < self.end_ms
    }
}

/// Why a subtitle file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The encoding's label is none that RVP knows.
    UnknownEncoding(String),
    /// Neither the content nor the file name tells the format.
    UnknownFormat,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownEncoding(label) => write!(f, "unknown text encoding {label:?}"),
            Error::UnknownFormat => f.write_str("not an SRT, ASS or WebVTT subtitle file"),
        }
    }
}

impl std::error::Error for Error {}

/// A subtitle file: its cues and what the writer keeps of the original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The format it was read in, or is made for.
    pub format: Format,
    /// The encoding its bytes were read in (encoding's name: `UTF-8`,
    /// `UTF-16LE`, `windows-1251`, `EUC-KR`...).
    pub encoding: &'static str,
    /// ASS: every line before `[Events]` (the script info, the styles and
    /// other sections). `WebVTT`: the `WEBVTT` line with its header and the
    /// `NOTE`, `STYLE` and `REGION` blocks. Lines joined by `'\n'`, each one
    /// ended. SRT: empty.
    pub header: String,
    /// ASS: the field names of the events' `Format:` line.
    pub ass_format: Vec<String>,
    /// ASS: every line of the sections after the events, each one ended.
    pub trailer: String,
    pub cues: Vec<Cue>,
}

impl Document {
    /// An empty document of `format`.
    #[must_use]
    pub fn new(format: Format) -> Document {
        Document {
            format,
            encoding: "UTF-8",
            header: String::new(),
            ass_format: Vec::new(),
            trailer: String::new(),
            cues: Vec::new(),
        }
    }

    /// Reads a subtitle file's bytes, guessing the encoding when there is
    /// no BOM and the bytes are not UTF-8 ([`decode`]). `name_hint`, the
    /// file's name, settles the format when the content does not.
    ///
    /// # Errors
    /// [`Error::UnknownFormat`] when neither content nor name tell the
    /// format.
    pub fn parse(bytes: &[u8], name_hint: Option<&str>) -> Result<Document, Error> {
        Document::parse_with_encoding(bytes, "auto", name_hint)
    }

    /// Reads a subtitle file's bytes in the encoding `encoding` names
    /// ([`crate::resolve_encoding`]; `auto` or empty: as [`Document::parse`]).
    /// A BOM still wins.
    ///
    /// # Errors
    /// [`Error::UnknownEncoding`] for a label RVP does not know,
    /// [`Error::UnknownFormat`] as [`Document::parse`].
    pub fn parse_with_encoding(
        bytes: &[u8],
        encoding: &str,
        name_hint: Option<&str>,
    ) -> Result<Document, Error> {
        let (text, encoding) = decode::decode(bytes, encoding)?;
        let mut document = Document::parse_str(&text, name_hint)?;
        document.encoding = encoding;
        Ok(document)
    }

    /// Reads subtitle text.
    ///
    /// # Errors
    /// [`Error::UnknownFormat`] as [`Document::parse`].
    pub fn parse_str(text: &str, name_hint: Option<&str>) -> Result<Document, Error> {
        let text = normalize_newlines(text.trim_start_matches('\u{feff}'));
        let hint = name_hint.and_then(Format::from_name);
        let format = detect(&text, hint).ok_or(Error::UnknownFormat)?;
        Ok(match format {
            Format::Srt => srt::parse(&text),
            Format::Ass => ass::parse(&text),
            Format::WebVtt => vtt::parse(&text),
        })
    }

    /// The document as text of `format` (see the crate's documentation
    /// for line ends and conversions).
    #[must_use]
    pub fn write(&self, format: Format) -> String {
        match format {
            Format::Srt => srt::write(self),
            Format::Ass => ass::write(self),
            Format::WebVtt => vtt::write(self),
        }
    }

    /// [`Document::write`] as the bytes of a UTF-8 file.
    #[must_use]
    pub fn to_bytes(&self, format: Format) -> Vec<u8> {
        self.write(format).into_bytes()
    }

    /// The cue shown at `ms`; the latest to start when several overlap.
    #[must_use]
    pub fn cue_at(&self, ms: i64) -> Option<usize> {
        self.cues
            .iter()
            .enumerate()
            .filter(|(_, cue)| cue.shows_at(ms))
            .max_by_key(|(index, cue)| (cue.start_ms, *index))
            .map(|(index, _)| index)
    }

    /// The first cue that starts after `ms`.
    #[must_use]
    pub fn next_cue(&self, ms: i64) -> Option<usize> {
        self.cues
            .iter()
            .enumerate()
            .filter(|(_, cue)| !cue.is_comment() && cue.start_ms > ms)
            .min_by_key(|(index, cue)| (cue.start_ms, *index))
            .map(|(index, _)| index)
    }

    /// Moves every cue by `delay_ms` (PotPlayer's Apply Subtitle Sync, mpv's
    /// `sub-delay`: positive shows the subtitles later). A cue that would
    /// start before 0 starts at 0; one that would end at or before 0 is
    /// dropped, as no player could show it.
    pub fn shift(&mut self, delay_ms: i64) {
        crate::time::shift(&mut self.cues, delay_ms);
    }

    /// Converts the timing of a subtitle made for a video at `from_fps` to
    /// the same video at `to_fps` (PotPlayer's Convert Subtitle FPS):
    /// every time is multiplied by `from_fps / to_fps`. mpv does the same
    /// at play time with `sub-fps` = `from_fps` ("the framerate of the
    /// subtitle file"): it shows a subtitle's time `t` at `t * sub-fps /
    /// video fps`. A subtitle timed for the 25 fps (PAL) release of a film
    /// thus gets longer for the 23.976 fps release, which runs slower.
    /// Rates that are not positive and finite change nothing.
    pub fn scale(&mut self, from_fps: f64, to_fps: f64) {
        crate::time::scale(&mut self.cues, from_fps, to_fps);
    }

    /// Orders the cues by start, keeping the order of cues that start
    /// together.
    pub fn sort(&mut self) {
        self.cues.sort_by_key(|cue| cue.start_ms);
    }
}

/// The format the content shows: `WebVTT`'s signature, ASS's sections, or
/// timing arrows, which are SRT's unless the name says `WebVTT` (a file
/// without its signature). Without any of them, the name's format.
fn detect(text: &str, hint: Option<Format>) -> Option<Format> {
    if text.trim_start().starts_with("WEBVTT") {
        return Some(Format::WebVtt);
    }
    let ass_section = text.lines().any(|line| {
        let line = line.trim().to_ascii_lowercase();
        matches!(
            line.as_str(),
            "[script info]" | "[events]" | "[v4+ styles]" | "[v4 styles]"
        )
    });
    if ass_section {
        return Some(Format::Ass);
    }
    if text.contains("-->") {
        return Some(if hint == Some(Format::WebVtt) {
            Format::WebVtt
        } else {
            Format::Srt
        });
    }
    hint
}

/// CRLF and lone CR become LF.
fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
