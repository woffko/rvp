"""Qt's .ts translation files as RVP keeps them (M19).

A language is its files `shell/i18n/<part>_<code>.ts`: `qml` (from
lupdate, the QML files' strings), `menu` (the menu and action labels,
QML's `Tools.tr_menu`) and `packaging` (the desktop entry's and the
Windows installer's strings). No language but English, the source text,
is written into RVP's code: the languages are the files found here.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parents[2]
I18N = ROOT / "shell" / "i18n"
PARTS = ("qml", "menu", "packaging")


@dataclass
class Message:
    context: str
    source: str
    comment: str = ""
    translation: str = ""
    unfinished: bool = False


def _unescape(text: str) -> str:
    # lupdate writes control characters as <byte value="xNN" />.
    text = re.sub(r'<byte value="x([0-9a-fA-F]+)" ?/>', lambda m: chr(int(m.group(1), 16)), text)
    return (
        text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", '"')
        .replace("&apos;", "'")
        .replace("&amp;", "&")
    )


def _escape(text: str) -> str:
    """As lupdate escapes: &, < and >, and control characters as bytes."""
    text = escape(text)
    return "".join(
        f'<byte value="x{ord(c):x}" />' if ord(c) < 0x20 and c not in "\n\t" else c for c in text
    )


def _tag(text: str, name: str) -> str | None:
    match = re.search(rf"<{name}(?: [^>]*)?>(.*?)</{name}>", text, re.S)
    return _unescape(match.group(1)) if match else None


def read(path: Path) -> tuple[str, list[Message]]:
    """The file's language attribute and its messages, in order."""
    text = path.read_text(encoding="utf-8")
    language = re.search(r'<TS [^>]*language="([^"]*)"', text)
    messages = []
    for context in text.split("<context>")[1:]:
        name = _tag(context, "name") or ""
        for block in context.split("<message>")[1:]:
            source = _tag(block, "source")
            if source is None:
                continue
            translation = _tag(block, "translation") or ""
            unfinished = 'type="unfinished"' in block or "<translation/>" in block
            messages.append(Message(name, source, _tag(block, "extracomment") or "", translation, unfinished))
    return (language.group(1) if language else ""), messages


def write(path: Path, language: str, messages: list[Message]) -> None:
    """Writes `messages` as lupdate lays a .ts file out (contexts in the
    order they come, `-locations none`)."""
    lines = [
        '<?xml version="1.0" encoding="utf-8"?>',
        "<!DOCTYPE TS>",
        f'<TS version="2.1" language="{language}">',
    ]
    context = None
    for message in messages:
        if message.context != context:
            if context is not None:
                lines.append("    </context>")
            lines += ["    <context>", f"        <name>{_escape(message.context)}</name>"]
            context = message.context
        lines += ["        <message>", f"            <source>{_escape(message.source)}</source>"]
        if message.comment:
            lines.append(f"            <extracomment>{_escape(message.comment)}</extracomment>")
        if message.unfinished or not message.translation:
            lines.append(f'            <translation type="unfinished">{_escape(message.translation)}</translation>')
        else:
            lines.append(f"            <translation>{_escape(message.translation)}</translation>")
        lines.append("        </message>")
    if context is not None:
        lines.append("    </context>")
    lines.append("</TS>")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def codes() -> list[str]:
    """The languages' codes, from the files found (sorted)."""
    found = set()
    for path in I18N.glob("*.ts"):
        part, _, code = path.stem.partition("_")
        if part in PARTS and code:
            found.add(code)
    return sorted(found)


def translations(code: str, part: str) -> dict[tuple[str, str], str]:
    """(context, source) -> translation of a language's part; empty when
    the file is missing."""
    path = I18N / f"{part}_{code}.ts"
    if not path.is_file():
        return {}
    _, messages = read(path)
    return {(m.context, m.source): m.translation for m in messages if m.translation and not m.unfinished}
