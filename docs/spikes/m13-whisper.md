# M13 study: subtitles from audio and PotPlayer's engines

Goal O asked for PotPlayer's Create Subtitles from Audio, ahead of time and
in real time, offline with whisper.cpp (CPU, Vulkan or CUDA, models the
user downloads), and for a study that decides on PotPlayer's other
engines. This is that study (2026-10-21) and the design RVP took.

## PotPlayer's engines

PotPlayer's Whisper AI engine menu lists seven engines; its real-time menu
lists the four whisper.cpp ones. PotPlayer downloads each into its engine
folder ("Open engine folder") and runs it as a program of its own.

| Engine | What it is | Licence | State (2026-10) | RVP |
|---|---|---|---|---|
| whisper.cpp CPU | ggml's CPU backend | MIT | v1.9.5, 2026-10-06 | yes: RVP's build, `-ng` |
| whisper.cpp BLAS | the CPU build with OpenBLAS | MIT | official Windows builds | yes: the user's copy of the official build |
| whisper.cpp Vulkan | ggml's Vulkan backend: NVIDIA, AMD and Intel GPUs | MIT | no official Windows build | yes: RVP's build |
| whisper.cpp CUDA | ggml's CUDA backend: NVIDIA | MIT; CUDA's runtime is NVIDIA's | official builds of 285 to 685 MB | yes: the user's copy of the official build |
| Whisper Const-me | DirectCompute (Direct3D 11), Windows only | MPL-2.0 | last release 1.12.0, 2023-07-22 | no |
| Whisper-Faster | Purfview's standalone faster-whisper (CTranslate2) | none given | r192.3, 2023 | no |
| Faster-Whisper-XXL | the same, larger, PotPlayer's "Recommended" | none given | r245.4, 1.4 GB | no |

Why the last three stay out:

- Const-me's Whisper has had no release since 2023, runs only on Windows
  through DirectCompute and reads only the older whisper.cpp model files
  (no large-v3 or large-v3-turbo). whisper.cpp's Vulkan backend reaches
  the same GPUs, and more, with current models.
- Purfview's builds come without a licence, so RVP can neither ship them
  nor rely on them, and are bundles of 0.1 to 1.4 GB for NVIDIA's CUDA or
  the CPU. faster-whisper fetches its models from the internet unless
  they are put in place by hand, which goes against the goal's offline
  rule. whisper.cpp's CUDA build runs the same Whisper models offline.

Should the owner want Faster-Whisper-XXL for its voice detection and
timing, RVP could run a copy the user installs, as PotPlayer does; that
needs the owner's decision and a host session with it.

## How RVP runs whisper.cpp

- **A program, not a library.** RVP starts whisper.cpp's `whisper-cli`
  with the model, the language (`auto` or a code), `-osrt` and `-pp`
  (progress), and reads its progress and the language it detects from its
  output. A program keeps a crash or a hung GPU driver out of RVP's
  process, needs no `unsafe` (AGENTS.md), and works the same for RVP's
  build and the official ones. It starts without a console; the whole file's job runs at
  normal priority on all cores but one (the user waits for it), real time
  below normal priority (`nice` on Linux) on half of them, so playback
  keeps the CPU first. On the Windows VM (2 cores) RVP's software
  rendering in session 0 took nearly all the CPU and left a job below
  normal priority there 25 times slower than alone.
- **One build for the CPU and Vulkan.** `tools/whisper/build.sh` builds
  whisper.cpp v1.9.5 with `GGML_BACKEND_DL`: the CPU backend for each
  instruction set (from SSE 4.2 to AVX-512) and the Vulkan backend are
  DLLs ggml loads at run time; without a Vulkan driver (the Windows VM)
  it runs on the CPU. `-ng` keeps a job on the CPU. The build uses the
  toolchain of RVP's libmpv build (`tools/libmpv`) and a `glslc` it builds
  for the build host; the result needs only Windows' own DLLs (UCRT and
  the driver's `vulkan-1.dll`), 59 MB, and goes into the package as
  `whisper/`.
- **Models** are `ggml-<model>.bin` files from
  huggingface.co/ggerganov/whisper.cpp that the user downloads into
  `whisper-models` of RVP's settings folder; RVP downloads nothing. The
  menus mark models not downloaded and engines not installed.
- **Audio.** A libmpv core of its own (`rvp_engine::encode`, M12's Record
  Video core) writes the playing audio track as a 16 kHz mono WAV file,
  with a filter for speech (high-pass 80 Hz, low-pass 7.6 kHz, dynaudnorm)
  when the user asks for it.
- **Ahead of time** the whole file becomes an SRT file next to the video,
  `<name>.<language>.srt` (mpv's own subtitle search finds it the next
  time), or `<name>.<language>.whisper.srt` when that name is taken, or
  in RVP's state folder when the video's folder takes no file or the user
  chose so. "Start" starts and stops it; the automatic modes start it
  when a file starts.
- **In real time** RVP makes 30 s pieces from the playback position on,
  two pieces ahead, and again from where a seek lands; their subtitles
  are one track (`subtitles/realtime.srt` in the state folder) that is
  given to mpv again as pieces arrive. For network sources the pieces come
  from the same address (seekable streams); a live stream cannot be read
  ahead.
- **GPU budget (D17).** A job on the GPU is one of the budget's steps,
  right after the smoothing's: above the budget the next pieces and jobs
  run on the CPU. A whisper.cpp run already going finishes as it started.

## Measured

- WSL (this machine, CPU): the tiny model on a 4 s espeak-ng clip
  ("Hello world. This is a test of subtitles made from speech.") takes
  0.7 s and gives "Hello world, this is a test on some titles made from
  speech."
- Windows VM (2 cores, no GPU): the same with RVP's build takes 4.5 s; the
  CPU backend loads `ggml-cpu-haswell.dll`.
- The host's GPU: not measured in this session (it needs the owner's
  permission and the power limit, AGENTS.md).

## Linux (goal S)

RVP finds `whisper-cli` in `whisper/<engine>` of its settings folder or on
the PATH; Linux packages get no build of their own yet. The tests in WSL
use a CPU build of the same tag.
