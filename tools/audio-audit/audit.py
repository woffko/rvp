#!/usr/bin/env python3
"""Audio audit: the steady parts of a loopback recording, in order, with
their levels (docs/audits/audio.md).

`host-session audio` records RVP and PotPlayer playing the audit's tones.
Loopback gives no samples while nothing plays, so the times in the
session's log do not map onto the recording; the parts are found by their
levels instead. A plateau is a run of 100 ms windows whose level per
channel stays within 0.25 dB of the window before; each is printed with
its start in the recording, its length, the median level of each channel
(dB RMS, the first and last windows left out) and its crest factor (peak
over RMS, 3.01 dB for a clean sine, less when it clips).

The session plays, in order: RVP's tone with the normalizer on (settling),
without it at volume 100, 50 and 22, RVP's 5.1 files, then PotPlayer's tone
while its volume steps down from 100 % (and 150 %, 200 %), then PotPlayer's
5.1 files. `fit_downmix.py` fits PotPlayer's 5.1 mix.

usage: audit.py RECORDING.wav [--min-seconds 0.8]

Needs numpy. Recordings and results stay out of the repository.
"""

import argparse
import math
from pathlib import Path

import numpy as np

WINDOW_S = 0.1
STEADY_DB = 0.25


def read_wav(path):
    """The samples of a 32-bit float WAV (hound's format) and its rate."""
    data = path.read_bytes()
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise SystemExit(f"{path}: not a WAV file")
    pos = 12
    channels = rate = bits = None
    while pos + 8 <= len(data):
        chunk, size = data[pos:pos + 4], int.from_bytes(data[pos + 4:pos + 8], "little")
        body = data[pos + 8:pos + 8 + size]
        if chunk == b"fmt ":
            channels = int.from_bytes(body[2:4], "little")
            rate = int.from_bytes(body[4:8], "little")
            bits = int.from_bytes(body[14:16], "little")
        elif chunk == b"data":
            if bits != 32:
                raise SystemExit(f"{path}: {bits}-bit samples, expected 32-bit float")
            samples = np.frombuffer(body[: len(body) // (4 * channels) * 4 * channels], dtype="<f4")
            return samples.reshape(-1, channels).astype(np.float64), rate
        pos += 8 + size + (size & 1)
    raise SystemExit(f"{path}: no data chunk")


def db(value):
    return 20 * math.log10(value) if value > 0 else float("-inf")


def window_rms(samples, rate):
    """RMS of each 100 ms window, per channel: windows x channels."""
    size = int(rate * WINDOW_S)
    count = len(samples) // size
    blocks = samples[: count * size].reshape(count, size, samples.shape[1])
    return np.sqrt(np.mean(blocks ** 2, axis=1))


def plateaus(samples, rate, min_seconds):
    """(start, end, median dB per channel, crest dB) of the steady runs."""
    rms = window_rms(samples, rate)
    levels = [[db(v) for v in row] for row in rms]
    size = int(rate * WINDOW_S)
    runs = []
    start = 0

    def steady(a, b):
        for x, y in zip(a, b):
            if math.isfinite(x) != math.isfinite(y):
                return False
            if math.isfinite(x) and abs(x - y) > STEADY_DB:
                return False
        return True

    for index in range(1, len(levels) + 1):
        if index < len(levels) and steady(levels[index], levels[index - 1]):
            continue
        if (index - start) * WINDOW_S >= min_seconds:
            inner = levels[start + 1:index - 1] if index - start > 3 else levels[start:index]
            median = np.median(np.array(inner), axis=0)
            part = samples[(start + 1) * size:(index - 1) * size]
            peak = np.max(np.abs(part)) if len(part) else 0.0
            loudest = max((v for v in median if math.isfinite(v)), default=float("-inf"))
            crest = db(peak) - loudest if math.isfinite(loudest) else float("nan")
            runs.append((start * WINDOW_S, index * WINDOW_S, median, crest))
        start = index
    return runs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("recording")
    parser.add_argument("--min-seconds", type=float, default=0.8)
    args = parser.parse_args()
    samples, rate = read_wav(Path(args.recording))
    print(f"{len(samples) / rate:.1f} s at {rate} Hz, {samples.shape[1]} channels")
    for start, end, median, crest in plateaus(samples, rate, args.min_seconds):
        levels = "  ".join(f"{v:8.2f}" for v in median)
        print(f"{start:7.1f} - {end:7.1f} s ({end - start:4.1f} s)  dB {levels}   crest {crest:5.2f} dB")


if __name__ == "__main__":
    main()
