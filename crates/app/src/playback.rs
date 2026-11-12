//! M11's playback commands and settings (PotPlayer's Playback menu and its
//! Playback, Time Jump and Skip pages):
//!
//! * the seek bar's tooltip, preview and marks: the time under the pointer
//!   (and the chapter or bookmark there), a picture of that place from the
//!   thumbnail core ([`crate::thumbs`]), and the chapters, bookmarks and A-B
//!   points on the bar;
//! * keyframe steps (a relative keyframe seek of half a frame: mpv seeks
//!   forward to the next keyframe or back to the one before), the start of
//!   the current subtitle (`sub-seek 0`) and the Jump to Time/Frame dialog;
//! * the A-B repeat's fine moves (0.1 s), the current, previous and next
//!   subtitle lines (mpv's `sub-start` and `sub-end`; a line before or after
//!   is reached with `sub-seek` first and taken once playback restarts),
//!   the A-B list's sections ([`crate::marks`]), the seconds behind the
//!   current point, loop counts (`ab-loop-count`) and "Prevent backward
//!   navigation from point A" (seeks before A land on A);
//! * skipping the intro (a seek past it when a file starts, unless the
//!   resume position lies beyond), the ending (the album moves on that many
//!   seconds before the end) and chapters whose titles (or numbers) the
//!   setup lists;
//! * shuffle: Next and the end of a file pick a random entry of the album
//!   not played in this round (Previous goes back the way it came);
//! * bookmarks per file: added at the current time (with a title asked for
//!   when "Prompt before adding a bookmark" is on), listed after the
//!   chapters in Chapter/Bookmark, and deleted after their jump when "Auto
//!   delete the selected bookmarks" is on.

use std::collections::BTreeSet;

use rvp_engine::{Engine, EngineError, PropertyId, Seek};
use serde::{Deserialize, Serialize};

use crate::action::{AbRepeat, AfterPlayback, BookmarkCommand, PowerAction, Skip};
use crate::app::{App, UiRequest};

/// What happens after a file fails to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnFailure {
    /// `<Can't open>`, then the next playable entry.
    #[default]
    Next,
    /// `<Can't open>`, and playback stops.
    Stop,
}

/// Playback > Skip.
// Independent options as PotPlayer's menu lists them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SkipSettings {
    pub enabled: bool,
    pub intro: bool,
    pub intro_seconds: f64,
    pub ending: bool,
    pub ending_seconds: f64,
    pub chapters: bool,
    /// Chapters to skip: titles (a part of the title, any case) or
    /// numbers, separated by `;`.
    pub chapter_names: String,
}

impl Default for SkipSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intro: true,
            intro_seconds: 30.0,
            ending: false,
            ending_seconds: 30.0,
            chapters: false,
            chapter_names: "Opening;Ending;Intro;Credits".to_owned(),
        }
    }
}

/// The A-B repeat's settings (Section Repeat Settings).
// Independent options as PotPlayer's menu lists them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AbSettings {
    /// How often a section repeats; 0 repeats it endlessly.
    pub loops: u32,
    /// Keep the A-B lists across runs ("Remember A-B Repeat List").
    pub remember: bool,
    pub prevent_backward: bool,
    /// A repeat-current command turns the repeat off when it is on.
    pub toggle_current: bool,
    /// The fine move, in seconds.
    pub step: f64,
    /// "Repeat 5 Sec. behind current point": the seconds.
    pub behind: f64,
}

impl Default for AbSettings {
    fn default() -> Self {
        Self {
            loops: 0,
            remember: true,
            prevent_backward: false,
            toggle_current: true,
            step: 0.1,
            behind: 5.0,
        }
    }
}

/// Playback > Chapter/Bookmark.
// Independent options as PotPlayer's menu lists them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BookmarkSettings {
    pub prompt: bool,
    pub in_menu: bool,
    pub thumbnails: bool,
    pub auto_delete: bool,
}

impl Default for BookmarkSettings {
    fn default() -> Self {
        Self {
            prompt: false,
            in_menu: true,
            thumbnails: false,
            auto_delete: false,
        }
    }
}

/// Misc > When Playback Finished.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Finished {
    /// Keep the choice across runs ("Remember playback finished action").
    pub remember: bool,
    /// The choice kept while `remember` is on.
    pub after: AfterPlayback,
    pub action: PowerAction,
    /// The notice before a power action, in seconds.
    pub countdown: u32,
}

impl Default for Finished {
    fn default() -> Self {
        Self {
            remember: false,
            after: AfterPlayback::Nothing,
            action: PowerAction::Shutdown,
            countdown: 30,
        }
    }
}

