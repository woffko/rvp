//! Impulse responses for the filters that convolve (M10), computed by RVP
//! and written as 32-bit float WAV files that the graphs read:
//!
//! * Freeverb ([`freeverb`]): Jezar at Dreampoint's public-domain reverb
//!   (eight comb filters with damping and four all-pass filters per side),
//!   run on an impulse. Freeverb is linear: what it does to a sound is this
//!   response convolved with it (FFmpeg's `afir`), with the wet and dry
//!   levels applied outside. Its delays are its tuning's sample counts at
//!   [`RATE`], unscaled, as PotPlayer runs them at a 48 kHz source's rate:
//!   goal L's host session matched PotPlayer within 0.1 dB this way and by
//!   up to 9 dB with the delays scaled from 44.1 kHz. (PotPlayer probably
//!   runs a 44.1 kHz source at its own rate, where its echoes come 9 %
//!   later than RVP's.)
//! * Crystality's echo ([`crystality_echo`]): PotPlayer's as goal L's host
//!   session measured it at its defaults, four delays of the stream with a
//!   gain from each side to each side and a low-pass for each pass through
//!   the delays, and weaker late repeats; four channels (left from left,
//!   left from right, right from left, right from right).
//! * The virtual speakers ([`head_model`]): Brown and Duda's spherical head
//!   ("A structural model for binaural sound synthesis", 1998): each ear
//!   hears a speaker through a one-pole, one-zero head shadow and after the
//!   delay round the head, with a few attenuated reflections for the
//!   virtualizer's distance. FFmpeg's `headphone` renders the eight
//!   speakers of 7.1 for both ears with them.
//!
//! The responses depend on their parameters only, so a file is named by
//! them and computed once.
//!
//! `afir` scales a response itself, to a sum of magnitudes of 1 in each
//! channel (FFmpeg 4.4 to 6.1's default `gtype=peak`, 7's `irnorm=1`), and
//! no option turns that off in every version (6.1 has no `irnorm`, newer
//! versions deprecate `gtype`). Freeverb's file therefore holds each
//! channel at that level, which every version leaves as it is, and its
//! [`Response`] carries the levels the graph puts back. `headphone` does
//! not scale its responses.

use std::path::{Path, PathBuf};

/// The rate of every response; the graphs resample to it first.
pub const RATE: u32 = 48_000;

/// Freeverb's tuning (`tuning.h`), in samples.
const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const FIXED_GAIN: f64 = 0.015;
const SCALE_DAMP: f64 = 0.4;
const SCALE_ROOM: f64 = 0.28;
const OFFSET_ROOM: f64 = 0.7;
const ALLPASS_FEEDBACK: f64 = 0.5;
/// Freeverb's wet and dry scaling (the response leaves them out).
pub const SCALE_WET: f64 = 3.0;
pub const SCALE_DRY: f64 = 2.0;
/// The longest reverb tail kept, in seconds (a room of 1 rings longer).
const MAX_TAIL: f64 = 6.0;

struct Comb {
    buffer: Vec<f64>,
    index: usize,
    store: f64,
}

