//! RIFE through NVIDIA's TensorRT (M18, the owner, 2026-11-07;
//! `docs/spikes/m18-tensorrt.md`). NVIDIA's licence lets RVP neither ship
//! its runtime in RVP's packages nor make it subject to RVP's GPL, so RVP
//! downloads it from NVIDIA when the user asks and has accepted NVIDIA's
//! terms: TensorRT's archive and CUDA's runtime and NVRTC archives
//! ([`ARCHIVES`]), each checked by its size and pinned SHA-256. Of them it
//! keeps what RIFE needs on this card (the builder resource of its
//! architecture only), next to vs-mlrt's plugin, its `trtexec` and RIFE's
//! model that RVP's package carries (`vapoursynth/vs-mlrt`), laid out as
//! vs-mlrt expects them, in `<state>/tensorrt`:
//!
//! * vs-mlrt's plugin ([`VSTRT`]), which loads TensorRT ([`NVINFER`]) from
//!   `vsmlrt-cuda/` (Windows: by its path; Linux: by its run path);
//! * `vsmlrt-cuda/`: NVIDIA's libraries and `trtexec`, which builds the
//!   engine for the card (a process of its own, so RVP's narrow library
//!   search does not apply to it);
//! * `models/rife/`, `engines/` (built engines), `licences/` (NVIDIA's) and
//!   [`INSTALLED`].
//!
//! The downloads go through the system's `curl` (Windows 10 and later have
//! it), resume after a cancel, and are deleted once unpacked.
//!
//! When RVP starts with TensorRT installed, a thread of its own asks
//! `nvidia-smi` which builder resource the card needs: after a change of
//! card (the host's RTX 30-series card, `sm86`, gave way to an RTX 50-series card,
//! `sm120`, on 2026-11-07) the installation is another card's, RIFE
//! (TensorRT) does not run, and the Motion Smoothing page offers the
//! download again. A new installation removes the files of the one before
//! that it does not keep, and its engines when the card is another.
//! `RVP_TENSORRT_MIRROR` (a folder URL, `file://` too) replaces NVIDIA's
//! servers in tests.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One file to keep from an archive: its path there, with `{sm}` for the
/// card's architecture (`sm86`), and its name in the install folder,
/// relative to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub path: String,
    pub name: String,
}

/// An archive RVP downloads: where from, its size and SHA-256, and what
/// it keeps of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub members: Vec<Member>,
}

impl Archive {
    /// The archive's file name (the last part of its URL).
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.url.rsplit('/').next().unwrap_or(&self.url)
    }
}

/// TensorRT's version: the plugin RVP's package carries is built for it.
pub const VERSION: &str = "10.15.1.29";

/// The builder resources TensorRT's archive has, by architecture; any
/// other card builds from PTX.
pub const BUILDERS: [&str; 7] = ["sm75", "sm80", "sm86", "sm89", "sm90", "sm100", "sm120"];

/// What the install folder holds when it is complete.
pub const INSTALLED: &str = "installed.json";

/// NVIDIA's archives for this system: TensorRT 10.15.1 for CUDA 13.1 and
/// CUDA 13.2.51's runtime and NVRTC (CUDA 13's minor versions are
/// compatible), which vs-mlrt v15.15's plugin (Windows) and
/// `tools/vapoursynth/build-vstrt-linux.sh`'s (Linux) are built for.
/// Not TensorRT 10.16: its engines computed RIFE v4.6 wrongly on the
/// host's RTX 50-series card (`sm120`, 2026-11-08: a fine grid over the whole
/// picture, fp16 and fp32 alike), where 10.15.1 and 10.8 match onnxruntime
/// on the processor (`docs/spikes/m18-tensorrt.md`). Windows: NVIDIA's zip
/// and CUDA's redistributable archives (their SHA-256 in NVIDIA's
/// `redistrib_13.2.0.json`; TensorRT's, which NVIDIA does not publish,
/// taken on 2026-11-08). Linux: NVIDIA's wheels (zips too; their SHA-256 in
/// NVIDIA's package index and the Python Package Index's).
#[must_use]
pub fn archives() -> Vec<Archive> {
    if cfg!(windows) {
        windows_archives()
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        linux_archives()
    } else {
        Vec::new()
    }
}

fn member(path: &str, name: &str) -> Member {
    Member {
        path: path.to_owned(),
        name: name.to_owned(),
    }
}

