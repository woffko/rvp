//! RVP's interface languages (M14): the Russian translation
//! (`shell/i18n/qml_ru.ts`, written for RVP) covers every string of the
//! QML files and every label of RVP's menus. Menu labels are looked up as
//! QML's `trMenu` does: their numbers stand aside as `%1`, `%2`...
//!
//! `RVP_DUMP_MENU=FILE` writes the menu labels without a translation to
//! FILE, one per line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use rvp_app::menu::{Live, MenuItem};
use rvp_app::{App, Settings};

/// QML's strings (lupdate's) and the menu labels (RVP's own list).
const TS: [&str; 2] = [
    include_str!("../../../shell/i18n/qml_ru.ts"),
    include_str!("../../../shell/i18n/menu_ru.ts"),
];

/// `context` -> `source` -> translation (empty when unfinished).
fn translations() -> BTreeMap<String, BTreeMap<String, String>> {
    let unescape = |text: &str| {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    };
    let between = |text: &str, open: &str, close: &str| -> Option<String> {
        let start = text.find(open)? + open.len();
        let end = text[start..].find(close)? + start;
        Some(text[start..end].to_owned())
    };
    let mut found = BTreeMap::new();
    for context in TS.iter().flat_map(|file| file.split("<context>").skip(1)) {
        let name = between(context, "<name>", "</name>").unwrap_or_default();
        let messages: &mut BTreeMap<String, String> = found.entry(name).or_default();
        for message in context.split("<message>").skip(1) {
            let Some(source) = between(message, "<source>", "</source>") else {
                continue;
            };
            let translation =
                if message.contains("type=\"unfinished\"") || message.contains("<translation/>") {
                    String::new()
                } else {
                    message
                        .find("<translation")
                        .and_then(|at| {
                            let rest = &message[at..];
                            let start = rest.find('>')? + 1;
                            let end = rest.find("</translation>")?;
                            Some(rest[start..end].to_owned())
                        })
                        .unwrap_or_default()
                };
            messages.insert(unescape(&source), unescape(&translation));
        }
    }
    found
}

/// A label as `trMenu` looks it up.
fn key(label: &str) -> String {
    let mut out = String::new();
    let mut count = 0;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            while chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
            // A decimal part belongs to the number.
            let mut ahead = chars.clone();
            if matches!(ahead.next(), Some('.' | ','))
                && ahead.next().is_some_and(|d| d.is_ascii_digit())
            {
                chars.next();
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
            }
            count += 1;
            out.push('%');
            out.push_str(&count.to_string());
        } else {
            out.push(c);
        }
    }
    out
}

fn walk(items: &[MenuItem], out: &mut BTreeSet<String>) {
    for item in items {
        if !item.separator && !item.label.is_empty() {
            out.insert(item.label.clone());
        }
        walk(&item.children, out);
    }
}

/// Labels that are names, not words: RVP's own names, the languages, the
/// user's albums and whisper.cpp's engines (listed when installed, as on
/// Windows).
fn is_name(label: &str) -> bool {
    ["RVP Default (built in)", "English", "Русский", "Default"].contains(&label)
        || rvp_app::speech::ENGINES
            .iter()
            .any(|(_, engine)| *engine == label)
        // A model that is present shows its bare name (RVP_WHISPER_MODELS).
        || rvp_app::speech::MODELS.contains(&label)
}

#[test]
fn every_qml_string_has_a_russian_translation() {
    let found = translations();
    let mut missing = Vec::new();
    for (context, messages) in &found {
        if context == "Menu" {
            continue;
        }
        for (source, translation) in messages {
            if translation.trim().is_empty() {
                missing.push(format!("{context}: {source}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} without a translation, the first: {:?}",
        missing.len(),
        missing.iter().take(5).collect::<Vec<_>>()
    );
    // Placeholders survive the translation.
    for (context, messages) in &found {
        for (source, translation) in messages {
            for n in 1..=9 {
                let marker = format!("%{n}");
                assert_eq!(
                    source.contains(&marker),
                    translation.contains(&marker),
                    "{context}: {source} -> {translation}"
                );
            }
        }
    }
}

#[test]
fn every_menu_label_has_a_russian_translation() {
    let dir = std::env::temp_dir().join(format!("rvp-translations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = App::new(Settings::default(), &dir).expect("app");
    let mut labels = BTreeSet::new();
    walk(&app.full_context_menu(&Live::default()), &mut labels);
    walk(&app.playlist_menu(&Live::default()), &mut labels);
    let _ = std::fs::remove_dir_all(&dir);
    let found = translations();
    let menu = found.get("Menu").cloned().unwrap_or_default();
    let missing: BTreeSet<String> = labels
        .iter()
        .filter(|label| !is_name(label))
        .map(|label| key(label))
        .filter(|key| {
            menu.get(key)
                .is_none_or(|translation| translation.trim().is_empty())
        })
        .collect();
    if let Some(path) = std::env::var_os("RVP_DUMP_MENU") {
        let text = missing
            .iter()
            .fold(String::new(), |text, key| text + key + "\n");
        std::fs::write(PathBuf::from(path), text).expect("dump");
    }
    assert!(
        missing.is_empty(),
        "{} menu labels without a translation, the first: {:?}",
        missing.len(),
        missing.iter().take(5).collect::<Vec<_>>()
    );
}

#[test]
fn keys_set_numbers_aside() {
    assert_eq!(key("30 Sec Backward"), "%1 Sec Backward");
    assert_eq!(key("Increase by 0.5s and 2"), "Increase by %1s and %2");
    assert_eq!(key("No numbers"), "No numbers");
}
