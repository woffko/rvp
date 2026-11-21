//! The Windows uninstaller (`packaging/windows/rvp.nsi`) removes what RVP
//! registers itself from its settings (`rvp_app::registration`): every key
//! Apply to System writes with every option on falls under one of the
//! uninstaller's removals, and every command there starts with RVP's own
//! path in quotes, which is how the uninstaller tells that an entry still
//! names its installation.

use rvp_app::registration::{AUTOPLAY, AssociationSettings, RegistryChange, registration};

const EXE: &str = r"C:\Users\u\AppData\Local\Programs\RVP\rvp.exe";

fn uninstaller() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packaging/windows/rvp.nsi"
    );
    std::fs::read_to_string(path).expect("rvp.nsi")
}

/// Everything RVP can register: each verb, every autoplay event and a type
/// of the user's.
fn everything() -> AssociationSettings {
    let mut settings = AssociationSettings {
        folder_menu: true,
        add_menu: true,
        play_menu: true,
        ..AssociationSettings::default()
    };
    settings.autoplay = AUTOPLAY
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    settings
        .extra
        .insert("rvptest".to_owned(), "video".to_owned());
    settings
}

/// The uninstaller's line that removes `key` (a value `name` of it).
fn removal(key: &str, name: &str) -> String {
    const CLASSES: &str = r"Software\Classes\";
    const HANDLERS: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\AutoplayHandlers\";
    if key == r"Software\RVP" || key.starts_with(r"Software\RVP\") {
        return r#"DeleteRegKey HKCU "${APP_KEY}""#.to_owned();
    }
    if key == r"Software\RegisteredApplications" && name == "RVP" {
        return r#"DeleteRegValue HKCU "Software\RegisteredApplications" "RVP""#.to_owned();
    }
    if let Some(event) = key.strip_prefix(HANDLERS).and_then(|rest| {
        rest.strip_prefix(r"Handlers\RVP")
            .or_else(|| rest.strip_prefix(r"EventHandlers\"))
    }) {
        return format!("!insertmacro RemoveHandler {event}");
    }
    let Some(class) = key.strip_prefix(CLASSES) else {
        panic!("{key}: no removal in the uninstaller");
    };
    for whole in [
        r"Applications\rvp.exe",
        r"Directory\shell\RVP.play",
        "RVP.AutoPlay",
    ] {
        if class == whole || class.starts_with(&format!("{whole}\\")) {
            return format!(r#"DeleteRegKey HKCU "Software\Classes\{whole}""#);
        }
    }
    if class.starts_with("RVP.") {
        return r#"DeleteRegKey HKCU "Software\Classes\$1""#.to_owned();
    }
    if class.starts_with('.') && class.ends_with(r"\OpenWithProgids") && name.starts_with("RVP.") {
        return r#"DeleteRegValue HKCU "Software\Classes\.$3\OpenWithProgids" "$1""#.to_owned();
    }
    if class.starts_with(r"SystemFileAssociations\.") && class.contains(r"\shell\RVP.play") {
        return r#"DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.$3\shell\RVP.play""#
            .to_owned();
    }
    panic!("{key}: no removal in the uninstaller");
}

#[test]
fn the_uninstaller_removes_all_rvp_registers() {
    let script = uninstaller();
    let changes = registration(&everything(), EXE);
    let mut missing = std::collections::BTreeSet::new();
    for change in &changes {
        let RegistryChange::Set { key, name, .. } = change else {
            continue;
        };
        let line = removal(key, name);
        if !script.contains(&line) {
            missing.insert(line);
        }
    }
    assert!(missing.is_empty(), "rvp.nsi lacks {missing:?}");
    // Every AutoPlay handler RVP may write.
    assert_eq!(
        script.matches("!insertmacro RemoveHandler Play").count(),
        AUTOPLAY.len()
    );
    // The Run value (`cpp/taskbar.cpp`) and the screen saver (`m14.rs`).
    for line in [
        r#"DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "RVP""#,
        r#"DeleteRegValue HKCU "Control Panel\Desktop" "SCRNSAVE.EXE""#,
    ] {
        assert!(script.contains(line), "rvp.nsi lacks {line}");
    }
}

#[test]
fn rvps_commands_start_with_its_own_path() {
    let quoted = format!("\"{EXE}\"");
    let icon = format!("{EXE},0");
    for change in registration(&everything(), EXE) {
        let RegistryChange::Set { key, name, value } = change else {
            continue;
        };
        // The uninstaller removes an entry only while it names its rvp.exe
        // this way.
        if key.ends_with(r"\command") {
            assert!(value.starts_with(&quoted), "{key}: {value}");
        }
        if name == "DefaultIcon" {
            assert_eq!(value, icon, "{key}");
        }
    }
}
