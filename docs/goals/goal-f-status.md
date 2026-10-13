# Goal F status: milestone M3, daily-use essentials

Date: 2026-10-11. Result: **done**: all tests and checks pass on WSL, the
Ubuntu VM, the Windows VM and, in three approved runs, the Windows host.
The Ubuntu boot run was done on 2026-10-12 and 13 (see "Ubuntu boot
run"). The one-week trial is the owner's (an open item).
Precondition met: `goal-e-status.md` records goal E as done.

At the start no answers to goal E's open questions were recorded and
PLAN.md had no yes to open question 2 (shared state across the dual boot),
so no shared state was built. The owner answered goal E's questions at the
end of the goal (see "Owner decisions"); exact seeks and the start-up stay
as they are.

## Summary

| Deliverable | Result | Where |
|-------------|--------|-------|
| Playlist panel | BlackSpace's `PlaylistSkinArea` as an editable list: albums as tabs (click to show, the mark on the shown tab deletes, the arrow at the end shows the next album, double click on the tab bar adds one), rows with current, played and unplayable (`<Can't open>`) marks and durations, selection (click, Ctrl, Shift), drag to reorder with a drop line, double click or Enter to play, a scroll bar (arrows, track, thumb) and the wheel, the footer buttons (top, up, down, bottom, add, remove) and the fps footer of M2 | `crates/skin/src/playlist.rs` (geometry and hits), `crates/app/src/pointer.rs`, `crates/app/src/playlist.rs` |
| Albums | RVP's albums are the playlist (mpv plays one entry at a time); add, rename, delete; add files and folders; remove selected, unselected, all, missing; sort ascending, descending, shuffle; move up, down, top, bottom; select all, invert; undo and redo (50 steps) | `crates/app/src/playlist.rs` |
| Playlist keys | R1 table 118 while the panel has the keyboard (a click in the panel gives it; a click on the video takes it back), plus Ctrl+Z, Ctrl+Y and the cursor keys (Shift selects); the panel's context menu | `crates/app/src/action.rs` (`ListAction`), `shell/qml/ListMenu.qml` |
| Persistence | `playlists.json` written on every change by the writer thread; the start reopens the album, the entry and the position, paused | `crates/app/src/playlist.rs`, `app.rs` |
| `.dpl` import | PotPlayer's playlists (read-only) as albums with titles, durations, played and unplayable marks, the current entry and resume positions; paths follow the system (owner decision): on Windows as PotPlayer wrote them, on Linux always Linux paths (`drive-map`, default `V:` → `/mnt/media`; other drives `/mnt/<letter>`; network paths `/mnt/<server>/<share>`); on Windows the first start imports `%APPDATA%\PotPlayerMini64\Playlist`; elsewhere "Import PotPlayer Playlists..." asks for the folder (`potplayer-playlists` remembers one) | `crates/app/src/dpl.rs` |
| Single instance | `rvp FILE` (`/current`) plays in the running RVP, `/add` appends, `/insert` puts files below the playing entry, `/new` starts a separate one; a `QLocalServer` channel `rvp-<user>` (Unix socket, Windows named pipe) | `crates/app/src/instance.rs`, `shell/cpp/shell.cpp`, `shell/src/main.rs` |
| Normalizer | S5's chain with dynaudnorm's alternative boundary mode, `dynaudnorm=f=5000:g=3:p=1.00:m=5:b=1`, as mpv `af` entry `@rvpnorm` (other filters untouched); a toggle during playback seeks to the current position; Boost off is `m=1`; Level and Release time scale the target peak and frame length; Shift+N and the Audio menu toggle it, with a short notice; settings `normalizer` (`enabled`, `boost`, `level`, `release`) | `crates/app/src/normalizer.rs` |
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
map to `/mnt/media/...`; 7 entries on other drives (`A:\`, `C:\`) become
`/mnt/a/...` and `/mnt/c/...`, and 4 URLs stay URLs, so no Windows path is
left on Linux; 4 albums have a
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

## Windows host (three runs, each approved)

The owner's desktop: 2560x1440, RTX 30-series card, `d3d11va-copy`, WASAPI;
PotPlayer and SVP Manager were running and were not touched (`ipc-server`
empty, so SVP did not attach). The synthetic 4K60 HEVC and 1080p clips from
goal C; settings and state in `C:\Users\<user>\rvp-test\m3-*` and `m2e-*`.

| Run | Build | Result | Window shown | First frame | Input | Seeks | Dropped frames |
|-----|-------|--------|--------------|-------------|-------|-------|----------------|
| 1 | M3 (first start of the new `rvp.exe`) | 34 checks PASS | 1169 ms | 1732 ms | 7-19 ms, outliers 56-456 ms | 1.16, 2.33 s | 18 |
| 2 | M3 | 34 checks PASS | 973 ms | 1519 ms | 8-45 ms, outliers 114-439 ms | 1.15, 2.46 s | 17 |
| 3 | goal E's build, for comparison | 16 checks PASS | 1183 ms | 1974 ms | 9-68 ms, one 232 ms | 0.93, 1.91 s | 0 |

Goal E's build starts as slowly today as the M3 build (goal E measured
0.76 s on another day), so start-up did not regress; the input outliers
appear in both builds. The dropped frames are new: the M3 self-test drops
17-18 frames per run where goal E's shorter test drops none. Where they
happen is not known yet: the self-test now prints the drop count with every
check, and the next host run will show the step (an open item).

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
   `~/.config/rvp/settings.json` (or link `/mnt/media` to the mount).
   Other drives appear as `/mnt/<letter>/...`; map them in `drive-map` if
   they are mounted elsewhere.
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

- The normalizer restarted at every seek and at the start of every file
  and assumed a gain of 1 for the frames it had not seen, so the level rose
  from the raw level for 8-12 s each time; Shift+N during playback rebuilt
  the chain, which dropped the ten seconds of audio mpv had decoded ahead,
  and the sound was gone to the end of the file. Measured silently in real
  time (`ebur128` after the filter, `ao=null`): with `b=1` the level is at
  its target from the first second and after a seek, and after a toggle
  RVP seeks to the current position, so the sound continues.
- Appending files (drop with append, Add Files, `/add`) and Next or
  Previous without a neighbour fitted the window to the playing video; the
  fit now waits for the new file to load.
- A fresh RVP state on Windows imports PotPlayer's playlists; the
  self-test now skips that, so it never works on the owner's albums.

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
  time other than PotPlayer's defaults use a guessed mapping. The filter
  looks about ten seconds ahead, so mpv decodes that much audio before
  playing on after a seek: about 50 ms per seek on a local file (200 → 250
  ms measured), more from the NAS.
- The skin's volume slider shows 0-100 %; above 100 % it stays full and the
  notice shows the value.
- No Windows file association or "Open with" entry is registered (that
  would change system settings); RVP can be picked by hand in "Open with".
- Mouse gestures apply to the window scope; the playlist panel's own mouse
  handling is fixed (as in PotPlayer).

## Ubuntu boot run

2026-10-12, with reruns on 2026-10-13, on the owner's Ubuntu 26.10 boot
(development branch, kernel 7.2.0-5). The machine: GNOME Wayland, two
2560x1440 screens at 60 Hz, an RTX 30-series card with driver 595.91.07, and a
32-core processor. `tools/linux-kit/kit.sh m4all` ran M3's steps together
with M4 and M5's, so it counts as the `m3all` run. It ran from a terminal
in the GNOME session with:

- the kit's libmpv (`b2c255c13e`, `nvdec-copy`, PipeWire);
- BlackSpace_en.dsf from the Windows partition, mounted read-only;
- SVP 4 Manager running.

The owner installed the packages, ran `ydotoold` as root and started SVP.
The report is `docs/test-runs/2026-10-13-ubuntu-boot.md`. M4
and M5's part, with the owner's manual checks, is in `goal-g-status.md`.

| Step | Result | Detail |
|------|--------|--------|
| `check`, `deps`, `mpv`, `m2build` | PASS | Qt 6.10.3, meson 1.12.1, the kit's libmpv |
| `m2` | PASS | The functional self-test passed, and the restart reopened the last file paused. Rerun after the fixes below: PASS |
| `m3` | PASS | The functional self-test passed with M3's checks: playlist panel, albums, normalizer, volume above 100 %, the Keys and Mouse page, second instance and aspect lock. All 3 kill tests reopened entry 2 of 2 paused, 0.67-0.71 s before the last position the test saw (5 s allowed) |
| `m2input` | PASS | With the real pointer (ydotool), the window moved by -80,60 and resized by -142,-80 with the aspect lock. A double click entered fullscreen at 2560x1440, and a second one left it. A right click opened the menu window. Rerun after the fixes: PASS |
| `m2gpu` | FAIL (goal D's open item) | 4K60 HEVC for 25 s: 8 dropped frames in fullscreen, 6 in a 1920x1080 window; the decoder dropped none |
| `m2svp` | PASS | SVP's `@svp` filter attached after 1.36 s. 12 dropped frames in 40 s in a window, none by the decoder; exit status 0 |
| `report` | PASS | |

**GPU checks.** Per 20 s, the counts are 6.4 in fullscreen and 4.8 in the
window. That is the level goal D left open (6 per 20 s at 4K60, GNOME
Wayland with the NVIDIA driver; mpv's own output drops a few too). The
decoder dropped no frames, so decoding keeps up. Nothing was changed for
this.

**SVP.** In `m2svp`, SVP attached through RVP's default IPC socket
(`/tmp/mpvsocket`) and played the 1080p clip to the end. SVP Manager was
also running during the other steps. It attached to the rerun's `m2`
self-test as well (that run's last status shows `@svp`), because the
kit's self-test steps leave `ipc-server` at its default. The Windows host
runs set it empty instead.

**Usability targets (section 4) on real hardware.** Measured across the
self-tests of `m2`, `m3`, `m4`, `m2gpu` and `m2svp`, including the reruns:

| Target | Ubuntu boot | Verdict |
|--------|-------------|---------|
| Input to visible reaction < 50 ms | 51 of 64 inputs took 0.8-43 ms; 13 took 51-314 ms | Met for most inputs. The outliers match the Windows host's (56-456 ms) |
| Seek to new frame < 300 ms | 103-516 ms over six seeks (only one under 300) | Not met: seeks are exact (goal E, question 1) |
| Window shown < 500 ms | 225-324 ms (324 ms on the first start after the build) | Met. The Ubuntu VM took 751-816 ms and the Windows host 973-1183 ms |
| First frame < 1 s | 614-869 ms | Met |
| State written on change and at least every 5 s; nothing lost after a crash | 3 of 3 kill tests | Met |

**Found and fixed in this run.** `cargo xtask ci` passes with all three
fixes, and the reruns of `m2`, `m4` and `m2input` pass:

- **The video was upside down.** On real films the owner saw the picture
  upside down and mirrored, which is what a vertical flip looks like.
  `MpvItem` mirrored itself vertically, but mpv already renders into the
  framebuffer object the way Qt samples it, so the mirror flipped it a
  second time. Compared with ffmpeg's frame of the same clip, earlier
  grabs on Linux and Windows show the same flip; the synthetic test
  pattern had hidden it. The mirror is gone (`shell/cpp/shell.cpp`). The
  rerun's grab (`m2-grabs/playing-0.png` in the kit's log directory) now
  matches ffmpeg's frame, with the timestamp at the top left.
- **Paths with `.` or `..`.** At fd5eda9 `cargo xtask ci` failed in
  `tests/bindings.rs` on this boot. The kit's mpv master drops `.` and
  `..` from the `path` property on Linux too. A path given with them then
  no longer matched its album entry, so RVP did not recognise the entry
  that played. CI did not catch it because it uses Ubuntu 24.04's mpv
  0.37, which reports paths as given. `playlist::local_path` now drops
  `.` and `..` itself on every system, so every mpv version reports the
  same path.
- **Menu labels ran into their shortcuts.** See `goal-g-status.md`.

## Open items

1. **Dropped frames in the host self-test** (17-18 per run, see "Windows
   host"): find the step with the per-check counts; needs a host run with
   the owner's permission.
2. **The one-week trial** (M3's exit in PLAN.md): normal viewing on one
   system without losing a position or a playlist.
3. **The Ubuntu boot run**: done on 2026-10-12 and 13 with `kit.sh m4all`
   (see "Ubuntu boot run"). All steps pass except `m2gpu`, whose dropped
   frames are goal D's open item at the same level.

## Owner decisions in this goal

- 2026-10-11: the normalizer stays `dynaudnorm` for now (S5's chain); the
  RVP-side AGC is not built. Its defects are fixed (see "Found and
  fixed"); the owner left the choice between fixing it and an own
  normalizer to the session, and fixing it was the smaller change.
- 2026-10-11: paths follow the system: Windows paths on Windows, Linux
  paths on Linux, never mixed.
- 2026-10-11, goal E's questions: exact seeks stay; the window's start time
  is accepted; the dropped frames after a loop are deferred (PLAN.md,
  deferred items); the Ubuntu boot run is done when it blocks an
  acceptance, with a reminder.
- 2026-10-11: three Windows host runs (the M3 self-test twice, goal E's
  build once), each approved.
- 2026-10-11: silent tests (no window, no sound) need no permission.

## Host-side effects and cleanup

- Windows host: the three approved runs above. `C:\Users\<user>\rvp-test`
  gained `run-m3.cmd`, `run-m2e.cmd`, `m3-config`, `m3-state`, `m3-out`,
  `m2e-config`, `m2e-state`, `m2e-out`, `rvp-bundle\rvp-m2.exe` (goal E's
  build) and `wsl-run-*.png`; `rvp-bundle\rvp.exe` is the M3 build. All can
  be deleted. The owner's PotPlayer playlists were only read through
  `/mnt/c`; no PotPlayer or SVP setting was touched.
- Not approved, a mistake: about six local self-test runs from WSL opened
  their windows on the owner's desktop through WSLg, playing the 96x72
  test fixtures (crf 51, Opus at 6 kbit/s) with sound. The owner saw them
  as a damaged picture and distorted sound. GUI programs started in WSL
  now count as host runs (AGENTS.md).
- Windows test VM: `C:\rvp-test` gained `m3-all.ps1`, `run.ps1` and
  `kill-*.log`; `rvp-bundle\rvp.exe` is the M3 build.
- Ubuntu VM: `/opt/rvp-kit/logs-m3` and `logs-m3b`; the kit restored every
  ACL and the pointer setting (`codex_acl_entries=0`).

## Versions

As in goal E (Rust 1.98.1, Qt 6.10.3, cxx-qt 0.10.0, libmpv `b2c255c13e`
builds, Ubuntu 24.04's libmpv 2 for WSL and CI tests). New Qt module:
Qt Network (`QLocalServer`); the Windows bundle already contained
`Qt6Network.dll`.
