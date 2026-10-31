//! The RVP skin built from a VLC theme (M9): PotPlayer-format markup and
//! pictures in memory for RVP's engine.
//!
//! * The window that holds the video (else the first shown one) is RVP's
//!   main window; each of its layouts is a file of the built skin
//!   (`main.N.xml`, `VideoSkin.xml` the last layout's, which VLC shows
//!   first), and `W.setLayout(L)` loads another (`CMD_LOAD_SKIN_XML`).
//!   The other windows are windows of their own (`RvpWindow='1'`) in
//!   their last layout, placed where the theme puts them beside the main
//!   window; `W.show()` and `W.hide()` show and hide them.
//! * A control's corners become the engine's offsets from its box's far
//!   edges (negative `Left`/`Top`, `Width`/`Height` of zero or less), a
//!   kept ratio near the middle a centred control; a `Panel` is a box of
//!   its own, `Group` offsets are added.
//! * `visible` and a checkbox's `state` are conditions the window
//!   computes (`RvpConditions`, flags `VLCX:N`): a checkbox is two buttons,
//!   one for each state. Texts are `VLCTEXT:N` labels whose `$` codes the
//!   window fills (`RvpTexts`); their fonts are the theme's own files
//!   (`fonts/`), which the shell registers with the system's fonts.
//! * Images tile (`mosaic`) or stretch (`RvpStretch`); buttons get their
//!   up, over and down pictures side by side; a slider is its straight
//!   track (VLC's curve from its first to its last point) with the thumb
//!   centred on it and its background a picture per value (`RvpFrames`);
//!   buttons take the pointer where their pictures are opaque
//!   (`RvpHit='alpha'`), as VLC's do;
//!   a playlist is RVP's; a `Video` the video's slot. Images that move
//!   the window are title bars, those that resize it `RVPRESIZE:` grips.
//! * Actions become RVP's (`RVP:name`, [`action_command`]); VLC's dialogs
//!   and menus RVP's own. VLC's `enable()`-style actions are RVP's toggles
//!   that do nothing while the state is already the asked one
//!   (`RvpUnless`). A control whose action RVP cannot run (the streaming
//!   wizard: libmpv has no stream output) shows and does nothing; one whose
//!   action VLC does not know is left out, as VLC leaves it out. Of a `;`
//!   list RVP runs the first action it can.

use std::collections::BTreeMap;

use super::expr::Expr;
use super::{Control, Layout, Place, Theme, Window};
use crate::archive::SkinArchive;
use crate::compose::{El, crop, join};
use crate::error::SkinError;
use crate::image::Image;
use crate::skin::Skin;

/// The flag of condition `n` (`RvpConditions`).
pub const CONDITION: &str = "VLCX:";
/// The text command of template `n` (`RvpTexts`).
pub const TEXT: &str = "VLCTEXT:";
/// The command of an image that resizes the window (`E`, `S`, `SE`).
pub const RESIZE: &str = "RVPRESIZE:";

/// VLC's boolean variables that do not name a window or layout.
pub const VARIABLES: [&str; 20] = [
    "vlc.isPlaying",
    "vlc.isPaused",
    "vlc.isStopped",
    "vlc.isSeekable",
    "vlc.isFullscreen",
    "vlc.hasVout",
    "vlc.hasAudio",
    "vlc.isMute",
    "vlc.isOnTop",
    "vlc.canRecord",
    "vlc.isRecording",
    "dvd.isActive",
    "equalizer.isEnabled",
    "playlist.isRandom",
    "playlist.isLoop",
    "playlist.isRepeat",
    "playtree.isRandom",
    "playtree.isLoop",
    "playtree.isRepeat",
    "vlc.isAlwaysOnTop",
];

/// What the build collects across the theme.
struct Builder<'a> {
    theme: &'a Theme,
    files: Vec<(String, Image)>,
    /// Pictures already written, by their key.
    written: BTreeMap<String, String>,
    colors: Vec<[u8; 3]>,
    conditions: Vec<String>,
    texts: Vec<String>,
    /// The main window's id and its layouts' files.
    main: String,
    layout_files: BTreeMap<String, String>,
    /// Ids of every window and layout (variables `W.isVisible`...).
    windows: Vec<String>,
    layouts: Vec<String>,
}

