#!/usr/bin/env python3
"""Builds RVP's packages (M8) from a release build.

windows: the Windows folder (rvp.exe, Qt, libmpv, the C++ runtime and,
    with --vapoursynth, RVP's VapourSynth runtime in `vapoursynth/`, made
    by tools/packaging/vapoursynth.py; with --whisper, whisper.cpp's
    CPU and Vulkan engine in `whisper/`, made by tools/whisper/build.sh;
    no models), the portable ZIP (with
    rvp.portable) and the NSIS installer (packaging/windows/rvp.nsi).
linux: the AppDir, the AppImage and the .deb. Build on the oldest system
    the packages support (Ubuntu 22.04); RVP's binary must carry
    `-Wl,-rpath,$ORIGIN/../lib -Wl,--disable-new-dtags` (see
    tools/linux-kit/kit.sh m8package), so the bundled libraries load
    without LD_LIBRARY_PATH, which programs RVP starts would inherit.

Both read the registered types from packaging/associations.tsv. The
version is the workspace's plus the commit (no tags): 0.0.1+git.<hash>
(`RVP_COMMIT` overrides the commit). Outputs are reproducible: sorted
entries and the commit's time as every file's time.

usage:
  package.py version | mime-types
  package.py windows --exe rvp.exe --qt QT --mpv libmpv-2.dll --runtime DIR
      --look DIR --out DIR [--vapoursynth DIR] [--whisper DIR] [--makensis PATH] [--nsisdir DIR]
      [--objdump PATH]
  package.py linux --exe rvp --qt QT --mpv DIR --look DIR --out DIR
      --runtime APPIMAGE_RUNTIME [--mksquashfs PATH]
"""

import argparse
import time
import hashlib
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
    return f"{workspace_version()}+git.{commit()}"


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
    for path in sorted(directory.rglob("*"), reverse=True):
        if not path.is_symlink():
            os.utime(path, (epoch, epoch))
    os.utime(directory, (epoch, epoch))


# ---------------------------------------------------------------- Windows


def pe_imports(objdump: str, path: Path) -> list[str]:
    output = subprocess.run([objdump, "-p", str(path)], capture_output=True, text=True, check=False).stdout
    return [line.split(":", 1)[1].strip() for line in output.splitlines() if "DLL Name:" in line]


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
    shutil.copy2(Path(args.look) / "icons" / "rvp.ico", bundle / "rvp.ico")
    remote_folder(Path(args.look), bundle)
    shutil.copy2(ROOT / "LICENSE", bundle / "LICENSE.txt")
    print(f"bundle: {len(copied)} Qt libraries in {bundle}")


REMOTE = Path("remote") / "unified-remote" / "RVP"


def remote_folder(look: Path, bundle: Path) -> None:
    """M15's Unified Remote remote beside rvp.exe: its files from the
    repository and its icons from `cargo xtask look`."""
    target = bundle / REMOTE
    target.mkdir(parents=True)
    for source in sorted((ROOT / REMOTE).iterdir()):
        shutil.copy2(source, target / source.name)
    for name in ("icon.png", "icon_hires.png"):
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
            f'    WriteRegStr HKCU "{classes}\\{progid}\\shell\\open\\command" "" \'"$INSTDIR\\rvp.exe" "%1"\'',
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
        numeric = ".".join((workspace_version().split(".") + ["0", "0", "0"])[:3]) + ".0"
        env = dict(os.environ)
        if args.nsisdir:
            env["NSISDIR"] = args.nsisdir
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
# the build system.
SYSTEM = re.compile(
    r"^(ld-linux.*|libc|libm|libdl|libpthread|librt|libresolv|libutil|libanl|libgcc_s|libstdc\+\+"
    r"|libGL|libGLX|libGLdispatch|libOpenGL|libEGL|libgbm|libdrm"
    r"|libX11|libX11-xcb|libXau|libXdmcp|libXext|libXfixes|libXrender|libXrandr|libXi|libXcursor"
    r"|libXinerama|libXss|libXv|libXpresent|libXxf86vm|libxshmfence"
    r"|libxcb|libxcb-(dri2|dri3|glx|present|randr|render|shape|shm|sync|xfixes|xinerama|xinput|xkb)"
    r"|libxkbcommon|libxkbcommon-x11|libwayland-(client|cursor|egl|server)"
    r"|libfontconfig|libfreetype|libharfbuzz|libexpat|libz|libdbus-1|libsystemd|libudev"
    r"|libasound|libpulse|libpulse-simple|libpipewire-0\.3|libjack|libva|libva-drm|libva-x11|libva-wayland|libvdpau"
    r"|libglib-2\.0|libgobject-2\.0|libgio-2\.0|libgthread-2\.0|libgmodule-2\.0|libffi|libpcre|libpcre2-8"
    r"|libssl|libcrypto|liblzma|libzstd|libbz2|libpng16|libuuid|libgcrypt|libgpg-error|libcap|libselinux"
    r"|libmount|libblkid)\.so(\.|$)"
)

