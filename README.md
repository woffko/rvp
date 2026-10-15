# RVP (Rust Video Player)

A desktop media player for Linux and Windows that looks like PotPlayer with
the BlackSpace skin and works with SVP 4 for frame interpolation. Playback
uses libmpv; the interface is drawn by RVP's own engine for PotPlayer `.dsf`
skins.

Status: milestones M1-M7. `rvp` plays files in the classic PotPlayer
window, drawn by its own skin RVP Default or by the BlackSpace skin, with
PotPlayer's keys, mouse actions, menus and a settings window; albums in
the playlist panel and every file's position survive restarts and
crashes; the normalizer and a ten-band equalizer; SVP 4 mode, VapourSynth
scripts (PotPlayer's `.vpy` files), GLSL shaders, colour controls and the
F7 control panel. Next: platform integration and packages (M8). See
[docs/PLAN.md](docs/PLAN.md) for the plan and [docs/goals](docs/goals) for
progress reports.

## Layout

| Path | Contents |
|------|----------|
| `crates/skin` | `.dsf` loader, tolerant markup parser, preprocessor, layout, display list, headless renderer, hit testing |
| `crates/look` | RVP's own art: the logo and the default skin RVP Default as vector sources (`art/`), an SVG renderer, and the builder of the icons and the skin |
| `crates/engine` | libmpv wrapper: options, observed properties, events, commands |
| `crates/app` | actions, PotPlayer key and mouse bindings, settings, crash-safe resume store, the window's chrome and mouse logic |
| `crates/platform` | operating system integration (placeholder) |
| `shell` | `rvp`: the Qt Quick window (cxx-qt, QML, the libmpv video item) |
| `xtask` | `cargo xtask ci`, `cargo xtask look DIR` (writes the icons, logo images and `RVP Default.dsf`) |

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
`RVP_CONFIG_DIR` and `RVP_STATE_DIR` move them. Without a skin set RVP
shows RVP Default, its own skin, which is built into the program; set
`skin` (or pass `--skin`) to use a PotPlayer `.dsf` skin such as
BlackSpace.

`rvp --selftest` runs a scripted check of the window and prints the
usability measurements; `--selftest=playback`, `resume`, `upright`, `svp`,
`vpy` and `speed` are the kit's playback, restart, orientation, SVP,
script slot and speed-change checks (see `shell/qml/Main.qml`).

## Skins

RVP ships one skin of its own, RVP Default, drawn for RVP from the sources
in `crates/look/art/skin` (`crates/look/art/SOURCES.md` lists every asset).
It also reads the `.dsf` skins that you already have; it does not ship
PotPlayer skins or artwork, and in their title bar it shows RVP's logo
instead of the player's. To render a skin to PNG:

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
