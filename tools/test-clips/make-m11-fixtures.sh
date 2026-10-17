#!/usr/bin/env bash
# Generates M11's fixtures in testdata/media (all synthetic: testsrc2, a
# sine tone, text written here):
#
#   fixture-chapters.mkv   20 s, 96x72, 10 fps, a keyframe every 2 s, four
#                          chapters (Opening 0 s, Part A 5 s, Part B 10 s,
#                          Ending 15 s)
#   fixture-chapters.srt   four subtitle lines at 1, 6, 11 and 16 s
#   dvd/VIDEO_TS/          a DVD with two titles (8 s with chapters at 0 and
#                          4 s, and 6 s): colour bars, 352x240 NTSC MPEG-2
#                          with AC3, authored with dvdauthor; each title
#                          ends with an exit. Titles are longer than the
#                          5 s FFmpeg reads to probe a stream: mpv reads on
#                          past a shorter title's end into the next title
#                          libdvdnav plays, and takes its length and
#                          chapters.
#
# usage: tools/test-clips/make-m11-fixtures.sh [FFMPEG [DVDAUTHOR]]
# (dvdauthor: Ubuntu's package, which may be unpacked anywhere; it needs
# its libdvdread on LD_LIBRARY_PATH then.)
set -euo pipefail
FFMPEG=${1:-ffmpeg}
DVDAUTHOR=${2:-dvdauthor}
OUT=$(cd "$(dirname "$0")/../.." && pwd)/testdata/media
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUT"

cat > "$WORK/chapters.txt" <<'META'
;FFMETADATA1
[CHAPTER]
TIMEBASE=1/1000
START=0
END=5000
title=Opening
[CHAPTER]
TIMEBASE=1/1000
START=5000
END=10000
title=Part A
[CHAPTER]
TIMEBASE=1/1000
START=10000
END=15000
title=Part B
[CHAPTER]
TIMEBASE=1/1000
START=15000
END=20000
title=Ending
META
"$FFMPEG" -hide_banner -nostdin -loglevel error -y \
    -f lavfi -i "testsrc2=size=96x72:rate=10:duration=20" \
    -f lavfi -i "sine=frequency=330:sample_rate=48000:duration=20" \
    -i "$WORK/chapters.txt" -map 0:v -map 1:a -map_metadata 2 -map_chapters 2 \
    -c:v libx264 -preset veryslow -crf 51 -g 20 -keyint_min 20 -sc_threshold 0 -pix_fmt yuv420p \
    -c:a libopus -b:a 6k -ac 1 -metadata title="RVP chapters fixture" \
    -fflags +bitexact "$OUT/fixture-chapters.mkv"

printf '%s\n' 1 '00:00:01,000 --> 00:00:02,500' 'First line' '' \
    2 '00:00:06,000 --> 00:00:07,500' 'Second line' '' \
    3 '00:00:11,000 --> 00:00:12,500' 'Third line' '' \
    4 '00:00:16,000 --> 00:00:17,500' 'Fourth line' '' > "$OUT/fixture-chapters.srt"

dvd_title() {
    local name=$1 seconds=$2
    "$FFMPEG" -hide_banner -nostdin -loglevel error -y \
        -f lavfi -i "smptebars=size=352x240:rate=30000/1001:duration=$seconds" \
        -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=$seconds" \
        -target ntsc-dvd -s 352x240 -b:v 150k -maxrate 600k -bufsize 1835008 \
        -c:a ac3 -b:a 32k -ac 1 "$WORK/$name.mpg"
}
dvd_title title1 8
dvd_title title2 6
cat > "$WORK/dvd.xml" <<XML
<dvdauthor dest="$WORK/DVD">
  <vmgm />
  <titleset>
    <titles>
      <video format="ntsc" aspect="4:3" resolution="352x240" />
      <pgc><vob file="$WORK/title1.mpg" chapters="0,0:04" /><post>exit;</post></pgc>
      <pgc><vob file="$WORK/title2.mpg" /><post>exit;</post></pgc>
    </titles>
  </titleset>
</dvdauthor>
XML
VIDEO_FORMAT=NTSC "$DVDAUTHOR" -x "$WORK/dvd.xml" > "$WORK/dvdauthor.log" 2>&1 \
    || { cat "$WORK/dvdauthor.log"; exit 1; }
rm -rf "$OUT/dvd"
mkdir -p "$OUT/dvd"
cp -r "$WORK/DVD/VIDEO_TS" "$OUT/dvd/"
ls -l "$OUT/fixture-chapters.mkv" "$OUT/fixture-chapters.srt" "$OUT/dvd/VIDEO_TS"
