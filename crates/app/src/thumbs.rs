//! The thumbnail core in the app (M11): RVP's requests to
//! [`rvp_engine::thumbnail`] and what comes back.
//!
//! The shell starts the core after the player's ([`App::start_thumbnails`];
//! the player's core must come first, see the engine's module) and calls
//! [`App::thumbnail_events`] when the core's `notify` says outcomes wait.
//! Durations found in the background go straight into the albums ("Detect
//! file durations"); pictures come back as [`ThumbEvent`]s for the window
//! to show. Requests of a group the window no longer shows are cancelled.
//!
//! A seek's picture (`Playback::seek_picture`, the owner, 2026-11-10):
//! while a smoothing engine's filter runs, mpv loads its script again at
//! every seek, and the old frame stayed until the engine's first frames
//! came (about a second with RIFE (TensorRT), more from far keyframes).
//! The core renders the target's frame meanwhile, and the window shows it
//! over the video, at the video's place (`osd-dimensions`), until mpv's
//! playback restarts. Only where that frame is what mpv will show: a local
//! file whose filters change neither the picture's shape nor its content
//! (the deinterlacer, the forced matrix and the smoothing itself), turned
//! by nothing, with the picture's shape on the screen.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rvp_engine::thumbnail::{
    FrameRequest, Group, Outcome, Picture, ThumbnailOptions, Thumbnailer, Ticket,
};
use rvp_engine::{EngineEvent, PropertyId, Seek};

use crate::app::{App, UiRequest};

/// A seek's picture is as large as the video on the screen, up to this.
const SEEK_PICTURE_BOX: u32 = 1920;
/// A seek's picture goes after this long without the restart (mpv stuck
/// at the end of a file).
const SEEK_PICTURE_TIMEOUT: Duration = Duration::from_secs(8);

/// A picture for the window.
#[derive(Debug, Clone, PartialEq)]
pub enum ThumbEvent {
    /// The seek bar's preview, for the newest time asked.
    Preview(Arc<Picture>),
    /// A picture of the scene browser that shows, by its index.
    Scene { index: usize, picture: Arc<Picture> },
    /// A playlist entry's thumbnail, by its path.
    Entry { path: String, picture: Arc<Picture> },
    /// A seek's target over the video, at `place`: its left, top, width and
    /// height in fractions of the video area (beyond 0 to 1 under a zoom).
    SeekPicture {
        picture: Arc<Picture>,
        place: [f64; 4],
    },
    /// The core cannot start: there are no pictures.
    Unavailable(String),
    /// A contact sheet with all its pictures, to be written (M12).
    Sheet(crate::capture::SheetImage),
}

/// The requests under way.
#[derive(Default)]
pub(crate) struct Thumbs {
    core: Option<Thumbnailer>,
    /// The newest preview asked for and its time.
    preview: Option<(Ticket, f64)>,
    /// The seek whose picture is asked for or shows, until mpv's playback
    /// restarts.
    seek: Option<SeekAsk>,
    scenes: BTreeMap<Ticket, usize>,
    entries: BTreeMap<Ticket, String>,
    /// Files whose duration was asked for, so it is not asked again.
    durations: BTreeSet<String>,
    /// Entries whose playlist thumbnail is in the window's atlas cache, and
    /// those that have none (no video): neither is asked for again.
    ready: BTreeSet<String>,
    without: BTreeSet<String>,
    /// Why the core is not there.
    unavailable: Option<String>,
}

impl std::fmt::Debug for Thumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thumbs")
            .field("running", &self.core.is_some())
            .field("preview", &self.preview)
            .field("scenes", &self.scenes.len())
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

/// mpv's `vf` changes neither the picture's shape nor its content: only
/// the smoothing's filters, the deinterlacer and the forced matrix.
fn plain_filters(vf: &str) -> bool {
    crate::audio::chain::split(vf)
        .into_iter()
        .all(|(label, _)| {
            label.is_some_and(|label| {
                label == crate::smoothing::LABEL
                    || crate::smoothing::FRUC_LABELS.contains(&label)
                    || label == crate::render::MATRIX_LABEL
                    || label == crate::render::DEINTERLACE_LABEL
            })
        })
}

