# Linux audit (2026-11-14)

Two independent read-only audits of RVP's Linux side ran on the owner's
Ubuntu 26.10 boot after the Ubuntu boot run, at `c3f58dc`:

- **A**: a Claude subagent, split into four parts (platform and shell
  Rust, C++ and QML, the app and engine crates, packaging, kit and CI).
- **B**: another agent, given the same brief, which also compared each
  Linux path with its Windows counterpart.

Both read the code, the kit's built packages, the installed `/opt/rvp`
and the run's logs. Neither changed or ran anything. The brief is
reproduced in "The brief" at the end.

This file merges the two. Every finding below was checked again in the
code at `c3f58dc`. It is marked:

- **confirmed**: the code does what the finding says.
- **likely**: the code is confirmed but the failure needs a desktop run.

Claims that did not hold are listed under "Rejected" with the reason.
Line numbers are from `c3f58dc`. **Win** marks a fix whose code also runs
on Windows; per D19 it needs the Windows VM's check too.

The work plan at the end groups the findings into packages for goal U
(`docs/PLAN.md`). Goal U fixed them on 2026-11-15
(`docs/goals/goal-u-status.md` says how each was checked and what was
left); goal U-W checks on Windows those that reach it.

## High

### H1. CI's Linux packages lack whisper.cpp and RVP's VapourSynth runtime

- **Sources:** A H1, B H5. Confirmed.
- **Where:**
  - `.github/workflows/ci.yml` `package-linux` runs only `kit.sh deps mpv`
    and `kit.sh m8package`.
  - `tools/linux-kit/kit.sh:1244-1246`: `m8package` adds `--whisper` and
    `--vapoursynth` only when earlier steps built them.
  - `tools/packaging/check-deb.sh` checks both parts only `if` they are
    present.
- **Failure:** every CI artifact silently ships without `bin/whisper/` and
  `bin/vapoursynth/`:
  - No subtitles from audio.
  - Smoothing is Blend only (no MVTools, RIFE or TensorRT).
  - The Ubuntu 26.10 container check never tests either part.
  - AGENTS.md's M17 note ("The Linux packages carry whisper.cpp ... and
    RVP's own VapourSynth runtime") is false for them.
- **Fix:**
  - CI builds `mvtools whisper vapoursynth` and caches their output
    folders.
  - `m8package` and `check-deb.sh` fail when a part is missing, unless a
    flag says the package is meant without it.
  - First fix M16, which the runtime would then trip.

### H2. On Wayland with NVIDIA, Qt gives RVP OpenGL ES, and NVScaler fails

- **Sources:** A M8, B H1; open item 4 of the boot run. Confirmed that
  RVP asks for no renderable type; that desktop OpenGL fixes it is likely.
- **Where:** `shell/cpp/shell.cpp:621-624`. `rvp_setup_graphics` only
  calls `QQuickWindow::setGraphicsApi(OpenGL)` and sets no default
  `QSurfaceFormat`.
- **Failure:**
  - Qt's Wayland EGL integration with the default renderable type took an
    OpenGL ES 3.2 context: `GL_VERSION='OpenGL ES 3.2 NVIDIA ...'` in
    today's m12.
  - There libplacebo has no `rgba32f`, so `NVScaler.glsl` fails. Every
    shader with `//!FORMAT rgba32f` fails the same way.
  - Under X11 and XWayland (GLX) the context is desktop OpenGL.
- **Fix:**
  - On Linux only (`#ifdef Q_OS_LINUX`), before `QGuiApplication`, set a
    default format with `setRenderableType(QSurfaceFormat::OpenGL)`.
  - Keep a way back for EGL drivers without desktop GL, for example an
    environment variable the self-test can set.
  - Check with `QSG_INFO=1` on NVIDIA Wayland (m12 with the shaders), X11
    (`QT_QPA_PLATFORM=xcb`) and Xvfb/Mesa (check-deb).

### H3. The packages bundle libraries that the system's GTK and glibc load too

- **Sources:** A M14 and M15, B H2. The overlap is confirmed in
  `/opt/rvp/lib`; a crash is likely, not observed.
- **Where:**
  - `tools/packaging/package.py:360-374`: `SYSTEM` has `libm` but not
    `libmvec`.
  - `rvp`'s `DT_RPATH` (`$ORIGIN/../lib`, `--disable-new-dtags`) applies to
    every library loaded under it.
