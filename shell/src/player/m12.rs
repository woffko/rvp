//! M12's part of the window (picture, video, window and capture): the
//! screen the window is on, the dialogs of `VideoDialogs.qml` with the
//! answers they bring back, and the capture thread.

use std::pin::Pin;

use cxx_qt::Threading;
use cxx_qt_lib::QString;
use rvp_app::UiRequest;
use rvp_app::capture::{self, CaptureJob, SheetImage};
use rvp_app::view;
use rvp_app::window::WindowRequest;
use serde_json::{Value, json};

use super::request;
use crate::ffi::bridge::{rvp_copy_image, rvp_save_image};
use crate::player_qobject::qobject;
use crate::video::ENGINE;

/// What a capture brought: a notice, or a picture for the clipboard.
enum Captured {
    Notice(String),
    Clipboard(Vec<u8>, i32, i32),
}

/// Carries a capture out (the capture thread): mpv writes PNG, JPEG and
/// WEBP itself; the rest takes `screenshot-raw`'s picture through Qt.
fn run_capture(job: &CaptureJob) -> Captured {
    let Some(engine) = ENGINE.get() else {
        return Captured::Notice("Cannot capture: no player".to_owned());
    };
    let failed =
        |error: &dyn std::fmt::Display| Captured::Notice(format!("Cannot capture: {error}"));
    if let Some(path) = &job.path
        && let Some(folder) = path.parent()
        && let Err(error) = std::fs::create_dir_all(folder)
    {
        return failed(&error);
    }
    if job.by_mpv() {
        let Some(path) = &job.path else {
            return failed(&"no file");
        };
        let quality = job.quality.to_string();
        drop(engine.set_text("screenshot-jpeg-quality", &quality));
        drop(engine.set_text("screenshot-webp-quality", &quality));
        let text = path.to_string_lossy().into_owned();
        return match engine.command("screenshot-to-file", &[&text, job.mode]) {
            Ok(()) => Captured::Notice(format!("Saved {}", view::file_name(&text))),
            Err(error) => failed(&error),
        };
    }
    let image = match engine.screenshot_raw(job.mode) {
        Ok(image) => image,
        Err(error) => return failed(&error),
    };
    let mut rgba = capture::rgba_pixels(&image);
    let mut size = (
        u32::try_from(image.width).unwrap_or(0),
        u32::try_from(image.height).unwrap_or(0),
    );
    if let Some(stored) = job.stored {
        rgba = capture::scale_rgba(&rgba, size, stored);
        size = stored;
    }
    let (Ok(width), Ok(height)) = (i32::try_from(size.0), i32::try_from(size.1)) else {
        return failed(&"the frame is too large");
    };
    match &job.path {
        Some(path) => {
            let text = path.to_string_lossy().into_owned();
            let quality = i32::try_from(job.quality).unwrap_or(90);
            if rvp_save_image(&rgba, width, height, &text, quality, "") {
                Captured::Notice(format!("Saved {}", view::file_name(&text)))
            } else {
                failed(&format!("{text} could not be written"))
            }
        }
        None => Captured::Clipboard(rgba, width, height),
    }
}

impl qobject::Player {
    /// Opens a dialog of `VideoDialogs.qml` with its data.
    pub(super) fn open_video_dialog(mut self: Pin<&mut Self>, data: &Value) {
        self.as_mut().set_dialog(QString::from(&data.to_string()));
        self.ui_request(request::VIDEO_DIALOG, 0);
    }

    /// A window size or move for `Main.qml` to carry out.
    pub(super) fn window_request(mut self: Pin<&mut Self>, request: &WindowRequest) {
        match request {
            WindowRequest::SizeDialog { width, height } => {
                self.open_video_dialog(&json!({
                    "kind": "input", "purpose": "window-size", "title": "Set Custom Window Size",
                    "label": "The window's size in pixels (960x600)",
                    "value": format!("{width}x{height}"),
                }));
            }
            WindowRequest::ControlDialog => {
                self.open_video_dialog(&json!({ "kind": "window-control" }));
            }
            other => {
                let text = serde_json::to_string(other).unwrap_or_default();
                self.as_mut().set_dialog(QString::from(&text));
                self.ui_request(request::WINDOW, 0);
            }
        }
    }

