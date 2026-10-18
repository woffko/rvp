//! M12's 3D and 360° video, as RVP's own filters before SVP's.
//!
//! * 3D (`@rvp3d`): FFmpeg's `stereo3d` turns side by side or top and
//!   bottom input (full or half resolution, either eye first) into side by
//!   side or top and bottom output, rows or columns interleaved, one of
//!   fourteen anaglyphs, or one eye. The right (or bottom) image can be
//!   moved against the left one, and both images flipped. The input is
//!   found from mpv's `stereo-in` (the file's stereo metadata) or from the
//!   name's suffix (`-lr`, `-rl`, `-ab`, `-ba`, `sbs`, `tab`, `hsbs`...),
//!   and "Auto enable" turns the mode on when it is found. Shutter
//!   glasses, NVIDIA 3D Vision and Windows' stereo output need a stereo
//!   swap chain, which mpv's renderer does not have; 3D subtitles and
//!   H.264 MVC are studies (`docs/spikes/m12-studies.md`).
//! * 360° (`@rvp360`): FFmpeg's `v360` shows an equirectangular, dual
//!   fisheye or cubemap video as a flat view, or four views at 90°. The
//!   mouse turns it: `yaw` and `pitch` change with `vf-command` while it
//!   runs. "Auto enable" turns it on for files named as 360° video.

use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

pub const STEREO_LABEL: &str = "rvp3d";
pub const VR_LABEL: &str = "rvp360";

/// When a mode runs: never, always, or when the file asks for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModeSwitch {
    #[default]
    Off,
    On,
    Auto,
}

/// How the two images are packed in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StereoInput {
    #[default]
    SideBySide,
    TopBottom,
    /// Not stereo: nothing to convert.
    Original,
}

/// What the screen gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StereoOutput {
    SideBySide,
    TopBottom,
    /// 3D to 2D: the left (or top) image.
    Left,
    Right,
    Rows,
    Columns,
    RedCyanGray,
    RedCyanHalf,
    #[default]
    RedCyanColor,
    RedCyanDubois,
    GreenMagentaGray,
    GreenMagentaHalf,
    GreenMagentaColor,
    GreenMagentaDubois,
    YellowBlueGray,
    YellowBlueHalf,
    YellowBlueColor,
    YellowBlueDubois,
    RedBlueGray,
    RedGreenGray,
}

impl StereoOutput {
    /// `stereo3d`'s output format.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            StereoOutput::SideBySide => "sbs2l",
            StereoOutput::TopBottom => "ab2l",
            StereoOutput::Left => "ml",
            StereoOutput::Right => "mr",
            StereoOutput::Rows => "irl",
            StereoOutput::Columns => "icl",
            StereoOutput::RedCyanGray => "arcg",
            StereoOutput::RedCyanHalf => "arch",
            StereoOutput::RedCyanColor => "arcc",
            StereoOutput::RedCyanDubois => "arcd",
            StereoOutput::GreenMagentaGray => "agmg",
            StereoOutput::GreenMagentaHalf => "agmh",
            StereoOutput::GreenMagentaColor => "agmc",
            StereoOutput::GreenMagentaDubois => "agmd",
            StereoOutput::YellowBlueGray => "aybg",
            StereoOutput::YellowBlueHalf => "aybh",
            StereoOutput::YellowBlueColor => "aybc",
            StereoOutput::YellowBlueDubois => "aybd",
            StereoOutput::RedBlueGray => "arbg",
            StereoOutput::RedGreenGray => "argg",
        }
    }
}

/// The 3D settings (Preferences: Video > H.264 MVC 3D's "3D Video Mode
/// Settings").
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Stereo {
    pub mode: ModeSwitch,
    pub input: StereoInput,
    /// Each image has the frame's full width (or height).
    pub full_resolution: bool,
    /// The right (or bottom) image comes first.
    pub swap: bool,
    /// The input comes from the file's metadata or name when it says.
    pub auto_detect: bool,
    pub output: StereoOutput,
    /// The right (or bottom) image's offset in pixels (x, y).
    pub offset: [i32; 2],
    /// Both images flipped horizontally.
    pub invert: bool,
}

