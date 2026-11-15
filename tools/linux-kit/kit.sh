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
#   whisper build whisper.cpp's CPU and Vulkan engine (tools/whisper/
#           build-linux.sh) for the packages (m8package) and m13
#   vapoursynth  build RVP's own VapourSynth runtime (tools/vapoursynth/
#           build-linux.sh, after mvtools) for the packages and m18
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
#   m9      milestone M9: the M9 self-test with mpv's OSC, then with each
#           script folder of RVP_M9_SCRIPTS (uosc, ModernZ), the classic
#           self-test with each .wsz of RVP_WINAMP_SKINS, the VLC self-test
#           with each .vlt of RVP_VLC_SKINS (local samples, never
#           committed), and the skin loaders' headless tests on them
#   m10     milestone M10: the functional self-test's audio steps (the
#           chain's effects, speakers, the second device, recording RVP's
#           own stream with parec, the master volume through pactl, Audio
#           Stretching; it plays sound), the recording's WAV sizes, and the
#           audio chain's and speakers' level tests
#   m11     milestone M11: the M11 self-test (the command line's /seek and
#           /title, previews, scenes, the queue, a DVD folder, external
#           audio, the clipboard), the power actions through the dry-run
#           file only, the trash (the step's own XDG_DATA_HOME), and the
#           thumbnail core's and M11's headless tests
#   m12     milestone M12: the M12 self-test (window sizes, dialogs,
#           capture; Wayland places windows itself), RVP's shaders compiled
#           by mpv, and M12's and the engine's headless tests
#   m13     milestone M13: the M13 self-test (with --speech-check when
#           RVP_WHISPER_CLI and RVP_WHISPER_MODELS are set), whisper.cpp on
#           the speech clip, and the subtitle files' and speech's tests
#   m14     milestone M14: the M14 self-test on RVP Default and on RVP_SKIN
#           (no registry part: that is the Windows VM's), the autostart
#           entry left as it was, and the settings', translations', menus'
#           and synthetic skin's tests
#   m15     milestone M15: the M15 self-test; with RVP_URSERVER, Unified
#           Remote Server for Linux run from its unpacked folder (never
#           installed) runs the remote's actions in the served RVP, the
#           token stays out of the log; the control API's and the remote's
#           tests
#   m16     milestone M16: the synthetic menu DVD and Blu-ray (made in the
#           kit's folder with dvdauthor, or RVP_DVD_MENU and RVP_BD_MENU),
#           the system's libdvdnav and libbluray, the M16 self-test and the
#           shell's navigation tests
#   m18     milestone M18: the smoothing and GPU budget self-tests (MVTools
#           with a VapourSynth runtime, else Blend) and the smoothing tests;
#           the GPU modes are the Ubuntu boot's (CHECKLIST.md)
#   m18trt  M18's RIFE (TensorRT) on an NVIDIA card, after vapoursynth:
#           NVIDIA's runtime installed as RVP installs it (from
#           RVP_TENSORRT_MIRROR, else NVIDIA's and PyPI's servers, 3.8 GB)
#           into the kit's folder, then RVP's RIFE (TensorRT) script measured
#           at 720p, 1080p and 1440p (the first builds the engine, minutes),
#           with nvidia-smi before and after; the card's power limit is
#           recorded (no 90 % limit on the Ubuntu boot since 2026-11-14)
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
#   m17all  m8all with whisper, and m9 to m16, m18 and m18trt after m8
#           (goal S)
#
# environment:
#   RVP_KIT_DIR     build and download directory (default ~/.cache/rvp-kit)
#   RVP_LOG_DIR     logs (default $RVP_KIT_DIR/logs)
#   RVP_VIDEO       test video for S3 (default: generated 4K60 or 1080p60)
#   RVP_S1_VIDEO    film-rate test video for S1 (default: generated 23.976 fps)
#   RVP_SKIN        .dsf skin for S3 and the m-steps (default: none for S3,
#                   which then paints nothing; RVP Default for RVP)
#   RVP_VPY         motion interpolation script for S2
#   RVP_TENSORRT_MIRROR  folder URL with NVIDIA's Linux wheels for m18trt
#                   (file:// too; default: NVIDIA's and PyPI's servers)
#   VSSCRIPT_PATH   libvapoursynth-script.so to use (default: searched)
#   RVP_VS_PLUGINS  extra VapourSynth plugins for S2, ':' separated (default:
#                   the kit's mvtools)
#   YDOTOOL_SOCKET  ydotoold's socket for the input check
#                   (default /tmp/.ydotool_socket)
#   RVP_GPU_CHECKS  1 or 0 to override GPU detection
#   RVP_KIT_REBUILD 1 to rebuild libmpv even if this commit is already built
#   RVP_KIT_SOURCES a folder of bare clones the builds clone from first
#                   (tools/linux-kit/sources.sh mirror DIR makes it)
#   RVP_M2_SECONDS  playback length of the m2gpu runs (default 25; the
#                   generated clip is 30 s long and must not loop)
#   RVP_M9_SCRIPTS  m9: a folder of mpv script folders, each laid out as
#                   RVP's mpv folder (scripts/, fonts/, script-opts/)
#   RVP_WINAMP_SKINS, RVP_VLC_SKINS  m9: folders of .wsz and .vlt skins
#   RVP_WHISPER_CLI, RVP_WHISPER_MODELS  m13: whisper.cpp's whisper-cli and
#                   a folder with ggml-tiny.bin (RVP downloads no model)
#   RVP_SPEECH_CLIP m13: the speech clip (default: made with espeak-ng)
#   RVP_URSERVER    m15: Unified Remote Server for Linux unpacked (its
#                   .deb's root or its opt/urserver)
#   RVP_PACKAGE_SLIM 1: m8package makes packages without whisper.cpp and
#                   the VapourSynth runtime (by default it fails without them)
#   RVP_DVD_MENU, RVP_BD_MENU  m16: the synthetic discs (default: made in
#                   the kit's folder)
set -u

KIT_SCRIPT=$(readlink -f "${BASH_SOURCE[0]}")
REPO=$(cd "$(dirname "$KIT_SCRIPT")/../.." && pwd)
# kit_clone and the source cache (RVP_KIT_SOURCES).
. "$REPO/tools/linux-kit/sources.sh"
KIT_DIR=${RVP_KIT_DIR:-$HOME/.cache/rvp-kit}
LOG_DIR=${RVP_LOG_DIR:-$KIT_DIR/logs}
QT_VERSION=6.10.3
# libmpv at the commits of RVP's Windows libmpv (tools/libmpv/build.sh,
# goal T): mpv, FFmpeg's master (fruc_vulkan, NVIDIA's optical flow, is
# not in a release yet) and libplacebo, so that both systems play with the
# same filters (D19). Vulkan's headers, glslang (FFmpeg compiles its
# Vulkan shaders with it while it builds), dav1d and AviSynth+'s headers
# are built in the kit at the Windows build's versions: Ubuntu 22.04's are
# too old for FFmpeg's master.
MPV_COMMIT=b2c255c13e
FFMPEG_COMMIT=37515cfd87561e07c5766ea04de5b1ca6f6c0a98
LIBPLACEBO_COMMIT=a83aaeb4e60fdd7ed01619885f4f42bb0ef34eaa
VULKAN_HEADERS_TAG=v1.4.366
GLSLANG_TAG=16.6.0
SHADERC_COMMIT=4f0683ab2419ecbb9e18c9edfd4ffd70c2ba8cc6
DAV1D_TAG=1.5.4
AVISYNTH_COMMIT=21fdc997f9724b994896ba5520ddf64d677976b3
# The libmpv build's own revision, bumped when its options change (so that a
# built or cached libmpv of the same commit is built again): 2 added
# rubberband, which Audio Stretching needs; 3 the Windows build's FFmpeg and
# libplacebo, Vulkan, Lua, the disc libraries (DVD, Blu-ray, CD), GnuTLS in
# place of OpenSSL (mpv-build's OpenSSL made FFmpeg --enable-nonfree, which
# no package may carry) and the audio filters' libraries; 4 libplacebo
# with the kit's shaderc; 5 the encoders of Record Video and the
# captures (x264, x265, vpx, aom, opus, lame, vorbis, webp).
MPV_BUILD=5
VS_TAG=R73
# The version in the Windows bundle (spikes/tools/windows-deps.sh).
MVTOOLS_TAG=v24
JOBS=$(nproc)
# A clone or fetch whose transfer stalls (the shared VM's network to GitHub
# hung for half an hour) gives up after a minute below 1 KB/s.
export GIT_HTTP_LOW_SPEED_LIMIT=1000 GIT_HTTP_LOW_SPEED_TIME=60
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
# dvdauthor makes M16's test DVD; pulseaudio-utils' pactl and parec are
# RVP's master volume and recording on Linux; patchelf sets the run paths
# of RVP's VapourSynth runtime.
PACKAGES="build-essential git curl python3-venv python3-jinja2 pkg-config nasm cmake ninja-build \
autoconf automake libtool \
libass-dev libfreetype-dev libfribidi-dev libharfbuzz-dev libfontconfig-dev \
libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxss-dev libxv-dev libxpresent-dev \
libxkbcommon-dev libwayland-dev wayland-protocols libegl-dev libgl-dev libgbm-dev libdrm-dev \
libasound2-dev libpulse-dev libpipewire-0.3-dev libva-dev libvdpau-dev libffmpeg-nvenc-dev \
zlib1g-dev libssl-dev liblcms2-dev libjpeg-dev \
libvulkan-dev glslang-dev libunwind-dev libfftw3-dev librubberband-dev \
libgnutls28-dev libdvdnav-dev libdvdread-dev libbluray-dev libcdio-paranoia-dev \
libmysofa-dev libbs2b-dev libsoxr-dev libzimg-dev libxml2-dev libuchardet-dev libarchive-dev \
libluajit-5.1-dev libx264-dev libx265-dev libvpx-dev libaom-dev libopus-dev libmp3lame-dev libvorbis-dev \
libwebp-dev \
libxcb-cursor0 libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-render-util0 libxcb-xinerama0 \
libxcb-xkb1 libxkbcommon-x11-0 libopengl0 libwayland-cursor0 libwayland-egl1 \
lsof mesa-utils ffmpeg dvdauthor pulseaudio-utils patchelf"

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
# vs_header NAME: VapourSynth's header NAME at VS_TAG into the kit's
# vs-sdk, from the source cache when it has VapourSynth.
vs_header() {
    local target=$KIT_DIR/vs-sdk/include/vapoursynth/$1
    [ -s "$target" ] && return 0
    if [ -d "${RVP_KIT_SOURCES:-/nonexistent}/vapoursynth.git" ]; then
        git --git-dir="$RVP_KIT_SOURCES/vapoursynth.git" show "$VS_TAG:include/$1" > "$target"
    else
        curl -fsSL "https://raw.githubusercontent.com/vapoursynth/vapoursynth/$VS_TAG/include/$1" -o "$target"
    fi
}

