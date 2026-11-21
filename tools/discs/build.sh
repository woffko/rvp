#!/usr/bin/env bash
# Builds the disc libraries RVP's DVD and Blu-ray menus load (M16, goal R)
# as Windows x86_64 DLLs: libdvdnav-4.dll with libdvdread inside (static,
# without libdvdcss: RVP never ships it; libdvdread loads a user's
# libdvdcss-2.dll at run time) and libbluray-4.dll (with its BD-J jars,
# which run with the user's own Java; no libaacs or keys: libbluray loads a
# user's libaacs at run time). Each at a release, built with meson and the
# toolchain of tools/libmpv/build.sh's work folder; libudfread, freetype and
# libxml2 come statically from that build's prefix. libbluray gets RVP's
# change libbluray-crop.patch (beside this script): a menu's crop
# rectangle from the disc is kept inside its object; the build stops when
# the patch does not apply.
#
# Runs where tools/libmpv/build.sh ran (inside the build root of
# tools/libmpv/rootless.sh: `tools/libmpv/rootless.sh ROOT WORK
# tools/discs/build.sh`), after it; it installs a JDK and ant into the
# build root for the jar. The result goes to OUT (default
# WORK/discs-out/discs): the DLLs, the jars and the libraries' licences,
# never committed; tools/packaging/package.py copies them next to rvp.exe.
#
# usage: tools/discs/build.sh [WORK [OUT]]    (default ~/rvp-deps/mpv-build)
set -euo pipefail

WORK=${1:-$HOME/rvp-deps/mpv-build}
OUT=${2:-$WORK/discs-out/discs}
HERE=$(cd "$(dirname "$0")" && pwd)
SYSROOT=$WORK/build64/x86_64-w64-mingw32
SRC=$WORK/discs-src
PREFIX=$WORK/discs-prefix
CROSS=$WORK/build64/meson_cross.txt
export PATH="$WORK/clang_root/bin:$PATH"
export LTO=0
export CCACHE_DIR=${CCACHE_DIR:-$WORK/ccache}

# name repository tag commit: each release by its tag, checked out at the
# tag's commit and checked (a tag can move)
LIBRARIES="
libdvdread https://code.videolan.org/videolan/libdvdread.git 7.1.1 8e43bfea6cc731f48902ce55e41c540b497b6a84
libdvdnav https://code.videolan.org/videolan/libdvdnav.git 7.0.0 38238caf599dc9405eddf1531c858c725015f776
libbluray https://code.videolan.org/videolan/libbluray.git 1.5.1 a24f4fad4d62893de647abc8671397747b2359dd
"

if ! command -v ant > /dev/null || ! command -v javac > /dev/null; then
    apt-get update -q
    DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends \
        openjdk-17-jdk-headless ant
fi

mkdir -p "$SRC"
rm -rf "$PREFIX" "$OUT"
# This build's own libraries first, then the recipe's static ones.
export PKG_CONFIG_PATH=$PREFIX/lib/pkgconfig
export PKG_CONFIG_LIBDIR=$SYSROOT/lib/pkgconfig

# name repository tag commit kind [meson options]
build() {
    local name=$1 url=$2 tag=$3 commit=$4 kind=$5
    shift 5
    if [ ! -d "$SRC/$name" ]; then
        git -c advice.detachedHead=false clone -q --depth 1 --branch "$tag" "$url" "$SRC/$name"
    fi
    git -C "$SRC/$name" -c advice.detachedHead=false checkout -q --force "$commit"
    if [ "$(git -C "$SRC/$name" rev-parse HEAD)" != "$commit" ]; then
        echo "$name: $SRC/$name is not at $commit ($tag)" >&2
        exit 1
    fi
    # The toolchain has no libssp, which libbluray asks for with MinGW
    # (the recipe drops the same line).
    [ "$name" != libbluray ] || sed -i "/find_library('ssp')/d" "$SRC/$name/meson.build"
    if [ "$name" = libbluray ]; then
        git -C "$SRC/$name" apply "$HERE/libbluray-crop.patch"
    fi
    # Inside the source: the build writes its ChangeLog with `git log`.
    local build=$SRC/$name/build-rvp
    rm -rf "$build"
    meson setup "$build" "$SRC/$name" --cross-file="$CROSS" --prefix="$PREFIX" \
        --libdir=lib --buildtype=release --default-library="$kind" --prefer-static \
        -Db_lto=false "$@"
    ninja -C "$build"
    ninja -C "$build" install
}

while read -r name url tag commit; do
    [ -n "$name" ] || continue
    case $name in
        # Static: libdvdnav takes it in (this prefix comes before the
        # recipe's own libdvdread.a). With libdvdnav's GCC layout of bit
        # fields (libdvdnav asks for it, libdvdread does not): MinGW's
        # default gave libdvdnav garbage button rectangles.
        libdvdread) build "$name" "$url" "$tag" "$commit" static -Dlibdvdcss=disabled -Dc_args=-mno-ms-bitfields ;;
        libdvdnav) build "$name" "$url" "$tag" "$commit" shared ;;
        libbluray) build "$name" "$url" "$tag" "$commit" shared -Denable_tools=false -Dbdj_jar=enabled \
            -Dfreetype=enabled -Dlibxml2=enabled -Dfontconfig=disabled ;;
    esac
done <<< "$LIBRARIES"

mkdir -p "$OUT"
cp "$PREFIX"/bin/libdvdnav-4.dll "$PREFIX"/bin/libbluray-4.dll "$OUT/"
cp "$PREFIX"/share/java/*.jar "$OUT/" 2> /dev/null || find "$PREFIX" -name '*.jar' -exec cp {} "$OUT/" \;
for name in libdvdread libdvdnav libbluray; do
    cp "$SRC/$name/COPYING" "$OUT/$name-COPYING.txt"
done
# The DLLs must not hold libdvdcss, nor need a DLL RVP does not ship.
if strings "$OUT"/*.dll | grep -q 'DVDCSS_'; then
    echo "a DLL contains libdvdcss" >&2
    exit 1
fi
if objdump -p "$OUT"/*.dll | grep 'DLL Name' | grep -qiv \
    -e 'KERNEL32' -e 'ADVAPI32' -e 'GDI32' -e 'SHELL32' -e 'USER32' -e 'bcrypt' -e 'api-ms-win-crt-'; then
    echo "a DLL needs another DLL:" >&2
    objdump -p "$OUT"/*.dll | grep 'DLL Name' >&2
    exit 1
fi
ls -l "$OUT"
