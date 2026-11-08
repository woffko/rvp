#!/usr/bin/env bash
# Builds RVP's own VapourSynth runtime for Linux (goal S), the Linux side
# of tools/packaging/vapoursynth.py: the Linux packages carry it in
# `vapoursynth/` beside rvp, and RVP points libmpv at its
# libvapoursynth-script.so.0 (never SVP's; rvp_app::svp::vsscript_candidates).
#
# - Python 3.12 from python-build-standalone (relocatable, as Windows'
#   embeddable Python) in `python/`;
# - VapourSynth R73 built here (it has no Linux binaries), with zimg
#   statically, and VSScript patched to start the Python beside it
#   (tools/vapoursynth/vsscript-own-python.patch: nothing in the
#   environment changes for the programs RVP starts);
# - MVTools in `vs-plugins` (the kit's build, the version of the Windows
#   bundle), with FFTW;
# - RIFE's ncnn-Vulkan plugin (its Linux release) with model v4.6 in
#   `rife/`, with GCC's OpenMP runtime;
# - vs-mlrt v15.15's TensorRT plugin and trtexec, built here
#   (tools/vapoursynth/build-vstrt-linux.sh: vs-mlrt has no Linux build),
#   with vsmlrt.py and RIFE v4.6's ONNX model, in `vs-mlrt/` (not loaded
#   on its own): RIFE (TensorRT) copies them next to NVIDIA's TensorRT,
#   which RVP downloads only on the user's request.
#
# The libraries find each other by their run paths ($ORIGIN, set with
# patchelf). Every download is checked by its SHA-256. The result goes to
# OUT, made anew; NOTICE.txt there names each part and its licence. Runs on
# the system the Linux packages are built on (Ubuntu 22.04, the kit's
# `vapoursynth` step).
#
# usage: tools/vapoursynth/build-linux.sh MVTOOLS_SO WORK OUT
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
# kit_clone (RVP_KIT_SOURCES, the kit's source cache).
. "$HERE/../linux-kit/sources.sh"
MVTOOLS=${1:?usage: build-linux.sh MVTOOLS_SO WORK OUT}
WORK=${2:?usage: build-linux.sh MVTOOLS_SO WORK OUT}
OUT=${3:?usage: build-linux.sh MVTOOLS_SO WORK OUT}
JOBS=$(nproc)

VS_TAG=R73
ZIMG_TAG=release-3.0.6
CYTHON=3.1.4
# Unpacks vs-mlrt's 7z model archive at build time only.
PY7ZR=1.1.4
VSMLRT_TAG=v15.15
RIFE_ONNX_URL=https://github.com/AmusementClub/vs-mlrt/releases/download/model-20220923/rife_v8.7z
RIFE_ONNX_SHA256=1a18614e72164c61d60861199fc8be6b0003110f747646b3dafb9acd632f6ef7
PYTHON=cpython-3.12.15+20261021-x86_64-unknown-linux-gnu-install_only.tar.gz
PYTHON_URL=https://github.com/astral-sh/python-build-standalone/releases/download/20261021/$PYTHON
PYTHON_SHA256=5ff9f0602ce964a529ed1e8555d3abab6341926d724402ba9a15e9e877191124
RIFE_TAG=r9_mod_v33
RIFE_URL=https://github.com/styler00dollar/VapourSynth-RIFE-ncnn-Vulkan/releases/download/$RIFE_TAG/librife_linux_x86-64.so
RIFE_SHA256=18a00a5e3ac90a5dfcfdf85fdb81d977381a5a2560a4c45dbe9a469d007b1886
RIFE_RAW=https://raw.githubusercontent.com/styler00dollar/VapourSynth-RIFE-ncnn-Vulkan/$RIFE_TAG
MODEL=rife-v4.6_ensembleFalse
FLOWNET_BIN_SHA256=03393cf14fe6d0af015d24d93bae1e5925f2828cb3b6555300a132643c617aa5
FLOWNET_PARAM_SHA256=ae9b08af43fba97e27aa2c40c04c35b1a75f637b2d2b01a941fa94361ab25cbd
RIFE_LICENSE_SHA256=d37437f9a9276c5b5f966a4a314c8b08128b06d3fa5b7b5716b8db489dc4136a

