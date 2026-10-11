//! Importing PotPlayer playlists into RVP's albums (goal F) on synthetic
//! `.dpl` files: albums come after the existing ones with unique names,
//! resume positions go to the resume store, drive letters are mapped.

use rvp_app::{App, Settings};

fn write(dir: &std::path::Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), format!("\u{feff}DAUMPLAYLIST\r\n{body}")).expect("write");
}

#[test]
fn imports_albums_with_positions() {
    let root = std::env::temp_dir().join(format!("rvp-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let playlists = root.join("Playlist");
    std::fs::create_dir_all(&playlists).expect("dir");
    write(
        &playlists,
        "PotPlayerMini64.dpl",
        "playname=V:\\F\\b.mkv\r\nplaytime=90000\r\n1*file*V:\\F\\a.mkv\r\n1*played*1\r\n\
         2*file*V:\\F\\b.mkv\r\n2*duration2*600000\r\n",
    );
    write(
        &playlists,
        "Series.dpl",
        "1*file*V:\\S\\e1.mkv\r\n1*start*30000\r\n",
    );

    let mut app = App::new(Settings::default(), &root.join("state")).expect("app");
    assert_eq!(app.import_playlists(&playlists).expect("import"), 2);
    let lists = app.library().lists();
    let names: Vec<&str> = lists.albums.iter().map(|a| a.name.as_str()).collect();
    // RVP's own empty "Default" takes PotPlayer's.
    assert_eq!(names, ["Default", "Series"]);
    let imported = &lists.albums[0];
    assert_eq!(imported.current, Some(1));
    assert!(imported.entries[0].played);
    assert_eq!(imported.entries[1].duration, Some(600.0));
    let b = if cfg!(windows) {
        r"V:\F\b.mkv"
    } else {
        "/mnt/media/F/b.mkv"
    };
    assert_eq!(imported.entries[1].path, b);
    assert_eq!(app.store().position(b), Some(90.0));
    let e1 = if cfg!(windows) {
        r"V:\S\e1.mkv"
    } else {
        "/mnt/media/S/e1.mkv"
    };
    assert_eq!(app.store().position(e1), Some(30.0));

    // A second import adds the albums again under new names (the albums of
    // the first one are not empty).
    assert_eq!(app.import_playlists(&playlists).expect("import"), 2);
    let names: Vec<String> = app
        .library()
        .lists()
        .albums
        .iter()
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(names, ["Default", "Series", "Default (2)", "Series (2)"]);
    let _ = std::fs::remove_dir_all(&root);
}
