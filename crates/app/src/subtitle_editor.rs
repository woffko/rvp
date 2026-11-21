//! M13: RVP's own work on subtitle files (goal O): saving them with the
//! sync applied (PotPlayer's Subtitle Saving), the subtitle browser and
//! editor (Browse/Edit/Sync Subtitles, Alt+E), Input Subtitles (Alt+P) and
//! sync changes from the next subtitle on.
//!
//! * mpv renders; RVP reads the file of the subtitle that shows (an
//!   external SRT, ASS or `WebVTT` file: `track-list`'s
//!   `external-filename`) with `rvp-subtitle`. libmpv gives no access to
//!   the lines of a track inside the video, so those cannot be saved or
//!   edited.
//! * Edits show at once: RVP writes the cues to a working copy in its state
//!   folder (`subtitles/editing-<pid>.<ext>`, each RVP's own, removed when
//!   it closes) and gives mpv that file in place of the track (`sub-add`
//!   with the source's name as its title, then `sub-remove` of the old
//!   track).
//! * The sync applied: a cue's time `t` shows at `t * speed + delay`, where
//!   the speed is the rate the subtitles were made for over the video's
//!   (`sub-fps`) and the delay is `sub-delay` (mpv's `dec_sub.c`). Saving
//!   writes the times as they show; saving into the file mpv shows also
//!   sets the sync back to none, as the file now carries it.
//! * A file RVP writes over is copied to `subtitles/backup/` first, under a
//!   name no file has there (`clip.srt`, then `clip (2).srt`...): a backup
//!   is never written over, so the first one keeps the original. Files are
//!   written to a temporary file of their own and renamed, without a flush
//!   (the window's thread waits for no disk); a subtitle that is a symbolic
//!   link stays one ([`crate::storage::write_user_file_unflushed`]). The
//!   backups, working copies and unsaved edits are RVP's own files, the
//!   user's alone (`crate::storage`).
//! * Edits not saved when another file starts or RVP closes are kept in
//!   `subtitles/unsaved-<name>` (a free name, as the backups'), with a
//!   message saying so.

use std::path::{Path, PathBuf};

use rvp_engine::{Engine, PropertyId};
use rvp_subtitle::{Cue, Document, Editor, Format};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::app::{App, UiRequest};

/// The subtitle browser's choices for the cues' times (Time visibility).
pub const BROWSER_TIMES: &[&str] = &["start-end", "start", "none"];

/// How long a subtitle typed while the video stands lasts, and the
/// shortest typed one (milliseconds).
const INPUT_LENGTH_MS: i64 = 3000;
const INPUT_MIN_MS: i64 = 1000;
/// How long a cue the browser inserts lasts.
const INSERT_LENGTH_MS: i64 = 2000;

/// The subtitle browser's settings (PotPlayer's Subtitle Browser page).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BrowserSettings {
    /// The cues' times: one of [`BROWSER_TIMES`].
    pub times: String,
    /// The times as they show, with the sync, rather than the file's.
    pub synced_times: bool,
    /// The window's opacity in percent (text and background alike: a
    /// framed window is see-through only as a whole).
    pub opacity: f64,
    /// The list's font (empty: the window's) and size in points.
    pub font: String,
    pub font_size: i64,
    /// The lines of text each cue shows.
    pub lines: i64,
    /// The list follows the subtitle that shows.
    pub follow: bool,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            times: "start-end".to_owned(),
            synced_times: true,
            opacity: 100.0,
            font: String::new(),
            font_size: 10,
            lines: 2,
            follow: true,
        }
    }
}

