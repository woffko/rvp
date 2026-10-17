//! The thumbnail core (M11): a second libmpv core without audio renders
//! frames into small pictures with mpv's software renderer. It serves the
//! seek bar's previews, the scene browsers and the playlist's thumbnail
//! views, and finds files' durations in the background.
//!
//! Threading: the core lives on a thread of its own, which creates it, waits
//! for its events and renders; the caller's thread (the GUI thread) only
//! queues requests and takes finished outcomes, never waiting for mpv. The
//! `notify` callback runs on the core's thread when outcomes wait; it must
//! only schedule a call to [`Thumbnailer::take`].
//!
//! Requests:
//! - a frame of a file at a time, fitted into a box keeping the video's
//!   shape (or filling the box with mpv's black bars), at a keyframe near
//!   the time (fast) or exactly;
//! - the seek bar's preview keeps one waiting request: a newer one replaces
//!   it, and the one being rendered finishes;
//! - other requests wait in order of their group (scenes, then the list)
//!   and their arrival; [`Thumbnailer::cancel`] drops a group's waiting
//!   requests and abandons its request under way;
//! - durations wait behind every frame.
//!
//! Pictures are kept in a cache of [`CACHE_BYTES`], least recently used
//! first out; a cached request is answered without the core.
//!
//! The core must be created after the player's: libmpv gives FFmpeg's log
//! to the first core of a process, and the normalizer reads it there.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle, ThreadId};
use std::time::{Duration, Instant};

use libmpv2::Mpv;
use libmpv2::events::Event;
use libmpv2::render::{SoftwareFormat, SoftwareRenderContext, mpv_render_update};

use crate::engine::EngineError;

/// The cache's size in bytes of pixels.
pub const CACHE_BYTES: usize = 48 << 20;

/// Opening a file may take this long (network sources take longer).
const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
/// A seek and its frame may take this long.
const SEEK_TIMEOUT: Duration = Duration::from_secs(5);
/// Waits for mpv's events in slices this long, to notice cancellations
/// and frames.
const SLICE: f64 = 0.002;

/// Who asked for a picture; a group's requests are cancelled together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// The seek bar's preview: only the newest waiting request counts.
    Preview,
    /// A scene browser's pictures.
    Scenes,
    /// The playlist's thumbnail views.
    List,
}

/// A picture to render.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameRequest {
    pub path: String,
    /// Seconds from the start.
    pub time: f64,
    /// The box the picture fits in.
    pub max_width: u32,
    pub max_height: u32,
    /// The picture fills the box (with black bars where the shapes
    /// differ) instead of taking the video's shape.
    pub fill: bool,
    /// The frame at the time, not a keyframe near it (slower).
    pub exact: bool,
    pub group: Group,
}

/// A rendered picture.
#[derive(Clone, PartialEq)]
pub struct Picture {
    pub path: String,
    /// The time asked for.
    pub time: f64,
    /// The time of the frame shown (a keyframe's may be earlier).
    pub shown: f64,
    pub width: u32,
    pub height: u32,
    /// RGBA, 4 bytes a pixel, rows without padding.
    pub rgba: Vec<u8>,
}

impl Picture {
    fn bytes(&self) -> usize {
        self.rgba.len()
    }
}

impl std::fmt::Debug for Picture {
    // The pixels would fill a log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Picture")
            .field("path", &self.path)
            .field("time", &self.time)
            .field("shown", &self.shown)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// Identifies a request in its outcome.
pub type Ticket = u64;

/// What came of a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Picture {
        ticket: Ticket,
        group: Group,
        picture: Arc<Picture>,
    },
    Failed {
        ticket: Ticket,
        group: Group,
        path: String,
        error: String,
    },
    /// A duration in seconds; `None` when the file has none (a stream) or
    /// cannot be opened.
    Duration { path: String, seconds: Option<f64> },
    /// The core could not start; every request fails.
    Unavailable(String),
}

