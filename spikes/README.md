# Goal B spikes

Throwaway programs that answer the questions of `docs/PLAN.md` M0 (S1, S2,
S3). They are a separate Cargo workspace: cxx-qt bridges and the libmpv
render API need `unsafe`, which the main workspace forbids, and the spikes
are not held to its clippy rules. Results are in `docs/spikes/`.

| Crate | Question |
|-------|----------|
| `s1-svp-attach` | Does SVP 4 Manager attach to a libmpv player that is not `mpv`? |
| `s2-vapoursynth` | Does libmpv run PotPlayer's motion interpolation script with SVP's VapourSynth runtime? |
| `s3-qt-shell` | cxx-qt + QML frameless window, a menu larger than the window, a Rust `QQuickPaintedItem` painting `rvp_skin`, an mpv OpenGL video item |

## Building on Linux

`tools/linux-kit/kit.sh deps mpv build` prepares Qt, libmpv and builds all
three; see `tools/linux-kit/CHECKLIST.md`. By hand:

```sh
QMAKE=/path/to/qt/6.10.3/gcc_64/bin/qmake MPV_LIB_DIR=/path/to/libmpv/lib \
    cargo build --release
```

## Cross-building for Windows

```sh
spikes/tools/windows-deps.sh     # Qt (both platforms), libmpv, VapourSynth runtime
spikes/tools/build-windows.sh    # cargo xwin build + runnable bundles
```

The bundles land in `$RVP_DEPS/win` (default `~/rvp-deps`), outside the
repository: they contain third-party binaries. See
`docs/spikes/windows-build-path.md`.
