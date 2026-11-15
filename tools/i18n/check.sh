#!/usr/bin/env bash
# Checks RVP's interface languages (M19) on an Xvfb screen of its own:
# `rvp --selftest=language --language=CODE` for each translation found in
# shell/i18n (the language loads by its own name, and every text of the
# settings window fits its control), isolated as tools/screenshots/tour.sh
# is (its own settings, state, runtime folder and session bus: nothing
# reaches a running RVP of the user's). With TOUR_FIRST and TOUR_SECOND
# (two videos, the second with an .srt beside it) it also takes the
# screenshot tour in each language into OUT/CODE/tour.
#
#   tools/i18n/check.sh OUT [CODE...]
#
# Environment: RVP (the binary, default `rvp`; a development build needs
# the kit's LD_LIBRARY_PATH), TOUR_FIRST, TOUR_SECOND, RVP_TOUR_AT.
set -uo pipefail

if [ $# -lt 1 ]; then
    sed -n '2,15p' "$0" >&2
    exit 2
fi
here=$(cd "$(dirname "$0")" && pwd)
out=$(realpath -m "$1")
shift
rvp=${RVP:-rvp}
if [ $# -gt 0 ]; then
    codes=("$@")
else
    mapfile -t codes < <(python3 "$here/tsfile.py")
fi
mkdir -p "$out"
failed=0
for code in "${codes[@]}"; do
    work=$(mktemp -d)
    mkdir -p "$work/config" "$work/state" "$work/run" "$out/$code"
    chmod 700 "$work/run"
    printf '{ "ipc-server": "" }\n' > "$work/config/settings.json"
    (
        cd "$work" || exit 1
        xvfb-run -a -s '-screen 0 1920x1080x24 -nolisten tcp' \
            env -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS QT_QPA_PLATFORM=xcb \
            XDG_RUNTIME_DIR="$work/run" RVP_CONFIG_DIR="$work/config" RVP_STATE_DIR="$work/state" \
            dbus-run-session -- timeout 180 "$rvp" --selftest=language "--language=$code" \
            "--out=$work/out" --mpv-options=ao=null
    ) > "$out/$code/language.log" 2>&1
    if grep -q '^failures=0$' "$out/$code/language.log"; then
        result=PASS
    else
        result=FAIL
        failed=$((failed + 1))
    fi
    echo "$code $result $(grep -h '^check m14_language_fits' "$out/$code/language.log" | cut -c1-160)"
    rm -rf "$work"
    if [ -n "${TOUR_FIRST:-}" ] && [ -n "${TOUR_SECOND:-}" ]; then
        RVP_TOUR_LANGUAGE=$code "$here/../screenshots/tour.sh" "$out/$code/tour" "$TOUR_FIRST" "$TOUR_SECOND" \
            > "$out/$code/tour.out" 2>&1 || { echo "$code tour FAIL (see $out/$code/tour.out)"; failed=$((failed + 1)); }
    fi
done
echo "languages: ${#codes[@]}, failed: $failed"
[ "$failed" -eq 0 ]
