//! libmpv in a Qt Quick scene: the controller owns the player, mpv's wakeup
//! callback queues event handling onto the Qt thread, and the C++ video
//! item calls `init_gl`/`render` on the scene graph render thread.

use std::cell::RefCell;
use std::ffi::c_void;
use std::pin::Pin;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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
        #[qproperty(i32, mistimed_frame_count)]
        #[qproperty(i32, decoder_frame_drop_count)]
        #[qproperty(f64, estimated_display_fps)]
        #[qproperty(f64, vsync_jitter)]
        #[qproperty(f64, display_fps)]
        #[qproperty(QString, current_vo)]
        type MpvController = super::MpvControllerRust;

        #[qinvokable]
        fn start(self: Pin<&mut Self>, source: &QString, hwdec: &QString);

        /// mpv in a native child window (`wid`) with its own video output,
        /// for comparison with the OpenGL item.
        #[qinvokable]
        fn start_wid(self: Pin<&mut Self>, source: &QString, hwdec: &QString, wid: i64);

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
    mistimed_frame_count: i32,
    decoder_frame_drop_count: i32,
    estimated_display_fps: f64,
    vsync_jitter: f64,
    display_fps: f64,
    current_vo: QString,
}

/// The player, leaked so the render thread can borrow it for the lifetime
/// of the process (a spike shortcut).
static MPV: OnceLock<&'static Mpv> = OnceLock::new();
static FRAMES: AtomicU64 = AtomicU64::new(0);
/// `vo=libmpv` needs the render context before a file starts, so the first
/// file is loaded only after the render thread created it.
static PENDING: Mutex<Option<String>> = Mutex::new(None);
/// Set in `wid` mode: mpv renders itself and the OpenGL item stays idle.
static WID_MODE: AtomicBool = AtomicBool::new(false);
/// Set by `render` when mpv drew a frame; `report_swap` reports it once.
static SWAP_PENDING: AtomicBool = AtomicBool::new(false);

/// Property values read by a worker thread. The Qt thread never calls a
/// blocking mpv function: with `wid` on Windows, mpv's video thread sends
/// window messages to the child window, which belongs to the Qt thread, so
/// a Qt thread waiting on mpv could deadlock.
#[derive(Clone, Default)]
struct Snapshot {
    frame_drop: i32,
    vo_delayed: i32,
    mistimed: i32,
    decoder_drop: i32,
    estimated_display_fps: f64,
    vsync_jitter: f64,
    display_fps: f64,
    vf_fps: f64,
    time_pos: f64,
    vo: String,
    hwdec: String,
}

static SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);

fn read_snapshot(mpv: &Mpv) -> Snapshot {
    let count = |name: &str| {
        mpv.get_property::<i64>(name)
            .ok()
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(-1)
    };
    let float = |name: &str| mpv.get_property::<f64>(name).unwrap_or(-1.0);
    let text = |name: &str| mpv.get_property::<String>(name).unwrap_or_default();
    Snapshot {
        frame_drop: count("frame-drop-count"),
        vo_delayed: count("vo-delayed-frame-count"),
        mistimed: count("mistimed-frame-count"),
        decoder_drop: count("decoder-frame-drop-count"),
        estimated_display_fps: float("estimated-display-fps"),
        vsync_jitter: float("vsync-jitter"),
        display_fps: float("display-fps"),
        vf_fps: float("estimated-vf-fps"),
        time_pos: float("time-pos"),
        vo: text("current-vo"),
        hwdec: text("hwdec-current"),
    }
}

