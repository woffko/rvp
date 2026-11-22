//! M15's Unified Remote remote (`remote/unified-remote/RVP`): every action
//! of RVP's it calls exists, every request goes to an endpoint of the
//! control API, `layout.xml` is well formed and each of its events names
//! an action of `remote.lua`, the settings carry RVP's default port and
//! no token, and the script never sends the token.
//!
//! With `RVP_LUA` naming a Lua interpreter (5.1 to 5.4), the script runs
//! with Unified Remote's libraries stubbed: its SHA-256 and HMAC against
//! FIPS 180's and RFC 4231's vectors and the `sha2` crate, its signed
//! requests against a control API server, its port setting, its one timer
//! chain and its polling past a failure. Without one the test says so and
//! passes (Unified Remote itself is checked on the Windows VM,
//! `m15-all.ps1`).
//!
//! `RVP_REMOTE_DIR` points at the remote's folder when the test runs
//! elsewhere (the Windows VM: the bundle's `remote\unified-remote\RVP`).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rvp_app::Action;
use rvp_app::control::{self, DEFAULT_PORT, LIMITS, Limits, Reply, Server, sign};
use sha2::{Digest, Sha256};

fn remote_path(name: &str) -> PathBuf {
    let dir = std::env::var_os("RVP_REMOTE_DIR").map_or_else(
        || {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../remote/unified-remote/RVP"
            ))
        },
        PathBuf::from,
    );
    dir.join(name)
}

fn remote(name: &str) -> String {
    let path = remote_path(name);
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
            [
                "status", "action/", "seek", "volume", "albums", "albums/", "nonce"
            ]
            .contains(&path.as_str()),
            "remote.lua asks for /api/{path}"
        );
    }
}

/// The script signs its requests (RVP's scheme) and never sends the token,
/// and it takes only a port that is a number from 1 to 65535.
#[test]
fn the_remote_never_sends_the_token() {
    let lua = remote("remote.lua");
    assert!(!lua.contains("Bearer"), "remote.lua sends the token");
    assert!(
        lua.contains(&format!("local SCHEME = \"{}\"", sign::SCHEME)),
        "remote.lua signs with another scheme"
    );
    assert!(lua.contains("exchange(\"get\", \"/api/nonce\")"));
    assert!(
        lua.contains("tonumber(settings.port)") && lua.contains("n > 65535"),
        "remote.lua checks its port"
    );
    assert!(lua.contains(&format!("return {DEFAULT_PORT}")));
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
        [
            ("port", DEFAULT_PORT.to_string().as_str()),
            ("token", ""),
            ("token_file", "")
        ]
    );
    let meta = remote("meta.prop");
    assert!(meta.contains("meta.id: RVP.Player"), "{meta}");
}

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Stand-ins for Unified Remote's libraries, the remote, and what the test
/// asks of it; `@@NAME@@` is filled in by the test. Each line it prints is
/// a tab-separated record.
const DRIVER: &str = r#"
local function unhex(text)
	return (string.gsub(text, "%x%x", function (pair) return string.char(tonumber(pair, 16)) end))
end
local nonces = { @@NONCES@@ }
local made = 0
local urls, requests = {}, {}
local answer = nil
local title = nil
local JSON = { ["{}"] = {} }
local armed = {}
libs = {
	http = { request = function (request)
		table.insert(urls, request.url)
		if string.find(request.url, "/api/nonce$") then
			local nonce = table.remove(nonces, 1)
			if nonce == nil then
				made = made + 1
				nonce = string.format("%032d", made)
			end
			local content = '{"nonce":"' .. nonce .. '"}'
			JSON[content] = { nonce = nonce }
			return { status = 200, content = content }
		end
		table.insert(requests, request)
		return { status = 200, content = answer or "{}" }
	end },
	data = {
		tojson = function (value)
			local keys, parts = {}, {}
			for key in pairs(value) do table.insert(keys, key) end
			table.sort(keys)
			for _, key in ipairs(keys) do
				table.insert(parts, '"' .. key .. '":' .. tostring(value[key]))
			end
			return "{" .. table.concat(parts, ",") .. "}"
		end,
		fromjson = function (text)
			local value = JSON[text]
			if value == nil then error("not JSON: " .. text) end
			return value
		end,
		sec2span = tostring,
	},
	server = { update = function (...)
		for _, item in ipairs({ ... }) do
			if item.id == "title" then title = item.text end
		end
	end },
	timer = {
		timeout = function (call) table.insert(armed, call) return #armed end,
		cancel = function () end,
	},
	fs = { exists = function () return false end, read = function () return "" end },
}
settings = { port = "@@PORT@@", token = "@@TOKEN@@" }
events = {}
actions = {}
OS_WINDOWS = false
rvp_check = {}
dofile([==[@@REMOTE@@]==])

