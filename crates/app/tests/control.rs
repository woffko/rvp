//! M15's control API (`rvp_app::control`) on a headless player (no
//! window, no sound): every endpoint, the token, the refusal of browsers'
//! requests, the limits, and that the window's thread never waits for the
//! server. The test's main thread plays the window's thread: it carries
//! out the commands the server hands it, drains the player and publishes
//! the status; clients run on threads of their own.
//!
//! Without a player: signed requests (nonces and signatures), slow
//! clients that hold neither the server's threads nor its stop, huge long
//! polls, and the `Host` and `Content-Length` rules.
//!
//! `RVP_TESTDATA` points at `testdata/media` when the test runs elsewhere
//! (the Windows VM).

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rvp_app::control::{self, Command, Counts, Limits, Reply, Server, sign};
use rvp_app::{App, Settings};
use rvp_engine::{Engine, EngineOptions};
use serde_json::Value;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Short limits, so that the test of each one is quick.
const TEST_LIMITS: Limits = Limits {
    header_bytes: 8 * 1024,
    body_bytes: 64 * 1024,
    request: Duration::from_millis(800),
    write: Duration::from_secs(2),
    connections: 6,
    long_poll: Duration::from_secs(30),
    command: Duration::from_millis(800),
    nonce_life: Duration::from_secs(10),
    nonces: 64,
};

fn media(name: &str) -> String {
    let dir = std::env::var_os("RVP_TESTDATA").map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media")),
        PathBuf::from,
    );
    dir.join(name).to_string_lossy().into_owned()
}

/// One exchange: the status and the JSON answer.
fn exchange(address: SocketAddr, raw: &[u8]) -> (u16, Value) {
    let mut stream = TcpStream::connect(address).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(40)))
        .expect("timeout");
    stream.write_all(raw).expect("send");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("answer");
    let text = String::from_utf8(answer).expect("utf-8 answer");
    let (head, body) = text.split_once("\r\n\r\n").expect("head and body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("status line");
    assert!(head.contains("Content-Type: application/json"), "{head}");
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

/// A request with the token (unless `token` is `None`).
fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
    token: Option<&str>,
) -> Vec<u8> {
    let authorization = token.map(|token| format!("Bearer {token}"));
    request_with(address, method, path, body, authorization.as_deref())
}

/// A request with this `Authorization` (none for `None`).
fn request_with(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
    authorization: Option<&str>,
) -> Vec<u8> {
    let mut raw = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        address.port()
    );
    if let Some(authorization) = authorization {
        let _ = write!(raw, "Authorization: {authorization}\r\n");
    }
    let body = body.unwrap_or("");
    if method == "POST" {
        let _ = write!(
            raw,
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
    }
    raw.push_str("\r\n");
    raw.push_str(body);
    raw.into_bytes()
}

struct Harness {
    app: App,
    engine: Engine,
    server: Server,
    commands: Receiver<(Command, Reply)>,
    dir: PathBuf,
    /// The longest `publish` and the longest command took on the window's
    /// thread.
    longest_publish: Duration,
    longest_command: Duration,
}