/// M11's playback settings (`playback` in `settings.json`).
// Independent options as PotPlayer's pages list them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Playback {
    /// The time over the seek bar while hovering and seeking.
    pub seek_tooltip: bool,
    /// A picture of the place under the pointer over the seek bar.
    pub seek_preview: bool,
    /// The tooltip names the chapter or bookmark there.
    pub preview_title: bool,
    /// The tooltip shows the time.
    pub preview_time: bool,
    /// Chapters, bookmarks and A-B points on the seek bar.
    pub seek_markers: bool,
    /// Playback pauses while the mouse holds the seek bar.
    pub pause_while_seeking: bool,
    /// Exact seeks drop the frames before the target before the video
    /// filters (mpv's `hr-seek-framedrop`; the owner, 2026-11-10: with
    /// RIFE a seek took up to 4.5 s, every frame from the keyframe on going
    /// through it). Not in SVP mode unless RVP's own smoothing filter runs:
    /// SVP's filter needs them (`App::apply_seek_framedrop`).
    pub seek_skip_before_target: bool,
    /// After a seek while a smoothing filter runs, the target's frame from
    /// the thumbnail core shows over the video until the smoothed picture
    /// is there ([`App::seek_picture`]).
    pub seek_picture: bool,
    pub shuffle: bool,
    /// The pointer hides over the playing video in the window too (it
    /// always does in fullscreen).
    pub hide_pointer: bool,
    /// Playback closes when the album ends.
    pub close_at_end: bool,
    /// Played files (with their subtitles) are offered to the Recycle
    /// Bin.
    pub recycle_played: bool,
    /// Positions are remembered in audio files too.
    pub remember_audio_positions: bool,
    /// A new file keeps the speed (otherwise it starts at 1.0x).
    pub keep_speed: bool,
    /// Playback speed step.
    pub speed_step: f64,
    pub on_failure: OnFailure,
    /// The window centres on the screen when it fits a new video.
    pub center_window: bool,
    /// Rotation metadata turns the video.
    pub auto_rotate: bool,
    /// Entries take the file's title tag.
    pub retitle: bool,
    /// Next and Previous at an album's ends go on with the folder's files.
    pub folder_neighbours: bool,
    /// Audio files next to a video with its name are loaded too.
    pub external_audio: bool,
    /// More folders where external audio files are looked for (mpv
    /// `audio-file-paths`).
    pub audio_folders: Vec<std::path::PathBuf>,
    /// Windows' process priority (mpv `priority`): `idle`, `belownormal`,
    /// `normal`, `abovenormal` or `high`; at the next start.
    pub priority: String,
    pub skip: SkipSettings,
    pub ab: AbSettings,
    pub bookmarks: BookmarkSettings,
    pub finished: Finished,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            seek_tooltip: true,
            seek_preview: true,
            preview_title: true,
            preview_time: true,
            seek_markers: true,
            pause_while_seeking: false,
            seek_skip_before_target: true,
            seek_picture: true,
            shuffle: false,
            hide_pointer: true,
            close_at_end: false,
            recycle_played: false,
            remember_audio_positions: true,
            keep_speed: true,
            speed_step: 0.1,
            on_failure: OnFailure::Next,
            center_window: false,
            auto_rotate: true,
            retitle: false,
            folder_neighbours: false,
            external_audio: false,
            audio_folders: Vec::new(),
            priority: "normal".to_owned(),
            skip: SkipSettings::default(),
            ab: AbSettings::default(),
            bookmarks: BookmarkSettings::default(),
            finished: Finished::default(),
        }
    }
}

pub(crate) fn clamp_number(
    value: &mut f64,
    low: f64,
    high: f64,
    name: &str,
    notes: &mut Vec<String>,
) {
    let repaired = if value.is_finite() {
        value.clamp(low, high)
    } else {
        low
    };
    if (repaired - *value).abs() > f64::EPSILON || !value.is_finite() {
        notes.push(format!("{name} must be {low} to {high}: {repaired} used"));
        *value = repaired;
    }
}

impl Playback {
    /// Brings the values into their ranges, with a note per repair.
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        clamp_number(
            &mut self.speed_step,
            0.01,
            1.0,
            "playback.speed-step",
            notes,
        );
        let skip = &mut self.skip;
        clamp_number(
            &mut skip.intro_seconds,
            1.0,
            3600.0,
            "playback.skip.intro-seconds",
            notes,
        );
        clamp_number(
            &mut skip.ending_seconds,
            1.0,
            3600.0,
            "playback.skip.ending-seconds",
            notes,
        );
        if skip.chapter_names.len() > 500 || skip.chapter_names.chars().any(char::is_control) {
            notes.push("playback.skip.chapter-names must be one line: the default used".to_owned());
            skip.chapter_names = SkipSettings::default().chapter_names;
        }
        let ab = &mut self.ab;
        if ab.loops > 1000 {
            notes.push("playback.ab.loops must be 0 to 1000: 1000 used".to_owned());
            ab.loops = 1000;
        }
        clamp_number(&mut ab.step, 0.01, 10.0, "playback.ab.step", notes);
        clamp_number(&mut ab.behind, 1.0, 60.0, "playback.ab.behind", notes);
        if !PRIORITIES.contains(&self.priority.as_str()) {
            notes.push(format!(
                "playback.priority {:?} is not one of {}: normal used",
                self.priority,
                PRIORITIES.join(", ")
            ));
            "normal".clone_into(&mut self.priority);
        }
        let countdown = self.finished.countdown;
        if !(5..=300).contains(&countdown) {
            let repaired = countdown.clamp(5, 300);
            notes.push(format!(
                "playback.finished.countdown must be 5 to 300: {repaired} used"
            ));
            self.finished.countdown = repaired;
        }
    }
}

/// Windows' process priorities mpv takes (`priority`).
pub const PRIORITIES: &[&str] = &["idle", "belownormal", "normal", "abovenormal", "high"];

/// mpv's path list for `audio-file-paths` (the system's separator).
fn path_list(folders: &[std::path::PathBuf]) -> String {
    let separator = if cfg!(windows) { ";" } else { ":" };
    folders
        .iter()
        .map(|folder| folder.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(separator)
}

/// The mpv options the playback settings set: external audio files and
/// their folders, rotation metadata and (on Windows) the priority.
pub(crate) fn engine_options(playback: &Playback) -> Vec<(String, String)> {
    let auto = if playback.external_audio {
        "fuzzy"
    } else {
        "no"
    };
    let mut options = vec![("audio-file-auto".to_owned(), auto.to_owned())];
    if !playback.audio_folders.is_empty() {
        options.push((
            "audio-file-paths".to_owned(),
            path_list(&playback.audio_folders),
        ));
    }
    if !playback.auto_rotate {
        options.push(("video-rotate".to_owned(), "no".to_owned()));
    }
    if cfg!(windows) && playback.priority != "normal" {
        options.push(("priority".to_owned(), playback.priority.clone()));
    }
    options
}

/// Where the pointer is over the seek bar ([`crate::pointer`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeekHover {
    pub fraction: f64,
    /// The pointer's x and the bar's top, in window pixels.
    pub x: i32,
    pub top: i32,
    /// The mouse holds the bar.
    pub held: bool,
}

/// What the window shows over the seek bar.
#[derive(Debug, Clone, PartialEq)]
pub struct SeekTip {
    pub text: String,
    pub x: i32,
    pub top: i32,
    /// The time under the pointer.
    pub time: f64,
    /// The size of the preview asked for (it comes later through
    /// [`App::thumbnail_events`]), when there is one.
    pub preview: Option<(u32, u32)>,
}

