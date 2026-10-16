//! The settings (M5) on synthetic files: where they live (portable mode),
//! the round trip through `settings.json`, checking, applying a change to
//! the running player, and import and export.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rvp_app::preferences::{self, change};
use rvp_app::{App, Dirs, PORTABLE_MARKER, Settings, read_json};
use rvp_engine::{Engine, EngineOptions};
use serde_json::json;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvp-settings-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| OsString::from(value))
    }
}

#[test]
fn portable_mode_keeps_the_settings_next_to_the_executable() {
    let exe = scratch("portable");
    let home = [
        ("HOME", "/home/someone"),
        ("APPDATA", r"C:\Users\someone\AppData\Roaming"),
    ];
    // Without the marker: the platform's folders.
    let dirs = Dirs::resolve(env(&home), Some(&exe), false).expect("dirs");
    assert_eq!(dirs.config, Path::new("/home/someone/.config/rvp"));
    assert_eq!(dirs.state, Path::new("/home/someone/.local/state/rvp"));
    assert!(!dirs.portable);
    let dirs = Dirs::resolve(env(&home), Some(&exe), true).expect("dirs");
    assert_eq!(
        dirs.config,
        Path::new(r"C:\Users\someone\AppData\Roaming").join("RVP")
    );
    // With it: next to the executable, on both systems.
    std::fs::write(exe.join(PORTABLE_MARKER), "").expect("marker");
    for windows in [false, true] {
        let dirs = Dirs::resolve(env(&home), Some(&exe), windows).expect("dirs");
        assert_eq!(dirs.config, exe);
        assert_eq!(dirs.state, exe.join("state"));
        assert!(dirs.portable);
    }
    // The test overrides win.
    let overrides = [("RVP_CONFIG_DIR", "/tmp/c"), ("RVP_STATE_DIR", "/tmp/s")];
    let dirs = Dirs::resolve(env(&overrides), Some(&exe), false).expect("dirs");
    assert_eq!(
        (dirs.config.as_path(), dirs.state.as_path()),
        (Path::new("/tmp/c"), Path::new("/tmp/s"))
    );
    assert!(!dirs.portable);
    // Nothing to go on.
    assert!(Dirs::resolve(env(&[]), None, false).is_err());

    // A portable start writes its settings there.
    let dirs = Dirs::resolve(env(&home), Some(&exe), false).expect("dirs");
    let app = App::load(&dirs).expect("load");
    assert_eq!(
        app.settings_path(),
        Some(exe.join("settings.json").as_path())
    );
    assert!(exe.join("settings.json").is_file());
    let _ = std::fs::remove_dir_all(&exe);
}

