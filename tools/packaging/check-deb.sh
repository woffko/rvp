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

# At run time rvp's run path ($ORIGIN/../lib) serves the libraries loaded
# after it that name no path of their own; ldd on one library alone needs
# it spelled out.
missing=$( {
    LD_LIBRARY_PATH=/opt/rvp/lib ldd /opt/rvp/bin/rvp
    find /opt/rvp -name '*.so*' -type f -exec env LD_LIBRARY_PATH=/opt/rvp/lib ldd {} \;
} 2> /dev/null | grep 'not found' | sort -u | tr -s ' \t' ' ' | tr '\n' ';')
if [ -z "$missing" ]; then
    check libraries_resolve PASS "rvp and $(find /opt/rvp -name '*.so*' -type f | wc -l) libraries"
else
    check libraries_resolve FAIL "$missing"
fi

apt-get install -y -q xvfb xauth libgl1-mesa-dri > /tmp/xvfb.log 2>&1 || tail -5 /tmp/xvfb.log

selftest() {
    # selftest <name> <command and its arguments...>
    local name=$1 log result
    shift
    log=/tmp/selftest-$name.log
    rm -rf /tmp/rvp
    mkdir -p /tmp/rvp/config
    printf '{ "ipc-server": "" }\n' > /tmp/rvp/config/settings.json
    RVP_CONFIG_DIR=/tmp/rvp/config RVP_STATE_DIR=/tmp/rvp/state \
        xvfb-run -a -s '-screen 0 1920x1080x24' "$@" > "$log" 2>&1
    result=$(grep -E '^failures=' "$log" | tail -1)
    if [ "$result" = failures=0 ]; then
        check "selftest_$name" PASS "$(grep -c '=PASS' "$log") checks"
    else
        grep -E '=FAIL' "$log" | head -15
        check "selftest_$name" FAIL "${result:-no result}"
    fi
}

functional=(--selftest --out=/tmp/rvp/out --skin= --mpv-options=ao=null,hwdec=no
    "$fixtures/fixture-60s.mkv" "$fixtures/fixture-3s.mkv")
selftest deb /usr/bin/rvp "${functional[@]}"
selftest appimage env APPIMAGE_EXTRACT_AND_RUN=1 "$packages"/RVP-*-x86_64.AppImage "${functional[@]}"
selftest m11 /usr/bin/rvp --selftest=m11 --out=/tmp/rvp/out --skin= --mpv-options=ao=null,hwdec=no \
    "--dvd-folder=$fixtures/dvd" "--audio=$fixtures/fixture-3s.mkv" "--clip=$fixtures/fixture-60s.mkv" \
    /seek=0:00:03 "/title=M11 title" "$fixtures/fixture-chapters.mkv" "$fixtures/fixture-60s.mkv" \
    "$fixtures/fixture-3s.mkv"
if [ -n "$discs" ]; then
    selftest m16 /usr/bin/rvp --selftest=m16 --out=/tmp/rvp/out --mpv-options=ao=null,hwdec=no \
        "--dvd-menus=$discs/dvd" "--bd-menus=$discs/bd"
fi
echo "failures=$failures"
[ "$failures" -eq 0 ]
