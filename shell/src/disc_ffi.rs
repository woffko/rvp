//! The C++ side of DVD menus (M16, `cpp/disc.cpp`): libdvdnav, loaded at
//! run time. `crate::disc` holds the logic.

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). The C++ functions
// take references, slices, string slices and plain values that cxx checks
// or passes by value; `DvdNavigator` is an owned C++ object behind a
// `UniquePtr`, whose functions libdvdnav allows from any thread as long as
// one call runs at a time, which `crate::disc` keeps with a mutex.
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod bridge {
    /// Where the navigator is, as `rvp_dvdnav_status` reads it.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct DvdStatus {
        /// libdvdnav's domain: 1 first play, 2 a title, 4 the video
        /// manager's menus, 8 a title set's menus.
        domain: i32,
        titles: i32,
        /// 0 in a menu.
        title: i32,
        part: i32,
        /// The selected button (0 none).
        button: i32,
        angle: i32,
        angles: i32,
        /// The VM's physical audio stream, -1 none.
        audio: i8,
        /// The VM's physical sub-picture stream, -1 none (0x80: forced
        /// only).
        spu: i8,
        /// Sectors into the title, and its length.
        position: u32,
        length: u32,
    }

    /// The selected button's rectangle (frame pixels, ends included) and
    /// its palette (colours in bits 16 to 31, contrast in 0 to 15).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct DvdButton {
        valid: bool,
        button: i32,
        palette: u32,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
    }

    unsafe extern "C++" {
        include!("rvp-shell/cpp/disc.h");

        type DvdNavigator;

        fn rvp_dvdnav_load(message: &mut String) -> bool;
        fn rvp_dvdnav_open(
            path: &str,
            menu: &str,
            audio: &str,
            spu: &str,
            error: &mut String,
        ) -> UniquePtr<DvdNavigator>;
        fn rvp_dvdnav_next(nav: Pin<&mut DvdNavigator>, block: &mut [u8], length: &mut i32) -> i32;
        fn rvp_dvdnav_still_skip(nav: Pin<&mut DvdNavigator>) -> bool;
        fn rvp_dvdnav_wait_skip(nav: Pin<&mut DvdNavigator>) -> bool;
        fn rvp_dvdnav_status(nav: Pin<&mut DvdNavigator>) -> DvdStatus;
        fn rvp_dvdnav_button(nav: Pin<&mut DvdNavigator>, mode: i32) -> DvdButton;
        fn rvp_dvdnav_select(nav: Pin<&mut DvdNavigator>, direction: i32) -> bool;
        fn rvp_dvdnav_activate(nav: Pin<&mut DvdNavigator>) -> bool;
        fn rvp_dvdnav_mouse(nav: Pin<&mut DvdNavigator>, x: i32, y: i32, activate: bool) -> bool;
        fn rvp_dvdnav_menu(nav: Pin<&mut DvdNavigator>, menu: i32) -> bool;
        fn rvp_dvdnav_go_up(nav: Pin<&mut DvdNavigator>) -> bool;
        fn rvp_dvdnav_sector_search(nav: Pin<&mut DvdNavigator>, sector: i64) -> bool;
        fn rvp_dvdnav_error(nav: Pin<&mut DvdNavigator>) -> String;
    }

    impl UniquePtr<DvdNavigator> {}
}

// SAFETY: a `DvdNavigator` owns its libdvdnav handle alone; libdvdnav's
// calls may come from any thread one at a time, which the mutex in
// `crate::disc` keeps (mpv's demuxer thread reads, the window's thread
// navigates).
unsafe impl Send for bridge::DvdNavigator {}
