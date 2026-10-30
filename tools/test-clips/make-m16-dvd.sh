#!/usr/bin/env bash
# Builds M16's synthetic DVD with menus (all synthetic: colour fields,
# rectangles, sine tones; no text, no fonts) into OUT/VIDEO_TS. The image
# is not committed: this script and its sources are.
#
#   First play: the main menu (VMGM menu 1), an endless still with three
#     buttons in a row: Play (title 1), Chapters (the title set's chapter
#     menu) and Audio (its audio menu). Each button has a grey frame in the
#     menu's sub-picture, which the highlight colours yellow and the
#     activation orange (spumux recolours the sub-picture's own pixels, as
#     real discs do).
#   Title set 1, menus:
#     chapters (entry ptt): two buttons, chapter 1 and chapter 2 of title 1;
#     audio (entry audio): two buttons, audio stream 1 (440 Hz) or 2
#       (880 Hz), each then playing title 1;
#     subtitles (entry subtitle): two buttons, subtitles off or on, each then
#       playing title 1;
#     root (entry root): goes back to the main menu.
#   Title 1: 8 s, chapters at 0 and 4 s, two audio streams (440 Hz, 880 Hz)
#     and one subpicture stream (a white bar from 1 s to 6 s); its cell ends
#     in a 3 s still; then the main menu again.
#   Title 2: 6 s; then the main menu again.
#
# Titles are longer than the 5 s FFmpeg reads to probe a stream (see
# make-m11-fixtures.sh). Button rectangles (720x480 menu coordinates) are
# printed to OUT/buttons.txt for the tests.
#
# usage: tools/test-clips/make-m16-dvd.sh OUT [FFMPEG [DVDAUTHOR [SPUMUX]]]
# (dvdauthor and spumux: Ubuntu's dvdauthor package, which may be unpacked
# anywhere; they need libdvdread on LD_LIBRARY_PATH then.)
set -euo pipefail
OUT=${1:?usage: make-m16-dvd.sh OUT [FFMPEG [DVDAUTHOR [SPUMUX]]]}
FFMPEG=${2:-ffmpeg}
DVDAUTHOR=${3:-dvdauthor}
SPUMUX=${4:-spumux}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
ff() { "$FFMPEG" -hide_banner -nostdin -loglevel error -y "$@"; }

# A menu's background: a colour field with the buttons' places drawn
# darker, 2 s of MPEG-2 NTSC (the cell is then held as a still).
menu_video() {
    local name=$1 colour=$2 boxes=$3
    ff -f lavfi -i "color=c=$colour:size=720x480:rate=30000/1001:duration=2,$boxes" \
        -f lavfi -i "anullsrc=r=48000:cl=mono:d=2" \
        -target ntsc-dvd -b:v 400k -c:a ac3 -b:a 32k -ac 1 -shortest "$WORK/$name-bg.mpg"
}
# The buttons' pictures for spumux: transparent, with the same frame
# around each button in three colours (normal, highlight, select).
button_pictures() {
    local name=$1 normal=$2 highlight=$3 select=$4
    picture 720x480 "$normal" "$WORK/$name-normal.png"
    picture 720x480 "$highlight" "$WORK/$name-highlight.png"
    picture 720x480 "$select" "$WORK/$name-select.png"
}
# A transparent SIZE picture with what FILTERS draw: drawn on black, which
# then turns transparent (drawbox does not paint a transparent canvas).
picture() {
    ff -f lavfi -i "color=c=black:size=$1,format=rgba,$2,colorkey=black:0.01:0" -frames:v 1 "$3"
}
# Muxes a row of `count` buttons (named b1, b2...; left and right move
# along the row) into a menu's stream.
menu_with_buttons() {
    local name=$1 count=$2 i x buttons=""
    for ((i = 1; i <= count; i++)); do
        x=$((80 + (i - 1) * 200))
        buttons+="      <button name=\"b$i\" x0=\"$x\" y0=\"190\" x1=\"$((x + 160))\" y1=\"290\""
        buttons+=" left=\"b$(( i > 1 ? i - 1 : count ))\" right=\"b$(( i < count ? i + 1 : 1 ))\""
        buttons+=" up=\"b$i\" down=\"b$i\" />"$'\n'
        echo "button b$i $x 190 160 100" >> "$WORK/rows.txt"
    done
    cat > "$WORK/$name-spu.xml" <<XML
<subpictures>
  <stream>
    <spu start="00:00:00.00" force="yes"
         image="$WORK/$name-normal.png" highlight="$WORK/$name-highlight.png"
         select="$WORK/$name-select.png">
$buttons    </spu>
  </stream>
</subpictures>
XML
    "$SPUMUX" -m dvd "$WORK/$name-spu.xml" < "$WORK/$name-bg.mpg" > "$WORK/$name.mpg" \
        2> "$WORK/$name-spumux.log" || { cat "$WORK/$name-spumux.log"; exit 1; }
}
# A row of `count` buttons, 160x100 each, 40 apart, starting at x=80,
# y=190; frames are 6 pixels wide.
row() {
    local count=$1 colour=$2 kind=$3 i x filters=""
    for ((i = 0; i < count; i++)); do
        x=$((80 + i * 200))
        case $kind in
            box) filters+="drawbox=x=$x:y=190:w=160:h=100:color=$colour:t=fill," ;;
            frame) filters+="drawbox=x=$x:y=190:w=160:h=100:color=$colour:t=6," ;;
        esac
    done
    printf '%s' "${filters%,}"
}

