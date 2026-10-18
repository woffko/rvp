//! The `Player` `QObject`'s bridge; the implementation is in `player.rs`.
//!
//! This file holds only the bridge, so the `unsafe` the generated code needs
//! is allowed for the file (cxx-qt accepts no attributes on the bridge
//! module itself).

// SAFETY: cxx-qt bridge (owner decision 2026-10-10). The declarations are
// plain Qt value types and safe methods; nothing here is called unsafely by
// hand.
#![allow(unsafe_code)]
// The generated setters compare a property's old and new value with `==`
// to skip unchanged updates; for `video_aspect` an exact comparison is
// what is wanted.
#![allow(clippy::float_cmp)]

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
        #[qproperty(QString, settings_page)]
        #[qproperty(bool, on_top)]
        #[qproperty(bool, info_shown)]
        #[qproperty(QString, external_url)]
        #[qproperty(bool, panel_shown)]
        #[qproperty(i32, panel_width)]
        #[qproperty(i32, panel_height)]
        #[qproperty(QString, panel_tooltip)]
        #[qproperty(QString, frame_path)]
        #[qproperty(QString, subtitle_font)]
        #[qproperty(bool, panel_has_position)]
        #[qproperty(i32, panel_x)]
        #[qproperty(i32, panel_y)]
        #[qproperty(QString, record_path)]
        #[qproperty(QString, seek_tip)]
        #[qproperty(i32, seek_tip_x)]
        #[qproperty(i32, seek_tip_y)]
        #[qproperty(bool, seek_tip_shown)]
        #[qproperty(i32, preview_serial)]
        #[qproperty(i32, preview_width)]
        #[qproperty(i32, preview_height)]
        #[qproperty(bool, hide_pointer)]
        #[qproperty(bool, center_window)]
        /// The video's display aspect ratio (0 without video), for the
        /// window's sizes (M12).
        #[qproperty(f64, video_aspect)]
        /// Fullscreen Mode's "Fill other monitors black" (M12).
        #[qproperty(bool, black_other_displays)]
        /// Fullscreen goes to the primary display (M12).
        #[qproperty(bool, fullscreen_primary)]
        /// The slideshow for audio files (JSON, `App::slideshow_json`).
        #[qproperty(QString, slideshow)]
        #[qproperty(QString, power_notice)]
        #[qproperty(i32, power_seconds)]
        #[qproperty(QString, dialog)]
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

        /// Pointer input in the control panel's window (F7), in its
        /// coordinates, as the `mouse_*` calls of the main window.
        #[qinvokable]
        fn panel_press(self: Pin<&mut Self>, x: f64, y: f64, button: i32);
        #[qinvokable]
        fn panel_double(self: Pin<&mut Self>, x: f64, y: f64, button: i32);
        #[qinvokable]
        fn panel_release(self: Pin<&mut Self>, x: f64, y: f64, button: i32);
        #[qinvokable]
        fn panel_move(self: Pin<&mut Self>, x: f64, y: f64);
        #[qinvokable]
        fn panel_leave(self: Pin<&mut Self>);

        /// The window system closed the control panel's window.
        #[qinvokable]
        fn panel_closed(self: Pin<&mut Self>);

        /// The rectangle of the control panel's control bound to a command
        /// (self-test); empty when it is not shown. `CMD_SELECT_PAGE:1`
        /// names the button of page 1.
        #[qinvokable]
        fn panel_rect(self: &Self, command: &QString) -> QRectF;

        /// Saves the current frame to a file chosen in the save dialog.
        #[qinvokable]
        fn save_frame_to(self: Pin<&mut Self>, path: &QString);

        /// Sets the subtitle font chosen in the font dialog (empty: mpv's
        /// default).
        #[qinvokable]
        fn apply_subtitle_font(self: Pin<&mut Self>, family: &QString);

        /// The control panel's window moved (screen coordinates).
        #[qinvokable]
        fn panel_moved(self: Pin<&mut Self>, x: i32, y: i32);

        /// A key press (`QKeyEvent::key()`, `modifiers()`); true when bound.
        #[qinvokable]
        fn key(self: Pin<&mut Self>, key: i32, modifiers: i32) -> bool;

        /// Runs an action by its settings name (menus, self-test).
        #[qinvokable]
        fn run_action(self: Pin<&mut Self>, name: &QString);

        /// A menu as JSON (`rvp_app::menu::to_json`): `context`,
        /// `playlist`, or `skin` (the skin menu a control asked for).
        #[qinvokable]
        fn menu_json(self: &Self, kind: &QString) -> QString;

        /// Carries out a menu item's command (`rvp_app::menu::Command::id`).
        #[qinvokable]
        fn run_menu_command(self: Pin<&mut Self>, id: &QString);

        /// Runs a playlist action by its settings name (self-test).
        #[qinvokable]
        fn run_list_action(self: Pin<&mut Self>, name: &QString);

        /// The settings window's data as JSON (`App::settings_view`); with
        /// `devices` also the audio devices (slow: only when it opens).
        #[qinvokable]
        fn settings_view(self: &Self, devices: bool) -> QString;

        /// Changes one setting (`rvp_app::preferences::change`; the value
        /// is JSON) and applies it; returns JSON with `notes`, `restart` and
        /// `error`.
        #[qinvokable]
        fn set_setting(self: Pin<&mut Self>, path: &QString, value: &QString) -> QString;

        /// Imports a settings file and applies it; JSON as `set_setting`.
        #[qinvokable]
        fn import_settings(self: Pin<&mut Self>, path: &QString) -> QString;

        /// Exports the settings; JSON as `set_setting`.
        #[qinvokable]
        fn export_settings(self: &Self, path: &QString) -> QString;

        /// Opens a folder's media files (Open Folder).
        #[qinvokable]
        fn open_folder(self: Pin<&mut Self>, path: &QString);

        /// Loads a subtitle file for the playing video.
        #[qinvokable]
        fn load_subtitle(self: Pin<&mut Self>, path: &QString);

        /// Sets the folder for saved frames.
        #[qinvokable]
        fn set_image_folder(self: Pin<&mut Self>, path: &QString);

        /// Sets the audio sync from the Adjust Audio Sync dialog (seconds).
        #[qinvokable]
        fn set_audio_delay(self: Pin<&mut Self>, seconds: f64);

        /// Records what RVP plays into the file the save dialog chose.
        #[qinvokable]
        fn start_recording(self: Pin<&mut Self>, path: &QString);

        /// Sets the folder audio recordings go to.
        #[qinvokable]
        fn set_record_folder(self: Pin<&mut Self>, path: &QString);

        /// The About box's text.
        #[qinvokable]
        fn about_text(self: &Self) -> QString;

        /// Playback information over the video (Tab), from the model.
        #[qinvokable]
        fn info_text(self: &Self) -> QString;

        /// The local time for the skin's clock (`HH:MM:SS`).
        #[qinvokable]
        fn set_clock(self: Pin<&mut Self>, text: &QString);

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

        // M11.
        /// The Jump to Time/Frame dialog's answer; returns the error to
        /// show (empty when it jumped).
        #[qinvokable]
        fn jump_to(self: Pin<&mut Self>, text: &QString, frames: bool) -> QString;

        /// Adds a bookmark with the title the dialog asked for.
        #[qinvokable]
        fn add_bookmark(self: Pin<&mut Self>, time: f64, title: &QString);

        /// The playing file's bookmarks as JSON (`time`, `title`).
        #[qinvokable]
        fn bookmarks_json(self: &Self) -> QString;

        /// The bookmark dialog: `jump`, `rename` (to `text`), `delete` or
        /// `add` (at the current time) a bookmark.
        #[qinvokable]
        fn bookmark_edit(self: Pin<&mut Self>, op: &QString, index: i32, text: &QString);

        /// The playing file's A-B repeat list as JSON (`start`, `end`,
        /// `title`).
        #[qinvokable]
        fn sections_json(self: &Self) -> QString;

        /// The A-B list dialog: `add` (the current A-B points), `repeat`,
        /// `rename` (to `text`) or `delete` a section.
        #[qinvokable]
        fn section_edit(self: Pin<&mut Self>, op: &QString, index: i32, text: &QString);

        /// The notice's Cancel before shutting down or exiting.
        #[qinvokable]
        fn cancel_countdown(self: Pin<&mut Self>);

        /// The shutdown timer's dialog: fire in `seconds` (`kind` 0 at a
        /// time, 1 after a duration), exiting RVP or powering off.
        #[qinvokable]
        fn set_power_timer(
            self: Pin<&mut Self>,
            kind: i32,
            seconds: f64,
            exit: bool,
            label: &QString,
        );

        /// The Recycle Bin dialog's Yes: the files (JSON list) go there
        /// and the album entries (JSON list) are removed; returns the error
        /// (empty when done).
        #[qinvokable]
        fn recycle(self: Pin<&mut Self>, files: &QString, entries: &QString) -> QString;

        /// The clipboard's text for a purpose (0 open, 1 add to the album,
        /// 2 add below the selected entry).
        #[qinvokable]
        fn paste_text(self: Pin<&mut Self>, purpose: i32, text: &QString);

        /// Load External Audio's file.
        #[qinvokable]
        fn load_external_audio(self: Pin<&mut Self>, path: &QString);

        /// A disc folder or file from its dialog.
        #[qinvokable]
        fn open_disc(self: Pin<&mut Self>, path: &QString, bluray: bool);

        /// A capture device from its dialog.
        #[qinvokable]
        fn open_device(self: Pin<&mut Self>, name: &QString);

        /// The capture devices the system has, as JSON (`name`, `audio`).
        #[qinvokable]
        fn devices_json(self: &Self) -> QString;

        /// A playlist dialog's answer (`rvp_app::UiRequest::ListDialog`).
        #[qinvokable]
        fn list_dialog_answer(self: Pin<&mut Self>, kind: &QString, text: &QString);

        /// A scene of the scene browser was chosen.
        #[qinvokable]
        fn scene_chosen(self: Pin<&mut Self>, time: f64);

        /// The scene browser closed: its pictures are no longer wanted.
        #[qinvokable]
        fn scenes_closed(self: Pin<&mut Self>);

        /// The rectangle of the main window's control bound to a command
        /// (self-test); empty when it is not shown.
        #[qinvokable]
        fn control_rect(self: &Self, command: &QString) -> QRectF;

        /// The favorites as JSON (`path`, `title`, `folder`).
        #[qinvokable]
        fn favorites_json(self: &Self) -> QString;

        /// The favorites' dialog's answer (the whole list as JSON).
        #[qinvokable]
        fn set_favorites(self: Pin<&mut Self>, json: &QString);

        /// Files dropped on the window (one a line): sorted first when the
        /// Playlist page says so, then opened.
        #[qinvokable]
        fn drop_files(self: Pin<&mut Self>, paths: &QString);

        /// The thumbnail core finished pictures (queued from its thread).
        #[qinvokable]
        fn thumbnails_ready(self: Pin<&mut Self>);

        /// The screen the window is on, as `Tools.screen_json` gives it
        /// (M12: the output device's aspect, rotation by aspect ratio).
        #[qinvokable]
        fn set_screen(self: Pin<&mut Self>, json: &QString);

        /// An M12 dialog's answer (`VideoDialogs.qml`): the purpose it was
        /// opened for and the text entered.
        #[qinvokable]
        fn video_dialog_answer(self: Pin<&mut Self>, purpose: &QString, text: &QString);

        /// Keeps the mouse pointer on the window's display while `wanted`
        /// (fullscreen and active) and the setting asks for it (M12).
        #[qinvokable]
        fn confine_cursor(self: Pin<&mut Self>, wanted: bool);

        /// Something only the window can do; see [`super::request`].
        #[qsignal]
        #[cxx_name = "uiRequest"]
        fn ui_request(self: Pin<&mut Self>, kind: i32, argument: i32);

        /// A scene browser's picture arrived (atlas `thumb/scene/INDEX`).
        #[qsignal]
        #[cxx_name = "sceneReady"]
        fn scene_ready(self: Pin<&mut Self>, index: i32);

        /// Chrome items to repaint: the static layer, region indexes
        /// separated by commas, and whether to wait for the next video
        /// frame.
        #[qsignal]
        #[cxx_name = "damaged"]
        fn damaged(self: Pin<&mut Self>, static_layer: bool, regions: QString, synced: bool);

        /// The control panel's window repaints (all of it: it is small).
        #[qsignal]
        #[cxx_name = "panelDamaged"]
        fn panel_damaged(self: Pin<&mut Self>);

        /// Resize the window to this size (a new video at 100 %).
        #[qsignal]
        #[cxx_name = "fitWindow"]
        fn fit_window(self: Pin<&mut Self>, width: i32, height: i32);
    }

    impl cxx_qt::Threading for Player {}
}
