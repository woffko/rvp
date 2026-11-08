#!/usr/bin/env bash
# Builds vs-mlrt's TensorRT plugin (libvstrt.so) and its trtexec for Linux
# (M18, RIFE (TensorRT); docs/spikes/m18-tensorrt.md): vs-mlrt publishes no
# Linux build. Windows' come from vs-mlrt's release
# (tools/packaging/vapoursynth.py).
#
# - vs-mlrt v15.15's vstrt, against TensorRT 10.15's and CUDA 13.2's
#   headers from NVIDIA's Ubuntu 22.04 packages (unpacked here, never
#   installed). It links CUDA's static runtime and a TensorRT library of
#   no content that only names libnvinfer.so.10: NVIDIA's own one, which
#   RVP downloads on the user's request (crates/app/src/tensorrt.rs),
#   comes in at run time from vsmlrt-cuda/ next to the plugin (its run
#   path), and gives the symbols.
# - trtexec from NVIDIA's TensorRT samples (Apache-2.0, v10.15) with
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
# Not TensorRT 10.16: its engines computed RIFE wrongly on an RTX 50-series card
# (crates/app/src/tensorrt.rs).
VSMLRT_TAG=v15.15
TRT_OSS_TAG=v10.15
# vstrt is a plugin of VapourSynth's API 3 (VapourSynth.h), whose headers
# vs-mlrt builds with; RVP's R73 runtime loads such plugins.
VS_TAG=R57
REPO=https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64
# NVIDIA's packages: file and SHA-256 (the repository's Packages index).
DEBS=(
    "libnvinfer-headers-dev_10.15.1.29-1+cuda13.1_amd64.deb bf5b93ef3a76eedf1917b7e62240785de1682e7504b55f5f8b22e4dccf809faf"
    "libnvinfer-headers-plugin-dev_10.15.1.29-1+cuda13.1_amd64.deb f5dcdb154835d4f139a495999a89a129b329a2ffbd89b2a1d8d1720cd35b2750"
    "libnvonnxparsers-dev_10.15.1.29-1+cuda13.1_amd64.deb 98d5abbb97483e29f33333b0c1443a483e39500e125accf21a174b5ee147faba"
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
# One clone per tag: a new tag never builds an older checkout.
VSMLRT=$WORK/vs-mlrt-$VSMLRT_TAG
TRT_OSS=$WORK/tensorrt-oss-$TRT_OSS_TAG
[ -d "$VSMLRT/.git" ] || kit_clone https://github.com/AmusementClub/vs-mlrt.git "$VSMLRT" "$VSMLRT_TAG"
[ -d "$TRT_OSS/.git" ] || kit_clone https://github.com/NVIDIA/TensorRT.git "$TRT_OSS" "$TRT_OSS_TAG"

# vstrt: CUDA's static runtime, NVIDIA's libnvinfer.so.10 from
# vsmlrt-cuda/ next to it.
cmake -S "$VSMLRT/vstrt" -B "$WORK/vstrt-build" -G Ninja --fresh \
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
SAMPLES=$TRT_OSS/samples
if [ ! -f "$TRT_OSS/.rvp-patched" ]; then
    cp "$VSMLRT/vstrt/trtexec/CMakeLists.txt" "$VSMLRT/vstrt/trtexec/"*.cpp "$SAMPLES/trtexec/"
    git -C "$TRT_OSS" apply "$VSMLRT/vstrt/trtexec/trtexec.patch"
    sed -i 's/fp16 || bf16 || int8 || fp8 || int4 || best/0/g' "$SAMPLES/common/sampleOptions.cpp"
    touch "$TRT_OSS/.rvp-patched"
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
