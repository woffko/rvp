# Goal B status: spikes S2 and S3, Windows build path, Linux test kit

Date: 2026-10-10. Result: done except S2, whose pass condition is not met
as written and needs an owner decision (open question 1). Further gaps are
listed under "Not verified".

## Summary

| Item | Result | Details |
|------|--------|---------|
| Windows build path | Decided: cross-build from WSL with cargo-xwin; native MSVC as fallback and later CI job | `docs/spikes/windows-build-path.md` |
| S2 on the Windows VM | **Not met as written.** PotPlayer's script fails unchanged on SVP's VapourSynth R73 (`vs.get_core()` was removed). The unchanged file reaches 23.976 → 60.000 fps either through a generated wrapper or, passed directly to libmpv, with a 6-line `get_core` shim in the Python runtime. Owner decision needed | `docs/spikes/s2-vapoursynth.md` |
| S3 on the Windows VM | PASS (software OpenGL, session 0): frameless window, menu window 264x509 beyond a 440x77 window, Rust `QQuickPaintedItem` painting BlackSpace (138 operations), mpv video item, wakeup callback on the Qt thread | `docs/spikes/s3-qt-shell.md` |
| S3 on the Ubuntu VM | PASS (GNOME 42 Wayland, llvmpipe): self-test plus a real-input check through ydotool. Move by the dragged distance, resize through the grip, a 200x388 menu popup window over a 440x77 window, BlackSpace painted with mpv video in its slot | `docs/spikes/s3-qt-shell.md`, `docs/test-runs/` |
| Linux test kit | Runs end to end on the Ubuntu VM: check, deps, mpv, build, s3, input and report PASS; s2 and s1 SKIPPED (no SVP); GPU checks SKIPPED | `tools/linux-kit/` |
| S1 | Not part of goal B (PLAN.md runs it in goals C and D); harness and kit step prepared | `docs/spikes/s1-svp-attach.md` |

## What was built

- `spikes/`: a separate Cargo workspace (excluded from the main one, so the
  main workspace keeps `unsafe` forbidden and its clippy rules) with three
  throwaway crates: `s1-svp-attach`, `s2-vapoursynth`, `s3-qt-shell`. See
  `spikes/README.md`.
- `spikes/tools/windows-deps.sh` and `spikes/tools/build-windows.sh`:
  reproducible Windows cross-build (Qt 6.10.3 for both platforms through
  aqtinstall, shinchiro libmpv 20261010, VapourSynth R73 portable, embeddable
  Python 3.12.9, mvtools v24) into `~/rvp-deps`, outside the repository.
  `spikes/tools/deploy-windows.py` replaces windeployqt.
- `tools/linux-kit/kit.sh`, `input-check.py` and `CHECKLIST.md`: builds
  Qt 6.10.3, libmpv (mpv `b2c255c13e`, FFmpeg and libplacebo releases,
  VapourSynth loaded at run time) and the spikes in user space, runs S1, S2,
  S3 and an input check, and writes `docs/test-runs/<date>-<host>.md`. Root
  steps are only printed.

## Versions

| Component | Version |
|-----------|---------|
| Rust | 1.98.1 (`rust-toolchain.toml`) |
| cxx / cxx-qt / cxx-qt-lib / cxx-qt-build | 1.0.204 / 0.10.0 / 0.10.0 / 0.10.0 |
| libmpv2 (crate) | 6.0.0 |
| Qt | 6.10.3 (aqtinstall 3.3.0) on both systems |
| cargo-xwin | 0.23.0, MSVC CRT 14.44, Windows SDK 10.0.26100, clang 18.1.3 |
| libmpv, Windows | shinchiro 20261010: mpv v0.41.0-1115-gb2c255c13, FFmpeg N-127267-gef305e32e, libplacebo v7.374.0 |
| libmpv, Linux | built by the kit: mpv `b2c255c13e`, FFmpeg n9.0.2, libplacebo v7.360.1, libass 0.17.5-11-gf61db56 (mpv-build `9443097`) |
| VapourSynth, Python (Windows) | R73 portable, embeddable 3.12.9 (as in SVP 4's `mpv64`) |

## Findings that change the plan

1. **PotPlayer's `.vpy` scripts need `vs.get_core()`**, which current
   VapourSynth (R73) lacks. How RVP provides it is open question 1.
2. **libmpv2 6.0.0's event loop is ambiguous**: `wait_event` returns `None`
   both for "no event" and for an unconvertible property change, which stalls
   a drain loop. RVP needs a small event wrapper or a patched binding (M2).
3. **The libmpv render context must exist before `loadfile`** with
   `vo=libmpv`, and shutdown order needs care (the spike exits hard).
4. **Qt Quick menus need a custom delegate**: `\t` shortcuts are not shown as
   a column. PotPlayer-style menus are custom-styled anyway (M3).
5. **Wayland needs real input for popups and system moves.** A grabbing
   popup and `startSystemMove`/`startSystemResize` are accepted only in
   response to an input event, so they must be called from press handlers,
   and automated checks need injected input (the kit's input check).
6. **The Windows build is a cross build.** It needs four pieces of glue (LLVM
   tool names, a clang-cl STL macro, a qmake stand-in, rcc without zstd); a
   native MSVC job is planned for CI at M2.
7. **On old Vulkan headers** (Ubuntu 22.04) the libmpv build must turn off
   Vulkan in libplacebo and mpv; the kit does this automatically.

## Not verified (moved to later goals)

- Real mouse move/resize and visible video on Windows: the test VM has no
  interactive desktop over SSH (session 0). Goal C on the host, or an
  interactive session on the VM (owner decision below).
- Performance on a real GPU (4K60 without dropped frames): goals C and D.
  Video is visible in the Ubuntu VM grab; on the Windows VM grabs show a
  black video slot (session 0, a new GL context per offscreen grab).
- S2 on Linux: needs SVP's VapourSynth runtime and mvtools on the owner's
  Ubuntu 26.10 boot (goal D).
- S1 on both systems: goals C and D.

## Open questions for the owner

1. **S2 pass condition.** "Runs unchanged" cannot be met with SVP's R73
   runtime as shipped. Choose: (a) accept a run-time wrapper that executes the
   unchanged file (recommended: nothing installed changes, works with SVP's
   runtime on Linux); (b) accept a `get_core` shim in a runtime RVP ships
   itself (the file goes to libmpv directly, but SVP's runtime cannot be
   used); (c) keep the condition and require an older VapourSynth, which
   contradicts SVP compatibility. Details in `docs/spikes/s2-vapoursynth.md`.
2. **License rule (still open from goal A).** The `rust` skill forbids
   GPL/LGPL dependencies, but RVP is GPL-3.0-or-later (D8) and the spikes
   now link libmpv (GPL build) and Qt (LGPL, dynamic). The spikes went ahead
   on the approved plan; please confirm that the rule is waived for libmpv
   and Qt before M2 adds them to the main workspace.
3. **Interactive Windows verification.** Either enable an interactive session
   on the Windows test VM (for example autologon of a test user with the
   spike started from a scheduled task) or keep these checks in goal C on the
   host with permission each time.
4. **clang 19+ in WSL.** Installing `clang-20 lld-20` needs `sudo` in WSL and
   removes the STL version macro from the cross build. Not required now.
