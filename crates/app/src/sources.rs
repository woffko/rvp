//! M11's ways of opening (PotPlayer's Open menu, Device pages and "When
//! Files Opened"):
//!
//! * discs: a DVD (`dvd://`, a folder holding `VIDEO_TS` or an `.IFO` in
//!   it), a Blu-ray (`bd://`, a folder holding `BDMV` or an `.MPLS`
//!   playlist: `bd://mpls/N`) and an audio CD (`cdda://`), each with its
//!   device in the address (`dvd://longest/D:/`), so an album entry plays
//!   the same disc again; the discs' menus are M16's;
//! * capture: the screen (FFmpeg's `gdigrab`, or `ddagrab` through lavfi)
//!   and webcams and other devices (`dshow` on Windows), listed by the
//!   shell from the system;
//! * addresses FFmpeg opens (HTTP, HTTPS, FTP, `WebDAV` as `dav://`), and
//!   streaming sites through the user's yt-dlp ([`crate::ytdl`]);
//! * the clipboard (file paths, `file://` URLs and other addresses, one a
//!   line), recent items and favorites ([`crate::marks`]);
//! * the Default Open Action, and "When Files Opened": a file opened alone
//!   brings the folder's similar files (the same name but its numbers and
//!   the same kind of extension) or all its media files, in name order,
//!   and the file opened plays;
//! * Load External Audio (`audio-add`) and Reload Subtitle (`sub-reload`).
//!
//! What libmpv cannot open (its `protocol-list` lacks the protocol) is
//! refused with a notice.

use std::path::{Path, PathBuf};

use rvp_engine::{Engine, EngineError, PropertyId};
use serde::{Deserialize, Serialize};

use crate::action::{Action, DefaultOpen, Source, WhenOpened};
use crate::app::{App, UiRequest};

/// Audio and video file extensions RVP adds from folders.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "3gp", "asf", "avi", "flv", "m2ts", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "mts", "ogm",
    "ogv", "rm", "rmvb", "ts", "vob", "webm", "wmv",
];
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "aac", "ac3", "aiff", "ape", "dts", "flac", "m4a", "mka", "mp3", "ogg", "opus", "wav", "wma",
    "wv",
];

/// How the screen is captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capture {
    /// FFmpeg's GDI grabber (any desktop).
    #[default]
    Gdigrab,
    /// FFmpeg's Desktop Duplication source (`ddagrab`, through lavfi).
    Ddagrab,
}

/// The Device pages' settings (`devices` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Devices {
    pub capture: Capture,
    /// Frames a second of the screen capture.
    pub capture_fps: u32,
    /// The pointer shows in the capture.
    pub capture_pointer: bool,
    /// The webcam or device opened without asking (empty: ask).
    pub webcam: String,
    /// The DVD drive or folder opened without asking (empty: ask).
    pub dvd: String,
    /// The Blu-ray drive or folder opened without asking (empty: ask).
    pub bluray: String,
    /// The CD drive (empty: libmpv's default).
    pub cd: String,
}

impl Default for Devices {
    fn default() -> Self {
        Self {
            capture: Capture::Gdigrab,
            capture_fps: 30,
            capture_pointer: true,
            webcam: String::new(),
            dvd: String::new(),
            bluray: String::new(),
            cd: String::new(),
        }
    }
}

/// The Open menu's settings (`open` in `settings.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct OpenSettings {
    pub default_open: DefaultOpen,
    pub when_opened: WhenOpened,
    /// Recent items are stored and listed in Album/Favorites.
    pub recent: bool,
    /// Streaming sites are resolved with yt-dlp.
    pub ytdl: bool,
    /// The yt-dlp program (a name on the PATH or a full path).
    pub ytdl_path: String,
    /// yt-dlp's format choice (`-f`).
    pub ytdl_format: String,
}

impl Default for OpenSettings {
    fn default() -> Self {
        Self {
            default_open: DefaultOpen::Files,
            when_opened: WhenOpened::Selected,
            recent: true,
            ytdl: true,
            ytdl_path: if cfg!(windows) {
                "yt-dlp.exe".to_owned()
            } else {
                "yt-dlp".to_owned()
            },
            ytdl_format: "bestvideo[height<=?1080]+bestaudio/best".to_owned(),
        }
    }
}

