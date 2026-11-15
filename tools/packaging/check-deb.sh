#!/bin/bash
# Checks RVP's Linux packages on another Ubuntu release, run as root inside
# a container of it (CI: ubuntu:26.10, the owner's desktop release):
#
#   docker run --rm -v "$PWD:/src:ro" ubuntu:26.10 \
#       bash /src/tools/packaging/check-deb.sh /src/packages /src/testdata/media
#
# The .deb installs with its dependencies from the release's archive, every
# library the package's files load resolves, and the functional self-test
# passes for the installed copy and for the AppImage (extracted, as a
# container has no FUSE) under Xvfb: software OpenGL, no sound, no session
# bus; M11's self-test runs on the installed copy too, and M16's with the
# synthetic menu discs when a third argument names their folder (dvd/ and
# bd/, from tools/test-clips/make-m16-dvd.sh and make-m16-bluray.py): the
# disc libraries the package carries. Arguments: the folder with
# rvp_*_amd64.deb and RVP-*-x86_64.AppImage, testdata/media
# (fixture-60s.mkv, fixture-3s.mkv, fixture-chapters.mkv and .srt, dvd) and
# optionally the discs' folder.
set -u
packages=$1
fixtures=$2
discs=${3:-}
failures=0

check() {
    # check <name> <PASS|FAIL> [detail]
    echo "check $1=$2 ${3:-}"
    [ "$2" = PASS ] || failures=$((failures + 1))
}

export DEBIAN_FRONTEND=noninteractive
. /etc/os-release
echo "system: $PRETTY_NAME"
apt-get update -qq > /tmp/update.log 2>&1 || tail -5 /tmp/update.log
if apt-get install -y -q "$packages"/rvp_*_amd64.deb > /tmp/apt.log 2>&1; then
    check deb_installed PASS "$(dpkg-query -W -f '${Version}' rvp)"
else
    tail -15 /tmp/apt.log
    check deb_installed FAIL
    echo "failures=$failures"
    exit 1
fi

# rvp and the programs beside it (whisper-cli...) resolve on their own run
# paths, without LD_LIBRARY_PATH: rvp's rpath must find its libraries, and
# the programs RVP starts get none. At run time rvp's run path
# ($ORIGIN/../lib) serves the libraries loaded after it that name no path
# of their own, so ldd on one library alone needs it spelled out. `-r`
# resolves the symbols too (a bundled library older than the system's
# users needs), except for Python's extension modules, which take theirs
# from the interpreter that loads them. vs-mlrt's TensorRT plugin and
# trtexec are left out: they need NVIDIA's libnvinfer.so.10, which RVP
# downloads only on the user's request. LC_ALL=C: ldd's "not found" is
# translated in other locales.
libraries=$(find /opt/rvp -name '*.so*' -type f -not -path '*/vs-mlrt/*')
missing=$( {
    find /opt/rvp -type f -perm -u+x -not -name '*.so*' -not -path '*/vs-mlrt/*' \
        -exec env LC_ALL=C ldd -r {} \;
    for library in $libraries; do
        case $library in
            */python/lib/python3*) LC_ALL=C LD_LIBRARY_PATH=/opt/rvp/lib ldd "$library" ;;
            *) LC_ALL=C LD_LIBRARY_PATH=/opt/rvp/lib ldd -r "$library" ;;
        esac
    done
} 2>&1 | grep -E 'not found|undefined symbol' | sort -u | tr -s ' \t' ' ' | tr '\n' ';')
if [ -z "$missing" ]; then
    check libraries_resolve PASS "rvp and $(printf '%s\n' "$libraries" | wc -l) libraries"
else
    check libraries_resolve FAIL "$missing"
fi

# Qt's plugins RVP needs on every desktop: a find over what is there does
# not notice one that is not (the gtk3 theme was missing until c3f58dc).
absent=""
for plugin in platforms/libqxcb.so platforms/libqwayland.so platforms/libqoffscreen.so \
    platformthemes/libqgtk3.so platformthemes/libqxdgdesktopportal.so \
    wayland-shell-integration/libxdg-shell.so wayland-graphics-integration-client/libqt-plugin-wayland-egl.so \
    xcbglintegrations/libqxcb-glx-integration.so xcbglintegrations/libqxcb-egl-integration.so; do
    [ -f "/opt/rvp/plugins/$plugin" ] || absent="$absent $plugin"
done
if [ -z "$absent" ]; then
    check qt_plugins PASS
else
    check qt_plugins FAIL "missing:$absent"
fi

apt-get install -y -q xvfb xauth libgl1-mesa-dri > /tmp/xvfb.log 2>&1 || tail -5 /tmp/xvfb.log

selftest() {
    # selftest <name> <checks that must pass, space-separated, or ""> <command and its arguments...>
    # A required check passes only without SKIPPED (a self-test that found
    # no disc library writes `=PASS SKIPPED no libdvdnav` and fails nothing).
    local name=$1 must=$2 log result check absent=""
    shift 2
    log=/tmp/selftest-$name.log
    rm -rf /tmp/rvp
    mkdir -p /tmp/rvp/config
    printf '{ "ipc-server": "" }\n' > /tmp/rvp/config/settings.json
    RVP_CONFIG_DIR=/tmp/rvp/config RVP_STATE_DIR=/tmp/rvp/state \
        xvfb-run -a -s '-screen 0 1920x1080x24' "$@" > "$log" 2>&1
    result=$(grep -E '^failures=' "$log" | tail -1)
    for check in $must; do
        grep "^check $check=PASS" "$log" | grep -qv "=PASS SKIPPED" || absent="$absent $check"
    done
    if [ "$result" = failures=0 ] && [ -n "$absent" ]; then
        grep -E "=PASS SKIPPED" "$log" | head -5
        check "selftest_$name" FAIL "not passed:$absent"
    elif [ "$result" = failures=0 ]; then
        check "selftest_$name" PASS "$(grep -c '=PASS' "$log") checks"
    elif [ -z "$result" ]; then
        # RVP ended before its result: what it printed last.
        tail -n 25 "$log"
        check "selftest_$name" FAIL "no result"
    else
        grep -E '=FAIL' "$log" | head -15
        check "selftest_$name" FAIL "$result"
    fi
}

functional=(--selftest --out=/tmp/rvp/out --skin= --mpv-options=ao=null,hwdec=no
    "$fixtures/fixture-60s.mkv" "$fixtures/fixture-3s.mkv")
selftest deb "" /usr/bin/rvp "${functional[@]}"
selftest appimage "" env APPIMAGE_EXTRACT_AND_RUN=1 "$packages"/RVP-*-x86_64.AppImage "${functional[@]}"
selftest m11 "" /usr/bin/rvp --selftest=m11 --out=/tmp/rvp/out --skin= --mpv-options=ao=null,hwdec=no \
    "--dvd-folder=$fixtures/dvd" "--audio=$fixtures/fixture-3s.mkv" "--clip=$fixtures/fixture-60s.mkv" \
    /seek=0:00:03 "/title=M11 title" "$fixtures/fixture-chapters.mkv" "$fixtures/fixture-60s.mkv" \
    "$fixtures/fixture-3s.mkv"
if [ -n "$discs" ]; then
    selftest m16 "m16_menu_shows m16_bd_menu_shows" /usr/bin/rvp --selftest=m16 --out=/tmp/rvp/out --mpv-options=ao=null,hwdec=no \
        "--dvd-menus=$discs/dvd" "--bd-menus=$discs/bd"
fi
echo "failures=$failures"
[ "$failures" -eq 0 ]
