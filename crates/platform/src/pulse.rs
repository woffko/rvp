//! The system's sound through PulseAudio's tools (`pactl`, `parec` from
//! pulseaudio-utils), which PipeWire's pulse server answers too: the
//! default output's master volume and mute (goal L's System Master volume
//! control), and the stream RVP itself plays, for recording it (goal L's
//! Audio Recording). The tools run with the C locale, so their output is
//! the same everywhere, and each call has a time limit.

use std::io::Read as _;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use rvp_app::audio::commands::SystemVolume;

/// How long one `pactl` call may take.
const TIMEOUT: Duration = Duration::from_secs(2);
/// The recording's format, which `parec` converts RVP's stream to.
pub const RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

/// Changes the default output's volume; returns what the OSD shows.
///
/// # Errors
/// Fails without `pactl` or a sound server, or when the server refuses.
pub fn change_master(change: SystemVolume) -> Result<String, String> {
    match change {
        SystemVolume::MasterStep(_) | SystemVolume::MasterSet(_) => {
            let level = match change {
                SystemVolume::MasterStep(delta) => {
                    let current =
                        volume_percent(&pactl(&["get-sink-volume", "@DEFAULT_SINK@"])?)
                            .ok_or_else(|| "Master volume: pactl gave no volume".to_owned())?;
                    current + delta
                }
                SystemVolume::MasterSet(fraction) => fraction * 100.0,
                SystemVolume::MasterMute => unreachable!("handled below"),
            }
            .clamp(0.0, 100.0)
            .round();
            pactl(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{level:.0}%")])?;
            Ok(format!("Master volume: {level:.0} %"))
        }
        SystemVolume::MasterMute => {
            let muted = pactl(&["get-sink-mute", "@DEFAULT_SINK@"])?.contains("yes");
            pactl(&[
                "set-sink-mute",
                "@DEFAULT_SINK@",
                if muted { "0" } else { "1" },
            ])?;
            Ok(format!(
                "Master volume: {}",
                if muted { "on" } else { "mute" }
            ))
        }
    }
}

/// The index of the first playback stream (sink input) of process `pid`:
/// the main player's, which starts before the second audio device's.
///
/// # Errors
/// Fails without `pactl` or a sound server.
pub fn stream_of(pid: u32) -> Result<Option<u32>, String> {
    Ok(stream_in(&pactl(&["list", "sink-inputs"])?, pid))
}

/// Starts `parec` on the stream `index`: its standard output is the
/// stream's sound as 32-bit float stereo at [`RATE`].
///
/// # Errors
/// Fails when `parec` cannot start.
pub fn record_stream(index: u32) -> Result<Child, String> {
    Command::new("parec")
        .args([
            &format!("--monitor-stream={index}"),
            "--format=float32le",
            &format!("--rate={RATE}"),
            &format!("--channels={CHANNELS}"),
            "--latency-msec=50",
        ])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Audio recording: parec (pulseaudio-utils): {error}"))
}

/// Runs `pactl` with `args` and returns its output.
fn pactl(args: &[&str]) -> Result<String, String> {
    let mut child = Command::new("pactl")
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("pactl (pulseaudio-utils): {error}"))?;
    let mut stdout = child.stdout.take().ok_or("pactl: no output")?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let read = stdout.read_to_string(&mut text);
        drop(sender.send(read.map(|_| text)));
    });
    let text = match receiver.recv_timeout(TIMEOUT) {
        Ok(Ok(text)) => text,
        Ok(Err(error)) => return Err(format!("pactl: {error}")),
        Err(_) => {
            drop(child.kill());
            drop(child.wait());
            return Err("pactl: the sound server did not answer".to_owned());
        }
    };
    let status = child.wait().map_err(|error| format!("pactl: {error}"))?;
    if status.success() {
        Ok(text)
    } else {
        let mut error = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            drop(stderr.read_to_string(&mut error));
        }
        Err(format!("pactl {}: {}", args.join(" "), error.trim()))
    }
}

/// The first channel's percent in `pactl get-sink-volume`'s answer
/// (`Volume: front-left: 32768 /  50% / -18.06 dB, ...`).
fn volume_percent(text: &str) -> Option<f64> {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("Volume:"))?;
    let percent = line.split('/').nth(1)?.trim().strip_suffix('%')?;
    percent.trim().parse().ok()
}

/// The first sink input in `pactl list sink-inputs`'s answer whose
/// `application.process.id` is `pid`.
fn stream_in(text: &str, pid: u32) -> Option<u32> {
    let wanted = format!("application.process.id = \"{pid}\"");
    let mut current = None;
    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(index) = line.strip_prefix("Sink Input #") {
            current = index.trim().parse::<u32>().ok();
        } else if line == wanted
            && let Some(index) = current
        {
            found.push(index);
        }
    }
    found.into_iter().min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_volume_is_read_from_the_first_channel() {
        let text = "Volume: front-left: 26214 /  40% / -23.88 dB,   front-right: 26214 /  40% / -23.88 dB\n        balance 0.00\n";
        assert_eq!(volume_percent(text), Some(40.0));
        assert_eq!(volume_percent("Mute: no\n"), None);
    }

    #[test]
    fn rvps_first_stream_is_found_by_its_process() {
        let text = "Sink Input #71\n\tDriver: protocol-native.c\n\tProperties:\n\t\tapplication.name = \"Firefox\"\n\t\tapplication.process.id = \"999\"\n\nSink Input #84\n\tProperties:\n\t\tmedia.name = \"film.mkv\"\n\t\tapplication.process.id = \"4242\"\nSink Input #80\n\tProperties:\n\t\tapplication.process.id = \"4242\"\n";
        assert_eq!(stream_in(text, 4242), Some(80));
        assert_eq!(stream_in(text, 999), Some(71));
        assert_eq!(stream_in(text, 1), None);
    }
}
