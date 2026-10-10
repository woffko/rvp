# Goal D status: the owner's Ubuntu 26.10 boot (S2, S1, S3 with the GPU)

Date: 2026-10-10. Result: done. **Linux gate: passed, with one open item**
(owner decision; verdict below).

The session ran on the Ubuntu boot itself, in the owner's GNOME session.
Every step that opened a window or moved the pointer was announced and ran
only after the owner's explicit yes; no keyboard input was injected. All
test clips are synthetic (owner decision): `tools/test-clips/make-clips.sh`
and clips generated with the same parameters. The NAS was not used.
PotPlayer's script and skin were read from the Windows partition at run
time; SVP's installation was not changed.

## Summary

| Item | Result | Details |
|------|--------|---------|
| S2: VapourSynth inside libmpv with SVP's runtime | **PASS.** SVP 4.7.0.321's VapourSynth R73 and Python 3.12.11 (`~/SVP4`). As on Windows, the unchanged script fails on `vs.get_core()`; through the wrapper it interpolates 23.976 → 60.000 fps in real time (ratio 1.000), with mvtools v24 built by the kit | `docs/spikes/s2-vapoursynth.md` |
| S1: SVP attaches | **PASS.** SVP 4.7.0.321 attached to the libmpv spike (process not named `mpv`) after 1.2 s through `/tmp/mpvsocket`, inserted its `@svp` filter and reached 59.94 fps from 23.976. The spike then crashed on exit with SVP's filter still active | `docs/spikes/s1-svp-attach.md` |
| S3: shell on the GPU | **PASS.** Frameless window, BlackSpace painted from the owner's skin by the Rust item, mpv video item fed through the wakeup callback on the Qt thread, NVDEC (`nvdec-copy`), 1167 frames rendered where 1080 were required | `docs/spikes/s3-qt-shell.md` |
| S3: 4K60 without dropped frames | **Not met; open item accepted by the owner.** 93 drops per 20 s with the kit's first libmpv, which played audio through the PulseAudio API; 6 after building libmpv with PipeWire. mpv's own output on the same desktop drops 3. Not caused by chrome repaints, the render loop, Xwayland, the decoder, display sync, GPU clocks, CoolerControl or ydotool (27 diagnostic runs) | `docs/spikes/s3-qt-shell.md` |
| S3: real input | **PASS.** Real pointer drags move (−80, 60) and resize (−100, −80) the frameless window; a right click on the 440x77 window opens a 200x401 menu window | `docs/spikes/s3-qt-shell.md` |
| Linux kit | Ran on the Ubuntu boot; fixed with what this run found (below) | `docs/test-runs/2026-10-10-ubuntu-boot.md`, `tools/linux-kit/` |

## Gate verdict for Linux

PLAN.md: "confirm D1-D5 or switch to the documented fallback before M2".
Owner decision 2026-10-10: the gate passes, and the remaining dropped frames
stay open and are rechecked in M2's GPU check on this machine.

