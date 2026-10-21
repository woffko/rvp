//! M13's part of `Player`: the subtitle browser and editor, Input
//! Subtitles (both `SubtitleBrowser.qml`) and the subtitle save dialog
//! (`VideoDialogs.qml`'s save dialog). The work is `rvp_app`'s
//! (`subtitle_editor`); QML asks for the cues as JSON and sends edits.

use std::pin::Pin;

use cxx_qt_lib::QString;
use rvp_app::UiRequest;
use serde_json::json;

use super::request;
use crate::player_qobject::qobject;

impl qobject::Player {
    pub(super) fn m13_request(self: Pin<&mut Self>, request: UiRequest) {
        match request {
            UiRequest::SubtitleEditor { input } => {
                self.ui_request(request::SUBTITLE_EDITOR, i32::from(input));
            }
            UiRequest::SaveSubtitleDialog(path) => self.open_video_dialog(&json!({
                "kind": "save", "purpose": "save-subtitle", "title": "Save Subtitle As",
                "file": super::file_url(&path),
                "filters": ["SubRip (*.srt)", "Advanced SubStation Alpha (*.ass)",
                            "WebVTT (*.vtt)", "All files (*)"],
            })),
            other => eprintln!("rvp: unhandled request {other:?}"),
        }
    }

    pub(crate) fn subtitle_browser_json(self: Pin<&mut Self>) -> QString {
        let Some(core) = self.core_mut() else {
            return QString::from("{}");
        };
        QString::from(&core.app.subtitle_browser_json(core.engine))
    }

    pub(crate) fn subtitle_cue_now(&self) -> i32 {
        self.core()
            .and_then(|core| i32::try_from(core.app.subtitle_cue_now()).ok())
            .unwrap_or(-1)
    }

    pub(crate) fn subtitle_edit(
        mut self: Pin<&mut Self>,
        op: &QString,
        index: i32,
        text: &QString,
    ) {
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let request = core.app.subtitle_edit(
            core.engine,
            &op.to_string(),
            i64::from(index),
            &text.to_string(),
        );
        if let Some(request) = request {
            self.as_mut().handle_request(request);
        }
        self.sync_chrome();
    }
}
