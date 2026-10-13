//! RVP's own video processing (M6): colour adjustments, deinterlacing,
//! video effects, rotation, deband, GLSL shaders and VapourSynth script
//! slots.
//!
//! Rules (goal C's rule 4 as changed for M6):
//!
//! * RVP sets only its own video filters, each under its own label
//!   (`@rvp...`) and added and removed by that label. Other filters, SVP's
//!   `@svp` above all, are never replaced, reordered or removed.
//! * RVP's filters run first, in [`VideoEffect::ALL`]'s order with the
//!   VapourSynth script last, so they come before `@svp`: a change removes
//!   RVP's filters and puts them back in front of the chain (`vf pre`).
//! * Deinterlacing is mpv's own (`deinterlace`), which mpv applies before
//!   every user filter. Colour adjustments, deband, rotation and shaders
//!   belong to the renderer and need no filter.
//! * Filters that run on the CPU need the decoded frames in memory: while
//!   one of them, a VapourSynth script or SVP mode is on, decoding uses the
//!   copy-back variant of the chosen hardware decoder
//!   ([`crate::svp::copy_back`]).
//!
//! PotPlayer's `.vpy` scripts call `vs.get_core()`, which VapourSynth R73
//! (SVP's runtime) no longer has (spike S2): RVP runs them through a
//! generated wrapper that defines it, loads the listed plugins (PotPlayer's
//! script needs mvtools, which SVP does not ship) and executes the script in
//! the same globals, so mpv's `video_in`, `container_fps` and `display_fps`
//! reach it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rvp_engine::{Engine, EngineError};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// mpv's deinterlacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Deinterlace {
    /// Interlaced frames only (mpv 0.38 and later; older mpv: off).
    #[default]
    Auto,
    On,
    Off,
}

impl Deinterlace {
    #[must_use]
    pub fn mpv_value(self) -> &'static str {
        match self {
            Deinterlace::Auto => "auto",
            Deinterlace::On => "yes",
            Deinterlace::Off => "no",
        }
    }

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Deinterlace::Auto => "Auto",
            Deinterlace::On => "On",
            Deinterlace::Off => "Off",
        }
    }
}

/// A video effect RVP runs as its own filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoEffect {
    Deblock,
    Denoise,
    Soften,
    Sharpen,
    FlipVertical,
    FlipHorizontal,
}

impl VideoEffect {
    /// Every effect, in the order the filters run.
    pub const ALL: [VideoEffect; 6] = [
        VideoEffect::Deblock,
        VideoEffect::Denoise,
        VideoEffect::Soften,
        VideoEffect::Sharpen,
        VideoEffect::FlipVertical,
        VideoEffect::FlipHorizontal,
    ];

    /// The filter's label in mpv's chain.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            VideoEffect::Deblock => "rvpdeblock",
            VideoEffect::Denoise => "rvpdenoise",
            VideoEffect::Soften => "rvpsoften",
            VideoEffect::Sharpen => "rvpsharpen",
            VideoEffect::FlipVertical => "rvpvflip",
            VideoEffect::FlipHorizontal => "rvphflip",
        }
    }

    /// The FFmpeg filter graph.
    #[must_use]
    pub fn graph(self) -> &'static str {
        match self {
            VideoEffect::Deblock => "deblock",
            VideoEffect::Denoise => "hqdn3d",
            VideoEffect::Soften => "gblur=sigma=1.0",
            VideoEffect::Sharpen => "unsharp=5:5:0.8:5:5:0.0",
            VideoEffect::FlipVertical => "vflip",
            VideoEffect::FlipHorizontal => "hflip",
        }
    }

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            VideoEffect::Deblock => "Deblock",
            VideoEffect::Denoise => "Denoise 3D",
            VideoEffect::Soften => "Soften",
            VideoEffect::Sharpen => "Sharpen",
            VideoEffect::FlipVertical => "Flip vertical",
            VideoEffect::FlipHorizontal => "Flip horizontal",
        }
    }
}

/// The label of the VapourSynth script slot's filter.
pub const SCRIPT_LABEL: &str = "rvpvs";

/// One of the colour adjustments (mpv's renderer properties, -100 to 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorKind {
    Brightness,
    Contrast,
    Saturation,
    Hue,
    Gamma,
}

