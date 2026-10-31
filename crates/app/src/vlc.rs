//! M9: VLC's variables and text codes for the skins RVP builds from VLC
//! themes (`rvp_skin::vlc`): what the window's conditions (`visible`, a
//! checkbox's `state`) and texts read.
//!
//! * The playback variables come from RVP's state, the window's from the
//!   shell ([`WindowState`]); [`VARIABLES`] says what each reads.
//! * Texts and tooltips take VLC's codes ([`TEXT_CODES`]) in one pass;
//!   other codes stay as they are.
//!
//! The tables here and `rvp_skin::vlc::build`'s are what
//! `docs/coverage/vlc.md` reports (`crate::coverage::vlc_report`).

use rvp_engine::PropertyId;

use crate::Repeat;
use crate::app::App;

/// What RVP reads for each of VLC's variables: those
/// `rvp_skin::vlc::build::VARIABLES` lists, then the window and layout ones
/// (window `W`, layout `L`).
pub const VARIABLES: [(&str, &str); 23] = [
    ("vlc.isPlaying", "a file plays and is not paused"),
    ("vlc.isPaused", "a file is paused"),
    ("vlc.isStopped", "no file is open"),
    (
        "vlc.isSeekable",
        "the file has a duration (VLC: its position is not 0)",
    ),
    ("vlc.isFullscreen", "the window is fullscreen"),
    ("vlc.hasVout", "the file has a video track"),
    ("vlc.hasAudio", "the file has an audio track"),
    ("vlc.isMute", "the sound is muted"),
    ("vlc.isOnTop", "the window stays on top"),
    (
        "vlc.canRecord",
        "a file is open: mpv cannot tell beforehand whether it can record a stream",
    ),
    ("vlc.isRecording", "RVP records the stream (Record Stream)"),
    (
        "dvd.isActive",
        "the file has chapters, as VLC reads it (also a chaptered MKV)",
    ),
    ("equalizer.isEnabled", "RVP's equalizer is on"),
    ("playlist.isRandom", "the album plays shuffled"),
    ("playlist.isLoop", "the album repeats"),
    ("playlist.isRepeat", "the file repeats"),
    ("playtree.isRandom", "as `playlist.isRandom`"),
    ("playtree.isLoop", "as `playlist.isLoop`"),
    ("playtree.isRepeat", "as `playlist.isRepeat`"),
    ("vlc.isAlwaysOnTop", "as `vlc.isOnTop`"),
    (
        "W.isVisible",
        "window `W` shows (the main window always does)",
    ),
    (
        "W.isMaximized",
        "`W` is the main window and it is maximized",
    ),
    (
        "L.isActive",
        "layout `L` of the main window shows; the other windows show their last layout",
    ),
];

/// VLC's text codes and what RVP puts in their place.
pub const TEXT_CODES: [(&str, &str); 13] = [
    ("$T", "the time, `H:MM:SS` (`-:--:--` without a position)"),
    ("$t", "the time, `MM:SS` below an hour"),
    ("$L", "the time left, `H:MM:SS`"),
    ("$l", "the time left, `MM:SS` below an hour"),
    ("$D", "the duration, `H:MM:SS`"),
    ("$d", "the duration, `MM:SS` below an hour"),
    (
        "$V",
        "the volume in RVP's percent (VLC's run from 0 to 200)",
    ),
    (
        "$N",
        "the title (mpv's `media-title`; VLC's follows its title format)",
    ),
    ("$F", "the file's path"),
    ("$B", "the audio bit rate in kb/s"),
    ("$S", "the sample rate in kHz"),
    ("$R", "the speed"),
    (
        "$H",
        "nothing: RVP keeps no help texts (VLC: the help of the control under the pointer)",
    ),
];

/// What a slider's `value` moves in RVP.
pub const SLIDERS: [(&str, &str); 5] = [
    ("time", "the position (`CMD_TIMEBAR_CTRL`)"),
    ("volume", "RVP's volume over its range (`CMD_VOLUME_CTRL`)"),
    (
        "equalizer.band(N)",
        "band N + 1 of RVP's ten-band equalizer (`CMD_EQULIZER_N`)",
    ),
    (
        "equalizer.preamp",
        "nothing: RVP's equalizer has no preamp (RVP's Preamp raises AC3 and DTS sources only), so the slider stays in the middle",
    ),
    (
        "playtree.slider",
        "left out: RVP's playlist scrolls itself and draws its own scroll bar",
    ),
];