for at, vector in ipairs({ @@VECTORS@@ }) do
	local digest
	if vector.key == nil then
		digest = rvp_check.sha256(unhex(vector.message))
	else
		digest = rvp_check.hmac(unhex(vector.key), unhex(vector.message))
	end
	print("vector\t" .. at .. "\t" .. digest)
end

-- Signed requests over RVP's nonces.
actions.play_pause()
actions.seek(12)
for _, request in ipairs(requests) do
	print("request\t" .. string.upper(request.method) .. "\t" .. request.url .. "\t"
		.. ((request.headers or {}).Authorization or "") .. "\t" .. (request.content or ""))
end

-- One timer chain: a second focus ends the first chain, also when its
-- timer still fires, and a blur ends the second.
events.focus()
events.focus()
local fired = #armed
for at = 1, fired do armed[at]() end
print("timers\t" .. fired .. "\t" .. #armed)
events.blur()
armed[#armed]()
print("blurred\t" .. #armed)

-- A token file that cannot be read and an answer that is not JSON: the
-- poll goes on, an action fails no more than its request, and the title
-- says so.
libs.fs.exists = function () return true end
libs.fs.read = function () error("unreadable") end
settings.token = ""
answer = "not json"
local before = #armed
local polled = pcall(events.focus)
local armed_now = #armed - before
title = nil
local acted = pcall(actions.play_pause)
print("failure\t" .. tostring(polled) .. "\t" .. armed_now .. "\t" .. tostring(acted) .. "\t" .. tostring(title))
events.blur()
answer = nil
libs.fs.exists = function () return false end

-- No token setting: the file token_file names (a portable RVP's, written
-- by RVP's Add to Unified Remote), which also makes the remote detected.
print("detect\tnone\t" .. tostring(events.detect()))
settings.token_file = "/portable/state/remote-token"
libs.fs.exists = function (path) return path == settings.token_file end
libs.fs.read = function (path)
	if path ~= settings.token_file then error("another file: " .. path) end
	return "@@TOKEN@@\n"
end
print("detect\tfile\t" .. tostring(events.detect()))
requests = {}
actions.play_pause()
for _, request in ipairs(requests) do
	print("tokenfile\t" .. string.upper(request.method) .. "\t" .. request.url .. "\t"
		.. ((request.headers or {}).Authorization or ""))
end
settings.token_file = nil
libs.fs.exists = function () return false end
libs.fs.read = function () return "" end

-- The port setting.
for _, port in ipairs({ @@PORTS@@ }) do
	settings.port = port
	urls = {}
	actions.play_pause()
	print("port\t" .. tostring(port) .. "\t" .. urls[1])
end
settings.port = nil
urls = {}
actions.play_pause()
print("port\tnil\t" .. urls[1])
"#;

/// One exchange with the server: the status and the body.
fn exchange(address: SocketAddr, raw: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(address).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .expect("timeout");
    stream.write_all(raw).expect("send");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("answer");
    let (head, body) = answer.split_once("\r\n\r\n").expect("head and body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("status line");
    (status, body.to_owned())
}

fn raw_request(
    address: SocketAddr,
    method: &str,
    target: &str,
    authorization: Option<&str>,
    body: &str,
) -> Vec<u8> {
    let mut raw = format!(
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n",
        address.port()
    );
    if let Some(authorization) = authorization {
        let _ = write!(raw, "Authorization: {authorization}\r\n");
    }
    if method == "POST" {
        let _ = write!(
            raw,
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
    }
    raw.push_str("\r\n");
    raw.push_str(body);
    raw.into_bytes()
}

/// The messages and keys the script hashes: FIPS 180's examples, every
/// length around SHA-256's padding edges, and RFC 4231's HMAC cases.
fn vectors() -> Vec<(Option<Vec<u8>>, Vec<u8>)> {
    let mut vectors: Vec<(Option<Vec<u8>>, Vec<u8>)> = vec![
        (None, b"abc".to_vec()),
        (
            None,
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".to_vec(),
        ),
        (
            None,
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
                .to_vec(),
        ),
    ];
    let pattern: Vec<u8> = (0..=255u8).cycle().take(200).collect();
    for length in [0, 1, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 200] {
        vectors.push((None, pattern[..length].to_vec()));
    }
    let long_key = vec![0xaa; 131];
    vectors.extend([
        (Some(vec![0x0b; 20]), b"Hi There".to_vec()),
        (
            Some(b"Jefe".to_vec()),
            b"what do ya want for nothing?".to_vec(),
        ),
        (Some(vec![0xaa; 20]), vec![0xdd; 50]),
        (Some((1..=25).collect()), vec![0xcd; 50]),
        (
            Some(long_key.clone()),
            b"Test Using Larger Than Block-Size Key - Hash Key First".to_vec(),
        ),
        (
            Some(long_key),
            b"This is a test using a larger than block-size key and a larger than block-size \
              data. The key needs to be hashed before being used by the HMAC algorithm."
                .to_vec(),
        ),
        (Some(TOKEN.as_bytes().to_vec()), b"".to_vec()),
    ]);
    vectors
}

/// The script's records of one kind, each split at its tabs.
fn records<'a>(output: &'a str, kind: &str) -> Vec<Vec<&'a str>> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            (fields.next() == Some(kind)).then(|| fields.collect())
        })
        .collect()
}

/// The port settings the script is given (and the number 48933, and none).
const PORTS: [&str; 10] = [
    "48931",
    " 48932 ",
    "1",
    "65535",
    "0",
    "65536",
    "80@evil.example",
    "1.5",
    "abc",
    "",
];

/// Runs `remote.lua` with a Lua interpreter (`RVP_LUA`) against a control
/// API server: its hashes are the standard's, its requests are signed as
/// RVP checks them and carry no token, and its port, timers and failures
/// behave.
#[test]
fn the_remote_signs_as_rvp_checks() {
    let Some(lua) = std::env::var_os("RVP_LUA") else {
        eprintln!(
            "note: RVP_LUA names no Lua interpreter: remote.lua's signatures were not run here"
        );
        return;
    };
    let dispatch: control::Dispatch = Arc::new(|_command, reply: Reply| {
        let _ = reply.send(Ok(serde_json::json!({})));
    });
    let limits = Limits {
        nonce_life: Duration::from_secs(120),
        ..LIMITS
    };
    let server = Server::start_with(0, TOKEN.to_owned(), dispatch, limits).expect("server");
    let address = server.address();
    let nonces: Vec<String> = (0..4)
        .map(|_| {
            let (status, body) = exchange(
                address,
                &raw_request(address, "GET", "/api/nonce", None, ""),
            );
            assert_eq!(status, 200, "{body}");
            let value: serde_json::Value = serde_json::from_str(&body).expect("JSON");
            value["nonce"].as_str().expect("nonce").to_owned()
        })
        .collect();
    let vectors = vectors();
    let output = run_remote(&lua, address.port(), &nonces, &vectors);
    check_hashes(&output, &vectors);
    check_requests(&output, &server, &nonces);

    // One timer chain; polling goes on past a failure.
    assert_eq!(records(&output, "timers"), [vec!["2", "3"]], "{output}");
    assert_eq!(records(&output, "blurred"), [vec!["3"]], "{output}");
    let failure = records(&output, "failure");
    assert_eq!(failure.len(), 1, "{output}");
    assert_eq!(&failure[0][..3], ["true", "1", "true"], "{output}");
    assert!(failure[0][3].starts_with("RVP does not answer"), "{output}");
    check_token_file(&output, address.port());

    check_ports(&output);
}

/// The script run by `lua` with its driver: what it printed.
fn run_remote(
    lua: &std::ffi::OsStr,
    port: u16,
    nonces: &[String],
    vectors: &[(Option<Vec<u8>>, Vec<u8>)],
) -> String {
    let lua_vectors = vectors
        .iter()
        .fold(String::new(), |mut text, (key, message)| {
            let message = sign::hex(message);
            let _ = match key {
                Some(key) => write!(
                    text,
                    "{{ key = \"{}\", message = \"{message}\" }}, ",
                    sign::hex(key)
                ),
                None => write!(text, "{{ message = \"{message}\" }}, "),
            };
            text
        });
    let lua_ports = PORTS.iter().fold(String::new(), |mut text, port| {
        let _ = write!(text, "\"{port}\", ");
        text
    }) + "48933";
    let script = DRIVER
        .replace(
            "@@NONCES@@",
            &nonces
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", "),
        )
        .replace("@@PORT@@", &port.to_string())
        .replace("@@TOKEN@@", TOKEN)
        .replace("@@REMOTE@@", &remote_path("remote.lua").to_string_lossy())
        .replace("@@VECTORS@@", &lua_vectors)
        .replace("@@PORTS@@", &lua_ports);
    let dir = std::env::temp_dir().join(format!("rvp-remote-lua-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let driver = dir.join("driver.lua");
    std::fs::write(&driver, script).expect("driver");
    let run = std::process::Command::new(lua)
        .arg(&driver)
        .output()
        .expect("the Lua interpreter runs");
    let _ = std::fs::remove_dir_all(&dir);
    let output = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        run.status.success(),
        "{output}{}",
        String::from_utf8_lossy(&run.stderr)
    );
    output
}

/// The hashes: the standard's and the sha2 crate's.
fn check_hashes(output: &str, vectors: &[(Option<Vec<u8>>, Vec<u8>)]) {
    let digests = records(output, "vector");
    assert_eq!(digests.len(), vectors.len(), "{output}");
    for ((key, message), record) in vectors.iter().zip(&digests) {
        let expected = match key {
            Some(key) => sign::hex(&sign::hmac_sha256(key, message)),
            None => sign::hex(&Sha256::digest(message)),
        };
        assert_eq!(record[1], expected, "vector {}", record[0]);
    }
    for (at, fips) in [
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(digests[at][1], *fips, "FIPS 180 example {at}");
    }
}

/// The requests: signed as RVP checks them, with no token, each passes
/// once.
fn check_requests(output: &str, server: &Server, nonces: &[String]) {
    let address = server.address();
    let requests = records(output, "request");
    assert_eq!(requests.len(), nonces.len(), "{output}");
    let base = format!("http://127.0.0.1:{}", address.port());
    let mut sent = Vec::new();
    for (record, nonce) in requests.iter().zip(nonces) {
        let &[method, url, authorization, body] = record.as_slice() else {
            panic!("{record:?}");
        };
        let target = url.strip_prefix(&base).expect("RVP's address");
        assert_eq!(
            *authorization,
            sign::authorization(
                TOKEN,
                address.port(),
                nonce,
                method,
                target,
                body.as_bytes()
            ),
            "{method} {target}"
        );
        assert!(!authorization.contains(TOKEN));
        let raw = raw_request(address, method, target, Some(authorization), body);
        let (status, answer) = exchange(address, &raw);
        assert_eq!(status, 200, "{method} {target}: {answer}");
        sent.push(raw);
    }
    assert_eq!(
        requests.iter().map(|r| (r[0], r[3])).collect::<Vec<_>>(),
        [
            ("POST", ""),
            ("GET", ""),
            ("POST", r#"{"to":12}"#),
            ("GET", "")
        ]
    );
    assert_eq!(exchange(address, &sent[0]).0, 401, "a nonce serves once");
    assert_eq!(server.counts().signed, 4);
    assert_eq!(server.counts().bearer, 0);
}

/// The token taken from the file `token_file` names: the request is
/// signed with it (over a nonce the driver made up), and the remote counts
/// as detected once that file is there.
fn check_token_file(output: &str, port: u16) {
    assert_eq!(
        records(output, "detect"),
        [vec!["none", "false"], vec!["file", "true"]],
        "{output}"
    );
    // The action and the status read after it.
    let requests = records(output, "tokenfile");
    assert_eq!(
        requests.iter().map(|r| r[0]).collect::<Vec<_>>(),
        ["POST", "GET"],
        "{output}"
    );
    for request in requests {
        let &[method, url, authorization] = request.as_slice() else {
            panic!("{request:?}");
        };
        let target = url
            .strip_prefix(&format!("http://127.0.0.1:{port}"))
            .expect("RVP's address");
        let nonce = authorization
            .split_once("nonce=")
            .and_then(|(_, rest)| rest.split(',').next())
            .expect("a nonce");
        assert_eq!(
            authorization,
            sign::authorization(TOKEN, port, nonce, method, target, b""),
            "{output}"
        );
    }
}

/// The port: a whole number from 1 to 65535, else RVP's default.
fn check_ports(output: &str) {
    let mut expected: Vec<(String, u16)> = PORTS
        .iter()
        .map(|port| {
            let chosen = match port.trim().parse::<u16>() {
                Ok(n) if n >= 1 => n,
                _ => DEFAULT_PORT,
            };
            ((*port).to_owned(), chosen)
        })
        .collect();
    expected.push(("48933".to_owned(), 48933));
    expected.push(("nil".to_owned(), DEFAULT_PORT));
    let used: Vec<(String, u16)> = records(output, "port")
        .iter()
        .map(|record| {
            let port = record[1]
                .strip_prefix("http://127.0.0.1:")
                .and_then(|rest| rest.strip_suffix("/api/nonce"))
                .and_then(|port| port.parse().ok())
                .unwrap_or_else(|| panic!("{}", record[1]));
            (record[0].to_owned(), port)
        })
        .collect();
    assert_eq!(used, expected);
}