impl ColorKind {
    pub const ALL: [ColorKind; 5] = [
        ColorKind::Brightness,
        ColorKind::Contrast,
        ColorKind::Saturation,
        ColorKind::Hue,
        ColorKind::Gamma,
    ];

    #[must_use]
    pub fn property(self) -> &'static str {
        match self {
            ColorKind::Brightness => "brightness",
            ColorKind::Contrast => "contrast",
            ColorKind::Saturation => "saturation",
            ColorKind::Hue => "hue",
            ColorKind::Gamma => "gamma",
        }
    }

    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            ColorKind::Brightness => "Brightness",
            ColorKind::Contrast => "Contrast",
            ColorKind::Saturation => "Saturation",
            ColorKind::Hue => "Hue",
            ColorKind::Gamma => "Gamma",
        }
    }
}

/// A colour control change (PotPlayer's 1 % steps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorChange {
    Down(ColorKind),
    Up(ColorKind),
    Reset(ColorKind),
    /// "Disable/Last used Color Controls": all adjustments off and back.
    Toggle,
}

/// Colour adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Color {
    /// Off: the picture as decoded; the values are kept for "last used".
    pub enabled: bool,
    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
    pub hue: i32,
    pub gamma: i32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            enabled: true,
            brightness: 0,
            contrast: 0,
            saturation: 0,
            hue: 0,
            gamma: 0,
        }
    }
}

impl Color {
    #[must_use]
    pub fn get(&self, kind: ColorKind) -> i32 {
        match kind {
            ColorKind::Brightness => self.brightness,
            ColorKind::Contrast => self.contrast,
            ColorKind::Saturation => self.saturation,
            ColorKind::Hue => self.hue,
            ColorKind::Gamma => self.gamma,
        }
    }

    pub fn set(&mut self, kind: ColorKind, value: i32) {
        let value = value.clamp(-100, 100);
        match kind {
            ColorKind::Brightness => self.brightness = value,
            ColorKind::Contrast => self.contrast = value,
            ColorKind::Saturation => self.saturation = value,
            ColorKind::Hue => self.hue = value,
            ColorKind::Gamma => self.gamma = value,
        }
    }

    /// The value mpv gets: the setting, or 0 while the controls are off.
    #[must_use]
    pub fn applied(&self, kind: ColorKind) -> i32 {
        if self.enabled { self.get(kind) } else { 0 }
    }
}

/// The screen rotation in degrees (mpv's `video-rotate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rotation {
    Cycle,
    To(u16),
}

/// GLSL shaders (mpv's `glsl-shaders`), applied in the listed order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Shaders {
    pub enabled: bool,
    pub files: Vec<PathBuf>,
}

/// Shader commands (PotPlayer's "Pixel Shaders" menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaderChange {
    Toggle,
    On,
    Off,
    /// Applies the list again (files changed on disk).
    Reload,
    OpenFolder,
}

/// VapourSynth script slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct VapourSynth {
    /// The active script runs.
    pub enabled: bool,
    /// The scripts to choose from (PotPlayer's `.vpy` files work).
    pub slots: Vec<PathBuf>,
    /// The slot that runs; unset: none.
    pub script: Option<PathBuf>,
    /// Plugins loaded before the script (mvtools for PotPlayer's).
    pub plugins: Vec<PathBuf>,
    /// The VSScript library; unset: SVP's runtime when installed, else the
    /// system's (read at start).
    pub library: Option<PathBuf>,
    /// mpv's `buffered-frames` for the filter.
    pub buffered_frames: u32,
}

impl Default for VapourSynth {
    fn default() -> Self {
        Self {
            enabled: false,
            slots: Vec::new(),
            script: None,
            plugins: Vec::new(),
            library: None,
            buffered_frames: 4,
        }
    }
}

/// A Python string literal for a path.
fn python_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// The wrapper that runs `script` with today's VapourSynth (see the module
/// documentation).
#[must_use]
pub fn script_wrapper(script: &Path, plugins: &[PathBuf]) -> String {
    let script = python_string(&script.to_string_lossy());
    let mut text = String::from(
        "# Generated by RVP: runs a PotPlayer VapourSynth script with the\n\
         # VapourSynth in use (R73 removed get_core) and loads its plugins.\n\
         import vapoursynth as vs\n\
         if not hasattr(vs, 'get_core'):\n\
         \x20   vs.get_core = lambda *args, **kwargs: vs.core\n",
    );
    for plugin in plugins {
        let _ = write!(
            text,
            "try:\n    vs.core.std.LoadPlugin({})\nexcept vs.Error:\n    pass\n",
            python_string(&plugin.to_string_lossy())
        );
    }
    let _ = write!(
        text,
        "with open({script}, encoding='utf-8-sig') as source:\n\
         \x20   exec(compile(source.read(), {script}, 'exec'))\n"
    );
    text
}

