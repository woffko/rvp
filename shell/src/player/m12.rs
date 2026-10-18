//! M12's part of the window (picture, video, window and capture): the
//! screen the window is on, and the dialogs of `VideoDialogs.qml` with the
//! answers they bring back.

use std::pin::Pin;

use cxx_qt_lib::QString;
use rvp_app::UiRequest;
use rvp_app::window::WindowRequest;
use serde_json::{Value, json};

use super::request;
use crate::player_qobject::qobject;

impl qobject::Player {
    /// Opens a dialog of `VideoDialogs.qml` with its data.
    fn open_video_dialog(mut self: Pin<&mut Self>, data: &Value) {
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

    /// M12's requests.
    pub(super) fn m12_request(self: Pin<&mut Self>, request: UiRequest) {
        match request {
            UiRequest::Window(window) => self.window_request(&window),
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
            other => eprintln!("rvp: unhandled request {other:?}"),
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
        };
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        core.app.set_screen(core.engine, screen);
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
            _ => None,
        };
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }
}
