# S6: other players' skins (M9, goal K's study)

Date: 2026-10-31. The study covers three formats: VLC's skins2 themes
(`.vlt`), mpv's on-screen controllers (Lua scripts), and classic Winamp
skins (`.wsz`).

**Sources.** I read the source code of:
- VLC: 3.0.x `2c5595ee` and master `c7834066` (`modules/gui/skins2`,
  `share/skins2`, the skins2 howto);
- mpv: v0.37.0, and `6ee40e64d`, the commit RVP's Windows libmpv is built
  from;
- uosc 5.13.0, ModernX (cyl0 `b90403ce`, zydezu v0.4.8), ModernZ v0.3.3 and
  thumbfast `0f711de3`;
- mpv.net, Celluloid and Haruna;
- Webamp `f5c578bf` (MIT);
- Winamp 5's source as released in 2024 (a mirror at `9c7f914`). Its
  licence (WCL 1.0.1) allows reading but not adapting, so RVP may use the
  facts recorded here but not the code.

**Samples:**
- VLC: the 15 most downloaded skins of VLC's gallery, its 5 best-rated
  skins of 2010 (the gallery's newest year) and VLC's default skin.
- Winamp: the Winamp Skin Museum's first 25 skins (the base skin, three
  ports and 21 skins ranked by the museum), plus the file lists of the
  museum's first 2,000 skins.

**What was run.** mpv's built-in controller ran headless on libmpv 0.37 in
WSL (section 1). VLC and Winamp were not run. Downloaded scripts (uosc,
ModernX) were not run either: this session does not run third-party code
without the owner's permission.

**Local-only material.** The downloads, the full notes (every element,
action, variable and sprite rectangle, with file and line references), the
measuring scripts and the probe are in `~/rvp-reference/s6`. They
are never committed: none of the sampled skins or scripts may be
redistributed by RVP (section 4).

## Short answer

| | mpv's on-screen controllers | Classic Winamp skins | VLC skins |
|---|---|---|---|
| What it is | A Lua script that mpv draws over the video and that takes the pointer and keys | 11 to 20 fixed-name bitmaps in a ZIP; fixed windows of 275x116 (main, equalizer), a resizable playlist and a video window | An XML theme with PNGs and fonts in a tar.gz or zip: free-form windows, layouts, controls and VLC's actions |
| How many | mpv's built-in OSC, uosc, ModernX/ModernZ | The Winamp Skin Museum's collection (its first 2,000 skins were measured) | VLC's gallery: 138 skins, the newest from 2010 |
| What RVP already has | libmpv with Lua on Windows; the probe shows the built-in OSC working through RVP's kind of embedding | RVP's equalizer has exactly Winamp's ten bands; the spectrum filter; bitmap sprites | The display list, sprites, texts, sliders and the playlist drawing of `.dsf` skins |
| What RVP lacks | A mode without its skin; passing the pointer, buttons, wheel and keys to mpv; two-way window state; mpv's `quit` | A second loader; several top-level skin windows that dock and snap; window shapes; ×2 scaling; bitmap fonts; more BMP variants; fallback art drawn by RVP | A second loader with VLC's layout model; several windows that dock and snap; window shapes; scaled images; skin fonts; Bezier sliders; VLC's expressions and actions |
| Cost | **M** | **L** (M-L once the shared window host exists) | **L** |
| Recommendation | First | Second | Last, or not at all |

The window host is shared: several shaped, docked skin windows (parts
X1 to X5 below) serve both Winamp and VLC. Doing both formats costs less
than the sum of the two.

## What RVP has today

- **`crates/skin`** reads `.dsf` only. It opens the archive, preprocesses
  the markup, lays out PotPlayer's docking model on `Element`s and paints
  a `DisplayList` of three operations:
  - `Op::Image`: a tile blit;
  - `Op::Fill`;
  - `Op::Text`: system fonts through Qt, or a 22-cell bitmap font
    (`PicLabel`).

  Sprites are tiled or nine-sliced, never scaled. The BMP decoder reads
  8, 24 and 32 bits per pixel, uncompressed only.