/// Loads `first` (in `wid` mode) and then keeps SNAPSHOT up to date.
fn start_poller(mpv: &'static Mpv, first: Option<String>) {
    std::thread::spawn(move || {
        if let Some(source) = first {
            let _ = mpv.command("loadfile", &[&source]);
        }
        loop {
            let snapshot = read_snapshot(mpv);
            if let Ok(mut slot) = SNAPSHOT.lock() {
                *slot = Some(snapshot);
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
}

thread_local! {
    /// The render context lives on the scene graph render thread only.
    static RENDER: RefCell<Option<RenderContext<'static>>> = const { RefCell::new(None) };
}

impl qobject::MpvController {
    fn start(self: Pin<&mut Self>, source: &QString, hwdec: &QString) {
        self.create(source, hwdec, None);
    }

    fn start_wid(self: Pin<&mut Self>, source: &QString, hwdec: &QString, wid: i64) {
        WID_MODE.store(true, Ordering::Relaxed);
        self.create(source, hwdec, Some(wid));
    }

    fn create(mut self: Pin<&mut Self>, source: &QString, hwdec: &QString, wid: Option<i64>) {
        let source = source.to_string();
        if let Some(mpv) = MPV.get() {
            let _ = mpv.command("loadfile", &[&source]);
            return;
        }
        crate::ffi::bridge::rvp_set_c_numeric_locale();
        let hwdec = hwdec.to_string();
        let created = Mpv::with_initializer(|init| {
            match wid {
                Some(wid) => init.set_option("wid", wid)?,
                None => init.set_option("vo", "libmpv")?,
            }
            init.set_option("hwdec", hwdec.as_str())?;
            // Extra options for experiments: RVP_MPV_OPTIONS="a=b,c=d".
            for pair in std::env::var("RVP_MPV_OPTIONS").unwrap_or_default().split(',') {
                if let Some((key, value)) = pair.split_once('=') {
                    init.set_option(key.trim(), value.trim())?;
                }
            }
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
        if wid.is_some() {
            // mpv owns its video output; nothing waits for a render context.
            start_poller(mpv, Some(source));
        } else {
            if let Ok(mut pending) = PENDING.lock() {
                *pending = Some(source);
            }
            start_poller(mpv, None);
            crate::ffi::bridge::rvp_mpv_request_update();
        }
        self.as_mut().set_status(QString::from("starting"));
    }

    fn refresh(mut self: Pin<&mut Self>) {
        let frames = i32::try_from(FRAMES.load(Ordering::Relaxed)).unwrap_or(i32::MAX);
        self.as_mut().set_frames_rendered(frames);
        let snapshot = SNAPSHOT.lock().ok().and_then(|slot| slot.clone());
        if let Some(snapshot) = snapshot {
            self.as_mut().set_frame_drop_count(snapshot.frame_drop);
            self.as_mut().set_vo_delayed_frame_count(snapshot.vo_delayed);
            self.as_mut().set_mistimed_frame_count(snapshot.mistimed);
            self.as_mut().set_decoder_frame_drop_count(snapshot.decoder_drop);
            self.as_mut().set_estimated_display_fps(snapshot.estimated_display_fps);
            self.as_mut().set_display_fps(snapshot.display_fps);
            self.as_mut().set_vsync_jitter(snapshot.vsync_jitter);
            self.as_mut().set_current_vo(QString::from(&snapshot.vo));
            self.as_mut().set_hwdec_current(QString::from(&snapshot.hwdec));
            if snapshot.vf_fps >= 0.0 {
                self.as_mut().set_estimated_vf_fps(snapshot.vf_fps);
            }
            if snapshot.time_pos >= 0.0 {
                self.as_mut().set_time_pos(snapshot.time_pos);
            }
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
    if WID_MODE.load(Ordering::Relaxed) {
        return;
    }
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
            SWAP_PENDING.store(true, Ordering::Relaxed);
        }
    });
}

/// Tells mpv that the frame drawn by `render` was presented, so it can time
/// frames by real swaps. The C++ video item calls this after
/// `QQuickWindow::frameSwapped` when `RVP_REPORT_SWAP=1`, on the same thread
/// as `render`; swaps without a new video frame are not reported.
pub fn report_swap() {
    if !SWAP_PENDING.swap(false, Ordering::Relaxed) {
        return;
    }
    RENDER.with(|slot| {
        if let Some(context) = slot.borrow().as_ref() {
            context.report_swap();
        }
    });
}