/// The box seek bar previews fit in.
pub const PREVIEW_BOX: (u32, u32) = (192, 108);

/// What the playback commands keep while RVP runs.
// Independent facts of the file that plays, not a mode.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub(crate) struct PlayState {
    /// The file's chapters, read when it loads and when their number
    /// changes.
    pub chapters: Vec<rvp_engine::Chapter>,
    /// The chapter a skip left last (a skip does not repeat on it).
    pub skipped_chapter: Option<i64>,
    /// The ending of the file that plays was skipped.
    pub ending_skipped: bool,
    /// A subtitle section waits for the seek to the line before or after,
    /// and the line shown when it was asked for.
    pub subtitle_section: bool,
    pub line_before: Option<f64>,
    /// Shuffle: the entries of the playing album played in this round
    /// (paths), in order, and the generator's state.
    pub shuffle_round: Vec<String>,
    pub shuffle_state: u64,
    /// The protocols libmpv opens (read on first need).
    pub protocols: Vec<String>,
    /// Playback paused while the mouse holds the seek bar.
    pub paused_for_seek: bool,
    /// Per-file options for the next file that loads (the command line's
    /// switches), and whether they set its start.
    pub next_options: Vec<(String, String)>,
    pub start_given: bool,
}

/// Splits `h:mm:ss.ms`, `m:ss`, or seconds into seconds; a leading `+` or
/// `-` makes it relative (`Some(sign)`).
#[must_use]
pub fn parse_time(text: &str) -> Option<(f64, Option<f64>)> {
    let text = text.trim();
    let (sign, rest) = match text.as_bytes().first() {
        Some(b'+') => (Some(1.0), &text[1..]),
        Some(b'-') => (Some(-1.0), &text[1..]),
        _ => (None, text),
    };
    let parts: Vec<&str> = rest.trim().split(':').collect();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|part| part.trim().is_empty()) {
        return None;
    }
    let mut seconds = 0.0;
    for (index, part) in parts.iter().enumerate() {
        let value: f64 = part.trim().parse().ok()?;
        let last = index + 1 == parts.len();
        if !value.is_finite() || value < 0.0 || (!last && value.fract() != 0.0) {
            return None;
        }
        // Minutes and seconds below the hours stay below 60.
        if index > 0 && value >= 60.0 {
            return None;
        }
        seconds = seconds * 60.0 + value;
    }
    Some((seconds, sign))
}

/// True when `title` (or the chapter's number, from 1) is one of the
/// `;`-separated names.
fn chapter_listed(names: &str, title: Option<&str>, number: usize) -> bool {
    names
        .split(';')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .any(|name| match name.parse::<usize>() {
            Ok(listed) => listed == number,
            Err(_) => {
                title.is_some_and(|title| title.to_lowercase().contains(&name.to_lowercase()))
            }
        })
}

/// `0:01:23` for A-B notices.
fn clock(seconds: f64) -> String {
    // Media times are far inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = seconds.max(0.0).round() as u64;
    format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
}

/// `30 Sec`, `1 Min`, `1 Min 30 Sec` for the Skip menu.
fn duration_label(seconds: f64) -> String {
    // Skip lengths are whole seconds below an hour.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = seconds.max(0.0).round() as u64;
    match (total / 60, total % 60) {
        (0, seconds) => format!("{seconds} Sec"),
        (minutes, 0) => format!("{minutes} Min"),
        (minutes, seconds) => format!("{minutes} Min {seconds} Sec"),
    }
}

impl App {
    // The seek bar.

    /// The pointer moved over the seek bar, or left it (`None`): the
    /// tooltip to show, with a preview asked for when it is on.
    pub fn seek_hover(&mut self, hover: Option<SeekHover>) -> Option<SeekTip> {
        let Some(hover) = hover else {
            self.cancel_preview();
            return None;
        };
        let playback = &self.settings.playback;
        if !self.model.media_loaded() || !(playback.seek_tooltip || playback.seek_preview) {
            return None;
        }
        let duration = self
            .model
            .double(PropertyId::Duration)
            .filter(|duration| *duration > 0.0)?;
        let time = (hover.fraction.clamp(0.0, 1.0) * duration * 10.0).round() / 10.0;
        let mut lines = Vec::new();
        if playback.seek_tooltip || playback.preview_time {
            lines.push(format!(
                "{} / {}",
                crate::view::format_time(time),
                crate::view::format_time(duration)
            ));
        }
        if playback.preview_title
            && let Some(title) = self.mark_title(time)
        {
            lines.push(title);
        }
        // Local files only: a network source would load in the thumbnail
        // core too, and could hold it up.
        let local = self
            .model
            .loaded_path()
            .is_some_and(|path| !crate::playlist::is_url(path));
        let preview = (playback.seek_preview && local && self.has_video())
            .then(|| {
                let path = self.model.loaded_path()?.to_owned();
                self.request_preview(&path, time).then_some(PREVIEW_BOX)
            })
            .flatten();
        Some(SeekTip {
            text: lines.join("\n"),
            x: hover.x,
            top: hover.top,
            time,
            preview,
        })
    }

    /// The file has video (not only cover art).
    fn has_video(&self) -> bool {
        self.model.int(PropertyId::VideoWidth).is_some()
    }

    /// The chapter or bookmark a time lies in.
    fn mark_title(&self, time: f64) -> Option<String> {
        let bookmark = self.model.loaded_path().and_then(|path| {
            self.marks()
                .file(path)
                .bookmarks
                .into_iter()
                .rev()
                .find(|bookmark| bookmark.time <= time + 0.05 && time - bookmark.time < 2.0)
                .map(|bookmark| bookmark.title)
        });
        bookmark.or_else(|| {
            let chapters = &self.play.chapters;
            let index = chapters.iter().rposition(|chapter| chapter.time <= time)?;
            Some(match &chapters[index].title {
                Some(title) if !title.is_empty() => title.clone(),
                _ => format!("Chapter {}", index + 1),
            })
        })
    }

