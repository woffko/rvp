//! Reads the graphics cards' power for the GPU power budget
//! (`rvp_app::gpu_power`, D17): once a second, on a thread of its own,
//! NVIDIA's cards through NVML and, on Windows, AMD's through Windows'
//! adapter counter (`cpp/gpu_power.cpp`), and on Linux amdgpu's hwmon
//! files, leaving cards the system put to sleep alone. A hung card can
//! hold such a call up, so the window's thread never makes one: it takes
//! the readings from the channel (`App::gpu_tick`).

use std::sync::mpsc;
use std::time::Duration;

use rvp_app::gpu_power::{CardState, Sample, Unit};

/// How often the cards are read.
const INTERVAL: Duration = Duration::from_secs(1);

/// Starts a reader; it ends at the first reading after `sender`'s
/// receiver is gone, and lets NVML and the adapters go.
pub(crate) fn start(sender: mpsc::Sender<Vec<Sample>>) {
    let spawned = std::thread::Builder::new()
        .name("rvp-gpu-power".to_owned())
        .spawn(move || {
            while sender.send(read()).is_ok() {
                std::thread::sleep(INTERVAL);
            }
            crate::ffi::bridge::rvp_gpu_power_close();
        });
    if let Err(error) = spawned {
        rvp_app::log::report(
            rvp_app::log::Level::Warning,
            &format!("cannot read the GPU's power: {error}"),
        );
    }
}

/// Every card's reading now.
fn read() -> Vec<Sample> {
    // NVML would wake a sleeping card (a laptop's, on Linux).
    #[cfg(target_os = "linux")]
    let nvidia = rvp_app::gpu_power::nvidia_awake(std::path::Path::new("/sys/bus/pci/devices"));
    #[cfg(not(target_os = "linux"))]
    let nvidia = true;
    let lines = crate::ffi::bridge::rvp_gpu_power_read(nvidia);
    let samples = lines.lines().filter_map(parse);
    #[cfg(target_os = "linux")]
    let samples = samples.chain(rvp_app::gpu_power::amdgpu_samples(std::path::Path::new(
        "/sys/class/drm",
    )));
    samples.collect()
}

/// One line of the C++ side: `unit<TAB>draw<TAB>rated<TAB>load<TAB>
/// graphics<TAB>memory<TAB>temperature<TAB>pstate<TAB>name`, the unit `W`
/// (watts) or `%` (a share in percent), -1 for a part the reader lacks.
fn parse(line: &str) -> Option<Sample> {
    let mut fields = line.splitn(9, '\t');
    let unit = match fields.next()? {
        "W" => Unit::Watts,
        "%" => Unit::Percent,
        _ => return None,
    };
    let draw = fields.next()?.parse().ok()?;
    let rated = fields.next()?.parse().ok()?;
    let mut part = || -> Option<Option<u32>> {
        let value: i64 = fields.next()?.parse().ok()?;
        Some(u32::try_from(value).ok())
    };
    let state = CardState {
        load: part()?,
        graphics_mhz: part()?,
        memory_mhz: part()?,
        temperature: part()?,
        pstate: part()?,
    };
    let name = fields.next()?.to_owned();
    Some(Sample {
        name,
        draw,
        rated,
        unit,
        state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_native_lines_become_samples() {
        assert_eq!(
            parse("W\t182.400\t300.000\t54\t2790\t13801\t61\t1\tRTX 50-series card"),
            Some(Sample {
                name: "RTX 50-series card".to_owned(),
                draw: 182.4,
                rated: 300.0,
                unit: Unit::Watts,
                state: CardState {
                    load: Some(54),
                    graphics_mhz: Some(2790),
                    memory_mhz: Some(13801),
                    temperature: Some(61),
                    pstate: Some(1),
                },
            })
        );
        // AMD's adapter counter has no state.
        assert_eq!(
            parse("%\t42.5\t100\t-1\t-1\t-1\t-1\t-1\tAMD Radeon RX 7800 XT")
                .map(|sample| (sample.unit, sample.state)),
            Some((Unit::Percent, CardState::default()))
        );
        assert_eq!(parse("W\t36.4\tNVIDIA"), None);
        assert_eq!(parse("kW\t1\t2\tcard"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn this_machines_cards_read_sensibly() {
        // No card (CI, the VMs) reads as none; a card has a rated power.
        let samples = read();
        for sample in &samples {
            println!("{sample:?}");
            assert!(sample.rated > 0.0 && sample.draw >= 0.0, "{sample:?}");
        }
        crate::ffi::bridge::rvp_gpu_power_close();
    }
}
