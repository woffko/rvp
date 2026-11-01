//! The resume store (D9): the last file, the position in every file, volume,
//! mute and the window geometry, in `state.json` in the state directory.
//!
//! Positions are keyed by mpv's `path` property, because mpv normalizes
//! paths differently per platform (goal E engine tests). The store is written
//! atomically ([`crate::storage`]) on important changes and at most every
//! [`SAVE_INTERVAL`] while playing, so a killed process loses less than that
//! (section 4 asks for at most 5 s). The writing happens on the storage
//! writer's thread; [`ResumeStore::save_and_wait`] waits for it.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::storage;

/// How long a changed position may stay unsaved during playback.
pub const SAVE_INTERVAL: Duration = Duration::from_secs(2);
/// Positions closer than this to the start or the end are not kept: such a
/// file starts from the beginning next time.
pub const EDGE_SECONDS: f64 = 3.0;
/// Oldest positions are dropped beyond this many files.
pub const MAX_POSITIONS: usize = 5000;

/// The saved position in one file.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub seconds: f64,
    pub duration: Option<f64>,
    /// Increases with every update; the smallest is dropped first.
    pub sequence: u64,
}

/// The window's size and, where the platform reports it, position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

/// Contents of `state.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResumeState {
    pub version: u32,
    pub last_file: Option<String>,
    pub volume: Option<f64>,
    pub mute: bool,
    pub window: Option<WindowGeometry>,
    /// Where the control panel's window was last (screen coordinates;
    /// platforms that report positions only).
    pub panel: Option<(i32, i32)>,
    pub positions: BTreeMap<String, Position>,
    pub sequence: u64,
    /// The audio track chosen in a file ("Remember audio track
    /// selections"), by mpv's track ID.
    #[serde(default)]
    pub audio_tracks: BTreeMap<String, AudioTrack>,
    /// The subtitle tracks chosen in a file ("Remember subtitle track
    /// selections").
    #[serde(default)]
    pub subtitle_tracks: BTreeMap<String, SubtitleTracks>,
    /// The audio skin showed last (M14: "Use video skin at program start"
    /// off starts in it).
    #[serde(default)]
    pub audio_skin: bool,
    /// The window of each skin mode (`video`, `audio`) when each keeps its
    /// own.
    #[serde(default)]
    pub mode_windows: BTreeMap<String, WindowGeometry>,
    /// The settings window's page shown last (its key in
    /// `shell/qml/Settings.qml`): Preferences opens there again.
    #[serde(default)]
    pub settings_page: Option<String>,
}

/// The subtitles chosen in one file: the first (`None`: off) and the
/// second, by mpv's track IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleTracks {
    pub first: Option<i64>,
    pub second: Option<i64>,
    /// Increases with every choice; the smallest is dropped first.
    pub sequence: u64,
}

/// The audio track chosen in one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioTrack {
    pub id: i64,
    /// Increases with every choice; the smallest is dropped first.
    pub sequence: u64,
}

impl Default for ResumeState {
    fn default() -> Self {
        Self {
            version: 1,
            last_file: None,
            volume: None,
            mute: false,
            window: None,
            panel: None,
            positions: BTreeMap::new(),
            sequence: 0,
            audio_tracks: BTreeMap::new(),
            subtitle_tracks: BTreeMap::new(),
            audio_skin: false,
            mode_windows: BTreeMap::new(),
            settings_page: None,
        }
    }
}

/// The resume store and its file.
#[derive(Debug)]
pub struct ResumeStore {
    path: PathBuf,
    state: ResumeState,
    dirty: bool,
    last_save: Option<Instant>,
    writer: storage::Writer,
}

impl ResumeStore {
    /// Opens `state.json` in `dir`; a missing or damaged file starts empty
    /// (a damaged file is kept as `state.json.bad`).
    ///
    /// # Errors
    /// Fails on I/O errors other than a missing file, or when the writer
    /// thread cannot start.
    pub fn open(dir: &Path) -> io::Result<Self> {
        let path = dir.join("state.json");
        let state = storage::read_json(&path)?.unwrap_or_default();
        Ok(Self {
            path,
            state,
            dirty: false,
            last_save: None,
            writer: storage::Writer::new()?,
        })
    }

