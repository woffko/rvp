//! M11's part of `Player`: the seek bar's tooltip and preview, the
//! thumbnail core's pictures, the dialogs of the Playback, Open and
//! playlist commands, the notice before a power action, the Recycle Bin,
//! the clipboard and new instances.
//!
//! Dialogs get their data as JSON in the `dialog` property and open with
//! [`request::DIALOG`]; QML (`PlaybackDialogs.qml`, `SceneBrowser.qml`)
//! answers through the invokables here. Pictures go into the atlas cache
//! under `thumb/preview`, `thumb/scene/N` and `thumb/seek` (a seek's target
//! over the video) and a `ThumbView` paints them.

use std::path::PathBuf;
use std::pin::Pin;
use std::time::Instant;

use cxx_qt_lib::{QRectF, QString};
use rvp_app::thumbs::ThumbEvent;
use rvp_app::{PowerTimer, UiRequest};
use rvp_engine::PropertyId;
use serde_json::{Value, json};

use super::{Core, request};
use crate::ffi::bridge::{rvp_atlas_insert, rvp_atlas_remove_prefix};
use crate::player_qobject::qobject;

/// Copies a picture into the atlas cache.
fn insert_picture(key: &str, picture: &rvp_engine::thumbnail::Picture) -> Option<(i32, i32)> {
    let width = i32::try_from(picture.width).ok()?;
    let height = i32::try_from(picture.height).ok()?;
    rvp_atlas_insert(key, &picture.rgba, width, height);
    Some((width, height))
}

/// Starts the thumbnail core after the player's (FFmpeg's log stays with
/// the first core); its thread only queues the pictures to the window's.
pub(super) fn start_thumbnails(
    app: &mut rvp_app::App,
    thread: cxx_qt::CxxQtThread<qobject::Player>,
) -> Option<String> {
    let thread = std::sync::Mutex::new(thread);
    app.start_thumbnails(
        rvp_engine::thumbnail::ThumbnailOptions::default(),
        move || {
            if let Ok(thread) = thread.lock() {
                let _ = thread.queue(qobject::Player::thumbnails_ready);
            }
        },
    )
    .err()
    .map(|error| format!("thumbnails: {error}"))
}

/// The dry-run file of the power actions: `--power-dry-run=FILE`, or with
/// `--selftest` a file in the state folder (a test never powers off).
pub(super) fn power_dry_run(cli: &rvp_app::Cli, state: &std::path::Path) -> Option<PathBuf> {
    if let Some(file) = cli.value("power-dry-run") {
        return Some(PathBuf::from(file));
    }
    (cli.flag("selftest") || cli.flag("power-dry-run")).then(|| state.join("power-dry-run.txt"))
}

impl qobject::Player {
    /// Opens a dialog of `PlaybackDialogs.qml` with its data.
    fn open_dialog(mut self: Pin<&mut Self>, data: &Value) {
        self.as_mut().set_dialog(QString::from(&data.to_string()));
        self.ui_request(request::DIALOG, 0);
    }

