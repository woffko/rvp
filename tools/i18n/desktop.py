#!/usr/bin/env python3
"""Writes the desktop entry's translated fields (M19).

`packaging/linux/rvp.desktop` is the packages' entry and the template of
the user's own (`registration::desktop_entry`). Its `GenericName[code]`
and `Comment[code]` lines come from each language's
`shell/i18n/packaging_<code>.ts` ("Desktop" context); this script
rewrites them, and `crates/app/tests/translations.rs` fails when they
differ from the files.

    tools/i18n/desktop.py [--check]
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tsfile  # noqa: E402

ENTRY = tsfile.ROOT / "packaging" / "linux" / "rvp.desktop"
KEYS = ("GenericName", "Comment")


def rewrite(text: str) -> str:
    lines = [line for line in text.splitlines() if not any(line.startswith(f"{key}[") for key in KEYS)]
    out = []
    for line in lines:
        out.append(line)
        key, _, english = line.partition("=")
        if key in KEYS:
            for code in tsfile.codes():
                translated = tsfile.translations(code, "packaging").get(("Desktop", english))
                if translated:
                    out.append(f"{key}[{code}]={translated}")
    return "\n".join(out) + "\n"


def main() -> int:
    text = ENTRY.read_text(encoding="utf-8")
    wanted = rewrite(text)
    if "--check" in sys.argv[1:]:
        if wanted != text:
            print(f"{ENTRY} differs from the translations: run tools/i18n/desktop.py", file=sys.stderr)
            return 1
        return 0
    ENTRY.write_text(wanted, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
