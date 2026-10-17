#!/usr/bin/env python3
"""Goal L's test files for the host comparison of PotPlayer and RVP
(docs/audits/audio.md, "M10"). Each file is a run of passes; every pass
starts with half a second of silence and a marker (2 kHz, left only, 0.25 s,
so that voice removal keeps it), so a recording can be cut into passes by
the markers; `host-session m10` switches PotPlayer's setting for the next
pass during that silence.

  m10-bands.wav    19 passes, one per equalizer preset: the ten bands'
                   frequencies one after the other (0.3 s, 0.1 s apart) at
                   0.1 of full scale, room for the presets' 12 dB.
  m10-effects.wav  7 passes (none, then each effect): a 1 kHz tone in the
                   centre, the same on the left only, 500 Hz in the centre
                   with 3 kHz left and 5 kHz right, a noise burst and two
                   seconds of silence (a reverb's tail), a tone over a
                   -46 dBFS noise floor (noise reduction).
  m10-51.wav       5.1 (side), 6 passes (stereo, then five speaker modes):
                   each channel alone for 0.8 s, the LFE at 50 Hz.
  m10-clicks.wav   40 s of 20 ms 1 kHz pips every half second: two
                   devices' recordings line up on them.
  block-*          the audit's tone (1 kHz at 0.95 of full scale) for 15 s
                   in the codecs whose normalizer block PotPlayer chose is
                   not known yet (made with ffmpeg).

usage: make_m10.py FFMPEG OUT_DIR

Needs numpy. The files stay out of the repository.
"""

import subprocess
import sys
from pathlib import Path

import numpy as np

RATE = 48000
BANDS = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000]
PRESET_PASSES = 19
EFFECT_PASSES = 7
SPEAKER_PASSES = 6
FADE = 0.005


def tone(frequency, seconds, amplitude):
    n = int(round(seconds * RATE))
    t = np.arange(n) / RATE
    wave = amplitude * np.sin(2 * np.pi * frequency * t)
    fade = int(FADE * RATE)
    ramp = 0.5 - 0.5 * np.cos(np.pi * np.arange(fade) / fade)
    wave[:fade] *= ramp
    wave[-fade:] *= ramp[::-1]
    return wave


def silence(seconds, channels):
    return np.zeros((int(round(seconds * RATE)), channels))


def frame(columns, channels):
    """Columns (channel index -> mono signal of equal length) as frames."""
    length = max(len(signal) for signal in columns.values())
    out = np.zeros((length, channels))
    for channel, signal in columns.items():
        out[: len(signal), channel] += signal
    return out


def marker(channels):
    return np.concatenate([
        silence(0.5, channels),
        frame({0: tone(2000, 0.25, 0.5)}, channels),
        silence(0.25, channels),
    ])


def bands_pass():
    parts = [marker(2)]
    for frequency in BANDS:
        wave = tone(frequency, 0.3, 0.1)
        parts.append(frame({0: wave, 1: wave}, 2))
        parts.append(silence(0.1, 2))
    parts.append(silence(0.25, 2))
    return np.concatenate(parts)


def effects_pass(rng):
    centre = tone(1000, 0.8, 0.25)
    parts = [
        marker(2),
        frame({0: centre, 1: centre}, 2),
        silence(0.3, 2),
        frame({0: tone(1000, 0.8, 0.25)}, 2),
        silence(0.3, 2),
    ]
    low = tone(500, 0.8, 0.1)
    parts.append(frame({0: low + tone(3000, 0.8, 0.1), 1: low + tone(5000, 0.8, 0.1)}, 2))
    parts.append(silence(0.3, 2))
    burst = rng.uniform(-0.2, 0.2, (int(0.2 * RATE), 2))
    parts.append(burst)
    parts.append(silence(2.0, 2))
    floor = rng.uniform(-1, 1, (RATE, 2)) * 10 ** (-46 / 20) * np.sqrt(3)
    signal = tone(1000, 1.0, 0.25)
    parts.append(floor + frame({0: signal, 1: signal}, 2))
    parts.append(silence(0.5, 2))
    return np.concatenate(parts)


