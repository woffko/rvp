# M18 study: motion smoothing without SVP

Goal T (D16) asked what RVP can use for smooth motion without SVP. This
file records what each engine reached on the Windows VM and the host, what
it looks like on synthetic clips and on two of the owner's films, how RVP
would get it onto the user's machine, and a recommendation. The owner picks
the engines and the defaults; M18's delivery follows that choice.

## How it was measured

- `crates/engine/examples/smoothing_bench.rs` runs one engine in a headless
  libmpv core (shinchiro's libmpv of 2026-10-10, as RVP ships it):
  - `speed` plays a clip as fast as the chain allows and prints the output
    rate it sustains (`capacity_fps`);
  - `square` finds a white square moving over grey in each output frame
    and measures how far its edge is from where it should be halfway
    (`edge_error`, pixels) and how many smeared pixels it leaves
    (`ghost_ratio`);
  - `grab` writes the output frames from the keyframe before a time.
- `tools/windows-vm/m18-study.ps1` runs every engine, preset and size, and
  on the host checks the GPU before and after each run. Since D17 those
  runs go at 90 % of the card's power limit.
- The clips come from `tools/test-clips/make-clips.sh`: a square at
  1080p24 and a Mandelbrot pan at 720p, 1080p and 2160p. The owner's clips
  are Apex (1080p, 24 fps, a fast pan through trees) and 28 Years Later
  (720p, 23.976 fps, letterboxed, people walking). The frames stay local.
- The machines are the Windows VM (2 vCPUs, no GPU) and the host (64
  threads, RTX 30-series card, driver 617.42).

The target is 2.5x of 23.976 fps, a 60 Hz display, so an engine needs 60
fps to keep up. At 2x it needs 48.

## The engines

| Engine | What it is | Where it runs |
|--------|------------|---------------|
| Blend | mpv's own interpolation (`interpolation`, `tscale`, `video-sync=display-resample`): neighbouring frames blended at the display's rate, no motion estimation | The renderer, almost free |
| MVTools | Block motion estimation and compensation (`mv.Super`, `mv.Analyse`, `mv.FlowFPS`/`BlockFPS`) in VapourSynth: the method SVP grew from | CPU |
| RIFE | A neural network's optical flow (the vs-rife ncnn-Vulkan plugin, model v4.6) in VapourSynth | GPU through Vulkan |
| fruc_vulkan | FFmpeg's filter over NVIDIA's optical flow hardware (`VK_NV_optical_flow`) | GPU, RTX 30 and later |
| minterpolate | FFmpeg's motion-compensated interpolation | CPU, one thread |
| NVIDIA Optical Flow SDK FRUC | NVIDIA's library | Not usable: see licences |

Presets as the bench sets them:

- **MVTools fast:** `BlockFPS`, blocks of 32, pel 1.
- **MVTools medium:** `FlowFPS`, blocks of 16, pel 2.
- **MVTools quality:** medium with `Recalculate` at blocks of 8.
- **RIFE:** fast, medium and quality are 1, 2 and 4 GPU threads.
- **fruc_vulkan:** `perf` fast, medium and slow.
- **minterpolate:** from OBMC with EPZS search up to AOBMC with UMH.

## Speed

The output rate each chain sustains, frames a second (60 needed at 2.5x,
48 at 2x):

| Engine and preset | Host 720p | Host 1080p | Host 2160p | VM 720p | VM 1080p | VM 2160p |
|-------------------|----------:|-----------:|-----------:|--------:|---------:|---------:|
| Decoding only | 2355 | 1294 | 447 | | | |
| MVTools fast | 481 | 166 | 37.3 | 67 | 29.5 | 7.1 |
| MVTools medium | 186 | 74.1 | 16.5 | 30.6 | 12.7 | 2.9 |
| MVTools quality | 157 | 51.0 | 14.2 | 24.8 | 10.1 | 2.4 |
| RIFE v4.6, 1 thread | 53.9 | 26.6 | 6.5 | no Vulkan | | |
| RIFE v4.6, 2 threads | 101 | 51.3 | 11.4 | | | |
| RIFE v4.6, 4 threads | 113 | 59.8 | 13.6 | | | |
| fruc_vulkan fast | 193 | 101 | 26.9 | no Vulkan | | |
| fruc_vulkan medium | 114 | 55.4 | 14.4 | | | |
| fruc_vulkan slow | 80.1 | 36.4 | 9.3 | | | |
| minterpolate fast | 4.9 | 2.0 | 0.4 | at most 2.4 | | |

Notes on the speed figures:

- **MVTools at 1080p** uses about 7 of the host's cores. More of
  VapourSynth's frames in flight did not help (medium with 32: 69.1 fps).
- **Other RIFE models are slower** than v4.6 and no better on the square:
  v4.22 lite, v4.25 lite and v4.26 reached 42.8 to 45.8 fps at 1080p.
- **The power limit costs RIFE speed.** At 1080p with 2 threads it reached
  42.5 fps at the 315 W limit (D17), against 51.3 without it.

## Accuracy on the square (2x)

| Engine | Edge error, mean (max), px | Ghost ratio |
|--------|---------------------------|------------:|
| MVTools fast, medium, quality | 0.01 (0) | 0.000 to 0.002 |
| RIFE v4.6 | 0.09 (1) | 0.002 |
| RIFE v4.22 lite / v4.25 lite / v4.26 | 0.17 / 0.34 / 0.42 (1 to 2) | 0.001 to 0.002 |
| minterpolate fast, medium | 0.18 (1) | 0.006 |
| fruc_vulkan slow | 0.34 (1) | 0.010 |
| fruc_vulkan fast / medium | 1.59 (8) / 1.67 (9) | 0.026 |

On the VM, MVTools fast missed by 1.06 px on average (12 at most), medium
by 0.08 and quality by 0.01.

## In RVP, in real time (host)

- **Blend** works with the Direct3D 11 renderer. With OpenGL, mpv does not
  know the display's rate in the render API and needs
  `display-fps-override`; RVP can take the rate from the screen Qt reports.
- **MVTools medium at 1080p** dropped no frames after a burst in the first
  3 s.
- **RIFE with 2 threads at 1080p** dropped about 41 frames a second: it did
  not keep up at the display's rate.
- **At 4K** nothing kept up. RIFE at 4K hung the GPU on 2026-10-18 (below).

## The owner's clips (part C)

Frames halfway between two source frames, after MVTools' first frames
(below), compared with the source frames on each side:

- **Blend** shows both pictures at half strength: doubled branches in
  Apex, doubled edges on the people in 28 Years Later.
- **MVTools, medium and quality,** make one picture halfway, slightly
  softer than the source at edges that move fast.
- **RIFE** makes one picture halfway, the sharpest of the engines.
- **fruc_vulkan** makes one picture halfway, but the rows at the
  letterbox's edges flicker. Their change against both neighbouring frames
  is 2.4 to 11.7 at medium and 2.0 to 4.1 at slow. It is 1.4 to 1.6 in the
  source and with MVTools, and at most 1.2 with RIFE.

## Findings

- **MVTools after a seek.** In mpv's VapourSynth filter MVTools gives three
  black frames after a seek, and the fourth is half black, before its first
  picture. During playback this would flash black after every seek;
  delivery has to hide it.
- **RIFE at process exit.** After every run with RIFE, the process crashes
  at exit. The plugin's ncnn instance is torn down by static destructors
  during process shutdown, calls NVIDIA's Vulkan driver, and jumps to
  address 0. ncnn needs `destroy_gpu_instance()` before exit, which the
  VapourSynth plugin never gets. RVP would have to end its process without
  running those destructors once RIFE has run, after saving its state. The
  plugin also needs `vcomp140.dll` (Microsoft's OpenMP runtime).
- **The GPU loss.** RVP with RIFE on a 2160p clip, with the OpenGL renderer,
  hung the RTX 30-series card at 21:54 on 2026-10-18:
  - the driver could not recover from the TDR, and only a reboot brought
    the card back;
  - the video had stalled about 2 s before it;
  - VRAM went from a baseline of 3.1 GB to 7.1 GB with RIFE at 4K and 2
    threads, and to 10.7 GB of 12 with 4 threads (the bench).

  Contributing factors stay open: hardware-accelerated GPU scheduling, the
  Windows Insider build, the driver, and the card at its full power limit.
  Since then host runs check the GPU before and after each run, RIFE at
  2160p needs the owner's separate permission, and D17 brought a power
  limit for the tests and the budget in the player.
- **DLLs from the user's PATH.** `rvp.exe` had SVP's `libEGL.dll` and
  `libGLESv2.dll` loaded from the user's PATH (SVP 4\mpv64), because libmpv
  probes ANGLE. Bandicam's implicit Vulkan layer was loaded too. RVP should
  load its libraries from its own folder only. It must use a VapourSynth
  runtime of its own, never SVP's (goal T's rule).
- **fruc_vulkan is not in RVP's libmpv.** The filter was merged into FFmpeg
  on 2026-08-29 and is in FFmpeg's master builds, not in shinchiro's
  libmpv. It also needs Vulkan decoding (`hwdec=vulkan`) and NVIDIA's
  optical flow hardware.

## Licences and how each engine reaches the user

| Engine | Licence | Files (Windows) | Delivery |
|--------|---------|-----------------|----------|
| Blend | mpv's | none | Always there |
| MVTools | MVTools GPL-2.0-or-later; VapourSynth LGPL-2.1; Python PSF | VapourSynth R73 portable (13.6 MB zipped), embeddable Python 3.12 (11.1 MB), mvtools v24 (2.5 MB); 57 MB unpacked with mvtools | In RVP's packages: RVP's own runtime, found before any other. On Linux, the distribution's `vapoursynth` and `vapoursynth-mvtools` or the AppImage (goal S). |
| RIFE | Plugin MIT, ncnn BSD-3-Clause, models MIT | Plugin 5.1 MB, model v4.6 11 MB, `vcomp140.dll` | Bundled with the MVTools runtime |
| fruc_vulkan | FFmpeg LGPL/GPL | An FFmpeg and libmpv of RVP's own build | Not before RVP's libmpv has it |
| minterpolate | FFmpeg's | none | Too slow |
| NVIDIA Optical Flow SDK FRUC | NVIDIA's SDK licence | | Section 4(e) forbids open-source licence terms on it |

All bundled licences are compatible with RVP's GPL-3.0-or-later.

## Recommendation

1. **Off by default.** Turned on, the default is MVTools medium at the
   display's rate up to 1080p, with Blend above that.
   - MVTools runs on any machine with enough cores, keeps up at 1080p on
     the host (74 fps), and is the most exact on the square.
   - It needs no GPU, so it cannot hang one.
   - The size limit and an automatic choice of preset follow the measured
     capacity, since a 2-core machine reaches only 720p.
2. **Blend is always there**: Direct3D 11, or OpenGL with the screen's
   rate.
3. **RIFE as an opt-in GPU mode, up to 1080p.** It is the sharpest on real
   films. It needs a Vulkan GPU, keeps up only at 720p, or at 1080p at 2x
   with 4 threads, and needs the exit fix and its power-budget steps.
4. **Not now:** fruc_vulkan (not in RVP's libmpv, RTX 30 and later only,
   flicker at letterbox edges); minterpolate (too slow); NVIDIA's FRUC SDK
   (licence).
5. **Budget steps (D17)**, before RVP's other steps:
   - MVTools quality → medium → fast → Blend → off.
   - RIFE → MVTools medium → Blend → off.
6. **Fix MVTools' black frames after seeks**, and load libraries from RVP's
   own folder only.

## The owner's choice (D18, 2026-10-19)

- Engines: MVTools, RIFE and fruc_vulkan. Blend is always there.
  fruc_vulkan comes through a libmpv that RVP builds itself with
  FFmpeg's master branch.
- Default: off. Turned on, MVTools medium up to 1080p and Blend above.
- Target rate: the display's.
