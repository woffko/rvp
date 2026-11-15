//! RVP's interface languages (M14, M19): every translation found in
//! `shell/i18n` (a language is its files `<part>_<code>.ts`, written for
//! RVP; no language is named here) covers every string of the QML files,
//! every label of RVP's menus and the packages' strings, keeps their
//! placeholders, names itself, and the desktop entry carries its fields.
//! Menu labels are looked up as QML's `trMenu` does: their numbers stand
//! aside as `%1`, `%2`...
//!
//! `RVP_DUMP_MENU=FILE` writes the menu labels without a translation in
//! some language to FILE, one per line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rvp_app::menu::{Live, MenuItem};
use rvp_app::{App, Settings};

/// `context` -> `source` -> translation (empty when unfinished).
type Messages = BTreeMap<String, BTreeMap<String, String>>;

fn i18n() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shell/i18n")
}

/// The languages found: code -> part (`qml`, `menu`, `packaging`) -> its
/// messages.
fn languages() -> BTreeMap<String, BTreeMap<String, Messages>> {
    let mut found: BTreeMap<String, BTreeMap<String, Messages>> = BTreeMap::new();
    for entry in std::fs::read_dir(i18n()).expect("shell/i18n") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|extension| extension != "ts") {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some((part, code)) = stem.split_once('_') else {
            continue;
        };
        let text = std::fs::read_to_string(&path).expect("read .ts");
        found
            .entry(code.to_owned())
            .or_default()
            .insert(part.to_owned(), parse(&text));
    }
    found
}

fn unescape(text: &str) -> String {
    // lupdate writes control characters as <byte value="xNN" />.
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("<byte value=\"x") {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 14..];
        let end = tail.find('"').unwrap_or(0);
        if let Some(c) = u32::from_str_radix(&tail[..end], 16)
            .ok()
            .and_then(char::from_u32)
        {
            out.push(c);
        }
        rest = tail.find("/>").map_or("", |close| &tail[close + 2..]);
    }
    out.push_str(rest);
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn between(text: &str, open: &str, close: &str) -> Option<String> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(text[start..end].to_owned())
}

fn parse(text: &str) -> Messages {
    let mut found = Messages::new();
    for context in text.split("<context>").skip(1) {
        let name = between(context, "<name>", "</name>").unwrap_or_default();
        let messages = found.entry(name).or_default();
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

/// Every language's messages of `part`, as one set of `(context, source)`.
fn union(
    languages: &BTreeMap<String, BTreeMap<String, Messages>>,
    part: &str,
) -> BTreeSet<(String, String)> {
    languages
        .values()
        .filter_map(|parts| parts.get(part))
        .flat_map(|messages| {
            messages.iter().flat_map(|(context, sources)| {
                sources
                    .keys()
                    .map(|source| (context.clone(), source.clone()))
            })
        })
        .collect()
}

fn lookup<'a>(
    languages: &'a BTreeMap<String, BTreeMap<String, Messages>>,
    code: &str,
    part: &str,
    context: &str,
    source: &str,
) -> Option<&'a str> {
    languages
        .get(code)?
        .get(part)?
        .get(context)?
        .get(source)
        .map(String::as_str)
        .filter(|translation| !translation.trim().is_empty())
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

/// Labels that are names, not words: RVP's own names, English (the
/// source's own name), the user's albums and whisper.cpp's engines
/// (listed when installed, as on Windows).
fn is_name(label: &str) -> bool {
    ["RVP Default (built in)", "English", "Default"].contains(&label)
        || rvp_app::speech::ENGINES
            .iter()
            .any(|(_, engine)| *engine == label)
        // A model that is present shows its bare name (RVP_WHISPER_MODELS).
        || rvp_app::speech::MODELS.contains(&label)
}

#[test]
fn every_language_translates_every_string() {
    // Every message any language has (the menu's too: submenu titles that
    // show only while a file plays, such as "Filters", are not in the
    // test's menus below).
    let languages = languages();
    assert!(!languages.is_empty(), "no translation in shell/i18n");
    let mut missing = Vec::new();
    for part in ["qml", "menu", "packaging"] {
        let all = union(&languages, part);
        for code in languages.keys() {
            for (context, source) in &all {
                if lookup(&languages, code, part, context, source).is_none() {
                    missing.push(format!("{code} {part} {context}: {source}"));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} strings without a translation, the first: {:?}",
        missing.len(),
        missing.iter().take(5).collect::<Vec<_>>()
    );
}

#[test]
fn translations_keep_their_placeholders() {
    for (code, parts) in &languages() {
        for messages in parts.values() {
            for (context, sources) in messages {
                for (source, translation) in sources {
                    if translation.is_empty() {
                        continue;
                    }
                    for n in 1..=9 {
                        let marker = format!("%{n}");
                        assert_eq!(
                            source.contains(&marker),
                            translation.contains(&marker),
                            "{code} {context}: {source} -> {translation}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn every_language_names_itself() {
    let languages = languages();
    for code in languages.keys() {
        let name = lookup(&languages, code, "qml", "Language", "English");
        assert!(
            name.is_some_and(|name| name != "English"),
            "{code} gives no name of its own for \"English\" (context Language)"
        );
    }
}

#[test]
fn every_menu_label_has_a_translation_in_every_language() {
    let dir = std::env::temp_dir().join(format!("rvp-translations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let app = App::new(Settings::default(), &dir).expect("app");
    let mut labels = BTreeSet::new();
    walk(&app.full_context_menu(&Live::default()), &mut labels);
    walk(&app.playlist_menu(&Live::default()), &mut labels);
    let _ = std::fs::remove_dir_all(&dir);
    let languages = languages();
    let mut missing = BTreeSet::new();
    for code in languages.keys() {
        for label in labels.iter().filter(|label| !is_name(label)) {
            let key = key(label);
            if lookup(&languages, code, "menu", "Menu", &key).is_none() {
                missing.insert(format!("{code}: {key}"));
            }
        }
    }
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
fn the_desktop_entry_carries_every_language() {
    // tools/i18n/desktop.py writes these lines from packaging_<code>.ts.
    let entry = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/linux/rvp.desktop"),
    )
    .expect("rvp.desktop");
    let languages = languages();
    let mut wanted = BTreeSet::new();
    for key in ["GenericName", "Comment"] {
        let english = entry
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .expect("an English field");
        for code in languages.keys() {
            if let Some(text) = lookup(&languages, code, "packaging", "Desktop", english) {
                wanted.insert(format!("{key}[{code}]={text}"));
            }
        }
    }
    let present: BTreeSet<String> = entry
        .lines()
        .filter(|line| line.starts_with("GenericName[") || line.starts_with("Comment["))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        present, wanted,
        "rvp.desktop differs from the translations: run tools/i18n/desktop.py"
    );
}

#[test]
fn keys_set_numbers_aside() {
    assert_eq!(key("30 Sec Backward"), "%1 Sec Backward");
    assert_eq!(key("Increase by 0.5s and 2"), "Increase by %1s and %2");
    assert_eq!(key("No numbers"), "No numbers");
}
