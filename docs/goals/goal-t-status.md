# Goal T status: milestone M18 (motion smoothing without SVP)

Date: 2026-10-19. Result: **in progress, paused by the owner** during
M18's study, after a host run lost the host's GPU. Precondition met:
`goal-n-status.md` records goal N as done. Nothing of M18's delivery is
built yet; the study's tools are in the working tree, not committed.

## The study so far

Tools (uncommitted until the study is written up):

- `crates/engine/examples/smoothing_bench.rs`: a headless bench. `speed`
  plays a clip at mpv's top speed through an engine and prints the rate
  the chain sustains (`capacity_fps`) and its ratio to real time;
  `square` measures the edge error and the ghost ratio of interpolated
  frames of a moving square (`--raw` for frames ffmpeg wrote); `grab`
  writes frames from a keyframe start.
- `tools/windows-vm/m18-study.ps1`: every engine, preset and size on the
  Windows VM or, with permission, the host (`-Gpu`: `nvidia-smi` must see
  the GPU before and after each run, RIFE at 2160p needs
  `-AllowRife4K`, and since D17 the runs go at the 90 % power limit of
  `gpu-limit.ps1`).
- `tools/test-clips/make-m18-fixtures.sh` (`testdata/media/fixture-square.mkv`,
  bit-exact) and `make-clips.sh`'s `smooth-*` clips (a square at 1080p24,
  a Mandelbrot pan at 720p, 1080p and 2160p).

Measured, as the rate each chain sustains for 2.5x of 23.976 fps (60 fps
needed):

| Engine | Windows VM (2 vCPU) | Host (64 threads, RTX 30-series card) |
|--------|---------------------|--------------------------------|
| MVTools fast | 720p 67, 1080p 29.5, 4K 7.1; square error 1.06 px (max 12) | |
| MVTools medium | 30.6, 12.7, 2.9; error 0.08 px | 1080p 74, 4K 16.5 (about 7 cores; more VapourSynth frames do not help) |
| MVTools quality | 24.8, 10.1, 2.4; error 0.01 px | |
| FFmpeg minterpolate | 0.1 to 2.4 | 0.1 to 4.9 |
| RIFE v4.6 (ncnn-Vulkan, 4 GPU threads) | no Vulkan | 720p 113, 1080p 59.8, 4K 13.6 (other models slower) |
| FFmpeg fruc_vulkan (NVIDIA optical flow) | no Vulkan | fast 1080p 101, 4K 27; slow 36 and 9.3; square error fast 1.6, slow 0.34 px |

In RVP in real time (part B, host): mpv's Blend works only with the
Direct3D 11 renderer (with OpenGL mpv does not know the display's rate:
`display-fps-override` would be needed); MVTools medium at 1080p dropped
no frames after a 3 s start; RIFE with 2 threads at 1080p dropped about 41
frames a second; nothing reached real time at 4K.

Licences: MVTools GPL-2.0-or-later, VapourSynth LGPL-2.1, the RIFE plugin
MIT (ncnn BSD-3-Clause, the models MIT). NVIDIA's Optical Flow SDK FRUC
cannot ship with RVP (its section 4(e) forbids open-source licence terms
on it). FFmpeg's `fruc_vulkan` (merged 2026-08-29, Ampere and later) is in
FFmpeg's master builds but not in the libmpv RVP ships.

## The host's GPU loss (2026-10-18)

RVP with RIFE in its VapourSynth slot on a 2160p clip (OpenGL renderer)
hung the host's RTX 30-series card at 21:54: a TDR the driver could not recover
from ("GPU is lost"), then black screens; the device dropped off the bus,
a manual driver restart failed (`DdiAddDevice` 0xC0000010) and only a
reboot brought it back. The dumps showed:

- the video stalled about 2 s before the TDR; the OpenGL driver ended the
  process afterwards;
- at every exit after RIFE (all 16 bench runs too), the RIFE plugin's
  static destructors (ncnn) call NVIDIA's Vulkan driver during process
  shutdown and jump to address 0: ncnn needs `destroy_gpu_instance()`
  before exit, which the VapourSynth plugin never gets;
- SVP's `libEGL.dll` and `libGLESv2.dll` were loaded into `rvp.exe` from
  the user's PATH (SVP 4\mpv64), and Bandicam's implicit Vulkan layer
  too. RVP should load its libraries from its own folder only, and use
  its own VapourSynth runtime, never SVP's.

