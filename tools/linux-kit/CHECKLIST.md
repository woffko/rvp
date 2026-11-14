# Linux test kit: owner checklist (Ubuntu 26.10, GNOME Wayland)

The kit builds libmpv, Qt, mvtools, the goal B spikes and the RVP player in
user space and records the results in `docs/test-runs/`: S1 (SVP attaches),
S2 (VapourSynth interpolation), S3 (Qt shell) and, for milestone M2, the
player's own checks (section 5). It never uses `sudo`: steps that need root
are listed here and printed by `kit.sh check`.

Everything the kit downloads or builds goes to `~/.cache/rvp-kit`.
Nothing is installed system-wide by the kit.

## 1. Before the session (as an administrator)

- [ ] Boot Ubuntu 26.10 and log in to a **GNOME Wayland** session (the
      default). Check with `echo $XDG_SESSION_TYPE` → `wayland`.
- [ ] NVIDIA driver installed and working: `nvidia-smi` lists the
      RTX 50-series card (an RTX 30-series card until 2026-11-07).
- [ ] Packages. Run `tools/linux-kit/kit.sh check`; it prints the exact
      `sudo apt-get install -y ...` line for whatever is missing. Run that
      line, then run `check` again until it passes. The list includes
      `libpipewire-0.3-dev`: without it mpv plays audio through the
      PulseAudio API, and on PipeWire that alone dropped 4-5 video frames
      per second at 60 fps (goal D).
- [ ] Rust for your user: install rustup from <https://rustup.rs>. The
      repository's `rust-toolchain.toml` selects the toolchain on first use.
- [ ] The repository: `git clone https://github.com/woffko/rvp.git ~/rvp`
      (private; log in first, for example with `gh auth login`).

## 2. Optional pieces (each step is skipped with a note when absent)

