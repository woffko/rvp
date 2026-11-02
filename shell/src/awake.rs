//! Keeps Windows' display and system awake while video plays (goal L):
//! the counterpart of the idle inhibit Linux gets through
//! `crate::desktop`. Both follow `App::keeps_screen_on`, which honours
//! "Prevent screen saver during playback".
//!
//! libmpv's own windows do this themselves, but RVP renders through the
//! render API into its own window, so mpv's screen saver handling never
//! runs. The execution state belongs to the calling thread: it is set and
//! cleared on the window's thread, which lives until RVP exits.

/// Whether the execution state asks for the display now.
#[derive(Default)]
pub(crate) struct KeepAwake {
    on: bool,
}

impl KeepAwake {
    /// Asks Windows to keep the display and the system on, or lets them
    /// sleep again; a no-op on other systems and when nothing changed.
    pub(crate) fn set(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        self.on = on;
        #[cfg(windows)]
        {
            use windows::Win32::System::Power::{
                ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
            };
            let flags = if on {
                ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
            } else {
                ES_CONTINUOUS
            };
            // SAFETY: FFI point. SetThreadExecutionState takes flags by value
            // and only records them for the calling thread (the window's,
            // alive until exit); there is no pointer and nothing to free. A
            // zero result means the call failed, which leaves the state as
            // it was.
            #[allow(unsafe_code)]
            let previous = unsafe { SetThreadExecutionState(flags) };
            if previous.0 == 0 {
                rvp_app::log::report(
                    rvp_app::log::Level::Warning,
                    "cannot change Windows' execution state",
                );
            }
        }
    }
}