impl Builder<'_> {
    /// The file of a theme picture.
    fn picture(&mut self, id: &str) -> Option<String> {
        if let Some(file) = self.written.get(id) {
            return Some(file.clone());
        }
        let image = self.theme.images.get(id)?.clone();
        let file = format!("i/{}.png", self.written.len());
        self.files.push((file.clone(), image));
        self.written.insert(id.to_owned(), file.clone());
        Some(file)
    }

    /// A strip of pictures side by side, written once per key.
    fn strip(&mut self, key: String, parts: &[&str]) -> Option<String> {
        if let Some(file) = self.written.get(&key) {
            return Some(file.clone());
        }
        let images: Option<Vec<Image>> = parts
            .iter()
            .map(|id| self.theme.images.get(*id).cloned())
            .collect();
        let file = format!("i/{}.png", self.written.len());
        self.files.push((file.clone(), join(&images?, false)));
        self.written.insert(key, file.clone());
        Some(file)
    }

    /// A colour's pixel in `colors.png`.
    fn color(&mut self, rgb: [u8; 3]) -> i32 {
        let index = self
            .colors
            .iter()
            .position(|c| *c == rgb)
            .unwrap_or_else(|| {
                self.colors.push(rgb);
                self.colors.len() - 1
            });
        i32::try_from(index).unwrap_or(0)
    }

    /// Whether every variable of an expression is one RVP knows.
    fn known(&self, expr: &Expr) -> bool {
        expr.eval(&mut |name| {
            let known = VARIABLES.contains(&name)
                || name.split_once('.').is_some_and(|(id, what)| match what {
                    "isVisible" | "isMaximized" => self.windows.iter().any(|w| w == id),
                    "isActive" => self.layouts.iter().any(|l| l == id),
                    _ => false,
                });
            known.then_some(true)
        })
        .is_some()
    }

    /// The flag of a condition: `visible` (when RVP knows it) and, for a
    /// checkbox's sets, its `state` or its opposite.
    fn condition(&mut self, visible: Option<&str>, state: Option<(&str, bool)>) -> Option<String> {
        let visible = visible.and_then(|text| {
            Expr::parse(text)
                .filter(|expr| self.known(expr))
                .map(|_| text)
        });
        let text = match (visible, state) {
            (None, None) => return None,
            (Some(v), None) => format!("( {v} )"),
            (None, Some((s, on))) => format!("{}( {s} )", if on { "" } else { "not " }),
            (Some(v), Some((s, on))) => {
                format!("( {v} ) and {}( {s} )", if on { "" } else { "not " })
            }
        };
        let index = self
            .conditions
            .iter()
            .position(|c| *c == text)
            .unwrap_or_else(|| {
                self.conditions.push(text);
                self.conditions.len() - 1
            });
        Some(format!("{CONDITION}{index}"))
    }

    /// What a control's action string becomes.
    fn run(&self, action: &str) -> Run {
        let mut first = None;
        for part in action
            .split(';')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            let command = if ACTIONS.contains(&part) {
                action_command(part).map(|(command, unless)| (command.to_owned(), unless))
            } else {
                match self.window_command(part) {
                    Run::Unknown => return Run::Unknown,
                    Run::Nothing => None,
                    Run::Command(command, unless) => Some((command, unless)),
                }
            };
            if first.is_none() {
                first = command;
            }
        }
        first.map_or(Run::Nothing, |(command, unless)| {
            Run::Command(command, unless)
        })
    }

    /// `W.show()`, `W.hide()`, `W.maximize()`, `W.unmaximize()`,
    /// `W.setLayout(L)` (`Run::Unknown`: none of them).
    fn window_command(&self, action: &str) -> Run {
        let Some((window, call)) = action.split_once('.') else {
            return Run::Unknown;
        };
        let main = window == self.main;
        let known = self.windows.iter().any(|w| w == window);
        let command = match call {
            "show()" => (known && !main).then(|| format!("RVPWIN:{window}:show")),
            "hide()" if main => Some("RVP:minimize".to_owned()),
            "hide()" => known.then(|| format!("RVPWIN:{window}:hide")),
            "maximize()" | "unmaximize()" => main.then(|| "RVP:maximize-restore".to_owned()),
            _ => {
                let Some(layout) = call
                    .strip_prefix("setLayout(")
                    .and_then(|rest| rest.strip_suffix(')'))
                else {
                    return Run::Unknown;
                };
                main.then(|| self.layout_files.get(layout))
                    .flatten()
                    .map(|file| format!("RVPFILE:{file}"))
            }
        };
        command.map_or(Run::Nothing, |command| Run::Command(command, None))
    }

    /// A control's command and its guard (`RvpUnless`); `None`: the control
    /// is left out.
    fn command_of(&mut self, action: &str) -> Option<(Option<String>, Option<String>)> {
        match self.run(action) {
            Run::Unknown => None,
            Run::Nothing => Some((None, None)),
            Run::Command(command, unless) => {
                let unless = unless.and_then(|expr| self.condition(Some(expr), None));
                Some((Some(command), unless))
            }
        }
    }
}