- **`Chrome`** (`crates/app/src/chrome.rs`):
  - holds a `&'static Skin` with one of four modes;
  - computes the layout, the dynamic regions and hits;
  - takes PotPlayer's `CmdID`s, which `ACTIONS` maps to RVP's 884
    actions.
- **Windows:**
  - the main window (frameless, rectangular, opaque);
  - the control panel (F7, a second `Chrome`).

  The playlist lies inside the main window. No skin window has a shape or
  a translucent part.
- **mpv's input and Lua:**
  - The main core sets `input-default-bindings=no`,
    `input-vo-keyboard=no`, `osc=no` and `ytdl=no`. Keys and the pointer
    reach only RVP's handlers.
  - Lua is present in RVP's Windows libmpv (LuaJIT, mpv 0.41-dev) and in
    CI's (shinchiro's). Both include all of mpv's built-in scripts.
  - Ubuntu 24.04's libmpv 0.37 (the sysroot and CI's Linux job) has
    Lua 5.2.
  - The Linux kit's own libmpv, used by the Linux packages, is built with
    `-Dlua=disabled -Djavascript=disabled`.

## 1. mpv's on-screen controllers

### Findings

- **The probe.** On libmpv 0.37 in WSL, with the software render API
  (1280x720), `osc=yes` and RVP's `input-default-bindings=no`:
  - Before the pointer moved, the bottom of the frame was the plain video.
  - After `mouse 640 680`, mpv drew its bottom bar into the rendered
    frame: transport, seek bar, times, track buttons and volume.
    `mouse-pos` reported `hover: true`.
  - `keydown`/`keyup MBTN_LEFT` on the play button started playback, and
    the same click on the seek bar went to 39 s of the 60 s clip.
  - The bar hid after the pointer moved away.

  The OSC's pointer sections are "forced", so they work while RVP keeps
  mpv's default bindings off. This is how a libmpv host gets the OSC.
- **The input contract**, from mpv's source (`input/input.c`,
  `player/command.c`, osc.lua, uosc's `cursor.lua`):
  - **Moves:** `mouse X Y`, in OSD pixels. With the render API that is the
    FBO's physical pixels, so Qt's logical position relative to the video
    item must be multiplied by the device pixel ratio.
  - **Enter and leave:** `keypress MOUSE_ENTER` and `MOUSE_LEAVE`. uosc
    ignores moves while hover is false.
  - **Buttons:** `keydown`/`keyup MBTN_LEFT|RIGHT|MID`. Never send
    `mouse X Y BUTTON`: that is a single press, which osc.lua and uosc both
    ignore.
  - **Wheel:** `keypress WHEEL_UP|DOWN`.
  - **Keys:** `keydown`/`keyup` with mpv's key names, at least while a
    controller's menu is open. Scripts announce this in
    `user-data/mpv/console/open` and `user-data/uosc/menu/type`.
- **Window commands.** The controllers' close, minimize, maximize and
  fullscreen buttons run `quit`, `cycle window-minimized`,
  `cycle window-maximized` and `cycle fullscreen`.
  - Under the render API, and under `wid` with a parent window, mpv only
    changes those option values. The host must observe them, apply them to
    its window, and write its real state back.
  - `quit` ends the core for good (`MPV_EVENT_SHUTDOWN`). RVP ignores that
    event today (`app.rs`).
  - mpv's built-in window dragging does nothing under the render API, and
    under `wid` it would move mpv's own child window. It must be off
    (`window-dragging=no`), and RVP moves its window itself.
- **Files and options under libmpv.** libmpv's built-in profile sets
  `config=no`, which voids `config-dir`: there is no `~~/scripts`,
  `script-opts`, `fonts` or `input.conf`. RVP would pass absolute paths
  instead: `scripts=`, `script-opts=` and `osd-fonts-dir=`.
