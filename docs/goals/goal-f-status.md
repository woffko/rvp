# Goal F status: milestone M3, daily-use essentials

Date: 2026-10-11. Result: **done** (all tests and checks pass on WSL, the
Ubuntu VM and the Windows VM); the Windows host and the Ubuntu boot runs
and the one-week trial are the owner's. Precondition met:
`goal-e-status.md` records goal E as done.

Owner answers applied: none were recorded. PLAN.md records no answer to
goal E's open questions (seek mode, window start time, drops after a loop,
the Ubuntu boot run) and no yes to open question 2 (shared state across the
dual boot), so seeks, start-up and looping are as in M2 and no shared state
was built.

## Summary

| Deliverable | Result | Where |
|-------------|--------|-------|
| Playlist panel | BlackSpace's `PlaylistSkinArea` as an editable list: albums as tabs (click to show, the mark on the shown tab deletes, the arrow at the end shows the next album, double click on the tab bar adds one), rows with current, played and unplayable (`<Can't open>`) marks and durations, selection (click, Ctrl, Shift), drag to reorder with a drop line, double click or Enter to play, a scroll bar (arrows, track, thumb) and the wheel, the footer buttons (top, up, down, bottom, add, remove) and the fps footer of M2 | `crates/skin/src/playlist.rs` (geometry and hits), `crates/app/src/pointer.rs`, `crates/app/src/playlist.rs` |
| Albums | RVP's albums are the playlist (mpv plays one entry at a time); add, rename, delete; add files and folders; remove selected, unselected, all, missing; sort ascending, descending, shuffle; move up, down, top, bottom; select all, invert; undo and redo (50 steps) | `crates/app/src/playlist.rs` |
| Playlist keys | R1 table 118 while the panel has the keyboard (a click in the panel gives it; a click on the video takes it back), plus Ctrl+Z, Ctrl+Y and the cursor keys (Shift selects); the panel's context menu | `crates/app/src/action.rs` (`ListAction`), `shell/qml/ListMenu.qml` |
| Persistence | `playlists.json` written on every change by the writer thread; the start reopens the album, the entry and the position, paused | `crates/app/src/playlist.rs`, `app.rs` |
| `.dpl` import | PotPlayer's playlists (read-only) as albums with titles, durations, played and unplayable marks, the current entry and resume positions; drive letters map to Linux folders (`drive-map`, default `V:` → `/mnt/media`); on Windows the first start imports `%APPDATA%\PotPlayerMini64\Playlist`; elsewhere "Import PotPlayer Playlists..." asks for the folder (`potplayer-playlists` remembers one) | `crates/app/src/dpl.rs` |
| Single instance | `rvp FILE` (`/current`) plays in the running RVP, `/add` appends, `/insert` puts files below the playing entry, `/new` starts a separate one; a `QLocalServer` channel `rvp-<user>` (Unix socket, Windows named pipe) | `crates/app/src/instance.rs`, `shell/cpp/shell.cpp`, `shell/src/main.rs` |
| Normalizer | S5's chain `dynaudnorm=f=5000:g=3:p=1.00:m=5` as mpv `af` entry `@rvpnorm` (other filters untouched); Boost off is `m=1`; Level and Release time scale the target peak and frame length; Shift+N and the Audio menu toggle it, with a short notice; settings `normalizer` (`enabled`, `boost`, `level`, `release`) | `crates/app/src/normalizer.rs` |
| Volume above 100 % | `volume-max` 200 % (setting `volume-max`); keys and wheel go past 100 % with a notice | `crates/app/src/app.rs` |
| Blind A/B test | `tools/s5/ab_test.py`: ABX trials from the S5 clips and PotPlayer's recordings, aligned and loudness-matched, scored with the chance of guessing | `tools/s5/ab_test.py` |
| Keys and Mouse editor | F5 (PotPlayer's Preferences key): every window and playlist action with its keys and mouse gestures (clicks, double clicks, middle and side buttons, wheel); capture a key, pick a gesture; a key or gesture in use asks before it moves; Reset, Reset All, Save (changes over PotPlayer's defaults in `settings.json`), Cancel | `crates/app/src/binding_editor.rs`, `shell/qml/KeysAndMouse.qml` |
| Aspect lock | Once a resize settles (300 ms), the window takes the video's shape within the size it was given (R1 finding 1: 1280x720 → 1280x654); setting `keep-aspect-window` | `crates/app/src/chrome.rs` (`fit_aspect`), `shell/src/player.rs` |
| Tests | See below; `cargo xtask ci` green | `crates/*/tests`, unit tests |
| Self-test | `rvp --selftest` gained 18 M3 checks, `--instance=NAME` (a second RVP adds a file), and kill-test support in `=playback` (`--next`, `time=` lines) and `=resume` (`--expect-tolerance`, `--expect-entry`, `--expect-entries`) | `shell/qml/Main.qml` |
| Linux kit | Steps `m3` (self-test and three `kill -9` tests) and `m3desktop` (optional "Open With" entry), `m3all`, checklist section 6 | `tools/linux-kit/` |
| Windows VM script | `m3-all.ps1` and `run.ps1` | `tools/windows-vm/` |

## Tests

| Area | Tests |
|------|-------|
| Playlist model and undo | `crates/app/src/playlist.rs`: edits, undo and redo, moves including drops that keep the playing entry, albums, sorting, neighbours that skip unplayable entries, selection and scrolling, the file after a reopen |
| Panel geometry | `crates/skin/src/playlist.rs`: hits on tabs, rows and the scroll bar; drops between rows |
| `.dpl` import | `crates/app/src/dpl.rs` on synthetic files (BOM, CRLF, gaps, marks, positions, folder albums, order, no mapping on Windows); `tests/import.rs` (albums, positions in the resume store, unique names, an empty "Default" filled); `tests/dpl_local.rs` on the owner's files (local only, below) |
| Single instance | `crates/app/src/instance.rs` (switches, absolute paths, the JSON line, channels); `tests/instance.rs` (`/add`, `/insert`, `/current` on the engine); the self-test's second process over the real socket or pipe |
| Normalizer | `tests/normalizer.rs`: a -30 dBFS and a -6 dBFS tone through mpv's `af` and `ao=pcm`: with Boost +14.0 dB and +5.6 dB, without Boost unchanged (±0.5 dB), no clipping |
| Bindings | `binding_editor.rs`: conflicts, reset, reset all, the settings round trip; `tests/bindings.rs`: every default binding on the engine, including the normalizer toggles and Keys and Mouse |
| Kill tests | `tests/resume.rs`: positions (6 kills), playlists (5 kills during scripted edits: the file on disk is whole and holds the albums of an edit from the last second), the album, entry and position after a kill; the kit and the Windows VM script kill the real program 3 times each |
| Pointer | `tests/chrome.rs` and `pointer.rs` updated for focus, modifiers and the wheel over the list |

## Checks on the test machines

All PASS; details in `docs/test-runs/2026-10-11-m3-vms.md`.

- **WSL** (WSLg): `rvp --selftest --instance=...` with BlackSpace: 34 checks
  PASS; a local `kill -9` and resume cycle PASS.
- **Ubuntu VM**: `m2build`, `m2`, `m3` (functional and 3 of 3 kill tests),
  `m2input` (real pointer: move, resize with the aspect lock, fullscreen,
  menu).
- **Windows VM**: functional with the second instance over the named pipe,
  3 of 3 kill tests (`Stop-Process -Force`).

## The owner's PotPlayer playlists (read-only)

`RVP_POTPLAYER_PLAYLISTS=.../PotPlayerMini64/Playlist cargo test -p rvp-app
--test dpl_local -- --nocapture` read the five `.dpl` files from the Windows
profile through `/mnt/c` without writing anything: 5 albums ("Default"
first), 199 entries, every `N*file*` line imported. 188 entries on `V:\`
map to `/mnt/media/...`; 7 entries on other drives (`A:\`, `C:\`) and 4 URLs
stay as they are until `drive-map` covers those drives; 4 albums have a
current entry; 4 resume positions; 43 played and 2 unplayable marks; one
album is empty. The test prints counts only; nothing from the
playlists is in the repository.

## Usability targets (section 4), measured again

The reference numbers on real hardware need a Windows host run (the
owner's permission) or the Ubuntu boot (`kit.sh m3all`); the VMs render
and decode in software.

| Target | Ubuntu VM | Windows VM (session 0) | Verdict |
|--------|-----------|------------------------|---------|
| Input to visible reaction < 50 ms | 17-297 ms | not measurable (no frames shown) | As in M2: met on the host in goal E, slow in software rendering |
| Seek to new frame < 300 ms | 585-661 ms | 0.60-1.45 s | Not met (exact seeks, goal E question 1) |
| Window shown < 500 ms | 751-816 ms | not measurable | Not met (goal E question 2); M3 added nothing to the start (the new dialogs and menus load on first use) |
| First frame < 1 s | 1.77 s | 3.79 s | Not met on the VMs (1.44 s on the host in goal E) |
| State written on change and at least every 5 s | albums on every change, positions every 2 s | same | Met; kill tests above |
| Nothing lost after a restart or a crash | album, entry, position, volume, normalizer, window | same | Met for M3's state |
| No modal interruptions | messages and the volume and normalizer notices show over the video; dialogs open only when asked for | same | Met |
| Tooltips on every skin control | unchanged | same | Met |
| The menu shows hotkeys | context and playlist menus, refreshed after rebinding | same | Met |
| Every action from mouse and keyboard, configurable, PotPlayer defaults | the Keys and Mouse editor | same | Met |

## For the owner: the one-week trial

Use RVP instead of PotPlayer for a week and note what differs or annoys.
On the Ubuntu boot: `tools/linux-kit/kit.sh m3all` first (checklist
section 6), optionally `kit.sh m3desktop` for "Open With"; on Windows the
bundle from goal E's folder with the new `rvp.exe`. Things to try:

1. **Playlist basics.** F6 or the skin's button shows the panel. Add files
   (Ctrl+I, drag and drop, the footer's +), a folder (Ctrl+O while the
   panel has the keyboard). Remove (Del, Ctrl+Del, Alt+Del, Ctrl+T for
   missing files). Reorder by dragging rows and with Ctrl+Up/Down/Home/End.
   Sort with Ctrl+1, Ctrl+2, shuffle with Ctrl+9. Undo and redo with Ctrl+Z
   and Ctrl+Y. Double click or Enter plays. With the video focused (click
   it), PgUp/PgDn move through the album that plays; in the panel they page
   the list. A file that cannot open is marked and skipped.
2. **Albums.** Ctrl+N adds, Ctrl+R renames, Ctrl+D or the mark on the tab
   deletes, the arrow at the end of the tabs shows the next album.
   Right-click in the panel for the playlist menu.
3. **Restore.** Close RVP in the middle of a file, or end it hard (Task
   Manager, `kill -9`), and start it again without a file: the same album,
   entry and position, paused.
4. **PotPlayer's playlists.** On Windows they are imported at the first
   start of a fresh RVP state (PotPlayer's "Default" fills RVP's empty
   one; the paths stay as they are). On Linux: "Import PotPlayer
   Playlists..." in the playlist or context menu and the Windows profile's
   `AppData/Roaming/PotPlayerMini64/Playlist` folder. For the NAS set
   `"drive-map": { "V:": "/mnt/nas/Media" }` in
   `~/.config/rvp/settings.json` (or link `/mnt/media` to the mount), plus
   any other drives you use (`A:`, `C:`).
5. **Normalizer.** Shift+N toggles it, the context menu's Audio submenu has
   "Boost audio"; the volume goes past 100 % with Up or the wheel. Compare
   with PotPlayer on your own films.
6. **Blind test.** `tools/s5/ab_test.py prepare --ffmpeg ffmpeg --out
   ~/rvp-ab --pair CLIP:RECORDING ...` with the S5 clips and recordings
   (`~/rvp-reference/{clips,s5}` on the WSL side), listen to each
   trial's A, B and X (in RVP or any player), fill in `answers.txt`, then
   `ab_test.py score --out ~/rvp-ab`. The script's header explains it.
7. **Keys and Mouse.** F5: rebind a key and a mouse button (middle click,
   a side button, a double click, the wheel), see the question when a key
   is already in use, try Reset and Reset All, Save, and check that the
   menus show the new keys.
8. **Single instance.** With RVP running, open files from the file manager
   or a terminal: `rvp FILE` plays it in the running window, `rvp /add
   FILE` and `rvp /insert FILE` only add, `rvp /new FILE` opens a second
   window.
9. **Window shape.** Resize the window while a video plays; when you let
   go it takes the video's shape (PotPlayer does this during the drag).

## Found and fixed

- On Windows mpv reports a file's path normalized (`C:/a/../b.mkv` plays as
  `C:\b.mkv`), while file dialogs and drops give forward slashes. The
  album entry then did not match mpv's path, so the title lost its
  `[i/n]` and entry label. Album entries now store local paths in mpv's
  form (`playlist::local_path`); CI's Windows run found it.
- A dragged corner on GNOME Wayland ends in an interactive resize; RVP
  fits the window to the video afterwards, which the kit's input check now
  accepts.

## Differences from PotPlayer (known, deferred or unverified)

- The window takes the video's shape 300 ms after a resize settles, not
  during the drag (Qt cannot constrain an interactive resize).
- PotPlayer's playlist commands not in M3: add URL, save or open a
  playlist file, the playback queue, search, sort by title, name,
  extension, size, duration or date, edit titles, rename or delete files,
  open the file location, copy and paste, file information, detect
  durations, playlist settings (F5 in the panel opens Keys and Mouse
  instead). Rows cannot be dragged to another album.
- The import orders albums "Default" first, then alphabetically (the
  registry has no album order); folder albums import empty; nothing is
  written back to PotPlayer.
- The normalizer is S5's `dynaudnorm` approximation. Level and Release
  time other than PotPlayer's defaults use a guessed mapping.
- The skin's volume slider shows 0-100 %; above 100 % it stays full and the
  notice shows the value.
- No Windows file association or "Open with" entry is registered (that
  would change system settings); RVP can be picked by hand in "Open with".
- Mouse gestures apply to the window scope; the playlist panel's own mouse
  handling is fixed (as in PotPlayer).

## Open questions for the owner

1. **Drive map.** Make `/mnt/nas/Media` the default for `V:`
   instead of `/mnt/media`? Which Linux folders do `A:` and the `C:` entries
   correspond to?
2. **Goal E's questions 1-4** are still open (exact or keyframe seeks,
   start-up time, drops after a loop, the Ubuntu boot run of `m2all`, now
   `m3all`).
3. **Windows host numbers.** See "Windows host" below.

## Owner decisions in this goal

- 2026-10-11: the normalizer stays `dynaudnorm` for now (S5's chain); the
  RVP-side AGC is not built.
- 2026-10-11: a Windows host run of the M3 self-test, approved.

## Host-side effects and cleanup

- Nothing ran on the Windows host. The owner's PotPlayer playlists were
  only read through `/mnt/c` (no PotPlayer or SVP setting touched).
- Windows test VM: `C:\rvp-test` gained `m3-all.ps1`, `run.ps1` and
  `kill-*.log`; `rvp-bundle\rvp.exe` is the M3 build.
- Ubuntu VM: `/opt/rvp-kit/logs-m3` and `logs-m3b`; the kit restored every
  ACL and the pointer setting (`codex_acl_entries=0`).

## Versions

As in goal E (Rust 1.98.1, Qt 6.10.3, cxx-qt 0.10.0, libmpv `b2c255c13e`
builds, Ubuntu 24.04's libmpv 2 for WSL and CI tests). New Qt module:
Qt Network (`QLocalServer`); the Windows bundle already contained
`Qt6Network.dll`.
