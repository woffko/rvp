# Goal G status: milestones M4 (menus and commands) and M5 (settings window)

Date: 2026-10-12. Result: **done**. All tests and checks pass on WSL, the
Ubuntu VM, the Windows VM and the Windows host. On the host, goal F's
17-18 dropped frames were traced to their steps by measuring goal F's
build. The new M4 and M5 steps' costs were measured, and the part RVP
caused was fixed (see "Windows host"). The goal asked for one host run.
The owner approved four: the new build, the build with the fixes, goal
F's build, and goal F's build with each step separated. Precondition met:
`goal-f-status.md` records goal F as done.

M3's one-week trial: the owner had reported no findings when this goal
started, so nothing was fixed ahead of M4 and M5.

## Summary

| Deliverable | Result | Where |
|-------------|--------|-------|
| Action table | One table maps every RVP action to its settings name, PotPlayer's menu text, the shortcut PotPlayer's menu shows, PotPlayer's numeric command IDs (menus and default hotkeys) and the skin's `CmdID`s: 123 rows, up from M3's 30 actions | `crates/app/src/action.rs` |
| New commands | Open folder, URL and subtitle; reopen; close; frame steps; jumps (start, middle, 30 s before the end, previous and next subtitle, previous and next chapter); speed (0.1x steps, default and last used); A-B repeat (set and reset points, toggle); repeat off, playlist or item; keyframe seeks; exit after the file or the list; audio and subtitle sync (steps are settings); subtitle size, position and alignment; audio, video and subtitle track cycling and selection; chapters; aspect ratios (default, original, 4:3, 16:9, 16:10, 1.85:1, 2.35:1); keep aspect, crop, stretch; zoom, stretch and move the picture (numeric keypad); save the frame with or without subtitles (PNG or JPEG, `Pictures/RVP`), open or choose that folder; stay on top (never, always, while playing); window size 0.5x to 2x and fit to the video's shape; playback information over the video (Tab); About; settings pages | `crates/app/src/commands.rs`, `shell/src/player.rs` |
| Context menu | PotPlayer's tree from R1 (`docs/r1/menu-static.tsv`, 894 items) limited to what RVP does: an item stays when its command ID maps to an action, a submenu when something in it stays, separators never lead, trail or double. Labels are PotPlayer's with the seek and sync steps filled in and "RVP" for "PotPlayer"; the shortcut is the key bound now; check marks show RVP's state. Where PotPlayer shows run-time lists RVP shows its own: audio, video and subtitle tracks, chapters, albums | `crates/app/src/menu.rs`, `shell/qml/Menus.qml` |
| Playlist menu | PotPlayer's playlist tree (R1 resource 111, now `docs/r1/menu-playlist.tsv`, 72 items) the same way, plus Undo, Redo and the PotPlayer import | same |
| Skin menus | BlackSpace's 19 `<UserMenu>`s: items name commands, built-in submenus (`CMD_POPUPMENU_*`, in place with `NoSubMenu='1'`) or separators; buttons open them with `CMD_POPUP_USER_MENU` and `UserMenuName` (right click: `AltUserMenuName`); a `CMD_POPUPMENU_*` button opens that part of PotPlayer's menu; a menu RVP has nothing for shows one disabled line "Not in RVP yet". `CMD_SELECT_PAGE` switches the skin's pages (`PageIndex`) | `crates/skin/src/menu.rs`, `crates/skin/src/hit.rs`, `crates/app/src/pointer.rs` |
| Interim menus | `ContextMenu.qml` and `ListMenu.qml` are gone; every menu is built from the same JSON (`Player.menu_json`) when it opens | `shell/qml/Menus.qml` |
| Coverage reports | `docs/coverage/menus.md`: every R1 menu item with a command, the action that carries it out or the reason it does not (context menu 138 of 672, playlist menu 17 of 54); `docs/coverage/blackspace.md`: every live BlackSpace command (257), where it appears and what RVP does with it (133 carried out or shown) or why not | `crates/app/src/coverage.rs`, `crates/app/tests/coverage.rs` |
| Settings window | F5 (PotPlayer's Preferences key, also "General Settings..." and the "... Settings..." items), loaded on first use: Playback (seek steps, keyframe seeks, resume, reopening the last file, fitting and keeping the video's shape, stay on top, end of file), Playlist (PotPlayer's playlist folder, import now, on Linux the drive map), Audio (normalizer on, Boost, Level, Release time, volume step and maximum, audio device from mpv's list, audio sync step), Video (hardware decoding, video sync, scaler, saved frames folder and format), SVP and VapourSynth (the IPC endpoint and its default; the rest waits for M6), Subtitles (languages, loading files next to the video, size, sync step), Keys and Mouse (M3's editor as a page) and Skins (the `.dsf` file, other skins next to it) | `shell/qml/Settings.qml`, `crates/app/src/preferences.rs` |
| Apply at once | Every change is checked, set on the running mpv (`hwdec`, `input-ipc-server`, `volume-max`, `audio-device`, `video-sync`, `scale` (back to mpv's default when emptied), `slang`, `sub-auto`, `sub-scale`, the normalizer chain), rebuilds the bindings and is saved; only the skin waits for the next start (the window says so). Key and mouse changes apply as they are made | `App::apply_settings` |
| Validation | Numbers out of range, unknown mpv values, bad drive-map entries and unreadable language lists are repaired with a note, at start, at import and for every change; a missing skin file is reported | `Settings::validate` |
| Portable mode | `rvp.portable` next to the executable: `settings.json` next to it and the state in its `state` folder; `RVP_CONFIG_DIR` and `RVP_STATE_DIR` still win | `crates/app/src/storage.rs` (`Dirs::resolve`) |
| Import and export | Export writes the settings file; import reads one (missing fields take their defaults, unknown ones are named and ignored, values are checked) and applies it as one change | `crates/app/src/preferences.rs`, the settings window's buttons |
| Other | The skin's clock (`CMD_SYSTEM_TIME_MESSAGE`); playback information over the video (Tab: file, video, decoder, audio, time, speed, A-B, sync, dropped frames) | `shell/qml/Main.qml`, `crates/app/src/view.rs` |
| Self-test | 14 new checks (menus with the bound keys and check marks, both menus built with the time it took, a menu command, F5, every settings page, the keys page, a setting that applies at once, the window closing); they replace M3's four Keys and Mouse window checks. Each step of M4's part that may cost video frames is followed by a 1.5 s pause and a `settled_*` line, so the drop counts show the step | `shell/qml/Main.qml` |
| Linux kit | Step `m4` (the self-test's M4 and M5 checks and a portable copy), `m4all`, checklist section 7 | `tools/linux-kit/` |
| Windows VM script | `m4-all.ps1` (the same checks, portable mode in the bundle folder, cleaned up afterwards) | `tools/windows-vm/` |

## Tests

| Area | Tests |
|------|-------|
| Action table | `action.rs`: names, IDs and skin names unique; PotPlayer IDs and skin commands map as expected |
| R1 trees against the table | `tests/coverage.rs`: every item with a command maps to an action or has a reason; the report is current (`RVP_BLESS=1` rewrites it); BlackSpace's report when the skin is there |
| Menus | `menu.rs`: both R1 trees parse (894 and 72 items), command IDs round-trip, separators are tidied; `tests/menus.rs`: the context menu's top level and paths, the bound key moving with a rebinding, check marks following settings, run-time tracks and chapters, the playlist menu, skin menus (commands, submenus in place and as entries, unknown items, the "Not in RVP yet" line), the JSON for the shell, and BlackSpace's 19 menus (each builds and contains every command RVP carries out) |
| Skin menus and commands | `crates/skin`: `<UserMenu>` parsing, the commands a skin uses with their areas (disabled lines excluded), the synthetic skin's menu; `tests/chrome.rs`: a menu button (left and right click) and a page button on the synthetic skin |
| Commands on the engine | `tests/bindings.rs`: every action bound by PotPlayer's defaults (now 80) runs once on headless libmpv with its expected effect (frame steps, jumps, speed, A-B, sync, subtitle size and position, tracks, aspect cycle, zoom, stay on top, window size, saved frame names, dialogs); `crates/engine/tests/headless.rs`: tracks, chapters and the plain property API |
| Settings | `preferences.rs`: repairs, changes by path, import and export (unknown fields, bad files); `tests/settings.rs`: portable mode and the overrides, changes applied to the running mpv and a restart reading them back, a settings file moved between two installations, the settings window's data |
| Information text | `view.rs`: the Tab overlay's lines |

`cargo xtask ci` is green. In one of three full local runs
`resume.rs::playlists_survive_killed_processes` failed under the load of
the parallel test binaries (the albums on disk were older than the last
second of edits); alone it passed 3 of 3 and the next full run passed. CI
will show whether this needs a wider window.

## Checks on the test machines

All PASS; details in `docs/test-runs/2026-10-12-m4-vms.md`.

- **Ubuntu VM**: `m2build`, `m2`, `m3` (functional and 3 of 3 kill tests),
  `m4` (43 checks, the portable copy), `m2input` (a real right click opened
  the new context menu as a popup window beyond the RVP window).
- **Windows VM**: `m4-all.ps1` (functional with M4's and M5's checks,
  portable mode) and `m3-all.ps1` (second instance, 3 of 3 kill tests).
- **WSL**: only silent tests (no window, no sound); no GUI run.
- **Windows host**: four approved self-test runs (two of this goal's
  builds, two of goal F's), all checks PASS (below).

## Windows host: the self-test's dropped frames (four approved runs)

Goal F's host runs dropped 17-18 frames per self-test with the M3 build
and none with goal E's. The self-test prints the drops with every check.
All three runs used the owner's desktop: 2560x1440 at 60 Hz, RTX 30-series card,
`d3d11va-copy` and WASAPI. Each played the 4K60 HEVC clip and then the
1080p clip with a fresh state and `ipc-server` empty, so SVP did not
attach. PotPlayer and SVP were not touched. Runs 1 and 2 used `run-g.cmd
--selftest --instance=host-g` with state in `C:\Users\<user>\rvp-test\g-*`.
Run 3 used goal F's `run-m3.cmd --selftest --instance=host-f` with
`m3-*`. Run 4 used `run-f4.cmd --selftest --instance=host-f4` with
`f4-*`.

| Run | Build | Result | Dropped frames by step |
|-----|-------|--------|------------------------|
| 1 | f4b1f22 | 44 checks PASS | M3's steps 0-1 (the counter restarts with each file); building the menus and the speed change (one step) 19; opening the settings window and its pages 3; changing a setting and the window grab 11; total 33 |
| 2 | with the fixes below (14c21ac) | 52 checks PASS | M3's steps 0-1; menus 0 (top level built in 38 ms); speed up 0; speed back 0; opening the window 2; showing all eight pages at once 4; the self-test's grab of the window 5; changing a setting 0; closing 0; the second instance 0; total 11 |
| 3 | goal F's build as it is on the host (`rvp-bundle\rvp.exe`, between 947acc2 and b2d7657: it prints the drops per check, its normalizer has no `b=1`), `run-m3.cmd` with a fresh `m3-state` | 34 checks PASS | mpv's counter restarts with each file and seek. 4K60 part: entering and leaving fullscreen 2, the resize to 900x500 8. After the seek to the middle: the appended file 5, the click in the list 6. After the 1080p file started: Shift+N and the volume 0; F5's check 4; the editor's checks 7; the closing's check 11 (the grab and the closing ran in one tick before it); the second instance's check 16; total at the end 16 |
| 4 | goal F's code (0578d90), built for this run with self-test pauses only, not committed: 2 s after F5, after the editor's checks, after the grab, after closing and after the second instance, each followed by a `settled_*` line | 40 checks PASS | F5 opening Keys and Mouse 4, then 1 more while it settled; the editor's checks 0; the self-test's grab of the window 5; closing 0; the second instance 0; total at the end 10. The earlier steps cost 0-1 this time |

Findings:

- **Goal F's 17-18 frames (runs 3 and 4, measured).** They are the count
  at the end of goal F's self-test. mpv's counter restarts with each file
  and seek, so the count covers the steps after the album's first entry
  started. With each step separated (run 4), goal F's build lost 5 frames
  opening the Keys and Mouse window and 5 when the self-test grabbed that
  window; the editor's checks, closing the window and the second instance
  cost 0. In run 3 the editor's checks, the grab and the closing ran in one
  tick, and their cost showed in the next checks (+3, +4, +5). An earlier
  version of this report put the last 5 frames on the window's closing by
  a control comparison; run 4 shows they belong to the grab. So goal F's
  drops came from one RVP step, opening a Qt Quick Controls window during
  playback, and one self-test step, grabbing that window. The drops of
  run 3's earlier steps (the 4K60 resize, the appended file) did not occur
  in runs 1, 2 and 4, so those steps vary between runs.
- **Run 1's 19 frames (measured, then attributed by a control).** In run 1
  they fell in one step that both built the menus and changed the speed.
  Run 2 measured the speed change on its own (0 frames) with unchanged
  speed-change code, so they came from the menu step, whose code changed.
  In run 1 that step built PotPlayer's whole tree (141 commands in about 25
  submenus) on the GUI thread and read each track's properties from mpv
  one by one. The two costs, timed apart on this PC (silent tests: Qt's
  `qml` tool with the offscreen platform, and headless libmpv playing the
  60 s fixture), are:

  | Cost | Before | After |
  |------|--------|-------|
  | Menu items | the whole tree: 122-187 ms | the top level: 15 ms |
  | Track and chapter reads | 0.06-0.07 ms per menu | 0.04-0.05 ms |

  So the tree build was nearly all of the step's GUI-thread time; the read
  change is a small cleanup. On the host, run 2 built the top level in
  38 ms with 0 dropped frames. Dropped frames for the two costs apart were
  not counted on the host.
- **Run 1's 11 frames for changing a setting** included the self-test's
  grab of the window and a re-read of mpv's audio device list on every
  change. Run 2 measured them apart: grab 5, change 0. The device list is
  now read only when the window opens.
- **Left as is:** opening a window during playback costs a few frames on
  the host: Keys and Mouse in goal F about 5, the settings window now 2.
  Closing either costs 0. Showing all eight pages
  once cost 4, about half a frame per page the first time it shows. The
  windows open on request only. The grabs belong to the self-test (a
  picture for each run's record), not to RVP.

## Ubuntu boot run

On 2026-10-12, with reruns on 2026-10-13, `kit.sh m4all` ran on the
owner's Ubuntu 26.10 boot: GNOME Wayland, RTX 30-series card, BlackSpace_en.dsf
from the read-only Windows partition, and SVP 4 Manager running.
`goal-f-status.md`, section "Ubuntu boot run", has the machine, every
step, the GPU checks, SVP and the usability targets. The report is
`docs/test-runs/2026-10-13-ubuntu-boot.md`.

| Step | Result | Detail |
|------|--------|--------|
| `m4` | PASS | 51 checks: the menus with the bound keys and check marks (24 top items built in 18-20 ms, 141 commands), a menu command, F5 with every page, the Keys and Mouse page with 151 actions, a setting applied at once, and the window closing. A portable copy then kept `settings.json` and `state` next to itself. Rerun after the fixes: PASS |
| `m2input` | PASS | A real right click opened the context menu in its own window: 200x617 at first, 241x617 after the menu fix |
| `m2gpu` | FAIL (goal D's open item) | 8 dropped frames per 25 s in fullscreen and 6 in a 1920x1080 window at 4K60; goal D measured 6 per 20 s |
| `m2svp` | PASS | SVP attached after 1.36 s; exit status 0 |

**Dropped frames by step.** These come from the functional self-tests of
`m3` and `m4`, in both the first run and the rerun. SVP was not attached,
and mpv's counter restarts with each file.

- With the 4K60 clip, entering and leaving fullscreen cost 1-2 frames.
- With the 1080p clip:
  - **14 frames in the 1.5 s after the speed returned from 1.1x to 1x.**
    On the Windows host (run 2 above) this step cost 0.
  - 0-1 frames while every settings page was shown.
  - 2-3 frames for the self-test's grab of the settings window.
  - 0 for each of the other steps: building the menus, the speed rise to
    1.1x, opening the settings window, changing a setting, and closing
    it.

The rerun of `m2` counted 0 at every step. SVP attached to that self-test,
though, so it played interpolated and does not compare. The cause of the
14 frames was not investigated (open item below).

**The owner's manual checks (checklist section 7).** Owner's notes, given
in the order of the checklist's seven items:

| Check | Owner's finding | Result |
|-------|-----------------|--------|
| Right-click menu | Labels ran into their shortcuts ("Fullscreen (Keep AR)" into Alt+Enter, "Playback/System Info" into Tab) | Fixed, see below |
| New keys | Work | |
| The skin's own menus | Seem to work | |
| The playlist panel's menu | Works ("more or less"; no fault named) | |
| F5, changes applied at once | Work | |
| Export and import | Taken as working | |
| Portable mode by hand | Deferred by the owner as not important for now | `m4`'s portable copy passed |

The owner also found that real films played upside down. The fix is in
`goal-f-status.md`.

**The menu fix.** Qt Quick Controls' `Menu` keeps its background's
implicit width of 200 pixels, whatever its items need. With Ubuntu's
fonts, the longer labels did not fit next to their shortcuts. Every menu
and submenu now takes the width of its widest item, and a label that
still does not fit ends in an ellipsis (`shell/qml/Menus.qml`). In the
rerun of `m2input`, the context menu opened 241 pixels wide.

**Open items from this run:**

1. The 14 dropped frames after the speed returns to 1x on this boot.
2. The kit's self-test steps (`m2`, `m3`, `m4`) leave `ipc-server` at its
   default, so a running SVP Manager can attach to them, as it did in the
   rerun of `m2`. Setting it empty, as the Windows host runs do, would
   keep their counts comparable.
3. The owner's own check of portable mode.

## Differences from PotPlayer

- The menus are Qt Quick menus in their own popup windows (D6: native
  look, not skinned), not Win32 menus; they look like the interim M2/M3
  menus. PotPlayer's own run-time lists (skins, languages, audio
  renderers, devices, recent files, presets) are not shown; RVP shows
  tracks, chapters and albums in their places.
- Labels say "RVP" where PotPlayer's say "PotPlayer"; the shortcut column
  shows the first key bound now, so a rebinding shows at once.
- 534 of 672 context-menu items and 37 of 54 playlist-menu items are not in
  RVP; `docs/coverage/menus.md` gives each a reason. Large groups: Daum's
  services, devices, TV and DVD, DirectShow filters and renderers, 3D,
  video effects and deinterlacing (M6), subtitle styles and editing, the
  equalizer and audio effects other than the normalizer, PotPlayer's own
  windows (control panel F7 in M6, jump dialog, file information, presets).
- BlackSpace: 124 of 257 commands are not carried out
  (`docs/coverage/blackspace.md`): 54 belong to Daum's discontinued
  broadcast and chat areas, 21 to the capture areas, 30 to the control
  panel, equalizer, colour and video filters (M6); the rest are listed
  with their reasons. Their skin menus show "Not in RVP yet".
- The settings window is RVP's own (PLAN M5: "a PotPlayer-like look is
  welcome but not required"), with RVP's options rather than PotPlayer's
  Preferences tree; the "... Settings..." menu items open its pages.
- Saved frames are named `<file> HH.MM.SS.mmm.png` (or `.jpg`) in
  `Pictures/RVP`; PotPlayer's naming options are not there.
- Zoom, stretch and move steps (7 % zoom in log steps, 5 % stretch, 1 %
  moves), speed steps (0.1x) and the subtitle position steps follow mpv's
  properties; PotPlayer's exact step sizes were not measured.
- The Tab overlay is RVP's summary from the observed properties, not
  PotPlayer's OSD layout.
- Stay on top uses the window system's hint; GNOME Wayland may ignore it.
- A frame step pressed while mpv still restarts after a seek (within a
  fraction of a second, longer with the normalizer's ten-second lookahead)
  is lost; the next press works.
- Subtitles cannot move sideways (mpv has no such property), so those items
  and commands are listed as not carried out.

## For the owner

1. **The Ubuntu boot run.** It was done on 2026-10-12 and 13 with `kit.sh
   m4all`, which runs M3's steps and M4/M5's together (checklist sections
   6 and 7). See "Ubuntu boot run".
2. **What to check** (on Windows with the new `rvp.exe`, on Ubuntu after
   `kit.sh m4all`; checklist section 7 has the same list):
   - Right-click menu: only what RVP does, PotPlayer's order and texts,
     the keys bound now, check marks (Repeat, Stay on Top, aspect ratio,
     Normalizer); tracks under Audio and Subtitles, chapters under
     Playback > Chapter/Bookmark on a file with chapters.
   - New keys: D, F (frames), X, C, Z (speed), [ ] \ (A-B), Shift+, Shift+.
     (audio sync), < > (subtitle sync), Alt+PgUp/PgDn and Alt+Up/Down
     (subtitles), Ctrl+F6 (aspect), keypad (zoom, move, 5 resets), 1-4
     and 8 (window size), Ctrl+T (on top), Tab (information), Ctrl+E and
     Ctrl+Alt+E (save frames), Backspace (start), Home/End (subtitle
     positions), Alt+V (video track).
   - BlackSpace's own menu buttons and right clicks (open, aspect ratio,
     capture, playback, subtitles, window size, favorites).
   - F5: each page; change the volume step, the normalizer's Boost, the
     video sync and the subtitle size during playback and see them apply;
     Export, change something, Import the exported file.
   - Portable mode: a copy of the program folder with an empty
     `rvp.portable` keeps its settings in that folder.

## Owner decisions

- 2026-10-12: two more host runs approved after the first one: one to
  check the fixes, and one of goal F's build for the direct attribution.
- 2026-10-12: at first, no fourth host run to separate the closing of the
  Keys and Mouse window from the second instance. The owner then asked for
  it, and run 4 measured each step apart.
- 2026-10-12: goal G is complete without the Ubuntu boot run. The run
  (`kit.sh m4all`, which covers `m3all`) stays open as an acceptance item
  of M3 and of M4/M5, for the owner to run when convenient. The reminder
  that M3's acceptance needs it was given.