/// Linux's archives ([`archives`]): NVIDIA's wheels, of 3.8 GB in all.
fn linux_archives() -> Vec<Archive> {
    let libs = "tensorrt_libs";
    let cuda = "nvidia/cu13/lib";
    vec![
        Archive {
            url: "https://pypi.nvidia.com/tensorrt-cu13-libs/tensorrt_cu13_libs-10.15.1.29-py2.py3-none-manylinux_2_28_x86_64.whl".to_owned(),
            size: 3_713_901_680,
            sha256: "0619b60ba97425909c18442eb36cbe7f99b1a7eeb4ebf9c6b278e6e2c5ac6c3b".to_owned(),
            members: vec![
                member(&format!("{libs}/libnvinfer.so.10"), "vsmlrt-cuda/libnvinfer.so.10"),
                member(&format!("{libs}/libnvinfer_plugin.so.10"), "vsmlrt-cuda/libnvinfer_plugin.so.10"),
                member(&format!("{libs}/libnvonnxparser.so.10"), "vsmlrt-cuda/libnvonnxparser.so.10"),
                member(
                    &format!("{libs}/libnvinfer_builder_resource_{{sm}}.so.10.15.1"),
                    "vsmlrt-cuda/libnvinfer_builder_resource_{sm}.so.10.15.1",
                ),
                member(
                    "tensorrt_cu13_libs-10.15.1.29.dist-info/LICENSE.txt",
                    "licences/TensorRT-LICENSE.txt",
                ),
            ],
        },
        Archive {
            url: "https://files.pythonhosted.org/packages/a6/5a/b116ad2b7e574d691458ca0139ab4e9f26beed62184c85570636ce127b7f/nvidia_cuda_runtime-13.2.51-py3-none-manylinux2014_x86_64.manylinux_2_17_x86_64.whl".to_owned(),
            size: 2_321_923,
            sha256: "9c43b06a52c5b9316e19abc047236932c4d5c729969918a83223c4d2a4132f9a".to_owned(),
            members: vec![
                member(&format!("{cuda}/libcudart.so.13"), "vsmlrt-cuda/libcudart.so.13"),
                member(
                    "nvidia_cuda_runtime-13.2.51.dist-info/licenses/License.txt",
                    "licences/CUDA-runtime-LICENSE.txt",
                ),
            ],
        },
        Archive {
            url: "https://files.pythonhosted.org/packages/5e/21/2fd0aa5a03a8c71962d281084ac44ae7b3b6690d6163ffd7d6486fdb7aa8/nvidia_cuda_nvrtc-13.2.51-py3-none-manylinux2010_x86_64.manylinux_2_12_x86_64.whl".to_owned(),
            size: 47_019_178,
            sha256: "c88076f32cbbd26e7ebd2107d4b093dd8667e2a90b23b3273d028f3daf574d2e".to_owned(),
            members: vec![
                member(&format!("{cuda}/libnvrtc.so.13"), "vsmlrt-cuda/libnvrtc.so.13"),
                member(&format!("{cuda}/libnvrtc-builtins.so.13.2"), "vsmlrt-cuda/libnvrtc-builtins.so.13.2"),
                member(
                    "nvidia_cuda_nvrtc-13.2.51.dist-info/licenses/License.txt",
                    "licences/CUDA-NVRTC-LICENSE.txt",
                ),
            ],
        },
    ]
}

/// Windows' archives ([`archives`]): NVIDIA's zips, of 2.2 GB in all.
fn windows_archives() -> Vec<Archive> {
    let trt = "TensorRT-10.15.1.29";
    let cudart = "cuda_cudart-windows-x86_64-13.2.51-archive";
    let nvrtc = "cuda_nvrtc-windows-x86_64-13.2.51-archive";
    vec![
        Archive {
            url: format!(
                "https://developer.download.nvidia.com/compute/machine-learning/tensorrt/10.15.1/zip/{trt}.Windows.amd64.cuda-13.1.zip"
            ),
            size: 1_934_486_030,
            sha256: "83304c1f9ab86534f083bc4864691b38dee34135fd1aa0391644531c7c4e1e1e".to_owned(),
            members: vec![
                member(
                    &format!("{trt}/bin/nvinfer_10.dll"),
                    "vsmlrt-cuda/nvinfer_10.dll",
                ),
                member(
                    &format!("{trt}/bin/nvinfer_plugin_10.dll"),
                    "vsmlrt-cuda/nvinfer_plugin_10.dll",
                ),
                member(
                    &format!("{trt}/bin/nvonnxparser_10.dll"),
                    "vsmlrt-cuda/nvonnxparser_10.dll",
                ),
                member(
                    &format!("{trt}/bin/nvinfer_builder_resource_{{sm}}_10.dll"),
                    "vsmlrt-cuda/nvinfer_builder_resource_{sm}_10.dll",
                ),
                member(
                    &format!("{trt}/doc/Acknowledgements.txt"),
                    "licences/TensorRT-Acknowledgements.txt",
                ),
            ],
        },
        Archive {
            url: format!(
                "https://developer.download.nvidia.com/compute/cuda/redist/cuda_cudart/windows-x86_64/{cudart}.zip"
            ),
            size: 3_092_970,
            sha256: "906b430b53e6396f7cbaa6874fbe7f9307a184e830b119a4ad3f58dc6e0165c6".to_owned(),
            members: vec![
                member(
                    &format!("{cudart}/bin/x64/cudart64_13.dll"),
                    "vsmlrt-cuda/cudart64_13.dll",
                ),
                member(
                    &format!("{cudart}/LICENSE"),
                    "licences/CUDA-runtime-LICENSE.txt",
                ),
            ],
        },
        Archive {
            url: format!(
                "https://developer.download.nvidia.com/compute/cuda/redist/cuda_nvrtc/windows-x86_64/{nvrtc}.zip"
            ),
            size: 288_205_206,
            sha256: "e85d74221e685fd12040b129eb0f91055b34f9149651cf5259079e21ba690897".to_owned(),
            members: vec![
                member(
                    &format!("{nvrtc}/bin/x64/nvrtc64_130_0.dll"),
                    "vsmlrt-cuda/nvrtc64_130_0.dll",
                ),
                member(
                    &format!("{nvrtc}/bin/x64/nvrtc-builtins64_132.dll"),
                    "vsmlrt-cuda/nvrtc-builtins64_132.dll",
                ),
                member(
                    &format!("{nvrtc}/LICENSE"),
                    "licences/CUDA-NVRTC-LICENSE.txt",
                ),
            ],
        },
    ]
}

