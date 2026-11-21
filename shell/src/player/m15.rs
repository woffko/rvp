//! M15's part of `Player`: the control API (`rvp_app::control`) for
//! remotes. Its server's threads hand commands to the window's thread
//! through `CxxQtThread::queue`, as the desktop's media controls do, and
//! the window's thread publishes the status as the chrome updates; the
//! Remote page (`Settings.qml`) turns it on and shows its address and
//! token.

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
        let token = match control::token::load_or_create(core.app.state_dir()) {
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
            .and_then(|core| control::token::load_or_create(core.app.state_dir()).ok())
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
        core.remote.server = None;
        self.as_mut().apply_remote();
        QString::from(&token)
    }
}
