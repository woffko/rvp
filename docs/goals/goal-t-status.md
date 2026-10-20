# Goal T status: milestone M18 (motion smoothing without SVP)

Date: 2026-10-20. Result: **done**. M18 is built and checked on the
Windows VM, in WSL and in four host sessions at the power limit. RVP's own
libmpv with NVIDIA Optical Flow is built and in the local packages. The
sessions found five problems, all fixed (see "Findings" 7 to 11). The
final build is in `C:\Users\<user>\rvp-test\rvp-t`.

Precondition met: `goal-n-status.md` records goal N as done. The goal was
paused on 2026-10-18 after a host run lost the GPU, and resumed on
2026-10-19 with D17's GPU power budget.

## Summary

| Part | Result | Where |
|------|--------|-------|
| The plan | One plan per file. Smoothing is off by default. When on, it uses MVTools medium up to 1080p and Blend above (the size limit is a setting), at the display's rate (Qt's refresh rate), at twice the video's rate or at a set rate. It stays off while SVP's `@svp` is in the chain (and comes back when `@svp` goes), while RVP's own VapourSynth script slot runs, and when the video already plays at the target rate. While SVP 4 Manager runs, it waits SVP's 8 s after a file starts (SVP does not attach to a video RVP's filter already plays). | `crates/app/src/smoothing.rs`, `svp.rs` |
| Blend | mpv's `interpolation` with `video-sync=display-resample` and `tscale=oversample`. With the OpenGL renderer RVP gives mpv the display's rate (`display-fps-override`). | `smoothing.rs` |
| MVTools and RIFE | RVP's labelled filter `@rvpsmooth`, last among RVP's filters, on a script RVP writes into its state folder, with copy-back decoding. MVTools has three presets: fast (`BlockFPS` with scene-change thresholds), medium (`FlowFPS`) and quality (`FlowFPS` with `Recalculate` at 8 px blocks), with as many frames in flight as mpv's default (one per logical CPU). RIFE (model v4.6) uses 1, 2 or 4 GPU threads with 8 frames in flight. Both scripts end with `core.clear_cache()` (see "Findings"). | `smoothing.rs` |
| NVIDIA Optical Flow | FFmpeg's `fruc_vulkan` (RTX 30 and later) as `@rvpsmooth`. It needs the Vulkan renderer: two `format` filters of RVP's before it make the decoded frames NV12 and have mpv upload them to its output's Vulkan device, and the filter's frames stay there. RVP probes at start whether libmpv's FFmpeg has the filter. | `smoothing.rs`, `render.rs` |
| Fallbacks | RIFE and NVIDIA Optical Flow give way to MVTools where they cannot run, and MVTools gives way to Blend. An engine whose filter plays for 6 s without raising the rate above 1.2x the source's is left out for the session, with a notice. One that runs but leaves the output dropping more than 5 % of its frames over 6 s (after its first 6 s) takes a lighter step for the rest of the file, with a notice: a lighter preset, then RIFE or NVIDIA Optical Flow give way to MVTools, then Blend (the owner, 2026-10-20). | `smoothing.rs` (`smoothing_watch`) |
| Controls | Video > Motion Smoothing: on and off (Ctrl+Shift+I), the four engines, Fast/Medium/Quality, the display's rate or twice the video's, and the settings. The Motion Smoothing page is in the Video group (engine, preset, rate, size limit, the plan now, the engines found here). The status gives the plan with the rate it makes. | `action.rs`, `menu.rs`, `shell/qml/Settings.qml` |
| GPU power budget (D17) | Smoothing comes first among the budget's steps: a lighter preset (RIFE gives way to MVTools), then Blend, then off. After that come the EWA scalers, GPU Super Resolution and RTX Video HDR, the shaders and the script. Each step comes back as before. | `crates/app/src/gpu_power.rs` |
| VapourSynth runtime | RVP's own runtime in `vapoursynth\` next to `rvp.exe`: VapourSynth R73 portable on an embeddable Python 3.12.9, MVTools v24, and RIFE's ncnn-Vulkan plugin r9_mod_v33 with model v4.6 and its runtime libraries. Every download is checked by its SHA-256, and `NOTICE.txt` names the licences. RVP never uses SVP's runtime or one found on the PATH. | `tools/packaging/vapoursynth.py`, `crates/app/src/svp.rs` |
| Libraries and exit | On Windows RVP loads libraries from its own folder, System32 and its runtime only. A process that has loaded RIFE ends with `TerminateProcess`. | `shell/src/process.rs` |
| Vulkan renderer | `render.renderer` `vulkan`: mpv's `gpu-next` with `winvk` in RVP's native video window, like Direct3D 11's, taken at start. | `render.rs`, `shell/cpp/video_window.cpp` |
| RVP's libmpv | shinchiro's recipe at a fixed commit, with glslang for the build host, which FFmpeg needs to compile its Vulkan shaders (`fruc_vulkan` among them), and curl held at its 8.22.0 release; `rootless.sh` builds it without sudo. Built on 2026-10-19 (see "RVP's libmpv"); the local Windows packages carry it, CI keeps shinchiro's. | `tools/libmpv/` |

## The study

`docs/spikes/m18-smoothing.md` has the study in full. The tools:

- `crates/engine/examples/smoothing_bench.rs`, a headless bench with three
  modes:
  - `speed` plays a clip at mpv's top speed through an engine and prints
    the rate the chain sustains (`capacity_fps`) and its ratio to real
    time;
  - `square` measures the edge error and the ghost ratio of interpolated
    frames of a moving square (`--raw` for frames ffmpeg wrote);
  - `grab` writes frames from a keyframe start.
- `tools/windows-vm/m18-study.ps1` runs every engine, preset and size on
  the Windows VM or, with permission, on the host. With `-Gpu`, nvidia-smi
  must see the GPU before and after each run, the runs go at the 90 %
  power limit of `gpu-limit.ps1`, and RIFE at 2160p needs `-AllowRife4K`.
- `tools/test-clips/make-m18-fixtures.sh` makes
  `testdata/media/fixture-square.mkv` (bit-exact), and `make-clips.sh`
  makes the `smooth-*` clips: a square at 1080p24 and a Mandelbrot pan at
  720p, 1080p and 2160p.

Measured as the rate each chain sustains for 2.5x of 23.976 fps (60 fps
needed):

| Engine | Windows VM (2 vCPU) | Host (64 threads, RTX 30-series card) |
|--------|---------------------|--------------------------------|
| MVTools fast | 720p 67, 1080p 29.5, 4K 7.1; square error 1.06 px (max 12) | |
| MVTools medium | 30.6, 12.7, 2.9; error 0.08 px | 1080p 74, 4K 16.5 (about 7 cores; more VapourSynth frames do not help) |
| MVTools quality | 24.8, 10.1, 2.4; error 0.01 px | |
| FFmpeg minterpolate | 0.1 to 2.4 | 0.1 to 4.9 |
| RIFE v4.6 (ncnn-Vulkan, 4 GPU threads) | no Vulkan | 720p 113, 1080p 59.8, 4K 13.6 (other models slower) |
| FFmpeg fruc_vulkan (NVIDIA optical flow) | no Vulkan | fast 1080p 101, 4K 27; slow 36 and 9.3; square error fast 1.6, slow 0.34 px |

In RVP in real time (part B, host, 2026-10-18):

- mpv's Blend worked only with the Direct3D 11 renderer. With OpenGL, mpv
  does not know the display's rate, so delivery gives it
  `display-fps-override`.
- MVTools medium at 1080p dropped no frames after a 3 s start.
- RIFE with 2 threads at 1080p dropped about 41 frames a second.
- Nothing reached real time at 4K.

Licences:

- MVTools is GPL-2.0-or-later and VapourSynth LGPL-2.1.
- The RIFE plugin is MIT, with ncnn under BSD-3-Clause and the models
  under MIT.
- NVIDIA's Optical Flow SDK FRUC cannot ship with RVP: its section 4(e)
  forbids open-source licence terms on it.
- FFmpeg's `fruc_vulkan` (merged 2026-08-29) is LGPL with the rest of
  FFmpeg.

### The host's GPU loss (2026-10-18)

RVP with RIFE in its VapourSynth slot on a 2160p clip (OpenGL renderer)
hung the host's RTX 30-series card at 21:54. It was a TDR the driver could not
recover from ("GPU is lost"), followed by black screens. The device dropped
off the bus, a manual driver restart failed (`DdiAddDevice` 0xC0000010),
and only a reboot brought it back. The dumps showed:

- the video stalled about 2 s before the TDR, and the OpenGL driver ended
  the process afterwards;
- at every exit after RIFE (all 16 bench runs too), the RIFE plugin's
  static destructors (ncnn) call NVIDIA's Vulkan driver during process
  shutdown and jump to address 0. ncnn needs `destroy_gpu_instance()`
  before exit, which the VapourSynth plugin never gets;
- SVP's `libEGL.dll` and `libGLESv2.dll` were loaded into `rvp.exe` from
  the user's PATH (SVP 4\mpv64), and so was Bandicam's implicit Vulkan
  layer.

Contributing factors stay open: hardware-accelerated GPU scheduling (it is
on, and forums report unrecoverable black screens with it on Ampere), the
Windows Insider build, driver 617.42, and the card running at its full
power limit.

After the loss, the study's host runs got the GPU guard above, and RIFE at
2160p needs a separate permission. D17 added a power limit for host GPU
runs and RVP's own GPU power budget (8af11bb).

### Host session, 2026-10-19 (with the owner's permission)

`m18-study.ps1 -Gpu -PowerLimit 90` ran after the owner answered the UAC
prompt. The limit was 315 W for the whole session and 350 W again
afterwards (`restored card=0 limit=350`), and nvidia-smi saw the card
before and after every run.

- **Windows' adapter counter against NVML on the RTX 30-series card.** At idle the
  counter read 32.5 % and NVML 114.7 W (32.8 % of the 350 W default).
  Under RIFE at 1080p the counter read 95.3 % and NVML 295.5 W (84.4 % of
  the default, 93.8 % of the 315 W in force). The counter follows the
  card's draw. Under load it read 11 points above NVML's share of the
  default, either as a share of the limit in force or as an instant
  reading against NVML's one-second average (one sample each). For AMD's
  cards on Windows the budget therefore steps in early rather than late.
- **RIFE v4.6 at 1080p with 2 threads at the 315 W limit:** 42.5 fps
  (51.3 fps without the limit on 2026-10-18), mean 159 W, peak 297 W. The
  square's edge error was 0.09 px.
- **Part C, the owner's clips.** The frames are kept in
  `~/rvp-reference/m18/partc-2026-10-19`. Apex (1080p, 24 fps, a
  fast pan through trees, from 44:56) and 28 Years Later (720p, 23.976
  fps, letterboxed, from 14:59) went through MVTools medium and quality,
  RIFE, and fruc_vulkan medium and quality:
  - every motion-compensated engine made one coherent picture halfway,
    where Blend shows two;
  - RIFE's picture is the sharpest, MVTools' slightly softer;
  - fruc_vulkan flickers at the letterbox's edges. Measured as the change
    of the rows there against both neighbours, it is 2.4 to 11.7 at
    medium and 2.0 to 4.1 at quality, against 1.4 to 1.6 in the source and
    MVTools and at most 1.2 with RIFE.

The owner chose MVTools, RIFE and fruc_vulkan, through a libmpv RVP builds,
with Blend always there. Smoothing is off by default; when turned on, it is
MVTools medium up to 1080p and Blend above, at the display's rate (D18).

## Host sessions of the delivery (2026-10-19, with the owner's permission)

Three sessions of `tools/windows-vm/m18-host.ps1` from `rvp-t` with RVP's
libmpv, each after the owner answered one UAC prompt. The card's limit was
315 W during each and 350 W again afterwards (`restored card=0 limit=350`,
and nvidia-smi from WSL), nvidia-smi saw the card before and after every
run, and no run lost the GPU or crashed at exit (no crash logs, RIFE's
included). The display is 2560x1440 at 59.951 Hz, so the target was 59.94
fps for the 23.976 fps clips (`smooth-pan-*`, 20 s; the first session ran
30 s, past their end, which failed only `still_playing`).

| Run | Session | Plan at the end | Dropped after the first 10 s | GPU, power (mean, peak) | RVP's CPU |
|-----|---------|-----------------|------------------------------|-------------------------|-----------|
| Blend, Direct3D 11, 1080p | 1 | Blend | 0 | 20 %, 47 / 117 W | |
| MVTools medium, OpenGL, 1080p | 1 | MVTools medium | 478 in 10 s | 14 %, 54 / 115 W | |
| the same after the fix (finding 7) | 2 | MVTools medium | 0 | 21 %, 71 / 137 W | 4.8 cores |
| MVTools medium, Direct3D 11, 1080p, fixed | 2 | MVTools medium | 0 | 26 %, 60 / 141 W | 4.5 cores |
| RIFE medium, 720p | 1 | RIFE medium | 0 | 32 %, 166 / 286 W | |
| RIFE medium, 1080p | 2 | RIFE medium | 237 in 5 s | 74 %, 249 / 308 W | 2.0 cores |
| RIFE quality, 1080p | 2 | RIFE quality | 288 in 5 s | 83 %, 252 / 313 W | 2.5 cores |
| NVIDIA Optical Flow medium, 720p, fixed (finding 8) | 3 | NVIDIA Optical Flow medium | 0 (none at all) | 17 %, 89 / 140 W | 0.4 cores |
| NVIDIA Optical Flow medium, 1080p, fixed | 3 | NVIDIA Optical Flow medium | 244 in 5 s | 11 %, 148 / 164 W | 0.4 cores |
| SVP, RVP's smoothing on | 1 to 3 | MVTools medium; SVP did not attach | | | |
| SVP, RVP's smoothing off | 3 | off; SVP attached at once | 0 | 36 %, 196 / 239 W | 1.6 cores |

- The background was measured from the second session on (`baseline`):
  11 % of the CPU's 64 threads with the owner's fuzzers in WSL paused
  (session 2), 26 % with them running (session 3); the GPU idle at 35 W.
  During the first session 12 of the owner's fuzzers ran in WSL (about 13
  threads).
