//! libmpv in a Qt Quick scene: the controller owns the player, mpv's wakeup
//! callback queues event handling onto the Qt thread, and the C++ video
//! item calls `init_gl`/`render` on the scene graph render thread.

use std::cell::RefCell;
use std::ffi::c_void;
use std::pin::Pin;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

use cxx_qt::Threading;
use cxx_qt_lib::QString;
use libmpv2::events::Event;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2::Mpv;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, status)]
        #[qproperty(f64, estimated_vf_fps)]
        #[qproperty(f64, time_pos)]
        #[qproperty(i32, frames_rendered)]
        #[qproperty(i32, events_on_qt_thread)]
        #[qproperty(i32, frame_drop_count)]
        #[qproperty(i32, vo_delayed_frame_count)]
        #[qproperty(QString, hwdec_current)]
        type MpvController = super::MpvControllerRust;

        #[qinvokable]
        fn start(self: Pin<&mut Self>, source: &QString, hwdec: &QString);

        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for MpvController {}
}

#[derive(Default)]
pub struct MpvControllerRust {
    status: QString,
    estimated_vf_fps: f64,
    time_pos: f64,
    frames_rendered: i32,
    events_on_qt_thread: i32,
    frame_drop_count: i32,
    vo_delayed_frame_count: i32,
    hwdec_current: QString,
}

/// The player, leaked so the render thread can borrow it for the lifetime
/// of the process (a spike shortcut).
static MPV: OnceLock<&'static Mpv> = OnceLock::new();
static FRAMES: AtomicU64 = AtomicU64::new(0);
/// `vo=libmpv` needs the render context before a file starts, so the first
/// file is loaded only after the render thread created it.
static PENDING: Mutex<Option<String>> = Mutex::new(None);

thread_local! {
    /// The render context lives on the scene graph render thread only.
    static RENDER: RefCell<Option<RenderContext<'static>>> = const { RefCell::new(None) };
}

impl qobject::MpvController {
    fn start(mut self: Pin<&mut Self>, source: &QString, hwdec: &QString) {
        let source = source.to_string();
        if let Some(mpv) = MPV.get() {
            let _ = mpv.command("loadfile", &[&source]);
            return;
        }
        crate::ffi::bridge::rvp_set_c_numeric_locale();
        let hwdec = hwdec.to_string();
        let created = Mpv::with_initializer(|init| {
            init.set_option("vo", "libmpv")?;
            init.set_option("hwdec", hwdec.as_str())?;
            init.set_option("loop-file", "inf")?;
            init.set_option("keep-open", "yes")?;
            init.set_option("terminal", "yes")?;
            let level = std::env::var("RVP_MPV_MSG_LEVEL").unwrap_or_else(|_| "all=warn".to_owned());
            init.set_option("msg-level", level.as_str())?;
            Ok(())
        });
        let mut mpv = match created {
            Ok(mpv) => mpv,
            Err(error) => {
                self.as_mut()
                    .set_status(QString::from(&format!("mpv failed: {error}")));
                return;
            }
        };
        let qt_thread = self.qt_thread();
        // Called on an mpv thread: only queue work, never call mpv here.
        mpv.set_wakeup_callback(move || {
            let _ = qt_thread.queue(|controller| controller.drain_events());
        });
        // No observe_property: libmpv2 6.0's wait_event returns None both
        // for an empty queue and for a change of a property that has no value
        // yet, so a drain loop would stop early and the wakeup callback would
        // not fire again. Properties are polled in refresh() instead.
        let mpv: &'static Mpv = Box::leak(Box::new(mpv));
        let _ = MPV.set(mpv);
        if let Ok(mut pending) = PENDING.lock() {
            *pending = Some(source);
        }
        crate::ffi::bridge::rvp_mpv_request_update();
        self.as_mut().set_status(QString::from("starting"));
    }

    fn refresh(mut self: Pin<&mut Self>) {
        let frames = i32::try_from(FRAMES.load(Ordering::Relaxed)).unwrap_or(i32::MAX);
        self.as_mut().set_frames_rendered(frames);
        if let Some(mpv) = MPV.get() {
            let count = |name: &str| {
                mpv.get_property::<i64>(name)
                    .ok()
                    .and_then(|value| i32::try_from(value).ok())
                    .unwrap_or(-1)
            };
            self.as_mut().set_frame_drop_count(count("frame-drop-count"));
            self.as_mut()
                .set_vo_delayed_frame_count(count("vo-delayed-frame-count"));
            let hwdec = mpv.get_property::<String>("hwdec-current").unwrap_or_default();
            if let Ok(value) = mpv.get_property::<f64>("estimated-vf-fps") {
                self.as_mut().set_estimated_vf_fps(value);
            }
            if let Ok(value) = mpv.get_property::<f64>("time-pos") {
                self.as_mut().set_time_pos(value);
            }
            self.as_mut().set_hwdec_current(QString::from(&hwdec));
        }
    }

    /// Runs on the Qt thread, queued by the wakeup callback.
    fn drain_events(mut self: Pin<&mut Self>) {
        let Some(mpv) = MPV.get() else {
            return;
        };
        let count = self.events_on_qt_thread().saturating_add(1);
        self.as_mut().set_events_on_qt_thread(count);
        while let Some(event) = mpv.wait_event(0.0) {
            match event {
                Ok(Event::FileLoaded) => self.as_mut().set_status(QString::from("playing")),
                Ok(Event::EndFile(reason)) => self
                    .as_mut()
                    .set_status(QString::from(&format!("end of file ({reason})"))),
                Ok(_) => {}
                Err(error) => self
                    .as_mut()
                    .set_status(QString::from(&format!("event error: {error}"))),
            }
        }
        self.refresh();
    }
}

fn get_proc_address(_context: &(), name: &str) -> *mut c_void {
    crate::ffi::bridge::rvp_gl_get_proc_address(name) as *mut c_void
}

/// Creates the render context on the render thread, once mpv exists.
pub fn init_gl() {
    let Some(mpv) = MPV.get() else {
        return;
    };
    RENDER.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let created = mpv.create_render_context([
            RenderParam::ApiType(RenderParamApiType::OpenGl),
            RenderParam::InitParams(OpenGLInitParams {
                get_proc_address,
                ctx: (),
            }),
        ]);
        match created {
            Ok(mut context) => {
                context.set_update_callback(crate::ffi::bridge::rvp_mpv_request_update);
                *slot.borrow_mut() = Some(context);
                println!("mpv_render_context=created");
                let source = PENDING.lock().ok().and_then(|mut pending| pending.take());
                if let Some(source) = source {
                    let _ = mpv.command("loadfile", &[&source]);
                }
            }
            Err(error) => println!("mpv_render_context=failed {error}"),
        }
    });
}

/// Renders the current video frame into the item's framebuffer object.
pub fn render(fbo: i32, width: i32, height: i32) {
    init_gl();
    RENDER.with(|slot| {
        if let Some(context) = slot.borrow().as_ref()
            && context.render::<()>(fbo, width, height, false).is_ok()
        {
            FRAMES.fetch_add(1, Ordering::Relaxed);
        }
    });
}
