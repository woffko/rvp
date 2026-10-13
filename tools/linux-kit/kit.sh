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
#   m3desktop  optional, not in m3all: an "RVP (kit)" entry in
#           ~/.local/share/applications so that Files can open videos in
#           the running RVP (single instance); delete the file to undo
#   report  write docs/test-runs/<date>-<host>.md from the logs
#   all     check deps mpv mvtools build s2 s3 input s1 report
#   m2all   check deps mpv m2build m2 m2input m2gpu m2svp report
#   m3all   check deps mpv m2build m2 m3 m2input m2gpu m2svp report
#   m4all   check deps mpv m2build m2 m3 m4 m2input m2gpu m2svp report
#
# environment:
#   RVP_KIT_DIR     build and download directory (default ~/.cache/rvp-kit)
#   RVP_LOG_DIR     logs (default $RVP_KIT_DIR/logs)
#   RVP_VIDEO       test video for S3 (default: generated 4K60 or 1080p60)
#   RVP_S1_VIDEO    film-rate test video for S1 (default: generated 23.976 fps)
#   RVP_SKIN        .dsf skin for S3 (default: none, the shell paints nothing)
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
libvulkan-dev glslang-dev libunwind-dev libfftw3-dev \
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
    if [ "${RVP_KIT_REBUILD:-0}" != 1 ] && [ "$(cat "$prefix/.kit-mpv" 2> /dev/null)" = "$MPV_COMMIT" ]; then
        record mpv PASS "libmpv $MPV_COMMIT in $prefix (already built)"
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
        printf '%s\n' -Dlibmpv=true -Dcplayer=false -Dvapoursynth=enabled \
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
        echo "$MPV_COMMIT" > "$prefix/.kit-mpv"
        record mpv PASS "libmpv $MPV_COMMIT in $prefix"
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
        for step in s2 s3 input s1 m2 m3 m4 m2input m2gpu m2svp; do
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
    sed -n '2,65p' "$KIT_SCRIPT"
    exit 2
fi
for step in "$@"; do
    case $step in
        all) set -- check deps mpv mvtools build s2 s3 input s1 report ;;
        m2all) set -- check deps mpv m2build m2 m2input m2gpu m2svp report ;;
        m3all) set -- check deps mpv m2build m2 m3 m2input m2gpu m2svp report ;;
        m4all) set -- check deps mpv m2build m2 m3 m4 m2input m2gpu m2svp report ;;
    esac
done
overall=0
for step in "$@"; do
    case $step in
        check | deps | mpv | mvtools | build | s1 | s2 | s3 | input | m2build | m2 | m2input | m2gpu | m2svp | m3 | m4 | m3desktop | report)
            "step_$step" || overall=1
            ;;
        all | m2all | m3all | m4all) ;;
        *) say "unknown step $step"; exit 2 ;;
    esac
done
exit "$overall"
