//! A skin chosen while RVP runs and at start (M14), on synthetic skins: one
//! that does not load leaves the skin shown and the setting as they were,
//! the setting names a skin only once the window has shown it, and a start
//! after a load that never finished (`rvp_app::skin_marker`) shows RVP
//! Default and says why. The shell's part (loading, showing, drawing) is
//! played here by its calls into the application.

use std::io::{Cursor, Write as _};
use std::path::{Path, PathBuf};

use rvp_app::skin_marker::{self, Marker};
use rvp_app::{App, Dirs, Settings, UiRequest, read_json};
use rvp_skin::{Skin, SkinError};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-skin-choice-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A `.dsf` skin: a ZIP archive of `files`.
fn write_skin(path: &Path, files: &[(&str, &str)]) {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in files {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .expect("member");
        writer.write_all(text.as_bytes()).expect("write");
    }
    let bytes = writer.finish().expect("zip").into_inner();
    std::fs::write(path, bytes).expect("skin file");
}

/// A skin over one of the skin engine's limits: more includes than any
/// skin has.
fn write_skin_over_a_limit(path: &Path) {
    let includes = "#include<part.txt>\n".repeat(rvp_skin::limits::INCLUDES + 10);
    write_skin(path, &[("VideoSkin.xml", &includes), ("part.txt", "")]);
}

/// The settings with `skin`, as a start reads them.
fn configure(dirs: &Dirs, skin: Option<&Path>) {
    let settings = Settings {
        skin: skin.map(Path::to_path_buf),
        ..Settings::default()
    };
    std::fs::create_dir_all(&dirs.config).expect("config dir");
    rvp_app::write_json(&dirs.config.join("settings.json"), &settings).expect("settings");
}

/// The skin setting on disk.
fn saved_skin(dirs: &Dirs) -> Option<PathBuf> {
    read_json::<Settings>(&dirs.config.join("settings.json"))
        .expect("read")
        .expect("settings")
        .skin
}

