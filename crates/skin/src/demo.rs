//! A small text format for preview content, so renders can show realistic
//! data without code changes.
//!
//! ```text
//! # comment
//! text CMD_TITLE_MESSAGE = [1/3] Film.mkv
//! slider CMD_VOLUME_CTRL = 0.22
//! slider CMD_TIMEBAR_CTRL = 0 disabled
//! album Default
//! album* Films                      # the selected album
//! entry 01:37:18 | Film one.mkv
//! entry *+ 02:08:49 | Film two.mkv  # * current, + selected, ~ played, ! cannot open
//! scroll 64                         # first visible entry; `scroll end` aligns the last
//! playing | maximized | inactive | media | detached | page 4 | flag NAME
//! ```

use crate::state::{DataModel, ListScroll, PlaylistEntry, SliderValue, UiState};

/// Preview content: what the skin shows and the interface state.
#[derive(Debug, Clone, PartialEq)]
pub struct Demo {
    pub data: DataModel,
    pub state: UiState,
}

/// Parses preview content.
///
/// # Errors
/// Reports the first line that cannot be read.
pub fn parse(text: &str) -> Result<Demo, String> {
    let mut demo = Demo {
        data: DataModel::default(),
        state: UiState::default(),
    };
    for (index, raw) in text.lines().enumerate() {
        let line = raw.split(" #").next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        parse_line(&mut demo, line).map_err(|reason| format!("line {}: {reason}", index + 1))?;
    }
    Ok(demo)
}

fn parse_line(demo: &mut Demo, line: &str) -> Result<(), String> {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    let rest = rest.trim();
    match word {
        "text" => {
            let (command, value) = rest.split_once('=').ok_or("expected CMD = value")?;
            demo.data.set_text(command.trim(), value.trim());
        }
        "slider" => {
            let (command, value) = rest.split_once('=').ok_or("expected CMD = value")?;
            let mut parts = value.split_whitespace();
            let number: f32 = parts
                .next()
                .ok_or("missing value")?
                .parse()
                .map_err(|_| "value is not a number")?;
            demo.data.sliders.insert(
                command.trim().to_owned(),
                SliderValue {
                    value: number,
                    enabled: parts.next() != Some("disabled"),
                },
            );
        }
        "album" | "album*" => {
            if word == "album*" {
                demo.data.playlist.current_album = demo.data.playlist.albums.len();
            }
            demo.data.playlist.albums.push(rest.to_owned());
        }
        "entry" => parse_entry(demo, rest)?,
        "scroll" => {
            demo.data.playlist.scroll = if rest == "end" {
                ListScroll::End
            } else {
                ListScroll::Entry(rest.parse().map_err(|_| "scroll is not a number")?)
            };
        }
        "playing" => demo.state.playing = true,
        "maximized" => demo.state.maximized = true,
        "inactive" => demo.state.window_active = false,
        "media" => demo.state.media_loaded = true,
        "detached" => demo.state.playlist_attached = false,
        "page" => demo.state.current_page = rest.parse().map_err(|_| "page is not a number")?,
        "flag" => {
            demo.state.flags.insert(rest.to_owned());
        }
        other => return Err(format!("unknown keyword {other}")),
    }
    Ok(())
}

fn parse_entry(demo: &mut Demo, rest: &str) -> Result<(), String> {
    let (head, title) = rest
        .split_once('|')
        .ok_or("expected [flags] duration | title")?;
    let mut parts: Vec<&str> = head.split_whitespace().collect();
    let duration_text = parts.pop().ok_or("missing duration")?;
    let flags = parts.concat();
    let duration = if duration_text == "-" {
        None
    } else {
        let mut seconds = 0_u32;
        for part in duration_text.split(':') {
            let value: u32 = part.parse().map_err(|_| "bad duration")?;
            seconds = seconds * 60 + value;
        }
        Some(seconds)
    };
    let index = demo.data.playlist.entries.len();
    if flags.contains('*') {
        demo.data.playlist.current = Some(index);
    }
    if flags.contains('+') {
        demo.data.playlist.selected.insert(index);
    }
    demo.data.playlist.entries.push(PlaylistEntry {
        title: title.trim().to_owned(),
        duration,
        playable: !flags.contains('!'),
        played: flags.contains('~'),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::state::ListScroll;

    #[test]
    fn reads_all_line_kinds() {
        let demo = parse(
            "# preview\ntext CMD_TITLE_MESSAGE = A = B\nslider CMD_TIMEBAR_CTRL = 0.5 disabled\n\
             album Default\nalbum* Films\nentry 01:00:01 | One\nentry *+! - | Two # note\n\
             scroll 1\nplaying\npage 4\nflag ID_AUDIO_EQULIZER",
        )
        .expect("parse");
        assert_eq!(demo.data.text("CMD_TITLE_MESSAGE"), Some("A = B"));
        assert!(!demo.data.sliders["CMD_TIMEBAR_CTRL"].enabled);
        assert_eq!(demo.data.playlist.current_album, 1);
        assert_eq!(demo.data.playlist.entries[0].duration, Some(3601));
        assert_eq!(demo.data.playlist.current, Some(1));
        assert!(!demo.data.playlist.entries[1].playable);
        assert_eq!(demo.data.playlist.scroll, ListScroll::Entry(1));
        assert!(demo.state.playing);
        assert_eq!(demo.state.current_page, 4);
        assert!(demo.state.flag("ID_AUDIO_EQULIZER"));
        assert!(parse("bogus").is_err());
    }
}
