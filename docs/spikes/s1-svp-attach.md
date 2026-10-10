# Spike S1: SVP 4 Manager attaches to a libmpv player

Question: does SVP 4 Manager detect a player built on libmpv whose process is
not called `mpv`, and switch on its interpolation through mpv's JSON IPC?

Status: **prepared, not run.** S1 needs SVP 4 Manager running on a desktop,
so it runs on the Windows host in goal C (with the owner's permission) and
on the owner's Ubuntu 26.10 boot through the Linux kit.

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

## What to look for

- Whether SVP needs the process name `mpv` (or `mpv.exe`). If it does,
  record what it checks (read-only) and evaluate the fallbacks in PLAN.md:
  process naming, or real `mpv` embedded through `wid` on X11 and Windows.
- Which IPC path SVP finds. SVP's own `mpvSockets.lua` creates
  `<tmp>/mpvSockets/<pid>`, its `mpv.conf` uses `mpvpipe` on Windows, and on
  Linux SVP looks up the socket's owner through `lsof` (in the kit's package
  list). If the default path is not picked up, rerun with
  `--ipc /tmp/mpvSockets/<pid>` style paths.
- Which `vf` string SVP inserts, so RVP can show and remove it.
