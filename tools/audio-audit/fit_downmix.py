#!/usr/bin/env python3
"""Audio audit: PotPlayer's 5.1 downmix from a loopback recording of
`tone-51.wav` (or its AC3 copy) played with PotPlayer's normalizer on.

The model mixes the source down with a candidate matrix (front channels 1,
centre, LFE and surround channels to fit), runs TomSteady (the S5 model,
tools/s5/tomsteady_fit.py: level 75, stepping 10, boost) over blocks of
the mix, and compares the levels per 100 ms with the recording, inside the
bursts only (0.25 s from their edges). It fits the centre, then the LFE,
then the surround level, and reports the fit with the normalizer after the
mix, before it, and with mpv's own matrix.

usage: fit_downmix.py --ffmpeg FFMPEG --source tone-51.wav
       --recording audio.wav --after SECONDS [--block 3072]

`--after` is a time in the recording before the first burst (after the
previous segment's sound). Needs numpy.
"""

import argparse
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "s5"))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from tomsteady_fit import block_gains  # noqa: E402
from audit import db, read_wav  # noqa: E402

SOURCE_RATE = 48000
# The bursts' starts in the source, seconds (FL FR FC LFE SL SR).
BURSTS = (1.0, 4.5, 8.0, 11.5, 15.0, 18.5)
BURST = 2.5


def gain(decibels):
    return 10 ** (decibels / 20)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ffmpeg", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--recording", required=True)
    parser.add_argument("--after", type=float, required=True)
    parser.add_argument("--block", type=int, default=2048)
    args = parser.parse_args()

    raw = subprocess.run(
        [args.ffmpeg, "-v", "error", "-i", args.source, "-f", "f32le", "-"],
        capture_output=True,
        check=True,
    ).stdout
    source = np.frombuffer(raw, dtype="<f4").reshape(-1, 6).astype(np.float64)
    recording, rate = read_wav(Path(args.recording))

    # The first burst's start: the first quarter window above -40 dB.
    step = rate // 40
    start = int(args.after * rate)
    onset = None
    for index in range(start, start + 8 * rate, step):
        if db(np.sqrt(np.mean(recording[index:index + rate // 10] ** 2))) > -40:
            onset = index / rate
            break
    if onset is None:
        raise SystemExit("no burst after --after")
    print(f"first burst at {onset:.3f} s in the recording")

    def simulate(centre, lfe, surround, before=False):
        fl, fr, fc, low, sl, sr = source.T
        mix = np.stack(
            [fl + centre * fc + lfe * low + surround * sl, fr + centre * fc + lfe * low + surround * sr],
            axis=1,
        )
        blocks = len(mix) // args.block
        basis = source if before else mix
        peaks = np.abs(basis[: blocks * args.block]).reshape(blocks, args.block, -1).max(axis=(1, 2)) * 32768.0
        gains = np.repeat(block_gains(peaks, 75, 10, True), args.block)
        return mix[: blocks * args.block] * gains[:, None]

    def windows(samples, samples_rate, t0, t1):
        size = samples_rate // 10
        return np.array(
            [
                [db(v) for v in np.sqrt(np.mean(samples[i:i + size] ** 2, axis=0))]
                for i in range(int(t0 * samples_rate), int(t1 * samples_rate) - size, size)
            ]
        )

    def error(centre, lfe, surround, end, before=False):
        model = windows(simulate(centre, lfe, surround, before), SOURCE_RATE, 1.2, end)
        recorded = windows(recording, rate, onset - 1.0 + 1.2, onset - 1.0 + end)
        n = min(len(model), len(recorded))
        model, recorded = model[:n], recorded[:n]
        times = 1.2 + np.arange(n) * 0.1
        inside = np.zeros(n, dtype=bool)
        for begin in BURSTS:
            inside |= (times >= begin + 0.25) & (times + 0.1 <= begin + BURST - 0.25)
        keep = np.isfinite(model) & np.isfinite(recorded) & (model > -40) & (recorded > -40)
        keep &= inside[:, None]
        return float(np.sqrt(np.mean((model[keep] - recorded[keep]) ** 2)))

    grid = np.arange(-6, 1.01, 0.1)
    centre = min(grid, key=lambda d: error(gain(d), gain(-3), 1.0, 10.4))
    lfe = min(grid, key=lambda d: error(gain(centre), gain(d), 1.0, 13.9))
    surround = min(grid, key=lambda d: error(gain(centre), gain(lfe), gain(d), 21.0))
    fit = (gain(centre), gain(lfe), gain(surround))
    print(f"block {args.block}: centre {centre:.1f} dB, LFE {lfe:.1f} dB, surround {surround:.1f} dB")
    print(f"  normalizer after the mix: rms {error(*fit, 21.0):.3f} dB")
    print(f"  normalizer before the mix: rms {error(*fit, 21.0, before=True):.3f} dB")
    print(f"  mpv's matrix (centre and surround -3 dB, no LFE): rms {error(gain(-3.0103), 0.0, gain(-3.0103), 21.0):.3f} dB")


if __name__ == "__main__":
    main()
