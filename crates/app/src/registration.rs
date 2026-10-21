//! M14: PotPlayer's Association page (goal P), for the current user only:
//! RVP's video and audio types (`crate::associations`, the installer's) and
//! those the user adds, each registered for RVP or not;
//! RVP's verbs on the files' and folders' context menus, and its handlers
//! for Windows' autoplay. RVP registers itself as a handler and never makes itself any
//! type's default (owner, 2026-10-16): Windows' Default apps, which the page
//! opens, chooses those.
//!
//! [`registration`] lists the registry changes under `HKEY_CURRENT_USER`
//! that Apply to System makes (Windows; the shell writes them); Linux's
//! desktop entry lists RVP's types itself (goal S).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// The kinds of the types: the table's and the user's.
pub const KINDS: &[&str] = &["video", "audio", "subtitle", "playlist"];

/// The autoplay events RVP can handle: Windows' names and RVP's.
pub const AUTOPLAY: &[(&str, &str)] = &[
    ("video", "PlayVideoFilesOnArrival"),
    ("audio", "PlayMusicFilesOnArrival"),
    ("cd", "PlayCDAudioOnArrival"),
    ("vcd", "PlayVideoCDMovieOnArrival"),
    ("svcd", "PlaySuperVideoCDMovieOnArrival"),
    ("dvd", "PlayDVDMovieOnArrival"),
    ("bluray", "PlayBluRayOnArrival"),
];

const CLASSES: &str = r"Software\Classes";
const CAPABILITIES: &str = r"Software\RVP\Capabilities";
const HANDLERS: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\AutoplayHandlers";

/// One type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FileType {
    /// Without the dot, lower case.
    pub extension: String,
    /// One of [`KINDS`].
    pub kind: String,
}

/// The table's types, in its order.
#[must_use]
pub fn table() -> Vec<FileType> {
    crate::associations::associations()
        .into_iter()
        .map(|association| FileType {
            extension: association.extension.to_owned(),
            kind: match association.kind {
                crate::associations::Kind::Video => "video",
                crate::associations::Kind::Audio => "audio",
            }
            .to_owned(),
        })
        .collect()
}

/// The Association page (`associations` in settings.json).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
// Independent options of PotPlayer's page.
#[allow(clippy::struct_excessive_bools)]
pub struct AssociationSettings {
    /// Types RVP is not registered for.
    pub off: BTreeSet<String>,
    /// Types the user added: extension and kind.
    pub extra: BTreeMap<String, String>,
    /// Descriptions the user gave, by extension.
    pub descriptions: BTreeMap<String, String>,
    /// "Play with RVP" on folders' context menus.
    pub folder_menu: bool,
    /// "Add to RVP's album" on RVP's types' context menus.
    pub add_menu: bool,
    /// "Play with RVP" on RVP's types' context menus whatever plays them.
    pub play_menu: bool,
    /// Autoplay events RVP handles (the first names of [`AUTOPLAY`]).
    pub autoplay: BTreeSet<String>,
    /// The Open File(s) dialog's media filter: extensions separated by
    /// spaces (empty: every file).
    pub dialog_extensions: String,
}

impl AssociationSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let before = self.extra.len();
        self.extra.retain(|extension, kind| {
            !extension.is_empty()
                && extension
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                && KINDS.contains(&kind.as_str())
        });
        if self.extra.len() != before {
            notes.push("associations.extra: entries without a plain extension or a known kind were left out".to_owned());
        }
        let known: Vec<&str> = AUTOPLAY.iter().map(|(name, _)| *name).collect();
        let before = self.autoplay.len();
        self.autoplay.retain(|name| known.contains(&name.as_str()));
        if self.autoplay.len() != before {
            notes.push(format!("associations.autoplay: only {known:?}"));
        }
    }

    /// Every type: the table's, then the user's.
    #[must_use]
    pub fn types(&self) -> Vec<FileType> {
        let mut types = table();
        for (extension, kind) in &self.extra {
            if !types.iter().any(|known| known.extension == *extension) {
                types.push(FileType {
                    extension: extension.clone(),
                    kind: kind.clone(),
                });
            }
        }
        types
    }

    /// A type's description: the user's, or "MKV video file".
    #[must_use]
    pub fn description(&self, file_type: &FileType) -> String {
        self.descriptions
            .get(&file_type.extension)
            .cloned()
            .unwrap_or_else(|| {
                format!(
                    "{} {} file",
                    file_type.extension.to_uppercase(),
                    file_type.kind
                )
            })
    }
}