    /// The pointer over the seek bar (or off it): the time tooltip and the
    /// preview.
    pub(super) fn seek_hover(
        mut self: Pin<&mut Self>,
        hover: Option<rvp_app::playback::SeekHover>,
    ) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(tip) = core.app.seek_hover(hover) {
            let preview = tip.preview.is_some();
            self.as_mut().set_seek_tip(QString::from(&tip.text));
            self.as_mut().set_seek_tip_x(super::window_length(tip.x));
            self.as_mut().set_seek_tip_y(super::window_length(tip.top));
            if !preview {
                self.as_mut().set_preview_width(0);
            }
            self.set_seek_tip_shown(true);
        } else {
            self.as_mut().set_seek_tip_shown(false);
            self.set_preview_width(0);
        }
    }

    /// The thumbnail core finished pictures: they go into the atlas cache
    /// and the items showing their keys repaint.
    pub(crate) fn thumbnails_ready(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let events = core.app.thumbnail_events();
        let mut list_changed = false;
        for event in events {
            match event {
                ThumbEvent::Preview(picture) => {
                    if !*self.seek_tip_shown() {
                        continue;
                    }
                    if let Some((width, height)) = insert_picture("thumb/preview", &picture) {
                        self.as_mut().set_preview_width(width);
                        self.as_mut().set_preview_height(height);
                        let serial = self.preview_serial().wrapping_add(1);
                        self.as_mut().set_preview_serial(serial);
                    }
                }
                ThumbEvent::Scene { index, picture } => {
                    let key = format!("thumb/scene/{index}");
                    if insert_picture(&key, &picture).is_some()
                        && let Ok(index) = i32::try_from(index)
                    {
                        self.as_mut().scene_ready(index);
                    }
                }
                ThumbEvent::Entry { path, picture } => {
                    let key = rvp_app::view::thumbnail_key(&path);
                    list_changed |= insert_picture(&key, &picture).is_some();
                }
                ThumbEvent::SeekPicture { picture, place } => {
                    if let Some((width, height)) = insert_picture("thumb/seek", &picture) {
                        let [x, y, w, h] = place;
                        self.as_mut()
                            .set_seek_picture_place(QRectF::new(x, y, w, h));
                        self.as_mut().set_seek_picture_height(height);
                        self.as_mut().set_seek_picture_width(width);
                        let serial = self.seek_picture_serial().wrapping_add(1);
                        self.as_mut().set_seek_picture_serial(serial);
                    }
                }
                ThumbEvent::Unavailable(error) => rvp_app::log::report(
                    rvp_app::log::Level::Warning,
                    &format!("thumbnails: {error}"),
                ),
                ThumbEvent::Sheet(sheet) => self.as_mut().save_sheet(sheet),
            }
        }
        // Durations found in the background, and new list thumbnails.
        if list_changed {
            self.as_mut().relayout();
        }
        self.sync_chrome();
    }

    /// M11's requests ([`super::qobject::Player::handle_request`] has the
    /// rest).
    pub(super) fn m11_request(mut self: Pin<&mut Self>, request: UiRequest) {
        match request {
            UiRequest::JumpDialog {
                time,
                duration,
                fps,
            } => self.open_dialog(&json!({
                "kind": "jump", "time": time, "duration": duration, "fps": fps
            })),
            UiRequest::SectionList => self.open_dialog(&json!({ "kind": "sections" })),
            UiRequest::Bookmarks => self.open_dialog(&json!({ "kind": "bookmarks" })),
            UiRequest::BookmarkTitle { time, title } => self.open_dialog(&json!({
                "kind": "bookmark-title", "time": time, "title": title
            })),
            UiRequest::PowerNotice(notice) => {
                let (text, seconds) = notice.unwrap_or_default();
                self.as_mut().set_power_notice(QString::from(&text));
                self.set_power_seconds(i32::try_from(seconds).unwrap_or(i32::MAX));
            }
            UiRequest::Power(action) => self.power_now(action),
            UiRequest::HideSeekPicture => {
                self.set_seek_picture_width(0);
                // The video's size: not kept for the next seek.
                rvp_atlas_remove_prefix("thumb/seek");
            }
            UiRequest::PowerTimerDialog(kind) => {
                let timer = match kind {
                    PowerTimer::At => "at",
                    PowerTimer::In => "in",
                    PowerTimer::Dialog => "dialog",
                };
                self.open_dialog(&json!({ "kind": "power-timer", "timer": timer }));
            }
            UiRequest::ConfirmRecycle { files, entries } => {
                let files: Vec<String> = files
                    .iter()
                    .map(|file| file.to_string_lossy().into_owned())
                    .collect();
                self.open_dialog(&json!({
                    "kind": "recycle", "files": files, "entries": entries
                }));
            }
            UiRequest::CopyText(text) => {
                self.as_mut().set_dialog(QString::from(&text));
                self.ui_request(request::COPY_TEXT, 0);
            }
            UiRequest::ReadClipboard(purpose) => {
                let code = match purpose {
                    rvp_app::sources::Paste::Open => 0,
                    rvp_app::sources::Paste::List => 1,
                    rvp_app::sources::Paste::ListBelow => 2,
                };
                self.ui_request(request::READ_CLIPBOARD, code);
            }
            UiRequest::OpenAudioDialog => self.open_dialog(&json!({ "kind": "audio" })),
            UiRequest::DiscDialog { bluray, files } => self.open_dialog(&json!({
                "kind": "disc", "bluray": bluray, "files": files
            })),
            UiRequest::DeviceDialog(_) => {
                let devices = crate::devices::list();
                self.open_dialog(&json!({ "kind": "devices", "devices": devices }));
            }
            UiRequest::SceneBrowser(browser) => {
                rvp_atlas_remove_prefix("thumb/scene/");
                self.as_mut().set_dialog(QString::from(
                    &serde_json::to_string(&browser).unwrap_or_default(),
                ));
                self.ui_request(request::SCENES, 0);
            }
            UiRequest::Favorites => self.open_dialog(&json!({ "kind": "favorites" })),
            UiRequest::ListDialog { kind, text } => self.open_dialog(&json!({
                "kind": "list", "list": kind, "text": text
            })),
            UiRequest::NewInstance(files) => {
                let mut arguments = vec!["/new".to_owned(), "--".to_owned()];
                arguments.extend(files);
                if let Err(error) = std::process::Command::new(
                    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rvp")),
                )
                .args(&arguments)
                .spawn()
                {
                    self.show_message(&format!("cannot start RVP: {error}"));
                }
            }
            UiRequest::ShowFile(path) => {
                self.as_mut()
                    .set_dialog(QString::from(&path.to_string_lossy().into_owned()));
                self.ui_request(request::SHOW_FILE, 0);
            }
            UiRequest::PopupMenu(command) => {
                if let Some(core) = self.as_mut().core_mut() {
                    core.popup = Some((command, None));
                }
                self.ui_request(request::SKIN_MENU, 0);
            }
            UiRequest::ListView => {
                rvp_atlas_remove_prefix("thumb/entry/");
                self.relayout();
            }
            other => self.m12_request(other),
        }
    }

    /// Shuts down, sleeps or hibernates now (or with the dry-run hook only
    /// writes it down).
    fn power_now(mut self: Pin<&mut Self>, action: rvp_app::PowerAction) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let result = match &core.power_dry_run {
            Some(file) => rvp_app::power::write_dry_run(file, action).map_err(|e| e.to_string()),
            None => crate::power::perform(action),
        };
        if let Err(error) = result {
            self.show_message(&format!(
                "cannot {}: {error}",
                rvp_app::power::action_text(action)
            ));
        }
    }

    /// Once a second: the power notice and timer, file jobs and yt-dlp.
    pub(super) fn tick_m11(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let mut requests = core.app.power_tick(Instant::now());
        requests.extend(core.app.file_job_done());
        requests.extend(core.app.resolver_tick(core.engine));
        for request in requests {
            self.as_mut().handle_request(request);
        }
    }

    pub(crate) fn jump_to(mut self: Pin<&mut Self>, text: &QString, frames: bool) -> QString {
        let Some(core) = self.as_mut().core_mut() else {
            return QString::default();
        };
        match core
            .app
            .jump_to_text(core.engine, &text.to_string(), frames)
        {
            Ok(()) => QString::default(),
            Err(error) => QString::from(&error),
        }
    }

    pub(crate) fn add_bookmark(mut self: Pin<&mut Self>, time: f64, title: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core.app.add_bookmark(time, &title.to_string());
        self.as_mut().handle_request(request);
        self.sync_chrome();
    }

    pub(crate) fn bookmarks_json(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        let bookmarks = core
            .app
            .model
            .loaded_path()
            .map(|path| core.app.marks().file(path).bookmarks)
            .unwrap_or_default();
        QString::from(&serde_json::to_string(&bookmarks).unwrap_or_else(|_| "[]".to_owned()))
    }

    pub(crate) fn bookmark_edit(
        mut self: Pin<&mut Self>,
        op: &QString,
        index: i32,
        text: &QString,
    ) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Some(path) = core.app.model.loaded_path().map(str::to_owned) else {
            return;
        };
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        let request = match op.to_string().as_str() {
            "jump" => core.app.jump_to_bookmark(core.engine, index),
            "rename" => {
                core.app
                    .marks_mut()
                    .rename_bookmark(&path, index, text.to_string().trim());
                None
            }
            "delete" => {
                core.app.marks_mut().remove_bookmark(&path, index);
                None
            }
            "add" => core.app.dispatch(
                core.engine,
                rvp_app::Action::Bookmark(rvp_app::BookmarkCommand::Add),
            ),
            _ => None,
        };
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    pub(crate) fn sections_json(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        let sections = core
            .app
            .model
            .loaded_path()
            .map(|path| core.app.marks().file(path).sections)
            .unwrap_or_default();
        QString::from(&serde_json::to_string(&sections).unwrap_or_else(|_| "[]".to_owned()))
    }

    pub(crate) fn section_edit(mut self: Pin<&mut Self>, op: &QString, index: i32, text: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core.app.section_edit(
            core.engine,
            &op.to_string(),
            usize::try_from(index).ok(),
            &text.to_string(),
        );
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    pub(crate) fn cancel_countdown(mut self: Pin<&mut Self>) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Some(request) = core.app.cancel_countdown() {
            self.handle_request(request);
        }
    }

    pub(crate) fn set_power_timer(
        mut self: Pin<&mut Self>,
        kind: i32,
        seconds: f64,
        exit: bool,
        label: &QString,
    ) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let kind = if kind == 0 {
            PowerTimer::At
        } else {
            PowerTimer::In
        };
        // Whole seconds of a timer are far inside u64.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let seconds = seconds.max(1.0).round() as u64;
        let request =
            core.app
                .set_power_timer(kind, seconds, exit, &label.to_string(), Instant::now());
        self.handle_request(request);
    }

    pub(crate) fn recycle(mut self: Pin<&mut Self>, files: &QString, entries: &QString) -> QString {
        let Some(core) = self.as_mut().core_mut() else {
            return QString::default();
        };
        let files: Vec<PathBuf> = serde_json::from_str::<Vec<String>>(&files.to_string())
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();
        let entries: Vec<String> = serde_json::from_str(&entries.to_string()).unwrap_or_default();
        core.app.prepare_recycle(core.engine, &files);
        let result = crate::recycle::send(&files);
        match result {
            Ok(()) => {
                core.app.recycled(&entries);
                self.sync_chrome();
                QString::default()
            }
            Err(error) => QString::from(&error),
        }
    }

    pub(crate) fn paste_text(mut self: Pin<&mut Self>, purpose: i32, text: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let purpose = match purpose {
            1 => rvp_app::sources::Paste::List,
            2 => rvp_app::sources::Paste::ListBelow,
            _ => rvp_app::sources::Paste::Open,
        };
        if purpose == rvp_app::sources::Paste::Open {
            core.request_fit();
        }
        if let Some(request) = core.app.paste_text(core.engine, purpose, &text.to_string()) {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    pub(crate) fn load_external_audio(mut self: Pin<&mut Self>, path: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core
            .app
            .load_external_audio(core.engine, std::path::Path::new(&path.to_string()));
        if let Some(request) = request {
            self.handle_request(request);
        }
    }

    pub(crate) fn open_disc(mut self: Pin<&mut Self>, path: &QString, bluray: bool) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.request_fit();
        let request =
            core.app
                .open_disc(core.engine, std::path::Path::new(&path.to_string()), bluray);
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    pub(crate) fn open_device(mut self: Pin<&mut Self>, name: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.request_fit();
        if let Some(request) = core.app.open_device(core.engine, &name.to_string()) {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    // A QML invokable, so a method even though it asks the system only.
    #[allow(clippy::unused_self)]
    pub(crate) fn devices_json(&self) -> QString {
        QString::from(
            &serde_json::to_string(&crate::devices::list()).unwrap_or_else(|_| "[]".to_owned()),
        )
    }

    pub(crate) fn list_dialog_answer(mut self: Pin<&mut Self>, kind: &QString, text: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request =
            core.app
                .list_dialog_answer(core.engine, &kind.to_string(), &text.to_string());
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }

    pub(crate) fn scene_chosen(mut self: Pin<&mut Self>, time: f64) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.app.scene_chosen(core.engine, time);
        self.sync_chrome();
    }

    pub(crate) fn scenes_closed(mut self: Pin<&mut Self>) {
        if let Some(core) = self.as_mut().core_mut() {
            core.app.cancel_scenes();
        }
        rvp_atlas_remove_prefix("thumb/scene/");
    }

    pub(crate) fn drop_files(mut self: Pin<&mut Self>, paths: &QString) {
        // The General page's advanced drag and drop asks first (M14).
        let request = self.core().and_then(|core| {
            core.app
                .drop_request(paths.to_string().lines().map(str::to_owned).collect())
        });
        if let Some(rvp_app::UiRequest::DropMenu(_)) = request {
            self.as_mut().set_dialog(paths.clone());
            self.ui_request(super::request::DROP_MENU, 0);
            return;
        }
        let sort = self
            .core()
            .is_some_and(|core| core.app.settings.playlist.sort_dropped);
        let mut lines: Vec<String> = paths.to_string().lines().map(str::to_owned).collect();
        if sort {
            lines.sort_by_key(|line| line.to_lowercase());
        }
        self.as_mut()
            .open_files(&QString::from(&lines.join("\n")), false);
    }

    /// The rectangle of the main window's control bound to `command`
    /// (self-test); empty when it is not shown.
    pub(crate) fn control_rect(&self, command: &QString) -> cxx_qt_lib::QRectF {
        let rect = self
            .core()
            .and_then(|core| core.chrome.as_ref())
            .and_then(|chrome| chrome.layout())
            .and_then(|placed| placed.find_command(&command.to_string()))
            .map(|node| node.rect)
            .unwrap_or_default();
        super::to_window(rect)
    }

    pub(crate) fn favorites_json(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("[]");
        };
        QString::from(
            &serde_json::to_string(&core.app.marks().marks().favorites)
                .unwrap_or_else(|_| "[]".to_owned()),
        )
    }

    pub(crate) fn set_favorites(mut self: Pin<&mut Self>, json: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        if let Ok(favorites) = serde_json::from_str(&json.to_string()) {
            core.app.marks_mut().set_favorites(favorites);
        }
    }
}

/// M11's part of the self-test's status: the seek bar's tooltip and
/// preview, marks, A-B points and lists, the playlist's queue, thumbnails
/// and durations, the power notice and its dry-run file, discs, tracks,
/// favorites, recent items and the thumbnail core's counters.
pub(super) fn status(player: &qobject::Player, core: &Core) -> Vec<(&'static str, String)> {
    let app = &core.app;
    let shown = app.library().shown();
    let marks = app
        .model
        .loaded_path()
        .map(|path| app.marks().file(path))
        .unwrap_or_default();
    let number =
        |value: Option<f64>| value.map_or_else(|| "null".to_owned(), |v| format!("{v:.3}"));
    let count = |value: usize| value.to_string();
    let session = app.session();
    let dry_run = core
        .power_dry_run
        .as_ref()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .unwrap_or_default();
    let tracks = core.engine.tracks();
    vec![
        (
            "seek_tip",
            Value::from(player.seek_tip().to_string()).to_string(),
        ),
        ("seek_tip_shown", player.seek_tip_shown().to_string()),
        ("preview_width", player.preview_width().to_string()),
        ("preview_height", player.preview_height().to_string()),
        (
            "seek_picture_width",
            player.seek_picture_width().to_string(),
        ),
        (
            "seek_picture_serial",
            player.seek_picture_serial().to_string(),
        ),
        ("marks", count(app.seek_marks().len())),
        ("bookmarks", count(marks.bookmarks.len())),
        ("sections", count(marks.sections.len())),
        ("sub_start", number(app.model.double(PropertyId::SubStart))),
        ("ab_a", number(session.point_a)),
        ("ab_b", number(session.point_b)),
        ("ab_active", session.ab_active.to_string()),
        ("queue", count(shown.queue.len())),
        (
            "list_rows",
            app.settings.playlist.thumbnail_rows.to_string(),
        ),
        ("list_thumbnails", count(app.list_thumbnails_ready())),
        (
            "durations_known",
            count(
                shown
                    .entries
                    .iter()
                    .filter(|e| e.duration.is_some())
                    .count(),
            ),
        ),
        ("power_seconds", player.power_seconds().to_string()),
        ("power_dry_run", Value::from(dry_run).to_string()),
        ("disc_titles", count(app.disc_titles().len())),
        (
            "dvd_supported",
            core.engine
                .protocols()
                .iter()
                .any(|protocol| protocol == "dvd")
                .to_string(),
        ),
        (
            "audio_tracks",
            count(
                tracks
                    .iter()
                    .filter(|track| track.kind == rvp_engine::TrackKind::Audio)
                    .count(),
            ),
        ),
        ("favorites", count(app.marks().marks().favorites.len())),
        ("recent", count(app.marks().marks().recent.len())),
        (
            "media_title",
            Value::from(core.engine.get_text("media-title").unwrap_or_default()).to_string(),
        ),
        ("thumbnail_stats", thumbnail_stats(app)),
    ]
}

/// The thumbnail core's counters as JSON (`null` without the core).
fn thumbnail_stats(app: &rvp_app::App) -> String {
    app.thumbnail_stats().map_or_else(
        || "null".to_owned(),
        |stats| {
            json!({
                "rendered": stats.rendered,
                "cache_hits": stats.cache_hits,
                "superseded": stats.superseded,
                "cancelled": stats.cancelled,
                "failed": stats.failed,
                "durations": stats.durations,
                "last_render_ms": stats.last_render.map(|took| took.as_secs_f64() * 1000.0),
                "off_gui_thread": stats
                    .render_thread
                    .is_some_and(|thread| thread != std::thread::current().id()),
            })
            .to_string()
        },
    )
}

impl Core {
    /// The window's pointer hides over the playing video in windowed mode.
    pub(super) fn hides_pointer(&self) -> bool {
        self.app.settings.playback.hide_pointer
            && self.app.model.playing()
            && self
                .app
                .model
                .int(rvp_engine::PropertyId::VideoWidth)
                .is_some()
    }
}
