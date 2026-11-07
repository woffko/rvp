# M18 study: RIFE through TensorRT, as SVP does

The owner asked on 2026-11-07 for a study of running RIFE through NVIDIA's
TensorRT, as SVP 4 does, before any implementation. RVP's RIFE (ncnn on
Vulkan, the VapourSynth plugin in `vapoursynth/rife`) cannot keep 1080p
at 59.94 on the host's RTX 30-series card (goal T, and the calibration of
2026-11-07: 103 frames a second at 720p, nothing larger kept). SVP's RIFE
keeps it through vs-mlrt's TensorRT plugin.

## Measured on the host (2026-11-07)

`crates/engine/examples/measure_vf.rs` played a generated 23.976 picture
through each script, without a window or sound, at up to 8 times real
time. It used RVP's own VapourSynth runtime and mpv's filter options as
RVP sets them (8 frames buffered and in flight). The card was at
`gpu-limit.ps1`'s 315 W (90 % of its default 350 W), and nothing else
loaded the machine. Output is frames a second at 59.94; 72 is what a
60 Hz file needs with RVP's fifth to spare.

| RIFE v4.6 | 720p | 1080p | 1440p |
|---|---|---|---|
| ncnn on Vulkan (RVP today, 4 GPU threads) | 126 | 53 | 27 |
| TensorRT 10.16 (vs-mlrt v15.16 `vstrt`, fp16, 2 streams) | 316 | 160 | 99 |
| TensorRT-RTX 1.4 (`vstrt_rtx`) | did not run | | |

- TensorRT is 2.5 to 3.7 times as fast. 1080p keeps the rate with more
  than twice what it needs, and 1440p keeps it too. 2160p was not tried:
  RIFE there took the host's GPU down on 2026-10-18.
- One engine covers 128x128 to 2560x1440, the shape range SVP uses (fp16,
  optimised for 1920x1088). It took 122 s to build the first time, on
  the card itself. It is 12 MB and is cached by vs-mlrt in its
  `engine_folder`.
- TensorRT-RTX's filter did not raise the rate, so mpv dropped it. The
  measuring core did not keep mpv's errors then; it does now, and the
  reason is still to find.
- Pictures go to the model as RGBH, padded to multiples of 32 (1080 to
  1088), and `vsmlrt.RIFE(multi=Fraction(5, 2), video_player=True)` turns
  23.976 into 59.94.

## The files

vs-mlrt v15.16 (AmusementClub/vs-mlrt, 2026-03-26):

- `VSTRT-Windows-x64` is the plugin, 0.5 MB, and `VSTRT-RTX` is 0.2 MB.
  Both are GPL-3.0.
- `vsmlrt-cuda` (2.5 GB packed, 3.7 GB unpacked) holds NVIDIA's runtime:
  TensorRT 10.16, TensorRT-RTX 1.4, CUDA 13.2, cuDNN 9, cuBLAS and cuFFT.
  - TensorRT alone on one architecture is about 610 MB: `nvinfer_10.dll`
    358 MB, the builder resource of the card's architecture (`sm86` for
    Ampere, 168 MB), the plugin, the ONNX parser and the CUDA runtime.
    vs-mlrt ships the builder resources of every architecture (1.8 GB).
  - TensorRT-RTX is `tensorrt_rtx_1_4.dll`, 191 MB.
  - CUDA 13 needs NVIDIA's driver 580 or later (the host has 617.42).
- The models are ONNX: RIFE v4.0 to v4.10 in the release `model-20220923`
  (`rife_v8.7z`, 195 MB, with no checksum published; its 7z CRCs pass).
  Practical-RIFE's models are MIT.

SVP 4 ships TensorRT 10.8 and CUDA 12 (2.6 GB in
`C:\Program Files (x86)\SVP 4\rife\vsmlrt-cuda`), and caches an engine
built for the card next to its models.

## NVIDIA's terms and RVP's licence

Read by a research agent on 2026-11-07 (sources below; facts, not legal
advice).

- TensorRT's licence makes "the runtime files .so and .dll" distributable
  in an application that adds "material additional functionality", for
  use by that application only. It forbids using the SDK "in a way that
  would cause it to become subject to an open source software license".
