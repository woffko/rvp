//! M12's video, picture and window features, headless (no window, no
//! sound): the aspect ratio modes, the output device, Extend/Crop, the
//! bottom margin and Video > Resize as filters before SVP's (whose `@svp`
//! stays where it is), the screen margins and the subtitles' room, the
//! rotation by aspect ratio, and Frame Size's presets.
//!
//! `RVP_TESTDATA` points at `testdata/media` when the test runs elsewhere
//! (the Windows VM).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rvp_app::picture::{
    AutoRotate, ExtendMethod, OutputDevice, PictureCommand, ResizeTarget, ResizeWhen, Screen, Side,
    SubtitleRoom,
};
use rvp_app::stereo::{ModeSwitch, StereoCommand, StereoOutput};
use rvp_app::video::VideoEffect;
use rvp_app::{Action, App, Aspect, Fit, Settings, UiRequest};
use rvp_engine::{Engine, EngineOptions, PropertyId};

fn media(name: &str) -> String {
    let dir = std::env::var_os("RVP_TESTDATA").map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media")),
        PathBuf::from,
    );
    dir.join(name).to_string_lossy().into_owned()
}

struct Harness {
    app: App,
    engine: Engine,
    dir: PathBuf,
}

impl Harness {
    fn new(name: &str, change: impl FnOnce(&mut Settings)) -> Self {
        let dir = std::env::temp_dir().join(format!("rvp-m12-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let mut settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            ..Settings::default()
        };
        change(&mut settings);
        Self {
            app: App::new(settings, &dir).expect("app"),
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
            dir,
        }
    }

    fn open(&mut self, files: &[String]) {
        self.app.open(&self.engine, files, false).expect("open");
        self.wait("the file", |h| {
            h.app.model.media_loaded() && h.app.model.int(PropertyId::VideoWidth).is_some()
        });
    }

    /// Drains until `done` holds or 15 s pass.
    fn wait(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !done(self) {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: vf {:?}, out {:?}x{:?}",
                self.engine.get_text("vf"),
                self.engine.get_int("video-out-params/w"),
                self.engine.get_int("video-out-params/h"),
            );
            std::thread::sleep(Duration::from_millis(5));
            self.app.drain(&self.engine, Instant::now());
        }
    }

    fn picture(&mut self, command: PictureCommand) -> Option<UiRequest> {
        self.app.dispatch(&self.engine, Action::Picture(command))
    }

    fn stereo(&mut self, command: StereoCommand) -> Option<UiRequest> {
        self.app.dispatch(&self.engine, Action::Stereo(command))
    }

    /// Waits for a frame (after RVP's filters) whose pixels pass `check`,
    /// given the frame's size and its colour at a place.
    fn wait_pixels(&mut self, what: &str, check: impl Fn(&rvp_engine::Image) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            self.app.drain(&self.engine, Instant::now());
            if let Ok(image) = self.engine.screenshot_raw("video")
                && check(&image)
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}: vf {:?}",
                self.engine.get_text("vf")
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn text(&self, name: &str) -> String {
        self.engine.get_text(name).unwrap_or_default()
    }

    fn number(&self, name: &str) -> f64 {
        self.engine.get_double(name).unwrap_or(f64::NAN)
    }

