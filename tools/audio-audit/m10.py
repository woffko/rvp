#!/usr/bin/env python3
"""Goal L's comparison of PotPlayer and RVP (docs/audits/audio.md, "M10").

PotPlayer's side is the recording `host-session m10` made of its output
while it played `make_m10.py`'s files: the 19 equalizer presets, the six
effects after a pass without, five speaker modes after a pass of 2.0
Stereo, and the normalizer on the `block-*` files. Passes are found by
their markers (2 kHz on the left, 0.25 s), whatever the recording's rate;
the block files' parts are the long loud ones after the last pass.

RVP's side is rendered here, one pass at a time, through RVP's own chain
with the same setting (`cargo run --release -p rvp-app --example
render_chain`, passed as --render), and measured the same way.

Levels are the tones' own (a single-frequency DFT over a Hann window: no
scalloping, harmonics and noise left out), in dB RMS of the full scale;
the presets are compared relative to the pass without one (the players'
own equalizer gain drops out); effects and speaker modes as levels, both
players at gain 1 (volume 100) with their normalizers off.

usage: m10.py --files DIR --render BIN --work DIR
              [--recording WAV=presets,effects --recording WAV=speakers,blocks]

Prints Markdown tables. Needs numpy. Recordings, renders and results stay
out of the repository.
"""

import argparse
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "s5"))
from tomsteady_fit import block_gains  # noqa: E402

BANDS = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000]
# PotPlayer's presets in its menu's order and RVP's values for them
# (crates/app/src/equalizer.rs: PotPlayer's measured response).
PRESETS = [
    ("(Default)", [0.0] * 10),
    ("Classical", [0.0, 0.0, 0.0, -0.1, -0.3, -2.5, -7.3, -8.3, -8.1, -7.6]),
    ("Club", [-0.1, 0.4, 1.3, 2.3, 2.3, 1.8, 1.2, 0.3, 0.2, 0.1]),
    ("Dance", [3.3, 3.9, 2.7, 0.7, -0.1, -2.2, -3.0, -1.8, -1.5, -1.2]),
    ("Full Bass", [3.4, 5.7, 6.0, 4.2, 2.1, -3.4, -7.8, -9.4, -9.2, -8.6]),
    ("Full Bass & Treble", [1.9, 5.7, 0.1, -1.9, -1.2, 5.4, 9.7, 10.0, 9.6, 9.1]),
    ("Full Treble", [-3.1, -4.7, -4.7, -1.4, 2.0, 13.0, 18.5, 17.5, 16.6, 15.6]),
    ("Laptop", [1.3, -4.7, 2.9, 0.0, 0.0, 4.3, 7.7, 8.7, 8.6, 8.3]),
    ("Large Hall", [3.4, 4.9, 4.4, 2.6, 1.0, -2.2, -3.3, -2.4, -2.1, -1.7]),
    ("Live", [-1.8, 0.0, 1.1, 2.0, 2.1, 2.0, 1.6, 0.9, 0.8, 0.7]),
    ("Loudness", [6.1, 7.8, 6.9, 4.3, 3.1, 7.4, 11.8, 13.0, 12.8, 12.4]),
    ("Party", [2.0, 7.8, 1.2, 0.4, 0.4, 1.6, 2.8, 3.6, 3.6, 3.6]),
    ("Pop", [-0.6, 1.7, 2.7, 2.8, 1.9, -1.5, -3.0, -2.8, -2.6, -2.3]),
    ("Reggae", [0.0, 1.7, -0.9, -1.8, -0.5, 2.6, 2.6, 0.9, 0.5, 0.3]),
    ("Rock", [2.7, 1.6, -1.5, -3.1, -1.7, 5.1, 8.4, 8.2, 7.8, 7.3]),
    ("Ska", [-0.7, -1.6, -1.1, 0.1, 1.1, 4.2, 6.4, 6.2, 5.9, 5.4]),
    ("Soft", [0.9, 0.3, -0.4, -0.9, 0.0, 5.1, 8.5, 8.8, 8.5, 8.1]),
    ("Soft Rock", [1.0, 1.4, 0.9, -0.4, -1.0, -1.0, 0.3, 1.7, 1.9, 2.0]),
    ("Techno", [2.4, 2.1, 0.4, -2.0, -1.7, 2.1, 4.7, 5.2, 5.0, 4.7]),
]
EFFECTS = [
    ("none", {}),
    ("crystality", {"audio": {"crystality": {"enabled": True}}}),
    ("noise-reduction", {"audio": {"noise-reduction": {"enabled": True}}}),
    ("freeverb", {"audio": {"freeverb": {"enabled": True}}}),
    ("voice-removal", {"audio": {"voice-removal": True}}),
    ("voice-emphasis", {"audio": {"voice-emphasis": True}}),
    ("crossfeed", {"audio": {"crossfeed": {"enabled": True}}}),
    # PotPlayer's Audio Stretching at 110 % (RVP's is a session value,
    # which render_chain does not take; tests/audio_chain.rs checks it).
    ("stretch-110", None),
]
SPEAKERS = [
    ("stereo", "stereo"),
    ("virtual-headphone", "virtual-headphone"),
    ("virtual-surround", "virtual-surround"),
    ("dolby-surround", "dolby"),
    ("dolby-pro-logic-ii", "dplii"),
    ("virtual-dolby-decoder", "dolby-decoder"),
]
SPEAKER_CHANNELS = ["FL", "FR", "FC", "LFE", "SL", "SR"]
BLOCK_FILES = [
    ("block-mp3.mp3", "mp3"),
    ("block-aac.m4a", "aac"),
    ("block-opus.opus", "opus"),
    ("block-pcm16.wav", "pcm_s16le"),
    ("block-ac3-stereo.mkv", "ac3"),
    ("block-eac3.mkv", "eac3"),
    ("block-dts.mkv", "dts"),
    ("block-flac44.flac", "flac"),
]
CANDIDATE_BLOCKS = [960, 1024, 1152, 1536, 2048, 2880, 3072, 4096]
# The passes, as make_m10.py writes them: the file's silence before the
# first pass, then each pass's lead, marker and gap, in seconds.
FILE_LEAD = 3.0
LEAD, MARKER, GAP = 0.5, 0.25, 0.25
PASS_SECONDS = {"bands": 5.25, "effects": 8.0, "speakers": 7.5}


