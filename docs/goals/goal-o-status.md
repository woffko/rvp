# Goal O status: milestone M13 (PotPlayer parity: subtitles)

Date: 2026-10-21. Result: **done**. M13 is built and checked in WSL, on
the Windows VM and in one host session on the RTX 30-series card at the power
limit. Every Subtitles item of R1's menus (static and run-time), its keys
and R2's subtitle pages works or keeps a reason. The final build is in
`C:\Users\<user>\rvp-test\rvp-o`.

Precondition met: `goal-t-status.md` records goal T as done. The owner
reported no findings from goal T's build, so nothing came before M13.
Online subtitles (search, download, translation, word search and the
AngelScript host) stay deferred by the owner's decision of 2026-10-16.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Second subtitle | A second subtitle shows at the same time (`secondary-sid`, its position `secondary-sub-pos`). Combine Subtitle loads a file as the second subtitle. RVP picks the second subtitle by its own languages, since mpv has no `secondary-slang`. The chosen tracks are remembered per file. | `crates/app/src/subtitles.rs`, `resume.rs` |
| Reload and copy | External files that change on disk reload (`sub-reload`, checked once a second by one `stat` per file). Ctrl+Alt+Y reloads by hand. The current subtitle copies to the clipboard (`sub-text`). | `subtitles.rs` |
| Style | Bold (Alt+B), italic, colours with their opacity (`#AARRGGBB`), outline, shadow, blur, letter spacing (Alt+Num6/4), line spacing (Alt+Num8/2), the letters' width and height (`ScaleX`/`ScaleY`), the opaque box (`sub-border-style`), alignment, vertical position and margins. ASS scripts take RVP's font, colour and scales as far as `sub-ass-override` lets them. Unset numbers and colours stay libmpv's own defaults (Linux's 0.37 and Windows' differ). | `subtitles.rs`, Font Style page |
| Sideways moves | Alt+Left and Alt+Right go from the left edge, with a growing margin, through the centre to the right edge (`sub-align-x`, `sub-margin-x`, 20 px steps up to 300). mpv has no free horizontal position. | `subtitles.rs` |
| Placement | Bitmap subtitles on the whole screen (`stretch-image-subs-to-screen`). Text subtitles inside the video (`sub-use-margins` off). Forced under the video (`sub-ass-force-margins`, `sub-pos` 100, M12's bottom margins). Aspect correction of anamorphic scripts. | Subtitles page |
| Sync | The Adjust Subtitle Sync dialog (a slider of ±10 s and a field of ±10 min) and the rate the subtitles were made for (Convert Subtitle FPS, `sub-fps`). PotPlayer's three sync steps (`<`/`>`, Ctrl, Alt). Ctrl+Alt+`.`/`,` sync to the next or previous subtitle (`sub-step`). Keeping the sync for the next files. "Apply sync changes starting from next position" moves only the later subtitles of a subtitle file RVP edits. | `subtitles.rs`, `SubtitleSync.qml` |
| Language and loading | Preferred languages, only preferred languages (`subs-fallback`), loading by file name (`sub-auto`), extra folders (`sub-file-paths`), the fonts folder, removing hearing-impaired notes and empty lines. A text encoding for files that are not UTF-8 (`sub-codepage`), which RVP's own reader follows. | Language/Sync and Location pages |
| RVP's reader and writer | `crates/subtitle` (rvp-subtitle) reads SRT, ASS/SSA and WebVTT in UTF-8, UTF-16 and legacy code pages (`encoding_rs`). ASS keeps its header, fields and comments; tags convert between the formats. It writes UTF-8 (CRLF for SRT and ASS). | `crates/subtitle` |
| Subtitle Saving | Apply Subtitle Sync (Alt+S) writes the times as they show, with the sync and the frame rate conversion, into the file that shows, and sets the sync back to none. Save as Video Filename (Ctrl+Shift+S) and Save As (Ctrl+Alt+S) do the same elsewhere. A file RVP writes over goes to `<state>\subtitles\backup\` first. | `crates/app/src/subtitle_editor.rs` |
| Browser and editor | Alt+E opens the cues of the subtitle file that shows. It follows the one that shows, jumps on a double click, and edits text and times (Now, Sync here). It also inserts, deletes, splits, joins and shifts cues, with undo and redo, Save and Save As. Edits show at once through a working copy mpv loads in place of the track. Settings: times, the times with the sync, opacity, font, size, lines and following. | `subtitle_editor.rs`, `SubtitleBrowser.qml` |
| Input Subtitles | Alt+P: each line typed becomes a cue from where its typing began (3 s while the video stands). The lines go into the subtitle file that shows or into new subtitles, which Save writes under the video's name. | same |
| Subtitles from audio | Create Subtitles from Audio runs whisper.cpp offline. "Start" makes the whole file into `<name>.<language>.srt` next to the video, or into the state folder. Two automatic modes start it when a file starts. Real time makes 30 s pieces ahead of the playback position, and again after seeks, as one track. Menus and a settings page cover the engine, the model and the language for each mode, the voice filter and the folders. A job on the GPU is a step of the GPU power budget. | `crates/app/src/speech.rs`, Subtitles from Audio page |
| whisper.cpp | RVP's own build of v1.9.5 serves the CPU and Vulkan engines, with ggml's backends loaded at run time; it needs only Windows' DLLs and the driver's Vulkan. CUDA and BLAS are whisper.cpp's own builds, which the user adds. The user downloads the models; RVP downloads nothing. | `tools/whisper/build.sh`, `package.py --whisper` |
| Menus | RVP's Subtitles menu now follows R1's run-time menu. It adds RVP's lists of whisper engines, models and languages, and keeps Add/Select Subtitles with the track list. | `crates/app/src/menu.rs` |

## Coverage

`docs/coverage/menus.md` and `preferences.md` are rewritten. No Subtitles
item is left "planned" or "not in RVP yet": a test
(`every_subtitle_item_and_hotkey_works_or_says_why_not`) checks the 300 and
more static and run-time items and each of PotPlayer's 30 and more
subtitle keys. A preferences test does the same for every option of the
subtitle pages.

Each item RVP lacks keeps a reason:

- DirectShow output surfaces and the VMR/EVR smooth output.
- libass's font renderers, GDI charsets, the second outline, the edge
  blur, ruby text, and the always-on animations.
- The fade: libass fades only ASS events that ask for it. An ASS copy of
  an SRT file would lose RVP's style settings.
- 3D subtitles: mpv draws subtitles once, over the finished picture (goal
  N's study found no way).
- SAMI's language classes and line options (FFmpeg reads one track).
- The shortest and longest showing times, the overlap and the case of
  closed captions (mpv shows events as they are).
- The second font and making only the browser's background see-through.
- Const-me's Whisper and Purfview's faster-whisper builds (see the study).
- Online: deferred by the owner.

## The study

`docs/spikes/m13-whisper.md` has the study. RVP takes whisper.cpp's CPU,
BLAS, Vulkan and CUDA engines and leaves out PotPlayer's other three:

- Const-me's Whisper: no release since 2023, Windows only, older models.
- Purfview's faster-whisper builds: no licence, 0.1 to 1.4 GB bundles for
  CUDA or the CPU, and they fetch their models online.

RVP runs whisper.cpp's command line as a process, not as a library:

- a crash or a hung driver stays out of RVP;
- no `unsafe` code is needed;
- RVP's build and the official ones work alike.

## Tests

- `crates/subtitle`: 16 tests. Round trips of generated SRT, ASS and
  WebVTT files, conversions, encodings (BOMs, labels, guesses), tolerant
  reading, sync and frame rate. Round trips also run on free samples kept
  in `~/rvp-reference/subtitles` (`RVP_SUBTITLE_SAMPLES`, 19
  files: Sintel's SRTs, libass-tests' scripts, web-platform-tests'
  WebVTT; never committed). The editor's model has 4 tests.
- `crates/app/tests/subtitles.rs` runs on a headless player:
  - the style, placement and encoding settings as mpv's options;
  - the browser's edits shown at once, undo and redo;
  - Save with the sync into the file, with the backup;
  - the frame rate conversion saved;
  - sync steps from the next subtitle;
  - typed subtitles saved under the video's name;
  - unsaved edits kept when another file starts.
- `crates/app/tests/speech.rs` uses a synthetic speech clip (espeak-ng)
  and the tiny model, and covers the whole file next to the video and
  real time. It passes with a note when whisper-cli, the model or the
  clip is missing.
- `bindings.rs` checks every new default key: Alt+B, Alt+arrows,
  Alt+Num, Ctrl/Alt+`<`/`>`, Ctrl+Alt+`.`/`,`, Alt+E, Alt+P, Alt+S,
  Ctrl+Shift+S and Ctrl+Alt+S. `coverage.rs` checks the subtitle items,
  and unit tests cover the settings checks, colours, the menu IDs and
  whisper's output.
- `rvp --selftest=m13` checks, through Player's handlers:
  - Alt+B;
  - the browser and an edit shown at once;
  - Input Subtitles;
  - Save As elsewhere;
  - the sync dialog, syncing to the next subtitle and Combine Subtitle;
  - with `--speech-check`, subtitles from the audio;
  - the settings pages.

  It passes in WSL under Xvfb.
- `rvp --selftest=speech FILE` times the whole file and real time; it is
  used in the host session.
- `cargo xtask ci` passes before every commit; GitHub CI is in "CI" below.

## Checks on the Windows VM

`tools/windows-vm/m13-all.ps1` on the VM, with the bundle and its
`whisper\`:

- The M13 self-test: 17 checks, all pass. Subtitles from audio ran with
  the bundle's whisper.cpp on the CPU. The fixture's subtitle file stays
  unchanged.
- whisper.cpp on the speech clip (tiny, `-ng`): "Hello world, this is a
  test on some titles made from speech." in 4.5 s on 2 cores.
- Headless tests on Windows' FFmpeg: `formats` (6), `editor` (4),
  `subtitles` (6) and `speech` (2) pass.
- M11's checks run again after the menu change: 47 pass.
- The functional self-tests of `m7-all.ps1` (RVP Default, then
  BlackSpace) pass: 178 checks.

In WSL under Xvfb, `--selftest` (87 checks), `--selftest=m13 --speech-check`
and `--selftest=speech` pass.

## The host session (2026-10-21, with the owner's permission)

`tools/windows-vm/m13-host.ps1` ran on the RTX 30-series card at 90 % (315 W)
through `gpu-limit.ps1`, which put 350 W back afterwards. nvidia-smi saw
the card before and after every run. The input was a 95 s espeak-ng clip
of RVP's own text (`tools/test-clips/make-m13-speech.sh`).

| Run | Time | Speed | GPU at most | Words found |
|-----|------|-------|-------------|-------------|
| whisper.cpp base, CPU (`-ng`) | 10.5 s | 9.1x | idle | 4/4 |
| whisper.cpp base, Vulkan, first run | 31.4 s | 3.0x | 118 W, 79 % | 4/4 |
| whisper.cpp small, CPU | 31.5 s | 3.0x | - | 4/4 |
| whisper.cpp small, Vulkan | 4.0 s | 23.7x | 155 W, 40 % | 4/4 |
| RVP `--selftest=speech`, Vulkan, small | the whole file in 4.7 s (20 lines); real time's first line after 2.5 s of playback | | 143 W, 46 % | |

- The first Vulkan run builds ggml's Vulkan pipelines (about 25 s once).
  The driver keeps them, and later runs took 4 to 5 s.
- The small model heard "A media player has a simple task on paper. / It
  opens the file, reads the pictures and the sound, / and shows them
  together at the right time."

## Findings

1. libmpv 0.37 (Linux) refuses `sub-margin-x` above 300, so the 400 of
   the first sideways moves stayed unset. Margins are now 0 to 300 on
   every libmpv.
2. The window's thread drains the app only on mpv's wakeups. The speech
   jobs' events also come in the shell's one-second tick. Before that, a
   paused file never got its subtitles.
3. A file's duration can arrive after `file-loaded` (Windows). The job
   now reads it from mpv when the model does not have it yet.
4. On the 2-core VM, RVP's software rendering left a whisper.cpp job
   below normal priority 25 times slower than alone. The whole file's job
   now runs at normal priority on all cores but one. Real time stays
   below normal (`nice` on Linux) so playback keeps the CPU first. On the
   VM, real time therefore cannot keep up with a 4 s clip; on the host it
   showed its first line after 2.5 s.
5. whisper.cpp starts its lines with a space and marks silence
   `[BLANK_AUDIO]`. Both are dropped, and a file without speech says "no
   speech found".
6. The recipe's libc++ does not give whisper.cpp's examples
   `<algorithm>` on its own. The build includes it in every file.

## Differences from PotPlayer

- Only external text subtitle files can be saved, browsed, edited or
  shifted from the next subtitle on. libmpv gives no access to the lines
  of a track inside the video. Saved files are UTF-8.
- Horizontal placement: an edge or the centre with a side margin (no
  free position). The second subtitle uses the first one's style. No
  fade for subtitles that are not ASS, no second outline and no ruby
  text.
- Subtitles from audio use whisper.cpp only: Const-me and faster-whisper
  are left out (see the study). Real time works on 30 s pieces read ahead
  of the playback position. A live stream cannot be read ahead, so it gets
  no real-time subtitles. Models are not downloaded for the user.
- The browser is see-through as a whole, not only its background.
- Online subtitles, translation and word search: deferred by the owner.

## Linux items for goal S

- libmpv 0.37 lacks `sub-border-style` (the opaque box),
  `sub-line-spacing` (0.37 uses `sub-ass-line-spacing`, which RVP falls
  back to) and `secondary-sub-pos`. The opaque box and the second
  subtitle's position need libmpv 0.38 or later in the Linux packages.
- whisper.cpp for the Linux packages: a CPU and Vulkan build like
  Windows'. Today RVP finds `whisper-cli` on the PATH or in
  `~/.config/rvp/whisper/<engine>`.
- The browser's opacity: Wayland has no window opacity.
- The host session's GPU checks on the owner's Ubuntu boot.

## What to try

In `C:\Users\<user>\rvp-test\rvp-o` (portable; `whisper-models` already has
`ggml-tiny.bin` and `ggml-base.bin`). Larger models go into the same
folder, from https://huggingface.co/ggerganov/whisper.cpp; small is a
good start on the RTX card.

1. **Style**: Alt+B, Alt+Left/Right, Alt+Num4/6/8/2, Alt+PgUp/PgDn.
   Preferences > Subtitles > Font Style (colours with their opacity, the
   opaque box, letter width and height, margins).
2. **Placement**: Subtitles > Display text subtitles inside the video,
   and Force subtitles to display under the video (with Video > Bottom
   Margin).
3. **Sync**: `<` `>`, Ctrl and Alt with them, and Ctrl+Alt+`.` to sync to
   the next line. Subtitles > Subtitle Sync > Adjust Subtitle Sync...,
   and Convert Subtitle FPS.
4. **Second subtitle**: Subtitles > Add/Select Subtitles > Combine
   Subtitle...; second languages on the Language/Sync page.
5. **Browser**: Alt+E on a film with an SRT file: edit a line, use Sync
   here, then Save (the old file goes to `state\subtitles\backup`).
   Alt+P types new lines.
6. **Saving**: Alt+S after shifting the sync; Ctrl+Shift+S; Ctrl+Alt+S.
7. **Subtitles from audio**: Subtitles > Create subtitles from audio >
   Whisper AI model > small (after downloading it), Whisper AI engine >
   whisper.cpp Vulkan, then Start. Real time: Create subtitles from
   audio (Real-time) > Always use.

## CI

GitHub CI passed for 9ba4624 (the first push of the goal: 56fca56 to
9ba4624). For c470d92 every job passed but `package-windows`, twice: on
the hosted runner `m8-smtc.ps1` saw RVP's first file still `Opened`, with
Pause and Next off, after its 10 s waits. The media controls are the same
code as before; goal T saw the same once (690a502). The waits are now
30 s (the check is about the controls, not about how fast a file starts),
in the commit of this file.

## Open

- Nothing of M13 is open. The owner's tries of the build may bring
  findings for the next goal.
