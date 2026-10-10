# Linux test kit: owner checklist (Ubuntu 26.10, GNOME Wayland)

The kit builds libmpv, Qt, mvtools and the goal B spikes in user space and
records the results of S1 (SVP attaches), S2 (VapourSynth interpolation) and
S3 (Qt shell) in `docs/test-runs/`. It never uses `sudo`: steps that need
root are listed here and printed by `kit.sh check`.

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
| `report` | — | the Markdown report was written |

GPU checks run when an NVIDIA, AMD or Intel GPU is detected; set
`RVP_GPU_CHECKS=0` or `1` to override. On the owner's machine (GNOME 51
Wayland, NVIDIA 595.91.07) the S3 GPU check still reports about 6 dropped
frames per 20 seconds after the PipeWire fix; mpv's own output drops a few
too. See `docs/spikes/s3-qt-shell.md`.
