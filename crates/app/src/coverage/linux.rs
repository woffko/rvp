//! RVP on Linux (M17, goal S): the actions, settings and parts whose Linux
//! side differs from Windows', each with how Linux does it or why it
//! cannot. `tests/coverage.rs` writes `docs/coverage/linux.md` from
//! [`ITEMS`], checks that every action and setting it names exists, and
//! that no reason says a part is merely not done yet. Everything not
//! listed works on Linux as on Windows (it is libmpv's, FFmpeg's, Qt's or
//! RVP's own code).

use std::fmt::Write as _;

/// What an item names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Actions by their settings names.
    Actions(&'static [&'static str]),
    /// Settings by their places in `settings.json`.
    Settings(&'static [&'static str]),
    /// A part with no action or setting of its own.
    Part,
}

/// How Linux has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linux {
    /// The same, through Linux's own means.
    Same(&'static str),
    /// Where it works, and why not elsewhere.
    Partly(&'static str),
    /// Why it cannot.
    Not(&'static str),
}

/// One item.
#[derive(Debug, Clone, Copy)]
pub struct Item {
    pub what: &'static str,
    pub key: Key,
    pub linux: Linux,
}

const fn item(what: &'static str, key: Key, linux: Linux) -> Item {
    Item { what, key, linux }
}

/// Why Wayland refuses a program's window commands.
const WAYLAND_PLACES: &str = "the compositor places the windows and refuses a program's moves and \
     coordinates; they work in an X11 session (and for RVP under XWayland)";

/// The items, in the order of PotPlayer's menus.
pub const ITEMS: &[Item] = &[
    item(
        "System master volume",
        Key::Actions(&[
            "system-master-up",
            "system-master-down",
            "system-master-mute",
            "volume-method-master",
        ]),
        Linux::Same(
            "the default output's volume through PulseAudio's `pactl`, which PipeWire's pulse server answers too",
        ),
    ),
    item(
        "Audio recording",
        Key::Actions(&["record-audio", "record-audio-toggle"]),
        Linux::Same(
            "PulseAudio's `parec` records RVP's own playback stream (found by RVP's process), after its whole chain and volume, as 32-bit float stereo WAV at 48 kHz",
        ),
    ),
    item(
        "Second audio device",
        Key::Settings(&["audio.second-device"]),
        Linux::Same(
            "a second libmpv core on the chosen device; PipeWire can also combine two outputs itself",
        ),
    ),
    item(
        "Open Screen Capture",
        Key::Actions(&["open-screen"]),
        Linux::Partly(
            "FFmpeg's `x11grab` in an X11 session; a Wayland desktop gives its screen to other programs only through its portal as a PipeWire stream, which FFmpeg cannot open, and XWayland's screen shows X11 programs' windows only",
        ),
    ),
    item(
        "Open Webcam/Other Device",
        Key::Actions(&["open-device"]),
        Linux::Same(
            "the `v4l2` devices sysfs lists (`/sys/class/video4linux`), by their `/dev` paths",
        ),
    ),
    item(
        "Power actions and the shutdown timer",
        Key::Actions(&[
            "power-shutdown",
            "power-sleep",
            "power-hibernate",
            "shutdown-at",
            "shutdown-in",
        ]),
        Linux::Same(
            "systemd-logind's PowerOff, Suspend and Hibernate on the system bus; polkit may ask the user",
        ),
    ),
    item(
        "Recycle played files, Delete Files",
        Key::Actions(&["recycle-played", "delete-files"]),
        Linux::Same(
            "the freedesktop.org trash: `~/.local/share/Trash`, or the trash at the top of the file's own filesystem, with the `.trashinfo` file managers restore from",
        ),
    ),
    item(
        "Always on top",
        Key::Actions(&[
            "on-top-always",
            "on-top-while-playing",
            "on-top-fullscreen",
            "on-top-cycle",
        ]),
        Linux::Partly(
            "X11 keeps the window above the others (and RVP under XWayland); GNOME's Wayland offers programs no protocol to stay on top",
        ),
    ),
    item(
        "Window moves and places",
        Key::Actions(&[
            "window-move-left",
            "window-move-right",
            "window-move-up",
            "window-move-down",
            "window-center",
            "window-top-left",
            "window-top-right",
            "window-bottom-left",
            "window-bottom-right",
        ]),
        Linux::Partly(WAYLAND_PLACES),
    ),
    item(
        "The window's place at start",
        Key::Settings(&["general.start.position"]),
        Linux::Partly(WAYLAND_PLACES),
    ),
    item(
        "Window opacity",
        Key::Settings(&[
            "skins.opacity",
            "skins.panel-opacity",
            "subtitles.browser.opacity",
        ]),
        Linux::Partly(
            "X11's window opacity (and RVP under XWayland); Wayland has no window opacity for programs",
        ),
    ),
    item(
        "Video renderer",
        Key::Settings(&["render.renderer"]),
        Linux::Partly(
            "OpenGL in RVP's window, or mpv's gpu-next with Vulkan in an X11 child window (Wayland has no child windows of another program, so RVP starts through XWayland with it); Direct3D 11 is Windows'",
        ),
    ),
    item(
        "Direct3D 11's HDR output, 10-bit output, GPU Super Resolution and RTX Video HDR",
        Key::Settings(&[
            "render.hdr-output",
            "render.ten-bit",
            "render.super-resolution",
            "render.rtx-hdr",
        ]),
        Linux::Not(
            "Direct3D 11's: the Super Resolution and RTX Video HDR of mpv's d3d11vpp have no Linux counterpart in mpv, and an HDR display needs Wayland's colour management, which mpv's render API in Qt's window does not reach (the Vulkan window passes HDR where the X11 server does)",
        ),
    ),
    item(
        "NVIDIA Optical Flow smoothing",
        Key::Part,
        Linux::Partly(
            "FFmpeg's fruc_vulkan in the packages' libmpv, with the Vulkan renderer (X11, or XWayland on Wayland) on an NVIDIA card",
        ),
    ),
    item(
        "Pointer kept on the fullscreen display",
        Key::Part,
        Linux::Not(
            "Qt confines the pointer on no Linux platform: X11 confines it only during a pointer grab, which takes the pointer from every other window, and Qt does not use Wayland's pointer-constraints protocol",
        ),
    ),
    item(
        "Desktop Mode",
        Key::Part,
        Linux::Not(
            "it puts the window into Explorer's desktop layer; GNOME's and KDE's desktops take no program's window into theirs",
        ),
    ),
    item(
        "Screen saver (rvp.scr)",
        Key::Settings(&["screensaver"]),
        Linux::Not(
            "Linux desktops' screen savers (GNOME's and KDE's lock screens) start no other program; `rvp --screensaver` still plays the page's files in fullscreen",
        ),
    ),
    item(
        "Tray icon",
        Key::Settings(&["general.tray"]),
        Linux::Partly(
            "Qt's tray icon through the StatusNotifierItem protocol: KDE, Xfce and Ubuntu's GNOME (its AppIndicator extension) show it; GNOME without that extension has no tray",
        ),
    ),
    item(
        "The taskbar button's progress",
        Key::Settings(&["general.taskbar"]),
        Linux::Same(
            "the Unity launcher API (`com.canonical.Unity.LauncherEntry`), which Ubuntu's dock and KDE's task manager show",
        ),
    ),
    item(
        "The taskbar button's buttons (previous, play/pause, next)",
        Key::Part,
        Linux::Not(
            "Linux's docks and task bars have no thumbnail toolbar; MPRIS gives the desktop's media controls the same three buttons",
        ),
    ),
    item(
        "Recent files in the taskbar",
        Key::Settings(&["open.recent"]),
        Linux::Same(
            "freedesktop.org's desktop bookmarks (`~/.local/share/recently-used.xbel`), which GTK's file dialogs and GNOME's Files list",
        ),
    ),
    item(
        "Pause when the screen locks",
        Key::Settings(&["general.pause-screen-locked"]),
        Linux::Same(
            "the screen saver's ActiveChanged signal (GNOME's and freedesktop.org's) on the session bus",
        ),
    ),
    item(
        "Run at system startup",
        Key::Settings(&["general.run-at-startup"]),
        Linux::Same("an XDG autostart entry (`~/.config/autostart/rvp.desktop`)"),
    ),
    item(
        "Media keys while another window has the keyboard",
        Key::Settings(&["input.global-media-keys"]),
        Linux::Same(
            "MPRIS: GNOME and KDE send the media keys to the player that holds MPRIS's name",
        ),
    ),
    item(
        "Apply to System (file types)",
        Key::Settings(&["associations.off", "associations.extra"]),
        Linux::Same(
            "the user's `rvp.desktop` (it takes the place of the packages' entry) lists the types that are on; the desktop's settings choose the defaults, as Windows' Default apps do",
        ),
    ),
    item(
        "AutoPlay handlers",
        Key::Settings(&["associations.autoplay"]),
        Linux::Same(
            "the user's `rvp.desktop` lists the `x-content` types of the chosen discs (DVD, Blu-ray, audio CD, VCD, SVCD), which GNOME's Removable Media settings offer; video and audio files on a new drive have no such type",
        ),
    ),
    item(
        "Folder and file context menu verbs",
        Key::Settings(&[
            "associations.folder-menu",
            "associations.add-menu",
            "associations.play-menu",
        ]),
        Linux::Not(
            "Linux's file managers list a type's programs under Open With, from the desktop entries; they take no program's own verbs",
        ),
    ),
    item(
        "Text to speech",
        Key::Settings(&[
            "accessibility.tts",
            "accessibility.voice",
            "accessibility.rate",
            "accessibility.volume",
        ]),
        Linux::Same(
            "speech-dispatcher through its client library, with its voices, rate and volume; a new text cancels RVP's own one being read",
        ),
    ),
    item(
        "Text to speech's audio output",
        Key::Settings(&["accessibility.output"]),
        Linux::Not(
            "speech-dispatcher plays through the desktop's sound and gives a program no choice of output",
        ),
    ),
    item(
        "XInput (Xbox controller)",
        Key::Settings(&["accessibility.xinput", "accessibility.xinput-focus-only"]),
        Linux::Same(
            "the kernel's joystick API (`/dev/input/js*`, the xpad driver's button numbers)",
        ),
    ),
    item(
        "XInput's mouse emulation",
        Key::Settings(&["accessibility.xinput-mouse"]),
        Linux::Partly(
            "the stick moves the pointer in an X11 session (Wayland lets no program move the pointer) and A clicks in RVP's own windows (the window system takes no injected input from a program)",
        ),
    ),
    item(
        "Classic Winamp skins' and VLC themes' windows",
        Key::Part,
        Linux::Partly(
            "docked and moved together under X11; on a Wayland desktop RVP starts through XWayland with such a skin, since Wayland lets no program place its windows",
        ),
    ),
    item(
        "mpv's on-screen controllers",
        Key::Settings(&["controller.kind"]),
        Linux::Same("the packages' libmpv has Lua and mpv's built-in scripts, as Windows' does"),
    ),
    item(
        "DVD and Blu-ray menus, audio CDs",
        Key::Part,
        Linux::Same(
            "libdvdnav, libbluray and libcdio with the packages' libmpv, as on Windows; encrypted discs only with the user's own libdvdcss, libaacs and keys",
        ),
    ),
    item(
        "AviSynth scripts and plugins",
        Key::Settings(&["vapoursynth.avs-plugins"]),
        Linux::Partly(
            "`.avs` files open through FFmpeg's AviSynth demuxer in the packages' libmpv once the user installs AviSynth+ for Linux (`libavisynth.so`); AviSynth plugins inside VapourSynth scripts (`avs.LoadPlugin`) need VapourSynth's AviSynth bridge, which it has on Windows only",
        ),
    ),
    item(
        "Blu-ray Java menus (BD-J)",
        Key::Part,
        Linux::Partly(
            "libbluray runs BD-J with the user's Java runtime and its own Java part (Debian's and Ubuntu's libbluray-bdj package, `/usr/share/java`), which the packages do not carry",
        ),
    ),
    item(
        "Subtitles from audio (whisper.cpp)",
        Key::Part,
        Linux::Same(
            "whisper.cpp's CPU and Vulkan engine in the packages (`whisper/` beside rvp); CUDA's and BLAS's are whisper.cpp's own builds the user puts in the settings folder, as on Windows",
        ),
    ),
    item(
        "Motion smoothing's VapourSynth runtime",
        Key::Part,
        Linux::Same(
            "RVP's own runtime in the packages (`vapoursynth/` beside rvp: VapourSynth R73 on Python 3.12, MVTools, RIFE with its model), as on Windows; never SVP's",
        ),
    ),
    item(
        "Unified Remote's remote",
        Key::Part,
        Linux::Same(
            "Unified Remote Server for Linux; the Remote Control page's Add to Unified Remote copies the packages' remote into `~/.urserver/remotes/custom` (on Windows into `%ProgramData%\\Unified Remote\\Remotes\\Custom`)",
        ),
    ),
];

/// The report, `docs/coverage/linux.md`.
#[must_use]
pub fn report() -> String {
    let count = |kind: fn(&Linux) -> bool| ITEMS.iter().filter(|item| kind(&item.linux)).count();
    let mut out = format!(
        "# RVP on Linux\n\nGenerated by `cargo test -p rvp-app --test coverage` (`RVP_BLESS=1` rewrites it) \
         from `crates/app/src/coverage/linux.rs`: the parts whose Linux side differs from Windows', \
         and how Linux does them or why it cannot. Everything else works on Linux as on Windows: \
         it is libmpv's, FFmpeg's, Qt's or RVP's own code.\n\n\
         {} items: {} the same through Linux's means, {} in part, {} not.\n\n\
         | Part | RVP | On Linux |\n|------|-----|----------|\n",
        ITEMS.len(),
        count(|linux| matches!(linux, Linux::Same(_))),
        count(|linux| matches!(linux, Linux::Partly(_))),
        count(|linux| matches!(linux, Linux::Not(_))),
    );
    for item in ITEMS {
        let names = match item.key {
            Key::Actions(names) | Key::Settings(names) => names
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", "),
            Key::Part => String::new(),
        };
        let linux = match item.linux {
            Linux::Same(how) => format!("Yes: {how}"),
            Linux::Partly(how) => format!("In part: {how}"),
            Linux::Not(why) => format!("No: {why}"),
        };
        let _ = writeln!(out, "| {} | {names} | {linux} |", item.what);
    }
    out
}