impl OpenSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let one_line = |text: &str| text.len() <= 1000 && !text.chars().any(char::is_control);
        if !one_line(&self.ytdl_path) || self.ytdl_path.trim().is_empty() {
            notes.push("open.ytdl-path must be a program: the default used".to_owned());
            self.ytdl_path = Self::default().ytdl_path;
        }
        if !one_line(&self.ytdl_format) {
            notes.push("open.ytdl-format must be one line: the default used".to_owned());
            self.ytdl_format = Self::default().ytdl_format;
        }
    }
}

impl Devices {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        if !(1..=120).contains(&self.capture_fps) {
            let fps = self.capture_fps.clamp(1, 120);
            notes.push(format!("devices.capture-fps must be 1 to 120: {fps} used"));
            self.capture_fps = fps;
        }
        for (name, value) in [
            ("devices.webcam", &mut self.webcam),
            ("devices.dvd", &mut self.dvd),
            ("devices.bluray", &mut self.bluray),
            ("devices.cd", &mut self.cd),
        ] {
            if value.len() > 1000 || value.chars().any(char::is_control) {
                notes.push(format!("{name} must be one line: cleared"));
                value.clear();
            }
        }
    }
}

/// What a text from the clipboard is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paste {
    /// Open it (Ctrl+V in the window).
    Open,
    /// Add it after the entries of the shown album.
    List,
    /// Add it below the selected entry.
    ListBelow,
}

/// A capture device the shell found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureDevice {
    /// The name FFmpeg opens it by.
    pub name: String,
    pub audio: bool,
}

/// True for media files by extension.
#[must_use]
pub fn is_media(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        VIDEO_EXTENSIONS
            .iter()
            .chain(AUDIO_EXTENSIONS)
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

fn is_audio(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        AUDIO_EXTENSIONS
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// A file name with its numbers taken out, for "similar files"
/// (`Show S01E02.mkv` and `Show S01E03.mkv` are alike).
fn name_pattern(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect()
}

/// The media files of a folder, sorted by name (any case), not looking
/// into subfolders.
#[must_use]
pub fn folder_media(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && is_media(path))
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|path| {
        (
            path.file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default(),
            path.clone(),
        )
    });
    files
}

/// The files "When Files Opened" adds for one opened file: itself, or
/// with the folder's similar or all media files (in name order).
#[must_use]
pub fn files_for(opened: &Path, rule: WhenOpened) -> Vec<PathBuf> {
    let Some(folder) = opened.parent() else {
        return vec![opened.to_path_buf()];
    };
    if rule == WhenOpened::Selected || !opened.is_file() {
        return vec![opened.to_path_buf()];
    }
    let pattern = name_pattern(opened);
    let audio = is_audio(opened);
    let mut files: Vec<PathBuf> = folder_media(folder)
        .into_iter()
        .filter(|path| {
            rule == WhenOpened::All || (is_audio(path) == audio && name_pattern(path) == pattern)
        })
        .collect();
    if !files.iter().any(|path| path == opened) {
        files.push(opened.to_path_buf());
    }
    files
}

/// Paths and addresses in a text: one a line, `file://` URLs as paths,
/// quotes taken off.
#[must_use]
pub fn clipboard_items(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim().trim_matches('"').trim())
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            if let Some(rest) = line.strip_prefix("file://") {
                let path = percent_decode(rest);
                // `file:///C:/x` on Windows.
                let path = if cfg!(windows) {
                    path.trim_start_matches('/').to_owned()
                } else {
                    path
                };
                return Some(path);
            }
            if crate::playlist::is_url(line) || Path::new(line).is_absolute() {
                return Some(line.to_owned());
            }
            None
        })
        .collect()
}

/// `%20` and the like in a `file://` URL.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Some(byte) = std::str::from_utf8(&bytes[index + 1..index + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The disc a path names: a drive or folder, or a file inside the disc
/// (`VIDEO_TS.IFO`, `00000.mpls`), as the address libmpv opens.
#[must_use]
pub fn disc_address(path: &Path, bluray: bool) -> String {
    let file = path.is_file()
        || path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("ifo") || extension.eq_ignore_ascii_case("mpls")
        });
    if bluray {
        // `BDMV/PLAYLIST/00001.mpls`: the disc is two folders up.
        if file {
            let number = path
                .file_stem()
                .and_then(|stem| stem.to_string_lossy().parse::<u32>().ok());
            let root = path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .unwrap_or(path);
            return match number {
                Some(number) => format!("bd://mpls/{number}/{}", device_text(root)),
                None => format!("bd://longest/{}", device_text(root)),
            };
        }
        return format!("bd://longest/{}", device_text(path));
    }
    // An `.IFO` lies in `VIDEO_TS`; libdvdnav opens that folder's parent.
    let folder = if file {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    let folder = if folder
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("VIDEO_TS"))
    {
        folder.parent().unwrap_or(folder)
    } else {
        folder
    };
    format!("dvd://longest/{}", device_text(folder))
}

