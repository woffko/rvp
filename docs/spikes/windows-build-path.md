# Windows build path for cxx-qt + Qt 6 + libmpv

Date: 2026-10-10. Decision: **cross-build from WSL with cargo-xwin** for
development and test-VM runs. A native MSVC build is the fallback and a
later CI check, not the daily path.

## What was tried

The full S3 shell was cross-built from WSL (Ubuntu 24.04) for
`x86_64-pc-windows-msvc`: Rust, cxx-qt 0.10.0 bridges, moc, QML module
generation (qmltyperegistrar, qmlcachegen, rcc), a C++ QQuickFramebufferObject
item and linking against Qt 6.10.3 MSVC and libmpv. The result runs on the
Windows 11 test VM (see `s3-qt-shell.md`). The S1 and S2 harnesses
(libmpv only) cross-build the same way.

A native MSVC build was not set up: the test VM (`<windows-vm>`,
2 vCPU, 8 GB) has no compiler, no Rust, no Python and no winget, and the
Visual Studio Build Tools are a multi-gigabyte install.

## How the cross build works

| Piece | Source | Notes |
|-------|--------|-------|
| Rust | rustup 1.98.1 | target `x86_64-pc-windows-msvc` |
| C/C++ compiler, linker | cargo-xwin 0.23.0: clang-cl (system clang 18.1.3), lld-link | MSVC CRT 14.44 (STL 143, update 202503) and Windows SDK 10.0.26100 from the xwin cache |
| Qt for the target | aqtinstall 3.3.0: `windows desktop 6.10.3 win64_msvc2022_64` | headers, `.lib`, `.prl`, DLLs, QML modules, plugins |
| Qt host tools | aqtinstall: `linux desktop 6.10.3 linux_gcc_64` | moc, rcc, qmlcachegen, qmltyperegistrar run on Linux |
| Qt detection | `qmake-msvc-cross`, a shell script answering `qmake -query` | `QT_INSTALL_*` point to the Windows Qt, `QT_HOST_*` to the Linux tools; qt-build-utils prefers `QT_HOST_LIBEXECS` for tools |
| libmpv | shinchiro mpv-winbuild-cmake 20261010 (`mpv v0.41.0-1115-gb2c255c13`) | `mpv.lib` generated from the DLL exports with `llvm-dlltool` |
| Deployment | `spikes/tools/deploy-windows.py` | windeployqt is a Windows program; the script copies QML and plugin trees and walks DLL imports with `llvm-objdump` |

Commands: `spikes/tools/windows-deps.sh`, then
`spikes/tools/build-windows.sh`. Everything lands in `$RVP_DEPS`
(default `~/rvp-deps`), outside the repository.

## Problems found and their fixes

1. **cargo-xwin looks for `llvm-lib`**, but Ubuntu installs `llvm-lib-18`.
   Fix: unversioned symlinks in `$RVP_DEPS/bin`.
2. **The MSVC STL in the xwin cache requires clang 19+** (`STL1000:
   Unexpected compiler version`). Fix: a `clang-cl` wrapper that defines
   `_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH`. Better fix: clang 19 or 20
   (`sudo apt install clang-20 lld-20` on Ubuntu 24.04), which needs an
   administrator.
3. **cxx-qt's default constructor passes `QObject*` to the base class**, which
   `QQuickPaintedItem` rejects. Fix: declare
   `impl cxx_qt::Constructor<()> for SkinItem` with empty base arguments.
   This is not specific to cross-building.
4. **rcc from the Linux Qt compresses resources with zstd**, but the Windows
   Qt is built without zstd (`qResourceFeatureZstd` is undefined at link time).
   Fix: an `rcc` wrapper that adds `--no-zstd`.

## Why cross-building

- One development machine: everything builds from WSL, next to the Linux
  build, with the same sources and lock file.
- Fast iteration: an incremental rebuild of the shell takes about 10 seconds.
- No Windows tooling to maintain on test machines; bundles are copied over
  SSH.
- Reproducible: two scripts, pinned versions.

## Risks and the fallback

- The qmake stand-in and the tool wrappers are home-made glue. A Qt or
  cxx-qt upgrade can break them; the failure modes above show what to check.
- The clang/STL version mismatch should be removed with clang 19+.
- No windeployqt: a missing plugin shows up only at run time. The deploy
  script copies whole `qml` and `plugins` trees to stay on the safe side
  (bundle about 240 MB).
- Fallback: a native MSVC build (Visual Studio Build Tools, rustup, Qt from
  aqtinstall, CMake not required by cxx-qt-build). The plan is to add it as a
  GitHub Actions job on `windows-2025` when the real shell exists (M2), so the
  release build is native even if day-to-day builds are crossed.
