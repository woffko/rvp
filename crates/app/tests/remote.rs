//! M15's Unified Remote remote (`remote/unified-remote/RVP`): every action
//! of RVP's it calls exists, every request goes to an endpoint of the
//! control API, `layout.xml` is well formed and each of its events names
//! an action of `remote.lua`, and the settings carry RVP's default port and
//! no token.
//!
//! `RVP_REMOTE_DIR` points at the remote's folder when the test runs
//! elsewhere (the Windows VM: the bundle's `remote\unified-remote\RVP`).

use std::collections::BTreeSet;
use std::path::PathBuf;

use rvp_app::Action;
use rvp_app::control::DEFAULT_PORT;

fn remote(name: &str) -> String {
    let dir = std::env::var_os("RVP_REMOTE_DIR").map_or_else(
        || {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../remote/unified-remote/RVP"
            ))
        },
        PathBuf::from,
    );
    let path = dir.join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The quoted texts that follow `prefix` in `text` (`act("` gives the
/// actions' names).
fn quoted_after(text: &str, prefix: &str) -> Vec<String> {
    text.match_indices(prefix)
        .filter_map(|(at, _)| {
            let rest = &text[at + prefix.len()..];
            rest.find('"').map(|end| rest[..end].to_owned())
        })
        .collect()
}

#[test]
fn every_action_the_remote_calls_exists() {
    let lua = remote("remote.lua");
    let names = quoted_after(&lua, "act(\"");
    assert!(names.len() >= 15, "{names:?}");
    for name in &names {
        assert!(
            Action::from_name(name).is_some(),
            "remote.lua calls {name}, which RVP lacks"
        );
    }
    // The other requests go to the control API's endpoints.
    let paths = quoted_after(&lua, ", \"/api/");
    assert!(!paths.is_empty());
    for path in paths {
        assert!(
            ["status", "action/", "seek", "volume", "albums", "albums/"].contains(&path.as_str()),
            "remote.lua asks for /api/{path}"
        );
    }
}

/// One element of a strict reading of `layout.xml`.
#[derive(Debug)]
struct Element {
    name: String,
    attributes: Vec<(String, String)>,
}

/// Reads XML strictly enough to call it well formed: one root, tags
/// that close in order, quoted attributes without duplicates, escaped
/// `<` and `&` in text and values, comments and the declaration.
fn read_xml(text: &str) -> Result<Vec<Element>, String> {
    let mut elements = Vec::new();
    let mut open: Vec<String> = Vec::new();
    let mut roots = 0;
    let mut rest = text.trim_start_matches('\u{feff}');
    if let Some(after) = rest.strip_prefix("<?xml") {
        rest = &after[after.find("?>").ok_or("unclosed declaration")? + 2..];
    }
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            if !rest.trim().is_empty() && open.is_empty() {
                return Err(format!("text outside the root: {rest:?}"));
            }
            check_text(rest)?;
            break;
        };
        check_text(&rest[..at])?;
        if !rest[..at].trim().is_empty() && open.is_empty() {
            return Err("text outside the root".to_owned());
        }
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("<!--") {
            let end = after.find("-->").ok_or("unclosed comment")?;
            rest = &after[end + 3..];
            continue;
        }
        let end = rest.find('>').ok_or("unclosed tag")?;
        let tag = &rest[1..end];
        rest = &rest[end + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            match open.pop() {
                Some(expected) if expected == name.trim() => {}
                other => return Err(format!("</{name}> closes {other:?}")),
            }
            continue;
        }
        let (tag, empty) = match tag.strip_suffix('/') {
            Some(tag) => (tag, true),
            None => (tag, false),
        };
        let name_end = tag.find(char::is_whitespace).unwrap_or(tag.len());
        let name = &tag[..name_end];
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(format!("bad element name {name:?}"));
        }
        if open.is_empty() {
            roots += 1;
        }
        let mut attributes: Vec<(String, String)> = Vec::new();
        let mut body = tag[name_end..].trim();
        while !body.is_empty() {
            let eq = body
                .find('=')
                .ok_or_else(|| format!("attribute without value in <{name}>"))?;
            let key = body[..eq].trim();
            let value_part = body[eq + 1..].trim_start();
            let quote = value_part.chars().next().ok_or("missing value")?;
            if quote != '"' && quote != '\'' {
                return Err(format!("unquoted value of {key} in <{name}>"));
            }
            let close = value_part[1..].find(quote).ok_or("unclosed value")?;
            let value = &value_part[1..=close];
            check_text(value)?;
            if attributes.iter().any(|(k, _)| k.eq_ignore_ascii_case(key)) {
                return Err(format!("{key} twice in <{name}>"));
            }
            attributes.push((key.to_ascii_lowercase(), value.to_owned()));
            body = value_part[close + 2..].trim_start();
        }
        elements.push(Element {
            name: name.to_owned(),
            attributes,
        });
        if !empty {
            open.push(name.to_owned());
        }
    }
    if !open.is_empty() {
        return Err(format!("unclosed elements {open:?}"));
    }
    if roots != 1 {
        return Err(format!("{roots} root elements"));
    }
    Ok(elements)
}

