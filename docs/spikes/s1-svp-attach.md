# Spike S1: SVP 4 Manager attaches to a libmpv player

Question: does SVP 4 Manager detect a player built on libmpv whose process is
not called `mpv`, and switch on its interpolation through mpv's JSON IPC?

Status: **Windows: PASS** (goal C, 2026-10-10). **Linux: PASS** (goal D,
2026-10-10, the owner's Ubuntu 26.10 boot): SVP attached after 1.2 s and
interpolated 23.976 fps to 59.94 fps.

SVP 4 Manager 4.7.0.302 attached to `s1-svp-attach.exe` 1.5 s after it
started and interpolated 23.976 fps to 59.94 fps (the display runs at
59.951 Hz). SVP does not need the process to be called `mpv`; the
`mpvpipe` JSON IPC server is enough. No fallback is needed on Windows.

## Harness

`spikes/s1-svp-attach` (libmpv2 6.0.0):

```text
s1-svp-attach --input FILE [--ipc PATH] [--seconds 60] [--hwdec auto-copy]
```

It creates a normal mpv window (`vo=gpu`, `force-window=yes`) and sets the
options SVP's own mpv integration asks for: `input-ipc-server` (default
`/tmp/mpvsocket` on Linux, `mpvpipe` on Windows), `hwdec=auto-copy` and
`hr-seek-framedrop=no`. Every half second it reads `vf`,
`container-fps` and `estimated-vf-fps`, and prints when the filter chain
changes.

Output: `pid`, `executable`, `mpv_version`, `ipc`, each `vf` change with
its time, `container_fps`, `estimated_vf_fps`, `best_fps_ratio`,
`svp_filter_seen_after_s` and `result`. PASS means a `vapoursynth` filter
appeared and the filtered frame rate exceeded 1.5 times the source rate.

## How to run

- **Windows (goal C):** the bundle from `spikes/tools/build-windows.sh`
  contains `s1-svp-attach.exe` next to `libmpv-2.dll`. With SVP 4 Manager
  running, `s1-svp-attach.exe --input <video>`.
- **Linux:** `tools/linux-kit/kit.sh s1` (start SVP 4 Manager first; the
  step is skipped otherwise).

## Windows host run (goal C)

Environment: Windows host, RTX 30-series card (driver 610.88), 2560x1440 at
59.951 Hz, 100% scaling; SVP 4 Manager 4.7.0.302 already running with the
owner's profile, which uses RIFE through TensorRT 10.8
(`SVP 4\rife\vsmlrt-cuda`). Clip: `video-1080p23976-h264.mkv` from
`tools/test-clips/make-clips.sh` (1920x1080, 24000/1001 fps, 60 s).

```text
s1-svp-attach.exe --input C:\Users\<user>\rvp-test\clips\video-1080p\video-1080p23976-h264.mkv --seconds 60
  mpv_version=mpv v0.41.0-1115-gb2c255c13
  ipc=mpvpipe
  t=1.5s vf=@svp:vapoursynth=file=%53%C:\Users\<user>\AppData\Roaming\SVP4\scripts\bf315745.py:buffered-frames=4:concurrent-frames=8
  container_fps=23.976025
  estimated_vf_fps=59.939533
  best_fps_ratio=2.50
  svp_filter_seen_after_s=1.5
  result=PASS
```

The first run attached just as fast (1.8 s) but stayed at the source rate
for the whole 60 s. SVP had started TensorRT to build a RIFE engine for this
input shape (min 128x128, opt 1920x1088, max 2560x1440, FP16). The build took
62.4 s, and SVP saved the engine and its timing cache in
`SVP 4\rife\models\rife\` (12.8 MB + 0.8 MB). This is SVP's normal first
use with TensorRT 10.8; the owner's earlier cache was for TensorRT 8.5. The
owner chose to keep the files. The run above is the second one, with the
engine cached.

What SVP inserts: a VapourSynth filter labelled `@svp` whose script SVP
generates per file in `%APPDATA%\SVP4\scripts\`, with
`buffered-frames=4:concurrent-frames=8`. RVP can recognise SVP's filter by
the `@svp` label (for the status display and the fps footer) and must not
remove it when it changes its own filters.

## Ubuntu 26.10 boot (goal D)

Environment: Ubuntu 26.10 (development branch), GNOME 51 Wayland, RTX 30-series card
(NVIDIA 595.91.07), 2560x1440 at 59.95 Hz; SVP 4 for Linux 4.7.0.321
installed by SVP's installer in `~/SVP4` (its own mpv, VapourSynth R73 and
Python 3.12.11), started with the owner's profile and settings unchanged.
Clip: the kit's 1080p 23.976 fps H.264 clip (the same parameters as
`video-1080p23976-h264.mkv`). The spike ran with the kit's libmpv (mpv
`b2c255c13e`) and `VSSCRIPT_PATH` pointing at SVP's
`libvapoursynth-script.so.0`, because SVP's filter runs inside our libmpv.

```text
kit.sh s1    (RVP_VIDEO=video-1080p23976-h264.mkv, VSSCRIPT_PATH=~/SVP4/mpv/libvapoursynth-script.so.0)
  mpv_version=mpv v0.41.0-1115-gb2c255c13
  ipc=/tmp/mpvsocket
  t=1.2s vf=@svp:vapoursynth=file=%47%~/.local/share/SVP4/scripts/19004f47.py:buffered-frames=4:concurrent-frames=25
  container_fps=23.976025
  estimated_vf_fps=59.940767
  best_fps_ratio=2.50
  svp_filter_seen_after_s=1.2
  result=PASS
```

Answers to the questions goal C left for Linux:

- **Process name.** Not needed on Linux either: the process is
  `s1-svp-attach`. SVP found the player through the default
  `/tmp/mpvsocket` (the path in SVP's own Linux `mpv.conf`).
- **Filter label.** The same `@svp` label as on Windows; SVP's generated
  script lives in `~/.local/share/SVP4/scripts/` and uses
  `concurrent-frames=25` here (8 on Windows).
- **Exit.** After printing `result=PASS` the spike crashed with a
  segmentation fault while libmpv was destroyed with SVP's filter still in
  the chain (no core dump was kept: Apport ignores programs that are not
  packaged). On Windows it exited cleanly. RVP must stop playback and let
  the filter chain shut down before `mpv_terminate_destroy` (M2); the kit
  now records the spike's exit status.

The kit originally played the S3 clip (4K60) for S1 as well; SVP has
nothing to interpolate on a 60 fps source at 60 Hz, so the kit now
generates a 23.976 fps clip for S1 (`RVP_S1_VIDEO` overrides it) and passes
SVP's `VSSCRIPT_PATH` to the spike.