- SVP attached to RVP's mpv through `mpvpipe` in both of its libmpv
  builds (SVP's log names mpv 0.41.0-1224-g6a262d11f and shinchiro's
  0.41.0-1115-gb2c255c13), but only with RVP's smoothing off: with
  RVP's filter in the chain SVP stayed at "waiting for the video info".
  With it off SVP took its "Алгоритм RIFE" profile and played at 59.94
  "[23.976 *5/2]" in RVP's own VapourSynth runtime (finding 9).
- RIFE at 1080p and NVIDIA Optical Flow medium at 1080p cannot keep 59.94
  fps on this card at 315 W; both do at 720p (finding 10).
- A fourth session (2026-10-20) checked the fixes on the final build with
  the clips looping for 40 s (`-Loop`): SVP attached with RVP's smoothing
  on (the status "Off (SVP is attached)", 59.94 fps, no frame dropped,
  RVP's filter never added); RIFE medium at 1080p stepped to RIFE fast at
  15.9 s and to MVTools medium at 30.0 s, which kept 59.94; NVIDIA
  Optical Flow at 1080p stepped from medium to fast at 15.6 s. MVTools at
  1080p ended at Blend: 3.4 s after the clip looped, mpv's rate estimate
  was still low and the watch took MVTools for an engine that did not run
  (finding 11).
- Pausing the owner's fuzzers for the second session (SIGSTOP for 175 s,
  with the owner's consent) made one of them, `guard_userdata` of
  `~/Linux_Storage_Spaces`, fail its own time check ("the checks took
  156.7s") and stop with a false crash artifact. The owner was told at
  once; host sessions no longer pause other work.

## Tests

- Unit tests:
  - `crates/app/src/smoothing.rs` (7): the filter watch (an engine that
    never raised the rate is left out, one that did is not after a loop,
    a seek restarts the wait, drops over 5 % lighten, slow playback is
    judged, a window with a loop is not); rates as fractions (NTSC rates
    exact), the plan following the owner's defaults and giving way
    (waiting for SVP too), the budget's steps (lighter, then Blend, then
    off), the dropped frames' steps (presets, then MVTools, then Blend; the
    5 % share; a seek leaves a window unjudged, slow playback does not),
    the scripts (the cache cleared, the rate made) and the filter entries,
    and the settings' checks;
  - `gpu_power.rs` (6): the governor's steps one at a time above the
    budget and back below its margin, with longer waits after a retake;
    no reading changes nothing; the busiest card counts; amdgpu's hwmon
    files; NVML left alone while NVIDIA's cards sleep;
  - `svp.rs`: RVP looks for its own VapourSynth runtime and never for
    SVP's; the smoothing waits for SVP in SVP mode until SVP 4 Manager is
    known not to run or the wait is over;
  - the shell's `gpu_power.rs` (2): the reader's lines, and this machine's
    cards read sensibly.