mkdir -p "$WORK/cache"

# fetch URL NAME SHA256: the file in the cache, downloaded when missing.
fetch() {
    local target=$WORK/cache/$2
    if [ ! -s "$target" ] || [ "$(sha256sum "$target" | cut -d' ' -f1)" != "$3" ]; then
        curl -fsSL --retry 3 -o "$target.part" "$1"
        if [ "$(sha256sum "$target.part" | cut -d' ' -f1)" != "$3" ]; then
            echo "$1: unexpected SHA-256" >&2
            exit 1
        fi
        mv "$target.part" "$target"
    fi
    echo "$target"
}

rm -rf "$OUT"
mkdir -p "$OUT/vs-plugins" "$OUT/rife/models/$MODEL"
tar -xzf "$(fetch "$PYTHON_URL" "$PYTHON" "$PYTHON_SHA256")" -C "$OUT"
PY=$OUT/python/bin/python3
"$PY" --version
# The relocatable Python's pkg-config files name its build's prefix: they
# name this one, so meson links VSScript with its libpython.
sed -i "s|^prefix=.*|prefix=$OUT/python|" "$OUT"/python/lib/pkgconfig/python-3*.pc

# Cython for the build only: in a virtual environment of its own.
if [ ! -x "$WORK/build-venv/bin/cython" ]; then
    "$PY" -m venv "$WORK/build-venv"
    "$WORK/build-venv/bin/pip" install -q "cython==$CYTHON" meson ninja
fi
export PATH=$WORK/build-venv/bin:$PATH
if [ ! -x "$WORK/build-venv/bin/py7zr" ]; then
    "$WORK/build-venv/bin/pip" install -q "py7zr==$PY7ZR"
fi

# zimg, static, for VapourSynth's core.
ZIMG=$WORK/zimg-prefix
if [ "$(cat "$ZIMG/.tag" 2> /dev/null)" != "$ZIMG_TAG" ]; then
    rm -rf "$WORK/zimg" "$ZIMG"
    # Its one submodule is its tests' googletest.
    kit_clone https://github.com/sekrit-twc/zimg.git "$WORK/zimg" "$ZIMG_TAG"
    (cd "$WORK/zimg" && ./autogen.sh && ./configure --prefix="$ZIMG" --enable-static --disable-shared --with-pic &&
        make -j "$JOBS" && make install) > "$WORK/zimg.log" 2>&1
    echo "$ZIMG_TAG" > "$ZIMG/.tag"
fi

# VapourSynth, its Python module installed into the runtime's Python.
SRC=$WORK/vapoursynth
rm -rf "$SRC" "$WORK/vs-build"
kit_clone https://github.com/vapoursynth/vapoursynth.git "$SRC" "$VS_TAG"
git -C "$SRC" apply "$HERE/vsscript-own-python.patch"
PKG_CONFIG_PATH=$ZIMG/lib/pkgconfig:$OUT/python/lib/pkgconfig meson setup "$WORK/vs-build" "$SRC" --prefix="$OUT/python" --libdir=lib \
    --buildtype=release -Denable_vspipe=false "-Dpython3_bin=$PY" -Dplugindir="$OUT/vs-plugins"
