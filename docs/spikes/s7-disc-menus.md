# S7: DVD and Blu-ray menus (M16, goal R's study)

Date: 2026-10-25. Sources: VLC master and 3.0.x (`modules/access/dvdnav.c`),
Kodi master (`DVDInputStreamNavigator.cpp`, `DVDInputStreamBluray.cpp`,
`VideoPlayer.cpp`), mpv's history (the menu code of 2013-2015, its removal)
and master (`stream/stream_dvdnav.c`, `demux/demux_disc.c`,
`player/discnav.c`, `stream/stream_bluray.c`), libdvdnav, libbluray
(`bluray.h`, `overlay.h`, `keys.h`), libmpv's `stream_cb.h` and the
vendored libmpv2 6.0.0, shinchiro's mpv-winbuild-cmake at RVP's pin, and
`strings` on RVP's own `libmpv-2.dll` files. Nothing was run from the
sources. The synthetic DVD (`tools/test-clips/make-m16-dvd.sh`) was walked
with libdvdnav to check it.

## Findings

### 1. mpv plays disc menus again (master, not yet released)

PLAN.md's M16 starts from "mpv removed DVD and Blu-ray menus". That was
true from 2015 to 2026. mpv master brought them back:

- PR #18080, "Add DVD / Blu-Ray menu and DVD-Audio support", merged
  2026-08-25; PR #18416, "BD/DVD more options and control over VM",
  2026-10-21; PR #18662, sparse stream handling, 2026-10-24. No release
  has them (0.41.0, 2025-12, is the newest tag).
- The interface: `--disc-menu` (or `dvd://menu`, `bd://menu`) boots into
  the disc's menu; the `disc-menu-active` property; the command
  `discnav <action> [<x> <y>]` with `up`, `down`, `left`, `right`,
  `select`, `menu`, `title-menu`, `popup`, `audio-menu`, `sub-menu`,
  `angle-menu`, `up-menu`, `prev`, `red`, `green`, `yellow`, `blue`,
  `mouse-move`, `mouse-click` (x and y from 0 to 1 of the source frame).
- How it works, and what the old code got wrong:
  - **Highlights.** The button's colours replace sub-picture pixel values
    0 to 3 inside the button's rectangle only (the DVD rule); mpv keeps the
    menu's one-time sub-picture and delivers it again after a flush. The
    2015 code drew a grey box instead, one reason it was removed. Blu-ray
    IG planes (libbluray's overlays) are drawn as mpv OSD bitmaps scaled
    from the video's size, in every renderer, the Direct3D 11 child window
    included.
  - **Stills.** An endless still ends the read with EOF so the decoder
    drains and the menu frame shows, and the player holds the file open
    until the user navigates. **Timed stills are skipped**, not held.
  - **Jumps.** At every VM jump the stream holds an EOF until the demuxer
    has flushed, then the player resyncs with an ordinary seek; positions
    map to timestamps through per-VOBU segments. Tracks follow the VM's
    choice at a jump.
  - **No read-ahead** while a menu is active: reading would run the disc's
    VM ahead of what shows.
- RVP's Windows DLLs (shinchiro's 2026-10-10 build `b2c255c13`, and RVP's
  own 2026-10-19 build `6a262d11f`) have the August part (`discnav`,
  `disc-menu-active`, `title-menu`, `popup`, the mouse actions) but not
  the October fixes (audio, subtitle and angle menus, `up-menu`, mouse
  clicks mapped through the shown rectangle, resync at every jump, the
  menu sub-picture after a resync, tracks at a jump). A libmpv built from
  today's master has them.
- Open upstream issues: #18423 (a menu that does not show right), #18481
  (external ASS subtitles out of sync after BD menu navigation), #18670
  (opening `VIDEO_TS.IFO` or `index.bdmv` should start the menu); a report
  in #18080 of frozen video after choosing a title.
- Linux: Ubuntu's libmpv 0.37 has none of it.

### 2. RVP already ships libdvdcss

shinchiro's recipe at RVP's pin builds libdvdread with
`-Dlibdvdcss=enabled` and libdvdcss as a static library, so both DLLs
contain libdvdcss (its `DVDCSS_METHOD`, `DVDCSS_CACHE`,
`DVDCSS_VERBOSE` strings), and so do the packages made with them: the
portable folders `rvp-o`, `rvp-p`, `rvp-q` on the host, the installers,
and CI's package artifacts (CI downloads shinchiro's 2026-10-10 build).
PLAN.md (M16) and goal R say RVP never ships it. The fix is RVP's own
build with `-Dlibdvdcss=disabled`: libdvdread then loads the user's
`libdvdcss-2.dll` at run time, if the user puts it beside `rvp.exe`
(`shell/src/process.rs` keeps the library search to RVP's folder,
System32 and its VapourSynth runtime). CI cannot build that libmpv (30 to
40 GB, hours); see the decision below.

