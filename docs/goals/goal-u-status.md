# Goal U status: the Linux audit's fixes

Started 2026-11-14, done 2026-11-15 on the owner's Ubuntu 26.10 boot
(GNOME 51 on Wayland, RTX 50-series card, NVIDIA 615.71.09). Result: **done on
Linux**. Every finding of `docs/audits/linux.md` (H1 to H3, M1 to M16, L1
to L27) is fixed with its check, but for three parts left with their
reasons: L18's activation token (Qt's private API), L25's source mirror
for vs-mlrt and TensorRT, and the owner's missing tab arrows, which did not
reproduce (the tabs' overlap the owner showed after package 10 is fixed).
Four more problems turned up and were fixed on the way: CI's bindings test
sharing its folder, an exit that waited for the speech thread, the
calibration core's end racing the process's, and code.videolan.org's
outage in CI. The work plan is the audit's last section; the final kit
run is in package 11 and `docs/test-runs/2026-11-15-ubuntu-boot.md`.
Goal U-W checks on the Windows VM what reaches Windows ("For goal U-W"
below).

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
| 5 | The window thread | M3, M4 (all of it, package 6's part too), L5, L6 fixed (below) |
| 6 | Speech and the joystick | M4 in package 5; L20 fixed, not tried with a controller (below) |
| 7 | Paths, files and processes | M1, M5, M7, L7 to L16 fixed (below; L15 and M7 had no package) |
| 8 | Recording | M15 fixed (below) |
| 9 | Smoothing and TensorRT | M11, L1, L2, L3 fixed (below) |
| 10 | Texts, the AUR package, code rules | M10, M14, L26, L27 fixed; the tab arrows' check made strict, the owner's case not reproduced (below) |
| 11 | Close | L25 fixed but its source mirror; m6svp as on Windows; the tabs' overlap fixed; the final kit run passed (below) |

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
  self-test under Xvfb with `XDG_CURRENT_DESKTOP=GNOME` passed.
- **CI (8e5acce, 2026-11-15):** `package-linux` built MVTools, whisper.cpp
  and the VapourSynth runtime on Ubuntu 22.04 (47 min the first time,
  cached since) and the Ubuntu 26.10 container check passed:
  `deb_installed` (0.0.1+git20261115054920.8e5accec2-1, with the new
  Depends), `libraries_resolve` (rvp and 177 libraries, `ldd -r`),
  `qt_plugins`, `package_parts`, `selftest_deb`, `selftest_deb_gtk` (the
  gtk3 theme and the system's GTK 3 under rvp's run path, built on 22.04:
  H3's case), `selftest_appimage` (92 checks each), `selftest_m11` (41)
  and `selftest_m16` (27, the menu checks required). Its first run (6b8938e)
  had failed on code.videolan.org refusing dav1d's clone: `kit_clone` now
  falls back to VideoLAN's GitHub mirror (1279108).

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
  this package's .deb installed: package 11's m8 run passed it ("RVP's
  inhibitor, no MPRIS name").
- **L19, a skin with windows chosen on Wayland:** the switch says on the
  OSD that the skin's windows need X11 and RVP starts through XWayland for
  it from the next start (`prefer_x11` reads the skin at start).

## Package 5: the window thread

- **M3, the master volume:** on Linux `pactl` runs on a thread of its own
  (`system_volume::change_later`); changes asked meanwhile wait together
  (steps add up, two mutes cancel) and the OSD shows the last one's text
  through the player's queue. Windows keeps its in-process Core Audio
  call. Test: `system_volume::tests::waiting_changes_add_up`.
- **M4, text to speech, all of it:** Linux's speech goes through
  speech-dispatcher's client library (`libspeechd.so.2`, loaded at run
  time) on a thread of its own with one connection: the voices are its
  exact names (`spd_list_synthesis_voices`; `spd-say -L`'s table had cut
  "Bishnupriya Manipuri+Adam" and "English (America)" at the first space),
  a new text cancels only RVP's own (`spd_cancel` on its connection;
  `spd-say -C` cancelled every client's, a screen reader's too), rate and
  volume are clamped, and nothing waits on the window's thread (the list
  was `waitForFinished(3000)` with an O(n^2) de-duplication of about 14,800
  lines). The Settings page reads the voices once the thread has connected
  (`Player.tts_state`), and M14's self-test waits for it and fails an empty
  list where speech-dispatcher answers (it passed on any list).
  `docs/coverage/linux.md` says so. Package 6 keeps only the joystick.
- **Found on the way, an exit that waited for good:** the speech thread
  waits on a condition variable for the whole run, and the `Speaker`
  holding it was a function's static: at exit its destructor (glibc's
  `pthread_cond_destroy`) waited for that waiter. M14's and M10's
  self-tests (both open the TTS page) finished their checks and never
  ended; the `Speaker` is never destroyed now. The kit's runs got a time
  limit for such a case (package 11).
