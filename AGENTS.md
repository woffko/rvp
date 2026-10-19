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
  runs on the host go through it (`m18-study.ps1 -Gpu`, or
  `gpu-limit.ps1 -Run COMMAND`), with `nvidia-smi` checked before and
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
  images, Windows icon resource and `RVP Default.dsf` into its `OUT_DIR`;
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
  display's rate; off while `@svp` is in the chain or RVP's own script
  slot runs) made by `App::apply_smoothing`. MVTools and RIFE are RVP's
  filter `@rvpsmooth`, last among RVP's; their scripts end with
  `core.clear_cache()` (mpv answers frames a script asks for while it
  loads with black ones, which VapourSynth's cache kept). Blend is mpv's
  `interpolation` with `video-sync=display-resample`, and with OpenGL
  `display-fps-override` from the screen's rate. VapourSynth is RVP's own
  runtime in `vapoursynth/` next to `rvp.exe`, never SVP's, nor one on the
  PATH (`svp::vsscript_candidates`). The budget's first steps are the
  smoothing's. Checks: `crates/app/tests/smoothing.rs` (with
  `VSSCRIPT_PATH`), `rvp --selftest=smoothing`, `tools/windows-vm/m18-all.ps1`.
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
  artifacts; nothing is published.
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
- The owner's Ubuntu 26.10 boot of the same PC (GNOME Wayland, RTX 30-series card):
  GPU, SVP and performance checks, run in separate sessions through this
  repository.
- Windows test VM `<windows-vm>`: Windows builds and functional tests.
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