| Decision | Verdict | Evidence |
|----------|---------|----------|
| D1 libmpv via `libmpv2` | Confirmed, with a rule | S1 (SVP attaches through `/tmp/mpvsocket`), S2 (SVP's VapourSynth through `VSSCRIPT_PATH` and the `get_core` wrapper), S3 (render API). Rule: on Linux, libmpv is built with the PipeWire audio output |
| D2 own skin engine for `.dsf` | Confirmed | The spike paints BlackSpace from the owner's `.dsf` on Linux (143 operations) |
| D3 Qt 6 via cxx-qt, QML | Confirmed | Frameless move and resize and a menu beyond the window with real input on GNOME 51 Wayland with the NVIDIA GPU |
| D4 OpenGL render API first | Confirmed, with an open item | 4K60 HEVC keeps up (1167 frames rendered); drops fell from 93 to 6 per 20 s with PipeWire. The remaining drops also hit mpv's own output (3), so no other output path does better on this desktop, and `wid` does not exist on Wayland |
| D5 display list painted with `QPainter` in a Rust `QQuickPaintedItem` | Confirmed | Whole 1920x1080 skin: 68 ms on the first paint (atlas decoding), about 1 ms after that; time label 2.5 ms on average |

Rules that M2 inherits, in addition to goal C's four:

5. Linux builds of libmpv include the PipeWire audio output (`ao=pipewire`),
   and diagnostics show `current-ao`. Through the PulseAudio API on PipeWire,
   60 fps video drops 4-5 frames per second.
6. Before `mpv_terminate_destroy`, stop playback and let the filter chain
   (SVP's VapourSynth filter included) shut down; test exit with SVP active
   on Linux.
7. Keep mpv's default `video-timing-offset` with the render API, and do not
   drive display-synced modes from Qt's swap times on Wayland: they are not
   presentation times.
8. Look for SVP's VapourSynth in `~/SVP4/mpv` (`libvapoursynth-script.so.0`)
   as well as `/opt/svp`; PotPlayer's script needs mvtools, which neither SVP
   nor Ubuntu ships (M6).

## Open item: remaining dropped frames

About 6 dropped frames per 20 s at 4K60 in a 1920x1080 window on GNOME 51
Wayland with NVIDIA 595.91.07, in bursts of one or two every 5-7 seconds;
mpv's own `gpu-next` output drops 3. Ruled out: chrome repaints (none: 6),
the render loop (threaded: 5), display sync (`display-resample`: 6), the
content rate (59.94 fps: 7), the decoder, GPU clocks, CoolerControl's daemon
and ydotool's virtual device. M2's GPU check on this machine should try
fullscreen (direct scanout), longer runs, film-rate content with SVP and
presentation feedback.

## Kit and spike changes from this run

- Packages: `libpipewire-0.3-dev` (PipeWire audio output),
  `libffmpeg-nvenc-dev` (NVDEC) and `libfftw3-dev` (mvtools).
- New `mvtools` step: mvtools v24 against the R73 API 3 headers; S2 loads it
  by default.
- S2: finds SVP's runtime in `~/SVP4/mpv`, passes or fails on the wrapper run
  (the unchanged script is expected to fail), and keeps bytecode out of
  SVP's installation.
- S1: plays its own 1080p 23.976 fps clip (`RVP_S1_VIDEO`; it used the 4K60
  S3 clip before, which SVP has no reason to interpolate), passes SVP's
  `VSSCRIPT_PATH` to the spike and records its exit status.
- Input check: talks to `/tmp/.ydotool_socket` (under `sudo` the daemon and
  the client otherwise look in different places), switches the pointer to
  the flat profile and restores it; the checklist covers Ubuntu's ydotool
  package.
- S3 spike: `RVP_REPORT_SWAP`, the per-second drop timeline, and no unpressed
  `startSystemMove()` on X11.

## Versions

| Component | Version |
|-----------|---------|
| OS | Ubuntu 26.10 (development branch), kernel 7.2.0-5-generic |
| Desktop | GNOME Shell 51.beta, Mutter 51~beta-1ubuntu2, Xwayland 24.1.13; Wayland session |
| Hardware | 32-core processor (64 threads), RTX 30-series card, one 2560x1440 monitor at 59.95 Hz, 100% scaling |
| NVIDIA driver | 595.91.07, open kernel modules |
| Audio | PipeWire 1.6.8, WirePlumber 0.5.15 |
| Rust | 1.98.1 (`rust-toolchain.toml`) |
| Qt | 6.10.3 (aqtinstall), as on Windows |
| libmpv (kit) | mpv `b2c255c13e` (v0.41.0-1115), FFmpeg n9.0.2, libplacebo v7.360.1, libass 0.17.5-11-gf61db56, mpv-build `9443097`; rebuilt with PipeWire 1.6.8 and nv-codec-headers 12.1.14 |
| SVP | SVP 4 for Linux 4.7.0.321 in `~/SVP4`: VapourSynth R73, Python 3.12.11 |
| mvtools | v24 (`d9270bec`) |
| ydotool | 1.0.4-3 (Ubuntu package) |

## Owner-side effects and cleanup

- Packages installed by the owner: the kit's list plus `ydotool`,
  `libfftw3-dev`, `libpipewire-0.3-dev` and `libffmpeg-nvenc-dev`. Ubuntu's
  `ydotool.service` was disabled for all users (`systemctl --global
  disable`); nobody was added to the `input` group.
- GitHub CLI 2.102.0 was installed in `~/.local/bin` for the clone (token in
  the GNOME keyring) and set as git's credential helper for github.com.
- The pointer acceleration was switched to Flat for the input check and
  restored to the default profile.
- The owner stopped CoolerControl's daemon for one run and started it again.
- The system power profile changed from balanced to performance at 17:39,
  while the owner was changing power settings; drops through the PulseAudio
  API were 74-93 before and 80-91 after. Switch it back if it is not wanted.
- Clips, logs, grabs (with BlackSpace) and the diagnostic runs stay in
  `~/.cache/rvp-kit` on the Ubuntu boot and are not committed.
- The Windows partition was mounted read-write by the desktop at
  `/run/media/<user>/01D5CDB0C6667800`; the kit only read from it.

## Open questions for the owner

1. **License rule (open since goal A).** Confirm that the `rust` skill's
   no-GPL/LGPL rule is waived for libmpv and Qt in this project before M2.
   Note also that mpv-build configures FFmpeg with `--enable-nonfree` and
   OpenSSL, which makes the kit's libmpv non-redistributable; it is only a
   test build, but M7's packages need another configuration.
2. **`/mnt/media`.** The Media share is mounted at `/mnt/nas/Media` on
   this system, while PLAN.md maps `V:\` to `/mnt/media` (goal F). Add a
   mount or link at `/mnt/media`, or make the mapping configurable?