### 3. How others drive a disc (for an RVP-made navigator)

- **VLC** reads one 2048-byte block per call (`dvdnav_get_next_block`),
  parses the packs itself, drains its decoders at a still (3.0 injects a
  fake sequence end; master waits until the output is empty, up to 2 s),
  calls `dvdnav_still_skip` from a timer for timed stills, waits for the
  output to empty before `dvdnav_wait_skip`, deletes every stream at a
  VTS change, resets the clock at a cell change, takes audio and
  subtitles from the VM, and draws a highlight by cropping the menu
  sub-picture to the button with the button's palette.
- **Kodi** holds its reader at a still so the frame stays, skips timed
  stills from a timer, reopens the demuxer at a VTS change, recolours the
  pixels in the highlight rectangle with the CLUT from `SPU_CLUT_CHANGE`,
  and maps the mouse to source pixels. Its Blu-ray input runs a
  `bd_read_ext` loop, reopens the demuxer at each playlist or play item
  (no seamless branching), keeps the HDMV PG and IG planes from
  libbluray's overlay callbacks and shows them as forced subtitles, and
  starts BD-J only with libbluray's jars beside the library and a Java
  found by libbluray (`JAVA_HOME`, the registry). Its gaps: BT.601 colours
  for HD menus, no `BD_OVERLAY_HIDE`, no `bd_set_scr`, no menu sounds.
- **libmpv's custom streams** (`stream_cb`, wrapped by libmpv2's
  `protocol`, which never sets `cancel_fn`) deliver bytes only: no
  discontinuity, no stream controls. A read that blocks is safe but the
  MPEG-2 decoder keeps the last picture until the next one or a drain, so
  a still never shows without returning EOF; after EOF mpv stops reading
  until a seek or a new `loadfile`. An RVP navigator through `stream_cb`
  therefore needs one `loadfile` per segment (up to a still, a jump or a
  title-set change), each with a gap, and draws highlights itself (an
  `overlay-add` BGRA image in window pixels, or a QML layer, which the
  Direct3D 11 child window cannot show over the video).

### 4. Blu-ray test material

No freely licensed Blu-ray with HDMV menus exists (libbluray and Kodi have
no test discs; the CC0 "Sita Sings the Blues" ISO has no menu or IG
stream). `bdmvkit` (LGPL-3.0-or-later, a Python tool from 2026-10-11)
claims to author IG menus; it is too new to trust. HDMV menus are checked
on the owner's discs on the host, with permission.

## Design options

### A. mpv's own disc menus (recommended)

- **libmpv.** RVP's build (`tools/libmpv/build.sh`) moves to a recipe
  commit with today's mpv master (#18416 and #18662 in) and builds
  libdvdread with `-Dlibdvdcss=disabled` (and libbluray without libaacs,
  which libbluray only loads at run time anyway). BD-J: libbluray's jars
  built with a JDK and Ant in the build root (`-Dbdj_jar=enabled`), shipped
  beside `libmpv-2.dll` (LGPL-2.1-or-later, with OW2 ASM's BSD-3-Clause
  code); BD-J then runs with the user's own Java.
- **RVP.** Opening a disc, a `VIDEO_TS` folder or `VIDEO_TS.IFO` plays
  `dvd://menu` (Blu-ray `bd://menu`) when "Play DVD menus" is on;
  `disc-menu-active` tells RVP a menu shows. While it does, RVP's own key
  handler sends `discnav up/down/left/right/select` for the arrows and
  Enter, and the pointer sends `discnav mouse-move/mouse-click` with
  coordinates RVP maps from its window to the video rectangle (mpv's
  child window takes no input). DVD Control's Title, Root, Chapter,
  Audio, Subtitle and Angle Menu and Previous Menu become actions
  (`title-menu`, `menu`, `menu` in the title set's chapter context,
  `audio-menu`, `sub-menu`, `angle-menu`, `prev`); Blu-ray's Pop-up Menu
  `popup`. Menu Language and Auto Skip (Preferences > Device > DVD/Blu-ray)
  map to mpv's options where they exist.
- **No new library in RVP**: libdvdnav, libdvdread and libbluray stay
  inside libmpv, which D8 waived.
