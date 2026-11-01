#!/usr/bin/env bash
# Builds whisper.cpp's command line (whisper-cli) for Linux x86_64 (goal S),
# the Linux side of tools/whisper/build.sh: one build serves the CPU and
# Vulkan engines, ggml loading its backends at run time (GGML_BACKEND_DL),
# the CPU's for each instruction set and Vulkan's. whisper.cpp's releases
# have no Linux build. Vulkan's headers are the Linux kit's (its SDK,
# Vulkan-Headers at the Windows build's version; Ubuntu 22.04's are older
# than ggml wants), linked against the system's loader, and ggml's shaders
# are compiled with the SDK's glslc (shaderc at the commit of the Windows
# build's; Ubuntu 22.04 has no glslc package). The libraries find each
# other beside whisper-cli (`$ORIGIN`).
#
# Runs on the system the Linux packages are built on (Ubuntu 22.04, the
# kit's `whisper` step). The result goes to OUT (default
# WORK/whisper-out/whisper): whisper-cli with its libraries and whisper.cpp's
# licence, never committed; tools/packaging/package.py copies the folder
# into the Linux packages (whisper/ beside rvp).
#
# usage: tools/whisper/build-linux.sh SDK [WORK [OUT]]
#   SDK   the kit's SDK folder (include/vulkan from Vulkan-Headers,
#         bin/glslc from shaderc)
set -euo pipefail

TAG=v1.9.5
COMMIT=d1be6fde11ac6e0407606b4e42fe72d34add8037
# ggml's Vulkan backend wants SPIRV-Headers' CMake package: the revision
# the SDK's shaderc builds with.
SPIRV_HEADERS_COMMIT=86f980c731e62ae4eaf383d320449d71687936bf
SDK=${1:?usage: build-linux.sh SDK [WORK [OUT]]}
WORK=${2:-$HOME/.cache/rvp-whisper}
OUT=${3:-$WORK/whisper-out/whisper}
JOBS=$(nproc)
mkdir -p "$WORK"
[ -x "$SDK/bin/glslc" ]

# kit_clone (RVP_KIT_SOURCES, the kit's source cache).
. "$(cd "$(dirname "$0")/../linux-kit" && pwd)/sources.sh"
SRC=$WORK/whisper.cpp
rm -rf "$SRC"
kit_clone https://github.com/ggml-org/whisper.cpp.git "$SRC" "$COMMIT"
[ "$(git -C "$SRC" rev-parse HEAD)" = "$COMMIT" ]

rm -rf "$WORK/spirv-headers" "$WORK/spirv-prefix"
kit_clone https://github.com/KhronosGroup/SPIRV-Headers.git "$WORK/spirv-headers" "$SPIRV_HEADERS_COMMIT"
cmake -Wno-dev -G Ninja -S "$WORK/spirv-headers" -B "$WORK/spirv-headers/build" \
    -DCMAKE_INSTALL_PREFIX="$WORK/spirv-prefix" -DSPIRV_HEADERS_ENABLE_TESTS=OFF
cmake --install "$WORK/spirv-headers/build"

BUILD=$WORK/whisper-build
loader=$(ls /usr/lib/x86_64-linux-gnu/libvulkan.so 2> /dev/null || ls /usr/lib/libvulkan.so)
cmake -Wno-dev -G Ninja -S "$SRC" -B "$BUILD" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_PREFIX_PATH="$WORK/spirv-prefix" \
    -DBUILD_SHARED_LIBS=ON \
    -DGGML_NATIVE=OFF -DGGML_BACKEND_DL=ON -DGGML_CPU_ALL_VARIANTS=ON -DGGML_OPENMP=OFF \
    -DGGML_VULKAN=ON \
    -DVulkan_INCLUDE_DIR="$SDK/include" \
    -DVulkan_LIBRARY="$loader" \
    -DVulkan_GLSLC_EXECUTABLE="$SDK/bin/glslc" \
    -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF -DWHISPER_SDL2=OFF -DWHISPER_CURL=OFF \
    -DCMAKE_BUILD_RPATH='$ORIGIN' -DCMAKE_INSTALL_RPATH='$ORIGIN' -DCMAKE_BUILD_WITH_INSTALL_RPATH=ON
ninja -C "$BUILD" -j "$JOBS" whisper-cli

rm -rf "$OUT"
mkdir -p "$OUT"
cp "$BUILD/bin/whisper-cli" "$OUT/"
# libwhisper, libggml and ggml's backends (each instruction set's CPU one,
# Vulkan's), with their version links.
find "$BUILD" -name '*.so*' \( -type f -o -type l \) -exec cp -P {} "$OUT/" \;
cp "$SRC/LICENSE" "$OUT/LICENSE-whisper.cpp.txt"
printf 'whisper.cpp %s (%s), built by tools/whisper/build-linux.sh\n' "$TAG" "$COMMIT" > "$OUT/VERSION.txt"
ls -la "$OUT"
