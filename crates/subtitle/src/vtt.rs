//! `WebVTT`: the `WEBVTT` header, then blocks parted by blank lines: cues
//! (an optional identifier, `start --> end` with settings, text) and the
//! `NOTE`, `STYLE` and `REGION` blocks, which are kept with the header (a
//! `NOTE` between cues moves up there). Hours are optional in times.

use crate::srt::timing;
use crate::text::{ass_to_tags, srt_to_vtt};
use crate::time::format_clock;
use crate::{Cue, Document, Format, VttCue};

/// Text normalized to LF (see [`Document::parse_str`]).
pub(crate) fn parse(text: &str) -> Document {
    let mut document = Document::new(Format::WebVtt);
    let mut header: Vec<&str> = Vec::new();
    let blocks = text.split("\n\n").map(|block| block.trim_matches('\n'));
    for (n, block) in blocks.filter(|block| !block.trim().is_empty()).enumerate() {
        let lines: Vec<&str> = block.lines().collect();
        let first = lines[0].trim_start();
        let kept = n == 0 && first.starts_with("WEBVTT")
            || ["NOTE", "STYLE", "REGION"]
                .iter()
                .any(|kind| first == *kind || first.starts_with(&format!("{kind} ")));
        if kept {
            header.push(block);
            continue;
        }
        let (id, timing_at) = if timing(lines[0]).is_some() {
            ("", 0)
        } else if lines.len() > 1 && timing(lines[1]).is_some() {
            (lines[0].trim(), 1)
        } else {
            continue;
        };
        let Some((start_ms, end_ms, settings)) = timing(lines[timing_at]) else {
            continue;
        };
        let mut cue = Cue::new(start_ms, end_ms, lines[timing_at + 1..].join("\n"));
        cue.vtt = Some(VttCue {
            id: id.to_owned(),
            settings: settings.to_owned(),
        });
        document.cues.push(cue);
    }
    if header.is_empty() {
        header.push("WEBVTT");
    }
    document.header = header.join("\n\n") + "\n";
    document
}

/// The cues as `WebVTT`, LF line ends; comments left out.
pub(crate) fn write(document: &Document) -> String {
    let mut out = if document.format == Format::WebVtt {
        document.header.trim_end().to_owned()
    } else {
        "WEBVTT".to_owned()
    };
    out.push_str("\n\n");
    for cue in document.cues.iter().filter(|cue| !cue.is_comment()) {
        let text = match document.format {
            Format::WebVtt => cue.text.clone(),
            Format::Srt => srt_to_vtt(&cue.text),
            Format::Ass => srt_to_vtt(&ass_to_tags(&cue.text)),
        };
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        if let Some(VttCue { id, .. }) = &cue.vtt
            && !id.is_empty()
        {
            out.push_str(id);
            out.push('\n');
        }
        out.push_str(&format_clock(cue.start_ms, '.'));
        out.push_str(" --> ");
        out.push_str(&format_clock(cue.end_ms, '.'));
        if let Some(VttCue { settings, .. }) = &cue.vtt
            && !settings.is_empty()
        {
            out.push(' ');
            out.push_str(settings);
        }
        out.push('\n');
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}
