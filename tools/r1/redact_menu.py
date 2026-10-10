#!/usr/bin/env python3
"""Removes the owner's personal entries from a runtime context-menu dump
(host-session r1 -> menu-runtime.tsv) before it is committed.

Replaced: recent files ("mkv - Some.File.mkv"), playlist album names in
Album/Favorites, and Windows paths. Everything else is PotPlayer's own text.

usage: redact_menu.py IN.tsv OUT.tsv
"""

import re
import sys

RECENT = re.compile(r"^\s*\w{2,5} - .+\.\w{2,5}\s*$")
PATH = re.compile(r"[A-Za-z]:\\|\\\\")


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    rows = open(sys.argv[1], encoding="utf-8").read().splitlines()
    out = [rows[0]]
    in_albums = False
    album = 0
    replaced = 0
    for row in rows[1:]:
        cells = row.split("\t")
        depth, text = int(cells[0]), cells[2]
        name = text.strip()
        if depth == 0:
            in_albums = name == "Album/Favorites"
        elif in_albums and depth == 1:
            if name == "---":
                in_albums = False
            elif name != "Default":
                album += 1
                cells[2] = "  " * depth + f"<album {album}>"
                replaced += 1
        if RECENT.match(name) or PATH.search(name):
            cells[2] = "  " * depth + "<recent file>"
            replaced += 1
        out.append("\t".join(cells))
    with open(sys.argv[2], "w", encoding="utf-8") as tsv:
        tsv.write("\n".join(out) + "\n")
    print(f"rows={len(out) - 1} replaced={replaced}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
