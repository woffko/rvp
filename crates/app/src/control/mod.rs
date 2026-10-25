//! RVP's control API (M15, D15): a small HTTP/1.1 JSON service through
//! which programs on this computer control RVP, such as the Unified Remote
//! remote in `remote/unified-remote/RVP`.
//!
//! * Off by default; the Remote page turns it on (`remote.enabled`) and
//!   chooses its port (`remote.port`). It listens on 127.0.0.1 only: other
//!   devices reach RVP through a program on this computer, such as Unified
//!   Remote's server.
//! * Every request carries the installation's token
//!   (`Authorization: Bearer TOKEN`, [`token`]). Requests with a browser's
//!   `Origin` header, or whose `Host` is not a loopback name, are refused,
//!   so that web pages cannot reach RVP. Headers, bodies, the time a
//!   request may take, connections and long polls are limited
//!   ([`LIMITS`]).
//! * The server's threads never call mpv and never touch [`App`]: commands
//!   go to the window's thread through the [`Dispatch`] the shell gives
//!   (as MPRIS's and Windows' media controls' commands do) and wait there
//!   for an answer; the status is a snapshot the window's thread publishes
//!   ([`Server::publish`]) as the chrome updates.
//!
//! | Request | Body or query | Answer |
//! |---|---|---|
//! | `GET /api/status` | `?since=V&wait=S`: waits up to S seconds (at most [`Limits::long_poll`]) for a version other than V | [`Status`] |
//! | `GET /api/actions` | | every action of `ACTIONS`: `name`, `label` |
//! | `POST /api/action/NAME` | | runs the action as keys and menus do |
//! | `POST /api/seek` | `{"to": s}`, `{"by": s}` or `{"fraction": f}` | |
//! | `POST /api/volume` | `{"percent": p}`, `{"mute": b}` or both | |
//! | `POST /api/open` | `{"path": "..."}`: a file, folder or URL | |
//! | `GET /api/albums` | | the albums: `index`, `name`, `entries`, `playing`, `current` |
//! | `GET /api/albums/A` | | album A with its `entries`: `index`, `title`, `duration` |
//! | `POST /api/albums/A/play/E` | | plays entry E of album A |
//!
//! Commands answer `{}` (or the data asked for) once the window's thread
//! carried them out; errors answer `{"error": "..."}` with their status.
//! Fields without a value are left out rather than `null` (Unified
//! Remote's JSON reader documents no `null`).

mod http;
pub mod token;

use std::io::Write as _;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rvp_engine::{Engine, PropertyId, Seek};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::action::{ACTIONS, Action};
use crate::app::{App, UiRequest};

use http::{ReadError, Request};

/// The port the Remote page starts with.
pub const DEFAULT_PORT: u16 = 48_931;

/// What a request may take.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// The request line and headers.
    pub header_bytes: usize,
    pub body_bytes: usize,
    /// Reading a whole request.
    pub request: Duration,
    /// Writing an answer.
    pub write: Duration,
    /// Connections served at once; more are answered 503.
    pub connections: usize,
    /// The longest wait of `GET /api/status?wait=`.
    pub long_poll: Duration,
    /// The longest wait for the window's thread to carry out a command.
    pub command: Duration,
}

/// The limits RVP serves with.
pub const LIMITS: Limits = Limits {
    header_bytes: 8 * 1024,
    body_bytes: 64 * 1024,
    request: Duration::from_secs(5),
    write: Duration::from_secs(5),
    connections: 8,
    long_poll: Duration::from_secs(30),
    command: Duration::from_secs(5),
};

/// The longest path `POST /api/open` takes.
const MAX_PATH: usize = 4096;

/// The lowest port the Remote page offers (below it ports need rights).
pub const MIN_PORT: u16 = 1024;

/// The Remote page (`remote` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RemoteSettings {
    /// The control API listens (off by default).
    pub enabled: bool,
    /// Its port on 127.0.0.1.
    pub port: u16,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
        }
    }
}

impl RemoteSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if self.port < MIN_PORT {
            notes.push(format!(
                "remote.port {} is below {MIN_PORT}: {DEFAULT_PORT}",
                self.port
            ));
            self.port = DEFAULT_PORT;
        }
    }
}

