//! M12's pixel shaders: PotPlayer's two lists (pre-resize and post-resize,
//! each on or off and a stack of shaders in order) over mpv's
//! `glsl-shaders`.
//!
//! Shaders come from three places, listed in this order:
//! - RVP's default set, built into RVP (`crates/app/shaders`, their
//!   sources and licenses in its `SOURCES.md`) and written to the state
//!   folder's `shaders/rvp` when used;
//! - PotPlayer's `PxShader` folder (Windows, read where PotPlayer is
//!   installed, never copied): MPC-HC's HLSL, translated into mpv hooks
//!   (`crate::hlsl`) into `shaders/translated/pre` or `post`;
//! - the user's folders and RVP's own shader folder: mpv's `.glsl` and
//!   `.hook` files as they are, `.hlsl`, `.txt` and `.fx` translated.
//!
//! mpv's own shaders run where their `//!HOOK` lines say, so each is listed
//! under the list of its stage; a translated one can go in either.

use std::path::{Path, PathBuf};

use crate::app::{App, UiRequest};
use crate::hlsl::Stage;

/// RVP's default set: the file, its source and its stage.
pub const BUILTIN: &[(&str, &str, Stage)] = &[
    ("FSR.glsl", include_str!("../shaders/FSR.glsl"), Stage::Pre),
    ("CAS.glsl", include_str!("../shaders/CAS.glsl"), Stage::Pre),
    (
        "CAS-scaled.glsl",
        include_str!("../shaders/CAS-scaled.glsl"),
        Stage::Pre,
    ),
    (
        "NVScaler.glsl",
        include_str!("../shaders/NVScaler.glsl"),
        Stage::Pre,
    ),
    (
        "NVSharpen.glsl",
        include_str!("../shaders/NVSharpen.glsl"),
        Stage::Pre,
    ),
    (
        "FSRCNNX_x2_8-0-4-1.glsl",
        include_str!("../shaders/FSRCNNX_x2_8-0-4-1.glsl"),
        Stage::Pre,
    ),
    (
        "ravu-lite-ar-r4.hook",
        include_str!("../shaders/ravu-lite-ar-r4.hook"),
        Stage::Pre,
    ),
    (
        "KrigBilateral.glsl",
        include_str!("../shaders/KrigBilateral.glsl"),
        Stage::Pre,
    ),
    (
        "SSimSuperRes.glsl",
        include_str!("../shaders/SSimSuperRes.glsl"),
        Stage::Post,
    ),
    (
        "SSimDownscaler.glsl",
        include_str!("../shaders/SSimDownscaler.glsl"),
        Stage::Post,
    ),
    (
        "adaptive-sharpen.glsl",
        include_str!("../shaders/adaptive-sharpen.glsl"),
        Stage::Post,
    ),
];

/// A shader of the lists.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShaderEntry {
    /// The name shown: the file's name without its extension.
    pub name: String,
    /// The file (a built-in's is in the state folder).
    pub path: PathBuf,
    /// mpv's own shader with its stage, or `None` for HLSL (either list).
    pub stage: Option<Stage>,
}

/// One of the Pixel Shaders lists' commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaderCommand {
    /// Use Pre-/Post-Resize Pixel Shaders.
    Toggle(Stage),
    /// Enable Shader Stacker: the list on.
    On(Stage),
    /// Do not use: the list off.
    Off(Stage),
    /// Stack...: the settings page with the lists.
    Stack(Stage),
    Reload,
    OpenFolder,
}

/// A file's kind by its extension: mpv's GLSL, MPC-HC's HLSL, or neither.
fn kind(path: &Path) -> Option<bool> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "glsl" | "hook" => Some(true),
        "hlsl" | "txt" | "fx" | "psh" => Some(false),
        _ => None,
    }
}

/// The stage of an mpv shader from its first `//!HOOK` line.
#[must_use]
pub fn glsl_stage(source: &str) -> Stage {
    let hook = source
        .lines()
        .find_map(|line| line.trim().strip_prefix("//!HOOK"))
        .map(str::trim)
        .unwrap_or_default();
    if matches!(hook, "OUTPUT" | "POSTKERNEL" | "SCALED" | "PREKERNEL") {
        Stage::Post
    } else {
        Stage::Pre
    }
}

/// The shader files of a folder, by name.
fn folder_shaders(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && kind(path).is_some())
        .collect();
    files.sort();
    files
}

