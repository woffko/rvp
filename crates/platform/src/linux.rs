//! MPRIS and idle inhibit over the session D-Bus.

use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};

use rvp_app::media::{MediaCommand, MediaState, RATE_RANGE, Status};
use zbus::blocking::{Connection, connection};
use zbus::fdo::RequestNameFlags;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

/// MPRIS's object path.
const PATH: &str = "/org/mpris/MediaPlayer2";
/// RVP's bus name; a second RVP adds `.instance<pid>`, as MPRIS asks.
const NAME: &str = "org.mpris.MediaPlayer2.rvp";
/// The track ID MPRIS reserves for "no track".
const NO_TRACK: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";
/// The portal's `Inhibit` flag for idle (the screen saver and blanking).
const INHIBIT_IDLE: u32 = 8;
/// MPRIS counts time in microseconds.
const MICROS: f64 = 1_000_000.0;

type OnCommand = Arc<dyn Fn(MediaCommand) + Send + Sync>;

/// How RVP shows itself on the bus.
#[derive(Debug, Clone)]
pub struct Options {
    /// The `.desktop` file's name without its extension (`rvp`).
    pub desktop_entry: String,
    /// Why the screen stays on, for the desktop's list of inhibitors.
    pub inhibit_reason: String,
}

/// RVP on the desktop's bus: MPRIS and idle inhibit. Dropping it ends its
/// thread, which releases an inhibit it holds.
pub struct Desktop {
    sender: Sender<Message>,
    bus_name: String,
}

enum Message {
    State(Box<MediaState>),
    Seeked(f64),
    Inhibit(bool),
}

impl Desktop {
    /// Registers on the session bus. `on_command` runs on zbus's thread for
    /// each MPRIS call.
    ///
    /// # Errors
    /// Fails without a session bus or when the names cannot be taken.
    pub fn start(
        options: Options,
        on_command: impl Fn(MediaCommand) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let on_command: OnCommand = Arc::new(on_command);
        Self::start_with(connection::Builder::session, options, &on_command)
    }

    /// Registers on the bus at `address` (a private bus for tests).
    ///
    /// # Errors
    /// Fails when the bus cannot be reached or the names cannot be taken.
    pub fn start_at(
        address: &str,
        options: Options,
        on_command: impl Fn(MediaCommand) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let address = address.to_owned();
        let on_command: OnCommand = Arc::new(on_command);
        Self::start_with(
            move || connection::Builder::address(address.as_str()),
            options,
            &on_command,
        )
    }

    fn start_with(
        builder: impl Fn() -> zbus::Result<connection::Builder<'static>>,
        options: Options,
        on_command: &OnCommand,
    ) -> Result<Self, String> {
        let state = Arc::new(Mutex::new(stopped()));
        let error = |e: zbus::Error| format!("MPRIS on the session bus: {e}");
        let connection = builder()
            .and_then(|builder| {
                builder.serve_at(
                    PATH,
                    Root {
                        desktop_entry: options.desktop_entry.clone(),
                        on_command: Arc::clone(on_command),
                    },
                )
            })
            .and_then(|builder| {
                builder.serve_at(
                    PATH,
                    Player {
                        state: Arc::clone(&state),
                        on_command: Arc::clone(on_command),
                    },
                )
            })
            .and_then(connection::Builder::build)
            .map_err(error)?;
        // Without queueing: a name another RVP holds fails at once (zbus's
        // builder would queue the request and report success).
        let take = |name: &str| {
            connection.request_name_with_flags(name.to_owned(), RequestNameFlags::DoNotQueue.into())
        };
        let instance = format!("{NAME}.instance{}", std::process::id());
        let bus_name = if take(NAME).is_ok() {
            NAME.to_owned()
        } else {
            take(&instance).map_err(error)?;
            instance
        };
        let (sender, receiver) = mpsc::channel();
        let reason = options.inhibit_reason;
        std::thread::Builder::new()
            .name("rvp-desktop".to_owned())
            .spawn(move || run(&connection, &state, &receiver, &reason))
            .map_err(|e| format!("desktop thread: {e}"))?;
        Ok(Self { sender, bus_name })
    }

    /// The bus name RVP took.
    #[must_use]
    pub fn bus_name(&self) -> &str {
        &self.bus_name
    }

    /// The playback state now; MPRIS announces what changed.
    pub fn update(&self, state: MediaState) {
        drop(self.sender.send(Message::State(Box::new(state))));
    }

    /// Playback jumped to `position` seconds (MPRIS's `Seeked`).
    pub fn seeked(&self, position: f64) {
        drop(self.sender.send(Message::Seeked(position)));
    }

    /// Keeps the screen on (video plays) or lets it sleep again.
    pub fn inhibit(&self, on: bool) {
        drop(self.sender.send(Message::Inhibit(on)));
    }
}