    /// The marks the seek bar shows: chapters, bookmarks and the A-B
    /// points, as fractions of the duration.
    #[must_use]
    pub fn seek_marks(&self) -> Vec<rvp_skin::Mark> {
        use rvp_skin::{Mark, MarkKind};
        if !self.settings.playback.seek_markers || !self.model.media_loaded() {
            return Vec::new();
        }
        let Some(duration) = self
            .model
            .double(PropertyId::Duration)
            .filter(|duration| *duration > 0.0)
        else {
            return Vec::new();
        };
        // A fraction of the bar; f32 is far finer than a pixel.
        #[allow(clippy::cast_possible_truncation)]
        let at = |time: f64| (time / duration).clamp(0.0, 1.0) as f32;
        let mut marks: Vec<Mark> = self
            .play
            .chapters
            .iter()
            .filter(|chapter| chapter.time > 0.0)
            .map(|chapter| Mark {
                at: at(chapter.time),
                kind: MarkKind::Chapter,
            })
            .collect();
        if let Some(path) = self.model.loaded_path() {
            marks.extend(
                self.marks()
                    .file(path)
                    .bookmarks
                    .iter()
                    .map(|bookmark| Mark {
                        at: at(bookmark.time),
                        kind: MarkKind::Bookmark,
                    }),
            );
        }
        for (point, kind) in [
            (self.session.point_a, MarkKind::PointA),
            (self.session.point_b, MarkKind::PointB),
        ] {
            if let Some(time) = point {
                marks.push(Mark { at: at(time), kind });
            }
        }
        marks
    }

    /// Re-reads the file's chapters (when it loads or their number
    /// changes).
    pub(crate) fn read_chapters(&mut self, engine: &Engine) {
        self.play.chapters = if self.model.media_loaded() {
            engine.chapters()
        } else {
            Vec::new()
        };
    }

    // Jumps.

    /// The previous or next keyframe: a relative keyframe seek of half a
    /// frame (mpv seeks forward to the next keyframe, or back to the
    /// keyframe before).
    pub(crate) fn keyframe_step(
        &mut self,
        engine: &Engine,
        forward: bool,
    ) -> Result<(), EngineError> {
        if !self.model.media_loaded() {
            return Ok(());
        }
        let fps = self
            .model
            .double(PropertyId::ContainerFps)
            .filter(|fps| *fps > 1.0)
            .unwrap_or(25.0);
        let delta = 0.5 / fps;
        let delta = if forward { delta } else { -delta };
        engine.command("seek", &[&format!("{delta:.4}"), "relative+keyframes"])
    }

    /// The Jump to Time/Frame dialog's answer: a time (`1:23:45.5`, `90`,
    /// `+30`) or with `frames` a frame number.
    ///
    /// # Errors
    /// The message to show when the text is not a time or frame.
    pub fn jump_to_text(
        &mut self,
        engine: &Engine,
        text: &str,
        frames: bool,
    ) -> Result<(), String> {
        if !self.model.media_loaded() {
            return Err("Nothing plays".to_owned());
        }
        let target = if frames {
            let frame: f64 = text
                .trim()
                .parse()
                .ok()
                .filter(|frame: &f64| frame.is_finite() && *frame >= 0.0)
                .ok_or_else(|| format!("{text:?} is not a frame number"))?;
            let fps = self
                .model
                .double(PropertyId::ContainerFps)
                .filter(|fps| *fps > 0.0)
                .ok_or_else(|| "The video's frame rate is not known".to_owned())?;
            Seek::Absolute(frame.floor() / fps)
        } else {
            let (seconds, relative) =
                parse_time(text).ok_or_else(|| format!("{text:?} is not a time"))?;
            match relative {
                Some(sign) => Seek::Relative(sign * seconds),
                None => Seek::Absolute(seconds),
            }
        };
        let duration = self.model.double(PropertyId::Duration);
        let target = match (target, duration) {
            (Seek::Absolute(seconds), Some(duration)) => Seek::Absolute(seconds.min(duration)),
            (other, _) => other,
        };
        self.seek_to(engine, target)
            .map_err(|error| error.to_string())
    }

    // The A-B repeat.

    /// M11's A-B commands ([`crate::commands`] has setting and clearing).
    pub(crate) fn ab_extra(&mut self, engine: &Engine, change: AbRepeat) -> Option<UiRequest> {
        let step = self.settings.playback.ab.step;
        let time = self.model.double(PropertyId::TimePos);
        let nudge = |point: Option<f64>, forward: bool| {
            point.map(|point| (point + if forward { step } else { -step }).max(0.0))
        };
        match change {
            AbRepeat::MoveA(forward) => {
                self.session.point_a = nudge(self.session.point_a, forward);
            }
            AbRepeat::MoveB(forward) => {
                self.session.point_b = nudge(self.session.point_b, forward);
            }
            AbRepeat::MoveBoth(forward) => {
                self.session.point_a = nudge(self.session.point_a, forward);
                self.session.point_b = nudge(self.session.point_b, forward);
            }
            AbRepeat::CurrentSubtitle => {
                return match self.subtitle_line() {
                    Some((start, end)) => Some(self.repeat_section(engine, start, end, true)),
                    None => self.ab_extra(engine, AbRepeat::Behind),
                };
            }
            AbRepeat::PreviousSubtitle | AbRepeat::NextSubtitle => {
                let skip = if change == AbRepeat::NextSubtitle {
                    "1"
                } else {
                    "-1"
                };
                if self.model.media_loaded() && engine.command("sub-seek", &[skip]).is_ok() {
                    self.play.subtitle_section = true;
                    self.play.line_before = self.model.double(PropertyId::SubStart);
                }
                return None;
            }
            AbRepeat::CurrentSection | AbRepeat::PreviousSection | AbRepeat::NextSection => {
                return self.marked_section(engine, change, time?);
            }
            AbRepeat::Behind => {
                let time = time?;
                let start = (time - self.settings.playback.ab.behind).max(0.0);
                return Some(self.repeat_section(engine, start, time, false));
            }
            AbRepeat::FewerLoops | AbRepeat::MoreLoops => {
                let ab = &mut self.settings.playback.ab;
                ab.loops = if change == AbRepeat::MoreLoops {
                    (ab.loops + 1).min(1000)
                } else {
                    ab.loops.saturating_sub(1)
                };
                self.save_settings();
                self.apply_loop_count(engine);
                let loops = self.settings.playback.ab.loops;
                return Some(UiRequest::Osd(if loops == 0 {
                    "A-B repeat: endless".to_owned()
                } else {
                    format!("A-B repeat: {loops} times")
                }));
            }
            AbRepeat::PreventBackward => {
                let ab = &mut self.settings.playback.ab;
                ab.prevent_backward = !ab.prevent_backward;
                let on = ab.prevent_backward;
                return Some(self.toggled("Prevent backward navigation from point A", on));
            }
            AbRepeat::ToggleCurrent => {
                let ab = &mut self.settings.playback.ab;
                ab.toggle_current = !ab.toggle_current;
                let on = ab.toggle_current;
                return Some(self.toggled("Toggle for repeat current section", on));
            }
            AbRepeat::RememberList => {
                let ab = &mut self.settings.playback.ab;
                ab.remember = !ab.remember;
                let on = ab.remember;
                return Some(self.toggled("Remember A-B Repeat List", on));
            }
            AbRepeat::ListSetup => return Some(UiRequest::SectionList),
            AbRepeat::Settings => return Some(UiRequest::Preferences(Some("section-repeat"))),
            AbRepeat::SetA
            | AbRepeat::SetB
            | AbRepeat::ClearA
            | AbRepeat::ClearB
            | AbRepeat::Toggle
            | AbRepeat::Clear => return None,
        }
        self.apply_ab(engine);
        Some(self.ab_notice())
    }

