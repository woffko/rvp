# Handoff: the Ubuntu boot run (M6 to M18)

State on 2026-11-11: every goal of `docs/PLAN.md` is done, goal S (M17,
Linux completion) last on 2026-11-01 (`docs/goals/goal-s-status.md`);
the owner's requests since then are listed below ("After goal S"). What
remains is one session on the owner's Ubuntu 26.10 boot of the same PC
(GNOME Wayland; an RTX 50-series card since 2026-11-07, an RTX 30-series card before):
the acceptance of M6, M8 and M17 and the parts of M9 to M18 that only
real hardware and a person at the desktop can check.

## Starting on the Ubuntu boot

1. Boot Ubuntu 26.10 and log in to the GNOME Wayland session.
2. Prepare as `tools/linux-kit/CHECKLIST.md` section 1 says (the owner,
   as an administrator): `nvidia-smi` lists the card; git and the GitHub
   CLI installed, `gh auth login` (the repository is private); rustup for
   your user; Claude Code installed.
3. `git clone https://github.com/woffko/rvp.git ~/rvp` (or `git -C ~/rvp
   pull`).
4. The local test material, which is never committed: mount the Windows
   partition read-only (the Files app does) and unpack
   `Users/<user>/rvp-test/linux-boot/rvp-local.tar` from it into `~/rvp-local`
   (`mkdir -p ~/rvp-local && tar -C ~/rvp-local -xf .../rvp-local.tar`): the
   sample mpv scripts (uosc, ModernZ), classic Winamp skins and VLC themes,
   Unified Remote Server for Linux unpacked from its .deb, whisper.cpp's
   tiny model and a speech clip. Then, in the terminal the session will
   use:

   ```sh
   export RVP_M9_SCRIPTS=~/rvp-local/scripts RVP_WINAMP_SKINS=~/rvp-local/winamp \
       RVP_VLC_SKINS=~/rvp-local/vlc RVP_URSERVER=~/rvp-local/ur \
       RVP_WHISPER_MODELS=~/rvp-local/m13/models RVP_SPEECH_CLIP=~/rvp-local/m13/speech.wav
   ```

   `RVP_SKIN` and `RVP_VPY` come from the PotPlayer folder on the same
   mount (the task below names them). Each step that lacks its piece is
   skipped with a note.

   NVIDIA's Linux wheels for RIFE (TensorRT) (TensorRT 10.15.1.29, CUDA
   13.2's runtime and NVRTC, 3.8 GB, never committed) are on the same
   mount in `Users/<user>/rvp-test/linux-boot/trt-linux`; copying them to
   the Linux disk first reads them once:

   ```sh
   mkdir -p ~/rvp-local/trt-linux && cp /path/to/mount/Users/<user>/rvp-test/linux-boot/trt-linux/*.whl ~/rvp-local/trt-linux/
   export RVP_TENSORRT_MIRROR=file://$HOME/rvp-local/trt-linux
   ```
5. Before the GPU steps (as an administrator, CHECKLIST.md section 10):
   the card's power limit at 90 % of its default
   (`nvidia-smi --query-gpu=power.default_limit --format=csv`, then
   `sudo nvidia-smi -pl WATTS`), and back to the default after the
   session.
6. In a terminal inside the GNOME session (not over SSH): `cd ~/rvp`,
   start Claude Code, and give it the task below as a `/goal`.

## The task

The `/goal` text is PLAN.md section 10's "Ubuntu boot run for M6 to
M18"; it is copied here as it stood on 2026-11-11, and PLAN.md wins if
they differ.

