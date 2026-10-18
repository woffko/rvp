//! M12's visualizations for audio files (PotPlayer's Visualization menu and
//! its General > Skins > Visualization page): the album art toggle (mpv's
//! `audio-display`), FFmpeg's visualizations through `lavfi-complex`
//! (`showcqt`, `showspectrum`, `avectorscope`, `showwaves`) in place of
//! PotPlayer's WMP plugins, and a slideshow of pictures that the window
//! shows (`shell/qml/Slideshow.qml`).
//!
//! `lavfi-complex` takes the audio track and gives mpv both the sound and
//! the picture, so it is set only while a file without video plays (cover
//! art is not video), and cleared before anything else loads.

use std::path::{Path, PathBuf};

use rvp_engine::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::app::{App, UiRequest};

/// What an audio file shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisualMode {
    /// The spectrum on a musical scale (`showcqt`).
    #[default]
    Bars,
    /// A scrolling spectrogram (`showspectrum`).
    Spectrum,
    /// The stereo image (`avectorscope`).
    Scope,
    /// The waveform (`showwaves`).
    Waves,
    /// Pictures from folders, one after another.
    Slideshow,
}

impl VisualMode {
    pub const ALL: [VisualMode; 5] = [
        VisualMode::Bars,
        VisualMode::Spectrum,
        VisualMode::Scope,
        VisualMode::Waves,
        VisualMode::Slideshow,
    ];

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            VisualMode::Bars => "Spectrum bars",
            VisualMode::Spectrum => "Spectrogram",
            VisualMode::Scope => "Vector scope",
            VisualMode::Waves => "Waveform",
            VisualMode::Slideshow => "Slideshow",
        }
    }
}

/// How a slide fills the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SlideFill {
    /// The whole picture, with bars.
    #[default]
    Fit,
    /// The area, cropping the picture.
    Fill,
    Stretch,
}

/// The visualization settings (`visual` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// PotPlayer's independent Visualization options, a check mark each.
#[allow(clippy::struct_excessive_bools)]
pub struct VisualSettings {
    /// "Enable visualization for audio tracks".
    pub enabled: bool,
    /// "Display album art/cover pictures" (mpv's `audio-display`); with it,
    /// a file's cover shows instead of the visualization.
    pub album_art: bool,
    pub mode: VisualMode,
    /// The visualization's size in pixels and its frame rate.
    pub size: [u32; 2],
    pub rate: u32,
    /// "Change visualization every # sec." (0: never).
    pub change_every: u32,
    /// Folders of the slideshow's pictures; none: the playing file's
    /// folder.
    pub folders: Vec<PathBuf>,
    /// Seconds a slide shows, and the cross-fade's.
    pub slide_seconds: f64,
    pub fade_seconds: f64,
    pub fill: SlideFill,
    pub random: bool,
    /// The picture's name in a corner.
    pub show_name: bool,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            album_art: true,
            mode: VisualMode::Bars,
            size: [1280, 720],
            rate: 30,
            change_every: 0,
            folders: Vec::new(),
            slide_seconds: 5.0,
            fade_seconds: 1.0,
            fill: SlideFill::Fit,
            random: false,
            show_name: false,
        }
    }
}

impl VisualSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let clamp = crate::playback::clamp_number;
        if !(160..=3840).contains(&self.size[0]) || !(120..=2160).contains(&self.size[1]) {
            notes.push("visual.size must be 160x120 to 3840x2160: 1280x720 used".to_owned());
            self.size = [1280, 720];
        }
        if !(5..=60).contains(&self.rate) {
            notes.push("visual.rate must be 5 to 60 frames a second: 30 used".to_owned());
            self.rate = 30;
        }
        if self.change_every > 3600 {
            notes.push("visual.change-every must be 0 to 3600 seconds".to_owned());
            self.change_every = 3600;
        }
        clamp(
            &mut self.slide_seconds,
            1.0,
            600.0,
            "visual.slide-seconds",
            notes,
        );
        clamp(
            &mut self.fade_seconds,
            0.0,
            10.0,
            "visual.fade-seconds",
            notes,
        );
    }
}