/// A seek's picture under way ([`Thumbs::seek`]).
#[derive(Debug, Clone, Copy)]
struct SeekAsk {
    ticket: Ticket,
    /// The target in seconds: a relative seek before the restart counts
    /// from here, as mpv's does.
    time: f64,
    asked: Instant,
    /// The window shows the picture.
    shown: bool,
}

impl Thumbs {
    /// The core, when it runs.
    pub(crate) fn core(&self) -> Option<&Thumbnailer> {
        self.core.as_ref()
    }
}

impl App {
    /// Starts the thumbnail core; `notify` runs on the core's thread when
    /// outcomes wait and must only schedule [`App::thumbnail_events`].
    ///
    /// # Errors
    /// Fails when the core's thread cannot start.
    pub fn start_thumbnails(
        &mut self,
        options: ThumbnailOptions,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Result<(), String> {
        if self.thumbs.core.is_some() {
            return Ok(());
        }
        let core = Thumbnailer::start(options, notify).map_err(|error| error.to_string())?;
        self.thumbs.core = Some(core);
        Ok(())
    }

    /// How many playlist thumbnails the window has (the self-test).
    #[must_use]
    pub fn list_thumbnails_ready(&self) -> usize {
        self.thumbs.ready.len()
    }

    /// The thumbnail core's counters (the self-test's metrics).
    #[must_use]
    pub fn thumbnail_stats(&self) -> Option<rvp_engine::thumbnail::Stats> {
        self.thumbs.core.as_ref().map(Thumbnailer::stats)
    }

    /// Asks for the seek bar's preview of `path` at `time`; false without
    /// the core.
    pub(crate) fn request_preview(&mut self, path: &str, time: f64) -> bool {
        let Some(core) = &self.thumbs.core else {
            return false;
        };
        if self
            .thumbs
            .preview
            .is_some_and(|(_, asked)| (asked - time).abs() < 0.05)
        {
            return true;
        }
        let ticket = core.request(FrameRequest {
            path: path.to_owned(),
            time,
            max_width: crate::playback::PREVIEW_BOX.0,
            max_height: crate::playback::PREVIEW_BOX.1,
            fill: false,
            exact: false,
            group: Group::Preview,
        });
        self.thumbs.preview = Some((ticket, time));
        true
    }

    /// The pointer left the seek bar: no preview is wanted.
    pub(crate) fn cancel_preview(&mut self) {
        if self.thumbs.preview.take().is_some()
            && let Some(core) = &self.thumbs.core
        {
            core.cancel(Group::Preview);
        }
    }

    /// After a seek to `target` while a smoothing engine's filter runs:
    /// asks the core for the target's frame (exactly, or at a keyframe
    /// for a keyframe seek), when the window can show it.
    pub(crate) fn request_seek_picture(&mut self, target: Seek, exact: bool, now: Instant) {
        let Some(core) = &self.thumbs.core else {
            return;
        };
        let Some(path) = self.seek_picture_source() else {
            return;
        };
        let Some(duration) = self.model.double(PropertyId::Duration).filter(|d| *d > 0.0) else {
            return;
        };
        let from = self
            .thumbs
            .seek
            .map(|ask| ask.time)
            .or_else(|| self.model.double(PropertyId::TimePos))
            .unwrap_or(0.0);
        let time = match target {
            Seek::Absolute(seconds) => seconds,
            Seek::Relative(seconds) => from + seconds,
            Seek::Fraction(fraction) => fraction.clamp(0.0, 1.0) * duration,
        }
        .clamp(0.0, duration);
        let Some([_, _, width, height]) = self.video_place_pixels() else {
            return;
        };
        let size = |pixels: f64| {
            // Pixels of the screen, within the box.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let pixels = pixels.round().clamp(16.0, f64::from(SEEK_PICTURE_BOX)) as u32;
            pixels
        };
        let ticket = core.request(FrameRequest {
            path,
            time,
            max_width: size(width),
            max_height: size(height),
            fill: false,
            exact,
            group: Group::Seek,
        });
        let shown = self.thumbs.seek.is_some_and(|ask| ask.shown);
        self.thumbs.seek = Some(SeekAsk {
            ticket,
            time,
            asked: now,
            shown,
        });
    }

    /// The file a seek's picture can come from: see the module's comment.
    fn seek_picture_source(&self) -> Option<String> {
        if !self.settings.playback.seek_picture
            || !self.smoothing_filter_in_force()
            || !(self.session.rotation + self.picture.auto_rotation).is_multiple_of(360)
        {
            return None;
        }
        let plain = plain_filters(self.model.text(PropertyId::VideoFilters).unwrap_or(""));
        let path = self.model.loaded_path()?;
        (plain && !crate::playlist::is_url(path)).then(|| path.to_owned())
    }

    /// The video's place in mpv's OSD pixels (`osd-dimensions`): left,
    /// top, width, height.
    fn video_place_pixels(&self) -> Option<[f64; 4]> {
        let get = |id| self.model.int(id);
        let margins = [
            get(PropertyId::OsdMarginLeft)?,
            get(PropertyId::OsdMarginTop)?,
            get(PropertyId::OsdMarginRight)?,
            get(PropertyId::OsdMarginBottom)?,
        ];
        let (width, height) = (get(PropertyId::OsdWidth)?, get(PropertyId::OsdHeight)?);
        let shown = (
            width - margins[0] - margins[2],
            height - margins[1] - margins[3],
        );
        if width <= 0 || height <= 0 || shown.0 <= 0 || shown.1 <= 0 {
            return None;
        }
        // Pixels of a window: far within f64's integers.
        #[allow(clippy::cast_precision_loss)]
        Some([
            margins[0] as f64,
            margins[1] as f64,
            shown.0 as f64,
            shown.1 as f64,
        ])
    }

    /// Where a seek's picture goes, in fractions of the video area; none
    /// when the video on the screen has another shape than the picture
    /// (an aspect ratio, a crop, a television's pixels).
    fn seek_picture_place(&self, picture: &Picture) -> Option<[f64; 4]> {
        let [left, top, width, height] = self.video_place_pixels()?;
        #[allow(clippy::cast_precision_loss)] // pixels of a window
        let (area_width, area_height) = (
            self.model.int(PropertyId::OsdWidth)? as f64,
            self.model.int(PropertyId::OsdHeight)? as f64,
        );
        if picture.width == 0 || picture.height == 0 {
            return None;
        }
        let own = f64::from(picture.width) / f64::from(picture.height);
        if ((width / height) / own - 1.0).abs() > 0.03 {
            return None;
        }
        Some([
            left / area_width,
            top / area_height,
            width / area_width,
            height / area_height,
        ])
    }

    /// After mpv's events: once its playback restarted (after the seek),
    /// the file ended, or the picture waited too long, the seek's picture
    /// goes; the window hides it if it showed.
    pub(crate) fn seek_picture_tick(
        &mut self,
        events: &[EngineEvent],
        now: Instant,
    ) -> Option<UiRequest> {
        let ask = self.thumbs.seek?;
        let over = events.iter().any(|event| {
            matches!(
                event,
                EngineEvent::PlaybackRestart | EngineEvent::EndFile { .. }
            )
        });
        if !over && now.duration_since(ask.asked) < SEEK_PICTURE_TIMEOUT {
            return None;
        }
        self.thumbs.seek = None;
        if let Some(core) = &self.thumbs.core {
            core.cancel(Group::Seek);
        }
        crate::log::write(
            crate::log::Level::Debug,
            &format!(
                "seek: {} ms to the player's picture ({})",
                now.duration_since(ask.asked).as_millis(),
                if ask.shown {
                    "the target's picture showed"
                } else {
                    "no picture of the target"
                }
            ),
        );
        ask.shown.then_some(UiRequest::HideSeekPicture)
    }

    /// Asks for a scene browser's pictures (index and time), each filling
    /// a `size` box (black bars where the shapes differ); the pictures
    /// asked before are dropped.
    pub fn request_scenes(&mut self, path: &str, times: &[(usize, f64)], size: (u32, u32)) {
        self.cancel_scenes();
        let Some(core) = &self.thumbs.core else {
            return;
        };
        for &(index, time) in times {
            let ticket = core.request(FrameRequest {
                path: path.to_owned(),
                time,
                max_width: size.0,
                max_height: size.1,
                fill: true,
                exact: false,
                group: Group::Scenes,
            });
            self.thumbs.scenes.insert(ticket, index);
        }
    }

    /// The scene browser closed.
    pub fn cancel_scenes(&mut self) {
        self.thumbs.scenes.clear();
        if let Some(core) = &self.thumbs.core {
            core.cancel(Group::Scenes);
        }
    }

    /// Asks for playlist thumbnails: each path at `position` percent of
    /// its duration (or 10 s in when that is unknown), in `size` boxes.
    pub fn request_entry_thumbnails(
        &mut self,
        paths: &[(String, Option<f64>)],
        position: f64,
        size: (u32, u32),
        fill: bool,
    ) {
        let Some(core) = &self.thumbs.core else {
            return;
        };
        for (path, duration) in paths {
            if self.thumbs.entries.values().any(|asked| asked == path) {
                continue;
            }
            let time = duration.map_or(10.0, |duration| duration * position / 100.0);
            let ticket = core.request(FrameRequest {
                path: path.clone(),
                time,
                max_width: size.0,
                max_height: size.1,
                fill,
                exact: false,
                group: Group::List,
            });
            self.thumbs.entries.insert(ticket, path.clone());
        }
    }

    /// The playlist's thumbnails are no longer shown (or are reloaded).
    pub fn cancel_entry_thumbnails(&mut self) {
        self.thumbs.entries.clear();
        self.thumbs.ready.clear();
        self.thumbs.without.clear();
        if let Some(core) = &self.thumbs.core {
            core.cancel(Group::List);
        }
    }

    /// The thumbnail view's part of the skin's data: the rows' lines, the
    /// box, and each entry's picture (once it is there) and details.
    pub fn list_view_data(&self, data: &mut rvp_skin::DataModel) {
        let settings = &self.settings.playlist;
        let lines = u32::from(settings.thumbnail_rows).max(1);
        data.playlist.row_lines = lines;
        data.playlist.wide_thumbnails = settings.wide_thumbnails;
        if lines < 2 {
            return;
        }
        let shown = self.library().shown();
        for (row, entry) in data.playlist.entries.iter_mut().zip(&shown.entries) {
            if self.thumbs.ready.contains(&entry.path) {
                row.thumbnail = Some(crate::view::thumbnail_key(&entry.path));
            }
            let folder = std::path::Path::new(&entry.path)
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .unwrap_or_default();
            row.details = Some(if crate::playlist::is_url(&entry.path) {
                entry.path.clone()
            } else {
                format!("{}  -  {folder}", crate::view::file_name(&entry.path))
            });
        }
    }

    /// Asks for the thumbnails of the shown album's rows `first` to
    /// `first + count` (the rows the panel shows) in a thumbnail view.
    pub fn request_list_thumbnails(&mut self, first: usize, count: usize) {
        let settings = &self.settings.playlist;
        let lines = u32::from(settings.thumbnail_rows);
        if lines < 2 {
            return;
        }
        let (width, height) = rvp_skin::playlist::thumbnail_size(lines, settings.wide_thumbnails);
        let size = (
            u32::try_from(width).unwrap_or(1),
            u32::try_from(height).unwrap_or(1),
        );
        let position = settings.thumbnail_position;
        let wanted: Vec<(String, Option<f64>)> = self
            .library()
            .shown()
            .entries
            .iter()
            .skip(first)
            .take(count)
            .filter(|entry| {
                entry.playable
                    && !self.thumbs.ready.contains(&entry.path)
                    && !self.thumbs.without.contains(&entry.path)
            })
            .map(|entry| (entry.path.clone(), entry.duration))
            .collect();
        self.request_entry_thumbnails(&wanted, position, size, true);
    }

    /// Queues the durations of `paths` ("Detect file durations").
    pub(crate) fn detect_durations(&mut self, paths: &[String]) {
        let Some(core) = &self.thumbs.core else {
            return;
        };
        for path in paths {
            if self.thumbs.durations.insert(path.clone()) {
                let _ = core.duration(path);
            }
        }
    }

    /// Forgets which durations were asked, so that they are asked again.
    pub(crate) fn redetect_durations(&mut self, paths: &[String]) {
        for path in paths {
            self.thumbs.durations.remove(path);
        }
        self.detect_durations(paths);
    }

    /// Takes what the thumbnail core finished: durations go into the
    /// albums, pictures come back for the window.
    pub fn thumbnail_events(&mut self) -> Vec<ThumbEvent> {
        let Some(core) = &self.thumbs.core else {
            return Vec::new();
        };
        let mut events = Vec::new();
        for outcome in core.take() {
            match outcome {
                Outcome::Picture {
                    ticket,
                    group,
                    picture,
                } => {
                    if group == Group::Seek {
                        events.extend(self.seek_picture_arrived(ticket, picture));
                    } else if group == Group::Preview {
                        // An older preview shows until the newest comes;
                        // none once the pointer left the bar.
                        if self.thumbs.preview.is_some() {
                            events.push(ThumbEvent::Preview(picture));
                        }
                    } else if group == Group::Sheet {
                        if let Some(sheet) = self.sheet_picture(ticket, Some(picture)) {
                            events.push(ThumbEvent::Sheet(sheet));
                        }
                    } else if let Some(index) = self.thumbs.scenes.remove(&ticket) {
                        events.push(ThumbEvent::Scene { index, picture });
                    } else if let Some(path) = self.thumbs.entries.remove(&ticket) {
                        self.thumbs.ready.insert(path.clone());
                        events.push(ThumbEvent::Entry { path, picture });
                    }
                }
                Outcome::Failed { ticket, group, .. } => {
                    if group == Group::Sheet
                        && let Some(sheet) = self.sheet_picture(ticket, None)
                    {
                        events.push(ThumbEvent::Sheet(sheet));
                    }
                    self.thumbs.scenes.remove(&ticket);
                    if let Some(path) = self.thumbs.entries.remove(&ticket) {
                        self.thumbs.without.insert(path);
                    }
                }
                Outcome::Duration { path, seconds } => {
                    if let Some(seconds) = seconds {
                        self.library_mut().set_duration(&path, seconds);
                    }
                }
                Outcome::Unavailable(error) => {
                    if self.thumbs.unavailable.is_none() {
                        self.thumbs.unavailable = Some(error.clone());
                        events.push(ThumbEvent::Unavailable(error));
                    }
                }
            }
        }
        events
    }

    /// A seek's picture came: it shows unless the restart came first.
    fn seek_picture_arrived(
        &mut self,
        ticket: Ticket,
        picture: Arc<Picture>,
    ) -> Option<ThumbEvent> {
        let ask = self.thumbs.seek.filter(|ask| ask.ticket == ticket)?;
        let Some(place) = self.seek_picture_place(&picture) else {
            crate::log::write(
                crate::log::Level::Debug,
                &format!(
                    "seek: the target's picture ({}x{}) does not fit the video's place {:?}",
                    picture.width,
                    picture.height,
                    self.video_place_pixels()
                ),
            );
            return None;
        };
        crate::log::write(
            crate::log::Level::Debug,
            &format!(
                "seek: the target's picture after {} ms",
                ask.asked.elapsed().as_millis()
            ),
        );
        self.thumbs.seek = Some(SeekAsk { shown: true, ..ask });
        Some(ThumbEvent::SeekPicture { picture, place })
    }
}

#[cfg(test)]
mod tests {
    use rvp_engine::Value;

