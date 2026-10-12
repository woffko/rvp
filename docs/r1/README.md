# R1: PotPlayer reference (goal C, 2026-10-10)

PotPlayer 260819 (64-bit, English UI) with BlackSpace_en.dsf in classic mode,
the playlist attached, on the owner's Windows host: one 2560x1440 monitor
at 100% scaling (checked: Qt reports a device pixel ratio of 1). Captures
and raw dumps are local only, in `~/rvp-reference/r1/`; this folder
holds the tables and findings.

## Files

| File | Content | Source |
|------|---------|--------|
| `menu-static.tsv` | The context menu as built from PotPlayer's resources: 894 items (88 submenus, 134 separators), English text, shortcuts as shown, command IDs | `tools/r1/potplayer_tables.py` (read-only: `PotPlayer64.dll` RT_MENU 101, `Language/English.ini`) |
| `menu-playlist.tsv` | The playlist panel's context menu as built from PotPlayer's resources: 72 items, same columns as `menu-static.tsv` (goal G) | `tools/r1/potplayer_tables.py` (read-only: `PotPlayer64.dll` RT_MENU 111) |
| `menu-runtime.tsv` | The live context menu while a video plays: 1580 items, including run-time lists (skins, languages, audio renderers, devices). Recent files and album names are replaced by placeholders | `host-session r1`, then `tools/r1/redact_menu.py` |
| `hotkeys.tsv` | Default hotkeys: accelerator table 157 (main window, 259 keys) and 118 (playlist, 54 keys), with the menu items they trigger | `tools/r1/potplayer_tables.py` |
| `mouse-actions.tsv` | Preferences → General → Mouse: each action, its current value, its index and all choices; the page's check boxes | `host-session r1` (Win32 messages, read-only) |

## Hotkeys

229 of the 259 main-window keys and 47 of the 54 playlist keys have a name:
from the popup menu item with the same command ID, from another menu, or
from the shortcut that the menu shows next to an item (PgUp/PgDn dispatch
by context: previous/next item, chapter or channel). The rest are listed by
command ID. The owner's `MainShortCutList` is empty, so these defaults are
what the owner uses. Examples: Enter and Alt+Enter = Fullscreen (Keep AR),
Ctrl+Enter = Fullscreen (Stretch), Space = Play/Pause, Up/Down = volume,
Left/Right = seek, M = mute, F5 = Preferences, F6 = Playlist.

## Mouse actions

The registry codes are indexes into each action's list of choices:
`MouseLeftSClick=4` is "Play/Pause playback" and `MouseLeftDClick=1` is
"Fullscreen (Keep AR)/Restore"; actions without a stored value use entry 0,
the "(Recommended)" default of that action. Owner's values:

| Action | Value |
|--------|-------|
| Left button, single click | Play/Pause playback (4) |
| Left button, double click | Fullscreen (Keep AR)/Restore (1) |
| Left button, double click in the left or right area | Default operation |
| Left button, hold | Do nothing |
| Right button, single click | Context menu |
| Right button, double click / hold | Do nothing |
| Wheel button, single click | Fullscreen (Keep AR)/Restore |
| Wheel up / down | Increase / decrease volume |
| X button 1 / 2, single click | 5 s backward / forward |
| Other X-button and wheel-button actions | Do nothing |
| "Operate the action 'Left Button - Single Click' in the video area only" | on |
| Pan/zoom with modifier drag, subtitle position drag, wheel with X button, "Prevent window from being dragged outside of title bar" | off |

## Captures (local)

Idle (no file), at exactly 1280x720, 1920x1080 and 2560x1080: plain, the
pointer over the play button, and the play button pressed (the button is
released outside itself, which cancels the click; PotPlayer did nothing).
Playing (the synthetic 1080p23.976 clip, SVP active), at widths 1280, 1920
and 2560 with the same three states, plus the top of the context menu and
the Preferences dialog.

## Findings for RVP

1. **While a video plays, PotPlayer keeps the window at the video's aspect
   ratio.** Setting 1280x720, 1920x1080 and 2560x1080 gave 1280x654,
   1920x1014 and 2560x1374. With the playlist attached, BlackSpace's chrome is
   310 px wide and 108 px high, and the video area keeps 16:9:
   `height = (width - 310) * 9 / 16 + 108`. Idle windows take any size.
2. **Opening a file resizes the window to show the video at 100 %**
   (1920x1080 video → a 2230x1188 window).
3. **The window position is remembered per state.** PotPlayer starts idle
   at the idle position and size it saved, and saves its current window
   into `HKCU\Software\DAUM\PotPlayerMini64\Positions` on exit (main window,
   video window and audio-mode window rectangles, rewritten as a whole).
4. **The live context menu is the static menu plus run-time lists**;
   M4 can build the static tree from `menu-static.tsv` and fill the lists
   from RVP's own state.
5. **Normalizer settings confirmed** on the Preferences page Audio →
   Normalizer/Freeverb: Enable on, "Operate only with video playback" off,
   method "Tomsteady 32-bit floating point arithmetic (Recommended)", Boost
   audio on, Level 75 %, Release time 10.

## Side effects on the host and their cleanup

PotPlayer saves its state on every exit. After each session the
`Positions` key was compared with a snapshot taken before it and the
changed values were written back (with the owner's permission), so it is
identical to the snapshot. `MInfo1`/`MInfo2` (encoded values PotPlayer
rewrites on every start) and the root `(Default)` value (it flipped between
0 and 1 on each start) were left as PotPlayer wrote them. The synthetic clips were added to
PotPlayer's Default playlist and history; the owner removes them. SVP built
and cached a TensorRT engine for RIFE (kept at the owner's choice).
