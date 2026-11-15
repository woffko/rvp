//! M12's recording: PotPlayer's Record Stream (Ctrl+A), which saves the
//! source as it comes, without decoding (mpv's `stream-record`, the
//! container from the file's extension), and the Video Recording menu's
//! folders. Record Video, the picture with RVP's filters, is encoded by a
//! second libmpv core (`crate::encode`, see `docs/spikes/m12-studies.md`).
//!
//! A stream recording ends with its file: mpv would start the next file's
//! recording over the same name.

use std::path::{Path, PathBuf};

use rvp_engine::{Engine, PropertyId};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// The recordings' settings (`record` in `settings.json`): PotPlayer's
/// Stream and Video storage folders (General > Navigation).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RecordSettings {
    /// Where recorded streams go; unset: Videos/RVP in the home folder.
    pub stream_folder: Option<PathBuf>,
    /// Where recorded videos go; unset: the same.
    pub video_folder: Option<PathBuf>,
    /// "Store video files into play folders (if available)".
    pub next_to_media: bool,
}

/// One of the recording commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecordCommand {
    /// Record Stream...: asks for the file, or stops the recording.
    Stream,
    /// Start/Stop Stream Recording: into the stream folder at once.
    StreamToggle,
    /// Record Video...: asks for the file, or stops the recording.
    Video,
    /// Start/Stop Video Recording: into the video folder at once.
    VideoToggle,
    VideoFolderDialog,
    OpenVideoFolder,
    ToggleNextToMedia,
}

/// What is being recorded now.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RecordState {
    /// The stream recording's file.
    pub(crate) stream: Option<PathBuf>,
    /// The video recording: its file and the time it started at.
    pub(crate) video: Option<(PathBuf, f64)>,
}

/// The user's videos folder's RVP (Linux: `user-dirs.dirs`'s).
fn default_folder() -> PathBuf {
    crate::storage::user_folder("XDG_VIDEOS_DIR", "Videos").join("RVP")
}

impl App {
    /// The folder recorded videos go to: the media's own folder when the
    /// settings ask and it is a local folder, else the video folder.
    #[must_use]
    pub fn video_folder(&self) -> PathBuf {
        let record = &self.settings.record;
        if record.next_to_media
            && let Some(path) = self.model.loaded_path()
            && !crate::playlist::is_url(path)
            && let Some(folder) = Path::new(path).parent()
            && folder.is_dir()
        {
            return folder.to_path_buf();
        }
        record.video_folder.clone().unwrap_or_else(default_folder)
    }

    /// A new file for a recording of the playing file: its name and the
    /// time, with `extension`.
    fn record_path(&self, folder: &Path, extension: &str) -> Option<PathBuf> {
        let path = self.model.loaded_path()?;
        let stem = Path::new(crate::view::file_name(path))
            .file_stem()
            .map_or_else(
                || "RVP".to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            );
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        // Milliseconds of a media time are far inside u64.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let millis = (time.max(0.0) * 1000.0).round() as u64;
        let name = crate::capture::frame_name(&self.settings.capture, &stem, millis, "");
        Some(crate::capture::unused_path(
            folder,
            name.trim_end(),
            extension,
        ))
    }

    /// The recording commands.
    pub(crate) fn recording_command(
        &mut self,
        engine: &Engine,
        command: RecordCommand,
    ) -> Option<UiRequest> {
        match command {
            RecordCommand::Stream | RecordCommand::StreamToggle => {
                if self.record.stream.is_some() {
                    return Some(UiRequest::Osd(self.stop_stream_record(engine)));
                }
                let folder = self
                    .settings
                    .record
                    .stream_folder
                    .clone()
                    .unwrap_or_else(default_folder);
                let path = self.record_path(&folder, "mkv")?;
                if command == RecordCommand::Stream {
                    Some(UiRequest::RecordStreamDialog(path))
                } else {
                    Some(self.start_stream_record(engine, &path))
                }
            }
            RecordCommand::Video | RecordCommand::VideoToggle => {
                if self.record.video.is_some() {
                    return self.stop_video_record();
                }
                let path = self.record_path(&self.video_folder(), "mkv")?;
                if command == RecordCommand::Video {
                    Some(UiRequest::RecordVideoDialog(path))
                } else {
                    Some(self.start_video_record(path))
                }
            }
            RecordCommand::VideoFolderDialog => Some(UiRequest::VideoFolderDialog),
            RecordCommand::OpenVideoFolder => {
                let folder = self.video_folder();
                drop(std::fs::create_dir_all(&folder));
                Some(UiRequest::OpenExternal(folder))
            }
            RecordCommand::ToggleNextToMedia => {
                let record = &mut self.settings.record;
                record.next_to_media = !record.next_to_media;
                let on = record.next_to_media;
                self.save_settings();
                Some(UiRequest::Osd(format!(
                    "Store recorded videos into play folders: {}",
                    if on { "on" } else { "off" }
                )))
            }
        }
    }