    use super::*;

    fn app(name: &str) -> App {
        let dir = std::env::temp_dir().join(format!("rvp-thumbs-{name}-{}", std::process::id()));
        App::new(crate::Settings::default(), &dir).expect("app")
    }

    fn picture(width: u32, height: u32) -> Picture {
        Picture {
            path: "a.mkv".to_owned(),
            time: 10.0,
            shown: 10.0,
            width,
            height,
            rgba: Vec::new(),
        }
    }

    /// mpv's `osd-dimensions`: the OSD's size and the video's margins
    /// (left, top, right, bottom).
    fn osd(app: &mut App, size: (i64, i64), margins: [i64; 4]) {
        let values = [
            (PropertyId::OsdWidth, size.0),
            (PropertyId::OsdHeight, size.1),
            (PropertyId::OsdMarginLeft, margins[0]),
            (PropertyId::OsdMarginTop, margins[1]),
            (PropertyId::OsdMarginRight, margins[2]),
            (PropertyId::OsdMarginBottom, margins[3]),
        ];
        for (id, value) in values {
            app.model.set(id, Some(Value::Int(value)));
        }
    }

    #[test]
    fn a_seeks_picture_lies_where_the_video_is() {
        let mut app = app("place");
        assert_eq!(app.seek_picture_place(&picture(1440, 1080)), None);
        // A 4:3 video in the middle of a 16:9 OSD.
        osd(&mut app, (1920, 1080), [240, 0, 240, 0]);
        assert_eq!(
            app.seek_picture_place(&picture(1440, 1080)),
            Some([0.125, 0.0, 0.75, 1.0])
        );
        // A smaller picture of the same shape is scaled up.
        assert_eq!(
            app.seek_picture_place(&picture(960, 720)),
            Some([0.125, 0.0, 0.75, 1.0])
        );
        // Another shape on the screen (an aspect ratio, a crop): no picture.
        assert_eq!(app.seek_picture_place(&picture(1920, 1080)), None);
        // Zoomed in, the video reaches past the OSD's edges.
        osd(&mut app, (1920, 1080), [-480, -270, -480, -270]);
        assert_eq!(
            app.seek_picture_place(&picture(1920, 1080)),
            Some([-0.25, -0.25, 1.5, 1.5])
        );
    }

