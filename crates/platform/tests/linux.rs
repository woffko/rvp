//! MPRIS, idle inhibit, the launcher's progress and the screen lock on a
//! private session bus (M8, M17): a `dbus-daemon` of the test's own with no
//! services to start, fake session manager, screen saver and portal
//! services, and a client that reads and calls what a desktop would. The
//! tests pass with a note where `dbus-daemon` is missing.
//!
//! zbus 6 has no blocking API: each call runs in `zbus::block_on`, and the
//! fake services' connections are served by zbus's helper threads between
//! those calls.

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::future::poll_fn;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_core::Stream;
use rvp_app::media::{MediaCommand, MediaState, Status};
use rvp_platform::{Desktop, Options};
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::object_server::SignalEmitter;
use zbus::{
    Connection, MatchRule, MessageStream, ObjectPath, OwnedObjectPath, OwnedVariant, Proxy,
    Variant, block_on, connection, interface, message,
};

const PATH: &str = "/org/mpris/MediaPlayer2";
const ROOT: &str = "org.mpris.MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";

/// A private session bus that starts no services.
struct Bus {
    daemon: Child,
    address: String,
    dir: PathBuf,
}

impl Bus {
    fn start(name: &str) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("rvp-bus-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        let config = dir.join("bus.conf");
        std::fs::write(
            &config,
            format!(
                "<!DOCTYPE busconfig PUBLIC \"-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN\" \
                 \"http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd\">\n\
                 <busconfig><type>session</type><listen>unix:path={}</listen>\
                 <auth>EXTERNAL</auth><policy context=\"default\"><allow send_destination=\"*\" \
                 eavesdrop=\"true\"/><allow eavesdrop=\"true\"/><allow own=\"*\"/></policy></busconfig>\n",
                dir.join("bus").display()
            ),
        )
        .ok()?;
        let Ok(mut daemon) = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config.display()))
            .args(["--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
        else {
            eprintln!("note: dbus-daemon is not installed; the bus test checks nothing");
            return None;
        };
        let mut address = String::new();
        BufReader::new(daemon.stdout.take()?)
            .read_line(&mut address)
            .ok()?;
        Some(Self {
            daemon,
            address: address.trim().to_owned(),
            dir,
        })
    }

    fn connect(&self) -> Connection {
        block_on(connection::Builder::address(self.address.as_str()).build())
            .expect("a client on the private bus")
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn options() -> Options {
    Options {
        desktop_entry: "rvp".to_owned(),
        inhibit_reason: "Playing video".to_owned(),
    }
}

/// The next item of a stream (zbus 6 hands out streams only).
async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    poll_fn(|context| Pin::new(&mut *stream).poll_next(context)).await
}

/// A desktop that ignores the screen lock.
fn start(bus: &Bus, on_command: impl Fn(MediaCommand) + Send + Sync + 'static) -> Desktop {
    Desktop::start_at(&bus.address, options(), on_command, |_| {}).expect("desktop")
}

fn wait_for(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn playing() -> MediaState {
    MediaState {
        status: Status::Playing,
        track: 3,
        title: "[2/5] Film.mkv".to_owned(),
        url: Some("file:///films/Film.mkv".to_owned()),
        length: Some(120.0),
        position: 10.0,
        volume: 0.8,
        rate: 1.0,
        can_next: true,
        can_previous: true,
        can_seek: true,
        can_play: true,
    }
}

fn get(properties: &PropertiesProxy<'_>, interface: &'static str, name: &str) -> OwnedVariant {
    block_on(properties.get(InterfaceName::from_static_str_unchecked(interface), name))
        .unwrap_or_else(|e| panic!("{interface}.{name}: {e}"))
}

fn text(value: &OwnedVariant) -> String {
    String::try_from(value.clone()).expect("a string")
}

#[test]
fn mpris_shows_playback_and_takes_commands() {
    let Some(bus) = Bus::start("mpris") else {
        return;
    };
    let commands = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&commands);
    let desktop = start(&bus, move |command| {
        sink.lock().expect("commands").push(command);
    });
    assert_eq!(desktop.bus_name(), "org.mpris.MediaPlayer2.rvp");

    let client = bus.connect();
    let properties = block_on(
        PropertiesProxy::builder(&client)
            .destination("org.mpris.MediaPlayer2.rvp")
            .path(PATH)
            .build(),
    )
    .expect("a properties proxy");
    let player = block_on(Proxy::new(
        &client,
        "org.mpris.MediaPlayer2.rvp",
        PATH,
        PLAYER,
    ))
    .expect("a player proxy");
    check_identity(&properties);
    check_state(&desktop, &properties);
    check_seeked(&desktop, &player);
    check_commands(&client, &properties, &player, &commands);

    // A second RVP takes MPRIS's instance name.
    let second = start(&bus, |_| {});
    assert_eq!(
        second.bus_name(),
        format!("org.mpris.MediaPlayer2.rvp.instance{}", std::process::id())
    );
}

/// `org.mpris.MediaPlayer2`'s properties, and nothing playing yet.
fn check_identity(properties: &PropertiesProxy<'_>) {
    assert_eq!(text(&get(properties, ROOT, "Identity")), "RVP");
    assert_eq!(text(&get(properties, ROOT, "DesktopEntry")), "rvp");
    let mime: Vec<String> = get(properties, ROOT, "SupportedMimeTypes")
        .try_into()
        .expect("strings");
    assert!(mime.iter().any(|m| m == "video/x-matroska"), "{mime:?}");
    assert_eq!(text(&get(properties, PLAYER, "PlaybackStatus")), "Stopped");
}

/// A new state is announced and reads back.
fn check_state(desktop: &Desktop, properties: &PropertiesProxy<'static>) {
    let mut signals = block_on(properties.receive_properties_changed()).expect("PropertiesChanged");
    let (sender, announced) = mpsc::channel();
    std::thread::spawn(move || {
        block_on(async {
            while let Some(signal) = next(&mut signals).await {
                if let Ok(args) = signal.args() {
                    let names: Vec<String> = args
                        .changed_properties()
                        .keys()
                        .map(|k| (*k).to_owned())
                        .collect();
                    if sender.send(names).is_err() {
                        break;
                    }
                }
            }
        });
    });
    desktop.update(playing());
    let names = announced
        .recv_timeout(Duration::from_secs(5))
        .expect("a PropertiesChanged signal");
    assert!(names.iter().any(|n| n == "PlaybackStatus"), "{names:?}");
    wait_for("Playing", || {
        text(&get(properties, PLAYER, "PlaybackStatus")) == "Playing"
    });
    let metadata: HashMap<String, OwnedVariant> = get(properties, PLAYER, "Metadata")
        .try_into()
        .expect("a{sv}");
    let track: OwnedObjectPath = metadata["mpris:trackid"].clone().try_into().expect("o");
    assert_eq!(track.as_str(), "/org/rvp/track/3");
    assert_eq!(text(&metadata["xesam:title"]), "[2/5] Film.mkv");
    assert_eq!(text(&metadata["xesam:url"]), "file:///films/Film.mkv");
    assert_eq!(
        i64::try_from(metadata["mpris:length"].clone()).expect("x"),
        120_000_000
    );
    assert_eq!(
        i64::try_from(get(properties, PLAYER, "Position")).expect("x"),
        10_000_000
    );
    assert!((f64::try_from(get(properties, PLAYER, "Volume")).expect("d") - 0.8).abs() < 1e-9);
    assert!(bool::try_from(get(properties, PLAYER, "CanGoNext")).expect("b"));
}

/// `Seeked` reaches the client.
fn check_seeked(desktop: &Desktop, player: &Proxy<'static>) {
    let mut seeks = block_on(player.receive_signal("Seeked")).expect("Seeked");
    let (sender, positions) = mpsc::channel();
    std::thread::spawn(move || {
        block_on(async {
            while let Some(message) = next(&mut seeks).await {
                if let Ok(position) = message.body().deserialize::<i64>() {
                    let _ = sender.send(position);
                    break;
                }
            }
        });
    });
    desktop.seeked(42.5);
    assert_eq!(
        positions.recv_timeout(Duration::from_secs(5)),
        Ok(42_500_000)
    );
}

/// Calls become commands; a position for another track does nothing.
fn check_commands(
    client: &Connection,
    properties: &PropertiesProxy<'_>,
    player: &Proxy<'_>,
    commands: &Mutex<Vec<MediaCommand>>,
) {
    block_on(player.call_method("PlayPause", &())).expect("PlayPause");
    block_on(player.call_method("Next", &())).expect("Next");
    block_on(player.call_method("Seek", &(5_000_000i64,))).expect("Seek");
    let this = ObjectPath::try_from("/org/rvp/track/3").expect("path");
    let other = ObjectPath::try_from("/org/rvp/track/9").expect("path");
    block_on(player.call_method("SetPosition", &(this, 30_000_000i64))).expect("SetPosition");
    block_on(player.call_method("SetPosition", &(other, 5_000_000i64)))
        .expect("SetPosition, another track");
    block_on(player.call_method("OpenUri", &("file:///films/Other.mkv",))).expect("OpenUri");
    let interface = InterfaceName::from_static_str_unchecked(PLAYER);
    block_on(properties.set(interface.clone(), "Volume", &Variant::from(0.5))).expect("Volume");
    block_on(properties.set(interface, "Rate", &Variant::from(0.0))).expect("Rate 0");
    block_on(async {
        let root = Proxy::new(client, "org.mpris.MediaPlayer2.rvp", PATH, ROOT).await?;
        root.call_method("Raise", &()).await
    })
    .expect("Raise");
    let expected = vec![
        MediaCommand::PlayPause,
        MediaCommand::Next,
        MediaCommand::SeekBy(5.0),
        MediaCommand::SetPosition {
            track: 3,
            seconds: 30.0,
        },
        MediaCommand::OpenUri("file:///films/Other.mkv".to_owned()),
        MediaCommand::SetVolume(0.5),
        // MPRIS: a rate of 0 acts as Pause.
        MediaCommand::Pause,
        MediaCommand::Raise,
    ];
    wait_for("the commands", || {
        commands.lock().expect("commands").len() >= expected.len()
    });
    assert_eq!(*commands.lock().expect("commands"), expected);
}

/// GNOME's session manager's `Inhibit`, recording the calls it gets.
struct FakeSessionManager {
    calls: Arc<Mutex<Vec<String>>>,
}

#[interface(name = "org.gnome.SessionManager")]
impl FakeSessionManager {
    fn inhibit(&self, app_id: &str, toplevel_xid: u32, reason: &str, flags: u32) -> u32 {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Inhibit {app_id} {toplevel_xid} {flags}: {reason}"));
        5
    }

    fn uninhibit(&self, cookie: u32) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Uninhibit {cookie}"));
    }
}

