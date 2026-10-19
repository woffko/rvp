//! Input binding tests (docs/PLAN.md section 7): every default binding
//! triggers its action, checked against the real engine (headless).

use std::time::{Duration, Instant};

use rvp_app::audio::commands::AudioCommand;
use rvp_app::capture::{CaptureCommand, Shot};
use rvp_app::hlsl::Stage;
use rvp_app::picture::PictureCommand;
use rvp_app::playlist::local_path;
use rvp_app::record::RecordCommand;
use rvp_app::shaders::ShaderCommand;
use rvp_app::smoothing::SmoothingCommand;
use rvp_app::video::{ColorChange, ColorKind, Rotation, VideoEffect};
use rvp_app::volume::mpv_volume;
use rvp_app::window::{WindowCommand, WindowRequest};
use rvp_app::{
    AbRepeat, Action, App, Bindings, BookmarkCommand, Delay, Fit, Jump, Key, KeyChord,
    MouseGesture, Scenes, SeekStep, Settings, Skip, Source, Speed, SubtitleStyle, UiRequest, Zoom,
};
use rvp_engine::{Engine, EngineOptions, PlayerModel, PropertyId};

const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);
const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);

fn chord(text: &str) -> KeyChord {
    KeyChord::parse(text).expect("valid key")
}

#[test]
fn potplayer_default_keys() {
    let bindings = Bindings::potplayer_defaults();
    for (key, action) in [
        ("Space", Action::PlayPause),
        ("Enter", Action::ToggleFullscreen),
        ("Alt+Enter", Action::ToggleFullscreen),
        ("Ctrl+Enter", Action::ToggleFullscreenStretch),
        ("Up", Action::VolumeUp),
        ("Down", Action::VolumeDown),
        ("M", Action::ToggleMute),
        ("Left", Action::SeekBackward(SeekStep::Short)),
        ("Right", Action::SeekForward(SeekStep::Short)),
        ("Ctrl+Left", Action::SeekBackward(SeekStep::Medium)),
        ("Shift+Right", Action::SeekForward(SeekStep::Long)),
        ("Ctrl+Alt+Left", Action::SeekBackward(SeekStep::VeryLong)),
        ("PgUp", Action::Previous),
        ("PgDn", Action::Next),
        ("F3", Action::OpenDefault),
        ("Ctrl+O", Action::OpenFiles),
        ("G", Action::JumpDialog),
        ("P", Action::Bookmark(BookmarkCommand::Add)),
        ("Ctrl+\\", Action::AbRepeat(AbRepeat::CurrentSubtitle)),
        ("Ins", Action::AbRepeat(AbRepeat::CurrentSubtitle)),
        ("Ctrl+Shift+Right", Action::JumpTo(Jump::NextKeyframe)),
        ("Shift+Z", Action::Scenes(Scenes::Time)),
        ("F4", Action::CloseMedia),
        ("F6", Action::TogglePlaylist),
        ("Alt+H", Action::ToggleSubtitles),
        ("F7", Action::ControlPanel),
        ("Shift+E", Action::ToggleEqualizer),
        ("Q", Action::Color(ColorChange::Toggle)),
        ("W", Action::Color(ColorChange::Down(ColorKind::Brightness))),
        ("E", Action::Color(ColorChange::Up(ColorKind::Brightness))),
        ("R", Action::Color(ColorChange::Down(ColorKind::Contrast))),
        ("T", Action::Color(ColorChange::Up(ColorKind::Contrast))),
        ("Y", Action::Color(ColorChange::Down(ColorKind::Saturation))),
        ("U", Action::Color(ColorChange::Up(ColorKind::Saturation))),
        ("I", Action::Color(ColorChange::Down(ColorKind::Hue))),
        ("O", Action::Color(ColorChange::Up(ColorKind::Hue))),
        ("Ctrl+I", Action::DeinterlaceCycle),
        ("Ctrl+B", Action::VideoEffect(VideoEffect::Soften)),
        ("Ctrl+R", Action::VideoEffect(VideoEffect::Sharpen)),
        ("Ctrl+H", Action::VideoEffect(VideoEffect::Deblock)),
        ("Ctrl+N", Action::VideoEffect(VideoEffect::Denoise)),
        ("Ctrl+P", Action::VideoEffect(VideoEffect::FlipVertical)),
        ("Ctrl+Z", Action::VideoEffect(VideoEffect::FlipHorizontal)),
        ("Alt+K", Action::Rotate(Rotation::Cycle)),
        (
            "Shift+P",
            Action::Shaders(ShaderCommand::Toggle(Stage::Pre)),
        ),
    ] {
        assert_eq!(bindings.key(&chord(key)), Some(action), "{key}");
    }
    // Alt+F4 is not an accelerator: the window system closes the window.
    assert_eq!(bindings.key(&chord("Alt+F4")), None);
    assert_eq!(
        bindings.key_for(Action::PlayPause),
        Some(KeyChord::plain(Key::Space))
    );
    assert_eq!(
        bindings.mouse(MouseGesture::LeftClick),
        Some(Action::PlayPause)
    );
    assert_eq!(
        bindings.mouse(MouseGesture::LeftDoubleClick),
        Some(Action::ToggleFullscreen)
    );
    assert_eq!(
        bindings.mouse(MouseGesture::WheelUp),
        Some(Action::VolumeUp)
    );
    assert_eq!(
        bindings.mouse(MouseGesture::X1Click),
        Some(Action::SeekBackward(SeekStep::Short))
    );
    assert!(bindings.left_click_video_only);
}