impl Default for Stereo {
    fn default() -> Self {
        Self {
            mode: ModeSwitch::Off,
            input: StereoInput::SideBySide,
            full_resolution: false,
            swap: false,
            auto_detect: true,
            output: StereoOutput::RedCyanColor,
            offset: [0, 0],
            invert: false,
        }
    }
}

/// A 360° video's projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Projection {
    /// From the frame's shape.
    #[default]
    Auto,
    Equirectangular,
    DualFisheye,
    /// A 3x2 cubemap.
    Cubemap,
    /// An equi-angular cubemap, 3x2.
    EquiAngular,
    /// An equi-angular cubemap of two eyes, top and bottom.
    EquiAngularStereo,
}

/// The 360° settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Vr {
    pub mode: ModeSwitch,
    pub projection: Projection,
    /// Four views: front, left, right and rear.
    pub four_views: bool,
    /// The view's horizontal field of view in degrees.
    pub fov: f64,
}

impl Default for Vr {
    fn default() -> Self {
        Self {
            mode: ModeSwitch::Off,
            projection: Projection::Auto,
            four_views: false,
            fov: 100.0,
        }
    }
}

impl Stereo {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        for value in &mut self.offset {
            if value.abs() > 200 {
                notes.push("stereo.offset must be -200 to 200 pixels".to_owned());
                *value = (*value).clamp(-200, 200);
            }
        }
    }
}

impl Vr {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        crate::playback::clamp_number(&mut self.fov, 30.0, 150.0, "vr.fov", notes);
    }
}

/// A 3D input found for a file: packing, full resolution, swapped.
pub type Found = (StereoInput, bool, bool);

/// The input mpv's `stereo-in` names (the file's metadata), if stereo.
#[must_use]
pub fn input_from_metadata(stereo_in: &str) -> Option<Found> {
    Some(match stereo_in {
        "sbsl" => (StereoInput::SideBySide, true, false),
        "sbsr" => (StereoInput::SideBySide, true, true),
        "sbs2l" => (StereoInput::SideBySide, false, false),
        "sbs2r" => (StereoInput::SideBySide, false, true),
        "abl" => (StereoInput::TopBottom, true, false),
        "abr" => (StereoInput::TopBottom, true, true),
        "ab2l" => (StereoInput::TopBottom, false, false),
        "ab2r" => (StereoInput::TopBottom, false, true),
        _ => return None,
    })
}

/// The input a file name's suffix or tag names (`film-lr.mkv`,
/// `film.hsbs.mkv`, `film_tab.mp4`), if any.
#[must_use]
pub fn input_from_name(path: &str) -> Option<Found> {
    let name = crate::view::file_name(path);
    let stem = name
        .rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_lowercase();
    let words: Vec<&str> = stem
        .split(['.', '_', '-', ' ', '[', ']'])
        .filter(|word| !word.is_empty())
        .collect();
    let has = |tag: &str| words.contains(&tag);
    let half_default = !(has("full") || has("fsbs") || has("ftab") || has("fou"));
    if has("lr") || has("sbs") || has("hsbs") || has("3dsbs") || has("fsbs") {
        return Some((StereoInput::SideBySide, !half_default, false));
    }
    if has("rl") {
        return Some((StereoInput::SideBySide, !half_default, true));
    }
    if has("ab")
        || has("tab")
        || has("htab")
        || has("ou")
        || has("hou")
        || has("ftab")
        || has("fou")
    {
        return Some((StereoInput::TopBottom, !half_default, false));
    }
    if has("ba") {
        return Some((StereoInput::TopBottom, !half_default, true));
    }
    None
}

/// `stereo3d`'s input format.
#[must_use]
pub fn input_code(input: StereoInput, full: bool, swap: bool) -> Option<&'static str> {
    Some(match (input, full, swap) {
        (StereoInput::Original, ..) => return None,
        (StereoInput::SideBySide, true, false) => "sbsl",
        (StereoInput::SideBySide, true, true) => "sbsr",
        (StereoInput::SideBySide, false, false) => "sbs2l",
        (StereoInput::SideBySide, false, true) => "sbs2r",
        (StereoInput::TopBottom, true, false) => "abl",
        (StereoInput::TopBottom, true, true) => "abr",
        (StereoInput::TopBottom, false, false) => "ab2l",
        (StereoInput::TopBottom, false, true) => "ab2r",
    })
}

