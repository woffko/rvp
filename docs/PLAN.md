# RVP (Rust Video Player): project plan

Status: in progress. Goals A-J are done (M1 skin engine, the gates, M2
player window, M3 daily-use essentials, M4 menus and commands, M5 settings
window, M6 SVP and video processing, M7 RVP's own logo and default skin, M8
packages and desktop integration), and so are goals L, M, N, T, O, P, Q,
R and K (M10 audio, M11 playback, playlist and opening, M12 video, picture
and window, M18 motion smoothing without SVP, M13 subtitles, M14
preferences, skins and the rest, M15 remote control, M16 DVD and Blu-ray
menus, M9 other players' skins on Windows); the Ubuntu boot run of M3-M5
passed on 2026-10-13; open: M3's one-week trial and one Ubuntu boot run for
M6, M7 and M8, which M6's and M8's acceptance need. Results are in
`docs/goals/`. Next (owner, 2026-10-16, D14): PotPlayer parity wherever
libmpv, FFmpeg and Qt allow (M10-M14), a remote control with a Unified
Remote remote (M15) and DVD and Blu-ray menus (M16), Windows first (goals
L-R), then other players' skins (goal K, done 2026-10-31) and Linux (M17,
goal S), which is next.

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
4. Has its own identity. RVP shows its own logo, never Daum's or
   PotPlayer's. It ships its own default skin in BlackSpace's spirit, so it
   works without PotPlayer installed (M7). Skins of other players (VLC, and
   mpv's on-screen controllers) are considered after that (M9). Owner request,
   2026-10-13.
5. Does what PotPlayer does, wherever libmpv, FFmpeg, Qt and the platform
   allow: every PotPlayer command, hotkey and setting either works in RVP or
   is listed with the reason it cannot (M10-M14, and M16 for disc menus).
   RVP can be controlled from a phone through Unified Remote (M15). Windows
   comes first, Linux follows (M17). Owner request, 2026-10-16 (D14).

Project facts: name RVP, license GPL-3.0-or-later, private GitHub repository
for now.

Non-goals (revised on 2026-10-16, when the owner asked for everything of
PotPlayer's that RVP can do):

- Shipping PotPlayer's own skins or artwork. RVP loads `.dsf` files the user
  already has and bundles only original art: its logo and its default skin
  (M7).
- Copying the look of PotPlayer's Preferences window. Its options are in
  scope (R2, M10-M14); RVP's settings window keeps its own look.
- A DirectShow playback engine. DirectShow is Windows' older media
  framework, a whole engine of splitters, decoders and renderers. A second
  engine beside libmpv would need a twin of everything RVP builds on mpv:
  SVP's connection, TomSteady, the filter chains, the thumbnails, the
  remote's status. Each DirectShow function has a counterpart instead:
  - decoders and splitters: FFmpeg (LAV Filters are FFmpeg too), with a
    decoders page in place of PotPlayer's filter priorities (M12);
  - renderers (EVR, madVR, MPC-VR): libplacebo's renderer, and the
    Direct3D 11 child window with HDR output (M12);
  - transform filters: FFmpeg's filters (M10, M12);
  - capture devices: FFmpeg's `dshow` input, which is DirectShow itself
    (M11).

  The owner may name a DirectShow filter that has no counterpart; then a
  DirectShow engine is reconsidered, with MPC-HC's and MPC-BE's GPL code
  as the reference.
- The dead Daum services that skins still reference (broadcast, chat, login),
  and PotPlayer's broadcasting tools (capture areas, broadcast effects).
- TV tuners (`WdmSkin.xml`, WDM/BDA channels): FFmpeg cannot tune them.
  Capture cards still open as devices (M11).

On 2026-10-16 the owner asked why some former non-goals cannot be done.
These moved into scope:

- AviSynth (M12);
- PotPlayer's HLSL pixel shaders, translated to GLSL (M12);
- PotPlayer's AngelScript extensions, with the deferred online subtitles
  (M13);
- DVD and Blu-ray menus (M16);
- playing to a second audio device at the same time (M10).

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
is mounted over CIFS at a fixed path (on the owner's Ubuntu boot today
`/mnt/nas/Media`, not a GVFS mount). The same files are then reachable
from both systems, which `.dpl` import uses through a configurable mapping of
Windows drive letters to Linux paths (owner decision 2026-10-10; default
`V:\` → `/mnt/media`, set to the real mount point in RVP's settings). It also makes shared
playback state between the two systems possible (open question 2).

## 3. Key decisions

| # | Decision | Reason | Verified by |
|---|----------|--------|-------------|
| D1 | Playback engine: libmpv on both systems, via the `libmpv2` crate (vendored with one event-handling fix, owner decision 2026-10-10; `vendor/libmpv2/RVP-PATCH.md`). | SVP's main path on Linux is mpv IPC. The VLC path locks us to VLC 3.0 binary modules; libVLC 3 can embed video only into an X11 window or via CPU frame copies; Rust libVLC bindings are unmaintained. | S1, S2 |
| D2 | Skin engine: our own Rust crate that loads `.dsf` directly. | Redrawing a "similar" skin can never be identical. Loading the user's own files is the only route to 1:1. | S4 |
| D3 | Shell: Qt 6 via `cxx-qt`, UI in QML. A small C++ shim is allowed for the mpv video item. `unsafe` is allowed in `shell/` only, for the cxx-qt bridges and FFI points (owner decision 2026-10-10). | Native-looking menus and dialogs on both systems; cascading menus that extend beyond a small window (`xdg_popup` on Wayland); mature on Windows. Fallback: Qt Widgets with a thin C++ UI layer. | S3 |
| D4 | Video output: OpenGL render API of libmpv inside Qt Quick (OpenGL scene graph backend) on both systems first. Optional later: mpv in a child window (`wid`) on Windows for native D3D11 output and HDR passthrough. | One code path; Wayland needs the render API. SVP needs copy-back decoding anyway, so zero-copy is not lost while SVP is active. | S3 |
| D5 | Skin drawing: Rust builds a display list; it is painted with `QPainter` in a `QQuickPaintedItem` subclass implemented through `cxx-qt`. | Native text rendering (Qt can use the GDI font engine on Windows for a closer match); no custom GPU text stack. | S3, S4 |
| D6 | Native menus and a native settings window, not skinned. | Matches what PotPlayer actually does. | Screenshots |
| D7 | Linux packaging: AppImage and native packages. No Flatpak or Snap. | SVP cannot attach to sandboxed applications. | SVP wiki |
| D8 | License GPL-3.0-or-later. The `rust` skill's no-GPL/LGPL rule is waived for libmpv and Qt (owner decision 2026-10-10), including the LGPL-2.1 binding crates `libmpv2` and `libmpv2-sys`; other Rust crates stay under `cargo deny`'s permissive list. | Confirmed by the owner. libmpv's default build is GPL; Qt is used under LGPL with dynamic linking. | Owner |
| D9 | Player state (playlists, positions, settings) in our own files, written atomically and often. | The resume requirement must survive crashes, not only clean exits; mpv's watch-later files are written only on quit. | M2, M3 |
| D10 | RVP shows its own logo everywhere: its window and application icons, its idle screen, its About box, and in place of the Daum logo in a loaded skin's title bar. RVP draws its logo over that sprite and leaves the user's `.dsf` unchanged. | RVP is not a Daum or Kakao product, and their marks in its window would suggest that it is. Owner request, 2026-10-13. | Owner |
| D11 | RVP's logo is a play triangle cut into three facets (amber, orange, copper), drawn as SVG in `crates/look/art/logo`; its colours are the accent of RVP Default. | The owner chose it from three drafts (goal I; draft A "Facet", after first picking C), 2026-10-15. | Owner |
| D12 | With no skin set, RVP shows RVP Default, which is built into the program; PotPlayer's installed BlackSpace is no longer the fallback on Windows. A skin that fails to load falls back to RVP Default with a message. | M7: RVP must work without PotPlayer installed. | Goal I |
| D13 | The normalizer is TomSteady, the method PotPlayer names: the gain control of Tom St Denis's freeware Winamp plugin (version 1.4), reconstructed from the plugin and run over blocks of 2048 samples, as PotPlayer runs it, by a thread of RVP's that reads each block's peak from mpv's log and sets the gain of a `volume` filter. It replaces S5's `dynaudnorm`. | It follows S5's recordings of PotPlayer within 0.1 dB (Level 75 %, Release time 10, `docs/spikes/s5-normalizer.md`); `dynaudnorm` differed by 2.2 LU and the owner heard it (quieter, voices raised less). The author gave the plugin and its source away freely; the owner allowed the reconstruction. | Owner, 2026-10-15 |
| D14 | PotPlayer parity, Windows first. RVP takes over every PotPlayer command, hotkey and setting that libmpv, FFmpeg, Qt and the platform allow (M10-M14) and lists the rest with the reason it cannot. Each part is built and accepted on Windows first; Linux keeps building and passing its headless tests, and its own parts and checks follow in M17. | The audio audit found 495 of PotPlayer's 672 menu commands missing; the owner: "we will do everything possible", Windows first, then Linux. | Owner, 2026-10-16 |
| D15 | Remote control through a local HTTP API of RVP's (off by default, 127.0.0.1, a token on every request). The Unified Remote remote is a Lua remote that calls it with Unified Remote's `http` library. | One API serves Unified Remote on both systems and other clients. A remote that sends key presses needs RVP's window in front, and window messages exist only on Windows. | M15 (goal Q) |
| D16 | RVP has its own motion smoothing for users without SVP (M18, goal T): mpv's frame blending and the motion-compensated engines that M18's study picks, never at the same time as SVP. | The owner asked for smooth motion like SVP's without SVP, "maybe not complete, but something substantial". | Owner, 2026-10-18 |
| D17 | A GPU power budget: RVP keeps the graphics card below a share of its rated power (Video settings, 50 to 100 %, 90 % by default, 100 % off) by lightening its own work step by step: EWA scalers become spline36, then GPU Super Resolution and RTX Video HDR, the pixel shaders and the VapourSynth script go off (M18's smoothing joins these steps). It never changes the card's own settings. NVIDIA's cards are read through NVML (Windows and Linux), AMD's through Windows' adapter counter (D3DKMT's adapter performance data, a share of the card's power: the licence of AMD's ADLX SDK forbids its use in software under a free licence) and amdgpu's hwmon files (Linux), only while a file plays and, on Linux, never while the system has put the card to sleep. GPU test runs on the host go at a real power limit, 90 % of the card's default, which `tools/windows-vm/gpu-limit.ps1` sets through one UAC prompt and puts back when the run ends. | On 2026-10-18 RIFE at 2160p in RVP hung the host's RTX 30-series card beyond recovery (a reboot was needed). The owner asked for a power slider in the player and the tests, 90 % by default, for NVIDIA and AMD, and chose RVP's own budget for the player and the real limit for the host's test runs. | Owner, 2026-10-19 |
| D18 | M18's engines and defaults: Blend (mpv's interpolation), MVTools (CPU) and RIFE (GPU, up to 1080p) in VapourSynth through a runtime RVP ships (never SVP's), and fruc_vulkan (NVIDIA's optical flow, RTX 30 and later) through a libmpv RVP builds with FFmpeg's master. Smoothing is off by default; turned on it is MVTools medium up to 1080p and Blend above, at the display's rate. | M18's study (`docs/spikes/m18-smoothing.md`): MVTools keeps up at 1080p on the CPU and is the most exact; RIFE is the sharpest on films; fruc_vulkan is the fastest on RTX cards but missing from the libmpv RVP ships. The owner chose all three and accepted building libmpv. | Owner, 2026-10-19 |
| D19 | Windows and Linux everywhere: from goal S on, every change (a goal's parts and the fixes between goals) is built and checked on both systems before it counts as done: Windows on the test VM (the host only with permission), Linux in WSL under Xvfb and on the Ubuntu VM; what only real hardware or a desktop session can check goes to the host's sessions and the owner's Ubuntu boot. It ends D14's "Windows first". | The owner, after goal K: "both Windows and Linux are needed everywhere". | Owner, 2026-11-01 |

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
|            remote control API for Unified Remote and other clients (M15)       |
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

Audio normalizer: TomSteady, PotPlayer's method (D13). mpv's filter chain
cuts the audio into blocks of 2048 samples and prints each block's peak to
mpv's log; a thread of RVP's reads it, runs TomSteady's gain control and sets
the gain of a `volume` filter in the chain (`af-command`), followed by a
limiter at TomSteady's ceiling. PotPlayer's "Level" is TomSteady's target
level and "Release time" its stepping. (Before D13: `dynaudnorm` with S5's
parameters.) Volume above 100% uses mpv's `volume-max`.

## 6. Milestones

Effort is relative: S (days), M (one to two weeks), L (several weeks).

### M0. Spikes and reference capture (S-M, no product code)

Each spike is a throwaway program. Each answers one question and records the
answer in `docs/spikes/`. S2 runs before S1, because SVP's interpolation needs
VapourSynth inside libmpv.

- **S2. libmpv with VapourSynth on both systems.** Linux: our own build
  (`-Dvapoursynth=enabled -Dlibmpv=true`). Windows: prebuilt `libmpv-2.dll`.
  Find out where VapourSynth and Python come from (SVP's bundled runtime or a
  separate install). Pass: PotPlayer's `motioninterpolation.vpy` runs unchanged
  through a run-time wrapper that restores `vs.get_core()` (owner decision
  2026-10-10, after goal B found that VapourSynth R73 removed it;
  `docs/spikes/s2-vapoursynth.md`).
  Goal D result (Linux): PASS with SVP's own runtime (VapourSynth R73,
  Python 3.12) and mvtools v24 built by the kit: 23.976 → 60 fps in real time.
- **S1. Does SVP accept a libmpv process?** Minimal Rust binary (libmpv2) with
  `input-ipc-server`, `hwdec=auto-copy`, `hr-seek-framedrop=no`. Run with SVP on
  the owner's Linux machine and on Windows. Pass: SVP attaches and
  `estimated-vf-fps` reaches the target rate. If SVP rejects a process not named
  `mpv`, record what it checks and evaluate fallbacks (process naming, running
  real `mpv` embedded through `wid` on X11 and Windows).
  Goal C result (Windows): PASS, SVP attached in 1.5 s and reached 59.94 fps
  (`docs/spikes/s1-svp-attach.md`).
  Goal D result (Linux): PASS, SVP attached in 1.2 s through `/tmp/mpvsocket`
  and reached 59.94 fps; the spike crashed on exit with SVP's filter active.
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
  Goal C result (Windows): PASS; the OpenGL path needs chrome updates applied
  together with the next video frame, the `wid` path does not
  (`docs/spikes/s3-qt-shell.md`).
  Goal D result (Linux, GNOME Wayland): window, menu, real-input move and
  resize, skin and video pass; libmpv must use the PipeWire audio output
  (93 dropped frames per 20 s through the PulseAudio API, 6 with PipeWire).
  The remaining 6 (Ubuntu's mpv on the same desktop: 10) are an open item
  accepted by the owner.
- **S4. Static skin render.** Parse BlackSpace `VideoSkin.xml` (classic area)
  and render the window frame to PNG at a fixed size. Compare with reference
  captures.
- **S5. Normalizer parity.** Find the filter chain and parameters that match
  PotPlayer's normalizer with boost (Level 75%, Release time 10). Compare
  short-term loudness (EBU R128) over the same clips: one with quiet dialogue
  and loud action, one music track. Use loopback recordings of PotPlayer's
  output on Windows if feasible; otherwise the owner does a blind A/B listening
  test.
  Goal C result: `dynaudnorm=f=5000:g=3:p=1.0:m=5` halves the difference
  (2.2 LU RMS against 5.3 LU without a normalizer); the owner's blind A/B test
  in M3 decides (`docs/spikes/s5-normalizer.md`).
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
- The window keeps the video's aspect ratio while it is resized during
  playback (R1 finding 1; not in M2).

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

### M7. Own look: logo and default skin (M)

- **RVP's logo.** Drawn for RVP from a vector source in the repository and
  chosen by the owner from a few drafts. It appears everywhere D10 lists:
  - the window and application icons (`.ico`, PNG sizes, and SVG for
    Linux);
  - the idle screen, which is black today because BlackSpace's Flash logo
    is not shown;
  - the About box;
  - over the Daum logo in a loaded skin's title bar.
- **An original default skin, "RVP Default", in BlackSpace's spirit.** It
  keeps the same classic layout and functions: the title bar, video, seek
  bar, buttons and info strip, the playlist docked on the right, the
  control panel from M6, and the same commands and menus, so habits carry
  over. Its design is its own: shapes, icons, palette and fonts.
- **The `.dsf` format.** The skin is written in PotPlayer's `.dsf` format,
  so the same engine draws it and users can edit it. Its sources are in the
  repository (XML and vector art), and `cargo xtask` builds the archive and
  its atlas. RVP ships it and uses it when no skin is set; today the window
  is then all video. Optional: atlases at 2x for HiDPI screens, as an RVP
  extension of the format.
- **Originality rules.** The layout and the placement of functions are
  shared on purpose; the look is not.
  - Nothing is taken from PotPlayer's skins: no files, sprites, crops,
    tracings or sampled colours.
  - No Daum, Kakao or PotPlayer names or marks appear.
  - Every asset is drawn for RVP or comes from an icon set or font under a
    GPL-compatible license. A sources file lists each asset with its
    origin and license.
  - The owner gets a side-by-side comparison with BlackSpace.
  - This is not legal advice. Before any public release, the owner decides
    whether a lawyer reviews the skin (see Risks).
- **Tests.**
  - The skin loads without warnings.
  - Geometry snapshots and golden renders at three sizes. These may be
    committed, because the art is RVP's.
  - Every control maps to an RVP action, in a coverage report like
    BlackSpace's.

Exit: the owner approves the logo and the skin, and RVP starts with the
default skin on both systems without PotPlayer installed.

### M8. Platform integration and packaging (M)

- Linux: MPRIS, screensaver inhibit, `.desktop` file and MIME associations,
  AppImage, `.deb`, AUR package.
- Windows: media transport controls, file associations, installer and portable
  ZIP, crash logs.
- The packages carry the default skin and RVP's icons from M7.
- Continuous integration builds for both systems in the private repository.

### M9. Other players' skins (M-L)

- **Study first** (a spike like M0's). Take a sample of popular VLC skins,
  mpv on-screen controllers and classic Winamp skins (the owner asked
  about them on 2026-10-15), and record:
  - what each format needs;
  - what RVP lacks for it;
  - what each part would cost.

  The owner then picks the scope.
- **VLC skins.** These are `.vlt` files in VLC's skins2 format: an archive
  with `theme.xml` and images that holds windows, layouts, buttons,
  sliders, text and the playlist, plus actions such as `vlc.play()`.
  Support would add:
  - a second loader in `crates/skin` that builds the same display list as
    `.dsf`;
  - VLC's actions mapped to RVP's;
  - a coverage report.

  The skins are loaded from the user's files on both systems.
- **mpv's on-screen controllers.** mpv has no skin files. Its look comes
  from controller scripts that mpv draws over the video: the built-in
  `osc.lua`, or third-party ones such as uosc and ModernX. Support needs:
  - libmpv with Lua on both systems (the kit builds it with
    `-Dlua=disabled` today);
  - an RVP mode where mpv's controller replaces the skin, with a borderless
    window and the pointer and keys passed to mpv;
  - the user's scripts folder. Third-party scripts are loaded from there,
    not shipped.
- **Classic Winamp skins.** These are `.wsz` files: ZIP archives of bitmap
  sprite sheets (`main.bmp`, `cbuttons.bmp`, `posbar.bmp`, `eqmain.bmp`,
  `pledit.bmp` and others) whose controls sit at fixed places in a
  275x116 main window, with an equalizer and a playlist window. Support
  would add a loader that turns the fixed layout into the same display
  list; the study records where the video goes in a window made for audio.

Exit: the parts the owner chose work on both systems: on Windows in goal K,
on Linux through M17 (D14).

### PotPlayer parity (M10 to M14)

On 2026-10-16 the owner asked for everything of PotPlayer's that RVP can
do, Windows first (D14). The lists come from these sources:

- R1's menus and hotkeys (`docs/r1/`);
- R2's Preferences tree (below);
- the coverage reports (`docs/coverage/`), which name each PotPlayer item
  RVP lacks, with its reason;
- the audio audit (`docs/audits/audio.md`), which adds measurements.

Each milestone takes one area: its menu items, its hotkeys and its
Preferences pages. By its end, every item of the area either works or
keeps a reason:

- A working item has PotPlayer's command IDs and default keys.
- A reason says why libmpv, FFmpeg, Qt or the platform cannot do the item.
  "Not in RVP yet" is no longer a reason.

Each milestone is built and accepted on Windows: on the Windows VM, and on
the host with permission. Linux keeps building and passing its headless
tests, and its own parts are listed for M17.

- **R2. PotPlayer's Preferences (a read-only reference).** At the start of
  M10, `tools/r1/potplayer_tables.py` also reads from the installed files:
  - the Preferences window's page tree;
  - each page's options, with their types, their choices and the defaults
    that the resources show (the dialogs in `PotPlayer64.dll`, with texts
    from `Language/English.ini`'s `[Dialog(N)]` sections);
  - the command-line switches from `CmdLine64.txt`.

  It writes `docs/r1/preferences.tsv` and `docs/r1/command-line.tsv`. A
  test like the menus' one writes `docs/coverage/preferences.md`. No
  program is started.

  The menus' coverage report also grows. Today it reads only the static
  tree (`menu-static.tsv`, `menu-playlist.tsv`). PotPlayer builds 720
  more commands at run time (`menu-runtime.tsv`), for example:
  - Create subtitles from audio, with its Whisper engines and models;
  - the Subtitle Translation modes;
  - 360° Video Mode, and H.264 MVC 3D decoding;
  - the A-B loop counts, the bookmark options, and the playlist's file
    moves and copies;
  - the context menu style.

  These are counted too. A run-time list (devices, languages, shaders,
  colour themes, presets) counts once, as the function that fills it.

### M10. PotPlayer parity: audio (M)

- **Windows first: M8's gap.** RVP keeps the display on while video plays
  (`SetThreadExecutionState`, the counterpart of Linux's idle inhibit).
  Playback's "Prevent screen saver during playback" turns this on and off.
- **Output.**
  - The Audio Renderer menu lists the devices (`audio-device-list`).
  - Exclusive mode (`audio-exclusive`).
  - Output Mode with pass-through of AC3, E-AC3, DTS, DTS-HD and TrueHD
    (`audio-spdif`). Only a receiver can play it, so tests check the
    options.
  - Output Format/Resample (`audio-format`, `audio-samplerate`, and the
    resampler's quality through `audio-swresample-o`).
  - The speaker layouts of R1's list that RVP lacks (3.0 to 6.1).
  - Dolby Surround and Pro Logic II mixes (swresample's `matrix_encoding`).
  - Virtual surround for headphones: `sofalizer` with an HRTF file where
    the build has libmysofa, else `headphone`, `crossfeed` or `bs2b`. The
    goal records the choice.
- **A second device.** Audio Renderer (2nd Device) plays the same sound on
  a second device at the same time. libmpv has one audio output, so a
  second core without video plays the same file to that device:
  - it has the same chain, and takes the normalizer's gain from the main
    core (TomSteady reads the main core's log);
  - it follows the main core: seeks, pauses, the speed and track changes
    are mirrored;
  - small speed changes correct the drift between the two devices'
    clocks, and a delay per device lines up their latencies;
  - it plays files and discs only, because a stream may not open twice.

  On Linux, PipeWire can also combine two outputs (M17). The host has two
  devices (the FiiO and the monitor's HDMI audio) for a check with
  permission.
- **Channels.** Both, Left or Right channel of a stereo track (a labelled
  `pan`), and Swap Stereo Channels.
- **Volume.**
  - PotPlayer's "System Wave" and "System Master" volume control: the
    slider and keys drive the session's or the device's volume (Windows'
    Core Audio through FFI in `shell/`; PipeWire in M17).
  - The System Volume menu: master and wave up, down and mute.
  - Individual volume (Volume Control's "Enable individual volume
    control", R2): a master gain and a gain per channel (left, right,
    centre, back and side pairs, LFE), a labelled stage after the
    normalizer.
  - Pre-amplifying AC3 and DTS, with PreAmp up and down (a labelled gain
    stage).
  - The volume control settings.
- **Equalizer.** PotPlayer's 19 presets with their values, and the bands
  and preamp of its equalizer page. The values look like Winamp's; check
  them against PotPlayer read-only, or measure them on the host with
  permission.
- **Sync.** The Adjust Audio Sync dialog, and keeping sync changes: never,
  over the runtime, or always (per file, in the resume store).
- **Effects.** Each effect is a labelled filter:
  - Crystality, built from FFmpeg's `aexciter`, `extrastereo` and
    `crystalizer` and fitted to PotPlayer's output;
  - Noise Reduction (`afftdn`);
  - Freeverb: Jezar's public-domain algorithm. RVP computes the impulse
    response for PotPlayer's parameters and runs it with `afir`;
  - Voice Removal (centre cancel);
  - Voice Emphasis (`dialoguenhance`);
  - Crossfeed.

  (Goal L's host comparison replaced these with fits to PotPlayer's
  measured effects, and found Audio Stretching to be a pitch shift:
  `docs/audits/audio.md`, "M10".)

  The chain has one documented order: downmix, channel, preamp,
  equalizer, effects, normalizer, tempo. It keeps the end-of-file rule
  (`App::defer_chain_change`).
- **Speed.** Playback's three time-stretching items:
  - high quality: `scaletempo2`, as today;
  - default quality: `scaletempo`;
  - pitch shifting: `audio-pitch-correction=no`.
- **Normalizer.**
  - PotPlayer's block per source (audit finding 2: 2048 samples for stereo
    FLAC, 3072 for 5.1 PCM, 1536 for AC3), once its rule is found.
  - "Operate only with video playback".
  - The rest of the Normalizer/Freeverb page.
- **Recording.** Audio Recording writes what RVP plays, after its chain,
  to a WAV or FLAC file, with the storage folder options. On Windows it
  uses process loopback capture
  (`AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`), which records RVP's
  own process only.
- **Not possible.** The built-in audio transform filter (DirectShow; its
  functions are FFmpeg's filters above).
- **Tests.**
  - Headless chain tests: labels, order, toggles while playing, the end
    of a file.
  - Level tests on synthetic tones like `tests/speakers.rs`: the
    channels, swapping, voice removal, the preamp and the Pro Logic II
    matrix.
  - The presets' gains and the recording.
  - The self-test and `m10-all.ps1` on the Windows VM.
  - One host session, with permission, that compares RVP with PotPlayer
    on the audit's tones: the new effects, the presets and the block
    sizes.

Exit: the Audio menu, its hotkeys and R2's audio pages are covered. The
host comparison matches within the audit's tolerances, or its
differences are listed.

### M11. PotPlayer parity: playback, playlist and opening (L)

- **The thumbnail core.** A second libmpv core without audio renders
  frames into small images with the software render API. It has a thread
  of its own (never the GUI thread) and a cache. It serves:
  - the seek bar;
  - the scene browsers;
  - the playlist's thumbnail views;
  - M12's thumbnail images.

  It also finds durations in the background ("Detect file durations").
- **Seek bar.** Time tooltips while hovering and seeking, thumbnail
  previews, and markers for chapters, bookmarks and A-B points.
- **Jumps.**
  - The Jump to Time/Frame dialog.
  - The previous and next keyframe, and the current subtitle's position.
  - Scene browsers by time intervals, by markers and by subtitle
    intervals.
  - The time jump settings.
- **Repeat and skip.**
  - A-B fine moves, repeating the current subtitle or the marked section,
    and A-B lists kept per file.
  - Skipping the intro, the ending and chapters, with the skip setup.
  - Shuffle playback.
  - Bookmarks per file (add, remove, rename, jump, list) beside the file's
    chapters.
- **When playback finishes.**
  - Shut down, sleep or hibernate after the file or after the list. A
    notice counts down first, and the user can cancel there.
  - Shut down at a set time or after a set time.
  - Tests use a dry-run hook and never power off a machine.
- **Other playback items.**
  - Send played and related files to the Recycle Bin, with confirmation
    (the `trash` crate or the shell's file operations).
  - Hide the pointer during windowed playback.
  - Remembering positions as a toggle.
- **Opening.**
  - Load External Audio (`audio-add`) and Reload Subtitle.
  - DVD (`dvd://`, a folder or an `.IFO`) and Blu-ray (`bd://`, a folder
    or an `.MPLS`): titles, chapters, angles, audio and subtitle streams.
    DVD Control's speeds use `play-direction=backward` for reverse play
    if it plays smoothly. The discs' menus are M16's. (Goal M: backward
    play is smooth on files but stalls on a disc's stream, so discs
    refuse it.)
  - Audio CDs (`cdda://`).
  - Webcams and other devices (FFmpeg's `dshow` on Windows), listed from
    the system.
  - The screen as a source (`gdigrab` or `ddagrab`).
  - Network addresses that FFmpeg opens (HTTP, HTTPS, FTP). A browser for
    FTP and WebDAV folders goes in only if it is cheap.
  - Streaming sites through the user's yt-dlp. libmpv has no Lua, so RVP
    resolves the address itself, as mpv's `ytdl_hook` does.
  - The clipboard, recent items and favorites (files and folders).
  - The Default Open Action, and "When Files Opened": add only the
    selected files, similar files, or all files of the folder.
- **Playlist panel.**
  - The playback queue.
  - Sorting by title, name, extension, size, duration, date and folder.
  - Search, editing titles, renaming files, and opening the file's folder.
  - Copy and paste, also of the recent list; Add URL.
  - File information.
  - Playlist files in and out (`.dpl`, `.m3u8`, `.pls`), and remembering
    the default playlist and recent URLs.
  - Thumbnail views: 2 to 7 rows with details, 16:9 thumbnails.
  - Starting albums from menus, and the playlist storage folder.
  - Deleting files to the Recycle Bin, with confirmation.
  - "Play within a new instance".
- **Command line.** PotPlayer's switches from R2's `command-line.tsv`
  that apply (seek, subtitle, title, playlist and instance switches).
- **Studies.** Seamless playback (gapless audio, preloading the next
  file), and PotPlayer's file navigator and Open Box. Each goes in only if
  it keeps the album rules (mpv plays one entry at a time) and the GUI
  thread free. (Goal M: neither went in, `docs/spikes/m11-studies.md`.)
- **Not possible.** TV tuners (WDM/BDA; non-goals).
- **Tests.**
  - Headless tests on synthetic media: chapters, keyframes, subtitles,
    A-B lists, skips, the queue, sorting, playlist files, and a DVD folder
    made with free tools.
  - The thumbnail core: sizes, timing, cancellation, never on the GUI
    thread.
  - The power hook and the command line.
  - The self-test and `m11-all.ps1` on the Windows VM.

### M12. PotPlayer parity: video, picture and window (L)

- **Frame and aspect.**
  - Custom aspect ratios and their dialog.
  - Keeping the frame at 0.5x to 2x of the output size.
  - Correcting the output device's aspect (`monitorpixelaspect`), and
    adjusting the aspect to the window.
  - Extend/Crop presets and by pixel, with moves (`video-crop`,
    `video-pan-x`, `video-pan-y`).
  - Bottom margins for subtitles (5 to 30 %, or 1 to 3 lines, optionally
    only when subtitles exist; `video-margin-ratio-bottom`).
  - Screen margins and their settings.
  - Auto rotation by aspect ratio.
- **Window.**
  - Minimum, maximum and custom sizes, with their dialog; increase and
    decrease; size presets (add and edit).
  - Fullscreen (Stretch by Keeping AR).
  - Window moves: left, right, up, down, the centre, the corners. Wayland
    forbids them.
  - The Window Size Control dialog, and staying on top in fullscreen.
  - Desktop Mode (the video behind the desktop icons, Windows only), as a
    study.
- **Effects.** All are RVP's labelled filters before `@svp` (goal H's
  rule 4):
  - rotation by any angle (`rotate`);
  - auto level and level control (`normalize`, `colorlevels`);
  - gradual and temporal denoise (`hqdn3d` levels, `atadenoise`);
  - luma and chroma offset (`chromashift`);
  - motion blur (`tmix`);
  - WarpSharp, approximated (`cas` or `unsharp`), with the difference
    listed.
- **3D video.** FFmpeg's `stereo3d`:
  - input side by side or top and bottom;
  - output side by side, top and bottom, row or column interleaved,
    anaglyph, or one eye;
  - swapping the eyes, full resolution, and detection from the file's
    stereo metadata.

  3D subtitles are a study, and so is H.264 MVC (3D Blu-ray) decoding:
  FFmpeg decodes only the base view.
- **360° video.** 360° Video Mode turns equirectangular, dual fisheye and
  other projections into a flat view (FFmpeg's `v360`), and the mouse
  turns the view.
- **Shaders.** PotPlayer's pixel shaders (`PxShader`) are HLSL in MPC-HC's
  single-pass `ps_2_0` form: `sampler s0`, the `p0` and `p1` constants,
  `tex2D`. RVP translates them into mpv's GLSL hooks when they load, from
  PotPlayer's folder or the user's.
  - Pre-resize shaders hook before scaling (`MAIN`), and post-resize ones
    after it (`OUTPUT`), so PotPlayer's two lists come back.
  - A shader that does not translate is listed with the reason.
    glslang's HLSL front end with SPIRV-Cross is the fallback for harder
    ones.
  - RVP ships a default set of mpv GLSL shaders (scalers, sharpeners,
    chroma upscaling) that work without PotPlayer, in the shader lists
    from the first start. Each is under a GPL-compatible license and is
    listed with its origin and license in a sources file (owner request,
    2026-10-18).
- **AviSynth.** The user installs AviSynth+. Then:
  - RVP opens `.avs` scripts like files (the Windows libmpv's FFmpeg
    reads them);
  - PotPlayer's AviSynth processing of the playing video (Video >
    AviSynth presets, with `potplayer_source()`) runs through RVP's
    VapourSynth slot and a bridge that runs AviSynth inside VapourSynth.
    One candidate is avsw's `Eval`, with the playing clip injected; a
    study picks the bridge;
  - VapourSynth scripts can load AviSynth plugins through
    `avs.LoadPlugin`.

  If no bridge runs the playing video well, that part keeps its reason.
- **Decoders.** A decoders page takes the place of DirectShow's filter
  priorities: per codec, hardware or software decoding (`hwdec-codecs`),
  and mpv's decoder and demuxer choices.
- **Rendering.** mpv's choices take the place of DirectShow's renderers:
  - `gpu-next` or `gpu` inside the render API;
  - the scalers (`scale`, `dscale`, `cscale`) and when to resize
    (`video-unscaled`);
  - tone mapping and peak detection for HDR, and the display's ICC
    profile (the Pixel Shaders menu's ICC, YCgCo and HDR items map here);
  - on Windows, D4's optional path as a renderer choice: mpv in a child
    window (`wid`) with Direct3D 11 and HDR output. A study first checks
    it against S3's measurements and SVP's copy-back rule.
- **Capture.**
  - Copying the source or the screen frame to the clipboard.
  - Save As, and BMP (RVP writes it from `screenshot-raw`).
  - File names with the date and time or the elapsed time
    (`screenshot-template`).
  - Messages and subtitles in the image, and aspect correction.
  - Consecutive images: every N seconds, a set count.
  - Thumbnail images: a contact sheet from the thumbnail core, with its
    grid settings.
  - Storing images next to the media.
- **Recording.** Record Stream saves the source as it is (`stream-record`),
  started and stopped from the menu, with the storage folder options; the
  Tools menu's Record Stream is the same action. Record Video (the picture
  with RVP's filters) needs libmpv to encode in a second core. It goes in
  only if a study shows that works; otherwise it keeps its reason.
- **Visualization.** For audio files:
  - the album art toggle (`audio-display`);
  - FFmpeg's visualizations (`showcqt`, `showspectrum`, `avectorscope`,
    `showwaves`, through `lavfi-complex`), chosen in a menu;
  - a slideshow of the folder's images, with previous and next slide.
- **Settings window.** PotPlayer's tree, moved here from M14 at the
  owner's request (2026-10-18): groups (General, Playback, Subtitles,
  Device, Video, Audio and the rest) open into their pages, and the
  selected group's pages show as tabs over the page, as in PotPlayer.
  M12's new pages go into it.
- **Not possible.**
  - DirectShow renderers and transform filters (non-goals; their
    counterparts are above).
  - WMP visualizations, PotPlayer's floating ball, and downloading
    visualizations.
- **Tests.**
  - Headless property and filter tests: labels, order, `@svp` untouched.
  - The 3D modes on a synthetic side-by-side clip, with pixel checks on
    `screenshot-raw`.
  - Capture formats and file names, and the contact sheet's geometry.
  - Window commands in the self-test, and `m12-all.ps1` on the Windows
    VM.
  - One host session, with permission, for the renderer choice, HDR and
    SVP with the new filters.

### M13. PotPlayer parity: subtitles (M)

- **Tracks.**
  - A second subtitle at the same time (`secondary-sid`), and Combine
    Subtitle.
  - Reload.
  - Copying the current subtitle to the clipboard (`sub-text`).
- **Style.**
  - Bold, horizontal spacing (`sub-spacing`), and the opaque box
    (`sub-border-style`).
  - The fade effect, if libass allows it.
  - Sideways moves, where mpv allows them (`sub-margin-x`, `sub-align-x`).
  - The font style settings: font, size, colours, outline, shadow,
    background.
- **Placement.** Bitmap subtitles outside the video, text subtitles inside,
  or forced under the video (`sub-use-margins`,
  `image-subs-video-resolution`, `sub-ass-force-margins`), with M12's
  bottom margins.
- **Sync.**
  - The Adjust Subtitle Sync dialog, and keeping sync changes like
    audio's.
  - FPS conversion (`sub-fps`).
  - The language and sync page: preferred languages (`slang`), the text
    encoding (`sub-codepage`), and loading by file name (`sub-auto`).
- **Files.** RVP's own reader and writer for SRT, ASS and WebVTT serve
  external text subtitles:
  - saving them with the sync applied, under the video's name or as a
    chosen file;
  - a subtitle browser and editor: the cues with their times, jumping,
    editing text and timing, saving;
  - Input Subtitles.
- **Generated subtitles.** PotPlayer's "Create subtitles from audio",
  ahead of time or in real time, runs offline, with no account and no
  service:
  - speech recognition with whisper.cpp (MIT licence; on the CPU, Vulkan
    or CUDA);
  - Whisper models that the user downloads, from tiny to large-v3-turbo;
  - the language chosen, or detected.

  The text becomes a subtitle track and can be saved. PotPlayer's other
  engines (Const-me, Faster-Whisper) are separate programs; RVP runs them
  only if the study finds them worth it.
- **Online (deferred).** The owner decided on 2026-10-16 that subtitles are
  added by hand for now. When the owner asks for it, RVP gets search and
  download, and translation with PotPlayer's display modes (translation
  only, above or below the source), in one of two ways:
  - PotPlayer's own extensions (OpenSubtitles, podnapisi, SubDL, DeepL,
    Google, LibreTranslate, LM Studio and others), run by an AngelScript
    host. AngelScript is zlib-licensed. The host implements the about 95
    functions of `Extension/api.txt` and asks before an extension loads a
    native library or starts a program. Extensions are read from the
    user's PotPlayer folder or RVP's own, never shipped;
  - services that RVP calls itself.

  Keys and accounts are the user's settings.
