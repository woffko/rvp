#!/usr/bin/env bash
# Collects the licence files of everything tools/libmpv/build.sh built into
# libmpv-2.dll (shinchiro's mpv-winbuild-cmake: each package that finished
# installing, and the toolchain's runtimes linked into it, LLVM's libc++
# and mingw-w64's), one folder per component, for
# `tools/packaging/package.py windows --licences OUT` (THIRD-PARTY.txt).
# Never libdvdcss or libaacs: RVP does not ship them.
#
# usage: tools/libmpv/licences.sh [WORK] OUT   (WORK: default ~/rvp-deps/mpv-build)
set -euo pipefail
if [ "$#" -eq 1 ]; then
    work=$HOME/rvp-deps/mpv-build
    out=$1
else
    work=$1
    out=$2
fi
sources=$work/src_packages
packages=$work/build64/packages
rm -rf "$out"
mkdir -p "$out"
# Packages of headers only state their licence in the headers; opus-dnn is
# Opus's model data.
stated_licence() {
    case "$1" in
    amf-headers | nvcodec-headers) echo "MIT (stated in the headers)" ;;
    angle-headers) echo "BSD-3-Clause (stated in the headers)" ;;
    avisynth-headers) echo "GPL-2.0-or-later with AviSynth's linking exception (stated in the headers)" ;;
    vapoursynth) echo "LGPL-2.1-or-later (VapourSynth's headers; stated in them)" ;;
    opus-dnn) echo "BSD-3-Clause (Opus's neural network data, under Opus's licence: see opus)" ;;
    esac
}
copy() {
    local name=$1 source=$2 file found=0 archive member
    # A package kept as its release archive: the licence files at the
    # archive's top.
    for archive in "$source"/*.tar.gz "$source"/*.tar.xz "$source"/*.tar.bz2; do
        [ -f "$archive" ] || continue
        while read -r member; do
            case "$(basename "$member" | tr '[:upper:]' '[:lower:]')" in
            licen[cs]e* | copying* | copyright* | notice*)
                mkdir -p "$out/$name"
                tar -xaf "$archive" -O "$member" > "$out/$name/$(basename "$member")"
                found=1
                ;;
            esac
        done < <(tar -taf "$archive" | grep -E '^[^/]+/[^/]+$')
    done
    if [ -n "$(stated_licence "$name")" ]; then
        mkdir -p "$out/$name"
        stated_licence "$name" > "$out/$name/LICENSE-NOTE.txt"
        found=1
    fi
    for file in "$source"/*; do
        [ -f "$file" ] || continue
        case "$(basename "$file" | tr '[:upper:]' '[:lower:]')" in
        licen[cs]e* | copying* | copyright* | notice* | unlicense* | patents*)
            mkdir -p "$out/$name"
            cp "$file" "$out/$name/"
            found=1
            ;;
        esac
    done
    [ "$found" -eq 1 ] || echo "no licence file in $source" >&2
}
for prefix in "$packages"/*-prefix; do
    name=$(basename "$prefix" -prefix)
    case "$name" in
    *dvdcss* | *aacs* | mpv-packaging) continue ;;
    esac
    stamp=$prefix/src/$name-stamp
    [ -e "$stamp/$name-install" ] || [ -e "$stamp/$name-done" ] || continue
    [ -d "$sources/$name" ] && copy "$name" "$sources/$name"
done
for name in llvm mingw-w64; do
    [ -d "$sources/$name" ] && copy "$name" "$sources/$name"
done
echo "licences of $(find "$out" -mindepth 1 -maxdepth 1 -type d | wc -l) components in $out"
