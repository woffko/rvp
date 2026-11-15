# Goal U status: the Linux audit's fixes

Started 2026-11-14 on the owner's Ubuntu 26.10 boot (GNOME 51 on Wayland,
RTX 50-series card, NVIDIA 615.71.09). The findings are those of
`docs/audits/linux.md` (H1 to H3, M1 to M16, L1 to L27); the work plan
is its last section. Goal U-W checks on the Windows VM what reaches
Windows ("For goal U-W" below).

## The owner's decisions (2026-11-14)

The owner answered the audit's questions in one round, each with the
recommended option:

1. **H1:** CI builds and caches whisper.cpp, MVTools and RVP's
   VapourSynth runtime for its Linux packages.
2. **H3:** the overlapping libraries move to `SYSTEM` (Depends); RVP's
   binary keeps its RPATH.
3. **M2:** with the media transport controls off, the screen kept on, the
   launcher's progress and pausing on lock still work; only MPRIS goes.
4. **M5, L12, L13:** the cases of several local accounts are fixed now.
5. **L15:** saved frames and recordings go to the XDG user folders
   (`user-dirs.dirs`), `~/Pictures/RVP` and `~/Videos/RVP` without them.
6. **L3:** the calibration's key is the content of the engines' files,
   not their size and time.
7. **M14:** the AUR package uses the system's VapourSynth with optdepends.
8. **m6svp:** the second window's check expects RVP's diagnostic (SVP's
   channel taken), as on Windows.

## Packages

| # | Package | Result |
|---|---------|--------|
| 0 | Start | The boot run's report, its status sections, the audit and goal U's entry committed with dbde0cd and c3f58dc, and pushed |
| 1 | Checks that pass without testing | M9, M13 fixed (below) |

## Package 1: checks that pass without testing

- **M9, `=PASS SKIPPED` counted as passed:** `kit_judge` (kit.sh) takes a
  required check only from a `=PASS` line that is not `=PASS SKIPPED`;
  `check-deb.sh`'s `selftest` takes the checks that must pass, and m16's
  requires `m16_menu_shows` and `m16_bd_menu_shows` (a missing libdvdnav
  or libbluray made M16 write `=PASS SKIPPED no libdvdnav` with
  `failures=0`).
- **M9, the disc tests:** `shell/src/disc/tests.rs` fails when
  `RVP_DVD_MENU`, `RVP_BD_MENU` or `RVP_DVD_REAL` names a disc and its
  library does not load (it skipped, which the boot run's soname miss
  would have passed). CI's test job installs libdvdnav4 and libbluray2.
- **M9, the library check:** `check-deb.sh` runs `ldd -r` (symbols too)
  with `LC_ALL=C` on `rvp` and every program beside it without
  `LD_LIBRARY_PATH`, on every library with the run path spelled out, and
  plain `ldd` on Python's extension modules (their symbols come from the
  interpreter); it also lists the Qt plugins RVP needs (`qt_plugins`).
  Against this boot's `/opt/rvp` it found M16's `_tkinter`
  (`libtcl9.0.so => not found`), fixed in package 2.
- **M9, the m8 install:** m8 compares the installed `rvp`'s version with
  the built .deb's: the same, or installed with `--allow-downgrades`, or
  FAIL (an older build passed every m8 check for the new one).
- **M9, the inhibit check:** m8 looks for RVP's own inhibitor among GNOME's
  session manager's (`GetInhibitors`, app ID `rvp`, idle flag 8), while
  playing and after the pause; `IsInhibited` is still logged. Another
  program's inhibitor made the old check pass.
- **M9, m15's server log:** m15 reads Unified Remote Server's own log
  (its HOME is the step's) and passes only when RVP's remote was loaded and
  used and no `(E)` or `(W)` line follows its loading
  (`m15_server_log`); the loader's "could not load layout: File not
  found" does not name the remote, so the old `grep RVP` missed it.
  Checked against this boot's log (clean) and a log with the layout error
  (not clean).
- **M9, the report:** `kit.sh report` includes m18trt's log.
- **M13, M12's window steps on Wayland:** the size dialog opens once (step
  8), the answer is 860x540 (the window was already 801x475, "near"
  800x500, so step 10 passed before the answer applied), and a size request
  counts as a change for `steady()`.
- **Checks:** `cargo xtask ci`; on the desktop (2026-11-15) kit steps m16
  PASS (27 checks, its menu checks required), m15 PASS (the server's log
  clean; the first run failed on this package's own inverted check value,
  fixed), m12's self-test 27 of 27 (`window_size_dialog_applies` 860x520;
  the step still fails on NVScaler, H2, package 3) and m8 PASS (14 checks:
  the installed .deb is the built one, and `idle_inhibit` found RVP's own
  inhibitor while playing and none paused, with `IsInhibited 8` already
  true from another program).