- **3D.** Subtitles for M12's 3D modes, if its study finds a way.
- **Not possible.** A font renderer other than libass; turning off ASS
  animations (libass always animates).
- **Tests.** No online service is used. The tests cover:
  - reader and writer round trips on subtitle files that the tests
    generate (SRT, ASS, WebVTT), and on free samples kept locally in
    `~/rvp-reference`;
  - the editor's model;
  - the style and placement properties on a headless player;
  - generated subtitles on a synthetic speech clip (made with a free
    speech synthesizer such as espeak-ng) with the tiny model. They are
    skipped where the model is absent;
  - the self-test and `m13-all.ps1` on the Windows VM.

### M14. PotPlayer parity: preferences, skins and the rest (L)

- **Preferences.** Every R2 option outside M10-M13's areas becomes a
  setting or keeps a reason: general, startup, window, OSD and messages,
  the playlist, and the rest. The settings window (PotPlayer's tree with
  tabs since M12) gets a search and a reset per page, and keeps RVP's
  look and its sliders.
- **Presets.** Manage Presets: named settings profiles to save, switch
  and delete (PotPlayer's Configuration page), with import and export as
  today.
- **On-screen display.** PotPlayer's OSD options (font, size, colours,
  position, duration, which messages show), and the Display Misc Info and
  Short Playback Info items.