- **Tests**: headless (libmpv's null outputs) on the synthetic DVD: the
  menu shows (`disc-menu-active`), buttons move and activate, title 1
  starts, chapter and audio menus choose, the endless still holds, a
  title ends in the main menu again; `rvp --selftest=m16`; `m16-all.ps1`
  on the VM with RVP's libmpv. CI's Linux libmpv (0.37) and Windows
  libmpv (shinchiro's 2026-10-10) lack parts: the tests pass with a note
  where `discnav`'s October actions are missing.
- **Differences from PotPlayer**: timed stills are skipped (studio logos
  before a menu do not wait); anything mpv's young code still gets wrong
  (the open issues above).

### B. RVP's own navigator (PLAN.md's plan)

libdvdnav and libdvdread (GPL-2.0-or-later) and libbluray
(LGPL-2.1-or-later) as DLLs of their own, called through FFI in `shell/`;
one `loadfile` of a custom stream per segment; RVP decodes the menu's
sub-pictures and draws highlights with `overlay-add`; RVP holds timed
stills; Blu-ray IG and PG planes from libbluray's overlay callbacks drawn
by RVP. Several times the work of A, with gaps between segments, and a
second copy of what libmpv already contains.

## Decisions for the owner

1. A or B. A uses mpv's code (young, improving upstream) and needs no new
   library in RVP; B follows the plan and needs libdvdnav, libdvdread and
   libbluray approved under D8.
2. libdvdcss: RVP's packages stop containing it with RVP's own libmpv
   build. CI downloads shinchiro's build to compile and test, and its
   package artifacts (private, pruned, never handed out) would still
   contain libdvdcss unless CI drops packaging libmpv or RVP's libmpv is
   cached for CI some other way.

## The owner's decisions (2026-10-29)

1. **B: RVP's own navigator.** libdvdnav and libdvdread (GPL-2.0-or-later)
   and libbluray (LGPL-2.1-or-later) are approved under D8. They are DLLs
   of their own beside `rvp.exe` (`tools/discs/build.sh`), loaded at run
   time; on Linux RVP loads the system's `libdvdnav.so.4` and
   `libbluray.so.2`.
2. **libdvdcss in CI is acceptable.** CI keeps shinchiro's libmpv (which
   contains libdvdcss) to build and test; its private, pruned artifacts are
   an accepted exception. Packages made locally (`rvp-r`, the installer)
   use RVP's own libmpv (`tools/libmpv/build.sh`, `no-libdvdcss.patch`) and
   contain neither libdvdcss nor libaacs.

## What the implementation found (goal R)

- **Segments are bound to their address.** mpv opens a file after
  `loadfile` returned: a segment opened late took the data of the one
  prepared after it (seen on the Windows VM). The address carries the
  segment's number (`rvpdisc://N`), the stream's open parses it, and an
  older segment reads nothing.
- **A title segment stops at its last sector.** mpv's probe reads up to
  the end of a seekable stream and seeks back; libdvdnav refuses to seek
  once it reached a cell's still ("Cannot seek in a still frame") and
  cannot go back after a jump. The navigator hands out a title's data up
  to its last sector and reads on (to the still, the post commands or the
  next program chain) only when mpv reached the end.
- **Cells that go back end a segment.** A motion menu loops (its cells
  start again without a channel hop) and a disc may chain one program
  chain per chapter: the timestamps start again, which one mpv file
  cannot take. The owner's DVD (a commercial copy with two warnings and a
  looping motion menu) found it; each loop is a segment.
- **Bit fields.** libdvdnav builds with `-mno-ms-bitfields`, libdvdread
  does not: MinGW's default layout gave libdvdnav garbage button
  rectangles until libdvdread was built with the same flag.
- **libbluray.** Events that end a segment (a playlist, a seek of its own,
  a still) come without data, so the boundary is exact. Its in-mux IG
  decoder (stc -1) decodes a menu as it is read; keys and the pointer
  redraw the plane at once (`bd_user_input`, `bd_mouse_select`), a
  button's commands run at the next read. `bd_get_current_title` is the
  playlist's index in navigation mode: the title comes from `BD_EVENT_TITLE`.
  libbluray 1.3.4 (Ubuntu 24.04's) crashes on a key when a clip has no EP
  map; the synthetic disc has one, as every authored disc does.
- **A synthetic Blu-ray.** No free disc with HDMV menus exists, and the
  owner's Blu-ray copy is a remux without menus (one playlist, no IG
  stream). The owner chose a synthetic disc:
  `tools/test-clips/make-m16-bluray.py` writes index.bdmv, MovieObject.bdmv,
  two playlists with marks, clip information with EP maps, and an IG
  stream (ICS, PDS, three ODS, END) muxed into FFmpeg's m2ts with its PMT
  entry; libbluray 1.3.4 and 1.5.1 both play it through its menu.
