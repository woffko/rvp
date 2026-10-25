# M15 study: a subset of MPC-HC's web interface

Goal Q asked whether RVP should also speak a small part of MPC-HC's web
interface, which several remote apps know, and to add it only if it is
small. It is small, but it cannot keep RVP's rule that every request
carries a token, so RVP leaves it out. Remotes reach RVP through its own
control API (`crates/app/src/control`), which the Unified Remote remote
(`remote/unified-remote/RVP`) uses.

## What the subset would be

Sources: clsid2/mpc-hc's `src/mpc-hc/WebServer.cpp`,
`WebClientSocket.cpp`, `res/web/variables.html` and `resource.h`
(read on 2026-10-25).

- `GET /command.html?wm_command=ID`: any positive ID is sent to the
  window as a `WM_COMMAND`. The ones remotes use are play 887, pause 888,
  play/pause 889, stop 890, fullscreen 830, volume up, down and mute
  907, 908 and 909, previous and next file 919 and 920, previous and next
  921 and 922, and the seek steps 899 to 904. Negative IDs are commands
  of the web server itself: -1 seeks (`position=HH:MM:SS` or
  `percent=`), -2 sets the volume (`volume=0..100`), and -3 picks a
  playlist entry.
- `GET /variables.html`: 19 `<p id=...>` fields, among them `state` (-1
  nothing open, 0 stopped, 1 paused, 2 playing), `position` and
  `duration` in milliseconds, `volumelevel`, `muted`, `file` and
  `filepath`. Clients read them with a regular expression.

That is about 20 commands and 19 fields, which RVP's `ACTIONS` table and
its control API's status already hold.

## Why RVP leaves it out

- **No authentication.** MPC-HC sets a session cookie but never checks it,
  answers every request with `Access-Control-Allow-Origin: *`, and takes
  commands as plain GET requests. Any web page the user opens could
  therefore control the player, and open files through it. No client of
  the interface sends a token: Yatse, Home Assistant's former `mpchc`
  integration, and the browser remotes.
- **A token would break the clients.** A variant that asks for a token (a
  query parameter or a header) would refuse every existing MPC-HC client,
  which removes the reason to speak the protocol at all.
- **127.0.0.1 is not enough.** A port bound to 127.0.0.1 is still within
  reach of the user's own browser: a web page can send it GET requests.
  RVP's own API refuses requests with a browser's `Origin` header, checks
  `Host`, and asks for the token on every request.
- **Unified Remote does not need it.** Unified Remote's own MPC-HC and
  PotPlayer remotes send window messages, not web requests, and RVP's
  remote calls RVP's API with the token in an `Authorization` header (as
  Unified Remote's Kodi remote does).

So the interface would be the one unauthenticated way into RVP. Decision:
not added. A client that wants to control RVP uses the control API.
