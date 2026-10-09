# RVP (Rust Video Player)

A desktop media player for Linux and Windows that looks like PotPlayer with
the BlackSpace skin and works with SVP 4 for frame interpolation. Playback
will use libmpv; the interface is drawn by RVP's own engine for PotPlayer
`.dsf` skins.

Status: early development. The skin engine (`crates/skin`) loads, lays out
and renders BlackSpace headless. Playback and the window shell come next;
see [docs/PLAN.md](docs/PLAN.md) for the plan and
[docs/goals](docs/goals) for progress reports.

## Layout

| Path | Contents |
|------|----------|
| `crates/skin` | `.dsf` loader, tolerant markup parser, preprocessor, layout, display list, headless renderer, hit testing |
| `crates/engine` | libmpv wrapper (placeholder) |
| `crates/app` | commands, input bindings, settings, playlists (placeholder) |
| `crates/platform` | operating system integration (placeholder) |
| `xtask` | `cargo xtask ci` |

## Building and checks

```sh
cargo build
cargo xtask ci        # fmt, clippy (pedantic, warnings denied), tests, cargo deny
```

The toolchain is pinned in `rust-toolchain.toml`.

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