- [ ] **SVP 4 for Linux** (S1 and S2). Install it with SVP's own installer
      and its requirements (see SVP's Linux guide). Do not change SVP
      settings for the kit.
      - The kit looks for `libvapoursynth-script.so*` under `/opt/svp`,
        `~/SVP4/mpv` (SVP's default user install), `~/.local/share/SVP4`,
        `/usr/local/lib` and `/usr/lib/x86_64-linux-gnu`. If yours is
        elsewhere, set `VSSCRIPT_PATH=/full/path/libvapoursynth-script.so`.
- [ ] **PotPlayer's script** for S2. Mount the Windows partition read-only
      and point the kit at the file, for example (the Files app mounts it
      under `/run/media/$USER/<volume>` or `/media/$USER/<volume>`):
      `export RVP_VPY="/run/media/$USER/<volume>/Program Files/DAUM/PotPlayer/VapourSynth/motioninterpolation.vpy"`.
      The script needs the mvtools plugin, which neither SVP nor Ubuntu
      ships; the kit's `mvtools` step builds it and S2 loads it. To use
      another build, set `RVP_VS_PLUGINS=/full/path/libmvtools.so`.
- [ ] **Test videos** (optional). S3 uses `RVP_VIDEO` (a 4K60 file is the
      point of the GPU check); without it the kit encodes a 30-second
      2160p60 HEVC clip (NVENC when available, otherwise x265). S1 uses
      `RVP_S1_VIDEO`, a film-rate file for SVP to interpolate; without it
      the kit encodes a 60-second 1080p 23.976 fps clip. A local file or
      one on the NAS mount works for both.
- [ ] **The BlackSpace skin** for S3: `export RVP_SKIN=/path/to/BlackSpace_en.dsf`
      (for example from `Skins/` of the PotPlayer folder on the Windows
      partition). Without it the shell paints no skin.
- [ ] **ydotool** for the input check (drags the frameless window with a
      real pointer). Needs ydotool 1.x: Ubuntu 26.10 ships 1.0.4
      (`sudo apt-get install -y ydotool`); Ubuntu 22.04 ships 0.1.8, which
      is too old, so build it from <https://github.com/ReimuNotMoe/ydotool>
      there.
      - Ubuntu's package enables a `ydotool` user service for every user,
        which fails at each login unless the user is in the `input` group.
        The kit does not need it: run
        `sudo systemctl --global disable ydotool.service` once, and do not
        add yourself to `input`.
      - For the check, run in a separate terminal and leave it running:
        `sudo ydotoold --socket-path=/tmp/.ydotool_socket --socket-perm=0666`.
        The path is explicit because under `sudo` the daemon and the client
        would otherwise look in different places; the kit points ydotool at
        `/tmp/.ydotool_socket` (`YDOTOOL_SOCKET` overrides it).
      - The kit switches GNOME's pointer acceleration to *Flat* for the
        check and restores the previous profile afterwards. If the check is
        interrupted, restore it in Settings → Mouse & Touchpad.
      - **The check moves the real mouse pointer for about 30 seconds; do
        not touch the mouse while it runs.**

## 3. Run

From a terminal **inside the GNOME session** (not over SSH: the S3 window
needs the session's display):

```sh
cd ~/rvp
tools/linux-kit/kit.sh all
```

`all` runs `check deps mpv mvtools build s2 s3 input s1 report`. The first
run spends most of its time building FFmpeg and libmpv (about 3 minutes on
the 64-thread machine); later runs reuse the builds. Steps can be run one by
one, for example `kit.sh s3 report`. After adding packages, rebuild libmpv
with `RVP_KIT_REBUILD=1 kit.sh mpv build`.

What appears on screen:

- `s3`: a 960x540 BlackSpace window that shrinks to 440x77, then plays the
  test video for about 20 seconds in a 1920x1080 window and closes. (On
  Wayland the scripted menu is refused without real input; the input step
  checks the menu.)
- `input`: the same window for about 30 seconds. The pointer scans the
  screen to find it, drags its title bar and its bottom-right corner,
  middle-clicks it (the window shrinks to 440x77) and right-clicks to open
  the menu; then the window closes. Only the mouse is used, no keys.
- `s1`: start SVP 4 Manager first. A plain mpv window plays the 23.976 fps
  clip for 60 seconds with a quiet 440 Hz tone; SVP should pick it up and
  switch on interpolation.

## 4. After the run

- [ ] `docs/test-runs/<date>-<host>.md` holds the system summary, the step
      results and the tail of each log. Raw logs stay in
      `~/.cache/rvp-kit/logs`.
- [ ] Check that the report contains nothing private (paths with personal
      names are fine to edit out), then commit and push it:
      `git add docs/test-runs && git commit -m "Record the Ubuntu 26.10 kit run" && git push`.
- [ ] Stop `ydotoold` (Ctrl+C in its terminal).

## 5. Milestone M2: the RVP player (goal E)

Same preparation as above (sections 1 and 2: packages, Rust, the skin in
`RVP_SKIN`, SVP 4 for the last step). Then, from a terminal inside the
GNOME session:

```sh
cd ~/rvp && git pull
tools/linux-kit/kit.sh m2all
```

`m2all` runs `check deps mpv m2build m2 m2input m2gpu m2svp report`; libmpv is
reused from earlier runs. RVP's settings and resume state for these runs
are kept in the kit's log directory; your own `~/.config/rvp` is not used.
**Do not touch the mouse or keyboard while the windows are open**: the
checks drive RVP themselves (through its own input handlers, not by
injecting input).

What appears on screen:

- `m2` (about a minute): the BlackSpace window plays the 4K60 test clip,
  seeks, pauses and resumes, changes volume and mute, moves to the next and
  previous file, goes fullscreen and back, shrinks to 900x500 and hides and
  shows the playlist. Then the film-rate clip plays for 6 seconds, RVP
  closes, starts again without a file and must reopen that clip paused
  where it stopped.
- `m2input` (about 40 seconds, needs `ydotoold` as in section 2): a
  900x560 RVP window playing the film-rate clip. The pointer finds it,
  drags its title bar and its bottom-right corner, double-clicks the video
  (fullscreen) and double-clicks again (back), then right-clicks for the
  menu. Only the mouse is used.
- `m2gpu` (about a minute, GPU machines only): the 4K60 clip plays for 25
  seconds in fullscreen, then 25 seconds in a 1920x1080 window. This
  rechecks goal D's open item (about 6 dropped frames per 20 seconds in a
  window). `RVP_M2_SECONDS` changes the length; keep it below the clip's
  30 seconds (or use a longer `RVP_VIDEO`): after mpv loops a file the
  drop counts are not comparable (see goal E's status).
- `m2svp` (about 45 seconds): start SVP 4 Manager first. RVP plays the
  23.976 fps clip with SVP attached, then exits; the step passes when RVP
  exits cleanly with SVP's filter active (goal D rule 6).

Afterwards, a few things only a person can judge (about 10 minutes; note
anything odd in the report or tell the session):

- [ ] `target/release/rvp --skin="$RVP_SKIN" <a video>`: the window looks
      like PotPlayer's BlackSpace window; drag it by the title bar, resize
      it from an edge and a corner, maximize and restore with the title
      bar button and by double-clicking the title bar, minimize.
- [ ] Double-click the video: fullscreen. Move the pointer to the bottom:
      the control bar appears; away from it: it hides; the pointer hides
      after two seconds. Esc or a double click leaves fullscreen.
- [ ] Space, Left/Right, Ctrl+Left/Right, Up/Down, M, Enter, PgUp/PgDn, the
      mouse wheel and the side buttons do what they do in PotPlayer.
- [ ] Right-click the video: the menu shows the shortcuts. Hover over a
      button: a tooltip appears. Drag a file from Files onto the window: it
      plays. F3 or Ctrl+O opens the file dialog.
- [ ] Close RVP while a video plays and start it again: it reopens the file
      paused at the same place, with the same volume and window size.

## 6. Milestone M3: daily-use essentials (goal F)

Same preparation as section 5. From a terminal inside the GNOME session:

```sh
cd ~/rvp && git pull
tools/linux-kit/kit.sh m3all
```

`m3all` runs `check deps mpv m2build m2 m3 m2input m2gpu m2svp report`:
section 5's steps on the new build, plus `m3`. Settings and state stay in
the kit's log directory. Again, do not touch the mouse or keyboard while
RVP's windows are open.

What `m3` shows (about two minutes):

- The functional self-test of section 5, then the playlist panel: rows are
  clicked, selected with Ctrl+A, deleted and brought back with Ctrl+Z, the
  first row is dragged to the end and double-clicked, an album "Selftest"
  is added and the first tab clicked. Shift+N turns the normalizer off and
  on, the volume goes to 101 %, F5 opens the Keys and Mouse window for a
  moment, a second RVP hands a file to the first one (`/add`), and the
  window is resized to 900x760 and must snap to the video's shape.
- Three kill tests: RVP plays the second entry of a two-file album and is
  killed (`kill -9`) after 12, 16 and 20 seconds; each restart must reopen
  that entry paused within 5 s of where it was.

Optional: `tools/linux-kit/kit.sh m3desktop` adds "RVP (kit)" to the
applications (`~/.local/share/applications/rvp-kit.desktop`), so that
Files can open videos with the running RVP ("Open With"). Delete that file
to undo it.

Then the one-week trial: use RVP instead of PotPlayer for daily viewing and
note anything that differs or annoys. `docs/goals/goal-f-status.md` lists
what to try; in short:

- [ ] Playlist (F6 or the skin's button): add files and folders (Ctrl+I,
      Ctrl+O, drag and drop), remove (Del, Ctrl+Del, Alt+Del), reorder
      (Ctrl+Up/Down/Home/End or by dragging rows), sort (Ctrl+1, Ctrl+2,
      Ctrl+9), undo and redo (Ctrl+Z, Ctrl+Y), double click or Enter to
      play, right-click for the playlist menu.
- [ ] Albums: Ctrl+N adds one, Ctrl+R renames, Ctrl+D or the mark on the
      tab deletes; the arrow at the tabs' end shows the next album.
- [ ] Close RVP, or end it with `kill -9`, and start it again: the album,
      the file and the position come back.
- [ ] PotPlayer's playlists: the playlist menu's "Import PotPlayer
      Playlists..." and the Windows `Playlist` folder (for example on the
      mounted Windows partition). `V:\` maps to `/mnt/media`; set
      `"drive-map": { "V:": "/mnt/nas/Media" }` in
      `~/.config/rvp/settings.json` for the NAS mount.
- [ ] Normalizer: Shift+N toggles it, the context menu has "Boost audio";
      the volume goes above 100 % (up to `volume-max`, 200 %).
- [ ] The blind A/B test of the normalizer: `tools/s5/ab_test.py` (its
      header explains it; the trials stay outside the repository).
- [ ] F5 opens Keys and Mouse: rebind a key and a mouse button (wheel,
      middle, side buttons, double click); a key already in use asks
      first; Reset and Reset All bring PotPlayer's defaults back.
- [ ] Open a second file from a terminal (`rvp FILE`, `rvp /add FILE`,
      `rvp /insert FILE`) while RVP runs: it goes to the running window;
      `rvp /new FILE` opens a second window.
- [ ] Resize the window while a video plays: once you let go, it takes the
      video's shape.

## 7. Milestones M4 and M5: menus and the settings window (goal G)

Same preparation as section 5. From a terminal inside the GNOME session:

```sh
cd ~/rvp && git pull
tools/linux-kit/kit.sh m4all
```

`m4all` runs `check deps mpv m2build m2 m3 m4 m2input m2gpu m2svp
report`: sections 5 and 6 on the new build, plus `m4`. If `m3all` has
not run on this machine yet, this run also counts for M3's acceptance.
Again, do not touch the mouse or keyboard while RVP's windows are open.

What `m4` shows (about a minute):

- The functional self-test of sections 5 and 6, now with M4's and M5's
  checks: the context and playlist menus are built with PotPlayer's tree
  and the keys bound now, a menu command changes the speed and back, F5
  opens the settings window, every page is shown once, the volume step is
  changed in the window and the next Down key uses it at once.
- A copy of RVP with `rvp.portable` next to it plays for 3 seconds; its
  `settings.json` and `state` folder must appear next to the copy (in the
  kit's log directory), not in `~/.config/rvp`.

Then try by hand:

- [ ] Right-click the video: PotPlayer's context menu with only what RVP
      does, the keys bound now and check marks (Repeat, Stay on Top, aspect
      ratio, normalizer). Audio > Select Audio Stream and Subtitles >
      Add/Select Subtitles list the file's tracks; Playback >
      Chapter/Bookmark lists its chapters (try a file with chapters).
- [ ] New commands: D and F (frame steps), X, C and Z (speed), [ ] and \
      (A-B repeat), Shift+, and Shift+. (audio sync), < and > (subtitle
      sync), Alt+PgUp/PgDn and Alt+Up/Down (subtitle size and position),
      Ctrl+F6 (aspect ratio), the numeric keypad (zoom and move the
      picture, 5 resets), 1 to 4 (window size), Ctrl+T (stay on top), Tab
      (playback information), Ctrl+E (save the frame to Pictures/RVP).
- [ ] The skin's own menus: the buttons with a small arrow or a right
      click on them (open, aspect ratio, capture, playback, subtitles,
      window size); menus RVP has nothing for show "Not in RVP yet".
- [ ] The playlist panel's right-click menu: PotPlayer's playlist tree
      plus Undo, Redo and the import.
- [ ] F5: every page, a change applies at once (try the volume step, the
      normalizer's Boost, the video sync, the subtitle size while a video
      plays); the skin page says the skin changes at the next start.
- [ ] Settings > Export…, then change something, then Import… the
      exported file: the old value comes back.
- [ ] Portable mode: copy the build folder elsewhere, add an empty
      `rvp.portable` next to `rvp`, start it: the settings live there.

## 8. Milestone M6: SVP and video processing (goal H)

Same preparation as section 5, with SVP 4 installed (for `m6svp` and the
script slot test). From a terminal inside the GNOME session:

```sh
cd ~/rvp && git pull
tools/linux-kit/kit.sh m6all
```

`m6all` runs `check deps mpv mvtools m2build m2 m3 m4 m6 m2input m2gpu
m6gpu m2svp m6svp report`. Start SVP 4 Manager before the run, or run
`m2svp m6svp report` again later with it running. Do not touch the mouse
or keyboard while RVP's windows are open.

What the M6 steps show (about three minutes, plus SVP's first start):

- `m6`: the functional self-test with M6's checks: F7 opens the control
  panel, a click on its colour slider sets the brightness and the reset
  button clears it, an equalizer band slider switches the equalizer on
  (before the normalizer in the audio chain) and the EQ button switches
  it off, F7 closes the panel, Shift+P switches a shader of RVP's own on
  and off, and with an endpoint SVP does not know RVP says why SVP did
  not attach. Then `--selftest=upright` plays a clip with a white mark at
  the top left and compares the picture with ffmpeg's frame (the
  upside-down picture of the last Ubuntu run would fail it). Then
  `--selftest=speed`: 1.1x and back to 1x one second into the film-rate
  clip, counting the frames dropped in the 1.5 s afterwards against 1.5 s
  of steady playback before (the Ubuntu run of M3-M5 dropped 14 after the
  return to 1x; at most 2 more than the steady count pass). Then a VapourSynth script slot in RVP with SVP's VapourSynth: RVP's own
  PotPlayer-style script (`vs.get_core()`) doubles the frame rate, and with
  `RVP_VPY` set (as for `s2`) PotPlayer's `motioninterpolation.vpy` runs
  with the kit's mvtools. Last, the headless video and equalizer tests.
- `m6gpu` (GPU only): the 4K60 clip in fullscreen with colours, deband, a
  shader and the equalizer on; the dropped frames are in the detail.
- `m6svp` (SVP running): SVP's acceptance. RVP in SVP mode
  (`/tmp/mpvsocket`) plays the 23.976 fps clip; SVP must attach and raise
  the frame rate to about twice the file's (59.94 on a 60 Hz display), and
  after three seeks audio and video must stay within 0.1 s. Eight seconds
  later a second window (`/new`) starts while the first holds the socket:
  it uses `/tmp/mpvSockets/<process id>`, and SVP must attach to it too.

Then try by hand (this is M6's acceptance on Linux):

- [ ] F7: the control panel opens beside the video; its five pages work
      (audio, video, subtitles, playback, aspect ratio); drag it by its
      background; F7 or its close button hides it.
- [ ] Audio page: the band sliders change the sound at once, PRESET and
      the arrow open the presets, the EQ button and Shift+E switch the
      equalizer; with the normalizer on, nothing stutters.
- [ ] Video page: the colour sliders and RESET buttons; BLUR, SHARP and
      POST (soften, sharpen, deblock) show pressed while on; CAPT saves
      the frame under a name you choose.
- [ ] Keys: Q (colours off and back), W/E, R/T, Y/U, I/O (brightness,
      contrast, saturation, hue), Ctrl+I (deinterlacing), Ctrl+P and
      Ctrl+Z (flips), Alt+K (rotation), Shift+P (shaders).
- [ ] With SVP running: a 24 fps film plays at the display's rate (SVP's
      tray icon shows it active); seeking keeps lip sync; a second RVP
      window (`rvp /new FILE`) is interpolated too.
- [ ] Settings > SVP and VapourSynth: the status line says whether SVP is
      attached; add PotPlayer's `motioninterpolation.vpy` as a script and
      the kit's `libmvtools.so` as a plugin, select the script and enable
      it: the frame rate rises without SVP (switch SVP off for this).
- [ ] Settings > Shaders: add an mpv `.glsl` shader (for example from
      mpv's shader collections), switch it on with Shift+P.
- [ ] The picture is upright and not mirrored with a real video.

## 9. Milestone M8: packages and the desktop (goal J)

One session for M6, M7 and M8 (section 8's preparation). From a terminal
inside the GNOME session:

```sh
cd ~/rvp && git pull
tools/linux-kit/kit.sh m8all
```

`m8all` is `m7all` with `m8package` and `m8` after `m7`. `m8package`
builds RVP again with its library folder in its run path and makes the
AppImage and the .deb from the kit's Qt and libmpv (a few minutes). `m8`
runs the AppImage's self-test, then asks for the .deb: install it

```sh
sudo apt install ./<the .deb the step names>
```

and run `tools/linux-kit/kit.sh m8 report` again. It checks the installed
copy (self-test, desktop entry, registration for the media types with no
default changed), MPRIS on GNOME's bus while the test video plays (playing,
metadata, Pause, Quit) and that GNOME keeps the screen on meanwhile
(`IsInhibited` of the session manager), and writes crash logs on purpose.
Afterwards remove the package (`sudo apt remove rvp`) and run
`tools/linux-kit/kit.sh m8removed report`.

Then try by hand (M8's acceptance on Linux):

- [ ] RVP is in the applications grid with its icon; a window shows RVP's
      icon in the dash.
- [ ] Files, a video's context menu, "Open With": RVP is listed; your
      default player is still the default.
- [ ] While a film plays: the top bar's media controls show RVP with the
      title; play, pause, next and previous work there and on the media
      keys; the screen does not blank (wait past the blank-screen delay).
      Paused, it may blank again.
- [ ] The AppImage (`chmod +x` it, run it) plays a film the same way.
- [ ] `rvp --crash-test=native` ends at once and leaves
      `crash-<time>-native.log` in `~/.local/state/rvp`; delete it after.

## 10. Milestones M9 to M18 on Linux (goal S)

One session for the parity milestones' Linux side, after section 9's
preparation.

On the shared Ubuntu VM the kit runs as the kit's user over SSH in the owner's GNOME
session: `eval "$(tools/linux-kit/attach-session.sh attach <user>)"` gives
the kit user's access to the session's Wayland socket and XWayland's authority
file (ACL entries, with sudo) and sets `WAYLAND_DISPLAY`, `DISPLAY` and
`XAUTHORITY`; `tools/linux-kit/attach-session.sh detach <user>` takes the
entries back after the run. Where the network to GitHub is poor,
`tools/linux-kit/sources.sh mirror DIR` on another machine makes a source
cache to copy over and name in `RVP_KIT_SOURCES`; the steps `mpv`,
`mvtools`, `whisper` and `vapoursynth` clone from it first. Optional pieces for these steps (each is SKIPPED with a note
when absent; none of them is ever committed):

- [ ] **mpv's on-screen controllers** for `m9`: a folder holding one folder
      per script, each laid out as RVP's mpv folder (`scripts/`, `fonts/`,
      `script-opts/`), for example uosc's release archive unpacked into
      `uosc/` and ModernZ into `modernz/`:
      `export RVP_M9_SCRIPTS=/path/to/scripts`. mpv's own OSC needs nothing.
- [ ] **Classic Winamp skins and VLC themes** for `m9`:
      `export RVP_WINAMP_SKINS=/path/to/wsz-folder RVP_VLC_SKINS=/path/to/vlt-folder`.
- [ ] **whisper.cpp** for `m13`: your `whisper-cli` and a folder with
      `ggml-tiny.bin` (RVP downloads no model):
      `export RVP_WHISPER_CLI=/path/to/whisper-cli RVP_WHISPER_MODELS=/path/to/models`.
      The speech clip is made with `espeak-ng` when it is installed, or
      `RVP_SPEECH_CLIP` names one.
- [ ] **Unified Remote Server for Linux** for `m15`: its `.deb` unpacked,
      never installed (`dpkg-deb -x urserver-*.deb ~/urserver`), then
      `export RVP_URSERVER=~/urserver`. The step runs it from there with its
      data in the log folder and stops it at the end.
- [ ] **dvdauthor** for `m16` (`sudo apt-get install -y dvdauthor`, which
      has `spumux`): the step makes the synthetic menu DVD and Blu-ray in the
      kit's folder once. `RVP_DVD_MENU` and `RVP_BD_MENU` name other copies.
- [ ] **NVIDIA's Linux wheels** for `m18trt` and RIFE (TensorRT) in RVP:
      TensorRT 10.15.1.29's and CUDA 13.2's runtime and NVRTC wheels, as
      `crates/app/src/tensorrt.rs` pins them, are on the Windows partition
      in `Users/<user>/rvp-test/linux-boot/trt-linux` (never committed):
      `export RVP_TENSORRT_MIRROR=file:///path/to/trt-linux` (an absolute
      path after `file://`). Without it the step and RVP's Download…
      fetch 3.8 GB from NVIDIA and PyPI.
- [ ] **The card's power limit**: none is needed on the Ubuntu boot since
      2026-11-14 (the owner, with the RTX 50-series card); the GPU steps run at
      the card's own limit, which `m18trt` records. Should `nvidia-smi`
      stop answering, stop everything.

From a terminal inside the GNOME session:

```sh
cd ~/rvp && git pull
tools/linux-kit/kit.sh m17all
```

`m17all` is `m8all` with `m9` to `m16`, `m18` and `m18trt` after `m8`;
each step also runs alone (`kit.sh m11 report`). What appears on screen:

- `m9`: RVP's window several times with mpv's controller over the video,
  then each classic skin (main window, equalizer and playlist docked under
  each other) and each VLC theme, about 30 seconds each. A classic or VLC
  skin runs through XWayland when the session has it, since Wayland lets no
  program dock its windows.
- `m10`: the functional self-test, about 3 minutes, **with sound**: the
  test tones play through the default output, and the master volume goes
  one step up and back.
- `m11` to `m16` and `m18`: RVP's window for one to three minutes each,
  without sound; `m12` resizes and moves it (on Wayland the moves are
  skipped), `m16` shows the synthetic discs' menus.
- `m15` with `RVP_URSERVER`: Unified Remote's manager listens on port
  9510 for the step's duration.
- `m18trt`: no window; the card works hard for a few minutes while
  TensorRT builds RIFE's engine and the script is measured.

Then try by hand (the parts only a desktop and its hardware can check):

- [ ] **The tray** (General > "Taskbar/Tray icon"): with Ubuntu's
      AppIndicator extension on, RVP's icon is in the top bar with its menu;
      "minimize to tray" hides the window and the icon brings it back.
- [ ] **The screen lock**: with "Pause playback when screen locked" on,
      locking the screen (Super+L) pauses a playing film and unlocking
      plays on.
- [ ] **The taskbar**: Ubuntu's dock shows the film's progress on RVP's
      icon; Files' "Recent" lists a film RVP played.
- [ ] **Text to speech** (Accessibility): with speech-dispatcher installed,
      the voices list shows its voices and RVP reads its notices aloud.
- [ ] **A gamepad** (Accessibility > XInput) if you have one: an Xbox
      controller's A pauses, the pad seeks; with the mouse emulation the
      stick moves the pointer on X11 (Wayland does not let a program move
      it).
- [ ] **The trash**: delete a played file from the playlist (Delete Files);
      it is in Files' trash and restores to its folder.
- [ ] **File types** (Association, Apply to System): RVP's Open With entry
      lists the chosen types; your default player stays the default.
- [ ] **The remote**: Remote Control, "Add to Unified Remote" with Unified
      Remote Server installed; the phone's remote plays and pauses RVP.
- [ ] **A webcam or capture card** if you have one: Open > Webcam/Other
      Device lists it by name and plays it.
- [ ] **The screen** (Open > Screen Capture): in an X11 session it plays the
      desktop; in the Wayland session RVP says why it cannot.
- [ ] **A DVD drive** if you have one: a film DVD's menu shows and its
      buttons work; an audio CD plays (Open > Audio CD).
- [ ] **GPU checks** (only real hardware): the shaders and HDR tone mapping
      on an HDR file; Whisper on the GPU (Vulkan, or your CUDA build in
      `~/.config/rvp/whisper/cuda`); M18's RIFE and, in an X11 session with
      the Vulkan renderer, NVIDIA Optical Flow raising the rate; the GPU
      power budget's readings (Video page) with `nvidia-smi` beside it.
- [ ] **SVP 4 for Linux** with RVP's own smoothing: with SVP 4 Manager
      running SVP attaches to a file, and RVP's smoothing stays off while
      SVP's filter plays (and for SVP's first 8 s).
