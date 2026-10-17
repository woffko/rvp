#!/usr/bin/env bash
# Linux test kit for goal B (see docs/PLAN.md and tools/linux-kit/CHECKLIST.md).
#
# Builds libmpv and the spikes in user space and records the results of the
# spikes S1 (SVP attaches), S2 (VapourSynth interpolation) and S3 (Qt shell)
# for docs/test-runs/. Steps that need root are only printed, never run.
#
# usage: tools/linux-kit/kit.sh <step>...
#   check   system, GPU, session, missing packages (prints the apt command)
#   deps    Python tools (meson, aqtinstall) and Qt in the kit directory
#   mpv     build libmpv (mpv master, FFmpeg, libplacebo) with VapourSynth
#           loaded at run time
#   mvtools build the mvtools VapourSynth plugin that PotPlayer's script needs
#   build   build the spikes
#   s2      VapourSynth interpolation through libmpv
#   s3      Qt shell self-test in the current graphical session
#   input   drag the frameless S3 window with injected mouse input (needs
#           ydotool and ydotoold running as root; optional)
#   s1      SVP 4 Manager attaches to a libmpv player (start SVP first)
#   m2build build the RVP player (milestone M2) against the kit's Qt and
#           libmpv
#   m2      RVP's functional self-test, then a restart that must resume the
#           last file paused at its position
#   m2input drag, resize, double-click and right-click the RVP window with
#           injected mouse input (needs ydotool, like input)
#   m2gpu   RVP plays the test video for RVP_M2_SECONDS (default 25) in
#           fullscreen and in a 1920x1080 window and counts dropped frames
#           (goal D's open item; GPU machines only)
#   m2svp   RVP plays the film-rate clip with SVP attached, then exits
#           (goal D rule 6; start SVP first)
#   m3      milestone M3: the functional self-test with the playlist panel,
#           albums, normalizer, key editor, a second instance (/add) and the
#           aspect lock, then three kill tests (kill -9 while the second
#           album entry plays; the restart must reopen it paused within
#           5 s of where it was)
#   m4      milestones M4 and M5: the functional self-test's menu and
#           settings checks (PotPlayer's menus with the bound keys, a menu
#           command, every settings page, a changed setting that applies at
#           once), then a portable copy of RVP (rvp.portable next to it)
#           that must keep its settings next to the executable
#   m6      milestone M6: the functional self-test's control panel (F7),
#           colour, equalizer, shader and SVP diagnostic checks; the
#           orientation check (--selftest=upright on a clip with a corner
#           mark); the speed carry-over (--selftest=speed: frames dropped
#           after 1.1x -> 1x at a set moment); a VapourSynth script slot in
#           RVP with SVP's VapourSynth
#           (RVP's own get_core script, and RVP_VPY with the kit's mvtools
#           when set); and the headless video tests (cargo)
#   m6svp   SVP's acceptance (section 7, start SVP first): RVP in SVP mode
#           on the film-rate clip reaches SVP's frame rate and stays in sync
#           after seeks; a second window (/new) on its own endpoint too
#   m6gpu   the 4K60 clip in fullscreen with M6's processing on (colours,
#           deband, a shader, the equalizer) counts dropped frames (GPU
#           machines only)
#   m7      milestone M7: the functional self-test on RVP Default, RVP's
#           own skin (--skin= with no path, whatever RVP_SKIN says): its
#           logo resources, the playlist tabs and the control panel's
#           checks; then the orientation check on it, with the pointer over
#           the picture in the window and in fullscreen with and without
#           the bars
#   m8package  milestone M8: RVP built again with its library folder in
#           its run path (target/package), then the AppImage and the .deb
#           from this system's Qt and libmpv (tools/packaging/package.py),
#           in the log directory's m8-packages
#   m8      milestone M8 on those packages: the AppImage's functional
#           self-test; the .deb installed (passwordless sudo, as on the
#           shared VM; elsewhere install it first: the step names the
#           command), its self-test, desktop entry, registration for the
#           media types (no default changed), MPRIS and the screen kept on
#           while video plays (on the desktop's session bus; MPRIS alone on
#           a private bus where there is none), crash logs, and removed
#           again leaving only RVP's settings and state
#   m8removed  after `sudo apt remove rvp` where m8 could not remove the
#           .deb itself: nothing of the package is left, RVP's settings
#           and state are
#   m3desktop  optional, not in m3all: an "RVP (kit)" entry in
#           ~/.local/share/applications so that Files can open videos in
#           the running RVP (single instance); delete the file to undo
#   report  write docs/test-runs/<date>-<host>.md from the logs
#   all     check deps mpv mvtools build s2 s3 input s1 report
#   m2all   check deps mpv m2build m2 m2input m2gpu m2svp report
#   m3all   check deps mpv m2build m2 m3 m2input m2gpu m2svp report
#   m4all   check deps mpv m2build m2 m3 m4 m2input m2gpu m2svp report
#   m6all   check deps mpv mvtools m2build m2 m3 m4 m6 m2input m2gpu m6gpu
#           m2svp m6svp report
#   m7all   m6all with m7 after m6
#   m8all   m7all with m8package and m8 after m7
#
# environment:
#   RVP_KIT_DIR     build and download directory (default ~/.cache/rvp-kit)
#   RVP_LOG_DIR     logs (default $RVP_KIT_DIR/logs)
#   RVP_VIDEO       test video for S3 (default: generated 4K60 or 1080p60)
#   RVP_S1_VIDEO    film-rate test video for S1 (default: generated 23.976 fps)
#   RVP_SKIN        .dsf skin for S3 and the m-steps (default: none for S3,
#                   which then paints nothing; RVP Default for RVP)
#   RVP_VPY         motion interpolation script for S2
#   VSSCRIPT_PATH   libvapoursynth-script.so to use (default: searched)
#   RVP_VS_PLUGINS  extra VapourSynth plugins for S2, ':' separated (default:
#                   the kit's mvtools)
#   YDOTOOL_SOCKET  ydotoold's socket for the input check
#                   (default /tmp/.ydotool_socket)
#   RVP_GPU_CHECKS  1 or 0 to override GPU detection
#   RVP_KIT_REBUILD 1 to rebuild libmpv even if this commit is already built
#   RVP_M2_SECONDS  playback length of the m2gpu runs (default 25; the
#                   generated clip is 30 s long and must not loop)
set -u

KIT_SCRIPT=$(readlink -f "${BASH_SOURCE[0]}")
REPO=$(cd "$(dirname "$KIT_SCRIPT")/../.." && pwd)
KIT_DIR=${RVP_KIT_DIR:-$HOME/.cache/rvp-kit}
LOG_DIR=${RVP_LOG_DIR:-$KIT_DIR/logs}
QT_VERSION=6.10.3
MPV_COMMIT=b2c255c13e
# The libmpv build's own revision, bumped when its options change (so that a
# built or cached libmpv of the same commit is built again): 2 added
# rubberband, which Audio Stretching needs.
MPV_BUILD=2
VS_TAG=R73
# The version in the Windows bundle (spikes/tools/windows-deps.sh).
MVTOOLS_TAG=v24
JOBS=$(nproc)
mkdir -p "$KIT_DIR" "$LOG_DIR"
RESULTS=$LOG_DIR/results.txt

say() { printf 'kit: %s\n' "$*"; }
record() {
    # record <step> <PASS|FAIL|SKIPPED> <detail>
    printf '%s\t%s\t%s\t%s\n' "$(date -Is)" "$1" "$2" "$3" >> "$RESULTS"
    say "$1 $2 $3"
}

gpu_vendor() {
    if command -v nvidia-smi > /dev/null 2>&1 && nvidia-smi -L > /dev/null 2>&1; then
        echo nvidia
        return
    fi
    local line
    # Only the device description: the class name ("VGA compatible
    # controller") would match "ati".
    line=$(lspci 2>/dev/null | grep -i -E 'vga|3d controller|display' | head -1 | sed 's/^[^:]*:[^:]*: //')
    case "${line,,}" in
        *vmware* | *virtio* | *qxl* | *cirrus* | *bochs* | *"basic display"*) echo virtual ;;
        *nvidia*) echo nvidia ;;
        *"advanced micro devices"* | *radeon* | *amd*) echo amd ;;
        *intel*) echo intel ;;
        *) echo unknown ;;
    esac
}

gpu_checks() {
    if [ -n "${RVP_GPU_CHECKS:-}" ]; then
        [ "$RVP_GPU_CHECKS" = 1 ]
        return
    fi
    case $(gpu_vendor) in nvidia | amd | intel) return 0 ;; *) return 1 ;; esac
}

qt_dir() { echo "$KIT_DIR/qt/$QT_VERSION/gcc_64"; }
mpv_prefix() { echo "$KIT_DIR/mpv"; }