vs_headers() {
    local dir=$KIT_DIR/vs-sdk
    mkdir -p "$dir/include/vapoursynth" "$dir/lib/pkgconfig"
    for header in VapourSynth4.h VSScript4.h VSHelper4.h VSConstants4.h; do
        vs_header "$header"
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

# Vulkan's headers (with a pkg-config file that links the system's
# loader), glslang (FFmpeg's shader compiler), shaderc (libplacebo's, as
# one static library with its own glslang and SPIRV-Tools; and glslc for
# whisper.cpp), dav1d and AviSynth+'s headers in $KIT_DIR/sdk, built once
# per set of versions. Prints the folder.
kit_sdk() {
    local sdk=$KIT_DIR/sdk src=$KIT_DIR/sdk-src dep revision
    local stamp="$VULKAN_HEADERS_TAG $GLSLANG_TAG $SHADERC_COMMIT $DAV1D_TAG $AVISYNTH_COMMIT"
    if [ "$(cat "$sdk/.kit-sdk" 2> /dev/null)" != "$stamp" ]; then
        rm -rf "$sdk" "$src"
        mkdir -p "$src" "$sdk/lib/pkgconfig" "$sdk/include" "$sdk/bin"
        {
            kit_clone https://github.com/KhronosGroup/Vulkan-Headers.git "$src/vulkan" "$VULKAN_HEADERS_TAG" &&
                cmake -Wno-dev -G Ninja -S "$src/vulkan" -B "$src/vulkan/build" -DCMAKE_INSTALL_PREFIX="$sdk" \
                    -DVULKAN_HEADERS_ENABLE_TESTS=OFF -DVULKAN_HEADERS_ENABLE_MODULE=OFF &&
                cmake --install "$src/vulkan/build" &&
                printf '%s\n' "prefix=$sdk" 'includedir=${prefix}/include' 'Name: Vulkan-Loader' \
                    "Description: Vulkan headers $VULKAN_HEADERS_TAG with the system's loader" \
                    "Version: ${VULKAN_HEADERS_TAG#v}" 'Libs: -lvulkan' 'Cflags: -I${includedir}' \
                    > "$sdk/lib/pkgconfig/vulkan.pc"
        } >&2 || return 1
        {
            kit_clone https://github.com/KhronosGroup/glslang.git "$src/glslang" "$GLSLANG_TAG" &&
                cmake -Wno-dev -G Ninja -S "$src/glslang" -B "$src/glslang/build" -DCMAKE_BUILD_TYPE=Release \
                    -DCMAKE_INSTALL_PREFIX="$sdk" -DENABLE_OPT=OFF -DGLSLANG_TESTS=OFF -DBUILD_TESTING=OFF \
                    -DBUILD_SHARED_LIBS=OFF &&
                ninja -C "$src/glslang/build" install
        } >&2 || return 1
        {
            # shaderc with the revisions its DEPS file names of the three
            # sources it builds (its tests' are left out).
            kit_clone https://github.com/google/shaderc.git "$src/shaderc" "$SHADERC_COMMIT" || return 1
            for dep in glslang:glslang SPIRV-Tools:spirv_tools SPIRV-Headers:spirv_headers; do
                revision=$(sed -n "s/.*'${dep#*:}_revision': '\([0-9a-f]*\)'.*/\1/p" "$src/shaderc/DEPS")
                kit_clone "https://github.com/KhronosGroup/${dep%%:*}.git" \
                    "$src/shaderc/third_party/$(echo "${dep%%:*}" | tr 'A-Z' 'a-z')" "$revision" || return 1
            done
            cmake -Wno-dev -G Ninja -S "$src/shaderc" -B "$src/shaderc/build" -DCMAKE_BUILD_TYPE=Release \
                -DCMAKE_INSTALL_PREFIX="$src/shaderc-install" -DSHADERC_SKIP_TESTS=ON \
                -DSHADERC_SKIP_EXAMPLES=ON -DSHADERC_SKIP_COPYRIGHT_CHECK=ON -DSPIRV_SKIP_TESTS=ON \
                -DSPIRV_SKIP_EXECUTABLES=ON -DENABLE_GLSLANG_BINARIES=OFF -DSPIRV_TOOLS_BUILD_STATIC=ON &&
                ninja -C "$src/shaderc/build" -j "$JOBS" install &&
                # Only the combined static library, as `shaderc`.
                cp -r "$src/shaderc-install/include/shaderc" "$sdk/include/" &&
                cp "$src/shaderc-install/lib/libshaderc_combined.a" "$sdk/lib/" &&
                cp "$src/shaderc-install/bin/glslc" "$sdk/bin/" &&
                sed -e "s|^prefix=.*|prefix=$sdk|" "$src/shaderc-install/lib/pkgconfig/shaderc_combined.pc" \
                    > "$sdk/lib/pkgconfig/shaderc.pc"
        } >&2 || return 1
        {
            kit_clone https://code.videolan.org/videolan/dav1d.git "$src/dav1d" "$DAV1D_TAG" &&
                meson setup "$src/dav1d/build" "$src/dav1d" --prefix="$sdk" --libdir=lib --buildtype=release \
                    --default-library=static -Denable_tools=false -Denable_tests=false &&
                ninja -C "$src/dav1d/build" install
        } >&2 || return 1
        {
            kit_clone https://github.com/AviSynth/AviSynthPlus.git "$src/avisynth" "$AVISYNTH_COMMIT" &&
                cmake -Wno-dev -G Ninja -S "$src/avisynth" -B "$src/avisynth/build" -DCMAKE_INSTALL_PREFIX="$sdk" \
                    -DHEADERS_ONLY=ON &&
                ninja -C "$src/avisynth/build" VersionGen &&
                ninja -C "$src/avisynth/build" install
        } >&2 || return 1
        echo "$stamp" > "$sdk/.kit-sdk"
        rm -rf "$src"
    fi
    echo "$sdk"
}

# What the kit's libmpv lacks of what RVP needs, found in the library's
# strings (FFmpeg's configure line, the libraries' and mpv's own names):
# a non-free FFmpeg, Lua with mpv's built-in scripts (the OSC), the disc
# libraries, the audio filters' libraries, fruc_vulkan.
mpv_lacks() {
    local lib=$1 missing="" text
    text=$(strings "$lib")
    if grep -q -- "--enable-nonfree" <<< "$text"; then
        missing="$missing nonfree-ffmpeg"
    fi
    for marker in --enable-gnutls --enable-libmysofa --enable-libbs2b --enable-librubberband \
        --enable-libdav1d --enable-avisynth --enable-vapoursynth --enable-libzimg \
        --enable-libx264 --enable-libwebp \
        dvdnav_ bd_open cdio_ luaL_ fruc_vulkan osc.lua; do
        grep -q -- "$marker" <<< "$text" || missing="$missing $marker"
    done
    echo "${missing# }"
}

step_mpv() {
    local log=$LOG_DIR/mpv.log build=$KIT_DIR/mpv-build prefix sdk lib lacking
    prefix=$(mpv_prefix)
    # mpv-build always rebuilds FFmpeg from scratch; skip a finished build of
    # the same commit unless RVP_KIT_REBUILD=1.
    if [ "${RVP_KIT_REBUILD:-0}" != 1 ] && [ "$(cat "$prefix/.kit-mpv" 2> /dev/null)" = "$MPV_COMMIT build $MPV_BUILD" ]; then
        record mpv PASS "libmpv $MPV_COMMIT build $MPV_BUILD in $prefix (already built)"
        return 0
    fi
    # A subshell: its exports and its `cd` stay in it, and an `exit` ends
    # it, not the kit (the Linux audit's L25).
    (
        export PATH=$KIT_DIR/venv/bin:$PATH
        # A failing clone must not wait for credentials; rerun the step.
        export GIT_TERMINAL_PROMPT=0
        sdk=$(kit_sdk) || exit 1
        export PATH=$sdk/bin:$PATH
        PKG_CONFIG_PATH=$sdk/lib/pkgconfig:$(vs_headers):${PKG_CONFIG_PATH:-}
        export PKG_CONFIG_PATH
        # libplacebo compiles its shaders with the kit's shaderc: with glslang
        # it took some libraries from Ubuntu's glslang 11.8 and libmpv
        # failed at its first shader (an undefined GetDefaultResources).
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
        ./use-ffmpeg-custom "$FFMPEG_COMMIT"
        ./use-libplacebo-custom "$LIBPLACEBO_COMMIT"
        ./use-mpv-custom "$MPV_COMMIT"
        printf '%s\n' -Dlibmpv=true -Dcplayer=false -Dvapoursynth=enabled -Drubberband=enabled \
            -Dlua=luajit -Djavascript=disabled -Ddvdnav=enabled -Dlibbluray=enabled -Dcdda=enabled \
            -Duchardet=enabled -Dlibarchive=enabled -Dlcms2=enabled -Dvulkan=enabled \
            "--prefix=$prefix" > mpv_options
        # GnuTLS is named: without it mpv-build picks OpenSSL and makes
        # FFmpeg --enable-nonfree. FFmpeg's Vulkan shaders are compiled
        # with the kit's glslang (an older glslc on the PATH comes first).
        # FFmpeg's master wants lcms2 2.13 (Ubuntu 22.04 has 2.12): its
        # lcms2 is left out; mpv's own ICC support has it.
        printf '%s\n' --disable-programs --disable-doc --enable-version3 --enable-gnutls \
            --enable-libdav1d --enable-libmysofa --enable-libbs2b --enable-librubberband --enable-libsoxr \
            --enable-libzimg --enable-libxml2 --enable-avisynth --enable-vapoursynth \
            --enable-libx264 --enable-libx265 --enable-libvpx --enable-libaom --enable-libopus \
            --enable-libmp3lame --enable-libvorbis --enable-libwebp \
            --enable-vulkan "--glslc=$sdk/bin/glslang" "--extra-cflags=-I$sdk/include -I$KIT_DIR/vs-sdk/include" \
            > ffmpeg_options
        printf '%s\n' -Dvulkan=enabled "-Dvulkan-registry=$sdk/share/vulkan/registry/vk.xml" \
            -Dglslang=disabled -Dshaderc=enabled > libplacebo_options
        # A failed rebuild left the last libmpv in place, which passed.
        ./update && ./rebuild "-j$JOBS" && ./install
    ) > "$log" 2>&1
    local built=$?
    if [ "$built" -ne 0 ]; then
        record mpv FAIL "exit $built, see $log"
        return 1
    fi
    lib=$(ls "$prefix"/lib/x86_64-linux-gnu/libmpv.so.2* "$prefix"/lib/libmpv.so.2* 2> /dev/null | head -1)
    if [ -z "$lib" ]; then
        record mpv FAIL "see $log"
        return 1
    fi
    lacking=$(mpv_lacks "$lib")
    # Every symbol must resolve now: an undefined one fails only when it is
    # first called (glslang's GetDefaultResources at the first shader).
    if ldd -r "$lib" 2>&1 | grep -q 'undefined symbol'; then
        lacking="$lacking undefined-symbols($(ldd -r "$lib" 2>&1 | grep -m3 -o 'undefined symbol: [^ ]*' | tr '\n' ' '))"
    fi
    if [ -n "$lacking" ]; then
        record mpv FAIL "libmpv lacks: $lacking (see $log)"
        return 1
    fi
    echo "$MPV_COMMIT build $MPV_BUILD" > "$prefix/.kit-mpv"
    record mpv PASS "libmpv $MPV_COMMIT build $MPV_BUILD (FFmpeg ${FFMPEG_COMMIT:0:10}) in $prefix"
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
            vs_header "$header"
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
            kit_clone https://github.com/dubhater/vapoursynth-mvtools.git "$dir/src" "$MVTOOLS_TAG"
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
    # The socket input_ready found: it runs in a command substitution, so
    # its export does not reach this shell (without it ydotool looks in
    # $XDG_RUNTIME_DIR and exits with 2).
    YDOTOOL_SOCKET=${YDOTOOL_SOCKET:-/tmp/.ydotool_socket} \
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
    local log=$LOG_DIR/m2build.log built
    (
        cd "$REPO" || exit 1
        export QMAKE MPV_LIB_DIR
        QMAKE=$(qt_dir)/bin/qmake
        MPV_LIB_DIR=$(mpv_lib_dir)
        cargo build --release -p rvp-shell
    ) > "$log" 2>&1
    built=$?
    # A failed build leaves the last binary behind: cargo's own result counts.
    if [ "$built" -eq 0 ] && [ -x "$(rvp_bin)" ]; then
        record m2build PASS "rvp built"
    else
        record m2build FAIL "cargo exit $built, see $log"
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
    # SVP support on (off by default), on RVP's default endpoint.
    export RVP_CONFIG_DIR=$LOG_DIR/m2svp-config RVP_STATE_DIR=$LOG_DIR/m2svp-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
    mkdir -p "$RVP_CONFIG_DIR"
    printf '{\n  "svp-enabled": true\n}\n' > "$RVP_CONFIG_DIR/settings.json"
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

# whisper.cpp's CPU and Vulkan engine, built by the `whisper` step.
whisper_out() { echo "$KIT_DIR/whisper/whisper-out/whisper"; }
# RVP's own VapourSynth runtime, built by the `vapoursynth` step.
vs_runtime() { echo "$KIT_DIR/vs-runtime/vapoursynth"; }

step_vapoursynth() {
    local log=$LOG_DIR/vapoursynth.log
    if [ ! -s "$(mvtools_lib)" ]; then
        record vapoursynth SKIPPED "run mvtools first"
        return 0
    fi
    "$REPO/tools/vapoursynth/build-linux.sh" "$(mvtools_lib)" "$KIT_DIR/vs-work" "$(vs_runtime)" > "$log" 2>&1
    local code=$?
    if [ "$code" -eq 0 ] && grep -q "with mv and rife" "$log"; then
        record vapoursynth PASS "$(grep -m1 "with mv and rife" "$log"), $(du -sh "$(vs_runtime)" | cut -f1)"
    else
        record vapoursynth FAIL "exit $code, see $log"
        return 1
    fi
}

step_whisper() {
    local log=$LOG_DIR/whisper.log sdk
    {
        sdk=$(kit_sdk) &&
            "$REPO/tools/whisper/build-linux.sh" "$sdk" "$KIT_DIR/whisper"
    } > "$log" 2>&1
    local code=$?
    if [ "$code" -eq 0 ] && "$(whisper_out)/whisper-cli" --help > /dev/null 2>&1; then
        record whisper PASS "$(cat "$(whisper_out)/VERSION.txt")"
    else
        record whisper FAIL "exit $code, see $log"
        return 1
    fi
}

step_m8package() {
    local log=$LOG_DIR/m8package.log out runtime url KIT_WHISPER="" KIT_VAPOURSYNTH=""
    out=$(m8_dir)
    # The engine for subtitles from audio and the VapourSynth runtime, which
    # the `whisper` and `vapoursynth` steps build: the packages carry both
    # (CI's lacked them without a word, the Linux audit's H1), unless
    # RVP_PACKAGE_SLIM=1 says they are meant without them.
    [ -x "$(whisper_out)/whisper-cli" ] && KIT_WHISPER=$(whisper_out)
    [ -e "$(vs_runtime)/libvapoursynth-script.so.0" ] && KIT_VAPOURSYNTH=$(vs_runtime)
    if [ "${RVP_PACKAGE_SLIM:-}" != 1 ] && { [ -z "$KIT_WHISPER" ] || [ -z "$KIT_VAPOURSYNTH" ]; }; then
        local lacking=""
        [ -n "$KIT_WHISPER" ] || lacking=" whisper.cpp (step whisper)"
        [ -n "$KIT_VAPOURSYNTH" ] || lacking="$lacking${lacking:+,} the VapourSynth runtime (step vapoursynth)"
        record m8package FAIL "no$lacking: build it first, or RVP_PACKAGE_SLIM=1 for packages without"
        return 1
    fi
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
            --runtime "$runtime" ${KIT_WHISPER:+--whisper "$KIT_WHISPER"} \
            ${KIT_VAPOURSYNTH:+--vapoursynth "$KIT_VAPOURSYNTH"}
    ) > "$log" 2>&1
    local made=$? appimage deb
    appimage=$(ls "$out"/RVP-*-x86_64.AppImage 2> /dev/null | head -1)
    deb=$(ls "$out"/rvp_*_amd64.deb 2> /dev/null | head -1)
    # A failed build leaves the last packages behind: the result counts.
    if [ "$made" -eq 0 ] && [ -n "$appimage" ] && [ -n "$deb" ]; then
        record m8package PASS "$(basename "$appimage") $(du -h "$appimage" | cut -f1), $(basename "$deb") $(du -h "$deb" | cut -f1)"
    else
        record m8package FAIL "exit $made, see $log"
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

# Whether RVP itself holds an idle inhibitor of GNOME's session manager:
# one with its app ID (`rvp`, its desktop entry) whose flags have idle (8).
# Prints yes or no, nothing without the session manager. Another
# program's inhibitor (IsInhibited true before RVP started) proves nothing.
m8_rvp_inhibits() {
    local list path app flags
    list=$(gdbus call --session --dest org.gnome.SessionManager --object-path /org/gnome/SessionManager \
        --method org.gnome.SessionManager.GetInhibitors 2> /dev/null) || return 0
    for path in $(printf '%s\n' "$list" | grep -o '/org/gnome/SessionManager/Inhibitor[0-9]*'); do
        app=$(gdbus call --session --dest org.gnome.SessionManager --object-path "$path" \
            --method org.gnome.SessionManager.Inhibitor.GetAppId 2> /dev/null)
        flags=$(gdbus call --session --dest org.gnome.SessionManager --object-path "$path" \
            --method org.gnome.SessionManager.Inhibitor.GetFlags 2> /dev/null |
            sed -n 's/.*uint32 \([0-9]*\).*/\1/p')
        if [ "$app" = "('rvp',)" ] && [ $(( ${flags:-0} & 8 )) -ne 0 ]; then
            echo yes
            return 0
        fi
    done
    echo no
}

# MPRIS and idle inhibit while `rvp` plays the test video on its own
# instance channel; prints check lines.
m8_mpris() {
    local rvp=$1 pid name status before during paused deadline
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
        # RVP's own inhibitor while it plays (asked on the desktop thread:
        # a moment after the playback starts).
        before=$(m8_idle_inhibited)
        deadline=$((SECONDS + 6))
        while during=$(m8_rvp_inhibits) && [ "$during" = no ] && [ $SECONDS -lt $deadline ]; do
            sleep 0.5
        done
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
        deadline=$((SECONDS + 6))
        while paused=$(m8_rvp_inhibits) && [ "$paused" = yes ] && [ $SECONDS -lt $deadline ]; do
            sleep 0.5
        done
        if [ -z "$during" ]; then
            echo "check idle_inhibit=SKIPPED no GNOME session manager on this bus"
        elif [ "$during" = yes ] && [ "$paused" = no ]; then
            echo "check idle_inhibit=PASS RVP's inhibitor while playing, none paused (IsInhibited 8 at the start: $before)"
        else
            echo "check idle_inhibit=FAIL RVP's inhibitor playing $during paused $paused"
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

# The General page's media controls off: no MPRIS, and RVP still keeps
# the screen on while the video plays (the Linux audit's M2: the switch
# took the inhibit, the launcher's progress and the lock with MPRIS).
m8_inhibit_without_mpris() {
    local rvp=$1 pid deadline during name
    kit_settings '"general": { "media-controls": false }'
    "$rvp" --instance=m8 --skin= "--mpv-options=hwdec=$(m2_hwdec),ao=null" "$(test_video)" \
        > "$LOG_DIR/m8-nompris-rvp.log" 2>&1 &
    pid=$!
    deadline=$((SECONDS + 20))
    while during=$(m8_rvp_inhibits) && [ "$during" = no ] && [ $SECONDS -lt $deadline ]; do
        sleep 0.5
    done
    name=$(m8_mpris_name "$pid")
    kill "$pid" 2> /dev/null
    wait "$pid" 2> /dev/null
    kit_settings
    if [ -z "$during" ]; then
        echo "check idle_inhibit_without_mpris=SKIPPED no GNOME session manager on this bus"
    elif [ "$during" = yes ] && [ -z "$name" ]; then
        echo "check idle_inhibit_without_mpris=PASS RVP's inhibitor, no MPRIS name"
    else
        echo "check idle_inhibit_without_mpris=FAIL RVP's inhibitor $during, MPRIS name '$name'"
    fi
}

# The crash logs of `rvp`: a panic and a fatal signal, each in a state
# folder of its own. Never the user's: on 2026-11-15 an unset
# RVP_STATE_DIR left the installed RVP's crash log in the owner's state
# folder (and a removal of the variable's folder could have taken it).
m8_crash() {
    local rvp=$1 kind text state
    for kind in panic native; do
        state=$(mktemp -d)
        RVP_STATE_DIR=$state RVP_CONFIG_DIR=$state/config "$rvp" "--crash-test=$kind" > /dev/null 2>&1
        text=$(cat "$state"/crash-*.log 2> /dev/null)
        rm -rf "$state"
        case $kind:$text in
            panic:*"panic in thread 'main'"*) echo "check crash_log_panic=PASS" ;;
            native:*"fatal signal 11"*) echo "check crash_log_native=PASS" ;;
            *) echo "check crash_log_$kind=FAIL $text" ;;
        esac
    done
}

