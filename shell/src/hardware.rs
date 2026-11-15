//! The machine's description for motion smoothing's calibration
//! (`rvp_app::calibration::key`): the processors, the graphics cards and
//! their drivers. A new card or driver measures again.

use std::num::NonZero;

/// The number of logical processors.
fn threads() -> usize {
    std::thread::available_parallelism().map_or(0, NonZero::get)
}

/// The processor's name, the adapters and their drivers (`cpp/gpu_power.cpp`).
#[cfg(windows)]
pub(crate) fn describe() -> String {
    format!(
        "{} processors; {}",
        threads(),
        crate::ffi::bridge::rvp_hardware()
    )
}

/// The processor's name, the cards' vendor and device IDs, NVIDIA's
/// driver version and the kernel's (Mesa's drivers come with the
/// system's updates).
#[cfg(not(windows))]
pub(crate) fn describe() -> String {
    let read = |path: &str| {
        std::fs::read_to_string(path)
            .map(|text| text.trim().to_owned())
            .unwrap_or_default()
    };
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split_once(':'))
                .map(|(_, name)| name.trim().to_owned())
        })
        .unwrap_or_default();
    let mut cards = std::collections::BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // `card0` and not its outputs (`card0-HDMI-A-1`).
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let device = entry.path().join("device");
            let id = |file: &str| {
                std::fs::read_to_string(device.join(file))
                    .map(|text| text.trim().to_owned())
                    .unwrap_or_default()
            };
            cards.insert(format!("{}:{}", id("vendor"), id("device")));
        }
    }
    // The C++ side describes nothing here today.
    let native = crate::ffi::bridge::rvp_hardware();
    // Not the kernel's release: a kernel update ran the calibration again
    // for the same card and driver (the owner, 2026-11-14, the Linux
    // audit's L3).
    format!(
        "{cpu}; {} processors; cards {}; nvidia {}{native}",
        threads(),
        cards.into_iter().collect::<Vec<_>>().join(","),
        read("/sys/module/nvidia/version")
    )
}
