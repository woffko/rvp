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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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
            eprintln!("rvp: cannot create the mpv render context: {error}");
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
        }
        rendered
    })
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
