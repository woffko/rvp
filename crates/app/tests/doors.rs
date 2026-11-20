//! Text from outside becomes an album entry only when it names a file or
//! a network address (`rvp_app::playlist::admitted`): a playlist file and
//! PotPlayer's playlists, the playlist's address dialog, the clipboard,
//! and what a dialog, a drop or another program asks to open. The rest is
//! left out with one notice, while RVP's own addresses (a disc, a capture
//! device) open and stay in the album as before. Headless: no window, no
//! sound.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rvp_app::playlist::local_path;
use rvp_app::sources::Paste;
use rvp_app::{App, Settings, UiRequest};
use rvp_engine::{Engine, EngineOptions};

const SHORT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-3s.mkv"
);
const LONG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../testdata/media/fixture-60s.mkv"
);

/// Addresses no door lets in: a capture device and a filter graph (in any
/// case), descriptors, a disc, mpv's containers of addresses, FFmpeg's
/// own protocols, another host's file.
const REFUSED: &[&str] = &[
    "av://v4l2:/dev/video0",
    "AV://lavfi:testsrc",
    "fd://0",
    "fdclose://9",
    "dvd://1",
    "edl://a.mkv",
    "memory://x",
    "lavf://file:/films/a.mkv",
    "file://example.org/share/a.mkv",
];

struct Harness {
    app: App,
    engine: Engine,
    dir: PathBuf,
}

impl Harness {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("rvp-doors-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let settings = Settings {
            reopen_last_file: false,
            remember_positions: false,
            ipc_server: String::new(),
            ..Settings::default()
        };
        let mut app = App::new(settings, &dir.join("state")).expect("app");
        app.skip_first_import();
        Self {
            app,
            engine: Engine::new(&EngineOptions::headless()).expect("engine"),
            dir,
        }
    }

    fn paths(&self) -> Vec<String> {
        self.app
            .library()
            .shown()
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect()
    }

    /// Nothing loads within a moment.
    fn stays_idle(&mut self) {
        std::thread::sleep(Duration::from_millis(200));
        self.app.drain(&self.engine, Instant::now());
        assert_eq!(self.app.model.loaded_path(), None);
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The notice for `count` addresses left out.
fn left_out(request: Option<&UiRequest>, count: usize) {
    let expected = if count == 1 {
        "Not opened: 1 address (".to_owned()
    } else {
        format!("Not opened: {count} addresses (")
    };
    assert!(
        matches!(request, Some(UiRequest::Message(text)) if text.starts_with(&expected)),
        "{request:?}"
    );
}

/// The refused addresses as a list's numbered lines.
fn numbered(line: impl Fn(usize, &str) -> String) -> String {
    let mut text = String::new();
    for (index, address) in REFUSED.iter().enumerate() {
        text.push_str(&line(index + 1, address));
    }
    text
}

/// A file URL of a path.
fn url(path: &str) -> String {
    rvp_app::media::file_url(&local_path(path))
}

#[test]
fn a_playlist_file_adds_files_and_network_addresses() {
    let mut h = Harness::new("list");
    let refused = REFUSED.join("\n");
    let lists = [
        (
            "list.m3u8",
            format!(
                "#EXTM3U\n#EXTINF:3,Camera\n{refused}\n#EXTINF:3,Short\n{SHORT}\n{}\n\
                 https://example.org/live.m3u8\n",
                url(LONG)
            ),
        ),
        (
            "list.pls",
            format!(
                "[playlist]\n{}File20={SHORT}\nTitle20=Short\nFile21={}\n\
                 File22=https://example.org/live.m3u8\n",
                numbered(|number, address| format!("File{number}={address}\n")),
                url(LONG)
            ),
        ),
        (
            "list.dpl",
            format!(
                "\u{feff}DAUMPLAYLIST\r\n{}20*file*{SHORT}\r\n20*title*Short\r\n21*file*{}\r\n\
                 22*file*https://example.org/live.m3u8\r\n",
                numbered(|number, address| format!("{number}*file*{address}\r\n")),
                url(LONG)
            ),
        ),
    ];
    let expected = [
        local_path(SHORT),
        local_path(LONG),
        "https://example.org/live.m3u8".to_owned(),
    ];
    for (name, text) in lists {
        let list = h.dir.join(name);
        std::fs::write(&list, text).expect("list");
        let before = h.paths().len();
        let answer = h
            .app
            .list_dialog_answer(&h.engine, "open", &list.to_string_lossy());
        left_out(answer.as_ref(), REFUSED.len());
        let paths = h.paths();
        assert_eq!(paths[before..], expected, "{name}");
        assert_eq!(
            h.app.library().shown().entries[before].title.as_deref(),
            Some("Short"),
            "{name}"
        );
    }
    // A list of nothing else adds nothing, and says so.
    let only = h.dir.join("only.m3u");
    std::fs::write(&only, REFUSED.join("\n")).expect("list");
    let before = h.paths();
    let answer = h
        .app
        .list_dialog_answer(&h.engine, "open", &only.to_string_lossy());
    left_out(answer.as_ref(), REFUSED.len());
    assert_eq!(h.paths(), before);
    h.stays_idle();
}

#[test]
fn potplayers_playlists_import_files_and_network_addresses() {
    let mut h = Harness::new("import");
    let playlists = h.dir.join("Playlist");
    std::fs::create_dir_all(&playlists).expect("dir");
    let body = numbered(|number, address| format!("{number}*file*{address}\r\n"));
    std::fs::write(
        playlists.join("Mixed.dpl"),
        format!(
            "\u{feff}DAUMPLAYLIST\r\n{body}30*file*{}\r\n31*file*mms://example.org/radio\r\n",
            local_path(SHORT)
        ),
    )
    .expect("dpl");
    assert_eq!(h.app.import_playlists(&playlists).expect("import"), 1);
    let album = h
        .app
        .library()
        .lists()
        .albums
        .iter()
        .find(|album| album.name == "Mixed")
        .expect("the album");
    let paths: Vec<&str> = album
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [local_path(SHORT).as_str(), "mms://example.org/radio"]
    );
    left_out(h.app.refusal().as_ref(), REFUSED.len());
    assert_eq!(h.app.refusal(), None);
}

