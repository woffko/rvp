#!/usr/bin/env bash
# The sources the Linux kit's builds clone (the SDK, whisper.cpp, the
# VapourSynth runtime), and a source cache for a machine whose network to
# the forges is poor (the shared Ubuntu VM's to GitHub stalled for minutes):
# with RVP_KIT_SOURCES naming a folder of bare clones, kit_clone takes a
# repository from there and only goes to the network for what is missing.
#
# Sourced by tools/linux-kit/kit.sh, tools/whisper/build-linux.sh and
# tools/vapoursynth/build-linux.sh for kit_clone. As a command,
#   tools/linux-kit/sources.sh mirror DIR
# makes or updates DIR with bare clones of every repository below (on a
# machine with a good network; copy DIR over with rsync or tar).

# Each repository by its URL; the cache keeps it as <last part>.git.
KIT_REPOSITORIES="
https://github.com/KhronosGroup/Vulkan-Headers.git
https://github.com/KhronosGroup/glslang.git
https://github.com/KhronosGroup/SPIRV-Tools.git
https://github.com/KhronosGroup/SPIRV-Headers.git
https://github.com/google/shaderc.git
https://code.videolan.org/videolan/dav1d.git
https://github.com/AviSynth/AviSynthPlus.git
https://github.com/ggml-org/whisper.cpp.git
https://github.com/sekrit-twc/zimg.git
https://github.com/vapoursynth/vapoursynth.git
https://github.com/dubhater/vapoursynth-mvtools.git
"

# kit_clone URL DIR REF: a working tree of URL at REF (a tag, branch or
# commit) in DIR, from the source cache when it has the repository.
kit_clone() {
    local url=$1 dir=$2 ref=$3 cache
    cache=${RVP_KIT_SOURCES:-}/$(basename "$url" .git).git
    if [ -n "${RVP_KIT_SOURCES:-}" ] && [ -d "$cache" ]; then
        git clone -q --no-checkout "$cache" "$dir" || return 1
        git -C "$dir" remote set-url origin "$url"
    else
        git clone -q --filter=blob:none --no-checkout "$url" "$dir" || return 1
    fi
    git -C "$dir" -c advice.detachedHead=false checkout -q "$ref"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    set -euo pipefail
    [ "${1:-}" = mirror ] && [ -n "${2:-}" ] || { echo "usage: sources.sh mirror DIR" >&2; exit 2; }
    mkdir -p "$2"
    for url in $KIT_REPOSITORIES; do
        target=$2/$(basename "$url" .git).git
        if [ -d "$target" ]; then
            git -C "$target" fetch -q --prune --tags origin '+refs/heads/*:refs/heads/*'
        else
            git clone -q --bare "$url" "$target"
        fi
        echo "$target"
    done
fi
