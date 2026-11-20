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
//!
//! One resolution runs at a time, for the entry that is starting: another
//! entry, Close and RVP's end cancel it (`App::cancel_resolution`) and
//! end the process, and an answer counts only while its entry is still
//! the one starting. yt-dlp gets [`TIMEOUT`] to answer; the address then
//! plays as it is, as after an error.

use std::io::Read as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use rvp_engine::Engine;
use serde_json::Value;

use crate::app::{App, UiRequest};

/// How long yt-dlp may take to answer. Sites answer in seconds; the first
/// run of a packed yt-dlp also unpacks itself.
pub const TIMEOUT: Duration = Duration::from_mins(1);

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

/// What a resolution asks yt-dlp.
#[derive(Debug, Clone)]
pub(crate) struct Ask {
    pub program: String,
    pub format: String,
    pub url: String,
}

/// The yt-dlp process of a resolution, shared between its thread and the
/// window's, so that a cancel ends it.
#[derive(Debug, Default)]
pub(crate) struct Process {
    child: Mutex<Option<std::process::Child>>,
    cancelled: AtomicBool,
}

impl Process {
    fn child(&self) -> std::sync::MutexGuard<'_, Option<std::process::Child>> {
        self.child.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Ends the process (or keeps it from starting).
    pub fn cancel(&self) {
        let mut child = self.child();
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(child) = child.as_mut() {
            drop(child.kill());
        }
    }