/// The files of vs-mlrt in RVP's package (`vapoursynth/vs-mlrt`) and
/// where the install folder takes them: the plugin, its `trtexec`, and
/// RIFE v4.6's model (MIT, Practical-RIFE's, converted by vs-mlrt).
pub const PACKAGE_FILES: [(&str, &str); 3] = [
    (VSTRT, VSTRT),
    (TRTEXEC, TRTEXEC_INSTALLED),
    ("models/rife/rife_v4.6.onnx", "models/rife/rife_v4.6.onnx"),
];

/// vs-mlrt's plugin and `trtexec` by their file names, where `trtexec`
/// goes (vs-mlrt looks for it in `vsmlrt-cuda`), and TensorRT's library,
/// which the plugin loads from there.
#[cfg(windows)]
pub const VSTRT: &str = "vstrt.dll";
#[cfg(not(windows))]
pub const VSTRT: &str = "libvstrt.so";
#[cfg(windows)]
pub const TRTEXEC: &str = "trtexec.exe";
#[cfg(not(windows))]
pub const TRTEXEC: &str = "trtexec";
#[cfg(windows)]
const TRTEXEC_INSTALLED: &str = "vsmlrt-cuda/trtexec.exe";
#[cfg(not(windows))]
const TRTEXEC_INSTALLED: &str = "vsmlrt-cuda/trtexec";
#[cfg(windows)]
pub const NVINFER: &str = "nvinfer_10.dll";
#[cfg(not(windows))]
pub const NVINFER: &str = "libnvinfer.so.10";

/// What [`INSTALLED`] says.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Installed {
    /// TensorRT's version.
    pub version: String,
    /// The card's architecture whose builder resource is there (`sm86`).
    pub builder: String,
    /// The files the folder holds, relative to it.
    pub files: Vec<String>,
}

/// What [`INSTALLED`] in `folder` says, complete or not.
fn recorded(folder: &Path) -> Option<Installed> {
    let text = std::fs::read_to_string(folder.join(INSTALLED)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The installation in `folder`, when it is complete.
#[must_use]
pub fn installed(folder: &Path) -> Option<Installed> {
    let installed = recorded(folder)?;
    (installed.version == VERSION
        && installed
            .files
            .iter()
            .all(|file| folder.join(file).is_file()))
    .then_some(installed)
}

/// The builder resource for a card of compute capability `major.minor`
/// (`nvidia-smi`'s `compute_cap`): its own (`sm86`) or PTX's; `None`
/// before Turing (7.5), which TensorRT 10 does not run on.
#[must_use]
pub fn builder_for(capability: &str) -> Option<String> {
    let (major, minor) = capability.trim().split_once('.')?;
    let (major, minor): (u32, u32) = (major.parse().ok()?, minor.parse().ok()?);
    if (major, minor) < (7, 5) {
        return None;
    }
    let own = format!("sm{major}{minor}");
    Some(if BUILDERS.contains(&own.as_str()) {
        own
    } else {
        "ptx".to_owned()
    })
}

/// The first card's compute capability by NVIDIA's `nvidia-smi` (it comes
/// with the driver); `None` without an NVIDIA card.
#[must_use]
pub fn compute_capability() -> Option<String> {
    let program = if cfg!(windows) {
        std::env::var_os("SystemRoot").map_or_else(
            || PathBuf::from("nvidia-smi"),
            |root| PathBuf::from(root).join("System32").join("nvidia-smi.exe"),
        )
    } else {
        PathBuf::from("nvidia-smi")
    };
    let mut command = std::process::Command::new(program);
    command.args(["--query-gpu=compute_cap", "--format=csv,noheader"]);
    hide_console(&mut command);
    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    output
        .status
        .success()
        .then(|| text.lines().next().unwrap_or_default().trim().to_owned())
        .filter(|line| !line.is_empty())
}

/// No console window over the video for a program RVP starts (Windows'
/// `CREATE_NO_WINDOW`).
fn hide_console(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

/// What an installation needs.
#[derive(Debug, Clone)]
pub struct Job {
    /// `<state>/tensorrt`.
    pub folder: PathBuf,
    /// The package's `vapoursynth/vs-mlrt`.
    pub package: PathBuf,
    pub archives: Vec<Archive>,
    /// A folder URL that replaces each archive's (`RVP_TENSORRT_MIRROR`).
    pub mirror: Option<String>,
    /// The card's architecture (`sm86`, or `ptx`); `None`: asked of
    /// `nvidia-smi` on the installer's thread.
    pub builder: Option<String>,
}

/// Where an installation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Downloading,
    Checking,
    Unpacking,
}

/// What the installer tells the window's thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Bytes done of the stage's total.
    Progress {
        stage: Stage,
        done: u64,
        total: u64,
    },
    Finished(Result<Installed, String>),
    Cancelled,
}

/// The installer's thread.
pub struct Installer {
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Installer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Installer").finish_non_exhaustive()
    }
}

