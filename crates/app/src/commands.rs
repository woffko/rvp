//! The commands M4 added to M2's window controls, as PotPlayer's menus name
//! them: frame steps and jumps, speed, A-B repeat, repeat and end-of-file
//! actions, audio and subtitle sync, subtitle style, tracks, aspect ratio,
//! fit and zoom, saved frames, window size and "Stay on Top".
//!
//! RVP keeps the picture settings for the session ([`Session`]) and sets
//! them on mpv; mpv keeps them across files, as PotPlayer does. A-B points
//! belong to one file and are cleared when the next one starts. Steps are
//! PotPlayer's: speed 0.1x, zoom and stretch about 7 %, picture moves 1 %
//! of the video's size, subtitle size 10 %, subtitle position 1 % of the
//! screen; the sync steps are settings.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rvp_engine::{Engine, EngineError, PropertyId, Seek, Track, TrackKind};

use crate::action::{
    AbRepeat, Action, AfterPlayback, Aspect, Delay, Fit, ImageFormat, Jump, OnTop, Repeat, Speed,
    SubtitleStyle, Zoom,
};
use crate::app::{App, UiRequest};

/// Picture and playback state RVP sets on mpv for this session.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub aspect: Aspect,
    pub fit: Fit,
    /// mpv `video-zoom` (log2 of the factor).
    pub zoom: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    /// The speed `Speed::Default` returns to from 1.0x.
    pub last_speed: f64,
    pub point_a: Option<f64>,
    pub point_b: Option<f64>,
    /// Repeating between A and B (when both are set).
    pub ab_active: bool,
    pub after: AfterPlayback,
    /// Playback information shown over the video.
    pub info: bool,
    /// The last "Stay on Top" state sent to the window.
    pub on_top: Option<bool>,
    /// The screen rotation in degrees.
    pub rotation: u16,
    /// The control panel window is shown.
    pub control_panel: bool,
    /// Both, Left or Right channel of a stereo track.
    pub stereo_channel: crate::audio::StereoChannel,
    /// The audio recording under way.
    pub recording: Option<std::path::PathBuf>,
    /// When a volume key last changed the volume, and how fast it goes
    /// ("Accelerate control speed").
    pub last_volume_key: Option<std::time::Instant>,
    pub volume_acceleration: f64,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            aspect: Aspect::Default,
            fit: Fit::KeepAspect,
            zoom: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            last_speed: 1.0,
            point_a: None,
            point_b: None,
            ab_active: false,
            after: AfterPlayback::Nothing,
            info: false,
            on_top: None,
            rotation: 0,
            control_panel: false,
            stereo_channel: crate::audio::StereoChannel::Both,
            recording: None,
            last_volume_key: None,
            volume_acceleration: 1.0,
        }
    }
}

const SPEED_STEP: f64 = 0.1;
const ZOOM_STEP: f64 = 0.1;
const STRETCH_STEP: f64 = 0.05;
const PAN_STEP: f64 = 0.01;
const SUBTITLE_SCALE_STEP: f64 = 0.1;

/// `0:01:23` for A-B notices.
fn clock(seconds: f64) -> String {
    // Media times are far inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = seconds.max(0.0).round() as u64;
    format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
}

/// `+0.5 s`, `-50 ms`.
pub(crate) fn signed_seconds(seconds: f64) -> String {
    if seconds.abs() < 1.0 && seconds != 0.0 {
        format!("{:+.0} ms", seconds * 1000.0)
    } else {
        format!("{seconds:+.1} s")
    }
}

/// How a track appears in menus and notices: `English (AC3)`.
#[must_use]
pub fn track_label(track: &Track) -> String {
    let mut label = match (&track.title, &track.language) {
        (Some(title), Some(language)) => format!("{title} [{language}]"),
        (Some(title), None) => title.clone(),
        (None, Some(language)) => language.clone(),
        (None, None) => format!("Track {}", track.id),
    };
    if let Some(codec) = &track.codec {
        let _ = write!(label, " ({codec})");
    }
    label
}

