# RVP (Rust Video Player): project plan

Status: draft, 2026-10-09. No product code exists yet.

## 1. Goal

Build RVP, a desktop media player for Linux and Windows written in Rust, that:

1. Looks like the owner's current PotPlayer setup, pixel for pixel where practical:
   the BlackSpace skin (`BlackSpace_en.dsf`) in classic mode with the playlist
   attached on the right.
2. Works with SVP 4 Manager (frame interpolation) on both systems.
3. Is comfortable for daily use. The owner's priorities, in order:
   - a reliable playlist that remembers the last played file and the position
     in every file, across restarts and crashes;
   - an audio normalizer with boost, equivalent to PotPlayer's
     "Normalizer (Auto Volume Control)" with "Boost audio";
   - full control from both mouse and keyboard, with configurable bindings and
     PotPlayer's defaults (Enter fullscreen, Space pause, Up/Down volume, ...);
   - polished usability (section 4).

Project facts: name RVP, license GPL-3.0-or-later, private GitHub repository
for now.

Non-goals for the first release:

- Shipping PotPlayer's own skins or artwork. RVP loads `.dsf` files the user
  already has and bundles only original art.
- A full copy of PotPlayer's Preferences window. RVP needs a settings window for
  its own options, but matching PotPlayer's look there is optional.
- DirectShow concepts (filter graphs, splitter and decoder priorities, OpenCodec).
- The dead Daum services that skins still reference (broadcast, chat, login).
- Screen and device capture, TV tuners (`WdmSkin.xml`), DVD menus.

## 2. Findings that drive the design

All of these were checked on the owner's machine on 2026-10-09
(PotPlayer build 260819, SVP 4 Pro).

### 2.1 PotPlayer

- Win32/MFC application on a DirectShow graph with EVR/VMR9/madVR presenters,
  its own FFmpeg-based codecs (`ffcodec64.dll`), BASS audio, AngelScript
  extensions (`Extension/api.txt`). Settings live in the registry under
  `HKCU\Software\DAUM\PotPlayerMini64`.
- Built-in VapourSynth support uses mpv's script contract: the bundled
  `VapourSynth/motioninterpolation.vpy` reads `video_in`, `container_fps` and
  `display_fps`.
- Playlists are `.dpl` text files (`DAUMPLAYLIST` header, `key=value` lines such
  as `playname=` for the last played file, `N*file*path` entries) in
  `%APPDATA%\PotPlayerMini64\Playlist`. Each playlist tab ("album") is one file.
  The owner's files point to Windows paths such as `V:\...`.
- The context menu and the Preferences dialog (F5) are plain native Win32 menus
  and dialogs. They are not drawn by the skin.

### 2.2 Owner's PotPlayer configuration

- Skin BlackSpace, classic `MainSkinArea` (lines 172-496 of `VideoSkin.xml`):
  title bar with `[index/count] file name`, opaque 71 px bottom bar, playlist
  panel attached on the right (`ConstSize='295'`) with album tabs and a footer
  showing source fps, output fps and resolution.
- Bottom bar info strip: container, decoder output format, video bitrate, audio
  codec, channels, sample rate, audio bitrate.
- Fullscreen shows video only; controls auto-hide (`AutoHideSkin=1`).
- UI language English; display scaling 100%.
- Hotkeys: the custom shortcut list (`MainShortCutList`) is empty, so PotPlayer's
  default keymap is in use. Mouse: `MouseLeftSClick=4`, `MouseLeftDClick=1`; the
  meaning of these codes is confirmed in R1.
- `RememberPosition=1`.
- Audio: Normalizer enabled, method "Tomsteady 32-bit floating point
  arithmetic", "Boost audio" on, Level 75%, Release time 10
  (`sAudioNormalizeSmooth=10`). Freeverb off.
- An SVP preset exists ("LAV Video Decoder with SVP (Smooth Video)").

### 2.3 Skin format (`.dsf`)

- A ZIP archive with XML-like layout files, sprite atlases (PNG/BMP) and text
  includes. BlackSpace contents: `VideoSkin.xml` (main), `SubCommon.txt`
  (playlist, control box, common), `AudioSkin.xml`, `WdmSkin.xml`, plus atlases
  `Main.png` (1070x1396), `Control.png`, `PlayList.png`, `Window.png`,
  `ScrollBar.bmp`.
- `Skins/SkinSupport.txt` in the PotPlayer folder lists what the skin engine
  understands: 540 actions, 378 commands/states, 27 sections, 294 attribute
  names. Treat it as the specification.