    /// The resolution was cancelled.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

/// What runs a resolution, on a thread of its own: yt-dlp ([`run`]), or a
/// test's stand-in.
pub(crate) type Run = Arc<dyn Fn(&Ask, &Process) -> Result<Resolved, String> + Send + Sync>;

/// A resolution under way.
#[derive(Debug)]
struct Pending {
    url: String,
    /// The entry's per-file options (the command line's), added to the
    /// answer's.
    options: Vec<(String, String)>,
    receiver: mpsc::Receiver<Result<Resolved, String>>,
    process: Arc<Process>,
    started: Instant,
}

/// The resolution under way, if any.
pub(crate) struct Resolver {
    pending: Option<Pending>,
    run: Run,
}

impl Default for Resolver {
    fn default() -> Self {
        Self {
            pending: None,
            run: Arc::new(run),
        }
    }
}

impl std::fmt::Debug for Resolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resolver")
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
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

/// Runs yt-dlp (on the caller's thread: a worker's). Its error output is
/// read on a thread of its own and its answer here, so that neither pipe
/// fills and holds it up; a cancel ([`Process::cancel`]) ends it.
fn run(ask: &Ask, process: &Process) -> Result<Resolved, String> {
    use std::process::Stdio;
    let program = &ask.program;
    let url = ask.url.strip_prefix("ytdl://").unwrap_or(&ask.url);
    let mut command = std::process::Command::new(program);
    command
        .args([
            "-J",
            "--no-playlist",
            "--no-warnings",
            "-f",
            &ask.format,
            "--",
            url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: no console flashes up from the GUI.
        command.creation_flags(0x0800_0000);
    }
    if process.cancelled() {
        return Err("cancelled".to_owned());
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{program}: {error}"))?;
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    {
        // A cancel either came first, and the process ends here, or finds
        // it in its place.
        let mut slot = process.child();
        if process.cancelled() {
            drop(child.kill());
            drop(child.wait());
            return Err("cancelled".to_owned());
        }
        *slot = Some(child);
    }
    let errors = stderr.and_then(|mut pipe| {
        std::thread::Builder::new()
            .name("rvp-ytdl-err".to_owned())
            .spawn(move || {
                let mut text = Vec::new();
                drop(pipe.read_to_end(&mut text));
                text
            })
            .ok()
    });
    let mut answer = Vec::new();
    if let Some(mut pipe) = stdout {
        drop(pipe.read_to_end(&mut answer));
    }
    let errors = errors
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();
    // The answer is complete once its pipe closed: the wait is short.
    let child = process.child().take();
    let status = child
        .map(|mut child| child.wait())
        .transpose()
        .map_err(|error| format!("{program}: {error}"))?;
    if process.cancelled() {
        return Err("cancelled".to_owned());
    }
    if !status.is_some_and(|status| status.success()) {
        let message = String::from_utf8_lossy(&errors);
        let line = message
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("failed");
        return Err(line.trim().to_owned());
    }
    let json: Value =
        serde_json::from_slice(&answer).map_err(|error| format!("yt-dlp's answer: {error}"))?;
    parse(&json)
}

impl App {
    /// Starts resolving `url` with yt-dlp when it needs it; true when the
    /// file loads later ([`App::resolver_tick`]) with `options` added.
    pub(crate) fn resolve_stream(
        &mut self,
        engine: &Engine,
        url: &str,
        options: &[(String, String)],
    ) -> bool {
        let open = &self.settings.open;
        if !open.ytdl || !needs_resolving(url) {
            return false;
        }
        let ask = Ask {
            program: open.ytdl_path.clone(),
            format: open.ytdl_format.clone(),
            url: url.to_owned(),
        };
        let (sender, receiver) = mpsc::channel();
        let process = Arc::new(Process::default());
        let (run, worker) = (Arc::clone(&self.resolver.run), Arc::clone(&process));
        let started = std::thread::Builder::new()
            .name("rvp-ytdl".to_owned())
            .spawn(move || drop(sender.send(run(&ask, &worker))));
        if started.is_err() {
            return false;
        }
        // What played before stops while the page resolves.
        drop(engine.stop());
        self.resolver.pending = Some(Pending {
            url: url.to_owned(),
            options: options.to_vec(),
            receiver,
            process,
            started: Instant::now(),
        });
        true
    }

    /// Cancels the resolution under way, if any, and ends its yt-dlp:
    /// another entry starts, playback closes or RVP ends.
    pub(crate) fn cancel_resolution(&mut self) {
        if let Some(pending) = self.resolver.pending.take() {
            pending.process.cancel();
        }
    }

    /// About once a second: a finished resolution loads its file, or the
    /// address plays as it is (an error, or no answer within [`TIMEOUT`]).
    /// An answer counts only while its entry is the one starting.
    pub fn resolver_tick(&mut self, engine: &Engine) -> Vec<UiRequest> {
        let Some(pending) = &self.resolver.pending else {
            return Vec::new();
        };
        let result = match pending.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) if pending.started.elapsed() < TIMEOUT => {
                return Vec::new();
            }
            Err(mpsc::TryRecvError::Empty) => {
                pending.process.cancel();
                Err(format!("no answer in {} s", TIMEOUT.as_secs()))
            }
            Err(mpsc::TryRecvError::Disconnected) => Err("yt-dlp stopped".to_owned()),
        };
        let Some(pending) = self.resolver.pending.take() else {
            return Vec::new();
        };
        let still_starting = self.starting()
            && self
                .library()
                .playing()
                .is_some_and(|(_, entry)| entry.path == pending.url);
        if !still_starting {
            return Vec::new();
        }
        let waited = pending.started.elapsed().as_secs_f64();
        let mut requests = Vec::new();
        match result {
            Ok(mut resolved) => {
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
                // The entry's own options last: the command line's title or
                // headers win over the page's.
                resolved.options.extend(pending.options);
                if let Err(error) = engine.load_with_options(&resolved.target, &resolved.options) {
                    requests.push(UiRequest::Message(format!("Cannot play: {error}")));
                }
                requests.push(UiRequest::Osd(format!("yt-dlp: {waited:.1} s")));
            }
            Err(error) => {
                // Not a site yt-dlp knows, or no yt-dlp: as FFmpeg opens it.
                requests.push(UiRequest::Osd(format!("yt-dlp: {error}")));
                if let Err(error) = engine.load_with_options(&pending.url, &pending.options) {
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
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media"))
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    /// A page: yt-dlp's (here the stand-in's). Played as it is, it is a
    /// port on this machine that refuses at once.
    const PAGE: &str = "http://127.0.0.1:9/watch?v=1";

    /// A headless player whose yt-dlp is a stand-in that answers only when
    /// the test sends it an answer, then says whether it was cancelled
    /// meanwhile.
    struct Late {
        app: App,
        engine: Engine,
        dir: PathBuf,
        answers: mpsc::Sender<Result<Resolved, String>>,
        told: mpsc::Receiver<(String, bool)>,
    }

    impl Late {
        /// The page starts with `options`, as the command line gives them.
        fn new(name: &str, options: Vec<(String, String)>) -> Self {
            let dir = std::env::temp_dir().join(format!("rvp-ytdl-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let engine = Engine::new(&rvp_engine::EngineOptions::headless()).expect("engine");
            let settings = crate::Settings {
                reopen_last_file: false,
                remember_positions: false,
                ipc_server: String::new(),
                ..crate::Settings::default()
            };
            assert!(settings.open.ytdl);
            let mut app = App::new(settings, &dir).expect("app");
            let (answers, gate) = mpsc::channel::<Result<Resolved, String>>();
            let gate = Mutex::new(gate);
            let (tell, told) = mpsc::channel();
            app.resolver.run = Arc::new(move |ask: &Ask, process: &Process| {
                let answer = gate
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .recv()
                    .unwrap_or_else(|_| Err("no answer".to_owned()));
                drop(tell.send((ask.url.clone(), process.cancelled())));
                answer
            });
            app.play.next_options = options;
            app.open(
                &engine,
                &[PAGE.to_owned(), fixture("fixture-60s.mkv")],
                false,
            )
            .expect("open");
            assert_eq!(app.resolving(), Some(PAGE));
            // Taken by the page, for its stream.
            assert!(app.play.next_options.is_empty());
            Self {
                app,
                engine,
                dir,
                answers,
                told,
            }
        }

        /// The page resolves to the 3 s fixture, with a title; the
        /// stand-in says whether it was cancelled.
        fn answer(&self) -> bool {
            self.answers
                .send(Ok(Resolved {
                    target: fixture("fixture-3s.mkv"),
                    options: vec![("force-media-title".to_owned(), "Page title".to_owned())],
                    title: Some("Page title".to_owned()),
                    duration: Some(3.0),
                }))
                .expect("send");
            let (url, cancelled) = self
                .told
                .recv_timeout(Duration::from_secs(5))
                .expect("the stand-in answered");
            assert_eq!(url, PAGE);
            cancelled
        }

        /// Drains and ticks for `ms`, or until `done` holds.
        fn pump(&mut self, ms: u64, done: impl Fn(&App) -> bool) -> Vec<UiRequest> {
            let end = Instant::now() + Duration::from_millis(ms);
            let mut requests = Vec::new();
            while Instant::now() < end && !done(&self.app) {
                std::thread::sleep(Duration::from_millis(5));
                requests.extend(self.app.drain(&self.engine, Instant::now()).requests);
                requests.extend(self.app.resolver_tick(&self.engine));
            }
            requests
        }

        fn loaded(&self, name: &str) -> bool {
            self.app
                .model
                .loaded_path()
                .is_some_and(|path| path.ends_with(name))
        }

        fn titles(&self) -> Vec<Option<String>> {
            self.app.library().lists().albums[0]
                .entries
                .iter()
                .map(|entry| entry.title.clone())
                .collect()
        }
    }

    impl Drop for Late {
        fn drop(&mut self) {
            drop(self.engine.stop());
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// A page's answer that comes after another entry started, or after
    /// Close, plays nothing and names nothing; the resolution was
    /// cancelled.
    #[test]
    fn a_late_answer_replaces_nothing() {
        let mut h = Late::new("other", Vec::new());
        h.app.play_entry(&h.engine, 0, 1, false).expect("play");
        assert_eq!(h.app.resolving(), None);
        h.pump(10_000, |app| {
            app.model
                .loaded_path()
                .is_some_and(|path| path.ends_with("fixture-60s.mkv"))
        });
        assert!(h.loaded("fixture-60s.mkv"));
        assert!(h.answer(), "cancelled when the other entry started");
        h.pump(400, |_| false);
        assert!(
            h.loaded("fixture-60s.mkv"),
            "{:?}",
            h.app.model.loaded_path()
        );
        assert_eq!(h.titles(), [None, None]);

        // Close while the page resolves: nothing plays afterwards.
        h.app.play_entry(&h.engine, 0, 0, false).expect("play");
        assert_eq!(h.app.resolving(), Some(PAGE));
        drop(h.app.dispatch(&h.engine, crate::Action::CloseMedia));
        assert_eq!(h.app.resolving(), None);
        assert!(h.answer(), "cancelled by Close");
        h.pump(400, |_| false);
        assert!(
            !h.app.model.media_loaded(),
            "{:?}",
            h.app.model.loaded_path()
        );
        assert_eq!(h.titles(), [None, None]);

        // The entry left the album while its page resolved: its answer is
        // no one's.
        h.app.play_entry(&h.engine, 0, 0, false).expect("play");
        h.app
            .library_mut()
            .remove(0, &std::collections::BTreeSet::from([0]));
        assert!(!h.answer());
        h.pump(5000, |app| app.resolving().is_none());
        assert_eq!(h.app.resolving(), None);
        h.pump(300, |_| false);
        assert!(
            !h.app.model.media_loaded(),
            "{:?}",
            h.app.model.loaded_path()
        );
        assert_eq!(h.titles(), [None]);
    }

    /// The answer for the entry still starting plays with the entry's own
    /// options (the command line's), which win over the page's.
    #[test]
    fn an_answer_plays_with_the_entrys_options() {
        let mut h = Late::new(
            "options",
            vec![(
                "force-media-title".to_owned(),
                "From the command line".to_owned(),
            )],
        );
        assert!(!h.answer());
        h.pump(10_000, |app| {
            app.model
                .loaded_path()
                .is_some_and(|path| path.ends_with("fixture-3s.mkv"))
        });
        assert!(h.loaded("fixture-3s.mkv"));
        assert_eq!(
            h.engine.get_text("media-title").as_deref(),
            Some("From the command line")
        );
        assert_eq!(h.titles(), [Some("Page title".to_owned()), None]);
        drop(h.app.dispatch(&h.engine, crate::Action::CloseMedia));
    }

    /// No answer in time: the resolution is cancelled and the address
    /// plays as it is.
    #[test]
    fn a_resolution_without_an_answer_ends() {
        let mut h = Late::new("timeout", Vec::new());
        let pending = h.app.resolver.pending.as_mut().expect("pending");
        pending.started = Instant::now()
            .checked_sub(TIMEOUT + Duration::from_secs(1))
            .expect("an instant that long ago");
        let requests = h.app.resolver_tick(&h.engine);
        assert_eq!(
            requests,
            [UiRequest::Osd("yt-dlp: no answer in 60 s".to_owned())]
        );
        assert_eq!(h.app.resolving(), None);
        assert!(h.answer(), "cancelled at the timeout");
        drop(h.app.dispatch(&h.engine, crate::Action::CloseMedia));
    }

    /// A cancel ends yt-dlp's process at once, whatever it was doing.
    #[cfg(unix)]
    #[test]
    fn a_cancel_ends_the_process() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = std::env::temp_dir().join(format!("rvp-ytdl-kill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let program = dir.join("yt-dlp");
        std::fs::write(&program, "#!/bin/sh\nexec sleep 30\n").expect("write");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("mode");
        let ask = Ask {
            program: program.to_string_lossy().into_owned(),
            format: "best".to_owned(),
            url: PAGE.to_owned(),
        };
        // A program just written can be busy for a moment while another
        // test's process starts: once more then.
        for _ in 0..3 {
            let process = Arc::new(Process::default());
            let (ask, worker) = (ask.clone(), Arc::clone(&process));
            let started = Instant::now();
            let thread = std::thread::spawn(move || run(&ask, &worker));
            // Until the process runs (or failed to start).
            while process.child().is_none()
                && !thread.is_finished()
                && started.elapsed() < Duration::from_secs(5)
            {
                std::thread::sleep(Duration::from_millis(5));
            }
            process.cancel();
            let result = thread.join().expect("the thread");
            if result.as_ref().is_err_and(|error| error.contains("busy")) {
                continue;
            }
            assert_eq!(result, Err("cancelled".to_owned()));
            assert!(started.elapsed() < Duration::from_secs(10));
            break;
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

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
