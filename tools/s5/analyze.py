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
       [--jobs N] [--grid quick|coarse|fine|long|final] [--family F] [--skip-start S]
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
# Only digital silence is left out of the score. The recording is quieter
# than the source by PotPlayer's volume, so a level threshold on it would
# drop quiet passages.
SILENCE = -100.0



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
    """S values every 100 ms (LUFS); silence (-inf or nan) becomes -120."""
    filters = (chain + "," if chain else "") + "ebur128=metadata=1,ametadata=print:key=lavfi.r128.S:file=-"
    args = [ffmpeg, "-hide_banner", "-nostdin", "-loglevel", "error", "-ss", f"{start:.4f}", "-i", str(path)]
    if length is not None:
        args += ["-t", f"{length:.4f}"]
    args += ["-af", filters, "-f", "null", "-"]
    out = subprocess.run(args, check=True, capture_output=True, text=True).stdout
    values = np.array([float(line.split("=", 1)[1]) for line in out.splitlines() if line.startswith("lavfi.r128.S=")])
    # Digital silence gives -inf or nan.
    return np.where(np.isfinite(values), values, -120.0)


def differences(candidate: np.ndarray, target: np.ndarray, skip: float = 0.0) -> np.ndarray:
    """candidate - target in LU where both are above SILENCE, leaving out the
    first `skip` seconds (PotPlayer's normalizer starts with a low gain and
    ramps up)."""
    n = min(len(candidate), len(target))
    cand, targ = candidate[:n], target[:n]
    keep = (targ > SILENCE) & (cand > SILENCE) & (np.arange(n) >= int(skip * 10))
    return cand[keep] - targ[keep]


def score(candidate: np.ndarray, target: np.ndarray, skip: float = 0.0) -> tuple[float, float]:
    """RMS difference in LU after removing the median offset, and the offset."""
    diff = differences(candidate, target, skip)
    if len(diff) < 10:
        return 99.0, 0.0
    offset = float(np.median(diff))
    return float(np.sqrt(np.mean((diff - offset) ** 2))), offset


def joint_score(diffs: list[np.ndarray]) -> tuple[float, float]:
    """One offset for all clips (PotPlayer's volume is the same for all of
    them); RMS per clip around it, averaged with equal weight per clip."""
    if any(len(d) < 10 for d in diffs):
        return 99.0, 0.0
    offset = float(np.median(np.concatenate(diffs)))
    rms = float(np.sqrt(np.mean([np.mean((d - offset) ** 2) for d in diffs])))
    return rms, offset


def dynaudnorm(f: int, g: int, p: float, m: float, r: float, s: float) -> str:
    chain = f"dynaudnorm=f={f}:g={g}:p={p}:m={m}"
    if r > 0:
        chain += f":r={r}"
    if s > 0:
        chain += f":s={s}"
    return chain


