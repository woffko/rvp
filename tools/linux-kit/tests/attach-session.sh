#!/usr/bin/env bash
# Checks tools/linux-kit/attach-session.sh without root and without a
# graphical session: a runtime folder of the caller's under a folder of its
# own (RVP_ATTACH_ROOT), and a `sudo` and a `setfacl` first on the PATH:
# sudo runs the command as the caller, setfacl writes its arguments to a
# log. A link planted as the Wayland socket or the runtime folder is
# refused, an authority file whose name is not mutter's gets no entry and
# its name is never run by the printed exports' `eval`, and detach takes
# the entries back from every authority file. CI's Linux job runs it.
#
# usage: tools/linux-kit/tests/attach-session.sh
set -euo pipefail

script=$(cd "$(dirname "$0")/.." && pwd)/attach-session.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
uid=$(id -u)
user=$(id -un)
root=$work/root
runtime=$root/run/user/$uid
log=$work/setfacl.log
failures=0

check() { # check NAME CONDITION...: PASS when the command succeeds
    local name=$1
    shift
    if "$@"; then
        echo "check $name=PASS"
    else
        echo "check $name=FAIL"
        failures=$((failures + 1))
    fi
}

mkdir -p "$work/bin"
cat > "$work/bin/sudo" << 'EOF'
#!/usr/bin/env bash
[ "${1:-}" != -n ] || shift
exec "$@"
EOF
cat > "$work/bin/setfacl" << EOF
#!/usr/bin/env bash
printf '%q ' "\$@" >> '$log'
echo >> '$log'
EOF
chmod +x "$work/bin/sudo" "$work/bin/setfacl"

# A socket at a relative path (a Unix socket's path is short).
socket() {
    (cd "$(dirname "$1")" && python3 -I -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' \
        "$(basename "$1")")
}

# A fresh session folder: the socket and, when named, authority files.
session() {
    rm -rf "$root" "$log"
    : > "$log"
    mkdir -p "$runtime"
    chmod 0700 "$runtime"
    socket "$runtime/wayland-0"
    local name
    for name in "$@"; do
        : > "$runtime/$name"
    done
}

# Runs the script with the fakes; its output in $work/out, its status in
# $status.
run() {
    status=0
    (cd "$work" && PATH=$work/bin:$PATH RVP_ATTACH_ROOT=$root "$script" "$@") > "$work/out" 2> "$work/err" ||
        status=$?
}

# What the printed exports set, through eval as the checklist does, from
# the work folder (where a name that ran would leave its file).
evaluated() {
    (cd "$work" && unset XAUTHORITY WAYLAND_DISPLAY && eval "$(cat "$work/out")" &&
        printf '%s\n%s\n' "${WAYLAND_DISPLAY:-}" "${XAUTHORITY:-}")
}

calls() { grep -c . "$log" || true; }
every_call_physical() { ! grep -v '^-P ' "$log" | grep -q .; }
named() { grep -qF -- "$(printf '%q' "$1")" "$log"; }

# 1. A session as GNOME makes it.
good=.mutter-Xwaylandauth.AB12cd
session "$good"
run attach "$user"
check attach_status [ "$status" = 0 ]
check attach_exports [ "$(evaluated)" = "$(printf '%s\n%s' "$runtime/wayland-0" "$runtime/$good")" ]
check attach_three_entries [ "$(calls)" = 3 ]
check attach_no_link_followed every_call_physical
check attach_folder named "$runtime"
check attach_socket named "$runtime/wayland-0"
check attach_authority named "$runtime/$good"

# 2. Authority files of other names, a link among them: no entry, nothing
# run by eval, no XAUTHORITY; then a good one beside them is the one taken.
hostile=(".mutter-Xwaylandauth.a;touch pwned1" '.mutter-Xwaylandauth.$(touch pwned2)'
    '.mutter-Xwaylandauth.`touch pwned3`' ".mutter-Xwaylandauth.a b" $'.mutter-Xwaylandauth.a\ntouch pwned4')
session "${hostile[@]}"
: > "$work/elsewhere"
ln -s "$work/elsewhere" "$runtime/.mutter-Xwaylandauth.Link01"
run attach "$user"
check hostile_status [ "$status" = 0 ]
check hostile_no_xauthority [ "$(evaluated)" = "$runtime/wayland-0" ]
check hostile_nothing_ran [ -z "$(cd "$work" && ls pwned* 2> /dev/null)" ]
check hostile_two_entries [ "$(calls)" = 2 ]
check hostile_link_untouched bash -c '! grep -qF -- "$1" "$2"' _ "Link01" "$log"
: > "$runtime/.mutter-Xwaylandauth.Good99"
: > "$log"
run attach "$user"
check hostile_good_taken [ "$(evaluated | tail -1)" = "$runtime/.mutter-Xwaylandauth.Good99" ]
check hostile_good_entry named "$runtime/.mutter-Xwaylandauth.Good99"
check hostile_still_nothing_ran [ -z "$(cd "$work" && ls pwned* 2> /dev/null)" ]

# 3. A link planted as the Wayland socket: refused before any entry.
session "$good"
rm "$runtime/wayland-0"
socket "$work/other-socket"
ln -s "$work/other-socket" "$runtime/wayland-0"
run attach "$user"
check link_socket_refused [ "$status" != 0 ]
check link_socket_no_entry [ "$(calls)" = 0 ]
check link_socket_no_exports [ ! -s "$work/out" ]

# 4. A link as the runtime folder: refused.
session "$good"
mv "$runtime" "$work/moved"
ln -s "$work/moved" "$runtime"
run attach "$user"
check link_folder_refused [ "$status" != 0 ]
check link_folder_no_entry [ "$(calls)" = 0 ]
rm "$runtime"

# 5. detach: the socket, the folder and every authority file.
session "$good" .mutter-Xwaylandauth.Second2
run detach "$user"
check detach_status [ "$status" = 0 ]
check detach_four_entries [ "$(calls)" = 4 ]
check detach_no_link_followed every_call_physical
check detach_first named "$runtime/$good"
check detach_second named "$runtime/.mutter-Xwaylandauth.Second2"

echo "failures=$failures"
[ "$failures" = 0 ]