- [ ] **RIFE (TensorRT) in RVP** (after 2026-11-01; with RVP started from the terminal that exports
      `RVP_TENSORRT_MIRROR`): Video > Motion Smoothing's Download… (the
      consent to NVIDIA's terms, then the installation for the card,
      `sm120` on the RTX 50-series card), Calibrate… and Start (each engine
      measured; RIFE (TensorRT)'s engine built, about two minutes), then a
      1080p film with RIFE (TensorRT): the rate rises, the picture has no
      fine grid over the in-between frames (TensorRT 10.16's fault), and
      the Log page at Debug writes the playback's dropped frames and the
      card's load, clocks and temperature every 3 s to `rvp.log`.
- [ ] **Smoothing's other new parts**: "At a scene change" repeats a
      scene's last frame at a cut instead of morphing; a file starts and
      a seek resumes without a burst of dropped frames; "Step down after
      dropped frames over" and the drop at once to a lighter step.
- [ ] **Seeks with smoothing** (Time jump page, "Jumps with motion
      smoothing"): the arrows, a click on the seek bar and a drag show
      the target's frame at once, then the smoothed picture; `rvp.log` at
      Debug has `seek:` lines with both times. With the Vulkan renderer
      (through XWayland) the frame shows over RVP's native video window.
- [ ] **The rest since goal S**: the OSD Messages page's colours,
      outline and shadow on the volume notice; Stop, then Play in GNOME's
      media controls plays the same file on from where it stopped; each of
      PotPlayer's bundled skins (`Skins/` on the Windows partition) opens
      and lays out as on Windows, its labels sized to this system's fonts.

## Steps and what they record

| Step | Needs | PASS means |
|------|-------|------------|
| `check` | packages, cargo | all build packages present |
| `deps` | network | meson, aqtinstall and Qt 6.10.3 in the kit directory |
| `mpv` | `deps` | libmpv at the commits of RVP's Windows libmpv (mpv `b2c255c13e`, FFmpeg `37515cfd87`, libplacebo `a83aaeb4`) with VapourSynth loaded at run time, Lua and mpv's scripts, the disc libraries, GnuTLS and the audio filters' libraries, `fruc_vulkan`, and no `--enable-nonfree` |
| `mvtools` | `deps` | mvtools v24 built against the VapourSynth R73 headers |
| `build` | `mpv` | the three spikes build against that libmpv and Qt |
| `s2` | SVP's VapourSynth, `RVP_VPY` | with the `get_core` wrapper the script reaches about 60 fps; the unchanged script is expected to fail on VapourSynth R73 and is recorded in the detail |
| `s3` | graphical session | frameless window, menu outside the window, skin painted, video frames rendered; on a real GPU also no dropped frames (`mpv_frame_drop_timeline` in the log shows when drops happened) |
| `input` | ydotool, `ydotoold` | the window moved and resized by the dragged distance, and a right click on the 440x77 window opened a larger menu window |
| `s1` | SVP 4 Manager running | SVP inserted its VapourSynth filter and the frame rate rose |
| `m2build` | `mpv` | the RVP player builds against the kit's libmpv and Qt |
| `m2` | graphical session | RVP's functional self-test passes; after a restart the last file is reopened paused within 2.5 s of where it stopped; the usability measurements are in the detail |
| `m2input` | ydotool, `ydotoold`, `m2` | the RVP window moved and resized by the dragged distance, a double click entered fullscreen at the screen size and a second one restored the window, and a right click opened the menu window |
| `m2gpu` | hardware GPU | no dropped frames over `RVP_M2_SECONDS` in fullscreen and in a 1920x1080 window (the counts and a per-second timeline are in the log) |
| `m2svp` | SVP 4 Manager running | SVP's `@svp` filter appeared and RVP exited with status 0 |
| `m3` | graphical session | the functional self-test including M3's checks passes (playlist panel, albums, normalizer, volume above 100 %, key editor, second instance, aspect lock), and all three kill tests reopen the album's second entry paused within 5 s of the last position |
| `m4` | graphical session | the functional self-test passes with all of M4's and M5's checks (menus with the bound keys and check marks, a menu command, the settings window with every page, a setting that applies at once), and a portable copy kept its settings next to the executable |
| `m6` | graphical session, cargo | the functional self-test passes with all of M6's checks (control panel, colour, equalizer, shader, SVP diagnostic), the picture is upright, at most 2 frames more than in steady playback drop after the speed returns to 1x, a script slot raises the frame rate (SKIPPED without SVP's VapourSynth; PotPlayer's script too when `RVP_VPY` is set), and the headless video and equalizer tests pass |
| `m6gpu` | hardware GPU | no dropped frames over `RVP_M2_SECONDS` in fullscreen with colours, deband, a shader and the equalizer on |
| `m6svp` | SVP 4 Manager running | SVP attached to both windows (the second one on its `mpvSockets` endpoint), the frame rate reached 1.9 times the file's, and audio and video stayed within 0.1 s after each seek |
| `m7` | graphical session | the functional self-test on RVP Default with M7's checks, and the picture upright in the window and in fullscreen with the pointer over it |
| `m8package` | `mpv`, network once (the AppImage runtime) | the AppImage and the .deb were made |
| `m8` | graphical session, `m8package` | the AppImage's self-test passes; with the .deb installed: its files, a valid desktop entry, the registration for the media types with the default unchanged, the self-test, MPRIS (playing, metadata, Pause, Quit), the screen kept on while video plays (SKIPPED without GNOME's session manager on the bus), the crash logs, and, where the step installed it, its removal |
| `m8removed` | `sudo apt remove rvp` | nothing of the package is left |
| `m9` | graphical session, cargo; `RVP_M9_SCRIPTS`, `RVP_WINAMP_SKINS`, `RVP_VLC_SKINS` for their runs | the M9 self-test with mpv's OSC (the controller takes its click) and with each script folder, the classic and VLC self-tests with each skin, and the skin loaders' tests pass |
| `m10` | graphical session, sound server, cargo | the functional self-test passes with M10's steps (the second device follows, the recording starts and stops, the master volume answers with its level, Audio Stretching), the recording is a WAV whose sizes match, and the level tests pass |
| `m11` | graphical session, cargo | the M11 self-test passes (with the DVD folder played, not skipped), the dry-run file says "shut down" and the machine kept running, the self-test's file is in the step's trash with its `.trashinfo`, and the thumbnail and M11 tests pass |
| `m12` | graphical session, cargo | the M12 self-test and RVP's shaders pass, and M12's and the engine's headless tests pass |
| `m13` | graphical session, cargo; whisper.cpp optional | the M13 self-test passes (with subtitles from speech when whisper.cpp is set), the fixture's subtitles are unchanged, whisper.cpp hears the clip, and the subtitle tests pass |
| `m14` | graphical session, cargo | the M14 self-test passes on RVP Default (and on `RVP_SKIN`), the autostart entry is as it was, and the settings, translations, menus and synthetic skin tests pass |
| `m15` | graphical session, cargo; `RVP_URSERVER` optional | the M15 self-test passes; with Unified Remote Server: it runs, its web client connects, the remote's actions reach RVP, the token is not in RVP's log, the server's log has no errors; the control API's and the remote's tests pass |
| `m16` | graphical session, cargo, dvdauthor | the system has libdvdnav and libbluray, the M16 self-test shows and works both discs' menus, and the shell's navigation tests ran on them |
| `m18` | graphical session, cargo | the smoothing and GPU budget self-tests pass (MVTools with a VapourSynth runtime, else Blend; seeks skip the frames before the target and ask for the target's picture) and the smoothing tests pass |
| `m18trt` | NVIDIA card, `vapoursynth`, cargo; `RVP_TENSORRT_MIRROR` optional | TensorRT is installed for the card as RVP installs it, RVP's RIFE (TensorRT) script makes 1080p at 60 frames a second or more (720p and 1440p are in the detail), and `nvidia-smi` answers afterwards |
| `m3desktop` | `m2build` | the desktop entry was written |
| `report` | — | the Markdown report was written |

GPU checks run when an NVIDIA, AMD or Intel GPU is detected; set
`RVP_GPU_CHECKS=0` or `1` to override. On the owner's machine (GNOME 51
Wayland, NVIDIA 595.91.07) the S3 GPU check still reports about 6 dropped
frames per 20 seconds after the PipeWire fix; mpv's own output drops a few
too. See `docs/spikes/s3-qt-shell.md`.