impl BrowserSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !BROWSER_TIMES.contains(&self.times.as_str()) {
            notes.push(format!(
                "subtitles.browser.times {:?} is none of {BROWSER_TIMES:?}: start-end",
                self.times
            ));
            "start-end".clone_into(&mut self.times);
        }
        if !self.opacity.is_finite() || !(20.0..=100.0).contains(&self.opacity) {
            notes.push(format!(
                "subtitles.browser.opacity {} is out of 20 to 100: 100",
                self.opacity
            ));
            self.opacity = 100.0;
        }
        if !(6..=48).contains(&self.font_size) {
            notes.push(format!(
                "subtitles.browser.font-size {} is out of 6 to 48: 10",
                self.font_size
            ));
            self.font_size = 10;
        }
        if !(1..=5).contains(&self.lines) {
            notes.push(format!(
                "subtitles.browser.lines {} is out of 1 to 5: 2",
                self.lines
            ));
            self.lines = 2;
        }
    }
}

/// The subtitle file RVP has open for the browser, the editor, Input
/// Subtitles and sync changes from the next subtitle on.
#[derive(Debug)]
pub(crate) struct OpenSubtitle {
    /// The file it was read from; `None` for subtitles typed in.
    source: Option<PathBuf>,
    /// mpv's track that shows it (the source's or the working copy's).
    track: Option<i64>,
    editor: Editor,
    /// mpv shows RVP's working copy rather than the source.
    copy: bool,
    /// Input Subtitles: where the line being typed began (the cue's time).
    input_start: Option<i64>,
}

impl OpenSubtitle {
    /// New subtitles typed in (Input Subtitles), written as SRT.
    fn typed() -> Self {
        Self {
            source: None,
            track: None,
            editor: Editor::new(Document::new(Format::Srt)),
            copy: false,
            input_start: None,
        }
    }

    fn name(&self) -> String {
        self.source.as_ref().map_or_else(
            || "Typed subtitles".to_owned(),
            |path| crate::view::file_name(&path.to_string_lossy()).to_owned(),
        )
    }
}

/// Milliseconds of `seconds`.
pub(crate) fn milliseconds(seconds: f64) -> i64 {
    // Times of a video are far within i64's milliseconds.
    #[allow(clippy::cast_possible_truncation)]
    let ms = (seconds * 1000.0).round() as i64;
    ms
}

/// Seconds of `ms`.
fn seconds(ms: i64) -> f64 {
    // Times of a video are far within f64's exact integers.
    #[allow(clippy::cast_precision_loss)]
    let seconds = ms as f64 / 1000.0;
    seconds
}

/// The subtitle track that shows (`sid`) and its file, if it has one.
fn shown_subtitle(engine: &Engine) -> Result<(i64, Option<PathBuf>, Option<String>), String> {
    let sid = engine
        .get_text("sid")
        .and_then(|text| text.parse::<i64>().ok())
        .ok_or_else(|| "No subtitle shows".to_owned())?;
    let list = engine.get_json("track-list").unwrap_or(Value::Null);
    let track = list
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .find(|track| track["type"] == "sub" && track["id"].as_i64() == Some(sid))
        .ok_or_else(|| "No subtitle shows".to_owned())?;
    let path = track["external-filename"].as_str().map(PathBuf::from);
    let language = track["lang"].as_str().map(str::to_owned);
    Ok((sid, path, language))
}

/// The highest ID of the external subtitle tracks: the one just added.
pub(crate) fn newest_external(engine: &Engine) -> Option<i64> {
    engine
        .tracks()
        .into_iter()
        .filter(|track| track.kind == rvp_engine::TrackKind::Subtitle && track.external)
        .map(|track| track.id)
        .max()
}

/// How many names [`create_free`] tries.
const FREE_NAMES: u32 = 10_000;

/// Creates the first file of `<stem>.<extension>`, `<stem> (2).<extension>`
/// ... that is not in `dir` (a name a file or a link has is never opened).
fn create_free(
    dir: &Path,
    stem: &str,
    extension: &str,
) -> std::io::Result<(PathBuf, std::fs::File)> {
    crate::storage::create_private_dir(dir)?;
    let mut options = crate::storage::private_options();
    options.write(true).create_new(true);
    let dot = if extension.is_empty() { "" } else { "." };
    for number in 1..=FREE_NAMES {
        let name = if number == 1 {
            format!("{stem}{dot}{extension}")
        } else {
            format!("{stem} ({number}){dot}{extension}")
        };
        let path = dir.join(name);
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other(format!(
        "{FREE_NAMES} files named {stem} are there already"
    )))
}

