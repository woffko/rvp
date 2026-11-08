# RVP agent instructions

These rules apply to every change in this repository, in addition to the
user's global instructions.

## Project

RVP (Rust Video Player) is a PotPlayer-style media player for Linux and
Windows built on libmpv, with its own engine for PotPlayer `.dsf` skins and
SVP 4 support. The plan, decisions and `/goal` texts are in `docs/PLAN.md`;
each goal writes `docs/goals/goal-<letter>-status.md`.

Since 2026-10-16 the work is PotPlayer parity, Windows first (D14, goals L
to S): build and accept each part on Windows; keep Linux building and its
headless tests green, and list Linux-only parts for M17 (goal S). By the
end of each parity goal, every PotPlayer item of its area that RVP lacks
keeps a reason in `crates/app/src/coverage.rs` that says why libmpv,
FFmpeg, Qt or the platform cannot do it.
From goal S on (owner, 2026-11-01, D19) "Windows first" ends: every
change, a goal's parts and the fixes between goals, is built and checked
on both Windows (the test VM) and Linux (Xvfb in WSL, the Ubuntu VM)
before it counts as done.
Goal T (M18, D16, added 2026-10-18) follows goal N: RVP's own motion
smoothing for users without SVP, after a study of the engines.

## Shared rules (PLAN.md section 10)

- Repository content is in English: code comments, documentation, commit
  messages, pull requests and issues. Conversation with the owner may be in
  Russian.
- Never commit PotPlayer or SVP files, renders or captures made from them, or
  third-party binaries. Read them from their installed locations at run time.
  Local-only material lives in `~/rvp-reference` (screenshots,
  renders, geometry snapshots of third-party skins, preview content).
- Never change PotPlayer, SVP or system settings on the Windows host.
  Read-only access (files, registry queries) is fine. The one exception
  is the GPU power limit for GPU test runs (owner, 2026-10-19, D17):
  `tools/windows-vm/gpu-limit.ps1` sets 90 % of the card's default
  through one UAC prompt and puts back the limit it found when the run
  ends; like every host run it needs the owner's permission first. GPU
  runs on the host go through it (`m18-study.ps1 -Gpu`, `m18-host.ps1`,
  or `gpu-limit.ps1 -Run COMMAND`), with `nvidia-smi` checked before and
  after each run: on a lost GPU stop everything and tell the owner.
- Anything that starts a program or captures the screen on the Windows host
  desktop needs advance notice and the owner's explicit permission, every
  time. Say what will run, which windows will appear and for how long, then
  wait for a yes.
- The repository stays private. No tags, releases or published packages.

## Code rules

- Rust edition 2024, toolchain pinned in `rust-toolchain.toml`. `Cargo.lock`
  is committed. Dependencies use exact versions (`=x.y.z`) in the workspace
  manifest and need a reason to be added.
- `unsafe` is forbidden in every crate except `shell/` (owner decision
  2026-10-10): cxx-qt's generated bridge code needs it. There it is allowed
  only on the cxx-qt bridge modules and at the few FFI points (such as the
  `QPainter` pointer passed to `paint`), each with a `SAFETY:` comment.
  Clippy runs with `all` and `pedantic` denied; a local `#[allow]` needs a
  comment that justifies it.
- `vendor/libmpv2` is libmpv2 6.0.0 with one fix (`vendor/libmpv2/RVP-PATCH.md`),
  used through `[patch.crates-io]`. Building needs libmpv (`MPV_LIB_DIR` for
  the linker and the loader path at run time) and, for `shell/`, Qt 6.10
  (`QMAKE`).
- Results must be deterministic: no hash-ordered containers where order is
  observable (use `BTreeMap`/`BTreeSet`).
- `cargo xtask ci` must pass before every commit: fmt, clippy with warnings
  denied, all tests, `cargo deny check`.

## Skin engine notes

- `crates/skin` documents the PotPlayer behaviour it reproduces at the top of
  `markup.rs`, `layout.rs`, `paint.rs` and `playlist.rs`. When a rule changes,
  update the module comment and the synthetic tests in
  `crates/skin/tests/synthetic.rs`.
- Synthetic fixtures generate their atlases in code. Keep them free of
  third-party data. `RVP_BLESS=1` rewrites geometry snapshots.
- Tests that need BlackSpace read `RVP_TEST_SKIN` and write to
  `RVP_REFERENCE_DIR`; they must pass (with a note) when the skin is absent.
- `cargo run --release -p rvp-skin --example compare_reference -- RENDER REF OUT`
  aligns a render with a scaled screenshot and writes side-by-side crops.

## Own look (crates/look)