    /// The A-B points as a notice.
    pub(crate) fn ab_notice(&self) -> UiRequest {
        let session = &self.session;
        let point = |p: Option<f64>| p.map_or_else(|| "-".to_owned(), clock);
        UiRequest::Osd(format!(
            "A-B Repeat{}: A {} B {}",
            if session.ab_active && session.point_a.is_some() && session.point_b.is_some() {
                ""
            } else {
                " (off)"
            },
            point(session.point_a),
            point(session.point_b)
        ))
    }

    /// The subtitle line shown now: its start and end.
    fn subtitle_line(&self) -> Option<(f64, f64)> {
        if !self.model.media_loaded() {
            return None;
        }
        let start = self.model.double(PropertyId::SubStart)?;
        let end = self.model.double(PropertyId::SubEnd)?;
        (end > start).then_some((start, end))
    }

    /// Repeats a section from `start` to `end`; a repeat of the same
    /// section turns it off when "toggle" is on (`current` commands only).
    fn repeat_section(
        &mut self,
        engine: &Engine,
        start: f64,
        end: f64,
        current: bool,
    ) -> UiRequest {
        let same = |point: Option<f64>, value: f64| point.is_some_and(|p| (p - value).abs() < 0.05);
        let session = &mut self.session;
        if current
            && self.settings.playback.ab.toggle_current
            && session.ab_active
            && same(session.point_a, start)
            && same(session.point_b, end)
        {
            session.ab_active = false;
        } else {
            session.point_a = Some(start);
            session.point_b = Some(end);
            session.ab_active = true;
        }
        self.apply_ab(engine);
        if self.session.ab_active
            && self
                .model
                .double(PropertyId::TimePos)
                .is_some_and(|time| time < start || time > end)
        {
            drop(engine.seek(Seek::Absolute(start)));
        }
        self.ab_notice()
    }

    /// The A-B list's section at the current time, or the one before or
    /// after it.
    fn marked_section(
        &mut self,
        engine: &Engine,
        change: AbRepeat,
        time: f64,
    ) -> Option<UiRequest> {
        let path = self.model.loaded_path()?.to_owned();
        let sections = self.marks().file(&path).sections;
        if sections.is_empty() {
            return Some(UiRequest::Osd("The A-B repeat list is empty".to_owned()));
        }
        let at = sections
            .iter()
            .rposition(|section| section.start <= time + 0.05)
            .unwrap_or(0);
        let index = match change {
            AbRepeat::PreviousSection => at.checked_sub(1)?,
            AbRepeat::NextSection if sections[at].start <= time + 0.05 => at + 1,
            _ => at,
        };
        let section = sections.get(index)?;
        let current = change == AbRepeat::CurrentSection;
        let (start, end) = (section.start, section.end);
        if !current {
            drop(engine.seek(Seek::Absolute(start)));
        }
        Some(self.repeat_section(engine, start, end, current))
    }

    /// The A-B list dialog: `add` the A-B points as a section (titled
    /// `text`), `repeat`, `rename` or `delete` a section.
    pub fn section_edit(
        &mut self,
        engine: &Engine,
        op: &str,
        index: Option<usize>,
        text: &str,
    ) -> Option<UiRequest> {
        let path = self.model.loaded_path()?.to_owned();
        match op {
            "add" => {
                let (Some(a), Some(b)) = (self.session.point_a, self.session.point_b) else {
                    return Some(UiRequest::Osd("Set points A and B first".to_owned()));
                };
                let title = if text.trim().is_empty() {
                    format!("{} - {}", clock(a.min(b)), clock(a.max(b)))
                } else {
                    text.trim().to_owned()
                };
                self.marks_mut().add_section(&path, a, b, &title);
                Some(UiRequest::Osd(format!("Section added: {title}")))
            }
            "repeat" => {
                let section = self.marks().file(&path).sections.get(index?)?.clone();
                drop(engine.seek(Seek::Absolute(section.start)));
                Some(self.repeat_section(engine, section.start, section.end, false))
            }
            "rename" => {
                self.marks_mut().rename_section(&path, index?, text.trim());
                None
            }
            "delete" => {
                self.marks_mut().remove_section(&path, index?);
                None
            }
            _ => None,
        }
    }