- **Without a video.** `osd-dimensions` is zero, so nothing draws while RVP
  is idle or plays audio. `force-window=yes` fixes that.
- **The controllers:**

  | Controller | Needs | Conflicts with RVP |
  |---|---|---|
  | mpv's OSC (built in) | nothing; its icon font is compiled in | prev/next drive mpv's playlist (RVP never uses it) and close is `quit`. From mpv 0.40 every button's command is an option, so RVP can send them to its own actions (`script-message-to main ...`); 0.37 hard-codes them. Its thumbnail hook (`user-data/osc/draw-preview`, in RVP's Windows libmpv) can be served by RVP's thumbnail core |
  | uosc 5.13 (LGPL-2.1, mpv 0.33+) | two fonts; the `ziggy` helper programs (subtitle search, clipboard, updater); optional thumbfast | its playlist, shuffle and autoload drive mpv's playlist; it can delete files and update itself through PowerShell; volume in mpv's scale |
  | ModernX / ModernZ (most active: ModernZ, about 1,300 stars) | an icon font; mpv 0.39+ for its menus | as the OSC; yt-dlp calls |
  | thumbfast (thumbnails for the others) | a separate `mpv` executable | none ships with RVP. In RVP's Direct3D 11 mode it retries at every hover. RVP could answer its protocol with its own thumbnail core instead |
- **Side finding (worth fixing on its own).** RVP turns off only `osc` and
  `ytdl`. Each of RVP's libmpv cores (player, second audio device,
  thumbnails, encoding) therefore still starts up to seven built-in Lua
  scripts: console, stats, select, positioning, commands, context_menu and
  auto_profiles. Setting `load-*=no` on the helper cores saves those Lua
  states.

### What RVP would build

