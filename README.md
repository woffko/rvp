# RVP (Rust Video Player)

A desktop media player for Linux and Windows that looks like PotPlayer with
the BlackSpace skin and works with SVP 4 for frame interpolation. Playback
uses libmpv; the interface is drawn by RVP's own engine for PotPlayer `.dsf`
skins.

Status: milestone M2, the player window. `rvp` plays files in the classic
BlackSpace window with PotPlayer's default keys and mouse actions and
resumes every file where it stopped. The playlist panel shows mpv's
playlist; editing it, menus and settings come in later milestones. See
[docs/PLAN.md](docs/PLAN.md) for the plan and [docs/goals](docs/goals) for
progress reports.

## Layout

| Path | Contents |
|------|----------|
| `crates/skin` | `.dsf` loader, tolerant markup parser, preprocessor, layout, display list, headless renderer, hit testing |
| `crates/engine` | libmpv wrapper: options, observed properties, events, commands |
| `crates/app` | actions, PotPlayer key and mouse bindings, settings, crash-safe resume store, the window's chrome and mouse logic |
| `crates/platform` | operating system integration (placeholder) |
| `shell` | `rvp`: the Qt Quick window (cxx-qt, QML, the libmpv video item) |
| `xtask` | `cargo xtask ci` |

## Building and checks

The engine links libmpv and the shell links Qt 6.10. Outside the system
paths, point `MPV_LIB_DIR` at the folder with `libmpv.so` (or `mpv.lib` on
Windows) and `QMAKE` at Qt's `qmake`:

```sh
export MPV_LIB_DIR=/path/to/libmpv QMAKE=/path/to/Qt/6.10.3/gcc_64/bin/qmake
cargo build --release -p rvp-shell
cargo xtask ci        # fmt, clippy (pedantic, warnings denied), tests, cargo deny
```

The toolchain is pinned in `rust-toolchain.toml`. Windows builds are
cross-built from Linux (`docs/spikes/windows-build-path.md`); the Linux test
kit (`tools/linux-kit`) builds libmpv with the PipeWire audio output and
runs RVP's checks on a GNOME Wayland desktop.

## Running

```sh
rvp [--skin=PATH] [FILE|URL]...
```

Without files RVP reopens the last file, paused where it stopped. Settings
are in `~/.config/rvp/settings.json` (Windows: `%APPDATA%\RVP`), written with
their defaults on the first start: the skin, hardware decoding, the seek
steps, the volume step, and key and mouse bindings that override
PotPlayer's defaults (`docs/r1/hotkeys.tsv`). The playback state (last file,
positions, volume, window) is in `~/.local/state/rvp/state.json`.
`RVP_CONFIG_DIR` and `RVP_STATE_DIR` move them. On Windows the skin
defaults to PotPlayer's installed BlackSpace; on Linux set `skin` or pass
`--skin`.

`rvp --selftest` runs a scripted check of the window and prints the
usability measurements; `--selftest=playback` and `--selftest=resume` are
the kit's playback and restart checks (see `shell/qml/Main.qml`).

## Skins

RVP reads `.dsf` skins that you already have; it does not ship PotPlayer
skins or artwork. To render a skin to PNG:

```sh
cargo run --release -p rvp-skin --example render_skin -- BlackSpace_en.dsf out.png 1920 1080
```

An optional fifth argument names a preview content file (see
`crates/skin/src/demo.rs`).

Tests that need the real BlackSpace skin read it from `RVP_TEST_SKIN`
(default: the PotPlayer folder as seen from WSL) and pass with a note when it
is missing. Their renders and geometry snapshots go to `RVP_REFERENCE_DIR`
(default `~/rvp-reference`), outside the repository. Fonts are found
in `RVP_FONT_DIRS` and the system font folders.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