- `crates/app/tests/smoothing.rs` (3, headless libmpv with
  `fixture-square.mkv`):
  - Blend sets mpv's interpolation and gives the old `video-sync` back;
  - MVTools at twice the rate gives frames 1/48 s apart, and the square in
    the frames between is halfway (4 px of 8);
  - turning smoothing on and off during playback leaves `@svp`'s entry as
    it was, and RVP's script slot takes over from smoothing.

  These tests need a VapourSynth runtime with MVTools (`VSSCRIPT_PATH`)
  and say so when there is none.
- `rvp --selftest=smoothing` (`shell/qml/SelfTestSmoothing.qml`, 10
  checks): Ctrl+Shift+I on and off during playback, MVTools raising the
  rate (Blend without a runtime that has it), Blend and the presets from
  the menu's actions, and the settings page.
- `rvp --selftest=gpu` (`SelfTestGpu.qml`, 16 checks): after the scalers
  and the shaders, the budget's smoothing steps (lighter, Blend, off, and
  back; Blend alone goes straight to off).
- `rvp --selftest=playback` also logs the smoothing plan at the end; the
  host session's script reads it.
- `tools/windows-vm/m18-all.ps1`: both self-tests and the headless tests
  on the Windows VM.
- `tools/windows-vm/m18-host.ps1`: the host sessions. Each GPU mode runs in
  RVP's own window at the power limit and gives one `result` line: the
  plan, the rate reached, drops, the CPU's and the GPU's load, power, exit
  code and crash logs, after a `baseline` line of the background; mpv's
  log of each run stays beside it. It was tried on the VM with `-Vm`.

