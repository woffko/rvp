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
- **Han characters on Windows** (goal V-W): Chinese mixed Yu Gothic
  UI's Japanese shapes with Microsoft YaHei or JhengHei UI on an English
  Windows; fixed on 2026-11-18 (the language's own UI font first). Look
  at Chinese once on Windows 11, where the controls' font is Segoe UI
  Variable.
- **The context menu's last item on Windows**: in the tour's grab on the
  host (2026-11-18, a 1440-pixel screen) About lies under the menu's
  frame, as in the VM's grabs before menus were capped to the screen: a
  flaw of the menu in Qt's FluentWinUI3 style, not of the cap. Look at
  it on the screen and find where the frame's height comes from.
- **SVP and a second window** (2026-11-18): SVP attached to the first
  window only after the second (`--svp-second`) had closed, 174 s later.
- The repository is public since 2026-11-18 with its history rewritten
  (D21): every commit has a new hash. Clones made before (the owner's
  Ubuntu boot, the kit's copies) must be cloned again; the private
  original is `woffko/rvp-private`.
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
