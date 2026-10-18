# Goal N status: milestone M12 (PotPlayer parity: video, picture and window)

Date: 2026-10-18. Result: **done**. Precondition met: `goal-m-status.md`
records goal M as done. The owner reported no findings from goal M's
build; during the goal they asked for PotPlayer's settings tree with the
group's pages as tabs (moved here from M14), a default shader set in the
build, and motion smoothing without SVP, which became D16, M18 and goal T
in `docs/PLAN.md`. Everything was built and checked on Windows first (the
test VM); the host ran one session, with the owner's permission, for the
renderer choice, HDR and SVP with the new filters.

## Summary

| Part | Result | Where |
|------|--------|-------|
| Frame and aspect | Custom aspect ratios and their dialog, the frame at 0.5x to 2x, the output device's aspect (by the screen's measured size or a ratio), adjusting the aspect to the window, Extend/Crop presets by ratio and by pixel with moves, Extend/Crop before the resize or scaling only the height, bottom and screen margins (the subtitles' room), rotation by the screen's aspect ratio, Frame Size's presets; the aspect mode and ratio are kept for the next run | `crates/app/src/picture.rs` |
| Video > Resize | When to resize (always, smaller, larger than a size or the display), the target (the display, a factor, a size), the scaler and its parameter, keeping the aspect, interlaced sources | `picture.rs` (`resize_graph`) |
| The window | The Window Size menu's sizes (0.5x to 2x, 30/45/60/75 % of the desktop, minimum, maximum, custom), moves to the edges and corners, the size dialog and Window Size Control, larger and smaller, staying on top in fullscreen, fullscreen on another display with the others black, the pointer kept on the fullscreen display (Windows) | `crates/app/src/window.rs`, `shell/qml/Main.qml` |
| Video effects | The rest of PotPlayer's effects as RVP's labelled filters before `@svp`: rotation by any angle, auto level, level control, gradual and temporal denoise, luma and chroma offset, motion blur, WarpSharp | `crates/app/src/video.rs` |
| 3D and 360° | 3D video through FFmpeg's `stereo3d` (input layout, output mode, swapping the eyes, a menu of PotPlayer's modes) and 360° video through `v360`, which the mouse turns | `crates/app/src/stereo.rs` |
| Pixel shaders | PotPlayer's pre-resize and post-resize lists over mpv's `glsl-shaders`; PotPlayer's MPC-HC HLSL shaders translated into mpv hooks when they load (34 of 34 installed ones compile); a default set of eleven mpv shaders in the build (FSR, CAS, NVScaler, NVSharpen, FSRCNNX, RAVU, KrigBilateral, SSim, adaptive-sharpen) with their licences | `crates/app/src/hlsl.rs`, `crates/app/src/shaders.rs`, `crates/app/shaders/` |
| AviSynth | `.avs` files open where FFmpeg has its AviSynth demuxer (Windows' libmpv has it) and the user's AviSynth+ is installed, AviSynth plugins load in VapourSynth scripts; PotPlayer's AviSynth processing of the playing video stays out (the study) | `crates/app/src/video.rs`, `docs/spikes/m12-studies.md` |
| Decoders | A page in place of DirectShow's filter priorities: hardware decoding per codec (`hwdec-codecs`), the AV1 software decoder, the decoders' threads | `crates/app/src/decoders.rs` |
| Rendering | The downscaler and chroma scaler, HDR tone mapping and its curve, the mastering metadata instead of measured peaks, the display's luminance and ICC profile, limited range, a forced YCbCr matrix, RVP's own software deinterlacer with the field order; the Pixel Shaders menu's colour items | `crates/app/src/render.rs` |
| Direct3D 11 renderer (D4) | The renderer is a choice taken at start: OpenGL in the window (the default) or, on Windows, mpv's `gpu-next` with Direct3D 11 in a native window of its own, with HDR output, a 10-bit swapchain, NVIDIA's and Intel's video super resolution and NVIDIA's RTX Video HDR (mpv's `d3d11vpp`) | `shell/cpp/video_window.cpp`, `render.rs`, the study |
| Capture | The source or the screen frame to the clipboard, Save As, BMP (written from `screenshot-raw`) and WEBP, file names with the date and time or the elapsed time, subtitles in source frames, aspect correction, storing next to the media, consecutive images, the thumbnail image (a contact sheet from M11's thumbnail core, laid out by RVP and written by Qt off the window's thread) | `crates/app/src/capture.rs`, `shell/src/player/m12.rs` |
| Recording | Record Stream (`stream-record`, the Tools menu's too) and Record Video, which a second core encodes with RVP's filters (the study) | `crates/app/src/record.rs`, `crates/engine/src/encode.rs` |
| Visualization | For audio files: the album art toggle, FFmpeg's `showcqt`, `showspectrum`, `avectorscope` and `showwaves` through `lavfi-complex`, a slideshow of the folder's pictures with previous and next | `crates/app/src/visual.rs`, `shell/qml/Slideshow.qml` |
| Settings window | PotPlayer's tree: groups open into their pages, and the selected group's pages show as tabs | `shell/qml/Settings.qml` |
| Coverage | The context menu's Video (146 items carried out, 58 with reasons), Aspect Ratio (29), Frame Size (23), Window Size (27, one with a reason) and Visualization (6, 4 with reasons) items, the live menu's (37 carried out, 2 lists), and every option of R2's Video pages are settings or keep a reason | `crates/app/src/coverage.rs`, `docs/coverage/` |

## The studies

`docs/spikes/m12-studies.md` records each:

- **Record Video** goes in: a second core in mpv's encoding mode
  re-encodes the recorded stretch of the source with RVP's filters (H.264
  and AAC, or VP9 and Opus for WebM). It is the source again, not the
  screen: no renderer colour controls, shaders or SVP frames, and not for
  live streams (Record Stream records those as they come).
- **PotPlayer's pixel shaders** run through a translation of their HLSL
  into mpv's hooks; glslang and SPIRV-Cross were not needed.
- **3D subtitles** stay out: mpv draws subtitles once over the finished
  picture; FFmpeg's `subtitles` filter would draw them per eye but only
  for text subtitles and with a filter rebuild for every change.
- **H.264 MVC** stays out: FFmpeg decodes the base view only.
- **Desktop Mode** stays out: it relies on Explorer's undocumented desktop
  layer (changed in Windows 11 24H2) and leaves the window no input.
- **AviSynth processing of the playing video** stays out: the only bridge
  (avsw) runs a 32-bit AviSynth in a second process for 8-bit video;
  VapourSynth scripts with AviSynth plugins take its place.
- **D4's Direct3D 11 output** goes in as a renderer choice: S3 measured
  no dropped frames for it, SVP's copy-back rule holds in both renderers,
  and it alone gives HDR output, `gpu-next` and the GPU's video processor.

## Tests

- `crates/app/tests/m12.rs` (14 tests, headless libmpv, the 12 s fixtures
  of `tools/test-clips/make-m12-fixtures.sh`): aspect modes, Extend/Crop
  and the margins before SVP's filter (whose entry stays where it is),
  the screen margins and the subtitles' room, rotation and frame presets,
  every effect running in mpv's FFmpeg, the 3D outputs' pixels on a
  side-by-side clip (each eye where it belongs), the 360° view turning
  while it plays, capture formats and names, recordings (the stream and
  a video's length and size), visualizations and the slideshow, rendering
  and decoder options, video filter changes waiting at the end of a file,
  what FFmpeg offers (demuxers, encoders), and on Windows the Direct3D 11
  video processor's options and copy-back rule.
- Unit tests for the pieces: the HLSL translation (each rewrite), the
  shader catalogue, Resize's and Extend/Crop's graphs, the window's
  sizes, the contact sheet's geometry, capture names, the renderer's and
  decoders' options, `Chrome::bars`.
- `tests/bindings.rs`, `tests/menus.rs` and `tests/coverage.rs` cover the
  new rows (every M12 key runs its action; the live menu's items).
- `rvp --selftest=m12` (27 checks, `shell/qml/SelfTestM12.qml`): the
  window's sizes, the size dialogs, moves, fullscreen on another display
  and on top, capture (BMP, the screen frame, the clipboard, consecutive
  images, the thumbnail image) and the settings window's tree and tabs.
- `rvp --selftest=shaders`: every shader of the catalogue compiled by mpv
  (45 of 45 under Linux's Mesa, PotPlayer's read in place).
- `rvp --selftest=direct3d11` (11 checks): the Direct3D 11 renderer.
- `rvp --selftest=svp` checks M12's crop and two effects with SVP too.

## Checks on the Windows VM

`tools/windows-vm/m12-all.ps1` (new) on the test VM: the M12 self-test,
the shaders, the Direct3D 11 renderer and the headless tests on Windows'
libmpv. The final runs pass: self-test failures 0; the shaders SKIPPED
(the VM's OpenGL is Mesa 11's 3.0, older than mpv's hooks need); the
Direct3D 11 window placed, disabled and cut as expected, its picture
SKIPPED (session 0 makes no swapchain); `tests/m12.rs` and
`tests/headless.rs` pass, the recording 62,159 bytes. `m7-all.ps1` passes
(one timing failure of goal L's Shift+J check in one of two runs).
Details: `docs/test-runs/2026-10-18-m12-runs.md`.

## The host session

With the owner's permission (eight runs, then two reruns), on the RTX 30-series card at 2560x1440 and 59.951 Hz, Windows HDR off:

- 4K60 HEVC: the Direct3D 11 renderer dropped no frame in 20 s; OpenGL
  dropped 6 in the first second while the window took the video's size,
  then none.
- The Direct3D 11 renderer's self-test: 11 of 11 (mpv's window in place,
  `gpu-next`, the holes, resizes, fullscreen).
- HDR10: OpenGL tone maps with peak detection; Direct3D 11 tone maps too,
  because the display is SDR (DXGI: `RGB_FULL_G22_NONE_P709`); HDR output
  to an HDR display is untested.
- NVIDIA RTX Video Super Resolution and RTX Video HDR start (mpv's log),
  1080p reaching the renderer as 2560x1440 PQ, no dropped frames.
- SVP: with both renderers, attached at once, 58-60 fps from 23.976,
  avsync 0 after seeks, RVP's sharpen and M12's crop, Auto Level and
  WarpSharp before `@svp`, SVP's entry unchanged.

## Findings

1. **Record Video's file was empty on Windows.** Dropping the encoder's
   handle returned while a script's client kept its core alive; the core
   finishes the file only when it quits. `encode()` sends `quit` and
   waits for the core's `Shutdown` event.
2. **The window's geometry fought itself on Windows.** Windows reports a
   change after the call returns, so `x` set after `width` put the old
   width back. Every place and size goes through one `setGeometry`.
3. **A pending fit to the video's shape undid commands**: the 300 ms fit
   computed from the old size landed after a new one. A command now
   cancels it.
4. **A fit left the window below its minimum** (320x114 with the playlist
   docked and a small video): the fit grows the video instead.
5. **A `vf` change in the last moments of a file can stall libmpv 0.37**
   (the time stops, `eof-reached` never comes), like the audio chain's:
   RVP's video filters wait for the next file there, and filter test
   clips are 12 s long.
6. **MSVC refused `std::max`** after `windows.h` (its `max` macro):
   `NOMINMAX`.
7. **The VM's software OpenGL (3.0) cannot run mpv's hooks**: shader
   checks are skipped there and run under Linux's Mesa and on the host.
8. **Session 0 makes no Direct3D 11 swapchain** (`0x887a0022`): the VM
   checks the renderer's window, the host its picture.
9. **PotPlayer's HLSL needed more than macros**: matrices stored
   transposed (`mul(a, b)` as `b * a`), brace initializers, scalars
   returned as vectors, `#ifdef GLSL` branches, the `c0` global; shaders
   needing a second texture or constants after `c2` are refused with the
   reason.
10. **Settings coverage needs exact paths** and two rows shared PotPlayer's
    ID 10287; the coverage test found both.
11. **Super resolution's scale was truncated** (1440 over 1080 written as
    1.333 gave 2559 pixels) and an unchanged processor entry was set
    again at the start: the scale is rounded up, and the chain changes
    only when the entry does.
12. **mpv's frame rate estimate needs time after the chain starts again
    with SVP's filter** (more than 5 s with Direct3D 11): the SVP
    self-test waits up to 15 s.

## Differences from PotPlayer

- Record Video encodes the recorded stretch again from the source with
  RVP's filters after it stops; PotPlayer records the shown picture.
- PotPlayer's HLSL shaders run translated; those needing a second
  texture, constants after `c2`, or projected or gradient sampling are
  refused with the reason. RVP's own set are mpv hook shaders.
- 3D subtitles and H.264 MVC's second view are not shown; Desktop Mode and
  the AviSynth processing of the playing video are not in RVP.
- Rendering is mpv's: DirectShow's renderers and their options keep
  reasons. With the Direct3D 11 renderer RVP's layers over the video show
  over black, dialogs are windows of their own, there is no exclusive
  fullscreen, and super resolution and RTX Video HDR do not run with SVP
  or RVP's own filters (they need the frames on the GPU).
- Visualizations are FFmpeg's (`showcqt`, `showspectrum`, `avectorscope`,
  `showwaves`), not PotPlayer's own.
- Decoding choices are mpv's per codec, not DirectShow's filters.

## Linux items for goal S

- The Direct3D 11 renderer is Windows'. On Linux a child window for
  mpv's `gpu-next` works on X11 only (Wayland has no foreign child
  surfaces); HDR output needs Wayland's colour management, which the
  render API in Qt's window does not reach. Super resolution and RTX
  Video HDR have no Linux path in mpv.
- `.avs` and `.vpy` files: Ubuntu's FFmpeg 6.1 has neither demuxer; the
  kit's libmpv needs FFmpeg built with them.
- Window moves, corners and placing by coordinates do nothing on
  Wayland (the compositor places windows); staying on top needs a
  protocol GNOME does not offer.
- Keeping the pointer on the fullscreen display (Windows' `ClipCursor`).
- Desktop Mode has no counterpart on GNOME Wayland.
- The shaders and the HDR tone mapping on the owner's Ubuntu boot (GPU).

## What to try

In `C:\Users\<user>\rvp-test\rvp-n` (portable: its settings stay beside
`rvp.exe`), with PotPlayer's menu texts and keys:

1. **Aspect and frame**: Ctrl+F6 cycles the aspect ratio; Video > Aspect
   Ratio > Custom; Num0 cycles Frame Size presets; Ctrl+Q opens
   Extend/Crop; Ctrl+Alt+B cycles the bottom margin.
2. **Window**: 1 to 4 (0.5x to 2x), Alt+1 to Alt+4 (30 % to 75 % of the
   desktop), \` (minimum), 5 (maximum), 9 and 0 (custom size and its
   dialog), Ctrl+Alt+Num+ and Num- (larger, smaller), Ctrl+Shift+Enter
   (fullscreen on another display).
3. **Effects and 3D**: Video > Effects (Auto Level, WarpSharp, motion
   blur...), Alt+K rotation; J opens 3D Video Mode (a side-by-side file
   shows as anaglyph or one eye); a 360° video turns with a drag.
4. **Shaders**: Shift+P and Ctrl+Alt+P (pre- and post-resize), S opens
   the Pixel Shaders menu: RVP's set and PotPlayer's translated ones.
5. **Capture**: Ctrl+E (the source frame), K opens Video Capture (the
   clipboard, Save As, BMP), Ctrl+G consecutive images, Alt+N the
   thumbnail image.
6. **Recording**: Ctrl+A (Record Stream), Alt+C (Record Video; Alt+C again
   stops and encodes).
7. **Renderer**: Preferences > Video, Output: "Direct3D 11 window" (then
   start RVP again). With Windows HDR on, HDR files go to the display as
   HDR; GPU Super Resolution and RTX Video HDR are below it (not with
   SVP).
8. **Audio files**: Visualization menu (FFmpeg's views, album art, the
   slideshow with its previous and next slide).
9. **Settings window**: the tree with the group's pages as tabs.

## Open

- HDR output to an HDR display (Windows HDR was off during the session).
- OpenGL's 6 dropped frames in the first second of 4K60 while the window
  grows to the video's size (none afterwards).