/// A shift by (`dx`, `dy`) pixels in the same frame size, black where the
/// picture leaves it.
fn shift(dx: i32, dy: i32) -> String {
    let (ax, ay) = (dx.abs(), dy.abs());
    format!(
        "crop=w=iw-{ax}:h=ih-{ay}:x={}:y={},pad=w=iw+{ax}:h=ih+{ay}:x={}:y={}:color=black",
        (-dx).max(0),
        (-dy).max(0),
        dx.max(0),
        dy.max(0)
    )
}

/// The `stereo3d` graph for an input found or set: `None` when there is
/// nothing to convert.
#[must_use]
pub fn stereo_graph(stereo: &Stereo, found: Found) -> Option<String> {
    let (input, full, swap) = found;
    let code = input_code(input, full, swap)?;
    let output = stereo.output.code();
    let [dx, dy] = stereo.offset;
    if dx == 0 && dy == 0 && !stereo.invert {
        return Some(format!("stereo3d={code}:{output}"));
    }
    // The images apart (full side by side, left first), each moved or
    // flipped, then packed again for the output.
    let flip = if stereo.invert { ",hflip" } else { "" };
    let right_shift = if dx == 0 && dy == 0 {
        String::new()
    } else {
        format!(",{}", shift(dx, dy))
    };
    Some(format!(
        "stereo3d={code}:sbsl,split[rvpa][rvpb];[rvpa]crop=w=iw/2:h=ih:x=0:y=0{flip}[rvpl];[rvpb]crop=w=iw/2:h=ih:x=iw/2:y=0{flip}{right_shift}[rvpr];[rvpl][rvpr]hstack,stereo3d=sbsl:{output}"
    ))
}

/// The projection a frame of `width` x `height` looks like.
#[must_use]
pub fn guess_projection(width: i64, height: i64) -> Projection {
    if height <= 0 {
        return Projection::Equirectangular;
    }
    // Width and height of a frame are far inside f64's exact range.
    #[allow(clippy::cast_precision_loss)]
    let aspect = width as f64 / height as f64;
    if (aspect - 1.5).abs() < 0.02 {
        Projection::Cubemap
    } else {
        Projection::Equirectangular
    }
}

/// True for a file named as 360° video (`clip.360.mp4`, `vr360`,
/// `equirectangular`).
#[must_use]
pub fn named_360(path: &str) -> bool {
    let name = crate::view::file_name(path).to_lowercase();
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| {
            matches!(
                word,
                "360" | "vr360" | "360vr" | "equirectangular" | "equirect" | "spherical"
            )
        })
}

/// The `v360` graph: one flat view of `size` turned to `yaw`, `pitch`
/// and `roll`, or four views at 90° when `four` is set.
#[must_use]
pub fn vr_graph(
    projection: Projection,
    four: bool,
    fov: f64,
    size: (u32, u32),
    turn: [f64; 3],
) -> String {
    let input = match projection {
        Projection::Auto | Projection::Equirectangular => "e",
        Projection::DualFisheye => "dfisheye",
        Projection::Cubemap => "c3x2",
        Projection::EquiAngular => "eac",
        Projection::EquiAngularStereo => "eac:in_stereo=tb",
    };
    let (width, height) = if four { (size.0 / 2, size.1 / 2) } else { size };
    let (width, height) = (width.max(16) & !1, height.max(16) & !1);
    let fov = fov.clamp(30.0, 150.0);
    let v_fov = (2.0
        * ((fov.to_radians() / 2.0).tan() * f64::from(height) / f64::from(width)).atan())
    .to_degrees();
    let view = |yaw: f64| {
        // v360 takes -180 to 180 degrees.
        let yaw = (yaw + 180.0).rem_euclid(360.0) - 180.0;
        format!(
            "v360={input}:flat:h_fov={}:v_fov={}:w={width}:h={height}:yaw={}:pitch={}:roll={}",
            number(fov),
            number(v_fov),
            number(yaw),
            number(turn[1]),
            number(turn[2])
        )
    };
    if !four {
        return view(turn[0]);
    }
    let yaw = turn[0];
    format!(
        "split=4[rvpf][rvpl][rvpr][rvpb];[rvpf]{}[rvpf2];[rvpl]{}[rvpl2];[rvpr]{}[rvpr2];[rvpb]{}[rvpb2];[rvpf2][rvpl2][rvpr2][rvpb2]xstack=inputs=4:layout=0_0|w0_0|0_h0|w0_h0",
        view(yaw),
        view(yaw - 90.0),
        view(yaw + 90.0),
        view(yaw + 180.0)
    )
}