## Checks on the Windows VM

With the portable package and its own runtime (`C:\rvp-test\rvp-t\RVP`):

- `m18-all.ps1`: smoothing 10 of 10, gpu 16 of 16, headless 3 of 3;
- the VapourSynth slot (`--selftest=vpy`) and the functional self-test,
  with the library search restricted;
- a fake `libEGL.dll` on the PATH: goal N's build loaded it, this one does
  not;
- the Vulkan renderer (the VM has no Vulkan): RVP says that mpv's output
  could not start and plays on, and the fruc probe answers no with
  shinchiro's libmpv.

With RVP's libmpv in place of shinchiro's, on the VM:

- `m18-all.ps1`: 27 checks and the 3 headless tests pass;
- `m12-all.ps1`: 43 checks and the headless tests (14 and 10) pass;
- `m11-all.ps1`: 47 checks and the headless tests (8 and 11) pass. Its
  DVD and external-audio checks failed at first with both libmpv builds:
  the bundle had lost the `dvd` fixture folder, and they pass with it
  back;
- `m10-all.ps1`: 94 checks and the headless tests pass (with both builds;
  one first run's `second_device_follows` failure did not come back);
- RVP's start-up probe finds `fruc_vulkan` (`vf add` accepted);
- the dropped frames' steps: MVTools medium at 1080p on the VM's software
  rendering, over 40 s of a looping clip, stepped to MVTools fast at
  16.6 s and then to Blend.

In WSL under Xvfb, `--selftest=smoothing` and `--selftest=gpu` pass with
Blend (no MVTools there).

CI is green for every pushed commit of the goal but one: 8af11bb,
d38b9f5, a930288, f1a021a, aef5cad, 78c42a5 and fd531d8, the last
(pushed in one batch with ad76250 and fec5576; its `prune-artifacts` job
left only fd531d8's two packages). 690a502, which changed only
`tools/libmpv` and the documentation, failed one check of
`package-windows`: on the hosted runner the media controls showed Next
as disabled for the 10 s `m8-smtc.ps1` waits (`buttons_playing`), while
Next itself worked. The same code passed before (78c42a5) and after
(fd531d8), so the check is taken as flaky on the runner.

## RVP's libmpv (2026-10-19)

Built in WSL with `tools/libmpv/rootless.sh` (the owner's choice: no sudo,
the system's packages unchanged): an Ubuntu 24.04 base image in
`~/rvp-deps/mpv-root`, entered as root of a user namespace, with the
recipe's packages and meson installed there. The result is
`mpv-dev-x86_64-20261019-git-6a262d11f6.7z`, after about an hour on the
64 threads (the recipe's own LLVM included) and four restarts. These four
things had to change, all in `tools/libmpv/`:

1. Ubuntu needs libc++'s headers (`libc++-dev`, `libc++abi-dev`), not
   only the libraries the recipe's README lists. Without them clang fell
   back to libstdc++ and the link of LLVM's `llvm-profdata` failed
   (`R_X86_64_TPOFF32` against `std::__once_callable`, under the recipe's
   `-ftls-model=local-exec`).
2. The recipe applies its patches with `git am`, which needs a committer.
   The build root's `/root/.gitconfig` names one.
3. curl's master (since 8.23.0) calls OpenSSL 4.1's
   `ASN1_STRING_get_length`, which the recipe's pinned OpenSSL (4.1.0-dev
   of 2026-07-18) lacks. `curl-release.patch` holds curl at 8.22.0.
4. FFmpeg's master no longer has `--enable-libshaderc`: it compiles its
   Vulkan shaders while it builds, with a glslang or glslc it finds on the
   PATH. `build.sh` builds glslang 16.6.0 for the build host, and FFmpeg's
   configure took it (`GLSLC=glslang`) and enabled `fruc_vulkan` with the
   other Vulkan filters.

The DLL exports the same 54 functions as shinchiro's libmpv of
2026-10-10, imports the same system DLLs, and has the filter ("Frame rate
up-conversion using the Vulkan NV optical flow extension"). Its parts:

| Part | Commit |
|------|--------|
| mpv | 6a262d11 (2026-10-18) |
| FFmpeg | fd9a8f25a (2026-10-19) |
| libplacebo | ccb74c6 (2026-10-18) |
| Vulkan headers | acbb939 (2026-10-16) |
| Vulkan loader | 52fc9f9 (2026-10-19) |
| shaderc and glslang for libplacebo | ba3e587, 5cf91e7 |
| libass | f61db56 (2026-09-17) |
| curl | 8.22.0 (01346829) |
| OpenSSL | d8bf6cdd4 (the recipe's pin) |

## Findings

1. **MVTools gave black frames after every start and seek.** While a
   script loads, mpv answers the frames it asks for with black dummy
   frames, and `mv.Analyse` asks for frame 0 then. VapourSynth's cache
   handed those frames out after every start and seek. RVP's scripts now
   end with `core.clear_cache()`; on the VM the first frame's mean went
   from 0 to 131.8.
2. **Diagnostics broke JSON parsing.** Right after a filter goes, mpv's
   rates are NaN, and the diagnostics' and the status' JSON carried them.
   Non-finite numbers are now null.
3. **SVP's libraries reached RVP through the PATH**, because SVP's installer
   puts its mpv folder there. RVP now loads libraries from its own folder,
   System32 and its runtime only (`SetDefaultDllDirectories` and
   `AddDllDirectory`, before anything else loads).
4. **RIFE's plugin crashes every process at exit** (ncnn's destructors call
   the Vulkan driver). After RIFE has loaded, RVP ends its process without
   unloading libraries; settings and state are written before that.
5. **fruc_vulkan needs a SPIR-V compiler in FFmpeg's build**: its
   configure dependencies are `vulkan vulkan_1_4 spirv_compiler`, and
   FFmpeg's master compiles its shaders while it builds, with a glslang or
   glslc on the build host's PATH (its libshaderc option is gone).
   shinchiro's build host has neither, so the filter is missing from their
   libmpv. Andarwinux's FFmpeg has it, but they publish no libmpv.
6. **mpv gives a lavfi graph a hardware device only through
   `hwdec_interop`**, taken from the video output's interops. That is why
   the filter needs the Vulkan renderer.
7. **MVTools could not keep up at 1080p on the host's 64 threads**: RVP's
   filter asked for 4 frames at once (`concurrent-frames=4`), and the
   output dropped about 48 of 60 frames a second. The study's bench and
   part B's script slot had 8 or mpv's default (one per logical CPU).
   MVTools now gets mpv's default: 0 frames dropped, 4.5 to 4.8 cores.
8. **FFmpeg's `hwupload` wants its device when the graph is parsed**
   (`hwupload_init`), while mpv's lavfi filter hands the `hwdec_interop`
   device to the graph's filters only after parsing, just before
   `avfilter_graph_config`: "hwupload: A hardware device reference is
   required to upload frames to", and the engine gave way to MVTools. RVP
   now puts `format=fmt=nv12` and `format=fmt=vulkan` before
   `fruc_vulkan`, so mpv's own conversion uploads the frames to the
   output's Vulkan device (it loads the output's Vulkan interop on
   demand), and the graph holds `fruc_vulkan` alone.
9. **SVP does not attach while RVP's smoothing filter plays**: SVP 4
   connected and waited for "the video info" forever, with both libmpv
   builds, and attached at once with RVP's smoothing off. RVP's smoothing
   now waits SVP's 8 s (`svp::ATTACH_WAIT`) after a file starts while SVP
   4 Manager runs (checked with `tasklist` on a thread), and starts when
   SVP has not attached by then.
10. **RIFE and NVIDIA Optical Flow medium do not reach 1080p at 60 fps** on
    the RTX 30-series card at its 315 W test limit (RIFE quality peaks at 313 W);
    at 720p both do with no frames dropped. Smoothing now steps down by
    itself when the output drops more than 5 % of the filter's frames
    (the owner's choice); on this card RIFE at 1080p ends at MVTools.

11. **A seek or a loop could leave a working engine out.** Right after
    one, mpv's rate estimate starts again from the source's rate, and the
    watch, which looked at it every second, took MVTools for an engine
    that never ran. The watch (`FilterWatch`) now judges an engine only
    after 6 s of steady playback, starts that wait again after a seek, a
    loop or a pause, and leaves out no engine that has raised the rate in
    the file; dropped frames are counted only over windows without a
    seek. Its unit test covers these cases; on the VM a looping clip
    stepped from MVTools medium to fast and then to Blend on its drops
    alone (the watch's ticks printed for the check).

## Differences from SVP

- **Engines.** SVP uses its own svpflow libraries, with GPU work through
  OpenCL and many tunables (block sizes, masks, artefact treatment, scene
  changes, profiles per video). RVP has MVTools (CPU), RIFE (Vulkan),
  NVIDIA Optical Flow (Vulkan, RTX 30 and later) and mpv's Blend, with
  three presets each.
- **Profiles.** SVP picks a profile by video size and rate. RVP has one
  rule: motion compensation up to a height (1080p by default) and Blend
  above.
- **Black bars and lights.** SVP's black-bar handling and its "lights"
  around the picture are not in RVP. NVIDIA Optical Flow flickers at
  letterbox edges, where MVTools and RIFE do not.
- **Never together.** RVP never runs its smoothing together with SVP's.
  While SVP 4 Manager runs, RVP's smoothing waits 8 s after a file starts
  so that SVP can attach; once SVP attaches (through `mpvpipe`), RVP's
  smoothing stays off until SVP's filter goes, and RVP never changes
  `@svp`. SVP itself does not attach to a video RVP's filter plays.
- **Load.** SVP keeps its profile's settings; RVP steps its smoothing down
  when the output drops frames, and when the card goes above the GPU
  power budget.
- **Process.** SVP's filter runs in RVP's VapourSynth runtime (a process
  has one): SVP's script loads SVP's plugins by their own paths. RVP uses
  none of SVP's files itself.

## Linux items for goal S

- **The VapourSynth runtime.** Linux packages carry none yet. RVP uses
  `vapoursynth/` next to the binary or the system's
  `libvapoursynth-script.so`, and MVTools, RIFE's `.so` and their models
  would have to come with the packages (AppImage, .deb).
- **NVIDIA Optical Flow.** It needs the kit's libmpv with FFmpeg's master
  (Ubuntu's FFmpeg 6.1 predates `fruc_vulkan`) and a Vulkan output.
  mpv's `gpu-next` in a child window works on X11 only, as for goal N's
  Direct3D 11 renderer.
- **Blend's display rate.** With OpenGL in Qt's window this comes from
  QScreen; check it on GNOME Wayland (the owner's Ubuntu boot).
- **The GPU power budget.** On the owner's Ubuntu boot: NVML through
  `libnvidia-ml.so.1`, amdgpu's hwmon (no AMD card here), and the
  card's runtime power status left alone.
- **RIFE's exit.** Check whether its exit crash exists under Linux's
  NVIDIA driver; `process::leave` ends the process early on Windows only.
- **Speed on Linux.** MVTools and RIFE on the 32-core processor and the RTX
  card under Linux (the study measured Windows only), and the dropped
  frames' steps there.
- **NVIDIA Optical Flow's upload on Linux.** mpv's conversion uploads the
  frames to the output's Vulkan device (finding 8): check it with the
  kit's libmpv and a Vulkan output on X11.
- **Waiting for SVP on Linux.** `svp::manager_running` reads `/proc` for
  SVP's Linux manager; check that SVP for Linux behaves as on Windows
  (it does not attach while RVP's filter plays).

## What to try

In `C:\Users\<user>\rvp-test\rvp-t` (portable: its settings stay beside
`rvp.exe`; this section is finished when the folder is there):

1. **Ctrl+Shift+I** on a 24 or 25 fps film: MVTools medium raises it to
   the display's 59.94 (the notice and the status say so), and
   Ctrl+Shift+I again turns it off. Above 1080p it is Blend.
2. **Video > Motion Smoothing**: Blend, MVTools, RIFE (on the GPU; on this
   card real time up to 720p; at 1080p it steps down by itself, to RIFE
   fast and then MVTools, each step about 12 s after the last, with a
   notice), and Fast/Medium/Quality.
3. **NVIDIA Optical Flow**: Preferences > Video, Output "Vulkan window",
   start RVP again, then Video > Motion Smoothing > NVIDIA Optical Flow
   (720p plays with no frames dropped; 1080p steps down to fast, then
   MVTools). The settings page says whether this machine has it.
4. **Preferences > Video > Motion Smoothing**: the engine, preset, rate,
   the height up to which motion compensation runs, what runs now and
   which engines this machine has.
5. **Power budget** (Preferences > Video): with RIFE at 1080p, a lower
   budget lightens smoothing first (RIFE gives way to MVTools, then Blend,
   then off) and gives it back after a minute below.
6. **With SVP running**: SVP attaches as before, also with RVP's smoothing
   on (the status says "Off (waiting for SVP)" for up to 8 s, then "Off
   (SVP is attached)"); quit SVP 4 Manager and RVP's own smoothing takes
   over with the next file.

## Left

Nothing of the goal's own work. The owner may want one more host run
of MVTools at 1080p over a looping clip to see finding 11's fix on the
card (its logic is covered by the unit test and the VM).
