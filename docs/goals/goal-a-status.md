# Goal A status: repository bootstrap, spike S4, milestone M1

Date: 2026-10-09. Result: done. The skin engine renders BlackSpace's classic
window with the attached playlist. It matches the owner's PotPlayer
screenshot within a mean difference of 2.1 per channel (0-255 scale). Most
of the remaining difference comes from text rasterization.

## What was built

- Repository `~/rvp` (moved from `~/VLCpot/docs`), private
  GitHub repository `woffko/rvp`, branch `main`.
- Cargo workspace with crates `skin` (implemented), `engine`, `app`,
  `platform` (placeholders) and `xtask`; toolchain 1.98.1; GPL-3.0-or-later;
  `AGENTS.md` with the shared rules and the Longrun section; `cargo xtask ci`
  (fmt, clippy `all` + `pedantic` denied, tests, `cargo deny`); GitHub
  Actions for Linux and Windows plus `cargo deny`.
- `crates/skin`:

  | Module | Role |
  |--------|------|
  | `archive` | `.dsf` (ZIP) or directory access, case-insensitive names |
  | `text` | UTF-8/UTF-16/Windows-1252 decoding; byte order marks anywhere |
  | `preprocess` | `#include` with macros, `#if`/`#else`/`#endif`, source line map |
  | `markup` | tolerant tag and attribute reader, element tree |
  | `lang` | `{$CPnnn=...$}` variants, `StringTable` |
  | `image` | PNG and BMP (8/24/32-bit, magenta key) decoding, PNG encoding |
  | `layout` | docking, coordinates, visibility rules, attached playlist |
  | `paint` | display list: nine-slices, button states, labels, `PicLabel`, sliders |
  | `playlist` | album tabs, rows, scroll bar, selection frame |
  | `fonts`, `render` | font lookup, glyph rasterization (swash), tiled blits |
  | `hit` | topmost control, title bar or slot under a point |
  | `compare` | box resize, mean difference, diff images |
  | `demo` | preview content format for renders |

- Examples: `render_skin` (skin area to PNG) and `compare_reference`
  (aligns a render with a scaled screenshot, writes side-by-side crops).
- Dependencies (exact versions): `zip` 9.0.0 (deflate only), `png` 0.18.1,
  `swash` 0.2.10. All third-party licenses are permissive (0BSD, Apache-2.0,
  MIT, Unicode-3.0, Unlicense, Zlib); no advisories.

## PotPlayer behaviour established

Each rule is documented at the top of its module and covered by a test.

- Disabled lines: text without a leading `<` is ignored. BlackSpace has 103
  such lines.
- A tag ends at the first `>` except in the `< >` tooltip separator. Quotes do
  not protect `>`.
- Attributes are read left to right. With a broken quote
  (`ThumbHeight='15 ImageLeft='132'`), the attribute after the broken one is
  lost. BlackSpace's volume slider proves it: PotPlayer draws its fill
  sprite from x = 0.
- Unquoted values (`AltCmdID=CMD_POPUP_USER_MENU`) and glued attributes
  (`'11'Transparent=`) are accepted; numbers follow C `atoi`.
- Docking follows document order. In BlackSpace the playlist reaches the
  bottom border and the bottom bar spans only the video.
- Negative `Left`/`Top` count from the far edge. `Width`/`Height` of zero or
  less put the far edge that many pixels before the parent's edge.
  `HoriCenter` uses `Left` as an offset from the centre. The title text is
  centred at `W/2 - 15`, which matches the screenshot.
- Nine-slice middles are tiled, not stretched (the dotted seek bar stays
  dotted).
- `PicLabel` bitmap fonts have 22 cells for `%`..`:`; cell width is
  `ImageWidth / 22` rounded down (134 px gives 6 px cells).
- `MappingDB` cells are `ImageWidth` wide. `CMD_PLAY_FILE_TYPE` uses the
  order DVD, CAM, CAP, ATV, DTV, ASF, MKV, REAL, MP4, OGG, AVI, MPG, ETC,
  UCC, TS.
- A disabled slider draws its `AltImage` track (idle seek bar).
- An attached playlist uses only the `CMD_PLAYLIST_HOLDER` part of
  `PlaylistSkinArea`, inside the container slot (not the splitter).
- The selected album tab uses the third tab sprite.

## Tests