/// What a client asks the window's thread for.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// An action of `ACTIONS`, as keys and menus run it.
    Action(Action),
    Seek(SeekTo),
    Volume {
        /// PotPlayer's percent.
        percent: Option<f64>,
        mute: Option<bool>,
    },
    /// A file, folder or URL to open and play.
    Open(String),
    Albums,
    Album(usize),
    Play {
        album: usize,
        entry: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekTo {
    /// Seconds from the start.
    To(f64),
    /// Seconds forward (negative: back).
    By(f64),
    /// 0.0 to 1.0 of the file.
    Fraction(f64),
}

/// The window thread's answer: data, or an HTTP status and a message.
pub type Answer = Result<Value, (u16, String)>;

/// Where the answer goes.
pub type Reply = SyncSender<Answer>;

/// Hands a command to the window's thread; the shell's queues it there
/// (`CxxQtThread::queue`). The answer goes to the reply; none within
/// [`Limits::command`] answers 503.
pub type Dispatch = Arc<dyn Fn(Command, Reply) + Send + Sync>;

/// One audio or subtitle track.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Track {
    pub id: i64,
    /// The title, language or codec, as the menus name it.
    pub title: String,
    pub language: Option<String>,
    pub selected: bool,
}

/// What `GET /api/status` answers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    /// Grows with each change of [`State`] (not with the position).
    pub version: u64,
    /// Seconds.
    pub position: f64,
    #[serde(flatten)]
    pub state: State,
}

/// The status apart from the position.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
// What a remote shows: independent flags of the player.
#[allow(clippy::struct_excessive_bools)]
pub struct State {
    /// "playing", "paused" or "stopped".
    pub state: String,
    /// The window's title for the file.
    pub title: String,
    /// The file's path or URL.
    pub file: Option<String>,
    /// The album that plays, its name and the entry.
    pub album: Option<usize>,
    pub album_name: Option<String>,
    pub entry: Option<usize>,
    /// The playing album's number of entries.
    pub entries: Option<usize>,
    /// Seconds.
    pub duration: Option<f64>,
    /// PotPlayer's percent, and the highest the settings allow.
    pub volume: f64,
    pub volume_max: f64,
    pub mute: bool,
    pub speed: f64,
    pub fullscreen: bool,
    pub audio_tracks: Vec<Track>,
    pub subtitle_tracks: Vec<Track>,
    pub normalizer: bool,
    pub equalizer: bool,
}

struct Published {
    version: u64,
    position: f64,
    state: State,
}

struct Shared {
    published: Mutex<Published>,
    changed: Condvar,
    stop: AtomicBool,
    connections: AtomicUsize,
    token: String,
    dispatch: Dispatch,
    limits: Limits,
    /// `GET /api/actions`'s answer, made once.
    actions: String,
}