def evaluate(
    job: tuple[str, str, list[tuple[str, list[float]]], float],
) -> tuple[str, list[tuple[float, float]], tuple[float, float]]:
    ffmpeg, chain, clips, skip = job
    results = []
    diffs = []
    for source, target in clips:
        series = short_term(ffmpeg, Path(source), chain)
        results.append(score(series, np.array(target), skip))
        diffs.append(differences(series, np.array(target), skip))
    return chain, results, joint_score(diffs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--pair", action="append", required=True, help="SOURCE:RECORDING")
    parser.add_argument("--jobs", type=int, default=32)
    parser.add_argument("--skip-start", type=float, default=0.0, help="seconds ignored at the start of each clip")
    parser.add_argument("--grid", choices=["quick", "coarse", "fine", "long", "final"], default="coarse")
    parser.add_argument(
        "--family",
        choices=["dynaudnorm", "speechnorm", "acompressor"],
        default="dynaudnorm",
        help="filter to search; speechnorm and acompressor (as an AGC) have separate attack and release",
    )
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    clips = []
    plain_diffs = []
    report = {"clips": []}
    for pair in args.pair:
        source, recording = (Path(part) for part in pair.split(":", 1))
        length = duration(args.ffmpeg, source)
        offset, correlation = find_offset(args.ffmpeg, source, recording)
        target = short_term(args.ffmpeg, recording, start=offset, length=length)
        plain = short_term(args.ffmpeg, source)
        baseline, base_offset = score(plain, target, args.skip_start)
        plain_diffs.append(differences(plain, target, args.skip_start))
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

    baseline_joint, baseline_offset = joint_score(plain_diffs)
    report["without_normalizer"] = {"joint_rms_lu": round(baseline_joint, 2), "joint_offset_lu": round(baseline_offset, 2)}
    print(json.dumps(report["without_normalizer"]))

    if args.grid == "quick":
        grid = itertools.product([200, 500], [15, 31], [0.95], [10.0, 30.0], [0.0], [0.0])
    elif args.grid == "final":
        # The long grid's optimum: smallest Gaussian window, frames of
        # 4-8 s, peak 1.0; here the maximum gain is varied further.
        grid = itertools.product([4000, 5000, 6000, 7000, 8000], [3], [0.95, 1.0], [4.0, 5.0, 6.0, 7.0, 8.0, 10.0], [0.0], [0.0])
    elif args.grid == "long":
        # Longer frames than the fine grid, whose optimum sat at its edge.
        grid = itertools.product(
            [1500, 2000, 3000, 4000, 5000, 6000, 8000], [3, 5, 7, 9, 11], [0.9, 0.95, 1.0], [3.0, 4.0, 5.0], [0.0], [0.0]
        )
    elif args.grid == "fine":
        # Around the coarse optimum of the joint score: a smoothing window
        # (frame length x Gaussian size) of about 2-4 s, maximum gain about 5.
        grid = itertools.product(
            [250, 400, 600, 800, 1000, 1500],
            [3, 5, 7, 9],
            [0.9, 0.95, 1.0],
            [4.0, 5.0, 6.0, 7.0],
            [0.0, 0.1],
            [0.0, 3.0],
        )
    else:
        grid = itertools.product(
            [100, 200, 300, 500, 800, 1000],
            [3, 7, 11, 15, 21, 31, 51],
            [0.5, 0.75, 0.95],
            [3.0, 5.0, 10.0, 20.0, 50.0, 100.0],
            [0.0, 0.1, 0.25],
            [0.0],
        )
    if args.family == "speechnorm":
        chains = [
            f"speechnorm=p={p}:e={e}:c={c}:r={r}:f={f}:m={m}"
            for p, e, c, r, f, m in itertools.product(
                [0.5, 0.95], [2, 5, 10, 25], [2, 5, 10, 25], [0.0001, 0.0003, 0.001, 0.003],
                [0.001, 0.003, 0.01, 0.03], [0.0, 0.1])
        ]
    elif args.family == "acompressor":
        # A compressor with a very low threshold, a high ratio and make-up
        # gain behaves like an AGC: fast attack, slow release, bounded boost.
        chains = [
            f"acompressor=threshold={t}:ratio={q}:attack={a}:release={r}:makeup={m}:detection=rms"
            for t, q, a, r, m in itertools.product(
                [0.001, 0.003, 0.01, 0.03], [4, 10, 20], [5, 20, 80], [1000, 3000, 6000, 9000], [2, 4, 8, 16])
        ]
    else:
        chains = [dynaudnorm(*params) for params in grid]
    jobs = [(args.ffmpeg, chain, clips, args.skip_start) for chain in chains]
    results = []
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        for chain, per_clip, (joint, joint_offset) in pool.map(evaluate, jobs, chunksize=4):
            mean = float(np.mean([rms for rms, _ in per_clip]))
            results.append((joint, chain, per_clip, mean, joint_offset))
    # Ranked by the RMS with one shared offset; the per-clip-offset mean is
    # reported alongside.
    results.sort()
    with open(args.out / "candidates.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("joint_rms_lu\tjoint_offset_lu\tmean_rms_lu\tchain\t"
                  + "\t".join(f"{Path(s).stem}_rms\t{Path(s).stem}_offset" for s, _ in clips) + "\n")
        for joint, chain, per_clip, mean, joint_offset in results:
            cells = "\t".join(f"{rms:.2f}\t{offset:.2f}" for rms, offset in per_clip)
            tsv.write(f"{joint:.3f}\t{joint_offset:.2f}\t{mean:.3f}\t{chain}\t{cells}\n")
    best = results[0]
    report["candidates"] = len(results)
    report["best"] = {
        "chain": best[1],
        "joint_rms_lu": round(best[0], 2),
        "joint_offset_lu": round(best[4], 2),
        "mean_rms_lu": round(best[3], 2),
        "per_clip": [{"rms_lu": round(r, 2), "offset_lu": round(o, 2)} for r, o in best[2]],
    }
    report["top5"] = [{"chain": c, "joint_rms_lu": round(j, 2), "mean_rms_lu": round(m, 2)}
                      for j, c, _, m, _ in results[:5]]
    (args.out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report["best"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