fn aspect_override(aspect: Aspect) -> (&'static str, &'static str) {
    // (video-aspect-override, video-aspect-method)
    match aspect {
        Aspect::Default => ("-1", "container"),
        Aspect::Original => ("-1", "bitstream"),
        Aspect::R4x3 => ("4:3", "container"),
        Aspect::R16x9 => ("16:9", "container"),
        Aspect::R16x10 => ("16:10", "container"),
        Aspect::R185x1 => ("1.85:1", "container"),
        Aspect::R235x1 => ("2.35:1", "container"),
    }
}

/// The text after "Aspect ratio:".
#[must_use]
pub fn aspect_text(aspect: Aspect) -> &'static str {
    match aspect {
        Aspect::Default => "Default",
        Aspect::Original => "Original",
        Aspect::R4x3 => "4:3",
        Aspect::R16x9 => "16:9",
        Aspect::R16x10 => "16:10",
        Aspect::R185x1 => "1.85:1",
        Aspect::R235x1 => "2.35:1",
    }
}

impl App {
    /// Carries out the actions [`App::dispatch`] does not handle itself.
    pub(crate) fn command(&mut self, engine: &Engine, action: Action) -> Option<UiRequest> {
        match action {
            Action::OpenFolder => Some(UiRequest::OpenFolderDialog),
            Action::OpenUrl => Some(UiRequest::OpenUrlDialog),
            Action::OpenSubtitle => Some(UiRequest::OpenSubtitleDialog),
            Action::Reopen => self.reopen(engine),
            Action::FrameBack => self.loaded_command(engine, "frame-back-step", &[]),
            Action::FrameForward => self.loaded_command(engine, "frame-step", &[]),
            Action::JumpTo(jump) => self.jump(engine, jump),
            Action::Speed(speed) => self.speed(engine, speed),
            Action::AbRepeat(change) => Some(self.ab_repeat(engine, change)),
            Action::RepeatCycle => {
                let next = match self.settings.repeat {
                    Repeat::Off => Repeat::Album,
                    Repeat::Album => Repeat::Item,
                    Repeat::Item => Repeat::Off,
                };
                Some(self.set_repeat(next))
            }
            Action::SetRepeat(repeat) => Some(self.set_repeat(repeat)),
            Action::ToggleKeyframeSeeks => {
                self.settings.keyframe_seeks = !self.settings.keyframe_seeks;
                self.save_settings();
                Some(UiRequest::Osd(
                    if self.settings.keyframe_seeks {
                        "Seek: to keyframes"
                    } else {
                        "Seek: exact"
                    }
                    .to_owned(),
                ))
            }
            Action::TogglePreventScreenSaver => {
                self.settings.prevent_screen_saver = !self.settings.prevent_screen_saver;
                self.save_settings();
                Some(UiRequest::Osd(format!(
                    "Prevent screen saver during playback: {}",
                    if self.settings.prevent_screen_saver {
                        "On"
                    } else {
                        "Off"
                    }
                )))
            }
            Action::AfterPlaybackCycle => {
                let next = match self.session.after {
                    AfterPlayback::Nothing => AfterPlayback::ExitAfterFile,
                    AfterPlayback::ExitAfterFile => AfterPlayback::ExitAfterList,
                    AfterPlayback::ExitAfterList => AfterPlayback::Nothing,
                };
                Some(self.set_after(next))
            }
            Action::SetAfterPlayback(after) => Some(self.set_after(after)),
            Action::CycleAudio => self.cycle_track(engine, TrackKind::Audio),
            Action::CycleSubtitles => self.cycle_track(engine, TrackKind::Subtitle),
            Action::CycleVideo => self.cycle_track(engine, TrackKind::Video),
            Action::AudioDelay(Delay::Reset) => {
                // The default sync of the Language/Sync page, in ms.
                let default = self.settings.audio.default_delay / 1000.0;
                self.set_audio_delay(engine, default)
            }
            Action::AudioDelay(delay) => {
                let step = self.settings.audio_delay_step;
                let value =
                    self.delay(engine, "audio-delay", PropertyId::AudioDelay, delay, step)?;
                self.audio_delay_changed(value);
                Some(UiRequest::Osd(format!(
                    "Audio sync: {}",
                    signed_seconds(value)
                )))
            }
            Action::Audio(command) => self.audio_command(engine, command),
            Action::SubtitleDelay(delay) => {
                let step = self.settings.subtitle_delay_step;
                self.delay(engine, "sub-delay", PropertyId::SubDelay, delay, step)
                    .map(|value| {
                        UiRequest::Osd(format!("Subtitle sync: {}", signed_seconds(value)))
                    })
            }
            Action::Subtitle(style) => self.subtitle_style(engine, style),
            Action::Preferences => Some(UiRequest::Preferences(None)),
            Action::KeysAndMouse => Some(UiRequest::Preferences(Some("input"))),
            Action::Settings(page) => Some(UiRequest::Preferences(Some(page.key()))),
            Action::About => Some(UiRequest::About),
            Action::ControlPanel => {
                self.session.control_panel = !self.session.control_panel;
                Some(UiRequest::ControlPanel(self.session.control_panel))
            }
            Action::ToggleEqualizer => Some(self.toggle_equalizer(engine)),
            Action::SaveFrameAs => self.frame_path().map(UiRequest::SaveFrameDialog),
            Action::SubtitleFont => Some(UiRequest::SubtitleFontDialog(
                self.settings.subtitle_font.clone(),
            )),
            other => self.view_command(engine, other),
        }
    }

