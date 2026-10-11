#!/usr/bin/env python3
"""Blind ABX test of RVP's normalizer against PotPlayer's (goal F, S5).

`prepare` makes listening trials from the S5 clips and PotPlayer's loopback
recordings of them: each recording is aligned with its source (as in
analyze.py), RVP's version is the source through RVP's filter chain (the
same libavfilter `dynaudnorm` mpv runs), and both are matched to the same
integrated loudness, so that only the normalizer's behaviour differs, not
PotPlayer's volume. Each trial is an excerpt as `A.wav`, `B.wav` and
`X.wav`; A and B are RVP and PotPlayer in random order, and X is one of
them. Listen in any player (RVP itself works), then fill in `answers.txt`:
which one X is, and which of A and B sounds better. `score` reveals the key:
how many X were identified (and how likely that is by guessing), and which
version was preferred.

usage: ab_test.py prepare --ffmpeg FFMPEG --out DIR --pair SOURCE:RECORDING [--pair ...]
                          [--chain CHAIN] [--trials N] [--length S] [--skip S] [--seed N]
       ab_test.py score --out DIR

The trials and the key stay local (they are made from PotPlayer's output):
write them outside the repository, for example to
~/rvp-reference/s5/ab.
"""

import argparse
import json
import math
import random
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from analyze import duration, find_offset  # noqa: E402

# RVP's chain for PotPlayer's defaults (crates/app/src/normalizer.rs).
RVP_CHAIN = "dynaudnorm=f=5000:g=3:p=1.00:m=5:b=1"
RATE = 48000
# Loudness both versions are brought to, lowered when a peak would clip.
TARGET_LUFS = -18.0
PEAK_LIMIT_DB = -1.0
KEY = ".key.json"
ANSWERS = "answers.txt"


