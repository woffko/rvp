#!/usr/bin/env python3
"""Builds RVP's packages (M8) from a release build.

windows: the Windows folder (rvp.exe, Qt, libmpv, the C++ runtime and,
    with --vapoursynth, RVP's VapourSynth runtime in `vapoursynth/`, made
    by tools/packaging/vapoursynth.py; with --whisper, whisper.cpp's
    CPU and Vulkan engine in `whisper/`, made by tools/whisper/build.sh;
    no models; with --discs, libdvdnav and libbluray for the disc menus,
    made by tools/discs/build.sh), the portable ZIP (with
    rvp.portable) and the NSIS installer (packaging/windows/rvp.nsi).
linux: the AppDir, the AppImage and the .deb. Build on the oldest system
    the packages support (Ubuntu 22.04); RVP's binary must carry
    `-Wl,-rpath,$ORIGIN/../lib -Wl,--disable-new-dtags` (see
    tools/linux-kit/kit.sh m8package), so the bundled libraries load
    without LD_LIBRARY_PATH, which programs RVP starts would inherit.
    With --whisper, whisper.cpp's CPU and Vulkan engine in `bin/whisper/`
    (tools/whisper/build-linux.sh); with --vapoursynth, RVP's VapourSynth
    runtime in `bin/vapoursynth/` (tools/vapoursynth/build-linux.sh); the
    Unified Remote remote in `share/rvp`.

Both read the registered types from packaging/associations.tsv. The
version is the workspace's plus the commit's time and hash:
0.1.0-alpha.1+git.<UTC time>.<hash>, the time first so that versions grow
with the commits (`RVP_COMMIT` overrides the hash, SOURCE_DATE_EPOCH the time). Outputs are reproducible: sorted
entries and the commit's time as every file's time.

usage:
  package.py version | mime-types
  package.py windows --exe rvp.exe --qt QT --mpv libmpv-2.dll --runtime DIR
      --look DIR --out DIR [--vapoursynth DIR] [--whisper DIR] [--discs DIR] [--makensis PATH]
      [--nsisdir DIR]
      [--objdump PATH]
  package.py linux --exe rvp --qt QT --mpv DIR --look DIR --out DIR
      --runtime APPIMAGE_RUNTIME [--whisper DIR] [--vapoursynth DIR] [--mksquashfs PATH]
  package.py libmpv --archive mpv-dev-*.7z --out DIR [--sevenzip PATH]
"""

import argparse
import array
import time
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKAGING = ROOT / "packaging"
sys.path.insert(0, str(ROOT / "tools" / "i18n"))
import tsfile  # noqa: E402  (RVP's translation files, M19)

# The AppImage runtime: AppImage/type2-runtime's release 20251108
# (runtime-x86_64), checked by its SHA-256 so another file is noticed.
APPIMAGE_RUNTIME_URL = (
    "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64"
)
APPIMAGE_RUNTIME_SHA256 = "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"