ninja -C "$WORK/vs-build" -j "$JOBS" install
# The core and VSScript beside each other at the runtime's top, where RVP
# looks for VSScript; the Python module finds the core there.
mv "$OUT"/python/lib/libvapoursynth.so* "$OUT"/python/lib/libvapoursynth-script.so* "$OUT/"
rm -rf "$OUT/python/include/vapoursynth" "$OUT/python/lib/pkgconfig/vapoursynth"*.pc
module=$(find "$OUT/python/lib" -name 'vapoursynth*.so' -path '*site-packages*' | head -1)
[ -n "$module" ]
vsscript=$(readlink -f "$OUT/libvapoursynth-script.so")
patchelf --set-rpath "\$ORIGIN:\$ORIGIN/python/lib" "$vsscript"
patchelf --set-rpath "\$ORIGIN/$(realpath --relative-to="$(dirname "$module")" "$OUT")" "$module"
# RVP looks for libvapoursynth-script.so.0 (the name VapourSynth's other
# builds give it); meson names it by VSScript's API version.
[ -e "$OUT/libvapoursynth-script.so.0" ] || ln -s "$(basename "$vsscript")" "$OUT/libvapoursynth-script.so.0"

# MVTools with the FFTW it needs.
cp "$MVTOOLS" "$OUT/vs-plugins/libmvtools.so"
fftw=$(ldd "$MVTOOLS" | sed -n 's/.*libfftw3f\.so\.3 => \([^ ]*\).*/\1/p')
if [ -n "$fftw" ]; then
    cp -L "$fftw" "$OUT/vs-plugins/libfftw3f.so.3"
fi
patchelf --set-rpath "\$ORIGIN" "$OUT/vs-plugins/libmvtools.so"

# RIFE, its model and GCC's OpenMP runtime.
cp "$(fetch "$RIFE_URL" "librife_linux_x86-64-$RIFE_TAG.so" "$RIFE_SHA256")" "$OUT/rife/librife_linux_x86-64.so"
cp "$(fetch "$RIFE_RAW/models/$MODEL/flownet.bin" "$MODEL-flownet.bin" "$FLOWNET_BIN_SHA256")" "$OUT/rife/models/$MODEL/flownet.bin"
cp "$(fetch "$RIFE_RAW/models/$MODEL/flownet.param" "$MODEL-flownet.param" "$FLOWNET_PARAM_SHA256")" "$OUT/rife/models/$MODEL/flownet.param"
cp "$(fetch "$RIFE_RAW/LICENSE" "rife-LICENSE-$RIFE_TAG" "$RIFE_LICENSE_SHA256")" "$OUT/rife/LICENSE"
gomp=$(ldconfig -p | sed -n 's/.*libgomp\.so\.1 (libc6,x86-64) => \(.*\)/\1/p' | head -1)
cp -L "$gomp" "$OUT/rife/libgomp.so.1"
patchelf --set-rpath "\$ORIGIN" "$OUT/rife/librife_linux_x86-64.so"

# vs-mlrt: the plugin and trtexec built here, vsmlrt.py, its licence, and
# RIFE v4.6's model.
"$(dirname "$0")/build-vstrt-linux.sh" "$WORK/vstrt" "$WORK/vstrt-out"
mkdir -p "$OUT/vs-mlrt/models/rife"
cp "$WORK/vstrt-out/libvstrt.so" "$WORK/vstrt-out/trtexec" "$OUT/vs-mlrt/"
cp "$WORK/vstrt/vs-mlrt-$VSMLRT_TAG/scripts/vsmlrt.py" "$WORK/vstrt/vs-mlrt-$VSMLRT_TAG/LICENSE" "$OUT/vs-mlrt/"
"$WORK/build-venv/bin/py7zr" x "$(fetch "$RIFE_ONNX_URL" "vs-mlrt-rife_v8.7z" "$RIFE_ONNX_SHA256")" "$WORK/rife-onnx"
cp "$WORK/rife-onnx/rife/rife_v4.6.onnx" "$OUT/vs-mlrt/models/rife/rife_v4.6.onnx"