#[test]
fn changes_apply_to_the_player_and_survive_a_restart() {
    let dir = scratch("round-trip");
    let dirs = Dirs::inside(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut app = App::load(&dirs).expect("load");
    assert_eq!(app.settings, Settings::default());

    let mut settings = app.settings.clone();
    for (path, value) in [
        ("hwdec", json!("no")),
        ("video-sync", json!("display-resample")),
        ("scaler", json!("spline36")),
        ("volume-max", json!(150)),
        ("subtitle-scale", json!(1.5)),
        ("subtitle-languages", json!("en,ru")),
        ("subtitle-auto", json!("exact")),
        ("normalizer.boost", json!(false)),
        ("normalizer.enabled", json!(false)),
        ("seek-seconds.0", json!(10)),
    ] {
        let (changed, notes) = change(&settings, path, value).expect(path);
        assert!(notes.is_empty(), "{path}: {notes:?}");
        settings = changed;
    }
    settings.keys.insert("Ctrl+B".into(), "frame-back".into());
    let applied = app.apply_settings(&engine, settings.clone());
    assert!(applied.notes.is_empty(), "{:?}", applied.notes);
    assert!(!applied.restart);
    // On the running player at once.
    let text = |name: &str| engine.get_text(name).unwrap_or_default();
    assert_eq!(text("hwdec"), "no");
    assert_eq!(text("video-sync"), "display-resample");
    assert_eq!(text("scale"), "spline36");
    // RVP's 150 % (PotPlayer's scale) is mpv's volume of the same gain,
    // a thousandth above it so that 150 % itself is in range.
    let volume_max: f64 = text("volume-max").parse().expect("a number");
    let wanted: f64 = rvp_app::volume::mpv_volume_max(150.0)
        .parse()
        .expect("a number");
    assert!((volume_max - wanted).abs() < 1e-4, "{volume_max}");
    assert!(volume_max > rvp_app::volume::mpv_volume(150.0));
    assert_eq!(text("sub-scale"), "1.500000");
    assert_eq!(text("slang"), "en,ru");
    assert_eq!(text("sub-auto"), "exact");
    // The normalizer is off: its chain and the tempo filter are gone.
    assert!(!text("af").contains("@rvpnorm"), "{}", text("af"));
    assert_eq!(
        app.bindings
            .key(&rvp_app::KeyChord::parse("Ctrl+B").expect("key")),
        Some(rvp_app::Action::FrameBack)
    );
    // Back to mpv's default scaler.
    let (default_scaler, _) = change(&settings, "scaler", json!("")).expect("scaler");
    app.apply_settings(&engine, default_scaler.clone());
    assert_eq!(text("scale"), text("option-info/scale/default-value"));
    // The skin waits for a restart.
    let (skin, _) = change(&default_scaler, "skin", json!("/no/such.dsf")).expect("skin");
    let applied = app.apply_settings(&engine, skin.clone());
    assert!(applied.restart);
    assert_eq!(applied.notes.len(), 1, "{:?}", applied.notes);

    app.shutdown(&engine, Instant::now()).expect("shutdown");
    let saved: Settings = read_json(&dirs.config.join("settings.json"))
        .expect("read")
        .expect("saved");
    assert_eq!(saved, skin);
    let again = App::load(&dirs).expect("reload");
    assert_eq!(again.settings, skin);
    assert_eq!(again.warnings, ["skin /no/such.dsf is not there"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_settings_file_moves_between_installations() {
    let dir = scratch("import-export");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut first = App::load(&Dirs::inside(&dir.join("first"))).expect("first");
    let (changed, _) = change(&first.settings, "volume-step", json!(4)).expect("change");
    first.apply_settings(&engine, changed);
    let file = dir.join("rvp-settings.json");
    preferences::export(&first.settings, &file).expect("export");

    let mut second = App::load(&Dirs::inside(&dir.join("second"))).expect("second");
    let (imported, notes) = preferences::import(&file).expect("import");
    assert!(notes.is_empty(), "{notes:?}");
    second.apply_settings(&engine, imported);
    assert_eq!(second.settings, first.settings);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_settings_window_gets_values_choices_and_the_file() {
    let dir = scratch("view");
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let app = App::load(&Dirs::inside(&dir)).expect("load");
    let view = app.settings_view(&engine, false, true);
    assert_eq!(view["settings"]["volume-step"], json!(1.0));
    assert_eq!(view["settings"]["normalizer"]["level"], json!(75.0));
    assert!(
        view["choices"]["hwdec"]
            .as_array()
            .is_some_and(|c| c.len() > 3)
    );
    assert!(
        view["choices"]["audio-device"]
            .as_array()
            .is_some_and(|devices| devices.iter().any(|d| d["name"] == "auto"))
    );
    assert_eq!(
        view["file"].as_str().map(PathBuf::from),
        Some(dir.join("config").join("settings.json"))
    );
    assert_eq!(view["portable"], json!(false));
    // Without the device list (reloads after a change).
    let view = app.settings_view(&engine, false, false);
    assert_eq!(view["choices"]["audio-device"], json!([]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_subtitle_font_reaches_mpv_and_the_control_panel() {
    let dir = scratch("font");
    let dirs = Dirs::inside(&dir);
    let engine = Engine::new(&EngineOptions::headless()).expect("engine");
    let mut app = App::load(&dirs).expect("load");
    let label = |app: &App| {
        let mut data = rvp_skin::DataModel::default();
        app.panel_data(&mut data);
        data.text("CMD_CAPTION_FONT_MESSAGE").map(str::to_owned)
    };
    assert_eq!(label(&app).as_deref(), Some("Default"));
    // From the font dialog.
    assert_eq!(
        app.set_subtitle_font(&engine, " DejaVu Sans "),
        rvp_app::UiRequest::Osd("Subtitle font: DejaVu Sans".to_owned())
    );
    assert_eq!(engine.get_text("sub-font").as_deref(), Some("DejaVu Sans"));
    assert_eq!(label(&app).as_deref(), Some("DejaVu Sans"));
    // Not a family: mpv's default, with a note.
    assert!(matches!(
        app.set_subtitle_font(&engine, "Bad\u{7}Font"),
        rvp_app::UiRequest::Message(_)
    ));
    assert_eq!(app.settings.subtitle_font, "");
    assert_eq!(
        engine.get_text("sub-font"),
        engine.get_text("option-info/sub-font/default-value")
    );
    // The control panel's place survives a restart.
    app.set_subtitle_font(&engine, "DejaVu Sans");
    app.set_panel_position(120, 340);
    app.shutdown(&engine, Instant::now()).expect("shutdown");
    let again = App::load(&dirs).expect("reload");
    assert_eq!(again.settings.subtitle_font, "DejaVu Sans");
    assert_eq!(again.panel_position(), Some((120, 340)));
    let _ = std::fs::remove_dir_all(&dir);
}