def commit() -> str:
    value = os.environ.get("RVP_COMMIT", "").strip()
    if value:
        return value
    result = subprocess.run(
        ["git", "-C", str(ROOT), "rev-parse", "--short=9", "HEAD"],
        capture_output=True, text=True, check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def commit_time() -> int:
    """The commit's time, as every packaged file's time."""
    value = os.environ.get("SOURCE_DATE_EPOCH", "").strip()
    if value:
        return int(value)
    result = subprocess.run(
        ["git", "-C", str(ROOT), "log", "-1", "--format=%ct"],
        capture_output=True, text=True, check=False,
    )
    return int(result.stdout.strip()) if result.returncode == 0 and result.stdout.strip() else 0


def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    section = text.split("[workspace.package]", 1)[1]
    match = re.search(r'^version\s*=\s*"([^"]+)"', section, re.M)
    if not match:
        raise SystemExit("no version in [workspace.package]")
    return match.group(1)


def version() -> str:
    """The workspace's version, the commit's time (UTC) and its hash:
    0.1.0-alpha.1+git.20261114105222.c3f58dc13. The time first, so that a later
    commit's package sorts higher (dpkg compares the hash's digits and
    letters, which are random)."""
    stamp = time.strftime("%Y%m%d%H%M%S", time.gmtime(commit_time()))
    return f"{workspace_version()}+git.{stamp}.{commit()}"


def numeric_version() -> str:
    """Windows' four numbers: the workspace's before its pre-release part
    (0.1.0-alpha.1 is 0.1.0.0)."""
    core = workspace_version().split("-", 1)[0].split("+", 1)[0]
    return ".".join((core.split(".") + ["0", "0", "0"])[:3]) + ".0"


def debian_version() -> str:
    """The .deb's version: a pre-release takes `~`, which dpkg sorts before
    the release (0.1.0~alpha.1 < 0.1.0), and the commit's part loses its
    first dot."""
    return version().replace("+git.", "+git").replace("-", "~") + "-1"


def associations() -> list[tuple[str, str, list[str]]]:
    """(extension, kind, MIME types) of packaging/associations.tsv."""
    rows = []
    for line in (PACKAGING / "associations.tsv").read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        extension, kind, mime = line.split("\t")
        rows.append((extension, kind, [] if mime == "-" else mime.split(";")))
    return rows


def mime_types() -> list[str]:
    seen: list[str] = []
    for _, _, types in associations():
        for mime in types:
            if mime not in seen:
                seen.append(mime)
    return seen


def desktop_entry() -> str:
    template = (PACKAGING / "linux" / "rvp.desktop").read_text(encoding="utf-8")
    return template.replace("@MIME_TYPES@", ";".join(mime_types()) + ";")


def run(*args: str, env: dict | None = None) -> None:
    print("+", " ".join(args), flush=True)
    subprocess.run(args, check=True, env=env)


def touch_all(directory: Path, epoch: int) -> None:
    """Every file's time the commit's, and its mode 0755 (folders and
    executables) or 0644, whatever the build's umask (the kit's left the
    packages group-writable)."""
    for path in sorted(directory.rglob("*"), reverse=True):
        if not path.is_symlink():
            mode = path.stat().st_mode
            path.chmod(0o755 if path.is_dir() or mode & 0o111 else 0o644)
            os.utime(path, (epoch, epoch))
    directory.chmod(0o755)
    os.utime(directory, (epoch, epoch))


# ---------------------------------------------------------------- Windows


def pe_imports(objdump: str, path: Path) -> list[str]:
    output = subprocess.run([objdump, "-p", str(path)], capture_output=True, text=True, check=False).stdout
    return [line.split(":", 1)[1].strip() for line in output.splitlines() if "DLL Name:" in line]


def without_libdvdcss(root: Path, pattern: str) -> None:
    """RVP never ships libdvdcss (goal R's decision): no library of a
    package may hold its code, which the names of its settings show
    (`DVDCSS_METHOD`...; tools/libmpv/build.sh and tools/discs/build.sh
    look for the same in what they build). A libdvdread that only loads
    the user's own libdvdcss has none of them. CI's packages are checked
    like a release's: CI takes RVP's own libmpv too (D22)."""
    for library in sorted(root.rglob(pattern)):
        if library.is_file() and not library.is_symlink():
            if b"DVDCSS_" in library.read_bytes():
                raise SystemExit(
                    f"{library} contains libdvdcss, which RVP never ships: "
                    "a package takes RVP's own libmpv (tools/libmpv/build.sh)"
                )


# Files whose bytes cannot be changed in place: an archive keeps a
# checksum of each member.
ARCHIVES = (".jar", ".zip", ".7z", ".whl", ".gz", ".xz", ".bz2", ".cab")


def neutral_paths(home: str) -> list[tuple[bytes, bytes]]:
    """The builder's home folder in the forms a program may hold it (with
    either separator, as UTF-8 and as UTF-16), each with what takes its
    place: the same path with the account's name written as so many
    letters of `rvp`, so that every string keeps its length and its place
    in the file (`/home/NAME/.cargo/...` is `/home/rvp.../.cargo/...`)."""
    pairs = []
    for form in dict.fromkeys((home, home.replace("\\", "/"), home.replace("/", "\\"))):
        cut = max(form.rfind("/"), form.rfind("\\")) + 1
        name = form[cut:]
        other = form[:cut] + ("rvp" * len(name))[: len(name)]
        if name and other != form:
            for encoding in ("utf-8", "utf-16-le"):
                pairs.append((form.encode(encoding), other.encode(encoding)))
    return pairs


def pe_checksum_offset(data: bytes) -> int | None:
    """Where a Windows program or library keeps its checksum (the optional
    header's CheckSum), or None for another kind of file."""
    if len(data) < 0x40 or data[:2] != b"MZ":
        return None
    pe = int.from_bytes(data[0x3C:0x40], "little")
    if pe % 2 or pe + 24 + 68 > len(data) or data[pe : pe + 4] != b"PE\0\0":
        return None
    return pe + 24 + 64


def pe_checksum(data: bytes, at: int) -> int:
    """The checksum Windows' own tools compute (CheckSumMappedFile): the
    file's 16-bit words added with their carries, the field itself left
    out, plus the file's length."""
    words = array.array("H")
    words.frombytes(data[: len(data) & ~1])
    if sys.byteorder != "little":
        words.byteswap()
    total = sum(words) - words[at // 2] - words[at // 2 + 1]
    if len(data) & 1:
        total += data[-1]
    while total >> 16:
        total = (total & 0xFFFF) + (total >> 16)
    return (total + len(data)) & 0xFFFFFFFF


def with_pe_checksum(before: bytes, after: bytes) -> bytes:
    """`after` with its checksum set again when `before` had a right one
    (MinGW's linker writes it, lld-link leaves zero): a changed file with a
    stale checksum looks tampered with."""
    at = pe_checksum_offset(before)
    if at is None:
        return after
    stored = int.from_bytes(before[at : at + 4], "little")
    if stored == 0 or stored != pe_checksum(before, at):
        return after
    return after[:at] + pe_checksum(after, at).to_bytes(4, "little") + after[at + 4 :]


def scrub_builder_paths(root: Path, home: str) -> None:
    """Programs and libraries carry the paths they were built at, in the
    texts of their assertions and panics: the Rust registry and toolchain,
    Qt's headers, the sources of the libmpv, disc and whisper builds, all
    under the builder's home folder, whose last part is an account's name.
    Nothing about the builder's machine goes out with RVP (D21), so each
    such path gets a neutral name of the same length (`neutral_paths`) in
    every file under `root`, and none may be left. The paths only show in
    messages: nothing looks a file up by them on another machine."""
    pairs = neutral_paths(home)
    places = files = 0
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.is_symlink():
            continue
        data = path.read_bytes()
        found = sum(data.count(old) for old, _ in pairs)
        if not found:
            continue
        if path.suffix.lower() in ARCHIVES:
            raise SystemExit(f"{path} holds the builder's folder {home}: an archive cannot be changed in place")
        changed = data
        for old, other in pairs:
            changed = changed.replace(old, other)
        times = path.stat()
        path.write_bytes(with_pe_checksum(data, changed))
        # The file's time stays: the same input gives the same archive.
        os.utime(path, ns=(times.st_atime_ns, times.st_mtime_ns))
        places += found
        files += 1
    for path in sorted(root.rglob("*")):
        if path.is_file() and not path.is_symlink():
            data = path.read_bytes()
            if any(old in data for old, _ in pairs):
                raise SystemExit(f"{path} still holds the builder's folder {home}")
    if places:
        print(f"paths: the builder's folder made neutral in {places} places of {files} files")


def libmpv_archive(args) -> None:
    """RVP's libmpv for Windows as CI and the packages take it: the
    archive tools/libmpv/build.sh made, refused with libdvdcss inside,
    its builder's paths made neutral, packed again under RVP's name. The
    result goes onto the repository's release `libmpv-<date>`, where
    .github/scripts/windows-libmpv.ps1 fetches it and checks its SHA-256,
    which this prints."""
    source = Path(args.archive).resolve()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    target = (out / f"rvp-{source.name}").resolve()
    with tempfile.TemporaryDirectory() as work:
        folder = Path(work) / "libmpv"
        folder.mkdir()
        subprocess.run([args.sevenzip, "x", "-y", "-bso0", f"-o{folder}", str(source)], check=True)
        if not (folder / "libmpv-2.dll").is_file():
            raise SystemExit(f"{source} has no libmpv-2.dll")
        without_libdvdcss(folder, "*.dll")
        scrub_builder_paths(folder, str(Path.home()))
        target.unlink(missing_ok=True)
        names = sorted(entry.name for entry in folder.iterdir())
        subprocess.run([args.sevenzip, "a", "-t7z", "-mx=9", "-bso0", str(target), *names], cwd=folder, check=True)
    print(f"{hashlib.sha256(target.read_bytes()).hexdigest()}  {target.name}")


def windows_bundle(args, bundle: Path) -> None:
    """rvp.exe with Qt (QML and plugins whole, the DLLs it reaches), libmpv
    and the C++ runtime, as windeployqt would lay them out."""
    if bundle.exists():
        shutil.rmtree(bundle)
    bundle.mkdir(parents=True)
    exe = Path(args.exe)
    qt = Path(args.qt)
    shutil.copy2(exe, bundle / "rvp.exe")
    # Windows runs screen savers as .scr programs: rvp.scr starts rvp.exe
    # beside it (M14's Screen Saver page).
    shutil.copy2(exe, bundle / "rvp.scr")

    def ignore(_directory: str, names: list[str]) -> list[str]:
        present = set(names)
        skipped = []
        for name in names:
            stem, _, extension = name.rpartition(".")
            if extension in ("pdb", "lib", "prl"):
                skipped.append(name)
            elif extension == "dll" and stem.endswith("d") and f"{stem[:-1]}.dll" in present:
                skipped.append(name)
        return skipped

    shutil.copytree(qt / "qml", bundle / "qml", ignore=ignore)
    shutil.copytree(qt / "plugins", bundle / "plugins", ignore=ignore)
    (bundle / "qt.conf").write_text("[Paths]\nPrefix = .\nPlugins = plugins\nQmlImports = qml\n")
    available = {p.name.lower(): p for p in (qt / "bin").glob("*.dll")}
    pending = [bundle / "rvp.exe", *(bundle / "plugins").rglob("*.dll"), *(bundle / "qml").rglob("*.dll")]
    copied: set[str] = set()
    while pending:
        current = pending.pop()
        for name in pe_imports(args.objdump, current):
            key = name.lower()
            if key in available and key not in copied:
                copied.add(key)
                target = bundle / available[key].name
                shutil.copy2(available[key], target)
                pending.append(target)
    for name in ("opengl32sw.dll", "d3dcompiler_47.dll"):
        if name in available:
            shutil.copy2(available[name], bundle / available[name].name)
    shutil.copy2(args.mpv, bundle / Path(args.mpv).name)
    for dll in sorted(Path(args.runtime).glob("*.dll")):
        shutil.copy2(dll, bundle / dll.name)
    if args.vapoursynth:
        # M18's smoothing: RVP's own runtime, which it finds next to itself.
        shutil.copytree(args.vapoursynth, bundle / "vapoursynth")
    if args.whisper:
        # M13's subtitles from audio: the engine RVP finds next to itself.
        shutil.copytree(args.whisper, bundle / "whisper")
    if args.discs:
        # M16's disc menus: the libraries RVP loads from its own folder,
        # libbluray's BD-J jars beside it (it looks there) and their
        # licences. Never libdvdcss, libaacs or keys.
        discs = Path(args.discs)
        names = ["libdvdnav-4.dll", "libbluray-4.dll"]
        names += sorted(p.name for p in discs.glob("libbluray-*.jar"))
        names += sorted(p.name for p in discs.glob("*-COPYING.txt"))
        for name in names:
            if "dvdcss" in name or "aacs" in name:
                raise SystemExit(f"{name} must not be in RVP's package")
            shutil.copy2(discs / name, bundle / name)
    shutil.copy2(Path(args.look) / "icons" / "rvp.ico", bundle / "rvp.ico")
    remote_folder(Path(args.look), bundle)
    shutil.copy2(ROOT / "LICENSE", bundle / "LICENSE.txt")
    third_party(args, bundle, "licenses", "LICENSE.txt", "x86_64-pc-windows-msvc")
    without_libdvdcss(bundle, "*.dll")
    scrub_builder_paths(bundle, str(Path.home()))
    print(f"bundle: {len(copied)} Qt libraries in {bundle}")


# Third-party notices (the first public alpha, 2026-11-18): every package
# carries THIRD-PARTY.txt and a licences folder with the licence files of
# what it bundles. A source tree's licence files are the files at its top
# whose names say so.
LICENCE_NAME = re.compile(r"^(licen[cs]es?|copying|copyright|notice|unlicense|patents)([._-].*)?$", re.I)
# The system's standard licence texts (Debian and Ubuntu), by the names
# THIRD-PARTY.txt gives them.
COMMON_LICENCES = Path("/usr/share/common-licenses")
COMMON_NAMES = {
    "GPL-2": "GPL-2.0",
    "GPL-3": "GPL-3.0",
    "LGPL-2.1": "LGPL-2.1",
    "LGPL-3": "LGPL-3.0",
    "Apache-2.0": "Apache-2.0",
    "MPL-2.0": "MPL-2.0",
}
NOTICE_HEAD = """Third-party software in RVP {version}

RVP is free software under the GNU General Public License 3.0 or later
({licence}). It includes the programs and libraries below, each under its
own licence; their licence files are in {folder}/<name>/.{common}
The VapourSynth runtime, whisper.cpp and the disc libraries carry their
own licence files beside them. Each component's source is its project's
own, at the versions RVP's build scripts pin in
https://github.com/woffko/rvp (tools/libmpv/build.sh, tools/linux-kit/kit.sh,
tools/packaging/vapoursynth.py, tools/whisper/, tools/discs/, Cargo.lock).
"""


def licence_files(folder: Path) -> list[Path]:
    """The licence files at the top of a source tree."""
    if not folder.is_dir():
        return []
    return sorted(p for p in folder.iterdir() if p.is_file() and LICENCE_NAME.match(p.name))


def rust_crates(target: str) -> list[dict]:
    """The crates rvp is built from: rvp-shell's normal dependencies on
    TARGET, the workspace's own left out (`cargo metadata`)."""
    output = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    data = json.loads(output)
    packages = {p["id"]: p for p in data["packages"]}
    nodes = {n["id"]: n for n in data["resolve"]["nodes"]}
    workspace = set(data["workspace_members"])
    start = next(i for i in workspace if packages[i]["name"] == "rvp-shell")
    seen: set[str] = set()
    pending = [start]
    while pending:
        for dep in nodes[pending.pop()]["deps"]:
            normal = any(kind["kind"] is None for kind in dep["dep_kinds"])
            if normal and dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                pending.append(dep["pkg"])
    return sorted((packages[i] for i in seen if i not in workspace), key=lambda p: (p["name"], p["version"]))


def debian_copyrights(libraries: dict[str, Path]) -> dict[str, Path]:
    """The Debian packages the bundled system libraries come from, each
    with its copyright file."""
    found: dict[str, Path] = {}
    for library in sorted(libraries.values()):
        result = subprocess.run(["dpkg", "-S", str(library.resolve())], capture_output=True, text=True)
        if result.returncode != 0:
            result = subprocess.run(["dpkg", "-S", str(library)], capture_output=True, text=True)
        if result.returncode != 0:
            raise SystemExit(f"no Debian package owns {library}")
        package = result.stdout.split(":", 1)[0].strip()
        copyright_file = Path("/usr/share/doc") / package / "copyright"
        if not copyright_file.is_file():
            raise SystemExit(f"{package} has no copyright file")
        found[package] = copyright_file
    return found


def third_party(args, root: Path, folder: str, licence: str, target: str,
                debian: dict[str, Path] | None = None) -> None:
    """THIRD-PARTY.txt in ROOT and the licence files in ROOT/FOLDER: Qt,
    the components in --licences (a folder of <name>/<licence files>, from
    tools/libmpv/licences.sh or the kit's libmpv), the Debian packages of
    the bundled system libraries (Linux) and the Rust crates. The
    VapourSynth runtime, whisper.cpp and the disc libraries carry their
    own licence files."""
    licences = root / folder
    if licences.exists():
        shutil.rmtree(licences)
    common = f"\nThe standard licence texts are in {folder}/common/." if COMMON_LICENCES.is_dir() else ""
    lines = [NOTICE_HEAD.format(version=version(), licence=licence, folder=folder, common=common)]
    if COMMON_LICENCES.is_dir():
        (licences / "common").mkdir(parents=True)
        for source, name in COMMON_NAMES.items():
            shutil.copy2(COMMON_LICENCES / source, licences / "common" / name)
    qt = Path(args.qt)
    lines.append(f"Qt {qt.parent.name} (its libraries, plugins and QML modules)")
    lines.append(f"    LGPL-3.0 (a few modules GPL-3.0): {folder}/common/LGPL-3.0 and GPL-3.0")
    lines.append("    https://download.qt.io/official_releases/qt/")
    if args.licences and not Path(args.licences).is_dir():
        raise SystemExit(f"no licence folder {args.licences}")
    if args.licences:
        for component in sorted(Path(args.licences).iterdir()):
            if not component.is_dir() or any(word in component.name for word in ("dvdcss", "aacs")):
                continue
            files = sorted(p for p in component.iterdir() if p.is_file())
            if not files:
                continue
            (licences / component.name).mkdir(parents=True)
            for file in files:
                shutil.copy2(file, licences / component.name / file.name)
            lines.append(component.name)
            lines.append(f"    {', '.join(f'{folder}/{component.name}/{f.name}' for f in files)}")
    for package, copyright_file in sorted((debian or {}).items()):
        (licences / "debian" / package).mkdir(parents=True)
        shutil.copy2(copyright_file, licences / "debian" / package / "copyright")
        lines.append(f"{package} (Ubuntu's package)")
        lines.append(f"    {folder}/debian/{package}/copyright")
    lines.append("")
    lines.append("Rust crates, built into rvp:")
    for crate in rust_crates(target):
        name = f"{crate['name']}-{crate['version']}"
        files = licence_files(Path(crate["manifest_path"]).parent)
        if files:
            (licences / "rust" / name).mkdir(parents=True)
            for file in files:
                shutil.copy2(file, licences / "rust" / name / file.name)
        where = f"{folder}/rust/{name}/" if files else "the standard text in common/"
        lines.append(f"{crate['name']} {crate['version']}: {crate.get('license') or crate.get('license_file')} ({where})")
    (root / "THIRD-PARTY.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"third-party notices: {len(lines)} lines in {root / 'THIRD-PARTY.txt'}")


REMOTE = Path("remote") / "unified-remote" / "RVP"


def remote_folder(look: Path, bundle: Path) -> None:
    """M15's Unified Remote remote beside rvp.exe: its files from the
    repository and its icons from `cargo xtask look`."""
    target = bundle / REMOTE
    target.mkdir(parents=True)
    for source in sorted((ROOT / REMOTE).iterdir()):
        shutil.copy2(source, target / source.name)
    # Its icons and button pictures (crates/look/src/outputs.rs).
    for name in ("icon.png", "icon_hires.png", "mute.png"):
        shutil.copy2(look / REMOTE / name, target / name)


def windows_zip(bundle: Path, target: Path, epoch: int) -> None:
    """The portable ZIP: the folder as `RVP/` with rvp.portable."""
    stamp = time_tuple(epoch)
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        files = sorted(p for p in bundle.rglob("*") if p.is_file())
        entries = [(f"RVP/{p.relative_to(bundle).as_posix()}", p.read_bytes()) for p in files]
        entries.append(("RVP/rvp.portable", b""))
        for name, data in sorted(entries):
            info = zipfile.ZipInfo(name, date_time=stamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, data)
    print(f"zip: {target}")


def time_tuple(epoch: int) -> tuple:
    # ZIP times start in 1980.
    return time.gmtime(max(epoch, 315532800))[:6]


def nsis_string(text: str) -> str:
    return text.replace("$", "$$").replace('"', '$\\"')


def windows_includes(bundle: Path, includes: Path) -> None:
    """types.nsh (the registry entries per type) and files.nsh (what the
    uninstaller removes). Shared keys (`.ext`, its `OpenWithProgids`) are
    removed on uninstall only when the installer created them and they are
    empty again: `Software\\RVP\\Created` remembers which it created."""
    classes = r"Software\Classes"
    created = r"Software\RVP\Created"

    def remember(key: str, name: str) -> list[str]:
        # EnumRegKey sets the error flag when the key does not exist.
        return [
            "    ClearErrors",
            f'    EnumRegKey $0 HKCU "{key}" 0',
            "    IfErrors 0 +2",
            f'        WriteRegDWORD HKCU "{created}" "{name}" 1',
        ]

    def forget(key: str, name: str) -> list[str]:
        return [
            f'    ReadRegDWORD $0 HKCU "{created}" "{name}"',
            "    StrCmp $0 1 0 +2",
            f'        DeleteRegKey /ifnosubkeys /ifnovalues HKCU "{key}"',
        ]

    register = ["!macro RegisterTypes"]
    unregister = ["!macro UnregisterTypes"]
    for extension, kind, _ in associations():
        progid = f"RVP.{extension}"
        name = f"{extension.upper()} {kind} file"
        key = f"{classes}\\.{extension}"
        register += remember(key, f".{extension}")
        register += remember(f"{key}\\OpenWithProgids", f".{extension}\\OpenWithProgids")
        register += [
            f'    WriteRegStr HKCU "{classes}\\{progid}" "" "{nsis_string(name)}"',
            f'    WriteRegStr HKCU "{classes}\\{progid}\\DefaultIcon" "" "$INSTDIR\\rvp.exe,0"',
            f'    WriteRegStr HKCU "{classes}\\{progid}\\shell\\open\\command" "" \'"$INSTDIR\\rvp.exe" -- "%1"\'',
            f'    WriteRegStr HKCU "{key}\\OpenWithProgids" "{progid}" ""',
            f'    WriteRegStr HKCU "{classes}\\Applications\\rvp.exe\\SupportedTypes" ".{extension}" ""',
            f'    WriteRegStr HKCU "Software\\RVP\\Capabilities\\FileAssociations" ".{extension}" "{progid}"',
        ]
        unregister += [
            f'    DeleteRegKey HKCU "{classes}\\{progid}"',
            f'    DeleteRegValue HKCU "{key}\\OpenWithProgids" "{progid}"',
        ]
        unregister += forget(f"{key}\\OpenWithProgids", f".{extension}\\OpenWithProgids")
        unregister += forget(key, f".{extension}")
    register += remember(r"Software\RegisteredApplications", "RegisteredApplications")
    unregister += forget(r"Software\RegisteredApplications", "RegisteredApplications")
    register.append("!macroend")
    unregister.append("!macroend")
    includes.mkdir(parents=True, exist_ok=True)
    (includes / "types.nsh").write_text("\n".join(register + [""] + unregister) + "\n", encoding="utf-8")
    remove = ["!macro RemoveFiles"]
    files = sorted(p for p in bundle.rglob("*") if p.is_file())
    for path in files:
        remove.append(f'    Delete "$INSTDIR\\{path.relative_to(bundle).as_posix().replace("/", chr(92))}"')
    directories = sorted((p for p in bundle.rglob("*") if p.is_dir()), key=lambda p: len(p.parts), reverse=True)
    for path in directories:
        remove.append(f'    RMDir "$INSTDIR\\{path.relative_to(bundle).as_posix().replace("/", chr(92))}"')
    remove.append("!macroend")
    (includes / "files.nsh").write_text("\n".join(remove) + "\n", encoding="utf-8")
    # The Unified Remote remote (M15): copied into Unified Remote's folder,
    # its settings left as the user set them; the uninstaller gets that
    # folder in $0.
    names = sorted(p.name for p in (bundle / REMOTE).iterdir())
    source = "${SOURCE}\\" + str(REMOTE).replace("/", "\\")
    install = ["!macro InstallRemote"]
    for name in names:
        line = f'    File "{source}\\{name}"'
        install += ["    SetOverwrite off", line, "    SetOverwrite on"] if name == "settings.prop" else [line]
    install.append("!macroend")
    uninstall = ["!macro RemoveRemote"] + [f'    Delete "$0\\{name}"' for name in names]
    uninstall += ['    RMDir "$0"', "!macroend"]
    (includes / "remote.nsh").write_text("\n".join(install + [""] + uninstall) + "\n", encoding="utf-8")


def nsis_languages(nsisdir: Path, includes: Path) -> None:
    """languages.nsh: the installer's languages besides English (M19), one
    for each language RVP has (`shell/i18n`), found among NSIS's own
    language files by the Windows locale of their LANGID (Python's
    `locale.windows_locale`), with rvp.nsi's English LangStrings translated
    from the language's packaging_<code>.ts ("Installer"); English where a
    string has no translation. No language is named here; one NSIS lacks
    stops the build."""
    import locale

    folder = nsisdir / "Contrib" / "Language files"
    offered: list[tuple[str, str]] = []
    for nlf in sorted(folder.glob("*.nlf")):
        lines = nlf.read_text(encoding="utf-8", errors="replace").splitlines()
        for at, line in enumerate(lines[:-1]):
            if line.strip() == "# Language ID" and lines[at + 1].strip().isdigit():
                name = locale.windows_locale.get(int(lines[at + 1].strip()))
                if name:
                    offered.append((name, nlf.stem))
                break
    english = re.findall(
        r'^LangString (\w+) \$\{LANG_ENGLISH\} "(.*)"$',
        (PACKAGING / "windows" / "rvp.nsi").read_text(encoding="utf-8"),
        re.M,
    )
    lines = ["; Generated by tools/packaging/package.py from shell/i18n (M19)."]
    for code in tsfile.codes():
        exact = [nsis for name, nsis in offered if name.lower() == code.lower()]
        # A code without a region: the language's own country first
        # (de_DE), then any (ja_JP).
        home = [nsis for name, nsis in offered if name.lower() == f"{code}_{code}".lower()]
        same = [nsis for name, nsis in offered if name.lower().startswith(code.lower() + "_")]
        found = (exact or home or same or [None])[0]
        if found is None:
            raise SystemExit(f"NSIS has no language file for {code} in {folder}")
        texts = tsfile.translations(code, "packaging")
        lines.append(f'!insertmacro MUI_LANGUAGE "{found}"')
        for symbol, text in english:
            translated = texts.get(("Installer", text.replace('$\\"', '"').replace("$$", "$")), text)
            lines.append(f'LangString {symbol} ${{LANG_{found.upper()}}} "{nsis_string(translated)}"')
    includes.mkdir(parents=True, exist_ok=True)
    (includes / "languages.nsh").write_text("\n".join(lines) + "\n", encoding="utf-8-sig")


# The oldest NSIS the installer is made with: an older installer started
# as SYSTEM keeps its plug-ins in a folder every user can write to.
NSIS_OLDEST = (3, 11)


def check_makensis(makensis: str, env: dict) -> None:
    """Stops the build when `makensis -VERSION` is older than NSIS_OLDEST
    (it prints v3.11, or v3.11-1 for Debian's)."""
    result = subprocess.run([makensis, "-VERSION"], capture_output=True, text=True, env=env, check=False)
    found = re.match(r"v(\d+)\.(\d+)", result.stdout.strip())
    if result.returncode != 0 or not found:
        raise SystemExit(f"{makensis} -VERSION: {(result.stdout + result.stderr).strip()}")
    if (int(found.group(1)), int(found.group(2))) < NSIS_OLDEST:
        raise SystemExit(
            f"{makensis} is NSIS {result.stdout.strip()}: the installer needs NSIS "
            f"{NSIS_OLDEST[0]}.{NSIS_OLDEST[1]} or later"
        )
    print(f"makensis {result.stdout.strip()}")


def windows(args) -> None:
    # Absolute: makensis reads the paths it gets from the script's folder.
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    name = f"RVP-{version()}-windows-x64"
    epoch = commit_time()
    bundle = out / "bundle"
    windows_bundle(args, bundle)
    touch_all(bundle, epoch)
    windows_zip(bundle, out / f"{name}-portable.zip", epoch)
    if args.makensis:
        includes = out / "nsis"
        windows_includes(bundle, includes)
        nsis_languages(Path(args.nsisdir) if args.nsisdir else Path(args.makensis).resolve().parent, includes)
        numeric = numeric_version()
        env = dict(os.environ)
        if args.nsisdir:
            env["NSISDIR"] = args.nsisdir
        check_makensis(args.makensis, env)
        run(
            args.makensis, "-V2",
            f"-DVERSION={version()}", f"-DNUMERIC={numeric}",
            f"-DSOURCE={bundle}", f"-DINCLUDES={includes}",
            f"-DOUTFILE={out / (name + '-setup.exe')}",
            str(PACKAGING / "windows" / "rvp.nsi"),
            env=env,
        )
    print(f"windows packages in {out}")


# ------------------------------------------------------------------ Linux

# Libraries every desktop system has (glibc, the graphics stack, sound,
# D-Bus, GLib, compression, crypto): the packages use the system's. The
# rest RVP's binary reaches is bundled, from Qt's and libmpv's folders or
# the build system. GnuTLS's own libraries (nettle, hogweed, GMP, libtasn1,
# libidn2, libunistring, p11-kit) come with it: the system's GnuTLS failed
# with a bundled older nettle (Ubuntu 26.10's needs newer symbols). For the
# same reason the system's GTK 3 (the gtk3 theme), its image loaders and
# Kerberos get their own libraries, never the bundled copies the binary's
# RPATH would hand them (the Linux audit's H3): every part of glibc
# (libmvec too, which libvorbis and x265 reach, and which imports
# GLIBC_PRIVATE symbols), GCC's OpenMP runtime, FriBidi, Brotli, LZ4, ACL,
# Kerberos, Little CMS, libjpeg and WebP, whose sonames stay the same across
# the distributions. Libraries whose sonames change between releases (ICU,
# libxml2) stay bundled: the system's copy has another name and cannot
# meet RVP's.
SYSTEM = re.compile(
    r"^(ld-linux.*|libc|libm|libdl|libpthread|librt|libresolv|libutil|libanl|libgcc_s|libstdc\+\+"
    r"|libmvec|libnsl|libnss_.*|libthread_db|libBrokenLocale|libc_malloc_debug|libcrypt|libgomp"
    r"|libfribidi|libbrotlicommon|libbrotlidec|libbrotlienc|liblz4|libacl|libattr"
    r"|libgssapi_krb5|libkrb5|libk5crypto|libcom_err|libkrb5support|libkeyutils"
    r"|liblcms2|libjpeg|libwebp|libwebpmux|libwebpdemux|libsharpyuv"
    r"|libGL|libGLX|libGLdispatch|libOpenGL|libEGL|libgbm|libdrm"
    r"|libX11|libX11-xcb|libXau|libXdmcp|libXext|libXfixes|libXrender|libXrandr|libXi|libXcursor"
    r"|libXinerama|libXss|libXv|libXpresent|libXxf86vm|libxshmfence"
    r"|libxcb|libxcb-(dri2|dri3|glx|present|randr|render|shape|shm|sync|xfixes|xinerama|xinput|xkb)"
    r"|libxkbcommon|libxkbcommon-x11|libwayland-(client|cursor|egl|server)"
    r"|libfontconfig|libfreetype|libharfbuzz|libexpat|libz|libdbus-1|libsystemd|libudev"
    r"|libasound|libpulse|libpulse-simple|libpipewire-0\.3|libjack|libva|libva-drm|libva-x11|libva-wayland|libvdpau"
    r"|libglib-2\.0|libgobject-2\.0|libgio-2\.0|libgthread-2\.0|libgmodule-2\.0|libffi|libpcre|libpcre2-8"
    r"|libssl|libcrypto|liblzma|libzstd|libbz2|libpng16|libuuid|libgcrypt|libgpg-error|libcap|libselinux"
    r"|libmount|libblkid|libgnutls|libnettle|libhogweed|libgmp|libtasn1|libidn2|libunistring|libp11-kit"
    r"|libvulkan)\.so(\.|$)"
)

# Qt's plugin folders the window needs: Wayland and X11, input methods,
# images and icons.
QT_PLUGINS = [
    "platforms", "platforminputcontexts", "wayland-decoration-client",
    "wayland-graphics-integration-client", "wayland-shell-integration", "xcbglintegrations",
    "imageformats", "iconengines",
]
# The desktop's own dialogs: the portal's (Qt takes it inside Flatpak or
# Snap) and GTK 3's, which Qt takes on GNOME and the other GTK desktops.
# Without gtk3 the packages showed Qt Quick's own file dialog there (the
# Ubuntu boot run, 2026-11-14): small, of one size, without the network
# drives of GTK's "Other Locations".
QT_PLATFORM_THEMES = ["libqxdgdesktopportal.so", "libqgtk3.so"]

# Qt's QML modules RVP does not import, nor Qt Quick Controls and Dialogs
# underneath: copied, they would bring Qt Test, SQL, Particles and others
# into usr/lib.
QML_LEFT_OUT = [
    "QtTest", "Qt/test", "QmlTime", "QtQuick/Particles", "QtQuick/LocalStorage", "QtQuick/VectorImage",
    "QtQuick/tooling", "Qt/labs/animation", "Qt/labs/assetdownloader", "Qt/labs/sharedimage",
    "Qt/labs/synchronizer", "Qt/labs/wavefrontmesh", "Qt/labs/settings",
]

# GTK 3's libraries, which only the gtk3 theme reaches: the system's, never
# bundled, and a Recommends rather than a Depends, since Qt passes over a
# theme whose libraries are missing and shows its own dialog.
GTK = re.compile(
    r"^(libgtk-3|libgdk-3|libpangocairo-1\.0|libpango-1\.0|libatk-1\.0|libcairo-gobject|libcairo"
    r"|libgdk_pixbuf-2\.0)\.so(\.|$)"
)

# Ubuntu 24.04 renamed these packages (the 64-bit time_t transition); a
# .deb built on 22.04 accepts either name.
T64 = {
    "libasound2": "libasound2t64",
    "libglib2.0-0": "libglib2.0-0t64",
    "libssl3": "libssl3t64",
    "libpipewire-0.3-0": "libpipewire-0.3-0t64",
    "libgnutls30": "libgnutls30t64",
    "libnettle8": "libnettle8t64",
    "libhogweed6": "libhogweed6t64",
}


def strip(library: Path) -> None:
    """Drops a bundled library's symbol table and debugging sections
    (libmpv's build kept 60 MB of them); rvp keeps its own, which name the
    functions of a panic's backtrace."""
    if shutil.which("strip"):
        library.chmod(library.stat().st_mode | 0o200)
        subprocess.run(["strip", "--strip-unneeded", str(library)], check=True)


def elf_needed(path: Path) -> list[str]:
    output = subprocess.run(
        ["readelf", "-d", str(path)], capture_output=True, text=True, check=False,
        env={**os.environ, "LC_ALL": "C"},
    ).stdout
    return re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", output)


def system_libraries() -> dict[str, Path]:
    output = subprocess.run(["/sbin/ldconfig", "-p"], capture_output=True, text=True, check=False,
                            env={**os.environ, "LC_ALL": "C"}).stdout
    found: dict[str, Path] = {}
    for line in output.splitlines():
        match = re.match(r"\s+(\S+) \(libc6,x86-64[^)]*\) => (\S+)", line)
        if match and match.group(1) not in found:
            found[match.group(1)] = Path(match.group(2))
    return found


def linux_appdir(args, appdir: Path) -> set[str]:
    """The AppDir: usr/bin/rvp with qt.conf, usr/lib (Qt, libmpv, what they
    reach that is not the system's), usr/plugins, usr/qml, the desktop
    entry and the icons. Returns the system libraries it relies on."""
    if appdir.exists():
        shutil.rmtree(appdir)
    usr = appdir / "usr"
    (usr / "bin").mkdir(parents=True)
    (usr / "lib").mkdir()
    qt = Path(args.qt)
    shutil.copy2(args.exe, usr / "bin" / "rvp")
    if args.whisper:
        # M13's engine beside rvp, where rvp_app::speech looks for it; its
        # libraries find each other there ($ORIGIN).
        shutil.copytree(args.whisper, usr / "bin" / "whisper", symlinks=True)
    if args.vapoursynth:
        # M18's runtime beside rvp, where rvp_app::svp looks for VSScript.
        shutil.copytree(args.vapoursynth, usr / "bin" / "vapoursynth", symlinks=True)
    (usr / "bin" / "qt.conf").write_text(
        "[Paths]\nPrefix = ..\nLibraries = lib\nPlugins = plugins\nQmlImports = qml\n"
    )
    for folder in QT_PLUGINS:
        if (qt / "plugins" / folder).is_dir():
            shutil.copytree(qt / "plugins" / folder, usr / "plugins" / folder)
    for theme in QT_PLATFORM_THEMES:
        source = qt / "plugins" / "platformthemes" / theme
        if source.is_file():
            (usr / "plugins" / "platformthemes").mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, usr / "plugins" / "platformthemes" / theme)
    shutil.copytree(qt / "qml", usr / "qml")
    for module in QML_LEFT_OUT:
        shutil.rmtree(usr / "qml" / module, ignore_errors=True)
    places = [qt / "lib", Path(args.mpv)]
    system = system_libraries()
    relied: set[str] = set()
    bundled: set[str] = set()
    from_system: dict[str, Path] = {}
    missing: dict[str, set[str]] = {}
    pending = [usr / "bin" / "rvp", *(usr / "plugins").rglob("*.so"), *(usr / "qml").rglob("*.so")]
    while pending:
        current = pending.pop()
        for name in elf_needed(current):
            if name in bundled:
                continue
            if SYSTEM.match(name):
                relied.add(name)
                continue
            if GTK.match(name):
                continue
            source = next((place / name for place in places if (place / name).exists()), None)
            if source is None and name in system:
                source = system[name]
                from_system[name] = source
            if source is None:
                missing.setdefault(name, set()).add(str(current.relative_to(appdir)))
                continue
            bundled.add(name)
            target = usr / "lib" / name
            shutil.copy2(source.resolve(), target)
            strip(target)
            pending.append(target)
    # A plugin or QML module that needs a library nobody has is left out.
    for name, users in sorted(missing.items()):
        for user in sorted(users):
            path = appdir / user
            if user.startswith("usr/lib/") or user == "usr/bin/rvp":
                raise SystemExit(f"{user} needs {name}, which is not found")
            print(f"left out {user}: needs {name}")
            path.unlink(missing_ok=True)
    share = usr / "share"
    (share / "applications").mkdir(parents=True)
    (share / "applications" / "rvp.desktop").write_text(desktop_entry(), encoding="utf-8")
    icons = Path(args.look) / "icons"
    for icon in sorted(icons.glob("rvp-*.png")):
        size = icon.stem.split("-")[1]
        folder = share / "icons" / "hicolor" / f"{size}x{size}" / "apps"
        folder.mkdir(parents=True, exist_ok=True)
        shutil.copy2(icon, folder / "rvp.png")
    scalable = share / "icons" / "hicolor" / "scalable" / "apps"
    scalable.mkdir(parents=True)
    shutil.copy2(icons / "rvp.svg", scalable / "rvp.svg")
    (share / "doc" / "rvp").mkdir(parents=True)
    shutil.copy2(ROOT / "LICENSE", share / "doc" / "rvp" / "copyright")
    third_party(args, share / "doc" / "rvp", "licenses", "copyright", "x86_64-unknown-linux-gnu",
                debian_copyrights({name: path for name, path in from_system.items() if name in bundled}))
    # M15's Unified Remote remote, which the Remote Control page copies into
    # the server's custom remotes (rvp_app::control::unified).
    remote_folder(Path(args.look), share / "rvp")
    print(f"appdir: {len(bundled)} libraries bundled, {len(relied)} from the system")
    return relied


def appimage(args, appdir: Path, target: Path, epoch: int) -> None:
    """AppRun, the top-level desktop entry and icon, then the runtime and a
    squashfs of the AppDir."""
    (appdir / "AppRun").symlink_to("usr/bin/rvp")
    shutil.copy2(appdir / "usr/share/applications/rvp.desktop", appdir / "rvp.desktop")
    shutil.copy2(appdir / "usr/share/icons/hicolor/256x256/apps/rvp.png", appdir / "rvp.png")
    (appdir / ".DirIcon").symlink_to("rvp.png")
    runtime = Path(args.runtime).read_bytes()
    digest = hashlib.sha256(runtime).hexdigest()
    print(f"AppImage runtime sha256 {digest}")
    if APPIMAGE_RUNTIME_SHA256 and digest != APPIMAGE_RUNTIME_SHA256:
        raise SystemExit("the AppImage runtime is not the one this script knows")
    touch_all(appdir, epoch)
    with tempfile.TemporaryDirectory() as scratch:
        image = Path(scratch) / "rvp.squashfs"
        # SOURCE_DATE_EPOCH sets the image's time and every file's.
        run(args.mksquashfs, str(appdir), str(image), "-root-owned", "-noappend", "-quiet",
            "-comp", "zstd", "-Xcompression-level", "19",
            env={**os.environ, "SOURCE_DATE_EPOCH": str(epoch)})
        target.write_bytes(runtime + image.read_bytes())
    target.chmod(0o755)
    print(f"AppImage: {target}")


def debian_depends(relied: set[str]) -> str:
    """The packages that hold the system libraries the AppDir relies on, with
    their t64 names as alternatives."""
    system = system_libraries()
    packages: set[str] = set()
    for name in sorted(relied):
        path = system.get(name)
        if path is None:
            continue
        owner = subprocess.run(["dpkg", "-S", str(path.resolve())], capture_output=True,
                               text=True, check=False).stdout
        if not owner:
            owner = subprocess.run(["dpkg", "-S", str(path)], capture_output=True,
                                   text=True, check=False).stdout
        if owner:
            packages.add(owner.split(":", 1)[0].strip())
    depends = []
    for package in sorted(packages):
        depends.append(f"{package} | {T64[package]}" if package in T64 else package)
    return ", ".join(depends)


def deb(appdir: Path, relied: set[str], target: Path, epoch: int) -> None:
    """/opt/rvp (the AppDir's usr, with RVP's own share/rvp), /usr/bin/rvp,
    the desktop entry, the icons and the copyright."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "root"
        shutil.copytree(appdir / "usr", root / "opt" / "rvp", symlinks=True)
        (root / "usr" / "share").mkdir(parents=True)
        for moved in ("applications", "icons", "doc"):
            shutil.move(str(root / "opt" / "rvp" / "share" / moved), str(root / "usr" / "share" / moved))
        (root / "usr" / "bin").mkdir(parents=True)
        (root / "usr" / "bin" / "rvp").symlink_to("/opt/rvp/bin/rvp")
        size = sum(p.stat().st_size for p in root.rglob("*") if p.is_file() and not p.is_symlink())
        control = root / "DEBIAN"
        control.mkdir()
        (control / "control").write_text(
            "Package: rvp\n"
            f"Version: {debian_version()}\n"
            "Architecture: amd64\n"
            "Maintainer: RVP <rvp@localhost>\n"
            f"Installed-Size: {size // 1024}\n"
            f"Depends: {debian_depends(relied)}\n"
            # The programs RVP runs: the master volume and recording
            # (pactl, parec), text to speech (spd-say), the desktop's
            # handler cache (update-desktop-database), RIFE (TensorRT)'s
            # download (curl); GTK 3 for the desktop's file dialogs (the
            # gtk3 theme, GTK above); Noto CJK for the Chinese, Japanese and
            # Korean interfaces (M19).
            "Recommends: pulseaudio-utils, speech-dispatcher, desktop-file-utils, curl,"
            " libgtk-3-0 | libgtk-3-0t64, fonts-noto-cjk\n"
            "Section: video\n"
            "Priority: optional\n"
            "Description: PotPlayer-style media player\n"
            " RVP plays video and audio with libmpv, shows PotPlayer's .dsf skins\n"
            " and its own, and works with SVP 4. Qt and libmpv come with it in\n"
            " /opt/rvp.\n"
        )
        touch_all(root, epoch)
        env = {**os.environ, "SOURCE_DATE_EPOCH": str(epoch)}
        run("dpkg-deb", "--root-owner-group", "-Zxz", "--build", str(root), str(target), env=env)
    print(f"deb: {target}")


def linux(args) -> None:
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    epoch = commit_time()
    appdir = out / "AppDir"
    relied = linux_appdir(args, appdir)
    without_libdvdcss(appdir, "*.so*")
    touch_all(appdir, epoch)
    deb(appdir, relied, out / f"rvp_{debian_version()}_amd64.deb", epoch)
    appimage(args, appdir, out / f"RVP-{version()}-x86_64.AppImage", epoch)
    print(f"linux packages in {out}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    kinds = parser.add_subparsers(dest="kind", required=True)
    win = kinds.add_parser("windows")
    win.add_argument("--exe", required=True)
    win.add_argument("--qt", required=True)
    win.add_argument("--mpv", required=True)
    win.add_argument("--runtime", required=True)
    win.add_argument("--vapoursynth", help="RVP's VapourSynth runtime (tools/packaging/vapoursynth.py)")
    win.add_argument("--whisper", help="whisper.cpp's CPU and Vulkan engine (tools/whisper/build.sh)")
    win.add_argument("--discs", help="libdvdnav and libbluray for the disc menus (tools/discs/build.sh)")
    win.add_argument("--look", required=True)
    win.add_argument("--licences", help="libmpv's components' licence files (tools/libmpv/licences.sh)")
    win.add_argument("--out", required=True)
    win.add_argument("--makensis")
    win.add_argument("--nsisdir")
    win.add_argument("--objdump", default=shutil.which("llvm-objdump") or "llvm-objdump-18")
    lin = kinds.add_parser("linux")
    lin.add_argument("--exe", required=True)
    lin.add_argument("--qt", required=True)
    lin.add_argument("--mpv", required=True)
    lin.add_argument("--look", required=True)
    lin.add_argument("--out", required=True)
    lin.add_argument("--runtime", required=True)
    lin.add_argument("--whisper", help="whisper.cpp's CPU and Vulkan engine (tools/whisper/build-linux.sh)")
    lin.add_argument("--vapoursynth", help="RVP's VapourSynth runtime (tools/vapoursynth/build-linux.sh)")
    lin.add_argument("--mksquashfs", default="mksquashfs")
    lin.add_argument("--licences", help="libmpv's components' licence files (the kit's libmpv)")
    mpv = kinds.add_parser("libmpv", help="RVP's libmpv archive for the release CI fetches it from")
    mpv.add_argument("--archive", required=True, help="tools/libmpv/build.sh's mpv-dev-*.7z")
    mpv.add_argument("--out", required=True)
    mpv.add_argument("--sevenzip", default="7z")
    kinds.add_parser("version")
    kinds.add_parser("mime-types", help="the desktop entry's MimeType value")
    args = parser.parse_args()
    if args.kind == "windows":
        windows(args)
    elif args.kind == "linux":
        linux(args)
    elif args.kind == "libmpv":
        libmpv_archive(args)
    elif args.kind == "mime-types":
        print(";".join(mime_types()) + ";")
    else:
        print(version())
    return 0


if __name__ == "__main__":
    sys.exit(main())