- 28 unit tests, 8 tests on a synthetic skin with a generated atlas
  (`crates/skin/tests/synthetic.rs`, geometry snapshot
  `tests/snapshots/synthetic-100x60.txt`). The synthetic skin covers every
  quirk above, button states, inactive window, sliders, bitmap labels, hit
  testing and the visibility rules.
- 4 BlackSpace tests (`crates/skin/tests/blackspace.rs`). They load with no
  warnings, check key geometry at 1280x720, 1920x1080 and 2560x1080 and the
  full geometry snapshots (kept in `~/rvp-reference/snapshots`), and
  write renders. Their screenshot comparison must stay below 2.5. They pass
  with a note when the skin is absent, as in CI.
- `cargo xtask ci`: all checks pass locally.

## Renders (local only, `~/rvp-reference`)

- `renders/blackspace-classic-1280x720.png`, `-1920x1080.png`,
  `-2560x1080.png`: generic sample content.
- `renders/blackspace-screenshot-match-2228x906.png`: content reconstructed
  from the owner's screenshot (`demo-screenshot.txt`).
- `compare/idle-*.png`: render (left) and screenshot (right), three times
  enlarged, per region.

## Comparison with the owner's screenshots

The screenshots were scaled down before they reached us. The alignment
search found a 2228x906 window scaled by 0.894 and offset by (3, 7). The row
pitch confirms it: 17.865 screenshot pixels per 20-pixel row.

Mean difference per channel by region:

| Region | Difference |
|--------|-----------:|
| Whole window | 2.08 |
| Video area | 0.74 |
| Bottom bar, centre | 2.82 |
| Bottom bar, left (info strip, time) | 4.78 |
| Bottom bar, right (volume, buttons, playlist footer) | 4.99 |
| Title bar, left / right | 6.14 / 8.15 |
| Playlist top / middle / bottom | 10.08 / 8.26 / 8.66 |

Visible differences in `potplayer-classic-idle.png`:

1. Text rasterization. PotPlayer draws GDI ClearType text (coloured
   fringes, hinted stems); RVP draws grey-scale anti-aliased text. Widths
   differ by about a pixel per word, so an ellipsis can cut one character
   earlier.
2. Playlist file icon. PotPlayer's icon (white page, orange ring) comes from
   PotPlayer itself, not the skin; RVP draws a simple stand-in.
3. Album tab widths. PotPlayer's tabs measure about 52, 62, 63 and 68 px;
   RVP uses the skin's `AlbumTabBtnW` (70) for every tab. The rule is unknown.
4. Album tab text looks slightly larger in PotPlayer (ClearType, or a
   different size than the 12 px Segoe UI used by RVP).
5. Window edge. PotPlayer's window has the Windows 11 border and shadow and
   the `UsingRgn` shape; the headless render is a plain rectangle. The
   title-bar buttons sit about 1-2 px higher in the screenshot.
6. Scroll bar. Sprite rectangles in `ScrollBar.bmp` are inferred; arrows and
   thumb differ slightly in size and position.
7. Unplayable rows. PotPlayer's `<Can't open>` rows are a dim blue-grey; RVP
   uses neutral grey.

The other screenshots (fullscreen, playing with a tooltip, context menu,
Preferences) concern the shell, tooltips and menus, which belong to later
goals.

## Assumptions to verify with reference captures (R1, goal C)

- `AltImage*` on statics means "window inactive" (the screenshot shows an
  active window only).
- Button sprite order normal, hover, pressed, disabled. Only the normal state
  was compared.
- Playlist internals calibrated on one screenshot: 20 px rows, Segoe UI
  12 px, 10 px empty space under the last row when scrolled to the end,
  4 px gap before the duration, a one-pixel `FrameColor` outline around
  selected rows.
- The PotPlayer list scrolls to its end; the top row is cut.

## Open questions for the owner

1. The `rust` skill forbids GPL/LGPL dependencies, but the approved plan uses
   libmpv (GPL) and Qt (LGPL), and RVP itself is GPL-3.0-or-later. Please
   confirm that this rule is waived for libmpv and Qt in this project before
   goal B adds them.
2. Album tab widths: R1 captures with several tab names would settle the
   rule (difference 3).
3. Playlist icon: draw our own original icon close to PotPlayer's, or keep a
   neutral one?
