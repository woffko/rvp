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
        rvp_platform::pulse::change_master(change)
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