#[test]
fn typed_and_pasted_addresses_are_files_and_network_addresses() {
    let mut h = Harness::new("typed");
    // The playlist's address dialog: one address a line.
    let text = format!(
        "{}\n{SHORT}\n  rtsp://example.org/live  \n",
        REFUSED.join("\n")
    );
    let answer = h.app.list_dialog_answer(&h.engine, "url", &text);
    left_out(answer.as_ref(), REFUSED.len());
    assert_eq!(
        h.paths(),
        [local_path(SHORT), "rtsp://example.org/live".to_owned()]
    );

    // The clipboard into the list, at its end and below the selection.
    let text = format!(
        "{}\n{}\nhttps://example.org/v\n",
        REFUSED.join("\n"),
        url(LONG)
    );
    for purpose in [Paste::List, Paste::ListBelow] {
        let before = h.paths().len();
        let answer = h.app.paste_text(&h.engine, purpose, &text);
        left_out(answer.as_ref(), REFUSED.len());
        assert_eq!(h.paths().len(), before + 2, "{purpose:?}");
        assert!(h.paths().contains(&local_path(LONG)), "{purpose:?}");
    }
    assert!(
        h.paths()
            .iter()
            .all(|path| !REFUSED.contains(&path.as_str())),
        "{:?}",
        h.paths()
    );
    // A clipboard of nothing else pastes nothing.
    let before = h.paths();
    for purpose in [Paste::Open, Paste::List] {
        let answer = h.app.paste_text(&h.engine, purpose, &REFUSED.join("\n"));
        left_out(answer.as_ref(), REFUSED.len());
        assert_eq!(h.paths(), before, "{purpose:?}");
    }
    assert_eq!(
        h.app
            .paste_text(&h.engine, Paste::List, "no address at all"),
        Some(UiRequest::Osd("Nothing to paste".to_owned()))
    );
    h.stays_idle();
}

#[test]
fn a_dialog_a_drop_or_a_program_opens_files_and_network_addresses() {
    let mut h = Harness::new("open");
    // What the file and address dialogs, a drop and another program ask
    // to open (the shell's `open_files`).
    let refused: Vec<String> = REFUSED.iter().map(|&address| address.to_owned()).collect();
    for append in [true, false] {
        let answer = h.app.open_outside(&h.engine, &refused, append);
        left_out(answer.as_ref(), REFUSED.len());
        assert!(h.paths().is_empty(), "append {append}");
    }
    let answer = h
        .app
        .open_outside(&h.engine, &["av://lavfi:testsrc".to_owned()], false);
    left_out(answer.as_ref(), 1);
    h.stays_idle();

    let mut mixed = refused.clone();
    mixed.insert(2, url(SHORT));
    mixed.push("ftp://example.org/films/a.mkv".to_owned());
    let answer = h.app.open_outside(&h.engine, &mixed, true);
    left_out(answer.as_ref(), REFUSED.len());
    assert_eq!(
        h.paths(),
        [
            local_path(SHORT),
            "ftp://example.org/films/a.mkv".to_owned()
        ]
    );

    // RVP's own addresses open as before, and the album keeps them: a
    // DVD's title of the fixture disc.
    let dvd = Path::new(SHORT).parent().expect("folder").join("dvd");
    if dvd.join("VIDEO_TS").is_dir() {
        let answer = h.app.open_disc(&h.engine, &dvd, false);
        // No notice, unless this libmpv has no DVD support.
        if answer.is_none() {
            assert!(
                h.paths()
                    .last()
                    .is_some_and(|path| path.starts_with("dvd://")),
                "{:?}",
                h.paths()
            );
        }
    }
}
