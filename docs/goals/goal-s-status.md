# Goal S status: milestone M17 (Linux completion)

Date: 2026-11-01. Result: **done**; the Ubuntu boot run followed on
2026-11-14 ("Ubuntu boot run", below).

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

## Ubuntu boot run

2026-11-14, on the owner's Ubuntu 26.10 boot: GNOME Shell 51.0 on
Wayland, kernel 7.3.0-9, PipeWire 1.6.8, an RTX 50-series card (`sm120`) with
NVIDIA 615.71.09, a 32-core processor, two 2560x1440 60 Hz monitors.
The kit ran as the desktop's own user from a terminal in the session
(`tools/linux-kit/kit.sh`, m17all's steps in m17all's order), with its
logs in `~/.cache/rvp-kit/logs-m17`; the report is
`docs/test-runs/2026-11-14-ubuntu-boot.md`. The steps ran at
71683c5; the steps that failed for the kit's or RVP's reasons ran again
at dbde0cd, which fixes them (below), with the packages built again;
m12 and m8 ran once more at c3f58dc, which fixes two things the owner
saw.

The owner's choices for this run:

- The Windows partition stayed unmounted: no `RVP_SKIN` (BlackSpace),
  `RVP_VPY` (PotPlayer's `motioninterpolation.vpy`), PotPlayer's bundled
  skins or the Winamp skins of `rvp-local.tar`. In their place, public
  copies kept in `~/rvp-local` (never committed): uosc 5.13.0 and
  ModernZ v0.3.3 from their releases, whisper.cpp's `ggml-tiny.bin`
  (SHA-1 `bd577a11...`, as whisper.cpp publishes it), the owner's own
  four VLC themes, and Unified Remote Server 3.14.0.2574 unpacked from
  the owner's `.deb` (never installed by the kit).
- NVIDIA's wheels came from NVIDIA's and PyPI's servers (no mirror).
- No power limit: the owner dropped D17's 90 % for this boot ("the limit
  is no longer needed"), so the GPU steps ran at the card's own 300 W
  and m18trt no longer skips above 90 % (dbde0cd). `nvidia-smi` answered
  before and after every step.
- The owner's own Unified Remote Server, which runs on this boot, was
  stopped for m15 only (port 9510) and started again as its login does.
- After the first start of the window steps showed RVP's windows on both
  monitors, the owner switched the displays to mirror mode (one logical
  2560x1440 screen at 59.91 Hz); every result below is from that layout.
  GNOME opens a new window on the pointer's monitor, and on Wayland RVP
  cannot choose one.

### Results

| Step | Result |
|------|--------|
| check, deps, mpv (build 5, FFmpeg 37515cfd87), mvtools v24, whisper (v1.9.5), vapoursynth (R73 with MVTools, RIFE and vs-mlrt, 93 MB) | PASS, 9 minutes in all |
| m2build, m2, m3 (3 kills, 3 resumes within 0.5 s of the position), m4 (96 checks, portable copy) | PASS |
| m6 | PASS: the control panel, colours, the equalizer, a shader, SVP's diagnostic ("SVP 4 Manager is not running"); upright (mean colour distance 1.17, flipped 103.5, mirrored 149.2, turned 82.7) in a window and in fullscreen; the speed carry-over on PipeWire 0 frames dropped after 1.1x -> 1x against 0 in steady playback (goal G counted 14 here); RVP's own script slot with SVP's VapourSynth 23.976 -> 43.2 fps. PotPlayer's script: not run (no `RVP_VPY`) |
| m7 (RVP Default) | PASS, upright in the window and in fullscreen with the bars |
| m8package | PASS: AppImage 147 MB, .deb 116 MB (with RVP's VapourSynth runtime and vs-mlrt, whisper.cpp and the disc libraries) |
| m8 (dbde0cd's packages, then c3f58dc's) | PASS both times, 14 checks: the AppImage's and the installed copy's self-tests, the files, the desktop entry, the registration for `.mkv` with Showtime still the default, MPRIS on GNOME's bus (playing, metadata, Pause, Quit), the screen kept on while video plays (`IsInhibited`: false, true playing, false paused), the crash logs |
| m9 | PASS at dbde0cd: mpv's OSC (17 checks), uosc (17), ModernZ (15), four VLC themes (10 each) through XWayland; no Winamp skins. At 71683c5 the Winamp loader's test failed on the kit's empty `RVP_WINAMP_SKINS` (fixed) |
| m10 | PASS, 96 checks with sound: the second device follows, the recording's WAV, the master volume through `pactl`, Audio Stretching |
| m11 | PASS at dbde0cd, 41 checks. At 71683c5 the second power timer of the self-test neither counted down nor wrote the dry-run file within 12 s (40 of 41); the code shows no cause, and the rerun passed |
| m12 | 26 window and capture checks PASS, and at c3f58dc 27 with the settings tabs' arrows (a run while the owner installed a package in a terminal failed the window placement and the clipboard, which need RVP's window to have the focus on Wayland; the run after it passed); the Vulkan renderer through XWayland on the RTX 50-series card PASS, 11 checks, 0 frames dropped (gpu-next, `nvdec-copy`, 59.91 Hz, fullscreen and back); RVP's shaders: 10 of 11 compile, NVScaler FAILS (open, below) |
| m13 | PASS, 17 checks with whisper.cpp; it heard the espeak-ng clip word for word. The step runs whisper.cpp on the processor (`-ng`); by hand the same build on the clip used the card (`using Vulkan0 backend`, the RTX 50-series card) and heard it the same, 18 s with the first run's shader compilation |
| m14 | PASS on RVP Default (50 checks); the BlackSpace half not run (no `RVP_SKIN`) |
| m15 | PASS at dbde0cd, 12 checks: Unified Remote Server 3.14 runs RVP's remote, its five actions reach RVP, the token stays out of the log. At 71683c5 the server did not load the remote (fixed, below) |
| m16 | PASS, 27 checks (the synthetic DVD's and Blu-ray's menus); at 71683c5 the system check failed on libbluray's soname (fixed, below) |
| m18 | PASS: smoothing 20 checks (MVTools Medium 59.94 fps, seeks that skip the frames before the target, the seek picture, Blend, the log, the calibration window), the GPU budget 16 (its steps and their return, smoothing as its first steps) |
| m18trt | PASS: TensorRT 10.15.1.29 installed for `sm120` from NVIDIA's servers, the engine built (about a minute), RIFE (TensorRT) at 720p 317.6, 1080p 153.8 and 1440p 76.6 frames a second out of the script (60 needed); the card at 163 W and 51 °C right after |
| m2input | PASS at dbde0cd (moved -80,60; resized with the aspect lock; fullscreen 2560x1440 and back; the menu). At 71683c5 ydotool lost its socket (fixed) |
| m2gpu | 4K60 HEVC (`nvdec-copy`): 4 frames dropped in 25 s in fullscreen, 2 in a 1920x1080 window, none by the decoder; single frames, never a burst |
| m6gpu | 2 frames dropped in 25 s in fullscreen with colours, deband, a shader and the equalizer |
| m2svp | PASS: SVP 4.7.0's filter after 1.66 s, RVP exits with 0 |
| m6svp | The first window PASS (SVP's acceptance, below); the second window (`/new`) FAIL: SVP does not attach to it |

**The GPU checks against goal D.** Goal D's open item was about 6
dropped frames per 20 s at 4K60 in a window on the RTX 30-series card. Here, per
20 s: 3.2 in fullscreen, 1.6 in a window, 1.6 with M6's processing on;
the kit's checks ask for none and so still fail. The drops are single
frames spread over the run (seconds 4, 5, 14 and 16 in fullscreen), not
the bursts goal D saw.

**RIFE (TensorRT).** 1080p at 153.8 frames a second, two and a half
times the 60 the calibration asks of an engine; 1440p keeps 60 (76.6).

**SVP's acceptance** (SVP 4.7.0.321, SVP mode on `/tmp/mpvsocket`):
SVP's filter after 2.1 s, 23.976 fps in and 59.5 out on the 60 Hz
screen, audio and video in sync after three seeks (avsync 0, 59.94 fps
after each), still interpolating at the end; RVP's filters go in front of
`@svp` and leave SVP's entry as it was. Two findings:

- SVP 4 for Linux does not attach to a second window on
  `/tmp/mpvSockets/<pid>` either: in 90 s nothing came, and RVP's
  diagnostic said why ("another player holds /tmp/mpvsocket"). So Linux
  is as Windows (goal H, owner decision 2026-10-14): the window SVP
  serves is the one holding the endpoint of SVP's settings.
- With deinterlacing forced on (the self-test's "deinterlace yes") the
  23.976 fps clip reached SVP at 47.6 fps, which SVP took to 119.9 fps on
  the 60 Hz screen: about 300 frames dropped in that part. This is the
  check's forced setting on a progressive file, not a way users play.

### Found and fixed (dbde0cd)

- **libbluray 1.4 and later are `libbluray.so.4`** (Ubuntu 26.04 and
  later, as the Windows build's 1.5.1). RVP opened `.so.2` and then the
  bare name, which only libbluray-dev has: m16's menus worked here only
  because the kit's packages were installed. RVP now takes the libbluray
  already in the process (libmpv's: the package's own or the system's),
  then `.so.4`, `.so.2` and the bare name, so that a package built on
  22.04 (bundling `.so.2`) never loads a second libbluray.
- **m15's remote did not load in Unified Remote 3.14**: since 1ff656f the
  layout shows `mute.png`, which `package.py` copies and the kit did not
  ("could not load layout: File not found", then "could not find remote:
  RVP.Player"). The kit copies every picture, and its log check now
  counts the server's `(E)` lines, which let this pass.
- **m2input lost ydotool's socket**: the export ran in a command
  substitution, so ydotool looked in `$XDG_RUNTIME_DIR` and exited with 2.
- **m9's loaders took an empty variable for a folder** (the kit passes
  `RVP_WINAMP_SKINS` and `RVP_VLC_SKINS` on as it finds them).
- On Ubuntu the crash test (`--crash-test=native`, m8) also makes apport
  write `/var/crash/_opt_rvp_bin_rvp.<uid>.crash` and offer to report it:
  close it without sending.

### The owner's checks

The owner chose to leave CHECKLIST.md's checks by hand (sections 8 to
10) for later, so these are not checked yet: in M18, NVIDIA Optical Flow
in the Vulkan renderer, RIFE on Vulkan, RIFE (TensorRT)'s Download…,
calibration and a 1080p film looked at for the grid, scene cuts and seeks
with smoothing by eye, the GPU budget's real readings beside `nvidia-smi`
and the Debug log's card lines, SVP with RVP's smoothing in a window;
Whisper on the owner's CUDA build; the desktop's tray, screen lock, dock
progress and Recent files, the voices, the trash, the file types, the
phone's remote, screen capture's reason, the OSD's colours, Stop then
Play in GNOME's media controls; a gamepad, a webcam, a DVD drive, HDR.
The kit's steps cover part of them: the Vulkan renderer through XWayland
on the card (m12), RIFE (TensorRT)'s installation and rates (m18trt),
the seeks and their picture and the GPU budget's steps with the
self-test's own readings (m18), MPRIS and the screen kept on (m8), SVP's
acceptance (m6svp), Whisper on Vulkan (by hand, above).

What the owner saw while looking at RVP:

- **The settings window's page tabs** scroll with the wheel but show no
  arrows when they do not fit, which PotPlayer's tab control shows. Qt
  Quick's `TabBar` has none in any style; on Linux, Fusion with Ubuntu's
  font and the Russian titles makes the Video group's ten pages wider
  than the window. Fixed in c3f58dc: two arrows at the row's end while
  the tabs overflow, each moving the row by a tab without choosing a page
  (m12's `settings_tab_arrows`). It touches Windows' settings window too:
  checked there still to come (D19).
- **The open dialog** (the playlist's +) of the installed package is
  small, keeps its size, shows no network drives in its side panel, and
  its Open button seemed to do nothing while a double click opened the
  file. The packages carry only Qt's portal theme
  (`libqxdgdesktopportal.so`), which Qt 6.10 takes only inside Flatpak
  or Snap; on GNOME it looks for its `gtk3` theme first, which the
  packages lack, then its own `gnome` theme, which has no dialogs, so RVP
  shows Qt Quick's own file dialog instead of the system's (GTK's file
  chooser with its "Other Locations"). Fixed in c3f58dc: the packages
  carry Qt's `gtk3` theme too, with GTK 3's libraries the system's (never
  bundled) and a Recommends (`libgtk-3-0 | libgtk-3-0t64`); m8 passes on
  that package.

The packaged `.deb` stays installed at the owner's wish, so `m8removed`
did not run; the VM's runs checked the removal (goal J).

### Open

- **The checks by hand** (above), and the settings tabs' arrows on
  Windows (D19).
- **NVScaler on OpenGL ES.** On Wayland with NVIDIA's EGL, Qt (built for
  desktop OpenGL) gave RVP's renderer an OpenGL ES 3.2 context, and
  libplacebo offers no `rgba32f` there ("Unrecognized/unavailable FORMAT
  name: 'rgba32f'"), which NVScaler's passes use. The other ten shaders
  compile, and the Vulkan renderer is unaffected. Why Qt chose ES here and
  whether RVP should ask for desktop OpenGL on Linux needs a look of its
  own: it would change every Linux user's context.
- **m6svp's second window**: the kit still expects SVP to attach to it;
  whether to make the check expect RVP's diagnostic, as on Windows, is
  the owner's.
- **m2gpu and m6gpu** drop single frames (above): below goal D's count,
  but not none. These ran with the monitors mirrored.
- Not run without the Windows partition: BlackSpace in m14 and by hand,
  PotPlayer's script in m6 and by hand, PotPlayer's 22 bundled skins,
  the Winamp skins.

## Open

- The Ubuntu boot run's open items (above).
- The Linux audit after the run (`docs/audits/linux.md`, two read-only
  audits merged and checked again): 3 high, 16 medium and 27 low
  findings, fixed by goal U on the Ubuntu boot and goal U-W on the
  Windows VM (`docs/PLAN.md`). NVScaler on OpenGL ES is its H2. Goal U
  fixed them on Linux on 2026-11-15 (`docs/goals/goal-u-status.md`),
  NVScaler and m6svp's second window among them; goal U-W is next.