    /// The picture, frame capture and window commands.
    fn view_command(&mut self, engine: &Engine, action: Action) -> Option<UiRequest> {
        match action {
            Action::AspectCycle => {
                let next = match self.session.aspect {
                    Aspect::Default | Aspect::Original => Aspect::R4x3,
                    Aspect::R4x3 => Aspect::R16x9,
                    Aspect::R16x9 => Aspect::R16x10,
                    Aspect::R16x10 => Aspect::R185x1,
                    Aspect::R185x1 => Aspect::R235x1,
                    Aspect::R235x1 => Aspect::Default,
                };
                Some(self.set_aspect(engine, next))
            }
            Action::SetAspect(aspect) => Some(self.set_aspect(engine, aspect)),
            Action::VideoFit(fit) => {
                self.session.fit = fit;
                drop(self.apply_picture(engine, false));
                Some(UiRequest::Osd(
                    match fit {
                        Fit::KeepAspect => "Keep aspect ratio",
                        Fit::Crop => "Stretch by keeping aspect ratio",
                        Fit::Stretch => "Stretch",
                    }
                    .to_owned(),
                ))
            }
            Action::Zoom(zoom) => Some(self.zoom(engine, zoom)),
            Action::SaveFrame => self.save_frame(false),
            Action::SaveScreen => self.save_frame(true),
            Action::SetImageFormat(format) => {
                self.settings.image_format = format;
                self.save_settings();
                Some(UiRequest::Osd(
                    match format {
                        ImageFormat::Png => "Images: PNG",
                        ImageFormat::Jpeg => "Images: JPEG",
                    }
                    .to_owned(),
                ))
            }
            Action::OpenImageFolder => {
                let folder = self.image_folder();
                drop(std::fs::create_dir_all(&folder));
                Some(UiRequest::OpenExternal(folder))
            }
            Action::SelectImageFolder => Some(UiRequest::ImageFolderDialog),
            Action::OnTopCycle => {
                let next = match self.settings.on_top {
                    OnTop::Never => OnTop::Always,
                    OnTop::Always => OnTop::WhilePlaying,
                    OnTop::WhilePlaying => OnTop::Never,
                };
                Some(self.set_on_top(next))
            }
            Action::SetOnTop(mode) => Some(self.set_on_top(mode)),
            Action::WindowScale(scale) => Some(UiRequest::ScaleWindow(scale.factor())),
            Action::WindowFitAspect => Some(UiRequest::FitAspect),
            Action::PlaybackInfo => {
                self.session.info = !self.session.info;
                Some(UiRequest::ShowInfo(self.session.info))
            }
            // M6's video processing; M2's and M3's actions are
            // App::dispatch's.
            other => self.video_command(engine, other),
        }
    }

