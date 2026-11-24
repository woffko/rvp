//! RVP's control API (M15, D15): a small HTTP/1.1 JSON service through
//! which programs on this computer control RVP, such as the Unified Remote
//! remote in `remote/unified-remote/RVP`.
//!
//! * Off by default; the Remote page turns it on (`remote.enabled`) and
//!   chooses its port (`remote.port`). It listens on 127.0.0.1 only: other
//!   devices reach RVP through a program on this computer, such as Unified
//!   Remote's server.
//! * Every request but `GET /api/nonce` shows that its client knows the
//!   installation's token ([`token`]), in one of two ways:
//!   `Authorization: Bearer TOKEN`, the token itself, for the user's own
//!   scripts and RVP's self-test, which know that they talk to RVP; or a
//!   signature (below), with which the token never travels, for RVP's
//!   remote, which cannot know that: while RVP does not listen, any
//!   account of the computer may take the remote's port.
//! * Requests with a browser's `Origin` header, or whose `Host` is not a
//!   loopback name, are refused, so that web pages cannot reach RVP; a
//!   request without `Host`, with `Host` or `Content-Length` twice, or
//!   with an empty port after the name, is malformed (400). Headers,
//!   bodies, the time a request may take, connections, long polls and
//!   nonces are limited ([`LIMITS`]). After an early answer the server
//!   reads what the client still sends for a moment, by one deadline and
//!   a byte cap. A connection over [`Limits::connections`] gets its 503
//!   from a short-lived thread of its own (as many of those at once as
//!   connections; past that it is closed unanswered), so a slow client
//!   holds neither the thread that accepts nor, when the server stops,
//!   the window's thread ([`Server`]'s `drop` returns within a second).
//! * The server's threads never call mpv and never touch [`App`]: commands
//!   go to the window's thread through the [`Dispatch`] the shell gives
//!   (as MPRIS's and Windows' media controls' commands do) and wait there
//!   for an answer; the status is a snapshot the window's thread publishes
//!   ([`Server::publish`]) as the chrome updates.
//!
//! # Signed requests
//!
//! 1. `GET /api/nonce`, with no token, answers `{"nonce": N}`: 32
//!    lower-case hex digits of 16 random bytes from the system.
//! 2. The client sends its request with
//!    `Authorization: RVP-HMAC-SHA256 nonce=N, signature=S`, where S is
//!    the HMAC-SHA-256 (RFC 2104 over FIPS 180-4's SHA-256) in 64
//!    lower-case hex digits, keyed with the token's text (its 64 hex
//!    digits as ASCII bytes), of the lines
//!    `RVP-HMAC-SHA256`, PORT, N, METHOD and TARGET, each ended by `\n`,
//!    followed by the body's bytes: PORT is the port the client connects
//!    to, in decimal; METHOD the request line's method (upper case);
//!    TARGET the request line's target as sent, the path with its query
//!    before any percent-decoding; the body is empty for a GET. No part
//!    before the body holds a line break, so two different requests never
//!    sign the same text.
//! 3. RVP makes the same text from its own port and the request as it
//!    arrived, and compares the signatures in a time that does not depend
//!    on where they differ ([`sign`]).
//!
//! * A nonce serves one request: the first request that names it spends
//!   it, whether its signature holds or not, so a request cannot be played
//!   again. It lives [`Limits::nonce_life`] (10 s) from being handed out;
//!   the check is made when the request arrives, so a long poll that then
//!   waits [`Limits::long_poll`] is not affected. Each signed request
//!   therefore asks for its nonce first: the remote polls the status once
//!   a second, two exchanges on this computer instead of one, and in
//!   return the client needs no clock and no counter of its own.
//! * RVP keeps at most [`Limits::nonces`] (64); one more drops the oldest,
//!   so `GET /api/nonce` cannot grow RVP's memory. A program that asks for
//!   nonces without end makes other clients' nonces unknown, as holding
//!   every connection denies them service: it gains no access.
//! * A failed check answers 401 `{"error": ...}`, with one message for a
//!   nonce that is unknown, spent or too old and another for a signature
//!   that does not match: neither tells more about the token than that the
//!   check failed.
//! * A listener that is not RVP chooses the nonces and sees the remote's
//!   signatures over them. That is no oracle for the token: HMAC-SHA-256
//!   with a 256-bit random key is a pseudorandom function, so signatures
//!   over texts of someone's choosing give neither the key nor the
//!   signature of any other text. And none of them is accepted by RVP
//!   later: RVP takes only nonces it handed out itself (128 random bits
//!   each, which no one can have had signed in advance) and only for its
//!   own port, so a listener on the remote's port that hands the remote
//!   nonces it took from RVP on another port gets signatures RVP refuses.
//!   The remote takes only a nonce of 32 hex digits, so no line break
//!   reaches the signed text. RVP's answers are not signed: such a
//!   listener can show the remote a false title, no more.
//!
//! | Request | Body or query | Answer |
//! |---|---|---|
//! | `GET /api/nonce` | (no token) | `{"nonce": N}` for one signed request |
//! | `GET /api/status` | `?since=V&wait=S`: waits up to S seconds (at most [`Limits::long_poll`]) for a version other than V | [`Status`] |
//! | `GET /api/actions` | | every action of `ACTIONS`: `name`, `label` |
//! | `POST /api/action/NAME` | | runs the action as keys and menus do |
//! | `POST /api/seek` | `{"to": s}`, `{"by": s}` or `{"fraction": f}` | |
//! | `POST /api/volume` | `{"percent": p}`, `{"mute": b}` or both | |
//! | `POST /api/open` | `{"path": "..."}`: a file, a folder or a network address ([`crate::playlist::admitted`]; 403 for any other) | |
//! | `POST /api/track` | `{"audio": id}`, `{"subtitle": id}` or `{"subtitle": false}` (none): a track of [`Status`]'s lists (404 for another) | |
//! | `GET /api/albums` | | the albums: `index`, `name`, `entries`, `playing`, `current` |
//! | `GET /api/albums/A` | | album A with its `entries`: `index`, `title`, `duration` |
//! | `POST /api/albums/A/play/E` | | plays entry E of album A |
//!
//! Commands answer `{}` (or the data asked for) once the window's thread
//! carried them out; errors answer `{"error": "..."}` with their status.
//! Fields without a value are left out rather than `null` (Unified
//! Remote's JSON reader documents no `null`).

