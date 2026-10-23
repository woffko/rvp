# Goal P status: milestone M14 (PotPlayer parity: preferences, skins and the rest)

Date: 2026-10-23. Result: **done**. M14 is built and checked in WSL, on
the Windows VM and in one host session that put PotPlayer's and RVP's
skin modes side by side. Every item of R1's menus and hotkeys and every
option of R2 works in RVP or keeps a reason, and no coverage report says
"not in RVP yet" or "planned" any more. The final build is in
`C:\Users\<user>\rvp-test\rvp-p`.

Precondition met: `goal-o-status.md` records goal O as done. The owner
reported no findings from goal O's build, so nothing came before M14.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Keyboard and Mouse | PotPlayer's Keyboard and Mouse pages: default hotkeys off (the user's own keys stay), the keyboard's media, volume and browser keys, the media keys while another window has the keyboard (Windows' hotkeys), the left click in the video only, moving the window by its title bar only, Alt, Ctrl and Shift drags that pan, zoom and stretch, Ctrl+Alt drags that move the subtitles, and the wheel with X1 or X2 held. | `crates/app/src/input.rs`, `pointer.rs` |
| Skins | The Skins menu lists RVP Default and the `.dsf` skins of RVP's skins folder, the chosen skin's folder and PotPlayer's, and switches at once. Files without video show the skin's audio skin (`AudioSkin.xml`); each skin mode can keep its own window; the video skin at start and locking the mode. Colour themes: the skin's `ColorTheme` list, or a custom HSB or RGB theme (Customize...). The window's and the control panel's opacity. Skin Settings. | `crates/app/src/skins.rs`, `ColorTheme.qml` |
| Logos and the on-screen control | The skin's `<Logo>`, RVP's, or a picture of the logos folder, while idle and while a file starts. The bars over the video (`MainSkinAreaOscInside`) can be turned off, made translucent and hidden after a while; "Auto hide main skin" (the skins' Floating Window button) shows the window that way while a video plays; in fullscreen the video can make room for the bars. | `logos.rs`, `skins.rs`, `crates/skin` |
| Context menu | Context Menu Design: RVP's whole menu or a menu file in PotPlayer's format (RVP's menus folder, and PotPlayer's on Windows, read at run time). Context Menu Style: RVP's look, the system's menus, or the skin's first or second style, dark colours with a dark system, a transparency and a size; Context Menu Items hides top-level items. Zoom Skinned Windows. | `menu_design.rs`, `Menus.qml` |
| OSD and info | RVP's notices: on or off, alignment, margins, fonts, sized with the video, drawn in the picture. Scroll Lock: the misc info line (time, percent, frame, clock, playlist position, file, chapter, battery). Shift+Tab: the short info notice. Ctrl+F1: Playback/System Info (file, video, audio, rendering, system), copied to the clipboard. | `osd.rs`, `SystemInfo.qml` |
| General, When Starting, Dialog Box | Each start opens its own window or hands its files over; the tray icon; Windows' taskbar progress, thumbnail buttons and jump list; the system's media controls; the start with the session. Pausing while minimized, without the focus or with the screen locked. Window resizing, snapping and starting place; tooltips; the dialogs' font, size and staying on top. | `general.rs`, `shell/cpp/taskbar.cpp`, `startup.rs` |
| Association and Screen Saver | Association lists RVP's types and the user's own; Apply to System registers them for the current user (ProgIDs, Open with, capabilities, verbs, autoplay handlers) and removes what is turned off; RVP never makes itself a type's default. `rvp.scr` starts RVP as Windows' screen saver on the page's folder or file. | `registration.rs`, `screensaver.rs` |
| Presets | Configuration (Misc > Manage Presets): the default preset is settings.json's values, each other preset a whole copy of the settings; add, duplicate, activate, move, rename, delete; reopen the file on activation, copy into the default, the default at every start; Auto-Preset conditions (file name, album, power source, duration, video size, codec, rate and aspect, audio format, rate and channels, audio only, subtitles). `/config="name"` starts with a preset. | `presets.rs` |
| Accessibility | Notices, text subtitles and tooltips read aloud (Windows' speech, SAPI), announced to screen readers (Qt's accessibility) or shown as the window's title. Touch: swipes over the video seek or change the volume; the skin's touch layout (`SkinType=Touch`). The Xbox controller (XInput). | `accessibility.rs`, `shell/cpp/tts.cpp`, `xinput.cpp` |
| Interface languages | English and Russian, written for RVP: QML strings through `qsTr`, menu and action labels through `Tools.tr_menu`; `shell/i18n/*.ts` compiled by Qt's lrelease at build time. General > Interface language and Misc > Interface Language (Shift+L): the system's, English or Russian. | `shell/i18n`, `Main.qml` |
| Settings window | PotPlayer's tree with RVP's pages as tabs, now with a search (any option or heading, in the interface's language; the match is shown and marked) and Reset Page (the page's options back to RVP's defaults, after a question). | `Settings.qml`, `preferences::reset` |
| Misc | Copying the elapsed and total times, the file's name and path; removing the history; Disable default hotkeys; the playlist's Create Thumbnail Image(s) (a contact sheet of each selected entry); the Whisper models' folder. | `capture.rs`, `albums.rs` |

## Coverage

`docs/coverage/` is rewritten. `tests/coverage.rs` now fails any report
that says "not in RVP yet", "planned in M14" or "(goal P)".

| Report | Before goal P | Now |
|--------|---------------|-----|
| R2's Preferences options (1040) | 422 in RVP | 631 in RVP, 409 with a reason |
| Context menu items with a command (672) | 522 carried out | 541 carried out, 131 with a reason |
| Playlist menu (54) | 53 | 54 |
| Live menu's run-time commands (720) | 101 carried out, 421 in RVP's lists, 198 not | 148 carried out, 481 in RVP's lists, 91 with a reason |
| BlackSpace's live commands (257) | 91 not carried out | 76: Daum's broadcast and chat (54), the broadcasting capture areas (21), the subtitle font menu (a dialog in RVP) |
| RVP Default's commands (180) | 16 not carried out | 1 (the subtitle font is a dialog, not a menu) |

The reasons that stay:

- DirectShow's renderers, filters and output surfaces (libmpv decodes
  and draws with FFmpeg and libplacebo).
- TV tuners: FFmpeg cannot tune them; Daum's broadcast and chat services
  and PotPlayer's broadcasting tools: the plan's non-goals.
- PotPlayer's file navigator, Open Box and seamless playback (goal M's
  study).
- The update service and the website: the repository is private.
- The built-in Korean interface and PotPlayer's other language files.
- DVD and Blu-ray menus: goal R.
- Online subtitles: deferred by the owner.

## Tests

- `crates/app/tests/coverage.rs`: the reports are current and have no
  pending reasons; `settings.rs`: presets round trip; `translations.rs`:
  every QML string and menu label has a Russian translation with its
  placeholders; `menus.rs`: the Skins menu, menu files and styles;
  `bindings.rs`: M14's keys.
- `crates/app/src/preferences.rs`: Reset Page puts back only the page's
  options.
- `crates/skin/tests/synthetic.rs`: logos, menu files and styles, the
  touch layout, and the colour themes checked pixel by pixel with an
  excluded area.
- `crates/look/tests/default_skin.rs`: golden renders of RVP Default in
  an HSB and an RGB theme (`tests/golden/theme-*.png`). BlackSpace's
  renders stay local.
- `rvp --selftest=m14` (48 checks in WSL with RVP Default and with
  BlackSpace): skin switching, themes, opacity, the audio skin and its
  window, auto hide and the bars over the video (only the one under the
  pointer), copied times, menu files and styles, the skin zoom, the info
  items and the info window,
  pausing when minimized or without the focus, presets and Auto-Preset,
  messages as the title, swipes, the voices, the touch layout, Russian
  loading and fitting the settings pages, the search and Reset Page, the
  entries' thumbnail images, and on Windows with `--registry-check` the
  Run key and the file types.
- `rvp --selftest` (87 checks) passes in English and in Russian;
  `--selftest=upright` passes with BlackSpace. With RVP Default its
  picture check sees a black grab in WSL's Xvfb, as it already did at
  goal O's last commit (checked on a build of it); the Ubuntu VM is where
  that check runs (kit step m7).
- `cargo xtask ci` passes before every commit.

## Checks on the Windows VM

`tools/windows-vm/m14-all.ps1` on the final build: 102 checks pass.

- The M14 self-test with RVP Default and with BlackSpace: every check
  passes. Windows' voices are listed (David and Zira), the Run key is
  written and removed, the 77 file types are registered and removed.
- The registry after the runs: no Run value and no RVP type left.
- Headless tests on Windows: `settings` (6), `translations` (3), `menus`
  (8) and `synthetic` (18).

The earlier milestones' checks on the same build pass too:
`m7-all.ps1`'s functional self-tests with RVP Default and BlackSpace
(178 checks) and `m11-all.ps1` (47 checks, its headless tests with
Windows' FFmpeg).

## The host session (2026-10-22 and 23, with the owner's permission)

`tools/windows-vm/m14-host.ps1` with host-session's `m14` put PotPlayer
and then RVP (`rvp-p`, on a channel of its own, with its settings in the
session's folder) through the same steps with BlackSpace: idle, the
video at 1280x720, fullscreen by a double click with the pointer at the
bottom, and an audio file. PotPlayer's registry key and playlist folder
were saved before each run and written back after it; the two values
PotPlayer added during the runs (`ForceMinWidth` and `ForceMinHeight`
under `Settings`) were deleted afterwards, with the owner's permission.
The captures are in `~/rvp-reference/m14` (never committed: they
show the owner's PotPlayer playlist).

There were three runs. In the first, the terminal that ran the tool
covered RVP's window, so RVP's captures and its double click were lost;
the tool now keeps each captured window on top, and RVP ran again alone.
A third, PotPlayer-only run captured fullscreen with the pointer at the
top and resting in the middle.

| Mode | PotPlayer | RVP before | RVP now |
|------|-----------|------------|---------|
| Idle | its logo | RVP's logo (D10) | the same |
| Video, the window set to 1280x720 | keeps the width: 1280x836 | keeps the height: 1126x720 | the same (a difference) |
| Fullscreen, pointer at the bottom | the control bar only | the control bar and the title bar | the control bar only |
| Fullscreen, pointer at the top | the title bar only | - | the title bar only |
| Fullscreen, pointer resting in the middle | no bar | - | no bar |
| Audio file | the audio skin, the window shrinks to 370x186 | the audio skin in a 1126x720 window | shrinks to 360x186 |

Two fixes came from it: over the video only the bar under the pointer
shows (`Player.shown_bar`), and "Keep the window's size while switching
between skin modes" is now off by default, as PotPlayer behaves. SVP
attached to the first run's RVP through `mpvpipe` (its filter does not
run on the 96x72 clip); the later runs had no IPC server.

## Findings

1. The QML engine keeps its last translation when `Qt.uiLanguage` names
   a language without a `.qm` file: Russian stayed after a switch back
   to English. English is now the empty language.
2. `KeysAndMouse.qml` called `Tools` without importing `RvpNative`: the
   keys page's action names failed to show (found in the self-test's
   log, fixed before it reached a build).
3. Two Windows-only placeholders did not fit in Russian; the VM's
   self-test caught them.
4. Turning every type off left RVP's AutoPlay ProgID behind; it now
   exists only while an autoplay handler needs it.
5. On Windows the whisper.cpp engines RVP finds are listed by name; the
   translation test now treats them as names.
6. A double click into fullscreen did not update the bars over the video
   until the pointer moved (fixed with the overlay's sync).
7. A menu file whose items RVP lacks gave an empty menu; it falls back to
   RVP's whole menu.
8. The host session: over the video RVP showed both bars where PotPlayer
   shows only the one under the pointer, and RVP kept the window's size
   on the audio skin where PotPlayer shrinks it (both fixed, see above).
   The first run lost RVP's captures to the terminal over its window; the
   tool now keeps the captured window on top.

## Differences from PotPlayer

- Colour themes: PotPlayer's formula was not recorded; RVP's HSB themes
  keep each pixel's lightness and take the theme's hue and saturation,
  RGB themes add offsets.
- The interface speaks English and Russian only. Notices RVP formats
  itself (the OSD texts) stay English in the Russian interface.
- Presets keep whole copies of RVP's settings; PotPlayer keeps registry
  snapshots.
- Touch: swipes seek and change the volume; no pinch.
- The global media keys are Windows' hotkeys on RVP's window; with the
  system's media controls on, Windows already sends the media keys to
  RVP.
- `ID_VIDEO_MARGIN`, a skin command no checked skin uses, is not carried
  out.
- A window set to a size from outside keeps the video's shape by
  narrowing (RVP) where PotPlayer grows its height; BlackSpace's audio
  window is 360 pixels wide in RVP (the skin's `WidthLimit`) and 370 in
  PotPlayer.
- The settings window keeps RVP's own look; the search covers RVP's
  pages and headings, not PotPlayer's page names.

## Linux items for goal S

- The tray icon (Qt Labs' platform tray needs a `QApplication` on
  Linux).
- Pausing when the screen locks (logind's `Lock`/`Unlock` signals).
- Text to speech (speech-dispatcher) and the Xbox controller (evdev or
  SDL).
- File types: the desktop entry and `xdg-mime` instead of the registry;
  the screen saver has no Linux counterpart.
- The taskbar's progress and buttons (the Unity launcher API where a
  desktop offers it) and the recent files (GTK's recently-used list).
- The global media keys: MPRIS already takes them on GNOME.

## What to try

In `C:\Users\<user>\rvp-test\rvp-p` (portable; `whisper-models` has the
tiny and base models from rvp-o):

1. **Skins**: the Skins menu lists RVP Default and PotPlayer's skins;
   switch while a video plays; open an audio file (the audio skin);
   Skins > Color Themes, and Customize...; Opacity.
2. **On-screen control**: in fullscreen move the pointer to the bottom
   edge (the control bar), then to the top edge (the title bar); Skins >
   Auto hide main skin while video screen is active does the same in the
   window. An audio file shrinks the window to BlackSpace's audio skin.
3. **Languages**: Misc > Interface Language > Русский (or Shift+L), then
   the menus and the settings window.
4. **Settings**: F5, type a word into "Find an option"; Reset Page on a
   page you changed.
5. **Presets**: Misc > Manage Presets..., add one, change the volume
   step, switch back to the default.
6. **Info**: Ctrl+F1, Shift+Tab, Scroll Lock.
7. **Context menu**: F5 > General > Skins: Context menu style (the
   skin's style, dark colours) and Context menu design (PotPlayer's menu
   files).
8. **Playlist**: select entries, then Create Thumbnail Image(s)... from
   the playlist's menu.
9. **SVP**: a film with SVP 4 Manager running, as with rvp-o.

## CI

GitHub CI passed for 25eae86 (f3a0a29 to 25eae86, run 39677126908) and
for bab4d16 (2551506 to bab4d16, run 39893261483: the checks on Ubuntu
and Windows, cargo deny, both packages, and the artifacts' pruning).
This file's commit follows them.

## Open

- Nothing of M14 is open.
- Other `.dsf` skins: the owner named none. RVP lists and loads the 23
  skins of PotPlayer's Skins folder; a coverage report for any of them
  is one test run away (`RVP_TEST_SKIN`), kept local like BlackSpace's.
- SVP with `rvp-p` on a real video was not part of the session (the
  first run's SVP attached to the 96x72 clip, where its filter does not
  run); the owner's tries will show it.
- The owner's tries of the build may bring findings for the next goal.