    fn loaded_command(&self, engine: &Engine, name: &str, args: &[&str]) -> Option<UiRequest> {
        if self.model.media_loaded() {
            drop(engine.command(name, args));
        }
        None
    }

    fn reopen(&mut self, engine: &Engine) -> Option<UiRequest> {
        let lists = self.library().lists();
        let target = lists
            .playing
            .and_then(|album| Some((album, lists.albums.get(album)?.current?)));
        match target {
            Some((album, index)) => {
                drop(self.play_entry(engine, album, index, false));
                None
            }
            None => self.store().state().last_file.clone().and_then(|last| {
                drop(self.open(engine, &[last], false));
                None
            }),
        }
    }

    fn jump(&mut self, engine: &Engine, jump: Jump) -> Option<UiRequest> {
        if !self.model.media_loaded() {
            return None;
        }
        let duration = self.model.double(PropertyId::Duration).unwrap_or(0.0);
        let result = match jump {
            Jump::Start => self.seek_to(engine, Seek::Absolute(0.0)),
            Jump::Middle => self.seek_to(engine, Seek::Fraction(0.5)),
            Jump::BeforeEnd => self.seek_to(engine, Seek::Absolute((duration - 30.0).max(0.0))),
            Jump::PreviousSubtitle => engine.command("sub-seek", &["-1"]),
            Jump::NextSubtitle => engine.command("sub-seek", &["1"]),
            Jump::PreviousChapter => engine.command("add", &["chapter", "-1"]),
            Jump::NextChapter => engine.command("add", &["chapter", "1"]),
        };
        drop(result);
        None
    }

    fn speed(&mut self, engine: &Engine, change: Speed) -> Option<UiRequest> {
        let current = self.model.double(PropertyId::Speed).unwrap_or(1.0);
        let next = match change {
            Speed::Faster => current + SPEED_STEP,
            Speed::Slower => current - SPEED_STEP,
            Speed::Default if (current - 1.0).abs() > 1e-6 => {
                self.session.last_speed = current;
                1.0
            }
            Speed::Default => self.session.last_speed,
        };
        let next = ((next * 10.0).round() / 10.0).clamp(0.1, 16.0);
        // With the normalizer on, the tempo filter after it takes the speed
        // (`crate::normalizer`): no seek is needed.
        engine.set_double("speed", next).ok()?;
        Some(UiRequest::Osd(format!("Speed: {next:.1}x")))
    }

