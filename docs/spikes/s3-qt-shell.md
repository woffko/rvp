# Spike S3: cxx-qt shell with a Rust skin item and an mpv video item

Questions (PLAN.md M0):

1. Does a cxx-qt QML window without a frame move and resize through
   `startSystemMove`/`startSystemResize`?
2. Can a context menu extend beyond a 440x77 window?
3. Can a `QQuickPaintedItem` implemented in Rust paint a `crates/skin`
   display list?
4. Does an mpv OpenGL video item work when mpv's wakeup callback is forwarded
   to the Qt thread?

Answer: **yes to all four, with the limits listed below.** The stack chosen in
PLAN.md (Qt 6 Quick + cxx-qt + libmpv render API) is confirmed on Windows and
on Linux (GNOME Wayland). On the owner's RTX 30-series card (goal C) 4K60 HEVC
plays without dropped frames while the chrome updates once per second, as
long as chrome updates are applied together with the next video frame; mpv
in a native child window (`wid`) needs no such care. Real mouse drags move
and resize the frameless window on Windows and on GNOME Wayland.

## The spike

`spikes/s3-qt-shell` (cxx 1.0.204, cxx-qt 0.10.0, libmpv2 6.0.0, Qt 6.10.3):

- `qml/Main.qml`: a frameless 960x540 `Window`, a title area that calls
  `startSystemMove()` on a left press, a 16x16 corner grip that calls
  `startSystemResize(Qt.RightEdge | Qt.BottomEdge)`, a `Menu` with
  `popupType: Popup.Window` (Qt 6.8+), and a scripted self-test
  (`--selftest`).
- `src/skin_item.rs`: `SkinItem`, a `QQuickPaintedItem` subclass declared in
  a cxx-qt bridge. `relayout()` loads the skin with `rvp_skin` and builds the
  display list; `paint()` executes it through three small C++ helpers
  (tiled image, fill, elided text). Atlases are decoded once into `QImage`.
- `cpp/spike.cpp`: `MpvItem`, a `QQuickFramebufferObject` whose renderer
  calls Rust (`rvp_mpv_init_gl`, `rvp_mpv_render`); `SpikeTools` for the
  self-test (window list, grabs, hard exit).
- `src/mpv.rs`: `MpvController`. The libmpv wakeup callback queues
  `drain_events()` on the Qt thread with `cxx_qt::Threading`; the render
  update callback calls `QQuickItem::update()` through a queued
  `QMetaObject::invokeMethod`.
- `--log-pointer` and `--log-geometry` make the window log pointer positions,
  its size and, when the menu opens, the window list; `S` shrinks it to
  440x77. The kit's input check (`tools/linux-kit/input-check.py`) uses them.

The self-test resizes the window to 440x77, opens the menu, checks that the
menu is a separate top-level window larger than the main window, calls the
system move/resize functions, restores 960x540, plays a video in the
`CMD_VIDEO_WINDOW` slot of the skin and checks frames, events and painting.
With `--gpu-checks` it also requires that rendering keeps up and no frames
are dropped.

## Windows test VM

Build: `spikes/tools/windows-deps.sh && spikes/tools/build-windows.sh`
(cross-built from WSL, see `windows-build-path.md`), bundle copied to
`C:\rvp-spikes\s3-bundle`.

Environment: Windows 11 10.0.26340.9616, 2 vCPU, no GPU. The program runs
over SSH, which is session 0: windows exist but are never shown or exposed.
Qt uses its software OpenGL (`QT_OPENGL=software`, Mesa llvmpipe, GL 3.0).

```text
powershell: $env:QT_OPENGL="software"
s3-qt-shell.exe --selftest --skin C:\rvp-spikes\s3-bundle\BlackSpace_en.dsf --out C:\rvp-spikes\s3-out
```

Video: the default `av://lavfi:testsrc2=size=1280x720:rate=24000/1001`,
`hwdec=no`, 6 seconds.

```text
environment={"platform":"windows","qt_version":"6.10.3","screens":[{"device_pixel_ratio":1,"height":768,"name":"WinDisc","width":1024}]}
graphics_api=3 (OpenGL=3)
frameless=PASS
menu_window_created=PASS
menu_extends_beyond_window=PASS menu 264x509 main 440x77
start_system_move_without_press=true
start_system_resize_without_press=true
video_rect=5,32 650x432
gl_renderer=Gallium 0.4 on llvmpipe (LLVM 3.6, 128 bits)
gl_version=3.0 Mesa 11.2.2
mpv_render_context=created
mpv_status=playing
mpv_events_on_qt_thread=4
mpv_estimated_vf_fps=23.976023976024017
mpv_time_pos=6.131125000000001
mpv_frames_rendered=14
mpv_frame_drop_count=133
skin_ops_painted=138
wakeup_callback_reached_qt_thread=PASS
video_playing=PASS time_pos 6.131125000000001
video_frames_rendered=PASS
skin_painted=PASS
gpu_checks=SKIPPED
failures=0
```

