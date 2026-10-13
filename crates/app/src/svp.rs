//! SVP mode (M6): SVP 4 Manager attaches to RVP through mpv's JSON IPC and
//! inserts its VapourSynth filter (`@svp`) into RVP's chain (spike S1).
//!
//! * SVP mode is on while the IPC endpoint (`ipc-server`) is set. RVP's
//!   defaults are the endpoints SVP's own mpv uses: the pipe `mpvpipe` on
//!   Windows and the socket `/tmp/mpvsocket` on Linux.
//! * Only one process can hold an endpoint. When it is taken (a second RVP
//!   window, `/new`, or another mpv), RVP uses the name SVP's
//!   `mpvSockets.lua` gives a process, `<temp>/mpvSockets/<pid>`; SVP's own
//!   `mpv.conf` names that script as the way to run several mpv windows at
//!   once.
//! * SVP needs frames in memory: in SVP mode decoding uses the copy-back
//!   variant of the chosen hardware decoder ([`copy_back`]), and mpv's
//!   `hr-seek-framedrop` stays off (the engine sets it).
//! * SVP's filter runs in RVP's libmpv with a VapourSynth runtime: RVP
//!   points mpv at SVP's own (rule 8: `~/SVP4/mpv` or `/opt/svp` on Linux,
//!   `mpv64` in SVP's folder on Windows) unless the settings name one.
//! * When SVP has not attached a few seconds after a file starts, RVP says
//!   why it may not have ([`Problem`]), once per session and on the settings
//!   window's SVP page.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use rvp_engine::PropertyId;

use crate::app::{App, UiRequest};
use crate::settings::Settings;

/// How long after a file starts SVP has to attach before RVP looks why not.
pub const ATTACH_WAIT: Duration = Duration::from_secs(8);

/// SVP mode: an endpoint is set.
#[must_use]
pub fn svp_mode(settings: &Settings) -> bool {
    !settings.ipc_server.trim().is_empty()
}

/// The endpoint SVP's `mpvSockets.lua` gives process `pid`: in Lua's
/// temporary folder (`/tmp` on Linux, the user's temporary folder on
/// Windows), `mpvSockets/<pid>`, joined with `/` as mpv does.
#[must_use]
pub fn mpv_sockets_endpoint(pid: u32, temp: &Path) -> String {
    let temp = temp.to_string_lossy();
    let temp = temp.trim_end_matches(['/', '\\']);
    format!("{temp}/mpvSockets/{pid}")
}

/// The temporary folder `mpvSockets.lua` sees.
#[must_use]
pub fn lua_temp_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::temp_dir()
    } else {
        PathBuf::from("/tmp")
    }
}

/// Whether an endpoint is one SVP looks for.
#[must_use]
pub fn is_known_endpoint(endpoint: &str) -> bool {
    endpoint == rvp_engine::default_ipc_server()
        || endpoint.replace('\\', "/").contains("/mpvSockets/")
}

/// The endpoint to open: the configured one, or the `mpvSockets` name when
/// another process holds it; `None` when SVP mode is off.
#[must_use]
pub fn choose_endpoint(
    configured: &str,
    pid: u32,
    in_use: impl Fn(&str) -> bool,
) -> Option<String> {
    let configured = configured.trim();
    if configured.is_empty() {
        return None;
    }
    if in_use(configured) {
        Some(mpv_sockets_endpoint(pid, &lua_temp_dir()))
    } else {
        Some(configured.to_owned())
    }
}

/// Whether a process serves `endpoint` now. On Windows it is a named pipe
/// (`\\.\pipe\<name>`, as mpv names it); a pipe that exists but is busy
/// counts as taken.
#[must_use]
pub fn endpoint_in_use(endpoint: &str) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::net::UnixStream::connect(endpoint).is_ok()
    }
    #[cfg(windows)]
    {
        let path = if endpoint.starts_with(r"\\.\pipe\") {
            endpoint.to_owned()
        } else {
            format!(r"\\.\pipe\{endpoint}")
        };
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
        {
            Ok(_) => true,
            // ERROR_PIPE_BUSY: every instance is connected.
            Err(error) => error.raw_os_error() == Some(231),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = endpoint;
        false
    }
}

/// Creates the folder of an `mpvSockets` socket (mpv does not).
pub fn prepare_endpoint(endpoint: &str) {
    if cfg!(unix)
        && let Some(parent) = Path::new(endpoint).parent()
        && endpoint.contains("/mpvSockets/")
    {
        drop(std::fs::create_dir_all(parent));
    }
}

