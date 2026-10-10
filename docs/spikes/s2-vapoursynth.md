# Spike S2: VapourSynth motion interpolation through libmpv

Question: does libmpv with VapourSynth run PotPlayer's
`VapourSynth/motioninterpolation.vpy` unchanged, headless, with a VapourSynth
and Python runtime compatible with SVP 4's?

Answer: **the chain works, but the script does not run unchanged.**
VapourSynth R73 (SVP's version) removed `vs.get_core()`, which the script
calls on line 2. Run through a small wrapper that restores `get_core`, the
unchanged script file interpolates 23.976 fps to 60 fps.

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
```

`realtime_ratio` is playback time divided by wall time: on two virtual CPUs
without a GPU, mvtools at 1280x720 keeps up with 83% of real time. Speed is
for the owner's machine to measure (goal C and goal D).

## Findings for RVP

1. **Compatibility wrapper.** RVP should run user `.vpy` files through a
   wrapper that restores `vs.get_core()` and then executes the original file
   in the same globals, so mpv's `video_in`, `container_fps` and
   `display_fps` reach it. The spike's wrapper (`write_compat_wrapper` in
   `spikes/s2-vapoursynth/src/main.rs`) does this without touching the file.
   It also loads extra plugins listed in `RVP_VS_PLUGINS`, because SVP's
   runtime has no mvtools.
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