| Part | What | Size |
|---|---|---|
| C1 | A controller mode, chosen on the Skins page:<br>• none (RVP's skin), mpv's OSC, or a script from the user's scripts folder (`<config>/mpv-scripts`, with its `fonts` and `script-opts`);<br>• in the mode, RVP's skin is hidden and the video fills the frameless window;<br>• the main core gets the options above, the other cores stay clean | S-M |
| C2 | Input:<br>• pointer moves with the pixel ratio, enter and leave, buttons, wheel, and keys while a controller's menu is open;<br>• a section of RVP's own under the controller's, so that clicks no controller takes (right click, double click, drags) still reach RVP's menu, fullscreen and window move;<br>• for both renderers (OpenGL render API and the Direct3D 11 child window) | M |
| C3 | Window state both ways (fullscreen, maximized, minimized, on top)<br>• `quit` closes RVP;<br>• with mpv 0.40+, the OSC's close, previous and next run RVP's actions | S |
| C4 | Thumbnails: answer the OSC's preview hook and thumbfast's messages with RVP's thumbnail core (optional) | S-M |
| C5 | Tests:<br>• a headless engine test like the probe (the OSC draws; a forwarded click toggles pause);<br>• `--selftest` steps;<br>• the Windows VM | S |

- **Linux, for goal S:** the kit's libmpv with Lua. Ubuntu's 0.37 OSC
  cannot remap its buttons.

## 2. Classic Winamp skins

### Findings

- **The archive.**
  - A ZIP whose files are found by base name: case and folders are
    ignored, and the last duplicate wins.
  - Classic Winamp reads only `.bmp`, `.txt`, `.cur` and `.ini`. PNG is a
    WACUP and Webamp extension.
  - Bitmaps come in every Windows variant: in the sample, 73 % are 24-bit,
    the rest 8-bit (also RLE8), 4-bit, 16-bit and 32-bit (alpha ignored).
  - A missing file falls back, file by file, to Winamp's built-in bitmap.
- **The windows.** All have fixed geometry:
  - main and equalizer: 275x116, with windowshade strips of 275x14;
  - playlist, video and the generic windows: resizable in steps of 25x29
    from 275x116.
  - "Double size" doubles the main window and the equalizer
    (nearest-neighbour).
  - Dragging the main window carries every window docked to it. Windows
    snap within 10 px.
  - `region.txt` gives polygon shapes (29 % of the museum's skins).
  - Up to 29 cursors, including animated ones stored as `.cur`.
  - Fonts: `text.bmp` (5x6 cells), `numbers.bmp`/`nums_ex.bmp` (9x13), and
    the playlist's Windows font and colours from `pledit.txt`.
- **Controls** sit at fixed rectangles. The notes record each of them with
  their formulas:
  - the time and the marquee;
  - kbps and kHz, mono/stereo, the play state;
  - volume and balance (28 frames), the seek bar;
  - the clutter bar;
  - the equalizer's ten bands, preamp and response curve;
  - the playlist frame, its rows and buttons;
  - a 76x16 visualizer with `viscolor.txt`'s 24 colours.
- **The video.** Winamp 5 frames its video window with `video.bmp`
  (234x119):
  - a 20-px title bar with the caption painted in, 11- and 8-px sides, and
    a 38-px bottom;
  - five buttons (fullscreen, ×1, ×2, library, menu) and an info line;
  - its default place is under the main window, 275x232.
  - A skin without `video.bmp` gets Winamp's own, not anything from the
    skin.

  Coverage of the museum's first 2,000 skins:
  - `video.bmp` 10.8 %, `gen.bmp` 11.7 %;
  - `mb.bmp` (same geometry) 76.8 %;
  - `pledit.bmp` 99 %. The playlist's title row and 38-px bottom use
    `video.bmp`'s x layout.
- **In a video player:**
  - transport, seek, volume, time, shuffle and repeat map directly;
  - eject maps to Open File;
  - RVP's equalizer has Winamp's bands (60, 170, 310, 600 Hz, 1, 3, 6, 12,
    14, 16 kHz) and a preamp;
  - the analyzer can be fed by RVP's spectrum filter. The oscilloscope
    cannot: mpv hands RVP no samples;
  - balance has no RVP setting (it could drive the channel gains);
  - the minibrowser, AVS and Media Library have no meaning in RVP.
- **Licences:** none of the 25 skins allows RVP to ship it. Several are
  game or film art. Winamp's base skin is Winamp's art. So:
  - tests use skins drawn in code;
  - RVP needs **its own fallback art** in the classic formats for files a
    skin lacks (at least the video frame), drawn under M7's rules.

### What RVP would build

| Part | What | Size |
|---|---|---|
| W1 | The archive and files: base-name lookup; the missing BMP variants (1-, 4- and 16-bit, RLE4/RLE8); `pledit.txt`, `viscolor.txt` and `region.txt`; cursors | S |
| W2 | RVP's own fallback art in the classic layout (an "RVP Classic" base set): needed for missing files and as the synthetic test skin | M |
| W3 | The main window: every sprite and formula (digits, marquee, sliders, clutter bar, title bars, the analyzer from RVP's spectrum) | M |
| W4 | The equalizer window: bands, preamp, on, presets (RVP's 19), the response curve | S-M |
| W5 | The playlist window: frame, rows in `pledit.txt`'s font and colours, scroll bar, mini transport, its menus mapped to RVP's playlist actions, RVP's albums | M |
| W6 | The video window, as the owner chooses (see the questions):<br>• `video.bmp` when the skin has it;<br>• otherwise a frame from the skin's own playlist pieces, which 99 % of skins carry;<br>• otherwise RVP's fallback art | S-M |
| W7 | Windowshade modes, double size, cursors | S-M |

Plus the shared window host (X1-X5).

## 3. VLC skins (skins2)

### Findings

- **The format.**
  - `theme.xml` (DTD 2.0) holds `Bitmap`/`SubBitmap`, `Font` (TTF in the
    archive), `Window` → `Layout` → `Panel`/`Group` → `Image`, `Button`,
    `Checkbox`, `Text`, `Slider` (with `SliderBackground` grids),
    `RadialSlider`, `Playtree` and `Video`, and `Anchor`s for docking.
  - Positions tie each control's corners to its box's corners, with a
    keep-ratio mode and `%`/`px` units. Windows resize only through images
    with `resizeE`, `resizeS` or `resizeSE`, within the layout's limits.
  - Images tile, stretch or fit.
  - Sliders follow one Bezier curve of degree n-1.
  - `visible` takes boolean expressions over VLC's state, with `or`
    binding tighter than `and`.
  - Actions are strings such as `vlc.play()`, `playlist.setRandom(true)`
    and `main.setLayout(pl)`.
  - Texts take `$` codes: time, title, volume and others.
  - A window's shape is the union of its opaque pixels.
- **Failures and quirks.**
  - A missing required attribute fails the whole skin.
  - An unknown action, bitmap or font silently drops only that control.
  - The last `Layout` of a window shows first.
  - The full list of such rules is in the notes (17 items).
- **The sample.**
  - Elements:
    - every skin uses images, buttons, sliders and texts;
    - 20 use checkboxes and a playlist;
    - 19 have a video area;
    - none uses `RadialSlider`, `PopupMenu`, `BitmapFont`, `IniFile` or
      `Include`.
  - Windows:
    - 19 skins have several windows (median 4: main, playlist,
      equalizer...);
    - 6 switch layouts;
    - 3 have a fullscreen controller;
    - 16 have a resizable main window;
    - 7 dock windows with anchors.
  - Sliders: 310 of 312 are straight lines, and 230 of them are vertical
    (equalizer bands, scroll bars).
  - Bitmaps: all 952 are 8-bit PNGs. Every skin ships its fonts, 12 of
    them proprietary ones (Verdana and others).
  - Actions:
    - every action used is valid;
    - the most used are move, quit, minimize, play, next and previous,
      window show and hide, mute, stop, resize, the playlist toggles,
      fullscreen and the equalizer.
- **Lost controls.** VLC's own rules drop 47 controls. 36 of them are in
  *darklounge* (707,000 downloads), whose archive names files with a
  backslash. A loader that reads `\` as a folder separator loads all 36.
- **What libmpv cannot do:**
  - `dialogs.streamingWizard()`: libmpv has no stream output.
  - Recording, cover art, the playlist tree, VLC's dialogs and its
    context menus become RVP's own.
  - A skin's volume slider spans 0-200 % of VLC's volume, which needs a
    mapping to PotPlayer's percent.
  - The equalizer is ±20 dB in VLC.

### What RVP would build

| Part | What | Size |
|---|---|---|
| V1 | The archive and XML: tar.gz and zip; `theme.xml` search; backslash names; the DTD's defaults; the failure rules | S |
| V2 | The model and layout: windows, layouts and their switching, panels, groups, corner anchoring and keep-ratio, units, `position`, resizing through grip images within the limits | M |
| V3 | Drawing:<br>• images that tile, stretch or fit (a scaled image operation; RVP never scales today);<br>• buttons and checkboxes from separate bitmaps;<br>• animated bitmaps;<br>• `alphacolor` keying with PNG alpha and per-pixel hit testing;<br>• texts in the skin's fonts (loaded from the archive) with scrolling and the `$` codes;<br>• Bezier sliders with background grids;<br>• the playlist from RVP's albums;<br>• the video area | M-L |
| V4 | Expressions and actions mapped to RVP's state and actions; VLC's dialogs and menus become RVP's own; the volume and equalizer scales; a format coverage report `docs/coverage/vlc.md` (VLC's actions, variables and `$` codes and what RVP does with each, generated like `blackspace.md`; no skin content) | S-M |

Plus the shared window host (X1-X5).

## 4. Shared parts for Winamp and VLC

| Part | What | Size |
|---|---|---|
| X1 | Several top-level skin windows: one per skin window, each a frameless window with its own display list, shown, hidden, moved and resized by the skin. The control panel (F7) is the precedent | M |
| X2 | Window shapes:<br>• VLC: the union of opaque pixels;<br>• Winamp: `region.txt`'s polygons.<br>Both are hard-edged regions: VLC sets them with `SetWindowRgn` on Windows and XShape on X11 (`win32_graphics.cpp`, `x11_display.cpp`). `QWindow::setMask` reproduces them without per-pixel translucency, which would put RVP's Direct3D 11 child window at risk (not tried) | S |
| X3 | Snapping and docking: Winamp's 10-px rule with the main window carrying its group; VLC's magnet and anchors with priorities; positions saved per skin | M |
| X4 | Scaled drawing: an image operation that stretches (VLC's `scale` and fit) and a nearest-neighbour ×2 for Winamp's double size | S |
| X5 | A second front end in `Chrome`:<br>• the shell takes the layout, display list, regions and hits from `.dsf`, VLC or Winamp alike;<br>• today `Chrome` holds a `.dsf` `Skin`, so this is a refactor | M |

- **Wayland (goal S):** a Wayland client cannot place its own windows.
  - Docked groups and saved positions there need XWayland, or one
    window holding a whole docked group.
  - The decision belongs to goal S. On Windows and X11 the parts work as
    described.

## 5. Order and sizes

1. **mpv's controllers (M).**
   - They are proven by the probe, need no new drawing in RVP, and give
     users three well-known looks.
   - The risks are small and known: input routing, `quit`, mpv's playlist.
2. **Classic Winamp (L, with X1-X5).**
   - It is the best-known format, by far the largest collection, and has
     one fixed layout.
   - Its equalizer is RVP's own.
   - The cost lies in the many exact sprite rules, RVP's fallback art and
     the window host.
3. **VLC (L, M-L after Winamp's window host).**
   - Its gallery stopped in 2010, and the format is VLC 3's only.
   - It needs the most drawing work: free layout, scaling, fonts and
     Bezier sliders.
   - The skins are mostly Windows Media Player and iTunes look-alikes.

## Decisions for the owner

1. **Scope:** mpv's controllers, Winamp, VLC, in any combination, or
   postpone goal K.
2. **mpv's controllers:** RVP's OSC only, or also the user's own scripts
   (uosc, ModernZ) from a scripts folder. Testing with the user's scripts
   needs the owner's permission to run the downloaded uosc and ModernZ
   (headless in WSL and on the VM).
3. **Winamp's video:** the recommendation is a video window docked under
   the main window, as Winamp 5 places it. It would be framed by:
   - the skin's `video.bmp`;
   - otherwise the skin's playlist frame pieces (99 % of skins);
   - otherwise RVP's own art.

   Fullscreen shows the video alone, as Winamp does.

## The owner's decisions (2026-10-31)

- **Scope:** all three, in the order above:
  1. mpv's controllers;
  2. classic Winamp, with the shared window host;
  3. VLC.
- **RVP's own skin stays.** A controller is one more choice, not a
  replacement. It can show:
  - **over the video**, with the skin around it; in fullscreen it takes
    the place of the skin's bars;
  - **without the skin**: the window holds only the video and the
    controller.
- **Controllers:** mpv's OSC and the user's own scripts. The owner allowed
  running the downloaded uosc and ModernZ for tests, headless in WSL and on
  the Windows VM.
- **Winamp's video:** a video window docked under the main window, framed
  by:
  - the skin's `video.bmp`;
  - otherwise the skin's playlist frame pieces;
  - otherwise RVP's own art.

  Fullscreen shows the video alone.
