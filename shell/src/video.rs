//! The libmpv render context on the scene graph's render thread (D4).
//!
//! The engine lives for the whole process in [`ENGINE`] because the render
//! context borrows it. The C++ video item calls [`render`] for every frame;
//! the first call creates the render context, and only then may files be
//! loaded (`vo=libmpv` without a render context disables video). The shell
//! is told through [`READY`] and a queued call.
//!
//! Goal D rule 7: mpv's default `video-timing-offset` is kept and Qt's swap
//! times are not reported to mpv (`report_swap` is never called).
//!
//! With the Direct3D 11 renderer (M12, `render.renderer`) mpv draws into a
//! native window of its own (`cpp/video_window.cpp`): no render context
//! then, and the video item stays empty under that window.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use rvp_engine::{Engine, libmpv2};

use crate::ffi::bridge::{rvp_gl_get_proc_address, rvp_video_request_update};

/// The player engine; never dropped (see the module comment and
/// `main::finish` for the shutdown order).
pub static ENGINE: OnceLock<Engine> = OnceLock::new();
/// Set once the render context exists, or at once when mpv draws into the
/// Direct3D 11 renderer's window.
pub static READY: AtomicBool = AtomicBool::new(false);
/// Set when mpv draws into the Direct3D 11 renderer's window (D4): the
/// video item then makes no render context and draws nothing.
static WINDOW: AtomicBool = AtomicBool::new(false);
/// Set when the window closes: only then may the video item's renderer
/// free the render context.
pub static CLOSING: AtomicBool = AtomicBool::new(false);
/// Set once the window presented its first frame.
static PRESENTED: AtomicBool = AtomicBool::new(false);
/// Renders attempted without a render context.
static WAITED: AtomicU32 = AtomicU32::new(0);
/// Called on the render thread once the render context exists.
static ON_READY: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
/// Video frames drawn since [`take_drawn`] last looked, when the last one
/// was (microseconds since [`EPOCH`]) and the longest time between two.
static DRAWN: AtomicU32 = AtomicU32::new(0);
static LAST_DRAWN: AtomicU64 = AtomicU64::new(0);
static LONGEST_GAP: AtomicU64 = AtomicU64::new(0);
static EPOCH: OnceLock<Instant> = OnceLock::new();

thread_local! {
    /// Lives on the render thread only.
    static CONTEXT: RefCell<Option<RenderContext<'static>>> = const { RefCell::new(None) };
}

/// Sets what runs (on the render thread) once the render context exists.
pub fn on_ready(callback: impl Fn() + Send + Sync + 'static) {
    let _ = ON_READY.set(Box::new(callback));
}

fn get_proc_address(_context: &(), name: &str) -> *mut c_void {
    // An address from the GL driver, passed through as an opaque pointer.
    rvp_gl_get_proc_address(name) as *mut c_void
}

fn create(engine: &'static Engine) -> Option<RenderContext<'static>> {
    let created = engine.mpv().create_render_context([
        RenderParam::ApiType(RenderParamApiType::OpenGl),
        RenderParam::InitParams(OpenGLInitParams {
            get_proc_address,
            ctx: (),
        }),
    ]);
    match created {
        Ok(mut context) => {
            context.set_update_callback(rvp_video_request_update);
            Some(context)
        }
        Err(error) => {
            rvp_app::log::report(
                rvp_app::log::Level::Critical,
                &format!("cannot create the mpv render context: {error}"),
            );
            None
        }
    }
}

/// mpv draws into its own window from the start (before the engine
/// exists): files may load at once.
pub fn use_window() {
    WINDOW.store(true, Ordering::Release);
    READY.store(true, Ordering::Release);
}

/// True when mpv draws into the Direct3D 11 renderer's window.
pub fn in_window() -> bool {
    WINDOW.load(Ordering::Acquire)
}

/// The window presented a frame (render thread).
pub fn presented() {
    PRESENTED.store(true, Ordering::Release);
}

/// Draws mpv's current frame into the framebuffer object `fbo`; false when
/// nothing was drawn (the caller clears the framebuffer).
///
/// The render context is created only after the window presented its first
/// frame, so the skin shows before libmpv sets up OpenGL. Windows that are
/// never presented (grabbed while hidden, Windows session 0) get it on the
/// third attempt.
pub fn render(fbo: i32, width: i32, height: i32) -> bool {
    let Some(engine) = ENGINE.get().filter(|_| !in_window()) else {
        return false;
    };
    CONTEXT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            if !PRESENTED.load(Ordering::Acquire) && WAITED.fetch_add(1, Ordering::AcqRel) < 2 {
                rvp_video_request_update();
                return false;
            }
            *slot = create(engine);
            if slot.is_some() {
                crate::metrics::mark("render_context_ms");
                READY.store(true, Ordering::Release);
                if let Some(callback) = ON_READY.get() {
                    callback();
                }
            }
        }
        let rendered = slot
            .as_ref()
            .is_some_and(|context| context.render::<()>(fbo, width, height, false).is_ok());
        if rendered {
            crate::metrics::video_rendered();
            count_drawn();
        }
        rendered
    })
}

