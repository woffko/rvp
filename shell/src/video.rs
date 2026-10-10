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

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use rvp_engine::{Engine, libmpv2};

use crate::ffi::bridge::{rvp_gl_get_proc_address, rvp_video_request_update};

/// The player engine; never dropped (see the module comment and
/// `main::finish` for the shutdown order).
pub static ENGINE: OnceLock<Engine> = OnceLock::new();
/// Set once the render context exists.
pub static READY: AtomicBool = AtomicBool::new(false);
/// Set when the window closes: only then may the video item's renderer
/// free the render context.
pub static CLOSING: AtomicBool = AtomicBool::new(false);
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

/// Draws mpv's current frame into the framebuffer object `fbo`.
pub fn render(fbo: i32, width: i32, height: i32) {
    let Some(engine) = ENGINE.get() else {
        return;
    };
    CONTEXT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = create(engine);
            if slot.is_some() {
                READY.store(true, Ordering::Release);
                if let Some(callback) = ON_READY.get() {
                    callback();
                }
            }
        }
        if let Some(context) = slot.as_ref()
            && context.render::<()>(fbo, width, height, false).is_ok()
        {
            crate::metrics::video_rendered();
        }
    });
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