step_m8() {
    local log=$LOG_DIR/m8.log out appimage deb installed=0 default_before bus="" want have
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
        # The package this step checks is the one m8package built: an
        # older installed build would pass every check below for it.
        want=$(dpkg-deb -f "$deb" Version 2> /dev/null)
        have=$(dpkg-query -W -f '${Version}' rvp 2> /dev/null)
        if [ -n "$have" ] && [ "$have" = "$want" ]; then
            echo "check deb_installed=PASS $want, installed before this step"
        elif sudo -n true 2> /dev/null; then
            if sudo -n apt-get install -y --allow-downgrades "$deb" > "$LOG_DIR/m8-apt.log" 2>&1; then
                installed=1
                echo "check deb_installed=PASS $want"
            else
                echo "check deb_installed=FAIL see $LOG_DIR/m8-apt.log"
            fi
        elif [ -n "$have" ]; then
            echo "check deb_installed=FAIL $have is installed, this step built $want: sudo apt install $deb"
        else
            echo "check deb_installed=SKIPPED install it first: sudo apt install $deb"
        fi
        if [ -n "$want" ] && [ "$(dpkg-query -W -f '${Version}' rvp 2> /dev/null)" = "$want" ]; then
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
            m8_inhibit_without_mpris /usr/bin/rvp
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
    printf '{\n  "svp-enabled": true,\n  "ipc-server": "/tmp/mpvsocket"\n}\n' > "$RVP_CONFIG_DIR/settings.json"
    vsscript=$(find_vsscript)
    : > "$log"
    echo "== first window" >> "$log"
    VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        timeout --kill-after=15 600 "$(rvp_bin)" --selftest=svp "--out=$out" "--skin=${RVP_SKIN:-}" \
        --mpv-options=hwdec=auto-copy "$(s1_video)" > "$LOG_DIR/m6svp-first.log" 2>&1 &
    pid=$!
    # The second window starts once the first holds the endpoint.
    sleep 8
    # Its own settings and state: two processes must not share them. SVP
    # attaches to the window on its endpoint only: the second one shows
    # RVP's diagnostic, as on Windows (the owner, 2026-11-14).
    echo "== second window (/new)" >> "$log"
    mkdir -p "$LOG_DIR/m6svp-config2"
    cp "$RVP_CONFIG_DIR/settings.json" "$LOG_DIR/m6svp-config2/"
    rm -rf "$LOG_DIR/m6svp-state2"
    RVP_CONFIG_DIR=$LOG_DIR/m6svp-config2 RVP_STATE_DIR=$LOG_DIR/m6svp-state2 \
        VSSCRIPT_PATH=${vsscript:-${VSSCRIPT_PATH:-}} PYTHONDONTWRITEBYTECODE=1 \
        timeout --kill-after=15 300 "$(rvp_bin)" /new --selftest=svp --svp-second "--out=$out" \
        "--skin=${RVP_SKIN:-}" --mpv-options=hwdec=auto-copy "$(s1_video)" > "$LOG_DIR/m6svp-second.log" 2>&1
    second=$?
    wait "$pid"
    first=$?
    cat "$LOG_DIR/m6svp-first.log" >> "$log"
    echo "== second window log" >> "$log"
    cat "$LOG_DIR/m6svp-second.log" >> "$log"
    local detail
    detail="first=$first second=$second $(grep -hE '^svp_endpoint=|^check svp_(attached|target_fps|second_diagnostic)' "$LOG_DIR/m6svp-first.log" "$LOG_DIR/m6svp-second.log" | tr '\n' ' ')"
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

# The steps of milestones M9 to M16 and M18 (goal S) follow the Windows
# VM's checks (tools/windows-vm/m9-all.ps1 to m18-all.ps1) with Linux's
# counterparts; Windows-only parts (the registry, powercfg, Direct3D 11,
# the installer) are left out where they come up.

kit_media() { echo "$REPO/testdata/media"; }

# One RVP run of step STEP named NAME: fresh settings and state in the
# step's folders ($LOG_DIR/STEP-config, -state; KIT_RUN_SETTINGS adds to
# settings.json, KIT_RUN_MPV is a folder copied in as RVP's mpv folder),
# RVP's output kept in $LOG_DIR/STEP-NAME.out and added to the step's
# log. Returns RVP's exit code.
kit_run() {
    local step=$1 name=$2 code
    local log=$LOG_DIR/$1.log run=$LOG_DIR/$1-$2.out
    shift 2
    export RVP_CONFIG_DIR=$LOG_DIR/$step-config RVP_STATE_DIR=$LOG_DIR/$step-state
    rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
    kit_settings "${KIT_RUN_SETTINGS:-}"
    if [ -n "${KIT_RUN_MPV:-}" ]; then
        cp -a "$KIT_RUN_MPV" "$RVP_CONFIG_DIR/mpv"
    fi
    echo "== $name" >> "$log"
    # A run that hangs (a self-test that never ends, an exit that waits)
    # fails here instead of holding the kit up for good: M14's self-test
    # once waited at exit (goal U).
    timeout --kill-after=15 "${KIT_RUN_TIMEOUT:-900}" "$(rvp_bin)" "$@" > "$run" 2>&1
    code=$?
    [ "$code" -eq 124 ] && echo "kit: $name timed out after ${KIT_RUN_TIMEOUT:-900} s" >> "$run"
    echo "kit_exit=$code" >> "$run"
    cat "$run" >> "$log"
    return "$code"
}

# Judges run NAME of STEP: its checks (each MUST passed), none failed, RVP
# ended well. Prints "NAME:passed/failed" (and what is missing) for the
# step's summary.
kit_judge() {
    local step=$1 name=$2 run passes fails code must missing=""
    run=$LOG_DIR/$1-$2.out
    shift 2
    passes=$(grep -c '^check .*=PASS' "$run" 2> /dev/null)
    fails=$(grep -c '^check .*=FAIL' "$run" 2> /dev/null)
    code=$(sed -n 's/^kit_exit=//p' "$run" 2> /dev/null | tail -1)
    # A required check must have passed: `=PASS SKIPPED ...` (a self-test
    # that found no library, no disc...) does not count.
    for must in "$@"; do
        grep "^check $must=PASS" "$run" 2> /dev/null | grep -qv "=PASS SKIPPED" ||
            missing="$missing $must"
    done
    echo "$name:${passes:-0}/${fails:-0}${missing:+ missing:${missing# }}"
    [ "${fails:-1}" -eq 0 ] && [ "${passes:-0}" -gt 0 ] && [ -z "$missing" ] && [ "${code:-1}" -eq 0 ]
}

# A check of the step's own, written to its log as the self-tests write
# theirs: kit_check LOG NAME OK DETAIL (OK 1 or 0; SKIPPED for a skip).
kit_check() {
    local result=FAIL
    case $3 in 1) result=PASS ;; SKIPPED) result=SKIPPED ;; esac
    echo "check $2=$result${4:+ $4}" >> "$1"
    [ "$result" != FAIL ]
}