    /// A save dialog of `VideoDialogs.qml` for a recording, suggesting
    /// `path`; the answer comes back with `purpose`.
    fn save_dialog(self: Pin<&mut Self>, purpose: &str, title: &str, path: &std::path::Path) {
        self.open_video_dialog(&json!({
            "kind": "save", "purpose": purpose, "title": title,
            "file": super::file_url(path),
            "filters": ["Matroska (*.mkv)", "MPEG-TS (*.ts)", "MP4 (*.mp4)", "All files (*)"],
        }));
    }

    /// Encodes a video recording with a core of its own, on a thread of its
    /// own; the notice comes back to the window's thread.
    pub(super) fn encode(self: Pin<&mut Self>, job: rvp_engine::encode::EncodeJob) {
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name("rvp-encode".to_owned())
            .spawn(move || {
                let name = view::file_name(&job.output.to_string_lossy()).to_owned();
                let created = job.output.parent().map_or(Ok(()), std::fs::create_dir_all);
                // A recording takes at most as long as the media's stretch,
                // and encoding is usually faster than playback.
                let limit =
                    std::time::Duration::from_secs_f64((job.end - job.start).max(1.0) * 4.0 + 60.0);
                let result = created.map_err(|error| error.to_string()).and_then(|()| {
                    rvp_engine::encode::encode(&job, limit).map_err(|error| error.to_string())
                });
                let notice = match result {
                    Ok(()) => format!("Recorded video: {name}"),
                    Err(error) => format!("Cannot record the video: {error}"),
                };
                let _ = thread.queue(move |player| player.set_osd(QString::from(&notice)));
            });
        if let Err(error) = spawned {
            eprintln!("rvp: cannot record the video: {error}");
        }
    }

    /// Captures a frame on the capture thread; the notice (or the
    /// clipboard's picture) comes back to the window's thread.
    pub(super) fn capture(self: Pin<&mut Self>, job: CaptureJob) {
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name("rvp-frame".to_owned())
            .spawn(move || {
                let captured = run_capture(&job);
                let _ = thread.queue(move |player| match captured {
                    Captured::Notice(text) => player.set_osd(QString::from(&text)),
                    Captured::Clipboard(rgba, width, height) => {
                        let text = if rvp_copy_image(&rgba, width, height) {
                            "Copied the frame to the clipboard"
                        } else {
                            "Cannot copy the frame"
                        };
                        player.set_osd(QString::from(text));
                    }
                });
            });
        if let Err(error) = spawned {
            eprintln!("rvp: cannot capture: {error}");
        }
    }

    /// Writes a finished contact sheet on the capture thread.
    pub(super) fn save_sheet(self: Pin<&mut Self>, sheet: SheetImage) {
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name("rvp-frame".to_owned())
            .spawn(move || {
                let text = sheet.path.to_string_lossy().into_owned();
                let texts = serde_json::to_string(&sheet.texts).unwrap_or_default();
                let saved = match (i32::try_from(sheet.width), i32::try_from(sheet.height)) {
                    (Ok(width), Ok(height)) => rvp_save_image(
                        &sheet.rgba,
                        width,
                        height,
                        &text,
                        i32::try_from(sheet.quality).unwrap_or(90),
                        &texts,
                    ),
                    _ => false,
                };
                let notice = if saved {
                    format!("Saved the thumbnail image {}", view::file_name(&text))
                } else {
                    format!("Cannot write the thumbnail image {text}")
                };
                let _ = thread.queue(move |player| player.set_osd(QString::from(&notice)));
            });
        if let Err(error) = spawned {
            eprintln!("rvp: cannot write the thumbnail image: {error}");
        }
    }

