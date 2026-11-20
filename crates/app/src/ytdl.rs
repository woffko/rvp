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
//! * the answer is the page's word, not RVP's: each `url` must be a
//!   network address ([`crate::playlist::network_address`]; mpv's own hook
//!   checks its answers the same way), or nothing of the answer plays;
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

/// The headers as mpv's `http-header-fields` takes them. Its string lists
/// split at commas, and a comma after a backslash is none: a header keeps
/// its own commas that way, and one that ends in a backslash gets a space
/// after it (HTTP drops it), or it would take the next header into its
/// value. A header with a control character is left out: a line break in
/// it would start another header, or the request's body.
fn header_list(headers: &serde_json::Map<String, Value>) -> String {
    headers
        .iter()
        .filter_map(|(name, value)| Some(format!("{name}: {}", value.as_str()?)))
        .filter(|header| !header.chars().any(char::is_control))
        .map(|header| {
            let mut header = header.replace(',', "\\,");
            if header.ends_with('\\') {
                header.push(' ');
            }
            header
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The address of one of the answer's formats (or of the answer itself):
/// a network address, as the module comment says.
fn stream_url<'a>(format: &'a Value, missing: &str) -> Result<&'a str, String> {
    let url = format["url"].as_str().ok_or_else(|| missing.to_owned())?;
    if crate::playlist::network_address(url) {
        Ok(url)
    } else {
        Err("an address that is no network stream".to_owned())
    }
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
                let url = stream_url(format, "a format without an address")?;
                if let Some(more) = format["http_headers"].as_object() {
                    for (name, value) in more {
                        headers.entry(name.clone()).or_insert_with(|| value.clone());
                    }
                }
                parts.push(format!("!no_clip;!no_chapters;{}", edl_part(url)));
            }
            format!("edl://{}", parts.join(";!new_stream;"))
        }
        Some(formats) if formats.len() == 1 => {
            stream_url(&formats[0], "a format without an address")?.to_owned()
        }
        _ => stream_url(video, "no address to play")?.to_owned(),
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
        let playlist = parse(&json!({
            "_type": "playlist",
            "entries": [{"url": "https://first.example/v"}]
        }));
        assert_eq!(
            playlist.expect("first entry").target,
            "https://first.example/v"
        );
    }

    #[test]
    fn an_answer_names_network_addresses_only() {
        // What a page could make yt-dlp answer: a capture device or a
        // filter graph, a local file, a descriptor, a disc, standard
        // input, yt-dlp again.
        for url in [
            "av://lavfi:testsrc",
            "AV://v4l2:/dev/video0",
            "file:///etc/hostname",
            "/etc/hostname",
            "fdclose://9",
            "dvd://1",
            "edl://a.mkv",
            "data://text/plain,x",
            "ytdl://example.org/again",
            "-",
            "",
        ] {
            // Alone, as the one format, as one of two, and in a playlist.
            for answer in [
                json!({"url": url}),
                json!({"requested_formats": [{"url": url}]}),
                json!({"requested_formats": [
                    {"url": "https://v.example/video"},
                    {"url": url}
                ]}),
                json!({"requested_formats": [
                    {"url": url},
                    {"url": "https://a.example/audio"}
                ]}),
                json!({"_type": "playlist", "entries": [{"url": url}]}),
            ] {
                assert_eq!(
                    parse(&answer),
                    Err("an address that is no network stream".to_owned()),
                    "{answer}"
                );
            }
        }
        // The streams sites give.
        for url in [
            "https://cdn.example/v.m3u8",
            "http://cdn.example/v.mp4",
            "rtmp://live.example/app/stream",
            "rtsp://camera.example/1",
            "mms://radio.example/live",
        ] {
            assert_eq!(parse(&json!({"url": url})).expect("plays").target, url);
        }
    }

    /// mpv's string lists as its `get_nextsep` splits them: a comma after
    /// a backslash is part of the item, and the backslash goes.
    fn mpv_list(text: &str) -> Vec<String> {
        let mut items = vec![String::new()];
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            let item = items.last_mut().expect("an item");
            match c {
                '\\' if chars.peek() == Some(&',') => {
                    item.push(',');
                    chars.next();
                }
                ',' => items.push(String::new()),
                other => item.push(other),
            }
        }
        items
    }

    #[test]
    fn headers_stay_apart_in_mpvs_list() {
        let headers = json!({
            "A-Comma": "one, two",
            "B-Backslash": "ends in \\",
            "C-Both": "x\\,y\\",
            "D-Break": "v\r\nInjected: 1",
            "E-Tab\u{1}": "v",
            "F-Last": "plain",
            "G-Number": 5
        });
        let list = header_list(headers.as_object().expect("an object"));
        assert_eq!(
            mpv_list(&list),
            [
                "A-Comma: one, two",
                // The space keeps the backslash from the comma after it.
                "B-Backslash: ends in \\ ",
                "C-Both: x\\,y\\ ",
                "F-Last: plain"
            ]
        );
        assert!(!list.chars().any(char::is_control), "{list:?}");
        // Without the space, the header after it would be part of it.
        assert_eq!(mpv_list("B: a\\,C: b"), ["B: a,C: b"]);
    }
}