/// The `vf` entry for the VapourSynth filter on `wrapper`: mpv splits its
/// arguments at `:`, so the path is given with its length (`%LEN%`).
#[must_use]
pub fn script_entry(wrapper: &Path, buffered_frames: u32) -> String {
    let path = wrapper.to_string_lossy().replace('\\', "/");
    format!(
        "@{SCRIPT_LABEL}:vapoursynth=file=%{}%{path}:buffered-frames={}",
        path.len(),
        buffered_frames.clamp(1, 64)
    )
}

impl App {
    /// RVP's video filters, in the order they run.
    #[must_use]
    pub fn video_filters(&self) -> Vec<(String, String)> {
        let mut filters: Vec<(String, String)> = VideoEffect::ALL
            .iter()
            .filter(|effect| self.settings.effects.contains(effect))
            .map(|effect| {
                (
                    effect.label().to_owned(),
                    format!("@{}:lavfi=[{}]", effect.label(), effect.graph()),
                )
            })
            .collect();
        let vs = &self.settings.vapoursynth;
        if vs.enabled && vs.script.is_some() {
            let wrapper = self.state_dir().join("vapoursynth").join("rvp-script.vpy");
            filters.push((
                SCRIPT_LABEL.to_owned(),
                script_entry(&wrapper, vs.buffered_frames),
            ));
        }
        filters
    }

    /// True while a filter that needs decoded frames in memory runs.
    #[must_use]
    pub fn needs_copy_back(&self) -> bool {
        !self.video_filters().is_empty() || crate::svp::svp_mode(&self.settings)
    }