    /// The check mark of a recording command.
    pub(crate) fn recording_checked(&self, command: RecordCommand) -> Option<bool> {
        match command {
            RecordCommand::Stream | RecordCommand::StreamToggle => {
                Some(self.record.stream.is_some())
            }
            RecordCommand::Video | RecordCommand::VideoToggle => Some(self.record.video.is_some()),
            RecordCommand::ToggleNextToMedia => Some(self.settings.record.next_to_media),
            _ => None,
        }
    }

    /// Starts recording the source into `path` (the dialog's answer).
    pub fn start_stream_record(&mut self, engine: &Engine, path: &Path) -> UiRequest {
        if self.model.loaded_path().is_none() {
            return UiRequest::Osd("Record Stream needs a file that plays".to_owned());
        }
        if let Some(folder) = path.parent()
            && let Err(error) = std::fs::create_dir_all(folder)
        {
            return UiRequest::Message(format!("Cannot record the stream: {error}"));
        }
        let text = path.to_string_lossy();
        if let Err(error) = engine.set_text("stream-record", &text) {
            return UiRequest::Message(format!("Cannot record the stream: {error}"));
        }
        if path
            .parent()
            .is_some_and(|folder| folder != default_folder())
        {
            self.settings.record.stream_folder = path.parent().map(Path::to_path_buf);
            self.save_settings();
        }
        self.record.stream = Some(path.to_path_buf());
        UiRequest::Osd(format!(
            "Recording the stream: {}",
            crate::view::file_name(&text)
        ))
    }

    /// Stops the stream recording.
    fn stop_stream_record(&mut self, engine: &Engine) -> String {
        drop(engine.set_text("stream-record", ""));
        match self.record.stream.take() {
            Some(path) => format!(
                "Recorded the stream: {}",
                crate::view::file_name(&path.to_string_lossy())
            ),
            None => "Stream recording stopped".to_owned(),
        }
    }

    /// The playing file ended or another one starts: a stream recording
    /// ends with it.
    pub(crate) fn record_file_ended(&mut self, engine: &Engine) -> Option<UiRequest> {
        self.record.stream.as_ref()?;
        Some(UiRequest::Osd(self.stop_stream_record(engine)))
    }

    /// Sets the folder recorded videos go to (the folder dialog's answer).
    pub fn set_video_folder(&mut self, folder: &Path) {
        self.settings.record.video_folder = Some(folder.to_path_buf());
        self.save_settings();
    }

    /// True while a stream is being recorded (the status).
    #[must_use]
    pub fn recording_stream(&self) -> bool {
        self.record.stream.is_some()
    }

    /// Starts a video recording into `path` from the current time (the
    /// dialog's answer, or Start/Stop Video Recording).
    pub fn start_video_record(&mut self, path: PathBuf) -> UiRequest {
        if self.model.int(PropertyId::VideoWidth).is_none() {
            return UiRequest::Osd("Record Video needs a video that plays".to_owned());
        }
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        let name = crate::view::file_name(&path.to_string_lossy()).to_owned();
        self.record.video = Some((path, time));
        UiRequest::Osd(format!(
            "Recording video from {}: {name}",
            crate::view::format_time(time)
        ))
    }

    /// Stops the video recording: a core of its own encodes the stretch
    /// from where it started to here, with RVP's video filters (never
    /// SVP's).
    fn stop_video_record(&mut self) -> Option<UiRequest> {
        let (path, start) = self.record.video.take()?;
        let source = self.model.loaded_path()?.to_owned();
        let end = self.model.double(PropertyId::TimePos).unwrap_or(start);
        if end <= start + 0.1 {
            return Some(UiRequest::Osd(
                "Video recording stopped: nothing to record".to_owned(),
            ));
        }
        let vf = self
            .video_filters()
            .iter()
            .map(|(_, entry)| entry.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("mkv")
            .to_owned();
        Some(UiRequest::Encode(rvp_engine::encode::EncodeJob {
            source,
            output: path,
            start,
            end,
            vf,
            extra: rvp_engine::encode::encoders(&extension),
        }))
    }

    /// True while a video is being recorded (the status).
    #[must_use]
    pub fn recording_video(&self) -> bool {
        self.record.video.is_some()
    }
}