/// Whether `path` plays (mpv reports it in [`local_path`]'s form).
fn is_file(model: &PlayerModel, path: &str) -> bool {
    model.loaded_path() == Some(local_path(path).as_str())
}

/// mpv's volume is the one PotPlayer's `percent` gives.
fn shown_volume(model: &PlayerModel, percent: f64) -> bool {
    model
        .double(PropertyId::Volume)
        .is_some_and(|volume| (volume - mpv_volume(percent)).abs() < 1e-6)
}

fn audio_filters(model: &PlayerModel) -> &str {
    model.text(PropertyId::AudioFilters).unwrap_or_default()
}

struct Harness {
    app: App,
    engine: Engine,
}

impl Harness {
    fn new() -> Self {
        let dir =
            std::env::temp_dir().join(format!("rvp-bindings-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ..Settings::default()
        };
        // Recordings and saved frames stay in the test's folder.
        settings.record.stream_folder = Some(dir.join("streams"));
        settings.record.video_folder = Some(dir.join("videos"));
        settings.image_folder = Some(dir.join("images"));
        Self {
            app: App::new(settings, &dir).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
        }
    }

    fn wait(&mut self, what: &str, done: impl Fn(&PlayerModel) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done(&self.app.model) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: loaded {:?}, path {:?}, time {:?}, pause {:?}",
                self.app.model.loaded_path(),
                self.app.model.text(PropertyId::Path),
                self.app.model.double(PropertyId::TimePos),
                self.app.model.flag(PropertyId::Pause)
            );
            std::thread::sleep(Duration::from_millis(10));
            self.app.drain(&self.engine, Instant::now());
        }
    }

    fn time(&self) -> f64 {
        self.app
            .model
            .double(PropertyId::TimePos)
            .unwrap_or_default()
    }

    fn dispatch(&mut self, action: Action) -> Option<UiRequest> {
        self.app.dispatch(&self.engine, action)
    }

    /// Carries out `action` once and waits for its expected effect. The
    /// player starts and ends paused at 30 s of the long fixture, at 50 %.
    fn check(&mut self, action: Action) {
        match action {
            Action::PlayPause => {
                assert_eq!(self.dispatch(action), None);
                self.wait("playing", PlayerModel::playing);
                self.dispatch(action);
                self.wait("paused", |m| m.flag(PropertyId::Pause) == Some(true));
            }
            Action::SeekBackward(step) | Action::SeekForward(step) => {
                // Only the short step fits the 60 s fixture both ways.
                if step == SeekStep::Short {
                    let before = self.time();
                    self.dispatch(action);
                    let expected = if matches!(action, Action::SeekForward(_)) {
                        before + 5.0
                    } else {
                        before - 5.0
                    };
                    self.wait("seek step", |m| {
                        (m.double(PropertyId::TimePos).unwrap_or_default() - expected).abs() < 0.3
                    });
                } else {
                    let before = self.time();
                    self.dispatch(action);
                    self.wait("long seek", |m| {
                        (m.double(PropertyId::TimePos).unwrap_or_default() - before).abs() > 4.0
                    });
                    self.back_to_30();
                }
            }
            // The volume steps in PotPlayer's percent (rvp_app::volume).
            Action::VolumeUp => {
                self.dispatch(action);
                self.wait("volume up", |m| shown_volume(m, 51.0));
            }
            Action::VolumeDown => {
                self.dispatch(action);
                self.wait("volume down", |m| shown_volume(m, 50.0));
            }
            Action::ToggleMute => {
                self.dispatch(action);
                self.wait("muted", |m| m.flag(PropertyId::Mute) == Some(true));
                self.dispatch(action);
                self.wait("unmuted", |m| m.flag(PropertyId::Mute) == Some(false));
            }
            // RVP's album is the playlist; mpv plays one entry at a time.
            Action::Next => {
                self.dispatch(action);
                self.wait("next file", |m| is_file(m, SHORT));
            }
            Action::Previous => {
                self.dispatch(Action::Next);
                self.wait("next file", |m| is_file(m, SHORT));
                self.dispatch(action);
                self.wait("previous file", |m| is_file(m, LONG));
            }
            Action::ToggleSubtitles => {
                let before = self.app.model.flag(PropertyId::SubVisibility);
                self.dispatch(action);
                self.wait("subtitles toggled", |m| {
                    m.flag(PropertyId::SubVisibility) == before.map(|on| !on)
                });
                self.dispatch(action);
                self.wait("subtitles back", |m| {
                    m.flag(PropertyId::SubVisibility) == before
                });
            }
            Action::CloseMedia => {}
            _ => self.check_request(action),
        }
    }

    /// Actions that ask the window for something or change the normalizer.
    fn check_request(&mut self, action: Action) {
        match action {
            Action::OpenFiles => assert_eq!(self.dispatch(action), Some(UiRequest::OpenFileDialog)),
            Action::Exit => assert_eq!(self.dispatch(action), Some(UiRequest::Exit)),
            Action::ToggleFullscreen => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ToggleFullscreen { fit: None })
                );
            }
            Action::ToggleFullscreenStretch => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ToggleFullscreen {
                        fit: Some(Fit::Stretch)
                    })
                );
            }
            Action::ContextMenu => assert_eq!(self.dispatch(action), Some(UiRequest::ContextMenu)),
            Action::Minimize => assert_eq!(self.dispatch(action), Some(UiRequest::Minimize)),
            Action::MaximizeRestore => {
                assert_eq!(self.dispatch(action), Some(UiRequest::MaximizeRestore));
            }
            Action::TogglePlaylist => {
                assert_eq!(self.dispatch(action), Some(UiRequest::TogglePlaylist));
            }
            Action::ToggleNormalizer => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Osd("Normalizer: Off, Boost: On".into()))
                );
                self.wait("normalizer off", |m| !audio_filters(m).contains("rvpnorm"));
                self.dispatch(action);
                self.wait("normalizer on", |m| audio_filters(m).contains("rvpnorm"));
            }
            Action::ToggleBoost => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Osd("Normalizer: On, Boost: Off".into()))
                );
                // Only the gain control changes; the chain stays.
                assert!(audio_filters(&self.app.model).contains("rvpnorm"));
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Osd("Normalizer: On, Boost: On".into()))
                );
            }
            Action::ImportPlaylists => {
                assert_eq!(self.dispatch(action), Some(UiRequest::ImportPlaylists));
            }
            Action::OpenFolder => {
                assert_eq!(self.dispatch(action), Some(UiRequest::OpenFolderDialog));
            }
            Action::OpenUrl => assert_eq!(self.dispatch(action), Some(UiRequest::OpenUrlDialog)),
            Action::OpenSubtitle => {
                assert_eq!(self.dispatch(action), Some(UiRequest::OpenSubtitleDialog));
            }
            Action::SubtitleFont => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::SubtitleFontDialog(String::new()))
                );
            }
            Action::Preferences => {
                assert_eq!(self.dispatch(action), Some(UiRequest::Preferences(None)));
            }
            Action::About => assert_eq!(self.dispatch(action), Some(UiRequest::About)),
            Action::WindowScale(scale) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ScaleWindow(scale.factor()))
                );
            }
            Action::WindowFitAspect => {
                assert_eq!(self.dispatch(action), Some(UiRequest::FitAspect));
            }
            Action::PlaybackInfo => {
                assert_eq!(self.dispatch(action), Some(UiRequest::ShowInfo(true)));
                assert_eq!(self.dispatch(action), Some(UiRequest::ShowInfo(false)));
            }
            Action::SaveFrame | Action::SaveScreen => match self.dispatch(action) {
                Some(UiRequest::Capture(job)) => {
                    let mode = if action == Action::SaveScreen {
                        "window"
                    } else {
                        "video"
                    };
                    assert_eq!(job.mode, mode);
                    let path = job.path.unwrap_or_default();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    assert!(name.starts_with("fixture-60s 00.00.3"), "{name}");
                }
                other => panic!("{action:?}: {other:?}"),
            },
            _ => self.check_command(action),
        }
    }

    /// Asserts that `action` shows `text` on screen.
    fn osd(&mut self, action: Action, text: &str) {
        assert_eq!(
            self.dispatch(action),
            Some(UiRequest::Osd(text.to_owned())),
            "{action:?}"
        );
    }

    /// Asserts that `action` shows a notice starting with `start`.
    fn osd_starts(&mut self, action: Action, start: &str) {
        match self.dispatch(action) {
            Some(UiRequest::Osd(text)) => assert!(text.starts_with(start), "{action:?}: {text}"),
            other => panic!("{action:?}: {other:?}"),
        }
    }

    fn wait_double(&mut self, what: &str, property: PropertyId, value: f64) {
        self.wait(what, |m| {
            m.double(property)
                .is_some_and(|now| (now - value).abs() < 1e-6)
        });
    }

    /// Seeks back to 30 s, paused.
    fn back_to_30(&mut self) {
        self.engine.set_pause(true).expect("pause");
        self.engine
            .seek(rvp_engine::Seek::Absolute(30.0))
            .expect("seek back");
        // A frame step sent while mpv still restarts after the seek is lost.
        self.wait("back at 30 s", |m| {
            m.flag(PropertyId::Pause) == Some(true)
                && m.flag(PropertyId::Seeking) == Some(false)
                && (m.double(PropertyId::TimePos).unwrap_or_default() - 30.0).abs() < 0.05
        });
    }

    /// M4's commands: frames, jumps, speed, A-B, sync, subtitles, tracks,
    /// aspect, zoom and window state.
    fn check_command(&mut self, action: Action) {
        match action {
            Action::Reopen => {
                assert_eq!(self.dispatch(action), None);
                // Positions are off in this harness: the file starts again.
                self.wait("reopened", |m| {
                    m.media_loaded() && m.double(PropertyId::TimePos).unwrap_or(99.0) < 5.0
                });
                self.back_to_30();
            }
            Action::FrameForward => {
                assert_eq!(self.dispatch(action), None);
                self.wait("next frame", |m| {
                    m.double(PropertyId::TimePos).unwrap_or_default() > 30.05
                });
                self.back_to_30();
            }
            Action::FrameBack => {
                assert_eq!(self.dispatch(action), None);
                self.wait("previous frame", |m| {
                    m.double(PropertyId::TimePos).unwrap_or(30.0) < 29.95
                });
                self.back_to_30();
            }
            Action::JumpTo(Jump::Start) => {
                assert_eq!(self.dispatch(action), None);
                self.wait("at the start", |m| {
                    m.double(PropertyId::TimePos).unwrap_or(99.0) < 0.2
                });
                self.back_to_30();
            }
            // The 60 s fixture's middle and 30 s before its end are both 30 s.
            Action::JumpTo(Jump::Middle | Jump::BeforeEnd) => {
                self.engine
                    .seek(rvp_engine::Seek::Absolute(10.0))
                    .expect("seek");
                self.wait("at 10 s", |m| {
                    (m.double(PropertyId::TimePos).unwrap_or_default() - 10.0).abs() < 0.05
                });
                assert_eq!(self.dispatch(action), None);
                self.back_to_30();
            }
            // The fixtures have no subtitles: nothing happens.
            Action::JumpTo(
                Jump::PreviousSubtitle
                | Jump::NextSubtitle
                | Jump::CurrentSubtitle
                | Jump::PreviousChapter
                | Jump::NextChapter,
            ) => assert_eq!(self.dispatch(action), None),
            Action::Speed(Speed::Default) => {
                self.osd(action, "Speed: 1.0x");
                self.wait_double("1.0x", PropertyId::Speed, 1.0);
            }
            Action::Speed(change) => {
                let (text, value) = if change == Speed::Faster {
                    ("Speed: 1.1x", 1.1)
                } else {
                    ("Speed: 0.9x", 0.9)
                };
                self.osd(action, text);
                self.wait_double(text, PropertyId::Speed, value);
                // Default goes back to 1.0x.
                self.osd(Action::Speed(Speed::Default), "Speed: 1.0x");
                self.wait_double("1.0x", PropertyId::Speed, 1.0);
            }
            Action::AbRepeat(_) | Action::AfterPlaybackCycle => self.check_repeat(action),
            Action::OnTopCycle => {
                self.osd(action, "Stay on top: always");
                self.osd(action, "Stay on top: while playing");
                self.osd(action, "Stay on top: while playing in fullscreen");
                self.osd(action, "Stay on top: never");
            }
            Action::AspectCycle => {
                for text in [
                    "4:3", "16:9", "16:10", "1.85:1", "2.35:1", "21:9", "Default",
                ] {
                    self.osd(action, &format!("Aspect ratio: {text}"));
                }
            }
            Action::AudioDelay(_) | Action::SubtitleDelay(_) | Action::Subtitle(_) => {
                self.check_sync_and_subtitles(action);
            }
            // The fixtures have one track each: cycling stays on it.
            Action::CycleAudio => self.osd_starts(action, "Audio: "),
            Action::CycleVideo => self.osd_starts(action, "Video: "),
            Action::CycleSubtitles => self.osd(action, "Subtitle: off"),
            Action::Zoom(_) => {
                self.osd_starts(action, "Zoom: ");
                self.osd(
                    Action::Zoom(Zoom::Reset),
                    "Zoom: 100 % (100 x 100 %), position +0 %, +0 %",
                );
            }
            Action::Audio(command) => self.check_audio(action, command),
            _ => self.check_video(action),
        }
    }

    /// M10's audio commands with default keys: the effects switch on and
    /// off in the chain, recording asks for a file, the volumes answer.
    fn check_audio(&mut self, action: Action, command: AudioCommand) {
        let label = match command {
            AudioCommand::SwapChannels => Some(("Swap Stereo Channels", "@rvpchannel:")),
            AudioCommand::Crystality => Some(("Crystality", "@rvpcrystality:")),
            AudioCommand::NoiseReduction => Some(("Noise Reduction", "@rvpdenoise:")),
            AudioCommand::Freeverb => Some(("Freeverb", "@rvpreverb:")),
            AudioCommand::VoiceRemoval | AudioCommand::VoiceEmphasis => Some((
                if command == AudioCommand::VoiceRemoval {
                    "Voice Removal"
                } else {
                    "Voice Emphasis"
                },
                "@rvpvoice:",
            )),
            _ => None,
        };
        if let Some((name, filter)) = label {
            self.osd(action, &format!("{name}: On"));
            self.wait(name, |m| audio_filters(m).contains(filter));
            self.osd(action, &format!("{name}: Off"));
            self.wait(name, |m| !audio_filters(m).contains(filter));
            return;
        }
        match command {
            AudioCommand::RecordDialog => match self.dispatch(action) {
                Some(UiRequest::RecordDialog(path)) => {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    assert!(name.starts_with("fixture-60s 00.00.3"), "{name}");
                }
                other => panic!("{action:?}: {other:?}"),
            },
            AudioCommand::MasterUp => {
                self.osd(action, "Master volume: +1 dB");
                self.osd(
                    Action::Audio(AudioCommand::MasterDown),
                    "Master volume: +0 dB",
                );
            }
            AudioCommand::MasterDown => {
                self.osd(action, "Master volume: -1 dB");
                self.osd(
                    Action::Audio(AudioCommand::MasterUp),
                    "Master volume: +0 dB",
                );
            }
            AudioCommand::SystemMasterUp
            | AudioCommand::SystemMasterDown
            | AudioCommand::SystemMasterMute => {
                assert!(
                    matches!(self.dispatch(action), Some(UiRequest::SystemVolume(_))),
                    "{action:?}"
                );
            }
            // Audio Stretching: rubberband in the chain, then out again.
            AudioCommand::StretchingDown | AudioCommand::StretchingUp => {
                let percent = if command == AudioCommand::StretchingUp {
                    101
                } else {
                    99
                };
                self.osd(action, &format!("Audio Stretching : {percent}%"));
                self.wait("Audio Stretching", |m| {
                    audio_filters(m).contains("@rvptempo:rubberband")
                });
                self.osd(
                    Action::Audio(AudioCommand::StretchingOff),
                    "Audio Stretching : 100%",
                );
                self.wait("Audio Stretching off", |m| {
                    !audio_filters(m).contains("@rvptempo:rubberband")
                });
            }
            AudioCommand::StretchingOff => self.osd(action, "Audio Stretching : 100%"),
            // Headless libmpv has no output stream to set: nothing, or the
            // volume.
            AudioCommand::SystemWaveUp
            | AudioCommand::SystemWaveDown
            | AudioCommand::SystemWaveMute => match self.dispatch(action) {
                None => {}
                Some(UiRequest::Osd(text)) => assert!(text.starts_with("Wave volume"), "{text}"),
                other => panic!("{action:?}: {other:?}"),
            },
            _ => panic!("{action:?} has no check"),
        }
    }

    /// M6's commands: the control panel, the equalizer, colours,
    /// deinterlacing, effects, rotation and shaders.
    fn check_video(&mut self, action: Action) {
        let vf = |h: &Self| h.engine.get_text("vf").unwrap_or_default();
        match action {
            Action::ControlPanel => {
                assert_eq!(self.dispatch(action), Some(UiRequest::ControlPanel(true)));
                assert_eq!(self.dispatch(action), Some(UiRequest::ControlPanel(false)));
            }
            Action::ToggleEqualizer => {
                self.osd(action, "Equalizer: (Default)");
                self.wait("equalizer on", |m| audio_filters(m).contains("@rvpeq:"));
                self.osd(action, "Equalizer: off");
                self.wait("equalizer off", |m| !audio_filters(m).contains("@rvpeq:"));
            }
            Action::Color(ColorChange::Toggle) => {
                self.osd(action, "Color controls: off");
                self.osd(action, "Color controls: last used");
            }
            Action::Color(ColorChange::Up(kind) | ColorChange::Down(kind)) => {
                let sign = if matches!(action, Action::Color(ColorChange::Up(_))) {
                    1
                } else {
                    -1
                };
                self.osd(action, &format!("{}: {sign}", kind.text()));
                assert_eq!(
                    self.engine.get_double(kind.property()).map(f64::round),
                    Some(f64::from(sign))
                );
                self.osd(
                    Action::Color(ColorChange::Reset(kind)),
                    &format!("{}: 0", kind.text()),
                );
            }
            Action::DeinterlaceCycle => {
                self.osd(action, "Deinterlacing: On");
                self.osd(action, "Deinterlacing: Off");
                self.osd(action, "Deinterlacing: Auto");
            }
            Action::VideoEffect(effect) => {
                self.osd(action, &format!("{}: on", effect.text()));
                assert!(
                    vf(self).contains(&format!("@{}:", effect.label())),
                    "{}",
                    vf(self)
                );
                self.osd(action, &format!("{}: off", effect.text()));
                assert!(!vf(self).contains(&format!("@{}:", effect.label())));
            }
            Action::Rotate(Rotation::Cycle) => {
                for degrees in [90, 180, 270, 0] {
                    self.osd(action, &format!("Rotation: {degrees}°"));
                }
            }
            Action::Shaders(ShaderCommand::Toggle(stage)) => {
                let name = if stage == Stage::Pre {
                    "Pre-resize shaders"
                } else {
                    "Post-resize shaders"
                };
                self.osd(action, &format!("{name}: on"));
                self.osd(action, &format!("{name}: off"));
            }
            _ => self.check_m11(action),
        }
    }

    /// Audio and subtitle sync, subtitle size and position.
    fn check_sync_and_subtitles(&mut self, action: Action) {
        match action {
            Action::AudioDelay(delay) => {
                let (text, value) = match delay {
                    Delay::Earlier => ("Audio sync: -50 ms", -0.05),
                    Delay::Later => ("Audio sync: +50 ms", 0.05),
                    Delay::Reset => ("Audio sync: +0.0 s", 0.0),
                };
                self.osd(action, text);
                self.wait_double(text, PropertyId::AudioDelay, value);
                self.osd(Action::AudioDelay(Delay::Reset), "Audio sync: +0.0 s");
                self.wait_double("audio sync reset", PropertyId::AudioDelay, 0.0);
            }
            Action::SubtitleDelay(delay) => {
                let (text, value) = match delay {
                    Delay::Earlier => ("Subtitle sync: -500 ms", -0.5),
                    Delay::Later => ("Subtitle sync: +500 ms", 0.5),
                    Delay::Reset => ("Subtitle sync: +0.0 s", 0.0),
                };
                self.osd(action, text);
                self.wait_double(text, PropertyId::SubDelay, value);
                self.osd(Action::SubtitleDelay(Delay::Reset), "Subtitle sync: +0.0 s");
                self.wait_double("subtitle sync reset", PropertyId::SubDelay, 0.0);
            }
            Action::Subtitle(style @ (SubtitleStyle::Bigger | SubtitleStyle::Smaller)) => {
                let (text, value, back) = if style == SubtitleStyle::Bigger {
                    ("Subtitle size: 110 %", 1.1, SubtitleStyle::Smaller)
                } else {
                    ("Subtitle size: 90 %", 0.9, SubtitleStyle::Bigger)
                };
                self.osd(action, text);
                self.wait_double(text, PropertyId::SubScale, value);
                self.osd(Action::Subtitle(back), "Subtitle size: 100 %");
                self.wait_double("subtitle size back", PropertyId::SubScale, 1.0);
            }
            Action::Subtitle(style) => {
                let (text, value) = match style {
                    SubtitleStyle::Up => ("Subtitle position: 99 %", 99.0),
                    SubtitleStyle::Down => ("Subtitle position: 101 %", 101.0),
                    _ => ("Subtitle position: 100 %", 100.0),
                };
                self.osd(action, text);
                self.wait_double(text, PropertyId::SubPos, value);
                let reset = Action::Subtitle(SubtitleStyle::DefaultPosition);
                self.osd(reset, "Subtitle position: 100 %");
                self.wait_double("subtitle position back", PropertyId::SubPos, 100.0);
            }
            _ => self.check_m11(action),
        }
    }

    /// The A-B repeat's points and the playback finished actions.
    fn check_repeat(&mut self, action: Action) {
        match action {
            Action::AbRepeat(
                change @ (AbRepeat::SetA
                | AbRepeat::SetB
                | AbRepeat::ClearA
                | AbRepeat::ClearB
                | AbRepeat::Toggle
                | AbRepeat::Clear),
            ) => {
                let text = match change {
                    AbRepeat::SetA => "A-B Repeat (off): A 0:00:30 B -",
                    AbRepeat::SetB => "A-B Repeat (off): A - B 0:00:30",
                    _ => "A-B Repeat (off): A - B -",
                };
                self.osd(action, text);
                self.osd(
                    Action::AbRepeat(AbRepeat::Clear),
                    "A-B Repeat (off): A - B -",
                );
            }
            Action::AfterPlaybackCycle => {
                self.osd(action, "When playback ends: exit");
                self.osd(action, "When the playlist ends: exit");
                self.osd(action, "When playback ends: shut down");
                self.osd(action, "When the playlist ends: shut down");
                self.osd(action, "When playback ends: do nothing");
            }
            _ => self.check_m11(action),
        }
    }

    /// M11's commands that keys start.
    fn check_m11(&mut self, action: Action) {
        match action {
            // Keyframes every second: from 30 s, 29 s and 31 s.
            Action::JumpTo(jump @ (Jump::PreviousKeyframe | Jump::NextKeyframe)) => {
                assert_eq!(self.dispatch(action), None);
                let expected = if jump == Jump::NextKeyframe {
                    31.0
                } else {
                    29.0
                };
                self.wait("keyframe", |m| {
                    (m.double(PropertyId::TimePos).unwrap_or_default() - expected).abs() < 0.05
                });
                self.back_to_30();
            }
            Action::OpenDefault => {
                assert_eq!(self.dispatch(action), Some(UiRequest::OpenFileDialog));
            }
            Action::JumpDialog => match self.dispatch(action) {
                Some(UiRequest::JumpDialog {
                    time,
                    duration,
                    fps,
                }) => {
                    assert!((time - 30.0).abs() < 0.1, "{time}");
                    assert!((duration - 60.0).abs() < 0.2, "{duration}");
                    assert!((fps - 10.0).abs() < 0.01, "{fps}");
                }
                other => panic!("{action:?}: {other:?}"),
            },
            Action::Scenes(_) => match self.dispatch(action) {
                Some(UiRequest::SceneBrowser(browser)) => {
                    // No markers: the time-based browser, every 3 s (60 / 24).
                    assert_eq!(browser.scenes.len(), 20, "{browser:?}");
                }
                other => panic!("{action:?}: {other:?}"),
            },
            Action::AbRepeat(AbRepeat::MoveA(_) | AbRepeat::MoveB(_) | AbRepeat::MoveBoth(_)) => {
                self.osd_starts(Action::AbRepeat(AbRepeat::SetA), "A-B Repeat");
                self.osd_starts(action, "A-B Repeat");
                self.osd_starts(Action::AbRepeat(AbRepeat::Clear), "A-B Repeat (off)");
            }
            // No subtitle line shows: the 5 s behind the current point.
            Action::AbRepeat(AbRepeat::CurrentSubtitle | AbRepeat::Behind) => {
                self.osd(action, "A-B Repeat: A 0:00:25 B 0:00:30");
                self.osd_starts(Action::AbRepeat(AbRepeat::Clear), "A-B Repeat (off)");
                self.back_to_30();
            }
            Action::AbRepeat(AbRepeat::PreviousSubtitle | AbRepeat::NextSubtitle) => {
                assert_eq!(self.dispatch(action), None);
            }
            Action::AbRepeat(
                AbRepeat::CurrentSection | AbRepeat::PreviousSection | AbRepeat::NextSection,
            ) => self.osd(action, "The A-B repeat list is empty"),
            Action::AbRepeat(AbRepeat::MoreLoops) => {
                self.osd(action, "A-B repeat: 1 times");
                self.osd(
                    Action::AbRepeat(AbRepeat::FewerLoops),
                    "A-B repeat: endless",
                );
            }
            Action::AbRepeat(AbRepeat::FewerLoops) => self.osd(action, "A-B repeat: endless"),
            Action::AbRepeat(AbRepeat::ListSetup) => {
                assert_eq!(self.dispatch(action), Some(UiRequest::SectionList));
            }
            Action::Skip(Skip::Toggle) => {
                self.osd(action, "Skip: On");
                self.osd(action, "Skip: Off");
            }
            Action::Skip(Skip::Setup) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Preferences(Some("skip")))
                );
            }
            Action::Bookmark(BookmarkCommand::Add) => {
                self.osd(action, "Bookmark added: Bookmark 1 (00:00:30)");
                let path = self
                    .app
                    .model
                    .loaded_path()
                    .map(str::to_owned)
                    .expect("plays");
                assert_eq!(self.app.marks().file(&path).bookmarks.len(), 1);
                self.app.marks_mut().clear_bookmarks(&path);
            }
            _ => self.check_m11_open(action),
        }
    }

    /// M11's opening commands that keys start.
    fn check_m11_open(&mut self, action: Action) {
        match action {
            Action::Open(Source::Network) => {
                assert_eq!(self.dispatch(action), Some(UiRequest::OpenUrlDialog));
            }
            Action::Open(Source::Clipboard) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ReadClipboard(rvp_app::sources::Paste::Open))
                );
            }
            // It would open the screen: `sources` checks its address.
            Action::Open(Source::Screen) => {}
            Action::Open(Source::Device) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::DeviceDialog(Vec::new()))
                );
            }
            Action::Open(source @ (Source::Dvd | Source::Bluray)) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::DiscDialog {
                        bluray: source == Source::Bluray,
                        files: false
                    })
                );
            }
            Action::Settings(page) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::Preferences(Some(page.key())))
                );
            }
            Action::ReloadSubtitle => self.osd(action, "No subtitle file to reload"),
            Action::ChapterMenu => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::PopupMenu("CMD_POPUPMENU_PLAY_BOOKMARK"))
                );
            }
            // Only asks: the window deletes after the user says yes.
            Action::DeleteFiles(_) => match self.dispatch(action) {
                Some(UiRequest::ConfirmRecycle { files, entries }) => {
                    assert!(!files.is_empty() && !entries.is_empty());
                }
                other => panic!("{action:?}: {other:?}"),
            },
            Action::AddFavoriteItem => self.osd(action, "Added to favorites: fixture-60s.mkv"),
            Action::AddFavoriteFolder => self.osd(action, "Added to favorites: media"),
            _ => self.check_m12(action),
        }
    }

    /// M12's commands that keys start.
    fn check_m12(&mut self, action: Action) {
        let vf = |h: &Self| h.engine.get_text("vf").unwrap_or_default();
        match action {
            Action::ToggleFullscreenCrop => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::ToggleFullscreen {
                        fit: Some(Fit::Crop)
                    })
                );
            }
            Action::Picture(PictureCommand::BottomMarginCycle) => {
                self.osd(action, "Bottom margin: 5 %");
                assert!(vf(self).contains("@rvpmargin:"), "{}", vf(self));
                for percent in [10, 15, 20, 25, 30] {
                    self.osd(action, &format!("Bottom margin: {percent} %"));
                }
                self.osd(action, "Bottom margin: off");
                assert!(!vf(self).contains("@rvpmargin:"), "{}", vf(self));
                // Without a video output, mpv's refresh after a filter
                // change can land on the next keyframe.
                self.back_to_30();
            }
            Action::Picture(PictureCommand::ModeCycle) => {
                for text in [
                    "Stretch by keeping aspect ratio",
                    "Stretch",
                    "Video frame at 0.5x",
                    "Video frame at 1.0x",
                    "Video frame at 1.5x",
                    "Video frame at 2.0x",
                    "Keep aspect ratio",
                ] {
                    self.osd(action, text);
                }
                assert_eq!(
                    self.engine.get_text("video-unscaled").as_deref(),
                    Some("no")
                );
            }
            Action::Picture(PictureCommand::PresetCycle) => {
                for name in [
                    "Zoom 1",
                    "Zoom 2",
                    "Zoom 3",
                    "Increasing width 1",
                    "Increasing width 2",
                    "Increasing width 3",
                    "Increasing Height 1",
                    "Increasing Height 2",
                    "Increasing Height 3",
                    "(Default)",
                ] {
                    self.osd(action, &format!("Frame size: {name}"));
                }
                assert_eq!(self.engine.get_double("video-scale-x"), Some(1.0));
            }
            other => self.check_m12_window(other),
        }
    }

    /// M12's window and capture keys.
    fn check_m12_window(&mut self, action: Action) {
        match action {
            // The window carries these out (Main.qml's windowRequest).
            Action::Window(command) => {
                let expected = match command {
                    WindowCommand::Minimum => WindowRequest::Minimum,
                    WindowCommand::Maximum => WindowRequest::Maximum,
                    WindowCommand::Custom => WindowRequest::Size {
                        width: 960,
                        height: 600,
                    },
                    WindowCommand::CustomDialog => WindowRequest::SizeDialog {
                        width: 960,
                        height: 600,
                    },
                    WindowCommand::Desktop(index) => WindowRequest::Desktop {
                        percent: [30.0, 45.0, 60.0, 75.0][index],
                    },
                    WindowCommand::Grow(larger) => WindowRequest::Grow {
                        factor: if larger { 1.1 } else { 1.0 / 1.1 },
                    },
                    WindowCommand::Move(placement) => WindowRequest::Move {
                        placement,
                        step: 20,
                    },
                    WindowCommand::ControlDialog => WindowRequest::ControlDialog,
                    WindowCommand::FullscreenOtherDisplay => WindowRequest::FullscreenOtherDisplay,
                };
                assert_eq!(self.dispatch(action), Some(UiRequest::Window(expected)));
            }
            Action::Capture(CaptureCommand::Copy(shot)) => match self.dispatch(action) {
                Some(UiRequest::Capture(job)) => {
                    assert_eq!(job.path, None);
                    let mode = if shot == Shot::Screen {
                        "window"
                    } else {
                        "video"
                    };
                    assert_eq!(job.mode, mode);
                }
                other => panic!("{action:?}: {other:?}"),
            },
            Action::Capture(CaptureCommand::ConsecutiveDialog) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::CaptureDialog("consecutive"))
                );
            }
            Action::Record(RecordCommand::Stream | RecordCommand::Video) => {
                match self.dispatch(action) {
                    Some(
                        UiRequest::RecordStreamDialog(path) | UiRequest::RecordVideoDialog(path),
                    ) => {
                        assert!(
                            path.extension().is_some_and(|e| e == "mkv"),
                            "{}",
                            path.display()
                        );
                    }
                    other => panic!("{action:?}: {other:?}"),
                }
            }
            Action::Record(RecordCommand::StreamToggle) => {
                match self.dispatch(action) {
                    Some(UiRequest::Osd(text)) => {
                        assert!(
                            text.starts_with("Recording the stream: fixture-60s"),
                            "{text}"
                        );
                    }
                    other => panic!("{action:?}: {other:?}"),
                }
                match self.dispatch(action) {
                    Some(UiRequest::Osd(text)) => {
                        assert!(text.starts_with("Recorded the stream"), "{text}");
                    }
                    other => panic!("{action:?}: {other:?}"),
                }
            }
            Action::Popup(popup) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::PopupMenu(popup.command()))
                );
            }
            Action::Capture(CaptureCommand::SheetDialog) => {
                assert_eq!(
                    self.dispatch(action),
                    Some(UiRequest::CaptureDialog("sheet"))
                );
            }
            _ => self.check_m18(action),
        }
    }

    /// M18's default key: motion smoothing on (MVTools, or Blend without a
    /// VapourSynth runtime here) and off again.
    fn check_m18(&mut self, action: Action) {
        match action {
            Action::Smoothing(SmoothingCommand::Toggle) => {
                for on in [true, false] {
                    match self.dispatch(action) {
                        Some(UiRequest::Osd(text)) => {
                            assert!(text.starts_with("Motion smoothing: "), "{text}");
                            assert_eq!(text.ends_with(": Off"), !on, "{text}");
                        }
                        other => panic!("{action:?}: {other:?}"),
                    }
                    assert_eq!(self.app.settings.smoothing.enabled, on);
                }
            }
            _ => panic!("{action:?} has no check"),
        }
    }
}

