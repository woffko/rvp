# Spike S1: SVP 4 Manager attaches to a libmpv player

Question: does SVP 4 Manager detect a player built on libmpv whose process is
not called `mpv`, and switch on its interpolation through mpv's JSON IPC?

Status: **Windows: PASS** (goal C, 2026-10-10). Linux: runs on the owner's
Ubuntu 26.10 boot through the Linux kit (goal D).

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

## What to look for (Linux, goal D)

- Whether SVP needs the process name `mpv` (on Windows it does not). If it does,
  record what it checks (read-only) and evaluate the fallbacks in PLAN.md:
  process naming, or real `mpv` embedded through `wid` on X11 and Windows.
- Which IPC path SVP finds. SVP's own `mpvSockets.lua` creates
  `<tmp>/mpvSockets/<pid>`, its `mpv.conf` uses `mpvpipe` on Windows, and on
  Linux SVP looks up the socket's owner through `lsof` (in the kit's package
  list). If the default path is not picked up, rerun with
  `--ipc /tmp/mpvSockets/<pid>` style paths.
- Whether the inserted filter carries the same `@svp` label as on Windows.
