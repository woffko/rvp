# Linux test kit: owner checklist (Ubuntu 26.10, GNOME Wayland)

The kit builds libmpv, Qt and the goal B spikes in user space and records the
results of S1 (SVP attaches), S2 (VapourSynth interpolation) and S3 (Qt
shell) in `docs/test-runs/`. It never uses `sudo`: steps that need root are
listed here and printed by `kit.sh check`.

Everything the kit downloads or builds goes to `~/.cache/rvp-kit`.
Nothing is installed system-wide by the kit.

## 1. Before the session (as an administrator)

- [ ] Boot Ubuntu 26.10 and log in to a **GNOME Wayland** session (the
      default). Check with `echo $XDG_SESSION_TYPE` → `wayland`.
- [ ] NVIDIA driver installed and working: `nvidia-smi` lists the
      RTX 30-series card.
- [ ] Packages. Run `tools/linux-kit/kit.sh check`; it prints the exact
      `sudo apt-get install -y ...` line for whatever is missing. Run that
      line, then run `check` again until it passes.
- [ ] Rust for your user: install rustup from <https://rustup.rs>. The
      repository's `rust-toolchain.toml` selects the toolchain on first use.
- [ ] The repository: `git clone https://github.com/woffko/rvp.git ~/rvp`
      (private; log in first, for example with `gh auth login`).

## 2. Optional pieces (each step is skipped with a note when absent)

- [ ] **SVP 4 for Linux** (S1 and S2). Install it with SVP's own installer
      and its requirements (VapourSynth with Python, see SVP's Linux guide).
      Do not change SVP settings for the kit.
      - The kit looks for `libvapoursynth-script.so` under `/opt/svp`,
        `~/.local/share/SVP4`, `/usr/local/lib` and
        `/usr/lib/x86_64-linux-gnu`. If yours is elsewhere, set
        `VSSCRIPT_PATH=/full/path/libvapoursynth-script.so`.
- [ ] **PotPlayer's script** for S2. Mount the Windows partition read-only
      and point the kit at the file, for example:
      `export RVP_VPY="/media/$USER/Windows/Program Files/DAUM/PotPlayer/VapourSynth/motioninterpolation.vpy"`.
      The script needs the mvtools plugin (`libmvtools.so`). If VapourSynth
      does not autoload it, set `RVP_VS_PLUGINS=/full/path/libmvtools.so`.
- [ ] **A real video** for S1 and S3 (optional): `export RVP_VIDEO=/path/to/file.mkv`
      (a local file or one on the NAS mount). Without it the kit encodes a
      30-second 2160p60 HEVC test clip (NVENC when available, otherwise
      x265).
- [ ] **The BlackSpace skin** for S3: `export RVP_SKIN=/path/to/BlackSpace_en.dsf`
      (for example from `Skins/` of the PotPlayer folder on the Windows
      partition). Without it the shell paints no skin.
- [ ] **ydotool** for the input check (drags the frameless window with a
      real pointer). Needs ydotool 1.x: use the distribution package if
      `apt-cache policy ydotool` shows 1.x (Ubuntu 22.04 ships 0.1.8, which
      is too old), otherwise build it from
      <https://github.com/ReimuNotMoe/ydotool>. Then in a separate terminal
      run `sudo ydotoold --socket-perm 0666` for the
      duration of the check. Set the pointer acceleration to *Flat*
      (Settings → Mouse & Touchpad) while the check runs, and restore it
      afterwards. **The check moves the real mouse pointer for about
      30 seconds; do not touch the mouse while it runs.**

## 3. Run

From a terminal **inside the GNOME session** (not over SSH: the S3 window
needs the session's display):

```sh
cd ~/rvp
tools/linux-kit/kit.sh all
```

`all` runs `check deps mpv build s2 s3 input s1 report`. The first run
spends most of its time building FFmpeg and libmpv; later runs reuse the
builds. Steps can be run one by one, for example
`kit.sh s3 report`.

What appears on screen:

- `s3`: a 960x540 BlackSpace window that shrinks to 440x77, then plays the
  test video for about 20 seconds and closes. (On Wayland the scripted menu
  is refused without real input; the input step checks the menu.)
- `input`: the same window for about 30 seconds. The pointer scans the
  screen to find it, drags its title bar and its bottom-right corner,
  middle-clicks it (the window shrinks to 440x77) and right-clicks to open
  the menu; then the window closes. Only the mouse is used, no keys.
- `s1`: start SVP 4 Manager first. A plain mpv window plays the video for
  60 seconds; SVP should pick it up and switch on interpolation.

## 4. After the run

- [ ] `docs/test-runs/<date>-<host>.md` holds the system summary, the step
      results and the tail of each log. Raw logs stay in
      `~/.cache/rvp-kit/logs`.
- [ ] Check that the report contains nothing private (paths with personal
      names are fine to edit out), then commit and push it:
      `git add docs/test-runs && git commit -m "Record the Ubuntu 26.10 kit run" && git push`.
- [ ] Undo the temporary changes: stop `ydotoold`, restore the pointer
      acceleration profile.

## Steps and what they record

| Step | Needs | PASS means |
|------|-------|------------|
| `check` | packages, cargo | all build packages present |
| `deps` | network | meson, aqtinstall and Qt 6.10.3 in the kit directory |
| `mpv` | `deps` | libmpv (mpv `b2c255c13e`, FFmpeg and libplacebo releases) with VapourSynth loaded at run time |
| `build` | `mpv` | the three spikes build against that libmpv and Qt |
| `s2` | SVP's VapourSynth, `RVP_VPY` | the unchanged script fails or passes as recorded; with the compatibility wrapper it reaches about 60 fps |
| `s3` | graphical session | frameless window, menu outside the window, skin painted, video frames rendered; on a real GPU also no dropped frames |
| `input` | ydotool, `ydotoold` | the window moved and resized by the dragged distance, and a right click on the 440x77 window opened a larger menu window |
| `s1` | SVP 4 Manager running | SVP inserted its VapourSynth filter and the frame rate rose |
| `report` | — | the Markdown report was written |

GPU checks run when an NVIDIA, AMD or Intel GPU is detected; set
`RVP_GPU_CHECKS=0` or `1` to override.