# Packages needed to build libmpv and mvtools and to run the Qt spike, by
# Debian name. Without libpipewire-0.3-dev mpv plays audio through the
# PulseAudio API, whose coarse timing made it drop 4-5 video frames per second
# at 60 fps on PipeWire systems (goal D); libffmpeg-nvenc-dev enables NVDEC.
PACKAGES="build-essential git curl python3-venv python3-jinja2 pkg-config nasm cmake ninja-build \
autoconf automake libtool \
libass-dev libfreetype-dev libfribidi-dev libharfbuzz-dev libfontconfig-dev \
libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxss-dev libxv-dev libxpresent-dev \
libxkbcommon-dev libwayland-dev wayland-protocols libegl-dev libgl-dev libgbm-dev libdrm-dev \
libasound2-dev libpulse-dev libpipewire-0.3-dev libva-dev libvdpau-dev libffmpeg-nvenc-dev \
zlib1g-dev libssl-dev liblcms2-dev libjpeg-dev \
libvulkan-dev glslang-dev libunwind-dev libfftw3-dev librubberband-dev \
libxcb-cursor0 libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-render-util0 libxcb-xinerama0 \
libxcb-xkb1 libxkbcommon-x11-0 libopengl0 libwayland-cursor0 libwayland-egl1 \
lsof mesa-utils ffmpeg"

step_check() {
    local log=$LOG_DIR/check.log
    {
        echo "date=$(date -Is)"
        echo "host=$(hostname)"
        echo "os=$(. /etc/os-release && echo "$PRETTY_NAME")"
        echo "kernel=$(uname -r)"
        echo "cpu=$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')"
        echo "cpus=$(nproc)"
        echo "gpu_vendor=$(gpu_vendor)"
        lspci 2>/dev/null | grep -i -E 'vga|3d controller|display' | sed 's/^/gpu=/'
        if command -v nvidia-smi > /dev/null 2>&1; then
            nvidia-smi --query-gpu=name,driver_version --format=csv,noheader 2>/dev/null | sed 's/^/nvidia=/'
        fi
        echo "session_type=${XDG_SESSION_TYPE:-none}"
        echo "desktop=${XDG_CURRENT_DESKTOP:-none}"
        echo "wayland_display=${WAYLAND_DISPLAY:-none}"
        echo "display=${DISPLAY:-none}"
        echo "python=$(python3 --version 2>&1)"
        # From the repository, so rust-toolchain.toml selects the toolchain.
        echo "rustc=$(cd "$REPO" && rustc --version 2>/dev/null || echo missing)"
        echo "cargo=$(cd "$REPO" && cargo --version 2>/dev/null || echo missing)"
        echo "svp_running=$(pgrep -f -l SVPManager > /dev/null && echo yes || echo no)"
    } > "$log"
    cat "$log"
    local missing=""
    for package in $PACKAGES; do
        dpkg -s "$package" > /dev/null 2>&1 || missing="$missing $package"
    done
    # qt-build-utils links Qt programs with lld, gold or mold (GNU ld.bfd
    # fails); newer binutils may no longer ship gold.
    if ! command -v lld > /dev/null 2>&1 && ! command -v ld.gold > /dev/null 2>&1 &&
        ! command -v mold > /dev/null 2>&1; then
        missing="$missing lld"
    fi
    if [ -n "$missing" ]; then
        say "missing packages; run as an administrator:"
        echo "  sudo apt-get install -y$missing" | tee -a "$log"
        record check FAIL "missing packages:$missing"
        return 1
    fi
    if ! command -v cargo > /dev/null 2>&1; then
        say "Rust is missing; install it with rustup (https://rustup.rs) as your user"
        record check FAIL "cargo missing"
        return 1
    fi
    record check PASS "gpu=$(gpu_vendor) session=${XDG_SESSION_TYPE:-none}"
}

step_deps() {
    local log=$LOG_DIR/deps.log
    {
        [ -x "$KIT_DIR/venv/bin/python" ] || python3 -m venv "$KIT_DIR/venv"
        "$KIT_DIR/venv/bin/pip" install -q --upgrade pip
        "$KIT_DIR/venv/bin/pip" install -q 'meson>=1.6' aqtinstall jinja2
        if [ ! -x "$(qt_dir)/bin/qmake" ]; then
            # aqt writes aqtinstall.log into the current directory.
            (cd "$KIT_DIR" && "$KIT_DIR/venv/bin/aqt" install-qt linux desktop "$QT_VERSION" linux_gcc_64 \
                --outputdir "$KIT_DIR/qt")
        fi
        "$(qt_dir)/bin/qmake" -query QT_VERSION
    } > "$log" 2>&1
    if [ -x "$(qt_dir)/bin/qmake" ]; then
        record deps PASS "Qt $QT_VERSION, meson $("$KIT_DIR/venv/bin/meson" --version)"
    else
        record deps FAIL "see $log"
        return 1
    fi
}

