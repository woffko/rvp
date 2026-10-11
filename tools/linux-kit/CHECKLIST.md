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
      RTX 30-series card.
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

## Steps and what they record

| Step | Needs | PASS means |
|------|-------|------------|
| `check` | packages, cargo | all build packages present |
| `deps` | network | meson, aqtinstall and Qt 6.10.3 in the kit directory |
| `mpv` | `deps` | libmpv (mpv `b2c255c13e`, FFmpeg and libplacebo releases) with VapourSynth loaded at run time |
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
| `report` | — | the Markdown report was written |

GPU checks run when an NVIDIA, AMD or Intel GPU is detected; set
`RVP_GPU_CHECKS=0` or `1` to override. On the owner's machine (GNOME 51
Wayland, NVIDIA 595.91.07) the S3 GPU check still reports about 6 dropped
frames per 20 seconds after the PipeWire fix; mpv's own output drops a few
too. See `docs/spikes/s3-qt-shell.md`.
