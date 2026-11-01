# Handoff: the Ubuntu boot run (M6 to M18)

State on 2026-11-01: every goal of `docs/PLAN.md` is done, goal S (M17,
Linux completion) last (`docs/goals/goal-s-status.md`, commits up to
2b397da). What remains is one session on the owner's Ubuntu 26.10 boot of
the same PC (GNOME Wayland, RTX 30-series card): the acceptance of M6, M8 and
M17 and the parts of M9 to M18 that only real hardware and a person at
the desktop can check.

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
5. In a terminal inside the GNOME session (not over SSH): `cd ~/rvp`,
   start Claude Code, and give it the task below as a `/goal`.

## The task

The `/goal` text is PLAN.md section 10's "Ubuntu boot run for M6 to
M18"; it is copied here as it stood at 2b397da, and PLAN.md wins if they
differ.

```
/goal On the owner's Ubuntu 26.10 boot (GNOME Wayland, RTX 30-series card), run the Linux kit for milestones M6 to M18 of the private repository woffko/rvp in one session and record the results; this is the Ubuntu boot run that the acceptance of M6, M8 and M17 needs (docs/goals/goal-h-status.md, goal-j-status.md, goal-s-status.md), with M9's and M18's Linux side. Clone or pull the repository to ~/rvp and follow AGENTS.md (its Linux section) and tools/linux-kit/CHECKLIST.md, sections 1 to 10. The owner does the privileged steps the checklist lists (the packages `kit.sh check` prints, `ydotoold` running as root for m2input, SVP 4 Manager started for m2svp and m6svp, installing the .deb that step m8 names and removing it afterwards); never use sudo yourself and never handle NAS credentials. Set RVP_SKIN to BlackSpace_en.dsf from the PotPlayer folder on the Windows partition, mounted read-only, and RVP_VPY to PotPlayer's VapourSynth/motioninterpolation.vpy there; set the optional pieces section 10 names where the owner has them (RVP_M9_SCRIPTS, RVP_WINAMP_SKINS and RVP_VLC_SKINS with local samples, never committed; RVP_WHISPER_MODELS with ggml-tiny.bin; RVP_URSERVER with Unified Remote Server for Linux unpacked, never installed). Run `tools/linux-kit/kit.sh m17all` from a terminal inside the GNOME session (it builds libmpv, whisper.cpp's engine and RVP's VapourSynth runtime in ~/.cache/rvp-kit first); before each step that opens windows or plays sound, tell the owner what will appear and for how long (nobody touches the mouse or keyboard during m2input). Then the checklist's manual checks with the owner: section 10's (the tray with GNOME's AppIndicator extension, the screen lock, the dock's progress, recent files, text to speech, a gamepad if the owner has one, the trash, file types, the Remote Control page's Add to Unified Remote, a webcam and a DVD drive if the owner has them, the screen capture refused on Wayland) and the GPU's (the Vulkan renderer through XWayland with NVIDIA Optical Flow and RIFE at gpu-limit's power limit, the GPU power budget, Whisper on Vulkan and, with the owner's own build, CUDA, SVP's acceptance). If a step fails, find the cause in its log in the kit's log directory; fix RVP only when the cause is clear and the fix small, run `cargo xtask ci` and rerun the step; otherwise record it as an open item. Then record the results: `kit.sh report` writes docs/test-runs/<date>-<host>.md; add an "Ubuntu boot run" section to docs/goals/goal-s-status.md with each step's result (the GPU checks against goal D's 6 dropped frames per 20 s at 4K60, SVP's acceptance, the packages, MPRIS and the screen kept on, the new audio, video, subtitle and playlist functions, the disc menus, the remote, M9's skins and controllers, M18's smoothing) and, if their run has not happened, the M6 and M8 sections that the M6-M8 run would have written into goal-h-status.md and goal-j-status.md, with what the owner found in the checklist's manual checks. Commit with `git -c user.name=woffko -c user.email=2505149+woffko@users.noreply.github.com commit` and push. Done when the report and the status sections are committed and pushed. Do not: change SVP or system settings beyond the checklist's steps, commit PotPlayer or SVP files, third-party skins, scripts or captures, make the repository public, tag or release. Stop on blockers or when an owner decision is needed.
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

## Other open items (not this session's)

- The owner's look at the Unified Remote remote's design (postponed in
  goal Q).
- M3's one-week trial, if the owner still wants it.