- Layout model: VCL-style docking (`Align=Top|Bottom|Left|Right|Client` with
  margins); absolute coordinates where negative values count from the right or
  bottom edge; `Width='-122'` means parent width minus 122; centering;
  visibility rules (`VisParentMinWidth`, `VisMinLeft`, `State=`, `PageIndex`);
  splitters (`IsSplitter`, `ConstSize`); slots for the video and playlist
  (`Contain='CMD_VIDEO_WINDOW'`, `Contain='CMD_PLAYLIST_TOGGLE'`).
- Rendering model: sprite rectangles from the atlas; text colors sampled from
  atlas pixels (`ColorX`/`ColorY`); bitmap-font labels (`PicLabel` with
  `MappingDB`); hue/saturation color themes (`ColorTheme`,
  `ExcludeColorThemeArea`); skin-defined menus (`UserMenu`/`MenuItem`, 19 in
  BlackSpace).
- Commands: buttons bind a left-click `CmdID` and a right-click `AltCmdID`.
  BlackSpace uses about 270 distinct IDs: about 60 belong to dead Daum
  services, about 20 to capture features, about 180 are ordinary player
  functions.
- The parser must be tolerant. BlackSpace is not valid XML: 103 lines are
  disabled by deleting their leading `<`, and some attributes have broken
  quotes (`ThumbHeight='15 ImageLeft=...`, `Top=0'`). PotPlayer accepts them.
  The format also has a text preprocessor (`#include<file #VAR=value>`,
  `#if<SkinName==X>`/`#else`/`#endif`) and multi-language strings
  (`{$CP949=...$}{$CP0=...$}`).
- BlackSpace's idle logo is Flash (`Logo.swf`). Current PotPlayer shows a black
  screen instead, so Flash is not needed.
- Tooltips are shown raw, including the `< >` separator
  (`Pause< >DVD Control`).

### 2.4 SVP 4 integration

From `SVP 4/script/generate.js` and the shipped binaries:

- Player codes: 2 = VLC, 3 = PotPlayer (AviSynth `potplayer_source()`,
  Windows only), 4 = DirectShow AviSynth/VapourSynth filter, default = mpv
  (`video_in`).
- mpv: SVP connects to mpv's JSON IPC (`input-ipc-server`). SVP's own
  `mpvSockets.lua` creates one socket per process (`<tmp>/mpvSockets/<pid>`);
  its `mpv.conf` uses `input-ipc-server=mpvpipe` on Windows. On Linux SVP finds
  the owning process with `lsof`. Requirements: mpv built with VapourSynth,
  `hwdec=auto-copy`, `hr-seek-framedrop=no`.
- VLC: SVP does not use the libVLC API. It installs two VLC 3.0 modules:
  `libvapoursynth_plugin` replaces the deinterlace filter, and
  `libsvpcontrol_plugin` talks to the manager over the pipe
  `svp-team.com.manager4`. Both are bound to the VLC 3.0 module ABI
  (`vlc_entry__3_0_0f`). On Linux SVP supports only distro-packaged VLC 3.0.
- SVP cannot work with Flatpak or Snap applications at all, according to its wiki.

### 2.5 Test environments

| Environment | Use | Notes |
|-------------|-----|-------|
| WSL2 (development machine) | Builds, unit and headless tests | WSLg can show windows for quick checks |
| Ubuntu 22.04 VM (shared with the another project project) | Linux builds, GNOME Wayland behavior: frameless window, popups, input | VMware SVGA, no real GPU. Distro Qt (6.2) and mpv (0.34) are too old: bring our own Qt and libmpv |
| Owner's Linux: the same host PC booted from another disk | Hardware decoding, SVP, performance, daily-use trials | Ubuntu 26.10, GNOME 50, Wayland only (X11 apps through XWayland), RTX 30-series card with the proprietary driver |
| Windows host | Windows builds, SVP on Windows, PotPlayer reference captures | Anything that launches PotPlayer or captures the screen needs advance notice and the owner's explicit permission each time |

Consequences of the dual boot:

- While the PC runs Ubuntu, Windows and WSL are down. Linux GPU work happens in
  separate sessions. The private GitHub repository is the bridge: each Linux
  session clones or pulls it, runs a prepared test kit (build script, checklist,
  result log), and commits the results to `docs/spikes/` or `docs/test-runs/`.
  The owner can run the kit by hand or start a Claude Code session on Ubuntu.
- The primary Linux target is GNOME on Wayland. An application cannot make
  itself "always on top" there; the user can still set it from the GNOME window
  menu.
- SVP 4 Linux is installed with SVP's own installer, which brings its own
  Python 3.12 and VapourSynth. RVP's libmpv must work with that runtime (S2).

