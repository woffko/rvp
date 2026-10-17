#!/usr/bin/env python3
"""Extracts PotPlayer's static context-menu tree, default hotkey tables,
Preferences pages and command-line switches from an installed copy,
read-only, for R1 and R2 (docs/PLAN.md). No program is started.

Sources:
  PotPlayer64.dll   RT_MENU 101 (the popup menu: command IDs per item),
                    RT_MENU 111 (the playlist panel's menu),
                    RT_ACCELERATOR tables (default hotkeys: key -> command ID)
                    and RT_DIALOG (the Preferences pages: each control's
                    ID, class, style and place)
  Language/English.ini [MenuString]  English menu texts by index path,
                    for example 101_0_1_2 = "Open Folder...<TAB>F2";
                    [Dialog(N)] the English texts of dialog N's controls
  CmdLine64.txt     the command-line switches (its English block)

The Preferences window builds its page tree in code, not from resources.
PAGES below names each page's dialog: the top level and the Audio pages
as R1's captures of the window show them, the other pages by their
content and PotPlayer's page names (the string table, 2055-2097 and
later).

usage: potplayer_tables.py POTPLAYER_DIR OUT_DIR
Writes OUT_DIR/menu-tree.tsv (docs/r1/menu-static.tsv),
OUT_DIR/menu-playlist.tsv, OUT_DIR/hotkeys.tsv, OUT_DIR/preferences.tsv,
OUT_DIR/command-line.tsv and OUT_DIR/summary.txt.
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


# The Preferences pages: tree path, dialog ID, and whether R1's captures of
# the window show the page under that name (the rest is by content).
PAGES: list[tuple[str, int, bool]] = [
    ("General", 354, True),
    ("General > When Starting", 457, False),
    ("General > Keyboard", 367, False),
    ("General > Mouse", 366, False),
    ("General > Skins", 376, False),
    ("General > Skins > Logo", 378, False),
    ("General > Skins > On Screen Control", 434, False),
    ("General > Skins > Visualization", 372, False),
    ("General > Navigation", 443, False),
    ("General > OSD Messages", 375, False),
    ("General > Dialog Box", 436, False),
    ("General > Playlist", 373, False),
    ("General > Playlist > Color/View", 389, False),
    ("Playback", 358, True),
    ("Playback > Fullscreen Mode", 456, False),
    ("Playback > Time Jump", 364, False),
    ("Playback > Seamless Playback", 379, False),
    ("Playback > Window Size", 490, False),
    ("Playback > Scene Browser", 471, False),
    ("Subtitles", 353, True),
    ("Subtitles > Font Style", 383, False),
    ("Subtitles > Fade/3D/Location", 368, False),
    ("Subtitles > Language/Sync/Other", 442, False),
    ("Subtitles > Subtitle Browser", 478, False),
    ("Subtitles > Online Subtitle Database", 460, False),
    ("Subtitles > Subtitle Translation", 493, False),
    ("Subtitles > Word Searching", 466, False),
    ("Device > Screen Capture", 428, False),
    ("Device > Webcam", 427, False),
    ("Device > Analog TV (WDM)", 380, False),
    ("Device > Digital TV (BDA)", 381, False),
    ("Device > TV Channels", 370, False),
    ("Device > EPG", 482, False),
    ("Device > DVD/Blu-ray", 382, False),
    ("Device > CD/DVD-ROM", 429, False),
    ("Filter Control", 359, True),
    ("Filter Control > Source/Splitter", 350, True),
    ("Filter Control > OpenCodec", 465, True),
    ("Filter Control > Video Decoder", 360, True),
    ("Filter Control > Audio Decoder", 361, True),
    ("Filter Control > Transform Filters", 362, False),
    ("Video", 355, True),
    ("Video > Aspect Ratio", 432, False),
    ("Video > Colorspaces", 384, False),
    ("Video > Resize", 369, False),
    ("Video > Extend/Crop", 374, False),
    ("Video > Deinterlacing", 385, False),
    ("Video > Levels/Offset", 371, False),
    ("Video > Effects", 386, False),
    ("Video > Pixel Shaders", 505, False),
    ("Video > AviSynth", 452, False),
    ("Video > VapourSynth", 487, False),
    ("Video > H.264 MVC 3D", 448, False),
    ("Audio", 356, True),
    ("Audio > Volume Control", 390, True),
    ("Audio > Equalizer", 363, True),
    ("Audio > Normalizer/Freeverb", 387, True),
    ("Audio > Crystality/Noise Reduction", 388, True),
    ("Audio > Language/Sync/Other", 441, True),
    ("Audio > Winamp DSP Plugins", 365, True),
    ("Extensions", 496, True),
    ("Extensions > Media URL", 497, False),
    ("Extensions > Media Playlist/Playitem", 498, False),
    ("Extensions > Media Source", 499, False),
    ("Extensions > Online Subtitle Searching", 500, False),
    ("Extensions > Web Account", 501, False),
    ("Extensions > Playback Statistics", 502, False),
    ("Accessibility > TTS", 468, False),
    ("Accessibility > Touch", 447, False),
    ("Accessibility > XInput", 479, False),
    ("Association", 336, True),
    ("Configuration", 377, True),
    ("Screen Saver", 485, True),
    ("Kakao TV", 357, False),
    ("Kakao TV > Broadcast List", 511, False),
    ("Kakao TV > CEF", 489, False),
]

# Dialog item classes by atom, and the kinds preferences.tsv names.
ATOMS = {0x80: "Button", 0x81: "Edit", 0x82: "Static", 0x83: "ListBox", 0x84: "ScrollBar",
         0x85: "ComboBox"}
# Buttons next to an option that reset or browse it; they belong to it.
HELPER_BUTTONS = {"Default", "R", "...", "D", "0 dB"}


def sz_or_ord(data: bytes, offset: int) -> tuple[object, int]:
    (first,) = struct.unpack_from("<H", data, offset)
    if first == 0:
        return "", offset + 2
    if first == 0xFFFF:
        (ordinal,) = struct.unpack_from("<H", data, offset + 2)
        return ordinal, offset + 4
    return read_wstring(data, offset)


def parse_dialog(data: bytes) -> list[dict]:
    """The controls of a DLGTEMPLATEEX in template order."""
    _, _, _, _, style, count, _, _, _, _ = struct.unpack_from("<HHIIIHhhhh", data, 0)
    offset = 26
    for _ in range(3):  # menu, class, title
        _, offset = sz_or_ord(data, offset)
    if style & 0x40:  # DS_SETFONT: point size, weight, italic, charset, face
        offset += 6
        _, offset = sz_or_ord(data, offset)
    items = []
    for _ in range(count):
        offset = (offset + 3) & ~3
        _, _, item_style, x, y, cx, cy, ident = struct.unpack_from("<IIIhhhhI", data, offset)
        offset += 24
        cls, offset = sz_or_ord(data, offset)
        text, offset = sz_or_ord(data, offset)
        (extra,) = struct.unpack_from("<H", data, offset)
        offset += 2 + extra
        if isinstance(cls, int):
            cls = ATOMS.get(cls, hex(cls))
        items.append({"id": ident, "cls": cls, "style": item_style, "x": x, "y": y, "cx": cx,
                      "cy": cy, "text": text if isinstance(text, str) else ""})
    return items


def control_kind(item: dict) -> str:
    cls, style = item["cls"], item["style"]
    if cls == "Button":
        return {2: "check", 3: "check", 4: "radio", 5: "check", 6: "check", 7: "group",
                9: "radio"}.get(style & 0xF, "button")
    return {"ComboBox": "combo", "Static": "label", "Edit": "edit", "msctls_trackbar32": "slider",
            "msctls_updown32": "spin", "SysLink": "label"}.get(cls, str(cls).lower())


def plain_text(text: str) -> str:
    """A control text without its accelerator marks and line breaks."""
    return text.replace("&&", "\0").replace("&", "").replace("\0", "&").replace("\n", " ").strip()


def english_dialogs(ini_path: Path) -> dict[int, dict[int, str]]:
    parser = configparser.RawConfigParser(strict=False, interpolation=None, delimiters=("=",))
    parser.optionxform = str
    parser.read(ini_path, encoding="utf-8-sig")
    texts = {}
    for section in parser.sections():
        if section.startswith("Dialog(") and section.endswith(")"):
            texts[int(section[7:-1])] = {int(k): v for k, v in parser.items(section) if k.isdigit()}
    return texts


def preference_rows(dialog: list[dict], english: dict[int, str]) -> list[tuple]:
    """(control, kind, text, group) of a page's options: check boxes, radio
    buttons, sliders, lists, fields and buttons, each named by its own text
    or the label on its left (or above), and the group box around it."""
    for item in dialog:
        item["kind"] = control_kind(item)
        item["en"] = plain_text(english.get(item["id"], item["text"]))
    groups = [i for i in dialog if i["kind"] == "group"]
    # A check box can name the slider or field on its right.
    labels = [i for i in dialog if i["kind"] in ("label", "check", "radio") and i["en"]]

    def group_of(item: dict) -> str:
        inside = [g for g in groups if g is not item and g["x"] <= item["x"] < g["x"] + g["cx"]
                  and g["y"] <= item["y"] < g["y"] + g["cy"]]
        return min(inside, key=lambda g: g["cx"] * g["cy"])["en"] if inside else ""

    def label_of(item: dict) -> str:
        # A combo box's height includes its open list: its row is the top.
        middle = item["y"] + min(item["cy"], 14) / 2
        left = [label for label in labels if label["x"] + label["cx"] <= item["x"] + 2
                and label["y"] - 2 <= middle <= label["y"] + label["cy"] + 2]
        if left:
            return max(left, key=lambda label: label["x"])["en"]
        above = [label for label in labels if item["y"] - 16 <= label["y"] + label["cy"] <= item["y"] + 1
                 and label["x"] - 4 <= item["x"] <= label["x"] + label["cx"]]
        return max(above, key=lambda label: label["y"])["en"] if above else ""

    rows = []
    for item in dialog:
        kind = item["kind"]
        if kind in ("label", "spin") or (kind == "button" and item["en"] in HELPER_BUTTONS):
            continue
        text = item["en"] if kind in ("check", "radio", "button", "group") else label_of(item)
        rows.append((item["id"], kind, text, group_of(item) if kind != "group" else ""))
    return rows


def command_line(path: Path) -> list[tuple[str, str]]:
    """The switches of the English block of CmdLine64.txt and what they do;
    continuation lines join the switch above."""
    text = path.read_text(encoding="utf-8-sig", errors="replace")
    block = text[text.index("{$CP0="):]
    block = block[:block.index("$}")]
    switches: list[list[str]] = []
    for line in block.splitlines()[1:]:
        stripped = line.strip()
        if not stripped or stripped.startswith("Usage:"):
            continue
        # The switch ends at the first colon outside brackets (/atv[:ch]).
        depth, cut = 0, -1
        for index, char in enumerate(stripped):
            depth += {"[": 1, "]": -1}.get(char, 0)
            if char == ":" and depth == 0:
                cut = index
                break
        name, sep, rest = (stripped[:cut], ":", stripped[cut + 1:]) if cut >= 0 else (stripped, "", "")
        if sep and not line[:1].isspace() and (name.startswith(("/", '"', "-")) or "=" in name):
            switches.append([" ".join(name.split()), " ".join(rest.split())])
        elif line[:1].isspace() and switches:
            switches[-1][1] += " " + " ".join(stripped.lstrip("-").split())
        elif stripped.startswith("- "):
            switches.append(["-", " ".join(stripped[1:].lstrip(":").split())])
    return [(name, description.strip()) for name, description in switches]


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

    dialogs = resources(pe, pefile.RESOURCE_TYPE["RT_DIALOG"])
    dialog_texts = english_dialogs(root / "Language" / "English.ini")
    options = 0
    with open(out / "preferences.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("page\tdialog\tcontrol\tkind\ttext\tgroup\tname_source\n")
        for page, dialog_id, captured in PAGES:
            for control, kind, label, group in preference_rows(parse_dialog(dialogs[dialog_id]),
                                                               dialog_texts.get(dialog_id, {})):
                options += kind != "group"
                source = "capture" if captured else "content"
                tsv.write(f"{page}\t{dialog_id}\t{control}\t{kind}\t{label}\t{group}\t{source}\n")
    switches = command_line(root / "CmdLine64.txt")
    with open(out / "command-line.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("switch\tdescription\n")
        for name, description in switches:
            tsv.write(f"{name}\t{description}\n")

    with open(out / "summary.txt", "w", encoding="utf-8") as summary:
        summary.write(f"preference pages: {len(PAGES)}, options: {options}\n")
        summary.write(f"command-line switches: {len(switches)}\n")
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