/// A screen saver service that records the calls it gets.
struct FakeScreenSaver {
    calls: Arc<Mutex<Vec<String>>>,
}

#[interface(name = "org.freedesktop.ScreenSaver")]
impl FakeScreenSaver {
    fn inhibit(&self, application: &str, reason: &str) -> u32 {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Inhibit {application}: {reason}"));
        7
    }

    fn un_inhibit(&self, cookie: u32) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("UnInhibit {cookie}"));
    }
}

/// The portal's `Inhibit` that records its calls and hands out one request.
struct FakePortal {
    calls: Arc<Mutex<Vec<String>>>,
}

const REQUEST: &str = "/org/freedesktop/portal/desktop/request/1_1/rvp";

#[interface(name = "org.freedesktop.portal.Inhibit")]
impl FakePortal {
    // zbus hands method arguments over by value.
    #[allow(clippy::needless_pass_by_value)]
    fn inhibit(
        &self,
        window: &str,
        flags: u32,
        options: HashMap<String, OwnedVariant>,
    ) -> OwnedObjectPath {
        let reason = options
            .get("reason")
            .and_then(|r| String::try_from(r.clone()).ok())
            .unwrap_or_default();
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Inhibit '{window}' {flags}: {reason}"));
        OwnedObjectPath::try_from(REQUEST).expect("path")
    }
}