Video library: `V:` on Windows is the SMB share `\\nas\Media`. On Ubuntu it
is mounted over CIFS at a fixed path (for example `/mnt/media`, through `fstab`,
not a GVFS mount). The same files are then reachable from both systems, which
`.dpl` import uses (`V:\` maps to the mount point). It also makes shared
playback state between the two systems possible (open question 2).

## 3. Key decisions

| # | Decision | Reason | Verified by |
|---|----------|--------|-------------|
| D1 | Playback engine: libmpv on both systems, via the `libmpv2` crate. | SVP's main path on Linux is mpv IPC. The VLC path locks us to VLC 3.0 binary modules; libVLC 3 can embed video only into an X11 window or via CPU frame copies; Rust libVLC bindings are unmaintained. | S1, S2 |
| D2 | Skin engine: our own Rust crate that loads `.dsf` directly. | Redrawing a "similar" skin can never be identical. Loading the user's own files is the only route to 1:1. | S4 |
| D3 | Shell: Qt 6 via `cxx-qt`, UI in QML. A small C++ shim is allowed for the mpv video item. | Native-looking menus and dialogs on both systems; cascading menus that extend beyond a small window (`xdg_popup` on Wayland); mature on Windows. Fallback: Qt Widgets with a thin C++ UI layer. | S3 |
| D4 | Video output: OpenGL render API of libmpv inside Qt Quick (OpenGL scene graph backend) on both systems first. Optional later: mpv in a child window (`wid`) on Windows for native D3D11 output and HDR passthrough. | One code path; Wayland needs the render API. SVP needs copy-back decoding anyway, so zero-copy is not lost while SVP is active. | S3 |
| D5 | Skin drawing: Rust builds a display list; it is painted with `QPainter` in a `QQuickPaintedItem` subclass implemented through `cxx-qt`. | Native text rendering (Qt can use the GDI font engine on Windows for a closer match); no custom GPU text stack. | S3, S4 |
| D6 | Native menus and a native settings window, not skinned. | Matches what PotPlayer actually does. | Screenshots |
| D7 | Linux packaging: AppImage and native packages. No Flatpak or Snap. | SVP cannot attach to sandboxed applications. | SVP wiki |
| D8 | License GPL-3.0-or-later. | Confirmed by the owner. libmpv's default build is GPL; Qt is used under LGPL with dynamic linking. | Owner |
| D9 | Player state (playlists, positions, settings) in our own files, written atomically and often. | The resume requirement must survive crashes, not only clean exits; mpv's watch-later files are written only on quit. | M2, M3 |

## 4. Usability requirements

Measurable targets, checked in every milestone from M2 on:

- Every action is reachable from both mouse and keyboard; every binding is
  configurable; defaults match PotPlayer.
- Nothing is lost: last played file, position in every file, playlists, volume,
  normalizer state, window size (and position where the platform allows) are
  restored after a restart and after a crash. State is written on change and
  at least every 5 seconds during playback.
- Response: input to visible reaction under 50 ms; seek to new frame under
  300 ms for local files; window shown within 500 ms of launch; first frame
  within 1 s for local files.
- No modal interruptions during playback. Errors appear as non-blocking
  messages; unplayable playlist entries are marked, not fatal.
- Tooltips on every skin control; the context menu shows the current hotkey of
  each item.

## 5. Architecture

```
+-------------------------- Qt 6 shell (QML + cxx-qt) ---------------------------+
|  MainWindow (frameless)                                                         |
|   +- SkinItem   (QQuickPaintedItem in Rust) -- paints display list, input       |
|   +- VideoItem  (mpv render API, OpenGL)    -- fills CMD_VIDEO_WINDOW rect      |
|  Native context menu, skin UserMenus, settings window, file dialogs, tray       |
+------------------------------------|--------------------------------------------+
                                     | Rust API (QObjects via cxx-qt)
+------------------------------------v--------------------------------------------+
| app        command dispatcher (PotPlayer IDs -> actions), input bindings, CLI, |
|            single instance, settings, playlists/albums, resume store           |
| skin       .dsf loader, tolerant parser, preprocessor, string tables, layout,   |
|            state/visibility, hit testing, display list, headless renderer      |
| engine     libmpv wrapper: commands, observed properties, events, audio chain, |
|            SVP mode                                                             |
| platform   MPRIS / SMTC, screensaver inhibit, file associations                 |
+---------------------------------------------------------------------------------+
```

Planned Cargo workspace:

- `crates/skin` - no Qt or mpv dependency; fully testable headless.
- `crates/engine` - libmpv wrapper and the property model the skin binds to.
- `crates/app` - commands, input bindings, settings, playlists, resume store,
  CLI, single instance.
- `crates/platform` - OS integration behind traits.
- `shell/` - QML, the cxx-qt bridge and the C++ video item shim.

Threading: libmpv runs its own threads. Its wakeup callback only queues a task
onto the Qt thread (`cxx-qt` `qt_thread().queue`); the Qt thread drains
`mpv_wait_event` and updates the property model. The render-update callback
schedules a redraw of the video item. No async runtime is needed for this.

libmpv embedding notes: libmpv does not read `mpv.conf` or auto-load scripts
unless told to, so RVP sets every option through the API, including
`input-ipc-server`. The IPC server is started by `mp_initialize()`, which
`mpv_initialize()` also calls, so it works in an embedded libmpv. The render API
offers only OpenGL and software backends (`MPV_RENDER_API_TYPE_OPENGL`,
`MPV_RENDER_API_TYPE_SW`).

Skin data binding: labels and states read from one property model fed by mpv
observations, for example `media-title`, `playlist-pos`, `playlist-count`,
`time-pos`, `duration`, `pause`, `volume`, `mute`, `file-format`,
`video-params` (size, pixel format), `video-bitrate`, `audio-codec-name`,
`audio-params` (channels, sample rate), `audio-bitrate`, `container-fps`,
`estimated-vf-fps`.

Audio normalizer: implemented as an mpv audio filter chain built from FFmpeg
filters (candidates: `dynaudnorm` for gain riding with a maximum gain for the
boost, followed by a limiter; `loudnorm` is unsuitable for live playback).
PotPlayer's "Level" and "Release time" map to filter parameters chosen in S5.
Volume above 100% uses mpv's `volume-max`.

## 6. Milestones

Effort is relative: S (days), M (one to two weeks), L (several weeks).

### M0. Spikes and reference capture (S-M, no product code)

Each spike is a throwaway program. Each answers one question and records the
answer in `docs/spikes/`. S2 runs before S1, because SVP's interpolation needs
VapourSynth inside libmpv.

- **S2. libmpv with VapourSynth on both systems.** Linux: our own build
  (`-Dvapoursynth=enabled -Dlibmpv=true`). Windows: prebuilt `libmpv-2.dll`.
  Find out where VapourSynth and Python come from (SVP's bundled runtime or a
  separate install). Pass: PotPlayer's `motioninterpolation.vpy` runs unchanged.
- **S1. Does SVP accept a libmpv process?** Minimal Rust binary (libmpv2) with
  `input-ipc-server`, `hwdec=auto-copy`, `hr-seek-framedrop=no`. Run with SVP on
  the owner's Linux machine and on Windows. Pass: SVP attaches and
  `estimated-vf-fps` reaches the target rate. If SVP rejects a process not named
  `mpv`, record what it checks and evaluate fallbacks (process naming, running
  real `mpv` embedded through `wid` on X11 and Windows).
- **S3. Qt shell.** cxx-qt + QML: frameless window that moves and resizes through
  `QWindow::startSystemMove`/`startSystemResize` on Windows, X11 and Wayland; a
  context menu that extends beyond the window bounds on all three (on Wayland
  it must be an `xdg_popup` positioned relative to the window); a
  `QQuickPaintedItem` implemented in Rust (cxx-qt's `custom_parent_class`
  example shows the pattern); the mpv video item on the OpenGL scene graph
  backend; the wakeup-callback-to-Qt-thread path; 4K60 playback with the chrome
  repainting once per second without dropped frames (on the owner's GPU
  machine). On Windows, also compare the OpenGL path with mpv in a child window
  (`wid`).
- **S4. Static skin render.** Parse BlackSpace `VideoSkin.xml` (classic area)
  and render the window frame to PNG at a fixed size. Compare with reference
  captures.
- **S5. Normalizer parity.** Find the filter chain and parameters that match
  PotPlayer's normalizer with boost (Level 75%, Release time 10). Compare
  short-term loudness (EBU R128) over the same clips: one with quiet dialogue
  and loud action, one music track. Use loopback recordings of PotPlayer's
  output on Windows if feasible; otherwise the owner does a blind A/B listening
  test.
- **R1. Reference capture on the Windows host.** Announce first and wait for
  the owner's explicit permission. Capture PotPlayer screenshots at fixed window
  sizes, the full context-menu tree, the default hotkey table, the mouse action
  table (meaning of codes such as `MouseLeftSClick=4`), and loudness recordings
  for S5.

Gate: confirm D1-D5 or switch to the documented fallback before M2. M1 does not
wait for the gate: the skin crate has no Qt or mpv dependency, so it is useful
whatever the spikes decide.

### M1. Skin engine core (M)

- ZIP loading, tolerant tokenizer, preprocessor, multi-language strings,
  `StringTable`.
- Object model for the sections BlackSpace uses; unknown elements and
  attributes are kept and logged, never fatal.
- Layout engine: docking, negative coordinates, centering, auto-size,
  visibility rules, pages, splitters.
- State machine: hover, pressed, disabled, toggled (`DownFlag`), `State=`
  conditions, inactive-window images (`AltImage*`).
- Hit testing, tooltips, left and right click actions.
- Display list and a headless renderer for tests.

Exit: geometry and golden-image tests pass for the classic BlackSpace window at
three sizes.

### M2. Player window MVP (L)

- Classic window with live video: title bar, frame, seek bar, volume and MUTE,
  info strip, current and total time, prev/play/next, window buttons.
- Move, resize, maximize, minimize; fullscreen with auto-hide.
- Open by drag and drop, file dialog and command line.
- Command dispatcher (PotPlayer IDs to actions) with the commands the classic
  window needs; M4 extends its coverage.
- Input bindings engine with PotPlayer's default keyboard and mouse maps
  (editing UI comes in M3).
- Settings and resume store (D9): last file, position per file, volume, window
  geometry; crash-safe.
- Next and previous file through mpv's internal playlist; M3 adds the panel.

Exit: single files play with resume after restart and after a killed process;
side-by-side with PotPlayer at the same size shows no visible layout difference.

### M3. Daily-use essentials (L)

- Playlist panel drawn from `PlaylistSkinArea` (colors, album tabs, scroll bar);
  albums as tabs; add, remove, reorder, undo; current, played and unplayable
  rows; footer with source fps, output fps and resolution.
- Playlists persist automatically; on start RVP opens the last album, selects
  the last played file and resumes its position.
- `.dpl` import with path mapping (Windows drive letters to Linux mounts).
- Single instance with `/add`, `/insert`, `/current`, `/new`.
- Audio normalizer with boost from S5, volume above 100%, toggles and settings.
- Input binding editor for keyboard and mouse (wheel, clicks, double click,
  middle and side buttons), with conflict detection and reset to defaults.

Exit: the owner uses RVP instead of PotPlayer for a week of normal viewing on
one system without losing a position or a playlist.

### M4. Menus and commands (M)

- Native context menu with the same tree as PotPlayer (from R1), limited to
  features that exist in RVP, showing current hotkeys.
- The 19 skin-defined menus of BlackSpace.
- Dispatcher coverage for the ~180 live BlackSpace command IDs.

### M5. Settings window (M)

- Section tree with pages for RVP's own options: playback and resume, playlist,
  audio (normalizer, output, volume), video (decoding, output, scaling,
  shaders), SVP/VapourSynth, subtitles, input bindings, skins.
- Settings stored as files in the platform config directory; portable mode;
  import and export. A PotPlayer-like look is welcome but not required.

### M6. SVP and video processing (M)

- SVP mode: IPC socket or pipe naming compatible with SVP, copy-back decoding,
  diagnostics when SVP is not attached.
- VapourSynth script slots compatible with PotPlayer's `.vpy` files; GLSL
  shaders; deinterlace; color adjustments.
- Control panel (F7) from `ControlBoxSkinArea`: equalizer, color, subtitle sync.

### M7. Platform integration and packaging (M)

- Linux: MPRIS, screensaver inhibit, `.desktop` file and MIME associations,
  AppImage, `.deb`, AUR package.
- Windows: media transport controls, file associations, installer and portable
  ZIP, crash logs.
- Continuous integration builds for both systems in the private repository.

### M8. Later

Audio-only skin mode (`AudioSkin.xml`), overlay controls mode
(`MainSkinAreaOscInside`), playback info window (Ctrl+F1), subtitle search and
translation, streaming via yt-dlp, an original default skin, other `.dsf` skins.

## 7. Testing strategy

- Unit tests for the tokenizer and preprocessor, using the BlackSpace quirks
  as fixtures (disabled lines, broken quotes, macros).
- Geometry tests: the layout of every element at fixed window sizes, stored as
  reviewed snapshots.
- Golden-image tests: headless render compared with PotPlayer captures from R1.
  Pixel tolerance for sprites; masked or looser comparison for text, because
  font rasterizers differ between systems.
- Engine tests with libmpv in headless mode (`vo=null`, `ao=null`) for the
  property model, commands and the audio filter chain.
- Resume tests: kill the process at random points during playback and check
  that the restored position is within 5 seconds of the last one.
- Input tests: every default binding triggers its action; rebinding and
  conflict detection.
- SVP acceptance on both systems: open a 24 fps file and check that output fps
  reaches the target and that seeking keeps audio in sync.

## 8. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| SVP rejects a process that is not `mpv` | Breaks a core requirement | S1 first; fallbacks listed there |
| libmpv builds without VapourSynth, or Python version mismatch | No SVP | S2; ship our own libmpv |
| Normalizer sounds different from PotPlayer's | Daily-use complaint | S5 measurement and A/B test; tunable parameters |
| Parser quirks that only PotPlayer's behavior defines | Visual differences | Golden images from R1; fix by observation |
| cxx-qt gaps (painted items, menus, popups) | Shell rework | S3; fallback to Qt Widgets |
| Render API performance with chrome repaints | Dropped frames | S3 measurements; repaint only changed regions |
| Desktop OpenGL quality on Windows (no ANGLE in Qt 6) | Glitches on some drivers | S3 compares with the `wid` child-window path |
| No GPU in the shared Ubuntu VM | Late discovery of GPU issues | GPU checks on the owner's Ubuntu boot at the end of M0, M2, M3 and M6 |
| Dual boot: Linux GPU sessions cannot run alongside the Windows/WSL tooling | Slow feedback loop | Self-contained test kit; results committed to the repository; batch several checks per reboot |
| NVIDIA on Wayland with the OpenGL render API | Stutter or tearing on the main Linux target | Covered by S3 on the owner's Ubuntu boot; compare with Vulkan-based `mpv` as a baseline |
| Wayland limits: always on top, window position | Missing PotPlayer behaviors | Document; works on X11 and Windows |
| Font differences on Linux (Tahoma, Arial) | Text not pixel-identical | Metric-compatible fonts; looser text comparison |
| HiDPI scaling of 96-dpi bitmap skins | Blurry or misaligned art | Integer scaling where possible; test at 100%, 150%, 200% |
| Third-party skin copyright | Legal | Never redistribute; load from the user's install |
| Codec patents when shipping FFmpeg in our builds | Legal, mainly for public binaries | Same practice as mpv and VLC builds; decide before any public release |

## 9. Open questions

1. The working directory is still `VLCpot`. Goal A moves the plan into a new
   repository at `~/rvp`.
2. Shared playback state across the dual boot: should Windows and Linux share
   playlists and resume positions, so a film paused on one system continues on
   the other? Design sketch: the state lives on the shared library (or is
   synced to it); entries are keyed by a path relative to a library root
   (`V:\` and `/mnt/media` both map to `Media:/`), with a fallback key from file
   size and a hash of the first and last megabyte for files outside known roots.
   If wanted, it belongs in M3.

## 10. Goals for `/goal`

The milestones are grouped into nine goals, A to I. Each goal is one `/goal`
run, writes `docs/goals/goal-<letter>-status.md` and ends with a commit pushed to
the private `origin`. Goals run in order; C and D can swap.

| Stage | Goal | Covers | Where it runs | Needs the owner |
|-------|------|--------|---------------|-----------------|
| 1 | A | Repository bootstrap, S4, M1 | WSL | No |
| 2 | B | S2 and S3 without the host desktop, Windows build path, Linux test kit | WSL, Windows VM, Ubuntu VM | No |
| 3 | C | S1, S3 performance, R1, S5 on Windows | Windows host desktop | Permission before every launch or capture |
| 3 | D | S2, S1, S3 on Linux with the GPU | A session started on the Ubuntu 26.10 boot | Privileged steps (driver, SVP install, NAS mount) |
| 4 | E | M2 player window | WSL and VMs | Final check on host and Ubuntu boot |
| 4 | F | M3 daily-use essentials | WSL and VMs | Week-long trial afterwards |
| 5 | G | M4 menus and commands, M5 settings window | WSL and VMs | No |
| 5 | H | M6 SVP and video processing | WSL, VMs, host | Permission for host checks |
| 5 | I | M7 platform integration and packaging | WSL and VMs | No |

Rules shared by all goals, also written into the project `AGENTS.md` by goal A:

- Repository content is in English.
- Never commit PotPlayer or SVP files, renders or captures made from them, or
  third-party binaries. Read them from their installed locations at run time.
  Local-only material lives in `~/rvp-reference`.
- Never change PotPlayer, SVP or system settings on the host. Read-only access
  is fine.
- Anything that starts a program or captures the screen on the Windows host
  desktop needs advance notice and the owner's explicit permission, every time.
- The repository stays private; no tags, releases or published packages.

Goal A:
```
/goal Execute goal A of docs/PLAN.md (repository bootstrap, spike S4 and milestone M1). Create the repository ~/rvp (git init, branch main), move ~/VLCpot/docs into it, create the private GitHub repository woffko/rvp and push main. Bootstrap: Cargo workspace with crates skin, engine, app and platform (stubs except skin), rust-toolchain.toml, GPL-3.0-or-later license, README, .gitignore, a project AGENTS.md in English with the shared rules of PLAN.md section 10 and the Longrun section from ~/AGENTS.md, `cargo xtask ci` (fmt, clippy -D warnings, tests, cargo deny) and GitHub Actions for Linux and Windows. Then implement crates/skin as described in PLAN.md section 2.3 and M1: .dsf loading, a tolerant tokenizer that accepts BlackSpace's disabled lines and broken quotes the way PotPlayer does, the preprocessor, multi-language strings and StringTable, object model, layout engine, states, hit testing, display list and a pure-Rust headless PNG renderer. Tests: unit tests on small synthetic fixtures that reproduce each quirk; geometry snapshot tests and PNG renders of BlackSpace's classic MainSkinArea with the attached playlist at 1280x720, 1920x1080 and 2560x1080, reading the skin from RVP_TEST_SKIN (default "/mnt/c/Program Files/DAUM/PotPlayer/Skins/BlackSpace_en.dsf") and skipping when it is absent; write the renders to ~/rvp-reference/renders, compare them with the owner's screenshots in ~/rvp-reference/screenshots and list every visible difference. Done when: xtask ci is green locally and in CI, the renders exist, docs/goals/goal-a-status.md lists results, differences and open questions, and everything is committed and pushed. Do not: commit any PotPlayer or SVP file or anything rendered from them, start on Qt or libmpv, run anything on the Windows host desktop, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal B:
```
/goal Execute goal B of docs/PLAN.md (spikes S2 and S3 without the host desktop, the Windows build path and the Linux test kit) in ~/rvp. Decide how to build a cxx-qt + Qt 6 + libmpv application for Windows (cross-build with cargo-xwin from WSL, or a native MSVC build on the Windows test VM <windows-vm>) and record the decision with reasons. S2 on the Windows VM: libmpv-2.dll with VapourSynth runs PotPlayer's motioninterpolation.vpy unchanged in headless mode, with a VapourSynth and Python runtime compatible with SVP's (inspect "/mnt/c/Program Files (x86)/SVP 4/mpv64" read-only). S3 as throwaway programs under spikes/: a cxx-qt QML frameless window that moves and resizes through startSystemMove/startSystemResize, a context menu that extends beyond a 440x77 window, a QQuickPaintedItem implemented in Rust that paints a display list from crates/skin, and an mpv OpenGL video item fed through the wakeup-callback-to-Qt-thread path; verify on the Windows VM and on the Ubuntu VM (GNOME Wayland; access as in ~/another project/AGENTS.md; bring Qt 6.8 or newer and our own libmpv because the distro versions are too old). Prepare tools/linux-kit: a script and checklist that build the spikes on the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 30-series card) and record S1, S2 and S3 results into docs/test-runs/, with privileged steps left to the owner. Done when: each spike result is in docs/spikes/ with exact versions and commands, the kit runs end to end on the Ubuntu VM with GPU checks marked as skipped, docs/goals/goal-b-status.md lists results and open questions, and everything is committed and pushed. Do not: run anything on the Windows host desktop, change the SVP or PotPlayer installations, commit third-party binaries, make the repository public. Stop on blockers or when an owner decision is needed.
```

Goal C:
```
/goal Execute goal C of docs/PLAN.md (Windows host session: S1, S3 performance, R1 and S5) in ~/rvp. This goal uses the owner's Windows desktop: before every step that starts a program or captures the screen, say what will run, which windows will appear and for how long, then wait for the owner's explicit permission. Ask the owner which video and music files to use as test clips. S1: run the spike player with input-ipc-server, hwdec=auto-copy and hr-seek-framedrop=no while SVP 4 Manager runs, and check that SVP attaches and estimated-vf-fps reaches the target; if SVP refuses, find out read-only what it checks and record the fallbacks. S3 performance: 4K60 playback in the Qt spike on the RTX 30-series card with the chrome repainting once per second; compare the OpenGL path with mpv in a child window. R1: at 100% scaling capture PotPlayer with BlackSpace in classic mode with the playlist attached at 1280x720, 1920x1080 and 2560x1080, idle, playing, with hover and with a pressed button; dump the full context-menu tree, the default hotkey table and the mouse-action table. S5: loopback-record PotPlayer's output with the normalizer (Tomsteady, Boost on, Level 75%, Release time 10) and without it on the test clips, then find a dynaudnorm-based chain whose EBU R128 short-term loudness matches. Keep captures and recordings in ~/rvp-reference; commit only measurements, tables and findings to docs/. Done when: docs/goals/goal-c-status.md has the results of S1, S3, R1 and S5 and the gate verdict for Windows, committed and pushed. Do not: change PotPlayer or SVP settings, open the owner's files other than the chosen clips, start anything without permission, make the repository public. Stop on blockers or when an owner decision is needed.
```

Goal D (start this from a session on the Ubuntu 26.10 boot, not from WSL):
```
/goal Execute goal D of docs/PLAN.md on the owner's Ubuntu 26.10 boot. Clone or pull the private repository woffko/rvp and follow tools/linux-kit. Privileged steps (NVIDIA driver, SVP 4 Linux through its own installer, mounting \\nas\Media at /mnt/media) are done by the owner when the kit asks; never handle NAS credentials. Build the spikes and run on GNOME Wayland with the RTX 30-series card: S2 (VapourSynth inside libmpv with SVP's runtime), S1 (SVP attaches to the spike player and estimated-vf-fps reaches the target), S3 (4K60 playback with chrome repaint, a context menu beyond the window, frameless move and resize). Record exact versions and results in docs/test-runs/ and the Linux gate verdict in docs/goals/goal-d-status.md, then commit and push. Do not: change anything outside the kit's documented steps without asking, store credentials in the repository, make the repository public. Stop on blockers or when an owner decision is needed.
```

Goal E:
```
/goal Execute goal E of docs/PLAN.md (milestone M2, player window MVP) in ~/rvp, only if goal-c-status.md and goal-d-status.md record a passed gate; otherwise stop and report. Build the real application: shell/ with cxx-qt and QML, crates/engine (libmpv wrapper, property model, threading as in section 5), crates/app (command dispatcher, input bindings with PotPlayer's defaults from R1, settings and a crash-safe resume store per D9). Deliver all of M2: the classic BlackSpace window with live video and every bottom-bar element bound to mpv properties, move, resize, maximize, minimize, fullscreen with auto-hide, drag and drop, file dialog, command line, next and previous file through mpv's playlist. Tests: golden-image comparison of headless renders against the R1 captures, headless libmpv engine tests, resume tests that kill the process at random points, input binding tests; functional checks on the Ubuntu VM and the Windows VM; add an M2 checklist to tools/linux-kit for the Ubuntu boot. Done when: tests and CI are green, the usability targets of section 4 are measured and reported, docs/goals/goal-e-status.md is written, and everything is committed and pushed. Do not: run anything on the Windows host desktop without advance notice and explicit permission, commit PotPlayer or SVP files or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal F:
```
/goal Execute goal F of docs/PLAN.md (milestone M3, daily-use essentials) in ~/rvp after goal E. Deliver: the playlist panel from PlaylistSkinArea with albums as tabs, add, remove, reorder and undo, current, played and unplayable rows and the fps footer; automatic persistence and start-up restore of the last album, file and position; .dpl import with V:\ mapped to /mnt/media; single instance with /add, /insert, /current and /new; the normalizer with boost using the chain and parameters from S5, and volume above 100%; an input binding editor for keyboard and mouse (wheel, clicks, double click, middle and side buttons) with conflict detection and reset to defaults. If PLAN.md records the owner's yes to open question 2, also implement shared state across the dual boot as sketched there. Done when: tests and CI are green, resume survives the kill tests, the owner's .dpl files from %APPDATA%\PotPlayerMini64\Playlist import correctly (read-only), docs/goals/goal-f-status.md lists what the owner should try during the one-week trial, and everything is committed and pushed. Do not: modify the owner's PotPlayer playlists, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal G:
```
/goal Execute goal G of docs/PLAN.md (milestones M4 and M5) in ~/rvp after goal F. M4: the native context menu with PotPlayer's tree from R1, limited to features RVP has and showing current hotkeys; the 19 skin-defined menus of BlackSpace; dispatcher coverage for the live BlackSpace command IDs with a coverage report. M5: the settings window with the sections listed in M5, file-based settings in the platform config directory, portable mode, import and export. Done when: tests and CI are green, the coverage report names every unimplemented command ID with a reason, docs/goals/goal-g-status.md is written, and everything is committed and pushed. Do not: run anything on the Windows host desktop without advance notice and explicit permission, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal H:
```
/goal Execute goal H of docs/PLAN.md (milestone M6, SVP and video processing) in ~/rvp after goal G. Deliver: SVP mode (IPC socket or pipe naming that SVP finds on both systems, copy-back decoding, a diagnostic when SVP is not attached), VapourSynth script slots compatible with PotPlayer's .vpy files, GLSL shaders, deinterlacing, color adjustments, and the control panel (F7) from ControlBoxSkinArea. Run the SVP acceptance of section 7 on the Windows host with advance notice and the owner's explicit permission, and add an M6 checklist to tools/linux-kit for the Ubuntu boot. Done when: tests and CI are green, the Windows SVP acceptance passes, docs/goals/goal-h-status.md is written, and everything is committed and pushed. Do not: change SVP or PotPlayer settings, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal I:
```
/goal Execute goal I of docs/PLAN.md (milestone M7, platform integration and packaging) in ~/rvp after goal H. Linux: MPRIS, screensaver inhibit, .desktop file and MIME associations, AppImage, .deb and an AUR PKGBUILD. Windows: media transport controls, file associations, an installer and a portable ZIP, crash logs. CI builds packages for both systems as artifacts of the private repository. Done when: the packages install, run and uninstall cleanly on the Ubuntu VM and the Windows VM, docs/goals/goal-i-status.md is written, and everything is committed and pushed. Do not: publish packages anywhere, install on the host without advance notice and explicit permission, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```
