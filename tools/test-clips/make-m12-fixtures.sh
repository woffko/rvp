#!/usr/bin/env bash
# Generates M12's fixtures in testdata/media (all synthetic: FFmpeg's test
# sources and solid colours):
#
#   fixture-sbs.mkv   2 s, 192x72 side by side: the left eye red, the right
#                     eye blue, each with a white mark at its top left;
#                     lossless H.264 4:4:4 and Matroska's stereo mode
#                     left_right (mpv reports it as sbs2l)
#   fixture-360.mkv   2 s, 256x128 equirectangular test pattern (testsrc2)
#
# usage: tools/test-clips/make-m12-fixtures.sh [FFMPEG]
set -euo pipefail
FFMPEG=${1:-ffmpeg}
OUT=$(cd "$(dirname "$0")/../.." && pwd)/testdata/media
mkdir -p "$OUT"

"$FFMPEG" -hide_banner -nostdin -loglevel error -y \
    -f lavfi -i "color=red:s=96x72:r=10:d=2,drawbox=x=4:y=4:w=12:h=8:color=white:t=fill[l];\
color=blue:s=96x72:r=10:d=2,drawbox=x=4:y=4:w=12:h=8:color=white:t=fill[r];[l][r]hstack" \
    -c:v libx264 -preset veryslow -qp 0 -pix_fmt yuv444p -metadata:s:v:0 stereo_mode=left_right \
    -fflags +bitexact -metadata title="RVP side by side fixture" "$OUT/fixture-sbs.mkv"

"$FFMPEG" -hide_banner -nostdin -loglevel error -y \
    -f lavfi -i "testsrc2=size=256x128:rate=10:duration=2" \
    -c:v libx264 -preset veryslow -crf 30 -pix_fmt yuv420p \
    -fflags +bitexact -metadata title="RVP 360 fixture" "$OUT/fixture-360.mkv"

ls -l "$OUT/fixture-sbs.mkv" "$OUT/fixture-360.mkv"