```
/goal On the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 50-series card), run the Linux kit for milestones M6 to M18 of the private repository woffko/rvp in one session and record the results; this is the Ubuntu boot run that the acceptance of M6, M8 and M17 needs (docs/goals/goal-h-status.md, goal-j-status.md, goal-s-status.md), with M9's and M18's Linux side and what came after goal S (HANDOFF.md). Clone or pull the repository to ~/rvp and follow AGENTS.md (its Linux section), HANDOFF.md and tools/linux-kit/CHECKLIST.md, sections 1 to 10. The owner does the privileged steps the checklist lists (the packages `kit.sh check` prints, `ydotoold` as root for m2input, SVP 4 Manager started for m2svp and m6svp, the .deb that m8 names installed and removed afterwards, the card's power limit at 90 % of its default before GPU steps and back after); never use sudo yourself and never handle NAS credentials. Set RVP_SKIN to BlackSpace_en.dsf and RVP_VPY to VapourSynth/motioninterpolation.vpy from the PotPlayer folder on the Windows partition, mounted read-only; set the optional pieces section 10 names (RVP_M9_SCRIPTS, RVP_WINAMP_SKINS, RVP_VLC_SKINS, RVP_WHISPER_MODELS, RVP_URSERVER from ~/rvp-local, and RVP_TENSORRT_MIRROR at the Linux wheels in Users/<user>/rvp-test/linux-boot/trt-linux; never committed). Run `tools/linux-kit/kit.sh m17all` from a terminal inside the GNOME session (it builds libmpv, whisper.cpp's engine and RVP's VapourSynth runtime with vs-mlrt in ~/.cache/rvp-kit first, and m18trt installs TensorRT and measures RIFE (TensorRT)); before each step that opens windows, plays sound or loads the GPU, tell the owner what will happen and for how long (nobody touches the mouse or keyboard during m2input); check `nvidia-smi` before and after GPU work and stop everything if the card is lost. Then section 10's manual checks with the owner, including the GPU's (the Vulkan renderer through XWayland with NVIDIA Optical Flow, RIFE on Vulkan and RIFE (TensorRT) with its download, calibration and a 1080p film free of grid artifacts, seeks with smoothing, scene cuts, the GPU power budget and the Debug log's card lines, Whisper on Vulkan and, with the owner's build, CUDA, SVP's acceptance) and the desktop's. If a step fails, find the cause in its log in the kit's log directory; fix RVP only when the cause is clear and the fix small, run `cargo xtask ci` and rerun the step; otherwise record it as an open item. Then `kit.sh report` writes docs/test-runs/<date>-<host>.md; add an "Ubuntu boot run" section to docs/goals/goal-s-status.md with each step's result (the GPU checks against goal D's 6 dropped frames per 20 s at 4K60, RIFE (TensorRT)'s rates, SVP's acceptance, the packages, MPRIS and the screen kept on, M9 to M18's functions) and, if their run has not happened, the M6 and M8 sections for goal-h-status.md and goal-j-status.md, with what the owner found in the manual checks. Commit with `git -c user.name=woffko -c user.email=2505149+woffko@users.noreply.github.com commit` and push. Done when the report and the status sections are committed and pushed. Do not: change SVP or system settings beyond the checklist's steps, commit PotPlayer, SVP or NVIDIA files, third-party skins, scripts or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
```

## What the session should know first

- `docs/goals/goal-s-status.md`, "Findings" and "Results": what the
  shared Ubuntu VM showed. Two of its findings matter on the boot too:
  GNOME maximizes a first window that covers most of the screen, and
  Wayland reports no free area to a program.
- The VM had no GPU, no SVP and no `ydotoold`: `m2gpu`, `m6gpu`,
  `m2svp`, `m6svp`, `m2input` and M18's GPU modes have not run on Linux
  yet.
- On the boot the kit runs as the desktop's own user, so the screen-
  keeping check (`idle_inhibit`, GNOME's session manager) runs there; on
  the VM it was skipped (the kit's user had a bus of its own).
- GPU runs guard the card as AGENTS.md says (`nvidia-smi` before and
  after; on a lost GPU stop and tell the owner).

## After goal S (2026-11-02 to 2026-11-11)

Built and checked on both systems (D19), but on Linux only in WSL and on
the Ubuntu VM, which have no NVIDIA card (WSL's libmpv runs no
VapourSynth filter). CHECKLIST.md section 10 has their checks; the
commit messages have the details.

- The log (Log page, `log.level`, `log.folder`): the playback's dropped
  frames each second and, at Debug, the card's load, clocks, temperature
  and P-state every 3 s through NVML (f133080, 180892a, de1d535).
- Motion smoothing: the drop window and the step down at once (f133080,
  3fc8d19); MVTools asking for 16 frames at once and the original first
  inside the script (ee44fe9, 56e89f6); the calibration and its window
  (d7d028f, cc9c0ab); the filter set before playback starts through mpv's
  `on_preloaded` hold (1d4413b); scene cuts that repeat a scene's last
  frame (93a5679); seeks that skip the frames before the target and show
  the target's frame at once (9fffafd).
- RIFE (TensorRT) on Windows and Linux (6895002, 960e759): NVIDIA's
  runtime downloaded on request after the consent to NVIDIA's terms
  (Linux: NVIDIA's wheels; `libvstrt.so` and `trtexec` built by
  `tools/vapoursynth/build-vstrt-linux.sh` in the kit's `vapoursynth`
  step), another card's installation noticed (1d4413b), and TensorRT
  10.15.1, since 10.16 computes RIFE wrongly on the RTX 50-series card (d683fe5).
  It has never run on Linux with a card: the kit's new `m18trt` step is
  its first run. On 2026-11-11 the wheels in `trt-linux` matched their
  pinned sizes and SHA-256, they installed for `sm120` from there (WSL,
  `rife_trt ... install`), and the Ubuntu VM's `vapoursynth` step built
  `libvstrt.so` and `trtexec` (after a fix for CMake 3.22).
- PotPlayer's 22 bundled skins in layout, fullscreen and switching
  (6bf54b5, ee44fe9: labels sized to their texts with this system's
  fonts).
- The OSD's colours, outline and shadow (b962e6a); the remote's mute
  button (1ff656f); Play after Stop brings back the same file (7a6960a).

## Other open items (not this session's)

- The owner's look at the Unified Remote remote's design (postponed in
  goal Q).
- M3's one-week trial, if the owner still wants it.