/// Writes `bytes` into a new file of a free name ([`create_free`]).
fn write_free(dir: &Path, stem: &str, extension: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let (path, mut file) = create_free(dir, stem, extension)?;
    if let Err(error) = std::io::Write::write_all(&mut file, bytes) {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

/// The stem and extension of `path`'s name (`clip`, `srt`).
fn stem_and_extension(path: &Path) -> (String, String) {
    let part = |part: Option<&std::ffi::OsStr>| {
        part.map(|part| part.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    (part(path.file_stem()), part(path.extension()))
}

impl App {
    /// RVP's folder for working copies, backups and unsaved edits.
    fn subtitle_folder(&self) -> PathBuf {
        self.state_dir().join("subtitles")
    }

    /// The speed and delay (ms) at which subtitles show against their own
    /// times (see the module's documentation).
    fn subtitle_timing(&self) -> (f64, f64) {
        let delay = self.model.double(PropertyId::SubDelay).unwrap_or(0.0) * 1000.0;
        let video = self
            .model
            .double(PropertyId::ContainerFps)
            .filter(|fps| *fps > 0.0);
        let speed = match (self.subtitle_fps(), video) {
            (Some(fps), Some(video)) => fps / video,
            _ => 1.0,
        };
        (speed, delay)
    }

    /// The playback position in the subtitles' own time.
    fn subtitle_now(&self) -> i64 {
        let (speed, delay) = self.subtitle_timing();
        let now = self.model.double(PropertyId::TimePos).unwrap_or(0.0) * 1000.0;
        milliseconds((now - delay) / speed / 1000.0)
    }

    /// Opens the subtitle file that shows for editing, unless it is open
    /// already; the reason when it cannot be. Edits not saved stay open
    /// while another track shows.
    fn open_shown_subtitle(&mut self, engine: &Engine) -> Result<(), String> {
        let (sid, path, _) = shown_subtitle(engine)?;
        if self
            .subtitle_editor
            .as_ref()
            .is_some_and(|open| open.track == Some(sid) || open.editor.is_dirty())
        {
            return Ok(());
        }
        let path = path.ok_or_else(|| {
            "The subtitle that shows is inside the video: libmpv gives RVP no access to its \
             lines (RVP edits and saves SRT, ASS and WebVTT files)"
                .to_owned()
        })?;
        let name = crate::view::file_name(&path.to_string_lossy()).to_owned();
        let bytes = std::fs::read(&path).map_err(|error| format!("Cannot read {name}: {error}"))?;
        let encoding = &self.settings.subtitles.encoding;
        let document =
            Document::parse_with_encoding(&bytes, encoding, Some(&name)).map_err(|_| {
                format!("RVP edits and saves SRT, ASS and WebVTT files: {name} is none of them")
            })?;
        self.subtitle_editor = Some(OpenSubtitle {
            source: Some(path),
            track: Some(sid),
            editor: Editor::new(document),
            copy: false,
            input_start: None,
        });
        Ok(())
    }

    /// The open subtitles, or new typed ones when no subtitle file shows
    /// (Input Subtitles).
    fn open_or_new_subtitle(&mut self, engine: &Engine) -> &mut OpenSubtitle {
        // Without a file that shows, the typed subtitles are new.
        drop(self.open_shown_subtitle(engine));
        self.subtitle_editor.get_or_insert_with(OpenSubtitle::typed)
    }

    /// Gives mpv the edited cues: a working copy in place of the track.
    fn show_edits(&mut self, engine: &Engine) -> Result<(), String> {
        let folder = self.subtitle_folder();
        let Some(open) = self.subtitle_editor.as_mut() else {
            return Ok(());
        };
        let format = open.editor.document().format;
        let copy = folder.join(crate::storage::process_file_name(
            "editing",
            format.extension(),
        ));
        crate::storage::write_unflushed(&copy, &open.editor.document().to_bytes(format))
            .map_err(|error| format!("Cannot write {}: {error}", copy.display()))?;
        let language = shown_subtitle(engine)
            .ok()
            .filter(|(sid, _, _)| Some(*sid) == open.track)
            .and_then(|(_, _, language)| language)
            .unwrap_or_default();
        let copy_text = copy.to_string_lossy();
        let name = open.name();
        engine
            .command("sub-add", &[&copy_text, "select", &name, &language])
            .map_err(|error| format!("Cannot show the edited subtitles: {error}"))?;
        let added = newest_external(engine);
        if let Some(old) = open.track.filter(|old| Some(*old) != added) {
            drop(engine.command("sub-remove", &[&old.to_string()]));
        }
        open.track = added;
        open.copy = true;
        self.refresh_subtitle_files(engine);
        Ok(())
    }

    /// The open subtitles with the sync applied (as they show).
    fn synced_document(&self) -> Option<Document> {
        let open = self.subtitle_editor.as_ref()?;
        let mut document = open.editor.document().clone();
        let video = self
            .model
            .double(PropertyId::ContainerFps)
            .filter(|fps| *fps > 0.0);
        if let (Some(fps), Some(video)) = (self.subtitle_fps(), video) {
            document.scale(fps, video);
        }
        document.shift(milliseconds(
            self.model.double(PropertyId::SubDelay).unwrap_or(0.0),
        ));
        Some(document)
    }

    /// The subtitle file named after the playing video, in its folder.
    fn video_subtitle_path(&self, format: Format) -> Option<PathBuf> {
        let video = self.model.loaded_path()?;
        if video.contains("://") {
            return None;
        }
        Some(Path::new(video).with_extension(format.extension()))
    }

    /// Puts a copy of `path` into RVP's backup folder before it is written
    /// over, under a name no backup has.
    fn back_up(&self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let failed =
            |error: std::io::Error| format!("Cannot keep a backup of {}: {error}", path.display());
        let mut source = std::fs::File::open(path).map_err(failed)?;
        let (stem, extension) = stem_and_extension(path);
        let (backup, mut file) =
            create_free(&self.subtitle_folder().join("backup"), &stem, &extension)
                .map_err(failed)?;
        if let Err(error) = std::io::copy(&mut source, &mut file) {
            drop(file);
            let _ = std::fs::remove_file(&backup);
            return Err(failed(error));
        }
        Ok(())
    }

    /// Saves the open subtitles with the sync applied to `target`. Saved
    /// into the file mpv shows (or as the typed subtitles' first file),
    /// they replace the track and the sync goes back to none.
    fn save_subtitle_to(&mut self, engine: &Engine, target: &Path) -> UiRequest {
        let Some(mut document) = self.synced_document() else {
            return UiRequest::Message("No subtitle file is open".to_owned());
        };
        let format = Format::from_name(&target.to_string_lossy()).unwrap_or(document.format);
        let name = crate::view::file_name(&target.to_string_lossy()).to_owned();
        if let Err(error) = self.back_up(target) {
            return UiRequest::Message(error);
        }
        if let Err(error) =
            crate::storage::write_user_file_unflushed(target, &document.to_bytes(format))
        {
            return UiRequest::Message(format!("Cannot save {name}: {error}"));
        }
        let delay = self.model.double(PropertyId::SubDelay).unwrap_or(0.0);
        let converted = self.subtitle_fps().is_some();
        let Some(open) = self.subtitle_editor.as_mut() else {
            return UiRequest::Message("No subtitle file is open".to_owned());
        };
        let replaces = open.source.is_none() || open.source.as_deref() == Some(target);
        if !replaces {
            return UiRequest::Osd(format!("Saved {name}"));
        }
        // The file now carries the sync: mpv shows it without one.
        document.format = format;
        open.editor = Editor::new(document);
        open.source = Some(target.to_path_buf());
        let old = open.track;
        let text = target.to_string_lossy();
        if engine.command("sub-add", &[&text, "select"]).is_ok() {
            let added = newest_external(engine);
            if let Some(old) = old.filter(|old| Some(*old) != added) {
                drop(engine.command("sub-remove", &[&old.to_string()]));
            }
            if let Some(open) = self.subtitle_editor.as_mut() {
                open.track = added;
                open.copy = false;
            }
        }
        drop(engine.set_double("sub-delay", 0.0));
        if converted {
            drop(self.set_subtitle_fps(engine, None));
        }
        self.subtitle_tracks_chosen(engine);
        self.refresh_subtitle_files(engine);
        if delay.abs() > 1e-6 || converted {
            UiRequest::Osd(format!(
                "Saved {name} with the sync ({})",
                crate::commands::signed_seconds(delay)
            ))
        } else {
            UiRequest::Osd(format!("Saved {name}"))
        }
    }

    /// Apply Subtitle Sync (Alt+S): the subtitle file that shows gets the
    /// sync written into it.
    pub(crate) fn apply_subtitle_sync(&mut self, engine: &Engine) -> UiRequest {
        if let Err(why) = self.open_shown_subtitle(engine)
            && self.subtitle_editor.is_none()
        {
            return UiRequest::Message(why);
        }
        let target = self.subtitle_editor.as_ref().and_then(|open| {
            open.source
                .clone()
                .or_else(|| self.video_subtitle_path(open.editor.document().format))
        });
        match target {
            Some(target) => self.save_subtitle_to(engine, &target),
            None => UiRequest::Message("The video is not a local file".to_owned()),
        }
    }

    /// Save Subtitle as Video Filename (Ctrl+Shift+S).
    pub(crate) fn save_subtitle_as_video_name(&mut self, engine: &Engine) -> UiRequest {
        if let Err(why) = self.open_shown_subtitle(engine)
            && self.subtitle_editor.is_none()
        {
            return UiRequest::Message(why);
        }
        let format = self
            .subtitle_editor
            .as_ref()
            .map_or(Format::Srt, |open| open.editor.document().format);
        match self.video_subtitle_path(format) {
            Some(target) => self.save_subtitle_to(engine, &target),
            None => UiRequest::Message("The video is not a local file".to_owned()),
        }
    }

    /// Save Subtitle As (Ctrl+Alt+S): the save dialog, suggesting the
    /// video's name.
    pub(crate) fn save_subtitle_dialog(&mut self, engine: &Engine) -> UiRequest {
        if let Err(why) = self.open_shown_subtitle(engine)
            && self.subtitle_editor.is_none()
        {
            return UiRequest::Message(why);
        }
        let format = self
            .subtitle_editor
            .as_ref()
            .map_or(Format::Srt, |open| open.editor.document().format);
        let suggested = self
            .video_subtitle_path(format)
            .or_else(|| {
                self.subtitle_editor
                    .as_ref()
                    .and_then(|open| open.source.clone())
            })
            .unwrap_or_else(|| PathBuf::from(format!("subtitles.{}", format.extension())));
        UiRequest::SaveSubtitleDialog(suggested)
    }

    /// The save dialog's answer.
    pub fn save_subtitle_as(&mut self, engine: &Engine, path: &Path) -> UiRequest {
        if self.subtitle_editor.is_none() {
            return UiRequest::Message("No subtitle file is open".to_owned());
        }
        self.save_subtitle_to(engine, path)
    }

    /// A sync step with "Apply sync changes starting from next position":
    /// the cues after the playback position move, the others stay. `None`
    /// when the step goes to the whole track (the option is off, or no
    /// subtitle file RVP edits shows).
    pub(crate) fn sync_from_next(&mut self, engine: &Engine, seconds: f64) -> Option<UiRequest> {
        if !self.settings.subtitles.sync_from_next || self.open_shown_subtitle(engine).is_err() {
            return None;
        }
        let now = self.subtitle_now();
        let open = self.subtitle_editor.as_mut()?;
        let later: Vec<usize> = open
            .editor
            .cues()
            .iter()
            .enumerate()
            .filter(|(_, cue)| cue.start_ms > now && !cue.is_comment())
            .map(|(index, _)| index)
            .collect();
        if later.is_empty() {
            return Some(UiRequest::Osd("No next subtitle to sync".to_owned()));
        }
        open.editor.shift(&later, milliseconds(seconds));
        Some(match self.show_edits(engine) {
            Ok(()) => UiRequest::Osd(format!(
                "Subtitle sync from the next subtitle: {}",
                crate::commands::signed_seconds(seconds)
            )),
            Err(error) => UiRequest::Message(error),
        })
    }

    /// Another file started or RVP closes: unsaved edits are kept in RVP's
    /// folder; the message says where.
    pub(crate) fn close_subtitle_editor(&mut self) -> Option<UiRequest> {
        let open = self.subtitle_editor.take()?;
        if !open.editor.is_dirty() || open.editor.cues().is_empty() {
            return None;
        }
        let document = open.editor.document();
        let stem = open
            .source
            .as_deref()
            .map_or_else(|| "typed".to_owned(), |path| stem_and_extension(path).0);
        // A free name: the edits kept before stay as they are.
        Some(UiRequest::Message(
            match write_free(
                &self.subtitle_folder(),
                &format!("unsaved-{stem}"),
                document.format.extension(),
                &document.to_bytes(document.format),
            ) {
                Ok(kept) => format!("Unsaved subtitle edits are kept in {}", kept.display()),
                Err(error) => format!("Unsaved subtitle edits were lost: {error}"),
            },
        ))
    }

    /// The subtitle browser's data (JSON): the cues with their own times,
    /// how they show (`speed`, `delay`), the editor's state and the
    /// browser's settings; `error` says why no file is open.
    pub fn subtitle_browser_json(&mut self, engine: &Engine) -> String {
        // Typed subtitles stay open while another track shows.
        let error = match self.open_shown_subtitle(engine) {
            Err(why) if self.subtitle_editor.is_none() => why,
            _ => String::new(),
        };
        let (speed, delay) = self.subtitle_timing();
        let settings = &self.settings.subtitles.browser;
        let data = match &self.subtitle_editor {
            Some(open) => {
                let cues: Vec<Value> = open
                    .editor
                    .cues()
                    .iter()
                    .map(
                        |cue| json!({ "start": cue.start_ms, "end": cue.end_ms, "text": cue.text }),
                    )
                    .collect();
                json!({
                    "error": error, "name": open.name(),
                    "format": open.editor.document().format.extension(),
                    "dirty": open.editor.is_dirty(), "undo": open.editor.can_undo(),
                    "redo": open.editor.can_redo(), "cues": cues,
                    "speed": speed, "delay": delay, "settings": settings,
                })
            }
            None => json!({
                "error": error, "name": "", "format": "", "dirty": false, "undo": false,
                "redo": false, "cues": [], "speed": speed, "delay": delay, "settings": settings,
            }),
        };
        data.to_string()
    }

    /// The cue that shows now, or else the next one (-1: none): where the
    /// browser's list goes.
    #[must_use]
    pub fn subtitle_cue_now(&self) -> i64 {
        self.subtitle_editor
            .as_ref()
            .and_then(|open| open.editor.cue_at(self.subtitle_now()))
            .and_then(|index| i64::try_from(index).ok())
            .unwrap_or(-1)
    }

    /// The browser and Input Subtitles: `op` on the cue `index` (text and
    /// times in `text`, clock times as `0:01:02.500`). Edits show at once.
    pub fn subtitle_edit(
        &mut self,
        engine: &Engine,
        op: &str,
        index: i64,
        text: &str,
    ) -> Option<UiRequest> {
        match op {
            "save" => return Some(self.apply_subtitle_sync(engine)),
            "save-as" => return Some(self.save_subtitle_dialog(engine)),
            "input-begin" => {
                let now = self.subtitle_now();
                let open = self.open_or_new_subtitle(engine);
                if open.input_start.is_none() {
                    open.input_start = Some(now);
                }
                return None;
            }
            "input" => return self.input_subtitle(engine, text),
            _ => {}
        }
        if self.subtitle_editor.is_none() {
            return Some(UiRequest::Message("No subtitle file is open".to_owned()));
        }
        let now = self.subtitle_now();
        let (speed, delay) = self.subtitle_timing();
        let open = self.subtitle_editor.as_mut()?;
        let index = usize::try_from(index).ok();
        let cue = index.and_then(|at| open.editor.cues().get(at).cloned());
        let editor = &mut open.editor;
        let changed = match (op, index, cue) {
            ("jump", _, Some(cue)) => {
                let at = seconds(cue.start_ms) * speed + delay / 1000.0;
                drop(engine.seek(rvp_engine::Seek::Absolute(at.max(0.0))));
                return None;
            }
            ("text", Some(at), _) => editor.set_text(at, text),
            ("start", Some(at), Some(cue)) => rvp_subtitle::parse_clock(text)
                .and_then(|start| editor.set_times(at, start, cue.end_ms.max(start)))
                .is_some(),
            ("end", Some(at), Some(cue)) => rvp_subtitle::parse_clock(text)
                .and_then(|end| editor.set_times(at, cue.start_ms, end))
                .is_some(),
            ("start-now", Some(at), Some(cue)) => {
                editor.set_times(at, now, cue.end_ms.max(now)).is_some()
            }
            ("end-now", Some(at), Some(cue)) => editor.set_times(at, cue.start_ms, now).is_some(),
            // This cue and the later ones move so that it starts now.
            ("sync-here", Some(at), Some(cue)) => {
                let later: Vec<usize> = (at..editor.cues().len()).collect();
                editor.shift(&later, now - cue.start_ms)
            }
            ("insert", _, _) => {
                editor.insert(Cue::new(now, now + INSERT_LENGTH_MS, String::new()));
                true
            }
            ("delete", Some(at), _) => editor.delete(at),
            ("split", Some(at), Some(cue)) => {
                let at_ms = if cue.start_ms < now && now < cue.end_ms {
                    now
                } else {
                    i64::midpoint(cue.start_ms, cue.end_ms)
                };
                editor.split(at, at_ms)
            }
            ("join", Some(at), _) => editor.join_next(at),
            ("shift", Some(at), _) => {
                let later: Vec<usize> = (at..editor.cues().len()).collect();
                text.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|ms| ms.is_finite())
                    .is_some_and(|ms| editor.shift(&later, milliseconds(ms / 1000.0)))
            }
            ("undo", _, _) => editor.undo(),
            ("redo", _, _) => editor.redo(),
            _ => false,
        };
        if !changed {
            return None;
        }
        self.show_edits(engine).err().map(UiRequest::Message)
    }

    /// Input Subtitles' Enter: the line typed becomes a cue from where its
    /// typing began up to now (at least [`INPUT_MIN_MS`]; while the video
    /// stands, [`INPUT_LENGTH_MS`]).
    fn input_subtitle(&mut self, engine: &Engine, text: &str) -> Option<UiRequest> {
        let text = text.trim();
        let now = self.subtitle_now();
        let open = self.open_or_new_subtitle(engine);
        let start = open.input_start.take().unwrap_or(now);
        if text.is_empty() {
            return None;
        }
        let end = if now - start < INPUT_MIN_MS {
            start + INPUT_LENGTH_MS
        } else {
            now
        };
        let mut cue = Cue::new(start, end, text.replace("\r\n", "\n"));
        if open.editor.document().format == Format::Ass {
            cue.style = Some("Default".to_owned());
        }
        open.editor.insert(cue);
        Some(match self.show_edits(engine) {
            Ok(()) => UiRequest::Osd(format!(
                "Subtitle added at {}",
                rvp_subtitle::format_clock(start, '.')
            )),
            Err(error) => UiRequest::Message(error),
        })
    }
}