- RVP's logo and its default skin RVP Default are vector sources in
  `crates/look/art` (SVG, the skin's markup and `sprites.xml`); M7's
  originality rules apply: draw every asset for RVP, take nothing from
  PotPlayer's skins, and list each new asset in `art/SOURCES.md`.
- Built files are not committed: `shell/build.rs` draws the icons, logo
  images, Windows icon resource (with `rvp.exe`'s version resource,
  `brand::version_info`) and `RVP Default.dsf` into its `OUT_DIR`;
  `cargo xtask look DIR` writes them for packages and reviews.
- `crates/look/src/svg.rs` renders only the SVG subset its module comment
  lists and rejects the rest; check new art with
  `cargo run --release -p rvp-look --example render_svg -- IN.svg OUT.png SIZE`.
- RVP Default's geometry snapshots and golden renders are in
  `crates/look/tests` (no fonts, so no text); `RVP_BLESS=1 cargo test -p
  rvp-look --test default_skin` rewrites them after a deliberate change,
  and `RVP_BLESS=1 cargo test -p rvp-app --test coverage` rewrites
  `docs/coverage/rvp-default.md`.
- With no skin set (or `--skin=` with no path) RVP shows RVP Default. In
  other skins RVP draws its logo on title-bar `CMD_POPUPMENU_ETC` buttons
  (D10, `Skin::replace_sprite`); RVP Default marks its own `RvpLogo='1'`.

## Shell notes

- `rvp --selftest` (also `=playback`, `=resume`, `=upright`, `=svp`,
  `=vpy`, `=speed`, `=gpu` and `=smoothing`, see `shell/qml/Main.qml`) is the functional check for the VMs,
  the kit and the host. It drives RVP through its own input handlers,
  never through the window system, and prints `check`, `metric` and
  `status` lines.
- Keep the GUI thread free during playback: no blocking mpv calls in loops
  (`Player::diagnostics` is for rare reads), no disk flushes (use
  `storage::Writer`), and chrome updates through `Player::damaged` only.
- `shell/qml/Main.qml` imports only Qt Quick: Qt Quick Controls and Dialogs
  live in files loaded on first use, which keeps the window's start fast.
- `shell/build.rs` deletes cxx-qt-build's stale `plugin.qmltypes` before each
  build; if an incremental build still crashes at start after a bridge
  change, run `cargo clean -p rvp-shell`.
- Windows host runs use `C:\Users\<user>\rvp-test\run-m2.cmd`, which keeps
  RVP's settings and state in that folder. Every run needs the owner's
  permission first.
- RVP's albums are the playlist (`crates/app/src/playlist.rs`); mpv plays
  one entry at a time, so never use mpv's playlist for Next, Previous or the
  end of a file. Undoable changes go through `Library::edit`, the rest
  (selection, scrolling, playback marks) through `Library::update`.
- A running `rvp` takes the files of the next one (`rvp_app::instance`, a
  `QLocalServer` channel `rvp-<user>`). The self-test neither forwards nor
  listens unless `--instance=NAME` names a separate channel, so test runs
  never reach the owner's player.
- The Windows test VM's checks are `tools/windows-vm/m3-all.ps1`,
  `m4-all.ps1`, `m6-all.ps1`, `m7-all.ps1` (RVP Default with no skin
  set, then the test copy of BlackSpace; copy them and `run.ps1` to
  `C:\rvp-test` with the bundle and `testdata/media/fixture-marker.*`),
  `m8-all.ps1` (the packages, see below), `m10-all.ps1` (M10's
  self-test steps, the awake request, the recording's file, and
  `tests/audio_chain.rs` and `tests/speakers.rs` built with `cargo xwin
  test --no-run` beside the bundle's libmpv: Windows' FFmpeg) and
  `m11-all.ps1` (`--selftest=m11` with `fixture-chapters.*` and the `dvd`
  folder beside the bundle, the power dry-run file, the Recycle Bin, and
  `tests/thumbnails.rs` and `tests/m11.rs` run with `RVP_TESTDATA` at
  the bundle).
- Video filters: RVP adds and removes only its own, by label (`@rvp*`,
  `vf pre`), and never touches SVP's `@svp` (goal H's rule 4). Like the
  audio chain's, a `vf` change in the last `NEAR_END` of a file can leave
  mpv stuck before the end (libmpv 0.37: the time stops, `eof-reached`
  never comes), so `App::apply_video_filters` lets it wait for the next
  file or for playback to leave the end; test clips for filters need
  room (M12's fixtures are 12 s).
- Audio filters: RVP's part of mpv's `af` is built at once by
  `crates/app/src/audio/chain.rs` in one order (downmix, channel, preamp,
  equalizer, effects, the user's own filters, normalizer, channel gains,
  `@rvptempo`, AC3 encoding) and set as one value
  (`App::apply_audio_chain`); every change goes through
  `App::audio_chain_changed`. Graphs are `lavfi` entries quoted with mpv's
  `%N%` prefix (`chain::lavfi`). A chain change or seek at the very end of
  a file leaves mpv's `eof-reached` false and the album stops, so in the
  last `NEAR_END` (1.5 s) changes wait for the next file
  (`App::defer_chain_change`); otherwise `App::resync_audio` plays on from
  the current position. While a file loads, the old file's end is ignored
  (`Tracking::Loading`), or the album moves on twice.
- FFmpeg differs between Linux (6.1 in CI and the sysroot) and Windows
  (shinchiro's, newer): use options every version has. `afir` normalizes
  its response itself (6.1 has no `irnorm`), so Freeverb's response is
  written at a level every version leaves alone (`audio/dsp.rs`);
  `headphone` lowers its output by 3 dB per input channel. mpv refuses
  `audio-format=no` (set only non-default output options).
- The effects are fits to PotPlayer's, measured in goal L's host session
  (`docs/audits/audio.md`, "M10"; re-run `tools/audio-audit/m10.py` after
  changing one). Freeverb's and Crystality's echo are responses RVP
  computes (`audio/dsp.rs`) into the state folder, named by their
  parameters: a change to how one is computed needs a new file name.
  Audio Stretching puts rubberband (`pitch-scale`) in `@rvptempo`'s place.
- The second audio device (`crates/app/src/audio/second.rs`) is a second
  core that follows the main one; it is started after the main core (so
  ffmpeg's log stays with the main core) and its chain has
  `Normalizer::follower_entry`. The normalizer's thread holds a weak
  handle of it: `Control::follow(None)` before the core is dropped, or
  mpv waits for that handle and the window's thread hangs.
- The normalizer is TomSteady (D13, `crates/app/src/tomsteady.rs`, its
  rules checked against S5's PotPlayer recordings by
  `tools/s5/tomsteady_fit.py`). Its chain prints each 2048-sample block's
  peak to mpv's log; `normalizer::Control` (a weak client on a thread of
  its own) reads it and sends the gain back with `af-command`. libmpv gives
  ffmpeg's log to the first player core of a process only: keep one core
  per process, and give tests that need the log a test binary of their
  own (`tests/normalizer_live.rs`).
- Windows' audio in `shell/`: the master volume (`system_volume.rs`,
  Core Audio) and recording RVP's own sound (`recorder.rs`, process
  loopback). `ActivateAudioInterfaceAsync` frees the activation blob it
  is given: allocate it with `CoTaskMemAlloc` (a stack blob corrupted the
  heap). The display stays on through `awake.rs`
  (`SetThreadExecutionState`); in the VM's session 0 `powercfg /requests`
  lists that request under SYSTEM only.
