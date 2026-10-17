//! Streaming sites through the user's yt-dlp (M11). libmpv has no Lua, so
//! mpv's `ytdl_hook` cannot run; RVP does what it does:
//!
//! * an `http`/`https` address whose path has no media or playlist
//!   extension (or one written `ytdl://...`) is given to yt-dlp first:
//!   `yt-dlp -J --no-playlist -f FORMAT -- URL`, on a thread of its own;
//! * its JSON's `requested_formats` (video and audio apart) become an EDL
//!   address with one stream each, a single format its `url`; the HTTP
//!   headers it asks for, the title and the duration go with the file
//!   (`loadfile`'s per-file options);
//! * when yt-dlp is missing or does not know the site, the address plays
//!   as it is, as FFmpeg opens it.
//!
//! The album keeps the page's address, so the entry resolves again when it
//! plays next (the media addresses yt-dlp gives expire).

use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use rvp_engine::Engine;
use serde_json::Value;

use crate::app::{App, UiRequest};

/// What a page resolved to.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    /// The address libmpv opens (`edl://...` for video and audio apart).
    pub target: String,
    /// Per-file options (`http-header-fields`, `force-media-title`).
    pub options: Vec<(String, String)>,
    pub title: Option<String>,
    pub duration: Option<f64>,
}

/// A resolution under way.
#[derive(Debug)]
struct Pending {
    url: String,
    receiver: mpsc::Receiver<Result<Resolved, String>>,
    started: Instant,
}

/// The resolution under way, if any.
#[derive(Debug, Default)]
pub(crate) struct Resolver {
    pending: Option<Pending>,
}

