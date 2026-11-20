//! Several RVPs on one settings and state folder ("Multiple instances",
//! `rvp /new`): each writes only its own changes onto the newest files on
//! disk (the rules of `crates/app/src/storage.rs`), so the positions,
//! marks, settings and albums of both survive whichever quits last, the
//! same album changed in both is the last save's, and what one removed the
//! other does not bring back.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rvp_app::marks::MarkStore;
use rvp_app::playlist::Library;
use rvp_app::preferences::change;
use rvp_app::{App, Dirs, ResumeStore};
use rvp_engine::{Engine, EngineOptions};
use serde_json::{Value, json};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-shared-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One setting changed as the settings window changes it.
fn set(app: &mut App, engine: &Engine, path: &str, value: Value) {
    let (settings, notes) = change(&app.settings, path, value).expect(path);
    assert!(notes.is_empty(), "{path}: {notes:?}");
    app.apply_settings(engine, settings);
}

/// Imports a PotPlayer playlist `name` of one file with a position: a new
/// album and a position in the resume store. Returns the file as RVP
/// keeps it.
fn import(app: &mut App, dir: &Path, name: &str, file: &str, start_ms: u32) -> String {
    let folder = dir.join(format!("import-{name}"));
    std::fs::create_dir_all(&folder).expect("dir");
    std::fs::write(
        folder.join(format!("{name}.dpl")),
        format!("\u{feff}DAUMPLAYLIST\r\n1*file*{file}\r\n1*start*{start_ms}\r\n"),
    )
    .expect("write");
    assert_eq!(app.import_playlists(&folder).expect("import"), 1);
    let lists = app.library().lists();
    let album = lists
        .albums
        .iter()
        .find(|album| album.name == name)
        .expect("the album");
    album.entries[0].path.clone()
}

fn album_index(app: &App, name: &str) -> usize {
    app.library()
        .lists()
        .albums
        .iter()
        .position(|album| album.name == name)
        .unwrap_or_else(|| panic!("no album {name}"))
}

/// The entries of the album `name`.
fn entries(app: &App, name: &str) -> Vec<String> {
    app.library().lists().albums[album_index(app, name)]
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect()
}

fn names(app: &App) -> Vec<String> {
    let mut names: Vec<String> = app
        .library()
        .lists()
        .albums
        .iter()
        .map(|album| album.name.clone())
        .collect();
    names.sort();
    names
}

fn path(name: &str) -> String {
    rvp_app::playlist::local_path(&format!("/m/{name}.mkv"))
}

/// What an earlier run left: "Default" with one entry, the albums "Films"
/// and "Gone", a bookmark and a setting.
fn earlier_run(dirs: &Dirs, engine: &Engine) {
    let mut app = App::load(dirs).expect("earlier run");
    let library = app.library_mut();
    library.insert(0, None, &[path("old")]);
    let films = library.add_album("Films");
    library.insert(films, None, &[path("film")]);
    library.add_album("Gone");
    app.marks_mut().add_bookmark(&path("old"), 5.0, "Old");
    app.marks_mut().add_bookmark(&path("gone"), 6.0, "Gone");
    set(&mut app, engine, "volume-max", json!(150));
    app.shutdown(engine, Instant::now())
        .expect("earlier run quits");
}