impl Installer {
    /// Starts `job` on a thread of its own; returns at once.
    ///
    /// # Errors
    /// Fails when the thread cannot start.
    pub fn start(job: Job) -> std::io::Result<Self> {
        let (sender, events) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancel);
        let thread = std::thread::Builder::new()
            .name("rvp-tensorrt".to_owned())
            .spawn(move || {
                let builder = job.builder.clone().or_else(|| {
                    compute_capability().and_then(|capability| builder_for(&capability))
                });
                let Some(builder) = builder else {
                    drop(sender.send(Event::Finished(Err(
                        "no NVIDIA card of Turing (GeForce RTX 20) or later was found".to_owned(),
                    ))));
                    return;
                };
                let job = Job {
                    builder: Some(builder),
                    ..job
                };
                let event = match install(&job, &stop, &sender) {
                    Ok(installed) => Event::Finished(Ok(installed)),
                    Err(_) if stop.load(Ordering::Relaxed) => Event::Cancelled,
                    Err(error) => Event::Finished(Err(error)),
                };
                drop(sender.send(event));
            })?;
        Ok(Self {
            events,
            cancel,
            thread: Some(thread),
        })
    }

    /// The events since the last call.
    #[must_use]
    pub fn take(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }

    /// Stops the download (what is downloaded stays for the next one).
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Installer {
    fn drop(&mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// Downloads, checks and unpacks `job`'s archives, takes the package's
/// files and writes [`INSTALLED`].
fn install(job: &Job, cancel: &AtomicBool, sender: &Sender<Event>) -> Result<Installed, String> {
    let builder = job.builder.as_deref().unwrap_or("ptx");
    let downloads = job.folder.join("downloads");
    std::fs::create_dir_all(&downloads).map_err(|error| error.to_string())?;
    let total: u64 = job.archives.iter().map(|archive| archive.size).sum();
    let mut before = 0;
    for archive in &job.archives {
        let target = downloads.join(archive.file_name());
        if !complete(&target, archive.size) {
            let url = job.mirror.as_ref().map_or_else(
                || archive.url.clone(),
                |mirror| format!("{}/{}", mirror.trim_end_matches('/'), archive.file_name()),
            );
            download(&url, &target, archive.size, cancel, &mut |done| {
                drop(sender.send(Event::Progress {
                    stage: Stage::Downloading,
                    done: before + done,
                    total,
                }));
            })?;
        }
        before += archive.size;
    }
    let mut checked = 0;
    for archive in &job.archives {
        let target = downloads.join(archive.file_name());
        let sha256 = file_sha256(&target, cancel, &mut |done| {
            drop(sender.send(Event::Progress {
                stage: Stage::Checking,
                done: checked + done,
                total,
            }));
        })?;
        if sha256 != archive.sha256 {
            drop(std::fs::remove_file(&target));
            return Err(format!(
                "{} is not NVIDIA's file (SHA-256 {sha256}): it was deleted",
                archive.file_name()
            ));
        }
        checked += archive.size;
    }
    let mut files = Vec::new();
    for archive in &job.archives {
        let target = downloads.join(archive.file_name());
        files.extend(unpack(&target, archive, builder, &job.folder, cancel)?);
        drop(
            sender.send(Event::Progress {
                stage: Stage::Unpacking,
                done: files.len() as u64,
                total: job
                    .archives
                    .iter()
                    .map(|archive| archive.members.len() as u64)
                    .sum(),
            }),
        );
    }
    for (from, to) in PACKAGE_FILES {
        let source = job.package.join(from);
        let destination = job.folder.join(to);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::copy(&source, &destination)
            .map_err(|error| format!("{}: {error}", source.display()))?;
        files.push(to.to_owned());
    }
    // Another card's installation, or another version's: what it kept and
    // this one does not, and the engines built for another card or by
    // another TensorRT.
    if let Some(before) = recorded(&job.folder) {
        for file in before.files.iter().filter(|file| !files.contains(file)) {
            drop(std::fs::remove_file(job.folder.join(file)));
        }
        if before.builder != builder || before.version != VERSION {
            drop(std::fs::remove_dir_all(job.folder.join("engines")));
        }
    }
    std::fs::create_dir_all(job.folder.join("engines")).map_err(|error| error.to_string())?;
    let installed = Installed {
        version: VERSION.to_owned(),
        builder: builder.to_owned(),
        files,
    };
    let text = serde_json::to_string_pretty(&installed).map_err(|error| error.to_string())?;
    std::fs::write(job.folder.join(INSTALLED), text).map_err(|error| error.to_string())?;
    // 2 GB that are not needed any more.
    drop(std::fs::remove_dir_all(&downloads));
    Ok(installed)
}

/// Whether `path` is a complete download of `size` bytes.
fn complete(path: &Path, size: u64) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.len() == size)
}