mod http;
pub mod sign;
pub mod token;
pub mod unified;

use std::io::Write as _;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rvp_engine::{Engine, PropertyId, Seek, TrackKind};
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
    /// How long a nonce of `GET /api/nonce` waits for its request.
    pub nonce_life: Duration,
    /// The nonces kept at once (one more drops the oldest).
    pub nonces: usize,
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
    // A client asks for its nonce just before its request: seconds are
    // plenty, also on a loaded machine.
    nonce_life: Duration::from_secs(10),
    // Each client holds one nonce at a time, for a moment.
    nonces: 64,
};

/// The longest path `POST /api/open` takes.
const MAX_PATH: usize = 4096;

/// How long the server reads what a client still sends after an early
/// answer (a refusal or an error), in all, and how much of it at most.
const LINGER: Duration = Duration::from_millis(200);
const LINGER_BYTES: usize = 64 * 1024;

/// The same for a connection over [`Limits::connections`], whose 503 is
/// written by a thread of its own, and that thread's write timeout.
const REFUSAL_LINGER: Duration = Duration::from_millis(50);
const REFUSAL_WRITE: Duration = Duration::from_millis(200);

/// How long [`Server`]'s `drop` waits for the accepting thread to end.
const STOP_WAIT: Duration = Duration::from_millis(800);

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
    /// A file, a folder or a network address to open and play.
    Open(String),
    /// The audio or subtitle track to play (`None`: no subtitles).
    Track(TrackKind, Option<i64>),
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

/// The requests a server let in, by how they showed the token, and those
/// it refused for want of it (the self-test counts the remote's).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// `Authorization: Bearer`.
    pub bearer: u64,
    /// A signature over a nonce.
    pub signed: u64,
    /// Answered 401.
    pub refused: u64,
}

struct Shared {
    published: Mutex<Published>,
    changed: Condvar,
    stop: AtomicBool,
    /// Connections served, and connections over the limit being refused.
    connections: AtomicUsize,
    refusals: AtomicUsize,
    token: String,
    /// The port this server listens on: signatures cover it.
    port: u16,
    nonces: sign::Nonces,
    bearer: AtomicU64,
    signed: AtomicU64,
    refused: AtomicU64,
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
    /// Disconnected once the accepting thread let the port go.
    accept_ended: Receiver<()>,
}

/// A connection's place among those counted by `count` ([`Shared`]'s
/// `connections` or `refusals`), given back when its thread ends, also
/// by a panic, and when the thread does not start.
struct Slot {
    shared: Arc<Shared>,
    count: fn(&Shared) -> &AtomicUsize,
}