/// A device or folder as it goes into a disc address (forward slashes).
fn device_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A DVD title from the disc's title table (`VIDEO_TS.IFO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DvdTitle {
    pub chapters: u16,
    pub angles: u8,
}

/// The titles of a DVD (a drive or folder holding `VIDEO_TS`): the title
/// search pointer table of `VIDEO_TS.IFO` (its sector at 0xC4; a count,
/// then 12 bytes a title: angles at 1, chapters at 2). libmpv 0.37 and
/// later give no title list.
#[must_use]
pub fn dvd_titles(device: &Path) -> Vec<DvdTitle> {
    let ifo = device.join("VIDEO_TS").join("VIDEO_TS.IFO");
    let Ok(bytes) = std::fs::read(&ifo) else {
        return Vec::new();
    };
    let read_u16 = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
    };
    let read_u32 = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    if !bytes.starts_with(b"DVDVIDEO-VMG") {
        return Vec::new();
    }
    let Some(base) = read_u32(0xC4).and_then(|sector| usize::try_from(sector).ok()) else {
        return Vec::new();
    };
    let base = base * 2048;
    let count = read_u16(base).unwrap_or(0);
    (0..usize::from(count))
        .map_while(|index| {
            let entry = base + 8 + 12 * index;
            Some(DvdTitle {
                angles: *bytes.get(entry + 1)?,
                chapters: read_u16(entry + 2)?,
            })
        })
        .collect()
}

/// The playlists (`BDMV/PLAYLIST/NNNNN.mpls`) of a Blu-ray, by number.
#[must_use]
pub fn bluray_playlists(device: &Path) -> Vec<u32> {
    let mut numbers: Vec<u32> = std::fs::read_dir(device.join("BDMV").join("PLAYLIST"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let path = entry.path();
                    path.extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("mpls"))
                        .then(|| path.file_stem()?.to_string_lossy().parse().ok())
                        .flatten()
                })
                .collect()
        })
        .unwrap_or_default();
    numbers.sort_unstable();
    numbers
}

/// A disc address's kind (`true` for Blu-ray) and device: `dvd://T/DEV`,
/// `bd://T/DEV`, `bd://mpls/N/DEV`.
#[must_use]
pub fn disc_device(address: &str) -> Option<(bool, PathBuf)> {
    let (bluray, rest) = if let Some(rest) = address.strip_prefix("dvd://") {
        (false, rest)
    } else {
        (true, address.strip_prefix("bd://")?)
    };
    let rest = rest.strip_prefix("mpls/").unwrap_or(rest);
    let (_, device) = rest.split_once('/')?;
    (!device.is_empty()).then(|| (bluray, PathBuf::from(device)))
}

/// The address of the screen capture.
#[must_use]
pub fn screen_address(devices: &Devices) -> String {
    match devices.capture {
        Capture::Gdigrab => "av://gdigrab:desktop".to_owned(),
        Capture::Ddagrab => format!(
            "av://lavfi:ddagrab=framerate={}:draw_mouse={},hwdownload,format=bgra",
            devices.capture_fps,
            u8::from(devices.capture_pointer)
        ),
    }
}

/// The address of a capture device: `dshow` on Windows, `v4l2` elsewhere.
#[must_use]
pub fn device_address(name: &str) -> String {
    if cfg!(windows) {
        format!("av://dshow:video={name}")
    } else {
        format!("av://v4l2:{name}")
    }
}