/// Downloads `url` to `target` through `curl`, going on from a `.part`
/// left by an earlier try; `progress` gets the bytes there every quarter
/// of a second.
fn download(
    url: &str,
    target: &Path,
    size: u64,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<(), String> {
    let part = target.with_extension("part");
    let program = if cfg!(windows) {
        std::env::var_os("SystemRoot").map_or_else(
            || PathBuf::from("curl"),
            |root| PathBuf::from(root).join("System32").join("curl.exe"),
        )
    } else {
        PathBuf::from("curl")
    };
    let mut command = std::process::Command::new(program);
    command
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--retry",
            "3",
        ])
        .args(["--continue-at", "-", "--output"])
        .arg(&part)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    hide_console(&mut command);
    let mut child = command.spawn().map_err(|error| {
        // Linux's packages only recommend curl (the Linux audit's L23).
        if error.kind() == std::io::ErrorKind::NotFound {
            "curl is not installed: the download needs it (the curl package)".to_owned()
        } else {
            format!("curl cannot start: {error}")
        }
    })?;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            drop(child.kill());
            drop(child.wait());
            return Err("cancelled".to_owned());
        }
        progress(std::fs::metadata(&part).map_or(0, |meta| meta.len()));
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(250)),
            Err(error) => return Err(error.to_string()),
        }
    };
    if !status.success() {
        let mut why = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            drop(stderr.read_to_string(&mut why));
        }
        return Err(format!("{url}: {}", why.trim()));
    }
    if !complete(&part, size) {
        let got = std::fs::metadata(&part).map_or(0, |meta| meta.len());
        drop(std::fs::remove_file(&part));
        return Err(format!("{url}: {got} bytes, not {size}"));
    }
    std::fs::rename(&part, target).map_err(|error| error.to_string())
}

/// The SHA-256 of `path` as lowercase hex.
fn file_sha256(
    path: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 20];
    let mut done = 0_u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_owned());
        }
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        done += read as u64;
        if done % (64 << 20) < (1 << 20) {
            progress(done);
        }
    }
    Ok(hex(&hasher.finalize()))
}

/// `bytes` as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Unpacks `archive`'s members from `path` into `folder`; returns their
/// names there.
fn unpack(
    path: &Path,
    archive: &Archive,
    builder: &str,
    folder: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|error| format!("{}: {error}", archive.file_name()))?;
    let mut names = Vec::new();
    for member in &archive.members {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_owned());
        }
        let inside = member.path.replace("{sm}", builder);
        let name = member.name.replace("{sm}", builder);
        let mut entry = zip
            .by_name(&inside)
            .map_err(|error| format!("{inside} in {}: {error}", archive.file_name()))?;
        let destination = folder.join(&name);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let temporary = destination.with_extension("unpacking");
        let mut out = std::fs::File::create(&temporary).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|error| format!("{inside}: {error}"))?;
        drop(out);
        std::fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
        names.push(name);
    }
    Ok(names)
}

/// About what an installation keeps for one architecture (TensorRT's
/// libraries, one builder resource, CUDA's runtime and NVRTC), for the
/// settings window.
pub const KEPT_BYTES: u64 = 700_000_000;

/// The builder resource the card needs (`None`: no NVIDIA card TensorRT
/// runs on), asked of `nvidia-smi` on a thread when RVP starts with
/// TensorRT installed.
#[derive(Debug, Default)]
enum Card {
    #[default]
    Unasked,
    Asking(Receiver<Option<String>>),
    Known(Option<String>),
}

/// The TensorRT installation's state in the [`App`].
#[derive(Debug, Default)]
pub(crate) struct State {
    /// What `<state>/tensorrt` holds, when complete.
    installed: Option<Installed>,
    card: Card,
    /// The installer while it runs, where it is, and the last failure.
    installer: Option<Installer>,
    progress: Option<(Stage, u64, u64)>,
    error: Option<String>,
}

impl App {
    /// `<state>/tensorrt`.
    #[must_use]
    pub(crate) fn tensorrt_folder(&self) -> PathBuf {
        self.state_dir().join("tensorrt")
    }

    /// Reads the installation (when RVP starts, after the runtime) and,
    /// when there is one, asks which builder resource the card needs
    /// (`nvidia-smi` takes a moment: not on the window's thread).
    pub(crate) fn load_tensorrt(&mut self) {
        let folder = self.tensorrt_folder();
        let trt = &mut self.tensorrt;
        trt.installed = installed(&folder);
        let Some(installed) = &trt.installed else {
            return;
        };
        let (sender, receiver) = channel();
        let asked = std::thread::Builder::new()
            .name("rvp-tensorrt-card".to_owned())
            .spawn(move || {
                drop(sender.send(compute_capability().and_then(|cap| builder_for(&cap))));
            });
        trt.card = match asked {
            Ok(_) => Card::Asking(receiver),
            // Without the answer, the installation is taken as this card's.
            Err(_) => Card::Known(Some(installed.builder.clone())),
        };
    }

    /// Whether RVP is still asking which builder resource the card needs
    /// (the calibration waits for the answer).
    #[must_use]
    pub(crate) fn tensorrt_checking(&self) -> bool {
        matches!(self.tensorrt.card, Card::Asking(_))
    }

    /// The installation, when it is this card's.
    fn tensorrt_for_this_card(&self) -> Option<&Installed> {
        let installed = self.tensorrt.installed.as_ref()?;
        matches!(&self.tensorrt.card, Card::Known(Some(builder)) if *builder == installed.builder)
            .then_some(installed)
    }

