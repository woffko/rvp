#!/usr/bin/env bash
# Prepares everything needed to cross-build the spikes for Windows (MSVC)
# from Linux/WSL with cargo-xwin. Nothing is installed system-wide; all
# files go to $RVP_DEPS (default ~/rvp-deps).
#
# Needs: python3 with venv, curl, gh (GitHub CLI), cargo-xwin, clang 18+
# (19+ avoids the STL version macro below), llvm-dlltool, llvm-lib.
set -euo pipefail

DEPS=${RVP_DEPS:-$HOME/rvp-deps}
QT_VERSION=6.10.3
MPV_RELEASE=20261010
MPV_ARCHIVE=mpv-dev-x86_64-20261010-git-b2c255c13e.7z
VS_TAG=R73
PYTHON_VERSION=3.12.9
MVTOOLS_TAG=v24
mkdir -p "$DEPS/bin" "$DEPS/win"
cd "$DEPS"

# Python tools: aqtinstall downloads Qt for both platforms.
[ -x venv/bin/aqt ] || { python3 -m venv venv && venv/bin/pip install -q aqtinstall; }
[ -x "qt/$QT_VERSION/gcc_64/bin/qmake" ] ||
    venv/bin/aqt install-qt linux desktop "$QT_VERSION" linux_gcc_64 --outputdir "$DEPS/qt"
[ -e "qt/$QT_VERSION/msvc2022_64/lib/Qt6Core.lib" ] ||
    venv/bin/aqt install-qt windows desktop "$QT_VERSION" win64_msvc2022_64 --outputdir "$DEPS/qt"

# 7-Zip: mpv and mvtools archives use the BCJ2 filter.
if [ ! -x bin/7zz ]; then
    curl -fsSLo 7z-linux.tar.xz https://www.7-zip.org/a/7z2501-linux-x64.tar.xz
    mkdir -p 7z && tar -xf 7z-linux.tar.xz -C 7z && cp 7z/7zz bin/
fi

# LLVM tools under the names cargo-xwin and cc-rs look for.
for tool in llvm-lib llvm-dlltool llvm-rc llvm-ar; do
    command -v "$tool" > /dev/null 2>&1 && continue
    versioned=$(compgen -c "$tool-" | sort -V | tail -1)
    [ -n "$versioned" ] && ln -sf "$(command -v "$versioned")" "bin/$tool"
done

# clang-cl: the MSVC STL in the xwin cache wants clang 19+. With clang 18,
# its documented escape hatch macro is needed.
clang_major=$(clang --version | sed -n 's/.*clang version \([0-9]*\).*/\1/p' | head -1)
if [ "${clang_major:-0}" -lt 19 ]; then
    cat > bin/clang-cl << 'EOF'
#!/bin/sh
exec /usr/bin/clang --driver-mode=cl -D_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH "$@"
EOF
    chmod +x bin/clang-cl
fi