/// The copy-back variant of an `hwdec` value: decoded frames come back to
/// memory, where SVP's filter and RVP's CPU filters can read them.
#[must_use]
pub fn copy_back(hwdec: &str) -> String {
    let hwdec = hwdec.trim();
    match hwdec {
        "" | "no" => "no".to_owned(),
        "yes" | "auto" | "auto-safe" => "auto-copy".to_owned(),
        value if value.ends_with("-copy") || value.starts_with("auto-copy") => value.to_owned(),
        value if value.contains(',') => value
            .split(',')
            .map(copy_back)
            .collect::<Vec<_>>()
            .join(","),
        value => format!("{value}-copy"),
    }
}

/// Where SVP keeps its VapourSynth runtime (VSScript), most likely first.
#[must_use]
pub fn vsscript_candidates(home: Option<&Path>) -> Vec<PathBuf> {
    if cfg!(windows) {
        let mut list = Vec::new();
        for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(base) = std::env::var_os(variable) {
                list.push(
                    PathBuf::from(base)
                        .join("SVP 4")
                        .join("mpv64")
                        .join("VSScript.dll"),
                );
            }
        }
        list.push(PathBuf::from(
            r"C:\Program Files (x86)\SVP 4\mpv64\VSScript.dll",
        ));
        // Where Windows finds it for mpv (SVP's installer puts mpv64 on
        // PATH).
        if let Some(path) = std::env::var_os("PATH") {
            list.extend(std::env::split_paths(&path).map(|dir| dir.join("VSScript.dll")));
        }
        list
    } else {
        let mut list = Vec::new();
        if let Some(home) = home {
            list.push(home.join("SVP4/mpv/libvapoursynth-script.so.0"));
            list.push(home.join("SVP4/mpv/libvapoursynth-script.so"));
        }
        list.push(PathBuf::from("/opt/svp/mpv/libvapoursynth-script.so.0"));
        list.push(PathBuf::from("/opt/svp/mpv/libvapoursynth-script.so"));
        list.push(PathBuf::from(
            "/usr/lib/x86_64-linux-gnu/libvapoursynth-script.so",
        ));
        list.push(PathBuf::from("/usr/local/lib/libvapoursynth-script.so"));
        list
    }
}

/// The VSScript library mpv should load: the setting when it exists, else
/// the first of [`vsscript_candidates`] that does.
#[must_use]
pub fn find_vsscript(configured: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|path| path.is_file()) {
        return Some(path.to_path_buf());
    }
    vsscript_candidates(home)
        .into_iter()
        .find(|path| path.is_file())
}

/// Whether SVP 4 Manager runs; `None` when it cannot be told. On Windows
/// this starts `tasklist`, so call it off the GUI thread.
#[must_use]
pub fn manager_running() -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        let entries = std::fs::read_dir("/proc").ok()?;
        Some(entries.flatten().any(|entry| {
            std::fs::read_to_string(entry.path().join("comm"))
                .is_ok_and(|name| name.trim().starts_with("SVPManager"))
        }))
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq SVPManager.exe", "/NH"])
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&output.stdout).contains("SVPManager.exe"))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        None
    }
}

/// Why SVP may not have attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// SVP 4 Manager is not running.
    ManagerNotRunning,
    /// RVP listens on an endpoint SVP does not look for.
    EndpointMismatch(String),
    /// mpv could not open the endpoint.
    NoEndpoint,
    /// No VapourSynth runtime for SVP's filter.
    VapourSynthMissing,
    /// SVP's filter is in the chain but the frame rate did not rise.
    FilterNotRunning,
    /// Everything RVP can check looks right.
    Unknown,
}

impl Problem {
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Problem::ManagerNotRunning => {
                "SVP did not attach: SVP 4 Manager is not running".to_owned()
            }
            Problem::EndpointMismatch(endpoint) => format!(
                "SVP did not attach: RVP listens on {endpoint}, SVP looks for {} (settings, SVP page)",
                rvp_engine::default_ipc_server()
            ),
            Problem::NoEndpoint => {
                "SVP did not attach: RVP could not open its IPC endpoint".to_owned()
            }
            Problem::VapourSynthMissing => {
                "SVP did not attach: no VapourSynth runtime found (install SVP 4 or set it on the SVP page)"
                    .to_owned()
            }
            Problem::FilterNotRunning => {
                "SVP attached, but its filter does not run (VapourSynth failed?)".to_owned()
            }
            Problem::Unknown => {
                "SVP did not attach: check SVP's profile for this video".to_owned()
            }
        }
    }
}

