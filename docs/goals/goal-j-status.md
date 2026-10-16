# Goal J status: milestone M8 (platform integration and packaging)

Date: 2026-10-16. Result: **done**. Tests and CI are green; the packages
install, run and uninstall cleanly on the Windows VM (installer and
portable ZIP) and the Ubuntu VM (AppImage and .deb). Precondition met:
`goal-i-status.md` records goal I as done.

**M8's acceptance on Linux, like M6's, still needs the owner's Ubuntu boot
run** (`tools/linux-kit/kit.sh m8all`, checklist sections 8 and 9; the
`/goal` text is in PLAN.md section 10). Windows' media controls still need
one short run on the host, with the owner's permission (below).

## M8 summary

| Deliverable | Result | Where |
|-------------|--------|-------|
| Registered types | 77 video and audio types: those PotPlayer is registered for on the owner's host (its `PotPlayerMini64.*` ProgIDs, read with `reg.exe`, read-only), without subtitles, playlists, disc files, Flash, PotPlayer's bookmarks and `.dsf` (PotPlayer's skins share it with DSD audio). Each has its MIME types from shared-mime-info for Linux. The installer, the desktop entry and MPRIS read the one list | `packaging/associations.tsv`, `crates/app/src/associations.rs` |
| MPRIS (Linux) | `org.mpris.MediaPlayer2` (Raise, Quit, identity, desktop entry `rvp`, URI schemes, MIME types) and its `Player` (Play, Pause, PlayPause, Stop, Next, Previous, Seek, SetPosition, OpenUri, status, metadata with title, URL and length, position, volume, rate 0.25-4 with 0 as Pause, the `Can*` flags, `Seeked`). A second RVP takes `org.mpris.MediaPlayer2.rvp.instance<pid>`. Bus calls run on a thread of their own; commands reach the window's thread as `MediaCommand`s | `crates/platform`, `crates/app/src/media.rs`, `shell/src/desktop.rs` |
| Idle inhibit (Linux) | While video plays (not audio alone, not paused): GNOME's session manager (`Inhibit`, app ID `rvp`, idle flag), else `org.freedesktop.ScreenSaver`, else the XDG portal; released on pause, stop and exit, and by the services when RVP's connection ends | `crates/platform/src/linux.rs` |
| Windows' media controls | System Media Transport Controls of the main window (`GetForWindow`, the one FFI point): title, status, Play, Pause, Stop, Next and Previous, which work like the window's own commands | `shell/src/media_controls.rs` |
| Crash logs | Panics (a hook), fatal signals on Unix and unhandled exceptions on Windows (with a minidump) write `crash-<UTC time>[-native].log` with the version and, for panics, the thread, place and backtrace, to RVP's state folder. `rvp --crash-test=panic` or `=native` crashes on purpose | `crates/app/src/crash.rs`, `shell/src/crash.rs` |
| Version | The workspace version and the commit: `0.0.1+git.<commit>` for packages, `0.0.1 (<commit>)` in the About box and crash logs (`crates/app/build.rs`; `RVP_COMMIT` where there is no `.git`) | `tools/packaging/package.py version` |
| Windows packages | The folder (`rvp.exe`, Qt, libmpv, the C++ runtime), a portable ZIP with `rvp.portable`, and a per-user NSIS installer (no administrator rights, `%LOCALAPPDATA%\Programs\RVP`): Start menu entry, Settings' app list, `Applications\rvp.exe`, a ProgID per type, `OpenWithProgids` and Default apps (`Capabilities`, `RegisteredApplications`). No type's default changes. The uninstaller removes the files it installed and, of shared keys, only those the installer created (`Software\RVP\Created`) | `packaging/windows/rvp.nsi`, `package.py windows` |
| Linux packages | An AppImage (the type2 runtime release 20251108, checked by SHA-256) and a .deb for Ubuntu 22.04 and later: `/opt/rvp`, `/usr/bin/rvp`, the desktop entry and hicolor icons, `Depends` with the t64 names as alternatives (checked on 22.04; 26.10 waits for the boot run). Both carry Qt, libmpv and what they reach that a desktop may lack, found through the run path; the system's libraries for the rest. Built on Ubuntu 22.04 with the kit's Qt and libmpv | `packaging/linux/rvp.desktop`, `package.py linux`, kit `m8package` |
| AUR | A PKGBUILD that builds from the repository; not tested (no Arch container runs here without changing the system) and not published | `packaging/aur/PKGBUILD` |
| CI | `package-linux` (Ubuntu 22.04, the kit with Qt and libmpv cached) and `package-windows` (windows-2025, NSIS) keep the packages as artifacts for 14 days; nothing is published | `.github/workflows/ci.yml` |
| Kit and checks | Kit steps `m8package`, `m8`, `m8removed` and `m8all` (`m7all` plus M8), checklist section 9; Windows VM `tools/windows-vm/m8-all.ps1`; host `tools/windows-vm/m8-smtc.ps1` | `tools/` |

## Tests

- `crates/app`: unit tests of the association list (every line reads,
  unique extensions, MIME type syntax, media only), of file paths and
  URLs, of crash log names and the version; `tests/media.rs` (the state
  the desktop sees while a file plays, and its commands steering
  playback); `tests/crash.rs` (a panic writes its log; a test binary of
  its own, since the hook is process-wide).
- `crates/platform/tests/linux.rs`: MPRIS on a private `dbus-daemon`
  (properties, `PropertiesChanged`, `Seeked`, the methods, the second
  instance's name) and the inhibit order with fake session manager,
  screen saver and portal services.

## Checks on the VMs

| Machine | Result |
|---------|--------|
| Windows VM | `m8-all.ps1`, 20 checks, 0 failures: install (files, Start menu, `OpenWithProgids`, the open command, Default apps, capabilities, uninstall entry), every type's default unchanged, the installed copy's self-test, crash logs (a panic; a native crash `0xc0000005` with a 61 KB minidump), uninstall (folder and Start menu gone, every key the installer touched as before, keys of other programs kept), the portable ZIP's self-test with its settings beside `rvp.exe` |
| Ubuntu VM, kit as the kit's user | `m8package PASS` (AppImage 57 MB, .deb 43 MB; 78 libraries bundled, 50 from the system); `m8 PASS`, 15 checks on a private bus (idle inhibit skipped there); `m8removed PASS` |
| Ubuntu VM, kit as the desktop's user | `m8` on GNOME's own session bus, 14 checks: the AppImage's and the installed copy's self-tests, files, desktop entry, registration for `video/x-matroska` with Totem still the default, MPRIS (playing, metadata, Pause, Quit), the screen kept on while playing and released when paused (`IsInhibited 8`), crash logs; then `m8removed PASS` |

## Findings

- GNOME refused the portal's idle inhibit: for a program outside a
  sandbox the portal passes an empty app ID to GNOME's session manager,
  which answers "Application ID not specified", while the portal still
  reports success (Ubuntu 22.04: xdg-desktop-portal 1.14.4,
  -gnome 42.1, -gtk 1.14.0). RVP now asks the session manager itself
  first, as GTK does outside a sandbox; the portal comes last, for
  sandboxes. The session manager drops the inhibit of a client that
  leaves the bus (checked by ending a client without releasing).
  This changes the goal's text ("the XDG desktop portal first,
  org.freedesktop.ScreenSaver as the fallback"); the requirement, the
  screen kept on while video plays, is met only this way.
- An MPRIS Quit seemed to leave RVP running: `pgrep -f` matched the shell
  running the check, whose command line holds RVP's. Found by process
  name, RVP ends 0.5 s after Quit.
- zbus's connection builder queues a request for a name another process
  holds and reports success; RVP asks for its name itself with
  `DoNotQueue`.
- The first uninstaller removed two empty type keys (`.divx`, `.amr`) that
  existed on the VM before; it now removes only keys the installer
  created. Both keys were made again on the VM.
- Windows' session 0 (the VM over SSH) has no media session manager
  ("The specified service does not exist as an installed service"), so
  the media controls need a desktop session. `m8-smtc.ps1` stops before
  starting RVP there. Its way of closing RVP (WM_CLOSE to the windows of
  class `Qt6103QWindowOwnDCIcon`) ends RVP with exit code 0 on the VM.
- CI: makensis reads the paths it gets from the script's folder, so
  `package.py` passes absolute ones; `upload-artifact` takes no path with
  `..`, so the Linux job copies its packages into the workspace.
- zip 9.0.0 to 9.0.2 were yanked: 9.0.3.

## Owner decisions in this goal

- 2026-10-16: the Windows installer is NSIS, built in WSL, per user,
  without administrator rights.
- 2026-10-16: RVP registers for the types PotPlayer is registered for on
  the host, as a handler only; it never makes itself the default.

## Host-side effects and cleanup

- Windows host: `reg.exe` queries of PotPlayer's ProgIDs, read-only.
  Nothing ran on the desktop, nothing was installed or changed.
- Windows VM: `C:\rvp-test\m8` (packages, check output) and
  `C:\rvp-test\m8-smtc` (the portable build and the script). Its registry
  reads as before the checks.
- Ubuntu VM: `/opt/rvp-kit/logs-j1` to `logs-j4`; the .deb was installed
  and removed several times and nothing of it is left.

## Versions

| Component | Version |
|-----------|---------|
| zbus | 5.19.0 |
| windows (crate) | 0.62.2 |
| libc (crate) | 0.2.190 |
| zip (crate) | 9.0.3 |
| NSIS | 3.09 (WSL: Ubuntu's packages, unpacked into `~/rvp-deps/nsis`; CI: the runner's) |
| AppImage runtime | type2-runtime 20251108 |

## Open items

- The Ubuntu boot run for M6, M7 and M8 (PLAN.md section 10). The owner
  installs and removes the .deb when `m8` asks (`sudo apt install`, then
  `sudo apt remove rvp` and `kit.sh m8removed report`), and tries the
  checklist's section 9 by hand.
- Windows' media controls on the host: `m8-smtc.ps1` with the portable
  build in a folder of `C:\Users\<user>\rvp-test`, about one minute, one RVP
  window, no sound; only with the owner's permission.
- M3's one-week trial.
