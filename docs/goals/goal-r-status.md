# Goal R status: milestone M16 (DVD and Blu-ray menus)

Date: 2026-10-31. Result: **done**. M16 is built and checked in WSL
(headless tests and the self-test under Xvfb), on the owner's own DVD
copy (headless, in WSL) and on the Windows VM. The final build is in
`C:\Users\<user>\rvp-test\rvp-r`.

Precondition met: `goal-q-status.md` records goal Q as done. The owner's
finding from goal Q's build (the fullscreen bar moved the picture) was
fixed in goal Q (cfb2b34), so nothing came before M16.

## The owner's decisions

- **RVP's own navigator** (option B of `docs/spikes/s7-disc-menus.md`),
  which approves libdvdnav and libdvdread (GPL-2.0-or-later) and libbluray
  (LGPL-2.1-or-later) under D8 (2026-10-29).
- **libdvdcss in CI's private artifacts is acceptable**: CI keeps
  shinchiro's libmpv to build and test; packages made locally use RVP's own
  libmpv, which has no libdvdcss.
- **A synthetic Blu-ray** (2026-10-30): the owner's Blu-ray copy ("Armour
  of God", a remux) has no menus (one playlist, no IG stream), so RVP
  authors its own test disc.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Study | VLC's, Kodi's and mpv's (old and new) disc code; mpv master's own menus (option A); libmpv's custom streams; the finding that RVP's earlier libmpv contained libdvdcss. | `docs/spikes/s7-disc-menus.md` |
| libmpv without libdvdcss | RVP's libmpv build drops libdvdcss (`no-libdvdcss.patch`) and fails if any `DVDCSS_` string remains; two more patches follow upstream changes (curl without libssh, FFmpeg without `--enable-opengl`). mpv v0.41.0-1371-g6ee40e64d. | `tools/libmpv/` |
| Disc libraries | `libdvdnav-4.dll` (libdvdnav 7.0.0 with libdvdread 7.1.1 inside, both with GCC's bit-field layout, no libdvdcss) and `libbluray-4.dll` 1.5.1 with its BD-J jars; they need only Windows' own DLLs. | `tools/discs/build.sh` |
| Segments | mpv plays a disc as segments `rvpdisc://N` of a custom stream (libmpv's `stream_cb`): one per still, VM jump, playlist, or cells that go back (a looping motion menu); a title's segment seeks by sectors (DVD) or bytes (Blu-ray) and shows its time and length. | `shell/src/disc` |
| DVD menus | libdvdnav runs the disc's program; RVP decodes the menu's sub-picture (RLE, the disc's palette, the selected button's colours) and shows it with `overlay-add` over the video; arrows, Enter and the pointer (moves select, a click activates); timed stills are waited out, endless ones hold; the program's audio and sub-picture streams play. | `crates/app/src/disc/spu.rs`, `overlay.rs`, `shell/src/player/m16.rs` |
| DVD Control | Previous, Root, Title, Chapter, Audio, Subtitle and Angle Menu (PotPlayer's IDs 10109-10115, Alt+BackSpace for the title menu) for the disc that plays. | `crates/app/src/action.rs` |
| Blu-ray menus | libbluray runs HDMV programs (and BD-J where the user's Java runs); its overlay events draw the interactive graphics plane (palette runs, BT.709 on HD planes; BD-J's ARGB) that RVP shows over the video; keys and the pointer go to libbluray while a menu shows; Title and Root Menu call the top menu, the others the pop-up menu. | `crates/app/src/disc/bluray.rs`, `shell/src/disc/bluray.rs` |
| Settings | Device page: play whole DVDs and Blu-rays through their menus (on by default), the DVD menu language (two letters), going straight to a DVD's menu past its opening; the note that encrypted discs need the user's own libdvdcss or libaacs and keys. The disc's machine prefers RVP's audio and subtitle languages. | `sources.rs`, `Settings.qml` |
| Encryption | RVP ships none of libdvdcss, libaacs or keys; libdvdread and libbluray load the user's own from RVP's folder. An AACS disc without them gets a message and plays nothing. | |
| Packages | `package.py --discs` puts the two DLLs, the jars and the libraries' licences beside `rvp.exe` and refuses decryption files. | `tools/packaging/package.py` |

## Tests

- `tools/test-clips/make-m16-dvd.sh`: a DVD authored with dvdauthor and
  spumux (synthetic video, tones and frames): a main menu with three
  buttons, a title set's chapter, audio, subtitle and root menus, title 1
  with two chapters, two audio streams, a sub-picture stream and a 3 s
  still, and title 2.
- `tools/test-clips/make-m16-bluray.py`: a Blu-ray with an HDMV menu
  (three buttons in an IG stream RVP's script writes and muxes into
  FFmpeg's m2ts), two titles, chapters, two audio streams and EP maps.
  Neither disc is committed; CI makes both.
- `shell/src/disc/tests.rs` (headless libmpv): the DVD (the main menu's
  still and first button, arrows and the pointer, a click that plays title
  1, its 3 s still, the menu again, the chapter menu's second chapter, the
  title menu from a title) and the Blu-ray (the menu and its first button
  by the plane's colours, arrows and the pointer, a click that plays title
  1 with the first audio PID, the top menu, the audio button, the menu
  after the title, title 2 from chapter 2). With `RVP_DVD_REAL` a real
  disc's opening plays to a menu with buttons and its picture, and a
  click starts what follows: the owner's "Семьянин" (two warnings, a
  looping motion menu, button 1 plays title 3) passes.
- Unit tests: SPU decoding and assembly from program-stream packs, the
  highlight's colours and contrast, scaling, the Blu-ray plane (runs,
  wipes, hiding, BD-J's ARGB, the picture's box).
- `rvp --selftest=m16 --dvd-menus=DIR --bd-menus=DIR`
  (`shell/qml/SelfTestM16.qml`): 17 DVD checks (the highlight, arrows,
  pointer, click, the machine's audio, DVD Control's Audio, Subtitle,
  Chapter and Root Menu, the chosen audio and subtitles, a timed still of
  3 s, the menus turned off) and 10 Blu-ray checks (the menu, arrows and
  pointer by its picture, a title with the first audio PID, the Title
  Menu, the second audio PID, chapter 2, the menus turned off). WSL
  (Xvfb): 27 of 27.
- `rvp --selftest=m11`'s DVD check turns the menus off (it checks mpv's
  `dvd://`).
- The bindings, menus and translation tests know the new actions.

## Checks on the Windows VM

`tools/windows-vm/m16-all.ps1` with the package's bundle, both synthetic
discs and the shell's tests built for Windows: all pass.

1. The bundle carries `libdvdnav-4.dll` and `libbluray-4.dll` and no
   libdvdcss, libaacs or keys.
2. `--selftest=m16`: 27 of 27 (the DVD's 17 and the Blu-ray's 10), and
   both parts ran (not skipped).
3. The headless navigation tests: both pass.

Two faults appeared only on Windows and were fixed: libdvdread and
libdvdnav disagreed on the bit fields' layout (button rectangles were
garbage), and a segment that mpv opened after the next one was prepared
took its data (segments are bound to their address now).

