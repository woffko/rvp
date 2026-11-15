//! The desktop's media controls (M8): MPRIS and idle inhibit on Linux
//! (`rvp_platform::Desktop`, running with the media controls off too:
//! only MPRIS follows the General page's switch), the System Media
//! Transport Controls on Windows (`crate::media_controls`). Both show
//! `App::media_state` and send their commands to the window's thread
//! (`Player::media_command`). Windows'
//! counterpart of the idle inhibit is `crate::awake`. On Linux the desktop
//! also shows the taskbar's progress (the launcher API; Windows' is
//! `cpp/taskbar.cpp`) and reports the screen lock, as Windows' session
//! notifications do (`Player::window_attention`).

use cxx_qt::CxxQtThread;
use rvp_app::App;
use rvp_app::media::MediaState;
use rvp_engine::EngineEvent;

use crate::player_qobject::qobject;

/// What the desktop shows of RVP, kept in step with the application.
pub(crate) struct Integration {
    #[cfg(target_os = "linux")]
    desktop: rvp_platform::Desktop,
    #[cfg(target_os = "linux")]
    inhibited: bool,
    #[cfg(windows)]
    controls: crate::media_controls::Controls,
    /// The General page's media controls (MPRIS).
    #[cfg(target_os = "linux")]
    media_controls: bool,
    last: Option<MediaState>,
}

impl Integration {
    /// Starts the controls; `window` is the main window's native handle,
    /// which Windows' controls belong to. On Linux it starts without the
    /// media controls too (`media_controls` false: no MPRIS), for keeping
    /// the screen on, the launcher's progress and the screen lock, which
    /// Windows has without it (the owner, 2026-11-14).
    ///
    /// # Errors
    /// Fails where the desktop offers none (no session bus, no controls).
    pub(crate) fn start(
        thread: CxxQtThread<qobject::Player>,
        window: usize,
        media_controls: bool,
    ) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            let _ = window;
            let locks = thread.clone();
            let desktop = rvp_platform::Desktop::start(
                rvp_platform::Options {
                    desktop_entry: "rvp".to_owned(),
                    inhibit_reason: "Playing video".to_owned(),
                    mpris: media_controls,
                },
                move |command| {
                    // Fails only once the window is gone.
                    let _ = thread.queue(move |player| player.media_command(command));
                },
                move |locked| {
                    // Kind 2: the screen lock (Windows' session notification).
                    let _ = locks.queue(move |player| player.window_attention(2, locked));
                },
            )?;
            Ok(Self {
                desktop,
                inhibited: false,
                media_controls,
                last: None,
            })
        }
        #[cfg(windows)]
        {
            // Windows starts its controls only when they are on.
            let _ = media_controls;
            let controls = crate::media_controls::Controls::start(thread, window)?;
            Ok(Self {
                controls,
                last: None,
            })
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (thread, window, media_controls);
            Err("no media controls on this system".to_owned())
        }
    }

    /// Linux: the General page turned the media controls on or off; MPRIS
    /// follows, the rest goes on.
    #[cfg(target_os = "linux")]
    pub(crate) fn set_media_controls(&mut self, on: bool) {
        if on != self.media_controls {
            self.media_controls = on;
            self.desktop.mpris(on);
        }
    }

    /// The taskbar's progress (Linux's launcher; Windows' is the taskbar
    /// button's): `state` 0 nothing loaded, 1 playing, 2 paused, `progress`
    /// below 0 when unknown.
    // Windows' taskbar is the shell's own (`cpp/taskbar.cpp`).
    #[cfg_attr(not(target_os = "linux"), allow(clippy::unused_self))]
    pub(crate) fn progress(&self, state: i32, progress: f64) {
        #[cfg(target_os = "linux")]
        self.desktop.progress(state, progress);
        #[cfg(not(target_os = "linux"))]
        let _ = (state, progress);
    }

    /// Shows what changed after a drain. On Linux it also reports seeks
    /// and keeps the screen on while video plays.
    pub(crate) fn sync(&mut self, app: &App, events: &[EngineEvent]) {
        let state = app.media_state();
        #[cfg(target_os = "linux")]
        {
            if events
                .iter()
                .any(|event| matches!(event, EngineEvent::PlaybackRestart))
            {
                self.desktop.seeked(state.position);
            }
            let inhibit = app.keeps_screen_on();
            if inhibit != self.inhibited {
                self.inhibited = inhibit;
                self.desktop.inhibit(inhibit);
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = events;
        if self.last.as_ref() == Some(&state) {
            return;
        }
        #[cfg(target_os = "linux")]
        self.desktop.update(state.clone());
        #[cfg(windows)]
        self.controls.show(&state);
        self.last = Some(state);
    }
}