impl Comb {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length],
            index: 0,
            store: 0.0,
        }
    }

    fn process(&mut self, input: f64, feedback: f64, damp: f64) -> f64 {
        let output = self.buffer[self.index];
        self.store = output * (1.0 - damp) + self.store * damp;
        self.buffer[self.index] = input + self.store * feedback;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

struct Allpass {
    buffer: Vec<f64>,
    index: usize,
}

impl Allpass {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length],
            index: 0,
        }
    }

    fn process(&mut self, input: f64) -> f64 {
        let delayed = self.buffer[self.index];
        let output = delayed - input;
        self.buffer[self.index] = input + delayed * ALLPASS_FEEDBACK;
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

/// Freeverb's response to an impulse, for its left and right outputs with
/// the width applied (`wet1`, `wet2`), and its fixed input gain; frames of
/// two channels at [`RATE`]. The wet level multiplies it, the dry signal is
/// added beside it.
#[must_use]
pub fn freeverb(room_size: f64, damping: f64, width: f64) -> Vec<[f32; 2]> {
    let feedback = room_size.clamp(0.0, 1.0) * SCALE_ROOM + OFFSET_ROOM;
    let damp = damping.clamp(0.0, 1.0) * SCALE_DAMP;
    let width = width.clamp(0.0, 1.0);
    let (wet1, wet2) = (width / 2.0 + 0.5, (1.0 - width) / 2.0);
    let mut sides: [(Vec<Comb>, Vec<Allpass>); 2] = [0, STEREO_SPREAD].map(|spread| {
        (
            COMBS.iter().map(|&c| Comb::new(c + spread)).collect(),
            ALLPASSES
                .iter()
                .map(|&a| Allpass::new(a + spread))
                .collect(),
        )
    });
    // The tail is cut where it stays below -100 dB for a second.
    // A few hundred thousand samples.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let longest = (MAX_TAIL * f64::from(RATE)) as usize;
    let quiet_run = RATE as usize;
    let mut frames = Vec::with_capacity(longest);
    let mut quiet = 0;
    for n in 0..longest {
        let input = if n == 0 { FIXED_GAIN } else { 0.0 };
        let mut out = [0.0f64; 2];
        for (side, (combs, allpasses)) in sides.iter_mut().enumerate() {
            let mut sum = 0.0;
            for comb in combs.iter_mut() {
                sum += comb.process(input, feedback, damp);
            }
            for allpass in allpasses.iter_mut() {
                sum = allpass.process(sum);
            }
            out[side] = sum;
        }
        let left = out[0] * wet1 + out[1] * wet2;
        let right = out[1] * wet1 + out[0] * wet2;
        // Single precision is the file's format.
        #[allow(clippy::cast_possible_truncation)]
        frames.push([left as f32, right as f32]);
        if left.abs().max(right.abs()) < 1e-5 {
            quiet += 1;
            if quiet >= quiet_run && n > quiet_run {
                frames.truncate(n + 1 - quiet + RATE as usize / 10);
                break;
            }
        } else {
            quiet = 0;
        }
    }
    frames
}

/// The speakers of 7.1 in FFmpeg's order with their azimuth in degrees
/// (positive to the left); the LFE has none.
pub const SPEAKERS: [(&str, Option<f64>); 8] = [
    ("FL", Some(30.0)),
    ("FR", Some(-30.0)),
    ("FC", Some(0.0)),
    ("LFE", None),
    ("BL", Some(150.0)),
    ("BR", Some(-150.0)),
    ("SL", Some(90.0)),
    ("SR", Some(-90.0)),
];

/// The head's radius in metres and the speed of sound.
const HEAD_RADIUS: f64 = 0.0875;
const SOUND_SPEED: f64 = 343.0;
/// Length of each response before reflections.
const DIRECT_LENGTH: usize = 192;

/// One ear's response to a speaker `incidence` degrees off the ear's axis:
/// the head shadow (Brown and Duda, eq. 3 and 4), discretized with the
/// bilinear transform, after the delay round the head (eq. 5), as a
/// fractional delay.
fn ear(incidence: f64, length: usize) -> Vec<f64> {
    let theta = incidence.abs().min(180.0);
    let omega0 = SOUND_SPEED / HEAD_RADIUS;
    let alpha_min: f64 = 0.1;
    let theta_min: f64 = 150.0;
    let alpha = (1.0 + alpha_min / 2.0)
        + (1.0 - alpha_min / 2.0) * (theta / theta_min * 180.0).to_radians().cos();
    // H(s) = (alpha s + 2 w0) / (s + 2 w0), s = K (1 - z^-1) / (1 + z^-1).
    let k = 2.0 * f64::from(RATE);
    let a0 = k + 2.0 * omega0;
    let (b0, b1) = (
        (alpha * k + 2.0 * omega0) / a0,
        (2.0 * omega0 - alpha * k) / a0,
    );
    let a1 = (2.0 * omega0 - k) / a0;
    // The delay round the head, made causal by a/c.
    let radians = theta.to_radians();
    let delay_seconds = if radians < std::f64::consts::FRAC_PI_2 {
        HEAD_RADIUS / SOUND_SPEED * (1.0 - radians.cos())
    } else {
        HEAD_RADIUS / SOUND_SPEED * (1.0 + radians - std::f64::consts::FRAC_PI_2)
    };
    let delay = delay_seconds * f64::from(RATE) + 8.0;
    // A windowed-sinc fractional delay of 16 taps around `delay`.
    let mut impulse = vec![0.0; length];
    // Delays are small positive sample counts.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let centre = delay.floor() as usize;
    let first = centre.saturating_sub(8);
    let taps = (centre + 8).min(length).saturating_sub(first);
    for (tap, slot) in impulse.iter_mut().enumerate().skip(first).take(taps) {
        // Tap indexes are far below 2^52.
        #[allow(clippy::cast_precision_loss)]
        let x = tap as f64 - delay;
        let sinc = if x.abs() < 1e-12 {
            1.0
        } else {
            (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
        };
        let window = 0.5 + 0.5 * (std::f64::consts::PI * x / 8.0).cos();
        *slot = sinc * window;
    }
    // The head shadow over the delayed impulse.
    let mut out = vec![0.0; length];
    let (mut x1, mut y1) = (0.0, 0.0);
    for (n, &x) in impulse.iter().enumerate() {
        let y = b0 * x + b1 * x1 - a1 * y1;
        out[n] = y;
        x1 = x;
        y1 = y;
    }
    out
}

/// The head model's responses for the eight speakers of [`SPEAKERS`]: per
/// frame, each speaker's left and right ear (16 channels, the order
/// FFmpeg's `headphone` with `hrir=multich` reads). `distance` (0 to 100)
/// adds a small room's reflections.
#[must_use]
pub fn head_model(distance: f64) -> Vec<[f32; 16]> {
    let room = distance.clamp(0.0, 100.0) / 100.0;
    // Reflections: delay in ms, gain at full distance.
    let reflections: [(f64, f64); 6] = [
        (7.0, 0.45),
        (11.0, 0.38),
        (17.0, 0.3),
        (23.0, 0.24),
        (31.0, 0.18),
        (43.0, 0.12),
    ];
    // Reflections reach 43 ms at most.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let length = if room > 0.0 {
        DIRECT_LENGTH + (0.05 * f64::from(RATE)) as usize
    } else {
        DIRECT_LENGTH
    };
    let mut frames = vec![[0.0f32; 16]; length];
    for (index, (_, azimuth)) in SPEAKERS.iter().enumerate() {
        // The left ear's axis is at +90 degrees, the right's at -90; the
        // LFE reaches both ears alike.
        let (left, right) = if let Some(azimuth) = azimuth {
            (ear(azimuth - 90.0, length), ear(azimuth + 90.0, length))
        } else {
            let both = ear(90.0, length);
            (both.clone(), both)
        };
        let mut channels = [left, right];
        if room > 0.0 {
            for (side, channel) in channels.iter_mut().enumerate() {
                let direct = channel[..DIRECT_LENGTH].to_vec();
                for (reflection, (ms, gain)) in reflections.iter().enumerate() {
                    // Reflections alternate sides, from the walls.
                    let gain = if (reflection + side) % 2 == 0 {
                        gain * room
                    } else {
                        gain * room * 0.8
                    };
                    // Offsets are a few thousand samples.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let offset = (ms / 1000.0 * f64::from(RATE)) as usize;
                    for (n, value) in direct.iter().enumerate() {
                        if let Some(slot) = channel.get_mut(offset + n) {
                            *slot += value * gain;
                        }
                    }
                }
            }
        }
        for (n, frame) in frames.iter_mut().enumerate() {
            // Single precision is the file's format.
            #[allow(clippy::cast_possible_truncation)]
            {
                frame[index * 2] = channels[0][n] as f32;
                frame[index * 2 + 1] = channels[1][n] as f32;
            }
        }
    }
    frames
}

/// Writes frames of `N` channels as a 32-bit float WAV (format 3), through
/// a temporary file so that a reader never sees half of it. Four channels
/// are named as FFmpeg's 4.0 layout (an extensible header), which the
/// graphs' `pan=4.0` gives `afir`; other counts leave the layout to the
/// reader.
///
/// # Errors
/// Fails on I/O errors.
pub fn write_wav<const N: usize>(path: &Path, frames: &[[f32; N]]) -> std::io::Result<()> {
    // FL FR FC BC.
    const QUAD_MASK: u32 = 0x107;
    let channels = u16::try_from(N).map_err(std::io::Error::other)?;
    let data_len = u32::try_from(frames.len() * N * 4).map_err(std::io::Error::other)?;
    let extensible = N == 4;
    let fmt_len: u32 = if extensible { 40 } else { 16 };
    let mut bytes = Vec::with_capacity(28 + fmt_len as usize + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(20 + fmt_len + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&fmt_len.to_le_bytes());
    bytes.extend_from_slice(&(if extensible { 0xFFFEu16 } else { 3 }).to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * u32::from(channels) * 4).to_le_bytes());
    bytes.extend_from_slice(&(channels * 4).to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    if extensible {
        bytes.extend_from_slice(&22u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(&QUAD_MASK.to_le_bytes());
        // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT.
        bytes.extend_from_slice(&[
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38,
            0x9B, 0x71,
        ]);
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for frame in frames {
        for sample in frame {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
    }
    // Flushed: a response file is written once and then used as it is.
    crate::storage::write_atomic(path, &bytes)
}

/// A response file for `afir` and the level of each of its channels, which
/// the file leaves out (see the module comment).
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub path: PathBuf,
    pub levels: Vec<f64>,
}

/// Scales each channel of `frames` to a sum of magnitudes of 1 and returns
/// the sums (0 for a silent channel, which stays as it is).
fn normalize<const N: usize>(frames: &mut [[f32; N]]) -> Vec<f64> {
    let mut levels = vec![0.0f64; N];
    for frame in frames.iter() {
        for (level, sample) in levels.iter_mut().zip(frame) {
            *level += f64::from(sample.abs());
        }
    }
    for frame in frames.iter_mut() {
        for (sample, level) in frame.iter_mut().zip(&levels) {
            if *level > 0.0 {
                // Single precision is the file's format.
                #[allow(clippy::cast_possible_truncation)]
                {
                    *sample = (f64::from(*sample) / level) as f32;
                }
            }
        }
    }
    levels
}

/// The file of Freeverb's response for these parameters under `folder`.
/// ("freeverb2": the files named "freeverb" had the delays scaled.)
#[must_use]
pub fn freeverb_path(folder: &Path, room_size: f64, damping: f64, width: f64) -> PathBuf {
    folder.join(format!(
        "freeverb2-r{room_size:.3}-d{damping:.3}-w{width:.3}.wav"
    ))
}

/// Freeverb's response for these parameters, its file under `folder`
/// written when missing. The response is computed either way, for its
/// levels.
///
/// # Errors
/// Fails on I/O errors.
pub fn freeverb_file(
    folder: &Path,
    room_size: f64,
    damping: f64,
    width: f64,
) -> std::io::Result<Response> {
    let mut frames = freeverb(room_size, damping, width);
    let levels = normalize(&mut frames);
    let path = freeverb_path(folder, room_size, damping, width);
    if !path.is_file() {
        write_wav(&path, &frames)?;
    }
    Ok(Response { path, levels })
}

/// One repeat of Crystality's echo: its delay in samples at [`RATE`], how
/// many times it went through the delays (and so through the low-pass),
/// and its gains to the left output from the left and right inputs and to
/// the right output from the left and right inputs.
struct EchoTap {
    delay: usize,
    passes: u8,
    gains: [f64; 4],
}

/// Crystality's echo at PotPlayer's default echo volume (11) and feedback
/// (30), measured in goal L's host session: a noise burst's cross-
/// correlation with what followed it found these four delays (17500 and
/// 28000 samples and their sums at 48 kHz) and their signs, the burst's
/// spectrum after each the levels (in dB: -24.4, -29.2, -31.0 and -26.3
/// for the first; -28.2, -23.6, none and -31.2 for the second; -25.3,
/// -31.0, -31.3 and -32.9; -38.8, -37.8, none and -52.0). The echo is
/// mostly the sides' difference, so a centre tone hears little of it, as
/// in PotPlayer.
const ECHO_TAPS: [EchoTap; 4] = [
    EchoTap {
        delay: 17_500,
        passes: 1,
        gains: [0.060_26, -0.034_67, -0.028_18, 0.048_42],
    },
    EchoTap {
        delay: 28_000,
        passes: 1,
        gains: [-0.038_90, 0.066_07, 0.0, 0.027_54],
    },
    EchoTap {
        delay: 35_000,
        passes: 2,
        gains: [0.054_33, -0.028_18, 0.027_23, -0.022_65],
    },
    EchoTap {
        delay: 45_500,
        passes: 2,
        gains: [-0.011_48, 0.012_88, 0.0, 0.002_51],
    },
];

/// PotPlayer's echo fades on after the last of those as a diffuse tail,
/// 40 to 60 dB down until 2 s: here as repeats of the sides' difference at
/// the delays' further sums, with their levels in dB.
const ECHO_TAIL: [(usize, f64); 7] = [
    (52_500, -38.0),
    (56_000, -41.0),
    (63_000, -43.0),
    (70_000, -45.0),
    (73_500, -47.0),
    (84_000, -52.0),
    (91_000, -56.0),
];

/// The echo's low-pass for each pass through the delays (a Butterworth
/// biquad, in Hz): PotPlayer's echoes lost 4 dB at 6 kHz and 7 dB at
/// 8 kHz on their first pass and about twice that on their second.
const ECHO_LOW_PASS: f64 = 6000.0;

/// How long each repeat's low-passed response is kept, in samples.
const ECHO_SPREAD: usize = 512;

/// The low-pass's response passed `passes` times, `length` samples long.
fn echo_low_pass(passes: u8, length: usize) -> Vec<f64> {
    let w0 = 2.0 * std::f64::consts::PI * ECHO_LOW_PASS / f64::from(RATE);
    let alpha = w0.sin() * std::f64::consts::FRAC_1_SQRT_2;
    let a0 = 1.0 + alpha;
    let b0 = (1.0 - w0.cos()) / 2.0 / a0;
    let b1 = (1.0 - w0.cos()) / a0;
    let a1 = -2.0 * w0.cos() / a0;
    let a2 = (1.0 - alpha) / a0;
    let mut response = vec![0.0; length];
    if let Some(first) = response.first_mut() {
        *first = 1.0;
    }
    for _ in 0..passes {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        for sample in &mut response {
            let x = *sample;
            let y = b0 * x + b1 * x1 + b0 * x2 - a1 * y1 - a2 * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            *sample = y;
        }
    }
    response
}

/// Crystality's echo for an echo volume and a feedback (0 to 100): the
/// first pass's repeats scale with the volume, each further pass's also
/// with the feedback (only the defaults were measured). Frames of four
/// channels at [`RATE`]: left from left, left from right, right from
/// left, right from right.
#[must_use]
pub fn crystality_echo(volume: f64, feedback: f64) -> Vec<[f32; 4]> {
    let volume = volume.clamp(0.0, 100.0) / 11.0;
    let feedback = feedback.clamp(0.0, 100.0) / 30.0;
    let last = ECHO_TAIL.last().map_or(0, |(delay, _)| *delay);
    let mut channels = vec![vec![0.0f64; last + ECHO_SPREAD]; 4];
    let tail = ECHO_TAIL.iter().map(|&(delay, level)| {
        let gain = db_to_gain(level);
        EchoTap {
            delay,
            passes: 3,
            gains: [gain, -gain, -gain, gain],
        }
    });
    for tap in ECHO_TAPS.into_iter().chain(tail) {
        let scale = volume * feedback.powi(i32::from(tap.passes) - 1);
        let spread = echo_low_pass(tap.passes, ECHO_SPREAD);
        for (channel, gain) in channels.iter_mut().zip(tap.gains) {
            for (slot, value) in channel[tap.delay..].iter_mut().zip(&spread) {
                *slot += gain * scale * value;
            }
        }
    }
    (0..last + ECHO_SPREAD)
        .map(|n| {
            // Single precision is the file's format.
            #[allow(clippy::cast_possible_truncation)]
            [0, 1, 2, 3].map(|c| channels[c][n] as f32)
        })
        .collect()
}

fn db_to_gain(level: f64) -> f64 {
    10f64.powf(level / 20.0)
}

/// The file of Crystality's echo for this echo volume and feedback under
/// `folder`.
#[must_use]
pub fn crystality_echo_path(folder: &Path, volume: f64, feedback: f64) -> PathBuf {
    folder.join(format!("crystality-echo-v{volume:.2}-f{feedback:.2}.wav"))
}

/// Crystality's echo for this echo volume and feedback, its file under
/// `folder` written when missing.
///
/// # Errors
/// Fails on I/O errors.
pub fn crystality_echo_file(
    folder: &Path,
    volume: f64,
    feedback: f64,
) -> std::io::Result<Response> {
    let mut frames = crystality_echo(volume, feedback);
    let levels = normalize(&mut frames);
    let path = crystality_echo_path(folder, volume, feedback);
    if !path.is_file() {
        write_wav(&path, &frames)?;
    }
    Ok(Response { path, levels })
}

/// The file of the head model's responses for this distance under
/// `folder`, written when missing.
///
/// # Errors
/// Fails on I/O errors.
pub fn head_model_file(folder: &Path, distance: f64) -> std::io::Result<PathBuf> {
    let path = folder.join(format!("head-model-d{:.0}.wav", distance.clamp(0.0, 100.0)));
    if !path.is_file() {
        write_wav(&path, &head_model(distance))?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(samples: impl Iterator<Item = f32>) -> f64 {
        samples.map(|s| f64::from(s) * f64::from(s)).sum()
    }

    #[test]
    fn freeverb_rings_longer_in_a_larger_room_and_is_deterministic() {
        let small = freeverb(0.2, 0.25, 1.0);
        let large = freeverb(0.9, 0.25, 1.0);
        assert!(
            large.len() > small.len(),
            "{} vs {}",
            large.len(),
            small.len()
        );
        assert_eq!(freeverb(0.5, 0.25, 1.0), freeverb(0.5, 0.25, 1.0));
        // Nothing comes out before the shortest comb's delay.
        let first = small
            .iter()
            .position(|frame| frame[0] != 0.0 || frame[1] != 0.0)
            .expect("an echo");
        assert!(first >= COMBS[0], "first echo at {first}");
        // Full width keeps the sides apart; no width makes them equal.
        let mono = freeverb(0.5, 0.25, 0.0);
        assert!(mono.iter().all(|frame| (frame[0] - frame[1]).abs() < 1e-9));
        let wide = freeverb(0.5, 0.25, 1.0);
        assert!(wide.iter().any(|frame| (frame[0] - frame[1]).abs() > 1e-6));
    }

    #[test]
    fn the_near_ear_hears_more_and_earlier() {
        let frames = head_model(0.0);
        // FL: left ear (channel 0) against right ear (channel 1).
        let left = energy(frames.iter().map(|f| f[0]));
        let right = energy(frames.iter().map(|f| f[1]));
        assert!(left > right * 1.2, "near {left} far {right}");
        let onset = |channel: usize| {
            frames
                .iter()
                .position(|f| f[channel].abs() > 0.1)
                .expect("an onset")
        };
        assert!(onset(0) < onset(1), "the left ear first");
        // FC reaches both ears alike.
        let centre_left = energy(frames.iter().map(|f| f[4]));
        let centre_right = energy(frames.iter().map(|f| f[5]));
        assert!((centre_left - centre_right).abs() < 1e-9);
        // A room adds length.
        assert!(head_model(50.0).len() > frames.len());
    }

    #[test]
    fn responses_are_written_once_as_float_wavs() {
        let dir = std::env::temp_dir().join(format!("rvp-dsp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = head_model_file(&dir, 0.0).expect("written");
        let bytes = std::fs::read(&path).expect("read");
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 3, "float");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 16, "channels");
        assert_eq!(
            bytes.len(),
            44 + head_model(0.0).len() * 16 * 4,
            "all frames"
        );
        let again = head_model_file(&dir, 0.0).expect("again");
        assert_eq!(again, path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
