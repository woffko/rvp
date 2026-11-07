#!/usr/bin/env bash
# Builds vs-mlrt's TensorRT plugin (libvstrt.so) and its trtexec for Linux
# (M18, RIFE (TensorRT); docs/spikes/m18-tensorrt.md): vs-mlrt publishes no
# Linux build. Windows' come from vs-mlrt's release
# (tools/packaging/vapoursynth.py).
#
# - vs-mlrt v15.16's vstrt, against TensorRT 10.16's and CUDA 13.2's
#   headers from NVIDIA's Ubuntu 22.04 packages (unpacked here, never
#   installed). It links CUDA's static runtime and a TensorRT library of
#   no content that only names libnvinfer.so.10: NVIDIA's own one, which
#   RVP downloads on the user's request (crates/app/src/tensorrt.rs),
#   comes in at run time from vsmlrt-cuda/ next to the plugin (its run
#   path), and gives the symbols.
# - trtexec from NVIDIA's TensorRT samples (Apache-2.0, v10.16) with
#   vs-mlrt's changes, as vs-mlrt's Windows release builds it: it loads
#   TensorRT's libraries itself, from its own folder (its run path).
#
# Every download is checked by its SHA-256. The result goes to OUT, made
# anew: libvstrt.so and trtexec.
#
# usage: tools/vapoursynth/build-vstrt-linux.sh WORK OUT
set -euo pipefail

WORK=${1:?usage: build-vstrt-linux.sh WORK OUT}
OUT=${2:?usage: build-vstrt-linux.sh WORK OUT}
VSMLRT_TAG=v15.16
TRT_OSS_TAG=v10.16
# vstrt is a plugin of VapourSynth's API 3 (VapourSynth.h), whose headers
# vs-mlrt builds with; RVP's R73 runtime loads such plugins.
VS_TAG=R57
REPO=https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64
# NVIDIA's packages: file and SHA-256 (the repository's Packages index).
DEBS=(
    "libnvinfer-headers-dev_10.16.0.72-1+cuda13.2_amd64.deb d6838dee392440ffe445f197f318bf434dd6bb2d5e4fe14d02696d558f7e8b9b"
    "libnvinfer-headers-plugin-dev_10.16.0.72-1+cuda13.2_amd64.deb 948d65e990c39fddd5c45e2790fd9d31e59a1272640249d0d3a0538967494467"
    "libnvonnxparsers-dev_10.16.0.72-1+cuda13.2_amd64.deb 25f2de7a7e024947e0bb0c8c39636143633a08a9b618150a99a65ebf75977976"
    "cuda-cudart-13-2_13.2.51-1_amd64.deb 533d2739ea283c82099a315425b0fb4697ede620174233915a10a7a20487a7ea"
    "cuda-cudart-dev-13-2_13.2.51-1_amd64.deb 78ba385f868e3f647d3b534c886f7d557829ae6fd746f2f2bece1d8d4ff0fb69"
    "cuda-crt-13-2_13.2.51-1_amd64.deb 8522a7920f06dccf8db038e140d7b0d84f1197550fd02d756f2dad2c93ba4cf2"
    "cuda-cccl-13-2_13.2.27-1_amd64.deb f1d5b14b3803d6f29ada1276e917795b4832bf4d545a9d63f054eeb08583465f"
    "cuda-nvcc-13-2_13.2.51-1_amd64.deb 1ec2156734233034db996310e49168d9922f83ed469b4c560e62ba3f70153323"
    "cuda-profiler-api-13-2_13.2.20-1_amd64.deb cc94b5a55605ad51105829084242ec61d100b266f2a25902d15e98226cf103e9"
)

mkdir -p "$WORK/downloads"
fetch() { # URL FILE SHA256: the file in WORK/downloads, checked
    local target=$WORK/downloads/$2
    if [ ! -f "$target" ] || ! echo "$3  $target" | sha256sum -c --quiet - 2>/dev/null; then
        curl -fsSL --retry 3 -o "$target.part" "$1"
        echo "$3  $target.part" | sha256sum -c --quiet -
        mv "$target.part" "$target"
    fi
    echo "$target"
}

# NVIDIA's headers, CUDA's static runtime and nvcc (CMake finds CUDA by it).
ROOT=$WORK/nvidia-root
rm -rf "$ROOT"
mkdir -p "$ROOT"
for entry in "${DEBS[@]}"; do
    read -r file sha256 <<< "$entry"
    dpkg-deb -x "$(fetch "$REPO/$file" "$file" "$sha256")" "$ROOT"
done
CUDA=$ROOT/usr/local/cuda-13.2
TRT_INCLUDE=$ROOT/usr/include/x86_64-linux-gnu

