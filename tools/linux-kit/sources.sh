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

# kit_clone URL DIR COMMIT: a working tree of URL at COMMIT in DIR, from
# the source cache when it has the repository. COMMIT is a full commit
# hash (a tag or a branch can move; the callers name the tag beside it),
# and the clone fails unless DIR's HEAD is that commit.
kit_clone() {
    local url=$1 dir=$2 commit=$3 cache mirror
    if ! [[ $commit =~ ^[0-9a-f]{40}$ ]]; then
        echo "kit_clone: $url: $commit is not a full commit hash" >&2
        return 1
    fi
    cache=${RVP_KIT_SOURCES:-}/$(basename "$url" .git).git
    if [ -n "${RVP_KIT_SOURCES:-}" ] && [ -d "$cache" ]; then
        git clone -q --no-checkout "$cache" "$dir" || return 1
        git -C "$dir" remote set-url origin "$url"
    elif ! git clone -q --filter=blob:none --no-checkout "$url" "$dir"; then
        # A host that does not answer: its mirror, when it has one.
        mirror=$(kit_mirror "$url")
        [ -n "$mirror" ] || return 1
        echo "kit_clone: $url failed, trying $mirror" >&2
        rm -rf "$dir"
        git clone -q --filter=blob:none --no-checkout "$mirror" "$dir" || return 1
    fi
    git -C "$dir" -c advice.detachedHead=false checkout -q "$commit" || return 1
    if [ "$(git -C "$dir" rev-parse HEAD)" != "$commit" ]; then
        echo "kit_clone: $url: $dir is not at $commit" >&2
        return 1
    fi
}

# The mirror of a repository whose host may not answer: VideoLAN's
# projects on GitHub (code.videolan.org's GitLab gave 502 and refused
# connections on 2026-11-14 and 15, and CI's package-linux failed with it).
kit_mirror() {
    case $1 in
        https://code.videolan.org/videolan/*) echo "https://github.com/videolan/${1#https://code.videolan.org/videolan/}" ;;
    esac
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