    #[test]
    fn a_seeks_picture_goes_at_the_restart_or_after_a_while() {
        let mut app = app("tick");
        let now = Instant::now();
        let ask = |shown| SeekAsk {
            ticket: 7,
            time: 10.0,
            asked: now,
            shown,
        };
        // Nothing asked: nothing to hide.
        assert_eq!(
            app.seek_picture_tick(&[EngineEvent::PlaybackRestart], now),
            None
        );
        app.thumbs.seek = Some(ask(true));
        assert_eq!(
            app.seek_picture_tick(&[], now + Duration::from_secs(1)),
            None
        );
        assert_eq!(
            app.seek_picture_tick(&[EngineEvent::PlaybackRestart], now),
            Some(UiRequest::HideSeekPicture)
        );
        assert!(app.thumbs.seek.is_none());
        // One that never showed goes without a request.
        app.thumbs.seek = Some(ask(false));
        assert_eq!(
            app.seek_picture_tick(&[EngineEvent::PlaybackRestart], now),
            None
        );
        assert!(app.thumbs.seek.is_none());
        // mpv stuck: the picture goes after a while.
        app.thumbs.seek = Some(ask(true));
        assert_eq!(
            app.seek_picture_tick(&[], now + SEEK_PICTURE_TIMEOUT),
            Some(UiRequest::HideSeekPicture)
        );
        // A picture that comes after the restart, or of an older seek,
        // does not show.
        osd(&mut app, (1920, 1080), [0, 0, 0, 0]);
        let late = Arc::new(picture(1920, 1080));
        assert_eq!(app.seek_picture_arrived(7, Arc::clone(&late)), None);
        app.thumbs.seek = Some(ask(false));
        assert_eq!(app.seek_picture_arrived(6, Arc::clone(&late)), None);
        assert!(matches!(
            app.seek_picture_arrived(7, late),
            Some(ThumbEvent::SeekPicture { place, .. })
                if place.iter().zip([0.0, 0.0, 1.0, 1.0]).all(|(got, want)| (got - want).abs() < 1e-9)
        ));
        assert!(app.thumbs.seek.is_some_and(|ask| ask.shown));
    }