    /// The subtitle line before or after came up after its seek: its
    /// section repeats (the line's times change after playback restarts).
    pub(crate) fn subtitle_section_arrived(&mut self, engine: &Engine) -> Option<UiRequest> {
        if !self.play.subtitle_section {
            return None;
        }
        let (start, end) = self.subtitle_line()?;
        if Some(start) == self.play.line_before {
            return None;
        }
        self.play.subtitle_section = false;
        Some(self.repeat_section(engine, start, end, false))
    }

    /// mpv's `ab-loop-count` from the setting.
    pub(crate) fn apply_loop_count(&self, engine: &Engine) {
        let loops = self.settings.playback.ab.loops;
        let value = if loops == 0 {
            "inf".to_owned()
        } else {
            loops.to_string()
        };
        // Older libmpv builds lack the option; the repeat stays endless.
        drop(engine.set_text("ab-loop-count", &value));
    }

    /// A seek target moved to point A when "Prevent backward navigation
    /// from point A" holds the section.
    pub(crate) fn keep_after_a(&self, target: Seek) -> Seek {
        let session = &self.session;
        let (Some(a), true, true) = (
            session.point_a,
            session.ab_active && session.point_b.is_some(),
            self.settings.playback.ab.prevent_backward,
        ) else {
            return target;
        };
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        let duration = self.model.double(PropertyId::Duration).unwrap_or(0.0);
        let absolute = match target {
            Seek::Absolute(seconds) => seconds,
            Seek::Relative(seconds) => time + seconds,
            Seek::Fraction(fraction) => fraction * duration,
        };
        if absolute < a {
            Seek::Absolute(a)
        } else {
            target
        }
    }

    // Skips.

    pub(crate) fn skip_command(&mut self, skip: Skip) -> UiRequest {
        let settings = &mut self.settings.playback.skip;
        let (text, on) = match skip {
            Skip::Toggle => {
                settings.enabled = !settings.enabled;
                ("Skip", settings.enabled)
            }
            Skip::Intro => {
                settings.intro = !settings.intro;
                settings.enabled |= settings.intro;
                ("Skip intro", settings.intro)
            }
            Skip::Ending => {
                settings.ending = !settings.ending;
                settings.enabled |= settings.ending;
                ("Skip ending", settings.ending)
            }
            Skip::Chapters => {
                settings.chapters = !settings.chapters;
                settings.enabled |= settings.chapters;
                ("Skip chapters", settings.chapters)
            }
            Skip::Setup => return UiRequest::Preferences(Some("skip")),
        };
        self.toggled(text, on)
    }

    /// Saves a toggled setting and tells it.
    pub(crate) fn toggled(&self, text: &str, on: bool) -> UiRequest {
        self.save_settings();
        UiRequest::Osd(format!("{text}: {}", if on { "On" } else { "Off" }))
    }

    /// The menu text of Skip's items with their lengths.
    pub(crate) fn skip_label(&self, skip: Skip, text: &str) -> String {
        let settings = &self.settings.playback.skip;
        match skip {
            Skip::Intro => text.replace("%s", &duration_label(settings.intro_seconds)),
            Skip::Ending => text.replace("%s", &duration_label(settings.ending_seconds)),
            _ => text.to_owned(),
        }
    }

    /// Where a file that just opened starts when the intro is skipped:
    /// past the intro, unless the resume position lies beyond it or the
    /// file is no longer than the intro (or its length is not known).
    pub(crate) fn intro_skip(&self, resume: Option<f64>, duration: Option<f64>) -> Option<f64> {
        let skip = &self.settings.playback.skip;
        if !(skip.enabled && skip.intro) {
            return None;
        }
        let intro = skip.intro_seconds;
        let fits = duration.is_some_and(|duration| duration > intro + 1.0);
        (fits && resume.is_none_or(|resume| resume < intro)).then_some(intro)
    }

    /// The ending is reached when "Skip Ending" is on: the album moves on.
    pub(crate) fn ending_reached(&mut self) -> bool {
        let skip = &self.settings.playback.skip;
        if !(skip.enabled && skip.ending) || self.play.ending_skipped {
            return false;
        }
        let (Some(time), Some(duration)) = (
            self.model.double(PropertyId::TimePos),
            self.model.double(PropertyId::Duration),
        ) else {
            return false;
        };
        let reached =
            duration > skip.ending_seconds + 1.0 && duration - time <= skip.ending_seconds;
        if reached {
            self.play.ending_skipped = true;
        }
        reached
    }

    /// The chapter that plays is one to skip: on to the next chapter, or
    /// `true` (the end of the file) after the last.
    pub(crate) fn chapter_skip(&mut self, engine: &Engine) -> bool {
        let skip = &self.settings.playback.skip;
        if !(skip.enabled && skip.chapters) {
            return false;
        }
        let Some(chapter) = self.model.int(PropertyId::Chapter) else {
            return false;
        };
        let Ok(index) = usize::try_from(chapter) else {
            return false;
        };
        if self.play.skipped_chapter == Some(chapter) {
            return false;
        }
        let Some(current) = self.play.chapters.get(index) else {
            return false;
        };
        if !chapter_listed(&skip.chapter_names, current.title.as_deref(), index + 1) {
            return false;
        }
        self.play.skipped_chapter = Some(chapter);
        if index + 1 < self.play.chapters.len() {
            drop(engine.set_text("chapter", &(index + 1).to_string()));
            false
        } else {
            true
        }
    }

    // Shuffle.

