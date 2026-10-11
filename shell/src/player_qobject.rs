//! The `Player` `QObject`'s bridge; the implementation is in `player.rs`.
//!
//! This file holds only the bridge, so the `unsafe` the generated code needs
//! is allowed for the file (cxx-qt accepts no attributes on the bridge
//! module itself).

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). The declarations are
// plain Qt value types and safe methods; nothing here is called unsafely by
// hand.
#![allow(unsafe_code)]

pub use crate::player::PlayerRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qrectf.h");
        type QRectF = cxx_qt_lib::QRectF;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, window_title)]
        #[qproperty(QRectF, video_rect)]
        #[qproperty(i32, region_count)]
        #[qproperty(i32, layout_serial)]
        #[qproperty(QString, tooltip)]
        #[qproperty(i32, cursor_shape)]
        #[qproperty(bool, fullscreen)]
        #[qproperty(bool, chrome_shown)]
        #[qproperty(QString, message)]
        #[qproperty(bool, has_skin)]
        #[qproperty(bool, has_video)]
        #[qproperty(i32, restore_x)]
        #[qproperty(i32, restore_y)]
        #[qproperty(i32, restore_width)]
        #[qproperty(i32, restore_height)]
        #[qproperty(bool, restore_position)]
        #[qproperty(bool, restore_maximized)]
        #[qproperty(QString, osd)]
        #[qproperty(QString, album_name)]
        type Player = super::PlayerRust;

        /// Loads settings, the skin and libmpv; sets the `restore_*`
        /// properties. Files open once the video item is ready.
        #[qinvokable]
        fn start(self: Pin<&mut Self>);

        /// The window's content size changed.
        #[qinvokable]
        fn resized(self: Pin<&mut Self>, width: i32, height: i32);

        /// The window was maximized or restored, activated or deactivated.
        #[qinvokable]
        fn window_state(self: Pin<&mut Self>, maximized: bool, active: bool);

        /// The window system changed fullscreen on its own (not at RVP's
        /// request).
        #[qinvokable]
        fn window_fullscreen(self: Pin<&mut Self>, fullscreen: bool);

        /// The normal (not maximized, not fullscreen) geometry, for the
        /// resume store.
        #[qinvokable]
        fn window_geometry(
            self: Pin<&mut Self>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            has_position: bool,
        );

        /// `modifiers`: `Qt::KeyboardModifiers` (Ctrl and Shift select in
        /// the playlist).
        #[qinvokable]
        fn mouse_press(self: Pin<&mut Self>, x: f64, y: f64, button: i32, modifiers: i32);
        #[qinvokable]
        fn mouse_double(self: Pin<&mut Self>, x: f64, y: f64, button: i32);
        #[qinvokable]
        fn mouse_release(self: Pin<&mut Self>, x: f64, y: f64, button: i32);
        #[qinvokable]
        fn mouse_move(self: Pin<&mut Self>, x: f64, y: f64);
        #[qinvokable]
        fn mouse_wheel(self: Pin<&mut Self>, x: f64, y: f64, delta: i32);
        #[qinvokable]
        fn mouse_leave(self: Pin<&mut Self>);

        /// A key press (`QKeyEvent::key()`, `modifiers()`); true when bound.
        #[qinvokable]
        fn key(self: Pin<&mut Self>, key: i32, modifiers: i32) -> bool;

        /// Runs an action by its settings name (menus, self-test).
        #[qinvokable]
        fn run_action(self: Pin<&mut Self>, name: &QString);

        /// PotPlayer's menu text of an action.
        #[qinvokable]
        fn menu_label(self: &Self, name: &QString) -> QString;

        /// The key shown next to an action in the menu.
        #[qinvokable]
        fn menu_shortcut(self: &Self, name: &QString) -> QString;

        /// Whether a menu entry shows a check mark: -1 none, 0 off, 1 on.
        #[qinvokable]
        fn menu_checked(self: &Self, name: &QString) -> i32;

        /// Runs a playlist action by its settings name (the playlist menu).
        #[qinvokable]
        fn run_list_action(self: Pin<&mut Self>, name: &QString);

        /// PotPlayer's text of a playlist action.
        #[qinvokable]
        fn list_menu_label(self: &Self, name: &QString) -> QString;

        /// The key shown next to a playlist action.
        #[qinvokable]
        fn list_menu_shortcut(self: &Self, name: &QString) -> QString;

        /// Opens files (one path or URL per line).
        #[qinvokable]
        fn open_files(self: Pin<&mut Self>, paths: &QString, append: bool);

        /// Adds a folder's media files to the shown album.
        #[qinvokable]
        fn add_folder(self: Pin<&mut Self>, path: &QString);

        /// Names a new album, or renames the shown one.
        #[qinvokable]
        fn name_album(self: Pin<&mut Self>, name: &QString, rename: bool);

        /// Imports PotPlayer's `.dpl` playlists from a folder (read-only).
        #[qinvokable]
        fn import_playlists(self: Pin<&mut Self>, folder: &QString);

        /// A request from another instance (one JSON line).
        #[qinvokable]
        fn remote_message(self: Pin<&mut Self>, message: &QString);

        /// The window was resized: keep the video's aspect ratio (R1
        /// finding 1) by asking for a size that fits it (`fitWindow`).
        #[qinvokable]
        fn fit_aspect(self: Pin<&mut Self>);

        /// The key and mouse editor: starts editing a copy of the bindings.
        #[qinvokable]
        fn editor_open(self: Pin<&mut Self>);

        /// Every action with its keys and gestures, as JSON.
        #[qinvokable]
        fn editor_rows(self: &Self) -> QString;

        /// The mouse gesture names, as JSON.
        #[qinvokable]
        fn editor_gestures(self: &Self) -> QString;

        /// The action a pressed key belongs to in a scope: JSON with the
        /// key's text and the owner's settings name and label (empty when
        /// free). Pure modifier keys give `{}`.
        #[qinvokable]
        fn editor_key_owner(self: &Self, scope: &QString, key: i32, modifiers: i32) -> QString;

        /// The label of the action a gesture is bound to (empty when free).
        #[qinvokable]
        fn editor_gesture_owner(self: &Self, gesture: &QString) -> QString;

        /// Binds the pressed key to an action; returns JSON with the key's
        /// text and the action that lost it (empty when none).
        #[qinvokable]
        fn editor_bind_key(
            self: Pin<&mut Self>,
            scope: &QString,
            action: &QString,
            key: i32,
            modifiers: i32,
        ) -> QString;

        #[qinvokable]
        fn editor_unbind_key(self: Pin<&mut Self>, scope: &QString, key: &QString);

        /// Binds a gesture; returns the action that lost it (empty when
        /// none).
        #[qinvokable]
        fn editor_bind_gesture(
            self: Pin<&mut Self>,
            action: &QString,
            gesture: &QString,
        ) -> QString;

        #[qinvokable]
        fn editor_unbind_gesture(self: Pin<&mut Self>, gesture: &QString);

        #[qinvokable]
        fn editor_reset(self: Pin<&mut Self>, scope: &QString, action: &QString);

        #[qinvokable]
        fn editor_reset_all(self: Pin<&mut Self>);

        /// Ends editing; `save` applies and stores the bindings.
        #[qinvokable]
        fn editor_close(self: Pin<&mut Self>, save: bool);

        /// Rectangle of a dynamic region; `serial` only makes QML bindings
        /// re-evaluate after a new layout.
        #[qinvokable]
        fn region_rect(self: &Self, index: i32, serial: i32) -> QRectF;

        /// A playlist panel rectangle (self-test): `what` is "row" or
        /// "tab" with its index, or "area".
        #[qinvokable]
        fn list_rect(self: &Self, what: &QString, index: i32) -> QRectF;

        /// Seeks to a fraction of the duration (self-test).
        #[qinvokable]
        fn seek_fraction(self: Pin<&mut Self>, fraction: f64);

        /// About ten times a second: shows a held-back playback update.
        #[qinvokable]
        fn flush_chrome(self: Pin<&mut Self>);

        /// About once a second: saves pending state.
        #[qinvokable]
        fn tick(self: Pin<&mut Self>);

        /// Stops playback, waits for mpv to go idle and saves (goal D rule
        /// 6). Called when the window closes.
        #[qinvokable]
        fn shutdown(self: Pin<&mut Self>);

        /// `name=milliseconds` lines of the usability measurements.
        #[qinvokable]
        fn metrics(self: &Self) -> QString;

        /// The player state as JSON (self-test and diagnostics).
        #[qinvokable]
        fn status(self: &Self) -> QString;

        /// Frame timing and output diagnostics from mpv as JSON (dropped
        /// frames, display rate, audio output; goal D rule 5).
        #[qinvokable]
        fn diagnostics(self: &Self) -> QString;

        /// True when `--name` was given on the command line.
        #[qinvokable]
        fn has_option(self: &Self, name: &QString) -> bool;

        /// The value of `--name=value`, empty when absent.
        #[qinvokable]
        fn option(self: &Self, name: &QString) -> QString;

        /// Something only the window can do; see [`super::request`].
        #[qsignal]
        #[cxx_name = "uiRequest"]
        fn ui_request(self: Pin<&mut Self>, kind: i32, argument: i32);

        /// Chrome items to repaint: the static layer, region indexes
        /// separated by commas, and whether to wait for the next video
        /// frame.
        #[qsignal]
        #[cxx_name = "damaged"]
        fn damaged(self: Pin<&mut Self>, static_layer: bool, regions: QString, synced: bool);

        /// Resize the window to this size (a new video at 100 %).
        #[qsignal]
        #[cxx_name = "fitWindow"]
        fn fit_window(self: Pin<&mut Self>, width: i32, height: i32);
    }

    impl cxx_qt::Threading for Player {}
}