/// The portal's request object for an inhibit.
struct FakeRequest {
    calls: Arc<Mutex<Vec<String>>>,
}

#[interface(name = "org.freedesktop.portal.Request")]
impl FakeRequest {
    fn close(&self) {
        self.calls.lock().expect("calls").push("Close".to_owned());
    }
}

/// Serves `object` at `path` under the bus name `name` until dropped.
fn serve(
    bus: &Bus,
    name: &str,
    path: &str,
    object: impl zbus::object_server::Interface,
) -> Connection {
    block_on(
        connection::Builder::address(bus.address.as_str())
            .name(name.to_owned())
            .serve_at(path.to_owned(), object)
            .build(),
    )
    .unwrap_or_else(|e| panic!("the fake {name}: {e}"))
}

/// Inhibits twice and lets go once, then drops the [`Desktop`].
fn inhibit_and_release(bus: &Bus) {
    let desktop = start(bus, |_| {});
    desktop.inhibit(true);
    desktop.inhibit(true);
    desktop.inhibit(false);
    desktop.inhibit(true);
    drop(desktop);
}

#[test]
fn inhibit_asks_gnome_then_the_screen_saver_then_the_portal() {
    let Some(bus) = Bus::start("inhibit") else {
        return;
    };
    let record = || Arc::new(Mutex::new(Vec::<String>::new()));
    let (portal_calls, saver_calls, session_calls) = (record(), record(), record());
    let calls = |log: &Arc<Mutex<Vec<String>>>| log.lock().expect("calls").clone();

    // The portal alone: its Inhibit (idle), closed when RVP lets go or ends.
    let portal = serve(
        &bus,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        FakePortal {
            calls: Arc::clone(&portal_calls),
        },
    );
    block_on(portal.object_server().at(
        REQUEST,
        FakeRequest {
            calls: Arc::clone(&portal_calls),
        },
    ))
    .expect("the request object");
    inhibit_and_release(&bus);
    wait_for("the portal's calls", || calls(&portal_calls).len() >= 4);
    assert_eq!(
        calls(&portal_calls),
        [
            "Inhibit '' 8: Playing video",
            "Close",
            "Inhibit '' 8: Playing video",
            "Close"
        ]
    );

    // A screen saver service comes before the portal.
    let _saver = serve(
        &bus,
        "org.freedesktop.ScreenSaver",
        "/org/freedesktop/ScreenSaver",
        FakeScreenSaver {
            calls: Arc::clone(&saver_calls),
        },
    );
    inhibit_and_release(&bus);
    wait_for("the screen saver's calls", || {
        calls(&saver_calls).len() >= 4
    });
    assert_eq!(
        calls(&saver_calls),
        [
            "Inhibit RVP: Playing video",
            "UnInhibit 7",
            "Inhibit RVP: Playing video",
            "UnInhibit 7"
        ]
    );

    // GNOME's session manager comes first, asked with RVP's app ID.
    let _session = serve(
        &bus,
        "org.gnome.SessionManager",
        "/org/gnome/SessionManager",
        FakeSessionManager {
            calls: Arc::clone(&session_calls),
        },
    );
    inhibit_and_release(&bus);
    wait_for("the session manager's calls", || {
        calls(&session_calls).len() >= 4
    });
    assert_eq!(
        calls(&session_calls),
        [
            "Inhibit rvp 0 8: Playing video",
            "Uninhibit 5",
            "Inhibit rvp 0 8: Playing video",
            "Uninhibit 5"
        ]
    );
    // Neither of the others was asked again.
    assert_eq!(calls(&portal_calls).len(), 4);
    assert_eq!(calls(&saver_calls).len(), 4);
}