- **Info window.** Playback/System Info (Ctrl+F1) shows the file, its
  streams and codecs, the filter chains and the system, and copies them
  to the clipboard. The overlay stays on its own key.
- **Skins.**
  - The skin list in the Skins menu: the skins folder, and switching while
    running.
  - Colour themes: HSB and RGB through the skin's `ColorTheme`, and the
    Customize dialog.
  - The main window's opacity.
  - Logo settings, while idle and while starting: the user's own image,
    or RVP's logo by default.
  - The on-screen control mode (`MainSkinAreaOscInside`: controls over the
    video).
  - The audio skin mode (`AudioSkin.xml` for audio files).
  - Auto hiding the main skin; auto resizing the video for fullscreen
    pop-ups.
  - Using the video skin at start, locking the skin mode, and remembering
    the window per skin mode.
  - The skin settings.
  - Other `.dsf` skins, checked with the user's files, with a list of
    what each one lacks.
- **Misc.**
  - Copying the item's name or path.
  - Removing the history of opened files, and Windows' jump list with
    recent files.
  - Disable default hotkeys.
  - The context menu's style, including the system's dark mode.
  - Interface languages through Qt's translations: English and Russian,
    written for RVP, never taken from PotPlayer's language files.
  - Touch gestures (Qt's touch events, tested with synthetic events).
- **Not possible.**
  - PotPlayer's AngelScript extensions; RVP has yt-dlp.
  - The built-in Korean interface.
  - Daum's services and broadcasting.
  - The update service and the website; the repository is private.
- **Tests.**
  - The preferences coverage report.
  - Profiles round trip.
  - Skin switching and colour themes on the synthetic skin and RVP
    Default, with golden renders.
  - Translations load and fit their controls.
  - The self-test and `m14-all.ps1` on the Windows VM.

Exit (M10-M14): on Windows, every item of R1's menus and hotkeys and every
R2 option works in RVP, or keeps a reason that says why libmpv, FFmpeg, Qt
or the platform cannot do it. M17 brings Linux to the same level.

### M15. Remote control: RVP's API and a Unified Remote remote (M)

The owner asked for a remote for Unified Remote on 2026-10-16. Unified
Remote's server runs on the PC and runs Lua remotes, which the phone app
shows (D15).

- **Control API.** A small HTTP/1.1 JSON service inside RVP. It is off by
  default; a Remote settings page turns it on and shows its address and
  token. It offers:
  - `GET /api/status`: the title, the file, the album and index, the
    position and duration, paused, the volume in PotPlayer's percent and
    its maximum, mute, speed, fullscreen, the audio and subtitle tracks,
    the normalizer and the equalizer;
  - `GET /api/actions`: the names and PotPlayer texts of the `ACTIONS`
    table;
  - `POST /api/action/NAME`: any action, through the same dispatch as keys
    and menus;
  - seek, volume, open (a path or URL), the albums, and playing an entry;
  - a long poll for changes, so a remote need not poll fast.
- **Rules.**
  - It listens on 127.0.0.1 only (owner, 2026-10-16). Other devices reach
    RVP through Unified Remote's server.
  - Every request carries a token that RVP generates per installation. It
    is kept in RVP's state folder, shown in the settings and renewable
    there.
  - Requests with a browser's `Origin` header are refused. Sizes and times
    are limited.
  - The server's thread never calls mpv. Commands go to the window's
    thread, as MPRIS's and SMTC's do (`App::media_command`). The status
    is a snapshot that the window's thread keeps.
  - The server is a minimal one on `std::net`, or one small dependency
    with a reason.
- **Unified Remote remote.** A custom remote in `remote/unified-remote/RVP/`:
  - `meta.prop`;
  - `layout.xml`;
  - `remote.lua`, which calls the API through Unified Remote's `http`
    library;
  - `settings.prop`, with the port and the token;
  - an icon that `cargo xtask` builds from RVP's logo.

  The layout follows Unified Remote's media remotes: play/pause, stop,
  previous and next, seek steps and a position slider, a volume slider
  and mute, fullscreen, audio and subtitle track cycling, speed, and the
  normalizer. It shows the title and the times, and a page lists the
  albums to pick an entry from.
- **Delivery.**
  - When Unified Remote Server is installed, the Windows installer offers
    to copy the remote into its custom remotes folder
    (`C:\ProgramData\Unified Remote\Remotes\Custom`). The uninstaller
    removes it, if the installer put it there.
  - The portable ZIP and the Linux packages carry the folder.
- **Optional (a study).** A subset of MPC-HC's web interface
  (`/command.html`, `/variables.html`), which several remote apps speak.
- **Tests.**
  - API tests on a headless player: every endpoint, the token, the
    `Origin` refusal, the limits, and nothing blocking the GUI thread.
  - Every action the remote calls exists, and `layout.xml` is well
    formed.
  - On the Windows VM, Unified Remote Server (installed on the VM only)
    loads the remote, and its calls reach RVP.
  - On the host, with permission, the owner tries it from a phone.

Exit: the owner controls RVP from Unified Remote on Windows. Linux
follows in M17.

### M16. DVD and Blu-ray menus (L)

mpv's developers disabled DVD and Blu-ray menus in 2014, and later removed
them. The highlights never drew right, and keeping the disc's state while
streams switched upset the playback core. libmpv now plays titles and
chapters only (M11). RVP can drive the menus itself, with the open-source
libraries that VLC and Kodi use.

- **Study first.** A spike records its answers in
  `docs/spikes/s7-disc-menus.md`:
  - how VLC (`modules/access/dvdnav.c`), Kodi (its DVD navigator and
    Blu-ray input stream) and mpv's removed `discnav` code handle
    navigation, still frames, highlights and stream changes;
  - whether RVP feeds mpv through a custom stream (libmpv's `stream_cb`,
    which `vendor/libmpv2`'s `protocol` module wraps) or reopens it per
    cell;
  - how the highlights are drawn: mpv's `overlay-add`, or a layer over
    the video.
- **DVD.** libdvdnav and libdvdread (GPL-2.0-or-later) run the disc's
  virtual machine:
  - the menus show with their button highlights;
  - the mouse and the keys (arrows, Enter) pick buttons;
  - DVD Control's Title, Root, Chapter, Audio, Subtitle and Angle menus
    work, and so does Previous Menu;
  - still frames hold, and the machine's audio and subtitle choices
    apply.

  Encrypted discs need the user's own libdvdcss, which RVP never ships.
- **Blu-ray.** libbluray (LGPL-2.1-or-later) runs HDMV menus and gives
  their overlays to RVP. BD-J menus need libbluray's Java part and the
  user's Java runtime, so they work only where those run. RVP ships no
  AACS keys or libraries; encrypted discs play only with what the user
  installed.
- **Rules.** The libraries are GPL-compatible, but D8 waived the no-GPL
  rule for libmpv and Qt only, so the goal asks the owner before adding
  them. Their FFI goes into `shell/`, under the unsafe rule.
- **Tests.**
  - A DVD with menus, authored in WSL with free tools (`dvdauthor`,
    `spumux`; synthetic video and buttons).
  - Headless navigation tests on it: moving between buttons, activating
    one, stills, starting a title, returning to the menu.
  - Blu-ray HDMV menus on a free sample if one exists, otherwise on the
    owner's discs on the host, with permission.
  - The self-test and `m16-all.ps1` on the Windows VM.

Exit: DVD menus work on Windows as in PotPlayer. Blu-ray HDMV menus work
where the disc allows. BD-J and encrypted discs depend on what the user
installed.

### M17. Linux completion (M)

- **The Linux side of M9-M16 and M18** (each goal's status file lists
  its items under "Linux items for goal S"):
  - the system's volume through PipeWire or PulseAudio;
  - audio recording from RVP's own PipeWire stream;
  - the second audio device; PipeWire can also combine outputs;
  - devices through `v4l2`;
  - the screen as a source, as a study: Wayland needs the screencast
    portal;
  - the trash;
  - power actions through logind;
  - in the kit's libmpv: DVD, Blu-ray and CD support (libdvdnav,
    libbluray, libcdio), the chosen audio filters' libraries (libmysofa,
    librubberband, libbs2b), and Lua with mpv's built-in scripts for
    M9's on-screen controllers;
  - M9's skin windows of their own (classic Winamp's and VLC's) docked,
    moved together and shaped on Wayland, which lets no client place its
    windows: a way that works, or the reason listed;
  - M16's disc menus with the same libraries;
  - AviSynth through AviSynth+ for Linux, if the user installs it;
  - generated subtitles with whisper.cpp's CUDA or Vulkan build;
  - the window commands Wayland refuses, listed with their reason;
  - M14's tray icon, pausing when the screen locks, text to speech, the
    Xbox controller, the file types through the desktop entry and
    `xdg-mime`, the taskbar's progress and the recent files;
  - M18's VapourSynth runtime in the packages, NVIDIA Optical Flow in
    the kit's libmpv, and waiting for SVP for Linux;
  - Unified Remote Server for Linux with the remote;
  - the translations;
  - zbus 6 for MPRIS and idle inhibit (zbus 5's helper crates are
    flagged unmaintained; `deny.toml` ignores them until then).