/// What a control's action string becomes.
enum Run {
    /// An RVP command, and the expression under which a click does nothing.
    Command(String, Option<&'static str>),
    /// Nothing: the control shows and does nothing.
    Nothing,
    /// An action VLC does not know: VLC leaves the control out.
    Unknown,
}

/// VLC 3's own actions (its interpreter's table), besides the window
/// actions `W.show()`, `W.hide()`, `W.maximize()`, `W.unmaximize()` and
/// `W.setLayout(L)`.
pub const ACTIONS: [&str; 53] = [
    "none",
    "dialogs.changeSkin()",
    "dialogs.fileSimple()",
    "dialogs.file()",
    "dialogs.directory()",
    "dialogs.disc()",
    "dialogs.net()",
    "dialogs.playlist()",
    "dialogs.messages()",
    "dialogs.prefs()",
    "dialogs.fileInfo()",
    "dialogs.streamingWizard()",
    "dialogs.popup()",
    "dialogs.audioPopup()",
    "dialogs.videoPopup()",
    "dialogs.miscPopup()",
    "playlist.load()",
    "playlist.save()",
    "playlist.add()",
    "playlist.del()",
    "playtree.del()",
    "playlist.next()",
    "playlist.previous()",
    "playlist.sort()",
    "playtree.sort()",
    "playlist.setRandom(true)",
    "playlist.setRandom(false)",
    "playlist.setLoop(true)",
    "playlist.setLoop(false)",
    "playlist.setRepeat(true)",
    "playlist.setRepeat(false)",
    "vlc.fullscreen()",
    "vlc.play()",
    "vlc.pause()",
    "vlc.stop()",
    "vlc.faster()",
    "vlc.slower()",
    "vlc.mute()",
    "vlc.volumeUp()",
    "vlc.volumeDown()",
    "vlc.minimize()",
    "vlc.onTop()",
    "vlc.quit()",
    "vlc.snapshot()",
    "vlc.toggleRecord()",
    "vlc.nextFrame()",
    "equalizer.enable()",
    "equalizer.disable()",
    "dvd.nextTitle()",
    "dvd.previousTitle()",
    "dvd.nextChapter()",
    "dvd.previousChapter()",
    "dvd.rootMenu()",
];

/// The skin command RVP runs for one of VLC's [`ACTIONS`], and the
/// expression under which a click does nothing (VLC's `enable()`-style
/// actions are RVP's toggles); `None`: the control does nothing (`none`,
/// and the actions `docs/coverage/vlc.md` explains).
#[must_use]
pub fn action_command(action: &str) -> Option<(&'static str, Option<&'static str>)> {
    let command = match action {
        "vlc.play()" => "RVP:play",
        "vlc.pause()" => "RVP:play-pause",
        "vlc.stop()" => "RVP:close-media",
        "vlc.faster()" => "RVP:speed-up",
        "vlc.slower()" => "RVP:speed-down",
        "vlc.mute()" => "RVP:toggle-mute",
        "vlc.volumeUp()" => "RVP:volume-up",
        "vlc.volumeDown()" => "RVP:volume-down",
        "vlc.fullscreen()" => "RVP:toggle-fullscreen",
        "vlc.snapshot()" => "RVP:save-frame",
        "vlc.toggleRecord()" => "RVP:record-stream-toggle",
        "vlc.nextFrame()" => "RVP:frame-forward",
        "vlc.onTop()" => "RVP:on-top-cycle",
        "vlc.minimize()" => "RVP:minimize",
        "vlc.quit()" => "RVP:exit",
        "playlist.add()" | "dialogs.fileSimple()" | "dialogs.file()" => "RVP:open-files",
        "playlist.del()" | "playtree.del()" => "CMD_PLAYLIST_DEL",
        "playlist.next()" => "RVP:next",
        "playlist.previous()" => "RVP:previous",
        "playlist.sort()" | "playtree.sort()" => "CMD_PLAYLIST_MENU_SORT",
        "playlist.setRandom(true)" => return Some(("RVP:shuffle", Some("playlist.isRandom"))),
        "playlist.setRandom(false)" => return Some(("RVP:shuffle", Some("not playlist.isRandom"))),
        "playlist.setLoop(true)" => "RVP:repeat-album",
        "playlist.setRepeat(true)" => "RVP:repeat-item",
        "playlist.setLoop(false)" | "playlist.setRepeat(false)" => "RVP:repeat-off",
        "playlist.load()" => "RVP:open-playlist",
        "playlist.save()" => "RVP:save-playlist",
        "dvd.nextChapter()" => "RVP:next-chapter",
        "dvd.previousChapter()" => "RVP:previous-chapter",
        "dvd.rootMenu()" => "RVP:dvd-root-menu",
        "equalizer.enable()" => return Some(("RVP:toggle-equalizer", Some("equalizer.isEnabled"))),
        "equalizer.disable()" => {
            return Some(("RVP:toggle-equalizer", Some("not equalizer.isEnabled")));
        }
        "dialogs.changeSkin()" => "RVP:skin-settings",
        "dialogs.directory()" => "RVP:open-folder",
        "dialogs.disc()" => "RVP:open-dvd",
        "dialogs.net()" => "RVP:open-url",
        "dialogs.playlist()" => "RVP:toggle-playlist",
        "dialogs.messages()" => "RVP:system-info",
        "dialogs.prefs()" => "RVP:preferences",
        "dialogs.fileInfo()" => "RVP:playback-info",
        "dialogs.popup()"
        | "dialogs.audioPopup()"
        | "dialogs.videoPopup()"
        | "dialogs.miscPopup()" => "RVP:context-menu",
        _ => return None,
    };
    Some((command, None))
}

/// A control's command and guard ([`Builder::command_of`]); a layout's file
/// loads through the engine's `CMD_LOAD_SKIN_XML`.
fn commanded(el: El, (command, unless): (Option<String>, Option<String>)) -> El {
    let el = match command
        .as_deref()
        .map(|command| (command, command.strip_prefix("RVPFILE:")))
    {
        Some((_, Some(file))) => el.a("CmdID", "CMD_LOAD_SKIN_XML").a("String", file),
        Some((command, None)) => el.a("CmdID", command),
        None => el,
    };
    match unless {
        Some(flag) => el.a("RvpUnless", flag),
        None => el,
    }
}

/// The engine's place of a control in its box: each edge from the box's
/// near edge (`Left`, `Top`, the size) or, tied to the far one, at a
/// signed distance from it (`RvpLeftFar`, `RvpRightFar`...); a kept ratio
/// near the middle centres it.
fn place(el: El, place: &Place) -> El {
    let el = axis(
        el,
        ["Left", "Width", "RvpLeftFar", "RvpRightFar", "HoriCenter"],
        place.x,
        place.w,
        place.box_w,
        place.lefttop.right,
        place.rightbottom.right,
        place.keep_x,
    );
    axis(
        el,
        ["Top", "Height", "RvpTopFar", "RvpBottomFar", "VertCenter"],
        place.y,
        place.h,
        place.box_h,
        place.lefttop.bottom,
        place.rightbottom.bottom,
        place.keep_y,
    )
}

/// One axis's attributes (`names`: offset, size, far start, far end,
/// centred).
#[allow(clippy::too_many_arguments)] // One axis of VLC's place.
fn axis(
    el: El,
    names: [&str; 5],
    start: i32,
    size: i32,
    whole: i32,
    start_far: bool,
    end_far: bool,
    keep: bool,
) -> El {
    let [offset, length, far_start, far_end, centred] = names;
    if keep {
        let free = whole - size;
        let ratio = if free > 0 {
            f64::from(start) / f64::from(free)
        } else {
            0.5
        };
        if (0.35..=0.65).contains(&ratio) {
            return el
                .a(offset, start - free / 2)
                .a(length, size.max(1))
                .a(centred, 1);
        }
        return if ratio > 0.5 {
            el.a(far_start, start - whole)
                .a(far_end, start + size - whole)
        } else {
            el.a(offset, start).a(length, size.max(1))
        };
    }
    match (start_far, end_far) {
        (false, false) => el.a(offset, start).a(length, size.max(1)),
        (false, true) => el.a(offset, start).a(far_end, start + size - whole),
        (true, _) => el
            .a(far_start, start - whole)
            .a(far_end, start + size - whole),
    }
}

/// The theme's controls as the engine's elements.
// One arm per VLC control, each short: the match reads as VLC's element
// list.
#[allow(clippy::too_many_lines)]
fn controls(
    builder: &mut Builder<'_>,
    controls: &[Control],
    main: bool,
    area_colors: &mut Vec<(&'static str, i32)>,
) -> Vec<El> {
    let mut out = Vec::new();
    for control in controls {
        match control {
            Control::Panel { place: p, children } => {
                let inner = self::controls(builder, children, main, area_colors);
                let mut el = place(El::new("Static"), p).a("RvpNoClip", 1);
                for child in inner {
                    el = el.child(child);
                }
                out.push(el);
            }
            Control::Image {
                place: p,
                image,
                resize,
                action,
                double,
                visible,
            } => {
                let Some(file) = builder.picture(image) else {
                    continue;
                };
                let (w, h) = builder.theme.size(image).unwrap_or((0, 0));
                let mut el = place(El::new("Static"), p).image(&file, 0, 0, w, h);
                match resize.as_str() {
                    "mosaic" => {}
                    "scale" => el = el.a("RvpStretch", 1),
                    _ => el = el.a("RvpStretch", "fit"),
                }
                match action.as_str() {
                    "move" => {
                        el = el.a("IsTitleBar", 1);
                        // A double click on it runs `action2` (RVP's title bars
                        // maximize).
                        if let Some((Some(command), _)) = builder.command_of(double) {
                            el = el.a("RvpDoubleCmd", command);
                        }
                    }
                    "resizeE" => el = el.a("CmdID", format!("{RESIZE}E")),
                    "resizeS" => el = el.a("CmdID", format!("{RESIZE}S")),
                    "resizeSE" => el = el.a("CmdID", format!("{RESIZE}SE")),
                    _ => {}
                }
                if let Some(flag) = builder.condition(visible.as_deref(), None) {
                    el = el.a("State", flag);
                }
                out.push(el);
            }
            Control::Button {
                place: p,
                up,
                down,
                over,
                action,
                tooltip,
                visible,
            } => {
                let Some(command) = builder.command_of(action) else {
                    continue;
                };
                let Some(file) = builder.strip(format!("{up}|{over}|{down}"), &[up, over, down])
                else {
                    continue;
                };
                let mut el = commanded(place(El::new("Button"), p), command)
                    .a("Tooltip", tooltip)
                    .image(&file, 0, 0, p.w, p.h)
                    .a("RvpHit", "alpha");
                if let Some(flag) = builder.condition(visible.as_deref(), None) {
                    el = el.a("State", flag);
                }
                out.push(el);
            }
            Control::Checkbox {
                place: p,
                images,
                actions,
                tooltips,
                state,
                visible,
            } => {
                let Some(expr) = Expr::parse(state) else {
                    continue;
                };
                if !builder.known(&expr) {
                    continue;
                }
                let (Some(first), Some(second)) = (
                    builder.command_of(&actions[0]),
                    builder.command_of(&actions[1]),
                ) else {
                    continue;
                };
                for (set, on, command) in [(0, false, first), (1, true, second)] {
                    let [up, down, over] = &images[set];
                    let Some(file) =
                        builder.strip(format!("{up}|{over}|{down}"), &[up, over, down])
                    else {
                        continue;
                    };
                    let flag = builder.condition(visible.as_deref(), Some((state, on)));
                    let mut el = commanded(place(El::new("Button"), p), command)
                        .a("Tooltip", &tooltips[set])
                        .image(&file, 0, 0, p.w, p.h)
                        .a("RvpHit", "alpha");
                    if let Some(flag) = flag {
                        el = el.a("State", flag);
                    }
                    out.push(el);
                }
            }
            Control::Text {
                place: p,
                font,
                text,
                color,
                alignment,
                visible,
            } => {
                let Some(font) = builder.theme.fonts.get(font).cloned() else {
                    continue;
                };
                let index = builder
                    .texts
                    .iter()
                    .position(|t| t == text)
                    .unwrap_or_else(|| {
                        builder.texts.push(text.clone());
                        builder.texts.len() - 1
                    });
                let color = builder.color(*color);
                let mut el = place(El::new("Label"), p)
                    .a("CmdID", format!("{TEXT}{index}"))
                    .a("FontName", font.family)
                    .a("FontSize", font.size)
                    .a("VertAlignment", 1)
                    .a(
                        "HoriAlignment",
                        match alignment.as_str() {
                            "center" | "centre" => 1,
                            "right" => 2,
                            _ => 0,
                        },
                    )
                    .a("Image", "colors.png")
                    .a("ColorX", color)
                    .a("ColorY", 0);
                if let Some(flag) = builder.condition(visible.as_deref(), None) {
                    el = el.a("State", flag);
                }
                out.push(el);
            }
            Control::Slider {
                place: p,
                up,
                down,
                points,
                value,
                tooltip,
                background,
                visible,
            } => {
                let command = match value.as_str() {
                    "time" => "CMD_TIMEBAR_CTRL".to_owned(),
                    "volume" => "CMD_VOLUME_CTRL".to_owned(),
                    "equalizer.preamp" => "RVP:preamp".to_owned(),
                    band => {
                        let Some(index) = band
                            .strip_prefix("equalizer.band(")
                            .and_then(|rest| rest.strip_suffix(')'))
                            .and_then(|index| index.parse::<u8>().ok())
                            .filter(|index| *index < 10)
                        else {
                            // The playlist's scroll bar and unknown values:
                            // RVP's list scrolls itself.
                            continue;
                        };
                        format!("CMD_EQULIZER_{index}")
                    }
                };
                if let Some(el) = slider(
                    builder,
                    p,
                    up,
                    down,
                    points,
                    &command,
                    tooltip,
                    background.as_ref(),
                    visible.as_deref(),
                ) {
                    out.push(el);
                }
            }
            Control::Playtree {
                place: p,
                background,
                fg,
                play,
                bg,
                selection,
                visible,
            } => {
                if let Some(file) = background.as_deref().and_then(|id| builder.picture(id)) {
                    let (w, h) = background
                        .as_deref()
                        .and_then(|id| builder.theme.size(id))
                        .unwrap_or((0, 0));
                    out.push(
                        place(El::new("Static"), p)
                            .image(&file, 0, 0, w, h)
                            .a("RvpStretch", 1),
                    );
                }
                let (fg, play, bg, selection) = (
                    builder.color(*fg),
                    builder.color(*play),
                    builder.color(*bg),
                    builder.color(*selection),
                );
                area_colors.clear();
                area_colors.extend([
                    ("TextColorX", fg),
                    ("SelTextColorX", fg),
                    ("PlayedTextColorX", fg),
                    ("CurTextColorX", play),
                    ("FrameColorX", play),
                    ("BkgndColor1X", bg),
                    ("SelBkgndColorX", selection),
                ]);
                let mut el = place(El::new("Static"), p)
                    .a("Contain", "CMD_PLAY_LIST_ALBUM_TAB")
                    .a("Image", "colors.png");
                if background.is_some() {
                    el = el.a("RvpTransparent", 1);
                }
                if let Some(flag) = builder.condition(visible.as_deref(), None) {
                    el = el.a("State", flag);
                }
                out.push(el);
            }
            Control::Video { place: p, visible } => {
                if !main {
                    continue;
                }
                let mut el = place(El::new("Static"), p).a("Contain", "CMD_VIDEO_WINDOW");
                if let Some(flag) = builder.condition(visible.as_deref(), None) {
                    el = el.a("State", flag);
                }
                out.push(el);
            }
        }
    }
    out
}

/// A straight slider: its track from the first to the last point, the
/// thumb (up, down) centred on it; the background's pictures, one per
/// value, stacked below the thumbs in the same file.
#[allow(clippy::too_many_arguments)] // A slider's parts, as VLC lists them.
fn slider(
    builder: &mut Builder<'_>,
    p: &Place,
    up: &str,
    down: &str,
    points: &[(i32, i32)],
    command: &str,
    tooltip: &str,
    background: Option<&super::SliderBackground>,
    visible: Option<&str>,
) -> Option<El> {
    let (tw, th) = builder.theme.size(up)?;
    let (first, last) = (*points.first()?, *points.last()?);
    let vertical = (last.1 - first.1).abs() > (last.0 - first.0).abs();
    let track = if vertical {
        (
            first.0 - tw / 2,
            first.1.min(last.1) - th / 2,
            tw,
            (last.1 - first.1).abs() + th,
        )
    } else {
        (
            first.0.min(last.0) - tw / 2,
            first.1 - th / 2,
            (last.0 - first.0).abs() + tw,
            th,
        )
    };
    let thumbs = join(
        &[
            builder.theme.images.get(up)?.clone(),
            builder.theme.images.get(down)?.clone(),
        ],
        false,
    );
    let mut parts = vec![thumbs];
    let mut frames = None;
    if let Some(back) = background {
        let image = builder.theme.images.get(&back.image)?;
        let (iw, ih) = (
            i32::try_from(image.width()).ok()?,
            i32::try_from(image.height()).ok()?,
        );
        let (step_x, step_y) = (
            (iw + back.pad_x) / back.columns,
            (ih + back.pad_y) / back.rows,
        );
        let size = (step_x - back.pad_x, step_y - back.pad_y);
        let mut pictures = Vec::new();
        for row in 0..back.rows {
            for column in 0..back.columns {
                pictures.push(crop(image, column * step_x, row * step_y, size.0, size.1));
            }
        }
        parts.push(join(&pictures, true));
        frames = Some((back.rows * back.columns, size));
    }
    let file = format!("i/{}.png", builder.files.len() + builder.written.len());
    builder.files.push((file.clone(), join(&parts, true)));
    let (left, top, width, height) = track;
    // The track inside the control, which VLC anchors as a whole.
    let shifted = Place {
        x: p.x + left,
        y: p.y + top,
        w: width,
        h: height,
        ..*p
    };
    let mut el = place(El::new("Slider"), &shifted)
        .a("CmdID", command)
        .a("Tooltip", tooltip)
        .a("Image", &file)
        .a("ThumbWidth", tw)
        .a("ThumbHeight", th)
        .a("ThumbImageLeft", 0)
        .a("ThumbImageTop", 0)
        .a("ThumbPressedImageLeft", tw)
        .a("ThumbPressedImageTop", 0);
    if vertical {
        el = el.a("Vert", 1);
    }
    if let Some((count, (fw, fh))) = frames {
        // VLC draws the background at the control's corner.
        el = el
            .a("ImageWidth", fw)
            .a("ImageHeight", fh)
            .a("BackImageLeft", 0)
            .a("BackImageTop", th)
            .a("RvpFrames", count)
            .a("RvpFrameOffsetX", -left)
            .a("RvpFrameOffsetY", -top);
    }
    if let Some(flag) = builder.condition(visible, None) {
        el = el.a("State", flag);
    }
    Some(el)
}

/// The window that is RVP's main one: the one with the video in its last
/// layout, else in any layout, else the first shown, else the first.
fn main_window(theme: &Theme) -> Option<&Window> {
    theme
        .windows
        .iter()
        .find(|w| w.layouts.last().is_some_and(Layout::has_video))
        .or_else(|| {
            theme
                .windows
                .iter()
                .find(|w| w.layouts.iter().any(Layout::has_video))
        })
        .or_else(|| {
            theme
                .windows
                .iter()
                .find(|w| w.visible && w.id != "fullscreenController")
        })
        .or_else(|| theme.windows.first())
}

/// An area of a layout.
fn area(builder: &mut Builder<'_>, name: &'static str, layout: &Layout, main: bool) -> El {
    let mut area_colors = Vec::new();
    let children = controls(builder, &layout.controls, main, &mut area_colors);
    let mut root = El::new("Static")
        .a("Align", "Client")
        .a("Image", "colors.png");
    for child in children {
        root = root.child(child);
    }
    let mut el = El::new(name)
        .a("Width", layout.width)
        .a("Height", layout.height)
        .a("WidthLimit", layout.min.0)
        .a("HeightLimit", layout.min.1)
        .a("RvpMaxWidth", layout.max.0)
        .a("RvpMaxHeight", layout.max.1)
        .a("UseWindowFrame", 0)
        .a("RvpShape", "alpha")
        .a("AlbumTabH", 0);
    for (name, x) in area_colors {
        el = el.a(name, x).a(&name.replace('X', "Y"), 0);
    }
    el.child(El::new("Control").child(root))
}

/// The built skin's files: `VideoSkin.xml` and `main.N.xml`, their
/// pictures, `colors.png` and the theme's fonts.
///
/// # Errors
/// Fails without a window, or when a picture cannot be encoded.
pub fn members(theme: &Theme) -> Result<Vec<(String, Vec<u8>)>, SkinError> {
    let main = main_window(theme).ok_or_else(|| SkinError::MissingArea("Window".to_owned()))?;
    let mut builder = Builder {
        theme,
        files: Vec::new(),
        written: BTreeMap::new(),
        colors: vec![[0, 0, 0]],
        conditions: Vec::new(),
        texts: Vec::new(),
        main: main.id.clone(),
        layout_files: main
            .layouts
            .iter()
            .enumerate()
            .map(|(index, layout)| (layout.id.clone(), format!("main.{index}.xml")))
            .collect(),
        windows: theme.windows.iter().map(|w| w.id.clone()).collect(),
        layouts: theme
            .windows
            .iter()
            .flat_map(|w| w.layouts.iter().map(|l| l.id.clone()))
            .collect(),
    };
    // The other windows, the same in every file.
    let mut extras = Vec::new();
    let main_layout = main.layouts.last();
    for window in theme
        .windows
        .iter()
        .filter(|w| w.id != main.id && w.id != "fullscreenController")
    {
        let Some(layout) = window.layouts.last() else {
            continue;
        };
        let name: &'static str =
            Box::leak(format!("VlcWindow_{}", sanitize(&window.id)).into_boxed_str());
        let mut el = area(&mut builder, name, layout, false)
            .a("RvpWindow", 1)
            .a("RvpToggle", format!("RVPWIN:{}", window.id))
            .a("RvpShown", i32::from(window.visible));
        match (window.x, window.y, main.x, main.y, main_layout) {
            (Some(x), Some(y), Some(mx), Some(my), _) => {
                el = el
                    .a("RvpDock", "offset")
                    .a("RvpOffsetX", x - mx)
                    .a("RvpOffsetY", y - my);
            }
            (.., Some(layout)) => {
                el = el
                    .a("RvpDock", "offset")
                    .a("RvpOffsetX", layout.width)
                    .a("RvpOffsetY", 0);
            }
            _ => el = el.a("RvpDock", "below"),
        }
        extras.push(el);
    }
    let mut files = Vec::new();
    for (index, layout) in main.layouts.iter().enumerate() {
        let video = area(&mut builder, "MainSkinArea", layout, true)
            .a("RvpLayout", &layout.id)
            .a("RvpWindowId", &main.id);
        files.push((format!("main.{index}.xml"), video));
    }
    let mut members = Vec::new();
    let last = files.len().saturating_sub(1);
    for (index, (file, video)) in files.into_iter().enumerate() {
        let markup = file_markup(video, &extras, &builder.conditions, &builder.texts);
        if index == last {
            members.push(("VideoSkin.xml".to_owned(), markup.clone().into_bytes()));
        }
        members.push((file, markup.into_bytes()));
    }
    let mut colors = Image::new(u32::try_from(builder.colors.len()).unwrap_or(1).max(1), 1);
    for (index, rgb) in builder.colors.iter().enumerate() {
        colors.data_mut()[index * 4..index * 4 + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    members.push(("colors.png".to_owned(), colors.encode_png()?));
    for (name, image) in &builder.files {
        members.push((name.clone(), image.encode_png()?));
    }
    for (name, bytes) in &theme.font_files {
        let base = name.rsplit('/').next().unwrap_or(name);
        members.push((format!("fonts/{base}"), bytes.clone()));
    }
    Ok(members)
}

/// One file of the built skin: the main window's layout `video`, the video
/// alone for fullscreen, the other windows and the conditions and texts.
fn file_markup(video: El, extras: &[El], conditions: &[String], texts: &[String]) -> String {
    let only_video = El::new("Static")
        .a("Align", "Client")
        .a("Contain", "CMD_VIDEO_WINDOW");
    let mut root = El::new("Skin")
        .a("Version", "1.0")
        .a("MinorVersion", "1000")
        .child(video)
        .child(El::new("MainSkinAreaOscInside").child(El::new("Control").child(only_video)));
    for extra in extras {
        root = root.child(extra.clone());
    }
    let mut table = El::new("RvpConditions");
    for (n, condition) in conditions.iter().enumerate() {
        table = table.child(El::new("C").a("N", n).a("X", condition));
    }
    root = root.child(table);
    let mut table = El::new("RvpTexts");
    for (n, text) in texts.iter().enumerate() {
        table = table.child(El::new("T").a("N", n).a("X", text));
    }
    root = root.child(table);
    let mut markup = String::from("<?XML version='1.0' ?>\n");
    root.write(&mut markup, 0);
    markup
}

/// An id as part of an element name.
fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Builds the RVP skin of a theme and loads `file` of it (`None`: the
/// first layout VLC shows).
///
/// # Errors
/// As [`members`], or when the file cannot be loaded.
pub fn skin(theme: &Theme, file: Option<&str>) -> Result<Skin, SkinError> {
    let members = members(theme)?;
    Skin::load(
        SkinArchive::from_files(&theme.name, members),
        file.unwrap_or("VideoSkin.xml"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(el: &El) -> String {
        let mut out = String::new();
        el.write(&mut out, 0);
        out
    }

    #[test]
    fn corners_become_distances_from_the_far_edges() {
        let names = ["Left", "Width", "RvpLeftFar", "RvpRightFar", "HoriCenter"];
        let x = |start, size, whole, a, b, keep| {
            attrs(&axis(El::new("S"), names, start, size, whole, a, b, keep))
        };
        // Tied to the left: as it is.
        assert_eq!(
            x(10, 20, 100, false, false, false),
            "<S Left='10' Width='20' />\n"
        );
        // Its right edge to the box's right: 70 pixels before it.
        assert_eq!(
            x(10, 20, 100, false, true, false),
            "<S Left='10' RvpRightFar='-70' />\n"
        );
        // Both at the right, even beyond a box of no width (VLC's panels).
        assert_eq!(
            x(10, 500, 0, true, true, false),
            "<S RvpLeftFar='10' RvpRightFar='510' />\n"
        );
        // A kept ratio in the middle: centred.
        assert_eq!(
            x(40, 20, 100, false, false, true),
            "<S Left='0' Width='20' HoriCenter='1' />\n"
        );
    }
}
