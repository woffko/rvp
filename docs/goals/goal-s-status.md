# Goal S status: milestone M17 (Linux completion)

Date: 2026-11-01. Result: **in progress**.

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
| The system's master volume (PulseAudio's `pactl`, which PipeWire answers) | L | done, VM check pending |
| Recording RVP's own stream (`parec` on RVP's playback stream) | L | done, VM check pending |
| The trash (freedesktop.org Trash 1.0) | M | done: unit tests, `--selftest=m11` under Xvfb |
| Power actions through logind | M | done (dry-run tests only) |
| Capture devices (`v4l2` from sysfs) | M | done, unit test |
| The screen as a source: `x11grab` in an X11 session; Wayland's reason | M | done |
| DVD, Blu-ray and CD in the kit's libmpv; M16's menus | M, R | libmpv built; `--selftest=m11` plays the DVD folder under Xvfb |
| The Vulkan renderer (mpv's gpu-next in an X11 child window; XWayland on Wayland) for NVIDIA Optical Flow | N, T | done: `--selftest=vulkan` on lavapipe under Xvfb (fullscreen needs a window manager: VM check pending) |
| File types: the user's `rvp.desktop` from the Association page, AutoPlay's `x-content` types | P | done, unit test |
| Recent files (`recently-used.xbel`) | P | done, unit tests |
| Text to speech (speech-dispatcher's `spd-say`) | P | done, VM check pending |
| The Xbox controller (the kernel's joystick API) | P | done, needs a controller |
| Pausing when the screen locks, the taskbar's progress (Unity launcher API) | P | done, tests on a private bus |
| zbus 6 for MPRIS and idle inhibit | M | done (zbus 6.2.0; the three RUSTSEC ignores are gone) |
| M9's skin windows on Wayland: XWayland for skins with windows of their own | K | done, VM check pending |
| The Unified Remote remote on Linux (token path, platform, the Remote Control page's Add button, the packages' copy) | Q | done, VM check pending |
| whisper.cpp's CPU and Vulkan build for Linux (`tools/whisper/build-linux.sh`, the kit's `whisper` step) | O | building on the VM |
| RVP's VapourSynth runtime in the Linux packages (`tools/vapoursynth/build-linux.sh`, the kit's `vapoursynth` step) | T | building on the VM |
| The kit's steps m9 to m16, m18, whisper, vapoursynth and m17all; CHECKLIST.md | K and all | written; VM run pending |
| The Linux reasons in `docs/coverage/` (`linux.md`) and a test that no code waits for goal S | M17 | done |
| The packages carry the new parts (AppImage, .deb, PKGBUILD) | M17 | written; VM check pending |

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
- **FFmpeg's master wants lcms2 2.13**, newer than Ubuntu 22.04's 2.12:
  FFmpeg's lcms2 is left out; mpv's ICC support keeps it.
- **The shared Ubuntu VM's network to GitHub stalls** (clones at under
  1 KB/s, a fetch hung for half an hour). The kit's git gives up after a
  minute below 1 KB/s, and the builds clone from a source cache when
  `RVP_KIT_SOURCES` names one (`tools/linux-kit/sources.sh mirror DIR`
  makes it on a machine with a good network).

## Open

- The VM runs (the kit's steps in the GNOME session), the Windows VM's
  checks, the Windows build on the host, the boot run's /goal text.