impl Slot {
    /// A place, unless `most` are taken.
    fn take(shared: &Arc<Shared>, count: fn(&Shared) -> &AtomicUsize, most: usize) -> Option<Self> {
        if count(shared).fetch_add(1, Ordering::AcqRel) >= most {
            count(shared).fetch_sub(1, Ordering::AcqRel);
            return None;
        }
        Some(Self {
            shared: Arc::clone(shared),
            count,
        })
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        (self.count)(&self.shared).fetch_sub(1, Ordering::AcqRel);
    }
}

fn connections(shared: &Shared) -> &AtomicUsize {
    &shared.connections
}

fn refusals(shared: &Shared) -> &AtomicUsize {
    &shared.refusals
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
            refusals: AtomicUsize::new(0),
            token,
            port: address.port(),
            nonces: sign::Nonces::new(limits.nonce_life, limits.nonces),
            bearer: AtomicU64::new(0),
            signed: AtomicU64::new(0),
            refused: AtomicU64::new(0),
            dispatch,
            limits,
            actions: Value::from(actions).to_string(),
        });
        let (ended, accept_ended) = mpsc::channel::<()>();
        let accept = {
            let shared = Arc::clone(&shared);
            std::thread::Builder::new()
                .name("rvp-control".to_owned())
                .spawn(move || {
                    accept_loop(&listener, &shared);
                    // The port goes before `drop` hears of the end, so that
                    // a new server can take it at once.
                    drop(listener);
                    drop(ended);
                })?
        };
        Ok(Self {
            shared,
            address,
            accept: Some(accept),
            accept_ended,
        })
    }

    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// The requests let in and refused so far.
    #[must_use]
    pub fn counts(&self) -> Counts {
        let count = |counter: &AtomicU64| counter.load(Ordering::Acquire);
        Counts {
            bearer: count(&self.shared.bearer),
            signed: count(&self.shared.signed),
            refused: count(&self.shared.refused),
        }
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
    /// Stops the service on the window's thread: within a second whatever
    /// the clients do. The accepting thread waits for nothing but the next
    /// connection, which the connection made here brings; the threads that
    /// serve clients end on their own within their limits.
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.changed.notify_all();
        // Wakes the blocking accept.
        drop(TcpStream::connect_timeout(
            &self.address,
            Duration::from_millis(200),
        ));
        let Some(accept) = self.accept.take() else {
            return;
        };
        // Disconnected when the thread let the port go; should it not end
        // in time, it is left to end on its own.
        if let Err(mpsc::RecvTimeoutError::Disconnected) = self.accept_ended.recv_timeout(STOP_WAIT)
        {
            let _ = accept.join();
        }
    }
}

/// Takes connections and hands each to a thread of its own; it never
/// reads or writes a connection itself, so no client can hold it.
fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>) {
    for stream in listener.incoming() {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        let most = shared.limits.connections;
        let (slot, refusal) = match Slot::take(shared, connections, most) {
            Some(slot) => (slot, false),
            // As many refusals at once as connections; past that the
            // connection closes without an answer.
            None => match Slot::take(shared, refusals, most) {
                Some(slot) => (slot, true),
                None => continue,
            },
        };
        // A thread that does not start drops its closure, and the slot
        // with it.
        let _ = std::thread::Builder::new()
            .name("rvp-control-client".to_owned())
            .spawn(move || {
                if refusal {
                    refuse(&mut stream);
                } else {
                    serve(&mut stream, &slot.shared);
                }
                drop(slot);
            });
    }
}

/// Answers a connection over [`Limits::connections`] with 503, briefly.
fn refuse(stream: &mut TcpStream) {
    let _ = stream.set_write_timeout(Some(REFUSAL_WRITE));
    let _ = http::respond(stream, 503, &error("too many connections"));
    linger(stream, REFUSAL_LINGER);
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
        linger(stream, LINGER);
    }
}

