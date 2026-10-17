//! The thumbnail core in the app (M11): RVP's requests to
//! [`rvp_engine::thumbnail`] and what comes back.
//!
//! The shell starts the core after the player's ([`App::start_thumbnails`];
//! the player's core must come first, see the engine's module) and calls
//! [`App::thumbnail_events`] when the core's `notify` says outcomes wait.
//! Durations found in the background go straight into the albums ("Detect
//! file durations"); pictures come back as [`ThumbEvent`]s for the window
//! to show. Requests of a group the window no longer shows are cancelled.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use rvp_engine::thumbnail::{
    FrameRequest, Group, Outcome, Picture, ThumbnailOptions, Thumbnailer, Ticket,
};

use crate::app::App;

/// A picture for the window.
#[derive(Debug, Clone, PartialEq)]
pub enum ThumbEvent {
    /// The seek bar's preview, for the newest time asked.
    Preview(Arc<Picture>),
    /// A picture of the scene browser that shows, by its index.
    Scene { index: usize, picture: Arc<Picture> },
    /// A playlist entry's thumbnail, by its path.
    Entry { path: String, picture: Arc<Picture> },
    /// The core cannot start: there are no pictures.
    Unavailable(String),
}

/// The requests under way.
#[derive(Default)]
pub(crate) struct Thumbs {
    core: Option<Thumbnailer>,
    /// The newest preview asked for and its time.
    preview: Option<(Ticket, f64)>,
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
                    if group == Group::Preview {
                        // An older preview shows until the newest comes;
                        // none once the pointer left the bar.
                        if self.thumbs.preview.is_some() {
                            events.push(ThumbEvent::Preview(picture));
                        }
                    } else if let Some(index) = self.thumbs.scenes.remove(&ticket) {
                        events.push(ThumbEvent::Scene { index, picture });
                    } else if let Some(path) = self.thumbs.entries.remove(&ticket) {
                        self.thumbs.ready.insert(path.clone());
                        events.push(ThumbEvent::Entry { path, picture });
                    }
                }
                Outcome::Failed { ticket, .. } => {
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
}