# Qt host tools for the cross build; rcc must not use zstd, which the
# Windows Qt build does not support.
host=$DEPS/qt/$QT_VERSION/gcc_64/libexec
mkdir -p qt-host-tools
for tool in "$host"/*; do ln -sf "$tool" "qt-host-tools/$(basename "$tool")"; done
rm -f qt-host-tools/rcc
printf '#!/bin/sh\nexec %s/rcc --no-zstd "$@"\n' "$host" > qt-host-tools/rcc
chmod +x qt-host-tools/rcc

# qmake stand-in: Windows paths for headers and libraries, Linux paths for
# tools. qt-build-utils reads these with `qmake -query`.
win=$DEPS/qt/$QT_VERSION/msvc2022_64
cat > bin/qmake-msvc-cross << EOF
#!/bin/sh
case "\$2" in
  QT_VERSION) echo $QT_VERSION ;;
  QT_INSTALL_PREFIX|QT_INSTALL_ARCHDATA|QT_INSTALL_DATA) echo $win ;;
  QT_INSTALL_HEADERS) echo $win/include ;;
  QT_INSTALL_LIBS) echo $win/lib ;;
  QT_INSTALL_LIBEXECS|QT_INSTALL_BINS) echo $win/bin ;;
  QT_INSTALL_PLUGINS) echo $win/plugins ;;
  QT_INSTALL_QML) echo $win/qml ;;
  QT_INSTALL_TRANSLATIONS) echo $win/translations ;;
  QT_HOST_PREFIX|QT_HOST_DATA) echo $DEPS/qt/$QT_VERSION/gcc_64 ;;
  QT_HOST_BINS|QT_HOST_BINS/get) echo $DEPS/qt/$QT_VERSION/gcc_64/bin ;;
  QT_HOST_LIBEXECS|QT_HOST_LIBEXECS/get) echo $DEPS/qt-host-tools ;;
  QT_HOST_LIBS) echo $DEPS/qt/$QT_VERSION/gcc_64/lib ;;
  QMAKE_SPEC|QMAKE_XSPEC) echo win32-msvc ;;
  QMAKE_VERSION) echo 3.1 ;;
  *) echo "" ;;
esac
EOF
chmod +x bin/qmake-msvc-cross

cd "$DEPS/win"
# libmpv (shinchiro build of mpv master, VapourSynth loaded at run time)
# and an MSVC import library generated from its exports.
if [ ! -e mpv-dev/libmpv-2.dll ]; then
    gh release download "$MPV_RELEASE" -R shinchiro/mpv-winbuild-cmake -p "$MPV_ARCHIVE" --clobber
    "$DEPS/bin/7zz" x -y -ompv-dev "$MPV_ARCHIVE" > /dev/null
fi
mkdir -p mpv-lib
{
    echo "LIBRARY libmpv-2.dll"
    echo "EXPORTS"
    llvm-readobj-18 --coff-exports mpv-dev/libmpv-2.dll | awk '/Name: /{print "    "$2}'
} > mpv-lib/mpv.def
"$DEPS/bin/llvm-dlltool" -m i386:x86-64 -d mpv-lib/mpv.def -D libmpv-2.dll -l mpv-lib/mpv.lib

# The VapourSynth runtime SVP 4 uses: R73 portable, embeddable Python 3.12,
# plus mvtools for PotPlayer's motion interpolation script.
[ -e "VapourSynth64-Portable-$VS_TAG.zip" ] ||
    gh release download "$VS_TAG" -R vapoursynth/vapoursynth -p "VapourSynth64-Portable-$VS_TAG.zip" --clobber
[ -e "python-$PYTHON_VERSION-embed-amd64.zip" ] ||
    curl -fsSLo "python-$PYTHON_VERSION-embed-amd64.zip" \
        "https://www.python.org/ftp/python/$PYTHON_VERSION/python-$PYTHON_VERSION-embed-amd64.zip"
if [ ! -e mvtools/libmvtools.dll ]; then
    gh release download "$MVTOOLS_TAG" -R dubhater/vapoursynth-mvtools \
        -p "vapoursynth-mvtools-$MVTOOLS_TAG-win64.7z" --clobber
    "$DEPS/bin/7zz" x -y -omvtools "vapoursynth-mvtools-$MVTOOLS_TAG-win64.7z" > /dev/null
fi
rm -rf vs-runtime && mkdir vs-runtime
"$DEPS/venv/bin/python" - "$VS_TAG" "$PYTHON_VERSION" << 'EOF'
import os, shutil, sys, zipfile
tag, python = sys.argv[1], sys.argv[2]
os.chdir("vs-runtime")
zipfile.ZipFile(f"../python-{python}-embed-amd64.zip").extractall(".")
with open("python312._pth", "a", newline="\r\n") as pth:
    pth.write("vs-scripts\nLib\\site-packages\n")
zipfile.ZipFile(f"../VapourSynth64-Portable-{tag}.zip").extractall(".")
os.remove("VSScriptPython38.dll")
os.makedirs("Lib/site-packages", exist_ok=True)
os.makedirs("vs-scripts", exist_ok=True)
wheel = [w for w in os.listdir("wheel") if "abi3" in w][0]
zipfile.ZipFile(f"wheel/{wheel}").extractall("Lib/site-packages")
# pip would install the core library from the wheel's data directory.
data = f"Lib/site-packages/vapoursynth-{tag[1:]}.data/data/Lib/site-packages/vapoursynth.dll"
shutil.move(data, "Lib/site-packages/vapoursynth.dll")
shutil.rmtree(f"Lib/site-packages/vapoursynth-{tag[1:]}.data")
shutil.copy("../mvtools/libmvtools.dll", "vs-plugins/")
EOF
echo "Windows dependencies ready in $DEPS"