/// After an early answer, reads what the client still sends before
/// closing: closing with unread data resets the connection, and the client
/// could lose the answer. One deadline for the whole of it, and at most
/// [`LINGER_BYTES`]: a client that sends a byte now and then cannot
/// stretch it.
fn linger(stream: &mut TcpStream, wait: Duration) {
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let deadline = Instant::now() + wait;
    let mut rest = [0u8; 4096];
    let mut drained = 0;
    while drained < LINGER_BYTES {
        let Some(left) = deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
        else {
            break;
        };
        if stream.set_read_timeout(Some(left)).is_err() {
            break;
        }
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

/// The name of a `Host` header without its port; `None` when the header
/// is malformed: an empty name, or an empty port or one that is not a
/// number after the colon.
fn host_name(host: &str) -> Option<&str> {
    let name = match host.rsplit_once(':') {
        Some((name, port)) => {
            (!port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())).then_some(name)?
        }
        None => host,
    };
    (!name.is_empty()).then_some(name)
}

/// A loopback name.
fn loopback_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("localhost") || name == "127.0.0.1"
}

/// Shows whether the request knows the token: the token itself
/// (`Bearer`) or a signature over a nonce RVP handed out (the module
/// comment); counts it.
fn authorize(shared: &Shared, request: &Request) -> Result<(), &'static str> {
    const NEEDED: &str = "a valid token or signature is needed";
    let value = request.header("authorization").ok_or(NEEDED)?;
    if let Some(presented) = value.strip_prefix("Bearer ") {
        if !token::matches(presented.trim(), &shared.token) {
            return Err(NEEDED);
        }
        shared.bearer.fetch_add(1, Ordering::AcqRel);
        return Ok(());
    }
    let credentials = value
        .strip_prefix(sign::SCHEME)
        .and_then(|rest| rest.strip_prefix(' '))
        .ok_or(NEEDED)?;
    let (nonce, presented) =
        sign::credentials(credentials).ok_or("the signature's credentials are malformed")?;
    if !shared.nonces.spend(nonce, Instant::now()) {
        return Err("the nonce is unknown, spent or too old: ask GET /api/nonce for another");
    }
    let expected = sign::signature(
        &shared.token,
        shared.port,
        nonce,
        &request.method,
        &request.target,
        &request.body,
    );
    if !token::matches(presented, &expected) {
        return Err("the signature does not match RVP's token");
    }
    shared.signed.fetch_add(1, Ordering::AcqRel);
    Ok(())
}

fn route(shared: &Shared, request: &Request) -> (u16, String) {
    if request.header("origin").is_some() {
        return (403, error("requests from web pages are refused"));
    }
    // `read_request` lets no request without one `Host` through.
    match request.header("host").and_then(host_name) {
        Some(name) if loopback_name(name) => {}
        Some(_) => return (403, error("this service answers only on 127.0.0.1")),
        None => return (400, error("malformed Host header")),
    }
    let segments: Vec<&str> = request
        .path
        .trim_end_matches('/')
        .split('/')
        .skip(1)
        .collect();
    let get = request.method == "GET";
    let post = request.method == "POST";
    if get && segments == ["api", "nonce"] {
        return match shared.nonces.issue(Instant::now()) {
            Some(nonce) => (200, json!({ "nonce": nonce }).to_string()),
            None => (503, error("the system gave no random bytes")),
        };
    }
    if let Err(why) = authorize(shared, request) {
        shared.refused.fetch_add(1, Ordering::AcqRel);
        return (401, error(why));
    }
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
        ["api", "track"] if post => match track_body(&request.body) {
            Ok((kind, id)) => ask(shared, Command::Track(kind, id)),
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
            "status" | "actions" | "albums" | "action" | "seek" | "volume" | "open" | "track"
            | "nonce",
            ..,
        ] => (405, error("method not allowed")),
        _ => (404, error("not found")),
    }
}

