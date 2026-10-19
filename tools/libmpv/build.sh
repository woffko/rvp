#!/usr/bin/env bash
# Builds RVP's libmpv for Windows x86_64 (goal T, D18): shinchiro's
# mpv-winbuild-cmake, the recipe of the libmpv RVP used before, at a fixed
# commit, with FFmpeg given its SPIR-V compiler (libshaderc). Without one
# FFmpeg leaves out every Vulkan compute filter, fruc_vulkan (NVIDIA's
# optical flow, M18's fourth engine) among them; shinchiro's builds have
# none. Everything else is the recipe's own.
#
# Runs on Ubuntu (WSL included) with the packages tools/libmpv/README.md
# lists and meson (pip) on the PATH; the recipe builds its own clang
# toolchain for MinGW. The work folder holds the sources, the toolchain and
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
git apply "$HERE/ffmpeg-libshaderc.patch"

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
ninja -C "$BUILD" mpv
ninja -C "$BUILD" mpv-packaging
ls -l "$BUILD"/mpv-dev-*.7z