/// Why RVP does nothing for some of VLC's actions, and how others differ
/// from VLC's.
pub const ACTION_NOTES: [(&str, &str); 11] = [
    ("none", "VLC's own no-op"),
    (
        "playlist.sort()",
        "VLC sorts at once, RVP opens its sort menu",
    ),
    (
        "playtree.sort()",
        "VLC sorts at once, RVP opens its sort menu",
    ),
    (
        "dialogs.streamingWizard()",
        "libmpv has no stream output (no transcoding or serving chain)",
    ),
    (
        "dvd.nextTitle()",
        "RVP has no step from title to title: a disc's titles are chosen in its menu (M16) or from DVD Title Menu",
    ),
    (
        "dvd.previousTitle()",
        "RVP has no step from title to title: a disc's titles are chosen in its menu (M16) or from DVD Title Menu",
    ),
    ("vlc.onTop()", "VLC turns it on and off"),
    ("dialogs.audioPopup()", "VLC's audio menu is part of RVP's"),
    ("dialogs.videoPopup()", "VLC's video menu is part of RVP's"),
    (
        "dialogs.miscPopup()",
        "VLC's playback menu is part of RVP's",
    ),
    (
        "dialogs.messages()",
        "VLC's message log; RVP shows its playback and system information",
    ),
];

/// VLC's window actions (`W` a window, `L` a layout) in RVP.
pub const WINDOW_ACTIONS: [(&str, &str); 5] = [
    (
        "W.show()",
        "shows window `W` (`RVPWIN:W:show`); nothing for the main window",
    ),
    ("W.hide()", "hides window `W`; the main window minimizes"),
    (
        "W.maximize()",
        "the main window: Maximize/Restore; nothing for the others",
    ),
    (
        "W.unmaximize()",
        "the main window: Maximize/Restore; nothing for the others",
    ),
    (
        "W.setLayout(L)",
        "the main window shows layout `L` (another file of the built skin); nothing for the others",
    ),
];

/// How RVP draws a theme's parts (`docs/coverage/vlc.md`).
pub const FEATURES: [(&str, &str); 22] = [
    (
        "`theme.xml`, `Bitmap`, `SubBitmap`, `Font`",
        "read from the `.vlt` (gzip'd tar or zip); `alphacolor` made transparent, the theme's fonts registered for the process",
    ),
    (
        "`Window`, `Layout`",
        "the window with the video (else the first shown) is RVP's main window and its layouts files of the built skin; the others are windows of their own in their last layout",
    ),
    (
        "`Window` `x`, `y`",
        "the other windows keep their place beside the main window and move with it",
    ),
    (
        "`Anchor`",
        "not read: windows neither snap to each other nor to the screen's edges, they move as one group",
    ),
    (
        "`Group`, `Panel`",
        "placed as VLC places them, with their corners and kept ratios",
    ),
    (
        "`lefttop`, `rightbottom`, `xkeepratio`, `ykeepratio`",
        "a control's corners follow its box's as the window resizes; a kept ratio near the middle centres it",
    ),
    (
        "`Image` `resize`",
        "`mosaic` tiles, `scale` stretches, `scale2` fits keeping the picture's aspect",
    ),
    (
        "`Image` `action`",
        "`move` drags the window, `resizeE`, `resizeS`, `resizeSE` resize it",
    ),
    (
        "`Image` `action2`",
        "a double click on a title-bar picture runs it (RVP's title bars maximize otherwise)",
    ),
    ("`Image` `art`", "drawn as the picture: no cover art"),
    (
        "`Button`, `Checkbox`",
        "up, over and down pictures; they take the pointer where their pictures are opaque, as VLC's do",
    ),
    (
        "animated bitmaps (`nbframes`)",
        "their first frame: RVP's skin pictures do not move",
    ),
    (
        "`Text`",
        "the theme's font, colour and alignment; a text wider than its box is cut where VLC scrolls it",
    ),
    (
        "`Text` `focus`",
        "ignored: RVP's labels do not take the pointer",
    ),
    (
        "`Slider`",
        "the straight track from its first point to its last (a curve, PsVLC's, becomes that line), with its background pictures",
    ),
    (
        "`RadialSlider`",
        "left out (no theme of the sample has one)",
    ),
    (
        "`Playtree`, `Playlist`",
        "RVP's playlist in the theme's colours and background; no tree, folder pictures or second row colour",
    ),
    (
        "`Video`",
        "the video's place; `autoresize` is not read (RVP's Fit window to video setting applies as with any skin)",
    ),
    (
        "`fullscreenController`",
        "left out: in fullscreen RVP shows the video alone (mpv's on-screen controller can give it controls)",
    ),
    ("`PopupMenu`, `MenuItem`", "left out: RVP's own menus"),
    ("tooltips", "the theme's, with their text codes filled in"),
    (
        "transparent pixels",
        "the windows take their shapes from the pictures' opaque pixels",
    ),
];

/// What the shell knows of its windows.
#[derive(Debug, Clone, Default)]
pub struct WindowState {
    pub fullscreen: bool,
    pub on_top: bool,
    pub maximized: bool,
    /// The main window's id and the other windows that show.
    pub main: String,
    pub shown: Vec<String>,
    /// The main window's layout that shows.
    pub layout: String,
}

