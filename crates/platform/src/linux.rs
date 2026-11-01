//! MPRIS, idle inhibit, the launcher's progress and the screen lock over the
//! session D-Bus.
//!
//! zbus 6 runs a connection on the runtime of the thread that made it
//! (zruntime's, in `zbus::block_on`). The desktop thread makes RVP's
//! connection inside one `block_on` that lasts as long as the [`Desktop`],
//! so that thread serves MPRIS's calls, sends the signals and makes the
//! inhibit calls; its messages come through an async channel, and the
//! screen lock's signals through a message stream it polls beside it.

use std::collections::{BTreeMap, HashMap};
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::Poll;
use std::time::Duration;

use futures_core::Stream;
use rvp_app::media::{MediaCommand, MediaState, RATE_RANGE, Status};
use zbus::fdo::RequestNameFlags;
use zbus::object_server::{InterfaceRef, SignalEmitter};
use zbus::{
    Connection, MatchRule, MessageStream, ObjectPath, OwnedObjectPath, OwnedVariant, Variant,
    connection, interface, message,
};
use zruntime::mpmc::{Receiver, Sender, unbounded};

/// MPRIS's object path.
const PATH: &str = "/org/mpris/MediaPlayer2";
/// RVP's bus name; a second RVP adds `.instance<pid>`, as MPRIS asks.
const NAME: &str = "org.mpris.MediaPlayer2.rvp";
/// The track ID MPRIS reserves for "no track".
const NO_TRACK: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";
/// The `Inhibit` flag for idle (the screen saver and blanking), the same
/// for GNOME's session manager and the portal.
const INHIBIT_IDLE: u32 = 8;
/// MPRIS counts time in microseconds.
const MICROS: f64 = 1_000_000.0;
/// The Unity launcher API, which Ubuntu's dock and KDE's task manager read
/// for a program's progress: its signal's interface, and the path RVP sends
/// it from (any path will do).
const LAUNCHER: &str = "com.canonical.Unity.LauncherEntry";
const LAUNCHER_PATH: &str = "/com/canonical/unity/launcherentry/rvp";
/// The smallest change of the progress worth a signal (half a percent).
const PROGRESS_STEP: f64 = 0.005;
/// The screen savers whose `ActiveChanged(b)` says the screen locked or
/// unlocked: GNOME's, and freedesktop.org's (KDE, Xfce, MATE).
const SCREEN_SAVERS: [&str; 2] = ["org.gnome.ScreenSaver", "org.freedesktop.ScreenSaver"];
/// How long the window's thread waits for the desktop thread to be on the
/// bus.
const START_TIMEOUT: Duration = Duration::from_secs(10);

type OnCommand = Arc<dyn Fn(MediaCommand) + Send + Sync>;
type OnLock = Arc<dyn Fn(bool) + Send + Sync>;

/// How RVP shows itself on the bus.
#[derive(Debug, Clone)]
pub struct Options {
    /// The `.desktop` file's name without its extension (`rvp`).
    pub desktop_entry: String,
    /// Why the screen stays on, for the desktop's list of inhibitors.
    pub inhibit_reason: String,
}

/// RVP on the desktop's bus: MPRIS, idle inhibit, the launcher's progress
/// and the screen lock. Dropping it ends its thread, which releases an
/// inhibit it holds and hides the progress.
pub struct Desktop {
    sender: Sender<Message>,
    bus_name: String,
}

enum Message {
    State(Box<MediaState>),
    Seeked(f64),
    Inhibit(bool),
    /// The taskbar state (0 nothing loaded, 1 playing, 2 paused) and the
    /// position as a fraction (below 0: unknown).
    Progress(i32, f64),
}

impl Desktop {
    /// Registers on the session bus. `on_command` runs on the desktop thread
    /// for each MPRIS call, `on_lock` when the screen locks (`true`) or
    /// unlocks.
    ///
    /// # Errors
    /// Fails without a session bus or when the names cannot be taken.
    pub fn start(
        options: Options,
        on_command: impl Fn(MediaCommand) + Send + Sync + 'static,
        on_lock: impl Fn(bool) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        Self::start_with(
            connection::Builder::session,
            options,
            Arc::new(on_command),
            Arc::new(on_lock),
        )
    }

    /// Registers on the bus at `address` (a private bus for tests).
    ///
    /// # Errors
    /// Fails when the bus cannot be reached or the names cannot be taken.
    pub fn start_at(
        address: &str,
        options: Options,
        on_command: impl Fn(MediaCommand) + Send + Sync + 'static,
        on_lock: impl Fn(bool) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let address = address.to_owned();
        Self::start_with(
            move || connection::Builder::address(address.as_str()),
            options,
            Arc::new(on_command),
            Arc::new(on_lock),
        )
    }