#[test]
fn two_players_keep_each_others_saves() {
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    for one_quits_first in [true, false] {
        let dir = scratch(if one_quits_first {
            "one-first"
        } else {
            "two-first"
        });
        let dirs = Dirs::inside(&dir);
        earlier_run(&dirs, &engine);
        let mut one = App::load(&dirs).expect("one");
        let mut two = App::load(&dirs).expect("two");
        // Each player its own setting, album, position and bookmark.
        set(&mut one, &engine, "volume-step", json!(7));
        set(&mut two, &engine, "seek-seconds.0", json!(10));
        let one_file = import(&mut one, &dir, "One", r"V:\F\one.mkv", 30_000);
        let two_file = import(&mut two, &dir, "Two", r"V:\F\two.mkv", 40_000);
        one.marks_mut().add_bookmark(&path("one"), 10.0, "One");
        two.marks_mut().add_bookmark(&path("two"), 20.0, "Two");
        let (first, second) = if one_quits_first {
            (&mut one, &mut two)
        } else {
            (&mut two, &mut one)
        };
        first
            .shutdown(&engine, Instant::now())
            .expect("first quits");
        second
            .shutdown(&engine, Instant::now())
            .expect("second quits");

        let next = App::load(&dirs).expect("next start");
        let settings = &next.settings;
        assert!((settings.volume_step - 7.0).abs() < 1e-9, "one's setting");
        assert!((settings.seek_seconds[0] - 10.0).abs() < 1e-9, "two's");
        assert!(
            (settings.volume_max - 150.0).abs() < 1e-9,
            "the earlier one"
        );
        assert_eq!(names(&next), ["Default", "Films", "Gone", "One", "Two"]);
        assert_eq!(entries(&next, "Default"), [path("old")]);
        assert_eq!(entries(&next, "Films"), [path("film")]);
        assert_eq!(entries(&next, "One"), std::slice::from_ref(&one_file));
        assert_eq!(entries(&next, "Two"), std::slice::from_ref(&two_file));
        assert_eq!(next.store().position(&one_file), Some(30.0));
        assert_eq!(next.store().position(&two_file), Some(40.0));
        for (file, title) in [("old", "Old"), ("one", "One"), ("two", "Two")] {
            let bookmarks = next.marks().file(&path(file)).bookmarks;
            assert_eq!(bookmarks.len(), 1, "{file}");
            assert_eq!(bookmarks[0].title, title);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The same album changed in both players: the last save's; the albums
/// only one of them changed keep that one's changes.
#[test]
fn the_last_save_wins_for_an_album_changed_in_both() {
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let dir = scratch("same-album");
    let dirs = Dirs::inside(&dir);
    earlier_run(&dirs, &engine);
    let mut one = App::load(&dirs).expect("one");
    let mut two = App::load(&dirs).expect("two");
    one.library_mut().insert(0, None, &[path("one")]);
    let films = album_index(&one, "Films");
    one.library_mut().insert(films, None, &[path("sequel")]);
    two.library_mut().insert(0, None, &[path("two")]);
    two.library_mut().add_album("Series");
    // Selection, scrolling and playback marks in an album the other edited
    // give way to the edit.
    let films = album_index(&two, "Films");
    two.library_mut().set_playing(films, 0);
    two.library_mut()
        .select(films, 0, rvp_app::playlist::Select::Only);
    one.shutdown(&engine, Instant::now()).expect("one quits");
    two.shutdown(&engine, Instant::now()).expect("two quits");

    let next = App::load(&dirs).expect("next start");
    assert_eq!(entries(&next, "Default"), [path("old"), path("two")]);
    assert_eq!(entries(&next, "Films"), [path("film"), path("sequel")]);
    assert_eq!(names(&next), ["Default", "Films", "Gone", "Series"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// What one player removed (an album, a bookmark, a position) stays
/// removed when the other, which still had it, saves later.
#[test]
fn a_removal_is_not_undone_by_the_other_player() {
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let dir = scratch("removal");
    let dirs = Dirs::inside(&dir);
    earlier_run(&dirs, &engine);
    let mut one = App::load(&dirs).expect("one");
    let mut two = App::load(&dirs).expect("two");
    let gone = album_index(&one, "Gone");
    assert!(one.library_mut().remove_album(gone));
    assert!(one.marks_mut().remove_bookmark(&path("gone"), 0));
    one.shutdown(&engine, Instant::now()).expect("one quits");
    two.library_mut().add_album("Two");
    two.marks_mut().add_bookmark(&path("two"), 1.0, "Two");
    two.shutdown(&engine, Instant::now()).expect("two quits");
    let next = App::load(&dirs).expect("next start");
    assert_eq!(names(&next), ["Default", "Films", "Two"]);
    assert!(next.marks().file(&path("gone")).bookmarks.is_empty());
    assert_eq!(next.marks().file(&path("two")).bookmarks.len(), 1);

    // Positions, in the resume store of each player.
    let state = dir.join("state");
    {
        let mut earlier = ResumeStore::open(&state).expect("earlier");
        earlier.set_position("gone", 100.0, None);
        earlier.set_position("kept", 200.0, None);
        earlier.save_and_wait(Instant::now()).expect("saved");
    }
    let mut one = ResumeStore::open(&state).expect("one");
    let mut two = ResumeStore::open(&state).expect("two");
    one.clear_position("gone");
    one.save_and_wait(Instant::now()).expect("one saves");
    two.set_position("other", 50.0, None);
    two.set_volume(30.0);
    two.save_and_wait(Instant::now()).expect("two saves");
    // The same position changed in both: the last save's.
    one.set_position("kept", 210.0, None);
    one.save_and_wait(Instant::now()).expect("one saves");
    let saved = ResumeStore::open(&state).expect("reopen");
    assert_eq!(saved.position("gone"), None);
    assert_eq!(saved.position("kept"), Some(210.0));
    assert_eq!(saved.position("other"), Some(50.0));
    assert_eq!(saved.state().volume, Some(30.0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Presets of two players: the list merges by name, each preset's values
/// and the default's by setting.
#[test]
fn presets_of_two_players_merge_by_name() {
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let dir = scratch("presets");
    let dirs = Dirs::inside(&dir);
    let mut one = App::load(&dirs).expect("one");
    let mut two = App::load(&dirs).expect("two");
    let default_step = one.settings.volume_step;
    set(
        &mut one,
        &engine,
        "presets.list",
        json!([{ "name": "Night", "conditions": {}, "values": null }]),
    );
    one.activate_preset(&engine, "Night", true);
    set(&mut one, &engine, "volume-step", json!(7));
    set(
        &mut two,
        &engine,
        "presets.list",
        json!([{ "name": "Day", "conditions": {}, "values": null }]),
    );
    set(&mut two, &engine, "seek-seconds.0", json!(10));
    one.shutdown(&engine, Instant::now()).expect("one quits");
    two.shutdown(&engine, Instant::now()).expect("two quits");

    let mut next = App::load(&dirs).expect("next start");
    let mut presets: Vec<&str> = next
        .settings
        .presets
        .list
        .iter()
        .map(|preset| preset.name.as_str())
        .collect();
    presets.sort_unstable();
    assert_eq!(presets, ["Day", "Night"]);
    // One's preset runs with its value; the default has two's.
    assert_eq!(next.settings.presets.active, "Night");
    assert!((next.settings.volume_step - 7.0).abs() < 1e-9);
    next.activate_preset(&engine, "", true);
    assert!((next.settings.volume_step - default_step).abs() < 1e-9);
    assert!((next.settings.seek_seconds[0] - 10.0).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file damaged on disk when a player writes: that player's whole copy
/// is written, as one player alone would write it, and the other player's
/// later saves go onto it.
#[test]
fn a_damaged_file_takes_the_whole_copy() {
    let dir = scratch("damaged");
    let mut one = Library::open(&dir).expect("one");
    let mut two = Library::open(&dir).expect("two");
    two.add_album("Two");
    two.flush().expect("two saves");
    std::fs::write(dir.join("playlists.json"), "{ damaged").expect("damage");
    one.insert(0, None, &[path("one")]);
    one.flush().expect("one saves");
    assert_eq!(Library::open(&dir).expect("reopen").lists(), one.lists());
    // What only the damaged file held is gone; the rest merges again.
    two.add_album("Later");
    two.flush().expect("two saves");
    let saved = Library::open(&dir).expect("reopen");
    let albums: Vec<(&str, usize)> = saved
        .lists()
        .albums
        .iter()
        .map(|album| (album.name.as_str(), album.entries.len()))
        .collect();
    assert_eq!(albums, [("Default", 1), ("Later", 0)]);

    let mut one = MarkStore::open(&dir).expect("one");
    let mut two = MarkStore::open(&dir).expect("two");
    two.add_favorite(&path("two"), false);
    two.flush().expect("two saves");
    std::fs::write(dir.join("marks.json"), "not json").expect("damage");
    one.add_recent(&path("one"));
    one.flush().expect("one saves");
    assert_eq!(MarkStore::open(&dir).expect("reopen").marks(), one.marks());
    two.add_bookmark(&path("two"), 1.0, "Two");
    two.flush().expect("two saves");
    let saved = MarkStore::open(&dir).expect("reopen");
    assert_eq!(saved.marks().recent, [path("one")]);
    assert_eq!(saved.file(&path("two")).bookmarks.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