- **Failure:**
  - **glibc:** `/opt/rvp/lib/libmvec.so.1` is libc6's own file, reached
    through libvorbis and x265. It imports GLIBC_PRIVATE symbols and
    breaks after the system's glibc moves on.
  - **The GTK theme's libraries:** since `c3f58dc` the gtk3 theme loads
    the system's GTK 3, pango and gdk-pixbuf, and their loaders. Through
    the executable's RPATH, or because a copy is already loaded by soname,
    they get RVP's bundled copies of:
    - libfribidi, libbrotlicommon and libbrotlidec
    - liblz4 and libacl
    - libgssapi_krb5, libkrb5, libk5crypto, libcom_err, libkrb5support
      and libkeyutils
    - liblcms2, libjpeg.so.8, libwebp and libwebpmux
    - libgomp
  - A package built on 22.04 (CI) and run on a newer system hands those
    libraries 22.04's copies. This is the same mechanism as the libnettle
    crash goal S fixed.
  - It has not shown yet for two reasons:
    - The kit's packages were built on 26.10.
    - CI's container sets no `XDG_CURRENT_DESKTOP`, so Qt never loads the
      gtk3 theme there.
- **Fix:**
  - Add the glibc parts to `SYSTEM`: `libmvec`, `libnsl`, `libnss_*`,
    `libthread_db`, `libBrokenLocale` and `libc_malloc_debug`.
  - Move the libraries listed above to `SYSTEM`, which puts them in
    Depends, where their sonames are stable across distributions.
  - Keep bundled those whose sonames differ between releases (ICU,
    libxml2), which cannot clash.
  - Add a check-deb self-test with `XDG_CURRENT_DESKTOP=GNOME`, so the
    container loads the theme.
  - The alternative, RUNPATH on `rvp`, changes AGENTS.md's packaging rule
    (question 2).

## Medium

### M1. Subtitle, audio and `/same` path lists are split at `:` on Linux (Win)

- **Sources:** A M1, B M17. Confirmed.
- **Where:**
  - `crates/app/src/sources.rs:964-966`, `:990-1005`: `external-files`,
    `sub-files` and `audio-files` are joined with `:`.
  - `crates/app/src/playback.rs:334-341`: `audio-file-paths`.
- **Failure:**
  - `/sub="/films/Alien: Covenant.srt"` reaches mpv as two paths.
  - Any URL in these options breaks at `https:`.
- **Fix:** one path per entry:
  - The `-append` options through mpv commands (`change-list NAME append`),
    since `mpv_set_option_string` knows no `-append`.
  - Or each path quoted with mpv's `%N%` form.
  - Test it with a name holding `:` and with a URL.

### M2. On Linux, the media-controls switch also turns off keeping the screen on, the launcher's progress and pausing on lock

- **Sources:** A M2, B M1. Confirmed.
- **Where:**
  - `shell/src/player.rs:1241-1251` and `shell/src/player/m14.rs:882-895`:
    `rvp_platform::Desktop` exists only while `general.media_controls` is
    on.
  - `shell/src/desktop.rs:51-54,91-106`: the inhibit, the Unity progress
    and the screen-lock signals live inside it.
- **Failure:**
  - With "Use the system's media transport controls" off, or when MPRIS
    did not start (the bus took over 10 s), the screen blanks during
    films, the launcher shows no progress and "Pause when the screen
    locks" does nothing.
  - Windows keeps all three (`awake.rs`, `taskbar.cpp`, the session
    notifications).
- **Fix:**
  - Start the desktop thread always; serve the MPRIS name and objects
    only while the switch is on.
  - `awake.rs`'s header ("Both follow `App::keeps_screen_on`") then holds.

### M3. The system master volume runs `pactl` on the window's thread (Win)

- **Sources:** A M3, B M3. Confirmed.
- **Where:**
  - `crates/platform/src/pulse.rs:25-56,89-125`.
  - Called from `shell/src/player.rs:2466-2468` (`UiRequest::SystemVolume`),
    from `App::change_volume` and from `set_volume_fraction`.
- **Failure:**
  - Each wheel notch or slider step runs one or two `pactl` processes,
    polled every 10 ms with a 2 s timeout each, on the GUI thread. This is
    against "Keep the GUI thread free".
  - A hung sound server freezes the window for up to 4 s a step.
- **Fix:**
  - A worker thread that keeps only the newest target.
  - It sends the OSD text back through `CxxQtThread::queue`.

### M4. Text to speech on Linux: truncated voices, a check that cannot fail, the window thread, other programs' speech

- **Sources:** A M5, M6 and low 1; B M4. Confirmed, except `-C`, which is
  likely.
- **Where:** `shell/cpp/tts.cpp:86-103` and `:150-159`;
  `shell/qml/SelfTestM14.qml:366-367`.
