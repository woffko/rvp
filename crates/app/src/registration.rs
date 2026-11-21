//! M14: PotPlayer's Association page (goal P), for the current user only:
//! RVP's video and audio types (`crate::associations`, the installer's) and
//! those the user adds, each registered for RVP or not;
//! RVP's verbs on the files' and folders' context menus, and its handlers
//! for Windows' autoplay. RVP registers itself as a handler and never makes itself any
//! type's default (owner, 2026-10-16): Windows' Default apps, which the page
//! opens, chooses those.
//!
//! [`registration`] lists the registry changes under `HKEY_CURRENT_USER`
//! that Apply to System makes (Windows; the shell writes them). On Linux
//! Apply writes the user's desktop entry ([`desktop_entry`]), which takes
//! the place of the packages' entry of the same name: it lists the MIME
//! types of the types that are on, and the desktop's own settings choose
//! the defaults.

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

/// The command of a verb: RVP at `exe`, its `switches` (PotPlayer's), then
/// `--` and the shell's `placeholder`: what the shell puts there is a
/// file, never an option.
fn command(exe: &str, switches: &str, placeholder: &str) -> String {
    format!("\"{exe}\" {switches}-- \"{placeholder}\"")
}

/// The changes Apply to System makes for `settings` with RVP at `exe`.
#[must_use]
pub fn registration(settings: &AssociationSettings, exe: &str) -> Vec<RegistryChange> {
    let open = command(exe, "", "%1");
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
    let open = command(exe, "", "%1");
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
                command(exe, "/add ", "%1"),
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
    // The handlers' ProgID, while any handler needs it.
    let progid = format!(r"{CLASSES}\RVP.AutoPlay");
    if settings.autoplay.is_empty() {
        changes.push(RegistryChange::DeleteKey { key: progid });
    } else {
        changes.push(set(
            format!(r"{progid}\shell\play\command"),
            "",
            command(exe, "", "%L"),
        ));
    }
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

/// The desktop entry (`rvp.desktop`) the packages install.
const DESKTOP_ENTRY: &str = include_str!("../../../packaging/linux/rvp.desktop");

/// The disc kinds of [`AUTOPLAY`] as shared-mime-info's `x-content` types,
/// which GNOME's Removable Media settings offer the programs of; video and
/// audio files on a new drive have none.
const AUTOPLAY_CONTENT: &[(&str, &str)] = &[
    ("cd", "x-content/audio-cdda"),
    ("vcd", "x-content/video-vcd"),
    ("svcd", "x-content/video-svcd"),
    ("dvd", "x-content/video-dvd"),
    ("bluray", "x-content/video-bluray"),
];

/// Linux's user desktop entry for `settings` with RVP at `exe`: the
/// packages' entry with the MIME types of the types that are on (a type
/// the user added takes its type from shared-mime-info's `globs`, the
/// text of `globs2`) and the `x-content` types of the chosen autoplay
/// discs.
#[must_use]
pub fn desktop_entry(settings: &AssociationSettings, exe: &str, globs: &str) -> String {
    let table = crate::associations::associations();
    let mut mime: Vec<String> = Vec::new();
    for file_type in settings.types() {
        if settings.off.contains(&file_type.extension) {
            continue;
        }
        let known = table
            .iter()
            .find(|association| association.extension == file_type.extension);
        let types: Vec<String> = match known {
            Some(association) => association
                .mime_types
                .iter()
                .map(|&t| t.to_owned())
                .collect(),
            None => glob_types(globs, &file_type.extension),
        };
        for name in types {
            if !mime.contains(&name) {
                mime.push(name);
            }
        }
    }
    for (kind, content) in AUTOPLAY_CONTENT {
        if settings.autoplay.contains(*kind) {
            mime.push((*content).to_owned());
        }
    }
    let types = if mime.is_empty() {
        String::new()
    } else {
        format!("{};", mime.join(";"))
    };
    DESKTOP_ENTRY
        .replace(
            "Exec=rvp -- %F",
            &format!("Exec={} -- %F", desktop_exec(exe)),
        )
        .replace("@MIME_TYPES@", &types)
}

/// The MIME types a desktop entry lists (its `MimeType` key).
#[must_use]
pub fn listed_types(entry: &str) -> BTreeSet<String> {
    entry
        .lines()
        .find_map(|line| line.strip_prefix("MimeType="))
        .map(|types| {
            types
                .split(';')
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether a desktop entry's types include one of `file_type`'s.
#[must_use]
pub fn listed(listed: &BTreeSet<String>, file_type: &FileType, globs: &str) -> bool {
    let types = crate::associations::associations()
        .into_iter()
        .find(|association| association.extension == file_type.extension)
        .map_or_else(
            || glob_types(globs, &file_type.extension),
            |association| {
                association
                    .mime_types
                    .iter()
                    .map(|&t| t.to_owned())
                    .collect()
            },
        );
    types.iter().any(|name| listed.contains(name))
}

/// shared-mime-info's types for `*.extension` (`weight:type:pattern`
/// lines of `globs2`).
fn glob_types(globs: &str, extension: &str) -> Vec<String> {
    let pattern = format!("*.{extension}");
    let mut found = Vec::new();
    for line in globs.lines().filter(|line| !line.starts_with('#')) {
        let mut cells = line.split(':');
        let (Some(_), Some(name), Some(glob)) = (cells.next(), cells.next(), cells.next()) else {
            continue;
        };
        if glob.eq_ignore_ascii_case(&pattern) && !found.iter().any(|known: &String| known == name)
        {
            found.push(name.to_owned());
        }
    }
    found
}

/// A program path as a desktop entry's `Exec` takes it: quoted when it
/// holds a reserved character. The string's escape comes before the
/// quoting's (the Desktop Entry specification), so a quoted `"`, `` ` ``
/// or `$` takes two backslashes and a backslash four; a `%` is doubled, or
/// it would start a field code (the Linux audit's L10). A line break, a
/// carriage return and a tab are the string's escapes (`\n`, `\r`, `\t`),
/// so the entry keeps the path on its own line.
#[must_use]
pub fn desktop_exec(exe: &str) -> String {
    if exe
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/-_.+".contains(c))
    {
        return exe.to_owned();
    }
    let mut quoted = String::from("\"");
    for c in exe.chars() {
        match c {
            '"' | '`' | '$' => quoted.push_str(r"\\"),
            '\\' => quoted.push_str(r"\\\"),
            '%' => quoted.push('%'),
            '\n' => {
                quoted.push_str(r"\n");
                continue;
            }
            '\r' => {
                quoted.push_str(r"\r");
                continue;
            }
            '\t' => {
                quoted.push_str(r"\t");
                continue;
            }
            _ => {}
        }
        quoted.push(c);
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linuxs_entry_lists_the_types_that_are_on() {
        let mut settings = AssociationSettings::default();
        settings.off.insert("mkv".to_owned());
        settings.extra.insert("xyz".to_owned(), "video".to_owned());
        settings.extra.insert("nope".to_owned(), "video".to_owned());
        let globs = "# header\n50:video/x-xyz:*.xyz\n50:video/x-matroska:*.mkv\n";
        let entry = desktop_entry(&settings, "/tmp/My Apps/RVP-x86_64.AppImage", globs);
        assert!(
            entry.contains("Exec=\"/tmp/My Apps/RVP-x86_64.AppImage\" -- %F\n"),
            "{entry}"
        );
        let types = listed_types(&entry);
        assert!(!types.contains("video/x-matroska"));
        assert!(types.contains("video/webm") && types.contains("video/x-xyz"));
        let file = |extension: &str| FileType {
            extension: extension.to_owned(),
            kind: "video".to_owned(),
        };
        assert!(listed(&types, &file("webm"), globs));
        assert!(listed(&types, &file("xyz"), globs));
        assert!(!listed(&types, &file("mkv"), globs));
        assert!(!listed(&types, &file("nope"), globs));
        let plain = desktop_entry(&settings, "/opt/rvp/bin/rvp", globs);
        assert!(plain.contains("\nExec=/opt/rvp/bin/rvp -- %F\n"));
        assert!(!listed_types(&plain).contains("x-content/video-dvd"));
        settings.autoplay.insert("dvd".to_owned());
        settings.autoplay.insert("video".to_owned());
        let discs = listed_types(&desktop_entry(&settings, "/opt/rvp/bin/rvp", globs));
        assert!(discs.contains("x-content/video-dvd"));
        assert!(
            !discs
                .iter()
                .any(|name| name.starts_with("x-content/video-") && name != "x-content/video-dvd")
        );
        assert_eq!(desktop_exec(r"/a/b$c\d"), r#""/a/b\\$c\\\\d""#);
        assert_eq!(
            desktop_exec(r#"/opt/RVP%20Player/a "b"`c`"#),
            r#""/opt/RVP%%20Player/a \\"b\\"\\`c\\`""#
        );
        assert_eq!(desktop_exec("/usr/bin/rvp"), "/usr/bin/rvp");
    }

    #[test]
    fn a_line_break_in_the_program_path_stays_on_the_exec_line() {
        assert_eq!(
            desktop_exec("/home/a\nb/c\rd\te/rvp"),
            r#""/home/a\nb/c\rd\te/rvp""#
        );
        let entry = desktop_entry(
            &AssociationSettings::default(),
            "/home/a\nOther=1\n/rvp",
            "",
        );
        assert!(!entry.lines().any(|line| line == "Other=1"), "{entry}");
        let exec: Vec<&str> = entry
            .lines()
            .filter(|line| line.starts_with("Exec="))
            .collect();
        assert_eq!(exec, [r#"Exec="/home/a\nOther=1\n/rvp" -- %F"#]);
    }

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
            r#""C:\RVP\rvp.exe" -- "%1""#
        )));
        assert!(has(&set(r"Software\Classes\RVP.xyz", "", "XYZ video file")));
        assert!(has(&set(
            r"Software\Classes\RVP.mp4\shell\add\command",
            "",
            r#""C:\RVP\rvp.exe" /add -- "%1""#
        )));
        assert!(has(&set(
            format!(r"{HANDLERS}\EventHandlers\PlayDVDMovieOnArrival"),
            "RVPPlayDVDMovieOnArrival",
            ""
        )));
        assert!(has(&RegistryChange::DeleteKey {
            key: format!(r"{HANDLERS}\Handlers\RVPPlayMusicFilesOnArrival")
        }));
        // Every verb's command ends its options before the shell's file.
        let commands: Vec<&str> = changes
            .iter()
            .filter_map(|change| match change {
                RegistryChange::Set { key, value, .. } if key.ends_with(r"\command") => {
                    Some(value.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(commands.len() > 60, "{}", commands.len());
        assert!(
            commands
                .iter()
                .all(|value| value.ends_with(r#" -- "%1""#) || value.ends_with(r#" -- "%L""#)),
            "{commands:?}"
        );
        assert!(has(&set(
            r"Software\Classes\RVP.AutoPlay\shell\play\command",
            "",
            r#""C:\RVP\rvp.exe" -- "%L""#
        )));
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