    fn start_with(
        builder: impl FnOnce() -> connection::Builder<'static> + Send + 'static,
        options: Options,
        on_command: OnCommand,
        on_lock: OnLock,
    ) -> Result<Self, String> {
        let (sender, receiver) = unbounded();
        let (started, start) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("rvp-desktop".to_owned())
            .spawn(move || {
                zbus::block_on(run(
                    builder, options, on_command, on_lock, receiver, started,
                ));
            })
            .map_err(|e| format!("desktop thread: {e}"))?;
        let bus_name = start
            .recv_timeout(START_TIMEOUT)
            .map_err(|_| "MPRIS on the session bus: no answer".to_owned())??;
        Ok(Self { sender, bus_name })
    }

    /// The bus name RVP took.
    #[must_use]
    pub fn bus_name(&self) -> &str {
        &self.bus_name
    }

    /// The playback state now; MPRIS announces what changed.
    pub fn update(&self, state: MediaState) {
        drop(self.sender.try_send(Message::State(Box::new(state))));
    }

    /// Playback jumped to `position` seconds (MPRIS's `Seeked`).
    pub fn seeked(&self, position: f64) {
        drop(self.sender.try_send(Message::Seeked(position)));
    }

    /// Keeps the screen on (video plays) or lets it sleep again.
    pub fn inhibit(&self, on: bool) {
        drop(self.sender.try_send(Message::Inhibit(on)));
    }

