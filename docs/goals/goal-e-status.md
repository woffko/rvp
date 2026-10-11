# Goal E status: milestone M2, the player window

Dates: 2026-10-10 and 2026-10-11. Result: **done**, with the open items
below. Precondition met: `goal-c-status.md` and `goal-d-status.md` both
record a passed gate.

RVP now plays files in the classic BlackSpace window on Linux and Windows:
`shell/` builds the `rvp` program (cxx-qt, QML and a small C++ video item),
`crates/engine` wraps libmpv and `crates/app` holds everything that does not
need Qt (actions, PotPlayer's key and mouse defaults, settings, the
crash-safe resume store, and the window's chrome and mouse logic).

## Summary

| Deliverable | Result | Where |
|-------------|--------|-------|
| Engine | libmpv through `libmpv2` 6.0.0 with one vendored fix; 30 observed properties feed a property model; events, commands; the wakeup callback only queues a drain on the Qt thread | `crates/engine` |
| Application core | Actions with PotPlayer's command IDs and skin `CmdID`s; key bindings from R1's table 157 (259 keys) and the owner's mouse page; `settings.json` with binding overrides; the resume store (D9) with atomic writes on a writer thread, saved on events and every 2 s during playback | `crates/app` |
| Window | Frameless classic BlackSpace window with live video; the skin painted by a static layer plus one item per control; every bottom-bar control bound to the player (list below) | `shell/` |
| Window behaviour | Move (title bar, empty skin, video drag), resize from edges and corners, maximize and restore (button, title double click), minimize, fullscreen with the skin's overlay bars that show only under the pointer and a pointer that hides after 2 s, drag and drop, file dialog, command line, next and previous file through mpv's playlist, context menu with PotPlayer's texts and the bound keys, tooltips | `shell/qml/Main.qml`, `crates/app/src/pointer.rs` |
| Tests | Headless engine tests; binding tests that carry out every default binding on the engine; resume tests that kill a playing process at pseudo-random points; synthetic-skin tests of the chrome split and the mouse logic; golden images of the idle window against R1's captures | `crates/*/tests` |
| Functional checks | Ubuntu VM: self-test, restart and resume, real-pointer input check, all PASS. Windows VM: self-test, restart and resume, all PASS. Windows host (4 runs with the owner's permission): all functional checks PASS, SVP attach and clean exit PASS, 4K60 0 dropped frames | `docs/test-runs/2026-10-11-ubuntu-vm.md`, `docs/test-runs/2026-10-11-windows-m2.md` |
| Linux kit | Steps `m2build`, `m2`, `m2input`, `m2gpu`, `m2svp` (`kit.sh m2all`) and the owner's M2 section in the checklist | `tools/linux-kit/` |
| CI | Qt 6.10.3 and libmpv on both runners; green | `.github/workflows/ci.yml` |

Bound bottom-bar elements: seek bar (`time-pos`/`duration`, click and drag
seek), volume slider and wheel (`volume`), MUTE (`mute`), current, total and
remaining time, Play/Pause (`pause`, swaps with the state), Previous/Next
(mpv's playlist; right click: 30 s back/forward as the skin's `AltCmdID`),
subtitles on/off (`sub-visibility`), playlist on/off (shows and hides the
attached panel), the title (`[index/count] name`), and the info strip:
container, video codec, decoder output (`DXVA` while hardware decoding runs,
as PotPlayer shows it), video and audio bitrate, audio codec tag, channels,
sample rate, source and output frame rate, video size, percent played; the
values match BlackSpace's `MappingDB` tables. The playlist panel shows mpv's
playlist read-only (editing is M3).

Not bound in M2 (PotPlayer features of later milestones; the buttons press
but do nothing): equalizer, OSD info, the audio stream menu, skin user menus
(filters, video renderer, bookmarks, aspect ratio) and the info strip's page
switch.

## Usability targets (section 4)

Measured with `rvp --selftest` (`shell/src/metrics.rs` defines each
measure). The Windows host numbers are the reference (real hardware); the
Ubuntu VM has no GPU and renders and decodes 1080p60 in software.

| Target | Windows host, final build | Ubuntu VM | Verdict |
|--------|---------------------------|-----------|---------|
| Input to visible reaction < 50 ms | 9-23 ms, one 68 ms | 43-564 ms (software rendering) | Met on the host except one wheel step; the measure starts in RVP's input handler |
| Seek to new frame < 300 ms | 0.9-1.7 s (4K60 HEVC, exact seeks) | 0.4-0.6 s (1080p60 H.264) | **Not met**: exact seeks decode from the previous keyframe (open question 1) |
| Window shown < 500 ms | 0.76-0.81 s (was 1.3-2.0 s) | 0.77-0.97 s | **Not met** on Windows; see the start-up breakdown below |
| First frame < 1 s (local file) | 1.44-1.48 s (4K60 HEVC) | 1.9-2.0 s | **Not met** for 4K HEVC; libmpv starts after the window shows |
| State written on change and at least every 5 s | Events save at once, playback every 2 s, on a writer thread | same | Met; the kill test (6 kills) finds every saved position within 5 s of the last one played |
| Nothing lost after a restart or a crash | Last file reopened paused at its position, volume and window restored | same | Met for M2's state (playlists and the normalizer are M3) |
| No modal interruptions | Errors show as a message over the video for 5 s | same | Met |
| Tooltips on every skin control | The skin's `Tooltip` text after 600 ms | same | Met |
| The menu shows hotkeys | PotPlayer's menu texts with the key bound now | same | Met |
| Every action from mouse and keyboard, configurable, PotPlayer defaults | Table 157 and the owner's mouse page; `keys`/`mouse` in `settings.json` | same | Met for M2's actions; the editor is M3 |

Start-up on the Windows host (final build): QGuiApplication after 36 ms,
the QML engine after 41 ms, `Player.start` at 494 ms (the skin, loaded on a
thread meanwhile, is ready), QML loaded at 638 ms, window shown at 757 ms,
mpv's render context at 872 ms, file loaded at 1086 ms, first 4K frame at
1443 ms. About 450 ms pass while QML creates the Qt Quick window, before
any RVP code runs; on Linux the same step takes about 30 ms headless and
130 ms on the VM. Changes that brought the window from 1.3-2.0 s to 0.76 s:
the menu, file dialog and tooltip load on first use (Qt Quick Controls and
Dialogs are not loaded at start), the skin loads in parallel, and libmpv's
render context is created after the first presented frame.

## Goal D's open item: 4K60 dropped frames

- **Windows host:** 0 dropped frames in 25 s, both fullscreen and in a
  1920x1080 window (4K60 HEVC, `d3d11va-copy`).
- **Ubuntu boot:** not run in this session (it needs the owner's Ubuntu
  boot). The kit's `m2gpu` step plays the 4K60 clip fullscreen and in a
  1920x1080 window and counts drops; `tools/linux-kit/CHECKLIST.md`
  section 5.
- Two causes of drops were found and removed on the way: repainting the
  whole bottom bar with every video frame (regions are now per control and
  playback updates the chrome at most 10 times a second), and disk flushes
  of `state.json` on the GUI thread (now on a writer thread).
- **New finding:** after mpv loops a file (`loop-file`), about 30 frames per
  second dropped until the end of the run (Windows host, fullscreen). Not
  investigated; RVP does not loop by default (open item 3).

## Rules inherited from the gates

| Rule | How M2 follows it |
|------|-------------------|
| 1. Chrome changes during playback ride on mpv's next frame; static layer plus small items | One path: `Player::damaged` → `Tools.repaint`, which waits for `MpvItem::frameUpdate` while a video plays (fallback after 150 ms); the skin is a static layer plus one item per control |
| 2. `.vpy` scripts through the `get_core` wrapper | M6 (RVP runs no `.vpy` in M2) |
| 3. With `wid`, the Qt thread never blocks on mpv | `wid` is not used; drains never block. Synchronous reads remain for playlist changes and the self-test's diagnostics, which are read rarely |
| 4. SVP's `@svp` filter is recognised and left alone | RVP never sets `vf`; the status shows it and the self-test detects `@svp` |
| 5. Linux libmpv with `ao=pipewire`; `current-ao` in diagnostics | The kit builds libmpv with PipeWire (`check` requires the package); `current_ao` is in the self-test's status and diagnostics. The shared Ubuntu VM's libmpv predates the rule (its packages are missing), so it plays through PulseAudio |
| 6. Stop playback before libmpv shuts down; test exit with SVP active | Closing stops playback, waits up to 3 s for mpv to go idle, frees the render context with the window, then asks the core to quit. Exit with SVP active: PASS on the Windows host; on Linux the kit's `m2svp` step |
| 7. Default `video-timing-offset`; no Qt swap times as presentation times | Nothing sets either; `report_swap` is never called |
| 8. SVP's VapourSynth in `~/SVP4/mpv` | The kit's `m2svp` finds it as S2 does; M6 |

## Found and fixed

- libmpv built without Lua (the kit's) has no `osc` and `ytdl` options;
  setting them stopped libmpv from starting. They are set best-effort.
- Qt builds and drops the video renderer for every grab of a hidden window
  (the self-test in Windows session 0). Freeing mpv's render context there
  disabled video for the file; it is now freed only when the window closes.
- On Wayland a window state request made before the compositor applied
  the previous one is lost (leaving fullscreen right after entering it,
  restoring right after maximizing). RVP remembers the state it asked for
  and asks again for 2 s; changes the window system makes on its own are
  followed.
- cxx-qt-build 0.10 compiles the QML before it writes the build's type
  information, so incremental builds used stale method indexes and crashed
  at start after a bridge changed. `shell/build.rs` deletes the stale file.
- The skin engine drew the seek bar's `AltImage` track with nothing loaded;
  R1's idle captures show no track. The rule is now: no media, no seek bar
  track; `AltImage` for media that cannot be sought (`crates/skin`).
- The first self-test measured its own load: it read 16 mpv properties
  synchronously on the GUI thread 20 times a second.

## Golden images (R1)

`crates/app/tests/golden.rs` renders RVP's idle window as the application
builds it (data model, hover, pressed) and compares it with R1's idle
captures at 1280x720, 1920x1080 and 2560x1080 outside the video area and the
playlist list (the owner's files). Mean differences per channel: 0.17-0.34
plain and hover, 0.62-1.23 pressed (tolerance 1.5). The pressed captures
contain PotPlayer's "Play" tooltip, which accounts for most of the
difference; the volume bar also differs, because the captures show the
owner's PotPlayer volume and RVP's idle window shows 100 %. The test needs
the local skin and captures and passes with a note without them.

## Differences from PotPlayer (known, deferred or unverified)

- PotPlayer keeps the window at the video's aspect ratio while resizing
  during playback (R1 finding 1); RVP does not yet (M3).
- Opening a file resizes the window to show the video at 100 % within the
  screen, as PotPlayer does (`fit-window-to-video`, on by default); a
  reopened last file keeps the saved window size.
- In fullscreen RVP shows the bars of `MainSkinAreaOscInside` under the
  pointer; what PotPlayer shows in classic fullscreen is not captured yet.
- A single click on the video acts at once; when it turns into a double
  click, the toggled action (Play/Pause) is undone before fullscreen.
- Esc leaves fullscreen when nothing else is bound to it.
- BlackSpace's MUTE button has no `DownFlag`, so it shows no muted state.
- Tooltips show the skin's text as is, including the `< >` separator
  between the left- and right-click descriptions.
- PotPlayer draws its logo in the idle video area; RVP leaves it black.

## Owner decisions in this goal

- 2026-10-10: `unsafe` only in `shell/` (the bridge files and the
  `QPainter` entry point), all other crates `forbid`.
- 2026-10-10: `libmpv2` with the vendored fix; LGPL-2.1 exception for
  `libmpv2` and `libmpv2-sys` in `deny.toml`.
- 2026-10-11: seek steps stay 5 s, 30 s, 1 min and 5 min; the volume step is
  1 % (the owner's `AudioVolumeUnit=1`).
- 2026-10-11: four runs on the Windows host desktop, each approved in
  advance.

## Host-side effects and cleanup

- `C:\Users\<user>\rvp-test` gained `rvp-bundle` (the Windows build, 236 MB),
  `run-m2.cmd`, `m2-config`, `m2-state` (RVP's test settings and state) and
  `m2-out` (grabs of RVP's own window). They can be deleted.
- No PotPlayer, SVP or system setting was changed; PotPlayer stayed open and
  idle and was not touched. SVP attached to RVP through `mpvpipe` in the
  second run, as it did to the spike in goal C. No input was injected.
- The Windows test VM has `C:\rvp-test` (bundle, fixtures, scripts). The
  Ubuntu VM has `/opt/rvp-kit/src/rvp`, `/opt/rvp-kit/logs-m2` and two
  wrapper scripts; every ACL and pointer setting the runs changed was
  restored.

## Versions

| Component | Version |
|-----------|---------|
| Rust | 1.98.1 |
| Qt | 6.10.3 (Linux `gcc_64`, Windows `msvc2022_64`) |
| cxx-qt, cxx-qt-lib, cxx-qt-build | 0.10.0; cxx 1.0.204 |
| libmpv | Windows: shinchiro `20261010-git-b2c255c13e`; Ubuntu VM: the kit's build of mpv `b2c255c13e`; WSL and CI tests: Ubuntu 24.04's libmpv 2 (mpv 0.37) |
| Windows host | 2560x1440 at 59.951 Hz, RTX 30-series card, `d3d11va-copy`, WASAPI |

## Open questions for the owner

1. **Seeks.** Exact seeks take 0.9-1.7 s on the 4K HEVC clip (the decoder
   runs from the previous keyframe; SVP is happiest with exact seeks and
   `hr-seek-framedrop=no`). Keyframe seeks for the arrow keys would be fast
   but land up to a few seconds off. Keep exact seeks, or make the short
   steps keyframe seeks (PotPlayer's own default is not known)?
2. **Window shown in 0.76 s on Windows** (target 0.5 s). The remaining time
   is Qt creating its Qt Quick window and OpenGL before RVP's code runs.
   Accept for now, or spend more effort (for example a native splash of the
   static skin)?
3. **Dropped frames after a loop** (fullscreen, after `loop-file` restarted
   the clip). Investigate now, or when looping becomes a feature?
4. **Ubuntu boot:** run `tools/linux-kit/kit.sh m2all` (checklist
   section 5) for the Linux GPU recheck of goal D's open item, the SVP exit
   check and the start-up numbers on real hardware.