- **Found on the way, the clipboard check:** M14's `m14_copy_elapsed_time`
  read the clipboard on the next tick; Wayland's answers a moment later
  (it failed once on this boot). It waits up to 2 s now.
- **L5, logind's power actions:** asked on a thread on Linux; polkit's
  password dialog no longer holds the window.
- **L6, "Add to Unified Remote":** the files are copied at once, the
  server's reload is asked on a thread and the OSD says how it went.
- **Checks (2026-11-15, this boot):** `cargo xtask ci`; kit steps m10 PASS
  (96 checks, `master_volume_answers` through the thread: "Master volume:
  100 %"), m15 PASS, and m14's self-test 49 of 50: `m14_voices_list` "ready,
  14805 voices" with their whole names, and `m14_copy_elapsed_time` failed
  twice with an empty clipboard. Wayland lets a window set the clipboard
  only with a recent input event of its own, and the owner was using the
  desktop meanwhile; the boot run's hands-off m14 passed it. Package 11
  runs m14 again with nobody at the keyboard.

## Package 6: the joystick

- **L20, Linux's joystick (`shell/cpp/xinput.cpp`):** with "act only when
  RVP has the focus" A no longer clicks in RVP's window while another
  program has the focus (its release still goes, so no press stays held),
  as on Windows; a pad moved straight from one side to the other is a
  press (the driver may send no centre between them); and only a device
  with at least four buttons (`JSIOCGBUTTONS`) is taken, so a laptop's
  accelerometer (`lis3lv02d`, axes alone) no longer holds the slot. Built
  and checked by `cargo xtask ci`; no controller was at hand on this boot
  to try it.

## Package 7: paths, files and processes

The audit's plan had M7 and L15 in no package; both are here.

- **M1, path lists split at `:`:** the command line's `/same` streams,
  `/sub` and `/aud` reach mpv's loadfile one entry each through the
  options' `-append` forms, and the external audio's folders through
  `change-list audio-file-paths append` once the core runs
  (`App::engine_started`) and on each change (libmpv's start options take
  no `-append`). Test: `crates/app/tests/m11.rs`'s
  `command_line_switches_set_the_first_file` with a stream and a subtitle
  whose names hold the separator (`:` on Linux, `;` on Windows).
- **M5, the instance channel:** on Unix in `$XDG_RUNTIME_DIR`
  (`rvp-<user>` there; the shared `/tmp` only without one); a channel's
  socket is removed only when nothing accepts on it, so a running RVP that
  answered late keeps its channel and main.rs's "The first one keeps the
  channel" holds; the state folder used when the settings cannot be read is
  in `$XDG_RUNTIME_DIR` or the process's own, never the shared
  `/tmp/rvp-state`. Tests: `instance::tests::channels_keep_the_self_test_apart`.
- **M7, peripheral batteries:** supplies of scope `Device` (a wireless
  mouse's, a gamepad's) are no system battery, and a USB-C charger online
  counts as power. Tests: `osd::tests`.
- **L7, new windows' zombies:** each `/new` child is waited for on a
  thread of its own.
- **L8, Qt's variables reaching other programs:** the variables RVP sets
  for itself before Qt starts (`rvp_set_env`: the platform, the scaling,
  VapourSynth's runtime) are left out of the programs it opens for the
  user: `Tools.open_url` (in QML's place of `Qt.openUrlExternally`, which
  passes them on) and `Tools.show_file` run `xdg-open` without them.
- **L9, relative XDG folders:** `XDG_CONFIG_HOME` (the autostart entry)
  and `XDG_DATA_HOME` (the desktop entries) are taken only when absolute,
  as the specification asks and the rest of RVP did.
- **L10, `Exec` escaping:** `registration::desktop_exec` doubles `%` and
  escapes a quoted `"`, `` ` `` and `$` with two backslashes (the string's
  escape comes first); the autostart entry uses it too. Tests:
  `registration::tests`.
- **L11, `recently-used.xbel`:** one update at a time in a process; a list
  RVP cannot read is left as it is (it was replaced by an empty one,
  dropping other programs' entries); a self-closing `<bookmark/>` is taken
  out alone; URIs are escaped as GLib's (`(` stays `(`: this boot's list
  had Nautilus's and RVP's entries for one file). Tests: `recent::tests`.
- **L12, another user's trash folder:** a trash folder that is a link or
  another user's is refused (and `.Trash/$uid` falls back to
  `.Trash-$uid`), as the specification and gio do. Tests: `trash::tests`
  (the tests take the process's own uid, not 1000: CI's runner is 1001).
- **L13, SVP's socket:** an endpoint is free only when nothing is there or
  it is a stale socket of the user's own ("permission denied", another
  user's socket in the sticky `/tmp`, counts as taken), and
  `/tmp/mpvSockets` is made sticky and open to every user. Test:
  `svp::tests::a_users_own_stale_socket_is_free`.
- **L14, `file://` URLs:** the shell's `file_url` escapes the path
  (`media::file_url`), and a backslash is a separator on Windows only
  (MPRIS's `xesam:url` too; on Linux `/C:/films` is a folder). Tests:
  `media::tests`.
- **L15, the user folders:** saved frames, recordings and audio recordings
  go to `user-dirs.dirs`'s folders (this boot's `~/Изображения/RVP`,
  `~/Видео/RVP`, `~/Музыка/RVP`), else `~/Pictures/RVP` and the others as
  before (the owner's answer); Windows keeps `%USERPROFILE%`. Tests:
  `storage::tests::user_folders_come_from_user_dirs`.
- **L16, `/cd:/dev/sr0`:** a device's path stays as it is; a drive letter
  takes its colon.
- **Checks:** `cargo xtask ci` (the unit tests above and m11's switches
  test with libmpv); on the desktop (2026-11-15, with packages 8 and 9)
  kit steps m3 PASS (`second_instance_adds`: the second RVP handed its file
  over through `$XDG_RUNTIME_DIR/rvp-kit-m3`, entries 3 to 4; 3 kills, 3
  resumes), m11 PASS (41) and m14 PASS (50 of 50, the clipboard check too
  with nobody at the keyboard).

## Package 8: recording

- **M15, RVP's sound recorded on Linux:** each `parec`'s bytes start a
  frame of their own (the 1 to 7 bytes a process ended with shifted every
  later sample); the main core's stream is `audio-client-name` "RVP" and
  the second audio device's "RVP (second device)" (the sound settings
  showed both as "mpv"), and the recording takes the stream by that name
  (`pulse::stream_of`; by order alone it moved to the second device's
  stream once mpv reopened the main one). Tests:
  `pulse::tests::the_main_stream_is_found_by_its_name`. On the desktop
  (2026-11-15) kit step m10 PASS (96 checks; the recording's WAV whole:
  "wav:ok 44 0").

## Package 9: smoothing and TensorRT

- **M11, the budget's lighter step after RIFE (TensorRT):** the step
  counts as a change for both RIFE engines, as the plan takes them to
  MVTools (`lighter_changes`; with RIFE (TensorRT) at Fast the budget went
  straight to Blend). This reaches Windows. Test:
  `smoothing::tests::the_lighter_step_takes_both_rifes_to_mvtools`.
- **L1, TensorRT's package files:** `libvstrt` and `trtexec` are copied
  beside and renamed over, as `unpack` does with NVIDIA's (written in
  place, a plugin a running RVP still maps could kill it with SIGBUS); the
  page's Remove stops a calibration that runs first. This reaches Windows.
- **L2, `nvidia-smi` at start:** on Linux the question waits until the
  NVIDIA card is awake (`gpu_power::nvidia_awake`), as D17 asks: a laptop's
  sleeping card was woken at every start.
- **L3, the calibration's key:** the engines' files by their contents
  (their size and SHA-256 of their first and last 64 KiB), no longer
  their times, and the hardware without the kernel's release (the owner's
  answer): a package or kernel update alone no longer runs the
  calibration again. The new key differs from the old one, so each
  machine calibrates once more. This reaches Windows (the files' part).
  Test: `calibration::tests::the_key_follows_the_engines_contents_not_their_times`.
- **Checks:** `cargo xtask ci`; on the desktop (2026-11-15) kit step m18
  PASS (MVTools' smoothing self-test 20/0, the GPU budget's 16/0, RVP's
  VapourSynth runtime with the new plugin folder); `nvidia-smi` read the
  card after each step (61 W, P1 after m18).

## Package 10: texts, the AUR package, code rules

- **M10, Settings on Linux:** the speech and XInput notes say what each
  system does ("on Linux speech-dispatcher with its voices", "on Linux the
  kernel's joystick devices"; they said Linux had neither); the voice's
  default is "The system's default" off Windows; Windows-only controls
  are hidden elsewhere (the speech's output device, the capture method's
  GDI and Desktop Duplication, "Clear and Rebuild the Icon Cache",
  "Default Programs…", which opened `ms-settings:`), with Linux's notes in
  their place (the X11 capture, the desktop's default applications); and
  x11grab gets the page's rate and pointer (`demuxer-lavf-o`, they were
  ignored). Russian texts in `qml_ru.ts`. `tests/coverage.rs` now also
  fails a line that says a Linux part comes later or is not there yet,
  which these two notes did.
- **M14, the AUR package:** every picture of the remote is installed
  (`mute.png` was missing, and Unified Remote then loads no layout),
  makedepends have `lld` and `qt6-tools` (`lrelease`), the system's
  VapourSynth with MVTools is in optdepends (the owner's answer), and
  `svp::vsscript_candidates` looks at Arch's `/usr/lib` and Fedora's
  `/usr/lib64` too, for VSScript's `.so.4` and `.so.0`. Test:
  `svp::tests::rvp_looks_for_its_own_runtime_and_never_svps`.
- **L26, `#[allow]`s without their reason:** `speech.rs`, `capture.rs`
  and `smoothing.rs`'s tests have one each now.
- **L27, stale comments:** `svp.rs`'s header (it said RVP points mpv at
  SVP's runtime) and `window.rs`'s "(goal S lists them)"; `tts.cpp`'s was
  rewritten in package 5, and `main.rs`'s "The first one keeps the
  channel" holds since package 7.
- **The settings tabs' arrows (the boot run's open item), not
  reproduced:** M12's `settings_tab_arrows` now opens the settings window
  at the owner's 848 px and requires both arrows inside the window. With
  the boot run's code and fresh settings they are (`arrowsEnd` 838 of 848,
  the Video group's tabs 1182 px in 606), and the owner's settings change
  no font or scale. A row that did not ask for its tabs' width
  (`Layout.preferredWidth: 0`) was tried and left out: it widened the tree
  beside it, and the TabBar's list then kept its current tab in view
  against the arrows (0 -> 11 -> 11). Likely cause, not shown: the
  owner's tree with longer open groups needs more width than the window
  has. Left for the owner: whether the new .deb shows them (package 11).
- **Checks:** `cargo xtask ci` (`tests/translations.rs`,
  `tests/coverage.rs` with the wider rule, the unit tests); on the desktop
  (2026-11-15) kit step m14 PASS (50 of 50: the new texts fit their
  controls in Russian) and M12's self-test with the settings window at
  848 px (`settings_tab_arrows`: both arrows inside, the row scrolled by a
  tab each way).

## Package 11: close

- **The owner's screenshot after package 10 (2026-11-15):** scrolled, the
  settings' page tabs were drawn over the search field on the left and
  under the arrows on the right, which showed only on hover (when their
  background covered the tabs). The TabBar clips its tabs now (e2fed86);
  M12's `settings_tab_arrows` requires it and grabs the window: the arrows
  show beside the last tab and the search field is clear.
- **L25, the kit:** every RVP run of `kit_run` and m6svp's two windows have
  a time limit (`KIT_RUN_TIMEOUT`, 900 s by default: M14's self-test once
  waited at exit and held the kit up); `step_mpv` runs in a subshell (its
  exports and `cd` no longer leak, `exit` ends it, not the kit) and fails
  when `./update`, `./rebuild` or `./install` fails (a failed rebuild had
  passed on the last libmpv); `input-check.py`'s drag lets the button go
  whatever fails; `build-vstrt-linux.sh` checks VapourSynth's headers by
  their SHA-256. Left: `sources.sh` mirrors neither vs-mlrt nor TensorRT
  (NVIDIA's archives come from NVIDIA's servers or
  `RVP_TENSORRT_MIRROR`; vs-mlrt's from its release, by SHA-256).
- **m6svp's second window (the owner's answer):** it runs
  `--selftest=svp --svp-second` and passes on RVP's diagnostic ("SVP did
  not attach", no `@svp` in its filters), as Windows' check does.
- **Found on the way, a crash at the calibration core's end:** the
  kit's m18trt measured well (720p 285, 1080p 136, 1440p 75 frames a
  second) and then failed: `rife_trt` ended with 132 (SIGILL). Under gdb
  the mpv core's thread was destroying the VapourSynth filter (VSScript,
  then vs-mlrt's TensorRT engine in `libnvinfer`) while the process
  exited: `calibrate::measure` sent `quit` and dropped its handle at once,
  and `mpv_destroy` does not wait. It waits for the core's `Shutdown` now
  (up to 10 s), which comes once the file and its filters are gone; RVP's
  own calibration takes the same path. m18trt passed twice after it
  (`rife_trt exit 0`, which its log now records).
- **Found on the way, M10's Audio Stretching check:** since package 5 the
  master volume's OSD text comes from its thread, and it arrived after
  Shift+J's "Audio Stretching : 101%" and took the OSD over (m10 failed
  twice in the final run). The self-test gives the volume's answer a
  second first.
- **Found in CI, Windows:** three of package 7's tests failed on CI's
  Windows (the instance channel's expected path with `/` where
  `Path::join` gives `\`, the USB-C charger's folder name with a `:`, and
  user-dirs.dirs's Unix paths): fixed in 7c652aa, whose Windows jobs pass.
  `package-linux` of 9e8cc46 rebuilt the packages' parts (vs-mlrt's build
  now checks VapourSynth's headers) and its Ubuntu 26.10 container check
  passed.
  `package-windows`' media-controls check (`m8-smtc.ps1`) failed in three
  of five runs from 97cb380 on (it had passed in every run before): RVP
  opened `clip-a.mkv`, the session showed its title but the status
  "Opened" for 30 s, so `playing_title` and `buttons_playing` failed,
  while every later step (Pause, Play, Next, Previous, Stop) passed.
  f83ab88 and 7c652aa passed it with the same code for Windows, and its
  rerun failed again. The title reached the session and the status did
  not: a status sent before Windows' session manager had taken the
  window's controls in is lost, and `Controls::show` never sent an
  unchanged one again (goal U likely made the first update come earlier).
  The controls now send their state again each second in their first 10
  seconds (`media_controls.rs`, Windows only; built and checked by CI's
  Windows jobs only, since this boot builds no Windows code). Goal U-W
  runs `m8-smtc.ps1` and `m8-all.ps1` on the VM a few times.

### The final run (2026-11-15, this boot, the .deb of 444110e installed)

| Step | Result |
|------|--------|
| m2build | PASS |
| m8 | PASS, 15 checks: the installed .deb is the built one; MPRIS; `idle_inhibit` (RVP's own inhibitor while playing, none paused); `idle_inhibit_without_mpris`; the crash logs |
| m2 | PASS |
| m3 | PASS (the second instance through `$XDG_RUNTIME_DIR`, 3 kills, 3 resumes) |
| m9 | PASS (OSC 17/0, ModernZ 15/0, uosc 17/0, four VLC themes 10/0 each; Winamp SKIPPED: no skins without the Windows partition) |
| m10 | PASS after the Audio Stretching wait above (96/0, the recording whole) |
| m11 | PASS (41/0) |
| m12 | PASS (self-test 27/0, shaders 15/0 with NVScaler, Vulkan 11/0) |
| m13 | PASS (17/0, whisper on) |
| m14 | PASS (50/0; skin SKIPPED: no BlackSpace without the Windows partition) |
| m15 | PASS (12/0, Unified Remote's log clean) |
| m16 | PASS (27/0) |
| m18 | PASS (smoothing 20/0, GPU 16/0) |
| m6svp | PASS: the first window attached to SVP in 1.7 s and reached its rate; the second showed RVP's diagnostic ("another player holds /tmp/mpvsocket, so this window listens on /tmp/mpvSockets/<pid>") |
| m18trt | PASS after the calibration core's fix above (720p 307, 1080p 145, 1440p 71 frames a second; `rife_trt exit 0`) |

`nvidia-smi` answered after every step (the card at its own 300 W limit).

## For goal U-W

The Windows VM cannot be reached from this boot (it runs on the same PC's
Windows). These changes of goal U reach Windows; goal U-W builds the
Windows bundle at goal U's last commit and runs the VM's checks below.

| Change | Package | What it changes on Windows | VM check |
|--------|---------|----------------------------|----------|
| Disc tests fail without their library when their disc is given | 1 | `shell/src/disc/tests.rs` | `m16-all.ps1` (the libraries are beside the bundle) |
| M12's window-size steps (860x540, the dialog once) and the fullscreen's screen check | 1, 4 | `SelfTestM12.qml` | M12's self-test on the VM (one screen: the screen part is skipped) |
| Package versions with the commit's time | 2 | the NSIS installer's and the ZIP's names and `VERSION` | `m8-all.ps1` (install, uninstall, the portable ZIP) |
| `touch_all` sets 0644/0755 | 2 | the Windows bundle's files' modes (ZIP entries) | `m8-all.ps1` |
| curl's missing-program message | 2 | `tensorrt.rs` (System32's curl exists) | `m18-all.ps1` |
| `requestRaise` shows a hidden window; the tray hides only when its icon shows | 4 | `Main.qml`, `Tray.qml` | `m14-all.ps1` (the tray, a second instance) |
| The speech page's state (`tts_state`) and its Timer; M14's voices check | 5 | `Settings.qml`, `SelfTestM14.qml` (Windows always "ready") | `m14-all.ps1` |
| M14's clipboard check waits up to 2 s | 5 | `SelfTestM14.qml` | `m14-all.ps1` |
| Path lists one entry each (`-append`, `change-list`) | 7 | `/same`, `/sub`, `/aud` and the external audio folders | `m11-all.ps1` (`tests/m11.rs` with `;` in the names) |
| `file_url` escapes; MPRIS's URL code keeps `\` as a separator on Windows only | 7 | the URLs QML opens, `media::file_url` | `m12`'s and `m13`'s folder dialogs, `m11-all.ps1` |
| `Tools.open_url` in QML's place of `Qt.openUrlExternally` | 7 | `QDesktopServices::openUrl` as before | `m14-all.ps1` (the Settings' folder buttons by hand) |
| New windows are waited for | 7 | `/new`'s child | `m11-all.ps1` |
| `/cd:D` | 7 | unchanged result `D:` | `tests` of `sources.rs` |
| `desktop_exec` escaping | 7 | Linux only in use | none |
| XDG user folders | 7 | Windows keeps `%USERPROFILE%\Pictures` etc. | `m12`'s capture (the default folder) |
| `audio-client-name` "RVP" and "RVP (second device)" | 8 | WASAPI's session name | `m10-all.ps1` (`tests/second_device.rs`, the recording) |
| The lighter step for RIFE (TensorRT); TensorRT's files renamed into place; Remove stops a calibration | 9 | `smoothing.rs`, `tensorrt.rs` | `m18-all.ps1`, and RIFE (TensorRT) on the host with permission |
| The calibration's key by content | 9 | one more calibration per machine | `m18-all.ps1` (`--calibrate`) |
| Settings: Windows-only controls kept on Windows, new texts | 10 | `Settings.qml` (Windows' texts unchanged) | `m14-all.ps1` (the texts fit) |
| The page tabs clipped; M12's tab-arrow check at 848 px | 10 | `Settings.qml`, `SelfTestM12.qml` | M12's self-test |
| m6svp's second window and `--svp-second` | 11 | `Main.qml`'s svp self-test option | the host's SVP check, with permission |
| The calibration core waits for its shutdown | 11 | `crates/engine/src/calibrate.rs` (RVP's calibration, `render_vf`'s users) | `m18-all.ps1` with `--calibrate`, and RIFE (TensorRT) on the host with permission |
| M10's self-test waits a second before Audio Stretching | 11 | `Main.qml` | `m10-all.ps1` |
| The media controls send their state again each second in their first 10 s | 11 | `shell/src/media_controls.rs` (Windows only) | `m8-smtc.ps1` and `m8-all.ps1` on the VM, a few times |