/// One change under `HKEY_CURRENT_USER`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryChange {
    /// A string value (`name` empty: the key's default value).
    Set {
        key: String,
        name: String,
        value: String,
    },
    /// A value of a key.
    DeleteValue { key: String, name: String },
    /// A key with everything in it.
    DeleteKey { key: String },
}

fn set(key: impl Into<String>, name: &str, value: impl Into<String>) -> RegistryChange {
    RegistryChange::Set {
        key: key.into(),
        name: name.to_owned(),
        value: value.into(),
    }
}

/// The changes Apply to System makes for `settings` with RVP at `exe`.
#[must_use]
pub fn registration(settings: &AssociationSettings, exe: &str) -> Vec<RegistryChange> {
    let open = format!("\"{exe}\" \"%1\"");
    let icon = format!("{exe},0");
    let mut changes = vec![
        set(CAPABILITIES, "ApplicationName", "RVP"),
        set(
            CAPABILITIES,
            "ApplicationDescription",
            "A PotPlayer-style media player",
        ),
        set(CAPABILITIES, "ApplicationIcon", icon.clone()),
        set(
            format!(r"{CLASSES}\Applications\rvp.exe"),
            "FriendlyAppName",
            "RVP",
        ),
        set(
            format!(r"{CLASSES}\Applications\rvp.exe\DefaultIcon"),
            "",
            icon.clone(),
        ),
        set(
            format!(r"{CLASSES}\Applications\rvp.exe\shell\open\command"),
            "",
            open.clone(),
        ),
        set(r"Software\RegisteredApplications", "RVP", CAPABILITIES),
    ];
    for file_type in settings.types() {
        type_changes(settings, &file_type, exe, &mut changes);
    }
    let folder = format!(r"{CLASSES}\Directory\shell\RVP.play");
    if settings.folder_menu {
        changes.extend([
            set(folder.clone(), "", "Play with RVP"),
            set(folder.clone(), "Icon", icon.clone()),
            set(format!(r"{folder}\command"), "", open),
        ]);
    } else {
        changes.push(RegistryChange::DeleteKey { key: folder });
    }
    autoplay_changes(settings, exe, &mut changes);
    changes
}

/// One type's changes: registered (with its verbs) or left out.
fn type_changes(
    settings: &AssociationSettings,
    file_type: &FileType,
    exe: &str,
    changes: &mut Vec<RegistryChange>,
) {
    let open = format!("\"{exe}\" \"%1\"");
    let icon = format!("{exe},0");
    let extension = &file_type.extension;
    let progid = format!(r"{CLASSES}\RVP.{extension}");
    let openwith = format!(r"{CLASSES}\.{extension}\OpenWithProgids");
    let supported = format!(r"{CLASSES}\Applications\rvp.exe\SupportedTypes");
    let capabilities = format!(r"{CAPABILITIES}\FileAssociations");
    let play_with = format!(r"{CLASSES}\SystemFileAssociations\.{extension}\shell\RVP.play");
    if settings.off.contains(extension) {
        changes.extend([
            RegistryChange::DeleteKey { key: progid },
            RegistryChange::DeleteValue {
                key: openwith,
                name: format!("RVP.{extension}"),
            },
            RegistryChange::DeleteValue {
                key: supported,
                name: format!(".{extension}"),
            },
            RegistryChange::DeleteValue {
                key: capabilities,
                name: format!(".{extension}"),
            },
            RegistryChange::DeleteKey { key: play_with },
        ]);
        return;
    }
    changes.extend([
        set(progid.clone(), "", settings.description(file_type)),
        set(format!(r"{progid}\DefaultIcon"), "", icon.clone()),
        set(format!(r"{progid}\shell\open\command"), "", open.clone()),
        set(openwith, &format!("RVP.{extension}"), ""),
        set(supported, &format!(".{extension}"), ""),
        set(
            capabilities,
            &format!(".{extension}"),
            format!("RVP.{extension}"),
        ),
    ]);
    if settings.add_menu {
        changes.extend([
            set(format!(r"{progid}\shell\add"), "", "Add to RVP's album"),
            set(
                format!(r"{progid}\shell\add\command"),
                "",
                format!("\"{exe}\" /add \"%1\""),
            ),
        ]);
    } else {
        changes.push(RegistryChange::DeleteKey {
            key: format!(r"{progid}\shell\add"),
        });
    }
    if settings.play_menu {
        changes.extend([
            set(play_with.clone(), "", "Play with RVP"),
            set(play_with.clone(), "Icon", icon),
            set(format!(r"{play_with}\command"), "", open),
        ]);
    } else {
        changes.push(RegistryChange::DeleteKey { key: play_with });
    }
}