/// The running service; dropping it stops it.
pub struct Server {
    shared: Arc<Shared>,
    address: SocketAddr,
    accept: Option<JoinHandle<()>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Server {
    /// Listens on 127.0.0.1:`port` (0: a free port, for tests).
    ///
    /// # Errors
    /// Fails when the port is taken or the thread does not start.
    pub fn start(port: u16, token: String, dispatch: Dispatch) -> std::io::Result<Self> {
        Self::start_with(port, token, dispatch, LIMITS)
    }

    /// [`Server::start`] with other limits (tests).
    ///
    /// # Errors
    /// As [`Server::start`].
    pub fn start_with(
        port: u16,
        token: String,
        dispatch: Dispatch,
        limits: Limits,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        let address = listener.local_addr()?;
        let actions: Vec<Value> = ACTIONS
            .iter()
            .map(|info| json!({ "name": info.name, "label": info.label }))
            .collect();
        let shared = Arc::new(Shared {
            published: Mutex::new(Published {
                version: 0,
                position: 0.0,
                state: State::default(),
            }),
            changed: Condvar::new(),
            stop: AtomicBool::new(false),
            connections: AtomicUsize::new(0),
            token,
            dispatch,
            limits,
            actions: Value::from(actions).to_string(),
        });
        let accept = {
            let shared = Arc::clone(&shared);
            std::thread::Builder::new()
                .name("rvp-control".to_owned())
                .spawn(move || accept_loop(&listener, &shared))?
        };
        Ok(Self {
            shared,
            address,
            accept: Some(accept),
        })
    }

    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// The window's thread shows its state: a change other than the
    /// position raises the version and wakes the long polls. It holds the
    /// lock only to copy.
    pub fn publish(&self, position: f64, state: &State) {
        let mut published = lock(&self.shared.published);
        published.position = position;
        if published.state != *state {
            published.state = state.clone();
            published.version += 1;
            drop(published);
            self.shared.changed.notify_all();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.changed.notify_all();
        // Wakes the blocking accept.
        drop(TcpStream::connect_timeout(
            &self.address,
            Duration::from_millis(200),
        ));
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}

fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>) {
    for stream in listener.incoming() {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        if shared.connections.fetch_add(1, Ordering::AcqRel) >= shared.limits.connections {
            shared.connections.fetch_sub(1, Ordering::AcqRel);
            let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
            let _ = http::respond(&mut stream, 503, &error("too many connections"));
            // Briefly: this thread accepts the others.
            linger(&mut stream, Duration::from_millis(50));
            continue;
        }
        let worker = Arc::clone(shared);
        let spawned = std::thread::Builder::new()
            .name("rvp-control-client".to_owned())
            .spawn(move || {
                serve(&mut stream, &worker);
                worker.connections.fetch_sub(1, Ordering::AcqRel);
            });
        if spawned.is_err() {
            shared.connections.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

fn serve(stream: &mut TcpStream, shared: &Shared) {
    let _ = stream.set_write_timeout(Some(shared.limits.write));
    let (status, body) = match http::read_request(stream, &shared.limits) {
        Ok(request) => route(shared, &request),
        Err(failure) => match failure.status() {
            Some(status) => (status, error(read_error_text(failure))),
            None => return,
        },
    };
    let _ = http::respond(stream, status, &body);
    let _ = stream.flush();
    if status >= 400 {
        linger(stream, Duration::from_millis(200));
    }
}

/// After an early answer, reads what the client still sends for a moment
/// before closing: closing with unread data resets the connection, and the
/// client could lose the answer.
fn linger(stream: &mut TcpStream, wait: Duration) {
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(wait));
    let mut rest = [0u8; 4096];
    let mut drained = 0;
    while drained < 256 * 1024 {
        match std::io::Read::read(stream, &mut rest) {
            Ok(0) | Err(_) => break,
            Ok(read) => drained += read,
        }
    }
}

fn read_error_text(failure: ReadError) -> &'static str {
    match failure {
        ReadError::Closed | ReadError::Malformed => "malformed request",
        ReadError::Timeout => "the request took too long",
        ReadError::BodyTooLarge => "the body is too large",
        ReadError::HeadersTooLarge => "the headers are too large",
        ReadError::Unsupported => "chunked bodies are not supported",
    }
}

fn error(message: &str) -> String {
    json!({ "error": message }).to_string()
}

/// A loopback name in a `Host` header (with or without its port).
fn loopback_host(host: &str) -> bool {
    let name = match host.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    name.eq_ignore_ascii_case("localhost") || name == "127.0.0.1"
}

fn route(shared: &Shared, request: &Request) -> (u16, String) {
    if request.header("origin").is_some() {
        return (403, error("requests from web pages are refused"));
    }
    if request
        .header("host")
        .is_some_and(|host| !loopback_host(host))
    {
        return (403, error("this service answers only on 127.0.0.1"));
    }
    let presented = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim);
    if !presented.is_some_and(|presented| token::matches(presented, &shared.token)) {
        return (401, error("a valid token is needed"));
    }
    let segments: Vec<&str> = request
        .path
        .trim_end_matches('/')
        .split('/')
        .skip(1)
        .collect();
    let get = request.method == "GET";
    let post = request.method == "POST";
    match segments.as_slice() {
        ["api", "status"] if get => status(shared, request),
        ["api", "actions"] if get => (200, shared.actions.clone()),
        ["api", "action", name] if post => match Action::from_name(name) {
            Some(action) => ask(shared, Command::Action(action)),
            None => (404, error("no such action")),
        },
        ["api", "seek"] if post => match seek_body(&request.body) {
            Ok(to) => ask(shared, Command::Seek(to)),
            Err(message) => (400, error(message)),
        },
        ["api", "volume"] if post => match volume_body(&request.body) {
            Ok((percent, mute)) => ask(shared, Command::Volume { percent, mute }),
            Err(message) => (400, error(message)),
        },
        ["api", "open"] if post => match open_body(&request.body) {
            Ok(path) => ask(shared, Command::Open(path)),
            Err(message) => (400, error(message)),
        },
        ["api", "albums"] if get => ask(shared, Command::Albums),
        ["api", "albums", album] if get => match album.parse() {
            Ok(album) => ask(shared, Command::Album(album)),
            Err(_) => (404, error("no such album")),
        },
        ["api", "albums", album, "play", entry] if post => match (album.parse(), entry.parse()) {
            (Ok(album), Ok(entry)) => ask(shared, Command::Play { album, entry }),
            _ => (404, error("no such entry")),
        },
        [
            "api",
            "status" | "actions" | "albums" | "action" | "seek" | "volume" | "open",
            ..,
        ] => (405, error("method not allowed")),
        _ => (404, error("not found")),
    }
}

/// The status now, or after the next change (a long poll).
fn status(shared: &Shared, request: &Request) -> (u16, String) {
    let since = request.query("since").and_then(|v| v.parse::<u64>().ok());
    let wait = request
        .query("wait")
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0)
        .map_or(Duration::ZERO, |v| {
            Duration::from_secs_f64(v).min(shared.limits.long_poll)
        });
    let deadline = Instant::now() + wait;
    let mut published = lock(&shared.published);
    while since == Some(published.version) && !shared.stop.load(Ordering::Acquire) {
        let Some(left) = deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
        else {
            break;
        };
        published = shared
            .changed
            .wait_timeout(published, left)
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
    let status = Status {
        version: published.version,
        position: published.position,
        state: published.state.clone(),
    };
    drop(published);
    match serde_json::to_value(&status) {
        Ok(value) => (200, without_nulls(value).to_string()),
        Err(failure) => (503, error(&failure.to_string())),
    }
}

/// The value with its objects' `null` members left out.
fn without_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(_, member)| !member.is_null())
                .map(|(key, member)| (key, without_nulls(member)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(without_nulls).collect()),
        other => other,
    }
}

