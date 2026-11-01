//! Shutting down, sleeping and hibernating through systemd-logind on the
//! system bus (M11's "When Playback Finished" and the shutdown timer).
//! Interactive calls: polkit may ask the user. Tests never get here; they
//! use the shell's dry-run hook.

use rvp_app::PowerAction;
use zbus::Connection;

/// Asks logind to carry out `action` now; waits for its answer on the
/// calling thread (the window's: a rare, deliberate action).
///
/// # Errors
/// Fails without a system bus or logind, or when polkit refuses.
pub fn perform(action: PowerAction) -> Result<(), String> {
    let method = match action {
        PowerAction::Shutdown => "PowerOff",
        PowerAction::Sleep => "Suspend",
        PowerAction::Hibernate => "Hibernate",
    };
    zbus::block_on(async {
        let connection = Connection::system()
            .await
            .map_err(|error| format!("the system bus: {error}"))?;
        connection
            .call_method(
                Some("org.freedesktop.login1"),
                "/org/freedesktop/login1",
                Some("org.freedesktop.login1.Manager"),
                method,
                &(true,),
            )
            .await
            .map(drop)
            .map_err(|error| format!("logind {method}: {error}"))
    })
}
