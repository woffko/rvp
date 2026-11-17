# Handoff: what comes next

State on 2026-11-17: every goal of `docs/PLAN.md` is done, goals U-W
and V-W (the Windows parts of U and V) last. Before them, on 2026-11-15:
the Ubuntu boot run (M6 to M18 on the owner's Ubuntu 26.10 boot, GNOME
Wayland, RTX 50-series card) is in `docs/test-runs/2026-11-15-ubuntu-boot.md`
and `docs/goals/goal-s-status.md`; the Linux audit that followed is
`docs/audits/linux.md`, and goal U fixed it (`docs/goals/goal-u-status.md`).
After goal U, on the same boot: the README for readers with screenshots
(`tools/screenshots/tour.sh`), a closed file's last frame no longer left
under the logo (3da5d59), the Unified Remote notes below, and goal V's
twelve interface languages.

## The next goals

Goals U-W and V-W, the Windows parts of U and V, ran on 2026-11-17 from
WSL on the Windows test VM (`docs/goals/goal-u-status.md` and
`goal-v-status.md`, "Windows"); no goal is planned after them. The
`/goal` texts of every goal are in PLAN.md section 10.

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

- **Windows' media controls in CI**: the "Opened" for the first 30 s was
  intermittent; since 29f9529 the state passes through a real change in
  the controls' first 30 s, and CI's `m8-smtc.ps1` passed at once there.
  Watch the next CI runs.
- **Han characters on Windows** (goal V-W): with Windows in English,
  Chinese mixes Yu Gothic UI's Japanese shapes with Microsoft YaHei or
  JhengHei UI; Japanese and Korean are even. The fix would put the
  language's own UI font first on Windows (as `FC_LANG` on Linux): the
  owner decides.
- **Menus on a real Windows desktop**: menus now take at most their
  screen's height and scroll (goal U-W). The VM's session-0 grabs still
  draw a menu's last rows under its frame, as before the change (its
  enter animation does not run in a window that is never exposed); look
  at a long menu on the host's screen once.
- **Host checks of goal U-W**, each with the owner's permission: RIFE
  (TensorRT)'s lighter step and the calibration on the RTX 50-series card, SVP's
  second window (`--svp-second`), and the OpenGL renderer's black picture
  after a closed file.
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