    /// The launcher's progress, as Windows' taskbar button shows it: `state`
    /// 0 when nothing is loaded, 1 playing, 2 paused; `progress` the
    /// position as a fraction of the length, below 0 when it is unknown.
    /// Only visible changes are sent.
    pub fn progress(&self, state: i32, progress: f64) {
        drop(self.sender.try_send(Message::Progress(state, progress)));
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
    fn metadata(&self) -> HashMap<String, OwnedVariant> {
        let state = self.snapshot();
        let mut metadata = HashMap::new();
        let mut put = |key: &str, value: Variant<'_>| {
            if let Ok(value) = OwnedVariant::try_from(value) {
                metadata.insert(key.to_owned(), value);
            }
        };
        if let Ok(path) = OwnedObjectPath::try_from(track_path(&state)) {
            put("mpris:trackid", Variant::from(path));
        }
        if state.status != Status::Stopped {
            put("xesam:title", Variant::from(state.title.clone()));
            if let Some(url) = &state.url {
                put("xesam:url", Variant::from(url.clone()));
            }
            if let Some(length) = state.length {
                put("mpris:length", Variant::from(micros(length)));
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
    /// GNOME's session manager's cookie.
    Session(u32),
    /// `org.freedesktop.ScreenSaver`'s cookie.
    ScreenSaver(u32),
    /// The portal's request object; closing it ends the inhibit.
    Portal(OwnedObjectPath),
}

/// What the desktop thread waits for.
enum Event {
    Message(Message),
    /// A screen saver's `ActiveChanged`.
    Signal(zbus::Message),
    /// The screen savers' signals ended (the bus went away).
    SignalsEnded,
    /// The [`Desktop`] was dropped.
    Closed,
}

/// What the launcher shows of RVP.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shown {
    visible: bool,
    value: f64,
}

/// The desktop thread: RVP's connection, made here; state changes become
/// MPRIS signals, inhibit requests bus calls, the progress the launcher's
/// signal, the screen savers' signals `on_lock` calls. Ends when the
/// [`Desktop`] is dropped.
async fn run(
    builder: impl FnOnce() -> connection::Builder<'static>,
    options: Options,
    on_command: OnCommand,
    on_lock: OnLock,
    receiver: Receiver<Message>,
    started: std::sync::mpsc::Sender<Result<String, String>>,
) {
    let shared = Arc::new(Mutex::new(stopped()));
    let (connection, bus_name) = match connect(builder(), &options, &on_command, &shared).await {
        Ok(connected) => connected,
        Err(error) => {
            drop(started.send(Err(error)));
            return;
        }
    };
    let player = connection
        .object_server()
        .interface::<_, Player>(PATH)
        .await
        .ok();
    // Watched before the start returns: a lock just after it is not missed.
    let mut signals = screen_saver_signals(&connection).await;
    drop(started.send(Ok(bus_name)));
    let mut held: Option<Inhibit> = None;
    let mut shown = Shown {
        visible: false,
        value: 0.0,
    };
    let mut locked = None;
    loop {
        match next_event(&receiver, &mut signals).await {
            Event::Closed => break,
            Event::SignalsEnded => signals = None,
            Event::Signal(signal) => {
                // Both screen savers may report the same change.
                if let Some(active) = screen_locked(&signal)
                    && locked != Some(active)
                {
                    locked = Some(active);
                    on_lock(active);
                }
            }
            Event::Message(Message::State(state)) => {
                let old = std::mem::replace(&mut *lock(&shared), (*state).clone());
                if let Some(player) = &player {
                    announce(player, &old, &state).await;
                }
            }
            Event::Message(Message::Seeked(position)) => {
                if let Some(player) = &player {
                    drop(Player::seeked(player.signal_emitter(), micros(position)).await);
                }
            }
            Event::Message(Message::Inhibit(true)) if held.is_none() => {
                held = inhibit(&connection, &options).await;
            }
            Event::Message(Message::Inhibit(false)) => {
                if let Some(inhibit) = held.take() {
                    release(&connection, inhibit).await;
                }
            }
            Event::Message(Message::Inhibit(true)) => {}
            Event::Message(Message::Progress(state, progress)) => {
                let visible = state != 0 && progress >= 0.0;
                let value = if visible {
                    progress.clamp(0.0, 1.0)
                } else {
                    shown.value
                };
                if visible != shown.visible
                    || (visible && (value - shown.value).abs() >= PROGRESS_STEP)
                {
                    shown = Shown { visible, value };
                    show_progress(&connection, &options, shown).await;
                }
            }
        }
    }
    if let Some(inhibit) = held.take() {
        release(&connection, inhibit).await;
    }
    if shown.visible {
        shown.visible = false;
        show_progress(&connection, &options, shown).await;
    }
}

/// Serves MPRIS on a new connection from `builder` and takes RVP's name;
/// returns the connection and the name.
async fn connect(
    builder: connection::Builder<'static>,
    options: &Options,
    on_command: &OnCommand,
    state: &Arc<Mutex<MediaState>>,
) -> Result<(Connection, String), String> {
    let error = |e: zbus::Error| format!("MPRIS on the session bus: {e}");
    let connection = builder
        .serve_at(
            PATH,
            Root {
                desktop_entry: options.desktop_entry.clone(),
                on_command: Arc::clone(on_command),
            },
        )
        .serve_at(
            PATH,
            Player {
                state: Arc::clone(state),
                on_command: Arc::clone(on_command),
            },
        )
        .build()
        .await
        .map_err(error)?;
    let instance = format!("{NAME}.instance{}", std::process::id());
    let bus_name = if take_name(&connection, NAME).await.is_ok() {
        NAME.to_owned()
    } else {
        take_name(&connection, &instance).await.map_err(error)?;
        instance
    };
    Ok((connection, bus_name))
}

/// Takes `name` without queueing: a name another RVP holds fails at once
/// (queued, the request would report success).
async fn take_name(connection: &Connection, name: &str) -> zbus::Result<()> {
    connection
        .request_name_with_flags(name.to_owned(), RequestNameFlags::DO_NOT_QUEUE)
        .await
        .map(drop)
}

/// The next message from the [`Desktop`], or screen saver signal.
async fn next_event(receiver: &Receiver<Message>, signals: &mut Option<MessageStream>) -> Event {
    // Dropping a pending receive takes no message from the channel.
    let mut message = pin!(receiver.recv());
    poll_fn(|context| {
        if let Poll::Ready(received) = message.as_mut().poll(context) {
            return Poll::Ready(received.map_or(Event::Closed, Event::Message));
        }
        if let Some(stream) = signals.as_mut() {
            loop {
                match Pin::new(&mut *stream).poll_next(context) {
                    Poll::Ready(Some(Ok(signal))) => return Poll::Ready(Event::Signal(signal)),
                    // A message that does not read is left out.
                    Poll::Ready(Some(Err(_))) => {}
                    Poll::Ready(None) => return Poll::Ready(Event::SignalsEnded),
                    Poll::Pending => break,
                }
            }
        }
        Poll::Pending
    })
    .await
}

/// The screen savers' `ActiveChanged` signals; `None` when the bus refuses
/// the match.
async fn screen_saver_signals(connection: &Connection) -> Option<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(message::Type::Signal)
        .member("ActiveChanged")
        .build()
        .ok()?;
    MessageStream::for_match_rule(rule, connection, None)
        .await
        .ok()
}

/// Whether a screen saver's `ActiveChanged` says the screen locked.
fn screen_locked(signal: &zbus::Message) -> Option<bool> {
    let header = signal.header();
    let interface = header.interface()?;
    if !SCREEN_SAVERS.contains(&interface.as_str()) {
        return None;
    }
    signal.body().deserialize::<bool>().ok()
}

/// The launcher API's `Update` for RVP's desktop entry.
async fn show_progress(connection: &Connection, options: &Options, shown: Shown) {
    let uri = format!("application://{}.desktop", options.desktop_entry);
    let properties = BTreeMap::from([
        ("progress", Variant::from(shown.value)),
        ("progress-visible", Variant::from(shown.visible)),
    ]);
    // A signal that cannot be sent (the bus went away) is not retried.
    drop(
        connection
            .emit_signal(
                None::<&str>,
                LAUNCHER_PATH,
                LAUNCHER,
                "Update",
                &(uri.as_str(), properties),
            )
            .await,
    );
}

/// Emits `PropertiesChanged` for what differs between two states.
async fn announce(player: &InterfaceRef<Player>, old: &MediaState, new: &MediaState) {
    let iface = player.get().await;
    let emitter = player.signal_emitter();
    // A signal that cannot be sent (the bus went away) is not retried.
    if old.status != new.status {
        drop(iface.playback_status_changed(emitter).await);
        drop(iface.can_pause_changed(emitter).await);
    }
    if (old.status, old.track, &old.title, &old.url, old.length)
        != (new.status, new.track, &new.title, &new.url, new.length)
    {
        drop(iface.metadata_changed(emitter).await);
    }
    if (old.volume - new.volume).abs() > f64::EPSILON {
        drop(iface.volume_changed(emitter).await);
    }
    if (old.rate - new.rate).abs() > f64::EPSILON {
        drop(iface.rate_changed(emitter).await);
    }
    if old.can_next != new.can_next {
        drop(iface.can_go_next_changed(emitter).await);
    }
    if old.can_previous != new.can_previous {
        drop(iface.can_go_previous_changed(emitter).await);
    }
    if old.can_play != new.can_play {
        drop(iface.can_play_changed(emitter).await);
    }
    if old.can_seek != new.can_seek {
        drop(iface.can_seek_changed(emitter).await);
    }
}

/// Asks GNOME's session manager, then `org.freedesktop.ScreenSaver` (KDE,
/// Xfce, MATE), then the portal to keep the screen on; `None` when none
/// answers. Each drops the inhibit of a client that leaves the bus.
///
/// The portal comes last: for a program outside a sandbox it passes an
/// empty app ID on to GNOME's session manager, which refuses it, and still
/// answers success (GNOME 42); GTK, too, asks the session manager itself
/// outside a sandbox. Inside one, the portal is the only service reachable.
async fn inhibit(connection: &Connection, options: &Options) -> Option<Inhibit> {
    let reason = options.inhibit_reason.as_str();
    let cookie = |reply: zbus::Result<zbus::Message>| reply.ok()?.body().deserialize::<u32>().ok();
    // No X11 window to name on Wayland: 0.
    let session = connection
        .call_method(
            Some("org.gnome.SessionManager"),
            "/org/gnome/SessionManager",
            Some("org.gnome.SessionManager"),
            "Inhibit",
            &(options.desktop_entry.as_str(), 0u32, reason, INHIBIT_IDLE),
        )
        .await;
    if let Some(cookie) = cookie(session) {
        return Some(Inhibit::Session(cookie));
    }
    let screen_saver = connection
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "Inhibit",
            &("RVP", reason),
        )
        .await;
    if let Some(cookie) = cookie(screen_saver) {
        return Some(Inhibit::ScreenSaver(cookie));
    }
    let portal_options = BTreeMap::from([("reason", Variant::from(reason))]);
    let reply = connection
        .call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.Inhibit"),
            "Inhibit",
            &("", INHIBIT_IDLE, portal_options),
        )
        .await
        .ok()?;
    reply
        .body()
        .deserialize::<OwnedObjectPath>()
        .ok()
        .map(Inhibit::Portal)
}

async fn release(connection: &Connection, inhibit: Inhibit) {
    // A release that fails ends with RVP anyway: the bus drops what a
    // client that leaves held.
    let reply = match inhibit {
        Inhibit::Session(cookie) => {
            connection
                .call_method(
                    Some("org.gnome.SessionManager"),
                    "/org/gnome/SessionManager",
                    Some("org.gnome.SessionManager"),
                    "Uninhibit",
                    &(cookie,),
                )
                .await
        }
        Inhibit::ScreenSaver(cookie) => {
            connection
                .call_method(
                    Some("org.freedesktop.ScreenSaver"),
                    "/org/freedesktop/ScreenSaver",
                    Some("org.freedesktop.ScreenSaver"),
                    "UnInhibit",
                    &(cookie,),
                )
                .await
        }
        Inhibit::Portal(handle) => {
            connection
                .call_method(
                    Some("org.freedesktop.portal.Desktop"),
                    handle.as_str(),
                    Some("org.freedesktop.portal.Request"),
                    "Close",
                    &(),
                )
                .await
        }
    };
    drop(reply);
}