/// True for addresses yt-dlp should see first.
#[must_use]
pub fn needs_resolving(url: &str) -> bool {
    if url.starts_with("ytdl://") {
        return true;
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    let path = lower
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .splitn(4, '/')
        .nth(3)
        .unwrap_or("");
    let file = Path::new(path);
    let direct = crate::sources::is_media(file)
        || file.extension().is_some_and(|extension| {
            [
                "m3u8", "m3u", "pls", "mpd", "jpg", "png", "srt", "ass", "vtt",
            ]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
        });
    !direct
}

/// An EDL segment: `%length%text`, so that any character may follow.
fn edl_part(text: &str) -> String {
    format!("%{}%{text}", text.len())
}

/// mpv's string lists split at commas: a header value keeps its own.
fn header_list(headers: &serde_json::Map<String, Value>) -> String {
    headers
        .iter()
        .filter_map(|(name, value)| Some(format!("{name}: {}", value.as_str()?)))
        .map(|header| header.replace(',', "\\,"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Reads yt-dlp's JSON for one video.
///
/// # Errors
/// The reason when the JSON names nothing to play.
pub fn parse(json: &Value) -> Result<Resolved, String> {
    let video = match json["_type"].as_str() {
        // A playlist (despite --no-playlist): its first entry.
        Some("playlist") => json["entries"]
            .as_array()
            .and_then(|entries| entries.first())
            .ok_or_else(|| "an empty playlist".to_owned())?,
        _ => json,
    };
    let title = video["title"].as_str().map(str::to_owned);
    let duration = video["duration"].as_f64();
    let mut headers = video["http_headers"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let target = match video["requested_formats"].as_array() {
        Some(formats) if formats.len() > 1 => {
            let mut parts = Vec::new();
            for format in formats {
                let url = format["url"]
                    .as_str()
                    .ok_or_else(|| "a format without an address".to_owned())?;
                if let Some(more) = format["http_headers"].as_object() {
                    for (name, value) in more {
                        headers.entry(name.clone()).or_insert_with(|| value.clone());
                    }
                }
                parts.push(format!("!no_clip;!no_chapters;{}", edl_part(url)));
            }
            format!("edl://{}", parts.join(";!new_stream;"))
        }
        Some(formats) if formats.len() == 1 => formats[0]["url"]
            .as_str()
            .ok_or_else(|| "a format without an address".to_owned())?
            .to_owned(),
        _ => video["url"]
            .as_str()
            .ok_or_else(|| "no address to play".to_owned())?
            .to_owned(),
    };
    let mut options = Vec::new();
    if !headers.is_empty() {
        options.push(("http-header-fields".to_owned(), header_list(&headers)));
    }
    if let Some(title) = &title {
        options.push(("force-media-title".to_owned(), title.clone()));
    }
    Ok(Resolved {
        target,
        options,
        title,
        duration,
    })
}

/// Runs yt-dlp (on the caller's thread: a worker's).
fn run(program: &str, format: &str, url: &str) -> Result<Resolved, String> {
    let url = url.strip_prefix("ytdl://").unwrap_or(url);
    let mut command = std::process::Command::new(program);
    command.args([
        "-J",
        "--no-playlist",
        "--no-warnings",
        "-f",
        format,
        "--",
        url,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: no console flashes up from the GUI.
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        let line = message
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("failed");
        return Err(line.trim().to_owned());
    }
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("yt-dlp's answer: {error}"))?;
    parse(&json)
}

impl App {
    /// Starts resolving `url` with yt-dlp when it needs it; true when the
    /// file loads later ([`App::resolver_tick`]).
    pub(crate) fn resolve_stream(&mut self, engine: &Engine, url: &str) -> bool {
        let open = &self.settings.open;
        if !open.ytdl || !needs_resolving(url) {
            return false;
        }
        let (program, format, page) = (
            open.ytdl_path.clone(),
            open.ytdl_format.clone(),
            url.to_owned(),
        );
        let (sender, receiver) = mpsc::channel();
        let started = std::thread::Builder::new()
            .name("rvp-ytdl".to_owned())
            .spawn(move || drop(sender.send(run(&program, &format, &page))));
        if started.is_err() {
            return false;
        }
        // What played before stops while the page resolves.
        drop(engine.stop());
        self.resolver.pending = Some(Pending {
            url: url.to_owned(),
            receiver,
            started: Instant::now(),
        });
        true
    }

    /// About once a second and after each drain: a finished resolution
    /// loads its file, or the address plays as it is.
    pub fn resolver_tick(&mut self, engine: &Engine) -> Vec<UiRequest> {
        let Some(pending) = &self.resolver.pending else {
            return Vec::new();
        };
        let result = match pending.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return Vec::new(),
            Err(mpsc::TryRecvError::Disconnected) => Err("yt-dlp stopped".to_owned()),
        };
        let Some(pending) = self.resolver.pending.take() else {
            return Vec::new();
        };
        let waited = pending.started.elapsed().as_secs_f64();
        let mut requests = Vec::new();
        match result {
            Ok(resolved) => {
                if let Some((album, index)) = self
                    .library()
                    .lists()
                    .playing
                    .zip(self.library().playing().map(|(index, _)| index))
                {
                    let untitled = self
                        .library()
                        .playing()
                        .is_some_and(|(_, entry)| entry.title.is_none());
                    if untitled && let Some(title) = &resolved.title {
                        self.library_mut().set_title(album, &[index], Some(title));
                    }
                    if let Some(duration) = resolved.duration {
                        self.library_mut().set_duration(&pending.url, duration);
                    }
                }
                if let Err(error) = engine.load_with_options(&resolved.target, &resolved.options) {
                    requests.push(UiRequest::Message(format!("Cannot play: {error}")));
                }
                requests.push(UiRequest::Osd(format!("yt-dlp: {waited:.1} s")));
            }
            Err(error) => {
                // Not a site yt-dlp knows, or no yt-dlp: as FFmpeg opens it.
                requests.push(UiRequest::Osd(format!("yt-dlp: {error}")));
                if let Err(error) = engine.load(&pending.url, false) {
                    requests.push(UiRequest::Message(format!("Cannot play: {error}")));
                }
            }
        }
        requests
    }

    /// A resolution is under way (the window shows it).
    #[must_use]
    pub fn resolving(&self) -> Option<&str> {
        self.resolver
            .pending
            .as_ref()
            .map(|pending| pending.url.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pages_go_to_yt_dlp_and_files_do_not() {
        assert!(needs_resolving("https://www.youtube.com/watch?v=abc"));
        assert!(needs_resolving("https://vimeo.com/12345"));
        assert!(needs_resolving("ytdl://anything"));
        assert!(!needs_resolving("https://example.com/films/a.mkv"));
        assert!(!needs_resolving(
            "https://example.com/live/index.m3u8?token=1"
        ));
        assert!(!needs_resolving("ftp://example.com/a"));
        assert!(!needs_resolving("/local/file"));
    }

    #[test]
    fn separate_video_and_audio_become_an_edl() {
        let answer = json!({
            "title": "A, title",
            "duration": 12.5,
            "http_headers": {"User-Agent": "UA, 1"},
            "requested_formats": [
                {"url": "https://v.example/video;1"},
                {"url": "https://a.example/audio", "http_headers": {"Referer": "https://x"}}
            ]
        });
        let resolved = parse(&answer).expect("parses");
        assert_eq!(
            resolved.target,
            "edl://!no_clip;!no_chapters;%25%https://v.example/video;1;!new_stream;!no_clip;!no_chapters;%23%https://a.example/audio"
        );
        assert_eq!(resolved.title.as_deref(), Some("A, title"));
        assert_eq!(resolved.duration, Some(12.5));
        assert!(resolved.options.contains(&(
            "http-header-fields".to_owned(),
            "Referer: https://x,User-Agent: UA\\, 1".to_owned()
        )));
        let single = parse(&json!({"url": "https://only.example/x.mp4"})).expect("single");
        assert_eq!(single.target, "https://only.example/x.mp4");
        assert!(parse(&json!({"title": "nothing"})).is_err());
        let playlist = parse(&json!({"_type": "playlist", "entries": [{"url": "u"}]}));
        assert_eq!(playlist.expect("first entry").target, "u");
    }
}