/// Counts a drawn frame for [`take_drawn`] (render thread).
fn count_drawn() {
    let now = u64::try_from(EPOCH.get_or_init(Instant::now).elapsed().as_micros()).unwrap_or(0);
    let last = LAST_DRAWN.swap(now, Ordering::Relaxed);
    if last != 0 {
        LONGEST_GAP.fetch_max(now.saturating_sub(last), Ordering::Relaxed);
    }
    DRAWN.fetch_add(1, Ordering::Relaxed);
}

/// The video frames the window drew since the last call and the longest
/// time between two of them in milliseconds (the log's seconds of
/// playback); `None` when mpv draws into a window of its own. While
/// nothing plays, the next frame starts the gaps again.
pub fn take_drawn(playing: bool) -> Option<(u32, u32)> {
    if in_window() {
        return None;
    }
    if !playing {
        LAST_DRAWN.store(0, Ordering::Relaxed);
    }
    let gap = LONGEST_GAP.swap(0, Ordering::Relaxed) / 1000;
    Some((
        DRAWN.swap(0, Ordering::Relaxed),
        u32::try_from(gap).unwrap_or(u32::MAX),
    ))
}

/// The window thread's beat for the log's seconds at Debug: a 20 ms timer
/// of `Main.qml` ([`BEAT_MS`]) calls [`beat`] while a file plays, and the
/// longest time between two beats shows how long the thread was held (the
/// owner's log of 2026-11-12: a frame dropped in each second while
/// neither the processors nor the card were busy).
static LAST_BEAT: AtomicU64 = AtomicU64::new(0);
static LONGEST_BEAT: AtomicU64 = AtomicU64::new(0);

/// The beat timer's interval in milliseconds.
pub const BEAT_MS: u64 = 20;

/// A beat of the window thread's timer.
pub fn beat() {
    let now = u64::try_from(EPOCH.get_or_init(Instant::now).elapsed().as_micros()).unwrap_or(0);
    let last = LAST_BEAT.swap(now, Ordering::Relaxed);
    if last != 0 {
        LONGEST_BEAT.fetch_max(now.saturating_sub(last), Ordering::Relaxed);
    }
}

/// How long the window thread was held beyond the beat's interval at most
/// since the last call, in milliseconds; `None` when the timer did not run
/// (the next beat starts again).
pub fn take_pause(beating: bool) -> Option<u32> {
    let longest = LONGEST_BEAT.swap(0, Ordering::Relaxed) / 1000;
    if !beating {
        LAST_BEAT.store(0, Ordering::Relaxed);
        return None;
    }
    Some(u32::try_from(longest.saturating_sub(BEAT_MS)).unwrap_or(u32::MAX))
}

/// The video item's renderer is going away. Qt also builds and drops
/// renderers for window grabs while the window is hidden (the self-test in
/// Windows session 0); `vo=libmpv` cannot take a new render context for the
/// file that plays, so the context is freed only when the window closes,
/// before the GL context goes away.
pub fn release() {
    if !CLOSING.load(Ordering::Acquire) {
        return;
    }
    CONTEXT.with(|slot| {
        slot.borrow_mut().take();
    });
}
