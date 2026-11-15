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

use rvp_engine::{Engine, PropertyId, Seek, Track, TrackKind};

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
    /// "Audio Stretching": the pitch in percent above (or below) 100, at
    /// the same speed.
    pub audio_stretching: i32,
    /// The audio recording under way.
    pub recording: Option<std::path::PathBuf>,
    /// How many videos of this run got their window size (M12's "only
    /// the first playback").
    pub sized_videos: u32,
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
            audio_stretching: 0,
            recording: None,
            sized_videos: 0,
            last_volume_key: None,
            volume_acceleration: 1.0,
        }
    }
}

const ZOOM_STEP: f64 = 0.1;
const STRETCH_STEP: f64 = 0.05;
const PAN_STEP: f64 = 0.01;
const SUBTITLE_SCALE_STEP: f64 = 0.1;

/// `+1.5 s`, `+1.25 s`, `-50 ms`: to the millisecond, with one decimal at
/// least.
pub(crate) fn signed_seconds(seconds: f64) -> String {
    if seconds.abs() < 1.0 && seconds != 0.0 {
        format!("{:+.0} ms", seconds * 1000.0)
    } else {
        let text = format!("{seconds:+.3}");
        let text = text.trim_end_matches('0');
        if text.ends_with('.') {
            format!("{text}0 s")
        } else {
            format!("{text} s")
        }
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
        Aspect::Custom => "Custom",
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
            Action::AbRepeat(change) => self.ab_repeat(engine, change),
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
                    AfterPlayback::ExitAfterList => AfterPlayback::PowerAfterFile,
                    AfterPlayback::PowerAfterFile => AfterPlayback::PowerAfterList,
                    AfterPlayback::PowerAfterList | AfterPlayback::Stay => AfterPlayback::Nothing,
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
            Action::SubtitleDelay(delay) => self.subtitle_delay(engine, delay),
            Action::Subtitle(style) => self.subtitle_style(engine, style),
            Action::Subtitles(command) => self.subtitle_command(engine, command),
            Action::Speech(command) => self.speech_command(engine, command),
            Action::Misc(command) => self.misc_command(command),
            Action::Skins(command) => Some(self.skin_command(command)),
            Action::Preferences => Some(UiRequest::Preferences(None)),
            Action::KeysAndMouse => Some(UiRequest::Preferences(Some("input"))),
            Action::Settings(page) => Some(UiRequest::Preferences(Some(page.key()))),
            Action::About => Some(UiRequest::About),
            Action::ControlPanel => {
                self.session.control_panel = !self.session.control_panel;
                Some(UiRequest::ControlPanel(self.session.control_panel))
            }
            Action::ToggleEqualizer => Some(self.toggle_equalizer(engine)),
            Action::SubtitleFont => Some(UiRequest::SubtitleFontDialog(
                self.settings.subtitle_font.clone(),
            )),
            other => self
                .m11_command(engine, other)
                .unwrap_or_else(|| self.view_command(engine, other)),
        }
    }

    /// M11's commands: `None` when the action is not one of them, else
    /// what it asks of the window.
    #[allow(clippy::option_option)] // Not M11's / M11's and what it asks.
    fn m11_command(&mut self, engine: &Engine, action: Action) -> Option<Option<UiRequest>> {
        let request = match action {
            Action::JumpDialog => self.model.media_loaded().then(|| UiRequest::JumpDialog {
                time: self.model.double(PropertyId::TimePos).unwrap_or(0.0),
                duration: self.model.double(PropertyId::Duration).unwrap_or(0.0),
                fps: self.model.double(PropertyId::ContainerFps).unwrap_or(0.0),
            }),
            Action::Scenes(kind) => self.open_scenes(engine, kind),
            Action::Skip(skip) => Some(self.skip_command(skip)),
            Action::Bookmark(command) => self.bookmark_command(engine, command),
            Action::ToggleShuffle
            | Action::ToggleRememberPositions
            | Action::ToggleHidePointer
            | Action::ToggleCloseAtEnd
            | Action::ToggleRecyclePlayed
            | Action::ToggleSeekTooltip
            | Action::ToggleSeekPreview
            | Action::ToggleSeekMarkers
            | Action::ToggleKeepSpeed
            | Action::ToggleRememberAfter => self.playback_toggle(action),
            Action::SetPowerAction(power) => Some(self.set_power_action(power)),
            Action::PowerTimer(kind) => Some(self.power_timer_command(kind)),
            Action::CopyName(what) => self.copy_name(what),
            Action::ToggleRecent => {
                let open = &mut self.settings.open;
                open.recent = !open.recent;
                let on = open.recent;
                if !on {
                    self.marks_mut().clear_recent();
                }
                Some(self.toggled("Store and show recent items", on))
            }
            Action::ClearRecent => {
                self.marks_mut().clear_recent();
                Some(UiRequest::Osd("Recent items cleared".to_owned()))
            }
            Action::SetWhenOpened(rule) => {
                self.settings.open.when_opened = rule;
                self.save_settings();
                Some(UiRequest::Osd(
                    match rule {
                        crate::action::WhenOpened::Selected => "When files open: only them",
                        crate::action::WhenOpened::Similar => "When files open: similar files too",
                        crate::action::WhenOpened::All => "When files open: the whole folder",
                    }
                    .to_owned(),
                ))
            }
            Action::SetDefaultOpen(choice) => {
                self.settings.open.default_open = choice;
                self.save_settings();
                None
            }
            Action::Open(source) => self.open_source(engine, source),
            Action::LoadExternalAudio => Some(UiRequest::OpenAudioDialog),
            Action::ReloadSubtitle => self.reload_subtitle(engine),
            Action::AddFavoriteItem => self.add_favorite(false),
            Action::AddFavoriteFolder => self.add_favorite(true),
            Action::EditFavorites => Some(UiRequest::Favorites),
            Action::ChapterMenu => {
                if self.settings.playback.bookmarks.thumbnails {
                    self.open_scenes(engine, crate::action::Scenes::Markers)
                } else {
                    Some(UiRequest::PopupMenu("CMD_POPUPMENU_PLAY_BOOKMARK"))
                }
            }
            Action::DeleteFiles(related) => {
                let action = if related {
                    crate::action::ListAction::DeleteRelated
                } else {
                    crate::action::ListAction::DeleteFiles
                };
                self.list_action(engine, action, false, 10)
            }
            Action::DiscMenu(menu) => Some(if self.disc_playing.is_some() {
                UiRequest::DiscMenu(menu)
            } else {
                UiRequest::Osd("DVD menus: no DVD plays with its menus".to_owned())
            }),
            Action::DvdSpeed(speed) => Some(UiRequest::Osd(match self.dvd_speed(engine, speed) {
                Ok(true) => format!(
                    "Speed: {}x {}",
                    speed.unsigned_abs(),
                    if speed < 0 { "backward" } else { "forward" }
                ),
                Ok(false) => "Backward play: not on discs".to_owned(),
                Err(error) => format!("Speed: {error}"),
            })),
            _ => return None,
        };
        Some(request)
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
                    Aspect::R235x1 => Aspect::Custom,
                    Aspect::Custom => Aspect::Default,
                };
                Some(self.set_aspect(engine, next))
            }
            Action::SetAspect(aspect) => Some(self.set_aspect(engine, aspect)),
            Action::VideoFit(fit) => {
                self.session.fit = fit;
                self.remember_aspect();
                drop(self.apply_picture(engine));
                Some(UiRequest::Osd(crate::picture::fit_text(fit).to_owned()))
            }
            Action::Zoom(zoom) => Some(self.zoom(engine, zoom)),
            Action::Picture(command) => self.picture_command(engine, command),
            Action::Stereo(command) => Some(self.stereo_command(engine, command)),
            Action::Window(command) => Some(self.window_command(command)),
            Action::SaveFrame => self.save_shot(crate::capture::Shot::Source),
            Action::SaveScreen => self.save_shot(crate::capture::Shot::Screen),
            Action::SaveFrameAs => self.save_as(crate::capture::Shot::Source),
            Action::Capture(command) => self.capture_command(command),
            Action::Record(command) => self.recording_command(engine, command),
            Action::Visual(command) => self.visual_command(engine, command),
            Action::Render(toggle) => Some(self.render_toggle(engine, toggle)),
            Action::Popup(popup) => Some(UiRequest::PopupMenu(popup.command())),
            Action::SetImageFormat(format) => {
                self.settings.image_format = format;
                self.save_settings();
                Some(UiRequest::Osd(
                    match format {
                        ImageFormat::Png => "Images: PNG",
                        ImageFormat::Jpeg => "Images: JPEG",
                        ImageFormat::Bmp => "Images: BMP",
                        ImageFormat::Webp => "Images: WEBP",
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
                    OnTop::WhilePlaying => OnTop::FullscreenPlaying,
                    OnTop::FullscreenPlaying => OnTop::Never,
                };
                Some(self.set_on_top(next))
            }
            Action::SetOnTop(mode) => Some(self.set_on_top(mode)),
            Action::WindowScale(scale) => Some(UiRequest::ScaleWindow(scale.factor())),
            Action::WindowFitAspect => Some(UiRequest::FitAspect),
            Action::Osd(command) => self.osd_command(engine, command),
            Action::SystemInfo => Some(UiRequest::SystemInfo),
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
            Jump::PreviousKeyframe => self.keyframe_step(engine, false),
            Jump::NextKeyframe => self.keyframe_step(engine, true),
            Jump::CurrentSubtitle => engine.command("sub-seek", &["0"]),
        };
        drop(result);
        None
    }

    fn speed(&mut self, engine: &Engine, change: Speed) -> Option<UiRequest> {
        let current = self.model.double(PropertyId::Speed).unwrap_or(1.0);
        let next = match change {
            Speed::Faster => current + self.settings.playback.speed_step,
            Speed::Slower => current - self.settings.playback.speed_step,
            Speed::Default if (current - 1.0).abs() > 1e-6 => {
                self.session.last_speed = current;
                1.0
            }
            Speed::Default => self.session.last_speed,
        };
        let next = ((next * 100.0).round() / 100.0).clamp(0.01, 16.0);
        // With the normalizer on, the tempo filter after it takes the speed
        // (`crate::normalizer`): no seek is needed.
        engine.set_double("speed", next).ok()?;
        Some(UiRequest::Osd(if (next * 10.0).fract().abs() < 1e-9 {
            format!("Speed: {next:.1}x")
        } else {
            format!("Speed: {next:.2}x")
        }))
    }

    fn ab_repeat(&mut self, engine: &Engine, change: AbRepeat) -> Option<UiRequest> {
        let time = self.model.double(PropertyId::TimePos);
        let session = &mut self.session;
        match change {
            AbRepeat::SetA
            | AbRepeat::SetB
            | AbRepeat::ClearA
            | AbRepeat::ClearB
            | AbRepeat::Clear
            | AbRepeat::Toggle => {}
            other => return self.ab_extra(engine, other),
        }
        match change {
            AbRepeat::SetA => session.point_a = time,
            AbRepeat::SetB => session.point_b = time,
            AbRepeat::ClearA => session.point_a = None,
            AbRepeat::ClearB => session.point_b = None,
            AbRepeat::Clear => {
                session.point_a = None;
                session.point_b = None;
                session.ab_active = false;
            }
            AbRepeat::Toggle => session.ab_active = !session.ab_active,
            _ => {}
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
        Some(self.ab_notice())
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
        // Choosing the chosen one again turns it off, as in PotPlayer.
        let after = if self.session.after == after && after != AfterPlayback::Nothing {
            AfterPlayback::Nothing
        } else {
            after
        };
        self.session.after = after;
        self.remember_after();
        let action = crate::power::action_text(self.settings.playback.finished.action);
        UiRequest::Osd(match after {
            AfterPlayback::Nothing => "When playback ends: do nothing".to_owned(),
            AfterPlayback::ExitAfterFile => "When playback ends: exit".to_owned(),
            AfterPlayback::ExitAfterList => "When the playlist ends: exit".to_owned(),
            AfterPlayback::PowerAfterFile => format!("When playback ends: {action}"),
            AfterPlayback::PowerAfterList => format!("When the playlist ends: {action}"),
            AfterPlayback::Stay => "When playback ends: stay".to_owned(),
        })
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
        // The second subtitle is selected too: the first is the one shown.
        let current = choices
            .iter()
            .position(|choice| {
                choice.is_some_and(|track| track.selected && track.main_selection != Some(1))
            })
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
        if kind == TrackKind::Subtitle {
            self.subtitle_tracks_chosen(engine);
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
        if kind == TrackKind::Subtitle {
            self.subtitle_tracks_chosen(engine);
        }
    }

    /// The first sync step (< and >): the whole track, or with "Apply sync
    /// changes starting from next position" the subtitles after now.
    fn subtitle_delay(&mut self, engine: &Engine, delay: Delay) -> Option<UiRequest> {
        let step = self.settings.subtitle_delay_step;
        let signed = match delay {
            Delay::Earlier => Some(-step),
            Delay::Later => Some(step),
            Delay::Reset => None,
        };
        if let Some(request) = signed.and_then(|signed| self.sync_from_next(engine, signed)) {
            return Some(request);
        }
        self.delay(engine, "sub-delay", PropertyId::SubDelay, delay, step)
            .map(|value| UiRequest::Osd(format!("Subtitle sync: {}", signed_seconds(value))))
    }

    pub(crate) fn delay(
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

    /// Size, position and alignment, kept in the settings as PotPlayer
    /// keeps them.
    fn subtitle_style(&mut self, engine: &Engine, style: SubtitleStyle) -> Option<UiRequest> {
        let text = match style {
            SubtitleStyle::Bigger | SubtitleStyle::Smaller => {
                let step = if style == SubtitleStyle::Bigger {
                    SUBTITLE_SCALE_STEP
                } else {
                    -SUBTITLE_SCALE_STEP
                };
                let next = ((self.settings.subtitle_scale + step) * 10.0).round() / 10.0;
                let next = next.clamp(0.3, 3.0);
                engine.set_double("sub-scale", next).ok()?;
                self.settings.subtitle_scale = next;
                format!("Subtitle size: {:.0} %", next * 100.0)
            }
            SubtitleStyle::Up | SubtitleStyle::Down | SubtitleStyle::DefaultPosition => {
                let position = self.settings.subtitles.position.unwrap_or(100.0);
                let next = match style {
                    SubtitleStyle::Up => Some(position - 1.0),
                    SubtitleStyle::Down => Some(position + 1.0),
                    _ => None,
                }
                .map(|next| next.clamp(0.0, 150.0));
                self.settings.subtitles.position = next;
                format!("Subtitle position: {:.0} %", next.unwrap_or(100.0))
            }
            SubtitleStyle::AlignLeft | SubtitleStyle::AlignCenter | SubtitleStyle::AlignRight => {
                let align = match style {
                    SubtitleStyle::AlignLeft => "left",
                    SubtitleStyle::AlignRight => "right",
                    _ => "center",
                };
                align.clone_into(&mut self.settings.subtitles.align);
                format!("Subtitles: align {align}")
            }
        };
        self.save_settings();
        self.apply_subtitles(engine);
        Some(UiRequest::Osd(text))
    }

    fn set_aspect(&mut self, engine: &Engine, aspect: Aspect) -> UiRequest {
        self.session.aspect = aspect;
        self.remember_aspect();
        drop(self.apply_picture(engine));
        let text = if aspect == Aspect::Custom {
            self.settings.picture.custom_aspect.clone()
        } else {
            aspect_text(aspect).to_owned()
        };
        UiRequest::Osd(format!("Aspect ratio: {text}"))
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
        self.picture.preset = None;
        drop(self.apply_picture(engine));
        self.remember_frame();
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
            crate::storage::user_folder("XDG_PICTURES_DIR", "Pictures").join("RVP")
        })
    }

    /// Sets the folder for saved frames (the folder dialog's answer).
    pub fn set_image_folder(&mut self, folder: &Path) {
        self.settings.image_folder = Some(folder.to_path_buf());
        self.save_settings();
    }

    fn set_on_top(&mut self, mode: OnTop) -> UiRequest {
        self.settings.on_top = mode;
        self.save_settings();
        self.session.on_top = None;
        let text = match mode {
            OnTop::Never => "Stay on top: never",
            OnTop::Always => "Stay on top: always",
            OnTop::WhilePlaying => "Stay on top: while playing",
            OnTop::FullscreenPlaying => "Stay on top: while playing in fullscreen",
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
            OnTop::FullscreenPlaying => self.model.playing() && self.picture.fullscreen,
        };
        if self.session.on_top == Some(wanted) {
            return None;
        }
        self.session.on_top = Some(wanted);
        Some(wanted)
    }
}
