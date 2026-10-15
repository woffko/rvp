# Goal I status: goal H's leftovers and milestone M7 (RVP's own look)

Date: 2026-10-15. Result: **done**. Tests and CI are green; the owner chose
the logo and approved the skin; RVP starts with its own skin, RVP Default,
on the Ubuntu VM and on the Windows VM, neither of which has PotPlayer.
Precondition met: `goal-h-status.md` records goal H as done.

**M6's acceptance still needs the owner's Ubuntu boot run**
(`tools/linux-kit/kit.sh m6all`, or `m7all`, which adds this goal's check;
checklist section 8). This goal did not need the Ubuntu boot.

## Goal H's leftovers

| Item | Result |
|------|--------|
| 1. A copy for the owner's hands-on checks | `C:\Users\<user>\rvp-test\rvp-current` on the host: a portable build (`rvp.portable`), default settings with SVP mode on `mpvpipe`; start it with `rvp.exe` (settings and state appear next to it). Its build (with BlackSpace, the old default on Windows) was handed over on 2026-10-15 for M6's checks and M3's trial and passed the self-test on the Windows VM. The final build of this goal went to a new folder, `C:\Users\<user>\rvp-test\rvp-m7` (portable, RVP Default unless a skin is set in F5, Skins), so the owner's copy was not touched; the other files in `rvp-test` were left alone. |
| 2. The control panel's subtitle font and its place | `ID_CAPTION_FONT_SELECT` (Alt+F, `subtitle-font`) opens a font dialog loaded on first use and sets mpv's `sub-font`; `CMD_CAPTION_FONT_MESSAGE` shows the family ("Default" when unset); the `subtitle-font` setting is checked (trimmed, at most 100 characters, no control characters) and has a field with a font dialog on the Subtitles page. The panel saves its place 500 ms after a move (not on Wayland, which reports no positions) and opens there next time when that point is on a screen. Commit c04df80. |
| 3. M6's GPU check on the Windows host | One approved run (`run-hg.cmd`): the 4K60 HEVC clip with colours, deband, a shader and the equalizer on, `hwdec=auto-copy` (d3d11va-copy), 25 s each. 1920x1080 window: 0 frames dropped (PASS). Fullscreen: 1 frame dropped, about 8 s in, the decoder dropped none (the check demands 0, so FAIL by one frame). Taken as noise: goals E and G measured 0 on the host; not re-run without permission. |

## M7 summary

