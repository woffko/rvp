//! M15's part of `Player`: the control API (`rvp_app::control`) for
//! remotes. Its server's threads hand commands to the window's thread
//! through `CxxQtThread::queue`, as the desktop's media controls do, and
//! the window's thread publishes the status as the chrome updates; the
//! Remote page (`Settings.qml`) turns it on and shows its address and
//! token.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::Threading;
use cxx_qt_lib::QString;
use rvp_app::control::{self, Command, Reply, Server, Track};
use rvp_engine::{EngineEvent, PropertyId};
use serde_json::json;

use super::Core;
use crate::player_qobject::qobject;

/// The control API's part of the player's core.
#[derive(Default)]
pub(super) struct Remote {
    server: Option<Server>,
    /// The port it was asked to listen on, and why it does not.
    port: u16,
    error: Option<String>,
    audio: Vec<Track>,
    subtitles: Vec<Track>,
    /// The tracks are read again at the next publish.
    tracks_stale: bool,
    /// Commands carried out (the self-test counts those from outside).
    commands: u64,
}

impl Remote {
    /// A file opened or closed, or another track plays: the track lists
    /// change.
    fn note(&mut self, events: &[EngineEvent], changes: rvp_engine::Changes) {
        if self.server.is_some()
            && (events.iter().any(|event| {
                matches!(
                    event,
                    EngineEvent::FileLoaded { .. } | EngineEvent::EndFile { .. }
                )
            }) || changes.contains(PropertyId::Aid)
                || changes.contains(PropertyId::Sid))
        {
            self.tracks_stale = true;
        }
    }

    /// Shows the status to the API's clients.
    pub(super) fn publish(
        &mut self,
        core_app: &rvp_app::App,
        engine: &rvp_engine::Engine,
        fullscreen: bool,
    ) {
        let Some(server) = self.server.as_ref() else {
            return;
        };
        if self.tracks_stale {
            // One read of mpv's track list, only when it changed.
            (self.audio, self.subtitles) = control::tracks(&engine.tracks());
            self.tracks_stale = false;
        }
        let state = core_app.control_state(fullscreen, &self.audio, &self.subtitles);
        server.publish(core_app.control_position(), &state);
    }
}

impl Core {
    /// What follows a drain outside the window: the desktop's media
    /// controls, the display kept on while video plays, and the control
    /// API's track lists.
    pub(super) fn show_drain(&mut self, drained: &rvp_app::Drained) {
        if let Some(desktop) = self.desktop.as_mut() {
            desktop.sync(&self.app, &drained.events);
        }
        self.awake.set(self.app.keeps_screen_on());
        self.remote.note(&drained.events, drained.changes);
    }
}

impl qobject::Player {
    /// Starts, moves or stops the control API as the Remote page says.
    pub(super) fn apply_remote(mut self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let settings = core.app.settings.remote.clone();
        let remote = &mut core.remote;
        if !settings.enabled {
            remote.server = None;
            remote.error = None;
            return;
        }
        if remote.server.is_some() && remote.port == settings.port {
            return;
        }
        // The old server lets its port go first.
        remote.server = None;
        remote.port = settings.port;
        let token = match load_token(core.app.state_dir()) {
            Ok(token) => token,
            Err(error) => {
                remote.error = Some(format!("no token: {error}"));
                return;
            }
        };
        let dispatch: control::Dispatch = Arc::new(move |command: Command, reply: Reply| {
            // Fails only once the window is gone: the reply is dropped and
            // the client hears that RVP did not answer.
            let _ = thread.queue(move |player| player.control_command(command, &reply));
        });
        match Server::start(settings.port, token, dispatch) {
            Ok(server) => {
                remote.server = Some(server);
                remote.error = None;
                remote.tracks_stale = true;
            }
            Err(error) => remote.error = Some(error.to_string()),
        }
        self.publish_remote();
    }

    /// Shows the status to the API's clients (with the chrome's updates).
    pub(super) fn publish_remote(mut self: Pin<&mut Self>) {
        let fullscreen = *self.fullscreen();
        let Some(core) = self.as_mut().core_mut() else {
            return;
        };
        let Core {
            remote,
            app,
            engine,
            ..
        } = core;
        remote.publish(app, engine, fullscreen);
    }