- **Failure:**
  - **Truncated names:** `spd-say -L` pads its columns with spaces (not
    tabs, as B said). Names hold spaces ("English (America)",
    "Bishnupriya Manipuri+Adam"), and
    `line.section(u' ', 0, 0, SectionSkipEmpty)` keeps the first word.
    Such voices cannot be chosen.
  - **A check that cannot fail:** `m14_voices_list` passes on any array,
    `[]` included.
  - **The window thread:** the list (about 14,800 lines here) is read with
    `waitForFinished(3000)` on the GUI thread, with an O(n²)
    `QJsonArray::contains`.
  - **Other programs' speech:** every notice runs `spd-say -C`, whose man
    page says "Cancel all messages". This likely cuts off Orca too.
  - **Unchecked values:** rate and volume are not clamped to spd-say's
    -100 to 100.
- **Fix:**
  - The name is every field but the last two.
  - Read the list once on a thread, de-duplicated with a set.
  - Drop `-C`, or cancel only RVP's own message through one libspeechd
    connection.
  - Clamp rate and volume.
  - The test requires a listed voice when speech-dispatcher is present.

### M5. The single-instance channel lives in the shared `/tmp`

- **Sources:** A M7 and lows 2 and 11, B H4. Confirmed. It matters only on
  machines with several local accounts.
- **Where:**
  - `crates/app/src/instance.rs:196-209`: a bare name `rvp-<USER>`, which
    Qt puts in `QDir::tempPath()`.
  - `shell/cpp/shell.cpp:571-619` (send and listen) and
    `shell/src/main.rs:88-107,135-153`.