    #[must_use]
    pub fn state(&self) -> &ResumeState {
        &self.state
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved position of `file`, if any.
    #[must_use]
    pub fn position(&self, file: &str) -> Option<f64> {
        self.state
            .positions
            .get(file)
            .map(|position| position.seconds)
    }

    /// Records the playback position of `file`. Positions near either end
    /// clear the entry instead.
    pub fn set_position(&mut self, file: &str, seconds: f64, duration: Option<f64>) {
        let near_end = duration.is_some_and(|total| seconds > total - EDGE_SECONDS);
        if seconds < EDGE_SECONDS || near_end {
            self.clear_position(file);
            return;
        }
        self.state.sequence += 1;
        let position = Position {
            seconds,
            duration,
            sequence: self.state.sequence,
        };
        if self.state.positions.get(file) != Some(&position) {
            self.state.positions.insert(file.to_owned(), position);
            self.dirty = true;
        }
        if self.state.positions.len() > MAX_POSITIONS
            && let Some(oldest) = self
                .state
                .positions
                .iter()
                .min_by_key(|(_, position)| position.sequence)
                .map(|(name, _)| name.clone())
        {
            self.state.positions.remove(&oldest);
        }
    }

    /// The audio track chosen last in `file`.
    #[must_use]
    pub fn audio_track(&self, file: &str) -> Option<i64> {
        self.state.audio_tracks.get(file).map(|track| track.id)
    }

    /// Records the audio track chosen in `file`.
    pub fn set_audio_track(&mut self, file: &str, id: i64) {
        if self.audio_track(file) == Some(id) {
            return;
        }
        self.state.sequence += 1;
        let track = AudioTrack {
            id,
            sequence: self.state.sequence,
        };
        self.state.audio_tracks.insert(file.to_owned(), track);
        self.dirty = true;
        if self.state.audio_tracks.len() > MAX_POSITIONS
            && let Some(oldest) = self
                .state
                .audio_tracks
                .iter()
                .min_by_key(|(_, track)| track.sequence)
                .map(|(name, _)| name.clone())
        {
            self.state.audio_tracks.remove(&oldest);
        }
    }

    /// The subtitles chosen last in `file`: the first (`None`: off) and the
    /// second.
    #[must_use]
    pub fn subtitle_tracks(&self, file: &str) -> Option<(Option<i64>, Option<i64>)> {
        self.state
            .subtitle_tracks
            .get(file)
            .map(|tracks| (tracks.first, tracks.second))
    }

    /// Records the subtitles chosen in `file`.
    pub fn set_subtitle_tracks(&mut self, file: &str, first: Option<i64>, second: Option<i64>) {
        if self.subtitle_tracks(file) == Some((first, second)) {
            return;
        }
        self.state.sequence += 1;
        let tracks = SubtitleTracks {
            first,
            second,
            sequence: self.state.sequence,
        };
        self.state.subtitle_tracks.insert(file.to_owned(), tracks);
        self.dirty = true;
        if self.state.subtitle_tracks.len() > MAX_POSITIONS
            && let Some(oldest) = self
                .state
                .subtitle_tracks
                .iter()
                .min_by_key(|(_, tracks)| tracks.sequence)
                .map(|(name, _)| name.clone())
        {
            self.state.subtitle_tracks.remove(&oldest);
        }
    }

    /// Forgets the position of `file` (played to the end).
    pub fn clear_position(&mut self, file: &str) {
        if self.state.positions.remove(file).is_some() {
            self.dirty = true;
        }
    }

    pub fn set_last_file(&mut self, file: &str) {
        if self.state.last_file.as_deref() != Some(file) {
            self.state.last_file = Some(file.to_owned());
            self.dirty = true;
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        if self.state.volume != Some(volume) {
            self.state.volume = Some(volume);
            self.dirty = true;
        }
    }

    pub fn set_audio_skin(&mut self, audio: bool) {
        if self.state.audio_skin != audio {
            self.state.audio_skin = audio;
            self.dirty = true;
        }
    }

    /// The window of a skin mode (`video`, `audio`).
    #[must_use]
    pub fn mode_window(&self, mode: &str) -> Option<WindowGeometry> {
        self.state.mode_windows.get(mode).copied()
    }

    pub fn set_mode_window(&mut self, mode: &str, window: WindowGeometry) {
        if self.state.mode_windows.get(mode) != Some(&window) {
            self.state.mode_windows.insert(mode.to_owned(), window);
            self.dirty = true;
        }
    }

    pub fn set_mute(&mut self, mute: bool) {
        if self.state.mute != mute {
            self.state.mute = mute;
            self.dirty = true;
        }
    }

    pub fn set_window(&mut self, window: WindowGeometry) {
        if self.state.window != Some(window) {
            self.state.window = Some(window);
            self.dirty = true;
        }
    }

    pub fn set_panel(&mut self, position: (i32, i32)) {
        if self.state.panel != Some(position) {
            self.state.panel = Some(position);
            self.dirty = true;
        }
    }

    pub fn set_settings_page(&mut self, key: &str) {
        if self.state.settings_page.as_deref() != Some(key) {
            self.state.settings_page = Some(key.to_owned());
            self.dirty = true;
        }
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Queues the file for writing if anything changed. Write errors
    /// surface later through [`ResumeStore::take_error`].
    ///
    /// # Errors
    /// Fails when the state cannot be serialized.
    pub fn save(&mut self, now: Instant) -> io::Result<()> {
        if self.dirty {
            let mut bytes = serde_json::to_vec_pretty(&self.state).map_err(io::Error::other)?;
            bytes.push(b'\n');
            self.writer.write(&self.path, bytes);
            self.dirty = false;
            self.last_save = Some(now);
        }
        Ok(())
    }

    /// Saves and waits until the file is on disk (shutdown, tests).
    ///
    /// # Errors
    /// Fails on serialization or I/O errors.
    pub fn save_and_wait(&mut self, now: Instant) -> io::Result<()> {
        self.save(now)?;
        self.writer.flush();
        match self.writer.take_error() {
            Some(error) => Err(io::Error::other(error)),
            None => Ok(()),
        }
    }

    /// A write error from the background writer, once.
    #[must_use]
    pub fn take_error(&self) -> Option<String> {
        self.writer.take_error()
    }

    /// Writes the file if it changed and [`SAVE_INTERVAL`] has passed since
    /// the last write.
    ///
    /// # Errors
    /// Fails on I/O errors.
    pub fn save_if_due(&mut self, now: Instant) -> io::Result<()> {
        let due = self
            .last_save
            .is_none_or(|last| now.duration_since(last) >= SAVE_INTERVAL);
        if due { self.save(now) } else { Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-resume-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn positions_round_trip_through_the_file() {
        let dir = scratch("round-trip");
        let mut store = ResumeStore::open(&dir).expect("open");
        store.set_position("/films/a.mkv", 125.5, Some(5400.0));
        store.set_last_file("/films/a.mkv");
        store.set_volume(22.0);
        store.set_window(WindowGeometry {
            x: Some(97),
            y: Some(68),
            width: 2230,
            height: 908,
            maximized: false,
        });
        store.save_and_wait(Instant::now()).expect("save");
        let reopened = ResumeStore::open(&dir).expect("reopen");
        assert_eq!(reopened.position("/films/a.mkv"), Some(125.5));
        assert_eq!(reopened.state().last_file.as_deref(), Some("/films/a.mkv"));
        assert_eq!(reopened.state().volume, Some(22.0));
        assert_eq!(reopened.state().window.map(|w| w.width), Some(2230));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn edges_clear_the_position() {
        let dir = scratch("edges");
        let mut store = ResumeStore::open(&dir).expect("open");
        store.set_position("a", 100.0, Some(200.0));
        assert_eq!(store.position("a"), Some(100.0));
        store.set_position("a", 1.0, Some(200.0));
        assert_eq!(store.position("a"), None);
        store.set_position("a", 100.0, Some(200.0));
        store.set_position("a", 198.5, Some(200.0));
        assert_eq!(store.position("a"), None);
    }

    #[test]
    fn saves_are_rate_limited() {
        let dir = scratch("rate");
        let mut store = ResumeStore::open(&dir).expect("open");
        let start = Instant::now();
        store.set_position("a", 10.0, None);
        store.save_if_due(start).expect("first save");
        assert!(!store.is_dirty());
        store.set_position("a", 11.0, None);
        store
            .save_if_due(start + Duration::from_millis(500))
            .expect("too early");
        assert!(store.is_dirty());
        store.save_if_due(start + SAVE_INTERVAL).expect("due");
        assert!(!store.is_dirty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oldest_positions_are_dropped() {
        let dir = scratch("cap");
        let mut store = ResumeStore::open(&dir).expect("open");
        for index in 0..=MAX_POSITIONS {
            store.set_position(&format!("file-{index}"), 10.0, None);
        }
        assert_eq!(store.state().positions.len(), MAX_POSITIONS);
        assert_eq!(store.position("file-0"), None);
        assert_eq!(store.position(&format!("file-{MAX_POSITIONS}")), Some(10.0));
    }
}
