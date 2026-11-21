#!/usr/bin/env bash
# Generates M18's fixtures in testdata/media (synthetic: solid colours):
#
#   fixture-square.mkv  12 s, 320x240 at 24 fps (288 frames): a 40x40
#                       white square that moves 8 pixels a frame to the
#                       right and back over a grey background with a
#                       darker grid; lossless H.264, so an interpolated
#                       frame's square can be found to the pixel (halfway:
#                       4 pixels on)
#   fixture-square-b.mkv  the same picture as one group of 288 frames
#                       with three non-reference B-frames between its P
#                       frames (215 of them), as films have: an exact
#                       seek with mpv's hr-seek-framedrop decodes none of
#                       those before its target, so a filter that counts
#                       frames shows the square of a later time (lossy,
#                       qp 10: lossless x264 makes no B-frames)
#
# Twelve seconds: RVP's filter changes wait in the last 1.5 s of a file
# (App::apply_video_filters), and the tests' steps must not land there.
#
# usage: tools/test-clips/make-m18-fixtures.sh [FFMPEG]
set -euo pipefail
FFMPEG=${1:-ffmpeg}
OUT=$(cd "$(dirname "$0")/../.." && pwd)/testdata/media
mkdir -p "$OUT"

# The square's left edge is at 28 in the first frame and moves 8 pixels a
# frame to 260, then back (overlay counts frames from 1 here).
"$FFMPEG" -hide_banner -nostdin -loglevel error -y \
    -f lavfi -i "color=0x808080:s=320x240:r=24:d=12,drawgrid=w=40:h=40:t=1:c=0x606060[bg];\
color=white:s=40x40:r=24:d=12[square];\
[bg][square]overlay=x='abs(mod(8*n+240,480)-240)+20':y=100:eval=frame" \
    -c:v libx264 -preset veryslow -qp 0 -pix_fmt yuv420p \
    -fflags +bitexact -metadata title="RVP moving square fixture" "$OUT/fixture-square.mkv"

"$FFMPEG" -hide_banner -nostdin -loglevel error -y \
    -f lavfi -i "color=0x808080:s=320x240:r=24:d=12,drawgrid=w=40:h=40:t=1:c=0x606060[bg];\
color=white:s=40x40:r=24:d=12[square];\
[bg][square]overlay=x='abs(mod(8*n+240,480)-240)+20':y=100:eval=frame" \
    -c:v libx264 -preset medium -qp 10 -pix_fmt yuv420p -bf 3 -b_strategy 0 \
    -x264-params "b-pyramid=none:keyint=288:min-keyint=288:scenecut=0" \
    -fflags +bitexact -metadata title="RVP moving square fixture, B-frames" "$OUT/fixture-square-b.mkv"

ls -l "$OUT/fixture-square.mkv" "$OUT/fixture-square-b.mkv"