    /// M12's requests.
    pub(super) fn m12_request(self: Pin<&mut Self>, request: UiRequest) {
        match request {
            UiRequest::Window(window) => self.window_request(&window),
            UiRequest::RecordStreamDialog(path) => {
                self.save_dialog("record-stream", "Record Stream", &path);
            }
            UiRequest::RecordVideoDialog(path) => {
                self.save_dialog("record-video", "Record Video", &path);
            }
            UiRequest::VideoFolderDialog => {
                let folder = self
                    .core()
                    .map(|core| core.app.video_folder())
                    .unwrap_or_default();
                self.open_video_dialog(&json!({
                    "kind": "folder", "purpose": "video-folder",
                    "title": "Set Video Storage Folder", "folder": super::file_url(&folder),
                }));
            }
            UiRequest::Encode(job) => self.encode(job),
            UiRequest::Slide(step) => self.ui_request(request::SLIDE, step),
            UiRequest::CaptureDialog(kind) => {
                let Some(core) = self.core() else {
                    return;
                };
                let capture = &core.app.settings.capture;
                let data = if kind == "sheet" {
                    json!({ "kind": "sheet", "purpose": "sheet", "sheet": capture.sheet })
                } else {
                    json!({
                        "kind": "consecutive", "purpose": "consecutive",
                        "interval": capture.interval, "count": capture.count,
                    })
                };
                self.open_video_dialog(&data);
            }
            UiRequest::PictureDialog { kind, value } => {
                let (title, label) = match kind {
                    "custom-aspect" => (
                        "Set Custom Aspect Ratio",
                        "Aspect ratio (16:9, 2.35:1 or 1.85)",
                    ),
                    _ => (
                        "Extend/Crop by pixel",
                        "The frame's size in pixels (1920x1080)",
                    ),
                };
                self.open_video_dialog(&json!({
                    "kind": "input", "purpose": kind, "title": title, "label": label,
                    "value": value,
                }));
            }
            other => self.m13_request(other),
        }
    }

    pub(crate) fn set_screen(mut self: Pin<&mut Self>, json: &QString) {
        let Ok(value) = serde_json::from_str::<Value>(&json.to_string()) else {
            return;
        };
        let number = |key: &str| value.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        let pixels = |key: &str| u32::try_from(value.get(key).and_then(Value::as_u64).unwrap_or(0));
        let screen = rvp_app::picture::Screen {
            width: pixels("width").unwrap_or(0),
            height: pixels("height").unwrap_or(0),
            width_mm: number("width_mm"),
            height_mm: number("height_mm"),
            refresh: number("refresh"),
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.app.set_screen(core.engine, screen);
    }

    pub(crate) fn confine_cursor(self: Pin<&mut Self>, wanted: bool) {
        let on = wanted
            && self
                .core()
                .is_some_and(|core| core.app.settings.window.restrict_cursor);
        crate::ffi::bridge::rvp_confine_cursor(on);
    }

    pub(crate) fn video_dialog_answer(mut self: Pin<&mut Self>, purpose: &QString, text: &QString) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let purpose = purpose.to_string();
        let request = match purpose.as_str() {
            kind @ ("custom-aspect" | "extend-size") => {
                core.app
                    .picture_dialog_answer(core.engine, kind, &text.to_string())
            }
            "window-size" => Some(core.app.window_size_answer(&text.to_string())),
            "consecutive" => core.app.consecutive_answer(&text.to_string()),
            "record-stream" => Some(
                core.app
                    .start_stream_record(core.engine, std::path::Path::new(&text.to_string())),
            ),
            "record-video" => Some(core.app.start_video_record(text.to_string().into())),
            "video-folder" => {
                core.app
                    .set_video_folder(std::path::Path::new(&text.to_string()));
                None
            }
            "sheet" => core.app.sheet_answer(&text.to_string()),
            "save-subtitle" => Some(
                core.app
                    .save_subtitle_as(core.engine, std::path::Path::new(&text.to_string())),
            ),
            _ => None,
        };
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }
}
