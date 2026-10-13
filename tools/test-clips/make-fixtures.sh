#!/usr/bin/env bash
# Generates the small media fixtures in testdata/media that the engine and
# app tests play (headless libmpv). They are synthetic (testsrc2 and a sine
# tone) and kept tiny; regenerate with this script if they are lost.
#
# usage: tools/test-clips/make-fixtures.sh [FFMPEG]
set -euo pipefail
FFMPEG=${1:-ffmpeg}
OUT=$(cd "$(dirname "$0")/../.." && pwd)/testdata/media
mkdir -p "$OUT"
make() {
    local name=$1 seconds=$2
    "$FFMPEG" -hide_banner -nostdin -loglevel error -y \
        -f lavfi -i "testsrc2=size=96x72:rate=10:duration=$seconds" \
        -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=$seconds" \
        -c:v libx264 -preset veryslow -crf 51 -g 10 -pix_fmt yuv420p \
        -c:a libopus -b:a 6k -ac 1 -metadata title="RVP fixture $seconds s" "$OUT/$name"
}
make fixture-60s.mkv 60
make fixture-3s.mkv 3
# The self-test's orientation check (`rvp --selftest=upright`): four
# coloured quadrants and a white mark at the top left, 160x90 at 24 fps,
# and ffmpeg's decoding of its first frame as the reference.
"$FFMPEG" -hide_banner -nostdin -loglevel error -y \
    -filter_complex "color=c=0xDC2828:s=80x46:r=24:d=10[a];color=c=0x28C83C:s=80x46:r=24:d=10[b];\
color=c=0x2846DC:s=80x46:r=24:d=10[c];color=c=0xE6D228:s=80x46:r=24:d=10[d];\
[a][b][c][d]xstack=inputs=4:layout=0_0|w0_0|0_h0|w0_h0,crop=160:90:0:1,\
drawbox=x=8:y=6:w=20:h=12:color=white:t=fill,format=yuv420p[v];\
sine=frequency=440:sample_rate=48000:duration=10[s]" \
    -map "[v]" -map "[s]" -c:v libx264 -preset veryslow -crf 28 -g 24 \
    -c:a libopus -b:a 6k -ac 1 -metadata title="RVP orientation fixture" "$OUT/fixture-marker.mkv"
"$FFMPEG" -hide_banner -nostdin -loglevel error -y -i "$OUT/fixture-marker.mkv" \
    -frames:v 1 "$OUT/fixture-marker.png"
ls -l "$OUT"
