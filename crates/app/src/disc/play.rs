//! A DVD or a Blu-ray playing with its menus (M16): the album's entry
//! stays the disc (`dvd://longest/DEVICE`, `bd://longest/DEVICE`), and the
//! shell plays it as segments (`rvpdisc://N`) its navigator hands mpv. The app asks the shell
//! to start (`take_request`), leaves the segments' ends to it, and moves
//! on in the album when the disc's program ends (`disc_finished`).

use std::time::Instant;

use rvp_engine::Engine;

use crate::app::{App, UiRequest};

/// The segments' scheme.
pub const SEGMENT_SCHEME: &str = "rvpdisc://";

/// Whether mpv plays one of the navigator's segments.
#[must_use]
pub fn is_segment(path: &str) -> bool {
    path.starts_with(SEGMENT_SCHEME)
}

/// DVD Control's menus (PotPlayer's Previous, Root, Title, Chapter, Audio,
/// Subtitle and Angle Menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscMenu {
    Previous,
    Root,
    Title,
    Chapter,
    Audio,
    Subtitle,
    Angle,
}

/// Which navigator plays a disc's menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscKind {
    /// libdvdnav.
    Dvd,
    /// libbluray.
    Bluray,
}

/// The disc whose menus play: its album entry and device, and what its
/// machine starts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscPlay {
    pub kind: DiscKind,
    pub entry: String,
    pub device: String,
    /// The languages the machine prefers for the menus, the audio and the
    /// subtitles (two letters; empty: the disc's default).
    pub languages: [String; 3],
    /// Straight to the disc's menu, past its opening.
    pub skip_opening: bool,
}

/// A two-letter language code (ISO 639-1, as DVDs name languages).
#[must_use]
pub fn language_code(text: &str) -> Option<String> {
    let code = text.trim();
    (code.len() == 2 && code.chars().all(|c| c.is_ascii_alphabetic()))
        .then(|| code.to_ascii_lowercase())
}

/// The first of a list of language codes (`en,ru`) that has two letters.
fn first_code(list: &str) -> String {
    list.split(',').find_map(language_code).unwrap_or_default()
}

/// The whole-disc addresses `play_entry` hands to the menus:
/// `dvd://longest/` and `bd://longest/` with their device (titles and
/// playlists chosen in DVD Control > Move play directly).
#[must_use]
pub fn menu_device(path: &str) -> Option<(DiscKind, &str)> {
    let (kind, device) = if let Some(device) = path.strip_prefix("dvd://longest/") {
        (DiscKind::Dvd, device)
    } else {
        (DiscKind::Bluray, path.strip_prefix("bd://longest/")?)
    };
    (!device.is_empty()).then_some((kind, device))
}

impl App {
    /// The shell found libdvdnav and libbluray (or not) at start.
    pub fn set_disc_menus_available(&mut self, dvd: bool, bluray: bool) {
        self.disc_menus_available = (dvd, bluray);
    }

    /// Whether `path` plays through the menus.
    pub(crate) fn plays_disc_menus(&self, path: &str) -> bool {
        let (dvd, bluray) = self.disc_menus_available;
        match menu_device(path) {
            Some((DiscKind::Dvd, _)) => dvd && self.settings.devices.dvd_menus,
            Some((DiscKind::Bluray, _)) => bluray && self.settings.devices.bluray_menus,
            None => false,
        }
    }

    /// A disc to start with its menus, once.
    pub fn take_disc_request(&mut self) -> Option<DiscPlay> {
        self.disc_request.take()
    }

    /// The disc that plays with its menus.
    #[must_use]
    pub fn disc_play(&self) -> Option<&DiscPlay> {
        self.disc_playing.as_ref()
    }

    /// Asks the shell to play `path`'s disc with its menus.
    pub(crate) fn request_disc_menus(&mut self, path: &str) {
        let Some((kind, device)) = menu_device(path) else {
            return;
        };
        let devices = &self.settings.devices;
        let play = DiscPlay {
            kind,
            entry: path.to_owned(),
            device: device.to_owned(),
            languages: [
                devices.dvd_menu_language.clone(),
                first_code(&self.settings.audio.languages),
                first_code(&self.settings.subtitle_languages),
            ],
            skip_opening: devices.dvd_skip_opening,
        };
        self.store_mut().set_last_file(path);
        self.disc_request = Some(play.clone());
        self.disc_playing = Some(play);
    }

    /// Another file plays: the disc's menus are over.
    pub(crate) fn leave_disc(&mut self, path: &str) {
        if !is_segment(path) {
            self.disc_playing = None;
        }
    }

    /// The disc's program ended: the album moves on as at a file's end.
    pub fn disc_finished(&mut self, engine: &Engine, now: Instant) -> Vec<UiRequest> {
        self.disc_playing = None;
        let mut requests = Vec::new();
        self.end_handled = true;
        self.end_of_file(engine, now, &mut requests);
        requests
    }
}