/// The status now, or after the next change (a long poll).
fn status(shared: &Shared, request: &Request) -> (u16, String) {
    let since = request.query("since").and_then(|v| v.parse::<u64>().ok());
    // Clamped while it is a number: a `Duration` of a huge one panics.
    let wait = request
        .query("wait")
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0)
        .map_or(Duration::ZERO, |v| {
            Duration::from_secs_f64(v.min(shared.limits.long_poll.as_secs_f64()))
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

/// `POST /api/track`'s body: one member, `audio` with a track's ID, or
/// `subtitle` with an ID or `false` for none.
fn track_body(body: &[u8]) -> Result<(TrackKind, Option<i64>), &'static str> {
    const EXPECTED: &str = "expected {\"audio\": id}, {\"subtitle\": id} or {\"subtitle\": false}";
    let Ok(Value::Object(members)) = serde_json::from_slice::<Value>(body) else {
        return Err(EXPECTED);
    };
    let mut members = members.into_iter();
    let (Some((name, value)), None) = (members.next(), members.next()) else {
        return Err(EXPECTED);
    };
    let id = value.as_i64().filter(|id| *id > 0);
    match (name.as_str(), id, value) {
        ("audio", Some(id), _) => Ok((TrackKind::Audio, Some(id))),
        ("subtitle", Some(id), _) => Ok((TrackKind::Subtitle, Some(id))),
        ("subtitle", None, Value::Bool(false)) => Ok((TrackKind::Subtitle, None)),
        _ => Err(EXPECTED),
    }
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

    /// Opens what a request names. It is another program's text: a file,
    /// a folder or a network address ([`crate::playlist::admitted`]), and
    /// nothing else mpv would open.
    fn control_open(&mut self, engine: &Engine, path: &str) -> Answer {
        let Some(path) = crate::playlist::admitted(path) else {
            let refused = "only files, folders and network addresses are opened";
            return Err((403, refused.to_owned()));
        };
        match self.open(engine, &[path], false) {
            Ok(()) => Ok(json!({})),
            Err(failure) => Err((409, failure.to_string())),
        }
    }

    /// `POST /api/track`: plays the file's track `id` of `kind` (`None`: no
    /// subtitles), as the Audio and Subtitles menus do.
    fn control_track(&mut self, engine: &Engine, kind: TrackKind, id: Option<i64>) -> Answer {
        if !self.model.media_loaded() {
            return Err((409, "nothing is open".to_owned()));
        }
        let known = id.is_none_or(|id| {
            engine
                .tracks()
                .iter()
                .any(|track| track.kind == kind && track.id == id)
        });
        if !known {
            return Err((404, "no such track".to_owned()));
        }
        self.select_track(engine, kind, id);
        Ok(json!({}))
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
            Command::Open(path) => (self.control_open(engine, &path), None),
            Command::Track(kind, id) => (self.control_track(engine, kind, id), None),
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
    fn a_track_is_named_by_one_member() {
        assert_eq!(
            track_body(br#"{"audio": 2}"#),
            Ok((TrackKind::Audio, Some(2)))
        );
        assert_eq!(
            track_body(br#"{"subtitle": 3}"#),
            Ok((TrackKind::Subtitle, Some(3)))
        );
        assert_eq!(
            track_body(br#"{"subtitle": false}"#),
            Ok((TrackKind::Subtitle, None))
        );
        for refused in [
            &br#"{"audio": false}"#[..],
            br#"{"audio": 0}"#,
            br#"{"audio": -1}"#,
            br#"{"audio": 1.5}"#,
            br#"{"subtitle": true}"#,
            br#"{"video": 1}"#,
            br#"{"audio": 1, "subtitle": 2}"#,
            b"{}",
            b"[1]",
            b"",
        ] {
            assert!(
                track_body(refused).is_err(),
                "{}",
                String::from_utf8_lossy(refused)
            );
        }
    }

    #[test]
    fn answers_leave_out_nulls() {
        let value = json!({ "a": null, "b": [{ "c": null, "d": 1 }], "e": { "f": null } });
        assert_eq!(without_nulls(value), json!({ "b": [{ "d": 1 }], "e": {} }));
    }

    #[test]
    fn hosts_are_loopback_names_only() {
        let loopback = |host| host_name(host).is_some_and(loopback_name);
        assert!(loopback("127.0.0.1:48931"));
        assert!(loopback("localhost"));
        assert!(loopback("LOCALHOST:80"));
        assert!(!loopback("evil.example:48931"));
        assert!(!loopback("127.0.0.1.evil.example"));
        assert!(!loopback("192.168.1.2:48931"));
        // Malformed.
        for host in ["localhost:", "", ":48931", "localhost:80a", "[::1]"] {
            assert_eq!(host_name(host), None, "{host:?}");
        }
    }

    /// A worker's place comes back when its thread panics.
    #[test]
    fn a_place_comes_back_after_a_panic() {
        let dispatch: Dispatch = Arc::new(|_, _| {});
        let server = Server::start_with(0, String::new(), dispatch, LIMITS).expect("server");
        let shared = &server.shared;
        let slot = Slot::take(shared, connections, 1).expect("a place");
        assert!(Slot::take(shared, connections, 1).is_none(), "one place");
        let worker = std::thread::spawn(move || {
            let _slot = slot;
            panic!("a worker fails");
        });
        assert!(worker.join().is_err());
        assert_eq!(shared.connections.load(Ordering::Acquire), 0);
        assert!(Slot::take(shared, connections, 1).is_some());
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
