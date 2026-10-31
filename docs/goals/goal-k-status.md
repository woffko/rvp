# Goal K status: milestone M9 (other players' skins)

Date: 2026-10-31. Result: **done**. The three parts the owner chose are
built, checked in WSL and on the Windows VM. The VM took no new
connections from about 09:33 UTC: another session's `route add` on the VM
had cut SSH and ping (nothing of RVP's). The SSH session that was
already open ran mpv's OSC and uosc there; the owner restarted the VM at
about 20:58 UTC, and the rest ran then.

Precondition met: `goal-r-status.md` records goal R as done.

## The owner's decisions

The study is `docs/spikes/s6-other-skins.md` (its last section records
the answers of 2026-10-31):

- **Scope: all three parts**, in the study's order: mpv's on-screen
  controllers, classic Winamp skins (with the shared window host), VLC's
  themes.
- **RVP's own skin stays.** A controller is one more choice: over the
  skin's video area (in fullscreen in the place of the skin's bars), or
  without the skin (the window holds the video and the controller).
- **Controllers: mpv's OSC and the user's own scripts.** The owner allowed
  running the downloaded uosc and ModernZ in tests, headless in WSL and on
  the Windows VM.
- **Winamp's video: a window docked under the main window**, framed by
  the skin's `video.bmp`, else its playlist frame's pieces, else RVP's own
  art; fullscreen shows the video alone.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Study | VLC skins2 (VLC 3's source, the gallery's 20 most downloaded and best rated themes plus VLC's default), mpv's OSC, uosc and ModernX/ModernZ (a headless probe of the OSC through libmpv's render API), classic Winamp (Winamp 5's released source for facts only, Webamp, 25 skins of the Winamp Skin Museum). | `docs/spikes/s6-other-skins.md`; notes and downloads in `~/rvp-reference/s6` |
| mpv's controllers | mpv's OSC or the user's scripts from `<settings>/mpv` (`scripts`, `script-opts`, `fonts`) over the skin's video area or without the skin, chosen on the Skins page. The pointer over the video goes to mpv; clicks no controller takes come back through RVP's bindings in mpv's `default` section and act as RVP's pointer; a controller's menu takes the keys while open; the window's state goes both ways (fullscreen, maximized, minimized, on top); the OSC's buttons (mpv 0.40+) run RVP's actions. RVP's helper cores (thumbnails, encoding, the second audio device) no longer load mpv's built-in scripts. | `crates/app/src/controller.rs`, `shell/src/player/m9.rs` |
| Window host | A skin's `RvpWindow='1'` areas are windows of their own (`SkinWindow.qml`, `shell/src/player/extras.rs`): docked above, below or beside the main window and moved with it, shown and hidden by `RVPWIN:` commands, shaped by polygons or by their pictures' opaque pixels. | `shell/qml/SkinWindow.qml`, `shell/src/player/extras.rs` |
| Classic Winamp | A `.wsz` in the skin list becomes an RVP skin built in memory: the video window (RVP's main window) and the main window, equalizer and playlist docked to it. Every BMP variant the skins use; missing bitmaps fall back to RVP's own art; the engine's extensions (picture-per-value sliders, bitmap fonts, the analyzer's colours, lit toggles, the equalizer's curve, window shapes). | `crates/skin/src/winamp`, `crates/app/src/classic.rs` |
| VLC's themes | A `.vlt` (tar.gz or zip) becomes an RVP skin built in memory: the window with the video is the main window with each layout a file of the skin, the other windows beside it; `visible` and checkbox states are conditions RVP computes, texts take VLC's `$` codes (also in tooltips), the theme's fonts are registered for the process; controls follow their corners as the window resizes; buttons take the pointer where their pictures are opaque, as VLC's do. | `crates/skin/src/vlc`, `crates/app/src/vlc.rs` |
| Coverage | `docs/coverage/vlc.md`: VLC 3's 53 actions (49 run in RVP), the window actions, variables, text codes, slider values and how RVP draws a theme's parts. Its test checks that every mapped action exists in RVP. | `crates/app/src/coverage.rs` |

## Tests

- In WSL under Xvfb (silent):
  - `rvp --selftest=m9 --controller=osc`: 15 of 15 (Ubuntu's libmpv 0.37
    in the sysroot); `--controller=scripts` with uosc: 17 of 17; with
    ModernZ (`--controller-bar=0.915`): passes.
  - `rvp --selftest=classic --skin=SKIN.wsz`: 10 of 10 with Winamp's base
    skin 2.91 and with Garfield.
  - `rvp --selftest=vlc --skin=THEME.vlt`: 10 of 10 checks with each of
    the 21 themes of the sample (PsVLC, which has no pause button, skips
    the two pause checks).
- Headless: `tests/winamp.rs` (the synthetic skin; with `RVP_WINAMP_SKINS`
  all 25 museum skins load without decoding warnings), `tests/vlc.rs`
  (with `RVP_VLC_SKINS` every layout of the 21 themes builds and loads),
  `crates/app/tests/chrome.rs` (VLC's controls: opaque-pixel hits, the
  guarded toggle, the title bar's double click), the coverage test, and
  unit tests for controllers, the markup's XML mode, VLC's expressions and
  variables.
- Renders: `cargo run -p rvp-skin --example render_skin` draws a `.wsz`
  or `.vlt` window headless; the renders checked against the skins'
  pictures stay in `~/rvp-reference/s6`.

## Checks on the Windows VM

`tools/windows-vm/m9-all.ps1` (the bundle, RVP's libmpv with LuaJIT, and
`C:\rvp-test\m9`: the test file, uosc and ModernZ in RVP's mpv-folder
layout, `winamp\*.wsz` and `vlc\*.vlt` from the study's sample):

| Run | Result |
|-----|--------|
| mpv's OSC (mpv 0.41's, with RVP's commands on its buttons) | 17 of 17, three times |
| uosc | 17 of 17 in the morning and alone after the restart; in the two full runs just after the restart one double-click step timed out each time (entering fullscreen once, leaving it once), see "Findings" |
| ModernZ (`--controller-bar=0.915`) | 15 of 15, twice |
| Classic Winamp: base 2.91, Garfield, Yufo06 | 10 of 10 each |
| Classic Winamp: Zelda Amp 3 | 9 of 10 in the first run (the play click resumed playback later than the step's 5 s), then 10 of 10 twice alone |
| VLC: VLC's default, PsVLC, Blissta, Dalin Media Player, WMP11, Alienware Darkstar, Darkvoodoo, darklounge, eDark, Win VLC 11 | 10 of 10 each |

## Findings

- **Fonts before the application:** the first skin loads on a thread of
  its own while Qt starts; `QFontDatabase::addApplicationFontFromData`
  there, before `QGuiApplication` existed, crashed RVP now and then. The
  theme's fonts now wait for the window's thread (`keep_skin`).
- **VLC's XML is real XML:** a quoted `>` (`tooltiptext="Show Info >>"`)
  ended the tag in PotPlayer's tolerant reading and lost the rest of
  Dalin Media Player's controls; `markup::parse_xml` keeps the quotes.
- **VLC hit-tests by pixels:** WMP11 stacks window-wide button pictures,
  each opaque only where its button shows; a rectangle hit test gave every
  click to the top one (`RvpHit='alpha'`).
- **mpv's `-append` options** do not exist for `mpv_set_option_string`;
  whole values are set. uosc closes its menu by setting `user-data` to
  `null`, which an observation does not report: menus are read when a key
  comes. mpv makes a double click of any two presses: RVP takes one only
  when both presses were its own.
- **Double clicks through mpv need time:** over a controller both
  presses go through mpv and come back as RVP's double click. On the VM
  (two CPUs, software OpenGL) right after its restart, while Defender
  scanned the new files, uosc (the heaviest of the three controllers)
  missed one double click per run; alone on the quiet VM, in WSL (three
  runs) and in the morning it passed. A loaded machine can drop such a
  double click; the single clicks and the controllers' own buttons are
  not affected.
- **Names the builder used that RVP lacks:** `RVP:open-playlist` and
  `RVP:save-playlist` are playlist actions; playlist actions now take
  `RVP:` names too, and the coverage test fails a mapped action RVP does
  not have.

## Differences from the originals

- **mpv's controllers:** mpv's window dragging and the OSC's idle screen
  stay off (RVP moves its window and shows its logo); older OSCs and the
  other scripts change the window through mpv's options, which RVP
  follows.
- **Classic Winamp:** windowshade mode and double size do nothing; the
  balance and the equalizer's preamp have no counterpart in RVP (their
  sliders stay in the middle); windows do not snap or undock; the
  skin's cursors are not used.
- **VLC:** `docs/coverage/vlc.md` lists every item. In short: windows do
  not snap (`Anchor`), animated bitmaps show their first frame, texts
  wider than their box are cut rather than scrolled, curved sliders
  become straight, the playlist is RVP's in the theme's colours, the
  fullscreen controller is left out, `dvd.nextTitle()`/`previousTitle()`
  and the streaming wizard do nothing, and the equalizer's preamp moves
  nothing.

## Linux items for goal S

- **Lua in the kit's libmpv:** the controllers need libmpv with Lua and
  mpv's built-in scripts (the OSC); Ubuntu's libmpv has them, the
  packages' bundled libmpv must keep them.
- **Windows of their own on Wayland:** Winamp's and VLC's other windows
  are placed by RVP beside the main window and moved with it; Wayland
  lets no client place its windows, so docking and moving the group need
  XWayland or another way. Window shapes (`QWindow::setMask`) need the
  same check there.
- **The OSC of Linux's libmpv** (0.37 in the sysroot) has no button
  commands (mpv 0.40+): its buttons go through mpv's window options.
- **The kit's steps:** `--selftest=m9`, `=classic` and `=vlc` with the
  local sample skins, and a check of the packages with a controller.

## What the owner should try

On the next build:

1. Skins page, "mpv's on-screen controller": mpv's OSC over the video,
   then "without the skin"; then uosc or ModernZ unpacked into
   `%APPDATA%\RVP\mpv` (as into mpv's own folder).
2. A classic Winamp skin (`.wsz`, e.g. from the Winamp Skin Museum) and a
   VLC theme (`.vlt`, VLC's skins page) put into RVP's `skins` folder and
   chosen from the skin list: the windows, their buttons, the equalizer,
   the playlist and moving the group.

## CI

Run 41200210267 for ed480f8 to 21a065c and run 41208660485 for 028e4b1:
green, with the checks on Ubuntu 24.04 and Windows Server 2025, both
packages and `cargo deny`.

## Open

- Nothing for goal K. The Linux side is goal S's (above).
