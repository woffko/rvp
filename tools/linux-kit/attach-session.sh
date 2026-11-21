#!/usr/bin/env bash
# Lets the kit, run by one user over SSH, open its windows in another
# user's graphical session (the shared Ubuntu VM: the kit runs as the kit's user
# in the owner's GNOME session), as goals B to J did by hand. Needs sudo; every
# change is an ACL entry for the kit's user that `detach` takes back.
#
# usage (as the kit's user):
#   tools/linux-kit/attach-session.sh attach SESSION_USER   # prints exports
#   tools/linux-kit/attach-session.sh detach SESSION_USER
#
# attach gives the kit's user access to SESSION_USER's runtime folder (to
# pass through it), the Wayland socket and XWayland's authority file, and
# prints WAYLAND_DISPLAY (the socket's absolute path), DISPLAY,
# XAUTHORITY and PULSE_SERVER for `eval`. The kit's user keeps its own
# runtime folder, session bus and sound server (named in PULSE_SERVER):
# MPRIS and the screen-saver checks use a private bus where the desktop's
# refuses another user.
#
# The session's user owns its runtime folder and so what is in it: an
# entry goes only on the folder itself, a socket `wayland-0` and regular
# files named as mutter names XWayland's authority files, each owned by
# that user, and setfacl never follows a link (-P). What attach prints is
# quoted for the shell. RVP_ATTACH_ROOT puts /run/user under another
# folder (tools/linux-kit/tests/attach-session.sh).
set -euo pipefail

action=${1:?usage: attach-session.sh attach|detach SESSION_USER}
owner=${2:?usage: attach-session.sh attach|detach SESSION_USER}
me=$(id -un)
uid=$(id -u "$owner")
root=${RVP_ATTACH_ROOT:-}
runtime=$root/run/user/$uid
# XWayland's authority file as mutter names it, in the session's folder.
auth_pattern='^/run/user/[0-9]+/\.mutter-Xwaylandauth\.[A-Za-z0-9]+$'

# kind PATH: its type and owner without following a link ("socket:1000").
kind() {
    sudo -n stat -c '%F:%u' -- "$1" 2> /dev/null || echo missing
}

# The session's authority files: regular files of the session's user,
# each name ended by a NUL (find cannot enter some of the session's
# folders, such as gvfs).
auth_files() {
    sudo -n find "$runtime" -maxdepth 1 -name '.mutter-Xwaylandauth.*' -type f -user "$uid" -print0 2> /dev/null || true
}

case $action in
    attach)
        if [ "$(kind "$runtime")" != "directory:$uid" ]; then
            echo "attach-session.sh: $runtime is not $owner's folder" >&2
            exit 1
        fi
        if [ "$(kind "$runtime/wayland-0")" != "socket:$uid" ]; then
            echo "attach-session.sh: $runtime/wayland-0 is not $owner's Wayland socket" >&2
            exit 1
        fi
        auth=""
        while IFS= read -r -d '' file; do
            if [[ ${file#"$root"} =~ $auth_pattern ]]; then
                auth=$file
                break
            fi
            echo "attach-session.sh: skipped an authority file of another name" >&2
        done < <(auth_files)
        sudo -n setfacl -P -m "u:$me:x" -- "$runtime"
        sudo -n setfacl -P -m "u:$me:rw" -- "$runtime/wayland-0"
        [ -z "$auth" ] || sudo -n setfacl -P -m "u:$me:r" -- "$auth"
        printf 'export WAYLAND_DISPLAY=%q\n' "$runtime/wayland-0"
        # GNOME starts XWayland on :0.
        echo "export DISPLAY=:0"
        [ -z "$auth" ] || printf 'export XAUTHORITY=%q\n' "$auth"
        echo "export XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=ubuntu:GNOME"
        # The kit's user's own sound server: with DISPLAY set, libpulse
        # would take the session's from XWayland's root window, which
        # refuses another user.
        printf 'export PULSE_SERVER=%q\n' "unix:/run/user/$(id -u)/pulse/native"
        ;;
    detach)
        sudo -n setfacl -P -x "u:$me" -- "$runtime/wayland-0" 2> /dev/null || true
        # Every authority file: attach may have taken another one than the
        # first that find gives now.
        while IFS= read -r -d '' file; do
            sudo -n setfacl -P -x "u:$me" -- "$file" 2> /dev/null || true
        done < <(auth_files)
        sudo -n setfacl -P -x "u:$me" -- "$runtime" 2> /dev/null || true
        echo "detached"
        ;;
    *)
        echo "usage: attach-session.sh attach|detach SESSION_USER" >&2
        exit 2
        ;;
esac
