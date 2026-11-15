#!/usr/bin/env python3
"""The translation work's files (M19, docs/i18n/README.md).

    work.py prepare CODE DIR --reference REF [--size N]
        DIR/CODE/messages.json (every message with an id) and the
        translator's chunks in-NN.json: id, part, context, source,
        comment and REF's translation (a second reference for meaning).
    work.py assemble CODE DIR
        DIR/CODE/ts/<part>_CODE.ts from out-*.json and then fixes*.json
        (id -> translation; fixes-N.json win over fixes.json by N), kept out of the
        repository until `install`.
    work.py install CODE DIR / work.py remove CODE DIR
        copy the language's .ts files into shell/i18n (to build, check or
        commit it) / take them out again when git does not track them.
    work.py back CODE DIR [--size N]
        the reviewer's chunks back-NN.json: id, context, comment and the
        translation, without the English or the reference.
    work.py compare CODE DIR
        compare-NN.json from back-out-*.json (id -> English): source,
        translation, back-translation and flags (placeholders, English
        left, glossary terms of docs/i18n/CODE.md not found, length).

No language is named here: CODE and REF come from the command line.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import unicodedata
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tsfile  # noqa: E402

NAMES = re.compile(
    r"\b(RVP|PotPlayer|BlackSpace|SVP|mpv|libmpv|FFmpeg|VapourSynth|AviSynth|MVTools|RIFE|TensorRT|CUDA|"
    r"NVIDIA|AMD|Vulkan|OpenGL|Direct3D|DirectShow|Qt|Unified Remote|whisper\.cpp|yt-dlp|Winamp|VLC|MPRIS|"
    r"PulseAudio|PipeWire|GNOME|KDE|Windows|Linux|Wayland|X11|DVD|Blu-ray|HDR|SDR|NVDEC|dav1d|TomSteady|"
    r"Crystality|Freeverb|Ctrl|Shift|Alt|Enter|Space|Esc|Tab)\b"
)


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def save(path: Path, data) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")


def chunks(items: list, size: int) -> list[list]:
    return [items[at:at + size] for at in range(0, len(items), size)]


def prepare(code: str, work: Path, reference: str, size: int) -> None:
    messages = []
    for part in tsfile.PARTS:
        path = tsfile.I18N / f"{part}_{reference}.ts"
        if not path.is_file():
            continue
        _, found = tsfile.read(path)
        for index, message in enumerate(found, 1):
            messages.append({
                "id": f"{part[0]}{index:04d}",
                "part": part,
                "context": message.context,
                "source": message.source,
                "comment": message.comment,
                reference: message.translation,
            })
    work.mkdir(parents=True, exist_ok=True)
    save(work / "messages.json", {"reference": reference, "messages": messages})
    for number, chunk in enumerate(chunks(messages, size), 1):
        save(work / f"in-{number:02d}.json", chunk)
    print(f"{code}: {len(messages)} messages in {len(chunks(messages, size))} chunks of {size} in {work}")


def translations(work: Path) -> dict[str, str]:
    found: dict[str, str] = {}
    # The translator's chunks by name, then the fixes by their number
    # (fixes.json first, then fixes-2.json, fixes-3.json...; other names
    # last, by name): a file's time is no order, a rewrite changes it.
    def fix_order(path: Path) -> tuple[int, str]:
        tail = path.stem[len("fixes"):].lstrip("-")
        return (1 if tail == "" else int(tail) if tail.isdigit() else 1000, path.name)

    for pattern, key in (("out-*.json", lambda path: (0, path.name)), ("fixes*.json", fix_order)):
        for path in sorted(work.glob(pattern), key=key):
            for key, value in load(path).items():
                if isinstance(value, str) and value.strip():
                    found[key] = value
    return found


def assemble(code: str, work: Path) -> None:
    messages = load(work / "messages.json")["messages"]
    found = translations(work)
    missing = []
    for part in tsfile.PARTS:
        own = [m for m in messages if m["part"] == part]
        if not own:
            continue
        out = []
        for m in own:
            text = found.get(m["id"], "")
            if not text:
                missing.append(m["id"])
            out.append(tsfile.Message(m["context"], m["source"], m["comment"], text, not text))
        (work / "ts").mkdir(exist_ok=True)
        tsfile.write(work / "ts" / f"{part}_{code}.ts", code, out)
    print(f"{code}: {len(messages) - len(missing)} of {len(messages)} translated"
          + (f"; missing {missing[:10]}" if missing else ""))


def current(code: str, work: Path) -> dict[tuple[str, str, str], str]:
    """The language's translations as assembled in the work folder (else
    as shell/i18n has them)."""
    found = {}
    for part in tsfile.PARTS:
        path = work / "ts" / f"{part}_{code}.ts"
        if not path.is_file():
            path = tsfile.I18N / f"{part}_{code}.ts"
        if not path.is_file():
            continue
        _, messages = tsfile.read(path)
        for m in messages:
            if m.translation and not m.unfinished:
                found[(part, m.context, m.source)] = m.translation
    return found


def install(code: str, work: Path) -> None:
    for path in sorted((work / "ts").glob(f"*_{code}.ts")):
        shutil.copy2(path, tsfile.I18N / path.name)
        print(f"installed {path.name}")


def remove(code: str) -> None:
    import subprocess

    for path in sorted(tsfile.I18N.glob(f"*_{code}.ts")):
        tracked = subprocess.run(["git", "ls-files", "--error-unmatch", str(path)], cwd=tsfile.ROOT,
                                 capture_output=True).returncode == 0
        if tracked:
            print(f"kept {path.name}: git tracks it")
        else:
            path.unlink()
            print(f"removed {path.name}")


def back(code: str, work: Path, size: int) -> None:
    messages = load(work / "messages.json")["messages"]
    now = current(code, work)
    rows = []
    for m in messages:
        text = now.get((m["part"], m["context"], m["source"]), "")
        rows.append({"id": m["id"], "context": m["context"], "comment": m["comment"], "translation": text})
    for number, chunk in enumerate(chunks(rows, size), 1):
        save(work / f"back-{number:02d}.json", chunk)
    print(f"{code}: {len(rows)} strings in {len(chunks(rows, size))} chunks for the back-translation")


def glossary(code: str) -> list[tuple[str, str]]:
    """(English term, the language's term) pairs of docs/i18n/CODE.md."""
    path = tsfile.ROOT / "docs" / "i18n" / f"{code}.md"
    if not path.is_file():
        return []
    pairs = []
    inside = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            inside = line.strip().lower() == "## glossary"
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if not inside or len(cells) < 2 or set(cells[0]) <= set("-: ") or cells[0].lower() == "english":
            continue
        english = [part.strip() for part in cells[0].split(" / ")]
        own = [part.strip() for part in cells[1].split(" / ")]
        if len(english) == len(own):
            pairs += list(zip(english, own))
        else:
            pairs.append((english[0], own[0]))
    return [(e, o) for e, o in pairs if e and o]


def width(text: str) -> int:
    return sum(2 if unicodedata.east_asian_width(c) in "WF" else 1 for c in text)


def found_term(term: str, text: str) -> bool:
    text = text.lower()
    for word in term.lower().split():
        word = re.sub(r"\(.*?\)", "", word).strip(".,;:")
        if not word:
            continue
        stem = word[: max(3, (len(word) * 2 + 2) // 3)] if word.isalpha() and len(word) > 4 else word
        if stem not in text:
            return False
    return True


def compare(code: str, work: Path) -> None:
    messages = {m["id"]: m for m in load(work / "messages.json")["messages"]}
    now = current(code, work)
    backs: dict[str, str] = {}
    for path in sorted(work.glob("back-out-*.json")):
        backs.update(load(path))
    terms = glossary(code)
    rows = []
    counts: dict[str, int] = {}
    for key, m in messages.items():
        source = m["source"]
        text = now.get((m["part"], m["context"], source), "")
        flags = []
        if set(re.findall(r"%\d", source)) != set(re.findall(r"%\d", text)):
            flags.append("placeholders")
        if not text:
            flags.append("untranslated")
        elif text == source and re.search(r"[A-Za-z]{3,}", NAMES.sub("", source)):
            flags.append("same as English")
        for english, own in terms:
            if re.search(rf"(?<![A-Za-z]){re.escape(english)}(?![A-Za-z])", source, re.I) and not found_term(own, text):
                flags.append(f"glossary: {english} -> {own}")
        if len(source) <= 40 and width(text) > 1.35 * width(source) + 2:
            flags.append(f"long: {width(text)} for {width(source)}")
        for flag in flags:
            counts[flag.split(":")[0]] = counts.get(flag.split(":")[0], 0) + 1
        rows.append({"id": key, "context": m["context"], "comment": m["comment"], "source": source,
                     "translation": text, "back": backs.get(key, ""), "flags": flags})
    for number, chunk in enumerate(chunks(rows, 220), 1):
        save(work / f"compare-{number:02d}.json", chunk)
    print(f"{code}: {len(rows)} rows, {len(backs)} back-translated; flags {dict(sorted(counts.items()))}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["prepare", "assemble", "back", "compare", "install", "remove"])
    parser.add_argument("code")
    parser.add_argument("dir", type=Path)
    parser.add_argument("--reference")
    parser.add_argument("--size", type=int, default=220)
    args = parser.parse_args()
    work = args.dir / args.code
    if args.command == "prepare":
        if not args.reference:
            parser.error("prepare needs --reference")
        prepare(args.code, work, args.reference, args.size)
    elif args.command == "assemble":
        assemble(args.code, work)
    elif args.command == "install":
        install(args.code, work)
    elif args.command == "remove":
        remove(args.code)
    elif args.command == "back":
        back(args.code, work, args.size)
    else:
        compare(args.code, work)
    return 0


if __name__ == "__main__":
    sys.exit(main())