- The CUDA EULA's Attachment A lists `cudart`, `nvrtc` and
  `nvrtc-builtins` as distributable.
- vs-mlrt is GPL-3.0 without exceptions, and RVP is GPL-3.0-or-later. The
  FSF's system library exception does not cover libraries shipped with
  the program, so RVP cannot ship NVIDIA's runtime in its packages.
- What other GPL projects do:
  - chaiNNer (GPL-3.0) and REAL-Video-Enhancer (AGPL-3.0) download
    TensorRT from NVIDIA when the user asks for it.
  - vs-mlrt publishes the runtime as a separate download.
  - mpv_PlayKit has an optional add-on.
  - OBS loads NVIDIA's SDK, which the user installs separately.
  - GPLv3 section 2 leaves what a project does not convey unconditional.

## The route

1. Keep RIFE on ncnn as the default for every card.
2. Add "RIFE (TensorRT)" for NVIDIA cards, Turing and later:
   - RVP ships vs-mlrt's GPL plugin and its own script.
   - The Motion Smoothing page downloads NVIDIA's files from NVIDIA when
     the user asks, checked by SHA-256, into the configuration folder:
     about 610 MB for the card's architecture.
   - RVP never puts them in a package.
3. Build the engine in the background before the engine is offered, for
   example as part of the calibration (two minutes on the host). The
   script builds it while it loads, longer than the drop watch waits, and
   MVTools plays meanwhile.
4. Find why TensorRT-RTX did not run. It is a fifth of the download and
   builds faster, but needs the ONNX model in fp16 from TensorRT 11 on.
5. `shell/src/process.rs` limits the library search to RVP's folder,
   System32 and its VapourSynth runtime. `vstrt` loads its libraries from
   `vsmlrt-cuda` next to it, so that folder has to be allowed.
6. Linux: vs-mlrt builds `vstrt` against NVIDIA's Ubuntu packages, but
   publishes no Linux release, so RVP would build it. TensorRT comes from
   NVIDIA's apt repository or its pip wheels.

## In RVP (2026-11-07)

The route is implemented for Windows: the engine "RIFE (TensorRT)"
(`crates/app/src/tensorrt.rs`), NVIDIA's files in `<state>/tensorrt`
rather than the configuration folder (local, not roaming). On the host,
in a test copy of RVP with the owner's settings at 315 W:

- The installation took 10 s from a local copy of the archives, with the
  SHA-256 checked and 13 files kept for `sm86`.
- The calibration built the engine (about two minutes) and measured the
  following, in frames a second at 59.94:

  | RIFE (TensorRT) | 480p | 720p | 1080p | 1440p |
  |---|---|---|---|---|
  | Fast (1 stream) | 250 | 138 | 70 | |
  | Medium (2 streams) | 412 | 247 | 123 | 77 |
  | Quality (3 streams) | 455 | 268 | 134 | 86 |

- The 1080p23.976 clip then started at once with RIFE (TensorRT) Quality.
  After the first 3 s (88 frames, as the original gave way to the
  filter) it dropped 19 frames over 37 s, where RIFE on ncnn drops 40 to
  50 a second.

## Sources

- docs.nvidia.com/deeplearning/tensorrt/latest/reference/sla.html
- docs.nvidia.com/cuda/eula/index.html
- docs.nvidia.com/deeplearning/tensorrt-rtx/latest/reference/sla.html
- github.com/AmusementClub/vs-mlrt: LICENSE, releases v15.16 and
  model-20220923, `scripts/vsmlrt.py`, workflows
  `windows-cuda-dependency.yml`, `windows-trt.yml`, `linux-trt.yml`
- www.gnu.org/licenses/gpl-faq.html (SystemLibraryException,
  GPLIncompatibleLibs, MereAggregation)
- www.svp-team.com/wiki/RIFE_AI_interpolation
- github.com/chaiNNer-org/chaiNNer (its TensorRT package),
  github.com/TNTwise/REAL-Video-Enhancer (`DownloadDeps.py`)