# VapourSynth headers only: libmpv loads libvapoursynth-script at run time.
vs_headers() {
    local dir=$KIT_DIR/vs-sdk
    mkdir -p "$dir/include/vapoursynth" "$dir/lib/pkgconfig"
    for header in VapourSynth4.h VSScript4.h VSHelper4.h VSConstants4.h; do
        [ -s "$dir/include/vapoursynth/$header" ] ||
            curl -fsSL "https://raw.githubusercontent.com/vapoursynth/vapoursynth/$VS_TAG/include/$header" \
                -o "$dir/include/vapoursynth/$header"
    done
    cat > "$dir/lib/pkgconfig/vapoursynth-script.pc" << EOF
prefix=$dir
includedir=\${prefix}/include/vapoursynth
Name: vapoursynth-script
Description: VapourSynth headers for run-time loading ($VS_TAG)
Version: ${VS_TAG#R}
Cflags: -I\${includedir}
EOF
    echo "$dir/lib/pkgconfig"
}

step_mpv() {
    local log=$LOG_DIR/mpv.log build=$KIT_DIR/mpv-build prefix
    prefix=$(mpv_prefix)
    # mpv-build always rebuilds FFmpeg from scratch; skip a finished build of
    # the same commit unless RVP_KIT_REBUILD=1.
    if [ "${RVP_KIT_REBUILD:-0}" != 1 ] && [ "$(cat "$prefix/.kit-mpv" 2> /dev/null)" = "$MPV_COMMIT build $MPV_BUILD" ]; then
        record mpv PASS "libmpv $MPV_COMMIT build $MPV_BUILD in $prefix (already built)"
        return 0
    fi
    {
        export PATH=$KIT_DIR/venv/bin:$PATH
        # A failing clone must not wait for credentials; rerun the step.
        export GIT_TERMINAL_PROMPT=0
        PKG_CONFIG_PATH=$(vs_headers):${PKG_CONFIG_PATH:-}
        export PKG_CONFIG_PATH
        [ -d "$build" ] || git clone --depth 1 https://github.com/mpv-player/mpv-build "$build"
        cd "$build" || exit 1
        # Partial clones: much smaller, and a plain clone of libass fails
        # with git 2.34 on Ubuntu 22.04. mpv-build keeps existing checkouts.
        for repo in ffmpeg=https://github.com/FFmpeg/FFmpeg.git \
            libass=https://github.com/libass/libass.git \
            libplacebo=https://github.com/haasn/libplacebo.git \
            mpv=https://github.com/mpv-player/mpv.git; do
            [ -d "${repo%%=*}/.git" ] || git clone --filter=blob:none "${repo#*=}" "${repo%%=*}"
        done
        ./use-ffmpeg-release
        ./use-libplacebo-release
        ./use-mpv-custom "$MPV_COMMIT"
        printf '%s\n' -Dlibmpv=true -Dcplayer=false -Dvapoursynth=enabled -Drubberband=enabled \
            -Dlua=disabled -Djavascript=disabled "--prefix=$prefix" > mpv_options
        printf '%s\n' --disable-programs --disable-doc > ffmpeg_options
        # FFmpeg's hwcontext_vulkan.h needs Vulkan headers 1.3.277+. With
        # older headers (Ubuntu 22.04 has 1.3.204) FFmpeg builds without
        # Vulkan, but libplacebo would still include that header, so Vulkan
        # is turned off in libplacebo and mpv too. RVP renders with OpenGL.
        if pkg-config --atleast-version=1.3.277 vulkan; then
            : > libplacebo_options
        else
            echo "Vulkan headers $(pkg-config --modversion vulkan): building without Vulkan"
            echo -Dvulkan=disabled > libplacebo_options
            echo -Dvulkan=disabled >> mpv_options
        fi
        ./update
        ./rebuild "-j$JOBS"
        ./install
    } > "$log" 2>&1
    if [ -e "$prefix/lib/x86_64-linux-gnu/libmpv.so" ] || [ -e "$prefix/lib/libmpv.so" ]; then
        echo "$MPV_COMMIT build $MPV_BUILD" > "$prefix/.kit-mpv"
        record mpv PASS "libmpv $MPV_COMMIT build $MPV_BUILD in $prefix"
    else
        record mpv FAIL "see $log"
        return 1
    fi
}

mvtools_lib() { echo "$KIT_DIR/mvtools/build/libmvtools.so"; }

# PotPlayer's motioninterpolation.vpy needs mvtools; neither SVP's runtime
# nor Ubuntu ships it. mvtools v24 uses the VapourSynth API 3 headers, which
# R73 still ships and loads.
step_mvtools() {
    local log=$LOG_DIR/mvtools.log dir=$KIT_DIR/mvtools
    if [ -s "$(mvtools_lib)" ] && [ "$(cat "$dir/.kit-mvtools" 2> /dev/null)" = "$MVTOOLS_TAG" ]; then
        record mvtools PASS "mvtools $MVTOOLS_TAG in $dir (already built)"
        return 0
    fi
    {
        export PATH=$KIT_DIR/venv/bin:$PATH
        export GIT_TERMINAL_PROMPT=0
        vs_headers > /dev/null
        for header in VapourSynth.h VSHelper.h; do
            [ -s "$KIT_DIR/vs-sdk/include/vapoursynth/$header" ] ||
                curl -fsSL "https://raw.githubusercontent.com/vapoursynth/vapoursynth/$VS_TAG/include/$header" \
                    -o "$KIT_DIR/vs-sdk/include/vapoursynth/$header"
        done
        mkdir -p "$dir/pkgconfig"
        cat > "$dir/pkgconfig/vapoursynth.pc" << EOF
prefix=$KIT_DIR/vs-sdk
includedir=\${prefix}/include/vapoursynth
Name: vapoursynth
Description: VapourSynth headers for building plugins ($VS_TAG)
Version: ${VS_TAG#R}
Cflags: -I\${includedir}
EOF
        [ -d "$dir/src/.git" ] ||
            git clone --depth 1 --branch "$MVTOOLS_TAG" https://github.com/dubhater/vapoursynth-mvtools "$dir/src"
        rm -rf "$dir/build"
        PKG_CONFIG_PATH=$dir/pkgconfig meson setup "$dir/build" "$dir/src" --buildtype=release &&
            ninja -C "$dir/build"
    } > "$log" 2>&1
    if [ -s "$(mvtools_lib)" ]; then
        echo "$MVTOOLS_TAG" > "$dir/.kit-mvtools"
        record mvtools PASS "mvtools $MVTOOLS_TAG in $dir"
    else
        record mvtools FAIL "see $log"
        return 1
    fi
}

mpv_lib_dir() {
    local prefix
    prefix=$(mpv_prefix)
    if [ -e "$prefix/lib/x86_64-linux-gnu/libmpv.so" ]; then
        echo "$prefix/lib/x86_64-linux-gnu"
    else
        echo "$prefix/lib"
    fi
}

# The spikes link the kit's Qt and libmpv, which are not on the system path.
spike_env() {
    case ":${LD_LIBRARY_PATH:-}:" in *":$(qt_dir)/lib:"*) return ;; esac
    LD_LIBRARY_PATH=$(qt_dir)/lib:$(mpv_lib_dir)${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
    export LD_LIBRARY_PATH
}

step_build() {
    local log=$LOG_DIR/build.log
    (
        cd "$REPO/spikes" || exit 1
        export QMAKE
        QMAKE=$(qt_dir)/bin/qmake
        export MPV_LIB_DIR
        MPV_LIB_DIR=$(mpv_lib_dir)
        cargo build --release -p s1-svp-attach -p s2-vapoursynth -p s3-qt-shell
    ) > "$log" 2>&1
    if [ -x "$REPO/spikes/target/release/s3-qt-shell" ]; then
        record build PASS "spikes built"
    else
        record build FAIL "see $log"
        return 1
    fi
}

find_vsscript() {
    if [ -n "${VSSCRIPT_PATH:-}" ] && [ -e "$VSSCRIPT_PATH" ]; then
        echo "$VSSCRIPT_PATH"
        return
    fi
    local candidate
    # SVP 4 for Linux installs into ~/SVP4 by default (4.7.0.321).
    for candidate in /opt/svp/*/libvapoursynth-script.so* "$HOME"/SVP4/mpv/libvapoursynth-script.so* \
        "$HOME"/.local/share/SVP4/*/libvapoursynth-script.so* \
        /usr/local/lib/libvapoursynth-script.so* /usr/lib/x86_64-linux-gnu/libvapoursynth-script.so*; do
        [ -e "$candidate" ] && { echo "$candidate"; return; }
    done
}

test_video() {
    if [ -n "${RVP_VIDEO:-}" ]; then
        echo "$RVP_VIDEO"
        return
    fi
    local clip=$KIT_DIR/test-1080p60.mkv
    if gpu_checks; then
        clip=$KIT_DIR/test-2160p60-hevc.mkv
    fi
    if [ ! -s "$clip" ] && command -v ffmpeg > /dev/null 2>&1; then
        if gpu_checks && [ "$(gpu_vendor)" = nvidia ]; then
            ffmpeg -nostdin -loglevel error -y -f lavfi -i testsrc2=size=3840x2160:rate=60 -t 30 \
                -c:v hevc_nvenc -b:v 40M "$clip" 2>> "$LOG_DIR/video.log" ||
                ffmpeg -nostdin -loglevel error -y -f lavfi -i testsrc2=size=3840x2160:rate=60 -t 30 \
                    -c:v libx265 -preset ultrafast -crf 22 "$clip" 2>> "$LOG_DIR/video.log"
        else
            ffmpeg -nostdin -loglevel error -y -f lavfi -i testsrc2=size=1920x1080:rate=60 -t 30 \
                -c:v libx264 -preset ultrafast -crf 20 "$clip" 2>> "$LOG_DIR/video.log"
        fi
    fi
    if [ -s "$clip" ]; then
        echo "$clip"
    else
        echo "av://lavfi:testsrc2=size=1920x1080:rate=60"
    fi
}

# S1 needs a film-rate source for SVP to interpolate; the S3 clip is 60 fps
# already. Same parameters as video-1080p23976-h264.mkv from
# tools/test-clips/make-clips.sh, the clip goal C used on Windows.
s1_video() {
    if [ -n "${RVP_S1_VIDEO:-}" ]; then
        echo "$RVP_S1_VIDEO"
        return
    fi
    local clip=$KIT_DIR/test-1080p23976-h264.mkv
    if [ ! -s "$clip" ] && command -v ffmpeg > /dev/null 2>&1; then
        ffmpeg -nostdin -loglevel error -y -f lavfi -i testsrc2=size=1920x1080:rate=24000/1001 \
            -f lavfi -i sine=frequency=440:sample_rate=48000,volume=0.03,aformat=channel_layouts=stereo \
            -t 60 -c:v libx264 -preset medium -crf 18 -pix_fmt yuv420p -c:a flac "$clip" 2>> "$LOG_DIR/video.log"
    fi
    if [ -s "$clip" ]; then
        echo "$clip"
    else
        echo "av://lavfi:testsrc2=size=1920x1080:rate=24000/1001"
    fi
}

step_s2() {
    local log=$LOG_DIR/s2.log vsscript
    spike_env
    vsscript=$(find_vsscript)
    if [ -z "$vsscript" ]; then
        record s2 SKIPPED "no libvapoursynth-script.so (install SVP 4 or set VSSCRIPT_PATH)"
        return 0
    fi
    if [ -z "${RVP_VPY:-}" ] || [ ! -e "$RVP_VPY" ]; then
        record s2 SKIPPED "set RVP_VPY to motioninterpolation.vpy"
        return 0
    fi
    local plugins=${RVP_VS_PLUGINS:-}
    [ -n "$plugins" ] || [ ! -s "$(mvtools_lib)" ] || plugins=$(mvtools_lib)
    : > "$log"
    # The unchanged script is expected to fail on VapourSynth R73 (no
    # vs.get_core); the result comes from the run with the wrapper.
    # PYTHONDONTWRITEBYTECODE keeps bytecode out of SVP's installation.
    for mode in plain --compat; do
        echo "== $mode" >> "$log"
        VSSCRIPT_PATH=$vsscript RVP_VS_PLUGINS=$plugins PYTHONDONTWRITEBYTECODE=1 \
            "$REPO/spikes/target/release/s2-vapoursynth" \
            --script "$RVP_VPY" --seconds 15 $([ "$mode" = --compat ] && echo --compat) >> "$log" 2>&1
    done
    local plain_result compat_result
    plain_result=$(grep '^result=' "$log" | head -1)
    compat_result=$(grep '^result=' "$log" | tail -1)
    if [ "$compat_result" = "result=PASS" ]; then
        record s2 PASS "unchanged script ${plain_result#result=}; wrapper $(grep -E '^(estimated_vf_fps_mean|realtime_ratio)=' "$log" | tail -2 | tr '\n' ' ')"
    else
        record s2 FAIL "see $log"
        return 1
    fi
}

step_s3() {
    local log=$LOG_DIR/s3.log out=$LOG_DIR/s3-grabs
    spike_env
    if [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; then
        record s3 SKIPPED "no graphical session"
        return 0
    fi
    local extra=() video
    video=$(test_video)
    if gpu_checks; then
        # Chrome updates ride on video frames, the rule goal C established
        # (docs/spikes/s3-qt-shell.md); the other modes drop frames at 60 Hz.
        extra+=(--gpu-checks --chrome synced --hwdec auto-copy --video-seconds 20 --width 1920 --height 1080)
    else
        extra+=(--hwdec no --video-seconds 8)
    fi
    "$REPO/spikes/target/release/s3-qt-shell" --selftest --out "$out" \
        --skin "${RVP_SKIN:-}" --video "$video" "${extra[@]}" > "$log" 2>&1
    local status=$?
    if gpu_checks; then
        : # GPU checks ran inside the self-test.
    else
        echo "gpu checks skipped: no hardware GPU" >> "$log"
    fi
    if [ "$status" -eq 0 ]; then
        record s3 PASS "$(grep -E '^(graphics_api|mpv_estimated_vf_fps|mpv_frames_rendered|gpu_checks)=' "$log" | tr '\n' ' ')"
    else
        record s3 FAIL "$( (grep -E '=FAIL' "$log" || tail -1 "$log") | tr '\n' ' ')"
    fi
    return "$status"
}

# Prepares an input check: ydotool, its socket and the screen size from
# `$2` (the log of a run that printed its environment). Prints "WIDTH HEIGHT".
input_ready() {
    local step=$1 size_log=$2
    if ! command -v ydotool > /dev/null 2>&1 || ! pgrep -x ydotoold > /dev/null 2>&1; then
        record "$step" SKIPPED "needs ydotool 1.x and 'sudo ydotoold' running"
        return 1
    fi
    # Under sudo, ydotoold has no XDG_RUNTIME_DIR and listens on
    # /tmp/.ydotool_socket, while the client looks in $XDG_RUNTIME_DIR; the
    # checklist starts the daemon with that path explicitly.
    export YDOTOOL_SOCKET=${YDOTOOL_SOCKET:-/tmp/.ydotool_socket}
    if [ ! -S "$YDOTOOL_SOCKET" ]; then
        record "$step" SKIPPED "no ydotoold socket at $YDOTOOL_SOCKET (see CHECKLIST.md)"
        return 1
    fi
    if [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; then
        record "$step" SKIPPED "no graphical session"
        return 1
    fi
    local size width height
    size=$(grep -o '"width":[0-9]*,"height":[0-9]*\|"height":[0-9]*,"name":"[^"]*","width":[0-9]*\|"height":[0-9]*,"name":"[^"]*","refresh_rate":[0-9.]*,"width":[0-9]*' "$size_log" 2> /dev/null | head -1)
    width=$(echo "$size" | grep -o '"width":[0-9]*' | cut -d: -f2)
    height=$(echo "$size" | grep -o '"height":[0-9]*' | cut -d: -f2)
    if [ -z "$width" ] || [ -z "$height" ]; then
        record "$step" SKIPPED "run $(basename "$size_log" .log) first (the screen size comes from its log)"
        return 1
    fi
    echo "$width $height"
}

# Runs input-check.py with GNOME's pointer acceleration set to flat
# (absolute moves are only exact then) and restores the profile afterwards.
run_input_check() {
    local log=$1
    shift
    local profile="" status
    if command -v gsettings > /dev/null 2>&1; then
        profile=$(gsettings get org.gnome.desktop.peripherals.mouse accel-profile 2> /dev/null)
        [ -z "$profile" ] || gsettings set org.gnome.desktop.peripherals.mouse accel-profile flat
    fi
    python3 "$REPO/tools/linux-kit/input-check.py" "$@" > "$log" 2>&1
    status=$?
    [ -z "$profile" ] || gsettings set org.gnome.desktop.peripherals.mouse accel-profile "$profile"
    return "$status"
}

step_input() {
    local log=$LOG_DIR/input.log screen
    spike_env
    screen=$(input_ready input "$LOG_DIR/s3.log") || return 0
    # shellcheck disable=SC2086 # "WIDTH HEIGHT"
    run_input_check "$log" "$REPO/spikes/target/release/s3-qt-shell" "${RVP_SKIN:-}" $screen \
        "$LOG_DIR/input-app.log"
    local status=$?
    if [ "$status" -eq 0 ]; then
        record input PASS "$(grep -E '^input_check' "$log" | tr '\n' ' ')"
    else
        record input FAIL "$(grep -E '^(input_check|window_origin)' "$log" | tr '\n' ' ')"
    fi
    return "$status"
}

step_s1() {
    local log=$LOG_DIR/s1.log
    spike_env
    if ! pgrep -f SVPManager > /dev/null 2>&1; then
        record s1 SKIPPED "SVP 4 Manager is not running"
        return 0
    fi
    # SVP's filter runs inside this libmpv, which loads VSScript at run time:
    # point it at SVP's runtime, as for S2.
    local vsscript video
    vsscript=$(find_vsscript)
    video=$(s1_video)
    VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        "$REPO/spikes/target/release/s1-svp-attach" --input "$video" --seconds 60 > "$log" 2>&1
    # On Linux the spike crashed on exit once SVP's filter was active (goal D).
    echo "exit_status=$?" >> "$log"
    if grep -q '^result=PASS' "$log"; then
        record s1 PASS "$(grep -E '^(svp_filter_seen_after_s|best_fps_ratio)=' "$log" | tr '\n' ' ')"
    else
        record s1 FAIL "$(grep -E '^(svp_filter_seen_after_s|best_fps_ratio)=' "$log" | tr '\n' ' ')"
    fi
}

rvp_bin() { echo "$REPO/target/release/rvp"; }

step_m2build() {
    local log=$LOG_DIR/m2build.log
    (
        cd "$REPO" || exit 1
        export QMAKE MPV_LIB_DIR
        QMAKE=$(qt_dir)/bin/qmake
        MPV_LIB_DIR=$(mpv_lib_dir)
        cargo build --release -p rvp-shell
    ) > "$log" 2>&1
    if [ -x "$(rvp_bin)" ]; then
        record m2build PASS "rvp built"
    else
        record m2build FAIL "see $log"
        return 1
    fi
}

# RVP's settings and state for kit runs live in the log directory; the
# user's own ~/.config/rvp and ~/.local/state/rvp are not touched.
m2_env() {
    spike_env
    export RVP_CONFIG_DIR=$LOG_DIR/m2-config RVP_STATE_DIR=$LOG_DIR/m2-state
}

m2_ready() {
    local step=$1
    if [ ! -x "$(rvp_bin)" ]; then
        record "$step" SKIPPED "run m2build first"
        return 1
    fi
    if [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; then
        record "$step" SKIPPED "no graphical session"
        return 1
    fi
}

# RVP's settings for the steps that measure RVP alone (goal H): with
# "ipc-server" empty a running SVP 4 Manager cannot attach, as in the
# Windows host runs, so the dropped-frame counts compare. Only m2svp lets
# SVP attach.
kit_settings() {
    mkdir -p "$RVP_CONFIG_DIR"
    printf '{\n  "ipc-server": ""%s\n}\n' "${1:+,
  $1}" > "$RVP_CONFIG_DIR/settings.json"
}

# Hardware decoding where a GPU decodes; the VM's virtual GPU does not.
m2_hwdec() {
    if gpu_checks; then echo auto-copy; else echo no; fi
}

step_m2() {
    local log=$LOG_DIR/m2.log out=$LOG_DIR/m2-grabs functional played resumed time summary
    m2_ready m2 || return 0
    m2_env
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR" "$out"
    kit_settings
    : > "$log"
    # Functional check with two files (next and previous).
    echo "== functional" >> "$log"
    "$(rvp_bin)" --selftest "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" >> "$log" 2>&1
    functional=$?
    # Resume: play a few seconds, exit, start again without files.
    rm -rf "$RVP_STATE_DIR"
    echo "== play before restart" >> "$log"
    "$(rvp_bin)" --selftest=playback --play-seconds=6 "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(s1_video)" >> "$log" 2>&1
    played=$?
    time=$(grep '^status=' "$log" | tail -1 | grep -o '"time":[0-9.]*' | cut -d: -f2)
    echo "== restart (expect $time s)" >> "$log"
    "$(rvp_bin)" --selftest=resume "--expect-time=${time:-0}" "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" >> "$log" 2>&1
    resumed=$?
    summary="functional=$functional played=$played resumed=$resumed $(grep -E '^metric ' "$log" | sed 's/^metric //' | tr '\n' ' ')"
    if [ "$functional" -eq 0 ] && [ "$played" -eq 0 ] && [ "$resumed" -eq 0 ]; then
        record m2 PASS "$summary"
    else
        record m2 FAIL "$summary $(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m2input() {
    local log=$LOG_DIR/m2input.log screen
    m2_ready m2input || return 0
    screen=$(input_ready m2input "$LOG_DIR/m2.log") || return 0
    m2_env
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
    mkdir -p "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
    # A 900x560 window that keeps its size when the video opens.
    kit_settings '"fit-window-to-video": false'
    printf '{ "window": { "x": null, "y": null, "width": 900, "height": 560, "maximized": false } }\n' \
        > "$RVP_STATE_DIR/state.json"
    # shellcheck disable=SC2086 # "WIDTH HEIGHT"
    run_input_check "$log" --rvp "$(rvp_bin)" "${RVP_SKIN:-}" $screen "$LOG_DIR/m2input-app.log" "$(s1_video)"
    local status=$?
    if [ "$status" -eq 0 ]; then
        record m2input PASS "$(grep -E '^input_check' "$log" | tr '\n' ' ')"
    else
        record m2input FAIL "$(grep -E '^(input_check|window_origin)' "$log" | tr '\n' ' ')"
    fi
    return "$status"
}

step_m2gpu() {
    local log=$LOG_DIR/m2gpu.log out=$LOG_DIR/m2-grabs seconds=${RVP_M2_SECONDS:-25}
    m2_ready m2gpu || return 0
    if ! gpu_checks; then
        record m2gpu SKIPPED "no hardware GPU"
        return 0
    fi
    m2_env
    : > "$log"
    local mode status=0 detail=""
    for mode in fullscreen window; do
        echo "== $mode" >> "$log"
        rm -rf "$RVP_STATE_DIR"
        local extra=()
        [ "$mode" = fullscreen ] && extra+=(--fullscreen)
        # A 1920x1080 window, as goal D measured.
        mkdir -p "$RVP_STATE_DIR" "$RVP_CONFIG_DIR"
        printf '{ "window": { "x": null, "y": null, "width": 1920, "height": 1080, "maximized": false } }\n' \
            > "$RVP_STATE_DIR/state.json"
        kit_settings '"fit-window-to-video": false'
        "$(rvp_bin)" --selftest=playback "--play-seconds=$seconds" --gpu-checks "${extra[@]}" \
            "--out=$out" "--skin=${RVP_SKIN:-}" --mpv-options=hwdec=auto-copy "$(test_video)" >> "$log" 2>&1 ||
            status=1
        detail="$detail $mode: $(grep '^frames_dropped=' "$log" | tail -1)"
    done
    if [ "$status" -eq 0 ]; then
        record m2gpu PASS "$detail"
    else
        record m2gpu FAIL "$detail"
        return 1
    fi
}

step_m2svp() {
    local log=$LOG_DIR/m2svp.log out=$LOG_DIR/m2-grabs vsscript status
    m2_ready m2svp || return 0
    if ! pgrep -f SVPManager > /dev/null 2>&1; then
        record m2svp SKIPPED "SVP 4 Manager is not running"
        return 0
    fi
    m2_env
    rm -rf "$RVP_STATE_DIR"
    vsscript=$(find_vsscript)
    VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        "$(rvp_bin)" --selftest=playback --play-seconds=40 --svp "--out=$out" "--skin=${RVP_SKIN:-}" \
        --mpv-options=hwdec=auto-copy "$(s1_video)" > "$log" 2>&1
    # The exit status shows whether RVP shut down cleanly with SVP's filter
    # active (goal D rule 6).
    status=$?
    echo "exit_status=$status" >> "$log"
    if [ "$status" -eq 0 ]; then
        record m2svp PASS "$(grep -E 'svp_filter_attached|^frames_dropped=' "$log" | tr '\n' ' ') exit 0"
    else
        record m2svp FAIL "exit $status $(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m3() {
    local log=$LOG_DIR/m3.log out=$LOG_DIR/m3-grabs run_log functional pid time
    local kills=0 resumed=0 run wait_s summary
    m2_ready m3 || return 0
    m2_env
    export RVP_CONFIG_DIR=$LOG_DIR/m3-config RVP_STATE_DIR=$LOG_DIR/m3-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR" "$out"
    kit_settings
    : > "$log"
    echo "== functional (M3)" >> "$log"
    "$(rvp_bin)" --selftest --instance=kit-m3 "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" >> "$log" 2>&1
    functional=$?
    # Kill tests: the second entry of an album plays, RVP is killed with
    # SIGKILL at a different point each time and must reopen that entry
    # paused within 5 s of the last position it reported.
    for run in 1 2 3; do
        rm -rf "$RVP_STATE_DIR"
        run_log=$LOG_DIR/m3-kill-$run.log
        wait_s=$((8 + run * 4))
        "$(rvp_bin)" --selftest=playback --next --play-seconds=90 "--out=$out" \
            "--skin=${RVP_SKIN:-}" "--mpv-options=hwdec=$(m2_hwdec)" \
            "$(test_video)" "$(s1_video)" > "$run_log" 2>&1 &
        pid=$!
        sleep "$wait_s"
        kill -9 "$pid" 2> /dev/null
        wait "$pid" 2> /dev/null
        time=$(grep '^time=' "$run_log" | tail -1 | sed 's/^time=\([0-9.]*\).*/\1/')
        {
            echo "== kill run $run after $wait_s s, last $(grep '^time=' "$run_log" | tail -1)"
            echo "== restart (expect $time s, entry 1 of 2)"
        } >> "$log"
        "$(rvp_bin)" --selftest=resume "--expect-time=${time:-0}" --expect-tolerance=5 \
            --expect-entry=1 --expect-entries=2 "--out=$out" "--skin=${RVP_SKIN:-}" \
            "--mpv-options=hwdec=$(m2_hwdec)" >> "$log" 2>&1 && resumed=$((resumed + 1))
        kills=$((kills + 1))
    done
    summary="functional=$functional kills=$kills resumed=$resumed $(grep -E '^metric ' "$log" | sed 's/^metric //' | tr '\n' ' ')"
    if [ "$functional" -eq 0 ] && [ "$resumed" -eq "$kills" ]; then
        record m3 PASS "$summary"
    else
        record m3 FAIL "$summary $(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m4() {
    local log=$LOG_DIR/m4.log out=$LOG_DIR/m4-grabs portable=$LOG_DIR/m4-portable
    local functional checks missing=0 name portable_ok summary
    m2_ready m4 || return 0
    m2_env
    export RVP_CONFIG_DIR=$LOG_DIR/m4-config RVP_STATE_DIR=$LOG_DIR/m4-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR" "$out" "$portable"
    kit_settings
    : > "$log"
    echo "== functional (M4 and M5 checks)" >> "$log"
    "$(rvp_bin)" --selftest "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" >> "$log" 2>&1
    functional=$?
    for name in context_menu_keys context_menu_marks playlist_menu_keys context_menu_builds \
        menu_command_runs f5_opens_settings settings_pages_load keys_page_rows \
        setting_applies_at_once settings_window_closes; do
        grep -q "^check $name=PASS" "$log" || missing=$((missing + 1))
    done
    checks=$(grep -c '^check .*=PASS' "$log")
    # Portable mode: a copy with the marker keeps settings.json next to it.
    echo "== portable" >> "$log"
    mkdir -p "$portable"
    cp "$(rvp_bin)" "$portable/rvp"
    : > "$portable/rvp.portable"
    (
        unset RVP_CONFIG_DIR RVP_STATE_DIR
        # The copy writes its own settings.json, so SVP is kept out through
        # mpv's option instead.
        "$portable/rvp" --selftest=playback --play-seconds=3 "--out=$out" \
            "--skin=${RVP_SKIN:-}" "--mpv-options=hwdec=$(m2_hwdec),input-ipc-server=" "$(s1_video)"
    ) >> "$log" 2>&1
    if [ -s "$portable/settings.json" ] && [ -d "$portable/state" ]; then
        portable_ok=1
    else
        portable_ok=0
    fi
    summary="functional=$functional passed=$checks m4_missing=$missing portable=$portable_ok"
    if [ "$functional" -eq 0 ] && [ "$missing" -eq 0 ] && [ "$portable_ok" -eq 1 ]; then
        record m4 PASS "$summary"
    else
        record m4 FAIL "$summary $(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m6() {
    local log=$LOG_DIR/m6.log out=$LOG_DIR/m6-grabs functional upright speed vpy tests vsscript
    local missing=0 name summary
    m2_ready m6 || return 0
    m2_env
    export RVP_CONFIG_DIR=$LOG_DIR/m6-config RVP_STATE_DIR=$LOG_DIR/m6-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR" "$out"
    kit_settings
    : > "$log"
    echo "== functional (M6 checks)" >> "$log"
    "$(rvp_bin)" --selftest "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" >> "$log" 2>&1
    functional=$?
    for name in f7_opens_panel color_slider_applies color_reset_applies equalizer_slider_applies \
        equalizer_button_toggles f7_closes_panel shift_p_shaders_on shift_p_shaders_off \
        svp_diagnostic_shows svp_mode_off; do
        grep -q "^check $name=PASS" "$log" || missing=$((missing + 1))
    done
    # The picture is upright: a clip with a mark at the top left against
    # ffmpeg's decoding of it.
    echo "== upright" >> "$log"
    rm -rf "$RVP_STATE_DIR"
    kit_settings '"fit-window-to-video": false'
    "$(rvp_bin)" --selftest=upright "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$REPO/testdata/media/fixture-marker.mkv" >> "$log" 2>&1
    upright=$?
    # Goal H's carry-over: 1.1x and back to 1x one second into the
    # film-rate clip (a moment at which the normalizer used to leave audio
    # and video 0.6 s apart); the frames dropped afterwards.
    echo "== speed" >> "$log"
    rm -rf "$RVP_STATE_DIR"
    kit_settings
    "$(rvp_bin)" --selftest=speed --speed-at=1 "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(s1_video)" >> "$log" 2>&1
    speed=$?
    # A script slot in RVP with SVP's VapourSynth (SKIPPED without it):
    # RVP's own PotPlayer-style script, then PotPlayer's when RVP_VPY names
    # it (with the kit's mvtools).
    vsscript=$(find_vsscript)
    echo "== vpy" >> "$log"
    rm -rf "$RVP_STATE_DIR"
    kit_settings
    VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        "$(rvp_bin)" --selftest=vpy "--out=$out" "--skin=${RVP_SKIN:-}" \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(s1_video)" >> "$log" 2>&1
    vpy=$?
    if [ -n "${RVP_VPY:-}" ] && [ -e "$RVP_VPY" ] && [ -s "$(mvtools_lib)" ]; then
        echo "== vpy (PotPlayer's script)" >> "$log"
        rm -rf "$RVP_STATE_DIR"
        VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
            "$(rvp_bin)" --selftest=vpy "--vpy=$RVP_VPY" "--vpy-plugins=$(mvtools_lib)" \
            "--out=$out" "--skin=${RVP_SKIN:-}" "--mpv-options=hwdec=$(m2_hwdec)" \
            "$(s1_video)" >> "$log" 2>&1 || vpy=1
    fi
    # The headless video tests; the script slot test runs with SVP's
    # VapourSynth when it is installed.
    echo "== cargo test (video, equalizer)" >> "$log"
    (
        cd "$REPO" || exit 1
        export QMAKE MPV_LIB_DIR
        QMAKE=$(qt_dir)/bin/qmake
        MPV_LIB_DIR=$(mpv_lib_dir)
        VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
            cargo test --release -p rvp-app --test video --test equalizer -- --nocapture
    ) >> "$log" 2>&1
    tests=$?
    summary="functional=$functional m6_missing=$missing upright=$upright speed=$speed vpy=$vpy tests=$tests vsscript=${vsscript:-none} $(grep -E '^(check (picture_upright|vpy_slot_runs)|speed_back_drops)' "$log" | tr '\n' ' ')"
    if [ "$functional" -eq 0 ] && [ "$missing" -eq 0 ] && [ "$upright" -eq 0 ] && [ "$speed" -eq 0 ] &&
        [ "$vpy" -eq 0 ] && [ "$tests" -eq 0 ]; then
        record m6 PASS "$summary"
    else
        record m6 FAIL "$summary $(grep -E '=FAIL|FAILED|panicked' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m7() {
    local log=$LOG_DIR/m7.log out=$LOG_DIR/m7-grabs functional upright missing=0 name summary
    m2_ready m7 || return 0
    m2_env
    export RVP_CONFIG_DIR=$LOG_DIR/m7-config RVP_STATE_DIR=$LOG_DIR/m7-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR" "$out"
    kit_settings
    : > "$log"
    echo "== functional (RVP Default)" >> "$log"
    "$(rvp_bin)" --selftest "--out=$out" --skin= \
        "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" >> "$log" 2>&1
    functional=$?
    # The picture with the pointer over it, in the window and in fullscreen
    # with and without the bars (the bars once hid it with black).
    echo "== upright (RVP Default)" >> "$log"
    rm -rf "$RVP_STATE_DIR"
    kit_settings '"fit-window-to-video": false'
    "$(rvp_bin)" --selftest=upright "--out=$out" --skin= \
        "--mpv-options=hwdec=$(m2_hwdec)" "$REPO/testdata/media/fixture-marker.mkv" >> "$log" 2>&1
    upright=$?
    for name in logo_resources tab_click_shows_album f7_opens_panel color_slider_applies \
        color_reset_applies equalizer_slider_applies equalizer_button_toggles f7_closes_panel \
        picture_upright picture_with_pointer picture_fullscreen picture_fullscreen_bars; do
        grep -q "^check $name=PASS" "$log" || missing=$((missing + 1))
    done
    summary="functional=$functional upright=$upright m7_missing=$missing $(grep -E '^check (f7_opens_panel|picture_fullscreen_bars)=' "$log" | tr '\n' ' ')"
    if [ "$functional" -eq 0 ] && [ "$upright" -eq 0 ] && [ "$missing" -eq 0 ]; then
        record m7 PASS "$summary"
    else
        record m7 FAIL "$summary $(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

# The packages m8package makes.
m8_dir() { echo "$LOG_DIR/m8-packages"; }

step_m8package() {
    local log=$LOG_DIR/m8package.log out runtime url
    out=$(m8_dir)
    runtime=$KIT_DIR/appimage-runtime-x86_64
    url=$(sed -n 's/^    "\(https:.*runtime-x86_64\)"$/\1/p' "$REPO/tools/packaging/package.py")
    (
        set -e
        cd "$REPO"
        export QMAKE MPV_LIB_DIR CARGO_TARGET_DIR RUSTFLAGS
        QMAKE=$(qt_dir)/bin/qmake
        MPV_LIB_DIR=$(mpv_lib_dir)
        # Its own build: the run path finds the bundled libraries without
        # LD_LIBRARY_PATH (DT_RPATH, which libmpv's own dependencies use).
        CARGO_TARGET_DIR=$REPO/target/package
        RUSTFLAGS='-C link-arg=-Wl,-rpath,$ORIGIN/../lib -C link-arg=-Wl,--disable-new-dtags'
        cargo build --release -p rvp-shell
        rm -rf "$out"
        mkdir -p "$out"
        cargo xtask look "$out/look" > /dev/null
        [ -s "$runtime" ] || curl -fsSL "$url" -o "$runtime"
        python3 tools/packaging/package.py linux --exe "$CARGO_TARGET_DIR/release/rvp" \
            --qt "$(qt_dir)" --mpv "$(mpv_lib_dir)" --look "$out/look" --out "$out" \
            --runtime "$runtime"
    ) > "$log" 2>&1
    local appimage deb
    appimage=$(ls "$out"/RVP-*-x86_64.AppImage 2> /dev/null | head -1)
    deb=$(ls "$out"/rvp_*_amd64.deb 2> /dev/null | head -1)
    if [ -n "$appimage" ] && [ -n "$deb" ]; then
        record m8package PASS "$(basename "$appimage") $(du -h "$appimage" | cut -f1), $(basename "$deb") $(du -h "$deb" | cut -f1)"
    else
        record m8package FAIL "see $log"
        return 1
    fi
}

# The bus name of the RVP with process `pid` on the session bus, if any.
m8_mpris_name() {
    local pid=$1 name owner
    for name in "org.mpris.MediaPlayer2.rvp" "org.mpris.MediaPlayer2.rvp.instance$pid"; do
        owner=$(gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.GetConnectionUnixProcessID "$name" 2> /dev/null)
        case $owner in *" $pid,"*) echo "$name"; return ;; esac
    done
}

m8_get() {
    gdbus call --session --dest "$1" --object-path /org/mpris/MediaPlayer2 \
        --method org.freedesktop.DBus.Properties.Get org.mpris.MediaPlayer2.Player "$2" 2> /dev/null
}

m8_idle_inhibited() {
    gdbus call --session --dest org.gnome.SessionManager --object-path /org/gnome/SessionManager \
        --method org.gnome.SessionManager.IsInhibited 8 2> /dev/null
}

# MPRIS and idle inhibit while `rvp` plays the test video on its own
# instance channel; prints check lines.
m8_mpris() {
    local rvp=$1 pid name status before during paused deadline
    before=$(m8_idle_inhibited)
    "$rvp" --instance=m8 --skin= "--mpv-options=hwdec=$(m2_hwdec),ao=null" "$(test_video)" \
        > "$LOG_DIR/m8-mpris-rvp.log" 2>&1 &
    pid=$!
    deadline=$((SECONDS + 40))
    while [ $SECONDS -lt $deadline ]; do
        name=$(m8_mpris_name "$pid")
        status=${name:+$(m8_get "$name" PlaybackStatus)}
        case $status in *Playing*) break ;; esac
        sleep 0.5
    done
    case $status in
        *Playing*) echo "check mpris_playing=PASS $name" ;;
        *) echo "check mpris_playing=FAIL name '$name' status '$status'" ;;
    esac
    if [ -n "$name" ]; then
        case $(m8_get "$name" Metadata) in
            *xesam:title*) echo "check mpris_metadata=PASS" ;;
            *) echo "check mpris_metadata=FAIL" ;;
        esac
        sleep 2
        during=$(m8_idle_inhibited)
        gdbus call --session --dest "$name" --object-path /org/mpris/MediaPlayer2 \
            --method org.mpris.MediaPlayer2.Player.Pause 2>&1
        deadline=$((SECONDS + 10))
        while [ $SECONDS -lt $deadline ]; do
            status=$(m8_get "$name" PlaybackStatus)
            case $status in *Paused*) break ;; esac
            sleep 0.5
        done
        case $status in
            *Paused*) echo "check mpris_pause=PASS" ;;
            *) echo "check mpris_pause=FAIL status '$status'" ;;
        esac
        paused=$(m8_idle_inhibited)
        if [ -z "$during" ]; then
            echo "check idle_inhibit=SKIPPED no GNOME session manager on this bus"
        elif [ "$during" = "(true,)" ] && { [ "$before" != "(false,)" ] || [ "$paused" = "(false,)" ]; }; then
            echo "check idle_inhibit=PASS before $before playing $during paused $paused"
        else
            echo "check idle_inhibit=FAIL before $before playing $during paused $paused"
        fi
        gdbus call --session --dest "$name" --object-path /org/mpris/MediaPlayer2 \
            --method org.mpris.MediaPlayer2.Quit > /dev/null 2>&1
    fi
    deadline=$((SECONDS + 20))
    while kill -0 "$pid" 2> /dev/null && [ $SECONDS -lt $deadline ]; do sleep 0.5; done
    if kill -0 "$pid" 2> /dev/null; then
        echo "check mpris_quit=FAIL"
        kill "$pid" 2> /dev/null
    else
        echo "check mpris_quit=PASS"
    fi
}

