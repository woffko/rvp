#!/usr/bin/env bash
# Runs build.sh without sudo (goal T): in a build root of its own, an
# Ubuntu 24.04 base image unpacked into ROOT and entered as root of a user
# namespace that maps only the caller's own user. The packages README.md
# lists go into that root, never into the system; the work folder and this
# checkout (read-only) are mounted inside at their own paths.
#
# usage: tools/libmpv/rootless.sh [ROOT [WORK]]
#        (default ~/rvp-deps/mpv-root and ~/rvp-deps/mpv-build)
set -euo pipefail

BASE=ubuntu-base-24.04.5-base-amd64.tar.gz
BASE_URL=https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release
ROOT=${1:-$HOME/rvp-deps/mpv-root}
WORK=${2:-$HOME/rvp-deps/mpv-build}
HERE=$(cd "$(dirname "$0")" && pwd)
CHECKOUT=$(cd "$HERE/../.." && pwd)
PACKAGES="build-essential bison flex gettext git mercurial subversion ninja-build gyp cmake yasm nasm
    automake autoconf pkgconf libtool libtool-bin gcc-multilib g++-multilib clang lld libc++-dev libc++abi-dev
    libgmp-dev libmpfr-dev libmpc-dev libgcrypt20-dev gperf ragel texinfo autopoint re2c asciidoc
    python3-pip python3-venv docbook2x unzip p7zip-full curl ccache ca-certificates xz-utils bzip2
    patch file wget"
PIP="meson==1.12.1 mako jsonschema rst2pdf jinja2"

# Runs a command as root of the build root.
inroot() {
    unshare --user --map-root-user --mount --pid --fork --kill-child -- /bin/bash -euo pipefail -c '
        root=$1 work=$2 checkout=$3
        shift 3
        mount --rbind /dev "$root/dev"
        mount -t proc proc "$root/proc"
        mount --rbind /sys "$root/sys"
        mkdir -p "$root$work" "$root$checkout"
        mount --bind "$work" "$root$work"
        mount --bind "$checkout" "$root$checkout"
        mount -o remount,bind,ro "$root$checkout"
        cp -L /etc/resolv.conf "$root/etc/resolv.conf"
        exec chroot "$root" /usr/bin/env -i HOME=/root LANG=C.UTF-8 TERM=dumb \
            PATH=/opt/venv/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin "$@"
    ' inroot "$ROOT" "$WORK" "$CHECKOUT" "$@"
}

mkdir -p "$WORK"
if [ ! -x "$ROOT/opt/venv/bin/meson" ]; then
    # The base image, checked against Ubuntu's signed checksums.
    download=$(mktemp -d)
    for file in "$BASE" SHA256SUMS SHA256SUMS.gpg; do
        curl -sfL -o "$download/$file" "$BASE_URL/$file"
    done
    gpgv --keyring /usr/share/keyrings/ubuntu-archive-keyring.gpg "$download/SHA256SUMS.gpg" "$download/SHA256SUMS"
    (cd "$download" && grep "$BASE\$" SHA256SUMS | sha256sum -c -)
    mkdir -p "$ROOT"
    unshare --user --map-root-user tar --numeric-owner --no-same-owner -xzf "$download/$BASE" -C "$ROOT"
    rm -r "$download"
    # With one user mapped, apt stays root instead of taking its sandbox user.
    printf 'APT::Sandbox::User "root";\n' > "$ROOT/etc/apt/apt.conf.d/99rootless"
    # shellcheck disable=SC2086 # the lists are meant to split
    inroot /bin/bash -c "apt-get update -q && DEBIAN_FRONTEND=noninteractive apt-get install -y -q \
        --no-install-recommends $(echo $PACKAGES) && python3 -m venv /opt/venv && /opt/venv/bin/pip install -q $PIP"
fi
# The recipe applies its patches with `git am`, which needs a committer:
# a name for the build root's own commits (they stay in its sources).
if [ ! -f "$ROOT/root/.gitconfig" ]; then
    printf '[user]\n\tname = RVP libmpv build\n\temail = libmpv-build@localhost\n' > "$ROOT/root/.gitconfig"
fi
inroot /bin/bash -c "CCACHE_DIR=$WORK/ccache $CHECKOUT/tools/libmpv/build.sh $WORK"
