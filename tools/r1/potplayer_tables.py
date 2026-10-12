#!/usr/bin/env python3
"""Extracts PotPlayer's static context-menu tree and default hotkey tables
from an installed copy, read-only, for R1 (docs/PLAN.md).

Sources:
  PotPlayer64.dll   RT_MENU 101 (the popup menu: command IDs per item),
                    RT_MENU 111 (the playlist panel's menu) and
                    RT_ACCELERATOR tables (default hotkeys: key -> command ID)
  Language/English.ini [MenuString]  English menu texts by index path,
                    for example 101_0_1_2 = "Open Folder...<TAB>F2"

usage: potplayer_tables.py POTPLAYER_DIR OUT_DIR
Writes OUT_DIR/menu-tree.tsv (docs/r1/menu-static.tsv),
OUT_DIR/menu-playlist.tsv, OUT_DIR/hotkeys.tsv and OUT_DIR/summary.txt.
Needs pefile (pip install pefile==2024.8.26).
"""

import configparser
import struct
import sys
from pathlib import Path

import pefile

POPUP_MENU = 101
PLAYLIST_MENU = 111

VK_NAMES = {
    0x08: "Backspace", 0x09: "Tab", 0x0D: "Enter", 0x13: "Pause", 0x14: "CapsLock",
    0x1B: "Esc", 0x20: "Space", 0x21: "PgUp", 0x22: "PgDn", 0x23: "End", 0x24: "Home",
    0x25: "Left", 0x26: "Up", 0x27: "Right", 0x28: "Down", 0x2C: "PrintScreen",
    0x2D: "Ins", 0x2E: "Del", 0x5D: "Apps", 0x6A: "Num*", 0x6B: "Num+", 0x6D: "Num-",
    0x6E: "Num.", 0x6F: "Num/", 0x90: "NumLock", 0x91: "ScrollLock",
    0xA6: "BrowserBack", 0xA7: "BrowserForward", 0xAD: "VolumeMute", 0xAE: "VolumeDown",
    0xAF: "VolumeUp", 0xB0: "MediaNext", 0xB1: "MediaPrev", 0xB2: "MediaStop",
    0xB3: "MediaPlayPause", 0xBA: ";", 0xBB: "=", 0xBC: ",", 0xBD: "-", 0xBE: ".",
    0xBF: "/", 0xC0: "`", 0xDB: "[", 0xDC: "\\", 0xDD: "]", 0xDE: "'",
}
for code in range(0x30, 0x3A):
    VK_NAMES[code] = chr(code)
for code in range(0x41, 0x5B):
    VK_NAMES[code] = chr(code)
for code in range(0x60, 0x6A):
    VK_NAMES[code] = f"Num{code - 0x60}"
for code in range(0x70, 0x88):
    VK_NAMES[code] = f"F{code - 0x6F}"


def resources(pe: pefile.PE, kind: int) -> dict[int, bytes]:
    found = {}
    for entry in pe.DIRECTORY_ENTRY_RESOURCE.entries:
        if entry.id != kind:
            continue
        for item in entry.directory.entries:
            language = item.directory.entries[0]
            rva, size = language.data.struct.OffsetToData, language.data.struct.Size
            found[item.id] = pe.get_data(rva, size)
    return found


def read_wstring(data: bytes, offset: int) -> tuple[str, int]:
    end = offset
    while data[end:end + 2] != b"\0\0":
        end += 2
    return data[offset:end].decode("utf-16-le"), end + 2


def parse_menu(data: bytes) -> list[dict]:
    """Returns menu items in document order with their index path."""
    version, header = struct.unpack_from("<HH", data, 0)
    items = []
    if version == 1:  # MENUEX_TEMPLATE
        offset = 4 + header

        def walk_ex(offset: int, path: list[int]) -> int:
            index = 0
            while True:
                kind, state, ident, res = struct.unpack_from("<IIIH", data, offset)
                text, offset = read_wstring(data, offset + 14)
                offset = (offset + 3) & ~3
                item = {"path": path + [index], "id": ident, "text": text,
                        "separator": bool(kind & 0x800), "popup": bool(res & 0x01)}
                items.append(item)
                if res & 0x01:
                    offset += 4  # dwHelpId
                    offset = walk_ex(offset, path + [index])
                index += 1
                if res & 0x80:
                    return offset

        walk_ex(offset, [])
    else:  # MENUITEMTEMPLATE
        offset = 4 + header

        def walk(offset: int, path: list[int]) -> int:
            index = 0
            while True:
                (flags,) = struct.unpack_from("<H", data, offset)
                offset += 2
                ident = 0
                if not flags & 0x10:
                    (ident,) = struct.unpack_from("<H", data, offset)
                    offset += 2
                text, offset = read_wstring(data, offset)
                item = {"path": path + [index], "id": ident, "text": text,
                        "separator": not flags & 0x10 and ident == 0 and text == "",
                        "popup": bool(flags & 0x10)}
                items.append(item)
                if flags & 0x10:
                    offset = walk(offset, path + [index])
                index += 1
                if flags & 0x80:
                    return offset

        walk(offset, [])
    return items


def parse_accelerators(data: bytes) -> list[tuple[str, int]]:
    entries = []
    for offset in range(0, len(data) - 7, 8):
        flags, key, ident, _ = struct.unpack_from("<HHHH", data, offset)
        if flags & 0x01:  # FVIRTKEY
            name = VK_NAMES.get(key, f"VK_{key:02X}")
        else:
            name = chr(key)
        parts = [m for bit, m in ((0x08, "Ctrl"), (0x10, "Alt"), (0x04, "Shift")) if flags & bit]
        entries.append(("+".join(parts + [name]), ident))
        if flags & 0x80:
            break
    return entries


