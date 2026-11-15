#!/usr/bin/env bash
# Takes the README's screenshots (docs/images): runs `rvp --selftest=tour`
# (shell/qml/SelfTestTour.qml) on an Xvfb screen of its own, so no window
# shows on the desktop and no sound plays, and captures the screen with
# FFmpeg's x11grab at each `tour_shot=NAME WIDTHxHEIGHT` line into
# OUT/NAME.png, cut to the main window (WIDTHxHEIGHT at the top left).
#
#   tools/screenshots/tour.sh OUT FIRST_VIDEO SECOND_VIDEO
#
# The second video's subtitles are an `.srt` of the same name beside it.
# Environment: RVP (the binary, default `rvp`; a development build needs
# the kit's LD_LIBRARY_PATH), RVP_TOUR_SIZE (the screen, default
# 1440x900), RVP_TOUR_AT (FIRST,SECOND: the seconds each video is shown
# at), RVP_TOUR_LANGUAGE (the interface's, default `en`). RVP's settings
# and state are a temporary folder's.
set -euo pipefail

if [ $# -ne 3 ]; then
    sed -n '2,15p' "$0" >&2
    exit 2
fi
out=$(realpath -m "$1")
first=$(realpath "$2")
second=$(realpath "$3")
rvp=${RVP:-rvp}
size=${RVP_TOUR_SIZE:-1440x900}
mkdir -p "$out"
work=$(mktemp -d)
xvfb=""
cleanup() {
    [ -n "$xvfb" ] && kill "$xvfb" 2>/dev/null
    rm -rf "$work"
}
trap cleanup EXIT

# Xvfb picks a free display and writes its number to the descriptor.
exec 3> "$work/display"
Xvfb -displayfd 3 -screen 0 "${size}x24" -nolisten tcp > "$work/xvfb.log" 2>&1 &
xvfb=$!
exec 3>&-
for _ in $(seq 50); do
    [ -s "$work/display" ] && break
    sleep 0.1
done
display=:$(head -n1 "$work/display")

mkdir -p "$work/config" "$work/state" "$work/run"
# Its own runtime folder and session bus: nothing reaches a running RVP of
# the user's (the second instance's channel, MPRIS, the screen saver).
chmod 700 "$work/run"
# No SVP: a running SVP 4 Manager would attach to the tour's player.
printf '{ "ipc-server": "", "general": { "language": "%s" } }\n' "${RVP_TOUR_LANGUAGE:-en}" \
    > "$work/config/settings.json"

status=0
env -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS DISPLAY="$display" QT_QPA_PLATFORM=xcb \
    XDG_RUNTIME_DIR="$work/run" RVP_CONFIG_DIR="$work/config" RVP_STATE_DIR="$work/state" \
    dbus-run-session -- "$rvp" --selftest=tour "--out=$work/out" "--mpv-options=ao=null" \
    ${RVP_TOUR_AT:+"--tour-at=$RVP_TOUR_AT"} "$first" "$second" 2>&1 |
    while IFS= read -r line; do
        printf '%s\n' "$line" >> "$work/rvp.log"
        case $line in
        tour_shot=*)
            read -r name window <<< "${line#tour_shot=}"
            ffmpeg -v error -y -f x11grab -draw_mouse 0 -video_size "$size" -i "$display" -frames:v 1 \
                -vf "crop=${window%x*}:${window#*x}:0:0" "$out/$name.png"
            echo "shot $name"
            ;;
        check\ *FAIL*) echo "$line" >&2 ;;
        esac
    done || status=$?
cp "$work/rvp.log" "$out/tour.log"
grep -q '^failures=0$' "$out/tour.log" || { echo "the tour failed: $out/tour.log" >&2; exit 1; }
exit "$status"