/// Hands a command to the window's thread and waits for its answer.
fn ask(shared: &Shared, command: Command) -> (u16, String) {
    let (reply, answer) = mpsc::sync_channel(1);
    (shared.dispatch)(command, reply);
    match answer.recv_timeout(shared.limits.command) {
        Ok(Ok(value)) => (200, without_nulls(value).to_string()),
        Ok(Err((status, message))) => (status, error(&message)),
        Err(_) => (503, error("RVP did not answer")),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeekBody {
    to: Option<f64>,
    by: Option<f64>,
    fraction: Option<f64>,
}

fn seek_body(body: &[u8]) -> Result<SeekTo, &'static str> {
    let body: SeekBody = serde_json::from_slice(body)
        .map_err(|_| "expected {\"to\"|\"by\"|\"fraction\": number}")?;
    let to = match (body.to, body.by, body.fraction) {
        (Some(to), None, None) => SeekTo::To(to),
        (None, Some(by), None) => SeekTo::By(by),
        (None, None, Some(fraction)) => SeekTo::Fraction(fraction),
        _ => return Err("give one of \"to\", \"by\" and \"fraction\""),
    };
    let (SeekTo::To(value) | SeekTo::By(value) | SeekTo::Fraction(value)) = to;
    if !value.is_finite() {
        return Err("the number must be finite");
    }
    Ok(to)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VolumeBody {
    percent: Option<f64>,
    mute: Option<bool>,
}

fn volume_body(body: &[u8]) -> Result<(Option<f64>, Option<bool>), &'static str> {
    let body: VolumeBody = serde_json::from_slice(body)
        .map_err(|_| "expected {\"percent\": number, \"mute\": bool}")?;
    if body.percent.is_none() && body.mute.is_none() {
        return Err("give \"percent\", \"mute\" or both");
    }
    if body.percent.is_some_and(|percent| !percent.is_finite()) {
        return Err("the percent must be finite");
    }
    Ok((body.percent, body.mute))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenBody {
    path: String,
}

fn open_body(body: &[u8]) -> Result<String, &'static str> {
    let body: OpenBody = serde_json::from_slice(body).map_err(|_| "expected {\"path\": text}")?;
    let path = body.path.trim();
    if path.is_empty() || path.len() > MAX_PATH || path.chars().any(char::is_control) {
        return Err("the path must be one line of up to 4096 bytes");
    }
    Ok(path.to_owned())
}

impl App {
    /// The control API's status apart from the position (see
    /// [`Server::publish`]); `fullscreen` is the window's, `audio` and
    /// `subtitles` the file's tracks.
    #[must_use]
    pub fn control_state(&self, fullscreen: bool, audio: &[Track], subtitles: &[Track]) -> State {
        let model = &self.model;
        let loaded = model.media_loaded();
        let lists = self.library().lists();
        let playing = lists
            .playing
            .and_then(|index| lists.albums.get(index).map(|a| (index, a)));
        State {
            state: if !loaded {
                "stopped"
            } else if model.flag(PropertyId::Pause) == Some(true) {
                "paused"
            } else {
                "playing"
            }
            .to_owned(),
            title: if loaded {
                crate::view::title(model, lists)
            } else {
                String::new()
            },
            file: loaded
                .then(|| model.text(PropertyId::Path))
                .flatten()
                .map(str::to_owned),
            album: playing.map(|(index, _)| index),
            album_name: playing.map(|(_, album)| album.name.clone()),
            entry: playing.and_then(|(_, album)| album.current),
            entries: playing.map(|(_, album)| album.entries.len()),
            duration: model
                .double(PropertyId::Duration)
                .filter(|d| loaded && d.is_finite() && *d > 0.0),
            volume: (self.volume_percent() * 10.0).round() / 10.0,
            volume_max: self.settings.volume_max,
            mute: model.flag(PropertyId::Mute) == Some(true),
            speed: model.double(PropertyId::Speed).unwrap_or(1.0),
            fullscreen,
            audio_tracks: audio.to_vec(),
            subtitle_tracks: subtitles.to_vec(),
            normalizer: self.settings.normalizer.enabled,
            equalizer: self.settings.equalizer.enabled,
        }
    }

    /// The position the control API shows (seconds).
    #[must_use]
    pub fn control_position(&self) -> f64 {
        self.model
            .double(PropertyId::TimePos)
            .filter(|t| self.model.media_loaded() && t.is_finite())
            .map_or(0.0, |t| (t * 10.0).round() / 10.0)
    }

    /// Carries out a command other than an action (the shell runs those
    /// as keys do): the answer, and what the window should do.
    pub fn control_command(
        &mut self,
        engine: &Engine,
        command: Command,
    ) -> (Answer, Option<UiRequest>) {
        let done = || Ok(json!({}));
        match command {
            Command::Action(action) => (done(), self.dispatch(engine, action)),
            Command::Seek(to) => {
                if !self.model.media_loaded() {
                    return (Err((409, "nothing is open".to_owned())), None);
                }
                let target = match to {
                    SeekTo::To(seconds) => Seek::Absolute(seconds.max(0.0)),
                    SeekTo::By(seconds) => Seek::Relative(seconds),
                    SeekTo::Fraction(fraction) => Seek::Fraction(fraction.clamp(0.0, 1.0)),
                };
                match self.seek_to(engine, target) {
                    Ok(()) => (done(), None),
                    Err(failure) => (Err((409, failure.to_string())), None),
                }
            }
            Command::Volume { percent, mute } => {
                let mut request = None;
                if let Some(percent) = percent {
                    request = self.set_volume(engine, percent);
                }
                if let Some(mute) = mute
                    && let Err(failure) = engine.set_flag("mute", mute)
                {
                    return (Err((409, failure.to_string())), request);
                }
                (done(), request)
            }
            Command::Open(path) => {
                let path = crate::media::file_path(&path);
                match self.open(engine, &[path], false) {
                    Ok(()) => (done(), None),
                    Err(failure) => (Err((409, failure.to_string())), None),
                }
            }
            Command::Albums => {
                let lists = self.library().lists();
                let albums: Vec<Value> = lists
                    .albums
                    .iter()
                    .enumerate()
                    .map(|(index, album)| {
                        json!({
                            "index": index,
                            "name": album.name,
                            "entries": album.entries.len(),
                            "playing": lists.playing == Some(index),
                            "current": album.current,
                        })
                    })
                    .collect();
                (Ok(Value::from(albums)), None)
            }
            Command::Album(index) => {
                let lists = self.library().lists();
                let Some(album) = lists.albums.get(index) else {
                    return (Err((404, "no such album".to_owned())), None);
                };
                let entries: Vec<Value> = album
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(at, entry)| {
                        json!({
                            "index": at,
                            "title": entry.label(),
                            "duration": entry.duration,
                        })
                    })
                    .collect();
                (
                    Ok(json!({
                        "index": index,
                        "name": album.name,
                        "playing": lists.playing == Some(index),
                        "current": album.current,
                        "entries": entries,
                    })),
                    None,
                )
            }
            Command::Play { album, entry } => {
                let exists = self
                    .library()
                    .lists()
                    .albums
                    .get(album)
                    .is_some_and(|target| entry < target.entries.len());
                if !exists {
                    return (Err((404, "no such entry".to_owned())), None);
                }
                match self.play_entry(engine, album, entry, false) {
                    Ok(()) => (done(), None),
                    Err(failure) => (Err((409, failure.to_string())), None),
                }
            }
        }
    }
}