#[test]
fn a_skin_that_does_not_load_leaves_the_skin_and_the_setting() {
    let dir = scratch("refused");
    let dirs = Dirs::inside(&dir);
    let shown = dir.join("Shown.dsf");
    write_skin(&shown, &[("VideoSkin.xml", "<Skin/>")]);
    let over = dir.join("Over.dsf");
    write_skin_over_a_limit(&over);
    configure(&dirs, Some(&shown));

    // Opened (or handed over by a second `rvp`): the skin is chosen, and
    // the media play. Until the window shows it, the settings keep the
    // skin before; RVP stops before its load returns.
    let mut app = App::load(&dirs).expect("load");
    app.skin_started(Some(shown.clone()));
    let mut files = vec![over.display().to_string(), "/media/a.mkv".to_owned()];
    let request = app.take_skin_files(&mut files);
    assert!(
        matches!(&request, Some(UiRequest::SkinChanged(text)) if text == "Skin: Over.dsf"),
        "{request:?}"
    );
    assert_eq!(files, ["/media/a.mkv"]);
    assert_eq!(app.chosen_skin().as_deref(), Some(over.as_path()));
    app.save_settings();
    drop(app);
    assert_eq!(saved_skin(&dirs).as_deref(), Some(shown.as_path()));

    // Again, and the load ends with the skin engine's error.
    let mut app = App::load(&dirs).expect("load");
    app.skin_started(Some(shown.clone()));
    let mut files = vec![over.display().to_string()];
    assert!(app.take_skin_files(&mut files).is_some());
    let error = Skin::open(&over).expect_err("over a limit");
    assert!(matches!(error, SkinError::Limit(_)), "{error}");
    let notice = app.skin_failed(Some(&over), &format!("{}: {error}", over.display()));
    assert!(
        notice.contains("Over.dsf")
            && notice.contains("include")
            && notice.ends_with("Shown.dsf stays"),
        "{notice}"
    );
    assert_eq!(app.settings.skin.as_deref(), Some(shown.as_path()));
    assert_eq!(app.chosen_skin().as_deref(), Some(shown.as_path()));
    assert_eq!(app.skin_showing(), Some(shown.as_path()));
    // The choice's notice gives way to the reason, once.
    assert!(app.take_skin_refused());
    assert!(!app.take_skin_refused());
    // The Skins menu still lists both.
    let list = app.skin_list();
    assert!(list.contains(&Some(over.clone())) && list.contains(&Some(shown.clone())));

    // A skin that loads is the setting from the moment it shows.
    let other = dir.join("Other.dsf");
    write_skin(&other, &[("VideoSkin.xml", "<Skin/>")]);
    let mut files = vec![other.display().to_string()];
    assert!(app.take_skin_files(&mut files).is_some());
    app.skin_loaded(Some(&other));
    assert_eq!(app.skin_showing(), Some(other.as_path()));
    drop(app);
    assert_eq!(saved_skin(&dirs).as_deref(), Some(other.as_path()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_command_line_skin_that_does_not_load_shows_rvp_default() {
    let dir = scratch("command-line");
    let dirs = Dirs::inside(&dir);
    let configured = dir.join("Configured.dsf");
    write_skin(&configured, &[("VideoSkin.xml", "<Skin/>")]);
    configure(&dirs, Some(&configured));
    // `--skin=Over.dsf` did not load: RVP Default shows, chosen until the
    // user picks a skin; the setting stays.
    let mut app = App::load(&dirs).expect("load");
    app.skin_started(None);
    assert_eq!(app.chosen_skin(), None);
    assert_eq!(app.settings.skin.as_deref(), Some(configured.as_path()));
    // The configured skin is listed to choose.
    assert!(app.skin_list().contains(&Some(configured.clone())));
    drop(app);
    assert_eq!(saved_skin(&dirs).as_deref(), Some(configured.as_path()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_start_after_a_load_that_never_finished_shows_rvp_default() {
    let dir = scratch("unfinished");
    let dirs = Dirs::inside(&dir);
    let skin = dir.join("Stops.dsf");
    write_skin(&skin, &[("VideoSkin.xml", "<Skin/>")]);
    configure(&dirs, Some(&skin));
    // The last RVP ended while it loaded the configured skin, with no
    // other RVP on the folder: its marker stays behind.
    let mut marker = Marker::create(&dirs.state, 4_000_001, true).expect("marker");
    marker.set_skin(Some(&skin));
    drop(marker);

    // The next start finds it before the skin loads.
    let (own, unfinished) = skin_marker::start(&dirs.state);
    assert_eq!(unfinished, std::slice::from_ref(&skin));
    let mut app = App::load(&dirs).expect("load");
    let notices = app.skip_unfinished_skins(&unfinished);
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(
        notices[0].contains("Stops.dsf") && notices[0].contains("RVP Default"),
        "{notices:?}"
    );
    // RVP Default shows and the setting is cleared, but the skin is still
    // there to choose on purpose.
    assert_eq!(app.settings.skin, None);
    app.skin_started(None);
    assert_eq!(app.chosen_skin(), None);
    assert!(app.skin_list().contains(&Some(skin.clone())));
    drop(app);
    assert_eq!(saved_skin(&dirs), None);
    // The start's own marker is the only one, and a clean exit removes it.
    let names = |state: &Path| -> Vec<String> {
        std::fs::read_dir(state)
            .expect("state")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("skin-load-"))
            .collect()
    };
    assert_eq!(
        names(&dirs.state),
        [format!("skin-load-{}.json", std::process::id())]
    );
    own.expect("a marker").remove();
    assert!(names(&dirs.state).is_empty());

    // A skin that is not the configured one: said, the setting stays.
    let configured = dir.join("Configured.dsf");
    write_skin(&configured, &[("VideoSkin.xml", "<Skin/>")]);
    configure(&dirs, Some(&configured));
    let mut marker = Marker::create(&dirs.state, 4_000_002, true).expect("marker");
    marker.set_skin(Some(&skin));
    drop(marker);
    let found = skin_marker::scan(&dirs.state);
    let mut app = App::load(&dirs).expect("load");
    let notices = app.skip_unfinished_skins(&found.unfinished);
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(
        notices[0].contains("Stops.dsf") && !notices[0].contains("RVP Default"),
        "{notices:?}"
    );
    assert_eq!(app.settings.skin.as_deref(), Some(configured.as_path()));
    drop(app);
    assert_eq!(saved_skin(&dirs).as_deref(), Some(configured.as_path()));

    // Another RVP that runs while it loads the configured skin: not this
    // start's business.
    let mut running = Marker::create(&dirs.state, 4_000_003, true).expect("marker");
    running.set_skin(Some(&configured));
    let found = skin_marker::scan(&dirs.state);
    assert!(found.others && found.unfinished.is_empty(), "{found:?}");
    running.remove();
    let _ = std::fs::remove_dir_all(&dir);
}
