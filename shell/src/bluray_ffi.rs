//! The C++ side of Blu-ray menus (M16, `cpp/bluray.cpp`): libbluray, loaded
//! at run time. `crate::disc::bluray` holds the logic.

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). The C++ functions
// take references, slices, string slices and plain values that cxx checks
// or passes by value; `BlurayNavigator` is an owned C++ object behind a
// `UniquePtr`, whose functions libbluray allows from any thread (it locks
// itself), one at a time as `crate::disc::bluray` keeps them with a mutex.
#![allow(unsafe_code)]

#[cxx_qt::bridge]
pub mod bridge {
    /// One of libbluray's events (`BD_EVENT`).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct BdEvent {
        event: u32,
        param: u32,
    }

    /// What libbluray found on the disc (`BLURAY_DISC_INFO`).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct BdInfo {
        bluray: bool,
        /// The disc can play through its menus.
        menus: bool,
        first_play: bool,
        top_menu: bool,
        hdmv_titles: u32,
        bdj_titles: u32,
        /// BD-J titles can run (Java and libbluray's jar were found).
        bdj_handled: bool,
        java: bool,
        aacs: bool,
        aacs_handled: bool,
    }

    /// One overlay event, copied from libbluray's callback: palette-coded
    /// runs (`data`: length << 16 | colour; `palette`: Y << 24 | Cr << 16 |
    /// Cb << 8 | T) or, for BD-J (`argb`), ARGB pixels.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    struct BdOverlay {
        argb: bool,
        plane: u8,
        command: u8,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        palette: Vec<u32>,
        data: Vec<u32>,
    }

    unsafe extern "C++" {
        include!("rvp-shell/cpp/bluray.h");

        type BlurayNavigator;

        fn rvp_bluray_load(message: &mut String) -> bool;
        fn rvp_bluray_open(
            path: &str,
            storage: &str,
            error: &mut String,
        ) -> UniquePtr<BlurayNavigator>;
        fn rvp_bluray_info(nav: Pin<&mut BlurayNavigator>) -> BdInfo;
        fn rvp_bluray_play(nav: Pin<&mut BlurayNavigator>) -> bool;
        fn rvp_bluray_read(
            nav: Pin<&mut BlurayNavigator>,
            block: &mut [u8],
            events: &mut Vec<BdEvent>,
        ) -> i32;
        fn rvp_bluray_skip_still(nav: Pin<&mut BlurayNavigator>) -> bool;
        fn rvp_bluray_key(nav: Pin<&mut BlurayNavigator>, key: u32) -> bool;
        fn rvp_bluray_mouse(nav: Pin<&mut BlurayNavigator>, x: u16, y: u16, activate: bool)
        -> bool;
        fn rvp_bluray_menu(nav: Pin<&mut BlurayNavigator>) -> bool;
        fn rvp_bluray_seek(nav: Pin<&mut BlurayNavigator>, position: u64) -> i64;
        fn rvp_bluray_tell(nav: Pin<&mut BlurayNavigator>) -> u64;
        fn rvp_bluray_title_size(nav: Pin<&mut BlurayNavigator>) -> u64;
        fn rvp_bluray_overlays(nav: Pin<&mut BlurayNavigator>) -> Vec<BdOverlay>;
    }

    impl UniquePtr<BlurayNavigator> {}
}

// SAFETY: a `BlurayNavigator` owns its libbluray handle alone; libbluray
// locks its own state and takes calls from any thread, which the mutex in
// `crate::disc::bluray` also keeps one at a time.
unsafe impl Send for bridge::BlurayNavigator {}
