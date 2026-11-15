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
| 5 | The window thread | M3, M4 (all of it, package 6's part too), L5, L6 fixed (below) |
| 6 | Speech and the joystick | M4 in package 5; L20 fixed, not tried with a controller (below) |
| 7 | Paths, files and processes | M1, M5, M7, L7 to L16 fixed (below; L15 and M7 had no package) |
| 8 | Recording | M15 fixed (below) |
| 9 | Smoothing and TensorRT | M11, L1, L2, L3 fixed (below) |

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
  test with libmpv); the desktop's kit steps (m3's second instance through
  the channel, m11, m14) run with packages 8 and 9 (below).

## Package 8: recording

- **M15, RVP's sound recorded on Linux:** each `parec`'s bytes start a
  frame of their own (the 1 to 7 bytes a process ended with shifted every
  later sample); the main core's stream is `audio-client-name` "RVP" and
  the second audio device's "RVP (second device)" (the sound settings
  showed both as "mpv"), and the recording takes the stream by that name
  (`pulse::stream_of`; by order alone it moved to the second device's
  stream once mpv reopened the main one). Tests:
  `pulse::tests::the_main_stream_is_found_by_its_name`. The desktop's m10
  (the recording) runs with package 9 (below).

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
