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
#   build   build the spikes
#   s2      VapourSynth interpolation through libmpv
#   s3      Qt shell self-test in the current graphical session
#   input   drag the frameless S3 window with injected mouse input (needs
#           ydotool and ydotoold running as root; optional)
#   s1      SVP 4 Manager attaches to a libmpv player (start SVP first)
#   report  write docs/test-runs/<date>-<host>.md from the logs
#   all     check deps mpv build s2 s3 input s1 report
#
# environment:
#   RVP_KIT_DIR     build and download directory (default ~/.cache/rvp-kit)
#   RVP_LOG_DIR     logs (default $RVP_KIT_DIR/logs)
#   RVP_VIDEO       test video for S1 and S3 (default: generated)
#   RVP_SKIN        .dsf skin for S3 (default: none, the shell paints nothing)
#   RVP_VPY         motion interpolation script for S2
#   VSSCRIPT_PATH   libvapoursynth-script.so to use (default: searched)
#   RVP_VS_PLUGINS  extra VapourSynth plugins for S2, ':' separated
#   RVP_GPU_CHECKS  1 or 0 to override GPU detection
#   RVP_KIT_REBUILD 1 to rebuild libmpv even if this commit is already built
set -u

KIT_SCRIPT=$(readlink -f "${BASH_SOURCE[0]}")
REPO=$(cd "$(dirname "$KIT_SCRIPT")/../.." && pwd)
KIT_DIR=${RVP_KIT_DIR:-$HOME/.cache/rvp-kit}
LOG_DIR=${RVP_LOG_DIR:-$KIT_DIR/logs}
QT_VERSION=6.10.3
MPV_COMMIT=b2c255c13e
VS_TAG=R73
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

# Packages needed to build libmpv and to run the Qt spike, by Debian name.
PACKAGES="build-essential git curl python3-venv python3-jinja2 pkg-config nasm cmake ninja-build \
autoconf automake libtool \
libass-dev libfreetype-dev libfribidi-dev libharfbuzz-dev libfontconfig-dev \
libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxss-dev libxv-dev libxpresent-dev \
libxkbcommon-dev libwayland-dev wayland-protocols libegl-dev libgl-dev libgbm-dev libdrm-dev \
libasound2-dev libpulse-dev libva-dev libvdpau-dev zlib1g-dev libssl-dev liblcms2-dev libjpeg-dev \
libvulkan-dev glslang-dev libunwind-dev \
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
            "$KIT_DIR/venv/bin/aqt" install-qt linux desktop "$QT_VERSION" linux_gcc_64 \
                --outputdir "$KIT_DIR/qt"
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
    for candidate in /opt/svp/*/libvapoursynth-script.so* "$HOME"/.local/share/SVP4/*/libvapoursynth-script.so* \
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
    local status=0
    : > "$log"
    for mode in plain --compat; do
        echo "== $mode" >> "$log"
        VSSCRIPT_PATH=$vsscript "$REPO/spikes/target/release/s2-vapoursynth" \
            --script "$RVP_VPY" --seconds 15 $([ "$mode" = --compat ] && echo --compat) >> "$log" 2>&1 || status=1
    done
    local compat_result
    compat_result=$(grep '^result=' "$log" | tail -1)
    if [ "$compat_result" = "result=PASS" ]; then
        record s2 PASS "$(grep -E '^(estimated_vf_fps_mean|realtime_ratio)=' "$log" | tail -2 | tr '\n' ' ')"
    else
        record s2 FAIL "see $log"
    fi
    return "$status"
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
        extra+=(--gpu-checks --hwdec auto-copy --video-seconds 20 --width 1920 --height 1080)
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

step_input() {
    local log=$LOG_DIR/input.log size
    spike_env
    if ! command -v ydotool > /dev/null 2>&1 || ! pgrep -x ydotoold > /dev/null 2>&1; then
        record input SKIPPED "needs ydotool 1.x and 'sudo ydotoold' running"
        return 0
    fi
    if [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; then
        record input SKIPPED "no graphical session"
        return 0
    fi
    size=$(grep -o '"width":[0-9]*,"height":[0-9]*\|"height":[0-9]*,"name":"[^"]*","width":[0-9]*' "$LOG_DIR/s3.log" 2> /dev/null | head -1)
    local width height
    width=$(echo "$size" | grep -o '"width":[0-9]*' | cut -d: -f2)
    height=$(echo "$size" | grep -o '"height":[0-9]*' | cut -d: -f2)
    if [ -z "$width" ] || [ -z "$height" ]; then
        record input SKIPPED "run s3 first (screen size comes from its log)"
        return 0
    fi
    python3 "$REPO/tools/linux-kit/input-check.py" "$REPO/spikes/target/release/s3-qt-shell" \
        "${RVP_SKIN:-}" "$width" "$height" "$LOG_DIR/input-app.log" > "$log" 2>&1
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
    "$REPO/spikes/target/release/s1-svp-attach" --input "$(test_video)" --seconds 60 > "$log" 2>&1
    if grep -q '^result=PASS' "$log"; then
        record s1 PASS "$(grep -E '^(svp_filter_seen_after_s|best_fps_ratio)=' "$log" | tr '\n' ' ')"
    else
        record s1 FAIL "$(grep -E '^(svp_filter_seen_after_s|best_fps_ratio)=' "$log" | tr '\n' ' ')"
    fi
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
        for step in s2 s3 input s1; do
            [ -s "$LOG_DIR/$step.log" ] || continue
            echo
            echo "## $step output"
            echo
            echo '```text'
            grep -v -E '^(windows=|\[libmpv_render\] Loaded extension)' "$LOG_DIR/$step.log" | tail -60
            echo '```'
        done
    } > "$file"
    record report PASS "$file"
}

if [ "$#" -eq 0 ]; then
    sed -n '2,31p' "$KIT_SCRIPT"
    exit 2
fi
for step in "$@"; do
    case $step in
        all) set -- check deps mpv build s2 s3 input s1 report ;;
    esac
done
overall=0
for step in "$@"; do
    case $step in
        check | deps | mpv | build | s1 | s2 | s3 | input | report) "step_$step" || overall=1 ;;
        all) ;;
        *) say "unknown step $step"; exit 2 ;;
    esac
done
exit "$overall"