    /// Whether the card's answer came and the installation is another
    /// card's.
    fn tensorrt_for_another_card(&self) -> bool {
        self.tensorrt.installed.as_ref().is_some_and(|installed| {
            matches!(&self.tensorrt.card,
                Card::Known(card) if card.as_deref() != Some(installed.builder.as_str()))
        })
    }

    /// The builder resource the card needs, once its answer came (`None`
    /// too without an NVIDIA card TensorRT runs on).
    fn tensorrt_card(&self) -> Option<&str> {
        match &self.tensorrt.card {
            Card::Known(card) => card.as_deref(),
            _ => None,
        }
    }

    /// Whether RIFE (TensorRT) can be installed or run here: the package
    /// has vs-mlrt and this system has NVIDIA's archives.
    fn tensorrt_possible(&self) -> bool {
        self.smoothing.runtime.tensorrt_scripts().is_some() && !archives().is_empty()
    }

    /// Whether TensorRT is being installed.
    #[must_use]
    pub(crate) fn tensorrt_installing(&self) -> bool {
        self.tensorrt.installer.is_some()
    }

    /// Whether an installation is there, this card's or not (the
    /// calibration's key names its files).
    #[must_use]
    pub(crate) fn tensorrt_present(&self) -> bool {
        self.tensorrt_possible() && self.tensorrt.installed.is_some()
    }

    /// Whether RIFE (TensorRT) is installed for this card (its files, not
    /// its engine).
    #[must_use]
    pub(crate) fn tensorrt_installed(&self) -> bool {
        self.tensorrt_possible() && self.tensorrt_for_this_card().is_some()
    }

    /// Whether RIFE (TensorRT) can run: installed, and its engine built
    /// and measured by the calibration (building it during playback would
    /// stop the picture for minutes).
    #[must_use]
    pub(crate) fn rife_trt_ready(&self) -> bool {
        self.tensorrt_installed()
            && !self.smoothing.left_out.contains(&Engine::RifeTrt)
            && self.calibration_measured(Engine::RifeTrt)
    }

    /// The settings window's Download: starts the installer.
    pub fn tensorrt_install(&mut self) -> Option<UiRequest> {
        if self.tensorrt.installer.is_some() || !self.tensorrt_possible() {
            return None;
        }
        let package = self.smoothing.runtime.tensorrt_scripts()?.to_path_buf();
        let job = Job {
            folder: self.tensorrt_folder(),
            package,
            archives: archives(),
            mirror: std::env::var("RVP_TENSORRT_MIRROR").ok(),
            builder: None,
        };
        self.tensorrt.error = None;
        self.tensorrt.progress = None;
        match Installer::start(job) {
            Ok(installer) => {
                crate::log::write(
                    crate::log::Level::Info,
                    &format!(
                        "tensorrt: downloading NVIDIA's TensorRT {VERSION} and CUDA's runtime"
                    ),
                );
                self.tensorrt.installer = Some(installer);
                None
            }
            Err(error) => {
                let text = format!("TensorRT cannot be installed: {error}");
                crate::log::report(crate::log::Level::Warning, &text);
                self.tensorrt.error = Some(text.clone());
                Some(UiRequest::Message(text))
            }
        }
    }

    /// The settings window's Cancel: stops the download (what is
    /// downloaded stays for the next try).
    pub fn tensorrt_cancel(&mut self) {
        if let Some(installer) = &self.tensorrt.installer {
            installer.cancel();
        }
    }

    /// The settings window's Remove: deletes `<state>/tensorrt` (NVIDIA's
    /// files and the built engines).
    ///
    /// # Errors
    /// Fails while it installs, or when the folder cannot be deleted.
    pub fn tensorrt_remove(&mut self, engine: &Mpv) -> Result<(), String> {
        if self.tensorrt.installer.is_some() {
            return Err("TensorRT is being installed".to_owned());
        }
        let folder = self.tensorrt_folder();
        if folder.exists() {
            std::fs::remove_dir_all(&folder)
                .map_err(|error| format!("{}: {error}", folder.display()))?;
        }
        self.tensorrt.installed = None;
        self.tensorrt.card = Card::Unasked;
        crate::log::write(crate::log::Level::Info, "tensorrt: removed");
        self.apply_smoothing(engine);
        Ok(())
    }

