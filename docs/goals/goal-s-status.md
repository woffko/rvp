# Goal S status: milestone M17 (Linux completion)

Date: 2026-11-01. Result: **done** (the Ubuntu boot run follows).

Precondition met: `goal-k-status.md` records goal K as done.

Goal S follows D19 (owner, 2026-11-01): every change is built and checked
on both Windows and Linux.

## The Linux items

The items come from the goals' status files ("Linux items for goal S", L
to R, T and K) and from M17 in PLAN.md section 6.

| Item | From | State |
|------|------|-------|
| The kit's libmpv at the Windows build's commits (mpv b2c255c13e, FFmpeg 37515cfd87, libplacebo a83aaeb4): Vulkan, Lua with mpv's scripts, DVD, Blu-ray and CD, GnuTLS, libmysofa, libbs2b, rubberband, dav1d, zimg, soxr, libxml2, AviSynth's and VapourSynth's demuxers, fruc_vulkan | M, N, O, T, K | built on the Ubuntu VM; the step checks the library's parts and that every symbol resolves |
| No non-free FFmpeg in the Linux packages | found in goal S | fixed (see "Findings") |
| The system's master volume (PulseAudio's `pactl`, which PipeWire answers) | L | done: `m10` on the VM (`master_volume_answers`) |
| Recording RVP's own stream (`parec` on RVP's playback stream) | L | done: `m10` on the VM (the WAV file's length) |
| The trash (freedesktop.org Trash 1.0) | M | done: unit tests, `--selftest=m11` under Xvfb |
| Power actions through logind | M | done (dry-run tests only) |
| Capture devices (`v4l2` from sysfs) | M | done, unit test |
| The screen as a source: `x11grab` in an X11 session; Wayland's reason | M | done |
| DVD, Blu-ray and CD in the kit's libmpv; M16's menus | M, R | libmpv built; `--selftest=m11` plays the DVD folder under Xvfb |
| The Vulkan renderer (mpv's gpu-next in an X11 child window; XWayland on Wayland) for NVIDIA Optical Flow | N, T | done: `--selftest=vulkan` on lavapipe under Xvfb and in the VM's GNOME session through XWayland (`m12`, fullscreen too); Optical Flow itself needs an NVIDIA card (the boot run) |
| File types: the user's `rvp.desktop` from the Association page, AutoPlay's `x-content` types | P | done, unit test |
| Recent files (`recently-used.xbel`) | P | done, unit tests |
| Text to speech (speech-dispatcher's `spd-say`) | P | done: `m14` on the VM lists speech-dispatcher's voices; hearing it is the boot run's |
| The Xbox controller (the kernel's joystick API) | P | done, needs a controller |
| Pausing when the screen locks, the taskbar's progress (Unity launcher API) | P | done, tests on a private bus |
| zbus 6 for MPRIS and idle inhibit | M | done (zbus 6.2.0; the three RUSTSEC ignores are gone) |
| M9's skin windows on Wayland: XWayland for skins with windows of their own | K | done: `m9` on the VM (four classic skins and the VLC themes in GNOME) |
| The Unified Remote remote on Linux (token path, platform, the Remote Control page's Add button, the packages' copy) | Q | done: `m15` on the VM with Unified Remote Server for Linux run from its unpacked `.deb` |
| whisper.cpp's CPU and Vulkan build for Linux (`tools/whisper/build-linux.sh`, the kit's `whisper` step) | O | built on the VM (v1.9.5) |
| RVP's VapourSynth runtime in the Linux packages (`tools/vapoursynth/build-linux.sh`, the kit's `vapoursynth` step) | T | built on the VM: R73 with MVTools and RIFE, checked from a moved copy, 70 MB |
| The kit's steps m9 to m16, m18, whisper, vapoursynth and m17all; CHECKLIST.md | K and all | done: `m17all` on the VM (see "Results"); CHECKLIST.md section 10 |
| The Linux reasons in `docs/coverage/` (`linux.md`) and a test that no code waits for goal S | M17 | done |
| The packages carry the new parts (AppImage, .deb, PKGBUILD) | M17 | done: `m8` on the VM (installed, run and removed); CI's check on Ubuntu 26.10 |

## Findings

- **A non-free FFmpeg in the Linux packages.** mpv-build enables OpenSSL
  with `--enable-nonfree` when it finds no GnuTLS, and the kit's package
  list had OpenSSL's headers only. The libmpv of every earlier Linux
  package was built so. Nothing was published (CI's artifacts only). The
  kit now names GnuTLS, and its `mpv` step fails a non-free library.
- **An undefined symbol in libmpv.** With Vulkan, libplacebo took some of
  glslang's libraries from Ubuntu's glslang 11.8 and some from the kit's
  16.6: libmpv linked (a shared library may leave symbols undefined) and
  failed at its first shader (`GetDefaultResources`). libplacebo now uses
  the kit's shaderc, one static library, and the step fails a library
  with an undefined symbol (`ldd -r`).
- **Qt's input transparency on X11 takes the child windows' map
  requests.** `Qt::WindowTransparentForInput` also selects
  SubstructureRedirect on the window, and Qt never grants the requests:
  mpv's window inside RVP's video window stayed unmapped and mpv waited
  for it, the window's thread waiting on mpv. RVP clears that one bit of
  the window's event mask.
- **A relocatable Python needs its home without PYTHONHOME.** VSScript
  starts the embedded Python; `PYTHONHOME` would also reach the programs
  RVP starts (yt-dlp is a Python program). RVP's VSScript is patched to
  name the Python beside it (`Py_SetPythonHome`, which VSScript's limited
  API has up to Python 3.12).
- **libpulse takes another user's server from X11.** With `DISPLAY` set,
  libpulse reads the sound server from XWayland's root window: the kit,
  running as another user in the desktop's session, reached the desktop
  user's server and was refused. The session attach names the kit user's
  own server (`PULSE_SERVER`).
- **An inhibit request held MPRIS's state back.** The desktop thread
  waited for each inhibit call in its loop. On a session bus whose portal
  cannot start (the kit user's, beside another user's desktop), the
  portal's `Inhibit` stayed unanswered until the bus's start timeout, and
  meanwhile MPRIS kept showing "Playing" after a Pause. The request now
  runs beside the loop's other messages; a test serves a portal that never
  answers.
- **GNOME maximizes a first window that covers most of the screen** (over
  80 % of the free area: RVP's 1280x720 on the VM's 1280x800 screen). Two
  things then undid a Window Size command: the fit to the first video,
  which waited for the window to be restored and came after the command,
  and the compositor's own restore, which on Wayland arrives after the
  program's next size. A video that opens in a maximized window now leaves
  it so, and on Wayland a command that restores the window waits for the
  size to settle.
- **Wayland gives a program no free area of the screen**: Qt's available
  area is the whole screen, and GNOME keeps the window off its top bar.
  M12's "maximum" check allows for the bar there.
- **A bundled nettle broke the system's GnuTLS on Ubuntu 26.10.** The
  packages take GnuTLS from the system but bundled libarchive's nettle
  from 22.04, which 26.10's GnuTLS found first through RVP's library
  folder (`nettle_sha3_256_shake_output` missing): RVP stopped at start.
  GnuTLS's own libraries now come from the system too, and CI's package
  check prints what RVP printed last when a self-test gives no result.
- **FFmpeg's master wants lcms2 2.13**, newer than Ubuntu 22.04's 2.12:
  FFmpeg's lcms2 is left out; mpv's ICC support keeps it.
- **The shared Ubuntu VM's network to GitHub stalls** (clones at under
  1 KB/s, a fetch hung for half an hour). The kit's git gives up after a
  minute below 1 KB/s, and the builds clone from a source cache when
  `RVP_KIT_SOURCES` names one (`tools/linux-kit/sources.sh mirror DIR`
  makes it on a machine with a good network).

## Results

### Ubuntu VM (GNOME Wayland, no GPU)

`kit.sh m17all` at 05803b3 as the kit's user in the owner's GNOME session
(`attach-session.sh`), with local sample skins, scripts, the tiny whisper
model and Unified Remote Server for Linux unpacked (none committed):

| Step | Result |
|------|--------|
| check, deps, mpv (build 5), mvtools, whisper, vapoursynth | PASS |
| m2build, m2, m3 (3 kills, 3 resumes), m4 (96 checks, portable), m6, m7 | PASS |
| m8package, m8 (AppImage; the .deb installed, run with MPRIS's Pause, and removed) | PASS, again with 328108e's packages; the screen-keeping check is SKIPPED: the kit user's bus has no GNOME session manager |
| m9 (OSC, uosc, ModernZ, four classic skins, the VLC themes) | PASS |
| m10 (96 checks, the recording's WAV) | PASS |
| m11 (41), m13 (17, whisper on), m14 (50 and, with the BlackSpace test copy, 51), m15 (12, Unified Remote on), m16 (27), m18 (smoothing 10, GPU budget 16) | PASS |
| m12 (26 window and capture checks, 14 shaders, 11 Vulkan renderer checks, the headless tests) | PASS in the step's rerun; in the long run one headless test (`video_filters_wait_at_the_end_of_a_file`) timed out waiting for the file's last second |
| m2input | SKIPPED: needs `ydotoold` run as root (the boot run's) |
| m2gpu, m6gpu, m2svp, m6svp | SKIPPED: no GPU, no SVP (the boot run's) |

### Windows test VM (session 0)

The build of 05803b3 (the code of 328108e) with the test executables built
by `cargo xwin test --no-run`, after asking the VM's other users each
time; every script passes:

- m7-all (192 checks), m11-all (47), m13-all (19), m16-all (33), m18-all
  (27).
- m9-all (206 checks: OSC, uosc, ModernZ, the classic skins and VLC
  themes), m10-all (101), m14-all (108 on RVP Default and the BlackSpace
  test copy), m15-all (24: the installer's remote is the bundle's, Unified
  Remote runs it).
- m12-all (53): pass; `window_grows` failed once (the Desktop Size box was
  computed before the video's shape was known, so the window stayed at
  its smallest) and passed in the rerun.
- Once a uosc run's RVP stayed alive after its last check, its one thread
  left waiting in the kernel, until the script that read its output
  ended; the rerun passed.

### CI

`cargo xtask ci` before each commit. GitHub's run for 328108e: the checks
on Linux and Windows, cargo-deny and both package jobs; the Linux
packages pass their self-tests on Ubuntu 26.10 in a container (the
functional self-test of the .deb and the AppImage, M11's and M16's).
Windows' media controls check (`m8-smtc.ps1` on the hosted runner) once
waited 30 s for the Next button with clip A playing and passed in the
job's rerun.

## The Windows build

`C:\Users\<user>\rvp-test\rvp-s` on the host: the portable package of 05803b3
(`rvp.portable`, RVP's VapourSynth runtime, whisper.cpp, the disc
libraries), copied there; nothing was started.

## Only the Ubuntu boot can check

The GPU (gpu-next on NVIDIA, HDR, the shaders' speed, the GPU budget's
readings), NVIDIA Optical Flow and RIFE through XWayland's Vulkan
renderer, whisper.cpp on Vulkan or the owner's CUDA build, SVP 4 for
Linux with RVP's smoothing and SVP's wait, `m2input` with `ydotoold`, the
screen kept on (GNOME's own session bus), and the desktop's parts that
need a person: the tray (with the AppIndicator extension), the screen
lock, the dock's progress, the voice heard, a gamepad, a DVD drive, a
webcam. The run is `/goal` "Ubuntu boot run for M6 to M18" in PLAN.md
section 10, following CHECKLIST.md sections 1 to 10.

## For the owner to try

- On Windows: `C:\Users\<user>\rvp-test\rvp-s\rvp.exe`. For Windows users goal
  S changed little: a video opened in a maximized window no longer
  resizes the window when it is restored, and Preferences opens on the
  page shown last (72163ce).
- On the Ubuntu boot: the AppImage or the .deb from CI's newest
  `rvp-linux` artifact, then the checklist's section 10 by hand.

## Open

- The Ubuntu boot run (above).
