//! What the desktop's media controls show and ask for (M8): MPRIS on Linux
//! (`rvp-desktop`), the System Media Transport Controls on Windows (the
//! shell). Both read [`MediaState`] and send [`MediaCommand`]s, which run
//! as RVP's own actions do (Stop is "Close Playback", as the skin's stop
//! button).

use std::fmt::Write as _;

use rvp_engine::{Engine, PropertyId, Seek};

use crate::action::Action;
use crate::app::{App, UiRequest};

/// Whether something plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    Stopped,
}

/// The playback state as the media controls show it.
// The four `can_*` flags are independent capabilities, as MPRIS names them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct MediaState {
    pub status: Status,
    /// Changes with each file that loads (MPRIS's track IDs).
    pub track: u64,
    /// The window's title for the file (album position and name).
    pub title: String,
    /// The file as a `file://` URL, or the stream's URL.
    pub url: Option<String>,
    /// Seconds; `None` without a known length.
    pub length: Option<f64>,
    /// Seconds.
    pub position: f64,
    /// 1.0 is 100 %.
    pub volume: f64,
    pub rate: f64,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
    /// A file is loaded or the album has an entry to start.
    pub can_play: bool,
}

/// What the media controls ask for.
#[derive(Debug, Clone, PartialEq)]
pub enum MediaCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    /// Seconds forward (negative: back).
    SeekBy(f64),
    /// Go to `seconds` of `track` (ignored for another track).
    SetPosition {
        track: u64,
        seconds: f64,
    },
    /// 1.0 is 100 %.
    SetVolume(f64),
    SetRate(f64),
    /// Play this file or URL.
    OpenUri(String),
    Raise,
    Quit,
}

/// The lowest and highest playback rate the controls may set.
pub const RATE_RANGE: (f64, f64) = (0.25, 4.0);

impl App {
    /// The state the media controls show now.
    #[must_use]
    pub fn media_state(&self) -> MediaState {
        let loaded = self.model.media_loaded();
        let status = if !loaded {
            Status::Stopped
        } else if self.model.flag(PropertyId::Pause) == Some(true) {
            Status::Paused
        } else {
            Status::Playing
        };
        let length = self
            .model
            .double(PropertyId::Duration)
            .filter(|d| loaded && *d > 0.0);
        let lists = self.library().lists();
        let shown_has_entries = lists
            .albums
            .get(lists.shown)
            .is_some_and(|album| !album.entries.is_empty());
        MediaState {
            status,
            track: self.media_track,
            title: if loaded {
                crate::view::title(&self.model, lists)
            } else {
                String::new()
            },
            url: loaded
                .then(|| self.model.text(PropertyId::Path))
                .flatten()
                .map(file_url),
            length,
            position: if loaded {
                self.model.double(PropertyId::TimePos).unwrap_or(0.0)
            } else {
                0.0
            },
            volume: self.volume_percent() / 100.0,
            rate: self.model.double(PropertyId::Speed).unwrap_or(1.0),
            can_next: loaded && self.library().neighbour(1).is_some(),
            can_previous: loaded && self.library().neighbour(-1).is_some(),
            can_seek: length.is_some(),
            can_play: loaded || shown_has_entries,
        }
    }