    fn ab_repeat(&mut self, engine: &Engine, change: AbRepeat) -> UiRequest {
        let time = self.model.double(PropertyId::TimePos);
        let session = &mut self.session;
        match change {
            AbRepeat::SetA => session.point_a = time,
            AbRepeat::SetB => session.point_b = time,
            AbRepeat::ClearA => session.point_a = None,
            AbRepeat::ClearB => session.point_b = None,
            AbRepeat::Clear => {
                session.point_a = None;
                session.point_b = None;
            }
            AbRepeat::Toggle => session.ab_active = !session.ab_active,
        }
        // Setting both points starts the repeat, as in PotPlayer.
        if matches!(change, AbRepeat::SetA | AbRepeat::SetB) {
            session.ab_active = true;
        }
        if let (Some(a), Some(b)) = (session.point_a, session.point_b)
            && b < a
        {
            session.point_a = Some(b);
            session.point_b = Some(a);
        }
        self.apply_ab(engine);
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

    /// Sets mpv's A-B loop from the session.
    pub(crate) fn apply_ab(&self, engine: &Engine) {
        let session = &self.session;
        let set = |name: &str, point: Option<f64>| {
            drop(match point.filter(|_| session.ab_active) {
                Some(seconds) => engine.set_double(name, seconds),
                None => engine.set_text(name, "no"),
            });
        };
        let both = session.point_a.is_some() && session.point_b.is_some();
        set("ab-loop-a", session.point_a.filter(|_| both));
        set("ab-loop-b", session.point_b.filter(|_| both));
    }

    fn set_repeat(&mut self, repeat: Repeat) -> UiRequest {
        self.settings.repeat = repeat;
        self.save_settings();
        UiRequest::Osd(
            match repeat {
                Repeat::Off => "Repeat: off",
                Repeat::Album => "Repeat: playlist",
                Repeat::Item => "Repeat: playing item",
            }
            .to_owned(),
        )
    }

    fn set_after(&mut self, after: AfterPlayback) -> UiRequest {
        self.session.after = after;
        UiRequest::Osd(
            match after {
                AfterPlayback::Nothing => "When playback ends: do nothing",
                AfterPlayback::ExitAfterFile => "When playback ends: exit",
                AfterPlayback::ExitAfterList => "When the playlist ends: exit",
            }
            .to_owned(),
        )
    }

    fn cycle_track(&mut self, engine: &Engine, kind: TrackKind) -> Option<UiRequest> {
        if !self.model.media_loaded() {
            return None;
        }
        let tracks: Vec<Track> = engine
            .tracks()
            .into_iter()
            .filter(|track| track.kind == kind)
            .collect();
        let property = match kind {
            TrackKind::Audio => "aid",
            TrackKind::Video => "vid",
            TrackKind::Subtitle => "sid",
        };
        let noun = match kind {
            TrackKind::Audio => "Audio",
            TrackKind::Video => "Video",
            TrackKind::Subtitle => "Subtitle",
        };
        // Subtitles cycle through "off" too.
        let mut choices: Vec<Option<&Track>> = tracks.iter().map(Some).collect();
        if kind == TrackKind::Subtitle {
            choices.push(None);
        }
        if choices.is_empty() {
            return Some(UiRequest::Osd(format!("{noun}: none")));
        }
        let current = choices
            .iter()
            .position(|choice| choice.is_some_and(|track| track.selected))
            .unwrap_or(choices.len() - 1);
        let next = choices[(current + 1) % choices.len()];
        let result = match next {
            Some(track) => engine.set_text(property, &track.id.to_string()),
            None => engine.set_text(property, "no"),
        };
        if kind == TrackKind::Audio
            && let Some(track) = next
        {
            self.audio_track_chosen(track.id);
        }
        result.ok().map(|()| {
            UiRequest::Osd(format!(
                "{noun}: {}",
                next.map_or_else(|| "off".to_owned(), track_label)
            ))
        })
    }

    /// Selects a track by mpv's ID (`None`: subtitles off).
    pub fn select_track(&mut self, engine: &Engine, kind: TrackKind, id: Option<i64>) {
        let property = match kind {
            TrackKind::Audio => "aid",
            TrackKind::Video => "vid",
            TrackKind::Subtitle => "sid",
        };
        drop(match id {
            Some(id) => engine.set_text(property, &id.to_string()),
            None => engine.set_text(property, "no"),
        });
        if kind == TrackKind::Audio
            && let Some(id) = id
        {
            self.audio_track_chosen(id);
        }
    }

    fn delay(
        &self,
        engine: &Engine,
        name: &str,
        property: PropertyId,
        delay: Delay,
        step: f64,
    ) -> Option<f64> {
        let current = self.model.double(property).unwrap_or(0.0);
        let next = match delay {
            Delay::Earlier => current - step,
            Delay::Later => current + step,
            Delay::Reset => 0.0,
        };
        let next = (next * 1000.0).round() / 1000.0;
        engine.set_double(name, next).ok().map(|()| next)
    }

    fn subtitle_style(&mut self, engine: &Engine, style: SubtitleStyle) -> Option<UiRequest> {
        let scale = self
            .model
            .double(PropertyId::SubScale)
            .unwrap_or(self.settings.subtitle_scale);
        let position = self.model.double(PropertyId::SubPos).unwrap_or(100.0);
        let (result, text) = match style {
            SubtitleStyle::Bigger | SubtitleStyle::Smaller => {
                let step = if style == SubtitleStyle::Bigger {
                    SUBTITLE_SCALE_STEP
                } else {
                    -SUBTITLE_SCALE_STEP
                };
                let next = ((scale + step) * 10.0).round() / 10.0;
                let next = next.clamp(0.3, 3.0);
                (
                    engine.set_double("sub-scale", next),
                    format!("Subtitle size: {:.0} %", next * 100.0),
                )
            }
            SubtitleStyle::Up | SubtitleStyle::Down | SubtitleStyle::DefaultPosition => {
                let next = match style {
                    SubtitleStyle::Up => position - 1.0,
                    SubtitleStyle::Down => position + 1.0,
                    _ => 100.0,
                }
                .clamp(0.0, 150.0);
                (
                    engine.set_double("sub-pos", next),
                    format!("Subtitle position: {next:.0} %"),
                )
            }
            SubtitleStyle::AlignLeft | SubtitleStyle::AlignCenter | SubtitleStyle::AlignRight => {
                let (value, text) = match style {
                    SubtitleStyle::AlignLeft => ("left", "Subtitles: align left"),
                    SubtitleStyle::AlignRight => ("right", "Subtitles: align right"),
                    _ => ("center", "Subtitles: align center"),
                };
                (engine.set_text("sub-align-x", value), text.to_owned())
            }
        };
        result.ok().map(|()| UiRequest::Osd(text))
    }

    fn set_aspect(&mut self, engine: &Engine, aspect: Aspect) -> UiRequest {
        self.session.aspect = aspect;
        drop(self.apply_picture(engine, false));
        UiRequest::Osd(format!("Aspect ratio: {}", aspect_text(aspect)))
    }

    /// Sets the aspect ratio, fit, zoom and position on mpv. `stretch` is
    /// fullscreen (stretch), which overrides the fit while it lasts.
    ///
    /// # Errors
    /// Fails when libmpv rejects a value.
    pub fn apply_picture(&self, engine: &Engine, stretch: bool) -> Result<(), EngineError> {
        let session = &self.session;
        let (aspect, method) = aspect_override(session.aspect);
        engine.set_text("video-aspect-override", aspect)?;
        engine.set_text("video-aspect-method", method)?;
        let keep = !stretch && session.fit != Fit::Stretch;
        engine.set_flag("keepaspect", keep)?;
        let panscan = if session.fit == Fit::Crop { 1.0 } else { 0.0 };
        engine.set_double("panscan", panscan)?;
        engine.set_double("video-zoom", session.zoom)?;
        // Older libmpv builds (Ubuntu 24.04's 0.37) have no video-scale-x/y.
        drop(engine.set_double("video-scale-x", session.scale_x));
        drop(engine.set_double("video-scale-y", session.scale_y));
        engine.set_double("video-pan-x", session.pan_x)?;
        engine.set_double("video-pan-y", session.pan_y)
    }

    fn zoom(&mut self, engine: &Engine, zoom: Zoom) -> UiRequest {
        let session = &mut self.session;
        match zoom {
            Zoom::In => session.zoom += ZOOM_STEP,
            Zoom::Out => session.zoom -= ZOOM_STEP,
            Zoom::Wider => session.scale_x += STRETCH_STEP,
            Zoom::Narrower => session.scale_x -= STRETCH_STEP,
            Zoom::Taller => session.scale_y += STRETCH_STEP,
            Zoom::Shorter => session.scale_y -= STRETCH_STEP,
            Zoom::Left => session.pan_x -= PAN_STEP,
            Zoom::Right => session.pan_x += PAN_STEP,
            Zoom::Up => session.pan_y -= PAN_STEP,
            Zoom::Down => session.pan_y += PAN_STEP,
            Zoom::ResetSize => {
                session.zoom = 0.0;
                session.scale_x = 1.0;
                session.scale_y = 1.0;
            }
            Zoom::ResetPosition => {
                session.pan_x = 0.0;
                session.pan_y = 0.0;
            }
            Zoom::Reset => {
                *session = Session {
                    zoom: 0.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    pan_x: 0.0,
                    pan_y: 0.0,
                    ..session.clone()
                }
            }
        }
        session.zoom = session.zoom.clamp(-3.0, 3.0);
        session.scale_x = session.scale_x.clamp(0.25, 4.0);
        session.scale_y = session.scale_y.clamp(0.25, 4.0);
        session.pan_x = session.pan_x.clamp(-3.0, 3.0);
        session.pan_y = session.pan_y.clamp(-3.0, 3.0);
        drop(self.apply_picture(engine, false));
        let session = &self.session;
        UiRequest::Osd(format!(
            "Zoom: {:.0} % ({:.0} x {:.0} %), position {:+.0} %, {:+.0} %",
            2f64.powf(session.zoom) * 100.0,
            session.scale_x * 100.0,
            session.scale_y * 100.0,
            session.pan_x * 100.0,
            session.pan_y * 100.0
        ))
    }

    /// The folder saved frames go to.
    #[must_use]
    pub fn image_folder(&self) -> PathBuf {
        self.settings.image_folder.clone().unwrap_or_else(|| {
            let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map_or_else(std::env::temp_dir, PathBuf::from);
            home.join("Pictures").join("RVP")
        })
    }

    /// Sets the folder for saved frames (the folder dialog's answer).
    pub fn set_image_folder(&mut self, folder: &Path) {
        self.settings.image_folder = Some(folder.to_path_buf());
        self.save_settings();
    }

    fn save_frame(&self, subtitles: bool) -> Option<UiRequest> {
        self.frame_path()
            .map(|path| UiRequest::SaveFrame { path, subtitles })
    }

    /// A new file in the image folder for the current frame, named after
    /// the video and the time; `None` without a video.
    fn frame_path(&self) -> Option<PathBuf> {
        let path = self.model.loaded_path()?;
        self.model.int(PropertyId::VideoWidth)?;
        let stem = Path::new(crate::view::file_name(path))
            .file_stem()
            .map_or_else(
                || "frame".to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            );
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        // Milliseconds of a media time are far inside u64.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let millis = (time.max(0.0) * 1000.0).round() as u64;
        let extension = match self.settings.image_format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
        };
        let folder = self.image_folder();
        let name = format!(
            "{stem} {:02}.{:02}.{:02}.{:03}",
            millis / 3_600_000,
            millis / 60_000 % 60,
            millis / 1000 % 60,
            millis % 1000
        );
        let mut file = folder.join(format!("{name}.{extension}"));
        let mut counter = 2;
        while file.exists() {
            file = folder.join(format!("{name} ({counter}).{extension}"));
            counter += 1;
        }
        Some(file)
    }

    fn set_on_top(&mut self, mode: OnTop) -> UiRequest {
        self.settings.on_top = mode;
        self.save_settings();
        self.session.on_top = None;
        let text = match mode {
            OnTop::Never => "Stay on top: never",
            OnTop::Always => "Stay on top: always",
            OnTop::WhilePlaying => "Stay on top: while playing",
        };
        // The window applies the state with the next update.
        UiRequest::Osd(text.to_owned())
    }

    /// Whether the window should stay on top now, when that changed since
    /// the last call.
    pub fn on_top_change(&mut self) -> Option<bool> {
        let wanted = match self.settings.on_top {
            OnTop::Never => false,
            OnTop::Always => true,
            OnTop::WhilePlaying => self.model.playing(),
        };
        if self.session.on_top == Some(wanted) {
            return None;
        }
        self.session.on_top = Some(wanted);
        Some(wanted)
    }
}
