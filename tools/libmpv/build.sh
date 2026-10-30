#!/usr/bin/env bash
# Builds RVP's libmpv for Windows x86_64 (goal T, D18): shinchiro's
# mpv-winbuild-cmake, the recipe of the libmpv RVP used before, at a fixed
# commit, with FFmpeg given its SPIR-V compiler: FFmpeg compiles its Vulkan
# shaders while it builds, with a glslang (or glslc) it finds on the PATH,
# and without one leaves out every Vulkan compute filter, fruc_vulkan
# (NVIDIA's optical flow, M18's fourth engine) among them. shinchiro's
# builds have none; this script builds glslang for the build host at a
# release. One more change keeps the recipe building: curl stays at its
# 8.22.0 release, since curl's master since 8.23.0 expects an OpenSSL 4.1
# with ASN1_STRING_get_length, newer than the 4.1.0-dev the recipe pins;
# curl leaves out SSH (libssh's master removed the SCP API curl 8.22.0
# uses; a player needs no SCP or SFTP); FFmpeg's master no longer has the
# OpenGL output device the recipe enables (both goal R). And libdvdread is built without libdvdcss (goal R): RVP never ships it;
# libdvdread loads a user's libdvdcss-2.dll at run time instead. The
# script checks the result for libdvdcss's strings.
# Everything else is the recipe's own, its other packages at their newest.
#
# Runs on Ubuntu (WSL included) with the packages tools/libmpv/README.md
# lists and meson (pip) on the PATH, or without sudo inside the build root
# of tools/libmpv/rootless.sh; the recipe builds its own clang toolchain
# for MinGW. The work folder holds the sources, the toolchain and
# the build (30 to 40 GB); the result is mpv-dev-x86_64-*.7z there (libmpv-2.dll,
# its headers and import library), never committed.
#
# usage: tools/libmpv/build.sh [WORK]    (default ~/rvp-deps/mpv-build)
set -euo pipefail

COMMIT=1f8d2e76255abc48f0456d9be99b4cab9c6566e6
WORK=${1:-$HOME/rvp-deps/mpv-build}
HERE=$(cd "$(dirname "$0")" && pwd)
BUILD=$WORK/build64

mkdir -p "$WORK"
cd "$WORK"
[ -d mpv-winbuild-cmake ] || git clone https://github.com/shinchiro/mpv-winbuild-cmake.git
cd mpv-winbuild-cmake
git fetch -q origin
git checkout -q --force "$COMMIT"
git apply "$HERE/curl-release.patch"
git apply "$HERE/curl-no-libssh.patch"
git apply "$HERE/ffmpeg-no-opengl.patch"
git apply "$HERE/no-libdvdcss.patch"

# glslang for the build host (see above), installed in the work folder.
GLSLANG=16.6.0
HOST=$WORK/host-tools
if [ ! -x "$HOST/bin/glslang" ]; then
    rm -rf "$WORK/host-glslang"
    git -c advice.detachedHead=false clone -q --depth 1 --branch "$GLSLANG" https://github.com/KhronosGroup/glslang.git "$WORK/host-glslang"
    cmake -Wno-dev -G Ninja -S "$WORK/host-glslang" -B "$WORK/host-glslang/build" \
        -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$HOST" \
        -DENABLE_OPT=OFF -DGLSLANG_TESTS=OFF -DBUILD_TESTING=OFF
    ninja -C "$WORK/host-glslang/build" install
fi
export PATH="$HOST/bin:$PATH"

cmake -Wno-dev --fresh -G Ninja -S . -B "$BUILD" \
    -DTARGET_ARCH=x86_64-w64-mingw32 \
    -DCOMPILER_TOOLCHAIN=clang \
    -DCMAKE_INSTALL_PREFIX="$WORK/clang_root" \
    -DMINGW_INSTALL_PREFIX="$BUILD/x86_64-w64-mingw32" \
    -DSINGLE_SOURCE_LOCATION="$WORK/src_packages" \
    -DRUSTUP_LOCATION="$WORK/install_rustup" \
    -DENABLE_CCACHE=ON \
    -DCLANG_PACKAGES_LTO=ON

# Sources first (a few fail to fetch on the first try and come with the
# build), then the toolchain, Rust, and libmpv with everything it needs.
ninja -C "$BUILD" download || true
if [ ! -x "$WORK/clang_root/bin/clang" ]; then
    ninja -C "$BUILD" llvm
    ninja -C "$BUILD" llvm-clang
fi
if [ ! -d "$WORK/install_rustup/.cargo/bin" ]; then
    ninja -C "$BUILD" rustup
fi
ninja -C "$BUILD" update
# A libdvdcss an earlier build installed must not be found again.
rm -rf "$BUILD"/x86_64-w64-mingw32/lib/libdvdcss.* "$BUILD"/x86_64-w64-mingw32/lib/pkgconfig/libdvdcss.pc \
    "$BUILD"/x86_64-w64-mingw32/include/dvdcss
ninja -C "$BUILD" mpv
ninja -C "$BUILD" mpv-packaging
ls -l "$BUILD"/mpv-dev-*.7z
DLL=$(ls -t "$BUILD"/mpv-dev-*/libmpv-2.dll | head -1)
if strings "$DLL" | grep -q 'DVDCSS_'; then
    echo "$DLL contains libdvdcss" >&2
    exit 1
fi