    /// A command from the control API, on the window's thread: actions run
    /// as keys and menus run them.
    pub(crate) fn control_command(mut self: Pin<&mut Self>, command: Command, reply: &Reply) {
        if self.as_mut().core_mut().is_none() {
            let _ = reply.send(Err((503, "RVP is starting".to_owned())));
            return;
        }
        let answer = if let Command::Action(action) = command {
            if let Some(core) = self.as_mut().core_mut() {
                core.remote.commands += 1;
            }
            self.as_mut().perform(action);
            Ok(json!({}))
        } else {
            let Some(core) = self.as_mut().core_mut() else {
                return;
            };
            core.remote.commands += 1;
            let (answer, request) = core.app.control_command(core.engine, command);
            if let Some(request) = request {
                self.as_mut().handle_request(request);
            }
            answer
        };
        self.as_mut().sync_chrome();
        let _ = reply.send(answer);
    }

    /// The Remote page's view: `enabled`, `listening`, `address`, the
    /// `error` that stops it and the `commands` carried out; for the
    /// self-test, the server's requests by how they showed the token
    /// (`bearer`, `signed`) and those it `refused`.
    pub(crate) fn control_info(&self) -> QString {
        let Some(core) = self.core() else {
            return QString::from("{}");
        };
        let remote = &core.remote;
        let address = remote
            .server
            .as_ref()
            .map(|server| format!("http://{}/", server.address()));
        let counts = remote
            .server
            .as_ref()
            .map(Server::counts)
            .unwrap_or_default();
        QString::from(
            &json!({
                "enabled": core.app.settings.remote.enabled,
                "listening": remote.server.is_some(),
                "address": address,
                "error": remote.error,
                "commands": remote.commands,
                "bearer": counts.bearer,
                "signed": counts.signed,
                "refused": counts.refused,
            })
            .to_string(),
        )
    }

    /// The installation's token for the Remote page (made on first use).
    pub(crate) fn control_token(&self) -> QString {
        self.core()
            .and_then(|core| load_token(core.app.state_dir()).ok())
            .map_or_else(QString::default, |token| QString::from(&token))
    }

    /// Linux: copies RVP's remote into Unified Remote Server's custom
    /// remotes and asks the server to read them again (the Remote Control
    /// page's button; Windows' installer does this). Returns what to show.
    pub(crate) fn add_unified_remote(&self) -> QString {
        use rvp_app::control::unified;
        let Some(source) = std::env::current_exe()
            .ok()
            .and_then(|exe| unified::remote_source(&exe))
        else {
            return QString::from("RVP's remote folder is missing beside RVP.");
        };
        let Some(home) = std::env::var_os("HOME") else {
            return QString::from("The home folder is unknown.");
        };
        let text = match unified::install(&source, &std::path::Path::new(&home).join(".urserver")) {
            Ok(target) => {
                // The server's answer can take seconds: asked on a thread,
                // and the OSD says how it went (the Linux audit's L6).
                let thread = self.qt_thread();
                let folder = target.display().to_string();
                std::thread::spawn(move || {
                    let text = if unified::reload() {
                        format!("RVP's remote is in Unified Remote ({folder}).")
                    } else {
                        format!(
                            "RVP's remote is in {folder}; Unified Remote Server shows it when it starts again."
                        )
                    };
                    // Fails only once the window is gone.
                    let _ = thread.queue(move |player| player.set_osd(QString::from(&text)));
                });
                format!(
                    "RVP's remote is in {}; Unified Remote Server is asked to read it.",
                    target.display()
                )
            }
            Err(error) => error,
        };
        QString::from(&text)
    }

    /// A new token; the API restarts with it, so the old one stops working.
    pub(crate) fn renew_control_token(mut self: Pin<&mut Self>) -> QString {
        let Some(core) = self.as_mut().core_mut() else {
            return QString::default();
        };
        let Ok(token) = control::token::renew(core.app.state_dir()) else {
            return QString::default();
        };
        keep_token_private(core.app.state_dir());
        core.remote.server = None;
        self.as_mut().apply_remote();
        QString::from(&token)
    }
}

/// The installation's token (made on first use), its file kept for the
/// user alone ([`keep_token_private`]).
fn load_token(state_dir: &Path) -> std::io::Result<String> {
    let token = control::token::load_or_create(state_dir)?;
    keep_token_private(state_dir);
    Ok(token)
}

/// Windows: gives the token's file access rules of its own, the user and
/// the system alone, not taken from its folder. A portable RVP's state
/// folder is beside the program, whose folder others may read; in the
/// profile the rules come out the same. On Unix the file is 0600 already
/// (`rvp_app::control::token`).
fn keep_token_private(state_dir: &Path) {
    #[cfg(windows)]
    if let Err(error) = owner_only(&state_dir.join(control::token::FILE)) {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("the control API's token keeps its folder's access rules: {error}"),
        );
    }
    #[cfg(not(windows))]
    let _ = state_dir;
}