/// Every action that some default key or mouse gesture is bound to, carried
/// out once with its expected effect.
#[test]
fn every_default_binding_triggers_its_action() {
    let bindings = Bindings::potplayer_defaults();
    let mut bound: Vec<Action> = bindings
        .keys
        .values()
        .chain(bindings.mouse.values())
        .copied()
        .collect();
    bound.sort();
    bound.dedup();
    // Playlist moves and Close Playback change the file: do them last.
    let changes_file =
        |a: &Action| matches!(a, Action::Previous | Action::Next | Action::CloseMedia);
    let (late, early): (Vec<Action>, Vec<Action>) = bound.into_iter().partition(changes_file);

    let mut h = Harness::new();
    h.app
        .open(&h.engine, &[LONG.to_owned(), SHORT.to_owned()], false)
        .expect("open");
    h.wait("loaded", |m| {
        m.media_loaded() && m.double(PropertyId::Duration).is_some()
    });
    h.app.apply_audio_chain(&h.engine);
    h.engine.set_pause(true).expect("pause");
    h.engine.set_volume(mpv_volume(50.0)).expect("volume");
    h.wait("paused at 50 %", |m| {
        m.flag(PropertyId::Pause) == Some(true) && shown_volume(m, 50.0)
    });
    h.back_to_30();

    for action in early.into_iter().chain(late) {
        h.check(action);
    }
    // Close Playback last: it unloads the file.
    h.dispatch(Action::CloseMedia);
    h.wait("idle", |m| !m.media_loaded());
    // With nothing loaded, Play plays the album's selected entry ...
    assert_eq!(h.dispatch(Action::PlayPause), None);
    h.wait("playing again", PlayerModel::media_loaded);
    h.dispatch(Action::CloseMedia);
    h.wait("idle again", |m| !m.media_loaded());
    // ... and asks for files when the album is empty.
    h.app
        .list_action(&h.engine, rvp_app::ListAction::RemoveAll, false, 10);
    assert_eq!(
        h.dispatch(Action::PlayPause),
        Some(UiRequest::OpenFileDialog)
    );
}