    /// Puts RVP's video filters in front of mpv's chain, in their order,
    /// leaving every other filter where it is.
    ///
    /// # Errors
    /// Fails when the VapourSynth wrapper cannot be written or libmpv
    /// rejects a filter.
    pub fn apply_video_filters(&self, engine: &Engine) -> Result<(), String> {
        let vs = &self.settings.vapoursynth;
        if vs.enabled
            && let Some(script) = &vs.script
        {
            let wrapper = self.state_dir().join("vapoursynth").join("rvp-script.vpy");
            crate::storage::write_atomic(&wrapper, script_wrapper(script, &vs.plugins).as_bytes())
                .map_err(|error| format!("{}: {error}", wrapper.display()))?;
        }
        // Only labels in the chain: mpv logs a warning for the others.
        let chain = engine.get_text("vf").unwrap_or_default();
        let labels = VideoEffect::ALL
            .iter()
            .map(|effect| effect.label())
            .chain([SCRIPT_LABEL]);
        for label in labels {
            if chain.contains(&format!("@{label}:")) {
                drop(engine.remove_video_filter(label));
            }
        }
        for (_, entry) in self.video_filters().iter().rev() {
            engine
                .prepend_video_filter(entry)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Sets the renderer's colour, deinterlacing, deband and shaders.
    ///
    /// # Errors
    /// Fails when libmpv rejects a value (deinterlacing's `auto` falls back
    /// to off on mpv before 0.38).
    pub fn apply_picture_settings(&self, engine: &Engine) -> Result<(), EngineError> {
        let color = &self.settings.color;
        for kind in ColorKind::ALL {
            engine.set_double(kind.property(), f64::from(color.applied(kind)))?;
        }
        let deinterlace = self.settings.deinterlace.mpv_value();
        if engine.set_text("deinterlace", deinterlace).is_err() {
            engine.set_text("deinterlace", "no")?;
        }
        // Only the gpu renderers have deband; the headless output has not.
        drop(engine.set_flag("deband", self.settings.deband));
        self.apply_shaders(engine)
    }

    /// Sets mpv's shader list from the settings.
    ///
    /// # Errors
    /// Fails when libmpv rejects the list.
    pub fn apply_shaders(&self, engine: &Engine) -> Result<(), EngineError> {
        engine.command("change-list", &["glsl-shaders", "clr", ""])?;
        if self.settings.shaders.enabled {
            for file in &self.settings.shaders.files {
                engine.command(
                    "change-list",
                    &["glsl-shaders", "append", &file.to_string_lossy()],
                )?;
            }
        }
        Ok(())
    }

    /// Where RVP keeps shader files by default: `shaders` next to the
    /// settings, or in the state folder.
    #[must_use]
    pub fn shader_folder(&self) -> PathBuf {
        self.settings_path().and_then(Path::parent).map_or_else(
            || self.state_dir().join("shaders"),
            |dir| dir.join("shaders"),
        )
    }

    /// The video commands of M6.
    pub(crate) fn video_command(
        &mut self,
        engine: &Engine,
        action: crate::action::Action,
    ) -> Option<UiRequest> {
        use crate::action::Action;
        match action {
            Action::Color(change) => Some(self.change_color(engine, change)),
            Action::DeinterlaceCycle => {
                let next = match self.settings.deinterlace {
                    Deinterlace::Auto => Deinterlace::On,
                    Deinterlace::On => Deinterlace::Off,
                    Deinterlace::Off => Deinterlace::Auto,
                };
                Some(self.set_deinterlace(engine, next))
            }
            Action::SetDeinterlace(mode) => Some(self.set_deinterlace(engine, mode)),
            Action::VideoEffect(effect) => Some(self.toggle_effect(engine, effect)),
            Action::ToggleDeband => {
                self.settings.deband = !self.settings.deband;
                drop(engine.set_flag("deband", self.settings.deband));
                self.save_settings();
                Some(on_off("Deband", self.settings.deband))
            }
            Action::Rotate(rotation) => Some(self.rotate(engine, rotation)),
            Action::Shaders(change) => Some(self.shaders(engine, change)),
            Action::ToggleVapourSynth => Some(self.toggle_script(engine)),
            _ => None,
        }
    }

    fn change_color(&mut self, engine: &Engine, change: ColorChange) -> UiRequest {
        let color = &mut self.settings.color;
        let text = match change {
            ColorChange::Toggle => {
                color.enabled = !color.enabled;
                if color.enabled {
                    "Color controls: last used".to_owned()
                } else {
                    "Color controls: off".to_owned()
                }
            }
            ColorChange::Down(kind) | ColorChange::Up(kind) | ColorChange::Reset(kind) => {
                let value = match change {
                    ColorChange::Down(_) => color.get(kind) - 1,
                    ColorChange::Up(_) => color.get(kind) + 1,
                    _ => 0,
                };
                color.set(kind, value);
                color.enabled = true;
                format!("{}: {}", kind.text(), color.get(kind))
            }
        };
        for kind in ColorKind::ALL {
            drop(engine.set_double(
                kind.property(),
                f64::from(self.settings.color.applied(kind)),
            ));
        }
        self.save_settings();
        UiRequest::Osd(text)
    }

    /// Sets one colour adjustment from a slider (-100 to 100).
    pub fn set_color(&mut self, engine: &Engine, kind: ColorKind, value: i32) {
        self.settings.color.set(kind, value);
        self.settings.color.enabled = true;
        for kind in ColorKind::ALL {
            drop(engine.set_double(
                kind.property(),
                f64::from(self.settings.color.applied(kind)),
            ));
        }
        self.save_settings();
    }

    fn set_deinterlace(&mut self, engine: &Engine, mode: Deinterlace) -> UiRequest {
        self.settings.deinterlace = mode;
        if engine.set_text("deinterlace", mode.mpv_value()).is_err() {
            drop(engine.set_text("deinterlace", "no"));
        }
        self.save_settings();
        UiRequest::Osd(format!("Deinterlacing: {}", mode.text()))
    }

    fn toggle_effect(&mut self, engine: &Engine, effect: VideoEffect) -> UiRequest {
        let on = !self.settings.effects.contains(&effect);
        if on {
            self.settings.effects.insert(effect);
        } else {
            self.settings.effects.remove(&effect);
        }
        self.save_settings();
        self.apply_decoding(engine);
        match self.apply_video_filters(engine) {
            Ok(()) => on_off(effect.text(), on),
            Err(error) => UiRequest::Message(format!("{}: {error}", effect.text())),
        }
    }

    fn rotate(&mut self, engine: &Engine, rotation: Rotation) -> UiRequest {
        let current = self.session.rotation;
        let next = match rotation {
            Rotation::Cycle => (current + 90) % 360,
            Rotation::To(degrees) => degrees % 360,
        };
        self.session.rotation = next;
        // An integer choice: mpv takes it as text, not as a double.
        drop(engine.set_text("video-rotate", &next.to_string()));
        UiRequest::Osd(format!("Rotation: {next}°"))
    }

    fn shaders(&mut self, engine: &Engine, change: ShaderChange) -> UiRequest {
        let shaders = &mut self.settings.shaders;
        match change {
            ShaderChange::Toggle => shaders.enabled = !shaders.enabled,
            ShaderChange::On => shaders.enabled = true,
            ShaderChange::Off => shaders.enabled = false,
            ShaderChange::Reload => {}
            ShaderChange::OpenFolder => {
                let folder = self.shader_folder();
                drop(std::fs::create_dir_all(&folder));
                return UiRequest::OpenExternal(folder);
            }
        }
        self.save_settings();
        let count = self.settings.shaders.files.len();
        match self.apply_shaders(engine) {
            Ok(()) if !self.settings.shaders.enabled => UiRequest::Osd("Shaders: off".to_owned()),
            Ok(()) => UiRequest::Osd(format!("Shaders: on ({count})")),
            Err(error) => UiRequest::Message(format!("Shaders: {error}")),
        }
    }

    fn toggle_script(&mut self, engine: &Engine) -> UiRequest {
        let vs = &mut self.settings.vapoursynth;
        if vs.script.is_none() {
            return UiRequest::Osd("VapourSynth: no script chosen (settings)".to_owned());
        }
        vs.enabled = !vs.enabled;
        let on = vs.enabled;
        self.save_settings();
        self.apply_decoding(engine);
        match self.apply_video_filters(engine) {
            Ok(()) => on_off("VapourSynth script", on),
            Err(error) => UiRequest::Message(format!("VapourSynth: {error}")),
        }
    }

    /// The decoder mpv uses: the setting, or its copy-back variant while a
    /// CPU filter, a script or SVP mode needs frames in memory.
    #[must_use]
    pub fn effective_hwdec(&self) -> String {
        if self.needs_copy_back() {
            crate::svp::copy_back(&self.settings.hwdec)
        } else {
            self.settings.hwdec.clone()
        }
    }

    /// Sets mpv's decoder when it should change.
    pub fn apply_decoding(&self, engine: &Engine) {
        let hwdec = self.effective_hwdec();
        if engine.get_text("hwdec").as_deref() != Some(hwdec.as_str()) {
            drop(engine.set_text("hwdec", &hwdec));
        }
    }
}

fn on_off(name: &str, on: bool) -> UiRequest {
    UiRequest::Osd(format!("{name}: {}", if on { "on" } else { "off" }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_defines_get_core_loads_plugins_and_runs_the_script() {
        let text = script_wrapper(
            Path::new("C:\\Program Files\\DAUM\\PotPlayer\\x's.vpy"),
            &[PathBuf::from("/opt/mvtools/libmvtools.so")],
        );
        assert!(text.contains("vs.get_core = lambda"));
        assert!(text.contains("vs.core.std.LoadPlugin('/opt/mvtools/libmvtools.so')"));
        assert!(text.contains(
            "open('C:\\\\Program Files\\\\DAUM\\\\PotPlayer\\\\x\\'s.vpy', encoding='utf-8-sig')"
        ));
        assert!(text.contains("exec(compile(source.read()"));
    }

    #[test]
    fn script_entries_quote_the_path() {
        let entry = script_entry(Path::new("C:\\state\\rvp-script.vpy"), 4);
        assert_eq!(
            entry,
            "@rvpvs:vapoursynth=file=%23%C:/state/rvp-script.vpy:buffered-frames=4"
        );
    }

    #[test]
    fn colour_values_stay_in_range_and_switch_off_together() {
        let mut color = Color::default();
        color.set(ColorKind::Hue, 250);
        assert_eq!(color.get(ColorKind::Hue), 100);
        color.set(ColorKind::Brightness, -7);
        color.enabled = false;
        assert_eq!(color.applied(ColorKind::Brightness), 0);
        color.enabled = true;
        assert_eq!(color.applied(ColorKind::Brightness), -7);
    }
}
