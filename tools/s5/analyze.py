#!/usr/bin/env python3
"""S5: finds a dynaudnorm-based filter chain whose EBU R128 short-term
loudness follows PotPlayer's normalizer.

For each clip the loopback recording of PotPlayer is aligned with the source
by cross-correlation and cut to the source's length. Short-term loudness (S,
3 s window, every 100 ms) comes from ffmpeg's ebur128 filter. A candidate
chain is applied to the source and scored by the RMS difference between its
S series and the recording's, after removing one constant offset per clip
(PotPlayer's volume and the output level differ from the source). The
"without normalizer" reference is the source itself (owner decision
2026-10-10).

usage: analyze.py --ffmpeg FFMPEG --out DIR --pair SOURCE:RECORDING [--pair ...]
       [--jobs N] [--quick]
"""

import argparse
import concurrent.futures
import itertools
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

RATE = 48000
SILENCE = -60.0  # S values below this are left out of the score


def decode_mono(ffmpeg: str, path: Path, rate: int, start: float = 0.0, length: float | None = None) -> np.ndarray:
    args = [ffmpeg, "-hide_banner", "-nostdin", "-loglevel", "error", "-ss", f"{start:.4f}", "-i", str(path)]
    if length is not None:
        args += ["-t", f"{length:.4f}"]
    args += ["-ac", "1", "-ar", str(rate), "-f", "f32le", "-"]
    raw = subprocess.run(args, check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32)


def duration(ffmpeg: str, path: Path) -> float:
    return len(decode_mono(ffmpeg, path, 1000)) / 1000.0


def onset(signal: np.ndarray, threshold: float = 1e-5) -> int:
    above = np.flatnonzero(np.abs(signal) > threshold)
    return int(above[0]) if len(above) else 0


def find_offset(ffmpeg: str, source: Path, recording: Path) -> tuple[float, float]:
    """Seconds into the recording where the source starts, and the
    normalized correlation there (1.0 = identical shape).

    The recording starts with digital silence, so the first sound gives a
    coarse position; a correlation of the whole clip within +-0.5 s of it
    refines it. A global correlation alone locks onto the wrong period of
    sustained tones."""
    rate = 8000
    src = decode_mono(ffmpeg, source, rate)
    rec = decode_mono(ffmpeg, recording, rate)
    coarse = onset(rec) - onset(src)
    best, best_value = coarse, -np.inf
    span = rate // 2
    for lag in range(max(0, coarse - span), coarse + span + 1, 1):
        window = rec[lag : lag + len(src)]
        if len(window) < len(src) // 2:
            break
        value = float(np.dot(src[: len(window)], window))
        if value > best_value:
            best, best_value = lag, value
    window = rec[best : best + len(src)]
    score = best_value / (np.linalg.norm(src[: len(window)]) * np.linalg.norm(window) + 1e-12)
    return best / rate, float(score)


def short_term(ffmpeg: str, path: Path, chain: str = "", start: float = 0.0, length: float | None = None) -> np.ndarray:
    """S values every 100 ms (LUFS), -inf replaced by -120."""
    filters = (chain + "," if chain else "") + "ebur128=metadata=1,ametadata=print:key=lavfi.r128.S:file=-"
    args = [ffmpeg, "-hide_banner", "-nostdin", "-loglevel", "error", "-ss", f"{start:.4f}", "-i", str(path)]
    if length is not None:
        args += ["-t", f"{length:.4f}"]
    args += ["-af", filters, "-f", "null", "-"]
    out = subprocess.run(args, check=True, capture_output=True, text=True).stdout
    values = []
    for line in out.splitlines():
        if line.startswith("lavfi.r128.S="):
            text = line.split("=", 1)[1]
            values.append(-120.0 if "inf" in text else float(text))
    return np.array(values)


