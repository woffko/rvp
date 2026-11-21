//! What the desktop's media controls show and ask for (M8): MPRIS on Linux
//! (`rvp-desktop`), the System Media Transport Controls on Windows (the
//! shell). Both read [`MediaState`] and send [`MediaCommand`]s, which run
//! as RVP's own actions do (Stop is "Close Playback", as the skin's stop
//! button).
//!
//! Every program on the session bus reads what MPRIS shows, so a stream's
//! address there carries no user name or password
//! ([`without_credentials`]), and a title made from an address
//! ([`url_name`], `view::file_name`) has no query or fragment either: a
//! signed stream's token sits in its query.

use std::borrow::Cow;
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
    /// Play this file (a `file://` URL) or `http` address.
    OpenUri(String),
    Raise,
    Quit,
}

/// The lowest and highest playback rate the controls may set.
pub const RATE_RANGE: (f64, f64) = (0.25, 4.0);

impl App {
    /// Whether the screen and the system stay awake now: video plays and
    /// "Prevent screen saver during playback" is on (Linux's idle inhibit,
    /// Windows' execution state).
    #[must_use]
    pub fn keeps_screen_on(&self) -> bool {
        self.settings.prevent_screen_saver
            && self.model.media_loaded()
            && self.model.flag(PropertyId::Pause) != Some(true)
            && self.model.int(PropertyId::VideoWidth).is_some()
    }

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
                crate::view::published_title(&self.model, lists)
            } else {
                String::new()
            },
            url: loaded
                .then(|| self.model.text(PropertyId::Path))
                .flatten()
                .map(|path| without_credentials(&file_url(path)).into_owned()),
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
                return match opened_uri(&uri) {
                    Some(file) => self.open_outside(engine, &[file], false),
                    None => Some(UiRequest::Message(
                        "Not opened: the media controls take a file or an http address".to_owned(),
                    )),
                };
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

/// What the media controls' `OpenUri` plays: an `http` or `https` address,
/// or the `file://` URL of a file that exists, as its path. These are the
/// schemes RVP tells the desktop it takes. Any program on the session bus
/// may call it, so a folder (which would replace the shown album with all
/// it holds) and every other address are not opened.
fn opened_uri(uri: &str) -> Option<String> {
    let (scheme, _) = uri.split_once("://")?;
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        return Some(uri.to_owned());
    }
    if !scheme.eq_ignore_ascii_case("file") {
        return None;
    }
    let path = crate::playlist::admitted(uri)?;
    std::path::Path::new(&path).is_file().then_some(path)
}

/// Bytes a `file://` URL keeps as they are.
fn plain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte)
}

/// A path as a `file://` URL; a URL stays as it is. A backslash is
/// Windows' separator only: on Linux it is a character of a name (the
/// Linux audit's L14).
#[must_use]
pub fn file_url(path: &str) -> String {
    if path.contains("://") {
        return path.to_owned();
    }
    let path = if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_owned()
    };
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

/// Whether `c` ends an address's authority: its path, query or fragment
/// begins, or the text around the address goes on (white space, quotes,
/// brackets no address holds unescaped).
fn ends_authority(c: char) -> bool {
    matches!(c, '/' | '?' | '#' | '"' | '<' | '>' | '\\') || c.is_whitespace()
}

/// `text` with every address's user name and password left out: after
/// each `://`, what comes before the last `@` of the authority
/// (`https://user:password@host/a` is `https://host/a`). The rest, the
/// query too, stays. For what RVP publishes (MPRIS) and logs.
#[must_use]
pub fn without_credentials(text: &str) -> Cow<'_, str> {
    let mut out = String::new();
    let mut copied = 0;
    for (at, _) in text.match_indices("://") {
        let start = at + 3;
        // An authority ends before the next `://`'s slashes, so the parts
        // left out never overlap.
        if start < copied {
            continue;
        }
        let authority = &text[start..];
        let end = authority.find(ends_authority).unwrap_or(authority.len());
        if let Some(sign) = authority[..end].rfind('@') {
            out.push_str(&text[copied..start]);
            copied = start + sign + 1;
        }
    }
    if copied == 0 {
        return Cow::Borrowed(text);
    }
    out.push_str(&text[copied..]);
    Cow::Owned(out)
}

/// What names an address in a title: the last part of its path, without
/// the query and the fragment, else its host without credentials
/// (`https://user:password@example.org/?v=1` is `example.org`).
#[must_use]
pub fn url_name(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let rest = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    let rest = rest.trim_end_matches('/');
    match rest.split_once('/') {
        Some((_, path)) => path.rsplit('/').next().unwrap_or(path),
        None => rest.rsplit_once('@').map_or(rest, |(_, host)| host),
    }
}