def read_wav(path):
    """Samples as float64 in -1..1 and the rate: PCM 16/24/32 or float 32."""
    data = Path(path).read_bytes()
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise SystemExit(f"{path}: not a WAV file")
    pos, channels, rate, bits, kind = 12, None, None, None, None
    while pos + 8 <= len(data):
        chunk, size = data[pos:pos + 4], int.from_bytes(data[pos + 4:pos + 8], "little")
        body = data[pos + 8:pos + 8 + size]
        if chunk == b"fmt ":
            kind = int.from_bytes(body[0:2], "little")
            channels = int.from_bytes(body[2:4], "little")
            rate = int.from_bytes(body[4:8], "little")
            bits = int.from_bytes(body[14:16], "little")
            if kind == 0xFFFE:
                kind = int.from_bytes(body[24:26], "little")
        elif chunk == b"data":
            if size in (0, 0xFFFFFFFF):
                body = data[pos + 8:]
            frame = channels * bits // 8
            body = body[: len(body) // frame * frame]
            if kind == 3 and bits == 32:
                samples = np.frombuffer(body, dtype="<f4").astype(np.float64)
            elif bits == 16:
                samples = np.frombuffer(body, dtype="<i2") / 32768.0
            elif bits == 24:
                raw = np.frombuffer(body, dtype=np.uint8).reshape(-1, 3)
                values = raw[:, 0].astype(np.int32) | (raw[:, 1].astype(np.int32) << 8) | (raw[:, 2].astype(np.int32) << 16)
                values = np.where(values >= 1 << 23, values - (1 << 24), values)
                samples = values / 8388608.0
            elif bits == 32:
                samples = np.frombuffer(body, dtype="<i4") / 2147483648.0
            else:
                raise SystemExit(f"{path}: {bits}-bit samples")
            return samples.reshape(-1, channels), rate
        pos += 8 + size + (size & 1)
    raise SystemExit(f"{path}: no data chunk")


def write_wav(path, frames, rate, mask=None):
    """24-bit PCM (as make_m10.py)."""
    channels = frames.shape[1]
    data = np.clip(np.round(frames * 8388607), -8388608, 8388607).astype("<i4")
    raw = data.reshape(-1, 1).view(np.uint8).reshape(-1, 4)[:, :3].tobytes()
    block = channels * 3
    if channels > 2:
        fmt = (
            (0xFFFE).to_bytes(2, "little") + channels.to_bytes(2, "little") + rate.to_bytes(4, "little")
            + (rate * block).to_bytes(4, "little") + block.to_bytes(2, "little") + (24).to_bytes(2, "little")
            + (22).to_bytes(2, "little") + (24).to_bytes(2, "little") + mask.to_bytes(4, "little")
            + bytes([1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113])
        )
    else:
        fmt = (
            (1).to_bytes(2, "little") + channels.to_bytes(2, "little") + rate.to_bytes(4, "little")
            + (rate * block).to_bytes(4, "little") + block.to_bytes(2, "little") + (24).to_bytes(2, "little")
        )
    body = b"fmt " + len(fmt).to_bytes(4, "little") + fmt + b"data" + len(raw).to_bytes(4, "little") + raw
    Path(path).write_bytes(b"RIFF" + (4 + len(body)).to_bytes(4, "little") + b"WAVE" + body)


def db(value):
    return 20 * math.log10(value) if value > 0 else float("-inf")


def tone_db(x, rate, frequency, t0, t1):
    """The tone's level in dB RMS of full scale, between t0 and t1 s."""
    part = x[int(t0 * rate):int(t1 * rate)]
    if len(part) < 16:
        return float("-inf")
    window = np.hanning(len(part))
    phase = np.exp(-2j * np.pi * frequency * np.arange(len(part)) / rate)
    amplitude = 2 * abs(np.sum(part * window * phase)) / np.sum(window)
    return db(amplitude / math.sqrt(2))


def peak_hz(x, rate, t0, t1, low, high):
    """The strongest frequency between `low` and `high` Hz in the segment."""
    part = x[int(t0 * rate):int(t1 * rate)]
    if len(part) < 16:
        return float("nan")
    size = 1 << 20
    spectrum = np.abs(np.fft.rfft(part * np.hanning(len(part)), size))
    freqs = np.fft.rfftfreq(size, 1 / rate)
    band = (freqs >= low) & (freqs <= high)
    return float(freqs[band][np.argmax(spectrum[band])])


def rms_db(x, rate, t0, t1):
    part = x[int(t0 * rate):int(t1 * rate)]
    return db(math.sqrt(float(np.mean(part ** 2)))) if len(part) else float("-inf")


def residual_db(x, rate, frequencies, t0, t1):
    """What is left of the segment after its tones (least squares), dB RMS."""
    part = x[int(t0 * rate):int(t1 * rate)]
    t = np.arange(len(part)) / rate
    columns = [np.ones_like(t)]
    for frequency in frequencies:
        columns += [np.sin(2 * np.pi * frequency * t), np.cos(2 * np.pi * frequency * t)]
    basis = np.stack(columns, axis=1)
    coefficients, *_ = np.linalg.lstsq(basis, part, rcond=None)
    rest = part - basis @ coefficients
    return db(math.sqrt(float(np.mean(rest ** 2))))


def markers(x, rate):
    """The onsets (s) of the passes' 2 kHz markers on the left channel."""
    left = x[:, 0]
    size = rate // 100
    frames = len(left) // size
    found, run = [], 0
    t = np.arange(size) / rate
    probe = np.exp(-2j * np.pi * 2000 * t)
    for index in range(frames):
        frame = left[index * size:(index + 1) * size]
        energy = float(np.mean(frame ** 2))
        tone = abs(np.sum(frame * probe)) * 2 / size
        # Only markers carry 2 kHz alone; PotPlayer's Crossfeed lowers one to
        # -34 dB.
        is_marker = energy > 10 ** (-50 / 10) and (tone ** 2 / 2) > 0.6 * energy
        if is_marker:
            run += 1
        else:
            # Only markers carry 2 kHz; a reverb's tail can lengthen one.
            if run >= 15:
                start = (index - run) * size
                part = np.abs(left[max(start - size, 0):start + size])
                peak = part.max()
                onset = max(start - size, 0) + int(np.argmax(part > 0.3 * peak))
                found.append(onset / rate)
            run = 0
    return found


def measure_pass(kind, x, rate, onset):
    """A pass's measurements, `onset` its marker's start in seconds."""
    start = onset + MARKER + GAP
    left, right = x[:, 0], x[:, 1]
    if kind == "bands":
        # PotPlayer switches 0.7 to 1 s after the marker: the 170 Hz tone's
        # window starts after that.
        return [
            tone_db(left, rate, f, start + 0.4 * i + (0.10 if i == 1 else 0.05), start + 0.4 * i + 0.25)
            for i, f in enumerate(BANDS)
        ]
    if kind == "effects":
        a, b, c = start, start + 1.1, start + 2.2
        burst = start + 3.3
        e = start + 5.5
        # The centre tone's window starts after PotPlayer's switch, which
        # comes 0.7 to 1 s after the marker.
        return {
            "centre_L": tone_db(left, rate, 1000, a + 0.5, a + 0.75),
            "centre_R": tone_db(right, rate, 1000, a + 0.5, a + 0.75),
            "left_L": tone_db(left, rate, 1000, b + 0.1, b + 0.7),
            "left_R": tone_db(right, rate, 1000, b + 0.1, b + 0.7),
            "mix_L500": tone_db(left, rate, 500, c + 0.1, c + 0.7),
            "mix_L3k": tone_db(left, rate, 3000, c + 0.1, c + 0.7),
            "mix_R5k": tone_db(right, rate, 5000, c + 0.1, c + 0.7),
            "mix_L5k": tone_db(left, rate, 5000, c + 0.1, c + 0.7),
            "mix_R3k": tone_db(right, rate, 3000, c + 0.1, c + 0.7),
            "mix_rest_L": residual_db(left, rate, [500, 3000, 5000], c + 0.1, c + 0.7),
            "burst": rms_db(left, rate, burst + 0.03, burst + 0.17),
            "tail_0.1": rms_db(left, rate, burst + 0.25, burst + 0.5),
            "tail_0.5": rms_db(left, rate, burst + 0.6, burst + 1.1),
            "tail_1.2": rms_db(left, rate, burst + 1.2, burst + 1.9),
            "floor_tone": tone_db(left, rate, 1000, e + 0.1, e + 0.9),
            "floor_rest": residual_db(left, rate, [1000], e + 0.1, e + 0.9),
            "centre_hz": peak_hz(left, rate, a + 0.5, a + 0.75, 800, 1300),
        }
    if kind == "speakers":
        rows = []
        for i, name in enumerate(SPEAKER_CHANNELS):
            f = 50 if name == "LFE" else 1000
            t0 = start + i + 0.1
            rows.append((tone_db(left, rate, f, t0, t0 + 0.6), tone_db(right, rate, f, t0, t0 + 0.6)))
        return rows
    raise ValueError(kind)


def first_sound(x, rate, after):
    """The first 10 ms above -60 dB after `after` seconds."""
    size = rate // 100
    start = int(after * rate)
    for index in range(start, len(x) - size, size):
        if rms_db(x[:, 0], rate, index / rate, (index + size) / rate) > -60 or rms_db(
            x[:, 1], rate, index / rate, (index + size) / rate
        ) > -60:
            return index / rate
    return None


def block_segments(x, rate, after):
    """The loud parts after `after` s that last 5 s or more: (start, end)."""
    size = rate // 20
    level = [rms_db(x[:, 0], rate, i / rate, (i + size) / rate) for i in range(int(after * rate), len(x) - size, size)]
    segments, begin = [], None
    for index, value in enumerate(level):
        t = after + index * size / rate
        if value > -20 and begin is None:
            begin = t
        elif value <= -20 and begin is not None:
            if t - begin >= 5:
                segments.append((begin, t))
            begin = None
    if begin is not None and after + len(level) * size / rate - begin >= 5:
        segments.append((begin, after + len(level) * size / rate))
    return segments


def fit_block(x, rate, start, end, source_rate=48000):
    """TomSteady's block (in the source's samples) that fits a block file's
    level best, in steps of 16: (block, rms, offset)."""
    step = 0.05
    times = np.arange(0.3, min(end - start, 9.5) - step, step)
    recorded = np.array([rms_db(x[:, 0], rate, start + t, start + t + step) for t in times])
    best = None
    for block in range(512, 8193, 16):
        blocks = int((times[-1] + 1) * source_rate / block) + 2
        gains = block_gains(np.full(blocks, 0.95 * 32768.0), 75, 10, True)
        model = np.array([
            db(0.95 / math.sqrt(2) * gains[min(int(t * source_rate / block), blocks - 1)]) for t in times
        ])
        offset = float(np.median(recorded - model))
        error = float(np.sqrt(np.mean((recorded - model - offset) ** 2)))
        if best is None or error < best[1]:
            best = (block, error, offset)
    return best


def render(binary, source, settings, out):
    out.parent.mkdir(parents=True, exist_ok=True)
    config = out.with_suffix(".json")
    config.write_text(json.dumps(settings))
    subprocess.run([binary, str(source), str(out), str(config)], check=True, capture_output=True)
    return read_wav(out)


def one_pass(files, name, seconds, channels, mask, work):
    """The first pass of a file, for RVP's renders."""
    x, rate = read_wav(files / name)
    path = work / f"pass-{name}"
    write_wav(path, x[int(FILE_LEAD * rate): int((FILE_LEAD + seconds) * rate)], rate, mask)
    return path


def rvp_side(files, binary, work):
    results = {"presets": [], "effects": [], "speakers": []}
    bands = one_pass(files, "m10-bands.wav", PASS_SECONDS["bands"], 2, None, work)
    for name, values in PRESETS:
        x, rate = render(binary, bands, {"equalizer": {"enabled": True, "bands": values, "preset": name}},
                         work / "rvp" / f"preset-{len(results['presets'])}.wav")
        results["presets"].append(measure_pass("bands", x, rate, markers(x, rate)[0]))
    effects = one_pass(files, "m10-effects.wav", PASS_SECONDS["effects"], 2, None, work)
    for name, settings in EFFECTS:
        if settings is None:
            results["effects"].append(None)
            continue
        x, rate = render(binary, effects, settings, work / "rvp" / f"effect-{name}.wav")
        results["effects"].append(measure_pass("effects", x, rate, markers(x, rate)[0]))
    surround = one_pass(files, "m10-51.wav", PASS_SECONDS["speakers"], 6, 0x60F, work)
    for name, choice in SPEAKERS:
        x, rate = render(binary, surround, {"output-speakers": choice}, work / "rvp" / f"speakers-{name}.wav")
        results["speakers"].append(measure_pass("speakers", x, rate, markers(x, rate)[0]))
    return results


SECTION_PASSES = {"presets": len(PRESETS), "effects": len(EFFECTS), "speakers": len(SPEAKERS)}
SECTION_KIND = {"presets": "bands", "effects": "effects", "speakers": "speakers"}


def potplayer_side(recordings):
    """PotPlayer's measurements from `(wav, [sections])` pairs, the sections
    in the order the recording holds them; `blocks` comes last."""
    results = {"presets": [], "effects": [], "speakers": [], "blocks": []}
    for recording, sections in recordings:
        x, rate = read_wav(recording)
        onsets = markers(x, rate)
        expected = sum(SECTION_PASSES.get(section, 0) for section in sections)
        print(f"<!-- recording {Path(recording).name}: {rate} Hz, {len(x) / rate:.1f} s, "
              f"{len(onsets)} markers (expected {expected}) for {', '.join(sections)} -->")
        if len(onsets) == expected - 1 and onsets:
            # A mode that restarts PotPlayer's output (Virtual Dolby
            # Decoder) loses the last pass's marker: that pass starts half a
            # second before its first sound.
            onset = first_sound(x, rate, onsets[-1] + 5.0)
            if onset is not None:
                print(f"<!-- last marker missing: the pass placed by its first sound at {onset:.2f} s -->")
                onsets.append(onset - MARKER - GAP)
        if len(onsets) != expected:
            raise SystemExit(f"{len(onsets)} markers found, {expected} expected: {[round(t, 2) for t in onsets]}")
        at = 0
        for section in sections:
            if section == "blocks":
                continue
            count = SECTION_PASSES[section]
            mine = onsets[at:at + count]
            following = onsets[at + count] if at + count < len(onsets) else None
            at += count
            for index, t in enumerate(mine):
                row = measure_pass(SECTION_KIND[section], x, rate, t)
                if section == "effects":
                    nxt = mine[index + 1] if index + 1 < len(mine) else following
                    if nxt is not None:
                        row["pass_s"] = nxt - t
                results[section].append(row)
            if section == "presets":
                # PotPlayer's 60 Hz tone still plays the previous preset
                # (its switch comes 0.7 to 1 s after the marker): each
                # preset's 60 Hz is the next pass's, the last one's unknown.
                rows = results["presets"][-count:]
                lows = [row[0] for row in rows]
                for row, low in zip(rows, lows[1:] + [float("nan")]):
                    row[0] = low
                print("<!-- PotPlayer's 60 Hz: each preset's from the next pass -->")
        if "blocks" in sections:
            after = onsets[-1] + PASS_SECONDS["speakers"] if onsets else 0.0
            segments = block_segments(x, rate, after)
            print(f"<!-- block segments: {[(round(a, 2), round(b, 2)) for a, b in segments]} -->")
            for (name, codec), (start, end) in zip(BLOCK_FILES, segments):
                block, error, offset = fit_block(x, rate, start, end, 44100 if "44" in name else 48000)
                results["blocks"].append((name, codec, block, error, offset))
    return results


def fmt(value):
    """A level, or a dash below -100 dB (nothing there)."""
    if not math.isfinite(value) or value < -100:
        return "—"
    return f"{value:.0f}" if value > 100 else f"{value:+.1f}"


def report(pot, rvp):
    print("### Equalizer presets (gain of each band against (Default), dB)\n")
    print("| Preset | Player | " + " | ".join(f"{f} Hz" for f in BANDS) + " |")
    print("|---|---|" + "---:|" * len(BANDS))
    for index, (name, values) in enumerate(PRESETS):
        rows = [("RVP's values", values)]
        for label, side in (("PotPlayer", pot), ("RVP", rvp)):
            if side:
                flat = side["presets"][0]
                rows.append((label, [a - b for a, b in zip(side["presets"][index], flat)]))
        for label, gains in rows:
            print(f"| {name if label == rows[0][0] else ''} | {label} | " + " | ".join(fmt(g) for g in gains) + " |")
    # PotPlayer's output reached the recording lower than RVP's renders:
    # its levels are raised by the difference on a tone both play alike
    # (the left-only 1 kHz without an effect, FR in 2.0 Stereo).
    effect_offset = speaker_offset = 0.0
    if pot and rvp:
        effect_offset = rvp["effects"][0]["left_L"] - pot["effects"][0]["left_L"]
        speaker_offset = rvp["speakers"][0][1][1] - pot["speakers"][0][1][1]
        print(f"\n<!-- PotPlayer raised by {effect_offset:+.1f} dB (effects) and {speaker_offset:+.1f} dB (speakers) -->")
    print("\n### Effects (levels in dB of full scale; both players at gain 1, normalizers off)\n")
    keys = list(pot["effects"][0].keys()) if pot else list(rvp["effects"][0].keys())
    print("| Effect | Player | " + " | ".join(keys) + " |")
    print("|---|---|" + "---:|" * len(keys))
    for index, (name, _) in enumerate(EFFECTS):
        for label, side in (("PotPlayer", pot), ("RVP", rvp)):
            if not side:
                continue
            row = side["effects"][index]
            if row is None:
                continue
            shift = effect_offset if label == "PotPlayer" else 0.0
            values = [row.get(k, float("nan")) + (shift if k not in ("centre_hz", "pass_s") else 0.0) for k in keys]
            print(f"| {name if label == 'PotPlayer' or not pot else ''} | {label} | " + " | ".join(fmt(v) for v in values) + " |")
    print("\n### Speaker modes (left / right level of each source channel, dB)\n")
    print("| Mode | Player | " + " | ".join(SPEAKER_CHANNELS) + " |")
    print("|---|---|" + "---|" * len(SPEAKER_CHANNELS))
    for index, (name, _) in enumerate(SPEAKERS):
        for label, side in (("PotPlayer", pot), ("RVP", rvp)):
            if side:
                shift = speaker_offset if label == "PotPlayer" else 0.0
                cells = [f"{fmt(l + shift)} / {fmt(r + shift)}" for l, r in side["speakers"][index]]
                print(f"| {name if label == 'PotPlayer' or not pot else ''} | {label} | " + " | ".join(cells) + " |")
    if pot and pot.get("blocks"):
        rules = {"ac3": 1536, "eac3": 1536, "mp3": 2304, "opus": 2880, "pcm_s16le": 3000}
        print("\n### Normalizer blocks (PotPlayer's fit, RVP's rule)\n")
        print("| File | Codec | PotPlayer | fit RMS, dB | RVP |")
        print("|---|---|---:|---:|---:|")
        for name, codec, block, error, _ in pot["blocks"]:
            print(f"| {name} | {codec} | {block} | {error:.3f} | {rules.get(codec, 2048)} |")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--files", required=True, type=Path)
    parser.add_argument("--render", required=True)
    parser.add_argument("--work", required=True, type=Path)
    parser.add_argument(
        "--recording", action="append", default=[],
        help="WAV=section,section... (presets, effects, speakers, blocks), in the recording's order",
    )
    args = parser.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    rvp = rvp_side(args.files, args.render, args.work)
    recordings = [(Path(item.split("=")[0]), item.split("=")[1].split(",")) for item in args.recording]
    pot = potplayer_side(recordings) if recordings else None
    (args.work / "results.json").write_text(json.dumps({"rvp": rvp, "potplayer": pot}, indent=1))
    report(pot, rvp)


if __name__ == "__main__":
    main()