/// RVP's handlers for Windows' autoplay events, on or off.
fn autoplay_changes(settings: &AssociationSettings, exe: &str, changes: &mut Vec<RegistryChange>) {
    let icon = format!("{exe},0");
    changes.push(set(
        format!(r"{CLASSES}\RVP.AutoPlay\shell\play\command"),
        "",
        format!("\"{exe}\" \"%L\""),
    ));
    for (name, event) in AUTOPLAY {
        let handler = format!("RVP{event}");
        let key = format!(r"{HANDLERS}\Handlers\{handler}");
        let event_key = format!(r"{HANDLERS}\EventHandlers\{event}");
        if settings.autoplay.contains(*name) {
            changes.extend([
                set(key.clone(), "Action", "Play with RVP"),
                set(key.clone(), "Provider", "RVP"),
                set(key.clone(), "DefaultIcon", icon.clone()),
                set(key.clone(), "InvokeProgID", "RVP.AutoPlay"),
                set(key, "InvokeVerb", "play"),
                set(event_key, &handler, ""),
            ]);
        } else {
            changes.extend([
                RegistryChange::DeleteKey { key },
                RegistryChange::DeleteValue {
                    key: event_key,
                    name: handler,
                },
            ]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_video_and_audio_types() {
        let types = table();
        assert!(types.len() > 60, "{}", types.len());
        assert!(types.iter().all(|t| t.kind == "video" || t.kind == "audio"));
        assert!(
            types
                .iter()
                .any(|t| t.extension == "mkv" && t.kind == "video")
        );
    }

    #[test]
    fn registration_registers_and_leaves_types_out() {
        let mut settings = AssociationSettings::default();
        settings.off.insert("mkv".to_owned());
        settings.extra.insert("xyz".to_owned(), "video".to_owned());
        settings.add_menu = true;
        settings.autoplay.insert("dvd".to_owned());
        let changes = registration(&settings, r"C:\RVP\rvp.exe");
        let has = |wanted: &RegistryChange| changes.contains(wanted);
        assert!(has(&RegistryChange::DeleteKey {
            key: r"Software\Classes\RVP.mkv".to_owned()
        }));
        assert!(has(&set(
            r"Software\Classes\RVP.mp4\shell\open\command",
            "",
            r#""C:\RVP\rvp.exe" "%1""#
        )));
        assert!(has(&set(r"Software\Classes\RVP.xyz", "", "XYZ video file")));
        assert!(has(&set(
            r"Software\Classes\RVP.mp4\shell\add\command",
            "",
            r#""C:\RVP\rvp.exe" /add "%1""#
        )));
        assert!(has(&set(
            format!(r"{HANDLERS}\EventHandlers\PlayDVDMovieOnArrival"),
            "RVPPlayDVDMovieOnArrival",
            ""
        )));
        assert!(has(&RegistryChange::DeleteKey {
            key: format!(r"{HANDLERS}\Handlers\RVPPlayMusicFilesOnArrival")
        }));
        // Never a type's default: no `.ext` default value is written.
        assert!(!changes.iter().any(|change| matches!(
            change,
            RegistryChange::Set { key, name, .. } if key.starts_with(r"Software\Classes\.") && name.is_empty()
        )));
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = AssociationSettings::default();
        settings.extra.insert("ok".to_owned(), "video".to_owned());
        settings
            .extra
            .insert("b a d".to_owned(), "video".to_owned());
        settings.autoplay.insert("floppy".to_owned());
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert_eq!(settings.extra.len(), 1);
        assert!(settings.autoplay.is_empty());
    }
}
