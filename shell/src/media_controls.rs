//! Windows' System Media Transport Controls (M8): the media keys and the
//! flyout with the title, play, pause, next and previous. The controls
//! belong to the main window, so getting them is a COM call with its handle
//! (one FFI point, AGENTS.md); button presses arrive on a system thread and
//! go to the window's thread.

use cxx_qt::CxxQtThread;
use rvp_app::media::{MediaCommand, MediaState, Status};
use windows::Foundation::TypedEventHandler;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;
use windows::core::{HSTRING, Ref, factory};

use crate::player_qobject::qobject;

/// What the flyout shows: status, title, next and previous.
type Shown = (Status, String, bool, bool);

/// The main window's media controls.
pub(crate) struct Controls {
    controls: SystemMediaTransportControls,
    shown: Option<Shown>,
}

fn error(error: &windows::core::Error) -> String {
    format!("media controls: {error}")
}

impl Controls {
    /// The controls of the window with this handle.
    ///
    /// # Errors
    /// Fails when Windows has no controls for the window.
    pub(crate) fn start(
        thread: CxxQtThread<qobject::Player>,
        window: usize,
    ) -> Result<Self, String> {
        let interop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
                .map_err(|e| error(&e))?;
        // SAFETY: FFI point. `window` is the main window's handle
        // (`QWindow::winId`), which exists until the process exits;
        // `GetForWindow` only reads it to find that window's controls and
        // returns a reference-counted COM object.
        #[allow(unsafe_code)]
        let controls: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(window as *mut core::ffi::c_void)) }
                .map_err(|e| error(&e))?;
        for enable in [
            SystemMediaTransportControls::SetIsEnabled,
            SystemMediaTransportControls::SetIsPlayEnabled,
            SystemMediaTransportControls::SetIsPauseEnabled,
            SystemMediaTransportControls::SetIsStopEnabled,
        ] {
            enable(&controls, true).map_err(|e| error(&e))?;
        }
        controls
            .ButtonPressed(&TypedEventHandler::new(
                move |_: Ref<SystemMediaTransportControls>,
                      args: Ref<SystemMediaTransportControlsButtonPressedEventArgs>| {
                let command = match args.ok()?.Button()? {
                    SystemMediaTransportControlsButton::Play => MediaCommand::Play,
                    SystemMediaTransportControlsButton::Pause => MediaCommand::Pause,
                    SystemMediaTransportControlsButton::Stop => MediaCommand::Stop,
                    SystemMediaTransportControlsButton::Next => MediaCommand::Next,
                    SystemMediaTransportControlsButton::Previous => MediaCommand::Previous,
                    _ => return Ok(()),
                };
                // Fails only once the window is gone.
                let _ = thread.queue(move |player| player.media_command(command));
                Ok(())
            }))
            .map_err(|e| error(&e))?;
        Ok(Self {
            controls,
            shown: None,
        })
    }

    /// Shows the state where it changed what the flyout shows.
    pub(crate) fn show(&mut self, state: &MediaState) {
        let shown = (
            state.status,
            state.title.clone(),
            state.can_next,
            state.can_previous,
        );
        if self.shown.as_ref() == Some(&shown) {
            return;
        }
        // The flyout shows what it can; a failed update is not retried.
        drop(self.update(state));
        self.shown = Some(shown);
    }

    fn update(&self, state: &MediaState) -> windows::core::Result<()> {
        self.controls.SetPlaybackStatus(match state.status {
            Status::Playing => MediaPlaybackStatus::Playing,
            Status::Paused => MediaPlaybackStatus::Paused,
            Status::Stopped => MediaPlaybackStatus::Stopped,
        })?;
        self.controls.SetIsNextEnabled(state.can_next)?;
        self.controls.SetIsPreviousEnabled(state.can_previous)?;
        let updater = self.controls.DisplayUpdater()?;
        if state.status == Status::Stopped {
            return updater.ClearAll();
        }
        updater.SetType(MediaPlaybackType::Video)?;
        updater
            .VideoProperties()?
            .SetTitle(&HSTRING::from(state.title.as_str()))?;
        updater.Update()
    }
}