#[test]
fn menu_shows_potplayer_texts_and_current_keys() {
    let dir = std::env::temp_dir().join(format!("rvp-menu-{}", std::process::id()));
    let app = App::new(Settings::default(), &dir).expect("app");
    let entry = |action| app.menu_entry(action);
    assert_eq!(
        entry(Action::SeekBackward(SeekStep::Medium)),
        ("30 Sec Backward".to_owned(), "Ctrl+Left".to_owned())
    );
    assert_eq!(
        entry(Action::SeekForward(SeekStep::VeryLong)),
        ("5 Min Forward".to_owned(), "Ctrl+Alt+Right".to_owned())
    );
    assert_eq!(
        entry(Action::ToggleFullscreen),
        ("Fullscreen (Keep AR)".to_owned(), "Alt+Enter".to_owned())
    );
    assert_eq!(
        entry(Action::Exit),
        ("Exit".to_owned(), "Alt+F4".to_owned())
    );
    // With PotPlayer's key unbound, the menu shows another bound key.
    let mut settings = Settings::default();
    settings.keys.insert("Alt+Enter".into(), String::new());
    let rebound = App::new(settings, &dir).expect("app");
    assert_eq!(rebound.menu_entry(Action::ToggleFullscreen).1, "Enter");
    let _ = std::fs::remove_dir_all(&dir);
}