def score(candidate: np.ndarray, target: np.ndarray) -> tuple[float, float]:
    """RMS difference in LU after removing the median offset, and the offset."""
    n = min(len(candidate), len(target))
    cand, targ = candidate[:n], target[:n]
    keep = (targ > SILENCE) & (cand > SILENCE)
    if keep.sum() < 10:
        return 99.0, 0.0
    diff = cand[keep] - targ[keep]
    offset = float(np.median(diff))
    return float(np.sqrt(np.mean((diff - offset) ** 2))), offset


def dynaudnorm(f: int, g: int, p: float, m: float, r: float, s: float) -> str:
    chain = f"dynaudnorm=f={f}:g={g}:p={p}:m={m}"
    if r > 0:
        chain += f":r={r}"
    if s > 0:
        chain += f":s={s}"
    return chain


def evaluate(job: tuple[str, str, list[tuple[str, list[float]]]]) -> tuple[str, list[tuple[float, float]]]:
    ffmpeg, chain, clips = job
    results = []
    for source, target in clips:
        series = short_term(ffmpeg, Path(source), chain)
        results.append(score(series, np.array(target)))
    return chain, results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--pair", action="append", required=True, help="SOURCE:RECORDING")
    parser.add_argument("--jobs", type=int, default=32)
    parser.add_argument("--quick", action="store_true", help="small grid, for testing")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    clips = []
    report = {"clips": []}
    for pair in args.pair:
        source, recording = (Path(part) for part in pair.split(":", 1))
        length = duration(args.ffmpeg, source)
        offset, correlation = find_offset(args.ffmpeg, source, recording)
        target = short_term(args.ffmpeg, recording, start=offset, length=length)
        plain = short_term(args.ffmpeg, source)
        baseline, base_offset = score(plain, target)
        clips.append((str(source), target.tolist()))
        entry = {
            "source": source.name,
            "recording": recording.name,
            "offset_s": round(offset, 4),
            "correlation": round(correlation, 3),
            "without_normalizer_rms_lu": round(baseline, 2),
            "without_normalizer_offset_lu": round(base_offset, 2),
        }
        report["clips"].append(entry)
        print(json.dumps(entry))
        np.savetxt(args.out / f"{source.stem}-recording-S.txt", target, fmt="%.2f")
        np.savetxt(args.out / f"{source.stem}-source-S.txt", plain, fmt="%.2f")

    if args.quick:
        grid = itertools.product([200, 500], [15, 31], [0.95], [10.0, 30.0], [0.0], [0.0])
    else:
        grid = itertools.product(
            [100, 200, 300, 500, 800, 1000],
            [3, 7, 11, 15, 21, 31, 51],
            [0.5, 0.75, 0.95],
            [3.0, 5.0, 10.0, 20.0, 50.0, 100.0],
            [0.0, 0.1, 0.25],
            [0.0],
        )
    jobs = [(args.ffmpeg, dynaudnorm(*params), clips) for params in grid]
    results = []
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        for chain, per_clip in pool.map(evaluate, jobs, chunksize=4):
            mean = float(np.mean([rms for rms, _ in per_clip]))
            results.append((mean, chain, per_clip))
    results.sort()
    with open(args.out / "candidates.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("mean_rms_lu\tchain\t" + "\t".join(f"{Path(s).stem}_rms\t{Path(s).stem}_offset" for s, _ in clips) + "\n")
        for mean, chain, per_clip in results:
            cells = "\t".join(f"{rms:.2f}\t{offset:.2f}" for rms, offset in per_clip)
            tsv.write(f"{mean:.3f}\t{chain}\t{cells}\n")
    best = results[0]
    report["candidates"] = len(results)
    report["best"] = {
        "chain": best[1],
        "mean_rms_lu": round(best[0], 2),
        "per_clip": [{"rms_lu": round(r, 2), "offset_lu": round(o, 2)} for r, o in best[2]],
    }
    report["top5"] = [{"chain": c, "mean_rms_lu": round(m, 2)} for m, c, _ in results[:5]]
    (args.out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report["best"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