The grabs (kept in `~/rvp-reference`, not committed) show BlackSpace
painted by the Rust item at 960x540 and the menu as a 264x509 window next to
the 440x77 main window. The video slot is black in grabs: in session 0 each
offscreen grab renders with a fresh GL context, and frames are pumped by a
timer (hence the dropped-frame count).

Not verifiable on the VM (no interactive desktop): real mouse drags and
video on screen. Both were checked on the host in goal C (below).

## Ubuntu VM (GNOME Wayland)

Build and run: `tools/linux-kit/kit.sh all` (see `tools/linux-kit/`), with
Qt 6.10.3 from aqtinstall and libmpv built by the kit: mpv `b2c255c13e`
(`git-release-396-gb2c255c13`), FFmpeg n9.0.2, libplacebo v7.360.1 and
libass 0.17.5-11-gf61db56, all through mpv-build `9443097`. The VM's Vulkan
headers are 1.3.204, older than FFmpeg's minimum (1.3.277), so the kit
builds libplacebo and mpv without Vulkan there. qt-build-utils links Qt
programs with lld, gold or mold because GNU ld.bfd fails; the VM has only
gold, which rustc reports as deprecated. The kit's check asks for `lld` when
none of the three is installed.

Environment: the shared Ubuntu 22.04.5 VM (GNOME Shell 42.9, Wayland,
1280x800, VMware SVGA, 12 vCPU). Qt's Wayland platform plugin with EGL on
Mesa llvmpipe (GL 4.5, Mesa 23.2.1). The kit ran over SSH as the kit's user,
attached to the logged-in user's GNOME Wayland session: a temporary ACL on
`/run/user/1000` and its `wayland-0` socket, `WAYLAND_DISPLAY` set to the
socket path, `ydotoold` 1.0.4 as root, and the pointer acceleration profile
set to flat. Everything was restored afterwards. The full report is
`docs/test-runs/2026-10-10-ubuntu-vm.md`.

Self-test (`kit.sh s3`: generated 1080p60 H.264 clip, `hwdec=no`, 8 s):

```text
environment={"platform":"wayland","qt_version":"6.10.3","screens":[{"device_pixel_ratio":1,"height":800,"name":"Virtual-1","width":1280}]}
frameless=PASS
qt.qpa.wayland: Failed to create grabbing popup. Ensure popup QQuickPopupWindow(...) has a transientParent set and that parent window has received input.
menu_window_created=SKIPPED Wayland needs real input (see the input check)
start_system_move_without_press=false
start_system_resize_without_press=false
mpv_status=playing
mpv_events_on_qt_thread=6
mpv_estimated_vf_fps=60.00000239999994
mpv_time_pos=7.717
mpv_frames_rendered=105
mpv_frame_drop_count=360
skin_ops_painted=138
wakeup_callback_reached_qt_thread=PASS
video_playing=PASS time_pos 7.717
video_frames_rendered=PASS
skin_painted=PASS
gpu_checks=SKIPPED
failures=0
```

Input check (`kit.sh input`, real pointer events through ydotool):

```text
pointer_scale=2.000,2.000
window_origin=220,177 size=960x540
input_check_move=PASS moved=-80,-60 expected=-80,-60
input_check_resize=PASS resized=-100,-80 expected=-100,-80
input_check_menu=PASS menu 200x388 main 440x77 class QQuickPopupWindow
```

So on GNOME Wayland a press on the title area followed by
`startSystemMove()` moves the window by exactly the dragged distance, the
grip with `startSystemResize()` resizes it, and a right click on the 440x77
window opens the menu as a separate 200x388 popup window. The final grab
(kept locally) shows BlackSpace with the mpv video playing in the skin's
video slot. Only 105 of 480 frames were rendered in 8 s, which is expected
from llvmpipe; GPU performance is for goal D.

## Windows host (goal C): performance and real input

Environment: the owner's Windows host, RTX 30-series card (OpenGL 4.6.0 NVIDIA
610.88), one 2560x1440 monitor at 59.951 Hz, 100% scaling. Clip:
`video-2160p60-hevc.mkv` from `tools/test-clips/make-clips.sh` (3840x2160,
60 fps, HEVC 40 Mb/s). Each run: `s3-qt-shell.exe --selftest --gpu-checks
--video CLIP --video-seconds 30 --width 1920 --height 1080` plus the options
in the table; video slot 1610x972; about 29.5 s of playback measured.

| Video output | hwdec | Chrome once per second | Rendered | VO drops |
|--------------|-------|------------------------|----------|----------|
| OpenGL item | `auto-copy` (d3d11va-copy) | whole skin item | 1753 | 13 |
| OpenGL item | `auto` (nvdec) | whole skin item | 1759 | 7 |
| OpenGL item | nvdec | none | 1779 | **0** |
| OpenGL item | nvdec | time label's rectangle of the skin item | 1765 | 15 |
| OpenGL item | nvdec | whole skin item, `video-sync=display-resample` | 1780 | 1 |
| OpenGL item | nvdec | time label as its own small item | 1765 | 10 |
| OpenGL item | d3d11va-copy | time label as its own small item | 1757 | 7 |
| OpenGL item | nvdec | small item, updated with the next video frame | 1774 | **0** |
| OpenGL item | d3d11va-copy | small item, updated with the next video frame | 1763 | **0** |
| OpenGL item | d3d11va-copy | the same, `video-sync=display-resample` | 1763 | **0** |
| child window (`wid`), `gpu-next` D3D11 | `auto` (d3d11va) | whole skin item | n/a | **0** |

