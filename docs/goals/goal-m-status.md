# Goal M status: milestone M11 (PotPlayer parity: playback, playlist and opening)

Date: 2026-10-17. Result: **done**. Precondition met: `goal-l-status.md`
records goal L as done. The owner reported no findings from goal L's
build, so M11 started with the thumbnail core. Everything was built and
checked on Windows first (the test VM); the host ran nothing: the build
was only copied there.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Thumbnail core | A second libmpv core without audio draws frames with mpv's software renderer (`vo=libmpv`, the software render API added to the vendored libmpv2) on a thread of its own, with a 48 MiB cache (least recently used out). Requests come in groups: the seek bar's preview (the newest replaces the waiting one), the scene browsers, the playlist's thumbnails; a group is cancelled when its view goes; durations are found behind them ("Detect file durations"). About 3 ms a picture on the fixtures | `crates/engine/src/thumbnail.rs`, `vendor/libmpv2/RVP-PATCH.md`, `crates/app/src/thumbs.rs` |
| Seek bar | Time tooltips while hovering and seeking (with the chapter or bookmark there), thumbnail previews, markers for chapters, bookmarks and A-B points, drawn by the skin engine in RVP Default and in PotPlayer's skins | `crates/app/src/playback.rs`, `crates/skin/src/paint.rs` |
| Jumps | The Jump to Time/Frame dialog (times, `+`/`-` seconds, frames), the previous and next keyframe, the current subtitle's position, the time jump settings | `playback.rs`, `shell/qml/PlaybackDialogs.qml` |
| Scene browsers | By time intervals (Shift+Z), by markers (Alt+Z: chapters and bookmarks) and by subtitle intervals (the lines of an external SubRip, WebVTT or ASS file), with pictures from the thumbnail core; a click jumps there | `crates/app/src/scenes.rs`, `shell/qml/SceneBrowser.qml` |
| Repeat and skip | A-B fine moves (0.1 s), the current, previous and next subtitle line, marked sections and the A-B list per file (kept when "Remember A-B Repeat List" is on), the seconds behind the current point, loop counts, "Prevent backward navigation from point A"; skipping the intro, the ending and listed chapters (Skip Setup); shuffle; bookmarks per file (add, rename, delete, jump, auto delete, a title asked first) after the chapters in Chapter/Bookmark | `playback.rs`, `crates/app/src/marks.rs` |
| When playback finishes | Exit, shut down, sleep or hibernate after the file or after the list, behind a notice that counts down and can be cancelled; the shutdown timer (at a time or after a time); the action can be remembered. The app decides, the shell acts; `--power-dry-run=FILE` (always on with `--selftest`) writes the action instead | `crates/app/src/power.rs`, `shell/src/power.rs` |
| Other playback items | Played and related files to the Recycle Bin after RVP's own confirmation (Windows' shell file operation with undo); the pointer hidden over playing video in the window; remembering positions as a toggle | `shell/src/recycle.rs`, `playback.rs` |
| Opening | Load External Audio and Reload Subtitle; DVD (a folder, a drive or an `.IFO`: titles from the disc's title table, chapters, angles, audio and subtitle streams, DVD Control's speeds) and Blu-ray (a folder, a drive or an `.MPLS`); audio CDs; webcams and other devices (DirectShow's list); the screen (`gdigrab`, or `ddagrab` through lavfi); HTTP, HTTPS, FTP and WebDAV addresses; streaming sites through the user's yt-dlp (RVP does what mpv's `ytdl_hook` does: an EDL of the formats, the headers, the title); the clipboard; recent items and favorites (files and folders); the Default Open Action; "When Files Opened" (the file alone, similar files, or the whole folder) | `crates/app/src/sources.rs`, `crates/app/src/ytdl.rs`, `shell/src/devices.rs` |
| Playlist panel | The playback queue (it plays before the next entry), sorting by title, name, extension, size, duration, date, folder and release year, search, editing titles, renaming files, opening the folder, moving and copying files (on a thread), copy and paste (the recent list too), Add URL, file information, playlist files in and out (`.dpl`, `.m3u`/`.m3u8`, `.pls`), remembering the default playlist and recent URLs, durations found in the background, thumbnail views of 2 to 7 lines with details and 16:9 pictures, starting albums from menus, the playlist storage folder, deleting files to the Recycle Bin, "Play within a new instance" | `crates/app/src/albums.rs`, `crates/app/src/playlist_files.rs`, `crates/skin/src/playlist.rs` |
| Command line | 25 of PotPlayer's 29 switches: files, folders and URLs (When Files Opened applies); `/seek`, `/sub`, `/aud`, `/title`, `/user_agent`, `/referer`, `/headers` for the first file; `/add`, `/insert`, `/autoplay`, `/same`, `/sort`, `/randomize`, `/new`, `/current`, `/clipboard`; the dialogs; `/cap`, `/cam`, `/dvd`, `/cd`; standard input | `crates/app/src/instance.rs`, `docs/coverage/command-line.md` |
| Studies | Seamless playback and the file navigator with Open Box: neither went in (below) | `docs/spikes/m11-studies.md` |
| Coverage | Every item of the context menu's Open (20 with Open File(s)), Playback (116), Album/Favorites (3) and DVD Control (13) menus and of the playlist menu (53) is carried out; the Playback, Time Jump, Scene Browser, Playlist, Device and DVD/Blu-ray pages' options are settings or keep a reason; the keys of these menus work or say why not | `crates/app/src/coverage.rs`, `docs/coverage/` |

## The studies

`docs/spikes/m11-studies.md` records both. **Seamless playback** is not
added: libmpv preloads and joins files only inside its own playlist or an
EDL, and either would let mpv move on (or merge entries) where RVP decides
(the queue, shuffle, Skip Ending, "When Playback Finished", the Recycle
Bin, resume positions). Today's switch, measured headless from the end of
one entry to the next entry's first frame, takes 68 to 74 ms in WSL and
85 to 102 ms on the Windows VM. **The file navigator and Open Box** are
not added: they could keep the album rules, but they are a second file
manager for ways in RVP already has. Whether RVP should grow either is
the owner's call (below).

## Tests

- `crates/engine/tests/thumbnails.rs` (8 tests): pictures keep the video's
  shape within the box or fill it, show the right frame (the marker
  fixture's pixels), render on the core's thread while requests return at
  once (about 3 ms a picture), a newer preview replaces the waiting one,
  cancelling drops a group, cached pictures come without the core,
  durations come in the background, a file without video fails and the
  core goes on.
- `crates/app/tests/m11.rs` (11 tests, headless libmpv, on
  `fixture-chapters.mkv`: 20 s, four chapters, a keyframe every 2 s, four
  subtitle lines): chapters mark the seek bar and listed ones are skipped;
  the intro and the ending; keyframe steps, the jump dialog's times and
  frames, and backward play at 1x; subtitle lines and marked sections
  repeat; the scene browsers' places; the queue comes first and shuffle
  plays each entry once; the gap between two entries; the power actions'
  countdown, cancel and timer; the command line's switches; a DVD folder
  made with dvdauthor (titles, chapters, angles, the `.IFO`, the second
  title in the same album entry, backward play refused); the thumbnail
  core answering the app off the GUI thread.
- Unit tests: playlist files round trip in every format (relative paths
  from the file's folder), sorting by each field is stable, yt-dlp's JSON
  becomes an EDL, similar files and disc addresses, the clipboard's lines,
  the DVD title table, capture addresses, bookmarks, sections, favorites
  and recent items, time scenes and subtitle files' lines, the jump
  dialog's times, chapter names to skip, the dry-run hook.
- `tests/menus.rs`, `tests/bindings.rs`, `tests/chrome.rs`,
  `tests/instance.rs` and `tests/coverage.rs` cover the new rows (every
  M11 key runs its action; the menus' bookmark, DVD and playlist items).
- `rvp --selftest=m11` (41 checks, `shell/qml/SelfTestM11.qml`): the
  command line's `/seek` and `/title`, the seek bar's tooltip and preview,
  the jump dialog, keyframes, the current subtitle, A-B subtitle and list
  sections, a chapter skip, a bookmark, both scene browsers, the
  playlist's thumbnail view, durations, the queue, sorting, search, a
  playlist file saved and opened, duplicates removed, the power countdown
  cancelled and then carried out by the dry-run hook, the Recycle Bin's
  question (and on Windows the file in the Recycle Bin), the DVD folder,
  external audio, the clipboard, favorites and recent items, yt-dlp's
  fallback, the capture devices' list, and the thumbnail core off the GUI
  thread. CI's Linux package check runs it on the installed .deb.
- `tools/test-clips/make-m11-fixtures.sh` makes the fixtures bit-exact
  from FFmpeg's test sources and dvdauthor.

## Checks on the Windows VM

`tools/windows-vm/m11-all.ps1` on the Windows test VM (session 0 over
SSH, software OpenGL, no sound device), all pass:

- `--selftest=m11`: 41 checks, failures 0, with the command line's
  `/seek` and `/title`, the seek bar's tooltip and preview (144x108 for
  the 4:3 fixture), the DVD folder (two titles) and the Recycle Bin.
- The power actions only through the dry-run hook: the self-test's
  countdown was cancelled once, then "shut down" was written to
  `power-dry-run.txt`; the VM's boot time was the same before and after.
- The Recycle Bin: the self-test's own file left its folder and was in
  the SSH user's Recycle Bin (its `$I` record names it), and its playlist
  entry went with it; the script then removed it from the Recycle Bin.
- `tests/thumbnails.rs` (8 tests, 3 to 4 ms a picture) and `tests/m11.rs`
  (11 tests: the DVD with shinchiro's libdvdnav, backward play 1 s in
  1.16 to 1.19 s, the gap between entries 85 to 102 ms) on Windows' own
  libmpv.

The first run found one failure, in the self-test itself: its Recycle Bin
check counted the playlist from before the file went (Linux skips that
check, so only Windows showed it).

Both scripts ran again on the package built from f447a7d (`rvp.exe`
SHA-256 4a06fbbb...), all passing: `m11-all.ps1` as above and, as a
regression check of M11's changes to the window, `m10-all.ps1`'s
functional self-test (failures 0, the awake request, the recording).

GitHub CI's run on f447a7d (39061769338) found what the VM's scripts
could not: two unit tests with Unix paths on Windows, and the Linux
package check's M11 self-test without DVD support (findings 11 to 13).
After the fixes, every test executable of the workspace (38, built for
Windows in debug as CI builds them) ran on the VM with the repository
mirrored at the path compiled into them: all pass, except
`tests/resume.rs`, whose three tests start a helper program from the
build machine's path (GitHub's Windows runner runs them).

## Findings

1. **mpv numbers DVD titles from 0.** `dvd://1/` is the second title; RVP
   took libdvdnav's numbering first. The title list now comes from
   `VIDEO_TS.IFO` (mpv 0.37 has no `disc-titles`) and its addresses count
   from 0.
2. **mpv reads past a short DVD title.** FFmpeg probes 5 s of a stream,
   and libdvdnav goes on into the next title it plays, so a title shorter
   than that reported the next one's length and chapters. The fixture's
   titles are 8 s and 6 s; real discs' short titles (logos, warnings)
   behave the same in mpv.
3. **Backward play stalls on discs.** mpv's `play-direction=backward` runs
   smoothly on files (1 s of video in 1.0 s) but the time stands still on
   a DVD's stream: DVD Control's backward speeds are refused on discs with
   a notice and work on files.
4. **A relayout at the same size crashed the window** (an index out of
   bounds in `Chrome::repaint`, old but reachable now): switching the
   playlist to thumbnails lays the skin out again at the same size, and
   the painted regions had been cleared without being counted as new.
5. **The thumbnail core waited 200 ms a picture** at first: vo_libmpv holds
   a seek until its frame is rendered, and any property read meanwhile
   waits for that timeout. The worker renders while it waits for the
   restart and reads no properties then.
6. **A probe that selects no tracks fails** (`vid=no`, `aid=no`: nothing to
   play); durations are probed with `track-auto-selection=no`.
7. **The Recycle Bin needs full paths**: Windows' shell deletes a relative
   path for good instead of recycling it; RVP passes absolute ones.
8. **Fusion's titled Dialog loops on its width** with an assigned
   `contentItem`; the Recycle Bin's dialog takes its layout as a child.
9. **The self-test's pointer moves were undone** under Xvfb: Qt repeats
   the real pointer's hover every frame while items change. Self-tests
   now ignore the window's own hover.
10. **"Reset A-B Section Repeat" left the repeat flagged on** with no
    points; it now ends it.
11. **Open DVD's answer skipped libmpv's protocol list**: with a libmpv
    without libdvdnav (the Linux kit's, found by CI's package check) the
    disc replaced playback with an address mpv cannot open. The disc
    dialog's answer now refuses with a notice, as the Open menu did.
12. **A self-test option was also a PotPlayer switch**: `--dvd=FOLDER`
    and `/dvd` reach the same command-line value, so the self-test also
    asked to open the DVD at start. It is `--dvd-folder` now.
13. **Two unit tests used Unix paths** (`/films/...` has no drive on
    Windows); GitHub's Windows runner found them. CI's test step now runs
    every test binary after a failure (`--no-fail-fast`), so one run shows
    all of them.
14. **The self-test's resize raced the first file's fit** to its 96x72
    video: the window is made larger once that file has loaded, and the
    pointer moves wait for the skin's layout at the new size.

## Differences from PotPlayer

- Seek bar previews, scenes and playlist thumbnails show the keyframe
  nearest the time (fast), not the exact frame, and only for local files.
- The subtitle scene browser lists external text subtitle files only:
  mpv gives no list of an embedded track's lines.
- DVD and Blu-ray: titles, chapters, angles and streams, not the discs'
  menus (goal R). Blu-ray titles are the disc's playlists by number. DVD
  Control's backward speeds are refused on discs (finding 3).
- Open FTP/WebDAV/HTTP opens an address; there is no folder browser for
  FTP or WebDAV servers.
- yt-dlp is the user's own (RVP does not ship it); without it, or for a
  site it does not know, the address plays as FFmpeg opens it.
- Capture devices are opened with FFmpeg's `dshow` and their listed
  names; PotPlayer's crossbar, tuner and per-device format pages are not
  in RVP. TV tuners are a non-goal.
- The playlist is the skin's attached panel: its font, colours and row
  numbers come from the skin (Color/View's options keep that reason).
  Entries without video show a black box in thumbnail views.
- Durations are found in the background when entries are added and with
  "Detect file durations", not for every album at start (network
  entries would load).
- Bookmarks are kept in RVP's state folder; PotPlayer's `.pbf` files are
  not read or written.
- The power actions, the Recycle Bin and the device list are Windows' for
  now (goal S).
- Seamless playback and the file navigator are not in RVP (the studies).
- Four command-line switches are not: `/simple` (the navigator), `/atv`
  and `/dtv` (tuners) and `config=` (presets are M14's).

## Linux items for goal S

- The Recycle Bin: the freedesktop trash (Linux refuses with "on Windows
  for now").
- Power actions through logind (shut down, sleep, hibernate).
- Capture devices: listing `v4l2` devices (the address is built already)
  and the screen through PipeWire's portal or `x11grab`.
- The Linux kit's libmpv is built without libdvdnav, libbluray and
  libcdio: DVD, Blu-ray and audio CD do not open in the Linux packages
  (Ubuntu's libmpv has them; the M11 self-test skips the DVD step there).
- zbus 5's helper crates are flagged unmaintained (RUSTSEC-2026-0370,
  -0371, -0372, ignored in `deny.toml` with the reason); zbus 6 drops the
  blocking API RVP uses and brings tokio, so moving to it is goal S's.

## What to try

In `C:\Users\<user>\rvp-test\rvp-m` (portable: its settings stay beside
`rvp.exe`), with PotPlayer's menu texts and keys:

1. **Seek bar**: hover over it for the time, the chapter or bookmark
   there and a picture of that place; chapters and bookmarks show as
   marks (Settings > Playback turns each part on or off).
2. **Jumps**: G (Jump to Time/Frame), Ctrl+Shift+Left and Right (the
   keyframe before and after), Ctrl+Home (the current subtitle's start).
3. **Scene browsers**: Shift+Z (by time), Alt+Z (chapters and bookmarks),
   and Playback > Jump > Browse Scenes by Subtitle Intervals with an
   external subtitle file; click a picture to jump.
4. **A-B repeat**: `[` and `]`, then Ctrl+[ / Alt+[ and Ctrl+] / Alt+] to
   move the points, Ctrl+\ to repeat the subtitle line, Shift+\ for the
   A-B list (add the section, then Alt+\ repeats it).
5. **Skips and bookmarks**: `'` (Skip Setup: intro, ending, chapters by
   name), P adds a bookmark; Playback > Chapter/Bookmark lists them.
6. **When playback finishes** (Misc > When Playback Finished; Ctrl+Alt+A
   cycles it): try "Exit RVP when playback ends" first. Shut down, sleep
   and hibernate are real: the notice counts down and Cancel stops it.
   Misc > "Add a task to turn off PC or exit RVP..." is the timer.
7. **Opening**: Ctrl+D (a DVD drive or folder), Open DVD files (*.IFO),
   Open Blu-ray, Ctrl+J (webcam), Ctrl+S (the screen), Ctrl+U (an
   address; with yt-dlp's program set in Settings > Playback, a video
   page), Ctrl+V (files or addresses from the clipboard), Alt+Insert and
   Ctrl+Insert (favorites) and Album/Favorites' recent items, Open > Load
   External Audio.
8. **Playlist panel** (its context menu): the queue, Sort by (duration,
   size, date...), search, edit titles, rename, file information, save
   and open playlists (`.dpl` from PotPlayer too), the thumbnail views (2
   to 7 lines, 16:9), Delete files (to the Recycle Bin, asked first).
9. **Command line**: `rvp.exe /seek=1:00 "/title=Test" file.mkv`, `/add`,
   `/new`, `/clipboard`.

## Open

- Seamless playback (gapless audio for audio albums through mpv's
  playlist, which bends the album rule) and a file navigator: the studies
  left both out; they go in only if the owner asks.