# What a player does not need: the Python's own tests, IDLE and Tk, pip
# and its installer, its headers, static library and caches; the libraries
# without their debugging symbols (the Python's carried 100 MB).
P=$OUT/python/lib/python3.12
rm -rf "$P"/test "$P"/idlelib "$P"/tkinter "$P"/turtledemo "$P"/ensurepip "$P"/lib2to3 "$P"/pydoc_data \
    "$P"/config-3.12-x86_64-linux-gnu "$P"/site-packages/pip "$P"/site-packages/pip-*.dist-info \
    "$OUT"/python/lib/tcl* "$OUT"/python/lib/tk* "$OUT"/python/lib/libtcl* "$OUT"/python/lib/libtk* \
    "$OUT"/python/lib/itcl* "$OUT"/python/lib/thread* "$OUT"/python/share "$OUT"/python/include \
    "$OUT"/python/lib/pkgconfig
find "$OUT/python" -name '__pycache__' -type d -prune -exec rm -rf {} +
find "$OUT" -name '*.so*' -type f -exec strip --strip-unneeded {} +

cat > "$OUT/NOTICE.txt" << NOTICE
RVP's VapourSynth runtime for Linux (motion smoothing, goals T and S; never SVP's)

VapourSynth $VS_TAG (built here, VSScript patched)  LGPL-2.1-or-later   https://github.com/vapoursynth/vapoursynth
  with zimg $ZIMG_TAG                          WTFPL               https://github.com/sekrit-twc/zimg
Python 3.12 (python-build-standalone)         PSF License 2.0     https://github.com/astral-sh/python-build-standalone (python/lib/python3.12/LICENSE.txt)
MVTools (vs-plugins)                          GPL-2.0-or-later    https://github.com/dubhater/vapoursynth-mvtools
  with FFTW (vs-plugins)                      GPL-2.0-or-later    https://www.fftw.org/
RIFE ncnn-Vulkan $RIFE_TAG (rife/)           MIT                 https://github.com/styler00dollar/VapourSynth-RIFE-ncnn-Vulkan (rife/LICENSE)
  with ncnn                                   BSD-3-Clause        https://github.com/Tencent/ncnn
  model $MODEL                   MIT                 RIFE, https://github.com/hzwer/Practical-RIFE
  with GCC's OpenMP runtime (libgomp)         GPL-3.0 with the GCC Runtime Library Exception
vs-mlrt $VSMLRT_TAG (vs-mlrt/, built here)      GPL-3.0             https://github.com/AmusementClub/vs-mlrt (vs-mlrt/LICENSE)
  trtexec                                     Apache-2.0          TensorRT's sample, https://github.com/NVIDIA/TensorRT
  models/rife/rife_v4.6.onnx                  MIT                 RIFE, https://github.com/hzwer/Practical-RIFE
NVIDIA's TensorRT is not part of this runtime: RVP downloads it from NVIDIA
on the user's request, under NVIDIA's licence.
NOTICE

# The runtime works on its own and where it is moved to (as the packages
# do): a copy elsewhere imports VapourSynth in its Python, and the core
# loads both plugins by their paths (no bytecode written).
rm -rf "$WORK/moved"
cp -a "$OUT" "$WORK/moved"
PYTHONDONTWRITEBYTECODE=1 "$WORK/moved/python/bin/python3" - "$WORK/moved" << 'CHECK'
import sys
import vapoursynth as vs
out = sys.argv[1]
core = vs.core
# The build's own folder is the compiled-in plugin folder, so MVTools may
# be loaded already; RVP's scripts load it the same way.
if not hasattr(core, "mv"):
    core.std.LoadPlugin(out + "/vs-plugins/libmvtools.so")
core.std.LoadPlugin(out + "/rife/librife_linux_x86-64.so")
assert hasattr(core, "mv") and hasattr(core, "rife"), "plugins missing"
print("VapourSynth", vs.__version__, "with mv and rife")
CHECK
rm -rf "$WORK/moved"
# The Python's programs: VSScript embeds its library.
rm -rf "$OUT/python/bin"
du -sh "$OUT"
