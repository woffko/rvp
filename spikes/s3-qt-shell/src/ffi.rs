//! The plain C++ <-> Rust boundary of the spike: graphics setup, painting
//! helpers and the mpv render hooks called from the C++ video item.

#[cxx_qt::bridge]
pub mod bridge {
    unsafe extern "C++" {
        include!("s3-qt-shell/cpp/spike.h");
        include!("cxx-qt-lib/qpainter.h");
        type QPainter = cxx_qt_lib::QPainter;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        fn rvp_setup_graphics();
        fn rvp_register_native_types();
        fn rvp_set_c_numeric_locale();
        fn rvp_gl_get_proc_address(name: &str) -> usize;
        fn rvp_mpv_request_update();
        #[allow(clippy::too_many_arguments)]
        fn rvp_paint_image(
            painter: Pin<&mut QPainter>,
            image: &QImage,
            sx: i32,
            sy: i32,
            sw: i32,
            sh: i32,
            dx: i32,
            dy: i32,
            dw: i32,
            dh: i32,
            cx: i32,
            cy: i32,
            cw: i32,
            ch: i32,
        );
        fn rvp_paint_translate(painter: Pin<&mut QPainter>, dx: i32, dy: i32);
        fn rvp_paint_fill(painter: Pin<&mut QPainter>, x: i32, y: i32, w: i32, h: i32, rgba: u32);
        #[allow(clippy::too_many_arguments)]
        fn rvp_paint_text(
            painter: Pin<&mut QPainter>,
            text: &str,
            trailing: &str,
            family: &str,
            pixel_size: i32,
            bold: bool,
            rgba: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            cx: i32,
            cy: i32,
            cw: i32,
            ch: i32,
            h_align: i32,
            v_align: i32,
            elide: bool,
        );
    }

    extern "Rust" {
        fn rvp_mpv_init_gl();
        fn rvp_mpv_render(fbo: i32, width: i32, height: i32);
        fn rvp_mpv_report_swap();
    }
}

fn rvp_mpv_init_gl() {
    crate::mpv::init_gl();
}

fn rvp_mpv_render(fbo: i32, width: i32, height: i32) {
    crate::mpv::render(fbo, width, height);
}

fn rvp_mpv_report_swap() {
    crate::mpv::report_swap();
}