- **Failure:**
  - **Hijacking:** another user who creates `/tmp/rvp-alice` (open to all)
    and answers `ok` receives every path, URL and header Alice opens, and
    her RVP exits.
  - **Denial:** her `removeServer` cannot delete that file in the sticky
    `/tmp`, so she can never listen.
  - **Shared names:** without `$USER` every account shares `rvp-user`.
  - **A live channel taken:** a sender that connected but timed out (the
    running instance's window thread busy for over 3 s) removes the live
    socket and starts a second instance. `main.rs`'s "The first one keeps
    the channel" does not hold on Unix.
  - **Shared state folder:** when the settings cannot be read, the state
    goes to `/tmp/rvp-state` (`shell/src/player.rs:893`), which another
    user can own.
- **Fix:**
  - On Unix, use `$XDG_RUNTIME_DIR/rvp-<uid>` (an absolute name Qt takes
    as a path), falling back to the temp folder only when it is unset.
  - Remove the socket only when nothing accepted the connection.
  - Use `$XDG_RUNTIME_DIR` or a `mkdtemp` folder for the fallback state.

### M6. "Minimize to tray" hides the window where no tray exists

- **Sources:** A M4. The code is confirmed; the missing icon is likely
  (Qt.labs.platform's behaviour).
- **Where:** `shell/qml/Tray.qml:23-26` hides on minimize when `toTray` is
  set; `shell/qml/Main.qml:514-519`'s `requestRaise` calls `showNormal()`
  only for a minimized window.
- **Failure:**
  - On X11 or XWayland without a StatusNotifierItem host (GNOME without
    the AppIndicator extension: Fedora, Debian), a minimize hides the
    window for good.
  - MPRIS Raise and a second `rvp` only `raise()` it.
- **Fix:**
  - Hide only when `icon.available`.
  - `requestRaise` also calls `showNormal()` when the window is not
    visible.

### M7. Wireless mice and gamepads count as the system battery

- **Sources:** A M9, B M9. Confirmed.
- **Where:** `crates/app/src/osd.rs:561-602`.
- **Failure:**
  - Peripheral batteries are `type=Battery` with `scope=Device`, and a
    desktop has no `Mains` supply.
  - On a desktop with a wireless mouse, `on_battery_from_sysfs` returns
    `Some(true)`:
    - Battery presets apply on mains power, and AC presets never do.
    - The info text shows the mouse's charge as the battery.
  - USB-C chargers (`type=USB`) are not counted as power.
  - Several `Mains` entries overwrite each other.
- **Fix:**
  - Skip supplies with `scope=Device`.
  - Count `Mains` or `USB` with `online=1` as power, any of them.
  - Return `None` when no system battery exists.
  - Unit tests with fake sysfs folders.

### M8. Package versions do not grow with commits (Win)

- **Sources:** A M11. Confirmed.
- **Where:** `tools/packaging/package.py:91-92`: `0.0.1+git.<hash>`.
- **Failure:**
  - `dpkg --compare-versions 0.0.1+git0ae812e0e-1 lt 0.0.1+gitaf36fa313-1`
    is true: a newer commit can sort lower.
  - `apt-get install` refuses the "downgrade", and m8's install uses it.
- **Fix:**
  - Put the commit time before the hash (`commit_time()` exists), for
    example `0.0.1+git20261114105222.c3f58dc13`.
  - Check the Windows installer's numeric version is unchanged.

### M9. Kit and package checks that pass without testing

- **Sources:** A M12, M13 and lows 25 to 27; B M10 and L10. Confirmed.
- **Disc menus:**
  - `SelfTestM16.qml:71,99` writes `check ...=PASS SKIPPED no libdvdnav`
    (or `no libbluray`) when the library is missing.
  - `check-deb.sh:71-73` looks only at `failures=0`, and `kit_judge`'s
    `grep "^check $must=PASS"` also matches `=PASS SKIPPED`.
  - `shell/src/disc/tests.rs:174-180,354-359` skip when the library is
    missing even with `RVP_DVD_MENU` or `RVP_BD_MENU` set.
  - The soname miss of this boot run would have passed every one of them.
- **Library check:** `check-deb.sh:48-56` runs `ldd` with
  `LD_LIBRARY_PATH=/opt/rvp/lib`. This hides a missing rpath on `rvp`, and
  it skips the executables (`whisper-cli`, `trtexec`). It runs without
  `-r` and without `LC_ALL=C`, and nothing checks that the Qt plugins RVP
  needs are present: `libqgtk3.so`, `libqxdgdesktopportal.so`,
  `libqxcb.so`, `libqwayland-*.so`.
- **The m8 install:** `kit.sh:1405-1406` passes with whatever `rvp` is
  installed. With an older build installed, m8 tests the old copy.
- **The inhibit check:** `kit.sh:1342` passes when something else already
  inhibits: today's log reads "before (true,) playing (true,) paused
  (true,)".
- **m15's server log:** `kit.sh:2128-2131` passes on an empty log, for
  example after a failed `curl`, and its `tr '"' '\n'` split drops context.
- **The report:** `step_report` (`kit.sh:2305-2306`) leaves out `m18trt`.
- **Fix:**
  - Required checks reject `=PASS SKIPPED`.
  - check-deb requires m16's menu checks.
  - The disc tests fail when their disc is given and the library is
    missing.
  - Run `ldd -r` without `LD_LIBRARY_PATH` on `rvp` and the executables,
    with `LC_ALL=C`, plus a plugin list.
  - m8 installs the built version, or fails, when the installed one
    differs.
  - The inhibit check reports SKIPPED when inhibited before, or finds RVP
    in `GetInhibitors`.
  - m15 requires `loading: RVP.Player` and splits the log's JSON by object.
  - The report includes m18trt.

### M10. Settings texts and controls that are wrong on Linux (Win)

- **Sources:** A M17 and low 23, B M5. Confirmed.
- **Texts (`shell/qml/Settings.qml` at `c3f58dc`):**
  - `:3378` "Windows' speech (SAPI) reads; on Linux there are no voices
    yet."
  - `:3412` "... Windows only; Linux's controllers come later."
  - Both parts work on Linux (`tts.cpp`, `xinput.cpp`,
    `docs/coverage/linux.md`).
  - "Windows' default" labels the voice and the output device.
- **Controls that do nothing on Linux:**
  - The output combo box (always empty there).
  - "Default Programs…" (`ms-settings:`).
  - "Clear and Rebuild the Icon Cache" (a no-op on Linux).
  - The capture method choices "GDI" and "Desktop Duplication", which
    `linux_screen_address` ignores.
- **The test:** `crates/app/tests/coverage.rs:561-567` looks for a few
  fixed phrases and misses these.
- **Fix:**
  - Rewrite the two notes for both systems, and the Russian ones in
    `qml_ru.ts`.
  - Say "System default" on Linux.
  - Hide the Windows-only controls when `Qt.platform.os !== "windows"`.
  - Widen the test, for example to `Linux.*(later|yet)`.
- **Related open item from the boot run:** the Settings window's tab
  arrows do not show in the installed .deb (the window is about 848 px
  wide); the working tree holds diagnostics for it
  (`Settings.qml` `tabScroll()`, `SelfTestM12.qml` `settings_tab_arrows`).

### M11. The GPU budget's lighter step skips MVTools after RIFE (TensorRT) (Win)

- **Sources:** B M8. Confirmed.
- **Where:** `crates/app/src/smoothing.rs:1801-1810` checks
  `engine == Engine::Rife`. The plan (`:501-505`) uses `engine.is_rife()`,
  which also covers `RifeTrt`.
- **Failure:**
  - With RIFE (TensorRT) at Fast, `smoothing_step_changes(SmoothingLighter)`
    is false.
  - The budget thinks the step does nothing and goes from RIFE (TensorRT)
    straight to Blend.
- **Fix:** `engine.is_rife()`, and a unit test for both RIFE engines.

### M12. The Vulkan renderer's X11 window covers the overlay when a layer hides the whole video

- **Sources:** B H6. B quoted code that is not in `c3f58dc`; HEAD's form
  has the same defect. Likely.
- **Where:** `shell/cpp/video_window.cpp:317-330` (Linux):
  `shown -= hole` for each hole, then `window->setMask(shown)`.
- **Failure:**
  - When the holes cover the whole video, `shown` is empty.
  - Qt's xcb `setMask` with an empty region removes the shape, so mpv's
    window shows whole over the layer that should cover it.
  - On Windows an empty region hides the child.
- **Fix:** when `shown` is empty, place the window at `(-10, -10, 1, 1)`,
  as for "not visible".

### M13. M12's window-size steps race on Wayland (Win)

- **Sources:** B M11. Confirmed in the code; likely the cause of today's
  `window_control_places=FAIL timed out 801x475`.
- **Where:** `shell/qml/SelfTestM12.qml:351-382`.
- **Failure:**
  - **Repeated action:** on Wayland, step 9 runs `window-custom-dialog`
    again on every 50 ms tick until the dialog opens.
  - **A pass too early:** on a 2560x1440 screen the window is already
    801x475 after `window-larger`, which is "near" 800. Step 10 can pass
    before the 800x500 answer applies, and step 12's placement then
    races the late resize.
- **Fix:**
  - Run the action once, in step 8.
  - Reset `geometryChanged` when a resize is asked.
  - Use a size no earlier step can reach, for example 860x540.

### M14. The AUR package: the remote's pictures, the linker, the system VapourSynth

- **Sources:** A M16 and low 16, B M14. Confirmed, except `lld`, which is
  likely.
- **Where:** `packaging/aur/PKGBUILD:20,61`; `crates/app/src/svp.rs:171-190`.
- **Failure:**
  - **The remote's pictures:** only `icon.png` and `icon_hires.png` are
    installed, so Unified Remote cannot load the layout (`mute.png`), the
    same error dbde0cd fixed in the kit.
  - **The build:** makedepends lack a linker that qt-build-utils accepts
    (`lld`).
  - **The system VapourSynth:** VSScript is looked for only at Debian's
    multiarch path and `/usr/local`, not at Arch's `/usr/lib` or Fedora's
    `/usr/lib64`, so smoothing stays Blend in the AUR build.
- **Fix:**
  - Install every picture.
  - Add `lld` (and `qt6-tools` if `lrelease` needs it).
  - Add those paths, `.so.4` included.

### M15. Recording RVP's sound on Linux: misaligned samples, the wrong stream

- **Sources:** B M6, A lows 6 and 7. Confirmed for the samples; likely for
  the stream.
- **Where:** `shell/src/recorder.rs:179-227`;
  `crates/platform/src/pulse.rs:137-154`.
- **Failure:**
  - **Misaligned samples:** `pending` outlives each `parec`. A process
    that ends inside a frame leaves 1 to 7 bytes, which shift every later
    sample of the recording.
  - **The wrong stream:** the stream is RVP's lowest sink-input. With the
    second audio device on, a new main stream (another file's format)
    can hand the recording to the second device's stream.
  - **The name:** no core sets `audio-client-name`, so the sound settings
    show RVP's streams as "mpv".
- **Fix:**
  - Clear `pending` for each new `parec`.
  - Give the cores distinct `audio-client-name`s and pick the stream by
    `application.name`.

### M16. RVP's VapourSynth runtime keeps `_tkinter` without Tcl/Tk

- **Sources:** A M10. Confirmed.
- **Where:** `tools/vapoursynth/build-linux.sh:157-161` removes tkinter and
  `libtcl*`/`libtk*`, but not `lib-dynload/_tkinter*.so`.
- **Failure:**
  - `ldd` on it gives `libtcl9.0.so => not found`.
  - check-deb's `libraries_resolve` fails as soon as CI's packages carry
    the runtime (H1).
- **Fix:** remove `"$P"/lib-dynload/_tkinter*` too.

## Low

Robustness, narrow setups and clarity. Confirmed unless marked likely.

**Smoothing and the GPU**

- **L1. TensorRT's package files are copied in place (Win).**
  `crates/app/src/tensorrt.rs:523-531` copies `libvstrt.so`/`.dll` and
  `trtexec` with `std::fs::copy` (truncate and write). A reinstall while
  the plugin is mapped can kill the process with SIGBUS; `unpack` already
  writes `.new` and renames for this reason. `tensorrt_remove` (`:876-890`)
  deletes the folder under a running calibration. Fix: rename into place;
  stop the calibration first. (B M7)
- **L2. `nvidia-smi` runs at every start with TensorRT installed.**
  `tensorrt.rs:739-757` runs it without the runtime-suspend check D17
  asks for, which wakes a laptop's sleeping card. (A low 17)
- **L3. The calibration runs again after every package or kernel
  update.** `calibration.rs:315-337` keys on size and mtime, `package.py`
  touches every file to the commit time, and `hardware.rs:61-65` adds the
  kernel release. This needs a decision (question 6). (A low 18)
- **L4. RVP's VapourSynth has the build machine's plugin folder.**
  `build-linux.sh:112` `-Dplugindir="$OUT/vs-plugins"`. A build path is
  compiled into `libvapoursynth.so` (seen in the kit's copy) and scanned at
  every core start. RVP loads its plugins by path; set a folder inside
  the package, or one that does not exist. (B M15)

**Window thread and processes**

- **L5. A logind power action can block the window.**
  `crates/platform/src/power.rs:14-35` calls `PowerOff` with
  `interactive=true` and no timeout on the window thread; polkit may ask
  for a password. Fix: a thread. (A low 3)
- **L6. "Add to Unified Remote" blocks for up to 3 s.**
  `control/unified.rs:61-77` is called from `player/m15.rs:217`. Fix: a
  thread. (A low 4)
- **L7. New windows leave zombies (Win).** `player/m11.rs:226-236` drops
  the `Child` without `wait()`. Fix: a reaper thread or
  `QProcess::startDetached`. (A low 5, B L5)
- **L8. Qt's platform variables reach programs RVP starts.**
  `player.rs:803,834` set `QT_QPA_PLATFORM=xcb` and
  `QT_ENABLE_HIGHDPI_SCALING=0`, and `xdg-open` passes them on: a Qt file
  manager runs under XWayland or unscaled. (A low 14)

**Files, paths and the desktop entry**

- **L9. `XDG_CONFIG_HOME` and `XDG_DATA_HOME` are taken even when
  relative.** `shell/src/startup.rs:20-23` and `player/m14.rs:1340-1343`;
  the rest of RVP filters `is_absolute()`. An empty value writes
  `./autostart/rvp.desktop`. (B M12)
- **L10. `Exec` escaping.** `startup.rs:38` writes `Exec="{}"` without
  escaping. `registration::desktop_exec` (`registration.rs:435-453`)
  doubles no `%`, and per the Desktop Entry spec a quoted `"`, `` ` ``,
  `$` or `\` needs its backslash escaped again (the string rule applies
  first). Fix: one escaper for both, with tests. (A low 10, B M12)
- **L11. `recently-used.xbel` updates race.**
  `crates/platform/src/recent.rs:43-50`. Every update thread (one per
  started file) writes the same `.rvp-<pid>` temporary file. An
  unreadable list is replaced by an empty one, losing other programs'
  entries. A self-closing `<bookmark/>` is cut up to the next
  `</bookmark>`. URIs are escaped differently from GLib's. Fix: one writer
  and a unique temporary name; keep a list it cannot read. (A low 8, B L2)
- **L12. The trash accepts another user's folder.** `trash.rs:79-85,136-148`
  does not check that `.Trash-$uid` and `.Trash/$uid` are no symlinks,
  belong to the user and are 0700, as the specification and gio do.
  (A low 9, B L3)
- **L13. SVP's socket check.** `svp.rs:107-111` treats "permission denied"
  and another user's stale socket as free, and `prepare_endpoint` creates
  `/tmp/mpvSockets` 0755, which other users cannot use. Fix: count every
  error but "refused" or "not found" as in use; create the folder 01777.
  (A low 15, B H3, downgraded: see "Rejected")
- **L14. `file_url` neither escapes nor keeps backslashes (Win).**
  `player.rs:914-921`: `#`, `?` or `%` in a folder break the URLs QML
  opens, and on Linux a `\` in a name becomes `/`. MPRIS's `xesam:url`
  does the same (`media.rs:190,236-238`). (A low 12)
- **L15. Saved frames and recordings ignore the XDG user folders.**
  `commands.rs:790-794`, `record.rs:56-59`, `audio/commands.rs:1060-1064`
  use `~/Pictures/RVP` and `~/Videos/RVP`; here they are `~/Изображения` and
  `~/Видео`. `audio/commands.rs` reads `USERPROFILE` before `HOME`. This
  needs a decision (question 5). (A low 13)
- **L16. `/cd:/dev/sr0` becomes `/dev/sr0:` (Win).** `sources.rs:1043-1047`
  appends `:` to any drive. Fix: only to a single letter. (B L1)

**Desktop (likely; need a desktop run)**

- **L17. Fullscreen on another display does nothing on Wayland.**
  `Main.qml:754-781` returns early, and `docs/coverage/linux.md` does not
  say so. Try `root.screen` before `showFullScreen()`. (A low 19)
- **L18. Restoring and raising on Wayland.** A tray click relies on
  `Window.Minimized`, which xdg-shell does not report (`Tray.qml:37-40`).
  No `XDG_ACTIVATION_TOKEN` reaches the running instance, so GNOME likely
  shows "RVP is ready" instead of raising it. (A lows 20 and 21)
- **L19. Classic Winamp or VLC skins chosen at run time on Wayland.**
  `prefer_x11` applies only at start; the skin's windows then cannot be
  dragged (`SkinWindow.qml` uses `setGeometry`). Fix: `startSystemMove()`
  on Wayland, or a note that a restart is needed. (B L8)
- **L20. The joystick (`shell/cpp/xinput.cpp`).**
  - `click()` (`:141-155`, used at `:194-197`) ignores "Act only when RVP
    has the focus".
  - A quick left-to-right on the pad without a centre event is lost
    (`before == 0`, `:188`).
  - `findPad` (`:205-222`) takes the first `js*` node, so a laptop's
    accelerometer can hold the slot. Fix: check `JSIOCGBUTTONS`.
  - (A low 22, B M13)
- **L21. The Vulkan child window's handle.** `video_window.cpp:200-204`
  keeps a raw `QWindow *` that `rvp_video_window_close` never frees; use
  `QPointer` and delete it there. (B L7)

**Packaging and kit**

- **L22. Package contents.**
  - Files are group-writable (`rwxrwxr-x`, the kit's umask).
  - The whole Qt `qml/` tree is copied, which pulls `libQt6Test`,
    `libQt6QuickTest`, `libQt6Sql`, `libQt6QuickParticles` and the other
    styles into `lib/`.
  - Nothing is stripped (`rvp` 28 MB).
  - Fix: normalize modes (0755/0644), copy only the QML modules RVP
    imports (Windows' `QT_QML` and what the dialogs need), strip.
  - (A low 31, B L9)
- **L23. `curl` is listed nowhere.** RIFE (TensorRT)'s Download runs the
  PATH's `curl`; neither the .deb's Recommends nor the PKGBUILD's
  optdepends name it, and the error does not say so. (A low 30)
- **L24. whisper's build folder is not cleaned.**
  `tools/whisper/build-linux.sh:49,69` copies every `*.so*` left there.
  (A low 29)
- **L25. The kit's smaller items.**
  - `step_mpv` (`kit.sh:466-525`) runs in a brace group: `|| exit 1` ends
    the kit without a result line, its `export`s and `cd` leak, and
    `./rebuild` and `./install` are unchecked.
  - `step_m6svp` has no time limit.
  - `input-check.py`'s drag can leave the button pressed.
  - `build-vstrt-linux.sh:81-84` fetches headers without a SHA-256.
  - `sources.sh` mirrors neither vs-mlrt nor TensorRT.
  - (A low 32)

**Code rules and comments**

- **L26. `#[allow]` without a justifying comment.**
  - `crates/app/src/speech.rs:712,719`.
  - `smoothing.rs:2118,2127,2161,2222`: the comment describes the test,
    not the cast.
  - `capture.rs:346,352,355,360`: only `:339`'s comment covers them.
  - (A)
- **L27. Stale comments.**
  - `svp.rs:22-24` says RVP points mpv at SVP's own runtime.
  - `tts.cpp` says a new text cancels only the one being read.
  - `main.rs:135` says "The first one keeps the channel".
  - `window.rs:9` says "(goal S lists them)".
  - (A)

## Rejected

- **B M16, "`setDesktopFileName` is never called":** `shell/src/main.rs:73`
  calls `QGuiApplication::set_desktop_file_name("rvp")`. The portal's "App
  info not found for 'rvp'" comes from kit runs with no `rvp.desktop`
  installed.
- **B M4, "`spd-say -L` is tab-separated":** its columns are padded with
  spaces. The real defect is names with spaces (M4).
- **B L4, "`SA_NODEFER` is missing":** `raise()` inside the handler leaves
  the signal pending. It is delivered with the default action
  (`SA_RESETHAND`) when the handler returns, as intended.
- **B L6, "NVML drops the sample when the default limit is unsupported":**
  `gpu_power.cpp:216-219` falls back to the enforced limit. A card with
  neither has no share the budget could use.
- **B H3, "any local user can run commands through `/tmp/mpvsocket`":**
  connecting to a Unix socket needs write permission on it. mpv creates
  it under the user's umask (0022 or 0002), so other users lack it. The
  other two parts stay as L13.
- **B L5, "`/new` breaks inside the AppImage after the first RVP exits":**
  A dropped it. Children inherit the AppImage runtime's keepalive pipe, so
  the mount stays.

## Questions for the owner

Goal U asks these in one round at its start. Without an answer it takes
the default (first) option.

1. **H1, CI:** build and cache whisper and the VapourSynth runtime in
   `package-linux` (about 9 minutes the first time, then cached), or keep
   CI's Linux packages slim and say so?
2. **H3, the bundled libraries:** move the overlapping ones to Depends (a
   .deb built on 22.04 then needs them on the system), or give `rvp`
   RUNPATH instead of RPATH (a change of AGENTS.md's packaging rule)?
3. **M2, "media transport controls" off:** keep the screen on, the
   launcher's progress and pausing on lock anyway (as Windows does), or
   turn them all off with it?
4. **M5, L12, L13, several accounts:** fix the shared-`/tmp` and trash
   cases now, or record them for later?
5. **L15, the user folders:** saved frames and recordings in the XDG
   folders (`~/Изображения/RVP`), or keep `~/Pictures/RVP`?
6. **L3, the calibration:** run it again after a package update (the
   engines' files changed), or key it on the files' content?
7. **M14, the AUR package:** carry RVP's own VapourSynth runtime, or use
   the system's with optdepends?
8. **Open from the boot run, still the owner's:**
   - m6svp's second window.
   - The manual checks of the checklist's section 10.

## Work plan (goal U)

Goal U runs on the Ubuntu boot, where the desktop and the card are
available. The Windows VM is not, since it runs on the same PC's
Windows. Goal U-W then checks on the Windows VM the fixes marked Win
(D19).

Each package:

- Re-reads the code (line numbers move).
- Fixes the findings.
- Adds or tightens the test that would have caught each.
- Runs `cargo xtask ci`.
- Is committed on its own.
- Is recorded in `docs/goals/goal-u-status.md`: fixed with its check, or
  left with the reason.

| # | Package | Findings | Linux check |
|---|---------|----------|-------------|
| 0 | Start | The boot run's report and status files committed and pushed; the questions asked | `kit.sh report` |
| 1 | Checks that pass without testing | M9, M13 | the kit's m8, m12, m15 and m16; check-deb's ldd part against `/opt/rvp` |
| 2 | Packages and CI | H1, H3, M8, M16, L4, L22, L23, L24 | `kit.sh m8package` and `m8`; the container check (CI) once pushed |
| 3 | OpenGL on Linux | H2, M12 | m12 with the shaders on Wayland (NVScaler), with `QT_QPA_PLATFORM=xcb`, and under Xvfb |
| 4 | Desktop integration | M2, M6, L17, L18, L19 | m8 (MPRIS, the inhibit with MPRIS off too), the tray by hand |
| 5 | The window thread | M3, M4 (the list), L5, L6 | m10 (the master volume), m14 (the voices), m15 |
| 6 | Speech and the joystick | M4 (rest), L20 | m14; the joystick by hand if one is at hand |
| 7 | Paths, files and processes | M1, M5, L7, L8, L9, L10, L11, L12, L13, L14, L16 | unit tests; m11, m14 and m3 on the desktop; two `rvp` starts with the channel in `$XDG_RUNTIME_DIR` |
| 8 | Recording | M15 | m10's recording with the second device on |
| 9 | Smoothing and TensorRT | M11, L1, L2, L3 | unit tests; m18 and m18trt (with the owner's permission for the GPU) |
| 10 | Texts, the AUR package, code rules | M10 (with the tab arrows), M14, L26, L27 | `tests/translations.rs`, `tests/coverage.rs`, m14's text fit, Settings by hand at the installed .deb's size |
| 11 | Close | The boot run's steps the packages touched, again | `kit.sh report`, goal-u-status.md, push, CI green |

Goal U-W (WSL, the Windows VM, after goal U) builds the Windows bundle
and runs the VM's checks for the packages marked Win:

- M1, M3, M8, M10, M11, M13, L1, L7, L14 and L16.
- `m10-all.ps1`, `m11-all.ps1`, `m12`'s self-test, `m14-all.ps1`,
  `m18-all.ps1` and the M8 installer check.

It records the results in goal-u-status.md's "Windows" section.

## The brief

Both auditors got the same brief:

- **Scope:** every Linux path in `crates/platform`, `shell/src`,
  `shell/cpp`, `crates/app`, `crates/engine`; the QML with Wayland
  branches; `tools/packaging`, `packaging/`, `tools/linux-kit`, the Linux
  build scripts and CI's Linux jobs.
- **What to look for:** correctness on Linux (paths, sonames, distro
  assumptions, Wayland and X11, GNOME and other desktops); blocking calls
  on the window thread; the processes RVP starts; security (the control
  API, sockets and files in `/tmp`, the trash, TensorRT's download);
  packaging; AGENTS.md's code rules; tests and kit steps that pass for
  the wrong reason.
- **Rules:** read only; nothing that opens a window, plays sound or loads
  the GPU; no sudo or network; quote the code and the failure for each
  finding.
