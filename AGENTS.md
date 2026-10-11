# RVP agent instructions

These rules apply to every change in this repository, in addition to the
user's global instructions.

## Project

RVP (Rust Video Player) is a PotPlayer-style media player for Linux and
Windows built on libmpv, with its own engine for PotPlayer `.dsf` skins and
SVP 4 support. The plan, decisions and `/goal` texts are in `docs/PLAN.md`;
each goal writes `docs/goals/goal-<letter>-status.md`.

## Shared rules (PLAN.md section 10)

- Repository content is in English: code comments, documentation, commit
  messages, pull requests and issues. Conversation with the owner may be in
  Russian.
- Never commit PotPlayer or SVP files, renders or captures made from them, or
  third-party binaries. Read them from their installed locations at run time.
  Local-only material lives in `~/rvp-reference` (screenshots,
  renders, geometry snapshots of third-party skins, preview content).
- Never change PotPlayer, SVP or system settings on the Windows host.
  Read-only access (files, registry queries) is fine.
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

## Shell notes

- `rvp --selftest` (also `=playback` and `=resume`, see `shell/qml/Main.qml`)
  is the functional check for the VMs, the kit and the host. It drives RVP
  through its own input handlers, never through the window system, and
  prints `check`, `metric` and `status` lines.
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
- The Windows test VM's M3 checks are `tools/windows-vm/m3-all.ps1` (copy
  it and `run.ps1` to `C:\rvp-test` with the bundle).

## Test environments

- WSL (this machine): builds, unit and headless tests.
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
