//! The plain C++ <-> Rust boundary: graphics setup, the skin painter's
//! drawing calls, and the hooks the C++ video item calls on the render
//! thread.
//!
//! This file holds only the bridge and three forwarding functions, so the
//! `unsafe` the generated code needs is allowed for the file (cxx-qt accepts
//! no attributes on the bridge module itself).

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). Every C++ function
// declared here takes only references, string slices, arrays and plain
// integers, which cxx passes by value or as checked references; none keeps
// a pointer past the call.
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod bridge {
    unsafe extern "C++" {
        include!("rvp-shell/cpp/shell.h");
        include!("cxx-qt-lib/qpainter.h");
        type QPainter = cxx_qt_lib::QPainter;

        /// Selects the OpenGL scene graph backend (D4); before the
        /// application object exists.
        fn rvp_setup_graphics();
        /// Registers `MpvItem` and the `Tools` singleton for QML.
        fn rvp_register_native_types();
        /// True when QML created a window (false when Main.qml failed).
        fn rvp_has_windows() -> bool;
        /// Sends one request line to the instance listening on `name`; true
        /// when it took the request.
        fn rvp_instance_send(name: &str, message: &str) -> bool;
        /// Listens for other instances on `name`; false when that failed.
        fn rvp_instance_listen(name: &str) -> bool;
        /// Whether an instance listens on `name` (connects and leaves).
        fn rvp_instance_running(name: &str) -> bool;
        /// libmpv needs `LC_NUMERIC=C`; Qt sets the user locale on Unix.
        fn rvp_set_c_numeric_locale();
        /// Gives every window RVP's icon (`QGuiApplication::setWindowIcon`).
        fn rvp_set_window_icon();
        /// The main window's native handle (`QWindow::winId`), 0 before it
        /// exists; Windows' media controls belong to it.
        fn rvp_main_window_id() -> usize;
        /// D4's Direct3D 11 renderer: makes the native video window (a
        /// disabled child of the main window, on a thread of its own) and
        /// returns its handle for mpv's `wid`; 0 when it cannot (or outside
        /// Windows). `Tools.place_video_window` places it.
        fn rvp_video_window_create() -> usize;
        /// Closes the video window and ends its thread (after mpv quit).
        fn rvp_video_window_close();
        /// Keeps the mouse pointer on the main window's display (`true`) or
        /// lets it go (Windows' ClipCursor; nothing elsewhere).
        fn rvp_confine_cursor(on: bool);
        /// Crashes on purpose (`--crash-test=native`): a write through a
        /// null pointer, which the native crash handler reports.
        fn rvp_crash_test();
        /// Sets an environment variable (`qputenv`); only while no other
        /// thread runs.
        fn rvp_set_env(name: &str, value: &str);
        /// Windows' taskbar (M14's General page, `cpp/taskbar.cpp`): its
        /// thumbnail buttons and the screen lock notices (once the main
        /// window exists; `enabled`: "Enable enhanced taskbar features"),
        /// the progress (`state` 0 idle, 1 playing, 2 paused; `progress`
        /// 0 to 1, below 0 for none), the jump list's recent files.
        /// Nothing elsewhere.
        fn rvp_taskbar_start(enabled: bool);
        fn rvp_taskbar_state(state: i32, progress: f64);
        fn rvp_taskbar_recent(path: &str);
        /// "Turn on global media control keys": Windows' hotkeys for the
        /// media keys on RVP's main window (`Tools::mediaKey`). Nothing
        /// elsewhere.
        fn rvp_global_media_keys(on: bool);
        /// Windows' Run key for "Automatically run at system startup";
        /// false when it could not be changed (and outside Windows).
        fn rvp_run_at_startup(on: bool, command: &str) -> bool;
        /// The current user's registry (`cpp/registry.cpp`, M14's
        /// Association page): a string value (`name` empty: the default),
        /// a value or a key removed, whether a key exists, and the shell
        /// told that types changed. False (and nothing) outside Windows.
        fn rvp_registry_set(key: &str, name: &str, value: &str) -> bool;
        fn rvp_registry_delete_value(key: &str, name: &str) -> bool;
        fn rvp_registry_delete_key(key: &str) -> bool;
        fn rvp_registry_has_key(key: &str) -> bool;
        fn rvp_associations_changed();
        /// Windows' speech (`cpp/tts.cpp`, M14's Accessibility page): the
        /// voices' and outputs' names (JSON lists, empty elsewhere), and
        /// a text read aloud in place of the one being read (`rate` -10
        /// to 10, `volume` 0 to 100); false when it cannot.
        fn rvp_tts_voices() -> String;
        fn rvp_tts_outputs() -> String;
        fn rvp_tts_state() -> String;
        fn rvp_tts_speak(text: &str, voice: &str, output: &str, rate: i32, volume: i32) -> bool;
        /// The Xbox controller (`cpp/xinput.cpp`): polled while `enabled`,
        /// only with RVP active when `focus_only`, the pointer moved by
        /// `stick` (0 none, 1 left, 2 right).
        fn rvp_xinput_start(enabled: bool, focus_only: bool, stick: i32);
        /// `QOpenGLContext::getProcAddress` of the current context.
        fn rvp_gl_get_proc_address(name: &str) -> usize;
        /// Schedules a video frame (mpv's render update callback; any
        /// thread).
        fn rvp_video_request_update();
        /// Every graphics card's power for the GPU power budget, a line
        /// each: `unit<TAB>draw<TAB>rated<TAB>name`, the unit `W` or `%`
        /// (`cpp/gpu_power.cpp`: NVML unless `nvidia` is false, and on
        /// Windows AMD's cards through D3DKMT). Only the budget's reader
        /// threads call it.
        fn rvp_gpu_power_read(nvidia: bool) -> String;
        /// Lets NVML go (a reader ends).
        fn rvp_gpu_power_close();
        /// On Windows the processor's name and each graphics adapter with
        /// its vendor and device IDs and its driver's version (the
        /// smoothing calibration's key); empty elsewhere.
        fn rvp_hardware() -> String;

        /// True when an atlas image of this name is cached on the C++ side.
        fn rvp_atlas_contains(name: &str) -> bool;
        /// Drops the cached atlas images whose names start with `prefix`
        /// (thumbnails no longer shown).
        fn rvp_atlas_remove_prefix(prefix: &str);
        /// Caches an RGBA atlas image (straight alpha, rows without padding).
        fn rvp_atlas_insert(name: &str, rgba: &[u8], width: i32, height: i32);
        /// Writes an RGBA picture to `path` (the format from the extension;
        /// `quality` for JPEG), with `texts` (JSON, `rvp_app::capture::SheetText`)
        /// drawn over it; any thread. False when it could not.
        fn rvp_save_image(
            rgba: &[u8],
            width: i32,
            height: i32,
            path: &str,
            quality: i32,
            texts: &str,
        ) -> bool;
        /// Puts an RGBA picture on the clipboard (the window's thread).
        fn rvp_copy_image(rgba: &[u8], width: i32, height: i32) -> bool;
        /// The local date and time as `YYYY-MM-DD HH.MM.SS` (file names).
        fn rvp_local_time() -> String;
        /// The local time's offset from UTC in seconds (the log's times).
        fn rvp_utc_offset() -> i32;
        /// A text's width in pixels in the font `rvp_paint_text` draws it
        /// with (the skins' `AutoSize`).
        fn rvp_text_width(text: &str, family: &str, pixel_size: i32, bold: bool) -> i32;
        /// Tiles `src` of an atlas over `dst`, clipped to `clip` (each
        /// rectangle as x, y, width, height).
        fn rvp_paint_atlas(
            painter: Pin<&mut QPainter>,
            name: &str,
            src: [i32; 4],
            dst: [i32; 4],
            clip: [i32; 4],
        );
        /// Scales `src` of an atlas to `dst`, clipped to `clip` (M9: VLC's
        /// stretched images).
        fn rvp_paint_stretch(
            painter: Pin<&mut QPainter>,
            name: &str,
            src: [i32; 4],
            dst: [i32; 4],
            clip: [i32; 4],
        );
        /// Adds a font file's faces to the application's fonts (M9: VLC's
        /// themes bring theirs); false when Qt cannot read it.
        fn rvp_add_font(data: &[u8]) -> bool;
        fn rvp_paint_translate(painter: Pin<&mut QPainter>, dx: i32, dy: i32);
        /// Draws larger by `factor` from here (M14's skin zoom).
        fn rvp_paint_scale(painter: Pin<&mut QPainter>, factor: f64);
        fn rvp_paint_fill(painter: Pin<&mut QPainter>, rect: [i32; 4], rgba: u32);
        /// Draws a text: `align` is horizontal 0/1/2 (left, centre, right)
        /// plus 3 times vertical 0/1/2 (top, centre, bottom).
        #[allow(clippy::too_many_arguments)]
        fn rvp_paint_text(
            painter: Pin<&mut QPainter>,
            text: &str,
            trailing: &str,
            family: &str,
            pixel_size: i32,
            bold: bool,
            rgba: u32,
            rect: [i32; 4],
            clip: [i32; 4],
            align: i32,
            elide: bool,
        );
    }

    extern "Rust" {
        /// Renders mpv's current frame into the video item's framebuffer
        /// (render thread); false when nothing was drawn.
        fn rvp_video_render(fbo: i32, width: i32, height: i32) -> bool;
        /// Frees the render context (render thread, the video item's
        /// renderer is going away).
        fn rvp_video_release();
        /// A frame was presented (render thread).
        fn rvp_frame_swapped();
    }
}

fn rvp_video_render(fbo: i32, width: i32, height: i32) -> bool {
    crate::video::render(fbo, width, height)
}

fn rvp_video_release() {
    crate::video::release();
}

fn rvp_frame_swapped() {
    crate::video::presented();
    crate::metrics::frame_swapped();
}
