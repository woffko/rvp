# Goal Q status: milestone M15 (remote control: RVP's control API and a Unified Remote remote)

Date: 2026-10-25. Result: **done**. M15 is built and checked in WSL, on
the Windows VM with Unified Remote Server, and by the owner from a phone
on the host (the remote works and is accepted; its design is for later).
The final build is in `C:\Users\<user>\rvp-test\rvp-q`.

Precondition met: `goal-p-status.md` records goal P as done. The owner
reported no findings from goal P's build (`rvp-p`) that block daily use,
so nothing came before M15.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Control API | A small HTTP/1.1 JSON service inside RVP on `std::net` (no new server dependency). Off by default; 127.0.0.1 only; the installation's token (32 random bytes from the system, `getrandom`) in `Authorization: Bearer` on every request; requests with a browser's `Origin` header or a `Host` that is not a loopback name refused; limits on headers (8 KiB), bodies (64 KiB), the time a request may take (5 s), connections (8), long polls (30 s) and the window thread's answer (5 s). | `crates/app/src/control` |
| Endpoints | `GET /api/status` (with `?since=V&wait=S`, a long poll for changes), `GET /api/actions`, `POST /api/action/NAME` (any action of `ACTIONS`, run as keys and menus run it), `POST /api/seek`, `/api/volume`, `/api/open`, `GET /api/albums`, `GET /api/albums/A`, `POST /api/albums/A/play/E`. | `control/mod.rs` |
| The window's thread | The server's threads never call mpv nor touch `App`: commands reach the window's thread through `CxxQtThread::queue`, as MPRIS's and Windows' media controls' commands do, and wait for its answer; the status is a snapshot the window's thread publishes as the chrome updates (the version grows with every change but the position). | `shell/src/player/m15.rs` |
| Remote Control page | A settings page of its own (turn on, port, address, the token shown, copied or renewed, the state), in English and Russian. | `Settings.qml`, `qml_ru.ts` |
| Unified Remote remote | `remote/unified-remote/RVP`: `meta.prop` (`RVP.Player`), `layout.xml` in the manner of Unified Remote's media remotes (Player: title, position and volume sliders, mute, jumps, play/pause, fullscreen, previous, stop, next; Tracks: audio and subtitle tracks, subtitles on or off, speed, the normalizer, the volume keys; Albums: the albums and their entries to play), `remote.lua` (Unified Remote's `http` library, the token in a header, the status once a second while the remote shows), `settings.prop` (port and token) and icons that `cargo xtask look` draws from RVP's icon. With an empty token setting the remote reads the token an installed RVP keeps in `%APPDATA%\RVP\remote-token`. | `remote/`, `crates/look/src/outputs.rs` |
| Delivery | The portable ZIP and the folder carry `remote\unified-remote\RVP`; the installer has a component that copies it into `%ProgramData%\Unified Remote\Remotes\Custom\RVP` when Unified Remote Server is installed (the component shows only then, and a folder the user cannot write to is left alone), keeps a `settings.prop` the user edited, notes where it went, and the uninstaller removes those files. | `package.py`, `rvp.nsi` |
| MPC-HC's web interface | Studied and left out: small, but it has no authentication, and its clients send no token (`docs/spikes/m15-mpc-hc.md`). | |

## Tests

- `crates/app/tests/control.rs`: the API on a headless player: the token
  (none, a wrong one), a browser's `Origin`, another host name, the status
  before and after a file opens, the action list, open, pause and play,
  a long poll that waits for the next change and one that ends with its
  wait, seeks to, by and to a fraction, the volume and mute, the albums
  and their entries, playing an entry, wrong paths, methods and bodies,
  and the limits: headers (431), bodies (413), an unfinished request
  (408), one connection too many (503), a window's thread that does not
  answer (503). Publishing and commands stay quick on the window's
  thread while long polls wait.
- `crates/app/tests/remote.rs`: every action `remote.lua` calls exists in
  `ACTIONS`, every request goes to an endpoint of the API, `layout.xml` is
  well formed (a strict reading that refuses broken XML) and each of its
  events names an action of `remote.lua`, every control `remote.lua`
  updates exists, and `settings.prop` carries the default port and no
  token.
- Unit tests: the token (made once, renewed, a damaged file replaced,
  readable by the user only on Linux, compared in full), loopback host
  names, the bodies' checks, answers without `null`, percent-decoding.
- `rvp --selftest=m15` (`shell/qml/SelfTestM15.qml`): the API turned on
  from its setting and called as a remote calls it: the status, an
  action, the volume and a seek reach RVP, a request without the token is
  refused, and turning it off closes the port; with `--remote-serve` it
  first waits for commands from outside. WSL (Xvfb): 11 of 11. The token
  is not in the log.
- `rvp --selftest=m14`: the new page loads and its Russian texts fit.

## Checks on the Windows VM

`tools/windows-vm/m15-all.ps1` with Unified Remote Server 3.13.0.2501
installed on the VM (its installer's SHA-256 checked against winget's
manifest): 24 checks pass.

1. RVP's installer, run silently, finds Unified Remote, copies the
   remote's six files into its custom remotes folder (the same files as
   the bundle's) and notes the folder.
2. Unified Remote's server loads `RVP.Player` and, for a client of its
   web-client protocol, runs the remote's actions (play/pause, volume
   down, mute twice, play/pause) while RVP's M15 self-test serves: 5 of 5
   commands reach RVP, with the token the remote read from
   `%APPDATA%\RVP\remote-token`. The self-test's own checks follow (all
   pass), the token is not in RVP's log, and Unified Remote's log shows
   the remote loaded and unloaded without errors.