: > "$WORK/rows.txt"
echo "main" >> "$WORK/rows.txt"
menu_video main 0x102040 "$(row 3 0x203040 box)"
button_pictures main "$(row 3 0x808080 frame)" "$(row 3 0xffff00 frame)" "$(row 3 0xff8000 frame)"
menu_with_buttons main 3
for menu in chapters audio subtitles; do
    echo "$menu" >> "$WORK/rows.txt"
    menu_video "$menu" 0x204010 "$(row 2 0x203040 box)"
    button_pictures "$menu" "$(row 2 0x808080 frame)" "$(row 2 0xffff00 frame)" "$(row 2 0xff8000 frame)"
    menu_with_buttons "$menu" 2
done
menu_video root 0x401020 "null"
cp "$WORK/root-bg.mpg" "$WORK/root.mpg"

# Title 1: two audio streams and a subpicture stream; title 2: plain.
ff -f lavfi -i "smptebars=size=352x240:rate=30000/1001:duration=8" \
    -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=8" \
    -f lavfi -i "sine=frequency=880:sample_rate=48000:duration=8" \
    -map 0:v -map 1:a -map 2:a -target ntsc-dvd -s 352x240 -b:v 150k -maxrate 600k -bufsize 1835008 \
    -c:a ac3 -b:a 32k -ac 1 "$WORK/title1-av.mpg"
picture 352x240 "drawbox=x=96:y=200:w=160:h=16:color=white:t=fill" "$WORK/subtitle.png"
cat > "$WORK/title1-spu.xml" <<XML
<subpictures>
  <stream>
    <spu start="00:00:01.00" end="00:00:06.00" image="$WORK/subtitle.png" />
  </stream>
</subpictures>
XML
"$SPUMUX" -m dvd "$WORK/title1-spu.xml" < "$WORK/title1-av.mpg" > "$WORK/title1.mpg" \
    2> "$WORK/title1-spumux.log" || { cat "$WORK/title1-spumux.log"; exit 1; }
ff -f lavfi -i "smptebars=size=352x240:rate=30000/1001:duration=6" \
    -f lavfi -i "sine=frequency=660:sample_rate=48000:duration=6" \
    -target ntsc-dvd -s 352x240 -b:v 150k -maxrate 600k -bufsize 1835008 \
    -c:a ac3 -b:a 32k -ac 1 "$WORK/title2.mpg"

cat > "$WORK/dvd.xml" <<XML
<dvdauthor dest="$WORK/DVD">
  <vmgm>
    <fpc>jump menu 1;</fpc>
    <menus>
      <video format="ntsc" aspect="4:3" />
      <pgc entry="title">
        <vob file="$WORK/main.mpg" pause="inf" />
        <button name="b1">jump title 1;</button>
        <button name="b2">jump titleset 1 menu entry ptt;</button>
        <button name="b3">jump titleset 1 menu entry audio;</button>
      </pgc>
    </menus>
  </vmgm>
  <titleset>
    <menus>
      <video format="ntsc" aspect="4:3" />
      <pgc entry="root">
        <vob file="$WORK/root.mpg" />
        <post>jump vmgm menu 1;</post>
      </pgc>
      <pgc entry="ptt">
        <vob file="$WORK/chapters.mpg" pause="inf" />
        <button name="b1">jump title 1 chapter 1;</button>
        <button name="b2">jump title 1 chapter 2;</button>
      </pgc>
      <pgc entry="audio">
        <vob file="$WORK/audio.mpg" pause="inf" />
        <button name="b1">audio = 0; jump title 1;</button>
        <button name="b2">audio = 1; jump title 1;</button>
      </pgc>
      <pgc entry="subtitle">
        <vob file="$WORK/subtitles.mpg" pause="inf" />
        <button name="b1">subtitle = 0; jump title 1;</button>
        <button name="b2">subtitle = 64; jump title 1;</button>
      </pgc>
    </menus>
    <titles>
      <video format="ntsc" aspect="4:3" resolution="352x240" />
      <audio format="ac3" lang="en" />
      <audio format="ac3" lang="de" />
      <subpicture lang="en" />
      <pgc>
        <vob file="$WORK/title1.mpg" chapters="0,0:04" pause="3" />
        <post>call vmgm menu 1;</post>
      </pgc>
      <pgc>
        <vob file="$WORK/title2.mpg" />
        <post>call vmgm menu 1;</post>
      </pgc>
    </titles>
  </titleset>
</dvdauthor>
XML
VIDEO_FORMAT=NTSC "$DVDAUTHOR" -x "$WORK/dvd.xml" > "$WORK/dvdauthor.log" 2>&1 \
    || { cat "$WORK/dvdauthor.log"; exit 1; }
rm -rf "$OUT"
mkdir -p "$OUT"
cp -r "$WORK/DVD/VIDEO_TS" "$OUT/"
cp "$WORK/rows.txt" "$OUT/buttons.txt"
ls -l "$OUT/VIDEO_TS"