/// What RVP knows about SVP for the file that plays.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    /// SVP mode is off.
    #[default]
    Off,
    /// Waiting for SVP to attach.
    Waiting,
    /// SVP's filter is in the chain.
    Attached,
    NotAttached(Problem),
}

impl Status {
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Status::Off => "SVP mode is off".to_owned(),
            Status::Waiting => "Waiting for SVP".to_owned(),
            Status::Attached => "SVP is attached".to_owned(),
            Status::NotAttached(problem) => problem.text(),
        }
    }
}

/// The inputs of a diagnosis besides the manager check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The endpoint RVP opened (`None`: none).
    pub endpoint: Option<String>,
    pub vapoursynth: bool,
    pub filter_in_chain: bool,
    /// The filtered frame rate is above the source's.
    pub interpolating: bool,
}

/// Why SVP is not working, from what RVP can see; `None` when it is.
#[must_use]
pub fn diagnose(facts: &Facts, manager: Option<bool>) -> Option<Problem> {
    if facts.filter_in_chain {
        return (!facts.interpolating).then_some(Problem::FilterNotRunning);
    }
    let Some(endpoint) = &facts.endpoint else {
        return Some(Problem::NoEndpoint);
    };
    if manager == Some(false) {
        return Some(Problem::ManagerNotRunning);
    }
    if !is_known_endpoint(endpoint) {
        return Some(Problem::EndpointMismatch(endpoint.clone()));
    }
    if !facts.vapoursynth {
        return Some(Problem::VapourSynthMissing);
    }
    Some(Problem::Unknown)
}

/// The watch RVP keeps over SVP while a file plays.
#[derive(Debug, Default)]
pub struct Watch {
    pub status: Status,
    /// When to look whether SVP attached.
    due: Option<Instant>,
    /// The manager check running on a thread.
    pending: Option<(Receiver<Option<bool>>, Facts)>,
    /// Problems already shown this session.
    shown: Vec<Problem>,
    /// The endpoint mpv opened.
    pub endpoint: Option<String>,
    /// The VSScript library RVP pointed mpv at.
    pub vapoursynth: Option<PathBuf>,
}

impl Watch {
    /// A file started playing.
    pub fn file_started(&mut self, svp_mode: bool, now: Instant) {
        self.pending = None;
        if svp_mode {
            self.status = Status::Waiting;
            self.due = Some(now + ATTACH_WAIT);
        } else {
            self.status = Status::Off;
            self.due = None;
        }
    }
}

impl App {
    /// Looks whether SVP attached once [`ATTACH_WAIT`] passed; a problem is
    /// returned as a notice, once per session.
    pub(crate) fn watch_svp(&mut self, now: Instant) -> Option<UiRequest> {
        let svp_mode = svp_mode(&self.settings);
        let watch = &mut self.svp;
        if !svp_mode {
            watch.status = Status::Off;
            watch.due = None;
            watch.pending = None;
            return None;
        }
        let vf = self.model.text(PropertyId::VideoFilters).unwrap_or("");
        let filter_in_chain = vf.contains("@svp");
        if filter_in_chain && watch.status != Status::Attached && watch.due.is_none() {
            watch.status = Status::Attached;
        }
        if let Some((receiver, facts)) = &watch.pending {
            let manager = match receiver.try_recv() {
                Ok(manager) => manager,
                Err(std::sync::mpsc::TryRecvError::Empty) => return None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
            };
            let problem = diagnose(facts, manager);
            watch.pending = None;
            return match problem {
                None => {
                    watch.status = Status::Attached;
                    None
                }
                Some(problem) => {
                    watch.status = Status::NotAttached(problem.clone());
                    if watch.shown.contains(&problem) {
                        None
                    } else {
                        watch.shown.push(problem.clone());
                        Some(UiRequest::Message(problem.text()))
                    }
                }
            };
        }
        let due = watch.due?;
        if now < due || !self.model.playing() {
            return None;
        }
        watch.due = None;
        let source = self.model.double(PropertyId::ContainerFps).unwrap_or(0.0);
        let filtered = self.model.double(PropertyId::EstimatedVfFps).unwrap_or(0.0);
        let facts = Facts {
            endpoint: watch.endpoint.clone(),
            vapoursynth: watch.vapoursynth.is_some(),
            filter_in_chain,
            interpolating: source > 0.0 && filtered > source * 1.2,
        };
        if filter_in_chain && facts.interpolating {
            watch.status = Status::Attached;
            return None;
        }
        let (sender, receiver) = channel();
        let spawned = std::thread::Builder::new()
            .name("rvp-svp-check".to_owned())
            .spawn(move || {
                let _ = sender.send(manager_running());
            });
        if spawned.is_err() {
            return None;
        }
        watch.pending = Some((receiver, facts));
        None
    }