# cargo test with the kit's Qt and libmpv (as m2build builds RVP); the
# caller exports what the tests read. Output to LOG; cargo's exit code.
kit_cargo_test() {
    local log=$1
    shift
    echo "== cargo test $*" >> "$log"
    (
        cd "$REPO" || exit 1
        export QMAKE MPV_LIB_DIR
        QMAKE=$(qt_dir)/bin/qmake
        MPV_LIB_DIR=$(mpv_lib_dir)
        cargo test --release "$@"
    ) >> "$log" 2>&1
}

# The step's result from its runs' and checks' words: PASS when BAD is 0.
kit_result() {
    local step=$1 bad=$2 summary=$3 log=$LOG_DIR/$1.log
    if [ "$bad" -eq 0 ]; then
        record "$step" PASS "$summary"
    else
        record "$step" FAIL "$summary $(grep -E '^check .*=FAIL|FAILED|panicked' "$log" | head -20 | tr '\n' ' ')"
        return 1
    fi
}

step_m9() {
    local log=$LOG_DIR/m9.log out=$LOG_DIR/m9-out clip bad=0 summary="" word folder name skin
    local KIT_RUN_MPV=""
    m2_ready m9 || return 0
    m2_env
    clip=$(kit_media)/fixture-60s.mkv
    rm -rf "$out"
    : > "$log"
    # Classic and VLC skins dock windows of their own; on Wayland RVP runs
    # them through XWayland when DISPLAY reaches it (shell: prefer_x11).
    echo "session=${XDG_SESSION_TYPE:-none} wayland=${WAYLAND_DISPLAY:-none} display=${DISPLAY:-none} qpa=${QT_QPA_PLATFORM:-auto}" >> "$log"
    # mpv's own OSC (the kit's libmpv carries Lua and mpv's scripts).
    kit_run m9 osc --selftest=m9 --controller=osc "--out=$out/osc" --skin= --mpv-options=ao=null "$clip"
    word=$(kit_judge m9 osc m9_controller_takes_click) || bad=$((bad + 1))
    summary="$summary $word"
    # The user's scripts: each folder of RVP_M9_SCRIPTS laid out as RVP's mpv
    # folder (scripts/, fonts/, script-opts/); third-party, never committed.
    if [ -n "${RVP_M9_SCRIPTS:-}" ] && [ -d "$RVP_M9_SCRIPTS" ]; then
        for folder in "$RVP_M9_SCRIPTS"/*/; do
            [ -d "$folder/scripts" ] || continue
            name=$(basename "$folder")
            local extra=()
            # ModernZ's seek bar lies higher than the others'.
            case ${name,,} in modernz*) extra=(--controller-bar=0.915) ;; esac
            KIT_RUN_MPV=${folder%/}
            kit_run m9 "$name" --selftest=m9 --controller=scripts "${extra[@]}" \
                "--out=$out/$name" --skin= --mpv-options=ao=null "$clip"
            KIT_RUN_MPV=
            word=$(kit_judge m9 "$name" m9_controller_takes_click) || bad=$((bad + 1))
            summary="$summary $word"
        done
    else
        kit_check "$log" m9_scripts SKIPPED "RVP_M9_SCRIPTS names no folder of scripts"
        summary="$summary scripts:SKIPPED"
    fi
    # Classic Winamp skins and VLC's themes from local samples.
    for skin in "${RVP_WINAMP_SKINS:-/nonexistent}"/*.wsz "${RVP_VLC_SKINS:-/nonexistent}"/*.vlt; do
        [ -f "$skin" ] || continue
        name=$(basename "$skin")
        name=${name//[^A-Za-z0-9]/_}
        case $skin in
            *.wsz) kit_run m9 "classic-$name" --selftest=classic "--skin=$skin" "--out=$out/$name" \
                --mpv-options=ao=null "$clip"
                word=$(kit_judge m9 "classic-$name") || bad=$((bad + 1)) ;;
            *) kit_run m9 "vlc-$name" --selftest=vlc "--skin=$skin" "--out=$out/$name" \
                --mpv-options=ao=null "$clip"
                word=$(kit_judge m9 "vlc-$name") || bad=$((bad + 1)) ;;
        esac
        summary="$summary $word"
    done
    [ -n "${RVP_WINAMP_SKINS:-}" ] || summary="$summary winamp:SKIPPED"
    [ -n "${RVP_VLC_SKINS:-}" ] || summary="$summary vlc:SKIPPED"
    # The loaders on every local skin, headless (they pass with a note
    # without the folders).
    (
        export RVP_WINAMP_SKINS="${RVP_WINAMP_SKINS:-}" RVP_VLC_SKINS="${RVP_VLC_SKINS:-}"
        kit_cargo_test "$log" -p rvp-skin --test winamp --test vlc -- --nocapture
    ) || bad=$((bad + 1))
    kit_result m9 "$bad" "${summary# }"
}

# The recording's file: a RIFF WAVE whose sizes match its length. Prints
# "ok BYTES DATA" or "bad ...".
kit_wav_sizes() {
    python3 -I - "$1" << 'PY'
import struct
import sys

data = open(sys.argv[1], "rb").read()
ok = len(data) >= 12 and data[:4] == b"RIFF" and data[8:12] == b"WAVE"
ok = ok and struct.unpack_from("<I", data, 4)[0] == len(data) - 8
at, size, start = 12, -1, -1
while ok and at + 8 <= len(data):
    chunk, length = data[at:at + 4], struct.unpack_from("<I", data, at + 4)[0]
    if chunk == b"data":
        size, start = length, at + 8
        break
    at += 8 + length + length % 2
ok = ok and start >= 0 and start + size == len(data)
print(("ok" if ok else "bad"), len(data), size)
PY
}

step_m10() {
    local log=$LOG_DIR/m10.log out=$LOG_DIR/m10-out media bad=0 summary word wav sizes
    m2_ready m10 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out"
    : > "$log"
    # The main self-test's M10 steps: effects in the chain, the speaker
    # choices, the individual volume, Freeverb, the sync dialog, the second
    # device, recording (parec on RVP's own stream: mpv's default output,
    # so this run plays sound), the system's master volume (pactl: it goes
    # up a step and back), Audio Stretching.
    kit_run m10 functional --selftest "--out=$out" --skin= "$media/fixture-60s.mkv" "$media/fixture-3s.mkv"
    word=$(kit_judge m10 functional second_device_follows recording_starts recording_stops \
        master_volume_answers shift_j_audio_stretching_up) || bad=$((bad + 1))
    summary=$word
    # The master volume's answer is the device's level, not an error.
    grep -q '^check master_volume_answers=PASS .*Master volume: [0-9]' "$LOG_DIR/m10-functional.out" ||
        { kit_check "$log" master_volume_level 0 "$(grep '^check master_volume_answers' "$LOG_DIR/m10-functional.out")"; bad=$((bad + 1)); }
    # Windows' powercfg check of the awake request has m8's idle inhibit as
    # its Linux counterpart.
    wav=$out/recording.wav
    if [ -s "$wav" ]; then
        sizes=$(kit_wav_sizes "$wav")
        kit_check "$log" recording_file_sizes "$([ "${sizes%% *}" = ok ] && echo 1 || echo 0)" "$sizes" ||
            bad=$((bad + 1))
        summary="$summary wav:$sizes"
    else
        kit_check "$log" recording_file_sizes 0 "no $wav"
        bad=$((bad + 1))
    fi
    # The audio chain's levels and the speakers' mix on this libmpv's FFmpeg.
    kit_cargo_test "$log" -p rvp-app --test audio_chain --test speakers -- --test-threads=2 ||
        bad=$((bad + 1))
    kit_result m10 "$bad" "$summary"
}

step_m11() {
    local log=$LOG_DIR/m11.log out=$LOG_DIR/m11-out data=$LOG_DIR/m11-data media bad=0 summary word
    local boot dry info
    m2_ready m11 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out" "$data"
    : > "$log"
    boot=$(uptime -s)
    # The trash goes to the step's own data folder: the user's is left
    # alone (the log folder's filesystem has it as the home trash).
    (
        export XDG_DATA_HOME=$data
        kit_run m11 selftest --selftest=m11 "--out=$out" "--dvd-folder=$media/dvd" \
            "--audio=$media/fixture-3s.mkv" "--clip=$media/fixture-60s.mkv" --mpv-options=ao=null \
            /seek=0:00:03 '/title=M11 title' \
            "$media/fixture-chapters.mkv" "$media/fixture-60s.mkv" "$media/fixture-3s.mkv"
    )
    word=$(kit_judge m11 selftest power_action_dry_run recycle_bin dvd_folder_plays) || bad=$((bad + 1))
    summary=$word
    # The kit's libmpv opens DVDs: the DVD step runs, it is not skipped.
    if grep -q '^check dvd_folder_plays=PASS SKIPPED' "$LOG_DIR/m11-selftest.out"; then
        kit_check "$log" dvd_not_skipped 0 "this libmpv opens no DVD"
        bad=$((bad + 1))
    fi
    # Power actions only through the dry-run hook, and the machine stays up.
    dry=$(cat "$LOG_DIR/m11-state/power-dry-run.txt" 2> /dev/null)
    kit_check "$log" power_dry_run_file "$(case $dry in *"shut down"*) echo 1 ;; *) echo 0 ;; esac)" "$dry" ||
        bad=$((bad + 1))
    kit_check "$log" machine_kept_running "$([ "$(uptime -s)" = "$boot" ] && echo 1 || echo 0)" ||
        bad=$((bad + 1))
    # The trash: the self-test's own file is gone, in the trash with its
    # info (rvp_platform::trash).
    info=$data/Trash/info/recycle-me.mkv.trashinfo
    kit_check "$log" trashed_file_gone "$([ ! -e "$out/recycle-me.mkv" ] && echo 1 || echo 0)" || bad=$((bad + 1))
    kit_check "$log" trashed_file_in_trash \
        "$([ -f "$data/Trash/files/recycle-me.mkv" ] && grep -q '^Path=.*/recycle-me\.mkv$' "$info" 2> /dev/null &&
            grep -q '^DeletionDate=[0-9]\{4\}-[0-9][0-9]-[0-9][0-9]T' "$info" && echo 1 || echo 0)" \
        "$(tr '\n' ' ' < "$info" 2> /dev/null)" || bad=$((bad + 1))
    # The thumbnail core's and M11's headless tests on this libmpv.
    (
        export RVP_TESTDATA=$media
        kit_cargo_test "$log" -p rvp-engine --test thumbnails -- --test-threads=1 --nocapture &&
            kit_cargo_test "$log" -p rvp-app --test m11 -- --test-threads=1 --nocapture
    ) || bad=$((bad + 1))
    kit_result m11 "$bad" "$summary"
}

