//! Cue text between the formats' markup: ASS override blocks (`{\i1}`)
//! and SRT's and `WebVTT`'s tags (`<i>`). Italic, bold, underline and
//! strike-out cross over; SRT's font colour becomes ASS's primary colour and
//! back is dropped; everything else is dropped.

use std::fmt::Write as _;

/// The four styles all three formats share, with their tag letters.
const SHARED: [char; 4] = ['i', 'b', 'u', 's'];

/// ASS text for SRT or `WebVTT`: the shared styles as tags, the other
/// override tags, vector drawings (`\p1` up to `\p0`) and comments dropped,
/// hard spaces (`\h`) and soft breaks (`\n`) as spaces.
pub(crate) fn ass_to_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut open: Vec<char> = Vec::new();
    let mut drawing = false;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '{'
            && let Some(end) = rest.find('}')
        {
            for (tag, on) in override_tags(&rest[1..end]) {
                apply_override(tag, on, &mut open, &mut drawing, &mut out);
            }
            rest = &rest[end + 1..];
            continue;
        }
        if c == '\\' {
            let after = &rest[1..];
            if let Some('h' | 'n') = after.chars().next() {
                if !drawing {
                    out.push(' ');
                }
                rest = &after[1..];
            } else {
                if !drawing {
                    out.push('\\');
                }
                rest = after;
            }
            continue;
        }
        if !drawing {
            out.push(c);
        }
        rest = &rest[c.len_utf8()..];
    }
    while let Some(style) = open.pop() {
        let _ = write!(out, "</{style}>");
    }
    out
}

/// One override tag of [`override_tags`] on the open tags of
/// [`ass_to_tags`].
fn apply_override(tag: char, on: bool, open: &mut Vec<char>, drawing: &mut bool, out: &mut String) {
    match tag {
        'r' => {
            while let Some(style) = open.pop() {
                let _ = write!(out, "</{style}>");
            }
        }
        'p' => *drawing = on,
        style => {
            if on && !open.contains(&style) {
                open.push(style);
                let _ = write!(out, "<{style}>");
            } else if !on && let Some(at) = open.iter().position(|o| *o == style) {
                // The ones opened later close too, to keep the tags nested,
                // and open again.
                let later: Vec<char> = open.drain(at..).collect();
                for closed in later.iter().rev() {
                    let _ = write!(out, "</{closed}>");
                }
                for reopened in &later[1..] {
                    open.push(*reopened);
                    let _ = write!(out, "<{reopened}>");
                }
            }
        }
    }
}

/// The tags of an ASS override block that matter here, each with whether
/// it turns on: the shared styles (`\i1`, `\b0`, bold's weights such as
/// `\b700`), drawing mode (`\p1`, `\p0`) and reset (`\r`, `\rStyle`).
fn override_tags(block: &str) -> Vec<(char, bool)> {
    let mut tags = Vec::new();
    for tag in block.split('\\').skip(1) {
        let mut chars = tag.chars();
        let Some(name) = chars.next() else { continue };
        let value: String = chars.take_while(char::is_ascii_digit).collect();
        let on = value.parse::<u32>().ok().is_none_or(|n| n != 0);
        let exact = !value.is_empty() && tag.trim_end().len() == 1 + value.len();
        match name {
            'r' => tags.push(('r', false)),
            'p' if exact => tags.push(('p', on)),
            style if SHARED.contains(&style) && exact => tags.push((style, on)),
            _ => {}
        }
    }
    tags
}

/// SRT or `WebVTT` text for ASS: the shared styles as override tags, SRT's
/// `<font color="#rrggbb">` as ASS's colour, the other tags dropped and
/// `WebVTT`'s entities read.
pub(crate) fn tags_to_ass(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    each_tag(text, |piece| match piece {
        Piece::Text(text) => out.push_str(&unescape(text)),
        Piece::Tag(tag) => {
            let (closing, name, attributes) = split_tag(tag);
            match name.as_str() {
                style
                    if style.len() == 1
                        && SHARED.contains(&style.chars().next().unwrap_or(' ')) =>
                {
                    let _ = write!(out, "{{\\{style}{}}}", u8::from(!closing));
                }
                "font" if closing => out.push_str("{\\c}"),
                "font" => {
                    if let Some(colour) = font_colour(attributes) {
                        let _ = write!(out, "{{\\c&H{colour}&}}");
                    }
                }
                _ => {}
            }
        }
    });
    out
}

/// `WebVTT` text for SRT: the shared styles kept, `WebVTT`'s other tags
/// (classes, voices, languages, ruby, timestamps) dropped and its entities
/// read.
pub(crate) fn vtt_to_srt(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    each_tag(text, |piece| match piece {
        Piece::Text(text) => out.push_str(&unescape(text)),
        Piece::Tag(tag) => {
            let (closing, name, _) = split_tag(tag);
            if name.len() == 1 && SHARED.iter().any(|s| name.starts_with(*s)) {
                out.push_str(if closing { "</" } else { "<" });
                out.push_str(&name);
                out.push('>');
            }
        }
    });
    out
}