/// A number for FFmpeg's options, without needless digits.
fn number(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// One of the 3D and 360° commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StereoCommand {
    Mode(ModeSwitch),
    Input(StereoInput),
    ToggleFullResolution,
    ToggleSwap,
    ToggleAutoDetect,
    Output(StereoOutput),
    /// Moves the right (or bottom) image: `None` back to its place.
    MoveRight(Option<crate::picture::Side>),
    ToggleInvert,
    VrMode(ModeSwitch),
    /// "Enable/Disable 360° Video Mode".
    VrToggle,
    Projection(Projection),
    ToggleFourViews,
}

/// The 3D and 360° state for this run.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct StereoState {
    /// The 3D input found for the file that plays.
    pub found: Option<Found>,
    /// The file is named as 360° video.
    pub named_360: bool,
    /// The 360° view's yaw, pitch and roll in degrees.
    pub turn: [f64; 3],
}

/// The pixels the mouse moves to turn the view by one degree.
const PIXELS_PER_DEGREE: f64 = 4.0;

const STEREO_STEP: i32 = 2;

impl App {
    /// The 3D input in force: found or set.
    fn stereo_input(&self) -> Option<Found> {
        let stereo = &self.settings.stereo;
        let found = self.stereo.found.filter(|_| stereo.auto_detect);
        match stereo.mode {
            ModeSwitch::Off => None,
            ModeSwitch::Auto => found,
            ModeSwitch::On => {
                Some(found.unwrap_or((stereo.input, stereo.full_resolution, stereo.swap)))
            }
        }
    }

    /// True while 360° video mode runs.
    #[must_use]
    pub fn vr_active(&self) -> bool {
        match self.settings.vr.mode {
            ModeSwitch::Off => false,
            ModeSwitch::On => true,
            ModeSwitch::Auto => self.stereo.named_360,
        }
    }

    /// The 3D and 360° filters, as `(label, vf entry)`.
    #[must_use]
    pub fn stereo_filters(&self) -> Vec<(String, String)> {
        let mut filters = Vec::new();
        if let Some(found) = self.stereo_input()
            && let Some(graph) = stereo_graph(&self.settings.stereo, found)
        {
            filters.push((
                STEREO_LABEL.to_owned(),
                format!("@{STEREO_LABEL}:lavfi=[{graph}]"),
            ));
        }
        if self.vr_active() {
            filters.push((
                VR_LABEL.to_owned(),
                format!("@{VR_LABEL}:lavfi=[{}]", self.vr_graph_now()),
            ));
        }
        filters
    }

    fn vr_graph_now(&self) -> String {
        let vr = &self.settings.vr;
        let projection = if vr.projection == Projection::Auto {
            guess_projection(
                self.model
                    .int(rvp_engine::PropertyId::VideoWidth)
                    .unwrap_or(2),
                self.model
                    .int(rvp_engine::PropertyId::VideoHeight)
                    .unwrap_or(1),
            )
        } else {
            vr.projection
        };
        let screen = self.picture.screen;
        let size = if screen.width > 0 && screen.height > 0 {
            (screen.width.min(3840), screen.height.min(2160))
        } else {
            (1920, 1080)
        };
        vr_graph(projection, vr.four_views, vr.fov, size, self.stereo.turn)
    }

    /// A new file: its 3D input and whether it is named as 360° video.
    pub(crate) fn stereo_file_loaded(&mut self, engine: &rvp_engine::Engine) {
        let path = self.model.loaded_path().unwrap_or_default().to_owned();
        let metadata = engine
            .get_text("video-params/stereo-in")
            .and_then(|stereo| input_from_metadata(&stereo));
        let found = metadata.or_else(|| input_from_name(&path));
        let named = named_360(&path);
        let changed = found != self.stereo.found || named != self.stereo.named_360;
        self.stereo.found = found;
        self.stereo.named_360 = named;
        self.stereo.turn = [0.0; 3];
        let auto = self.settings.stereo.mode == ModeSwitch::Auto
            || self.settings.vr.mode == ModeSwitch::Auto
            || self.settings.vr.projection == Projection::Auto;
        if changed && auto {
            self.apply_decoding(engine);
            drop(self.apply_video_filters(engine));
        }
    }

