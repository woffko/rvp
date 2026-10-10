#!/usr/bin/env python3
"""S5: fits a simple AGC model of PotPlayer's normalizer to the loopback
recordings, to see whether such a model explains them better than
dynaudnorm (analyze.py).

Model, on loudness every 100 ms: the wanted gain is target - M (momentary
loudness of the source, 400 ms), limited to [g_min, g_min + range]. If the
wanted gain is lower than the current gain, the gain follows at once
(attack); otherwise it rises by at most `release` dB per second. The gain
starts at g_min. The output's short-term loudness is the energy mean of
M + gain over the last 3 s, compared with the recording's S as in
analyze.py (one offset for all clips, equal weight per clip).

usage: agc_model.py --ffmpeg FFMPEG --out DIR --pair SOURCE:RECORDING [--pair ...] [--skip-start S]
"""

import argparse
import itertools
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

SILENCE = -100.0


def series(ffmpeg: str, path: Path, key: str) -> np.ndarray:
    filters = f"ebur128=metadata=1,ametadata=print:key=lavfi.r128.{key}:file=-"
    out = subprocess.run(
        [ffmpeg, "-hide_banner", "-nostdin", "-loglevel", "error", "-i", str(path), "-af", filters, "-f", "null", "-"],
        check=True, capture_output=True, text=True,
    ).stdout
    prefix = f"lavfi.r128.{key}="
    values = np.array([float(line.split("=", 1)[1]) for line in out.splitlines() if line.startswith(prefix)])
    # Digital silence gives -inf or nan.
    return np.where(np.isfinite(values), values, -120.0)


def simulate(momentary: np.ndarray, target: float, g_min: float, span: float, release: float) -> np.ndarray:
    step = release * 0.1
    gain = g_min
    gains = np.empty(len(momentary))
    for i, level in enumerate(momentary):
        wanted = min(max(target - level, g_min), g_min + span) if level > -70 else gain
        gain = wanted if wanted < gain else min(wanted, gain + step)
        gains[i] = gain
    energy = 10 ** ((momentary + gains) / 10)
    kernel = np.ones(30) / 30
    short = np.convolve(energy, kernel)[: len(energy)]
    return 10 * np.log10(np.maximum(short, 1e-30))


def joint(outputs: list[np.ndarray], targets: list[np.ndarray], skip: float) -> tuple[float, float]:
    diffs = []
    for out, target in zip(outputs, targets):
        n = min(len(out), len(target))
        keep = (target[:n] > SILENCE) & (np.arange(n) >= int(skip * 10))
        diffs.append(out[:n][keep] - target[:n][keep])
    offset = float(np.median(np.concatenate(diffs)))
    return float(np.sqrt(np.mean([np.mean((d - offset) ** 2) for d in diffs]))), offset


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--pair", action="append", required=True)
    parser.add_argument("--skip-start", type=float, default=0.0)
    parser.add_argument("--offsets", help="JSON from analyze.py report: clip offsets in seconds", default=None)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    offsets = {}
    if args.offsets:
        for clip in json.loads(Path(args.offsets).read_text())["clips"]:
            offsets[clip["recording"]] = clip["offset_s"]
    sources, targets = [], []
    for pair in args.pair:
        source, recording = (Path(p) for p in pair.split(":", 1))
        sources.append(series(args.ffmpeg, source, "M"))
        rec = series(args.ffmpeg, recording, "S")
        shift = int(round(offsets.get(recording.name, 0.0) * 10))
        targets.append(rec[shift:])
    results = []
    for target, g_min, span, release in itertools.product(
        [-30.0, -25.0, -20.0, -15.0], [-20.0, -15.0, -10.0, -5.0, 0.0], [8.0, 12.0, 16.0, 20.0, 24.0],
        [0.3, 0.5, 0.65, 0.8, 1.0, 1.5, 2.0],
    ):
        outputs = [simulate(m, target, g_min, span, release) for m in sources]
        rms, offset = joint(outputs, targets, args.skip_start)
        results.append((rms, offset, target, g_min, span, release))
    results.sort()
    best = results[0]
    report = {
        "skip_start_s": args.skip_start,
        "best": {"joint_rms_lu": round(best[0], 2), "offset_lu": round(best[1], 2), "target_lufs": best[2],
                 "g_min_db": best[3], "range_db": best[4], "release_db_per_s": best[5]},
        "top5": [{"joint_rms_lu": round(r[0], 2), "target": r[2], "g_min": r[3], "range": r[4], "release": r[5]}
                 for r in results[:5]],
    }
    (args.out / "agc-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report["best"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