/// SRT text for `WebVTT`: the shared styles kept, font tags dropped, and
/// `&` escaped as `WebVTT` needs.
pub(crate) fn srt_to_vtt(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    each_tag(text, |piece| match piece {
        Piece::Text(text) => out.push_str(&text.replace('&', "&amp;")),
        Piece::Tag(tag) => {
            let (closing, name, _) = split_tag(tag);
            if name.len() == 1 && SHARED.iter().any(|s| name.starts_with(*s)) {
                out.push_str(if closing { "</" } else { "<" });
                out.push_str(&name);
                out.push('>');
            }
        }
    });
    out
}

enum Piece<'a> {
    Text(&'a str),
    /// What is between `<` and `>`.
    Tag(&'a str),
}

/// Calls `visit` with the text and the tags of SRT or `WebVTT` cue text. A
/// `<` without a closing `>` is text.
fn each_tag(text: &str, mut visit: impl FnMut(Piece<'_>)) {
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        if open > 0 {
            visit(Piece::Text(&rest[..open]));
        }
        visit(Piece::Tag(&rest[open + 1..open + close]));
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        visit(Piece::Text(rest));
    }
}

/// A tag's parts: closing or not, its lowercase name (without `WebVTT`
/// classes), and its attributes.
fn split_tag(tag: &str) -> (bool, String, &str) {
    let (closing, tag) = match tag.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, tag),
    };
    let tag = tag.trim();
    let end = tag
        .find(|c: char| c.is_whitespace() || c == '.')
        .unwrap_or(tag.len());
    (closing, tag[..end].to_ascii_lowercase(), &tag[end..])
}

/// `color="#rrggbb"` of a font tag as ASS's `bbggrr`.
fn font_colour(attributes: &str) -> Option<String> {
    let lower = attributes.to_ascii_lowercase();
    let at = lower.find("color")?;
    let value = lower[at + 5..].trim_start().strip_prefix('=')?.trim_start();
    let value = value.trim_start_matches(['"', '\'']).strip_prefix('#')?;
    let hex: String = value.chars().take(6).collect();
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("{}{}{}", &hex[4..6], &hex[2..4], &hex[0..2]).to_ascii_uppercase())
}

/// `WebVTT`'s character references in text.
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", "\u{a0}")
        .replace("&lrm;", "\u{200e}")
        .replace("&rlm;", "\u{200f}")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ass_overrides_become_tags_and_the_rest_goes() {
        assert_eq!(ass_to_tags(r"{\i1}Hello{\i0} world"), "<i>Hello</i> world");
        assert_eq!(ass_to_tags(r"{\pos(10,20)\b1}Bold"), "<b>Bold</b>");
        assert_eq!(ass_to_tags(r"{\b700}Heavy{\b0}"), "<b>Heavy</b>");
        assert_eq!(ass_to_tags(r"a\hb\nc"), "a b c");
        assert_eq!(ass_to_tags(r"{\p1}m 0 0 l 10 10{\p0}Text"), "Text");
        assert_eq!(ass_to_tags(r"{a comment}Text"), "Text");
        assert_eq!(
            ass_to_tags(r"{\i1\b1}x{\i0}y{\r}z"),
            "<i><b>x</b></i><b>y</b>z"
        );
        assert_eq!(ass_to_tags(r"{\bord2}{\blur3}plain"), "plain");
    }

    #[test]
    fn tags_become_ass_overrides() {
        assert_eq!(
            tags_to_ass("<i>Hi</i> <B>there</B>"),
            r"{\i1}Hi{\i0} {\b1}there{\b0}"
        );
        assert_eq!(
            tags_to_ass(r##"<font color="#FF8000">Orange</font>"##),
            r"{\c&H0080FF&}Orange{\c}"
        );
        assert_eq!(tags_to_ass("<v Bob>Hi &amp; bye</v>"), "Hi & bye");
        assert_eq!(tags_to_ass("<c.yellow>Hi</c>"), "Hi");
    }

    #[test]
    fn webvtt_and_srt_tags_meet_halfway() {
        assert_eq!(
            vtt_to_srt("<v Ann><i>Hi</i></v> <00:00:01.000>you &lt;3"),
            "<i>Hi</i> you <3"
        );
        assert_eq!(
            srt_to_vtt(r##"<font color="#fff">Tom & Jerry</font>"##),
            "Tom &amp; Jerry"
        );
        assert_eq!(srt_to_vtt("a < b"), "a < b");
    }
}