/// One of the Visualization menu's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisualCommand {
    ToggleEnabled,
    ToggleAlbumArt,
    Mode(VisualMode),
    /// Jump to previous (`false`) or next slide.
    Slide(bool),
}

/// The `lavfi-complex` graph of a mode, or `None` for the slideshow.
#[must_use]
pub fn graph(mode: VisualMode, size: [u32; 2], rate: u32) -> Option<String> {
    let [width, height] = size;
    let picture = match mode {
        VisualMode::Bars => {
            format!("showcqt=s={width}x{height}:r={rate}:axis=0:bar_g=2:sono_g=4:bar_v=9:sono_v=17")
        }
        VisualMode::Spectrum => format!(
            "showspectrum=s={width}x{height}:slide=scroll:mode=combined:color=intensity:scale=log,fps={rate}"
        ),
        VisualMode::Scope => {
            let square = width.min(height);
            format!(
                "avectorscope=s={square}x{square}:r={rate}:zoom=1.5:draw=line:rc=40:gc=200:bc=255"
            )
        }
        VisualMode::Waves => {
            format!("showwaves=s={width}x{height}:r={rate}:mode=cline:colors=0x40c0ff")
        }
        VisualMode::Slideshow => return None,
    };
    Some(format!(
        "[aid1]asplit[ao][rvpviz];[rvpviz]{picture},format=yuv420p[vo]"
    ))
}

/// Picture files of a folder, by name (the slideshow's).
#[must_use]
pub fn pictures(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "bmp" | "gif" | "webp"
                    )
                })
        })
        .collect();
    files.sort();
    files
}

/// This run's visualization state.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct VisualState {
    /// The `lavfi-complex` graph RVP set (empty: none).
    pub(crate) graph: String,
    /// The slideshow shows (the window draws it).
    pub(crate) slideshow: bool,
    /// The mode `change_every` reached, and when it changed last
    /// (playback seconds).
    pub(crate) cycled: Option<(VisualMode, f64)>,
}

impl App {
    /// The mode that shows now: the setting's, or the one "Change
    /// visualization" reached.
    fn visual_mode(&self) -> VisualMode {
        self.visual
            .cycled
            .map_or(self.settings.visual.mode, |(mode, _)| mode)
    }

    /// Sets what an audio file shows: the cover, a visualization or the
    /// slideshow (after a file loaded, and after a change). The file is
    /// audio only without a video track that is not a cover.
    pub(crate) fn apply_visual(&mut self, engine: &Engine) {
        let settings = &self.settings.visual;
        drop(engine.set_text(
            "audio-display",
            if settings.album_art {
                "embedded-first"
            } else {
                "no"
            },
        ));
        if !self.model.media_loaded() {
            return;
        }
        let tracks = engine.tracks();
        let video = |cover: bool| {
            tracks
                .iter()
                .any(|track| track.kind == rvp_engine::TrackKind::Video && track.album_art == cover)
        };
        let audio = tracks
            .iter()
            .any(|track| track.kind == rvp_engine::TrackKind::Audio);
        let has_cover = settings.album_art && video(true);
        let wanted = settings.enabled && audio && !video(false) && !has_cover;
        let mode = self.visual_mode();
        let graph = if wanted {
            graph(mode, settings.size, settings.rate).unwrap_or_default()
        } else {
            String::new()
        };
        self.visual.slideshow = wanted && mode == VisualMode::Slideshow;
        if graph != self.visual.graph {
            drop(engine.set_text("lavfi-complex", &graph));
            self.visual.graph = graph;
        }
    }

    /// Clears the visualization before another file loads: the graph
    /// names the audio file's track.
    pub(crate) fn clear_visual(&mut self, engine: &Engine) {
        if !self.visual.graph.is_empty() {
            drop(engine.set_text("lavfi-complex", ""));
            self.visual.graph.clear();
        }
        self.visual.slideshow = false;
    }