/// Files that belong to a media file: its subtitles and other files of
/// the same name in its folder ("related files").
#[must_use]
pub fn related_files(path: &Path) -> Vec<PathBuf> {
    const RELATED: &[&str] = &[
        "srt", "ass", "ssa", "sub", "idx", "vtt", "smi", "sup", "txt", "pbf", "nfo",
    ];
    let (Some(folder), Some(stem)) = (path.parent(), path.file_stem()) else {
        return Vec::new();
    };
    let stem = stem.to_string_lossy().to_lowercase();
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|candidate| {
                    candidate != path
                        && candidate.is_file()
                        && candidate.extension().is_some_and(|extension| {
                            RELATED
                                .iter()
                                .any(|known| extension.eq_ignore_ascii_case(known))
                        })
                        && candidate.file_stem().is_some_and(|candidate_stem| {
                            let candidate_stem = candidate_stem.to_string_lossy().to_lowercase();
                            candidate_stem == stem
                                || candidate_stem.starts_with(&format!("{stem}."))
                        })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

impl App {
    /// The protocols libmpv opens, read once.
    pub(crate) fn protocol_available(&mut self, engine: &Engine, protocol: &str) -> bool {
        if self.play.protocols.is_empty() {
            self.play.protocols = engine.protocols();
        }
        self.play.protocols.iter().any(|known| known == protocol)
    }

    /// The Open button and F3: the Default Open Action's dialog.
    pub(crate) fn default_open(&self) -> UiRequest {
        match self.settings.open.default_open {
            DefaultOpen::Files => UiRequest::OpenFileDialog,
            DefaultOpen::Url => UiRequest::OpenUrlDialog,
            DefaultOpen::Folder => UiRequest::OpenFolderDialog,
        }
    }

    /// The Open menu's other sources.
    pub(crate) fn open_source(&mut self, engine: &Engine, source: Source) -> Option<UiRequest> {
        let devices = self.settings.devices.clone();
        match source {
            Source::Network => Some(UiRequest::OpenUrlDialog),
            Source::Clipboard => Some(UiRequest::ReadClipboard(Paste::Open)),
            Source::Screen => {
                if !self.protocol_available(engine, "av") {
                    return Some(self.cannot_open("Screen capture", "FFmpeg's devices"));
                }
                self.open_now(engine, &[screen_address(&devices)])
            }
            Source::Device => {
                if !devices.webcam.is_empty() {
                    return self.open_device(engine, &devices.webcam);
                }
                Some(UiRequest::DeviceDialog(Vec::new()))
            }
            Source::Dvd | Source::DvdFiles => {
                if !self.protocol_available(engine, "dvd") {
                    return Some(self.cannot_open("DVD", "dvdnav"));
                }
                if source == Source::Dvd && !devices.dvd.is_empty() {
                    let address = disc_address(Path::new(&devices.dvd), false);
                    return self.open_now(engine, &[address]);
                }
                Some(UiRequest::DiscDialog {
                    bluray: false,
                    files: source == Source::DvdFiles,
                })
            }
            Source::Bluray | Source::BlurayFiles => {
                if !self.protocol_available(engine, "bd") {
                    return Some(self.cannot_open("Blu-ray", "libbluray"));
                }
                if source == Source::Bluray && !devices.bluray.is_empty() {
                    let address = disc_address(Path::new(&devices.bluray), true);
                    return self.open_now(engine, &[address]);
                }
                Some(UiRequest::DiscDialog {
                    bluray: true,
                    files: source == Source::BlurayFiles,
                })
            }
            Source::AudioCd => {
                if !self.protocol_available(engine, "cdda") {
                    return Some(self.cannot_open("Audio CD", "libcdio"));
                }
                let address = if devices.cd.is_empty() {
                    "cdda://".to_owned()
                } else {
                    format!("cdda:///{}", device_text(Path::new(&devices.cd)))
                };
                self.open_now(engine, &[address])
            }
        }
    }

    fn cannot_open(&self, what: &str, missing: &str) -> UiRequest {
        let _ = self;
        UiRequest::Message(format!("{what}: this libmpv was built without {missing}"))
    }

    /// Opens addresses now (they join the shown album and the first plays).
    fn open_now(&mut self, engine: &Engine, addresses: &[String]) -> Option<UiRequest> {
        match self.open(engine, addresses, false) {
            Ok(()) => None,
            Err(error) => Some(UiRequest::Message(format!("Cannot open: {error}"))),
        }
    }

    /// The disc dialog's answer: a drive, a folder or a file of the disc.
    pub fn open_disc(&mut self, engine: &Engine, path: &Path, bluray: bool) -> Option<UiRequest> {
        let address = disc_address(path, bluray);
        self.open_now(engine, &[address])
    }

    /// The device dialog's answer (or the device setting).
    pub fn open_device(&mut self, engine: &Engine, name: &str) -> Option<UiRequest> {
        if !self.protocol_available(engine, "av") {
            return Some(self.cannot_open("Capture devices", "FFmpeg's devices"));
        }
        self.open_now(engine, &[device_address(name)])
    }

    /// Load External Audio's answer: the file joins the playing video as an
    /// audio track and plays.
    pub fn load_external_audio(&mut self, engine: &Engine, path: &Path) -> Option<UiRequest> {
        if !self.model.media_loaded() {
            return Some(UiRequest::Osd("Open a video first".to_owned()));
        }
        let text = path.to_string_lossy();
        Some(match engine.command("audio-add", &[&text, "select"]) {
            Ok(()) => UiRequest::Osd(format!("Audio: {}", crate::view::file_name(&text))),
            Err(error) => UiRequest::Message(format!("Cannot load {text}: {error}")),
        })
    }

    /// Reload Subtitle: the selected subtitle file is read again.
    pub(crate) fn reload_subtitle(&self, engine: &Engine) -> Option<UiRequest> {
        if !self.model.media_loaded() {
            return None;
        }
        Some(match engine.command("sub-reload", &[]) {
            Ok(()) => UiRequest::Osd("Subtitles reloaded".to_owned()),
            Err(_) => UiRequest::Osd("No subtitle file to reload".to_owned()),
        })
    }

    /// The clipboard's text, for `purpose`.
    pub fn paste_text(&mut self, engine: &Engine, purpose: Paste, text: &str) -> Option<UiRequest> {
        let items = clipboard_items(text);
        if items.is_empty() {
            return Some(UiRequest::Osd("Nothing to paste".to_owned()));
        }
        match purpose {
            Paste::Open => self.open_now(engine, &items),
            Paste::List => {
                let shown = self.library().lists().shown;
                self.library_mut().insert(shown, None, &items);
                None
            }
            Paste::ListBelow => {
                let shown = self.library().lists().shown;
                let at = self
                    .library()
                    .shown()
                    .selected
                    .iter()
                    .next_back()
                    .map(|index| index + 1);
                self.library_mut().insert(shown, at, &items);
                None
            }
        }
    }

    /// The files opening `files` adds, by "When Files Opened": a file
    /// opened alone brings its similar or all files; the first returned
    /// index is the one to play.
    pub(crate) fn expand_opened(&self, files: &[String]) -> (Vec<String>, usize) {
        let rule = self.settings.open.when_opened;
        let [file] = files else {
            return (files.to_vec(), 0);
        };
        if rule == WhenOpened::Selected || crate::playlist::is_url(file) {
            return (files.to_vec(), 0);
        }
        let opened = Path::new(file);
        let expanded: Vec<String> = files_for(opened, rule)
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        let first = expanded
            .iter()
            .position(|path| Path::new(path) == opened)
            .unwrap_or(0);
        (expanded, first)
    }

    /// The folder's next or previous media file after the playing entry
    /// ("Load previous/next files in play folder"), added to its album.
    pub(crate) fn folder_neighbour(&mut self, step: i32) -> Option<(usize, usize)> {
        if !self.settings.playback.folder_neighbours {
            return None;
        }
        let album = self.library().lists().playing?;
        let (index, entry) = self.library().playing()?;
        let path = PathBuf::from(&entry.path);
        let files = folder_media(path.parent()?);
        let at = files.iter().position(|file| *file == path)?;
        let target = if step > 0 {
            files.get(at + 1)?
        } else {
            files.get(at.checked_sub(1)?)?
        };
        let text = target.to_string_lossy().into_owned();
        let insert_at = if step > 0 { index + 1 } else { index };
        self.library_mut().insert(album, Some(insert_at), &[text])?;
        Some((album, insert_at))
    }

    /// Records an opened item as recent (when that is on).
    pub(crate) fn remember_recent(&mut self, path: &str) {
        if self.settings.open.recent {
            self.marks_mut().add_recent(path);
        }
    }

    /// Favorites: the playing item or its folder.
    pub(crate) fn add_favorite(&mut self, folder: bool) -> Option<UiRequest> {
        let path = self.model.loaded_path().map(str::to_owned).or_else(|| {
            self.library()
                .playing()
                .map(|(_, entry)| entry.path.clone())
        })?;
        let (target, folder) = if folder && !crate::playlist::is_url(&path) {
            let parent = Path::new(&path).parent()?.to_string_lossy().into_owned();
            (parent, true)
        } else {
            (path, false)
        };
        let added = self.marks_mut().add_favorite(&target, folder);
        Some(UiRequest::Osd(if added {
            format!("Added to favorites: {}", crate::view::file_name(&target))
        } else {
            "Already a favorite".to_owned()
        }))
    }

    /// Opens a favorite or a recent item (the Album/Favorites menu).
    pub fn open_item(&mut self, engine: &Engine, path: &str, folder: bool) -> Option<UiRequest> {
        if folder {
            return match self.open_folder(engine, Path::new(path)) {
                Ok(0) => Some(UiRequest::Osd("No media files there".to_owned())),
                Ok(_) => None,
                Err(error) => Some(UiRequest::Message(format!("Cannot open: {error}"))),
            };
        }
        self.open_now(engine, &[path.to_owned()])
    }

    /// The text "Copy current item's ..." copies.
    pub(crate) fn copy_name(&self, what: crate::action::CopyName) -> Option<UiRequest> {
        let path = self.model.loaded_path().map(str::to_owned).or_else(|| {
            self.library()
                .playing()
                .map(|(_, entry)| entry.path.clone())
        })?;
        let text = match what {
            crate::action::CopyName::Name => crate::view::file_name(&path).to_owned(),
            crate::action::CopyName::Folder => Path::new(&path)
                .parent()
                .map_or_else(String::new, |parent| parent.to_string_lossy().into_owned()),
            crate::action::CopyName::Path => path,
        };
        Some(UiRequest::CopyText(text))
    }

    /// The titles of the disc that plays: their labels and addresses
    /// (DVD Control > Move).
    #[must_use]
    pub fn disc_titles(&self) -> Vec<(String, String)> {
        let Some((bluray, device)) = self.model.loaded_path().and_then(disc_device) else {
            return Vec::new();
        };
        let device_text = device_text(&device);
        if bluray {
            return bluray_playlists(&device)
                .into_iter()
                .map(|number| {
                    (
                        format!("Playlist {number:05}"),
                        format!("bd://mpls/{number}/{device_text}"),
                    )
                })
                .collect();
        }
        dvd_titles(&device)
            .iter()
            .enumerate()
            .map(|(index, title)| {
                let chapters = if title.chapters == 1 {
                    "1 chapter".to_owned()
                } else {
                    format!("{} chapters", title.chapters)
                };
                // mpv counts DVD titles from 0 (`dvd://0/` is the first).
                (
                    format!("Title {} ({chapters})", index + 1),
                    format!("dvd://{index}/{device_text}"),
                )
            })
            .collect()
    }

    /// The angles of the DVD title that plays (DVD Control > Angle
    /// Option), 0 when it is not a DVD.
    #[must_use]
    pub fn disc_angles(&self) -> u8 {
        let Some(path) = self.model.loaded_path() else {
            return 0;
        };
        let Some((false, device)) = disc_device(path) else {
            return 0;
        };
        let titles = dvd_titles(&device);
        let title = path
            .strip_prefix("dvd://")
            .and_then(|rest| rest.split('/').next())
            .and_then(|title| title.parse::<usize>().ok());
        // "longest": the title with the most chapters stands for it.
        let chosen = match title {
            Some(number) => titles.get(number),
            None => titles.iter().max_by_key(|title| title.chapters),
        };
        chosen.map_or(0, |title| title.angles)
    }

    /// Plays a title of the disc (DVD Control > Move): the album's entry
    /// becomes that title.
    pub fn play_disc_title(&mut self, engine: &Engine, index: usize) -> Option<UiRequest> {
        let (_, address) = self.disc_titles().into_iter().nth(index)?;
        let old = self.model.loaded_path()?.to_owned();
        self.library_mut().replace_path(&old, &address);
        let (album, entry) = (self.library().lists().playing?, self.library().playing()?.0);
        match self.play_entry(engine, album, entry, false) {
            Ok(()) => None,
            Err(error) => Some(UiRequest::Message(format!("Cannot play: {error}"))),
        }
    }

    /// Shows a DVD angle (DVD Control > Angle Option): the title opens
    /// again at the same time with mpv's `dvd-angle`.
    pub fn set_disc_angle(&mut self, engine: &Engine, angle: u8) -> Option<UiRequest> {
        let time = self.model.double(PropertyId::TimePos).unwrap_or(0.0);
        engine.set_text("dvd-angle", &angle.to_string()).ok()?;
        let (album, entry) = (self.library().lists().playing?, self.library().playing()?.0);
        self.play.next_options = vec![("start".to_owned(), format!("{time:.3}"))];
        self.play.start_given = true;
        match self.play_entry(engine, album, entry, false) {
            Ok(()) => Some(UiRequest::Osd(format!("Angle {angle}"))),
            Err(error) => Some(UiRequest::Message(format!("Cannot play: {error}"))),
        }
    }

    /// DVD Control's speeds: forward at 1x to 8x, or backward
    /// (`play-direction=backward`); `false` when backward play is refused.
    /// mpv's backward play runs smoothly on files but stalls on a disc's
    /// stream (the time stands still: `tests/m11.rs`), so discs only play
    /// forward.
    pub(crate) fn dvd_speed(&mut self, engine: &Engine, speed: i8) -> Result<bool, EngineError> {
        if !self.model.media_loaded() {
            return Ok(true);
        }
        if speed < 0 && self.model.loaded_path().and_then(disc_device).is_some() {
            return Ok(false);
        }
        let direction = if speed < 0 { "backward" } else { "forward" };
        engine.set_text("play-direction", direction)?;
        engine.set_double("speed", f64::from(speed.unsigned_abs()))?;
        if self.model.flag(PropertyId::Pause) == Some(true) {
            engine.set_pause(false)?;
        }
        Ok(true)
    }

    /// Check marks of the Open menu's choices.
    pub(crate) fn source_checked(&self, action: Action) -> Option<bool> {
        let open = &self.settings.open;
        Some(match action {
            Action::SetDefaultOpen(choice) => open.default_open == choice,
            Action::SetWhenOpened(rule) => open.when_opened == rule,
            Action::ToggleRecent => open.recent,
            _ => return None,
        })
    }
}

/// mpv's list separator for path options (`sub-files`, `audio-files`).
fn path_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

impl App {
    /// The command line's switches before its files open: their order,
    /// `/same`, and the first file's options (start, subtitle, audio,
    /// title, HTTP headers).
    pub fn prepare_switches(&mut self, request: &mut crate::instance::Request) {
        let switches = &request.switches;
        if switches.sort {
            request
                .files
                .sort_by_key(|file| crate::view::file_name(file).to_lowercase());
        } else if switches.randomize {
            let mut state = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |elapsed| elapsed.as_secs() | 1);
            for i in (1..request.files.len()).rev() {
                let bound = u64::try_from(i + 1).unwrap_or(u64::MAX);
                let j = crate::playlist::next_random(&mut state) % bound;
                request.files.swap(i, usize::try_from(j).unwrap_or(0));
            }
        }
        let mut options = Vec::new();
        if switches.same && request.files.len() > 1 {
            let streams = request.files.split_off(1);
            options.push(("external-files".to_owned(), streams.join(path_separator())));
        }
        let mut start_given = false;
        if let Some((seconds, None)) = switches
            .seek
            .as_deref()
            .and_then(crate::playback::parse_time)
        {
            options.push(("start".to_owned(), format!("{seconds:.3}")));
            start_given = true;
        }
        for (switch, option) in [
            (&switches.sub, "sub-files"),
            (&switches.aud, "audio-files"),
            (&switches.title, "force-media-title"),
            (&switches.user_agent, "user-agent"),
            (&switches.referer, "referrer"),
            (&switches.headers, "http-header-fields"),
        ] {
            if let Some(value) = switch {
                options.push((option.to_owned(), value.clone()));
            }
        }
        if !request.files.is_empty() || switches.open.as_deref() == Some("autoplay") {
            self.play.next_options = options;
            self.play.start_given = start_given;
        }
    }

    /// The command line's switch that opens something at once (after its
    /// files): a dialog, a source or the last item.
    pub fn switch_action(
        &mut self,
        engine: &Engine,
        request: &crate::instance::Request,
    ) -> Option<UiRequest> {
        match request.switches.open.as_deref()? {
            "filedlg" => Some(UiRequest::OpenFileDialog),
            "urldlg" => Some(UiRequest::OpenUrlDialog),
            "folderdlg" => Some(UiRequest::OpenFolderDialog),
            "cap" => self.open_source(engine, Source::Screen),
            "cam" => self.open_source(engine, Source::Device),
            "dvd" => self.open_source(engine, Source::Dvd),
            "cd" => {
                if let Some(drive) = &request.switches.drive {
                    self.settings.devices.cd = format!("{drive}:");
                }
                self.open_source(engine, Source::AudioCd)
            }
            "autoplay" => self.dispatch(engine, Action::Reopen),
            "clipboard" => Some(UiRequest::ReadClipboard(Paste::Open)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rvp-sources-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        for file in files {
            std::fs::write(dir.join(file), b"x").expect("file");
        }
        dir
    }

    #[test]
    fn similar_files_share_the_name_without_numbers() {
        let dir = folder(
            "similar",
            &[
                "Show S01E02.mkv",
                "Show S01E01.mkv",
                "Show S01E10.mkv",
                "Other 1.mkv",
                "Show S01E01.srt",
                "song 1.mp3",
            ],
        );
        let opened = dir.join("Show S01E02.mkv");
        let names = |files: Vec<PathBuf>| -> Vec<String> {
            files
                .iter()
                .map(|file| {
                    file.file_name()
                        .expect("name")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        assert_eq!(
            names(files_for(&opened, WhenOpened::Similar)),
            ["Show S01E01.mkv", "Show S01E02.mkv", "Show S01E10.mkv"]
        );
        assert_eq!(
            names(files_for(&opened, WhenOpened::All)),
            [
                "Other 1.mkv",
                "Show S01E01.mkv",
                "Show S01E02.mkv",
                "Show S01E10.mkv",
                "song 1.mp3"
            ]
        );
        assert_eq!(
            names(files_for(&opened, WhenOpened::Selected)),
            ["Show S01E02.mkv"]
        );
        let related = related_files(&dir.join("Show S01E01.mkv"));
        assert_eq!(names(related), ["Show S01E01.srt"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disc_addresses_name_their_device() {
        let dvd = folder("dvd", &[]);
        std::fs::create_dir_all(dvd.join("VIDEO_TS")).expect("video_ts");
        std::fs::write(dvd.join("VIDEO_TS/VIDEO_TS.IFO"), b"x").expect("ifo");
        let root = device_text(&dvd);
        assert_eq!(disc_address(&dvd, false), format!("dvd://longest/{root}"));
        assert_eq!(
            disc_address(&dvd.join("VIDEO_TS"), false),
            format!("dvd://longest/{root}")
        );
        assert_eq!(
            disc_address(&dvd.join("VIDEO_TS/VIDEO_TS.IFO"), false),
            format!("dvd://longest/{root}")
        );
        let bd = Path::new("/discs/film");
        assert_eq!(disc_address(bd, true), "bd://longest//discs/film");
        assert_eq!(
            disc_address(&bd.join("BDMV/PLAYLIST/00012.mpls"), true),
            "bd://mpls/12//discs/film"
        );
        let _ = std::fs::remove_dir_all(&dvd);
    }

    #[test]
    fn the_clipboard_gives_paths_and_addresses() {
        let text =
            "\"/films/a b.mkv\"\nfile:///films/c%20d.mkv\n\nhttps://example.com/v\nnot a path\n";
        let items = clipboard_items(text);
        let expected_c = if cfg!(windows) {
            "films/c d.mkv"
        } else {
            "/films/c d.mkv"
        };
        assert_eq!(items[0], "/films/a b.mkv");
        assert_eq!(items[1], expected_c);
        assert_eq!(items[2], "https://example.com/v");
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn dvd_titles_come_from_the_title_table() {
        let dvd = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/media/dvd"
        ));
        assert_eq!(
            dvd_titles(&dvd),
            [
                DvdTitle {
                    chapters: 2,
                    angles: 1
                },
                DvdTitle {
                    chapters: 1,
                    angles: 1
                }
            ]
        );
        assert!(dvd_titles(Path::new("/no/such/disc")).is_empty());
        assert_eq!(
            disc_device("dvd://longest//discs/a"),
            Some((false, PathBuf::from("/discs/a")))
        );
        assert_eq!(
            disc_device("bd://mpls/12/D:/"),
            Some((true, PathBuf::from("D:/")))
        );
        assert_eq!(disc_device("/films/a.mkv"), None);
    }

    #[test]
    fn capture_addresses() {
        let mut devices = Devices::default();
        assert_eq!(screen_address(&devices), "av://gdigrab:desktop");
        devices.capture = Capture::Ddagrab;
        assert_eq!(
            screen_address(&devices),
            "av://lavfi:ddagrab=framerate=30:draw_mouse=1,hwdownload,format=bgra"
        );
        let camera = device_address("USB Camera");
        assert!(camera.ends_with("USB Camera"), "{camera}");
    }
}