def surround_pass():
    parts = [marker(6)]
    for channel in range(6):
        frequency = 50 if channel == 3 else 1000
        parts.append(frame({channel: tone(frequency, 0.8, 0.5)}, 6))
        parts.append(silence(0.2, 6))
    parts.append(silence(0.5, 6))
    return np.concatenate(parts)


def clicks():
    out = silence(40.0, 2)
    pip = tone(1000, 0.02, 0.5)
    for start in range(RATE // 2, len(out) - RATE, RATE // 2):
        out[start:start + len(pip), 0] += pip
        out[start:start + len(pip), 1] += pip
    return out


def write_wav(path, frames, mask=None):
    """24-bit PCM; more than two channels as WAVE_FORMAT_EXTENSIBLE."""
    channels = frames.shape[1]
    data = np.clip(np.round(frames * 8388607), -8388608, 8388607).astype("<i4")
    raw = data.reshape(-1, 1).view(np.uint8).reshape(-1, 4)[:, :3].tobytes()
    block = channels * 3
    if channels > 2:
        fmt = (
            (0xFFFE).to_bytes(2, "little") + channels.to_bytes(2, "little")
            + RATE.to_bytes(4, "little") + (RATE * block).to_bytes(4, "little")
            + block.to_bytes(2, "little") + (24).to_bytes(2, "little")
            + (22).to_bytes(2, "little") + (24).to_bytes(2, "little")
            + mask.to_bytes(4, "little")
            + bytes([1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113])
        )
    else:
        fmt = (
            (1).to_bytes(2, "little") + channels.to_bytes(2, "little")
            + RATE.to_bytes(4, "little") + (RATE * block).to_bytes(4, "little")
            + block.to_bytes(2, "little") + (24).to_bytes(2, "little")
        )
    body = b"fmt " + len(fmt).to_bytes(4, "little") + fmt + b"data" + len(raw).to_bytes(4, "little") + raw
    path.write_bytes(b"RIFF" + (4 + len(body)).to_bytes(4, "little") + b"WAVE" + body)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    ffmpeg, out = sys.argv[1], Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(20261017)
    write_wav(out / "m10-bands.wav", np.concatenate([bands_pass() for _ in range(PRESET_PASSES)]))
    write_wav(out / "m10-effects.wav", np.concatenate([effects_pass(rng) for _ in range(EFFECT_PASSES)]))
    # FL FR FC LFE SL SR.
    write_wav(out / "m10-51.wav", np.concatenate([surround_pass() for _ in range(SPEAKER_PASSES)]), 0x60F)
    write_wav(out / "m10-clicks.wav", clicks())
    tone_source = ["-f", "lavfi", "-i", "aevalsrc=0.95*sin(2*PI*1000*t)|0.95*sin(2*PI*1000*t):s=48000:d=15"]
    encodings = {
        "block-mp3.mp3": ["-c:a", "libmp3lame", "-b:a", "320k"],
        "block-aac.m4a": ["-c:a", "aac", "-b:a", "256k"],
        "block-opus.opus": ["-c:a", "libopus", "-b:a", "192k"],
        "block-pcm16.wav": ["-c:a", "pcm_s16le"],
        "block-ac3-stereo.mkv": ["-c:a", "ac3", "-b:a", "448k"],
        "block-eac3.mkv": ["-c:a", "eac3", "-b:a", "448k"],
        "block-dts.mkv": ["-strict", "-2", "-c:a", "dca", "-b:a", "768k"],
        "block-flac44.flac": ["-ar", "44100", "-c:a", "flac"],
    }
    for name, codec in encodings.items():
        subprocess.run(
            [ffmpeg, "-nostdin", "-loglevel", "error", "-y", *tone_source, *codec, str(out / name)],
            check=True,
        )
    for path in sorted(out.iterdir()):
        print(f"{path.name}\t{path.stat().st_size}")


if __name__ == "__main__":
    main()