step_m12() {
    local log=$LOG_DIR/m12.log out=$LOG_DIR/m12-out media bad=0 summary word
    m2_ready m12 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out"
    mkdir -p "$out"
    : > "$log"
    # The window's sizes, dialogs and moves (Wayland places windows itself:
    # the self-test skips the moves there), capture, the settings tree.
    kit_run m12 selftest --selftest=m12 "--out=$out" --mpv-options=ao=null "$media/fixture-60s.mkv"
    word=$(kit_judge m12 selftest) || bad=$((bad + 1))
    summary=$word
    # RVP's own shaders compiled by mpv with this system's OpenGL (Mesa
    # compiles what the Windows VM's OpenGL 3.0 cannot). Direct3D 11 is
    # Windows' renderer: no Linux run.
    kit_run m12 shaders --selftest=shaders "--out=$out" "--mpv-log=$out/mpv-shaders.log" \
        "--mpv-options=ao=null,log-file=$out/mpv-shaders.log" "$media/fixture-60s.mkv"
    word=$(kit_judge m12 shaders) || bad=$((bad + 1))
    summary="$summary $word"
    # The Vulkan renderer: mpv's gpu-next in RVP's X11 child window (RVP
    # starts through XWayland on Wayland); without a GPU on Mesa's software
    # Vulkan (lavapipe, which mpv takes with gpu-sw).
    local sw=""
    gpu_checks || sw=,gpu-sw=yes
    (
        KIT_RUN_SETTINGS='"render": { "renderer": "vulkan" }'
        kit_run m12 vulkan --selftest=vulkan "--out=$out/vulkan" "--mpv-log=$out/mpv-vulkan.log" \
            "--mpv-options=ao=null$sw,log-file=$out/mpv-vulkan.log" "$media/fixture-60s.mkv"
    )
    word=$(kit_judge m12 vulkan renderer_mpv_window) || bad=$((bad + 1))
    summary="$summary $word"
    (
        export RVP_TESTDATA=$media
        kit_cargo_test "$log" -p rvp-app --test m12 -- --test-threads=1 --nocapture &&
            kit_cargo_test "$log" -p rvp-engine --test headless -- --test-threads=1 --nocapture
    ) || bad=$((bad + 1))
    kit_result m12 "$bad" "$summary"
}