impl App {
    /// PotPlayer's `PxShader` folder: beside the skins folder of the
    /// configured `.dsf`, or where PotPlayer installs itself on Windows.
    #[must_use]
    pub fn potplayer_shader_folder(&self) -> Option<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(potplayer) = self
            .settings
            .skin
            .as_ref()
            .and_then(|skin| skin.parent())
            .and_then(Path::parent)
        {
            candidates.push(potplayer.join("PxShader"));
        }
        if cfg!(windows) {
            candidates.push(PathBuf::from(r"C:\Program Files\DAUM\PotPlayer\PxShader"));
            candidates.push(PathBuf::from(
                r"C:\Program Files (x86)\DAUM\PotPlayer\PxShader",
            ));
        }
        candidates.into_iter().find(|folder| folder.is_dir())
    }

    /// Every shader the lists offer, in their order: RVP's, PotPlayer's,
    /// then the user's folders and RVP's shader folder.
    #[must_use]
    pub fn shader_catalogue(&self) -> Vec<ShaderEntry> {
        let mut entries: Vec<ShaderEntry> = BUILTIN
            .iter()
            .map(|(file, _, stage)| ShaderEntry {
                name: file_stem(file),
                path: self.builtin_shader_folder().join(file),
                stage: Some(*stage),
            })
            .collect();
        let mut folders: Vec<PathBuf> = self.potplayer_shader_folder().into_iter().collect();
        folders.extend(self.settings.shaders.folders.iter().cloned());
        folders.push(self.shader_folder());
        for folder in folders {
            for path in folder_shaders(&folder) {
                if entries.iter().any(|entry| entry.path == path) {
                    continue;
                }
                let stage = match kind(&path) {
                    Some(true) => Some(
                        std::fs::read_to_string(&path)
                            .map_or(Stage::Pre, |source| glsl_stage(&source)),
                    ),
                    _ => None,
                };
                entries.push(ShaderEntry {
                    name: file_stem(&path.to_string_lossy()),
                    path,
                    stage,
                });
            }
        }
        entries
    }

    /// The catalogue's shaders a list offers: its stage's and the HLSL
    /// ones.
    #[must_use]
    pub fn shader_choices(&self, stage: Stage) -> Vec<ShaderEntry> {
        self.shader_catalogue()
            .into_iter()
            .filter(|entry| entry.stage.is_none_or(|own| own == stage))
            .collect()
    }

    fn builtin_shader_folder(&self) -> PathBuf {
        self.state_dir().join("shaders").join("rvp")
    }

    /// A list's files.
    pub(crate) fn shader_list(&self, stage: Stage) -> &Vec<PathBuf> {
        match stage {
            Stage::Pre => &self.settings.shaders.files,
            Stage::Post => &self.settings.shaders.post_files,
        }
    }

    /// Whether a list is on.
    #[must_use]
    pub fn shaders_on(&self, stage: Stage) -> bool {
        match stage {
            Stage::Pre => self.settings.shaders.enabled,
            Stage::Post => self.settings.shaders.post_enabled,
        }
    }

    fn set_shaders_on(&mut self, stage: Stage, on: bool) {
        match stage {
            Stage::Pre => self.settings.shaders.enabled = on,
            Stage::Post => self.settings.shaders.post_enabled = on,
        }
    }

    /// The file mpv gets for a list's shader: a built-in written out, an
    /// HLSL shader translated; `Err` with the reason when it cannot run.
    pub(crate) fn shader_file(&self, path: &Path, stage: Stage) -> Result<PathBuf, String> {
        let name = file_stem(&path.to_string_lossy());
        if path.starts_with(self.builtin_shader_folder())
            && let Some((file, source, _)) = BUILTIN
                .iter()
                .find(|(file, _, _)| path.file_name().is_some_and(|own| own == *file))
        {
            write_if_changed(&self.builtin_shader_folder().join(file), source)?;
            return Ok(path.to_path_buf());
        }
        match kind(path) {
            Some(true) => Ok(path.to_path_buf()),
            Some(false) => {
                let source = std::fs::read_to_string(path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                let title = crate::hlsl::declared_name(&source).unwrap_or_else(|| name.clone());
                let glsl = crate::hlsl::translate(&source, stage, &title)
                    .map_err(|reason| format!("{name}: {reason}"))?;
                let folder =
                    self.state_dir()
                        .join("shaders")
                        .join("translated")
                        .join(match stage {
                            Stage::Pre => "pre",
                            Stage::Post => "post",
                        });
                let target = folder.join(format!("{name}.glsl"));
                write_if_changed(&target, &glsl)?;
                Ok(target)
            }
            None => Err(format!("{name}: not a shader file")),
        }
    }

    /// mpv's shader list for both lists; the shaders that cannot run are
    /// left out and named.
    pub(crate) fn shader_files(&self) -> (Vec<PathBuf>, Vec<String>) {
        let mut files = Vec::new();
        let mut refused = Vec::new();
        for stage in [Stage::Pre, Stage::Post] {
            // The GPU power budget may have taken both lists off.
            if !self.shaders_on(stage) || self.gpu_step_taken(crate::gpu_power::Step::Shaders) {
                continue;
            }
            for path in self.shader_list(stage) {
                match self.shader_file(path, stage) {
                    Ok(file) => files.push(file),
                    Err(reason) => refused.push(reason),
                }
            }
        }
        (files, refused)
    }

    /// A list's shader `index` of [`App::shader_choices`] in or out of the
    /// list (a menu item); the list goes on with it.
    pub fn pick_shader(
        &mut self,
        engine: &rvp_engine::Engine,
        stage: Stage,
        index: usize,
    ) -> Option<UiRequest> {
        let entry = self.shader_choices(stage).into_iter().nth(index)?;
        let list = match stage {
            Stage::Pre => &mut self.settings.shaders.files,
            Stage::Post => &mut self.settings.shaders.post_files,
        };
        let text = if let Some(at) = list.iter().position(|path| *path == entry.path) {
            list.remove(at);
            format!("Shader off: {}", entry.name)
        } else {
            list.push(entry.path.clone());
            format!("Shader on: {}", entry.name)
        };
        self.set_shaders_on(stage, true);
        self.save_settings();
        Some(self.shaders_applied(engine, text))
    }

    /// Whether a list's shader `index` is in its list (its check mark).
    #[must_use]
    pub fn shader_picked(&self, stage: Stage, path: &Path) -> bool {
        self.shaders_on(stage) && self.shader_list(stage).iter().any(|file| file == path)
    }

    /// The Pixel Shaders menu's commands.
    pub(crate) fn shader_command(
        &mut self,
        engine: &rvp_engine::Engine,
        command: ShaderCommand,
    ) -> UiRequest {
        let list_name = |stage: Stage| match stage {
            Stage::Pre => "Pre-resize shaders",
            Stage::Post => "Post-resize shaders",
        };
        let text = match command {
            ShaderCommand::Toggle(stage) => {
                let on = !self.shaders_on(stage);
                self.set_shaders_on(stage, on);
                format!("{}: {}", list_name(stage), if on { "on" } else { "off" })
            }
            ShaderCommand::On(stage) => {
                self.set_shaders_on(stage, true);
                format!(
                    "{}: on ({})",
                    list_name(stage),
                    self.shader_list(stage).len()
                )
            }
            ShaderCommand::Off(stage) => {
                self.set_shaders_on(stage, false);
                format!("{}: off", list_name(stage))
            }
            ShaderCommand::Stack(_) => return UiRequest::Preferences(Some("shaders")),
            ShaderCommand::Reload => "Shaders reloaded".to_owned(),
            ShaderCommand::OpenFolder => {
                let folder = self.shader_folder();
                drop(std::fs::create_dir_all(&folder));
                return UiRequest::OpenExternal(folder);
            }
        };
        self.save_settings();
        self.shaders_applied(engine, text)
    }

    /// Applies the lists and says what happened (or which shaders could
    /// not run).
    fn shaders_applied(&mut self, engine: &rvp_engine::Engine, text: String) -> UiRequest {
        match self.apply_shaders(engine) {
            Ok(refused) if refused.is_empty() => UiRequest::Osd(text),
            Ok(refused) => UiRequest::Message(format!("{text}; not used: {}", refused.join("; "))),
            Err(error) => UiRequest::Message(format!("Shaders: {error}")),
        }
    }

    /// The check mark of a shader command.
    pub(crate) fn shader_checked(&self, command: ShaderCommand) -> Option<bool> {
        match command {
            ShaderCommand::Toggle(stage) | ShaderCommand::On(stage) => Some(self.shaders_on(stage)),
            ShaderCommand::Off(stage) => Some(!self.shaders_on(stage)),
            _ => None,
        }
    }
}

/// A file's name without its extension.
fn file_stem(path: &str) -> String {
    let name = crate::view::file_name(path);
    Path::new(name).file_stem().map_or_else(
        || name.to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    )
}

/// Writes `text` to `path` unless it holds it already.
fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    crate::storage::write_atomic(path, text.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_set_is_listed_by_stage() {
        for (file, source, stage) in BUILTIN {
            assert_eq!(glsl_stage(source), *stage, "{file}");
            assert!(source.contains("//!HOOK"), "{file}");
        }
        assert_eq!(glsl_stage("//!HOOK OUTPUT\n"), Stage::Post);
        assert_eq!(glsl_stage("//!HOOK MAIN\n"), Stage::Pre);
        assert_eq!(kind(Path::new("a.HLSL")), Some(false));
        assert_eq!(kind(Path::new("a.hook")), Some(true));
        assert_eq!(kind(Path::new("a.png")), None);
    }
}