/// Counters for tests and diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub rendered: u64,
    pub cache_hits: u64,
    /// Waiting previews a newer one replaced.
    pub superseded: u64,
    /// Requests dropped by [`Thumbnailer::cancel`].
    pub cancelled: u64,
    pub failed: u64,
    pub durations: u64,
    /// The thread pictures are rendered on.
    pub render_thread: Option<ThreadId>,
    /// How long the last picture took from its start on the core's thread.
    pub last_render: Option<Duration>,
}

/// How the core is created.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThumbnailOptions {
    /// Further `name=value` options, applied last.
    pub extra: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
enum Work {
    Frame(FrameRequest),
    Duration(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Job {
    ticket: Ticket,
    work: Work,
}

/// The cache's key: the file, the time in milliseconds and the request's
/// shape.
type Key = (String, i64, u32, u32, bool, bool);

fn key(request: &FrameRequest) -> Key {
    // Times are far inside i64 milliseconds.
    #[allow(clippy::cast_possible_truncation)]
    let millis = (request.time * 1000.0).round() as i64;
    (
        request.path.clone(),
        millis,
        request.max_width,
        request.max_height,
        request.fill,
        request.exact,
    )
}

#[derive(Debug, Default)]
struct Cache {
    pictures: BTreeMap<Key, (Arc<Picture>, u64)>,
    bytes: usize,
    clock: u64,
}

impl Cache {
    fn get(&mut self, key: &Key) -> Option<Arc<Picture>> {
        self.clock += 1;
        let clock = self.clock;
        self.pictures.get_mut(key).map(|(picture, used)| {
            *used = clock;
            Arc::clone(picture)
        })
    }

    fn insert(&mut self, key: Key, picture: Arc<Picture>) {
        self.clock += 1;
        self.bytes += picture.bytes();
        if let Some((old, _)) = self.pictures.insert(key, (picture, self.clock)) {
            self.bytes -= old.bytes();
        }
        while self.bytes > CACHE_BYTES {
            let Some(oldest) = self
                .pictures
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some((picture, _)) = self.pictures.remove(&oldest) {
                self.bytes -= picture.bytes();
            }
        }
    }
}

#[derive(Debug, Default)]
struct State {
    /// The seek bar's waiting preview.
    preview: Option<Job>,
    /// Other waiting jobs by (group, ticket); durations after every group.
    frames: BTreeMap<(Group, Ticket), Job>,
    durations: BTreeMap<Ticket, Job>,
    /// The job under way and its group.
    current: Option<(Ticket, Option<Group>)>,
    /// Set when the job under way was cancelled.
    abandoned: bool,
    outcomes: Vec<Outcome>,
    cache: Cache,
    next_ticket: Ticket,
    stats: Stats,
    /// The core could not start (its error).
    unavailable: Option<String>,
    quit: bool,
}

impl State {
    fn next_job(&mut self) -> Option<Job> {
        if let Some(job) = self.preview.take() {
            return Some(job);
        }
        if let Some((_, job)) = self.frames.pop_first() {
            return Some(job);
        }
        self.durations.pop_first().map(|(_, job)| job)
    }
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
    notify: Box<dyn Fn() + Send + Sync>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Delivers an outcome and tells the owner.
    fn deliver(&self, outcome: Outcome) {
        self.lock().outcomes.push(outcome);
        (self.notify)();
    }
}

/// The handle to the thumbnail core; dropping it stops the core.
pub struct Thumbnailer {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Thumbnailer {
    /// Starts the core's thread, which creates the core; returns at once.
    /// Should the core fail to start, an [`Outcome::Unavailable`] says why.
    ///
    /// # Errors
    /// Fails when the thread cannot start.
    pub fn start(
        options: ThumbnailOptions,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, EngineError> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
            notify: Box::new(notify),
        });
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("rvp-thumbnails".to_owned())
            .spawn(move || run(&worker, &options))
            .map_err(|error| EngineError(format!("thumbnail thread: {error}")))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    /// Queues a picture; the outcome comes through [`Thumbnailer::take`]. A
    /// cached picture is answered at once (and `notify` runs on this
    /// thread).
    #[must_use]
    pub fn request(&self, request: FrameRequest) -> Ticket {
        let mut state = self.shared.lock();
        state.next_ticket += 1;
        let ticket = state.next_ticket;
        let group = request.group;
        if let Some(picture) = state.cache.get(&key(&request)) {
            state.stats.cache_hits += 1;
            state.outcomes.push(Outcome::Picture {
                ticket,
                group,
                picture,
            });
            drop(state);
            (self.shared.notify)();
            return ticket;
        }
        if let Some(error) = state.unavailable.clone() {
            state.stats.failed += 1;
            state.outcomes.push(Outcome::Failed {
                ticket,
                group,
                path: request.path,
                error,
            });
            drop(state);
            (self.shared.notify)();
            return ticket;
        }
        let job = Job {
            ticket,
            work: Work::Frame(request),
        };
        if group == Group::Preview {
            if state.preview.replace(job).is_some() {
                state.stats.superseded += 1;
            }
        } else {
            state.frames.insert((group, ticket), job);
        }
        drop(state);
        self.shared.wake.notify_one();
        ticket
    }

    /// Queues finding a file's duration ("Detect file durations").
    #[must_use]
    pub fn duration(&self, path: &str) -> Ticket {
        let mut state = self.shared.lock();
        state.next_ticket += 1;
        let ticket = state.next_ticket;
        state.durations.insert(
            ticket,
            Job {
                ticket,
                work: Work::Duration(path.to_owned()),
            },
        );
        drop(state);
        self.shared.wake.notify_one();
        ticket
    }

    /// Drops a group's waiting requests and abandons its request under
    /// way; their outcomes never come.
    pub fn cancel(&self, group: Group) {
        let mut state = self.shared.lock();
        let mut dropped = 0;
        if group == Group::Preview && state.preview.take().is_some() {
            dropped += 1;
        }
        let before = state.frames.len();
        state.frames.retain(|(candidate, _), _| *candidate != group);
        dropped += before - state.frames.len();
        if state
            .current
            .is_some_and(|(_, current)| current == Some(group))
        {
            state.abandoned = true;
            dropped += 1;
        }
        state.stats.cancelled += u64::try_from(dropped).unwrap_or(u64::MAX);
    }

    /// Drops every waiting duration request.
    pub fn cancel_durations(&self) {
        let mut state = self.shared.lock();
        state.stats.cancelled += u64::try_from(state.durations.len()).unwrap_or(u64::MAX);
        state.durations.clear();
    }

    /// The outcomes that wait, oldest first.
    #[must_use]
    pub fn take(&self) -> Vec<Outcome> {
        std::mem::take(&mut self.shared.lock().outcomes)
    }

    /// A cached picture, without queueing anything.
    #[must_use]
    pub fn cached(&self, request: &FrameRequest) -> Option<Arc<Picture>> {
        self.shared.lock().cache.get(&key(request))
    }

    /// How many requests wait (not counting the one under way).
    #[must_use]
    pub fn waiting(&self) -> usize {
        let state = self.shared.lock();
        usize::from(state.preview.is_some()) + state.frames.len() + state.durations.len()
    }

    #[must_use]
    pub fn stats(&self) -> Stats {
        self.shared.lock().stats.clone()
    }
}

impl Drop for Thumbnailer {
    fn drop(&mut self) {
        self.shared.lock().quit = true;
        self.shared.wake.notify_all();
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// The core's thread: creates the core and serves requests until the
/// handle is dropped.
fn run(shared: &Arc<Shared>, options: &ThumbnailOptions) {
    let mpv = match create(options) {
        Ok(mpv) => mpv,
        Err(error) => return unavailable(shared, &error),
    };
    let mut render = match mpv.create_software_render_context() {
        Ok(render) => render,
        Err(error) => return unavailable(shared, &format!("software renderer: {error}")),
    };
    let frames = Arc::new(FrameSignal::default());
    let signal = Arc::clone(&frames);
    // On an mpv thread: only signal (the render API's rule).
    render.set_update_callback(move || signal.raise());
    let mut worker = Worker {
        mpv: &mpv,
        render: &render,
        frames: &frames,
        shared,
        loaded: None,
    };
    worker.serve();
    drop(render);
    drop(mpv);
}

fn unavailable(shared: &Shared, error: &str) {
    let mut state = shared.lock();
    state.unavailable = Some(error.to_owned());
    let mut outcomes = vec![Outcome::Unavailable(error.to_owned())];
    while let Some(job) = state.next_job() {
        match job.work {
            Work::Frame(request) => outcomes.push(Outcome::Failed {
                ticket: job.ticket,
                group: request.group,
                path: request.path,
                error: error.to_owned(),
            }),
            Work::Duration(path) => outcomes.push(Outcome::Duration {
                path,
                seconds: None,
            }),
        }
    }
    state.outcomes.extend(outcomes);
    drop(state);
    (shared.notify)();
}

fn create(options: &ThumbnailOptions) -> Result<Mpv, String> {
    Mpv::with_initializer(|init| {
        init.set_option("vo", "libmpv")?;
        // No audio: no track, no device.
        init.set_option("aid", "no")?;
        init.set_option("ao", "null")?;
        init.set_option("sid", "no")?;
        init.set_option("sub-auto", "no")?;
        init.set_option("audio-file-auto", "no")?;
        init.set_option("hwdec", "no")?;
        init.set_option("vd-lavc-threads", "2")?;
        // Pictures, not playback: a cheaper decode is good enough.
        init.set_option("vd-lavc-skiploopfilter", "all")?;
        init.set_option("pause", "yes")?;
        init.set_option("keep-open", "always")?;
        init.set_option("idle", "yes")?;
        init.set_option("image-display-duration", "inf")?;
        init.set_option("osd-level", "0")?;
        init.set_option("demuxer-readahead-secs", "0")?;
        init.set_option("cache", "no")?;
        init.set_option("input-default-bindings", "no")?;
        init.set_option("input-vo-keyboard", "no")?;
        init.set_option("terminal", "no")?;
        for name in ["osc", "ytdl", "load-scripts"] {
            drop(init.set_option(name, "no"));
        }
        for (name, value) in &options.extra {
            init.set_option(name.as_str(), value.as_str())?;
        }
        Ok(())
    })
    .map_err(|error| format!("thumbnail core: {error}"))
}

/// Raised by mpv's render update callback.
#[derive(Default)]
struct FrameSignal {
    raised: Mutex<bool>,
    changed: Condvar,
}

impl FrameSignal {
    fn raise(&self) {
        *self.raised.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.changed.notify_all();
    }

    fn clear(&self) {
        *self.raised.lock().unwrap_or_else(PoisonError::into_inner) = false;
    }

    /// True (and cleared) when it was raised.
    fn take(&self) -> bool {
        std::mem::take(&mut *self.raised.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Waits up to `timeout` for the signal, leaving it raised.
    fn wait(&self, timeout: Duration) {
        let raised = self.raised.lock().unwrap_or_else(PoisonError::into_inner);
        drop(
            self.changed
                .wait_timeout_while(raised, timeout, |raised| !*raised)
                .unwrap_or_else(PoisonError::into_inner),
        );
    }
}

/// Why a job stopped.
enum Stop {
    Cancelled,
    Failed(String),
}

struct Loaded {
    path: String,
    /// Opened with video (frames), not only for its duration.
    video: bool,
    duration: Option<f64>,
    /// The video's display size (after its aspect ratio).
    display: Option<(u32, u32)>,
}

struct Worker<'a> {
    mpv: &'a Mpv,
    render: &'a SoftwareRenderContext<'a>,
    frames: &'a FrameSignal,
    shared: &'a Shared,
    loaded: Option<Loaded>,
}

impl Worker<'_> {
    fn serve(&mut self) {
        loop {
            let job = {
                let mut state = self.shared.lock();
                loop {
                    if state.quit {
                        return;
                    }
                    if let Some(job) = state.next_job() {
                        let group = match &job.work {
                            Work::Frame(request) => Some(request.group),
                            Work::Duration(_) => None,
                        };
                        state.current = Some((job.ticket, group));
                        state.abandoned = false;
                        break job;
                    }
                    state = self
                        .shared
                        .wake
                        .wait(state)
                        .unwrap_or_else(PoisonError::into_inner);
                }
            };
            match job.work {
                Work::Frame(request) => self.frame(job.ticket, request),
                Work::Duration(path) => self.duration(path),
            }
            self.shared.lock().current = None;
        }
    }

    /// True when the job under way was cancelled or the handle dropped.
    fn stopped(&self) -> bool {
        let state = self.shared.lock();
        state.quit || state.abandoned
    }

    fn frame(&mut self, ticket: Ticket, request: FrameRequest) {
        let started = Instant::now();
        let group = request.group;
        match self.render_frame(&request) {
            Ok(picture) => {
                let picture = Arc::new(picture);
                let mut state = self.shared.lock();
                if state.abandoned {
                    return;
                }
                state.cache.insert(key(&request), Arc::clone(&picture));
                state.stats.rendered += 1;
                state.stats.render_thread = Some(thread::current().id());
                state.stats.last_render = Some(started.elapsed());
                state.outcomes.push(Outcome::Picture {
                    ticket,
                    group,
                    picture,
                });
                drop(state);
                (self.shared.notify)();
            }
            Err(Stop::Cancelled) => {}
            Err(Stop::Failed(error)) => {
                self.shared.lock().stats.failed += 1;
                self.shared.deliver(Outcome::Failed {
                    ticket,
                    group,
                    path: request.path,
                    error,
                });
            }
        }
    }

    fn duration(&mut self, path: String) {
        let known = self
            .loaded
            .as_ref()
            .filter(|loaded| loaded.path == path)
            .map(|loaded| loaded.duration);
        let seconds = match known {
            Some(duration) => duration,
            None => match self.open(&path, false) {
                Ok(()) => self.loaded.as_ref().and_then(|loaded| loaded.duration),
                Err(Stop::Cancelled) => return,
                Err(Stop::Failed(_)) => None,
            },
        };
        self.shared.lock().stats.durations += 1;
        self.shared.deliver(Outcome::Duration { path, seconds });
    }

    /// Opens `path`; `video` decodes its video (frames), otherwise only its
    /// duration is read: with no track selected at all, mpv still opens
    /// the file (with tracks selected but none of them video or audio, it
    /// would refuse it as having nothing to play).
    fn open(&mut self, path: &str, video: bool) -> Result<(), Stop> {
        self.loaded = None;
        let failed = |error: libmpv2::Error| Stop::Failed(error.to_string());
        self.mpv
            .set_property("track-auto-selection", if video { "yes" } else { "no" })
            .map_err(failed)?;
        self.mpv
            .command("loadfile", &[path, "replace"])
            .map_err(failed)?;
        let deadline = Instant::now() + LOAD_TIMEOUT;
        loop {
            match self.next_event(deadline, "opening")? {
                Some(Ok(Event::FileLoaded)) => break,
                Some(Err(error)) => return Err(Stop::Failed(error.to_string())),
                // The end of the file opened before, and the rest.
                _ => {}
            }
        }
        let duration = self
            .mpv
            .get_property::<f64>("duration")
            .ok()
            .filter(|seconds| seconds.is_finite() && *seconds > 0.0);
        self.loaded = Some(Loaded {
            path: path.to_owned(),
            video,
            duration,
            display: None,
        });
        if video {
            // The first frame after opening: playback restarts once, and a
            // seek must not take that restart for its own.
            self.settle(None)?;
            let display = |name: &str| {
                self.mpv
                    .get_property::<i64>(name)
                    .ok()
                    .and_then(|value| u32::try_from(value).ok())
                    .filter(|value| *value > 0)
            };
            let size = display("dwidth").zip(display("dheight"));
            if let Some(loaded) = self.loaded.as_mut() {
                loaded.display = size;
            }
        }
        Ok(())
    }

    /// The next event of mpv within a short wait (`None` when none came);
    /// fails once `deadline` passed, or when the job was cancelled.
    fn next_event(
        &self,
        deadline: Instant,
        what: &str,
    ) -> Result<Option<Result<Event<'_>, libmpv2::Error>>, Stop> {
        if self.stopped() {
            return Err(Stop::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(Stop::Failed(format!("timed out {what}")));
        }
        Ok(self.mpv.wait_event(SLICE))
    }

    /// Waits until playback restarted after opening or seeking, rendering
    /// every frame mpv offers meanwhile: `vo=libmpv` holds playback until
    /// the frame it offered was rendered (or 200 ms passed), so waiting for
    /// the restart first would cost that time. For the same reason no
    /// property is read here: reading one waits for the playback core,
    /// which waits for the frame. Returns the last frame rendered for
    /// `request` (the one playback restarted on).
    fn settle(&self, request: Option<&FrameRequest>) -> Result<Option<Picture>, Stop> {
        let deadline = Instant::now() + SEEK_TIMEOUT;
        let mut restarted = false;
        let mut picture = None;
        // After the restart, a frame that does not come is the one shown
        // already (a seek to where playback was).
        let mut patience: Option<Instant> = None;
        loop {
            if self.frames.take()
                && self
                    .render
                    .update()
                    .is_ok_and(|flags| flags & mpv_render_update::Frame != 0)
            {
                picture = Some(self.picture(request)?);
            }
            if restarted && (request.is_none() || picture.is_some()) {
                return Ok(picture);
            }
            if let Some(until) = patience
                && Instant::now() >= until
            {
                return self.picture(request).map(Some);
            }
            if restarted {
                // Only a frame is awaited now.
                if self.stopped() {
                    return Err(Stop::Cancelled);
                }
                self.frames.wait(Duration::from_millis(2));
                continue;
            }
            match self.next_event(deadline, "seeking")? {
                Some(Ok(Event::PlaybackRestart)) => {
                    restarted = true;
                    patience = Some(Instant::now() + Duration::from_millis(300));
                }
                Some(Err(error)) => return Err(Stop::Failed(error.to_string())),
                _ => {}
            }
        }
    }

    /// Renders the current frame for `request`, or (with none) into a tiny
    /// picture that only lets playback go on. `shown` is filled in later
    /// (see [`Worker::settle`]).
    fn picture(&self, request: Option<&FrameRequest>) -> Result<Picture, Stop> {
        let (width, height) = request.map_or((16, 16), |request| self.size(request));
        let rgba = self.draw(width, height)?;
        Ok(Picture {
            path: request
                .map(|request| request.path.clone())
                .unwrap_or_default(),
            time: request.map_or(0.0, |request| request.time),
            shown: 0.0,
            width,
            height,
            rgba,
        })
    }

    fn render_frame(&mut self, request: &FrameRequest) -> Result<Picture, Stop> {
        let ready = self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.path == request.path && loaded.video);
        if !ready {
            self.open(&request.path, true)?;
        }
        let has_video = self
            .mpv
            .get_property::<String>("vid")
            .is_ok_and(|vid| vid != "no");
        if !has_video {
            return Err(Stop::Failed("no video".to_owned()));
        }
        let duration = self.loaded.as_ref().and_then(|loaded| loaded.duration);
        let mut time = request.time.max(0.0);
        if let Some(duration) = duration {
            time = time.min((duration - 0.1).max(0.0));
        }
        let at = self.mpv.get_property::<f64>("time-pos").ok();
        if !ready && time < 0.05 && at.is_some_and(|at| at < 0.05) {
            // Just opened at the start: the frame shown is the one asked for.
            let mut picture = self.picture(Some(request))?;
            picture.shown = at.unwrap_or(0.0);
            return Ok(picture);
        }
        // Frames offered before the seek must not count.
        self.frames.clear();
        drop(self.render.update());
        let flags = if request.exact {
            "absolute+exact"
        } else {
            "absolute+keyframes"
        };
        self.mpv
            .command("seek", &[&format!("{time:.3}"), flags])
            .map_err(|error| Stop::Failed(error.to_string()))?;
        let mut picture = match self.settle(Some(request))? {
            Some(picture) => picture,
            None => self.picture(Some(request))?,
        };
        picture.shown = self.mpv.get_property::<f64>("time-pos").unwrap_or(time);
        Ok(picture)
    }

    /// The picture's size: the video's display shape fitted into the box,
    /// or the box.
    fn size(&self, request: &FrameRequest) -> (u32, u32) {
        let bounds = (request.max_width.max(1), request.max_height.max(1));
        let display = self.loaded.as_ref().and_then(|loaded| loaded.display);
        match display {
            Some(shape) if !request.fill => fit(shape, bounds),
            _ => bounds,
        }
    }

    /// Renders the current frame as RGBA.
    fn draw(&self, width: u32, height: u32) -> Result<Vec<u8>, Stop> {
        let columns = width as usize;
        let rows = height as usize;
        // Rows aligned to 64 bytes, as mpv's renderer prefers.
        let stride = (columns * 4).div_ceil(64) * 64;
        let mut pixels = vec![0_u32; stride / 4 * rows];
        let (Ok(surface_width), Ok(surface_height)) = (i32::try_from(width), i32::try_from(height))
        else {
            return Err(Stop::Failed("picture too large".to_owned()));
        };
        self.render
            .render(
                surface_width,
                surface_height,
                SoftwareFormat::Rgb0,
                stride,
                &mut pixels,
            )
            .map_err(|error| Stop::Failed(format!("render: {error}")))?;
        let mut rgba = Vec::with_capacity(columns * rows * 4);
        for row in pixels.chunks_exact(stride / 4) {
            for pixel in &row[..columns] {
                let [red, green, blue, _] = pixel.to_ne_bytes();
                rgba.extend_from_slice(&[red, green, blue, 255]);
            }
        }
        Ok(rgba)
    }
}

/// A `(width, height)` shape scaled to fit `bounds`, at least 1x1.
#[must_use]
pub fn fit(shape: (u32, u32), bounds: (u32, u32)) -> (u32, u32) {
    let (width, height) = (f64::from(shape.0.max(1)), f64::from(shape.1.max(1)));
    let (max_width, max_height) = (f64::from(bounds.0.max(1)), f64::from(bounds.1.max(1)));
    let scale = (max_width / width).min(max_height / height);
    // Within the bounds, which are u32.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scaled = |value: f64, max: f64| (value * scale).round().clamp(1.0, max) as u32;
    (scaled(width, max_width), scaled(height, max_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_keeps_the_shape() {
        assert_eq!(fit((1920, 1080), (160, 160)), (160, 90));
        assert_eq!(fit((1080, 1920), (160, 160)), (90, 160));
        assert_eq!(fit((96, 72), (160, 90)), (120, 90));
        assert_eq!(fit((0, 0), (10, 10)), (10, 10));
    }

    #[test]
    fn the_cache_drops_the_least_recently_used() {
        let mut cache = Cache::default();
        let picture = |bytes: usize| {
            Arc::new(Picture {
                path: String::new(),
                time: 0.0,
                shown: 0.0,
                width: 1,
                height: 1,
                rgba: vec![0; bytes],
            })
        };
        let key = |n: i64| (String::from("a"), n, 1, 1, false, false);
        cache.insert(key(1), picture(CACHE_BYTES / 2));
        cache.insert(key(2), picture(CACHE_BYTES / 2));
        assert!(cache.get(&key(1)).is_some());
        cache.insert(key(3), picture(CACHE_BYTES / 2));
        assert!(cache.get(&key(2)).is_none(), "2 was used least recently");
        assert!(cache.get(&key(1)).is_some());
        assert!(cache.get(&key(3)).is_some());
        assert_eq!(cache.bytes, CACHE_BYTES);
    }
}