/// What names an address in RVP's own lists: the last part of its path
/// with its query (the entries of one site differ only there), else its
/// host; never the credentials (`https://u:p@host/watch?v=1` is
/// `watch?v=1`). What goes outside RVP takes [`url_name`].
#[must_use]
pub fn url_label(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    match path.trim_end_matches('/').rsplit_once('/') {
        Some((before, _)) => rest[before.len() + 1..].trim_end_matches('/'),
        None => url_name(url),
    }
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
    // A drive letter after the root slash: `/C:/films` is `C:/films` on
    // Windows (on Linux it is a folder of that name).
    match path.as_bytes() {
        [b'/', drive, b':', ..] if cfg!(windows) && drive.is_ascii_alphabetic() => {
            path[1..].to_owned()
        }
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_urls_round_trip() {
        assert_eq!(
            file_url("/home/user/My Films/фильм #1.mkv"),
            "file:///home/user/My%20Films/%D1%84%D0%B8%D0%BB%D1%8C%D0%BC%20%231.mkv"
        );
        if cfg!(windows) {
            assert_eq!(file_url("C:\\Films\\a b.mkv"), "file:///C:/Films/a%20b.mkv");
            assert_eq!(file_path("file:///C:/Films/a%20b.mkv"), "C:/Films/a b.mkv");
        } else {
            // A backslash in a Linux name, and a folder named `C:`.
            assert_eq!(file_url("/films/a\\b.mkv"), "file:///films/a%5Cb.mkv");
            assert_eq!(file_path("file:///C:/Films/a%20b.mkv"), "/C:/Films/a b.mkv");
        }
        assert_eq!(
            file_url("https://example.org/a.m3u8"),
            "https://example.org/a.m3u8"
        );
        for path in ["/home/user/My Films/фильм #1.mkv", "/tmp/100%.mkv"] {
            assert_eq!(file_path(&file_url(path)), path);
        }
        assert_eq!(file_path("file://localhost/tmp/a.mkv"), "/tmp/a.mkv");
        assert_eq!(file_path("https://example.org/a"), "https://example.org/a");
    }

    #[test]
    fn addresses_lose_their_credentials() {
        for (text, clean) in [
            (
                "https://user:pass@example.org/a.mkv?sig=1#t=2",
                "https://example.org/a.mkv?sig=1#t=2",
            ),
            (
                "mpv [cplayer] Playing: rtsp://cam@10.0.0.2:554/live.",
                "mpv [cplayer] Playing: rtsp://10.0.0.2:554/live.",
            ),
            ("https://user:pass@example.org", "https://example.org"),
            // The last `@` of the authority; one in the path stays.
            (
                "ftp://a@b:c@example.org/x@y.mkv",
                "ftp://example.org/x@y.mkv",
            ),
            // Every address of a line, nested ones too.
            (
                "edl://%30%https://u:p@a.example/v;%30%https://u:p@b.example/a",
                "edl://%30%https://a.example/v;%30%https://b.example/a",
            ),
            (
                "\"url\": \"https://u:p@example.org\", next https://x@y/",
                "\"url\": \"https://example.org\", next https://y/",
            ),
            (
                "https://example.org/?to=https://u:p@inner.example/a",
                "https://example.org/?to=https://inner.example/a",
            ),
        ] {
            assert_eq!(without_credentials(text), clean, "{text}");
        }
        // Nothing to leave out: the text itself.
        for text in [
            "file:///home/u/a@b.mkv",
            "mail user@example.org",
            "https://example.org/a@b",
            "",
        ] {
            assert!(
                matches!(without_credentials(text), Cow::Borrowed(same) if same == text),
                "{text}"
            );
        }
    }

    #[test]
    fn addresses_are_named_without_query_or_credentials() {
        for (url, name) in [
            ("https://example.org/films/a.mkv?token=secret#t=3", "a.mkv"),
            ("https://example.org/dir/", "dir"),
            ("https://user:secret@example.org/?v=1", "example.org"),
            ("https://user:secret@example.org:8443", "example.org:8443"),
            ("https://example.org/watch?v=abc", "watch"),
            ("dvd://longest//dev/sr0", "sr0"),
            ("rvpdisc://3", "3"),
        ] {
            assert_eq!(url_name(url), name, "{url}");
        }
    }
}
