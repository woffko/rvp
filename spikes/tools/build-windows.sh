#!/usr/bin/env bash
# Cross-builds the spikes for Windows and assembles runnable folders:
#   $RVP_DEPS/win/s2-bundle  S2 harness + libmpv + VapourSynth runtime
#   $RVP_DEPS/win/s3-bundle  S3 shell + S1 harness + host-session tool + Qt + libmpv
# Run tools/windows-deps.sh first.
set -euo pipefail

DEPS=${RVP_DEPS:-$HOME/rvp-deps}
SPIKES=$(cd "$(dirname "$0")/.." && pwd)
TARGET=x86_64-pc-windows-msvc
export PATH=$DEPS/bin:$PATH
export QMAKE=$DEPS/bin/qmake-msvc-cross
export MPV_LIB_DIR=$DEPS/win/mpv-lib

cd "$SPIKES"
cargo xwin build --release --target "$TARGET" -p s1-svp-attach -p s2-vapoursynth -p s3-qt-shell -p host-session
out=$SPIKES/target/$TARGET/release
win=$DEPS/win

rm -rf "$win/s2-bundle"
cp -r "$win/vs-runtime" "$win/s2-bundle"
cp "$win/mpv-dev/libmpv-2.dll" "$out/s2-vapoursynth.exe" "$out/s1-svp-attach.exe" "$win/s2-bundle/"

runtime=$win/vs-runtime
python3 "$SPIKES/tools/deploy-windows.py" "$out/s3-qt-shell.exe" "$DEPS/qt/6.10.3/msvc2022_64" \
    "$win/s3-bundle" "$win/mpv-dev/libmpv-2.dll" "$out/s1-svp-attach.exe" \
    "$runtime/msvcp140.dll" "$runtime/msvcp140_1.dll" "$runtime/msvcp140_2.dll" \
    "$runtime/vcruntime140.dll" "$runtime/vcruntime140_1.dll" "$runtime/concrt140.dll"
cp "$out/host-session.exe" "$win/s3-bundle/"
echo "bundles: $win/s2-bundle $win/s3-bundle"