/// `H:MM:SS`, or (`short`) `MM:SS` below an hour.
fn clock(seconds: f64, short: bool) -> String {
    // A file's times: far below u64's range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = seconds.max(0.0).floor() as u64;
    let (hours, minutes, secs) = (total / 3600, total / 60 % 60, total % 60);
    if short && hours == 0 {
        format!("{minutes:02}:{secs:02}")
    } else {
        format!("{hours}:{minutes:02}:{secs:02}")
    }
}

impl App {
    /// A VLC variable's value (`None`: one RVP does not know).
    #[must_use]
    pub fn vlc_variable(&self, name: &str, window: &WindowState) -> Option<bool> {
        let model = &self.model;
        let loaded = model.media_loaded();
        let paused = model.flag(PropertyId::Pause) == Some(true);
        let repeat = self.settings.repeat;
        Some(match name {
            "vlc.isPlaying" => loaded && !paused,
            "vlc.isPaused" => loaded && paused,
            "vlc.isStopped" => !loaded,
            "vlc.isSeekable" => {
                loaded && model.double(PropertyId::Duration).is_some_and(|d| d > 0.0)
            }
            "vlc.isFullscreen" => window.fullscreen,
            "vlc.hasVout" => loaded && model.int(PropertyId::VideoWidth).is_some(),
            "vlc.hasAudio" => loaded && model.int(PropertyId::AudioChannels).is_some(),
            "vlc.isMute" => model.flag(PropertyId::Mute) == Some(true),
            "vlc.isOnTop" | "vlc.isAlwaysOnTop" => window.on_top,
            "vlc.canRecord" => loaded,
            "vlc.isRecording" => self.recording_stream(),
            "dvd.isActive" => model
                .int(PropertyId::Chapters)
                .is_some_and(|count| count > 0),
            "equalizer.isEnabled" => self.settings.equalizer.enabled,
            "playlist.isRandom" | "playtree.isRandom" => self.settings.playback.shuffle,
            "playlist.isLoop" | "playtree.isLoop" => repeat == Repeat::Album,
            "playlist.isRepeat" | "playtree.isRepeat" => repeat == Repeat::Item,
            other => {
                let (id, what) = other.split_once('.')?;
                match what {
                    "isVisible" => id == window.main || window.shown.iter().any(|w| w == id),
                    "isMaximized" => id == window.main && window.maximized,
                    "isActive" => id == window.layout,
                    _ => return None,
                }
            }
        })
    }

    /// A text with VLC's codes filled in.
    #[must_use]
    pub fn vlc_text(&self, template: &str) -> String {
        if !template.contains('$') {
            return template.to_owned();
        }
        let model = &self.model;
        let loaded = model.media_loaded();
        let time = model.double(PropertyId::TimePos).filter(|_| loaded);
        let duration = model.double(PropertyId::Duration).filter(|_| loaded);
        let times = |value: Option<f64>, short: bool| {
            value.map_or_else(|| "-:--:--".to_owned(), |v| clock(v, short))
        };
        let left = time.zip(duration).map(|(t, d)| (d - t).max(0.0));
        let mut out = String::with_capacity(template.len());
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            let Some(&code) = chars.peek() else {
                out.push(c);
                break;
            };
            let value = match code {
                'T' => times(time, false),
                't' => times(time, true),
                'L' => times(left, false),
                'l' => times(left, true),
                'D' => times(duration, false),
                'd' => times(duration, true),
                'V' => format!(
                    "{:.0}",
                    crate::volume::percent(model.double(PropertyId::Volume).unwrap_or(0.0))
                ),
                'N' => model
                    .text(PropertyId::MediaTitle)
                    .filter(|_| loaded)
                    .unwrap_or("")
                    .to_owned(),
                'F' => model.loaded_path().unwrap_or("").to_owned(),
                'B' => model
                    .double(PropertyId::AudioBitrate)
                    .filter(|_| loaded)
                    .map(|bits| format!("{:.0}", bits / 1000.0))
                    .unwrap_or_default(),
                'S' => model
                    .int(PropertyId::AudioSampleRate)
                    .filter(|_| loaded)
                    .map(|rate| (rate / 1000).to_string())
                    .unwrap_or_default(),
                'R' => format!("{}", model.double(PropertyId::Speed).unwrap_or(1.0)),
                'H' => String::new(),
                _ => {
                    out.push(c);
                    continue;
                }
            };
            chars.next();
            out.push_str(&value);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variable_says_what_it_reads() {
        let dir = std::env::temp_dir().join(format!("rvp-vlc-{}", std::process::id()));
        let app = App::new(crate::Settings::default(), &dir).expect("app");
        let window = WindowState::default();
        for name in rvp_skin::vlc::build::VARIABLES {
            assert!(
                VARIABLES.iter().any(|(known, _)| *known == name),
                "{name} has no row"
            );
            assert!(
                app.vlc_variable(name, &window).is_some(),
                "RVP does not read {name}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clocks_as_vlc_prints_them() {
        assert_eq!(clock(42.0, false), "0:00:42");
        assert_eq!(clock(42.0, true), "00:42");
        assert_eq!(clock(3725.0, true), "1:02:05");
    }
}
