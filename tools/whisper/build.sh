#!/usr/bin/env bash
# Builds whisper.cpp's command line (whisper-cli) for Windows x86_64: the
# engine of RVP's subtitles made from the audio (M13, goal O). One build
# serves the CPU and Vulkan engines: ggml loads its backends at run time
# (GGML_BACKEND_DL), the CPU's for each instruction set and Vulkan's, and
# RVP's -ng keeps a job on the CPU. whisper.cpp's releases have no Vulkan
# build for Windows, so this one is made with the toolchain and libraries
# of tools/libmpv/build.sh's work folder (the recipe's clang for MinGW, its
# Vulkan headers and loader import library), and ggml's Vulkan shaders are
# compiled with a glslc built for the build host from the recipe's own
# shaderc, SPIRV-Tools, SPIRV-Headers and glslang sources. The recipe's
# libc++ includes less on its own than whisper.cpp's examples expect
# (std::partial_sort without <algorithm>), so every C++ file gets it.
#
# Every program and DLL gets a version resource (tools/whisper/version.cmake:
# the host's antivirus quarantined a whisper-cli.exe without one).
#
# Runs where tools/libmpv/build.sh ran (inside the build root of
# tools/libmpv/rootless.sh: `tools/libmpv/rootless.sh ROOT WORK
# tools/whisper/build.sh`), after it. The result goes to OUT (default
# WORK/whisper-out/whisper): whisper-cli.exe with its DLLs and whisper.cpp's
# licence, never committed; tools/packaging/package.py copies the folder
# into the Windows package (whisper/ next to rvp.exe).
#
# usage: tools/whisper/build.sh [WORK [OUT]]    (default ~/rvp-deps/mpv-build)
set -euo pipefail

TAG=v1.9.5
COMMIT=d1be6fde11ac6e0407606b4e42fe72d34add8037
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=${1:-$HOME/rvp-deps/mpv-build}
OUT=${2:-$WORK/whisper-out/whisper}
SYSROOT=$WORK/build64/x86_64-w64-mingw32
HOST=$WORK/host-tools
export PATH="$WORK/clang_root/bin:$HOST/bin:$PATH"
# The recipe's compiler wrappers: no link-time optimisation for these
# DLLs, ccache in the work folder.
export LTO=0
export CCACHE_DIR=${CCACHE_DIR:-$WORK/ccache}

# glslc for the build host.
if [ ! -x "$HOST/bin/glslc" ]; then
    cmake -Wno-dev --fresh -G Ninja -S "$WORK/src_packages/shaderc" -B "$WORK/host-shaderc" \
        -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER=/usr/bin/gcc -DCMAKE_CXX_COMPILER=/usr/bin/g++ \
        -DSHADERC_SKIP_TESTS=ON -DSHADERC_SKIP_EXAMPLES=ON -DSHADERC_SKIP_COPYRIGHT_CHECK=ON \
        -DSHADERC_SKIP_SPVC=ON -DSPIRV_SKIP_TESTS=ON -DSPIRV_SKIP_EXECUTABLES=ON \
        -DENABLE_GLSLANG_BINARIES=OFF -DSPIRV_TOOLS_BUILD_STATIC=ON
    ninja -C "$WORK/host-shaderc" glslc_exe
    mkdir -p "$HOST/bin"
    cp "$WORK/host-shaderc/glslc/glslc" "$HOST/bin/glslc"
fi

SRC=$WORK/whisper.cpp
if [ ! -d "$SRC/.git" ]; then
    git -c advice.detachedHead=false clone -q --depth 1 --branch "$TAG" https://github.com/ggml-org/whisper.cpp.git "$SRC"
fi
git -C "$SRC" fetch -q --depth 1 origin "refs/tags/$TAG:refs/tags/$TAG"
git -C "$SRC" -c advice.detachedHead=false checkout -q --force "$COMMIT"
[ "$(git -C "$SRC" rev-parse HEAD)" = "$COMMIT" ]

# ggml builds its shader generator for the build host.
BUILD=$WORK/whisper-build
mkdir -p "$BUILD"
cat > "$BUILD/host.cmake" <<HOSTEOF
set(CMAKE_C_COMPILER /usr/bin/gcc)
set(CMAKE_CXX_COMPILER /usr/bin/g++)
HOSTEOF

cmake -Wno-dev --fresh -G Ninja -S "$SRC" -B "$BUILD" \
    -DCMAKE_TOOLCHAIN_FILE="$WORK/build64/toolchain.cmake" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=ON \
    -DGGML_NATIVE=OFF -DGGML_BACKEND_DL=ON -DGGML_CPU_ALL_VARIANTS=ON -DGGML_OPENMP=OFF \
    -DGGML_VULKAN=ON \
    -DGGML_VULKAN_SHADERS_GEN_TOOLCHAIN="$BUILD/host.cmake" \
    -DVulkan_INCLUDE_DIR="$SYSROOT/include" \
    -DVulkan_LIBRARY="$SYSROOT/lib/libvulkan-1.dll.a" \
    -DVulkan_GLSLC_EXECUTABLE="$HOST/bin/glslc" \
    -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF -DWHISPER_SDL2=OFF -DWHISPER_CURL=OFF \
    -DCMAKE_EXE_LINKER_FLAGS="-static-libgcc" -DCMAKE_SHARED_LINKER_FLAGS="-static-libgcc" \
    -DCMAKE_CXX_FLAGS="-include algorithm" \
    -DCMAKE_PROJECT_whisper.cpp_INCLUDE="$HERE/version.cmake" \
    -DRVP_WHISPER_TAG="$TAG" -DRVP_WHISPER_COMMIT="$COMMIT" \
    -DRVP_WHISPER_COPYRIGHT="$(grep -m1 '^Copyright' "$SRC/LICENSE")"
ninja -C "$BUILD" whisper-cli

rm -rf "$OUT"
mkdir -p "$OUT"
cp "$BUILD/bin/whisper-cli.exe" "$OUT/"
find "$BUILD/bin" -maxdepth 1 -name '*.dll' -exec cp {} "$OUT/" \;
cp "$SRC/LICENSE" "$OUT/LICENSE-whisper.cpp.txt"
printf 'whisper.cpp %s (%s), built by tools/whisper/build.sh\n' "$TAG" "$COMMIT" > "$OUT/VERSION.txt"
# Each file carries its version resource (its UTF-16 text without the
# zero bytes reads as ASCII).
for file in "$OUT"/*.exe "$OUT"/*.dll; do
    if ! tr -d '\0' < "$file" | grep -q -a "OriginalFilename$(basename "$file")"; then
        echo "no version resource in $file" >&2
        exit 1
    fi
done
ls -la "$OUT"
