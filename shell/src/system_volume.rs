//! The system's master volume (goal L): PotPlayer's "System Master volume
//! control" method and its System Volume menu change the default output
//! device's volume, through Windows' Core Audio (one FFI block, AGENTS.md).
//! The wave volume is mpv's `ao-volume` (`rvp_app::audio::commands`).
//! On Linux the default output's volume goes through PulseAudio's `pactl`,
//! which PipeWire answers too (`rvp_platform::pulse`).

use rvp_app::audio::commands::SystemVolume;

/// Changes the default output device's volume; returns what the OSD shows.
///
/// # Errors
/// Fails when the system has no device or refuses the change.
pub(crate) fn change(change: SystemVolume) -> Result<String, String> {
    #[cfg(windows)]
    {
        windows_change(change).map_err(|error| format!("Master volume: {error}"))
    }
    #[cfg(target_os = "linux")]
    {
        rvp_platform::pulse::change_master(change).map_err(|error| {
            if error.starts_with("Master volume") {
                error
            } else {
                format!("Master volume: {error}")
            }
        })
    }
}

/// Linux: the changes `pactl` carries out on a thread of its own, so the
/// window's thread never waits for the sound server (a call can take up to
/// 2 s, a hung server 4 s a step; the Linux audit's M3). Changes asked
/// while one runs wait together: steps add up, two mutes cancel, and the
/// OSD shows the last change's text.
#[cfg(target_os = "linux")]
pub(crate) fn change_later(
    thread: cxx_qt::CxxQtThread<crate::player_qobject::qobject::Player>,
    change: SystemVolume,
) {
    use std::sync::{Condvar, Mutex, OnceLock};
    struct Queue {
        changes: Mutex<Vec<SystemVolume>>,
        wake: Condvar,
    }
    static QUEUE: OnceLock<Queue> = OnceLock::new();
    let mut started = false;
    let queue = QUEUE.get_or_init(|| {
        started = true;
        Queue {
            changes: Mutex::new(Vec::new()),
            wake: Condvar::new(),
        }
    });
    if started {
        let spawned = std::thread::Builder::new()
            .name("rvp-volume".to_owned())
            .spawn(move || {
                let queue = QUEUE.get().expect("the queue");
                loop {
                    let changes = {
                        let mut waiting = queue
                            .changes
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        while waiting.is_empty() {
                            waiting = queue
                                .wake
                                .wait(waiting)
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                        }
                        std::mem::take(&mut *waiting)
                    };
                    let mut text = None;
                    for change in changes {
                        text = Some(self::change(change).unwrap_or_else(|error| error));
                    }
                    if let Some(text) = text {
                        // Fails only once the window is gone.
                        let _ = thread.queue(move |player| {
                            player.set_osd(cxx_qt_lib::QString::from(&text));
                        });
                    }
                }
            });
        if let Err(error) = spawned {
            rvp_app::log::report(
                rvp_app::log::Level::Warning,
                &format!("volume thread: {error}"),
            );
            return;
        }
    }
    let mut waiting = queue
        .changes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    merge(&mut waiting, change);
    queue.wake.notify_one();
}

/// Adds `change` to the changes waiting: a step after a step adds up, a
/// mute after a mute takes both back.
#[cfg(any(target_os = "linux", test))]
fn merge(waiting: &mut Vec<SystemVolume>, change: SystemVolume) {
    match (waiting.last_mut(), change) {
        (Some(SystemVolume::MasterStep(sum)), SystemVolume::MasterStep(delta)) => *sum += delta,
        (Some(SystemVolume::MasterMute), SystemVolume::MasterMute) => {
            waiting.pop();
        }
        _ => waiting.push(change),
    }
}

#[cfg(windows)]
fn windows_change(change: SystemVolume) -> windows::core::Result<String> {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoInitializeEx};
    use windows::Win32::System::Com::{CoCreateInstance, CoUninitialize};

    // SAFETY: FFI point. This runs on the window's thread. COM is joined in
    // the apartment Qt already chose for it (a second join only counts up
    // and is balanced below); the enumerator, the device and its volume are
    // reference-counted COM objects released when they drop, before COM is
    // left; the event context GUID may be null; no pointer outlives a call.
    #[allow(unsafe_code)]
    unsafe {
        let joined = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            match change {
                SystemVolume::MasterStep(delta) | SystemVolume::MasterSet(delta) => {
                    let current = volume.GetMasterVolumeLevelScalar()?;
                    // Volume steps are a few percent.
                    #[allow(clippy::cast_possible_truncation)]
                    let level = match change {
                        SystemVolume::MasterStep(_) => current + (delta / 100.0) as f32,
                        _ => delta as f32,
                    }
                    .clamp(0.0, 1.0);
                    volume.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
                    Ok(format!("Master volume: {:.0} %", level * 100.0))
                }
                SystemVolume::MasterMute => {
                    let mute = !volume.GetMute()?.as_bool();
                    volume.SetMute(mute, std::ptr::null())?;
                    Ok(format!(
                        "Master volume: {}",
                        if mute { "mute" } else { "on" }
                    ))
                }
            }
        })();
        if joined {
            CoUninitialize();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_changes_add_up() {
        let mut waiting = Vec::new();
        merge(&mut waiting, SystemVolume::MasterStep(2.0));
        merge(&mut waiting, SystemVolume::MasterStep(3.0));
        assert_eq!(waiting, [SystemVolume::MasterStep(5.0)]);
        // Two mutes take each other back.
        merge(&mut waiting, SystemVolume::MasterMute);
        merge(&mut waiting, SystemVolume::MasterMute);
        assert_eq!(waiting, [SystemVolume::MasterStep(5.0)]);
        merge(&mut waiting, SystemVolume::MasterSet(0.5));
        merge(&mut waiting, SystemVolume::MasterStep(-1.0));
        assert_eq!(
            waiting,
            [
                SystemVolume::MasterStep(5.0),
                SystemVolume::MasterSet(0.5),
                SystemVolume::MasterStep(-1.0)
            ]
        );
    }
}