    /// Waits for the frame after RVP's filters to be `width` x `height`.
    fn wait_out(&mut self, what: &str, width: i64, height: i64) {
        self.wait(what, |h| {
            h.engine.get_int("video-out-params/w") == Some(width)
                && h.engine.get_int("video-out-params/h") == Some(height)
        });
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn aspect_ratio_modes_reach_mpv() {
    let mut h = Harness::new("aspect", |settings| {
        "2.39:1".clone_into(&mut settings.picture.custom_aspect);
    });
    h.open(&[media("fixture-60s.mkv")]);
    h.app.dispatch(&h.engine, Action::SetAspect(Aspect::Custom));
    assert!(near(h.number("video-aspect-override"), 2.39));
    // The custom ratio's dialog: a new ratio applies and is kept.
    assert!(matches!(
        h.picture(PictureCommand::CustomAspectDialog),
        Some(UiRequest::PictureDialog {
            kind: "custom-aspect",
            ..
        })
    ));
    h.app
        .picture_dialog_answer(&h.engine, "custom-aspect", "21:9");
    assert!(near(h.number("video-aspect-override"), 21.0 / 9.0));
    assert!(matches!(
        h.app
            .picture_dialog_answer(&h.engine, "custom-aspect", "wide"),
        Some(UiRequest::Message(_))
    ));
    assert_eq!(h.app.settings.picture.custom_aspect, "21:9");
    // Keep Video Frame to 2.0x: unscaled, zoomed by one step of two.
    h.app
        .dispatch(&h.engine, Action::VideoFit(Fit::FrameDouble));
    assert_eq!(h.text("video-unscaled"), "yes");
    assert!(near(h.number("video-zoom"), 1.0));
    h.app.dispatch(&h.engine, Action::VideoFit(Fit::KeepAspect));
    assert_eq!(h.text("video-unscaled"), "no");
    assert!(near(h.number("video-zoom"), 0.0));
    // Fullscreen (Stretch by Keeping AR) fills the screen while it lasts.
    h.app.set_fullscreen(&h.engine, true, Some(Fit::Crop));
    assert!(near(h.number("panscan"), 1.0));
    h.app.set_fullscreen(&h.engine, false, None);
    assert!(near(h.number("panscan"), 0.0));
    // "Adjust AR to Window Size": stretched in the window only.
    h.picture(PictureCommand::ToggleAdjustToWindow);
    assert_eq!(h.text("keepaspect"), "no");
    h.app.set_fullscreen(&h.engine, true, None);
    assert_eq!(h.text("keepaspect"), "yes");
    // The output device: a 4:3 television fed 1920x1080.
    h.app.set_screen(
        &h.engine,
        Screen {
            width: 1920,
            height: 1080,
            width_mm: 600.0,
            height_mm: 340.0,
        },
    );
    h.picture(PictureCommand::OutputDevice(OutputDevice::Tv4x3));
    assert!(near(h.number("monitorpixelaspect"), 0.75));
    h.picture(PictureCommand::OutputDevice(OutputDevice::Auto));
    assert!(near(
        h.number("monitorpixelaspect"),
        (600.0 / 340.0) / (1920.0 / 1080.0)
    ));
}

#[test]
fn extend_crop_and_margins_change_the_frame_before_svps_filter() {
    let mut h = Harness::new("crop", |_| {});
    // 96x72 (4:3), square pixels.
    h.open(&[media("fixture-60s.mkv")]);
    // SVP's filter, as SVP adds it: RVP's go in front of it.
    h.engine
        .command("vf", &["add", "@svp:lavfi=[null]"])
        .expect("svp's filter");
    h.picture(PictureCommand::ExtendPreset(
        ExtendMethod::Extend,
        Aspect::R16x9,
    ));
    h.wait_out("extended to 16:9", 128, 72);
    h.picture(PictureCommand::ExtendPreset(
        ExtendMethod::Crop,
        Aspect::R16x9,
    ));
    h.wait_out("cropped to 16:9", 96, 54);
    let vf = h.text("vf");
    let crop = vf.find("@rvpcrop:").unwrap_or(usize::MAX);
    let svp = vf.find("@svp:").unwrap_or(0);
    assert!(crop < svp, "{vf}");
    // A move changes where the picture is cut.
    h.picture(PictureCommand::Move(Side::Top));
    assert!(h.text("vf").contains("(ih-oh)*0.45"), "{}", h.text("vf"));
    h.picture(PictureCommand::ResetPosition);
    let [x, y] = h.app.settings.picture.position;
    assert!(near(x, 50.0) && near(y, 50.0));
    // By pixel: cut to 64x64, or extended to 160x90.
    h.app
        .picture_dialog_answer(&h.engine, "extend-size", "64x64");
    h.wait_out("cut to 64x64", 64, 64);
    h.app
        .picture_dialog_answer(&h.engine, "extend-size", "160x90");
    h.wait_out("extended to 160x90", 160, 90);
    h.picture(PictureCommand::ExtendOff);
    // The bottom margin: 10 % of black below the picture.
    h.picture(PictureCommand::BottomMargin(10));
    h.wait_out("10 % below", 96, 78);
    // Only with subtitles: the fixture has none.
    h.picture(PictureCommand::ToggleBottomMarginSubtitlesOnly);
    h.wait_out("no margin without subtitles", 96, 72);
    h.picture(PictureCommand::ToggleBottomMarginSubtitlesOnly);
    h.picture(PictureCommand::BottomMargin(0));
    // Video > Resize: twice the size, always.
    h.app.settings.picture.resize.target = ResizeTarget::Multiply;
    h.picture(PictureCommand::Resize(ResizeWhen::Always));
    h.wait_out("resized", 192, 144);
    // Only larger frames shrink: this one stays.
    h.picture(PictureCommand::Resize(ResizeWhen::Larger));
    h.wait_out("not resized", 96, 72);
    h.picture(PictureCommand::Resize(ResizeWhen::Never));
    let vf = h.text("vf");
    assert!(!vf.contains("@rvp"), "{vf}");
    assert!(vf.contains("@svp:"), "{vf}");
}

#[test]
fn screen_margins_and_the_subtitles_room() {
    let mut h = Harness::new("margins", |_| {});
    h.open(&[media("fixture-chapters.mkv")]);
    h.wait("the subtitle track", |h| {
        h.app.model.int(PropertyId::Sid).is_some()
    });
    h.picture(PictureCommand::ScreenMargin(Side::Left, true));
    h.picture(PictureCommand::ScreenMargin(Side::Left, true));
    h.picture(PictureCommand::ScreenMargin(Side::Top, true));
    assert!(near(h.number("video-margin-ratio-left"), 0.02));
    assert!(near(h.number("video-margin-ratio-top"), 0.01));
    // Only in fullscreen: the window has none.
    h.picture(PictureCommand::ToggleMarginsFullscreenOnly);
    assert!(near(h.number("video-margin-ratio-left"), 0.0));
    h.app.set_fullscreen(&h.engine, true, None);
    assert!(near(h.number("video-margin-ratio-left"), 0.02));
    h.app.set_fullscreen(&h.engine, false, None);
    h.picture(PictureCommand::ToggleMarginsFullscreenOnly);
    // Not remembered: a new run starts without them.
    assert!(near(h.app.settings.picture.margins.left, 0.0));
    h.picture(PictureCommand::ScreenMargin(Side::Left, false));
    h.picture(PictureCommand::ScreenMargin(Side::Left, false));
    h.picture(PictureCommand::ScreenMargin(Side::Top, false));
    // The subtitles' room: two lines under the video.
    h.picture(PictureCommand::SubtitleRoom(SubtitleRoom::TwoLines));
    let room = h.number("video-margin-ratio-bottom");
    assert!(room > 0.15 && room < 0.3, "{room}");
    assert_eq!(h.text("sub-use-margins"), "yes");
    // The space below the video: the video goes to the top.
    h.picture(PictureCommand::SubtitleRoom(SubtitleRoom::Available));
    assert!(near(h.number("video-margin-ratio-bottom"), 0.0));
    assert!(near(h.number("video-align-y"), -1.0));
    h.picture(PictureCommand::ToggleSubtitleMargin);
    assert!(near(h.number("video-align-y"), 0.0));
}

#[test]
fn rotation_by_aspect_ratio_and_frame_presets() {
    let mut h = Harness::new("rotate", |_| {});
    h.open(&[media("fixture-60s.mkv")]);
    // A portrait screen: the 4:3 video turns.
    h.app.set_screen(
        &h.engine,
        Screen {
            width: 1080,
            height: 1920,
            width_mm: 0.0,
            height_mm: 0.0,
        },
    );
    h.picture(PictureCommand::AutoRotate(AutoRotate::Quarter));
    assert_eq!(h.text("video-rotate"), "90");
    assert_eq!(h.app.auto_rotation_now(), 90);
    h.picture(PictureCommand::AutoRotate(AutoRotate::Half));
    assert_eq!(h.text("video-rotate"), "270");
    // Choosing the active one again turns it off.
    h.picture(PictureCommand::AutoRotate(AutoRotate::Half));
    assert_eq!(h.app.auto_rotation_now(), 0);
    // Frame Size's presets stretch the frame; a new file starts afresh.
    h.picture(PictureCommand::Preset(4));
    assert!(near(h.number("video-scale-x"), 1.1));
    assert!(near(h.number("video-scale-y"), 1.0));
    assert_eq!(h.app.picture_checked(PictureCommand::Preset(4)), Some(true));
    h.open(&[media("fixture-3s.mkv")]);
    h.wait("the next file", |h| {
        h.app
            .model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-3s.mkv"))
    });
    assert!(near(h.app.session().scale_x, 1.0));
    // Remembered: the next file keeps it.
    h.picture(PictureCommand::ToggleRememberFrame);
    h.picture(PictureCommand::Preset(1));
    h.open(&[media("fixture-60s.mkv")]);
    h.wait("the first file again", |h| {
        h.app
            .model
            .loaded_path()
            .is_some_and(|path| path.ends_with("fixture-60s.mkv"))
    });
    assert!(near(h.app.session().scale_x, 1.1));
    assert!(near(h.number("video-scale-y"), 1.1));
}

#[test]
fn every_effect_runs_in_mpvs_ffmpeg() {
    let mut h = Harness::new("effects", |_| {});
    h.open(&[media("fixture-60s.mkv")]);
    for effect in VideoEffect::ALL {
        h.app.dispatch(&h.engine, Action::VideoEffect(effect));
        let label = format!("@{}:", effect.label());
        assert!(
            h.text("vf").contains(&label),
            "{effect:?}: {}",
            h.text("vf")
        );
        // A quarter turn swaps the frame's sides; the others keep them.
        let (width, height) = if effect == VideoEffect::Rotate {
            (72, 96)
        } else {
            (96, 72)
        };
        h.wait_out(effect.text(), width, height);
        h.app.dispatch(&h.engine, Action::VideoEffect(effect));
        assert!(!h.text("vf").contains(&label), "{effect:?}");
    }
    // A strength changed in the settings rebuilds the filter.
    h.app
        .dispatch(&h.engine, Action::VideoEffect(VideoEffect::MotionBlur));
    let mut changed = h.app.settings.clone();
    changed.video_effects.motion_blur = 5;
    h.app.apply_settings(&h.engine, changed);
    assert!(h.text("vf").contains("tmix=frames=5"), "{}", h.text("vf"));
}

/// Red, green or blue above the others.
fn reddish(c: [u8; 3]) -> bool {
    c[0] > 180 && c[1] < 80 && c[2] < 80
}
fn bluish(c: [u8; 3]) -> bool {
    c[2] > 180 && c[0] < 80 && c[1] < 80
}

#[test]
fn three_d_outputs_put_each_eye_where_it_belongs() {
    let mut h = Harness::new("stereo", |_| {});
    drop(h.engine.set_text("loop-file", "inf"));
    // Left eye red, right eye blue; the file says side by side.
    h.open(&[media("fixture-sbs.mkv")]);
    h.stereo(StereoCommand::Mode(ModeSwitch::Auto));
    h.wait("the file's stereo metadata", |h| {
        h.engine
            .get_text("vf")
            .is_some_and(|vf| vf.contains("stereo3d=sbs2l:arcc"))
    });
    // Anaglyph red/cyan: red from the left eye, cyan from the right.
    h.wait_pixels("red/cyan", |image| {
        image
            .rgb(image.width / 2, image.height / 2)
            .is_some_and(|c| c[0] > 180 && c[1] < 80 && c[2] > 180)
    });
    h.stereo(StereoCommand::Output(StereoOutput::Left));
    h.wait_pixels("the left eye", |image| {
        image
            .rgb(image.width / 2, image.height / 2)
            .is_some_and(reddish)
    });
    h.stereo(StereoCommand::Output(StereoOutput::Right));
    h.wait_pixels("the right eye", |image| {
        image
            .rgb(image.width / 2, image.height / 2)
            .is_some_and(bluish)
    });
    h.stereo(StereoCommand::Output(StereoOutput::TopBottom));
    h.wait_pixels("top and bottom", |image| {
        let x = image.width / 2;
        image.rgb(x, image.height / 4).is_some_and(reddish)
            && image.rgb(x, image.height * 3 / 4).is_some_and(bluish)
    });
    h.stereo(StereoCommand::Output(StereoOutput::Rows));
    h.wait_pixels("rows", |image| {
        let x = image.width / 2;
        image.rgb(x, 20).is_some_and(reddish) && image.rgb(x, 21).is_some_and(bluish)
    });
    h.stereo(StereoCommand::Output(StereoOutput::Columns));
    h.wait_pixels("columns", |image| {
        let y = image.height / 2;
        image.rgb(40, y).is_some_and(reddish) && image.rgb(41, y).is_some_and(bluish)
    });
    // Swapped: the left image is the right eye's.
    h.stereo(StereoCommand::Output(StereoOutput::Left));
    h.stereo(StereoCommand::ToggleAutoDetect);
    h.stereo(StereoCommand::Mode(ModeSwitch::On));
    h.stereo(StereoCommand::ToggleSwap);
    h.wait_pixels("the swapped left eye", |image| {
        image
            .rgb(image.width / 2, image.height / 2)
            .is_some_and(bluish)
    });
    // A moved right image keeps the frame's size.
    h.stereo(StereoCommand::MoveRight(Some(Side::Right)));
    assert!(
        h.text("vf").contains("split[rvpa][rvpb]"),
        "{}",
        h.text("vf")
    );
    h.stereo(StereoCommand::Mode(ModeSwitch::Off));
    assert!(!h.text("vf").contains("@rvp3d:"));
}

#[test]
fn the_360_view_turns_while_it_plays() {
    let mut h = Harness::new("vr", |_| {});
    drop(h.engine.set_text("loop-file", "inf"));
    h.app.set_screen(
        &h.engine,
        Screen {
            width: 320,
            height: 180,
            width_mm: 0.0,
            height_mm: 0.0,
        },
    );
    // Named as 360° video: Auto turns it on.
    h.stereo(StereoCommand::VrMode(ModeSwitch::Auto));
    h.open(&[media("fixture-360.mkv")]);
    h.wait_out("a flat view of the screen's size", 320, 180);
    let vf = h.text("vf");
    assert!(
        vf.contains("@rvp360:") && vf.contains("v360=e:flat"),
        "{vf}"
    );
    h.app.turn_vr(&h.engine, -40.0, 20.0);
    assert!(near(h.app.vr_turn()[0], 10.0) && near(h.app.vr_turn()[1], 5.0));
    // Newer FFmpeg takes the turn as a command (the chain stays); 6.1's
    // v360 has no commands, so the filter is rebuilt with the turn.
    let turned = h.text("vf");
    assert!(
        turned == vf || turned.contains("yaw=10:pitch=5"),
        "{turned}"
    );
    if turned == vf {
        println!("note: v360 turned by vf-command");
    }
    h.stereo(StereoCommand::ToggleFourViews);
    h.wait_out("four views", 320, 180);
    assert!(h.text("vf").contains("xstack"), "{}", h.text("vf"));
    h.stereo(StereoCommand::VrToggle);
    assert!(!h.text("vf").contains("@rvp360:"));
}