step_m13() {
    local log=$LOG_DIR/m13.log out=$LOG_DIR/m13-out media bad=0 summary word clip whisper=0 text
    local speech=()
    m2_ready m13 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out"
    mkdir -p "$out"
    : > "$log"
    # whisper.cpp: RVP_WHISPER_CLI, else the `whisper` step's build, with
    # RVP_WHISPER_MODELS (with ggml-tiny.bin); RVP downloads no model.
    if [ -z "${RVP_WHISPER_CLI:-}" ] && [ -x "$(whisper_out)/whisper-cli" ]; then
        RVP_WHISPER_CLI=$(whisper_out)/whisper-cli
    fi
    if [ -x "${RVP_WHISPER_CLI:-}" ] && [ -s "${RVP_WHISPER_MODELS:-}/ggml-tiny.bin" ]; then
        whisper=1
        speech=(--speech-check)
        export RVP_WHISPER_CLI RVP_WHISPER_MODELS
    fi
    clip=${RVP_SPEECH_CLIP:-$KIT_DIR/m13/speech.wav}
    if [ "$whisper" -eq 1 ] && [ ! -s "$clip" ] && command -v espeak-ng > /dev/null 2>&1; then
        "$REPO/tools/test-clips/make-m13-speech.sh" "$(dirname "$clip")" >> "$log" 2>&1
    fi
    kit_run m13 selftest --selftest=m13 "${speech[@]}" "--out=$out" --mpv-options=ao=null \
        "$media/fixture-chapters.mkv"
    word=$(kit_judge m13 selftest) || bad=$((bad + 1))
    summary=$word
    kit_check "$log" fixture_subtitles_unchanged \
        "$(grep -q 'First line' "$media/fixture-chapters.srt" && echo 1 || echo 0)" || bad=$((bad + 1))
    if [ "$whisper" -eq 1 ] && [ -s "$clip" ]; then
        "$RVP_WHISPER_CLI" -m "$RVP_WHISPER_MODELS/ggml-tiny.bin" -f "$clip" -l en -osrt \
            -of "$out/speech" -ng >> "$log" 2>&1
        text=$(tr -s '[:space:]' ' ' < "$out/speech.srt" 2> /dev/null)
        kit_check "$log" whisper_hears_the_clip "$(grep -qi hello <<< "$text" && echo 1 || echo 0)" "$text" ||
            bad=$((bad + 1))
        summary="$summary whisper:on"
    else
        kit_check "$log" whisper_hears_the_clip SKIPPED "no RVP_WHISPER_CLI, RVP_WHISPER_MODELS or clip"
        summary="$summary whisper:SKIPPED"
    fi
    # The subtitle files, RVP's subtitle work on a headless player, and the
    # subtitles made from speech (with a note without whisper.cpp).
    (
        export RVP_TESTDATA=$media
        [ -s "$clip" ] && export RVP_SPEECH_CLIP=$clip
        kit_cargo_test "$log" -p rvp-subtitle --test formats --test editor -- --test-threads=1 &&
            kit_cargo_test "$log" -p rvp-app --test subtitles --test speech -- --test-threads=1 --nocapture
    ) || bad=$((bad + 1))
    kit_result m13 "$bad" "$summary"
}

step_m14() {
    local log=$LOG_DIR/m14.log out=$LOG_DIR/m14-out media bad=0 summary="" word autostart before after
    m2_ready m14 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out"
    : > "$log"
    # The user's autostart entry: the self-test leaves it as it was. The
    # registry part (--registry-check) is the Windows VM's only.
    autostart=${XDG_CONFIG_HOME:-$HOME/.config}/autostart/rvp.desktop
    before=$(cat "$autostart" 2> /dev/null | md5sum)
    kit_run m14 default --selftest=m14 "--out=$out/default" --mpv-options=ao=null --skin= \
        "$media/fixture-60s.mkv" "$media/fixture-tone.ogg"
    word=$(kit_judge m14 default) || bad=$((bad + 1))
    summary=$word
    if [ -n "${RVP_SKIN:-}" ] && [ -e "$RVP_SKIN" ]; then
        kit_run m14 skin --selftest=m14 "--out=$out/skin" --mpv-options=ao=null "--skin=$RVP_SKIN" \
            "$media/fixture-60s.mkv" "$media/fixture-tone.ogg"
        word=$(kit_judge m14 skin) || bad=$((bad + 1))
        summary="$summary $word"
    else
        kit_check "$log" m14_skin SKIPPED "RVP_SKIN names no .dsf skin"
        summary="$summary skin:SKIPPED"
    fi
    after=$(cat "$autostart" 2> /dev/null | md5sum)
    kit_check "$log" autostart_left_as_it_was "$([ "$before" = "$after" ] && echo 1 || echo 0)" || bad=$((bad + 1))
    (
        [ -n "${RVP_SKIN:-}" ] && export RVP_TEST_SKIN=$RVP_SKIN
        kit_cargo_test "$log" -p rvp-app --test settings --test translations --test menus \
            --test spectrum_live -- --test-threads=1 &&
            kit_cargo_test "$log" -p rvp-skin --test synthetic -- --test-threads=1
    ) || bad=$((bad + 1))
    kit_result m14 "$bad" "$summary"
}

