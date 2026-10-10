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
ls -l "$OUT"