    /// "Change visualization every # sec.": the next mode when its time
    /// came (playback seconds).
    pub(crate) fn visual_tick(&mut self, engine: &Engine, time: f64) {
        let every = self.settings.visual.change_every;
        if every == 0 || !self.settings.visual.enabled || self.visual.graph.is_empty() {
            return;
        }
        let (mode, since) = self
            .visual
            .cycled
            .unwrap_or((self.settings.visual.mode, time));
        if time < since {
            self.visual.cycled = Some((mode, time));
            return;
        }
        if time - since < f64::from(every) {
            self.visual.cycled.get_or_insert((mode, since));
            return;
        }
        // The FFmpeg modes in turn (the slideshow has its own timing).
        let modes = &VisualMode::ALL[..4];
        let index = modes
            .iter()
            .position(|m| *m == mode)
            .map_or(0, |i| (i + 1) % modes.len());
        self.visual.cycled = Some((modes[index], time));
        self.apply_visual(engine);
    }

    /// The Visualization menu's commands.
    pub(crate) fn visual_command(
        &mut self,
        engine: &Engine,
        command: VisualCommand,
    ) -> Option<UiRequest> {
        let visual = &mut self.settings.visual;
        let text = match command {
            VisualCommand::ToggleEnabled => {
                visual.enabled = !visual.enabled;
                format!(
                    "Visualization: {}",
                    if visual.enabled { "on" } else { "off" }
                )
            }
            VisualCommand::ToggleAlbumArt => {
                visual.album_art = !visual.album_art;
                format!("Album art: {}", if visual.album_art { "on" } else { "off" })
            }
            VisualCommand::Mode(mode) => {
                visual.mode = mode;
                visual.enabled = true;
                self.visual.cycled = None;
                format!("Visualization: {}", mode.text())
            }
            VisualCommand::Slide(next) => {
                return self.visual.slideshow.then_some(UiRequest::Slide(if next {
                    1
                } else {
                    -1
                }));
            }
        };
        self.save_settings();
        self.apply_visual(engine);
        Some(UiRequest::Osd(text))
    }

    /// The check mark of a Visualization command.
    pub(crate) fn visual_checked(&self, command: VisualCommand) -> Option<bool> {
        let visual = &self.settings.visual;
        match command {
            VisualCommand::ToggleEnabled => Some(visual.enabled),
            VisualCommand::ToggleAlbumArt => Some(visual.album_art),
            VisualCommand::Mode(mode) => Some(visual.enabled && visual.mode == mode),
            VisualCommand::Slide(_) => None,
        }
    }

    /// Whether the slideshow shows now.
    #[must_use]
    pub fn slideshow_shown(&self) -> bool {
        self.visual.slideshow
    }

    /// The slideshow for the window (JSON): whether it shows, its pictures
    /// and its timing.
    #[must_use]
    pub fn slideshow_json(&self) -> String {
        let visual = &self.settings.visual;
        let mut files = Vec::new();
        if self.visual.slideshow {
            let folders: Vec<PathBuf> = if visual.folders.is_empty() {
                self.model
                    .loaded_path()
                    .and_then(|path| Path::new(path).parent())
                    .map(Path::to_path_buf)
                    .into_iter()
                    .collect()
            } else {
                visual.folders.clone()
            };
            for folder in &folders {
                files.extend(pictures(folder));
            }
        }
        let files: Vec<Value> = files
            .iter()
            .map(|path| Value::String(path.to_string_lossy().into_owned()))
            .collect();
        json!({
            "shown": self.visual.slideshow,
            "files": files,
            "seconds": visual.slide_seconds,
            "fade": visual.fade_seconds,
            "fill": visual.fill,
            "random": visual.random,
            "names": visual.show_name,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphs_give_mpv_the_sound_and_a_picture() {
        let bars = graph(VisualMode::Bars, [1280, 720], 30).unwrap_or_default();
        assert!(bars.starts_with("[aid1]asplit[ao][rvpviz];[rvpviz]showcqt=s=1280x720:r=30"));
        assert!(bars.ends_with(",format=yuv420p[vo]"));
        let scope = graph(VisualMode::Scope, [1280, 720], 25).unwrap_or_default();
        assert!(scope.contains("avectorscope=s=720x720:r=25"), "{scope}");
        assert_eq!(graph(VisualMode::Slideshow, [1280, 720], 30), None);
    }
}
