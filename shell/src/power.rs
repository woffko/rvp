//! Shutting the computer down, sleeping or hibernating (M11's "When
//! Playback Finished" and the shutdown timer). Tests never get here: they
//! use the dry-run hook (`player::m11::power_dry_run`), which `--selftest`
//! always turns on. Linux asks systemd-logind (`rvp_platform::power`).

use rvp_app::PowerAction;

/// Carries out the action now.
///
/// # Errors
/// Fails when the system refuses it.
pub(crate) fn perform(action: PowerAction) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows_perform(action).map_err(|error| error.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        rvp_platform::power::perform(action)
    }
}

#[cfg(windows)]
fn windows_perform(action: PowerAction) -> windows::core::Result<()> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
    use windows::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::System::Power::SetSuspendState;
    use windows::Win32::System::Shutdown::{
        EWX_POWEROFF, ExitWindowsEx, SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: FFI point. The token handle is opened for this process and
    // closed below; the privilege structure lives on the stack for the
    // call and the previous state is not asked for (null); the power calls
    // take plain values. Each call's failure comes back as an error.
    #[allow(unsafe_code)]
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &raw mut token,
        )?;
        let mut luid = LUID::default();
        let looked_up = LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &raw mut luid);
        let privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        let adjusted = looked_up.and_then(|()| {
            AdjustTokenPrivileges(token, false, Some(&raw const privileges), 0, None, None)
        });
        drop(CloseHandle(token));
        adjusted?;
        match action {
            PowerAction::Shutdown => ExitWindowsEx(
                EWX_POWEROFF,
                SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED,
            ),
            PowerAction::Sleep | PowerAction::Hibernate => {
                let hibernate = action == PowerAction::Hibernate;
                if SetSuspendState(hibernate, false, false) {
                    Ok(())
                } else {
                    Err(windows::core::Error::from_thread())
                }
            }
        }
    }
}