fn stopped() -> MediaState {
    MediaState {
        status: Status::Stopped,
        track: 0,
        title: String::new(),
        url: None,
        length: None,
        position: 0.0,
        volume: 1.0,
        rate: 1.0,
        can_next: false,
        can_previous: false,
        can_seek: false,
        can_play: false,
    }
}

/// Microseconds, as MPRIS counts.
fn micros(seconds: f64) -> i64 {
    // Within i64 for any real length; `as` saturates.
    #[allow(clippy::cast_possible_truncation)]
    let micros = (seconds * MICROS).round() as i64;
    micros
}

fn seconds(micros: i64) -> f64 {
    // Exact below 2^53 microseconds (285 years).
    #[allow(clippy::cast_precision_loss)]
    let micros = micros as f64;
    micros / MICROS
}

/// The object path naming a track.
fn track_path(state: &MediaState) -> String {
    if state.status == Status::Stopped {
        NO_TRACK.to_owned()
    } else {
        format!("/org/rvp/track/{}", state.track)
    }
}

fn lock(state: &Mutex<MediaState>) -> std::sync::MutexGuard<'_, MediaState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `org.mpris.MediaPlayer2`.
struct Root {
    desktop_entry: String,
    on_command: OnCommand,
}

// zbus calls interface methods on `&self`, constant properties too.
#[allow(clippy::unused_self)]
#[interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        (self.on_command)(MediaCommand::Raise);
    }

    fn quit(&self) {
        (self.on_command)(MediaCommand::Quit);
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_raise(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn identity(&self) -> String {
        "RVP".to_owned()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn desktop_entry(&self) -> String {
        self.desktop_entry.clone()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn supported_uri_schemes(&self) -> Vec<String> {
        ["file", "http", "https"].map(str::to_owned).to_vec()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn supported_mime_types(&self) -> Vec<String> {
        rvp_app::associations::mime_types()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

/// `org.mpris.MediaPlayer2.Player`.
struct Player {
    state: Arc<Mutex<MediaState>>,
    on_command: OnCommand,
}

impl Player {
    fn snapshot(&self) -> MediaState {
        lock(&self.state).clone()
    }

    fn send(&self, command: MediaCommand) {
        (self.on_command)(command);
    }
}

// zbus calls interface methods on `&self`, constant properties too.
#[allow(clippy::unused_self)]
#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    fn next(&self) {
        self.send(MediaCommand::Next);
    }

    fn previous(&self) {
        self.send(MediaCommand::Previous);
    }

    fn pause(&self) {
        self.send(MediaCommand::Pause);
    }

    fn play_pause(&self) {
        self.send(MediaCommand::PlayPause);
    }

    fn stop(&self) {
        self.send(MediaCommand::Stop);
    }

    fn play(&self) {
        self.send(MediaCommand::Play);
    }

    fn seek(&self, offset: i64) {
        self.send(MediaCommand::SeekBy(seconds(offset)));
    }

    // zbus hands method arguments over by value.
    #[allow(clippy::needless_pass_by_value)]
    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        let state = self.snapshot();
        // MPRIS: ignored for another track or a position out of range.
        let in_range = position >= 0
            && state
                .length
                .is_none_or(|length| seconds(position) <= length);
        if track_id.as_str() == track_path(&state) && in_range {
            self.send(MediaCommand::SetPosition {
                track: state.track,
                seconds: seconds(position),
            });
        }
    }

    fn open_uri(&self, uri: &str) {
        self.send(MediaCommand::OpenUri(uri.to_owned()));
    }

    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> String {
        match self.snapshot().status {
            Status::Playing => "Playing",
            Status::Paused => "Paused",
            Status::Stopped => "Stopped",
        }
        .to_owned()
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        self.snapshot().rate
    }

    #[zbus(property)]
    fn set_rate(&self, rate: f64) {
        // MPRIS: a rate of 0 acts as Pause.
        if rate > 0.0 {
            self.send(MediaCommand::SetRate(rate));
        } else {
            self.send(MediaCommand::Pause);
        }
    }

    /// A `HashMap` because zbus takes a dictionary property as one; zvariant
    /// sends its entries sorted (its `Dict` is a `BTreeMap`), so the order on
    /// the bus does not depend on the hash.
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let state = self.snapshot();
        let mut metadata = HashMap::new();
        let mut put = |key: &str, value: Value<'_>| {
            if let Ok(value) = OwnedValue::try_from(value) {
                metadata.insert(key.to_owned(), value);
            }
        };
        if let Ok(path) = OwnedObjectPath::try_from(track_path(&state)) {
            put("mpris:trackid", Value::from(path));
        }
        if state.status != Status::Stopped {
            put("xesam:title", Value::from(state.title.clone()));
            if let Some(url) = &state.url {
                put("xesam:url", Value::from(url.clone()));
            }
            if let Some(length) = state.length {
                put("mpris:length", Value::from(micros(length)));
            }
        }
        metadata
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.snapshot().volume
    }

    #[zbus(property)]
    fn set_volume(&self, volume: f64) {
        self.send(MediaCommand::SetVolume(volume.max(0.0)));
    }

    /// Asked for, never announced (MPRIS: `Seeked` reports jumps).
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        micros(self.snapshot().position)
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn minimum_rate(&self) -> f64 {
        RATE_RANGE.0
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn maximum_rate(&self) -> f64 {
        RATE_RANGE.1
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.snapshot().can_next
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.snapshot().can_previous
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.snapshot().can_play
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.snapshot().status != Status::Stopped
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.snapshot().can_seek
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> bool {
        true
    }
}

/// An inhibit RVP holds.
enum Inhibit {
    /// The portal's request object; closing it ends the inhibit.
    Portal(OwnedObjectPath),
    /// `org.freedesktop.ScreenSaver`'s cookie.
    ScreenSaver(u32),
}

/// The desktop thread: state changes become MPRIS signals, inhibit
/// requests become bus calls. Ends when the [`Desktop`] is dropped.
fn run(
    connection: &Connection,
    shared: &Mutex<MediaState>,
    receiver: &Receiver<Message>,
    reason: &str,
) {
    let player = connection.object_server().interface::<_, Player>(PATH).ok();
    let mut held: Option<Inhibit> = None;
    for message in receiver {
        match message {
            Message::State(state) => {
                let old = std::mem::replace(&mut *lock(shared), (*state).clone());
                if let Some(player) = &player {
                    announce(player, &old, &state);
                }
            }
            Message::Seeked(position) => {
                if let Some(player) = &player {
                    drop(zbus::block_on(Player::seeked(
                        player.signal_emitter(),
                        micros(position),
                    )));
                }
            }
            Message::Inhibit(true) if held.is_none() => held = inhibit(connection, reason),
            Message::Inhibit(false) => {
                if let Some(inhibit) = held.take() {
                    release(connection, inhibit);
                }
            }
            Message::Inhibit(true) => {}
        }
    }
    if let Some(inhibit) = held.take() {
        release(connection, inhibit);
    }
}

/// Emits `PropertiesChanged` for what differs between two states.
fn announce(
    player: &zbus::blocking::object_server::InterfaceRef<Player>,
    old: &MediaState,
    new: &MediaState,
) {
    let iface = player.get();
    let emitter = player.signal_emitter();
    // A signal that cannot be sent (the bus went away) is not retried.
    if old.status != new.status {
        drop(zbus::block_on(iface.playback_status_changed(emitter)));
        drop(zbus::block_on(iface.can_pause_changed(emitter)));
    }
    if (old.status, old.track, &old.title, &old.url, old.length)
        != (new.status, new.track, &new.title, &new.url, new.length)
    {
        drop(zbus::block_on(iface.metadata_changed(emitter)));
    }
    if (old.volume - new.volume).abs() > f64::EPSILON {
        drop(zbus::block_on(iface.volume_changed(emitter)));
    }
    if (old.rate - new.rate).abs() > f64::EPSILON {
        drop(zbus::block_on(iface.rate_changed(emitter)));
    }
    if old.can_next != new.can_next {
        drop(zbus::block_on(iface.can_go_next_changed(emitter)));
    }
    if old.can_previous != new.can_previous {
        drop(zbus::block_on(iface.can_go_previous_changed(emitter)));
    }
    if old.can_play != new.can_play {
        drop(zbus::block_on(iface.can_play_changed(emitter)));
    }
    if old.can_seek != new.can_seek {
        drop(zbus::block_on(iface.can_seek_changed(emitter)));
    }
}

/// Asks the portal, then `org.freedesktop.ScreenSaver`, to keep the screen
/// on; `None` when neither answers.
fn inhibit(connection: &Connection, reason: &str) -> Option<Inhibit> {
    let options = BTreeMap::from([("reason", Value::from(reason))]);
    if let Ok(reply) = connection.call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.portal.Inhibit"),
        "Inhibit",
        &("", INHIBIT_IDLE, options),
    ) && let Ok(handle) = reply.body().deserialize::<OwnedObjectPath>()
    {
        return Some(Inhibit::Portal(handle));
    }
    let reply = connection
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "Inhibit",
            &("RVP", reason),
        )
        .ok()?;
    reply
        .body()
        .deserialize::<u32>()
        .ok()
        .map(Inhibit::ScreenSaver)
}

fn release(connection: &Connection, inhibit: Inhibit) {
    // A release that fails ends with RVP anyway: the bus drops what a
    // client that leaves held.
    match inhibit {
        Inhibit::Portal(handle) => drop(connection.call_method(
            Some("org.freedesktop.portal.Desktop"),
            handle.as_str(),
            Some("org.freedesktop.portal.Request"),
            "Close",
            &(),
        )),
        Inhibit::ScreenSaver(cookie) => drop(connection.call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "UnInhibit",
            &(cookie,),
        )),
    }
}