KEY_ALIASES = {"ins": "insert", "del": "delete", "pgup": "pgup", "pgdn": "pgdn"}


def normalize_key(text: str) -> str:
    """Compares PotPlayer's shortcut texts ("Alt+PGDN", "Numpad 2") with
    accelerator names ("Alt+PgDn", "Num2"); modifier order is ignored."""
    parts = [part.strip().lower().replace("numpad ", "num") for part in text.split("+")]
    parts = [KEY_ALIASES.get(part, part) for part in parts if part]
    return "+".join(sorted(parts[:-1]) + parts[-1:])


def english_menu(ini_path: Path) -> dict[str, str]:
    text = ini_path.read_text(encoding="utf-8-sig", errors="replace")
    start = text.index("[MenuString]")
    end = text.find("\n[", start + 1)
    section = text[start:end if end > 0 else None]
    parser = configparser.RawConfigParser(strict=False, interpolation=None, delimiters=("=",))
    parser.optionxform = str
    parser.read_string(section)
    return dict(parser.items("MenuString"))


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    root, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    pe = pefile.PE(str(root / "PotPlayer64.dll"), fast_load=True)
    pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_RESOURCE"]])
    menus = resources(pe, pefile.RESOURCE_TYPE["RT_MENU"])
    accelerators = resources(pe, pefile.RESOURCE_TYPE["RT_ACCELERATOR"])
    english = english_menu(root / "Language" / "English.ini")

    def menu_rows(menu_id: int) -> list[tuple]:
        rows = []
        for item in parse_menu(menus[menu_id]):
            # The first item is the container (101_0 "Popup Menu", 111_0
            # "PL"); its children are the menu.
            if len(item["path"]) == 1:
                continue
            key = "_".join(str(i) for i in [menu_id] + item["path"])
            label = english.get(key, item["text"])
            name, _, shortcut = label.partition("\t")
            name, shortcut = name.rstrip(), shortcut.strip()
            if menu_id == POPUP_MENU:
                # As written by the first version of this tool.
                name, _, shortcut = label.partition("\t")
            depth = len(item["path"]) - 2
            if item["separator"]:
                name = "---"
            rows.append((key, depth, item["id"], name, shortcut, item["popup"]))
        return rows

    def write_tree(name: str, rows: list[tuple]) -> None:
        with open(out / name, "w", encoding="utf-8") as tsv:
            tsv.write("path\tdepth\tcommand_id\ttext\tshortcut_shown\tsubmenu\n")
            for key, depth, ident, text, shortcut, popup in rows:
                tsv.write(f"{key}\t{depth}\t{ident}\t{'  ' * depth}{text}\t{shortcut}\t{'yes' if popup else ''}\n")

    rows = menu_rows(POPUP_MENU)
    by_id: dict[int, list[str]] = {}
    for _, _, ident, name, _, popup in rows:
        if ident and not popup and name != "---":
            by_id.setdefault(ident, []).append(name)
    write_tree("menu-tree.tsv", rows)
    write_tree("menu-playlist.tsv", menu_rows(PLAYLIST_MENU))

    # Commands without a popup-menu item may still appear in the other menus
    # (playlist, skin menus); use them as a fallback for naming.
    other_names: dict[int, list[str]] = {}
    for menu_id, data in sorted(menus.items()):
        if menu_id == POPUP_MENU:
            continue
        try:
            other = parse_menu(data)
        except (struct.error, UnicodeDecodeError, IndexError):
            continue
        for item in other:
            if not item["id"] or item["popup"] or item["separator"]:
                continue
            key = "_".join(str(i) for i in [menu_id] + item["path"])
            name = english.get(key, item["text"]).partition("\t")[0]
            other_names.setdefault(item["id"], []).append(f"{name} (menu {menu_id})")

    # Some commands dispatch by context (PgUp is previous item, chapter or
    # channel); the menu shows such keys next to the items they reach.
    shown: dict[str, list[str]] = {}
    for _, _, _, name, shortcut, popup in rows:
        if shortcut:
            label = name.strip() + (" (submenu)" if popup else "")
            shown.setdefault(normalize_key(shortcut), []).append(label)

    with open(out / "hotkeys.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("table\tkey\tcommand_id\tmenu_items\n")
        for table, data in sorted(accelerators.items()):
            for key, ident in parse_accelerators(data):
                names = " | ".join(by_id.get(ident) or other_names.get(ident, []))
                if not names and normalize_key(key) in shown:
                    names = "shown at: " + " | ".join(shown[normalize_key(key)])
                tsv.write(f"{table}\t{key}\t{ident}\t{names}\n")

    with open(out / "summary.txt", "w", encoding="utf-8") as summary:
        summary.write(f"menu items: {len(rows)} (popups {sum(1 for r in rows if r[5])}, "
                      f"separators {sum(1 for r in rows if r[3] == '---')})\n")
        summary.write(f"english menu strings: {len([k for k in english if k.startswith('101_0')])}\n")
        for table, data in sorted(accelerators.items()):
            entries = parse_accelerators(data)
            mapped = sum(1 for _, ident in entries if ident in by_id)
            other = sum(1 for _, ident in entries if ident not in by_id and ident in other_names)
            via_shown = sum(
                1 for key, ident in entries
                if ident not in by_id and ident not in other_names and normalize_key(key) in shown
            )
            summary.write(f"accelerator table {table}: {len(entries)} entries, {mapped} map to popup-menu "
                          f"items, {other} to other menus, {via_shown} by the shortcut shown in the menu\n")
    print((out / "summary.txt").read_text(encoding="utf-8"), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