    /// The next entry in shuffle order: a random playable entry of the
    /// album not played in this round; `None` when every one played.
    pub(crate) fn shuffle_next(&mut self) -> Option<(usize, usize)> {
        let album_index = self.library().lists().playing?;
        let album = self.library().lists().albums.get(album_index)?;
        let current = album
            .current
            .and_then(|index| album.entries.get(index))
            .map(|entry| entry.path.clone());
        let candidates: Vec<(usize, String)> = album
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.playable)
            .map(|(index, entry)| (index, entry.path.clone()))
            .collect();
        if let Some(current) = current
            && self.play.shuffle_round.last() != Some(&current)
        {
            self.play.shuffle_round.push(current);
        }
        let played: BTreeSet<&str> = self.play.shuffle_round.iter().map(String::as_str).collect();
        let left: Vec<usize> = candidates
            .iter()
            .filter(|(_, path)| !played.contains(path.as_str()))
            .map(|(index, _)| *index)
            .collect();
        if left.is_empty() {
            return None;
        }
        if self.play.shuffle_state == 0 {
            self.play.shuffle_state = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |elapsed| {
                    u64::try_from(elapsed.as_nanos() & u128::from(u64::MAX)).unwrap_or(1) | 1
                });
        }
        let bound = u64::try_from(left.len()).unwrap_or(u64::MAX);
        let pick = crate::playlist::next_random(&mut self.play.shuffle_state) % bound;
        let index = left[usize::try_from(pick).unwrap_or(0)];
        Some((album_index, index))
    }

    /// Previous in shuffle order: the entry played before in this round.
    pub(crate) fn shuffle_previous(&mut self) -> Option<(usize, usize)> {
        let album_index = self.library().lists().playing?;
        let album = self.library().lists().albums.get(album_index)?;
        let current = album
            .current
            .and_then(|index| album.entries.get(index))
            .map(|entry| entry.path.clone());
        let paths: Vec<String> = album
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect();
        if self.play.shuffle_round.last() == current.as_ref() {
            self.play.shuffle_round.pop();
        }
        let previous = self.play.shuffle_round.pop()?;
        let index = paths.iter().position(|path| *path == previous)?;
        Some((album_index, index))
    }

    /// Starts a new shuffle round (shuffle switched, another album).
    pub(crate) fn reset_shuffle(&mut self) {
        self.play.shuffle_round.clear();
    }

    // Bookmarks.

    pub(crate) fn bookmark_command(
        &mut self,
        engine: &Engine,
        command: BookmarkCommand,
    ) -> Option<UiRequest> {
        let _ = engine;
        let settings = &mut self.settings.playback.bookmarks;
        let (text, on) = match command {
            BookmarkCommand::Add => return self.add_bookmark_here(),
            BookmarkCommand::Edit => return Some(UiRequest::Bookmarks),
            BookmarkCommand::Thumbnails => {
                settings.thumbnails = !settings.thumbnails;
                ("View bookmarks as thumbnails", settings.thumbnails)
            }
            BookmarkCommand::AutoDelete => {
                settings.auto_delete = !settings.auto_delete;
                ("Auto delete the selected bookmarks", settings.auto_delete)
            }
            BookmarkCommand::Prompt => {
                settings.prompt = !settings.prompt;
                ("Prompt before adding a bookmark", settings.prompt)
            }
            BookmarkCommand::ShowInMenu => {
                settings.in_menu = !settings.in_menu;
                ("Show bookmark list in the menu", settings.in_menu)
            }
        };
        Some(self.toggled(text, on))
    }

    /// Adds a bookmark at the current time, or asks for its title first.
    fn add_bookmark_here(&mut self) -> Option<UiRequest> {
        let path = self.model.loaded_path()?.to_owned();
        let time = self.model.double(PropertyId::TimePos)?;
        let title = format!(
            "Bookmark {} ({})",
            self.marks().file(&path).bookmarks.len() + 1,
            crate::view::format_time(time)
        );
        if self.settings.playback.bookmarks.prompt {
            return Some(UiRequest::BookmarkTitle { time, title });
        }
        Some(self.add_bookmark(time, &title))
    }

    /// Adds a bookmark to the file that plays (the title dialog's answer).
    pub fn add_bookmark(&mut self, time: f64, title: &str) -> UiRequest {
        let Some(path) = self.model.loaded_path().map(str::to_owned) else {
            return UiRequest::Osd("Nothing plays".to_owned());
        };
        let title = if title.trim().is_empty() {
            crate::view::format_time(time)
        } else {
            title.trim().to_owned()
        };
        self.marks_mut().add_bookmark(&path, time, &title);
        UiRequest::Osd(format!("Bookmark added: {title}"))
    }

    /// Jumps to a bookmark of the file that plays (the menu, the scene
    /// browser), deleting it after when "Auto delete" is on.
    pub fn jump_to_bookmark(&mut self, engine: &Engine, index: usize) -> Option<UiRequest> {
        let path = self.model.loaded_path()?.to_owned();
        let bookmark = self.marks().file(&path).bookmarks.get(index)?.clone();
        drop(self.seek_to(engine, Seek::Absolute(bookmark.time)));
        if self.settings.playback.bookmarks.auto_delete {
            self.marks_mut().remove_bookmark(&path, index);
        }
        Some(UiRequest::Osd(bookmark.title))
    }

    // When playback finishes.

    /// "Perform the action" chooses this action.
    pub(crate) fn set_power_action(&mut self, action: PowerAction) -> UiRequest {
        self.settings.playback.finished.action = action;
        self.save_settings();
        UiRequest::Osd(format!(
            "When playback finishes: {}",
            crate::power::action_text(action)
        ))
    }

    /// Keeps the "When Playback Finished" choice when it is to be
    /// remembered.
    pub(crate) fn remember_after(&mut self) {
        let finished = &mut self.settings.playback.finished;
        if finished.remember && finished.after != self.session.after {
            finished.after = self.session.after;
            self.save_settings();
        }
    }

    /// M11's part of [`App::apply_settings`].
    pub(crate) fn apply_playback_settings(&mut self, engine: &Engine, old: &crate::Settings) {
        let (new, old) = (&self.settings.playback, &old.playback);
        if new.external_audio != old.external_audio {
            let value = if new.external_audio { "fuzzy" } else { "no" };
            drop(engine.set_text("audio-file-auto", value));
        }
        if new.auto_rotate != old.auto_rotate && self.session.rotation == 0 {
            let value = if new.auto_rotate { "0" } else { "no" };
            drop(engine.set_text("video-rotate", value));
        }
        if new.audio_folders != old.audio_folders {
            drop(engine.set_text("audio-file-paths", &path_list(&new.audio_folders)));
        }
        if new.ab.loops != old.ab.loops {
            self.apply_loop_count(engine);
        }
        if new.shuffle != old.shuffle {
            self.reset_shuffle();
        }
    }

    /// The toggles of the Playback menu (M11): saved and told.
    pub(crate) fn playback_toggle(&mut self, action: crate::action::Action) -> Option<UiRequest> {
        use crate::action::Action;
        let playback = &mut self.settings.playback;
        let (text, on) = match action {
            Action::ToggleShuffle => {
                playback.shuffle = !playback.shuffle;
                let on = playback.shuffle;
                self.reset_shuffle();
                ("Shuffle", on)
            }
            Action::ToggleRememberPositions => {
                self.settings.remember_positions = !self.settings.remember_positions;
                (
                    "Remember playback positions",
                    self.settings.remember_positions,
                )
            }
            Action::ToggleHidePointer => {
                playback.hide_pointer = !playback.hide_pointer;
                ("Hide mouse cursor during playback", playback.hide_pointer)
            }
            Action::ToggleCloseAtEnd => {
                playback.close_at_end = !playback.close_at_end;
                ("Close playback when playlist ends", playback.close_at_end)
            }
            Action::ToggleRecyclePlayed => {
                playback.recycle_played = !playback.recycle_played;
                (
                    "Send played files to the Recycle Bin",
                    playback.recycle_played,
                )
            }
            Action::ToggleSeekTooltip => {
                playback.seek_tooltip = !playback.seek_tooltip;
                ("Time info on the seek bar's tooltip", playback.seek_tooltip)
            }
            Action::ToggleSeekPreview => {
                playback.seek_preview = !playback.seek_preview;
                ("Thumbnail previews on the seek bar", playback.seek_preview)
            }
            Action::ToggleSeekMarkers => {
                playback.seek_markers = !playback.seek_markers;
                ("Markers on the seek bar", playback.seek_markers)
            }
            Action::ToggleKeepSpeed => {
                playback.keep_speed = !playback.keep_speed;
                ("Keep playback speed changes", playback.keep_speed)
            }
            Action::ToggleRememberAfter => {
                let finished = &mut playback.finished;
                finished.remember = !finished.remember;
                let on = finished.remember;
                if on {
                    finished.after = self.session.after;
                }
                ("Remember playback finished action", on)
            }
            _ => return None,
        };
        Some(self.toggled(text, on))
    }

    /// Check marks of M11's playback items.
    pub(crate) fn playback_checked(&self, action: crate::action::Action) -> Option<bool> {
        use crate::action::Action;
        let playback = &self.settings.playback;
        Some(match action {
            Action::ToggleShuffle => playback.shuffle,
            Action::ToggleRememberPositions => self.settings.remember_positions,
            Action::ToggleHidePointer => playback.hide_pointer,
            Action::ToggleCloseAtEnd => playback.close_at_end,
            Action::ToggleRecyclePlayed => playback.recycle_played,
            Action::ToggleSeekTooltip => playback.seek_tooltip,
            Action::ToggleSeekPreview => playback.seek_preview,
            Action::ToggleSeekMarkers => playback.seek_markers,
            Action::ToggleKeepSpeed => playback.keep_speed,
            Action::ToggleRememberAfter => playback.finished.remember,
            Action::SetPowerAction(power) => playback.finished.action == power,
            Action::Skip(Skip::Toggle) => playback.skip.enabled,
            Action::Skip(Skip::Intro) => playback.skip.intro,
            Action::Skip(Skip::Ending) => playback.skip.ending,
            Action::Skip(Skip::Chapters) => playback.skip.chapters,
            Action::AbRepeat(AbRepeat::PreventBackward) => playback.ab.prevent_backward,
            Action::AbRepeat(AbRepeat::ToggleCurrent) => playback.ab.toggle_current,
            Action::AbRepeat(AbRepeat::RememberList) => playback.ab.remember,
            Action::Bookmark(BookmarkCommand::Thumbnails) => playback.bookmarks.thumbnails,
            Action::Bookmark(BookmarkCommand::AutoDelete) => playback.bookmarks.auto_delete,
            Action::Bookmark(BookmarkCommand::Prompt) => playback.bookmarks.prompt,
            Action::Bookmark(BookmarkCommand::ShowInMenu) => playback.bookmarks.in_menu,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_parse_as_the_jump_dialog_takes_them() {
        assert_eq!(parse_time("1:23:45.5"), Some((5025.5, None)));
        assert_eq!(parse_time("23:45"), Some((1425.0, None)));
        assert_eq!(parse_time(" 90 "), Some((90.0, None)));
        assert_eq!(parse_time("+30"), Some((30.0, Some(1.0))));
        assert_eq!(parse_time("-1:00"), Some((60.0, Some(-1.0))));
        for bad in ["", ":", "1:2:3:4", "1:75", "x", "1.5:00", "-", "1::2"] {
            assert_eq!(parse_time(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn chapters_to_skip_match_by_title_or_number() {
        let names = "Opening; 3 ;credits";
        assert!(chapter_listed(names, Some("Opening Song"), 1));
        assert!(chapter_listed(names, Some("End Credits"), 7));
        assert!(chapter_listed(names, None, 3));
        assert!(!chapter_listed(names, Some("Part A"), 2));
        assert!(!chapter_listed("", Some("Opening"), 1));
    }

    #[test]
    fn skip_lengths_read_as_in_the_menu() {
        assert_eq!(duration_label(30.0), "30 Sec");
        assert_eq!(duration_label(60.0), "1 Min");
        assert_eq!(duration_label(90.0), "1 Min 30 Sec");
    }

    #[test]
    fn the_defaults_are_valid() {
        let mut notes = Vec::new();
        Playback::default().validate(&mut notes);
        assert!(notes.is_empty(), "{notes:?}");
        let mut wrong = Playback {
            speed_step: 0.0,
            ..Playback::default()
        };
        wrong.ab.loops = 5000;
        wrong.finished.countdown = 1;
        wrong.validate(&mut notes);
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert_eq!(wrong.ab.loops, 1000);
        assert_eq!(wrong.finished.countdown, 5);
    }
}