| Deliverable | Result | Where |
|-------------|--------|-------|
| Logo drafts | Three drafts drawn as SVG and shown on a local page (`C:\Users\<user>\rvp-test\m7-review\logo`): A "Facet" (a play triangle in three facets, amber, orange and copper), B "Orbit" (an open ring with the play triangle breaking out), C "Monogram" (an R whose counter is a play triangle). The owner picked C, then A, on 2026-10-15 (D11) | `crates/look/art/logo` (A only) |
| Icons | PNG at 16-256 pixels, `rvp.ico` (16, 20, 24, 32, 40, 48 and 64 as 32-bit bitmaps, 256 as PNG), a Windows resource (`RT_GROUP_ICON` `IDI_ICON1`) linked into `rvp.exe` and the SVG for Linux packages (M8). Every window gets the icon through `QGuiApplication::setWindowIcon`. On the Windows VM, Windows extracted the icon from `rvp.exe` (the graphite tile with the symbol) | `crates/look/src/brand.rs`, `shell/build.rs`, `shell/cpp/shell.cpp` |
| Idle screen and About box | While no video picture shows, the symbol sits in the middle of the video area (a quarter of its height, at most 200 pixels), after 0.4 s so that it does not flash between files; the About box shows the wordmark, dark or light after the dialog's palette | `shell/qml/Main.qml`, `About.qml` |
| The logo in other skins (D10) | A `CMD_POPUPMENU_ETC` button in a title bar (`IsTitleBar`) is PotPlayer's logo button (Daum's, in BlackSpace). RVP draws its symbol there instead of the sprite, in four states (normal, lighter on hover, a pixel lower when pressed, grey when disabled); the button keeps its place, commands and tooltip, and the `.dsf` is not changed. A skin can keep its own picture with `RvpLogo='1'` (RVP Default does). Seen in a grab of BlackSpace on the Windows VM. The "Daum" in the background of BlackSpace's F7 panel stays: it is part of a large background sprite, and the owner does not mind | `crates/skin/src/skin.rs` (`logo_slots`, `replace_sprite`), `paint.rs` |
| RVP Default | The classic layout and commands: title bar (aspect menu, the wordmark as the options button, title, window buttons), borders, the playlist docked on the right with its splitter and handles, the bottom bar (seek bar, mute and volume, file type, codecs and bitrates, the time with its remaining-time and clock pages, previous, play or pause and next, audio, subtitles and playlist buttons, INFO, EQ, VIDEO and PANEL toggles, the video page with colour sliders), the fullscreen bars, the playlist (album tabs, rows, toolbar, scroll bar) and the control panel's five pages. Its own look: graphite bars, copper and amber accents from the logo, rounded flat buttons, 40 icons and the caption font RVP Caps, all drawn for RVP; labels name Segoe UI (the system sans on Linux). 180 commands, 164 carried out or shown; the 16 others are kept for the classic layout and listed with their reasons | `crates/look/art/skin`, `docs/coverage/rvp-default.md` |
| Build and shipping | `crates/look` draws the SVG subset its art uses (no SVG library: a strict XML reader and `zeno`, already in the tree through `swash`), packs the sprites of `sprites.xml` into four atlases (`ScrollBar.bmp` at PotPlayer's fixed places), fills the markup's placeholders and writes a `.dsf` whose bytes depend only on the sources. `shell/build.rs` builds it into `rvp`; `cargo xtask look DIR` writes it and the icons for packages and reviews. Nothing built is committed | `crates/look/src/{svg,xml,font,sprites,skin,outputs}.rs`, `xtask` |
| Default skin | With no skin set RVP shows RVP Default (D12); `--skin=` with no path forces it (the kit's check); a skin that fails to load falls back to it with a message. The settings page says so | `shell/src/player.rs`, `Settings.qml` |
| Engine | Page tabs: a `CMD_SELECT_PAGE` button with `DownFlag` shows its pressed state while its page shows (BlackSpace's page buttons have no `DownFlag`, so it looks as before) | `crates/skin/src/paint.rs` |
| Sources file | Every asset with its origin; nothing from PotPlayer's skins, no third-party icons or fonts | `crates/look/art/SOURCES.md` |

## Tests

- `rvp-look` unit tests: the XML reader, the SVG renderer (fills,
  opacity, gradients, symbols, `currentColor`, clipping, blur, what it
  rejects), the icon files, the font, the placeholders and the packing.
- `crates/look/tests/default_skin.rs`: the archive loads without warnings
  and builds the same bytes twice; geometry snapshots and golden renders
  (no fonts, so no text) of the main window at 960x540, 1280x720 and
  1920x1080, the video page, the fullscreen bars and the five control
  panel pages; renders may differ by 3 levels per channel between systems.
- `crates/skin/tests/synthetic.rs`: logo buttons take RVP's picture,
  marked ones keep theirs; page tabs show the current page.
- `crates/app/tests/coverage.rs`: `docs/coverage/rvp-default.md` is
  current, and only the 16 listed commands are not carried out.
- The self-test checks that the logo resources load (`logo_resources`).

## Checks on the VMs

| Machine | Result |
|---------|--------|
| Windows VM (no PotPlayer installed) | `tools/windows-vm/m7-all.ps1`: the functional self-test with no skin set runs on RVP Default (its F7 panel is 320x178; colour, equalizer and page checks pass), then with the test copy of BlackSpace, whose title bar shows RVP's symbol; `rvp.exe`'s icon as Windows extracts it is RVP's. Final build: three runs, 0 failures in all six self-tests. Two earlier builds failed once each because of the album bugs below |
| Ubuntu VM | `kit.sh m2build m7 m6`: `m7 PASS` (the functional self-test on RVP Default with `--skin=`, 0 failures, `logo_resources`, `tab_click_shows_album` and the panel checks pass) and `m6 PASS` with BlackSpace. A grab shows the skin over the kit's 1080p clip with the system's sans font for the labels |

## Album bugs found by the checks

Both came from goal H's end-of-file handling and showed only under the
Windows VM's timing; both are fixed in `crates/app/src/app.rs`.

| Bug | Cause and fix |
|-----|---------------|
| The album skipped an entry: after the 3-second fixture the third file played, not the second (one run in four) | `play_entry` cleared "end handled" when it asked for the next file, while the model still held the old file's end; when RVP looked again before mpv reported the new file, it moved on a second time. RVP now ignores the old end while a file loads (`Tracking::Loading`). `tests/album_end.rs` reproduces it every time by looking without pause (it fails without the fix) |
| The album stopped at the end of the short fixture after the normalizer was switched off and on 0.7 s before its end (one run) | The resync seek after the first change reached mpv after the end, which leaves mpv's `eof-reached` false (goal H's pitfall); RVP waited only in the last 0.5 s. Changes of RVP's audio filters now wait in the last 1.5 s (`NEAR_END`) |

## Found by the owner after the hand-over

In fullscreen the picture turned black (the sound played on) as soon as the
pointer moved over it. RVP Default's `MainSkinAreaOscInside` had a video
slot (`Contain='CMD_VIDEO_WINDOW'`); in fullscreen the skin lies over the
video, the slot counted as a bar (so the pointer over the picture showed
the bars), and the engine painted the slot black. Fixed three ways: the
skin's fullscreen area has no slot (as in BlackSpace); a video slot in
`MainSkinAreaOscInside` draws nothing (`paint.rs`, synthetic test); and it
is not a bar (`Chrome::bar_rects`, chrome test). `--selftest=upright` now
also checks the picture with the pointer over it in the window, and in
fullscreen without and with the bars; the kit's `m7` runs it on RVP
Default (Ubuntu VM: all four PASS). Windows VM grabs show no video, so
they cannot catch it there.

## Notes

- The owner asked whether VLC, mpv and even classic Winamp skins are
  planned: VLC skins and mpv's on-screen controllers are M9 (goal K),
  which starts with a study; classic Winamp skins (`.wsz`) were added to
  that study (PLAN.md, M9 and goal K's `/goal` text).
- Rendering uses the platform's `sin` and `cos` (through `zeno` for arcs),
  so a pixel can round apart between systems; the golden comparisons
  allow for it.
- BlackSpace's renders for the comparison page stayed local
  (`C:\Users\<user>\rvp-test\m7-review\skin`, `~/rvp-reference`).

## Open items

- M6's Ubuntu boot run (above).
- M3's one-week trial, if the owner still wants it before M8.
