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
        /// libmpv needs `LC_NUMERIC=C`; Qt sets the user locale on Unix.
        fn rvp_set_c_numeric_locale();
        /// `QOpenGLContext::getProcAddress` of the current context.
        fn rvp_gl_get_proc_address(name: &str) -> usize;
        /// Schedules a video frame (mpv's render update callback; any
        /// thread).
        fn rvp_video_request_update();

        /// True when an atlas image of this name is cached on the C++ side.
        fn rvp_atlas_contains(name: &str) -> bool;
        /// Caches an RGBA atlas image (straight alpha, rows without padding).
        fn rvp_atlas_insert(name: &str, rgba: &[u8], width: i32, height: i32);
        /// Tiles `src` of an atlas over `dst`, clipped to `clip` (each
        /// rectangle as x, y, width, height).
        fn rvp_paint_atlas(
            painter: Pin<&mut QPainter>,
            name: &str,
            src: [i32; 4],
            dst: [i32; 4],
            clip: [i32; 4],
        );
        fn rvp_paint_translate(painter: Pin<&mut QPainter>, dx: i32, dy: i32);
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