- **Checks.** The Ubuntu VM runs the self-test's new sections and kit
  steps `m9` to `m16` and `m18` (with `m17all` for all of them). The
  packages (AppImage, `.deb`, PKGBUILD) carry the new parts. The
  checklist gains sections for the owner's Ubuntu boot.
- **The Ubuntu boot run.** One session on the Ubuntu boot checks M6 to
  M18 (section 10). It takes the place of the run for M6, M7 and M8 if
  that has not happened by then.

Exit: the kit passes on the Ubuntu VM, and the owner's Ubuntu boot run
passes.

### M18. Motion smoothing without SVP (M-L)

Added on 2026-10-18 at the owner's request (D16): smooth motion like
SVP's for users without SVP. It need not match SVP in full, but it must
be a substantial part of it. It runs after M12 (goal T), Windows first.

- **Study.** Each candidate is measured on the Windows VM and, with
  permission, on the host's RTX 30-series card: the rate reached at 720p, 1080p
  and 4K, CPU and GPU load, dropped frames, and artefacts on synthetic
  clips and on clips the owner names (kept local).
  - mpv's own interpolation (`video-sync=display-resample`,
    `interpolation`, `tscale`): it blends neighbouring frames at the
    display's rate without motion estimation. It is cheap and keeps
    hardware decoding.
  - FFmpeg's `minterpolate`: motion-compensated, on the CPU, in one
    thread; probably for small videos only.
  - MVTools in VapourSynth (`mv.Super`, `mv.Analyse`, `mv.FlowFPS` or
    `mv.BlockFPS`), the method SVP grew out of, through RVP's VapourSynth
    slot with a script RVP writes.
  - RIFE in VapourSynth (ncnn-Vulkan, or TensorRT on NVIDIA): optical
    flow from a neural network on the GPU.
  - NVIDIA's Optical Flow FRUC library, if its license lets RVP ship it.

  The study writes `docs/spikes/m18-smoothing.md`: what each engine
  reaches, how RVP gets it onto the user's machine (files bundled in the
  packages under GPL-compatible licenses, or the user's own install), and
  a recommendation. The owner picks the engines and the defaults.