# A TensorRT library of no content, named as NVIDIA's.
TRT_HOME=$WORK/trt-home
rm -rf "$TRT_HOME"
mkdir -p "$TRT_HOME/lib"
ln -s "$TRT_INCLUDE" "$TRT_HOME/include"
echo 'void rvp_tensorrt_stub(void) {}' > "$TRT_HOME/stub.c"
cc -shared -fPIC -Wl,-soname,libnvinfer.so.10 -o "$TRT_HOME/lib/libnvinfer.so.10" "$TRT_HOME/stub.c"
ln -s libnvinfer.so.10 "$TRT_HOME/lib/libnvinfer.so"

# VapourSynth's headers.
VS_INCLUDE=$WORK/vs-include
mkdir -p "$VS_INCLUDE"
for header in VapourSynth.h VSHelper.h; do
    curl -fsSL --retry 3 -o "$VS_INCLUDE/$header" \
        "https://raw.githubusercontent.com/vapoursynth/vapoursynth/$VS_TAG/include/$header"
done

# The kit's clone (RVP_KIT_SOURCES, its source cache), once.
. "$(cd "$(dirname "$0")" && pwd)/../linux-kit/sources.sh"
[ -d "$WORK/vs-mlrt/.git" ] || kit_clone https://github.com/AmusementClub/vs-mlrt.git "$WORK/vs-mlrt" "$VSMLRT_TAG"
[ -d "$WORK/tensorrt-oss/.git" ] || kit_clone https://github.com/NVIDIA/TensorRT.git "$WORK/tensorrt-oss" "$TRT_OSS_TAG"

# vstrt: CUDA's static runtime, NVIDIA's libnvinfer.so.10 from
# vsmlrt-cuda/ next to it.
cmake -S "$WORK/vs-mlrt/vstrt" -B "$WORK/vstrt-build" -G Ninja --fresh \
    -D CMAKE_BUILD_TYPE=Release \
    -D VAPOURSYNTH_INCLUDE_DIRECTORY="$VS_INCLUDE" \
    -D TENSORRT_HOME="$TRT_HOME" \
    -D CUDAToolkit_ROOT="$CUDA" \
    -D CMAKE_CXX_FLAGS="-ffast-math -march=x86-64-v2" \
    -D CMAKE_SHARED_LINKER_FLAGS="-Wl,--no-as-needed -Wl,--enable-new-dtags" \
    -D CMAKE_BUILD_WITH_INSTALL_RPATH=ON \
    -D CMAKE_INSTALL_RPATH='$ORIGIN/vsmlrt-cuda'
cmake --build "$WORK/vstrt-build"

# trtexec: NVIDIA's sample with vs-mlrt's changes (vstrt/trtexec).
SAMPLES=$WORK/tensorrt-oss/samples
if [ ! -f "$WORK/tensorrt-oss/.rvp-patched" ]; then
    cp "$WORK/vs-mlrt/vstrt/trtexec/CMakeLists.txt" "$WORK/vs-mlrt/vstrt/trtexec/"*.cpp "$SAMPLES/trtexec/"
    git -C "$WORK/tensorrt-oss" apply "$WORK/vs-mlrt/vstrt/trtexec/trtexec.patch"
    sed -i 's/fp16 || bf16 || int8 || fp8 || int4 || best/0/g' "$SAMPLES/common/sampleOptions.cpp"
    touch "$WORK/tensorrt-oss/.rvp-patched"
fi
cmake -S "$SAMPLES/trtexec" -B "$WORK/trtexec-build" -G Ninja --fresh \
    -D CMAKE_BUILD_TYPE=Release \
    -D CUDAToolkit_ROOT="$CUDA" \
    -D CMAKE_CXX_STANDARD=20 \
    -D CMAKE_CXX_FLAGS="-I$TRT_INCLUDE" \
    -D CMAKE_EXE_LINKER_FLAGS="-Wl,--enable-new-dtags" \
    -D CMAKE_BUILD_WITH_INSTALL_RPATH=ON \
    -D CMAKE_INSTALL_RPATH='$ORIGIN'
cmake --build "$WORK/trtexec-build"

rm -rf "$OUT"
mkdir -p "$OUT"
cp "$WORK/vstrt-build/libvstrt.so" "$OUT/libvstrt.so"
cp "$WORK/trtexec-build/trtexec" "$OUT/trtexec"
# NVIDIA's library by name, from vsmlrt-cuda/ next to the plugin; trtexec
# looks next to itself.
readelf -d "$OUT/libvstrt.so" | grep -q 'NEEDED.*\[libnvinfer.so.10\]'
readelf -d "$OUT/libvstrt.so" | grep -q 'RUNPATH.*\[\$ORIGIN/vsmlrt-cuda\]'
readelf -d "$OUT/trtexec" | grep -q 'RUNPATH.*\[\$ORIGIN\]'
ls -la "$OUT"