    #[test]
    fn a_seeks_picture_needs_filters_that_leave_the_picture_alone() {
        let smooth = "@rvpsmooth:vapoursynth=file=%9%/a/b.vpy:buffered-frames=8";
        assert!(plain_filters(""));
        assert!(plain_filters(smooth));
        assert!(plain_filters(&format!(
            "@rvpmatrix:format=colormatrix=bt.709,@rvpdeint:lavfi=[bwdif=deint=interlaced],{smooth}"
        )));
        assert!(plain_filters(
            "@rvpsmoothnv12:format=fmt=nv12,@rvpsmoothvk:format=fmt=vulkan,\
             @rvpsmooth:lavfi=[fruc_vulkan=fps=60/1:perf=medium]:hwdec_interop=vulkan"
        ));
        // The picture's geometry, an effect, SVP's filter or the user's own.
        assert!(!plain_filters(&format!(
            "@rvpcrop:lavfi=[crop=iw:ih*3/4],{smooth}"
        )));
        assert!(!plain_filters(&format!("@rvphflip:lavfi=[hflip],{smooth}")));
        assert!(!plain_filters("@svp:vapoursynth=file=%9%/a/b.vpy"));
        assert!(!plain_filters(&format!("hflip,{smooth}")));
        // No smoothing filter in force: mpv's own picture comes at once.
        let mut app = app("source");
        app.model
            .set(PropertyId::Path, Some(Value::Text("/v/a.mkv".to_owned())));
        assert_eq!(app.seek_picture_source(), None);
    }
}
