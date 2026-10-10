# Spike S2: VapourSynth motion interpolation through libmpv

Question: does libmpv with VapourSynth run PotPlayer's
`VapourSynth/motioninterpolation.vpy` unchanged, headless, with a VapourSynth
and Python runtime compatible with SVP 4's?

Answer: **not as written; the owner accepted a run-time wrapper.** With SVP's runtime as it
is (VapourSynth R73), the unchanged script fails on line 2: R73 removed
`vs.get_core()`. The libmpv + VapourSynth + mvtools chain itself works. The
unchanged file (SHA-256 identical to PotPlayer's) interpolates 23.976 fps to
60 fps in two ways, each of which adds something that SVP's runtime does not
have:

| Run | What is added | Result |
|-----|---------------|--------|
| Original file, SVP-compatible runtime as is | nothing | FAIL (`get_core` missing) |
| Original file passed directly to libmpv | a 6-line `sitecustomize.py` in the Python runtime that defines `vs.get_core` (`spikes/s2-vapoursynth/shim/`) and `import site` enabled in `python312._pth` | PASS, 60.000 fps |
| Generated wrapper `.vpy` that defines `get_core` and executes the original file (`--compat`) | nothing in the runtime | PASS, 60.000 fps |

## Runtime compared with SVP 4

Read-only inspection of `C:\Program Files (x86)\SVP 4\mpv64` on the host:

| Component | SVP 4 | Spike runtime |
|-----------|-------|---------------|
| mpv | `mpv v0.41.0-dirty`, built by SVP with `-Dvapoursynth=enabled` | libmpv from shinchiro 20261010, `mpv v0.41.0-1115-gb2c255c13` (master) |
| FFmpeg | n7.1.3 | N-127267-gef305e32e |
| VapourSynth | Core R73 | R73 portable (`VapourSynth64-Portable-R73.zip`) |
| Python | embeddable 3.12.9 | embeddable 3.12.9 |
| Plugins | AvsCompat, MiscFilters (SVP brings its own svpflow) | AvsCompat, mvtools v24 (`libmvtools.dll`, needed by the script) |

libmpv loads VSScript at run time (it honours `VSSCRIPT_PATH`), so the same
libmpv works with any compatible VapourSynth install.

PotPlayer bundles an older VapourSynth (Python 3.6/3.9 modules in
`Module/VapourSynth`), which still had `get_core()`.

## Environment

- Windows test VM `<windows-vm>`: Windows 11 10.0.26340.9616, AMD Ryzen
  Embedded V1500B (2 vCPU), 8 GB, no GPU (Microsoft Basic Display Adapter).
- Harness: `spikes/s2-vapoursynth` (libmpv2 6.0.0), cross-built from WSL; bundle
  from `spikes/tools/build-windows.sh`.
- Input: `av://lavfi:testsrc2=size=1280x720:rate=24000/1001` (generated, no
  file needed). Options: `vo=null`, `ao=null`, `hwdec=no`,
  `hr-seek-framedrop=no`, `display-fps-override=60`,
  `vf=vapoursynth=file=%LEN%PATH:buffered-frames=4`.

## Commands and results

```text
s2-vapoursynth.exe --script C:/rvp-spikes/s2-bundle/motioninterpolation.vpy --seconds 15
  [vapoursynth] Python exception: module 'vapoursynth' has no attribute 'get_core'
  [vapoursynth]   File "C:/rvp-spikes/s2-bundle/motioninterpolation.vpy", line 2, in <module>
  [vapoursynth]     core = vs.get_core()
  Disabling filter vapoursynth.00 because it has failed.
  container_fps=23.976
  realtime_ratio=1.002
  estimated_vf_fps_mean=23.976
  result=FAIL

s2-vapoursynth.exe --script C:/rvp-spikes/s2-bundle/motioninterpolation.vpy --seconds 15 --compat
  container_fps=23.976
  realtime_ratio=0.831
  estimated_vf_fps_mean=60.000
  result=PASS

# copy of s2-bundle with spikes/s2-vapoursynth/shim/sitecustomize.py in
# Lib/site-packages and "import site" enabled in python312._pth
s2-vapoursynth.exe --script C:/rvp-spikes/s2-shim/motioninterpolation.vpy --seconds 15
  [vapoursynth] Frame requested during init! This is unsupported.
  [vapoursynth] Returning black dummy frame with 0 duration.
  vf=vapoursynth=file=%45%C:/rvp-spikes/s2-shim/motioninterpolation.vpy:buffered-frames=4
  container_fps=23.976
  realtime_ratio=0.769
  estimated_vf_fps_mean=60.000
  result=PASS
```

The "frame requested during init" warning comes from mpv's filter while
the script initializes; it costs one black frame at the start.

`realtime_ratio` is playback time divided by wall time: on two virtual CPUs
without a GPU, mvtools at 1280x720 keeps up with 83% of real time. Speed is
for the owner's machine to measure (goal C and goal D).

## Decision

PLAN.md's pass condition ("runs unchanged" with an SVP-compatible runtime)
cannot be met with VapourSynth R73 as shipped. Options considered; the owner
chose option 1 on 2026-10-10:

1. **Wrapper at run time (recommended).** RVP hands libmpv a generated
   wrapper that defines `get_core` and executes the original file in the same
   globals. Nothing installed is changed, so it also works with SVP's own
   runtime on Linux, which RVP must not modify.
2. **Shim in the runtime.** The original file goes to libmpv directly, but the
   Python runtime needs `sitecustomize.py`. That works only for a runtime RVP
   ships itself; it cannot be applied to SVP's installation.
3. **An older VapourSynth** that still has `get_core()` (PotPlayer bundles
   one). Not compatible with SVP's R73 runtime, so it contradicts S2's premise.

## Findings for RVP

1. **`get_core` compatibility.** Whichever option is chosen, `vs.get_core()`
   must be provided. The spike's wrapper (`write_compat_wrapper` in
   `spikes/s2-vapoursynth/src/main.rs`) executes the original file in the
   same globals, so mpv's `video_in`, `container_fps` and `display_fps`
   reach it. It also loads extra plugins listed in `RVP_VS_PLUGINS`, because
   SVP's runtime has no mvtools.
2. **Quote filter arguments.** mpv splits filter arguments at `:`, so a
   Windows path such as `C:/...` must be written as `file=%LEN%C:/...`.
   Unquoted, the drive letter is silently dropped.
3. **Runtime layout.** The portable VapourSynth wheel keeps `vapoursynth.dll`
   in its data directory; without pip it has to be moved next to
   `vapoursynth.pyd` (see `spikes/tools/windows-deps.sh`).
4. **Linux.** mpv master loads `libvapoursynth-script.so` with `dlopen` and
   needs only the VapourSynth headers at build time. The Linux kit builds
   libmpv that way, so SVP's own runtime can be used through `VSSCRIPT_PATH`.
   There is no Linux VapourSynth wheel for R73, and the shared Ubuntu 22.04
   VM has Python 3.10, so S2 on Linux runs on the owner's Ubuntu 26.10 boot
   (goal D).