/// Replaces `path`'s access rules by full access for the current user and
/// SYSTEM, protected from the folder's inherited ones.
#[cfg(windows)]
fn owner_only(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW};
    use windows::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAce, CreateWellKnownSid,
        DACL_SECURITY_INFORMATION, GetLengthSid, GetTokenInformation, InitializeAcl,
        PROTECTED_DACL_SECURITY_INFORMATION, PSID, TOKEN_QUERY, TOKEN_USER, TokenUser,
        WinLocalSystemSid,
    };
    use windows::Win32::Storage::FileSystem::FILE_ALL_ACCESS;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // The process token's user: TOKEN_USER with its SID after it, in a
    // buffer of u64s so the structure's pointer is aligned.
    let mut token = HANDLE::default();
    // SAFETY: FFI point. GetCurrentProcess returns a pseudo handle;
    // OpenProcessToken writes a token handle into `token`, closed below.
    #[allow(unsafe_code)]
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) };
    opened.map_err(|error| format!("no process token: {error}"))?;
    let mut size = 0u32;
    // SAFETY: FFI point. The first call only asks for the size (no buffer);
    // the second fills a buffer of at least that many bytes, which
    // outlives every use of the SID inside it; the token is closed once.
    #[allow(unsafe_code)]
    let user = unsafe {
        let _ = GetTokenInformation(token, TokenUser, None, 0, &raw mut size);
        let mut buffer = vec![0u64; usize::try_from(size).unwrap_or(0).div_ceil(8)];
        let read = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            size,
            &raw mut size,
        );
        drop(CloseHandle(token));
        read.map(|()| buffer)
    };
    let user = user.map_err(|error| format!("no user in the process token: {error}"))?;
    if user.len() * 8 < std::mem::size_of::<TOKEN_USER>() {
        return Err("no user in the process token".to_owned());
    }
    // SAFETY: the buffer holds a TOKEN_USER Windows wrote, aligned for it;
    // the SID it points to is inside the buffer, which lives until the end
    // of this function.
    #[allow(unsafe_code)]
    let user_sid = unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    // SYSTEM's SID: at most SECURITY_MAX_SID_SIZE (68) bytes.
    let mut system = [0u64; 9];
    let mut system_size = u32::try_from(std::mem::size_of_val(&system)).unwrap_or(0);
    let system_sid = PSID(system.as_mut_ptr().cast());
    // SAFETY: FFI point. `system` holds `system_size` bytes, more than any
    // SID takes.
    #[allow(unsafe_code)]
    let made = unsafe {
        CreateWellKnownSid(
            WinLocalSystemSid,
            None,
            Some(system_sid),
            &raw mut system_size,
        )
    };
    made.map_err(|error| format!("no SYSTEM SID: {error}"))?;
    // SAFETY: FFI point. Both SIDs are valid: Windows wrote them above.
    #[allow(unsafe_code)]
    let sids = unsafe { GetLengthSid(user_sid) + GetLengthSid(system_sid) };
    // The list's header and two ACEs, each the structure less its first
    // SID word, then its SID.
    let ace = std::mem::size_of::<ACCESS_ALLOWED_ACE>() - std::mem::size_of::<u32>();
    let acl_size = std::mem::size_of::<ACL>()
        + 2 * ace
        + usize::try_from(sids).map_err(|error| error.to_string())?;
    let mut acl = vec![0u32; acl_size.div_ceil(4)];
    let acl_ptr = acl.as_mut_ptr().cast::<ACL>();
    let length = u32::try_from(acl.len() * 4).map_err(|error| error.to_string())?;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: FFI point. `acl` is a DWORD-aligned buffer of `length`
    // bytes, enough for the header and both ACEs; the SIDs are valid;
    // `wide` is the NUL-terminated path. Windows copies the list into the
    // file's security descriptor and keeps no pointer to it.
    #[allow(unsafe_code)]
    let set = unsafe {
        InitializeAcl(acl_ptr, length, ACL_REVISION)
            .and_then(|()| AddAccessAllowedAce(acl_ptr, ACL_REVISION, FILE_ALL_ACCESS.0, user_sid))
            .and_then(|()| {
                AddAccessAllowedAce(acl_ptr, ACL_REVISION, FILE_ALL_ACCESS.0, system_sid)
            })
            .map(|()| {
                SetNamedSecurityInfoW(
                    windows::core::PCWSTR(wide.as_ptr()),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    None,
                    None,
                    Some(acl_ptr.cast_const()),
                    None,
                )
            })
    };
    let set = set.map_err(|error| error.to_string())?;
    if set.0 == 0 {
        Ok(())
    } else {
        Err(format!("Windows error {}", set.0))
    }
}
