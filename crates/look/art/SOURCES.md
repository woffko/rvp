# Sources of RVP's own art

Every file below was drawn or written for RVP in goal I (milestone M7) and
is part of RVP, under RVP's license (GPL-3.0-or-later). None is taken from
PotPlayer or its skins: no files, sprites, crops, tracings or sampled
colours, and no Daum, Kakao or PotPlayer names or marks. The skin keeps the
classic PotPlayer window's layout and commands on purpose (PLAN.md, M7);
its pictures, colours and fonts are its own. No third-party icon set or
font is used.

## Logo (`logo/`)

| File | What | Origin |
|------|------|--------|
| `symbol.svg` | The RVP symbol: a play triangle cut into three facets (amber, orange, copper) | Drawn for RVP; draft A "Facet", chosen by the owner on 2026-10-15 |
| `icon.svg` | The application icon: the symbol on a graphite tile | Drawn for RVP |
| `wordmark.svg` | The symbol followed by the letters R, V and P | Drawn for RVP; letters constructed from rectangles and arcs |

`crates/look` renders them into the PNG icons, `rvp.ico`, the Windows
resource, the idle screen's symbol and the About box's wordmark, and
`icon.svg` also into the Unified Remote remote's `icon.png` (64 pixels) and
`icon_hires.png` (512 pixels) (M15). RVP Default's `icon-mute` (below) is
also the remote's `mute.png` button picture (48 pixels, white).

## RVP Default (`skin/`)

| File | What | Origin |
|------|------|--------|
| `VideoSkin.xml`, `TitleBar.txt`, `ControlBar.txt` | The skin markup | Written for RVP in PotPlayer's skin format; element names, attribute names and command IDs are that format's vocabulary |
| `sprites.xml` | The sprite list and the palette | Written for RVP; colours chosen for RVP around the logo |
| `icons.svg` | 40 icons on a 24-unit grid | Drawn for RVP |
| `font.svg` | RVP Caps: monoline capitals, digits and signs for captions and the time display | Drawn for RVP |

At run time the skin's labels (title, playlist, equalizer preset, subtitle
font) name the system font Segoe UI; where it is missing (Linux), the
system's default sans-serif font is used. No font is shipped with the
skin.