3. The uninstaller removes the remote's files and folder, and leaves
   Unified Remote's folder.
4. The headless tests on Windows: `control` and `remote`.

Found on the way: Unified Remote's manager call `/system/remote/run`,
which its old Server Manager remote uses, answers but runs nothing in
3.13; the web-client protocol (`/client/connect`, `/client/request`)
runs actions. A server started over SSH ends with the session.

The installer changed (a components page that shows only with Unified
Remote, the remote's component), so `m8-all.ps1` ran again on the final
package (eab777e), with Unified Remote on the VM: the installed copy's
self-test and `m7-all.ps1` (RVP Default and BlackSpace), the crash logs,
and the uninstaller (every registry key as before, the folder gone) pass.
In the portable ZIP's self-test the Crystality step timed out once (an
audio-chain step that times out on a loaded machine, as in goal P); the
portable part alone then passed.

## The build

`C:\Users\<user>\rvp-test\rvp-q`: the portable ZIP of eab777e, unpacked
(file copies only), with the Whisper models copied from `rvp-p`; its
`rvp.exe` and `rvp.scr` were then replaced by the build with the
fullscreen fix (finding 4).

## The host session (2026-10-25, with the owner's permission)

The owner's PC already has Unified Remote Server (3.13, with a password
for the phone app). With the owner's permission and nothing started on
the host, the remote was copied into
`C:\ProgramData\Unified Remote\Remotes\Custom\RVP`, a new token was made
for `rvp-q` (`rvp-q\state\remote-token`) and the same token and port
48931 were put into the remote's `settings.prop`. The owner starts
`rvp-q`, turns on Remote Control and tries the remote from the phone.

The owner tried it: the remote works, and the owner accepted it, with
questions about its design for later. Unified Remote's log on the host
shows the phone (an Android client) syncing the remote's layout and
loading it twice, and no error of the remote.

## Findings

1. Unified Remote's manager call `/system/remote/run` runs nothing in
   Unified Remote 3.13 (see the VM checks); the check speaks the
   web-client protocol.
2. The server redirected to a file cannot be read while RVP writes it on
   Windows: the VM check waits for the API with the token instead of a
   log line.
3. Refusing a connection over the limit while its request was unread
   reset the connection: the server now reads what the client sends for
   a moment before closing (`control::linger`).
4. The owner, trying `rvp-q`: in fullscreen the control bar "jumps up and
   moves the picture by exactly its height". That was goal P's "Auto
   resize video depending on fullscreen pop-ups", turned on by default
   then after the owner's remark that part of the bar was missing. Goal
   P's host captures show PotPlayer's video keeping its place and size
   under an opaque bar (the bar is a window of its own). Now the shown bar
   lies on a black backdrop (with the bar's opacity and fade), the video
   keeps the screen, and the option is off by default; it still works
   when turned on. Settings saved by goal P's builds keep it on: the
   owner turns it off once (Skins page, "In fullscreen the video makes
   room for the bars while they show"). The functional self-test checks
   both ways (`m7-all.ps1` on the VM: 184 checks pass).

## Differences from PotPlayer

PotPlayer has no control API; Unified Remote's PotPlayer remote sends
window messages to PotPlayer's window. RVP's remote talks to RVP's API
instead, so it shows RVP's state (title, times, volume, tracks, speed,
the normalizer, the albums) and works whatever window is in front.

## Linux items for goal S

- The remote on Linux: Unified Remote Server for Linux keeps custom
  remotes in `/opt/urserver/remotes/custom` (unconfirmed), and the
  remote's token path is Windows' (`%APPDATA%\RVP`); on Linux RVP keeps
  it in `$XDG_STATE_HOME/rvp/remote-token`. `meta.platform` is `windows`
  until then.
- The Linux packages (AppImage, .deb) carry the remote's folder.
- The API itself is the same on Linux and is tested there headless.

## What the owner should try

1. Settings > Remote Control: turn it on; the address shows; Show or
   Copy the token.
2. With Unified Remote Server and its phone app: the installer's
   component copies the remote (or copy `remote\unified-remote\RVP` from
   the portable folder into `%ProgramData%\Unified Remote\Remotes\Custom`);
   an installed RVP needs nothing more, a portable one needs the token in
   the remote's settings (Unified Remote's manager, Remotes, RVP,
   configure). On the phone open the RVP remote: the title and times
   follow RVP, the sliders seek and set the volume, the Albums page plays
   an entry.
3. Renew the token on the page: the remote is refused until it has the
   new one.
4. Fullscreen: move the pointer to the bottom; the control bar shows on
   black and the picture stays where it is (in `rvp-q` first turn off
   "In fullscreen the video makes room for the bars while they show" on
   the Skins page, which goal P's build saved as on).

Unified Remote Server 3.13 has CVE-2023-52252: a web page can make it
run Lua code as the user. That is Unified Remote's; RVP's API refuses
browsers itself.

## CI

GitHub CI passed for eab777e (run 40174569426): the checks on Ubuntu and
Windows (the control API's and the remote's tests among them), cargo
deny, both packages and the artifacts' pruning. This file's commit, with
the fullscreen fix, follows it.

## Open

- The remote's design: the owner has questions for later.
- The host's Unified Remote keeps RVP's remote in its custom folder
  (`C:\ProgramData\Unified Remote\Remotes\Custom\RVP`, copied with
  permission); it goes when the owner asks.