    /// The mouse turned the 360° view by (`dx`, `dy`) pixels.
    pub fn turn_vr(&mut self, engine: &rvp_engine::Engine, dx: f64, dy: f64) {
        if !self.vr_active() {
            return;
        }
        let turn = &mut self.stereo.turn;
        turn[0] = (turn[0] - dx / PIXELS_PER_DEGREE + 180.0).rem_euclid(360.0) - 180.0;
        turn[1] = (turn[1] + dy / PIXELS_PER_DEGREE).clamp(-90.0, 90.0);
        let [yaw, pitch, _] = *turn;
        if self.settings.vr.four_views {
            drop(self.apply_video_filters(engine));
            return;
        }
        // While it runs: the filter's own commands, else a new filter.
        let commands = engine
            .command("vf-command", &[VR_LABEL, "yaw", &number(yaw)])
            .and_then(|()| engine.command("vf-command", &[VR_LABEL, "pitch", &number(pitch)]));
        if commands.is_err() {
            drop(self.apply_video_filters(engine));
        }
    }

    /// The 360° view's yaw and pitch (the self-test).
    #[must_use]
    pub fn vr_turn(&self) -> [f64; 3] {
        self.stereo.turn
    }

    /// The 3D and 360° commands.
    pub(crate) fn stereo_command(
        &mut self,
        engine: &rvp_engine::Engine,
        command: StereoCommand,
    ) -> UiRequest {
        let stereo = &mut self.settings.stereo;
        let notice = match command {
            StereoCommand::Mode(mode) => {
                stereo.mode = mode;
                format!(
                    "3D video mode: {}",
                    match mode {
                        ModeSwitch::Off => "off",
                        ModeSwitch::On => "on",
                        ModeSwitch::Auto => "automatic",
                    }
                )
            }
            StereoCommand::Input(input) => {
                stereo.input = input;
                if stereo.mode == ModeSwitch::Off {
                    stereo.mode = ModeSwitch::On;
                }
                format!(
                    "3D input: {}",
                    match input {
                        StereoInput::SideBySide => "side by side",
                        StereoInput::TopBottom => "top and bottom",
                        StereoInput::Original => "original image",
                    }
                )
            }
            StereoCommand::ToggleFullResolution => {
                stereo.full_resolution = !stereo.full_resolution;
                on_off_text("3D full resolution", stereo.full_resolution)
            }
            StereoCommand::ToggleSwap => {
                stereo.swap = !stereo.swap;
                on_off_text("3D: left and right swapped", stereo.swap)
            }
            StereoCommand::ToggleAutoDetect => {
                stereo.auto_detect = !stereo.auto_detect;
                on_off_text("3D input from the file", stereo.auto_detect)
            }
            StereoCommand::Output(output) => {
                stereo.output = output;
                if stereo.mode == ModeSwitch::Off {
                    stereo.mode = ModeSwitch::On;
                }
                format!("3D output: {}", output_text(output))
            }
            StereoCommand::MoveRight(side) => {
                let offset = &mut stereo.offset;
                match side {
                    None => *offset = [0, 0],
                    Some(crate::picture::Side::Left) => offset[0] -= STEREO_STEP,
                    Some(crate::picture::Side::Right) => offset[0] += STEREO_STEP,
                    Some(crate::picture::Side::Top) => offset[1] -= STEREO_STEP,
                    Some(crate::picture::Side::Bottom) => offset[1] += STEREO_STEP,
                }
                for value in offset.iter_mut() {
                    *value = (*value).clamp(-200, 200);
                }
                format!("3D right image: {:+} px, {:+} px", offset[0], offset[1])
            }
            StereoCommand::ToggleInvert => {
                stereo.invert = !stereo.invert;
                on_off_text("3D images flipped", stereo.invert)
            }
            StereoCommand::VrMode(mode) => {
                self.settings.vr.mode = mode;
                self.stereo.turn = [0.0; 3];
                format!(
                    "360° video mode: {}",
                    match mode {
                        ModeSwitch::Off => "off",
                        ModeSwitch::On => "on",
                        ModeSwitch::Auto => "automatic",
                    }
                )
            }
            StereoCommand::VrToggle => {
                let on = !self.vr_active();
                self.settings.vr.mode = if on { ModeSwitch::On } else { ModeSwitch::Off };
                self.stereo.turn = [0.0; 3];
                on_off_text("360° video mode", on)
            }
            StereoCommand::Projection(projection) => {
                self.settings.vr.projection = projection;
                format!("360° projection: {}", projection_text(projection))
            }
            StereoCommand::ToggleFourViews => {
                let vr = &mut self.settings.vr;
                vr.four_views = !vr.four_views;
                on_off_text("360°: four camera angles", vr.four_views)
            }
        };
        self.save_settings();
        self.apply_decoding(engine);
        match self.apply_video_filters(engine) {
            Ok(()) => UiRequest::Osd(notice),
            Err(error) => UiRequest::Message(format!("{notice}: {error}")),
        }
    }

