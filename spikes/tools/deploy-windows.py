#!/usr/bin/env python3
"""Assembles a runnable Windows folder for a cross-built Qt spike.

windeployqt is a Windows program, so this does its job from Linux: it copies
the executable, the Qt QML and plugin trees, every Qt DLL reachable through
the import tables (read with llvm-objdump), opengl32sw.dll for machines
without an OpenGL driver, and extra runtime files given on the command line.

usage: deploy-windows.py EXE QT_DIR OUT_DIR [EXTRA_FILE ...]
"""

import pathlib
import shutil
import subprocess
import sys


def imports(path: pathlib.Path) -> list[str]:
    output = subprocess.run(
        ["llvm-objdump-18", "-p", str(path)], capture_output=True, text=True, check=False
    ).stdout
    return [line.split(":", 1)[1].strip() for line in output.splitlines() if "DLL Name:" in line]


def main() -> int:
    if len(sys.argv) < 4:
        print(__doc__)
        return 2
    exe = pathlib.Path(sys.argv[1])
    qt = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    extras = [pathlib.Path(p) for p in sys.argv[4:]]
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    shutil.copy2(exe, out / exe.name)
    # Debug builds (name ending in "d" next to a release twin), debug
    # symbols and static plugin archives are not needed at run time.
    def ignore(directory: str, names: list[str]) -> list[str]:
        present = set(names)
        skipped = []
        for name in names:
            stem, dot, extension = name.rpartition(".")
            if extension in ("pdb", "lib", "prl"):
                skipped.append(name)
            elif extension == "dll" and stem.endswith("d") and f"{stem[:-1]}.dll" in present:
                skipped.append(name)
        return skipped

    shutil.copytree(qt / "qml", out / "qml", ignore=ignore)
    shutil.copytree(qt / "plugins", out / "plugins", ignore=ignore)
    (out / "qt.conf").write_text("[Paths]\nPrefix = .\nPlugins = plugins\nQmlImports = qml\n")
    available = {p.name.lower(): p for p in (qt / "bin").glob("*.dll")}
    pending = [out / exe.name]
    pending += list((out / "plugins").rglob("*.dll"))
    pending += list((out / "qml").rglob("*.dll"))
    copied: set[str] = set()
    while pending:
        current = pending.pop()
        for name in imports(current):
            key = name.lower()
            if key in available and key not in copied:
                copied.add(key)
                target = out / available[key].name
                shutil.copy2(available[key], target)
                pending.append(target)
    for name in ("opengl32sw.dll", "d3dcompiler_47.dll"):
        if name in available:
            shutil.copy2(available[name], out / available[name].name)
    for extra in extras:
        if extra.is_dir():
            shutil.copytree(extra, out / extra.name, dirs_exist_ok=True)
        else:
            shutil.copy2(extra, out / extra.name)
    print(f"deployed {len(copied)} Qt libraries to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