- **Modes (D18).** Off (the default), Blend (mpv's interpolation, always
  there), MVTools, RIFE and fruc_vulkan, with quality presets and a
  target rate (the display's by default, twice the source, 60 fps or a
  number). Turned on, smoothing is MVTools medium up to 1080p and Blend
  above. MVTools and RIFE run in a VapourSynth runtime RVP ships;
  fruc_vulkan needs a libmpv RVP builds with FFmpeg's master
  (`tools/libmpv`, RTX 30 and later) and the Vulkan renderer, a third
  renderer choice beside OpenGL and Direct3D 11 (its frames stay on the
  output's Vulkan device).
- **Rules.** The smoothing is RVP's own labelled filter (`@rvpsmooth`)
  after RVP's other filters, or mpv's interpolation options. It never
  touches `@svp` and turns off while SVP is attached. It needs copy-back
  decoding, as SVP does. It stays off above a size or rate the user sets.
- **GPU power budget (D17).** The smoothing modes are steps of the
  budget (`crates/app/src/gpu_power.rs`): above the budget a lighter
  preset first, then Blend or off, before RVP's other steps, and back the
  same way. The study's GPU runs on the host go at the real power limit
  of `gpu-limit.ps1` (`m18-study.ps1 -Gpu`, `-PowerLimit 90`).
- **Interface.** Video menu entries for the modes, with a hotkey; a page
  in the settings window's Video group; the rate reached in the status.
- **Tests.** Frame counts and timestamps of a smoothed synthetic clip of
  a moving square; pixel checks of an interpolated frame (the square
  halfway); toggles during playback; `@svp` untouched; the budget's
  steps with the smoothing (`rvp --selftest=gpu`); the self-test and
  `m18-all.ps1` on the Windows VM; one host session with permission for
  the GPU modes, at the 90 % power limit.

Exit: on Windows, video plays smoothed at the target rate without SVP in
the modes the owner picked, within the limits the study measured.

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
- Coverage reports (`docs/coverage/`): every PotPlayer menu item, hotkey,
  skin command and Preferences option (R2) maps to an RVP action or
  setting, or names the reason it cannot. After M14, no "not in RVP yet"
  reasons remain for Windows; after M17, none remain for Linux.
- Actions that reach outside RVP run against test doubles: shutting down
  and sleeping (a dry-run hook), the Recycle Bin (temporary folders), and
  online services (a local fake server). The system's volume is changed
  only on the VMs.
- From M10 on, each milestone has a Windows VM script
  (`tools/windows-vm/mNN-all.ps1`); M17 adds the matching kit steps.

## 8. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| SVP rejects a process that is not `mpv` | Breaks a core requirement | S1 first; fallbacks listed there |
| libmpv builds without VapourSynth, or Python version mismatch | No SVP | S2; ship our own libmpv |
| Normalizer sounds different from PotPlayer's | Daily-use complaint | S5 measurement; PotPlayer's own method reproduced and checked against S5's recordings (D13) |
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
| The default skin is too close to BlackSpace | Copyright or trade dress claim | M7's rules: original art with a sources file, its own design, no Daum or PotPlayer marks; the owner decides on a legal review before any public release |
| Other players' skins depend on those players' internals (VLC's variables, mpv's Lua scripts) | Partial support | M9 starts with a study; unsupported parts are listed as in BlackSpace's coverage report |
| Codec patents when shipping FFmpeg in our builds | Legal, mainly for public binaries | Same practice as mpv and VLC builds; decide before any public release |
| The size of PotPlayer parity (about 500 menu commands and the Preferences tree) | Goals that never end | One area per milestone (M10-M14); coverage reports give each gap a reason; Windows first (D14) |
| Filters or protocols missing from a libmpv build (libmysofa, rubberband, bs2b, dvdnav, libbluray) | A function works on one system only | Check the build's filters first and list the gaps; the kit builds Linux's libmpv with them (M17) |
| The control API lets other programs control RVP | Unwanted control | Off by default, 127.0.0.1 only, a token on every request, browser requests refused (M15, D15) |
| Online subtitle search and translation need accounts and keys | Cost and privacy | Deferred by the owner (2026-10-16); when it comes, the owner picks the services, keys stay in the user's settings, and tests use a fake server |
| Disc menus: mpv's own menus failed and were removed (2014) | A large task that stalls | A study of VLC's and Kodi's code first; a synthetic DVD for tests; titles and chapters already play (M11) |
| PotPlayer's extensions and shaders from its folder (M12, M13) | Extensions can load native libraries, start programs and reach the network | Load only from folders the user chose; ask before a native library or a program; shaders run as mpv's GLSL |
| Shutting down, sleeping and deleting files | Lost work or files | A cancellable countdown, confirmations, the Recycle Bin; tests use a dry-run hook |

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
3. Online subtitles (M13): which services for search and translation?
   The owner's answer, 2026-10-16: subtitles are added by hand for now,
   with no online search or translation. Tests use generated files and
   free samples. The online part waits until the owner asks for it.
4. Remote control (M15): should only Unified Remote on the same PC reach
   the API? The owner's answer, 2026-10-16: yes, 127.0.0.1 only for now.

Deferred items (owner decisions, picked up when they become relevant):

- Dropped frames after mpv loops a file (`loop-file`; goal E saw about 30
  per second in fullscreen on the Windows host after the loop). Deferred
  until looping becomes a feature (2026-10-11).

## 10. Goals for `/goal`

The milestones are grouped into twenty goals, A to T. Each goal is one
`/goal` run, writes `docs/goals/goal-<letter>-status.md` and ends with a
commit pushed to the private `origin`. Goals run in the order of the table
below; C and D can swap. Goal I follows H so that the default skin can
include M6's control panel. Goals I to K were added on 2026-10-13 at the
owner's request, and packaging became M8 and goal J. Goals L to S were
added on 2026-10-16 (D14): on Windows, PotPlayer parity area by area, the
remote control and the disc menus (L to R); then goal K; then Linux (S)
and one Ubuntu boot run for M6 to M17. Goal T (M18, motion smoothing
without SVP) was added on 2026-10-18 at the owner's request and runs
after goal N.

| Stage | Goal | Covers | Where it runs | Needs the owner |
|-------|------|--------|---------------|-----------------|
| 1 | A | Repository bootstrap, S4, M1 | WSL | No |
| 2 | B | S2 and S3 without the host desktop, Windows build path, Linux test kit | WSL, Windows VM, Ubuntu VM | No |
| 3 | C | S1, S3 performance, R1, S5 on Windows | Windows host desktop | Permission before every launch or capture |
| 3 | D | S2, S1, S3 on Linux with the GPU | A session started on the Ubuntu 26.10 boot | Privileged steps (driver, SVP install, NAS mount) |
| 4 | E | M2 player window | WSL and VMs | Final check on host and Ubuntu boot |
| 4 | F | M3 daily-use essentials | WSL and VMs | Week-long trial afterwards |
| 5 | G | M4 menus and commands, M5 settings window | WSL and VMs | Permission for one host check |
| 5 | H | M6 SVP and video processing | WSL, VMs, host | Permission for host checks |
| 6 | I | Goal H's leftovers without the Ubuntu boot, M7 own look: logo and default skin | WSL, VMs, host | Picks the logo, approves the skin, permission for one host check |
| 6 | J | M8 platform integration and packaging | WSL and VMs | Decisions in the goal text (2026-10-16); afterwards one Ubuntu boot session for M6, M7 and M8 |
| 7 | L | R2 (PotPlayer's Preferences), M10 audio parity; Windows keeps the display on during video | WSL, Windows VM, host | Permission for the host comparison |
| 7 | M | M11 playback, playlist and opening | WSL, Windows VM | Tries each goal's build on Windows |
| 7 | N | M12 video, picture and window | WSL, Windows VM, host | Permission for one host check |
| 7 | T | M18 motion smoothing without SVP | WSL, Windows VM, host | Picks the engines after the study; permission for the host sessions |
| 7 | O | M13 subtitles (by hand and generated; online deferred) | WSL, Windows VM | Tries each goal's build on Windows |
| 7 | P | M14 preferences, skins and the rest | WSL, Windows VM, host | Permission for one host check |
| 7 | Q | M15 remote control: RVP's API and a Unified Remote remote | WSL, Windows VM, host | Tries the remote from a phone (permission for the host session) |
| 7 | R | M16 DVD and Blu-ray menus | WSL, Windows VM, host | Approves the disc libraries (D8); discs for the Blu-ray check (permission) |
| 8 | K | M9 other players' skins (VLC, mpv's on-screen controllers, classic Winamp) | WSL, Windows VM (Linux in S) | Picks the scope after the study |
| 9 | S | M17 Linux completion (and M9's and M18's Linux side) | WSL, Ubuntu VM | Afterwards one Ubuntu boot session for M6 to M18 |

A `/goal` condition holds at most 4000 characters; keep each goal's text
below that (goal H's first text was 4382 and was refused).

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
/goal Execute goal E of docs/PLAN.md (milestone M2, player window MVP) in ~/rvp, only if goal-c-status.md and goal-d-status.md record a passed gate; otherwise stop and report. Follow the rules that M2 inherits from both gates (goal-c-status.md rules 1-4, goal-d-status.md rules 5-8) and recheck goal D's open item (dropped frames at 4K60 on the Ubuntu boot) in M2's GPU check. Build the real application: shell/ with cxx-qt and QML, crates/engine (libmpv wrapper, property model, threading as in section 5), crates/app (command dispatcher, input bindings with PotPlayer's defaults from R1, settings and a crash-safe resume store per D9). Deliver all of M2: the classic BlackSpace window with live video and every bottom-bar element bound to mpv properties, move, resize, maximize, minimize, fullscreen with auto-hide, drag and drop, file dialog, command line, next and previous file through mpv's playlist. Tests: golden-image comparison of headless renders against the R1 captures, headless libmpv engine tests, resume tests that kill the process at random points, input binding tests; functional checks on the Ubuntu VM and the Windows VM; add an M2 checklist to tools/linux-kit for the Ubuntu boot. Done when: tests and CI are green, the usability targets of section 4 are measured and reported, docs/goals/goal-e-status.md is written, and everything is committed and pushed. Do not: run anything on the Windows host desktop without advance notice and explicit permission, commit PotPlayer or SVP files or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal F:
```
/goal Execute goal F of docs/PLAN.md (milestone M3, daily-use essentials) in ~/rvp, only if docs/goals/goal-e-status.md records goal E as done; otherwise stop and report. Follow AGENTS.md, including its shell notes (nothing blocks the GUI thread during playback; chrome repaints go through Player::damaged), and keep the rules M2 inherited (goal-e-status.md). Deliver: the playlist panel from PlaylistSkinArea with albums as tabs, add, remove, reorder (also by dragging inside the list) and undo, double click to play, current, played and unplayable rows, the scroll bar and the fps footer, and PotPlayer's playlist keys from R1 table 118; automatic persistence and start-up restore of the last album, file and position; .dpl import with a configurable mapping of Windows drive letters to Linux paths (default V:\ to /mnt/media; the owner's mount is /mnt/nas/Media); single instance with /add, /insert, /current and /new; the normalizer with boost using the chain and parameters from S5, volume above 100%, its toggles and settings, and a blind A/B test the owner can run against PotPlayer's S5 recordings; an input binding editor for keyboard and mouse (wheel, clicks, double click, middle and side buttons) with conflict detection and reset to defaults; the window keeping the video's aspect ratio while it is resized during playback (R1 finding 1). Apply the owner's answers to goal E's open questions where PLAN.md or goal-e-status.md records them. If PLAN.md records the owner's yes to open question 2, also implement shared state across the dual boot as sketched there. Tests: the playlist model and undo, .dpl import on synthetic files, single-instance commands, the normalizer chain on synthetic audio, binding conflicts, kill tests for playlists as well as positions; extend `rvp --selftest` and run it on the Ubuntu VM and the Windows VM; add an M3 section and kit steps to tools/linux-kit for the Ubuntu boot. Done when: tests and CI are green, positions and playlists survive the kill tests, the owner's .dpl files from %APPDATA%\PotPlayerMini64\Playlist import correctly (read-only), the usability targets of section 4 are measured again, docs/goals/goal-f-status.md lists what the owner should try during the one-week trial, and everything is committed and pushed. Do not: modify the owner's PotPlayer playlists or settings, commit PotPlayer files or the contents of the owner's playlists, run anything on the Windows host desktop without advance notice and explicit permission, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal G:
```
/goal Execute goal G of docs/PLAN.md (milestones M4 and M5) in ~/rvp, only if docs/goals/goal-f-status.md records goal F as done; otherwise stop and report. Follow AGENTS.md, including its shell notes and its test rules (GUI programs started in WSL open on the owner's desktop and count as host runs; silent tests need no permission), and keep the rules M2 and M3 inherited (goal-e-status.md, goal-f-status.md). If the owner has reported findings from M3's one-week trial, fix what blocks daily use first and record it in docs/goals/goal-g-status.md. M4: the native context menu with PotPlayer's tree from R1 (docs/r1/menu-static.tsv and menu-runtime.tsv), limited to features RVP has, showing the current hotkeys and check marks, replacing the interim QML menus; the playlist panel's menu from R1's playlist tree; the 19 skin-defined menus of BlackSpace; dispatcher coverage for the live BlackSpace command IDs with a coverage report that names every unimplemented ID with a reason. M5: the settings window, loaded on first use, with the sections listed in M5 over the existing settings.json: playback and resume (seek steps, positions, reopening the last file, fitting and keeping the video's shape), playlist (the PotPlayer playlist folder and, on Linux, the drive map), audio (normalizer on, Boost, Level, Release time, volume step and maximum, output), video (decoding, output), SVP and VapourSynth (the IPC endpoint; the rest waits for M6), subtitles, input bindings (M3's Keys and Mouse editor as a page) and skins; changes apply at once where possible; portable mode (settings next to the executable when a marker file is there); import and export of the settings file. Carry over from goal F: find which self-test step drops 17-18 frames on the Windows host (the self-test prints the drops with every check) in one host run with the owner's permission, and fix it if RVP causes it. Tests: the menu trees from R1 against the action table (every item maps to an action or is listed as unimplemented), the coverage report, the settings round trip and validation, portable mode and import and export on synthetic files; extend `rvp --selftest` (menus open with the right keys, every settings page loads, a changed setting applies) and run it on the Ubuntu VM and the Windows VM (tools/windows-vm); add M4 and M5 kit steps and a checklist section to tools/linux-kit for the Ubuntu boot. Done when: tests and CI are green, the coverage report is in docs/, docs/goals/goal-g-status.md lists results, differences from PotPlayer and what the owner should check, and everything is committed and pushed; if goal-f-status.md does not record the Ubuntu boot run (kit.sh m3all), remind the owner that M3's acceptance needs it. Do not: run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission, change PotPlayer or SVP settings, commit PotPlayer or SVP files or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Ubuntu boot run for M3, M4 and M5 (after goal G; M3's acceptance needs it):
```
/goal On the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 30-series card), run the Linux kit for milestones M3, M4 and M5 of the private repository woffko/rvp and record the results; this is the Ubuntu boot run that M3's acceptance needs (docs/goals/goal-f-status.md, goal-g-status.md). Clone or pull the repository to ~/rvp and follow AGENTS.md and tools/linux-kit/CHECKLIST.md sections 1, 2, 5, 6 and 7. The owner does the privileged steps the checklist lists (the packages `kit.sh check` prints, `ydotoold` running as root for m2input, SVP 4 Manager started for m2svp); never use sudo yourself and never handle NAS credentials. Set RVP_SKIN to BlackSpace_en.dsf from the PotPlayer folder on the Windows partition, mounted read-only. Run `tools/linux-kit/kit.sh m4all` from a terminal inside the GNOME session; before each step that opens windows, tell the owner what will appear and for how long (nobody touches the mouse or keyboard during m2input). If a step fails, find the cause in its log in the kit's log directory; fix RVP only when the cause is clear and the fix small, run `cargo xtask ci` and rerun the step; otherwise record it as an open item. Then record the results: `kit.sh report` writes docs/test-runs/<date>-<host>.md; add an "Ubuntu boot run" section to docs/goals/goal-f-status.md (M3) and to docs/goals/goal-g-status.md (M4 and M5) with each step's result, the GPU checks (dropped frames in fullscreen and in a 1920x1080 window, compared with goal D's 6 per 20 s at 4K60), SVP, the usability metrics, and what the owner found in the checklist's manual checks of section 7. Commit with `git -c user.name=woffko -c user.email=2505149+woffko@users.noreply.github.com commit` and push. Done when the report and both status sections are committed and pushed. Do not: change SVP or system settings beyond the checklist's steps, commit PotPlayer or SVP files or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal H:
```
/goal Execute goal H of docs/PLAN.md (milestone M6, SVP and video processing) in ~/rvp, only if goal-g-status.md records goal G as done and goal-f-status.md records the Ubuntu boot run of M3-M5; otherwise stop and report. Follow AGENTS.md (shell notes; GUI programs started in WSL open on the owner's desktop and count as host runs; silent tests need no permission) and the rules M2-M5 inherited (goal-e- to goal-g-status.md), with rule 4 changed: RVP may set video filters, but only its own, each added and removed by its own label; SVP's @svp is never replaced, reordered or removed. If the owner reported findings from M3's trial or the manual checks, first fix what blocks daily use and record it in docs/goals/goal-h-status.md. Carry-overs from the Ubuntu boot run (goal-g-status.md): (1) find why Linux drops 14 frames in the 1.5 s after the speed returns from 1.1x to 1x (Windows host: 0) and fix it if RVP causes it; (2) the kit's self-test steps (m2, m3, m4) set ipc-server empty so a running SVP Manager cannot attach; (3) a self-test check that the picture is upright (a clip with a corner marker against ffmpeg's frame; SKIPPED with a note on the Windows VM, whose session-0 grabs show no video). M6: (a) SVP mode: keep RVP's default endpoints (mpvpipe, /tmp/mpvsocket), give a /new window an endpoint SVP still finds (section 2.4), force a copy-back hwdec while SVP mode is on, set hr-seek-framedrop=no, and show a diagnostic when SVP has not attached within a few seconds (Manager not running, endpoint mismatch, VapourSynth missing) as a notice and on the SVP settings page; (b) VapourSynth script slots running PotPlayer's .vpy files through the get_core wrapper (rule 2) with SVP's VapourSynth (rule 8); (c) GLSL shaders, deinterlacing, color adjustments (brightness, contrast, saturation, hue, gamma); (d) a defined filter order, deinterlacing and RVP's scripts before @svp, tested with SVP attached; (e) the F7 control panel from BlackSpace's ControlBoxSkinArea: color, the equalizer (a labelled audio filter beside the normalizer), subtitle and audio sync; (f) the settings window's SVP and VapourSynth, video and shader pages; (g) docs/coverage/ updated for M6's commands. Tests: headless libmpv tests for the filter chain (labels, order, @svp untouched, toggles during playback), shaders and color properties, the equalizer with the normalizer, and the .vpy slots where VapourSynth is installed (skipped otherwise); control panel geometry and hits on the synthetic skin and, locally, BlackSpace. Extend `rvp --selftest` (F7 opens, a color and an equalizer change apply, a shader toggles, the SVP diagnostic shows) and run it on the Ubuntu VM and the Windows VM (tools/windows-vm). Run the SVP acceptance (section 7: a 24 fps file reaches SVP's target fps; seeking keeps audio in sync) on the Windows host, with advance notice and the owner's explicit permission for each run. Add kit steps and checklist section 8 (M6 checks, SVP acceptance, GPU check) for the owner's Ubuntu boot, run in a later session; remind the owner that M6's acceptance needs it; this goal does not wait for it. Done when: tests and CI are green, the Windows SVP acceptance passes, goal-h-status.md lists the results, the differences from PotPlayer and what the owner should check, and everything is committed and pushed. Do not: run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change SVP or PotPlayer settings; commit PotPlayer or SVP files, scripts or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Ubuntu boot run for M6 (after goal H; replaced by the run for M6, M7 and
M8 after goal J below):
```
/goal On the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 30-series card), run the Linux kit for milestone M6 of the private repository woffko/rvp and record the results; this is the Ubuntu boot run that M6's acceptance needs (docs/goals/goal-h-status.md). Clone or pull the repository to ~/rvp and follow AGENTS.md and tools/linux-kit/CHECKLIST.md sections 1, 2, 5 and 8. The owner does the privileged steps the checklist lists (the packages `kit.sh check` prints, `ydotoold` running as root for m2input, SVP 4 Manager started for m2svp and m6svp); never use sudo yourself and never handle NAS credentials. Set RVP_SKIN to BlackSpace_en.dsf from the PotPlayer folder on the Windows partition, mounted read-only, and RVP_VPY to PotPlayer's VapourSynth/motioninterpolation.vpy there. Run `tools/linux-kit/kit.sh m6all` from a terminal inside the GNOME session; before each step that opens windows, tell the owner what will appear and for how long (nobody touches the mouse or keyboard during m2input). If a step fails, find the cause in its log in the kit's log directory; fix RVP only when the cause is clear and the fix small, run `cargo xtask ci` and rerun the step; otherwise record it as an open item. Then record the results: `kit.sh report` writes docs/test-runs/<date>-<host>.md; add an "Ubuntu boot run" section to docs/goals/goal-h-status.md with each step's result: the M6 checks, the picture upright, the script slots with SVP's VapourSynth (RVP's script, PotPlayer's with mvtools), SVP's acceptance (target fps, sync after seeks) and whether SVP for Linux finds the second window on /tmp/mpvSockets/<pid>, the GPU checks (m2gpu and m6gpu dropped frames against goal D's 6 per 20 s at 4K60), the frames dropped after the speed returns to 1x (--selftest=speed in m6 against its baseline; the M3-M5 run counted 14), the usability metrics, and what the owner found in the manual checks of section 8. Commit with `git -c user.name=woffko -c user.email=2505149+woffko@users.noreply.github.com commit` and push. Done when the report and the status section are committed and pushed. Do not: change SVP or system settings beyond the checklist's steps, commit PotPlayer or SVP files, scripts or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal I:
```
/goal Execute goal I of docs/PLAN.md (milestone M7, RVP's own look: logo and default skin) in ~/rvp, only if docs/goals/goal-h-status.md records goal H as done; otherwise stop and report. Follow AGENTS.md and the rules M2-M6 inherited. First, goal H's leftovers that need no Ubuntu boot, recorded in docs/goals/goal-i-status.md: (1) copy the current build to a new folder C:\Users\<user>\rvp-test\rvp-current in portable mode (rvp.portable, default settings with SVP mode on mpvpipe) for the owner's hands-on M6 checks and M3's trial, leave the other files there as they are, and tell the owner how to start it; (2) the control panel's subtitle font (ID_CAPTION_FONT_SELECT and CMD_CAPTION_FONT_MESSAGE: a font dialog loaded on first use, mpv's sub-font, a checked setting and a settings page entry) and the panel keeping its place between sessions; (3) M6's GPU check on the Windows host: the 4K60 clip in fullscreen and in a 1920x1080 window with colours, deband, a shader and the equalizer on, counting dropped frames (as kit.sh m6gpu), one run with advance notice and the owner's explicit permission. Then M7. Logo: draw two or three drafts of an RVP logo as vector sources, show them to the owner and wait for the choice; then use it as the window and application icon, on the idle screen and in the About box, and draw it over the Daum logo in a loaded skin's title bar (decision D10; the user's .dsf stays unchanged). Default skin: an original skin "RVP Default" in BlackSpace's spirit, with the same classic layout and functions (title bar, video, seek bar, buttons, info strip, the playlist docked on the right, M6's control panel, the same commands and menus) but its own shapes, icons, palette and fonts. Write it in the .dsf format from sources in the repository, build it with cargo xtask, ship it with RVP and use it when no skin is set. Follow M7's originality rules: take nothing from PotPlayer's skins (no files, sprites, crops, tracings or sampled colours); use no Daum, Kakao or PotPlayer names or marks; draw every asset for RVP or take it from icon sets and fonts under GPL-compatible licenses, listing each asset's origin and license in a sources file. Tests: the skin loads without warnings, geometry snapshots and golden renders at three sizes, and a coverage report mapping every control to an action. Show the owner the renders next to BlackSpace's; BlackSpace's renders stay local in ~/rvp-reference. Wait for the owner's approval of the skin. Done when: tests and CI are green, the leftovers above are done, the owner has approved the logo and the skin, RVP starts with the default skin without PotPlayer installed on the Ubuntu VM and the Windows VM, docs/goals/goal-i-status.md is written, the owner is reminded that M6's Ubuntu boot run is still open, and everything is committed and pushed. Do not: copy, trace or sample anything from PotPlayer's skins; commit PotPlayer or SVP files or renders made from them; change SVP or PotPlayer settings; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; make the repository public; tag or release. Stop on blockers or when an owner decision is needed.
```

Goal J:
```
/goal Execute goal J of docs/PLAN.md (milestone M8, platform integration and packaging) in ~/rvp, only if docs/goals/goal-i-status.md records goal I as done; otherwise stop and report. Follow AGENTS.md and the rules M2-M7 inherited (the goal status files), including the unsafe rule (FFI only in shell/, each with a SAFETY comment) and exact versions with a reason for every new dependency. Owner decisions (2026-10-16; the owner may change them before the run): the Windows installer is NSIS, built in WSL, a per-user installation that needs no administrator rights (%LOCALAPPDATA%\Programs\RVP); RVP registers for the video and audio types PotPlayer is associated with on the host (read the host registry read-only and keep the list in the repository; Linux gets the matching MIME types); RVP only registers itself as a handler (Windows: Capabilities, RegisteredApplications and OpenWithProgids under HKCU; Linux: the .desktop file's MimeType) and never makes itself the default or changes other programs' associations: the user picks it in the system's default apps. Linux: MPRIS (org.mpris.MediaPlayer2 and its Player interface: play, pause, stop, next, previous, seek, position, metadata, volume; GNOME's media controls on Wayland), idle and screensaver inhibit while video plays (the XDG desktop portal first, org.freedesktop.ScreenSaver as the fallback), a .desktop file with the hicolor icons drawn from RVP's logo, an AppImage, a .deb that installs on Ubuntu 22.04 (the VM) and 26.10 (the owner's boot), and an AUR PKGBUILD in the repository (not published; tested in an Arch container only if one runs without changing the system). Windows: System Media Transport Controls (media keys and the flyout with the title, play, pause, next, previous), the installer with its uninstaller, a portable ZIP (with rvp.portable), and crash logs: panics and native crashes write a log, on Windows with a minidump, to RVP's state folder. The packages carry RVP Default and RVP's icons from goal I; their version comes from the workspace manifest plus the commit (no tags). CI builds the packages for both systems as artifacts of the private repository. Tests: headless tests where the platform allows (the MPRIS interface on a private session bus, the association lists, the crash log writer); install, run (the self-test from the installed copy, and tools/windows-vm/m7-all.ps1 on the Windows VM) and uninstall on the Ubuntu VM (AppImage and .deb) and the Windows VM (installer and ZIP), checking that uninstalling leaves no files or registry keys besides the user's settings and state. Then prepare the owner's combined Ubuntu boot run: a kit step that installs the package on Ubuntu 26.10 and checks M8 there (MPRIS in GNOME, the screensaver kept off during playback, the associations), and a /goal text in PLAN.md section 10 for one boot session that runs M6's and M7's kit checks (kit.sh m6all or m7all) and M8's on the installed package. Done when: tests and CI are green, the packages install, run and uninstall cleanly on the Ubuntu VM and the Windows VM, docs/goals/goal-j-status.md is written, the owner is reminded of the combined Ubuntu boot run (M6's acceptance still needs it), and everything is committed and pushed. Do not: publish packages anywhere (no GitHub releases, no AUR upload), install anything on the Windows host or change its file associations without advance notice and explicit permission, change PotPlayer's or SVP's settings or associations, commit PotPlayer or SVP files, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Ubuntu boot run for M6, M7 and M8 (after goal J; M6's and M8's acceptance
need it; it can run whenever the owner boots Ubuntu, and if it has not run
by goal S, the run for M6 to M17 below takes its place):
```
/goal On the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 30-series card), run the Linux kit for milestones M6, M7 and M8 of the private repository woffko/rvp in one session and record the results; this is the Ubuntu boot run that M6's and M8's acceptance need (docs/goals/goal-h-status.md, docs/goals/goal-j-status.md). Clone or pull the repository to ~/rvp and follow AGENTS.md and tools/linux-kit/CHECKLIST.md sections 1, 2, 8 and 9. The owner does the privileged steps the checklist lists (the packages `kit.sh check` prints, `ydotoold` running as root for m2input, SVP 4 Manager started for m2svp and m6svp, `sudo apt install` of the .deb that step m8 names and `sudo apt remove rvp` after it); never use sudo yourself and never handle NAS credentials. Set RVP_SKIN to BlackSpace_en.dsf from the PotPlayer folder on the Windows partition, mounted read-only, and RVP_VPY to PotPlayer's VapourSynth/motioninterpolation.vpy there. Run `tools/linux-kit/kit.sh m8all` from a terminal inside the GNOME session; before each step that opens windows, tell the owner what will appear and for how long (nobody touches the mouse or keyboard during m2input). When m8 asks for the .deb, let the owner install it and run `kit.sh m8` again; after the owner removes it, run `kit.sh m8removed`. If a step fails, find the cause in its log in the kit's log directory; fix RVP only when the cause is clear and the fix small, run `cargo xtask ci` and rerun the step; otherwise record it as an open item. Then record the results: `kit.sh report` writes docs/test-runs/<date>-<host>.md; add an "Ubuntu boot run" section to docs/goals/goal-h-status.md (M6: the M6 checks, the picture upright, the script slots with SVP's VapourSynth, SVP's acceptance and the second window on /tmp/mpvSockets/<pid>, the GPU checks against goal D's 6 dropped frames per 20 s at 4K60, the frames dropped after the speed returns to 1x) and one to docs/goals/goal-j-status.md (M8: the AppImage and the .deb on 26.10, MPRIS in GNOME's media controls, the screen kept on during playback, the associations, the removal), with what the owner found in the manual checks of sections 8 and 9. Commit with `git -c user.name=woffko -c user.email=2505149+woffko@users.noreply.github.com commit` and push. Done when the report and the status sections are committed and pushed. Do not: change SVP or system settings beyond the checklist's steps, commit PotPlayer or SVP files, scripts or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal L:
```
/goal Execute goal L of docs/PLAN.md (R2 and milestone M10, PotPlayer parity: audio) in ~/rvp, only if docs/goals/goal-j-status.md records goal J as done; otherwise stop and report. Follow AGENTS.md, the rules M2-M8 inherited (the goal status files), docs/audits/audio.md and D14. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m10-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from the audio audit's build, fix what blocks daily use first. Then M8's Windows gap: keep the display on while video plays (SetThreadExecutionState), with Playback's "Prevent screen saver during playback". Then R2 as PLAN.md section 6 describes it: PotPlayer's Preferences tree and command-line switches, read read-only from its installed files into docs/r1/preferences.tsv and docs/r1/command-line.tsv, and a test that writes docs/coverage/preferences.md; the menus' coverage also counts the 720 commands of docs/r1/menu-runtime.tsv that the static tree lacks. Then M10 as section 6 lists it: output (devices, exclusive mode, pass-through, format and resampling, the remaining speaker layouts, Dolby Surround and Pro Logic II, virtual surround), the second audio device, channels, the system's volume, individual volume, preamp, PotPlayer's 19 equalizer presets with their values, the audio sync dialog and keeping sync changes, the effects in one defined chain order, the time-stretching modes, the normalizer's block per source and its remaining options, and audio recording. Coverage: every audio item of R1's menus and hotkeys and R2's audio pages either works (an ACTIONS row with PotPlayer's IDs and default keys; a setting with its check in Settings::validate, a page in shell/qml/Settings.qml and apply_settings when mpv takes it at run time) or keeps a reason in crates/app/src/coverage.rs that says why it cannot; rewrite docs/coverage/ (RVP_BLESS=1). Tests: headless chain tests (labels, order, toggles during playback, the end-of-file rule), level tests on synthetic tones as in crates/app/tests/speakers.rs, the presets, the recording, rvp --selftest additions, m10-all.ps1 on the Windows VM. Then one host session, with advance notice and permission, comparing RVP with PotPlayer on the audit's tones for the new effects, presets and block sizes (host-session audio; save and restore PotPlayer's state as the audit did). Done when: tests and CI are green, the comparison is in docs/audits/audio.md, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-l on the host (file copies only), docs/goals/goal-l-status.md lists results, differences from PotPlayer, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files, recordings or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal M:
```
/goal Execute goal M of docs/PLAN.md (milestone M11, PotPlayer parity: playback, playlist and opening) in ~/rvp, only if docs/goals/goal-l-status.md records goal L as done; otherwise stop and report. Follow AGENTS.md (its album rules: mpv plays one entry at a time), the rules M2-M10 inherited (the goal status files) and D14. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m11-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from goal L's build, fix what blocks daily use first. Deliver M11 as PLAN.md section 6 lists it, starting with the thumbnail core (a second libmpv core without audio, the software render API, a thread of its own, a cache): seek bar previews, time tooltips and markers; the jump dialog, keyframe steps and scene browsers; A-B lists and fine moves, skips, shuffle and bookmarks; the actions when playback finishes, with shut down, sleep and hibernate behind a cancellable countdown and the shutdown timer; the Recycle Bin for played files, with confirmation; opening external audio, DVD and Blu-ray titles (their menus are goal R's), audio CDs, devices, the screen, network addresses, streaming sites through the user's yt-dlp, the clipboard, recent items and favorites, with the open actions and "When Files Opened" rules; the playlist panel's queue, sorting, search, editing, copy and paste, file information, playlist files in and out, background durations and thumbnail views; PotPlayer's command-line switches that apply (R2). Study seamless playback and the file navigator, record the decision, and add them only if they keep the album rules and the GUI thread free. Coverage: every Playback, Open, Album/Favorites, DVD Control and playlist-panel item of R1's menus and hotkeys and R2's matching pages either works (an ACTIONS row with PotPlayer's IDs and default keys; a setting with its check in Settings::validate, a page in shell/qml/Settings.qml and apply_settings when mpv takes it at run time) or keeps a reason in crates/app/src/coverage.rs that says why it cannot; rewrite docs/coverage/ (RVP_BLESS=1). Tests: headless tests on synthetic media (chapters, keyframes, subtitles, A-B lists, skips, the queue, sorting, playlist files, a DVD folder made with free tools), the thumbnail core (sizes, timing, cancellation, never on the GUI thread), the power actions through a dry-run hook, the command line; rvp --selftest additions; m11-all.ps1 on the Windows VM. Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-m on the host (file copies only), docs/goals/goal-m-status.md lists results, differences from PotPlayer, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: shut down, suspend or hibernate the host or a VM (tests use the dry-run hook); run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal N:
```
/goal Execute goal N of docs/PLAN.md (milestone M12, PotPlayer parity: video, picture and window) in ~/rvp, only if docs/goals/goal-m-status.md records goal M as done; otherwise stop and report. Follow AGENTS.md (video filters: only RVP's own, by label; SVP's @svp is never touched), the rules M2-M11 inherited (the goal status files) and D14. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m12-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from goal M's build, fix what blocks daily use first. Deliver M12 as PLAN.md section 6 lists it: frame, aspect and crop (custom ratios, frame sizes, the output's aspect correction, Extend/Crop with moves, bottom and screen margins, rotation by aspect ratio); the window (sizes and presets, moves, the size dialog, on top in fullscreen; Desktop Mode as a study); the remaining video effects as labelled filters; 3D video through FFmpeg's stereo3d (3D subtitles and H.264 MVC as studies); 360° video (v360); PotPlayer's HLSL pixel shaders translated into mpv's GLSL hooks, with the pre- and post-resize lists; AviSynth with the user's AviSynth+ (opening .avs scripts, PotPlayer's AviSynth processing through a VapourSynth bridge a study picks, AviSynth plugins in VapourSynth scripts); a decoders page in place of DirectShow's filter priorities; rendering choices (gpu-next or gpu, scalers and when to resize, HDR tone mapping, the display's ICC profile) and, after a study against S3's measurements and SVP's copy-back rule, D4's child-window Direct3D 11 path on Windows as a renderer choice; capture (the clipboard, Save As, BMP, file names, consecutive images, thumbnail images from M11's thumbnail core); Record Stream (stream-record), and Record Video only if a study shows that libmpv can encode in a second core; visualizations and the slideshow for audio files. Coverage: every Video, Frame Size, Aspect Ratio, Window Size and Visualization item of R1's menus and hotkeys and R2's matching pages either works (an ACTIONS row with PotPlayer's IDs and default keys; a setting with its check in Settings::validate, a page in shell/qml/Settings.qml and apply_settings when mpv takes it at run time) or keeps a reason in crates/app/src/coverage.rs that says why it cannot; rewrite docs/coverage/ (RVP_BLESS=1). Tests: headless property and filter tests (labels, order, @svp untouched, toggles during playback), the 3D modes on a synthetic side-by-side clip (pixel checks), capture formats and names, the contact sheet's geometry, window commands in rvp --selftest; m12-all.ps1 on the Windows VM; one host session, with advance notice and permission, for the renderer choice, HDR and SVP with the new filters. Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-n on the host (file copies only), docs/goals/goal-n-status.md lists results, differences from PotPlayer, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal T (after goal N; added 2026-10-18):
```
/goal Execute goal T of docs/PLAN.md (milestone M18, motion smoothing without SVP) in ~/rvp, only if docs/goals/goal-n-status.md records goal N as done; otherwise stop and report. Follow AGENTS.md (video filters: only RVP's own, by label; SVP's @svp is never touched), the rules M2-M12 inherited (the goal status files), D14 and D16. Windows first: build and check on the Windows VM <windows-vm> with a new tools/windows-vm/m18-all.ps1, and on the host only with advance notice and the owner's explicit permission for each run; keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S. Start with M18's study: mpv's interpolation (display-resample, tscale), FFmpeg's minterpolate, MVTools in VapourSynth, RIFE in VapourSynth (ncnn-Vulkan or TensorRT) and NVIDIA's Optical Flow FRUC if its license allows shipping it. Measure each on the Windows VM and, in one host session with permission, on the RTX 30-series card at 720p, 1080p and 4K: the rate reached, CPU and GPU load, drops, artefacts on synthetic clips and on clips the owner names (kept local). Write docs/spikes/m18-smoothing.md with the results, how RVP gets each engine onto the user's machine (files bundled in the packages under GPL-compatible licenses, or the user's own install) and a recommendation; show it to the owner and wait for the choice of engines and defaults. Then deliver M18: the modes Off, Blend and the chosen motion-compensated modes with quality presets and a target rate, as RVP's labelled filter after its other filters or mpv's interpolation options, off while SVP is attached, with copy-back decoding and the user's size and rate limits; Video menu entries with a hotkey, a page in the settings window's Video group, the rate reached in the status. Tests: frame counts and timestamps of a smoothed synthetic clip of a moving square, pixel checks of an interpolated frame, toggles during playback, @svp untouched; rvp --selftest additions; m18-all.ps1 on the Windows VM; one host session with permission for the GPU modes. Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-t on the host (file copies only), docs/goals/goal-t-status.md lists results, differences from SVP, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: use SVP's files or libraries; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files, third-party binaries or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal T, resumed after the owner's pause (2026-10-19; the study had lost
the host's GPU, and D17 came in meanwhile):
```
/goal Resume goal T of docs/PLAN.md (milestone M18, motion smoothing without SVP) in ~/rvp from where it paused on 2026-10-19: read docs/goals/goal-t-status.md first. Follow AGENTS.md (video filters: only RVP's own, by label; SVP's @svp is never touched; GPU runs on the host only at gpu-limit.ps1's power limit), the rules M2-M12 inherited (the goal status files), D14, D16 and D17. First commit D17's GPU power budget, which is in the working tree, on its own: cargo xtask ci, rvp --selftest=gpu on the Windows VM <windows-vm>, then push and check that CI is green. In the first permitted host session, also compare Windows' adapter counter with NVML on the RTX 30-series card (RVP or the shell's gpu_power test with RVP_GPU_POWER_ADAPTERS=all, the card idle and under load) and record it in goal-t-status.md: the counter is AMD's only reading on Windows. Then finish M18's study: part C, the owner's clips (Apex 1080p at 45:00, 28 Years Later 720p at 15:00) through MVTools, RIFE and fruc_vulkan with grabs, in one host session with advance notice and the owner's explicit permission; every GPU run goes through tools/windows-vm/m18-study.ps1 -Gpu -PowerLimit 90 (one UAC prompt the owner answers; nvidia-smi before and after each run) and no RIFE at 2160p; on a lost GPU, a missing Vulkan device or a power limit that went back up, stop all host runs and tell the owner at once. Write docs/spikes/m18-smoothing.md from goal-t-status.md's numbers and part C: what each engine reaches, how RVP gets it onto the user's machine (files bundled in the packages under GPL-compatible licenses, or the user's own install), the RIFE exit crash and the DLL search path findings, and a recommendation; show it to the owner and wait for the choice of engines and defaults. Then deliver M18 as PLAN.md lists it: the modes Off, Blend and the chosen motion-compensated modes with quality presets and a target rate, as RVP's labelled filter after its other filters or mpv's interpolation options, off while SVP is attached, with copy-back decoding and the user's size and rate limits; the smoothing modes as steps of the GPU power budget (a lighter preset first, then Blend or off); RVP's own VapourSynth runtime, never SVP's, and libraries loaded from RVP's own folder only; Video menu entries with a hotkey, a page in the settings window's Video group, the rate reached in the status. Tests: frame counts and timestamps of a smoothed synthetic clip of a moving square, pixel checks of an interpolated frame, toggles during playback, @svp untouched, the budget's smoothing steps in rvp --selftest=gpu, rvp --selftest additions, a new tools/windows-vm/m18-all.ps1 on the Windows VM; one host session with permission for the GPU modes at the power limit. Keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S. Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-t on the host (file copies only), docs/goals/goal-t-status.md lists results, differences from SVP, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: use SVP's files or libraries; run anything on the Windows host desktop, including GUI programs from WSL and gpu-limit.ps1's UAC prompt, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host, except gpu-limit.ps1's power limit during permitted GPU runs; commit PotPlayer or SVP files, third-party binaries or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal O:
```
/goal Execute goal O of docs/PLAN.md (milestone M13, PotPlayer parity: subtitles) in ~/rvp, only if docs/goals/goal-n-status.md records goal N as done; otherwise stop and report. Follow AGENTS.md, the rules M2-M12 inherited (the goal status files) and D14. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m13-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from goal N's build, fix what blocks daily use first. Owner decision (2026-10-16): subtitles are added by hand for now; no online search or translation service and no AngelScript host in this goal (M13's online part stays deferred). Deliver the rest of M13 as PLAN.md section 6 lists it: a second subtitle at the same time and Combine Subtitle, reloading, copying the current subtitle; style (bold, spacing, the opaque box, the font style page, sideways moves and fades where libass allows); placement of bitmap and text subtitles with M12's margins; the sync dialog, keeping sync changes and FPS conversion; the language, encoding and loading options; saving external text subtitles with the sync applied, the subtitle browser and editor and Input Subtitles (RVP's own reader and writer for SRT, ASS and WebVTT); PotPlayer's Create subtitles from audio, ahead of time and in real time, offline with whisper.cpp (CPU, Vulkan or CUDA; models the user downloads; a study decides on PotPlayer's other engines); 3D subtitles if goal N's study found a way. Coverage: every Subtitles item of R1's menus (static and run-time) and hotkeys and R2's subtitle pages either works (an ACTIONS row with PotPlayer's IDs and default keys; a setting with its check in Settings::validate, a page in shell/qml/Settings.qml and apply_settings when mpv takes it at run time) or keeps a reason in crates/app/src/coverage.rs that says why it cannot (the online items: deferred by the owner); rewrite docs/coverage/ (RVP_BLESS=1). Tests, with no online service: reader and writer round trips on subtitle files the tests generate and on free samples kept in ~/rvp-reference, the editor's model, the style and placement properties on a headless player, generated subtitles on a synthetic speech clip (a free speech synthesizer such as espeak-ng) with the tiny model, skipped where it is absent, rvp --selftest additions, m13-all.ps1 on the Windows VM. Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-o on the host (file copies only), docs/goals/goal-o-status.md lists results, differences from PotPlayer, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: send the owner's files or subtitles to any online service; commit Whisper models or third-party subtitle files; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal P:
```
/goal Execute goal P of docs/PLAN.md (milestone M14, PotPlayer parity: preferences, skins and the rest) in ~/rvp, only if docs/goals/goal-o-status.md records goal O as done; otherwise stop and report. Follow AGENTS.md (the skin engine's notes; M7's originality rules for anything RVP draws), the rules M2-M13 inherited (the goal status files) and D14. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m14-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from goal O's build, fix what blocks daily use first. Deliver M14 as PLAN.md section 6 lists it: every remaining option of docs/r1/preferences.tsv as a setting or a reason, and the settings window grown into PotPlayer's tree where that helps (search, reset per page, RVP's own look and sliders); settings profiles (Manage Presets); PotPlayer's OSD options and the on-screen info items; the Playback/System Info window (Ctrl+F1); the Skins menu (the skin list and switching while running, colour themes and their dialog, opacity, logo settings, the on-screen control mode MainSkinAreaOscInside, the audio skin mode AudioSkin.xml, auto hiding, the skin mode options, the window per skin mode, skin settings), checked with RVP Default and BlackSpace and with other .dsf skins the owner names (kept local); copying names and paths, the history and Windows' jump list, Disable default hotkeys, the context menu's style, interface languages (Qt's translations: English and Russian, written for RVP, never taken from PotPlayer's language files) and touch gestures. Coverage: every remaining item of R1's menus and hotkeys and every option of R2 either works (an ACTIONS row with PotPlayer's IDs and default keys; a setting with its check in Settings::validate, a page in shell/qml/Settings.qml and apply_settings when mpv takes it at run time) or keeps a reason in crates/app/src/coverage.rs that says why libmpv, FFmpeg, Qt or the platform cannot do it; no "not in RVP yet" reason remains for Windows; rewrite docs/coverage/ (RVP_BLESS=1). Tests: the preferences coverage report, profiles round trip, skin switching and colour themes on the synthetic skin and RVP Default (golden renders; BlackSpace's stay local), translations load and fit their controls, rvp --selftest additions, m14-all.ps1 on the Windows VM; one host session, with advance notice and permission, showing RVP's and PotPlayer's skin modes side by side (captures stay in ~/rvp-reference). Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-p on the host (file copies only), docs/goals/goal-p-status.md lists results, differences from PotPlayer, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: copy text, art or settings from PotPlayer's files into RVP; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal Q:
```
/goal Execute goal Q of docs/PLAN.md (milestone M15, remote control: RVP's control API and a Unified Remote remote) in ~/rvp, only if docs/goals/goal-p-status.md records goal P as done; otherwise stop and report. Follow AGENTS.md (the GUI thread stays free: the server's thread never calls mpv), the rules M2-M14 inherited (the goal status files), D14 and D15, and exact versions with a reason for any new dependency. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m15-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from goal P's build, fix what blocks daily use first. Deliver M15 as PLAN.md section 6 lists it. The API: a small HTTP/1.1 JSON service, off by default and turned on from a Remote settings page that shows its address and token; 127.0.0.1 only (owner, 2026-10-16); a token per installation on every request; requests with a browser's Origin header refused; limits on sizes and times; status, the action list, any action of ACTIONS through the usual dispatch, seek, volume, open, albums and entries, and a long poll for changes; commands reach the window's thread as MPRIS's and SMTC's do, and the status is a snapshot the window's thread keeps. The remote: remote/unified-remote/RVP with meta.prop, layout.xml, remote.lua (Unified Remote's http library), settings.prop (port and token) and an icon that cargo xtask builds from RVP's logo; a layout in the manner of Unified Remote's media remotes (transport, a seek slider, a volume slider and mute, fullscreen, tracks, speed, the normalizer, the title and times, an albums page). Delivery: the installer offers to copy the remote into C:\ProgramData\Unified Remote\Remotes\Custom when Unified Remote Server is installed, and removes what it copied on uninstall; the portable ZIP carries the folder. Study a subset of MPC-HC's web interface and add it only if it is small. Tests: the API on a headless player (every endpoint, the token, the Origin refusal, the limits, nothing blocking the GUI thread), every action the remote calls exists, layout.xml is well formed; on the Windows VM, Unified Remote Server (installed on the VM only) loads the remote and its calls reach RVP; m15-all.ps1. Then, with advance notice and the owner's explicit permission, a host session where the owner tries the remote from a phone. Done when: tests and CI are green, the owner has tried the remote (or chose not to), the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-q on the host (file copies only), docs/goals/goal-q-status.md lists results, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: open the API beyond 127.0.0.1; put tokens into the repository or logs; install Unified Remote or anything else on the host without permission; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal R:
```
/goal Execute goal R of docs/PLAN.md (milestone M16, DVD and Blu-ray menus) in ~/rvp, only if docs/goals/goal-q-status.md records goal Q as done; otherwise stop and report. Follow AGENTS.md (the unsafe rule: FFI only in shell/, each with a SAFETY comment), the rules M2-M15 inherited (the goal status files) and D14. Windows first: build and check each part on Windows (the Windows VM <windows-vm> with a new tools/windows-vm/m16-all.ps1; the host only with advance notice and the owner's explicit permission for each run); keep Linux building with its headless tests and CI green, and list Linux-only parts for goal S instead of doing them. If the owner reported findings from goal Q's build, fix what blocks daily use first. Start with M16's study: how VLC (modules/access/dvdnav.c), Kodi (its DVD navigator and Blu-ray input stream) and mpv's removed discnav code handle navigation, still frames, highlights and stream changes; whether RVP feeds mpv through a custom stream (libmpv's stream_cb, wrapped by vendor/libmpv2's protocol module) or reopens it per cell; how the highlights are drawn. Record the design in docs/spikes/s7-disc-menus.md, then ask the owner to approve libdvdnav and libdvdread (GPL-2.0-or-later) and libbluray (LGPL-2.1-or-later) under D8, which waived the no-GPL rule only for libmpv and Qt. Then deliver M16 as PLAN.md section 6 lists it: DVD menus with button highlights, the mouse and keys, DVD Control's Title, Root, Chapter, Audio, Subtitle and Angle menus and Previous Menu, still frames, and the machine's audio and subtitle choices; Blu-ray HDMV menus through libbluray's overlays; BD-J menus where libbluray's Java part runs with the user's Java; encrypted discs only with the user's own libdvdcss, libaacs and keys. Coverage: DVD Control's items in docs/coverage/ either work or keep a reason that says why they cannot; rewrite docs/coverage/ (RVP_BLESS=1). Tests: a DVD with menus authored in WSL with free tools (dvdauthor, spumux; synthetic video and buttons) and headless navigation tests on it (moving between buttons, activating one, stills, starting a title, returning to the menu); Blu-ray HDMV menus on a free sample if one exists, otherwise on the owner's discs on the host, with advance notice and permission; rvp --selftest additions; m16-all.ps1 on the Windows VM. Done when: tests and CI are green, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-r on the host (file copies only), docs/goals/goal-r-status.md lists results, differences from PotPlayer, the Linux items for goal S and what the owner should try, and everything is committed and pushed. Do not: ship or download libdvdcss, libaacs or decryption keys; commit disc images other than the synthetic DVD's sources and build script; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; change PotPlayer's, SVP's or the system's settings on the host; commit PotPlayer or SVP files or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal K (after goal R; the owner may postpone it):
```
/goal Execute goal K of docs/PLAN.md (milestone M9, other players' skins) in ~/rvp, only if docs/goals/goal-r-status.md records goal R as done; otherwise stop and report. Follow AGENTS.md, the rules M2-M16 inherited (the goal status files) and D14. Start with the study. Ask the owner which VLC skins, mpv on-screen controllers and classic Winamp skins to try; without an answer, take the most downloaded skins from VLC's skins page plus uosc, ModernX and mpv's built-in OSC, and a few popular classic Winamp skins (.wsz). Keep the downloads local in ~/rvp-reference. Record in docs/spikes/s6-other-skins.md what each format needs, what RVP lacks and the cost of each part, then stop and ask the owner to choose the scope. Then implement the chosen parts, Windows first: check them on the Windows VM (<windows-vm>) and list their Linux side for goal S. For VLC .vlt skins: a second loader in crates/skin that builds the same display list, VLC's actions mapped to RVP's, and a coverage report. For mpv's on-screen controllers: libmpv with Lua (the kit's Linux build is goal S's), an RVP mode where mpv's controller replaces the skin (borderless window, pointer and keys passed to mpv), and the user's scripts folder. For classic Winamp skins: a loader that turns their fixed layout into the same display list, with the video placed as the study decided. Done when: tests and CI are green, the chosen parts work on the Windows VM, docs/goals/goal-k-status.md lists results and the Linux items for goal S, and everything is committed and pushed. Do not: commit third-party skins or scripts or renders made from them, run anything on the Windows host without advance notice and explicit permission, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Goal S:
```
/goal Execute goal S of docs/PLAN.md (milestone M17, Linux completion) in ~/rvp, only if docs/goals/goal-k-status.md records goal K as done; otherwise stop and report. Follow AGENTS.md (test environments: GUI programs started in WSL count as host runs; silent tests need no permission), the rules M2-M18 inherited (the goal status files), D14 and D19 (both systems: every change of goal S is built and checked on Windows and Linux, and keeps Windows working). Collect the Linux items that goals L to R, T and K listed in their status files ("Linux items for goal S") and deliver M17 as PLAN.md section 6 lists it: the system's volume through PipeWire or PulseAudio, recording RVP's own PipeWire stream, the second audio device, devices through v4l2, the screen as a source (a study: Wayland needs the screencast portal), the trash, power actions through logind (dry-run tests only), DVD, Blu-ray and CD support, M16's disc menus and the chosen audio filters' libraries (libdvdnav, libbluray, libcdio, libmysofa, librubberband, libbs2b) in the kit's libmpv, Lua with mpv's built-in scripts for M9's on-screen controllers, M9's skin windows of their own (classic Winamp's and VLC's: docked, moved together and shaped) on Wayland, which lets no client place its windows (a way that works, or the reason listed), M14's tray icon, screen lock, text to speech, Xbox controller, file types through xdg-mime, taskbar progress and recent files, M18's VapourSynth runtime in the packages, NVIDIA Optical Flow in the kit's libmpv and waiting for SVP for Linux, AviSynth+ for Linux if the user installs it, whisper.cpp's GPU builds, the window commands Wayland refuses listed with their reason, Unified Remote Server for Linux with the remote, the translations, and the packages (AppImage, .deb, PKGBUILD) carrying the new parts. Run it all on the Ubuntu VM (<ubuntu-vm>, tools/linux-kit; access as in ~/another project/AGENTS.md): rvp --selftest's new sections (M9's =m9, =classic and =vlc with local sample skins and scripts, never committed), new kit steps m9 to m16 and m18 and m17all for all of them, the packages installed, run and removed; on Windows, the VM's checks of the areas goal S changes as it goes and the whole set (tools/windows-vm m7-all, m9-all to m16-all, m18-all) before the end; add CHECKLIST.md sections for the owner's Ubuntu boot, with the checks only real hardware can do (GPU, SVP, Whisper on CUDA, a DVD drive or a webcam if the owner has them). Then bring the /goal text "Ubuntu boot run for M6 to M17" in PLAN.md section 10 in line with the kit's real steps. Coverage: no "not in RVP yet" reason remains for Linux in docs/coverage/; reasons that only Linux has (Wayland's limits) say so. Done when: tests and CI are green, the kit passes on the Ubuntu VM and the Windows VM's checks pass, the Windows build is in a new portable folder C:\Users\<user>\rvp-test\rvp-s on the host (file copies only), docs/goals/goal-s-status.md lists results, what only the Ubuntu boot can check and what the owner should try, the owner is reminded of the boot run, and everything is committed and pushed. Do not: shut down, suspend or hibernate a VM (tests use the dry-run hook); use sudo on the Ubuntu VM beyond what tools/linux-kit documents; run anything on the Windows host desktop, including GUI programs from WSL, without advance notice and explicit permission; commit PotPlayer or SVP files or captures; make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

Ubuntu boot run for M6 to M18 (after goal S; the acceptance of M6, M8 and
M17 needs it; goal S brought it in line with the kit's steps):
```
/goal On the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 30-series card), run the Linux kit for milestones M6 to M18 of the private repository woffko/rvp in one session and record the results; this is the Ubuntu boot run that the acceptance of M6, M8 and M17 needs (docs/goals/goal-h-status.md, goal-j-status.md, goal-s-status.md), with M9's and M18's Linux side. Clone or pull the repository to ~/rvp and follow AGENTS.md (its Linux section) and tools/linux-kit/CHECKLIST.md, sections 1 to 10. The owner does the privileged steps the checklist lists (the packages `kit.sh check` prints, `ydotoold` running as root for m2input, SVP 4 Manager started for m2svp and m6svp, installing the .deb that step m8 names and removing it afterwards); never use sudo yourself and never handle NAS credentials. Set RVP_SKIN to BlackSpace_en.dsf from the PotPlayer folder on the Windows partition, mounted read-only, and RVP_VPY to PotPlayer's VapourSynth/motioninterpolation.vpy there; set the optional pieces section 10 names where the owner has them (RVP_M9_SCRIPTS, RVP_WINAMP_SKINS and RVP_VLC_SKINS with local samples, never committed; RVP_WHISPER_MODELS with ggml-tiny.bin; RVP_URSERVER with Unified Remote Server for Linux unpacked, never installed). Run `tools/linux-kit/kit.sh m17all` from a terminal inside the GNOME session (it builds libmpv, whisper.cpp's engine and RVP's VapourSynth runtime in ~/.cache/rvp-kit first); before each step that opens windows or plays sound, tell the owner what will appear and for how long (nobody touches the mouse or keyboard during m2input). Then the checklist's manual checks with the owner: section 10's (the tray with GNOME's AppIndicator extension, the screen lock, the dock's progress, recent files, text to speech, a gamepad if the owner has one, the trash, file types, the Remote Control page's Add to Unified Remote, a webcam and a DVD drive if the owner has them, the screen capture refused on Wayland) and the GPU's (the Vulkan renderer through XWayland with NVIDIA Optical Flow and RIFE at gpu-limit's power limit, the GPU power budget, Whisper on Vulkan and, with the owner's own build, CUDA, SVP's acceptance). If a step fails, find the cause in its log in the kit's log directory; fix RVP only when the cause is clear and the fix small, run `cargo xtask ci` and rerun the step; otherwise record it as an open item. Then record the results: `kit.sh report` writes docs/test-runs/<date>-<host>.md; add an "Ubuntu boot run" section to docs/goals/goal-s-status.md with each step's result (the GPU checks against goal D's 6 dropped frames per 20 s at 4K60, SVP's acceptance, the packages, MPRIS and the screen kept on, the new audio, video, subtitle and playlist functions, the disc menus, the remote, M9's skins and controllers, M18's smoothing) and, if their run has not happened, the M6 and M8 sections that the M6-M8 run would have written into goal-h-status.md and goal-j-status.md, with what the owner found in the checklist's manual checks. Commit with `git -c user.name=woffko -c user.email=2505149+woffko@users.noreply.github.com commit` and push. Done when the report and the status sections are committed and pushed. Do not: change SVP or system settings beyond the checklist's steps, commit PotPlayer or SVP files, third-party skins, scripts or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```
