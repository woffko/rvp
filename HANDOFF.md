# Handoff: what comes next

State on 2026-11-15: every goal of `docs/PLAN.md` up to goal U is done.
The Ubuntu boot run (M6 to M18 on the owner's Ubuntu 26.10 boot, GNOME
Wayland, RTX 50-series card) is in `docs/test-runs/2026-11-15-ubuntu-boot.md`
and `docs/goals/goal-s-status.md`; the Linux audit that followed is
`docs/audits/linux.md`, and goal U fixed it (`docs/goals/goal-u-status.md`).
After goal U, on the same boot: the README for readers with screenshots
(`tools/screenshots/tour.sh`), a closed file's last frame no longer left
under the logo (3da5d59), and the Unified Remote notes below.

## The next goals

Their `/goal` texts are in PLAN.md section 10 (PLAN.md wins over any
copy).

| Goal | What | Where |
|------|------|-------|
| U-W | Goal U's Windows part: the VM checks listed in goal-u-status.md's "For goal U-W" | WSL, the Windows test VM |
| V | M19, interface languages (D20): no language but English in the code, then twelve languages translated by agents and reviewed by back-translation | the Ubuntu boot (Xvfb) |
| V-W | Goal V's Windows part: the fit checks and the installer's languages on the VM | WSL, after goal V (with U-W or after it) |

## Starting on the Ubuntu boot

1. Log in to the GNOME Wayland session; `git -C ~/rvp pull`.
2. The local test material (never committed) is in `~/rvp-local`, and
   `~/rvp-local/kit-env.sh` sets the kit's variables (`source` it).
3. In a terminal inside the GNOME session (not over SSH): `cd ~/rvp`,
   start Claude Code, and give it the goal's text.

Never use sudo: the owner installs packages. Silent checks (Xvfb, no
sound) need no notice; anything that opens a window on the desktop or
plays sound is announced first, and GPU work is guarded with
`nvidia-smi` before and after.

## Starting in WSL

`git -C ~/rvp pull`, then the goal's text. The Windows test VM
is shared: ask every session that uses it (`ListAgents`) before a run.
Anything on the Windows host desktop needs the owner's permission each
time.

## Open items

- **Windows' media controls in CI** (`package-windows`, `m8-smtc.ps1`):
  now and then the session shows "Opened" instead of "Playing" for its
  first 30 s, until the next real change (a pause). Setting the same
  values again every 2 s (4504b38) did not help at b678257: equal values
  change nothing, so goal U-W looks for a real change once the session
  appears.
- Wayland: a second `rvp` cannot hand the window its activation token
  (needs Qt's private Wayland API); GNOME may say "RVP is ready" instead
  of raising it.
- `tools/linux-kit/sources.sh` mirrors neither vs-mlrt nor TensorRT.
- Not tried on hardware: an Xbox controller on Linux; GNOME without the
  AppIndicator extension (no tray).
- A crash log in the owner's state folder, `crash-20261115-052321-native.log`
  (c3f58dc, signal 11, no backtrace): its cause is unknown.
- Unified Remote on the owner's boot (not RVP's code): the owner's own
  server starts with the session, and RVP's remote is not installed in
  it yet (Remote Control page: enable, then Add to Unified Remote);
  custom remotes such as RVP's need Unified Remote's Full app. Its Power
  remote has no Lock on Linux (Unified Remote's own layout), and its
  mouse moves but does not click on Wayland: its uinput device sends a
  button's press and release in one frame (`evtest`, 2026-11-15), which
  libinput drops. Workarounds: GSConnect for the mouse, or a small proxy
  that splits the frames (not made; the owner chooses).
- The owner's look at the Unified Remote remote's design (postponed in
  goal Q).
- M3's one-week trial, if the owner still wants it.