impl Harness {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("rvp-control-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            ..Settings::default()
        };
        let (sender, commands) = mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let dispatch: control::Dispatch = Arc::new(move |command, reply| {
            let _ = sender.lock().expect("dispatch lock").send((command, reply));
        });
        Self {
            app: App::new(settings, &dir).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
            server: Server::start_with(0, TOKEN.to_owned(), dispatch, TEST_LIMITS).expect("server"),
            commands,
            dir,
            longest_publish: Duration::ZERO,
            longest_command: Duration::ZERO,
        }
    }

    fn address(&self) -> SocketAddr {
        self.server.address()
    }

    /// One turn of the window's thread.
    fn pump(&mut self) {
        while let Ok((command, reply)) = self.commands.try_recv() {
            let started = Instant::now();
            let (answer, _request) = self.app.control_command(&self.engine, command);
            self.longest_command = self.longest_command.max(started.elapsed());
            let _ = reply.send(answer);
        }
        drop(self.app.drain(&self.engine, Instant::now()));
        let state = self.app.control_state(false, &[], &[]);
        let started = Instant::now();
        self.server.publish(self.app.control_position(), &state);
        self.longest_publish = self.longest_publish.max(started.elapsed());
    }

    /// Runs a client on its own thread while the window's thread turns.
    fn call(&mut self, method: &str, path: &str, body: Option<&str>) -> (u16, Value) {
        let raw = request(self.address(), method, path, body, Some(TOKEN));
        self.send(raw)
    }

    fn send(&mut self, raw: Vec<u8>) -> (u16, Value) {
        let address = self.address();
        let client = std::thread::spawn(move || exchange(address, &raw));
        self.finish(client)
    }

    fn finish<T>(&mut self, client: JoinHandle<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(40);
        while !client.is_finished() {
            assert!(Instant::now() < deadline, "the client hangs");
            self.pump();
            std::thread::sleep(Duration::from_millis(5));
        }
        client.join().expect("client")
    }

    /// Turns until the version stays the same for a moment (the file's
    /// late properties have arrived); the version.
    fn settle(&mut self) -> u64 {
        let mut last = None;
        loop {
            let (_, value) = self.call("GET", "/api/status", None);
            let version = value["version"].as_u64().expect("version");
            if last == Some(version) {
                return version;
            }
            last = Some(version);
            for _ in 0..30 {
                self.pump();
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    /// Turns until the status shows `done`.
    fn wait(&mut self, what: &str, done: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let (status, value) = self.call("GET", "/api/status", None);
            assert_eq!(status, 200, "{value}");
            if done(&value) {
                return value;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: {value}"
            );
            for _ in 0..10 {
                self.pump();
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn near(value: &Value, key: &str, expected: f64, within: f64) -> bool {
    value[key]
        .as_f64()
        .is_some_and(|v| (v - expected).abs() <= within)
}

#[test]
fn the_control_api_serves_a_headless_player() {
    let mut h = Harness::new();
    assert!(h.address().ip().is_loopback(), "{}", h.address());
    access(&mut h);
    playback(&mut h);
    albums(&mut h);
    addresses(&mut h);
    mistakes(&mut h);
    limits(&mut h);
    window_thread(&mut h);
}

/// The token, browsers and other hosts; the status before anything
/// opens, and the actions.
fn access(h: &mut Harness) {
    let address = h.address();
    let (status, _) = h.send(request(address, "GET", "/api/status", None, None));
    assert_eq!(status, 401, "no token");
    let (status, _) = h.send(request(
        address,
        "GET",
        "/api/status",
        None,
        Some(&TOKEN.replace('0', "1")),
    ));
    assert_eq!(status, 401, "a wrong token");
    let mut raw = request(address, "GET", "/api/status", None, Some(TOKEN));
    raw.splice(
        raw.len() - 2..raw.len() - 2,
        b"Origin: http://evil.example\r\n".iter().copied(),
    );
    assert_eq!(h.send(raw).0, 403, "a browser's request");
    let raw = format!(
        "GET /api/status HTTP/1.1\r\nHost: evil.example:{}\r\nAuthorization: Bearer {TOKEN}\r\n\r\n",
        address.port()
    );
    assert_eq!(h.send(raw.into_bytes()).0, 403, "another host name");

    // The status before anything opens, and the actions.
    let (status, value) = h.call("GET", "/api/status", None);
    assert_eq!(status, 200);
    assert_eq!(value["state"], "stopped", "{value}");
    assert!(value["version"].as_u64().is_some(), "{value}");
    let (status, actions) = h.call("GET", "/api/actions", None);
    assert_eq!(status, 200);
    let names: Vec<&str> = actions
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|action| action["name"].as_str())
        .collect();
    assert_eq!(names.len(), rvp_app::ACTIONS.len());
    for name in [
        "play-pause",
        "next",
        "previous",
        "toggle-fullscreen",
        "toggle-mute",
    ] {
        assert!(names.contains(&name), "{name} in {names:?}");
    }
}

/// Open, pause and play, long polls, seeks, the volume and mute.
fn playback(h: &mut Harness) {
    let address = h.address();
    let fixture = media("fixture-60s.mkv");
    let body = serde_json::json!({ "path": fixture }).to_string();
    assert_eq!(h.call("POST", "/api/open", Some(&body)).0, 200);
    let playing = h.wait("the file to play", |s| {
        s["state"] == "playing" && near(s, "duration", 60.0, 1.0)
    });
    assert!(
        playing["file"]
            .as_str()
            .is_some_and(|f| f.ends_with("fixture-60s.mkv")),
        "{playing}"
    );
    assert_eq!(playing["album"], 0, "{playing}");
    assert_eq!(playing["entry"], 0, "{playing}");
    let version = h.settle();

    // A long poll waits for the next change: the pause.
    let poll = {
        let raw = request(
            address,
            "GET",
            &format!("/api/status?since={version}&wait=20"),
            None,
            Some(TOKEN),
        );
        std::thread::spawn(move || {
            let started = Instant::now();
            let answer = exchange(address, &raw);
            (answer, started.elapsed())
        })
    };
    for _ in 0..40 {
        h.pump();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !poll.is_finished(),
        "the long poll answered before any change"
    );
    assert_eq!(h.call("POST", "/api/action/play-pause", None).0, 200);
    let ((status, changed), waited) = h.finish(poll);
    assert_eq!(status, 200);
    assert!(changed["version"].as_u64() > Some(version), "{changed}");
    assert!(waited < Duration::from_secs(10), "{waited:?}");
    h.wait("the pause", |s| s["state"] == "paused");
    // Without a change it answers when its wait ends.
    let (_, now) = h.call("GET", "/api/status", None);
    let current = now["version"].as_u64().expect("version");
    let started = Instant::now();
    let (status, same) = h.call("GET", &format!("/api/status?since={current}&wait=1"), None);
    assert_eq!(status, 200);
    assert_eq!(same["version"].as_u64(), Some(current), "{same}");
    assert!(
        started.elapsed() >= Duration::from_millis(900),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(h.call("POST", "/api/action/play-pause", None).0, 200);
    h.wait("playback again", |s| s["state"] == "playing");

    // Seek, volume and mute.
    assert_eq!(h.call("POST", "/api/seek", Some(r#"{"to": 30}"#)).0, 200);
    h.wait("the seek to 30 s", |s| near(s, "position", 30.0, 2.0));
    assert_eq!(h.call("POST", "/api/seek", Some(r#"{"by": -10}"#)).0, 200);
    h.wait("the seek back to 20 s", |s| near(s, "position", 20.0, 2.0));
    assert_eq!(
        h.call("POST", "/api/seek", Some(r#"{"fraction": 0.75}"#)).0,
        200
    );
    h.wait("the seek to 45 s", |s| near(s, "position", 45.0, 2.0));
    assert_eq!(
        h.call("POST", "/api/volume", Some(r#"{"percent": 40}"#)).0,
        200
    );
    h.wait("the volume", |s| near(s, "volume", 40.0, 0.6));
    assert_eq!(
        h.call("POST", "/api/volume", Some(r#"{"mute": true}"#)).0,
        200
    );
    h.wait("mute", |s| s["mute"] == true);
    assert_eq!(h.call("POST", "/api/action/toggle-mute", None).0, 200);
    h.wait("mute off", |s| s["mute"] == false);
}

/// Albums and entries.
fn albums(h: &mut Harness) {
    let (status, albums) = h.call("GET", "/api/albums", None);
    assert_eq!(status, 200);
    assert_eq!(albums[0]["playing"], true, "{albums}");
    assert_eq!(albums[0]["entries"], 1, "{albums}");
    let tone = serde_json::json!({ "path": media("fixture-tone.ogg") }).to_string();
    assert_eq!(h.call("POST", "/api/open", Some(&tone)).0, 200);
    h.wait("the tone", |s| s["entry"] == 1 && s["state"] == "playing");
    let (status, album) = h.call("GET", "/api/albums/0", None);
    assert_eq!(status, 200);
    let titles: Vec<&str> = album["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter_map(|entry| entry["title"].as_str())
        .collect();
    assert_eq!(titles.len(), 2, "{album}");
    assert_eq!(h.call("POST", "/api/albums/0/play/0", None).0, 200);
    h.wait("the first entry again", |s| {
        s["entry"] == 0 && s["state"] == "playing"
    });
    assert_eq!(h.call("GET", "/api/albums/7", None).0, 404);
    assert_eq!(h.call("POST", "/api/albums/0/play/9", None).0, 404);
}

/// `open` takes files and network addresses: every other address is
/// refused, and nothing of it reaches the album or the player.
fn addresses(h: &mut Harness) {
    let entries = |h: &mut Harness| {
        let (status, album) = h.call("GET", "/api/albums/0", None);
        assert_eq!(status, 200);
        album["entries"].as_array().expect("entries").len()
    };
    let before = entries(h);
    for path in [
        "av://lavfi:testsrc",
        "AV://v4l2:/dev/video0",
        "fdclose://9",
        "dvd://longest//discs/film",
        "edl://a.mkv",
        "file://example.org/share/a.mkv",
        "-",
    ] {
        let body = serde_json::json!({ "path": path }).to_string();
        let (status, answer) = h.call("POST", "/api/open", Some(&body));
        assert_eq!(status, 403, "{path}: {answer}");
        assert!(answer["error"].is_string(), "{path}: {answer}");
    }
    assert_eq!(entries(h), before);
    let (_, status) = h.call("GET", "/api/status", None);
    assert_eq!(status["entry"], 0, "{status}");
    assert!(
        status["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("fixture-60s.mkv")),
        "{status}"
    );
    // A file URL of this computer is its file: the tone joins the album
    // once more and plays.
    let url = rvp_app::media::file_url(&media("fixture-tone.ogg"));
    assert!(url.starts_with("file:///"), "{url}");
    let body = serde_json::json!({ "path": url }).to_string();
    assert_eq!(h.call("POST", "/api/open", Some(&body)).0, 200);
    h.wait("the tone by its URL", |s| {
        s["entry"] == before
            && s["state"] == "playing"
            && s["file"]
                .as_str()
                .is_some_and(|file| file.ends_with("fixture-tone.ogg"))
    });
    assert_eq!(entries(h), before + 1);
    assert_eq!(h.call("POST", "/api/albums/0/play/0", None).0, 200);
    h.wait("the first entry again", |s| {
        s["entry"] == 0 && s["state"] == "playing"
    });
}

/// Wrong paths, methods and bodies.
fn mistakes(h: &mut Harness) {
    assert_eq!(h.call("POST", "/api/action/no-such-action", None).0, 404);
    assert_eq!(h.call("GET", "/api/action/play-pause", None).0, 405);
    assert_eq!(h.call("POST", "/api/status", None).0, 405);
    assert_eq!(h.call("GET", "/nothing", None).0, 404);
    assert_eq!(
        h.call("POST", "/api/seek", Some(r#"{"to": "soon"}"#)).0,
        400
    );
    assert_eq!(h.call("POST", "/api/volume", Some("{}")).0, 400);
    assert_eq!(h.call("POST", "/api/open", Some(r#"{"path": ""}"#)).0, 400);
}

/// The limits: headers, bodies, slow clients, connections, and a
/// window's thread that does not answer.
fn limits(h: &mut Harness) {
    let address = h.address();
    let raw = format!(
        "GET /api/status HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Pad: {}\r\n\r\n",
        "a".repeat(9000)
    );
    assert_eq!(h.send(raw.into_bytes()).0, 431);
    let raw = format!(
        "POST /api/open HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\n\r\n",
        70 * 1024
    );
    assert_eq!(h.send(raw.into_bytes()).0, 413);
    let started = Instant::now();
    let (status, _) = h.send(b"GET /api/status HTTP/1.1\r\nHost: 127.0.0.1\r\n".to_vec());
    assert_eq!(status, 408, "an unfinished request");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    let idle: Vec<TcpStream> = (0..TEST_LIMITS.connections)
        .map(|_| TcpStream::connect(address).expect("idle connection"))
        .collect();
    std::thread::sleep(Duration::from_millis(100));
    let (status, _) = h.send(request(address, "GET", "/api/status", None, Some(TOKEN)));
    assert_eq!(status, 503, "one connection too many");
    drop(idle);
    // The idle connections end with their request time.
    std::thread::sleep(TEST_LIMITS.request + Duration::from_millis(300));
    let raw = request(address, "POST", "/api/action/play-pause", None, Some(TOKEN));
    let unanswered = std::thread::spawn(move || exchange(address, &raw));
    let started = Instant::now();
    let (status, value) = unanswered.join().expect("client");
    assert_eq!(status, 503, "{value}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    // The command waits in the queue; the window's thread carries it out
    // late, and nobody hears.
    h.pump();
}

/// The window's thread never waited for the server: with long polls
/// pending, publishing stays quick.
fn window_thread(h: &mut Harness) {
    let address = h.address();
    let polls: Vec<JoinHandle<(u16, Value)>> = (0..3)
        .map(|_| {
            let raw = request(
                address,
                "GET",
                "/api/status?since=0&wait=0.1",
                None,
                Some(TOKEN),
            );
            std::thread::spawn(move || exchange(address, &raw))
        })
        .collect();
    for poll in polls {
        assert_eq!(h.finish(poll).0, 200);
    }
    assert!(
        h.longest_publish < Duration::from_millis(20),
        "publish took {:?}",
        h.longest_publish
    );
    assert!(
        h.longest_command < Duration::from_millis(500),
        "a command took {:?}",
        h.longest_command
    );
    eprintln!(
        "longest publish {:?}, longest command {:?}",
        h.longest_publish, h.longest_command
    );
}

/// A server without a player: its dispatch answers every command at once.
fn bare_server(limits: Limits) -> Server {
    let dispatch: control::Dispatch = Arc::new(|_command, reply: Reply| {
        let _ = reply.send(Ok(serde_json::json!({})));
    });
    Server::start_with(0, TOKEN.to_owned(), dispatch, limits).expect("server")
}

/// A nonce from `GET /api/nonce`, asked without a token.
fn nonce(address: SocketAddr) -> String {
    let (status, value) = exchange(address, &request(address, "GET", "/api/nonce", None, None));
    assert_eq!(status, 200, "{value}");
    let nonce = value["nonce"].as_str().expect("a nonce").to_owned();
    assert!(
        nonce.len() == 32
            && nonce
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "{nonce}"
    );
    nonce
}

/// A request signed with `token` for `signed` (method, target, body, port)
/// that sends `sent` (method, target, body).
fn signed_request(
    address: SocketAddr,
    nonce: &str,
    token: &str,
    signed: (&str, &str, &str, u16),
    sent: (&str, &str, &str),
) -> Vec<u8> {
    let (method, target, body, port) = signed;
    let authorization = sign::authorization(token, port, nonce, method, target, body.as_bytes());
    let (method, target, body) = sent;
    let body = (method == "POST").then_some(body);
    request_with(address, method, target, body, Some(&authorization))
}

/// `GET /api/status` signed with `token` over `nonce`.
fn status_signed(address: SocketAddr, nonce: &str, token: &str) -> Vec<u8> {
    let (method, target, port) = ("GET", "/api/status", address.port());
    signed_request(
        address,
        nonce,
        token,
        (method, target, "", port),
        (method, target, ""),
    )
}

/// Signed requests: the token never travels, each part of the request is
/// covered, and a nonce serves one request within its life; `Bearer`
/// still serves scripts.
#[test]
fn signed_requests_carry_no_token() {
    let limits = Limits {
        nonce_life: Duration::from_millis(600),
        nonces: 4,
        ..TEST_LIMITS
    };
    let server = bare_server(limits);
    let address = server.address();
    signatures_pass_once(address);
    signatures_cover_every_part(address);
    nonces_are_young_and_few(address, &limits);

    // The nonces are for this computer's programs, not for web pages; the
    // token still serves scripts.
    let mut raw = request(address, "GET", "/api/nonce", None, None);
    raw.splice(
        raw.len() - 2..raw.len() - 2,
        b"Origin: http://evil.example\r\n".iter().copied(),
    );
    assert_eq!(exchange(address, &raw).0, 403);
    let raw = request(address, "POST", "/api/nonce", Some(""), Some(TOKEN));
    assert_eq!(exchange(address, &raw).0, 405);
    let raw = request(address, "GET", "/api/status", None, Some(TOKEN));
    assert_eq!(exchange(address, &raw).0, 200);
    assert_eq!(
        server.counts(),
        Counts {
            bearer: 2,
            signed: 4 + limits.nonces as u64,
            // A spent nonce, four parts, another token, a nonce RVP did
            // not give, no signature, an old nonce and a dropped one.
            refused: 10,
        }
    );
}

/// A signed GET and POST pass; a nonce serves once.
fn signatures_pass_once(address: SocketAddr) {
    let raw = status_signed(address, &nonce(address), TOKEN);
    let (status, value) = exchange(address, &raw);
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["state"], "", "{value}");
    let (status, value) = exchange(address, &raw);
    assert_eq!(status, 401, "a spent nonce: {value}");
    assert!(
        value["error"].as_str().is_some_and(|e| e.contains("nonce")),
        "{value}"
    );
    let port = address.port();
    for (method, target, body) in [
        ("POST", "/api/seek", r#"{"to":12}"#),
        ("POST", "/api/action/play-pause", ""),
        ("GET", "/api/status?since=0&wait=0", ""),
    ] {
        let raw = signed_request(
            address,
            &nonce(address),
            TOKEN,
            (method, target, body, port),
            (method, target, body),
        );
        assert_eq!(exchange(address, &raw).0, 200, "{method} {target}");
    }
}

/// Each part counts: what was signed is not what was sent.
fn signatures_cover_every_part(address: SocketAddr) {
    let port = address.port();
    for (signed, sent, what) in [
        (
            ("POST", "/api/seek", r#"{"to":12}"#, port),
            ("POST", "/api/seek", r#"{"to":13}"#),
            "another body",
        ),
        (
            ("GET", "/api/status", "", port),
            ("GET", "/api/status?since=1", ""),
            "another query",
        ),
        (
            ("GET", "/api/action/play-pause", "", port),
            ("POST", "/api/action/play-pause", ""),
            "another method",
        ),
        (
            ("GET", "/api/status", "", port.wrapping_add(1)),
            ("GET", "/api/status", ""),
            "another port",
        ),
    ] {
        let raw = signed_request(address, &nonce(address), TOKEN, signed, sent);
        let (status, value) = exchange(address, &raw);
        assert_eq!(status, 401, "{what}: {value}");
        assert!(
            value["error"]
                .as_str()
                .is_some_and(|e| e.contains("signature")),
            "{what}: {value}"
        );
    }
    let raw = status_signed(address, &nonce(address), &TOKEN.replace('0', "1"));
    assert_eq!(exchange(address, &raw).0, 401, "another token");
    let raw = status_signed(address, "00112233445566778899aabbccddeeff", TOKEN);
    assert_eq!(exchange(address, &raw).0, 401, "a nonce RVP did not give");
    let malformed = format!("{} nonce={}", sign::SCHEME, nonce(address));
    let raw = request_with(address, "GET", "/api/status", None, Some(&malformed));
    assert_eq!(exchange(address, &raw).0, 401, "no signature");
}

/// A nonce lives `nonce_life`, and at most `nonces` are kept: one more
/// drops the oldest.
fn nonces_are_young_and_few(address: SocketAddr, limits: &Limits) {
    let old = nonce(address);
    std::thread::sleep(limits.nonce_life + Duration::from_millis(100));
    let raw = status_signed(address, &old, TOKEN);
    assert_eq!(exchange(address, &raw).0, 401, "an old nonce");
    let kept: Vec<String> = (0..=limits.nonces).map(|_| nonce(address)).collect();
    for (at, nonce) in kept.iter().enumerate() {
        let expected = if at == 0 { 401 } else { 200 };
        let raw = status_signed(address, nonce, TOKEN);
        assert_eq!(exchange(address, &raw).0, expected, "nonce {at}");
    }
}

/// Connects, sends `raw`, reads the answer to its end (the server shuts
/// its side after answering), then sends a byte every `every` until a
/// write fails: how long after the answer the server closed (`None`: not
/// within 10 s).
fn trickle_after_answer(address: SocketAddr, raw: &[u8], every: Duration) -> Option<Duration> {
    let mut stream = TcpStream::connect(address).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .expect("timeout");
    stream.write_all(raw).expect("send");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("answer");
    assert!(answer.starts_with(b"HTTP/1.1 "), "{answer:?}");
    let answered = Instant::now();
    while answered.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(every);
        if stream.write_all(b"x").is_err() {
            return Some(answered.elapsed());
        }
    }
    None
}

/// Slow clients hold nothing for long: a client that trickles bytes after
/// its refusal frees its worker within the linger's deadline; one over
/// the connection limit does not hold the accepting thread; and stopping
/// the server takes under a second with such clients connected.
#[test]
fn slow_clients_hold_neither_the_workers_nor_the_server() {
    let limits = Limits {
        request: Duration::from_secs(5),
        connections: 3,
        ..TEST_LIMITS
    };
    let server = bare_server(limits);
    let address = server.address();

    // Clients without the token, a byte every 100 ms after their 401.
    let tricklers: Vec<JoinHandle<Option<Duration>>> = (0..limits.connections)
        .map(|_| {
            let raw = request(address, "GET", "/api/status", None, None);
            std::thread::spawn(move || {
                trickle_after_answer(address, &raw, Duration::from_millis(100))
            })
        })
        .collect();
    for trickler in tricklers {
        let closed = trickler.join().expect("trickler");
        assert!(
            closed.is_some_and(|after| after < Duration::from_secs(2)),
            "closed {closed:?} after the answer"
        );
    }
    // Their workers are free again.
    let raw = request(address, "GET", "/api/status", None, Some(TOKEN));
    assert_eq!(exchange(address, &raw).0, 200);

    // Every worker waits for a request (5 s at most); a client over the
    // limit trickles a byte every 20 ms after its 503, and the next one is
    // still answered at once.
    let idle: Vec<TcpStream> = (0..limits.connections)
        .map(|_| TcpStream::connect(address).expect("idle connection"))
        .collect();
    std::thread::sleep(Duration::from_millis(100));
    let over = {
        let raw = request(address, "GET", "/api/status", None, Some(TOKEN));
        std::thread::spawn(move || trickle_after_answer(address, &raw, Duration::from_millis(20)))
    };
    std::thread::sleep(Duration::from_millis(100));
    let started = Instant::now();
    let raw = request(address, "GET", "/api/status", None, Some(TOKEN));
    assert_eq!(exchange(address, &raw).0, 503, "one connection too many");
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "the next client waited {:?}",
        started.elapsed()
    );
    let closed = over.join().expect("trickler");
    assert!(
        closed.is_some_and(|after| after < Duration::from_secs(1)),
        "the refused client closed {closed:?} after its answer"
    );

    // Stopping, with the workers held and clients over the limit
    // trickling, takes under a second.
    let over: Vec<JoinHandle<Option<Duration>>> = (0..2)
        .map(|_| {
            let raw = request(address, "GET", "/api/status", None, Some(TOKEN));
            std::thread::spawn(move || {
                trickle_after_answer(address, &raw, Duration::from_millis(20))
            })
        })
        .collect();
    std::thread::sleep(Duration::from_millis(100));
    let (stopped, stop) = mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        drop(server);
        let _ = stopped.send(());
    });
    assert!(
        stop.recv_timeout(Duration::from_secs(1)).is_ok(),
        "drop(server) took over a second"
    );
    eprintln!("the server stopped in {:?}", started.elapsed());
    drop(idle);
    for trickler in over {
        let _ = trickler.join();
    }
    // The port is free at once.
    assert!(
        TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err(),
        "nothing listens"
    );
}

/// A long poll's wait is clamped before it is used: a huge one answers,
/// and no worker's place is lost.
#[test]
fn huge_waits_answer_and_keep_every_worker() {
    let server = bare_server(TEST_LIMITS);
    let address = server.address();
    for wait in ["1e300", "1e20", "inf", "NaN", "-5"] {
        for _ in 0..=TEST_LIMITS.connections {
            let path = format!("/api/status?wait={wait}");
            let (status, value) =
                exchange(address, &request(address, "GET", &path, None, Some(TOKEN)));
            assert_eq!(status, 200, "wait={wait}: {value}");
        }
    }
    // A wait for a change past the longest poll ends with the poll.
    let raw = request(
        address,
        "GET",
        "/api/status?since=0&wait=1e300",
        None,
        Some(TOKEN),
    );
    let client = std::thread::spawn(move || exchange(address, &raw));
    std::thread::sleep(Duration::from_millis(200));
    assert!(!client.is_finished(), "a long poll waits");
    drop(server);
    assert_eq!(client.join().expect("client").0, 200, "stopping ends it");
}

/// One `Host`, a loopback name with a port that is a number, and one
/// `Content-Length` at most.
#[test]
fn a_request_names_its_host_and_length_once() {
    let server = bare_server(TEST_LIMITS);
    let address = server.address();
    let port = address.port();
    let auth = format!("Authorization: Bearer {TOKEN}\r\n");
    for (raw, expected, what) in [
        (
            format!("GET /api/status HTTP/1.1\r\n{auth}\r\n"),
            400,
            "no Host",
        ),
        (
            format!("GET /api/status HTTP/1.1\r\nHost: localhost:\r\n{auth}\r\n"),
            400,
            "an empty port",
        ),
        (
            format!("GET /api/status HTTP/1.1\r\nHost: \r\n{auth}\r\n"),
            400,
            "an empty Host",
        ),
        (
            format!(
                "GET /api/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nHost: evil.example\r\n{auth}\r\n"
            ),
            400,
            "two Host headers",
        ),
        (
            format!(
                "POST /api/seek HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Length: 9\r\nContent-Length: 9\r\n\r\n{{\"to\":12}}"
            ),
            400,
            "two Content-Length headers",
        ),
        (
            format!("GET /api/status HTTP/1.1\r\nHost: evil.example:{port}\r\n{auth}\r\n"),
            403,
            "another host",
        ),
        (
            format!("GET /api/status HTTP/1.1\r\nHost: localhost:{port}\r\n{auth}\r\n"),
            200,
            "localhost with its port",
        ),
        (
            format!(
                "POST /api/seek HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}Content-Length: 9\r\n\r\n{{\"to\":12}}"
            ),
            200,
            "one of each",
        ),
    ] {
        let (status, value) = exchange(address, raw.as_bytes());
        assert_eq!(status, expected, "{what}: {value}");
    }
}
