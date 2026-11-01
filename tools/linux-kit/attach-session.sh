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
# prints WAYLAND_DISPLAY (the socket's absolute path), DISPLAY and
# XAUTHORITY for `eval`. The kit's user keeps its own runtime folder,
# session bus and sound server: MPRIS and the screen-saver checks use a
# private bus where the desktop's refuses another user.
set -euo pipefail

action=${1:?usage: attach-session.sh attach|detach SESSION_USER}
owner=${2:?usage: attach-session.sh attach|detach SESSION_USER}
me=$(id -un)
uid=$(id -u "$owner")
runtime=/run/user/$uid
# (find cannot enter some of the session's folders, such as gvfs.)
auth=$(sudo -n find "$runtime" -maxdepth 1 -name '.mutter-Xwaylandauth.*' 2> /dev/null | head -1 || true)

case $action in
    attach)
        sudo -n setfacl -m "u:$me:x" "$runtime"
        sudo -n setfacl -m "u:$me:rw" "$runtime/wayland-0"
        [ -n "$auth" ] && sudo -n setfacl -m "u:$me:r" "$auth"
        echo "export WAYLAND_DISPLAY=$runtime/wayland-0"
        # GNOME starts XWayland on :0.
        echo "export DISPLAY=:0"
        [ -n "$auth" ] && echo "export XAUTHORITY=$auth"
        echo "export XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=ubuntu:GNOME"
        ;;
    detach)
        sudo -n setfacl -x "u:$me" "$runtime/wayland-0" 2> /dev/null || true
        [ -n "$auth" ] && { sudo -n setfacl -x "u:$me" "$auth" 2> /dev/null || true; }
        sudo -n setfacl -x "u:$me" "$runtime" 2> /dev/null || true
        echo "detached"
        ;;
    *)
        echo "usage: attach-session.sh attach|detach SESSION_USER" >&2
        exit 2
        ;;
esac