fn check_text(text: &str) -> Result<(), String> {
    if text.contains('<') {
        return Err(format!("unescaped < in {text:?}"));
    }
    for (at, _) in text.match_indices('&') {
        let entity = &text[at..];
        if !["&amp;", "&lt;", "&gt;", "&quot;", "&apos;", "&#"]
            .iter()
            .any(|known| entity.starts_with(known))
        {
            return Err(format!("unescaped & in {text:?}"));
        }
    }
    Ok(())
}

#[test]
fn layout_xml_is_well_formed_and_its_events_are_actions() {
    let elements = read_xml(&remote("layout.xml")).expect("well formed");
    assert_eq!(elements.first().map(|e| e.name.as_str()), Some("layout"));
    let lua = remote("remote.lua");
    // `actions.NAME = function` lines.
    let actions: BTreeSet<&str> = lua
        .lines()
        .filter_map(|line| line.strip_prefix("actions."))
        .filter_map(|rest| rest.split_once(" = function").map(|(name, _)| name))
        .collect();
    assert!(actions.contains("play_pause"), "{actions:?}");
    let mut events = 0;
    for element in &elements {
        for (key, value) in &element.attributes {
            if key.starts_with("on") {
                events += 1;
                assert!(
                    actions.contains(value.as_str()),
                    "<{} {key}=\"{value}\"> names no action of remote.lua",
                    element.name
                );
            }
        }
    }
    assert!(events >= 20, "{events} events");
    // Controls that remote.lua updates exist.
    let ids: BTreeSet<&str> = elements
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|(key, _)| key == "id")
        .map(|(_, value)| value.as_str())
        .collect();
    for id in quoted_after(&lua, "id = \"") {
        assert!(
            ids.contains(id.as_str()),
            "remote.lua updates {id}, which layout.xml lacks"
        );
    }
    // Pictures are RVP's own, drawn by `cargo xtask look`.
    let drawn: BTreeSet<&str> = rvp_look::outputs::REMOTE_BUTTONS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let pictures: Vec<&str> = elements
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|(key, _)| key == "image")
        .map(|(_, value)| value.as_str())
        .collect();
    assert!(pictures.contains(&"mute.png"), "{pictures:?}");
    for picture in pictures {
        assert!(
            drawn.contains(picture),
            "layout.xml shows {picture}, which nothing draws"
        );
    }
}

#[test]
fn strict_reading_refuses_broken_xml() {
    for broken in [
        "<layout><row></layout></row>",
        "<layout><button text=x /></layout>",
        "<layout><label text=\"a < b\" /></layout>",
        "<layout></layout><layout></layout>",
        "<layout><row>",
        "<layout a=\"1\" a=\"2\"></layout>",
    ] {
        assert!(read_xml(broken).is_err(), "{broken}");
    }
}

#[test]
fn the_settings_carry_the_default_port_and_no_token() {
    let settings = remote("settings.prop");
    let values: Vec<(&str, &str)> = settings
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect();
    assert_eq!(
        values,
        [("port", DEFAULT_PORT.to_string().as_str()), ("token", "")]
    );
    let meta = remote("meta.prop");
    assert!(meta.contains("meta.id: RVP.Player"), "{meta}");
}