Decoder drops were 0 in every run. Painting is not the cause: the whole
skin item takes 5.1 ms on average (85 ms once, when its atlases are
decoded), the small time-label item 2.1 ms, yet both drop frames when they
are updated on their own. Any update outside the video item makes the scene
graph sync and present an extra frame; a 60 fps video on a 59.95 Hz display
has no spare vsync for it, so the next video frame is late. Updating the
chrome item in the same event-loop pass as the video item (the spike's
`--chrome synced`: `SpikeTools::updateWithNextFrame` queues the item and
`MpvItem::frameUpdate` updates both) removes the drops. Two earlier synced
runs dropped 5 and 29 frames while the owner moved the mouse over the
window; the skin item was then repainted 44 and 84 times instead of 4, so
pointer movement over the window also triggers repaints that must go
through the same path.

Real input (`host-session s3-input`, mouse injected with `SendInput`,
window positions read with `GetWindowRect`):

```text
window=800,426 440x77
input_check_move=PASS moved=80,60 expected=80,60 window=880,486 440x77
input_check_resize=PASS grown=100,80 expected=100,80 window=880,486 540x157
input_check_menu=PASS menu=1048,504 264x509 class=Qt6103QWindowPopupSaveBitsOwnDC main_height=157
```

## Findings for RVP

1. **cxx-qt constructors.** cxx-qt's default constructor passes `QObject*`
   as the parent to the base class, which `QQuickPaintedItem` does not
   accept. Declare `impl cxx_qt::Constructor<()>` with empty base arguments
   for every item based on `QQuickItem`.
2. **libmpv2 6.0.0 event API.** `Mpv::wait_event` returns `None` both for
   "no event" and for a property change whose value could not be converted
   (for example an unavailable property). A drain loop that stops on `None`
   leaves events in the queue and mpv stops waking the client. The spike
   polls properties instead of observing them. RVP needs its own small event
   wrapper (or a patched binding) that tells these cases apart.
3. **Render context before `loadfile`.** With `vo=libmpv`, a file loaded
   before the render context exists fails with "No render context set". The
   spike defers `loadfile` until the scene graph has created the context.
4. **Exit.** The spike leaks the mpv handle and its render context, and an
   orderly Qt shutdown then hung on Windows; even `std::_Exit` hung there,
   because DLL detach code still runs. The spike ends with
   `TerminateProcess`. The cause was not isolated. RVP must shut down in
   order (free the render context on the render thread, then
   `mpv_terminate_destroy`, then leave the Qt event loop) and test that on
   both platforms in M2.
5. **Menus.** `popupType: Popup.Window` gives a real top-level menu that
   extends beyond the window on Windows and on Wayland (an `xdg_popup`). On
   Wayland the compositor creates a grabbing popup only in response to real
   input, so menus must open from input handlers (as they will) and tests
   must use injected input. A `\t` in `MenuItem.text` is rendered as a tab
   stop, not as a right-aligned shortcut column, and long items are elided.
   PotPlayer-style menus need a custom `MenuItem` delegate with a shortcut
   column and a dark style.
6. **Chrome updates ride on video frames.** With the OpenGL item, every
   chrome change during playback (time display, hover, state changes) must
   be applied in the same scene-graph sync as a video frame, or 60 fps video
   on a 60 Hz display drops frames. Split the skin into a static layer and
   small dynamic items, and queue dynamic updates until mpv's next frame
   (goal C measurements above). Painting cost itself is small: 5 ms for the
   whole 1920x1080 skin, 2 ms for the time label.
7. **Locale.** libmpv requires the C numeric locale, and `QGuiApplication`
   sets the user's locale on Unix. The spike resets `LC_NUMERIC` to `C` right
   before creating the mpv handle.
8. **System move and resize need the press.** Without a pressed button,
   `startSystemMove()`/`startSystemResize()` return `false` on Wayland (true
   on Windows, where nothing happens). RVP must call them from the press
   handler of the title bar and the resize borders.
9. **Injected input on GNOME.** ydotool's absolute moves were scaled by 2 on
   the VM while relative moves were not, even with a flat profile at speed 0.
   The input check measures the scale first and drags with absolute moves.
10. **Child window is the low-risk path on Windows.** mpv in a native child
    window (`vo=gpu-next`, D3D11, d3d11va) dropped nothing with unsynchronised
    full chrome repaints, because DWM composes its swapchain independently.
    SVP's own `mpv.conf` uses the same output (`vo=gpu-next`). Keep it as the
    Windows fallback (D4). With `wid`, the Qt thread must never wait on mpv:
    mpv's video thread sends window messages to the child window, which the
    Qt thread owns (the spike polls properties on a worker thread).