    /// Whether a 3D or 360° command's menu entry is checked.
    #[must_use]
    pub fn stereo_checked(&self, command: StereoCommand) -> Option<bool> {
        let stereo = &self.settings.stereo;
        let vr = &self.settings.vr;
        Some(match command {
            StereoCommand::Mode(mode) => stereo.mode == mode,
            StereoCommand::Input(input) => stereo.input == input,
            StereoCommand::ToggleFullResolution => stereo.full_resolution,
            StereoCommand::ToggleSwap => stereo.swap,
            StereoCommand::ToggleAutoDetect => stereo.auto_detect,
            StereoCommand::Output(output) => stereo.output == output,
            StereoCommand::ToggleInvert => stereo.invert,
            StereoCommand::VrMode(mode) => vr.mode == mode,
            StereoCommand::VrToggle => self.vr_active(),
            StereoCommand::Projection(projection) => vr.projection == projection,
            StereoCommand::ToggleFourViews => vr.four_views,
            StereoCommand::MoveRight(_) => return None,
        })
    }

    /// M12's part of [`App::apply_settings`]: the picture, 3D and 360°.
    pub(crate) fn apply_m12_settings(
        &mut self,
        engine: &rvp_engine::Engine,
        old: &crate::Settings,
    ) {
        self.apply_picture_settings_change(engine, old);
        if self.settings.stereo != old.stereo || self.settings.vr != old.vr {
            self.apply_decoding(engine);
            drop(self.apply_video_filters(engine));
        }
        if self.settings.visual != old.visual {
            self.visual.cycled = None;
            self.apply_visual(engine);
        }
        if self.settings.render != old.render {
            self.apply_render(engine);
            let filters_changed = self.settings.render.matrix != old.render.matrix
                || self.settings.render.deinterlacer != old.render.deinterlacer;
            if filters_changed {
                drop(self.apply_picture_settings(engine));
                drop(self.apply_video_filters(engine));
            }
        }
        if self.settings.deinterlace != old.deinterlace
            && !self.settings.render.deinterlacer.is_empty()
        {
            drop(self.apply_video_filters(engine));
        }
        if self.settings.decoders != old.decoders {
            // mpv takes them for the next decoder it opens.
            for (name, value) in self.settings.decoders.mpv_options() {
                drop(engine.set_text(name, &value));
            }
        }
    }
}

fn on_off_text(name: &str, on: bool) -> String {
    format!("{name}: {}", if on { "on" } else { "off" })
}

/// The output's name in notices.
#[must_use]
pub fn output_text(output: StereoOutput) -> &'static str {
    match output {
        StereoOutput::SideBySide => "side by side",
        StereoOutput::TopBottom => "top and bottom",
        StereoOutput::Left => "left image",
        StereoOutput::Right => "right image",
        StereoOutput::Rows => "row interleave",
        StereoOutput::Columns => "column interleave",
        StereoOutput::RedCyanGray => "anaglyph red/cyan gray",
        StereoOutput::RedCyanHalf => "anaglyph red/cyan half colored",
        StereoOutput::RedCyanColor => "anaglyph red/cyan colored",
        StereoOutput::RedCyanDubois => "anaglyph red/cyan Dubois",
        StereoOutput::GreenMagentaGray => "anaglyph green/magenta gray",
        StereoOutput::GreenMagentaHalf => "anaglyph green/magenta half colored",
        StereoOutput::GreenMagentaColor => "anaglyph green/magenta colored",
        StereoOutput::GreenMagentaDubois => "anaglyph green/magenta Dubois",
        StereoOutput::YellowBlueGray => "anaglyph yellow/blue gray",
        StereoOutput::YellowBlueHalf => "anaglyph yellow/blue half colored",
        StereoOutput::YellowBlueColor => "anaglyph yellow/blue colored",
        StereoOutput::YellowBlueDubois => "anaglyph yellow/blue Dubois",
        StereoOutput::RedBlueGray => "anaglyph red/blue gray",
        StereoOutput::RedGreenGray => "anaglyph red/green gray",
    }
}

