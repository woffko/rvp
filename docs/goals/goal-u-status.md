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
| 2 | Packages and CI | H1, H3, M8, M16, L4, L22, L23, L24 fixed (below) |
| 3 | OpenGL on Linux | H2 fixed and checked; M12 and L21 fixed (below) |
| 4 | Desktop integration | M2, M6, L17, L19 fixed; L18's tray click fixed, its activation token left (below) |

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

## Package 2: packages and CI

- **H1, CI's packages without their parts:** `package-linux` builds
  MVTools, whisper.cpp and RVP's VapourSynth runtime (kit steps `mvtools
  whisper vapoursynth`) when its cache misses, keyed by their build
  scripts and the kit's versions, and caches the results; `m8package`
  fails without either part unless `RVP_PACKAGE_SLIM=1`, and
  `check-deb.sh` checks that the package carries them
  (`package_parts`: whisper-cli runs, VSScript, MVTools, RIFE and
  vs-mlrt are there).
- **H3, bundled libraries the system's GTK loads:** `SYSTEM` takes every
  part of glibc (`libmvec`, `libnsl`, `libnss_*`, `libthread_db`,
  `libBrokenLocale`, `libc_malloc_debug`, `libcrypt`), GCC's OpenMP, and
  FriBidi, Brotli, LZ4, ACL, Kerberos (gssapi, krb5, k5crypto, com_err,
  krb5support, keyutils), Little CMS, libjpeg and WebP (with sharpyuv):
  their sonames hold across the distributions, so the .deb depends on
  them. ICU and libxml2 stay bundled (their sonames differ by release).
  AGENTS.md's packaging note says why.
- **M8, versions:** `0.0.1+git.<UTC time>.<hash>` (the .deb's
  `0.0.1+git<time>.<hash>-1`), so a later commit sorts higher.
- **M16, `_tkinter`:** removed with Tcl/Tk from the runtime's Python.
- **L4, VapourSynth's plugin folder:** `/dev/null/vapoursynth-plugins`, a
  path no one can make, instead of the build's.
- **L22, package contents:** every file 0644 or 0755 whatever the umask
  (`touch_all`), bundled libraries stripped (`--strip-unneeded`; rvp keeps
  its symbols for panics' backtraces), and Qt's QML modules RVP does not
  use left out (`QML_LEFT_OUT`: QtTest, Qt/test, QmlTime, Particles,
  LocalStorage, VectorImage, tooling and five Qt.labs modules), which
  keeps Qt Test, Quick Test, SQL and Particles out of `lib/`.
- **L23, curl:** in the .deb's Recommends and the PKGBUILD's optdepends;
  a download without curl says so.
- **L24, whisper's build folder:** made anew for each build.
- **H3, the container's check:** `check-deb.sh` runs the functional
  self-test again with `XDG_CURRENT_DESKTOP=GNOME` (`selftest_deb_gtk`),
  so Qt loads the gtk3 theme and the system's GTK under rvp's run path;
  `libraries_resolve`'s `ldd -r` of `libqgtk3.so` with the run path does
  the same for the symbols.
- **Checks (2026-11-15, this boot):** kit steps `vapoursynth`, `whisper`
  and `m8package` PASS (`rvp_0.0.1+git20261115052548.a45809a22-1`, 112 MB;
  the AppImage 141 MB). The runtime has no `_tkinter` and no build path
  (`strings`: only `/dev/null/vapoursynth-plugins`); `lib/` holds none of
  the libraries moved to `SYSTEM` (the .deb depends on libacl1, libbrotli1,
  libfribidi0, libgomp1, libgssapi-krb5-2, libjpeg8, liblcms2-2,
  liblz4-1, libwebp7, libwebpmux3...), libmpv is stripped (60 to 52 MB),
  every file is 0644 or 0755, and check-deb's library and plugin checks
  pass on the AppDir (`rvp` and 181 libraries). The AppDir's functional
  self-test under Xvfb with `XDG_CURRENT_DESKTOP=GNOME` passed. CI's
  `package-linux` (built on 22.04, checked on 26.10) is below.

## Package 3: OpenGL on Linux

The audit's plan had no package for L21 (the Vulkan window's handle);
it is here, with the video window. L15 goes to package 7, M7 too (it has
none either), and L25 to package 11.

- **H2, OpenGL ES on Wayland:** `rvp_setup_graphics` sets the default
  `QSurfaceFormat`'s renderable type to desktop OpenGL on Linux before
  `QGuiApplication` (`RVP_GL_ES=1` leaves the API to the driver), and
  M12's shader self-test checks it (`opengl_desktop`). On this boot
  (2026-11-15): kit step m12 PASS for the first time (self-test 27/0,
  shaders 15/0 with NVScaler, Vulkan 11/0), `GL_VERSION 4.6.0 NVIDIA
  615.71.09` on Wayland (it was `OpenGL ES 3.2`); the shader self-test
  through XWayland (`QT_QPA_PLATFORM=xcb`) 11 of 11 on 4.6.0 NVIDIA, and
  under Xvfb 11 of 11 on Mesa's 4.6 compatibility profile.
- **M12, a layer over the whole video:** when the holes leave no region,
  the Vulkan renderer's X11 window goes where no video shows (an empty
  `setMask` is "no mask" to Qt's xcb, which showed mpv's window whole
  over the layer). Not reproduced on purpose: fixed from the code.
- **L21, the window's handle:** a `QPointer`, deleted at exit after mpv
  let its window go; m12's Vulkan run ends cleanly with it.
- **Found on the way:** CI's `play_after_stop_resumes_the_file_that_played`
  had failed since 2026-11-12 on both systems (an album of three where it
  opened two): every harness of `crates/app/tests/bindings.rs` used one
  folder (the process and `Harness::new`'s line) and the parallel tests
  removed or loaded each other's state. Each harness has its own folder
  now. CI's `package-linux` of 6b8938e failed on code.videolan.org
  refusing connections (dav1d's clone), not on RVP.

## Package 4: desktop integration

- **M2, the media-controls switch:** the Linux desktop thread runs with
  the General page's media controls off too (`Options::mpris`, and
  `Desktop::mpris` at run time: MPRIS's objects and name come and go on
  the same connection), so keeping the screen on, the launcher's progress
  and pausing on lock work without MPRIS, as on Windows (the owner's
  answer). An MPRIS name that cannot be taken no longer stops the rest
  (a warning in the log). Checks: `crates/platform/tests/linux.rs`'s
  `without_mpris_the_screen_still_stays_on` (no MPRIS name, GNOME's
  Inhibit asked, then MPRIS on and off with the inhibit held), and kit
  step m8's `idle_inhibit_without_mpris` (RVP's own inhibitor with
  `general.media-controls` false and no MPRIS name).
- **M6, "Minimize to tray" without a tray:** the window hides only while
  the tray shows the icon (`icon.available`); a raise (MPRIS, a second
  `rvp`) shows a hidden window too. Ubuntu's GNOME has a tray
  (AppIndicator), so the case without one is fixed from the code.
- **L17, fullscreen on another display on Wayland:** the window's screen
  is set before the fullscreen request, which names its output. M12's
  `fullscreen_other_display` now also checks that the screen changed where
  two displays lie apart (mirrored ones share one place). On this boot's
  Wayland (2026-11-15): `screen DP-2 -> DP-1`, PASS.
- **L18, the tray on Wayland:** a click minimizes the active window and
  brings back any other (Wayland reports no minimized state). Left: the
  activation token a second `rvp` could hand over (GNOME may show "RVP is
  ready" instead of raising the window); Qt takes `XDG_ACTIVATION_TOKEN`
  only from its own start, and passing one to a running window needs Qt's
  private Wayland API.
- **Checks:** `cargo xtask ci`; `crates/platform/tests/linux.rs` (6 tests
  on a private bus); kit step m12 PASS on this boot (self-test 27/0, with
  the fullscreen's screen change). m8's `idle_inhibit_without_mpris` needs
  this package's .deb installed: package 11's m8 run.
- **L19, a skin with windows chosen on Wayland:** the switch says on the
  OSD that the skin's windows need X11 and RVP starts through XWayland for
  it from the next start (`prefer_x11` reads the skin at start).