    /// SVP's status for the settings window.
    #[must_use]
    pub fn svp_status(&self) -> String {
        self.svp.status.text()
    }

    /// Before libmpv starts: picks the IPC endpoint for SVP mode (the
    /// configured one, or the `mpvSockets` name while another process
    /// holds it), which [`App::engine_options`] then uses, and records the
    /// VapourSynth runtime the process points mpv at.
    pub fn prepare_svp(&mut self, vapoursynth: Option<PathBuf>) {
        let endpoint = choose_endpoint(
            &self.settings.ipc_server,
            std::process::id(),
            endpoint_in_use,
        );
        if let Some(endpoint) = &endpoint {
            prepare_endpoint(endpoint);
        }
        self.svp.endpoint = endpoint;
        self.svp.vapoursynth = vapoursynth;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_back_variants() {
        assert_eq!(copy_back("auto-copy"), "auto-copy");
        assert_eq!(copy_back("auto"), "auto-copy");
        assert_eq!(copy_back("auto-safe"), "auto-copy");
        assert_eq!(copy_back("d3d11va"), "d3d11va-copy");
        assert_eq!(copy_back("nvdec-copy"), "nvdec-copy");
        assert_eq!(copy_back("no"), "no");
        assert_eq!(copy_back("vaapi,nvdec"), "vaapi-copy,nvdec-copy");
        assert_eq!(copy_back("auto-copy-safe"), "auto-copy-safe");
    }

    #[test]
    fn a_taken_endpoint_becomes_the_mpvsockets_name() {
        let free = choose_endpoint("mpvpipe", 42, |_| false);
        assert_eq!(free.as_deref(), Some("mpvpipe"));
        let taken = choose_endpoint("mpvpipe", 42, |_| true).expect("an endpoint");
        assert!(taken.ends_with("/mpvSockets/42"), "{taken}");
        assert!(is_known_endpoint(&taken));
        assert_eq!(choose_endpoint("  ", 42, |_| false), None);
        assert_eq!(
            mpv_sockets_endpoint(7, Path::new("C:\\Users\\x\\AppData\\Local\\Temp\\")),
            "C:\\Users\\x\\AppData\\Local\\Temp/mpvSockets/7"
        );
    }

    #[test]
    fn known_endpoints() {
        assert!(is_known_endpoint(rvp_engine::default_ipc_server()));
        assert!(is_known_endpoint("/tmp/mpvSockets/123"));
        assert!(!is_known_endpoint("/tmp/my-player"));
    }

    #[test]
    fn diagnoses_in_order_of_likelihood() {
        let facts = Facts {
            endpoint: Some(rvp_engine::default_ipc_server().to_owned()),
            vapoursynth: true,
            filter_in_chain: false,
            interpolating: false,
        };
        assert_eq!(
            diagnose(&facts, Some(false)),
            Some(Problem::ManagerNotRunning)
        );
        assert_eq!(diagnose(&facts, Some(true)), Some(Problem::Unknown));
        let custom = Facts {
            endpoint: Some("/tmp/other".to_owned()),
            ..facts.clone()
        };
        assert_eq!(
            diagnose(&custom, Some(true)),
            Some(Problem::EndpointMismatch("/tmp/other".to_owned()))
        );
        let no_vs = Facts {
            vapoursynth: false,
            ..facts.clone()
        };
        assert_eq!(diagnose(&no_vs, None), Some(Problem::VapourSynthMissing));
        let attached = Facts {
            filter_in_chain: true,
            interpolating: true,
            ..facts.clone()
        };
        assert_eq!(diagnose(&attached, None), None);
        let stuck = Facts {
            filter_in_chain: true,
            ..facts.clone()
        };
        assert_eq!(diagnose(&stuck, None), Some(Problem::FilterNotRunning));
        let none = Facts {
            endpoint: None,
            ..facts
        };
        assert_eq!(diagnose(&none, Some(true)), Some(Problem::NoEndpoint));
    }
}