    /// Carries out what the media controls ask for.
    pub fn media_command(&mut self, engine: &Engine, command: MediaCommand) -> Option<UiRequest> {
        let loaded = self.model.media_loaded();
        let paused = self.model.flag(PropertyId::Pause) == Some(true);
        let at_end = self.model.flag(PropertyId::EofReached) == Some(true);
        match command {
            MediaCommand::Play if !loaded || paused || at_end => {
                return self.dispatch(engine, Action::PlayPause);
            }
            MediaCommand::Pause if loaded && !paused => drop(engine.set_pause(true)),
            MediaCommand::PlayPause => return self.dispatch(engine, Action::PlayPause),
            MediaCommand::Stop => return self.dispatch(engine, Action::CloseMedia),
            MediaCommand::Next => return self.dispatch(engine, Action::Next),
            MediaCommand::Previous => return self.dispatch(engine, Action::Previous),
            MediaCommand::SeekBy(seconds) if loaded => {
                drop(self.seek_to(engine, Seek::Relative(seconds)));
            }
            MediaCommand::SetPosition { track, seconds }
                if loaded && track == self.media_track && seconds >= 0.0 =>
            {
                drop(self.seek_to(engine, Seek::Absolute(seconds)));
            }
            MediaCommand::SetVolume(volume) => {
                let percent = (volume * 100.0).clamp(0.0, self.settings.volume_max.max(100.0));
                drop(engine.set_volume(crate::volume::mpv_volume(percent)));
            }
            MediaCommand::SetRate(rate) => {
                drop(engine.set_double("speed", rate.clamp(RATE_RANGE.0, RATE_RANGE.1)));
            }
            MediaCommand::OpenUri(uri) => {
                drop(self.open(engine, &[file_path(&uri)], false));
            }
            MediaCommand::Raise => return Some(UiRequest::Raise),
            MediaCommand::Quit => return Some(UiRequest::Exit),
            // Already playing or paused, nothing loaded, or another track.
            MediaCommand::Play
            | MediaCommand::Pause
            | MediaCommand::SeekBy(_)
            | MediaCommand::SetPosition { .. } => {}
        }
        None
    }
}

/// Bytes a `file://` URL keeps as they are.
fn plain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte)
}

/// A path as a `file://` URL; a URL stays as it is.
#[must_use]
pub fn file_url(path: &str) -> String {
    if path.contains("://") {
        return path.to_owned();
    }
    let path = path.replace('\\', "/");
    let mut url = String::from(if path.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for byte in path.bytes() {
        if plain(byte) {
            url.push(char::from(byte));
        } else {
            // Writing to a String does not fail.
            let _ = write!(url, "%{byte:02X}");
        }
    }
    url
}

/// A `file://` URL as a path; other URLs (streams) stay as they are.
#[must_use]
pub fn file_path(uri: &str) -> String {
    let Some(rest) = uri.strip_prefix("file://") else {
        return uri.to_owned();
    };
    // `file:///C:/...` on Windows, `file:///home/...` elsewhere.
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    let mut path = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |at: usize| {
            bytes
                .get(at)
                .and_then(|b| char::from(*b).to_digit(16))
                .and_then(|d| u8::try_from(d).ok())
        };
        if bytes[i] == b'%'
            && let (Some(high), Some(low)) = (hex(i + 1), hex(i + 2))
        {
            path.push(high * 16 + low);
            i += 3;
        } else {
            path.push(bytes[i]);
            i += 1;
        }
    }
    let path = String::from_utf8_lossy(&path).into_owned();
    // A drive letter after the root slash: `/C:/films` is `C:/films`.
    match path.as_bytes() {
        [b'/', drive, b':', ..] if drive.is_ascii_alphabetic() => path[1..].to_owned(),
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_urls_round_trip() {
        assert_eq!(
            file_url("~/My Films/фильм #1.mkv"),
            "file://~/My%20Films/%D1%84%D0%B8%D0%BB%D1%8C%D0%BC%20%231.mkv"
        );
        assert_eq!(file_url("C:\\Films\\a b.mkv"), "file:///C:/Films/a%20b.mkv");
        assert_eq!(
            file_url("https://example.org/a.m3u8"),
            "https://example.org/a.m3u8"
        );
        for path in ["~/My Films/фильм #1.mkv", "/tmp/100%.mkv"] {
            assert_eq!(file_path(&file_url(path)), path);
        }
        assert_eq!(file_path("file:///C:/Films/a%20b.mkv"), "C:/Films/a b.mkv");
        assert_eq!(file_path("file://localhost/tmp/a.mkv"), "/tmp/a.mkv");
        assert_eq!(file_path("https://example.org/a"), "https://example.org/a");
    }
}
