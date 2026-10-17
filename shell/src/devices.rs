//! Webcams and other capture devices for Open Webcam/Other Device (M11):
//! DirectShow's video input category on Windows, the names FFmpeg's
//! `dshow` opens them by. Linux's `v4l2` devices come with goal S.

use rvp_app::sources::CaptureDevice;

/// The capture devices the system lists now (empty when it lists none).
#[must_use]
pub(crate) fn list() -> Vec<CaptureDevice> {
    #[cfg(windows)]
    {
        windows_list().unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn windows_list() -> windows::core::Result<Vec<CaptureDevice>> {
    use windows::Win32::Media::DirectShow::ICreateDevEnum;
    use windows::Win32::System::Com::StructuredStorage::IPropertyBag;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IEnumMoniker, IMoniker,
    };
    use windows::Win32::System::Variant::{VARIANT, VariantClear};
    use windows::core::{GUID, w};

    // DirectShow's system device enumerator and its video input category
    // (strmif.h; windows-rs keeps them with Media Foundation).
    const CLSID_SYSTEM_DEVICE_ENUM: GUID =
        GUID::from_u128(0x62be_5d10_60eb_11d0_bd3b_00a0_c911_ce86);
    const CLSID_VIDEO_INPUT_DEVICE_CATEGORY: GUID =
        GUID::from_u128(0x860b_b310_5d01_11d0_bd3b_00a0_c911_ce86);

    let mut devices = Vec::new();
    // SAFETY: FFI point, on the window's thread. COM is joined in the
    // apartment Qt chose (balanced below); the enumerators, monikers and
    // property bags are reference-counted COM objects released when they
    // drop, before COM is left; the variant is cleared after its string is
    // copied; no pointer outlives a call.
    #[allow(unsafe_code)]
    unsafe {
        let joined = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| -> windows::core::Result<()> {
            let enumerator: ICreateDevEnum =
                CoCreateInstance(&CLSID_SYSTEM_DEVICE_ENUM, None, CLSCTX_INPROC_SERVER)?;
            let mut monikers: Option<IEnumMoniker> = None;
            enumerator.CreateClassEnumerator(
                &CLSID_VIDEO_INPUT_DEVICE_CATEGORY,
                &raw mut monikers,
                0,
            )?;
            // No devices: S_FALSE and no enumerator.
            let Some(monikers) = monikers else {
                return Ok(());
            };
            loop {
                let mut moniker: [Option<IMoniker>; 1] = [None];
                let mut fetched = 0;
                if monikers.Next(&mut moniker, Some(&raw mut fetched)).is_err() || fetched == 0 {
                    break;
                }
                let Some(moniker) = moniker[0].take() else {
                    break;
                };
                let Ok(bag) = moniker.BindToStorage::<_, _, IPropertyBag>(None, None) else {
                    continue;
                };
                let mut name = VARIANT::default();
                if bag.Read(w!("FriendlyName"), &raw mut name, None).is_ok() {
                    let text = name.to_string();
                    if !text.is_empty() {
                        devices.push(CaptureDevice {
                            name: text,
                            audio: false,
                        });
                    }
                }
                drop(VariantClear(&raw mut name));
            }
            Ok(())
        })();
        if joined {
            CoUninitialize();
        }
        result?;
    }
    Ok(devices)
}