def run(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(args, check=True, capture_output=True, text=True)


def ffmpeg_base(ffmpeg: str) -> list[str]:
    return [ffmpeg, "-hide_banner", "-nostdin", "-loglevel", "error", "-y"]


def loudness(ffmpeg: str, path: Path) -> tuple[float, float]:
    """Integrated loudness (LUFS) and sample peak (dBFS) from ebur128."""
    out = subprocess.run(
        [ffmpeg, "-hide_banner", "-nostdin", "-i", str(path), "-af", "ebur128=peak=sample",
         "-f", "null", "-"],
        check=True, capture_output=True, text=True).stderr
    summary = out[out.rfind("Summary:"):]
    integrated = float(re.search(r"I:\s+(-?[\d.]+|-inf) LUFS", summary).group(1))
    peak = float(re.search(r"Peak:\s+(-?[\d.]+|-inf) dBFS", summary).group(1))
    return integrated, peak


def render(ffmpeg: str, source: Path, chain: str, out: Path) -> None:
    """The source through RVP's chain, as float WAV."""
    run(ffmpeg_base(ffmpeg) + ["-i", str(source), "-af", f"{chain},aresample={RATE}",
                               "-ac", "2", "-c:a", "pcm_f32le", str(out)])


def cut_recording(ffmpeg: str, recording: Path, offset: float, length: float, out: Path) -> None:
    run(ffmpeg_base(ffmpeg) + ["-ss", f"{offset:.4f}", "-t", f"{length:.4f}", "-i", str(recording),
                               "-af", f"aresample={RATE}", "-ac", "2", "-c:a", "pcm_f32le", str(out)])


def excerpt(ffmpeg: str, full: Path, start: float, length: float, gain_db: float, out: Path) -> None:
    fade = 0.05
    filters = (f"volume={gain_db:.2f}dB,afade=t=in:d={fade},"
               f"afade=t=out:st={length - fade:.3f}:d={fade}")
    run(ffmpeg_base(ffmpeg) + ["-ss", f"{start:.3f}", "-t", f"{length:.3f}", "-i", str(full),
                               "-af", filters, "-c:a", "pcm_s24le", str(out)])


def prepare(args: argparse.Namespace) -> int:
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    work = out / "full"
    work.mkdir(exist_ok=True)
    seed = args.seed if args.seed is not None else int(time.time())
    rng = random.Random(seed)

    clips = []
    for pair in args.pair:
        source, recording = (Path(p) for p in pair.split(":", 1))
        name = source.stem
        offset, correlation = find_offset(args.ffmpeg, source, recording)
        length = duration(args.ffmpeg, source)
        rvp = work / f"{name}-rvp.wav"
        pot = work / f"{name}-potplayer.wav"
        render(args.ffmpeg, source, args.chain, rvp)
        cut_recording(args.ffmpeg, recording, offset, length, pot)
        (rvp_i, rvp_peak), (pot_i, pot_peak) = loudness(args.ffmpeg, rvp), loudness(args.ffmpeg, pot)
        # One target for both; lower it until neither version clips.
        target = min(TARGET_LUFS, PEAK_LIMIT_DB - (rvp_peak - rvp_i), PEAK_LIMIT_DB - (pot_peak - pot_i))
        clips.append({"name": name, "length": length, "offset": offset, "correlation": correlation,
                      "rvp": str(rvp), "potplayer": str(pot),
                      "gain": {"rvp": target - rvp_i, "potplayer": target - pot_i}})
        print(f"{name}: offset {offset:.3f} s, correlation {correlation:.3f}, "
              f"RVP {rvp_i:.1f} LUFS, PotPlayer {pot_i:.1f} LUFS -> both {target:.1f} LUFS")

    trials = []
    for number in range(1, args.trials + 1):
        clip = clips[(number - 1) % len(clips)]
        latest = max(args.skip, clip["length"] - args.length)
        start = rng.uniform(args.skip, latest) if latest > args.skip else 0.0
        order = ["rvp", "potplayer"]
        rng.shuffle(order)
        x = rng.choice(["A", "B"])
        files = {"A": order[0], "B": order[1], "X": order[0] if x == "A" else order[1]}
        for label, version in files.items():
            excerpt(args.ffmpeg, Path(clip[version]), start, args.length, clip["gain"][version],
                    out / f"trial-{number:02d}-{label}.wav")
        trials.append({"trial": number, "clip": clip["name"], "start": round(start, 2),
                       "A": order[0], "B": order[1], "X": x})

    key = {"seed": seed, "chain": args.chain, "length": args.length, "clips": clips, "trials": trials}
    (out / KEY).write_text(json.dumps(key, indent=2) + "\n")
    lines = ["# RVP normalizer ABX test. For each trial listen to A, B and X.",
             "# Column x: is X the same as A or as B?  Column prefer: which of A and B",
             "# sounds better (A, B, or - for no preference). Replace each ? and save,",
             "# then run: ab_test.py score --out DIR",
             "# trial  x  prefer"]
    lines += [f"{t['trial']:02d}  ?  ?" for t in trials]
    (out / ANSWERS).write_text("\n".join(lines) + "\n")
    print(f"{len(trials)} trials in {out}; fill in {out / ANSWERS}. Do not open {KEY}.")
    return 0


def binomial_tail(k: int, n: int) -> float:
    """Chance of k or more correct out of n by guessing."""
    return sum(math.comb(n, i) for i in range(k, n + 1)) / 2 ** n


def score(args: argparse.Namespace) -> int:
    out = Path(args.out)
    key = json.loads((out / KEY).read_text())
    answers = {}
    for line in (out / ANSWERS).read_text().splitlines():
        parts = line.split()
        if not parts or parts[0].startswith("#") or len(parts) < 2:
            continue
        answers[int(parts[0])] = (parts[1].upper(), parts[2].upper() if len(parts) > 2 else "-")

    answered = correct = 0
    prefer = {"rvp": 0, "potplayer": 0, "none": 0}
    for trial in key["trials"]:
        x, preference = answers.get(trial["trial"], ("?", "-"))
        if x in ("A", "B"):
            answered += 1
            hit = x == trial["X"]
            correct += hit
        else:
            hit = None
        if preference in ("A", "B"):
            prefer[trial[preference]] += 1
        else:
            prefer["none"] += 1
        print(f"trial {trial['trial']:02d} {trial['clip']} at {trial['start']} s: "
              f"A={trial['A']} B={trial['B']} X={trial['X']}"
              f" -> answer {x} {'right' if hit else 'wrong' if hit is False else '(none)'}"
              f", preferred {trial[preference] if preference in ('A', 'B') else '-'}")
    if answered:
        chance = binomial_tail(correct, answered)
        verdict = ("audible difference (p < 0.05)" if chance < 0.05
                   else "no reliable difference")
        print(f"\nX identified {correct} of {answered}; by guessing {chance:.3f} -> {verdict}")
    print(f"preferred: RVP {prefer['rvp']}, PotPlayer {prefer['potplayer']}, "
          f"no preference {prefer['none']}")
    print(f"chain: {key['chain']}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--ffmpeg", default="ffmpeg")
    p.add_argument("--out", required=True)
    p.add_argument("--pair", action="append", required=True, help="SOURCE:RECORDING")
    p.add_argument("--chain", default=RVP_CHAIN)
    p.add_argument("--trials", type=int, default=12)
    p.add_argument("--length", type=float, default=20.0, help="seconds per excerpt")
    p.add_argument("--skip", type=float, default=15.0,
                   help="leave out the first seconds (PotPlayer's gain ramps up)")
    p.add_argument("--seed", type=int)
    s = sub.add_parser("score")
    s.add_argument("--out", required=True)
    args = parser.parse_args()
    return prepare(args) if args.command == "prepare" else score(args)


if __name__ == "__main__":
    sys.exit(main())