# Qt's plugin folders the window needs: Wayland and X11, input methods,
# the desktop portal's file dialogs, images and icons.
QT_PLUGINS = [
    "platforms", "platforminputcontexts", "wayland-decoration-client",
    "wayland-graphics-integration-client", "wayland-shell-integration", "xcbglintegrations",
    "imageformats", "iconengines",
]
QT_PLATFORM_THEMES = ["libqxdgdesktopportal.so"]

# Ubuntu 24.04 renamed these packages (the 64-bit time_t transition); a
# .deb built on 22.04 accepts either name.
T64 = {
    "libasound2": "libasound2t64",
    "libglib2.0-0": "libglib2.0-0t64",
    "libssl3": "libssl3t64",
    "libpipewire-0.3-0": "libpipewire-0.3-0t64",
}


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
    places = [qt / "lib", Path(args.mpv)]
    system = system_libraries()
    relied: set[str] = set()
    bundled: set[str] = set()
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
            source = next((place / name for place in places if (place / name).exists()), None)
            source = source or system.get(name)
            if source is None:
                missing.setdefault(name, set()).add(str(current.relative_to(appdir)))
                continue
            bundled.add(name)
            target = usr / "lib" / name
            shutil.copy2(source.resolve(), target)
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
    """/opt/rvp (the AppDir's usr), /usr/bin/rvp, the desktop entry, the
    icons and the copyright."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "root"
        shutil.copytree(appdir / "usr", root / "opt" / "rvp", symlinks=True)
        for moved in ("share",):
            shutil.move(str(root / "opt" / "rvp" / moved), str(root / "usr" / moved))
        (root / "usr" / "bin").mkdir(parents=True)
        (root / "usr" / "bin" / "rvp").symlink_to("/opt/rvp/bin/rvp")
        size = sum(p.stat().st_size for p in root.rglob("*") if p.is_file() and not p.is_symlink())
        control = root / "DEBIAN"
        control.mkdir()
        (control / "control").write_text(
            "Package: rvp\n"
            f"Version: {version().replace('+git.', '+git')}-1\n"
            "Architecture: amd64\n"
            "Maintainer: RVP <rvp@localhost>\n"
            f"Installed-Size: {size // 1024}\n"
            f"Depends: {debian_depends(relied)}\n"
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
    touch_all(appdir, epoch)
    deb(appdir, relied, out / f"rvp_{version().replace('+git.', '+git')}-1_amd64.deb", epoch)
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
    win.add_argument("--look", required=True)
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
    lin.add_argument("--mksquashfs", default="mksquashfs")
    kinds.add_parser("version")
    kinds.add_parser("mime-types", help="the desktop entry's MimeType value")
    args = parser.parse_args()
    if args.kind == "windows":
        windows(args)
    elif args.kind == "linux":
        linux(args)
    elif args.kind == "mime-types":
        print(";".join(mime_types()) + ";")
    else:
        print(version())
    return 0


if __name__ == "__main__":
    sys.exit(main())