# Unified Remote's manager for m15 (its web-client protocol, as
# m15-all.ps1 drives it): `kit_ur_client MANAGER ACTION...` connects,
# authenticates and runs each action of the remote RVP.Player; prints
# check and unified_remote_ lines.
kit_ur_client() {
    python3 -I - "$@" << 'PY'
import json
import re
import sys
import time
import urllib.request
import uuid

manager, actions = sys.argv[1], sys.argv[2:]
source = "web-" + str(uuid.uuid4())


def get(path):
    with urllib.request.urlopen(f"{manager}/{path}", timeout=20) as answer:
        return answer.read().decode()


def send(connection, packet):
    packet["Source"] = source
    request = urllib.request.Request(
        f"{manager}/client/request", data=json.dumps(packet).encode(), method="POST",
        headers={"UR-Connection-ID": connection, "Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=20) as answer:
        return answer.read().decode()


try:
    connection = json.loads(get("client/connect"))["id"].strip()
    send(connection, {"Action": 0, "Request": 0, "Version": 10,
                      "Password": str(uuid.uuid4()), "Platform": "web"})
    auth = send(connection, {"Action": 1, "Request": 1, "Capabilities": {
        "Actions": True, "Sync": True, "Grid": True, "Fast": False, "Loading": True,
        "Encryption2": True}})
    ok = re.search(r'"Security"\s*:\s*1', auth) is not None
    print(f"check m15_unified_remote_client_connects={'PASS' if ok else 'FAIL'} {' '.join(auth.split())}")
    for action in actions:
        answer = send(connection, {"ID": "RVP.Player", "Action": 7, "Request": 7,
                                   "Run": {"Name": action}})
        print(f"unified_remote_run {action} -> {' '.join(answer.split())}")
        time.sleep(0.7)
    # The remote stops polling RVP.
    send(connection, {"ID": "RVP.Player", "Action": 4, "Request": 4})
    get("client/disconnect")
except Exception as error:  # noqa: BLE001 - every failure is the check's
    print(f"check m15_unified_remote_client_connects=FAIL {error}")
PY
}

# Waits until RVP's control API on PORT answers with the token in FILE
# (read here: never on a command line or in the log). Exit 0 when it does.
kit_api_up() {
    python3 -I - "$1" "$2" << 'PY'
import sys
import time
import urllib.request

port, path = sys.argv[1], sys.argv[2]
deadline = time.time() + 60
while time.time() < deadline:
    time.sleep(0.5)
    try:
        token = open(path).read().strip()
        request = urllib.request.Request(f"http://127.0.0.1:{port}/api/status",
                                         headers={"Authorization": "Bearer " + token})
        with urllib.request.urlopen(request, timeout=5) as answer:
            if answer.status == 200:
                sys.exit(0)
    except (OSError, ValueError):
        continue
sys.exit(1)
PY
}

# What Unified Remote Server's log says about RVP's remote: the lines the
# loader writes for it and the session's, then "clean" when the remote was
# loaded, run and no "(E)" or "(W)" line follows its loading.
m15_server_log() {
    awk '
        /Loader: loading remote: .*\/RVP\/meta\.prop/ { ours = 1; loaded = 1; print; next }
        /Loader: loading remote:/ { ours = 0; next }
        ours && /\((E|W)\)/ { bad = 1; print; next }
        /RVP\.Player/ { print; if (/Agent: loading: RVP\.Player/) used = 1 }
        END { print (loaded && used && !bad) ? "clean" : "not clean: loaded " loaded + 0 " used " used + 0 " errors " bad + 0 }
    ' "$1" 2> /dev/null || echo "not clean: no server log $1"
}

step_m15() {
    local log=$LOG_DIR/m15.log out=$LOG_DIR/m15-out media bad=0 summary word
    local ur=$LOG_DIR/m15-urserver server="" remotes pidfile state manager=http://127.0.0.1:9510
    local actions=(play_pause volume_down mute mute play_pause) rvp waited token up candidate
    local why="RVP_URSERVER names no unpacked Unified Remote Server"
    m2_ready m15 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out" "$ur"
    : > "$log"
    # Unified Remote Server for Linux unpacked (never installed): its
    # binary in RVP_URSERVER (the .deb's opt/urserver, or that folder).
    for candidate in "${RVP_URSERVER:-/nonexistent}/opt/urserver/urserver" "${RVP_URSERVER:-/nonexistent}/urserver"; do
        [ -x "$candidate" ] && { server=$candidate; break; }
    done
    # Unified Remote runs one server for a user: starting the step's
    # stopped the user's own, which their session starts (the owner's
    # boot, 2026-11-15; it came back 20 s later). While one runs, the
    # remote's part is left out.
    if [ -n "$server" ] && pgrep -u "$(id -u)" -x urserver > /dev/null; then
        server=""
        why="the user's own Unified Remote Server runs, and a second one would stop it (/opt/urserver/urserver-stop stops it until the next login)"
    fi
    if [ -z "$server" ]; then
        # The control API alone.
        kit_run m15 selftest --selftest=m15 "--out=$out" --mpv-options=ao=null "$media/fixture-60s.mkv"
        word=$(kit_judge m15 selftest) || bad=$((bad + 1))
        kit_check "$log" m15_unified_remote SKIPPED "$why"
        summary="$word unified_remote:SKIPPED"
    else
        # The server in user space: its data in the step's folder (HOME),
        # its remotes the bundled ones and RVP's in custom/RVP, with the
        # pictures m8package drew (the icons and the layout's mute.png,
        # without which the server does not load the remote) when they
        # are there. The remote reads RVP's token where an installed RVP
        # keeps it: $XDG_STATE_HOME/rvp.
        remotes=$ur/remotes
        pidfile=$ur/urserver.pid
        state=$ur/state
        mkdir -p "$remotes/custom" "$ur/home" "$state"
        cp -a "$(dirname "$server")/remotes" "$remotes/bundled"
        cp -a "$REPO/remote/unified-remote/RVP" "$remotes/custom/RVP"
        cp "$(m8_dir)"/look/remote/unified-remote/RVP/*.png "$remotes/custom/RVP/" 2> /dev/null
        HOME=$ur/home XDG_STATE_HOME=$state "$server" "--remotes=$remotes" --daemon "--pidfile=$pidfile" \
            >> "$log" 2>&1
        for _ in $(seq 1 30); do
            curl -fs "$manager/system/status" > "$ur/status.json" 2> /dev/null && break
            sleep 1
        done
        kit_check "$log" m15_unified_remote_runs \
            "$(grep -q '"running" *: *true' "$ur/status.json" 2> /dev/null && echo 1 || echo 0)" \
            "$(grep -o '"version" *: *"[^"]*"' "$ur/status.json" 2> /dev/null)" || bad=$((bad + 1))
        curl -fs "$manager/system/reload" > /dev/null 2>&1
        sleep 3
        # RVP serves first (--remote-serve) on the remote's port, with its
        # state where the remote reads the token.
        export RVP_CONFIG_DIR=$LOG_DIR/m15-config RVP_STATE_DIR=$state/rvp
        rm -rf "$RVP_CONFIG_DIR" "$RVP_STATE_DIR"
        kit_settings
        echo "== selftest (served)" >> "$log"
        "$(rvp_bin)" --selftest=m15 --remote-port=48931 --remote-serve=90 "--remote-expect=${#actions[@]}" \
            "--out=$out" --mpv-options=ao=null "$media/fixture-60s.mkv" > "$LOG_DIR/m15-selftest.out" 2>&1 &
        rvp=$!
        kit_api_up 48931 "$RVP_STATE_DIR/remote-token"
        up=$?
        kit_check "$log" m15_api_answers_the_script "$([ "$up" -eq 0 ] && echo 1 || echo 0)" ||
            bad=$((bad + 1))
        sleep 2
        kit_ur_client "$manager" "${actions[@]}" >> "$log" 2>&1
        grep -q '^check m15_unified_remote_client_connects=PASS' "$log" || bad=$((bad + 1))
        waited=0
        while kill -0 "$rvp" 2> /dev/null && [ "$waited" -lt 120 ]; do
            sleep 1
            waited=$((waited + 1))
        done
        if kill -0 "$rvp" 2> /dev/null; then
            kill "$rvp"
            kit_check "$log" m15_selftest_ends 0 killed
            bad=$((bad + 1))
        fi
        wait "$rvp"
        echo "kit_exit=$?" >> "$LOG_DIR/m15-selftest.out"
        cat "$LOG_DIR/m15-selftest.out" >> "$log"
        word=$(kit_judge m15 selftest m15_outside_commands) || bad=$((bad + 1))
        summary="$word unified_remote:on"
        # The token stays out of RVP's log (grep reads it from the file).
        if [ -s "$RVP_STATE_DIR/remote-token" ]; then
            token=$RVP_STATE_DIR/remote-token
            # One pattern line: the token without its newline.
            kit_check "$log" m15_token_not_logged \
                "$(grep -q -F -f <(tr -d '[:space:]' < "$token"; echo) "$LOG_DIR/m15-selftest.out" &&
                    echo 0 || echo 1)" || bad=$((bad + 1))
        fi
        # The server's own log (its HOME is the step's): RVP's remote was
        # loaded and used, and no error or warning follows the loader's
        # "loading remote: .../RVP/meta.prop" before the next remote. The
        # loader's errors do not name the remote ("(E) Loader: could not
        # load layout: File not found" for a missing picture).
        m15_server_log "$ur/home/.urserver/urserver.log" > "$ur/log.txt"
        tail -12 "$ur/log.txt" | sed 's/^/unified_remote_log /' >> "$log"
        kit_check "$log" m15_unified_remote_log_clean \
            "$(grep -q '^clean$' "$ur/log.txt" && echo 1 || echo 0)" || bad=$((bad + 1))
        # Its own process only, by its pidfile.
        [ -s "$pidfile" ] && kill "$(cat "$pidfile")" 2> /dev/null
    fi
    # The control API and the remote's files, headless.
    (
        export RVP_TESTDATA=$media RVP_REMOTE_DIR=$REPO/remote/unified-remote/RVP
        kit_cargo_test "$log" -p rvp-app --test control --test remote -- --test-threads=1
    ) || bad=$((bad + 1))
    kit_result m15 "$bad" "$summary"
}

step_m16() {
    local log=$LOG_DIR/m16.log out=$LOG_DIR/m16-out bad=0 summary word dvd bd lib must=(m16_menu_shows)
    m2_ready m16 || return 0
    m2_env
    rm -rf "$out"
    : > "$log"
    # The synthetic discs (never committed): made once in the kit's
    # folder, or the user's (RVP_DVD_MENU, RVP_BD_MENU).
    dvd=${RVP_DVD_MENU:-$KIT_DIR/m16-dvd}
    bd=${RVP_BD_MENU:-$KIT_DIR/m16-bd}
    if [ ! -s "$dvd/VIDEO_TS/VIDEO_TS.IFO" ] && command -v dvdauthor > /dev/null 2>&1 &&
        command -v spumux > /dev/null 2>&1; then
        "$REPO/tools/test-clips/make-m16-dvd.sh" "$dvd" >> "$log" 2>&1
    fi
    if [ ! -d "$bd/BDMV" ]; then
        python3 -I "$REPO/tools/test-clips/make-m16-bluray.py" "$bd" >> "$log" 2>&1
    fi
    if [ ! -s "$dvd/VIDEO_TS/VIDEO_TS.IFO" ]; then
        record m16 SKIPPED "no synthetic DVD: install dvdauthor (it has spumux) or set RVP_DVD_MENU"
        return 0
    fi
    # RVP's navigators load the system's libdvdnav and libbluray (.so.4
    # from 1.4 on, Ubuntu 26.04 and later; .so.2 before), never the
    # development package's bare name; no libdvdcss, libaacs or keys come
    # with RVP.
    for lib in libdvdnav.so.4 "libbluray.so.4 libbluray.so.2"; do
        kit_check "$log" "m16_system_has_${lib%% *}" \
            "$(/sbin/ldconfig -p | awk -v libs="$lib" 'BEGIN { n = split(libs, want, " ") }
                { for (i = 1; i <= n; i++) if ($1 == want[i]) found = 1 } END { exit !found }' &&
                echo 1 || echo 0)" "$lib" || bad=$((bad + 1))
    done
    [ -d "$bd/BDMV" ] && must+=(m16_bd_menu_shows)
    kit_run m16 selftest --selftest=m16 "--out=$out" --mpv-options=ao=null "--dvd-menus=$dvd" "--bd-menus=$bd"
    word=$(kit_judge m16 selftest "${must[@]}") || bad=$((bad + 1))
    summary=$word
    # The shell's navigation tests on the same discs; none may be skipped.
    (
        export RVP_DVD_MENU=$dvd RVP_BD_MENU=$bd
        kit_cargo_test "$log" -p rvp-shell --bin rvp -- synthetic --test-threads=1 --nocapture
    ) || bad=$((bad + 1))
    if grep -E '^note: .*skipped$' "$log" | grep -q -v RVP_DVD_REAL; then
        kit_check "$log" headless_disc_ran 0 "$(grep -E '^note: .*skipped$' "$log" | tr '\n' ' ')"
        bad=$((bad + 1))
    fi
    kit_result m16 "$bad" "$summary"
}

step_m18() {
    local log=$LOG_DIR/m18.log out=$LOG_DIR/m18-out media bad=0 summary="" word mode vsscript
    m2_ready m18 || return 0
    m2_env
    media=$(kit_media)
    rm -rf "$out"
    : > "$log"
    # A VapourSynth runtime (VSSCRIPT_PATH, the `vapoursynth` step's or the
    # system's); without one the smoothing is Blend, which the self-tests
    # check instead of MVTools.
    vsscript=${VSSCRIPT_PATH:-}
    if [ -z "$vsscript" ] && [ -e "$(vs_runtime)/libvapoursynth-script.so.0" ]; then
        vsscript=$(vs_runtime)/libvapoursynth-script.so.0
    fi
    [ -n "$vsscript" ] || vsscript=$(find_vsscript)
    echo "vsscript=${vsscript:-none}" >> "$log"
    for mode in smoothing gpu; do
        (
            export PYTHONDONTWRITEBYTECODE=1
            [ -n "$vsscript" ] && export VSSCRIPT_PATH=$vsscript
            kit_run m18 "$mode" "--selftest=$mode" "--out=$out/$mode" --mpv-options=ao=null \
                "$media/fixture-60s.mkv"
        )
        word=$(kit_judge m18 "$mode") || bad=$((bad + 1))
        summary="$summary $word"
    done
    # The moving square at twice the rate (with a note without a runtime);
    # the GPU modes (RIFE, NVIDIA Optical Flow) need a GPU: the owner's
    # Ubuntu boot (CHECKLIST.md).
    (
        export RVP_TESTDATA=$media PYTHONDONTWRITEBYTECODE=1
        [ -n "$vsscript" ] && export VSSCRIPT_PATH=$vsscript
        kit_cargo_test "$log" -p rvp-app --test smoothing -- --test-threads=1 --nocapture
    ) || bad=$((bad + 1))
    gpu_checks || summary="$summary gpu_modes:SKIPPED"
    kit_result m18 "$bad" "vsscript=${vsscript:-none}$summary"
}

# The card's power limit and its default in whole watts, "LIMIT DEFAULT".
nvidia_power() {
    nvidia-smi --query-gpu=power.limit,power.default_limit --format=csv,noheader,nounits 2> /dev/null |
        head -1 | awk -F', *' '{ printf "%d %d\n", $1, $2 }'
}

step_m18trt() {
    local log=$LOG_DIR/m18trt.log runtime power limit default before after code line bad=0 summary
    if [ "$(gpu_vendor)" != nvidia ]; then
        record m18trt SKIPPED "no NVIDIA card"
        return 0
    fi
    runtime=$(vs_runtime)
    if [ ! -e "$runtime/vs-mlrt/libvstrt.so" ]; then
        record m18trt SKIPPED "no vs-mlrt in RVP's VapourSynth runtime: run vapoursynth first"
        return 0
    fi
    # The card's power limit goes into the result. The owner dropped D17's
    # 90 % limit for the Ubuntu boot on 2026-11-14 (the RTX 50-series card).
    power=$(nvidia_power)
    limit=${power% *}
    default=${power#* }
    : > "$log"
    before=$(nvidia-smi --query-gpu=name,driver_version,compute_cap,power.draw,power.limit,temperature.gpu \
        --format=csv,noheader 2>&1)
    echo "before: $before" >> "$log"
    (
        cd "$REPO" || exit 1
        spike_env
        export QMAKE MPV_LIB_DIR VSSCRIPT_PATH=$runtime/libvapoursynth-script.so.0 PYTHONDONTWRITEBYTECODE=1
        QMAKE=$(qt_dir)/bin/qmake
        MPV_LIB_DIR=$(mpv_lib_dir)
        cargo run --release -p rvp-app --example rife_trt -- "$KIT_DIR/tensorrt" "${RVP_TENSORRT_MIRROR:--}" medium
    ) >> "$log" 2>&1
    code=$?
    echo "rife_trt exit $code" >> "$log"
    # A card that no longer answers stops everything (AGENTS.md).
    if ! after=$(nvidia-smi --query-gpu=name,power.draw,temperature.gpu --format=csv,noheader 2>&1); then
        echo "after: $after" >> "$log"
        record m18trt FAIL "nvidia-smi no longer answers after the run: stop and tell the owner"
        exit 1
    fi
    echo "after: $after" >> "$log"
    summary="$(grep -m1 '^installation ' "$log")"
    kit_check "$log" tensorrt_installed "$(grep -q '^installation ' "$log" && echo 1 || echo 0)" "$summary" ||
        bad=$((bad + 1))
    for size in 1280x720 1920x1080 2560x1440; do
        line=$(grep -m1 "^size=$size " "$log")
        summary="$summary, ${line:-size=$size missing}"
    done
    # 1080p at 60 frames a second, as the calibration asks of an engine.
    line=$(grep -m1 '^size=1920x1080 ' "$log" | sed -n 's/.*output_fps=\([0-9.]*\).*/\1/p')
    kit_check "$log" rife_trt_keeps_1080p60 "$(awk -v fps="${line:-0}" 'BEGIN { print (fps >= 60) ? 1 : 0 }')" \
        "${line:-no measurement} fps" || bad=$((bad + 1))
    [ "$code" -eq 0 ] || bad=$((bad + 1))
    kit_result m18trt "$bad" "$summary, power limit $limit W of $default W"
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
        for step in s2 s3 input s1 m2 m3 m4 m6 m7 m8 m9 m10 m11 m12 m13 m14 m15 m16 m18 m18trt m2input m2gpu m6gpu \
            m2svp m6svp; do
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
    # The header's comment lines.
    awk 'NR > 1 && /^#/ { print; next } NR > 1 { exit }' "$KIT_SCRIPT"
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
        m17all) set -- check deps mpv mvtools whisper vapoursynth m2build m2 m3 m4 m6 m7 m8package m8 m9 m10 m11 m12 m13 m14 m15 m16 \
            m18 m18trt m2input m2gpu m6gpu m2svp m6svp report ;;
    esac
done
overall=0
for step in "$@"; do
    case $step in
        check | deps | mpv | mvtools | whisper | vapoursynth | build | s1 | s2 | s3 | input | m2build | m2 | m2input | m2gpu | m2svp | m3 | m4 | m6 | m7 | m8package | m8 | m8removed | m9 | m10 | m11 | m12 | m13 | m14 | m15 | m16 | m18 | m18trt | m6svp | m6gpu | m3desktop | report)
            "step_$step" || overall=1
            ;;
        all | m2all | m3all | m4all | m6all | m7all | m8all | m17all) ;;
        *) say "unknown step $step"; exit 2 ;;
    esac
done
exit "$overall"