    /// Once a second: the card's answer and the installer's news. A
    /// finished installation asks the calibration to build and measure
    /// RIFE (TensorRT)'s engine.
    pub fn tensorrt_tick(&mut self, engine: &Mpv) -> Option<UiRequest> {
        if let Card::Asking(receiver) = &self.tensorrt.card {
            let answer = match receiver.try_recv() {
                Ok(card) => Some(card),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(None),
            };
            if let Some(card) = answer {
                self.tensorrt.card = Card::Known(card);
                if let Some(installed) = self
                    .tensorrt
                    .installed
                    .as_ref()
                    .filter(|_| self.tensorrt_for_another_card())
                {
                    crate::log::report(
                        crate::log::Level::Warning,
                        &format!(
                            "tensorrt: installed for {}, and this card needs {}: RIFE (TensorRT) \
                             waits for its download again (the Motion Smoothing page)",
                            installed.builder,
                            self.tensorrt_card()
                                .unwrap_or("an NVIDIA card of Turing or later")
                        ),
                    );
                }
                self.apply_smoothing(engine);
            }
        }
        let events = self
            .tensorrt
            .installer
            .as_ref()
            .map(Installer::take)
            .unwrap_or_default();
        let mut request = None;
        for event in events {
            match event {
                Event::Progress { stage, done, total } => {
                    self.tensorrt.progress = Some((stage, done, total));
                }
                Event::Finished(Ok(done)) => {
                    crate::log::write(
                        crate::log::Level::Info,
                        &format!(
                            "tensorrt: TensorRT {} installed for {} ({} files)",
                            done.version,
                            done.builder,
                            done.files.len()
                        ),
                    );
                    // The installer asked the card itself.
                    self.tensorrt.card = Card::Known(Some(done.builder.clone()));
                    self.tensorrt.installed = Some(done);
                    self.tensorrt.installer = None;
                    self.tensorrt.progress = None;
                    // Its engine is built and measured before it runs; the
                    // other engines' measurements stay.
                    self.calibrate_engine(Engine::RifeTrt);
                    self.apply_smoothing(engine);
                    request = Some(UiRequest::Osd(
                        "TensorRT installed: the calibration builds RIFE's engine when nothing plays"
                            .to_owned(),
                    ));
                }
                Event::Finished(Err(error)) => {
                    crate::log::report(crate::log::Level::Warning, &format!("tensorrt: {error}"));
                    self.tensorrt.installer = None;
                    self.tensorrt.progress = None;
                    self.tensorrt.error = Some(error.clone());
                    request = Some(UiRequest::Message(format!(
                        "TensorRT was not installed: {error}"
                    )));
                }
                Event::Cancelled => {
                    crate::log::write(crate::log::Level::Info, "tensorrt: download cancelled");
                    self.tensorrt.installer = None;
                    self.tensorrt.progress = None;
                }
            }
        }
        request
    }

    /// The installation for the settings window: its state, the stage and
    /// bytes of a running one, the sizes, the last failure, and whether
    /// RIFE (TensorRT)'s engine is ready.
    #[must_use]
    pub fn tensorrt_status(&self) -> serde_json::Value {
        let trt = &self.tensorrt;
        let status = if !self.tensorrt_possible() {
            "unavailable"
        } else if trt.installer.is_some() {
            "installing"
        } else if self.tensorrt_for_another_card() {
            "other-card"
        } else if trt.installed.is_some() {
            "installed"
        } else {
            "missing"
        };
        let (stage, done, total) = trt.progress.map_or((None, 0, 0), |(stage, done, total)| {
            (Some(stage), done, total)
        });
        serde_json::json!({
            "state": status,
            "version": VERSION,
            "stage": stage,
            "done": done,
            "total": total,
            "download": archives().iter().map(|archive| archive.size).sum::<u64>(),
            "kept": KEPT_BYTES,
            "builder": trt.installed.as_ref().map(|installed| installed.builder.clone()),
            "card": self.tensorrt_card(),
            "error": trt.error,
            "ready": self.rife_trt_ready(),
        })
    }
}