/// Splits a file's tracks into the control API's audio and subtitle
/// lists.
#[must_use]
pub fn tracks(all: &[rvp_engine::Track]) -> (Vec<Track>, Vec<Track>) {
    let convert = |track: &rvp_engine::Track| Track {
        id: track.id,
        title: track
            .title
            .clone()
            .or_else(|| track.language.clone())
            .or_else(|| track.codec.clone())
            .unwrap_or_else(|| format!("Track {}", track.id)),
        language: track.language.clone(),
        selected: track.selected,
    };
    let of = |kind| {
        all.iter()
            .filter(|track| track.kind == kind)
            .map(convert)
            .collect()
    };
    (
        of(rvp_engine::TrackKind::Audio),
        of(rvp_engine::TrackKind::Subtitle),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_leave_out_nulls() {
        let value = json!({ "a": null, "b": [{ "c": null, "d": 1 }], "e": { "f": null } });
        assert_eq!(without_nulls(value), json!({ "b": [{ "d": 1 }], "e": {} }));
    }

    #[test]
    fn hosts_are_loopback_names_only() {
        assert!(loopback_host("127.0.0.1:48931"));
        assert!(loopback_host("localhost"));
        assert!(loopback_host("LOCALHOST:80"));
        assert!(!loopback_host("evil.example:48931"));
        assert!(!loopback_host("127.0.0.1.evil.example"));
        assert!(!loopback_host("192.168.1.2:48931"));
    }

    #[test]
    fn bodies_are_checked() {
        assert_eq!(seek_body(br#"{"to": 12.5}"#), Ok(SeekTo::To(12.5)));
        assert_eq!(seek_body(br#"{"by": -10}"#), Ok(SeekTo::By(-10.0)));
        assert!(seek_body(br#"{"to": 1, "by": 2}"#).is_err());
        assert!(seek_body(br#"{"at": 1}"#).is_err());
        assert!(seek_body(b"{}").is_err());
        assert_eq!(volume_body(br#"{"percent": 40}"#), Ok((Some(40.0), None)));
        assert_eq!(volume_body(br#"{"mute": true}"#), Ok((None, Some(true))));
        assert!(volume_body(b"{}").is_err());
        assert_eq!(
            open_body(br#"{"path": " C:\\a.mkv "}"#),
            Ok("C:\\a.mkv".to_owned())
        );
        assert!(open_body(br#"{"path": "a\nb"}"#).is_err());
        assert!(open_body(br#"{"path": ""}"#).is_err());
    }
}
