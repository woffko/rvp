//! MPRIS and idle inhibit on a private session bus (M8): a `dbus-daemon` of
//! the test's own with no services to start, fake portal and screen saver
//! services, and a client that reads and calls what a desktop would. The
//! tests pass with a note where `dbus-daemon` is missing.

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rvp_app::media::{MediaCommand, MediaState, Status};
use rvp_platform::{Desktop, Options};
use zbus::blocking::{Connection, connection, fdo::PropertiesProxy};
use zbus::interface;
use zbus::names::InterfaceName;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

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
        connection::Builder::address(self.address.as_str())
            .and_then(connection::Builder::build)
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

fn get(properties: &PropertiesProxy<'_>, interface: &'static str, name: &str) -> OwnedValue {
    properties
        .get(InterfaceName::from_static_str_unchecked(interface), name)
        .unwrap_or_else(|e| panic!("{interface}.{name}: {e}"))
}

fn text(value: &OwnedValue) -> String {
    String::try_from(value.clone()).expect("a string")
}

#[test]
fn mpris_shows_playback_and_takes_commands() {
    let Some(bus) = Bus::start("mpris") else {
        return;
    };
    let commands = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&commands);
    let desktop = Desktop::start_at(&bus.address, options(), move |command| {
        sink.lock().expect("commands").push(command);
    })
    .expect("MPRIS on the private bus");
    assert_eq!(desktop.bus_name(), "org.mpris.MediaPlayer2.rvp");

    let client = bus.connect();
    let properties = PropertiesProxy::builder(&client)
        .destination("org.mpris.MediaPlayer2.rvp")
        .and_then(|b| b.path(PATH))
        .and_then(zbus::blocking::proxy::Builder::build)
        .expect("a properties proxy");
    let player = zbus::blocking::Proxy::new(&client, "org.mpris.MediaPlayer2.rvp", PATH, PLAYER)
        .expect("a player proxy");
    check_identity(&properties);
    check_state(&desktop, &properties);
    check_seeked(&desktop, &player);
    check_commands(&client, &properties, &player, &commands);

    // A second RVP takes MPRIS's instance name.
    let second = Desktop::start_at(&bus.address, options(), |_| {}).expect("a second RVP");
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
fn check_state(desktop: &Desktop, properties: &PropertiesProxy<'_>) {
    let signals = properties
        .receive_properties_changed()
        .expect("PropertiesChanged");
    let (sender, announced) = mpsc::channel();
    std::thread::spawn(move || {
        for signal in signals {
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
    desktop.update(playing());
    let names = announced
        .recv_timeout(Duration::from_secs(5))
        .expect("a PropertiesChanged signal");
    assert!(names.iter().any(|n| n == "PlaybackStatus"), "{names:?}");
    wait_for("Playing", || {
        text(&get(properties, PLAYER, "PlaybackStatus")) == "Playing"
    });
    let metadata: HashMap<String, OwnedValue> = get(properties, PLAYER, "Metadata")
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
fn check_seeked(desktop: &Desktop, player: &zbus::blocking::Proxy<'_>) {
    let seeks = player.receive_signal("Seeked").expect("Seeked");
    let (sender, positions) = mpsc::channel();
    std::thread::spawn(move || {
        for message in seeks {
            if let Ok(position) = message.body().deserialize::<i64>() {
                let _ = sender.send(position);
                break;
            }
        }
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
    player: &zbus::blocking::Proxy<'_>,
    commands: &Mutex<Vec<MediaCommand>>,
) {
    player.call_method("PlayPause", &()).expect("PlayPause");
    player.call_method("Next", &()).expect("Next");
    player.call_method("Seek", &(5_000_000i64,)).expect("Seek");
    let this = ObjectPath::try_from("/org/rvp/track/3").expect("path");
    let other = ObjectPath::try_from("/org/rvp/track/9").expect("path");
    player
        .call_method("SetPosition", &(this, 30_000_000i64))
        .expect("SetPosition");
    player
        .call_method("SetPosition", &(other, 5_000_000i64))
        .expect("SetPosition, another track");
    player
        .call_method("OpenUri", &("file:///films/Other.mkv",))
        .expect("OpenUri");
    let interface = InterfaceName::from_static_str_unchecked(PLAYER);
    properties
        .set(interface.clone(), "Volume", Value::from(0.5))
        .expect("Volume");
    properties
        .set(interface, "Rate", Value::from(0.0))
        .expect("Rate 0");
    zbus::blocking::Proxy::new(client, "org.mpris.MediaPlayer2.rvp", PATH, ROOT)
        .and_then(|root| root.call_method("Raise", &()))
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
        options: HashMap<String, OwnedValue>,
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

#[test]
fn inhibit_asks_the_portal_or_else_the_screen_saver() {
    let Some(bus) = Bus::start("inhibit") else {
        return;
    };
    // No portal: the screen saver's interface.
    let saver_calls = Arc::new(Mutex::new(Vec::new()));
    let _saver = connection::Builder::address(bus.address.as_str())
        .and_then(|b| b.name("org.freedesktop.ScreenSaver"))
        .and_then(|b| {
            b.serve_at(
                "/org/freedesktop/ScreenSaver",
                FakeScreenSaver {
                    calls: Arc::clone(&saver_calls),
                },
            )
        })
        .and_then(connection::Builder::build)
        .expect("the fake screen saver");
    let desktop = Desktop::start_at(&bus.address, options(), |_| {}).expect("desktop");
    desktop.inhibit(true);
    desktop.inhibit(true);
    desktop.inhibit(false);
    let calls = || saver_calls.lock().expect("calls").clone();
    wait_for("the screen saver's calls", || calls().len() >= 2);
    assert_eq!(calls(), ["Inhibit RVP: Playing video", "UnInhibit 7"]);
    drop(desktop);

    // With a portal: its Inhibit (idle), closed when RVP lets go or ends.
    let portal_calls = Arc::new(Mutex::new(Vec::new()));
    let _portal = connection::Builder::address(bus.address.as_str())
        .and_then(|b| b.name("org.freedesktop.portal.Desktop"))
        .and_then(|b| {
            b.serve_at(
                "/org/freedesktop/portal/desktop",
                FakePortal {
                    calls: Arc::clone(&portal_calls),
                },
            )
        })
        .and_then(|b| {
            b.serve_at(
                REQUEST,
                FakeRequest {
                    calls: Arc::clone(&portal_calls),
                },
            )
        })
        .and_then(connection::Builder::build)
        .expect("the fake portal");
    let desktop = Desktop::start_at(&bus.address, options(), |_| {}).expect("desktop");
    desktop.inhibit(true);
    let calls = || portal_calls.lock().expect("calls").clone();
    wait_for("the portal's Inhibit", || !calls().is_empty());
    drop(desktop);
    wait_for("the request closed", || calls().len() >= 2);
    assert_eq!(calls(), ["Inhibit '' 8: Playing video", "Close"]);
    // The screen saver was not asked again.
    assert_eq!(saver_calls.lock().expect("calls").len(), 2);
}
