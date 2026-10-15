#!/usr/bin/env python3
"""S5 follow-up: compares PotPlayer's recorded normalizer gain with
TomSteady's gain control (the rules in crates/app/src/tomsteady.rs) over a
grid of block sizes and steppings.

PotPlayer's gain is the recording's RMS minus the source's in 100 ms windows
(the recording aligned as in analyze.py); the model's is TomSteady's gain
over the source, block by block. One offset for all clips removes
PotPlayer's volume; the score is the RMS difference in dB, averaged over
the clips.

usage: tomsteady_fit.py --ffmpeg FFMPEG --pair SOURCE:RECORDING [--pair ...]
       [--blocks 1024,2048,...] [--steppings 8,9,10,...] [--levels 75]

Needs numpy. The recordings stay in ~/rvp-reference (never in the
repository).
"""

import argparse
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze import find_offset  # noqa: E402

RATE = 48000
WINDOW = RATE // 10
FULL_SCALE = 32768.0
CLIP = 30000.0
QUIET = 10.0


def block_gains(peaks, level, stepping, boost=True):
    """TomSteady's gain for each block, given the blocks' peaks (16-bit
    scale): the gain a block plays with, after its clipping cut."""
    vol = 1 << stepping
    counters = [7] * 4096
    history = 0
    raises = 0
    gains = np.empty(len(peaks))
    for n, peak in enumerate(peaks):
        vol = max(vol, 1)
        drops = 0
        passed = False
        while True:
            out = vol / float(1 << stepping) * peak
            if out <= CLIP:
                break
            vol = vol - stepping if vol > stepping else 1
            drops += 1
            if drops >= 1024:
                passed = True
                break
        gains[n] = 1.0 if passed else vol / float(1 << stepping)
        if passed:
            continue
        if out > QUIET:
            if out <= (level << 15) * 0.01:
                counters[history] = min(counters[history] + 1, 15)
                agrees = counters[history] > 7
                history = (history | 4096) >> 1
                if agrees:
                    raises += 1
                    vol += stepping if raises >= 128 else 1
            else:
                counters[history] = max(counters[history] - 1, 0)
                agrees = counters[history] <= 7
                history >>= 1
                raises = 0
                if agrees and vol >= 2:
                    vol -= 1
        if not boost:
            vol = min(vol, 1 << stepping)
    return gains


def decode(ffmpeg, path, start=0.0, length=None):
    args = [ffmpeg, "-hide_banner", "-nostdin", "-loglevel", "error", "-ss", f"{start:.5f}", "-i", str(path)]
    if length is not None:
        args += ["-t", f"{length:.5f}"]
    args += ["-ac", "2", "-ar", str(RATE), "-f", "f32le", "-"]
    raw = subprocess.run(args, check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).reshape(-1, 2).astype(np.float64)


def window_db(x):
    n = len(x) // WINDOW
    w = x[: n * WINDOW].reshape(n, WINDOW, -1)
    return 20 * np.log10(np.maximum(np.sqrt(np.mean(w**2, axis=(1, 2))), 1e-12))


def model_db(source, block, level, stepping):
    n = len(source) // block
    peaks = np.abs(source[: n * block]).reshape(n, block, -1).max(axis=(1, 2)) * FULL_SCALE
    tail = np.abs(source[n * block :]).max() * FULL_SCALE if len(source) > n * block else None
    if tail is not None:
        peaks = np.append(peaks, tail)
    per_sample = np.repeat(block_gains(peaks, level, stepping), block)[: len(source)]
    windows = len(source) // WINDOW
    return 20 * np.log10(per_sample[: windows * WINDOW].reshape(windows, WINDOW).mean(axis=1))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ffmpeg", required=True)
    parser.add_argument("--pair", action="append", required=True)
    parser.add_argument("--blocks", default="576,1024,1152,1536,2048,4096")
    parser.add_argument("--steppings", default="8,9,10,11,12")
    parser.add_argument("--levels", default="75")
    args = parser.parse_args()
    clips = []
    for pair in args.pair:
        source_path, recording_path = (Path(p) for p in pair.split(":", 1))
        offset, corr = find_offset(args.ffmpeg, source_path, recording_path)
        source = decode(args.ffmpeg, source_path)
        recording = decode(args.ffmpeg, recording_path, offset, len(source) / RATE)
        n = min(len(source), len(recording))
        source, recording = source[:n], recording[:n]
        source_db = window_db(source)
        clips.append((source_path.stem, source, window_db(recording) - source_db, source_db > -75))
        print(f"{source_path.stem}: offset {offset:.3f} s, correlation {corr:.3f}", flush=True)
    rows = []
    for level in (int(v) for v in args.levels.split(",")):
        for block in (int(v) for v in args.blocks.split(",")):
            for stepping in (int(v) for v in args.steppings.split(",")):
                diffs = []
                for _, source, potplayer, keep in clips:
                    model = model_db(source, block, level, stepping)
                    n = min(len(model), len(potplayer))
                    diffs.append((potplayer[:n] - model[:n])[keep[:n]])
                offset = float(np.median(np.concatenate(diffs)))
                per = [float(np.sqrt(np.mean((d - offset) ** 2))) for d in diffs]
                p95 = [float(np.percentile(np.abs(d - offset), 95)) for d in diffs]
                rows.append((float(np.mean(per)), level, block, stepping, per, p95))
                print(
                    f"level {level} block {block} stepping {stepping}: rms {np.mean(per):.3f} dB "
                    f"per clip {' '.join(f'{v:.3f}' for v in per)} p95 {' '.join(f'{v:.2f}' for v in p95)}",
                    flush=True,
                )
    rows.sort()
    best = rows[0]
    print(f"best: level {best[1]} block {best[2]} stepping {best[3]} rms {best[0]:.3f} dB")


if __name__ == "__main__":
    main()
