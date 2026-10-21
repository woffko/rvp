//! `SubRip` (SRT): numbered cues of `start --> end` and text lines.
//!
//! Reading is as tolerant as players are: indices may be missing or wrong
//! (they are renumbered on writing), milliseconds may follow a comma or a
//! dot, coordinates after the end time are ignored, and blank lines between
//! cues do not matter. A cue's text runs up to the next timing line (with
//! the index before it); blank lines inside it are dropped, since they would
//! end the cue for other players.

use std::fmt::Write as _;

use crate::text::{ass_to_tags, vtt_to_srt};
use crate::time::{format_clock, parse_clock};
use crate::{Cue, Document, Format};

/// The start and end of a timing line (`00:00:01,000 --> 00:00:02,500`).
pub(crate) fn timing(line: &str) -> Option<(i64, i64, &str)> {
    let (start, rest) = line.split_once("-->")?;
    let rest = rest.trim_start();
    let end_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let start = parse_clock(start)?;
    let end = parse_clock(&rest[..end_len])?;
    Some((start, end, rest[end_len..].trim()))
}

/// Text normalized to LF (see [`Document::parse_str`]).
pub(crate) fn parse(text: &str) -> Document {
    let lines: Vec<&str> = text.lines().collect();
    let timings: Vec<(usize, i64, i64)> = lines
        .iter()
        .enumerate()
        .filter_map(|(n, line)| timing(line).map(|(start, end, _)| (n, start, end)))
        .collect();
    let mut document = Document::new(Format::Srt);
    for (k, &(at, start_ms, end_ms)) in timings.iter().enumerate() {
        let mut until = timings.get(k + 1).map_or(lines.len(), |next| next.0);
        // The next cue's index.
        if k + 1 < timings.len() && until > at + 1 && is_index(lines[until - 1]) {
            until -= 1;
        }
        let text: Vec<&str> = lines[at + 1..until]
            .iter()
            .map(|line| line.trim_end())
            .filter(|line| !line.is_empty())
            .collect();
        document
            .cues
            .push(Cue::new(start_ms, end_ms, text.join("\n")));
    }
    document
}

fn is_index(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit())
}

/// The cues as SRT, numbered from 1, CRLF line ends; comments left out.
pub(crate) fn write(document: &Document) -> String {
    let mut out = String::new();
    let shown = document.cues.iter().filter(|cue| !cue.is_comment());
    for (n, cue) in shown.enumerate() {
        let text = match document.format {
            Format::Srt => cue.text.clone(),
            Format::Ass => ass_to_tags(&cue.text),
            Format::WebVtt => vtt_to_srt(&cue.text),
        };
        let _ = write!(
            out,
            "{}\r\n{} --> {}\r\n",
            n + 1,
            format_clock(cue.start_ms, ','),
            format_clock(cue.end_ms, ',')
        );
        for line in text.lines() {
            let line = line.trim_end();
            if !line.is_empty() {
                out.push_str(line);
                out.push_str("\r\n");
            }
        }
        out.push_str("\r\n");
    }
    out
}