use crate::app::{App, UiRequest};
use crate::smoothing::Engine;
use rvp_engine::Engine as Mpv;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-tensorrt-{name}-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// A zip of `files` (name, bytes) at `path`.
    fn zip_of(path: &Path, files: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).expect("zip"));
        for (name, bytes) in files {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .expect("entry");
            writer.write_all(bytes).expect("bytes");
        }
        writer.finish().expect("finish");
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    #[test]
    fn the_card_picks_its_builder_resource() {
        assert_eq!(builder_for("8.6").as_deref(), Some("sm86"));
        assert_eq!(builder_for("12.0\n").as_deref(), Some("sm120"));
        // A card of an architecture the archive has no resource of.
        assert_eq!(builder_for("8.7").as_deref(), Some("ptx"));
        // Pascal is too old for TensorRT 10.
        assert_eq!(builder_for("6.1"), None);
        assert_eq!(builder_for("none"), None);
    }

    #[test]
    fn linux_archives_keep_what_rife_needs() {
        let names: Vec<String> = linux_archives()
            .iter()
            .flat_map(|archive| {
                archive
                    .members
                    .iter()
                    .map(|member| member.name.replace("{sm}", "sm86"))
            })
            .collect();
        for wanted in [
            "vsmlrt-cuda/libnvinfer.so.10",
            "vsmlrt-cuda/libnvinfer_plugin.so.10",
            "vsmlrt-cuda/libnvonnxparser.so.10",
            "vsmlrt-cuda/libnvinfer_builder_resource_sm86.so.10.15.1",
            "vsmlrt-cuda/libcudart.so.13",
            "vsmlrt-cuda/libnvrtc.so.13",
        ] {
            assert!(names.iter().any(|name| name == wanted), "{wanted}");
        }
        assert!(
            linux_archives()
                .iter()
                .all(|archive| archive.sha256.len() == 64)
        );
    }

    #[test]
    fn windows_archives_keep_what_rife_needs() {
        let archives = windows_archives();
        let names: Vec<String> = archives
            .iter()
            .flat_map(|archive| {
                archive
                    .members
                    .iter()
                    .map(|member| member.name.replace("{sm}", "sm86"))
            })
            .collect();
        for wanted in [
            "vsmlrt-cuda/nvinfer_10.dll",
            "vsmlrt-cuda/nvinfer_plugin_10.dll",
            "vsmlrt-cuda/nvinfer_builder_resource_sm86_10.dll",
            "vsmlrt-cuda/cudart64_13.dll",
            "vsmlrt-cuda/nvrtc64_130_0.dll",
        ] {
            assert!(names.iter().any(|name| name == wanted), "{wanted}");
        }
        assert!(archives.iter().all(|archive| archive.sha256.len() == 64));
    }

    /// A mirror with one archive (TensorRT's library and two builder
    /// resources), the package's files and a job for `sm86` under a
    /// scratch folder; `None` without curl.
    fn test_installation(name: &str) -> Option<(PathBuf, Job, Vec<u8>)> {
        if std::process::Command::new("curl")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("skipped: no curl");
            return None;
        }
        let root = scratch(name);
        let mirror = root.join("mirror");
        std::fs::create_dir_all(&mirror).expect("mirror");
        let library = b"a library for the test".to_vec();
        zip_of(
            &mirror.join("runtime.zip"),
            &[
                ("runtime/bin/nvinfer_10.dll", &library),
                ("runtime/bin/builder_sm86.dll", b"sm86"),
                ("runtime/bin/builder_sm89.dll", b"sm89"),
            ],
        );
        let bytes = std::fs::read(mirror.join("runtime.zip")).expect("read");
        let package = root.join("package");
        for (from, _) in PACKAGE_FILES {
            let path = package.join(from);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
            std::fs::write(&path, from).expect("package file");
        }
        let archive = Archive {
            url: "https://example.invalid/runtime.zip".to_owned(),
            size: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            members: vec![
                Member {
                    path: "runtime/bin/nvinfer_10.dll".to_owned(),
                    name: "vsmlrt-cuda/nvinfer_10.dll".to_owned(),
                },
                Member {
                    path: "runtime/bin/builder_{sm}.dll".to_owned(),
                    name: "vsmlrt-cuda/builder_{sm}.dll".to_owned(),
                },
            ],
        };
        let text = mirror.to_string_lossy().replace('\\', "/");
        let url = if text.starts_with('/') {
            format!("file://{text}")
        } else {
            format!("file:///{text}")
        };
        let job = Job {
            folder: root.join("tensorrt"),
            package,
            archives: vec![archive],
            mirror: Some(url),
            builder: Some("sm86".to_owned()),
        };
        Some((root, job, library))
    }

    #[test]
    fn an_installation_downloads_checks_unpacks_and_takes_the_package() {
        let Some((root, job, library)) = test_installation("install") else {
            return;
        };
        let folder = job.folder.clone();
        let (sender, events) = channel();
        let done = install(&job, &AtomicBool::new(false), &sender).expect("install");
        assert_eq!(done.builder, "sm86");
        assert_eq!(
            std::fs::read(folder.join("vsmlrt-cuda/nvinfer_10.dll")).expect("unpacked"),
            library
        );
        assert!(folder.join("vsmlrt-cuda/builder_sm86.dll").is_file());
        assert!(!folder.join("vsmlrt-cuda/builder_sm89.dll").exists());
        assert!(folder.join(VSTRT).is_file() && folder.join(TRTEXEC_INSTALLED).is_file());
        assert!(!folder.join("downloads").exists(), "the downloads go");
        assert_eq!(installed(&folder), Some(done));
        assert!(events.try_iter().any(|event| matches!(
            event,
            Event::Progress {
                stage: Stage::Downloading,
                ..
            }
        )));
        // An archive that is not the pinned one is refused and deleted.
        let wrong = Archive {
            sha256: "0".repeat(64),
            ..job.archives[0].clone()
        };
        let job = Job {
            folder: root.join("wrong"),
            archives: vec![wrong],
            ..job
        };
        let error = install(&job, &AtomicBool::new(false), &sender).expect_err("refused");
        assert!(error.contains("is not NVIDIA's file"), "{error}");
        assert!(!root.join("wrong/downloads/runtime.zip").exists());
        assert_eq!(installed(&root.join("wrong")), None);
        drop(std::fs::remove_dir_all(&root));
    }

    #[test]
    fn another_cards_installation_gives_way_to_this_ones() {
        let Some((root, job, _)) = test_installation("again") else {
            return;
        };
        let folder = job.folder.clone();
        let (sender, _events) = channel();
        install(&job, &AtomicBool::new(false), &sender).expect("install");
        // The host's RTX 30-series card gave way to an RTX 50-series card: the new installation
        // keeps that card's builder resource, and the one before and the
        // engines built for it go.
        std::fs::write(folder.join("engines/rife.sm86.engine"), b"engine").expect("engine");
        let again = Job {
            builder: Some("sm89".to_owned()),
            ..job
        };
        let done = install(&again, &AtomicBool::new(false), &sender).expect("again");
        assert_eq!(done.builder, "sm89");
        assert!(folder.join("vsmlrt-cuda/builder_sm89.dll").is_file());
        assert!(!folder.join("vsmlrt-cuda/builder_sm86.dll").exists());
        assert!(folder.join("vsmlrt-cuda/nvinfer_10.dll").is_file());
        assert!(folder.join("engines").is_dir());
        assert!(!folder.join("engines/rife.sm86.engine").exists());
        assert_eq!(installed(&folder), Some(done));
        drop(std::fs::remove_dir_all(&root));
    }
}