Contributing factors stay open: hardware-accelerated GPU scheduling (on;
forums report unrecoverable black screens with it on Ampere), the
Windows Insider build, driver 617.42, and the card running at its full
power limit. Afterwards the study's host runs got the GPU guard above,
RIFE at 2160p a separate permission, and D17 a power limit for host GPU
runs and RVP's own GPU power budget.

## D17: the GPU power budget (built during the pause)

RVP keeps the card below a share of its rated power (Video settings,
"Power budget", 90 % by default) by lightening its own work: EWA scalers
become spline36, then GPU Super Resolution and RTX Video HDR, the pixel
shaders and the VapourSynth script go off; each comes back after the
card stays 10 points below for a minute (`crates/app/src/gpu_power.rs`).
NVIDIA's cards are read through NVML, AMD's through Windows' adapter
counter (D3DKMT; AMD's ADLX SDK licence forbids use in GPL software) and
amdgpu's hwmon files on Linux (AMD's untested: no card here). The cards
are read only while a file plays, and on Linux never while the system has
put a card to sleep (reading would wake it). Checked: the unit tests,
`rvp --selftest=gpu` (14 checks) in WSL under Xvfb and on the Windows VM,
NVML reading the host's card from WSL (33.8 W of 350 W), the Windows
build and its reader test on the VM (whose adapter has no counter). Not
yet checked: what Windows' adapter counter reports for a real card
(compare it with NVML on the host; `RVP_GPU_POWER_ADAPTERS=all` reads
every adapter). M18's smoothing joins the budget's steps. Committed as
8af11bb.

## Host session, 2026-10-19 (with the owner's permission)

`m18-study.ps1 -Gpu -PowerLimit 90` with the owner's UAC answer: the
limit was 315 W for the whole session and 350 W again afterwards
(`restored card=0 limit=350`); `nvidia-smi` saw the card before and after
every run.

- Windows' adapter counter against NVML on the RTX 30-series card: idle, 32.5 %
  against 114.7 W (32.8 % of the 350 W default); under RIFE at 1080p,
  95.3 % against 295.5 W (84.4 % of the default, 93.8 % of the 315 W in
  force). The counter follows the card's draw; under load it read 11
  points above NVML's share of the default (a share of the limit in
  force, or an instant reading against NVML's one-second average: one
  sample each). For AMD's cards on Windows the budget therefore steps in
  early rather than late.
- RIFE v4.6 at 1080p with 2 threads at the 315 W limit: 42.5 fps (51.3
  without the limit on 2026-10-18), mean 159 W, peak 297 W; the square's
  edge error 0.09 px.
- Part C, the owner's clips (frames kept in
  `~/rvp-reference/m18/partc-2026-10-19`): Apex (1080p, 24 fps,
  a fast pan through trees, from 44:56) and 28 Years Later (720p, 23.976
  fps, letterboxed, from 14:59) through MVTools medium and quality, RIFE
  and fruc_vulkan medium and quality. Every motion-compensated engine
  made one coherent picture halfway where Blend shows two; RIFE's is the
  sharpest, MVTools' slightly softer. fruc_vulkan flickers at the
  letterbox's edges (the change of the rows there against both
  neighbours: 2.4 to 11.7 at medium, 2.0 to 4.1 at quality, against 1.4
  to 1.6 in the source and MVTools and at most 1.2 with RIFE).
- MVTools in mpv's VapourSynth filter gives three black frames after a
  seek before its first picture (the fourth is half black): delivery has
  to hide them.
- ffmpeg's `-noaccurate_seek` dropped the frames before the requested
  time, so fruc_vulkan's grabs start at the time, not at the keyframe
  the others start from; its frames were compared with their own
  neighbours.

## Left

1. Part C: done (above).
2. `docs/spikes/m18-smoothing.md`: written. The owner's choice (D18,
   2026-10-19): MVTools, RIFE and fruc_vulkan (with a libmpv RVP builds
   with FFmpeg's master), Blend always; off by default, MVTools medium up
   to 1080p and Blend above when turned on; the display's rate.
3. M18's delivery as `docs/PLAN.md` lists it, with the smoothing as
   budget steps, RVP's own VapourSynth runtime, and libraries loaded from
   RVP's folder only; the RIFE exit crash handled if RIFE is chosen.
4. Tests, `m18-all.ps1` on the Windows VM, one host session at the power
   limit, the portable build in `C:\Users\<user>\rvp-test\rvp-t`.