/// A screen saver that only announces its state.
struct FakeLockingSaver;

#[interface(name = "org.gnome.ScreenSaver")]
impl FakeLockingSaver {
    #[zbus(signal)]
    async fn active_changed(emitter: &SignalEmitter<'_>, active: bool) -> zbus::Result<()>;
}

#[test]
fn the_screen_lock_reaches_rvp_once_per_change() {
    let Some(bus) = Bus::start("lock") else {
        return;
    };
    let (sender, locks) = mpsc::channel();
    let sender = Mutex::new(sender);
    let _desktop = Desktop::start_at(
        &bus.address,
        options(),
        |_| {},
        move |locked| {
            let _ = sender.lock().expect("sender").send(locked);
        },
    )
    .expect("desktop");
    let saver = serve(
        &bus,
        "org.gnome.ScreenSaver",
        "/org/gnome/ScreenSaver",
        FakeLockingSaver,
    );
    let emit = |active: bool| {
        block_on(async {
            let emitter = SignalEmitter::new(&saver, "/org/gnome/ScreenSaver")?;
            FakeLockingSaver::active_changed(&emitter, active).await
        })
        .expect("ActiveChanged");
    };
    // The same state twice (GNOME and freedesktop.org's both may say so)
    // reaches RVP once.
    emit(true);
    emit(true);
    emit(false);
    let wait = Duration::from_secs(5);
    assert_eq!(locks.recv_timeout(wait), Ok(true));
    assert_eq!(locks.recv_timeout(wait), Ok(false));
    assert!(locks.recv_timeout(Duration::from_millis(300)).is_err());
}

#[test]
fn the_launcher_shows_the_progress_while_a_file_is_loaded() {
    let Some(bus) = Bus::start("launcher") else {
        return;
    };
    let client = bus.connect();
    let rule = MatchRule::builder()
        .msg_type(message::Type::Signal)
        .interface("com.canonical.Unity.LauncherEntry")
        .member("Update")
        .build()
        .expect("a match rule");
    let mut updates =
        block_on(MessageStream::for_match_rule(rule, &client, None)).expect("Update signals");
    let (sender, received) = mpsc::channel();
    std::thread::spawn(move || {
        block_on(async {
            while let Some(Ok(signal)) = next(&mut updates).await {
                let Ok((uri, properties)) = signal
                    .body()
                    .deserialize::<(String, HashMap<String, OwnedVariant>)>()
                else {
                    continue;
                };
                let progress = properties
                    .get("progress")
                    .and_then(|v| f64::try_from(v.clone()).ok());
                let visible = properties
                    .get("progress-visible")
                    .and_then(|v| bool::try_from(v.clone()).ok());
                if sender.send((uri, progress, visible)).is_err() {
                    break;
                }
            }
        });
    });
    let desktop = start(&bus, |_| {});
    desktop.progress(1, 0.25);
    // Below half a percent: not worth a signal.
    desktop.progress(1, 0.252);
    desktop.progress(2, 0.5);
    desktop.progress(0, -1.0);
    let wait = Duration::from_secs(5);
    let uri = "application://rvp.desktop".to_owned();
    assert_eq!(
        received.recv_timeout(wait),
        Ok((uri.clone(), Some(0.25), Some(true)))
    );
    assert_eq!(
        received.recv_timeout(wait),
        Ok((uri.clone(), Some(0.5), Some(true)))
    );
    assert_eq!(
        received.recv_timeout(wait),
        Ok((uri.clone(), Some(0.5), Some(false)))
    );
    // Shown again, then hidden when RVP ends.
    desktop.progress(1, 0.75);
    assert_eq!(
        received.recv_timeout(wait),
        Ok((uri.clone(), Some(0.75), Some(true)))
    );
    drop(desktop);
    assert_eq!(
        received.recv_timeout(wait),
        Ok((uri, Some(0.75), Some(false)))
    );
}