# The crash logs of `rvp`: a panic and a fatal signal.
m8_crash() {
    local rvp=$1 kind text
    for kind in panic native; do
        rm -rf "$RVP_STATE_DIR"
        "$rvp" "--crash-test=$kind" > /dev/null 2>&1
        text=$(cat "$RVP_STATE_DIR"/crash-*.log 2> /dev/null)
        case $kind:$text in
            panic:*"panic in thread 'main'"*) echo "check crash_log_panic=PASS" ;;
            native:*"fatal signal 11"*) echo "check crash_log_native=PASS" ;;
            *) echo "check crash_log_$kind=FAIL $text" ;;
        esac
    done
}

step_m8() {
    local log=$LOG_DIR/m8.log out appimage deb installed=0 default_before bus=""
    out=$(m8_dir)
    appimage=$(ls "$out"/RVP-*-x86_64.AppImage 2> /dev/null | head -1)
    deb=$(ls "$out"/rvp_*_amd64.deb 2> /dev/null | head -1)
    if [ -z "$appimage" ] || [ -z "$deb" ]; then
        record m8 SKIPPED "run m8package first"
        return 0
    fi
    if [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; then
        record m8 SKIPPED "no graphical session"
        return 0
    fi
    # The packages bring their own Qt and libmpv.
    unset LD_LIBRARY_PATH
    export RVP_CONFIG_DIR=$LOG_DIR/m8-config RVP_STATE_DIR=$LOG_DIR/m8-state
    : > "$log"
    {
        echo "== AppImage $(basename "$appimage")"
        rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
        kit_settings
        chmod +x "$appimage"
        if "$appimage" --selftest "--out=$LOG_DIR/m8-grabs" --skin= \
            "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" > "$LOG_DIR/m8-appimage.log" 2>&1; then
            echo "check appimage_selftest=PASS"
        else
            echo "check appimage_selftest=FAIL $(grep -E '=FAIL|^failures=' "$LOG_DIR/m8-appimage.log" | tr '\n' ' ')"
        fi
        echo "== .deb $(basename "$deb")"
        default_before=$(xdg-mime query default video/x-matroska 2> /dev/null)
        if dpkg -s rvp > /dev/null 2>&1; then
            echo "check deb_installed=PASS installed before this step"
        elif sudo -n true 2> /dev/null; then
            if sudo -n apt-get install -y "$deb" > "$LOG_DIR/m8-apt.log" 2>&1; then
                installed=1
                echo "check deb_installed=PASS"
            else
                echo "check deb_installed=FAIL see $LOG_DIR/m8-apt.log"
            fi
        else
            echo "check deb_installed=SKIPPED install it first: sudo apt install $deb"
        fi
        if dpkg -s rvp > /dev/null 2>&1; then
            [ "$(readlink /usr/bin/rvp)" = /opt/rvp/bin/rvp ] && [ -x /opt/rvp/bin/rvp ] &&
                echo "check deb_files=PASS" || echo "check deb_files=FAIL"
            if command -v desktop-file-validate > /dev/null 2>&1; then
                desktop-file-validate /usr/share/applications/rvp.desktop > /dev/null 2>&1 &&
                    echo "check desktop_entry=PASS" ||
                    echo "check desktop_entry=FAIL $(desktop-file-validate /usr/share/applications/rvp.desktop 2>&1 | head -3 | tr '\n' ' ')"
            fi
            gio mime video/x-matroska 2> /dev/null | grep -q 'rvp\.desktop' &&
                echo "check registered_mkv=PASS" || echo "check registered_mkv=FAIL"
            [ "$(xdg-mime query default video/x-matroska 2> /dev/null)" = "$default_before" ] &&
                echo "check default_unchanged=PASS $default_before" ||
                echo "check default_unchanged=FAIL $default_before"
            rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
            kit_settings
            if /usr/bin/rvp --selftest "--out=$LOG_DIR/m8-grabs" --skin= \
                "--mpv-options=hwdec=$(m2_hwdec)" "$(test_video)" "$(s1_video)" > "$LOG_DIR/m8-deb.log" 2>&1; then
                echo "check deb_selftest=PASS"
            else
                echo "check deb_selftest=FAIL $(grep -E '=FAIL|^failures=' "$LOG_DIR/m8-deb.log" | tr '\n' ' ')"
            fi
            # MPRIS on the desktop's session bus, or on a private one.
            if ! gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
                --method org.freedesktop.DBus.ListNames > /dev/null 2>&1; then
                bus=$(dbus-daemon --session --fork --print-address=1 --print-pid=1 2> /dev/null)
                export DBUS_SESSION_BUS_ADDRESS=${bus%%$'\n'*}
                echo "(a private session bus: $DBUS_SESSION_BUS_ADDRESS)"
            fi
            m8_mpris /usr/bin/rvp
            if [ -n "$bus" ]; then
                kill "${bus##*$'\n'}" 2> /dev/null
                unset DBUS_SESSION_BUS_ADDRESS
            fi
            m8_crash /usr/bin/rvp
        fi
        if [ "$installed" = 1 ]; then
            sudo -n apt-get remove -y rvp >> "$LOG_DIR/m8-apt.log" 2>&1
            if [ ! -e /opt/rvp ] && [ ! -e /usr/bin/rvp ] && [ ! -e /usr/share/applications/rvp.desktop ] &&
                [ -z "$(find /usr/share/icons -name 'rvp.*' 2> /dev/null)" ]; then
                echo "check deb_removed=PASS"
            else
                echo "check deb_removed=FAIL"
            fi
            [ -d "$RVP_STATE_DIR" ] && echo "check user_state_kept=PASS" || echo "check user_state_kept=FAIL"
        fi
    } >> "$log" 2>&1
    local failed passed
    failed=$(grep -c '=FAIL' "$log")
    passed=$(grep -c '=PASS' "$log")
    if [ "$failed" -eq 0 ]; then
        record m8 PASS "$passed checks passed $(grep -E '=SKIPPED' "$log" | tr '\n' ' ')"
    else
        record m8 FAIL "$(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m8removed() {
    local left
    left=$( { ls -d /opt/rvp /usr/bin/rvp /usr/share/applications/rvp.desktop 2> /dev/null
        find /usr/share/icons -name 'rvp.*' 2> /dev/null; } | tr '\n' ' ')
    if dpkg -s rvp > /dev/null 2>&1; then
        record m8removed FAIL "rvp is still installed: sudo apt remove rvp"
        return 1
    elif [ -n "$left" ]; then
        record m8removed FAIL "left: $left"
        return 1
    elif [ -d "$LOG_DIR/m8-state" ]; then
        record m8removed PASS "nothing of the package left, RVP's state kept"
    else
        record m8removed PASS "nothing of the package left"
    fi
}

step_m6svp() {
    local log=$LOG_DIR/m6svp.log out=$LOG_DIR/m6-grabs vsscript first second pid
    m2_ready m6svp || return 0
    if ! pgrep -f SVPManager > /dev/null 2>&1; then
        record m6svp SKIPPED "SVP 4 Manager is not running"
        return 0
    fi
    m2_env
    export RVP_CONFIG_DIR=$LOG_DIR/m6svp-config RVP_STATE_DIR=$LOG_DIR/m6svp-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
    # SVP mode on RVP's default endpoint (/tmp/mpvsocket).
    kit_settings
    printf '{\n  "ipc-server": "/tmp/mpvsocket"\n}\n' > "$RVP_CONFIG_DIR/settings.json"
    vsscript=$(find_vsscript)
    : > "$log"
    echo "== first window" >> "$log"
    VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        "$(rvp_bin)" --selftest=svp "--out=$out" "--skin=${RVP_SKIN:-}" \
        --mpv-options=hwdec=auto-copy "$(s1_video)" > "$LOG_DIR/m6svp-first.log" 2>&1 &
    pid=$!
    # The second window starts once the first holds the endpoint.
    sleep 8
    # Its own settings and state: two processes must not share them.
    echo "== second window (/new)" >> "$log"
    mkdir -p "$LOG_DIR/m6svp-config2"
    cp "$RVP_CONFIG_DIR/settings.json" "$LOG_DIR/m6svp-config2/"
    rm -rf "$LOG_DIR/m6svp-state2"
    RVP_CONFIG_DIR=$LOG_DIR/m6svp-config2 RVP_STATE_DIR=$LOG_DIR/m6svp-state2 \
        VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        "$(rvp_bin)" /new --selftest=svp "--out=$out" "--skin=${RVP_SKIN:-}" \
        --mpv-options=hwdec=auto-copy "$(s1_video)" > "$LOG_DIR/m6svp-second.log" 2>&1
    second=$?
    wait "$pid"
    first=$?
    cat "$LOG_DIR/m6svp-first.log" >> "$log"
    echo "== second window log" >> "$log"
    cat "$LOG_DIR/m6svp-second.log" >> "$log"
    local detail
    detail="first=$first second=$second $(grep -hE '^svp_endpoint=|^check svp_(attached|target_fps)' "$LOG_DIR/m6svp-first.log" "$LOG_DIR/m6svp-second.log" | tr '\n' ' ')"
    if [ "$first" -eq 0 ] && [ "$second" -eq 0 ]; then
        record m6svp PASS "$detail"
    else
        record m6svp FAIL "$detail $(grep -hE '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m6gpu() {
    local log=$LOG_DIR/m6gpu.log out=$LOG_DIR/m6-grabs seconds=${RVP_M2_SECONDS:-25} shader
    m2_ready m6gpu || return 0
    if ! gpu_checks; then
        record m6gpu SKIPPED "no hardware GPU"
        return 0
    fi
    m2_env
    export RVP_CONFIG_DIR=$LOG_DIR/m6gpu-config RVP_STATE_DIR=$LOG_DIR/m6gpu-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
    mkdir -p "$RVP_CONFIG_DIR"
    shader=$RVP_CONFIG_DIR/identity.glsl
    printf '//!HOOK MAIN\n//!BIND HOOKED\n//!DESC RVP kit (no change)\nvec4 hook() { return HOOKED_texOff(0); }\n' > "$shader"
    kit_settings '"color": { "enabled": true, "brightness": 4, "saturation": 6 },
  "deband": true,
  "shaders": { "enabled": true, "files": ["'"$shader"'"] },
  "equalizer": { "enabled": true, "preset": "Rock", "bands": [5, 3, -3.5, -5, -2, 2.5, 5.5, 7, 7, 7] }'
    "$(rvp_bin)" --selftest=playback "--play-seconds=$seconds" --gpu-checks --fullscreen \
        "--out=$out" "--skin=${RVP_SKIN:-}" --mpv-options=hwdec=auto-copy "$(test_video)" > "$log" 2>&1
    local status=$?
    local detail
    detail="$(grep '^frames_dropped=' "$log" | tail -1) $(grep -o '"glsl_shaders":"[^"]*"' "$log" | tail -1)"
    if [ "$status" -eq 0 ]; then
        record m6gpu PASS "$detail"
    else
        record m6gpu FAIL "$detail $(grep -E '=FAIL' "$log" | tr '\n' ' ')"
        return 1
    fi
}

step_m3desktop() {
    local file=$HOME/.local/share/applications/rvp-kit.desktop
    if [ ! -x "$(rvp_bin)" ]; then
        record m3desktop SKIPPED "run m2build first"
        return 0
    fi
    mkdir -p "$(dirname "$file")"
    {
        echo "[Desktop Entry]"
        echo "Type=Application"
        echo "Name=RVP (kit)"
        echo "Comment=RVP test build from $REPO"
        echo "Exec=env LD_LIBRARY_PATH=$(mpv_lib_dir):$(qt_dir)/lib ${RVP_SKIN:+RVP_SKIN=$RVP_SKIN} $(rvp_bin) %F"
        echo "Terminal=false"
        echo "Categories=AudioVideo;Video;Player;"
        echo "MimeType=video/x-matroska;video/mp4;video/webm;video/x-msvideo;video/quicktime;video/mpeg;video/mp2t;audio/flac;audio/mpeg;audio/x-wav;"
    } > "$file"
    record m3desktop PASS "$file (delete it to undo)"
}

step_report() {
    local file
    file=$REPO/docs/test-runs/$(date +%Y-%m-%d)-$(hostname).md
    mkdir -p "$(dirname "$file")"
    {
        echo "# Linux test kit run on $(hostname)"
        echo
        echo "Generated by \`tools/linux-kit/kit.sh report\` on $(date -Is)."
        echo
        echo "## System"
        echo
        echo '```text'
        cat "$LOG_DIR/check.log" 2> /dev/null
        echo '```'
        echo
        echo "## Results"
        echo
        echo "| Time | Step | Result | Detail |"
        echo "|------|------|--------|--------|"
        awk -F'\t' '{ printf "| %s | %s | %s | %s |\n", $1, $2, $3, $4 }' "$RESULTS" 2> /dev/null
        for step in s2 s3 input s1 m2 m3 m4 m6 m7 m2input m2gpu m6gpu m2svp m6svp; do
            [ -s "$LOG_DIR/$step.log" ] || continue
            echo
            echo "## $step output"
            echo
            echo '```text'
            grep -v -E '^(windows=|grabbed=|\[libmpv_render\] Loaded extension)' "$LOG_DIR/$step.log" | tail -60
            echo '```'
        done
    } > "$file"
    record report PASS "$file"
}

if [ "$#" -eq 0 ]; then
    sed -n '2,102p' "$KIT_SCRIPT"
    exit 2
fi
for step in "$@"; do
    case $step in
        all) set -- check deps mpv mvtools build s2 s3 input s1 report ;;
        m2all) set -- check deps mpv m2build m2 m2input m2gpu m2svp report ;;
        m3all) set -- check deps mpv m2build m2 m3 m2input m2gpu m2svp report ;;
        m4all) set -- check deps mpv m2build m2 m3 m4 m2input m2gpu m2svp report ;;
        m6all) set -- check deps mpv mvtools m2build m2 m3 m4 m6 m2input m2gpu m6gpu m2svp m6svp report ;;
        m7all) set -- check deps mpv mvtools m2build m2 m3 m4 m6 m7 m2input m2gpu m6gpu m2svp m6svp report ;;
        m8all) set -- check deps mpv mvtools m2build m2 m3 m4 m6 m7 m8package m8 m2input m2gpu m6gpu m2svp m6svp report ;;
    esac
done
overall=0
for step in "$@"; do
    case $step in
        check | deps | mpv | mvtools | build | s1 | s2 | s3 | input | m2build | m2 | m2input | m2gpu | m2svp | m3 | m4 | m6 | m7 | m8package | m8 | m8removed | m6svp | m6gpu | m3desktop | report)
            "step_$step" || overall=1
            ;;
        all | m2all | m3all | m4all | m6all | m7all | m8all) ;;
        *) say "unknown step $step"; exit 2 ;;
    esac
done
exit "$overall"