fn projection_text(projection: Projection) -> &'static str {
    match projection {
        Projection::Auto => "automatic",
        Projection::Equirectangular => "equirectangular",
        Projection::DualFisheye => "dual fisheye",
        Projection::Cubemap => "cubemap",
        Projection::EquiAngular => "equi-angular cubemap 3x2",
        Projection::EquiAngularStereo => "equi-angular cubemap 2x3 (stereo)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_come_from_metadata_and_names() {
        assert_eq!(
            input_from_metadata("sbs2l"),
            Some((StereoInput::SideBySide, false, false))
        );
        assert_eq!(
            input_from_metadata("abr"),
            Some((StereoInput::TopBottom, true, true))
        );
        assert_eq!(input_from_metadata("mono"), None);
        assert_eq!(
            input_from_name("/films/Avatar.2009.3D.HSBS.mkv"),
            Some((StereoInput::SideBySide, false, false))
        );
        assert_eq!(
            input_from_name("C:\\films\\clip-rl.mp4"),
            Some((StereoInput::SideBySide, false, true))
        );
        assert_eq!(
            input_from_name("/films/clip_full_tab.mkv"),
            Some((StereoInput::TopBottom, true, false))
        );
        assert_eq!(input_from_name("/films/table.mkv"), None);
        assert!(named_360("/clips/Alps 360.mp4"));
        assert!(named_360("/clips/alps_vr360.webm"));
        assert!(!named_360("/clips/3600 seconds.mp4"));
    }

    #[test]
    fn stereo_graphs_pack_the_eyes() {
        let mut stereo = Stereo {
            output: StereoOutput::TopBottom,
            ..Stereo::default()
        };
        assert_eq!(
            stereo_graph(&stereo, (StereoInput::SideBySide, true, true)).as_deref(),
            Some("stereo3d=sbsr:ab2l")
        );
        assert_eq!(
            stereo_graph(&stereo, (StereoInput::Original, true, false)),
            None
        );
        stereo.offset = [2, 0];
        stereo.invert = true;
        let graph =
            stereo_graph(&stereo, (StereoInput::TopBottom, false, false)).unwrap_or_default();
        assert!(
            graph.starts_with("stereo3d=ab2l:sbsl,split[rvpa][rvpb]"),
            "{graph}"
        );
        assert!(
            graph.contains("hflip,crop=w=iw-2:h=ih-0:x=0:y=0,pad=w=iw+2"),
            "{graph}"
        );
        assert!(graph.ends_with("hstack,stereo3d=sbsl:ab2l"), "{graph}");
    }

    #[test]
    fn vr_graphs_turn_the_view() {
        let graph = vr_graph(
            Projection::Equirectangular,
            false,
            90.0,
            (1600, 900),
            [30.0, -10.0, 0.0],
        );
        assert!(
            graph.starts_with("v360=e:flat:h_fov=90:v_fov=58.716"),
            "{graph}"
        );
        assert!(
            graph.ends_with("w=1600:h=900:yaw=30:pitch=-10:roll=0"),
            "{graph}"
        );
        let four = vr_graph(Projection::DualFisheye, true, 90.0, (1600, 900), [0.0; 3]);
        assert!(
            four.contains("v360=dfisheye:flat") && four.contains("yaw=-90"),
            "{four}"
        );
        assert!(
            four.ends_with("xstack=inputs=4:layout=0_0|w0_0|0_h0|w0_h0"),
            "{four}"
        );
        assert_eq!(guess_projection(3840, 2560), Projection::Cubemap);
        assert_eq!(guess_projection(3840, 1920), Projection::Equirectangular);
    }
}