- Silent self-test in WSL (no window on the desktop, no sound): Xvfb
  (`xvfb-run`), `QT_QPA_PLATFORM=xcb` with `WAYLAND_DISPLAY` unset, the xcb
  libraries unpacked into `~/rvp-deps/sysroot`, `--mpv-options=ao=null`
  (the second device's core gets the same options) and `RVP_CONFIG_DIR`
  and `RVP_STATE_DIR` in a temporary folder.
- The VapourSynth runtime is chosen before any thread starts
  (`point_at_vapoursynth` sets `VSSCRIPT_PATH` through `rvp_set_env`); do
  not set environment variables later or from Rust (`set_var` is unsafe).
- The Direct3D 11 renderer (M12, D4, `render.renderer`, taken at start):
  mpv's `gpu-next` draws into a disabled native child window that its own
  thread owns (`shell/cpp/video_window.cpp`; the GUI thread waits on mpv,
  mpv's threads message their window's parent). QML places it from
  `videoWindowPlace` and cuts holes for layers over the video; dialogs
  over it need `popupType: Tools.native_video ? Popup.Window : Popup.Item`.
  Window geometry goes through `setWindowGeometry` (one `setGeometry`:
  Windows reports changes late, so separate x/width writes undo each
  other). The VM's session 0 makes no swapchain: the picture is checked
  on the host.
- The GPU power budget (D17, `crates/app/src/gpu_power.rs`): the cards
  are read while a file plays, on a thread of the shell's
  (`shell/src/gpu_power.rs`; `shell/cpp/gpu_power.cpp` loads NVML from
  System32 only and reads AMD's cards on Windows through D3DKMT's adapter
  counter: AMD's ADLX SDK licence forbids use in GPL software) and the
  readings reach `App::gpu_tick` through a channel; the window's thread
  never calls them (a hung card can hold a call up). On Linux a card in
  runtime suspend is not read (reading would wake it).
  Steps change only what mpv gets (`App::effective_scaler`,
  `processor_runs`, `shader_files`, `video_filters`), never the
  settings. A new GPU-heavy feature of RVP's becomes a step of
  `Step::ALL`. `rvp --selftest=gpu` gives its own readings
  (`Player.feed_gpu_power`); AMD's readers are untested on a card.
- M18's motion smoothing (D18, `crates/app/src/smoothing.rs`): a plan per
  file (off by default; MVTools medium up to 1080p, Blend above, at the
  display's rate; off while `@svp` is in the chain or RVP's own script slot
  runs, and, while SVP 4 Manager runs, for SVP's 8 s after a file starts:
  SVP does not attach to a video RVP's filter already plays) made by
  `App::apply_smoothing`. MVTools and RIFE are RVP's filter `@rvpsmooth`,
  last among RVP's; their scripts end with `core.clear_cache()` (mpv answers
  frames a script asks for while it loads with black ones, which
  VapourSynth's cache kept). Blend is mpv's `interpolation` with
  `video-sync=display-resample`, and with OpenGL `display-fps-override` from
  the screen's rate. VapourSynth is RVP's own runtime in `vapoursynth/` next
  to `rvp.exe`, never SVP's, nor one on the PATH
  (`svp::vsscript_candidates`). The budget's first steps are the
  smoothing's. NVIDIA Optical Flow (`fruc_vulkan`) needs the Vulkan renderer
  (`render.renderer` "vulkan": gpu-next with `winvk` in RVP's window; mpv
  uploads the frames to the output's Vulkan device through two `format`
  filters of RVP's, since FFmpeg's `hwupload` wants its device before mpv's
  lavfi filter hands it over) and a libmpv whose FFmpeg has the filter
  (`tools/libmpv/build.sh`: shinchiro's recipe with glslang for FFmpeg's
  shaders; RVP probes `lavfi-fruc_vulkan` at start). An engine whose filter
  does not raise the rate within 6 s is left out for the session, and one
  that leaves the output dropping more than 5 % of its frames over the
  page's `drop-window` (20 s by default; 0 never), or more than half of
  them over 5 s (at once; RIFE then gives way to MVTools, as its presets
  change only its GPU threads), takes a lighter step for the rest of the
  file (`App::smoothing_watch`). Checks: `crates/app/tests/smoothing.rs` (with
  `VSSCRIPT_PATH`), `rvp --selftest=smoothing`,
  `tools/windows-vm/m18-all.ps1`, and on the host, with permission,
  `tools/windows-vm/m18-host.ps1` (each GPU mode in RVP's window at the
  power limit; `-Vm` tries it on the VM). MVTools' filter asks for 16
  frames at once (`concurrent-frames`, the host's runs of 2026-11-04:
  mpv's default of one per logical CPU dropped a frame a second), and,
  as SVP does, the script's first `original-first` seconds (1 s) are the
  source's frames repeated at the rate (`FrameEval`; not RIFE (TensorRT)'s):
  mpv loads the script again after each start and seek. While smoothing
  may run a filter, mpv holds each file at its `on_preloaded` hook
  (`Engine::hold_preloaded`, `App::smoothing_preloaded`) until the plan is
  made from the demuxer's video track (`track-list`), so the filter is in
  place before playback starts: one added after it stopped mpv's core
  while the script loaded, the sound played on and the video dropped what
  it was behind. Every `EngineEvent::Preloaded` must be continued, or the
  file never plays. Adding the filter later instead
  dropped 80 frames while mpv's chain changed. WSL's builds share the
  host's processors through the hypervisor, and RVP's process priority
  does not win them back (2026-11-06, MVTools under a full WSL load: 45,
  44 and 38 dropped frames a second at normal, above normal and high):
  read the log's `processors` share before blaming a change.
- Smoothing's calibration (`crates/app/src/calibration.rs`, the owner,
  2026-11-07, SVP's way): while nothing plays, a core without a window
  (`rvp_engine::calibrate`: `vo=null` at speed 8, not `untimed`, which
  leaves `estimated-vf-fps` at the source's rate) plays `testsrc2`
  through each engine's script at 480p to 2160p; a size is skipped once
  the last one, scaled by the pixels, cannot keep 60 with room (RIFE at
  2160p took the host's GPU down). Results and the machine's key
  (`shell/src/hardware.rs`, `rvp_hardware` in `gpu_power.cpp`, libmpv,
  the engines' files, `calibration::VERSION`: bump it when the scripts
  change) are in `<state>/calibration.json`; each file starts at the
  first step of the ladder that keeps its rate with a fifth to spare.
  NVIDIA Optical Flow is not measured. Steps down taken while other
  programs held the processors come back at the next seek
  (`App::smoothing_seek`). Self-tests measure only with `--calibrate`.
  The calibration window (`shell/qml/Calibration.qml`, from the Motion
  Smoothing page's Calibrate…/Recalibrate…, `Settings.openCalibration`)
  has Start (it pauses playback), Cancel (no run on its own until Start)
  and the progress.
- RIFE (TensorRT) (`smoothing::Engine::RifeTrt`, `crates/app/src/tensorrt.rs`,
  `docs/spikes/m18-tensorrt.md`): vs-mlrt v15.16's `vstrt` with RIFE v4.6.
  NVIDIA's licence keeps TensorRT out of RVP's packages: the Motion
  Smoothing page's Download… asks for consent to NVIDIA's terms, then
  fetches TensorRT 10.16 and CUDA 13.2's runtime and NVRTC from NVIDIA
  through the system's `curl` (2.2 GB, sizes and SHA-256 pinned in
  `tensorrt::archives`), and keeps what the card needs (its builder
  resource by `nvidia-smi`'s compute capability, about 700 MB) with the
  package's `vapoursynth/vs-mlrt` (plugin, `trtexec`, `vsmlrt.py`, model;
  `tools/packaging/vapoursynth.py`) in `<state>/tensorrt`, laid out as
  vs-mlrt expects. `vstrt` loads `vsmlrt-cuda/nvinfer_10.dll` by its path
  (RVP's library search allows it); `trtexec` builds the engine as a
  process of its own. The engine runs only once the calibration has built
  (two minutes on the host) and measured it; until then RIFE on ncnn
  stands in. At start a thread asks `nvidia-smi` which builder the card
  needs (the calibration waits for it): another card's installation
  (`other-card`) does not run and the page offers Download… again; a new
  installation removes the files and engines of the one before. mpv loads
  the script again at every start and seek, and RIFE (TensorRT)'s first
  frames after a load take about a second: it has no original frames
  first, so mpv waits for them before its clock runs (with the original
  first, the switch dropped about 60 frames; frames made while the script
  loads did not help, the host's runs of 2026-11-08). `--install-tensorrt` installs at once (tests);
  `RVP_TENSORRT_MIRROR` (a folder URL, `file://` too) replaces NVIDIA's
  servers; the host's copy of the archives is in
  `~/rvp-reference/trt/nvidia` (Linux's wheels in `nvidia-linux`).
  Never commit NVIDIA's files. Linux: NVIDIA's wheels (TensorRT's 3.7 GB,
  CUDA's runtime and NVRTC from PyPI, zips too, about 960 MB kept);
  vs-mlrt has no Linux build, so `tools/vapoursynth/build-vstrt-linux.sh`
  (called by `build-linux.sh`) builds `libvstrt.so` and `trtexec` against
  the headers of NVIDIA's Ubuntu 22.04 packages, the plugin linked to a
  TensorRT library of no content that only names `libnvinfer.so.10`
  (NVIDIA's comes in at run time by `$ORIGIN/vsmlrt-cuda`), and
  `check-deb.sh` leaves `vs-mlrt/` out of its library check.
  `cargo run -p rvp-app --example rife_trt -- FOLDER MIRROR` installs and
  measures without RVP's window (`VSSCRIPT_PATH` at RVP's runtime);
  `cargo run -p rvp-engine --example render_vf -- FILE START FRAMES OUT
  [VF]` writes the frames a filter makes as PNG files, to compare engines.
- The control panel (F7) is a second window (`shell/qml/ControlPanel.qml`)
  with its own `Chrome` in `Mode::ControlBox`, painted whole
  (`SkinView.surface` 1, `panelDamaged`).
- SVP 4 for Windows connects only to the pipe its settings name
  (`mpvpipe`); a second RVP window cannot get SVP there (goal H).
- Every action is one row of `ACTIONS` (`crates/app/src/action.rs`): its
  settings name, PotPlayer's menu text, numeric IDs and skin `CmdID`s.
  Menus are built from that table and R1's trees (`crates/app/src/menu.rs`)
  as JSON each time one opens (`shell/qml/Menus.qml`). After changing the
  table or `crates/app/src/coverage.rs`, rewrite `docs/coverage/` with
  `RVP_BLESS=1 cargo test -p rvp-app --test coverage` (with the skin
  present for `blackspace.md`).
- Settings change through `Player.set_setting` and `App::apply_settings`
  (`crates/app/src/preferences.rs`): a new setting needs a check in
  `Settings::validate`, a page in `shell/qml/Settings.qml`, and, when mpv
  can take it at run time, a line in `apply_settings`. Numbers are sliders
  (`Amount`, the owner's preference); the self-test moves them with
  `pick`, not `set`.
- Never name a QML component or file after a JavaScript global (`Number`,
  `String`, `Date`, `Math`, `JSON`...): it hides the global in that file
  (`Number(text)` threw, and the settings window applied no typed number).
- The volume is PotPlayer's percent everywhere RVP shows or takes it
  (`crates/app/src/volume.rs`: square law up to 100 %, linear above, as
  measured in `docs/audits/audio.md`); mpv gets the volume of the same
  gain, the state file keeps mpv's, and `volume-max` comes from
  `volume::mpv_volume_max` (a thousandth above, or mpv refuses the highest
  volume).
- RVP's own QML files shadow Qt types of the same name inside the `Rvp`
  module (`FolderDialog.qml`): import Qt's under a qualifier.
- Portable mode: `rvp.portable` next to the executable keeps the settings
  there; `RVP_CONFIG_DIR` and `RVP_STATE_DIR` still win (test runs).
- The log (`crates/app/src/log.rs`, the Log page, `log.level`, off by
  default): `rvp.log` in the page's folder (`log.folder`; unset: the state
  folder) at Critical, Warning, Info or Debug, written by a thread of its
  own; a new folder reopens the file at once. RVP's messages go through
  `log::report` (the standard error and the log); mpv's come on a weak
  client of the player's core read on another thread, never through the
  window's thread. Debug adds a line per second of playback: mpv's frame
  counters, the frames the OpenGL renderer drew (`video::take_drawn`) and
  the processors' load (`shell/src/cpu.rs`, `rvp_platform::load`).

## Packages and the desktop (M8)

- `tools/packaging/package.py` makes the packages from a release build:
  `windows` (the folder, with RVP's VapourSynth runtime in `vapoursynth/`
  from `tools/packaging/vapoursynth.py`: R73, Python 3.12, MVTools, RIFE
  and its model, each download checked by its SHA-256; the portable ZIP
  with `rvp.portable`, the NSIS installer of `packaging/windows/rvp.nsi`) and `linux` (AppDir, AppImage,
  .deb, on Ubuntu 22.04: kit steps `m8package` and `m8`). In WSL, makensis
  is Ubuntu's nsis package unpacked into `~/rvp-deps/nsis` (`--makensis
  ~/rvp-deps/nsis/usr/bin/makensis --nsisdir ~/rvp-deps/nsis/usr/share/nsis`)
  and the C++ runtime is `~/rvp-deps/win/msvc-runtime`. CI builds both as
  artifacts and keeps only the newest run's (`prune-artifacts`: the
  account's artifact storage is small, so push commits in batches);
  nothing is published.
- The registered types are `packaging/associations.tsv` (the installer,
  the `.desktop` file, MPRIS). RVP only registers itself as a handler and
  never makes itself the default (owner, 2026-10-16). The installer is per
  user (HKCU); its uninstaller removes only the files it installed and,
  of shared keys, those it created (`Software\RVP\Created`).
- Linux packages bundle Qt, libmpv and the libraries a desktop may lack,
  and use the system's for the rest (`SYSTEM` in package.py). RVP's binary
  for them carries `-Wl,-rpath,$ORIGIN/../lib -Wl,--disable-new-dtags`:
  no LD_LIBRARY_PATH, which programs RVP starts would inherit.
- MPRIS and idle inhibit: `crates/platform` (zbus; zbus's builder queues a
  taken name instead of failing, so RVP asks for its name itself with
  DoNotQueue), shown by `shell/src/desktop.rs`. Windows' media controls:
  `shell/src/media_controls.rs` (one FFI point, `GetForWindow`). Both send
  `MediaCommand`s to the window's thread (`App::media_command`).
- Idle inhibit asks GNOME's session manager first, then
  `org.freedesktop.ScreenSaver`, and the portal last: GNOME refuses the
  portal's inhibit for a program outside a sandbox (empty app ID) while
  the portal still answers success. Check it with the session manager's
  `IsInhibited 8` on the desktop's own bus, and find RVP by process name
  (`pgrep -x rvp`): `pgrep -f` also matches the shell running the check.
- On Windows `shell/src/process.rs` restricts the library search to
  RVP's folder, System32 and its VapourSynth runtime (never the PATH:
  SVP's `libEGL.dll` got in through it), and ends a process that loaded
  RIFE's plugin without unloading its libraries (its ncnn destructors
  crash at exit).
- Crash logs go to RVP's state folder: panics (`rvp_app::crash`), fatal
  signals and unhandled exceptions with a minidump (`shell/src/crash.rs`).
  `rvp --crash-test=panic` or `=native` crashes on purpose.
- The Windows VM's M8 check is `tools/windows-vm/m8-all.ps1` (the
  installer, `m7-all.ps1` on the installed copy, registry before and
  after, the portable ZIP). Windows' media session manager runs only for a
  user signed in at a desktop (the per-user service NPSMSvc), which the
  VM's SSH sessions are not: CI's `package-windows` job, whose hosted
  runner is, checks the media controls (`m8-smtc.ps1`); on the owner's
  host it runs only with permission.
- `tools/packaging/check-deb.sh` checks the Linux packages on another
  Ubuntu release inside a container (CI: `ubuntu:26.10`): install,
  libraries, the self-tests under Xvfb (M11's too).

## Subtitles (M13)

- Subtitle options go to mpv after it starts, each by the first name the
  running libmpv knows (`SubtitleSettings::mpv_options`: Linux's 0.37
  lacks `sub-border-style` and `sub-line-spacing`, and keeps margins
  within 0 to 300); an unset number or colour stays libmpv's own default
  (`MpvDefaults`, read through `option-info`).
- RVP reads and writes SRT, ASS and WebVTT itself (`crates/subtitle`):
  the browser and editor, Input Subtitles and Subtitle Saving work on the
  external file that shows (`crate::subtitle_editor`). libmpv gives no
  access to embedded tracks' lines. Edits show through a working copy
  (`<state>/subtitles/editing.<ext>`) added in place of the track; a file
  RVP writes over goes to `<state>/subtitles/backup/` first, and unsaved
  edits to `unsaved-<name>` when another file starts or RVP closes.
  Saving writes the times as they show (`t * sub-fps / video fps +
  sub-delay`, mpv's `dec_sub.c`) and, into the file mpv shows, sets the
  sync back to none.
- Subtitles from audio (`crate::speech`, `docs/spikes/m13-whisper.md`)
  run whisper.cpp's `whisper-cli` as a process (no console; real time below
  normal priority) on a WAV file a libmpv encode core writes. RVP's
  CPU and Vulkan build is `tools/whisper/build.sh`
  (`tools/libmpv/rootless.sh ROOT WORK tools/whisper/build.sh`, after the
  libmpv build; every program and DLL of it gets a version resource from
  `tools/whisper/version.cmake`: the host's antivirus quarantined a whisper-cli.exe
  without one, 2026-11-06) and goes into the package as `whisper/`
  (`package.py --whisper`); CUDA and BLAS are whisper.cpp's own builds the
  user puts in `<config>/whisper/<engine>`. Models are never downloaded or
  committed: the user puts `ggml-<model>.bin` into
  `<config>/whisper-models`. Tests take `RVP_WHISPER_CLI`,
  `RVP_WHISPER_MODELS` (with `ggml-tiny.bin`) and `RVP_SPEECH_CLIP` or
  espeak-ng (in WSL unpacked into `~/rvp-deps/espeak`); without them they
  pass with a note. The jobs' events reach the window's thread in
  `App::speech_tick` (each drain and the shell's one-second tick). A job
  on the GPU is a step of the GPU power budget; whisper on the host's GPU
  runs only at gpu-limit.ps1's power limit, with permission.
- Free subtitle samples for round trips are kept in
  `~/rvp-reference/subtitles` (`RVP_SUBTITLE_SAMPLES`), never
  committed.
- The Windows VM's M13 check is `tools/windows-vm/m13-all.ps1`
  (`--selftest=m13 --speech-check`, whisper.cpp on the speech clip, and
  the headless tests `formats`, `editor`, `subtitles` and `speech` built
  with `cargo xwin test --no-run`), with the clip and the tiny model in
  `C:\rvp-test\m13`.

## Preferences, skins and the interface (M14)

- Interface languages are English (the source text) and Russian,
  written for RVP (`shell/i18n/qml_ru.ts` from lupdate, `menu_ru.ts`
  for the "Menu" context); `shell/build.rs` compiles them with Qt's
  lrelease. Every user-visible QML string goes through `qsTr` (with
  `.arg()` for values); menu and action labels from Rust go through
  `Tools.tr_menu`, which sets numbers aside as `%1`, `%2`... A QML file
  that uses `Tools` imports `RvpNative`. After changing strings run
  `lupdate -no-obsolete -locations none` on the QML files (not the
  `SelfTest*` ones) into `qml_ru.ts`, translate the new entries and keep
  Russian texts short enough for their controls: `tests/translations.rs`
  fails an untranslated string or a lost placeholder, and
  `--selftest=m14` fails a settings text that does not fit.
- The QML engine keeps its last translation when `Qt.uiLanguage` names
  a language without a `.qm` file, and loads the system's at start:
  English is `Qt.uiLanguage = ""` (`Main.qml` `applyLanguage`).
- Notices RVP formats in Rust (OSD texts, errors) stay English.
- Presets (`crates/app/src/presets.rs`): the default preset is
  settings.json's own values; another preset keeps a whole copy, and
  while it is active every change goes into it.
- `docs/coverage/` must not say "not in RVP yet" or "planned" for M14's
  areas (`tests/coverage.rs` fails it): every item works or keeps a
  reason that says why it cannot.
- In fullscreen (and with "Auto hide main skin") the bar over the video
  lies on a black backdrop (`Main.qml`, `barBackdrop`): PotPlayer's bars
  are opaque windows of their own, and its video keeps its place. The
  video makes room for the bar only with `skins.fullscreen-resize` (off by
  default since goal Q: on, the picture jumped by the bar's height).
- PotPlayer's 22 bundled skins (the skin check of 2026-11-04): an opened
  `.dsf`, `.wsz` or `.vlt` (also handed over by a second `rvp`) becomes
  the skin, as in PotPlayer; another skin file keeps the video's size and
  builds the window around it. The engine follows `Alignment`,
  `AutoVisible` (`UiState::values`, `view::apply_auto_values`),
  `VisParentMaxWidth`/`Height`, `VisMinWidth`/`Height` and the states
  `CMD_ON_MEDIA_PLAY`, `CMD_ON_FULLSCREEN`; a skin without
  `MainSkinAreaOscInside` gets one built from its bottom bars; `AutoSize`
  labels take their texts' widths (`layout::text_widths`, Qt's
  `QFontMetrics` through `rvp_text_width`) and boxes their children's
  (`skin.rs`, `RvpBuilt`). `tools/windows-vm/skins-host.ps1` with
  host-session's `skins` switches every skin in PotPlayer and RVP on the
  host, with permission (captures to `~/rvp-reference`).
- `--selftest=m14 --registry-check` (the test VM only) writes the Run
  key and the file types to the current user's registry and removes
  them; never pass `--registry-check` on the host. The VM's check is
  `tools/windows-vm/m14-all.ps1`; the host comparison of skin modes is
  `tools/windows-vm/m14-host.ps1` with host-session's `m14`, with
  permission, saving and writing back PotPlayer's registry key and
  playlist folder.

## Remote control (M15)

- The control API (`crates/app/src/control`, D15) is a small HTTP/1.1
  JSON service on `std::net`: off by default (`remote.enabled`, the
  Remote Control page), 127.0.0.1 only, the installation's token
  (`<state>/remote-token`, `control::token`) in `Authorization: Bearer`
  on every request, requests with `Origin` or a `Host` that is not a
  loopback name refused, and `control::LIMITS` on headers, bodies,
  request time, connections, long polls and the window thread's answer.
  Never log the token (the self-test checks its log) and never commit one.
- Its threads never call mpv or touch `App`: commands reach the window's
  thread through `CxxQtThread::queue` (`shell/src/player/m15.rs`, as the
  media controls' do) and the status is a snapshot that `sync_chrome`
  publishes (`Server::publish`; the version grows with every change but
  the position). Actions run through `Player::perform`, as keys do.
  Answers leave out `null` fields (Unified Remote's JSON reader documents
  none).
- The Unified Remote remote is `remote/unified-remote/RVP` (`meta.id:
  RVP.Player`); `cargo xtask look` draws its `icon.png` and
  `icon_hires.png` from RVP's icon; `tests/remote.rs` checks that every
  action it calls exists and that `layout.xml` is well formed. With an
  empty token setting it reads `%APPDATA%\RVP\remote-token`. The
  installer's component copies it into
  `%ProgramData%\Unified Remote\Remotes\Custom\RVP` when Unified Remote
  is installed (`HKCU\Software\RVP\UnifiedRemote` notes where) and the
  uninstaller removes those files.
- Unified Remote Server is installed on the Windows test VM only, never
  on the host without permission. On port 9510 its manager reloads
  remotes (`/system/reload`) and its web-client protocol runs a remote's
  action (`/client/connect`, then `/client/request` packets: handshake,
  authenticate, `Action` 7 with `Run`); the manager's
  `/system/remote/run` does not run actions in 3.13. Started over SSH it
  ends with the session. Unified Remote Server 3.13 has CVE-2023-52252
  (web pages can make it run Lua). The VM's check is
  `tools/windows-vm/m15-all.ps1` (`--selftest=m15 --remote-serve`).
- MPC-HC's web interface was studied and left out: it has no token
  (`docs/spikes/m15-mpc-hc.md`).

## Disc menus (M16)

- A whole disc (`dvd://longest/DEVICE`, `bd://longest/DEVICE`) plays
  through RVP's own navigators (D8 approved libdvdnav, libdvdread and
  libbluray, `docs/spikes/s7-disc-menus.md`): `shell/cpp/disc.cpp` and
  `bluray.cpp` load `libdvdnav-4.dll` and `libbluray-4.dll` from RVP's
  folder (Linux: the system's `.so.4` and `.so.2`), `shell/src/disc`
  drives them, `shell/src/player/m16.rs` is the window's part, and
  `crates/app/src/disc` holds what needs no library (SPU decoding, the
  highlight, the Blu-ray plane). Without a library the disc plays as
  mpv's `dvd://` or `bd://`.
- mpv plays a disc as segments `rvpdisc://N` of a custom stream
  (`stream.rs`, one open disc per process): one per still, VM jump,
  playlist, or cells that go back. The number in the address binds the
  segment (mpv opens it after `loadfile` returned); a title's segment
  seeks and ends at its last sector, and the navigator reads past it
  only when mpv reached the end (`finish`). Never hold the navigator's
  lock while calling mpv: mpv's stream open takes it.
- RVP draws the menus with `overlay-add` (slot 7; the pixels stay alive
  in `Playing::shown`), maps the pointer through `osd-dimensions`, and
  takes the arrows, Enter and the left button while a menu with buttons
  shows. The disc's program chooses the streams (`Navigation::streams`:
  mpv's source IDs, DVD 0x80+ audio and 0x20+ sub-pictures, Blu-ray PIDs
  0x1100+ and 0x1200+).
- RVP never ships libdvdcss, libaacs or keys (`tools/discs/build.sh` and
  `tools/libmpv/build.sh` fail on DVDCSS strings; `package.py --discs`
  refuses such files); a user may put their own beside `rvp.exe`.
  libdvdread is built with `-mno-ms-bitfields` like libdvdnav (MinGW's
  default layout broke the button rectangles).
- Tests: `tools/test-clips/make-m16-dvd.sh` and `make-m16-bluray.py`
  write the synthetic discs (never committed; CI makes them);
  `shell/src/disc/tests.rs` (`RVP_DVD_MENU`, `RVP_BD_MENU`, and
  `RVP_DVD_REAL` for a real disc's opening and menu: the owner's copy is
  in `~/rvp-reference/discs`), one disc at a time;
  `rvp --selftest=m16 --dvd-menus=DIR --bd-menus=DIR`; the VM's check is
  `tools/windows-vm/m16-all.ps1` (the discs in `C:\rvp-test\m16`).

## Other players' skins (M9)

- The study is `docs/spikes/s6-other-skins.md`; the owner chose all three
  parts (mpv's controllers, classic Winamp, VLC). The downloads, notes and
  probes are in `~/rvp-reference/s6`: third-party skins and
  scripts are never committed, nor renders of them.
- mpv's on-screen controllers (`crates/app/src/controller.rs`,
  `shell/src/player/m9.rs`): mpv's OSC or the user's scripts from
  `<settings>/mpv` (`scripts`, `script-opts`, `fonts`, as mpv's own
  folder), chosen on the Skins page for the next start; over the skin's
  video area or, "without the skin", alone in the window. libmpv's
  `mpv_set_option_string` knows no `-append` forms: whole values for
  `scripts` (the system's path separator) and `script-opts` (values
  quoted `%N%`). Over the video the pointer goes to mpv (`mouse`,
  `MOUSE_ENTER`/`LEAVE`, `keydown`/`keyup`, never a single press, which
  the controllers ignore); clicks no controller takes come back through
  RVP's bindings in mpv's `default` section (`key-binding` messages to the
  client `main`) and run RVP's own pointer. mpv makes a double click of
  any two presses: RVP takes one only when both presses were its own.
  `user-data` reads as JSON (uosc closes its menu by setting `null`, which
  an observation does not report): menus are read when a key comes. The
  window's state goes both ways through mpv's `fullscreen`,
  `window-maximized`, `window-minimized` and `ontop`; `quit` closes RVP.
  RVP's helper cores load none of mpv's built-in scripts
  (`BUILTIN_SCRIPTS`).
- Classic Winamp skins (`crates/skin/src/winamp`): `.wsz` files in the
  skin list load as an RVP skin built in memory (`build.rs`: markup in
  PotPlayer's format and PNGs), so RVP's engine lays out, paints and
  hit-tests them. The video window is RVP's main window (`video.bmp`'s
  frame, else the playlist frame's pieces); the main window, equalizer
  and playlist are `RvpWindow='1'` areas shown as windows of their own
  (`shell/src/player/extras.rs`, `shell/qml/SkinWindow.qml`, surfaces
  from 10), docked to the main window; a drag on one moves the group.
  The engine's extensions for them: `RvpFrames` sliders, `RvpChars`
  bitmap fonts (Unicode numbers: the markup has no escapes),
  `RvpRowColorsX` visuals, `RvpOnImageLeft` toggles,
  `ThumbPressedImageLeft`, the `RvpGraph` equalizer curve, `RvpRegion`
  shapes (`Tools.set_window_mask`). `CmdID='RVP:name'` runs an action by
  its settings name, `RVPWIN:` shows and hides a window, and the `RVP:`
  texts and states come from `crate::classic`. Missing bitmaps fall back
  to RVP's own drawn in code (`fallback.rs`), which is also the tests'
  synthetic skin; a bitmap's first attribute occurrence wins in the
  markup (`El::rect` already sets `Left`).
- VLC's themes (`crates/skin/src/vlc`): `mod.rs` reads a `.vlt` (tar.gz
  or zip; `theme.xml` with `markup::parse_xml`, where quotes protect `>`
  as in libxml2) into a model, `build.rs` writes PotPlayer-format markup
  and pictures in memory, as for Winamp. The window with the video is
  the main window (its layouts `main.N.xml`, `W.setLayout(L)` loads one
  with `CMD_LOAD_SKIN_XML`), the others `RvpWindow='1'` areas placed at
  `RvpDock='offset'`. `visible` and checkbox states are flags `VLCX:N`
  (`RvpConditions`) and texts `VLCTEXT:N` (`RvpTexts`) that
  `crates/app/src/vlc.rs` computes; corners tied to the box's far edges
  are `RvpLeftFar`...`RvpBottomFar`, `RvpNoClip` panels, `RvpStretch`
  images, `RvpShape='alpha'` windows (`render::opaque_region`).
  Buttons take the pointer where their pictures are opaque
  (`RvpHit='alpha'`, `hit::hit_test_in`); VLC's `enable()`-style actions
  are RVP's toggles held back by `RvpUnless`; a title-bar picture's
  `action2` is `RvpDoubleCmd`. An action VLC knows but RVP cannot run
  leaves its control inert; one VLC does not know drops it, as VLC does.
  The theme's fonts are queued in `open_skin_file` and given to Qt in
  `keep_skin` on the window's thread: the first skin loads on a thread
  of its own before `QGuiApplication` exists, and `QFontDatabase` crashed
  there. `docs/coverage/vlc.md` comes from the builder's `ACTIONS` and
  `vlc.rs`'s tables (`RVP_BLESS=1 cargo test -p rvp-app --test
  coverage`), and its test checks that every mapped action exists.
- Tests: `rvp --selftest=m9 --controller=osc|scripts FILE`
  (`--controller-bar=FRACTION` where the seek bar lies higher: ModernZ
  0.915; scripts put into the test's `RVP_CONFIG_DIR/mpv`); the VM's
  check is `tools/windows-vm/m9-all.ps1` (OSC, uosc and ModernZ from
  `C:\rvp-test\m9`, then `--selftest=classic` and `=vlc` with the skins
  in its `winamp` and `vlc` folders; `-Part controllers` or `skins` runs
  one half). The VM is shared with other sessions and has two CPUs: a
  double click through mpv can time out while it is busy. Running downloaded scripts in tests needs the
  owner's permission (given for uosc and ModernZ, 2026-10-31).
  `rvp --selftest=classic --skin=SKIN.wsz FILE`; `tests/winamp.rs` loads
  the synthetic skin and, with `RVP_WINAMP_SKINS`, every local `.wsz`
  (the museum's sample in `~/rvp-reference/s6/winamp/skins`);
  `cargo run -p rvp-skin --example render_skin -- SKIN.wsz OUT.png W H ""
  AREA` renders a window headless (`.vlt` too).
  `rvp --selftest=vlc --skin=THEME.vlt FILE` clicks the theme's pause and
  play buttons wherever they show (`Player.control_point`) and its volume;
  `tests/vlc.rs` builds every layout of each theme in `RVP_VLC_SKINS`
  (the gallery's sample in `~/rvp-reference/s6/vlc/skins`).

## Linux (M17)

- Linux's counterparts of the shell's Windows calls live in
  `crates/platform` (zbus 6: each thread that serves a connection stays in
  one `zbus::block_on`; the desktop thread takes its messages from a
  zruntime channel): the trash (`trash.rs`, the freedesktop.org
  specification, a rename on the file's own filesystem), logind's power
  actions (`power.rs`), PulseAudio's `pactl` and `parec` for the master
  volume and recording RVP's own stream (`pulse.rs`, which PipeWire's
  pulse server answers), the recent files (`recent.rs`), the Unity
  launcher's progress and the screen saver's `ActiveChanged` (`linux.rs`).
  No code may say a part waits for Linux: `tests/coverage.rs` fails "on
  Windows for now" and its kind; `docs/coverage/linux.md` (from
  `crates/app/src/coverage/linux.rs`) lists what differs and why.
- Wayland lets no program place its windows or have child windows of
  another's: a skin with windows of its own and the Vulkan renderer start
  RVP through XWayland (`player::prefer_x11`, `QT_QPA_PLATFORM=xcb` before
  Qt starts, unless the user chose a platform).
- The Vulkan renderer on Linux is a native child QWindow (X11) that mpv's
  `x11vk` draws into. `Qt::WindowTransparentForInput` keeps the pointer
  for RVP's window under it, and also selects SubstructureRedirect, with
  which Qt swallows mpv's map request: `video_window.cpp` clears that bit
  (libxcb). The window stays mapped (outside the main window while no
  video shows): mpv waits until its own window is mapped.
- The kit's libmpv (`kit.sh mpv`) is built at the Windows build's commits
  (mpv, FFmpeg's master, libplacebo) from an SDK the kit builds (Vulkan's
  headers, glslang for FFmpeg's shaders, shaderc for libplacebo's, dav1d,
  AviSynth+'s headers); the step fails a non-free FFmpeg, a missing part
  and an undefined symbol (`ldd -r`). Bump `MPV_BUILD` with any change of
  its options. On a machine whose network to GitHub stalls, clone through
  the source cache (`RVP_KIT_SOURCES`, `tools/linux-kit/sources.sh`).
- The Linux packages carry whisper.cpp's CPU and Vulkan engine
  (`tools/whisper/build-linux.sh`, `bin/whisper/`) and RVP's own
  VapourSynth runtime (`tools/vapoursynth/build-linux.sh`,
  `bin/vapoursynth/`: VapourSynth R73 with VSScript patched to start the
  relocatable Python beside it, never through `PYTHONHOME`, which would
  reach the programs RVP starts). The runtime's compiled-in plugin folder
  is the build's: RVP's scripts load MVTools and RIFE by their paths.
- On the shared Ubuntu VM the kit runs as the kit's user in the owner's GNOME session
  through `tools/linux-kit/attach-session.sh` (ACL entries it takes back).

## Playback, playlist and opening (M11)

- The thumbnail core (`crates/engine/src/thumbnail.rs`) is a second
  libmpv core without audio, drawn with the software render API on a
  thread of its own, with a cache; it also finds durations. Create it
  after the player's core (ffmpeg's log goes to the first core).
  vo_libmpv holds a seek until its frame is rendered, and a property read
  then waits for its 200 ms timeout: the worker renders while it waits
  for the restart and reads no properties meanwhile. Previews, scenes and
  list thumbnails are for local files only (a network source could hold
  the core up).
- Power actions never run in tests: `--power-dry-run=FILE` (always on
  with `--selftest`: `<state>/power-dry-run.txt`) writes the action
  instead. Never shut down, sleep or hibernate the host or a VM.
- The Recycle Bin (`shell/src/recycle.rs`) gets full paths: the shell
  deletes a relative one for good. Linux's trash is goal S's.
- mpv numbers DVD titles from 0 (`dvd://0/DEVICE` is the first); RVP
  reads the titles from `VIDEO_TS.IFO`. mpv reads 5 s of a stream to probe
  it and goes on past a shorter title into the next one, so the DVD
  fixture's titles are longer. Backward play stalls on discs (refused
  there).
- M11's fixtures come from `tools/test-clips/make-m11-fixtures.sh`
  (bit-exact; dvdauthor for the DVD). Tests that run elsewhere take
  them from `RVP_TESTDATA`.
- Self-tests move the pointer through Player's handlers; the window's
  own hover is ignored while one runs (Qt repeats it every frame). A
  self-test option must not share a name with PotPlayer's switches
  (`/dvd`, `/cd`, `/sub`...): both reach the same command-line value.
- RVP chooses the next entry itself (queue, shuffle, the end of a file):
  seamless playback and the file navigator were studied and left out
  (`docs/spikes/m11-studies.md`).

## Audio audit (docs/audits/audio.md)

- PotPlayer's sound was measured on the host (with permission) by
  `spikes/host-session` `audio`: WASAPI loopback while RVP and PotPlayer
  play `tools/audio-audit/make-tones.sh`'s tones, PotPlayer's volume set
  with its remote messages (`WM_USER`, `0x5001` set, `0x5000` read). Before
  such a run, export `HKCU\Software\DAUM\PotPlayerMini64` and copy its
  Playlist folder; afterwards write back what changed (PotPlayer adds the
  tones to its playlist and saves its window).
- Loopback gives no samples while nothing plays: find the parts of a
  recording by their levels (`tools/audio-audit/audit.py`), not by the
  log's times. `tools/audio-audit/fit_downmix.py` fits PotPlayer's 5.1 mix
  with the S5 TomSteady model.
- Goal L's comparison: `tools/audio-audit/make_m10.py` writes test files
  whose passes start with a 2 kHz marker; `host-session m10` reads
  PotPlayer's context menu and audio Preferences pages (read only, the
  dialog closed without applying), records the presets, effects, speaker
  modes and normalizer blocks, switching passes by PotPlayer's position
  (`WM_USER` `0x5004`), and puts PotPlayer's checked states back;
  `tools/audio-audit/m10.py` cuts the recording by its markers, renders
  the same passes through RVP's chain (`cargo run --release -p rvp-app
  --example render_chain`) and prints the tables.

## Test environments

- WSL (this machine): builds, unit and headless tests. GUI programs
  started here open on the owner's Windows desktop through WSLg, with
  sound: they count as host runs (advance notice and permission every
  time). Silent tests (no window, no sound) need no permission (owner,
  2026-10-11).
- Ubuntu 22.04 VM shared with another project (`<ubuntu-vm>`, see
  the other project's notes): Linux builds and GNOME Wayland
  behaviour. It has no real GPU.
- The owner's Ubuntu 26.10 boot of the same PC (GNOME Wayland; an RTX 50-series card, `sm120`, since 2026-11-07, an RTX 30-series card before; measurements
  before that date are the RTX 30-series card's):
  GPU, SVP and performance checks, run in separate sessions through this
  repository.
- Windows test VM `<windows-vm>`: Windows builds and functional tests.
  Other projects' sessions use it too (one project's timed
  acceptance runs on a desktop of its own, and others): before a run, ask
  every session that uses it (`ListAgents`) and wait for their answer;
  RVP's self-test loads both CPUs and spoils their timings.
- Windows host: SVP on Windows and PotPlayer reference captures, only with
  permission (see above).

## Long-running commands with Longrun MCP

- For a reviewed, trusted, non-interactive command expected to run longer than about 30 seconds, use `longrun.start_job` exactly once when it is available and the exact project root is enrolled.
- After a bridge-enabled `start_job` returns, never call `longrun.get_job`, `collaboration.wait_agent`, a generic wait tool, `write_stdin`, log-tail tools, or any polling loop in that turn. In the automatically resumed turn, call `longrun.get_job` exactly once and continue from the terminal result.
- Use `wake_policy="none"` only with ordinary `codex` or an explicitly requested manual fallback. In that mode automatic wakeup is unavailable: end the turn without waiting, and use one `longrun.get_job` only in a later user-resumed turn. If it is still running, end that turn without polling again.
- Treat `longrun.run_and_wait` as legacy compatibility mode because current Codex runtimes may convert a pending blocking call into model-driven wait cycles. Progress heartbeats are disabled by default and do not fix that outer wait loop.
- Never put secret values in command arguments, MCP fields, prompts, or environment. For one reviewed non-interactive command that accepts one finite secret stdin payload, prefer `project_memory_stage_test_asset_for_longrun` when a suitable encrypted test-only asset exists, then pass only its one-time `stdin_secret_id` to Longrun; do not ask the user to re-enter that stored credential. Use `codex-longrun-secret` only when no suitable asset is enrolled. Output is suppressed for secret-stdin jobs. TTY-dependent or repeated password prompts remain unsupported. Do not use longrun for interactive programs, daemons, or unreviewed commands.
- Keep project roots narrowly enrolled. Never broaden `LONGRUN_ALLOWED_ROOTS` to `~`; add each trusted project root explicitly.
- The global MCP configuration is loaded by new Codex processes. After installing, upgrading, or changing longrun configuration, start a new process or resume the session from a new process.
- Whenever creating a project-level `AGENTS.md`, include this section unless that project explicitly opts out or already has stricter equivalent guidance.