## Findings

See `docs/spikes/s7-disc-menus.md`, "What the implementation found": the
address-bound segments, the title's last sector, cells that go back, the
bit fields, libbluray's events and titles, libbluray 1.3.4's crash
without an EP map, and the synthetic Blu-ray.

## Differences from PotPlayer

- Between segments (a still, a jump, each loop of a motion menu) mpv
  loads the next one: a short pause where PotPlayer's DirectShow
  navigator plays on.
- An audio or subtitle track picked in RVP's own menus during a disc's
  title is not told to the disc's program: the next segment plays the
  program's choice again.
- DVD subtitles in titles come from mpv's decoder without the disc's
  palette (FFmpeg guesses one): their colours may differ.
- A Blu-ray pop-up menu's button during playback runs at libbluray's next
  read: what mpv had buffered (a few seconds) is skipped. Blu-ray has no
  Previous Menu; Chapter, Audio, Subtitle and Angle Menu open its pop-up
  menu.
- A disc's title shows its own time and length, and the seek bar has no
  chapter marks (mpv sees one segment, not the disc).
- "Auto Skip" is RVP's reading: straight to the disc's title or root menu
  (a Blu-ray's top menu) after the first play started.
- BD-J menus are built in but untested: no BD-J disc and no Java on the
  test machines. A disc whose titles are all BD-J refuses without Java.
- Blu-ray languages are libbluray's defaults (DVD takes RVP's languages).

## Linux items for goal S

- The Linux packages: bundle `libdvdnav.so.4` and `libbluray.so.2` (or
  depend on the system's) and libbluray's jars; `check-deb.sh` should run
  `--selftest=m16` with both synthetic discs.
- CI's Linux job already runs the headless tests on Ubuntu 24.04's
  libdvdnav 6.1.1 and libbluray 1.3.4; the Ubuntu VM and the owner's
  Ubuntu boot should run `--selftest=m16` on a desktop (Wayland: the
  pointer's mapping) and try a real disc drive.
- libbluray's BD-J with a Linux JRE.

## What the owner should try

1. In `C:\Users\<user>\rvp-test\rvp-r`, open the DVD folder
   `V:\CURRENT_VIDEOS\DVD Семьянин (The Family Man) ...` (Open > DVD, or
   drop the folder): the warnings, then the motion menu with its buttons;
   arrows and Enter, the mouse; Alt+BackSpace (Title Menu) during the
   film; DVD Control's other menus.
2. Preferences > Device: the menu switches, the menu language, going
   straight to the menu.
3. Any Blu-ray copy with menus you find (a full disc folder or ISO, not a
   remux): Title Menu for the top menu, Chapter Menu for the pop-up menu.
4. An encrypted disc only with your own libdvdcss-2.dll or libaacs.dll
   and keys beside `rvp.exe`.

## CI

Recorded after the push.

## Open

- BD-J on a disc with Java (none available).
- The owner's DVD copy is kept locally in
  `~/rvp-reference/discs/family-man` for later tests (never
  committed).
